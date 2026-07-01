#![allow(unused_imports)]

use crate::consts::*;
use crate::sync::{EventBus, EventFlag, Spin, SyncQueue, GKL};
use crate::trap::CLK;
use std::cmp::min;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

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

/// Per-file-descriptor access options.
///
/// `rd` and `wr` control read/write permission, `ap` means append mode, and
/// `nb` means nonblocking mode.
#[derive(Debug, Clone, Copy)]
pub struct FdOpt {
    pub rd: bool,
    pub wr: bool,
    pub ap: bool,
    pub nb: bool,
}

impl Default for FdOpt {
    fn default() -> Self {
        Self {
            rd: true,
            wr: false,
            ap: false,
            nb: false,
        }
    }
}

/// Shared open-file-description state for duplicated file handles.
///
/// Multiple `FHandle` values can point at the same `FdState`, so offset and
/// options are protected by an `RwLock` and shared through `Arc`.
struct FdState {
    off: u64,
    opt: FdOpt,
    flk: u8, //USELESS...
}

impl FdState {
    fn create(opt: FdOpt) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(FdState {
            off: 0,
            opt,
            flk: 0,
        }))
    }
}

/// In-memory file handle used by the kernel simulation.
///
/// `data` stores the file bytes, while `desc` is the shared open-file
/// description. Duplicated handles therefore share the same file offset and
/// access options, matching the usual `dup` behavior.
pub struct FHandle {
    pub path: String,
    pub data: Arc<Mutex<Vec<u8>>>,
    desc: Arc<RwLock<FdState>>,
    pub pipe: bool,
    pub cloexec: bool,
}

/// Seek origin used by `FHandle::seek`.
#[derive(Debug)]
pub enum FSeek {
    Start(u64),
    End(i64),
    Cur(i64),
}

impl FHandle {
    pub fn new(path: &str, opt: FdOpt, pipe: bool, cloexec: bool) -> Self {
        Self {
            path: path.to_string(),
            data: Arc::new(Mutex::new(Vec::new())),
            desc: FdState::create(opt),
            pipe,
            cloexec,
        }
    }

    pub fn with_data(path: &str, opt: FdOpt, initial_data: Vec<u8>) -> Self {
        Self {
            path: path.to_string(),
            data: Arc::new(Mutex::new(initial_data)),
            desc: FdState::create(opt),
            pipe: false,
            cloexec: false,
        }
    }

    pub fn dup(&self, cloexec: bool) -> Self {
        FHandle {
            path: self.path.clone(),
            data: self.data.clone(),
            desc: self.desc.clone(),
            pipe: self.pipe,
            cloexec,
        }
    }

    //Debug fix: add more supports (confused).
    pub fn set_opt(&self, arg: usize) {
        let mut state = self.desc.write().unwrap();
        state.opt.nb = (arg & O_NONBLOCK) != 0;
        state.opt.ap = (arg & O_APPEND) != 0;
    }

