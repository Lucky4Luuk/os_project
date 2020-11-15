#![no_std]
#![no_main]

#![feature(asm)]

use core::panic::PanicInfo;
fn vga_println(str: &str) {
    //We don't actually care about what we print
    unsafe { asm!("mov rax, 'b'"); }
    unsafe { asm!("mov [0xb8000], rax"); }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga_println("userspace panic"); //&format!("{}", info)
    //Instead of just showing a character on the screen, this function should instead
    //call a syscall, so the kernel knows the process ran into an error
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        unsafe { asm!("nop"); }
    } //Do nothing for now
    // panic!("Hello, userspace!");
}

//Useless rn
fn main() {
    panic!("Hello, userspace!");
}
