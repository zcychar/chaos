# 子模块-功能迁移索引

本文是 L4 级别的迁移记录：按新增文件列出 public API、关键内部结构、trait、impl 功能块，以及它们对应的 `kernel.rs` 语义迁移项。文件级总账见 [restored-tree.md](restored-tree.md)。

状态含义沿用 [README.md](README.md)：`BASELINE_RESTORED`、`MIGRATION_PENDING`、`NO_DIRECT_PORT`、`NEEDS_APPROVAL`、`PARTIAL_MIGRATED`、`RUNTIME_BOUNDARY`、`MIGRATED`。

## `crate/memory`

### `crate/memory/Cargo.toml`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| crate manifest 和依赖形状 | 无直接 `kernel.rs` 语义 | 保持上游 `rcore-memory` crate 依赖面，供 `kernel/Cargo.toml` path dependency 使用。 | `BASELINE_RESTORED` |

### `crate/memory/src/lib.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `addr`、`cow`、`memory_set`、`no_mmu`、`paging` 模块边界 | `kernel.rs` VM/AddrSpace 区域 | 保持上游模块边界；迁移语义进入子模块。 | `BASELINE_RESTORED` |
| `VMError`、`VMResult` | `VmMap`/user copy 错误返回 | 不新增错误通道；后续边界失败统一映射到 `VMError` 或上层 `SysError`。 | `MIGRATION_PENDING` |

### `crate/memory/src/addr.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `PhysAddr`、`VirtAddr` | `p2v`、`v2p`、`k_off` | 地址类型保持别名；边界语义在调用方 checked arithmetic 中对齐。 | `MIGRATION_PENDING` |
| `Page::of_addr`、`Page::start_address`、`Page::range_of` | `VmRegion` page rounding | 需要确认 page range 是半开区间，长度/上界计算不 overflow。 | `MIGRATION_PENDING` |
| `PageRange::next` | `VmMap` range iteration | 保持迭代终止条件；迁移重点是创建 range 前的 checked end。 | `MIGRATION_PENDING` |

### `crate/memory/src/cow.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `CowExt<T>` | `SharedPage`、COW fault | 对齐 fork 后共享 writable page 和写时复制语义；refcount 边界已迁移，完整生命周期仍需后续核对。 | `PARTIAL_MIGRATED` |
| `CowExt::page_fault_handler` | `SharedPage::fault` | source frame refcount 不能 underflow；fault 成功后权限和 frame 计数一致。 | `PARTIAL_MIGRATED` |
| `FrameRcMap` | `PgFrame::up/down` | increase 已 saturating，decrease 已避免缺失/0 count underflow。 | `PARTIAL_MIGRATED` |

### `crate/memory/src/no_mmu.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `NoMMUSupport` | 无直接运行迁移 | no-MMU 支持保留上游；当前 RISC-V MMU 基线不从 `kernel.rs` 迁移到这里。 | `NO_DIRECT_PORT` |
| no-MMU `MemorySet`、`MemoryArea` | `AddrSpace` 抽象概念 | 仅作为架构备选记录，不作为首批迁移落点。 | `NO_DIRECT_PORT` |

### `crate/memory/src/memory_set/mod.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `MemoryArea` | `VmRegion` | 半开区间和 align helper 已加入 checked 边界。 | `PARTIAL_MIGRATED` |
| `MemoryAttr` | `VM_READ/WRITE/EXEC/SHARED` | 权限转换保持 rCore 形状；迁移边界检查。 | `MIGRATION_PENDING` |
| `MemorySet<T>` | `VmMap`、`AddrSpace` | insert/remove/split/fault 的基础区间不变量已部分对齐。 | `PARTIAL_MIGRATED` |
| map/unmap/push/pop 类方法 | `VmMap::insert`、`AddrSpace::unmap_range/split_region` | start+len、page rounding、area overlap 已加入 checked/早返回基础处理。 | `PARTIAL_MIGRATED` |
| `handle_page_fault*` | `VmMap` fault path | access 类型和 handler dispatch 已保留；`0x100e8` 硬件页表可见性边界已解除，后续按新故障继续核对。 | `PARTIAL_MIGRATED` |

