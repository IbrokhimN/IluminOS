use crate::framebuffer::{self, draw_text_at, fill_rect, draw_rect};
use crate::keyboard::{self, KEY_BACKSPACE, KEY_ENTER};
use crate::mouse;
use crate::apps::{Clock, Calc, Paint};
use crate::fs::{self, FILE_MAX_BYTES};
use crate::html;
use crate::framebuffer::draw_text_scaled;
use alloc::vec::Vec;
use alloc::string::String;

// цвета win 3.1
const DESKTOP: u32 = 0x008080;
const WIN_FACE: u32 = 0xC0C0C0;
const WIN_LIGHT: u32 = 0xFFFFFF;
const WIN_DARK: u32 = 0x808080;
const TITLE_BG: u32 = 0x000080;
const TITLE_FG: u32 = 0xFFFFFF;
const BLACK: u32 = 0x000000;
const TERM_BG: u32 = 0x000000;
const TERM_FG: u32 = 0x00FF00;
const NG_BLUE: u32 = 0x4488FF;
const NG_RED: u32 = 0xFF4444;
const NG_YELLOW: u32 = 0xFFDD33;
const NG_GREEN: u32 = 0x33CC66;
const LINK: u32 = 0x0000EE;

// какое приложение сейчас открыто
#[derive(PartialEq, Clone, Copy)]
enum App {
    Desktop,
    Terminal,
    Browser,
    Clock,
    Calc,
    Paint,
}

// объёмная грань win 3.1
fn bevel(x: usize, y: usize, w: usize, h: usize, raised: bool) {
    let (tl, br) = if raised { (WIN_LIGHT, WIN_DARK) } else { (WIN_DARK, WIN_LIGHT) };
    fill_rect(x, y, w, 2, tl);
    fill_rect(x, y, 2, h, tl);
    fill_rect(x, y + h - 2, w, 2, br);
    fill_rect(x + w - 2, y, 2, h, br);
}

// прямоугольник кнопки закрытия окна. возвращает координаты для клика
struct CloseBtn {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl CloseBtn {
    fn contains(&self, mx: i32, my: i32) -> bool {
        mx >= self.x as i32
            && mx < (self.x + self.w) as i32
            && my >= self.y as i32
            && my < (self.y + self.h) as i32
    }
}

// нарисовать окно вернуть область содержимого и кнопку закрытия
fn draw_window(x: usize, y: usize, w: usize, h: usize, title: &str) -> (usize, usize, usize, usize, CloseBtn) {
    fill_rect(x, y, w, h, WIN_FACE);
    bevel(x, y, w, h, true);
    draw_rect(x, y, w, h, BLACK);

    let title_h = 18;
    fill_rect(x + 3, y + 3, w - 6, title_h, TITLE_BG);
    draw_text_at(title, x + 7, y + 3 + 5, TITLE_FG);

    // кнопка закрытия справа
    let bx = x + w - 20;
    let by = y + 5;
    fill_rect(bx, by, 14, 14, WIN_FACE);
    bevel(bx, by, 14, 14, true);
    draw_text_at("x", bx + 4, by + 3, BLACK);

    let cx = x + 4;
    let cy = y + 3 + title_h + 2;
    let cw = w - 8;
    let ch = h - (3 + title_h + 2) - 4;
    (cx, cy, cw, ch, CloseBtn { x: bx, y: by, w: 14, h: 14 })
}

// иконка на рабочем столе
struct Icon {
    x: usize,
    y: usize,
    label: &'static str,
    app: App,
}

impl Icon {
    // попадание клика в иконку область 64x64
    fn contains(&self, mx: i32, my: i32) -> bool {
        mx >= self.x as i32
            && mx < (self.x + 64) as i32
            && my >= self.y as i32
            && my < (self.y + 72) as i32
    }

