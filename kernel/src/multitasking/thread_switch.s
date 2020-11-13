//; in src/multitasking/thread_switch.s
//; use intel asm syntax
.intel_syntax noprefix

//; The C-compatible signature of this function is:
//; asm_thread_switch(stack_pointer: u64, thread_id: u64)
asm_thread_switch:
    pushfq                      //; push RFLAGS register to stack

    mov rax, rsp                //; save old stack pointer in `rax` register
    mov rsp, rdi                //; load new stack pointer (given as argument)

    mov rdi, rax                //; use saved stack pointer as argument
    call add_paused_thread      //; call function with argument

    popfq                       //; pop RFLAGS register to stack
    ret                         //; pop return address from stack and jump to it
