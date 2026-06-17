use chaos_tests::*;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

static GKL_TEST_LOCK: Mutex<()> = Mutex::new(());

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
    let _guard = GKL_TEST_LOCK.lock().unwrap();
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
fn audit_block_cache_sync_skips_busy_chain_without_stalling_gkl() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let cache = Arc::new(BlockCache::new(64));
    let chain_idx = cache.idx(17);
    cache.chains[chain_idx].lk.acquire();

    let sync_cache = cache.clone();
    let (tx, rx) = mpsc::channel();
    let sync = thread::spawn(move || {
        sync_cache.sync_all(88);
        let _ = tx.send(());
    });

    let sync_result = rx.recv_timeout(Duration::from_millis(200)).ok();
    cache.chains[chain_idx].lk.release();
    sync.join().unwrap();

    assert_eq!(sync_result, Some(()));
    assert!(!GKL.held());
}

#[test]
fn audit_kernel_tick_skips_busy_cache_chain_without_stalling_gkl() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let kernel = Arc::new(Kernel::new(64));
    let chain_idx = kernel.cache.idx(17);
    kernel.cache.chains[chain_idx].lk.acquire();

    let tick_kernel = kernel.clone();
    let (tx, rx) = mpsc::channel();
    let tick = thread::spawn(move || {
        tick_kernel.tick(77);
        let _ = tx.send(());
    });

    let tick_result = rx.recv_timeout(Duration::from_millis(200)).ok();
    kernel.cache.chains[chain_idx].lk.release();
    tick.join().unwrap();

    assert_eq!(tick_result, Some(()));
}

#[test]
fn audit_kernel_tick_preserves_existing_gkl_owner() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let kernel = Kernel::new(64);
    GKL.enter(78);

    kernel.tick(78);
    let still_held = GKL.held();
    let owner = GKL.owner();
    let level = GKL.level();
    if still_held {
        GKL.leave();
    }

    assert!(still_held);
    assert_eq!(owner, 78);
    assert_eq!(level, 1);
}

#[test]
fn audit_scheduler_tick_not_blocked_by_block_cache_miss_latency() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
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

fn run_with_timeout<F: FnOnce() + Send + 'static>(f: F, ms: u64) -> bool {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        f();
        let _ = tx.send(());
    });
    rx.recv_timeout(Duration::from_millis(ms)).is_ok()
}

#[test]
fn audit_gkl_scoped_skips_enter_when_gkl_already_held() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    GKL.enter(5);
    gkl_scoped(99, || {
        assert!(GKL.held());
        assert_eq!(GKL.owner(), 5);
        assert_eq!(GKL.level(), 1);
    });
    assert!(GKL.held());
    assert_eq!(GKL.owner(), 5);
    assert_eq!(GKL.level(), 1);
    GKL.leave();
}

#[test]
fn audit_kernel_tick_cross_id_gkl_does_not_self_deadlock() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let kernel = Kernel::new(64);
    GKL.enter(1);
    let done = run_with_timeout(move || kernel.tick(2), 500);
    assert!(done);
    assert!(GKL.held());
    assert_eq!(GKL.owner(), 1);
    assert_eq!(GKL.level(), 1);
    GKL.leave();
}

#[test]
fn audit_block_cache_sync_cross_id_gkl_does_not_self_deadlock() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let cache = BlockCache::new(8);
    GKL.enter(10);
    let done = run_with_timeout(move || cache.sync_all(20), 500);
    assert!(done);
    assert!(GKL.held());
    assert_eq!(GKL.owner(), 10);
    GKL.leave();
}

#[test]
fn audit_framepool_get_cross_id_gkl_does_not_self_deadlock() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let pool = FramePool::new(4);
    GKL.enter(100);
    let done = run_with_timeout(
        move || {
            let _ = pool.get(200);
        },
        500,
    );
    assert!(done);
    assert!(GKL.held());
    assert_eq!(GKL.owner(), 100);
    GKL.leave();
}

#[test]
fn audit_fetch_with_gkl_held_skips_busy_chain_without_spin() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let cache = Arc::new(BlockCache::new(64));
    let chain_idx = cache.idx(17);
    GKL.enter(1);
    cache.chains[chain_idx].lk.acquire();
    let fetch_cache = cache.clone();
    let done = run_with_timeout(
        move || {
            let miss = fetch_cache.fetch(17, Duration::from_millis(0));
            assert!(miss.is_none());
        },
        200,
    );
    cache.chains[chain_idx].lk.release();
    GKL.leave();
    assert!(done);
}

