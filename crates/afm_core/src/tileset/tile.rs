//! Domain model for 8x8 character tiles, geometric transformations and tile history.

use crate::codecs::atrview::SavedTileData;
use std::collections::VecDeque;

pub const TILE_WIDTH: usize = 8;
pub const TILE_HEIGHT: usize = 8;
pub const TILE_CELLS: usize = TILE_WIDTH * TILE_HEIGHT;
pub const NUM_TILES_IN_SET: usize = 256;
pub const TILE_UNDO_BUFFER_SIZE: usize = 250;

/// An 8x8 matrix of optional character codes with line font assignments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileData {
    pub view: [Option<u8>; TILE_CELLS],
    pub selected_font: [u8; TILE_HEIGHT],
}

impl Default for TileData {
    fn default() -> Self {
        Self::new()
    }
}

impl TileData {
    /// Create a new empty tile with all cells unset and font 1 on all lines.
    pub const fn new() -> Self {
        Self {
            view: [None; TILE_CELLS],
            selected_font: [1; TILE_HEIGHT],
        }
    }

    /// Check if at least one cell in the tile has a character assigned.
    pub fn is_valid(&self) -> bool {
        self.view.iter().any(|cell| cell.is_some())
    }

    /// Get cell character value at (x, y).
    pub fn get(&self, x: usize, y: usize) -> Option<u8> {
        if x < TILE_WIDTH && y < TILE_HEIGHT {
            self.view[y * TILE_WIDTH + x]
        } else {
            None
        }
    }

    /// Set cell character value at (x, y).
    pub fn set(&mut self, x: usize, y: usize, val: Option<u8>) {
        if x < TILE_WIDTH && y < TILE_HEIGHT {
            self.view[y * TILE_WIDTH + x] = val;
        }
    }

    /// Load tile from serialized SavedTileData.
    pub fn load_saved(&mut self, data: &SavedTileData) {
        let width = if data.width == 0 { 5 } else { data.width };
        let height = if data.height == 0 { 5 } else { data.height };

        let view_bytes = hex::decode(&data.view).unwrap_or_default();
        let null_chars: Vec<char> = data.nulls.chars().collect();

        self.view.fill(None);

        let mut idx = 0;
        for y in 0..height.min(TILE_HEIGHT) {
            for x in 0..width.min(TILE_WIDTH) {
                if idx < view_bytes.len() {
                    let is_null = null_chars.get(idx).copied().unwrap_or('1') != '0';
                    if is_null {
                        self.set(x, y, None);
                    } else {
                        self.set(x, y, Some(view_bytes[idx]));
                    }
                }
                idx += 1;
            }
        }

        if let Ok(font_bytes) = hex::decode(&data.font) {
            for (i, &f) in font_bytes.iter().take(TILE_HEIGHT).enumerate() {
                self.selected_font[i] = f;
            }
        }
    }

    /// Save tile to serialized SavedTileData.
    pub fn to_saved(&self, tile_nr: usize) -> Option<SavedTileData> {
        let mut num_nulls = 0;
        let mut view_hex = String::with_capacity(TILE_CELLS * 2);
        let mut nulls_str = String::with_capacity(TILE_CELLS);

        for y in 0..TILE_HEIGHT {
            for x in 0..TILE_WIDTH {
                match self.get(x, y) {
                    None => {
                        nulls_str.push('1');
                        view_hex.push_str("00");
                        num_nulls += 1;
                    }
                    Some(ch) => {
                        nulls_str.push('0');
                        view_hex.push_str(&format!("{ch:02X}"));
                    }
                }
            }
        }

        if num_nulls == TILE_CELLS {
            return None;
        }

        let font_hex = hex::encode(self.selected_font);

        Some(SavedTileData {
            nr: tile_nr,
            view: view_hex,
            font: font_hex,
            nulls: nulls_str,
            width: TILE_WIDTH,
            height: TILE_HEIGHT,
        })
    }

    // Transformations

    /// Rotate tile 90 degrees clockwise.
    pub fn rotate_right(&mut self) {
        let mut work = [None; TILE_CELLS];
        for y in 0..TILE_HEIGHT {
            for x in 0..TILE_WIDTH {
                work[y * TILE_WIDTH + x] = self.get(y, TILE_WIDTH - x - 1);
            }
        }
        self.view = work;
    }

    /// Rotate tile 90 degrees counter-clockwise.
    pub fn rotate_left(&mut self) {
        let mut work = [None; TILE_CELLS];
        for y in 0..TILE_HEIGHT {
            for x in 0..TILE_WIDTH {
                work[y * TILE_WIDTH + x] = self.get(TILE_HEIGHT - y - 1, x);
            }
        }
        self.view = work;
    }

    /// Flip tile horizontally along vertical axis.
    pub fn mirror_horizontal(&mut self) {
        for y in 0..TILE_HEIGHT {
            for x in 0..TILE_WIDTH / 2 {
                let left_idx = y * TILE_WIDTH + x;
                let right_idx = y * TILE_WIDTH + (TILE_WIDTH - x - 1);
                self.view.swap(left_idx, right_idx);
            }
        }
    }

