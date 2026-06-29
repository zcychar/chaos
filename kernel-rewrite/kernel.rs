#![allow(dead_code)]

use std::cmp::min;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt;
use std::ops::{Deref, Index};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::thread;
use std::time::Duration;

pub const PAGE_SZ: usize = 4096;
pub const N_PROC: usize = 256;
pub const N_FRAMES: usize = 65536;
pub const KERN_BASE: usize = 0xFFFF_FFFF_8000_0000;
pub const PHYS_OFF: usize = 0xFFFF_FFFF_0000_0000;
pub const MEM_OFF: usize = 0x8000_0000;
pub const KHEAP_SZ: usize = 0x800000;
pub const N_CHAINS: usize = 64;
pub const RBUF_CAP: usize = 256;
pub const N_REGS: usize = 16;
pub const MNT_DEPTH: usize = 8;
pub const MAX_CPU: usize = 8;
pub const KSTK_SZ: usize = 0x4000;
pub const USR_STK_OFF: usize = 0x7FFF_0000;
pub const USR_STK_SZ: usize = 0x10000;
pub const USEC_TICK: usize = 1000;
pub const FOLLOW_LIM: usize = 3;

pub const F_DUPFD: usize = 0;
pub const F_GETFD: usize = 1;
pub const F_SETFD: usize = 2;
pub const F_GETFL: usize = 3;
pub const F_SETFL: usize = 4;
pub const F_GETLK: usize = 5;
pub const F_SETLK: usize = 6;
pub const F_SETLKW: usize = 7;
pub const FD_CLOEXEC: usize = 1;
pub const F_DUPFD_CLOEXEC: usize = 1030;
pub const O_NONBLOCK: usize = 0o4000;
pub const O_APPEND: usize = 0o2000;
pub const O_CLOEXEC: usize = 0o2000000;
pub const AT_NOFOLLOW: usize = 0x100;

pub const TCGETS: usize = 0x5401;
pub const TCSETS: usize = 0x5402;
pub const TIOCGPGRP: usize = 0x540F;
pub const TIOCSPGRP: usize = 0x5410;
pub const TIOCGWINSZ: usize = 0x5413;
pub const FIONCLEX: usize = 0x5450;
pub const FIOCLEX: usize = 0x5451;
pub const FIONBIO: usize = 0x5421;

pub const AT_PHDR: u8 = 3;
pub const AT_PHENT: u8 = 4;
pub const AT_PHNUM: u8 = 5;
pub const AT_PAGESZ: u8 = 6;
pub const AT_BASE: u8 = 7;
pub const AT_ENTRY: u8 = 9;

pub const LM_ISIG: u32 = 0o000001;
pub const LM_ICANON: u32 = 0o000002;
pub const LM_ECHO: u32 = 0o000010;
pub const LM_ECHOE: u32 = 0o000020;
pub const LM_ECHOK: u32 = 0o000040;
pub const LM_ECHONL: u32 = 0o000100;
pub const LM_NOFLSH: u32 = 0o000200;
pub const LM_TOSTOP: u32 = 0o000400;
pub const LM_IEXTEN: u32 = 0o100000;
pub const LM_XCASE: u32 = 0o000004;
pub const LM_ECHOCTL: u32 = 0o001000;
pub const LM_ECHOPRT: u32 = 0o002000;
pub const LM_ECHOKE: u32 = 0o004000;
pub const LM_FLUSHO: u32 = 0o010000;
pub const LM_PENDIN: u32 = 0o040000;
pub const LM_EXTPROC: u32 = 0o200000;

pub const VM_READ: u32 = 0x01;
pub const VM_WRITE: u32 = 0x02;
pub const VM_EXEC: u32 = 0x04;
pub const VM_SHARED: u32 = 0x08;
pub const VM_GROWSDOWN: u32 = 0x10;
pub const VM_DONTCOPY: u32 = 0x20;
pub const VM_HUGETLB: u32 = 0x40;
pub const VM_PFNMAP: u32 = 0x80;

pub const ZONE_DMA: usize = 0;
pub const ZONE_NORMAL: usize = 1;
pub const ZONE_HIGH: usize = 2;
pub const N_ZONES: usize = 3;

pub const PRIO_MIN: i32 = -20;
pub const PRIO_MAX: i32 = 19;
pub const PRIO_DEFAULT: i32 = 0;
pub const SCHED_NORMAL: u8 = 0;
pub const SCHED_FIFO: u8 = 1;
pub const SCHED_RR: u8 = 2;
pub const SCHED_BATCH: u8 = 3;

pub const SLAB_OBJ_MIN: usize = 8;
pub const SLAB_OBJ_MAX: usize = 2048;
pub const SLAB_ALIGN: usize = 8;

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

pub const TIMER_WHEEL_SIZE: usize = 256;
pub const TIMER_TICK_HZ: usize = 100;
pub const BOOT_EPOCH: usize = 0;

pub const SOCK_STREAM: u32 = 1;
pub const SOCK_DGRAM: u32 = 2;
pub const SOCK_RAW: u32 = 3;
pub const AF_INET: u32 = 2;
pub const AF_INET6: u32 = 10;
pub const AF_UNIX: u32 = 1;

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_STAT: usize = 4;
pub const SYS_FSTAT: usize = 5;
pub const SYS_MMAP: usize = 9;
pub const SYS_MUNMAP: usize = 11;
pub const SYS_BRK: usize = 12;
pub const SYS_IOCTL: usize = 16;
pub const SYS_PIPE: usize = 22;
pub const SYS_DUP: usize = 32;
pub const SYS_DUP2: usize = 33;
pub const SYS_FORK: usize = 57;
pub const SYS_EXEC: usize = 59;
pub const SYS_EXIT: usize = 60;
pub const SYS_WAIT4: usize = 61;
pub const SYS_KILL: usize = 62;
pub const SYS_FCNTL: usize = 72;
pub const SYS_GETPID: usize = 39;
pub const SYS_GETPPID: usize = 110;
pub const SYS_SETPGID: usize = 109;
pub const SYS_GETPGID: usize = 121;
pub const SYS_SETSID: usize = 112;
pub const SYS_EPOLL_CREATE: usize = 213;
pub const SYS_EPOLL_CTL: usize = 233;
pub const SYS_EPOLL_WAIT: usize = 232;
pub const SYS_CLOCK_GETTIME: usize = 228;
pub const SYS_SIGACTION: usize = 13;
pub const SYS_SIGPROCMASK: usize = 14;
pub const SYS_FUTEX: usize = 202;

pub const IOQUEUE_DEPTH: usize = 128;

pub static CLK: AtomicUsize = AtomicUsize::new(0);

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

    pub fn coalesce_pending(&self) -> u64 {
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
        // These two signals cannot be blocked, so we clear them from the blocked mask.
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

    // resets all signal handlers that are not SIG_DFL or SIG_IGN to SIG_DFL
    pub fn clear_non_caught(&mut self) {
        for i in 1..self.actions.len() {
            if self.actions[i].handler != SIG_DFL && self.actions[i].handler != SIG_IGN {
                self.actions[i].handler = SIG_DFL;
            }
        }
    }
}

/// Represents one simulated timer scheduled against the global tick clock.
pub struct TimerEntry {
    pub deadline: usize,
    pub interval: usize,
    pub callback_id: usize,
    pub active: bool,
    pub repeat: bool,
}

impl TimerEntry {
    pub fn new(deadline: usize, interval: usize, callback_id: usize) -> Self {
        Self {
            deadline,
            interval,
            callback_id,
            active: true,
            repeat: interval > 0,
        }
    }

    pub fn expired(&self) -> bool {
        // Debug fix: timers expire at the deadline tick, not only after it.
        CLK.load(Ordering::Relaxed) >= self.deadline
    }

    pub fn reset(&mut self) {
        if self.repeat {
            // Debug fix: repeated timer deadlines must not overflow.
            self.deadline = CLK.load(Ordering::Relaxed).saturating_add(self.interval);
        } else {
            self.active = false;
        }
    }

    pub fn remaining(&self) -> usize {
        let now = CLK.load(Ordering::Relaxed);
        if now >= self.deadline {
            0
        } else {
            self.deadline - now
        }
    }

    pub fn cancel(&mut self) {
        self.active = false;
    }
}

/// Fixed-size timer wheel for grouping timers by deadline slot.
pub struct TimerWheel {
    pub slots: Vec<Vec<TimerEntry>>,
    pub current_slot: usize,
}

impl TimerWheel {
    pub fn new() -> Self {
        let mut slots = Vec::with_capacity(TIMER_WHEEL_SIZE);
        for _ in 0..TIMER_WHEEL_SIZE {
            slots.push(Vec::new());
        }
        Self {
            slots,
            current_slot: 0,
        }
    }

    pub fn add_timer(&mut self, entry: TimerEntry) {
        let slot_index = entry.deadline % TIMER_WHEEL_SIZE;
        self.slots[slot_index].push(entry);
    }

