# kernel.rs 重构变更对照

对照来源：

- 原始文件：`kernel/src/kernel.rs`
- 重构文件：`kernel-rewrite/kernel.rs`

本文记录结构体和公共函数层面的变化。未特别标成“删除”“新增”“改名”或“类型签名变化”的公共函数，仍保留原来的公开入口；常见改动是参数名可读化、字段名展开、注释补充、删除无效临时代码，以及把手写边界逻辑换成更直接的安全检查。

## 总览

- 结构体数量保持 74 个；其中 `EvFlag`/`EvBus`/`FlgGuard` 改名为 `EventFlag`/`EventBus`/`FlagGuard`。
- 枚举 `SocketState`、`FSeek`、`PipeDir`、`FLike` 的变体保持不变。
- 类型别名里 `EvCb` 改名为 `EventCallback`；`Tid`、`Pgid`、`SemId`、`SemNum`、`SemOp`、`ShmId` 保持不变。
- 公共函数/方法整体基本保留；移除了 4 个独立或内核辅助入口，新增 1 个调度入口，并把事件相关函数随类型改名。

## 结构体逐项

### memory / VM

| 结构体 | 改动 |
| --- | --- |
| `VmRegion` | 字段保持不变；补充半开区间语义，`overlaps`/`split_at`/`merge_with` 加强相邻区间和溢出处理。 |
| `VmMap` | 字段保持不变；插入、删除、查找空洞等逻辑改成更直接的区间操作。 |
| `ZoneInfo` | 字段保持不变；方法位置提前，保留水位线和回收目标计算。 |
| `PgFrame` | 字段 `rc` 改名为 `ref_count`；递减和条件递增避免引用计数下溢。 |
| `FramePool` | 字段 `cap` 改名为 `capacity`；连续分配校验 `align_log2`，批量分配和归还逻辑更直接。 |
| `SharedPage` | 字段 `w` 改名为 `writable`；CoW fault 通过 `PgFrame::down` 递减源引用。 |
| `KStk` | 字段保持不变；构造和 `top` 保持原语义。 |
| `SlabEntry` | 字段保持不变；分配时按需清零，释放时拒绝重复 free。 |
| `AddrSpace` | 字段保持不变；fork、CoW、protect/unmap 改为复用 `VmMap` 能力。 |
| `BuddyAllocator` | 字段保持不变；分配/释放逻辑整理，并在空闲区判断中处理溢出。 |

### sync / futex / semaphore

| 结构体 | 改动 |
| --- | --- |
| `KernLock` | 字段保持不变；`leave` 修正递归持锁时过早解锁的问题。 |
| `Spin` | 字段 `v` 改名为 `locked`；自旋锁接口保持不变。 |
| `FlagGuard` | 原 `FlgGuard` 改名；`enter` 语义保持不变。 |
| `EventFlag` | 原 `EvFlag` 改名；事件位常量保持不变。 |
| `EventBus` | 原 `EvBus` 改名；字段 `ev`/`cbs` 改为 `events`/`callbacks`，回调观察稳定事件掩码。 |
| `RegEp` | 字段保持不变；仅随 epoll 等待队列命名整理。 |
| `SyncQueue` | 字段 `q`/`eq`/`sig` 改为 `waiters`/`epoll_registrations`/`pending_signal_tokens`；修正提前唤醒 token、超时和陈旧 waiter 清理。 |
| `SemaInner` | 字段 `cnt`/`rm`/`bus` 改为 `permit_count`/`removed`/`EventBus`；语义不变但命名清晰。 |
| `Sema` | 字段保持不变；`set_val` 现在会同步清除或设置可获取事件。 |
| `SemaGuard` | 字段 `s` 改名为 `semaphore`；Drop 释放语义保持不变。 |
| `FutexBucket` | 字段保持不变；等待超时会移除 waiter，wake(0) 不再产生副作用。 |
| `FutexTable` | 字段保持不变；锁使用统一化，wait/wake/requeue 仍按地址分桶。 |

### fs / pipe / epoll / cache / disk

