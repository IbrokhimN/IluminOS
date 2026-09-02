// графическая подсистема
pub mod framebuffer;
pub mod desktop;
pub mod html;
pub mod widgets;

// crate::gui::run() -> desktop::run
pub use desktop::run;
