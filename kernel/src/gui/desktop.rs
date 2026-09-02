use crate::framebuffer::{self, draw_text_at, fill_rect, draw_rect};
use crate::keyboard::{self, KEY_BACKSPACE, KEY_ENTER};
use crate::mouse;
use crate::gui::widgets::apps::{Clock, Calc, Paint};
use crate::fs::{self, FILE_MAX_BYTES};
use crate::html;
use crate::framebuffer::draw_text_scaled;
use alloc::vec::Vec;
use alloc::string::String;

const DESKTOP: u32 = 0x008080;   // бирюзовый фон стола
const WIN_FACE: u32 = 0xC0C0C0;  // серая "поверхность" окон
const WIN_LIGHT: u32 = 0xFFFFFF; // светлая грань (объём)
const WIN_DARK: u32 = 0x808080;  // тёмная грань (объём)
const TITLE_BG: u32 = 0x000080;  // синий заголовок окна
const TITLE_FG: u32 = 0xFFFFFF;  // белый текст заголовка
const BLACK: u32 = 0x000000;
const TERM_BG: u32 = 0x000000;   // фон терминала
const TERM_FG: u32 = 0x00FF00;   // зелёный текст терминала
const NG_BLUE: u32 = 0x4488FF;   // цвета логотипа Not-Google
const NG_RED: u32 = 0xFF4444;
const NG_YELLOW: u32 = 0xFFDD33;
const NG_GREEN: u32 = 0x33CC66;
const LINK: u32 = 0x0000EE;      // синие ссылки

// Какое приложение сейчас открыто
#[derive(PartialEq, Clone, Copy)]
enum App {
    Desktop, Terminal, Browser, Clock, Calc, Paint,
}

// нарисовать "объёмную" грань (эффект вдавленности/выпуклости)
// Светлые линии сверху-слева + тёмные снизу-справа = выпуклая кнопка
fn bevel(x: usize, y: usize, w: usize, h: usize, raised: bool) {
    let (tl, br) = if raised { (WIN_LIGHT, WIN_DARK) } else { (WIN_DARK, WIN_LIGHT) };
    fill_rect(x, y, w, 2, tl);         // верх
    fill_rect(x, y, 2, h, tl);         // лево
    fill_rect(x, y + h - 2, w, 2, br); // низ
    fill_rect(x + w - 2, y, 2, h, br); // право
}

// Кнопка закрытия окна — храним её область, чтобы ловить по ней клик
struct CloseBtn { x: usize, y: usize, w: usize, h: usize }

impl CloseBtn {
    // попал ли клик (mx,my) внутрь кнопки
    fn contains(&self, mx: i32, my: i32) -> bool {
        mx >= self.x as i32 && mx < (self.x + self.w) as i32
            && my >= self.y as i32 && my < (self.y + self.h) as i32
    }
}

// нарисовать рамку окна с заголовком и кнопкой [x]
// Возвращает область содержимого (cx,cy,cw,ch) и кнопку закрытия
fn draw_window(x: usize, y: usize, w: usize, h: usize, title: &str) -> (usize, usize, usize, usize, CloseBtn) {
    fill_rect(x, y, w, h, WIN_FACE); // тело окна
    bevel(x, y, w, h, true);         // объёмная рамка
    draw_rect(x, y, w, h, BLACK);    // чёрный контур

    let title_h = 18;
    fill_rect(x + 3, y + 3, w - 6, title_h, TITLE_BG); // синяя полоса заголовка
    draw_text_at(title, x + 7, y + 3 + 5, TITLE_FG);   // текст заголовка

    // кнопка [x] справа в заголовке
    let bx = x + w - 20;
    let by = y + 5;
    fill_rect(bx, by, 14, 14, WIN_FACE);
    bevel(bx, by, 14, 14, true);
    draw_text_at("x", bx + 4, by + 3, BLACK);

    // область содержимого (под заголовком)
    let cx = x + 4;
    let cy = y + 3 + title_h + 2;
    let cw = w - 8;
    let ch = h - (3 + title_h + 2) - 4;
    (cx, cy, cw, ch, CloseBtn { x: bx, y: by, w: 14, h: 14 })
}

