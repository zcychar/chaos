use chaos_tests::*;

#[test]
fn audit_schedule_policy_negative_priority_does_not_panic() {
    let result = std::panic::catch_unwind(|| SchedulePolicy::with_prio(-1));

    assert!(result.is_ok());
}

#[test]
fn audit_runqueue_enqueue_rejects_duplicate_task() {
    let rq = RunQueue::new();
    rq.enqueue(1, SchedulePolicy::new());
    rq.enqueue(1, SchedulePolicy::new());

    assert_eq!(rq.len(), 1);
}

#[test]
fn audit_runqueue_preempt_enable_at_zero_does_not_underflow() {
    let rq = RunQueue::new();

    rq.preempt_enable();

    assert!(rq.preemptible());
}

#[test]
fn audit_runqueue_update_vruntime_overflow_does_not_panic() {
    let rq = RunQueue::new();
    rq.enqueue(1, SchedulePolicy::new());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rq.update_vruntime(1, usize::MAX as u64);
    }));

    assert!(result.is_ok());
}

#[test]
fn audit_fork_task_links_child_once() {
    let table = TaskTable::new();
    let root = table.spawn_root();

    let _child = table.fork_task(&root);

    assert_eq!(root.n_children(), 1);
}

#[test]
fn audit_reap_removes_child_from_parent_list() {
    let table = TaskTable::new();
    let root = table.spawn_root();
    let child = table.fork_task(&root);
    let child_id = child.id();

    table.reap(child_id);

    assert_eq!(root.n_children(), 0);
}

#[test]
fn audit_task_send_sig_coalesces_duplicate_standard_signals() {
    let task = Task::make(1, "worker");

    task.send_sig(SIGCHLD as i32, -1);
    task.send_sig(SIGCHLD as i32, -1);

    assert_eq!(task.sig_queue.lock().unwrap().len(), 1);
}

#[test]
fn audit_task_set_cloexec_updates_file_state() {
    let task = Task::make(1, "fd-owner");
    let file = FHandle::new("tmp", FdOpt::default(), false, false);
    let fd = task.add_file(FLike::File(file));

    assert_eq!(task.set_cloexec(fd, true), Ok(()));

    match task.get_file(fd).unwrap() {
        FLike::File(file) => assert!(file.cloexec),
        _ => unreachable!(),
    }
}
