use crate::framebuffer::{self, fill_rect, draw_text_at, draw_text_scaled, dimensions};
use crate::keyboard::{self, KEY_ENTER, KEY_BACKSPACE, KEY_TAB};
use crate::sound;
use alloc::string::String;

const USER: &str = "root";
const PASS: &str = "iluminos";

const BG_TOP: u32 = 0x0A0A18;      // top background
const BG_BOT: u32 = 0x1A1030;      // bottom background
const CARD_BG: u32 = 0x15152A;     // card background
const CARD_BORDER: u32 = 0x4444AA; // card border
const FIELD_BG: u32 = 0x0E0E1E;    // input field background
const FIELD_ACTIVE: u32 = 0x5566FF;// active field border
const FIELD_IDLE: u32 = 0x333355;  // idle field border
const ACCENT: u32 = 0x66DDFF;      // accent color
const TITLE_C: u32 = 0x88AAFF;     // title color
const LABEL_C: u32 = 0x8888AA;     // field label color
const TEXT_C: u32 = 0xDDDDEE;      // typed text color
const HINT_C: u32 = 0x555577;      // bottom hint color
const ERR_C: u32 = 0xFF5566;       // error color

// which field is active
#[derive(PartialEq, Clone, Copy)]
enum Field { User, Pass }

// show login return only on correct password
pub fn run() {
    let mut username = String::new();
    let mut password = String::new();
    let mut field = Field::User;
    let mut error = false;

    // draw static background card and title once to avoid flicker
    draw_static();
    draw_dynamic(&username, &password, field, error);

    loop {
        let key = keyboard::read_key();

        match key {
            KEY_ENTER => {
                if field == Field::User {
                    field = Field::Pass;
                } else {
                    if username == USER && password == PASS {
                        success_animation();
                        return;
                    } else {
                        // wrong password shake the card
                        error = true;
                        password.clear();
                        sound::beep(120, 1);
                        for i in 0..6 {
                            let shake = if i % 2 == 0 { 6 } else { -6 };
                            draw_shake(&username, &password, field, error, shake);
                            sound::delay(1);
                        }
                        // redraw static after the shake
                        draw_static();
                    }
                }
            }
            KEY_TAB => {
                field = if field == Field::User { Field::Pass } else { Field::User };
                error = false;
            }
            KEY_BACKSPACE => {
                match field {
                    Field::User => { username.pop(); }
                    Field::Pass => { password.pop(); }
                }
                error = false;
            }
            0x20..=0x7e | 0x80..=0xff => {
                match field {
                    Field::User => if username.len() < 20 { username.push(key as char); }
                    Field::Pass => if password.len() < 20 { password.push(key as char); }
                }
                error = false;
            }
            _ => {}
        }

        // redraw only fields and error on each key not the whole screen
        draw_dynamic(&username, &password, field, error);
    }
}

// card geometry shared by static and dynamic draws
fn card_geom() -> (usize, usize, usize, usize) {
    let (w, h) = dimensions();
    let card_w = 420usize;
    let card_h = 240usize;
    ((w - card_w) / 2, (h - card_h) / 2, card_w, card_h)
}

// background card title and labels drawn once
fn draw_static() {
    let (w, h) = dimensions();
    draw_gradient(w, h);

    let (card_x, card_y, card_w, card_h) = card_geom();

    // shadow body and border of the card
    fill_rect(card_x + 6, card_y + 6, card_w, card_h, 0x05050A);
    fill_rect(card_x, card_y, card_w, card_h, CARD_BG);
    draw_border(card_x, card_y, card_w, card_h, 2, CARD_BORDER);

    // title and subtitle
    let title = "IluminOS";
    let title_w = title.len() * 8 * 2;
    draw_text_scaled(title, card_x + (card_w - title_w) / 2, card_y + 24, TITLE_C, 2);
    let sub = "please sign in";
    draw_text_at(sub, card_x + (card_w - sub.len() * 8) / 2, card_y + 52, LABEL_C);

    fill_rect(card_x + 30, card_y + 74, card_w - 60, 1, CARD_BORDER);

    let fx = card_x + 40;
    draw_text_at("USERNAME", fx, card_y + 90, LABEL_C);
    draw_text_at("PASSWORD", fx, card_y + 140, LABEL_C);

    // bottom hints
    let hint = "Tab: switch field    Enter: confirm";
    draw_text_at(hint, (w - hint.len() * 8) / 2, h - 40, HINT_C);
    let demo = "demo: root / iluminos";
    draw_text_at(demo, (w - demo.len() * 8) / 2, h - 24, 0x666688);
}

