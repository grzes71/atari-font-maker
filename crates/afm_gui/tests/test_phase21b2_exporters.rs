//! Phase 21B-2 exporter regression tests — BMP Mono / BMP Color / Binary View.
//!
//! These tests exercise the real state → afm_core → bytes chain (the same chain
//! reached by the Slint modal → controller → state flow). The controller wiring
//! (dialog, clipboard, file write) is covered by the unit tests in
//! `crates/afm_gui/src/controller.rs`.

#[path = "../src/io.rs"]
mod io;

#[path = "../src/state.rs"]
mod state;

use afm_core::exporters::{FontSelection, ViewExportRegion};
use state::GuiState;

/// Decode the key fields of a 24-bit BMP: `(width, height, bits_per_pixel)`.
fn bmp_dimensions(bytes: &[u8]) -> (u32, i32, u16) {
    assert_eq!(&bytes[0..2], b"BM", "missing BMP magic");
    let width = u32::from_le_bytes(bytes[18..22].try_into().unwrap());
    let height = i32::from_le_bytes(bytes[22..26].try_into().unwrap());
    let bpp = u16::from_le_bytes(bytes[28..30].try_into().unwrap());
    (width, height, bpp)
}

/// Read a 24-bit BMP pixel at top-down `(x, y)` as `(B, G, R)`.
fn bmp_pixel(bytes: &[u8], width: usize, height: i32, x: usize, y: usize) -> (u8, u8, u8) {
    let stride = (width * 3).div_ceil(4) * 4;
    let row = (height as usize - 1 - y) * stride;
    let off = 54 + row + x * 3;
    (bytes[off], bytes[off + 1], bytes[off + 2])
}

fn mono_colors(s: &GuiState) -> ((u8, u8, u8), (u8, u8, u8)) {
    let fg = s.renderer.cached_colors()[0];
    let bg = s.renderer.cached_colors()[1];
    ((fg[0], fg[1], fg[2]), (bg[0], bg[1], bg[2]))
}

// ==========================================
// BMP Mono
// ==========================================

#[test]
fn test_bmp_mono_empty_font_stacks_normal_and_inverse() {
    let mut s = GuiState::new();
    s.fonts.as_bytes_mut().fill(0); // isolate the scenario from the default font
    let (fg, bg) = mono_colors(&s);

    let bytes = s.export_font_bmp_bytes(FontSelection::Font1, false);
    assert_eq!(bmp_dimensions(&bytes), (256, 64, 24));

    // Single-font BMP covers 128 atlas rows (normal 0..63 + inverse 64..127),
    // sampled every 2nd row: top half = normal (background), bottom = inverse (foreground).
    for y in 0..32 {
        for x in 0..256 {
            assert_eq!(
                bmp_pixel(&bytes, 256, 64, x, y),
                bg,
                "normal half at ({x},{y})"
            );
        }
    }
    for y in 32..64 {
        for x in 0..256 {
            assert_eq!(
                bmp_pixel(&bytes, 256, 64, x, y),
                fg,
                "inverse half at ({x},{y})"
            );
        }
    }
}

#[test]
fn test_bmp_mono_single_pixel() {
    let mut s = GuiState::new();
    s.fonts.as_bytes_mut().fill(0); // isolate the scenario from the default font
    let (fg, bg) = mono_colors(&s);
    s.fonts.as_bytes_mut()[0] = 0x80; // char 0, top-left pixel

    let bytes = s.export_font_bmp_bytes(FontSelection::Font1, false);
    assert_eq!(bmp_dimensions(&bytes), (256, 64, 24));

    // Normal half: exactly one foreground pixel, at (0, 0).
    let mut top_fg = 0;
    for y in 0..32 {
        for x in 0..256 {
            if bmp_pixel(&bytes, 256, 64, x, y) == fg {
                top_fg += 1;
            }
        }
    }
    assert_eq!(top_fg, 1);
    assert_eq!(bmp_pixel(&bytes, 256, 64, 0, 0), fg);

    // Inverse half: exactly one background pixel, at (0, 0); rest foreground.
    let mut bottom_bg = 0;
    for y in 32..64 {
        for x in 0..256 {
            if bmp_pixel(&bytes, 256, 64, x, y) == bg {
                bottom_bg += 1;
            }
        }
    }
    assert_eq!(bottom_bg, 1);
    assert_eq!(bmp_pixel(&bytes, 256, 64, 0, 32), bg);
}

#[test]
fn test_bmp_mono_full_char() {
    let mut s = GuiState::new();
    s.fonts.as_bytes_mut().fill(0); // isolate the scenario from the default font
    let (fg, bg) = mono_colors(&s);
    for i in 0..8 {
        s.fonts.as_bytes_mut()[i] = 0xFF; // char 0 = 8x8 solid block
    }

    let bytes = s.export_font_bmp_bytes(FontSelection::Font1, false);

    // Normal half: 8x8 foreground block at top-left, rest background.
    for y in 0..32 {
        for x in 0..256 {
            let expect = if x < 8 && y < 8 { fg } else { bg };
            assert_eq!(
                bmp_pixel(&bytes, 256, 64, x, y),
                expect,
                "normal at ({x},{y})"
            );
        }
    }
    // Inverse half: 8x8 background block (at rows 32..39), rest foreground.
    for y in 32..64 {
        for x in 0..256 {
            let expect = if x < 8 && y < 40 { bg } else { fg };
            assert_eq!(
                bmp_pixel(&bytes, 256, 64, x, y),
                expect,
                "inverse at ({x},{y})"
            );
        }
    }
}

