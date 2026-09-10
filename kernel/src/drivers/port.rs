// io port access via inline asm

use core::arch::asm;

#[inline]
pub fn inb(port: u16) -> u8 {
    // read byte from port
    let val: u8;
    unsafe {
        asm!("in al, dx", out("al") val, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    val
}

#[inline]
pub fn outb(port: u16, val: u8) {
    // write byte to port
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub fn inw(port: u16) -> u16 {
    // read 16 bits
    let val: u16;
    unsafe {
        asm!("in ax, dx", out("ax") val, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    val
}

#[inline]
pub fn outw(port: u16, val: u16) {
    // write 16 bits
    unsafe {
        asm!("out dx, ax", in("dx") port, in("ax") val, options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub fn inl(port: u16) -> u32 {
    // read 32 bits for pci
    let val: u32;
    unsafe {
        core::arch::asm!("in eax, dx", out("eax") val, in("dx") port,
            options(nomem, nostack, preserves_flags));
    }
    val
}

#[inline]
pub fn outl(port: u16, val: u32) {
    // write 32 bits
    unsafe {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") val,
            options(nomem, nostack, preserves_flags));
    }
}