| 结构体 | 改动 |
| --- | --- |
| `CircBuf` | 字段 `rd`/`wr`/`cap`/`n` 改为 `read_cursor`/`write_cursor`/`capacity`/`len`；修正游标越界、零容量和 drain 长度。 |
| `FdOpt` | 字段保持不变；默认值语义保持不变。 |
| `FdState` | 字段保持不变；`flk` 标注为当前无实际使用。 |
| `FHandle` | 字段保持不变；读写、append、seek、fallocate、splice 等路径增加溢出和偏移修正。 |
| `PipeBuf` | 字段 `bus` 类型从 `EvBus` 改为 `EventBus`；读写端计数含义保持不变。 |
| `PipeNode` | 字段保持不变；clone/drop 维护端点计数，写无 reader 时返回错误。 |
| `PseudoNode` | 字段保持不变；只做参数命名和读写实现整理。 |
| `EpData` | 字段保持不变。 |
| `EpEvent` | 字段保持不变；`has` 仅参数名可读化。 |
| `EpCtlOp` | 字段/常量保持不变；epoll ctl 调用改用常量名。 |
| `EpEventMap` | 字段保持不变；插入/删除接口保持不变。 |
| `EpInst` | 字段保持不变；`control` 从 `&mut self` 改为 `&self`，依赖内部锁完成修改，并清理 DEL 的陈旧状态。 |
| `TrmIO` | 字段保持不变；默认配置保持不变。 |
| `WinSz` | 字段保持不变。 |
| `Channel` | 字段保持不变；send/send_batch/close 处理关闭态和批量唤醒更严谨。 |
| `PageCacheEntry` | 字段保持不变。 |
| `PageCache` | 字段保持不变；容量作为硬上限，所有页 pinned 时不再超容量插入。 |
| `KObjEntry` | 字段保持不变。 |
| `KObjRegistry` | 字段保持不变；注册、索引、引用计数和 GC 逻辑做了实现整理。 |
| `CacheSlot` | 字段保持不变。 |
| `CacheChain` | 字段保持不变；删除了无效注释和冗余实现。 |
| `BlockCache` | 字段保持不变；处理 width 为 0 的情况，失效路径使用与 fetch 相同的 hash 链。 |
| `MountEntry` | 字段保持不变。 |
| `MountTable` | 字段保持不变；抽出 slash 规范化和 prefix 匹配，避免 `/mnt` 错配 `/mnted`。 |
| `IoRequest` | 字段保持不变。 |
| `IoQueue` | 字段保持不变；合并相邻请求时避免持有 pending 锁后再重入。 |
| `Disk` | 字段保持不变；批量读用 checked_add 防溢出，有限读填充与单块读保持一致。 |

### IPC / SysV semaphore / shared memory

| 结构体 | 改动 |
| --- | --- |
| `IpcPerm` | 字段保持不变。 |
| `SemDs` | 字段保持不变。 |
| `SemArr` | 字段保持不变；创建/获取时校验 semaphore 数量，并更新 otime/ctime。 |
| `SemCtx` | 字段保持不变；删除数组时清理 undo 记录，drop replay 正数 undo 的完整幅度。 |
| `ShmTag` | 字段保持不变；只做参数命名可读化。 |
| `ShmCtx` | 字段保持不变；接口保持原语义。 |

### process / signal / trap / kernel

