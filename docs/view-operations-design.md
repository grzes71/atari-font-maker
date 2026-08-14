# Specyfikacja Operacji Blokowych na Widoku (afm_core::view::operations)

> **Dokument**: Specyfikacja Techniczna Operacji na Obszarach Widoku  
> **Faza**: Phase 10a — Core Domain Extensions  
> **Data**: 2026-08-14  
> **Źródła C#**: `AtariViewEditor.cs`, `ViewActionsWindow.cs`, `ImportViewWindow.cs`

---

## 1. Zakres Odpowiedzialności

Moduł `afm_core::view::operations` dostarcza czystych funkcji do manipulacji zawartością siatki ekranu Atari View:
- Zamiana znaków `ReplaceCharXWithY` z uwzględnieniem filtrowania banków i wierszy.
- Wypełnianie prostokątnego obszaru `FillArea`.
- Bezstanowe wycinanie i formowanie rastra widoku z zewnętrznego bufora binarnego `extract_view_import`.

---

## 2. API i Funkcje

```rust
use crate::exporters::ViewExportRegion;

/// Zamiana znaku `char_x` na `char_y` w wybranym obszarze prostokątnym,
/// z ograniczeniem do wierszy przypisanych do wybranych banków czcionek.
pub fn replace_char_x_with_y(
    view_bytes: &mut [u8],
    view_width: usize,
    view_height: usize,
    region: ViewExportRegion,
    char_x: u8,
    char_y: u8,
    active_fonts: [bool; 4],
    line_fonts: &[u8],
);

/// Wypełnienie prostokątnego obszaru wybranym znakiem.
pub fn fill_area(
    view_bytes: &mut [u8],
    view_width: usize,
    view_height: usize,
    region: ViewExportRegion,
    fill_char: u8,
);

/// Wycięcie prostokątnego wycinka bajtów z dowolnego bufora binarnego (np. pliku)
/// i ułożenie go w buforze ekranu widoku o zadanych wymiarach.
pub fn extract_view_import(
    source_bytes: &[u8],
    line_width: usize,
    skip_x: usize,
    skip_y: usize,
    copy_w: usize,
    copy_h: usize,
    target_w: usize,
    target_h: usize,
) -> Vec<u8>;
```

---

## 3. Semantyka Graniczna
- Koordynaty `region.rx..rx+rw` i `region.ry..ry+rh` są bezpiecznie przycinane do `view_width` i `view_height`.
- Jeśli `active_fonts` nie zawiera fontu przypisanego do danego wiersza (`line_fonts[y]`), wiersz ten nie podlega modyfikacji.
- `extract_view_import` zabezpiecza przed odczytem poza zakresem `source_bytes` zwracając zera dla brakujących bajtów.
