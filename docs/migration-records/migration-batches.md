# 迁移批次执行单

本文把 `function-index.md` 中的 `NEEDS_APPROVAL` 和高优先级 `MIGRATION_PENDING` 项整理成可执行批次。每个批次都满足源码修改前的记录要求：目标文件/行、迁移语义、当前 rCore 表达方式、预期对齐结果、最小修改范围。

当前状态：B01 已按批准完成源码迁移；B02 已完成部分源码对齐，并已通过 QEMU 验证进入 busybox shell；B03 已完成 append/seek、mmap range、epoll state、pipe endpoint lifecycle 和 TTY/channel wakeup 子项；B04 IPC 已完成；B05 process 的 process brk / heap boundary、fd lifecycle/cloexec、user stack 错误传播、ELF bounds 和 fork/wait parent-child 核心子项均已完成。B02 后续不能再按“整批批准”推进，必须围绕新的具体故障或子模块语义差距提出行级修复报告。

## 批次总览

| 批次 | 子系统 | 目标 | 状态 |
| --- | --- | --- | --- |
| B01 | sync + futex | 等待/唤醒、timeout cleanup、wake count 对齐。 | `MIGRATED` |
| B02 | memory + trap | VM range、COW、user access、tick conversion、init-time stack、RISC-V PTE update、file-backed LOAD 填充对齐。 | `PARTIAL_MIGRATED` / `QEMU_SHELL_REACHED` |
| B03 | fs | file offset、mmap range、pipe、epoll、TTY/channel 对齐。 | `PARTIAL_MIGRATED`；B03.1-B03.5 已完成 |
| B04 | ipc | SysV semaphore undo、zero-size、existing-key、shm key/size 对齐。 | `MIGRATED` |
| B05 | process | stack init、process brk、fork/wait/fd lifecycle、ELF bounds 对齐。 | `MIGRATED` |
| B06 | deferred/no-direct | swap、mock、fbdev、random 等无直接迁移项确认。 | `NO_DIRECT_PORT` / `MIGRATION_PENDING` |

## B01：sync + futex

详细函数级记录见 [batches/B01-sync-futex.md](batches/B01-sync-futex.md)。

### B01.1 `Condvar::wait_events`

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/sync/condvar.rs:55` |
| `kernel.rs` 来源 | `SyncQueue::wait_events` |
| 迁移语义 | 多队列等待返回后，不在未触发的队列中留下 stale waiter；等待过程不能 lost wakeup。 |
| 当前 rCore 表达 | `wait_events` 中 thread/token registration 逻辑仍是注释形态，循环中只反复 lock/unlock 队列。 |
| 预期对齐结果 | 每次 wait 注册可识别 waiter；返回前从所有注册队列清理该 waiter；条件满足时返回具体结果。 |
| 最小修改范围 | `Condvar::wait_events` 内的 waiter token、注册、清理逻辑；保持 `Condvar` public API。 |
| 状态 | `MIGRATED` |

### B01.2 `Condvar::wait_timeout`

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/sync/condvar.rs:109` |
| `kernel.rs` 来源 | `SyncQueue::wait_timeout` |
| 迁移语义 | timeout 后清理当前 waiter；返回值区分真实 wake 与 timeout。 |
| 当前 rCore 表达 | `wait_timeout` 的 token push/remove 逻辑是注释；timeout 只通过时间差返回 `None`。 |
| 预期对齐结果 | timeout 不留下 waiter；被 notify 唤醒时返回 `Some(guard)`，超时返回 `None`。 |
| 最小修改范围 | `Condvar::wait_timeout` wait queue registration/removal。 |
| 状态 | `MIGRATED` |

