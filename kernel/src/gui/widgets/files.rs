use crate::fs;
use crate::framebuffer;
use crate::gui::style;
use crate::gui::wm::{Rect, TextField};
use crate::keyboard::{KEY_ENTER, KEY_ESC};
use alloc::string::String;
use alloc::vec::Vec;

const ROW_H: i32 = 18;
const TOOLBAR_Y: i32 = 20;
const STATUS_Y: i32 = 44;
const LIST_TOP: i32 = 66;

struct Entry {
    name: String,
    size: u32,
    is_dir: bool,
}

#[derive(PartialEq, Clone, Copy)]
enum Prompt {
    None,
    NewFile,
    NewDir,
}

pub struct Files {
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    entries: Vec<Entry>,
    selected: Option<usize>,
    status: String,
    prompt: Prompt,
    input: TextField,
    btn_new_dir: Rect,
    btn_new_file: Rect,
    btn_delete: Rect,
    btn_up: Rect,
}

impl Files {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let (bx, by) = (cx as i32 + 8, cy as i32 + TOOLBAR_Y);
        let (bw, bh, gap) = (78, 20, 6);
        let btn_new_dir = Rect::new(bx, by, bw, bh);
        let btn_new_file = Rect::new(bx + (bw + gap), by, bw, bh);
        let btn_delete = Rect::new(bx + 2 * (bw + gap), by, bw, bh);
        let btn_up = Rect::new(bx + 3 * (bw + gap), by, 50, bh);

        let input_area = Rect::new(cx as i32 + 8, cy as i32 + STATUS_Y, cw as i32 - 16, 20);

