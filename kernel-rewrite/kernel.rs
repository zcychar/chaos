#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};

pub const PAGE_SZ: usize = 4096;
pub const KERN_BASE: usize = 0xFFFF_FFFF_8000_0000;

pub const VM_READ: u32 = 0x01;
pub const VM_WRITE: u32 = 0x02;
pub const VM_EXEC: u32 = 0x04;
pub const VM_SHARED: u32 = 0x08;
pub const VM_GROWSDOWN: u32 = 0x10;
pub const VM_DONTCOPY: u32 = 0x20;
pub const VM_HUGETLB: u32 = 0x40;
pub const VM_PFNMAP: u32 = 0x80;

pub const CAP_CHOWN: u32 = 0;
pub const CAP_KILL: u32 = 5;
pub const CAP_SETUID: u32 = 7;
pub const CAP_SETGID: u32 = 6;
pub const CAP_NET_BIND: u32 = 10;
pub const CAP_NET_RAW: u32 = 13;
pub const CAP_SYS_ADMIN: u32 = 21;
pub const CAP_SYS_PTRACE: u32 = 19;
pub const INHERITABLE_MASK: u64 = 0x0000_00FF_FFFF_FFFF;

pub const NSIG: u32 = 64;
pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;
pub const SIGKILL: u32 = 9;
pub const SIGSTOP: u32 = 19;
pub const SIGCHLD: u32 = 17;
pub const SIGUSR1: u32 = 10;
pub const SIGUSR2: u32 = 12;
pub const SIGALRM: u32 = 14;

/// Describes one virtual-memory area and its mapping metadata.
///
/// The region is a half-open address range: `[base, base + len)`.
/// `flags` stores VM_* permission and behavior bits, while `offset` points to
/// the start of the backing file data for file-backed mappings.
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

/// Tracks permitted, effective, and ambient capability bitsets.
///
/// `bits` is the permitted capability set, `effective` is the set checked for
/// active permission decisions, and `ambient` records capabilities that may be
/// carried across inheritance paths when they remain permitted.
pub struct CapSet {
    pub bits: u64,
    pub effective: u64,
    pub ambient: u64,
}

impl CapSet {
    pub fn new() -> Self {
        Self {
            bits: 0,
            effective: 0,
            ambient: 0,
        }
    }

    pub fn full() -> Self {
        Self {
            bits: !0u64,
            effective: !0u64,
            ambient: 0,
        }
    }

    pub fn check(&self, cap: u32) -> bool {
        if cap >= 64 {
            return false;
        }
        (self.effective & (1u64 << cap)) != 0
    }

    pub fn grant(&mut self, cap: u32) {
        if cap < 64 {
            self.bits |= 1u64 << cap;
            self.effective |= 1u64 << cap;
        }
    }

    pub fn drop_cap(&mut self, cap: u32) {
        if cap < 64 {
            self.bits &= !(1u64 << cap);
            self.effective &= !(1u64 << cap);
        }
    }

    pub fn inherit(parent: &CapSet) -> CapSet {
        let mask = INHERITABLE_MASK;
        let parent_bits = parent.bits;
        let parent_effective = parent.effective;
        // Debug fix: inheritance keeps only bits allowed by INHERITABLE_MASK.
        let filtered_bits = parent_bits & mask;
        let filtered_effective = parent_effective & filtered_bits;
        CapSet {
            bits: filtered_bits,
            effective: filtered_effective,
            // Debug fix: ambient capabilities must remain a subset of inherited bits.
            ambient: parent.ambient & filtered_bits,
        }
    }

    pub fn has_any(&self, mask: u64) -> bool {
        (self.effective & mask) != 0
    }

    pub fn clear_ambient(&mut self) {
        self.ambient = 0;
    }

    pub fn raise_ambient(&mut self, cap: u32) -> bool {
        if cap >= 64 {
            return false;
        }
        let cap_bit = 1u64 << cap;
        if (self.bits & cap_bit) != 0 {
            self.ambient |= cap_bit;
            true
        } else {
            false
        }
    }
}

/// Describes how one signal should be handled.
///
/// Signal-number validity is enforced by `SigSet::set_action`; this struct only
/// stores the handler payload, action flags, and handler-time signal mask.
pub struct SigAction {
    pub handler: usize,
    pub flags: u32,
    pub mask: u64,
}

/// Tracks pending signals, blocked signals, and per-signal actions.
pub struct SigSet {
    pub pending: u64,
    pub blocked: u64,
    pub actions: Vec<SigAction>,
}

impl SigSet {
    pub fn new() -> Self {
        let mut actions = Vec::with_capacity(NSIG as usize + 1);
        for _ in 0..=NSIG {
            actions.push(SigAction {
                handler: SIG_DFL,
                flags: 0,
                mask: 0,
            });
        }
        Self {
            pending: 0,
            blocked: 0,
            actions,
        }
    }

    pub fn sig_pending(&self, signo: u32) -> bool {
        // Debug fix: signal 0 and out-of-range signals must not be shifted into the mask.
        if signo == 0 || signo >= NSIG {
            return false;
        }
        (self.pending & (1u64 << signo)) != 0
    }

    pub fn sig_raise(&mut self, signo: u32) {
        // Debug fix: signal 0 is not deliverable and must not become pending.
        if signo > 0 && signo < NSIG {
            self.pending |= 1u64 << signo;
        }
    }

    pub fn coalesce_pending(&mut self) -> u64 {
        let active = self.pending & !self.blocked;
        let mut result: u64 = 0;
        for i in 1..NSIG {
            if (active & (1u64 << i)) != 0 {
                result |= 1u64 << i;
            }
        }
        result
    }

    pub fn sig_clear(&mut self, signo: u32) {
        if signo < NSIG {
            self.pending &= !(1u64 << signo);
        }
    }

    pub fn sig_block(&mut self, mask: u64) {
        self.blocked |= mask;
        self.blocked &= !((1u64 << SIGKILL) | (1u64 << SIGSTOP));
    }

    pub fn sig_unblock(&mut self, mask: u64) {
        self.blocked &= !mask;
    }

    pub fn sig_setmask(&mut self, mask: u64) {
        self.blocked = mask & !((1u64 << SIGKILL) | (1u64 << SIGSTOP));
    }

    pub fn deliverable(&self) -> Option<u32> {
        let actionable = self.pending & !self.blocked;
        if actionable == 0 {
            return None;
        }
        for i in 1..NSIG {
            if (actionable & (1u64 << i)) != 0 {
                return Some(i);
            }
        }
        None
    }

    pub fn set_action(&mut self, signo: u32, action: SigAction) {
        // Debug fix: signal 0, SIGKILL, and SIGSTOP cannot install custom actions.
        if signo > 0 && signo < NSIG as u32 && signo != SIGKILL && signo != SIGSTOP {
            self.actions[signo as usize] = action;
        }
    }

    pub fn get_action(&self, signo: u32) -> &SigAction {
        if (signo as usize) < self.actions.len() {
            &self.actions[signo as usize]
        } else {
            &self.actions[0]
        }
    }

    pub fn is_ignored(&self, signo: u32) -> bool {
        if (signo as usize) < self.actions.len() {
            self.actions[signo as usize].handler == SIG_IGN
        } else {
            false
        }
    }

    pub fn clear_non_caught(&mut self) {
        for i in 1..self.actions.len() {
            if self.actions[i].handler != SIG_DFL && self.actions[i].handler != SIG_IGN {
                self.actions[i].handler = SIG_DFL;
            }
        }
    }
}
