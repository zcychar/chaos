//! Integration audits for remote signal `adv_scheduler_fs_memory_deadlock_chain`.
//! Maps scheduler + fs(cache) + memory + GKL/chain lock ordering.

use chaos_tests::*;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

static GKL_TEST_LOCK: Mutex<()> = Mutex::new(());

fn run_with_timeout<F: FnOnce() + Send + 'static>(f: F, ms: u64) -> bool {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        f();
        let _ = tx.send(());
    });
    rx.recv_timeout(Duration::from_millis(ms)).is_ok()
}

fn kernel_with_current_task(nf: usize) -> Arc<Kernel> {
    let kernel = Arc::new(Kernel::new(nf));
    kernel.proc_init();
    let root = kernel.tasks.root.lock().unwrap().clone().unwrap();
    kernel.set_cur(0, Some(root));
    kernel
}

#[test]
fn audit_chain_tick_sync_fetch_memory_no_hang() {
    let kernel = kernel_with_current_task(128);
    let key = 17usize;

    let k_fetch = kernel.clone();
    let fetch = thread::spawn(move || {
        for _ in 0..3 {
            let _ = k_fetch.cache.fetch(key, Duration::from_millis(80));
        }
    });

    let k_sync = kernel.clone();
    let sync = thread::spawn(move || {
        for i in 0..40 {
            k_sync.cache.sync_all(90 + i);
        }
    });

    let k_tick = kernel.clone();
    let tick = thread::spawn(move || {
        for i in 0..40 {
            k_tick.tick(70 + i);
        }
    });

    let k_mem = kernel.clone();
    let mem = thread::spawn(move || {
        for _ in 0..40 {
            let _ = k_mem.pool.get(301);
            let _ = k_mem.dispatch_syscall(SYS_BRK, 0x2000, 0, 0, 0, 0, 0);
        }
    });

    let done = run_with_timeout(
        move || {
            fetch.join().unwrap();
            sync.join().unwrap();
            tick.join().unwrap();
            mem.join().unwrap();
        },
        2000,
    );
    assert!(done, "tick + sync_all + fetch + memory paths must not hang");
}

#[test]
fn audit_chain_syscall_brk_while_tick_and_busy_chain() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let kernel = kernel_with_current_task(64);
    let chain_idx = kernel.cache.idx(42);
    GKL.enter(11);
    kernel.cache.chains[chain_idx].lk.acquire();

    let k_brk = kernel.clone();
    let (tx_brk, rx_brk) = mpsc::channel();
    let brk = thread::spawn(move || {
        let r = k_brk.dispatch_syscall(SYS_BRK, 0x3000, 0, 0, 0, 0, 0);
        let _ = tx_brk.send(r.is_ok());
    });

    let k_tick = kernel.clone();
    let (tx_tick, rx_tick) = mpsc::channel();
    let tick = thread::spawn(move || {
        k_tick.tick(22);
        let _ = tx_tick.send(());
    });

    let brk_ok = rx_brk.recv_timeout(Duration::from_millis(500)).ok();
    let tick_ok = rx_tick.recv_timeout(Duration::from_millis(500)).ok();
    kernel.cache.chains[chain_idx].lk.release();
    brk.join().unwrap();
    tick.join().unwrap();
    GKL.leave();

    assert_eq!(brk_ok, Some(true));
    assert_eq!(tick_ok, Some(()));
}

#[test]
fn audit_chain_frame_alloc_vs_get_under_gkl() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let pool = Arc::new(FramePool::new(32));
    GKL.enter(1005);

    let p_get = pool.clone();
    let (tx_get, rx_get) = mpsc::channel();
    let get_th = thread::spawn(move || {
        let _ = p_get.get(1006);
        let _ = tx_get.send(());
    });

    let p_alloc = pool.clone();
    let (tx_alloc, rx_alloc) = mpsc::channel();
    let alloc_th = thread::spawn(move || {
        let _ = frame_alloc(&p_alloc);
        let _ = tx_alloc.send(());
    });

    let get_done = rx_get.recv_timeout(Duration::from_millis(500)).is_ok();
    let alloc_done = rx_alloc.recv_timeout(Duration::from_millis(500)).is_ok();
    get_th.join().unwrap();
    alloc_th.join().unwrap();
    GKL.leave();

    assert!(get_done && alloc_done);
}

