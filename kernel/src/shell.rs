// tty shell простой repl с папками
use crate::editor;
use crate::fs::{self, FILE_MAX_BYTES};
use crate::keyboard::{self, KEY_BACKSPACE, KEY_ENTER};
use crate::{print, println, print_color, framebuffer};
use crate::framebuffer::{GREEN, RED, CYAN, YELLOW, GRAY};
use crate::allocator;
use alloc::vec::Vec;
use alloc::string::String;

const LINE_MAX: usize = 128;

pub fn run() -> ! {
    println!();
    print_color!(GREEN, "IluminOS shell. type help.\n");
    loop {
        print_prompt();
        let mut line = [0u8; LINE_MAX];
        let len = read_line(&mut line);
        if let Ok(s) = core::str::from_utf8(&line[..len]) {
            handle(s.trim());
        }
    }
}

// приглашение показывает текущий путь foo bar
fn print_prompt() {
    let mut buf = [0u8; 256];
    let n = fs::pwd_into(&mut buf);
    if let Ok(path) = core::str::from_utf8(&buf[..n]) {
        print_color!(YELLOW, "{}", path);
    }
    print_color!(CYAN, " > ");
}

fn read_line(buf: &mut [u8; LINE_MAX]) -> usize {
    let mut len = 0;
    let mut blink: u32 = 0;
    loop {
        if let Some(key) = keyboard::try_read_key() {
            framebuffer::hide_cursor();
            match key {
                KEY_ENTER => {
                    println!();
                    return len;
                }
                KEY_BACKSPACE => {
                    if len > 0 {
                        len -= 1;
                        print!("{}", 0x08 as char);
                    }
                }
                0x20..=0x7e => {
                    if len < LINE_MAX {
                        buf[len] = key;
                        len += 1;
                        print!("{}", key as char);
                    }
                }
                _ => {}
            }
            blink = 0;
        } else {
            blink = blink.wrapping_add(1);
            if blink >= 3_000_000 {
                framebuffer::toggle_cursor();
                blink = 0;
            }
            core::hint::spin_loop();
        }
    }
}

fn handle(line: &str) {
    if line.is_empty() {
        return;
    }
    let (cmd, arg) = match line.find(' ') {
        Some(i) => (&line[..i], line[i + 1..].trim()),
        None => (line, ""),
    };

    match cmd {
        "help" => cmd_help(),
        "clear" => framebuffer::clear(),
        "ls" => cmd_ls(),
        "echo" => cmd_echo(arg),
        "touch" => cmd_touch(arg),
        "cat" => cmd_cat(arg),
        "edit" => cmd_edit(arg),
        "rm" => cmd_rm(arg),
        "df" => cmd_df(),
        "mkdir" => cmd_mkdir(arg),
        "cd" => cmd_cd(arg),
        "pwd" => cmd_pwd(),
        "mem" => cmd_mem(),
        "memtest" => cmd_memtest(),
        "wasm" => crate::wasm::run_demo(),
        "run" => cmd_run(arg),
        "rand" => cmd_rand(arg),
        "gui" => cmd_gui(),
        _ => print_color!(RED, "unknown command: {}. type help\n", cmd),
    }
}

fn cmd_help() {
print_color!(GREEN, "+-- System Help ------------------------------------------+\n");
println!("|  COMMAND        ARGS          DESCRIPTION               |");
print_color!(GREEN, "+-- File System ------------------------------------------+\n");
println!("|                                                         |");
println!("|  ls / pwd                     List contents / print path|");
println!("|  cd             <dir>         Change current directory  |");
println!("|  cat / edit     <file>        View or edit file         |");
println!("|  touch / mkdir  <name>        Create empty file / dir   |");
println!("|  rm             <name>        Delete file or empty dir  |");
println!("|  df                           Disk usage statistics     |");
println!("|                                                         |");
print_color!(YELLOW, "+-- Executables & Dev ------------------------------------+\n");
println!("|                                                         |");
println!("|  run            <file>        Execute script file       |");
println!("|  wasm                         Run WebAssembly module    |");
println!("|  mem / memtest                Memory state & diagnostics|");
println!("|                                                         |");
print_color!(RED, "+-- Core & Utilities -------------------------------------+ \n");
println!("|                                                         |");
println!("|  echo           <text>        Write text into console   |");
println!("|  rand           [max]         Random number (hardware)  |");
println!("|  gui                          Switch to graphic desktop |");
println!("|  clear / help                 Control terminal session  |");
println!("|                                                         |");
print_color!(GREEN, "+---------------------------------------------------------+\n");
}

