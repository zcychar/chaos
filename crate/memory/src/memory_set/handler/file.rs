use super::*;

/// Delay mapping a page to an area of a file.
#[derive(Clone)]
pub struct File<F, T> {
    pub file: F,
    pub mem_start: usize,
    pub file_start: usize,
    pub file_end: usize,
    pub allocator: T,
}

pub trait Read: Clone + Send + Sync + 'static {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize;
}

impl<F: Read, T: FrameAllocator> MemoryHandler for File<F, T> {
    fn box_clone(&self) -> Box<dyn MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut dyn PageTable, addr: usize, attr: &MemoryAttr) {
        let entry = pt.map(addr, 0);
        entry.set_present(false);
        attr.apply(entry);
    }

    fn unmap(&self, pt: &mut dyn PageTable, addr: usize) {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            self.allocator.dealloc(entry.target());
        }

        // PageTable::unmap requires page to be present
        entry.set_present(true);
        pt.unmap(addr);
    }

    fn clone_map(
        &self,
        pt: &mut dyn PageTable,
        src_pt: &mut dyn PageTable,
        addr: usize,
        attr: &MemoryAttr,
    ) {
        let entry = src_pt.get_entry(addr).expect("failed to get entry");
        if entry.present() && !attr.readonly {
            // eager map and copy data
            let data = src_pt.get_page_slice_mut(addr);
            let target = self.allocator.alloc().expect("failed to alloc frame");
            let entry = pt.map(addr, target);
            attr.apply(entry);
            pt.get_page_slice_mut(addr).copy_from_slice(data);
            pt.flush_cache_copy_user(addr, addr + data.len(), attr.execute);
        } else {
            // delay map
            self.map(pt, addr, attr);
        }
    }

    fn handle_page_fault_ext(
        &self,
        pt: &mut dyn PageTable,
        addr: usize,
        access: super::AccessType,
    ) -> bool {
        let addr = addr & !(PAGE_SIZE - 1);
        let entry = pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            // permission check.
            if access.check_access(entry) {
                return true;
            }
            // permisison check failed.
            error!(
                "Permission check failed at 0x{:x}, access = {:?}.",
                addr, access
            );
            return false;
        }
        let execute = entry.execute();
        let frame = match self.allocator.alloc() {
            Some(frame) => frame,
            None => return false,
        };
        entry.set_target(frame);
        entry.set_present(true);
        entry.update();

        let read_size = self.fill_data(pt, addr);
        if let Some(end) = addr.checked_add(read_size) {
            pt.flush_cache_copy_user(addr, end, execute);
        }
        true
    }
}

impl<F: Read, T: FrameAllocator> File<F, T> {
    fn fill_data(&self, pt: &mut dyn PageTable, addr: VirtAddr) -> usize {
        let data = pt.get_page_slice_mut(addr);
        data.iter_mut().for_each(|x| *x = 0);

        let page_end = match addr.checked_add(PAGE_SIZE) {
            Some(end) => end,
            None => return 0,
        };
        let file_len = match self.file_end.checked_sub(self.file_start) {
            Some(len) => len,
            None => return 0,
        };
        let mem_file_end = match self.mem_start.checked_add(file_len) {
            Some(end) => end,
            None => return 0,
        };
        let copy_start = addr.max(self.mem_start);
        let copy_end = page_end.min(mem_file_end);
        if copy_start >= copy_end {
            return 0;
        }

        let page_offset = copy_start - addr;
        let file_offset = match copy_start
            .checked_sub(self.mem_start)
            .and_then(|offset| self.file_start.checked_add(offset))
        {
            Some(offset) => offset,
            None => return 0,
        };
        let read_size = copy_end - copy_start;
        let read_size = self
            .file
            .read_at(file_offset, &mut data[page_offset..page_offset + read_size]);
        read_size
    }
}

impl<F, T> Debug for File<F, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.debug_struct("FileHandler")
            .field("mem_start", &self.mem_start)
            .field("file_start", &self.file_start)
            .field("file_end", &self.file_end)
            .finish()
    }
}
