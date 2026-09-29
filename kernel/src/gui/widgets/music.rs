use crate::framebuffer::{self, draw_text_scaled};
use crate::gui::style;
use crate::gui::wm::Rect;
use crate::keyboard::{KEY_LEFT, KEY_RIGHT};
use crate::player::{self, PLAYER_SAMPLE_RATE};
use crate::sound::{self, SongPlayer};
use alloc::format;
use alloc::string::String;

const ROW_H: i32 = 18;
const LIST_TOP: i32 = 132;
const HEADER_H: i32 = 56;
const PUMP_INTERVAL_MS: u64 = 20;
const BAR_Y: i32 = 74;
const CONTROLS_Y: i32 = 94;

#[derive(PartialEq, Clone, Copy)]
enum State {
    Stopped,
    Playing,
    Paused,
}

pub struct MusicPlayer {
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    current: usize,
    state: State,
    song: Option<SongPlayer>,
    last_sec: u32,
    dirty: bool,
    failed: bool,
    full: bool,
    next_pump: u64,
    bar: Rect,
    bar_hit: Rect,
    btn_prev: Rect,
    btn_toggle: Rect,
    btn_stop: Rect,
    btn_next: Rect,
}

impl MusicPlayer {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let (x, y, w) = (cx as i32, cy as i32, cw as i32);

        let bar = Rect::new(x + 10, y + BAR_Y, w - 20, 8);
        let bar_hit = Rect::new(bar.x, bar.y - 6, bar.w, bar.h + 12);

        let (bw, bh, gap) = (72, 26, 8);
        let total = bw * 4 + gap * 3;
        let bx = x + (w - total) / 2;
        let by = y + CONTROLS_Y;

