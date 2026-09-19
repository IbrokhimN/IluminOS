// "Not-Google" browser widget: fake search, local .html files, and real
// gemini:// capsules, all sharing one `html::Document` render pipeline.
use crate::framebuffer::{self, draw_text_at};
use crate::fs::{self, FILE_MAX_BYTES};
use crate::gui::style::{self, hit};
use crate::html;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_8X13_BOLD, FONT_9X18_BOLD};
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_text::alignment::HorizontalAlignment;
use embedded_text::style::{HeightMode, TextBoxStyleBuilder};
use embedded_text::TextBox;

const LINK: u32 = 0x2255CC;

pub struct Browser {
    pub query: String,
    results: Vec<String>,
    searched: bool,
    pub(crate) viewing_html: bool,
    page_doc: Option<html::Document>,
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    scroll_y: usize,
    content_height: usize,
    search_btn: (i32, i32, i32, i32),
    // clickable link regions from the last render_html() pass, in screen
    // pixel coordinates: (x0, y0, x1, y1, href)
    link_hits: Vec<(i32, i32, i32, i32, String)>,
    // the gemini:// url the current page was loaded from, if any — needed
    // to resolve relative links (`./foo.gmi`, `/bar`) clicked on the page
    current_gemini_url: Option<crate::tcp::gemini_proto::GeminiUrl>,
}

