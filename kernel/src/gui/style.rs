// shared look for windowed gui based on embedded graphics primitives

use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    prelude::*,
    primitives::{CornerRadii, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

use crate::framebuffer::{self, Display, u32_to_rgb888};
use spin::Once;

// palette

pub const DESKTOP_BG: u32 = 0x232A2E;
pub const WINDOW_BG: u32 = 0x2B3339;
pub const TITLEBAR_BG: u32 = 0x3A4750;
pub const SURFACE: u32 = 0x333B40;
pub const SURFACE_DARK: u32 = 0x232A2E;
pub const BORDER: u32 = 0x1B2124;

pub const TEXT: u32 = 0xD3C6AA;
pub const TEXT_DIM: u32 = 0x8A9591;

pub const ACCENT: u32 = 0x7FBBB3;
pub const GOOD: u32 = 0xA7C080;
pub const WARN: u32 = 0xDBBC7F;
pub const DANGER: u32 = 0xE67E80;

const CORNER_RADIUS: u32 = 6;

// primitives

// filled rounded rectangle
pub fn rounded_fill(x: i32, y: i32, w: u32, h: u32, color: u32) {
    let rect = Rectangle::new(Point::new(x, y), Size::new(w, h));
    let rounded = RoundedRectangle::new(
        rect,
        CornerRadii::new(Size::new(CORNER_RADIUS, CORNER_RADIUS)),
    );
    let _ = rounded
        .into_styled(PrimitiveStyle::with_fill(u32_to_rgb888(color)))
        .draw(&mut Display);
}

// rounded rectangle outline
pub fn rounded_outline(x: i32, y: i32, w: u32, h: u32, color: u32) {
    let rect = Rectangle::new(Point::new(x, y), Size::new(w, h));
    let rounded = RoundedRectangle::new(
        rect,
        CornerRadii::new(Size::new(CORNER_RADIUS, CORNER_RADIUS)),
    );
    let _ = rounded
        .into_styled(PrimitiveStyle::with_stroke(u32_to_rgb888(color), 1))
        .draw(&mut Display);
}

// flat panel with border
pub fn panel(x: i32, y: i32, w: u32, h: u32) {
    rounded_fill(x, y, w, h, SURFACE_DARK);
    rounded_outline(x, y, w, h, BORDER);
}

// button with centered text
pub fn button(x: i32, y: i32, w: u32, h: u32, label: &str) {
    rounded_fill(x, y, w, h, SURFACE);
    rounded_outline(x, y, w, h, BORDER);
    text_centered(label, x, y, w, h, TEXT);
}

// hit test for rectangle area
pub fn hit(px: i32, py: i32, x: i32, y: i32, w: u32, h: u32) -> bool {
    px >= x && px < x + w as i32 && py >= y && py < y + h as i32
}

// text

// draw text with font 6x10
pub fn text(s: &str, x: i32, y: i32, color: u32) {
    let style = MonoTextStyle::new(&FONT_6X10, u32_to_rgb888(color));
    let _ = Text::new(s, Point::new(x, y + 8), style).draw(&mut Display);
}

// centered text in box
pub fn text_centered(s: &str, x: i32, y: i32, w: u32, h: u32, color: u32) {
    let text_w = s.len() as i32 * 6;
    let tx = x + (w as i32 - text_w).max(0) / 2;
    let ty = y + (h as i32 - 10).max(0) / 2;
    text(s, tx, ty, color);
}

// text width in pixels
pub fn text_width(s: &str) -> i32 {
    s.len() as i32 * 6
}

// icons

use tinybmp::Bmp;

pub struct Icon(&'static [u8]);

pub const ICON_TERMINAL: Icon = Icon(include_bytes!("icons/terminal.bmp"));
pub const ICON_BROWSER: Icon = Icon(include_bytes!("icons/browser.bmp"));
pub const ICON_CLOCK: Icon = Icon(include_bytes!("icons/clock.bmp"));
pub const ICON_CALC: Icon = Icon(include_bytes!("icons/calc.bmp"));
pub const ICON_PAINT: Icon = Icon(include_bytes!("icons/paint.bmp"));

impl Icon {
    // draw icon at x y
    pub fn draw(&self, x: i32, y: i32) {
        match Bmp::<embedded_graphics::pixelcolor::Rgb888>::from_slice(self.0) {
            Ok(bmp) => {
                let _ =
                    embedded_graphics::image::Image::new(&bmp, Point::new(x, y)).draw(&mut Display);
            }
            Err(_) => {
                framebuffer::fill_rect(x as usize, y as usize, 28, 28, ACCENT);
            }
        }
    }
}

// wallpaper

use embedded_graphics::geometry::Size;

const WALLPAPER_BYTES: &[u8] = include_bytes!("wallpapers/wallpaper.bmp");

static WALLPAPER_BMP: Once<Option<Bmp<'static, embedded_graphics::pixelcolor::Rgb888>>> =
    Once::new();

pub fn init_wallpaper() {
    WALLPAPER_BMP.call_once(|| {
        Bmp::<embedded_graphics::pixelcolor::Rgb888>::from_slice(WALLPAPER_BYTES).ok()
    });
}

pub fn draw_wallpaper() {
    let (screen_w, screen_h) = framebuffer::dimensions();

    let bmp_option = WALLPAPER_BMP.call_once(|| {
        Bmp::<embedded_graphics::pixelcolor::Rgb888>::from_slice(WALLPAPER_BYTES).ok()
    });

    if let Some(bmp) = bmp_option {
        let size = bmp.size();

        if size.width < screen_w as u32 || size.height < screen_h as u32 {
            framebuffer::fill_rect(0, 0, screen_w, screen_h, DESKTOP_BG);
        }

        let _ = embedded_graphics::image::Image::new(bmp, Point::zero()).draw(&mut Display);
    } else {
        framebuffer::fill_rect(0, 0, screen_w, screen_h, DESKTOP_BG);
    }
}
