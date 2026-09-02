use crate::framebuffer::{draw_text_at, draw_text_scaled, fill_rect, draw_rect};
use alloc::string::String;
use crate::random::rdtsc;

const WIN_FACE: u32 = 0xC0C0C0;
const WIN_LIGHT: u32 = 0xFFFFFF;
const WIN_DARK: u32 = 0x808080;
const BLACK: u32 = 0x000000;

fn bevel(x: usize, y: usize, w: usize, h: usize, raised: bool) {
    let (tl, br) = if raised { (WIN_LIGHT, WIN_DARK) } else { (WIN_DARK, WIN_LIGHT) };
    fill_rect(x, y, w, 2, tl);
    fill_rect(x, y, 2, h, tl);
    fill_rect(x, y + h - 2, w, 2, br);
    fill_rect(x + w - 2, y, 2, h, br);
}

// показывают счётчик тактов как время работы системы
pub struct Clock {
    start_tsc: u64,   // счётчик тактов в момент запуска
    tsc_per_sec: u64, // тактов в секунду калибровка
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
}

impl Clock {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        Clock { start_tsc: rdtsc(), tsc_per_sec: 2_000_000_000, cx, cy, cw, ch }
    }

    pub fn update(&mut self) {
        // ничего не накапливаем время берём из реального счётчика
    }

    pub fn redraw(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, 0x001020);
        // реальное время работы из счётчика тактов процессора
        let elapsed = rdtsc().wrapping_sub(self.start_tsc);
        let secs = elapsed / self.tsc_per_sec;
        let h = (secs / 3600) % 24;
        let m = (secs / 60) % 60;
        let s = secs % 60;
        let mut buf = String::new();
        push_2d(&mut buf, h);
        buf.push(':');
        push_2d(&mut buf, m);
        buf.push(':');
        push_2d(&mut buf, s);
        // крупно по центру
        let text_w = buf.len() * 8 * 4;
        let x = self.cx + (self.cw.saturating_sub(text_w)) / 2;
        let y = self.cy + self.ch / 2 - 16;
        draw_text_scaled(&buf, x, y, 0x33FF88, 4);
        draw_text_at("system uptime", self.cx + self.cw / 2 - 52, self.cy + 10, 0x88AACC);
    }
}

pub struct Calc {
    display: String,
    acc: i64,      // накопленное значение
    pending: u8,   // ожидающая операция + - * / или 0
    fresh: bool,   // следующая цифра начинает новое число
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    buttons: [(u8, usize, usize, usize, usize); 20], // символ x y w h
}

const CALC_KEYS: [&[u8]; 5] = [
    b"789/",
    b"456*",
    b"123-",
    b"0C=+",
    b"",
];

impl Calc {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let mut buttons = [(0u8, 0usize, 0usize, 0usize, 0usize); 20];
        let bw = 50;
        let bh = 36;
        let gap = 6;
        let startx = cx + 10;
        let starty = cy + 44;
        let mut idx = 0;
        for (row, keys) in CALC_KEYS.iter().enumerate() {
            for (col, &k) in keys.iter().enumerate() {
                let bx = startx + col * (bw + gap);
                let by = starty + row * (bh + gap);
                if idx < 20 {
                    buttons[idx] = (k, bx, by, bw, bh);
                    idx += 1;
                }
            }
        }
        Calc { display: String::from("0"), acc: 0, pending: 0, fresh: true, cx, cy, cw, ch, buttons }
    }

    pub fn redraw(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, WIN_FACE);
        // дисплей
        fill_rect(self.cx + 10, self.cy + 8, self.cw - 20, 28, 0x203020);
        draw_rect(self.cx + 10, self.cy + 8, self.cw - 20, 28, WIN_DARK);
        let tw = self.display.len() * 16;
        let dx = self.cx + self.cw - 14 - tw;
        draw_text_scaled(&self.display, dx, self.cy + 12, 0x66FF66, 2);
        // кнопки
        for &(k, bx, by, bw, bh) in self.buttons.iter() {
            if k == 0 { continue; }
            fill_rect(bx, by, bw, bh, WIN_FACE);
            bevel(bx, by, bw, bh, true);
            let s = [k];
            if let Ok(st) = core::str::from_utf8(&s) {
                draw_text_scaled(st, bx + bw/2 - 8, by + bh/2 - 8, BLACK, 2);
            }
        }
    }

    // обработать клик вернуть true всегда для перерисовки
    pub fn click(&mut self, mx: i32, my: i32) -> bool {
        for &(k, bx, by, bw, bh) in self.buttons.iter() {
            if k == 0 { continue; }
            if mx >= bx as i32 && mx < (bx+bw) as i32 && my >= by as i32 && my < (by+bh) as i32 {
                self.press(k);
                return true;
            }
        }
        false
    }

    fn press(&mut self, k: u8) {
        match k {
            b'0'..=b'9' => {
                if self.fresh || self.display == "0" {
                    self.display = String::new();
                    self.fresh = false;
                }
                if self.display.len() < 10 {
                    self.display.push(k as char);
                }
            }
            b'+' | b'-' | b'*' | b'/' => {
                self.apply();
                self.pending = k;
                self.fresh = true;
            }
            b'=' => {
                self.apply();
                self.pending = 0;
                self.fresh = true;
            }
            b'C' => {
                self.display = String::from("0");
                self.acc = 0;
                self.pending = 0;
                self.fresh = true;
            }
            _ => {}
        }
    }

    fn apply(&mut self) {
        let cur = parse_i64(&self.display);
        let result = match self.pending {
            b'+' => self.acc + cur,
            b'-' => self.acc - cur,
            b'*' => self.acc * cur,
            b'/' => if cur != 0 { self.acc / cur } else { 0 },
            _ => cur,
        };
        self.acc = result;
        self.display = i64_to_string(result);
    }
}

