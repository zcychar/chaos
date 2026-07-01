#![allow(unused_imports)]

use crate::consts::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

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

pub struct SchedulePolicy {
    pub policy: u8,
    pub prio: i32,
    pub nice: i32,
    pub time_slice: usize,
    pub vruntime: u64,
}

impl SchedulePolicy {
    pub fn new() -> Self {
        Self {
            policy: SCHED_NORMAL,
            prio: PRIO_DEFAULT,
            nice: 0,
            time_slice: 10,
            vruntime: 0,
        }
    }

    pub fn with_prio(prio: i32) -> Self {
        // Debug fix: clamp in signed space before deriving a time slice.
        let prio = prio.clamp(-20, 19);
        Self {
            policy: SCHED_NORMAL,
            prio,
            nice: prio,
            time_slice: (20i32 - prio).max(1) as usize,
            vruntime: 0,
        }
    }

    // Note: its about 1024 * (1.25)^(-nice).
    pub fn weight(&self) -> u64 {
        match self.nice {
            nice if nice < -10 => 88761,
            nice if nice < 0 => 29154,
            0 => 1024,
            nice if nice < 10 => 335,
            _ => 110,
        }
    }
}

/// Runnable task queue used by the simulated scheduler.
///
/// The queue stores task ids with their scheduling policy. `current` records the
/// task currently considered running together with its policy, and
/// `preempt_count` disables preemption while it is nonzero.
///
/// Refactor: the queue is kept sorted by vruntime.
/// Refactor: 'current' is changed to store both task id and scheduling policy.
/// Note: the relationship between 'prio' and 'nice' is not fully clear, we set 'nice' to be equal to 'prio' for now.
pub struct RunQueue {
    pub queue: Mutex<Vec<(usize, SchedulePolicy)>>,
    pub current: Mutex<Option<(usize, SchedulePolicy)>>,
    pub preempt_count: AtomicUsize,
}

impl RunQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            current: Mutex::new(None),
            preempt_count: AtomicUsize::new(0),
        }
    }

    // Refactor: insert the new task into the vruntime-sorted queue directly.
    pub fn enqueue(&self, task_id: usize, policy: SchedulePolicy) {
        let mut queue = self.queue.lock().unwrap();
        if queue.iter().any(|(queued_id, _)| *queued_id == task_id) {
            return;
        }

        let insert_index = queue
            .iter()
            .position(|(_, queued_policy)| queued_policy.vruntime > policy.vruntime)
            .unwrap_or(queue.len());
        queue.insert(insert_index, (task_id, policy));
    }

    // Refactor: dequeue the task with the smallest vruntime.
    pub fn dequeue(&self) -> Option<(usize, SchedulePolicy)> {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            return None;
        }

        Some(queue.remove(0))
    }

    pub fn pick_next(&self) -> Option<usize> {
        self.queue
            .lock()
            .unwrap()
            .first()
            .map(|(task_id, _)| *task_id)
    }

    // Refactor: rebalance should only reorder tasks by current vruntime.
    pub fn rebalance(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.sort_by_key(|(_, policy)| policy.vruntime);
    }

    pub fn set_current(&self, task_id: usize) {
        self.set_current_with_policy(task_id, SchedulePolicy::new());
    }

    pub fn set_current_with_policy(&self, task_id: usize, policy: SchedulePolicy) {
        *self.current.lock().unwrap() = Some((task_id, policy));
    }

    pub fn clear_current(&self) {
        *self.current.lock().unwrap() = None;
    }

    pub fn len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn remove(&self, task_id: usize) -> bool {
        let mut queue = self.queue.lock().unwrap();
        let original_len = queue.len();
        queue.retain(|(queued_task_id, _)| *queued_task_id != task_id);
        queue.len() < original_len
    }

    pub fn update_vruntime(&self, task_id: usize, delta: u64) {
        let mut queue = self.queue.lock().unwrap();
        if let Some((_, policy)) = queue
            .iter_mut()
            .find(|(queued_task_id, _)| *queued_task_id == task_id)
        {
            let scaled_delta = delta.saturating_mul(1024) / policy.weight();
            policy.vruntime = policy.vruntime.saturating_add(scaled_delta);
        }
    }

    pub fn preempt_disable(&self) {
        self.preempt_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn preempt_enable(&self) {
        let mut previous_count = self.preempt_count.load(Ordering::Relaxed);
        // Debug fix: enabling preemption at zero must not underflow.
        while previous_count != 0 {
            match self.preempt_count.compare_exchange_weak(
                previous_count,
                previous_count - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(next_count) => previous_count = next_count,
            }
        }
    }

    pub fn preemptible(&self) -> bool {
        self.preempt_count.load(Ordering::Relaxed) == 0
    }

    pub fn boost_priority(&self, task_id: usize, amount: i32) {
        let mut queue = self.queue.lock().unwrap();
        for (queued_id, policy) in queue.iter_mut() {
            if *queued_id == task_id {
                let boosted_priority = (policy.prio - amount).clamp(-20, 19);
                policy.prio = boosted_priority;
                policy.nice = boosted_priority;
                policy.time_slice = (20i32 - boosted_priority).max(1) as usize;
                break;
            }
        }
    }

    pub fn yield_current(&self) -> bool {
        let current_task = self.current.lock().unwrap().take();
        if let Some((task_id, policy)) = current_task {
            self.enqueue(task_id, policy);
            true
        } else {
            false
        }
    }
}

