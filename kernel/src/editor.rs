// мини-vim: модальный редактор (normal/insert), hjkl, dd/dw/x, :w :q :wq
use crate::fs::{self, FILE_MAX_BYTES};
use crate::keyboard::{self, KEY_BACKSPACE, KEY_ENTER, KEY_ESC};
use crate::framebuffer::{self, CYAN, GREEN, YELLOW, WHITE, GRAY};
use crate::{print, println, print_color};

const ROWS: usize = 23; // 24 строки экрана минус статус-бар
const COLS: usize = 79;

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Normal,
    Insert,
    Command, // ввод команды после :
}

struct Editor {
    grid: [[u8; COLS]; ROWS],
    line_len: [usize; ROWS],
    n_lines: usize, // сколько строк реально используется
    cur_r: usize,
    cur_c: usize,
    mode: Mode,
    cmd: [u8; 16], // буфер командной строки после :
    cmd_len: usize,
    pending: u8, // ожидание второй клавиши (для dd, dw)
    dirty: bool, // были ли изменения
    quit: bool,
}

pub fn run(name: &str) {
    let mut ed = Editor {
        grid: [[b' '; COLS]; ROWS],
        line_len: [0; ROWS],
        n_lines: 1,
        cur_r: 0,
        cur_c: 0,
        mode: Mode::Normal,
        cmd: [0; 16],
        cmd_len: 0,
        pending: 0,
        dirty: false,
        quit: false,
    };

    ed.load(name);
    ed.redraw(name);

    while !ed.quit {
        let key = keyboard::read_key();
        match ed.mode {
            Mode::Normal => ed.handle_normal(key, name),
            Mode::Insert => ed.handle_insert(key),
            Mode::Command => ed.handle_command(key, name),
        }
        ed.redraw(name);
    }

    framebuffer::clear();
}

