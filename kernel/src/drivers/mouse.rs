// драйвер PS/2 мыши. мышь висит на втором канале PS/2 контроллера
// портах 0x60 данные и 0x64 команды. после включения шлёт пакеты
// по 3 байта флаги смещение X смещение Y.
use crate::port::{inb, outb};
use spin::Mutex;

const DATA: u16 = 0x60;
const STATUS: u16 = 0x64;
const CMD: u16 = 0x64;

// состояние мыши позиция и кнопки
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub left: bool,
    pub right: bool,
}

static STATE: Mutex<MouseState> = Mutex::new(MouseState {
    x: 400,
    y: 300,
    left: false,
    right: false,
});

// ждём пока можно писать в контроллер бит 1 статуса должен быть 0
fn wait_write() {
    let mut t = 0;
    while inb(STATUS) & 2 != 0 {
        t += 1;
        if t > 100_000 {
            break;
        }
    }
}

// ждём пока можно читать бит 0 статуса должен быть 1
fn wait_read() {
    let mut t = 0;
    while inb(STATUS) & 1 == 0 {
        t += 1;
        if t > 100_000 {
            break;
        }
    }
}

// послать команду именно мыши префикс 0xD4
fn write_mouse(val: u8) {
    wait_write();
    outb(CMD, 0xD4);
    wait_write();
    outb(DATA, val);
}

// прочитать ответ обычно 0xFA подтверждение
fn read_ack() -> u8 {
    wait_read();
    inb(DATA)
}

// инициализация мыши
pub fn init() {
    // включить второй канал PS/2 команда 0xA8
    wait_write();
    outb(CMD, 0xA8);

    // включить генерацию событий в конфиге контроллера
    wait_write();
    outb(CMD, 0x20); // читать байт конфига
    wait_read();
    let mut config = inb(DATA);
    config |= 2; // бит 1 разрешить прерывание второго канала
    config &= !0x20; // сбросить бит отключения второго канала
    wait_write();
    outb(CMD, 0x60); // писать байт конфига
    wait_write();
    outb(DATA, config);

    // сказать мыши использовать настройки по умолчанию
    write_mouse(0xF6);
    read_ack();

    // включить передачу пакетов
    write_mouse(0xF4);
    read_ack();
}

// накопитель байтов пакета
static PACKET: Mutex<[u8; 3]> = Mutex::new([0; 3]);
static PACKET_IDX: Mutex<usize> = Mutex::new(0);

// опросить мышь без ожидания. если пришёл полный пакет обновить состояние.
// вызывать часто в цикле GUI.
pub fn poll() {
    // есть ли данные и это данные мыши бит 5 статуса
    let status = inb(STATUS);
    if status & 1 == 0 {
        return; // нет данных
    }
    if status & 0x20 == 0 {
        // это данные клавиатуры не мыши пропускаем
        return;
    }

    let byte = inb(DATA);
    let mut idx = PACKET_IDX.lock();
    let mut pkt = PACKET.lock();

    // первый байт пакета должен иметь бит 3 установленным иначе рассинхрон
    if *idx == 0 && byte & 0x08 == 0 {
        return; // ждём корректный первый байт
    }

    pkt[*idx] = byte;
    *idx += 1;

    if *idx >= 3 {
        *idx = 0;
        let flags = pkt[0];
        let dx = pkt[1];
        let dy = pkt[2];

        // смещения знаковые 9 бит через флаги но берём простой вариант i8
        let mdx = dx as i8 as i32;
        let mdy = dy as i8 as i32;

        let mut s = STATE.lock();
        s.x += mdx;
        s.y -= mdy; // экран Y растёт вниз мышь вверх инвертируем
        s.left = flags & 1 != 0;
        s.right = flags & 2 != 0;

        // ограничиваем в пределах экрана
        let (w, h) = crate::framebuffer::dimensions();
        if s.x < 0 {
            s.x = 0;
        }
        if s.y < 0 {
            s.y = 0;
        }
        if s.x > w as i32 - 1 {
            s.x = w as i32 - 1;
        }
        if s.y > h as i32 - 1 {
            s.y = h as i32 - 1;
        }
    }
}

// текущее состояние копия
pub fn get() -> (i32, i32, bool, bool) {
    let s = STATE.lock();
    (s.x, s.y, s.left, s.right)
}