### `crate/memory/src/memory_set/handler/mod.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `AccessType` | trap/page fault access | 作为真实 fault 分类入口；需要和 syscall/user access 错误区分。 | `MIGRATION_PENDING` |
| `MemoryHandler` trait | VM region backing | 保持上游 trait；迁移语义进入各 handler 实现。 | `BASELINE_RESTORED` |
| `FrameAllocator` trait | `FramePool` | trait 形状保持；非法输入语义已迁移到 `kernel/src/memory.rs::GlobalFrameAlloc`。 | `PARTIAL_MIGRATED` |
| handler re-export | `VmRegion` backing 类型 | 保持上游 API。 | `BASELINE_RESTORED` |

### `crate/memory/src/memory_set/handler/byframe.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `ByFrame<T>` | anonymous frame backing | 匿名页 eager backing 保持上游，fault path 已补权限核对。 | `MIGRATED` |
| `MemoryHandler for ByFrame` | `FramePool`、`VmMap` fault | 已映射页只有 present 且 access 满足时返回 true。 | `MIGRATED` |

### `crate/memory/src/memory_set/handler/delay.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `Delay<T>` | lazy allocation | 对齐 demand paging 和非法访问不隐式建页。 | `MIGRATION_PENDING` |
| `MemoryHandler for Delay` | `VmMap` fault | fault handler 必须区分 alloc failure 与 permission failure。 | `MIGRATION_PENDING` |

### `crate/memory/src/memory_set/handler/file.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `Read` trait | `INodeForMap`、file mmap | 保持上游 file-backed handler 读取抽象。 | `BASELINE_RESTORED` |
| `File<F,T>` | `FLike::mmap_fl` | file offset、mapping length 和 page offset 计算已部分 checked。 | `PARTIAL_MIGRATED` |
| `MemoryHandler for File` | mmap page fault | file offset + page offset 已 checked，短读/越界 zero-fill；当前 executable fault 仍是运行边界。 | `PARTIAL_MIGRATED` |

### `crate/memory/src/memory_set/handler/linear.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `Linear` | direct map、`p2v/v2p` | 保持物理线性映射；边界检查放在 map 调用方。 | `MIGRATION_PENDING` |
| `MemoryHandler for Linear` | kernel/direct mapping | 不复制模拟 direct-map 结构，只对齐地址计算安全性。 | `MIGRATION_PENDING` |

### `crate/memory/src/memory_set/handler/shared.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `SharedGuard<T>` | `ShmCtx`、shared page guard | 对齐 shared frame 生命周期、detach/drop 行为。 | `MIGRATION_PENDING` |
| `Shared<T>` | `SharedPage`、shared memory mapping | shared mapping 不能重复释放或泄漏 frame。 | `MIGRATION_PENDING` |
| `MemoryHandler for Shared` | shared memory page fault | attach/fault/drop 和 `ipc` 记录联动。 | `MIGRATION_PENDING` |

### `crate/memory/src/paging/mod.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `PageTable` trait | page table facade | 保持上游 trait；迁移在 arch paging/MemorySet 调用。 | `BASELINE_RESTORED` |
| `Entry` trait | page flags/refcount 语义 | 对齐 permission、present、writable 位语义。 | `MIGRATION_PENDING` |
| `PageTableExt` | map/unmap helper | 重点是 map/unmap 参数 range 已经 checked。 | `MIGRATION_PENDING` |

### `crate/memory/src/paging/mock_page_table.rs`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `MockPageTable`、`MockEntry` | `VmMap` 行为模型 | 用作测试/参考，不进入真实内核运行迁移。 | `NO_DIRECT_PORT` |
| `PageFaultHandler` | 模拟 fault callback | 不作为迁移落点。 | `NO_DIRECT_PORT` |

### `crate/memory/src/swap/*`

