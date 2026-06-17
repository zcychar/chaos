# Kernel Pre-Fix Notes

This document is the required pre-fix record for audit bugs found in `kernel/src/kernel.rs`.
It lists only code parts with located problems; audited parts with no found bug are excluded.
Line numbers match the current audit pass and may move after fixes.

Fixing principle: preserve the intended OS-simulation behavior and repair the violated invariant directly. Do not make a test pass by weakening validation, dropping state, or breaking valid existing use cases.

Pre-fix compile evidence:

- `cd chaos-tests && cargo test --no-run --test basic --test audit_sync --test audit_memory --test audit_helpers --test audit_fileio --test audit_channel --test audit_cache_disk --test audit_ipc --test audit_signal_timer --test audit_context_trap --test audit_scheduler --test audit_syscalls --test audit_utils --test audit_resources --test audit_buddy`: passes on 2026-05-16.

## Sync, Event, Futex

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `SyncQueue::wait_events`, `kernel/src/kernel.rs:457` | Registers the current thread in every queue and never removes stale registrations from queues that did not wake it. | Track the current thread id and remove it from all registered queues after wake/return while keeping the predicate recheck loop. |
| `SyncQueue::wait_timeout`, `kernel/src/kernel.rs:475` | Pushes the current thread, parks with timeout, returns `true`, and leaves timed-out waiters in `q`. | Remove only the current thread on return. Return wake status from whether cleanup found the waiter, but keep queue cleanup as the core invariant. |
| `FutexTable::ftx_wake`, `kernel/src/kernel.rs:621` | Uses `wk <= limit`, increments before removal, and for `count == 1` reports a wake without removing or unparking the waiter. | Return `0` for `count == 0`; remove and unpark at most `count` matching waiters; return exactly the number actually woken. |

## Memory, VM, Frames

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `PgFrame::down`, `kernel/src/kernel.rs:696` | Unconditional `fetch_sub(1)` wraps a zero refcount to `usize::MAX`. | Use a compare-exchange loop or saturating path so zero stays zero and the returned previous count remains meaningful. |
| `VmRegion::overlaps`, `kernel/src/kernel.rs:738` | End calculation wraps and the no-overlap check uses `b_end < self.base`, making adjacency asymmetric. | Use checked end addresses and half-open interval logic: no overlap when either end is `<=` the other base. |
| `VmMap::insert`, `kernel/src/kernel.rs:792` | Computes the region end with wrapping arithmetic and accepts ranges crossing `KERN_BASE`. | Reject zero/overflowing ranges and ranges whose exclusive end is above `KERN_BASE` before overlap insertion. |
| `FramePool::get_contig`, `kernel/src/kernel.rs:1012` | Computes `1usize << align_log2` before validating the shift width. | Reject invalid alignment orders before shifting; then scan with a nonzero step. |
| `SharedPage::fault`, `kernel/src/kernel.rs:1178` | Directly decrements the source frame refcount with `fetch_sub(1)`, so a zero source underflows. | Route through the fixed `PgFrame::down` or apply the same saturating decrement at the CoW source update. |

## Heap, Slab, Helpers

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `heap_init`, `kernel/src/kernel.rs:1269` | Aligns `base` and computes `end` with unchecked addition near `usize::MAX`. | Use checked alignment and checked end calculation; return a conservative non-panicking value on overflow. |
| `heap_grow`, `kernel/src/kernel.rs:1279` | Computes `n * 2` for `max_attempts`, which overflows for huge requests. | Use `saturating_mul` or bound attempts by available frames so oversized requests stop cleanly. |
| `SlabEntry::slab_alloc`, `kernel/src/kernel.rs:1403` | The zeroing branch runs when `zeroed` is false, so reused zeroed allocations retain old bytes. | Zero the slot when `zeroed == true`; leave non-zeroed allocations unchanged. |
| `SlabEntry::slab_free`, `kernel/src/kernel.rs:1421` | Detects duplicate frees but ignores the result and pushes the same slot again. | Reject duplicate frees before pushing to `free_list`; decrement `allocated` only for a real free. |
| `validate_elf_header`, `kernel/src/kernel.rs:1487` | Program-header table end uses unchecked addition/multiplication. | Use checked multiplication and addition for `phoff + entsize * phnum`; malformed input returns `Err`. |
| `verify_page_alignment`, `kernel/src/kernel.rs:1619` | Computes `PAGE_SZ << order` before validating `order`. | Validate order first; use checked shift/addition for alignment span checks. |
| `compute_rss_watermark`, `kernel/src/kernel.rs:1636` | Rounds huge region lengths with unchecked `r.len + PAGE_SZ - 1`. | Use checked or saturating page rounding and saturating weight accumulation. |

