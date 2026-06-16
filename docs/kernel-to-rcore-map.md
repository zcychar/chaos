# kernel.rs 到 rCore 恢复模块映射

本文回答两个问题：

- `kernel/src/kernel.rs` 的每个主要部分，对应恢复后 rCore 原项目里的哪一部分。
- 恢复后的模块如何和 `kernel.rs` 产生关联，以及接口差异如何处理。

结论先写清楚：不是用原版 rCore 代码“替代” `kernel.rs`，也不是把 `kernel.rs` 直接拆进原版 rCore 模块。正确关系是：

```text
chaos-tests  ──直接编译──>  kernel/src/kernel.rs

rCore 内核   ──通过 lib.rs──>  kernel/src/fs
                         ├──>  kernel/src/ipc
                         ├──>  kernel/src/memory.rs + crate/memory
                         ├──>  kernel/src/process
                         ├──>  kernel/src/sync
                         └──>  kernel/src/trap.rs

kernel.rs    ──作为语义参考──>  恢复后的 rCore 模块
```

也就是说，恢复源码后，真实 rCore 构建通过 `kernel/src/lib.rs` 中已有的 `pub mod fs;`、`pub mod process;` 等声明接入原项目模块。`kernel.rs` 本身不进入真实 rCore 内核构建，它的作用是给每个恢复模块提供语义核对点。

恢复后的每个新增目录/文件及其子模块-功能迁移记录见 [docs/migration-records/README.md](/home/zcychar/chaos/docs/migration-records/README.md)。

## 接口处理原则

第一轮只恢复匹配上游 commit `66cb4181ec6d3336d507c7c1ff100127f56fcc0a` 的原项目模块接口，不发明替代 API。

恢复后的接口必须优先满足当前 rCore 已有调用方：

- `kernel/src/syscall/*`
- `kernel/src/arch/*`
- `kernel/src/drivers/*`
- `kernel/src/signal/*`
- `kernel/src/net/*`
- `kernel/src/lkm/*`
- `kernel/src/shell.rs`

`kernel.rs` 和 rCore 的接口差异按下面方式处理：

| `kernel.rs` 形态 | rCore 恢复模块形态 | 处理方式 |
| --- | --- | --- |
| `std::sync::Mutex`、`RwLock`、`Condvar` | `SpinLock`、`SpinNoIrqLock`、`Condvar`、async `Waker` | 不直接复制同步代码，只迁移“无 lost wakeup、timeout 清理、精确 wake 数”等语义。 |
| `std::thread::park/unpark` | executor、`Future`、`Waker`、`EventBus` | 把阻塞语义落到 rCore 的 async/sleep 机制。 |
| 单体 `Kernel::dispatch_syscall` | `kernel/src/syscall/{fs,mem,proc,ipc,signal,time,net,misc,user}.rs` | 按 syscall 分类迁移检查点，保留 rCore dispatcher 结构。 |
| `VmMap`、`PgFrame`、`SharedPage` | `rcore_memory::MemorySet`、handler、`CowExt`、`SharedGuard` | 用 rCore 页表和 handler 表达 VM 语义，不复制模拟结构。 |
| `FHandle`、`PipeNode`、`EpInst` | `FileHandle`、`Pipe`、`FileLike`、`EpollInstance` | 名称相近但结构不同，迁移 fd/offset/lifetime/epoll 语义。 |
| `Result<T, &'static str>` 或布尔失败 | `SysResult`、`VMResult`、`FsError` | 在 rCore 层转换为已有错误类型，避免新错误通道。 |
| `HashMap` 为主的模拟状态 | `BTreeMap`、`Arc`、spin lock、全局表 | 保留 rCore 数据结构，只检查生命周期和边界行为。 |

因此，“接口问题”不是新增一个 `kernel.rs` 到 rCore 的适配层，也不是恢复后再去寻找接口阻塞。接口处理的核心是建立逐段映射：找到 `kernel.rs` 中的语义责任，再落到恢复模块的既有 public API 或内部函数上。完全按匹配上游恢复的 rCore 模块视为可运行基线，后续工作只讨论如何把 `kernel.rs` 的行为对齐迁移进这个基线。

## 逐段映射