| 文件 | 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- | --- |
| `swap/mod.rs` | `SwapManager`、`Swapper`、`SwapExt`、`SwapError` | cache/allocator 辅助概念 | 上游恢复但当前真实运行目标不迁移 `kernel.rs` 模拟 cache 到 swap。 | `NO_DIRECT_PORT` |
| `swap/fifo.rs` | `FifoSwapManager` | 无直接对应 | 保持上游。 | `NO_DIRECT_PORT` |
| `swap/enhanced_clock.rs` | `EnhancedClockSwapManager` | 无直接对应 | 保持上游。 | `NO_DIRECT_PORT` |
| `swap/mock_swapper.rs` | `MockSwapper` | 无直接对应 | 保持上游测试辅助。 | `NO_DIRECT_PORT` |

## `kernel/src/memory.rs` 与 `kernel/src/trap.rs`

| 文件 | 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- | --- |
| `kernel/src/memory.rs` | `MemorySet` type alias 和 `rcore_memory` re-export | `VmMap`/`AddrSpace` | 保持 rCore API；新增 active `MemorySet` 作为 init-time fault fallback。 | `PARTIAL_MIGRATED` |
| `kernel/src/memory.rs` | `FrameAlloc`、`FRAME_ALLOCATOR`、`GlobalFrameAlloc` | `FramePool` | illegal align/order 干净失败，dealloc 不 underflow。 | `PARTIAL_MIGRATED` |
| `kernel/src/memory.rs` | `phys_to_virt`、`virt_to_phys`、`kernel_offset` | `p2v/v2p/k_off` | 地址转换保持上游常量；边界由调用方校验。 | `MIGRATION_PENDING` |
| `kernel/src/memory.rs` | `KernelStack` | `KStk` | stack top/bottom 不 overflow。 | `MIGRATION_PENDING` |
| `kernel/src/memory.rs` | `handle_page_fault`、`handle_page_fault_ext`、`with_active_memory_set` | trap fault dispatch | 先用 current thread VM，无 current thread 时使用 active `MemorySet`。 | `PARTIAL_MIGRATED` |
| `kernel/src/memory.rs` | `init_heap`、`enlarge_heap` | `heap_init`、`heap_grow` | OOM/array capacity/unchecked unwrap 需要后续核对。 | `MIGRATION_PENDING` |
| `kernel/src/memory.rs` | `access_ok`、`copy_from_user`、`copy_to_user` | `check_access`、`validate_access`、`cfu/ctu` | checked add，半开上界，非法用户范围返回失败。 | `PARTIAL_MIGRATED` |
| `kernel/src/trap.rs` | `wall_tick`、`cpu_tick`、`do_tick` | clock counters | tick counter 语义保持；转换处防 overflow。 | `MIGRATION_PENDING` |
| `kernel/src/trap.rs` | `uptime_msec` | `up_ms` | `wall_tick * USEC_PER_TICK` 已使用 `saturating_mul`。 | `MIGRATED` |
| `kernel/src/trap.rs` | `NAIVE_TIMER`、`timer` | `TimerWheel` | deadline/expiry 语义对齐。 | `MIGRATION_PENDING` |
| `kernel/src/trap.rs` | `serial` | serial CR/LF、TTY channel | `\r` 到 `\n` 已有；TTY wakeup 联动已在 `fs/devfs/tty.rs` 和 `sync/event_bus.rs` 落地。 | `MIGRATED` for B03.5 linkage |

## `kernel/src/fs`

