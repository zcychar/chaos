#![allow(unused_imports)]

use crate::sync::{Spin, SyncQueue};
use std::cmp::min;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;

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
