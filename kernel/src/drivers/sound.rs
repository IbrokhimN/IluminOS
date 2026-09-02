use crate::port::{inb, outb};

// базовая частота PIT (тиков в секунду)
const PIT_FREQ: u32 = 1_193_182;

// включить динамик на заданной частоте (Гц)
pub fn play_freq(freq: u32) {
    if freq == 0 {
        stop();
        return;
    }
    let divisor = PIT_FREQ / freq; // сколько тиков PIT на один период волны

    // настроить PIT канал 2 на square wave (0xB6)
    outb(0x43, 0xB6);
    // записать делитель двумя байтами младший потом старший
    outb(0x42, (divisor & 0xFF) as u8);
    outb(0x42, ((divisor >> 8) & 0xFF) as u8);

    // включить динамик биты 0 и 1 порта 0x61
    let tmp = inb(0x61);
    if tmp & 0x03 != 0x03 {
        outb(0x61, tmp | 0x03);
    }
}

// выключить динамик (сбросить биты 0 и 1 порта 0x61)
pub fn stop() {
    let tmp = inb(0x61) & 0xFC; // & 0xFC = обнулить два младших бита
    outb(0x61, tmp);
}

// пискнуть частотой freq примерно duration "единиц" (грубая задержка)
// Точного таймера у нас нет, поэтому длительность — это просто busy-loop
pub fn beep(freq: u32, duration: u32) {
    play_freq(freq);
    delay(duration);
    stop();
}

// грубая задержка через пустой цикл (нет настоящего таймера)
// Число подобрано эмпирически; на разном железе скорость будет отличаться
pub fn delay(units: u32) {
    let mut count = units.wrapping_mul(200_000);
    while count > 0 {
        core::hint::spin_loop();
        count -= 1;
    }
}

// Таблица частот нот (одна октава + запас). Частоты в Гц, округлённые
// Это ноты 4-й и 5-й октавы — удобный диапазон для пианино
pub const NOTE_C4: u32 = 262;  // до
pub const NOTE_CS4: u32 = 277; // до-диез
pub const NOTE_D4: u32 = 294;  // ре
pub const NOTE_DS4: u32 = 311; // ре-диез
pub const NOTE_E4: u32 = 330;  // ми
pub const NOTE_F4: u32 = 349;  // фа
pub const NOTE_FS4: u32 = 370; // фа-диез
pub const NOTE_G4: u32 = 392;  // соль
pub const NOTE_GS4: u32 = 415; // соль-диез
pub const NOTE_A4: u32 = 440;  // ля
pub const NOTE_AS4: u32 = 466; // ля-диез
pub const NOTE_B4: u32 = 494;  // си
pub const NOTE_C5: u32 = 523;  // до (следующая октава)
pub const NOTE_D5: u32 = 587;
pub const NOTE_E5: u32 = 659;
pub const NOTE_F5: u32 = 698;
pub const NOTE_G5: u32 = 784;
