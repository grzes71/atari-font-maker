//! Atari FontMaker Core - Headless domain library.

pub mod analysis;
pub mod codecs;
pub mod constants;
pub mod error;
pub mod exporters;
pub mod font;
pub mod palette;
pub mod renderer;
pub mod tileset;
pub mod undo;
pub mod view;

pub fn core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_headless() {
        assert_eq!(core_version(), "0.1.0");
    }
}