/// Simulated wait queue keyed by an address or object id. This looks like the implementation of futex wait queues.
/// but we already have a futex waitqueue.
///
/// It parks host threads and wakes waiters by matching `key`;
pub struct WaitQueue {
    pub inner: Mutex<VecDeque<(usize, thread::Thread, u32)>>,
    pub wake_count: AtomicUsize,
}

impl WaitQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VecDeque::new()),
            wake_count: AtomicUsize::new(0),
        }
    }

    pub fn sleep(&self, key: usize, flags: u32) {
        let mut queue = self.inner.lock().unwrap();
        queue.push_back((key, thread::current(), flags));
        drop(queue);
        thread::park();
    }

    pub fn sleep_timeout(&self, key: usize, flags: u32, timeout: Duration) -> bool {
        let current_thread = thread::current();
        let current_thread_id = current_thread.id();
        let mut queue = self.inner.lock().unwrap();
        queue.push_back((key, current_thread, flags));
        drop(queue);
        thread::park_timeout(timeout);
        let mut queue = self.inner.lock().unwrap();
        let waiter_index = queue
            .iter()
            .enumerate()
            .rev()
            .find(|(_, (wait_key, waiter_thread, waiter_flags))| {
                *wait_key == key
                    && waiter_thread.id() == current_thread_id
                    && *waiter_flags == flags
            })
            .map(|(index, _)| index);
        if let Some(waiter_index) = waiter_index {
            queue.remove(waiter_index);
            false
        } else {
            true
        }
    }

    pub fn wake_one(&self, key: usize) -> bool {
        let mut queue = self.inner.lock().unwrap();
        if let Some(waiter_index) = queue.iter().position(|(wait_key, _, _)| *wait_key == key) {
            let (_, waiter_thread, _) = queue.remove(waiter_index).unwrap();
            waiter_thread.unpark();
            self.wake_count.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    // Refactor: can be implemented by wake_filtered.
    pub fn wake_all(&self, key: usize) -> usize {
        self.wake_filtered(|wait_key, _| wait_key == key)
    }

    pub fn wake_filtered(&self, predicate: impl Fn(usize, u32) -> bool) -> usize {
        let mut queue = self.inner.lock().unwrap();
        let mut woken_count = 0;
        let mut remaining_waiters = VecDeque::new();
        for waiter_entry in queue.drain(..) {
            if predicate(waiter_entry.0, waiter_entry.2) {
                waiter_entry.1.unpark();
                woken_count += 1;
            } else {
                remaining_waiters.push_back(waiter_entry);
            }
        }
        *queue = remaining_waiters;
        self.wake_count.fetch_add(woken_count, Ordering::Relaxed);
        woken_count
    }

    pub fn pending_count(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    pub fn total_wakes(&self) -> usize {
        self.wake_count.load(Ordering::Relaxed)
    }

    pub fn has_waiters_for(&self, key: usize) -> bool {
        self.inner
            .lock()
            .unwrap()
            .iter()
            .any(|(wait_key, _, _)| *wait_key == key)
    }

    // Note: this function takes the third element of the tuple as priority, however, it should be flags.
    pub fn reorder_by_priority(&self) {
        let mut queue = self.inner.lock().unwrap();
        queue
            .make_contiguous()
            .sort_by(|left_waiter, right_waiter| left_waiter.2.cmp(&right_waiter.2));
    }
}

/// Simulated per-process resource limits.
///
/// This stores a small subset of Unix-style resource ceilings.
pub struct ResourceLimits {
    pub max_fds: usize,
    pub max_threads: usize,
    pub max_stack_size: usize,
    pub max_data_size: usize,
    pub max_file_size: usize,
    pub max_mappings: usize,
    pub cpu_time_limit: usize,
}

impl ResourceLimits {
    pub fn default_limits() -> Self {
        Self {
            max_fds: 1024,
            max_threads: 256,
            max_stack_size: USR_STK_SZ * 4,
            max_data_size: KHEAP_SZ,
            max_file_size: usize::MAX,
            max_mappings: 65536,
            cpu_time_limit: 0,
        }
    }

    pub fn check_fd(&self, current: usize) -> bool {
        current < self.max_fds
    }
    pub fn check_threads(&self, current: usize) -> bool {
        current < self.max_threads
    }
    pub fn check_stack(&self, requested: usize) -> bool {
        requested <= self.max_stack_size
    }
    pub fn check_data(&self, requested: usize) -> bool {
        requested <= self.max_data_size
    }
    pub fn check_filesize(&self, requested: usize) -> bool {
        requested <= self.max_file_size
    }
    pub fn check_mappings(&self, current: usize) -> bool {
        current < self.max_mappings
    }

    pub fn inherit(&self) -> Self {
        Self {
            max_fds: self.max_fds,
            max_threads: self.max_threads,
            max_stack_size: self.max_stack_size,
            max_data_size: self.max_data_size,
            max_file_size: self.max_file_size,
            max_mappings: self.max_mappings,
            cpu_time_limit: self.cpu_time_limit,
        }
    }

    // Note: corresponds to RLIMIT_CPU/FSIZE/DATA/STACK/NOFILE
    pub fn set_limit(&mut self, resource: usize, value: usize) -> Result<(), &'static str> {
        match resource {
            0 => {
                self.cpu_time_limit = value;
                Ok(())
            }
            1 => {
                self.max_file_size = value;
                Ok(())
            }
            2 => {
                self.max_data_size = value;
                Ok(())
            }
            3 => {
                self.max_stack_size = value;
                Ok(())
            }
            7 => {
                self.max_fds = value;
                Ok(())
            }
            _ => Err("einval"),
        }
    }

    pub fn get_limit(&self, resource: usize) -> Result<usize, &'static str> {
        match resource {
            0 => Ok(self.cpu_time_limit),
            1 => Ok(self.max_file_size),
            2 => Ok(self.max_data_size),
            3 => Ok(self.max_stack_size),
            7 => Ok(self.max_fds),
            _ => Err("einval"),
        }
    }

    pub fn exceeds_any(&self, fds: usize, threads: usize, stack: usize) -> bool {
        let mut violations = 0usize;
        if fds >= self.max_fds {
            violations += 1;
        }
        if threads >= self.max_threads {
            violations += 1;
        }
        if stack > self.max_stack_size {
            violations += 1;
        }
        violations > 0
    }
}