#[test]
fn audit_cache_stats_with_gkl_and_busy_chain_does_not_hang() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let kernel = Kernel::new(64);
    let chain_idx = kernel.cache.idx(9);
    GKL.enter(3);
    kernel.cache.chains[chain_idx].lk.acquire();
    let k = Arc::new(kernel);
    let stats_kernel = k.clone();
    let (tx, rx) = mpsc::channel();
    let stats = thread::spawn(move || {
        let _ = stats_kernel.cache_stats();
        let _ = tx.send(());
    });
    let ok = rx.recv_timeout(Duration::from_millis(200)).ok().is_some();
    k.cache.chains[chain_idx].lk.release();
    stats.join().unwrap();
    GKL.leave();
    assert!(ok);
}

#[test]
fn audit_syscall_read_with_gkl_and_busy_chain_completes() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let kernel = Arc::new(Kernel::new(64));
    let fd = 17usize;
    let chain_idx = kernel.cache.idx(fd);
    GKL.enter(7);
    kernel.cache.chains[chain_idx].lk.acquire();
    let kern = kernel.clone();
    let (tx, rx) = mpsc::channel();
    let syscall = thread::spawn(move || {
        let r = kern.dispatch_syscall(SYS_READ, fd, 0x1000, 64, 0, 0, 0);
        let _ = tx.send(r.is_ok());
    });
    let ok = rx.recv_timeout(Duration::from_millis(200)).ok();
    kernel.cache.chains[chain_idx].lk.release();
    syscall.join().unwrap();
    GKL.leave();
    assert_eq!(ok, Some(true));
}

#[test]
fn audit_syscall_cache_index_matches_fetch_hash() {
    let cache = BlockCache::new(64);
    let key = 128usize;
    assert_ne!(key % cache.width, cache.idx(key));
    assert_eq!(cache.idx(key), (key ^ (key >> 7)) % cache.width);
}

#[test]
fn audit_syscall_read_sees_entry_after_fetch_same_key() {
    let kernel = Arc::new(Kernel::new(64));
    let fd = 128usize;
    assert!(kernel.cache.fetch(fd, Duration::from_millis(0)).is_some());
    let kern = kernel.clone();
    let read = kern.dispatch_syscall(SYS_READ, fd, 0x1000, 64, 0, 0, 0);
    assert!(read.is_ok());
    let n = read.unwrap();
    assert!(n > 0);
    let chain = kernel.cache.idx(fd);
    let ch = &kernel.cache.chains[chain];
    ch.lk.acquire();
    let has = ch.items.lock().unwrap().iter().any(|s| s.id == fd);
    ch.lk.release();
    assert!(has);
}

#[test]
fn audit_syscall_write_marks_modified_on_fetch_chain() {
    let kernel = Arc::new(Kernel::new(64));
    let fd = 128usize;
    assert!(kernel.cache.fetch(fd, Duration::from_millis(0)).is_some());
    let kern = kernel.clone();
    assert!(kern
        .dispatch_syscall(SYS_WRITE, fd, 0x2000, 8, 0, 0, 0)
        .is_ok());
    let chain = kernel.cache.idx(fd);
    let ch = &kernel.cache.chains[chain];
    ch.lk.acquire();
    let modified = ch
        .items
        .lock()
        .unwrap()
        .iter()
        .find(|s| s.id == fd)
        .map(|s| s.modified)
        .unwrap_or(false);
    ch.lk.release();
    assert!(modified);
}

#[test]
fn audit_block_cache_sync_all_skips_busy_then_clears_on_retry() {
    let cache = Arc::new(BlockCache::new(8));
    let key = 33usize;
    let ci = cache.idx(key);
    assert!(cache.fetch(key, Duration::from_millis(0)).is_some());
    let ch = &cache.chains[ci];
    ch.lk.acquire();
    {
        let mut items = ch.items.lock().unwrap();
        if let Some(s) = items.iter_mut().find(|s| s.id == key) {
            s.modified = true;
        }
    }
    let sync_cache = cache.clone();
    let (tx, rx) = mpsc::channel();
    let t = thread::spawn(move || {
        sync_cache.sync_all(1);
        let _ = tx.send(());
    });
    assert!(rx.recv_timeout(Duration::from_millis(300)).ok().is_some());
    t.join().unwrap();
    {
        let items = ch.items.lock().unwrap();
        assert!(
            items.iter().any(|s| s.id == key && s.modified),
            "while chain lock held externally, sync_all must skip without blocking"
        );
    }
    ch.lk.release();
    cache.sync_all(2);
    ch.lk.acquire();
    let dirty = ch
        .items
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.id == key && s.modified);
    ch.lk.release();
    assert!(
        !dirty,
        "second sync_all after lock release must clear modified"
    );
}
