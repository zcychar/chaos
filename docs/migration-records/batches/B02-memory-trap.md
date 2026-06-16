# B02 memory + trap 详细迁移记录

本文是 B02 的批次级执行记录。它把 [migration-batches.md](../migration-batches.md) 中的 B02 拆成函数级/子模块级迁移单元，用于明确：目标文件/行、`kernel.rs` 语义来源、当前 rCore 表达、接口处理方式、最小修改范围、已落地结果和运行边界。

当前状态：`PARTIAL_MIGRATED`。工作区已经存在 B02 的部分源码迁移、RISC-V PTE 可见性修正和 file-backed LOAD 页内填充修正；本文同步实际源码状态。后续任何新的源码修正仍需重新按仓库规则报告文件/行、现象、根因、预期行为和最小修改，并等待用户确认。

当前运行验证：`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；单核 QEMU 已越过旧的 `0x3fffffff` init stack fault、`0x100e8` busybox 入口反复 instruction fault 和 `sepc=0x0` 跳转 fault，进入 busybox shell 提示符 `/ #`。这不代表 B02 全部语义完成，只说明本轮 memory/paging/file-backed LOAD 运行边界已解除。

## 批次定位

| 项目 | 内容 |
| --- | --- |
| 批次 | B02 memory + trap |
| 恢复模块 | `crate/memory/*`、`kernel/src/memory.rs`、`kernel/src/trap.rs` |
| `kernel.rs` 来源行段 | `916-1689`、`5179-5181`、`7529-7568`、`7728-7788` |
| 上游基线 | 恢复源码与 rCore commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` hash 一致 |
| 接口原则 | 保留 rCore 的 `MemorySet`、`MemoryHandler`、`FrameAllocator`、`VMResult`、`SysError` 调用面，不复制 `kernel.rs` 的模拟 `VmMap`/`FramePool` 类型 |

## 已落地源码对齐记录

| 子项 | 当前落点 | `kernel.rs` 来源语义 | 已处理接口 | 当前状态 |
| --- | --- | --- | --- | --- |
| COW/refcount | `crate/memory/src/cow.rs` | `PgFrame::{up,down,count,inc_if_nonzero}`、`SharedPage::fault` | 保留 `CowExt<T>` 和 `FrameRcMap` 内部结构；refcount 增加改 saturating，减少改为 bool helper，0/缺失条目不 panic。 | `PARTIAL_MIGRATED` |
| MemorySet/range | `crate/memory/src/memory_set/mod.rs` | `VmRegion::{end,contains}`、`VmMap::{insert,find_free}`、`AddrSpace::{unmap_range,split_region}` | 保留 `MemorySet` public API；新增 checked align helper，非法/overflow range 提前返回，`with` 改为泛型返回值。 | `PARTIAL_MIGRATED` |
| file-backed mapping | `crate/memory/src/memory_set/handler/file.rs` | `FLike::mmap_fl` 中 file offset/length 边界 | 保留 `Read::read_at` 和 `MemoryHandler` trait；页内先清零，再按文件段与 fault page 的交集读取，支持非页对齐 ELF LOAD；分配失败返回 false。 | `PARTIAL_MIGRATED` |
| eager frame fault | `crate/memory/src/memory_set/handler/byframe.rs` | 匿名页已映射后只应做权限核对 | 保留 `ByFrame<T>`；fault 只在 entry present 且 access 权限满足时返回 true。 | `MIGRATED` |
| kernel memory facade | `kernel/src/memory.rs` | `FramePool`、`check_access`、`validate_access`、`cfu`、`ctu`、init-time VM fault | 保留 rCore `FrameAllocator`/copy-user API；加入 active `MemorySet` fallback、checked contiguous alloc/dealloc、checked user range。 | `PARTIAL_MIGRATED` |
| uptime | `kernel/src/trap.rs` | `up_ms` | 保留 `uptime_msec() -> usize`；乘法改为 `saturating_mul`。 | `MIGRATED` |
| init stack 写入 | `kernel/src/process/abi.rs`、`kernel/src/process/thread.rs` | `ProcInit::push_at` 的栈布局与 checked 指针移动 | 不改 `Thread::new_user_vm` 对外返回；新增 `push_at_in_vm`/`VmStackWriter`，真实用户进程初始化通过正在构造的 `MemorySet` 写栈。 | `PARTIAL_MIGRATED` |
| RISC-V PTE update | `kernel/src/arch/riscv/paging.rs` | page fault 后 PTE 可见性/TLB 刷新 | 保留 riscv crate wrapper；`sfence_vma` 参数修正为 fault page vaddr；新增非 leaf PTE flags 归一化，清除上级页表项中的 `A/D/U`。 | `MIGRATED` |

## 运行发现

B01 完成后执行短时 QEMU 启动验证，内核已能编译并进入 OpenSBI/rCore，但在初始化用户 shell 时触发 rCore stack trace。符号化结果指向本批次范围：

| 位置 | 现象 |
| --- | --- |
| `kernel/src/process/thread.rs:224` | `Thread::new_user_vm` 在 `vm.with(|| init_info.push_at(ustack_top))` 内写入用户栈。 |
| `kernel/src/memory.rs:144-152` | `handle_page_fault_ext` 收到内核态 page fault 后直接 `current_thread().unwrap()` 并转发到当前线程 VM。 |
| `crate/memory/src/memory_set/mod.rs:379-385` | `MemorySet::handle_page_fault_ext` 依赖 area lookup 和 handler 处理 fault。 |

源码修改前报告摘要：

- 文件/行：`kernel/src/memory.rs:144-152`、`kernel/src/process/thread.rs:224`、`crate/memory/src/memory_set/mod.rs:379-385`。
- 失败症状：QEMU 启动进入 rCore 后，在 `Thread::new_user_vm` 初始化用户栈阶段打印 stack trace；符号化显示 panic/fault 路径经过 `handle_page_fault_ext`。
- 根因/当前表达：用户 VM 初始化期间还没有可用的 `current_thread()`，但 page fault facade 假设所有 fault 都发生在当前线程上下文；同时 B02 中 MemorySet/range/fault 边界还未吸收 `kernel.rs` 的 checked 初始化语义。
- 预期行为：用户栈初始化应只写入已映射 eager stack 页，或在 init-time fault 中使用正在构造的 `MemorySet` 上下文；不得依赖不存在的 current thread；非法/越界 fault 应干净失败而非 unwrap panic。
- 最小修改：先在 B02 中处理 `kernel/src/memory.rs` fault facade 与 `MemorySet::handle_page_fault_ext` 的 init-time 边界，再联动 B05 的 `ProcInitInfo::push_at` checked stack 写入；public API 变更需另行报批。

已处理结果：

- `kernel/src/memory.rs` 增加 `ACTIVE_MEMORY_SET`、`ActiveMemorySetGuard`、`with_active_memory_set`，page fault facade 先尝试当前线程 VM，没有 current thread 时回落到正在构造的 active VM。
- `kernel/src/process/abi.rs` 增加 `InitStackWriter`、`VmStackWriter` 和 `ProcInitInfo::push_at_in_vm`，通过 `MemorySet::handle_page_fault_ext` 准备栈页并通过页表 slice 写入用户栈。
- `kernel/src/process/thread.rs` 的 `Thread::new_user_vm` 已改为调用 `init_info.push_at_in_vm(vm, ustack_top)`。
- 运行上已越过旧的 `0x3fffffff` 栈写入 fault，启动日志进入 `process: init end` 和 `Hello RISCV!`。

已解决运行边界：

- `0x100e8` 反复 instruction page fault：QEMU monitor 确认 `satp = 0x8000000000080ed8`，leaf PTE 已是 `V|R|X|U|A|D` 并指向 `0x80ee0000`，但上级非 leaf PTE 是 `V|A|D`。修复点是 `kernel/src/arch/riscv/paging.rs::normalize_intermediate_entries`，对映射路径中的非 leaf PTE 清除 `A/D/U`。
- `sepc=0x0` instruction page fault：反汇编确认 `.init_array` 循环从 `0x10bff0` 读取函数指针后 `jalr`；`busybox` 第二个 LOAD 段从 `vaddr=0x10bff0`、`offset=0x0faff0` 开始，不按页对齐。旧 `File::fill_data` 对 fault page `0x10b000` 执行 `addr.checked_sub(mem_start)` 失败并清零整页，导致 `.init_array` 读到 0。修复点是 `crate/memory/src/memory_set/handler/file.rs::fill_data`，按 page 与 file-backed 段交集读取到页内正确 offset。
- 验证结果：`/tmp/chaos-qemu-b02-after-file-load.log` 显示进入 `set_tid_address`、多次 syscall/page fault、`openat("/dev/tty")`、`wait4`、TTY ioctl，最终出现 busybox shell 提示符 `/ #`；未再出现 `page fault from user @ 0x0` 或 panic。

