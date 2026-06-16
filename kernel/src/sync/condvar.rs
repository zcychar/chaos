use super::*;
use crate::consts::{INFORM_PER_MSEC, USEC_PER_TICK};
use crate::process::{current_thread, Process, Thread};
use crate::syscall::TimeSpec;
use alloc::collections::VecDeque;
use alloc::sync::Arc;

pub struct RegisteredProcess {
    proc: Arc<SpinNoIrqLock<Process>>,
    tid: usize,
    epfd: usize,
    fd: usize,
}

#[derive(Default)]
pub struct Condvar {
    wait_queue: SpinNoIrqLock<VecDeque<Arc<Thread>>>,
    pub epoll_queue: SpinNoIrqLock<VecDeque<RegisteredProcess>>,
}

impl Condvar {
    pub fn new() -> Self {
        Condvar::default()
    }

    pub fn wait_queue_len(&self) -> usize {
        self.wait_queue.lock().len()
    }

    /// Park current thread and wait for this condvar to be notified.
    #[deprecated(note = "this may leads to lost wakeup problem. please use `wait` instead.")]
    pub fn _wait(&self) {
        // The condvar might be notified between adding to queue and thread parking.
        // So park current thread before wait queue lock is freed.
        // Avoid racing
        let lock = self.add_to_wait_queue();
        //thread::park_action(move || {
        //drop(lock);
        //});
    }

    fn add_to_wait_queue(&self) -> MutexGuard<VecDeque<Arc<Thread>>, SpinNoIrq> {
        let mut lock = self.wait_queue.lock();
        if let Some(thread) = current_thread() {
            Self::push_waiter_locked(&mut lock, &thread);
        }
        lock
    }

    fn push_waiter_locked(queue: &mut VecDeque<Arc<Thread>>, thread: &Arc<Thread>) {
        if queue.iter().all(|waiter| waiter.tid != thread.tid) {
            queue.push_back(thread.clone());
        }
    }

    fn remove_waiter_locked(queue: &mut VecDeque<Arc<Thread>>, tid: usize) -> bool {
        let before = queue.len();
        queue.retain(|waiter| waiter.tid != tid);
        before != queue.len()
    }

    fn remove_waiter(&self, tid: usize) -> bool {
        let mut queue = self.wait_queue.lock();
        Self::remove_waiter_locked(&mut queue, tid)
    }

    /// Wait for condvar until condition() returns Some
    pub fn wait_event<T>(condvar: &Condvar, condition: impl FnMut() -> Option<T>) -> T {
        Self::wait_events(&[condvar], condition)
    }

    /// Wait for condvars until condition() returns Some
    pub fn wait_events<T>(condvars: &[&Condvar], mut condition: impl FnMut() -> Option<T>) -> T {
        let thread = current_thread();
        let tid = thread.as_ref().map(|thread| thread.tid);
        loop {
            if let Some(res) = condition() {
                let _ = FlagsGuard::no_irq_region();
                if let Some(tid) = tid {
                    for condvar in condvars {
                        condvar.remove_waiter(tid);
                    }
                }
                return res;
            } else {
                if let Some(thread) = &thread {
                    for condvar in condvars {
                        let mut queue = condvar.wait_queue.lock();
                        Self::push_waiter_locked(&mut queue, thread);
                    }
                }
            }
            //thread::yield_now();
        }
    }

