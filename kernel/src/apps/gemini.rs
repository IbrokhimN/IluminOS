// amfora-style console client for the gemini protocol.
//
// gemtext rendering is split behind the `GemtextRenderer` trait so a future
// gui widget can plug in its own renderer (or just call
// `gui::gemtext::to_blocks()` and reuse the existing html block-drawing
// code — see that function's doc comment) without touching navigation,
// history, input prompts, or the network code in `drivers::net::gemini_proto`.
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::framebuffer::{self, CYAN, GRAY, GREEN, MAGENTA, RED, WHITE, YELLOW};
use crate::gui::gemtext::{self, GemDoc, LineKind};
use crate::keyboard::{self, KEY_BACKSPACE, KEY_ENTER};
use crate::print_color;
use crate::tcp::gemini_proto::{self as proto, GeminiError, GeminiResponse, GeminiUrl};
use crate::{print, println};

const MAX_REDIRECTS: u8 = 5;

/// the rendering hook: anything that can draw a parsed gemtext document and
/// show a one-line status message. `ConsoleRenderer` is the only
/// implementation today; a gui browser would provide another one.
pub trait GemtextRenderer {
    fn render(&mut self, doc: &GemDoc);
    fn status_line(&mut self, text: &str, ok: bool);
}

/// one already-wrapped, already-colored row of the pager's line buffer.
struct Row {
    color: u32,
    text: String,
}

/// console renderer with a built-in pager: instead of printing the whole
/// (possibly very long) page straight to the terminal and relying on it to
/// auto-scroll — which loses everything that scrolled off the top — the
/// full page is laid out into `rows` once, and only a `rows()`-sized
/// viewport of it is drawn at a time. `run()` below drives `scroll_up` /
/// `scroll_down` from the keyboard.
pub struct ConsoleRenderer {
    title: String,
    rows: Vec<Row>,
    scroll: usize,
}

impl ConsoleRenderer {
    pub fn new() -> Self {
        ConsoleRenderer { title: String::new(), rows: Vec::new(), scroll: 0 }
    }

    /// how many rows of content fit on screen: total console rows minus
    /// the title line, a blank line, and the footer/help line.
    fn viewport_rows(&self) -> usize {
        framebuffer::console_rows().saturating_sub(4).max(3)
    }

    fn max_scroll(&self) -> usize {
        self.rows.len().saturating_sub(self.viewport_rows())
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll = core::cmp::min(self.scroll + n, self.max_scroll());
        self.draw();
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
        self.draw();
    }

    /// redraw the current viewport (title + visible slice of `rows` +
    /// footer). called after every scroll or new page.
    pub fn draw(&self) {
        framebuffer::clear();
        print_color!(CYAN, "== {} ==\n\n", self.title);

        let view = self.viewport_rows();
        let end = core::cmp::min(self.scroll + view, self.rows.len());
        for row in &self.rows[self.scroll..end] {
            print_color!(row.color, "{}\n", row.text);
        }

        print_color!(
            GRAY,
            "\n-- lines {}-{}/{} -- [up/down j/k scroll] [space/u page] [n<enter> link] [b back] [g url] [r reload] [q quit] --\n",
            self.scroll + 1,
            end,
            self.rows.len().max(1),
        );
    }
}

impl GemtextRenderer for ConsoleRenderer {
    fn render(&mut self, doc: &GemDoc) {
        self.title = doc.title.clone();
        self.rows.clear();
        self.scroll = 0;

        let cols = framebuffer::console_cols().max(20);
        let mut link_no = 0usize;
        let mut push = |color: u32, text: String| self.rows.push(Row { color, text });

        for l in &doc.lines {
            match l.kind {
                LineKind::Heading1 => push(GREEN, alloc::format!("# {}", l.text)),
                LineKind::Heading2 => push(GREEN, alloc::format!("## {}", l.text)),
                LineKind::Heading3 => push(GREEN, alloc::format!("### {}", l.text)),
                LineKind::ListItem => {
                    for (i, chunk) in wrap(&l.text, cols.saturating_sub(4)).into_iter().enumerate() {
                        let prefix = if i == 0 { "  * " } else { "    " };
                        push(WHITE, alloc::format!("{}{}", prefix, chunk));
                    }
                }
                LineKind::Quote => push(GRAY, alloc::format!("  > {}", l.text)),
                LineKind::PreToggle => {
                    if l.text.is_empty() {
                        push(MAGENTA, String::from("```"));
                    } else {
                        push(MAGENTA, alloc::format!("``` {}", l.text));
                    }
                }
                LineKind::Pre => push(YELLOW, alloc::format!("  {}", l.text)),
                LineKind::Link => {
                    link_no += 1;
                    push(CYAN, alloc::format!("[{}] {}", link_no, l.text));
                }
                LineKind::Text => {
                    if l.text.is_empty() {
                        push(WHITE, String::new());
                    } else {
                        for chunk in wrap(&l.text, cols) {
                            push(WHITE, chunk);
                        }
                    }
                }
            }
        }

        self.draw();
    }

