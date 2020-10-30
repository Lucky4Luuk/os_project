//TODO: Move this to interrupts/apic.rs

use cpuio::outb;

use core::sync::atomic::{AtomicU64, Ordering};

use crate::memory::{memory_read_32, memory_write_32};

///////////////////////////////////////////////////////////////////////////////////////////////////
// APIC
///////////////////////////////////////////////////////////////////////////////////////////////////
const APIC_ADDRESS: u64 = 0xFEE00000; //TODO: Get this from ACPI table although it shouldn't change

pub fn get_apic_address(apic_id: u8) -> u64 {
    APIC_ADDRESS + 0x10 * (apic_id as u64)
}

pub unsafe fn disable_pic() {
    // Set ICW1
    outb(0x11, 0x20);
    outb(0x11, 0xa0);

    // Set ICW2 (IRQ base offsets)
    outb(0xe0, 0x21);
    outb(0xe8, 0xa1);

    // Set ICW3
    outb(4, 0x21);
    outb(2, 0xa1);

    // Set ICW4
    outb(1, 0x21);
    outb(1, 0xa1);

    // Set OCW1 (interrupt masks)
    outb(0xff, 0x21);
    outb(0xff, 0xa1);
}

pub unsafe fn enable_apic(apic_id: u8) {
    let apic_addr = get_apic_address(apic_id);
    let mut val = memory_read_32(apic_addr + 0xF0);
    val |= (1<<8);
    memory_write_32(apic_addr + 0xF0, val);
}

pub unsafe fn apic_send_eoi(apic_id: u8) {
    let apic_addr = get_apic_address(apic_id);
    memory_write_32(apic_addr + 0xB0, 0);
}

pub unsafe fn apic_set_timer(apic_id: u8) {
    let apic_addr = get_apic_address(apic_id);

    memory_write_32(apic_addr + 0x3E0, 0x3); //0x3 = 011 = 16, divider

    memory_write_32(apic_addr + 0x320, 0x20020); //Enable timer, set periodic mode, set vector to 0x20 = 32
    memory_write_32(apic_addr + 0x380, 0x0); //Reset timer to -1
    crate::hardware::rtc::sleep(0.01); //Sleep for 10ms
    memory_write_32(apic_addr + 0x320, 0x10020); //Stop the timer

    let ticks = memory_read_32(apic_addr + 0x390);

    trace!("apic timer ticks in 10ms");
    trace!("{}", ticks);

    //Dunno what to do with this, `ticks` is always 0
    // memory_write_32(apic_addr + 0x320, 32 | 0x20000);
    // memory_write_32(apic_addr + 0x3E0, 0x3);
    // memory_write_32(apic_addr + 0x380, ticks);
}

///////////////////////////////////////////////////////////////////////////////////////////////////
// IOAPIC
///////////////////////////////////////////////////////////////////////////////////////////////////
static IOAPIC_ADDR: AtomicU64 = AtomicU64::new(0);
pub fn update_ioapic_addr(addr: u64) {
    IOAPIC_ADDR.store(addr, Ordering::Relaxed);
}

pub unsafe fn ioapic_read(index: u32) -> u32 {
    let addr = IOAPIC_ADDR.load(Ordering::Relaxed); //Read from the atomic IOAPIC_ADDR
    // Write the index to the index register
    memory_write_32(addr, index);
    // Read the value from the data register
    memory_read_32(addr + 0x10)
}

pub unsafe fn ioapic_write(index: u32, value: u32) {
    let addr = IOAPIC_ADDR.load(Ordering::Relaxed); //Read from the atomic IOAPIC_ADDR
    // Write the index to the index register
    memory_write_32(addr, index);
    // Write the value to the data register
    memory_write_32(addr + 0x10, value);
}

pub unsafe fn ioapic_set_irq(irq: u8, apic_id: u32, vector: u8) {
    let low_index: u32 = 0x10 + (irq as u32)*2;
    let high_index: u32 = 0x10 + (irq as u32)*2 + 1;

    let mut high = ioapic_read(high_index);
    // Set APIC ID
    high &= !0xff000_000;
    high |= apic_id << 24;
    ioapic_write(high_index, high);

    let mut low = ioapic_read(low_index);

    // Unmask the IRQ
    low &= !(1<<16);

    // Set to physical delivery mode
    low &= !(1<<11);

    // Set to fixed delivery mode
    low &= !0x700;

    // Set delivery vector
    low &= !0xff;
    low |= vector as u32;

    ioapic_write(low_index, low);
}