    pub fn advance(&mut self) -> Vec<TimerEntry> {
        self.current_slot = (self.current_slot + 1) % TIMER_WHEEL_SIZE;
        let mut fired = Vec::new();
        let slot = &mut self.slots[self.current_slot];
        let mut remaining = Vec::new();

        for entry in slot.drain(..) {
            if entry.active && entry.expired() {
                fired.push(entry);
            } else if entry.active {
                remaining.push(entry);
            }
        }
        *slot = remaining;

        for timer in fired.iter_mut() {
            if timer.repeat {
                timer.reset();
                let rescheduled_timer =
                    TimerEntry::new(timer.deadline, timer.interval, timer.callback_id);
                self.add_timer(rescheduled_timer);
            }
        }
        fired
    }

    pub fn cancel(&mut self, callback_id: usize) -> bool {
        for slot in self.slots.iter_mut() {
            for entry in slot.iter_mut() {
                if entry.callback_id == callback_id && entry.active {
                    entry.active = false;
                    return true;
                }
            }
        }
        false
    }

    pub fn active_count(&self) -> usize {
        self.slots
            .iter()
            .flat_map(|slot| slot.iter())
            .filter(|entry| entry.active)
            .count()
    }
}

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

/// Describes one physical-memory allocation zone.
///
/// Zones group page frames by PFN range and track free-page watermarks. The
/// frame allocator uses these watermarks to decide whether the zone can satisfy
/// another allocation and how much reclaim pressure exists.
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

/// Fixed-size circular byte buffer used by channel and terminal-style queues.
///
/// `read_cursor` and `write_cursor` are cursor counters, `capacity` is the ring capacity, and `len` is the
/// number of bytes currently stored. Push and pop wrap cursor counters back into
/// the backing vector with modulo arithmetic.
/// Note: the Buffer writes to index 1 instead of index 0 at first time. We do not change this behavior now.
pub struct CircBuf {
    pub data: Vec<u8>,
    pub read_cursor: usize,
    pub write_cursor: usize,
    pub capacity: usize,
    pub len: usize,
}

impl CircBuf {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0u8; capacity],
            read_cursor: 0,
            write_cursor: 0,
            capacity: capacity,
            len: 0,
        }
    }

    pub fn with_pos(capacity: usize, read_cursor: usize, write_cursor: usize) -> Self {
        // Debug fix: fix if the cursors are out of bounds.
        let len = if capacity == 0 {
            0
        } else {
            let read_index = read_cursor % capacity;
            let write_index = write_cursor % capacity;
            if write_index >= read_index {
                write_index - read_index
            } else {
                capacity - read_index + write_index
            }
        };

        Self {
            data: vec![0u8; capacity],
            read_cursor: read_cursor,
            write_cursor: write_cursor,
            capacity: capacity,
            len: len,
        }
    }

    pub fn push(&mut self, v: u8) -> bool {
        // Debug fix: reject full or zero-capacity rings before moving the write cursor.
        if self.capacity == 0 || self.len >= self.capacity {
            return false;
        }
        self.write_cursor = self.write_cursor.wrapping_add(1);
        let write_index = self.write_cursor % self.capacity;
        if write_index >= self.data.len() {
            self.write_cursor = self.write_cursor.wrapping_sub(1);
            return false;
        }
        self.data[write_index] = v;
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.capacity == 0 || self.len == 0 {
            return None;
        }
        self.read_cursor = self.read_cursor.wrapping_add(1);
        let read_index = self.read_cursor % self.capacity;
        if read_index >= self.data.len() {
            self.read_cursor = self.read_cursor.wrapping_sub(1);
            return None;
        }
        self.len -= 1;
        Some(self.data[read_index])
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn empty(&self) -> bool {
        self.len == 0
    }

    pub fn full(&self) -> bool {
        self.len >= self.capacity
    }

    pub fn peek(&self) -> Option<u8> {
        if self.capacity == 0 || self.len == 0 {
            return None;
        }
        let read_index = self.read_cursor.wrapping_add(1) % self.capacity;
        if read_index >= self.data.len() {
            return None;
        }
        Some(self.data[read_index])
    }

    //Debug fix: drain_to should not drain more than the current length of the buffer.
    pub fn drain_to(&mut self, dst: &mut Vec<u8>, max: usize) -> usize {
        let mut drained = 0;
        for _ in 0..min(max, self.len) {
            if let Some(byte) = self.pop() {
                dst.push(byte);
                drained += 1;
            } else {
                break;
            }
        }
        drained
    }

    pub fn fill_from(&mut self, src: &[u8]) -> usize {
        let mut written = 0;
        for &byte in src {
            if !self.push(byte) {
                break;
            }
            written += 1;
        }
        written
    }

    pub fn remaining(&self) -> usize {
        self.capacity.saturating_sub(self.len)
    }
}

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

/// Event bit constants used by event buses, poll-like readiness, semaphores,
/// task exit notifications, and signal wakeups.
pub struct EventFlag;

impl EventFlag {
    pub const READABLE: u32 = 1 << 0;
    pub const WRITABLE: u32 = 1 << 1;
    pub const ERROR: u32 = 1 << 2;
    pub const CLOSED: u32 = 1 << 3;
    pub const PROC_QUIT: u32 = 1 << 10;
    pub const CHILD_QUIT: u32 = 1 << 11;
    pub const RECV_SIG: u32 = 1 << 12;
    pub const SEM_RM: u32 = 1 << 20;
    pub const SEM_ACQ: u32 = 1 << 21;
}

pub type EventCallback = Box<dyn Fn(u32) -> bool + Send>;

/// Small event bus that stores the current event mask and one-shot callbacks.
///
/// Callbacks return `true` when they have handled the event and should be
/// removed. This is used by poll-like readiness and wakeup paths in the
/// simulation.
#[derive(Default)]
pub struct EventBus {
    pub events: u32,
    pub callbacks: Vec<EventCallback>,
}

impl EventBus {
    pub fn make() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }

    pub fn set(&mut self, set_bits: u32) {
        self.change(0, set_bits);
    }

    pub fn clear(&mut self, clear_bits: u32) {
        self.change(clear_bits, 0);
    }

    //Use updated mask to call callbacks, and remove the callback if it returns true.
    pub fn change(&mut self, clear_bits: u32, set_bits: u32) {
        let previous_events = self.events;
        self.events = (self.events & !clear_bits) | set_bits;
        if self.events != previous_events {
            // Callbacks observe a stable copy of the updated event mask.
            let current_events = self.events;
            self.callbacks.retain(|f| !f(current_events));
        }
    }

    pub fn sub(&mut self, callback: EventCallback) {
        self.callbacks.push(callback);
    }

    pub fn callback_count(&self) -> usize {
        self.callbacks.len()
    }
}

// Waits for one or more events to be set on the bus, returning the matching event mask.
pub fn wait_event(bus: &Arc<Mutex<EventBus>>, mask: u32) -> u32 {
    if mask == 0 {
        return 0;
    }
    loop {
        {
            let g = bus.lock().unwrap();
            if (g.events & mask) != 0 {
                return g.events & mask;
            }
        }
        thread::yield_now();
    }
}

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
/// Actually, we do not use this for now.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Closed,
    Listen,
    SynSent,
    SynRecvd,
    Established,
    FinWait1,
    FinWait2,
    TimeWait,
    CloseWait,
    LastAck,
    Closing,
}

/// Records one epoll-style registration in the shared synchronization queue.
///
/// `task_id` identifies the waiting task, `epfd` is that task's epoll file
/// descriptor, and `fd` is the watched file descriptor that may become ready.
pub struct RegEp {
    pub task_id: usize,
    pub epfd: usize,
    pub fd: usize,
}

/// Condition-variable style wait queue used by simulated kernel subsystems.
///
/// `waiters` stores parked threads waiting for a wakeup, `epoll_registrations`
/// stores epoll-style registrations, and `pending_signal_tokens` counts wake
/// tokens that arrived before a thread was able to park. The token counter is
/// what prevents signal-before-wait lost wakeups in the std-thread simulation.
///
/// Note: currently, we cannot establish a connection between rust Thread and the simulated task id, so we cannot use the task id to identify the waiting thread.
/// Also, we do not understand why epoll should be implemented in this way, but we just keep it for now.
pub struct SyncQueue {
    waiters: Mutex<VecDeque<thread::Thread>>,
    epoll_registrations: Mutex<VecDeque<RegEp>>,
    pending_signal_tokens: AtomicUsize,
}

impl SyncQueue {
    pub fn new() -> Self {
        Self {
            waiters: Mutex::new(VecDeque::new()),
            epoll_registrations: Mutex::new(VecDeque::new()),
            pending_signal_tokens: AtomicUsize::new(0),
        }
    }