    fn status_line(&mut self, text: &str, ok: bool) {
        if ok {
            print_color!(GREEN, "{}\n", text);
        } else {
            print_color!(RED, "{}\n", text);
        }
    }
}

// simple greedy word wrap for the console
fn wrap(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return alloc::vec![String::new()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.len() + 1 + word.len() <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            out.push(core::mem::take(&mut cur));
            cur.push_str(word);
        }
    }
    if !cur.is_empty() || out.is_empty() {
        out.push(cur);
    }
    out
}

struct Browser {
    history: Vec<String>,
    link_targets: Vec<String>,
    base_url: Option<GeminiUrl>,
}

impl Browser {
    fn new() -> Self {
        Browser { history: Vec::new(), link_targets: Vec::new(), base_url: None }
    }

    fn navigate(&mut self, url: &str, renderer: &mut dyn GemtextRenderer, push_history: bool) {
        self.navigate_inner(url, renderer, push_history, 0);
    }

    fn navigate_inner(&mut self, url: &str, renderer: &mut dyn GemtextRenderer, push_history: bool, depth: u8) {
        if depth > MAX_REDIRECTS {
            renderer.status_line("too many redirects", false);
            return;
        }

        let resolved = match &self.base_url {
            Some(base) if !url.contains("://") => resolve_relative(base, url),
            _ => String::from(url),
        };

        print_color!(GRAY, "\nconnecting to {} ...\n", resolved);

        match proto::fetch(&resolved) {
            Ok((base, resp)) => self.handle_response(base, resp, &resolved, renderer, push_history, depth),
            Err(e) => renderer.status_line(&describe_error(e), false),
        }
    }

    fn handle_response(
        &mut self,
        base: GeminiUrl,
        resp: GeminiResponse,
        resolved: &str,
        renderer: &mut dyn GemtextRenderer,
        push_history: bool,
        depth: u8,
    ) {
        match resp.category() {
            1 => {
                // input requested
                print_color!(YELLOW, "{}\n", resp.meta);
                print_color!(WHITE, "> ");
                let answer = read_line();
                let with_query = alloc::format!("{}?{}", resolved, urlencode(&answer));
                self.navigate_inner(&with_query, renderer, push_history, depth + 1);
            }
            2 => {
                // success
                if push_history {
                    self.push_history(resolved);
                }
                self.base_url = Some(base);
                let body = String::from_utf8_lossy(&resp.body).to_string();
                let doc = gemtext::parse(&body);
                self.link_targets = collect_links(&doc, self.base_url.as_ref().unwrap());
                renderer.render(&doc);
            }
            3 => {
                // redirect
                if resp.meta.is_empty() {
                    renderer.status_line("redirect with no target", false);
                    return;
                }
                print_color!(GRAY, "redirect -> {}\n", resp.meta);
                let target = if resp.meta.contains("://") {
                    resp.meta.clone()
                } else {
                    resolve_relative(&base, &resp.meta)
                };
                self.navigate_inner(&target, renderer, push_history, depth + 1);
            }
            4 | 5 => renderer.status_line(&alloc::format!("error {} {}", resp.status, resp.meta), false),
            6 => renderer.status_line(&alloc::format!("client cert required: {}", resp.meta), false),
            _ => renderer.status_line("unknown status from server", false),
        }
    }

    fn push_history(&mut self, url: &str) {
        if self.history.last().map(String::as_str) != Some(url) {
            self.history.push(String::from(url));
        }
    }

    fn back(&mut self, renderer: &mut dyn GemtextRenderer) {
        if self.history.len() > 1 {
            self.history.pop(); // drop current page
            let prev = self.history.last().cloned().unwrap();
            self.navigate(&prev, renderer, false);
            self.push_history(&prev);
        } else {
            renderer.status_line("no more history", false);
        }
    }
}

fn collect_links(doc: &GemDoc, base: &GeminiUrl) -> Vec<String> {
    let mut out = Vec::new();
    for l in &doc.lines {
        if l.kind == LineKind::Link {
            if l.url.contains("://") {
                out.push(l.url.clone());
            } else {
                out.push(resolve_relative(base, &l.url));
            }
        }
    }
    out
}