pub struct Paint {
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    color: u32,
    palette: [(u32, usize, usize); 8], // цвет x y
    canvas_y: usize, // верх области рисования
}

const PALETTE: [u32; 8] = [
    0x000000, 0xFF0000, 0x00AA00, 0x0000FF,
    0xFFDD00, 0xFF00FF, 0x00DDDD, 0xFFFFFF,
];

impl Paint {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let mut palette = [(0u32, 0usize, 0usize); 8];
        for (i, &col) in PALETTE.iter().enumerate() {
            palette[i] = (col, cx + 10 + i * 34, cy + 8);
        }
        Paint { cx, cy, cw, ch, color: 0x000000, palette, canvas_y: cy + 44 }
    }

    // полная отрисовка при открытии окна включая очистку холста
    pub fn redraw(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, WIN_FACE);
        // холст белый только при полной отрисовке
        fill_rect(self.cx + 4, self.canvas_y, self.cw - 8, self.cy + self.ch - self.canvas_y - 4, 0xFFFFFF);
        self.draw_toolbar();
    }

    // нарисовать только панель инструментов палитра и кнопки БЕЗ холста
    fn draw_toolbar(&self) {
        // фон панели сверху до холста
        fill_rect(self.cx, self.cy, self.cw, self.canvas_y - self.cy, WIN_FACE);
        // палитра
        for &(col, px, py) in self.palette.iter() {
            fill_rect(px, py, 28, 28, col);
            draw_rect(px, py, 28, 28, BLACK);
        }
        // кнопка очистки справа
        let clx = self.cx + self.cw - 70;
        fill_rect(clx, self.cy + 8, 60, 28, WIN_FACE);
        bevel(clx, self.cy + 8, 60, 28, true);
        draw_text_at("Clear", clx + 14, self.cy + 18, BLACK);
        // индикатор текущего цвета
        fill_rect(self.cx + self.cw - 140, self.cy + 8, 28, 28, self.color);
        draw_rect(self.cx + self.cw - 140, self.cy + 8, 28, 28, BLACK);
    }

    // обработка зажатой мыши рисуем точку кистью. вернуть нужна ли перерисовка палитры
    pub fn on_drag(&mut self, mx: i32, my: i32) {
        // в области холста рисуем кисть 3x3
        if my as usize >= self.canvas_y && (my as usize) < self.cy + self.ch - 4
            && mx as usize >= self.cx + 4 && (mx as usize) < self.cx + self.cw - 4 {
            let bx = mx as usize;
            let by = my as usize;
            fill_rect(bx.saturating_sub(1), by.saturating_sub(1), 4, 4, self.color);
        }
    }

    // клик проверить палитру и кнопку очистки вернуть true если сменилось состояние
    // вернуть 0 ничего 1 сменился цвет только toolbar 2 очистка холста
    pub fn on_click(&mut self, mx: i32, my: i32) -> u8 {
        // палитра - смена цвета
        for &(col, px, py) in self.palette.iter() {
            if mx >= px as i32 && mx < (px+28) as i32 && my >= py as i32 && my < (py+28) as i32 {
                self.color = col;
                self.draw_toolbar(); // обновляем только панель холст не трогаем
                return 1;
            }
        }
        // кнопка очистки
        let clx = self.cx + self.cw - 70;
        if mx >= clx as i32 && mx < (clx+60) as i32
            && my >= (self.cy+8) as i32 && my < (self.cy+36) as i32 {
            fill_rect(self.cx + 4, self.canvas_y, self.cw - 8,
                      self.cy + self.ch - self.canvas_y - 4, 0xFFFFFF);
            return 2;
        }
        0
    }
}

fn push_2d(s: &mut String, v: u64) {
    s.push((b'0' + (v / 10 % 10) as u8) as char);
    s.push((b'0' + (v % 10) as u8) as char);
}

fn parse_i64(s: &str) -> i64 {
    let mut n: i64 = 0;
    let mut neg = false;
    for (i, b) in s.bytes().enumerate() {
        if i == 0 && b == b'-' { neg = true; continue; }
        if b >= b'0' && b <= b'9' {
            n = n * 10 + (b - b'0') as i64;
        }
    }
    if neg { -n } else { n }
}

fn i64_to_string(mut v: i64) -> String {
    if v == 0 { return String::from("0"); }
    let neg = v < 0;
    if neg { v = -v; }
    let mut digits = String::new();
    let mut tmp = [0u8; 20];
    let mut i = 0;
    while v > 0 {
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
        i += 1;
    }
    if neg { digits.push('-'); }
    while i > 0 {
        i -= 1;
        digits.push(tmp[i] as char);
    }
    digits
}