    fn consume_signal(&self) -> bool {
        loop {
            let token_count = self.pending_signal_tokens.load(Ordering::Acquire);
            if token_count == 0 {
                return false;
            }
            if self
                .pending_signal_tokens
                .compare_exchange_weak(
                    token_count,
                    token_count - 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return true;
            }
        }
    }

    fn record_signal(&self) {
        let mut token_count = self.pending_signal_tokens.load(Ordering::Acquire);
        loop {
            if token_count == usize::MAX {
                return;
            }
            match self.pending_signal_tokens.compare_exchange_weak(
                token_count,
                token_count + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(next_count) => token_count = next_count,
            }
        }
    }

    fn remove_waiter_by_id(&self, thread_id: &thread::ThreadId) -> bool {
        let mut waiters = self.waiters.lock().unwrap();
        let waiter_count_before_cleanup = waiters.len();
        waiters.retain(|waiter| waiter.id() != thread_id.clone());
        waiters.len() != waiter_count_before_cleanup
    }

    fn remove_waiter_from_all(queues: &[&SyncQueue], thread_id: &thread::ThreadId) {
        for queue in queues {
            queue.remove_waiter_by_id(thread_id);
        }
    }

    fn enqueue_current_thread_if(&self, should_wait: impl FnOnce() -> bool) -> bool {
        let current_thread = thread::current();
        let current_thread_id = current_thread.id();
        let mut waiters = self.waiters.lock().unwrap();
        if !should_wait() {
            return false;
        }
        if !waiters
            .iter()
            .any(|waiter| waiter.id() == current_thread_id.clone())
        {
            waiters.push_back(current_thread);
        }
        true
    }

    pub fn park_on<T>(&self, state: &Mutex<T>, predicate: impl Fn(&T) -> bool) -> bool {
        {
            let state_guard = state.lock().unwrap();
            if predicate(&state_guard) {
                return true;
            }
        }
        // Debug fix: a wake that happens before this thread parks is stored as a token.
        if self.consume_signal() {
            let state_guard = state.lock().unwrap();
            return predicate(&state_guard);
        }
        let current_thread = thread::current();
        let current_thread_id = current_thread.id();
        let mut waiters = self.waiters.lock().unwrap();
        if self.consume_signal() {
            drop(waiters);
            let state_guard = state.lock().unwrap();
            return predicate(&state_guard);
        }
        waiters.push_back(current_thread);
        drop(waiters);
        thread::park();
        self.remove_waiter_by_id(&current_thread_id);
        // Debug fix: after any wakeup, recheck the predicate and return its result.
        let state_guard = state.lock().unwrap();
        predicate(&state_guard)
    }

    pub fn signal(&self) {
        let mut waiters = self.waiters.lock().unwrap();
        if let Some(waiter) = waiters.pop_front() {
            drop(waiters);
            waiter.unpark();
        } else {
            self.record_signal();
        }
    }

    pub fn broadcast(&self) {
        let mut waiters = self.waiters.lock().unwrap();
        let wake_batch: Vec<thread::Thread> = waiters.drain(..).collect();
        drop(waiters);
        for waiter in wake_batch {
            waiter.unpark();
        }
    }

    pub fn signal_n(&self, requested_wake_count: usize) -> usize {
        let mut waiters = self.waiters.lock().unwrap();
        let available_waiters = waiters.len();
        let to_wake = min(requested_wake_count, available_waiters);
        let mut woken = 0;
        for _ in 0..to_wake {
            match waiters.pop_front() {
                Some(waiter) => {
                    waiter.unpark();
                    woken += 1;
                }
                None => break,
            }
        }
        woken
    }

    pub fn pending(&self) -> usize {
        let waiters = self.waiters.lock().unwrap();
        waiters.len()
    }

    //Debug fix: wait_ev must remove the current thread from the waiters list after waking up, to avoid leaving stale waiters.
    pub fn wait_ev<T>(
        &self,
        state: &Mutex<T>,
        mut condition: impl FnMut(&T) -> Option<bool>,
    ) -> bool {
        let current_thread = thread::current();
        let current_thread_id = current_thread.id();

        loop {
            {
                let state_guard = state.lock().unwrap();
                if let Some(result) = condition(&state_guard) {
                    self.remove_waiter_by_id(&current_thread_id);
                    return result;
                }
            }

            {
                let mut waiters = self.waiters.lock().unwrap();
                if !waiters
                    .iter()
                    .any(|waiter| waiter.id() == current_thread_id)
                {
                    waiters.push_back(current_thread.clone());
                }
            }

            thread::park();
            self.remove_waiter_by_id(&current_thread_id);
        }
    }

    pub fn wait_events<T>(
        queues: &[&SyncQueue],
        state: &Mutex<T>,
        mut condition: impl FnMut(&T) -> Option<bool>,
    ) -> bool {
        let current_thread = thread::current();
        let current_thread_id = current_thread.id();

        loop {
            {
                let state_guard = state.lock().unwrap();
                if let Some(result) = condition(&state_guard) {
                    // Debug fix: remove this thread from every queue it registered with.
                    Self::remove_waiter_from_all(queues, &current_thread_id);
                    return result;
                }
            }
            for queue in queues {
                let mut waiters = queue.waiters.lock().unwrap();
                if !waiters
                    .iter()
                    .any(|waiter| waiter.id() == current_thread_id)
                {
                    waiters.push_back(current_thread.clone());
                }
            }
            thread::park();
            // Debug fix: waking from one queue must not leave stale waiters in the others.
            Self::remove_waiter_from_all(queues, &current_thread_id);
        }
    }

    //Note: whats this? we did not get mutex guard, so we cannot actually release the mutex.
    pub fn wait_guard<T>(&self, state: &Mutex<T>) {
        {
            let mut waiters = self.waiters.lock().unwrap();
            waiters.push_back(thread::current());
        }
        drop(state.lock().unwrap());
        thread::park();
    }

    pub fn wait_timeout<T>(&self, state: &Mutex<T>, timeout: Duration) -> bool {
        let current_thread = thread::current();
        let current_thread_id = current_thread.id();
        {
            let mut waiters = self.waiters.lock().unwrap();
            waiters.push_back(current_thread);
        }
        drop(state.lock().unwrap());
        thread::park_timeout(timeout);
        // Debug fix: timeout return must remove only the current thread's waiter.
        let was_still_waiting = self.remove_waiter_by_id(&current_thread_id);
        !was_still_waiting
    }

    pub fn reg_epoll(&self, task_id: usize, epfd: usize, fd: usize) {
        self.epoll_registrations
            .lock()
            .unwrap()
            .push_back(RegEp { task_id, epfd, fd });
    }

    pub fn unreg_epoll(&self, task_id: usize, epfd: usize, fd: usize) -> bool {
        let mut epoll_registrations = self.epoll_registrations.lock().unwrap();
        for index in 0..epoll_registrations.len() {
            if epoll_registrations[index].task_id == task_id
                && epoll_registrations[index].epfd == epfd
                && epoll_registrations[index].fd == fd
            {
                epoll_registrations.remove(index);
                return true;
            }
        }
        false
    }
}

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

/// Per-address futex wait bucket.
///
/// Each waiter records the futex address, the parked Rust thread, and a wake flag
/// used to distinguish a real wake from timeout-style returns.
pub struct FutexBucket {
    waiters: Mutex<VecDeque<(usize, thread::Thread, Arc<AtomicBool>)>>,
}

impl FutexBucket {
    pub fn new() -> Self {
        Self {
            waiters: Mutex::new(VecDeque::new()),
        }
    }

    pub fn wait(
        &self,
        addr: usize,
        expected: u32,
        val: &AtomicU32,
        timeout: Option<Duration>,
    ) -> Result<(), &'static str> {
        let wake_flag = Arc::new(AtomicBool::new(false));
        if val.load(Ordering::SeqCst) != expected {
            return Err("changed");
        }
        {
            let mut waiters = self.waiters.lock().unwrap();
            waiters.push_back((addr, thread::current(), wake_flag.clone()));
        }
        if let Some(duration) = timeout {
            thread::park_timeout(duration);
        } else {
            thread::park();
        }
        if wake_flag.load(Ordering::Relaxed) {
            Ok(())
        } else {
            self.remove_waiter(addr, &wake_flag);
            Err("timeout")
        }
    }

    // Debug fix: wait() should remove the waiter when timeouts occur.
    fn remove_waiter(&self, addr: usize, wake_flag: &Arc<AtomicBool>) -> bool {
        let mut waiters = self.waiters.lock().unwrap();
        let before = waiters.len();
        waiters.retain(|(wait_addr, _, waiter_flag)| {
            !(*wait_addr == addr && Arc::ptr_eq(waiter_flag, wake_flag))
        });
        waiters.len() != before
    }

    pub fn wake(&self, addr: usize, count: usize) -> usize {
        let mut waiters = self.waiters.lock().unwrap();
        let mut woken = 0;
        waiters.retain(|(wait_addr, waiter, wake_flag)| {
            if *wait_addr == addr && woken < count {
                wake_flag.store(true, Ordering::Relaxed);
                waiter.unpark();
                woken += 1;
                false
            } else {
                true
            }
        });
        woken
    }

    pub fn requeue(&self, src: usize, dst: usize, wake_n: usize, move_n: usize) -> usize {
        let mut waiters = self.waiters.lock().unwrap();
        let (mut woken, mut moved) = (0, 0);
        for entry in waiters.iter_mut() {
            if entry.0 == src {
                if woken < wake_n {
                    entry.2.store(true, Ordering::Relaxed);
                    entry.1.unpark();
                    woken += 1;
                } else if moved < move_n {
                    entry.0 = dst;
                    moved += 1;
                }
            }
        }
        waiters.retain(|(_, _, wake_flag)| !wake_flag.load(Ordering::Relaxed));
        woken
    }

    pub fn pending_at(&self, addr: usize) -> usize {
        self.waiters
            .lock()
            .unwrap()
            .iter()
            .filter(|(wait_addr, _, _)| *wait_addr == addr)
            .count()
    }
}

