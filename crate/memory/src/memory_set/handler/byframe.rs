use super::*;

#[derive(Debug, Clone)]
pub struct ByFrame<T: FrameAllocator> {
    allocator: T,
}

impl<T: FrameAllocator> MemoryHandler for ByFrame<T> {
    fn box_clone(&self) -> Box<dyn MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut dyn PageTable, addr: VirtAddr, attr: &MemoryAttr) {
        let target = self.allocator.alloc().expect("failed to allocate frame");
        let entry = pt.map(addr, target);
        attr.apply(entry);
    }

    fn unmap(&self, pt: &mut dyn PageTable, addr: VirtAddr) {
        let target = pt.get_entry(addr).expect("fail to get entry").target();
        self.allocator.dealloc(target);
        pt.unmap(addr);
    }

    fn clone_map(
        &self,
        pt: &mut dyn PageTable,
        src_pt: &mut dyn PageTable,
        addr: VirtAddr,
        attr: &MemoryAttr,
    ) {
        self.map(pt, addr, attr);
        let data = src_pt.get_page_slice_mut(addr);
        pt.get_page_slice_mut(addr).copy_from_slice(data);
    }

    fn handle_page_fault_ext(
        &self,
        pt: &mut dyn PageTable,
        addr: VirtAddr,
        access: super::AccessType,
    ) -> bool {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        entry.present() && access.check_access(entry)
    }
}

impl<T: FrameAllocator> ByFrame<T> {
    pub fn new(allocator: T) -> Self {
        ByFrame { allocator }
    }
}
