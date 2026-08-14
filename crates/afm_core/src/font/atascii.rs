//! ASCII/ATASCII string to Atari Screen Codes and Clipboard conversion.

use crate::codecs::clipboard::ClipboardJson;
use crate::font::bank::FontBankSet;
use crate::font::glyph::convert_atari_char;

/// Convert an ASCII string into Atari Screen Codes (with optional inverse flag).
pub fn text_to_atari_screen_codes(text: &str, inverse: bool) -> Vec<u8> {
    text.bytes()
        .map(|b| {
            let mut ch = convert_atari_char(b);
            if inverse {
                ch |= 128;
            }
            ch
        })
        .collect()
}

/// Render a text string into a ClipboardJson object containing screen codes and font glyph bytes.
pub fn render_text_to_clipboard(
    text: &str,
    inverse: bool,
    bank_index: usize,
    fonts: &FontBankSet,
) -> ClipboardJson {
    let screen_codes = text_to_atari_screen_codes(text, inverse);
    let bank_offset = (bank_index % 4) * 1024;

    let mut char_bytes_hex = String::with_capacity(text.len() * 2);
    let mut font_bytes_hex = String::with_capacity(text.len() * 16);
    let nulls = "0".repeat(text.len());

    let font_bytes = fonts.as_bytes();

    for &code in &screen_codes {
        char_bytes_hex.push_str(&format!("{code:02X}"));

        let char_in_font = (code as usize & 127) * 8 + bank_offset;
        for k in 0..8 {
            let byte_val = font_bytes.get(char_in_font + k).copied().unwrap_or(0);
            font_bytes_hex.push_str(&format!("{byte_val:02X}"));
        }
    }

    ClipboardJson {
        width: Some(text.len().to_string()),
        height: Some("1".to_string()),
        chars: Some(char_bytes_hex),
        data: Some(font_bytes_hex),
        font_nr: Some(((bank_index % 4) + 1).to_string()),
        nulls: Some(nulls),
    }
}
