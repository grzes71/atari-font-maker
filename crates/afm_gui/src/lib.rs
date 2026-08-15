//! Atari FontMaker - Rust + Slint GUI Library.

slint::include_modules!();

pub mod app;
pub mod controller;
pub mod io;
pub mod state;

pub use app::AfmApp;
pub use controller::GuiController;
pub use state::{GuiState, PendingAction};
