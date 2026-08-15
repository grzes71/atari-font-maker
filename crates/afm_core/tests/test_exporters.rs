use std::fs;
use std::path::Path;

use afm_core::exporters::{
    DataType, FontSelection, FormatType, ViewExportRegion, export_font_as_text, export_font_bmp,
    export_font_lst, export_view_as_text, export_view_binary,
};
use afm_core::font::bank::FontBankSet;
use afm_core::palette::Palette;
use afm_core::renderer::{FontAtlasBuffer, FontRenderer, RenderColorMode};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

fn read_fixture_bytes(relative: &str) -> Vec<u8> {
    fs::read(fixture_path(relative)).expect("Read fixture bytes")
}

fn read_fixture_str(relative: &str) -> String {
    let raw = fs::read_to_string(fixture_path(relative)).expect("Read fixture string");
    raw.trim_start_matches('\u{feff}').to_string()
}

fn create_standard_font_setup() -> FontBankSet {
    let default_fnt = read_fixture_bytes("projects/Default.fnt");
    let mut fonts = FontBankSet::new();
    for bank_idx in 0..4 {
        fonts.copy_to(&default_fnt, 0, bank_idx * 1024, 1024);
    }
    fonts
}

fn create_standard_renderer() -> (FontBankSet, FontRenderer) {
    let fonts = create_standard_font_setup();
    let pal_bytes = read_fixture_bytes("palette/altirraPAL.pal");
    let mut pal_arr = [0u8; 768];
    pal_arr.copy_from_slice(&pal_bytes);
    let palette = Palette::from_bytes(&pal_arr);

    let selected_colors = [0x00, 0x28, 0xCA, 0x46, 0x98, 0x1A, 0x76, 0x54, 0x32, 0x00];
    let renderer = FontRenderer::new(palette, selected_colors);
    (fonts, renderer)
}

fn create_sample_view_grid() -> (Vec<u8>, usize, usize) {
    let width = 40;
    let height = 26;
    let mut view_bytes = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            view_bytes[y * width + x] = ((x + y * 40) % 128) as u8;
        }
    }
    (view_bytes, width, height)
}

// ==========================================
// 1. Font Text Exporter Tests (12 tests)
// ==========================================

#[test]
fn test_export_font_asm_dec_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::Assembler,
        DataType::Decimal,
    );
    let expected = read_fixture_str("exports/font_asm_dec.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_asm_hex_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::Assembler,
        DataType::Hexadecimal,
    );
    let expected = read_fixture_str("exports/font_asm_hex.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_action_dec_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::Action,
        DataType::Decimal,
    );
    let expected = read_fixture_str("exports/font_action_dec.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_action_hex_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::Action,
        DataType::Hexadecimal,
    );
    let expected = read_fixture_str("exports/font_action_hex.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_ataribasic_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::AtariBasic,
        DataType::Decimal,
    );
    let expected = read_fixture_str("exports/font_ataribasic.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_fastbasic_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::FastBasic,
        DataType::Decimal,
    );
    let expected = read_fixture_str("exports/font_fastbasic.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_mads_dec_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::MADSdta,
        DataType::Decimal,
    );
    let expected = read_fixture_str("exports/font_mads_dec.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_mads_hex_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::MADSdta,
        DataType::Hexadecimal,
    );
    let expected = read_fixture_str("exports/font_mads_hex.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_c_dec_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::CDataArray,
        DataType::Decimal,
    );
    let expected = read_fixture_str("exports/font_c_dec.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_c_hex_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::CDataArray,
        DataType::Hexadecimal,
    );
    let expected = read_fixture_str("exports/font_c_hex.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_pascal_dec_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::MadPascalArray,
        DataType::Decimal,
    );
    let expected = read_fixture_str("exports/font_pascal_dec.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_font_pascal_hex_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_as_text(
        fonts.as_bytes(),
        FontSelection::Font1,
        FormatType::MadPascalArray,
        DataType::Hexadecimal,
    );
    let expected = read_fixture_str("exports/font_pascal_hex.txt");
    assert_eq!(actual, expected);
}

// ==========================================
// 2. Font BASIC Listing .lst (1 test)
// ==========================================

#[test]
fn test_export_font_lst_golden() {
    let fonts = create_standard_font_setup();
    let actual = export_font_lst(fonts.as_bytes(), 0);
    let expected = read_fixture_bytes("exports/font_default.lst");
    assert_eq!(actual, expected, "font_default.lst binary mismatch!");
}

// ==========================================
// 3. Font BMP Exporters (2 tests)
// ==========================================

#[test]
fn test_export_font_bmp_mono_golden() {
    let (fonts, renderer) = create_standard_renderer();
    let mut atlas = FontAtlasBuffer::new();
    renderer.render_all_fonts(&fonts, RenderColorMode::Mono, &mut atlas);

    let actual = export_font_bmp(&atlas, FontSelection::Font1, false);
    let expected = read_fixture_bytes("exports/font_default_mono.bmp");
    assert_eq!(actual, expected, "font_default_mono.bmp binary mismatch!");
}

