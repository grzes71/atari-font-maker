//! RGB color structure representing a single palette color.

/// Single RGB color entry (8-bit per channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColorRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ColorRgb {
    /// Create a new RGB color.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert to a 3-byte array `[r, g, b]`.
    pub const fn to_rgb_bytes(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }

    /// Create from a 3-byte slice `[r, g, b]`.
    pub const fn from_rgb_bytes(bytes: [u8; 3]) -> Self {
        Self {
            r: bytes[0],
            g: bytes[1],
            b: bytes[2],
        }
    }
}
