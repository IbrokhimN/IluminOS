// window manager draws title bar and close button and defines widget trait

use crate::framebuffer;
use crate::gui::style;
use alloc::string::String;

// geometry

// rectangular area for hit test
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        style::hit(px, py, self.x, self.y, self.w as u32, self.h as u32)
    }
}

// widget contract

pub trait Widget {
    // draw window contents
    fn draw(&mut self);

    // left click inside window
    fn on_click(&mut self, _x: i32, _y: i32) -> bool {
        false
    }

    // key press while window is focused
    fn on_key(&mut self, _key: u8) -> bool {
        false
    }

    // mouse movement while dragging
    fn on_drag(&mut self, _x: i32, _y: i32) {}

    // periodic update tick
    fn tick(&mut self) -> bool {
        false
    }
}

// window frame

const TITLE_H: i32 = 22;
const CLOSE_SIZE: i32 = 14;

// on screen geometry of a window
#[derive(Clone, Copy)]
pub struct WindowGeom {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl WindowGeom {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        WindowGeom { x, y, w, h }
    }

    // content area below title bar
    pub fn content_area(&self) -> Rect {
        let pad = 6;
        Rect::new(
            (self.x as i32) + pad,
            (self.y as i32) + TITLE_H + pad,
            (self.w as i32) - pad * 2,
            (self.h as i32) - TITLE_H - pad * 2,
        )
    }

    // close button area
    pub fn close_button(&self) -> Rect {
        let cy = self.y as i32 + (TITLE_H - CLOSE_SIZE) / 2;
        Rect::new(self.x as i32 + self.w as i32 - CLOSE_SIZE - 6, cy, CLOSE_SIZE, CLOSE_SIZE)
    }
}

// draw window body title bar and close button
pub fn draw_frame(g: WindowGeom, title: &str) {
    let (x, y, w, h) = (g.x as i32, g.y as i32, g.w as u32, g.h as u32);

    style::rounded_fill(x, y, w, h, style::WINDOW_BG);
    style::rounded_outline(x, y, w, h, style::BORDER);

    // title bar strip
    framebuffer::fill_rect((x + 2) as usize, (y + 2) as usize, (w - 4) as usize, TITLE_H as usize - 2, style::TITLEBAR_BG);
    style::text(title, x + 8, y + (TITLE_H - 10) / 2, style::TEXT);

    // close button circle
    let cb = g.close_button();
    style::rounded_fill(cb.x, cb.y, cb.w as u32, cb.h as u32, style::DANGER);
    style::text_centered("x", cb.x, cb.y, cb.w as u32, cb.h as u32, style::WINDOW_BG);
}

pub fn close_hit(g: WindowGeom, x: i32, y: i32) -> bool {
    g.close_button().contains(x, y)
}

// routing

pub fn route_click(widget: &mut dyn Widget, x: i32, y: i32) -> bool {
    widget.on_click(x, y)
}

pub fn route_key(widget: &mut dyn Widget, key: u8) -> bool {
    widget.on_key(key)
}

pub fn route_drag(widget: &mut dyn Widget, x: i32, y: i32) {
    widget.on_drag(x, y);
}

pub fn route_tick(widget: &mut dyn Widget) -> bool {
    widget.tick()
}

pub fn route_draw(widget: &mut dyn Widget) {
    widget.draw();
}

// app adapters

use crate::gui::widgets::apps::{Browser, Calc, Clock, Files, Paint, Term};
use crate::keyboard::{KEY_BACKSPACE, KEY_DOWN, KEY_ENTER, KEY_UP};

impl Widget for Calc {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
        self.click(x, y)
    }
}

impl Widget for Clock {
    fn draw(&mut self) {
        self.redraw();
    }

    fn tick(&mut self) -> bool {
        self.update();
        true
    }
}

impl Widget for Paint {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
        // paint on click redraws toolbar only
        Paint::on_click(self, x, y);
        false
    }

    fn on_drag(&mut self, x: i32, y: i32) {
        Paint::on_drag(self, x, y);
    }
}

impl Widget for Term {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_key(&mut self, key: u8) -> bool {
        match key {
            KEY_ENTER => {
                let input = self.input.clone();
                self.input.clear();
                self.exec(&input);
                true
            }
            KEY_BACKSPACE => {
                self.input.pop();
                true
            }
            0x20..=0x7e | 0x80..=0xff => {
                self.input.push(key as char);
                true
            }
            _ => false,
        }
    }
}

impl Widget for Browser {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
        if self.search_btn_hit(x, y) {
            self.do_search();
            return true;
        }
        if self.viewing_html {
            if let Some(href) = self.link_at(x, y) {
                self.navigate_link(&href);
                return true;
            }
        }
        false
    }

    fn on_key(&mut self, key: u8) -> bool {
        match key {
            KEY_ENTER => {
                self.do_search();
                true
            }
            KEY_BACKSPACE => {
                self.query.pop();
                true
            }
            KEY_UP if self.viewing_html => {
                self.scroll(-24);
                true
            }
            KEY_DOWN if self.viewing_html => {
                self.scroll(24);
                true
            }
            0x20..=0x7e | 0x80..=0xff => {
                self.query.push(key as char);
                true
            }
            _ => false,
        }
    }
}

impl Widget for Files {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
        Files::on_click(self, x, y)
    }

    fn on_key(&mut self, key: u8) -> bool {
        Files::on_key(self, key)
    }
}

// reusable UI controls

pub struct Button {
    pub area: Rect,
    pub text: &'static str,
}

impl Button {
    pub fn new(text: &'static str, area: Rect) -> Self {
        Button { text, area }
    }

    pub fn draw(&self) {
        style::button(self.area.x, self.area.y, self.area.w as u32, self.area.h as u32, self.text);
    }

    pub fn hit(&self, mx: i32, my: i32) -> bool {
        self.area.contains(mx, my)
    }
}

pub struct Label {
    pub x: i32,
    pub y: i32,
    pub text: &'static str,
    pub color: u32,
}

impl Label {
    pub fn new(x: i32, y: i32, text: &'static str, color: u32) -> Self {
        Label { x, y, text, color }
    }

    pub fn draw(&self) {
        style::text(self.text, self.x, self.y, self.color);
    }
}

pub struct TextField {
    pub area: Rect,
    pub text: String,
    pub max_len: usize,
    pub focused: bool,
}

impl TextField {
    pub fn new(area: Rect, max_len: usize) -> Self {
        TextField { area, text: String::new(), max_len, focused: true }
    }

    // handle keypress
    pub fn key(&mut self, key: u8) -> bool {
        match key {
            0x08 => {
                self.text.pop();
                true
            }
            0x20..=0x7e | 0x80..=0xff if self.text.len() < self.max_len => {
                self.text.push(key as char);
                true
            }
            _ => false,
        }
    }

    pub fn draw(&self) {
        let (x, y, w, h) = (self.area.x, self.area.y, self.area.w as u32, self.area.h as u32);
        style::panel(x, y, w, h);
        style::text(&self.text, x + 6, y + (self.area.h - 10) / 2, style::TEXT);
        if self.focused {
            let cursor_x = x + 6 + style::text_width(&self.text);
            style::rounded_fill(cursor_x, y + 4, 2, (h as i32 - 8).max(0) as u32, style::TEXT);
        }
    }

    pub fn hit(&self, mx: i32, my: i32) -> bool {
        self.area.contains(mx, my)
    }
}
