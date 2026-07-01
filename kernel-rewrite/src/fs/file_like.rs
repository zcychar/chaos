#![allow(unused_imports)]

use super::epoll::EpInst;
use super::file::FHandle;
use super::pipe::PipeNode;
use crate::consts::*;
use std::fmt;

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
