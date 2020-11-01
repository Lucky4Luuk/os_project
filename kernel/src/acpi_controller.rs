use acpi::{
    AcpiHandler, PhysicalMapping,
    AcpiTables, AcpiError,
};

use aml::{
    AmlContext,
    AmlName,
};

use x86_64::{
    structures::paging::{
        mapper,
        PageTable,
        OffsetPageTable,
        Page,
        PageSize,
        PhysFrame,
        Mapper,
        Size4KiB,
        FrameAllocator,
        PageTableFlags as Flags,
    },
    VirtAddr,
    PhysAddr,
};

use crate::println;

use core::ptr::NonNull;
use alloc::alloc::{Layout, alloc, dealloc};
use alloc::vec::Vec;

fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

#[derive(Copy, Clone)]
pub struct AcpiMemoryHandler {
    pub phys_mem_offset: u64,
}

pub struct AcpiController {
    pub phys_mem_offset: u64,
    pub acpi: AcpiTables<AcpiMemoryHandler>,
    pub aml: AmlContext,
}

impl AcpiController {
    // TODO: Error handling
    pub fn new(phys_mem_offset: u64) -> Result<Self, AcpiError> {
        let acpi_data = {
            let acpi_handler = AcpiMemoryHandler {
                phys_mem_offset: phys_mem_offset,
            };
            unsafe { AcpiTables::search_for_rsdp_bios(acpi_handler) }
        }?;

        let mut aml_context = AmlContext::new();
        match &acpi_data.dsdt {
            Some(dsdt) => {
                debug!("DSDT table located!");
                let address = phys_mem_offset + dsdt.address as u64;
                let stream = unsafe { core::slice::from_raw_parts(address as *mut u8, dsdt.length as usize) };
                // aml_context.parse_table(stream).expect("Failed to load AML table!");
                match aml_context.parse_table(stream) {
                    Ok(ok) => debug!("DSDT AML table parsed!"),
                    Err(err) => error!("DSDT AML table failed to parse!"),
                }
                debug!("DSDT table parsed");
            },
            None => {},
        }

        for ssdt in &acpi_data.ssdts {
            debug!("SSDT table located!");
            let address = phys_mem_offset + ssdt.address as u64;
            let stream = unsafe { core::slice::from_raw_parts(address as *mut u8, ssdt.length as usize) };
            // aml_context.parse_table(stream).expect("Failed to load AML table!");
            match aml_context.parse_table(stream) {
                Ok(ok) => debug!("SSDT AML table parsed!"),
                Err(err) => error!("SSDT AML table failed to parse!"),
            }
            debug!("SSDT table parsed");
        }

        Ok(Self {
            phys_mem_offset: phys_mem_offset,
            acpi: acpi_data,
            aml: aml_context,
        })
    }

    pub fn get_cpu(&self) -> crate::hardware::cpu::CPU {
        let cpu_name = aml::AmlName::from_str(r"\_SB_.CPUS").expect("Failed to parse CPU!"); //TODO: Perhaps more error checking?
        let cpu_value = self.aml.namespace.get_by_path(&cpu_name).expect("Failed to get CPU device!"); //TODO: Perhaps more error checking?

        // let cpu_core_count = self.acpi.application_processors.len() + 1; //+1 because base processor
        let platform_info = self.acpi.platform_info().expect("Failed to get platform info!");
        let processor_info = match platform_info.processor_info {
            Some(info) => info,
            None => panic!("Failed to get processor info!"),
        };

        let cpu_core_count = processor_info.application_processors.len() + 1;

        let mut cpu = crate::hardware::cpu::CPU::new();

        for i in 0..cpu_core_count {
            let acpi_core: acpi::platform::Processor = {
                if i == 0 { processor_info.boot_processor }
                else { processor_info.application_processors[i - 1] }
            };

            let cpu_aml_address = format!(r"\_SB_.CPUS.C{:03}", i);
            let core = self.aml.namespace.get_by_path(&aml::AmlName::from_str(&cpu_aml_address).unwrap()).expect("Failed to get CPU core!");

            trace!("CPU{:03}: {:?}", i, core);
            trace!("{}", acpi_core.processor_uid);

            match core {
                aml::AmlValue::Processor{id, pblk_address, pblk_len} => {
                        let processor = crate::hardware::cpu::Processor {
                        id: *id,
                        apic_id: acpi_core.local_apic_id,

                        pblk_address: *pblk_address,
                        pblk_len: *pblk_len,

                        is_ap: acpi_core.is_ap,
                        state: acpi_core.state,
                    };

                    cpu.processors.push(processor);
                },
                _ => {},
            }
        }

        cpu
    }

