//! TileSet domain model, 8x8 character tiles and transformations.

pub mod tile;

pub use tile::{
    NUM_TILES_IN_SET, TILE_CELLS, TILE_HEIGHT, TILE_UNDO_BUFFER_SIZE, TILE_WIDTH, TileData,
    TileSet, TileUndoBuffer,
};