## B02.1 COW refcount

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `crate/memory/src/cow.rs:91`、`:169`、`:178`、`:186`、`:195` |
| rCore 函数 | `CowExt::page_fault_handler`、`FrameRcMap::{read_increase,read_decrease,write_increase,write_decrease}` |
| `kernel.rs` 来源 | `PgFrame::{up,down,count,inc_if_nonzero}`，`kernel/src/kernel.rs:916-982`；`SharedPage::fault`，`:1566-1606` |
| 迁移语义 | COW/source frame refcount 增减必须对称；decrease 不得在 0 上 underflow；COW fault 成功后 frame、权限、shared 标记和 refcount 状态一致。 |
| 迁移前 rCore 表达 | `FrameRcMap::{read_decrease,write_decrease}` 直接 `unwrap().0 -= 1` / `unwrap().1 -= 1`；`page_fault_handler` 在 single-writer 快路径和 unmap/map 路径调用 decrease。 |
| 已落地结果 | `read_increase` / `write_increase` 使用 `saturating_add`；`read_decrease` / `write_decrease` 改为返回 bool 的内部 `decrease` helper，缺失 frame 或 0 count 不 unwrap/underflow，读写计数归零时移除条目。 |
| 接口处理 | 不改变 `CowExt<T: PageTable>` public API；边界失败应留在内部 helper 或返回 `false`，而不是新增错误类型。 |
| 最小修改范围 | `FrameRcMap` decrement helper 和 `CowExt::page_fault_handler` 调用点；必要时在内部清理 0-count 条目。 |
| 不应修改 | `PageTable` trait、entry shared bit API、调用方 fault facade。 |
| 验收点 | zero-count decrement 不 panic/underflow；single-writer COW 快路径只减少一次 writable ref；copy path 不重复释放 source frame；fault 成功后 entry present+writable 且 shared 标记清理正确。 |
| 状态 | `PARTIAL_MIGRATED` |

