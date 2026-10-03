use core::ffi::{c_char, c_int, c_long, c_void};
use core::ptr::{null, null_mut};

use tinyrlibc as t;

use super::{ext, stdio};
use crate::framebuffer::{GREEN, RED, YELLOW};
use crate::{print_color, println};

macro_rules! c {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

macro_rules! u {
    ($s:literal) => {
        concat!($s, "\0").as_ptr()
    };
}

struct Tally {
    pass: u32,
    fail: u32,
}

impl Tally {
    fn check(&mut self, name: &str, ok: bool) {
        if ok {
            self.pass += 1;
        } else {
            self.fail += 1;
            print_color!(RED, "[FAIL]");
            println!(" {}", name);
        }
    }
}

// содержимое C-буфера= ожидаемая строка
unsafe fn eq<T>(buf: *const T, want: &str) -> bool {
    let buf = buf as *const u8;
    core::slice::from_raw_parts(buf, t::strlen(buf)) == want.as_bytes()
}

extern "C" fn cmp_i32(a: *const c_void, b: *const c_void) -> c_int {
    let (x, y) = unsafe { (*(a as *const i32), *(b as *const i32)) };
    (x > y) as c_int - (x < y) as c_int
}

pub fn run() {
    print_color!(YELLOW, "libc self-test\n");
    let mut r = Tally { pass: 0, fail: 0 };
    unsafe {
        strings(&mut r);
        numbers(&mut r);
        memory(&mut r);
        printf(&mut r);
    }
    math(&mut r);
    if r.fail == 0 {
        print_color!(GREEN, "[ok]");
        println!(" libc: all {} checks passed", r.pass);
    } else {
        print_color!(RED, "[FAIL]");
        println!(" libc: {} passed, {} failed", r.pass, r.fail);
    }
}

unsafe fn strings(r: &mut Tally) {
    r.check("strlen", t::strlen(u!("hello")) == 5 && t::strlen(u!("")) == 0);
    r.check("strcmp", t::strcmp(u!("abc"), u!("abc")) == 0);
    r.check("strcmp <", t::strcmp(u!("abc"), u!("abd")) < 0);
    r.check("strcmp prefix", t::strcmp(u!("ab"), u!("abc")) < 0);
    r.check("strncmp", t::strncmp(u!("abcX"), u!("abcY"), 3) == 0);
    r.check("strncasecmp", t::strncasecmp(u!("HeLLo"), u!("hello"), 5) == 0);

    let mut buf = [0u8; 32];
    let b = buf.as_mut_ptr();
    t::strcpy(b, u!("foo"));
    t::strcat(b, u!("bar"));
    r.check("strcpy+strcat", eq(b, "foobar"));

    let mut small = [1u8; 8];
    t::strncpy(small.as_mut_ptr(), u!("ab"), 8);
    r.check("strncpy pads zeros", small[2] == 0 && small[7] == 0);

    let s = u!("a/b/c");
    r.check("strchr", t::strchr(s, '/' as c_int) == s.add(1));
    r.check("strchr missing", t::strchr(s, 'z' as c_int).is_null());
    r.check("strrchr", t::strrchr(s, '/' as c_int) == s.add(3));
    let h = u!("find the needle here");
    r.check("strstr", t::strstr(h, u!("needle")) == h.add(9));
    r.check("strstr missing", t::strstr(h, u!("haystack")).is_null());
    r.check("strstr tail", t::strstr(u!("abc"), u!("abcd")).is_null());
    r.check("strspn", t::strspn(u!("123abc"), u!("0123456789")) == 3);
    r.check("strcspn", t::strcspn(u!("abc,def"), u!(",;")) == 3);
    let hay = u!("abcdef");
    r.check("memchr", t::memchr(hay as *const c_void, 'd' as c_int, 6) == hay.add(3) as *const c_void);
    r.check("memchr miss", t::memchr(hay as *const c_void, 'z' as c_int, 6).is_null());


    r.check("strnlen", ext::strnlen(c!("hello"), 3) == 3 && ext::strnlen(c!("hi"), 9) == 2);
    r.check("strcasecmp", ext::strcasecmp(c!("HeLLo"), c!("hello")) == 0 && ext::strcasecmp(c!("a"), c!("b")) < 0);
    let d = ext::strdup(c!("duplicate"));
    r.check("strdup", !d.is_null() && eq(d, "duplicate"));
    t::free(d as *mut u8);

    r.check("isalpha/isdigit", t::isalpha('a' as c_int) != 0 && t::isdigit('7' as c_int) != 0 && t::isdigit('x' as c_int) == 0);
    r.check("isspace", t::isspace(' ' as c_int) != 0 && t::isspace('\n' as c_int) != 0 && t::isspace('a' as c_int) == 0);
    r.check("isalnum", ext::isalnum('z' as c_int) != 0 && ext::isalnum('_' as c_int) == 0);
    r.check("isxdigit", ext::isxdigit('F' as c_int) != 0 && ext::isxdigit('G' as c_int) == 0);
    r.check("ispunct", ext::ispunct('!' as c_int) != 0 && ext::ispunct('a' as c_int) == 0);
    r.check("toupper/tolower", ext::toupper('q' as c_int) == 'Q' as c_int && ext::tolower('Q' as c_int) == 'q' as c_int);
    r.check("ctype EOF", ext::isalnum(-1) == 0 && ext::toupper(-1) == -1);
}

unsafe fn numbers(r: &mut Tally) {
    r.check("atoi", t::atoi(u!("  -42xyz")) == -42);
    r.check("strtol hex", t::strtol(u!("0x1F"), null_mut(), 16) == 31);
    r.check("strtol auto base", t::strtol(u!("0x10"), null_mut(), 0) == 16 && t::strtol(u!("010"), null_mut(), 0) == 8);
    r.check("strtol base 2", t::strtol(u!("1011"), null_mut(), 2) == 11);

    let mut end: *const u8 = null();
    let src = u!("123abc");
    let v = t::strtol(src, &mut end, 10);
    r.check("strtol endptr", v == 123 && end == src.add(3));
    r.check("strtol overflow clamps", t::strtol(u!("99999999999999999999"), null_mut(), 10) == c_long::MAX);
    r.check("strtoul", t::strtoul(u!("4000000000"), null_mut(), 10) == 4_000_000_000);
    r.check("abs", t::abs(-5) == 5 && t::abs(5) == 5);

    let mut arr = [5i32, -3, 9, 0, 9, 2, -8, 7, 1, 4, 6, -1, 3, 8, -2, 11, 10, -9, 12];
    t::qsort(arr.as_mut_ptr() as *mut c_void, arr.len(), 4, Some(cmp_i32));
    r.check("qsort", arr.windows(2).all(|w| w[0] <= w[1]));

    t::srand(1);
    let a = t::rand();
    t::srand(1);
    r.check("rand deterministic", a == t::rand());
}

unsafe fn memory(r: &mut Tally) {
    let p = t::malloc(16);
    r.check("malloc aligned", !p.is_null() && (p as usize) % 16 == 0);
    for i in 0..16 {
        *p.add(i) = i as u8;
    }
    let q = t::realloc(p, 4096);
    r.check("realloc grow keeps data", !q.is_null() && (0..16).all(|i| *q.add(i) == i as u8));
    let s = t::realloc(q, 8);
    r.check("realloc shrink keeps data", !s.is_null() && (0..8).all(|i| *s.add(i) == i as u8));
    t::free(s);

    let z = t::calloc(64, 4);
    r.check("calloc zeroes", !z.is_null() && (0..256).all(|i| *z.add(i) == 0));
    t::free(z);
    t::free(null_mut());
}

unsafe fn printf(r: &mut Tally) {
    let mut buf = [0 as c_char; 128];
    let b = buf.as_mut_ptr();
    let n = buf.len();

    macro_rules! fmt {
        ($want:expr, $($arg:expr),+) => {{
            stdio::snprintf(b, n, $($arg),+);
            eq(b, $want)
        }};
    }

    r.check("%d", fmt!("42|-7", c!("%d|%d"), 42 as c_int, -7 as c_int));
    r.check("%5d %-5d", fmt!("   42|42   |", c!("%5d|%-5d|"), 42 as c_int, 42 as c_int));
    r.check("%05d", fmt!("00042|-0042", c!("%05d|%05d"), 42 as c_int, -42 as c_int));
    r.check("%+d", fmt!("+5", c!("%+d"), 5 as c_int));
    r.check("%u", fmt!("4294967295", c!("%u"), -1 as c_int));
    r.check("%x %X", fmt!("ff FF", c!("%x %X"), 255 as c_int, 255 as c_int));
    r.check("%#x", fmt!("0xff", c!("%#x"), 255 as c_int));
    r.check("%ld", fmt!("1234567890123", c!("%ld"), 1234567890123 as c_long));
    r.check("%zu", fmt!("99", c!("%zu"), 99usize));
    r.check("%lld min", fmt!("-9223372036854775808", c!("%lld"), i64::MIN));
    r.check("%s", fmt!("hi|   hi|hi   |", c!("%s|%5s|%-5s|"), c!("hi"), c!("hi"), c!("hi")));
    r.check("%.2s", fmt!("he", c!("%.2s"), c!("hello")));
    r.check("%s NULL", fmt!("(null)", c!("%s"), null::<c_char>()));
    r.check("%c", fmt!("A|  B", c!("%c|%3c"), 'A' as c_int, 'B' as c_int));
    r.check("%%", fmt!("100%", c!("100%%")));
    r.check("%*d", fmt!("   42", c!("%*d"), 5 as c_int, 42 as c_int));
    r.check("%f", fmt!("3.141590", c!("%f"), 3.14159f64));
    r.check("%.2f", fmt!("3.14", c!("%.2f"), 3.14159f64));
    r.check("%8.3f", fmt!("   1.500", c!("%8.3f"), 1.5f64));

    let mut tiny = [0x55 as c_char; 4];
    let full = stdio::snprintf(tiny.as_mut_ptr(), 4, c!("%s"), c!("abcdefgh"));
    r.check("snprintf truncates", full == 8 && eq(tiny.as_ptr(), "abc"));
    let mut untouched = [0x55 as c_char; 2];
    let full = stdio::snprintf(untouched.as_mut_ptr(), 0, c!("%d"), 12345 as c_int);
    r.check("snprintf size 0", full == 5 && untouched[0] == 0x55);
    r.check("snprintf(NULL,0)", stdio::snprintf(null_mut(), 0, c!("%d:%s"), 123 as c_int, c!("xy")) == 6);

    let mut sbuf = [0 as c_char; 32];
    let n = stdio::sprintf(sbuf.as_mut_ptr(), c!("%s-%d"), c!("id"), 9 as c_int);
    r.check("sprintf", n == 4 && eq(sbuf.as_ptr(), "id-9"));

    let printed = stdio::printf(c!("libc printf works: %d %s\n"), 7 as c_int, c!("ok"));
    r.check("printf return", printed == 24);
    stdio::puts(c!("libc puts works"));
}

fn math(r: &mut Tally) {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    r.check("libm sqrt", near(libm::sqrt(2.0), core::f64::consts::SQRT_2));
    r.check("libm sin/cos", near(libm::sin(0.0), 0.0) && near(libm::cos(0.0), 1.0));
    r.check("libm pow", near(libm::pow(2.0, 10.0), 1024.0));
    r.check("libm exp/log", near(libm::log(libm::exp(1.5)), 1.5));
    r.check("libm floor/ceil", libm::floor(-1.5) == -2.0 && libm::ceil(1.2) == 2.0);
    r.check("libm atan2", near(libm::atan2(1.0, 1.0), core::f64::consts::FRAC_PI_4));
    #[cfg(feature = "c-math")]
    r.check("C sqrt export", near(super::math::sqrt(9.0), 3.0));
}