    pub fn get_hpet_info(&self) -> acpi::HpetInfo {
        acpi::HpetInfo::new(&self.acpi).expect("ACPI table has no information on HPET!")
    }

    pub fn get_apic_addr(&self) -> u64 {
        let platform_info = self.acpi.platform_info().expect("Failed to get platform info!");
        let interrupt_model = platform_info.interrupt_model;
        match interrupt_model {
            acpi::platform::InterruptModel::Apic(apic) => {
                return apic.local_apic_address
            },
            acpi::platform::InterruptModel::Unknown => println!("Did not find APIC!"),
            _ => {},
        }
        panic!("Failed to locate APIC address! Is it supported on this system?");
    }

    pub fn get_io_apic_addr(&self) -> Vec<u32> {
        let platform_info = self.acpi.platform_info().expect("Failed to get platform info!");
        let interrupt_model = platform_info.interrupt_model;
        let mut result = Vec::new();
        match interrupt_model {
            acpi::platform::InterruptModel::Apic(apic) => {
                for io_apic in &apic.io_apics {
                    result.push(io_apic.address);
                }
            },
            acpi::platform::InterruptModel::Unknown => println!("Did not find APIC!"),
            _ => {},
        }
        result
    }

    pub fn debug_print(&self) {
        let platform_info = self.acpi.platform_info().expect("Failed to get platform info!");
        let processor_info = match platform_info.processor_info {
            Some(info) => info,
            None => panic!("Failed to get processor info!"),
        };
        let interrupt_model = platform_info.interrupt_model;

        println!("=====ACPI=====");

        // println!("ACPI revision: {}", self.acpi.acpi_revision);

        println!("Boot processor: {:?}", processor_info.boot_processor);

        println!("AP count: {}", processor_info.application_processors.len());
        for processor in &processor_info.application_processors {
            println!("AP: {:?}", processor);
        }

        println!("SSDT count: {}", self.acpi.ssdts.len());

        println!("Power profile: {:?}", platform_info.power_profile);

        println!("=====++++=====");

        println!("");

        let interrupt_model = interrupt_model;
        match interrupt_model {
            acpi::platform::InterruptModel::Apic(apic) => {
                println!("APIC_addr: 0x{:x}", apic.local_apic_address);
                for io_apic in &apic.io_apics {
                    println!("io_apic_addr_{}: 0x{:x}", io_apic.id, io_apic.address);
                }
            },
            acpi::platform::InterruptModel::Unknown => println!("Did not find APIC!"),
            _ => {},
        }
    }
}

impl AcpiHandler for AcpiMemoryHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize
    ) -> PhysicalMapping<Self, T> {
        // `physical_address` might not be page aligned, so padding might be needed
        // The size of the allocated memory needs to be the same as or bigger than size_of::<T>()
        // `size` should contain the size of T in bytes, I think, so I'll simply allocate that

        let virtual_start = self.phys_mem_offset + physical_address as u64;

        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: core::ptr::NonNull::new_unchecked(virtual_start as *mut u8 as *mut T),
            region_length: size,
            mapped_length: size,
            handler: Self {
                phys_mem_offset: self.phys_mem_offset,
            },
        }
    }

    fn unmap_physical_region<T>(&self, region: &PhysicalMapping<Self, T>) {
        // Unmap the given physical region
    }
}
