#![allow(unused_imports)]

use crate::consts::*;
use crate::sync::GKL;
use crate::trap::CLK;
use std::cmp::min;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Tracks permitted, effective, and ambient capability bitsets.
///
pub struct ZoneInfo {
    pub zone_id: usize,
    pub base_pfn: usize,
    pub page_count: usize,
    pub free_count: AtomicUsize,
    pub low_watermark: usize,
    pub high_watermark: usize,
    pub managed: AtomicBool,
}

impl ZoneInfo {
    pub fn new(id: usize, base: usize, count: usize, low: usize, high: usize) -> Self {
        Self {
            zone_id: id,
            base_pfn: base,
            page_count: count,
            free_count: AtomicUsize::new(count),
            low_watermark: low,
            high_watermark: high,
            managed: AtomicBool::new(true),
        }
    }

    pub fn zone_can_alloc(&self) -> bool {
        self.free_count.load(Ordering::Relaxed) > self.low_watermark
    }

    pub fn zone_pressure(&self) -> usize {
        let free = self.free_count.load(Ordering::Relaxed);
        if free >= self.high_watermark {
            return 0;
        }
        if free <= self.low_watermark {
            return 100;
        }
        let range = self.high_watermark - self.low_watermark;
        let deficit = self.high_watermark - free;
        (deficit * 100) / range
    }

    pub fn reclaim_target(&self) -> usize {
        let free = self.free_count.load(Ordering::Relaxed);
        if free >= self.high_watermark {
            return 0;
        }
        self.high_watermark - free
    }

    pub fn contains_pfn(&self, pfn: usize) -> bool {
        pfn >= self.base_pfn && pfn < self.base_pfn + self.page_count
    }
}

/// Reference counter for one physical page frame.
///
/// The counter is atomic because simulated VM and copy-on-write paths may share
/// frame metadata across tasks.
pub struct PgFrame {
    pub ref_count: AtomicUsize,
}

impl PgFrame {
    pub fn new() -> Self {
        Self {
            ref_count: AtomicUsize::new(0),
        }
    }

    pub fn with_rc(initial_count: usize) -> Self {
        Self {
            ref_count: AtomicUsize::new(initial_count),
        }
    }

    pub fn up(&self) -> usize {
        self.ref_count.fetch_add(1, Ordering::Relaxed)
    }

    pub fn down(&self) -> usize {
        loop {
            let current_count = self.ref_count.load(Ordering::Relaxed);
            if current_count == 0 {
                return 0;
            }
            // Debug fix: decrement with compare-exchange so zero never wraps to usize::MAX.
            if self
                .ref_count
                .compare_exchange_weak(
                    current_count,
                    current_count - 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return current_count;
            }
        }
    }

    pub fn count(&self) -> usize {
        self.ref_count.load(Ordering::Relaxed)
    }

    pub fn set(&self, count: usize) {
        self.ref_count.store(count, Ordering::Relaxed);
    }

    pub fn cas(&self, expected: usize, desired: usize) -> bool {
        self.ref_count
            .compare_exchange(expected, desired, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }

    pub fn inc_if_nonzero(&self) -> bool {
        loop {
            let current_count = self.ref_count.load(Ordering::Relaxed);
            if current_count == 0 {
                return false;
            }
            if self
                .ref_count
                .compare_exchange_weak(
                    current_count,
                    current_count + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return true;
            }
        }
    }
}

pub struct FramePool {
    slots: Mutex<Vec<bool>>,
    pub(crate) capacity: usize,
}

impl FramePool {
    pub fn new(frame_count: usize) -> Self {
        Self {
            slots: Mutex::new(vec![true; frame_count]),
            capacity: frame_count,
        }
    }

    pub fn get(&self, id: usize) -> Option<usize> {
        let already_locked = GKL.held();
        if !already_locked {
            GKL.enter(id);
        }
        let frame = self.get_inner();
        if !already_locked {
            GKL.leave();
        }
        frame
    }

    pub fn get_inner(&self) -> Option<usize> {
        let mut slots = self.slots.lock().unwrap();
        for (frame_index, is_free) in slots.iter_mut().enumerate() {
            if *is_free {
                *is_free = false;
                return Some(frame_index);
            }
        }
        None
    }

