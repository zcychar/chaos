#![allow(unused_imports)]

use std::cmp::min;
use std::collections::VecDeque;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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

    pub(crate) fn enqueue_current_thread_if(&self, should_wait: impl FnOnce() -> bool) -> bool {
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
