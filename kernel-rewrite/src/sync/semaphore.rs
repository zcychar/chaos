#![allow(unused_imports)]

use super::event_bus::{EventBus, EventFlag};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::thread;

/// Internal state for a counting semaphore.
///
/// `permit_count` is the available permit count, `pid` records the last associated task
/// id for SysV-style accounting, `removed` marks a removed semaphore, and `bus`
/// publishes acquire/remove events to observers.
struct SemaInner {
    permit_count: isize,
    pid: usize,
    removed: bool,
    bus: EventBus,
}

/// Shared counting semaphore used by the synchronization and IPC simulations.
pub struct Sema {
    inner: Arc<Mutex<SemaInner>>,
}

/// RAII permit guard returned by `Sema::access`.
///
/// Dropping the guard releases exactly one permit back to the semaphore.
pub struct SemaGuard<'a> {
    semaphore: &'a Sema,
}

impl Sema {
    pub fn new(initial_count: isize) -> Self {
        Sema {
            inner: Arc::new(Mutex::new(SemaInner {
                permit_count: initial_count,
                removed: false,
                pid: 0,
                bus: EventBus::default(),
            })),
        }
    }

    pub fn remove(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.removed = true;
        inner.bus.set(EventFlag::SEM_RM);
    }

    pub fn release(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.permit_count += 1;
        if inner.permit_count >= 1 {
            inner.bus.set(EventFlag::SEM_ACQ);
        }
    }

    pub fn try_acquire(&self) -> Result<bool, &'static str> {
        let mut inner = self.inner.lock().unwrap();
        if inner.removed {
            return Err("removed");
        }
        if inner.permit_count >= 1 {
            inner.permit_count -= 1;
            if inner.permit_count < 1 {
                inner.bus.clear(EventFlag::SEM_ACQ);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn acquire_spin(&self) -> Result<(), &'static str> {
        loop {
            match self.try_acquire()? {
                true => return Ok(()),
                false => thread::yield_now(),
            }
        }
    }

    pub fn access(&self) -> Result<SemaGuard<'_>, &'static str> {
        self.acquire_spin()?;
        Ok(SemaGuard { semaphore: self })
    }

    pub fn get_val(&self) -> isize {
        self.inner.lock().unwrap().permit_count
    }

    pub fn get_ncnt(&self) -> usize {
        self.inner.lock().unwrap().bus.callback_count()
    }

    pub fn get_pid(&self) -> usize {
        self.inner.lock().unwrap().pid
    }

    pub fn set_pid(&self, pid: usize) {
        self.inner.lock().unwrap().pid = pid;
    }

    // Debug fix: set_val must update the bus state to reflect the new permit count.
    pub fn set_val(&self, value: isize) {
        let mut inner = self.inner.lock().unwrap();
        inner.permit_count = value;
        if inner.permit_count >= 1 {
            inner.bus.set(EventFlag::SEM_ACQ);
        } else {
            inner.bus.clear(EventFlag::SEM_ACQ);
        }
    }
}

impl<'a> Drop for SemaGuard<'a> {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

impl<'a> Deref for SemaGuard<'a> {
    type Target = Sema;

    fn deref(&self) -> &Self::Target {
        self.semaphore
    }
}