| 文件 | 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- | --- |
| `fs/mod.rs` | `ROOT_INODE`、devfs/root setup | mount table/root fs | 保持上游 root inode；路径语义进入 `INodeExt`。 | `MIGRATION_PENDING` |
| `fs/mod.rs` | `INodeExt::lookup_follow` | mount/path lookup | follow depth、component boundary、symlink 语义后续核对。 | `MIGRATION_PENDING` |
| `fs/file.rs` | `OpenFileDescription` | shared open-file description | dup 共享 offset/options；fd-local cloexec 不共享，真实 fd flag 由 `Process.fd_cloexec` 记录。 | `MIGRATED` for B05.3 fd sharing |
| `fs/file.rs` | `FileHandle::dup` | `FHandle` dup | offset/options 共享，FileHandle 内部 `fd_cloexec` 只作 file 分支兼容，fd-local 真值由 `Process` helper 同步。 | `MIGRATED` for B05.3 |
| `fs/file.rs` | `FileHandle::read/read_at` | `FHandle::read` | nonblock/async poll 与 EOF/Again 语义对齐。 | `MIGRATION_PENDING` |
| `fs/file.rs` | `FileHandle::write/write_at` | append/write overflow | append 写后 offset 写回实际写入终点；`offset + len` checked；metadata update 保持。 | `MIGRATED` |
| `fs/file.rs` | `FileHandle::seek` | negative seek | `SeekFrom::End/Current` 使用 signed checked 计算，负结果和超出 `u64::MAX` 返回错误。 | `MIGRATED` |
| `fs/file.rs` | `FileHandle::mmap` | `FLike::mmap_fl` | `start/end` 和 file-backed `offset + len` checked；非法 range 返回 `InvalidParam`。 | `MIGRATED` |
| `fs/file_like.rs` | `FileLike::dup` | `FLike::dup` | epoll clone 已通过共享 `EpollInstance` 状态保持 registration/ready/new_ctl；file/socket 对象共享不承载 fd-local cloexec。 | `MIGRATED` for B03.3/B05.3 |
| `fs/file_like.rs` | `read/write/ioctl/mmap/poll` dispatch | syscall file facade | 保持分发；mmap range checked 已落到 syscall/file handler。 | `PARTIAL_MIGRATED` |
| `fs/pipe.rs` | `PipeEnd`、`PipeData`、`Pipe` | `PipeNode` | reader/writer endpoint 生命周期、read/write/poll。 | `MIGRATED` |
| `fs/pipe.rs` | `INode for Pipe` | pipe read/write/EOF | no-reader write 返回错误，writer close 后 reader EOF/ready。 | `MIGRATED` |
| `fs/epoll.rs` | `EpollInstance::new/control` | `EpInst::control` | ADD duplicate 返回 `EEXIST`，MOD missing 返回错误，DEL 同时清理 events/ready/new_ctl。 | `MIGRATED` |
| `fs/epoll.rs` | `Clone for EpollInstance` | epoll dup sharing | clone 保持共享 registration/ready/new_ctl state。 | `MIGRATED` |
| `fs/epoll.rs` | `EpollEvent`、`EpollData` | epoll event ABI | ABI layout 后续按 syscall/user copy 校验。 | `MIGRATION_PENDING` |
| `fs/fcntl.rs` | fcntl/open flags constants | fd flags | 常量保持上游；`F_DUPFD/F_DUPFD_CLOEXEC/F_GETFD/F_SETFD` 行为已在 `syscall/fs.rs` 与 `Process.fd_cloexec` 落地。 | `MIGRATED` for B05.3 syscall usage |
| `fs/ioctl.rs` | `Termios`、`Winsize`、TTY ioctl constants | terminal metadata | 与 TTY ioctl 路径联动。 | `MIGRATION_PENDING` |
| `fs/device.rs` | `MemBuf`、`Device for MemBuf` | device/block buffer | 保持上游设备 buffer，不迁移模拟 block cache 结构。 | `MIGRATION_PENDING` |
| `fs/pseudo.rs` | `Pseudo` inode wrapper | pseudo registry 概念 | 只保留真实 rCore pseudo inode。 | `MIGRATION_PENDING` |
| `fs/devfs/mod.rs` | devfs module exports | device namespace | 保持上游模块边界。 | `BASELINE_RESTORED` |
| `fs/devfs/tty.rs` | `TtyINode::push/read_at/io_control/poll` | `Channel`、terminal | `read_at` 支持空 buffer 和多字节 drain；readable 清理与 buffer 状态一致；async poll 使用 EventBus waker/mask 去重。 | `MIGRATED` |
| `fs/devfs/tty.rs` | `foreground_pgid` | process group/TTY | 与 process group 迁移联动。 | `MIGRATION_PENDING` |
| `fs/devfs/serial.rs` | `Serial::new`、`INode for Serial` | serial helper | 与 `trap.rs::serial` 联动。 | `MIGRATION_PENDING` |
| `fs/devfs/random.rs` | `RandomINode` | 无直接对应 | 保持上游设备。 | `NO_DIRECT_PORT` |
| `fs/devfs/shm.rs` | `ShmINode` | shared memory devfs hook | 与 IPC shm 迁移联动。 | `MIGRATION_PENDING` |
| `fs/devfs/fbdev.rs` | `Fbdev`、framebuffer structs/ioctl | 无直接对应 | 保持上游 framebuffer。 | `NO_DIRECT_PORT` |

