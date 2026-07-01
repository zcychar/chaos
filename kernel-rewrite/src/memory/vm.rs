#![allow(unused_imports)]

use super::{FramePool, PgFrame};
use crate::consts::*;
use std::cmp::min;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct VmRegion {
    pub base: usize,
    pub len: usize,
    pub flags: u32,
    pub offset: usize,
    pub tag: u16,
    pub ref_count: AtomicUsize,
}

impl VmRegion {
    pub fn new(base: usize, len: usize, flags: u32) -> Self {
        Self {
            base,
            len,
            flags,
            offset: 0,
            tag: 0,
            ref_count: AtomicUsize::new(1),
        }
    }

    pub fn with_offset(base: usize, len: usize, flags: u32, file_offset: usize) -> Self {
        Self {
            base,
            len,
            flags,
            offset: file_offset,
            tag: 0,
            ref_count: AtomicUsize::new(1),
        }
    }

    pub fn end(&self) -> usize {
        self.base.saturating_add(self.len)
    }

    pub fn contains(&self, addr: usize) -> bool {
        match self.base.checked_add(self.len) {
            Some(end) => addr >= self.base && addr < end,
            None => false,
        }
    }

    pub fn overlaps(&self, other: &VmRegion) -> bool {
        let Some(self_end) = self.base.checked_add(self.len) else {
            return false;
        };
        let Some(other_end) = other.base.checked_add(other.len) else {
            return false;
        };
        // Debug fix: VmRegion uses half-open ranges, so adjacent endpoints do not overlap.
        self.base < other_end && other.base < self_end
    }

    pub fn split_at(&self, addr: usize) -> Option<(VmRegion, VmRegion)> {
        let Some(self_end) = self.base.checked_add(self.len) else {
            return None;
        };
        if addr <= self.base || addr >= self_end {
            return None;
        }
        let left_len = addr - self.base;
        let right_len = self.len - left_len;
        let left_offset = self.offset;
        let right_offset = self.offset.wrapping_add(left_len);
        let mut left_flags = self.flags;
        let right_flags = self.flags;
        //If original region has VM_GROWSDOWN, only right region should have it, left region should not have it.(?)
        if self.flags & VM_GROWSDOWN != 0 {
            left_flags &= !VM_GROWSDOWN;
        }
        let left_region = VmRegion {
            base: self.base,
            len: left_len,
            flags: left_flags,
            offset: left_offset,
            tag: self.tag,
            ref_count: AtomicUsize::new(self.ref_count.load(Ordering::Relaxed)),
        };
        let right_region = VmRegion {
            base: addr,
            len: right_len,
            flags: right_flags,
            offset: right_offset,
            tag: self.tag,
            ref_count: AtomicUsize::new(self.ref_count.load(Ordering::Relaxed)),
        };
        Some((left_region, right_region))
    }

    pub fn merge_with(&self, other: &VmRegion) -> Option<VmRegion> {
        let Some(self_end) = self.base.checked_add(self.len) else {
            return None;
        };
        if self_end != other.base || self.flags != other.flags || self.tag != other.tag {
            return None;
        }
        let combined = VmRegion {
            base: self.base,
            len: self.len + other.len,
            flags: self.flags,
            offset: self.offset,
            tag: self.tag,
            ref_count: AtomicUsize::new(
                self.ref_count
                    .load(Ordering::Relaxed)
                    .max(other.ref_count.load(Ordering::Relaxed)),
            ),
        };
        Some(combined)
    }

    pub fn ref_up(&self) -> usize {
        self.ref_count.fetch_add(1, Ordering::Relaxed)
    }

    pub fn ref_down(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::Relaxed)
    }

    pub fn ref_get(&self) -> usize {
        self.ref_count.load(Ordering::Relaxed)
    }
}

