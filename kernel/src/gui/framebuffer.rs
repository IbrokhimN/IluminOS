use core::fmt;
use spin::{Mutex, Once};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::*,
    primitives::Rectangle,
    Pixel,
};

// colors in 0x00rrggbb format
pub const WHITE: u32    =    0xFFFFFF;  // #ffffff
pub const GRAY: u32     =    0xAAAAAA;  // #aaaaaa
pub const GREEN: u32    =    0x33FF66;  // #33ff66
pub const RED: u32      =    0xFF4444;  // #ff4444 
pub const CYAN: u32     =    0x33DDFF;  // #33ddff
pub const YELLOW: u32   =    0xFFDD33;  // #ffdd33
pub const BLACK: u32    =    0x000000;  // #000000
pub const BLUE: u32     =    0x5599FF;  // #5599ff
pub const MAGENTA: u32  =    0xCC77FF;  // #cc77ff


// everforest color palette
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


// вшитый шрифт psf
static FONT_BYTES: &[u8] = include_bytes!("./fonts/ruscii_8x8.psfu");

struct PsfFont {
    width: usize,
    height: usize,
    bytes_per_row: usize,
    glyph_size: usize,
    glyphs_offset: usize,
    num_glyphs: usize,
}

impl PsfFont {
    fn parse(data: &[u8]) -> Self {
        // проверка psf1
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
        // проверка psf2
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
        // запасной шрифт если файл не распознан
        PsfFont { width: 8, height: 16, bytes_per_row: 1, glyph_size: 16, glyphs_offset: 0, num_glyphs: 0 }
    }

    #[inline]
    fn glyph(&self, ch: u8) -> &'static [u8] {
        let idx = if (ch as usize) < self.num_glyphs { ch as usize } else { 0 };
        let start = self.glyphs_offset + idx * self.glyph_size;
        let end = start + self.glyph_size;
        &FONT_BYTES[start..end]
    }

    #[inline]
    fn bit_set(&self, glyph: &[u8], x: usize, y: usize) -> bool {
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

// цвет текста темы
pub fn theme_fg() -> u32 {
    *THEME_FG.lock()
}

// сменить цветовую тему
pub fn set_theme(fg: u32, bg: u32) {
    *THEME_FG.lock() = fg;
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        fb.fg = fg;
        fb.bg = bg;
    }
}

unsafe impl Send for Fb {}

// инициализация фреймбуфера
pub fn init(addr: *mut u8, width: usize, height: usize, pitch: usize) {
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

// установить текущий цвет
pub fn set_color(color: u32) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        fb.fg = color;
    }
}

