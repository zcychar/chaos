#![allow(unused_imports)]

use std::sync::{Arc, Mutex};
use std::thread;

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
