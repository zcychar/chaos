use chaos_tests::*;
use std::sync::atomic::Ordering;
use std::time::Duration;

#[test]
fn audit_addrspace_fork_increments_writable_region_refcount_once() {
    let mut parent = AddrSpace::new(1);
    parent
        .vm_map
        .insert(VmRegion::new(0x1000, PAGE_SZ, VM_READ | VM_WRITE))
        .unwrap();

    let _child = AddrSpace::fork_from(&parent, 2);

    assert_eq!(
        parent.vm_map.regions[0].ref_count.load(Ordering::Relaxed),
        2,
    );
}

#[test]
fn audit_addrspace_split_region_produces_non_overlapping_halves() {
    let mut space = AddrSpace::new(1);
    space
        .vm_map
        .insert(VmRegion::new(0x1000, PAGE_SZ, VM_READ))
        .unwrap();

    assert_eq!(space.split_region(0x1800), Ok(()));

    assert_eq!(space.vm_map.regions.len(), 2);
    let a = &space.vm_map.regions[0];
    let b = &space.vm_map.regions[1];
    assert!(a.end() <= b.base || b.end() <= a.base);
}

#[test]
fn audit_addrspace_unmap_range_overflow_does_not_panic() {
    let mut space = AddrSpace::new(1);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        space.unmap_range(usize::MAX, 1)
    }));

    assert!(result.is_ok());
}

#[test]
fn audit_waitqueue_timeout_returns_false_when_not_woken() {
    let wait = WaitQueue::new();

    assert!(!wait.sleep_timeout(1, 0, Duration::from_millis(1)));
}

#[test]
fn audit_waitqueue_timeout_preserves_other_same_key_waiters() {
    let wait = WaitQueue::new();
    wait.inner
        .lock()
        .unwrap()
        .push_back((7, std::thread::current(), 0));

    let _ = wait.sleep_timeout(7, 0, Duration::from_millis(1));

    assert!(wait.has_waiters_for(7));
}

#[test]
fn audit_resource_limits_exceeds_any_matches_individual_fd_check() {
    let limits = ResourceLimits::default_limits();

    assert!(!limits.check_fd(limits.max_fds));
    assert!(limits.exceeds_any(limits.max_fds, 0, 0));
}
