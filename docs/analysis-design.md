# Specyfikacja Silnika Analizy Czcionek (afm_core::analysis)

> **Dokument**: Specyfikacja Techniczna Silnika Analizy Czcionek i Użycia Glifów  
> **Faza**: Phase 10a — Core Domain Extensions  
> **Data**: 2026-08-14  
> **Źródła C#**: `FontAnalysisWindow.cs`, `AtariFont.cs`

---

## 1. Zakres Odpowiedzialności

Moduł `afm_core::analysis` odpowiada za bezstanową analizę użycia znaków w projekcie wielostronicowym (`AtrViewProject` / `SavedPageData`) oraz identyfikację duplikatów glifów w bankach czcionek (`FontBankSet`).

Moduł nie zależy od żadnych bibliotek GUI ani obiektów platformowych.

---

## 2. Struktury Danych

```rust
/// Wynik ogólnej analizy częstości wystąpień znaków i duplikatów.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontAnalysisResult {
    /// Licznik wystąpień dla każdego znaku (4 banki × 256 znaków = 1024 wpisy).
    /// Znak normalny i odwrócony (inwersja) są liczone osobno.
    pub full_char_counts: [u32; 4 * 256],

    /// Zsumowany licznik znaków (4 banki × 128 znaków = 512 wpisów).
    /// Znak normalny i odwrócony są sumowane do jednego wpisu bazowego (0..127).
    pub combined_char_counts: [u32; 4 * 128],

    /// Indeks pierwszego duplikatu dla każdego znaku bazowego (4 banki × 128 znaków).
    /// Wartość -1 oznacza brak wcześniejszego duplikatu.
    pub duplicate_of_char: [i32; 4 * 128],
}

/// Szczegółowa informacja o wystąpieniu znaku na pojedynczej stronie projektu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageUsageDetail {
    pub page_index: usize,
    pub page_name: String,
    pub normal_count: u32,
    pub inverted_count: u32,
    /// Indeks pierwszego wystąpienia w siatce strony (x + y * 40), lub None jeśli nie występuje.
    pub first_occurrence_index: Option<usize>,
}

/// Szczegółowy raport użycia danego znaku na wszystkich stronach projektu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterUsageReport {
    pub font_index: usize,
    pub base_char: u8,
    pub page_usages: Vec<PageUsageDetail>,
}

/// Szczegółowy raport duplikatów danego znaku w obrębie banku czcionki.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateReport {
    pub font_index: usize,
    pub base_char: u8,
    /// Lista indeksów innych znaków w banku będących duplikatami tego znaku.
    pub duplicate_char_indices: Vec<u8>,
}
```

---

## 3. Algorytmy i Niezmienniki

1. **`analyze_project(project: &AtrViewProject, fonts: &FontBankSet) -> FontAnalysisResult`**:
   - Dla każdej strony projektu iteruje po wierszach `y` (0..26) i kolumnach `x` (0..40).
   - Pobiera numer fontu wiersza `font_nr = page.selected_font[y]`.
   - Jeśli `font_nr` mieści się w zakresie 1..=4, oblicza przesunięcia banku:
     - `full_offset = (font_nr - 1) * 256`
     - `combined_offset = (font_nr - 1) * 128`
   - Inkrementuje `full_char_counts[full_offset + char]` oraz `combined_char_counts[combined_offset + (char & 127)]`.
   - Sprawdza duplikaty glifów w bankach (0..4) i znakach (0..128) metodą porównania 8 bajtów glifu `is_duplicate`.
2. **`analyze_character_usage(...) -> CharacterUsageReport`**:
   - Wyznacza wystąpienia znaków `base_char` oraz `base_char + 128` na stronach z przypisanym fontem `font_index`.
3. **`analyze_duplicates(...) -> DuplicateReport`**:
   - Zwraca listę znaków o identycznej zawartości graficznej 8 bajtów glifu w danym banku.
