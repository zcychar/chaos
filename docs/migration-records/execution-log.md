# 迁移执行日志

本文记录按方案推进的每个步骤。源码语义修改仍遵守“先报告、等批准、再修改”的流程；B01 已按用户批准执行，B02 当前为部分迁移并已通过 QEMU 启动到 busybox shell。

## 2026-06-06：Phase 1 恢复 rCore 基线

| 项目 | 记录 |
| --- | --- |
| 操作 | 从上游 commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` 恢复缺失模块面。 |
| 恢复路径 | `crate/memory/`、`kernel/src/memory.rs`、`kernel/src/trap.rs`、`kernel/src/fs/`、`kernel/src/ipc/`、`kernel/src/process/`、`kernel/src/sync/`、`rust-toolchain`。 |
| 是否修改 `kernel.rs` | 否。 |
| 是否修改恢复源码语义 | 否，按上游原样恢复。 |
| 文件数 | 上游新增文件 50 个。 |
| 校验证据 | 本地恢复文件与上游 `ls-tree` 对象逐文件 `git hash-object` 比对，无 mismatch。 |
| 当前状态 | `BASELINE_RESTORED`。 |

## 2026-06-06：建立分层迁移记录

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 `docs/migration-records/` 文档树。 |
| 覆盖层级 | L0 恢复批次、L1 顶层路径、L2 子目录、L3 文件、L4 子模块-功能。 |
| 覆盖性校验 | 所有恢复目录和 50 个恢复文件均能在 `docs/migration-records/` 中查到记录。 |
| 当前状态 | `MIGRATION_PENDING` 项已按模块登记；源码迁移尚未开始。 |

## 2026-06-06：补充 L4 子模块-功能索引

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [function-index.md](function-index.md)。 |
| 覆盖范围 | 每个新增 Rust 文件的 public API、关键内部结构、trait、impl 功能块，以及 `rust-toolchain` 的基线记录。 |
| 覆盖性校验 | 50 个恢复文件均能在 `restored-tree.md` 或 `function-index.md` 中查到；新增目录均能在记录体系中查到。 |
| hash 校验 | 恢复文件与上游对象再次逐文件比对，无 mismatch。 |
| 当前状态 | 迁移记录已覆盖到 L4 子模块-功能级；源码迁移仍等待具体迁移项批准。 |

## 2026-06-06：整理迁移批次执行单

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [migration-batches.md](migration-batches.md)。 |
| 覆盖范围 | sync/futex、memory/trap、fs、ipc、process，以及 no-direct/deferred 项。 |
| 记录字段 | 目标文件/行、`kernel.rs` 来源、迁移语义、当前 rCore 表达、预期对齐结果、最小修改范围、状态。 |
| 当前状态 | 所有批次均为记录和报批材料，尚未进行源码语义修改。 |

## 2026-06-06：补充完成度审计和批准队列

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [completion-audit.md](completion-audit.md) 和 [approval-queue.md](approval-queue.md)。 |
| 覆盖范围 | 用户目标逐项证据、恢复文件覆盖、未完成源码迁移项、建议批准顺序。 |
| 当前状态 | 记录体系已经覆盖恢复文件/文件夹和子模块-功能；源码语义迁移等待具体批次批准。 |

## 2026-06-06：补充逐路径可追踪矩阵

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [path-traceability.md](path-traceability.md)。 |
| 覆盖范围 | 12 个工作区新增目录，包括 `crate/` 容器目录；50 个恢复文件；每个路径均记录层级、模块文档、关联迁移批次和当前状态。 |
| 目标调整 | 明确后续不再把“找接口阻塞”作为独立阶段；重点改为按路径和批次把 `kernel.rs` 语义正确对齐到 rCore 恢复模块。 |
| 校验证据 | 文件覆盖、目录覆盖、上游 hash 三组命令均无输出。 |
| 当前状态 | 路径级追踪材料已补齐；源码语义迁移仍等待具体批次批准。 |

## 2026-06-06：补充符号级覆盖记录

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [symbol-coverage.md](symbol-coverage.md)。 |
| 覆盖范围 | 48 个恢复 Rust 文件；其中 46 个存在可提取类型/trait/impl/函数/常量入口，2 个为纯模块边界文件。 |
| 记录目的 | 为“每个子模块-功能单独迁移记录”提供符号级覆盖证据，并指向 [function-index.md](function-index.md) 的 L4 记录。 |
| 校验证据 | Rust 文件计数为 48；符号记录反查命令无 `MISSING_SYMBOL_RECORD` 输出。 |
| 当前状态 | 子模块-功能覆盖证据已补强；源码语义迁移仍等待具体批次批准。 |

## 2026-06-06：补充 B01 函数级批次记录

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [batches/B01-sync-futex.md](batches/B01-sync-futex.md)。 |
| 覆盖范围 | `Condvar::wait_events`、`Condvar::wait_timeout`、`Futex::wake`、`FutexFuture::poll`。 |
| 记录来源 | `kernel/src/kernel.rs:566-595`、`:605-618`、`:835-853`，以及 `kernel/src/sync/condvar.rs`、`kernel/src/process/futex.rs` 当前恢复表达。 |
| 当前状态 | B01 源码修改前报告材料已补齐；随后已按批准执行源码迁移。 |

## 2026-06-06：执行 B01 sync + futex 源码迁移

| 项目 | 记录 |
| --- | --- |
| 操作 | 按 [batches/B01-sync-futex.md](batches/B01-sync-futex.md) 执行 B01 源码迁移。 |
| 修改文件 | `kernel/src/process/futex.rs`、`kernel/src/sync/condvar.rs`。 |
| 子模块-功能 | `Futex::wake`、`FutexFuture::poll`、`Condvar::wait_events`、`Condvar::wait_timeout`。 |
| `kernel.rs` 来源 | `SyncQueue::wait_events`、`SyncQueue::wait_timeout`、`FutexTable::ftx_wake`。 |
| 接口处理 | 保留 rCore public API，不复制 `kernel.rs` 的 `std::thread::park/unpark` 模型。 |
| 主要结果 | futex timeout 后清理 waiter；wake 只统计有效 waiter；condvar wait path 使用 `current_thread().tid` 注册/去重/清理。 |
| 格式化 | `rustfmt kernel/src/process/futex.rs kernel/src/sync/condvar.rs` 已完成。 |
| 构建验证 | `make build ARCH=riscv64 objcopy=/usr/bin/llvm-objcopy` 已通过，生成 `target/riscv64/release/kernel.img`。 |
| 运行验证 | 生成 `user/build/riscv64.qcow2` 后执行 20 秒 QEMU 冒烟；进入 OpenSBI/rCore 后在 B02 memory 初始化路径 stack trace。 |
| 上游差异 | `kernel/src/process/futex.rs`、`kernel/src/sync/condvar.rs` 相对上游产生有意差异；后续 hash 校验需把它们列为 B01 已迁移例外。 |
| 当前状态 | `MIGRATED`。 |

## 2026-06-06：补充 B02 函数级/子模块级批次记录

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [batches/B02-memory-trap.md](batches/B02-memory-trap.md)。 |
| 覆盖范围 | COW refcount、MemorySet range arithmetic、file-backed mapping offset、kernel frame/user access、tick conversion。 |
| 记录来源 | `kernel/src/kernel.rs:916-982`、`:1007-1213`、`:1566-1684`、`:5179-5181`、`:7529-7568`、`:7728-7788`，以及 B02 恢复源码当前表达。 |
| 当前状态 | B02 源码修改前报告材料已补齐；随后工作区已进入 B02 部分源码迁移。 |

## 2026-06-06：执行 B02 memory + trap 部分源码迁移

| 项目 | 记录 |
| --- | --- |
| 操作 | 按 [batches/B02-memory-trap.md](batches/B02-memory-trap.md) 中的 memory/trap 边界项执行部分源码迁移，并补入 init-time stack 的 `MemorySet` 耦合。 |
| 修改文件 | `kernel/src/memory.rs`、`kernel/src/trap.rs`、`crate/memory/src/memory_set/mod.rs`、`crate/memory/src/memory_set/handler/file.rs`、`crate/memory/src/memory_set/handler/byframe.rs`、`crate/memory/src/cow.rs`、`kernel/src/process/abi.rs`、`kernel/src/process/thread.rs`。 |
| 子模块-功能 | active `MemorySet` fault fallback、frame alloc/dealloc checked 边界、user access checked range、uptime saturating conversion、MemorySet range helper、file-backed offset/read size、ByFrame present/access check、COW refcount 增减边界、VM-backed init stack writer。 |
| `kernel.rs` 来源 | `PgFrame`、`SharedPage::fault`、`VmRegion`、`VmMap`、`AddrSpace`、`FramePool`、`check_access`、`validate_access`、`cfu`、`ctu`、`up_ms`、`ProcInit::push_at`。 |
| 接口处理 | 保留 rCore `MemorySet`、`MemoryHandler`、`FrameAllocator`、copy-user、`ProcInitInfo` 主调用面；新增 `push_at_in_vm` 作为 exec 初始化路径的内部耦合点，不把 `kernel.rs` 模拟 VM 类型搬入 rCore。 |
| 主要结果 | 旧的 init stack 直接写用户 VA 路径被替换为 VM-backed writer；`0x3fffffff` 初始化栈 fault 已不再出现，启动能进入 `process: init end` 和 `Hello RISCV!`。 |
| 当前状态 | `PARTIAL_MIGRATED`。 |

## 2026-06-06：执行已批准的 RISC-V `sfence.vma` 修正

| 项目 | 记录 |
| --- | --- |
| 操作 | 修正 `kernel/src/arch/riscv/paging.rs::PageEntry::update` 的 `sfence_vma` 参数。 |
| 修改文件 | `kernel/src/arch/riscv/paging.rs`。 |
| 根因 | 当前 riscv crate wrapper 把第一个参数传给 `sfence.vma` 的 `rs1`，原实现传入 frame 物理地址，不能精确刷新 fault page 的虚拟地址。 |
| 已落地结果 | 调用改为 `sfence_vma(self.1.start_address().as_usize(), 0)`，并保留注释说明 wrapper 参数含义。 |
| 格式化 | `rustfmt kernel/src/arch/riscv/paging.rs` 已完成。 |
| 构建验证 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 已通过，只有既有 warning。 |
| 运行验证 | 后续 monitor 证据确认该修正必要但不足：leaf PTE 已正确，非 leaf PTE 带 `A/D` 导致硬件页表遍历 fault。 |
| 当前状态 | `MIGRATED`，后续运行边界另行定位。 |

## 2026-06-06：定位并修复 B02 `0x100e8` 运行边界

| 项目 | 记录 |
| --- | --- |
| 操作 | 用 trace/QEMU MMU/monitor 日志确认 B02 后续边界，并按用户确认的最小方案修复。 |
| 现象 | `busybox` ELF entry point 为 `0x100e8`，第一个 LOAD 段从 `0x10000` 开始且权限为 `R E`；运行时反复触发 `cause=0xc` instruction page fault。 |
| 内核侧证据 | trace 日志显示首次 fault 分配 frame，后续重复 fault 不再分配，说明 rCore handler 认为 PTE 已 present 且 access-ok。 |
| 硬件侧证据 | QEMU MMU 日志显示 `address=100e8 ret 1 physical 0000000000000000 prot 0`，硬件页表遍历仍未看到有效可执行映射。 |
| monitor 证据 | `satp = 0x8000000000080ed8`；leaf PTE `0x80eda080 = 0x203b80db` 已是 `V|R|X|U|A|D`，但上两级非 leaf PTE 为 `0x203b64c1`、`0x203b68c1`，即 `V|A|D`。 |
| 根因 | riscv crate 的 `PTE::set()` 会为所有 PTE 加 `A|D`，但 Sv39 非 leaf PTE 不应带 `A/D/U`。 |
| 修改文件 | `kernel/src/arch/riscv/paging.rs`。 |
| 已落地结果 | `PageTableImpl::map` 后调用 `normalize_intermediate_entries(page)`，只清理映射路径上非 leaf PTE 的 `A/D/U`；保留 leaf PTE 和内核大页映射。 |
| 构建验证 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过，仅有既有 warning。 |
| 运行验证 | `0x100e8` 不再重复，用户程序继续执行到 `set_tid_address` 和后续缺页。 |
| 当前状态 | `MIGRATED`。 |

## 2026-06-06：定位并修复 B02 file-backed 非页对齐 LOAD

| 项目 | 记录 |
| --- | --- |
| 操作 | 用 trace 日志和 `busybox` ELF/反汇编确认 `sepc=0x0` instruction fault，并按用户确认的最小方案修复。 |
| 现象 | `0x100e8` 修复后，用户态在 `sepc=0x0` instruction page fault；反汇编显示 `0xbdc42: ld a5,0(s0)`、`0xbdc46: jalr a5`，从 `.init_array/.fini_array` 读函数指针。 |
| ELF 证据 | 第二个 LOAD 段 `Offset=0x0faff0`、`VirtAddr=0x10bff0`，不按页对齐；`.init_array` 原始内容为 `0x101be`，`.fini_array` 为 `0x10182`。 |
| 根因 | 旧 `File::fill_data` 对 fault page `0x10b000` 执行 `addr.checked_sub(mem_start)` 失败后清零整页，误清掉页尾的 `.init_array/.fini_array`。 |
| 修改文件 | `crate/memory/src/memory_set/handler/file.rs`。 |
| 已落地结果 | `fill_data` 先整页 zero-fill，再计算 fault page 与 file-backed 段的交集，并把对应 file offset 读入页内正确 offset。 |
| 构建验证 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过，仅有既有 warning。 |
| 运行验证 | `/tmp/chaos-qemu-b02-after-file-load.log` 显示进入 busybox shell `/ #`；未再出现 `page fault from user @ 0x0` 或 panic。 |
| 当前状态 | `MIGRATED`。 |