impl Browser {
    pub fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        Browser {
            query: String::new(),
            results: Vec::new(),
            searched: false,
            viewing_html: false,
            page_doc: None,
            cx,
            cy,
            cw,
            ch,
            scroll_y: 0,
            content_height: 0,
            search_btn: (0, 0, 0, 0),
            link_hits: Vec::new(),
            current_gemini_url: None,
        }
    }

    /// scroll the currently viewed page by `delta` pixels (negative = up),
    /// clamped to the document's actual height. called from the window
    /// manager on up/down arrow keys while this window is focused.
    pub fn scroll(&mut self, delta: i32) {
        let viewport = self.ch.saturating_sub(24);
        let max_scroll = self.content_height.saturating_sub(viewport);
        let mut new_scroll = self.scroll_y as i32 + delta;
        if new_scroll < 0 {
            new_scroll = 0;
        }
        let new_scroll = new_scroll as usize;
        self.scroll_y = core::cmp::min(new_scroll, max_scroll);
        self.redraw();
    }

    pub fn redraw(&mut self) {
        if self.viewing_html {
            self.render_html();
            return;
        }

        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, 0xFFFFFF);
        self.draw_logo();
        self.draw_search_box();

        if self.searched {
            self.draw_results();
        } else {
            style::text(
                "search, or type a page.html to open a file",
                self.cx as i32 + 30,
                self.cy as i32 + 80,
                style::TEXT_DIM,
            );
        }
    }

    fn draw_logo(&self) {
        let logo = "Not-Google";
        let colors = [0x4488FF, 0xE67E80, 0xDBBC7F, 0x4488FF, 0xA7C080, 0xE67E80];
        let logo_x = self.cx as i32 + self.cw as i32 / 2 - style::text_width(logo) / 2;
        let logo_y = self.cy as i32 + 30;
        for (i, ch) in logo.chars().enumerate() {
            let mut buf = [0u8; 4];
            style::text(
                ch.encode_utf8(&mut buf),
                logo_x + i as i32 * 6,
                logo_y,
                colors[i % colors.len()],
            );
        }
    }

    fn draw_search_box(&mut self) {
        let box_y = self.cy as i32 + 54;
        let box_x = self.cx as i32 + 30;
        let box_w = self.cw as i32 - 130;

        style::rounded_fill(box_x, box_y, box_w as u32, 22, 0xFFFFFF);
        style::rounded_outline(box_x, box_y, box_w as u32, 22, style::BORDER);
        style::text(&self.query, box_x + 6, box_y + 6, 0x000000);

        let btn_x = box_x + box_w + 8;
        style::button(btn_x, box_y, 70, 22, "Search");
        self.search_btn = (btn_x, box_y, 70, 22);
    }

    fn draw_results(&self) {
        let mut y = self.cy as i32 + 96;
        style::text("results for", self.cx as i32 + 10, y, style::TEXT_DIM);
        style::text(&self.query, self.cx as i32 + 10 + 70, y, 0x000000);
        y += 18;
        for r in &self.results {
            style::text(r, self.cx as i32 + 10, y, LINK);
            y += 16;
        }
    }

    pub fn do_search(&mut self) {
        let q = self.query.trim();
        if q.is_empty() {
            return;
        }

        if q.starts_with("gemini://") {
            let url = String::from(q);
            self.open_gemini(&url);
            return;
        }

        if q.ends_with(".html") || q.ends_with(".htm") {
            let name = String::from(q);
            self.open_html(&name);
            return;
        }

        self.viewing_html = false;
        self.scroll_y = 0;
        self.results = alloc::vec![
            alloc::format!("www.{}.com - official site", q),
            alloc::format!("en.notpedia.org/wiki/{}", q),
            alloc::format!("{} - news and updates", q),
            alloc::format!("shop.notzone.com/search?q={}", q),
            alloc::format!("How to learn {} - tutorial", q),
        ];
        self.searched = true;
    }

    pub fn open_html(&mut self, name: &str) {
        let mut buf = [0u8; FILE_MAX_BYTES];
        self.page_doc = match fs::read(name, &mut buf) {
            Ok(size) => core::str::from_utf8(&buf[..size]).ok().map(html::parse),
            Err(_) => None,
        };
        self.viewing_html = true;
        self.scroll_y = 0;
        self.current_gemini_url = None;
    }

    /// fetch and render a gemini:// url. reuses the exact same `page_doc` /
    /// `render_html()` pipeline as local html files: `gemtext::to_blocks()`
    /// (see its doc comment) turns the parsed gemtext into the same `Block`
    /// list the html renderer already knows how to draw, so no separate
    /// gemini-specific drawing code is needed here at all — this is the gui
    /// "hook" the console client (`apps::gemini`) was built to leave open.
    pub fn open_gemini(&mut self, url: &str) {
        self.open_gemini_inner(url, 0);
        self.viewing_html = true;
        self.scroll_y = 0;
    }

    /// follow a link clicked on the currently rendered page. absolute
    /// gemini:// links and local .html/.htm files are handled directly;
    /// relative links (bare "foo.gmi", "/bar", "./baz") are resolved
    /// against `current_gemini_url`, same as the console client does.
    /// anything else (http(s)://, mailto:, unknown schemes) just shows a
    /// message — this toy browser has no real internet beyond gemini and
    /// the local filesystem.
    pub fn navigate_link(&mut self, href: &str) {
        if href.is_empty() {
            return;
        }

        if href.starts_with("gemini://") {
            self.open_gemini(href);
            return;
        }

        if href.contains("://") {
            self.show_message(&alloc::format!("can't open this kind of link here: {}", href));
            return;
        }

        if let Some(base) = self.current_gemini_url.clone() {
            let target = crate::apps::gemini::resolve_relative(&base, href);
            self.open_gemini(&target);
        } else if href.ends_with(".html") || href.ends_with(".htm") {
            let name = String::from(href);
            self.open_html(&name);
        } else {
            self.show_message(&alloc::format!("can't open: {}", href));
        }
    }

    fn show_message(&mut self, msg: &str) {
        self.page_doc = Some(gemini_message_doc(msg));
        self.viewing_html = true;
        self.scroll_y = 0;
    }

    fn open_gemini_inner(&mut self, url: &str, depth: u8) {
        if depth > 5 {
            self.page_doc = Some(gemini_message_doc("too many redirects"));
            return;
        }

        match crate::tcp::gemini_proto::fetch(url) {
            Ok((base, resp)) => match resp.category() {
                2 => {
                    let body = String::from_utf8_lossy(&resp.body).to_string();
                    let doc = crate::gemtext::parse(&body);
                    self.page_doc = Some(crate::gemtext::to_blocks(&doc));
                    self.current_gemini_url = Some(base);
                }
                3 => {
                    let target = if resp.meta.contains("://") {
                        resp.meta.clone()
                    } else if resp.meta.starts_with('/') {
                        alloc::format!("gemini://{}{}", base.host, resp.meta)
                    } else {
                        alloc::format!("gemini://{}/{}", base.host, resp.meta)
                    };
                    self.open_gemini_inner(&target, depth + 1);
                }
                1 => {
                    self.page_doc = Some(gemini_message_doc(&alloc::format!(
                        "this capsule wants input ({}), which the gui browser can't prompt for yet — try the console gemini client instead",
                        resp.meta
                    )));
                }
                _ => {
                    self.page_doc = Some(gemini_message_doc(&alloc::format!("{} {}", resp.status, resp.meta)));
                }
            },
            Err(e) => {
                self.page_doc = Some(gemini_message_doc(&crate::apps::gemini::describe_error(e)));
            }
        }
    }

    pub fn render_html(&mut self) {
        framebuffer::fill_rect(self.cx, self.cy, self.cw, self.ch, 0xFFFFFF);
        draw_text_at(
            "Not-Google viewer",
            self.cx + 4,
            self.cy + 4,
            style::TEXT_DIM,
        );

        let Some(doc) = &self.page_doc else {
            draw_text_at(
                "page not found or empty",
                self.cx + 10,
                self.cy + 30,
                0xAA0000,
            );
            self.content_height = 0;
            return;
        };

        let left = self.cx + 8;
        let right_limit = self.cx + self.cw - 8;
        let top = self.cy + 20;
        let bottom = self.cy + self.ch;
        let scroll = self.scroll_y;

        // clip everything below to this window's content rect, so long
        // pages can't bleed past its edges onto whatever's behind/below it —
        // this is what makes scrolling actually safe to draw.
        framebuffer::set_clip(self.cx as i32, top as i32, (self.cx + self.cw) as i32, bottom as i32);

        let mut y = top;
        self.link_hits.clear();

        for block in &doc.blocks {
            if block.kind == html::BlockKind::Rule {
                if y + 4 >= scroll {
                    framebuffer::fill_rect(left, y + 4 - scroll, self.cw - 16, 2, style::BORDER);
                }
                y += 12;
                continue;
            }
            if block.text.is_empty() {
                y += 12;
                continue;
            }

            let bx = left + block.indent;
            let mut start_x = bx;

            if block.kind == html::BlockKind::ListItem {
                if block.list_num > 0 {
                    if y >= scroll {
                        draw_text_at(&alloc::format!("{}.", block.list_num), bx, y - scroll, 0x000000);
                    }
                    start_x = bx + 24;
                } else {
                    if y + 3 >= scroll {
                        framebuffer::fill_rect(bx, y + 3 - scroll, 4, 4, 0x000000);
                    }
                    start_x = bx + 12;
                }
            }

            y = draw_wrapped_block(block, start_x, y, scroll, right_limit, &mut self.link_hits);
        }

        framebuffer::clear_clip();
        self.content_height = y.saturating_sub(top);
    }

    pub fn window_title(&self) -> &str {
        if self.viewing_html {
            if let Some(doc) = &self.page_doc {
                return &doc.title;
            }
        }
        "Not-Google"
    }

    pub fn search_btn_hit(&self, mx: i32, my: i32) -> bool {
        let (x, y, w, h) = self.search_btn;
        hit(mx, my, x, y, w as u32, h as u32)
    }

    /// hit-test the last rendered page's link regions; returns the href of
    /// whichever link is under (mx, my), if any.
    pub fn link_at(&self, mx: i32, my: i32) -> Option<String> {
        self.link_hits
            .iter()
            .find(|(x0, y0, x1, y1, _)| mx >= *x0 && mx < *x1 && my >= *y0 && my < *y1)
            .map(|(_, _, _, _, href)| href.clone())
    }
}