### B01.3 `Futex::wake`

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/process/futex.rs:36` |
| `kernel.rs` 来源 | `FutexTable::ftx_wake` |
| 迁移语义 | `wake_count == 0` 返回 0；返回值等于实际移除并唤醒的 waiter 数。 |
| 当前 rCore 表达 | `for i in 0..wake_count` pop queue；大体计数正确，但需和 timeout cleanup 联动，避免唤醒过期 waiter。 |
| 预期对齐结果 | wake 不命中过期 waiter；计数只包含实际有效唤醒。 |
| 最小修改范围 | `Futex::wake` 和 waiter 状态判断。 |
| 状态 | `MIGRATED` |

### B01.4 `FutexFuture::poll`

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/process/futex.rs:62` |
| `kernel.rs` 来源 | futex timeout cleanup audit 语义 |
| 迁移语义 | timeout 后从 `FutexInner.waiters` 中移除当前 waiter。 |
| 当前 rCore 表达 | timeout 时只设置 `inner.woken = true` 并返回 `ETIMEDOUT`，没有从 futex queue 移除。 |
| 预期对齐结果 | 超时 waiter 不会被后续 wake 再次取出；queue 中只保留仍有效的 waiters。 |
| 最小修改范围 | `FutexFuture::poll` timeout 分支。 |
| 状态 | `MIGRATED` |

## B02：memory + trap

详细函数级/子模块级记录见 [batches/B02-memory-trap.md](batches/B02-memory-trap.md)。

### B02.1 COW refcount

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `crate/memory/src/cow.rs:91`、`:169`、`:178`、`:186`、`:195` |
| `kernel.rs` 来源 | `SharedPage::fault`、`PgFrame::up/down` |
| 迁移语义 | COW source frame refcount 不 underflow；fork/source frame 计数增减只发生一次。 |
| 当前 rCore 表达 | `read_increase` / `write_increase` 已使用 saturating add；`read_decrease` / `write_decrease` 已经通过内部 `decrease` helper 避免缺失条目和 0 count underflow。 |
| 预期对齐结果 | zero count decrement 干净失败或保持不变；COW fault 后权限、frame、计数一致。当前仍需后续在真实 fork/COW 路径中复核完整生命周期。 |
| 最小修改范围 | `FrameRcMap` decrement helper 和 `CowExt::page_fault_handler` 调用路径。 |
| 状态 | `PARTIAL_MIGRATED` |

### B02.2 MemorySet range arithmetic

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `crate/memory/src/memory_set/mod.rs:180`、`:197`、`:232`、`:246`、`:379` |
| `kernel.rs` 来源 | `VmMap::insert`、`AddrSpace::unmap_range`、`AddrSpace::split_region` |
| 迁移语义 | start/end/page rounding 使用 checked arithmetic；半开区间和 split/unmap 不变量保持一致。 |
| 当前 rCore 表达 | `align_up`、`check_read_array`、`find_free_area`、`push`、`pop`、`pop_with_split` 已加入 checked/早返回；`with` 已改为 `with<R>` 传回 closure 返回值。 |
| 预期对齐结果 | overflow 或非法范围返回错误/失败路径，而不是 panic 或 wrap；split 后不重叠。当前仍需结合运行 fault 继续核对页表 backend 与 SATP 可见性。 |
| 最小修改范围 | `find_free_area`、`push`、`pop`、`pop_with_split` 和 fault lookup 前置校验。 |
| 状态 | `PARTIAL_MIGRATED` |

### B02.3 file-backed mapping offset

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `crate/memory/src/memory_set/handler/file.rs:61`、`:94` |
| `kernel.rs` 来源 | `FLike::mmap_fl`、file mmap overflow |
| 迁移语义 | file offset + page offset checked；短读和越界干净处理。 |
| 当前 rCore 表达 | `fill_data` 已使用 `checked_sub`/`checked_add`/`checked_sub` 计算 file offset 和 read size；非法 offset zero-fill；分配失败返回 false。 |
| 预期对齐结果 | file_offset 计算不 overflow/underflow；越界部分 zero fill；非页对齐 LOAD 的有效页内数据不会被清零。当前已通过 QEMU shell 验证。 |
| 最小修改范围 | `File::fill_data` 和 fault-time read path。 |
| 状态 | `PARTIAL_MIGRATED` |

