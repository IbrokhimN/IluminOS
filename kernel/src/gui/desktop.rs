// desktop draws background icons taskbar runs main event loop
use crate::framebuffer;
use crate::gui::style::{self, Icon};
use crate::gui::widgets::apps::{Browser, Calc, Clock, Paint, Term};
use crate::gui::wm::{self, Rect, Widget, WindowGeom};
use crate::keyboard;
use crate::mouse;
use alloc::boxed::Box;
use spin::Mutex;

#[derive(PartialEq, Clone, Copy)]
enum App {
    Terminal,
    Browser,
    Clock,
    Calc,
    Paint,
}

impl App {
    fn title(self) -> &'static str {
        match self {
            App::Terminal => "Terminal",
            App::Browser => "Not-Google",
            App::Clock => "Clock",
            App::Calc => "Calculator",
            App::Paint => "Paint",
        }
    }

    fn icon(self) -> &'static Icon {
        match self {
            App::Terminal => &style::ICON_TERMINAL,
            App::Browser => &style::ICON_BROWSER,
            App::Clock => &style::ICON_CLOCK,
            App::Calc => &style::ICON_CALC,
            App::Paint => &style::ICON_PAINT,
        }
    }

    fn spawn(self, c: Rect) -> Box<dyn Widget> {
        let (cx, cy, cw, ch) = (c.x as usize, c.y as usize, c.w as usize, c.h as usize);
        match self {
            App::Terminal => Box::new(Term::new(cx, cy, cw, ch)),
            App::Browser => Box::new(Browser::new(cx, cy, cw, ch)),
            App::Clock => Box::new(Clock::new(cx, cy, cw, ch)),
            App::Calc => Box::new(Calc::new(cx, cy, cw, ch)),
            App::Paint => Box::new(Paint::new(cx, cy, cw, ch)),
        }
    }
}

const ICON_SLOT: i32 = 44;
const ICON_PAD: i32 = 8;
const TASKBAR_H: usize = 28;

// bg patch dimensions
const BG_PATCH_W: usize = 528;
const BG_PATCH_H: usize = 348;

struct DesktopIcon {
    x: i32,
    y: i32,
    label: &'static str,
    app: App,
}

impl DesktopIcon {
    fn hit(&self, mx: i32, my: i32) -> bool {
        style::hit(
            mx,
            my,
            self.x,
            self.y,
            ICON_SLOT as u32,
            ICON_SLOT as u32 + 16,
        )
    }

    fn draw(&self) {
        style::rounded_fill(
            self.x,
            self.y,
            ICON_SLOT as u32,
            ICON_SLOT as u32,
            style::SURFACE,
        );
        self.app.icon().draw(self.x + ICON_PAD, self.y + ICON_PAD);
        style::text_centered(
            self.label,
            self.x - 10,
            self.y + ICON_SLOT + 4,
            ICON_SLOT as u32 + 20,
            12,
            style::TEXT,
        );
    }
}

fn draw_taskbar(screen_w: usize, screen_h: usize) {
    let y = (screen_h - TASKBAR_H) as i32;
    framebuffer::fill_rect(0, y as usize, screen_w, TASKBAR_H, style::TITLEBAR_BG);
    style::text("IluminOS", 10, y + (TASKBAR_H as i32 - 10) / 2, style::TEXT);
}

fn compose_desktop(icons: &[DesktopIcon]) {
    let (w, h) = framebuffer::dimensions();
    style::draw_wallpaper();
    for icon in icons {
        icon.draw();
    }
    draw_taskbar(w, h);
}

// clean bg buffer
static CLEAN_WINDOW_BG: Mutex<[u32; BG_PATCH_W * BG_PATCH_H]> =
    Mutex::new([0; BG_PATCH_W * BG_PATCH_H]);
static BG_IS_INITIALIZED: Mutex<bool> = Mutex::new(false);

