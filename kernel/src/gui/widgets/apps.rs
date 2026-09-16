use crate::framebuffer::{self, draw_text_at, draw_text_scaled};
use crate::fs::{self, FILE_MAX_BYTES};
use crate::gui::style::{self, hit};
use crate::html;
use alloc::string::String;
use alloc::vec::Vec;

// clock

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

// calc

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

// paint

pub struct Paint {
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    color: u32,
    palette: [(u32, i32, i32); 8],
    canvas_y: usize,
}

const PALETTE: [u32; 8] = [
    0x1B2124, 0xE67E80, 0xA7C080, 0x7FBBB3, 0xDBBC7F, 0xD699B6, 0x83C092, 0xD3C6AA,
];

const SWATCH: i32 = 26;

impl Paint {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let mut palette = [(0u32, 0i32, 0i32); 8];
        for (i, &color) in PALETTE.iter().enumerate() {
            palette[i] = (
                color,
                cx as i32 + 10 + i as i32 * (SWATCH + 6),
                cy as i32 + 8,
            );
        }
        Paint {
            cx,
            cy,
            cw,
            ch,
            color: PALETTE[0],
            palette,
            canvas_y: cy + 44,
        }
    }

    pub fn redraw(&self) {
        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, style::WINDOW_BG);
        self.clear_canvas();
        self.draw_toolbar();
    }

    fn clear_canvas(&self) {
        framebuffer::fill_rect(
            self.cx + 4,
            self.canvas_y,
            self.cw - 8,
            self.cy + self.ch - self.canvas_y - 4,
            0xFFFFFF,
        );
    }

    fn draw_toolbar(&self) {
        framebuffer::fill_rect(
            self.cx,
            self.cy,
            self.cw,
            self.canvas_y - self.cy,
            style::WINDOW_BG,
        );

        for &(color, x, y) in &self.palette {
            style::rounded_fill(x, y, SWATCH as u32, SWATCH as u32, color);
            if color == self.color {
                style::rounded_outline(x, y, SWATCH as u32, SWATCH as u32, style::ACCENT);
            }
        }

        let clear_x = self.cx as i32 + self.cw as i32 - 70;
        style::button(clear_x, self.cy as i32 + 8, 60, 28, "Clear");

        // swatch right
        let sw_x = self.cx as i32 + self.cw as i32 - 140;
        style::rounded_fill(sw_x, self.cy as i32 + 8, 28, 28, self.color);
        style::rounded_outline(sw_x, self.cy as i32 + 8, 28, 28, style::BORDER);
    }

    pub fn on_drag(&mut self, mx: i32, my: i32) {
        let inside_canvas = my as usize >= self.canvas_y
            && (my as usize) < self.cy + self.ch - 4
            && mx as usize >= self.cx + 4
            && (mx as usize) < self.cx + self.cw - 4;
        if inside_canvas {
            let (bx, by) = (mx as usize, my as usize);
            framebuffer::fill_rect(bx.saturating_sub(1), by.saturating_sub(1), 4, 4, self.color);
        }
    }

    pub fn on_click(&mut self, mx: i32, my: i32) -> u8 {
        if let Some(&(color, ..)) = self
            .palette
            .iter()
            .find(|&&(_, x, y)| hit(mx, my, x, y, SWATCH as u32, SWATCH as u32))
        {
            self.color = color;
            self.draw_toolbar();
            return 1;
        }

        let clear_x = self.cx as i32 + self.cw as i32 - 70;
        if hit(mx, my, clear_x, self.cy as i32 + 8, 60, 28) {
            self.clear_canvas();
            return 2;
        }

        0
    }
}

// term

const TERM_BG: u32 = 0x000000;
const TERM_FG: u32 = 0x33FF66;

pub struct Term {
    pub lines: Vec<String>,
    pub input: String,
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
}

