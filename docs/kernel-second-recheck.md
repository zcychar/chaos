# Kernel Second Recheck

Status: complete for the current visible/audit simulation pass.

This document is the post-fix overall revise requested after all approved bug fixes were applied. It records the final test gate, the second-pass review focus, and remaining risks.

## Completion Rules

- Every located bug in `docs/kernel-pre-fix-notes.md` must either be fixed or explicitly deferred with a reason.
- Each fixed subsystem must record the focused audit command and result.
- `cd chaos-tests && cargo test --test basic` must pass after each fix batch and at the end.
- The second pass must look for hidden problems and newly introduced problems, not only confirm the original failing tests.
- Any new located bug must get a pre-fix note before kernel code is changed again.

## Fix Batch Plan

| Order | Subsystem | Primary audit command | Second-pass focus |
| --- | --- | --- | --- |
| 1 | Sync, event, futex | `cd chaos-tests && cargo test --test audit_sync` | Lost/stale waiters, wake counts, timeout cleanup, signal-token behavior. |
| 2 | Memory, VM, frames | `cd chaos-tests && cargo test --test audit_memory` | Refcount saturation, half-open ranges, kernel-boundary checks, CoW source accounting. |
| 3 | Heap, slab, helpers | `cd chaos-tests && cargo test --test audit_helpers` | Checked arithmetic, zeroed allocation semantics, duplicate free rejection, malformed input handling. |
| 4 | File, pipe, epoll | `cd chaos-tests && cargo test --test audit_fileio` | Shared fd offsets, pipe endpoint lifetime, append/seek correctness, epoll dup state sharing. |
| 5 | Terminal and channel | `cd chaos-tests && cargo test --test audit_channel` | Closed-channel writes, receiver wake counts, interaction with existing blocking recv behavior. |
| 6 | Cache, mount, disk | `cd chaos-tests && cargo test --test audit_cache_disk` | Cache capacity, zero-width behavior, lock re-entry, mount component boundaries, queue merge/deadlock risks. |
| 7 | IPC, semaphores, shared memory | `cd chaos-tests && cargo test --test audit_ipc` | Creation validation, existing-key sizing, undo cleanup/replay, private shared-memory uniqueness. |
| 8 | Process init, capabilities, signals, timers | `cd chaos-tests && cargo test --test audit_signal_timer` | Stack underflow, capability inheritance masks, valid signal ranges, timer deadline overflow. |
| 9 | Context, trap, clock | `cd chaos-tests && cargo test --test audit_context_trap` | IRQ state restoration, vector dispatch ordering, uptime overflow. |
| 10 | Scheduler and tasks | `cd chaos-tests && cargo test --test audit_scheduler` | Duplicate run-queue entries, preempt counters, fork/reap parent-child invariants, fd state mutation. |
| 11 | Syscall facade | `cd chaos-tests && cargo test --test audit_syscalls` | Syscall side effects, fd/task registration, argument validation, return counts. |
| 12 | Access and utilities | `cd chaos-tests && cargo test --test audit_utils` | Access-validator consistency and bounded pattern scanning. |
| 13 | Address space, wait queues, resources | `cd chaos-tests && cargo test --test audit_resources` | Region split/unmap boundaries, timeout waiter identity, resource-limit equality semantics. |
| 14 | Bit utilities and buddy allocator | `cd chaos-tests && cargo test --test audit_buddy` | Alignment overflow, double-free rejection, nonzero-base coalescing. |

## Post-Fix Results

Record entries here as fixes land.

| Batch | Fix summary | Focused test result | Basic test result | Hidden/new issues found |
| --- | --- | --- | --- | --- |
| Sync, event, futex | Fixed stale `SyncQueue` waiters and exact `FutexTable` wake counts. | `cd chaos-tests && cargo test --test audit_sync -- --test-threads=1`: 4 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Memory, VM, frames | Fixed refcount underflow, VM overlap/bounds checks, contiguous alignment overflow, and CoW source decrement. | `cd chaos-tests && cargo test --test audit_memory -- --test-threads=1`: 5 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Heap, slab, helpers | Fixed checked heap/ELF/alignment/RSS arithmetic and slab zero/double-free behavior. | `cd chaos-tests && cargo test --test audit_helpers -- --test-threads=1`: 7 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| File, pipe, epoll | Fixed append/seek/overflow handling, pipe endpoint lifetime/write errors, and shared epoll registration state. | `cd chaos-tests && cargo test --test audit_fileio -- --test-threads=1`: 9 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Terminal and channel | Fixed closed-channel send rejection and multi-receiver batch wakeups. | `cd chaos-tests && cargo test --test audit_channel -- --test-threads=1`: 3 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Cache, mount, disk | Fixed page/block cache edge cases, GKL re-entry preservation, mount boundary matching, I/O queue deadlock/overflow, and disk read pattern consistency. | `cd chaos-tests && cargo test --test audit_cache_disk -- --test-threads=1`: 9 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| IPC, semaphores, shared memory | Fixed semaphore creation/sizing, undo cleanup/replay, private shared memory, and existing segment sizing. | `cd chaos-tests && cargo test --test audit_ipc -- --test-threads=1`: 6 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Process init, capabilities, signals, timers | Fixed stack underflow, inheritable/ambient capability filtering, signal 0/range handling, and timer deadline overflow/expiry. | `cd chaos-tests && cargo test --test audit_signal_timer -- --test-threads=1`: 8 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Context, trap, clock | Fixed IRQ state restoration, page-fault vector dispatch order, and uptime overflow. | `cd chaos-tests && cargo test --test audit_context_trap -- --test-threads=1`: 4 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Scheduler and tasks | Fixed priority arithmetic, duplicate run-queue entries, vruntime/preempt underflow, fork/reap links, standard signal coalescing, and cloexec mutation. | `cd chaos-tests && cargo test --test audit_scheduler -- --test-threads=1`: 8 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Syscall facade | Fixed syscall side effects and validation for write counts, close/dup/fork registration, mmap/munmap sizing, kill/sigaction ranges, epoll wait sizing, and futex wake/requeue counts. | `cd chaos-tests && cargo test --test audit_syscalls -- --test-threads=1`: 10 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Access and utilities | Fixed kernel-boundary range validation and zero-match pattern scanning. | `cd chaos-tests && cargo test --test audit_utils -- --test-threads=1`: 2 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Address space, wait queues, resources | Fixed fork refcount accounting, checked unmap ranges, split-region replacement, timeout waiter cleanup, and resource-limit boundary comparisons. | `cd chaos-tests && cargo test --test audit_resources -- --test-threads=1`: 6 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |
| Bit utilities and buddy allocator | Fixed alignment overflow, double-free rejection, nonzero-base buddy coalescing, and allocation accounting saturation. | `cd chaos-tests && cargo test --test audit_buddy -- --test-threads=1`: 3 passed. | `cd chaos-tests && cargo test --test basic -- --test-threads=1`: 33 passed. | None found in focused/basic/final second pass. |

