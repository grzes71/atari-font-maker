//! Phase 21B-1 MegaCopy regression tests — view selection/copy/paste/transform.

#[path = "../src/state.rs"]
mod state;

use state::{ClipboardTransform, GuiState};

#[test]
fn test_megacopy_selection_rect_normalization() {
    let mut s = GuiState::new();
    s.begin_megacopy_selection(3, 4);
    s.update_megacopy_selection(0, 1); // reversed drag
    assert_eq!(s.megacopy_selection_rect(), Some((0, 1, 4, 4)));
}

#[test]
fn test_megacopy_copy_paste_preserves_chars_and_fonts() {
    let mut s = GuiState::new();
    for y in 0..2 {
        for x in 0..3 {
            s.project.view_bytes[y * 40 + x] = (x + y * 3) as u8;
        }
    }
    s.project.line_fonts[0] = 1;
    s.project.line_fonts[1] = 3;

    s.begin_megacopy_selection(0, 0);
    s.finish_megacopy_selection(2, 1);
    assert_eq!(s.megacopy_selection_rect(), Some((0, 0, 3, 2)));

    s.copy_megacopy_selection();
    let clip = s.clipboard.clone().expect("clipboard filled");
    assert_eq!(clip.verify_width_height(), Some((3, 2)));
    // C# FontNr format: decimal digits, one per row.
    assert_eq!(clip.font_nr.as_deref(), Some("13"));
    // Data: 3*2*8 = 48 glyph bytes.
    assert_eq!(
        hex::decode(clip.data.as_deref().unwrap()).unwrap().len(),
        48
    );
    assert_eq!(clip.nulls.as_deref(), Some("000000"));

    // Paste at (5,5).
    s.paste_view_selection(5, 5);
    assert_eq!(s.project.view_bytes[5 * 40 + 5], 0);
    assert_eq!(s.project.view_bytes[5 * 40 + 6], 1);
    assert_eq!(s.project.view_bytes[5 * 40 + 7], 2);
    assert_eq!(s.project.view_bytes[6 * 40 + 5], 3);
    assert_eq!(s.project.line_fonts[5], 1);
    assert_eq!(s.project.line_fonts[6], 3);
}

#[test]
fn test_megacopy_paste_undo_redo() {
    let mut s = GuiState::new();
    s.project.view_bytes[0] = 0xAA;
    s.begin_megacopy_selection(0, 0);
    s.finish_megacopy_selection(0, 0);
    s.copy_megacopy_selection();

    s.paste_view_selection(1, 1);
    assert_eq!(s.project.view_bytes[1 * 40 + 1], 0xAA);

    s.view_undo();
    assert_eq!(s.project.view_bytes[1 * 40 + 1], 0);

    s.view_redo();
    assert_eq!(s.project.view_bytes[1 * 40 + 1], 0xAA);
}

#[test]
fn test_megacopy_paste_clips_to_screen() {
    let mut s = GuiState::new();
    for i in 0..4 {
        s.project.view_bytes[i] = 0x10 + i as u8;
    }
    s.begin_megacopy_selection(0, 0);
    s.finish_megacopy_selection(3, 0);
    s.copy_megacopy_selection();

    // Paste at (39, 25) — only cell (39,25) fits.
    s.paste_view_selection(39, 25);
    assert_eq!(s.project.view_bytes[25 * 40 + 39], 0x10);
    // Cell (0,0) must be untouched (out-of-bounds paste clipped).
    assert_eq!(s.project.view_bytes[0], 0x10);
}

#[test]
fn test_megacopy_transform_shift_left_mono() {
    let mut s = GuiState::new();
    s.fonts.as_bytes_mut()[0] = 0x0F;
    s.fonts.as_bytes_mut()[1] = 0xF0;
    s.project.view_bytes[0] = 0; // char 0, font 1
    s.project.line_fonts[0] = 1;

    s.begin_megacopy_selection(0, 0);
    s.finish_megacopy_selection(0, 0);
    s.copy_megacopy_selection();

    let before = hex::decode(s.clipboard.as_ref().unwrap().data.as_deref().unwrap()).unwrap();
    assert_eq!(before[0], 0x0F);
    assert_eq!(before[1], 0xF0);

    s.transform_clipboard(ClipboardTransform::ShiftLeft);
    let after = hex::decode(s.clipboard.as_ref().unwrap().data.as_deref().unwrap()).unwrap();
    assert_eq!(after[0], 0x1E);
    assert_eq!(after[1], 0xE1);

    s.paste_clipboard_into_font(1);
    assert_eq!(s.fonts.as_bytes()[0], 0x1E);
    assert_eq!(s.fonts.as_bytes()[1], 0xE1);
}

#[test]
fn test_megacopy_copy_null_data_when_no_selection() {
    let mut s = GuiState::new();
    s.copy_megacopy_selection();
    assert!(s.clipboard.is_none());
}

#[test]
fn test_megacopy_survives_save_reload() {
    let p = std::env::temp_dir().join(format!("afm_mc_{}.atrview", std::process::id()));
    {
        let mut s = GuiState::new();
        for y in 0..2 {
            for x in 0..2 {
                s.project.view_bytes[y * 40 + x] = 0x50 + (x + y * 2) as u8;
            }
        }
        s.begin_megacopy_selection(0, 0);
        s.finish_megacopy_selection(1, 1);
        s.copy_megacopy_selection();
        s.paste_view_selection(10, 10);
        s.save_project_file(&p).unwrap();
    }
    let mut s = GuiState::new();
    s.open_project_file(&p).unwrap();
    assert_eq!(s.project.view_bytes[10 * 40 + 10], 0x50);
    assert_eq!(s.project.view_bytes[10 * 40 + 11], 0x51);
    assert_eq!(s.project.view_bytes[11 * 40 + 10], 0x52);

    let _ = std::fs::remove_file(&p);
}