    fn draw(&self) {
        // квадрат иконки 40x40 по центру
        let ix = self.x + 12;
        let iy = self.y;
        fill_rect(ix, iy, 40, 40, WIN_FACE);
        bevel(ix, iy, 40, 40, true);
        // рисунок внутри разный для приложений
        match self.app {
            App::Terminal => {
                fill_rect(ix + 6, iy + 6, 28, 28, BLACK);
                draw_text_at(">", ix + 10, iy + 14, TERM_FG);
                draw_text_at("_", ix + 18, iy + 16, TERM_FG);
            }
            App::Browser => {
                // цветной кружок как логотип
                fill_rect(ix + 8, iy + 8, 24, 24, NG_BLUE);
                fill_rect(ix + 14, iy + 14, 12, 12, WIN_FACE);
            }
            App::Clock => {
                // циферблат белый круг со стрелками
                fill_rect(ix + 6, iy + 6, 28, 28, 0xFFFFFF);
                draw_rect(ix + 6, iy + 6, 28, 28, BLACK);
                fill_rect(ix + 19, iy + 12, 2, 9, BLACK);
                fill_rect(ix + 20, iy + 19, 8, 2, BLACK);
            }
            App::Calc => {
                // калькулятор экран и кнопки
                fill_rect(ix + 6, iy + 6, 28, 10, 0x203020);
                fill_rect(ix + 8, iy + 20, 6, 6, BLACK);
                fill_rect(ix + 17, iy + 20, 6, 6, BLACK);
                fill_rect(ix + 26, iy + 20, 6, 6, BLACK);
            }
            App::Paint => {
                // палитра цветные квадратики
                fill_rect(ix + 6, iy + 6, 12, 12, 0xFF0000);
                fill_rect(ix + 22, iy + 6, 12, 12, 0x0000FF);
                fill_rect(ix + 6, iy + 22, 12, 12, 0x00AA00);
                fill_rect(ix + 22, iy + 22, 12, 12, 0xFFDD00);
            }
            _ => {}
        }
        // подпись под иконкой
        draw_text_at(self.label, self.x + 4, self.y + 44, WIN_LIGHT);
    }
}

// рабочий стол с иконками
fn draw_desktop(icons: &[Icon]) {
    let (w, h) = framebuffer::dimensions();
    fill_rect(0, 0, w, h, DESKTOP);
    for icon in icons {
        icon.draw();
    }
    // панель задач
    fill_rect(0, h - 24, w, 24, WIN_FACE);
    bevel(0, h - 24, w, 24, true);
    draw_text_at("IluminOS", 8, h - 24 + 8, BLACK);
}

struct Term {
    lines: Vec<String>,
    input: String,
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
}

impl Term {
    fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let mut lines = Vec::new();
        lines.push(String::from("IluminOS terminal"));
        lines.push(String::from("commands help ver clear"));
        lines.push(String::from(""));
        Term { lines, input: String::new(), cx, cy, cw, ch }
    }

    fn redraw(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, TERM_BG);
        let max_rows = (self.ch / 10).max(2);
        let visible = max_rows - 1;
        let start = if self.lines.len() > visible { self.lines.len() - visible } else { 0 };
        let mut row = 0;
        for line in &self.lines[start..] {
            draw_text_at(line, self.cx + 2, self.cy + 2 + row * 10, TERM_FG);
            row += 1;
        }
        draw_text_at(">", self.cx + 2, self.cy + 2 + row * 10, TERM_FG);
        draw_text_at(&self.input, self.cx + 2 + 8, self.cy + 2 + row * 10, TERM_FG);
    }

    fn exec(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        let mut echo = String::from("> ");
        echo.push_str(cmd);
        self.lines.push(echo);
        match cmd {
            "" => {}
            "help" => self.lines.push(String::from("commands help ver clear")),
            "ver" => self.lines.push(String::from("IluminOS 1.0 GUI")),
            "clear" => self.lines.clear(),
            _ => {
                let mut e = String::from("unknown ");
                e.push_str(cmd);
                self.lines.push(e);
            }
        }
    }
}

// Not google
struct Browser {
    query: String,
    results: Vec<String>,
    searched: bool,
    viewing_html: bool,      // режим просмотра html страницы
    page_doc: Option<html::Document>, // распарсенный документ
    cx: usize,
    cy: usize,
    cw: usize,
    ch: usize,
    search_btn: (usize, usize, usize, usize),
}