## 2026-06-06：补充 B03 函数级/子模块级批次记录

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [batches/B03-fs.md](batches/B03-fs.md)。 |
| 覆盖范围 | append/seek/write range、mmap/munmap range、epoll state、pipe endpoint lifecycle、TTY/channel wakeup。 |
| 记录来源 | `kernel/src/kernel.rs:2241-2370`、`:2488-2785`、`:2907-3214`、`:6430-6505`，以及 B03 恢复源码当前表达。 |
| 当前状态 | B03 源码修改前报告材料已补齐；仍等待用户明确批准后才能修改源码。 |

## 2026-06-06：补充 B04 函数级/子模块级批次记录

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [batches/B04-ipc.md](batches/B04-ipc.md)。 |
| 覆盖范围 | semaphore create/get、semaphore undo/remove、shared memory key/size。 |
| 记录来源 | `kernel/src/kernel.rs:4158-4386`，以及 B04 恢复源码当前表达。 |
| 当前状态 | B04 源码修改前报告材料已补齐；仍等待用户明确批准后才能修改源码。 |

## 2026-06-06：补充 B05 函数级/子模块级批次记录

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [batches/B05-process.md](batches/B05-process.md)。 |
| 覆盖范围 | user stack init、fork/wait parent-child、fd lifecycle/cloexec、ELF bounds。 |
| 记录来源 | `kernel/src/kernel.rs:1950-2022`、`:4387-4470`、`:5736-5801`、`:5814-5968`、`:6700-6785`、`:7436-7475`，以及 B05 恢复源码当前表达。 |
| 当前状态 | B05 源码修改前报告材料已补齐；仍等待用户明确批准后才能修改源码。 |

