// tty shell простой repl с папками
use crate::editor;
use crate::fs::{self, FILE_MAX_BYTES, NAME_MAX};
use crate::keyboard::{self, KEY_BACKSPACE, KEY_ENTER, KEY_TAB, KEY_UP, KEY_DOWN};
use crate::{print, println, print_color, framebuffer};
use crate::framebuffer::{GREEN, RED, CYAN, YELLOW, GRAY, BLUE, MAGENTA, WHITE};
use crate::allocator;
use crate::time;
use alloc::vec::Vec;
use alloc::string::String;

const LINE_MAX: usize = 128;
const HISTORY_MAX: usize = 32;

// все известные команды (для автодополнения и списка)
const COMMANDS: &[&str] = &[
    "help", "clear", "cls", "ls", "ll", "echo", "touch", "cat", "edit", "rm",
    "df", "mkdir", "cd", "pwd", "mem", "memtest", "wasm", "run", "rand", "gui",
    "uptime", "whoami", "hostname", "theme", "history", "cowsay", "calc",
    "date", "about", "tree", "wc", "find", "cp", "lspci", "nic", "ping", "htop", "piano",
    "lock", "beep", "reboot", "shutdown", "neofetch", "sleep", "dice", "banner",
];

// история команд и счётчик выполненных
struct ShellState {
    history: Vec<String>,
    count: u32,
}

static mut SHELL: Option<ShellState> = None;

fn state() -> &'static mut ShellState {
    unsafe {
        let ptr = core::ptr::addr_of_mut!(SHELL);
        if (*ptr).is_none() {
            *ptr = Some(ShellState { history: Vec::new(), count: 0 });
        }
        (*ptr).as_mut().unwrap()
    }
}

pub fn run() -> ! {
    println!();
    print_color!(GREEN, "IluminOS shell. type help.\n");
    loop {
        print_prompt();
        let mut line = [0u8; LINE_MAX];
        let len = read_line(&mut line);
        if let Ok(s) = core::str::from_utf8(&line[..len]) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                push_history(trimmed);
                state().count += 1;
            }
            handle(trimmed);
        }
    }
}

fn push_history(line: &str) {
    let st = state();
    // не дублируем подряд одинаковые
    if st.history.last().map(|s| s.as_str()) == Some(line) {
        return;
    }
    st.history.push(String::from(line));
    if st.history.len() > HISTORY_MAX {
        st.history.remove(0);
    }
}

// приглашение: [N] путь >
fn print_prompt() {
    let n = state().count;
    print_color!(GRAY, "[{}] ", n);
    let mut buf = [0u8; 256];
    let bn = fs::pwd_into(&mut buf);
    if let Ok(path) = core::str::from_utf8(&buf[..bn]) {
        print_color!(YELLOW, "{}", path);
    }
    print_color!(CYAN, " > ");
}