| `kernel.rs` 行段 | `kernel.rs` 主要内容 | 恢复后的 rCore 对应位置 | 接口处理和迁移目标 |
| --- | --- | --- | --- |
| `1-428` | 常量、syscall 编号、文件 flags、signal/capability/VM flags、全局锁、基础事件/ring 声明。 | `kernel/src/consts.rs`、`kernel/src/fs/fcntl.rs`、`kernel/src/fs/ioctl.rs`、`kernel/src/process/abi.rs`、`kernel/src/signal/*`、`kernel/src/syscall/mod.rs`、`kernel/src/syscall/*`。 | 常量不集中恢复到一个文件，而是保留 rCore 原来的分布。`KernLock` 是模拟内核的大锁，不迁移成真实 rCore 全局锁；只把“锁状态恢复、嵌套计数不破坏”的语义用于审计 sync/trap/driver 路径。 |
| `429-904` | `SyncQueue`、event bus、semaphore、futex table。 | `kernel/src/sync/condvar.rs`、`kernel/src/sync/event_bus.rs`、`kernel/src/sync/semaphore.rs`、`kernel/src/sync/mutex.rs`、`kernel/src/process/futex.rs`。 | `std::Condvar/thread` 语义改写为 rCore 的 `Condvar`、`EventBus`、`Future<Waker>`。重点接口是 `Condvar::wait_events/wait_timeout/notify_n`、`Semaphore` guard、`Futex::wait/wake`。语义目标：无 lost wakeup、timeout 后清理 waiter、`wake_count == 0` 返回 0、wake 返回真实唤醒数量。 |
| `916-1689` | 地址转换、物理帧、VM region/map、用户拷贝、COW/shared page、kernel stack。 | `crate/memory/src/addr.rs`、`crate/memory/src/memory_set/*`、`crate/memory/src/cow.rs`、`crate/memory/src/paging/*`、`kernel/src/memory.rs`、`kernel/src/arch/*/paging.rs`。 | `p2v/v2p/k_off` 对应 `phys_to_virt/virt_to_phys/kernel_offset`。`VmMap` 对应 `MemorySet`，`VmRegion` 对应 `MemoryArea`，`FramePool` 对应 `GlobalFrameAlloc`，`SharedPage` 对应 `CowExt`/`SharedGuard`。语义目标：checked arithmetic、半开区间、拒绝跨 kernel space、refcount 不 underflow、user copy 非法范围返回失败。 |
| `1692-2205` | heap、circular buffer、slab、ELF 校验、网络 checksum、调度辅助算法。 | `kernel/src/memory.rs::init_heap/enlarge_heap`、`buddy_system_allocator`、`kernel/src/process/structs.rs`、`kernel/src/net/*`、`kernel/src/util/mod.rs`。 | heap/allocator 只保留 rCore 原有 allocator 接口。ELF 加载语义落到 `ElfExt` 和 `ProcInitInfo`。checksum/varint/模拟 slab 等若没有 rCore 调用面，不应为了“对应”而强行移植。语义目标：越界、溢出、异常 ELF 元数据不能 panic。 |
| `2208-2991` | 文件句柄、pipe、`FLike`、epoll、terminal/ioctl 结构。 | `kernel/src/fs/file.rs`、`kernel/src/fs/pipe.rs`、`kernel/src/fs/file_like.rs`、`kernel/src/fs/epoll.rs`、`kernel/src/fs/fcntl.rs`、`kernel/src/fs/ioctl.rs`、`kernel/src/fs/devfs/tty.rs`、`kernel/src/syscall/fs.rs`。 | `FHandle` 对应 `FileHandle`，`FLike` 对应 `FileLike`，`PipeNode` 对应 `Pipe`，`EpInst` 对应 `EpollInstance`。接口目标：fd 通过 `Process.files` 管理，syscall 走 `syscall/fs.rs`。语义目标：dup 共享 open-file description offset 但 fd flags 独立、append 后 offset 到新 EOF、negative seek 拒绝、pipe 无 reader 时 write 报错、epoll ADD 重复拒绝、DEL 清理 ready/control state、dup 后 epoll registration 共享。 |
| `2992-3214` | `Channel`，模拟终端/管道式收发和关闭语义。 | `kernel/src/fs/devfs/tty.rs`、`kernel/src/fs/devfs/serial.rs`、`kernel/src/drivers/serial/*`、`kernel/src/trap.rs::serial`、必要时参考 `kernel/src/fs/pipe.rs`。 | rCore 没有同名 `Channel` 模块，不新增一层。它的语义映射到 TTY/serial 输入缓冲和 pipe-like blocking。语义目标：关闭后写入失败或返回 0、批量写唤醒足够等待者、receiver 不留下 stale waiter。 |
| `3216-4154` | page cache、kernel object registry、block cache、mount table、I/O queue、disk 模拟。 | `rcore-fs` 相关 crate、`kernel/src/fs/mod.rs`、`kernel/src/fs/device.rs`、`kernel/src/drivers/block/*`、`kernel/src/drivers/provider.rs`、`kernel/src/syscall/fs.rs`。 | 这些在 rCore 中主要由外部 `rcore-fs` 和驱动层承担，不按模拟结构恢复。语义只作为审计参考：cache 容量为 0 不应存储、dirty/sync 路径不破坏锁状态、mount 解析按路径组件边界匹配、block index/hash/invalidate 语义一致、I/O queue 不自死锁。 |
| `4158-4386` | IPC permission、SysV semaphore array/context、shared memory context。 | `kernel/src/ipc/semary.rs`、`kernel/src/ipc/shared_mem.rs`、`kernel/src/ipc/mod.rs`、`kernel/src/syscall/ipc.rs`。 | `SemArr` 对应 `SemArray`，`SemCtx` 对应 per-process `SemProc`，`ShmCtx/ShmTag` 对应 `ShmProc`/`ShmIdentifier` 和 `SharedGuard<GlobalFrameAlloc>`。语义目标：拒绝 `nsems == 0`、existing key 请求更大 size/count 时失败或明确处理、remove 清理 stale undo、drop replay 完整 undo magnitude、`IPC_PRIVATE` 不复用普通 key。 |
| `4387-4776` | 进程初始化、capability、signal set/action、timer entry。 | `kernel/src/process/abi.rs`、`kernel/src/process/proc.rs`、`kernel/src/process/thread.rs`、`kernel/src/signal/mod.rs`、`kernel/src/signal/action.rs`、`kernel/src/syscall/signal.rs`、`kernel/src/syscall/time.rs`、`kernel/src/trap.rs`。 | `ProcInit` 对应 `ProcInitInfo`/`StackWriter`，`SigSet/SigAction` 对应 `Sigset`/`SignalAction`/`Siginfo`。capability 在该 rCore 基线中没有完整对应面，除非真实运行需要，不新增 capability 子系统。语义目标：用户栈布局 checked subtraction、signal 0 和越界 signal 规则、`SIGKILL/SIGSTOP` 不可捕获、timer deadline 溢出安全。 |
| `4777-5191` | context、trap controller、clock、serial helper。 | `kernel/src/trap.rs`、`kernel/src/arch/*/interrupt/*`、`kernel/src/arch/*/signal.rs`、`kernel/src/arch/*/syscall.rs`、`trapframe` crate、`kernel/src/drivers/serial/*`。 | `Context/TrapCtl` 是模拟抽象；rCore 使用 `trapframe::UserContext`、架构 trap handler 和 `trap.rs::{timer,serial}`。语义目标：trap/IRQ 进入退出恢复原状态、page fault dispatch 不被其他 vector 分支吞掉、tick 到时间转换不溢出、`\r` 映射为 `\n`。 |
| `5194-6054` | scheduler、run queue、task、task table、fork/reap、fd table。 | `kernel/src/process/thread.rs`、`kernel/src/process/proc.rs`、`kernel/src/process/futex.rs`、executor crate、`trapframe::UserContext`、`kernel/src/syscall/proc.rs`、`kernel/src/syscall/fs.rs`。 | `TaskTable/Task` 对应 `Process`、`Thread`、全局 `PROCESSES/THREADS`、executor spawn/yield。`RunQueue` 不直接迁移成独立模块。语义目标：parent-child 只链接一次、reap 清理 parent child list、pid/tid 生命周期一致、fd close/dup/cloexec 改真实 fd state、priority/vruntime 算术不 underflow/overflow。 |
| `6055-7528` | 单体 `Kernel` 门面和 syscall 实现。 | `kernel/src/syscall/mod.rs`、`kernel/src/syscall/fs.rs`、`kernel/src/syscall/mem.rs`、`kernel/src/syscall/proc.rs`、`kernel/src/syscall/ipc.rs`、`kernel/src/syscall/signal.rs`、`kernel/src/syscall/time.rs`、`kernel/src/syscall/net.rs`、`kernel/src/syscall/misc.rs`、`kernel/src/syscall/user.rs`。 | 单体 dispatcher 拆分到 rCore 既有 syscall 模块。接口目标：保留 `handle_syscall` 和 `Syscall::{sys_*}` 的结构。语义目标：参数校验、用户指针校验、errno 映射、fd/task/vm side effect 与 Linux-like 语义一致。 |
| `7529-7659` | access validation、utility encoder/checksum、pattern scan。 | `kernel/src/memory.rs::access_ok/copy_from_user/copy_to_user`、`kernel/src/syscall/user.rs`、`kernel/src/util/mod.rs`，部分 checksum 对应 `kernel/src/net/*`。 | `validate_access/check_access/cfu/ctu` 对应 `access_ok` 和 `UserPtr`。不把所有工具函数迁移进 rCore；只有有调用面的工具才落地。语义目标：半开用户范围允许 `end == user_limit`、拒绝 overflow、`max_matches == 0` 不产生结果。 |
| `7662-8071` | address space、process group/session、wait queue、resource limits。 | `crate/memory/src/memory_set/*`、`kernel/src/process/proc.rs`、`kernel/src/process/thread.rs`、`kernel/src/syscall/proc.rs`、`kernel/src/syscall/misc.rs`、`kernel/src/sync/condvar.rs`、`kernel/src/sync/event_bus.rs`。 | `AddrSpace` 对应 `MemorySet`，`ProcessGroup` 对应 `Pgid` 和 process table，`WaitQueue` 对应 eventbus/condvar/executor wait，resource limit 对应 `RLimit` 和相关 syscall。语义目标：fork 后 shared writable refcount 只增加一次、split/unmap 不重叠且不溢出、wait filtering 正确、resource limit 边界比较一致。 |
| `8072-8338` | bit utilities、buddy allocator。 | `buddy_system_allocator`、`bitmap-allocator`、`crate/memory/src/*`、`kernel/src/memory.rs`。 | 不为模拟 buddy allocator 新增 rCore 模块。真实帧分配由 `FrameAlloc`/`GlobalFrameAlloc` 和 allocator crate 负责。语义目标：align_up checked arithmetic、double free 不破坏 free list、buddy 地址按 allocator base 计算、大 order/align 输入失败而不是 panic。 |

