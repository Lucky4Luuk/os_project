use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::structures::idt::PageFaultErrorCode;

use pic8259_simple::ChainedPics;
use spin;

use acpi::platform::InterruptSourceOverride;
use alloc::vec::Vec;

use crate::{print, println, gdt, hlt_loop};

pub mod apic;
pub mod ioapic;

///////////////////////////////////////////////////////////////////////////////////////////////////
// PIC
///////////////////////////////////////////////////////////////////////////////////////////////////
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub const PIC_OFFSET: u8 = 32;

// pub static PICS: spin::Mutex<ChainedPics> =
//     spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

//NOTE: Maybe this belongs in apic.rs?
pub fn initialize_apic(id: u8, isos: Vec<InterruptSourceOverride>) {
    unsafe {
        apic::disable_pic();

        crate::hardware::rtc::enable_rtc(6); //Default value of 1024 hz

        apic::enable_apic(id);

        let mut timer_irq    = 0;
        let mut keyboard_irq = 1;
        let mut spurious_irq = 7;
        let mut rtc_irq      = 8;

        for iso in isos.iter() {
            match iso.isa_source {
                0 => timer_irq = iso.global_system_interrupt,
                1 => keyboard_irq = iso.global_system_interrupt,
                7 => spurious_irq = iso.global_system_interrupt,
                8 => rtc_irq = iso.global_system_interrupt,
                _ => {}, //Do nothing
            }
        }

        // Default IRQs
        ioapic::ioapic_set_irq(timer_irq, id as u32, InterruptIndex::Timer.as_u8());
        ioapic::ioapic_set_irq(keyboard_irq, id as u32, InterruptIndex::Keyboard.as_u8());
        ioapic::ioapic_set_irq(spurious_irq, id as u32, InterruptIndex::Spurious.as_u8());
        ioapic::ioapic_set_irq(rtc_irq, id as u32, InterruptIndex::RTC.as_u8());

        //apic::apic_set_timer(id);
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_OFFSET,
    Keyboard = PIC_OFFSET + 1,

    HPET_Timer = PIC_OFFSET + 2,

    Spurious = PIC_OFFSET + 7,
    RTC = PIC_OFFSET + 8,
    ACPI = PIC_OFFSET + 9,

    PrimaryATA = PIC_OFFSET + 14,
    SecondaryATA = PIC_OFFSET + 15,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// IDT
///////////////////////////////////////////////////////////////////////////////////////////////////
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // Exceptions
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);

        // Legacy IRQ interrupts
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Spurious.as_usize()].set_handler_fn(spurious_interrupt_handler);
        idt[InterruptIndex::RTC.as_usize()].set_handler_fn(rtc_interrupt_handler);
        idt[InterruptIndex::ACPI.as_usize()].set_handler_fn(acpi_interrupt_handler);

        // Hardcoded interrupts
        idt[InterruptIndex::HPET_Timer.as_usize()].set_handler_fn(hpet_interrupt_handler);

        idt
    };
}

/// Initialization
pub fn init_idt() {
    IDT.load();
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Exception handlers
///////////////////////////////////////////////////////////////////////////////////////////////////
/// Breakpoint handler
extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
    // unsafe { apic::send_apic_eoi(0); }
}

/// Double fault handler
extern "x86-interrupt" fn double_fault_handler(stack_frame: &mut InterruptStackFrame, _error_code: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

/// Page fault handler
extern "x86-interrupt" fn page_fault_handler(stack_frame: &mut InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

/// Divide error
extern "x86-interrupt" fn divide_error_handler(_stack_frame: &mut InterruptStackFrame) {
    println!("Divide error!");
}

/// General Protection Fault
extern "x86-interrupt" fn general_protection_fault_handler(_stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("General Protection Fault!");
}

/// Stack segment fault
extern "x86-interrupt" fn stack_segment_fault_handler(_stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("Stack segment fault!");
}

/// Invalid TSS
extern "x86-interrupt" fn invalid_tss_handler(_stack_frame: &mut InterruptStackFrame, error_code: u64) {
    println!("Invalid TSS!");
}

/// Segment not present
extern "x86-interrupt" fn segment_not_present_handler(_stack_frame: &mut InterruptStackFrame, error_code: u64) {
    panic!("Segment not present!");
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Legacy IRQ handlers
///////////////////////////////////////////////////////////////////////////////////////////////////
/// Timer interrupt handler
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    // print!(".");
    unsafe { apic::apic_send_eoi(0); }
}

/// Keyboard interrupt handler
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    debug!("Keyboard interrupt!");

    //Read the dataport so the buffer won't fill
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    unsafe { apic::apic_send_eoi(0); }
}

extern "x86-interrupt" fn acpi_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    println!("ACPI INTERRUPT!");

    unsafe { apic::apic_send_eoi(0); }
}

extern "x86-interrupt" fn hpet_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    // println!("HPET INTERRUPT!");
    print!(";");

    unsafe { apic::apic_send_eoi(0); }
}

extern "x86-interrupt" fn rtc_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    use core::sync::atomic::Ordering;
    crate::hardware::rtc::TICK_COUNT.fetch_add(1, Ordering::SeqCst);
    // if crate::hardware::rtc::TICK_COUNT.load(Ordering::SeqCst) > 16384 {
    //     debug!("hi 16384");
    // }
    unsafe {
        apic::apic_send_eoi(0);

        use cpuio::{inb, outb};
        outb(0x0C, 0x70);
        inb(0x71);
    }
}

extern "x86-interrupt" fn spurious_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    //TODO: Check ISR to make sure it's not a real interrupt
    unsafe { apic::apic_send_eoi(0); }
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// Test cases
///////////////////////////////////////////////////////////////////////////////////////////////////
#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}

//TODO: Implement test case for double fault exceptions. See https://os.phil-opp.com/double-fault-exceptions/#a-stack-overflow-test
