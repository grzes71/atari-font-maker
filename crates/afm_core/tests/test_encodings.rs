use afm_core::font::glyph::{GlyphBytes, convert_atari_char};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct MonoVector {
    input_byte: u8,
    decoded: [u8; 8],
}

#[derive(Deserialize)]
struct Color2BitVector {
    input_byte: u8,
    decoded: [u8; 4],
}

#[derive(Deserialize)]
struct Color4BitVector {
    input_byte: u8,
    decoded: [u8; 2],
}

#[derive(Deserialize)]
struct AtariCharVector {
    ascii_in: u8,
    atari_char_out: u8,
}

#[derive(Deserialize)]
struct GlyphMatrixConversions {
    sample_glyph: String,
    mono_matrix_char0: [[u8; 8]; 8],
    color5_matrix_char0: [[u8; 8]; 8],
    color4bit_matrix_char0: [[u8; 8]; 8],
    encoded_char1_from_color5: String,
    encoded_char2_from_color4bit: String,
}

fn load_json_fixture<T: DeserializeOwned>(relative: &str) -> T {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative);
    let raw = fs::read_to_string(path).expect("Failed to read fixture file");
    let clean = raw.trim_start_matches('\u{feff}');
    serde_json::from_str(clean).expect("Failed to parse JSON fixture")
}

#[test]
fn test_mono_vectors_golden() {
    let vectors: Vec<MonoVector> = load_json_fixture("encodings/mono_vectors.json");

    assert_eq!(vectors.len(), 256);
    for vec in vectors {
        let decoded = GlyphBytes::decode_mono(vec.input_byte);
        assert_eq!(
            decoded, vec.decoded,
            "Failed DecodeMono for byte {}",
            vec.input_byte
        );

        let encoded = GlyphBytes::encode_mono(&vec.decoded);
        assert_eq!(
            encoded, vec.input_byte,
            "Failed EncodeMono roundtrip for byte {}",
            vec.input_byte
        );
    }
}

#[test]
fn test_color_2bit_vectors_golden() {
    let vectors: Vec<Color2BitVector> = load_json_fixture("encodings/color_2bit_vectors.json");

    assert_eq!(vectors.len(), 256);
    for vec in vectors {
        let decoded = GlyphBytes::decode_color_2bit(vec.input_byte);
        assert_eq!(
            decoded, vec.decoded,
            "Failed DecodeColor2Bit for byte {}",
            vec.input_byte
        );

        let encoded = GlyphBytes::encode_color_2bit(&vec.decoded);
        assert_eq!(
            encoded, vec.input_byte,
            "Failed EncodeColor2Bit roundtrip for byte {}",
            vec.input_byte
        );
    }
}

#[test]
fn test_color_4bit_vectors_golden() {
    let vectors: Vec<Color4BitVector> = load_json_fixture("encodings/color_4bit_vectors.json");

    assert_eq!(vectors.len(), 256);
    for vec in vectors {
        let decoded = GlyphBytes::decode_color_4bit(vec.input_byte);
        assert_eq!(
            decoded, vec.decoded,
            "Failed DecodeColor4Bit for byte {}",
            vec.input_byte
        );

        let encoded = GlyphBytes::encode_color_4bit(&vec.decoded);
        assert_eq!(
            encoded, vec.input_byte,
            "Failed EncodeColor4Bit roundtrip for byte {}",
            vec.input_byte
        );
    }
}

#[test]
fn test_atari_convert_char_golden() {
    let vectors: Vec<AtariCharVector> =
        load_json_fixture("encodings/atari_convert_char_vectors.json");

    assert_eq!(vectors.len(), 256);
    for vec in vectors {
        let converted = convert_atari_char(vec.ascii_in);
        assert_eq!(
            converted, vec.atari_char_out,
            "Failed AtariConvertChar for byte {}",
            vec.ascii_in
        );
    }
}

#[test]
fn test_glyph_matrix_conversions_golden() {
    let fixture: GlyphMatrixConversions =
        load_json_fixture("encodings/glyph_matrix_conversions.json");

    let sample_raw = hex::decode(&fixture.sample_glyph).unwrap();
    let mut sample_bytes = [0u8; 8];
    sample_bytes.copy_from_slice(&sample_raw);
    let glyph = GlyphBytes::new(sample_bytes);

    // 1. Mono matrix check
    let mono_matrix = glyph.to_2color_matrix();
    assert_eq!(mono_matrix, fixture.mono_matrix_char0);

    // 2. 5-color matrix check
    let color5_matrix = glyph.to_5color_matrix();
    assert_eq!(color5_matrix, fixture.color5_matrix_char0);

    // 3. 4-bit matrix check
    let color4bit_matrix = glyph.to_4bit_matrix();
    assert_eq!(color4bit_matrix, fixture.color4bit_matrix_char0);

    // 4. Set5Color encoding check
    let encoded_color5 = GlyphBytes::from_5color_matrix(&fixture.color5_matrix_char0);
    assert_eq!(
        hex::encode_upper(encoded_color5.as_bytes()),
        fixture.encoded_char1_from_color5
    );

    // 5. Set4Bit encoding check
    let encoded_color4bit = GlyphBytes::from_4bit_matrix(&fixture.color4bit_matrix_char0);
    assert_eq!(
        hex::encode_upper(encoded_color4bit.as_bytes()),
        fixture.encoded_char2_from_color4bit
    );
}
