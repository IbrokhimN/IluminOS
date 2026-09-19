// simple four-function calculator widget
use crate::framebuffer::{self, draw_text_scaled};
use crate::gui::style::{self, hit};
use alloc::string::String;
use alloc::vec::Vec;

pub struct Calc {
    display: String,
    acc: i64,
    pending: u8,
    fresh: bool,
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    buttons: [CalcButton; 16],
}

#[derive(Clone, Copy)]
struct CalcButton {
    key: u8,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

const CALC_KEYS: [&[u8]; 4] = [b"789/", b"456*", b"123-", b"0C=+"];

impl Calc {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let (bw, bh, gap) = (50, 36, 6);
        let (start_x, start_y) = (cx as i32 + 10, cy as i32 + 44);

        let mut buttons = [CalcButton {
            key: 0,
            x: 0,
            y: 0,
            w: bw,
            h: bh,
        }; 16];
        let mut i = 0;
        for (row, keys) in CALC_KEYS.iter().enumerate() {
            for (col, &key) in keys.iter().enumerate() {
                buttons[i] = CalcButton {
                    key,
                    x: start_x + col as i32 * (bw + gap),
                    y: start_y + row as i32 * (bh + gap),
                    w: bw,
                    h: bh,
                };
                i += 1;
            }
        }

        Calc {
            display: String::from("0"),
            acc: 0,
            pending: 0,
            fresh: true,
            cx,
            cy,
            cw,
            ch,
            buttons,
        }
    }

    pub fn redraw(&self) {
        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, style::WINDOW_BG);

        // screen
        let (sx, sy, sw) = (self.cx as i32 + 10, self.cy as i32 + 8, self.cw as i32 - 20);
        style::panel(sx, sy, sw as u32, 28);
        let digit_w = self.display.len() * 16;
        draw_text_scaled(
            &self.display,
            (sx + sw) as usize - 14 - digit_w,
            sy as usize + 4,
            style::GOOD,
            2,
        );

        // keypad
        for b in &self.buttons {
            let label = [b.key];
            let text = core::str::from_utf8(&label).unwrap_or("?");
            style::button(b.x, b.y, b.w as u32, b.h as u32, text);
        }
    }

    pub fn click(&mut self, mx: i32, my: i32) -> bool {
        let Some(b) = self
            .buttons
            .iter()
            .find(|b| hit(mx, my, b.x, b.y, b.w as u32, b.h as u32))
        else {
            return false;
        };
        let key = b.key;
        self.press(key);
        true
    }

    fn press(&mut self, key: u8) {
        match key {
            b'0'..=b'9' => {
                if self.fresh || self.display == "0" {
                    self.display.clear();
                    self.fresh = false;
                }
                if self.display.len() < 10 {
                    self.display.push(key as char);
                }
            }
            b'+' | b'-' | b'*' | b'/' => {
                self.apply();
                self.pending = key;
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
        self.acc = match self.pending {
            b'+' => self.acc + cur,
            b'-' => self.acc - cur,
            b'*' => self.acc * cur,
            b'/' if cur != 0 => self.acc / cur,
            b'/' => 0,
            _ => cur,
        };
        self.display = i64_to_string(self.acc);
    }
}

fn parse_i64(s: &str) -> i64 {
    let mut n: i64 = 0;
    let mut neg = false;
    for (i, b) in s.bytes().enumerate() {
        if i == 0 && b == b'-' {
            neg = true;
        } else if b.is_ascii_digit() {
            n = n * 10 + (b - b'0') as i64;
        }
    }
    if neg { -n } else { n }
}

fn i64_to_string(mut v: i64) -> String {
    if v == 0 {
        return String::from("0");
    }
    let neg = v < 0;
    if neg {
        v = -v;
    }
    let mut digits = Vec::new();
    while v > 0 {
        digits.push(b'0' + (v % 10) as u8);
        v /= 10;
    }
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    for &d in digits.iter().rev() {
        out.push(d as char);
    }
    out
}
