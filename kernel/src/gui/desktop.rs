// desktop.rs - рабочий стол на оконном менеджере (wm.rs)

use crate::framebuffer::{self, draw_text_at, fill_rect, draw_rect};
use crate::keyboard;
use crate::mouse;
use crate::gui::widgets::apps::{Clock, Calc, Paint, Term, Browser};
use crate::gui::wm::{self, Widget, WindowGeom, Rect};
use alloc::boxed::Box;

// какое приложение открыть
#[derive(PartialEq, Clone, Copy)]
enum App {
    Terminal, Browser, Clock, Calc, Paint,
}

const DESKTOP: u32 = 0x008080;
const WIN_FACE: u32 = 0xC0C0C0;
const WIN_LIGHT: u32 = 0xFFFFFF;
const WIN_DARK: u32 = 0x808080;
const BLACK: u32 = 0x000000;
const TERM_FG: u32 = 0x00FF00;
const NG_BLUE: u32 = 0x4488FF;

fn bevel(x: usize, y: usize, w: usize, h: usize, raised: bool) {
    let (tl, br) = if raised { (WIN_LIGHT, WIN_DARK) } else { (WIN_DARK, WIN_LIGHT) };
    fill_rect(x, y, w, 2, tl);
    fill_rect(x, y, 2, h, tl);
    fill_rect(x, y + h - 2, w, 2, br);
    fill_rect(x + w - 2, y, 2, h, br);
}

struct Icon { x: usize, y: usize, label: &'static str, app: App }

impl Icon {
    fn contains(&self, mx: i32, my: i32) -> bool {
        mx >= self.x as i32 && mx < (self.x + 64) as i32
            && my >= self.y as i32 && my < (self.y + 72) as i32
    }

    fn draw(&self) {
        let ix = self.x + 12;
        let iy = self.y;
        fill_rect(ix, iy, 40, 40, WIN_FACE);
        bevel(ix, iy, 40, 40, true);
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
                fill_rect(ix + 19, iy + 12, 2, 9, BLACK);
                fill_rect(ix + 20, iy + 19, 8, 2, BLACK);
            }
            App::Calc => {
                fill_rect(ix + 6, iy + 6, 28, 10, 0x203020);
                fill_rect(ix + 8, iy + 20, 6, 6, BLACK);
                fill_rect(ix + 17, iy + 20, 6, 6, BLACK);
                fill_rect(ix + 26, iy + 20, 6, 6, BLACK);
            }
            App::Paint => {
                fill_rect(ix + 6, iy + 6, 12, 12, 0xFF0000);
                fill_rect(ix + 22, iy + 6, 12, 12, 0x0000FF);
                fill_rect(ix + 6, iy + 22, 12, 12, 0x00AA00);
                fill_rect(ix + 22, iy + 22, 12, 12, 0xFFDD00);
            }
        }
        draw_text_at(self.label, self.x + 4, self.y + 44, WIN_LIGHT);
    }
}

// нарисовать стол фон иконки панель задач
fn draw_desktop(icons: &[Icon]) {
    let (w, h) = framebuffer::dimensions();
    fill_rect(0, 0, w, h, DESKTOP);
    for icon in icons {
        icon.draw();
    }
    fill_rect(0, h - 24, w, 24, WIN_FACE);
    bevel(0, h - 24, w, 24, true);
    draw_text_at("IluminOS", 8, h - 24 + 8, BLACK);
}

fn app_title(app: App) -> &'static str {
    match app {
        App::Terminal => "Terminal",
        App::Browser => "Not-Google",
        App::Clock => "Clock",
        App::Calc => "Calculator",
        App::Paint => "Paint",
    }
}

