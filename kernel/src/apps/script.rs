use alloc::vec::Vec;
use alloc::string::String;
use crate::fs::{self, FILE_MAX_BYTES};
use crate::print_color;
use crate::framebuffer::{GREEN, RED, YELLOW, CYAN};

// one variable name and value
struct Var {
    name: String,
    value: i64,
}

// interpreter state variable list and error flag
struct Interp {
    vars: Vec<Var>,
    error: Option<&'static str>, // error text if something broke
}

impl Interp {
    fn new() -> Self {
        Interp { vars: Vec::new(), error: None }
    }

    // find variable value by name
    fn get_var(&self, name: &str) -> Option<i64> {
        for v in &self.vars {
            if v.name == name {
                return Some(v.value);
            }
        }
        None
    }

    // set a variable update existing or add new
    fn set_var(&mut self, name: &str, value: i64) {
        for v in &mut self.vars {
            if v.name == name {
                v.value = value; // already exists update it
                return;
            }
        }
        self.vars.push(Var { name: String::from(name), value }); // not found add it
    }

    // run one script line
    fn exec_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return; // empty or comment skip it
        }

        // line of the form let x equals
        if let Some(rest) = line.strip_prefix("let ") {
            if let Some(eq) = rest.find('=') {
                let name = rest[..eq].trim();       // before equals is the name
                let expr = rest[eq + 1..].trim();   // after equals is the expression
                if !is_valid_name(name) {
                    self.error = Some("invalid variable name");
                    return;
                }
                let val = self.eval(expr);          // evaluate right hand side
                if self.error.is_some() {
                    return;
                }
                self.set_var(name, val);            // store into variable
            } else {
                self.error = Some("let without =");
            }
        }
        // line of the form print
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

    // evaluate expression parser entry point
    fn eval(&mut self, expr: &str) -> i64 {
        let tokens = tokenize(expr); // step 1 string to tokens
        if tokens.is_empty() {
            self.error = Some("empty expression");
            return 0;
        }
        let mut pos = 0; // current position in token list
        let result = self.parse_add_sub(&tokens, &mut pos); // step 2 parse
        // leftover tokens after parsing means something is wrong
        if pos != tokens.len() && self.error.is_none() {
            self.error = Some("unexpected tokens");
        }
        result
    }

    // addition subtraction level lowest precedence
    fn parse_add_sub(&mut self, tokens: &[Token], pos: &mut usize) -> i64 {
        let mut left = self.parse_mul_div(tokens, pos); // left side already handles mul div
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
                _ => break, // not plus or minus this level is done
            }
        }
        left
    }

    // multiplication division level higher precedence
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

    // lowest level a single value number variable parens or unary minus
    fn parse_atom(&mut self, tokens: &[Token], pos: &mut usize) -> i64 {
        if *pos >= tokens.len() {
            self.error = Some("expected value");
            return 0;
        }
        match &tokens[*pos] {
            Token::Num(n) => {
                *pos += 1;
                *n // plain number
            }
            Token::Ident(name) => {
                *pos += 1;
                match self.get_var(name) { // variable value
                    Some(v) => v,
                    None => {
                        self.error = Some("undefined variable");
                        0
                    }
                }
            }
            Token::LParen => {
                *pos += 1;
                // inside parens is a full expression recurse to top level
                let val = self.parse_add_sub(tokens, pos);
                // expect closing paren
                if *pos < tokens.len() && matches!(tokens[*pos], Token::RParen) {
                    *pos += 1;
                } else {
                    self.error = Some("missing )");
                }
                val
            }
            Token::Minus => {
                // unary minus
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

// a piece of an expression enum lists all token kinds
#[derive(Clone)]
enum Token {
    Num(i64),      // number
    Ident(String), // variable name
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

// turn a string into a token list step 1 of parsing
fn tokenize(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b' ' | b'\t' => i += 1, // skip whitespace
            b'+' => { tokens.push(Token::Plus); i += 1; }
            b'-' => { tokens.push(Token::Minus); i += 1; }
            b'*' => { tokens.push(Token::Star); i += 1; }
            b'/' => { tokens.push(Token::Slash); i += 1; }
            b'(' => { tokens.push(Token::LParen); i += 1; }
            b')' => { tokens.push(Token::RParen); i += 1; }
            b'0'..=b'9' => {
                // read the whole number several digits
                let mut n: i64 = 0;
                while i < bytes.len() && bytes[i] >= b'0' && bytes[i] <= b'9' {
                    n = n * 10 + (bytes[i] - b'0') as i64; // accumulate number
                    i += 1;
                }
                tokens.push(Token::Num(n));
            }
            _ if is_ident_start(c) => {
                // read the whole variable name
                let start = i;
                while i < bytes.len() && is_ident_char(bytes[i]) {
                    i += 1;
                }
                if let Ok(name) = core::str::from_utf8(&bytes[start..i]) {
                    tokens.push(Token::Ident(String::from(name)));
                }
            }
            _ => i += 1, // unknown char skip it
        }
    }
    tokens
}

fn is_ident_start(c: u8) -> bool {
    // name starts with a letter or underscore
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || c == b'_'
}

fn is_ident_char(c: u8) -> bool {
    // name can also contain digits
    is_ident_start(c) || (c >= b'0' && c <= b'9')
}

fn is_valid_name(name: &str) -> bool {
    // non empty all chars valid first not a digit
    !name.is_empty() && name.bytes().all(|c| is_ident_char(c)) && is_ident_start(name.as_bytes()[0])
}

// evaluate one expression for the shell calc command no variables
pub fn eval_expr(expr: &str) -> Result<i64, &'static str> {
    let mut interp = Interp::new();
    let val = interp.eval(expr);
    match interp.error {
        Some(e) => Err(e),
        None => Ok(val),
    }
}

// run a script file command run
pub fn run_file(name: &str) {
    // read the file
    let mut buf = [0u8; FILE_MAX_BYTES];
    let size = match fs::read(name, &mut buf) {
        Ok(s) => s,
        Err(e) => { print_color!(RED, "error: {}\n", e); return; }
    };

    // check that it is text
    let text = match core::str::from_utf8(&buf[..size]) {
        Ok(t) => t,
        Err(_) => { print_color!(RED, "error: not a text file\n"); return; }
    };

    print_color!(YELLOW, "running {}...\n", name);
    let mut interp = Interp::new();
    let mut line_num = 0;
    // run line by line report line number and stop on error
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
