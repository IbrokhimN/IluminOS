use crate::fs::{self, FILE_MAX_BYTES};
use crate::keyboard::{self, KEY_BACKSPACE, KEY_ENTER, KEY_ESC};
use crate::framebuffer::{self, CYAN, GREEN, YELLOW, WHITE, GRAY};
use crate::{print, println, print_color};

const ROWS: usize = 23; // text rows screen minus status bar
const COLS: usize = 79; // chars per row

// editor modes
#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Normal,  // navigation and commands
    Insert,  // typing text
    Command, // typing a command after colon
}

// full editor state
struct Editor {
    grid: [[u8; COLS]; ROWS],  // character canvas
    line_len: [usize; ROWS],   // length of each line
    n_lines: usize,            // lines in use
    cur_r: usize,              // cursor row
    cur_c: usize,              // cursor column
    mode: Mode,
    cmd: [u8; 16],             // command buffer after colon
    cmd_len: usize,
    pending: u8,               // waiting for second key for dd dw
    dirty: bool,               // unsaved changes present
    quit: bool,                // time to exit
}

// entry point open file name in the editor
pub fn run(name: &str) {
    let mut ed = Editor {
        grid: [[b' '; COLS]; ROWS], // blank canvas of spaces
        line_len: [0; ROWS],
        n_lines: 1,
        cur_r: 0, cur_c: 0,
        mode: Mode::Normal,
        cmd: [0; 16], cmd_len: 0,
        pending: 0,
        dirty: false, quit: false,
    };

    ed.load(name);     // load file contents into the grid
    ed.redraw(name);   // draw

    // main loop key then mode handler then redraw
    while !ed.quit {
        let key = keyboard::read_key(); // blocking read
        match ed.mode {
            Mode::Normal => ed.handle_normal(key, name),
            Mode::Insert => ed.handle_insert(key),
            Mode::Command => ed.handle_command(key, name),
        }
        ed.redraw(name);
    }

    framebuffer::clear(); // clear screen on exit
}

impl Editor {
    // read file and split into grid lines by newline
    fn load(&mut self, name: &str) {
        let mut buf = [0u8; FILE_MAX_BYTES];
        if let Ok(size) = fs::read(name, &mut buf) {
            let mut r = 0;
            let mut c = 0;
            for &b in &buf[..size] {
                if b == b'\n' {
                    self.line_len[r] = c; // end of line
                    r += 1;
                    c = 0;
                    if r >= ROWS {        // file longer than screen truncate
                        r = ROWS - 1;
                        break;
                    }
                } else if c < COLS {
                    self.grid[r][c] = b;  // char into grid
                    c += 1;
                }
            }
            if r < ROWS {
                self.line_len[r] = c;
            }
            self.n_lines = r + 1;
        }
    }

    fn cur_line_len(&self) -> usize {
        self.line_len[self.cur_r]
    }

    // keep cursor within line and text bounds
    fn clamp_cursor(&mut self) {
        if self.cur_r >= self.n_lines {
            self.cur_r = self.n_lines - 1;
        }
        // insert mode cursor can sit past last char normal mode sits on a char
        let max_c = if self.mode == Mode::Insert {
            self.cur_line_len()
        } else {
            self.cur_line_len().saturating_sub(1) // wont go below zero
        };
        if self.cur_c > max_c {
            self.cur_c = max_c;
        }
    }

    fn handle_normal(&mut self, key: u8, _name: &str) {
        // waiting for second key after d for dd or dw
        if self.pending == b'd' {
            self.pending = 0;
            match key {
                b'd' => self.delete_line(), // dd deletes a line
                b'w' => self.delete_word(), // dw deletes a word
                _ => {}
            }
            self.clamp_cursor();
            return;
        }

        match key {
            // cursor movement classic vim keys
            b'h' => if self.cur_c > 0 { self.cur_c -= 1; },
            b'l' => if self.cur_c + 1 < self.cur_line_len() { self.cur_c += 1; },
            b'j' => if self.cur_r + 1 < self.n_lines { self.cur_r += 1; self.clamp_cursor(); },
            b'k' => if self.cur_r > 0 { self.cur_r -= 1; self.clamp_cursor(); },
            b'0' => self.cur_c = 0,                                   // start of line
            b'$' => self.cur_c = self.cur_line_len().saturating_sub(1), // end of line
            // ways to enter insert mode
            b'i' => self.mode = Mode::Insert,                        // before cursor
            b'a' => { if self.cur_line_len() > 0 { self.cur_c += 1; } self.mode = Mode::Insert; }, // after cursor
            b'A' => { self.cur_c = self.cur_line_len(); self.mode = Mode::Insert; },  // end of line
            b'I' => { self.cur_c = 0; self.mode = Mode::Insert; },   // start of line
            b'o' => { self.open_line_below(); self.mode = Mode::Insert; }, // new line below
            b'x' => self.delete_char(),                              // delete char under cursor
            b'd' => self.pending = b'd',                             // wait for second key dd dw
            b':' => { self.mode = Mode::Command; self.cmd_len = 0; }, // enter command mode
            _ => {}
        }
    }

