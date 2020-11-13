#![no_std]

#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

#![feature(abi_x86_interrupt)]
#![feature(wake_trait)]

#![feature(raw)]
#![feature(never_type)]
#![feature(naked_functions)]
#![feature(option_expect_none)]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(asm)]

#![feature(alloc_error_handler)]
#![feature(allocator_api)]

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;

extern crate rlibc;
extern crate core;
#[macro_use] extern crate alloc;

pub mod vga_buffer;
pub mod kernel_logger;
pub mod allocator;
pub mod memory;
pub mod gdt;
pub mod interrupts;
pub use interrupts::apic;
pub mod hardware;
pub mod acpi_controller;
pub mod multitasking;
pub mod userspace;

///////////////////////////////////////////////////////////////////////////////////////////////////
// Error handling
///////////////////////////////////////////////////////////////////////////////////////////////////
use core::panic::PanicInfo;
/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::vga_buffer::kernel_panic(info);
    loop {}
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Handles initialization of the kernel. For now, this only initializes the GDT and the interrupt IDT.
pub fn init() {
    //Enable syscall extensions on x86_64
    {
        let mut efer = x86_64::registers::model_specific::Efer::read();
        efer |= x86_64::registers::control::EferFlags::NO_EXECUTE_ENABLE;
        efer |= x86_64::registers::control::EferFlags::SYSTEM_CALL_EXTENSIONS;
        unsafe {
            x86_64::registers::model_specific::Efer::write(efer);
        }
    }

    gdt::init();
    interrupts::init_idt();
}
