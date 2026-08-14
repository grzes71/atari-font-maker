use afm_core::font::PixelMatrix;
use afm_core::renderer::RenderColorMode;

#[test]
fn test_pixel_matrix_roundtrip_and_shifts() {
    // 2x2 characters = 16x16 pixels
    let mut glyphs = vec![0u8; 4 * 8];
    glyphs[0] = 0x80; // (0, 0) pixel is set in first char

    let mut matrix = PixelMatrix::from_glyph_bytes(&glyphs, 2, 2);
    assert_eq!(matrix.get(0, 0), 1);
    assert_eq!(matrix.get(1, 0), 0);

    // Shift left by 1 pixel -> (0, 0) wraps to (15, 0)
    matrix.shift_left(1);
    assert_eq!(matrix.get(15, 0), 1);
    assert_eq!(matrix.get(0, 0), 0);

    // Shift right by 1 pixel -> (15, 0) wraps back to (0, 0)
    matrix.shift_right(1);
    assert_eq!(matrix.get(0, 0), 1);

    // Shift up -> (0, 0) wraps to (0, 15)
    matrix.shift_up();
    assert_eq!(matrix.get(0, 15), 1);

    // Shift down -> (0, 15) wraps back to (0, 0)
    matrix.shift_down();
    assert_eq!(matrix.get(0, 0), 1);

    let roundtrip_glyphs = matrix.to_glyph_bytes(2, 2);
    assert_eq!(roundtrip_glyphs, glyphs);
}

#[test]
fn test_pixel_matrix_mirror_invert_rotate() {
    // 2x2 chars = 16x16 pixels
    let mut matrix = PixelMatrix::new(16, 16);
    matrix.set(0, 0, 1);

    // Horizontal mirror -> (0, 0) to (15, 0)
    matrix.horizontal_mirror(RenderColorMode::Mono);
    assert_eq!(matrix.get(15, 0), 1);
    assert_eq!(matrix.get(0, 0), 0);

    // Vertical mirror -> (15, 0) to (15, 15)
    matrix.vertical_mirror();
    assert_eq!(matrix.get(15, 15), 1);
    assert_eq!(matrix.get(15, 0), 0);

    // Invert: 1 becomes 0, all 0s become 1s
    matrix.invert();
    assert_eq!(matrix.get(15, 15), 0);
    assert_eq!(matrix.get(0, 0), 1);
    matrix.invert(); // revert

    // Rotate 90 deg clockwise: (15, 15) -> (0, 15)
    matrix.rotate_right(RenderColorMode::Mono);
    assert_eq!(matrix.get(0, 15), 1);
}
