#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

//Test stuff
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use] extern crate log;
#[macro_use] extern crate alloc;

use core::panic::PanicInfo;
use core::sync::atomic::Ordering;
use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};

use bootloader::{BootInfo, entry_point};

use raw_cpuid::CpuId;

use kernel::{
    print,
    println,

    hlt_loop,

    kernel_logger,

    multitasking::{self, thread::Thread, with_scheduler},
};

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

    // kernel::vga_buffer::set_mode(ModeEnum::Text80x25(
    //     Text80x25::new()
    // ));

    // match kernel::vga_buffer::WRITER.lock().mode {
    //     ModeEnum::Graphics640x480x16(m) => {
    //         for x in 0..640 {
    //             for y in 0..480 {
    //                 let mut color = Color16::Black;
    //                 if (x + y) % 7 == 0 { color = Color16::DarkGrey; }
    //                 if (x + y) % 14 == 0 { color = Color16::Blue; }
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

    {
        let mut mapper = kernel::memory::MAPPER.lock();
        let mut frame_allocator = kernel::memory::FRAME_ALLOCATOR.lock();
        kernel::allocator::init_heap(mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).expect("Heap initialization failed!");
    }

    // panic!("Test panic");

    kernel::init();

    let cpuid = CpuId::new();
    // match cpuid.get_vendor_info() {
    //     Some(vf) => debug!("CPU vendor: {}", vf.as_string()),
    //     None => warn!("Failed to find the CPU vendor!"),
    // }
    //
    // match cpuid.get_feature_info() {
    //     Some(features) => {
    //         debug!("CPU has APIC: {}", features.has_apic());
    //         debug!("CPU has TSC: {}", features.has_tsc());
    //     },
    //     None => warn!("Failed to find the CPU's feature info!"),
    // }

    /*
    match cpuid.get_tsc_info() {
        Some(tsc_info) => {
            match tsc_info.tsc_frequency() {
                Some(freq) => debug!("TSC freq: {}", freq),
                None => warn!("TSC freg is unknown!"),
            }
        },
        None => warn!("Failed to find the CPU's TSC info!"),
    }
    */

    let acpi_controller = kernel::acpi_controller::AcpiController::new(phys_mem_offset.as_u64()).expect("Failed to get ACPI data!");

    debug!("Found ACPI data!");

    {
        let mut ioapics = kernel::interrupts::ioapic::IOAPICS.lock();
        *ioapics = acpi_controller.get_io_apic();
    }

    x86_64::instructions::interrupts::without_interrupts(|| {
        kernel::interrupts::initialize_apic(0, acpi_controller.get_io_apic_iso());
    });

    let hpet_info = acpi_controller.get_hpet_info();
    // trace!("HPET_ADDR: {:#08X}", hpet_info.base_address);
    kernel::hardware::hpet::HPET_BASE_ADDR.store(hpet_info.base_address as u64, Ordering::Relaxed);
    kernel::hardware::hpet::initialize_hpet();

    // debug!("[RTC] Sleeping for 2 seconds");
    // debug!("RDTSC value: {}", kernel::hardware::rdtsc::read_rdtsc());
    // kernel::hardware::rtc::sleep(2.0); //Sleep for 2 seconds
    // let res = kernel::hardware::rdtsc::read_rdtsc();
    // debug!("[RTC] Sleep ended!");
    // debug!("RDTSC value: {}", res);

    // hlt_loop();

    //Kernel space threads
    {
        let mut mapper = kernel::memory::MAPPER.lock();
        let mut frame_allocator = kernel::memory::FRAME_ALLOCATOR.lock();

        let idle_thread = Thread::create(idle_thread, 2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).unwrap();
        with_scheduler(|s| s.set_idle_thread(idle_thread));

        let test_thread = Thread::create(test_thread, 2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).unwrap();
        with_scheduler(|s| s.add_new_thread(test_thread));
    }

    // Testing loading code at runtime. Pagefaults right now lol
    /*{
        let userspace_addr = 0xFF00_0000;

        let test_elf = include_bytes!("../../target/x86_64-os_project/release/userspace");
        let binary = elfloader::ElfBinary::new("test", test_elf).expect("Failed to load ELF file!");
        let mut loader = kernel::custom_elfloader::CustomElfLoader::new(userspace_addr);
        binary.load(&mut loader).expect("Can't load the binary!");

        let entry_point = userspace_addr + binary.entry_point();
        info!("Entry point: {:#X}", entry_point);
        let entry_point_fn = unsafe {
            core::mem::transmute::<u64, fn() -> !>(entry_point)
        };

        let mut mapper = kernel::memory::MAPPER.lock();
        let mut frame_allocator = kernel::memory::FRAME_ALLOCATOR.lock();

        let user_thread = Thread::create(entry_point_fn, 2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).unwrap();
        with_scheduler(|s| s.add_new_thread(user_thread));
    }*/

    //Userspace
    // kernel::userspace::init();
    thread_entry();

    hlt_loop();
}

fn idle_thread() -> ! {
    loop {
        x86_64::instructions::hlt();
        multitasking::yield_now();
    }
}

fn test_thread() -> ! {
    loop {
        let thread_id = with_scheduler(|s| s.current_thread_id()).as_u64();
        if (thread_id % 2) > 0 {
            print!("A");
        } else {
            print!("B");
        }
        multitasking::yield_now(); //Manually yield for performance
        // x86_64::instructions::hlt(); //Alternatively, run hlt so interrupts still work
    }
}

fn thread_entry() -> ! {
    let thread_id = with_scheduler(|s| s.current_thread_id()).as_u64();
    for _ in 0..=thread_id {
        print!("{}", thread_id);
        x86_64::instructions::hlt();
    }
    multitasking::exit_thread();
}