### B02.4 kernel frame/user access

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/memory.rs:77`、`:87`、`:196`、`:205`、`:222` |
| `kernel.rs` 来源 | `FramePool::get_contig`、`check_access`、`validate_access`、`cfu`、`ctu` |
| 迁移语义 | contiguous alloc 的 align/order 非法输入干净失败；user pointer range checked add，半开上界一致。 |
| 当前 rCore 表达 | `alloc_contiguous`、`dealloc` 和 `access_ok` 已加入 checked 边界；`handle_page_fault(_ext)` 已能在无 current thread 时回落 active VM。 |
| 预期对齐结果 | overflow/underflow 不 panic；非法用户范围返回 `None`/`false`；init-time fault 不再因缺少 current thread 失败。 |
| 最小修改范围 | `GlobalFrameAlloc` wrapper、`access_ok`、用户拷贝前置校验。 |
| 状态 | `PARTIAL_MIGRATED` |

### B02.5 tick conversion

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/trap.rs:30` |
| `kernel.rs` 来源 | `up_ms` |
| 迁移语义 | `wall_tick * USEC_PER_TICK / 1000` 不因乘法溢出 panic/wrap。 |
| 当前 rCore 表达 | 已改为 `saturating_mul` 后除以 1000。 |
| 预期对齐结果 | checked 或 saturating arithmetic，保持单调时间语义。 |
| 最小修改范围 | `uptime_msec`。 |
| 状态 | `MIGRATED` |

### B02.6 init-time user stack VM write

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/process/abi.rs:19`、`:92`、`:118`；`kernel/src/process/thread.rs:223` |
| `kernel.rs` 来源 | `ProcInit::push_at` |
| 迁移语义 | 用户初始栈写入必须与正在构造的 `MemorySet` 关联，不依赖还不存在的 current thread。 |
| 当前 rCore 表达 | 已新增 `push_at_in_vm` / `VmStackWriter`；`Thread::new_user_vm` 使用 VM-backed writer 写入初始栈。 |
| 预期对齐结果 | init stack 写入越过旧的 `0x3fffffff` fault；后续 checked 错误传播归入 B05 收口。 |
| 最小修改范围 | 已完成当前最小耦合；后续不在未报告情况下继续改。 |
| 状态 | `PARTIAL_MIGRATED` |

### B02.7 RISC-V PTE update

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/arch/riscv/paging.rs:81` |
| `kernel.rs` 来源 | fault 成功后 mapping 应立即对下一次执行/访问可见。 |
| 迁移语义 | 修改 PTE 后应刷新 fault page 的虚拟地址，不应把物理 frame 地址传给 `sfence.vma` wrapper。 |
| 当前 rCore 表达 | `PageEntry::update` 已改为 `sfence_vma(self.1.start_address().as_usize(), 0)`。 |
| 预期对齐结果 | `sfence.vma` vaddr 修正配合 Sv39 非 leaf PTE flags 归一化后，`0x100e8` 重复 instruction page fault 已解除。 |
| 最小修改范围 | 已完成已批准修正；后续需另报新的具体 paging/SATP 修复点。 |
| 状态 | `MIGRATED` |

## B03：fs

详细函数级/子模块级记录见 [batches/B03-fs.md](batches/B03-fs.md)。

### B03.1 append write and negative seek

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/fs/file.rs:139`、`:151`、`:160` |
| `kernel.rs` 来源 | `FHandle::write`、`FHandle::seek`、`FHandle::write_at` |
| 迁移语义 | append 写后 offset 更新到新 EOF；negative seek 返回错误；write offset + len checked。 |
| 当前 rCore 表达 | append 时用 EOF 作为写入 offset，但随后对旧 descriptor offset 加 len；seek 把 signed 结果 cast 到 `u64`。 |
| 预期对齐结果 | append 后 offset 是 append_start + written_len；seek 结果小于 0 时返回错误；write_at 不 wrap。 |
| 最小修改范围 | `FileHandle::write`、`write_at`、`seek`、`sys_lseek`。 |
| 状态 | `MIGRATED` |

### B03.2 mmap file range

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/fs/file.rs:228`、`kernel/src/syscall/mem.rs:10`、`:96` |
| `kernel.rs` 来源 | `FLike::mmap_fl`、`SYS_MMAP`、`SYS_MUNMAP` |
| 迁移语义 | mmap/munmap len、addr+len、file range checked；zero-length munmap 语义明确。 |
| 当前 rCore 表达 | 多处 `addr + len` 和 `area.end_vaddr` 直接计算。 |
| 预期对齐结果 | overflow/zero-length 返回错误，合法半开 range 映射。 |
| 最小修改范围 | `sys_mmap`、`sys_munmap`、`FileHandle::mmap`。 |
| 状态 | `MIGRATED` |

