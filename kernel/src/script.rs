use alloc::vec::Vec;
use alloc::string::String;
use crate::fs::{self, FILE_MAX_BYTES};
use crate::print_color;
use crate::framebuffer::{GREEN, RED, YELLOW, CYAN};

// одна переменная: имя и значение
struct Var {
    name: String,
    value: i64,
}

// состояние интерпретатора: список переменных и флаг ошибки
struct Interp {
    vars: Vec<Var>,
    error: Option<&'static str>, // если что-то сломалось — тут текст ошибки
}

impl Interp {
    fn new() -> Self {
        Interp { vars: Vec::new(), error: None }
    }

    // найти значение переменной по имени
    fn get_var(&self, name: &str) -> Option<i64> {
        for v in &self.vars {
            if v.name == name {
                return Some(v.value);
            }
        }
        None
    }

    // задать переменную (обновить существующую или добавить новую)
    fn set_var(&mut self, name: &str, value: i64) {
        for v in &mut self.vars {
            if v.name == name {
                v.value = value; // уже есть — обновляем
                return;
            }
        }
        self.vars.push(Var { name: String::from(name), value }); // нет — добавляем
    }

    // выполнить одну строку скрипта
    fn exec_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return; // пусто или комментарий — пропускаем
        }

        // строка вида "let x = ..."
        if let Some(rest) = line.strip_prefix("let ") {
            if let Some(eq) = rest.find('=') {
                let name = rest[..eq].trim();       // до "=" — имя
                let expr = rest[eq + 1..].trim();   // после "=" — выражение
                if !is_valid_name(name) {
                    self.error = Some("invalid variable name");
                    return;
                }
                let val = self.eval(expr);          // вычисляем правую часть
                if self.error.is_some() {
                    return;
                }
                self.set_var(name, val);            // сохраняем в переменную
            } else {
                self.error = Some("let without =");
            }
        }
        // строка вида "print ..."
        else if let Some(expr) = line.strip_prefix("print ") {
            let val = self.eval(expr.trim());
            if self.error.is_some() {
                return;
            }
            print_color!(CYAN, "{}\n", val);
        } else {
            self.error = Some("unknown statement");
        }
    }

    // вычислить выражение (точка входа парсера)
    fn eval(&mut self, expr: &str) -> i64 {
        let tokens = tokenize(expr); // шаг 1: строка -> токены
        if tokens.is_empty() {
            self.error = Some("empty expression");
            return 0;
        }
        let mut pos = 0; // текущая позиция в списке токенов (общий "курсор")
        let result = self.parse_add_sub(&tokens, &mut pos); // шаг 2: парсим
        // если после разбора остались лишние токены — что-то не так
        if pos != tokens.len() && self.error.is_none() {
            self.error = Some("unexpected tokens");
        }
        result
    }

    // уровень сложения/вычитания (НИЗШИЙ приоритет)
    // Сначала берёт левый операнд через уровень выше (mul_div), потом, пока
    // видит + или -, добавляет/вычитает следующие операнды
    fn parse_add_sub(&mut self, tokens: &[Token], pos: &mut usize) -> i64 {
        let mut left = self.parse_mul_div(tokens, pos); // левая часть (уже с учётом * /)
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
                _ => break, // не + и не - — этот уровень закончился
            }
        }
        left
    }

    // уровень умножения/деления (ВЫШЕ приоритетом)
    // Аналогично, но операнды берёт из parse_atom (число/скобка)
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

    // самый нижний уровень: одно "значение"
    // Это число, переменная, выражение в скобках или унарный минус
    fn parse_atom(&mut self, tokens: &[Token], pos: &mut usize) -> i64 {
        if *pos >= tokens.len() {
            self.error = Some("expected value");
            return 0;
        }
        match &tokens[*pos] {
            Token::Num(n) => {
                *pos += 1;
                *n // просто число
            }
            Token::Ident(name) => {
                *pos += 1;
                match self.get_var(name) { // значение переменной
                    Some(v) => v,
                    None => {
                        self.error = Some("undefined variable");
                        0
                    }
                }
            }
            Token::LParen => {
                *pos += 1;
                // Внутри скобок — снова полное выражение (рекурсия на верхний уровень)
                let val = self.parse_add_sub(tokens, pos);
                // ждём закрывающую скобку
                if *pos < tokens.len() && matches!(tokens[*pos], Token::RParen) {
                    *pos += 1;
                } else {
                    self.error = Some("missing )");
                }
                val
            }
            Token::Minus => {
                // унарный минус: -5, -(2+3)
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

// "кусочек" выражения. enum перечисляет все возможные виды токенов
#[derive(Clone)]
enum Token {
    Num(i64),      // число
    Ident(String), // имя переменной
    Plus,          // +
    Minus,         // -
    Star,          // *
    Slash,         // /
    LParen,        // (
    RParen,        // )
}

// превратить строку в список токенов (шаг 1 парсинга)
fn tokenize(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b' ' | b'\t' => i += 1, // пробелы пропускаем
            b'+' => { tokens.push(Token::Plus); i += 1; }
            b'-' => { tokens.push(Token::Minus); i += 1; }
            b'*' => { tokens.push(Token::Star); i += 1; }
            b'/' => { tokens.push(Token::Slash); i += 1; }
            b'(' => { tokens.push(Token::LParen); i += 1; }
            b')' => { tokens.push(Token::RParen); i += 1; }
            b'0'..=b'9' => {
                // читаем число целиком (несколько цифр подряд)
                let mut n: i64 = 0;
                while i < bytes.len() && bytes[i] >= b'0' && bytes[i] <= b'9' {
                    n = n * 10 + (bytes[i] - b'0') as i64; // накапливаем число
                    i += 1;
                }
                tokens.push(Token::Num(n));
            }
            _ if is_ident_start(c) => {
                // читаем имя переменной целиком
                let start = i;
                while i < bytes.len() && is_ident_char(bytes[i]) {
                    i += 1;
                }
                if let Ok(name) = core::str::from_utf8(&bytes[start..i]) {
                    tokens.push(Token::Ident(String::from(name)));
                }
            }
            _ => i += 1, // неизвестный символ — пропускаем
        }
    }
    tokens
}

fn is_ident_start(c: u8) -> bool {
    // имя начинается с буквы или _
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || c == b'_'
}

fn is_ident_char(c: u8) -> bool {
    // внутри имени можно ещё и цифры
    is_ident_start(c) || (c >= b'0' && c <= b'9')
}

fn is_valid_name(name: &str) -> bool {
    // непустое, все символы допустимы, первый — не цифра
    !name.is_empty() && name.bytes().all(|c| is_ident_char(c)) && is_ident_start(name.as_bytes()[0])
}

// вычислить ОДНО выражение (для команды calc в shell)
// Переиспользует тот же парсер, но без переменных
pub fn eval_expr(expr: &str) -> Result<i64, &'static str> {
    let mut interp = Interp::new();
    let val = interp.eval(expr);
    match interp.error {
        Some(e) => Err(e),
        None => Ok(val),
    }
}

// выполнить скрипт из файла (команда run)
pub fn run_file(name: &str) {
    // читаем файл
    let mut buf = [0u8; FILE_MAX_BYTES];
    let size = match fs::read(name, &mut buf) {
        Ok(s) => s,
        Err(e) => { print_color!(RED, "error: {}\n", e); return; }
    };

    // проверяем, что это текст
    let text = match core::str::from_utf8(&buf[..size]) {
        Ok(t) => t,
        Err(_) => { print_color!(RED, "error: not a text file\n"); return; }
    };

    print_color!(YELLOW, "running {}...\n", name);
    let mut interp = Interp::new();
    let mut line_num = 0;
    // выполняем построчно; при ошибке — сообщаем номер строки и стоп
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
