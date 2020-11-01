#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

//Test stuff
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use] extern crate log;
#[macro_use] extern crate alloc;

use core::panic::PanicInfo;
use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};

use bootloader::{BootInfo, entry_point};

use raw_cpuid::CpuId;

use kernel::{
    print,
    println,

    hlt_loop,

    kernel_logger,
};

///////////////////////////////////////////////////////////////////////////////////////////////////
// Error handling
///////////////////////////////////////////////////////////////////////////////////////////////////
/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Main function
///////////////////////////////////////////////////////////////////////////////////////////////////
entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use kernel::memory::BootInfoFrameAllocator;
    use kernel::vga_buffer::ModeEnum;
    use vga::writers::{Text80x25, Graphics320x200x256, Graphics640x480x16, GraphicsWriter};
    use vga::colors::Color16;
    use x86_64::{structures::paging::MapperAllSizes, VirtAddr};

    kernel::vga_buffer::set_mode(ModeEnum::Graphics640x480x16(
        Graphics640x480x16::new()
    ));

    // match kernel::vga_buffer::WRITER.lock().mode {
    //     ModeEnum::Graphics640x480x16(m) => {
    //         for x in 0..640 {
    //             for y in 0..480 {
    //                 let mut color = Color16::Black;
    //                 if (x + y) % 7 == 0 { color = Color16::DarkGrey; }
    //                 if (x + y) % 14 == 0 { color = Color16::Brown; }
    //                 m.set_pixel(x,y, color);
    //             }
    //         }
    //     },
    //     _ => {},
    // }

    kernel_logger::init().expect("Failed to load the kernel logger!");
    println!("Hello, world!");

    let regions = boot_info.memory_map.iter();
    let addr_ranges = regions.map(|r| r.range.start_addr()..r.range.end_addr());
    let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
    let available_memory = frame_addresses.count() * 4;
    println!("Memory available: {} KiB", available_memory);

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    kernel::memory::update_physical_memory_offset(phys_mem_offset.as_u64());
    {
        let mut mapper = kernel::memory::MAPPER.lock();
        *mapper = unsafe { Some(kernel::memory::init(phys_mem_offset)) };
        let mut frame_allocator = kernel::memory::FRAME_ALLOCATOR.lock();
        *frame_allocator = unsafe {
            Some(BootInfoFrameAllocator::init(&boot_info.memory_map))
        };
        println!("Mapper and frame allocator created!");
    }

    let mut mapper = kernel::memory::MAPPER.lock();
    let mut frame_allocator = kernel::memory::FRAME_ALLOCATOR.lock();

    kernel::init();
    kernel::allocator::init_heap(mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).expect("Heap initialization failed!");

    let cpuid = CpuId::new();
    match cpuid.get_vendor_info() {
        Some(vf) => debug!("CPU vendor: {}", vf.as_string()),
        None => warn!("Failed to find the CPU vendor!"),
    }

    match cpuid.get_feature_info() {
        Some(features) => {
            debug!("CPU has APIC: {}", features.has_apic());
            debug!("CPU has TSC: {}", features.has_tsc());
        },
        None => warn!("Failed to find the CPU's feature info!"),
    }

    match cpuid.get_tsc_info() {
        Some(tsc_info) => {
            match tsc_info.tsc_frequency() {
                Some(freq) => debug!("TSC freq: {}", freq),
                None => warn!("TSC freg is unknown!"),
            }
        },
        None => warn!("Failed to find the CPU's TSC info!"),
    }

    let acpi_controller = kernel::acpi_controller::AcpiController::new(phys_mem_offset.as_u64());

    match acpi_controller {
        Ok(controller) => {
            debug!("Found ACPI data!");
            // controller.debug_print();

            // controller.get_cpu();
            trace!("APIC_ADDR: {:#08X}", controller.get_apic_addr());
            kernel::apic::update_ioapic_addr(*controller.get_io_apic_addr().iter().next().expect("Failed to get the first IOAPIC addr!") as u64);

            let hpet_info = controller.get_hpet_info();
            trace!("HPET_ADDR: {:#08X}", hpet_info.base_address);
            kernel::hardware::hpet::initialize_hpet();
        },
        Err(err) => {
            debug!("Did not find ACPI data :(");
            debug!("Reason: {:?}", err);
        },
    }

    kernel::interrupts::initialize_apic(0);

    debug!("[RTC] Sleeping for 2 seconds");
    debug!("RDTSC value: {}", kernel::hardware::rdtsc::read_rdtsc());
    kernel::hardware::rtc::sleep(2.0); //Sleep for 2 seconds
    let res = kernel::hardware::rdtsc::read_rdtsc();
    debug!("[RTC] Sleep ended!");
    debug!("RDTSC value: {}", res);

    // debug!("[APIC] Sleeping for 2 seconds");
    // debug!("[APIC] Sleep ended!");

    hlt_loop();
}
