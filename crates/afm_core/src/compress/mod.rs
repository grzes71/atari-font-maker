//! Data compression codecs (ZX0).

pub mod zx0;

pub use zx0::{Zx0Error, zx0_compress, zx0_decompress};
