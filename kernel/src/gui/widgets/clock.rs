// clock widget: shows system uptime, big and centered
use crate::framebuffer::{self, draw_text_scaled};
use crate::gui::style;
use alloc::string::String;

pub struct Clock {
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
}

impl Clock {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        Clock { cx, cy, cw, ch }
    }

    pub fn update(&mut self) {}

    pub fn redraw(&self) {
        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, style::SURFACE_DARK);

        let (h, m, s) = crate::time::uptime_hms();
        let clock_text = format_hms(h % 24, m, s);

        let text_w = clock_text.len() * 8 * 4;
        let x = self.cx + self.cw.saturating_sub(text_w) / 2;
        let y = self.cy + self.ch / 2 - 16;
        draw_text_scaled(&clock_text, x, y, style::GOOD, 4);

        style::text(
            "system uptime",
            self.cx as i32 + self.cw as i32 / 2 - 50,
            self.cy as i32 + 10,
            style::TEXT_DIM,
        );
    }
}

fn format_hms(h: u64, m: u64, s: u64) -> String {
    let mut out = String::new();
    push_2d(&mut out, h);
    out.push(':');
    push_2d(&mut out, m);
    out.push(':');
    push_2d(&mut out, s);
    out
}

fn push_2d(s: &mut String, v: u64) {
    s.push((b'0' + (v / 10 % 10) as u8) as char);
    s.push((b'0' + (v % 10) as u8) as char);
}
