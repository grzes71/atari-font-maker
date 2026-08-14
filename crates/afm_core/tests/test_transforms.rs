use afm_core::font::bank::FontBankSet;
use afm_core::font::glyph::GlyphBytes;
use afm_core::font::transforms;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct CharacterOffsetVector {
    character_index: usize,
    offset_bank1: usize,
    offset_bank2: usize,
}

#[derive(Deserialize)]
struct GlyphTransformGolden {
    character_index: usize,
    original: String,
    rotate_left: String,
    rotate_right: String,
    mirror_h_mono: String,
    mirror_h_2bit: String,
    mirror_h_4bit: String,
    mirror_v: String,
    shift_left_mono: String,
    shift_left_2bit: String,
    shift_left_4bit: String,
    shift_right_mono: String,
    shift_right_2bit: String,
    shift_right_4bit: String,
    shift_up: String,
    shift_down: String,
    inverted: String,
    cleared: String,
}

#[derive(Deserialize)]
struct EdgeCaseTransformGolden {
    name: String,
    original: String,
    rotate_left: String,
    rotate_right: String,
    mirror_h_mono: String,
    mirror_h_2bit: String,
    mirror_h_4bit: String,
    mirror_v: String,
    shift_left_mono: String,
    shift_left_2bit: String,
    shift_left_4bit: String,
    shift_right_mono: String,
    shift_right_2bit: String,
    shift_right_4bit: String,
    shift_up: String,
    shift_down: String,
    inverted: String,
}

#[derive(Deserialize)]
struct BankOpGolden {
    op: String,
    hash: Option<String>,
    dup_same: Option<bool>,
    dup_different: Option<bool>,
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

fn hex_to_glyph(hex_str: &str) -> GlyphBytes {
    let bytes = hex::decode(hex_str).expect("Valid hex string");
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes);
    GlyphBytes::new(arr)
}

fn glyph_to_hex(glyph: &GlyphBytes) -> String {
    hex::encode_upper(glyph.as_bytes())
}

#[test]
fn test_character_offsets_golden() {
    let vectors: Vec<CharacterOffsetVector> =
        load_json_fixture("transforms/character_offsets.json");

    assert_eq!(vectors.len(), 512);
    for vec in vectors {
        let off1 = FontBankSet::character_offset(vec.character_index, false);
        assert_eq!(
            off1, vec.offset_bank1,
            "Failed offset_bank1 for char {}",
            vec.character_index
        );

        let off2 = FontBankSet::character_offset(vec.character_index, true);
        assert_eq!(
            off2, vec.offset_bank2,
            "Failed offset_bank2 for char {}",
            vec.character_index
        );
    }
}

#[test]
fn test_glyph_transforms_golden_master() {
    let items: Vec<GlyphTransformGolden> =
        load_json_fixture("transforms/glyph_transforms_golden.json");

    assert_eq!(items.len(), 128);
    for item in &items {
        let orig = hex_to_glyph(&item.original);

        assert_eq!(
            glyph_to_hex(&transforms::rotate_left(&orig)),
            item.rotate_left,
            "RotateLeft mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::rotate_right(&orig)),
            item.rotate_right,
            "RotateRight mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::mirror_horizontal(&orig, 1)),
            item.mirror_h_mono,
            "MirrorH Mono mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::mirror_horizontal(&orig, 2)),
            item.mirror_h_2bit,
            "MirrorH 2-bit mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::mirror_horizontal(&orig, 4)),
            item.mirror_h_4bit,
            "MirrorH 4-bit mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::mirror_vertical(&orig)),
            item.mirror_v,
            "MirrorV mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_left(&orig, 1)),
            item.shift_left_mono,
            "ShiftLeft Mono mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_left(&orig, 2)),
            item.shift_left_2bit,
            "ShiftLeft 2-bit mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_left(&orig, 4)),
            item.shift_left_4bit,
            "ShiftLeft 4-bit mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_right(&orig, 1)),
            item.shift_right_mono,
            "ShiftRight Mono mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_right(&orig, 2)),
            item.shift_right_2bit,
            "ShiftRight 2-bit mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_right(&orig, 4)),
            item.shift_right_4bit,
            "ShiftRight 4-bit mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_up(&orig)),
            item.shift_up,
            "ShiftUp mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_down(&orig)),
            item.shift_down,
            "ShiftDown mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::invert(&orig)),
            item.inverted,
            "Invert mismatch for char {}",
            item.character_index
        );
        assert_eq!(
            glyph_to_hex(&transforms::clear()),
            item.cleared,
            "Clear mismatch for char {}",
            item.character_index
        );
    }
}

