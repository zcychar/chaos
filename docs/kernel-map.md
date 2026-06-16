# Kernel Map

This file tracks `kernel/src/kernel.rs` during the std-based test-simulation bug-fix pass. Keep upstream rCore as reference material only; do not convert this file to no-std or QEMU integration during the current phase.

## Visible Test Coverage

- `group_01`: address translation, global kernel lock, event flags.
- `group_02`: spin/semaphore behavior and sleeping while synchronized.
- `group_03`: `SyncQueue` condition-style behavior.
- `group_04`: frame pool and VM-map basics.
- `group_05`: task creation, exit, and run-queue ordering.
- `group_06`: file handles, pipes, and epoll basics.
- `group_07`: page cache, block cache, and mount lookup.
- `group_08`: capabilities, signals, and timers.
- `group_09`: trap/context helpers, time ticks, and serial translation.
- `group_10`: syscall facade and process fork path.
- `group_11`: integrated fork/exec, pipe IPC, and mmap/file workload.

## Line-Range Map

| Lines | Subsystem | Upstream Reference | Recheck Notes |
| --- | --- | --- | --- |
| 1-428 | Imports, constants, global lock, basic event/ring declarations | rCore syscall, memory, signal, scheduler constants | Confirm public constants match test expectations; visible `group_01` and compile gates cover lock/constants use. |
| 429-904 | Synchronization primitives, event bus, semaphores, futex table | rCore sync/futex concepts | Check lost wakeups, poisoning tolerance, timeout cleanup, and futex wake counts. |
| 916-1689 | Address helpers, frames, VM regions/maps, user copy helpers | rCore memory and page-table layers | Check overflow, alignment, permissions, and copy-on-write semantics. |
| 1692-2205 | Heap, circular buffers, slabs, ELF/network/checksum/scheduling helpers | rCore loader, allocator, net helpers | Check boundary values, checked arithmetic, zeroed allocation, and deterministic helper output. |
| 2208-2991 | File handles, pipes, file-like enum, epoll, terminal structs | rCore fs/syscall fs | Check offset sharing, fd options, pipe closure, readiness, event masks, and terminal metadata. |
| 2992-3214 | Channels | rCore tty/console and pipe concepts | Check closed-channel writes, receiver wakeups, buffer boundaries, and channel EOF. |
| 3216-4154 | Page cache, kernel object registry, block cache, mounts, I/O queue, disk | rCore block/fs/cache concepts | Check cache invalidation, mount resolution, I/O completion, and bounds checks. |
| 4158-4386 | IPC permissions, semaphores, shared memory contexts | rCore IPC/syscall ipc | Check permission logic, clone/drop ownership, sizing, and ID reuse. |
| 4387-4776 | Process init, capabilities, signal sets/actions, timers | rCore process/signal/time | Check signal masks, uncatchable signals, timer ordering, and inheritance. |
| 4777-5191 | Context, traps, clocks, serial helpers | rCore arch trap/context/time | Check register bounds, trap dispatch setup, IRQ restoration, and tick accounting. |
| 5194-6054 | Scheduler, run queues, tasks, task table | rCore task/scheduler | Check priorities, fork/exec inheritance, parent-child links, wait state, and PID reuse. |
| 6055-7528 | Kernel facade and syscall implementations | rCore syscall layer | Check errno-like returns, resource lifetime, fd/task lookup, and argument validation. |
| 7529-7659 | Access validation and utility encoders/checksums | rCore user access/utilities | Check integer overflow and empty-pattern edge cases. |
| 7662-8071 | Address spaces, process groups, wait queues, resource limits | rCore memory/process/wait/resource concepts | Check clone semantics, group/session invariants, wait filtering, and limit enforcement. |
| 8072-8338 | Bit utilities and buddy allocator | rCore allocator helpers | Check zero/overflow cases, alignment, coalescing, and free-list invariants. |

## Recheck Log

- Done: visible compile/runtime gate `cd chaos-tests && cargo test --test basic` passes with 33 tests.
- Done: group-by-group visible failures from `group_01` through `group_11` have been triaged and fixed in `docs/runtime-failures.md`.
- Done: module-by-module manual recheck after visible tests pass. Fourteen audit targets now cover the behavioral line ranges from synchronization through the buddy allocator, and the constants/imports range is checked by compile-time use across the visible and audit targets.
- Done: pre-fix notes for all located audit bugs are recorded in `docs/kernel-pre-fix-notes.md`; kernel code fixes remain gated by the required approval step.
- Done: pre-fix compile gate on 2026-05-16: `cd chaos-tests && cargo test --no-run --test basic --test audit_sync --test audit_memory --test audit_helpers --test audit_fileio --test audit_channel --test audit_cache_disk --test audit_ipc --test audit_signal_timer --test audit_context_trap --test audit_scheduler --test audit_syscalls --test audit_utils --test audit_resources --test audit_buddy`.
- Done: after approved kernel fixes, final second-pass review is recorded in `docs/kernel-second-recheck.md`.
- Done: 2026-05-16 fix batch 1: `audit_sync`, `audit_memory`, `audit_helpers`, and `basic -- --test-threads=1` pass after sync/futex, memory/VM/frame, and heap/slab/helper fixes.
- Done: 2026-05-16 fix batch 2: `audit_fileio`, `audit_channel`, `audit_cache_disk`, and `basic -- --test-threads=1` pass after file/pipe/epoll, terminal/channel, and cache/mount/disk fixes.
- Done: 2026-05-16 fix batch 3: `audit_ipc`, `audit_signal_timer`, `audit_context_trap`, `audit_scheduler`, and `basic -- --test-threads=1` pass after IPC, signal/timer, context/trap, and scheduler/task fixes.
- Done: 2026-05-16 fix batch 4: `audit_syscalls`, `audit_utils`, `audit_resources`, `audit_buddy`, and `basic -- --test-threads=1` pass after syscall facade, access/utilities, address-space/wait/resource, and buddy allocator fixes.
- Done: 2026-05-16 final verification gate: selected visible/audit targets compile with `cargo test --no-run --test basic --test audit_sync --test audit_memory --test audit_helpers --test audit_fileio --test audit_channel --test audit_cache_disk --test audit_ipc --test audit_signal_timer --test audit_context_trap --test audit_scheduler --test audit_syscalls --test audit_utils --test audit_resources --test audit_buddy`, and the same targets pass at runtime with `-- --test-threads=1`.

