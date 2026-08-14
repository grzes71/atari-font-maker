# Specyfikacja Transformacji Obszarowych MegaCopy (afm_core::font::area_transforms)

> **Dokument**: Specyfikacja Techniczna Wieloznakowych Transformacji Pikselowych  
> **Faza**: Phase 10a — Core Domain Extensions  
> **Data**: 2026-08-14  
> **Źródła C#**: `CharacterEditor.cs` (`ExecuteCopyAreaShift*`, `ExecuteCopyAreaMirror*`, `ExecuteCopyAreaRotate*`, `ExecuteCopyAreaInvert`)

---

## 1. Zakres Odpowiedzialności

Operacje MegaCopy pozwalają na zaznaczenie dowolnego prostokątnego bloku znaków (np. 2×2, 3×3, 4×4) i wykonanie transformacji geometrycznych na całym połączonym rastrze pikseli (`pixel_width = width_chars * 8`, `pixel_height = height_chars * 8`):
- **Przesunięcia**: `shift_left`, `shift_right`, `shift_up`, `shift_down` (wielokrotność pikseli w zależności od trybu koloru: 1 px mono, 2 px mode 4/5, 4 px mode 10).
- **Odbicia**: `horizontal_mirror` (z zachowaniem parzystości pikseli 2-bitowych lub 4-bitowych), `vertical_mirror`.
- **Inwersja**: `invert`.
- **Obroty**: `rotate_left`, `rotate_right` (dla rastrów kwadratowych w trybie mono oraz o proporcjach 2:1 w Mode 4/5 i 4:1 w Mode 10).

---

## 2. API i Struktury

```rust
use crate::renderer::RenderColorMode;

/// Matryca pikseli wyodrębniona z bloku znaków (1 bajt na piksel: 0 lub 1 w mono / indeks koloru).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelMatrix {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl PixelMatrix {
    pub fn new(width: usize, height: usize) -> Self;
    pub fn from_glyph_bytes(glyphs: &[u8], width_chars: usize, height_chars: usize) -> Self;
    pub fn to_glyph_bytes(&self, width_chars: usize, height_chars: usize) -> Vec<u8>;

    pub fn shift_left(&mut self, step: usize);
    pub fn shift_right(&mut self, step: usize);
    pub fn shift_up(&mut self);
    pub fn shift_down(&mut self);

    pub fn horizontal_mirror(&mut self, color_mode: RenderColorMode);
    pub fn vertical_mirror(&mut self);
    pub fn invert(&mut self);

    pub fn rotate_left(&mut self, color_mode: RenderColorMode);
    pub fn rotate_right(&mut self, color_mode: RenderColorMode);
}
```