/// Sorted virtual-memory map for one simulated address space.
///
/// `regions` is kept ordered by base address, `brk` tracks the heap break, and
/// `mmap_base` is the starting search point for anonymous/file mappings.
///
/// Note: currently this struct have many confused implementations, we tried to unify them but saftey (such as overflow) is not guaranteed.
pub struct VmMap {
    pub regions: Vec<VmRegion>,
    pub brk: usize,
    pub mmap_base: usize,
}

impl VmMap {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            brk: 0x0040_0000,
            mmap_base: 0x7000_0000,
        }
    }

    pub fn insert(&mut self, region: VmRegion) -> Result<(), &'static str> {
        let region_base = region.base;
        // Debug fix: reject zero-length, overflowing, or kernel-crossing ranges.
        let region_end = region_base.checked_add(region.len).ok_or("overflow")?;
        if region.len == 0
            || region_base >= KERN_BASE
            || region_end > KERN_BASE
            || region_end <= region_base
        {
            return Err("einval");
        }
        let mut insert_index = 0;
        while insert_index < self.regions.len() {
            let existing_region = &self.regions[insert_index];
            if existing_region.overlaps(&region) {
                return Err("overlap");
            }
            if existing_region.base > region_base {
                break;
            }
            insert_index += 1;
        }
        self.regions.insert(insert_index, region);
        Ok(())
    }

    pub fn find(&self, addr: usize) -> Option<&VmRegion> {
        let region_count = self.regions.len();
        if region_count == 0 {
            return None;
        }
        let mut low = 0;
        let mut high = region_count;
        while low < high {
            let mid = low + (high - low) / 2;
            let region = &self.regions[mid];
            if addr < region.base {
                high = mid;
            } else if addr >= region.end() {
                low = mid + 1;
            } else {
                return Some(region);
            }
        }
        None
    }

    pub fn remove_range(&mut self, base: usize, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        let before = self.regions.len();
        let mut index = 0;
        let target_region = VmRegion::new(base, len, 0);
        while index < self.regions.len() {
            if self.regions[index].overlaps(&target_region) {
                self.regions.remove(index);
            } else {
                index += 1;
            }
        }
        before - self.regions.len()
    }

    // Optimized to scan once because regions are sorted by base address.
    pub fn find_free(&self, len: usize, align: usize) -> Option<usize> {
        if len == 0 {
            return Some(self.mmap_base);
        }
        let alignment = if align > 1 { align } else { PAGE_SZ };
        let alignment_mask = alignment - 1;
        let mut candidate = (self.mmap_base + alignment_mask) & !alignment_mask;

        for region in self.regions.iter() {
            let candidate_end = candidate.checked_add(len)?;
            if candidate_end > KERN_BASE {
                return None;
            }

            let region_end = region.end();
            if region_end <= candidate {
                continue;
            }

            if candidate_end <= region.base {
                return Some(candidate);
            }

            candidate = (region_end + alignment_mask) & !alignment_mask;
        }

        let candidate_end = candidate.checked_add(len)?;
        if candidate_end <= KERN_BASE {
            Some(candidate)
        } else {
            None
        }
    }

    pub fn total_mapped(&self) -> usize {
        let mut total = 0usize;
        for region in self.regions.iter() {
            total = total.wrapping_add(region.len);
        }
        total
    }

    pub fn clone_regions(&self) -> Vec<VmRegion> {
        let mut cloned_regions = Vec::with_capacity(self.regions.len());
        for region in self.regions.iter() {
            let cloned_region = VmRegion {
                base: region.base,
                len: region.len,
                flags: region.flags,
                offset: region.offset,
                tag: region.tag,
                ref_count: AtomicUsize::new(region.ref_count.load(Ordering::Relaxed)),
            };
            cloned_regions.push(cloned_region);
        }
        cloned_regions
    }

    pub fn gap_after(&self, idx: usize) -> usize {
        if idx >= self.regions.len() {
            return 0;
        }
        if idx + 1 < self.regions.len() {
            self.regions[idx + 1]
                .base
                .saturating_sub(self.regions[idx].end())
        } else {
            KERN_BASE.saturating_sub(self.regions[idx].end())
        }
    }
}