/// Simple global futex table used by tests that do not need per-task buckets.
///
/// Each entry stores a futex address and the parked Rust thread waiting on that
/// address. `FutexBucket` is the richer per-address version used elsewhere.
pub struct FutexTable {
    table: Mutex<VecDeque<(usize, thread::Thread)>>,
}

impl FutexTable {
    pub fn new() -> Self {
        Self {
            table: Mutex::new(VecDeque::new()),
        }
    }

    pub fn ftx_wait(&self, addr: usize, expected: u32, val: &AtomicU32) -> bool {
        // Debug fix: unify lock usage
        let mut waiters = self.table.lock().unwrap();
        if val.load(Ordering::SeqCst) != expected {
            return false;
        }
        waiters.push_back((addr, thread::current()));
        drop(waiters);
        thread::park();
        true
    }

    pub fn ftx_wake(&self, addr: usize, count: usize) -> usize {
        // Debug fix: waking zero waiters must return zero and must not unpark anyone.
        let mut waiters = self.table.lock().unwrap();
        let mut woken = 0usize;
        let mut cursor = 0;
        while cursor < waiters.len() && woken < count {
            if waiters[cursor].0 == addr {
                let entry = waiters.remove(cursor).unwrap();
                entry.1.unpark();
                woken += 1;
            } else {
                cursor += 1;
            }
        }
        woken
    }

    pub fn ftx_requeue(
        &self,
        src_addr: usize,
        dst_addr: usize,
        wake_n: usize,
        move_n: usize,
    ) -> usize {
        let mut waiters = self.table.lock().unwrap();
        let mut woken = 0;
        let mut moved = 0;
        let mut cursor = 0;
        while cursor < waiters.len() {
            if waiters[cursor].0 == src_addr {
                if woken < wake_n {
                    let (_, waiter) = waiters.remove(cursor).unwrap();
                    waiter.unpark();
                    woken += 1;
                } else if moved < move_n {
                    waiters[cursor].0 = dst_addr;
                    moved += 1;
                    cursor += 1;
                } else {
                    cursor += 1;
                }
            } else {
                cursor += 1;
            }
        }
        woken
    }
}

/// Converts a physical address to the kernel's direct-map virtual address.
///
/// This helper intentionally only applies the fixed offset; callers are
/// responsible for validating the address range before mapping or dereferencing.
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

/// Computes the TCP checksum over the IPv4 pseudo-header and TCP payload.
pub fn tcp_checksum(source_ip: u32, destination_ip: u32, payload: &[u8]) -> u16 {
    let mut checksum_data = build_pseudo_header(source_ip, destination_ip, 6, payload.len() as u16);
    checksum_data.extend_from_slice(payload);
    compute_inet_checksum(&checksum_data)
}

/// Parses the fixed fields needed by the network simulation from an IPv4 packet.
///
/// Returns source address, destination address, protocol, and total length.
pub fn parse_ipv4_header(packet: &[u8]) -> Option<(u32, u32, u8, u16)> {
    if packet.len() < 20 {
        return None;
    }

    let version = packet[0] >> 4;
    if version != 4 {
        return None;
    }

    let header_len = ((packet[0] & 0x0F) as usize).checked_mul(4)?;
    if header_len < 20 || packet.len() < header_len {
        return None;
    }

    let total_len = u16::from_be_bytes([packet[2], packet[3]]);
    let protocol = packet[9];
    let src_ip = u32::from_be_bytes([packet[12], packet[13], packet[14], packet[15]]);
    let dst_ip = u32::from_be_bytes([packet[16], packet[17], packet[18], packet[19]]);
    Some((src_ip, dst_ip, protocol, total_len))
}

/// Builds the 12-byte IPv4 pseudo-header used by TCP/UDP checksums.
pub fn build_pseudo_header(src_ip: u32, dst_ip: u32, protocol: u8, length: u16) -> Vec<u8> {
    let mut header = Vec::with_capacity(12);
    header.extend_from_slice(&src_ip.to_be_bytes());
    header.extend_from_slice(&dst_ip.to_be_bytes());
    header.push(0);
    header.push(protocol);
    header.extend_from_slice(&length.to_be_bytes());
    header
}

/// Computes the standard one's-complement Internet checksum.
pub fn compute_inet_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut chunks = data.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(&last_byte) = chunks.remainder().first() {
        sum += (last_byte as u32) << 8;
    }

    while sum > 0xFFFF {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}

/// Bitmap-style physical frame pool.
///
/// Each slot represents one physical frame: `true` means free and `false` means
/// allocated.
///
/// Note(IMPORTANT): FramePool should store PgFrame instead of just a bitmap, but this requires too much refactoring, so we just keep it for now.
pub struct FramePool {
    slots: Mutex<Vec<bool>>,
    capacity: usize,
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
#[derive(Debug, Clone, Copy)]
pub struct FdOpt {
    pub rd: bool,
    pub wr: bool,
    pub ap: bool,
    pub nb: bool,
}

impl Default for FdOpt {
    fn default() -> Self {
        Self {
            rd: true,
            wr: false,
            ap: false,
            nb: false,
        }
    }
}

/// Shared open-file-description state for duplicated file handles.
///
/// Multiple `FHandle` values can point at the same `FdState`, so offset and
/// options are protected by an `RwLock` and shared through `Arc`.
struct FdState {
    off: u64,
    opt: FdOpt,
    flk: u8, //USELESS...
}

impl FdState {
    fn create(opt: FdOpt) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(FdState {
            off: 0,
            opt,
            flk: 0,
        }))
    }
}

/// In-memory file handle used by the kernel simulation.
///
/// `data` stores the file bytes, while `desc` is the shared open-file
/// description. Duplicated handles therefore share the same file offset and
/// access options, matching the usual `dup` behavior.
pub struct FHandle {
    pub path: String,
    pub data: Arc<Mutex<Vec<u8>>>,
    desc: Arc<RwLock<FdState>>,
    pub pipe: bool,
    pub cloexec: bool,
}

/// Seek origin used by `FHandle::seek`.
#[derive(Debug)]
pub enum FSeek {
    Start(u64),
    End(i64),
    Cur(i64),
}

impl FHandle {
    pub fn new(path: &str, opt: FdOpt, pipe: bool, cloexec: bool) -> Self {
        Self {
            path: path.to_string(),
            data: Arc::new(Mutex::new(Vec::new())),
            desc: FdState::create(opt),
            pipe,
            cloexec,
        }
    }

    pub fn with_data(path: &str, opt: FdOpt, initial_data: Vec<u8>) -> Self {
        Self {
            path: path.to_string(),
            data: Arc::new(Mutex::new(initial_data)),
            desc: FdState::create(opt),
            pipe: false,
            cloexec: false,
        }
    }

    pub fn dup(&self, cloexec: bool) -> Self {
        FHandle {
            path: self.path.clone(),
            data: self.data.clone(),
            desc: self.desc.clone(),
            pipe: self.pipe,
            cloexec,
        }
    }

    //Debug fix: add more supports (confused).
    pub fn set_opt(&self, arg: usize) {
        let mut state = self.desc.write().unwrap();
        state.opt.nb = (arg & O_NONBLOCK) != 0;
        state.opt.ap = (arg & O_APPEND) != 0;
    }