    pub fn get_opt(&self) -> FdOpt {
        self.desc.read().unwrap().opt
    }

    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        let offset = self.desc.read().unwrap().off as usize;
        let bytes_read = self.read_at(offset, buffer)?;
        self.desc.write().unwrap().off += bytes_read as u64;
        Ok(bytes_read)
    }

    pub fn read_at(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, &'static str> {
        if !self.desc.read().unwrap().opt.rd {
            return Err("ebadf");
        }
        let contents = self.data.lock().unwrap();
        if offset >= contents.len() {
            return Ok(0);
        }
        let bytes_to_copy = min(buffer.len(), contents.len() - offset);
        buffer[..bytes_to_copy].copy_from_slice(&contents[offset..offset + bytes_to_copy]);
        Ok(bytes_to_copy)
    }

    pub fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        let write_offset = {
            let state = self.desc.read().unwrap();
            if state.opt.ap {
                self.data.lock().unwrap().len() as u64
            } else {
                state.off
            }
        } as usize;
        let bytes_written = self.write_at(write_offset, buffer)?;
        // Debug fix: append writes must advance the descriptor offset to the
        // end of the actual append, not from the old descriptor offset.
        let new_offset = write_offset.checked_add(bytes_written).ok_or("eoverflow")?;
        self.desc.write().unwrap().off = new_offset as u64;
        Ok(bytes_written)
    }

    pub fn write_at(&self, offset: usize, buffer: &[u8]) -> Result<usize, &'static str> {
        if !self.desc.read().unwrap().opt.wr {
            return Err("ebadf");
        }
        let mut contents = self.data.lock().unwrap();
        // Debug fix: checked arithmetic prevents `off + len` from wrapping
        // before resize and slice bounds are computed.
        let end_offset = offset.checked_add(buffer.len()).ok_or("einval")?;
        if end_offset > contents.len() {
            contents.resize(end_offset, 0);
        }
        contents[offset..end_offset].copy_from_slice(buffer);
        Ok(buffer.len())
    }

    pub fn seek(&self, pos: FSeek) -> Result<u64, &'static str> {
        let mut state = self.desc.write().unwrap();
        let next_offset = match pos {
            FSeek::Start(offset) => offset as i128,
            FSeek::End(delta) => self.data.lock().unwrap().len() as i128 + delta as i128,
            FSeek::Cur(delta) => state.off as i128 + delta as i128,
        };
        // Debug fix: keep the calculation signed until negative results have
        // been rejected, avoiding wraparound into a huge u64 offset.
        if next_offset < 0 || next_offset > u64::MAX as i128 {
            return Err("einval");
        }
        state.off = next_offset as u64;
        Ok(state.off)
    }

    pub fn transfer(
        &self,
        direction: u8,
        offset: Option<usize>,
        read_buffer: Option<&mut [u8]>,
        write_buffer: Option<&[u8]>,
    ) -> Result<usize, &'static str> {
        if direction & 1 != 0 {
            match (offset, read_buffer) {
                (Some(read_offset), Some(buffer)) => self.read_at(read_offset, buffer),
                (None, Some(buffer)) => self.read(buffer),
                _ => Err("einval"),
            }
        } else {
            match (offset, write_buffer) {
                (Some(write_offset), Some(buffer)) => self.write_at(write_offset, buffer),
                (None, Some(buffer)) => self.write(buffer),
                _ => Err("einval"),
            }
        }
    }

    pub fn set_len(&self, new_len: u64) -> Result<(), &'static str> {
        if !self.desc.read().unwrap().opt.wr {
            return Err("ebadf");
        }
        self.data.lock().unwrap().resize(new_len as usize, 0);
        Ok(())
    }

    //Note: a lot of useless functions below ...
    pub fn sync_all(&self) -> Result<(), &'static str> {
        Ok(())
    }

    pub fn sync_data(&self) -> Result<(), &'static str> {
        Ok(())
    }

    pub fn metadata_sz(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    pub fn lookup(&self, _path: &str, _depth: usize) -> Result<(), &'static str> {
        Ok(())
    }

    pub fn read_entry(&self) -> Result<String, &'static str> {
        let mut state = self.desc.write().unwrap();
        if !state.opt.rd {
            return Err("ebadf");
        }
        let entry_offset = state.off;
        state.off += 1;
        Ok(format!("entry_{}", entry_offset))
    }

    pub fn poll_status(&self) -> (bool, bool, bool) {
        (true, true, false)
    }

    pub fn io_ctl(&self, _cmd: u32, _arg: usize) -> Result<usize, &'static str> {
        Ok(0)
    }

    pub fn mmap(&self, _start: usize, _end: usize, _off: usize) -> Result<(), &'static str> {
        Ok(())
    }

    pub fn inode_ref(&self) -> Arc<Mutex<Vec<u8>>> {
        self.data.clone()
    }

    pub fn advise_readahead(&self, offset: usize, length: usize) -> Result<(), &'static str> {
        if length == 0 {
            return Ok(());
        }

        offset.checked_add(length).ok_or("einval")?;

        let contents = self.data.lock().unwrap();
        if offset >= contents.len() {
            return Ok(());
        }

        Ok(())
    }

    pub fn fallocate(&self, offset: usize, length: usize) -> Result<(), &'static str> {
        if !self.desc.read().unwrap().opt.wr {
            return Err("ebadf");
        }
        let mut contents = self.data.lock().unwrap();
        let required_len = offset.checked_add(length).ok_or("einval")?;
        if required_len > contents.len() {
            contents.resize(required_len, 0);
        }
        Ok(())
    }

    pub fn splice_to(&self, dst: &FHandle, count: usize) -> Result<usize, &'static str> {
        let source_offset = self.desc.read().unwrap().off;
        let source_data = self.data.lock().unwrap();
        if source_offset as usize >= source_data.len() {
            return Ok(0);
        }
        let available = source_data.len() - source_offset as usize;
        let bytes_to_splice = min(count, available);
        let chunk: Vec<u8> =
            source_data[source_offset as usize..source_offset as usize + bytes_to_splice].to_vec();
        drop(source_data);
        self.desc.write().unwrap().off += bytes_to_splice as u64;
        dst.write(&chunk)
    }
}

