use crate::multitasking::{self, thread::Thread, with_scheduler};

global_asm!(include_str!("userspace.s"));

static mut USER_STACK_END: u64 = 0;

pub fn init() {
    // crate::gdt::setup_usermode();
    trace!("Usermode setup!");
    let mut mapper = crate::memory::MAPPER.lock();
    let mut frame_allocator = crate::memory::FRAME_ALLOCATOR.lock();
    let stack_bounds = crate::memory::alloc_user_stack(2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).expect("Failed to map user stack!");
    unsafe {
        // asm!("mov rsp, {0}", in(reg) stack_bounds.end().as_u64());
        // asm!("call asm_jump_usermode");        //Throws a GPF
        // asm!("call asm_jump_usermode_sysret"); //Throws a page fault

        // asm!("mov ax,0x1B
        // mov ds,ax
        // mov es,ax
        // mov fs,ax
        // mov gs,ax               //;we don't need to worry about SS. it's handled by iret
        //
        // mov rax, rsp
        // ;//mov rax, {0}
        // push 0x1B               //;user data segment with bottom 2 bits set for ring 3
        // push rax                //;push our current esp for the iret stack frame
        // pushfq
        // push 0x23               //;user code segment with bottom 2 bits set for ring 3
        // push init_userspace     //;Rust function
        // iretq"
        // , in(reg) stack_bounds.end().as_u64());

        USER_STACK_END = stack_bounds.end().as_u64();

        // asm!("cli"); //Disable interrupts
        asm!("mov rcx, {0}", in(reg) init_userspace as u64);
        asm!("pushfq");
        asm!("pop r11");
        asm!("sysret");
        // panic!("INIT_USERSPACE ADDR: 0x{:0X}", init_userspace as u64);
    }
}

//Pagefault occurs because this function is memory mapped to non-accessible page.
//Perhaps modify the page beforehand, using its address to find the corresponding page.
#[no_mangle]
pub extern "C" fn init_userspace() -> ! {
    unsafe { asm!("mov rsp, {0}", in(reg) USER_STACK_END); } //Jump to userspace stack
    panic!("aaaa");

    //Userspace threads
    // for _ in 0..10 {
    //     let thread = Thread::create(test_thread, 2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).unwrap();
    //     with_scheduler(|s| s.add_new_thread(thread));
    // }

    loop {}
}