## B02.2 MemorySet range arithmetic

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `crate/memory/src/memory_set/mod.rs:180`、`:197`、`:232`、`:246`、`:379` |
| rCore 函数 | `MemorySet::{find_free_area,push,pop,pop_with_split,handle_page_fault_ext}` 和 `MemoryArea` range helper |
| `kernel.rs` 来源 | `VmRegion::{end,contains}`，`kernel/src/kernel.rs:1007-1015`；`VmMap::insert/find/find_free`，`:1113-1213`；`AddrSpace::{unmap_range,split_region}`，`:7728-7788` |
| 迁移语义 | 所有 start/end、page round-up、addr+len 使用 checked 或 saturating 语义；半开区间 `[start,end)` 保持一致；非法/overflow range 不 wrap、不 panic。 |
| 迁移前 rCore 表达 | `find_free_area` 使用 `addr + PAGE_SIZE - 1` 和 `addr + len`；`push` 使用 `end_addr + PAGE_SIZE - 1`；`pop`/`pop_with_split` 对非法输入 assert/panic；split 路径直接构造左右区间。 |
| 已落地结果 | `MemoryArea::align_up`、`MemorySet::align_up` 使用 `checked_add`；`check_read_array` 使用 checked size/ptr end；`find_free_area` 对候选 start/end 使用 checked；`push` 对非法/overflow/重叠 range 直接返回；`pop`/`pop_with_split` 对空或反向区间直接返回；`with` 改为 `with<R>` 保留 closure 返回值。 |
| 接口处理 | 优先保持现有 public 签名；无法返回 `Result` 的函数中，最小修改应先集中在私有 checked helper 和调用前置校验，避免扩散 API。 |
| 最小修改范围 | `find_free_area`、`push`、`pop`、`pop_with_split` 的 range helper；`handle_page_fault_ext` 的 lookup 前置边界校验。 |
| 不应修改 | `MemorySet<T>` 结构体字段布局、`PageTableExt` trait、handler trait 签名。 |
| 验收点 | `len == 0` 或 `end <= start` 明确拒绝；round-up overflow 不 wrap；相邻区域不被视为重叠；split 后左右区间不重叠且不产生空区间；fault lookup 不接受越界地址。 |
| 状态 | `PARTIAL_MIGRATED` |

