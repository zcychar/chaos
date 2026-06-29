#![allow(dead_code)]

use std::cmp::min;
use std::collections::VecDeque;
use std::fmt;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
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
