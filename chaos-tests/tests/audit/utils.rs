use chaos_tests::*;

#[test]
fn audit_validate_access_accepts_last_user_byte() {
    assert!(check_access(KERN_BASE - 1, 1));

    assert_eq!(validate_access(0, KERN_BASE - 1, 1, 0), Ok(()));
}

#[test]
fn audit_mem_scan_pattern_respects_zero_match_limit() {
    let matches = mem_scan_pattern(b"aaaa", b"a", 0);

    assert!(matches.is_empty());
}
