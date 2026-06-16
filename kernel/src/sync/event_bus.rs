use crate::sync::SpinNoIrqLock as Mutex;
use alloc::boxed::Box;
use alloc::{sync::Arc, vec::Vec};
use bitflags::bitflags;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

bitflags! {
    #[derive(Default)]
    pub struct Event: u32 {
        /// File
        const READABLE                      = 1 << 0;
        const WRITABLE                      = 1 << 1;
        const ERROR                         = 1 << 2;
        const CLOSED                        = 1 << 3;

        /// Process
        const PROCESS_QUIT                  = 1 << 10;
        const CHILD_PROCESS_QUIT            = 1 << 11;
        const RECEIVE_SIGNAL                = 1 << 12;

        /// Semaphore
        const SEMAPHORE_REMOVED             = 1 << 20;
        const SEMAPHORE_CAN_ACQUIRE         = 1 << 21;
    }
}

pub type EventHandler = Box<dyn Fn(Event) -> bool + Send>;

struct EventCallback {
    mask: Option<Event>,
    waker: Option<Waker>,
    handler: EventHandler,
}

impl EventCallback {
    fn new(handler: EventHandler) -> Self {
        Self {
            mask: None,
            waker: None,
            handler,
        }
    }

    fn new_waker(mask: Event, waker: Waker) -> Self {
        let handler_waker = waker.clone();
        Self {
            mask: Some(mask),
            waker: Some(waker),
            handler: Box::new(move |event| {
                if (event & mask).is_empty() {
                    return false;
                }
                handler_waker.wake_by_ref();
                true
            }),
        }
    }

    fn matches_waker(&self, mask: Event, waker: &Waker) -> bool {
        self.mask == Some(mask)
            && self
                .waker
                .as_ref()
                .map_or(false, |existing| existing.will_wake(waker))
    }

    fn call(&self, event: Event) -> bool {
        (self.handler)(event)
    }
}

#[derive(Default)]
pub struct EventBus {
    event: Event,
    callbacks: Vec<EventCallback>,
}

impl EventBus {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }

    pub fn set(&mut self, set: Event) {
        self.change(Event::empty(), set);
    }

    pub fn clear(&mut self, set: Event) {
        self.change(set, Event::empty());
    }

    pub fn change(&mut self, reset: Event, set: Event) {
        let orig = self.event;
        let mut new = self.event;
        new.remove(reset);
        new.insert(set);
        self.event = new;
        if new != orig {
            self.callbacks.retain(|f| !f.call(new));
        }
    }

    pub fn subscribe(&mut self, callback: EventHandler) {
        self.callbacks.push(EventCallback::new(callback));
    }

    pub fn subscribe_waker(&mut self, mask: Event, waker: Waker) -> bool {
        if !(self.event & mask).is_empty() {
            waker.wake_by_ref();
            return true;
        }
        if self
            .callbacks
            .iter()
            .any(|callback| callback.matches_waker(mask, &waker))
        {
            return false;
        }
        self.callbacks.push(EventCallback::new_waker(mask, waker));
        false
    }

    pub fn get_callback_len(&self) -> usize {
        self.callbacks.len()
    }
}

pub fn wait_for_event(bus: Arc<Mutex<EventBus>>, mask: Event) -> impl Future<Output = Event> {
    EventBusFuture { bus, mask }
}

#[must_use = "future does nothing unless polled/`await`-ed"]
struct EventBusFuture {
    bus: Arc<Mutex<EventBus>>,
    mask: Event,
}

impl Future for EventBusFuture {
    type Output = Event;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut lock = self.bus.lock();
        if !(lock.event & self.mask).is_empty() {
            return Poll::Ready(lock.event);
        }
        let mask = self.mask;
        lock.subscribe_waker(mask, cx.waker().clone());
        Poll::Pending
    }
}