## 2026-06-06：补充 B06 no-direct 批次记录

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [batches/B06-no-direct.md](batches/B06-no-direct.md)。 |
| 覆盖范围 | swap、mock page table、random、fbdev、rust-toolchain。 |
| 记录目的 | 明确这些路径保持上游基线或作为验证参考，不从 `kernel.rs` 迁移同名模拟结构。 |
| 当前状态 | B06 no-direct 记录已补齐，不需要源码修改。 |

## 2026-06-06：补充最终交付入口

| 项目 | 记录 |
| --- | --- |
| 操作 | 新增 [final-delivery.md](final-delivery.md)。 |
| 覆盖范围 | 汇总 L0-L4 记录、B01-B06 批次记录、验证结果和下一步源码迁移批准点。 |
| 当前状态 | 中文交付入口已补齐；B01 源码语义迁移已执行，B02 已部分迁移并进入运行边界；后续已继续完成 B03 append/seek、mmap range、epoll state、pipe endpoint 和 B05 brk 子项，其余批次仍等待具体行级报告。 |

## 2026-06-06：同步 B02 中文交付文档并复核构建

| 项目 | 记录 |
| --- | --- |
| 操作 | 将 B02 的实际源码迁移状态、init stack 耦合、RISC-V `sfence.vma` 修正和 `0x100e8` 运行边界同步到中文交付文档。 |
| 更新文档 | [batches/B02-memory-trap.md](batches/B02-memory-trap.md)、[migration-batches.md](migration-batches.md)、[execution-log.md](execution-log.md)、[path-traceability.md](path-traceability.md)、[function-index.md](function-index.md)、[modules/kernel-memory-trap.md](modules/kernel-memory-trap.md)、[modules/crate-memory.md](modules/crate-memory.md)、[modules/kernel-process.md](modules/kernel-process.md)、[restored-tree.md](restored-tree.md)、[completion-audit.md](completion-audit.md)、[final-delivery.md](final-delivery.md)、[approval-queue.md](approval-queue.md)、[README.md](README.md)。 |
| 构建验证 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；仅有既有 warning。 |
| 文档校验 | `git diff --check -- docs/migration-records` 无输出。 |
| 当前状态 | 此条记录是旧边界同步点；后续已完成 `0x100e8` 和 file-backed 非页对齐 LOAD 修复，当前运行验证已进入 busybox shell。 |

