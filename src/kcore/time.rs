use crate::random::rdtsc; // reuse tsc reader from random.rs
use spin::Mutex;

// tsc value at boot time
static BOOT_TSC: Mutex<u64> = Mutex::new(0);

// estimated ticks per second
const TSC_HZ: u64 = 2_500_000_000; // ~2.5 ghz

// save boot tsc, call once at startup
pub fn init() {
    *BOOT_TSC.lock() = rdtsc();
}

// ticks passed since boot
pub fn ticks_since_boot() -> u64 {
    // wrapping sub avoids panic on overflow
    rdtsc().wrapping_sub(*BOOT_TSC.lock())
}

// seconds since boot estimate
pub fn uptime_secs() -> u64 {
    ticks_since_boot() / TSC_HZ
}

// split seconds into hours minutes seconds
pub fn uptime_hms() -> (u64, u64, u64) {
    let total = uptime_secs();
    (
        total / 3600,
        (total % 3600) / 60,
        total % 60,
    )
}