| 结构体 | 改动 |
| --- | --- |
| `CapSet` | 字段保持不变；继承和 ambient 提权受 `INHERITABLE_MASK` 与有效位约束。 |
| `SigAction` | 字段保持不变。 |
| `SigSet` | 字段保持不变；signal 0、越界信号、`SIGKILL`/`SIGSTOP` 的处理更严格。 |
| `TimerEntry` | 字段保持不变；到达 deadline 即过期，重复 timer 重置防溢出。 |
| `TimerWheel` | 字段保持不变；advance/cancel/active_count 实现整理。 |
| `ProcInit` | 字段保持不变；向下压栈使用 checked_sub，避免 underflow。 |
| `Context` | 字段保持不变；寄存器恢复顺序修正，clone/diff/hash 逻辑整理。 |
| `TrapCtl` | 字段保持不变；IRQ mask clear/set 语义、handler 状态恢复、page fault vector 处理更明确。 |
| `SchedulePolicy` | 字段保持不变；优先级 clamp 和 time slice 计算整理。 |
| `RunQueue` | 字段 `current` 从 `Option<usize>` 改为 `Option<(usize, SchedulePolicy)>`；队列按 vruntime 保持有序，yield 能保留当前调度策略。 |
| `Pid` | 字段保持不变。 |
| `TaskInfo` | 字段保持不变。 |
| `ThdCtx` | 字段保持不变。 |
| `Task` | 字段 `ev` 类型从 `EvBus` 改为 `EventBus`；减少重复加锁，修正标准信号合并、dup/dup2 和 cloexec 更新。 |
| `TaskTable` | 字段保持不变；reap 子任务时同时从父任务 children 列表移除。 |
| `Kernel` | 字段保持不变；系统调用分发、调度 tick、页分配/释放、fork/exec/wait 等实现大幅整理。 |
| `ProcessGroup` | 字段保持不变；仅补 foreground 字段说明。 |
| `WaitQueue` | 字段保持不变；`wake_filtered` 复用统一唤醒逻辑。 |
| `ResourceLimits` | 字段保持不变；接口保持原语义。 |

## 枚举和类型别名

- `SocketState`：变体保持不变，仅随网络辅助函数一起补充更清晰的命名。
- `FSeek`：变体保持不变。
- `PipeDir`：变体保持不变。
- `FLike`：变体保持不变；clone/read/write/ioctl/mmap/poll 实现整理。
- `EvCb` -> `EventCallback`：仅命名展开。

## 公共 API 变化

### 改名

- `EvFlag` -> `EventFlag`
- `EvBus` -> `EventBus`
- `EvCb` -> `EventCallback`
- `FlgGuard` -> `FlagGuard`
- `wait_ev` -> `wait_event`
- `EvBus::cb_len` -> `EventBus::callback_count`

### 新增

- `RunQueue::set_current_with_policy(task_id, policy)`：允许保存当前任务的调度策略，供 `yield_current` 恢复。

### 删除

- `Kernel::lookup_path`：删除单独路径 lookup 包装；挂载解析仍由 `MountTable::resolve` 提供。
- `audit_fd_table`：删除未接入主流程的 fd 审计辅助函数。
- `rehash_mount_cache`：删除临时 mount cache 重建辅助函数。
- `read_as_vec`：删除简单 `to_vec` 包装。

### 类型签名变化

- `SigSet::coalesce_pending(&mut self)` -> `SigSet::coalesce_pending(&self)`：函数只读取 pending/blocked，不再要求可变借用。
- `EpInst::control(&mut self, ...)` -> `EpInst::control(&self, ...)`：内部结构已有锁，调用方不再需要独占 `EpInst`。

### 参数名可读化但类型不变

