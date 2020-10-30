use alloc::vec::Vec;

pub struct CPU {
    pub processors: Vec<Processor>,
}

impl CPU {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }
}

pub struct Processor {
    pub id: u8,
    pub apic_id: u8,

    pub pblk_address: u32,
    pub pblk_len: u8,

    pub is_ap: bool,
    pub state: acpi::ProcessorState,
}
