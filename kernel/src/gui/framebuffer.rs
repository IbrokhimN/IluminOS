use core::fmt;
use spin::{Mutex, Once};

// написал цвета справа потому что мой нвим показывает цвета написанных юникодов
// и мне так удобнее
// готовые цвета в формате 0x00RRGGBB
pub const WHITE: u32    =    0xFFFFFF;  // #ffffff
pub const GRAY: u32     =    0xAAAAAA;  // #aaaaaa
pub const GREEN: u32    =    0x33FF66;  // #33ff66
pub const RED: u32      =    0xFF4444;  // #ff4444 
pub const CYAN: u32     =    0x33DDFF;  // #33ddff
pub const YELLOW: u32   =    0xFFDD33;  // #ffdd33
pub const BLACK: u32    =    0x000000;  // #000000
pub const BLUE: u32     =    0x5599FF;  // #5599ff
pub const MAGENTA: u32  =    0xCC77FF;  // #cc77ff


// EverForest Colors
pub const EVERFOREST_BACKGROUND: u32   =   0x272E33;  // #272E33
pub const EVERFOREST_FOREGROUND: u32   =   0xD3C6AA;  // #D4C6AA

pub const EVERFOREST_BLACK: u32        =   0x475258;  // #475258
pub const EVERFOREST_RED: u32          =   0xE67E80;  // #e67e80
pub const EVERFOREST_GREEN: u32        =   0xA7C080;  // #a7c080
pub const EVERFOREST_YELLOW: u32       =   0xDBBC7F;  // #dbbc7f
pub const EVERFOREST_BLUE: u32         =   0x7FBBB3;  // #7fbbb3
pub const EVERFOREST_MAGENTA: u32      =   0xD699B6;  // #d699b6
pub const EVERFOREST_CYAN: u32         =   0x83C092;  // #83c092
pub const EVERFOREST_WHITE: u32        =   0xD3C6AA;  // #d3c6aa


// файл шрифта зашивается прямо в бинарь на этапе компиляции
static FONT_BYTES: &[u8] = include_bytes!("./fonts/ruscii_8x8.psfu");

struct PsfFont {
    width: usize,
    height: usize,
    bytes_per_row: usize, // сколько байт занимает одна строка глифа
    glyph_size: usize,    // размер одного глифа в байтах
    glyphs_offset: usize, // с какого байта в файле начинаются сами глифы
    num_glyphs: usize,
}

impl PsfFont {
    fn parse(data: &[u8]) -> Self {
        if data.len() >= 4 && data[0] == 0x36 && data[1] == 0x04 {
            let mode = data[2];
            let charsize = data[3] as usize;
            let num_glyphs = if mode & 0x01 != 0 { 512 } else { 256 };
            return PsfFont {
                width: 8,
                height: charsize,
                bytes_per_row: 1,
                glyph_size: charsize,
                glyphs_offset: 4,
                num_glyphs,
            };
        }
        // PSF2: магия 0x72 0xb5 0x4a 0x86, дальше заголовок из u32 le полей
        if data.len() >= 32 && data[0] == 0x72 && data[1] == 0xb5 && data[2] == 0x4a && data[3] == 0x86 {
            let rd = |o: usize| -> usize {
                u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]) as usize
            };
            let headersize = rd(8);
            let length = rd(16);
            let charsize = rd(20);
            let height = rd(24);
            let width = rd(28);
            let bytes_per_row = (width + 7) / 8;
            return PsfFont {
                width,
                height,
                bytes_per_row,
                glyph_size: charsize,
                glyphs_offset: headersize,
                num_glyphs: length,
            };
        }
        // не смогли распознать файл - фолбек 8x16 пустышка чтобы не паниковать
        PsfFont { width: 8, height: 16, bytes_per_row: 1, glyph_size: 16, glyphs_offset: 0, num_glyphs: 0 }
    }

    #[inline]
    fn glyph(&self, ch: u8) -> &'static [u8] {
        // юникод-таблицу PSF (если она есть в файле) не разбираем, индексируем
        // глиф напрямую кодом символа, как было с BASIC_LEGACY
        let idx = if (ch as usize) < self.num_glyphs { ch as usize } else { 0 };
        let start = self.glyphs_offset + idx * self.glyph_size;
        let end = start + self.glyph_size;
        &FONT_BYTES[start..end]
    }

    #[inline]
    fn bit_set(&self, glyph: &[u8], x: usize, y: usize) -> bool {
        // в psf строка глифа хранится msb-первым
        let byte = glyph[y * self.bytes_per_row + x / 8];
        let bit = 7 - (x % 8);
        (byte & (1 << bit)) != 0
    }
}

