// HTML парсер с расширенным набором тегов.
// поддержка h1 h6 p b i u code a br hr ul ol li blockquote center
// font color title. игнорирует html body head обёртки.
use alloc::vec::Vec;
use alloc::string::String;

// тип блока для рендеринга
#[derive(Clone, Copy, PartialEq)]
pub enum BlockKind {
    Text,      // обычный текст
    Heading,   // заголовок
    ListItem,  // элемент списка с буллетом
    Rule,      // горизонтальная линия hr
    Quote,     // цитата с отступом
    Code,      // код на фоне
}

pub struct Block {
    pub text: String,
    pub kind: BlockKind,
    pub scale: usize,
    pub color: u32,
    pub bg: Option<u32>,    // фон блока если есть
    pub underline: bool,
    pub is_link: bool,
    pub indent: usize,      // отступ слева в пикселях
    pub center: bool,       // центрировать
    pub br_after: bool,
    pub list_num: usize,    // номер для нумерованного списка 0 если буллет
}

impl Block {
    fn default_text() -> Self {
        Block {
            text: String::new(),
            kind: BlockKind::Text,
            scale: 1,
            color: 0x000000,
            bg: None,
            underline: false,
            is_link: false,
            indent: 0,
            center: false,
            br_after: false,
            list_num: 0,
        }
    }
}

// цвета
const COL_TEXT: u32 = 0x000000;
const COL_LINK: u32 = 0x0000EE;
const COL_CODE: u32 = 0x00AA00;
const COL_CODE_BG: u32 = 0x202020;
const COL_QUOTE: u32 = 0x666666;
const COL_BOLD: u32 = 0xAA0000;

// состояние стиля во время парсинга
#[derive(Clone, Copy)]
struct Style {
    scale: usize,
    color: u32,
    underline: bool,
    is_link: bool,
    bold: bool,
    center: bool,
    indent: usize,
    kind: BlockKind,
    bg: Option<u32>,
}

impl Style {
    fn base() -> Self {
        Style {
            scale: 1,
            color: COL_TEXT,
            underline: false,
            is_link: false,
            bold: false,
            center: false,
            indent: 0,
            kind: BlockKind::Text,
            bg: None,
        }
    }
}

// разобрать имя цвета в число
fn color_by_name(name: &str) -> u32 {
    match name.trim() {
        "red" => 0xFF0000,
        "green" => 0x00AA00,
        "blue" => 0x0000FF,
        "yellow" => 0xDDAA00,
        "orange" => 0xFF8800,
        "purple" => 0x8800FF,
        "gray" | "grey" => 0x888888,
        "black" => 0x000000,
        "white" => 0xFFFFFF,
        _ => COL_TEXT,
    }
}

// извлечь значение атрибута color из строки тега типа font color=red
fn extract_color(inner: &str) -> Option<u32> {
    if let Some(pos) = inner.find("color") {
        let rest = &inner[pos + 5..];
        // пропускаем = и кавычки пробелы
        let val: String = rest
            .chars()
            .skip_while(|c| *c == '=' || *c == '"' || *c == '\'' || *c == ' ')
            .take_while(|c| c.is_alphabetic())
            .collect();
        if !val.is_empty() {
            return Some(color_by_name(&val));
        }
    }
    None
}

pub struct Document {
    pub title: String,
    pub blocks: Vec<Block>,
}

