// wm.rs - мини-фреймворк оконной системы (window manager)
// Rect + трейт Widget + примитивы отрисовки + адаптеры существующих приложений

use crate::framebuffer::{fill_rect, draw_rect, draw_text_at};
use crate::gui::widgets::apps::{Calc, Clock, Paint};

// Rect - прямоугольная область

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }

    // попадает ли точка (px, py) внутрь прямоугольника
    // правая/нижняя границы исключительные (как принято в пиксельных координатах)
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

// Widget - контракт любого окна/приложения

pub trait Widget {
    // менеджер уже нарисовал рамку окна и заголовок - виджет рисует только
    // своё нутро внутри выданной ему области (см. WindowManager::content_area)
    fn draw(&mut self);

    // клик левой кнопкой внутри окна (координаты абсолютные, экранные)
    // вернуть true если нужно перерисовать окно после клика
    fn on_click(&mut self, _x: i32, _y: i32) -> bool {
        false
    }

    // нажата клавиша (только когда окно в фокусе)
    // вернуть true если нужно перерисовать
    fn on_key(&mut self, _key: u8) -> bool {
        false
    }

    // мышь движется с зажатой левой кнопкой (для рисования в Paint)
    fn on_drag(&mut self, _x: i32, _y: i32) {}

    // "тик" - вызывается менеджером периодически, для окон которые сами
    // меняются со временем (например часы). true = перерисовать.
    fn tick(&mut self) -> bool {
        false
    }
}

// Примитивы отрисовки - общие кирпичики для всех окон

// цвета оформления в стиле Windows 3.1 (те же что использует desktop.rs)
pub const WIN_FACE: u32 = 0xC0C0C0;
pub const WIN_LIGHT: u32 = 0xFFFFFF;
pub const WIN_DARK: u32 = 0x808080;
pub const BLACK: u32 = 0x000000;

// объёмная грань: светлое сверху-слева, тёмное снизу-справа
// raised=true - кнопка выпуклая (обычная), false - вдавленная (нажатая)
pub fn bevel(x: usize, y: usize, w: usize, h: usize, raised: bool) {
    let (tl, br) = if raised { (WIN_LIGHT, WIN_DARK) } else { (WIN_DARK, WIN_LIGHT) };
    fill_rect(x, y, w, 2, tl);
    fill_rect(x, y, 2, h, tl);
    fill_rect(x, y + h - 2, w, 2, br);
    fill_rect(x + w - 2, y, 2, h, br);
}

// панель - залитый прямоугольник с объёмной рамкой (фон окна/области)
pub fn panel(r: Rect) {
    fill_rect(r.x as usize, r.y as usize, r.w as usize, r.h as usize, WIN_FACE);
    bevel(r.x as usize, r.y as usize, r.w as usize, r.h as usize, true);
}

// кнопка с текстом. рисует объёмный прямоугольник и подпись по центру.
// сам факт клика проверяется отдельно через Rect::contains - эта функция
// только рисует.
pub fn button(r: Rect, label: &str) {
    fill_rect(r.x as usize, r.y as usize, r.w as usize, r.h as usize, WIN_FACE);
    bevel(r.x as usize, r.y as usize, r.w as usize, r.h as usize, true);
    // подпись примерно по центру (шрифт 8px на символ)
    let tx = r.x as usize + (r.w as usize).saturating_sub(label.len() * 8) / 2;
    let ty = r.y as usize + (r.h as usize).saturating_sub(8) / 2;
    draw_text_at(label, tx, ty, BLACK);
}

pub fn label(x: i32, y: i32, text: &str, color: u32) {
    draw_text_at(text, x as usize, y as usize, color);
}

// рамка без заливки (например для поля ввода)
pub fn outline(r: Rect, color: u32) {
    draw_rect(r.x as usize, r.y as usize, r.w as usize, r.h as usize, color);
}

// Адаптер: Calc как Widget

impl Widget for Calc {
    fn draw(&mut self) {
        // Calc::redraw берёт &self, а трейт даёт &mut self - это совместимо
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
        // Calc::click уже возвращает "нужно ли перерисовать"
        self.click(x, y)
    }

    // on_key / on_drag / tick - у калькулятора не нужны, работает поведение
    // по умолчанию из трейта (ничего не делает)
}

// Адаптер: Clock как Widget

impl Widget for Clock {
    fn draw(&mut self) {
        self.redraw();
    }

    // tick - часы обновляются сами. update() пересчитывает состояние,
    // возвращаем true чтобы менеджер перерисовал (время идёт).
    fn tick(&mut self) -> bool {
        self.update();
        true
    }
}

// Адаптер: Paint как Widget

impl Widget for Paint {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
        // вызов inherent-метода Paint, не трейта, иначе рекурсия
        let _ = Paint::on_click(self, x, y);
        false
    }

    fn on_drag(&mut self, x: i32, y: i32) {
        Paint::on_drag(self, x, y);
    }
}

// WindowManager - рамка окна, заголовок, кнопка закрытия, роутинг событий

#[derive(Clone, Copy)]
pub struct WindowGeom {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

const TITLE_BG: u32 = 0x000080;
const TITLE_FG: u32 = 0xFFFFFF;
const TITLE_H: usize = 18;

impl WindowGeom {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        WindowGeom { x, y, w, h }
    }

    // область содержимого - куда виджет рисует своё нутро (под заголовком)
    pub fn content_area(&self) -> Rect {
        Rect::new(
            (self.x + 4) as i32,
            (self.y + 3 + TITLE_H + 2) as i32,
            (self.w - 8) as i32,
            (self.h - (3 + TITLE_H + 2) - 4) as i32,
        )
    }

