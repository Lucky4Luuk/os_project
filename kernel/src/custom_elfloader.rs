use elfloader::*;

pub struct CustomElfLoader {
    vbase: u64, //Base offset for all loaded ELF files using this loader
}

impl CustomElfLoader {
    pub fn new(vbase: u64) -> Self {
        Self {
            vbase
        }
    }
}

impl ElfLoader for CustomElfLoader {
    fn allocate(&mut self, load_headers: LoadableHeaders) -> Result<(), &'static str> {
        let mut start_addr = 0;
        let mut end_addr = 0;
        for header in load_headers {
            let addr = self.vbase + header.virtual_addr();
            if start_addr > addr || start_addr == 0 { start_addr = addr; }
            if end_addr < addr + header.mem_size() { end_addr = addr + header.mem_size(); }
            info!(
                "allocate base = {:#X} size = {:#X} flags = {}",
                addr,
                header.mem_size(),
                header.flags()
            );
        }
        let alloc_len = end_addr - start_addr;
        let mut mapper = crate::memory::MAPPER.lock();
        let mut frame_allocator = crate::memory::FRAME_ALLOCATOR.lock();
        // info!("Mapping area:");
        // info!("|-Start: {:#X}", start_addr);
        // info!("|-End: {:#X}", start_addr + alloc_len);
        let mapped_area = crate::memory::alloc_user_memory(
            start_addr, //Page-align downwards
            alloc_len / 4096, //Get size in pages
            mapper.as_mut().unwrap(),
            frame_allocator.as_mut().unwrap(),
        ).expect("Failed to allocate pages for user memory!");
        info!("Mapped area:");
        info!("|-Start: {:#X}", mapped_area.start().as_u64());
        info!("|-End: {:#X}", mapped_area.end().as_u64() + 4096);
        Ok(())
    }

    fn relocate(&mut self, entry: &Rela<P64>) -> Result<(), &'static str> {
        let typ = TypeRela64::from(entry.get_type());
        let addr: *mut u64 = (self.vbase + entry.get_offset()) as *mut u64;

        panic!("Relocating not supported yet!");

        match typ {
            TypeRela64::R_RELATIVE => {
                // This is a relative relocation, add the offset (where we put our
                // binary in the vspace) to the addend and we're done.
                info!(
                    "R_RELATIVE *{:p} = {:#x}",
                    addr,
                    self.vbase + entry.get_addend()
                );
                Ok(())
            }
            _ => Err("Unexpected relocation encountered"),
        }
    }

    fn load(&mut self, flags: Flags, base: VAddr, region: &[u8]) -> Result<(), &'static str> {
        let start = self.vbase + base;
        let end = self.vbase + base + region.len() as u64;
        info!("load region into = {:#x} -- {:#x}", start, end);

        //Load region into new memory location
        unsafe {
            let mut offset = 0;
            for byte in region {
                crate::memory::memory_write(start + offset, byte);
                offset += 1;
            }
        }

        Ok(())
    }

    fn tls(
        &mut self,
        tdata_start: VAddr,
        _tdata_length: u64,
        total_size: u64,
        _align: u64
    ) -> Result<(), &'static str> {
        let tls_end = tdata_start +  total_size;
        info!("Initial TLS region is at = {:#x} -- {:#x}", tdata_start, tls_end);
        Ok(())
    }

}