## `kernel/src/ipc`

| 文件 | 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- | --- |
| `ipc/mod.rs` | `SemProc` | `SemCtx` | per-process sem array table 和 undo。 | `PARTIAL_MIGRATED` |
| `ipc/mod.rs` | `SemProc::add/remove/get/add_undo` | sem add/remove/undo | remove 清理 stale undo，add/get 保持上游接口。 | `MIGRATED` |
| `ipc/mod.rs` | `Drop for SemProc` | sem undo replay | replay 完整正向 undo magnitude，不只处理 `1`；stale id/num 不 panic。 | `MIGRATED` |
| `ipc/mod.rs` | `ShmProc` | `ShmCtx` | shm id/addr tracking 保持上游；key/size 生命周期在 `ipc/shared_mem.rs` 和 `syscall/ipc.rs` 对齐。 | `PARTIAL_MIGRATED` |
| `ipc/semary.rs` | `IpcPerm`、`SemidDs` | IPC permission metadata | 权限语义后续与 syscall/ipc 对齐。 | `MIGRATION_PENDING` |
| `ipc/semary.rs` | `SemArray` | `SemArr` | zero-length create 拒绝；existing key 请求更大 count 返回 `EINVAL`。 | `MIGRATED` |
| `ipc/shared_mem.rs` | `ShmIdentifier` | `ShmTag` | `IPC_PRIVATE` 每次创建独立 guard；existing key 请求更大 size 时扩展 `SharedGuard.size`。 | `MIGRATED` |

## `kernel/src/process`

| 文件 | 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- | --- |
| `process/mod.rs` | module exports、`init`、`current_thread` | current task facade | 保持上游入口；语义落到 thread/proc。 | `BASELINE_RESTORED` |
| `process/abi.rs` | `ProcInitInfo`、`InitStackWriter`、`VmStackWriter` | `ProcInit` | argv/env/auxv 栈初始化已能通过正在构造的 `MemorySet` 写入，失败通过 `Result` 返回给 exec loader。 | `MIGRATED` |
| `process/abi.rs` | `StackWriter` / `VmStackWriter` | `ProcInit::push_at` | size 乘法、sp 下移、alignment、VA 加法、flush range 和 page prepare 均为 checked/fallible path。 | `MIGRATED` |
| `process/futex.rs` | `Futex::wake` | `FutexTable::ftx_wake` | wake count 精确，`0` 返回 `0`。 | `MIGRATED` |
| `process/futex.rs` | `Futex::wait` / `FutexFuture::poll` | futex timeout cleanup | timeout 后从 queue 清理 waiter。 | `MIGRATED` |
| `process/proc.rs` | `Pid`、`Pgid` | pid/process group | wait 的 pgid filtering 已在 B05.2 落地；session/setpgid 权限等更广语义后续按真实边界处理。 | `PARTIAL_MIGRATED` |
| `process/proc.rs` | `Process` | `Task`、`VmMap::brk` | VM、files、fd-local cloexec、futexes、semaphores、children、signals、shm；program break、orphan reparent 和 wait/reap cleanup 已接入。 | `PARTIAL_MIGRATED` |
| `process/proc.rs` | `USER_BRK_START`、`Process::{brk_start,brk}` | `VmMap::brk`、`SYS_BRK` | 新进程 heap break 初始化和 fork 继承；`sys_brk` 使用 `MemorySet` lazy mapping 增缩堆。 | `MIGRATED` |
| `process/proc.rs` | `process_of/process/process_group/add_to_process_table` | `TaskTable` | wait/reap 对全局表的删除已与 parent children 同步；PID reuse/lookup 更广生命周期仍按后续边界核对。 | `PARTIAL_MIGRATED` |
| `process/proc.rs` | `Process::{add_file,add_file_with_cloexec,close_file,is_fd_cloexec,set_fd_cloexec}` | fd lifecycle | fd-local close-on-exec、close metadata cleanup、exec close loop 和 dup/fcntl side effect。 | `MIGRATED` |
| `process/proc.rs` | `Process::get_futex/exit/exited/reparent_children_to_init` | task lifecycle | exit 关闭 fd 时同步清理 fd-local metadata，并将 living/zombie children 转交 init；wait/reap 不变量已由 B05.2 收口。 | `MIGRATED` for B05.2/B05.3 |
| `process/structs.rs` | `ElfExt` | ELF validation/loading | LOAD virtual/file range、interpreter bias、PHDR inferred address 和 farthest memory checked；malformed ELF 返回错误。 | `MIGRATED` |
| `process/structs.rs` | `INodeForMap` | file-backed mmap | mmap read bounds 和 file handler 联动。 | `MIGRATION_PENDING` |
| `process/thread.rs` | `Tid`、`ThreadContext`、`ThreadInner`、`Thread` | scheduler/task context | executor 体系替代模拟 `RunQueue`；user VM init 栈写入已接入 `push_at_in_vm`。 | `PARTIAL_MIGRATED` |
| `process/thread.rs` | `Thread::new/fork/exec/run` 类 impl | fork/exec/context | exec 初始栈和 ELF loader 已 fallible；fork 已继承 fd-local cloexec，parent-child link 经 B05.2 对齐；VM clone 深层语义后续按实际边界核对。 | `PARTIAL_MIGRATED` |
| `process/thread.rs` | `spawn`、`yield_now`、executor futures | `RunQueue` | 不直接迁移 RunQueue，迁移调度语义到 executor/thread。 | `NO_DIRECT_PORT` |