### B03.3 epoll state

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/fs/file_like.rs:20`、`kernel/src/fs/epoll.rs:10`、`:16`、`:35`、`kernel/src/syscall/fs.rs:318`、`:364` |
| `kernel.rs` 来源 | `FLike::dup`、`EpInst::control ADD/DEL` |
| 迁移语义 | duplicated epoll instance 共享 registration；ADD duplicate 拒绝；DEL 清理 events/ready/new_ctl。 |
| 修改前 rCore 表达 | `EpollInstance::clone` 返回空实例；ADD 直接 insert 覆盖；DEL 只 remove `events`；`sys_epoll_ctl` 对 DEL 也读取 event；`sys_epoll_pwait` 未限制写回数。 |
| 预期对齐结果 | dup 后 registration/ready/control state 保持共享；ADD existing 返回错误；DEL 后没有 stale ready/control fd。 |
| 最小修改范围 | `EpollInstance` state representation、`Clone`、`control`、`FileLike::dup` epoll 分支、`sys_epoll_ctl`、`sys_epoll_pwait`。 |
| 已落地结果 | epoll 内部状态改为共享锁；ADD existing 返回 `EEXIST`，DEL 清理三个集合，invalid op 返回 `EINVAL`；DEL 不强制读取 event；wait 按事件快照遍历并限制 `maxevents`。 |
| 状态 | `MIGRATED` |

### B03.4 pipe endpoint lifecycle

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/fs/pipe.rs:22`、`:36`、`:52`、`:90`、`:100`、`:141`、`kernel/src/fs/file.rs:94`、`kernel/src/fs/file_like.rs:39` |
| `kernel.rs` 来源 | `PipeNode::clone/drop`、`PipeNode::write_at` |
| 迁移语义 | clone/drop endpoint count 正确；read end 关闭后 write 返回错误；poll readiness 反映 endpoint 生命周期。 |
| 修改前 rCore 表达 | `Pipe` derive clone，但 `Drop` 总是 `end_cnt -= 1`；`can_write` 只看总端点数；`write_at` 不显式检查 live reader；poll error 固定 false。 |
| 预期对齐结果 | clone/drop 按 endpoint 方向维护 reader/writer；无 reader 写失败并 wake waiters；writer 关闭后 reader EOF/ready；poll error 可见。 |
| 最小修改范围 | `Pipe` clone/drop/state、`can_read/can_write`、`read_at/write_at/poll/async_poll`、`FileLike::write` EPIPE 桥接。 |
| 已落地结果 | `PipeData` 增加 `readers/writers`；drop saturating；write broken pipe 通过 `FileLike::write` 映射为 `SysError::EPIPE`；poll/async_poll 反映 broken pipe。 |
| 状态 | `MIGRATED` |

