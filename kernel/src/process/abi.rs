use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::{cmp::min, ptr::null};
use rcore_memory::{paging::PageTable, PAGE_SIZE};

pub struct ProcInitInfo {
    pub args: Vec<String>,
    pub envs: Vec<String>,
    pub auxv: BTreeMap<u8, usize>,
}

impl ProcInitInfo {
    pub unsafe fn push_at(&self, stack_top: usize) -> usize {
        self.try_push_at(stack_top).unwrap_or(0)
    }

    pub unsafe fn try_push_at(&self, stack_top: usize) -> Result<usize, &'static str> {
        let mut writer = StackWriter { sp: stack_top };
        self.push_to(&mut writer)
    }

    pub fn push_at_in_vm(&self, vm: &mut crate::memory::MemorySet, stack_top: usize) -> usize {
        self.try_push_at_in_vm(vm, stack_top).unwrap_or(0)
    }

    pub fn try_push_at_in_vm(
        &self,
        vm: &mut crate::memory::MemorySet,
        stack_top: usize,
    ) -> Result<usize, &'static str> {
        let mut writer = VmStackWriter { sp: stack_top, vm };
        self.push_to(&mut writer)
    }

    fn push_to<W: InitStackWriter>(&self, writer: &mut W) -> Result<usize, &'static str> {
        if self.args.is_empty() {
            return Err("init stack missing argv");
        }
        // from stack_top:
        // program name
        writer.push_str(&self.args[0])?;
        // environment strings
        let mut envs = Vec::with_capacity(self.envs.len());
        for arg in self.envs.iter() {
            writer.push_str(arg.as_str())?;
            envs.push(writer.sp());
        }
        // argv strings
        let mut argv = Vec::with_capacity(self.args.len());
        for arg in self.args.iter() {
            writer.push_str(arg.as_str())?;
            argv.push(writer.sp());
        }
        // auxiliary vector entries
        writer.push_slice(&[null::<u8>(), null::<u8>()])?;
        for (&type_, &value) in self.auxv.iter() {
            writer.push_slice(&[type_ as usize, value])?;
        }
        // envionment pointers
        writer.push_slice(&[null::<u8>()])?;
        writer.push_slice(envs.as_slice())?;
        // argv pointers
        writer.push_slice(&[null::<u8>()])?;
        writer.push_slice(argv.as_slice())?;
        // argc
        writer.push_slice(&[argv.len()])?;
        Ok(writer.sp())
    }
}

trait InitStackWriter {
    fn sp(&self) -> usize;
    fn push_slice<T: Copy>(&mut self, vs: &[T]) -> Result<(), &'static str>;
    fn push_str(&mut self, s: &str) -> Result<(), &'static str> {
        self.push_slice(&[b'\0'])?;
        self.push_slice(s.as_bytes())
    }
}

struct StackWriter {
    sp: usize,
}

impl InitStackWriter for StackWriter {
    fn sp(&self) -> usize {
        self.sp
    }

    fn push_slice<T: Copy>(&mut self, vs: &[T]) -> Result<(), &'static str> {
        use core::{
            mem::{align_of, size_of},
            slice,
        };
        if vs.is_empty() {
            return Ok(());
        }
        let size = vs
            .len()
            .checked_mul(size_of::<T>())
            .ok_or("init stack slice size overflow")?;
        self.sp = self
            .sp
            .checked_sub(size)
            .ok_or("init stack pointer underflow")?;
        self.sp = self
            .sp
            .checked_sub(self.sp % align_of::<T>())
            .ok_or("init stack alignment underflow")?;
        unsafe { slice::from_raw_parts_mut(self.sp as *mut T, vs.len()) }.copy_from_slice(vs);
        Ok(())
    }
}

struct VmStackWriter<'a> {
    sp: usize,
    vm: &'a mut crate::memory::MemorySet,
}

impl<'a> InitStackWriter for VmStackWriter<'a> {
    fn sp(&self) -> usize {
        self.sp
    }

    fn push_slice<T: Copy>(&mut self, vs: &[T]) -> Result<(), &'static str> {
        use core::mem::{align_of, size_of};
        if vs.is_empty() {
            return Ok(());
        }
        let size = vs
            .len()
            .checked_mul(size_of::<T>())
            .ok_or("init stack slice size overflow")?;
        self.sp = self
            .sp
            .checked_sub(size)
            .ok_or("init stack pointer underflow")?;
        self.sp = self
            .sp
            .checked_sub(self.sp % align_of::<T>())
            .ok_or("init stack alignment underflow")?;
        let bytes = unsafe { core::slice::from_raw_parts(vs.as_ptr() as *const u8, size) };
        self.write_bytes(bytes)
    }
}

impl<'a> VmStackWriter<'a> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let mut written = 0;
        while written < bytes.len() {
            let va = self
                .sp
                .checked_add(written)
                .ok_or("init stack address overflow")?;
            let page_start = va & !(PAGE_SIZE - 1);
            if !self
                .vm
                .handle_page_fault_ext(page_start, crate::memory::AccessType::write(true))
            {
                return Err("failed to prepare init stack page");
            }
            let page_offset = va - page_start;
            let len = min(PAGE_SIZE - page_offset, bytes.len() - written);
            {
                let page = self.vm.get_page_table_mut().get_page_slice_mut(page_start);
                page[page_offset..page_offset + len]
                    .copy_from_slice(&bytes[written..written + len]);
            }
            let flush_end = va
                .checked_add(len)
                .ok_or("init stack flush range overflow")?;
            self.vm
                .get_page_table_mut()
                .flush_cache_copy_user(va, flush_end, false);
            written += len;
        }
        Ok(())
    }
}

pub const AT_PHDR: u8 = 3;
pub const AT_PHENT: u8 = 4;
pub const AT_PHNUM: u8 = 5;
pub const AT_PAGESZ: u8 = 6;
pub const AT_BASE: u8 = 7;
pub const AT_ENTRY: u8 = 9;