static FONT: Once<PsfFont> = Once::new();

#[inline]
fn font() -> &'static PsfFont {
    FONT.call_once(|| PsfFont::parse(FONT_BYTES))
}


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

static THEME_FG: Mutex<u32> = Mutex::new(EVERFOREST_FOREGROUND);

// вернуть текущий дефолтный цвет текста темы
pub fn theme_fg() -> u32 {
    *THEME_FG.lock()
}

// поставить тему: dark = белый текст на чёрном lgiht = чёрный текст на белом
pub fn set_theme(fg: u32, bg: u32) {
    *THEME_FG.lock() = fg;
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        fb.fg = fg;
        fb.bg = bg;
    }
}

unsafe impl Send for Fb {}

pub fn init(addr: *mut u8, width: usize, height: usize, pitch: usize) {
    // прогреваем парсинг шрифта заранее чтобы первый print! не тормозил
    font();
    let mut guard = FB.lock();
    *guard = Some(Fb {
        addr,
        width,
        height,
        pitch,
        col: 0,
        row: 0,
        fg: EVERFOREST_FOREGROUND,
        bg: EVERFOREST_BACKGROUND,
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
        let f = font();
        let px = col * f.width;
        let py = row * f.height;
        let fg = fb.fg;
        // нижние 2 строки пикселей клетки
        for y in (f.height - 2)..f.height {
            for x in 0..f.width {
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
        self.width / font().width
    }

    fn rows(&self) -> usize {
        self.height / font().height
    }

    fn draw_glyph(&mut self, ch: u8, cx: usize, cy: usize) {
        let f = font();
        let glyph = f.glyph(ch);
        let px = cx * f.width;
        let py = cy * f.height;
        for row in 0..f.height {
            for col in 0..f.width {
                let color = if f.bit_set(glyph, col, row) { self.fg } else { self.bg };
                self.put_pixel(px + col, py + row, color);
            }
        }
    }

    fn fill_cell(&mut self, cx: usize, cy: usize, color: u32) {
        let f = font();
        let px = cx * f.width;
        let py = cy * f.height;
        for y in 0..f.height {
            for x in 0..f.width {
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
        let line_bytes = self.pitch * font().height;
        let total = self.pitch * self.height;
        unsafe {
            let src = self.addr.add(line_bytes);
            let dst = self.addr;
            // Перемещаем старые пиксели вверх
            core::ptr::copy(src, dst, total - line_bytes);

            // Заполняем новую нижнюю строку фоновым цветом self.bg
            let start_y = self.height - font().height;
            let bg = self.bg;

            for y in start_y..self.height {
                for x in 0..self.width {
                    let offset = y * self.pitch + x * 4;
                    self.addr.add(offset).cast::<u32>().write_volatile(bg);
                }
            }
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
        let bg = fb.bg;
        if bg == BLACK {
            let total = fb.pitch * fb.height;
            unsafe {
                core::ptr::write_bytes(fb.addr, 0, total);
            }
        } else {
            // не чёрный фон — заливаем попиксельно
            for y in 0..fb.height {
                for x in 0..fb.width {
                    fb.put_pixel(x, y, bg);
                }
            }
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
        let f = font();
        let glyph = f.glyph(ch);
        for row in 0..f.height {
            for col in 0..f.width {
                if f.bit_set(glyph, col, row) {
                    fb.put_pixel(px + col, py + row, fg);
                }
            }
        }
    }
}

// нарисовать строку в пиксельной позиции
pub fn draw_text_at(text: &str, px: usize, py: usize, fg: u32) {
    let mut x = px;
    let w = font().width;
    for b in text.bytes() {
        draw_char_at(b, x, py, fg);
        x += w;
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
        let f = font();
        let glyph = f.glyph(ch);
        for row in 0..f.height {
            for col in 0..f.width {
                if f.bit_set(glyph, col, row) {
                    // рисуем квадрат scale x scale вместо одного пикселя
                    for sy in 0..scale {
                        for sx in 0..scale {
                            fb.put_pixel(px + col * scale + sx, py + row * scale + sy, fg);
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
    let w = font().width;
    for b in text.bytes() {
        draw_char_scaled(b, x, py, fg, scale);
        x += w * scale;
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
        $crate::framebuffer::set_color($crate::framebuffer::theme_fg());
    }};
}
