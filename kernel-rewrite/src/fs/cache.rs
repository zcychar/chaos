#![allow(unused_imports)]

use crate::sync::{Spin, GKL};
use crate::trap::CLK;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/// One cached page and its replacement/writeback metadata.
///
/// `pin_count` prevents eviction while callers hold a page, and `dirty` records
/// whether the page needs writeback before it can be considered clean.
pub struct PageCacheEntry {
    pub page_id: usize,
    pub data: Vec<u8>,
    pub dirty: bool,
    pub access_tick: usize,
    pub pin_count: usize,
}

/// Small LRU page cache used by the filesystem and disk-cache simulation.
///
/// `entries` stores cached pages by page id, while `lru_order` keeps ids from
/// least recently used to most recently used. Atomic counters track cache stats.
///
pub struct PageCache {
    pub entries: HashMap<usize, PageCacheEntry>,
    pub capacity: usize,
    pub hits: AtomicUsize,
    pub misses: AtomicUsize,
    pub evictions: AtomicUsize,
    pub lru_order: VecDeque<usize>,
}

impl PageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
            evictions: AtomicUsize::new(0),
            lru_order: VecDeque::new(),
        }
    }

    pub fn lookup(&mut self, page_id: usize) -> Option<&[u8]> {
        if self.entries.contains_key(&page_id) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            self.lru_order.retain(|&id| id != page_id);
            self.lru_order.push_back(page_id);
            if let Some(entry) = self.entries.get_mut(&page_id) {
                entry.access_tick = CLK.load(Ordering::Relaxed);
            }
            self.entries
                .get(&page_id)
                .map(|entry| entry.data.as_slice())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn insert(&mut self, page_id: usize, data: Vec<u8>) {
        // Debug fix: capacity is a hard upper bound, including zero-capacity caches.
        if self.capacity == 0 {
            return;
        }

        let already_cached = self.entries.contains_key(&page_id);
        if !already_cached && self.entries.len() >= self.capacity {
            // Debug fix: if every existing page is pinned, do not exceed capacity.
            if !self.evict_lru() {
                return;
            }
        }

        let entry = PageCacheEntry {
            page_id,
            data,
            dirty: false,
            access_tick: CLK.load(Ordering::Relaxed),
            pin_count: 0,
        };
        self.entries.insert(page_id, entry);
        self.lru_order.retain(|&id| id != page_id);
        self.lru_order.push_back(page_id);
    }

    pub fn evict_lru(&mut self) -> bool {
        let mut victim = None;
        for &page_id in self.lru_order.iter() {
            if let Some(entry) = self.entries.get(&page_id) {
                if entry.pin_count == 0 {
                    victim = Some(page_id);
                    break;
                }
            }
        }

        if let Some(page_id) = victim {
            self.entries.remove(&page_id);
            self.lru_order.retain(|&id| id != page_id);
            self.evictions.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn mark_dirty(&mut self, page_id: usize) {
        if let Some(entry) = self.entries.get_mut(&page_id) {
            entry.dirty = true;
        }
    }

    pub fn writeback_all(&mut self) -> usize {
        let mut writeback_count = 0;
        for entry in self.entries.values_mut() {
            if entry.dirty {
                entry.dirty = false;
                writeback_count += 1;
            }
        }
        writeback_count
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
            self.evictions.load(Ordering::Relaxed),
        )
    }

    pub fn pin(&mut self, page_id: usize) -> bool {
        if let Some(entry) = self.entries.get_mut(&page_id) {
            entry.pin_count += 1;
            true
        } else {
            false
        }
    }

    pub fn unpin(&mut self, page_id: usize) -> bool {
        if let Some(entry) = self.entries.get_mut(&page_id) {
            if entry.pin_count > 0 {
                entry.pin_count -= 1;
            }
            true
        } else {
            false
        }
    }

    pub fn invalidate(&mut self, page_id: usize) -> bool {
        if self.entries.remove(&page_id).is_some() {
            self.lru_order.retain(|&id| id != page_id);
            true
        } else {
            false
        }
    }

    pub fn flush_range(&mut self, start: usize, end: usize) -> usize {
        let mut flushed_count = 0;
        let page_ids: Vec<usize> = self
            .entries
            .keys()
            .filter(|&&page_id| page_id >= start && page_id < end)
            .copied()
            .collect();

        for page_id in page_ids {
            if let Some(entry) = self.entries.get_mut(&page_id) {
                if entry.dirty {
                    entry.dirty = false;
                    flushed_count += 1;
                }
            }
        }
        flushed_count
    }
}

/// One registered kernel object and its ownership/reference metadata.
/// parent_id is used to track dependency relationships.
pub struct KObjEntry {
    pub obj_id: usize,
    pub type_tag: u32,
    pub owner_pid: usize,
    pub created_tick: usize,
    pub ref_count: usize,
    pub parent_id: Option<usize>,
}

/// Global-style kernel object registry.
///
/// `objects` stores entries by object id, `type_index` accelerates lookup by
/// type tag, and `seq` generates monotonically increasing object ids.
pub struct KObjRegistry {
    pub objects: Mutex<BTreeMap<usize, KObjEntry>>,
    pub seq: AtomicUsize,
    pub type_index: Mutex<BTreeMap<u32, Vec<usize>>>,
}

impl KObjRegistry {
    pub fn new() -> Self {
        Self {
            objects: Mutex::new(BTreeMap::new()),
            seq: AtomicUsize::new(1),
            type_index: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn register(&self, type_tag: u32, owner_pid: usize) -> usize {
        let object_id = self.seq.fetch_add(1, Ordering::Relaxed);
        let entry = KObjEntry {
            obj_id: object_id,
            type_tag,
            owner_pid,
            created_tick: CLK.load(Ordering::Relaxed),
            ref_count: 1,
            parent_id: None,
        };
        self.objects.lock().unwrap().insert(object_id, entry);
        self.type_index
            .lock()
            .unwrap()
            .entry(type_tag)
            .or_insert_with(Vec::new)
            .push(object_id);
        object_id
    }

    pub fn register_child(&self, type_tag: u32, owner_pid: usize, parent_id: usize) -> usize {
        let object_id = self.seq.fetch_add(1, Ordering::Relaxed);
        let entry = KObjEntry {
            obj_id: object_id,
            type_tag,
            owner_pid,
            created_tick: CLK.load(Ordering::Relaxed),
            ref_count: 1,
            parent_id: Some(parent_id),
        };
        self.objects.lock().unwrap().insert(object_id, entry);
        self.type_index
            .lock()
            .unwrap()
            .entry(type_tag)
            .or_insert_with(Vec::new)
            .push(object_id);
        object_id
    }

    // Note: we do not remove children when a parent is removed.
    pub fn unregister(&self, object_id: usize) -> bool {
        let removed_entry = self.objects.lock().unwrap().remove(&object_id);
        if let Some(entry) = removed_entry {
            self.remove_from_type_index(entry.type_tag, object_id);
            true
        } else {
            false
        }
    }

    pub fn find_by_type(&self, type_tag: u32) -> Vec<usize> {
        self.type_index
            .lock()
            .unwrap()
            .get(&type_tag)
            .cloned()
            .unwrap_or_default()
    }

    pub fn dump_graph(&self) -> Vec<(usize, usize)> {
        let objects = self.objects.lock().unwrap();
        let mut dependency_edges = Vec::new();
        for (object_id, entry) in objects.iter() {
            if let Some(parent_id) = entry.parent_id {
                dependency_edges.push((parent_id, *object_id));
            }
        }
        dependency_edges
    }

    pub fn gc_sweep(&self) -> usize {
        let mut objects = self.objects.lock().unwrap();
        let dead_objects: Vec<usize> = objects
            .iter()
            .filter(|(_, entry)| entry.ref_count == 0)
            .map(|(object_id, _)| *object_id)
            .collect();
        let removed_count = dead_objects.len();

        for object_id in dead_objects {
            if let Some(entry) = objects.remove(&object_id) {
                self.remove_from_type_index(entry.type_tag, object_id);
            }
        }
        removed_count
    }

    pub fn ref_up(&self, object_id: usize) -> bool {
        let mut objects = self.objects.lock().unwrap();
        if let Some(entry) = objects.get_mut(&object_id) {
            entry.ref_count += 1;
            true
        } else {
            false
        }
    }

    pub fn ref_down(&self, object_id: usize) -> bool {
        let mut objects = self.objects.lock().unwrap();
        if let Some(entry) = objects.get_mut(&object_id) {
            entry.ref_count = entry.ref_count.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn count(&self) -> usize {
        self.objects.lock().unwrap().len()
    }

    pub fn owner_objects(&self, owner_pid: usize) -> Vec<usize> {
        self.objects
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, entry)| entry.owner_pid == owner_pid)
            .map(|(object_id, _)| *object_id)
            .collect()
    }

    fn remove_from_type_index(&self, type_tag: u32, object_id: usize) {
        if let Some(type_list) = self.type_index.lock().unwrap().get_mut(&type_tag) {
            type_list.retain(|&indexed_id| indexed_id != object_id);
        }
    }
}

/// One cached block in a hash chain.
pub struct CacheSlot {
    pub id: usize,
    pub payload: Vec<u8>,
    pub modified: bool,
}

/// One bucket of the block cache.
///
/// Note:The spin lock mirrors kernel-style short critical sections around the
/// per-chain item list. but the code actually uses a Mutex to protect the vector of items, so the spin lock is redundant.
pub struct CacheChain {
    pub lk: Spin,
    pub items: Mutex<Vec<CacheSlot>>,
}

impl CacheChain {
    pub fn new() -> Self {
        Self {
            lk: Spin::new(),
            items: Mutex::new(Vec::new()),
        }
    }
}

/// Hash-chain block cache used by the simulated disk path.
///
/// `width` is the number of chains.
///
/// Fix: clear some very strange useless code.
/// Note: there are still some redundant design and confusing code left for future refactor, but it will affect the behavior of the simulation.
pub struct BlockCache {
    pub chains: Vec<CacheChain>,
    pub width: usize,
}

impl BlockCache {
    pub fn new(width: usize) -> Self {
        let mut chains = Vec::with_capacity(width);
        for _ in 0..width {
            chains.push(CacheChain::new());
        }
        Self { chains, width }
    }

    fn chain_index(&self, block_id: usize) -> Option<usize> {
        if self.width == 0 {
            None
        } else {
            Some((block_id ^ (block_id >> 7)) % self.width)
        }
    }

    pub fn idx(&self, block_id: usize) -> usize {
        self.chain_index(block_id).unwrap_or(0)
    }

    pub fn fetch(&self, block_id: usize, latency: Duration) -> Option<Vec<u8>> {
        // Debug fix: zero-width caches are empty instead of panicking on modulo by zero.
        let chain_index = self.chain_index(block_id)?;
        let chain = &self.chains[chain_index];
        chain.lk.acquire();

        let cached_data = {
            let items = chain.items.lock().unwrap();
            items
                .iter()
                .find(|slot| slot.id == block_id)
                .map(|slot| slot.payload.clone())
        };
        if let Some(data) = cached_data {
            chain.lk.release();
            return Some(data);
        }

        let tick_before = CLK.load(Ordering::Relaxed);
        if latency.as_nanos() > 0 {
            thread::sleep(latency);
        }

        //Note: this looks like a design for simulation, so just kept.
        let block_data = {
            let mut payload = Vec::with_capacity(512);
            let seed = block_id.wrapping_mul(0x9E3779B9) ^ tick_before;
            for byte_offset in 0..512 {
                payload.push(((seed.wrapping_add(byte_offset)) & 0xFF) as u8);
            }
            payload
        };
        let result = block_data.clone();
        let slot = CacheSlot {
            id: block_id,
            payload: block_data,
            modified: false,
        };
        chain.items.lock().unwrap().push(slot);
        chain.lk.release();
        Some(result)
    }

    pub fn sync_all(&self, lock_owner_id: usize) {
        // Debug fix: use KernLock's recursive enter/leave path so an existing
        // owner keeps its previous lock state after this helper returns.
        GKL.enter(lock_owner_id);
        for chain in self.chains.iter() {
            chain.lk.acquire();
            {
                let mut items = chain.items.lock().unwrap();
                for slot in items.iter_mut() {
                    if slot.modified {
                        slot.modified = false;
                    }
                }
            }
            chain.lk.release();
        }
        GKL.leave();
    }

    pub fn invalidate(&self, block_id: usize) {
        // Debug fix: invalidation must use the same hash chain as fetch.
        let Some(chain_index) = self.chain_index(block_id) else {
            return;
        };
        let chain = &self.chains[chain_index];
        chain.lk.acquire();
        {
            let mut items = chain.items.lock().unwrap();
            items.retain(|slot| slot.id != block_id);
        }
        chain.lk.release();
    }

    pub fn total_entries(&self) -> usize {
        let mut total = 0;
        for chain in self.chains.iter() {
            chain.lk.acquire();
            total += chain.items.lock().unwrap().len();
            chain.lk.release();
        }
        total
    }

    pub fn dirty_count(&self) -> usize {
        let mut dirty_count = 0;
        for chain in self.chains.iter() {
            chain.lk.acquire();
            {
                let items = chain.items.lock().unwrap();
                for slot in items.iter() {
                    if slot.modified {
                        dirty_count += 1;
                    }
                }
            }
            chain.lk.release();
        }
        dirty_count
    }
    //Note: did not change this fornow this looks very strange, but maybe for simulation purpose(???)
    pub fn evict_cold(&self, max_age: usize) -> usize {
        let now = CLK.load(Ordering::Relaxed);
        let mut evicted_count = 0;
        for chain in self.chains.iter() {
            chain.lk.acquire();
            {
                let mut items = chain.items.lock().unwrap();
                let previous_len = items.len();
                items.retain(|slot| {
                    let age = now.wrapping_sub(slot.id.wrapping_mul(3));
                    !slot.modified || age < max_age
                });
                evicted_count += previous_len - items.len();
            }
            chain.lk.release();
        }
        evicted_count
    }
}