## B02.3 file-backed mapping offset

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `crate/memory/src/memory_set/handler/file.rs:61`、`:94` |
| rCore 函数 | `File<F,T>::handle_page_fault_ext`、`File<F,T>::fill_data` |
| `kernel.rs` 来源 | `FLike::mmap_fl`，`kernel/src/kernel.rs:2754`；file mmap/read range 语义参考 `kernel.rs` 文件读写边界记录 |
| 迁移语义 | `file_offset = addr + file_start - mem_start` 不得 overflow/underflow；非页对齐 file-backed 段必须把文件内容写入 fault page 的正确页内偏移；短读和文件外页内区域必须 zero-fill。 |
| 迁移前 rCore 表达 | `fill_data` 直接计算 `addr + self.file_start - self.mem_start`；再把 `file_end - file_offset` 转成 `isize` 求 min/max。 |
| 已落地结果 | `fill_data` 改为先整页 zero-fill；再计算 `[addr, addr + PAGE_SIZE)` 与 `[mem_start, mem_start + file_len)` 的交集；只把交集对应的 file offset 读入 `data[page_offset..]`；fault-time alloc 失败返回 false；cache flush end 使用 `addr.checked_add(read_size)`。 |
| 接口处理 | 保留 `Read::read_at(offset, buf) -> usize`；边界失败应返回 `0` read_size 或使 fault handler 返回 `false`，不新增 file handler public API。 |
| 最小修改范围 | `fill_data` 的 offset/read_size 计算和 `handle_page_fault_ext` 对分配失败/非法 offset 的处理。 |
| 不应修改 | `Read` trait、`MemoryHandler` trait、file-backed handler 字段含义。 |
| 验收点 | `addr < mem_start` 不 underflow；`addr + PAGE_SIZE` 和 `mem_start + file_len` 不 overflow；offset 超出 file-backed 内容时 zero-fill 并返回 0；非页对齐 LOAD 的 `.init_array`/`.fini_array` 能读到文件原始函数指针。 |
| 状态 | `PARTIAL_MIGRATED` |

## B02.4 kernel frame/user access

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/memory.rs:77`、`:87`、`:196`、`:205`、`:222` |
| rCore 函数 | `GlobalFrameAlloc::{alloc_contiguous,dealloc}`、`access_ok`、`copy_from_user`、`copy_to_user` |
| `kernel.rs` 来源 | `check_access`，`kernel/src/kernel.rs:1634-1642`；`check_access_rw`、`cfu`、`ctu`，`:1644-1684`；`validate_access`，`:7529-7568` |
| 迁移语义 | contiguous alloc 的 size/align 输入非法时干净失败；dealloc 目标低于 `MEMORY_OFFSET` 时不 underflow；用户指针范围使用 checked add，半开上界允许 `end == user_limit`。 |
| 迁移前 rCore 表达 | `alloc_contiguous` 直接下传 allocator；`dealloc` 使用 `target - MEMORY_OFFSET`；`access_ok` 使用 `(addr + len) < PHYSICAL_MEMORY_OFFSET`，既可能 overflow，也拒绝 `end == limit`。 |
| 已落地结果 | `alloc_contiguous` 拒绝 `size == 0` 和过大 `align_log2`，id 到物理地址使用 checked 乘加；`dealloc` 用 `checked_sub(MEMORY_OFFSET)`；`access_ok` 用 `checked_add` 并允许半开区间 `end == PHYSICAL_MEMORY_OFFSET`；`copy_from_user` / `copy_to_user` 继续依赖 `access_ok` 返回失败。 |
| 接口处理 | 保留 `FrameAllocator` trait；`alloc_contiguous` 非法输入返回 `None`；`access_ok` 返回 bool；用户拷贝非法范围保持 `None`/`false`。 |
| 最小修改范围 | `GlobalFrameAlloc::{alloc_contiguous,dealloc}` 的前置校验；`access_ok` 的 checked add；`copy_from_user`/`copy_to_user` 只依赖更新后的 `access_ok`。 |
| 不应修改 | `read_user_fixup` ABI、copy_user section、arch paging re-export。 |
| 验收点 | `len == 0` 按半开区间语义可通过；`addr.checked_add(len).is_none()` 返回 false；`end == PHYSICAL_MEMORY_OFFSET` 可通过；`target < MEMORY_OFFSET` 不 underflow；非法 contiguous alloc 返回 `None`。 |
| 状态 | `PARTIAL_MIGRATED` |

## B02.5 tick conversion

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/trap.rs:30` |
| rCore 函数 | `uptime_msec` |
| `kernel.rs` 来源 | `up_ms`，`kernel/src/kernel.rs:5179-5181` |
| 迁移语义 | `wall_tick * USEC_PER_TICK / 1000` 使用 saturating 或 checked arithmetic；时间值应保持单调，不因乘法溢出 wrap。 |
| 迁移前 rCore 表达 | `unsafe { wall_tick() * USEC_PER_TICK / 1000 }` 直接乘法。 |
| 已落地结果 | `unsafe { wall_tick().saturating_mul(USEC_PER_TICK) / 1000 }`。 |
| 接口处理 | 保留 `uptime_msec() -> usize`；优先使用 `saturating_mul`，不新增错误返回。 |
| 最小修改范围 | `uptime_msec` 单函数。 |
| 不应修改 | `wall_tick`、`do_tick`、`timer`、`NAIVE_TIMER` 调用面。 |
| 验收点 | 大 tick 下不 wrap；返回值随 tick 单调不减；常规 tick 与原公式一致。 |
| 状态 | `MIGRATED` |

