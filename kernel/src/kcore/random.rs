// генератор случайных чисел на основе счётчика тактов процессора (RDTSC).
// на голом железе нет ОС-источника случайности - берём её из аппаратного
// счётчика тактов, младшие биты которого зависят от точного тайминга.
// затем xorshift64 порождает поток чисел из seed.
use core::arch::asm;
use spin::Mutex;

static STATE: Mutex<u64> = Mutex::new(0);

// счётчик тактов процессора
pub fn rdtsc() -> u64 {
    let hi: u32;
    let lo: u32;
    unsafe {
        asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    ((hi as u64) << 32) | (lo as u64)
}

// собираем seed из нескольких замеров tsc.
// вызвать один раз при старте.
pub fn init() {
    // смешиваем несколько замеров счётчика тактов с золотым сечением
    let a = rdtsc();
    let b = rdtsc().rotate_left(17);
    let c = rdtsc().rotate_left(31);
    let seed = a ^ b ^ c ^ 0x9E3779B97F4A7C15;

    let mut s = STATE.lock();
    // xorshift застревает на нуле - не допускаем нулевой seed
    *s = if seed == 0 { 0xDEADBEEFCAFEBABE } else { seed };
}

// следующее случайное u64 (xorshift64)
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

// случайное число в [0, max)
pub fn next_range(max: u64) -> u64 {
    if max == 0 {
        return 0;
    }
    next_u64() % max
}

// seed для внешних библиотек (например ahash в rhai)
pub fn get_seed() -> u64 {
    next_u64() ^ rdtsc()
}
