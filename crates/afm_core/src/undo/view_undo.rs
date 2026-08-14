//! Undo/redo history for Atari View screens and pages.

use std::collections::VecDeque;

/// Maximum number of undo states maintained in view history.
pub const VIEW_UNDO_BUFFER_SIZE: usize = 250;

/// Snapshot of an Atari View screen state for undo/redo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewUndoState {
    pub view_bytes: Vec<u8>,
    pub use_font_on_line: Vec<u8>,
}

impl ViewUndoState {
    pub fn new(view_bytes: Vec<u8>, use_font_on_line: Vec<u8>) -> Self {
        Self {
            view_bytes,
            use_font_on_line,
        }
    }
}

/// View screen undo/redo manager matching legacy C# `AtariViewUndoBuffer`.
#[derive(Debug, Clone, Default)]
pub struct ViewUndoBuffer {
    undo_commands: VecDeque<ViewUndoState>,
    redo_commands: Vec<ViewUndoState>,
}

impl ViewUndoBuffer {
    /// Create a new empty view undo buffer.
    pub fn new() -> Self {
        Self {
            undo_commands: VecDeque::with_capacity(VIEW_UNDO_BUFFER_SIZE),
            redo_commands: Vec::new(),
        }
    }

    /// Push current view state into undo history, discarding oldest if limit is reached.
    pub fn push(&mut self, state: ViewUndoState) {
        while self.undo_commands.len() >= VIEW_UNDO_BUFFER_SIZE {
            self.undo_commands.pop_front();
        }
        self.undo_commands.push_back(state);
        self.redo_commands.clear();
    }

    /// Undo: saves `current_state` to redo stack and returns previous state from undo history.
    pub fn undo(&mut self, current_state: ViewUndoState) -> Option<ViewUndoState> {
        if self.undo_commands.is_empty() {
            return None;
        }

        self.redo_commands.push(current_state);
        self.undo_commands.pop_back()
    }

    /// Redo: saves `current_state` to undo history and restores next state from redo stack.
    pub fn redo(&mut self, current_state: ViewUndoState) -> Option<ViewUndoState> {
        if self.redo_commands.is_empty() {
            return None;
        }

        while self.undo_commands.len() >= VIEW_UNDO_BUFFER_SIZE {
            self.undo_commands.pop_front();
        }
        self.undo_commands.push_back(current_state);
        self.redo_commands.pop()
    }

    /// Returns `(undo_available, redo_available)`.
    pub fn get_redo_undo_button_state(&self) -> (bool, bool) {
        (
            !self.undo_commands.is_empty(),
            !self.redo_commands.is_empty(),
        )
    }

    /// Number of undo states currently stored.
    pub fn undo_count(&self) -> usize {
        self.undo_commands.len()
    }

    /// Number of redo states currently stored.
    pub fn redo_count(&self) -> usize {
        self.redo_commands.len()
    }
}