    pub fn get_contig(&self, size: usize, align_log2: usize) -> Option<usize> {
        // Debug fix: validate the shift width before computing `1usize << align_log2`.
        if size == 0 || align_log2 >= usize::BITS as usize {
            return None;
        }
        let mut slots = self.slots.lock().unwrap();
        let alignment = 1usize << align_log2;
        for start in (0..slots.len()).step_by(alignment) {
            let end = match start.checked_add(size) {
                Some(end) => end,
                None => break,
            };
            if end > slots.len() {
                break;
            }
            if (start..end).all(|frame_index| slots[frame_index]) {
                for frame_index in start..end {
                    slots[frame_index] = false;
                }
                return Some(start);
            }
        }
        None
    }

    pub fn put(&self, frame_index: usize) {
        let mut slots = self.slots.lock().unwrap();
        if frame_index < slots.len() {
            slots[frame_index] = true;
        }
    }

    pub fn avail(&self, frame_index: usize) -> bool {
        let slots = self.slots.lock().unwrap();
        frame_index < slots.len() && slots[frame_index]
    }

    pub fn free_count(&self) -> usize {
        self.slots
            .lock()
            .unwrap()
            .iter()
            .filter(|&&is_free| is_free)
            .count()
    }

    //Note: these two functions are used for zone-aware allocation, not unified with the others.
    pub fn get_zone_aware(&self, zone: &ZoneInfo) -> Option<usize> {
        if !zone.zone_can_alloc() {
            return None;
        }
        let mut slots = self.slots.lock().unwrap();
        let base = zone.base_pfn;
        let limit = base + zone.page_count;
        for frame_index in base..min(limit, slots.len()) {
            if slots[frame_index] {
                slots[frame_index] = false;
                zone.free_count.fetch_sub(1, Ordering::Relaxed);
                return Some(frame_index);
            }
        }
        None
    }

    pub fn put_zone_aware(&self, frame_index: usize, zone: &ZoneInfo) {
        let mut slots = self.slots.lock().unwrap();
        if frame_index < slots.len() {
            slots[frame_index] = true;
            zone.free_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn batch_alloc(&self, count: usize) -> Vec<usize> {
        let mut slots = self.slots.lock().unwrap();
        let mut allocated_frames = Vec::with_capacity(count);
        for (frame_index, is_free) in slots.iter_mut().enumerate() {
            if allocated_frames.len() >= count {
                break;
            }
            if *is_free {
                *is_free = false;
                allocated_frames.push(frame_index);
            }
        }
        allocated_frames
    }
}

/// Allocates one physical frame and returns its simulated physical address.
pub fn frame_alloc(pool: &FramePool) -> Option<usize> {
    let frame_index = {
        let mut slots = pool.slots.lock().unwrap();
        let scan_start = CLK.load(Ordering::Relaxed) % slots.len().max(1);
        let mut allocated = None;

        for offset in 0..slots.len() {
            let frame_index = (scan_start + offset) % slots.len();
            if slots[frame_index] {
                slots[frame_index] = false;
                allocated = Some(frame_index);
                break;
            }
        }
        allocated
    }?;

    frame_index
        .checked_mul(PAGE_SZ)
        .and_then(|offset| MEM_OFF.checked_add(offset))
}

/// Frees a frame previously returned by `frame_alloc` or `frame_alloc_contig`.
pub fn frame_dealloc(pool: &FramePool, physical_addr: usize) {
    if physical_addr < MEM_OFF {
        return;
    }

    let offset = physical_addr - MEM_OFF;
    if offset % PAGE_SZ != 0 {
        return;
    }

    let frame_index = offset / PAGE_SZ;
    pool.put(frame_index);
}

/// Allocates `frame_count` contiguous frames with `align_log2` frame alignment.
pub fn frame_alloc_contig(
    pool: &FramePool,
    frame_count: usize,
    align_log2: usize,
) -> Option<usize> {
    let start_frame = pool.get_contig(frame_count, align_log2)?;
    start_frame
        .checked_mul(PAGE_SZ)
        .and_then(|offset| MEM_OFF.checked_add(offset))
}

/// Reports the number of free slots in the simulated frame bitmap.
/// Note: name is not correct at all, we just keep useful parts of the original code for now.
pub fn defragment_frame_pool(slots: &mut Vec<bool>) -> usize {
    slots.iter().filter(|&&is_free| is_free).count()
}

/// Checks whether `addr` is aligned to `PAGE_SZ << order`.
/// Note: simplified (alot)
pub fn verify_page_alignment(addr: usize, order: usize) -> bool {
    if order >= 12 || addr >= KERN_BASE {
        return false;
    }

    let alignment = PAGE_SZ << order;
    let mask = alignment - 1;
    (addr & mask) == 0 && addr.checked_add(alignment).is_some()
}
