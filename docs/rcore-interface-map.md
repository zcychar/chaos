# rCore 接口映射

本文把 [docs/rcore-coupling-plan.md](/home/zcychar/chaos/docs/rcore-coupling-plan.md) 细化为接口级核对清单。原则是：先恢复匹配上游的 rCore 接口形状，并把它视为可运行基线；随后按职责把 `kernel.rs` 中更完整或更稳健的行为语义迁移到对应 rCore 模块。

如果需要按 `kernel.rs` 行段查对应的恢复模块和接口转换关系，先看 [docs/kernel-to-rcore-map.md](/home/zcychar/chaos/docs/kernel-to-rcore-map.md)。

## 事实来源

匹配上游 commit：

```text
rcore-os/rCore 66cb4181ec6d3336d507c7c1ff100127f56fcc0a
```

行为参考：

```text
kernel/src/kernel.rs
```

说明：`chaos-tests` 只用于理解和回归 `kernel.rs` 这个独立模拟内核，不作为耦合后 rCore 项目的验收门槛。

当前恢复状态：

```text
Phase 1 paths have been restored from rCore commit 66cb4181ec6d3336d507c7c1ff100127f56fcc0a.
Restored file hashes match upstream objects.
```

## 先恢复、再迁移对齐

不要为缺失 rCore 模块发明替代 API。当前内核树已经在很多地方导入这些 API，尤其是：

- `kernel/src/syscall/*`
- `kernel/src/arch/*`
- `kernel/src/drivers/*`
- `kernel/src/signal/*`
- `kernel/src/net/*`
- `kernel/src/lkm/*`
- `kernel/src/shell.rs`

第一轮已精确恢复上游 API 形状。后续不以寻找接口阻塞为目标，而是按 `kernel.rs` 的职责映射做语义迁移。任何源码修改都应先说明目标文件/位置、迁移语义、当前 rCore 表达方式、预期对齐结果和最小修改范围，并遵守“先报告、等批准、再修改”的流程。

## 恢复后的 API 面

### `crate/memory`

主要公开 API：

- `PhysAddr`、`VirtAddr`、`PAGE_SIZE`
- `Page`、`PageRange`
- `VMError`、`VMResult`
- `paging::{PageTable, Entry, PageTableExt, MockPageTable}`
- `memory_set::{MemorySet, MemoryArea, MemoryAttr}`
- `memory_set::handler::{AccessType, MemoryHandler, FrameAllocator, ByFrame, Delay, File, Linear, Shared, SharedGuard}`
- `cow::CowExt`
- `swap::{SwapExt, SwapManager, Swapper}`

内核调用面：

- `kernel/src/memory.rs`
- `kernel/src/arch/*/paging.rs`
- `kernel/src/syscall/mem.rs`
- `kernel/src/syscall/net.rs`
- `kernel/src/process/structs.rs`
- `kernel/src/lkm/kernelvm.rs`
- `kernel/src/rvm/*`

可从 `kernel.rs` 借鉴的行为合同：

- user/kernel 边界检查必须使用 checked arithmetic。
- VM region 是半开区间，相邻 region 不应被视为重叠。
- mapping insert 必须拒绝溢出或跨入 kernel space 的范围。
- frame refcount 不能 underflow。
- COW source refcount 不能被减到 0 以下。
- 大 alignment/order 值应干净失败，不能 panic。

迁移对齐重点：

- `kernel/src/memory.rs:196` 的 `access_ok` 直接计算 `addr + len`，需要按 `kernel.rs` 的边界语义检查 overflow。
- `crate/memory/src/memory_set/mod.rs` 多处会做地址 round 和加法，需要核查 overflow 和半开区间不变量。

### `kernel/src/memory.rs`

主要公开 API：

- `MemorySet`
- `FrameAlloc`、`FRAME_ALLOCATOR`
- `phys_to_virt`、`virt_to_phys`、`kernel_offset`
- `GlobalFrameAlloc`
- `alloc_frame`、`dealloc_frame`、`alloc_frame_contiguous`
- `KernelStack`
- `handle_page_fault`、`handle_page_fault_ext`
- `init_heap`、`enlarge_heap`
- `access_ok`、`read_user_fixup`、`copy_from_user`、`copy_to_user`

内核调用面：

- 架构 paging 和 trap handler。
- driver DMA/provider 路径。
- syscall 用户指针校验。
- 进程 VM 创建和 mmap 路径。

可从 `kernel.rs` 借鉴的行为合同：

- 地址校验接受最后一个有效用户字节，但拒绝 overflow。
- 用户拷贝 helper 对非法范围返回失败，不能 panic。
- contiguous allocation 应校验 alignment/order 输入。

### `kernel/src/sync`

主要公开 API：

- `SpinLock<T>`、`SpinNoIrqLock<T>`、`SleepLock<T>`
- `Mutex<T, S>`、`MutexGuard`
- `MutexSupport`、`Spin`、`SpinNoIrq`、`FlagsGuard`
- `Condvar`
- `Semaphore`、`SemaphoreGuard`
- `Event`、`EventBus`、`wait_for_event`