Note: audit finding line references below are historical pre-fix references. The current line-range map above reflects the post-`rustfmt` file layout.

## Manual Audit Strategy

Use this as the next-phase checklist. For each line range, record concrete invariants checked, tests added or run, and any bug report before code edits.

1. Read the module and write down its intended contracts.
   - State ownership and synchronization rules.
   - Boundary values: zero, one, full capacity, overflow, underflow, empty collections, invalid IDs, closed resources.
   - Lifetime rules: clone/drop/refcount, wake/sleep, fd/task/resource cleanup.
   - Error rules: which paths return `Err`, `None`, false, or negative syscall values.

2. Add focused tests only after the contract is clear.
   - Prefer new audit targets under `chaos-tests/tests/audit/` when a test is expected to fail until a newly found bug is approved and fixed.
   - Keep `chaos-tests/tests/basic/` as the visible baseline gate.
   - Keep each test tied to one invariant so failures identify the subsystem.
   - Include at least one negative/error case for every helper with validation logic.

3. Before fixing any located kernel bug, report:
   - file/line
   - failing symptom or new test
   - root cause
   - expected behavior
   - proposed minimal fix
   - wait for approval before editing kernel code

4. After each approved fix:
   - run the focused test or group
   - run `cd chaos-tests && cargo test --test basic`
   - update this file and `docs/runtime-failures.md`

## Audit Work Queue

| Order | Lines | Subsystem | Initial Test Targets |
| --- | --- | --- | --- |
| 1 | 429-904 | synchronization, event bus, semaphores, futex table | Lost wakeups, stale waiters, timeout cleanup, lock recursion/ownership, futex key wake filtering. |
| 2 | 916-1689 | address helpers, frames, VM maps, user copy | Address overflow, region split/remove boundaries, frame refcount underflow, CoW allocation exhaustion. |
| 3 | 1692-2205 | heap, circular buffers, slabs, parsers, networking, helper algorithms | Zero capacity, full/empty transitions, malformed ELF/IP inputs, checksum and scheduling boundary cases. |
| 4 | 2208-2991 | file handles, pipes, file-like enum, epoll, terminal structs | Shared offsets, append semantics, pipe EOF/closure, epoll callback retention/removal, invalid fd operations. |
| 5 | 2992-3214 | channels | Close-after-wait, send/recv ordering, stale waiters, closed-channel sends. |
| 6 | 3216-4154 | page cache, registry, block cache, mounts, I/O queue, disk | Dirty invalidation, registry dependency cleanup, mount precedence, queue depth/merge fairness, disk retry limits. |
| 7 | 4158-4386 | IPC, semaphores, shared memory | Permission checks, semaphore removal wakeups, ID reuse, clone/drop ownership. |
| 8 | 4387-4776 | process init, capabilities, signals, timers | Uncatchable signals, blocked/pending coalescing, capability inheritance, timer repeat/cancel. |
| 9 | 4777-5191 | context, traps, clocks, serial helpers | Register bounds, trap nesting, mask clear/set semantics, page fault address classes, tick accounting. |
| 10 | 5194-6054 | scheduler, run queues, tasks, task table | Priority order, duplicate enqueue, fork parent links, zombie/reap, PID reuse. |
| 11 | 6055-7528 | kernel facade and syscall implementations | Argument validation, fd/task lifetime, errno-like returns, syscall side effects and resource cleanup. |
| 12 | 7529-7659 | access validation and utilities | Overflow, empty pattern behavior, varint truncation, CRC/checksum known vectors. |
| 13 | 7662-8071 | address spaces, process groups, wait queues, resource limits | Clone semantics, session/group invariants, timeout waiter cleanup, limit boundary comparisons. |
| 14 | 8072-8338 | bit utilities and buddy allocator | Zero values, invalid alignments, allocation/free coalescing, fragmentation accounting. |

## Audit Findings

### 2026-05-15: Imports/Constants Slice

Command status:

- `cd chaos-tests && cargo test --no-run --test basic --test audit_sync --test audit_memory --test audit_helpers --test audit_fileio --test audit_channel --test audit_cache_disk --test audit_ipc --test audit_signal_timer --test audit_context_trap --test audit_scheduler --test audit_syscalls --test audit_utils --test audit_resources --test audit_buddy`: all present visible/audit targets compile.

Notes:

- `kernel/src/kernel.rs:1-167` contains imports, public constants, and syscall numbers.
- No direct runtime behavior lives in this range; constants are referenced by the visible and audit tests.
- Separate `cargo test --tests --no-run` is blocked by pre-existing manifest entries for missing `tests/advanced/main.rs` and `tests/pressure/main.rs`.

