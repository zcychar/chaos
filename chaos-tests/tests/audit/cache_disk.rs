use chaos_tests::*;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

#[test]
fn audit_page_cache_zero_capacity_stores_nothing() {
    let mut cache = PageCache::new(0);

    cache.insert(1, vec![1, 2, 3]);

    assert_eq!(cache.entries.len(), 0);
    assert!(cache.lookup(1).is_none());
}

#[test]
fn audit_page_cache_does_not_exceed_capacity_when_all_entries_pinned() {
    let mut cache = PageCache::new(1);
    cache.insert(1, vec![1]);
    assert!(cache.pin(1));

    cache.insert(2, vec![2]);

    assert!(cache.entries.len() <= 1);
    assert!(cache.entries.contains_key(&1));
}

#[test]
fn audit_block_cache_zero_width_does_not_panic() {
    let cache = BlockCache::new(0);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cache.fetch(1, Duration::from_millis(0))
    }));

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn audit_block_cache_invalidate_uses_same_hash_as_fetch() {
    let cache = BlockCache::new(64);
    assert!(cache.fetch(128, Duration::from_millis(0)).is_some());

    cache.invalidate(128);

    assert_eq!(cache.total_entries(), 0);
}

#[test]
fn audit_block_cache_sync_preserves_existing_gkl_owner() {
    let cache = BlockCache::new(1);
    GKL.enter(42);

    cache.sync_all(42);
    let still_held = GKL.held();
    if still_held {
        GKL.leave();
    }

    assert!(still_held);
}

#[test]
fn audit_scheduler_tick_not_blocked_by_block_cache_miss_latency() {
    let kernel = Arc::new(Kernel::new(64));
    let slow_kernel = kernel.clone();

    let slow_fetch = thread::spawn(move || {
        assert!(slow_kernel
            .cache
            .fetch(17, Duration::from_millis(600))
            .is_some());
    });

    thread::sleep(Duration::from_millis(25));

    let tick_kernel = kernel.clone();
    let (tx, rx) = mpsc::channel();
    let tick = thread::spawn(move || {
        tick_kernel.tick(77);
        let _ = tx.send(());
    });

    let tick_result = rx.recv_timeout(Duration::from_millis(200)).ok();
    slow_fetch.join().unwrap();
    tick.join().unwrap();

    assert_eq!(tick_result, Some(()));
}

#[test]
fn audit_mount_prefix_matches_path_component_boundary() {
    let table = MountTable::new();
    table.bind("/mnt", "dev0");

    assert_eq!(table.resolve("/mnted/file").unwrap(), "/mnted/file");
    assert!(table.find_mount("/mnted/file").is_none());
}

#[test]
fn audit_ioqueue_merge_adjacent_overflow_does_not_panic() {
    let queue = IoQueue::new();
    queue.submit(usize::MAX, false, 0);
    queue.submit(0, false, 0);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| queue.merge_adjacent()));

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn audit_ioqueue_submit_batch_does_not_deadlock_when_over_depth() {
    let queue = IoQueue::new();
    let requests: Vec<_> = (0..(IOQUEUE_DEPTH + 1))
        .map(|block| (block, false, 0))
        .collect();
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let count = queue.submit_batch(&requests);
        let _ = tx.send(count);
    });

    assert_eq!(
        rx.recv_timeout(Duration::from_millis(200)).ok(),
        Some(IOQUEUE_DEPTH + 1),
    );
}

#[test]
fn audit_disk_read_variants_fill_same_success_pattern() {
    let d1 = Disk::new("direct");
    let d2 = Disk::new("limited");
    let mut direct = [0u8; 16];
    let mut limited = [0u8; 16];

    assert_eq!(d1.read_block(0, &mut direct), Ok(()));
    assert_eq!(d2.read_block_n(0, &mut limited, 1), Ok(1));

    assert_eq!(direct, limited);
}
