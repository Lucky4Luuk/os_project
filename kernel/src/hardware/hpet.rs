use core::sync::atomic::{AtomicU64, Ordering};

use crate::interrupts::InterruptIndex;
use crate::memory::{memory_read_64, memory_write_64, memory_read_32, memory_write_32};

pub static HPET_BASE_ADDR: AtomicU64 = AtomicU64::new(0xFED0_0000);

const HPET_REG_GEN_CAP_ID: u64 = 0x000;//-0x007
const HPET_REG_GEN_CONFIG: u64 = 0x010;//-0x017
const HPET_REG_GEN_INT_ST: u64 = 0x020;//-0x027
const HPET_REG_MAIN_CNT_V: u64 = 0x0F0;//-0x0F7
const HPET_REG_TMR_CONCAP: u64 = 0x100;//-0x107 //Offset by 0x20 * N, where N is the timer channel
const HPET_REG_TMR_COMP_V: u64 = 0x108;//-0x10F //Offset by 0x20 * N
const HPET_REG_TMR_FSBRTE: u64 = 0x110;//-0x117 //Offset by 0x20 * N

lazy_static! {
    pub static ref HPET_INFO: spin::Mutex<HPET_Information> = spin::Mutex::new(HPET_Information::default());
}

/// Contains information related to the HPET, including its capabilities.
#[derive(Default)]
pub struct HPET_Information {
    pub supports_64_bit: bool,
    pub counters: u8,
    pub supports_legacy_mapping: bool,

    pub vendor_id: u16,

    pub freq: u64,
    pub period: u32, //Stored only because I don't want to recalculate it everytime I need it
}

fn hpet_read_period() -> u32 {
    unsafe {
        (hpet_read_64(HPET_REG_GEN_CAP_ID) >> 32) as u32
    }
}

fn hpet_read_irq(channel: u8) -> u32 {
    unsafe {
        (hpet_read_64(HPET_REG_TMR_CONCAP + 0x20 * (channel as u64)) >> 32) as u32
    }
}

fn hpet_write_32(addr: u64, val: u32) {
    unsafe {
        memory_write_32(HPET_BASE_ADDR.load(Ordering::Relaxed) + addr, val)
    }
}

fn hpet_read_32(addr: u64) -> u32 {
    unsafe {
        memory_read_32(HPET_BASE_ADDR.load(Ordering::Relaxed) + addr)
    }
}

fn hpet_write_64(addr: u64, val: u64) {
    unsafe {
        memory_write_64(HPET_BASE_ADDR.load(Ordering::Relaxed) + addr, val)
    }
}

fn hpet_read_64(addr: u64) -> u64 {
    unsafe {
        memory_read_64(HPET_BASE_ADDR.load(Ordering::Relaxed) + addr)
    }
}

/// This function guarantees a timer that will trigger in `timer` amount or longer.
fn hpet_set_oneshot_timer(channel: u8, mut timer: u64) {
    let period = HPET_INFO.lock().period;

}

/// This function guarantees a timer that will trigger every `timer` amount or longer.
fn hpet_set_period_timer(channel: u8, mut timer: u64, idt_index: InterruptIndex) {
    // let period = HPET_INFO.lock().period as u64;
    // if timer < period {
    //     timer = period;
    // }
    let channel_offset = 0x20 * channel as u64;
    trace!("TIMER: {}", timer);
    if (hpet_read_64(HPET_REG_TMR_CONCAP + channel_offset) & (1<<4)) == 0 {
        panic!("Cannot enable periodic mode on a timer that does not support periodic mode!");
    }
    let ioapic_irq_allowed = hpet_read_irq(channel);
    trace!("HPET IRQ: 0b{:032b}", ioapic_irq_allowed);
    let mut ioapic_irq: u32 = 0;
    'search: for i in 0..32 {
        if ioapic_irq_allowed & (0x1 << i) != 0 {
            trace!("[HPET] Available IRQ: {}", i);
            ioapic_irq = i;
            break 'search;
        }
    }

    //Only needed for QEMU
    ioapic_irq += 9;

    //TODO: 64 bit timer stuff probably only works when the HPET supports 64 bit mode lol
    hpet_write_64(HPET_REG_TMR_CONCAP + channel_offset, ((ioapic_irq as u64) << 9) | (1<<2) | (1<<3) | (1<<6));
    hpet_write_64(HPET_REG_TMR_COMP_V + channel_offset, hpet_read_64(HPET_REG_MAIN_CNT_V) + timer);
    hpet_write_64(HPET_REG_TMR_COMP_V + channel_offset, timer);

    use crate::interrupts::ioapic;
    unsafe { ioapic::ioapic_set_irq(ioapic_irq, 0, idt_index.as_u8()); }
}

/// Collects a bunch of information of HPET and enables a periodic timer on the first
/// channel, meant for use in the task scheduler.
pub fn initialize_hpet() {
    let period = hpet_read_period();
    let freq: u64 = 1_000_000_000_000_000 / (period as u64);
    trace!("HPET freq: {}", freq);

    //Check general capabilities of HPET
    let cap_field = unsafe { memory_read_32(HPET_BASE_ADDR.load(Ordering::Relaxed) + HPET_REG_GEN_CAP_ID) };
    trace!("HPET cap field: 0b{:032b}", cap_field);
    let bit64_capable = ((0x1<<13) & cap_field) != 0; //64 bit main counter support
    trace!("HPET 64 bit capable: {}", bit64_capable);
    let vendor_id = (cap_field >> 16) as u16; //Should report 0x8086
    trace!("HPET vendor ID: {:#04X}", vendor_id);
    let counters = ((cap_field >> 8) & 0b11111) as u8 + 1;
    trace!("HPET counters: {}", counters);
    let legacy_mapping = ((0x1<<15) & cap_field) != 0; //Legacy mapping available
    trace!("HPET legacy mapping: {}", legacy_mapping);

    {
        let mut hpet_info = HPET_INFO.lock();
        *hpet_info = HPET_Information {
            supports_64_bit: bit64_capable,
            counters: counters,
            supports_legacy_mapping: legacy_mapping,

            vendor_id: vendor_id,

            freq: freq,
            period: period,
        };
    }

    unsafe {
        debug!("0b{:064b}", hpet_read_64(HPET_REG_GEN_CONFIG));
        hpet_write_64(HPET_REG_GEN_CONFIG, hpet_read_64(HPET_REG_GEN_CONFIG) & !(0b11 as u64));
        debug!("0b{:064b}", hpet_read_64(HPET_REG_GEN_CONFIG));
    }

    //Enable a periodic timer on channel 1
    //No need to check if its available, because
    //every system where HPET is supported has a minimum of 3 channels available
    let irq_freq = 256 as u64; //irq_freq of 2 means 2hz aka twice a second
    hpet_set_period_timer(0, freq / irq_freq, InterruptIndex::HPET_Timer);

    //Enable the main counter
    unsafe {
        hpet_write_64(HPET_REG_GEN_CONFIG, hpet_read_64(HPET_REG_GEN_CONFIG) | (0b1 as u64));
        debug!("0b{:064b}", hpet_read_64(HPET_REG_GEN_CONFIG));
    }
}
