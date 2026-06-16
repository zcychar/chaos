use super::abi::{self, ProcInitInfo};
use crate::arch::paging::*;
use crate::fs::{FileHandle, FileLike, OpenOptions, FOLLOW_MAX_DEPTH};
use crate::ipc::SemProc;
use crate::memory::{
    phys_to_virt, ByFrame, Delay, File, GlobalFrameAlloc, KernelStack, MemoryAttr, MemorySet, Read,
};
use crate::sync::{SpinLock, SpinNoIrqLock as Mutex};
use crate::{
    signal::{Siginfo, Signal, SignalAction, SignalStack, Sigset},
    syscall::handle_syscall,
};
use alloc::{
    boxed::Box, collections::BTreeMap, collections::VecDeque, string::String, sync::Arc,
    sync::Weak, vec::Vec,
};
use bitflags::_core::cell::Ref;
use core::fmt;
use core::str;
use core::{
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    task::{Context, Poll},
};
use log::*;
use pc_keyboard::KeyCode::BackTick;
use rcore_fs::vfs::INode;
use rcore_memory::{Page, PAGE_SIZE};
use spin::RwLock;
use trapframe::TrapFrame;
use trapframe::UserContext;
use xmas_elf::{
    header,
    program::{Flags, SegmentData, Type},
    ElfFile,
};

trait ToMemoryAttr {
    fn to_attr(&self) -> MemoryAttr;
}

impl ToMemoryAttr for Flags {
    fn to_attr(&self) -> MemoryAttr {
        let mut flags = MemoryAttr::default().user();
        if self.is_execute() {
            flags = flags.execute();
        }
        if !self.is_write() {
            flags = flags.readonly();
        }
        flags
    }
}

/// Helper functions to process ELF file
pub trait ElfExt {
    /// Setup MemorySet according to the ELF file.
    fn make_memory_set(
        &self,
        ms: &mut MemorySet,
        inode: &Arc<dyn INode>,
    ) -> Result<usize, &'static str>;

    /// Get interpreter string if it has.
    fn get_interpreter(&self) -> Result<&str, &str>;

    /// Append current ELF file as interpreter into given memory set.
    /// This will insert the interpreter it a place which is "good enough" (since ld.so should be PIC).
    fn append_as_interpreter(
        &self,
        inode: &Arc<dyn INode>,
        memory_set: &mut MemorySet,
        bias: usize,
    ) -> Result<(), &'static str>;

    /// Get virtual address of PHDR section if it has.
    fn get_phdr_vaddr(&self) -> Result<Option<u64>, &'static str>;
}

impl ElfExt for ElfFile<'_> {
    fn make_memory_set(
        &self,
        ms: &mut MemorySet,
        inode: &Arc<dyn INode>,
    ) -> Result<usize, &'static str> {
        debug!("creating MemorySet from ELF");
        let mut farthest_memory: Option<usize> = None;
        for ph in self.program_iter() {
            if ph.get_type() != Ok(Type::Load) {
                continue;
            }
            let mem_start = u64_to_usize(ph.virtual_addr())?;
            let mem_size = u64_to_usize(ph.mem_size())?;
            let mem_end = mem_start
                .checked_add(mem_size)
                .ok_or("elf virtual range overflow")?;
            if mem_start >= mem_end {
                return Err("elf empty load range");
            }
            let file_start = u64_to_usize(ph.offset())?;
            let file_size = u64_to_usize(ph.file_size())?;
            if file_size > mem_size {
                return Err("elf file range exceeds memory range");
            }
            let file_end = file_start
                .checked_add(file_size)
                .ok_or("elf file range overflow")?;
            ms.push(
                mem_start,
                mem_end,
                ph.flags().to_attr(),
                File {
                    file: INodeForMap(inode.clone()),
                    mem_start,
                    file_start,
                    file_end,
                    allocator: GlobalFrameAlloc,
                },
                "elf",
            );
            farthest_memory =
                Some(farthest_memory.map_or(mem_end, |farthest| farthest.max(mem_end)));
        }

        let farthest_memory = farthest_memory.ok_or("elf has no load segment")?;
        let bias_addr = farthest_memory
            .checked_add(PAGE_SIZE)
            .ok_or("elf bias overflow")?;
        Ok(Page::of_addr(bias_addr).start_address())
    }
    fn append_as_interpreter(
        &self,
        inode: &Arc<dyn INode>,
        ms: &mut MemorySet,
        bias: usize,
    ) -> Result<(), &'static str> {
        debug!("inserting interpreter from ELF");

        let mut load_count = 0;
        for ph in self.program_iter() {
            if ph.get_type() != Ok(Type::Load) {
                continue;
            }
            let virtual_addr = u64_to_usize(ph.virtual_addr())?;
            let mem_start = virtual_addr
                .checked_add(bias)
                .ok_or("elf interpreter bias overflow")?;
            let mem_size = u64_to_usize(ph.mem_size())?;
            let mem_end = mem_start
                .checked_add(mem_size)
                .ok_or("elf interpreter virtual range overflow")?;
            if mem_start >= mem_end {
                return Err("elf interpreter empty load range");
            }
            let file_start = u64_to_usize(ph.offset())?;
            let file_size = u64_to_usize(ph.file_size())?;
            if file_size > mem_size {
                return Err("elf interpreter file range exceeds memory range");
            }
            let file_end = file_start
                .checked_add(file_size)
                .ok_or("elf interpreter file range overflow")?;
            ms.push(
                mem_start,
                mem_end,
                ph.flags().to_attr(),
                File {
                    file: INodeForMap(inode.clone()),
                    mem_start,
                    file_start,
                    file_end,
                    allocator: GlobalFrameAlloc,
                },
                "elf-interp",
            );
            load_count += 1;
        }
        if load_count == 0 {
            return Err("elf interpreter has no load segment");
        }
        Ok(())
    }
    fn get_interpreter(&self) -> Result<&str, &str> {
        let header = self
            .program_iter()
            .filter(|ph| ph.get_type() == Ok(Type::Interp))
            .next()
            .ok_or("no interp header")?;
        let mut data = match header.get_data(self)? {
            SegmentData::Undefined(data) => data,
            _ => unreachable!(),
        };
        // skip NULL
        while let Some(0) = data.last() {
            data = &data[..data.len() - 1];
        }
        let path = str::from_utf8(data).map_err(|_| "failed to convert to utf8")?;
        Ok(path)
    }

    fn get_phdr_vaddr(&self) -> Result<Option<u64>, &'static str> {
        if let Some(phdr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Phdr))
        {
            // if phdr exists in program header, use it
            Ok(Some(phdr.virtual_addr()))
        } else if let Some(elf_addr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Load) && ph.offset() == 0)
        {
            // otherwise, check if elf is loaded from the beginning, then phdr can be inferred.
            Ok(Some(
                elf_addr
                    .virtual_addr()
                    .checked_add(self.header.pt2.ph_offset())
                    .ok_or("elf phdr overflow")?,
            ))
        } else {
            warn!("elf: no phdr found, tls might not work");
            Ok(None)
        }
    }
}

fn u64_to_usize(value: u64) -> Result<usize, &'static str> {
    if value > usize::MAX as u64 {
        return Err("elf value does not fit usize");
    }
    Ok(value as usize)
}

#[derive(Clone)]
pub struct INodeForMap(pub Arc<dyn INode>);

impl Read for INodeForMap {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.0.read_at(offset, buf).unwrap()
    }
}