## 下一批待批准迁移项

当前不建议再批准“整个 B02”。B02 已部分迁移并已通过 QEMU shell 运行验证；下一步应按新的具体故障或明确子模块语义差距继续做行级报告。B03 append/seek、mmap range、epoll state、pipe endpoint、TTY/channel wakeup，B04 IPC，以及 B05 核心 process 子项均已完成；旧的 B03-B05 批次队列已收口。

### 候选 1：B02 运行边界继续定位

- 目标现象：当前无 B02 立即运行 panic；QEMU 已进入 busybox shell。
- 候选文件：后续按新故障或 B03-B05 批次重新确定。
- 当前证据：`/tmp/chaos-qemu-b02-after-file-load.log` 到达 `/ #`。
- 预期对齐结果：继续把 FS/IPC/process 等恢复模块与 `kernel.rs` 语义对齐。
- 状态：`NEXT_BATCH_PENDING`。

## 2026-06-06：执行 B05 process brk / heap boundary 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | QEMU 已进入 busybox shell，但启动期间多次出现 `brk is unimplemented`，当前 `SYS_BRK` 返回 `ENOMEM`。 |
| 行级报告 | 目标文件为 `kernel/src/syscall/mod.rs:241`、`kernel/src/syscall/mem.rs:11`、`kernel/src/process/proc.rs:73`、`kernel/src/process/thread.rs:339`/`:384`；根因是恢复后的 `Process` 没有 program break 状态，syscall 分发仍走 unimplemented。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:6509-6540` 的 `SYS_BRK` 分支和 `kernel/src/kernel.rs:1100-1108` 的 `VmMap::brk`。 |
| 接口处理 | 不复制 `kernel.rs` 的模拟 `VmMap`；在 rCore `Process` 中记录 `brk_start/brk`，通过 `MemorySet`、`Delay` 和 `GlobalFrameAlloc` 建立 lazy heap mapping。 |
| 已修改源码 | `kernel/src/process/proc.rs`、`kernel/src/process/thread.rs`、`kernel/src/syscall/mod.rs`、`kernel/src/syscall/mem.rs`。 |
| 验证 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；`/tmp/chaos-qemu-brk.log` 进入 `/ #`，无 `brk is unimplemented`，出现 `0x400010/0x401ff0/0x403ff0` 等 heap 页 demand fault。 |
| 后续边界 | 当前仍有 `getuid/geteuid is unimplemented` warning；这属于后续 syscall/process credential 兼容性，不并入本次 brk 修复。 |

