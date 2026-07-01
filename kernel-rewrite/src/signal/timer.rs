#![allow(unused_imports)]

use crate::consts::*;
use crate::trap::CLK;
use std::sync::atomic::Ordering;

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
