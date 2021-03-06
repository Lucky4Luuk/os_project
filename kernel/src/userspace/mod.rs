use x86_64::VirtAddr;

use crate::multitasking::{self, thread::Thread, with_scheduler};

global_asm!(include_str!("userspace.s"));

#[naked]
unsafe extern "C" fn syscall_entry_fn() {
    //TODO: Jump to kernel stack
    //      Load old IP from RCX
    //      Load syscall info from RDI

    //Handle syscall

    //TODO: Jump to task stack
    //      Load old IP into RCX
    //      Move back to RING 3
}

pub fn init() {
    crate::gdt::setup_usermode_gdt();
    x86_64::registers::model_specific::LStar::write(VirtAddr::new(syscall_entry_fn as u64));
    trace!("Usermode gdt setup!");

    let userspace_addr = 0xFF00_0000;
    // let userspace_addr = 0x1000_0000;

    // let test_elf = include_bytes!("../../test.elf");
    let test_elf = include_bytes!("../../../target/x86_64-os_project/release/userspace");
    let binary = elfloader::ElfBinary::new("test", test_elf).expect("Failed to load ELF file!");
    let mut loader = crate::custom_elfloader::CustomElfLoader::new(userspace_addr);
    binary.load(&mut loader).expect("Can't load the binary!");

    let entry_point = userspace_addr + binary.entry_point();
    info!("Entry point: {:#X}", entry_point);

    // panic!("INIT_USERSPACE ADDR: 0x{:0X}", entry_point);
    // loop {}

    let mut mapper = crate::memory::MAPPER.lock();
    let mut frame_allocator = crate::memory::FRAME_ALLOCATOR.lock();
    let stack_bounds = crate::memory::alloc_user_stack(2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).expect("Failed to map user stack!");

    info!("hello?");

    unsafe {
        asm!("cli"); //Disable interrupts
        asm!("mov rcx, {0}", in(reg) entry_point);
        // x86_64::registers::model_specific::LStar::write(VirtAddr::new(entry_point));
        // unsafe { asm!("mov rsp, {0}", in(reg) stack_bounds.end().as_u64()); } //Jump to userspace stack
        stack_bounds.switch_to();
        asm!("pushfq");
        asm!("pop r11");
        asm!("sysretq");
    }
}

//Pagefault occurs because this function is memory mapped to non-accessible page.
//Perhaps modify the page beforehand, using its address to find the corresponding page.
#[no_mangle]
pub extern "C" fn init_userspace() -> ! {
    panic!("Panic from userspace!");

    //Userspace threads
    // for _ in 0..10 {
    //     let thread = Thread::create(test_thread, 2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).unwrap();
    //     with_scheduler(|s| s.add_new_thread(thread));
    // }

    loop {}
}
