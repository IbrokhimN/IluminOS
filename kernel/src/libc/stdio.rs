use core::ffi::{CStr, c_char, c_int, VaList};
use core::fmt;

use printf_compat::{format, output};

// консоль ядра
struct Console;

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::print!("{}", s);
        Ok(())
    }
}

struct Buf {
    ptr: *mut u8,
    cap: usize,
    len: usize,
}

impl fmt::Write for Buf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            if self.len < self.cap {
                unsafe { *self.ptr.add(self.len) = b };
            }
            self.len += 1;
        }
        Ok(())
    }
}

unsafe fn to_buf(buf: *mut c_char, size: usize, fmt: *const c_char, ap: VaList) -> c_int {
    let mut w = Buf { ptr: buf as *mut u8, cap: size.saturating_sub(1), len: 0 };
    let n = format(fmt, ap, output::fmt_write(&mut w));
    if size > 0 {
        *buf.add(w.len.min(w.cap)) = 0;
    }
    n
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(fmt: *const c_char, args: ...) -> c_int {
    format(fmt, args, output::fmt_write(&mut Console))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vprintf(fmt: *const c_char, ap: VaList) -> c_int {
    format(fmt, ap, output::fmt_write(&mut Console))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, args: ...) -> c_int {
    to_buf(buf, size, fmt, args)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vsnprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ap: VaList) -> c_int {
    to_buf(buf, size, fmt, ap)
}

// буфер не ограничен как и в обычной libc
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sprintf(buf: *mut c_char, fmt: *const c_char, args: ...) -> c_int {
    to_buf(buf, usize::MAX, fmt, args)
}

#[unsafe(no_mangle)]
pub extern "C" fn putchar(c: c_int) -> c_int {
    crate::print!("{}", c as u8 as char);
    c & 0xff
}

// как в C добавляет '\n'
#[unsafe(no_mangle)]
pub unsafe extern "C" fn puts(s: *const c_char) -> c_int {
    for &b in CStr::from_ptr(s).to_bytes() {
        putchar(b as c_int);
    }
    putchar('\n' as c_int);
    1
}