#[test]
fn test_export_font_bmp_color_golden() {
    let (fonts, renderer) = create_standard_renderer();
    let mut atlas = FontAtlasBuffer::new();
    renderer.render_all_fonts(&fonts, RenderColorMode::Mono, &mut atlas);

    let actual = export_font_bmp(&atlas, FontSelection::Font1, true);
    let expected = read_fixture_bytes("exports/font_default_color.bmp");
    assert_eq!(actual, expected, "font_default_color.bmp binary mismatch!");
}

// ==========================================
// 4. View Text Exporter Tests (8 tests)
// ==========================================

#[test]
fn test_export_view_asm_hex_golden() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_as_text(
        &view,
        w,
        h,
        ViewExportRegion::full_standard(),
        FormatType::Assembler,
        DataType::Hexadecimal,
        false,
    );
    let expected = read_fixture_str("exports/view_asm_hex.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_view_action_hex_golden() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_as_text(
        &view,
        w,
        h,
        ViewExportRegion::full_standard(),
        FormatType::Action,
        DataType::Hexadecimal,
        false,
    );
    let expected = read_fixture_str("exports/view_action_hex.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_view_ataribasic_golden() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_as_text(
        &view,
        w,
        h,
        ViewExportRegion::full_standard(),
        FormatType::AtariBasic,
        DataType::Decimal,
        false,
    );
    let expected = read_fixture_str("exports/view_ataribasic.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_view_fastbasic_golden() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_as_text(
        &view,
        w,
        h,
        ViewExportRegion::full_standard(),
        FormatType::FastBasic,
        DataType::Decimal,
        false,
    );
    let expected = read_fixture_str("exports/view_fastbasic.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_view_mads_hex_golden() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_as_text(
        &view,
        w,
        h,
        ViewExportRegion::full_standard(),
        FormatType::MADSdta,
        DataType::Hexadecimal,
        false,
    );
    let expected = read_fixture_str("exports/view_mads_hex.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_view_c_hex_golden() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_as_text(
        &view,
        w,
        h,
        ViewExportRegion::full_standard(),
        FormatType::CDataArray,
        DataType::Hexadecimal,
        false,
    );
    let expected = read_fixture_str("exports/view_c_hex.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_view_pascal_hex_golden() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_as_text(
        &view,
        w,
        h,
        ViewExportRegion::full_standard(),
        FormatType::MadPascalArray,
        DataType::Hexadecimal,
        false,
    );
    let expected = read_fixture_str("exports/view_pascal_hex.txt");
    assert_eq!(actual, expected);
}

#[test]
fn test_export_view_asm_transposed_golden() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_as_text(
        &view,
        w,
        h,
        ViewExportRegion::full_standard(),
        FormatType::Assembler,
        DataType::Hexadecimal,
        true,
    );
    let expected = read_fixture_str("exports/view_asm_transposed.txt");
    assert_eq!(actual, expected);
}

// ==========================================
// 5. Binary View Exporter Tests (3 tests)
// ==========================================

#[test]
fn test_export_view_binary_row_major() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_binary(&view, w, h, ViewExportRegion::full_standard(), false);
    // Row-major, full 40x26 region: identical to the underlying storage.
    assert_eq!(actual.len(), 40 * 26);
    assert_eq!(actual.as_slice(), &view[..]);
}

#[test]
fn test_export_view_binary_transposed() {
    let (view, w, h) = create_sample_view_grid();
    let actual = export_view_binary(&view, w, h, ViewExportRegion::full_standard(), true);
    assert_eq!(actual.len(), 40 * 26);
    // Column-major: element (x, y) -> index x * 26 + y.
    for x in 0..40 {
        for y in 0..26 {
            assert_eq!(actual[x * 26 + y], view[y * 40 + x]);
        }
    }
}

#[test]
fn test_export_view_binary_subregion_clamps_out_of_bounds() {
    let view = vec![7u8; 40 * 26];
    // Region starting at (38, 24) with size 4x4: only cells (38..40, 24..26) are valid.
    let region = ViewExportRegion::new(38, 24, 4, 4);
    let actual = export_view_binary(&view, 40, 26, region, false);
    assert_eq!(actual.len(), 4 * 4);
    // First two cells of each row are valid (x=38,39), the next two are padded zeros.
    assert_eq!(actual[0], 7);
    assert_eq!(actual[1], 7);
    assert_eq!(actual[2], 0);
    assert_eq!(actual[3], 0);
    assert_eq!(actual[4], 7);
    assert_eq!(actual[5], 7);
    assert_eq!(actual[6], 0);
    assert_eq!(actual[7], 0);
    // Rows 3 and 4 (y=26,27) are fully out of bounds -> all zeros.
    assert_eq!(&actual[8..16], &[0u8; 8]);
}
