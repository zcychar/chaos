#![allow(unused_imports)]

pub mod frame;
pub mod vm;

pub use frame::*;
pub use vm::*;

use crate::consts::*;
use crate::sync::GKL;
use crate::trap::CLK;
use crate::util::log2_floor;
use std::cmp::min;
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

/// One slab allocator chunk for fixed-size objects.
///
/// `data` stores all object bytes, `obj_size` is the aligned size of each slot,
/// `free_list` stores free slot offsets inside `data`, and `allocated` tracks
/// how many slots are currently checked out.
pub struct SlabEntry {
    pub data: Vec<u8>,
    pub obj_size: usize,
    pub capacity: usize,
    pub free_list: VecDeque<usize>,
    pub allocated: usize,
    pub tag: u32,
}

impl SlabEntry {
    fn empty() -> Self {
        Self {
            data: Vec::new(),
            obj_size: 0,
            capacity: 0,
            free_list: VecDeque::new(),
            allocated: 0,
            tag: 0,
        }
    }

    fn aligned_object_size(obj_size: usize) -> Option<usize> {
        obj_size
            .checked_add(SLAB_ALIGN - 1)
            .map(|size| size & !(SLAB_ALIGN - 1))
    }

    fn slot_range(&self, offset: usize) -> Option<std::ops::Range<usize>> {
        if self.obj_size == 0 {
            return None;
        }
        let end = offset.checked_add(self.obj_size)?;
        if end <= self.data.len() {
            Some(offset..end)
        } else {
            None
        }
    }

    fn is_valid_slot_offset(&self, offset: usize) -> bool {
        if self.obj_size == 0 {
            return false;
        }
        offset % self.obj_size == 0 && self.slot_range(offset).is_some()
    }

    fn is_free_slot(&self, offset: usize) -> bool {
        self.free_list.iter().any(|&slot| slot == offset)
    }

    pub fn new(obj_size: usize, capacity: usize) -> Self {
        let Some(aligned) = Self::aligned_object_size(obj_size) else {
            return Self::empty();
        };
        let Some(total) = aligned.checked_mul(capacity) else {
            return Self::empty();
        };
        let mut free_offsets = VecDeque::with_capacity(capacity);
        for i in 0..capacity {
            free_offsets.push_back(i * aligned);
        }
        Self {
            data: vec![0u8; total],
            obj_size: aligned,
            capacity,
            free_list: free_offsets,
            allocated: 0,
            tag: 0,
        }
    }

    pub fn slab_alloc(&mut self, zeroed: bool) -> Option<usize> {
        let slot = self.free_list.pop_front()?;
        let slot_range = self.slot_range(slot)?;
        if zeroed {
            // Debug fix: zero the slot only when the caller requested zeroed allocation.
            let region = &mut self.data[slot_range];
            region.fill(0);
        }
        self.allocated += 1;
        Some(slot)
    }

    pub fn slab_free(&mut self, offset: usize) {
        if !self.is_valid_slot_offset(offset) {
            return;
        }
        // Debug fix: reject duplicate frees before returning the slot to free_list.
        if self.is_free_slot(offset) {
            return;
        }
        self.free_list.push_back(offset);
        if self.allocated > 0 {
            self.allocated -= 1;
        }
    }

    pub fn slab_used(&self) -> usize {
        self.allocated
    }

    pub fn slab_avail(&self) -> usize {
        self.free_list.len()
    }

    // Note: this is useless for now.
    pub fn shrink(&mut self) -> usize {
        let before = self.data.len();
        if self.allocated == 0 {
            self.data.clear();
            self.free_list.clear();
        }
        before - self.data.len()
    }

    pub fn obj_at(&self, offset: usize) -> Option<&[u8]> {
        let slot_range = self.slot_range(offset)?;
        Some(&self.data[slot_range])
    }

    pub fn obj_at_mut(&mut self, offset: usize) -> Option<&mut [u8]> {
        let slot_range = self.slot_range(offset)?;
        Some(&mut self.data[slot_range])
    }
}

/// TCP-like socket lifecycle states used by the network/socket simulation.
///
pub fn p2v(physical_addr: usize) -> usize {
    PHYS_OFF.wrapping_add(physical_addr)
}