### 2026-05-14: Sync/Event/Futex Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_sync`: fails with 4 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/sync.rs::audit_futex_wake_one_unparks_waiter`
- `chaos-tests/tests/audit/sync.rs::audit_futex_wake_zero_wakes_none`
- `chaos-tests/tests/audit/sync.rs::audit_syncqueue_timeout_removes_waiter`
- `chaos-tests/tests/audit/sync.rs::audit_syncqueue_wait_events_removes_stale_waiters`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:621`, `FutexTable::ftx_wake`
   - `count == 0` can report one wake.
   - `count == 1` reports one wake without removing or unparking the waiter.
   - Expected invariant: return count equals actual removed/unparked matching waiters.

2. `kernel/src/kernel.rs:475`, `SyncQueue::wait_timeout`
   - Timeout leaves the current thread in `q`.
   - Expected invariant: timed-out waiters are removed before returning.

3. `kernel/src/kernel.rs:457`, `SyncQueue::wait_events`
   - When one queue wakes the thread, stale entries remain in the other queues.
   - Expected invariant: after return, the current thread is removed from every queue it registered with.

### 2026-05-14: Memory/VM/Frame Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_memory`: fails with 5 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/memory.rs::audit_pgframe_down_does_not_underflow`
- `chaos-tests/tests/audit/memory.rs::audit_vmregion_adjacent_regions_do_not_overlap_symmetrically`
- `chaos-tests/tests/audit/memory.rs::audit_vmmap_rejects_kernel_crossing_region`
- `chaos-tests/tests/audit/memory.rs::audit_framepool_get_contig_large_alignment_does_not_panic`
- `chaos-tests/tests/audit/memory.rs::audit_shared_page_fault_does_not_underflow_source_refcount`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:696`, `PgFrame::down`
   - Decrementing a zero refcount wraps to `usize::MAX`.
   - Expected invariant: refcounts never underflow.

2. `kernel/src/kernel.rs:738`, `VmRegion::overlaps`
   - Adjacent regions are treated asymmetrically; `right.overlaps(left)` is true when `left.end() == right.base`.
   - Expected invariant: adjacent regions do not overlap, regardless of argument order.

3. `kernel/src/kernel.rs:792`, `VmMap::insert`
   - A region crossing `KERN_BASE` is accepted.
   - Expected invariant: user VM regions do not overflow or cross into kernel space.

4. `kernel/src/kernel.rs:1012`, `FramePool::get_contig`
   - Large `align_log2` can panic with shift overflow.
   - Expected invariant: invalid alignments fail without panicking.

5. `kernel/src/kernel.rs:1178`, `SharedPage::fault`
   - Faulting against a zero-refcount source decrements it to `usize::MAX`.
   - Expected invariant: CoW source refcounts never underflow.

### 2026-05-14: Heap/Slab/Helper Slice

Command status:

- `cd chaos-tests && cargo test --test audit_helpers`: fails with 7 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/helpers.rs::audit_slab_zeroed_alloc_clears_reused_object`
- `chaos-tests/tests/audit/helpers.rs::audit_slab_double_free_does_not_duplicate_slot`
- `chaos-tests/tests/audit/helpers.rs::audit_heap_init_overflow_does_not_panic`
- `chaos-tests/tests/audit/helpers.rs::audit_heap_grow_huge_request_does_not_panic`
- `chaos-tests/tests/audit/helpers.rs::audit_validate_elf_header_rejects_overflowing_phdr_table`
- `chaos-tests/tests/audit/helpers.rs::audit_verify_page_alignment_large_order_does_not_panic`
- `chaos-tests/tests/audit/helpers.rs::audit_compute_rss_watermark_huge_region_does_not_panic`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:1403`, `SlabEntry::slab_alloc`
   - `zeroed == true` does not clear reused object memory because the zeroing branch runs only when `zeroed` is false.
   - Expected invariant: zeroed allocations return an all-zero object.

2. `kernel/src/kernel.rs:1421`, `SlabEntry::slab_free`
   - Double free detection is computed but ignored, so the same slot can be inserted into `free_list` multiple times and allocated twice.
   - Expected invariant: each free slot appears at most once.

3. `kernel/src/kernel.rs:1269`, `heap_init`
   - Aligning a near-`usize::MAX` base panics with arithmetic overflow.
   - Expected invariant: invalid or overflowing heap ranges fail or clamp without panicking.

4. `kernel/src/kernel.rs:1279`, `heap_grow`
   - `n * 2` panics for huge requests.
   - Expected invariant: oversized growth requests are bounded by available frames and never panic.

5. `kernel/src/kernel.rs:1487`, `validate_elf_header`
   - Program-header table bounds calculation can overflow before rejecting malformed ELF input.
   - Expected invariant: malformed ELF metadata returns `Err`, not a panic.

6. `kernel/src/kernel.rs:1619`, `verify_page_alignment`
   - `PAGE_SZ << order` is evaluated before `order` is validated, so large orders panic.
   - Expected invariant: invalid orders return false.

7. `kernel/src/kernel.rs:1636`, `compute_rss_watermark`
   - Page-count rounding can overflow for huge region lengths.
   - Expected invariant: invalid or huge VM region lengths do not panic while computing a watermark.

