//! 250-state circular undo/redo history for Atari font banks.

use crate::font::bank::FontBankSet;

/// Maximum number of undo states maintained in circular font history.
pub const FONT_UNDO_BUFFER_SIZE: usize = 250;

/// Font bank undo/redo manager matching legacy C# `AtariFontUndoBuffer`.
#[derive(Debug, Clone)]
pub struct FontUndoBuffer {
    buffer: Box<[[u8; 4096]; FONT_UNDO_BUFFER_SIZE + 1]>,
    flags: [i32; FONT_UNDO_BUFFER_SIZE + 1],
    index: usize,
}

impl Default for FontUndoBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl FontUndoBuffer {
    /// Create a new font undo buffer with empty flags.
    pub fn new() -> Self {
        let mut undo = Self {
            buffer: Box::new([[0u8; 4096]; FONT_UNDO_BUFFER_SIZE + 1]),
            flags: [-1; FONT_UNDO_BUFFER_SIZE + 1],
            index: 0,
        };
        undo.setup();
        undo
    }

    /// Reset all undo flags and rewind index to 0.
    pub fn setup(&mut self) {
        self.index = 0;
        self.flags.fill(-1);
    }

    /// Current cursor index in the circular buffer.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Read-only view of the flag array.
    pub fn flags(&self) -> &[i32] {
        &self.flags
    }

    /// Record initial baseline state of the font buffer without advancing index.
    pub fn add_to_undo_initial(&mut self, fonts: &FontBankSet) {
        let prev_index = self.index;
        self.buffer[self.index].copy_from_slice(fonts.as_bytes());
        self.flags[self.index] = self.flags[prev_index] + 1;
        self.flags[(self.index + 1) % FONT_UNDO_BUFFER_SIZE] = -1; // Disallow redo after state change
    }

    /// Advance circular index and record new font state if `difference` is true.
    pub fn add_to_undo(&mut self, fonts: &FontBankSet, difference: bool) {
        if difference {
            let prev_index = self.index;
            self.index = (self.index + 1) % FONT_UNDO_BUFFER_SIZE;
            self.buffer[self.index].copy_from_slice(fonts.as_bytes());
            self.flags[self.index] = self.flags[prev_index] + 1;
            self.flags[(self.index + 1) % FONT_UNDO_BUFFER_SIZE] = -1; // Disallow redo after state change
        }
    }

    /// Check if current font bytes differ from current undo state, and record if changed.
    pub fn add_to_undo_full_difference_scan(&mut self, fonts: &FontBankSet) -> bool {
        let current_bytes = fonts.as_bytes();
        let last_undo_bytes = &self.buffer[self.index];

        if current_bytes != last_undo_bytes {
            self.add_to_undo(fonts, true);
            true
        } else {
            false
        }
    }

    /// Calculate previous circular undo index.
    pub fn get_prev_undo_index(&self) -> usize {
        if self.index == 0 {
            FONT_UNDO_BUFFER_SIZE - 1
        } else {
            self.index - 1
        }
    }

    /// Restore previous font bank state and decrement circular cursor.
    pub fn undo(&mut self, fonts: &mut FontBankSet) {
        let prev_index = self.get_prev_undo_index();
        fonts
            .as_bytes_mut()
            .copy_from_slice(&self.buffer[prev_index]);
        self.index = prev_index;
    }

    /// Restore next font bank state and increment circular cursor if redo is valid.
    pub fn redo(&mut self, fonts: &mut FontBankSet) {
        let next_index = (self.index + 1) % FONT_UNDO_BUFFER_SIZE;

        if self.flags[next_index] > -1 {
            fonts
                .as_bytes_mut()
                .copy_from_slice(&self.buffer[next_index]);
        }

        self.index = next_index;
    }

    /// Returns `(redo_button_enabled, undo_button_enabled)`.
    pub fn get_redo_undo_button_state(&self, edited: bool) -> (bool, bool) {
        let next_index = (self.index + 1) % FONT_UNDO_BUFFER_SIZE;
        let prev_index = self.get_prev_undo_index();

        let redo_enabled = self.flags[next_index] != -1 && !edited;
        let undo_enabled = edited
            || (self.flags[self.index] > self.flags[prev_index] && self.flags[prev_index] > -1);

        (redo_enabled, undo_enabled)
    }
}
