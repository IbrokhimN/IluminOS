use crate::framebuffer::{draw_text_at, draw_text_scaled, fill_rect, draw_rect};
use alloc::string::String;
use alloc::vec::Vec;
use crate::random::rdtsc;
use crate::fs::{self, FILE_MAX_BYTES};
use crate::html;

const WIN_FACE: u32 = 0xC0C0C0;
const WIN_LIGHT: u32 = 0xFFFFFF;
const WIN_DARK: u32 = 0x808080;
const BLACK: u32 = 0x000000;
const TERM_BG: u32 = 0x000000;   // terminal background
const TERM_FG: u32 = 0x00FF00;   // terminal green text
const NG_BLUE: u32 = 0x4488FF;   // not-google logo colors
const NG_RED: u32 = 0xFF4444;
const NG_YELLOW: u32 = 0xFFDD33;
const NG_GREEN: u32 = 0x33CC66;
const LINK: u32 = 0x0000EE;      // link blue

fn bevel(x: usize, y: usize, w: usize, h: usize, raised: bool) {
    let (tl, br) = if raised { (WIN_LIGHT, WIN_DARK) } else { (WIN_DARK, WIN_LIGHT) };
    fill_rect(x, y, w, 2, tl);
    fill_rect(x, y, 2, h, tl);
    fill_rect(x, y + h - 2, w, 2, br);
    fill_rect(x + w - 2, y, 2, h, br);
}

// shows uptime derived from the tsc counter
pub struct Clock {
    start_tsc: u64,   // tsc value at start
    tsc_per_sec: u64, // ticks per second calibration
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
        // time comes from the real counter nothing to accumulate
    }

    pub fn redraw(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, 0x001020);
        // real uptime from the cpu tick counter
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
        // large text centered
        let text_w = buf.len() * 8 * 4;
        let x = self.cx + (self.cw.saturating_sub(text_w)) / 2;
        let y = self.cy + self.ch / 2 - 16;
        draw_text_scaled(&buf, x, y, 0x33FF88, 4);
        draw_text_at("system uptime", self.cx + self.cw / 2 - 52, self.cy + 10, 0x88AACC);
    }
}

pub struct Calc {
    display: String,
    acc: i64,      // accumulated value
    pending: u8,   // pending operator or 0
    fresh: bool,   // next digit starts a new number
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    buttons: [(u8, usize, usize, usize, usize); 20], // char x y w h
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
        // display
        fill_rect(self.cx + 10, self.cy + 8, self.cw - 20, 28, 0x203020);
        draw_rect(self.cx + 10, self.cy + 8, self.cw - 20, 28, WIN_DARK);
        let tw = self.display.len() * 16;
        let dx = self.cx + self.cw - 14 - tw;
        draw_text_scaled(&self.display, dx, self.cy + 12, 0x66FF66, 2);
        // buttons
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

    // handle click always returns true to redraw
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
    palette: [(u32, usize, usize); 8], // color x y
    canvas_y: usize, // top of the drawing area
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

    // full redraw on window open includes clearing canvas
    pub fn redraw(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, WIN_FACE);
        // white canvas only on full redraw
        fill_rect(self.cx + 4, self.canvas_y, self.cw - 8, self.cy + self.ch - self.canvas_y - 4, 0xFFFFFF);
        self.draw_toolbar();
    }

    // draw toolbar palette and buttons only not the canvas
    fn draw_toolbar(&self) {
        // toolbar background above the canvas
        fill_rect(self.cx, self.cy, self.cw, self.canvas_y - self.cy, WIN_FACE);
        // palette
        for &(col, px, py) in self.palette.iter() {
            fill_rect(px, py, 28, 28, col);
            draw_rect(px, py, 28, 28, BLACK);
        }
        // clear button on the right
        let clx = self.cx + self.cw - 70;
        fill_rect(clx, self.cy + 8, 60, 28, WIN_FACE);
        bevel(clx, self.cy + 8, 60, 28, true);
        draw_text_at("Clear", clx + 14, self.cy + 18, BLACK);
        // current color swatch
        fill_rect(self.cx + self.cw - 140, self.cy + 8, 28, 28, self.color);
        draw_rect(self.cx + self.cw - 140, self.cy + 8, 28, 28, BLACK);
    }

    // drag draws a brush dot
    pub fn on_drag(&mut self, mx: i32, my: i32) {
        // 3x3 brush inside the canvas area
        if my as usize >= self.canvas_y && (my as usize) < self.cy + self.ch - 4
            && mx as usize >= self.cx + 4 && (mx as usize) < self.cx + self.cw - 4 {
            let bx = mx as usize;
            let by = my as usize;
            fill_rect(bx.saturating_sub(1), by.saturating_sub(1), 4, 4, self.color);
        }
    }

    // click checks palette and clear button 0 nothing 1 color changed 2 canvas cleared
    pub fn on_click(&mut self, mx: i32, my: i32) -> u8 {
        // palette changes color
        for &(col, px, py) in self.palette.iter() {
            if mx >= px as i32 && mx < (px+28) as i32 && my >= py as i32 && my < (py+28) as i32 {
                self.color = col;
                self.draw_toolbar(); // redraw toolbar only leave canvas alone
                return 1;
            }
        }
        // clear button
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

// terminal and browser moved from desktop.rs

pub struct Term {
    pub lines: Vec<String>, // output lines
    pub input: String,      // current input
    cx: usize, cy: usize, cw: usize, ch: usize, // window content area
}

impl Term {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let mut lines = Vec::new();
        lines.push(String::from("IluminOS terminal"));
        lines.push(String::from("commands help ver clear"));
        lines.push(String::from(""));
        Term { lines, input: String::new(), cx, cy, cw, ch }
    }

    // redraw terminal last lines plus the input line
    pub fn redraw(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, TERM_BG);
        let max_rows = (self.ch / 10).max(2);
        let visible = max_rows - 1;
        // show only the last visible lines scroll
        let start = if self.lines.len() > visible { self.lines.len() - visible } else { 0 };
        let mut row = 0;
        for line in &self.lines[start..] {
            draw_text_at(line, self.cx + 2, self.cy + 2 + row * 10, TERM_FG);
            row += 1;
        }
        // input line with a prompt
        draw_text_at(">", self.cx + 2, self.cy + 2 + row * 10, TERM_FG);
        draw_text_at(&self.input, self.cx + 2 + 8, self.cy + 2 + row * 10, TERM_FG);
    }

    // run a terminal command small builtin set
    pub fn exec(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        let mut echo = String::from("> ");
        echo.push_str(cmd);
        self.lines.push(echo); // echo the typed command
        match cmd {
            "" => {}
            "help" => self.lines.push(String::from("commands help ver clear")),
            "ver" => self.lines.push(String::from("IluminOS 1.0 GUI")),
            "clear" => self.lines.clear(),
            _ => {
                let mut e = String::from("unknown ");
                e.push_str(cmd);
                self.lines.push(e);
            }
        }
    }
}