### B03.5 TTY/channel wakeup

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/fs/devfs/tty.rs:39`、`:68`、`:86`、`:142`，`kernel/src/sync/event_bus.rs:110` |
| `kernel.rs` 来源 | `Channel::send`、`Channel::send_batch`、terminal/channel close/wakeup |
| 迁移语义 | TTY push/read/poll 与 channel readable/wakeup 语义对齐；batch input wakeup 不遗漏 waiter。 |
| 修改前 rCore 表达 | `read_at` 只读一个 byte 且零长度读会越界；async poll 每次 pending 都 subscribe callback。 |
| 预期对齐结果 | TTY eventbus 不累积 stale callbacks；readable 清理与 buffer 状态一致。 |
| 最小修改范围 | `TtyINode` pop/read_at/async_poll，联动 `EventBus::subscribe_waker`。 |
| 已落地结果 | `read_at` 支持空 buffer 和多字节 drain；buffer 清空后清理 READABLE；TTY async poll 和 `wait_for_event` 使用 waker/mask 去重订阅。 |
| 状态 | `MIGRATED` |

## B04：ipc

详细函数级/子模块级记录见 [batches/B04-ipc.md](batches/B04-ipc.md)。

### B04.1 semaphore create/get

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/ipc/semary.rs:95`、`:117`、`kernel/src/syscall/ipc.rs:15` |
| `kernel.rs` 来源 | `SemArr::get_or_create` |
| 迁移语义 | 创建 semaphore array 时拒绝 `nsems == 0`；existing key 请求更大 count 时返回错误。 |
| 修改前 rCore 表达 | `get_or_create` existing key 直接返回 array；new array 用 `0..nsems`，允许 zero-length。 |
| 预期对齐结果 | zero-length 返回 `EINVAL`；existing key count 不满足请求时返回错误。 |
| 最小修改范围 | `SemArray::get_or_create` 和 `sys_semget` 前置校验。 |
| 已落地结果 | `sys_semget` 和 `SemArray::get_or_create` 均拒绝 `nsems == 0`；existing key 且已有 `nsems` 小于请求 `nsems` 时返回 `EINVAL`。 |
| 状态 | `MIGRATED` |

### B04.2 semaphore undo/remove

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/ipc/mod.rs:47`、`:62`、`:80`、`kernel/src/syscall/ipc.rs:56` |
| `kernel.rs` 来源 | `SemCtx::remove`、`SemCtx::drop` |
| 迁移语义 | remove 清理该 semaphore id 的 undo；drop replay 完整 undo magnitude，不只处理 `1`。 |
| 修改前 rCore 表达 | `SemProc::remove` 只移除 array；`Drop for SemProc` 对 undo 值非 `0/1` 走 `unimplemented!`。 |
| 预期对齐结果 | stale undo 不影响新 id；所有正向 undo magnitude 按次数或等价操作 replay。 |
| 最小修改范围 | `SemProc::remove`、`SemProc::drop`、必要的 bounds check。 |
| 已落地结果 | `SemProc::remove` 清理该 id undo；drop 跳过 stale id/num，并对 `op > 0` 循环 release；旧工具链不支持 `BTreeMap::retain`，使用收集 key 后 remove。 |
| 状态 | `MIGRATED` |

### B04.3 shared memory key/size

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/ipc/shared_mem.rs:27`、`kernel/src/syscall/ipc.rs:113` |
| `kernel.rs` 来源 | `shm_get_or_create` |
| 迁移语义 | `IPC_PRIVATE` 创建唯一 segment；existing key 请求更大 size 时扩展 backing size。 |
| 修改前 rCore 表达 | `new_shared_guard` 对任何 key 都查 `KEY2SHM`；existing key 直接返回 guard，不检查 size。 |
| 预期对齐结果 | private key 不复用普通 key；existing key size 不足时扩展 `SharedGuard.size`。 |
| 最小修改范围 | `ShmIdentifier::new_shared_guard`、`sys_shmget`。 |
| 已落地结果 | `sys_shmget` 拒绝 `size == 0`；`key == 0` 直接创建独立 guard；existing key 命中时扩展 `SharedGuard.size` 后返回原 guard。 |
| 状态 | `MIGRATED` |

## B05：process

详细函数级/子模块级记录见 [batches/B05-process.md](batches/B05-process.md)。