    pub fn get_opt(&self) -> FdOpt {
        self.desc.read().unwrap().opt
    }

    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        let offset = self.desc.read().unwrap().off as usize;
        let bytes_read = self.read_at(offset, buffer)?;
        self.desc.write().unwrap().off += bytes_read as u64;
        Ok(bytes_read)
    }

    pub fn read_at(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, &'static str> {
        if !self.desc.read().unwrap().opt.rd {
            return Err("ebadf");
        }
        let contents = self.data.lock().unwrap();
        if offset >= contents.len() {
            return Ok(0);
        }
        let bytes_to_copy = min(buffer.len(), contents.len() - offset);
        buffer[..bytes_to_copy].copy_from_slice(&contents[offset..offset + bytes_to_copy]);
        Ok(bytes_to_copy)
    }

    pub fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        let write_offset = {
            let state = self.desc.read().unwrap();
            if state.opt.ap {
                self.data.lock().unwrap().len() as u64
            } else {
                state.off
            }
        } as usize;
        let bytes_written = self.write_at(write_offset, buffer)?;
        // Debug fix: append writes must advance the descriptor offset to the
        // end of the actual append, not from the old descriptor offset.
        let new_offset = write_offset.checked_add(bytes_written).ok_or("eoverflow")?;
        self.desc.write().unwrap().off = new_offset as u64;
        Ok(bytes_written)
    }

    pub fn write_at(&self, offset: usize, buffer: &[u8]) -> Result<usize, &'static str> {
        if !self.desc.read().unwrap().opt.wr {
            return Err("ebadf");
        }
        let mut contents = self.data.lock().unwrap();
        // Debug fix: checked arithmetic prevents `off + len` from wrapping
        // before resize and slice bounds are computed.
        let end_offset = offset.checked_add(buffer.len()).ok_or("einval")?;
        if end_offset > contents.len() {
            contents.resize(end_offset, 0);
        }
        contents[offset..end_offset].copy_from_slice(buffer);
        Ok(buffer.len())
    }

    pub fn seek(&self, pos: FSeek) -> Result<u64, &'static str> {
        let mut state = self.desc.write().unwrap();
        let next_offset = match pos {
            FSeek::Start(offset) => offset as i128,
            FSeek::End(delta) => self.data.lock().unwrap().len() as i128 + delta as i128,
            FSeek::Cur(delta) => state.off as i128 + delta as i128,
        };
        // Debug fix: keep the calculation signed until negative results have
        // been rejected, avoiding wraparound into a huge u64 offset.
        if next_offset < 0 || next_offset > u64::MAX as i128 {
            return Err("einval");
        }
        state.off = next_offset as u64;
        Ok(state.off)
    }

    pub fn transfer(
        &self,
        direction: u8,
        offset: Option<usize>,
        read_buffer: Option<&mut [u8]>,
        write_buffer: Option<&[u8]>,
    ) -> Result<usize, &'static str> {
        if direction & 1 != 0 {
            match (offset, read_buffer) {
                (Some(read_offset), Some(buffer)) => self.read_at(read_offset, buffer),
                (None, Some(buffer)) => self.read(buffer),
                _ => Err("einval"),
            }
        } else {
            match (offset, write_buffer) {
                (Some(write_offset), Some(buffer)) => self.write_at(write_offset, buffer),
                (None, Some(buffer)) => self.write(buffer),
                _ => Err("einval"),
            }
        }
    }

    pub fn set_len(&self, new_len: u64) -> Result<(), &'static str> {
        if !self.desc.read().unwrap().opt.wr {
            return Err("ebadf");
        }
        self.data.lock().unwrap().resize(new_len as usize, 0);
        Ok(())
    }

    //Note: a lot of useless functions below ...
    pub fn sync_all(&self) -> Result<(), &'static str> {
        Ok(())
    }

    pub fn sync_data(&self) -> Result<(), &'static str> {
        Ok(())
    }

    pub fn metadata_sz(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    pub fn lookup(&self, _path: &str, _depth: usize) -> Result<(), &'static str> {
        Ok(())
    }

    pub fn read_entry(&self) -> Result<String, &'static str> {
        let mut state = self.desc.write().unwrap();
        if !state.opt.rd {
            return Err("ebadf");
        }
        let entry_offset = state.off;
        state.off += 1;
        Ok(format!("entry_{}", entry_offset))
    }

    pub fn poll_status(&self) -> (bool, bool, bool) {
        (true, true, false)
    }

    pub fn io_ctl(&self, _cmd: u32, _arg: usize) -> Result<usize, &'static str> {
        Ok(0)
    }

    pub fn mmap(&self, _start: usize, _end: usize, _off: usize) -> Result<(), &'static str> {
        Ok(())
    }

    pub fn inode_ref(&self) -> Arc<Mutex<Vec<u8>>> {
        self.data.clone()
    }

    pub fn advise_readahead(&self, offset: usize, length: usize) -> Result<(), &'static str> {
        if length == 0 {
            return Ok(());
        }

        offset.checked_add(length).ok_or("einval")?;

        let contents = self.data.lock().unwrap();
        if offset >= contents.len() {
            return Ok(());
        }

        Ok(())
    }

    pub fn fallocate(&self, offset: usize, length: usize) -> Result<(), &'static str> {
        if !self.desc.read().unwrap().opt.wr {
            return Err("ebadf");
        }
        let mut contents = self.data.lock().unwrap();
        let required_len = offset.checked_add(length).ok_or("einval")?;
        if required_len > contents.len() {
            contents.resize(required_len, 0);
        }
        Ok(())
    }

    pub fn splice_to(&self, dst: &FHandle, count: usize) -> Result<usize, &'static str> {
        let source_offset = self.desc.read().unwrap().off;
        let source_data = self.data.lock().unwrap();
        if source_offset as usize >= source_data.len() {
            return Ok(0);
        }
        let available = source_data.len() - source_offset as usize;
        let bytes_to_splice = min(count, available);
        let chunk: Vec<u8> =
            source_data[source_offset as usize..source_offset as usize + bytes_to_splice].to_vec();
        drop(source_data);
        self.desc.write().unwrap().off += bytes_to_splice as u64;
        dst.write(&chunk)
    }
}

impl fmt::Debug for FHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let state = self.desc.read().unwrap();
        f.debug_struct("FH")
            .field("off", &state.off)
            .field("path", &self.path)
            .finish()
    }
}

/// Direction of one pipe endpoint.
#[derive(Clone, PartialEq)]
pub enum PipeDir {
    Rd,
    Wr,
}

/// Shared buffer and endpoint counters for a pipe pair.
///
/// `readers` and `writers` track live endpoints by direction, while `ends`
/// tracks the total endpoint count for compatibility with the original model.
pub struct PipeBuf {
    pub buf: VecDeque<u8>,
    pub bus: EventBus,
    pub ends: i32,
    readers: i32,
    writers: i32,
}

/// One read or write endpoint of a pipe.
pub struct PipeNode {
    data: Arc<Mutex<PipeBuf>>,
    dir: PipeDir,
}

impl Clone for PipeNode {
    fn clone(&self) -> Self {
        let mut pipe = self.data.lock().unwrap();
        // Debug fix: cloning an endpoint must increment the counters that Drop
        // later decrements, otherwise dropping a duplicate closes the original.
        pipe.ends += 1;
        match &self.dir {
            PipeDir::Rd => pipe.readers += 1,
            PipeDir::Wr => pipe.writers += 1,
        }
        drop(pipe);
        Self {
            data: self.data.clone(),
            dir: self.dir.clone(),
        }
    }
}

impl Drop for PipeNode {
    fn drop(&mut self) {
        let mut pipe = self.data.lock().unwrap();
        pipe.ends = pipe.ends.saturating_sub(1);
        match &self.dir {
            PipeDir::Rd => pipe.readers = pipe.readers.saturating_sub(1),
            PipeDir::Wr => pipe.writers = pipe.writers.saturating_sub(1),
        }
        if pipe.readers == 0 || pipe.writers == 0 {
            pipe.bus.set(EventFlag::CLOSED);
        }
    }
}

impl PipeNode {
    pub fn pair() -> (PipeNode, PipeNode) {
        let inner = PipeBuf {
            buf: VecDeque::new(),
            bus: EventBus::default(),
            ends: 2,
            readers: 1,
            writers: 1,
        };
        let shared_pipe = Arc::new(Mutex::new(inner));
        (
            PipeNode {
                data: shared_pipe.clone(),
                dir: PipeDir::Rd,
            },
            PipeNode {
                data: shared_pipe,
                dir: PipeDir::Wr,
            },
        )
    }

    pub fn can_read(&self) -> bool {
        if self.dir != PipeDir::Rd {
            return false;
        }
        let pipe = self.data.lock().unwrap();
        !pipe.buf.is_empty() || pipe.writers == 0
    }

    pub fn can_write(&self) -> bool {
        if self.dir != PipeDir::Wr {
            return false;
        }
        self.data.lock().unwrap().readers > 0
    }

    pub fn read_at(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.dir != PipeDir::Rd {
            return Ok(0);
        }

        let mut pipe = self.data.lock().unwrap();
        if pipe.buf.is_empty() && pipe.writers > 0 {
            return Err("again");
        }

        let bytes_to_read = min(buffer.len(), pipe.buf.len());
        for slot in buffer.iter_mut().take(bytes_to_read) {
            *slot = pipe.buf.pop_front().unwrap();
        }
        if pipe.buf.is_empty() {
            pipe.bus.clear(EventFlag::READABLE);
        }
        Ok(bytes_to_read)
    }

