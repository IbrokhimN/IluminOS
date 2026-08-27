// интерпретатор мини-языка. читает скрипт и исполняет построчно.
// синтаксис:
//   let <имя> = <выражение>
//   print <выражение>
// выражение: числа, переменные, + - * / со скобками и приоритетом.
use alloc::vec::Vec;
use alloc::string::String;
use crate::fs::{self, FILE_MAX_BYTES};
use crate::{print, println, print_color};
use crate::framebuffer::{GREEN, RED, YELLOW, CYAN};

// одна переменная: имя + значение
struct Var {
    name: String,
    value: i64,
}

struct Interp {
    vars: Vec<Var>,
    error: Option<&'static str>,
}

impl Interp {
    fn new() -> Self {
        Interp {
            vars: Vec::new(),
            error: None,
        }
    }

    fn get_var(&self, name: &str) -> Option<i64> {
        for v in &self.vars {
            if v.name == name {
                return Some(v.value);
            }
        }
        None
    }

    fn set_var(&mut self, name: &str, value: i64) {
        for v in &mut self.vars {
            if v.name == name {
                v.value = value;
                return;
            }
        }
        self.vars.push(Var {
            name: String::from(name),
            value,
        });
    }

    // исполнить одну строку
    fn exec_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return; // пустая строка или комментарий
        }

        if let Some(rest) = line.strip_prefix("let ") {
            // let x = выражение
            if let Some(eq) = rest.find('=') {
                let name = rest[..eq].trim();
                let expr = rest[eq + 1..].trim();
                if !is_valid_name(name) {
                    self.error = Some("invalid variable name");
                    return;
                }
                let val = self.eval(expr);
                if self.error.is_some() {
                    return;
                }
                self.set_var(name, val);
            } else {
                self.error = Some("let without =");
            }
        } else if let Some(expr) = line.strip_prefix("print ") {
            let val = self.eval(expr.trim());
            if self.error.is_some() {
                return;
            }
            print_color!(CYAN, "{}\n", val);
        } else {
            self.error = Some("unknown statement");
        }
    }

    // вычислить выражение (точка входа)
    fn eval(&mut self, expr: &str) -> i64 {
        let tokens = tokenize(expr);
        if tokens.is_empty() {
            self.error = Some("empty expression");
            return 0;
        }
        let mut pos = 0;
        let result = self.parse_add_sub(&tokens, &mut pos);
        if pos != tokens.len() && self.error.is_none() {
            self.error = Some("unexpected tokens");
        }
        result
    }

    // сложение/вычитание (низший приоритет)
    fn parse_add_sub(&mut self, tokens: &[Token], pos: &mut usize) -> i64 {
        let mut left = self.parse_mul_div(tokens, pos);
        while *pos < tokens.len() {
            match &tokens[*pos] {
                Token::Plus => {
                    *pos += 1;
                    left += self.parse_mul_div(tokens, pos);
                }
                Token::Minus => {
                    *pos += 1;
                    left -= self.parse_mul_div(tokens, pos);
                }
                _ => break,
            }
        }
        left
    }

    // умножение/деление высший приоритет
    fn parse_mul_div(&mut self, tokens: &[Token], pos: &mut usize) -> i64 {
        let mut left = self.parse_atom(tokens, pos);
        while *pos < tokens.len() {
            match &tokens[*pos] {
                Token::Star => {
                    *pos += 1;
                    left *= self.parse_atom(tokens, pos);
                }
                Token::Slash => {
                    *pos += 1;
                    let r = self.parse_atom(tokens, pos);
                    if r == 0 {
                        self.error = Some("division by zero");
                        return 0;
                    }
                    left /= r;
                }
                _ => break,
            }
        }
        left
    }

    fn parse_atom(&mut self, tokens: &[Token], pos: &mut usize) -> i64 {
        if *pos >= tokens.len() {
            self.error = Some("expected value");
            return 0;
        }
        match &tokens[*pos] {
            Token::Num(n) => {
                *pos += 1;
                *n
            }
            Token::Ident(name) => {
                *pos += 1;
                match self.get_var(name) {
                    Some(v) => v,
                    None => {
                        self.error = Some("undefined variable");
                        0
                    }
                }
            }
            Token::LParen => {
                *pos += 1;
                let val = self.parse_add_sub(tokens, pos);
                if *pos < tokens.len() && matches!(tokens[*pos], Token::RParen) {
                    *pos += 1;
                } else {
                    self.error = Some("missing )");
                }
                val
            }
            Token::Minus => {
                // унарный минус
                *pos += 1;
                -self.parse_atom(tokens, pos)
            }
            _ => {
                self.error = Some("unexpected token");
                0
            }
        }
    }
}


#[derive(Clone)]
enum Token {
    Num(i64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b' ' | b'\t' => i += 1,
            b'+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            b'-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            b'*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            b'/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            b'(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            b')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            b'0'..=b'9' => {
                let mut n: i64 = 0;
                while i < bytes.len() && bytes[i] >= b'0' && bytes[i] <= b'9' {
                    n = n * 10 + (bytes[i] - b'0') as i64;
                    i += 1;
                }
                tokens.push(Token::Num(n));
            }
            _ if is_ident_start(c) => {
                let start = i;
                while i < bytes.len() && is_ident_char(bytes[i]) {
                    i += 1;
                }
                if let Ok(name) = core::str::from_utf8(&bytes[start..i]) {
                    tokens.push(Token::Ident(String::from(name)));
                }
            }
            _ => i += 1, // неизвестный символ пропускаем
        }
    }
    tokens
}

fn is_ident_start(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || c == b'_'
}

fn is_ident_char(c: u8) -> bool {
    is_ident_start(c) || (c >= b'0' && c <= b'9')
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|c| is_ident_char(c)) && is_ident_start(name.as_bytes()[0])
}

// точка входа: запустить скрипт из файла

pub fn run_file(name: &str) {
    let mut buf = [0u8; FILE_MAX_BYTES];
    let size = match fs::read(name, &mut buf) {
        Ok(s) => s,
        Err(e) => {
            print_color!(RED, "error: {}\n", e);
            return;
        }
    };

    let text = match core::str::from_utf8(&buf[..size]) {
        Ok(t) => t,
        Err(_) => {
            print_color!(RED, "error: not a text file\n");
            return;
        }
    };

    print_color!(YELLOW, "running {}...\n", name);
    let mut interp = Interp::new();
    let mut line_num = 0;
    for line in text.split('\n') {
        line_num += 1;
        interp.exec_line(line);
        if let Some(e) = interp.error {
            print_color!(RED, "error on line {}: {}\n", line_num, e);
            return;
        }
    }
    print_color!(GREEN, "done.\n");
}
