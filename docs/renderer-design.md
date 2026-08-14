# Specyfikacja Silnika Renderowania Czcionek Atari (Phase 5)

> **Dokument**: Specyfikacja Techniczna Silnika Renderującego `afm_core`  
> **Faza**: Phase 5 — Atari Font Renderer  
> **Data**: 2026-08-14  
> **Źródła C#**: `AtariFontRenderer.cs`, `Colors.cs`, `Helpers.cs`, `ReferenceHarness`

---

## 1. Zakres i Rola Renderer'a

Silnik renderujący w `afm_core` jest bezstanowym (lub posiadającym dedykowany bufor pamięci) modułem headless generującym atlasy rastrowe i pojedyncze znaki fontu w formacie 32-bitowym (BGRA / RGBA) dla wszystkich 4 banków czcionek.

Obsługiwane tryby renderowania:
1. **Mono** (1-bit / 2 kolory, zoom 2×2).
2. **Mode 4 / Mode 5** (2-bit / 5 kolorów, przełączanie rejestru PF2/PF3 na znakach inwersyjnych, zoom 2×2).
3. **Mode 10** (4-bit / 9 rejestrów GTIA / 16 wartości, zoom 2×2).

---

## 2. Wymiary Atlasu i Układ Pamięci (Atlas Layout)

- **Szerokość (Width)**: **512 pikseli** (32 znaki w linii × 8 pikseli szerokości znaku × skala 2 = 512 px).
- **Wysokość (Height)**: **1024 piksele** (16 wierszy znaków mono + 16 wierszy znaków kolorowych = 32 wiersze × 16 px wysokości znaku = 1024 px).
- **Format piksela**: 32-bitowy BGRA (`[B: u8, G: u8, R: u8, A: u8]`), gdzie `A = 255 (0xFF)`.
- **Całkowity rozmiar bufora atlasu**: `512 * 1024 * 4 = 2 097 152 bajty` (2 MB).

### 2.1. Podział Pionowy Atlasu (Płaszczyzna Y)

```
Zakres Y (px)      Zawartość Atlasu
────────────────────────────────────────────────────────────────
0   - 63           Bank 1 (Font 0) Mono — Znaki normalne (wiersze 0..3)
64  - 127          Bank 1 (Font 0) Mono — Znaki inwersyjne (wiersze 4..7)
128 - 191          Bank 2 (Font 1) Mono — Znaki normalne (wiersze 8..11)
192 - 255          Bank 2 (Font 1) Mono — Znaki inwersyjne (wiersze 12..15)
256 - 319          Bank 3 (Font 2) Mono — Znaki normalne (wiersze 16..19)
320 - 383          Bank 3 (Font 2) Mono — Znaki inwersyjne (wiersze 20..23)
384 - 447          Bank 4 (Font 3) Mono — Znaki normalne (wiersze 24..27)
448 - 511          Bank 4 (Font 3) Mono — Znaki inwersyjne (wiersze 28..31)
────────────────────────────────────────────────────────────────
512 - 575          Bank 1 (Font 0) Kolor — Znaki normalne
576 - 639          Bank 1 (Font 0) Kolor — Znaki inwersyjne (z przełącznikiem PF3 / Mode 10 inv)
640 - 703          Bank 2 (Font 1) Kolor — Znaki normalne
704 - 767          Bank 2 (Font 1) Kolor — Znaki inwersyjne
768 - 831          Bank 3 (Font 2) Kolor — Znaki normalne
832 - 895          Bank 3 (Font 2) Kolor — Znaki inwersyjne
896 - 959          Bank 4 (Font 3) Kolor — Znaki normalne
960 - 1023         Bank 4 (Font 3) Kolor — Znaki inwersyjne
────────────────────────────────────────────────────────────────
```

---

## 3. Odwzorowanie Kolorów i Semantyka Trybów Graficznych

Paleta robocza (`ColorRegisters`) zawiera 10 indeksów barw (`0..9`):
- `color[0]`: Mono Foreground (znak)
- `color[1]`: Mono Background (tło) / Mode 4 Kolor 0 (BAK)
- `color[2]`: Mode 4 Kolor 1 (PF0)
- `color[3]`: Mode 4 Kolor 2 (PF1)
- `color[4]`: Mode 4 Kolor 3 (PF2 dla znaków normalnych 0..127)
- `color[5]`: Mode 4 Kolor 4 (PF3 dla znaków inwersyjnych 128..255)
- `color[6..8]`: Kolory pomocnicze Mode 10

### 3.1. Renderowanie Mono
- Bit `0`: Kolor tła `color[1]`
- Bit `1`: Kolor znaku `color[0]`
- W wersji inwersyjnej zamiana: bit `0` -> `color[0]`, bit `1` -> `color[1]`.

### 3.2. Renderowanie Mode 4 / 5 (2 bity / piksel)
- Wartość `00` (0): `color[1]` (BAK)
- Wartość `01` (1): `color[2]` (PF0)
- Wartość `10` (2): `color[3]` (PF1)
- Wartość `11` (3):
  - Dla znaku normalnego: `color[4]` (PF2)
  - Dla znaku inwersyjnego: `color[5]` (PF3)

### 3.3. Renderowanie Mode 10 (4 bity / piksel)
- Tablica mapowania normalnego: `[0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8, 4, 5, 6, 7]` (+1 indeks w `color`)
- Tablica mapowania inwersyjnego: `[7, 6, 5, 4, 8, 8, 8, 8, 7, 6, 5, 4, 3, 2, 1, 0]` (+1 indeks w `color`)

---

## 4. Architektura API w Rust (`afm_core`)

```rust
pub struct FontAtlasBuffer {
    pixels: Vec<u8>, // 512 * 1024 * 4 bytes (BGRA)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderColorMode {
    Mono = 2,
    Mode4 = 4,
    Mode5 = 5,
    Mode10 = 10,
}

pub struct FontRenderer {
    palette: Palette,
    color_registers: [u8; 10],
}
```

Metody:
- `render_all_fonts(&self, fonts: &FontBankSet, color_mode: RenderColorMode, buffer: &mut FontAtlasBuffer)`
- `render_one_character(&self, fonts: &FontBankSet, color_mode: RenderColorMode, char_idx: usize, on_bank2: bool, buffer: &mut FontAtlasBuffer)`

---

## 5. Strategia Weryfikacji z Golden Masters

Weryfikacja w testach integracyjnych porównuje wyjściowe bufory bajt po bajcie (`assert_eq!(actual, expected)`) względem:
- `tests/fixtures/renders/font_atlas_mono.raw`
- `tests/fixtures/renders/font_atlas_mode4.raw`
- `tests/fixtures/renders/font_atlas_mode10.raw`
- Test parytetu `render_one_character` względem `render_all_fonts`.
