use crate::port::{inb, outb};

// pit base frequency ticks per second
const PIT_FREQ: u32 = 1_193_182;

// turn on speaker at given frequency hz
pub fn play_freq(freq: u32) {
    if freq == 0 {
        stop();
        return;
    }
    let divisor = PIT_FREQ / freq; // pit ticks per wave period

    // configure pit channel 2 for square wave
    outb(0x43, 0xB6);
    // write divisor low byte then high byte
    outb(0x42, (divisor & 0xFF) as u8);
    outb(0x42, ((divisor >> 8) & 0xFF) as u8);

    // enable speaker bits 0 and 1 on port 0x61
    let tmp = inb(0x61);
    if tmp & 0x03 != 0x03 {
        outb(0x61, tmp | 0x03);
    }
}

// turn off speaker
pub fn stop() {
    let tmp = inb(0x61) & 0xFC; // clear bits 0 and 1
    outb(0x61, tmp);
}

// beep freq for a rough duration busy loop
pub fn beep(freq: u32, duration: u32) {
    play_freq(freq);
    delay(duration);
    stop();
}

// crude delay via spin loop no real timer available
pub fn delay(units: u32) {
    let mut count = units.wrapping_mul(200_000);
    while count > 0 {
        core::hint::spin_loop();
        count -= 1;
    }
}

// note frequency table octaves 4 and 5
pub const NOTE_C4: u32 = 262;
pub const NOTE_CS4: u32 = 277;
pub const NOTE_D4: u32 = 294;
pub const NOTE_DS4: u32 = 311;
pub const NOTE_E4: u32 = 330;
pub const NOTE_F4: u32 = 349;
pub const NOTE_FS4: u32 = 370;
pub const NOTE_G4: u32 = 392;
pub const NOTE_GS4: u32 = 415;
pub const NOTE_A4: u32 = 440;
pub const NOTE_AS4: u32 = 466;
pub const NOTE_B4: u32 = 494;
pub const NOTE_C5: u32 = 523; // next octave
pub const NOTE_D5: u32 = 587;
pub const NOTE_E5: u32 = 659;
pub const NOTE_F5: u32 = 698;
pub const NOTE_G5: u32 = 784;
