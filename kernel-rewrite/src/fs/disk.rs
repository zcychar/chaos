#![allow(unused_imports)]

use crate::consts::*;
use crate::trap::CLK;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// One pending disk I/O request.
///
/// `block` is the target block number, `write` distinguishes write from read,
/// and `priority` is stored for scheduler policy experiments.
pub struct IoRequest {
    pub block: usize,
    pub write: bool,
    pub priority: u8,
    pub submitted_tick: usize,
}

/// Simple disk I/O scheduler queue.
///
/// The dispatch policy follows the current head position and direction, while
/// `merged` tracks adjacent requests that were coalesced.
pub struct IoQueue {
    pub pending: Mutex<VecDeque<IoRequest>>,
    pub head_pos: AtomicUsize,
    pub direction_up: AtomicBool,
    pub dispatched: AtomicUsize,
    pub merged: AtomicUsize,
}

impl IoQueue {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
            head_pos: AtomicUsize::new(0),
            direction_up: AtomicBool::new(true),
            dispatched: AtomicUsize::new(0),
            merged: AtomicUsize::new(0),
        }
    }

    pub fn submit(&self, block_id: usize, write: bool, priority: u8) {
        let request = IoRequest {
            block: block_id,
            write,
            priority,
            submitted_tick: CLK.load(Ordering::Relaxed),
        };
        let mut queue = self.pending.lock().unwrap();
        queue.push_back(request);
    }

    pub fn submit_batch(&self, requests: &[(usize, bool, u8)]) -> usize {
        let mut queue = self.pending.lock().unwrap();
        let mut submitted_count = 0;
        for &(block_id, write, priority) in requests {
            let request = IoRequest {
                block: block_id,
                write,
                priority,
                submitted_tick: CLK.load(Ordering::Relaxed),
            };
            queue.push_back(request);
            submitted_count += 1;
        }
        let depth = queue.len();
        let should_merge = depth > IOQUEUE_DEPTH;
        drop(queue);

        // Debug fix: do not call merge_adjacent while still holding pending.
        if should_merge {
            self.merge_adjacent();
        }
        submitted_count
    }

    /// Note: currently, the dispatch policy is alike a simple SCAN algorithm, however, it deals backward requests in a very strange way,
    /// also, merge_adjacent confusing in the context, since it removes a request from the queue completely.
    pub fn dispatch(&self) -> Option<(usize, bool)> {
        let mut queue = self.pending.lock().unwrap();
        if queue.is_empty() {
            return None;
        }

        let head_position = self.head_pos.load(Ordering::Relaxed);
        let going_up = self.direction_up.load(Ordering::Relaxed);
        let mut best_index = 0;
        let mut best_distance = usize::MAX;

        for (index, request) in queue.iter().enumerate() {
            let distance = if going_up {
                if request.block >= head_position {
                    request.block - head_position
                } else {
                    usize::MAX / 2 + request.block
                }
            } else if request.block <= head_position {
                head_position - request.block
            } else {
                usize::MAX / 2 + head_position
            };

            if distance < best_distance {
                best_distance = distance;
                best_index = index;
            }
        }

        let request = queue.remove(best_index)?;
        self.head_pos.store(request.block, Ordering::Relaxed);
        if going_up && request.block >= head_position {
            if queue.iter().all(|queued| queued.block < request.block) {
                self.direction_up.store(false, Ordering::Relaxed);
            }
        } else if !going_up && request.block <= head_position {
            if queue.iter().all(|queued| queued.block > request.block) {
                self.direction_up.store(true, Ordering::Relaxed);
            }
        }
        self.dispatched.fetch_add(1, Ordering::Relaxed);
        Some((request.block, request.write))
    }

    pub fn merge_adjacent(&self) -> usize {
        let mut queue = self.pending.lock().unwrap();
        let mut merged_count = 0;
        let mut index = 0;
        while index + 1 < queue.len() {
            // Debug fix: checked_add avoids overflow for the final block id.
            if queue[index].block.checked_add(1) == Some(queue[index + 1].block)
                && queue[index].write == queue[index + 1].write
            {
                queue.remove(index + 1);
                merged_count += 1;
            } else {
                index += 1;
            }
        }
        self.merged.fetch_add(merged_count, Ordering::Relaxed);
        merged_count
    }

    pub fn depth(&self) -> usize {
        self.pending.lock().unwrap().len()
    }
}