// создать виджет приложения в области content окна
fn spawn(app: App, c: Rect) -> Box<dyn Widget> {
    let (cx, cy, cw, ch) = (c.x as usize, c.y as usize, c.w as usize, c.h as usize);
    match app {
        App::Terminal => Box::new(Term::new(cx, cy, cw, ch)),
        App::Browser  => Box::new(Browser::new(cx, cy, cw, ch)),
        App::Clock    => Box::new(Clock::new(cx, cy, cw, ch)),
        App::Calc     => Box::new(Calc::new(cx, cy, cw, ch)),
        App::Paint    => Box::new(Paint::new(cx, cy, cw, ch)),
    }
}

pub fn run() {
    mouse::init();
    let (sw, sh) = framebuffer::dimensions();

    let icons = [
        Icon { x: 30, y: 30,  label: "Terminal",   app: App::Terminal },
        Icon { x: 30, y: 120, label: "Not-Google", app: App::Browser },
        Icon { x: 30, y: 210, label: "Clock",      app: App::Clock },
        Icon { x: 30, y: 300, label: "Calc",       app: App::Calc },
        Icon { x: 30, y: 390, label: "Paint",      app: App::Paint },
    ];

    // окно по центру экрана
    let geom = WindowGeom::new((sw - 520) / 2, (sh - 340) / 2, 520, 340);

    // активное окно None = стол Some = приложение открыто
    let mut active: Option<Box<dyn Widget>> = None;

    draw_desktop(&icons);

    let (imx, imy, _, _) = mouse::get();
    framebuffer::save_under_cursor(imx as usize, imy as usize);
    framebuffer::draw_cursor_arrow(imx as usize, imy as usize);
    let mut last_mx = imx;
    let mut last_my = imy;
    let mut last_left = false;
    let mut tick_frame: u64 = 0;

    loop {
        mouse::poll();
        let (mx, my, left, _right) = mouse::get();

        if mx != last_mx || my != last_my {
            if active.is_some() && left {
                // drag (кисть Paint)
                framebuffer::restore_under_cursor();
                if let Some(w) = active.as_mut() {
                    wm::route_drag(w.as_mut(), mx, my);
                }
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

        if let Some(w) = active.as_mut() {
            if tick_frame % 40000 == 0 && wm::route_tick(w.as_mut()) {
                framebuffer::restore_under_cursor();
                wm::route_draw(w.as_mut());
                framebuffer::save_under_cursor(mx as usize, my as usize);
                framebuffer::draw_cursor_arrow(mx as usize, my as usize);
            }
        }
        tick_frame = tick_frame.wrapping_add(1);

        // клик по фронту нажатия
        if left && !last_left {
            framebuffer::restore_under_cursor();
            let mut opened: Option<App> = None;
            let mut closing = false;
            // сначала решаем что делать, не присваивая active внутри его же borrow
            if let Some(w) = active.as_mut() {
                if wm::close_hit(geom, mx, my) {
                    closing = true;
                } else if wm::route_click(w.as_mut(), mx, my) {
                    wm::route_draw(w.as_mut());
                }
            } else {
                for icon in &icons {
                    if icon.contains(mx, my) {
                        opened = Some(icon.app);
                        break;
                    }
                }
            }
            // применяем решение вне borrow
            if closing {
                active = None;
                draw_desktop(&icons);
            }
            if let Some(app) = opened {
                draw_desktop(&icons);
                wm::draw_frame(geom, app_title(app));
                let mut w = spawn(app, geom.content_area());
                wm::route_draw(w.as_mut());
                active = Some(w);
            }
            framebuffer::save_under_cursor(mx as usize, my as usize);
            framebuffer::draw_cursor_arrow(mx as usize, my as usize);
        }
        last_left = left;

        if let Some(key) = keyboard::try_read_key() {
            if key == 0x1b {
                return;
            }
            if let Some(w) = active.as_mut() {
                framebuffer::restore_under_cursor();
                if wm::route_key(w.as_mut(), key) {
                    wm::route_draw(w.as_mut());
                }
                framebuffer::save_under_cursor(mx as usize, my as usize);
                framebuffer::draw_cursor_arrow(mx as usize, my as usize);
            }
        }
    }
}
