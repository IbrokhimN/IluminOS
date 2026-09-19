// tiny paint canvas with a color palette
use crate::framebuffer;
use crate::gui::style::{self, hit};

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
