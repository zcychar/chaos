#![allow(unused_imports)]

use std::cmp::min;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

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
