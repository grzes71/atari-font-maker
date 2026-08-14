use std::fs;
use std::path::Path;

use afm_core::font::bank::FontBankSet;
use afm_core::palette::Palette;
use afm_core::renderer::{
    ATLAS_BUFFER_SIZE, ATLAS_HEIGHT, ATLAS_STRIDE, ATLAS_WIDTH, FontAtlasBuffer, FontRenderer,
    RenderColorMode,
};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

fn create_standard_test_setup() -> (FontBankSet, FontRenderer) {
    let default_fnt_bytes =
        fs::read(fixture_path("projects/Default.fnt")).expect("Read Default.fnt");
    let mut fonts = FontBankSet::new();
    for bank_idx in 0..4 {
        fonts.copy_to(&default_fnt_bytes, 0, bank_idx * 1024, 1024);
    }

    let pal_bytes = fs::read(fixture_path("palette/altirraPAL.pal")).expect("Read altirraPAL.pal");
    let mut pal_arr = [0u8; 768];
    pal_arr.copy_from_slice(&pal_bytes);
    let palette = Palette::from_bytes(&pal_arr);

    let selected_colors = [0x00, 0x28, 0xCA, 0x46, 0x98, 0x1A, 0x76, 0x54, 0x32, 0x00];
    let renderer = FontRenderer::new(palette, selected_colors);

    (fonts, renderer)
}

#[test]
fn test_renderer_dimensions_and_constants() {
    assert_eq!(ATLAS_WIDTH, 512);
    assert_eq!(ATLAS_HEIGHT, 1024);
    assert_eq!(ATLAS_STRIDE, 2048);
    assert_eq!(ATLAS_BUFFER_SIZE, 2097152);

    let buffer = FontAtlasBuffer::new();
    assert_eq!(buffer.as_bytes().len(), ATLAS_BUFFER_SIZE);
}

#[test]
fn test_renderer_mono_atlas_golden() {
    let (fonts, renderer) = create_standard_test_setup();
    let mut buffer = FontAtlasBuffer::new();

    renderer.render_all_fonts(&fonts, RenderColorMode::Mono, &mut buffer);

    let golden_raw =
        fs::read(fixture_path("renders/font_atlas_mono.raw")).expect("Read font_atlas_mono.raw");
    assert_eq!(golden_raw.len(), ATLAS_BUFFER_SIZE);

    assert_eq!(
        buffer.as_bytes(),
        golden_raw.as_slice(),
        "Mono font atlas did not match golden master font_atlas_mono.raw byte-for-byte!"
    );
}

#[test]
fn test_renderer_mode4_atlas_golden() {
    let (fonts, renderer) = create_standard_test_setup();
    let mut buffer = FontAtlasBuffer::new();

    renderer.render_all_fonts(&fonts, RenderColorMode::Mode4, &mut buffer);

    let golden_raw =
        fs::read(fixture_path("renders/font_atlas_mode4.raw")).expect("Read font_atlas_mode4.raw");
    assert_eq!(golden_raw.len(), ATLAS_BUFFER_SIZE);

    assert_eq!(
        buffer.as_bytes(),
        golden_raw.as_slice(),
        "Mode 4 font atlas did not match golden master font_atlas_mode4.raw byte-for-byte!"
    );
}

#[test]
fn test_renderer_mode10_atlas_golden() {
    let (fonts, renderer) = create_standard_test_setup();
    let mut buffer = FontAtlasBuffer::new();

    renderer.render_all_fonts(&fonts, RenderColorMode::Mode10, &mut buffer);

    let golden_raw = fs::read(fixture_path("renders/font_atlas_mode10.raw"))
        .expect("Read font_atlas_mode10.raw");
    assert_eq!(golden_raw.len(), ATLAS_BUFFER_SIZE);

    assert_eq!(
        buffer.as_bytes(),
        golden_raw.as_slice(),
        "Mode 10 font atlas did not match golden master font_atlas_mode10.raw byte-for-byte!"
    );
}

#[test]
fn test_render_one_character_parity_mode4() {
    let (fonts, renderer) = create_standard_test_setup();
    let mut full_buffer = FontAtlasBuffer::new();
    renderer.render_all_fonts(&fonts, RenderColorMode::Mode4, &mut full_buffer);

    let mut incremental_buffer = FontAtlasBuffer::new();

    // Render all characters across bank1 and bank2 grid using render_one_character
    for char_idx in 0..512 {
        renderer.render_one_character(
            &fonts,
            RenderColorMode::Mode4,
            char_idx,
            false,
            &mut incremental_buffer,
        );
        renderer.render_one_character(
            &fonts,
            RenderColorMode::Mode4,
            char_idx,
            true,
            &mut incremental_buffer,
        );
    }

    assert_eq!(
        incremental_buffer.as_bytes(),
        full_buffer.as_bytes(),
        "Incremental render_one_character produced different output than render_all_fonts in Mode 4!"
    );
}

#[test]
fn test_render_one_character_parity_mode10() {
    let (fonts, renderer) = create_standard_test_setup();
    let mut full_buffer = FontAtlasBuffer::new();
    renderer.render_all_fonts(&fonts, RenderColorMode::Mode10, &mut full_buffer);

    let mut incremental_buffer = FontAtlasBuffer::new();

    // Render all characters across bank1 and bank2 grid using render_one_character
    for char_idx in 0..512 {
        renderer.render_one_character(
            &fonts,
            RenderColorMode::Mode10,
            char_idx,
            false,
            &mut incremental_buffer,
        );
        renderer.render_one_character(
            &fonts,
            RenderColorMode::Mode10,
            char_idx,
            true,
            &mut incremental_buffer,
        );
    }

    assert_eq!(
        incremental_buffer.as_bytes(),
        full_buffer.as_bytes(),
        "Incremental render_one_character produced different output than render_all_fonts in Mode 10!"
    );
}
