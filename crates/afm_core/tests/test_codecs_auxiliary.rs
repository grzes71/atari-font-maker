use std::fs;
use std::io::Cursor;
use std::path::Path;

use afm_core::codecs::clipboard::ClipboardJson;
use afm_core::codecs::config::ConfigurationJson;
use afm_core::codecs::tileset::{AtrTileJson, AtrTileSetJson};

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
fn test_clipboard_sample_golden() {
    let json_text = read_fixture_str("projects/clipboard_sample.json");

    let mut clip = ClipboardJson::from_json_str(&json_text).expect("Parse clipboard_sample.json");
    assert_eq!(clip.verify_width_height(), Some((4, 2)));

    // Matches ReferenceHarness: FixCharacters called
    clip.fix_characters(4, 2);

    let reserialized = clip.to_json_string().unwrap();
    let expected_obj: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    let actual_obj: serde_json::Value = serde_json::from_str(&reserialized).unwrap();
    assert_eq!(actual_obj, expected_obj);
}

#[test]
fn test_clipboard_fix_and_padding_behavior() {
    let mut clip = ClipboardJson {
        width: Some("2".to_string()),
        height: Some("3".to_string()),
        chars: Some("0102".to_string()), // needs 2*3*2 = 12 hex chars -> 8 zeros padded
        data: None,                      // needs 2*3*16 = 96 hex chars -> 96 zeros
        font_nr: Some("1".to_string()),  // needs 3 chars -> 2 ones padded
        nulls: None,                     // needs 2*3 = 6 chars -> 6 zeros
    };

    assert_eq!(clip.verify_width_height(), Some((2, 3)));
    assert!(clip.fix_all());

    let expected_data = "0".repeat(96);
    assert_eq!(clip.chars.as_deref(), Some("010200000000"));
    assert_eq!(clip.data.as_deref(), Some(expected_data.as_str()));
    assert_eq!(clip.font_nr.as_deref(), Some("111"));
    assert_eq!(clip.nulls.as_deref(), Some("000000"));

    // Invalid dimensions
    let bad_clip1 = ClipboardJson {
        width: Some("0".to_string()),
        height: Some("5".to_string()),
        ..Default::default()
    };
    assert_eq!(bad_clip1.verify_width_height(), None);

    let bad_clip2 = ClipboardJson {
        width: Some("abc".to_string()),
        height: Some("2".to_string()),
        ..Default::default()
    };
    assert_eq!(bad_clip2.verify_width_height(), None);
}

#[test]
fn test_sample_atrtileset_golden() {
    let json_text = read_fixture_str("projects/sample.atrtileset");

    let tile_set = AtrTileSetJson::from_json_str(&json_text).expect("Parse sample.atrtileset");
    assert_eq!(tile_set.version.as_deref(), Some("1"));
    assert_eq!(tile_set.tiles.as_ref().map(|t| t.len()), Some(1));

    let tile0 = &tile_set.tiles.as_ref().unwrap()[0];
    assert_eq!(tile0.nr, 0);
    assert_eq!(tile0.width, 5);
    assert_eq!(tile0.height, 5);
    assert_eq!(tile0.font, "11111111");

    // Roundtrip check
    let mut buf = Vec::new();
    tile_set.save(&mut buf).unwrap();

    let reloaded = AtrTileSetJson::load(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(tile_set, reloaded);
}

#[test]
fn test_sample_atrtile_golden() {
    let json_text = read_fixture_str("projects/sample.atrtile");

    let tile_container = AtrTileJson::from_json_str(&json_text).expect("Parse sample.atrtile");
    assert_eq!(tile_container.version.as_deref(), Some("1"));
    assert_eq!(tile_container.tile.nr, 42);
    assert_eq!(tile_container.tile.width, 5);
    assert_eq!(tile_container.tile.height, 5);
    assert_eq!(tile_container.tile.font, "22222222");

    // Roundtrip check
    let mut buf = Vec::new();
    tile_container.save(&mut buf).unwrap();

    let reloaded = AtrTileJson::load(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(tile_container, reloaded);
}

#[test]
fn test_sample_config_golden() {
    let json_text = read_fixture_str("projects/sample_config.json");

    let config = ConfigurationJson::from_json_str(&json_text).expect("Parse sample_config.json");
    assert_eq!(config.color_sets.len(), 6);
    for cs in &config.color_sets {
        assert_eq!(cs, "0E0028CA9446");
    }
    assert_eq!(config.analysis_alpha, 128);
    assert_eq!(config.import_line_width, 1);
    assert_eq!(config.import_width, 1);
    assert_eq!(config.import_height, 1);

    // Default configuration equals sample_config.json
    let default_config = ConfigurationJson::default();
    let default_serialized = default_config.to_json_string().unwrap();
    let expected_obj: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    let actual_obj: serde_json::Value = serde_json::from_str(&default_serialized).unwrap();
    assert_eq!(actual_obj, expected_obj);
}

#[test]
fn test_config_verify_defaults_clamping() {
    let mut config = ConfigurationJson {
        color_sets: vec!["00".to_string()],
        analysis_color: 200,      // clamp to 0
        analysis_alpha: 999,      // clamp to 128
        analysis_dup_color: -5,   // clamp to 0
        analysis_dup_alpha: -10,  // clamp to 128
        export_view_region_x: 50, // clamp to 0
        export_view_region_y: 30, // clamp to 0
        export_view_region_w: 100,
        export_view_region_h: 100,
        import_line_width: 0, // clamp to 1
        import_width: -2,     // clamp to 1
        import_height: 0,     // clamp to 1
        ..Default::default()
    };

    config.verify_defaults();

    assert_eq!(config.color_sets.len(), 6);
    assert_eq!(config.analysis_color, 0);
    assert_eq!(config.analysis_alpha, 128);
    assert_eq!(config.analysis_dup_color, 0);
    assert_eq!(config.analysis_dup_alpha, 128);
    assert_eq!(config.export_view_region_x, 0);
    assert_eq!(config.export_view_region_y, 0);
    assert_eq!(config.export_view_region_w, 1);
    assert_eq!(config.export_view_region_h, 1);
    assert_eq!(config.import_line_width, 1);
    assert_eq!(config.import_width, 1);
    assert_eq!(config.import_height, 1);
}
