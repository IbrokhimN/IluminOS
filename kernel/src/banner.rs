use crate::framebuffer::{self, GRAY};
use crate::println;

pub fn show() {
    // градиент от голубого к фиолетовому по строкам
    let colors: [u32; 6] = [
        0x33DDFF, // голубой
        0x44BBFF,
        0x5599FF,
        0x6677FF,
        0x7755FF,
        0x8844FF, // фиолетовый
    ];

    let lines: [&str; 6] = [
        r"  _____ _                 _        ____   _____ ",
        r" |_   _| |               (_)      / __ \ / ____|",
        r"   | | | |_   _ _ __ ___  _ _ __ | |  | | (___  ",
        r"   | | | | | | | '_ ` _ \| | '_ \| |  | |\___ \ ",
        r"  _| |_| | |_| | | | | | | | | | | |__| |____) |",
        r" |_____|_|\__,_|_| |_| |_|_|_| |_|\____/|_____/ ",
    ];

    println!();
    for i in 0..6 {
        framebuffer::set_color(colors[i]);
        println!("{}", lines[i]);
    }
    framebuffer::set_color(GRAY);
    println!();
    println!("        a tiny OS that runs wasm and its own language");
    framebuffer::set_color(framebuffer::WHITE);
    println!();
}
