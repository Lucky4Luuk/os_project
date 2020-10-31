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

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

/// Handles initialization of the kernel. For now, this only initializes the GDT and the interrupt IDT.
pub fn init() {
    gdt::init();
    interrupts::init_idt();
}