### 2026-05-14: File/Pipe/Epoll Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_fileio`: fails with 9 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/fileio.rs::audit_pipe_clone_drop_keeps_original_writer_open`
- `chaos-tests/tests/audit/fileio.rs::audit_pipe_write_after_reader_drop_errors`
- `chaos-tests/tests/audit/fileio.rs::audit_fhandle_append_write_updates_offset_to_new_end`
- `chaos-tests/tests/audit/fileio.rs::audit_fhandle_negative_seek_is_rejected`
- `chaos-tests/tests/audit/fileio.rs::audit_fhandle_write_at_overflow_does_not_panic`
- `chaos-tests/tests/audit/fileio.rs::audit_flike_mmap_overflow_does_not_panic`
- `chaos-tests/tests/audit/fileio.rs::audit_epoll_add_existing_fd_is_rejected`
- `chaos-tests/tests/audit/fileio.rs::audit_epoll_del_clears_ready_and_ctl_state`
- `chaos-tests/tests/audit/fileio.rs::audit_epoll_dup_shares_registration_state`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:1857`, `PipeNode::clone` / `PipeNode::drop`
   - Cloning a pipe endpoint does not increment `PipeBuf::ends`, but dropping the clone decrements it.
   - Expected invariant: dropping a duplicate endpoint must not close the original endpoint.

2. `kernel/src/kernel.rs:1893`, `PipeNode::write_at`
   - Writing after the read endpoint is closed still appends bytes and returns success.
   - Expected invariant: writes with no reader fail instead of silently buffering unreachable data.

3. `kernel/src/kernel.rs:1743`, `FHandle::write`
   - Append-mode writes choose the end of file as the write offset, then advance the old descriptor offset by the byte count.
   - Expected invariant: after an append write, the descriptor offset is the new file end.

4. `kernel/src/kernel.rs:1755`, `FHandle::seek`
   - Negative seeks before byte 0 wrap to a huge `u64`.
   - Expected invariant: seeks that would produce a negative offset return `Err("einval")`.

5. `kernel/src/kernel.rs:1749`, `FHandle::write_at`
   - `off + buf.len()` can overflow before resize bounds are checked.
   - Expected invariant: impossible write ranges return an error, not a panic.

6. `kernel/src/kernel.rs:2047`, `FLike::mmap_fl`
   - Page-count rounding can overflow for huge mapping ranges.
   - Expected invariant: invalid or overflowing ranges return an error.

7. `kernel/src/kernel.rs:2159`, `EpInst::control`
   - `EPOLL_CTL_ADD` overwrites an existing fd instead of rejecting it.
   - Expected invariant: adding an already-registered fd fails.

8. `kernel/src/kernel.rs:2173`, `EpInst::control`
   - `EPOLL_CTL_DEL` removes only `events`, leaving stale `ready` and `new_ctl` entries.
   - Expected invariant: deleting an fd clears all epoll state for that fd.

9. `kernel/src/kernel.rs:1931`, `FLike::dup` for `EpInst`
   - Duplicated epoll handles share `ready`/`new_ctl` but copy `events`, so registration changes diverge between duplicates.
   - Expected invariant: duplicated epoll fds share one coherent epoll instance state.

### 2026-05-14: Terminal/Channel Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_channel`: fails with 3 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/channel.rs::audit_channel_send_after_close_is_rejected`
- `chaos-tests/tests/audit/channel.rs::audit_channel_send_batch_after_close_is_rejected`
- `chaos-tests/tests/audit/channel.rs::audit_channel_send_batch_wakes_each_waiting_receiver`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:2287`, `Channel::send`
   - A closed channel still accepts a byte and reports success.
   - Expected invariant: sends after close fail and do not grow the buffer.

2. `kernel/src/kernel.rs:2333`, `Channel::send_batch`
   - A closed channel still accepts a batch and reports the number written.
   - Expected invariant: batched sends after close fail and do not grow the buffer.

3. `kernel/src/kernel.rs:2348`, `Channel::send_batch`
   - Batched sends wake only one waiting receiver even when multiple bytes become readable.
   - Expected invariant: receivers blocked before a batch should be woken up to consume the bytes made available by that batch.

### 2026-05-15: Cache/Mount/Disk Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_cache_disk`: fails with 9 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/cache_disk.rs::audit_page_cache_zero_capacity_stores_nothing`
- `chaos-tests/tests/audit/cache_disk.rs::audit_page_cache_does_not_exceed_capacity_when_all_entries_pinned`
- `chaos-tests/tests/audit/cache_disk.rs::audit_block_cache_zero_width_does_not_panic`
- `chaos-tests/tests/audit/cache_disk.rs::audit_block_cache_invalidate_uses_same_hash_as_fetch`
- `chaos-tests/tests/audit/cache_disk.rs::audit_block_cache_sync_preserves_existing_gkl_owner`
- `chaos-tests/tests/audit/cache_disk.rs::audit_mount_prefix_matches_path_component_boundary`
- `chaos-tests/tests/audit/cache_disk.rs::audit_ioqueue_merge_adjacent_overflow_does_not_panic`
- `chaos-tests/tests/audit/cache_disk.rs::audit_ioqueue_submit_batch_does_not_deadlock_when_over_depth`
- `chaos-tests/tests/audit/cache_disk.rs::audit_disk_read_variants_fill_same_success_pattern`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:2435`, `PageCache::insert`
   - A zero-capacity cache still stores entries.
   - Expected invariant: capacity is a hard upper bound.

2. `kernel/src/kernel.rs:2435`, `PageCache::insert`
   - When all current entries are pinned, `evict_lru()` fails but insertion still proceeds and exceeds capacity.
   - Expected invariant: if no page is evictable, inserting a new page must fail or be skipped.

3. `kernel/src/kernel.rs:2691`, `BlockCache::fetch`
   - `BlockCache::new(0)` allows a zero-width cache, then `fetch` panics on modulo by zero.
   - Expected invariant: invalid cache width is rejected or behaves as an empty cache without panicking.

4. `kernel/src/kernel.rs:2771`, `BlockCache::invalidate`
   - `fetch` hashes with `(k ^ (k >> 7)) % width`, but `invalidate` uses `k % width`, so fetched entries can survive invalidation.
   - Expected invariant: all block-cache operations use the same chain index function.

5. `kernel/src/kernel.rs:2765`, `BlockCache::sync_all`
   - If the caller already holds `GKL`, `sync_all` increments recursive depth but then unconditionally clears owner/depth/flag.
   - Expected invariant: a helper that enters an already-held global lock preserves the previous lock state on return.

6. `kernel/src/kernel.rs:2874` and `kernel/src/kernel.rs:2948`, `MountTable::resolve` / `find_mount`
   - Prefix matching treats `/mnt` as matching `/mnted/file`.
   - Expected invariant: mount prefixes match complete path components, unless the prefix is `/`.

7. `kernel/src/kernel.rs:3062`, `IoQueue::merge_adjacent`
   - `q[i].block + 1` can overflow for `usize::MAX`.
   - Expected invariant: merge checks use checked arithmetic and never panic.

8. `kernel/src/kernel.rs:3019`, `IoQueue::submit_batch`
   - It calls `merge_adjacent()` while still holding `pending`, and `merge_adjacent()` locks `pending` again, causing deadlock when depth exceeds `IOQUEUE_DEPTH`.
   - Expected invariant: batch submission must not re-lock the same non-recursive mutex.

9. `kernel/src/kernel.rs:3128`, `Disk::read_block_n`
   - Successful limited reads fill `0xAA ^ index`, while `read_block` fills `0xAA`.
   - Expected invariant: both successful disk-read paths expose the same block data pattern in this simulation.

### 2026-05-15: IPC/Semaphore/Shared-Memory Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_ipc`: fails with 6 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/ipc.rs::audit_semarr_rejects_zero_semaphore_create`
- `chaos-tests/tests/audit/ipc.rs::audit_semarr_existing_key_rejects_larger_nsems_request`
- `chaos-tests/tests/audit/ipc.rs::audit_semctx_remove_clears_stale_undo_before_id_reuse`
- `chaos-tests/tests/audit/ipc.rs::audit_semctx_drop_replays_full_undo_magnitude`
- `chaos-tests/tests/audit/ipc.rs::audit_shm_private_key_creates_unique_segments`
- `chaos-tests/tests/audit/ipc.rs::audit_shm_existing_key_rejects_larger_size_request`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:3204`, `SemArr::get_or_create`
   - Creating a semaphore array with `nsems == 0` succeeds and produces an unusable empty array.
   - Expected invariant: new semaphore arrays must contain at least one semaphore.