fn read_line(buf: &mut [u8; LINE_MAX]) -> usize {
    let mut len = 0;
    let mut blink: u32 = 0;
    // индекс просмотра истории: len истории = "текущая пустая строка"
    let mut hist_idx = state().history.len();
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
                KEY_TAB => {
                    len = tab_complete(buf, len);
                }
                KEY_UP => {
                    let st = state();
                    if hist_idx > 0 {
                        hist_idx -= 1;
                        len = replace_line(buf, len, &st.history[hist_idx]);
                    }
                }
                KEY_DOWN => {
                    let st = state();
                    if hist_idx < st.history.len() {
                        hist_idx += 1;
                        if hist_idx == st.history.len() {
                            len = replace_line(buf, len, "");
                        } else {
                            len = replace_line(buf, len, &st.history[hist_idx]);
                        }
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

// стереть текущую строку с экрана и напечатать новую
fn replace_line(buf: &mut [u8; LINE_MAX], cur_len: usize, new: &str) -> usize {
    for _ in 0..cur_len {
        print!("{}", 0x08 as char);
    }
    let nb = new.as_bytes();
    let n = nb.len().min(LINE_MAX);
    buf[..n].copy_from_slice(&nb[..n]);
    for i in 0..n {
        print!("{}", buf[i] as char);
    }
    n
}

// автодополнение имени команды по Tab (дополняем только первое слово)
fn tab_complete(buf: &mut [u8; LINE_MAX], len: usize) -> usize {
    if buf[..len].contains(&b' ') {
        return len;
    }
    if let Ok(prefix) = core::str::from_utf8(&buf[..len]) {
        let mut matches: Vec<&str> = Vec::new();
        for c in COMMANDS {
            if c.starts_with(prefix) {
                matches.push(c);
            }
        }
        if matches.len() == 1 {
            return replace_line(buf, len, matches[0]);
        } else if matches.len() > 1 {
            println!();
            for m in &matches {
                print_color!(CYAN, "{}  ", m);
            }
            println!();
            print_prompt();
            let cur = String::from(prefix);
            return replace_line(buf, 0, &cur);
        }
    }
    len
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
        "clear" | "cls" => cmd_clear(),
        "ls" | "ll" => cmd_ls(),
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
        "uptime" => cmd_uptime(),
        "whoami" => cmd_whoami(),
        "hostname" => cmd_hostname(),
        "theme" => cmd_theme(arg),
        "history" => cmd_history(),
        "cowsay" => cmd_cowsay(arg),
        "calc" => cmd_calc(arg),
        "date" => cmd_date(),
        "about" => cmd_about(),
        "tree" => cmd_tree(),
        "wc" => cmd_wc(arg),
        "find" => cmd_find(arg),
        "cp" => cmd_cp(arg),
        "lspci" => cmd_lspci(),
        "nic" => cmd_nic(),
        "ping" => cmd_ping(arg),
        "htop" | "monitor" => crate::monitor::run(),
        "piano" => crate::piano::run(),
        "lock" => cmd_lock(),
        "beep" => cmd_beep(),
        "reboot" => cmd_reboot(),
        "shutdown" => cmd_shutdown(),
        "neofetch" => cmd_neofetch(),
        "sleep" => cmd_sleep(arg),
        "dice" => cmd_dice(),
        "banner" => crate::banner::show(),
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
println!("|  cp             <src> <dst>   Copy a file               |");
println!("|  tree / find    <name>        Show tree / search file   |");
println!("|  wc             <file>        Count lines/words/chars   |");
println!("|  df                           Disk usage statistics     |");
println!("|                                                         |");
print_color!(YELLOW, "+-- Executables & Dev ------------------------------------+\n");
println!("|                                                         |");
println!("|  run            <file>        Execute script file       |");
println!("|  wasm                         Run WebAssembly module    |");
println!("|  calc           <expr>        Quick arithmetic          |");
println!("|  mem / memtest                Memory state & diagnostics|");
println!("|                                                         |");
print_color!(RED, "+-- Core & Utilities -------------------------------------+ \n");
println!("|                                                         |");
println!("|  echo           <text>        Write text into console   |");
println!("|  rand           [max]         Random number (hardware)  |");
println!("|  cowsay         <text>        ASCII cow says text        |");
println!("|  uptime / date                Time since boot           |");
println!("|  whoami/hostname              System identity           |");
println!("|  theme      everforest|dark|light Switch color theme    |");
println!("|  history                      Show command history      |");
println!("|  about                        System info & version     |");
println!("|  gui                          Switch to graphic desktop |");
println!("|  clear / help                 Control terminal session  |");
println!("|  lock / neofetch              Lock screen / sysinfo    |");
println!("|  beep / dice / sleep <n>      Sound / random / pause   |");
println!("|  reboot / shutdown            Power control            |");
println!("|                                                         |");
print_color!(GREEN, "+---------------------------------------------------------+\n");
}

fn cmd_clear() {
    framebuffer::clear();
    crate::banner::show();
}

fn cmd_ls() {
    let mut count = 0;
    fs::list(|name, size, is_dir| {
        if is_dir {
            print_color!(BLUE, "{}/\n", name);
        } else {
            let color = color_for_file(name);
            print_color!(color, "{}", name);
            print_color!(GRAY, "  ({} bytes)\n", size);
        }
        count += 1;
    });
    if count == 0 {
        print_color!(GRAY, "(empty)\n");
    }
}

// цвет файла по расширению
fn color_for_file(name: &str) -> u32 {
    if name.ends_with(".txt") || name.ends_with(".md") {
        WHITE
    } else if name.ends_with(".rs") || name.ends_with(".c") || name.ends_with(".wasm") {
        GREEN
    } else if name.ends_with(".sh") || name.ends_with(".script") {
        YELLOW
    } else if name.ends_with(".html") || name.ends_with(".htm") {
        MAGENTA
    } else {
        CYAN
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
        let r = crate::random::next_u64();
        print_color!(GREEN, "{}\n", r);
    } else {
        let max = parse_u64(arg);
        if max == 0 {
            print_color!(RED, "usage: rand [max]  (max > 0)\n");
            return;
        }
        let r = crate::random::next_range(max);
        print_color!(GREEN, "{}\n", r);
    }
}

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


fn cmd_uptime() {
    let (h, m, s) = time::uptime_hms();
    let ticks = time::ticks_since_boot();
    print_color!(GREEN, "up ");
    println!("{}h {}m {}s  ({} tsc ticks)", h, m, s, ticks);
}

fn cmd_whoami() {
    print_color!(CYAN, "root\n");
}

fn cmd_hostname() {
    print_color!(CYAN, "iluminos\n");
}

fn cmd_date() {
    let (h, m, s) = time::uptime_hms();
    print_color!(YELLOW, "{:02}:{:02}:{:02}\n", h, m, s);
}

fn cmd_theme(arg: &str) {
    match arg {
        "everforest" | "" => {
            framebuffer::set_theme(framebuffer::EVERFOREST_FOREGROUND, framebuffer::EVERFOREST_BACKGROUND);
            framebuffer::clear();
            crate::banner::show();
            print_color!(GREEN, "theme: everforest\n");
        }
        "dark" => {
            framebuffer::set_theme(WHITE, framebuffer::BLACK);
            framebuffer::clear();
            crate::banner::show();
            print_color!(GREEN, "theme: dark\n");
        }
        "light" => {
            framebuffer::set_theme(framebuffer::BLACK, WHITE);
            framebuffer::clear();
            crate::banner::show();
            print_color!(GREEN, "theme: light\n");
        }
        _ => print_color!(RED, "usage: theme everforest|dark|light\n"),
    }
}

fn cmd_history() {
    let st = state();
    if st.history.is_empty() {
        print_color!(GRAY, "(no history)\n");
        return;
    }
    for (i, cmd) in st.history.iter().enumerate() {
        print_color!(GRAY, "{:>3}  ", i + 1);
        println!("{}", cmd);
    }
}

fn cmd_cowsay(arg: &str) {
    let text = if arg.is_empty() { "moo" } else { arg };
    let len = text.chars().count();
    print!(" ");
    for _ in 0..len + 2 { print!("_"); }
    println!();
    print_color!(WHITE, "< {} >\n", text);
    print!(" ");
    for _ in 0..len + 2 { print!("-"); }
    println!();
    print_color!(GRAY, "        \\   ^__^\n");
    print_color!(GRAY, "         \\  (oo)\\_______\n");
    print_color!(GRAY, "            (__)\\       )\\/\\\n");
    print_color!(GRAY, "                ||----w |\n");
    print_color!(GRAY, "                ||     ||\n");
}

fn cmd_calc(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: calc <expr>   e.g. calc 2 + 3 * 4\n");
        return;
    }
    match crate::script::eval_expr(arg) {
        Ok(v) => print_color!(GREEN, "{}\n", v),
        Err(e) => print_color!(RED, "error: {}\n", e),
    }
}

fn cmd_about() {
    print_color!(CYAN,    "   _ _ _             _        ___  ___\n");
    print_color!(CYAN,    "  (_) | |_  _ _ __ (_)_ _  __/ _ \\/ __|\n");
    print_color!(BLUE,    "  | | | | || | '  \\| | ' \\/ _ \\ (_) \\__ \\\n");
    print_color!(MAGENTA, "  |_|_|_|\\_,_|_|_|_|_|_||_\\___/\\___/|___/\n");
    println!();
    print_color!(GREEN, "IluminOS ");
    println!("v0.2");
    print_color!(GRAY, "a tiny 64-bit OS in Rust by IbrokhimN\n");
    print_color!(GRAY, "framebuffer + fs + wasm + gui\n");
}

fn cmd_tree() {
    print_color!(BLUE, ".\n");
    tree_walk(fs::cwd(), 0);
}

fn tree_walk(dir: usize, depth: usize) {
    fs::list_dir(dir, |idx, name, _size, is_dir| {
        for _ in 0..depth { print!("  "); }
        print_color!(GRAY, "|- ");
        if is_dir {
            print_color!(BLUE, "{}/\n", name);
            if depth < 8 {
                tree_walk(idx, depth + 1);
            }
        } else {
            let color = color_for_file(name);
            print_color!(color, "{}\n", name);
        }
    });
}

fn cmd_wc(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: wc <file>\n");
        return;
    }
    let mut buf = [0u8; FILE_MAX_BYTES];
    match fs::read(arg, &mut buf) {
        Ok(size) => {
            let mut lines = 0usize;
            let mut words = 0usize;
            let mut in_word = false;
            for &b in &buf[..size] {
                if b == b'\n' { lines += 1; }
                if b == b' ' || b == b'\n' || b == b'\t' {
                    in_word = false;
                } else if !in_word {
                    in_word = true;
                    words += 1;
                }
            }
            if size > 0 && buf[size - 1] != b'\n' {
                lines += 1;
            }
            print_color!(GREEN, "{}", lines);
            print!("  ");
            print_color!(GREEN, "{}", words);
            print!("  ");
            print_color!(GREEN, "{}", size);
            print_color!(GRAY, "   {}\n", arg);
        }
        Err(e) => print_color!(RED, "error: {}\n", e),
    }
}

fn cmd_find(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: find <name>\n");
        return;
    }
    let mut found = 0u32;
    fs::find_recursive(fs::cwd(), arg, &mut |idx, is_dir| {
        found += 1;
        let mut nb = [0u8; NAME_MAX];
        let n = fs::name_of(idx, &mut nb);
        if let Ok(name) = core::str::from_utf8(&nb[..n]) {
            if is_dir {
                print_color!(BLUE, "{}/\n", name);
            } else {
                print_color!(CYAN, "{}\n", name);
            }
        }
    });
    if found == 0 {
        print_color!(GRAY, "not found: {}\n", arg);
    }
}