    fn handle_insert(&mut self, key: u8) {
        match key {
            KEY_ESC => { // exit to normal mode
                self.mode = Mode::Normal;
                if self.cur_c > 0 { self.cur_c -= 1; }
                self.clamp_cursor();
            }
            KEY_ENTER => self.insert_newline(),
            KEY_BACKSPACE => self.backspace(),
            0x20..=0x7e | 0x80..=0xff => self.insert_char(key), // printable char including cyrillic
            _ => {} // ignore arrows and other keys
        }
    }

    fn handle_command(&mut self, key: u8, name: &str) {
        match key {
            KEY_ESC => { self.mode = Mode::Normal; self.cmd_len = 0; }, // cancel
            KEY_ENTER => { // run the command
                self.exec_command(name);
                self.mode = Mode::Normal;
                self.cmd_len = 0;
            }
            KEY_BACKSPACE => if self.cmd_len > 0 { self.cmd_len -= 1; },
            0x20..=0x7e | 0x80..=0xff => if self.cmd_len < self.cmd.len() { // build up the command
                self.cmd[self.cmd_len] = key;
                self.cmd_len += 1;
            },
            _ => {}
        }
    }

    // run w q wq or q! commands
    fn exec_command(&mut self, name: &str) {
        let cmd = &self.cmd[..self.cmd_len];
        match cmd {
            b"w" => self.save(name),                        // save
            b"q" => self.quit = true,                       // quit
            b"wq" | b"x" => { self.save(name); self.quit = true; }, // save and quit
            b"q!" => self.quit = true,                      // quit without saving
            _ => {}
        }
    }

    // insert char at cursor shifting the tail right
    fn insert_char(&mut self, ch: u8) {
        let len = self.line_len[self.cur_r];
        if len >= COLS {
            return; // line is full
        }
        // shift chars right of cursor one step right
        let mut i = len;
        while i > self.cur_c {
            self.grid[self.cur_r][i] = self.grid[self.cur_r][i - 1];
            i -= 1;
        }
        self.grid[self.cur_r][self.cur_c] = ch; // place new char
        self.line_len[self.cur_r] += 1;
        self.cur_c += 1;
        self.dirty = true;
    }

    // enter splits the line at cursor position
    fn insert_newline(&mut self) {
        if self.n_lines >= ROWS {
            return; // reached line limit
        }
        // shift lines below cursor down one to make room
        let mut r = self.n_lines;
        while r > self.cur_r + 1 {
            self.grid[r] = self.grid[r - 1];
            self.line_len[r] = self.line_len[r - 1];
            r -= 1;
        }
        // move current line tail after cursor to the new line
        let tail_start = self.cur_c;
        let tail_len = self.line_len[self.cur_r] - tail_start;
        let mut new_line = [b' '; COLS];
        for i in 0..tail_len {
            new_line[i] = self.grid[self.cur_r][tail_start + i];
        }
        self.grid[self.cur_r + 1] = new_line;
        self.line_len[self.cur_r + 1] = tail_len;
        self.line_len[self.cur_r] = tail_start; // truncate current line

        self.n_lines += 1;
        self.cur_r += 1;
        self.cur_c = 0;
        self.dirty = true;
    }

    // delete char to the left at line start merge with previous
    fn backspace(&mut self) {
        if self.cur_c > 0 {
            // shift line tail left overwriting the char
            let len = self.line_len[self.cur_r];
            for i in (self.cur_c - 1)..len.saturating_sub(1) {
                self.grid[self.cur_r][i] = self.grid[self.cur_r][i + 1];
            }
            self.grid[self.cur_r][len - 1] = b' ';
            self.line_len[self.cur_r] -= 1;
            self.cur_c -= 1;
            self.dirty = true;
        } else if self.cur_r > 0 {
            // cursor at line start merge into previous line
            let prev_len = self.line_len[self.cur_r - 1];
            let cur_len = self.line_len[self.cur_r];
            if prev_len + cur_len <= COLS {
                for i in 0..cur_len {
                    self.grid[self.cur_r - 1][prev_len + i] = self.grid[self.cur_r][i];
                }
                self.line_len[self.cur_r - 1] = prev_len + cur_len;
                // shift lines below up current line is gone
                for r in self.cur_r..(self.n_lines - 1) {
                    self.grid[r] = self.grid[r + 1];
                    self.line_len[r] = self.line_len[r + 1];
                }
                self.n_lines -= 1;
                self.cur_r -= 1;
                self.cur_c = prev_len; // cursor at the merge point
                self.dirty = true;
            }
        }
    }

