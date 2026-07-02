# kernel-rewrite 文件结构索引

## crate root

- `src/lib.rs`
- `src/consts.rs`
- `src/kernel.rs`: `Kernel`
- `src/syscall/mod.rs`
- `src/trap.rs`: `Context`, `TrapCtl`, `wclk`, `cclk`, `dtk`, `up_ms`, `tmr`, `ser`
- `src/net.rs`: `SocketState`, `tcp_checksum`, `parse_ipv4_header`, `build_pseudo_header`, `compute_inet_checksum`
- `src/util.rs`: `validate_access`, `mem_scan_pattern`, `compute_crc32`, `encode_varint`, `decode_varint`, `bitwise_merge`, `rotate_bits`, `popcount64`, `clz64`, `ffs64`, `align_up`, `align_down`, `is_power_of_two`, `log2_floor`, `hash_combine`, `murmurhash3_finalize`

## memory

- `src/memory/mod.rs`: `SlabEntry`, `SharedPage`, `KStk`, `BuddyAllocator`
  - helpers: `empty`, `aligned_object_size`, `slot_range`, `is_valid_slot_offset`, `is_free_slot`, `p2v`, `v2p`, `k_off`, `heap_init`, `heap_grow`, `validate_elf_header`, `compute_load_balance`, `range_is_free`
- `src/memory/frame.rs`: `ZoneInfo`, `PgFrame`, `FramePool`
  - helpers: `frame_alloc`, `frame_dealloc`, `frame_alloc_contig`, `defragment_frame_pool`, `verify_page_alignment`
- `src/memory/vm.rs`: `VmRegion`, `VmMap`, `AddrSpace`
  - helpers: `check_access`, `check_access_rw`, `cfu`, `ctu`, `rdu_fixup`, `compute_rss_watermark`

## sync

- `src/sync/mod.rs`
- `src/sync/lock.rs`: `KernLock`, `Spin`, `FlagGuard`
- `src/sync/event_bus.rs`: `EventFlag`, `EventCallback`, `EventBus`, `wait_event`
- `src/sync/condvar.rs`: `RegEp`, `SyncQueue`, `consume_signal`, `record_signal`, `remove_waiter_by_id`, `remove_waiter_from_all`
- `src/sync/semaphore.rs`:  `SemaInner`, `Sema`, `SemaGuard`

## process

- `src/process/mod.rs`
- `src/process/abi.rs`: `ProcInit`
- `src/process/futex.rs`: `FutexBucket`, `FutexTable`, `remove_waiter`
- `src/process/structs.rs`: `CapSet`, `SchedulePolicy`, `RunQueue`, `WaitQueue`, `ResourceLimits`
- `src/process/task.rs`: `Tid`, `Pgid`, `Pid`, `TaskInfo`, `ThdCtx`, `Task`, `TaskTable`, `ProcessGroup`, `yield_now_sync`

## fs

- `src/fs/mod.rs`
- `src/fs/file.rs`: `FdOpt`, `FdState`, `FHandle`, `FSeek`,`create`
- `src/fs/file_like.rs`: `FLike`
- `src/fs/pipe.rs`: `PipeDir`, `PipeBuf`, `PipeNode`
- `src/fs/epoll.rs`: `EpData`, `EpEvent`, `EpCtlOp`, `EpEventMap`, `EpInst`
- `src/fs/pseudo.rs`: `PseudoNode`
- `src/fs/termios.rs`: `TrmIO`, `WinSz`
- `src/fs/channel.rs`: `CircBuf`, `Channel`
- `src/fs/cache.rs`:  `PageCacheEntry`, `PageCache`, `KObjEntry`, `KObjRegistry`, `CacheSlot`, `CacheChain`, `BlockCache`, `remove_from_type_index`, `chain_index`
- `src/fs/mount.rs`: `MountEntry`, `MountTable`, `prefix_matches`, `canonicalize_slashes`
- `src/fs/disk.rs`: `IoRequest`, `IoQueue`, `Disk`,`consume_transient_error`

## ipc

- `src/ipc/mod.rs`
- `src/ipc/semary.rs`: IpcPerm`, `SemDs`, `SemArr`, `SemId`, `SemNum`, `SemOp`, `SemCtx`, `index`, `free_id`
- `src/ipc/shared_mem.rs`; `ShmId`, `ShmTag`, `ShmCtx`, `shm_get_or_create`

## signal

- `src/signal/mod.rs`
- `src/signal/action.rs`: `SigAction`, `SigSet`
- `src/signal/timer.rs`: `TimerEntry`, `TimerWheel`
