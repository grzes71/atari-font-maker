//! Atari Font rasterization and atlas rendering.

pub mod buffer;
pub mod engine;

pub use buffer::{
    ATLAS_BUFFER_SIZE, ATLAS_HEIGHT, ATLAS_STRIDE, ATLAS_WIDTH, BYTES_PER_PIXEL, FontAtlasBuffer,
};
pub use engine::{FontRenderer, RenderColorMode};
