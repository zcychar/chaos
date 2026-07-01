#![allow(unused_imports)]

use crate::sync::Sema;
use crate::trap::CLK;
use std::collections::BTreeMap;
use std::ops::Index;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock, Weak};

/// SysV-style IPC permission metadata.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IpcPerm {
    // User supplied IPC lookup key.
    pub key: u32,
    // Current owner user and group ids.
    pub uid: u32,
    pub gid: u32,
    // Creator user and group ids.
    pub cuid: u32,
    pub cgid: u32,
    // Permission bits; only low mode bits are used by setters.
    pub mode: u32,
    // Sequence number for id reuse/generation tracking.
    pub seq: u32,
    // ABI padding fields.
    pub pad1: usize,
    pub pad2: usize,
}

/// SysV semaphore-set descriptor.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SemDs {
    // Permission and ownership metadata.
    pub perm: IpcPerm,
    // Last semaphore operation time.
    pub otime: usize,
    // ABI padding.
    _p1: usize,
    // Last metadata change time.
    pub ctime: usize,
    // ABI padding.
    _p2: usize,
    // Number of semaphores in this set.
    pub nsems: usize,
}

/// A semaphore array plus its descriptor metadata.
pub struct SemArr {
    pub ds: Mutex<SemDs>,
    pub sems: Vec<Sema>,
}

impl Index<usize> for SemArr {
    type Output = Sema;

    fn index(&self, index: usize) -> &Sema {
        &self.sems[index]
    }
}

impl SemArr {
    pub fn remove(&self) {
        for semaphore in &self.sems {
            semaphore.remove();
        }
    }

    //Debug fix: the original code does not update otime and ctime.
    pub fn otime_now(&self) {
        self.ds.lock().unwrap().otime = CLK.load(Ordering::Relaxed);
    }

    pub fn ctime_now(&self) {
        self.ds.lock().unwrap().ctime = CLK.load(Ordering::Relaxed);
    }

    pub fn set_ds(&self, new_descriptor: &SemDs) {
        let mut descriptor = self.ds.lock().unwrap();
        descriptor.perm.uid = new_descriptor.perm.uid;
        descriptor.perm.gid = new_descriptor.perm.gid;
        descriptor.perm.mode = new_descriptor.perm.mode & 0x1ff;
    }

    // Note: the weak reference design need further review.
    pub fn get_or_create(
        key: u32,
        nsems: usize,
        flags: usize,
        store: &RwLock<BTreeMap<u32, Weak<SemArr>>>,
    ) -> Result<Arc<Self>, &'static str> {
        // Debug fix: a semaphore array must contain at least one semaphore.
        if nsems == 0 {
            return Err("einval");
        }

        let mut map = store.write().unwrap();
        let mut resolved_key = key;
        if resolved_key == 0 {
            resolved_key = (1u32..)
                .find(|candidate_key| map.get(candidate_key).is_none())
                .unwrap();
        } else if let Some(weak_array) = map.get(&resolved_key) {
            if let Some(array) = weak_array.upgrade() {
                if (flags & (1 << 9)) != 0 && (flags & (1 << 10)) != 0 {
                    return Err("eexist");
                }
                // Debug fix: an existing array must satisfy the requested size.
                if array.ds.lock().unwrap().nsems < nsems {
                    return Err("einval");
                }
                return Ok(array);
            }
        }

        let semaphores = Vec::with_capacity(nsems);

        let array = Arc::new(SemArr {
            ds: Mutex::new(SemDs {
                perm: IpcPerm {
                    key: resolved_key,
                    uid: 0,
                    gid: 0,
                    cuid: 0,
                    cgid: 0,
                    mode: (flags as u32) & 0x1ff,
                    seq: 0,
                    pad1: 0,
                    pad2: 0,
                },
                otime: 0,
                _p1: 0,
                ctime: 0,
                _p2: 0,
                nsems,
            }),
            sems: semaphores,
        });
        map.insert(resolved_key, Arc::downgrade(&array));
        Ok(array)
    }
}

type SemId = usize;
type SemNum = u16;
type SemOp = i16;

/// Per-task semaphore context.
///
/// `arrays` maps local semaphore ids to semaphore arrays. `undos` stores
/// SEM_UNDO-style adjustments keyed by `(semaphore id, semaphore number)`.
#[derive(Default)]
pub struct SemCtx {
    pub arrays: BTreeMap<SemId, Arc<SemArr>>,
    pub undos: BTreeMap<(SemId, SemNum), SemOp>,
}

impl SemCtx {
    pub fn add(&mut self, array: Arc<SemArr>) -> SemId {
        let id = self.free_id();
        self.arrays.insert(id, array);
        id
    }

    pub fn remove(&mut self, id: SemId) {
        self.arrays.remove(&id);
        // Debug fix: removing an array id must also clear stale undo records for it.
        self.undos.retain(|(undo_id, _), _| *undo_id != id);
    }

    fn free_id(&self) -> SemId {
        (0..)
            .find(|candidate_id| self.arrays.get(candidate_id).is_none())
            .unwrap()
    }

    pub fn get(&self, id: SemId) -> Option<Arc<SemArr>> {
        self.arrays.get(&id).cloned()
    }

    pub fn add_undo(&mut self, id: SemId, sem_num: SemNum, op: SemOp) {
        let old = *self.undos.get(&(id, sem_num)).unwrap_or(&0);
        self.undos.insert((id, sem_num), old - op);
    }
}

impl Clone for SemCtx {
    fn clone(&self) -> Self {
        SemCtx {
            arrays: self.arrays.clone(),
            undos: BTreeMap::new(),
        }
    }
}

impl Drop for SemCtx {
    fn drop(&mut self) {
        for (&(id, sem_num), &op) in &self.undos {
            if let Some(array) = self.arrays.get(&id) {
                let semaphore = &array[sem_num as usize];
                if op > 0 {
                    // Debug fix: replay the full positive undo magnitude, not only op == 1.
                    for _ in 0..op as usize {
                        semaphore.release();
                    }
                } else if op < 0 {
                    for _ in 0..(-op) as usize {
                        let _ = semaphore.try_acquire();
                    }
                }
            }
        }
    }
}

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