2. `kernel/src/kernel.rs:3215`, `SemArr::get_or_create`
   - Existing-key lookup ignores a larger requested `nsems` and returns the smaller existing array.
   - Expected invariant: existing arrays must satisfy the requested size, or the call must fail.

3. `kernel/src/kernel.rs:3252`, `SemCtx::remove`
   - Removing a semaphore id does not clear its undo records, so a reused id can apply stale undo to a different array.
   - Expected invariant: removing an id also removes all undo records for that id.

4. `kernel/src/kernel.rs:3269`, `SemCtx::drop`
   - Undo replay only handles `op == 1`; larger positive undo magnitudes are ignored.
   - Expected invariant: dropping a semaphore context applies the full stored undo adjustment.

5. `kernel/src/kernel.rs:3295`, `shm_get_or_create`
   - Key `0` returns the same live segment on repeated calls.
   - Expected invariant: private shared-memory keys create unique segments.

6. `kernel/src/kernel.rs:3296`, `shm_get_or_create`
   - Existing-key lookup ignores a larger requested size and returns an undersized segment.
   - Expected invariant: existing shared-memory segments must satisfy the requested size, or the call must fail.

### 2026-05-15: Process Init/Capability/Signal/Timer Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_signal_timer`: fails with 8 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/signal_timer.rs::audit_procinit_push_at_small_stack_does_not_underflow`
- `chaos-tests/tests/audit/signal_timer.rs::audit_capset_inherit_keeps_inheritable_capabilities`
- `chaos-tests/tests/audit/signal_timer.rs::audit_capset_inherit_does_not_leave_ambient_without_permitted_bit`
- `chaos-tests/tests/audit/signal_timer.rs::audit_sigset_signal_zero_is_not_pending`
- `chaos-tests/tests/audit/signal_timer.rs::audit_sigset_pending_large_signal_does_not_panic`
- `chaos-tests/tests/audit/signal_timer.rs::audit_sigset_set_action_ignores_signal_zero`
- `chaos-tests/tests/audit/signal_timer.rs::audit_timer_entry_expires_at_deadline`
- `chaos-tests/tests/audit/signal_timer.rs::audit_timer_reset_overflow_does_not_panic`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:3333`, `ProcInit::push_at`
   - Small stack tops underflow during subtraction and panic.
   - Expected invariant: stack layout detects insufficient space without arithmetic underflow.

2. `kernel/src/kernel.rs:3399`, `CapSet::inherit`
   - Inheritance keeps bits outside `INHERITABLE_MASK` and drops bits inside the mask.
   - Expected invariant: inheritable capabilities named by the mask remain available to the child.

3. `kernel/src/kernel.rs:3407`, `CapSet::inherit`
   - Ambient capabilities are copied unchanged even when the corresponding permitted bit was dropped.
   - Expected invariant: ambient capabilities remain a subset of permitted capabilities.

4. `kernel/src/kernel.rs:3443`, `SigSet::sig_raise`
   - Signal number `0` is accepted into `pending`, but it is never deliverable.
   - Expected invariant: signal `0` is not queued as a real pending signal.

5. `kernel/src/kernel.rs:3439`, `SigSet::sig_pending`
   - Querying signal `64` panics with shift overflow.
   - Expected invariant: out-of-range signal queries return false.

6. `kernel/src/kernel.rs:3490`, `SigSet::set_action`
   - Signal `0` can have an action installed.
   - Expected invariant: signal actions are valid only for real signals `1..NSIG`.

7. `kernel/src/kernel.rs:3527`, `TimerEntry::expired`
   - A timer whose deadline equals the current clock tick is not considered expired.
   - Expected invariant: timers expire at `now >= deadline`.

8. `kernel/src/kernel.rs:3532`, `TimerEntry::reset`
   - Repeating timer reset can overflow `now + interval`.
   - Expected invariant: invalid or overflowing deadlines are handled without panic.

### 2026-05-15: Context/Trap/Clock Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_context_trap`: fails with 4 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/context_trap.rs::audit_trap_handle_irq_preserves_existing_active_state`
- `chaos-tests/tests/audit/context_trap.rs::audit_trap_handle_irq_preserves_existing_irq_state`
- `chaos-tests/tests/audit/context_trap.rs::audit_trap_dispatch_vector_14_reaches_page_fault_handler`
- `chaos-tests/tests/audit/context_trap.rs::audit_up_ms_overflow_does_not_panic`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:3868`, `TrapCtl::handle_irq`
   - The previous `active` state is saved but ignored; the handler always stores `false` on exit.
   - Expected invariant: nested/previous active state is restored after IRQ handling.

