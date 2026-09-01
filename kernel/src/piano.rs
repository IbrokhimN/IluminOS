use crate::keyboard;
use crate::sound::{self, *};
use crate::framebuffer;
use crate::print_color;
use crate::framebuffer::{GREEN, CYAN, YELLOW, GRAY, WHITE, RED};

// перевести букву клавиши в частоту ноты (0 = не нота)
fn key_to_note(key: u8) -> u32 {
    match key {
        // нижний ряд — белые клавиши
        b'z' => NOTE_C4,
        b'x' => NOTE_D4,
        b'c' => NOTE_E4,
        b'v' => NOTE_F4,
        b'b' => NOTE_G4,
        b'n' => NOTE_A4,
        b'm' => NOTE_B4,
        b',' => NOTE_C5,
        // верхний ряд — чёрные клавиши (диезы)
        b's' => NOTE_CS4,
        b'd' => NOTE_DS4,
        b'g' => NOTE_FS4,
        b'h' => NOTE_GS4,
        b'j' => NOTE_AS4,
        // второй ряд октавой выше
        b'q' => NOTE_C5,
        b'w' => NOTE_D5,
        b'e' => NOTE_E5,
        b'r' => NOTE_F5,
        b't' => NOTE_G5,
        _ => 0,
    }
}

// имя ноты для показа на экране
fn note_name(key: u8) -> &'static str {
    match key {
        b'z' => "C4", b'x' => "D4", b'c' => "E4", b'v' => "F4",
        b'b' => "G4", b'n' => "A4", b'm' => "B4", b',' => "C5",
        b's' => "C#4", b'd' => "D#4", b'g' => "F#4", b'h' => "G#4", b'j' => "A#4",
        b'q' => "C5", b'w' => "D5", b'e' => "E5", b'r' => "F5", b't' => "G5",
        _ => "",
    }
}

// запустить пианино. Esc для выхода
pub fn run() {
    framebuffer::clear();
    draw_help();

    loop {
        // блокирующе ждём клавишу
        let key = keyboard::read_key();

        if key == 0x1b { // Esc — выход
            break;
        }

        let freq = key_to_note(key);
        if freq > 0 {
            // показать сыгранную ноту
            print_color!(GREEN, "  ~ {} ({} Hz)\n", note_name(key), freq);
            // пискнуть: частота + короткая длительность
            sound::beep(freq, 2);
        }
    }

    sound::stop(); // на всякий случай выключить динамик
    framebuffer::clear();
    crate::banner::show();
    print_color!(GREEN, "piano closed.\n");
}

// нарисовать раскладку клавиш
fn draw_help() {
    print_color!(CYAN, "  === IluminOS Piano ===\n\n");
    print_color!(WHITE, "  White keys (bottom row):\n");
    print_color!(GRAY,  "    z  x  c  v  b  n  m  ,\n");
    print_color!(GRAY,  "    C  D  E  F  G  A  B  C\n\n");
    print_color!(WHITE, "  Black keys (diesis):\n");
    print_color!(GRAY,  "    s=C#  d=D#  g=F#  h=G#  j=A#\n\n");
    print_color!(WHITE, "  Higher octave:  q w e r t\n\n");
    print_color!(YELLOW, "  Press keys to play. Esc to quit.\n\n");
    print_color!(RED, "  (note: PC Speaker needs QEMU audio backend to be heard)\n\n");
}
