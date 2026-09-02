// системный монитор в стиле htop команда htop

use crate::framebuffer::{self, fill_rect, draw_text_at, dimensions};
use crate::keyboard;
use crate::allocator;
use crate::fs;
use crate::time;
use crate::framebuffer::{GREEN, RED, YELLOW, CYAN, GRAY, WHITE};
use alloc::string::String;

const BAR_BG: u32 = 0x222222; // пустая часть бара
const BORDER: u32 = 0x444444; // рамка бара
const BG: u32 = 0x000000;     // фон

// координаты чтобы статику и динамику рисовать в одни места
const LEFT: usize = 20;
const Y_UPTIME: usize = 60;
const Y_MEM_LABEL: usize = 90;
const Y_MEM_BAR: usize = 104;
const Y_MEM_KB: usize = 122;
const Y_DISK_LABEL: usize = 150;
const Y_DISK_BAR: usize = 164;
const Y_DISK_INFO: usize = 182;
const Y_FILES: usize = 210;
const Y_CPU: usize = 228;
const Y_SCREEN: usize = 246;

// запуск крутится пока не нажмут q Esc
pub fn run() {
    // один раз фон подписи рамки
    draw_static();

    let mut frame: u32 = 0;
    loop {
        if let Some(key) = keyboard::try_read_key() {
            if key == b'q' || key == 0x1b {
                break;
            }
        }
        // обновляем только меняющиеся значения
        if frame % 400_000 == 0 {
            draw_dynamic();
        }
        frame = frame.wrapping_add(1);
        core::hint::spin_loop();
    }

    framebuffer::clear();
    crate::banner::show();
    crate::print_color!(GREEN, "monitor closed.\n");
}

// рисуем то что не меняется один раз
fn draw_static() {
    let (w, h) = dimensions();
    fill_rect(0, 0, w, h, BG); // единственная полная заливка

    draw_text_at("IluminOS System Monitor", LEFT, 20, GREEN);
    draw_text_at("[q] quit", w - 90, 20, GRAY);

    // подписи метрик
    draw_text_at("Uptime:", LEFT, Y_UPTIME, CYAN);
    draw_text_at("Memory", LEFT, Y_MEM_LABEL, CYAN);
    draw_text_at("Disk", LEFT, Y_DISK_LABEL, CYAN);
    draw_text_at("Files:", LEFT, Y_FILES, CYAN);
    draw_text_at("CPU:", LEFT, Y_CPU, CYAN);
    draw_text_at("Screen:", LEFT, Y_SCREEN, CYAN);
}

// обновить только меняющиеся значения
fn draw_dynamic() {
    let (w, _h) = dimensions();
    let bar_w = w - 60;
    let val_x = LEFT + 80; // где начинаются значения

    // uptime
    let (uh, um, us) = time::uptime_hms();
    let mut up = String::new();
    push_num(&mut up, uh as usize); up.push_str("h ");
    push_num2(&mut up, um as usize); up.push_str("m ");
    push_num2(&mut up, us as usize); up.push('s');
    clear_val(val_x, Y_UPTIME, 160);          // затереть старое
    draw_text_at(&up, val_x, Y_UPTIME, WHITE);

    // память
    let mem_used = allocator::heap_used();
    let mem_total = allocator::heap_size();
    let mem_pct = percent(mem_used, mem_total);
    let mut mp = String::new(); push_num(&mut mp, mem_pct); mp.push('%');
    clear_val(LEFT + 64, Y_MEM_LABEL, 48);
    draw_text_at(&mp, LEFT + 64, Y_MEM_LABEL, load_color(mem_pct));
    // бар сам себя перерисовывает
    draw_bar(LEFT, Y_MEM_BAR, bar_w, mem_pct, load_color(mem_pct));
    let mut ml = String::new();
    push_num(&mut ml, mem_used / 1024); ml.push_str(" KB / ");
    push_num(&mut ml, mem_total / 1024); ml.push_str(" KB");
    clear_val(LEFT, Y_MEM_KB, bar_w);
    draw_text_at(&ml, LEFT, Y_MEM_KB, GRAY);

    // диск
    let disk_used = fs::used_blocks() as usize;
    let disk_total = fs::total_blocks() as usize;
    let disk_pct = percent(disk_used, disk_total);
    let mut dp = String::new(); push_num(&mut dp, disk_pct); dp.push('%');
    clear_val(LEFT + 64, Y_DISK_LABEL, 48);
    draw_text_at(&dp, LEFT + 64, Y_DISK_LABEL, load_color(disk_pct));
    draw_bar(LEFT, Y_DISK_BAR, bar_w, disk_pct, load_color(disk_pct));
    let mut dl = String::new();
    push_num(&mut dl, disk_used); dl.push_str(" / ");
    push_num(&mut dl, disk_total); dl.push_str(" blocks");
    clear_val(LEFT, Y_DISK_INFO, bar_w);
    draw_text_at(&dl, LEFT, Y_DISK_INFO, GRAY);

    // файлы
    let mut fc = 0usize;
    fs::list(|_n, _s, _d| { fc += 1; });
    let mut fl = String::new(); push_num(&mut fl, fc);
    clear_val(val_x, Y_FILES, 80);
    draw_text_at(&fl, val_x, Y_FILES, WHITE);

    // такты CPU
    let ticks = time::ticks_since_boot();
    let mut cl = String::new(); push_num(&mut cl, ticks as usize); cl.push_str(" ticks");
    clear_val(val_x, Y_CPU, 200);
    draw_text_at(&cl, val_x, Y_CPU, GRAY);

    // разрешение
    let mut sl = String::new();
    push_num(&mut sl, w); sl.push('x'); push_num(&mut sl, _h);
    clear_val(val_x, Y_SCREEN, 120);
    draw_text_at(&sl, val_x, Y_SCREEN, GRAY);
}

// затереть маленькую область под значением
fn clear_val(x: usize, y: usize, w: usize) {
    fill_rect(x, y, w, 10, BG);
}

// графический бар сам перерисовывается целиком
fn draw_bar(x: usize, y: usize, w: usize, pct: usize, color: u32) {
    let bh = 12;
    fill_rect(x, y, w, bh, BORDER);
    fill_rect(x + 1, y + 1, w - 2, bh - 2, BAR_BG);
    let fill_w = ((w - 2) * pct.min(100)) / 100;
    if fill_w > 0 {
        fill_rect(x + 1, y + 1, fill_w, bh - 2, color);
    }
}

fn percent(used: usize, total: usize) -> usize {
    if total == 0 { return 0; }
    (used * 100) / total
}

fn load_color(pct: usize) -> u32 {
    if pct < 60 { GREEN } else if pct < 85 { YELLOW } else { RED }
}

fn push_num(s: &mut String, mut v: usize) {
    if v == 0 { s.push('0'); return; }
    let mut tmp = [0u8; 20];
    let mut i = 0;
    while v > 0 { tmp[i] = b'0' + (v % 10) as u8; v /= 10; i += 1; }
    while i > 0 { i -= 1; s.push(tmp[i] as char); }
}

fn push_num2(s: &mut String, v: usize) {
    s.push((b'0' + (v / 10 % 10) as u8) as char);
    s.push((b'0' + (v % 10) as u8) as char);
}