// нарисовать курсор подчеркивания
pub fn draw_edit_cursor(col: usize, row: usize) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        let f = font();
        let px = col * f.width;
        let py = row * f.height;
        let fg = fb.fg;
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
            core::ptr::copy(src, dst, total - line_bytes);

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
            0x01..=0x07 | 0x0b..=0x0f | 0x20..=0x7e | 0x80..=0xff => {
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

// очистить экран
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

// переключить видимость курсора
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

// скрыть курсор
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

// буфер пикселей напрямую из физ памяти без кучи
pub struct RawCanvas {
    ptr: *mut u32,
    len: usize,
}

unsafe impl Send for RawCanvas {}

// базовый виртуальный адрес для холста
const CANVAS_BASE: u64 = 0xFFFF_A000_0000_0000;

impl RawCanvas {
    // выделить физические фреймы и смапить их
    pub fn new(pixel_count: usize) -> Option<Self> {
        use crate::mem::{allocator, paging};

        let bytes = (pixel_count as u64) * 4;
        let frame_count = bytes.div_ceil(allocator::FRAME_SIZE) as usize;
        let phys_base = allocator::alloc_frames(frame_count)?;

        for i in 0..frame_count as u64 {
            let virt = CANVAS_BASE + i * allocator::FRAME_SIZE;
            let phys = phys_base + i * allocator::FRAME_SIZE;
            paging::map(virt, phys, paging::WRITABLE | paging::NO_EXECUTE).ok()?;
        }

        Some(RawCanvas { ptr: CANVAS_BASE as *mut u32, len: pixel_count })
    }

    pub fn as_slice(&self) -> &[u32] {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

// размеры экрана в пикселях
pub fn dimensions() -> (usize, usize) {
    let guard = FB.lock();
    if let Some(fb) = guard.as_ref() {
        (fb.width, fb.height)
    } else {
        (0, 0)
    }
}

// скопировать область экрана в буфер
pub fn capture_into(dst: &mut [u32], x: usize, y: usize, w: usize, h: usize) {
    let guard = FB.lock();
    if let Some(fb) = guard.as_ref() {
        for row in 0..h {
            let sy = y + row;
            if sy >= fb.height {
                break;
            }
            for col in 0..w {
                let sx = x + col;
                if sx >= fb.width {
                    break;
                }
                let offset = sy * fb.pitch + sx * 4;
                dst[row * w + col] = unsafe { fb.addr.add(offset).cast::<u32>().read_volatile() };
            }
        }
    }
}

// скопировать буфер обратно на экран
pub fn blit(x: usize, y: usize, w: usize, h: usize, buf: &[u32]) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        for row in 0..h {
            let dy = y + row;
            if dy >= fb.height {
                break;
            }
            let row_start = row * w;
            for col in 0..w {
                let dx = x + col;
                if dx >= fb.width {
                    break;
                }
                fb.put_pixel(dx, dy, buf[row_start + col]);
            }
        }
    }
}

// конвертация u32 в rgb888
pub fn u32_to_rgb888(color: u32) -> Rgb888 {
    Rgb888::new((color >> 16) as u8, (color >> 8) as u8, color as u8)
}

// конвертация rgb888 в u32
fn rgb888_to_u32(color: Rgb888) -> u32 {
    ((color.r() as u32) << 16) | ((color.g() as u32) << 8) | color.b() as u32
}

pub struct Display;

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        let (w, h) = dimensions();
        Size::new(w as u32, h as u32)
    }
}

impl DrawTarget for Display {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let mut guard = FB.lock();
        if let Some(fb) = guard.as_mut() {
            for Pixel(point, color) in pixels {
                if point.x >= 0 && point.y >= 0 {
                    fb.put_pixel(point.x as usize, point.y as usize, rgb888_to_u32(color));
                }
            }
        }
        Ok(())
    }

    // быстрый закрас прямоугольника
    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let mut guard = FB.lock();
        if let Some(fb) = guard.as_mut() {
            let c = rgb888_to_u32(color);
            let x0 = area.top_left.x.max(0) as usize;
            let y0 = area.top_left.y.max(0) as usize;
            for y in y0..y0 + area.size.height as usize {
                for x in x0..x0 + area.size.width as usize {
                    fb.put_pixel(x, y, c);
                }
            }
        }
        Ok(())
    }

    // быстрая отрисовка сплошного потока пикселей
    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let mut guard = FB.lock();
        if let Some(fb) = guard.as_mut() {
            let x0 = area.top_left.x.max(0) as usize;
            let y0 = area.top_left.y.max(0) as usize;
            let w = area.size.width as usize;
            let h = area.size.height as usize;
            let mut colors = colors.into_iter();
            'rows: for row in 0..h {
                let y = y0 + row;
                for col in 0..w {
                    let Some(color) = colors.next() else { break 'rows };
                    let x = x0 + col;
                    if x < fb.width && y < fb.height {
                        fb.put_pixel(x, y, rgb888_to_u32(color));
                    }
                }
            }
        }
        Ok(())
    }
}

// нарисовать один пиксель
pub fn pixel(x: usize, y: usize, color: u32) {
    let _ = Display.draw_iter([Pixel(Point::new(x as i32, y as i32), u32_to_rgb888(color))]);
}

