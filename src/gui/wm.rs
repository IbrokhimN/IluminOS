// window manager framework widgets implement one trait the manager stays agnostic

use crate::framebuffer::{fill_rect, draw_rect, draw_text_at};
use crate::gui::widgets::apps::{Calc, Clock, Paint};

// rectangle area with a hit test helper

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

    // right and bottom edges are exclusive
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

// contract every app window implements only draw is required the rest default to no-op

pub trait Widget {
    // draw window contents manager already drew the frame and title
    fn draw(&mut self);

    // left click inside the window returns true if a redraw is needed
    fn on_click(&mut self, _x: i32, _y: i32) -> bool {
        false
    }

    // key press while focused returns true if a redraw is needed
    fn on_key(&mut self, _key: u8) -> bool {
        false
    }

    // mouse drag with left button held used by paint
    fn on_drag(&mut self, _x: i32, _y: i32) {}

    // periodic tick for windows that change on their own like the clock
    fn tick(&mut self) -> bool {
        false
    }
}

// shared drawing primitives for all windows

// windows 3.1 style colors
pub const WIN_FACE: u32 = 0xC0C0C0;  // gray surface
pub const WIN_LIGHT: u32 = 0xFFFFFF; // light bevel edge
pub const WIN_DARK: u32 = 0x808080;  // dark bevel edge
pub const BLACK: u32 = 0x000000;

// beveled edge raised true for normal false for pressed
pub fn bevel(x: usize, y: usize, w: usize, h: usize, raised: bool) {
    let (tl, br) = if raised { (WIN_LIGHT, WIN_DARK) } else { (WIN_DARK, WIN_LIGHT) };
    fill_rect(x, y, w, 2, tl);          // top edge
    fill_rect(x, y, 2, h, tl);          // left edge
    fill_rect(x, y + h - 2, w, 2, br);  // bottom edge
    fill_rect(x + w - 2, y, 2, h, br);  // right edge
}

// panel a filled rect with a beveled border
pub fn panel(r: Rect) {
    fill_rect(r.x as usize, r.y as usize, r.w as usize, r.h as usize, WIN_FACE);
    bevel(r.x as usize, r.y as usize, r.w as usize, r.h as usize, true);
}

// button with centered label click detection is separate via rect contains
pub fn button(r: Rect, label: &str) {
    fill_rect(r.x as usize, r.y as usize, r.w as usize, r.h as usize, WIN_FACE);
    bevel(r.x as usize, r.y as usize, r.w as usize, r.h as usize, true);
    // label roughly centered 8px font
    let tx = r.x as usize + (r.w as usize).saturating_sub(label.len() * 8) / 2;
    let ty = r.y as usize + (r.h as usize).saturating_sub(8) / 2;
    draw_text_at(label, tx, ty, BLACK);
}

// plain text label
pub fn label(x: i32, y: i32, text: &str, color: u32) {
    draw_text_at(text, x as usize, y as usize, color);
}

// unfilled border for things like input fields
pub fn outline(r: Rect, color: u32) {
    draw_rect(r.x as usize, r.y as usize, r.w as usize, r.h as usize, color);
}

// calc adapter wraps its existing redraw and click methods as a widget

impl Widget for Calc {
    fn draw(&mut self) {
        // redraw takes self by ref which is fine with the trait's mut ref
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
        // click already returns whether to redraw
        self.click(x, y)
    }

    // key drag and tick unused default no-op behavior applies
}

// clock adapter updates itself via tick no clicks or keys

impl Widget for Clock {
    fn draw(&mut self) {
        self.redraw();
    }

    // update recalculates state return true so the manager redraws
    fn tick(&mut self) -> bool {
        self.update();
        true
    }
}

// paint adapter handles both clicks for the palette and drag for the brush

impl Widget for Paint {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
        // call paint's own method not the trait to avoid recursion
        let _ = Paint::on_click(self, x, y);
        false
    }

    fn on_drag(&mut self, x: i32, y: i32) {
        Paint::on_drag(self, x, y);
    }
}

// window manager draws the shared frame title and close button around one active window

// on screen window geometry
#[derive(Clone, Copy)]
pub struct WindowGeom {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

const TITLE_BG: u32 = 0x000080;  // title bar background
const TITLE_FG: u32 = 0xFFFFFF;  // title text color
const TITLE_H: usize = 18;       // title bar height

impl WindowGeom {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        WindowGeom { x, y, w, h }
    }

