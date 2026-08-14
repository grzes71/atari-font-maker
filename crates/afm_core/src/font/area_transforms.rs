//! Multi-character MegaCopy area pixel matrix transformations.

use crate::renderer::RenderColorMode;

/// 2D pixel matrix for arbitrary rectangular character selections (e.g. 2x2, 3x3, 4x4 characters).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelMatrix {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl PixelMatrix {
    /// Create a zero-initialized pixel matrix.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0u8; width * height],
        }
    }

    /// Return the horizontal pixel shift step (in 1-bit units) for the given color mode.
    pub fn pixel_step_for_mode(mode: RenderColorMode) -> usize {
        match mode {
            RenderColorMode::Mono => 1,
            RenderColorMode::Mode4 | RenderColorMode::Mode5 => 2,
            RenderColorMode::Mode10 => 4,
        }
    }

    /// Convert a linear slice of glyph bytes (8 bytes per character, row by row) into a 2D pixel matrix.
    pub fn from_glyph_bytes(glyphs: &[u8], width_chars: usize, height_chars: usize) -> Self {
        let pixel_width = width_chars * 8;
        let pixel_height = height_chars * 8;
        let mut matrix = Self::new(pixel_width, pixel_height);

        let mut src_idx = 0;
        for cy in 0..height_chars {
            let target_y = cy * 8;
            for cx in 0..width_chars {
                let target_x = cx * 8;
                for z in 0..8 {
                    let line = if src_idx < glyphs.len() {
                        glyphs[src_idx]
                    } else {
                        0
                    };
                    src_idx += 1;

                    let mut mask = 128u8;
                    for i in 0..8 {
                        let bit = if (line & mask) != 0 { 1 } else { 0 };
                        matrix.set(target_x + i, target_y + z, bit);
                        mask >>= 1;
                    }
                }
            }
        }

        matrix
    }

    /// Convert the 2D pixel matrix back into standard linear 8-byte glyph byte representations.
    pub fn to_glyph_bytes(&self, width_chars: usize, height_chars: usize) -> Vec<u8> {
        let total_chars = width_chars * height_chars;
        let mut glyphs = Vec::with_capacity(total_chars * 8);

        for cy in 0..height_chars {
            let src_y = cy * 8;
            for cx in 0..width_chars {
                let src_x = cx * 8;

                for in_y in 0..8 {
                    let mut accu = 0u8;
                    let mut mask = 128u8;
                    for px in 0..8 {
                        if self.get(src_x + px, src_y + in_y) > 0 {
                            accu |= mask;
                        }
                        mask >>= 1;
                    }
                    glyphs.push(accu);
                }
            }
        }

        glyphs
    }

    pub fn get(&self, x: usize, y: usize) -> u8 {
        if x < self.width && y < self.height {
            self.data[y * self.width + x]
        } else {
            0
        }
    }

    pub fn set(&mut self, x: usize, y: usize, val: u8) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = val;
        }
    }

    // Transformations

    /// Shift pixels to the left by `step` pixels with column wrap-around.
    pub fn shift_left(&mut self, step: usize) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        for _ in 0..step {
            let temp: Vec<u8> = (0..self.height).map(|y| self.get(0, y)).collect();

            for x in 0..self.width - 1 {
                for y in 0..self.height {
                    let next_val = self.get(x + 1, y);
                    self.set(x, y, next_val);
                }
            }

            for (y, &val) in temp.iter().enumerate() {
                self.set(self.width - 1, y, val);
            }
        }
    }

    /// Shift pixels to the right by `step` pixels with column wrap-around.
    pub fn shift_right(&mut self, step: usize) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        for _ in 0..step {
            let temp: Vec<u8> = (0..self.height)
                .map(|y| self.get(self.width - 1, y))
                .collect();

            for x in (1..self.width).rev() {
                for y in 0..self.height {
                    let prev_val = self.get(x - 1, y);
                    self.set(x, y, prev_val);
                }
            }

            for (y, &val) in temp.iter().enumerate() {
                self.set(0, y, val);
            }
        }
    }

    /// Shift pixels up by 1 pixel with row wrap-around.
    pub fn shift_up(&mut self) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let temp: Vec<u8> = (0..self.width).map(|x| self.get(x, 0)).collect();

        for y in 0..self.height - 1 {
            for x in 0..self.width {
                let next_val = self.get(x, y + 1);
                self.set(x, y, next_val);
            }
        }

        for (x, &val) in temp.iter().enumerate() {
            self.set(x, self.height - 1, val);
        }
    }

    /// Shift pixels down by 1 pixel with row wrap-around.
    pub fn shift_down(&mut self) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let temp: Vec<u8> = (0..self.width)
            .map(|x| self.get(x, self.height - 1))
            .collect();

        for y in (1..self.height).rev() {
            for x in 0..self.width {
                let prev_val = self.get(x, y - 1);
                self.set(x, y, prev_val);
            }
        }

        for (x, &val) in temp.iter().enumerate() {
            self.set(x, 0, val);
        }
    }

    /// Flip pixels horizontally preserving pixel bit alignment according to color mode.
    pub fn horizontal_mirror(&mut self, mode: RenderColorMode) {
        let mut target = self.clone();

        match mode {
            RenderColorMode::Mode4 | RenderColorMode::Mode5 => {
                // Two bits per pixel
                for y in 0..self.height {
                    let mut x = 0;
                    while x < self.width {
                        if self.width >= x + 2 {
                            let b0 = self.get(self.width - 2 - x, y);
                            let b1 = self.get(self.width - 1 - x, y);
                            target.set(x, y, b0);
                            target.set(x + 1, y, b1);
                        }
                        x += 2;
                    }
                }
            }
            RenderColorMode::Mode10 => {
                // Four bits per pixel
                for y in 0..self.height {
                    let mut x = 0;
                    while x < self.width {
                        if self.width >= x + 4 {
                            let b0 = self.get(self.width - 4 - x, y);
                            let b1 = self.get(self.width - 3 - x, y);
                            let b2 = self.get(self.width - 2 - x, y);
                            let b3 = self.get(self.width - 1 - x, y);
                            target.set(x, y, b0);
                            target.set(x + 1, y, b1);
                            target.set(x + 2, y, b2);
                            target.set(x + 3, y, b3);
                        }
                        x += 4;
                    }
                }
            }
            RenderColorMode::Mono => {
                // 1 bit per pixel
                for y in 0..self.height {
                    for x in 0..self.width {
                        target.set(x, y, self.get(self.width - 1 - x, y));
                    }
                }
            }
        }

        *self = target;
    }

    /// Flip pixels vertically along the horizontal axis.
    pub fn vertical_mirror(&mut self) {
        let mut target = self.clone();
        for y in 0..self.height {
            for x in 0..self.width {
                target.set(x, y, self.get(x, self.height - 1 - y));
            }
        }
        *self = target;
    }

    /// Invert bits in the pixel matrix (0 <-> 1).
    pub fn invert(&mut self) {
        for val in &mut self.data {
            *val = if *val == 0 { 1 } else { 0 };
        }
    }

    /// Rotate pixels 90 degrees counter-clockwise.
    pub fn rotate_left(&mut self, mode: RenderColorMode) {
        let mut target = self.clone();

        match mode {
            RenderColorMode::Mode4 | RenderColorMode::Mode5 => {
                if self.width == self.height * 2 {
                    for y in 0..self.height {
                        for x in 0..self.width / 2 {
                            let b0 = self.get(x * 2, y);
                            let b1 = self.get(x * 2 + 1, y);
                            target.set(y * 2, self.width / 2 - x - 1, b0);
                            target.set(y * 2 + 1, self.width / 2 - x - 1, b1);
                        }
                    }
                }
            }
            RenderColorMode::Mode10 => {
                if self.width == self.height * 4 {
                    for y in 0..self.height {
                        for x in 0..self.width / 4 {
                            let b0 = self.get(x * 4, y);
                            let b1 = self.get(x * 4 + 1, y);
                            let b2 = self.get(x * 4 + 2, y);
                            let b3 = self.get(x * 4 + 3, y);
                            target.set(y * 4, self.width / 4 - x - 1, b0);
                            target.set(y * 4 + 1, self.width / 4 - x - 1, b1);
                            target.set(y * 4 + 2, self.width / 4 - x - 1, b2);
                            target.set(y * 4 + 3, self.width / 4 - x - 1, b3);
                        }
                    }
                }
            }
            RenderColorMode::Mono => {
                if self.width == self.height {
                    for y in 0..self.height {
                        for x in 0..self.width {
                            target.set(y, self.width - 1 - x, self.get(x, y));
                        }
                    }
                }
            }
        }

        *self = target;
    }

    /// Rotate pixels 90 degrees clockwise.
    pub fn rotate_right(&mut self, mode: RenderColorMode) {
        let mut target = self.clone();

        match mode {
            RenderColorMode::Mode4 | RenderColorMode::Mode5 => {
                if self.width == self.height * 2 {
                    for y in 0..self.height {
                        for x in 0..self.width / 2 {
                            let b0 = self.get(x * 2, y);
                            let b1 = self.get(x * 2 + 1, y);
                            target.set((self.height - y) * 2 - 2, x, b0);
                            target.set((self.height - y) * 2 - 1, x, b1);
                        }
                    }
                }
            }
            RenderColorMode::Mode10 => {
                if self.width == self.height * 4 {
                    for y in 0..self.height {
                        for x in 0..self.width / 4 {
                            let b0 = self.get(x * 4, y);
                            let b1 = self.get(x * 4 + 1, y);
                            let b2 = self.get(x * 4 + 2, y);
                            let b3 = self.get(x * 4 + 3, y);
                            target.set((self.height - y) * 4 - 4, x, b0);
                            target.set((self.height - y) * 4 - 3, x, b1);
                            target.set((self.height - y) * 4 - 2, x, b2);
                            target.set((self.height - y) * 4 - 1, x, b3);
                        }
                    }
                }
            }
            RenderColorMode::Mono => {
                if self.width == self.height {
                    for y in 0..self.height {
                        for x in 0..self.width {
                            target.set(self.height - 1 - y, x, self.get(x, y));
                        }
                    }
                }
            }
        }

        *self = target;
    }
}
