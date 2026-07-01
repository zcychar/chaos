#![allow(unused_imports)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

/// User data carried by an epoll event.
#[derive(Clone, Copy)]
pub struct EpData {
    pub ptr: u64,
}

/// Registered epoll interest mask and caller data.
#[derive(Clone)]
pub struct EpEvent {
    pub events: u32,
    pub data: EpData,
}

impl EpEvent {
    pub const IN: u32 = 0x001;
    pub const OUT: u32 = 0x004;
    pub const ERR: u32 = 0x008;
    pub const HUP: u32 = 0x010;
    pub const PRI: u32 = 0x002;
    pub const RDNORM: u32 = 0x040;
    pub const RDBAND: u32 = 0x080;
    pub const WRNORM: u32 = 0x100;
    pub const WRBAND: u32 = 0x200;
    pub const MSG: u32 = 0x400;
    pub const RDHUP: u32 = 0x2000;
    pub const EXCL: u32 = 1 << 28;
    pub const WAKEUP: u32 = 1 << 29;
    pub const ONESHOT: u32 = 1 << 30;
    pub const ET: u32 = 1 << 31;

    pub fn has(&self, event_mask: u32) -> bool {
        (self.events & event_mask) != 0
    }
}

/// epoll control operation constants.
pub struct EpCtlOp;

impl EpCtlOp {
    pub const ADD: i32 = 1;
    pub const DEL: i32 = 2;
    pub const MOD: i32 = 3;
}

/// Shared epoll registration table.
#[derive(Clone)]
pub struct EpEventMap {
    inner: Arc<Mutex<BTreeMap<usize, EpEvent>>>,
}

impl EpEventMap {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn insert(&self, fd: usize, event: EpEvent) -> Option<EpEvent> {
        self.inner.lock().unwrap().insert(fd, event)
    }

    pub fn contains_key(&self, fd: &usize) -> bool {
        self.inner.lock().unwrap().contains_key(fd)
    }

    pub fn remove(&self, fd: &usize) -> Option<EpEvent> {
        self.inner.lock().unwrap().remove(fd)
    }
}

/// Minimal epoll instance state.
///
/// `events` stores registrations, `ready` stores ready fds, and `new_ctl`
/// tracks fds whose control state changed since the last wait path observed it.
#[derive(Clone)]
pub struct EpInst {
    pub events: EpEventMap,
    pub ready: Arc<Mutex<BTreeSet<usize>>>,
    pub new_ctl: Arc<Mutex<BTreeSet<usize>>>,
}

impl EpInst {
    pub fn new() -> Self {
        EpInst {
            events: EpEventMap::new(),
            ready: Arc::new(Mutex::new(BTreeSet::new())),
            new_ctl: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    pub fn control(&self, op: i32, fd: usize, event: &EpEvent) -> Result<(), &'static str> {
        match op {
            EpCtlOp::ADD => {
                // Debug fix: ADD must reject an fd that is already registered.
                if self.events.contains_key(&fd) {
                    return Err("eexist");
                }
                self.events.insert(fd, event.clone());
                self.new_ctl.lock().unwrap().insert(fd);
                Ok(())
            }
            EpCtlOp::MOD => {
                if self.events.contains_key(&fd) {
                    self.events.insert(fd, event.clone());
                    self.new_ctl.lock().unwrap().insert(fd);
                    Ok(())
                } else {
                    Err("eperm")
                }
            }
            EpCtlOp::DEL => {
                if self.events.remove(&fd).is_some() {
                    // Debug fix: DEL must remove all stale state for the fd.
                    self.ready.lock().unwrap().remove(&fd);
                    self.new_ctl.lock().unwrap().remove(&fd);
                    Ok(())
                } else {
                    Err("eperm")
                }
            }
            _ => Err("eperm"),
        }
    }
}