// not-google joke browser plus html file viewer
pub struct Browser {
    pub query: String,                    // search query
    results: Vec<String>,             // fake results
    searched: bool,                   // whether a search ran
    viewing_html: bool,               // html page view mode
    page_doc: Option<html::Document>, // parsed page
    cx: usize, cy: usize, cw: usize, ch: usize,
    search_btn: (usize, usize, usize, usize), // search button area
}

impl Browser {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        Browser {
            query: String::new(), results: Vec::new(),
            searched: false, viewing_html: false, page_doc: None,
            cx, cy, cw, ch, search_btn: (0, 0, 0, 0),
        }
    }

    // either the html page or the search home page
    pub fn redraw(&mut self) {
        if self.viewing_html {
            self.render_html();
            return;
        }
        fill_rect(self.cx, self.cy, self.cw, self.ch, 0xFFFFFF); // white background

        // not-google logo centered letters in different colors
        let logo_y = self.cy + 30;
        let cx_center = self.cx + self.cw / 2;
        let logo = "Not-Google";
        let logo_x = cx_center - (logo.len() * 8) / 2;
        let colors = [NG_BLUE, NG_RED, NG_YELLOW, NG_BLUE, NG_GREEN, NG_RED];
        let mut lx = logo_x;
        for (i, ch) in logo.bytes().enumerate() {
            let color = colors[i % colors.len()];
            let s = [ch];
            if let Ok(st) = core::str::from_utf8(&s) {
                draw_text_at(st, lx, logo_y, color);
            }
            lx += 8;
        }

        // search box
        let box_y = logo_y + 24;
        let box_x = self.cx + 30;
        let box_w = self.cw - 130;
        fill_rect(box_x, box_y, box_w, 20, 0xFFFFFF);
        draw_rect(box_x, box_y, box_w, 20, WIN_DARK);
        draw_text_at(&self.query, box_x + 4, box_y + 6, BLACK);

        // search button remember its area for click detection
        let btn_x = box_x + box_w + 8;
        let btn_w = 70;
        fill_rect(btn_x, box_y, btn_w, 20, WIN_FACE);
        bevel(btn_x, box_y, btn_w, 20, true);
        draw_text_at("Search", btn_x + 8, box_y + 6, BLACK);
        self.search_btn = (btn_x, box_y, btn_w, 20);

        // results or a hint
        if self.searched {
            let mut ry = box_y + 40;
            draw_text_at("results for", self.cx + 10, ry, WIN_DARK);
            draw_text_at(&self.query, self.cx + 10 + 96, ry, BLACK);
            ry += 16;
            for r in &self.results {
                draw_text_at(r, self.cx + 10, ry, LINK);
                ry += 14;
            }
        } else {
            draw_text_at("search, or type page.html to open a file", self.cx + 20, box_y + 40, WIN_DARK);
        }
    }

    // search open the file if query ends in html else fake results
    pub fn do_search(&mut self) {
        let q = self.query.trim();
        if q.is_empty() {
            return;
        }
        // html filename opens the page
        if q.ends_with(".html") || q.ends_with(".htm") {
            let name = String::from(q);
            self.open_html(&name);
            return;
        }
        // normal search generate plausible fake links
        self.viewing_html = false;
        self.results.clear();
        let mut r1 = String::from("www.");
        r1.push_str(q); r1.push_str(".com - official site");
        self.results.push(r1);
        let mut r2 = String::from("en.notpedia.org/wiki/");
        r2.push_str(q);
        self.results.push(r2);
        let mut r3 = String::from(q);
        r3.push_str(" - news and updates");
        self.results.push(r3);
        let mut r4 = String::from("shop.notzone.com/search?q=");
        r4.push_str(q);
        self.results.push(r4);
        let mut r5 = String::from("How to learn ");
        r5.push_str(q); r5.push_str(" - tutorial");
        self.results.push(r5);
        self.searched = true;
    }

    // read html file from fs and parse it
    pub fn open_html(&mut self, name: &str) {
        let mut buf = [0u8; FILE_MAX_BYTES];
        match fs::read(name, &mut buf) {
            Ok(size) => {
                if let Ok(text) = core::str::from_utf8(&buf[..size]) {
                    self.page_doc = Some(html::parse(text)); // parse see html.rs
                    self.viewing_html = true;
                }
            }
            Err(_) => {
                self.page_doc = None;
                self.viewing_html = true;
            }
        }
    }

    // draw parsed page block by block
    pub fn render_html(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, 0xFFFFFF);
        draw_text_at("Not-Google viewer", self.cx + 4, self.cy + 4, WIN_DARK);

        let doc = match &self.page_doc {
            Some(d) => d,
            None => {
                draw_text_at("page not found or empty", self.cx + 10, self.cy + 30, 0xAA0000);
                return;
            }
        };

        let left = self.cx + 8;
        let right_limit = self.cx + self.cw - 8;
        let max_cols = (self.cw - 16) / 8; // chars that fit in width
        let mut y = self.cy + 20;

        // draw each block in order moving y down
        for block in &doc.blocks {
            // hr horizontal rule
            if block.kind == html::BlockKind::Rule {
                fill_rect(left, y + 4, self.cw - 16, 2, WIN_DARK);
                y += 12;
                continue;
            }
            // empty block just adds spacing
            if block.text.is_empty() {
                y += 12;
                continue;
            }

            let bx = left + block.indent;
            let mut startx = bx;

            // list item marker number or bullet
            if block.kind == html::BlockKind::ListItem {
                if block.list_num > 0 {
                    let n = block.list_num;
                    let mut label = String::new();
                    if n >= 10 { label.push((b'0' + (n/10) as u8) as char); }
                    label.push((b'0' + (n % 10) as u8) as char);
                    label.push('.');
                    draw_text_at(&label, bx, y, BLACK);
                    startx = bx + 24;
                } else {
                    fill_rect(bx, y + 3, 4, 4, BLACK); // bullet
                    startx = bx + 12;
                }
            }

            let char_w = 8 * block.scale;       // char width scaled
            let line_h = 10 * block.scale + 4;  // line height

            // chars that fit in remaining width
            let avail_cols = if char_w > 0 { (right_limit.saturating_sub(startx)) / char_w } else { max_cols };

            // background for code blocks
            if let Some(bg) = block.bg {
                let tw = block.text.len() * char_w;
                fill_rect(startx.saturating_sub(2), y.saturating_sub(1), tw + 4, line_h, bg);
            }

            // position accounting for centering
            let text_w = block.text.len() * char_w;
            let draw_x = if block.center {
                left + (self.cw - 16).saturating_sub(text_w) / 2
            } else {
                startx
            };

            // fits as is just draw it
            if block.text.len() <= avail_cols || block.scale > 1 || block.center {
                let width = draw_text_scaled(&block.text, draw_x, y, block.color, block.scale);
                if block.underline || block.is_link {
                    fill_rect(draw_x, y + 8 * block.scale, width, 1, block.color); // link underline
                }
                y += line_h;
            } else {
                // long text word wrap
                let words = block.text.split(' ');
                let mut line = String::new();
                for w in words {
                    let test_len = line.len() + w.len() + 1;
                    // word does not fit flush current line and start a new one
                    if test_len > avail_cols && !line.is_empty() {
                        draw_text_scaled(&line, startx, y, block.color, block.scale);
                        y += line_h;
                        line = String::new();
                    }
                    if !line.is_empty() { line.push(' '); }
                    line.push_str(w);
                }
                // remainder
                if !line.is_empty() {
                    let width = draw_text_scaled(&line, startx, y, block.color, block.scale);
                    if block.underline || block.is_link {
                        fill_rect(startx, y + 8, width, 1, block.color);
                    }
                    y += line_h;
                }
            }

            // past bottom of window stop drawing
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

    // whether click hit the search button
    pub fn search_btn_hit(&self, mx: i32, my: i32) -> bool {
        let (bx, by, bw, bh) = self.search_btn;
        mx >= bx as i32 && mx < (bx + bw) as i32 && my >= by as i32 && my < (by + bh) as i32
    }
}

// app window title