        MusicPlayer {
            cx,
            cy,
            cw,
            ch,
            current: 0,
            state: State::Stopped,
            song: None,
            last_sec: 0,
            dirty: false,
            failed: false,
            full: true,
            next_pump: 0,
            bar,
            bar_hit,
            btn_prev: Rect::new(bx, by, bw, bh),
            btn_toggle: Rect::new(bx + (bw + gap), by, bw, bh),
            btn_stop: Rect::new(bx + 2 * (bw + gap), by, bw, bh),
            btn_next: Rect::new(bx + 3 * (bw + gap), by, bw, bh),
        }
    }

    fn visible_rows(&self) -> usize {
        let free = self.ch as i32 - LIST_TOP - 6;
        (free / ROW_H).max(1) as usize
    }

    fn first_visible(&self) -> usize {
        let rows = self.visible_rows();
        if self.current >= rows {
            self.current + 1 - rows
        } else {
            0
        }
    }

    fn elapsed_secs(&self) -> u32 {
        match &self.song {
            Some(s) => (s.position() / PLAYER_SAMPLE_RATE as usize) as u32,
            None => 0,
        }
    }

    fn total_secs(&self) -> u32 {
        match player::song(self.current) {
            Some((_, data)) => (data.len() / PLAYER_SAMPLE_RATE as usize) as u32,
            None => 0,
        }
    }

    fn start(&mut self, index: usize) {
        let Some((_, data)) = player::song(index) else {
            return;
        };
        if data.is_empty() {
            return;
        }
        self.current = index;
        self.failed = false;
        self.last_sec = 0;

        let mut s = SongPlayer::new(data);
        if sound::stream_start(&mut s, PLAYER_SAMPLE_RATE) {
            self.song = Some(s);
            self.state = State::Playing;
        } else {
            sound::stop();
            self.song = None;
            self.state = State::Stopped;
            self.failed = true;
        }
    }

    fn toggle(&mut self) {
        match self.state {
            State::Playing => {
                sound::stop();
                self.state = State::Paused;
            }
            State::Paused => {
                let ok = match self.song.as_mut() {
                    Some(s) => sound::stream_start(s, PLAYER_SAMPLE_RATE),
                    None => false,
                };
                if ok {
                    self.state = State::Playing;
                } else {
                    self.song = None;
                    self.state = State::Stopped;
                    self.failed = true;
                }
            }
            State::Stopped => self.start(self.current),
        }
    }

    fn stop(&mut self) {
        if self.state == State::Playing {
            sound::stop();
        }
        self.song = None;
        self.state = State::Stopped;
        self.last_sec = 0;
    }

    fn step(&mut self, forward: bool) {
        let n = player::count();
        if n == 0 {
            return;
        }
        let idx = if forward {
            (self.current + 1) % n
        } else {
            (self.current + n - 1) % n
        };
        self.start(idx);
    }

    fn seek_to(&mut self, mx: i32) {
        let Some(s) = self.song.as_mut() else {
            return;
        };
        let rel = (mx - self.bar.x).clamp(0, self.bar.w) as usize;
        let target = s.len() * rel / self.bar.w as usize;
        s.seek(target);
        if self.state == State::Playing && !sound::stream_start(s, PLAYER_SAMPLE_RATE) {
            self.song = None;
            self.state = State::Stopped;
            self.failed = true;
        }
    }

    pub fn pump(&mut self) {
        if self.state != State::Playing {
            return;
        }
        let now = crate::time::ticks_since_boot();
        if now < self.next_pump {
            return;
        }
        self.next_pump = now + crate::time::ticks_per_ms() * PUMP_INTERVAL_MS;
        let mut done = false;
        if let Some(s) = self.song.as_mut() {
            sound::stream_tick(s);
            done = s.finished() && s.drained();
        }
        if done {
            self.step(true);
            self.dirty = true;
            self.full = true;
        }
    }

    pub fn update(&mut self) -> bool {
        let sec = self.elapsed_secs();
        let changed = self.dirty || sec != self.last_sec;
        self.last_sec = sec;
        self.dirty = false;
        changed
    }

    pub fn click(&mut self, mx: i32, my: i32) -> bool {
        self.full = true;
        if self.btn_toggle.contains(mx, my) {
            self.toggle();
            return true;
        }
        if self.btn_stop.contains(mx, my) {
            self.stop();
            return true;
        }
        if self.btn_prev.contains(mx, my) {
            self.step(false);
            return true;
        }
        if self.btn_next.contains(mx, my) {
            self.step(true);
            return true;
        }
        if self.bar_hit.contains(mx, my) {
            self.seek_to(mx);
            return true;
        }

        let list_y = self.cy as i32 + LIST_TOP;
        let list_x = self.cx as i32 + 10;
        let list_w = self.cw as i32 - 20;
        if my >= list_y && mx >= list_x && mx < list_x + list_w {
            let row = ((my - list_y) / ROW_H) as usize;
            if row < self.visible_rows() {
                let idx = self.first_visible() + row;
                if idx < player::count() {
                    self.start(idx);
                    return true;
                }
            }
        }
        false
    }

    pub fn key(&mut self, key: u8) -> bool {
        self.full = true;
        match key {
            b' ' => {
                self.toggle();
                true
            }
            KEY_LEFT => {
                self.step(false);
                true
            }
            KEY_RIGHT => {
                self.step(true);
                true
            }
            _ => false,
        }
    }

    pub fn redraw(&mut self) {
        if self.full {
            self.full = false;
            self.draw_all();
        } else {
            self.draw_dynamic();
        }
    }

    fn draw_dynamic(&self) {
        let (x, y, w) = (self.cx as i32, self.cy as i32, self.cw as i32);
        framebuffer::fill_rect(
            (x + 12) as usize,
            (y + 42) as usize,
            (w - 24) as usize,
            12,
            style::SURFACE_DARK,
        );
        self.draw_status(x, y, w);
        self.draw_bar();
    }

    fn draw_status(&self, x: i32, y: i32, w: i32) {
        let (label, color) = if self.failed {
            ("audio buffer unavailable", style::DANGER)
        } else {
            match self.state {
                State::Playing => ("playing", style::GOOD),
                State::Paused => ("paused", style::WARN),
                State::Stopped => ("stopped", style::TEXT_DIM),
            }
        };
        style::text(label, x + 20, y + 44, color);

        let time = format!(
            "{:02}:{:02} / {:02}:{:02}",
            self.elapsed_secs() / 60,
            self.elapsed_secs() % 60,
            self.total_secs() / 60,
            self.total_secs() % 60
        );
        style::text(&time, x + w - 20 - style::text_width(&time), y + 44, style::TEXT);
    }

    fn draw_bar(&self) {
        framebuffer::fill_rect(
            self.bar.x as usize,
            self.bar.y as usize,
            self.bar.w as usize,
            self.bar.h as usize,
            style::SURFACE_DARK,
        );
        if let Some(s) = &self.song {
            if s.len() > 0 {
                let filled = (self.bar.w as usize * s.position() / s.len()).min(self.bar.w as usize);
                framebuffer::fill_rect(
                    self.bar.x as usize,
                    self.bar.y as usize,
                    filled,
                    self.bar.h as usize,
                    style::ACCENT,
                );
            }
        }
    }

    fn draw_all(&self) {
        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, style::WINDOW_BG);

        let (x, y, w) = (self.cx as i32, self.cy as i32, self.cw as i32);

        style::panel(x + 10, y + 8, (w - 20) as u32, HEADER_H as u32);
        match player::song(self.current) {
            Some((name, _)) => {
                let max = ((w - 40) / 16) as usize;
                let title: String = name
                    .chars()
                    .map(|c| if c == '_' { ' ' } else { c })
                    .take(max)
                    .collect();
                draw_text_scaled(&title, (x + 20) as usize, (y + 14) as usize, style::TEXT, 2);
            }
            None => {
                style::text("no songs in src/apps/music", x + 20, y + 24, style::TEXT_DIM);
            }
        }

        self.draw_status(x, y, w);
        self.draw_bar();

        let toggle_label = if self.state == State::Playing { "pause" } else { "play" };
        for (rect, text) in [
            (self.btn_prev, "prev"),
            (self.btn_toggle, toggle_label),
            (self.btn_stop, "stop"),
            (self.btn_next, "next"),
        ] {
            style::button(rect.x, rect.y, rect.w as u32, rect.h as u32, text);
        }

        let count = player::count();
        let first = self.first_visible();
        let rows = self.visible_rows();
        for i in first..count.min(first + rows) {
            let ry = y + LIST_TOP + (i - first) as i32 * ROW_H;
            let selected = i == self.current;
            if selected {
                style::rounded_fill(x + 10, ry, (w - 20) as u32, (ROW_H - 2) as u32, style::SURFACE);
            }
            let Some((name, _)) = player::song(i) else {
                continue;
            };
            let entry = format!("{:>2}  {}", i + 1, name.replace('_', " "));
            let text_color = if selected { style::ACCENT } else { style::TEXT };
            style::text(&entry, x + 18, ry + 3, text_color);
        }
    }
}

impl Drop for MusicPlayer {
    fn drop(&mut self) {
        if self.state == State::Playing {
            sound::stop();
        }
    }
}