## 2026-06-06：执行 B03.1 append write and negative seek 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | B03 文档和源码对照确认 `FileHandle::write` append 后 offset 更新基于旧 descriptor offset，`seek` 对负结果直接 cast 到 `u64`。 |
| 行级报告 | 目标文件为 `kernel/src/fs/file.rs:139`、`:151`、`:160` 和 `kernel/src/syscall/fs.rs:764`；根因是恢复后的 file offset 路径仍使用直接加法/cast，没有吸收 `kernel.rs` checked offset 语义。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:2319-2357` 的 `FHandle::{write,write_at,seek}`。 |
| 接口处理 | 保留 `FileHandle`、`OpenFileDescription`、`SeekFrom` 和 syscall ABI；非法 offset 使用现有 `FsError::InvalidParam` / `SysError::EINVAL`。 |
| 已修改源码 | `kernel/src/fs/file.rs`、`kernel/src/syscall/fs.rs`。 |
| 验证 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；`/tmp/chaos-qemu-b03-file.log` 进入 `/ #`，无 panic 或 `page fault from user @ 0x0`。 |
| 后续边界 | B03.3 epoll state、B03.4 pipe endpoint 和 B03.5 TTY/channel wakeup 后续均已完成。 |

## 2026-06-06：执行 B03.2 mmap file range 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | B03 文档和源码对照确认 `sys_mmap`、`sys_munmap`、`FileHandle::mmap` 多处直接使用 `addr + len` 或 `offset + end - start`。 |
| 行级报告 | 目标文件为 `kernel/src/syscall/mem.rs:62`、`:148` 和 `kernel/src/fs/file.rs:241`；根因是恢复后的 mmap range 路径没有吸收 `kernel.rs` checked range 语义。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:6430-6505` 的 `SYS_MMAP/SYS_MUNMAP` 和 `kernel/src/kernel.rs:2754-2775` 的 `FLike::mmap_fl`。 |
| 接口处理 | 保留 `MmapProt`、`MmapFlags`、`MMapArea`、`MemorySet` public API；非法 range 使用现有 `SysError::EINVAL` / `FsError::InvalidParam`。 |
| 已修改源码 | `kernel/src/syscall/mem.rs`、`kernel/src/fs/file.rs`。 |
| 验证 | `make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；`/tmp/chaos-qemu-b03-mmap.log` 进入 `/ #`，无 panic 或 `page fault from user @ 0x0`。 |
| 后续边界 | B03.3 epoll state、B03.4 pipe endpoint 和 B03.5 TTY/channel wakeup 后续均已完成；`sys_mprotect` 仍有同类 `addr + len` 表达，但没有直接 `kernel.rs` syscall 对应语义，本轮不混入。 |

## 2026-06-06：执行 B03.3 epoll state 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | 源码对照确认 `EpollInstance::clone` 返回空实例，ADD 覆盖已有 fd，DEL 只清理 `events`，`sys_epoll_ctl` 对 DEL 也读取 event 指针，`epoll_pwait` 写回未受 `maxevents` 保护。 |
| 行级报告 | 目标文件为 `kernel/src/fs/epoll.rs:10`、`:16`、`:35` 和 `kernel/src/syscall/fs.rs:318`、`:364`；根因是恢复后的 epoll 状态没有吸收 `kernel.rs` 中 `EpInst` 共享状态和 control lifecycle。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:2603-2627` 的 `FLike::dup` epoll 分支和 `kernel/src/kernel.rs:2907-2950` 的 `EpInst::control`。 |
| 接口处理 | 保留 `FileLike::EpollInstance`、`EpollEvent` ABI 和 `Process::get_epoll_instance*` 调用面；共享语义落在 `EpollInstance` 内部 `Arc<SpinNoIrqLock<_>>` 状态。 |
| 已修改源码 | `kernel/src/fs/epoll.rs`、`kernel/src/syscall/fs.rs`。 |
| 验证 | `rustfmt --edition 2018 kernel/src/fs/epoll.rs kernel/src/syscall/fs.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 后续边界 | B03.4 pipe endpoint 和 B03.5 TTY/channel wakeup 后续均已完成；`epoll_pwait` 内部 file callback unregister 分支存在新的独立风险，未纳入本次已批准补丁，后续若触发需单独报告。 |

## 2026-06-06：执行 B03.4 pipe endpoint lifecycle 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | 源码对照确认 `Pipe` 只用 `end_cnt`，无法区分 reader/writer；读端关闭后 `write_at` 仍可写入 buffer 并返回成功；`poll.error` 固定为 false。 |
| 行级报告 | 目标文件为 `kernel/src/fs/pipe.rs:22`、`:36`、`:52`、`:90`、`:100`、`:141`，以及 `kernel/src/fs/file.rs:94`、`kernel/src/fs/file_like.rs:39`；根因是恢复后的 pipe endpoint 生命周期没有吸收 `kernel.rs` 的 reader/writer 独立计数。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:2480-2594` 的 `PipeBuf`、`PipeNode::{clone,drop,can_read,can_write,read_at,write_at,poll}`。 |
| 接口处理 | 保留 `INode for Pipe`、`Pipe::create_pair` 和 `FileLike::File` pipe 表达；在 pipe 内部维护 `readers/writers`。`rcore-fs::FsError` 没有 `BrokenPipe`，所以 syscall 可见的 broken pipe 在 `FileLike::write` 层桥接为 `SysError::EPIPE`。 |
| 已修改源码 | `kernel/src/fs/pipe.rs`、`kernel/src/fs/file.rs`、`kernel/src/fs/file_like.rs`。 |
| 验证 | `rustfmt --edition 2018 kernel/src/fs/pipe.rs kernel/src/fs/file.rs kernel/src/fs/file_like.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 后续边界 | B03.5 TTY/channel wakeup 后续已完成；pipe async_poll 的旧 `subscribe` 路径仍可作为独立风险在后续真实触发时单独报告，不混入本次 TTY 子项。 |

