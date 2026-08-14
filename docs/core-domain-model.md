# Atari FontMaker — Specyfikacja Minimalnego Modelu Domenowego (Phase 2)

> **Dokument**: Specyfikacja Techniczna Modelu Domenowego `afm_core`  
> **Faza**: Phase 2 — Minimal Domain Model & Glyph Transformations  
> **Data**: 2026-08-14  
> **Źródła C#**: `AtariFont.cs`, `Constants.cs`, `Helpers.cs`, `CharacterEditor.cs`

---

## 1. Zakres i Odpowiedzialność Fazy 2

Faza 2 ustanawia minimalny, idiomatyczny fundament domenowy w bibliotece `afm_core`, odpowiadający za:
1. Przechowywanie danych fontów w pamięci (4 banki po 128 znaków po 8 bajtów = 4096 bajtów).
2. Reprezentację pojedynczego glifu 8×8 (`GlyphBytes`).
3. Deterministyczne kodowanie i dekodowanie bitowe (1-bit Mono, 2-bit Mode 4/5, 4-bit Mode 10).
4. Wszystkie transformacje geometryczne i bitowe glifów (rotacje 90° CW/CCW, odbicia lustrzane H/V, przesunięcia pikselowe z zawijaniem, inwersja bitowa, czyszczenie).
5. Obliczanie przesunięć adresowych w buforze fontów (`get_character_offset`) dla siatki 32×16 (512 znaków na 2 bankach).
6. Operacje całobankowe (przesunięcia w lewo/prawo z dziurą i bez, usuwanie i dosuwanie znaków, detekcja duplikatów).
7. Konwersję kodów znaków Atari (`convert_atari_char`).
8. Konwersje macierzowe 2D dla edytora glifów (`Get2ColorCharacter`, `Get5ColorCharacter`, `Get4BitColorCharacter`, `Set5ColorCharacter`, `Set4BitCharacter`).

---

## 2. Odwzorowanie Typów C# na Rust

| Typ / Klasa C# | Odpowiednik w Rust (`afm_core`) | Opis i Niezmienniki |
|---|---|---|
| `Constants.NumFontBytes` (4096) | `afm_core::constants::TOTAL_FONTS_SIZE` | Stała całkowitego rozmiaru bufora 4 banków. |
| `AtariFont.FontBytes` (`byte[4096]`) | `afm_core::font::FontBankSet` | Bezpieczna struktura opakowująca `[u8; 4096]`. |
| `AtariFont.OneCharacterBuffer` (`byte[8]`) | `afm_core::font::GlyphBytes` | Wartość 8 bajtów reprezentująca glif 8×8 pikseli. |
| `AtariFont.DecodeMono` / `EncodeMono` | `GlyphBytes::decode_mono` / `encode_mono` | Konwersja 1 bajtu ↔ 8 pikseli 1-bitowych (`0` lub `1`). |
| `AtariFont.DecodeColor2Bit` / `EncodeColor2Bit` | `GlyphBytes::decode_color_2bit` / `encode_color_2bit` | Konwersja 1 bajtu ↔ 4 pikseli 2-bitowych (`0..3`). |
| `AtariFont.DecodeColor4Bit` / `EncodeColor4Bit` | `GlyphBytes::decode_color_4bit` / `encode_color_4bit` | Konwersja 1 bajtu ↔ 2 pikseli 4-bitowych (`0..15`). |
| `AtariFont.RotateLeft` / `RotateRight` | `transforms::rotate_left` / `rotate_right` | Czyste funkcje rotacji macierzy 8×8 glifu. |
| `AtariFont.MirrorHorizontal` | `transforms::mirror_horizontal` | Odbicie poziome (1-bit: bit reverse, 2-bit: pary bitów, 4-bit: nibbles). |
| `AtariFont.MirrorVertical` | `transforms::mirror_vertical` | Odwrócenie kolejności 8 wierszy glifu. |
| `AtariFont.ShiftLeft` / `ShiftRight` | `transforms::shift_left` / `shift_right` | Przesunięcie bitowe z cyklicznym zawijaniem w wierszu. |
| `AtariFont.ShiftUp` / `ShiftDown` | `transforms::shift_up` / `shift_down` | Cykliczne przesunięcie wierszy glifu w pionie. |
| `AtariFont.InvertCharacter` / `ClearCharacter` | `transforms::invert` / `clear` | Negacja bitowa (`^ 0xFF`) oraz zerowanie glifu. |
| `AtariFont.GetCharacterOffset` | `FontBankSet::character_offset` | Obliczenie adresu początku znaku w buforze 4096 B. |
| `AtariFont.ShiftFontLeft` / `ShiftFontRight` | `FontBankSet::shift_font_left` / `shift_font_right` | Przesunięcie znaków w obrębie 1024-bajtowego banku. |
| `AtariFont.DeleteAndShiftLeft` / `DeleteAndShiftRight` | `FontBankSet::delete_and_shift_left` / `delete_and_shift_right` | Usunięcie znaku z przesunięciem ogona banku. |
| `AtariFont.IsDuplicate` | `FontBankSet::is_duplicate` | Porównanie 8 bajtów dwóch znaków w danym banku. |
| `Helpers.AtariConvertChar` | `font::convert_atari_char` | Konwersja kodów ASCII na kody wewnętrzne Atari. |