impl Browser {
    fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        Browser {
            query: String::new(),
            results: Vec::new(),
            searched: false,
            viewing_html: false,
            page_doc: None,
            cx, cy, cw, ch,
            search_btn: (0, 0, 0, 0),
        }
    }

    fn redraw(&mut self) {
        // режим просмотра html страницы
        if self.viewing_html {
            self.render_html();
            return;
        }
        // белый фон страницы
        fill_rect(self.cx, self.cy, self.cw, self.ch, 0xFFFFFF);

        // логотип Not-Google по центру разноцветный
        let logo_y = self.cy + 30;
        let cx_center = self.cx + self.cw / 2;
        let logo = "Not-Google";
        let logo_x = cx_center - (logo.len() * 8) / 2;
        // рисуем по буквам чередуя цвета гугла
        let colors = [NG_BLUE, NG_RED, NG_YELLOW, NG_BLUE, NG_GREEN, NG_RED];
        let mut lx = logo_x;
        for (i, ch) in logo.bytes().enumerate() {
            let color = colors[i % colors.len()];
            let s = [ch];
            if let Ok(st) = core::str::from_utf8(&s) {
                draw_text_at(st, lx, logo_y, color);
            }
            lx += 8;
        }

        // строка поиска
        let box_y = logo_y + 24;
        let box_x = self.cx + 30;
        let box_w = self.cw - 130;
        fill_rect(box_x, box_y, box_w, 20, 0xFFFFFF);
        draw_rect(box_x, box_y, box_w, 20, WIN_DARK);
        draw_text_at(&self.query, box_x + 4, box_y + 6, BLACK);

        // кнопка Search
        let btn_x = box_x + box_w + 8;
        let btn_w = 70;
        fill_rect(btn_x, box_y, btn_w, 20, WIN_FACE);
        bevel(btn_x, box_y, btn_w, 20, true);
        draw_text_at("Search", btn_x + 8, box_y + 6, BLACK);
        self.search_btn = (btn_x, box_y, btn_w, 20);

        // результаты
        if self.searched {
            let mut ry = box_y + 40;
            draw_text_at("results for", self.cx + 10, ry, WIN_DARK);
            draw_text_at(&self.query, self.cx + 10 + 96, ry, BLACK);
            ry += 16;
            for r in &self.results {
                draw_text_at(r, self.cx + 10, ry, LINK);
                ry += 14;
            }
        } else {
            draw_text_at("search, or type page.html to open a file", self.cx + 20, box_y + 40, WIN_DARK);
        }
    }

    // выполнить поиск или открыть html файл
    fn do_search(&mut self) {
        let q = self.query.trim();
        if q.is_empty() {
            return;
        }
        // если запрос это имя html файла - открыть страницу
        if q.ends_with(".html") || q.ends_with(".htm") {
            let name = String::from(q);
            self.open_html(&name);
            return;
        }
        // иначе обычный поиск
        self.viewing_html = false;
        self.results.clear();
        // генерируем правдоподобные фейковые ссылки
        let mut r1 = String::from("www.");
        r1.push_str(q);
        r1.push_str(".com - official site");
        self.results.push(r1);

        let mut r2 = String::from("en.notpedia.org/wiki/");
        r2.push_str(q);
        self.results.push(r2);

        let mut r3 = String::from(q);
        r3.push_str(" - news and updates");
        self.results.push(r3);

        let mut r4 = String::from("shop.notzone.com/search?q=");
        r4.push_str(q);
        self.results.push(r4);

        let mut r5 = String::from("How to learn ");
        r5.push_str(q);
        r5.push_str(" - tutorial");
        self.results.push(r5);

        self.searched = true;
    }

    // открыть html файл из файловой системы и распарсить
    fn open_html(&mut self, name: &str) {
        let mut buf = [0u8; FILE_MAX_BYTES];
        match fs::read(name, &mut buf) {
            Ok(size) => {
                if let Ok(text) = core::str::from_utf8(&buf[..size]) {
                    self.page_doc = Some(html::parse(text));
                    self.viewing_html = true;
                }
            }
            Err(_) => {
                self.page_doc = None;
                self.viewing_html = true;
            }
        }
    }

    // нарисовать распарсенную html страницу
    fn render_html(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, 0xFFFFFF);
        draw_text_at("Not-Google viewer", self.cx + 4, self.cy + 4, WIN_DARK);

        let doc = match &self.page_doc {
            Some(d) => d,
            None => {
                draw_text_at("page not found or empty", self.cx + 10, self.cy + 30, 0xAA0000);
                return;
            }
        };

        let left = self.cx + 8;
        let right_limit = self.cx + self.cw - 8;
        let max_cols = (self.cw - 16) / 8; // сколько символов влезает по ширине scale 1
        let mut y = self.cy + 20;

        for block in &doc.blocks {
            // горизонтальная линия
            if block.kind == html::BlockKind::Rule {
                fill_rect(left, y + 4, self.cw - 16, 2, WIN_DARK);
                y += 12;
                continue;
            }
            // пустой блок перенос
            if block.text.is_empty() {
                y += 12;
                continue;
            }

            let bx = left + block.indent;
            let mut startx = bx;

            // маркер списка
            if block.kind == html::BlockKind::ListItem {
                if block.list_num > 0 {
                    let n = block.list_num;
                    let mut label = String::new();
                    if n >= 10 { label.push((b'0' + (n/10) as u8) as char); }
                    label.push((b'0' + (n % 10) as u8) as char);
                    label.push('.');
                    draw_text_at(&label, bx, y, BLACK);
                    startx = bx + 24;
                } else {
                    fill_rect(bx, y + 3, 4, 4, BLACK);
                    startx = bx + 12;
                }
            }

            // ширина символа для этого блока
            let char_w = 8 * block.scale;
            let line_h = 10 * block.scale + 4;

            // перенос по словам если текст длинный (только для scale 1 обычный текст)
            let avail_cols = if char_w > 0 { (right_limit.saturating_sub(startx)) / char_w } else { max_cols };

            // фон для кода на всю строку
            if let Some(bg) = block.bg {
                let tw = block.text.len() * char_w;
                fill_rect(startx.saturating_sub(2), y.saturating_sub(1), tw + 4, line_h, bg);
            }

            // центрирование считаем позицию
            let text_w = block.text.len() * char_w;
            let draw_x = if block.center {
                left + (self.cw - 16).saturating_sub(text_w) / 2
            } else {
                startx
            };

            // если текст влезает в строку рисуем как есть
            if block.text.len() <= avail_cols || block.scale > 1 || block.center {
                let width = draw_text_scaled(&block.text, draw_x, y, block.color, block.scale);
                if block.underline || block.is_link {
                    fill_rect(draw_x, y + 8 * block.scale, width, 1, block.color);
                }
                y += line_h;
            } else {
                // перенос по словам для длинного обычного текста
                let words = block.text.split(' ');
                let mut line = String::new();
                for w in words {
                    let test_len = line.len() + w.len() + 1;
                    if test_len > avail_cols && !line.is_empty() {
                        draw_text_scaled(&line, startx, y, block.color, block.scale);
                        y += line_h;
                        line = String::new();
                    }
                    if !line.is_empty() { line.push(' '); }
                    line.push_str(w);
                }
                if !line.is_empty() {
                    let width = draw_text_scaled(&line, startx, y, block.color, block.scale);
                    if block.underline || block.is_link {
                        fill_rect(startx, y + 8, width, 1, block.color);
                    }
                    y += line_h;
                }
            }

            if y > self.cy + self.ch - 20 {
                break;
            }
        }
    }

    fn window_title(&self) -> &str {
        if self.viewing_html {
            if let Some(doc) = &self.page_doc {
                return &doc.title;
            }
        }
        "Not-Google"
    }

    fn search_btn_hit(&self, mx: i32, my: i32) -> bool {
        let (bx, by, bw, bh) = self.search_btn;
        mx >= bx as i32 && mx < (bx + bw) as i32 && my >= by as i32 && my < (by + bh) as i32
    }
}