    // delete char under cursor command x
    fn delete_char(&mut self) {
        let len = self.line_len[self.cur_r];
        if self.cur_c < len {
            for i in self.cur_c..len.saturating_sub(1) {
                self.grid[self.cur_r][i] = self.grid[self.cur_r][i + 1];
            }
            self.grid[self.cur_r][len - 1] = b' ';
            self.line_len[self.cur_r] -= 1;
            if self.cur_c > 0 && self.cur_c >= self.line_len[self.cur_r] {
                self.cur_c = self.line_len[self.cur_r].saturating_sub(1);
            }
            self.dirty = true;
        }
    }

    // delete whole line command dd
    fn delete_line(&mut self) {
        if self.n_lines <= 1 {
            // only line just clear it
            self.grid[0] = [b' '; COLS];
            self.line_len[0] = 0;
            self.cur_c = 0;
            self.dirty = true;
            return;
        }
        // shift lines below up
        for r in self.cur_r..(self.n_lines - 1) {
            self.grid[r] = self.grid[r + 1];
            self.line_len[r] = self.line_len[r + 1];
        }
        self.n_lines -= 1;
        if self.cur_r >= self.n_lines {
            self.cur_r = self.n_lines - 1;
        }
        self.cur_c = 0;
        self.dirty = true;
    }

    // delete word from cursor command dw
    fn delete_word(&mut self) {
        let len = self.line_len[self.cur_r];
        if self.cur_c >= len {
            return;
        }
        // find end of word up to space
        let mut end = self.cur_c;
        while end < len && self.grid[self.cur_r][end] != b' ' {
            end += 1;
        }
        // and include trailing spaces
        while end < len && self.grid[self.cur_r][end] == b' ' {
            end += 1;
        }
        let del = end - self.cur_c; // chars to delete
        // shift tail into the deleted space
        for i in self.cur_c..(len - del) {
            self.grid[self.cur_r][i] = self.grid[self.cur_r][i + del];
        }
        for i in (len - del)..len {
            self.grid[self.cur_r][i] = b' ';
        }
        self.line_len[self.cur_r] -= del;
        self.clamp_cursor();
        self.dirty = true;
    }

    // insert empty line below command o
    fn open_line_below(&mut self) {
        if self.n_lines >= ROWS {
            return;
        }
        let mut r = self.n_lines;
        while r > self.cur_r + 1 {
            self.grid[r] = self.grid[r - 1];
            self.line_len[r] = self.line_len[r - 1];
            r -= 1;
        }
        self.grid[self.cur_r + 1] = [b' '; COLS];
        self.line_len[self.cur_r + 1] = 0;
        self.n_lines += 1;
        self.cur_r += 1;
        self.cur_c = 0;
        self.dirty = true;
    }

    // rebuild bytes from grid lines joined by newline and save
    fn save(&mut self, name: &str) {
        let mut out = [0u8; FILE_MAX_BYTES];
        let mut n = 0;
        for r in 0..self.n_lines {
            let len = self.line_len[r];
            for c in 0..len {
                if n < FILE_MAX_BYTES {
                    out[n] = self.grid[r][c];
                    n += 1;
                }
            }
            // newline between lines except the last
            if r + 1 < self.n_lines && n < FILE_MAX_BYTES {
                out[n] = b'\n';
                n += 1;
            }
        }
        let _ = fs::write(name, &out[..n]);
        self.dirty = false;
    }

    // redraw whole screen text status bar cursor
    fn redraw(&self, name: &str) {
        framebuffer::clear();
        for r in 0..self.n_lines {
            let len = self.line_len[r];
            highlight_line(&self.grid[r], len); // with syntax highlighting
            println!();
        }
        // fill remaining rows blank to bottom of screen
        for _ in self.n_lines..ROWS {
            println!();
        }
        self.draw_status(name);
        self.draw_cursor();
    }

