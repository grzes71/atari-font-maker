//! Atari FontMaker - Rust + Slint GUI Application Binary.

use afm_gui::AfmApp;

fn main() -> Result<(), slint::PlatformError> {
    let app = AfmApp::new()?;
    app.run()
}
