use alloc::boxed::Box;
use core::mem;
use core::raw::TraitObject;

use x86_64::VirtAddr;

use super::with_scheduler;

pub struct Stack {
    pointer: VirtAddr,
}

impl Stack {
    pub unsafe fn new(stack_pointer: VirtAddr) -> Self {
        Stack {
            pointer: stack_pointer,
        }
    }

    pub fn get_stack_pointer(self) -> VirtAddr {
        self.pointer
    }

    pub fn set_up_for_closure(&mut self, closure: Box<dyn FnOnce() -> !>) {
        let trait_object: TraitObject = unsafe { mem::transmute(closure) };
        unsafe { self.push(trait_object.data) };
        unsafe { self.push(trait_object.vtable) };

        self.set_up_for_entry_point(call_closure_entry);
    }

    pub fn set_up_for_entry_point(&mut self, entry_point: fn() -> !) {
        unsafe { self.push(entry_point) };
        let rflags: u64 = 0x200;
        unsafe { self.push(rflags) };
    }

    unsafe fn push<T>(&mut self, value: T) {
        self.pointer -= core::mem::size_of::<T>();
        let ptr: *mut T = self.pointer.as_mut_ptr();
        ptr.write(value);
    }
}

#[naked]
fn call_closure_entry() -> ! {
    unsafe {
        llvm_asm!("
        pop rsi
        pop rdi
        call call_closure
    " ::: "mem" : "intel", "volatile")
    };
    unreachable!();
}

// no_mangle required because of https://github.com/rust-lang/rust/issues/68136
#[no_mangle]
extern "C" fn call_closure(data: *mut (), vtable: *mut ()) -> ! {
    let trait_object = TraitObject { data, vtable };
    let f: Box<dyn FnOnce() -> !> = unsafe { mem::transmute(trait_object) };
    f()
}
