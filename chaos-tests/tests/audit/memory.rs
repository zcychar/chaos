use chaos_tests::*;

#[test]
fn audit_pgframe_down_does_not_underflow() {
    let frame = PgFrame::new();

    assert_eq!(frame.down(), 0);
    assert_eq!(frame.count(), 0);
}

#[test]
fn audit_vmregion_adjacent_regions_do_not_overlap_symmetrically() {
    let left = VmRegion::new(0x1000, 0x1000, VM_READ);
    let right = VmRegion::new(0x2000, 0x1000, VM_READ);

    assert!(!left.overlaps(&right));
    assert!(!right.overlaps(&left));
}

#[test]
fn audit_vmmap_rejects_kernel_crossing_region() {
    let mut map = VmMap::new();
    let region = VmRegion::new(KERN_BASE - 0x1000, 0x2000, VM_READ);

    assert!(map.insert(region).is_err());
}

#[test]
fn audit_framepool_get_contig_large_alignment_does_not_panic() {
    let pool = FramePool::new(8);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pool.get_contig(1, usize::BITS as usize)
    }));

    assert!(result.is_ok());
}

#[test]
fn audit_shared_page_fault_does_not_underflow_source_refcount() {
    let pool = FramePool::new(4);
    let source = PgFrame::new();
    let shared = SharedPage::new(0);

    let _ = shared.fault(&pool, &source);

    assert_eq!(source.count(), 0);
}
