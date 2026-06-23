# `kernel/src/memory.rs` 与 `kernel/src/trap.rs` 迁移记录

## 模块定位

这两个单文件模块把恢复后的 `rcore-memory`、架构 paging、trap/timer/serial 路径接入真实 rCore 内核。它们是 `kernel.rs` 中地址转换、frame pool、user copy、trap controller、clock、serial helper 的主要落点。

当前状态：`kernel/src/memory.rs` 已完成 B02 部分迁移，`kernel/src/trap.rs::uptime_msec` 已迁移；运行上已经越过 init stack 的 `0x3fffffff` fault、RISC-V 用户入口 `0x100e8` 重复 instruction fault 和 `sepc=0x0` 跳转 fault，QEMU 已进入 busybox shell `/ #`。

## 文件-功能记录

### `kernel/src/memory.rs`

| 功能 | 主要 API | `kernel.rs` 对齐语义 | 迁移状态 |
| --- | --- | --- | --- |
| 地址转换 | `phys_to_virt`、`virt_to_phys`、`kernel_offset` | `p2v`、`v2p`、`k_off`。 | `MIGRATION_PENDING` |
| 帧分配 | `GlobalFrameAlloc::{alloc,alloc_contiguous,dealloc}`、`alloc_frame*` | `FramePool::get/get_contig/put`。 | `PARTIAL_MIGRATED` |
| Kernel stack | `KernelStack::new/top/drop` | `KStk` 分配和 top 边界。 | `MIGRATION_PENDING` |
| page fault facade | `handle_page_fault`、`handle_page_fault_ext`、`with_active_memory_set` | `VmMap` fault、access type 分派；init-time VM fault。 | `PARTIAL_MIGRATED` |
| heap | `init_heap`、`enlarge_heap` | `heap_init`、`heap_grow`。 | `MIGRATION_PENDING` |
| user access | `access_ok`、`copy_from_user`、`copy_to_user`、`read_user_fixup` | `check_access`、`validate_access`、`cfu`、`ctu`。 | `PARTIAL_MIGRATED` |

已落地迁移：

- `ACTIVE_MEMORY_SET`、`ActiveMemorySetGuard` 和 `with_active_memory_set` 用于没有 `current_thread()` 的 VM 构造期 fault。
- `handle_page_fault` / `handle_page_fault_ext` 先尝试当前线程 VM，再回落到 active `MemorySet`，最后返回 false。
- `alloc_frame_contiguous(size, align_log2)` 拒绝 `size == 0` 和过大 align，frame id 到地址使用 checked 乘加。
- `dealloc` 对低于 `MEMORY_OFFSET` 的地址不再 underflow。
- `access_ok(addr, len)` 使用 checked add，并按 `[addr, addr + len)` 允许 `end == PHYSICAL_MEMORY_OFFSET`。
- `copy_from_user` / `copy_to_user` 继续通过更新后的 `access_ok` 对非法范围返回失败。

仍需后续核对：

- `enlarge_heap` 的批量 frame 分配应避免越界写入局部数组或 unwrap OOM 路径。
- `0x100e8` instruction page fault 已定位到 RISC-V 非 leaf PTE `A/D` flags 污染，并在 `kernel/src/arch/riscv/paging.rs` 中通过映射路径归一化解除；后续 memory facade 仍需按新故障逐项核对。

### `kernel/src/trap.rs`

| 功能 | 主要 API | `kernel.rs` 对齐语义 | 迁移状态 |
| --- | --- | --- | --- |
| tick 计数 | `wall_tick`、`cpu_tick`、`do_tick` | `wclk`、`cclk`、tick accounting。 | `MIGRATION_PENDING` |
| uptime | `uptime_msec` | `up_ms` checked/saturating conversion。 | `MIGRATED` |
| timer | `NAIVE_TIMER`、`timer` | `TimerWheel` deadline/expiry。 | `MIGRATION_PENDING` |
| serial | `serial` | `ser_push`、`\r` 到 `\n`。 | `MIGRATED` for B03.5 TTY wakeup linkage |

已落地迁移：

- tick 到 millisecond 的转换使用 `saturating_mul`，不因乘法溢出 panic/wrap。

仍需后续核对：

- timer deadline 比较保持 `now >= deadline` 语义。
- serial 输入路径保持 CR 到 LF 的转换；TTY waiter 唤醒已在 B03.5 通过 `TtyINode::async_poll` 和 `EventBus::subscribe_waker` 对齐。

## 待批准迁移候选

| 优先级 | 位置 | 迁移语义 | 最小范围 |
| --- | --- | --- | --- |
| 已完成 | `kernel/src/memory.rs::access_ok` | 用户指针范围 checked add，半开区间边界与 `kernel.rs` 对齐。 | 已迁移，后续只需回归核对。 |
| 已完成 | `kernel/src/trap.rs::uptime_msec` | tick 到 msec 使用 checked/saturating arithmetic。 | 已迁移。 |
| 已完成 | `kernel/src/memory.rs::alloc_frame_contiguous` | alignment/order 非法输入干净失败。 | 已迁移到 `GlobalFrameAlloc::alloc_contiguous` wrapper。 |
| 已完成 | RISC-V 用户入口 instruction page fault | fault 成功后 executable mapping 应对硬件可见。 | 已通过 `sfence.vma` vaddr 修正和 Sv39 非 leaf PTE flags 归一化解除。 |
