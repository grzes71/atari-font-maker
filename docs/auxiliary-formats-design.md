# Specyfikacja Formatów Pomocniczych (Phase 7)

> **Dokument**: Specyfikacja Techniczna Formatów Pomocniczych (`ClipboardJson`, `TileSet`, `Configuration`)  
> **Faza**: Phase 7 — Auxiliary Project/Data Formats  
> **Data**: 2026-08-14  
> **Źródła C#**: `CharacterEditor.cs` (`ClipboardJson`), `TileSet.cs` (`AtrTileSetJson`, `AtrTileJson`), `Configuration.cs` (`ConfigurationJson`), `ReferenceHarness`

---

## 1. Wprowadzenie i Zakres

Faza 7 obejmuje implementację trzech pomocniczych struktur i formatów serializacji w bibliotece `afm_core`:
1. **`ClipboardJson`** — format schowka znaków i kafelków edytora (`clipboard_sample.json`).
2. **`AtrTileSetJson` i `AtrTileJson`** — formaty zapisu pojedynczych kafelków (`.atrtile`) oraz zestawów kafelków (`.atrtileset`).
3. **`ConfigurationJson`** — schemat konfiguracji i preferencji użytkownika (`sample_config.json`, `FontMaker.json`).

Wszystkie formaty są zaimplementowane w sposób niezależny od GUI i oparte o `serde::{Serialize, Deserialize}`.

---

## 2. Struktury Danych i Reguły Normalizacji

### 2.1. Format Schowka (`ClipboardJson`)

Struktura służy do wymiany fragmentów ekranu i definicji znaków:
- `Width: Option<String>` / `Height: Option<String>` — wymiary zaznaczenia w znakach (min. 1×1).
- `Chars: Option<String>` — heksadecymalny ciąg kodów znaków (`Width * Height * 2` znaków hex).
- `Data: Option<String>` — heksadecymalny ciąg danych matryc glifów (`Width * Height * 2 * 8 = Width * Height * 16` znaków hex).
- `FontNr: Option<String>` — numery czcionek dla poszczególnych wierszy (`Height` znaków, np. `"11"`).
- `Nulls: Option<String>` — flaga przezroczystości/użycia dla każdego znaku (`Width * Height` znaków: `'0'` lub `'1'`).

#### Metody i Niezmienniki:
- `verify_width_height(&self) -> Option<(usize, usize)>`: Walidacja, czy `Width` i `Height` są poprawnymi liczbami całkowitymi $\ge 1$.
- `fix_characters(&mut self, w: usize, h: usize)`: Uzupełnia brakujące znaki zerami `'0'` do długości `w * h * 2`.
- `fix_data(&mut self, w: usize, h: usize)`: Uzupełnia brakujące dane glifów zerami `'0'` do długości `w * h * 16`.
- `fix_font_nr(&mut self, h: usize)`: Uzupełnia numery fontów jedynkami `'1'` do długości `h`.
- `fix_nulls(&mut self, w: usize, h: usize)`: Uzupełnia flagi null zerami `'0'` do długości `w * h`.
- `fix_all(&mut self) -> bool`: Wykonuje pełną walidację i normalizację pól.

---

### 2.2. Formaty Kafelków (`.atrtile`, `.atrtileset`)

#### Pojedynczy Kafelek (`AtrTileJson` -> `.atrtile`):
```json
{
  "Version": "1",
  "Tile": {
    "Nr": 42,
    "View": "000102030405060708090A0B0C0D0E0F101112131415161718",
    "Font": "22222222",
    "Nulls": "0000000000000000000000000",
    "Width": 5,
    "Height": 5
  }
}
```

#### Zestaw Kafelków (`AtrTileSetJson` -> `.atrtileset`):
```json
{
  "Version": "1",
  "Tiles": [
    {
      "Nr": 0,
      "View": "...",
      "Font": "...",
      "Nulls": "...",
      "Width": 5,
      "Height": 5
    }
  ]
}
```

---

### 2.3. Format Konfiguracji (`ConfigurationJson` -> `FontMaker.json`)

Zawiera trwałe ustawienia edytora:
- `ColorSets`: Lista 6 zestawów kolorów (każdy po 12 lub 20 znaków hex).
- `AnalysisColor`, `AnalysisAlpha`, `AnalysisDuplicates`, `AnalysisDupColor`, `AnalysisDupAlpha`: Ustawienia analizy czcionek.
- `ExportView*`: Ustawienia i zapamiętany region eksportu widoku.
- `ImportView*`: Ustawienia importu ekranu.
- `CompressorId`: Identyfikator wybranego algorytmu kompresji (0 = ZX0, 1 = ZX1, 2 = ZX2).

#### Reguły `verify_defaults`:
- `ColorSets` musi zawierać co najmniej 6 elementów (brakujące są uzupełniane domyślnym `"0E0028CA9446"`).
- Ograniczenia zakresów współrzędnych eksportu (`RegionX` w `0..40`, `RegionY` w `0..26`).
- Wymiary importu (`ImportLineWidth`, `ImportWidth`, `ImportHeight`) muszą wynosić co najmniej 1.

---

## 3. Mapowanie C# → Rust

| Typ / Klasa C# | Odpowiednik w Rust (`afm_core`) | Rola |
|---|---|---|
| `ClipboardJson` | `afm_core::codecs::clipboard::ClipboardJson` | Model i normalizacja schowka. |
| `AtrTileJson` | `afm_core::codecs::tileset::AtrTileJson` | Format pliku pojedynczego kafelka `.atrtile`. |
| `AtrTileSetJson` | `afm_core::codecs::tileset::AtrTileSetJson` | Format pliku zestawu kafelków `.atrtileset`. |
| `SavedTileData` | `afm_core::codecs::tileset::SavedTileData` | DTO danych kafelka. |
| `ConfigurationJson` | `afm_core::codecs::config::ConfigurationJson` | DTO pliku konfiguracyjnego `FontMaker.json`. |
| `Configuration.VerifyDefaults()` | `ConfigurationJson::verify_defaults()` | Walidacja i ustawianie domyślnych wartości preferencji. |

---

## 4. Strategia Testowa

Weryfikacja na podstawie golden masters w `tests/fixtures/projects/`:
1. `clipboard_sample.json` (odczyt, normalizacja `fix_*`, serializacja).
2. `sample.atrtile` (odczyt, serializacja, roundtrip).
3. `sample.atrtileset` (odczyt, serializacja, roundtrip).
4. `sample_config.json` (odczyt, weryfikacja wartości domyślnych `verify_defaults`, serializacja).
5. Testy odporności na brakujące i błędne pola oraz dane spoza zakresu.