// перерисовать текущий экран

fn win_title(app: App) -> &'static str {
    match app {
        App::Terminal => "Terminal",
        App::Browser => "Not-Google",
        App::Clock => "Clock",
        App::Calc => "Calculator",
        App::Paint => "Paint",
        App::Desktop => "",
    }
}

// нарисовать содержимое приложения в окно
fn draw_app_content(app: App, term: &Term, browser: &mut Browser,
                    clock: &Clock, calc: &Calc, paint: &Paint) {
    match app {
        App::Terminal => term.redraw(),
        App::Browser => browser.redraw(),
        App::Clock => clock.redraw(),
        App::Calc => calc.redraw(),
        App::Paint => paint.redraw(),
        App::Desktop => {}
    }
}

pub fn run() {
    mouse::init();
    let (sw, sh) = framebuffer::dimensions();

    let icons = [
        Icon { x: 30, y: 30, label: "Terminal", app: App::Terminal },
        Icon { x: 30, y: 120, label: "Not-Google", app: App::Browser },
        Icon { x: 30, y: 210, label: "Clock", app: App::Clock },
        Icon { x: 30, y: 300, label: "Calc", app: App::Calc },
        Icon { x: 30, y: 390, label: "Paint", app: App::Paint },
    ];

    let ww = 520;
    let wh = 340;
    let wx = (sw - ww) / 2;
    let wy = (sh - wh) / 2;

    let (cx, cy, cw, ch, _) = draw_window(wx, wy, ww, wh, "");
    let mut term = Term::new(cx, cy, cw, ch);
    let mut browser = Browser::new(cx, cy, cw, ch);
    let mut clock = Clock::new(cx, cy, cw, ch);
    let mut calc = Calc::new(cx, cy, cw, ch);
    let mut paint = Paint::new(cx, cy, cw, ch);
    let mut close_btn = CloseBtn { x: 0, y: 0, w: 0, h: 0 };

    let mut app = App::Desktop;
    draw_desktop(&icons);

    let (imx, imy, _, _) = mouse::get();
    framebuffer::save_under_cursor(imx as usize, imy as usize);
    framebuffer::draw_cursor_arrow(imx as usize, imy as usize);
    let mut last_mx = imx;
    let mut last_my = imy;
    let mut last_left = false;
    let mut clock_frame: u64 = 0;

    loop {
        mouse::poll();
        let (mx, my, left, _right) = mouse::get();

        if mx != last_mx || my != last_my {
            // Paint рисуем кистью если кнопка зажата и мы в холсте
            if app == App::Paint && left {
                framebuffer::restore_under_cursor();
                paint.on_drag(mx, my);
                framebuffer::save_under_cursor(mx as usize, my as usize);
                framebuffer::draw_cursor_arrow(mx as usize, my as usize);
            } else {
                framebuffer::restore_under_cursor();
                framebuffer::save_under_cursor(mx as usize, my as usize);
                framebuffer::draw_cursor_arrow(mx as usize, my as usize);
            }
            last_mx = mx;
            last_my = my;
        }

        // Clock тикает и обновляется
        if app == App::Clock {
            clock.update();
            // перерисуем раз в примерно N итераций чтобы секунды шли
            if clock_frame % 40000 == 0 {
                framebuffer::restore_under_cursor();
                clock.redraw();
                framebuffer::save_under_cursor(mx as usize, my as usize);
                framebuffer::draw_cursor_arrow(mx as usize, my as usize);
            }
            clock_frame = clock_frame.wrapping_add(1);
        }

        if left && !last_left {
            framebuffer::restore_under_cursor();
            match app {
                App::Desktop => {
                    for icon in &icons {
                        if icon.contains(mx, my) {
                            app = icon.app;
                            draw_desktop(&icons);
                            let (_, _, _, _, cb) = draw_window(wx, wy, ww, wh, win_title(app));
                            close_btn = cb;
                            draw_app_content(app, &term, &mut browser, &clock, &calc, &paint);
                        }
                    }
                }
                _ => {
                    if close_btn.contains(mx, my) {
                        app = App::Desktop;
                        draw_desktop(&icons);
                    } else if app == App::Browser && browser.search_btn_hit(mx, my) {
                        browser.do_search();
                        browser.redraw();
                    } else if app == App::Calc {
                        if calc.click(mx, my) { calc.redraw(); }
                    } else if app == App::Paint {
                        paint.on_click(mx, my); // сам рисует toolbar или очистку
                    }
                }
            }
            framebuffer::save_under_cursor(mx as usize, my as usize);
            framebuffer::draw_cursor_arrow(mx as usize, my as usize);
        }
        last_left = left;

        if let Some(key) = keyboard::try_read_key() {
            if key == 0x1b {
                return;
            }
            framebuffer::restore_under_cursor();
            let mut redraw_needed = false;
            match app {
                App::Terminal => {
                    match key {
                        KEY_ENTER => {
                            let inp = term.input.clone();
                            term.input.clear();
                            term.exec(&inp);
                            redraw_needed = true;
                        }
                        KEY_BACKSPACE => { term.input.pop(); redraw_needed = true; }
                        0x20..=0x7e => { term.input.push(key as char); redraw_needed = true; }
                        _ => {}
                    }
                    if redraw_needed { term.redraw(); }
                }
                App::Browser => {
                    match key {
                        KEY_ENTER => { browser.do_search(); redraw_needed = true; }
                        KEY_BACKSPACE => { browser.query.pop(); redraw_needed = true; }
                        0x20..=0x7e => { browser.query.push(key as char); redraw_needed = true; }
                        _ => {}
                    }
                    if redraw_needed { browser.redraw(); }
                }
                _ => {}
            }
            framebuffer::save_under_cursor(mx as usize, my as usize);
            framebuffer::draw_cursor_arrow(mx as usize, my as usize);
        }
    }
}

