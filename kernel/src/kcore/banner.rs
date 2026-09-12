use crate::framebuffer::{self, GRAY};
use crate::println;

pub fn show() {
    // gradient from cyan to purple
    let colors: [u32; 6] = [
        0x33DDFF, // cyan
        0x44BBFF,
        0x5599FF,
        0x6677FF,
        0x7755FF,
        0x8844FF, // purple
    ];

    let lines: [&str; 6] = [

        r".011010. .111.   10    01 .110  110. .011010. 10 0110.   .1100.  .01000.",
        r"   10     01     10    11 00'1001'11    10    11'    10 .11  01. 10'  11",
        r"   00     00     11    01 01  00  10    00    01     11 10    00 '100.  ",
        r"   10     10     01    10 10  01  10    10    11     01 01    11  '100. ",
        r"   01     00     11    11 00  01  01    01    10     11 '00  11' 01   00",
        r"'010110' .10110'  \0111/  01  10  10 '010110' 01     01  '1001'  '01101'",

    ];

    println!();
    for i in 0..6 {
        framebuffer::set_color(colors[i]);
        println!("{}", lines[i]);
    }
    framebuffer::set_color(GRAY);
    println!();
    println!("        a tiny OS made by IbrokhimN");
    framebuffer::set_color(framebuffer::theme_fg());
    println!();
}
