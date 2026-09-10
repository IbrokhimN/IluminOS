// rng based on cpu tsc, seeds xorshift64
use core::arch::asm;
use spin::Mutex;

static STATE: Mutex<u64> = Mutex::new(0);

// read cpu tick counter
pub fn rdtsc() -> u64 {
    let hi: u32;
    let lo: u32;
    unsafe {
        asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    ((hi as u64) << 32) | (lo as u64)
}

// build seed from several tsc reads, call once at startup
pub fn init() {
    let a = rdtsc();
    let b = rdtsc().rotate_left(17);
    let c = rdtsc().rotate_left(31);
    let seed = a ^ b ^ c ^ 0x9E3779B97F4A7C15;

    let mut s = STATE.lock();
    // xorshift gets stuck at zero so avoid zero seed
    *s = if seed == 0 { 0xDEADBEEFCAFEBABE } else { seed };
}

// next random u64 via xorshift64
pub fn next_u64() -> u64 {
    let mut s = STATE.lock();
    let mut x = *s;
    if x == 0 {
        x = rdtsc() | 1;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *s = x;
    x
}

// random number in 0 to max
pub fn next_range(max: u64) -> u64 {
    if max == 0 {
        return 0;
    }
    next_u64() % max
}

// seed for external crates like ahash in rhai
pub fn get_seed() -> u64 {
    next_u64() ^ rdtsc()
}
