use chaos_tests::*;
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

#[test]
fn audit_procinit_push_at_small_stack_does_not_underflow() {
    let init = ProcInit {
        args: vec!["init".to_string()],
        envs: Vec::new(),
        auxv: BTreeMap::new(),
    };

    let result = std::panic::catch_unwind(|| init.push_at(0));

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn audit_capset_inherit_keeps_inheritable_capabilities() {
    let mut parent = CapSet::new();
    parent.grant(CAP_KILL);

    let child = CapSet::inherit(&parent);

    assert!(child.check(CAP_KILL));
}

#[test]
fn audit_capset_inherit_does_not_leave_ambient_without_permitted_bit() {
    let mut parent = CapSet::new();
    parent.grant(CAP_KILL);
    assert!(parent.raise_ambient(CAP_KILL));

    let child = CapSet::inherit(&parent);

    assert_eq!(child.ambient & !child.bits, 0);
}

#[test]
fn audit_sigset_signal_zero_is_not_pending() {
    let mut set = SigSet::new();

    set.sig_raise(0);

    assert!(!set.sig_pending(0));
    assert_eq!(set.deliverable(), None);
}

#[test]
fn audit_sigset_pending_large_signal_does_not_panic() {
    let set = SigSet::new();
    let result = std::panic::catch_unwind(|| set.sig_pending(64));

    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn audit_sigset_set_action_ignores_signal_zero() {
    let mut set = SigSet::new();

    set.set_action(
        0,
        SigAction {
            handler: 0x1234,
            flags: 0,
            mask: 0,
        },
    );

    assert_eq!(set.get_action(0).handler, SIG_DFL);
}

#[test]
fn audit_timer_entry_expires_at_deadline() {
    CLK.store(100, Ordering::Relaxed);
    let timer = TimerEntry::new(100, 0, 1);

    assert!(timer.expired());
}

#[test]
fn audit_timer_reset_overflow_does_not_panic() {
    CLK.store(1, Ordering::Relaxed);
    let mut timer = TimerEntry::new(1, usize::MAX, 1);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        timer.reset();
    }));

    assert!(result.is_ok());
}