## File, Pipe, Epoll

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `FHandle::write`, `kernel/src/kernel.rs:1743` | Append mode writes at EOF but advances the old descriptor offset by length. | After `write_at`, set descriptor offset to `append_start + len`; preserve normal write offset behavior. |
| `FHandle::write_at`, `kernel/src/kernel.rs:1749` | Uses `off + buf.len()` for resize and slice bounds, which can overflow. | Compute end with `checked_add`; return `Err("einval")` or equivalent for impossible ranges. |
| `FHandle::seek`, `kernel/src/kernel.rs:1755` | Negative seek results are cast to `u64`, wrapping below zero into huge offsets. | Calculate in signed space and reject negative results before storing `off`. |
| `PipeNode` clone/drop, `kernel/src/kernel.rs:1857` | Derived clone does not increment `PipeBuf::ends`, but every drop decrements it. | Implement manual `Clone` that increments endpoint count, and only set close events when an endpoint actually closes. |
| `PipeNode::write_at`, `kernel/src/kernel.rs:1893` | Writes succeed after the read endpoint is dropped, buffering unreachable data. | Check writer direction and live reader count; return an error when no reader remains. |
| `FLike::dup` for `EpInst`, `kernel/src/kernel.rs:1931` | Duplicates share `ready` and `new_ctl` but copy `events`, so registrations diverge. | Make epoll registration state shared as one instance, or update `EpInst` so duplicated handles share the events map too. |
| `FLike::mmap_fl`, `kernel/src/kernel.rs:2047` | Page-count rounding for a huge range can overflow. | Use checked `end - start` and checked/saturating page rounding before calling file mmap. |
| `EpInst::control` add, `kernel/src/kernel.rs:2159` | `EPOLL_CTL_ADD` overwrites an existing fd instead of rejecting it. | Return an error when the fd is already registered. |
| `EpInst::control` delete, `kernel/src/kernel.rs:2173` | Delete removes `events` only, leaving stale `ready` and `new_ctl` state. | Remove the fd from all epoll state sets on successful delete. |

## Terminal And Channel

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `Channel::send`, `kernel/src/kernel.rs:2287` | Closed channels still accept a byte and report success. | Check `shut` before mutating the ring; failed sends must leave depth unchanged. |
| `Channel::send_batch`, `kernel/src/kernel.rs:2333` | Closed channels still accept batch data. | Check `shut` before and during the locked write path; return zero after close. |
| `Channel::send_batch`, `kernel/src/kernel.rs:2348` | A batch wake only unparks one receiver even when multiple bytes become readable. | Wake up to the number of bytes written or pending waiters so blocked receivers can consume the batch. |

