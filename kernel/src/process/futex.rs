use crate::trap::NAIVE_TIMER;
use crate::{
    arch::timer::timer_now,
    sync::SpinNoIrqLock as Mutex,
    syscall::{SysError, SysResult},
};
use alloc::boxed::Box;
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::{future::Future, task::Waker, time::Duration};

pub struct Waiter {
    waker: Option<Waker>,
    woken: bool,
}

pub struct FutexInner {
    waiters: VecDeque<Arc<Mutex<Waiter>>>,
}

impl FutexInner {
    fn remove_waiter(&mut self, target: &Arc<Mutex<Waiter>>) -> bool {
        if let Some(index) = self
            .waiters
            .iter()
            .position(|waiter| Arc::ptr_eq(waiter, target))
        {
            self.waiters.remove(index);
            true
        } else {
            false
        }
    }

    fn contains_waiter(&self, target: &Arc<Mutex<Waiter>>) -> bool {
        self.waiters
            .iter()
            .any(|waiter| Arc::ptr_eq(waiter, target))
    }
}

pub struct Futex {
    pub inner: Mutex<FutexInner>,
}

impl Futex {
    pub fn new() -> Self {
        Futex {
            inner: Mutex::new(FutexInner {
                waiters: VecDeque::new(),
            }),
        }
    }

    pub fn wake(&self, wake_count: usize) -> usize {
        if wake_count == 0 {
            return 0;
        }

        let mut inner = self.inner.lock();
        let mut woken = 0;
        let mut wakers = Vec::new();
        while woken < wake_count {
            match inner.waiters.pop_front() {
                Some(waiter) => {
                    let mut waiter = waiter.lock();
                    if waiter.woken {
                        continue;
                    }
                    waiter.woken = true;
                    if let Some(waker) = waiter.waker.take() {
                        wakers.push(waker);
                    }
                    woken += 1;
                }
                None => break,
            }
        }
        drop(inner);

        for waker in wakers {
            waker.wake();
        }
        woken
    }

    pub fn wait(self: &Arc<Self>, timeout: Option<Duration>) -> impl Future<Output = SysResult> {
        #[must_use = "future does nothing unless polled/`await`-ed"]
        struct FutexFuture {
            futex: Arc<Futex>,
            waiter: Arc<Mutex<Waiter>>,
            deadline: Option<Duration>,
        }

        impl Future for FutexFuture {
            type Output = SysResult;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let mut futex = self.futex.inner.lock();
                let mut inner = self.waiter.lock();
                // check wakeup
                if inner.woken {
                    return Poll::Ready(Ok(0));
                }
                if let Some(deadline) = self.deadline {
                    if timer_now() >= deadline {
                        inner.woken = true;
                        inner.waker.take();
                        futex.remove_waiter(&self.waiter);
                        return Poll::Ready(Err(SysError::ETIMEDOUT));
                    }
                }

                // first time?
                if inner.waker.is_none() {
                    inner.waker.replace(cx.waker().clone());
                    if !futex.contains_waiter(&self.waiter) {
                        futex.waiters.push_back(self.waiter.clone());
                    }
                    drop(inner);
                    drop(futex);

                    // timer
                    if let Some(deadline) = self.deadline {
                        let waker = cx.waker().clone();
                        NAIVE_TIMER
                            .lock()
                            .add(deadline, Box::new(move |_| waker.wake()));
                    }
                    return Poll::Pending;
                }
                drop(inner);
                drop(futex);
                Poll::Pending
            }
        }

        FutexFuture {
            futex: self.clone(),
            waiter: Arc::new(Mutex::new(Waiter {
                waker: None,
                woken: false,
            })),
            deadline: timeout.map(|t| timer_now() + t),
        }
    }
}