impl Term {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let lines = alloc::vec![
            String::from("IluminOS terminal"),
            String::from("commands: help ver clear"),
            String::new(),
        ];
        Term {
            lines,
            input: String::new(),
            cx,
            cy,
            cw,
            ch,
        }
    }

    pub fn redraw(&self) {
        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, TERM_BG);

        let max_rows = (self.ch / 10).max(2);
        let visible = max_rows - 1;
        let start = self.lines.len().saturating_sub(visible);

        let mut row = 0;
        for line in &self.lines[start..] {
            draw_text_at(line, self.cx + 2, self.cy + 2 + row * 10, TERM_FG);
            row += 1;
        }

        draw_text_at(">", self.cx + 2, self.cy + 2 + row * 10, TERM_FG);
        draw_text_at(&self.input, self.cx + 10, self.cy + 2 + row * 10, TERM_FG);
    }

    pub fn exec(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        self.lines.push(alloc::format!("> {}", cmd));
        match cmd {
            "" => {}
            "help" => self.lines.push(String::from("commands: help ver clear")),
            "ver" => self.lines.push(String::from("IluminOS 1.0 GUI")),
            "clear" => self.lines.clear(),
            other => self.lines.push(alloc::format!("unknown: {}", other)),
        }
    }
}

// browser

const LINK: u32 = 0x2255CC;

pub struct Browser {
    pub query: String,
    results: Vec<String>,
    searched: bool,
    viewing_html: bool,
    page_doc: Option<html::Document>,
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    search_btn: (i32, i32, i32, i32),
}

impl Browser {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        Browser {
            query: String::new(),
            results: Vec::new(),
            searched: false,
            viewing_html: false,
            page_doc: None,
            cx,
            cy,
            cw,
            ch,
            search_btn: (0, 0, 0, 0),
        }
    }

    pub fn redraw(&mut self) {
        if self.viewing_html {
            self.render_html();
            return;
        }

        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, 0xFFFFFF);
        self.draw_logo();
        self.draw_search_box();

        if self.searched {
            self.draw_results();
        } else {
            style::text(
                "search, or type a page.html to open a file",
                self.cx as i32 + 30,
                self.cy as i32 + 80,
                style::TEXT_DIM,
            );
        }
    }

    fn draw_logo(&self) {
        let logo = "Not-Google";
        let colors = [0x4488FF, 0xE67E80, 0xDBBC7F, 0x4488FF, 0xA7C080, 0xE67E80];
        let logo_x = self.cx as i32 + self.cw as i32 / 2 - style::text_width(logo) / 2;
        let logo_y = self.cy as i32 + 30;
        for (i, ch) in logo.chars().enumerate() {
            let mut buf = [0u8; 4];
            style::text(
                ch.encode_utf8(&mut buf),
                logo_x + i as i32 * 6,
                logo_y,
                colors[i % colors.len()],
            );
        }
    }

    fn draw_search_box(&mut self) {
        let box_y = self.cy as i32 + 54;
        let box_x = self.cx as i32 + 30;
        let box_w = self.cw as i32 - 130;

        style::rounded_fill(box_x, box_y, box_w as u32, 22, 0xFFFFFF);
        style::rounded_outline(box_x, box_y, box_w as u32, 22, style::BORDER);
        style::text(&self.query, box_x + 6, box_y + 6, 0x000000);

        let btn_x = box_x + box_w + 8;
        style::button(btn_x, box_y, 70, 22, "Search");
        self.search_btn = (btn_x, box_y, 70, 22);
    }

    fn draw_results(&self) {
        let mut y = self.cy as i32 + 96;
        style::text("results for", self.cx as i32 + 10, y, style::TEXT_DIM);
        style::text(&self.query, self.cx as i32 + 10 + 70, y, 0x000000);
        y += 18;
        for r in &self.results {
            style::text(r, self.cx as i32 + 10, y, LINK);
            y += 16;
        }
    }

    pub fn do_search(&mut self) {
        let q = self.query.trim();
        if q.is_empty() {
            return;
        }

        if q.ends_with(".html") || q.ends_with(".htm") {
            let name = String::from(q);
            self.open_html(&name);
            return;
        }

        self.viewing_html = false;
        self.results = alloc::vec![
            alloc::format!("www.{}.com - official site", q),
            alloc::format!("en.notpedia.org/wiki/{}", q),
            alloc::format!("{} - news and updates", q),
            alloc::format!("shop.notzone.com/search?q={}", q),
            alloc::format!("How to learn {} - tutorial", q),
        ];
        self.searched = true;
    }

    pub fn open_html(&mut self, name: &str) {
        let mut buf = [0u8; FILE_MAX_BYTES];
        self.page_doc = match fs::read(name, &mut buf) {
            Ok(size) => core::str::from_utf8(&buf[..size]).ok().map(html::parse),
            Err(_) => None,
        };
        self.viewing_html = true;
    }

    pub fn render_html(&self) {
        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, 0xFFFFFF);
        draw_text_at(
            "Not-Google viewer",
            self.cx + 4,
            self.cy + 4,
            style::TEXT_DIM,
        );

        let Some(doc) = &self.page_doc else {
            draw_text_at(
                "page not found or empty",
                self.cx + 10,
                self.cy + 30,
                0xAA0000,
            );
            return;
        };

        let left = self.cx + 8;
        let right_limit = self.cx + self.cw - 8;
        let mut y = self.cy + 20;

        for block in &doc.blocks {
            if block.kind == html::BlockKind::Rule {
                framebuffer::fill_rect(left, y + 4, self.cw - 16, 2, style::BORDER);
                y += 12;
                continue;
            }
            if block.text.is_empty() {
                y += 12;
                continue;
            }

            let bx = left + block.indent;
            let mut start_x = bx;

            if block.kind == html::BlockKind::ListItem {
                if block.list_num > 0 {
                    draw_text_at(&alloc::format!("{}.", block.list_num), bx, y, 0x000000);
                    start_x = bx + 24;
                } else {
                    framebuffer::fill_rect(bx, y + 3, 4, 4, 0x000000);
                    start_x = bx + 12;
                }
            }

            y = draw_wrapped_block(block, start_x, y, left, right_limit, self.cw);

            if y > self.cy + self.ch - 20 {
                break;
            }
        }
    }

    pub fn window_title(&self) -> &str {
        if self.viewing_html {
            if let Some(doc) = &self.page_doc {
                return &doc.title;
            }
        }
        "Not-Google"
    }

    pub fn search_btn_hit(&self, mx: i32, my: i32) -> bool {
        let (x, y, w, h) = self.search_btn;
        hit(mx, my, x, y, w as u32, h as u32)
    }
}