内核调用面：

- logging、console、drivers、network structs、文件/syscall blocking、process state、signal state、block devices、LKM manager。

可从 `kernel.rs` 借鉴的行为合同：

- signal 发生在 wait 前时不能 lost wakeup。
- `wait_events` 返回后必须清理所有队列中的 stale registration。
- timeout wait 必须清理 timed-out waiter。
- 在同步保护中睡眠前必须释放受保护状态。
- `notify_n(0)` 应唤醒 0 个等待者并返回 0。

迁移对齐重点：

- `kernel/src/sync/condvar.rs` 中 `wait_events`、`wait`、`wait_timeout` 的线程入队、睡眠和队列清理逻辑有大量注释状态，需要重点核查。
- `kernel/src/sync/event_bus.rs` 需要检查 stale callback 清理和精确事件投递。
- `kernel/src/sync/semaphore.rs` 需要检查 remove/wakeup 行为和 guard release 行为。

### `kernel/src/process`

主要公开 API：

- `Pid`、`Pgid`、`Process`
- `process_of`、`process`、`process_group`、`add_to_process_table`
- `Thread`、`ThreadContext`、`ThreadInner`、`Tid`
- `spawn`、`yield_now`
- `Futex`、`Waiter`
- `ProcInitInfo`
- `ElfExt`、`INodeForMap`
- `current_thread`、`init`

内核调用面：

- syscall dispatch 和 process syscall。
- signal delivery。
- trap handler 和 current-thread lookup。
- FS mmap 和 epoll 集成。
- shell 初始化。

可从 `kernel.rs` 借鉴的行为合同：

- fork 只链接一次 child，并保持 parent-child 不变量。
- reap 必须从 parent list 中移除 child。
- standard signal 的重复 pending 应 coalesce。
- `FD_CLOEXEC` 状态变更应实际修改 fd state。
- process group 和 wait filtering 应保持类 Linux 语义。
- negative priority 和 vruntime 算术不能 panic 或 underflow。

迁移对齐重点：

- `process/futex.rs` 需要检查 timeout waiter removal；wait timeout 后，该 waiter 不应留在 futex queue 中等待后续 wake。
- `process/proc.rs` 和 `process/thread.rs` 需要对照 parent-child 生命周期、fd table clone/drop 行为。

### `kernel/src/fs`

主要公开 API：

- `ROOT_INODE`、`FOLLOW_MAX_DEPTH`、`INodeExt`
- `FileHandle`、`OpenOptions`、`SeekFrom`
- `FileLike`
- `Pipe`
- `Pseudo`
- `EpollInstance`、`EpollEvent`、`EpollData`
- `fcntl` 常量：`F_DUPFD`、`F_GETFD`、`F_SETFD`、`F_GETFL`、`F_SETFL`、`F_DUPFD_CLOEXEC`、`FD_CLOEXEC`、`O_NONBLOCK`、`O_APPEND`、`O_CLOEXEC`
- `ioctl` 常量和结构：`Termios`、`Winsize`
- devfs 节点：`Serial`、`TTY`、`ShmINode`、`RandomINode`、`Fbdev`

内核调用面：

- `kernel/src/syscall/fs.rs`
- `kernel/src/syscall/net.rs`
- `kernel/src/syscall/proc.rs`
- `kernel/src/shell.rs`
- `kernel/src/trap.rs`
- serial driver 和 epoll wakeup 路径。

可从 `kernel.rs` 借鉴的行为合同：

- dup 出来的 fd 共享 open-file description offset，但保留各自的 close-on-exec 状态。
- append write 应定位到新文件末尾，并把 offset 更新到新末尾。
- negative seek 应返回错误，不能 cast 成巨大的 unsigned offset。
- write/mmap offset 算术不能 overflow。
- read end 关闭后，pipe write 应返回错误。
- pipe endpoint clone/drop 不能过早关闭原 endpoint。
- epoll ADD 已存在 fd 应拒绝。
- epoll DEL 应清理 ready 和 control state。
- duplicated epoll instance 应共享 registration state。

迁移对齐重点：

- `fs/file.rs:146` 附近 seek 代码把 signed position cast 成 `u64`，需要检查 negative seek。
- `fs/file.rs:214` 用多段地址计算 mmap file end，需要检查 overflow。
- `fs/pipe.rs:96` write 没有检查 read end 是否关闭。
- `fs/epoll.rs:13` 的 `Clone` 创建空实例，不能保持 shared registration state。
- `fs/epoll.rs:25` ADD 直接 insert，没有拒绝已注册 fd。
- `fs/epoll.rs:43` DEL 只移除 `events`，还需要按目标语义考虑 ready/control list 清理。

### `kernel/src/ipc`

主要公开 API：

- `SemProc`、`ShmProc`
- `SemArray`、`SemidDs`、`IpcPerm`
- `ShmIdentifier`

