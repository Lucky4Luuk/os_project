use x86_64::VirtAddr;

use super::{
    thread::ThreadId,
    SwitchReason,
    with_scheduler,
};

global_asm!(include_str!("thread_switch.s"));

pub unsafe fn thread_switch_to(
    new_stack_pointer: VirtAddr,
    prev_thread_id: ThreadId,
    switch_reason: SwitchReason,
) {
    llvm_asm!(
        "call asm_thread_switch"
        :
        : "{rdi}"(new_stack_pointer), "{rsi}"(prev_thread_id), "{rdx}"(switch_reason as u64)
        : "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "r8", "r9", "r10",
        "r11", "r12", "r13", "r14", "r15", "rflags", "memory"
        : "intel", "volatile"
    );
}

#[no_mangle]
pub extern "C" fn add_paused_thread(
    paused_stack_pointer: VirtAddr,
    paused_thread_id: ThreadId,
    switch_reason: SwitchReason,
) {
    with_scheduler(|s| s.add_paused_thread(paused_stack_pointer, paused_thread_id, switch_reason));
}