#[test]
fn test_bmp_mono_font_selection_dimensions() {
    let mut s = GuiState::new();
    let cases = [
        (FontSelection::Font1, 64),
        (FontSelection::Font2, 64),
        (FontSelection::Font3, 64),
        (FontSelection::Font4, 64),
        (FontSelection::Font1_2, 128),
        (FontSelection::Font3_4, 128),
        (FontSelection::FontAll, 256),
    ];
    for (sel, expected_height) in cases {
        let bytes = s.export_font_bmp_bytes(sel, false);
        assert_eq!(
            bmp_dimensions(&bytes),
            (256, expected_height, 24),
            "wrong dimensions for {sel:?}"
        );
    }
}

// ==========================================
// BMP Color
// ==========================================

fn solid_char(s: &mut GuiState) {
    for i in 0..8 {
        s.fonts.as_bytes_mut()[i] = 0xFF;
    }
}

#[test]
fn test_bmp_color_modes_produce_valid_bmp() {
    let mut s = GuiState::new();
    solid_char(&mut s);

    s.active_color_mode = 1; // Mode 4
    let mode4 = s.export_font_bmp_bytes(FontSelection::Font1, true);

    s.active_color_mode = 2; // Mode 5
    let mode5 = s.export_font_bmp_bytes(FontSelection::Font1, true);

    s.active_color_mode = 3; // Mode 10
    let mode10 = s.export_font_bmp_bytes(FontSelection::Font1, true);

    assert_eq!(bmp_dimensions(&mode4), (256, 64, 24));
    assert_eq!(bmp_dimensions(&mode5), (256, 64, 24));
    assert_eq!(bmp_dimensions(&mode10), (256, 64, 24));

    // Mode 4 and Mode 5 share the same 2-bit color rendering (as in C#).
    assert_eq!(mode4, mode5);
    // Mode 10 uses a different (4-bit) rasterization geometry.
    assert_ne!(mode4, mode10);
}

#[test]
fn test_bmp_color_register_change_changes_output() {
    let mut s = GuiState::new();
    // 0x55 => every 2-bit pixel is 1 => PF0 (color register index 2) in Mode 4.
    for i in 0..8 {
        s.fonts.as_bytes_mut()[i] = 0x55;
    }
    s.active_color_mode = 1; // Mode 4

    let before = s.export_font_bmp_bytes(FontSelection::Font1, true);

    let old = s.project.colors[2]; // PF0
    s.project.colors[2] = 0x02;
    s.renderer.set_color_registers(s.project.colors);

    let after = s.export_font_bmp_bytes(FontSelection::Font1, true);
    assert_ne!(
        before, after,
        "changing a color register must change the raster"
    );

    s.project.colors[2] = old;
    s.renderer.set_color_registers(s.project.colors);
}

// ==========================================
// Binary View
// ==========================================

#[test]
fn test_binary_view_empty_screen() {
    let s = GuiState::new();
    let bytes = s.export_view_binary_bytes(ViewExportRegion::full_standard(), false);
    assert_eq!(bytes.len(), 40 * 26);
    assert!(bytes.iter().all(|&b| b == 0));
}

#[test]
fn test_binary_view_various_chars_and_transpose() {
    let mut s = GuiState::new();
    for i in 0..(40 * 26) {
        s.project.view_bytes[i] = (i % 128) as u8;
    }

    let row = s.export_view_binary_bytes(ViewExportRegion::full_standard(), false);
    assert_eq!(&row[..], &s.project.view_bytes[..]);

    let col = s.export_view_binary_bytes(ViewExportRegion::full_standard(), true);
    for x in 0..40 {
        for y in 0..26 {
            assert_eq!(col[x * 26 + y], s.project.view_bytes[y * 40 + x]);
        }
    }
}

#[test]
fn test_binary_view_exports_active_page() {
    let mut s = GuiState::new();
    for i in 0..(40 * 26) {
        s.project.view_bytes[i] = 0x11; // page 1 pattern
    }

    s.add_new_page("Page 2"); // switches to a blank page 2
    let page2 = s.export_view_binary_bytes(ViewExportRegion::full_standard(), false);
    assert!(page2.iter().all(|&b| b == 0));

    s.switch_to_page(0); // back to page 1
    let page1 = s.export_view_binary_bytes(ViewExportRegion::full_standard(), false);
    assert!(page1.iter().all(|&b| b == 0x11));
}

#[test]
fn test_binary_view_line_fonts_do_not_affect_bytes() {
    let mut s = GuiState::new();
    for i in 0..(40 * 26) {
        s.project.view_bytes[i] = (i % 128) as u8;
    }

    let before = s.export_view_binary_bytes(ViewExportRegion::full_standard(), false);
    for f in s.project.line_fonts.iter_mut() {
        *f = 4;
    }
    let after = s.export_view_binary_bytes(ViewExportRegion::full_standard(), false);

    // Binary View exports raw screen codes; line fonts only affect rendering.
    assert_eq!(before, after);
}