fn cmd_cp(arg: &str) {
    let parts: Vec<&str> = arg.split_whitespace().collect();
    if parts.len() != 2 {
        print_color!(RED, "usage: cp <src> <dst>\n");
        return;
    }
    let (src, dst) = (parts[0], parts[1]);
    let mut buf = [0u8; FILE_MAX_BYTES];
    let size = match fs::read(src, &mut buf) {
        Ok(s) => s,
        Err(e) => { print_color!(RED, "error: {}\n", e); return; }
    };
    if fs::find(dst).is_none() {
        if let Err(e) = fs::create(dst) {
            print_color!(RED, "error: {}\n", e);
            return;
        }
    }
    match fs::write(dst, &buf[..size]) {
        Ok(()) => print_color!(GREEN, "copied {} -> {} ({} bytes)\n", src, dst, size),
        Err(e) => print_color!(RED, "error: {}\n", e),
    }
}

// найти сетевую карту RTL8139 через PCI
fn cmd_lspci() {
    match crate::tcp::pci::find_device(
        crate::tcp::pci::RTL8139_VENDOR,
        crate::tcp::pci::RTL8139_DEVICE,
    ) {
        Some(dev) => {
            print_color!(GREEN, "found RTL8139\n");
            println!("  vendor:device = {:04x}:{:04x}", dev.vendor_id, dev.device_id);
            println!("  bar0 = {:#010x}", dev.bar0);
            println!("  irq  = {}", dev.irq_line);
        }
        None => print_color!(RED, "no RTL8139 (нужен флаг QEMU: -device rtl8139)\n"),
    }
}