    // status bar mode filename dirty mark position
    fn draw_status(&self, name: &str) {
        let mode_str = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
        };
        let mode_color = match self.mode {
            Mode::Normal => CYAN,
            Mode::Insert => GREEN,
            Mode::Command => YELLOW,
        };
        print_color!(mode_color, "-- {} --", mode_str);
        let dirty_mark = if self.dirty { "[+]" } else { "" }; // plus means unsaved
        print_color!(WHITE, " {} {}  {}:{}", name, dirty_mark, self.cur_r + 1, self.cur_c + 1);

        // command mode shows the command being typed
        if self.mode == Mode::Command {
            print!("  :");
            for i in 0..self.cmd_len {
                print!("{}", self.cmd[i] as char);
            }
        }
    }

    // underline under current cell
    fn draw_cursor(&self) {
        framebuffer::draw_edit_cursor(self.cur_c, self.cur_r);
    }
}

// rust syntax highlighting colors a line by token type

// token colors
const COL_KEYWORD: u32 = 0xFF8844; // orange for keywords
const COL_STRING: u32 = GREEN;     // green for strings
const COL_NUMBER: u32 = YELLOW;    // yellow for numbers
const COL_COMMENT: u32 = GRAY;     // gray for comments
const COL_TYPE: u32 = CYAN;        // cyan for capitalized types
const COL_NORMAL: u32 = WHITE;     // white for everything else

// list of rust keywords to highlight
const KEYWORDS: &[&[u8]] = &[
    b"fn", b"let", b"mut", b"if", b"else", b"for", b"while", b"loop",
    b"match", b"return", b"break", b"continue", b"struct", b"enum",
    b"impl", b"trait", b"pub", b"use", b"mod", b"const", b"static",
    b"self", b"Self", b"as", b"in", b"ref", b"move", b"where", b"type",
    b"unsafe", b"extern", b"crate", b"super", b"dyn", b"async", b"await",
    b"true", b"false",
];

fn is_ident_char(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || (c >= b'0' && c <= b'9') || c == b'_'
}

fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

fn is_keyword(word: &[u8]) -> bool {
    for kw in KEYWORDS {
        if *kw == word {
            return true;
        }
    }
    false
}

// prints a line coloring tokens a tiny lexer
fn highlight_line(line: &[u8; COLS], len: usize) {
    let mut i = 0;
    while i < len {
        let c = line[i];

        // line comment gray to end of line
        if c == b'/' && i + 1 < len && line[i + 1] == b'/' {
            framebuffer::set_color(COL_COMMENT);
            while i < len {
                print!("{}", line[i] as char);
                i += 1;
            }
            break;
        }

        // double quoted string in green
        if c == b'"' {
            framebuffer::set_color(COL_STRING);
            print!("{}", c as char);
            i += 1;
            while i < len {
                let ch = line[i];
                print!("{}", ch as char);
                i += 1;
                if ch == b'"' {
                    break; // closing quote escapes not handled
                }
            }
            continue;
        }

        // single quoted char also green
        if c == b'\'' {
            framebuffer::set_color(COL_STRING);
            print!("{}", c as char);
            i += 1;
            while i < len {
                let ch = line[i];
                print!("{}", ch as char);
                i += 1;
                if ch == b'\'' {
                    break;
                }
            }
            continue;
        }

        // number in yellow handles hex digits dot underscore x
        if is_digit(c) {
            framebuffer::set_color(COL_NUMBER);
            while i < len && (is_digit(line[i]) || line[i] == b'.' || line[i] == b'_'
                || (line[i] >= b'a' && line[i] <= b'f') || (line[i] >= b'A' && line[i] <= b'F')
                || line[i] == b'x') {
                print!("{}", line[i] as char);
                i += 1;
            }
            continue;
        }

        // identifier or keyword
        if is_ident_char(c) {
            let start = i;
            while i < len && is_ident_char(line[i]) {
                i += 1;
            }
            let word = &line[start..i];
            let color = if is_keyword(word) {
                COL_KEYWORD               // keyword in orange
            } else if word[0] >= b'A' && word[0] <= b'Z' {
                COL_TYPE                  // capitalized treated as a type in cyan
            } else {
                COL_NORMAL                // plain name in white
            };
            framebuffer::set_color(color);
            for &b in word {
                print!("{}", b as char);
            }
            continue;
        }

        // everything else brackets operators in white
        framebuffer::set_color(COL_NORMAL);
        print!("{}", c as char);
        i += 1;
    }
    framebuffer::set_color(WHITE); // restore default color
}
