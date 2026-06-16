use chaos_tests::*;

fn make_elf_header() -> Vec<u8> {
    let mut data = vec![0u8; 64];
    data[0] = 0x7f;
    data[1] = b'E';
    data[2] = b'L';
    data[3] = b'F';
    data[4] = 2;
    data[5] = 1;
    data[6] = 1;
    data[16..18].copy_from_slice(&2u16.to_le_bytes());
    data
}

#[test]
fn audit_slab_zeroed_alloc_clears_reused_object() {
    let mut slab = SlabEntry::new(8, 1);
    let offset = slab.slab_alloc(false).unwrap();

    slab.obj_at_mut(offset).unwrap().fill(0xAA);
    slab.slab_free(offset);

    let offset = slab.slab_alloc(true).unwrap();
    assert!(slab.obj_at(offset).unwrap().iter().all(|&b| b == 0));
}

#[test]
fn audit_slab_double_free_does_not_duplicate_slot() {
    let mut slab = SlabEntry::new(8, 1);
    let offset = slab.slab_alloc(false).unwrap();

    slab.slab_free(offset);
    slab.slab_free(offset);

    assert_eq!(slab.slab_avail(), 1);
    assert!(slab.slab_alloc(false).is_some());
    assert!(slab.slab_alloc(false).is_none());
}

#[test]
fn audit_heap_init_overflow_does_not_panic() {
    let result = std::panic::catch_unwind(|| heap_init(usize::MAX - (PAGE_SZ / 2), PAGE_SZ));

    assert!(result.is_ok());
}

#[test]
fn audit_heap_grow_huge_request_does_not_panic() {
    let pool = FramePool::new(1);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        heap_grow(&pool, usize::MAX)
    }));

    assert!(result.is_ok());
}

#[test]
fn audit_validate_elf_header_rejects_overflowing_phdr_table() {
    let mut data = make_elf_header();
    data[32..40].copy_from_slice(&(usize::MAX as u64 - 7).to_le_bytes());
    data[54..56].copy_from_slice(&8u16.to_le_bytes());
    data[56..58].copy_from_slice(&2u16.to_le_bytes());

    let result = std::panic::catch_unwind(|| validate_elf_header(&data));

    assert!(result.is_ok());
    assert!(result.unwrap().is_err());
}

#[test]
fn audit_verify_page_alignment_large_order_does_not_panic() {
    let result = std::panic::catch_unwind(|| verify_page_alignment(PAGE_SZ, usize::BITS as usize));

    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn audit_compute_rss_watermark_huge_region_does_not_panic() {
    let regions = [VmRegion::new(0x1000, usize::MAX, VM_WRITE)];
    let result = std::panic::catch_unwind(|| compute_rss_watermark(&regions, 1024));

    assert!(result.is_ok());
}
