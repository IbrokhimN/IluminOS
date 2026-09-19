// gemtext parser
use alloc::string::String;
use alloc::vec::Vec;
use crate::gui::html::{Block, BlockKind, Document};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Text,
    Link,
    Heading1,
    Heading2,
    Heading3,
    ListItem,
    Quote,
    PreToggle,
    Pre,
}

pub struct GemLine {
    pub kind: LineKind,
    pub text: String,
    pub url: String,
}

pub struct GemDoc {
    pub title: String,
    pub lines: Vec<GemLine>,
}

fn line(kind: LineKind, text: &str) -> GemLine {
    GemLine { kind, text: String::from(text), url: String::new() }
}

// parse text
pub fn parse(src: &str) -> GemDoc {
    let mut lines = Vec::new();
    let mut title = String::new();
    let mut pre = false;

    for raw in src.split('\n') {
        let l = raw.strip_suffix('\r').unwrap_or(raw);

        if let Some(alt) = l.strip_prefix("```") {
            pre = !pre;
            lines.push(line(LineKind::PreToggle, alt.trim()));
            continue;
        }
        if pre {
            lines.push(line(LineKind::Pre, l));
            continue;
        }

        if let Some(rest) = l.strip_prefix("=>") {
            let rest = rest.trim_start();
            let mut parts = rest.splitn(2, char::is_whitespace);
            let url = parts.next().unwrap_or("").trim();
            let label = parts
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(url);
            lines.push(GemLine {
                kind: LineKind::Link,
                text: String::from(label),
                url: String::from(url),
            });
        } else if let Some(rest) = l.strip_prefix("###") {
            lines.push(line(LineKind::Heading3, rest.trim()));
        } else if let Some(rest) = l.strip_prefix("##") {
            lines.push(line(LineKind::Heading2, rest.trim()));
        } else if let Some(rest) = l.strip_prefix('#') {
            let t = rest.trim();
            if title.is_empty() {
                title = String::from(t);
            }
            lines.push(line(LineKind::Heading1, t));
        } else if let Some(rest) = l.strip_prefix("* ") {
            lines.push(line(LineKind::ListItem, rest));
        } else if let Some(rest) = l.strip_prefix('>') {
            lines.push(line(LineKind::Quote, rest.trim_start()));
        } else {
            lines.push(line(LineKind::Text, l));
        }
    }

    if title.is_empty() {
        title = String::from("gemini page");
    }

    GemDoc { title, lines }
}

// convert to blocks
pub fn to_blocks(doc: &GemDoc) -> Document {
    const COL_TEXT: u32 = 0x000000;
    const COL_LINK: u32 = 0x2255CC;
    const COL_CODE: u32 = 0x00AA00;
    const COL_CODE_BG: u32 = 0x202020;
    const COL_QUOTE: u32 = 0x666666;

    let mut blocks = Vec::new();
    for l in &doc.lines {
        if l.kind == LineKind::PreToggle {
            continue;
        }

        let mut b = Block {
            text: l.text.clone(),
            kind: BlockKind::Text,
            scale: 1,
            color: COL_TEXT,
            bg: None,
            underline: false,
            is_link: false,
            href: String::new(),
            indent: 0,
            center: false,
            br_after: true,
            list_num: 0,
        };

        match l.kind {
            LineKind::Heading1 => { b.kind = BlockKind::Heading; b.scale = 3; b.color = 0x000080; }
            LineKind::Heading2 => { b.kind = BlockKind::Heading; b.scale = 2; b.color = 0x0000A0; }
            LineKind::Heading3 => { b.kind = BlockKind::Heading; b.scale = 2; b.color = 0x0000C0; }
            LineKind::ListItem => { b.kind = BlockKind::ListItem; b.indent = 16; }
            LineKind::Quote => { b.kind = BlockKind::Quote; b.color = COL_QUOTE; b.indent = 20; }
            LineKind::Pre => { b.kind = BlockKind::Code; b.color = COL_CODE; b.bg = Some(COL_CODE_BG); }
            LineKind::Link => {
                b.is_link = true;
                b.underline = true;
                b.color = COL_LINK;
                b.href = l.url.clone();
                b.text = alloc::format!("-> {}", l.text);
            }
            LineKind::Text | LineKind::PreToggle => {}
        }

        blocks.push(b);
    }

    Document { title: doc.title.clone(), blocks }
}
