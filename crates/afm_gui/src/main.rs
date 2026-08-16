//! Atari FontMaker - Rust + Slint GUI Application Binary.

use afm_gui::AfmApp;

/// TEMPORARY diagnostic logger: prints every `log` record to stderr so that
/// `rfd`'s internal `log::error!` lines (portal failure / zenity fallback
/// failure) are visible when diagnosing native file dialogs.
fn init_diagnostics_logger() {
    struct StderrLogger;

    impl log::Log for StderrLogger {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            eprintln!("[rfd-diag] [{}] {}", record.level(), record.args());
        }

        fn flush(&self) {}
    }

    let _ = log::set_boxed_logger(Box::new(StderrLogger));
    log::set_max_level(log::LevelFilter::Info);
}

fn main() -> Result<(), slint::PlatformError> {
    init_diagnostics_logger();
    let app = AfmApp::new()?;
    app.run()
}
