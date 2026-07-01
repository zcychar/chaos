#![allow(unused_imports)]

use crate::consts::*;
use std::cmp::min;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

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
pub(crate) struct FdState {
    off: u64,
    pub(crate) opt: FdOpt,
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
    pub(crate) desc: Arc<RwLock<FdState>>,
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