impl fmt::Debug for FHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let state = self.desc.read().unwrap();
        f.debug_struct("FH")
            .field("off", &state.off)
            .field("path", &self.path)
            .finish()
    }
}

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

/// File-descriptor object variants used by the simulation. Unifies regular files, pipes, and epoll instances under one type for the kernel.
///
/// Note: rewrite for simplicity, remove a lot of useless code.
pub enum FLike {
    File(FHandle),
    Pipe(PipeNode),
    Ep(EpInst),
}

impl Clone for FLike {
    fn clone(&self) -> Self {
        match self {
            FLike::File(file) => FLike::File(file.dup(file.cloexec)),
            FLike::Pipe(pipe) => FLike::Pipe(pipe.clone()),
            FLike::Ep(epoll) => FLike::Ep(epoll.clone()),
        }
    }
}

impl FLike {
    pub fn dup(&self, cloexec: bool) -> FLike {
        match self {
            FLike::File(file) => FLike::File(file.dup(cloexec)),
            FLike::Pipe(pipe) => FLike::Pipe(pipe.clone()),
            // Debug fix: epoll duplicates must share the full instance state,
            // including the registration map, ready set, and control-change set.
            FLike::Ep(epoll) => FLike::Ep(epoll.clone()),
        }
    }

    pub fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        if buffer.is_empty() {
            return Ok(0);
        }

        match self {
            FLike::File(file) => file.read(buffer),
            FLike::Pipe(pipe) => pipe.read_at(buffer),
            FLike::Ep(_) => Err("enosys"),
        }
    }

    pub fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        if buffer.is_empty() {
            return Ok(0);
        }

        match self {
            FLike::File(file) => file.write(buffer),
            FLike::Pipe(pipe) => pipe.write_at(buffer),
            FLike::Ep(_) => Err("enosys"),
        }
    }

    //additional controls
    pub fn io_ctl(&self, request: usize, arg: usize) -> Result<usize, &'static str> {
        match self {
            FLike::File(file) => match request as u32 {
                0..=0xFF => Ok(0),
                _ => file.io_ctl(request as u32, arg),
            },
            FLike::Pipe(_) => match request {
                FIONBIO => Ok(0),
                _ => Err("enotty"),
            },
            FLike::Ep(_) => Err("enosys"),
        }
    }

    pub fn mmap_fl(&self, start: usize, end: usize, offset: usize) -> Result<(), &'static str> {
        if start >= end {
            return Err("einval");
        }
        let len = end.checked_sub(start).ok_or("einval")?;
        // Debug fix: page-count rounding for huge ranges must not overflow.
        len.checked_add(PAGE_SZ - 1)
            .map(|rounded_len| rounded_len / PAGE_SZ)
            .ok_or("einval")?;

        match self {
            FLike::File(file) => file.mmap(start, end, offset),
            _ => Err("enosys"),
        }
    }

    //returns ready state
    pub fn poll(&self) -> (bool, bool, bool) {
        match self {
            FLike::File(file) => {
                let options = file.desc.read().unwrap().opt;
                let error = file.path.is_empty() && file.data.lock().unwrap().is_empty();
                (options.rd, options.wr, error)
            }
            FLike::Pipe(pipe) => pipe.poll(),
            FLike::Ep(epoll) => {
                let has_ready = !epoll.ready.lock().unwrap().is_empty();
                (has_ready, false, false)
            }
        }
    }
}

impl fmt::Debug for FLike {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FLike::File(handle) => write!(f, "F({:?})", handle),
            FLike::Pipe(_) => write!(f, "P"),
            FLike::Ep(_) => write!(f, "E"),
        }
    }
}

/// Looks like a file node, test-use only for now.
pub struct PseudoNode {
    pub content: Vec<u8>,
    pub ftype: u8,
}

impl PseudoNode {
    pub fn new(content: &str, file_type: u8) -> Self {
        Self {
            content: content.as_bytes().to_vec(),
            ftype: file_type,
        }
    }

    pub fn read_at(&self, offset: usize, buffer: &mut [u8]) -> usize {
        if offset >= self.content.len() {
            return 0;
        }
        let bytes_to_read = min(self.content.len() - offset, buffer.len());
        buffer[..bytes_to_read].copy_from_slice(&self.content[offset..offset + bytes_to_read]);
        bytes_to_read
    }