### B05.1 user stack init

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/process/abi.rs:13`、`:24`、`:63`、`:81`、`:102`、`:119`；`kernel/src/process/thread.rs:116`、`:195`、`:221` |
| `kernel.rs` 来源 | `ProcInit::push_at` |
| 迁移语义 | 用户栈布局所有 `sp` 下移和 alignment 都使用 checked subtraction；栈空间不足干净失败。 |
| 修改前 rCore 表达 | `StackWriter::push_slice` 直接 `self.sp -= len * size_of::<T>()`；`VmStackWriter` 对 size/sp/address/page prepare 失败使用 `expect/panic`；`Thread::new_user_vm` 直接在传入 `MemorySet` 上 clear/push。 |
| 预期对齐结果 | underflow/overflow 不 panic/wrap；调用方可识别失败。 |
| 最小修改范围 | `StackWriter` 和 `ProcInitInfo::push_at` 返回错误或安全 sentinel 的接口设计。 |
| 已落地结果 | 新增 `try_push_at`、`try_push_at_in_vm` 和 fallible `InitStackWriter`；`push_slice`、`push_str`、`write_bytes` 统一返回 `Result`；size 乘法、sp 下移、alignment、VA 加法、flush range、page prepare 都走 checked/error path；`Thread::new_user_vm` 在临时 `MemorySet` 上完成装载并在成功后替换传入 VM，exec 失败不提前破坏旧地址空间。 |
| 状态 | `MIGRATED` |

### B05.2 fork/wait parent-child

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/process/thread.rs:365`、`:425`、`kernel/src/syscall/proc.rs:23`、`:85`、`:151` |
| `kernel.rs` 来源 | `TaskTable::fork_task`、`TaskTable::reap` |
| 迁移语义 | fork 只链接一次 child；wait/reap 从 parent children 和 global table 清理一致。 |
| 修改前 rCore 表达 | `Thread::fork` 负责 parent-child link；`sys_wait4(pid > 0)` 在证明 parent-child 关系前读取全局进程表；`pid == 0` 没有实际 group filter；`pid < -1` 未实现；父进程 exit 不 reparent children。 |
| 预期对齐结果 | parent-child link 不重复、不泄漏；指定 pid/group wait filtering 正确。 |
| 最小修改范围 | `Thread::fork`、`sys_wait4`，必要时 `Process` helper。 |
| 已落地结果 | `sys_wait4` 改为只从当前 parent children 快照匹配 wait 目标，支持 `pid == 0` 当前 pgid 和 `pid < -1` 指定 pgid；reap 同步清理 `PROCESSES` 与 parent children；`Process::exit` 通过私有 helper 将 children 转交 init。 |
| 状态 | `MIGRATED` |

