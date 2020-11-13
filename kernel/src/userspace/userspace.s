//; use intel asm syntax
.intel_syntax noprefix

//; The C-compatible signature of this function is:
//; asm_jump_usermode()
asm_jump_usermode:
    mov ax,0x23
    mov ds,ax
    mov es,ax
    mov fs,ax
    mov gs,ax           //;we don't need to worry about SS. it's handled by iret

    mov rax, rsp
    push 0x23           //;user data segment with bottom 2 bits set for ring 3
    push rax            //;push our current esp for the iret stack frame
    pushfq
    push 0x1B           //;user code segment with bottom 2 bits set for ring 3
    push init_userspace //;Rust function
    iretq
//;end