fn draw_wrapped_block(
    block: &html::Block,
    start_x: usize,
    y: usize,
    left: usize,
    right_limit: usize,
    window_w: usize,
) -> usize {
    let char_w = 8 * block.scale;
    let line_h = 10 * block.scale + 4;
    let avail_cols = if char_w > 0 {
        right_limit.saturating_sub(start_x) / char_w
    } else {
        usize::MAX
    };

    if let Some(bg) = block.bg {
        let tw = block.text.len() * char_w;
        framebuffer::fill_rect(
            start_x.saturating_sub(2),
            y.saturating_sub(1),
            tw + 4,
            line_h,
            bg,
        );
    }

    let text_w = block.text.len() * char_w;
    let draw_x = if block.center {
        left + (window_w - 16).saturating_sub(text_w) / 2
    } else {
        start_x
    };

    // one line draw
    if block.text.len() <= avail_cols || block.scale > 1 || block.center {
        let width = draw_text_scaled(&block.text, draw_x, y, block.color, block.scale);
        if block.underline || block.is_link {
            framebuffer::fill_rect(draw_x, y + 8 * block.scale, width, 1, block.color);
        }
        return y + line_h;
    }

    // word wrap
    let mut y = y;
    let mut line = String::new();
    for word in block.text.split(' ') {
        if line.len() + word.len() + 1 > avail_cols && !line.is_empty() {
            draw_text_scaled(&line, start_x, y, block.color, block.scale);
            y += line_h;
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        let width = draw_text_scaled(&line, start_x, y, block.color, block.scale);
        if block.underline || block.is_link {
            framebuffer::fill_rect(start_x, y + 8, width, 1, block.color);
        }
        y += line_h;
    }
    y
}
