// gui subsystem
pub mod framebuffer;
pub mod desktop;
pub mod html;
pub mod widgets;
pub mod wm;

// crate::gui::run points to desktop::run
pub use desktop::run;