fn cmd_ls() {
    let mut count = 0;
    fs::list(|name, size, is_dir| {
        if is_dir {
            print_color!(YELLOW, "{}/\n", name);
        } else {
            print_color!(CYAN, "{}", name);
            println!("  ({} bytes)", size);
        }
        count += 1;
    });
    if count == 0 {
        print_color!(GRAY, "(empty)\n");
    }
}

fn cmd_echo(arg: &str) {
    println!("{}", arg);
}

fn cmd_pwd() {
    let mut buf = [0u8; 256];
    let n = fs::pwd_into(&mut buf);
    if let Ok(path) = core::str::from_utf8(&buf[..n]) {
        println!("{}", path);
    }
}

fn cmd_mkdir(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: mkdir <name>\n");
        return;
    }
    match fs::mkdir(arg) {
        Ok(()) => print_color!(GREEN, "created dir: {}\n", arg),
        Err(e) => print_color!(RED, "error: {}\n", e),
    }
}

fn cmd_cd(arg: &str) {
    if arg.is_empty() {
        // cd без аргумента в корень
        let _ = fs::chdir("/");
        return;
    }
    match fs::chdir(arg) {
        Ok(()) => {}
        Err(e) => print_color!(RED, "error: {}\n", e),
    }
}

fn cmd_touch(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: touch <name>\n");
        return;
    }
    match fs::create(arg) {
        Ok(()) => print_color!(GREEN, "created: {}\n", arg),
        Err(e) => print_color!(RED, "error: {}\n", e),
    }
}

fn cmd_cat(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: cat <name>\n");
        return;
    }
    let mut buf = [0u8; FILE_MAX_BYTES];
    match fs::read(arg, &mut buf) {
        Ok(size) => {
            if let Ok(s) = core::str::from_utf8(&buf[..size]) {
                println!("{}", s);
            } else {
                print_color!(RED, "(binary data)\n");
            }
        }
        Err(e) => print_color!(RED, "error: {}\n", e),
    }
}

fn cmd_rm(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: rm <name>\n");
        return;
    }
    match fs::remove(arg) {
        Ok(()) => print_color!(GREEN, "removed: {}\n", arg),
        Err(e) => print_color!(RED, "error: {}\n", e),
    }
}

fn cmd_df() {
    let used = fs::used_blocks();
    let total = fs::total_blocks();
    let free = total - used;
    println!("blocks: {} used, {} free, {} total", used, free, total);
    println!("({} KB used of {} KB)", used / 2, total / 2);
}

fn cmd_mem() {
    let total = allocator::heap_size();
    let used = allocator::heap_used();
    let free = allocator::heap_free();
    println!("heap: {} bytes total", total);
    println!("      {} used, {} free", used, free);
}

// демонстрация динамической памяти строим Vec и String во время работы
fn cmd_memtest() {
    print_color!(YELLOW, "building a Vec at runtime...\n");
    let mut v: Vec<u32> = Vec::new();
    for i in 0..10 {
        v.push(i * i);
    }
    print!("squares: ");
    for x in &v {
        print!("{} ", x);
    }
    println!();

    print_color!(YELLOW, "building a String at runtime...\n");
    let mut s = String::new();
    s.push_str("hello ");
    s.push_str("from ");
    s.push_str("the heap!");
    println!("{}", s);

    print_color!(GREEN, "dynamic memory works! ({} bytes in Vec)\n", v.len() * 4);
    // v и s освобождаются автоматически при выходе из функции Drop
}

fn cmd_run(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: run <file>\n");
        return;
    }
    crate::script::run_file(arg);
}

fn cmd_rand(arg: &str) {
    if arg.is_empty() {
        // без аргумента случайное u64
        let r = crate::random::next_u64();
        print_color!(GREEN, "{}\n", r);
    } else {
        // с аргументом число в диапазоне 0 max
        let max = parse_u64(arg);
        if max == 0 {
            print_color!(RED, "usage: rand [max]  (max > 0)\n");
            return;
        }
        let r = crate::random::next_range(max);
        print_color!(GREEN, "{}\n", r);
    }
}

// простой парсер числа из строки
fn parse_u64(s: &str) -> u64 {
    let mut n: u64 = 0;
    for b in s.bytes() {
        if b >= b'0' && b <= b'9' {
            n = n.wrapping_mul(10).wrapping_add((b - b'0') as u64);
        } else {
            return 0;
        }
    }
    n
}

fn cmd_gui() {
    crate::gui::run();
    // после выхода из GUI очищаем экран и возвращаем консоль
    framebuffer::clear();
    print_color!(GREEN, "back to console.\n");
}

fn cmd_edit(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: edit <name>\n");
        return;
    }
    if fs::find(arg).is_none() {
        if let Err(e) = fs::create(arg) {
            print_color!(RED, "error: {}\n", e);
            return;
        }
    }
    editor::run(arg);
}