## 2026-06-06：执行 B03.5 TTY/channel wakeup 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | 源码对照确认 `TtyINode::read_at` 在有数据时只写 `buf[0]`，零长度读会越界；`TtyINode::async_poll` pending 时每次都向 `EventBus` 注册 callback；`kernel.rs::Channel` 支持按缓冲区取走已有字节并按事件唤醒等待者。 |
| 行级报告 | 目标文件为 `kernel/src/fs/devfs/tty.rs:84`、`:122` 和 `kernel/src/sync/event_bus.rs:85`；根因是恢复后的 TTY 仍是单字节读接口表达，事件等待没有同一 waker/mask 去重。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:2992-3214` 的 `Channel::{recv,send,send_batch,close}`。 |
| 接口处理 | 不新增 `Channel` 模块；保留 `TtyINode`、`INode` trait、TTY ioctl ABI 和 `EventBus::subscribe` 旧接口；新增 `EventBus::subscribe_waker` 只承载 B03.5 需要的去重等待。 |
| 已修改源码 | `kernel/src/fs/devfs/tty.rs`、`kernel/src/sync/event_bus.rs`。 |
| 验证 | `rustfmt --edition 2018 kernel/src/sync/event_bus.rs kernel/src/fs/devfs/tty.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 后续边界 | B03.1-B03.5 已完成；后续 B04 IPC 与 B05.3 fd lifecycle 均已完成，新的运行边界再单独报告。 |

## 2026-06-06：执行 B04.1 semaphore create/get 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | 源码对照确认 `sys_semget` 只拒绝 `nsems > SEMMSL`，`SemArray::get_or_create` 允许 zero-length array，并且 existing key 直接返回已有 array，不检查请求的 `nsems` 是否超过已有 `nsems`。 |
| 行级报告 | 目标文件为 `kernel/src/ipc/semary.rs:95`、`:117` 和 `kernel/src/syscall/ipc.rs:15`；根因是恢复后的 semaphore create/get 路径没有吸收 `kernel.rs::SemArr::get_or_create` 的 zero-size 与 existing-key count 边界。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:4209-4260` 的 `SemArr::get_or_create`。 |
| 接口处理 | 保留 `SemArray::get_or_create(...) -> Result<Arc<Self>, SysError>`、`SemidDs`、`IpcPerm` 和全局 `KEY2SEM` 表达；非法输入统一使用现有 `SysError::EINVAL`，existing + `IPC_CREAT | IPC_EXCL` 继续使用 `EEXIST`。 |
| 已修改源码 | `kernel/src/ipc/semary.rs`、`kernel/src/syscall/ipc.rs`。 |
| 验证 | `rustfmt --edition 2018 kernel/src/ipc/semary.rs kernel/src/syscall/ipc.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 后续边界 | B04.2 semaphore undo/remove 后续已完成；B04.3 shared memory key/size 仍待后续。 |

## 2026-06-07：执行 B04.2 semaphore undo/remove 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | 源码对照确认 `SemProc::remove` 只移除 array，不清理该 id 的 undo；`Drop for SemProc` 直接索引 `self.arrays[&id]`，遇到 stale id 会 panic；undo 值只支持 `1/0`，更大正向值走 `unimplemented!`。 |
| 行级报告 | 目标文件为 `kernel/src/ipc/mod.rs:47`、`:80` 和 `kernel/src/syscall/ipc.rs:56`；根因是恢复后的 per-process semaphore context 没有吸收 `kernel.rs::SemCtx::{remove,drop}` 的 stale cleanup 与完整 undo replay。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:4278-4312` 的 `SemCtx::{remove,drop}`。 |
| 接口处理 | 保留 `SemProc` public 方法、`Process.semaphores` 和 `sys_semctl` ABI；`IPC_RMID` 仍通过现有 `SemProc::remove(id)` 获得清理语义；undo replay 继续使用 `Semaphore::release`。 |
| 已修改源码 | `kernel/src/ipc/mod.rs`。 |
| 编译阻塞处理 | 首次使用 `BTreeMap::retain` 时当前旧 nightly/alloc 无该方法；已改为先收集 undo key 再逐个 `remove` 的兼容写法。 |
| 验证 | `rustfmt --edition 2018 kernel/src/ipc/mod.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 后续边界 | B04.3 shared memory key/size 后续已完成；B05.3 fd lifecycle 后续也已完成。 |