## Cache, Mount, Disk

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `PageCache::insert`, `kernel/src/kernel.rs:2435` | A zero-capacity cache still stores entries. | Treat capacity as a hard upper bound; no-op when capacity is zero. |
| `PageCache::insert`, `kernel/src/kernel.rs:2435` | If all current entries are pinned, failed eviction is ignored and capacity is exceeded. | Insert only if space exists after eviction; if no page is evictable, keep existing pages. |
| `BlockCache::fetch`, `kernel/src/kernel.rs:2691` | `BlockCache::new(0)` creates a zero-width cache and `fetch` divides by zero. | Make zero-width fetch return `None`, or normalize width at construction without panicking. |
| `BlockCache::invalidate`, `kernel/src/kernel.rs:2771` | Uses `k % width` while `fetch` uses `(k ^ (k >> 7)) % width`, so invalidation can miss entries. | Centralize the hash-chain index and use it in fetch, invalidate, and related operations. |
| `BlockCache::sync_all`, `kernel/src/kernel.rs:2765` | Re-entering an already-held `GKL` increments depth, then unconditionally clears owner/depth/flag. | Save previous lock state and restore it; only fully release when this method acquired the lock. |
| `MountTable::resolve` and `find_mount`, `kernel/src/kernel.rs:2874`, `kernel/src/kernel.rs:2948` | Raw prefix byte matching treats `/mnt` as matching `/mnted/file`. | Require a component boundary after the prefix unless the prefix is `/`. |
| `IoQueue::submit_batch`, `kernel/src/kernel.rs:3019` | Calls `merge_adjacent()` while still holding `pending`, causing self-deadlock. | Release the queue lock before merging, or refactor merge to work on the held queue without relocking. |
| `IoQueue::merge_adjacent`, `kernel/src/kernel.rs:3062` | Compares `q[i].block + 1` with unchecked addition. | Use `checked_add(1)` before comparing adjacent blocks. |
| `Disk::read_block_n`, `kernel/src/kernel.rs:3128` | Successful limited reads fill `0xAA ^ index`, unlike `read_block`'s `0xAA` pattern. | Use the same simulated success pattern for both read paths. |

## IPC, Semaphores, Shared Memory

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `SemArr::get_or_create`, `kernel/src/kernel.rs:3204` | Allows creation of zero-semaphore arrays. | Reject `nsems == 0` for new arrays. |
| `SemArr::get_or_create`, `kernel/src/kernel.rs:3215` | Existing-key lookup ignores a larger requested semaphore count. | Validate that an existing array satisfies requested `nsems`; otherwise return an error. |
| `SemCtx::remove`, `kernel/src/kernel.rs:3252` | Removes the array id but leaves undo records for that id. | Drop all undo entries whose semaphore id matches the removed array. |
| `SemCtx::drop`, `kernel/src/kernel.rs:3269` | Undo replay only handles `op == 1`. | Replay the full positive undo magnitude, and avoid applying stale records after remove. |
| `shm_get_or_create`, `kernel/src/kernel.rs:3295` | Key `0` is stored and reused, so private shared-memory calls return the same segment. | Treat key `0` as private: create a unique segment without reusing the map entry. |
| `shm_get_or_create`, `kernel/src/kernel.rs:3296` | Existing-key lookup ignores larger requested size and returns an undersized segment. | Ensure existing segment length is at least `npages`; grow or reject instead of returning undersized state. |

## Process Init, Capabilities, Signals, Timers

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `ProcInit::push_at`, `kernel/src/kernel.rs:3333` | Subtracts stack layout sizes from small `top` without checking underflow. | Use checked subtraction throughout; return `0` when the layout cannot fit. |
| `CapSet::inherit`, `kernel/src/kernel.rs:3399` | Uses `pb & !INHERITABLE_MASK`, dropping inheritable capabilities and keeping others. | Keep only bits allowed by `INHERITABLE_MASK` for child permitted/effective sets. |
| `CapSet::inherit`, `kernel/src/kernel.rs:3407` | Copies ambient capabilities even when corresponding permitted bits were dropped. | Mask ambient with the inherited permitted bits. |
| `SigSet::sig_pending`, `kernel/src/kernel.rs:3439` | Shifts by the raw signal number, so `64` panics. | Return false for signal 0 and out-of-range signals before shifting. |
| `SigSet::sig_raise`, `kernel/src/kernel.rs:3443` | Allows signal `0` into the pending mask even though it is not deliverable. | Queue only real signals in `1..NSIG`. |
| `SigSet::set_action`, `kernel/src/kernel.rs:3490` | Allows installing an action for signal `0`. | Accept only real catchable signals in `1..NSIG`, excluding `SIGKILL` and `SIGSTOP`. |
| `TimerEntry::expired`, `kernel/src/kernel.rs:3527` | Uses `now > deadline`, so a timer at its deadline is not expired. | Use `now >= deadline`. |
| `TimerEntry::reset`, `kernel/src/kernel.rs:3532` | Computes `now + interval` unchecked. | Use checked or saturating addition for repeated timer deadlines. |

