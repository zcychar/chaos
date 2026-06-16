use rcore_fs::vfs::MMapArea;
use rcore_memory::memory_set::handler::{Delay, File, Linear, Shared};
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::PAGE_SIZE;

use super::*;
use crate::consts::USER_STACK_OFFSET;
use crate::memory::GlobalFrameAlloc;

impl Syscall<'_> {
    pub fn sys_brk(&mut self, new_brk: usize) -> SysResult {
        let mut proc = self.process();
        let current = proc.brk;
        if new_brk == 0 {
            return Ok(current);
        }
        if new_brk < proc.brk_start {
            return Ok(current);
        }
        let aligned = new_brk
            .checked_add(PAGE_SIZE - 1)
            .map(|addr| addr & !(PAGE_SIZE - 1))
            .ok_or(SysError::ENOMEM)?;
        if aligned >= USER_STACK_OFFSET {
            return Err(SysError::ENOMEM);
        }

        let current_aligned = current
            .checked_add(PAGE_SIZE - 1)
            .map(|addr| addr & !(PAGE_SIZE - 1))
            .ok_or(SysError::ENOMEM)?;
        if aligned == current_aligned {
            proc.brk = aligned;
            return Ok(aligned);
        }

        let vm = proc.vm.clone();
        let mut vm = vm.lock();
        if aligned > current_aligned {
            if vm
                .iter()
                .any(|area| area.is_overlap_with(current_aligned, aligned))
            {
                return Err(SysError::ENOMEM);
            }
            vm.push(
                current_aligned,
                aligned,
                MemoryAttr::default().user(),
                Delay::new(GlobalFrameAlloc),
                "brk",
            );
        } else {
            vm.pop_with_split(aligned, current_aligned);
        }
        drop(vm);

        proc.brk = aligned;
        Ok(aligned)
    }

    pub fn sys_mmap(
        &mut self,
        addr: usize,
        len: usize,
        prot: usize,
        flags: usize,
        fd: usize,
        offset: usize,
    ) -> SysResult {
        let prot = MmapProt::from_bits_truncate(prot);
        let flags = MmapFlags::from_bits_truncate(flags);
        info!(
            "mmap: addr={:#x}, size={:#x}, prot={:?}, flags={:?}, fd={}, offset={:#x}",
            addr, len, prot, flags, fd as isize, offset
        );
        if len == 0 {
            return Err(SysError::EINVAL);
        }

        let mut proc = self.process();
        let mut addr = addr;
        if addr == 0 {
            // although NULL can be a valid address
            // but in C, NULL is regarded as allocation failure
            // so just skip it
            addr = PAGE_SIZE;
        }

        if flags.contains(MmapFlags::FIXED) {
            let end = addr.checked_add(len).ok_or(SysError::EINVAL)?;
            let aligned_end = end
                .checked_add(PAGE_SIZE - 1)
                .map(|end| end & !(PAGE_SIZE - 1))
                .ok_or(SysError::EINVAL)?;
            // we have to map it to addr, so remove the old mapping first
            self.vm().pop_with_split(addr, aligned_end);
        } else {
            addr = self.vm().find_free_area(addr, len);
        }
        let end = addr.checked_add(len).ok_or(SysError::EINVAL)?;
        let _aligned_end = end
            .checked_add(PAGE_SIZE - 1)
            .map(|end| end & !(PAGE_SIZE - 1))
            .ok_or(SysError::EINVAL)?;

        if flags.contains(MmapFlags::ANONYMOUS) {
            if flags.contains(MmapFlags::SHARED) {
                self.vm().push(
                    addr,
                    end,
                    prot.to_attr(),
                    Shared::new(GlobalFrameAlloc),
                    "mmap_anon_shared",
                );
                return Ok(addr);
            } else {
                self.vm().push(
                    addr,
                    end,
                    prot.to_attr(),
                    Delay::new(GlobalFrameAlloc),
                    "mmap_anon",
                );
                return Ok(addr);
            }
        } else {
            let file_like = proc.get_file_like(fd)?;
            let area = MMapArea {
                start_vaddr: addr,
                end_vaddr: end,
                prot: prot.bits(),
                flags: flags.bits(),
                offset,
            };
            file_like.mmap(area)?;
            Ok(addr)
        }
    }

    pub fn sys_mprotect(&mut self, addr: usize, len: usize, prot: usize) -> SysResult {
        let prot = MmapProt::from_bits_truncate(prot);
        info!(
            "mprotect: addr={:#x}, size={:#x}, prot={:?}",
            addr, len, prot
        );
        let _attr = prot.to_attr();

        // TODO: properly set the attribute of the area
        //        now some mut ptr check is fault
        let vm = self.vm();
        let memory_area = vm
            .iter()
            .find(|area| area.is_overlap_with(addr, addr + len));
        if memory_area.is_none() {
            return Err(SysError::ENOMEM);
        }
        Ok(0)
    }

    pub fn sys_munmap(&mut self, addr: usize, len: usize) -> SysResult {
        info!("munmap addr={:#x}, size={:#x}", addr, len);
        if len == 0 || addr % PAGE_SIZE != 0 {
            return Err(SysError::EINVAL);
        }
        let end = addr.checked_add(len).ok_or(SysError::EINVAL)?;
        let aligned_end = end
            .checked_add(PAGE_SIZE - 1)
            .map(|end| end & !(PAGE_SIZE - 1))
            .ok_or(SysError::EINVAL)?;
        self.vm().pop_with_split(addr, aligned_end);
        Ok(0)
    }
}

bitflags! {
    pub struct MmapProt: usize {
        /// Data cannot be accessed
        const NONE = 0;
        /// Data can be read
        const READ = 1 << 0;
        /// Data can be written
        const WRITE = 1 << 1;
        /// Data can be executed
        const EXEC = 1 << 2;
    }
}

#[cfg(target_arch = "mips")]
bitflags! {
    pub struct MmapFlags: usize {
        /// Changes are shared.
        const SHARED = 1 << 0;
        /// Changes are private.
        const PRIVATE = 1 << 1;
        /// Place the mapping at the exact address
        const FIXED = 1 << 4;
        /// The mapping is not backed by any file. (non-POSIX)
        const ANONYMOUS = 0x800;
    }
}

#[cfg(not(target_arch = "mips"))]
bitflags! {
    pub struct MmapFlags: usize {
        /// Changes are shared.
        const SHARED = 1 << 0;
        /// Changes are private.
        const PRIVATE = 1 << 1;
        /// Place the mapping at the exact address
        const FIXED = 1 << 4;
        /// The mapping is not backed by any file. (non-POSIX)
        const ANONYMOUS = 1 << 5;
    }
}

impl MmapProt {
    pub fn to_attr(self) -> MemoryAttr {
        let mut attr = MemoryAttr::default().user();
        if self.contains(MmapProt::EXEC) {
            attr = attr.execute();
        }
        // TODO: see sys_mprotect
        //        if !self.contains(MmapProt::WRITE) { attr = attr.readonly(); }
        attr
    }
}