### B05.3 fd lifecycle and cloexec

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/process/proc.rs:80`、`:190`、`:206`、`:212`、`:221`；`kernel/src/process/thread.rs:332`、`:378`；`kernel/src/syscall/fs.rs:310`、`:669`、`:670`、`:900`、`:905`、`:922`、`:1162`、`:1365`；`kernel/src/syscall/proc.rs:201` |
| `kernel.rs` 来源 | `Task::close_fd`、`Task::dup_fd`、`Task::set_cloexec`、`exec` close-on-exec |
| 迁移语义 | close/dup/cloexec 实际修改 `Process.files`；exec 关闭 `FD_CLOEXEC` fd。 |
| 修改前 rCore 表达 | `sys_close` 只 remove fd；`dup_impl` 先删目标再取源，`dup2(old, old)` 会误关 fd；`sys_fcntl` 只在 `FileLike::File` 上修改 `fd_cloexec`，socket/epoll fd 无 fd-local close-on-exec；`F_DUPFD` 落到默认 `Ok(0)`；exec close loop 只看 file fd。 |
| 预期对齐结果 | fd table side effect 与 Linux-like 语义一致；epoll/file/socket 不破坏共享状态。 |
| 最小修改范围 | `Process.files` helper、`sys_close`、`dup_impl`、`sys_fcntl`、exec close loop。 |
| 已落地结果 | `Process` 增加 fd-local `fd_cloexec` 集合和 `add_file_with_cloexec/close_file/is_fd_cloexec/set_fd_cloexec` helper；fork 继承 fd flags；open/pipe/epoll create 写入 close-on-exec；close 同步清理 fd-local metadata；dup2 同 fd no-op，dup3 同 fd或非法 flags 返回 `EINVAL`；`F_DUPFD`/`F_DUPFD_CLOEXEC` 复制到不小于 arg 的空 fd；`F_GETFD/F_SETFD` 对 file/socket/epoll 均有效；exec 关闭所有 fd-local cloexec fd。 |
| 状态 | `MIGRATED` |

### B05.4 ELF bounds

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/process/structs.rs:56`、`:82`、`:132`、`:202`、`:220`；`kernel/src/process/thread.rs:155`、`:168`、`:184` |
| `kernel.rs` 来源 | `validate_elf_header` |
| 迁移语义 | program header virtual/file range checked arithmetic；farthest memory calculation不 overflow。 |
| 修改前 rCore 表达 | `ph.virtual_addr() as usize + ph.mem_size() as usize`、`ph.offset() as usize + ph.file_size() as usize`、interpreter `+ bias`、PHDR 推导和 farthest memory 直接加；`make_memory_set` 没有错误通道。 |
| 预期对齐结果 | malformed ELF 返回错误或拒绝加载，而不是 wrap/panic。 |
| 最小修改范围 | `ElfExt::make_memory_set`、`append_as_interpreter`，可能需要调整 trait 返回错误。 |
| 已落地结果 | `ElfExt::{make_memory_set,append_as_interpreter,get_phdr_vaddr}` 改为 fallible；LOAD virtual/file range、file_size <= mem_size、interpreter bias、PHDR inferred address、farthest memory 和 bias address 均 checked；无 LOAD 或空 LOAD range 返回错误；`Thread::new_user_vm` 将错误上传给 `sys_exec` 映射为 `EINVAL`。 |
| 状态 | `MIGRATED` |

### B05.5 process brk / heap boundary

| 项目 | 内容 |
| --- | --- |
| 目标文件/行 | `kernel/src/process/proc.rs:73`、`:94`；`kernel/src/process/thread.rs:339`、`:384`；`kernel/src/syscall/mod.rs:241`；`kernel/src/syscall/mem.rs:11` |
| `kernel.rs` 来源 | `VmMap::brk`、`SYS_BRK` |
| 迁移语义 | process 记录 program break；`brk(0)` 返回当前 break；增长/缩小用恢复后的 `MemorySet` 表达 heap VM 区间。 |
| 当前 rCore 表达 | 原先 `SYS_BRK` 直接返回 `ENOMEM` 并打印 unimplemented warning。 |
| 预期对齐结果 | busybox 启动期间不再因 `brk` fallback 到 unimplemented；heap 页通过 page fault lazy 分配。 |
| 最小修改范围 | `Process` brk 字段、new/fork 初始化继承、`Syscall::sys_brk` 和分发。 |
| 状态 | `MIGRATED` |

## B06：deferred/no-direct

详细 no-direct 记录见 [batches/B06-no-direct.md](batches/B06-no-direct.md)。

| 路径 | 原因 | 状态 |
| --- | --- | --- |
| `crate/memory/src/swap/*` | 上游恢复的 swap 辅助；当前真实 rCore 迁移目标不需要把 `kernel.rs` 模拟 cache/allocator 结构迁到 swap。 | `NO_DIRECT_PORT` |
| `crate/memory/src/paging/mock_page_table.rs` | 测试/mock 页表，不进入真实内核运行路径。 | `NO_DIRECT_PORT` |
| `kernel/src/fs/devfs/random.rs` | 无直接 `kernel.rs` 语义迁移；保持上游设备节点。 | `NO_DIRECT_PORT` |
| `kernel/src/fs/devfs/fbdev.rs` | 无直接 `kernel.rs` 语义迁移；保持上游 framebuffer 设备。 | `NO_DIRECT_PORT` |
| `rust-toolchain` | 工具链基线文件，不是内核语义迁移。 | `BASELINE_RESTORED` |