// разбудить карту и прочитать MAC
fn cmd_nic() {
    if !crate::tcp::rtl8139::init() {
        print_color!(RED, "no network card (нужен -device rtl8139 в QEMU)\n");
        return;
    }
    print_color!(GREEN, "card initialized\n");
    match crate::tcp::rtl8139::mac_address() {
        Some(mac) => {
            print!("  MAC = ");
            for (i, b) in mac.iter().enumerate() {
                if i > 0 { print!(":"); }
                print!("{:02x}", b);
            }
            println!();
        }
        None => print_color!(RED, "  failed to read MAC\n"),
    }
}

// пропинговать IP ICMP echo
fn cmd_ping(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: ping <ip>   (напр. ping 10.0.2.2)\n");
        return;
    }
    crate::tcp::net::cmd_ping(arg);
}

// заблокировать экран вернуться на вход
fn cmd_lock() {
    crate::login::run();
    framebuffer::clear();
    crate::banner::show();
    print_color!(GREEN, "unlocked.\n");
}

// короткий тестовый писк
fn cmd_beep() {
    crate::sound::beep(880, 1);
    print_color!(GRAY, "beep!\n");
}

// перезагрузка через контроллер клавиатуры порт 0x64
fn cmd_reboot() {
    print_color!(YELLOW, "rebooting...\n");
    crate::sound::delay(2);
    crate::port::outb(0x64, 0xFE); // pulse reset
    loop { core::hint::spin_loop(); }
}