    pub fn write_at(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        if self.dir != PipeDir::Wr {
            return Ok(0);
        }

        let mut pipe = self.data.lock().unwrap();
        // Debug fix: writing to a pipe with no readers must fail instead of
        // buffering bytes that no endpoint can read.
        if pipe.readers == 0 {
            return Err("epipe");
        }

        for &byte in buffer {
            pipe.buf.push_back(byte);
        }
        pipe.bus.set(EventFlag::READABLE);
        Ok(buffer.len())
    }

    pub fn poll(&self) -> (bool, bool, bool) {
        (self.can_read(), self.can_write(), false)
    }
}

/// File-descriptor object variants used by the simulation. Unifies regular files, pipes, and epoll instances under one type for the kernel.
///
/// Note: rewrite for simplicity, remove a lot of useless code.
pub enum FLike {
    File(FHandle),
    Pipe(PipeNode),
    Ep(EpInst),
}

impl FLike {
    pub fn dup(&self, cloexec: bool) -> FLike {
        match self {
            FLike::File(file) => FLike::File(file.dup(cloexec)),
            FLike::Pipe(pipe) => FLike::Pipe(pipe.clone()),
            // Debug fix: epoll duplicates must share the full instance state,
            // including the registration map, ready set, and control-change set.
            FLike::Ep(epoll) => FLike::Ep(epoll.clone()),
        }
    }

    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        if buffer.is_empty() {
            return Ok(0);
        }

        match self {
            FLike::File(file) => file.read(buffer),
            FLike::Pipe(pipe) => pipe.read_at(buffer),
            FLike::Ep(_) => Err("enosys"),
        }
    }

    pub fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        if buffer.is_empty() {
            return Ok(0);
        }

        match self {
            FLike::File(file) => file.write(buffer),
            FLike::Pipe(pipe) => pipe.write_at(buffer),
            FLike::Ep(_) => Err("enosys"),
        }
    }

    //additional controls
    pub fn io_ctl(&self, request: usize, arg: usize) -> Result<usize, &'static str> {
        match self {
            FLike::File(file) => match request as u32 {
                0..=0xFF => Ok(0),
                _ => file.io_ctl(request as u32, arg),
            },
            FLike::Pipe(_) => match request {
                FIONBIO => Ok(0),
                _ => Err("enotty"),
            },
            FLike::Ep(_) => Err("enosys"),
        }
    }

    pub fn mmap_fl(&self, start: usize, end: usize, offset: usize) -> Result<(), &'static str> {
        if start >= end {
            return Err("einval");
        }
        let len = end.checked_sub(start).ok_or("einval")?;
        // Debug fix: page-count rounding for huge ranges must not overflow.
        len.checked_add(PAGE_SZ - 1)
            .map(|rounded_len| rounded_len / PAGE_SZ)
            .ok_or("einval")?;

        match self {
            FLike::File(file) => file.mmap(start, end, offset),
            _ => Err("enosys"),
        }
    }

    //returns ready state
    pub fn poll(&self) -> (bool, bool, bool) {
        match self {
            FLike::File(file) => {
                let options = file.desc.read().unwrap().opt;
                let error = file.path.is_empty() && file.data.lock().unwrap().is_empty();
                (options.rd, options.wr, error)
            }
            FLike::Pipe(pipe) => pipe.poll(),
            FLike::Ep(epoll) => {
                let has_ready = !epoll.ready.lock().unwrap().is_empty();
                (has_ready, false, false)
            }
        }
    }
}

impl fmt::Debug for FLike {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FLike::File(handle) => write!(f, "F({:?})", handle),
            FLike::Pipe(_) => write!(f, "P"),
            FLike::Ep(_) => write!(f, "E"),
        }
    }
}

/// Looks like a file node, test-use only for now.
pub struct PseudoNode {
    pub content: Vec<u8>,
    pub ftype: u8,
}

impl PseudoNode {
    pub fn new(content: &str, file_type: u8) -> Self {
        Self {
            content: content.as_bytes().to_vec(),
            ftype: file_type,
        }
    }

    pub fn read_at(&self, offset: usize, buffer: &mut [u8]) -> usize {
        if offset >= self.content.len() {
            return 0;
        }
        let bytes_to_read = min(self.content.len() - offset, buffer.len());
        buffer[..bytes_to_read].copy_from_slice(&self.content[offset..offset + bytes_to_read]);
        bytes_to_read
    }

    pub fn write_at(&self, _offset: usize, _buffer: &[u8]) -> Result<usize, &'static str> {
        Err("nosup")
    }

    pub fn metadata_sz(&self) -> usize {
        self.content.len()
    }
}

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

/// Termios-compatible terminal configuration exposed through ioctl-style calls.
///
/// Note: just copy from original code, not used.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrmIO {
    pub iflag: u32,
    pub oflag: u32,
    pub cflag: u32,
    pub lflag: u32,
    pub line: u8,
    pub cc: [u8; 32],
    pub ispeed: u32,
    pub ospeed: u32,
}

