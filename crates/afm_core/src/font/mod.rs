//! Font data model, character encodings, glyph transformations, and bank operations.

pub mod area_transforms;
pub mod atascii;
pub mod bank;
pub mod glyph;
pub mod transforms;

pub use area_transforms::PixelMatrix;
pub use atascii::{render_text_to_clipboard, text_to_atari_screen_codes};
pub use bank::FontBankSet;
pub use glyph::{GlyphBytes, convert_atari_char};
