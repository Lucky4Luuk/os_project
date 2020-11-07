use alloc::vec::Vec;
use spin::Mutex;

use acpi::platform::IoApic;

use crate::memory::{memory_read_32, memory_write_32};

lazy_static! {
    pub static ref IOAPICS: Mutex<Vec<IoApic>> = Mutex::new(Vec::new());
}

pub fn get_io_apic_addr(index: u32) -> u64 {
    IOAPICS.lock().iter().next().expect("Failed to get first IOAPIC!").address as u64
}

pub unsafe fn ioapic_read(index: u32) -> u32 {
    let addr = get_io_apic_addr(index);
    // Write the index to the index register
    memory_write_32(addr, index);
    // Read the value from the data register
    memory_read_32(addr + 0x10)
}

pub unsafe fn ioapic_write(index: u32, value: u32) {
    let addr = get_io_apic_addr(index);
    // Write the index to the index register
    memory_write_32(addr, index);
    // Write the value to the data register
    memory_write_32(addr + 0x10, value);
}

/// Set an IRQ on the IOAPIC
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