impl Default for TrmIO {
    fn default() -> Self {
        Self {
            iflag: 0o66402,
            oflag: 0o5,
            cflag: 0o2277,
            lflag: 0o105073,
            line: 0,
            cc: [
                3, 28, 127, 21, 4, 0, 1, 0, 17, 19, 26, 255, 18, 15, 23, 22, 255, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            ispeed: 0,
            ospeed: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct WinSz {
    pub row: u16,
    pub col: u16,
    pub xpx: u16,
    pub ypx: u16,
}

/// Blocking byte channel used by terminal-style producers and consumers.
///
/// `buf` stores bytes in a circular buffer, `guard` serializes receiving paths,
/// `wq` tracks blocked receivers, and `shut` marks EOF/closed state.
///
/// Note: simplify a lot of useless inlines.
pub struct Channel {
    pub buf: Mutex<CircBuf>,
    pub guard: Spin,
    pub wq: SyncQueue,
    pub shut: AtomicBool,
}

impl Channel {
    const MAX_CAPACITY: usize = 1 << 20;

    pub fn new(capacity: usize) -> Self {
        let effective_capacity = capacity.clamp(1, Self::MAX_CAPACITY);
        Self {
            buf: Mutex::new(CircBuf::new(effective_capacity)),
            guard: Spin::new(),
            wq: SyncQueue::new(),
            shut: AtomicBool::new(false),
        }
    }

    pub fn recv(&self) -> Option<u8> {
        loop {
            self.guard.acquire();

            let mut ring = self.buf.lock().unwrap();
            if let Some(value) = ring.pop() {
                drop(ring);
                self.guard.release();
                return Some(value);
            }

            if self.shut.load(Ordering::Acquire) {
                drop(ring);
                self.guard.release();
                return None;
            }

            let queued = self
                .wq
                .enqueue_current_thread_if(|| !self.shut.load(Ordering::Acquire));
            drop(ring);
            self.guard.release();

            if queued {
                thread::park();
            } else {
                return None;
            }
        }
    }

    pub fn send(&self, value: u8) -> bool {
        if self.shut.load(Ordering::Acquire) {
            return false;
        }

        let written = {
            let mut ring = self.buf.lock().unwrap();
            // Debug fix: closed channels reject sends without mutating buffer depth.
            if self.shut.load(Ordering::Acquire) {
                false
            } else {
                ring.push(value)
            }
        };

        if written {
            self.wq.signal_n(1);
        }
        written
    }

    pub fn close(&self) {
        self.shut.store(true, Ordering::Release);
        self.wq.broadcast();
    }

    pub fn try_recv(&self) -> Option<u8> {
        if !self.guard.try_acquire() {
            return None;
        }

        let result = self.buf.lock().unwrap().pop();
        self.guard.release();
        result
    }

    pub fn send_batch(&self, data: &[u8]) -> usize {
        if self.shut.load(Ordering::Acquire) {
            return 0;
        }

        let written = {
            let mut ring = self.buf.lock().unwrap();
            // Debug fix: a close observed during the locked write path rejects the batch.
            if self.shut.load(Ordering::Acquire) {
                0
            } else {
                ring.fill_from(data)
            }
        };

        if written > 0 {
            // Debug fix: wake as many receivers as the batch made newly readable.
            self.wq.signal_n(written);
        }
        written
    }

    pub fn depth(&self) -> usize {
        self.buf.lock().unwrap().len()
    }

    pub fn drain_all(&self) -> Vec<u8> {
        let mut result = Vec::new();
        self.buf.lock().unwrap().drain_to(&mut result, usize::MAX);
        result
    }

    pub fn is_closed(&self) -> bool {
        self.shut.load(Ordering::Acquire)
    }

    pub fn remaining_capacity(&self) -> usize {
        self.buf.lock().unwrap().remaining()
    }
}

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

/// One mount mapping from a path prefix to a backing target.
#[derive(Clone, Debug)]
pub struct MountEntry {
    pub prefix: String,
    pub target: String,
}

/// Ordered mount table.
///
/// Entries are sorted by descending prefix length so the longest matching
/// mount point wins during resolution.
///
/// Fix: derived a helper function to canonicalize slashes, and remove some redundant code.
/// Note: cannot make sure if resolve is correct.
pub struct MountTable {
    pub entries: RwLock<Vec<MountEntry>>,
}

impl MountTable {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    pub fn bind(&self, prefix: &str, target: &str) {
        let mut entries = self.entries.write().unwrap();
        let already_bound = entries
            .iter()
            .any(|entry| entry.prefix == prefix && entry.target == target);
        if already_bound {
            return;
        }

        entries.push(MountEntry {
            prefix: prefix.to_string(),
            target: target.to_string(),
        });
        entries.sort_by(|left, right| right.prefix.len().cmp(&left.prefix.len()));
    }

    fn prefix_matches(prefix: &str, path: &str) -> bool {
        if prefix == "/" {
            return path.starts_with('/');
        }
        if !path.starts_with(prefix) {
            return false;
        }
        // Debug fix: `/mnt` must match `/mnt/file`, but not `/mnted/file`.
        path.len() == prefix.len() || path.as_bytes().get(prefix.len()) == Some(&b'/')
    }

    //remove redudant slashes.
    fn canonicalize_slashes(path: &str) -> String {
        let mut canonical = String::with_capacity(path.len());
        let mut previous_was_slash = false;
        for ch in path.chars() {
            if ch == '/' {
                if !previous_was_slash {
                    canonical.push(ch);
                }
                previous_was_slash = true;
            } else {
                canonical.push(ch);
                previous_was_slash = false;
            }
        }
        if canonical.is_empty() {
            path.to_string()
        } else {
            canonical
        }
    }

    pub fn resolve(&self, path: &str) -> Result<String, &'static str> {
        match self.find_mount(path) {
            Some(entry) => {
                let remaining_path = &path[entry.prefix.len()..];
                let resolved_suffix = self.resolve(remaining_path)?;
                let mut result =
                    String::with_capacity(entry.target.len() + 1 + resolved_suffix.len());
                result.push_str(&entry.target);
                result.push(':');
                result.push_str(&resolved_suffix);
                Ok(result)
            }
            None => Ok(Self::canonicalize_slashes(path)),
        }
    }

    pub fn unmount(&self, prefix: &str) -> bool {
        let mut entries = self.entries.write().unwrap();
        let previous_len = entries.len();
        entries.retain(|entry| entry.prefix != prefix);
        entries.len() < previous_len
    }

    pub fn list_mounts(&self) -> Vec<(String, String)> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .map(|entry| (entry.prefix.clone(), entry.target.clone()))
            .collect()
    }

    pub fn find_mount(&self, path: &str) -> Option<MountEntry> {
        let entries = self.entries.read().unwrap();
        let mut best_match: Option<&MountEntry> = None;
        let mut best_prefix_len = 0usize;

        for entry in entries.iter() {
            let prefix_len = entry.prefix.len();
            if prefix_len == 0 {
                continue;
            }
            if Self::prefix_matches(&entry.prefix, path) && prefix_len > best_prefix_len {
                best_prefix_len = prefix_len;
                best_match = Some(entry);
            }
        }

        best_match.cloned()
    }

    pub fn mount_count(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn has_prefix(&self, prefix: &str) -> bool {
        self.entries
            .read()
            .unwrap()
            .iter()
            .any(|entry| entry.prefix.as_bytes() == prefix.as_bytes())
    }
}

/// One pending disk I/O request.
///
/// `block` is the target block number, `write` distinguishes write from read,
/// and `priority` is stored for scheduler policy experiments.
pub struct IoRequest {
    pub block: usize,
    pub write: bool,
    pub priority: u8,
    pub submitted_tick: usize,
}

/// Simple disk I/O scheduler queue.
///
/// The dispatch policy follows the current head position and direction, while
/// `merged` tracks adjacent requests that were coalesced.
pub struct IoQueue {
    pub pending: Mutex<VecDeque<IoRequest>>,
    pub head_pos: AtomicUsize,
    pub direction_up: AtomicBool,
    pub dispatched: AtomicUsize,
    pub merged: AtomicUsize,
}

impl IoQueue {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
            head_pos: AtomicUsize::new(0),
            direction_up: AtomicBool::new(true),
            dispatched: AtomicUsize::new(0),
            merged: AtomicUsize::new(0),
        }
    }

    pub fn submit(&self, block_id: usize, write: bool, priority: u8) {
        let request = IoRequest {
            block: block_id,
            write,
            priority,
            submitted_tick: CLK.load(Ordering::Relaxed),
        };
        let mut queue = self.pending.lock().unwrap();
        queue.push_back(request);
    }

    pub fn submit_batch(&self, requests: &[(usize, bool, u8)]) -> usize {
        let mut queue = self.pending.lock().unwrap();
        let mut submitted_count = 0;
        for &(block_id, write, priority) in requests {
            let request = IoRequest {
                block: block_id,
                write,
                priority,
                submitted_tick: CLK.load(Ordering::Relaxed),
            };
            queue.push_back(request);
            submitted_count += 1;
        }
        let depth = queue.len();
        let should_merge = depth > IOQUEUE_DEPTH;
        drop(queue);

        // Debug fix: do not call merge_adjacent while still holding pending.
        if should_merge {
            self.merge_adjacent();
        }
        submitted_count
    }

    /// Note: currently, the dispatch policy is alike a simple SCAN algorithm, however, it deals backward requests in a very strange way,
    /// also, merge_adjacent confusing in the context, since it removes a request from the queue completely.
    pub fn dispatch(&self) -> Option<(usize, bool)> {
        let mut queue = self.pending.lock().unwrap();
        if queue.is_empty() {
            return None;
        }

        let head_position = self.head_pos.load(Ordering::Relaxed);
        let going_up = self.direction_up.load(Ordering::Relaxed);
        let mut best_index = 0;
        let mut best_distance = usize::MAX;

        for (index, request) in queue.iter().enumerate() {
            let distance = if going_up {
                if request.block >= head_position {
                    request.block - head_position
                } else {
                    usize::MAX / 2 + request.block
                }
            } else if request.block <= head_position {
                head_position - request.block
            } else {
                usize::MAX / 2 + head_position
            };

            if distance < best_distance {
                best_distance = distance;
                best_index = index;
            }
        }

        let request = queue.remove(best_index)?;
        self.head_pos.store(request.block, Ordering::Relaxed);
        if going_up && request.block >= head_position {
            if queue.iter().all(|queued| queued.block < request.block) {
                self.direction_up.store(false, Ordering::Relaxed);
            }
        } else if !going_up && request.block <= head_position {
            if queue.iter().all(|queued| queued.block > request.block) {
                self.direction_up.store(true, Ordering::Relaxed);
            }
        }
        self.dispatched.fetch_add(1, Ordering::Relaxed);
        Some((request.block, request.write))
    }

    pub fn merge_adjacent(&self) -> usize {
        let mut queue = self.pending.lock().unwrap();
        let mut merged_count = 0;
        let mut index = 0;
        while index + 1 < queue.len() {
            // Debug fix: checked_add avoids overflow for the final block id.
            if queue[index].block.checked_add(1) == Some(queue[index + 1].block)
                && queue[index].write == queue[index + 1].write
            {
                queue.remove(index + 1);
                merged_count += 1;
            } else {
                index += 1;
            }
        }
        self.merged.fetch_add(merged_count, Ordering::Relaxed);
        merged_count
    }

    pub fn depth(&self) -> usize {
        self.pending.lock().unwrap().len()
    }
}

/// Simulated block device with optional journal fallback.
///
/// `errs` is a countdown of remaining synthetic I/O failures that can be tried; `usize::MAX`
/// means persistent failure. `ops` counts attempted operations.
///
/// Note(IMPORTANT): this struct is full of errors, but we do not know what it should behave based on current testcases.
pub struct Disk {
    pub errs: AtomicUsize,
    pub ops: AtomicUsize,
    pub label: String,
    pub journal: Option<Arc<Disk>>,
}

impl Disk {
    pub fn new(label: &str) -> Self {
        Self {
            errs: AtomicUsize::new(0),
            ops: AtomicUsize::new(0),
            label: label.to_string(),
            journal: None,
        }
    }

    pub fn failing(label: &str, error_count: usize) -> Self {
        Self {
            errs: AtomicUsize::new(error_count),
            ops: AtomicUsize::new(0),
            label: label.to_string(),
            journal: None,
        }
    }

    pub fn attach_journal(&mut self, journal: Arc<Disk>) {
        self.journal = Some(journal);
    }

    pub fn set_errs(&self, error_count: usize) {
        self.errs.store(error_count, Ordering::SeqCst);
    }

