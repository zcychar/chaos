#![allow(unused_imports)]

use crate::sync::{EventBus, EventFlag};
use std::cmp::min;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Direction of one pipe endpoint.
#[derive(Clone, PartialEq)]
pub enum PipeDir {
    Rd,
    Wr,
}

/// Shared buffer and endpoint counters for a pipe pair.
///
/// `readers` and `writers` track live endpoints by direction, while `ends`
/// tracks the total endpoint count for compatibility with the original model.
pub struct PipeBuf {
    pub buf: VecDeque<u8>,
    pub bus: EventBus,
    pub ends: i32,
    readers: i32,
    writers: i32,
}

/// One read or write endpoint of a pipe.
pub struct PipeNode {
    data: Arc<Mutex<PipeBuf>>,
    dir: PipeDir,
}

impl Clone for PipeNode {
    fn clone(&self) -> Self {
        let mut pipe = self.data.lock().unwrap();
        // Debug fix: cloning an endpoint must increment the counters that Drop
        // later decrements, otherwise dropping a duplicate closes the original.
        pipe.ends += 1;
        match &self.dir {
            PipeDir::Rd => pipe.readers += 1,
            PipeDir::Wr => pipe.writers += 1,
        }
        drop(pipe);
        Self {
            data: self.data.clone(),
            dir: self.dir.clone(),
        }
    }
}

impl Drop for PipeNode {
    fn drop(&mut self) {
        let mut pipe = self.data.lock().unwrap();
        pipe.ends = pipe.ends.saturating_sub(1);
        match &self.dir {
            PipeDir::Rd => pipe.readers = pipe.readers.saturating_sub(1),
            PipeDir::Wr => pipe.writers = pipe.writers.saturating_sub(1),
        }
        if pipe.readers == 0 || pipe.writers == 0 {
            pipe.bus.set(EventFlag::CLOSED);
        }
    }
}

impl PipeNode {
    pub fn pair() -> (PipeNode, PipeNode) {
        let inner = PipeBuf {
            buf: VecDeque::new(),
            bus: EventBus::default(),
            ends: 2,
            readers: 1,
            writers: 1,
        };
        let shared_pipe = Arc::new(Mutex::new(inner));
        (
            PipeNode {
                data: shared_pipe.clone(),
                dir: PipeDir::Rd,
            },
            PipeNode {
                data: shared_pipe,
                dir: PipeDir::Wr,
            },
        )
    }

    pub fn can_read(&self) -> bool {
        if self.dir != PipeDir::Rd {
            return false;
        }
        let pipe = self.data.lock().unwrap();
        !pipe.buf.is_empty() || pipe.writers == 0
    }

    pub fn can_write(&self) -> bool {
        if self.dir != PipeDir::Wr {
            return false;
        }
        self.data.lock().unwrap().readers > 0
    }

    pub fn read_at(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.dir != PipeDir::Rd {
            return Ok(0);
        }

        let mut pipe = self.data.lock().unwrap();
        if pipe.buf.is_empty() && pipe.writers > 0 {
            return Err("again");
        }

        let bytes_to_read = min(buffer.len(), pipe.buf.len());
        for slot in buffer.iter_mut().take(bytes_to_read) {
            *slot = pipe.buf.pop_front().unwrap();
        }
        if pipe.buf.is_empty() {
            pipe.bus.clear(EventFlag::READABLE);
        }
        Ok(bytes_to_read)
    }

    pub fn write_at(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        if self.dir != PipeDir::Wr {
            return Ok(0);
        }

        let mut pipe = self.data.lock().unwrap();
        // Debug fix: writing to a pipe with no readers must fail instead of
        // buffering bytes that no endpoint can read.
        if pipe.readers == 0 {
            return Err("epipe");
        }

        for &byte in buffer {
            pipe.buf.push_back(byte);
        }
        pipe.bus.set(EventFlag::READABLE);
        Ok(buffer.len())
    }

    pub fn poll(&self) -> (bool, bool, bool) {
        (self.can_read(), self.can_write(), false)
    }
}