## Final Verification

- `cd chaos-tests && cargo test --no-run --test basic --test audit_sync --test audit_memory --test audit_helpers --test audit_fileio --test audit_channel --test audit_cache_disk --test audit_ipc --test audit_signal_timer --test audit_context_trap --test audit_scheduler --test audit_syscalls --test audit_utils --test audit_resources --test audit_buddy`: passed after `rustfmt`.
- `cd chaos-tests && cargo test --test basic --test audit_sync --test audit_memory --test audit_helpers --test audit_fileio --test audit_channel --test audit_cache_disk --test audit_ipc --test audit_signal_timer --test audit_context_trap --test audit_scheduler --test audit_syscalls --test audit_utils --test audit_resources --test audit_buddy -- --test-threads=1`: 117 passed, 0 failed.

## Second-Pass Review

| Current Lines | Subsystem | Second-pass result |
| --- | --- | --- |
| 1-428 | Imports, constants, global lock, event/ring declarations | Constants compile through all visible/audit targets; global-lock and ring behavior rechecked by visible groups and batch tests. |
| 429-904 | Sync, event, semaphore, futex | Waiter cleanup, wake counts, and timeout behavior match the focused audit tests; no new stale-waiter issue found in touched paths. |
| 916-1689 | Address helpers, frames, VM, user copy | Checked ranges and refcount paths now avoid the original underflow/overflow cases; no new CoW or kernel-boundary regression found. |
| 1692-2205 | Heap, buffers, slabs, ELF/helpers | Overflow-prone arithmetic and slab free/zero semantics rechecked; malformed helper inputs return bounded results instead of panicking. |
| 2208-3214 | File, pipe, epoll, terminal metadata, channel | Descriptor offsets, pipe endpoint counts, epoll shared state, and closed-channel sends match intended behavior in focused and visible tests. |
| 3216-4154 | Cache, mount, disk | Cache capacity, invalidation hashes, GKL preservation, mount component boundaries, I/O merge, and disk pattern behavior rechecked cleanly. |
| 4158-4776 | IPC, process init, caps, signals, timers | IPC sizing/undo and signal/timer boundary behavior match the approved fixes; no new ID reuse or signal range issue found in the audited paths. |
| 4777-6054 | Context, trap, scheduler, tasks | IRQ restoration, vector dispatch, scheduling counters, parent-child links, reaping, and cloexec mutation rechecked without new regressions. |
| 6055-7528 | Kernel facade/syscalls | Syscall side effects now delegate to task/fd helpers where required; argument sizing and signal/futex validation passed final audits. |
| 7529-8338 | Access utilities, address spaces, wait/resources, buddy | Access boundaries, pattern limits, unmap/split behavior, timeout waiter identity, resource equality, alignment, and buddy coalescing rechecked. |

## Final Second-Pass Checklist

- [x] `docs/kernel-pre-fix-notes.md` has no unhandled located bug.
- [x] Every audit target listed in the fix batch plan passes.
- [x] `cd chaos-tests && cargo test --test basic` passes.
- [x] Cross-subsystem interactions were manually rechecked after all batches.
- [x] Any new hidden problem found during the second pass has a documented report and approval before code changes.
- [x] Remaining risks are explicitly listed below.

## Remaining Risks

- `chaos-tests/Cargo.toml` still lists `advanced` and `pressure` targets whose source files are absent, so `cargo test --tests` and `cargo fmt` for the whole manifest remain blocked by pre-existing missing files.
- The verification is the std-based simulation suite. It does not replace a RISC-V kernel build or QEMU boot/run check.
- The audit tests cover the documented invariants, but they are not a proof that every helper in the teaching kernel is free of unrelated latent issues.