    pub fn write_at(&self, _offset: usize, _buffer: &[u8]) -> Result<usize, &'static str> {
        Err("nosup")
    }

    pub fn metadata_sz(&self) -> usize {
        self.content.len()
    }
}

/// User data carried by an epoll event.
#[derive(Clone, Copy)]
pub struct EpData {
    pub ptr: u64,
}

/// Registered epoll interest mask and caller data.
#[derive(Clone)]
pub struct EpEvent {
    pub events: u32,
    pub data: EpData,
}

impl EpEvent {
    pub const IN: u32 = 0x001;
    pub const OUT: u32 = 0x004;
    pub const ERR: u32 = 0x008;
    pub const HUP: u32 = 0x010;
    pub const PRI: u32 = 0x002;
    pub const RDNORM: u32 = 0x040;
    pub const RDBAND: u32 = 0x080;
    pub const WRNORM: u32 = 0x100;
    pub const WRBAND: u32 = 0x200;
    pub const MSG: u32 = 0x400;
    pub const RDHUP: u32 = 0x2000;
    pub const EXCL: u32 = 1 << 28;
    pub const WAKEUP: u32 = 1 << 29;
    pub const ONESHOT: u32 = 1 << 30;
    pub const ET: u32 = 1 << 31;

    pub fn has(&self, event_mask: u32) -> bool {
        (self.events & event_mask) != 0
    }
}

/// epoll control operation constants.
pub struct EpCtlOp;

impl EpCtlOp {
    pub const ADD: i32 = 1;
    pub const DEL: i32 = 2;
    pub const MOD: i32 = 3;
}

/// Shared epoll registration table.
#[derive(Clone)]
pub struct EpEventMap {
    inner: Arc<Mutex<BTreeMap<usize, EpEvent>>>,
}

impl EpEventMap {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn insert(&self, fd: usize, event: EpEvent) -> Option<EpEvent> {
        self.inner.lock().unwrap().insert(fd, event)
    }

    pub fn contains_key(&self, fd: &usize) -> bool {
        self.inner.lock().unwrap().contains_key(fd)
    }

    pub fn remove(&self, fd: &usize) -> Option<EpEvent> {
        self.inner.lock().unwrap().remove(fd)
    }
}

/// Minimal epoll instance state.
///
/// `events` stores registrations, `ready` stores ready fds, and `new_ctl`
/// tracks fds whose control state changed since the last wait path observed it.
#[derive(Clone)]
pub struct EpInst {
    pub events: EpEventMap,
    pub ready: Arc<Mutex<BTreeSet<usize>>>,
    pub new_ctl: Arc<Mutex<BTreeSet<usize>>>,
}

impl EpInst {
    pub fn new() -> Self {
        EpInst {
            events: EpEventMap::new(),
            ready: Arc::new(Mutex::new(BTreeSet::new())),
            new_ctl: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    pub fn control(&self, op: i32, fd: usize, event: &EpEvent) -> Result<(), &'static str> {
        match op {
            EpCtlOp::ADD => {
                // Debug fix: ADD must reject an fd that is already registered.
                if self.events.contains_key(&fd) {
                    return Err("eexist");
                }
                self.events.insert(fd, event.clone());
                self.new_ctl.lock().unwrap().insert(fd);
                Ok(())
            }
            EpCtlOp::MOD => {
                if self.events.contains_key(&fd) {
                    self.events.insert(fd, event.clone());
                    self.new_ctl.lock().unwrap().insert(fd);
                    Ok(())
                } else {
                    Err("eperm")
                }
            }
            EpCtlOp::DEL => {
                if self.events.remove(&fd).is_some() {
                    // Debug fix: DEL must remove all stale state for the fd.
                    self.ready.lock().unwrap().remove(&fd);
                    self.new_ctl.lock().unwrap().remove(&fd);
                    Ok(())
                } else {
                    Err("eperm")
                }
            }
            _ => Err("eperm"),
        }
    }
}

/// Termios-compatible terminal configuration exposed through ioctl-style calls.
///
/// Note: just copy from original code, not used.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrmIO {
    pub iflag: u32,
    pub oflag: u32,
    pub cflag: u32,
    pub lflag: u32,
    pub line: u8,
    pub cc: [u8; 32],
    pub ispeed: u32,
    pub ospeed: u32,
}