/// Converts a direct-map kernel virtual address back to a physical address.
pub fn v2p(virtual_addr: usize) -> usize {
    virtual_addr.wrapping_sub(PHYS_OFF)
}

/// Returns the byte offset of a kernel virtual address from `KERN_BASE`.
pub fn k_off(virtual_addr: usize) -> usize {
    virtual_addr.wrapping_sub(KERN_BASE)
}

/// Copy-on-write page state for one shared mapping.
///
/// `frame` stores the current frame id, `w` records whether this mapping has
/// become writable, and `pending` means a write fault still needs to allocate a
/// private frame.
pub struct SharedPage {
    pub frame: AtomicUsize,
    pub writable: AtomicBool,
    pub pending: AtomicBool,
}

impl SharedPage {
    pub fn new(frame_id: usize) -> Self {
        Self {
            frame: AtomicUsize::new(frame_id),
            writable: AtomicBool::new(false),
            pending: AtomicBool::new(true),
        }
    }

    pub fn fault(&self, pool: &FramePool, source_frame: &PgFrame) -> Result<usize, &'static str> {
        let is_pending = self.pending.load(Ordering::Relaxed);
        let current_frame = self.frame.load(Ordering::Relaxed);
        if !is_pending {
            return Ok(current_frame);
        }
        // Debug fix: useless inline
        let new_frame = pool.get_inner().ok_or("oom")?;
        self.frame.store(new_frame, Ordering::Relaxed);
        // Debug fix: route the CoW source decrement through PgFrame::down to avoid underflow.
        // Note(IMPORTANT): currently the frame system is not unified, actually we cannot get the PgFrame from the frame id, but this requires too much refactoring.
        source_frame.down();
        self.writable.store(true, Ordering::Relaxed);
        self.pending.store(false, Ordering::Relaxed);
        Ok(new_frame)
    }

    pub fn is_cow_resolved(&self) -> bool {
        !self.pending.load(Ordering::Relaxed) && self.writable.load(Ordering::Relaxed)
    }

    pub fn frame_id(&self) -> usize {
        self.frame.load(Ordering::Relaxed)
    }
}

/// Heap-backed kernel stack buffer used by simulated tasks.
///
/// The tuple field stores the base pointer of a boxed byte slice. `top` returns
/// the conventional downward-growing stack top address.
pub struct KStk(usize);

impl KStk {
    pub fn new() -> Self {
        let stack = vec![0u8; KSTK_SZ].into_boxed_slice();
        let base_ptr = Box::into_raw(stack) as *mut u8 as usize;
        KStk(base_ptr)
    }

    pub fn top(&self) -> usize {
        self.0 + KSTK_SZ
    }
}

impl Drop for KStk {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: self.0 was created from a Box<[u8]> of exactly KSTK_SZ bytes in new().
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(self.0 as *mut u8, KSTK_SZ));
        }
    }
}

/// Aligns a heap span and returns the exclusive end address of the usable heap.
pub fn heap_init(base: usize, size: usize) -> usize {
    let aligned_base = match base.checked_add(PAGE_SZ - 1) {
        Some(value) => value & !(PAGE_SZ - 1),
        None => return 0,
    };
    let aligned_size = size & !(PAGE_SZ - 1);
    aligned_base.checked_add(aligned_size).unwrap_or(0)
}

/// Grows the simulated heap by allocating up to `page_count` pages.
///
/// Fix: useless inlines and do not allow adding span infront of the last span(???)
pub fn heap_grow(pool: &FramePool, page_count: usize) -> Vec<(usize, usize)> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for frame_index in pool.batch_alloc(page_count.min(pool.capacity)) {
        let Some(virtual_addr) = frame_index
            .checked_mul(PAGE_SZ)
            .and_then(|offset| PHYS_OFF.checked_add(offset))
        else {
            continue;
        };

        match spans.last_mut() {
            Some(last_span) if last_span.0.checked_add(last_span.1) == Some(virtual_addr) => {
                last_span.1 += PAGE_SZ;
            }
            _ => spans.push((virtual_addr, PAGE_SZ)),
        }
    }

    spans
}