    // content area below the title where the widget draws
    pub fn content_area(&self) -> Rect {
        Rect::new(
            (self.x + 4) as i32,
            (self.y + 3 + TITLE_H + 2) as i32,
            (self.w - 8) as i32,
            (self.h - (3 + TITLE_H + 2) - 4) as i32,
        )
    }

    // close button rect in the top right corner
    pub fn close_button(&self) -> Rect {
        Rect::new((self.x + self.w - 20) as i32, (self.y + 5) as i32, 14, 14)
    }
}

// draw window frame body bevel title and close button
pub fn draw_frame(g: WindowGeom, title: &str) {
    panel(Rect::new(g.x as i32, g.y as i32, g.w as i32, g.h as i32));
    draw_rect(g.x, g.y, g.w, g.h, BLACK); // black outline

    // blue title bar
    fill_rect(g.x + 3, g.y + 3, g.w - 6, TITLE_H, TITLE_BG);
    draw_text_at(title, g.x + 7, g.y + 3 + 5, TITLE_FG);

    // close button
    let cb = g.close_button();
    button(cb, "x");
}

// check if click hit the close button
pub fn close_hit(g: WindowGeom, x: i32, y: i32) -> bool {
    g.close_button().contains(x, y)
}

// route events to the active widget return whether to redraw
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

// terminal adapter keyboard only typed chars accumulate enter runs the command

use crate::gui::widgets::apps::{Term, Browser};
use crate::keyboard::{KEY_ENTER, KEY_BACKSPACE};

impl Widget for Term {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_key(&mut self, key: u8) -> bool {
        match key {
            KEY_ENTER => {
                // run the typed command and clear input
                let inp = self.input.clone();
                self.input.clear();
                self.exec(&inp);
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

// browser adapter takes keys for the query and a click on search enter also searches

impl Widget for Browser {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
        // clicking search runs the search
        if self.search_btn_hit(x, y) {
            self.do_search();
            true
        } else {
            false
        }
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
            0x20..=0x7e | 0x80..=0xff => {
                self.query.push(key as char);
                true
            }
            _ => false,
        }
    }
}

// reusable ui widget structs each stores its own area and content

// default text colors
const TEXT_DARK: u32 = 0x000000;

// button clickable with a label

pub struct Button {
    pub area: Rect,          // where the button sits
    pub text: &'static str,  // label
}

impl Button {
    pub fn new(text: &'static str, area: Rect) -> Self {
        Button { text, area }
    }

    // draw beveled button with centered label
    pub fn draw(&self) {
        button(self.area, self.text);
    }

    // whether this button was clicked
    pub fn hit(&self, mx: i32, my: i32) -> bool {
        self.area.contains(mx, my)
    }
}

// label plain text at a position

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
        label(self.x, self.y, self.text, self.color);
    }
}

// textfield single line input owns its own text state

pub struct TextField {
    pub area: Rect,
    pub text: alloc::string::String,
    pub max_len: usize,
    pub focused: bool, // whether to draw the cursor
}

impl TextField {
    pub fn new(area: Rect, max_len: usize) -> Self {
        TextField {
            area,
            text: alloc::string::String::new(),
            max_len,
            focused: true,
        }
    }

    // handle a keypress return true if content changed
    pub fn key(&mut self, key: u8) -> bool {
        match key {
            0x08 => {
                // backspace
                self.text.pop();
                true
            }
            0x20..=0x7e | 0x80..=0xff => {
                if self.text.len() < self.max_len {
                    self.text.push(key as char);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    // draw background border text and cursor
    pub fn draw(&self) {
        let x = self.area.x as usize;
        let y = self.area.y as usize;
        let w = self.area.w as usize;
        let h = self.area.h as usize;
        fill_rect(x, y, w, h, 0xFFFFFF);      // white background
        outline(self.area, WIN_DARK);         // gray border
        draw_text_at(&self.text, x + 4, y + (h.saturating_sub(8)) / 2, TEXT_DARK);
        // cursor a vertical bar after the text
        if self.focused {
            let cx = x + 4 + self.text.len() * 8;
            fill_rect(cx, y + 4, 2, h.saturating_sub(8), TEXT_DARK);
        }
    }

    // whether click landed in the field to give it focus
    pub fn hit(&self, mx: i32, my: i32) -> bool {
        self.area.contains(mx, my)
    }
}
