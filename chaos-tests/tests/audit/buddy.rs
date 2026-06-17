use chaos_tests::*;
use std::sync::atomic::Ordering;

#[test]
fn audit_align_up_overflow_does_not_panic() {
    let result = std::panic::catch_unwind(|| align_up(usize::MAX, PAGE_SZ));

    assert!(result.is_ok());
}

#[test]
fn audit_buddy_double_free_does_not_duplicate_free_block() {
    let mut buddy = BuddyAllocator::new(0x1000, 1, 0);
    let addr = buddy.alloc_order(0).unwrap();

    buddy.free_order(addr, 0);
    buddy.free_order(addr, 0);

    assert_eq!(buddy.free_pages_count(), 1);
    assert_eq!(buddy.allocated.load(Ordering::Relaxed), 0);
}

#[test]
fn audit_buddy_coalesces_blocks_with_nonzero_base() {
    let mut buddy = BuddyAllocator::new(0x1000, 2, 1);
    let first = buddy.alloc_order(0).unwrap();
    let second = buddy.alloc_order(0).unwrap();

    buddy.free_order(first, 0);
    buddy.free_order(second, 0);

    assert_eq!(buddy.largest_free_order(), 1);
    assert_eq!(buddy.free_pages_count(), 2);
}