// выключение в QEMU через ACPI порт 0x604
fn cmd_shutdown() {
    print_color!(YELLOW, "shutting down...\n");
    crate::sound::delay(2);
    crate::port::outw(0x604, 0x2000);
    // не сработало значит реальное железо
    print_color!(RED, "shutdown not supported on this machine.\n");
    loop { core::hint::spin_loop(); }
}

// сводка о системе как neofetch
fn cmd_neofetch() {
    let total = allocator::heap_size();
    let used = allocator::heap_used();
    let (h, m, s) = time::uptime_hms();
    let disk_used = fs::used_blocks();
    let disk_total = fs::total_blocks();
    let (sw, sh) = framebuffer::dimensions();

    // лого слева инфо справа
    print_color!(CYAN,    "    ___         ");  print_color!(GREEN, "root");
    print_color!(GRAY, "@"); print_color!(GREEN, "iluminos\n");
    print_color!(CYAN,    "   / _ \\        "); print_color!(GRAY, "-----------------\n");
    print_color!(CYAN,    "  | | | |       "); print_color!(YELLOW, "OS:      "); println!("IluminOS v0.2");
    print_color!(CYAN,    "  | | | |       "); print_color!(YELLOW, "Kernel:  "); println!("Rust no_std");
    print_color!(CYAN,    "  | |_| |       "); print_color!(YELLOW, "Uptime:  "); println!("{}h {}m {}s", h, m, s);
    print_color!(CYAN,    "   \\___/        "); print_color!(YELLOW, "Shell:   "); println!("iluminos-sh");
    print_color!(CYAN,    "               ");  print_color!(YELLOW, " Res:     "); println!("{}x{}", sw, sh);
    print_color!(GRAY,    "               ");  print_color!(YELLOW, " Memory:  "); println!("{} / {} KB", used/1024, total/1024);
    print_color!(GRAY,    "               ");  print_color!(YELLOW, " Disk:    "); println!("{} / {} blocks", disk_used, disk_total);
    println!();
    // палитра
    print_color!(RED, "  ###"); print_color!(GREEN, "###"); print_color!(YELLOW, "###");
    print_color!(BLUE, "###"); print_color!(MAGENTA, "###"); print_color!(CYAN, "###");
    print_color!(WHITE, "###\n");
}

// пауза на N единиц грубо секунды
fn cmd_sleep(arg: &str) {
    if arg.is_empty() {
        print_color!(RED, "usage: sleep <n>\n");
        return;
    }
    let n = parse_u64(arg);
    if n == 0 || n > 60 {
        print_color!(RED, "sleep: 1..60\n");
        return;
    }
    print_color!(GRAY, "sleeping {}...\n", n);
    crate::sound::delay(n as u32 * 5);
    print_color!(GREEN, "awake.\n");
}

// бросок кубика 1..6 через рандом
fn cmd_dice() {
    let roll = crate::random::next_range(6) + 1;
    print_color!(YELLOW, "  you rolled: ");
    print_color!(GREEN, "{}\n", roll);
}
