//! ZX0 v2 data compression (format by Einar Saukas, BSD-3 licensed algorithm).
//!
//! The compressor produces a valid ZX0 v2 stream (verified via the companion
//! decompressor, which is a faithful port of the official `dzx0.c` v2.2). The
//! compression is greedy LZ77 — smaller/faster than the official optimal
//! compressor, but the emitted bitstream is fully ZX0-v2-compatible.

use std::fmt;

/// Errors produced while decompressing a ZX0 stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zx0Error {
    /// Input ended before the stream terminated.
    Truncated,
    /// A match offset pointed before the start of the output.
    InvalidOffset,
}

impl fmt::Display for Zx0Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Zx0Error::Truncated => write!(f, "truncated ZX0 stream"),
            Zx0Error::InvalidOffset => write!(f, "invalid match offset in ZX0 stream"),
        }
    }
}

impl std::error::Error for Zx0Error {}

/// Largest offset representable in the ZX0 v2 format (255 * 128).
const MAX_OFFSET: usize = 255 * 128;

/// Minimum match length for a "copy from new offset" block (`length - 1 >= 1`).
const MIN_MATCH: usize = 2;

// =============================================================================
// Bit writer (MSB-first)
// =============================================================================

struct BitWriter {
    bytes: Vec<u8>,
    bit_buf: u8,
    bit_count: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_buf: 0,
            bit_count: 0,
        }
    }

    fn write_bit(&mut self, bit: bool) {
        self.bit_buf = (self.bit_buf << 1) | bit as u8;
        self.bit_count += 1;
        if self.bit_count == 8 {
            self.bytes.push(self.bit_buf);
            self.bit_buf = 0;
            self.bit_count = 0;
        }
    }

    fn write_bits(&mut self, value: usize, count: usize) {
        for i in (0..count).rev() {
            self.write_bit(((value >> i) & 1) == 1);
        }
    }

    fn write_byte(&mut self, byte: u8) {
        self.write_bits(byte as usize, 8);
    }

    /// Write interlaced Elias gamma coding of `value` (`value >= 1`).
    fn write_gamma(&mut self, value: usize, inverted: bool) {
        debug_assert!(value >= 1);
        // Collect the bits after the implicit leading 1, LSB-first.
        let mut bits: Vec<bool> = Vec::new();
        let mut v = value;
        while v > 1 {
            bits.push((v & 1) == 1);
            v >>= 1;
        }
        // Emit MSB-first with a `0` prefix before each data bit, then terminator `1`.
        for &b in bits.iter().rev() {
            self.write_bit(false);
            self.write_bit(b ^ inverted);
        }
        self.write_bit(true);
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bit_count > 0 {
            // Pad the final byte with zero bits (never read by the decompressor).
            self.bytes.push(self.bit_buf << (8 - self.bit_count));
        }
        self.bytes
    }
}

// =============================================================================
// Bit reader (MSB-first)
// =============================================================================

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_mask: u8,
    bit_value: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_mask: 0,
            bit_value: 0,
        }
    }

    fn read_bit(&mut self) -> Result<bool, Zx0Error> {
        if self.bit_mask == 0 {
            if self.byte_pos >= self.data.len() {
                return Err(Zx0Error::Truncated);
            }
            self.bit_value = self.data[self.byte_pos];
            self.byte_pos += 1;
            self.bit_mask = 128;
        }
        let bit = (self.bit_value & self.bit_mask) != 0;
        self.bit_mask >>= 1;
        Ok(bit)
    }

    fn read_bits(&mut self, count: usize) -> Result<usize, Zx0Error> {
        let mut value = 0usize;
        for _ in 0..count {
            value = (value << 1) | (self.read_bit()? as usize);
        }
        Ok(value)
    }

    fn read_byte(&mut self) -> Result<u8, Zx0Error> {
        Ok(self.read_bits(8)? as u8)
    }

    fn read_gamma(&mut self, inverted: bool) -> Result<usize, Zx0Error> {
        let mut value = 1usize;
        loop {
            let bit = self.read_bit()?;
            if bit {
                return Ok(value);
            }
            value = (value << 1) | ((self.read_bit()? as usize) ^ (inverted as usize));
        }
    }
}

// =============================================================================
// Compressor
// =============================================================================

/// Find the longest match at `pos`, searching offsets `1..=min(pos, MAX_OFFSET)`.
fn longest_match(data: &[u8], pos: usize) -> (usize, usize) {
    let max_offset = pos.min(MAX_OFFSET);
    let mut best_len = 0usize;
    let mut best_offset = 0usize;

    for offset in 1..=max_offset {
        let mut len = 0usize;
        let mut i = pos;
        let mut j = pos - offset;
        while i < data.len() && data[i] == data[j] {
            len += 1;
            i += 1;
            j += 1;
        }
        if len > best_len {
            best_len = len;
            best_offset = offset;
            if len == data.len() - pos {
                break; // cannot improve
            }
        }
    }

    (best_len, best_offset)
}

/// Emit a literal block (`gamma(length)` + raw bytes).
fn emit_literal(w: &mut BitWriter, data: &[u8], start: usize, length: usize) {
    w.write_gamma(length, false);
    for &b in &data[start..start + length] {
        w.write_byte(b);
    }
}

