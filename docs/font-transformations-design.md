# Architektura i Projekt: Font Transformations & Operations

> **Dokument**: Projekt techniczny transformacji i operacji na czcionkach w `afm_core` i `afm_gui`  
> **Faza**: Phase 15 — Font Transformations & Operations  
> **Data**: 2026-08-14  

---

## 1. Wprowadzenie

Celem niniejszego modułu jest udostępnienie w GUI pełnego zestawu operacji na glifach, obszarach znakowych i bankach czcionek, odpowiadających funkcjom dostępnym w oryginalnym C# Atari FontMaker (`CharacterEditor.cs`, `AtariFont.cs`, `Keyboard.cs`, `FontMakerForm.cs`).

---

## 2. Mapowanie Operacji C# → Rust

| Kategoria | Operacja C# (`AtariFont.cs` / `CharacterEditor.cs`) | Odpowiednik Rust (`afm_core`) | Zakres / Poziom |
|---|---|---|---|
| **Single Glyph** | `AtariFont.ShiftLeft(idx, bank, color, mode)` | `FontBankSet::shift_left` | Pojedynczy glif (8 bajtów) |
| **Single Glyph** | `AtariFont.ShiftRight(idx, bank, color, mode)` | `FontBankSet::shift_right` | Pojedynczy glif (8 bajtów) |
| **Single Glyph** | `AtariFont.ShiftUp(idx, bank)` | `FontBankSet::shift_up` | Pojedynczy glif (8 bajtów) |
| **Single Glyph** | `AtariFont.ShiftDown(idx, bank)` | `FontBankSet::shift_down` | Pojedynczy glif (8 bajtów) |
| **Single Glyph** | `AtariFont.RotateLeft(idx, bank)` | `FontBankSet::rotate_left` | Pojedynczy glif (8 bajtów) |
| **Single Glyph** | `AtariFont.RotateRight(idx, bank)` | `FontBankSet::rotate_right` | Pojedynczy glif (8 bajtów) |
| **Single Glyph** | `AtariFont.MirrorHorizontal(idx, bank, color, mode)` | `FontBankSet::mirror_horizontal` | Pojedynczy glif (8 bajtów) |
| **Single Glyph** | `AtariFont.MirrorVertical(idx, bank)` | `FontBankSet::mirror_vertical` | Pojedynczy glif (8 bajtów) |
| **Single Glyph** | `AtariFont.InvertCharacter(idx, bank)` | `FontBankSet::invert_character` | Pojedynczy glif (8 bajtów) |
| **Single Glyph** | `AtariFont.ClearCharacter(idx, bank)` | `FontBankSet::clear_character` | Pojedynczy glif (8 bajtów) |
| **Bank Shift** | `AtariFont.ShiftFontLeft(idx, bank, makeHole)` | `FontBankSet::shift_font_left` | Bank 128 znaków (1024 B) |
| **Bank Shift** | `AtariFont.ShiftFontRight(idx, bank, makeHole)` | `FontBankSet::shift_font_right` | Bank 128 znaków (1024 B) |
| **Bank Shift** | `AtariFont.DeleteAndShiftLeft(idx, bank)` | `FontBankSet::delete_and_shift_left` | Bank 128 znaków (1024 B) |
| **Bank Shift** | `AtariFont.DeleteAndShiftRight(idx, bank)` | `FontBankSet::delete_and_shift_right` | Bank 128 znaków (1024 B) |
| **Area Matrix** | MegaCopy Area Transformations | `afm_core::font::area_transforms::PixelMatrix` | Obszar N×M znaków |

---

## 3. Semantyka Undo / Redo

### 3.1. Operacje na pojedynczym glifie
- Wykonanie operacji oznacza znak jako zmodyfikowany (`is_char_edited = true`) i aktualizuje atlas przez `render_one_char_atlas`.
- Kliknięcie **Undo** przy `is_char_edited == true`:
  1. Zatwierdza zmieniony stan do kolejnego slotu bufora (`Add2Undo(true)`).
  2. Przywraca poprzedni bazowy stan sprzed modyfikacji (`Undo()`).
  3. Uaktywnia przycisk **Redo**, który pozwala natychmiast przywrócić wykonaną transformację.
- Zmiana aktywnego znaku lub zmiana pary banków automatycznie zatwierdza bieżący stan w buforze historii.

### 3.2. Operacje bankowe i obszarowe (MegaCopy)
- Operacje wpływające na wiele znaków jednocześnie (np. `shift_font_left`, `delete_and_shift_right`, `PixelMatrix`) wywołują `FontUndoBuffer::add_to_undo_full_difference_scan`, rejestrując pełny snapshot i wymuszając odświeżenie atlasu `render_full_atlas`.

---

## 4. Różne Szerokości Logicznego Piksela w Trybach Kolorów

Przy przesunięciach poziomych i operacjach obszarowych krok pikselowy zależy ściśle od trybu renderowania:
- **Monochrome (Mode 2 / Gr.0)**: 1 bit na piksel (`step = 1 px`).
- **Mode 4 & Mode 5 (Graphics 12/13)**: 2 bity na piksel (`step = 2 px` / 1 piksel kolorowy).
- **Mode 10 (Graphics 10)**: 4 bity na piksel (`step = 4 px` / 1 piksel 16-kolorowy).

---

## 5. Skróty Klawiszowe (Parity z `Keyboard.cs` i `FontMakerForm.cs`)

- `,` lub `[`: Wybór poprzedniego znaku (`ExecuteSelectPreviousCharacter`, z zawijaniem 0..511).
- `.` lub `]`: Wybór kolejnego znaku (`ExecuteSelectNextCharacter`, z zawijaniem 0..511).
- `r`: Rotate Left.
- `R` (`Shift+R`): Rotate Right.
- `m`: Mirror Horizontal.
- `M` (`Shift+M`): Mirror Vertical.
- `b` lub `B`: Przełączenie pary banków (`Banks 1 & 2` ↔ `Banks 3 & 4`).
- `i` lub `I`: Invert Character (w trybie Mono).
- `c` lub `C` (bez Ctrl): Clear Character.
- `Ctrl+Z`: Undo font change.
- `Ctrl+Y`: Redo font change.
- `1`..`4` / `0`: Szybki wybór aktywnego koloru rysowania.

---

## 6. Strategia Testów

1. **Testy jednostkowe `afm_core`**:
   - Sprawdzenie wszystkich operacji jednostkowych i bankowych dla Banków 1, 2, 3, 4.
   - Weryfikacja przesunięć obszarowych `PixelMatrix` dla kroków 1, 2 i 4 px.
2. **Testy integracyjne `afm_gui`**:
   - Wywołanie każdej transformacji z poziomu kontrolera.
   - Sprawdzenie inkrementalnego odświeżania bufora atlasu (`render_one_char_atlas`).
   - Weryfikacja cyklu Undo → Redo → nowa edycja.
   - Weryfikacja skrótów klawiszowych.
