// gui widgets — one file per app
pub mod clock;
pub mod calc;
pub mod paint;
pub mod term;
pub mod browser;
pub mod files;

// re-exports under the old `apps` path, so wm.rs/desktop.rs don't need to
// change how they import these
pub mod apps {
    pub use super::browser::Browser;
    pub use super::calc::Calc;
    pub use super::clock::Clock;
    pub use super::files::Files;
    pub use super::paint::Paint;
    pub use super::term::Term;
}
