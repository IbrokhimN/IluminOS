#![allow(unsafe_op_in_unsafe_fn)]

pub mod ext;
#[cfg(feature = "c-math")]
pub mod math;
pub mod selftest;
pub mod stdio;
