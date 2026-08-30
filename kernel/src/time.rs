// время работы через счётчик тактов процессора (rdtsc).
// на голом железе без настроенного PIT/HPET точного таймера нет,
// поэтому частоту tsc берём грубой оценкой. uptime получается
// приблизительным, но монотонным — этого достаточно для uptime/date.
use crate::random::rdtsc;
use spin::Mutex;

static BOOT_TSC: Mutex<u64> = Mutex::new(0);
// оценка тиков в секунду (~2.5 ГГц). uptime масштабируется по ней.
const TSC_HZ: u64 = 2_500_000_000;

// запомнить точку старта. вызвать один раз при загрузке.
pub fn init() {
    *BOOT_TSC.lock() = rdtsc();
}

// сколько тактов прошло с загрузки
pub fn ticks_since_boot() -> u64 {
    rdtsc().wrapping_sub(*BOOT_TSC.lock())
}

// секунды аптайма (оценка)
pub fn uptime_secs() -> u64 {
    ticks_since_boot() / TSC_HZ
}

// разложить аптайм на часы/минуты/секунды
pub fn uptime_hms() -> (u64, u64, u64) {
    let total = uptime_secs();
    (total / 3600, (total % 3600) / 60, total % 60)
}