    // прямоугольник кнопки закрытия [x] в правом верхнем углу
    pub fn close_button(&self) -> Rect {
        Rect::new((self.x + self.w - 20) as i32, (self.y + 5) as i32, 14, 14)
    }
}

// нарисовать рамку окна: тело, объёмная грань, заголовок, кнопка [x]
pub fn draw_frame(g: WindowGeom, title: &str) {
    panel(Rect::new(g.x as i32, g.y as i32, g.w as i32, g.h as i32));
    draw_rect(g.x, g.y, g.w, g.h, BLACK);

    fill_rect(g.x + 3, g.y + 3, g.w - 6, TITLE_H, TITLE_BG);
    draw_text_at(title, g.x + 7, g.y + 3 + 5, TITLE_FG);

    let cb = g.close_button();
    button(cb, "x");
}

// попал ли клик по кнопке закрытия окна
pub fn close_hit(g: WindowGeom, x: i32, y: i32) -> bool {
    g.close_button().contains(x, y)
}

// передать событие активному виджету (тонкие обёртки - чтобы desktop не знал
// деталей трейта). Возвращают "нужно ли перерисовать".
pub fn route_click(widget: &mut dyn Widget, x: i32, y: i32) -> bool {
    widget.on_click(x, y)
}

pub fn route_key(widget: &mut dyn Widget, key: u8) -> bool {
    widget.on_key(key)
}

pub fn route_drag(widget: &mut dyn Widget, x: i32, y: i32) {
    widget.on_drag(x, y);
}

pub fn route_tick(widget: &mut dyn Widget) -> bool {
    widget.tick()
}

pub fn route_draw(widget: &mut dyn Widget) {
    widget.draw();
}

// Адаптер: Terminal как Widget

use crate::gui::widgets::apps::{Term, Browser};
use crate::keyboard::{KEY_ENTER, KEY_BACKSPACE};

impl Widget for Term {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_key(&mut self, key: u8) -> bool {
        match key {
            KEY_ENTER => {
                // выполнить набранную команду и очистить ввод
                let inp = self.input.clone();
                self.input.clear();
                self.exec(&inp);
                true
            }
            KEY_BACKSPACE => {
                self.input.pop();
                true
            }
            0x20..=0x7e => {
                self.input.push(key as char);
                true
            }
            _ => false,
        }
    }
}

// Адаптер: Browser (Not-Google) как Widget

impl Widget for Browser {
    fn draw(&mut self) {
        self.redraw();
    }

    fn on_click(&mut self, x: i32, y: i32) -> bool {
            if self.search_btn_hit(x, y) {
            self.do_search();
            true
        } else {
            false
        }
    }

    fn on_key(&mut self, key: u8) -> bool {
        match key {
            KEY_ENTER => {
                self.do_search();
                true
            }
            KEY_BACKSPACE => {
                self.query.pop();
                true
            }
            0x20..=0x7e => {
                self.query.push(key as char);
                true
            }
            _ => false,
        }
    }
}

// Виджеты-кирпичи - структуры Button/Label/TextField со своей областью и отрисовкой

const TEXT_DARK: u32 = 0x000000;

// Button - кликабельная кнопка с подписью
pub struct Button {
    pub area: Rect,
    pub text: &'static str,
}

impl Button {
    pub fn new(text: &'static str, area: Rect) -> Self {
        Button { text, area }
    }

    // нарисовать кнопку (объёмная грань + подпись по центру)
    pub fn draw(&self) {
        button(self.area, self.text);
    }

    pub fn hit(&self, mx: i32, my: i32) -> bool {
        self.area.contains(mx, my)
    }
}

// Label - просто текст в позиции
pub struct Label {
    pub x: i32,
    pub y: i32,
    pub text: &'static str,
    pub color: u32,
}

impl Label {
    pub fn new(x: i32, y: i32, text: &'static str, color: u32) -> Self {
        Label { x, y, text, color }
    }

    pub fn draw(&self) {
        label(self.x, self.y, self.text, self.color);
    }
}

// TextField - однострочное поле ввода со своим состоянием

pub struct TextField {
    pub area: Rect,
    pub text: alloc::string::String,
    pub max_len: usize,
    pub focused: bool, // рисовать ли курсор
}

impl TextField {
    pub fn new(area: Rect, max_len: usize) -> Self {
        TextField {
            area,
            text: alloc::string::String::new(),
            max_len,
            focused: true,
        }
    }

    // обработать нажатие клавиши; вернуть true если содержимое изменилось
    pub fn key(&mut self, key: u8) -> bool {
        match key {
            0x08 => {
                self.text.pop();
                true
            }
            0x20..=0x7e => {
                if self.text.len() < self.max_len {
                    self.text.push(key as char);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    // нарисовать поле: фон, рамка, текст, курсор
    pub fn draw(&self) {
        let x = self.area.x as usize;
        let y = self.area.y as usize;
        let w = self.area.w as usize;
        let h = self.area.h as usize;
        fill_rect(x, y, w, h, 0xFFFFFF);
        outline(self.area, WIN_DARK);
        draw_text_at(&self.text, x + 4, y + (h.saturating_sub(8)) / 2, TEXT_DARK);
        // курсор - вертикальная палочка после текста
        if self.focused {
            let cx = x + 4 + self.text.len() * 8;
            fill_rect(cx, y + 4, 2, h.saturating_sub(8), TEXT_DARK);
        }
    }

    // попал ли клик в поле (чтобы поставить фокус)
    pub fn hit(&self, mx: i32, my: i32) -> bool {
        self.area.contains(mx, my)
    }
}