/// Checks whether the half-open user range `[addr, addr + len)` stays below the kernel base.
pub fn check_access(addr: usize, len: usize) -> bool {
    if len == 0 {
        return true;
    }

    // Debug fix: checked arithmetic rejects ranges that wrap around usize.
    match addr.checked_add(len) {
        Some(end) => addr < KERN_BASE && end <= KERN_BASE,
        None => false,
    }
}

/// Fix: This is useless now since we need to change signature to include VmMap for checking rw.
pub fn check_access_rw(addr: usize, len: usize, _writable: bool) -> bool {
    check_access(addr, len)
}

/// Simulated copy-from-user helper.
///
/// The standalone rewrite cannot dereference guest memory, so a valid range
/// returns `T::default()` rather than reading real bytes.
pub fn cfu<T: Copy + Default>(addr: usize, len: usize) -> Option<T> {
    let effective_len = if len == 0 {
        std::mem::size_of::<T>()
    } else {
        len
    };
    if check_access(addr, effective_len) {
        Some(T::default())
    } else {
        None
    }
}

/// Simulated copy-to-user helper; validates the destination range only.
pub fn ctu<T: Copy>(addr: usize, len: usize, _value: &T) -> bool {
    let effective_len = if len == 0 {
        std::mem::size_of::<T>()
    } else {
        len
    };
    check_access_rw(addr, effective_len, true)
}

/// Simulated read-user fixup marker. (USELESS...)
pub fn rdu_fixup() -> usize {
    1
}

/// Computes a rough resident-set watermark from VM region sizes and permissions.
pub fn compute_rss_watermark(regions: &[VmRegion], pool_capacity: usize) -> usize {
    if regions.is_empty() || pool_capacity == 0 {
        return 0;
    }

    let mut total_weight = 0u64;
    for region in regions {
        let pages = region
            .len
            .checked_add(PAGE_SZ - 1)
            .map(|rounded_len| rounded_len / PAGE_SZ)
            .unwrap_or(usize::MAX / PAGE_SZ);
        let permission_weight = if region.flags & VM_EXEC != 0 {
            3
        } else if region.flags & VM_WRITE != 0 {
            2
        } else {
            1
        };
        let sharing_weight = if region.flags & VM_SHARED != 0 { 1 } else { 2 };
        total_weight = total_weight.saturating_add(
            (pages as u64)
                .saturating_mul(permission_weight)
                .saturating_mul(sharing_weight),
        );
    }

    let capacity = pool_capacity as u64;
    let raw_watermark = total_weight.saturating_mul(100) / capacity;
    min(raw_watermark, capacity / 2) as usize
}

/// Per-file-descriptor access options.
///
/// `rd` and `wr` control read/write permission, `ap` means append mode, and
/// `nb` means nonblocking mode.
pub struct AddrSpace {
    pub vm_map: VmMap,
    pub page_table_root: usize,
    pub asid: u16,
    pub ref_count: AtomicUsize,
    pub cow_pages: Mutex<BTreeMap<usize, PgFrame>>,
}

impl AddrSpace {
    pub fn new(asid: u16) -> Self {
        Self {
            vm_map: VmMap::new(),
            page_table_root: 0,
            asid,
            ref_count: AtomicUsize::new(1),
            cow_pages: Mutex::new(BTreeMap::new()),
        }
    }