2. `kernel/src/kernel.rs:3848`, `TrapCtl::handle_irq`
   - The previous `irq_on` state is saved but ignored; the handler leaves IRQ state as `true`.
   - Expected invariant: IRQ enable state is restored after IRQ handling.

3. `kernel/src/kernel.rs:3880`, `TrapCtl::dispatch_vector`
   - Vector `14` is matched by the `8..=15` branch before the explicit page-fault branch, so the page-fault branch is unreachable unless the software mask happens to dispatch it.
   - Expected invariant: page-fault vector dispatch reaches `on_pgfault` and saves a trap frame.

4. `kernel/src/kernel.rs:3939`, `up_ms`
   - `wclk() * USEC_TICK` can overflow for large clock values.
   - Expected invariant: uptime conversion uses checked or saturating arithmetic.

### 2026-05-15: Scheduler/Task Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_scheduler`: fails with 8 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/scheduler.rs::audit_schedule_policy_negative_priority_does_not_panic`
- `chaos-tests/tests/audit/scheduler.rs::audit_runqueue_enqueue_rejects_duplicate_task`
- `chaos-tests/tests/audit/scheduler.rs::audit_runqueue_preempt_enable_at_zero_does_not_underflow`
- `chaos-tests/tests/audit/scheduler.rs::audit_runqueue_update_vruntime_overflow_does_not_panic`
- `chaos-tests/tests/audit/scheduler.rs::audit_fork_task_links_child_once`
- `chaos-tests/tests/audit/scheduler.rs::audit_reap_removes_child_from_parent_list`
- `chaos-tests/tests/audit/scheduler.rs::audit_task_send_sig_coalesces_duplicate_standard_signals`
- `chaos-tests/tests/audit/scheduler.rs::audit_task_set_cloexec_updates_file_state`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:3958`, `SchedulePolicy::with_prio`
   - Negative priorities panic when cast to `usize` and subtracted from `20`.
   - Expected invariant: priority values are clamped or validated without arithmetic underflow.

2. `kernel/src/kernel.rs:3990`, `RunQueue::enqueue`
   - Duplicate task detection is computed but ignored, so the same task can be queued more than once.
   - Expected invariant: a runnable task appears at most once in a run queue.

3. `kernel/src/kernel.rs:4108`, `RunQueue::preempt_enable`
   - Enabling preemption when the count is already zero wraps the counter to `usize::MAX`.
   - Expected invariant: preemption count never underflows.

4. `kernel/src/kernel.rs:4096`, `RunQueue::update_vruntime`
   - `delta * 1024` can overflow before scaling by task weight.
   - Expected invariant: runtime accounting uses checked or saturating arithmetic.

5. `kernel/src/kernel.rs:4530` and `kernel/src/kernel.rs:4534`, `TaskTable::fork_task`
   - The child is pushed into the parent's `subtasks` list twice.
   - Expected invariant: one fork creates one parent-child edge.

6. `kernel/src/kernel.rs:4477`, `TaskTable::reap`
   - Reaping a child removes it from the global map but not from its parent's `subtasks` list.
   - Expected invariant: parent child lists do not retain reaped children.

7. `kernel/src/kernel.rs:4359`, `Task::send_sig`
   - Duplicate standard-signal detection is computed but ignored.
   - Expected invariant: duplicate standard pending signals coalesce.

8. `kernel/src/kernel.rs:4419`, `Task::set_cloexec`
   - The method validates the fd but never updates the stored file's `cloexec` state.
   - Expected invariant: successful `FD_CLOEXEC` changes are visible in the fd table.

### 2026-05-15: Syscall Facade Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_syscalls`: fails with 10 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/syscalls.rs::audit_sys_close_removes_fd_from_current_task`
- `chaos-tests/tests/audit/syscalls.rs::audit_sys_dup_installs_returned_fd`
- `chaos-tests/tests/audit/syscalls.rs::audit_sys_fork_creates_child_task`
- `chaos-tests/tests/audit/syscalls.rs::audit_sys_sigaction_allows_catchable_signal`
- `chaos-tests/tests/audit/syscalls.rs::audit_sys_kill_rejects_nsig`
- `chaos-tests/tests/audit/syscalls.rs::audit_sys_futex_wake_zero_wakes_none`
- `chaos-tests/tests/audit/syscalls.rs::audit_sys_mmap_huge_length_does_not_panic`
- `chaos-tests/tests/audit/syscalls.rs::audit_sys_munmap_zero_length_is_rejected`
- `chaos-tests/tests/audit/syscalls.rs::audit_sys_epoll_wait_buffer_size_overflow_does_not_panic`
- `chaos-tests/tests/audit/syscalls.rs::audit_sys_write_cross_page_returns_requested_count`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:4886`, `Kernel::dispatch_syscall` / `SYS_CLOSE`
   - Close returns success without removing the fd from the current task's fd table.
   - Expected invariant: a successful close invalidates that fd for the task.

2. `kernel/src/kernel.rs:5048`, `Kernel::dispatch_syscall` / `SYS_DUP`
   - Dup returns a free fd number but does not install a duplicated file entry.
   - Expected invariant: the returned fd is immediately usable.

3. `kernel/src/kernel.rs:5091`, `Kernel::dispatch_syscall` / `SYS_FORK`
   - Fork reserves a pid from `seq` but does not create/register a child task.
   - Expected invariant: a successful fork returns the pid of a real child task.

4. `kernel/src/kernel.rs:5482`, `Kernel::dispatch_syscall` / `SYS_SIGACTION`
   - The catchability check is inverted: catchable signals return `einval`, while uncatchable signals pass.
   - Expected invariant: `SIGKILL` and `SIGSTOP` are rejected, ordinary valid signals are accepted.

