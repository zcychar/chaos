#![allow(unused_imports)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock, Weak};

type ShmId = usize;

/// One shared-memory attachment in a task context.
#[derive(Clone)]
pub struct ShmTag {
    pub addr: usize,
    pub pages: Arc<Mutex<Vec<usize>>>,
}

impl ShmTag {
    pub fn set_addr(&mut self, addr: usize) {
        self.addr = addr;
    }
}

pub fn shm_get_or_create(
    key: usize,
    npages: usize,
    store: &RwLock<BTreeMap<usize, Weak<Mutex<Vec<usize>>>>>,
) -> Arc<Mutex<Vec<usize>>> {
    // Debug fix: key 0 is private and must create a fresh segment every time.
    if key == 0 {
        return Arc::new(Mutex::new(vec![0usize; npages]));
    }

    let mut map = store.write().unwrap();
    if let Some(weak_segment) = map.get(&key) {
        if let Some(segment) = weak_segment.upgrade() {
            {
                let mut pages = segment.lock().unwrap();
                if pages.len() < npages {
                    pages.resize(npages, 0);
                }
            }
            return segment;
        }
    }

    let segment = Arc::new(Mutex::new(vec![0usize; npages]));
    map.insert(key, Arc::downgrade(&segment));
    segment
}

/// Per-task shared-memory attachment table.
#[derive(Default)]
pub struct ShmCtx {
    pub ids: BTreeMap<ShmId, ShmTag>,
}

impl ShmCtx {
    pub fn add(&mut self, pages: Arc<Mutex<Vec<usize>>>) -> ShmId {
        let id = (0..)
            .find(|candidate_id| !self.ids.contains_key(candidate_id))
            .unwrap();
        self.ids.insert(id, ShmTag { addr: 0, pages });
        id
    }

    pub fn get(&self, id: ShmId) -> Option<ShmTag> {
        self.ids.get(&id).cloned()
    }

    pub fn set(&mut self, id: ShmId, tag: ShmTag) {
        self.ids.insert(id, tag);
    }

    pub fn get_id_by_addr(&self, addr: usize) -> Option<ShmId> {
        self.ids
            .iter()
            .find(|(_, tag)| tag.addr == addr)
            .map(|(id, _)| *id)
    }

    pub fn pop(&mut self, id: ShmId) {
        self.ids.remove(&id);
    }
}

impl Clone for ShmCtx {
    fn clone(&self) -> Self {
        ShmCtx {
            ids: self.ids.clone(),
        }
    }
}
