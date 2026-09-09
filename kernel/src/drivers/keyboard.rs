// ps 2 клавиатура опросом без irq читаем 0x60 когда в 0x64 есть данные
use spin::Mutex;
use crate::port::inb;

pub const KEY_ENTER: u8 = b'\n';
pub const KEY_BACKSPACE: u8 = 0x08;
pub const KEY_ESC: u8 = 0x1b;
pub const KEY_TAB: u8 = 0x09;
pub const KEY_UP: u8 = 0x11;
pub const KEY_DOWN: u8 = 0x12;
pub const KEY_LEFT: u8 = 0x13;
pub const KEY_RIGHT: u8 = 0x14;

#[derive(Clone, Copy, PartialEq)]
enum Layout {
    En,
    Ru,
}

struct KbState {
    shift: bool,
    caps: bool,
    ctrl: bool,
    alt: bool,
    win: bool,
    extended: bool,
    layout: Layout,
}

static STATE: Mutex<KbState> = Mutex::new(KbState {
    shift: false,
    caps: false,
    ctrl: false,
    alt: false,
    win: false,
    extended: false,
    layout: Layout::En,
});

// на случай если гуе захочет нарисовать индикатор раскладки где-нибудь в углу
pub fn layout_name() -> &'static str {
    match STATE.lock().layout {
        Layout::En => "EN",
        Layout::Ru => "RU",
    }
}

static FUN_SYMBOLS: [u8; 10] = [
    0x01, // 1 -> ☺
    0x02, // 2 -> ☻
    0x03, // 3 -> ♥
    0x04, // 4 -> ♦
    0x05, // 5 -> ♣
    0x06, // 6 -> ♠
    0x07, // 7 -> •
    0x0d, // 8 -> ♪
    0x0e, // 9 -> ♫
    0x0f, // 0 -> ☼
];

pub fn has_key() -> bool {
    let status = inb(0x64);
    // есть данные бит 0 и это НЕ мышь бит 0x20 равен 0
    status & 1 != 0 && status & 0x20 == 0
}

// обработать один готовый скан код вернуть Some ascii если это символ
fn process_one() -> Option<u8> {
    let code = inb(0x60);

    // префикс расширенных клавиш (стрелки, правые ctrl/alt, win) — запоминаем и ждём следующий байт
    if code == 0xe0 {
        STATE.lock().extended = true;
        return None;
    }

    {
        let mut s = STATE.lock();
        if s.extended {
            s.extended = false;
            let released = code & 0x80 != 0;
            let make = code & 0x7f;
            match make {
                0x1d => { s.ctrl = !released; return None; } // правый ctrl
                0x38 => { s.alt = !released; return None; }  // правый alt (altgr)
                0x5b | 0x5c => { s.win = !released; return None; } // левый/правый win
                _ => {}
            }
            drop(s);
            if released {
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
        let mut s = STATE.lock();
        match make {
            0x2a | 0x36 => s.shift = false,
            0x1d => s.ctrl = false, // левый ctrl отпущен
            0x38 => s.alt = false,  // левый alt отпущен
            _ => {}
        }
        return None;
    }

    match code {
        0x0f => Some(KEY_TAB),
        0x2a | 0x36 => {
            STATE.lock().shift = true;
            None
        }
        0x1d => {
            STATE.lock().ctrl = true; // левый ctrl нажат
            None
        }
        0x38 => {
            STATE.lock().alt = true; // левый alt нажат
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
        0x39 => {
            let mut s = STATE.lock();
            if s.win {
                s.layout = match s.layout {
                    Layout::En => Layout::Ru,
                    Layout::Ru => Layout::En,
                };
                None
            } else {
                Some(b' ')
            }
        }
        0x02..=0x0b => {
            let s = STATE.lock();
            if s.ctrl && s.alt {
                Some(FUN_SYMBOLS[(code - 0x02) as usize])
            } else {
                let shift = s.shift;
                drop(s);
                scancode_to_ascii(code, shift, false, Layout::En)
            }
        }
        _ => {
            let s = STATE.lock();
            let upper = s.shift ^ s.caps;
            let shift = s.shift;
            let layout = s.layout;
            drop(s);
            scancode_to_ascii(code, shift, upper, layout)
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

// scan code set 1 ascii/cp866 (в зависимости от раскладки)
fn scancode_to_ascii(code: u8, shift: bool, upper: bool, layout: Layout) -> Option<u8> {
    if layout == Layout::Ru {
        if let Some(ch) = ru_scancode(code, shift, upper) {
            return Some(ch);
        }
    }

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

fn ru_scancode(code: u8, shift: bool, upper: bool) -> Option<u8> {
    let cyr = |lower: u8, upper_byte: u8| if upper { upper_byte } else { lower };
    match code {
        0x10 => Some(cyr(0xa9, 0x89)), // q -> й/Й
        0x11 => Some(cyr(0xe6, 0x96)), // w -> ц/Ц
        0x12 => Some(cyr(0xe3, 0x93)), // e -> у/У
        0x13 => Some(cyr(0xaa, 0x8a)), // r -> к/К
        0x14 => Some(cyr(0xa5, 0x85)), // t -> е/Е
        0x15 => Some(cyr(0xad, 0x8d)), // y -> н/Н
        0x16 => Some(cyr(0xa3, 0x83)), // u -> г/Г
        0x17 => Some(cyr(0xe8, 0x98)), // i -> ш/Ш
        0x18 => Some(cyr(0xe9, 0x99)), // o -> щ/Щ
        0x19 => Some(cyr(0xa7, 0x87)), // p -> з/З
        0x1e => Some(cyr(0xe4, 0x94)), // a -> ф/Ф
        0x1f => Some(cyr(0xeb, 0x9b)), // s -> ы/Ы
        0x20 => Some(cyr(0xa2, 0x82)), // d -> в/В
        0x21 => Some(cyr(0xa0, 0x80)), // f -> а/А
        0x22 => Some(cyr(0xaf, 0x8f)), // g -> п/П
        0x23 => Some(cyr(0xe0, 0x90)), // h -> р/Р
        0x24 => Some(cyr(0xae, 0x8e)), // j -> о/О
        0x25 => Some(cyr(0xab, 0x8b)), // k -> л/Л
        0x26 => Some(cyr(0xa4, 0x84)), // l -> д/Д
        0x2c => Some(cyr(0xef, 0x9f)), // z -> я/Я
        0x2d => Some(cyr(0xe7, 0x97)), // x -> ч/Ч
        0x2e => Some(cyr(0xe1, 0x91)), // c -> с/С
        0x2f => Some(cyr(0xac, 0x8c)), // v -> м/М
        0x30 => Some(cyr(0xa8, 0x88)), // b -> и/И
        0x31 => Some(cyr(0xe2, 0x92)), // n -> т/Т
        0x32 => Some(cyr(0xec, 0x9c)), // m -> ь/Ь
        0x1a => Some(cyr(0xe5, 0x95)), // [ -> х/Х
        0x1b => Some(cyr(0xea, 0x9a)), // ] -> ъ/Ъ
        0x27 => Some(cyr(0xa6, 0x86)), // ; -> ж/Ж
        0x28 => Some(cyr(0xed, 0x9d)), // ' -> э/Э
        0x29 => Some(cyr(0xf1, 0xf0)), // ` -> ё/Ё
        0x33 => Some(cyr(0xa1, 0x81)), // , -> б/Б
        0x34 => Some(cyr(0xee, 0x9e)), // . -> ю/Ю
        0x35 => Some(if shift { b',' } else { b'.' }), // / -> . или ,
        _ => None,
    }
}