    /// Park current thread and wait for this condvar to be notified.
    pub fn wait<'a, T, S>(&self, guard: MutexGuard<'a, T, S>) -> MutexGuard<'a, T, S>
    where
        S: MutexSupport,
    {
        let mutex = guard.mutex;
        let thread = current_thread();
        let tid = thread.as_ref().map(|thread| thread.tid);
        let mut lock = self.wait_queue.lock();
        if let Some(thread) = &thread {
            Self::push_waiter_locked(&mut lock, thread);
        }

        //thread::park_action(move || {
        //drop(lock);
        //drop(guard);
        //});
        drop(lock);
        drop(guard);
        if let Some(tid) = tid {
            while self
                .wait_queue
                .lock()
                .iter()
                .any(|waiter| waiter.tid == tid)
            {
                core::sync::atomic::spin_loop_hint();
            }
        }
        mutex.lock()
    }

    /// Park current thread and wait for this condvar to be notified or timeout.
    pub fn wait_timeout<'a, T, S>(
        &self,
        guard: MutexGuard<'a, T, S>,
        timeout: TimeSpec,
    ) -> Option<MutexGuard<'a, T, S>>
    where
        S: MutexSupport,
    {
        let mutex = guard.mutex;
        let thread = current_thread();
        let tid = thread.as_ref().map(|thread| thread.tid);
        let mut lock = self.wait_queue.lock();
        if let Some(thread) = &thread {
            Self::push_waiter_locked(&mut lock, thread);
        }
        drop(lock);
        drop(guard);

        let timeout = core::time::Duration::new(timeout.sec as u64, timeout.nsec as u32);
        let timeout_millis = core::cmp::min(timeout.as_millis(), usize::MAX as u128) as usize;
        let begin = crate::trap::uptime_msec();
        loop {
            if let Some(tid) = tid {
                if self
                    .wait_queue
                    .lock()
                    .iter()
                    .all(|waiter| waiter.tid != tid)
                {
                    return Some(mutex.lock());
                }
            }
            let end = crate::trap::uptime_msec();
            if end.saturating_sub(begin) >= timeout_millis {
                if let Some(tid) = tid {
                    self.remove_waiter(tid);
                }
                return None;
            }
            core::sync::atomic::spin_loop_hint();
        }
    }

    pub fn notify_one(&self) {
        let mut queue = self.wait_queue.lock();
        if let Some(t) = queue.front() {
            self.epoll_callback(t);
            // info!("nofity thread: {}", t.id());
            //t.unpark();
            queue.pop_front();
        }
    }

    pub fn notify_all(&self) {
        let mut queue = self.wait_queue.lock();
        for t in queue.iter() {
            self.epoll_callback(t);
            //t.unpark();
        }
        queue.clear();
    }

    /// Notify up to `n` waiters.
    /// Return the number of waiters that were woken up.
    pub fn notify_n(&self, n: usize) -> usize {
        let mut count = 0;
        let mut queue = self.wait_queue.lock();
        for t in queue.iter() {
            if count >= n {
                break;
            }
            self.epoll_callback(t);
            //t.unpark();
            count += 1;
        }
        for _ in 0..count {
            queue.pop_front();
        }
        count
    }

    pub fn register_epoll_list(
        &self,
        proc: Arc<SpinNoIrqLock<Process>>,
        tid: usize,
        epfd: usize,
        fd: usize,
    ) {
        self.epoll_queue.lock().push_back(RegisteredProcess {
            proc: proc,
            tid: tid,
            epfd: epfd,
            fd: fd,
        });
    }

    pub fn unregister_epoll_list(&self, tid: usize, epfd: usize, fd: usize) -> bool {
        let mut epoll_list = self.epoll_queue.lock();
        for idx in 0..epoll_list.len() {
            if epoll_list[idx].tid == tid
                && epoll_list[idx].epfd == epfd
                && epoll_list[idx].fd == fd
            {
                epoll_list.remove(idx);
                return true;
            }
        }
        return false;
    }

    fn epoll_callback(&self, thread: &Arc<Thread>) {
        let epoll_list = self.epoll_queue.lock();
        for ist in epoll_list.iter() {
            //if thread.id() == ist.tid {
            if true {
                let proc = ist.proc.lock();
                match proc.get_epoll_instance(ist.epfd) {
                    Ok(instacne) => {
                        let mut ready_list = instacne.ready_list.lock();
                        ready_list.insert(ist.fd);
                    }
                    Err(_) => {
                        panic!("epoll instance not exist");
                    }
                }
            }
        }
    }
}