这些函数的公开类型保持不变，主要是把短参数名展开：`BlockCache::{new,idx,fetch,sync_all,invalidate}`，`Channel::{new,send}`，`CircBuf::{new,with_pos}`，`Context::{capture,clone_with_ret,reg_class,set_ip,set_sp,set_ret,set_tls,transform}`，`Disk::{new,failing,attach_journal,set_errs,read_block,read_block_n,write_block}`，`EpEvent::has`，`EpEventMap::insert`，`FHandle::{with_data,read,read_at,write,write_at,transfer,set_len,mmap,advise_readahead,fallocate}`，`FLike::{read,write,io_ctl,mmap_fl}`，`FramePool::{new,get_contig,put,avail,put_zone_aware}`，`IoQueue::submit`，`KObjRegistry::{register_child,unregister,find_by_type,ref_up,ref_down,owner_objects}`，`Kernel::{new,tick,set_cur,handle_pgfault_ext,tty_push}`，`MountTable::{bind,unmount,has_prefix}`，`PgFrame::{with_rc,set}`，`PipeNode::{read_at,write_at}`，`PseudoNode::{new,read_at,write_at}`，`RunQueue::set_current`，`SemArr::set_ds`，`SemCtx::{add,add_undo}`，`Sema::{new,set_pid,set_val}`，`SharedPage::{new,fault}`，`ShmCtx::add`，`ShmTag::set_addr`，`SyncQueue::{park_on,signal_n,wait_ev,wait_events,wait_guard,wait_timeout}`，`Task::{link_parent,link_child,get_free_fd_from,add_file,get_futex,set_ep,end_run,set_cloexec}`，`TaskTable::{find,reap,fork_task,clone_thread,terminate_and_collect}`，`TimerEntry::new`，`TimerWheel::cancel`，`TrapCtl::{configure,on_pgfault}`，`VmRegion::with_offset`，`WaitQueue::wake_filtered`，以及 free functions `p2v`/`v2p`/`k_off`/`tcp_checksum`/`parse_ipv4_header`/`build_pseudo_header`/`frame_dealloc`/`frame_alloc_contig`/`check_access_rw`/`ctu`/`heap_init`/`heap_grow`/`compute_rss_watermark`/`ser`。

## 公共函数归属清单

下列清单覆盖 `kernel-rewrite/kernel.rs` 中保留的公共函数。除上面列出的 API 级变化外，同名函数主要是实现整理或边界修正。

