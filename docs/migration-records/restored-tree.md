# 新增文件树迁移记录

本文记录本轮新增文件和文件夹的分层迁移状态。它是每个新加入路径的总账；具体子模块-功能迁移细节见 `modules/` 下的文档。

## L0：恢复批次

| 项目 | 内容 |
| --- | --- |
| 上游 commit | `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` |
| 恢复方式 | `git archive` 固定 commit 后解包到 `/home/zcychar/chaos` |
| 新增上游文件数 | 50 |
| hash 校验 | 本地文件与上游 tree 对象逐文件比对，无 mismatch |
| 基线定位 | 上游 rCore 可运行模块面，后续迁移只在此基础上对齐 `kernel.rs` 语义；B01 和 B02 部分文件已产生有意差异 |

## L1：顶层新增路径

| 路径 | 层级 | 文件数 | 对应模块记录 | 迁移状态 |
| --- | --- | ---: | --- | --- |
| `crate/` | L1 容器目录 | 18 | [modules/crate-memory.md](modules/crate-memory.md) | `PARTIAL_MIGRATED` |
| `crate/memory/` | L1 crate | 18 | [modules/crate-memory.md](modules/crate-memory.md) | `PARTIAL_MIGRATED` |
| `kernel/src/memory.rs` | L1 单文件模块 | 1 | [modules/kernel-memory-trap.md](modules/kernel-memory-trap.md) | `PARTIAL_MIGRATED` |
| `kernel/src/trap.rs` | L1 单文件模块 | 1 | [modules/kernel-memory-trap.md](modules/kernel-memory-trap.md) | `MIGRATED` for uptime |
| `kernel/src/fs/` | L1 内核模块 | 15 | [modules/kernel-fs.md](modules/kernel-fs.md) | `BASELINE_RESTORED` |
| `kernel/src/ipc/` | L1 内核模块 | 3 | [modules/kernel-ipc.md](modules/kernel-ipc.md) | `BASELINE_RESTORED` |
| `kernel/src/process/` | L1 内核模块 | 6 | [modules/kernel-process.md](modules/kernel-process.md) | `PARTIAL_MIGRATED` |
| `kernel/src/sync/` | L1 内核模块 | 5 | [modules/kernel-sync.md](modules/kernel-sync.md) | `BASELINE_RESTORED` |
| `rust-toolchain` | L1 工具链文件 | 1 | [modules/rust-toolchain.md](modules/rust-toolchain.md) | `BASELINE_RESTORED` |

## L2：新增目录记录

| 路径 | 父级 | 职责 | 对应 `kernel.rs` 行段 | 迁移状态 |
| --- | --- | --- | --- | --- |
| `crate/memory/src/` | `crate/memory/` | 通用地址、页表、MemorySet、COW、no-MMU/swap 辅助。 | `916-1689`、`7662-8071`、`8072-8338` | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/` | `crate/memory/src/` | VM area、attr、map、page fault 和 map/unmap 管理。 | `916-1689`、`7662-8071` | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/handler/` | `crate/memory/src/memory_set/` | VM area backing handlers：by-frame、delay、file、linear、shared。 | `916-1689` | `PARTIAL_MIGRATED` |
| `crate/memory/src/paging/` | `crate/memory/src/` | 页表 trait、entry trait、mock page table。 | `916-1689` | `MIGRATION_PENDING` |
| `crate/memory/src/swap/` | `crate/memory/src/` | swap manager/swapper 扩展。 | `1692-2205`、`8072-8338` | `NO_DIRECT_PORT` |
| `kernel/src/fs/devfs/` | `kernel/src/fs/` | TTY、serial、random、shm、fbdev 设备节点。 | `2208-3214`、`4777-5191` | `PARTIAL_MIGRATED` |

## L3：新增文件记录