        let mut me = Files {
            cx,
            cy,
            cw,
            ch,
            entries: Vec::new(),
            selected: None,
            status: String::new(),
            prompt: Prompt::None,
            input: TextField::new(input_area, fs::NAME_MAX),
            btn_new_dir,
            btn_new_file,
            btn_delete,
            btn_up,
        };
        me.refresh();
        me
    }

    fn refresh(&mut self) {
        self.entries.clear();
        fs::list(|name, size, is_dir| {
            self.entries.push(Entry { name: String::from(name), size, is_dir });
        });
        self.entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
        self.selected = None;
    }

    fn path_string(&self) -> String {
        let mut buf = [0u8; 256];
        let len = fs::pwd_into(&mut buf);
        core::str::from_utf8(&buf[..len]).unwrap_or("/").into()
    }

    fn visible_rows(&self) -> usize {
        let avail = (self.ch as i32 - LIST_TOP - 4).max(0);
        (avail / ROW_H).max(0) as usize
    }

    pub fn redraw(&mut self) {
        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, style::WINDOW_BG);

        // path bar
        let path = self.path_string();
        style::text(&path, self.cx as i32 + 8, self.cy as i32 + 2, style::ACCENT);

        // toolbar
        style::button(self.btn_new_dir.x, self.btn_new_dir.y, self.btn_new_dir.w as u32, self.btn_new_dir.h as u32, "New Dir");
        style::button(self.btn_new_file.x, self.btn_new_file.y, self.btn_new_file.w as u32, self.btn_new_file.h as u32, "New File");
        style::button(self.btn_delete.x, self.btn_delete.y, self.btn_delete.w as u32, self.btn_delete.h as u32, "Delete");
        style::button(self.btn_up.x, self.btn_up.y, self.btn_up.w as u32, self.btn_up.h as u32, "Up");

        // status / prompt row
        match self.prompt {
            Prompt::None => {
                let msg = if self.status.is_empty() {
                    match self.selected {
                        Some(i) => alloc::format!("{} - {} bytes", self.entries[i].name, self.entries[i].size),
                        None => String::from("click a folder to open it, a file to select it"),
                    }
                } else {
                    self.status.clone()
                };
                style::text(&msg, self.cx as i32 + 8, self.cy as i32 + STATUS_Y + 4, style::TEXT_DIM);
            }
            Prompt::NewDir | Prompt::NewFile => {
                let label = if self.prompt == Prompt::NewDir { "new folder name:" } else { "new file name:" };
                style::text(label, self.cx as i32 + 8, self.cy as i32 + STATUS_Y - 12, style::WARN);
                self.input.draw();
            }
        }

        // entries
        let top = self.cy as i32 + LIST_TOP;
        let visible = self.visible_rows();
        for (i, e) in self.entries.iter().enumerate().take(visible) {
            let y = top + i as i32 * ROW_H;
            if Some(i) == self.selected {
                framebuffer::fill_rect(self.cx + 4, y as usize, self.cw - 8, ROW_H as usize - 2, style::SURFACE);
            }
            draw_row_icon(self.cx as i32 + 8, y + 2, e.is_dir, file_color(&e.name));
            let name_color = if e.is_dir { style::WARN } else { style::TEXT };
            style::text(&e.name, self.cx as i32 + 26, y + 2, name_color);
            if !e.is_dir {
                let size_text = alloc::format!("{}B", e.size);
                let sx = self.cx as i32 + self.cw as i32 - 12 - style::text_width(&size_text);
                style::text(&size_text, sx, y + 2, style::TEXT_DIM);
            }
        }
        if self.entries.len() > visible {
            let y = top + visible as i32 * ROW_H;
            style::text(
                &alloc::format!("+ {} more", self.entries.len() - visible),
                self.cx as i32 + 26,
                y + 2,
                style::TEXT_DIM,
            );
        } else if self.entries.is_empty() {
            style::text("(empty folder)", self.cx as i32 + 26, top + 2, style::TEXT_DIM);
        }
    }

    pub fn on_click(&mut self, x: i32, y: i32) -> bool {
        if self.prompt != Prompt::None {
            if !self.input.hit(x, y) {
                self.prompt = Prompt::None;
            }
            return true;
        }

        if self.btn_new_dir.contains(x, y) {
            self.prompt = Prompt::NewDir;
            self.input.text.clear();
            self.status.clear();
            return true;
        }
        if self.btn_new_file.contains(x, y) {
            self.prompt = Prompt::NewFile;
            self.input.text.clear();
            self.status.clear();
            return true;
        }
        if self.btn_delete.contains(x, y) {
            if let Some(i) = self.selected {
                let name = self.entries[i].name.clone();
                match fs::remove(&name) {
                    Ok(()) => {
                        self.status = alloc::format!("deleted {}", name);
                        self.refresh();
                    }
                    Err(e) => self.status = alloc::format!("can't delete {}: {}", name, e),
                }
            } else {
                self.status = String::from("select something to delete first");
            }
            return true;
        }
        if self.btn_up.contains(x, y) {
            let _ = fs::chdir("..");
            self.status.clear();
            self.refresh();
            return true;
        }

        // list rows
        let top = self.cy as i32 + LIST_TOP;
        if y >= top {
            let row = ((y - top) / ROW_H) as usize;
            if row < self.entries.len().min(self.visible_rows()) {
                if self.entries[row].is_dir {
                    let name = self.entries[row].name.clone();
                    if fs::chdir(&name).is_ok() {
                        self.status.clear();
                        self.refresh();
                    } else {
                        self.status = alloc::format!("can't open {}", name);
                    }
                } else {
                    self.selected = Some(row);
                    self.status.clear();
                }
                return true;
            }
        }

        false
    }

    pub fn on_key(&mut self, key: u8) -> bool {
        if self.prompt == Prompt::None {
            return false;
        }

        match key {
            KEY_ENTER => {
                let name = self.input.text.clone();
                let result = if name.is_empty() {
                    Err("name can't be empty")
                } else if self.prompt == Prompt::NewDir {
                    fs::mkdir(&name)
                } else {
                    fs::create(&name)
                };
                match result {
                    Ok(()) => {
                        self.status = alloc::format!("created {}", name);
                        self.prompt = Prompt::None;
                        self.refresh();
                    }
                    Err(e) => {
                        self.status = alloc::format!("error: {}", e);
                        self.prompt = Prompt::None;
                    }
                }
                true
            }
            KEY_ESC => {
                self.prompt = Prompt::None;
                true
            }
            _ => self.input.key(key),
        }
    }
}

fn draw_row_icon(x: i32, y: i32, is_dir: bool, color: u32) {
    if is_dir {
        framebuffer::fill_rect(x as usize, y as usize, 6, 3, style::WARN);
        framebuffer::fill_rect(x as usize, (y + 3) as usize, 14, 9, style::WARN);
    } else {
        framebuffer::fill_rect(x as usize, y as usize, 11, 13, color);
        framebuffer::fill_rect((x + 8) as usize, y as usize, 3, 3, style::WINDOW_BG);
    }
}

fn file_color(name: &str) -> u32 {
    if name.ends_with(".html") || name.ends_with(".htm") {
        style::DANGER
    } else if name.ends_with(".gmi") || name.ends_with(".gemini") {
        style::ACCENT
    } else if name.ends_with(".wasm") {
        0x9977DD
    } else if name.ends_with(".txt") || name.ends_with(".md") {
        style::TEXT_DIM
    } else {
        style::TEXT
    }
}
