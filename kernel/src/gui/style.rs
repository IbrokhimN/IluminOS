// One shared look for the whole windowed GUI (window manager, desktop,
// widgets). Everything here is built on embedded-graphics primitives.
//
// Before this file existed, `bevel()` and the Windows-3.1-style gray/blue
// palette were copy-pasted three times (wm.rs, desktop.rs, widgets/apps.rs)
// with slightly different colors each time. Now there is one palette and one
// set of drawing helpers that every screen uses, so changing "how a button
// looks" means editing one function instead of three.

use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    prelude::*,
    primitives::{CornerRadii, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

use crate::framebuffer::{self, Display, u32_to_rgb888};
use spin::Once;
// --- palette -----------------------------------------------------------
// A dark, muted theme (derived from the Everforest palette already used by
// the text console) instead of the old bright teal desktop / gray Windows
// chrome, so the GUI doesn't hurt to look at.

pub const DESKTOP_BG: u32 = 0x232A2E; // behind the icons
pub const WINDOW_BG: u32 = 0x2B3339; // window body
pub const TITLEBAR_BG: u32 = 0x3A4750; // window title bar
pub const SURFACE: u32 = 0x333B40; // raised surfaces: buttons, icon slots
pub const SURFACE_DARK: u32 = 0x232A2E; // sunken surfaces: text fields, screens
pub const BORDER: u32 = 0x1B2124; // outlines / separators

pub const TEXT: u32 = 0xD3C6AA; // primary text (everforest foreground)
pub const TEXT_DIM: u32 = 0x8A9591; // secondary / hint text

pub const ACCENT: u32 = 0x7FBBB3; // focus highlight, links, accents (blue)
pub const GOOD: u32 = 0xA7C080; // green
pub const WARN: u32 = 0xDBBC7F; // yellow
pub const DANGER: u32 = 0xE67E80; // red, e.g. the close button

const CORNER_RADIUS: u32 = 6;

// --- primitives ----------------------------------------------------------

/// Filled rounded rectangle, the base shape for windows, buttons and panels.
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

/// Rounded rectangle outline, used for borders and focus rings.
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

/// A flat panel: sunken surface with a thin border, no fake 3D bevel.
pub fn panel(x: i32, y: i32, w: u32, h: u32) {
    rounded_fill(x, y, w, h, SURFACE_DARK);
    rounded_outline(x, y, w, h, BORDER);
}

/// A clickable-looking button with a centered label. Hit-testing is done
/// separately by the caller (see `hit`), this only draws.
pub fn button(x: i32, y: i32, w: u32, h: u32, label: &str) {
    rounded_fill(x, y, w, h, SURFACE);
    rounded_outline(x, y, w, h, BORDER);
    text_centered(label, x, y, w, h, TEXT);
}

/// Whether a point falls inside a rectangular area, used for click/hit tests
/// throughout the GUI instead of repeating the same four comparisons.
pub fn hit(px: i32, py: i32, x: i32, y: i32, w: u32, h: u32) -> bool {
    px >= x && px < x + w as i32 && py >= y && py < y + h as i32
}

// --- text ------------------------------------------------------------------
//
// The console keeps using the bundled PSF font (see framebuffer::draw_text_at)
// so boot logs and text-mode apps look the same as always. The windowed GUI
// instead uses embedded-graphics' built-in 6x10 font, which keeps window
// chrome and widgets on the normal embedded-graphics `Text` + `MonoTextStyle`
// path rather than mixing two unrelated text renderers.

pub fn text(s: &str, x: i32, y: i32, color: u32) {
    let style = MonoTextStyle::new(&FONT_6X10, u32_to_rgb888(color));
    // embedded-graphics anchors text at its baseline; +8 lines it up with the
    // top-left corner callers actually pass in
    let _ = Text::new(s, Point::new(x, y + 8), style).draw(&mut Display);
}

/// Text centered inside a `w`x`h` box whose top-left corner is `(x, y)`.
pub fn text_centered(s: &str, x: i32, y: i32, w: u32, h: u32, color: u32) {
    let text_w = s.len() as i32 * 6; // FONT_6X10 advance width
    let tx = x + (w as i32 - text_w).max(0) / 2;
    let ty = y + (h as i32 - 10).max(0) / 2;
    text(s, tx, ty, color);
}

/// Width in pixels that `text()` would use to draw `s`.
pub fn text_width(s: &str) -> i32 {
    s.len() as i32 * 6
}

// --- icons -------------------------------------------------------------
//
// Desktop icons are small BMP files decoded with tinybmp instead of being
// drawn pixel-by-pixel in code. Add a new icon by dropping a 28x28 24-bit
// BMP into gui/icons/ and a matching entry in `Icon`.

use tinybmp::Bmp;

pub struct Icon(&'static [u8]);

pub const ICON_TERMINAL: Icon = Icon(include_bytes!("icons/terminal.bmp"));
pub const ICON_BROWSER: Icon = Icon(include_bytes!("icons/browser.bmp"));
pub const ICON_CLOCK: Icon = Icon(include_bytes!("icons/clock.bmp"));
pub const ICON_CALC: Icon = Icon(include_bytes!("icons/calc.bmp"));
pub const ICON_PAINT: Icon = Icon(include_bytes!("icons/paint.bmp"));

impl Icon {
    /// Draw this icon's top-left corner at `(x, y)`. Icons are baked with a
    /// `SURFACE` background so they blend into `rounded_fill` slots without
    /// needing real transparency.
    pub fn draw(&self, x: i32, y: i32) {
        match Bmp::<embedded_graphics::pixelcolor::Rgb888>::from_slice(self.0) {
            Ok(bmp) => {
                let _ =
                    embedded_graphics::image::Image::new(&bmp, Point::new(x, y)).draw(&mut Display);
            }
            Err(_) => {
                // malformed asset: fall back to a plain accent square rather
                // than panicking or drawing nothing
                framebuffer::fill_rect(x as usize, y as usize, 28, 28, ACCENT);
            }
        }
    }
}

// --- wallpaper -----------------------------------------------------------
//
// Baked at build time by build.rs (see the comment there): it crops the
// source photo down and bakes it to a fixed-size BMP, which tinybmp decodes
// the same way it does the icons above.

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

    // Инициализируем при первом вызове, если еще не успели вызвать init_wallpaper()
    let bmp_option = WALLPAPER_BMP.call_once(|| {
        Bmp::<embedded_graphics::pixelcolor::Rgb888>::from_slice(WALLPAPER_BYTES).ok()
    });

    if let Some(bmp) = bmp_option {
        let size = bmp.size();

        // Заливаем фон только если обои меньше размера экрана
        if size.width < screen_w as u32 || size.height < screen_h as u32 {
            framebuffer::fill_rect(0, 0, screen_w, screen_h, DESKTOP_BG);
        }

        let _ = embedded_graphics::image::Image::new(bmp, Point::zero()).draw(&mut Display);
    } else {
        // Fallback: если BMP повреждён
        framebuffer::fill_rect(0, 0, screen_w, screen_h, DESKTOP_BG);
    }
}