#[test]
fn test_edge_cases_transforms_golden_master() {
    let items: Vec<EdgeCaseTransformGolden> =
        load_json_fixture("transforms/edge_cases_transforms_golden.json");

    assert_eq!(items.len(), 8);
    for item in &items {
        let orig = hex_to_glyph(&item.original);

        assert_eq!(
            glyph_to_hex(&transforms::rotate_left(&orig)),
            item.rotate_left,
            "RotateLeft mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::rotate_right(&orig)),
            item.rotate_right,
            "RotateRight mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::mirror_horizontal(&orig, 1)),
            item.mirror_h_mono,
            "MirrorH Mono mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::mirror_horizontal(&orig, 2)),
            item.mirror_h_2bit,
            "MirrorH 2-bit mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::mirror_horizontal(&orig, 4)),
            item.mirror_h_4bit,
            "MirrorH 4-bit mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::mirror_vertical(&orig)),
            item.mirror_v,
            "MirrorV mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_left(&orig, 1)),
            item.shift_left_mono,
            "ShiftLeft Mono mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_left(&orig, 2)),
            item.shift_left_2bit,
            "ShiftLeft 2-bit mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_left(&orig, 4)),
            item.shift_left_4bit,
            "ShiftLeft 4-bit mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_right(&orig, 1)),
            item.shift_right_mono,
            "ShiftRight Mono mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_right(&orig, 2)),
            item.shift_right_2bit,
            "ShiftRight 2-bit mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_right(&orig, 4)),
            item.shift_right_4bit,
            "ShiftRight 4-bit mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_up(&orig)),
            item.shift_up,
            "ShiftUp mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::shift_down(&orig)),
            item.shift_down,
            "ShiftDown mismatch for {}",
            item.name
        );
        assert_eq!(
            glyph_to_hex(&transforms::invert(&orig)),
            item.inverted,
            "Invert mismatch for {}",
            item.name
        );
    }
}

fn load_default_fnt_4banks() -> FontBankSet {
    let default_fnt_bytes = fs::read(fixture_path("transforms/Default.fnt")).unwrap();
    assert_eq!(default_fnt_bytes.len(), 1024);

    let mut bank_set = FontBankSet::new();
    for bank_idx in 0..4 {
        bank_set.copy_to(&default_fnt_bytes, 0, bank_idx * 1024, 1024);
    }
    bank_set
}

#[test]
fn test_bank_operations_golden_master() {
    let ops: Vec<BankOpGolden> = load_json_fixture("transforms/bank_operations_golden.json");

    for item in ops {
        let mut bank_set = load_default_fnt_4banks();

        match item.op.as_str() {
            "ShiftFontLeft_noHole_char0_bank1" => {
                bank_set.shift_font_left(0, false, false);
                let actual_hex = hex::encode_upper(bank_set.as_bytes());
                assert_eq!(actual_hex, item.hash.unwrap());
            }
            "ShiftFontLeft_makeHole_char16_bank1" => {
                bank_set.shift_font_left(16, false, true);
                let actual_hex = hex::encode_upper(bank_set.as_bytes());
                assert_eq!(actual_hex, item.hash.unwrap());
            }
            "ShiftFontRight_noHole_char0_bank1" => {
                bank_set.shift_font_right(0, false, false);
                let actual_hex = hex::encode_upper(bank_set.as_bytes());
                assert_eq!(actual_hex, item.hash.unwrap());
            }
            "ShiftFontRight_makeHole_char32_bank1" => {
                bank_set.shift_font_right(32, false, true);
                let actual_hex = hex::encode_upper(bank_set.as_bytes());
                assert_eq!(actual_hex, item.hash.unwrap());
            }
            "DeleteAndShiftLeft_char10_bank1" => {
                bank_set.delete_and_shift_left(10, false);
                let actual_hex = hex::encode_upper(bank_set.as_bytes());
                assert_eq!(actual_hex, item.hash.unwrap());
            }
            "DeleteAndShiftRight_char20_bank1" => {
                bank_set.delete_and_shift_right(20, false);
                let actual_hex = hex::encode_upper(bank_set.as_bytes());
                assert_eq!(actual_hex, item.hash.unwrap());
            }
            "IsDuplicate_test" => {
                assert_eq!(bank_set.is_duplicate(0, 0, 0), item.dup_same.unwrap());
                assert_eq!(bank_set.is_duplicate(0, 0, 1), item.dup_different.unwrap());
            }
            other => panic!("Unknown bank operation in fixture: {}", other),
        }
    }
}