#[test]
fn audit_chain_schedule_tick_does_not_replace_kernel_tick() {
    let kernel = Arc::new(Kernel::new(64));
    let key = 55usize;
    let chain_idx = kernel.cache.idx(key);

    let k_fetch = kernel.clone();
    let fetch = thread::spawn(move || {
        let _ = k_fetch.cache.fetch(key, Duration::from_millis(400));
    });

    let k_sched = kernel.clone();
    let sched_done = run_with_timeout(
        move || {
            for _ in 0..20 {
                k_sched.schedule_tick(0);
            }
        },
        300,
    );
    assert!(
        sched_done,
        "schedule_tick must return without waiting on cache miss latency"
    );

    fetch.join().unwrap();

    let ch = &kernel.cache.chains[chain_idx];
    ch.lk.acquire();
    let mut marked = false;
    for slot in ch.items.lock().unwrap().iter_mut() {
        if slot.id == key {
            slot.modified = true;
            marked = true;
        }
    }
    ch.lk.release();
    assert!(
        marked,
        "fetch should have populated cache before tick maintenance"
    );

    kernel.tick(99);
    ch.lk.acquire();
    let still_dirty = ch
        .items
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.id == key && s.modified);
    ch.lk.release();
    assert!(
        !still_dirty,
        "Kernel::tick must clear modified on cache chains"
    );
}

#[test]
fn audit_chain_cross_id_tick_sync_alloc() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let kernel = Arc::new(Kernel::new(64));
    GKL.enter(outer_id());

    let k1 = kernel.clone();
    let t1 = thread::spawn(move || run_with_timeout(move || k1.tick(2), 500));

    let k2 = kernel.clone();
    let t2 = thread::spawn(move || run_with_timeout(move || k2.cache.sync_all(3), 500));

    let k3 = kernel.clone();
    let t3 = thread::spawn(move || {
        run_with_timeout(
            move || {
                let _ = k3.pool.get(4);
            },
            500,
        )
    });

    assert!(t1.join().unwrap());
    assert!(t2.join().unwrap());
    assert!(t3.join().unwrap());
    assert!(GKL.held());
    assert_eq!(GKL.owner(), outer_id());
    GKL.leave();
}

fn outer_id() -> usize {
    1007
}

#[test]
fn audit_schedule_tick_clears_block_cache_modified_like_kernel_tick() {
    let kernel = Arc::new(Kernel::new(64));
    let key = 77usize;
    let ci = kernel.cache.idx(key);
    assert!(kernel.cache.fetch(key, Duration::from_millis(0)).is_some());
    let ch = &kernel.cache.chains[ci];
    ch.lk.acquire();
    if let Some(s) = ch.items.lock().unwrap().iter_mut().find(|s| s.id == key) {
        s.modified = true;
    }
    ch.lk.release();
    kernel.schedule_tick(0);
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
        "schedule_tick should run the same non-blocking cache maintenance as Kernel::tick"
    );
}

#[test]
fn audit_chain_brk_frame_alloc_respects_gkl_like_pool_get() {
    let _guard = GKL_TEST_LOCK.lock().unwrap();
    let kernel = kernel_with_current_task(64);
    GKL.enter(2001);
    let k = kernel.clone();
    let (tx, rx) = mpsc::channel();
    let th = thread::spawn(move || {
        let r = k.dispatch_syscall(SYS_BRK, 0x3000, 0, 0, 0, 0, 0);
        let _ = tx.send(r.is_ok());
    });
    let ok = rx.recv_timeout(Duration::from_millis(500)).ok();
    th.join().unwrap();
    GKL.leave();
    assert_eq!(
        ok,
        Some(true),
        "SYS_BRK must complete while another thread holds GKL"
    );
}

#[test]
fn audit_chain_sync_all_eventually_clears_dirty_under_held_chain_lock() {
    let cache = Arc::new(BlockCache::new(8));
    let key = 44usize;
    let ci = cache.idx(key);
    assert!(cache.fetch(key, Duration::from_millis(0)).is_some());
    let ch = &cache.chains[ci];
    ch.lk.acquire();
    if let Some(s) = ch.items.lock().unwrap().iter_mut().find(|s| s.id == key) {
        s.modified = true;
    }
    let c = cache.clone();
    let sync_th = thread::spawn(move || {
        c.sync_all(9);
    });
    ch.lk.release();
    let done = run_with_timeout(move || sync_th.join().unwrap(), 500);
    assert!(
        done,
        "sync_all must not hang once external chain lock is released"
    );
    ch.lk.acquire();
    let dirty = ch
        .items
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.id == key && s.modified);
    ch.lk.release();
    assert!(!dirty, "sync_all must clear modified after acquiring chain");
}