pub fn parse(html: &str) -> Document {
    let mut blocks = Vec::new();
    let mut title = String::from("Not-Google");
    let bytes = html.as_bytes();
    let mut i = 0;

    let mut style = Style::base();
    let mut text_buf = String::new();
    let mut in_title = false;
    let mut list_counter = 0usize; // для ol
    let mut ordered = false;

    // сброс накопленного текста в блок
    fn flush(blocks: &mut Vec<Block>, buf: &mut String, style: &Style, br: bool, list_num: usize) {
        let t = buf.trim();
        if !t.is_empty() {
            let mut b = Block::default_text();
            b.text = String::from(t);
            b.kind = style.kind;
            b.scale = style.scale;
            b.color = if style.bold { COL_BOLD } else { style.color };
            b.underline = style.underline;
            b.is_link = style.is_link;
            b.center = style.center;
            b.indent = style.indent;
            b.bg = style.bg;
            b.list_num = list_num;
            b.br_after = br
                || style.kind == BlockKind::Heading
                || style.kind == BlockKind::ListItem
                || style.kind == BlockKind::Quote;
            blocks.push(b);
        }
        buf.clear();
    }

    while i < bytes.len() {
        if bytes[i] == b'<' {
            // сбросить текст перед тегом
            if in_title {
                if let Some(t) = text_buf.trim().get(..) {
                    if !t.is_empty() {
                        title = String::from(t);
                    }
                }
                text_buf.clear();
            } else {
                flush(&mut blocks, &mut text_buf, &style, false, if ordered { list_counter } else { 0 });
            }

            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != b'>' {
                end += 1;
            }
            if let Ok(inner) = core::str::from_utf8(&bytes[start..end]) {
                let inner_trim = inner.trim();
                let closing = inner_trim.starts_with('/');
                let name_part = inner_trim.trim_start_matches('/').trim();
                let tag = name_part.split(|c| c == ' ' || c == '>').next().unwrap_or("").to_lowercase_ascii();

                match tag.as_str() {
                    "h1" => { style = if closing { Style::base() } else { let mut s = Style::base(); s.scale=3; s.color=0x000080; s.kind=BlockKind::Heading; s } }
                    "h2" => { style = if closing { Style::base() } else { let mut s = Style::base(); s.scale=2; s.color=0x0000A0; s.kind=BlockKind::Heading; s } }
                    "h3" => { style = if closing { Style::base() } else { let mut s = Style::base(); s.scale=2; s.color=0x0000C0; s.kind=BlockKind::Heading; s } }
                    "h4" | "h5" | "h6" => { style = if closing { Style::base() } else { let mut s = Style::base(); s.scale=1; s.color=0x0000C0; s.kind=BlockKind::Heading; s.bold=true; s } }
                    "p" => { style = if closing { Style::base() } else { let mut s = Style::base(); s.kind=BlockKind::Text; s } }
                    "b" | "strong" => { style.bold = !closing; }
                    "i" | "em" => { style.color = if closing { COL_TEXT } else { 0x006666 }; }
                    "u" => { style.underline = !closing; }
                    "a" => { if closing { style.is_link=false; style.underline=false; style.color=COL_TEXT; } else { style.is_link=true; style.underline=true; style.color=COL_LINK; } }
                    "code" => { if closing { style.kind=BlockKind::Text; style.color=COL_TEXT; style.bg=None; } else { style.kind=BlockKind::Code; style.color=COL_CODE; style.bg=Some(COL_CODE_BG); } }
                    "center" => { style.center = !closing; }
                    "blockquote" => { if closing { style.kind=BlockKind::Text; style.color=COL_TEXT; style.indent=0; } else { style.kind=BlockKind::Quote; style.color=COL_QUOTE; style.indent=20; } }
                    "font" => { if closing { style.color=COL_TEXT; } else if let Some(col)=extract_color(name_part) { style.color=col; } }
                    "ul" => { ordered = false; if !closing { list_counter=0; } }
                    "ol" => { ordered = !closing; if !closing { list_counter=0; } }
                    "li" => {
                        if !closing {
                            if ordered { list_counter += 1; }
                            style = { let mut s=Style::base(); s.kind=BlockKind::ListItem; s.indent=16; s };
                        } else {
                            style = Style::base();
                        }
                    }
                    "br" => {
                        let mut b = Block::default_text();
                        b.br_after = true;
                        blocks.push(b);
                    }
                    "hr" => {
                        let mut b = Block::default_text();
                        b.kind = BlockKind::Rule;
                        b.br_after = true;
                        blocks.push(b);
                    }
                    "title" => { in_title = !closing; }
                    // игнорируем обёртки
                    "html" | "body" | "head" | "meta" | "div" | "span" => {}
                    _ => {}
                }
            }
            i = end + 1;
        } else {
            text_buf.push(bytes[i] as char);
            i += 1;
        }
    }
    flush(&mut blocks, &mut text_buf, &style, false, if ordered { list_counter } else { 0 });

    Document { title, blocks }
}

// вспомогательный трейт для to_lowercase в no_std на байтах ascii
trait LowerAscii {
    fn to_lowercase_ascii(&self) -> String;
}
impl LowerAscii for str {
    fn to_lowercase_ascii(&self) -> String {
        let mut s = String::new();
        for c in self.chars() {
            if c >= 'A' && c <= 'Z' {
                s.push((c as u8 + 32) as char);
            } else {
                s.push(c);
            }
        }
        s
    }
}