/// Simulated block device with optional journal fallback.
///
/// `errs` is a countdown of remaining synthetic I/O failures that can be tried; `usize::MAX`
/// means persistent failure. `ops` counts attempted operations.
///
/// Note(IMPORTANT): this struct is full of errors, but we do not know what it should behave based on current testcases.
pub struct Disk {
    pub errs: AtomicUsize,
    pub ops: AtomicUsize,
    pub label: String,
    pub journal: Option<Arc<Disk>>,
}

impl Disk {
    pub fn new(label: &str) -> Self {
        Self {
            errs: AtomicUsize::new(0),
            ops: AtomicUsize::new(0),
            label: label.to_string(),
            journal: None,
        }
    }

    pub fn failing(label: &str, error_count: usize) -> Self {
        Self {
            errs: AtomicUsize::new(error_count),
            ops: AtomicUsize::new(0),
            label: label.to_string(),
            journal: None,
        }
    }

    pub fn attach_journal(&mut self, journal: Arc<Disk>) {
        self.journal = Some(journal);
    }

    pub fn set_errs(&self, error_count: usize) {
        self.errs.store(error_count, Ordering::SeqCst);
    }

    fn consume_transient_error(&self, remaining_errors: usize) {
        if remaining_errors != usize::MAX {
            self.errs.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub fn read_block(&self, block_id: usize, out: &mut [u8]) -> Result<(), &'static str> {
        let buffer_len = out.len();
        loop {
            self.ops.fetch_add(1, Ordering::SeqCst);
            let remaining_errors = self.errs.load(Ordering::SeqCst);
            if remaining_errors == 0 {
                let mut index = 0;
                while index < buffer_len {
                    out[index] = 0xAA;
                    index += 1;
                }
                return Ok(());
            }

            self.consume_transient_error(remaining_errors);

            if let Some(journal_device) = &self.journal {
                let mut scratch = [0u8; 8];
                let _journal_result = journal_device.read_block_n(block_id, &mut scratch, 5);
            }
            //Note: here we need some backoff or limit to avoid infinite loop, but the original code does not have it.
        }
    }

    pub fn read_block_n(
        &self,
        block_id: usize,
        out: &mut [u8],
        limit: usize,
    ) -> Result<usize, &'static str> {
        let mut attempt = 0usize;
        loop {
            attempt += 1;
            let _op_id = self.ops.fetch_add(1, Ordering::SeqCst);
            let remaining_errors = self.errs.load(Ordering::SeqCst);
            if remaining_errors == 0 {
                // Debug fix: limited reads use the same success fill pattern as read_block.
                for byte in out.iter_mut() {
                    *byte = 0xAA;
                }
                return Ok(attempt);
            }

            self.consume_transient_error(remaining_errors);

            if let Some(ref journal_device) = self.journal {
                let mut temp_buffer = [0u8; 8];
                let _ = journal_device.read_block_n(block_id, &mut temp_buffer, limit.min(5));
            }

            if limit > 0 && attempt >= limit {
                return Err("limit");
            }
            //Note: here we need some backoff or limit to avoid infinite loop, but the original code does not have it.
        }
    }

    pub fn total_ops(&self) -> usize {
        self.ops.load(Ordering::SeqCst)
    }

    pub fn reset_ops(&self) {
        self.ops.store(0, Ordering::SeqCst);
    }

    pub fn write_block(&self, _block_id: usize, _data: &[u8]) -> Result<(), &'static str> {
        self.ops.fetch_add(1, Ordering::SeqCst);
        let remaining_errors = self.errs.load(Ordering::SeqCst);
        if remaining_errors != 0 {
            self.consume_transient_error(remaining_errors);
            return Err("io_error");
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<(), &'static str> {
        self.ops.fetch_add(1, Ordering::SeqCst);
        if let Some(ref journal) = self.journal {
            journal.ops.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
}
