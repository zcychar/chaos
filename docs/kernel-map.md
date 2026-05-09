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
| 1-167 | Imports and constants | rCore syscall, memory, signal, scheduler constants | Confirm public constants match test expectations and syscall numbers. |
| 169-630 | Synchronization primitives, event bus, futex table | rCore sync/futex concepts | Check lost wakeups, poisoning tolerance, and nonblocking paths. |
| 631-1209 | Address helpers, frames, VM regions/maps, user copy helpers | rCore memory and page-table layers | Check overflow, alignment, permissions, and copy-on-write semantics. |
| 1214-1592 | Heap, buffers, slabs, ELF/network/checksum/scheduling helpers | rCore loader, allocator, net helpers | Check boundary values and deterministic helper output. |
| 1608-2134 | File handles, pipes, file-like enum, epoll | rCore fs/syscall fs | Check offset sharing, fd options, pipe closure, readiness, and event masks. |
| 2134-2356 | Terminal I/O and channels | rCore tty/console concepts | Check canonical mode, echo behavior, process-group interactions, and channel EOF. |
| 2356-3124 | Page cache, kernel object registry, block cache, mounts, I/O queue, disk | rCore block/fs/cache concepts | Check cache invalidation, mount resolution, I/O completion, and bounds checks. |
| 3130-3289 | IPC permissions, semaphores, shared memory contexts | rCore IPC/syscall ipc | Check permission logic, clone/drop ownership, and ID reuse. |
| 3289-3574 | Process init, capabilities, signal sets/actions, timers | rCore process/signal/time | Check signal masks, uncatchable signals, timer ordering, and inheritance. |
| 3574-3913 | Context, traps, clocks, serial helpers | rCore arch trap/context/time | Check register bounds, syscall dispatch setup, and tick accounting. |
| 3916-4583 | Scheduler, run queues, tasks, task table | rCore task/scheduler | Check priorities, fork/exec inheritance, parent-child links, wait state, and PID reuse. |
| 4585-5771 | Kernel facade and syscall implementations | rCore syscall layer | Check errno-like returns, resource lifetime, fd/task lookup, and argument validation. |
| 5772-5868 | Access validation and utility encoders/checksums | rCore user access/utilities | Check integer overflow and empty-pattern edge cases. |
| 5868-6220 | Address spaces, process groups, wait queues, resource limits | rCore memory/process/wait/resource concepts | Check clone semantics, group/session invariants, wait filtering, and limit enforcement. |
| 6223-6403 | Bit utilities and buddy allocator | rCore allocator helpers | Check zero/overflow cases, alignment, coalescing, and free-list invariants. |

## Recheck Log

- Pending: compile gate `cd chaos-tests && cargo test --test basic`.
- Pending: group-by-group test gate from `group_01` through `group_11`.
- Pending: module-by-module manual recheck after visible tests pass.
