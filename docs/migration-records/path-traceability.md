# 新增路径可追踪矩阵

本文按“新增路径”为主键，补齐每一个恢复目录和文件的记录位置、迁移批次和当前状态。它用于回答：任意一个从 rCore 上游恢复进来的路径，在哪一级记录中出现，后续从 `kernel.rs` 迁移语义时应落到哪个模块和批次。`crate/` 是本轮在工作区出现的容器目录，真实上游源码恢复范围从 `crate/memory/` 开始。

## 使用方式

- 先看本文定位路径的记录层级、模块文档和批次。
- 再进入 [restored-tree.md](restored-tree.md) 查看 L0-L3 的恢复记录。
- 再进入 [function-index.md](function-index.md) 或 `modules/*.md` 查看 L4 子模块-功能记录。
- 若“关联批次”为 B03-B05 或 B01/B02 未覆盖的后续项，源码修改前仍需按 [migration-batches.md](migration-batches.md) 报告并等待批准。B02 当前已部分迁移，后续只按具体运行边界重新报告。
- 若“关联批次”为 B06 或 `无`，表示该路径只保留上游基线，或只作为迁移参考，不强行从 `kernel.rs` 复制同名结构。

## 目录矩阵

| 路径 | 类型 | 记录层级 | 记录位置 | 关联批次 | 当前状态 |
| --- | --- | --- | --- | --- | --- |
| `crate/` | 工作区容器目录 | L1 容器 | 本文、[restored-tree.md](restored-tree.md) | 无，承载 `crate/memory/` | `BASELINE_RESTORED` |
| `crate/memory/` | 顶层 crate 目录 | L1 | [restored-tree.md](restored-tree.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `PARTIAL_MIGRATED` |
| `crate/memory/src/` | crate 源码目录 | L2 | [restored-tree.md](restored-tree.md)、[modules/crate-memory.md](modules/crate-memory.md)、[function-index.md](function-index.md) | B02 | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/` | VM 子目录 | L2/L4 | [restored-tree.md](restored-tree.md)、[modules/crate-memory.md](modules/crate-memory.md)、[function-index.md](function-index.md) | B02 | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/handler/` | VM handler 子目录 | L2/L4 | [restored-tree.md](restored-tree.md)、[modules/crate-memory.md](modules/crate-memory.md)、[function-index.md](function-index.md) | B02 | `PARTIAL_MIGRATED` |
| `crate/memory/src/paging/` | 页表抽象子目录 | L2/L4 | [restored-tree.md](restored-tree.md)、[modules/crate-memory.md](modules/crate-memory.md)、[function-index.md](function-index.md) | B02 | `MIGRATION_PENDING` |
| `crate/memory/src/swap/` | swap 辅助子目录 | L2/L4 | [restored-tree.md](restored-tree.md)、[modules/crate-memory.md](modules/crate-memory.md)、[function-index.md](function-index.md) | B06 | `NO_DIRECT_PORT` |
| `kernel/src/fs/` | 内核文件系统目录 | L1/L4 | [restored-tree.md](restored-tree.md)、[modules/kernel-fs.md](modules/kernel-fs.md)、[function-index.md](function-index.md) | B03 | `PARTIAL_MIGRATED` |
| `kernel/src/fs/devfs/` | devfs 子目录 | L2/L4 | [restored-tree.md](restored-tree.md)、[modules/kernel-fs.md](modules/kernel-fs.md)、[function-index.md](function-index.md) | B03 | `PARTIAL_MIGRATED` |
| `kernel/src/ipc/` | IPC 目录 | L1/L4 | [restored-tree.md](restored-tree.md)、[modules/kernel-ipc.md](modules/kernel-ipc.md)、[function-index.md](function-index.md) | B04 | `MIGRATION_PENDING` |
| `kernel/src/process/` | 进程目录 | L1/L4 | [restored-tree.md](restored-tree.md)、[modules/kernel-process.md](modules/kernel-process.md)、[function-index.md](function-index.md) | B05，`futex.rs` 属于 B01，`abi.rs/thread.rs` 有 B02 交叉落点 | `PARTIAL_MIGRATED` |
| `kernel/src/sync/` | 同步目录 | L1/L4 | [restored-tree.md](restored-tree.md)、[modules/kernel-sync.md](modules/kernel-sync.md)、[function-index.md](function-index.md) | B01，`event_bus.rs` 也服务 B03/B05 等等待路径 | `MIGRATION_PENDING` |

## 文件矩阵

| 路径 | 类型 | 记录层级 | 记录位置 | 关联批次 | 当前状态 |
| --- | --- | --- | --- | --- | --- |
| `crate/memory/Cargo.toml` | manifest | L3 | [restored-tree.md](restored-tree.md)、[modules/crate-memory.md](modules/crate-memory.md) | 无 | `BASELINE_RESTORED` |
| `crate/memory/src/lib.rs` | crate 根模块 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `BASELINE_RESTORED` |
| `crate/memory/src/addr.rs` | 地址类型 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `MIGRATION_PENDING` |
| `crate/memory/src/cow.rs` | COW/refcount | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `PARTIAL_MIGRATED` |
| `crate/memory/src/no_mmu.rs` | no-MMU 辅助 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B06 | `NO_DIRECT_PORT` |
| `crate/memory/src/memory_set/mod.rs` | MemorySet 核心 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/handler/mod.rs` | handler trait | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `MIGRATION_PENDING` |
| `crate/memory/src/memory_set/handler/byframe.rs` | 匿名 frame backing | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `MIGRATED` |
| `crate/memory/src/memory_set/handler/delay.rs` | lazy allocation backing | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `MIGRATION_PENDING` |
| `crate/memory/src/memory_set/handler/file.rs` | file-backed mapping | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02/B03 | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/handler/linear.rs` | direct mapping backing | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `MIGRATION_PENDING` |
| `crate/memory/src/memory_set/handler/shared.rs` | shared mapping backing | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02/B04 | `MIGRATION_PENDING` |
| `crate/memory/src/paging/mod.rs` | 页表 trait | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B02 | `MIGRATION_PENDING` |
| `crate/memory/src/paging/mock_page_table.rs` | mock 页表 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B06 | `NO_DIRECT_PORT` |
| `crate/memory/src/swap/mod.rs` | swap 框架 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B06 | `NO_DIRECT_PORT` |
| `crate/memory/src/swap/fifo.rs` | FIFO swap manager | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B06 | `NO_DIRECT_PORT` |
| `crate/memory/src/swap/enhanced_clock.rs` | enhanced clock swap manager | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B06 | `NO_DIRECT_PORT` |
| `crate/memory/src/swap/mock_swapper.rs` | mock swapper | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/crate-memory.md](modules/crate-memory.md) | B06 | `NO_DIRECT_PORT` |
| `kernel/src/memory.rs` | 内核 memory facade | L1/L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-memory-trap.md](modules/kernel-memory-trap.md) | B02 | `PARTIAL_MIGRATED` |
| `kernel/src/trap.rs` | trap/timer/serial facade | L1/L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-memory-trap.md](modules/kernel-memory-trap.md) | B02，trap/signal 后续联动 | `MIGRATED` for uptime，其他 trap 联动待后续 |
| `kernel/src/fs/mod.rs` | fs 根模块 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03 | `MIGRATION_PENDING` |
| `kernel/src/fs/file.rs` | file handle | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03 | `PARTIAL_MIGRATED` |
| `kernel/src/fs/file_like.rs` | FileLike enum | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03 | `PARTIAL_MIGRATED` |
| `kernel/src/fs/pipe.rs` | pipe endpoint | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03/B01 | `MIGRATED` for B03.4 pipe endpoint |
| `kernel/src/fs/epoll.rs` | epoll instance | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03/B01 | `MIGRATED` for B03.3 epoll state |
| `kernel/src/fs/fcntl.rs` | flags/constants | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03 | `MIGRATION_PENDING` |
| `kernel/src/fs/ioctl.rs` | ioctl/termios 常量 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03 | `MIGRATION_PENDING` |
| `kernel/src/fs/device.rs` | device buffer | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03/B06 | `MIGRATION_PENDING` |
| `kernel/src/fs/pseudo.rs` | pseudo inode | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03/B06 | `MIGRATION_PENDING` |
| `kernel/src/fs/devfs/mod.rs` | devfs 根模块 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03 | `MIGRATION_PENDING` |
| `kernel/src/fs/devfs/tty.rs` | TTY 设备 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03/B01 | `MIGRATED` for B03.5 TTY/channel |
| `kernel/src/fs/devfs/serial.rs` | serial devfs 节点 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B03 | `MIGRATION_PENDING` |
| `kernel/src/fs/devfs/random.rs` | random devfs 节点 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B06 | `NO_DIRECT_PORT` |
| `kernel/src/fs/devfs/shm.rs` | shm devfs 节点 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B04/B03 | `MIGRATION_PENDING` |
| `kernel/src/fs/devfs/fbdev.rs` | framebuffer devfs 节点 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-fs.md](modules/kernel-fs.md) | B06 | `NO_DIRECT_PORT` |
| `kernel/src/ipc/mod.rs` | IPC per-process state | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-ipc.md](modules/kernel-ipc.md) | B04 | `MIGRATED` for B04.2 semaphore undo/remove |
| `kernel/src/ipc/semary.rs` | SysV semaphore array | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-ipc.md](modules/kernel-ipc.md) | B04 | `MIGRATED` for B04.1 semaphore create/get |
| `kernel/src/ipc/shared_mem.rs` | shared memory id | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-ipc.md](modules/kernel-ipc.md) | B04/B02 | `MIGRATED` for B04.3 shared memory key/size |
| `kernel/src/process/mod.rs` | process 根模块 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-process.md](modules/kernel-process.md) | B05 module boundary | `BASELINE_RESTORED` |
| `kernel/src/process/abi.rs` | process init ABI | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-process.md](modules/kernel-process.md) | B05，B02 init stack VM 写入交叉落点 | `PARTIAL_MIGRATED` |
| `kernel/src/process/futex.rs` | futex wait/wake | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-process.md](modules/kernel-process.md) | B01/B05 | `MIGRATED` for B01 |
| `kernel/src/process/proc.rs` | process state | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-process.md](modules/kernel-process.md) | B05/B04/B03 | `PARTIAL_MIGRATED` for B05.2/B05.3/B05.5 |
| `kernel/src/process/structs.rs` | ELF/mmap helpers | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-process.md](modules/kernel-process.md) | B05/B02/B03 | `MIGRATED` for B05.4 ELF bounds |
| `kernel/src/process/thread.rs` | thread/executor state | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-process.md](modules/kernel-process.md) | B05/B01，B02 init stack VM 写入交叉落点 | `PARTIAL_MIGRATED` for B05.1/B05.2/B05.3/B05.4/B05.5 |
| `kernel/src/sync/mod.rs` | sync 根模块 | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-sync.md](modules/kernel-sync.md) | B01 | `BASELINE_RESTORED` |
| `kernel/src/sync/mutex.rs` | spin/noirq/sleep mutex | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-sync.md](modules/kernel-sync.md) | B01 | `MIGRATION_PENDING` |
| `kernel/src/sync/condvar.rs` | wait queue/condvar | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-sync.md](modules/kernel-sync.md) | B01 | `MIGRATED` |
| `kernel/src/sync/event_bus.rs` | event bus future | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-sync.md](modules/kernel-sync.md) | B01/B03/B05 | `PARTIAL_MIGRATED` for B03.5 waker dedupe |
| `kernel/src/sync/semaphore.rs` | counting semaphore | L3/L4 | [restored-tree.md](restored-tree.md)、[function-index.md](function-index.md)、[modules/kernel-sync.md](modules/kernel-sync.md) | B01/B04 | `MIGRATION_PENDING` |
| `rust-toolchain` | 工具链 pin | L1/L3 | [restored-tree.md](restored-tree.md)、[modules/rust-toolchain.md](modules/rust-toolchain.md) | 无 | `BASELINE_RESTORED` |