---

## 3. Szczegółowe Niezmienniki i Algorytmy

### 3.1. Adresowanie Znaków (`get_character_offset`)
W aplikacji C# siatka wyboru znaków ma wymiary 32 kolumny na 16 wierszy:
- Wiersze 0..3 (indeksy 0..127): Bank 1, znaki normalne.
- Wiersze 4..7 (indeksy 128..255): Bank 1, znaki odwrócone (ten sam offset pamięci co 0..127).
- Wiersze 8..11 (indeksy 256..383): Bank 2, znaki normalne (offset 1024..2047).
- Wiersze 12..15 (indeksy 384..511): Bank 2, znaki odwrócone (offset 1024..2047).
- Flaga `on_bank2 == true` przesuwa bazę o 2048 bajtów (dostęp do Banku 3 i Banku 4).

Wzór zgodny z C#:
```rust
pub fn get_character_offset(character_index: usize, on_bank2: bool) -> usize {
    let mut ry = character_index / 32;
    let rx = character_index % 32;
    if ry > 3 && ry < 12 {
        ry -= 4;
    }
    if ry > 11 && ry < 16 {
        ry -= 8;
    }
    ry * 32 * 8 + rx * 8 + if on_bank2 { 2048 } else { 0 }
}
```

### 3.2. Czyste Transformacje Glifów
Wszystkie transformacje są zaimplementowane jako czyste funkcje przyjmujące `&GlyphBytes` i zwracające nowy `GlyphBytes`. Pozwala to na ich bezpośrednie testowanie jednostkowe bez mutowania globalnego bufora.

---

## 4. Zgodność z Golden Masters

Wszystkie funkcjonalności Fazy 2 są weryfikowane testami headless względem następujących orakularnych fixtures z C# Reference Harness:
1. `tests/fixtures/transforms/character_offsets.json` (512 wektorów offsetów).
2. `tests/fixtures/transforms/glyph_transforms_golden.json` (128 znaków standardowych × 16 transformacji = 2048 asercji).
3. `tests/fixtures/transforms/edge_cases_transforms_golden.json` (8 zestawów syntetycznych).
4. `tests/fixtures/transforms/bank_operations_golden.json` (operacje całobankowe).
5. `tests/fixtures/encodings/mono_vectors.json` (256 bajtów 1-bit).
6. `tests/fixtures/encodings/color_2bit_vectors.json` (256 bajtów 2-bit).
7. `tests/fixtures/encodings/color_4bit_vectors.json` (256 bajtów 4-bit).
8. `tests/fixtures/encodings/atari_convert_char_vectors.json` (256 kodów ASCII).
9. `tests/fixtures/encodings/glyph_matrix_conversions.json` (konwersje 2D dla edytora).
