use chaos_tests::*;
use std::sync::Arc;

fn kernel_with_root() -> (Kernel, Arc<Task>) {
    kernel_with_root_frames(64)
}

fn kernel_with_root_frames(frames: usize) -> (Kernel, Arc<Task>) {
    let kernel = Kernel::new(frames);
    kernel.proc_init();
    let root = kernel.tasks.root.lock().unwrap().clone().unwrap();
    kernel.set_cur(0, Some(root.clone()));
    (kernel, root)
}

fn rw_file(name: &str) -> FLike {
    FLike::File(FHandle::new(
        name,
        FdOpt {
            rd: true,
            wr: true,
            ap: false,
            nb: false,
        },
        false,
        false,
    ))
}

#[test]
fn audit_sys_close_removes_fd_from_current_task() {
    let (kernel, task) = kernel_with_root();
    for idx in 0..4 {
        assert_eq!(task.add_file(rw_file(&format!("fd{idx}"))), idx);
    }

    assert_eq!(kernel.dispatch_syscall(SYS_CLOSE, 3, 0, 0, 0, 0, 0), Ok(0));

    assert!(task.get_file(3).is_none());
}

#[test]
fn audit_sys_dup_installs_returned_fd() {
    let (kernel, task) = kernel_with_root();
    let old_fd = task.add_file(rw_file("dup-src"));

    let new_fd = kernel
        .dispatch_syscall(SYS_DUP, old_fd, 0, 0, 0, 0, 0)
        .unwrap();

    assert!(task.get_file(new_fd).is_some());
}

#[test]
fn audit_sys_fork_creates_child_task() {
    let (kernel, _task) = kernel_with_root_frames(N_FRAMES);

    let child_pid = kernel.dispatch_syscall(SYS_FORK, 0, 0, 0, 0, 0, 0).unwrap();

    assert!(kernel.tasks.find(child_pid).is_some());
}

#[test]
fn audit_sys_sigaction_allows_catchable_signal() {
    let (kernel, _task) = kernel_with_root();

    assert_eq!(
        kernel.dispatch_syscall(SYS_SIGACTION, SIGCHLD as usize, 0, 0, 0, 0, 0),
        Ok(0),
    );
}

#[test]
fn audit_sys_kill_rejects_nsig() {
    let (kernel, task) = kernel_with_root();

    assert_eq!(
        kernel.dispatch_syscall(SYS_KILL, task.id(), NSIG as usize, 0, 0, 0, 0),
        Err("einval"),
    );
}

#[test]
fn audit_sys_futex_wake_zero_wakes_none() {
    let (kernel, _task) = kernel_with_root();

    assert_eq!(
        kernel.dispatch_syscall(SYS_FUTEX, 0x1000, 1, 0, 0, 0, 0),
        Ok(0),
    );
}

#[test]
fn audit_sys_mmap_huge_length_does_not_panic() {
    let (kernel, _task) = kernel_with_root();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        kernel.dispatch_syscall(SYS_MMAP, 0, usize::MAX, 0, 0, 0, 0)
    }));

    assert!(result.is_ok());
    assert!(result.unwrap().is_err());
}

#[test]
fn audit_sys_munmap_zero_length_is_rejected() {
    let (kernel, _task) = kernel_with_root();

    assert_eq!(
        kernel.dispatch_syscall(SYS_MUNMAP, PAGE_SZ, 0, 0, 0, 0, 0),
        Err("einval"),
    );
}

#[test]
fn audit_sys_epoll_wait_buffer_size_overflow_does_not_panic() {
    let (kernel, _task) = kernel_with_root();
    let event_sz = std::mem::size_of::<EpEvent>();
    let max_events = usize::MAX / event_sz + 1;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        kernel.dispatch_syscall(SYS_EPOLL_WAIT, 3, 0x1000, max_events, 0, 0, 0)
    }));

    assert!(result.is_ok());
    assert!(result.unwrap().is_err());
}

#[test]
fn audit_sys_write_cross_page_returns_requested_count() {
    let (kernel, _task) = kernel_with_root();

    assert_eq!(
        kernel.dispatch_syscall(SYS_WRITE, 1, 0x1001, PAGE_SZ, 0, 0, 0),
        Ok(PAGE_SZ),
    );
}
