use crate::multitasking::{self, thread::Thread, with_scheduler};

global_asm!(include_str!("userspace.s"));

pub fn init() {
    crate::gdt::setup_usermode();
    trace!("Usermode setup!");
    // let mut mapper = crate::memory::MAPPER.lock();
    // let mut frame_allocator = crate::memory::FRAME_ALLOCATOR.lock();
    // let stack_bounds = crate::memory::alloc_user_stack(2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).expect("Failed to map user stack!");
    unsafe {
        asm!("call asm_jump_usermode"); //Throws a GPF
    }
}

#[no_mangle]
pub extern "C" fn init_userspace() {
    info!("Userspace activated!");

    //Userspace threads
    // for _ in 0..10 {
    //     let thread = Thread::create(test_thread, 2, mapper.as_mut().unwrap(), frame_allocator.as_mut().unwrap()).unwrap();
    //     with_scheduler(|s| s.add_new_thread(thread));
    // }
}
