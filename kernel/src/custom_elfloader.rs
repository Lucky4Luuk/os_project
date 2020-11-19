use elfloader::*;

use crate::memory::memory_write;

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
        for header in load_headers {
            let addr = self.vbase + header.virtual_addr();
            info!(
                "allocate base = {:#X} size = {:#X} flags = {}",
                addr,
                header.mem_size(),
                header.flags()
            );
            let mut mapper = crate::memory::MAPPER.lock();
            let mut frame_allocator = crate::memory::FRAME_ALLOCATOR.lock();
            let mapped_area = crate::memory::alloc_user_memory(
                addr, //No need to align it, done in function
                header.mem_size() / 4096 + 1, //Get size in pages
                mapper.as_mut().unwrap(),
                frame_allocator.as_mut().unwrap(),
            ).expect("Failed to allocate pages for user memory!");

            //Zero the data
            let size = header.mem_size() - header.mem_size() % 4096 + 4096; //Align up
            for i in 0..size {
                unsafe { memory_write(addr + i, 0u8); }
            }
        }

        Ok(())
    }

    fn relocate(&mut self, entry: &Rela<P64>) -> Result<(), &'static str> {
        let typ = TypeRela64::from(entry.get_type());
        let addr = self.vbase + entry.get_offset();

        match typ {
            TypeRela64::R_RELATIVE => {
                // This is a relative relocation, add the offset (where we put our
                // binary in the vspace) to the addend and we're done.
                info!(
                    "R_RELATIVE *{:p} = {:#x}",
                    addr as *mut u64,
                    self.vbase + entry.get_addend()
                );

                unsafe { memory_write(addr, self.vbase + entry.get_addend()); }

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
                memory_write(start + offset, byte);
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
