#![allow(unused_imports)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A simple recursive global kernel lock used by the simulation kernel.
///
/// `holder` records the logical owner id, and `depth` tracks recursive entries
/// by that same owner. The lock is fully released only when the depth reaches
/// zero.
pub struct KernLock {
    flag: AtomicBool,
    holder: AtomicUsize,
    depth: AtomicUsize,
}

impl KernLock {
    pub const fn new() -> Self {
        Self {
            flag: AtomicBool::new(false),
            holder: AtomicUsize::new(0),
            depth: AtomicUsize::new(0),
        }
    }

    pub fn enter(&self, id: usize) {
        if self.holder.load(Ordering::Relaxed) == id && id != 0 {
            self.depth.fetch_add(1, Ordering::Relaxed);
            return;
        }
        while self
            .flag
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        self.holder.store(id, Ordering::Relaxed);
        self.depth.store(1, Ordering::Relaxed);
    }

    pub fn leave(&self) {
        let d = self.depth.load(Ordering::Relaxed);
        if d > 1 {
            // Debug fix: recursive leave decrements depth instead of unlocking early.
            self.depth.store(d - 1, Ordering::Relaxed);
            return;
        }
        self.holder.store(0, Ordering::Relaxed);
        self.depth.store(0, Ordering::Relaxed);
        self.flag.store(false, Ordering::Release);
    }

    pub fn held(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    pub fn owner(&self) -> usize {
        self.holder.load(Ordering::Relaxed)
    }

    pub fn level(&self) -> usize {
        self.depth.load(Ordering::Relaxed)
    }

    pub fn try_enter(&self, id: usize) -> bool {
        if self.holder.load(Ordering::Relaxed) == id && id != 0 {
            self.depth.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        if self
            .flag
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            self.holder.store(id, Ordering::Relaxed);
            self.depth.store(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

unsafe impl Send for KernLock {}
unsafe impl Sync for KernLock {}

pub static GKL: KernLock = KernLock::new();

/// Minimal spin lock used by small simulated kernel objects.
///
/// The lock is a single atomic flag. `acquire` spins until it flips the flag
/// from unlocked to locked, while `release` stores the flag back to unlocked.
pub struct Spin {
    locked: AtomicBool,
}

impl Spin {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    pub fn acquire(&self) {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    pub fn try_acquire(&self) -> bool {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    pub fn release(&self) {
        self.locked.store(false, Ordering::Release);
    }

    pub fn is_held(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }
}

unsafe impl Send for Spin {}
unsafe impl Sync for Spin {}

/// Placeholder guard for flag/IRQ-state scoped regions in the simulation.
///
/// The current std-based kernel simulation does not model real interrupt flags,
/// so `enter` stores a dummy value and `drop` performs no restoration.
pub struct FlagGuard(usize);

impl FlagGuard {
    pub fn enter() -> Self {
        Self(0)
    }
}

impl Drop for FlagGuard {
    fn drop(&mut self) {}
}