## Context, Trap, Clock

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `TrapCtl::handle_irq`, `kernel/src/kernel.rs:3848` | Saves previous `irq_on` but leaves IRQ state as `true`. | Restore `was_irq_on` before return. |
| `TrapCtl::handle_irq`, `kernel/src/kernel.rs:3868` | Saves previous `active` but always stores `false` on exit. | Restore `was_active` before return. |
| `TrapCtl::dispatch_vector`, `kernel/src/kernel.rs:3880` | `8..=15` branch catches vector `14` before the explicit page-fault branch. | Match vector `14` before the software interrupt range or split the range around it. |
| `up_ms`, `kernel/src/kernel.rs:3939` | Multiplies `wclk() * USEC_TICK` unchecked. | Use saturating or checked multiplication before division. |

## Scheduler And Tasks

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `SchedulePolicy::with_prio`, `kernel/src/kernel.rs:3958` | Casts negative priority to `usize` and subtracts from 20. | Clamp/validate priority in signed space and derive a non-panicking time slice. |
| `RunQueue::enqueue`, `kernel/src/kernel.rs:3990` | Detects duplicates but still pushes the task. | Return early if the task id is already queued. |
| `RunQueue::update_vruntime`, `kernel/src/kernel.rs:4096` | Computes `delta * 1024` unchecked. | Use saturating multiplication/addition or checked fallback. |
| `RunQueue::preempt_enable`, `kernel/src/kernel.rs:4108` | `fetch_sub(1)` at zero wraps to `usize::MAX`. | Use compare-exchange or a load check so zero remains zero. |
| `Task::send_sig`, `kernel/src/kernel.rs:4359` | Detects duplicate standard signals but still pushes duplicates. | Coalesce duplicate standard signals; preserve queueing behavior for nonstandard or distinct sender cases as intended by tests. |
| `Task::set_cloexec`, `kernel/src/kernel.rs:4419` | Validates fd existence but never updates stored file state. | Mutate the stored `FLike::File` entry's `cloexec` field on success. |
| `TaskTable::reap`, `kernel/src/kernel.rs:4477` | Removes the task from the global map but not from its parent's `subtasks`. | Remove the reaped child from its parent's child list during reap. |
| `TaskTable::fork_task`, `kernel/src/kernel.rs:4530`, `kernel/src/kernel.rs:4534` | Pushes the child into the parent's `subtasks` list twice. | Link the child exactly once while preserving parent pointer, file duplication, pid registration, and thread initialization. |

## Syscall Facade

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `SYS_WRITE`, `kernel/src/kernel.rs:4796` | Cross-page length calculation adds `page_off`, so return value can exceed requested count. | Return the requested count or actual transferred count, never include address offset bytes. |
| `SYS_CLOSE`, `kernel/src/kernel.rs:4886` | Returns success for non-stdio fds without removing the fd from the current task. | Call the current task's `close_fd` after cache cleanup; preserve stdio handling if intended. |
| `SYS_MMAP`, `kernel/src/kernel.rs:4931` | Aligning huge lengths can overflow and later modulus can panic. | Checked-align length and reject impossible mappings before address selection. |
| `SYS_MUNMAP`, `kernel/src/kernel.rs:4957` | Allows `len == 0` to succeed. | Return `Err("einval")` for zero-length unmap before alignment work. |
| `SYS_DUP`, `kernel/src/kernel.rs:5048` | Finds a free fd number but does not install a duplicated file. | Delegate to `Task::dup_fd` for current tasks; return its installed fd. |
| `SYS_FORK`, `kernel/src/kernel.rs:5091` | Reserves a pid from `seq` but does not create/register a child task. | Delegate to `TaskTable::fork_task` for the current task and return the real child id. |
| `SYS_KILL`, `kernel/src/kernel.rs:5235` | Accepts `sig == NSIG` because validation uses `>`. | Reject `sig >= NSIG`; keep signal `0` as an existence probe. |
| `SYS_EPOLL_WAIT`, `kernel/src/kernel.rs:5439` | Multiplies `max_events * size_of::<EpEvent>()` before overflow validation. | Use `checked_mul` before access validation. |
| `SYS_SIGACTION`, `kernel/src/kernel.rs:5482` | Inverts catchability: ordinary catchable signals are rejected, uncatchable signals pass. | Reject only signal 0, out-of-range, `SIGKILL`, and `SIGSTOP`; accept ordinary signals. |
| `SYS_FUTEX`, `kernel/src/kernel.rs:5532` | Converts wake count zero into one wake. | Use `val` as the wake count directly and return zero for zero. |

