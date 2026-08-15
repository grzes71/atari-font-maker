//! Phase 21C-1 regression tests — destructive-operation confirmation dialogs.
//!
//! These tests reproduce the C# `MessageBox.YesNo` semantics for destructive
//! operations (REL-1..REL-7 from `final-release-audit-report.md`): the operation
//! must NOT execute before the user confirms, and Cancel/No must leave the
//! application state bit-for-bit unchanged.

use std::cell::RefCell;
use std::rc::Rc;

use afm_gui::io::{TestClipboard, TestFileDialogs};
use afm_gui::{GuiController, GuiState};

fn controller() -> (
    Rc<RefCell<GuiState>>,
    GuiController,
    Rc<RefCell<TestClipboard>>,
    Rc<TestFileDialogs>,
) {
    let state = Rc::new(RefCell::new(GuiState::new()));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let dialogs = Rc::new(TestFileDialogs::new(vec![]));
    let ctrl = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs.clone(),
        clipboard.clone(),
    );
    (state, ctrl, clipboard, dialogs)
}

/// A compact, order-sensitive snapshot of every mutable domain field relevant to
/// destructive operations. Used to prove that Cancel/No is non-destructive.
#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    fonts: Vec<u8>,
    view_bytes: Vec<u8>,
    line_fonts: Vec<u8>,
    colors: [u8; 10],
    pages: Vec<(String, String, String)>, // (name, view-hex, selected-font-hex)
    active_page_index: usize,
    tileset_non_empty: Vec<usize>,
    is_dirty: bool,
    color_sets: Vec<String>,
    current_color_set_idx: usize,
}

fn snapshot(state: &GuiState) -> Snapshot {
    Snapshot {
        fonts: state.fonts.as_bytes().to_vec(),
        view_bytes: state.project.view_bytes.clone(),
        line_fonts: state.project.line_fonts.clone(),
        colors: state.project.colors,
        pages: state
            .project
            .pages
            .iter()
            .map(|p| (p.name.clone(), p.view.clone(), p.selected_font.clone()))
            .collect(),
        active_page_index: state.active_page_index,
        tileset_non_empty: state
            .tileset
            .tiles
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_valid())
            .map(|(i, _)| i)
            .collect(),
        is_dirty: state.is_dirty,
        color_sets: state.config.color_sets.clone(),
        current_color_set_idx: state.current_color_set_idx,
    }
}

// =========================================================================
// REL-1 — New Project
// =========================================================================

#[test]
fn test_new_project_cancel_keeps_state_bit_for_bit() {
    let (state, ctrl, _, _) = controller();

    // Make the project dirty and non-trivial.
    {
        let mut s = state.borrow_mut();
        s.set_view_cell(0, 0, 0xAA);
        s.set_line_font(0, 3);
        s.set_pixel(1, 1, 0);
        s.project.pages[0].name = "Custom Page".to_string();
    }
    let before = snapshot(&state.borrow());

    ctrl.new_project(); // shows confirmation, must NOT reset yet
    assert!(state.borrow().show_confirm_dialog);
    assert_eq!(
        snapshot(&state.borrow()),
        before,
        "state changed before confirm"
    );

    ctrl.cancel_pending(); // "No"
    assert!(!state.borrow().show_confirm_dialog);
    assert_eq!(
        snapshot(&state.borrow()),
        before,
        "cancel must be non-destructive"
    );
}

#[test]
fn test_new_project_confirm_resets_project() {
    let (state, ctrl, _, _) = controller();

    {
        let mut s = state.borrow_mut();
        s.set_view_cell(0, 0, 0xAA);
        assert!(s.is_dirty);
    }

    ctrl.new_project();
    assert!(state.borrow().show_confirm_dialog);
    ctrl.confirm_pending();

    let s = state.borrow();
    assert!(!s.show_confirm_dialog);
    assert_eq!(s.project.pages.len(), 1);
    assert_eq!(s.project.pages[0].name, "Page 1");
    assert!(s.project.view_bytes.iter().all(|&b| b == 0));
    assert!(!s.is_dirty);
    assert_eq!(s.active_page_index, 0);
}

#[test]
fn test_new_project_prompts_even_when_clean() {
    let (state, ctrl, _, _) = controller();
    assert!(!state.borrow().is_dirty);

    ctrl.new_project();
    // C# `ActionNewFontAndView` prompts unconditionally (not gated on is_dirty).
    assert!(state.borrow().show_confirm_dialog);
    ctrl.cancel_pending();
}

// =========================================================================
// REL-2 — Delete Page
// =========================================================================

