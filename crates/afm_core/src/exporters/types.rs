//! Common types and options for code and data exporters.

/// Target programming language or format for text exporters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatType {
    Assembler = 0,
    Action = 1,
    AtariBasic = 2,
    FastBasic = 3,
    MADSdta = 4,
    CDataArray = 5,
    MadPascalArray = 6,
}

/// Numerical representation of exported bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataType {
    #[default]
    Decimal = 0,
    Hexadecimal = 1,
}

/// Font bank selection range for font exporters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontSelection {
    #[default]
    Font1 = 0,
    Font2 = 1,
    Font3 = 2,
    Font4 = 3,
    Font1_2 = 4,
    Font3_4 = 5,
    FontAll = 6,
}

impl FontSelection {
    /// Calculate starting and ending byte offsets within the 4096-byte font bank buffer.
    pub fn byte_range(self) -> (usize, usize) {
        match self {
            Self::Font1 => (0, 1024),
            Self::Font2 => (1024, 2048),
            Self::Font3 => (2048, 3072),
            Self::Font4 => (3072, 4096),
            Self::Font1_2 => (0, 2048),
            Self::Font3_4 => (2048, 4096),
            Self::FontAll => (0, 4096),
        }
    }
}

/// Rectangular export region within the Atari View grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewExportRegion {
    pub rx: usize,
    pub ry: usize,
    pub rw: usize,
    pub rh: usize,
}

impl ViewExportRegion {
    pub const fn new(rx: usize, ry: usize, rw: usize, rh: usize) -> Self {
        Self { rx, ry, rw, rh }
    }

    pub const fn full_standard() -> Self {
        Self {
            rx: 0,
            ry: 0,
            rw: 40,
            rh: 26,
        }
    }
}
