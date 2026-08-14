use std::fs;
use std::path::Path;

use afm_core::analysis::{analyze_character_usage, analyze_duplicates, analyze_project};
use afm_core::codecs::atrview::{AtrViewProject, SavedPageData};
use afm_core::font::bank::FontBankSet;

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

#[test]
fn test_analysis_default_project() {
    let json_str = read_fixture_str("projects/default.atrview");
    let project = AtrViewProject::from_json_str(&json_str).unwrap();

    let default_fnt = read_fixture_bytes("projects/Default.fnt");
    let mut fonts = FontBankSet::new();
    for i in 0..4 {
        fonts.copy_to(&default_fnt, 0, i * 1024, 1024);
    }

    let result = analyze_project(&project, &fonts);

    // Verify default.atrview has 40x26 = 1040 total characters
    let total_full_chars: u32 = result.full_char_counts.iter().sum();
    assert_eq!(total_full_chars, 1040);

    let total_combined_chars: u32 = result.combined_char_counts.iter().sum();
    assert_eq!(total_combined_chars, 1040);

    // Detailed character usage
    let usage_space = analyze_character_usage(&project, 0, 0);
    assert_eq!(usage_space.font_index, 0);
    assert_eq!(usage_space.base_char, 0);
    assert!(!usage_space.page_usages.is_empty());
    assert_eq!(usage_space.page_usages[0].first_occurrence_index, Some(0));

    // Duplicate analysis on identical default fonts
    let dup_report = analyze_duplicates(&result, 0, 0);
    assert_eq!(dup_report.font_index, 0);
}

#[test]
fn test_analysis_duplicate_detection() {
    let default_fnt = read_fixture_bytes("projects/Default.fnt");
    let mut fonts = FontBankSet::new();
    for i in 0..4 {
        fonts.copy_to(&default_fnt, 0, i * 1024, 1024);
    }

    let mut page_view_bytes = vec![0u8; 40 * 26];
    page_view_bytes[0] = 65; // 'A'
    page_view_bytes[1] = 65 + 128; // inverted 'A'

    let mut project = AtrViewProject::default();
    project.pages.push(SavedPageData {
        nr: 1,
        name: "Test Page".to_string(),
        view: hex::encode(page_view_bytes),
        selected_font: hex::encode(vec![1u8; 26]),
        width: 40,
        height: 26,
    });

    // In Default.fnt, make char 5 identical to char 65 ('A')
    let glyph = fonts.get_glyph_at(65 * 8);
    fonts.set_glyph_at(5 * 8, &glyph);

    let result = analyze_project(&project, &fonts);
    assert_eq!(result.duplicate_of_char[5], 5);
    assert_eq!(result.duplicate_of_char[65], 5);

    let dup_report = analyze_duplicates(&result, 0, 65);
    assert!(dup_report.duplicate_char_indices.contains(&5));
}
