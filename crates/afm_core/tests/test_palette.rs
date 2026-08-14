use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use afm_core::constants::{PALETTE_ENTRIES, PALETTE_SIZE};
use afm_core::error::PaletteFormatError;
use afm_core::palette::{ColorRgb, Palette};

#[derive(Deserialize)]
struct PaletteRgbEntry {
    index: usize,
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Deserialize)]
struct FindClosestVector {
    query_r: u8,
    query_g: u8,
    query_b: u8,
    matched_index: u8,
    matched_r: u8,
    matched_g: u8,
    matched_b: u8,
}

fn fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

fn load_json_fixture<T: DeserializeOwned>(relative: &str) -> T {
    let path = fixture_path(relative);
    let raw = fs::read_to_string(path).expect("Failed to read fixture file");
    let clean = raw.trim_start_matches('\u{feff}');
    serde_json::from_str(clean).expect("Failed to parse JSON fixture")
}

#[test]
fn test_palette_contents_and_rgb_golden() {
    let pal_file_bytes =
        fs::read(fixture_path("palette/altirraPAL.pal")).expect("Read altirraPAL.pal");
    assert_eq!(pal_file_bytes.len(), PALETTE_SIZE);

    // 1. Direct load from .pal file
    let mut cursor = Cursor::new(&pal_file_bytes);
    let loaded_palette = Palette::load(&mut cursor).expect("Palette::load");

    // 2. Default embedded palette
    let default_palette = Palette::default_altirra();
    assert_eq!(loaded_palette, default_palette);

    // 3. Verify all 256 colors against palette_rgb.json
    let expected_rgb: Vec<PaletteRgbEntry> = load_json_fixture("palette/palette_rgb.json");
    assert_eq!(expected_rgb.len(), PALETTE_ENTRIES);

    for entry in expected_rgb {
        let color = loaded_palette.color(entry.index as u8);
        assert_eq!(
            color.r, entry.r,
            "R mismatch at palette index {}",
            entry.index
        );
        assert_eq!(
            color.g, entry.g,
            "G mismatch at palette index {}",
            entry.index
        );
        assert_eq!(
            color.b, entry.b,
            "B mismatch at palette index {}",
            entry.index
        );
    }
}

#[test]
fn test_palette_save_and_roundtrip() {
    let pal_file_bytes =
        fs::read(fixture_path("palette/altirraPAL.pal")).expect("Read altirraPAL.pal");
    let palette = Palette::default_altirra();

    // 1. to_bytes check
    let exported_bytes = palette.to_bytes();
    assert_eq!(exported_bytes.as_slice(), pal_file_bytes.as_slice());

    // 2. save to writer check
    let mut out_buf = Vec::new();
    palette.save(&mut out_buf).expect("Palette::save");
    assert_eq!(out_buf, pal_file_bytes);

    // 3. Roundtrip with synthetic data
    let mut synthetic_bytes = [0u8; PALETTE_SIZE];
    for (i, b) in synthetic_bytes.iter_mut().enumerate() {
        *b = ((i * 11 + 37) % 256) as u8;
    }
    let synthetic_palette = Palette::from_bytes(&synthetic_bytes);
    assert_eq!(synthetic_palette.to_bytes(), synthetic_bytes);
}

#[test]
fn test_find_closest_vectors_golden() {
    let palette = Palette::default_altirra();
    let vectors: Vec<FindClosestVector> = load_json_fixture("palette/find_closest_vectors.json");

    for vec in vectors {
        let matched_index = palette.find_closest(vec.query_r, vec.query_g, vec.query_b);
        assert_eq!(
            matched_index, vec.matched_index,
            "FindClosest mismatch for RGB ({}, {}, {})",
            vec.query_r, vec.query_g, vec.query_b
        );

        // Verify that matched index is always even (Atari GTIA invariant)
        assert_eq!(
            matched_index % 2,
            0,
            "FindClosest returned odd index {} for RGB ({}, {}, {})",
            matched_index,
            vec.query_r,
            vec.query_g,
            vec.query_b
        );

        // Verify color components of matched index
        let matched_color = palette.color(matched_index);
        assert_eq!(matched_color.r, vec.matched_r);
        assert_eq!(matched_color.g, vec.matched_g);
        assert_eq!(matched_color.b, vec.matched_b);

        // Verify find_closest_rgb convenience method gives identical result
        let rgb_result =
            palette.find_closest_rgb(ColorRgb::new(vec.query_r, vec.query_g, vec.query_b));
        assert_eq!(rgb_result, matched_index);
    }
}

#[test]
fn test_find_closest_tie_breaking_behavior() {
    // Construct a synthetic palette where index 10 and index 20 have the EXACT same RGB distance
    let mut raw_bytes = [0u8; PALETTE_SIZE];
    // Index 10 -> (100, 100, 100)
    raw_bytes[10 * 3] = 100;
    raw_bytes[10 * 3 + 1] = 100;
    raw_bytes[10 * 3 + 2] = 100;
    // Index 20 -> (100, 100, 100)
    raw_bytes[20 * 3] = 100;
    raw_bytes[20 * 3 + 1] = 100;
    raw_bytes[20 * 3 + 2] = 100;

    let custom_palette = Palette::from_bytes(&raw_bytes);

    // Query (100, 100, 100): distance is 0 for both index 10 and index 20.
    // The tie-breaker (strict inequality best_distance > distance) MUST pick index 10 (the earlier one).
    let matched = custom_palette.find_closest(100, 100, 100);
    assert_eq!(matched, 10, "Tie breaker must favor the earlier index");
}

#[test]
fn test_palette_malformed_and_truncated_inputs() {
    let invalid_sizes = [0, 1, 10, 256, 500, 767, 769, 1024];

    for size in invalid_sizes {
        let bad_data = vec![0x33u8; size];
        let mut cur = Cursor::new(&bad_data);
        let res = Palette::load(&mut cur);

        match res {
            Err(PaletteFormatError::InvalidSize { expected, actual }) => {
                assert_eq!(expected, PALETTE_SIZE);
                assert_eq!(actual, size);
            }
            other => panic!(
                "Expected InvalidSize error for size {}, got {:?}",
                size, other
            ),
        }
    }
}