pub(crate) fn resolve_relative(base: &GeminiUrl, link: &str) -> String {
    let path = if link.starts_with('/') {
        String::from(link)
    } else {
        let dir = match base.resource.rfind('/') {
            Some(i) => &base.resource[..=i],
            None => "/",
        };
        alloc::format!("{}{}", dir, link)
    };
    let normalized = normalize_path(&path);
    if base.port == proto::DEFAULT_PORT {
        alloc::format!("gemini://{}{}", base.host, normalized)
    } else {
        alloc::format!("gemini://{}:{}{}", base.host, base.port, normalized)
    }
}

// collapse "." and ".." segments in a resource path
fn normalize_path(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                stack.pop();
            }
            s => stack.push(s),
        }
    }
    let mut out = String::from("/");
    for (i, s) in stack.iter().enumerate() {
        if i > 0 {
            out.push('/');
        }
        out.push_str(s);
    }
    out
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&alloc::format!("%{:02X}", b)),
        }
    }
    out
}

pub(crate) fn describe_error(e: GeminiError) -> String {
    match e {
        GeminiError::NoNic => String::from("no network card (see the 'nic' command / QEMU -device rtl8139)"),
        GeminiError::BadUrl => String::from("bad gemini url"),
        GeminiError::DnsFailed => String::from("could not resolve host"),
        GeminiError::Connect => String::from("connection failed"),
        GeminiError::Timeout => String::from("timed out"),
        GeminiError::Tls(msg) => alloc::format!("tls error: {}", msg),
        GeminiError::BadResponse => String::from("malformed gemini response"),
    }
}

// minimal blocking line reader, same key handling style as the shell's
// read_line but without history/tab-completion (not needed here)
fn read_line() -> String {
    let mut buf: Vec<u8> = Vec::new();
    loop {
        if let Some(key) = keyboard::try_read_key() {
            match key {
                KEY_ENTER => {
                    println!();
                    break;
                }
                KEY_BACKSPACE => {
                    if buf.pop().is_some() {
                        print!("{}", 0x08 as char);
                    }
                }
                0x20..=0x7e => {
                    buf.push(key);
                    print!("{}", key as char);
                }
                _ => {}
            }
        } else {
            core::hint::spin_loop();
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

/// entry point, called from the shell's `gemini`/`gem` command.
pub fn run(start_url: Option<&str>) {
    let mut renderer = ConsoleRenderer::new();
    let mut browser = Browser::new();

    match start_url {
        Some(u) => browser.navigate(u, &mut renderer, true),
        None => {
            renderer.draw();
            renderer.status_line("type g to enter a gemini:// url", true);
        }
    }

    // digits typed so far for "go to link N"; any non-digit key clears it
    let mut digits = String::new();

    loop {
        let Some(key) = keyboard::try_read_key() else {
            core::hint::spin_loop();
            continue;
        };

        match key {
            keyboard::KEY_UP => {
                digits.clear();
                renderer.scroll_up(1);
            }
            keyboard::KEY_DOWN => {
                digits.clear();
                renderer.scroll_down(1);
            }
            b'k' if digits.is_empty() => renderer.scroll_up(1),
            b'j' if digits.is_empty() => renderer.scroll_down(1),
            b' ' if digits.is_empty() => {
                let page = renderer.viewport_rows();
                renderer.scroll_down(page);
            }
            b'u' if digits.is_empty() => {
                let page = renderer.viewport_rows();
                renderer.scroll_up(page);
            }
            b'0'..=b'9' => {
                digits.push(key as char);
                print!("{}", key as char);
            }
            KEY_BACKSPACE => {
                if digits.pop().is_some() {
                    print!("{}", 0x08 as char);
                }
            }
            KEY_ENTER => {
                if !digits.is_empty() {
                    if let Ok(n) = digits.parse::<usize>() {
                        if n >= 1 && n <= browser.link_targets.len() {
                            let target = browser.link_targets[n - 1].clone();
                            browser.navigate(&target, &mut renderer, true);
                        } else {
                            renderer.status_line("no such link", false);
                        }
                    }
                    digits.clear();
                }
            }
            b'b' if digits.is_empty() => browser.back(&mut renderer),
            b'r' if digits.is_empty() => {
                if let Some(cur) = browser.history.last().cloned() {
                    browser.navigate(&cur, &mut renderer, false);
                } else {
                    renderer.status_line("nothing to reload", false);
                }
            }
            b'g' if digits.is_empty() => {
                print_color!(WHITE, "\nurl: ");
                let url = read_line();
                if !url.trim().is_empty() {
                    browser.navigate(url.trim(), &mut renderer, true);
                } else {
                    renderer.draw();
                }
            }
            b'q' if digits.is_empty() => break,
            _ => {}
        }
    }

    framebuffer::clear();
    crate::banner::show();
    print_color!(GREEN, "gemini client closed.\n");
}