#[test]
fn test_delete_page_cancel_keeps_pages() {
    let (state, ctrl, _, _) = controller();

    ctrl.view_add_page(); // Page 2, active index 1
    {
        let mut s = state.borrow_mut();
        s.project.view_bytes[0] = 0x77;
    }
    let before = snapshot(&state.borrow());
    assert_eq!(state.borrow().project.pages.len(), 2);

    ctrl.view_delete_page();
    assert!(state.borrow().show_confirm_dialog);
    assert_eq!(snapshot(&state.borrow()), before);

    ctrl.cancel_pending();
    assert_eq!(state.borrow().project.pages.len(), 2);
    assert_eq!(snapshot(&state.borrow()), before);
}

#[test]
fn test_delete_page_confirm_removes_only_active_page() {
    let (state, ctrl, _, _) = controller();

    // Page 1 content
    state.borrow_mut().set_view_cell(0, 0, 0x11);
    ctrl.view_add_page(); // Page 2 (active), switch saves Page 1
    state.borrow_mut().set_view_cell(0, 0, 0x22);
    ctrl.view_add_page(); // Page 3 (active)
    state.borrow_mut().set_view_cell(0, 0, 0x33);

    assert_eq!(state.borrow().project.pages.len(), 3);
    assert_eq!(state.borrow().active_page_index, 2);

    ctrl.view_delete_page(); // request delete Page 3
    ctrl.confirm_pending();

    let s = state.borrow();
    assert_eq!(s.project.pages.len(), 2);
    // Page 1 content unchanged (0x11), Page 2 content unchanged (0x22).
    let p0 = s.project.pages[0].view.chars().take(2).collect::<String>();
    let p1 = s.project.pages[1].view.chars().take(2).collect::<String>();
    assert_eq!(p0, "11", "page 1 must be untouched");
    assert_eq!(p1, "22", "page 2 must be untouched");
    // Active page becomes Page 2 (index 1), loaded without stale overwrite.
    assert_eq!(s.active_page_index, 1);
    assert_eq!(s.project.view_bytes[0], 0x22);
}

#[test]
fn test_delete_page_single_page_is_noop() {
    let (state, ctrl, _, _) = controller();
    assert_eq!(state.borrow().project.pages.len(), 1);

    ctrl.view_delete_page();
    // C# `ActionDeletePage`: `if (Pages.Count <= 1) return;` — no dialog, no delete.
    assert!(!state.borrow().show_confirm_dialog);
    assert_eq!(state.borrow().project.pages.len(), 1);
}

// =========================================================================
// REL-3 — New TileSet
// =========================================================================

#[test]
fn test_new_tileset_cancel_keeps_tiles() {
    let (state, ctrl, _, _) = controller();

    {
        let mut s = state.borrow_mut();
        s.set_tile_cell(0, 0, Some(0x42));
        s.tileset.tiles[3].set(1, 1, Some(0x43));
    }
    let before = snapshot(&state.borrow());
    assert_eq!(before.tileset_non_empty.len(), 2);

    ctrl.tileset_new_set();
    assert!(state.borrow().show_confirm_dialog);
    assert_eq!(snapshot(&state.borrow()), before);

    ctrl.cancel_pending();
    assert_eq!(snapshot(&state.borrow()), before);
    assert_eq!(state.borrow().tileset.tiles[0].get(0, 0), Some(0x42));
    assert_eq!(state.borrow().tileset.tiles[3].get(1, 1), Some(0x43));
}

#[test]
fn test_new_tileset_confirm_clears_tiles() {
    let (state, ctrl, _, _) = controller();

    {
        let mut s = state.borrow_mut();
        s.set_tile_cell(0, 0, Some(0x42));
    }

    ctrl.tileset_new_set();
    ctrl.confirm_pending();

    let s = state.borrow();
    assert!(!s.show_confirm_dialog);
    assert!(s.tileset.tiles.iter().all(|t| !t.is_valid()));
}

// =========================================================================
// REL-4 — Clear View (ViewActionsWindow counterpart: no prompt, fill 0,
// line fonts preserved — this matches C# `ViewActionsWindow.buttonClearView_Click`).
// =========================================================================

#[test]
fn test_clear_view_matches_view_actions_window_semantics() {
    let (state, ctrl, _, _) = controller();

    {
        let mut s = state.borrow_mut();
        s.project.view_bytes[0] = 0x55;
        s.project.line_fonts[0] = 3;
        s.project.line_fonts[1] = 2;
    }

    ctrl.clear_entire_view();

    let s = state.borrow();
    assert!(
        s.project.view_bytes.iter().all(|&b| b == 0),
        "view must be zeroed"
    );
    assert_eq!(s.project.line_fonts[0], 3, "line fonts must be preserved");
    assert_eq!(s.project.line_fonts[1], 2);
    assert!(s.can_view_undo(), "clear view must be undoable");
    assert!(s.is_dirty);
}