| 文件 | 所属记录 | 主要职责 | 对应 `kernel.rs` 语义 | 当前状态 |
| --- | --- | --- | --- | --- |
| `crate/memory/Cargo.toml` | crate-memory | `rcore-memory` crate manifest。 | 无行为迁移；保持上游依赖形状。 | `BASELINE_RESTORED` |
| `crate/memory/src/lib.rs` | crate-memory | 导出 `addr`、`cow`、`memory_set`、`no_mmu`、`paging`、`VMError`。 | VM 错误通道和模块边界。 | `BASELINE_RESTORED` |
| `crate/memory/src/addr.rs` | crate-memory | `PhysAddr`、`VirtAddr`、`Page`、`PageRange`。 | 地址/page 范围半开语义、checked arithmetic。 | `MIGRATION_PENDING` |
| `crate/memory/src/cow.rs` | crate-memory | `CowExt`、frame refcount map、COW page fault。 | `SharedPage`/COW refcount 不 underflow。 | `PARTIAL_MIGRATED` |
| `crate/memory/src/no_mmu.rs` | crate-memory | no-MMU `MemorySet` 辅助。 | 模拟 `AddrSpace` 的非页表变体，当前无直接迁移。 | `NO_DIRECT_PORT` |
| `crate/memory/src/memory_set/mod.rs` | crate-memory | `MemoryArea`、`MemoryAttr`、`MemorySet`。 | `VmRegion`、`VmMap`、`AddrSpace`。 | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/handler/mod.rs` | crate-memory | `AccessType`、`MemoryHandler`、`FrameAllocator`。 | VM fault/access 分类、frame allocator 接口。 | `MIGRATION_PENDING` |
| `crate/memory/src/memory_set/handler/byframe.rs` | crate-memory | 每页独立 frame backing。 | `FramePool`、匿名映射。 | `MIGRATED` |
| `crate/memory/src/memory_set/handler/delay.rs` | crate-memory | lazy allocation backing。 | demand paging、用户访问触发分配。 | `MIGRATION_PENDING` |
| `crate/memory/src/memory_set/handler/file.rs` | crate-memory | file-backed mapping。 | `FLike::mmap_fl`、file mmap overflow。 | `PARTIAL_MIGRATED` |
| `crate/memory/src/memory_set/handler/linear.rs` | crate-memory | physical-linear mapping。 | `p2v/v2p`、direct map。 | `MIGRATION_PENDING` |
| `crate/memory/src/memory_set/handler/shared.rs` | crate-memory | shared memory handler 和 `SharedGuard`。 | `ShmCtx`、shared page 生命周期。 | `MIGRATION_PENDING` |
| `crate/memory/src/paging/mod.rs` | crate-memory | 页表和 entry trait。 | VM permission 和 page table contract。 | `MIGRATION_PENDING` |
| `crate/memory/src/paging/mock_page_table.rs` | crate-memory | 测试/mock 页表实现。 | `VmMap` 行为验证参考。 | `NO_DIRECT_PORT` |
| `crate/memory/src/swap/mod.rs` | crate-memory | swap 扩展框架。 | 模拟 cache/allocator 辅助，当前无直接运行迁移。 | `NO_DIRECT_PORT` |
| `crate/memory/src/swap/fifo.rs` | crate-memory | FIFO swap manager。 | 无直接迁移。 | `NO_DIRECT_PORT` |
| `crate/memory/src/swap/enhanced_clock.rs` | crate-memory | enhanced clock swap manager。 | 无直接迁移。 | `NO_DIRECT_PORT` |
| `crate/memory/src/swap/mock_swapper.rs` | crate-memory | mock swapper。 | 无直接迁移。 | `NO_DIRECT_PORT` |
| `kernel/src/memory.rs` | kernel-memory-trap | 内核 frame allocator、heap、user copy、page fault facade。 | `p2v/v2p/k_off`、`FramePool`、`KStk`、`check_access/cfu/ctu`。 | `PARTIAL_MIGRATED` |
| `kernel/src/trap.rs` | kernel-memory-trap | tick、timer、serial 输入。 | `TrapCtl`、clock、serial CR/LF。 | `MIGRATED` for uptime |
| `kernel/src/fs/mod.rs` | kernel-fs | VFS root、mount、re-export、`INodeExt`。 | path/mount/lookup 行为。 | `MIGRATION_PENDING` |
| `kernel/src/fs/file.rs` | kernel-fs | `FileHandle`、offset/options、read/write/seek/mmap。 | `FHandle`、append、negative seek、mmap overflow。 | `PARTIAL_MIGRATED` |
| `kernel/src/fs/file_like.rs` | kernel-fs | `FileLike` enum、dup/read/write/ioctl/mmap/poll。 | `FLike` 分发、dup 共享状态。 | `PARTIAL_MIGRATED` |
| `kernel/src/fs/pipe.rs` | kernel-fs | pipe inode、endpoint、poll/read/write。 | `PipeNode` 生命周期和 no-reader write。 | `MIGRATED` for B03.4 pipe endpoint |
| `kernel/src/fs/epoll.rs` | kernel-fs | `EpollInstance`、events、ready/control list。 | `EpInst` ADD/DEL/dup semantics。 | `MIGRATED` for B03.3 epoll state |
| `kernel/src/fs/fcntl.rs` | kernel-fs | fcntl/open flags constants。 | fd flags、cloexec、append/nonblock。 | `MIGRATED` for B05.3 fcntl fd flags usage |
| `kernel/src/fs/ioctl.rs` | kernel-fs | termios、winsize、ioctl constants。 | terminal metadata。 | `MIGRATION_PENDING` |
| `kernel/src/fs/device.rs` | kernel-fs | memory-backed device buffer。 | block/device buffer 概念映射。 | `MIGRATION_PENDING` |
| `kernel/src/fs/pseudo.rs` | kernel-fs | pseudo inode wrapper。 | kernel object registry/pseudo device 概念。 | `MIGRATION_PENDING` |
| `kernel/src/fs/devfs/mod.rs` | kernel-fs | devfs 子模块 re-export。 | terminal/channel 入口。 | `MIGRATION_PENDING` |
| `kernel/src/fs/devfs/tty.rs` | kernel-fs | TTY input、foreground pgid、termios/ioctl。 | `Channel`、terminal、serial wakeup。 | `MIGRATED` for B03.5 TTY/channel |
| `kernel/src/fs/devfs/serial.rs` | kernel-fs | serial device inode。 | serial helper。 | `MIGRATION_PENDING` |
| `kernel/src/fs/devfs/random.rs` | kernel-fs | random inode。 | 无直接迁移，保持设备语义。 | `NO_DIRECT_PORT` |
| `kernel/src/fs/devfs/shm.rs` | kernel-fs | shm devfs inode。 | shared memory device hook。 | `MIGRATION_PENDING` |
| `kernel/src/fs/devfs/fbdev.rs` | kernel-fs | framebuffer device inode/ioctl。 | 无直接 `kernel.rs` 对应，保持上游。 | `NO_DIRECT_PORT` |
| `kernel/src/ipc/mod.rs` | kernel-ipc | `SemProc`、`ShmProc`、undo/drop、per-process state。 | `SemCtx`、`ShmCtx`。 | `MIGRATED` for B04.2 semaphore undo/remove |
| `kernel/src/ipc/semary.rs` | kernel-ipc | `SemArray`、`IpcPerm`、global semaphore arrays。 | `SemArr` create/existing-key/zero-size。 | `MIGRATED` for B04.1 semaphore create/get |
| `kernel/src/ipc/shared_mem.rs` | kernel-ipc | `ShmIdentifier`。 | `ShmTag`、attached shm metadata。 | `MIGRATED` for B04.3 shared memory key/size |
| `kernel/src/process/mod.rs` | kernel-process | process module exports、init、current thread. | `TaskTable` facade。 | `BASELINE_RESTORED` |
| `kernel/src/process/abi.rs` | kernel-process | `ProcInitInfo`、stack writer。 | `ProcInit::push_at`、user stack layout。 | `MIGRATED` for B05.1 user stack init |
| `kernel/src/process/futex.rs` | kernel-process | async futex wait/wake。 | `FutexTable` wake count and timeout cleanup。 | `MIGRATED` |
| `kernel/src/process/proc.rs` | kernel-process | `Process`、pid/pgid、fd table、fd-local flags、children、signals、ipc state。 | `Task`、`TaskTable`、process group/wait/fd lifecycle。 | `PARTIAL_MIGRATED` for B05.2/B05.3/B05.5 |
| `kernel/src/process/structs.rs` | kernel-process | ELF loader helpers、`INodeForMap`。 | ELF validation、file-backed mmap。 | `MIGRATED` for B05.4 ELF bounds |
| `kernel/src/process/thread.rs` | kernel-process | `Thread`、executor spawn、yield、context。 | scheduler/runqueue/context/thread lifecycle；user VM init。 | `PARTIAL_MIGRATED` for B05.1/B05.2/B05.3/B05.4/B05.5 |
| `kernel/src/sync/mod.rs` | kernel-sync | sync re-export。 | sync module boundary。 | `BASELINE_RESTORED` |
| `kernel/src/sync/mutex.rs` | kernel-sync | spin/no-irq/sleep mutex support。 | `KernLock`/sleeping while synchronized 语义。 | `MIGRATION_PENDING` |
| `kernel/src/sync/condvar.rs` | kernel-sync | wait queues、epoll registration、notify。 | `SyncQueue` wait/wake/timeout/stale cleanup。 | `MIGRATED` |
| `kernel/src/sync/event_bus.rs` | kernel-sync | process event bus、future wait。 | `EvBus` stale callback/event delivery。 | `PARTIAL_MIGRATED` for B03.5 waker dedupe |
| `kernel/src/sync/semaphore.rs` | kernel-sync | counting semaphore、guards、async acquire。 | `Sema` guard release/wake count。 | `MIGRATION_PENDING` |
| `rust-toolchain` | rust-toolchain | 固定 `nightly-2020-06-04`。 | 无 `kernel.rs` 迁移；保证上游构建环境。 | `BASELINE_RESTORED` |