    /// Flip tile vertically along horizontal axis.
    pub fn mirror_vertical(&mut self) {
        for x in 0..TILE_WIDTH {
            for y in 0..TILE_HEIGHT / 2 {
                let top_idx = y * TILE_WIDTH + x;
                let bottom_idx = (TILE_HEIGHT - y - 1) * TILE_WIDTH + x;
                self.view.swap(top_idx, bottom_idx);
            }
        }
    }

    /// Shift tile rows left by 1 cell with wrap-around.
    pub fn shift_left(&mut self) {
        for y in 0..TILE_HEIGHT {
            let leftmost = self.view[y * TILE_WIDTH];
            for x in 0..TILE_WIDTH - 1 {
                self.view[y * TILE_WIDTH + x] = self.view[y * TILE_WIDTH + x + 1];
            }
            self.view[y * TILE_WIDTH + TILE_WIDTH - 1] = leftmost;
        }
    }

    /// Shift tile rows right by 1 cell with wrap-around.
    pub fn shift_right(&mut self) {
        for y in 0..TILE_HEIGHT {
            let rightmost = self.view[y * TILE_WIDTH + TILE_WIDTH - 1];
            for x in (1..TILE_WIDTH).rev() {
                self.view[y * TILE_WIDTH + x] = self.view[y * TILE_WIDTH + x - 1];
            }
            self.view[y * TILE_WIDTH] = rightmost;
        }
    }

    /// Shift tile columns up by 1 cell with wrap-around.
    pub fn shift_up(&mut self) {
        for x in 0..TILE_WIDTH {
            let topmost = self.view[x];
            for y in 0..TILE_HEIGHT - 1 {
                self.view[y * TILE_WIDTH + x] = self.view[(y + 1) * TILE_WIDTH + x];
            }
            self.view[(TILE_HEIGHT - 1) * TILE_WIDTH + x] = topmost;
        }
    }

    /// Shift tile columns down by 1 cell with wrap-around.
    pub fn shift_down(&mut self) {
        for x in 0..TILE_WIDTH {
            let bottommost = self.view[(TILE_HEIGHT - 1) * TILE_WIDTH + x];
            for y in (1..TILE_HEIGHT).rev() {
                self.view[y * TILE_WIDTH + x] = self.view[(y - 1) * TILE_WIDTH + x];
            }
            self.view[x] = bottommost;
        }
    }
}

/// Undo/Redo history manager for TileData matrices.
#[derive(Debug, Clone, Default)]
pub struct TileUndoBuffer {
    undo_commands: VecDeque<[Option<u8>; TILE_CELLS]>,
    redo_commands: Vec<[Option<u8>; TILE_CELLS]>,
}

impl TileUndoBuffer {
    pub fn new() -> Self {
        Self {
            undo_commands: VecDeque::with_capacity(TILE_UNDO_BUFFER_SIZE),
            redo_commands: Vec::new(),
        }
    }

    pub fn push(&mut self, state: [Option<u8>; TILE_CELLS]) {
        while self.undo_commands.len() >= TILE_UNDO_BUFFER_SIZE {
            self.undo_commands.pop_front();
        }
        self.undo_commands.push_back(state);
        self.redo_commands.clear();
    }

    pub fn undo(&mut self, current: [Option<u8>; TILE_CELLS]) -> Option<[Option<u8>; TILE_CELLS]> {
        if self.undo_commands.is_empty() {
            return None;
        }
        self.redo_commands.push(current);
        self.undo_commands.pop_back()
    }

    pub fn redo(&mut self, current: [Option<u8>; TILE_CELLS]) -> Option<[Option<u8>; TILE_CELLS]> {
        if self.redo_commands.is_empty() {
            return None;
        }
        while self.undo_commands.len() >= TILE_UNDO_BUFFER_SIZE {
            self.undo_commands.pop_front();
        }
        self.undo_commands.push_back(current);
        self.redo_commands.pop()
    }

    pub fn get_redo_undo_button_state(&self) -> (bool, bool) {
        (
            !self.undo_commands.is_empty(),
            !self.redo_commands.is_empty(),
        )
    }
}

/// A set of 256 tiles with a cursor for current selection.
#[derive(Debug, Clone)]
pub struct TileSet {
    pub tiles: Vec<TileData>,
    pub current_index: usize,
}

impl Default for TileSet {
    fn default() -> Self {
        Self::new()
    }
}

impl TileSet {
    pub fn new() -> Self {
        Self {
            tiles: vec![TileData::new(); NUM_TILES_IN_SET],
            current_index: 0,
        }
    }

    pub fn current_tile(&self) -> &TileData {
        &self.tiles[self.current_index % NUM_TILES_IN_SET]
    }

    pub fn current_tile_mut(&mut self) -> &mut TileData {
        let idx = self.current_index % NUM_TILES_IN_SET;
        &mut self.tiles[idx]
    }
}