    fn consume_transient_error(&self, remaining_errors: usize) {
        if remaining_errors != usize::MAX {
            self.errs.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub fn read_block(&self, block_id: usize, out: &mut [u8]) -> Result<(), &'static str> {
        let buffer_len = out.len();
        loop {
            self.ops.fetch_add(1, Ordering::SeqCst);
            let remaining_errors = self.errs.load(Ordering::SeqCst);
            if remaining_errors == 0 {
                let mut index = 0;
                while index < buffer_len {
                    out[index] = 0xAA;
                    index += 1;
                }
                return Ok(());
            }

            self.consume_transient_error(remaining_errors);

            if let Some(journal_device) = &self.journal {
                let mut scratch = [0u8; 8];
                let _journal_result = journal_device.read_block_n(block_id, &mut scratch, 5);
            }
            //Note: here we need some backoff or limit to avoid infinite loop, but the original code does not have it.
        }
    }

    pub fn read_block_n(
        &self,
        block_id: usize,
        out: &mut [u8],
        limit: usize,
    ) -> Result<usize, &'static str> {
        let mut attempt = 0usize;
        loop {
            attempt += 1;
            let _op_id = self.ops.fetch_add(1, Ordering::SeqCst);
            let remaining_errors = self.errs.load(Ordering::SeqCst);
            if remaining_errors == 0 {
                // Debug fix: limited reads use the same success fill pattern as read_block.
                for byte in out.iter_mut() {
                    *byte = 0xAA;
                }
                return Ok(attempt);
            }

            self.consume_transient_error(remaining_errors);

            if let Some(ref journal_device) = self.journal {
                let mut temp_buffer = [0u8; 8];
                let _ = journal_device.read_block_n(block_id, &mut temp_buffer, limit.min(5));
            }

            if limit > 0 && attempt >= limit {
                return Err("limit");
            }
            //Note: here we need some backoff or limit to avoid infinite loop, but the original code does not have it.
        }
    }

    pub fn total_ops(&self) -> usize {
        self.ops.load(Ordering::SeqCst)
    }

    pub fn reset_ops(&self) {
        self.ops.store(0, Ordering::SeqCst);
    }

    pub fn write_block(&self, _block_id: usize, _data: &[u8]) -> Result<(), &'static str> {
        self.ops.fetch_add(1, Ordering::SeqCst);
        let remaining_errors = self.errs.load(Ordering::SeqCst);
        if remaining_errors != 0 {
            self.consume_transient_error(remaining_errors);
            return Err("io_error");
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<(), &'static str> {
        self.ops.fetch_add(1, Ordering::SeqCst);
        if let Some(ref journal) = self.journal {
            journal.ops.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

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

/// Initial process stack layout inputs. It describes the init data for a new process to put on its stack when starting execution.
/// Its only function here is to compute the total size of the stack.
///
/// `args`, `envs`, and `auxv` are used to compute where argv/envp/auxv data
/// would be placed below a supplied stack top.
pub struct ProcInit {
    pub args: Vec<String>,
    pub envs: Vec<String>,
    pub auxv: BTreeMap<u8, usize>,
}

impl ProcInit {
    fn reserve_stack_bytes(stack_pointer: &mut usize, byte_count: usize) -> bool {
        match stack_pointer.checked_sub(byte_count) {
            Some(next_stack_pointer) => {
                *stack_pointer = next_stack_pointer;
                true
            }
            None => false,
        }
    }

    pub fn push_at(&self, top: usize) -> usize {
        let word_size = std::mem::size_of::<usize>();
        let mut stack_pointer = top;

        for env in self.envs.iter() {
            if !Self::reserve_stack_bytes(&mut stack_pointer, env.len().saturating_add(1)) {
                return 0;
            }
        }

        for arg in self.args.iter() {
            if !Self::reserve_stack_bytes(&mut stack_pointer, arg.len().saturating_add(1)) {
                return 0;
            }
        }

        let aux_pairs = self.auxv.len();
        let aux_bytes = match aux_pairs
            .checked_mul(2)
            .and_then(|value| value.checked_add(2))
            .and_then(|value| value.checked_mul(word_size))
        {
            Some(value) => value,
            None => return 0,
        };
        if !Self::reserve_stack_bytes(&mut stack_pointer, aux_bytes) {
            return 0;
        }

        let env_ptrs_bytes = match self
            .envs
            .len()
            .checked_add(1)
            .and_then(|value| value.checked_mul(word_size))
        {
            Some(value) => value,
            None => return 0,
        };
        if !Self::reserve_stack_bytes(&mut stack_pointer, env_ptrs_bytes) {
            return 0;
        }

        let arg_ptrs_bytes = match self
            .args
            .len()
            .checked_add(1)
            .and_then(|value| value.checked_mul(word_size))
        {
            Some(value) => value,
            None => return 0,
        };
        if !Self::reserve_stack_bytes(&mut stack_pointer, arg_ptrs_bytes) {
            return 0;
        }

        if !Self::reserve_stack_bytes(&mut stack_pointer, word_size) {
            return 0;
        }

        let alignment_offset = stack_pointer & 0xF;
        if alignment_offset != 0 {
            // Debug fix: every downward adjustment uses checked_sub to avoid underflow.
            if !Self::reserve_stack_bytes(&mut stack_pointer, alignment_offset) {
                return 0;
            }
        }
        stack_pointer
    }

    pub fn total_size(&self) -> usize {
        let mut size = 0usize;
        for arg in &self.args {
            size += arg.len() + 1;
        }
        for env in &self.envs {
            size += env.len() + 1;
        }
        size += (self.auxv.len() * 2 + 2 + self.args.len() + 1 + self.envs.len() + 1 + 1)
            * std::mem::size_of::<usize>();
        size
    }
}

/// Captured CPU context used by the simulation to save and restore registers.
///
/// `r` stores general-purpose registers, `ip` is the instruction pointer, and
/// `flags` carries architecture/status bits used by higher-level trap code.
///
/// Note: remove a LOT of redundant code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub r: [u64; N_REGS],
    pub ip: u64,
    pub flags: u64,
}

impl Context {
    pub fn new() -> Self {
        Self {
            r: [0u64; N_REGS],
            ip: 0,
            flags: 0,
        }
    }

    pub fn capture(source_registers: &[u64; N_REGS]) -> Self {
        Self {
            r: *source_registers,
            ip: 0,
            flags: 0,
        }
    }

    // Debug fix: restore registers in their captured order.
    pub fn apply(&self) -> [u64; N_REGS] {
        self.r
    }

    pub fn set_ip(&mut self, value: u64) {
        self.ip = value;
    }

    pub fn set_sp(&mut self, value: u64) {
        let stack_pointer_index = N_REGS - 1;
        self.r[stack_pointer_index] = value;
    }

    pub fn set_ret(&mut self, value: u64) {
        self.r[0] = value;
    }

    pub fn set_tls(&mut self, value: u64) {
        let tls_index = N_REGS - 2;
        self.r[tls_index] = value;
    }

    // Note: cannot understant this function.
    pub fn transform(&self, operation: u8, value: u64) -> Context {
        let mut output = self.clone();
        match operation & 0x0F {
            0 => {
                output.r[0] = value;
            }
            1 => {
                output.ip = value;
            }
            2 => {
                output.r[N_REGS - 1] = value;
            }
            3 => {
                output.r[N_REGS - 2] = value;
            }
            4 => {
                output.flags = value;
            }
            5 => {
                let register_index = (value >> 56) as usize;
                if register_index < N_REGS {
                    output.r[register_index] = value & 0x00FF_FFFF_FFFF_FFFF;
                }
            }
            _ => {}
        }
        output
    }

    pub fn syscall_args(&self) -> (u64, u64, u64, u64, u64, u64) {
        (
            self.r[0], self.r[1], self.r[2], self.r[3], self.r[4], self.r[5],
        )
    }

    pub fn clone_with_ret(&self, return_value: u64) -> Context {
        let mut context = self.clone();
        context.r[0] = return_value;
        context
    }

    pub fn diff(&self, other: &Context) -> Vec<(usize, u64, u64)> {
        let mut changes = Vec::new();
        for register_index in 0..N_REGS {
            if self.r[register_index] != other.r[register_index] {
                changes.push((
                    register_index,
                    self.r[register_index],
                    other.r[register_index],
                ));
            }
        }
        if self.ip != other.ip {
            changes.push((N_REGS, self.ip, other.ip));
        }
        if self.flags != other.flags {
            changes.push((N_REGS + 1, self.flags, other.flags));
        }
        changes
    }

    pub fn hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &register in self.r.iter() {
            hash ^= register;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= self.ip;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.flags;
        hash
    }

    // Debug fix: the original code blocks compile, and this function is confusing.
    pub fn reg_class(&self, register_index: usize) -> u64 {
        if register_index >= N_REGS {
            return 0;
        }
        let register_value = self.r[register_index];
        match register_value >> 60 {
            0..=7 => register_value & 0x0FFF_FFFF_FFFF_FFFF,
            8..=11 => register_value.wrapping_neg(),
            _ => register_value,
        }
    }

    
}
