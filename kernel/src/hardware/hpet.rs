use core::sync::atomic::{AtomicU64, Ordering};

use crate::memory::{memory_read_64, memory_write_64, memory_read_32, memory_write_32};

pub static HPET_BASE_ADDR: AtomicU64 = AtomicU64::new(0xFED0_0000);

const HPET_REG_GEN_CAP_ID: u64 = 0x000;//-0x007
const HPET_REG_GEN_CONFIG: u64 = 0x010;//-0x017
const HPET_REG_GEN_INT_ST: u64 = 0x020;//-0x027
const HPET_REG_MAIN_CNT_V: u64 = 0x0F0;//-0x0F7
const HPET_REG_TMR_CONCAP: u64 = 0x100;//-0x107 //Offset by 0x20 * N, where N is the timer channel
const HPET_REG_TMR_COMP_V: u64 = 0x108;//-0x10F //Offset by 0x20 * N
const HPET_REG_TMR_FSBRTE: u64 = 0x110;//-0x117 //Offset by 0x20 * N

fn hpet_read_period() -> u32 {
    unsafe {
        (memory_read_64(HPET_BASE_ADDR.load(Ordering::Relaxed) + HPET_REG_GEN_CAP_ID) >> 32) as u32
    }
}

pub fn initialize_hpet() {
    // trace!("HPET period: {}", hpet_read_period());
    // trace!("HPET data: 0b{:032b}", unsafe{memory_read_32(HPET_BASE_ADDR + HPET_REG_GEN_CAP_ID)});

    let freq: u64 = 1_000_000_000_000_000 / (hpet_read_period() as u64);
    trace!("HPET freq: {}", freq);

    //Check general capabilities of HPET
    let cap_field = unsafe { memory_read_32(HPET_BASE_ADDR.load(Ordering::Relaxed) + HPET_REG_GEN_CAP_ID) };
    trace!("HPET cap field: 0b{:032b}", cap_field);
    let bit64_capable = ((0x1<<13) & cap_field) != 0; //64 bit main counter support
    trace!("HPET 64 bit capable: {}", bit64_capable);
    let vendor_id = (cap_field >> 16) as u16; //Should report 0x8086
    trace!("HPET vendor ID: {:#04X}", vendor_id);
    let counters = ((cap_field >> 8) & 0b11111) as u8;
    trace!("HPET counters: {}", counters + 1);
    let legacy_mapping = ((0x1<<15) & cap_field) != 0; //Legacy mapping available
    trace!("HPET legacy mapping: {}", legacy_mapping);

    
}