impl Editor {
    fn load(&mut self, name: &str) {
        let mut buf = [0u8; FILE_MAX_BYTES];
        if let Ok(size) = fs::read(name, &mut buf) {
            let mut r = 0;
            let mut c = 0;
            for &b in &buf[..size] {
                if b == b'\n' {
                    self.line_len[r] = c;
                    r += 1;
                    c = 0;
                    if r >= ROWS {
                        r = ROWS - 1;
                        break;
                    }
                } else if c < COLS {
                    self.grid[r][c] = b;
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

    // ограничить курсор в пределах строки
    fn clamp_cursor(&mut self) {
        if self.cur_r >= self.n_lines {
            self.cur_r = self.n_lines - 1;
        }
        let max_c = if self.mode == Mode::Insert {
            self.cur_line_len()
        } else {
            self.cur_line_len().saturating_sub(1)
        };
        if self.cur_c > max_c {
            self.cur_c = max_c;
        }
    }

    fn handle_normal(&mut self, key: u8, _name: &str) {
        if self.pending == b'd' {
            self.pending = 0;
            match key {
                b'd' => self.delete_line(),
                b'w' => self.delete_word(),
                _ => {}
            }
            self.clamp_cursor();
            return;
        }

        match key {
            b'h' => {
                if self.cur_c > 0 {
                    self.cur_c -= 1;
                }
            }
            b'l' => {
                if self.cur_c + 1 < self.cur_line_len() {
                    self.cur_c += 1;
                }
            }
            b'j' => {
                if self.cur_r + 1 < self.n_lines {
                    self.cur_r += 1;
                    self.clamp_cursor();
                }
            }
            b'k' => {
                if self.cur_r > 0 {
                    self.cur_r -= 1;
                    self.clamp_cursor();
                }
            }
            b'0' => self.cur_c = 0,
            b'$' => {
                self.cur_c = self.cur_line_len().saturating_sub(1);
            }
            b'i' => self.mode = Mode::Insert,
            b'a' => {
                if self.cur_line_len() > 0 {
                    self.cur_c += 1;
                }
                self.mode = Mode::Insert;
            }
            b'A' => {
                self.cur_c = self.cur_line_len();
                self.mode = Mode::Insert;
            }
            b'I' => {
                self.cur_c = 0;
                self.mode = Mode::Insert;
            }
            b'o' => {
                self.open_line_below();
                self.mode = Mode::Insert;
            }
            b'x' => self.delete_char(),
            b'd' => self.pending = b'd',
            b':' => {
                self.mode = Mode::Command;
                self.cmd_len = 0;
            }
            _ => {}
        }
    }

    fn handle_insert(&mut self, key: u8) {
        match key {
            KEY_ESC => {
                self.mode = Mode::Normal;
                if self.cur_c > 0 {
                    self.cur_c -= 1;
                }
                self.clamp_cursor();
            }
            KEY_ENTER => self.insert_newline(),
            KEY_BACKSPACE => self.backspace(),
            0x20..=0x7e => self.insert_char(key),
            _ => {}
        }
    }

    fn handle_command(&mut self, key: u8, name: &str) {
        match key {
            KEY_ESC => {
                self.mode = Mode::Normal;
                self.cmd_len = 0;
            }
            KEY_ENTER => {
                self.exec_command(name);
                self.mode = Mode::Normal;
                self.cmd_len = 0;
            }
            KEY_BACKSPACE => {
                if self.cmd_len > 0 {
                    self.cmd_len -= 1;
                }
            }
            0x20..=0x7e => {
                if self.cmd_len < self.cmd.len() {
                    self.cmd[self.cmd_len] = key;
                    self.cmd_len += 1;
                }
            }
            _ => {}
        }
    }

    fn exec_command(&mut self, name: &str) {
        let cmd = &self.cmd[..self.cmd_len];
        match cmd {
            b"w" => {
                self.save(name);
            }
            b"q" => {
                self.quit = true;
            }
            b"wq" | b"x" => {
                self.save(name);
                self.quit = true;
            }
            b"q!" => {
                self.quit = true;
            }
            _ => {}
        }
    }

    fn insert_char(&mut self, ch: u8) {
        let len = self.line_len[self.cur_r];
        if len >= COLS {
            return;
        }
        // сдвигаем хвост строки вправо
        let mut i = len;
        while i > self.cur_c {
            self.grid[self.cur_r][i] = self.grid[self.cur_r][i - 1];
            i -= 1;
        }
        self.grid[self.cur_r][self.cur_c] = ch;
        self.line_len[self.cur_r] += 1;
        self.cur_c += 1;
        self.dirty = true;
    }

    fn insert_newline(&mut self) {
        if self.n_lines >= ROWS {
            return;
        }
        // сдвигаем строки ниже вниз
        let mut r = self.n_lines;
        while r > self.cur_r + 1 {
            self.grid[r] = self.grid[r - 1];
            self.line_len[r] = self.line_len[r - 1];
            r -= 1;
        }
        // переносим хвост текущей строки на новую
        let tail_start = self.cur_c;
        let tail_len = self.line_len[self.cur_r] - tail_start;
        let mut new_line = [b' '; COLS];
        for i in 0..tail_len {
            new_line[i] = self.grid[self.cur_r][tail_start + i];
        }
        self.grid[self.cur_r + 1] = new_line;
        self.line_len[self.cur_r + 1] = tail_len;
        self.line_len[self.cur_r] = tail_start;

        self.n_lines += 1;
        self.cur_r += 1;
        self.cur_c = 0;
        self.dirty = true;
    }

    fn backspace(&mut self) {
        if self.cur_c > 0 {
            // удаляем символ слева, сдвигаем хвост
            let len = self.line_len[self.cur_r];
            for i in (self.cur_c - 1)..len.saturating_sub(1) {
                self.grid[self.cur_r][i] = self.grid[self.cur_r][i + 1];
            }
            self.grid[self.cur_r][len - 1] = b' ';
            self.line_len[self.cur_r] -= 1;
            self.cur_c -= 1;
            self.dirty = true;
        } else if self.cur_r > 0 {
            // склеиваем с предыдущей строкой
            let prev_len = self.line_len[self.cur_r - 1];
            let cur_len = self.line_len[self.cur_r];
            if prev_len + cur_len <= COLS {
                for i in 0..cur_len {
                    self.grid[self.cur_r - 1][prev_len + i] = self.grid[self.cur_r][i];
                }
                self.line_len[self.cur_r - 1] = prev_len + cur_len;
                // сдвигаем строки ниже вверх
                for r in self.cur_r..(self.n_lines - 1) {
                    self.grid[r] = self.grid[r + 1];
                    self.line_len[r] = self.line_len[r + 1];
                }
                self.n_lines -= 1;
                self.cur_r -= 1;
                self.cur_c = prev_len;
                self.dirty = true;
            }
        }
    }

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

    fn delete_line(&mut self) {
        if self.n_lines <= 1 {
            // последняя строка - просто очищаем
            self.grid[0] = [b' '; COLS];
            self.line_len[0] = 0;
            self.cur_c = 0;
            self.dirty = true;
            return;
        }
        // сдвигаем строки ниже вверх
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

    fn delete_word(&mut self) {
        let len = self.line_len[self.cur_r];
        if self.cur_c >= len {
            return;
        }
        // конец слова: до следующего пробела
        let mut end = self.cur_c;
        while end < len && self.grid[self.cur_r][end] != b' ' {
            end += 1;
        }
        // захватываем пробелы после слова
        while end < len && self.grid[self.cur_r][end] == b' ' {
            end += 1;
        }
        let del = end - self.cur_c;
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
            if r + 1 < self.n_lines && n < FILE_MAX_BYTES {
                out[n] = b'\n';
                n += 1;
            }
        }
        let _ = fs::write(name, &out[..n]);
        self.dirty = false;
    }

    fn redraw(&self, name: &str) {
        framebuffer::clear();
        for r in 0..self.n_lines {
            let len = self.line_len[r];
            highlight_line(&self.grid[r], len);
            println!();
        }
        // переводим на строку статус-бара (23-я строка, 0-индекс)
        // печатаем пустые строки до низа
        for _ in self.n_lines..ROWS {
            println!();
        }
        self.draw_status(name);
        self.draw_cursor();
    }

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
        let dirty_mark = if self.dirty { "[+]" } else { "" };
        print_color!(WHITE, " {} {}  {}:{}", name, dirty_mark, self.cur_r + 1, self.cur_c + 1);

        // если в командном режиме показываем ввод команды
        if self.mode == Mode::Command {
            print!("  :");
            for i in 0..self.cmd_len {
                print!("{}", self.cmd[i] as char);
            }
        }
    }

    // курсор рисуем перемещением текстового курсора фреймбуфера
    // (мигалка из framebuffer следит за позицией печати, так что
    //  печатаем символ под курсором заново, оставляя позицию там)
    fn draw_cursor(&self) {
        // статичный курсор-подчёркивание под текущей клеткой
        framebuffer::draw_edit_cursor(self.cur_c, self.cur_r);
    }
}

// подстветка
// цвета токенов
const COL_KEYWORD: u32 = 0xFF8844; // оранжевый - ключевые слова
const COL_STRING: u32 = GREEN;      // зелёный - строки
const COL_NUMBER: u32 = YELLOW;     // жёлтый - числа
const COL_COMMENT: u32 = GRAY;      // серый - комментарии
const COL_TYPE: u32 = CYAN;         // голубой - типы (с заглавной)
const COL_NORMAL: u32 = WHITE;      // белый - остальное

// ключевые слова Rust
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

// печатает строку с подсветкой синтаксиса Rust
fn highlight_line(line: &[u8; COLS], len: usize) {
    let mut i = 0;
    while i < len {
        let c = line[i];

        // комментарий // - до конца строки
        if c == b'/' && i + 1 < len && line[i + 1] == b'/' {
            framebuffer::set_color(COL_COMMENT);
            while i < len {
                print!("{}", line[i] as char);
                i += 1;
            }
            break;
        }

        // строка в двойных кавычках
        if c == b'"' {
            framebuffer::set_color(COL_STRING);
            print!("{}", c as char);
            i += 1;
            while i < len {
                let ch = line[i];
                print!("{}", ch as char);
                i += 1;
                if ch == b'"' {
                    break; // закрывающая кавычка (без учёта экранирования - упрощение)
                }
            }
            continue;
        }

        // символ в одинарных кавычках x
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

        // число
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

        // идентификатор или ключевое слово
        if is_ident_char(c) {
            let start = i;
            while i < len && is_ident_char(line[i]) {
                i += 1;
            }
            let word = &line[start..i];
            let color = if is_keyword(word) {
                COL_KEYWORD
            } else if word[0] >= b'A' && word[0] <= b'Z' {
                COL_TYPE // тип или enum-вариант
            } else {
                COL_NORMAL
            };
            framebuffer::set_color(color);
            for &b in word {
                print!("{}", b as char);
            }
            continue;
        }

        // всё остальное белым
        framebuffer::set_color(COL_NORMAL);
        print!("{}", c as char);
        i += 1;
    }
    framebuffer::set_color(WHITE);
}