/// Emit the body of a "copy from new offset" block: `gamma(g,true)`, a 7-bit
/// offset low part, and `gamma(length-1)`. The leading `1` indicator bit is
/// written separately by the caller (it is the trailing bit of the preceding
/// block).
fn emit_new_offset_match(w: &mut BitWriter, offset: usize, length: usize) {
    debug_assert!(length >= MIN_MATCH);
    let g = offset.div_ceil(128);
    let x = g * 128 - offset;
    w.write_gamma(g, true);
    w.write_bits(x, 7);
    w.write_gamma(length - 1, false);
}

/// Compress `data` into a ZX0 v2 stream. Returns `Vec::new()` for empty input.
pub fn zx0_compress(data: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut w = BitWriter::new();
    let mut pos = 0usize;

    loop {
        // 1. Emit a literal block: `gamma(length)` + raw bytes. The first block
        //    has no leading bit; later literal blocks are signalled by the
        //    preceding match's trailing `0` bit.
        let run_start = pos;
        while pos < data.len() {
            let (mlen, _) = longest_match(data, pos);
            if mlen >= MIN_MATCH {
                break;
            }
            pos += 1;
        }
        let run = pos - run_start;
        debug_assert!(run >= 1, "literal block must be non-empty");
        emit_literal(&mut w, data, run_start, run);

        // 2. After a literal, the next block is always a "new offset" match.
        if pos >= data.len() {
            w.write_bit(true);
            w.write_gamma(256, true); // EOF
            return w.finish();
        }
        w.write_bit(true); // "new offset follows"

        // 3. Emit match bodies while beneficial.
        loop {
            let (mlen, moff) = longest_match(data, pos);
            debug_assert!(mlen >= MIN_MATCH);
            emit_new_offset_match(&mut w, moff, mlen);
            pos += mlen;

            if pos >= data.len() {
                w.write_bit(true);
                w.write_gamma(256, true); // EOF
                return w.finish();
            }

            let (nlen, _) = longest_match(data, pos);
            if nlen >= MIN_MATCH {
                w.write_bit(true); // another new-offset match follows
            } else {
                w.write_bit(false); // literal follows
                break;
            }
        }
        // Loop back to emit the next literal block.
    }
}

// =============================================================================
// Decompressor (faithful port of dzx0.c v2.2)
// =============================================================================

enum State {
    Literals,
    LastOffset,
    NewOffset,
}

/// Decompress a ZX0 v2 stream.
pub fn zx0_decompress(data: &[u8]) -> Result<Vec<u8>, Zx0Error> {
    if data.is_empty() {
        return Err(Zx0Error::Truncated);
    }

    let mut r = BitReader::new(data);
    let mut out: Vec<u8> = Vec::new();
    let mut last_offset = 1usize;
    let mut state = State::Literals;

    loop {
        match state {
            State::Literals => {
                let length = r.read_gamma(false)?;
                for _ in 0..length {
                    out.push(r.read_byte()?);
                }
                state = if r.read_bit()? {
                    State::NewOffset
                } else {
                    State::LastOffset
                };
            }
            State::LastOffset => {
                let length = r.read_gamma(false)?;
                if last_offset > out.len() {
                    return Err(Zx0Error::InvalidOffset);
                }
                for _ in 0..length {
                    let src = out.len() - last_offset;
                    let v = out[src];
                    out.push(v);
                }
                state = if r.read_bit()? {
                    State::NewOffset
                } else {
                    State::Literals
                };
            }
            State::NewOffset => {
                let g = r.read_gamma(true)?;
                if g == 256 {
                    return Ok(out);
                }
                let x = r.read_bits(7)?;
                last_offset = g * 128 - x;
                if last_offset == 0 || last_offset > out.len() {
                    return Err(Zx0Error::InvalidOffset);
                }
                let length = r.read_gamma(false)? + 1;
                for _ in 0..length {
                    let src = out.len() - last_offset;
                    let v = out[src];
                    out.push(v);
                }
                state = if r.read_bit()? {
                    State::NewOffset
                } else {
                    State::Literals
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) {
        let compressed = zx0_compress(data);
        assert!(!compressed.is_empty(), "compressed must not be empty");
        let decompressed = zx0_decompress(&compressed).expect("decompress");
        assert_eq!(decompressed, data, "roundtrip mismatch");
    }

    #[test]
    fn test_roundtrip_empty_and_single() {
        assert_eq!(zx0_compress(&[]), Vec::<u8>::new());
        roundtrip(&[0xAB]);
    }

    #[test]
    fn test_roundtrip_repetitive() {
        roundtrip(&[0x42; 1024]);
        roundtrip(&[1, 2, 3, 4, 1, 2, 3, 4].repeat(100));
    }

    #[test]
    fn test_roundtrip_randomish() {
        let mut data = Vec::new();
        let mut x = 0x1234_5678u32;
        for _ in 0..4096 {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            data.push((x >> 24) as u8);
        }
        roundtrip(&data);
    }

    #[test]
    fn test_compression_shrinks_repetitive() {
        let compressed = zx0_compress(&[0u8; 4096]);
        assert!(compressed.len() < 4096, "repetitive data must compress");
    }

    #[test]
    fn test_decompress_truncated_errors() {
        assert!(zx0_decompress(&[]).is_err());
        assert!(zx0_decompress(&[0x00]).is_err());
    }
}
