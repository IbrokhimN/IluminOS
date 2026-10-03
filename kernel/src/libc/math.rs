macro_rules! f64_1 {
    ($($name:ident),* $(,)?) => {$(
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(x: f64) -> f64 { libm::$name(x) }
    )*};
}

macro_rules! f64_2 {
    ($($name:ident),* $(,)?) => {$(
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(x: f64, y: f64) -> f64 { libm::$name(x, y) }
    )*};
}

macro_rules! f32_1 {
    ($($name:ident),* $(,)?) => {$(
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(x: f32) -> f32 { libm::$name(x) }
    )*};
}

macro_rules! f32_2 {
    ($($name:ident),* $(,)?) => {$(
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(x: f32, y: f32) -> f32 { libm::$name(x, y) }
    )*};
}

f64_1!(
    sin, cos, tan, asin, acos, atan, sinh, cosh, tanh,
    exp, exp2, log, log2, log10, sqrt, cbrt,
    floor, ceil, trunc, round, fabs,
);
f64_2!(atan2, pow, fmod, hypot, fmin, fmax);

f32_1!(sinf, cosf, tanf, expf, logf, sqrtf, floorf, ceilf, fabsf);
f32_2!(powf, atan2f, fmodf);
