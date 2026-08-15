//! Binary font exporter (`.dat`), matching C# `ExportFontWindow.SaveBinaryData`.

use super::types::FontSelection;
use crate::compress::zx0_compress;

/// Export raw (or ZX0-compressed) font bytes for a bank selection.
///
/// Matches C# `GetFontData`: the raw byte range is compressed with ZX0 only if
/// the compressed result is strictly shorter than the original (otherwise the
/// raw data is returned unchanged).
pub fn export_font_binary(font_bytes: &[u8], selection: FontSelection, compress: bool) -> Vec<u8> {
    let (start_byte, end_byte) = selection.byte_range();
    let start = start_byte.min(font_bytes.len());
    let end = end_byte.min(font_bytes.len());
    let raw = &font_bytes[start..end];

    if compress {
        let compressed = zx0_compress(raw);
        if !compressed.is_empty() && compressed.len() < raw.len() {
            return compressed;
        }
    }

    raw.to_vec()
}
