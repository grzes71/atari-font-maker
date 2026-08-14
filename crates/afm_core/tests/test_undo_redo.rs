use serde_json::json;
use std::fs;
use std::path::Path;

use afm_core::font::bank::FontBankSet;
use afm_core::undo::{FontUndoBuffer, ViewUndoBuffer, ViewUndoState};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

fn read_fixture_str(relative: &str) -> String {
    let raw = fs::read_to_string(fixture_path(relative)).expect("Read fixture string");
    raw.trim_start_matches('\u{feff}').to_string()
}

#[test]
fn test_undo_redo_state_transitions_golden() {
    let mut undo_log = Vec::new();

    // 1. AtariFontUndoBuffer Tests
    let mut font_undo = FontUndoBuffer::new();
    let mut fonts = FontBankSet::new();
    fonts.as_bytes_mut()[0] = 0x11;
    font_undo.add_to_undo_initial(&fonts);

    let (redo_init, undo_init) = font_undo.get_redo_undo_button_state(false);
    undo_log.push(json!({
        "step": "font_init",
        "redo": redo_init,
        "undo": undo_init,
        "val": fonts.as_bytes()[0]
    }));

    // Mutation 1
    fonts.as_bytes_mut()[0] = 0x22;
    font_undo.add_to_undo(&fonts, true);
    let (redo_mut1, undo_mut1) = font_undo.get_redo_undo_button_state(false);
    undo_log.push(json!({
        "step": "font_mut1",
        "redo": redo_mut1,
        "undo": undo_mut1,
        "val": fonts.as_bytes()[0]
    }));

    // Undo
    font_undo.undo(&mut fonts);
    let (redo_after_undo, undo_after_undo) = font_undo.get_redo_undo_button_state(false);
    undo_log.push(json!({
        "step": "font_after_undo",
        "redo": redo_after_undo,
        "undo": undo_after_undo,
        "val": fonts.as_bytes()[0]
    }));

    // Redo
    font_undo.redo(&mut fonts);
    let (redo_after_redo, undo_after_redo) = font_undo.get_redo_undo_button_state(false);
    undo_log.push(json!({
        "step": "font_after_redo",
        "redo": redo_after_redo,
        "undo": undo_after_redo,
        "val": fonts.as_bytes()[0]
    }));

    // Test 250+ entries circular buffer overflow (260 edits)
    font_undo.setup();
    font_undo.add_to_undo_initial(&fonts);
    for i in 1..=260 {
        fonts.as_bytes_mut()[0] = (i % 256) as u8;
        font_undo.add_to_undo(&fonts, true);
    }
    undo_log.push(json!({
        "step": "font_overflow_260_edits",
        "current_index": font_undo.index(),
        "val_at_current": fonts.as_bytes()[0]
    }));

    // 2. AtariViewUndoBuffer Tests
    let mut view_undo = ViewUndoBuffer::new();
    let mut view_bytes = vec![0u8; 40 * 26];
    let font_lines = vec![1u8; 26];

    view_bytes[0] = 0x05;
    view_undo.push(ViewUndoState::new(view_bytes.clone(), font_lines.clone()));

    view_bytes[0] = 0x0A;
    view_undo.push(ViewUndoState::new(view_bytes.clone(), font_lines.clone()));

    let (undo_view_before, redo_view_before) = view_undo.get_redo_undo_button_state();
    undo_log.push(json!({
        "step": "view_pushed_twice",
        "undo_available": undo_view_before,
        "redo_available": redo_view_before,
        "val": view_bytes[0] as i64
    }));

    let restored_state = view_undo
        .undo(ViewUndoState::new(view_bytes.clone(), font_lines.clone()))
        .unwrap();
    view_bytes = restored_state.view_bytes;
    let (undo_view_after, redo_view_after) = view_undo.get_redo_undo_button_state();
    undo_log.push(json!({
        "step": "view_after_undo",
        "undo_available": undo_view_after,
        "redo_available": redo_view_after,
        "val": view_bytes[0] as i64
    }));

    let redo_state = view_undo
        .redo(ViewUndoState::new(view_bytes.clone(), font_lines.clone()))
        .unwrap();
    view_bytes = redo_state.view_bytes;
    undo_log.push(json!({
        "step": "view_after_redo",
        "val": view_bytes[0] as i64
    }));

    // Push 260 view states
    for i in 0..260 {
        view_bytes[0] = (i % 256) as u8;
        view_undo.push(ViewUndoState::new(view_bytes.clone(), font_lines.clone()));
    }
    undo_log.push(json!({
        "step": "view_overflow_260_pushes",
        "val": view_bytes[0] as i64
    }));

    let expected_json = read_fixture_str("undo/undo_redo_state_transitions.json");
    let expected_val: serde_json::Value = serde_json::from_str(&expected_json).unwrap();
    let actual_val: serde_json::Value = serde_json::Value::Array(undo_log);

    assert_eq!(
        actual_val, expected_val,
        "Undo/Redo state transitions mismatch with golden master!"
    );
}

#[test]
fn test_font_undo_difference_scan_and_branching() {
    let mut undo = FontUndoBuffer::new();
    let mut fonts = FontBankSet::new();

    undo.add_to_undo_initial(&fonts);
    assert_eq!(undo.index(), 0);

    // Scan with same bytes -> no change
    assert!(!undo.add_to_undo_full_difference_scan(&fonts));
    assert_eq!(undo.index(), 0);

    // Mutate and scan -> changes and records
    fonts.as_bytes_mut()[100] = 0xAA;
    assert!(undo.add_to_undo_full_difference_scan(&fonts));
    assert_eq!(undo.index(), 1);

    // Undo -> restores 0
    undo.undo(&mut fonts);
    assert_eq!(fonts.as_bytes()[100], 0x00);
    assert_eq!(undo.index(), 0);

    // New edit after undo -> branching (disallows redo)
    fonts.as_bytes_mut()[100] = 0xBB;
    undo.add_to_undo(&fonts, true);
    assert_eq!(undo.index(), 1);

    let (redo_enabled, undo_enabled) = undo.get_redo_undo_button_state(false);
    assert!(!redo_enabled);
    assert!(undo_enabled);
}

#[test]
fn test_view_undo_edge_cases_and_isolation() {
    let mut undo1 = ViewUndoBuffer::new();
    let undo2 = ViewUndoBuffer::new();

    let dummy_state1 = ViewUndoState::new(vec![1, 2, 3], vec![1; 26]);
    let dummy_state2 = ViewUndoState::new(vec![4, 5, 6], vec![2; 26]);

    // Undo/Redo on empty history returns None
    assert_eq!(undo1.undo(dummy_state1.clone()), None);
    assert_eq!(undo1.redo(dummy_state1.clone()), None);

    // Buffer 1 push
    undo1.push(dummy_state1.clone());
    assert_eq!(undo1.undo_count(), 1);
    assert_eq!(undo2.undo_count(), 0); // Isolated

    // Buffer 1 undo -> pushed to redo stack
    let undone = undo1.undo(dummy_state2.clone()).unwrap();
    assert_eq!(undone, dummy_state1);
    assert_eq!(undo1.redo_count(), 1);

    // Push new state -> clears redo stack
    undo1.push(dummy_state2);
    assert_eq!(undo1.redo_count(), 0);
}
