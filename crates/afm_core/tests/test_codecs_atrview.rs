use std::fs;
use std::io::Cursor;
use std::path::Path;

use afm_core::codecs::atrview::{
    AtrViewInfoJson, AtrViewProject, fix_color_hex_string, fix_font_data_hex_string,
};
use afm_core::error::AtrViewFormatError;

fn fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

fn read_fixture_str(relative: &str) -> String {
    let raw = fs::read_to_string(fixture_path(relative)).expect("Read fixture");
    raw.trim_start_matches('\u{feff}').to_string()
}

#[test]
fn test_default_atrview_loading_and_reserialization_golden() {
    let json_text = read_fixture_str("projects/default.atrview");

    // 1. Direct DTO deserialization
    let dto: AtrViewInfoJson = serde_json::from_str(&json_text).expect("Parse AtrViewInfoJson");
    assert_eq!(dto.version.as_deref(), Some("2023"));
    assert_eq!(dto.colored_gfx, "0");
    assert_eq!(dto.fontname1, "Default.fnt");
    assert_eq!(dto.fontname2, "Default.fnt");
    assert_eq!(dto.fontname3.as_deref(), Some("Default.fnt"));
    assert_eq!(dto.fontname4.as_deref(), Some("Default.fnt"));
    assert_eq!(dto.pages.as_ref().map(|p| p.len()), Some(1));

    // 2. Direct DTO serialization matches default_reserialized.atrview byte-for-byte
    let reserialized_json = serde_json::to_string(&dto).unwrap();
    let expected_reserialized = read_fixture_str("projects/default_reserialized.atrview");
    assert_eq!(reserialized_json, expected_reserialized);

    // 3. High-level domain project load
    let mut cursor = Cursor::new(json_text.as_bytes());
    let project = AtrViewProject::load(&mut cursor).expect("AtrViewProject::load");
    assert_eq!(project.version, "2023");
    assert_eq!(project.colored_gfx, 0);
    assert_eq!(project.width, 40);
    assert_eq!(project.height, 26);
    assert_eq!(project.view_bytes.len(), 40 * 26);
    assert_eq!(project.line_fonts.len(), 26);
    assert_eq!(project.font_banks.as_bytes().len(), 4096);
    // Colors normalized to 10 registers (12 hex chars -> 20 hex chars)
    assert_eq!(
        project.colors,
        [0x0E, 0x00, 0x28, 0xCA, 0x94, 0x46, 0x16, 0x1A, 0xB4, 0xBA]
    );
}

#[test]
fn test_sample_v1911_backward_compatibility_golden() {
    let json_text = read_fixture_str("projects/sample_v1911.atrview");
    let project = AtrViewProject::from_json_str(&json_text).expect("Load sample_v1911");

    assert_eq!(project.version, "1911");
    assert_eq!(project.colored_gfx, 0);
    // V1911 compatibility: width/height defaulted to 40x26
    assert_eq!(project.width, 40);
    assert_eq!(project.height, 26);
    assert_eq!(project.forty_bytes, "0");

    // Font names for 3 and 4 defaulted to Default.fnt
    assert_eq!(project.font_names[2], "Default.fnt");
    assert_eq!(project.font_names[3], "Default.fnt");

    // 2048-byte font was duplicated into 4096-byte 4-bank buffer
    let bank0 = &project.font_banks.as_bytes()[0..2048];
    let bank1 = &project.font_banks.as_bytes()[2048..4096];
    assert_eq!(bank0, bank1);
}

#[test]
fn test_sample_v2007_backward_compatibility_golden() {
    let json_text = read_fixture_str("projects/sample_v2007.atrview");
    let project = AtrViewProject::from_json_str(&json_text).expect("Load sample_v2007");

    assert_eq!(project.version, "2007");
    assert_eq!(project.colored_gfx, 1);
    assert_eq!(project.width, 32);
    assert_eq!(project.height, 26);
    assert_eq!(project.forty_bytes, "0");
}

#[test]
fn test_atrview_roundtrip_domain() {
    let original_json = read_fixture_str("projects/default.atrview");
    let project = AtrViewProject::from_json_str(&original_json).unwrap();

    let mut saved_bytes = Vec::new();
    project.save(&mut saved_bytes).unwrap();

    let reloaded_project = AtrViewProject::load(&mut Cursor::new(&saved_bytes)).unwrap();
    assert_eq!(project, reloaded_project);
}

#[test]
fn test_fix_color_and_font_data_helpers() {
    // 12-char hex string (6 colors) -> padded with 161AB4BA
    let short_colors = "0028CA46981A";
    let fixed_colors = fix_color_hex_string(short_colors);
    assert_eq!(fixed_colors, "0028CA46981A161AB4BA");

    // 20-char hex string remains unchanged
    let full_colors = "0028CA46981A161AB4BA";
    assert_eq!(fix_color_hex_string(full_colors), full_colors);

    // 4096-char font data is duplicated
    let short_font = "A".repeat(4096);
    let fixed_font = fix_font_data_hex_string(&short_font);
    assert_eq!(fixed_font.len(), 8192);
    assert_eq!(fixed_font, "A".repeat(8192));
}

#[test]
fn test_atrview_malformed_inputs() {
    // Invalid JSON
    let bad_json = "{ invalid json: 123 ";
    let res1 = AtrViewProject::from_json_str(bad_json);
    assert!(matches!(res1, Err(AtrViewFormatError::Json(_))));

    // Invalid hex characters in Chars
    let bad_hex_json = r#"{"Version":"2023","Chars":"NOT_HEX"}"#;
    let res2 = AtrViewProject::from_json_str(bad_hex_json);
    assert!(matches!(res2, Err(AtrViewFormatError::Hex(_))));
}
