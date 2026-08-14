//! Atari FontMaker Core - Headless domain library.

pub mod constants;
pub mod font;

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
