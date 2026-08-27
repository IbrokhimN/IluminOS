// framebuffer вывод заменяет vga text mode рисуем глифы шрифтом 8x8 в пиксели
use core::fmt;
use spin::Mutex;
use font8x8::legacy::BASIC_LEGACY;

// готовые цвета в формате 0x00RRGGBB
pub const WHITE: u32 = 0xFFFFFF;
pub const GRAY: u32 = 0xAAAAAA;
pub const GREEN: u32 = 0x33FF66;
pub const RED: u32 = 0xFF4444;
pub const CYAN: u32 = 0x33DDFF;
pub const YELLOW: u32 = 0xFFDD33;
pub const BLACK: u32 = 0x000000;

struct Fb {
    addr: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
    col: usize,
    row: usize,
    fg: u32,
    bg: u32,
    cursor_on: bool,
}

static FB: Mutex<Option<Fb>> = Mutex::new(None);

const GLYPH_W: usize = 8;
const GLYPH_H: usize = 8;

unsafe impl Send for Fb {}

pub fn init(addr: *mut u8, width: usize, height: usize, pitch: usize) {
    let mut guard = FB.lock();
    *guard = Some(Fb {
        addr,
        width,
        height,
        pitch,
        col: 0,
        row: 0,
        fg: WHITE,
        bg: BLACK,
        cursor_on: false,
    });
    drop(guard);
    clear();
}

pub fn set_color(color: u32) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        fb.fg = color;
    }
}

// поставить позицию текстового курсора для редактора

// нарисовать курсор подчёркивание под клеткой для редактора статичный
pub fn draw_edit_cursor(col: usize, row: usize) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        let px = col * GLYPH_W;
        let py = row * GLYPH_H;
        let fg = fb.fg;
        // нижние 2 строки пикселей клетки
        for y in (GLYPH_H - 2)..GLYPH_H {
            for x in 0..GLYPH_W {
                fb.put_pixel(px + x, py + y, fg);
            }
        }
    }
}

#[allow(dead_code)]
pub fn set_cursor_pos(col: usize, row: usize) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        // если курсор был нарисован в старом месте стереть
        if fb.cursor_on {
            let (ocx, ocy) = (fb.col, fb.row);
            let bg = fb.bg;
            fb.fill_cell(ocx, ocy, bg);
            fb.cursor_on = false;
        }
        fb.col = col;
        fb.row = row;
    }
}

impl Fb {
    #[inline]
    fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = y * self.pitch + x * 4;
        unsafe {
            self.addr.add(offset).cast::<u32>().write_volatile(color);
        }
    }

    fn cols(&self) -> usize {
        self.width / GLYPH_W
    }

    fn rows(&self) -> usize {
        self.height / GLYPH_H
    }

    fn draw_glyph(&mut self, ch: u8, cx: usize, cy: usize) {
        let glyph = BASIC_LEGACY[ch as usize];
        let px = cx * GLYPH_W;
        let py = cy * GLYPH_H;
        for (row, bits) in glyph.iter().enumerate() {
            for bit in 0..8 {
                let color = if bits & (1 << bit) != 0 { self.fg } else { self.bg };
                self.put_pixel(px + bit, py + row, color);
            }
        }
    }

    fn fill_cell(&mut self, cx: usize, cy: usize, color: u32) {
        let px = cx * GLYPH_W;
        let py = cy * GLYPH_H;
        for y in 0..GLYPH_H {
            for x in 0..GLYPH_W {
                self.put_pixel(px + x, py + y, color);
            }
        }
    }

    fn newline(&mut self) {
        self.col = 0;
        if self.row + 1 < self.rows() {
            self.row += 1;
        } else {
            self.scroll();
        }
    }

    fn scroll(&mut self) {
        let line_bytes = self.pitch * GLYPH_H;
        let total = self.pitch * self.height;
        unsafe {
            let src = self.addr.add(line_bytes);
            let dst = self.addr;
            core::ptr::copy(src, dst, total - line_bytes);
            let last = self.addr.add(total - line_bytes);
            core::ptr::write_bytes(last, 0, line_bytes);
        }
    }

    fn write_char(&mut self, c: u8) {
        match c {
            b'\n' => self.newline(),
            0x08 => {
                if self.col > 0 {
                    self.col -= 1;
                } else if self.row > 0 {
                    self.row -= 1;
                    self.col = self.cols() - 1;
                }
                let (cx, cy) = (self.col, self.row);
                self.draw_glyph(b' ', cx, cy);
            }
            0x20..=0x7e => {
                if self.col >= self.cols() {
                    self.newline();
                }
                let (cx, cy) = (self.col, self.row);
                self.draw_glyph(c, cx, cy);
                self.col += 1;
            }
            _ => {}
        }
    }
}

pub fn clear() {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        let total = fb.pitch * fb.height;
        unsafe {
            core::ptr::write_bytes(fb.addr, 0, total);
        }
        fb.col = 0;
        fb.row = 0;
        fb.cursor_on = false;
    }
}

pub fn toggle_cursor() {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        let (cx, cy) = (fb.col, fb.row);
        let (fg, bg, on) = (fb.fg, fb.bg, fb.cursor_on);
        if on {
            fb.fill_cell(cx, cy, bg);
            fb.cursor_on = false;
        } else {
            fb.fill_cell(cx, cy, fg);
            fb.cursor_on = true;
        }
    }
}

pub fn hide_cursor() {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        if fb.cursor_on {
            let (cx, cy) = (fb.col, fb.row);
            let bg = fb.bg;
            fb.fill_cell(cx, cy, bg);
            fb.cursor_on = false;
        }
    }
}


