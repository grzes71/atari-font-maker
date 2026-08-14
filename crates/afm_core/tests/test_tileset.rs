use afm_core::tileset::{TileData, TileUndoBuffer};

#[test]
fn test_tile_transformations_and_rotations() {
    let mut tile = TileData::new();
    tile.set(0, 0, Some(0x41)); // 'A' at top-left
    tile.set(7, 7, Some(0x42)); // 'B' at bottom-right

    // Rotate 90 deg clockwise: (0,0) -> (7, 0), (7,7) -> (0, 7)
    tile.rotate_right();
    assert_eq!(tile.get(7, 0), Some(0x41));
    assert_eq!(tile.get(0, 7), Some(0x42));

    // Rotate 90 deg counter-clockwise: back to original
    tile.rotate_left();
    assert_eq!(tile.get(0, 0), Some(0x41));
    assert_eq!(tile.get(7, 7), Some(0x42));

    // Mirror horizontal
    tile.mirror_horizontal();
    assert_eq!(tile.get(7, 0), Some(0x41));
    assert_eq!(tile.get(0, 7), Some(0x42));

    // Mirror vertical
    tile.mirror_vertical();
    assert_eq!(tile.get(7, 7), Some(0x41));
    assert_eq!(tile.get(0, 0), Some(0x42));
}

#[test]
fn test_tile_shifts() {
    let mut tile = TileData::new();
    tile.set(0, 0, Some(0x10));

    // Shift left wraps (0, 0) -> (7, 0)
    tile.shift_left();
    assert_eq!(tile.get(7, 0), Some(0x10));
    assert_eq!(tile.get(0, 0), None);

    // Shift right wraps (7, 0) -> (0, 0)
    tile.shift_right();
    assert_eq!(tile.get(0, 0), Some(0x10));

    // Shift up wraps (0, 0) -> (0, 7)
    tile.shift_up();
    assert_eq!(tile.get(0, 7), Some(0x10));

    // Shift down wraps (0, 7) -> (0, 0)
    tile.shift_down();
    assert_eq!(tile.get(0, 0), Some(0x10));
}

#[test]
fn test_tile_undo_redo_history() {
    let mut undo = TileUndoBuffer::new();
    let mut tile = TileData::new();

    undo.push(tile.view);
    tile.set(0, 0, Some(0x99));

    // Undo -> restores None
    let undone = undo.undo(tile.view).unwrap();
    tile.view = undone;
    assert_eq!(tile.get(0, 0), None);

    // Redo -> restores 0x99
    let redone = undo.redo(tile.view).unwrap();
    tile.view = redone;
    assert_eq!(tile.get(0, 0), Some(0x99));
}