## 与 `kernel.rs` 对齐关系

本文不表示把 `kernel.rs` 的同名结构直接复制到 rCore。对齐关系是：

| `kernel.rs` 语义区 | 主要恢复路径 | 迁移批次 |
| --- | --- | --- |
| `SyncQueue`、`EvBus`、`Sema`、futex | `kernel/src/sync/*`、`kernel/src/process/futex.rs` | B01 |
| 地址、VM、COW、用户拷贝、trap 时间 | `crate/memory/*`、`kernel/src/memory.rs`、`kernel/src/trap.rs` | B02 |
| file、pipe、epoll、TTY/channel、fd flags | `kernel/src/fs/*`、`kernel/src/fs/devfs/*` | B03 |
| SysV semaphore、shared memory | `kernel/src/ipc/*`、`kernel/src/fs/devfs/shm.rs`、`crate/memory/src/memory_set/handler/shared.rs` | B04 |
| process/thread/fork/wait/fd lifecycle/ELF | `kernel/src/process/*` | B05；stack、process brk、fd lifecycle/cloexec、ELF bounds、fork/wait parent-child 核心子项已迁移 |
| swap/mock/no direct 运行面 | `crate/memory/src/swap/*`、`crate/memory/src/paging/mock_page_table.rs`、`kernel/src/fs/devfs/random.rs`、`kernel/src/fs/devfs/fbdev.rs` | B06 |

当前结论：12 个工作区新增目录，包括 `crate/` 容器目录，以及 50 个新增文件均有 L1-L4 中至少一个记录落点；B02 已部分完成源码对齐，B05 核心 process 子项已完成，后续源码迁移只应从本文的关联批次或当前运行边界进入，不再做“找接口阻塞”作为独立阶段。
