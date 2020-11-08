use alloc::vec::Vec;
use spin::Mutex;

use acpi::platform::IoApic;

use crate::memory::{memory_read_32, memory_write_32};

lazy_static! {
    pub static ref IOAPICS: Mutex<Vec<IoApic>> = Mutex::new(Vec::new());
}

pub fn get_io_apic_addr(id: u32, index: u32) -> u64 {
    IOAPICS.lock().iter().nth(id as usize).expect("Failed to get IOAPIC!").address as u64
}

pub unsafe fn ioapic_read(id: u32, index: u32) -> u32 {
    let addr = get_io_apic_addr(id, index);
    // Write the index to the index register
    memory_write_32(addr, index);
    // Read the value from the data register
    memory_read_32(addr + 0x10)
}

pub unsafe fn ioapic_write(id: u32, index: u32, value: u32) {
    let addr = get_io_apic_addr(id, index);
    // Write the index to the index register
    memory_write_32(addr, index);
    // Write the value to the data register
    memory_write_32(addr + 0x10, value);
}

pub fn get_io_apic_index(irq: u32) -> u32 {
    let mut id = 0;
    for ioapic in IOAPICS.lock().iter() {
        if irq as u32 >= ioapic.global_system_interrupt_base {
            //Found the right IOAPIC (I think)
            return id;
        }
        id += 1;
    }
    panic!("Can't find a matching IOAPIC!");
}

/// Set an IRQ on the IOAPIC
pub unsafe fn ioapic_set_irq(irq: u32, apic_id: u32, vector: u8) {
    let ioapic_id = get_io_apic_index(irq);

    let low_index: u32 = 0x10 + (irq as u32)*2;
    let high_index: u32 = 0x10 + (irq as u32)*2 + 1;

    let mut high = ioapic_read(ioapic_id, high_index);
    // Set APIC ID
    high &= !0xff000_000;
    high |= apic_id << 24;
    ioapic_write(ioapic_id, high_index, high);

    let mut low = ioapic_read(ioapic_id, low_index);

    // Unmask the IRQ
    low &= !(1<<16);

    // Set to physical delivery mode
    low &= !(1<<11);

    // Set to fixed delivery mode
    low &= !0x700;

    // Set delivery vector
    low &= !0xff;
    low |= vector as u32;

    ioapic_write(ioapic_id, low_index, low);
}
