pub fn read_rdtsc() -> u64 {
    let hi: u32;
    let lo: u32;
    unsafe {
        asm!("rdtsc", out("edx") hi, out("eax") lo);
    }
    ((hi as u64) << 32) | lo as u64
}
