use afm_core::font::bank::FontBankSet;
use afm_core::font::{render_text_to_clipboard, text_to_atari_screen_codes};

#[test]
fn test_atascii_string_to_screen_codes() {
    let text = "ATARI";
    let codes = text_to_atari_screen_codes(text, false);
    // 'A' -> 33 (0x21), 'T' -> 52 (0x34), 'R' -> 50 (0x32), 'I' -> 41 (0x29)
    assert_eq!(codes, vec![33, 52, 33, 50, 41]);

    let inv_codes = text_to_atari_screen_codes(text, true);
    assert_eq!(
        inv_codes,
        vec![33 + 128, 52 + 128, 33 + 128, 50 + 128, 41 + 128]
    );
}

#[test]
fn test_render_text_to_clipboard_structure() {
    let mut fonts = FontBankSet::new();
    // Put distinctive byte at 'A' in Font 1
    fonts.as_bytes_mut()[33 * 8] = 0xAA;

    let clip = render_text_to_clipboard("A", false, 0, &fonts);
    assert_eq!(clip.width, Some("1".to_string()));
    assert_eq!(clip.height, Some("1".to_string()));
    assert_eq!(clip.chars, Some("21".to_string()));
    assert_eq!(clip.font_nr, Some("1".to_string()));
    assert_eq!(clip.nulls, Some("0".to_string()));

    let data_hex = clip.data.unwrap();
    assert!(data_hex.starts_with("AA"));
}