内核调用面：

- `kernel/src/syscall/ipc.rs`
- process clone/drop state。
- 通过 `GlobalFrameAlloc` 参与 shared memory mmap。

可从 `kernel.rs` 借鉴的行为合同：

- 创建 semaphore array 时 `nsems == 0` 应拒绝。
- existing key lookup 应拒绝请求更大的 semaphore count。
- semaphore id remove/reuse 时应清理 stale undo state。
- drop 应 replay 完整 undo magnitude，不只处理 magnitude 1。
- private shared memory key 应创建唯一 segment。
- existing shared memory key 应拒绝更大的 size 请求。

迁移对齐重点：

- `ipc/semary.rs:101` 返回 existing semaphore array 时没有检查请求的 `nsems` 是否大于已有集合。
- `ipc/semary.rs:108` 允许创建 zero-length semaphore array。
- `ipc/mod.rs:80` 只处理 semaphore undo 值 `1`，其他 magnitude 走 `unimplemented!`。
- `ipc/mod.rs` 需要检查 id remove/reuse 后 stale undo cleanup。

### `kernel/src/trap.rs`

主要公开 API：

- `TICK`、`TICK_ALL_PROCESSORS`、`TICK_ACTIVITY`、`NAIVE_TIMER`
- `wall_tick`、`cpu_tick`、`do_tick`、`uptime_msec`
- `timer`
- `serial`

内核调用面：

- 架构 interrupt handler。
- time syscall。
- misc syscall 的 polling/timeout loop。
- serial driver 和 TTY input。

可从 `kernel.rs` 借鉴的行为合同：

- tick 到 millisecond 的转换不能因 overflow panic。
- timer expiry 和 deadline 算术需要处理边界值。
- serial carriage return 映射到 newline。
- trap/page-fault dispatch 应保持既有 IRQ/active state。

迁移对齐重点：

- `trap.rs:30` 使用 `wall_tick() * USEC_PER_TICK`，应按目标语义考虑 checked 或 saturating arithmetic。
- 架构 trap handler 不在缺失源码面内，但依赖恢复后的 `trap.rs` 和 `memory.rs`，需要单独核查 state restoration。

## 当前调用压力

缺失模块不是孤立的。当前已存在文件已经通过这些方式依赖它们：

- `kernel/src/syscall/fs.rs` 使用 `fs`、`MemorySet`、`Condvar`、`TICK_ACTIVITY`、`EpollInstance`、fcntl 常量和 `Process`。
- `kernel/src/syscall/mod.rs` 使用 `EpollEvent`、`copy_from_user`、`MemorySet`、`process::*` 和 sync guard。
- `kernel/src/syscall/ipc.rs` re-export `ipc::*` 并使用 `GlobalFrameAlloc`。
- `kernel/src/syscall/mem.rs` 使用 `GlobalFrameAlloc`。
- `kernel/src/syscall/net.rs` 使用 `FileLike` 和 `MemorySet`。
- `kernel/src/syscall/signal.rs` 使用 `process::*`。
- `kernel/src/syscall/time.rs` 使用 `trap::wall_tick`。
- `kernel/src/signal/mod.rs` 使用 `process`、`process_of`、`Process`、`Thread` 和 sync 类型。
- 架构 interrupt handler 使用 `process::thread::Thread`、`trap::timer`、`memory::handle_page_fault` 和 `read_user_fixup`。
- driver 使用 `phys_to_virt`、`virt_to_phys`、`SpinLock`、`SpinNoIrqLock`、`Condvar` 和 `trap::serial`。

因此 Phase 1 必须先恢复上游 API 面，再进行 `kernel.rs` 语义迁移。

## 迁移对齐顺序

Phase 1 恢复后，建议按以下顺序迁移和对齐语义：

1. `kernel/src/sync/condvar.rs`、`kernel/src/process/futex.rs`
   - 风险：stale waiter、lost wakeup、timeout cleanup。
2. `kernel/src/memory.rs`、`crate/memory/src/memory_set/*`、`crate/memory/src/cow.rs`
   - 风险：overflow、非法用户范围、refcount underflow。
3. `kernel/src/fs/file.rs`、`fs/pipe.rs`、`fs/epoll.rs`
   - 风险：fd sharing、negative seek、closed pipe write、epoll state sharing。
4. `kernel/src/ipc/*`
   - 风险：zero-size create、stale undo entry、existing-key size。
5. `kernel/src/process/*`、`kernel/src/syscall/proc.rs`
   - 风险：parent-child invariant、wait 语义、pgid/session state。
6. `kernel/src/trap.rs`、`kernel/src/signal/*`、architecture handlers
   - 风险：tick overflow、timer deadline、IRQ state preservation。

## 批准边界

下一次恢复源码的文件系统变更需要明确批准。批准内容应提到 [docs/rcore-coupling-plan.md](/home/zcychar/chaos/docs/rcore-coupling-plan.md) 中的 Phase 1 路径集合。
