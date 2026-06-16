# Runtime Failures

This log tracks runtime failures found after the compile gate passed. Code fixes must be applied after reporting the bug and receiving user approval.

Full-suite command: `cd chaos-tests && cargo test --test basic`

Last successful full runtime run: 2026-05-13

Latest result after group_01 through group_11 fixes: 33 passed, 0 failed.

Latest result after 2026-05-16 audit fix batch 1: `cd chaos-tests && cargo test --test basic -- --test-threads=1` passes with 33 tests.

Latest result after 2026-05-16 audit fix batch 2: `cd chaos-tests && cargo test --test basic -- --test-threads=1` passes with 33 tests.

Latest result after 2026-05-16 audit fix batch 3: `cd chaos-tests && cargo test --test basic -- --test-threads=1` passes with 33 tests.

Latest result after 2026-05-16 audit fix batch 4: `cd chaos-tests && cargo test --test basic -- --test-threads=1` passes with 33 tests.

Final 2026-05-16 selected visible/audit runtime gate after `rustfmt`: `cd chaos-tests && cargo test --test basic --test audit_sync --test audit_memory --test audit_helpers --test audit_fileio --test audit_channel --test audit_cache_disk --test audit_ipc --test audit_signal_timer --test audit_context_trap --test audit_scheduler --test audit_syscalls --test audit_utils --test audit_resources --test audit_buddy -- --test-threads=1` passes with 117 tests.

## Pending

None. The visible basic suite passes.

## Fixed

- `group_01::basic_bkl_double_acquire_single_release`: `KernLock::leave` now decrements nested depth and unlocks only on the final release.
- `group_01::basic_cross_module_lock_order`: `FramePool::get` now avoids reacquiring `GKL` when the global lock is already held.
- `group_02::basic_sleep_under_spinlock_uniprocessor`: `Channel::recv` now releases the receive spin guard before parking and retries after wake.
- `group_03::basic_condvar_signal_before_wait`: `SyncQueue::signal` now records a signal token when no waiter exists, and `park_on` consumes it instead of sleeping forever.
- `group_03::basic_spurious_wakeup_no_recheck`: `SyncQueue::park_on` now rechecks the predicate after wake and returns the actual predicate result.
- `group_06::basic_block_read_success`: `Disk::read_block` now fills successful reads with the expected `0xAA` pattern.
- `group_08::basic_ring_full_reject`: `CircBuf::push` now rejects full and zero-capacity buffers before mutating the write cursor.
- `group_09::basic_save_restore_context`: `Context::apply` now restores captured registers without swapping register 0 and 1.
- `group_09::basic_interrupt_mask_set`: `TrapCtl::configure` now applies clear/set semantics to the hardware interrupt mask.
- `group_09::basic_page_fault_in_process_context`: `TrapCtl::on_pgfault` now accepts user-address faults in process context while still rejecting inactive kernel-space faults.
- `group_10::basic_access_ok_overflow`: `check_access` now uses checked arithmetic and rejects overflowing user ranges.
- `group_11::basic_fork_exec_workload`: cleared by the group_01 global-lock fixes.
- `group_11::basic_mmap_file_io_workload`: cleared by the `check_access` overflow fix.
