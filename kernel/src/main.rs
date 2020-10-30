#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

//Test stuff
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use] extern crate log;
extern crate alloc;

use core::panic::PanicInfo;
use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};

use bootloader::{BootInfo, entry_point};

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
    use x86_64::{structures::paging::MapperAllSizes, VirtAddr};

    kernel_logger::init().expect("Failed to load the kernel logger!");
    debug!("Hello, world!");

    let regions = boot_info.memory_map.iter();
    let addr_ranges = regions.map(|r| r.range.start_addr()..r.range.end_addr());
    let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
    let available_memory = frame_addresses.count() * 4;
    debug!("Memory available: {} KiB", available_memory);

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    kernel::memory::update_physical_memory_offset(phys_mem_offset.as_u64());
    {
        let mut mapper = kernel::memory::MAPPER.lock();
        *mapper = unsafe { Some(kernel::memory::init(phys_mem_offset)) };
        let mut frame_allocator = kernel::memory::FRAME_ALLOCATOR.lock();
        *frame_allocator = unsafe {
            Some(BootInfoFrameAllocator::init(&boot_info.memory_map))
        };
        debug!("Mapper and frame allocator created!");
    }

    let mut mapper = kernel::memory::MAPPER.lock();
    let mut frame_allocator = kernel::memory::FRAME_ALLOCATOR.lock();

    kernel::init();
    kernel::allocator::init_heap(mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).expect("Heap initialization failed!");

    let acpi_controller = kernel::acpi_controller::AcpiController::new(phys_mem_offset.as_u64());

    match acpi_controller {
        Ok(controller) => {
            debug!("Found ACPI data!");
            controller.debug_print();

            controller.get_cpu();
            kernel::apic::update_ioapic_addr(*controller.get_io_apic_addr().iter().next().expect("Failed to get the first IOAPIC addr!") as u64);
        },
        Err(err) => {
            debug!("Did not find ACPI data :(");
            debug!("Reason: {:?}", err);
        },
    }

    kernel::interrupts::initialize_apic(0);

    debug!("[RTC] Sleeping for 2 seconds");
    kernel::hardware::rtc::sleep(2.0); //Sleep for 2 seconds
    debug!("[RTC] Sleep ended!");

    // debug!("[APIC] Sleeping for 2 seconds");
    // debug!("[APIC] Sleep ended!");

    hlt_loop();
}