// redraw only the changing parts fields and error message
fn draw_dynamic(username: &str, password: &str, field: Field, error: bool) {
    let (card_x, card_y, card_w, _card_h) = card_geom();
    let fx = card_x + 40;
    let fw = card_w - 80;

    // fields clear their own background so old text disappears
    draw_field(fx, card_y + 104, fw, username, false, field == Field::User);
    draw_field(fx, card_y + 154, fw, password, true, field == Field::Pass);

    // clear error area then draw message if any
    fill_rect(card_x + 20, card_y + 190, card_w - 40, 12, CARD_BG);
    if error {
        let msg = "invalid credentials";
        draw_text_at(msg, card_x + (card_w - msg.len() * 8) / 2, card_y + 194, ERR_C);
    }
}

// full redraw with card offset for shake animation
fn draw_shake(username: &str, password: &str, field: Field, error: bool, shake: i32) {
    let (w, h) = dimensions();
    draw_gradient(w, h);
    let (base_x, card_y, card_w, card_h) = card_geom();
    let card_x = base_x.wrapping_add(shake as usize);

    fill_rect(card_x + 6, card_y + 6, card_w, card_h, 0x05050A);
    fill_rect(card_x, card_y, card_w, card_h, CARD_BG);
    draw_border(card_x, card_y, card_w, card_h, 2, CARD_BORDER);
    let title = "IluminOS";
    let title_w = title.len() * 8 * 2;
    draw_text_scaled(title, card_x + (card_w - title_w) / 2, card_y + 24, TITLE_C, 2);
    fill_rect(card_x + 30, card_y + 74, card_w - 60, 1, CARD_BORDER);
    let fx = card_x + 40;
    let fw = card_w - 80;
    draw_text_at("USERNAME", fx, card_y + 90, LABEL_C);
    draw_text_at("PASSWORD", fx, card_y + 140, LABEL_C);
    draw_field(fx, card_y + 104, fw, username, false, field == Field::User);
    draw_field(fx, card_y + 154, fw, password, true, field == Field::Pass);
    if error {
        let msg = "invalid credentials";
        draw_text_at(msg, card_x + (card_w - msg.len() * 8) / 2, card_y + 194, ERR_C);
    }
}

// draw input field border text or dots for password and cursor
fn draw_field(x: usize, y: usize, w: usize, value: &str, secret: bool, active: bool) {
    let fh = 24usize;
    // field background
    fill_rect(x, y, w, fh, FIELD_BG);
    // brighter border when active
    let border = if active { FIELD_ACTIVE } else { FIELD_IDLE };
    draw_border(x, y, w, fh, if active { 2 } else { 1 }, border);

    // content password shown as dots
    let tx = x + 10;
    let ty = y + 8;
    if secret {
        // draw a dot per password character
        let mut px = tx;
        for _ in 0..value.len() {
            fill_rect(px, ty + 2, 5, 5, TEXT_C); // dot square
            px += 12;
        }
        // cursor after the dots if active
        if active {
            fill_rect(px, ty - 1, 2, 10, ACCENT);
        }
    } else {
        draw_text_at(value, tx, ty, TEXT_C);
        // cursor after the text
        if active {
            fill_rect(tx + value.len() * 8, ty - 1, 2, 10, ACCENT);
        }
    }
}

// rectangle border of thickness t
fn draw_border(x: usize, y: usize, w: usize, h: usize, t: usize, color: u32) {
    fill_rect(x, y, w, t, color);              // top
    fill_rect(x, y + h - t, w, t, color);      // bottom
    fill_rect(x, y, t, h, color);              // left
    fill_rect(x + w - t, y, t, h, color);      // right
}

// vertical background gradient from top to bottom color
fn draw_gradient(w: usize, h: usize) {
    let bands = h / 4;
    // split colors into channels for interpolation
    let (r1, g1, b1) = ((BG_TOP >> 16) & 0xFF, (BG_TOP >> 8) & 0xFF, BG_TOP & 0xFF);
    let (r2, g2, b2) = ((BG_BOT >> 16) & 0xFF, (BG_BOT >> 8) & 0xFF, BG_BOT & 0xFF);
    for band in 0..bands {
        let t = ((band * 255) / bands.max(1)) as u32; // 0 to 255 across height
        let r = r1 + (r2 - r1) * t / 255;
        let g = g1 + (g2 - g1) * t / 255;
        let b = b1 + (b2 - b1) * t / 255;
        let color = (r << 16) | (g << 8) | b;
        fill_rect(0, band * 4, w, 4, color);
    }
}

// success chime on correct login
fn success_animation() {
    // rising notes for success
    framebuffer::clear();
}