- `VmRegion`：`new`、`with_offset`、`end`、`contains`、`overlaps`、`split_at`、`merge_with`、`ref_up`、`ref_down`、`ref_get`。
- `CapSet`：`new`、`full`、`check`、`grant`、`drop_cap`、`inherit`、`has_any`、`clear_ambient`、`raise_ambient`。
- `SigSet`：`new`、`sig_pending`、`sig_raise`、`coalesce_pending`、`sig_clear`、`sig_block`、`sig_unblock`、`sig_setmask`、`deliverable`、`set_action`、`get_action`、`is_ignored`、`clear_non_caught`。
- `TimerEntry`：`new`、`expired`、`reset`、`remaining`、`cancel`。
- `TimerWheel`：`new`、`add_timer`、`advance`、`cancel`、`active_count`。
- `KernLock`：`new`、`enter`、`leave`、`held`、`owner`、`level`、`try_enter`。
- `ZoneInfo`：`new`、`zone_can_alloc`、`zone_pressure`、`reclaim_target`、`contains_pfn`。
- `CircBuf`：`new`、`with_pos`、`push`、`pop`、`len`、`empty`、`full`、`peek`、`drain_to`、`fill_from`、`remaining`。
- `Spin`：`new`、`acquire`、`try_acquire`、`release`、`is_held`。
- `FlagGuard`：`enter`。
- `EventBus`：`make`、`set`、`clear`、`change`、`sub`、`callback_count`。
- `SyncQueue`：`new`、`park_on`、`signal`、`broadcast`、`signal_n`、`pending`、`wait_ev`、`wait_events`、`wait_guard`、`wait_timeout`、`reg_epoll`、`unreg_epoll`。
- `Sema`：`new`、`remove`、`release`、`try_acquire`、`acquire_spin`、`access`、`get_val`、`get_ncnt`、`get_pid`、`set_pid`、`set_val`。
- `FutexBucket`：`new`、`wait`、`wake`、`requeue`、`pending_at`。
- `FutexTable`：`new`、`ftx_wait`、`ftx_wake`、`ftx_requeue`。
- `PgFrame`：`new`、`with_rc`、`up`、`down`、`count`、`set`、`cas`、`inc_if_nonzero`。
- `VmMap`：`new`、`insert`、`find`、`remove_range`、`find_free`、`total_mapped`、`clone_regions`、`gap_after`。
- `FramePool`：`new`、`get`、`get_inner`、`get_contig`、`put`、`avail`、`free_count`、`get_zone_aware`、`put_zone_aware`、`batch_alloc`。
- `SharedPage`：`new`、`fault`、`is_cow_resolved`、`frame_id`。
- `KStk`：`new`、`top`。
- `SlabEntry`：`new`、`slab_alloc`、`slab_free`、`slab_used`、`slab_avail`、`shrink`、`obj_at`、`obj_at_mut`。
- `FHandle`：`new`、`with_data`、`dup`、`set_opt`、`get_opt`、`read`、`read_at`、`write`、`write_at`、`seek`、`transfer`、`set_len`、`sync_all`、`sync_data`、`metadata_sz`、`lookup`、`read_entry`、`poll_status`、`io_ctl`、`mmap`、`inode_ref`、`advise_readahead`、`fallocate`、`splice_to`。
- `PipeNode`：`pair`、`can_read`、`can_write`、`read_at`、`write_at`、`poll`。
- `FLike`：`dup`、`read`、`write`、`io_ctl`、`mmap_fl`、`poll`。
- `PseudoNode`：`new`、`read_at`、`write_at`、`metadata_sz`。
- `EpEvent`：`has`。
- `EpEventMap`：`new`、`insert`、`contains_key`、`remove`。
- `EpInst`：`new`、`control`。
- `Channel`：`new`、`recv`、`send`、`close`、`try_recv`、`send_batch`、`depth`、`drain_all`、`is_closed`、`remaining_capacity`。
- `PageCache`：`new`、`lookup`、`insert`、`evict_lru`、`mark_dirty`、`writeback_all`、`stats`、`pin`、`unpin`、`invalidate`、`flush_range`。
- `KObjRegistry`：`new`、`register`、`register_child`、`unregister`、`find_by_type`、`dump_graph`、`gc_sweep`、`ref_up`、`ref_down`、`count`、`owner_objects`。
- `CacheChain`：`new`。
- `BlockCache`：`new`、`idx`、`fetch`、`sync_all`、`invalidate`、`total_entries`、`dirty_count`、`evict_cold`。
- `MountTable`：`new`、`bind`、`resolve`、`unmount`、`list_mounts`、`find_mount`、`mount_count`、`has_prefix`。
- `IoQueue`：`new`、`submit`、`submit_batch`、`dispatch`、`merge_adjacent`、`depth`。
- `Disk`：`new`、`failing`、`attach_journal`、`set_errs`、`read_block`、`read_block_n`、`total_ops`、`reset_ops`、`write_block`、`flush`。
- `SemArr`：`remove`、`otime_now`、`ctime_now`、`set_ds`、`get_or_create`。
- `SemCtx`：`add`、`remove`、`get`、`add_undo`。
- `ShmTag`：`set_addr`。
- `ShmCtx`：`add`、`get`、`set`、`get_id_by_addr`、`pop`。
- `ProcInit`：`push_at`、`total_size`。
- `Context`：`new`、`capture`、`apply`、`set_ip`、`set_sp`、`set_ret`、`set_tls`、`transform`、`syscall_args`、`clone_with_ret`、`diff`、`hash`、`reg_class`。
- `TrapCtl`：`new`、`configure`、`hw`、`sw`、`in_handler`、`dispatch`、`current`、`handle_irq`、`on_pgfault`、`dispatch_vector`、`push_frame`、`pop_frame`、`nest_depth`、`suppress`、`unsuppress`。
- `SchedulePolicy`：`new`、`with_prio`、`weight`。
- `RunQueue`：`new`、`enqueue`、`dequeue`、`pick_next`、`rebalance`、`set_current`、`set_current_with_policy`、`clear_current`、`len`、`remove`、`update_vruntime`、`preempt_disable`、`preempt_enable`、`preemptible`、`boost_priority`、`yield_current`。
- `Pid`：`new`、`get`、`is_init`。
- `Task`：`make`、`id`、`tag`、`link_parent`、`link_child`、`done`、`n_children`、`get_free_fd`、`get_free_fd_from`、`add_file`、`get_file`、`get_futex`、`exit_proc`、`exited`、`get_ep_mut`、`get_ep_ref`、`set_ep`、`begin_run`、`end_run`、`has_sig`、`send_sig`、`close_fd`、`dup_fd`、`dup2_fd`、`fd_count`、`set_cloexec`。
- `TaskTable`：`new`、`spawn`、`spawn_root`、`find`、`find_by_tag`、`process_of_tid`、`pgid_group`、`register`、`reap`、`count`、`fork_task`、`clone_thread`、`new_user_task`、`terminate_and_collect`、`active_tasks`、`zombie_tasks`、`send_signal_group`。
- `Kernel`：`new`、`tick`、`cur_task`、`set_cur`、`handle_pgfault`、`handle_pgfault_ext`、`proc_init`、`tty_push`、`tty_pop`、`get_sem`、`get_shm`、`spawn_thread`、`dispatch_syscall`、`schedule_tick`、`balance_load`、`reclaim_zombies`、`alloc_pages`、`free_pages`、`memory_pressure`、`cache_stats`、`do_fork`、`do_exec`、`do_pipe`、`do_wait`。
- `AddrSpace`：`new`、`fork_from`、`handle_cow_fault`、`unmap_range`、`protect`、`rss_pages`、`cow_sharers`、`split_region`。
- `ProcessGroup`：`new`、`add_member`、`remove_member`、`is_empty`、`member_count`、`is_leader`、`set_foreground`、`is_foreground`、`broadcast_signal`。
- `WaitQueue`：`new`、`sleep`、`sleep_timeout`、`wake_one`、`wake_all`、`wake_filtered`、`pending_count`、`total_wakes`、`has_waiters_for`、`reorder_by_priority`。
- `ResourceLimits`：`default_limits`、`check_fd`、`check_threads`、`check_stack`、`check_data`、`check_filesize`、`check_mappings`、`inherit`、`set_limit`、`get_limit`、`exceeds_any`。
- `BuddyAllocator`：`new`、`alloc_order`、`free_order`、`free_pages_count`、`largest_free_order`、`fragmentation_score`、`snapshot`。
- 独立公共函数：`wait_event`、`p2v`、`v2p`、`k_off`、`tcp_checksum`、`parse_ipv4_header`、`build_pseudo_header`、`compute_inet_checksum`、`frame_alloc`、`frame_dealloc`、`frame_alloc_contig`、`check_access`、`check_access_rw`、`cfu`、`ctu`、`rdu_fixup`、`heap_init`、`heap_grow`、`validate_elf_header`、`compute_load_balance`、`defragment_frame_pool`、`verify_page_alignment`、`compute_rss_watermark`、`shm_get_or_create`、`wclk`、`cclk`、`dtk`、`up_ms`、`tmr`、`ser`、`yield_now_sync`、`validate_access`、`mem_scan_pattern`、`compute_crc32`、`encode_varint`、`decode_varint`、`bitwise_merge`、`rotate_bits`、`popcount64`、`clz64`、`ffs64`、`align_up`、`align_down`、`is_power_of_two`、`log2_floor`、`hash_combine`、`murmurhash3_finalize`。

## 主要实现变更摘要

- 同步相关：`EventBus`/`SyncQueue`/`Sema`/`Futex*` 重点修正提前唤醒、陈旧 waiter、事件位同步和锁使用。
- 内存相关：`VmRegion`/`VmMap`/`FramePool`/`PgFrame`/`AddrSpace`/`BuddyAllocator` 重点修正溢出、引用计数下溢、对齐和区间边界。
- 文件和 I/O：`FHandle`/`PipeNode`/`EpInst`/`PageCache`/`BlockCache`/`MountTable`/`IoQueue`/`Disk` 重点修正 offset、append、epoll 状态清理、cache 容量、mount prefix 和块范围溢出。
- 进程调度：`RunQueue` 改为按 vruntime 维护队列，当前任务保存 policy；`Task`/`TaskTable` 减少重复加锁并修正 fd、signal、reap 行为。
- trap/syscall：`Context`/`TrapCtl` 修正寄存器恢复和 trap 状态；`Kernel::dispatch_syscall` 中 read/write/open/dup/kill 等路径删掉大量无效代码并改为调用已有对象方法。
