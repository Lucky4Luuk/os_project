use crate::multitasking::{self, thread::Thread, with_scheduler};

global_asm!(include_str!("userspace.s"));

pub fn init() {
    crate::gdt::setup_usermode();
    trace!("Usermode setup!");
    let mut mapper = crate::memory::MAPPER.lock();
    let mut frame_allocator = crate::memory::FRAME_ALLOCATOR.lock();
    let stack_bounds = crate::memory::alloc_user_stack(2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).expect("Failed to map user stack!");
    unsafe {
        // asm!("cli"); //Disable interrupts
        asm!("mov rcx, {0}", in(reg) init_userspace as u64);
        asm!("pushfq");
        asm!("pop r11");
        unsafe { asm!("mov rsp, {0}", in(reg) stack_bounds.end().as_u64()); } //Jump to userspace stack
        asm!("sysretq");
        // panic!("INIT_USERSPACE ADDR: 0x{:0X}", init_userspace as u64);
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
