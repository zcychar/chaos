# kernel-rewrite 拆分方案

本文只记录 `kernel-rewrite/src` 当前的结构拆分。`kernel-rewrite/kernel.rs` 保留为原始单文件基准，不参与修改。

## 保持单文件

- `syscall/mod.rs`
- `kernel.rs`
- `trap.rs`
- `net.rs`
- `util.rs`

## 已拆分模块

### memory

- `vm.rs`: `VmRegion`, `VmMap`, `AddrSpace`, user-access helpers, RSS watermark
- `frame.rs`: `ZoneInfo`, `PgFrame`, `FramePool`, frame alloc/free helpers, frame bitmap/page-alignment helpers
- `mod.rs`: re-export，并暂时保留 `SlabEntry`, `SharedPage`, `KStk`, `BuddyAllocator`, address/heap/ELF/load-balance helpers

### sync

- `lock.rs`: `KernLock`, `Spin`, `FlagGuard`, `GKL`
- `event_bus.rs`: `EventFlag`, `EventBus`, `wait_event`
- `condvar.rs`: `RegEp`, `SyncQueue`
- `semaphore.rs`: `Sema`, `SemaGuard`
- `mod.rs`: re-export，并兼容 re-export `process::futex::{FutexBucket, FutexTable}`

### process

- `abi.rs`: `ProcInit`
- `futex.rs`: `FutexBucket`, `FutexTable`
- `task.rs`: pid/tid、`Task`, `TaskTable`, `ProcessGroup`, `yield_now_sync`
- `structs.rs`: capability、scheduler/run queue、wait queue、resource limits

### fs

- `file.rs`: `FdOpt`, `FHandle`, `FSeek`
- `file_like.rs`: `FLike`
- `pipe.rs`: pipe endpoint and buffer
- `epoll.rs`: epoll event/map/instance
- `pseudo.rs`: `PseudoNode`
- `termios.rs`: `TrmIO`, `WinSz`
- `channel.rs`: `CircBuf`, terminal-style channel
- `cache.rs`: page cache, object registry, block cache
- `mount.rs`: mount table
- `disk.rs`: I/O queue and disk simulation

### ipc

- `semary.rs`: SysV semaphore data structures and `SemCtx`
- `shared_mem.rs`: shared-memory tags, lookup helper, `ShmCtx`

### signal

- `action.rs`: `SigAction`, `SigSet`
- `timer.rs`: `TimerEntry`, `TimerWheel`

## 约束

- 只移动完整 Rust item，不改行为逻辑。
- crate-root 公开路径通过各 `mod.rs` 和 `src/lib.rs` re-export 保持兼容。
- 拆分后用 `cargo test`、`cargo fmt --check` 和 item inventory diff 验证。