// Иконка приложения на рабочем столе
struct Icon { x: usize, y: usize, label: &'static str, app: App }

impl Icon {
    // попал ли клик в область иконки (64x72)
    fn contains(&self, mx: i32, my: i32) -> bool {
        mx >= self.x as i32 && mx < (self.x + 64) as i32
            && my >= self.y as i32 && my < (self.y + 72) as i32
    }

    // нарисовать иконку (квадратик + рисунок внутри + подпись)
    fn draw(&self) {
        let ix = self.x + 12;
        let iy = self.y;
        fill_rect(ix, iy, 40, 40, WIN_FACE);
        bevel(ix, iy, 40, 40, true);
        // у каждого приложения свой мини-рисунок
        match self.app {
            App::Terminal => {
                fill_rect(ix + 6, iy + 6, 28, 28, BLACK);
                draw_text_at(">", ix + 10, iy + 14, TERM_FG);
                draw_text_at("_", ix + 18, iy + 16, TERM_FG);
            }
            App::Browser => {
                fill_rect(ix + 8, iy + 8, 24, 24, NG_BLUE);
                fill_rect(ix + 14, iy + 14, 12, 12, WIN_FACE);
            }
            App::Clock => {
                fill_rect(ix + 6, iy + 6, 28, 28, 0xFFFFFF);
                draw_rect(ix + 6, iy + 6, 28, 28, BLACK);
                fill_rect(ix + 19, iy + 12, 2, 9, BLACK);  // стрелки часов
                fill_rect(ix + 20, iy + 19, 8, 2, BLACK);
            }
            App::Calc => {
                fill_rect(ix + 6, iy + 6, 28, 10, 0x203020); // экранчик
                fill_rect(ix + 8, iy + 20, 6, 6, BLACK);     // кнопки
                fill_rect(ix + 17, iy + 20, 6, 6, BLACK);
                fill_rect(ix + 26, iy + 20, 6, 6, BLACK);
            }
            App::Paint => {
                // палитра из 4 цветных квадратиков
                fill_rect(ix + 6, iy + 6, 12, 12, 0xFF0000);
                fill_rect(ix + 22, iy + 6, 12, 12, 0x0000FF);
                fill_rect(ix + 6, iy + 22, 12, 12, 0x00AA00);
                fill_rect(ix + 22, iy + 22, 12, 12, 0xFFDD00);
            }
            _ => {}
        }
        draw_text_at(self.label, self.x + 4, self.y + 44, WIN_LIGHT); // подпись
    }
}

// нарисовать стол: фон, иконки, панель задач
fn draw_desktop(icons: &[Icon]) {
    let (w, h) = framebuffer::dimensions();
    fill_rect(0, 0, w, h, DESKTOP);
    for icon in icons {
        icon.draw();
    }
    // панель задач снизу
    fill_rect(0, h - 24, w, 24, WIN_FACE);
    bevel(0, h - 24, w, 24, true);
    draw_text_at("IluminOS", 8, h - 24 + 8, BLACK);
}

// Приложение "Терминал" (простой, отдельный от основного shell)
struct Term {
    lines: Vec<String>, // строки вывода
    input: String,      // текущий ввод
    cx: usize, cy: usize, cw: usize, ch: usize, // область содержимого окна
}

impl Term {
    fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        let mut lines = Vec::new();
        lines.push(String::from("IluminOS terminal"));
        lines.push(String::from("commands help ver clear"));
        lines.push(String::from(""));
        Term { lines, input: String::new(), cx, cy, cw, ch }
    }

