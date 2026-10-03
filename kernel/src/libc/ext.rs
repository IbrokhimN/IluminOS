
use core::ffi::{CStr, c_char, c_int};


macro_rules! ctype {
    ($name:ident, |$b:ident| $e:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(c: c_int) -> c_int {
            if !(0..=255).contains(&c) {
                return 0;
            }
            let $b = c as u8;
            ($e) as c_int
        }
    };
}

ctype!(isalnum, |b| b.is_ascii_alphanumeric());
ctype!(islower, |b| b.is_ascii_lowercase());
ctype!(isxdigit, |b| b.is_ascii_hexdigit());
ctype!(ispunct, |b| b.is_ascii_punctuation());
ctype!(iscntrl, |b| b.is_ascii_control());
ctype!(isprint, |b| b.is_ascii_graphic() || b == b' ');

#[unsafe(no_mangle)]
pub extern "C" fn toupper(c: c_int) -> c_int {
    if (0..=255).contains(&c) { (c as u8).to_ascii_uppercase() as c_int } else { c }
}

#[unsafe(no_mangle)]
pub extern "C" fn tolower(c: c_int) -> c_int {
    if (0..=255).contains(&c) { (c as u8).to_ascii_lowercase() as c_int } else { c }
}


#[unsafe(no_mangle)]
pub unsafe extern "C" fn strnlen(s: *const c_char, max: usize) -> usize {
    let mut n = 0;
    while n < max && *s.add(n) != 0 {
        n += 1;
    }
    n
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strcasecmp(a: *const c_char, b: *const c_char) -> c_int {
    let (a, b) = (CStr::from_ptr(a).to_bytes(), CStr::from_ptr(b).to_bytes());
    let mut i = 0;
    loop {
        let x = a.get(i).copied().unwrap_or(0).to_ascii_lowercase();
        let y = b.get(i).copied().unwrap_or(0).to_ascii_lowercase();
        if x != y || x == 0 {
            return x as c_int - y as c_int;
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strdup(s: *const c_char) -> *mut c_char {
    let len = CStr::from_ptr(s).to_bytes().len() + 1;
    let p = tinyrlibc::malloc(len) as *mut c_char;
    if !p.is_null() {
        core::ptr::copy_nonoverlapping(s, p, len);
    }
    p
}

#[unsafe(no_mangle)]
pub extern "C" fn abort() -> ! {
    crate::println!("libc: abort() called");
    panic!("abort() called");
}

#[unsafe(no_mangle)]
pub extern "C" fn exit(code: c_int) -> ! {
    crate::println!("libc: exit({}) called", code);
    panic!("exit() called");
}
