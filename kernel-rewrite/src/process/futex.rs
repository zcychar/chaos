#![allow(unused_imports)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Per-address futex wait bucket.
///
/// Each waiter records the futex address, the parked Rust thread, and a wake flag
/// used to distinguish a real wake from timeout-style returns.
pub struct FutexBucket {
    waiters: Mutex<VecDeque<(usize, thread::Thread, Arc<AtomicBool>)>>,
}

impl FutexBucket {
    pub fn new() -> Self {
        Self {
            waiters: Mutex::new(VecDeque::new()),
        }
    }

    pub fn wait(
        &self,
        addr: usize,
        expected: u32,
        val: &AtomicU32,
        timeout: Option<Duration>,
    ) -> Result<(), &'static str> {
        let wake_flag = Arc::new(AtomicBool::new(false));
        if val.load(Ordering::SeqCst) != expected {
            return Err("changed");
        }
        {
            let mut waiters = self.waiters.lock().unwrap();
            waiters.push_back((addr, thread::current(), wake_flag.clone()));
        }
        if let Some(duration) = timeout {
            thread::park_timeout(duration);
        } else {
            thread::park();
        }
        if wake_flag.load(Ordering::Relaxed) {
            Ok(())
        } else {
            self.remove_waiter(addr, &wake_flag);
            Err("timeout")
        }
    }

    // Debug fix: wait() should remove the waiter when timeouts occur.
    fn remove_waiter(&self, addr: usize, wake_flag: &Arc<AtomicBool>) -> bool {
        let mut waiters = self.waiters.lock().unwrap();
        let before = waiters.len();
        waiters.retain(|(wait_addr, _, waiter_flag)| {
            !(*wait_addr == addr && Arc::ptr_eq(waiter_flag, wake_flag))
        });
        waiters.len() != before
    }

    pub fn wake(&self, addr: usize, count: usize) -> usize {
        let mut waiters = self.waiters.lock().unwrap();
        let mut woken = 0;
        waiters.retain(|(wait_addr, waiter, wake_flag)| {
            if *wait_addr == addr && woken < count {
                wake_flag.store(true, Ordering::Relaxed);
                waiter.unpark();
                woken += 1;
                false
            } else {
                true
            }
        });
        woken
    }

    pub fn requeue(&self, src: usize, dst: usize, wake_n: usize, move_n: usize) -> usize {
        let mut waiters = self.waiters.lock().unwrap();
        let (mut woken, mut moved) = (0, 0);
        for entry in waiters.iter_mut() {
            if entry.0 == src {
                if woken < wake_n {
                    entry.2.store(true, Ordering::Relaxed);
                    entry.1.unpark();
                    woken += 1;
                } else if moved < move_n {
                    entry.0 = dst;
                    moved += 1;
                }
            }
        }
        waiters.retain(|(_, _, wake_flag)| !wake_flag.load(Ordering::Relaxed));
        woken
    }

    pub fn pending_at(&self, addr: usize) -> usize {
        self.waiters
            .lock()
            .unwrap()
            .iter()
            .filter(|(wait_addr, _, _)| *wait_addr == addr)
            .count()
    }
}

/// Simple global futex table used by tests that do not need per-task buckets.
///
/// Each entry stores a futex address and the parked Rust thread waiting on that
/// address. `FutexBucket` is the richer per-address version used elsewhere.
pub struct FutexTable {
    table: Mutex<VecDeque<(usize, thread::Thread)>>,
}

impl FutexTable {
    pub fn new() -> Self {
        Self {
            table: Mutex::new(VecDeque::new()),
        }
    }

    pub fn ftx_wait(&self, addr: usize, expected: u32, val: &AtomicU32) -> bool {
        // Debug fix: unify lock usage
        let mut waiters = self.table.lock().unwrap();
        if val.load(Ordering::SeqCst) != expected {
            return false;
        }
        waiters.push_back((addr, thread::current()));
        drop(waiters);
        thread::park();
        true
    }

    pub fn ftx_wake(&self, addr: usize, count: usize) -> usize {
        // Debug fix: waking zero waiters must return zero and must not unpark anyone.
        let mut waiters = self.table.lock().unwrap();
        let mut woken = 0usize;
        let mut cursor = 0;
        while cursor < waiters.len() && woken < count {
            if waiters[cursor].0 == addr {
                let entry = waiters.remove(cursor).unwrap();
                entry.1.unpark();
                woken += 1;
            } else {
                cursor += 1;
            }
        }
        woken
    }

    pub fn ftx_requeue(
        &self,
        src_addr: usize,
        dst_addr: usize,
        wake_n: usize,
        move_n: usize,
    ) -> usize {
        let mut waiters = self.table.lock().unwrap();
        let mut woken = 0;
        let mut moved = 0;
        let mut cursor = 0;
        while cursor < waiters.len() {
            if waiters[cursor].0 == src_addr {
                if woken < wake_n {
                    let (_, waiter) = waiters.remove(cursor).unwrap();
                    waiter.unpark();
                    woken += 1;
                } else if moved < move_n {
                    waiters[cursor].0 = dst_addr;
                    moved += 1;
                    cursor += 1;
                } else {
                    cursor += 1;
                }
            } else {
                cursor += 1;
            }
        }
        woken
    }
}
