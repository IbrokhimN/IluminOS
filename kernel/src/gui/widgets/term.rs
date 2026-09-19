// tiny toy terminal widget (not the real shell, just a gui demo prompt)
use crate::framebuffer::{self, draw_text_at};
use alloc::string::String;
use alloc::vec::Vec;

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