    // перерисовать терминал: последние строки + строку ввода
    fn redraw(&self) {
        fill_rect(self.cx, self.cy, self.cw, self.ch, TERM_BG);
        let max_rows = (self.ch / 10).max(2);
        let visible = max_rows - 1;
        // показываем только последние `visible` строк (прокрутка)
        let start = if self.lines.len() > visible { self.lines.len() - visible } else { 0 };
        let mut row = 0;
        for line in &self.lines[start..] {
            draw_text_at(line, self.cx + 2, self.cy + 2 + row * 10, TERM_FG);
            row += 1;
        }
        // строка ввода с приглашением ">"
        draw_text_at(">", self.cx + 2, self.cy + 2 + row * 10, TERM_FG);
        draw_text_at(&self.input, self.cx + 2 + 8, self.cy + 2 + row * 10, TERM_FG);
    }

    // выполнить команду терминала (мини-набор)
    fn exec(&mut self, cmd: &str) {
        let cmd = cmd.trim();
        let mut echo = String::from("> ");
        echo.push_str(cmd);
        self.lines.push(echo); // эхо введённой команды
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

// Приложение "Not-Google" — шуточный браузер + просмотр html из файлов
struct Browser {
    query: String,                    // поисковый запрос
    results: Vec<String>,             // фейковые результаты
    searched: bool,                   // был ли поиск
    viewing_html: bool,               // режим просмотра html-страницы
    page_doc: Option<html::Document>, // распарсенная страница
    cx: usize, cy: usize, cw: usize, ch: usize,
    search_btn: (usize, usize, usize, usize), // область кнопки Search
}

impl Browser {
    fn new(cx: usize, cy: usize, cw: usize, ch: usize) -> Self {
        Browser {
            query: String::new(), results: Vec::new(),
            searched: false, viewing_html: false, page_doc: None,
            cx, cy, cw, ch, search_btn: (0, 0, 0, 0),
        }
    }

    // либо страница html, либо поисковая "домашняя" страница
    fn redraw(&mut self) {
        if self.viewing_html {
            self.render_html();
            return;
        }
        fill_rect(self.cx, self.cy, self.cw, self.ch, 0xFFFFFF); // белый фон

        // логотип Not-Google по центру, буквы разными цветами
        let logo_y = self.cy + 30;
        let cx_center = self.cx + self.cw / 2;
        let logo = "Not-Google";
        let logo_x = cx_center - (logo.len() * 8) / 2;
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

        // кнопка Search (запоминаем её область для кликов)
        let btn_x = box_x + box_w + 8;
        let btn_w = 70;
        fill_rect(btn_x, box_y, btn_w, 20, WIN_FACE);
        bevel(btn_x, box_y, btn_w, 20, true);
        draw_text_at("Search", btn_x + 8, box_y + 6, BLACK);
        self.search_btn = (btn_x, box_y, btn_w, 20);

        // результаты или подсказка
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

    // поиск. Если запрос это *.html — открыть файл, иначе фейк-выдача
    fn do_search(&mut self) {
        let q = self.query.trim();
        if q.is_empty() {
            return;
        }
        // имя html-файла -> открыть страницу
        if q.ends_with(".html") || q.ends_with(".htm") {
            let name = String::from(q);
            self.open_html(&name);
            return;
        }
        // обычный поиск -> генерируем правдоподобные фейковые ссылки
        self.viewing_html = false;
        self.results.clear();
        let mut r1 = String::from("www.");
        r1.push_str(q); r1.push_str(".com - official site");
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
        r5.push_str(q); r5.push_str(" - tutorial");
        self.results.push(r5);
        self.searched = true;
    }

    // прочитать html-файл из ФС и распарсить в документ
    fn open_html(&mut self, name: &str) {
        let mut buf = [0u8; FILE_MAX_BYTES];
        match fs::read(name, &mut buf) {
            Ok(size) => {
                if let Ok(text) = core::str::from_utf8(&buf[..size]) {
                    self.page_doc = Some(html::parse(text)); // парсим (см. html.rs)
                    self.viewing_html = true;
                }
            }
            Err(_) => {
                self.page_doc = None;
                self.viewing_html = true;
            }
        }
    }

    // нарисовать распарсенную страницу (блок за блоком)
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
        let max_cols = (self.cw - 16) / 8; // сколько символов влезает по ширине
        let mut y = self.cy + 20;

        // каждый блок документа рисуем по очереди, двигая y вниз
        for block in &doc.blocks {
            // горизонтальная линейка <hr>
            if block.kind == html::BlockKind::Rule {
                fill_rect(left, y + 4, self.cw - 16, 2, WIN_DARK);
                y += 12;
                continue;
            }
            // пустой блок — просто отступ
            if block.text.is_empty() {
                y += 12;
                continue;
            }

            let bx = left + block.indent;
            let mut startx = bx;

            // маркер элемента списка (номер или точка)
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
                    fill_rect(bx, y + 3, 4, 4, BLACK); // буллет
                    startx = bx + 12;
                }
            }

            let char_w = 8 * block.scale;       // ширина символа с учётом масштаба
            let line_h = 10 * block.scale + 4;  // высота строки

            // сколько символов влезет в оставшуюся ширину
            let avail_cols = if char_w > 0 { (right_limit.saturating_sub(startx)) / char_w } else { max_cols };

            // фон для блоков кода
            if let Some(bg) = block.bg {
                let tw = block.text.len() * char_w;
                fill_rect(startx.saturating_sub(2), y.saturating_sub(1), tw + 4, line_h, bg);
            }

            // позиция с учётом центрирования
            let text_w = block.text.len() * char_w;
            let draw_x = if block.center {
                left + (self.cw - 16).saturating_sub(text_w) / 2
            } else {
                startx
            };

            // если текст влезает — рисуем как есть
            if block.text.len() <= avail_cols || block.scale > 1 || block.center {
                let width = draw_text_scaled(&block.text, draw_x, y, block.color, block.scale);
                if block.underline || block.is_link {
                    fill_rect(draw_x, y + 8 * block.scale, width, 1, block.color); // подчёркивание ссылок
                }
                y += line_h;
            } else {
                // длинный текст — перенос ПО СЛОВАМ (word wrap)
                let words = block.text.split(' ');
                let mut line = String::new();
                for w in words {
                    let test_len = line.len() + w.len() + 1;
                    // слово не влезает — печатаем накопленную строку и начинаем новую
                    if test_len > avail_cols && !line.is_empty() {
                        draw_text_scaled(&line, startx, y, block.color, block.scale);
                        y += line_h;
                        line = String::new();
                    }
                    if !line.is_empty() { line.push(' '); }
                    line.push_str(w);
                }
                // остаток
                if !line.is_empty() {
                    let width = draw_text_scaled(&line, startx, y, block.color, block.scale);
                    if block.underline || block.is_link {
                        fill_rect(startx, y + 8, width, 1, block.color);
                    }
                    y += line_h;
                }
            }

            // вышли за низ окна — прекращаем рисовать
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

    // попал ли клик в кнопку Search
    fn search_btn_hit(&self, mx: i32, my: i32) -> bool {
        let (bx, by, bw, bh) = self.search_btn;
        mx >= bx as i32 && mx < (bx + bw) as i32 && my >= by as i32 && my < (by + bh) as i32
    }
}

// заголовок окна для приложения
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

// нарисовать содержимое активного приложения
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

// ГЛАВНЫЙ ЦИКЛ GUI
pub fn run() {
    mouse::init();
    let (sw, sh) = framebuffer::dimensions();

    // иконки на столе
    let icons = [
        Icon { x: 30, y: 30, label: "Terminal", app: App::Terminal },
        Icon { x: 30, y: 120, label: "Not-Google", app: App::Browser },
        Icon { x: 30, y: 210, label: "Clock", app: App::Clock },
        Icon { x: 30, y: 300, label: "Calc", app: App::Calc },
        Icon { x: 30, y: 390, label: "Paint", app: App::Paint },
    ];

    // геометрия окна по центру экрана
    let ww = 520;
    let wh = 340;
    let wx = (sw - ww) / 2;
    let wy = (sh - wh) / 2;

    // создаём все приложения заранее (они хранят своё состояние)
    let (cx, cy, cw, ch, _) = draw_window(wx, wy, ww, wh, "");
    let mut term = Term::new(cx, cy, cw, ch);
    let mut browser = Browser::new(cx, cy, cw, ch);
    let mut clock = Clock::new(cx, cy, cw, ch);
    let mut calc = Calc::new(cx, cy, cw, ch);
    let mut paint = Paint::new(cx, cy, cw, ch);
    let mut close_btn = CloseBtn { x: 0, y: 0, w: 0, h: 0 };

    let mut app = App::Desktop; // старт — рабочий стол
    draw_desktop(&icons);

    // рисуем курсор мыши в стартовой позиции
    let (imx, imy, _, _) = mouse::get();
    framebuffer::save_under_cursor(imx as usize, imy as usize);
    framebuffer::draw_cursor_arrow(imx as usize, imy as usize);
    let mut last_mx = imx;
    let mut last_my = imy;
    let mut last_left = false;       // прошлое состояние левой кнопки (для детекта клика)
    let mut clock_frame: u64 = 0;

    loop {
        mouse::poll(); // опросить мышь
        let (mx, my, left, _right) = mouse::get();

        if mx != last_mx || my != last_my {
            if app == App::Paint && left {
                // в Paint при зажатой кнопке — рисуем кистью
                framebuffer::restore_under_cursor();
                paint.on_drag(mx, my);
                framebuffer::save_under_cursor(mx as usize, my as usize);
                framebuffer::draw_cursor_arrow(mx as usize, my as usize);
            } else {
                // обычное движение: вернуть фон, сохранить новый, нарисовать курсор
                framebuffer::restore_under_cursor();
                framebuffer::save_under_cursor(mx as usize, my as usize);
                framebuffer::draw_cursor_arrow(mx as usize, my as usize);
            }
            last_mx = mx;
            last_my = my;
        }

        if app == App::Clock {
            clock.update();
            // перерисовываем не каждую итерацию, а раз в N — чтобы секунды шли
            if clock_frame % 40000 == 0 {
                framebuffer::restore_under_cursor();
                clock.redraw();
                framebuffer::save_under_cursor(mx as usize, my as usize);
                framebuffer::draw_cursor_arrow(mx as usize, my as usize);
            }
            clock_frame = clock_frame.wrapping_add(1);
        }

        // left && !last_left = "кнопка ТОЛЬКО ЧТО нажалась" (фронт сигнала)
        if left && !last_left {
            framebuffer::restore_under_cursor();
            match app {
                App::Desktop => {
                    // на столе — проверяем попадание по иконкам
                    for icon in &icons {
                        if icon.contains(mx, my) {
                            app = icon.app; // открыть приложение
                            draw_desktop(&icons);
                            let (_, _, _, _, cb) = draw_window(wx, wy, ww, wh, win_title(app));
                            close_btn = cb;
                            draw_app_content(app, &term, &mut browser, &clock, &calc, &paint);
                        }
                    }
                }
                _ => {
                    // внутри приложения
                    if close_btn.contains(mx, my) {
                        app = App::Desktop; // закрыли окно
                        draw_desktop(&icons);
                    } else if app == App::Browser && browser.search_btn_hit(mx, my) {
                        browser.do_search();
                        browser.redraw();
                    } else if app == App::Calc {
                        if calc.click(mx, my) { calc.redraw(); }
                    } else if app == App::Paint {
                        paint.on_click(mx, my);
                    }
                }
            }
            framebuffer::save_under_cursor(mx as usize, my as usize);
            framebuffer::draw_cursor_arrow(mx as usize, my as usize);
        }
        last_left = left;

        if let Some(key) = keyboard::try_read_key() {
            if key == 0x1b { // Esc — выход из GUI обратно в консоль
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