## Access And Utilities

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `validate_access`, `kernel/src/kernel.rs:5813` | Rejects ranges whose exclusive end equals `KERN_BASE`, unlike `check_access`. | Allow `end == KERN_BASE` for half-open user ranges and keep overflow rejection. |
| `mem_scan_pattern`, `kernel/src/kernel.rs:5854` | Pushes a match before checking `max_matches`, so zero limit still returns one match. | Return early when `max_matches == 0`, or check the limit before pushing. |

## Address Space, Wait Queues, Resources

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `AddrSpace::fork_from`, `kernel/src/kernel.rs:5928`, `kernel/src/kernel.rs:5944` | Writable VM region refcounts are incremented in two loops. | Increment each shared writable region exactly once per fork. |
| `AddrSpace::unmap_range`, `kernel/src/kernel.rs:5975` | Computes `start + len` unchecked. | Use checked addition; on overflow remove nothing or clamp according to existing unmap semantics without panicking. |
| `AddrSpace::split_region`, `kernel/src/kernel.rs:6015` | Adds a second half while leaving the original full region, creating overlap. | Replace the original region with two non-overlapping halves from `VmRegion::split_at`. |
| `WaitQueue::sleep_timeout`, `kernel/src/kernel.rs:6112` | Removes its own timed-out waiter and interprets that removal as a wake. | Track the current thread entry and return false when timeout cleanup removes it. |
| `WaitQueue::sleep_timeout`, `kernel/src/kernel.rs:6119` | Timeout cleanup removes all waiters with the same key. | Remove only the current thread's waiter entry; preserve other waiters on the same key. |
| `ResourceLimits::exceeds_any`, `kernel/src/kernel.rs:6251` | Uses `>` while individual fd/thread/mapping checks reject equality. | Match individual boundary semantics: equality to count limits is exceeding for current-count resources. |

## Bit Utilities And Buddy Allocator

| Code part | Original problematic code | Fix thinking |
| --- | --- | --- |
| `align_up`, `kernel/src/kernel.rs:6300` | Computes `addr + align - 1` unchecked. | Use checked addition and return a non-panicking conservative value on overflow. |
| `BuddyAllocator::free_order`, `kernel/src/kernel.rs:6389` | Double-free inserts duplicate free blocks and underflows `allocated`. | Reject frees for blocks already present in free lists; use saturating or checked allocation accounting. |
| `BuddyAllocator::free_order`, `kernel/src/kernel.rs:6395` | Buddy address uses `current_addr ^ block_size`, valid only for zero-based allocators. | Compute buddy relative to `base_addr`: `base_addr + ((current_addr - base_addr) ^ block_size)`. |

## Required Second Revise After Fixes

After the approved fixes are applied and focused tests pass, fill `docs/kernel-second-recheck.md`. It should mirror `docs/kernel-map.md` and record:

- Each fixed subsystem and exact test command run.
- Any new or hidden problems introduced by the fixes.
- A second manual pass over sync, memory/VM, files/pipes/epoll, terminal/channel, cache/mount/disk, IPC, signals/timers, trap/context, scheduler/tasks, syscall facade, process groups/wait/resources, and allocator/utilities.
- Remaining risks or invariants that still need tests.