// размеры экрана в пикселях для GUI
pub fn dimensions() -> (usize, usize) {
    let guard = FB.lock();
    if let Some(fb) = guard.as_ref() {
        (fb.width, fb.height)
    } else {
        (0, 0)
    }
}

// поставить один пиксель напрямую для GUI
pub fn pixel(x: usize, y: usize, color: u32) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        fb.put_pixel(x, y, color);
    }
}

// залитый прямоугольник
pub fn fill_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        for yy in y..y + h {
            for xx in x..x + w {
                fb.put_pixel(xx, yy, color);
            }
        }
    }
}

// рамка прямоугольника толщиной 1 пиксель
pub fn draw_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        for xx in x..x + w {
            fb.put_pixel(xx, y, color);
            fb.put_pixel(xx, y + h - 1, color);
        }
        for yy in y..y + h {
            fb.put_pixel(x, yy, color);
            fb.put_pixel(x + w - 1, yy, color);
        }
    }
}

// нарисовать один символ в пиксельной позиции заданным цветом
pub fn draw_char_at(ch: u8, px: usize, py: usize, fg: u32) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        let glyph = BASIC_LEGACY[(ch & 0x7f) as usize];
        for (row, bits) in glyph.iter().enumerate() {
            for bit in 0..8 {
                if bits & (1 << bit) != 0 {
                    fb.put_pixel(px + bit, py + row, fg);
                }
            }
        }
    }
}

// нарисовать строку в пиксельной позиции
pub fn draw_text_at(text: &str, px: usize, py: usize, fg: u32) {
    let mut x = px;
    for b in text.bytes() {
        draw_char_at(b, x, py, fg);
        x += 8;
    }
}


// нарисовать курсор-стрелку мыши в позиции. простая стрелка 12x12
pub fn draw_cursor_arrow(px: usize, py: usize) {
    // битовая маска стрелки 1 чёрный контур 2 белая заливка 0 прозрачно
    let arrow: [&[u8]; 12] = [
        b"1           ",
        b"11          ",
        b"121         ",
        b"1221        ",
        b"12221       ",
        b"122221      ",
        b"1222221     ",
        b"12222221    ",
        b"122221111   ",
        b"1221221     ",
        b"121 1221    ",
        b"11   1221   ",
    ];
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        for (row, line) in arrow.iter().enumerate() {
            for (col, &ch) in line.iter().enumerate() {
                let color = match ch {
                    b'1' => Some(0x000000u32), // чёрный контур
                    b'2' => Some(0xFFFFFFu32), // белая заливка
                    _ => None,
                };
                if let Some(c) = color {
                    fb.put_pixel(px + col, py + row, c);
                }
            }
        }
    }
}


// буфер под курсором 12x12 пикселей для сохранения фона
static mut CURSOR_BG: [u32; 144] = [0; 144];
static mut CURSOR_SAVED: bool = false;
static mut CURSOR_X: usize = 0;
static mut CURSOR_Y: usize = 0;

// сохранить фон под будущим курсором в позиции
pub fn save_under_cursor(px: usize, py: usize) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        unsafe {
            for row in 0..12 {
                for col in 0..12 {
                    let x = px + col;
                    let y = py + row;
                    if x < fb.width && y < fb.height {
                        let offset = y * fb.pitch + x * 4;
                        let p = fb.addr.add(offset).cast::<u32>().read_volatile();
                        CURSOR_BG[row * 12 + col] = p;
                    }
                }
            }
            CURSOR_X = px;
            CURSOR_Y = py;
            CURSOR_SAVED = true;
        }
    }
}

// восстановить фон где был курсор
pub fn restore_under_cursor() {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        unsafe {
            if !CURSOR_SAVED {
                return;
            }
            for row in 0..12 {
                for col in 0..12 {
                    let x = CURSOR_X + col;
                    let y = CURSOR_Y + row;
                    if x < fb.width && y < fb.height {
                        let offset = y * fb.pitch + x * 4;
                        fb.addr.add(offset).cast::<u32>().write_volatile(CURSOR_BG[row * 12 + col]);
                    }
                }
            }
        }
    }
}


// нарисовать символ с масштабом каждый пиксель глифа рисуется квадратом scale x scale
pub fn draw_char_scaled(ch: u8, px: usize, py: usize, fg: u32, scale: usize) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        let glyph = BASIC_LEGACY[(ch & 0x7f) as usize];
        for (row, bits) in glyph.iter().enumerate() {
            for bit in 0..8 {
                if bits & (1 << bit) != 0 {
                    // рисуем квадрат scale x scale вместо одного пикселя
                    for sy in 0..scale {
                        for sx in 0..scale {
                            fb.put_pixel(px + bit * scale + sx, py + row * scale + sy, fg);
                        }
                    }
                }
            }
        }
    }
}

// нарисовать строку с масштабом вернуть ширину в пикселях
pub fn draw_text_scaled(text: &str, px: usize, py: usize, fg: u32, scale: usize) -> usize {
    let mut x = px;
    for b in text.bytes() {
        draw_char_scaled(b, x, py, fg, scale);
        x += 8 * scale;
    }
    x - px
}


pub struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut guard = FB.lock();
        if let Some(fb) = guard.as_mut() {
            for b in s.bytes() {
                fb.write_char(b);
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    hide_cursor();
    let mut w = Writer;
    let _ = w.write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::framebuffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print_color {
    ($color:expr, $($arg:tt)*) => {{
        $crate::framebuffer::set_color($color);
        $crate::print!($($arg)*);
        $crate::framebuffer::set_color($crate::framebuffer::WHITE);
    }};
}