/// Validates the minimum ELF64 little-endian executable/shared-object header.
pub fn validate_elf_header(data: &[u8]) -> Result<usize, &'static str> {
    if data.len() < 64 {
        return Err("too_short");
    }
    if data[0] != 0x7f || data[1] != b'E' || data[2] != b'L' || data[3] != b'F' {
        return Err("bad_magic");
    }
    if data[4] != 2 {
        return Err("not_64bit");
    }
    if data[5] != 1 {
        return Err("not_le");
    }
    if data[6] != 1 {
        return Err("bad_version");
    }

    let elf_type = u16::from_le_bytes([data[16], data[17]]);
    if elf_type != 2 && elf_type != 3 {
        return Err("not_exec");
    }

    let entry = usize::try_from(u64::from_le_bytes([
        data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
    ]))
    .map_err(|_| "entry_overflow")?;
    let ph_offset = usize::try_from(u64::from_le_bytes([
        data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
    ]))
    .map_err(|_| "ph_overflow")?;
    let ph_entry_size = u16::from_le_bytes([data[54], data[55]]) as usize;
    let ph_count = u16::from_le_bytes([data[56], data[57]]) as usize;
    if ph_count == 0 {
        return Err("no_phdrs");
    }

    let ph_span = ph_entry_size.checked_mul(ph_count).ok_or("ph_overflow")?;
    let ph_end = ph_offset.checked_add(ph_span).ok_or("ph_overflow")?;
    if ph_end > data.len() {
        return Err("ph_overflow");
    }
    if ph_entry_size < 4 {
        return Err("ph_overflow");
    }

    // Look for at least one loadable segment (type 1) in the program headers.
    let mut load_count = 0usize;
    for index in 0..ph_count {
        let entry_offset = index.checked_mul(ph_entry_size).ok_or("ph_overflow")?;
        let base = ph_offset.checked_add(entry_offset).ok_or("ph_overflow")?;
        let program_type =
            u32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]);
        if program_type == 1 {
            load_count += 1;
        }
    }
    if load_count == 0 {
        return Err("no_load");
    }
    Ok(entry)
}

/// Chooses the CPU with the best simple load-balancing score.
pub fn compute_load_balance(
    task_counts: &[usize],
    priorities: &[i32],
    io_blocked: &[bool],
) -> usize {
    let cpu_count = task_counts.len();
    if cpu_count == 0 {
        return 0;
    }

    let mut best_cpu = 0usize;
    let mut best_score = i64::MIN;
    for cpu_id in 0..cpu_count {
        let task_count = task_counts.get(cpu_id).copied().unwrap_or(0);
        let priority = priorities.get(cpu_id).copied().unwrap_or(0) as i64;
        let is_io_blocked = io_blocked.get(cpu_id).copied().unwrap_or(false);

        let mut score = -(task_count as i64) * 100;
        score += priority * 10;
        if is_io_blocked {
            score -= 500;
        }
        score += if task_count > 0 { 50 } else { 0 };
        score += if cpu_id < cpu_count / 2 { 10 } else { -10 };

        if score > best_score {
            best_score = score;
            best_cpu = cpu_id;
        }
    }

    best_cpu
}

/// Simulated buddy allocator for page-sized physical memory blocks.
///
/// Each free list stores block base addresses for one order, where order `n`
/// means a block of `2^n` pages. This allocator tracks metadata only; it does
/// not own backing memory or enforce page-table mappings.
pub struct BuddyAllocator {
    pub free_lists: Vec<Vec<usize>>,
    pub max_order: usize,
    pub base_addr: usize,
    pub total_pages: usize,
    pub allocated: AtomicUsize,
}

impl BuddyAllocator {
    /// Creates a new buddy allocator. It always creates free blocks of the largest possible order.
    pub fn new(base: usize, total_pages: usize, max_order: usize) -> Self {
        let mut free_lists = Vec::with_capacity(max_order + 1);
        for _ in 0..=max_order {
            free_lists.push(Vec::new());
        }
        let order = log2_floor(total_pages);
        let usable_order = min(order, max_order);
        let block_pages = 1 << usable_order;
        let mut addr = base;
        let mut remaining = total_pages;
        while remaining >= block_pages {
            free_lists[usable_order].push(addr);
            addr += block_pages * PAGE_SZ;
            remaining -= block_pages;
        }
        for split_order in (0..usable_order).rev() {
            let pages = 1 << split_order;
            while remaining >= pages {
                free_lists[split_order].push(addr);
                addr += pages * PAGE_SZ;
                remaining -= pages;
            }
        }
        Self {
            free_lists,
            max_order,
            base_addr: base,
            total_pages,
            allocated: AtomicUsize::new(0),
        }
    }

