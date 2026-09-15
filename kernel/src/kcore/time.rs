use crate::random::rdtsc; 
use core::sync::atomic::{AtomicU64, Ordering};
use crate::drivers::port::{outb, inb};
use time::Duration;

static TSC_HZ: AtomicU64 = AtomicU64::new(0);

static BOOT_TSC: AtomicU64 = AtomicU64::new(0);

const PIT_CHANNEL2: u16 = 0x42;
const PIT_COMMAND: u16 = 0x43;
const PIT_CONTROL: u16 = 0x61;
const PIT_FREQ: u64 = 1_193_182; // PIT

fn calibrate_via_pit() -> u64 {
    const SAMPLE_MS: u64 = 10;
    let pit_ticks: u16 = ((PIT_FREQ * SAMPLE_MS) / 1000) as u16;

    unsafe {
        let mut ctrl = inb(PIT_CONTROL);
        ctrl = (ctrl & 0xFC) | 0x01;
        outb(PIT_CONTROL, ctrl);

        outb(PIT_COMMAND, 0b10_11_000_0);
        outb(PIT_CHANNEL2, (pit_ticks & 0xFF) as u8);
        outb(PIT_CHANNEL2, (pit_ticks >> 8) as u8);

        let start = rdtsc();

        while inb(PIT_CONTROL) & 0x20 == 0 {
            core::hint::spin_loop();
        }

        let end = rdtsc();
        let elapsed_ticks = end.wrapping_sub(start);

        elapsed_ticks * 1000 / SAMPLE_MS
    }
}

fn calibrate_via_cpuid() -> Option<u64> {
    use core::arch::x86_64::__cpuid;

    let max_leaf = unsafe { __cpuid(0) }.eax;
    if max_leaf < 0x15 {
        return None;
    }

    let leaf15 = unsafe { __cpuid(0x15) };
    if leaf15.ebx == 0 || leaf15.eax == 0 {
        return None;
    }

    let crystal_hz = if leaf15.ecx != 0 {
        leaf15.ecx as u64
    } else if max_leaf >= 0x16 {
        let leaf16 = unsafe { __cpuid(0x16) };
        if leaf16.eax == 0 {
            return None;
        }
        return Some(leaf16.eax as u64 * 1_000_000);
    } else {
        return None;
    };

    Some(crystal_hz * (leaf15.ebx as u64) / (leaf15.eax as u64))
}

pub fn init() {
    let hz = calibrate_via_cpuid().unwrap_or_else(calibrate_via_pit);
    TSC_HZ.store(hz, Ordering::Relaxed);
    BOOT_TSC.store(rdtsc(), Ordering::Relaxed);
}

pub fn ticks_since_boot() -> u64 {
    rdtsc().wrapping_sub(BOOT_TSC.load(Ordering::Relaxed))
}

pub fn uptime() -> Duration {
    let hz = TSC_HZ.load(Ordering::Relaxed).max(1);
    let ticks = ticks_since_boot();

    let secs = ticks / hz;
    let remainder = ticks % hz;
    let nanos = (remainder as u128 * 1_000_000_000 / hz as u128) as i64;

    Duration::new(secs as i64, nanos as i32)
}

pub fn uptime_secs() -> u64 {
    uptime().whole_seconds() as u64
}

pub fn uptime_hms() -> (u64, u64, u64) {
    let total = uptime().whole_seconds();
    (
        (total / 3600) as u64,
        ((total % 3600) / 60) as u64,
        (total % 60) as u64,
    )
}
