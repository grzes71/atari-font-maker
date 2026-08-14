//! Strongly-typed domain and format errors for afm_core.

use thiserror::Error;

/// Root error type for afm_core operations.
#[derive(Error, Debug)]
pub enum AfmError {
    #[error("Font format error: {0}")]
    FontFormat(#[from] FontFormatError),

    #[error("Palette format error: {0}")]
    PaletteFormat(#[from] PaletteFormatError),

    #[error("AtrView project format error: {0}")]
    AtrViewFormat(#[from] AtrViewFormatError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to loading, parsing, or saving font formats (.fnt, .fn2).
#[derive(Error, Debug)]
pub enum FontFormatError {
    #[error("Invalid font file size: expected {expected} bytes, got {actual} bytes")]
    InvalidSize { expected: usize, actual: usize },

    #[error("Invalid font bank index: {0} (must be in range 0..4)")]
    InvalidBankIndex(usize),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to loading, parsing, or saving palette formats (.pal).
#[derive(Error, Debug)]
pub enum PaletteFormatError {
    #[error("Invalid palette file size: expected {expected} bytes, got {actual} bytes")]
    InvalidSize { expected: usize, actual: usize },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to loading, parsing, or saving .atrview project files.
#[derive(Error, Debug)]
pub enum AtrViewFormatError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Hex decoding error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid or unsupported project data: {0}")]
    InvalidData(String),
}
