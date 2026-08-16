//! Minimal `rfd` diagnostic probe — independent of the rest of the app.
//!
//! Exercises the exact same `rfd` build (same crate version and Cargo features
//! as `afm_gui`) so the results reflect the application's runtime environment.
//!
//! Usage:
//!   cargo run -p afm_gui --example rfd_probe -- [open|save|folder|all]
//!
//! Each mode prints:
//!   * environment diagnostics (display, session, portal service, zenity),
//!   * a "calling ..." line before the blocking `rfd` call,
//!   * a result line after the call (selected path or None + elapsed time),
//!   * any `rfd` internal `log::error!` output (portal / zenity failures).

use std::path::PathBuf;
use std::process::Command;

struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        eprintln!("[rfd-probe] [{}] {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

fn env_var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| "<unset>".to_string())
}

fn has_binary(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn portal_on_bus() -> bool {
    Command::new("timeout")
        .args([
            "5",
            "gdbus",
            "call",
            "--session",
            "--dest",
            "org.freedesktop.DBus",
            "--object-path",
            "/org/freedesktop/DBus",
            "--method",
            "org.freedesktop.DBus.ListNames",
        ])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("org.freedesktop.portal.Desktop"))
        .unwrap_or(false)
}

fn print_environment() {
    eprintln!("=== environment ===");
    eprintln!("DISPLAY              = {}", env_var("DISPLAY"));
    eprintln!("WAYLAND_DISPLAY      = {}", env_var("WAYLAND_DISPLAY"));
    eprintln!("XDG_CURRENT_DESKTOP  = {}", env_var("XDG_CURRENT_DESKTOP"));
    eprintln!("XDG_SESSION_TYPE     = {}", env_var("XDG_SESSION_TYPE"));
    eprintln!("XDG_RUNTIME_DIR      = {}", env_var("XDG_RUNTIME_DIR"));
    eprintln!(
        "DBUS_SESSION_BUS     = {}",
        env_var("DBUS_SESSION_BUS_ADDRESS")
    );
    eprintln!(
        "xdg-desktop-portal   = {}",
        has_binary("xdg-desktop-portal")
    );
    eprintln!(
        "xdg-desktop-portal-gtk = {}",
        has_binary("xdg-desktop-portal-gtk")
    );
    eprintln!("zenity               = {}", has_binary("zenity"));
    eprintln!(
        "org.freedesktop.portal.Desktop on bus = {}",
        portal_on_bus()
    );
    eprintln!("=====================");
}

fn probe(name: &str, f: impl FnOnce() -> Option<PathBuf>) {
    eprintln!("[probe] {name}: invoking rfd ...");
    let started = std::time::Instant::now();
    let result = f();
    let elapsed = started.elapsed();
    match &result {
        Some(path) => eprintln!("[probe] {name}: SELECTED {path:?} (elapsed {elapsed:?})"),
        None => {
            eprintln!("[probe] {name}: None (cancelled OR backend error) (elapsed {elapsed:?})")
        }
    }
}

fn main() {
    let _ = log::set_boxed_logger(Box::new(StderrLogger));
    log::set_max_level(log::LevelFilter::Info);

    print_environment();

    let mode = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());

    if mode == "open" || mode == "all" {
        probe("pick_file", || {
            rfd::FileDialog::new()
                .add_filter("All files", &["*"])
                .pick_file()
        });
    }
    if mode == "save" || mode == "all" {
        probe("save_file", || {
            rfd::FileDialog::new()
                .set_file_name("probe.txt")
                .save_file()
        });
    }
    if mode == "folder" || mode == "all" {
        probe("pick_folder", || rfd::FileDialog::new().pick_folder());
    }
}
