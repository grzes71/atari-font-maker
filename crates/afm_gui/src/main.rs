//! Atari FontMaker - Rust + Slint GUI Application Shell.

slint::include_modules!();

pub mod app;
pub mod controller;
pub mod state;

use app::AfmApp;

fn main() -> Result<(), slint::PlatformError> {
    let app = AfmApp::new()?;
    app.run()
}