    // Refactor: use vm_map's methods.
    // Note: 'cow_pages' maybe should be shared between parent and child, so it should be a Arc<Mutex<>>, but we do not change it for now.
    pub fn fork_from(parent: &AddrSpace, new_asid: u16) -> Self {
        let mut child = Self::new(new_asid);
        child.vm_map.brk = parent.vm_map.brk;
        child.vm_map.mmap_base = parent.vm_map.mmap_base;
        for source_region in parent.vm_map.regions.iter() {
            if source_region.flags & VM_WRITE != 0 {
                source_region.ref_up();
            }
        }
        for cloned_region in parent.vm_map.clone_regions() {
            let _ = child.vm_map.insert(cloned_region);
        }
        {
            let parent_cow = parent.cow_pages.lock().unwrap();
            let mut child_cow = child.cow_pages.lock().unwrap();
            for (&addr, frame) in parent_cow.iter() {
                frame.up();
                // here we can only copy a new PgFrame with the same count.
                child_cow.insert(addr, PgFrame::with_rc(frame.count()));
            }
        }
        child
    }

    // Note: the correct behavior of this function is to detect whether the page is shared then assign a new frame to the page.
    // Note: currently the system is not uniform on address space management.
    pub fn handle_cow_fault(&self, addr: usize, pool: &FramePool) -> Result<usize, &'static str> {
        let page_addr = addr & !(PAGE_SZ - 1);
        let region = self.vm_map.find(addr).ok_or("segfault")?;
        if region.flags & VM_WRITE == 0 {
            return Err("segfault");
        }
        let mut cow = self.cow_pages.lock().unwrap();
        if let Some(frame) = cow.get(&page_addr) {
            let rc = frame.count();
            if rc <= 1 {
                return Ok(page_addr);
            }
            let new_frame_id = pool.get_inner().ok_or("oom")?;
            frame.down();
            let new_frame = PgFrame::with_rc(1);
            cow.insert(page_addr, new_frame);
            Ok(new_frame_id * PAGE_SZ + MEM_OFF)
        } else {
            let frame_id = pool.get_inner().ok_or("oom")?;
            cow.insert(page_addr, PgFrame::with_rc(1));
            Ok(frame_id * PAGE_SZ + MEM_OFF)
        }
    }

    // remove the mapping of the specified range and free the associated PgFrame.
    // Note: we do not understand why it only removes from cow_pages.
    pub fn unmap_range(&mut self, start: usize, len: usize) -> usize {
        let Some(end) = start.checked_add(len) else {
            return 0;
        };
        let removed = self.vm_map.remove_range(start, len);
        let mut cow = self.cow_pages.lock().unwrap();
        let pages_to_remove: Vec<usize> = cow
            .keys()
            .filter(|&&addr| addr >= start && addr < end)
            .copied()
            .collect();
        for addr in &pages_to_remove {
            if let Some(frame) = cow.remove(addr) {
                frame.down();
            }
        }
        removed + pages_to_remove.len()
    }

    // currently this function changes flags of overlapping regions.
    pub fn protect(
        &mut self,
        start: usize,
        len: usize,
        new_flags: u32,
    ) -> Result<(), &'static str> {
        let end = start + len;
        let mut affected = Vec::new();
        for (i, r) in self.vm_map.regions.iter().enumerate() {
            if r.base < end && r.end() > start {
                affected.push(i);
            }
        }
        for &idx in affected.iter().rev() {
            if idx < self.vm_map.regions.len() {
                self.vm_map.regions[idx].flags = new_flags;
            }
        }
        Ok(())
    }

    pub fn rss_pages(&self) -> usize {
        self.cow_pages.lock().unwrap().len()
    }

    pub fn cow_sharers(&self) -> usize {
        let cow = self.cow_pages.lock().unwrap();
        cow.values().filter(|f| f.count() > 1).count()
    }

    pub fn split_region(&mut self, addr: usize) -> Result<(), &'static str> {
        let idx = self
            .vm_map
            .regions
            .iter()
            .position(|r| r.contains(addr))
            .ok_or("enomem")?;
        let (left, right) = self.vm_map.regions[idx].split_at(addr).ok_or("einval")?;
        self.vm_map.regions[idx] = left;
        self.vm_map.regions.insert(idx + 1, right);
        Ok(())
    }
}