fn capture_clean_bg(sw: usize, sh: usize) {
    let margin = 4;
    let gx = ((sw as i32 - 520) / 2 - margin).max(0) as usize;
    let gy = ((sh as i32 - 340) / 2 - margin).max(0) as usize;

    let mut patch = CLEAN_WINDOW_BG.lock();
    framebuffer::capture_into(patch.as_mut_slice(), gx, gy, BG_PATCH_W, BG_PATCH_H);
    *BG_IS_INITIALIZED.lock() = true;
}

fn restore_clean_bg(sw: usize, sh: usize) {
    if !*BG_IS_INITIALIZED.lock() {
        return;
    }
    let margin = 4;
    let gx = ((sw as i32 - 520) / 2 - margin).max(0) as usize;
    let gy = ((sh as i32 - 340) / 2 - margin).max(0) as usize;

    let patch = CLEAN_WINDOW_BG.lock();
    framebuffer::blit(gx, gy, BG_PATCH_W, BG_PATCH_H, patch.as_slice());
}

fn redraw_active(widget: &mut dyn Widget, mx: i32, my: i32) {
    framebuffer::restore_under_cursor();
    wm::route_draw(widget);
    framebuffer::save_under_cursor(mx as usize, my as usize);
    framebuffer::draw_cursor_arrow(mx as usize, my as usize);
}

pub fn run() {
    mouse::init();
    let (sw, sh) = framebuffer::dimensions();

    let icons = [
        DesktopIcon {
            x: 30,
            y: 30,
            label: "Terminal",
            app: App::Terminal,
        },
        DesktopIcon {
            x: 30,
            y: 120,
            label: "Not-Google",
            app: App::Browser,
        },
        DesktopIcon {
            x: 30,
            y: 210,
            label: "Clock",
            app: App::Clock,
        },
        DesktopIcon {
            x: 30,
            y: 300,
            label: "Calc",
            app: App::Calc,
        },
        DesktopIcon {
            x: 30,
            y: 390,
            label: "Paint",
            app: App::Paint,
        },
    ];

    let geom = WindowGeom::new((sw - 520) / 2, (sh - 340) / 2, 520, 340);
    let mut active: Option<Box<dyn Widget>> = None;

    // draw desktop
    compose_desktop(&icons);

    // capture bg
    capture_clean_bg(sw, sh);

    // enable mouse
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
            framebuffer::restore_under_cursor();
            if left {
                if let Some(w) = active.as_mut() {
                    wm::route_drag(w.as_mut(), mx, my);
                }
            }
            framebuffer::save_under_cursor(mx as usize, my as usize);
            framebuffer::draw_cursor_arrow(mx as usize, my as usize);
            last_mx = mx;
            last_my = my;
        }

        if let Some(w) = active.as_mut() {
            if tick_frame % 40_000 == 0 && wm::route_tick(w.as_mut()) {
                redraw_active(w.as_mut(), mx, my);
            }
        }
        tick_frame = tick_frame.wrapping_add(1);

        if left && !last_left {
            framebuffer::restore_under_cursor();
            handle_click(&icons, geom, &mut active, mx, my, sw, sh);
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

fn handle_click(
    icons: &[DesktopIcon],
    geom: WindowGeom,
    active: &mut Option<Box<dyn Widget>>,
    mx: i32,
    my: i32,
    sw: usize,
    sh: usize,
) {
    enum Action {
        None,
        Close,
        Redraw,
        Open(usize),
    }

    let action = if let Some(w) = active.as_mut() {
        if wm::close_hit(geom, mx, my) {
            Action::Close
        } else if wm::route_click(w.as_mut(), mx, my) {
            wm::route_draw(w.as_mut());
            Action::Redraw
        } else {
            Action::None
        }
    } else {
        match icons.iter().position(|icon| icon.hit(mx, my)) {
            Some(i) => Action::Open(i),
            None => Action::None,
        }
    };

    match action {
        Action::Close => {
            *active = None;
            // restore bg
            restore_clean_bg(sw, sh);
        }
        Action::Open(i) => {
            let icon = &icons[i];
            wm::draw_frame(geom, icon.app.title());
            let mut w = icon.app.spawn(geom.content_area());
            wm::route_draw(w.as_mut());
            *active = Some(w);
        }
        Action::Redraw | Action::None => {}
    }
}
