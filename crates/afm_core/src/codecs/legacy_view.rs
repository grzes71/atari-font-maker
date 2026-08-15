//! Legacy binary view importers (`.vf2` / `.vfn`), matching C# `ActionLoadView`.
//!
//! These formats are **import-only** in the original C# application (the save
//! dialog only offers `.atrview` and raw `.dat`). They are lenient parsers:
//! truncated data is zero-filled, mirroring C#'s `try { ReadExactly } catch {}`.

use crate::palette::Palette;

/// Result of parsing a legacy view file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyView {
    /// Color mode: 0 = mono, 1 = Mode 4, 2 = Mode 5, 3 = Mode 10.
    pub color_mode: u8,
    /// Closest-palette color indexes for the first 6 selected-color registers.
    pub colors: [u8; 6],
    /// Per-line font assignments (only the first 8 lines are set by `.vf2`).
    pub line_fonts: Option<[u8; 8]>,
    /// 40x26 view screen, row-major, zero-filled outside the source region.
    pub view: [u8; 40 * 26],
}

/// Map a C# `SetupColorMode` byte to a color mode index.
///
/// C# semantics: `0` = B/W, `2` = Mode 5, `3` = Mode 10, anything else = Mode 4.
fn map_color_mode(c: u8) -> u8 {
    match c {
        0 => 0,
        2 => 2,
        3 => 3,
        _ => 1,
    }
}

/// Parse a `.vf2` legacy view file.
///
/// Layout: `version` (1 byte), `color_mode` (1 byte), 8 x i32 LE `line_fonts`
/// (32 bytes), 6 x RGB (18 bytes), then screen data by version:
/// v2 = 31x8 (248 bytes), v3 = 32x26 (832 bytes), v0/v1 = none.
pub fn parse_vf2(data: &[u8], palette: &Palette) -> Result<LegacyView, String> {
    if data.is_empty() {
        return Err("empty .vf2 file".to_string());
    }

    let version = data.first().copied().unwrap_or(0);
    if version > 3 {
        return Err(format!(
            ".vf2 created by a newer FontMaker version ({version})"
        ));
    }
    let color_byte = data.get(1).copied().unwrap_or(0);

    let mut line_fonts = [1u8; 8];
    for (a, slot) in line_fonts.iter_mut().enumerate() {
        let off = 2 + a * 4;
        if off + 4 <= data.len() {
            let bytes = [data[off], data[off + 1], data[off + 2], data[off + 3]];
            let value = i32::from_le_bytes(bytes);
            *slot = (value as u8).clamp(1, 4);
        }
    }

    let mut colors = [0u8; 6];
    let color_base = 2 + 8 * 4;
    for (a, slot) in colors.iter_mut().enumerate() {
        let off = color_base + a * 3;
        if off + 3 <= data.len() {
            *slot = palette.find_closest(data[off], data[off + 1], data[off + 2]);
        }
    }

    let mut view = [0u8; 40 * 26];
    let screen_base = color_base + 6 * 3;
    match version {
        2 => {
            let (cols, rows) = (31usize, 8usize);
            let count = (cols * rows).min(data.len().saturating_sub(screen_base));
            for i in 0..count {
                let x = i % cols;
                let y = i / cols;
                view[y * 40 + x] = data[screen_base + i];
            }
        }
        3 => {
            let (cols, rows) = (32usize, 26usize);
            let count = (cols * rows).min(data.len().saturating_sub(screen_base));
            for i in 0..count {
                let x = i % cols;
                let y = i / cols;
                view[y * 40 + x] = data[screen_base + i];
            }
        }
        _ => {}
    }

    Ok(LegacyView {
        color_mode: map_color_mode(color_byte),
        colors,
        line_fonts: Some(line_fonts),
        view,
    })
}

/// Parse a `.vfn` legacy view file.
///
/// Layout: `color_mode` (1 byte), 6 x RGB (18 bytes), 31x6 screen data
/// (186 bytes); screen columns 6 and 7 are zeroed (C# `ActionLoadView`).
pub fn parse_vfn(data: &[u8], palette: &Palette) -> Result<LegacyView, String> {
    if data.is_empty() {
        return Err("empty .vfn file".to_string());
    }

    let color_byte = data.first().copied().unwrap_or(0);

    let mut colors = [0u8; 6];
    for (a, slot) in colors.iter_mut().enumerate() {
        let off = 1 + a * 3;
        if off + 3 <= data.len() {
            *slot = palette.find_closest(data[off], data[off + 1], data[off + 2]);
        }
    }

    let mut view = [0u8; 40 * 26];
    let screen_base = 1 + 6 * 3;
    let (cols, rows) = (31usize, 6usize);
    let count = (cols * rows).min(data.len().saturating_sub(screen_base));
    for i in 0..count {
        let x = i % cols;
        let y = i / cols;
        view[y * 40 + x] = data[screen_base + i];
    }
    // Columns 6 and 7 are explicitly zeroed by C#.
    for y in 0..8 {
        view[y * 40 + 6] = 0;
        view[y * 40 + 7] = 0;
    }

    Ok(LegacyView {
        color_mode: map_color_mode(color_byte),
        colors,
        line_fonts: None,
        view,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        Palette::default_altirra()
    }

    #[test]
    fn test_vf2_v3_layout() {
        let mut data = vec![0u8; 2 + 32 + 18 + 832];
        data[0] = 3; // version 3
        data[1] = 0; // mono
        // 8 line fonts: font 2 (offset 2)
        data[2] = 2;
        // screen data at 2+32+18 = 52: set first cell
        data[52] = 0x41;
        let v = parse_vf2(&data, &palette()).expect("parse");
        assert_eq!(v.color_mode, 0);
        assert_eq!(v.line_fonts.unwrap()[0], 2);
        assert_eq!(v.view[0], 0x41);
    }

    #[test]
    fn test_vf2_v2_layout() {
        let mut data = vec![0u8; 2 + 32 + 18 + 248];
        data[0] = 2; // version 2
        data[1] = 0; // mono
        let v = parse_vf2(&data, &palette()).expect("parse");
        assert_eq!(v.color_mode, 0);
        assert_eq!(v.view[7 * 40 + 30], 0);
    }

    #[test]
    fn test_vf2_newer_version_errors() {
        let data = [4u8, 0];
        assert!(parse_vf2(&data, &palette()).is_err());
    }

    #[test]
    fn test_vf2_truncated_is_lenient() {
        let data = [3u8, 1]; // version 3, mode 4, no screen data
        let v = parse_vf2(&data, &palette()).expect("lenient parse");
        assert_eq!(v.color_mode, 1);
        assert!(v.view.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_vfn_layout() {
        let mut data = vec![0u8; 1 + 18 + 186];
        data[0] = 3; // mode 10
        data[1 + 18] = 0x42; // first screen byte
        let v = parse_vfn(&data, &palette()).expect("parse");
        assert_eq!(v.color_mode, 3);
        assert_eq!(v.view[0], 0x42);
        assert_eq!(v.line_fonts, None);
    }
}
