# Preferences & Configuration Architecture Design

> **Dokument**: Architektura i projekt konfiguracji oraz preferencji użytkownika  
> **Faza**: Phase 20 — Preferences, Keyboard Support & Final GUI Polish  
> **Data**: 2026-08-14  

---

## 1. Cel i Zakres

Zapewnienie pełnej obsługi preferencji aplikacji `FontMaker.json` zgodnie z klasami `Configuration.cs` i `FontMakerConfigurationWindow.cs` w aplikacji referencyjnej C#.

Konfiguracja obejmuje:
- Wybór domyślnego algorytmu kompresji eksportowanych danych (`ZX0`, `ZX1`, `ZX2`, `apultra`),
- Domyślne zestawy palet kolorów (`ColorSets`, 6 zestawów),
- Opcje analizy czcionki (`AnalysisColor`, `AnalysisAlpha`, `AnalysisDuplicates`, `AnalysisDupColor`, `AnalysisDupAlpha`),
- Domyślne opcje eksportera widoku (`ExportViewRemember`, `ExportViewExportType`, `ExportViewDataType`, obszar X/Y/W/H, offset X/Y, transpose),
- Domyślne opcje importera widoku (`ImportViewRemember`, `ImportLineWidth`, `ImportSkipX`, `ImportSkipY`, `ImportWidth`, `ImportHeight`).

---

## 2. Model Domenowy

Wykorzystany zostaje istniejący model z `afm_core::codecs::config::ConfigurationJson`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigurationJson {
    pub color_sets: Vec<String>,
    pub analysis_color: i32,
    pub analysis_alpha: i32,
    pub analysis_duplicates: bool,
    pub analysis_dup_color: i32,
    pub analysis_dup_alpha: i32,
    pub export_view_remember: bool,
    pub export_view_export_type: i32,
    pub export_view_data_type: i32,
    pub export_view_region_x: i32,
    pub export_view_region_y: i32,
    pub export_view_region_w: i32,
    pub export_view_region_h: i32,
    pub export_view_offset_x: i32,
    pub export_view_offset_y: i32,
    pub export_view_transpose: bool,
    pub import_view_remember: bool,
    pub import_line_width: i32,
    pub import_skip_x: i32,
    pub import_skip_y: i32,
    pub import_width: i32,
    pub import_height: i32,
    pub compressor_id: i32, // 0 = ZX0, 1 = ZX1, 2 = ZX2, 3 = apultra
}
```

---

## 3. Komponent UI: ConfigurationModal

Modalne okno preferencji `ConfigurationModal.slint` udostępnia:
1. **Wybór kompresora**:
   - `ZX0` (wartość 0),
   - `ZX1` (wartość 1),
   - `ZX2` (wartość 2),
   - `apultra` (wartość 3).
2. **Zestawy kolorów**: Podgląd 6 zestawów kolorów z możliwością przywrócenia domyślnych (`0E0028CA9446`).
3. **Zapamiętywanie ustawień eksportu/importu**: Przełączniki `Remember Export Settings` oraz `Remember Import Settings`.
4. **Przyciski akcji**:
   - `Reset Defaults`: przywraca ustawienia domyślne zgodnie z `verify_defaults()`.
   - `OK / Save`: zatwierdza i zapisuje konfigurację do pliku `FontMaker.json`.
   - `Cancel`: odrzuca wprowadzone zmiany i zamyka okno.

---

## 4. Walidacja i Trwałość

- Przy wczytywaniu pliku `FontMaker.json` wywoływana jest funkcja `verify_defaults()`, która naprawia brakujące lub błędne wartości.
- Plik konfiguracyjny zapisywany jest w formacie UTF-8 bez BOM.