5. `kernel/src/kernel.rs:5235`, `Kernel::dispatch_syscall` / `SYS_KILL`
   - `sig == NSIG` is accepted because the check uses `>` instead of `>=`.
   - Expected invariant: valid real signal numbers are below `NSIG`, with signal `0` handled only as an existence probe.

6. `kernel/src/kernel.rs:5532`, `Kernel::dispatch_syscall` / `SYS_FUTEX`
   - Wake with `val == 0` is converted to waking one waiter.
   - Expected invariant: wake count zero wakes zero waiters.

7. `kernel/src/kernel.rs:4931`, `Kernel::dispatch_syscall` / `SYS_MMAP`
   - Page-aligning a huge length can overflow and panic.
   - Expected invariant: overflowing lengths return an error.

8. `kernel/src/kernel.rs:4957`, `Kernel::dispatch_syscall` / `SYS_MUNMAP`
   - `len == 0` succeeds.
   - Expected invariant: zero-length unmap returns `einval`.

9. `kernel/src/kernel.rs:5439`, `Kernel::dispatch_syscall` / `SYS_EPOLL_WAIT`
   - `max_events * size_of::<EpEvent>()` can overflow before validation.
   - Expected invariant: oversized event buffers return an error without panic.

10. `kernel/src/kernel.rs:4796`, `Kernel::dispatch_syscall` / `SYS_WRITE`
   - Cross-page writes add `page_off` into the returned byte count, so the syscall can report more bytes than requested.
   - Expected invariant: successful write returns the requested count or fewer, never more.

### 2026-05-15: Access/Utility Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_utils`: fails with 2 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/utils.rs::audit_validate_access_accepts_last_user_byte`
- `chaos-tests/tests/audit/utils.rs::audit_mem_scan_pattern_respects_zero_match_limit`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:5813`, `validate_access`
   - It rejects ranges whose exclusive end equals `KERN_BASE`, while `check_access` accepts the same valid last-user-byte range.
   - Expected invariant: all user-access validators agree that `[KERN_BASE - 1, KERN_BASE)` is valid.

2. `kernel/src/kernel.rs:5854`, `mem_scan_pattern`
   - With `max_matches == 0`, the function still records the first match before checking the limit.
   - Expected invariant: a zero match limit returns an empty result.

### 2026-05-15: Address-Space/Process-Group/Wait/Resource Slice

Command status:

- `cd chaos-tests && cargo test --test basic`: passes with 33 tests.
- `cd chaos-tests && cargo test --test audit_resources`: fails with 6 focused audit failures.

New focused tests:

- `chaos-tests/tests/audit/resources.rs::audit_addrspace_fork_increments_writable_region_refcount_once`
- `chaos-tests/tests/audit/resources.rs::audit_addrspace_split_region_produces_non_overlapping_halves`
- `chaos-tests/tests/audit/resources.rs::audit_addrspace_unmap_range_overflow_does_not_panic`
- `chaos-tests/tests/audit/resources.rs::audit_waitqueue_timeout_returns_false_when_not_woken`
- `chaos-tests/tests/audit/resources.rs::audit_waitqueue_timeout_preserves_other_same_key_waiters`
- `chaos-tests/tests/audit/resources.rs::audit_resource_limits_exceeds_any_matches_individual_fd_check`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:5928` and `kernel/src/kernel.rs:5944`, `AddrSpace::fork_from`
   - Writable VM regions have their parent refcount incremented in two separate loops.
   - Expected invariant: one fork adds one reference to each shared writable region.

2. `kernel/src/kernel.rs:6015`, `AddrSpace::split_region`
   - Splitting adds the second half but leaves the original region unchanged, producing overlapping regions.
   - Expected invariant: split replaces one region with two non-overlapping halves.

3. `kernel/src/kernel.rs:5975`, `AddrSpace::unmap_range`
   - `start + len` can overflow and panic.
   - Expected invariant: overflowing unmap ranges fail without panic.

4. `kernel/src/kernel.rs:6112`, `WaitQueue::sleep_timeout`
   - Timeout returns true because it removes its own waiter and interprets removal as success.
   - Expected invariant: timeout without wake returns false.

5. `kernel/src/kernel.rs:6119`, `WaitQueue::sleep_timeout`
   - Timeout cleanup removes every waiter with the same key, not just the current timed-out waiter.
   - Expected invariant: one timed-out waiter does not delete unrelated waiters.

6. `kernel/src/kernel.rs:6251`, `ResourceLimits::exceeds_any`
   - It treats equality to a limit as non-exceeding, while individual allocation checks reject `current == limit`.
   - Expected invariant: aggregate checks match individual resource-boundary checks.

### 2026-05-15: Bit/Buddy Allocator Slice

Command status:

- `cd chaos-tests && cargo test --test audit_buddy`: fails with 3 focused audit failures.
- `cd chaos-tests && cargo test --test basic`: passes with 33 tests on rerun. One immediate prior run hit a parallel-test `GKL` global-state flake in `group_01::basic_bkl_single_acquire_release`.

New focused tests:

- `chaos-tests/tests/audit/buddy.rs::audit_align_up_overflow_does_not_panic`
- `chaos-tests/tests/audit/buddy.rs::audit_buddy_double_free_does_not_duplicate_free_block`
- `chaos-tests/tests/audit/buddy.rs::audit_buddy_coalesces_blocks_with_nonzero_base`

Located bugs pending approval before kernel fixes:

1. `kernel/src/kernel.rs:6300`, `align_up`
   - `addr + align - 1` can overflow and panic.
   - Expected invariant: impossible aligned addresses are handled without panic.

2. `kernel/src/kernel.rs:6389`, `BuddyAllocator::free_order`
   - Double-freeing the same block inserts duplicate free blocks and underflows `allocated`.
   - Expected invariant: a block can be present in the free lists at most once, and allocated pages never underflow.

