use chaos_tests::*;
use std::sync::atomic::Ordering;

#[test]
fn audit_trap_handle_irq_preserves_existing_active_state() {
    let trap = TrapCtl::new();
    trap.active.store(true, Ordering::SeqCst);

    let _ = trap.handle_irq(Context::new());

    assert!(trap.active.load(Ordering::SeqCst));
}

#[test]
fn audit_trap_handle_irq_preserves_existing_irq_state() {
    let trap = TrapCtl::new();
    trap.irq_on.store(false, Ordering::SeqCst);

    let _ = trap.handle_irq(Context::new());

    assert!(!trap.irq_on.load(Ordering::SeqCst));
}

#[test]
fn audit_trap_dispatch_vector_14_reaches_page_fault_handler() {
    let trap = TrapCtl::new();
    let ctx = Context::new();

    let _ = trap.dispatch_vector(14, ctx);

    assert!(trap.current().is_some());
}

#[test]
fn audit_up_ms_overflow_does_not_panic() {
    CLK.store(usize::MAX, Ordering::Relaxed);
    let result = std::panic::catch_unwind(up_ms);
    CLK.store(0, Ordering::Relaxed);

    assert!(result.is_ok());
}