## 2026-06-07：执行 B04.3 shared memory key/size 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | 源码对照确认 `ShmIdentifier::new_shared_guard` 对任意 key 都查 `KEY2SHM`，导致 `key == 0` 可能复用全局 key 0 segment；existing key 直接返回旧 guard，不检查或扩展 size；`sys_shmget` 不拒绝 `size == 0`。 |
| 行级报告 | 目标文件为 `kernel/src/ipc/shared_mem.rs:27` 和 `kernel/src/syscall/ipc.rs:113`；根因是恢复后的 shared memory get/create 路径没有吸收 `kernel.rs::shm_get_or_create` 的 private key 独立创建和 existing key size 扩展语义。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:4328-4351` 的 `shm_get_or_create`，以及 `kernel/src/kernel.rs:4353-4385` 的 `ShmCtx`。 |
| 接口处理 | 保留 `ShmIdentifier`、`ShmProc`、`SharedGuard<GlobalFrameAlloc>` public shape 和 `sys_shmat/sys_shmdt` ABI；`key == 0` 绕开 `KEY2SHM`；existing key 命中时扩展 `SharedGuard.size`。 |
| 已修改源码 | `kernel/src/ipc/shared_mem.rs`、`kernel/src/syscall/ipc.rs`。 |
| 验证 | `rustfmt --edition 2018 kernel/src/ipc/shared_mem.rs kernel/src/syscall/ipc.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止。 |
| 后续边界 | B04 IPC 三个子项均已完成；B05.3 fd lifecycle 后续已完成，新的实际运行边界再单独报告。 |

## 2026-06-07：执行 B05.3 fd lifecycle / cloexec 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | 源码对照确认 `dup_impl` 先删除目标 fd 再读取源 fd，导致 `dup2(old, old)` 误关 fd；`FD_CLOEXEC` 只存在 `FileHandle`，socket/epoll fd 的 `F_GETFD/F_SETFD` 与 exec close loop 无法覆盖；`F_DUPFD` 落到默认 `Ok(0)`。 |
| 行级报告 | 目标文件为 `kernel/src/process/proc.rs:80`、`:190`、`:206`、`:212`、`:221`，`kernel/src/process/thread.rs:332`、`:378`，`kernel/src/syscall/fs.rs:310`、`:669`、`:670`、`:900`、`:905`、`:922`、`:1162`、`:1365`，`kernel/src/syscall/proc.rs:201`；根因是恢复后的 fd table 只有 `BTreeMap<usize, FileLike>`，fd-local metadata 没有落在 `Process` 层。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:5736-5801` 的 `Task::{close_fd,dup_fd,dup2_fd,set_cloexec}`；`kernel/src/kernel.rs:7436-7475` 的 exec close-on-exec loop；`kernel/src/kernel.rs:6888-6932` 的 `SYS_FCNTL` fd flag 分支。 |
| 接口处理 | 保留 `Process.files: BTreeMap<usize, FileLike>` 和 `FileLike` enum；新增 `Process.fd_cloexec: BTreeSet<usize>` 承载 fd-local flag，不把 close-on-exec 塞进 socket/epoll 对象，也不破坏 B03 的 epoll registration 共享和 file open-file description 共享。 |
| 已修改源码 | `kernel/src/process/proc.rs`、`kernel/src/process/thread.rs`、`kernel/src/syscall/fs.rs`、`kernel/src/syscall/proc.rs`。 |
| 已落地结果 | `Process::{add_file_with_cloexec,close_file,is_fd_cloexec,set_fd_cloexec}` 统一管理 fd-local metadata；fork 继承 `fd_cloexec`；open/pipe/epoll create 接入 close-on-exec；close 清理 metadata；dup2 同 fd no-op；dup3 同 fd或非法 flags 返回 `EINVAL`；`F_DUPFD` 和 `F_DUPFD_CLOEXEC` 按 arg 查找空 fd；`F_GETFD/F_SETFD` 对 file/socket/epoll 均有效；exec 关闭所有 fd-local cloexec fd。 |
| 验证 | `rustfmt --edition 2018 kernel/src/process/proc.rs kernel/src/process/thread.rs kernel/src/syscall/fs.rs kernel/src/syscall/proc.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止；未运行 `chaos-tests`。 |
| 后续边界 | B05.3 已完成；B05.1 stack 错误传播和 B05.4 ELF bounds 后续也已完成；B05.2 fork/wait parent-child 后续已完成。 |

## 2026-06-07：执行 B05.1 user stack init 与 B05.4 ELF bounds 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | 源码对照确认 `StackWriter::push_slice` 仍直接 `self.sp -= len * size_of::<T>()`；`VmStackWriter` 对 size/sp/address/page prepare 失败使用 `expect/panic`；`ElfExt::make_memory_set`、`append_as_interpreter` 和 `get_phdr_vaddr` 对 LOAD virtual/file range、interpreter bias、PHDR inferred address、farthest memory 直接相加。 |
| 行级报告 | 目标文件为 `kernel/src/process/abi.rs:13`、`:24`、`:63`、`:81`、`:102`、`:119`，`kernel/src/process/structs.rs:56`、`:82`、`:132`、`:202`、`:220`，`kernel/src/process/thread.rs:116`、`:155`、`:168`、`:184`、`:195`、`:221`；根因是恢复后的 exec loader 缺少 fallible stack writer 和 ELF checked range 错误通道。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:4387-4470` 的 `ProcInit::push_at`；`kernel/src/kernel.rs:1950-2022` 的 `validate_elf_header`。 |
| 接口处理 | 保留 `ProcInitInfo::push_at` 和 `push_at_in_vm` 兼容 wrapper；新增 `try_push_at` / `try_push_at_in_vm` 服务真实 exec path。保留 `ElfExt` 作为 loader trait，但将 `make_memory_set`、`append_as_interpreter`、`get_phdr_vaddr` 改为 `Result`；错误在 `Thread::new_user_vm` 收口，再由 `sys_exec` 映射为 `EINVAL`。 |
| 已修改源码 | `kernel/src/process/abi.rs`、`kernel/src/process/structs.rs`、`kernel/src/process/thread.rs`。 |
| 已落地结果 | stack writer size 乘法、sp 下移、alignment、VA 加法、flush range、page prepare 都走 checked/fallible path；ELF LOAD virtual/file range、`file_size <= mem_size`、interpreter bias、PHDR inferred address、farthest memory 和 bias address 均 checked；无 LOAD 或空 LOAD range 返回错误；`Thread::new_user_vm` 在临时 `MemorySet` 上完成 ELF/stack 装载，成功后替换传入 VM，避免 exec 失败提前破坏旧地址空间。 |
| 验证 | `rustfmt --edition 2018 kernel/src/process/abi.rs kernel/src/process/structs.rs kernel/src/process/thread.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止；未运行 `chaos-tests`。 |
| 后续边界 | B05.1 和 B05.4 已完成；B05.2 fork/wait parent-child 后续已完成。 |