fn gemini_message_doc(msg: &str) -> html::Document {
    let block = html::Block {
        text: String::from(msg),
        kind: html::BlockKind::Text,
        scale: 1,
        color: 0xAA0000,
        bg: None,
        underline: false,
        is_link: false,
        href: String::new(),
        indent: 0,
        center: false,
        br_after: true,
        list_num: 0,
    };
    html::Document { title: String::from("gemini"), blocks: alloc::vec![block] }
}

fn u32_to_rgb888(c: u32) -> Rgb888 {
    framebuffer::u32_to_rgb888(c)
}

// word-wrapping + drawing for html/gemini text blocks, via the `embedded-text`
// crate (TextBox) instead of a hand-rolled wrap loop. `framebuffer::Display`
// already implements `embedded_graphics::DrawTarget`, so this plugs straight
// in. font is picked from the block's heading scale; color/background/
// underline map onto `MonoTextStyleBuilder`, and `HeightMode::FitToText`
// tells the box to measure its own wrapped height so blocks can still be
// stacked vertically like before. `scroll` shifts the drawn position (can go
// negative on screen — `Display` and the clip rect set by the caller both
// silently discard anything outside the window, so there's no need to
// special-case blocks scrolled off-screen here).
fn draw_wrapped_block(
    block: &html::Block,
    start_x: usize,
    content_y: usize,
    scroll: usize,
    right_limit: usize,
    link_hits: &mut Vec<(i32, i32, i32, i32, String)>,
) -> usize {
    if block.text.is_empty() {
        return content_y + 12;
    }

    let font = match block.scale {
        1 => &FONT_6X10,
        2 => &FONT_8X13_BOLD,
        _ => &FONT_9X18_BOLD,
    };

    let mut style_builder = MonoTextStyleBuilder::new().font(font).text_color(u32_to_rgb888(block.color));
    if block.underline || block.is_link {
        style_builder = style_builder.underline_with_color(u32_to_rgb888(block.color));
    }
    if let Some(bg) = block.bg {
        style_builder = style_builder.background_color(u32_to_rgb888(bg));
    }
    let character_style = style_builder.build();

    let textbox_style = TextBoxStyleBuilder::new()
        .height_mode(HeightMode::FitToText)
        .alignment(if block.center { HorizontalAlignment::Center } else { HorizontalAlignment::Left })
        .build();

    let screen_y = content_y as i32 - scroll as i32;
    let width = right_limit.saturating_sub(start_x) as u32;
    let bounds = Rectangle::new(Point::new(start_x as i32, screen_y), Size::new(width, 0));

    let text_box = TextBox::with_textbox_style(&block.text, bounds, character_style, textbox_style);
    let box_bounds = text_box.bounding_box();
    let height = box_bounds.size.height;
    let _ = text_box.draw(&mut framebuffer::Display);

    if block.is_link && !block.href.is_empty() {
        link_hits.push((
            box_bounds.top_left.x,
            box_bounds.top_left.y,
            box_bounds.top_left.x + box_bounds.size.width as i32,
            box_bounds.top_left.y + box_bounds.size.height as i32,
            block.href.clone(),
        ));
    }

    content_y + height as usize + 4
}
