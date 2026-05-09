# Runtime Failures

This log tracks runtime failures found after the compile gate passed. Code fixes must be applied after reporting the bug and receiving user approval.

Command: `cd chaos-tests && cargo test --test basic`

## Pending

- `group_01::basic_bkl_double_acquire_single_release`: `GKL.held()` is false after a double acquire and single release.
- `group_01::basic_cross_module_lock_order`: worker did not complete under expected lock behavior.
- `group_02::basic_sleep_under_spinlock_uniprocessor`: channel guard remains held after sleep path.
- `group_03::basic_condvar_signal_before_wait`: signal-before-wait did not let the waiter complete.
- `group_03::basic_spurious_wakeup_no_recheck`: wait returned true despite no real signal.
- `group_06::basic_block_read_success`: successful block read did not fill the buffer with `0xAA`.
- `group_08::basic_ring_full_reject`: full circular buffer accepted one extra byte.
- `group_09::basic_interrupt_mask_set`: interrupt mask set returned `255` instead of `0`.
- `group_09::basic_save_restore_context`: restored context register value was `187` instead of `170`.
- `group_09::basic_page_fault_in_process_context`: page-fault handler returned an error.
- `group_10::basic_access_ok_overflow`: overflowing user range was accepted.
- `group_11::basic_fork_exec_workload`: `GKL` remained held after integrated fork/exec workload.
- `group_11::basic_mmap_file_io_workload`: overflowing user range was accepted.

## Fixed

None yet.