## 2026-06-07：执行 B05.2 fork/wait parent-child 迁移

| 项目 | 内容 |
| --- | --- |
| 触发证据 | 源码对照确认 `sys_wait4(pid > 0)` 先读取全局 `process(pid)`，在证明目标是当前进程 child 前即可观察/回收；`pid == 0` 名义上是 group wait 但实际等同 any child；`pid < -1` 走 `unimplemented!()`；`Process::exit` 不把 living/zombie children 转交 init。 |
| 行级报告 | 目标文件为 `kernel/src/syscall/proc.rs:84`、`:97`、`:111`、`:147` 和 `kernel/src/process/proc.rs:245`、`:297`；根因是恢复后的 wait/reap 逻辑没有完全吸收 `kernel.rs::TaskTable::{fork_task,reap}` 和 `SYS_WAIT4/SYS_EXIT` 的 parent-child 生命周期不变量。 |
| `kernel.rs` 来源 | `kernel/src/kernel.rs:5814-5935` 的 `TaskTable::{fork_task,reap}`；`kernel/src/kernel.rs:6700-6815` 的 `SYS_EXIT/SYS_WAIT4`。 |
| 接口处理 | 保留 `Thread::fork(&UserContext) -> Arc<Thread>`、`Process.children`、`PROCESSES` 和 `sys_wait4(pid, wstatus)` ABI；wait target 只在 syscall 内部表达为 any child / process group / pid；orphan reparent 落在 `Process` 私有 helper。 |
| 已修改源码 | `kernel/src/process/proc.rs`、`kernel/src/syscall/proc.rs`。 |
| 已落地结果 | `sys_wait4` 只从当前 parent children 快照匹配目标；`pid == 0` 等待当前 pgid children；`pid < -1` 等待指定 pgid children，溢出返回 `EINVAL`；无匹配 child 返回 `ECHILD`；reap 同步清理 global process table 和 parent children；parent exit 将 children 转交 init，并在转交 zombie child 时唤醒 init wait。 |
| 验证 | `rustfmt --edition 2018 kernel/src/syscall/proc.rs kernel/src/process/proc.rs` 通过；`make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy` 通过；手动 QEMU 启动进入 busybox shell `/ #`，`timeout` 结束码 124 为预期外部终止；未运行 `chaos-tests`。 |
| 后续边界 | B05 核心 process 批次已完成；更广 process group/session、signal、resource、scheduler 语义后续按真实运行边界单独报告。 |

## 2026-06-07：迁移记录覆盖复核

| 项目 | 内容 |
| --- | --- |
| 触发原因 | 用户目标要求每个新增文件/文件夹都有分层迁移记录，并且每个子模块-功能单独记录。B05.2 完成后需要复核文档覆盖仍一致。 |
| 覆盖命令 | 对 `crate/`、`crate/memory/`、`kernel/src/{fs,ipc,process,sync}` 等新增目录运行 `rg --fixed-strings` 覆盖检查；对当前工作区恢复文件运行 `MISSING_FILE_RECORD` 检查；对恢复 Rust 文件运行 `MISSING_SYMBOL_RECORD` 检查；对 B01-B06 批次和 `final-delivery.md` 运行链接检查。 |
| 结果 | 无 `MISSING_DIR_RECORD`、`MISSING_FILE_RECORD`、`MISSING_SYMBOL_RECORD`、`MISSING_BATCH_DOC`、`MISSING_BATCH_LINK`、`MISSING_FINAL_DELIVERY`、`MISSING_FINAL_DELIVERY_LINK` 输出。 |
| 说明 | `/tmp/rcore-upstream` 当前不存在，因此本次未重新执行 upstream tree hash 比对；`completion-audit.md` 和 `final-delivery.md` 已改为记录当前可复核的工作区覆盖检查。 |
| QEMU 交互尝试 | 尝试通过 PTY、`-serial stdio`、本地 TCP serial + `nc` 注入 fork/wait shell 命令，但当前工具/QEMU stdin 组合没有让命令进入 guest shell；该尝试不作为内核运行失败证据。保留已通过的构建和 QEMU shell 启动证据。 |
