use cpuio::{inb, outb};

use core::sync::atomic::{AtomicU16, AtomicU64, Ordering};

pub static TICK_COUNT: AtomicU64 = AtomicU64::new(0);
static TICKS_PER_SECOND: AtomicU16 = AtomicU16::new(0);

pub unsafe fn enable_rtc(rate: u8) {
    if rate < 3 || rate > 15 { panic!("Incorrect rate!"); }
    x86_64::instructions::interrupts::disable();
    // Enable IRQ 8
    outb(0x8B, 0x70);
    let prev = inb(0x71);
    outb(0x8B, 0x70);
    outb(prev | 0x40, 0x71);
    x86_64::instructions::interrupts::enable();

    // Change interrupt rate
    x86_64::instructions::interrupts::disable();
    let rate_div = rate & 0x0F;
    outb(0x8A, 0x70);
    let prev = inb(0x71);
    outb(0x8A, 0x70);
    outb((prev & 0xF0) | rate, 0x71);
    x86_64::instructions::interrupts::enable();

    let freq = 32768 >> (rate-1); //ticks per second
    trace!("freq: {}", freq);
    TICKS_PER_SECOND.store(freq, Ordering::SeqCst);
}

pub fn sleep(seconds: f32) {
    let ticks = TICK_COUNT.load(Ordering::SeqCst) + (seconds * TICKS_PER_SECOND.load(Ordering::SeqCst) as f32) as u64;
    while TICK_COUNT.compare_and_swap(ticks, ticks+1, Ordering::Relaxed) != ticks {}
}