// =========================================================================
// REL-5 — Restore default colors
// =========================================================================

#[test]
fn test_restore_defaults_cancel_keeps_colors() {
    let (state, ctrl, _, _) = controller();

    {
        let mut s = state.borrow_mut();
        s.project.colors[2] = 0x77;
        s.project.colors[3] = 0x88;
    }
    let before = snapshot(&state.borrow());

    ctrl.restore_default_colors();
    assert!(state.borrow().show_confirm_dialog);
    assert_eq!(snapshot(&state.borrow()), before);

    ctrl.cancel_pending();
    assert_eq!(snapshot(&state.borrow()), before);
    assert_eq!(state.borrow().project.colors[2], 0x77);
    assert_eq!(state.borrow().project.colors[3], 0x88);
}

#[test]
fn test_restore_defaults_confirm_resets_registers() {
    let (state, ctrl, _, _) = controller();

    state.borrow_mut().project.colors[2] = 0x77;

    ctrl.restore_default_colors();
    ctrl.confirm_pending();

    assert_eq!(state.borrow().project.colors[2], 0x28);
    assert_eq!(state.borrow().project.colors[0], 0x0E);
}

// =========================================================================
// REL-6 — Load embedded fonts
// =========================================================================

fn fixture_path(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(rel)
}

#[test]
fn test_load_fonts_cancel_keeps_current_fonts() {
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(fixture_path(
        "projects/default.atrview",
    ))]));
    let state = Rc::new(RefCell::new(GuiState::new()));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let ctrl = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs.clone(),
        clipboard.clone(),
    );

    let fonts_before = state.borrow().fonts.as_bytes().to_vec();

    ctrl.open_project_from_path(&fixture_path("projects/default.atrview"));
    // Project metadata loaded; fonts pending confirmation.
    assert!(state.borrow().show_confirm_dialog);
    assert_eq!(
        state.borrow().fonts.as_bytes().to_vec(),
        fonts_before,
        "fonts must not change before confirmation"
    );

    ctrl.cancel_pending(); // C# "No": keep current fonts.
    assert!(!state.borrow().show_confirm_dialog);
    assert_eq!(
        state.borrow().fonts.as_bytes().to_vec(),
        fonts_before,
        "cancel must keep current fonts"
    );
}

#[test]
fn test_load_fonts_confirm_restores_embedded_fonts() {
    let dialogs = Rc::new(TestFileDialogs::new(vec![Some(fixture_path(
        "projects/default.atrview",
    ))]));
    let state = Rc::new(RefCell::new(GuiState::new()));
    let clipboard = Rc::new(RefCell::new(TestClipboard::new()));
    let ctrl = GuiController::new_with_io(
        state.clone(),
        slint::Weak::default(),
        dialogs.clone(),
        clipboard.clone(),
    );

    ctrl.open_project_from_path(&fixture_path("projects/default.atrview"));
    ctrl.confirm_pending(); // C# "Yes": load embedded fonts.

    assert_eq!(
        state.borrow().fonts.as_bytes(),
        state.borrow().project.font_banks.as_bytes()
    );
}

// =========================================================================
// REL-7 — Quit
// =========================================================================

#[test]
fn test_quit_cancel_keeps_application_running() {
    let (state, ctrl, _, _) = controller();

    ctrl.request_quit_confirmation();
    assert!(state.borrow().show_confirm_dialog);
    assert_eq!(
        state.borrow().pending_action,
        Some(afm_gui::PendingAction::Quit)
    );

    ctrl.cancel_pending();
    assert!(!state.borrow().show_confirm_dialog);
    assert_eq!(state.borrow().pending_action, None);
}

#[test]
fn test_quit_confirm_saves_configuration_without_panic() {
    let (state, ctrl, _, _) = controller();

    // The quit path writes `FontMaker.json` in the CWD; clean up any leftover.
    let config_path = std::path::Path::new("FontMaker.json");
    let existed = config_path.exists();
    let prior = existed.then(|| std::fs::read(config_path).ok()).flatten();

    ctrl.request_quit_confirmation();
    // Confirming quit saves the configuration (C# `ActionExitApplication`).
    // With a default (unconnected) window handle, hiding the window is a no-op
    // and must not panic.
    ctrl.confirm_pending();

    assert!(!state.borrow().show_confirm_dialog);
    assert_eq!(state.borrow().pending_action, None);

    // Restore the CWD configuration file to its prior state.
    match (existed, prior) {
        (true, Some(bytes)) => std::fs::write(config_path, bytes).unwrap(),
        (true, None) => {}
        (false, _) => {
            let _ = std::fs::remove_file(config_path);
        }
    }
}