impl Default for TrmIO {
    fn default() -> Self {
        Self {
            iflag: 0o66402,
            oflag: 0o5,
            cflag: 0o2277,
            lflag: 0o105073,
            line: 0,
            cc: [
                3, 28, 127, 21, 4, 0, 1, 0, 17, 19, 26, 255, 18, 15, 23, 22, 255, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            ispeed: 0,
            ospeed: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct WinSz {
    pub row: u16,
    pub col: u16,
    pub xpx: u16,
    pub ypx: u16,
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

/// One cached page and its replacement/writeback metadata.
///
/// `pin_count` prevents eviction while callers hold a page, and `dirty` records
/// whether the page needs writeback before it can be considered clean.
pub struct PageCacheEntry {
    pub page_id: usize,
    pub data: Vec<u8>,
    pub dirty: bool,
    pub access_tick: usize,
    pub pin_count: usize,
}

/// Small LRU page cache used by the filesystem and disk-cache simulation.
///
/// `entries` stores cached pages by page id, while `lru_order` keeps ids from
/// least recently used to most recently used. Atomic counters track cache stats.
///
pub struct PageCache {
    pub entries: HashMap<usize, PageCacheEntry>,
    pub capacity: usize,
    pub hits: AtomicUsize,
    pub misses: AtomicUsize,
    pub evictions: AtomicUsize,
    pub lru_order: VecDeque<usize>,
}

impl PageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
            evictions: AtomicUsize::new(0),
            lru_order: VecDeque::new(),
        }
    }

    pub fn lookup(&mut self, page_id: usize) -> Option<&[u8]> {
        if self.entries.contains_key(&page_id) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            self.lru_order.retain(|&id| id != page_id);
            self.lru_order.push_back(page_id);
            if let Some(entry) = self.entries.get_mut(&page_id) {
                entry.access_tick = CLK.load(Ordering::Relaxed);
            }
            self.entries
                .get(&page_id)
                .map(|entry| entry.data.as_slice())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn insert(&mut self, page_id: usize, data: Vec<u8>) {
        // Debug fix: capacity is a hard upper bound, including zero-capacity caches.
        if self.capacity == 0 {
            return;
        }

        let already_cached = self.entries.contains_key(&page_id);
        if !already_cached && self.entries.len() >= self.capacity {
            // Debug fix: if every existing page is pinned, do not exceed capacity.
            if !self.evict_lru() {
                return;
            }
        }

        let entry = PageCacheEntry {
            page_id,
            data,
            dirty: false,
            access_tick: CLK.load(Ordering::Relaxed),
            pin_count: 0,
        };
        self.entries.insert(page_id, entry);
        self.lru_order.retain(|&id| id != page_id);
        self.lru_order.push_back(page_id);
    }

    pub fn evict_lru(&mut self) -> bool {
        let mut victim = None;
        for &page_id in self.lru_order.iter() {
            if let Some(entry) = self.entries.get(&page_id) {
                if entry.pin_count == 0 {
                    victim = Some(page_id);
                    break;
                }
            }
        }

        if let Some(page_id) = victim {
            self.entries.remove(&page_id);
            self.lru_order.retain(|&id| id != page_id);
            self.evictions.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn mark_dirty(&mut self, page_id: usize) {
        if let Some(entry) = self.entries.get_mut(&page_id) {
            entry.dirty = true;
        }
    }

    pub fn writeback_all(&mut self) -> usize {
        let mut writeback_count = 0;
        for entry in self.entries.values_mut() {
            if entry.dirty {
                entry.dirty = false;
                writeback_count += 1;
            }
        }
        writeback_count
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
            self.evictions.load(Ordering::Relaxed),
        )
    }

    pub fn pin(&mut self, page_id: usize) -> bool {
        if let Some(entry) = self.entries.get_mut(&page_id) {
            entry.pin_count += 1;
            true
        } else {
            false
        }
    }

    pub fn unpin(&mut self, page_id: usize) -> bool {
        if let Some(entry) = self.entries.get_mut(&page_id) {
            if entry.pin_count > 0 {
                entry.pin_count -= 1;
            }
            true
        } else {
            false
        }
    }

    pub fn invalidate(&mut self, page_id: usize) -> bool {
        if self.entries.remove(&page_id).is_some() {
            self.lru_order.retain(|&id| id != page_id);
            true
        } else {
            false
        }
    }

    pub fn flush_range(&mut self, start: usize, end: usize) -> usize {
        let mut flushed_count = 0;
        let page_ids: Vec<usize> = self
            .entries
            .keys()
            .filter(|&&page_id| page_id >= start && page_id < end)
            .copied()
            .collect();

        for page_id in page_ids {
            if let Some(entry) = self.entries.get_mut(&page_id) {
                if entry.dirty {
                    entry.dirty = false;
                    flushed_count += 1;
                }
            }
        }
        flushed_count
    }
}

/// One registered kernel object and its ownership/reference metadata.
/// parent_id is used to track dependency relationships.
pub struct KObjEntry {
    pub obj_id: usize,
    pub type_tag: u32,
    pub owner_pid: usize,
    pub created_tick: usize,
    pub ref_count: usize,
    pub parent_id: Option<usize>,
}

/// Global-style kernel object registry.
///
/// `objects` stores entries by object id, `type_index` accelerates lookup by
/// type tag, and `seq` generates monotonically increasing object ids.
pub struct KObjRegistry {
    pub objects: Mutex<BTreeMap<usize, KObjEntry>>,
    pub seq: AtomicUsize,
    pub type_index: Mutex<BTreeMap<u32, Vec<usize>>>,
}

impl KObjRegistry {
    pub fn new() -> Self {
        Self {
            objects: Mutex::new(BTreeMap::new()),
            seq: AtomicUsize::new(1),
            type_index: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn register(&self, type_tag: u32, owner_pid: usize) -> usize {
        let object_id = self.seq.fetch_add(1, Ordering::Relaxed);
        let entry = KObjEntry {
            obj_id: object_id,
            type_tag,
            owner_pid,
            created_tick: CLK.load(Ordering::Relaxed),
            ref_count: 1,
            parent_id: None,
        };
        self.objects.lock().unwrap().insert(object_id, entry);
        self.type_index
            .lock()
            .unwrap()
            .entry(type_tag)
            .or_insert_with(Vec::new)
            .push(object_id);
        object_id
    }

    pub fn register_child(&self, type_tag: u32, owner_pid: usize, parent_id: usize) -> usize {
        let object_id = self.seq.fetch_add(1, Ordering::Relaxed);
        let entry = KObjEntry {
            obj_id: object_id,
            type_tag,
            owner_pid,
            created_tick: CLK.load(Ordering::Relaxed),
            ref_count: 1,
            parent_id: Some(parent_id),
        };
        self.objects.lock().unwrap().insert(object_id, entry);
        self.type_index
            .lock()
            .unwrap()
            .entry(type_tag)
            .or_insert_with(Vec::new)
            .push(object_id);
        object_id
    }

    // Note: we do not remove children when a parent is removed.
    pub fn unregister(&self, object_id: usize) -> bool {
        let removed_entry = self.objects.lock().unwrap().remove(&object_id);
        if let Some(entry) = removed_entry {
            self.remove_from_type_index(entry.type_tag, object_id);
            true
        } else {
            false
        }
    }

    pub fn find_by_type(&self, type_tag: u32) -> Vec<usize> {
        self.type_index
            .lock()
            .unwrap()
            .get(&type_tag)
            .cloned()
            .unwrap_or_default()
    }

    pub fn dump_graph(&self) -> Vec<(usize, usize)> {
        let objects = self.objects.lock().unwrap();
        let mut dependency_edges = Vec::new();
        for (object_id, entry) in objects.iter() {
            if let Some(parent_id) = entry.parent_id {
                dependency_edges.push((parent_id, *object_id));
            }
        }
        dependency_edges
    }

    pub fn gc_sweep(&self) -> usize {
        let mut objects = self.objects.lock().unwrap();
        let dead_objects: Vec<usize> = objects
            .iter()
            .filter(|(_, entry)| entry.ref_count == 0)
            .map(|(object_id, _)| *object_id)
            .collect();
        let removed_count = dead_objects.len();

        for object_id in dead_objects {
            if let Some(entry) = objects.remove(&object_id) {
                self.remove_from_type_index(entry.type_tag, object_id);
            }
        }
        removed_count
    }

    pub fn ref_up(&self, object_id: usize) -> bool {
        let mut objects = self.objects.lock().unwrap();
        if let Some(entry) = objects.get_mut(&object_id) {
            entry.ref_count += 1;
            true
        } else {
            false
        }
    }

    pub fn ref_down(&self, object_id: usize) -> bool {
        let mut objects = self.objects.lock().unwrap();
        if let Some(entry) = objects.get_mut(&object_id) {
            entry.ref_count = entry.ref_count.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn count(&self) -> usize {
        self.objects.lock().unwrap().len()
    }

    pub fn owner_objects(&self, owner_pid: usize) -> Vec<usize> {
        self.objects
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, entry)| entry.owner_pid == owner_pid)
            .map(|(object_id, _)| *object_id)
            .collect()
    }

    fn remove_from_type_index(&self, type_tag: u32, object_id: usize) {
        if let Some(type_list) = self.type_index.lock().unwrap().get_mut(&type_tag) {
            type_list.retain(|&indexed_id| indexed_id != object_id);
        }
    }
}

/// One cached block in a hash chain.
pub struct CacheSlot {
    pub id: usize,
    pub payload: Vec<u8>,
    pub modified: bool,
}

/// One bucket of the block cache.
///
/// Note:The spin lock mirrors kernel-style short critical sections around the
/// per-chain item list. but the code actually uses a Mutex to protect the vector of items, so the spin lock is redundant.
pub struct CacheChain {
    pub lk: Spin,
    pub items: Mutex<Vec<CacheSlot>>,
}

impl CacheChain {
    pub fn new() -> Self {
        Self {
            lk: Spin::new(),
            items: Mutex::new(Vec::new()),
        }
    }
}

/// Hash-chain block cache used by the simulated disk path.
///
/// `width` is the number of chains.
///
/// Fix: clear some very strange useless code.
/// Note: there are still some redundant design and confusing code left for future refactor, but it will affect the behavior of the simulation.
pub struct BlockCache {
    pub chains: Vec<CacheChain>,
    pub width: usize,
}

impl BlockCache {
    pub fn new(width: usize) -> Self {
        let mut chains = Vec::with_capacity(width);
        for _ in 0..width {
            chains.push(CacheChain::new());
        }
        Self { chains, width }
    }

    fn chain_index(&self, block_id: usize) -> Option<usize> {
        if self.width == 0 {
            None
        } else {
            Some((block_id ^ (block_id >> 7)) % self.width)
        }
    }

    pub fn idx(&self, block_id: usize) -> usize {
        self.chain_index(block_id).unwrap_or(0)
    }

    pub fn fetch(&self, block_id: usize, latency: Duration) -> Option<Vec<u8>> {
        // Debug fix: zero-width caches are empty instead of panicking on modulo by zero.
        let chain_index = self.chain_index(block_id)?;
        let chain = &self.chains[chain_index];
        chain.lk.acquire();

        let cached_data = {
            let items = chain.items.lock().unwrap();
            items
                .iter()
                .find(|slot| slot.id == block_id)
                .map(|slot| slot.payload.clone())
        };
        if let Some(data) = cached_data {
            chain.lk.release();
            return Some(data);
        }

        let tick_before = CLK.load(Ordering::Relaxed);
        if latency.as_nanos() > 0 {
            thread::sleep(latency);
        }

        //Note: this looks like a design for simulation, so just kept.
        let block_data = {
            let mut payload = Vec::with_capacity(512);
            let seed = block_id.wrapping_mul(0x9E3779B9) ^ tick_before;
            for byte_offset in 0..512 {
                payload.push(((seed.wrapping_add(byte_offset)) & 0xFF) as u8);
            }
            payload
        };
        let result = block_data.clone();
        let slot = CacheSlot {
            id: block_id,
            payload: block_data,
            modified: false,
        };
        chain.items.lock().unwrap().push(slot);
        chain.lk.release();
        Some(result)
    }

    pub fn sync_all(&self, lock_owner_id: usize) {
        // Debug fix: use KernLock's recursive enter/leave path so an existing
        // owner keeps its previous lock state after this helper returns.
        GKL.enter(lock_owner_id);
        for chain in self.chains.iter() {
            chain.lk.acquire();
            {
                let mut items = chain.items.lock().unwrap();
                for slot in items.iter_mut() {
                    if slot.modified {
                        slot.modified = false;
                    }
                }
            }
            chain.lk.release();
        }
        GKL.leave();
    }

    pub fn invalidate(&self, block_id: usize) {
        // Debug fix: invalidation must use the same hash chain as fetch.
        let Some(chain_index) = self.chain_index(block_id) else {
            return;
        };
        let chain = &self.chains[chain_index];
        chain.lk.acquire();
        {
            let mut items = chain.items.lock().unwrap();
            items.retain(|slot| slot.id != block_id);
        }
        chain.lk.release();
    }

    pub fn total_entries(&self) -> usize {
        let mut total = 0;
        for chain in self.chains.iter() {
            chain.lk.acquire();
            total += chain.items.lock().unwrap().len();
            chain.lk.release();
        }
        total
    }

    pub fn dirty_count(&self) -> usize {
        let mut dirty_count = 0;
        for chain in self.chains.iter() {
            chain.lk.acquire();
            {
                let items = chain.items.lock().unwrap();
                for slot in items.iter() {
                    if slot.modified {
                        dirty_count += 1;
                    }
                }
            }
            chain.lk.release();
        }
        dirty_count
    }
    //Note: did not change this fornow this looks very strange, but maybe for simulation purpose(???)
    pub fn evict_cold(&self, max_age: usize) -> usize {
        let now = CLK.load(Ordering::Relaxed);
        let mut evicted_count = 0;
        for chain in self.chains.iter() {
            chain.lk.acquire();
            {
                let mut items = chain.items.lock().unwrap();
                let previous_len = items.len();
                items.retain(|slot| {
                    let age = now.wrapping_sub(slot.id.wrapping_mul(3));
                    !slot.modified || age < max_age
                });
                evicted_count += previous_len - items.len();
            }
            chain.lk.release();
        }
        evicted_count
    }
}

/// One mount mapping from a path prefix to a backing target.
#[derive(Clone, Debug)]
pub struct MountEntry {
    pub prefix: String,
    pub target: String,
}

/// Ordered mount table.
///
/// Entries are sorted by descending prefix length so the longest matching
/// mount point wins during resolution.
///
/// Fix: derived a helper function to canonicalize slashes, and remove some redundant code.
/// Note: cannot make sure if resolve is correct.
pub struct MountTable {
    pub entries: RwLock<Vec<MountEntry>>,
}

impl MountTable {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    pub fn bind(&self, prefix: &str, target: &str) {
        let mut entries = self.entries.write().unwrap();
        let already_bound = entries
            .iter()
            .any(|entry| entry.prefix == prefix && entry.target == target);
        if already_bound {
            return;
        }

        entries.push(MountEntry {
            prefix: prefix.to_string(),
            target: target.to_string(),
        });
        entries.sort_by(|left, right| right.prefix.len().cmp(&left.prefix.len()));
    }

    fn prefix_matches(prefix: &str, path: &str) -> bool {
        if prefix == "/" {
            return path.starts_with('/');
        }
        if !path.starts_with(prefix) {
            return false;
        }
        // Debug fix: `/mnt` must match `/mnt/file`, but not `/mnted/file`.
        path.len() == prefix.len() || path.as_bytes().get(prefix.len()) == Some(&b'/')
    }

    //remove redudant slashes.
    fn canonicalize_slashes(path: &str) -> String {
        let mut canonical = String::with_capacity(path.len());
        let mut previous_was_slash = false;
        for ch in path.chars() {
            if ch == '/' {
                if !previous_was_slash {
                    canonical.push(ch);
                }
                previous_was_slash = true;
            } else {
                canonical.push(ch);
                previous_was_slash = false;
            }
        }
        if canonical.is_empty() {
            path.to_string()
        } else {
            canonical
        }
    }

    pub fn resolve(&self, path: &str) -> Result<String, &'static str> {
        match self.find_mount(path) {
            Some(entry) => {
                let remaining_path = &path[entry.prefix.len()..];
                let resolved_suffix = self.resolve(remaining_path)?;
                let mut result =
                    String::with_capacity(entry.target.len() + 1 + resolved_suffix.len());
                result.push_str(&entry.target);
                result.push(':');
                result.push_str(&resolved_suffix);
                Ok(result)
            }
            None => Ok(Self::canonicalize_slashes(path)),
        }
    }

    pub fn unmount(&self, prefix: &str) -> bool {
        let mut entries = self.entries.write().unwrap();
        let previous_len = entries.len();
        entries.retain(|entry| entry.prefix != prefix);
        entries.len() < previous_len
    }

    pub fn list_mounts(&self) -> Vec<(String, String)> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .map(|entry| (entry.prefix.clone(), entry.target.clone()))
            .collect()
    }

    pub fn find_mount(&self, path: &str) -> Option<MountEntry> {
        let entries = self.entries.read().unwrap();
        let mut best_match: Option<&MountEntry> = None;
        let mut best_prefix_len = 0usize;

        for entry in entries.iter() {
            let prefix_len = entry.prefix.len();
            if prefix_len == 0 {
                continue;
            }
            if Self::prefix_matches(&entry.prefix, path) && prefix_len > best_prefix_len {
                best_prefix_len = prefix_len;
                best_match = Some(entry);
            }
        }

        best_match.cloned()
    }

    pub fn mount_count(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    pub fn has_prefix(&self, prefix: &str) -> bool {
        self.entries
            .read()
            .unwrap()
            .iter()
            .any(|entry| entry.prefix.as_bytes() == prefix.as_bytes())
    }
}

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
