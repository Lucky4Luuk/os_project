use crate::memory::{memory_read_64, memory_write_64, memory_read_32, memory_write_32};

const HPET_BASE_ADDR: u64 = 0xFED00000;

const HPET_REG_GEN_CAP_ID: u64 = 0x000;//-0x007
const HPET_REG_GEN_CONFIG: u64 = 0x010;//-0x017
const HPET_REG_GEN_INT_ST: u64 = 0x020;//-0x027
const HPET_REG_MAIN_CNT_V: u64 = 0x0F0;//-0x0F7
const HPET_REG_TMR_CONCAP: u64 = 0x100;//-0x107 //Offset by 0x20 * N, where N is the timer channel
const HPET_REG_TMR_COMP_V: u64 = 0x108;//-0x10F //Offset by 0x20 * N
const HPET_REG_TMR_FSBRTE: u64 = 0x110;//-0x117 //Offset by 0x20 * N

fn hpet_read_period() -> u32 {
    unsafe {
        (memory_read_64(HPET_BASE_ADDR + HPET_REG_GEN_CAP_ID) >> 32) as u32
    }
}

pub fn initialize_hpet() {
    let freq: u64 = 1_000_000_000_000_000 / (hpet_read_period() as u64);
    trace!("HPET freq: {}", freq);
}
