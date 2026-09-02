// ps 2 клавиатура опросом без irq читаем 0x60 когда в 0x64 есть данные
use spin::Mutex;
use crate::port::inb;

pub const KEY_ENTER: u8 = b'\n';
pub const KEY_BACKSPACE: u8 = 0x08;
pub const KEY_ESC: u8 = 0x1b;
pub const KEY_TAB: u8 = 0x09;
// спец-коды вне печатного диапазона для стрелок (editor их игнорирует)
pub const KEY_UP: u8 = 0x11;
pub const KEY_DOWN: u8 = 0x12;
pub const KEY_LEFT: u8 = 0x13;
pub const KEY_RIGHT: u8 = 0x14;

struct KbState {
    shift: bool,
    caps: bool,
    extended: bool,
}

static STATE: Mutex<KbState> = Mutex::new(KbState {
    shift: false,
    caps: false,
    extended: false,
});

// есть ли готовый байт в буфере клавиатуры без ожидания
pub fn has_key() -> bool {
    let status = inb(0x64);
    // есть данные бит 0 и это НЕ мышь бит 0x20 равен 0
    status & 1 != 0 && status & 0x20 == 0
}

// обработать один готовый скан код вернуть Some ascii если это символ
fn process_one() -> Option<u8> {
    let code = inb(0x60);

    // префикс расширенных клавиш (стрелки и т.п.) — запоминаем и ждём следующий байт
    if code == 0xe0 {
        STATE.lock().extended = true;
        return None;
    }

    // если предыдущий байт был 0xe0 — это расширенная клавиша
    {
        let mut s = STATE.lock();
        if s.extended {
            s.extended = false;
            drop(s);
            // отпускание (bit7) расширенной клавиши игнорируем
            if code & 0x80 != 0 {
                return None;
            }
            return match code {
                0x48 => Some(KEY_UP),
                0x50 => Some(KEY_DOWN),
                0x4b => Some(KEY_LEFT),
                0x4d => Some(KEY_RIGHT),
                _ => None,
            };
        }
    }

    if code & 0x80 != 0 {
        let make = code & 0x7f;
        if make == 0x2a || make == 0x36 {
            STATE.lock().shift = false;
        }
        return None;
    }

    match code {
        0x0f => Some(KEY_TAB),
        0x2a | 0x36 => {
            STATE.lock().shift = true;
            None
        }
        0x3a => {
            let mut s = STATE.lock();
            s.caps = !s.caps;
            None
        }
        0x1c => Some(KEY_ENTER),
        0x0e => Some(KEY_BACKSPACE),
        0x01 => Some(KEY_ESC),
        _ => {
            let s = STATE.lock();
            let upper = s.shift ^ s.caps;
            let shift = s.shift;
            drop(s);
            scancode_to_ascii(code, shift, upper)
        }
    }
}

// неблокирующее чтение вернуть символ если он уже есть иначе None
pub fn try_read_key() -> Option<u8> {
    if !has_key() {
        return None;
    }
    process_one()
}

// блокирующее чтение одного символа
pub fn read_key() -> u8 {
    loop {
        if !has_key() {
            core::hint::spin_loop();
            continue;
        }
        if let Some(ch) = process_one() {
            return ch;
        }
    }
}

// scan code set 1 ascii
fn scancode_to_ascii(code: u8, shift: bool, upper: bool) -> Option<u8> {
    let ch = match code {
        0x02 => if shift { b'!' } else { b'1' },
        0x03 => if shift { b'@' } else { b'2' },
        0x04 => if shift { b'#' } else { b'3' },
        0x05 => if shift { b'$' } else { b'4' },
        0x06 => if shift { b'%' } else { b'5' },
        0x07 => if shift { b'^' } else { b'6' },
        0x08 => if shift { b'&' } else { b'7' },
        0x09 => if shift { b'*' } else { b'8' },
        0x0a => if shift { b'(' } else { b'9' },
        0x0b => if shift { b')' } else { b'0' },
        0x0c => if shift { b'_' } else { b'-' },
        0x0d => if shift { b'+' } else { b'=' },
        0x1a => if shift { b'{' } else { b'[' },
        0x1b => if shift { b'}' } else { b']' },
        0x27 => if shift { b':' } else { b';' },
        0x28 => if shift { b'"' } else { b'\'' },
        0x29 => if shift { b'~' } else { b'`' },
        0x2b => if shift { b'|' } else { b'\\' },
        0x33 => if shift { b'<' } else { b',' },
        0x34 => if shift { b'>' } else { b'.' },
        0x35 => if shift { b'?' } else { b'/' },
        0x39 => b' ',
        0x10 => letter(b'q', upper),
        0x11 => letter(b'w', upper),
        0x12 => letter(b'e', upper),
        0x13 => letter(b'r', upper),
        0x14 => letter(b't', upper),
        0x15 => letter(b'y', upper),
        0x16 => letter(b'u', upper),
        0x17 => letter(b'i', upper),
        0x18 => letter(b'o', upper),
        0x19 => letter(b'p', upper),
        0x1e => letter(b'a', upper),
        0x1f => letter(b's', upper),
        0x20 => letter(b'd', upper),
        0x21 => letter(b'f', upper),
        0x22 => letter(b'g', upper),
        0x23 => letter(b'h', upper),
        0x24 => letter(b'j', upper),
        0x25 => letter(b'k', upper),
        0x26 => letter(b'l', upper),
        0x2c => letter(b'z', upper),
        0x2d => letter(b'x', upper),
        0x2e => letter(b'c', upper),
        0x2f => letter(b'v', upper),
        0x30 => letter(b'b', upper),
        0x31 => letter(b'n', upper),
        0x32 => letter(b'm', upper),
        _ => return None,
    };
    Some(ch)
}

fn letter(base: u8, upper: bool) -> u8 {
    if upper {
        base - 32
    } else {
        base
    }
}