## `kernel/src/sync`

| 文件 | 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- | --- |
| `sync/mod.rs` | sync exports | sync facade | 保持上游 re-export。 | `BASELINE_RESTORED` |
| `sync/mutex.rs` | `Mutex`、`MutexGuard` | `KernLock`、sleep lock | guard drop、unlock、IRQ state 恢复。 | `MIGRATION_PENDING` |
| `sync/mutex.rs` | `MutexSupport`、`Spin`、`SpinNoIrq`、`FlagsGuard` | trap IRQ state | no-irq region 必须恢复原状态。 | `MIGRATION_PENDING` |
| `sync/mutex.rs` | `SleepLock` via `Condvar` | sleeping while synchronized | 与 condvar wait path 联动。 | `MIGRATION_PENDING` |
| `sync/condvar.rs` | `Condvar::wait_events` | `SyncQueue::wait_events` | stale waiter cleanup，无 lost wakeup。 | `MIGRATED` |
| `sync/condvar.rs` | `Condvar::wait_timeout` | `SyncQueue::wait_timeout` | timeout 后移除 waiter，返回值区分 wake/timeout。 | `MIGRATED` |
| `sync/condvar.rs` | `notify_one/notify_all/notify_n` | precise wake count | `notify_n(0)` 返回 0，唤醒数量精确。 | `MIGRATION_PENDING` |
| `sync/condvar.rs` | epoll registration helpers | epoll ready/control | 与 `fs/epoll.rs` 联动。 | `MIGRATION_PENDING` |
| `sync/event_bus.rs` | `EventBus::subscribe/set/clear/subscribe_waker` | `EvBus` | 为 B03.5 增加 waker/mask 去重订阅；旧 callback 入口保持兼容。 | `PARTIAL_MIGRATED` |
| `sync/event_bus.rs` | `wait_for_event` future | async event wait | 使用 `subscribe_waker`，同一 waker/mask pending poll 不重复注册。 | `MIGRATED` |
| `sync/semaphore.rs` | `Semaphore::{acquire,try_acquire,release}` | `Sema` | acquire/release/wakeup/timeout 语义。 | `MIGRATION_PENDING` |
| `sync/semaphore.rs` | `SemaphoreGuard` | semaphore guard | drop release 只发生一次。 | `MIGRATION_PENDING` |

## `rust-toolchain`

| 功能项 | 迁移来源 | 迁移记录 | 状态 |
| --- | --- | --- | --- |
| `nightly-2020-06-04` 工具链固定 | 无直接 `kernel.rs` 语义 | 保持上游可运行基线；工具链升级需单独批准。 | `BASELINE_RESTORED` |