## B02.6 init-time user stack VM 写入

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/process/abi.rs:19`、`:63`、`:92`、`:118`；`kernel/src/process/thread.rs:223` |
| rCore 函数 | `ProcInitInfo::{push_at,push_at_in_vm,push_to}`、`VmStackWriter::write_bytes`、`Thread::new_user_vm` |
| `kernel.rs` 来源 | `ProcInit::push_at`，`kernel/src/kernel.rs:4387-4470` |
| 迁移语义 | init-time 用户栈写入不能依赖已有 `current_thread()`；写入用户 VA 前必须在正在构造的 `MemorySet` 中准备页，并通过页表 backing frame 写入。 |
| 迁移前 rCore 表达 | `Thread::new_user_vm` 在 `vm.with(|| init_info.push_at(ustack_top))` 中直接写用户虚拟地址；page fault facade 只知道 current thread VM。 |
| 已落地结果 | 新增 `InitStackWriter` 抽象和 `VmStackWriter`；真实 exec 初始化路径调用 `push_at_in_vm(vm, ustack_top)`；每段写入前调用 `vm.handle_page_fault_ext(page_start, AccessType::write(true))`，随后通过 `get_page_table_mut().get_page_slice_mut(page_start)` 写入并 flush。 |
| 接口处理 | 保留原 `push_at(stack_top) -> usize` 作为旧 direct writer；新增 VM writer 不扩散到 syscall facade。 |
| 后续边界 | VM writer 的 stack overflow/page prepare 错误传播已在 B05.1 收口；B02 后续仅按新的 memory/trap 运行边界单独报告。 |
| 状态 | `PARTIAL_MIGRATED` |

## B02.7 RISC-V PTE update/TLB flush

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `kernel/src/arch/riscv/paging.rs:37-48`、`:81-85`、`:156-193` |
| rCore 函数 | `PageTableImpl::map`、`PageEntry::update`、`PageTableImpl::normalize_intermediate_entries` |
| `kernel.rs` 来源 | page fault 后 PTE 状态应对执行硬件可见的语义；`kernel.rs` 模拟页表无真实 TLB，但其 fault 成功后立即可重试。 |
| 运行症状 | VM-backed init stack 修复后，QEMU 进入用户态入口 `0x100e8`，但反复 instruction page fault。 |
| 根因/当前表达 | 第一层问题是 `riscv::asm::sfence_vma` wrapper 的第一个参数对应 `rs1`/vaddr，原实现传入 frame 物理地址；第二层问题是 riscv crate 的 `PTE::set()` 会无条件加入 `A|D`，导致 `map_to()` 创建的 Sv39 非 leaf PTE 变成 `V|A|D`，硬件页表遍历在到达 leaf 前 fault。 |
| 已落地结果 | `PageEntry::update` 改为 `sfence_vma(self.1.start_address().as_usize(), 0)`；`PageTableImpl::map` 后调用 `normalize_intermediate_entries(page)`；该函数只对映射路径上 `VALID` 且无 `R/W/X` 的非 leaf PTE 清除 `A/D/U`，不改变 leaf 页和内核大页映射。 |
| 验收结果 | QEMU monitor 曾确认修复前 leaf PTE `0x80eda080 = 0x203b80db`，上级非 leaf 为 `0x203b64c1`、`0x203b68c1`；修复后 `0x100e8` 只触发一次 demand fault，用户程序继续执行到后续 syscall。 |
| 状态 | `MIGRATED` |

## B02.8 file-backed 非页对齐 ELF LOAD

| 项目 | 内容 |
| --- | --- |
| 目标位置 | `crate/memory/src/memory_set/handler/file.rs:99-132` |
| rCore 函数 | `File<F,T>::fill_data` |
| `kernel.rs` 来源 | `FLike::mmap_fl` 和 file-backed mapping 的 file offset/zero-fill 语义。 |
| 运行症状 | `0x100e8` 修复后，用户态在 `sepc=0x0` 触发 instruction page fault。反汇编显示 `0xbdc42: ld a5,0(s0)`、`0xbdc46: jalr a5`，其中 `s0` 指向 `.init_array/.fini_array`。 |
| 根因/当前表达 | `busybox` 第二个 LOAD 段从 `vaddr=0x10bff0` 开始；fault page 是 `0x10b000`。旧逻辑要求 `addr >= mem_start`，否则直接 zero-fill 整页，导致 `.init_array` 的 `0x101be` 和 `.fini_array` 的 `0x10182` 被清零。 |
| 已落地结果 | `fill_data` 对整页先清零，再计算 fault page 与 file-backed 段的交集；对交集执行 `read_at(file_offset, &mut data[page_offset..])`，使非页对齐 LOAD 的文件内容落入正确页内偏移。 |
| 验收结果 | QEMU debug 运行进入 busybox shell `/ #`；日志没有 `page fault from user @ 0x0` 和 panic。 |
| 状态 | `MIGRATED` |

