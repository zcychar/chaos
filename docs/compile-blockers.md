# Compile Blockers

This log tracks compile-blocking bugs found by `cd chaos-tests && cargo test --test basic`. Kernel code fixes must be applied after reporting the bug and receiving user approval.

## Pending

None. The visible basic suite now compiles and runs.

## Fixed

1. `kernel/src/kernel.rs:5427`
   - Fixed by adding `BOOT_EPOCH` near the timer constants with value `0`, preserving the current simulated tick behavior.
2. `kernel/src/kernel.rs:1566`
   - Fixed by annotating `order` as `usize` before calling `saturating_sub`.
3. `kernel/src/kernel.rs:1785`
   - Fixed by casting the spliced byte count to `u64` before advancing the file offset.
4. `kernel/src/kernel.rs:2983`
   - Fixed by keeping queue depth as a nonnegative `usize` and comparing directly with `IOQUEUE_DEPTH`.
5. `kernel/src/kernel.rs:3424`
   - Fixed by using a `u64` pending-signal accumulator and `1u64 << i` bits in `coalesce_pending`.
6. `kernel/src/kernel.rs:3718`
   - Fixed by returning the already-loaded register value from the fallback arm in `Context::reg_class`.
7. `kernel/src/kernel.rs:4777` and `kernel/src/kernel.rs:4862`
   - Fixed by adding `Kernel::disk` and initializing it with `Disk::new("root")`.
8. `kernel/src/kernel.rs:4827`
   - Fixed by using `FHandle::new("anon", opt, false, _cloexec)` in the simulated `SYS_OPEN` path.
9. `kernel/src/kernel.rs:4831`
   - Fixed by storing the new handle directly as `FLike::File(fh)`, preserving sharing through `FHandle` internals.
10. `kernel/src/kernel.rs:5146` and `kernel/src/kernel.rs:5182`
    - Fixed by iterating `Arc<Task>` process-group entries directly in `SYS_WAIT4` and returning `task.id()`.
11. `kernel/src/kernel.rs:6146`
    - Fixed by sorting the wait queue through `VecDeque::make_contiguous().sort_by(...)` while holding the queue mutex.
12. `kernel/src/kernel.rs:6219`
    - Fixed by returning `violations > 0` from `ResourceLimits::exceeds_any`.
13. `kernel/src/kernel.rs:6396`
    - Fixed by carrying the current allocated page count into `BuddyAllocator::snapshot`.
14. `kernel/src/kernel.rs:1928`
    - Fixed by copying the event mask before retaining pipe-read callbacks.
15. `kernel/src/kernel.rs:1971`, `kernel/src/kernel.rs:4253`, `kernel/src/kernel.rs:4261`, and `kernel/src/kernel.rs:4331`
    - Fixed by copying the event mask before retaining callbacks at the remaining event-bus sites.
16. `kernel/src/kernel.rs:5983`
    - Fixed by changing `AddrSpace::split_region` to take `&mut self`.
17. `kernel/src/kernel.rs:6049`
    - Fixed by ignoring missing task IDs after the process-group member snapshot is cloned and the lock is released.