// закрашенный прямоугольник
pub fn fill_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    use embedded_graphics::primitives::PrimitiveStyle;
    let rect = Rectangle::new(Point::new(x as i32, y as i32), Size::new(w as u32, h as u32));
    let _ = rect.into_styled(PrimitiveStyle::with_fill(u32_to_rgb888(color))).draw(&mut Display);
}

// контур прямоугольника в один пиксель
pub fn draw_rect(x: usize, y: usize, w: usize, h: usize, color: u32) {
    use embedded_graphics::primitives::PrimitiveStyle;
    let rect = Rectangle::new(Point::new(x as i32, y as i32), Size::new(w as u32, h as u32));
    let _ = rect.into_styled(PrimitiveStyle::with_stroke(u32_to_rgb888(color), 1)).draw(&mut Display);
}

// нарисовать символ по координатам
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

// нарисовать строку по координатам
pub fn draw_text_at(text: &str, px: usize, py: usize, fg: u32) {
    let mut x = px;
    let w = font().width;
    for c in text.chars() {
        draw_char_at(c as u8, x, py, fg);
        x += w;
    }
}

// нарисовать стрелку курсора мыши
pub fn draw_cursor_arrow(px: usize, py: usize) {
    fill_triangle_local(px, py, (0, 0), (0, 11), (8, 8), BLACK);
    fill_triangle_local(px, py, (1, 2), (1, 9), (6, 7), WHITE);
}

// проверка стороны точки относительно отрезка
fn edge_sign(p: (i32, i32), a: (i32, i32), b: (i32, i32)) -> i32 {
    (p.0 - b.0) * (a.1 - b.1) - (a.0 - b.0) * (p.1 - b.1)
}

// закрасить треугольник
fn fill_triangle_local(px: usize, py: usize, a: (i32, i32), b: (i32, i32), c: (i32, i32), color: u32) {
    let min_x = a.0.min(b.0).min(c.0).max(0);
    let max_x = a.0.max(b.0).max(c.0).min(11);
    let min_y = a.1.min(b.1).min(c.1).max(0);
    let max_y = a.1.max(b.1).max(c.1).min(11);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x, y);
            let d1 = edge_sign(p, a, b);
            let d2 = edge_sign(p, b, c);
            let d3 = edge_sign(p, c, a);
            let has_neg = d1 < 0 || d2 < 0 || d3 < 0;
            let has_pos = d1 > 0 || d2 > 0 || d3 > 0;
            if !(has_neg && has_pos) {
                pixel(px + x as usize, py + y as usize, color);
            }
        }
    }
}

// буфер для сохранения фона под курсором
static mut CURSOR_BG: [u32; 144] = [0; 144];
static mut CURSOR_SAVED: bool = false;
static mut CURSOR_X: usize = 0;
static mut CURSOR_Y: usize = 0;

// сохранить пиксели под курсором
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

// восстановить пиксели под курсором
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

// нарисовать увеличенный символ
pub fn draw_char_scaled(ch: u8, px: usize, py: usize, fg: u32, scale: usize) {
    let mut guard = FB.lock();
    if let Some(fb) = guard.as_mut() {
        let f = font();
        let glyph = f.glyph(ch);
        for row in 0..f.height {
            for col in 0..f.width {
                if f.bit_set(glyph, col, row) {
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

// нарисовать увеличенный текст
pub fn draw_text_scaled(text: &str, px: usize, py: usize, fg: u32, scale: usize) -> usize {
    let mut x = px;
    let w = font().width;
    for c in text.chars() {
        draw_char_scaled(c as u8, x, py, fg, scale);
        x += w * scale;
    }
    x - px
}

pub struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut guard = FB.lock();
        if let Some(fb) = guard.as_mut() {
            for c in s.chars() {
                fb.write_char(c as u8);
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