## 接口边界

| `kernel.rs` 结构 | rCore 落点 | 处理方式 |
| --- | --- | --- |
| `VmMap`、`VmRegion` | `rcore_memory::memory_set::{MemorySet,MemoryArea,MemoryAttr}` | 不复制模拟 VM 容器；只迁移 range checked arithmetic 和半开区间不变量。 |
| `PgFrame`、`SharedPage` | `CowExt`、`FrameRcMap`、`SharedGuard` | 迁移 refcount 不 underflow 和 COW fault 生命周期。 |
| `FramePool` | `GlobalFrameAlloc`、`FrameAllocator` trait | 保持 allocator trait；非法输入返回 `None` 或内部拒绝。 |
| `check_access`、`validate_access`、`cfu`、`ctu` | `access_ok`、`copy_from_user`、`copy_to_user` | 保持 rCore 用户拷贝 API，只对齐 checked range 和错误返回语义。 |
| `up_ms` | `uptime_msec` | 保持 `usize` 返回，使用 saturating/checked conversion。 |

## 批次内顺序与实际进度

1. `uptime_msec` 已迁移为 saturating conversion。
2. `access_ok`、用户拷贝前置校验和 `GlobalFrameAlloc` 边界已部分迁移。
3. `File<F,T>::fill_data` 的 file offset 和非页对齐 LOAD 页内填充已迁移。
4. `MemorySet` range helper、push/pop/split 基础边界已部分迁移。
5. COW refcount 增减边界已部分迁移。
6. init-time 用户栈写入已通过 VM writer 耦合到 `MemorySet`。
7. RISC-V `PageEntry::update` 的 `sfence.vma` 参数修正和 Sv39 非 leaf PTE flags 归一化已完成，`0x100e8` 重复 instruction page fault 已解除。
8. file-backed 非页对齐 ELF LOAD 页内填充已完成，`.init_array` 被清零导致的 `sepc=0x0` 跳转 fault 已解除。

## 风险和验收

| 风险 | 验收方式 |
| --- | --- |
| 改 `MemorySet` public 签名导致上游调用面扩散 | 优先用私有 helper 或保持 panic 点之外的前置校验；确需改签名时单独报批。 |
| `access_ok` 边界从 `<` 改为 `<=` 影响用户空间上界 | 按半开区间 `[addr, addr + len)` 验证，只允许 `end == PHYSICAL_MEMORY_OFFSET`。 |
| COW refcount 清理改变共享页生命周期 | 逐路径核对 map/unmap/page fault；只清理当前 frame 的当前计数。 |
| file-backed page fault offset 失败时行为不明确 | 失败路径明确为 zero-fill/0 read；非页对齐 LOAD 只读取 fault page 与文件段的交集，不能清零有效页内数据。 |

## 后续源码修改报告边界

B02 不再处于“整批等待批准”的旧状态。本轮已按行级报告完成 `0x100e8` 和 `sepc=0x0` 两个运行边界修复。B03、B04 和 B05 核心子项后续也已收口；如果继续修改源码，仍必须围绕新的具体故障或具体子模块语义差距重新报告。