3. `kernel/src/kernel.rs:6395`, `BuddyAllocator::free_order`
   - Buddy calculation uses `current_addr ^ block_size`, which only works for zero-based allocators; nonzero `base_addr` blocks do not coalesce.
   - Expected invariant: buddy addresses are computed relative to `base_addr`.

### 2026-06-07：B05.3 fd lifecycle / cloexec 迁移记录

源码范围：

- `kernel/src/process/proc.rs`：`Process.fd_cloexec`、`add_file_with_cloexec`、`close_file`、`is_fd_cloexec`、`set_fd_cloexec`。
- `kernel/src/process/thread.rs`：新进程 fd-local flags 初始化、fork 继承。
- `kernel/src/syscall/fs.rs`：open/pipe/epoll create、close、dup2/dup3、`F_DUPFD`、`F_DUPFD_CLOEXEC`、`F_GETFD`、`F_SETFD`。
- `kernel/src/syscall/proc.rs`：exec close-on-exec loop。

`kernel.rs` 对应语义：

- `kernel/src/kernel.rs:5736-5801`：`Task::{close_fd,dup_fd,dup2_fd,set_cloexec}`。
- `kernel/src/kernel.rs:6888-6932`：`SYS_FCNTL` fd flag 分支。
- `kernel/src/kernel.rs:7436-7475`：exec close-on-exec loop。

迁移不变量：

- `FD_CLOEXEC` 是 fd-local metadata，不属于 socket/epoll/open-file-description 共享对象。
- `dup2(old, old)` 不关闭 fd；`dup3(old, old)` 返回 `EINVAL`。
- `F_DUPFD` 和 `F_DUPFD_CLOEXEC` 必须返回不小于 arg 的新 fd，后者设置 fd-local close-on-exec。
- exec 关闭所有 fd-local cloexec fd，非 cloexec fd 保留。

验证：

- `rustfmt --edition 2018 kernel/src/process/proc.rs kernel/src/process/thread.rs kernel/src/syscall/fs.rs kernel/src/syscall/proc.rs`
- `cd kernel && make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy`
- `cd kernel && timeout 15s /home/zcychar/qemu-7.0.0/build/qemu-system-riscv64 ...`
- QEMU 已进入 busybox shell `/ #`；`timeout` 退出码 124 为预期外部终止；未运行 `chaos-tests`。

### 2026-06-07：B05.1 stack init / B05.4 ELF bounds 迁移记录

源码范围：

- `kernel/src/process/abi.rs`：`ProcInitInfo::{try_push_at,try_push_at_in_vm,push_to}`、`InitStackWriter`、`StackWriter::push_slice`、`VmStackWriter::{push_slice,write_bytes}`。
- `kernel/src/process/structs.rs`：`ElfExt::{make_memory_set,append_as_interpreter,get_phdr_vaddr}`、`u64_to_usize`。
- `kernel/src/process/thread.rs`：`Thread::new_user_vm`。

`kernel.rs` 对应语义：

- `kernel/src/kernel.rs:4387-4470`：`ProcInit::push_at` 的 checked subtraction 和 sentinel failure。
- `kernel/src/kernel.rs:1950-2022`：`validate_elf_header` 的 PHDR/LOAD validation。

迁移不变量：

- init stack 的 size 乘法、sp 下移、alignment、VA 加法、flush range 不 wrap；page prepare 失败返回错误。
- `Thread::new_user_vm` 在临时 `MemorySet` 上完成 ELF/stack 装载，成功后再替换传入 VM。
- ELF LOAD virtual/file range、interpreter bias、PHDR inferred address、farthest memory 均 checked。
- malformed ELF、无 LOAD、空 LOAD range 或 `file_size > mem_size` 通过 `new_user_vm` 返回错误，并由 `sys_exec` 映射为 `EINVAL`。

验证：

- `rustfmt --edition 2018 kernel/src/process/abi.rs kernel/src/process/structs.rs kernel/src/process/thread.rs`
- `cd kernel && make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy`
- `cd kernel && timeout 15s /home/zcychar/qemu-7.0.0/build/qemu-system-riscv64 ...`
- QEMU 已进入 busybox shell `/ #`；`timeout` 退出码 124 为预期外部终止；未运行 `chaos-tests`。

### 2026-06-07：B05.2 fork/wait parent-child 迁移记录

源码范围：

- `kernel/src/process/proc.rs`：`Process::reparent_children_to_init`、`Process::exit`。
- `kernel/src/syscall/proc.rs`：`Syscall::sys_wait4`。

`kernel.rs` 对应语义：

- `kernel/src/kernel.rs:5814-5935`：`TaskTable::{fork_task,reap}`。
- `kernel/src/kernel.rs:6700-6815`：`SYS_EXIT` / `SYS_WAIT4` 的退出、等待和回收语义。

迁移不变量：

- `wait4(pid > 0)` 只能匹配当前进程的 child，不能先通过全局进程表观察或回收无关进程。
- `wait4(0)` 匹配当前进程组的 children；`wait4(pid < -1)` 匹配 pgid 为 `-pid` 的 children。
- reap 先写回 `wstatus`，再同步清理 global process table 和 parent children list。
- 非 init 父进程退出时，living/zombie children 转交给 pid 1；已退出 child 转交后唤醒 init 的 child wait。

验证：

- `rustfmt --edition 2018 kernel/src/syscall/proc.rs kernel/src/process/proc.rs`
- `cd kernel && make build ARCH=riscv64 LOG=debug objcopy=/usr/bin/llvm-objcopy`
- `cd kernel && timeout 15s /home/zcychar/qemu-7.0.0/build/qemu-system-riscv64 ...`
- QEMU 已进入 busybox shell `/ #`；`timeout` 退出码 124 为预期外部终止；未运行 `chaos-tests`。