## 恢复模块和调用面

Phase 1 恢复后，这些路径会重新进入真实 rCore 编译：

| 恢复路径 | 被谁调用 | 与 `kernel.rs` 的关联方式 |
| --- | --- | --- |
| `crate/memory/` | `kernel/src/memory.rs`、arch paging、syscall mem、process ELF/mmap、LKM/RVM。 | 对齐 `VmMap`、COW、frame/refcount、user/kernel 边界语义。 |
| `kernel/src/memory.rs` | arch trap/page fault、drivers DMA/provider、syscall user access、process VM。 | 对齐 `p2v/v2p/k_off`、`FramePool`、`KStk`、user copy helper。 |
| `kernel/src/sync/` | drivers、fs、process、signal、syscall blocking、logging。 | 对齐 `SyncQueue`、`EvBus`、`Sema` 的等待/唤醒合同。 |
| `kernel/src/process/` | syscall dispatcher、signal、trap current thread、fs fd table、shell。 | 对齐 `Task`、`TaskTable`、futex、fork/wait/fd 生命周期。 |
| `kernel/src/fs/` | syscall fs/net/proc、shell、trap serial、devfs。 | 对齐 `FHandle`、pipe、epoll、terminal/channel 行为。 |
| `kernel/src/ipc/` | syscall ipc、process clone/drop、shared memory mmap。 | 对齐 SysV semaphore 和 shm 的 ID、undo、size 语义。 |
| `kernel/src/trap.rs` | arch interrupt handler、time syscall、serial/TTY。 | 对齐 tick/timer/serial/trap 状态语义。 |

## 迁移顺序

1. 已从匹配上游恢复缺失 rCore 文件，未修改 `kernel/src/kernel.rs`，恢复模块面作为可运行基线。
2. 按本文和 [docs/migration-records/README.md](/home/zcychar/chaos/docs/migration-records/README.md) 的记录，把 `kernel.rs` 的语义责任拆成迁移项：同步、VM、文件、IPC、signal/timer、process/syscall 等。
3. 对每个迁移项先确定 rCore 落点：public API、内部 helper、syscall handler、process state，或确认该模拟语义在真实 rCore 中没有迁移价值。
4. 对需要改源码的迁移项，先报告目标文件/位置、迁移语义、当前 rCore 表达方式、预期对齐结果和最小修改范围，等待批准后再改。
5. 每批迁移后用 `cd kernel && make build ARCH=riscv64` 和 `cd kernel && make run ARCH=riscv64 GRAPHIC=off` 验证真实 rCore 基线仍可构建运行；`chaos-tests` 只作为 `kernel.rs` 行为参考。