    pub fn alloc_order(&mut self, order: usize) -> Option<usize> {
        if order > self.max_order {
            return None;
        }
        for source_order in order..=self.max_order {
            if let Some(block) = self.free_lists[source_order].pop() {
                let mut current_order = source_order;
                let addr = block;
                while current_order > order {
                    current_order -= 1;
                    let buddy = addr + (1 << current_order) * PAGE_SZ;
                    self.free_lists[current_order].push(buddy);
                }
                self.allocated.fetch_add(1 << order, Ordering::Relaxed);
                return Some(addr);
            }
        }
        None
    }

    // Note: this actually means if [addr,addr+2^order*PAGE_SZ) is in a free block.
    fn range_is_free(&self, addr: usize, order: usize) -> bool {
        let pages = 1usize << order;
        let size = pages.saturating_mul(PAGE_SZ);
        // Debug fix: when overflow occurs, we cannot consider it free.
        let Some(end) = addr.checked_add(size) else {
            return false;
        };
        for (free_order, list) in self.free_lists.iter().enumerate() {
            let free_size = (1usize << free_order).saturating_mul(PAGE_SZ);
            for &block in list {
                let Some(block_end) = block.checked_add(free_size) else {
                    continue;
                };
                if addr >= block && end <= block_end {
                    return true;
                }
            }
        }
        false
    }

    // the addr of a block's buddy can be computed by flipping the bit corresponding to the block size in the block's address.
    // this is the core of this function. but the detail is strange.
    pub fn free_order(&mut self, addr: usize, order: usize) {
        if order > self.max_order {
            return;
        }
        if self.range_is_free(addr, order) {
            return;
        }
        let mut current_addr = addr;
        let mut current_order = order;
        while current_order < self.max_order {
            let block_size = (1 << current_order) * PAGE_SZ;
            let Some(relative) = current_addr.checked_sub(self.base_addr) else {
                break;
            };
            let Some(buddy_addr) = self.base_addr.checked_add(relative ^ block_size) else {
                break;
            };
            if let Some(buddy_index) = self.free_lists[current_order]
                .iter()
                .position(|&candidate_addr| candidate_addr == buddy_addr)
            {
                self.free_lists[current_order].remove(buddy_index);
                current_addr = min(current_addr, buddy_addr);
                current_order += 1;
            } else {
                break;
            }
        }
        self.free_lists[current_order].push(current_addr);
        let pages = 1usize << order;
        let current_allocated_pages = self.allocated.load(Ordering::Relaxed);
        self.allocated.store(
            current_allocated_pages.saturating_sub(pages),
            Ordering::Relaxed,
        );
    }

    pub fn free_pages_count(&self) -> usize {
        let mut count = 0;
        for (order, list) in self.free_lists.iter().enumerate() {
            count += list.len() * (1 << order);
        }
        count
    }

    // There's a bug that it cannot distinguish free order 0 and no free pages.
    pub fn largest_free_order(&self) -> usize {
        for candidate_order in (0..=self.max_order).rev() {
            if !self.free_lists[candidate_order].is_empty() {
                return candidate_order;
            }
        }
        0
    }

    pub fn fragmentation_score(&self) -> usize {
        let total_free = self.free_pages_count();
        if total_free == 0 {
            return 0;
        }
        let largest = self.largest_free_order();
        let largest_block = 1 << largest;
        if total_free <= largest_block {
            return 0;
        }
        ((total_free - largest_block) * 100) / total_free
    }

    pub fn snapshot(&self) -> BuddyAllocator {
        BuddyAllocator {
            free_lists: self.free_lists.clone(),
            max_order: self.max_order,
            base_addr: self.base_addr,
            total_pages: self.total_pages,
            allocated: AtomicUsize::new(self.allocated.load(Ordering::Relaxed)),
        }
    }
}
