# PHASE 21B-3 — VIEW LINE-FONT EDITING AUDIT & FIX

**Data:** 2026-08-14
**Zakres:** View Line-Font Editing — audyt i naprawa osiągalności z GUI.
**Punkt odniesienia:** C# `atari-fontmaker-master/` (specyfikacja).

---

## 1. Executive Summary

Zgłoszenie FINAL RE-AUDIT potwierdzone: `set_line_font` istniało w `GuiState`, ale
**nie było osiągalne z GUI** (brak jakiegokolwiek wywołania z kontrolera/Slint).
Dodatkowo stwierdzono dwa rozbieżności semantyczne względem C#:

1. `set_line_font` wypychał krok undo (`push_view_undo`), podczas gdy C#
   `ActionCharacterSetSelector` **nie** wywołuje `PushState` — zmiana fontu wiersza
   nie jest samodzielnym krokiem undo.
2. Ładowanie `.atrview` nie normalizowało wartości `0 → 1` (C# `AtariView.Load` to robi).

Wszystkie trzy problemy naprawiono. Werdykt: **PASS**.

## 2. Analiza C# jako specyfikacji

### 2.1 Model danych
- `AtariView.UseFontOnLine : byte[26]` — font (1..4) używany w każdym z 26 wierszy.
- `AtariView.Setup()` inicjalizuje wszystkie wartości na **1**.
- Zapis **per page**: `PageData.SelectedFont` (hex string); także top-level `Lines`.

### 2.2 Interakcja użytkownika (jedyny mechanizm w C#)
`pictureBoxCharacterSetSelector` — pionowy pasek 15 px obok widoku, rysujący
`ToDraw[UseFontOnLine[OffsetY+y]]` = `"1".."4"` dla każdego wiersza (`RedrawLineTypes`).
`MouseDown → ActionCharacterSetSelector(e)`:
- `ry = e.Y / CellHeight` (CellHeight = 16; w Mode 5 = 32).
- **Ctrl** → ustaw font na 1.
- **Shift lub RMB** → cykl wstecz (wrap `1 → 4`).
- **LMB** → cykl w przód (wrap `4 → 1`).
- Po zmianie: `RedrawLineTypes(); RedrawView();`.

### 2.3 Rendering
`UseFontOnLine[OffsetY+y]` wybiera font przez `FontYOffset[font-1]` = `[0, 128, 256, 384]`
(= `(font-1)*128` wierszy atlasu); wersja kolorowa `FontPageOffset`. Zmiana fontu
zmienia **tylko** wybór bloku atlasu, nie dane znaków.

### 2.4 Undo/Redo
`ActionCharacterSetSelector` **nie** wywołuje `PushState()` → zmiana fontu wiersza
nie jest samodzielnym krokiem undo. (Migawka `AtariViewUndoInfo` zawiera
`UseFontOnLine`, więc undo późniejszej edycji znaku może ją pośrednio cofnąć.)

### 2.5 Dirty
C# **nie posiada** mechanizmu dirty tracking (brak jakichkolwiek pól `dirty`/`modified`).

### 2.6 Klawiatura
`Keyboard.cs` nie zawiera żadnego skrótu do zmiany fontu wiersza — mechanizm jest
**wyłącznie myszowy** (pasek selektora).

### 2.7 Stare formaty
`AtariView.Load` normalizuje `UseFontOnLine[i] == 0 → 1` (tylko 0, wartości >4 bez zmian).

## 3. Obecna implementacja Rust (przed naprawą)

- Model: `AtrViewProject.line_fonts: Vec<u8>` (26 wartości), `SavedPageData.selected_font` — OK.
- Rendering: `FontAtlasBuffer::render_view_image_rgba` używa `line_fonts[vy]` z
  `(font-1)*128` — **poprawny**.
- Persistence: `save_project_file`/`open_project_file`/`switch_to_page` przenoszą
  `line_fonts` — **poprawna**.
- `GuiState::set_line_font` — istniało, ale: (a) martwy kod (brak wywołań),
  (b) push undo niezgodny z C#.
- Brak UI (pasek selektora) i brak metody kontrolera.

## 4. GUI reachability matrix

| Element | C# | Rust przed | Rust po | Status |
|---|---|---|---|---|
| Model `line_fonts` (26) | ✔ | ✔ | ✔ | PASS |
| `set_line_font` | ✔ (ActionCharacterSetSelector) | ✔ (martwy) | ✔ | PASS |
| Kontroler | ✔ MouseDown | ✘ | ✔ `view_line_font_clicked` | PASS |
| Slint UI (pasek selektora) | ✔ pictureBoxCharacterSetSelector | ✘ | ✔ `view_editor_panel.slint` | PASS |
| Renderer używa fontów | ✔ | ✔ | ✔ | PASS |
| Per-page zapis/odczyt | ✔ | ✔ | ✔ | PASS |

## 5. `line_fonts` semantics

- `len == 26` — test `test_line_fonts_model_26_values_default_all_one`.
- Wartości `1..=4` — `set_line_font` clampuje; test `test_set_line_font_clamps_to_legal_range`.
- Pierwszy/ostatni wiersz niezależne — `test_first_and_last_line_independent`.
- Wszystkie 26 niezależne — `test_all_26_lines_independent`.
- Cykl forward/backward z wrap — `test_cycle_forward_and_backward_wraps`.

## 6. Rendering parity

- `test_rendering_font1_vs_font2_differs_and_view_bytes_unchanged`: font 1 vs font 2
  dają różny obraz; `view_bytes` pozostają identyczne.
- `test_other_lines_unchanged_after_single_line_change`: zmiana jednego wiersza
  nie rusza pozostałych.

## 7. Page parity

- `test_page_isolation_and_roundtrip` (Page1→Page2→Page1).
- `test_page_switch_saves_line_fonts`.
- `test_delete_page_keeps_surviving_page_fonts`.

## 8. Save/Open parity

- `test_save_new_open_roundtrip_all_26_lines` — pełny round-trip 26 wierszy.
- `test_existing_fixture_loads_all_font_one` — fixture `default.atrview` (wszystkie `01`).
- `test_zero_line_font_normalized_to_one_on_load` — stary format `0 → 1`.

## 9. Dirty tracking

C# nie ma dirty. W Rust `is_dirty` jest rozszerzeniem własnym aplikacji; zmiana
fontu wiersza ustawia `is_dirty = true` (spójnie z innymi mutacjami widoku, bo
`line_fonts` jest zapisywane). Save/Open czyszczą dirty — testy
`test_line_font_change_sets_dirty`, `test_save_and_open_clear_dirty`.

## 10. Undo/Redo

- `test_line_font_change_is_not_undoable`: zmiana fontu wiersza nie jest krokiem
  undo (zgodnie z C# — brak `PushState`). Usunięto `push_view_undo()` z `set_line_font`.

## 11. Keyboard

C# nie ma skrótu klawiaturowego — Rust nie dodaje żadnego (zgodnie z zasadą
„nie wymyślaj interakcji, których C# nie ma").

## 12. Mouse

Zaimplementowano pasek selektora (lewy margines widoku, 26 wierszy po 16 px),
klikalny jak C# `pictureBoxCharacterSetSelector`: LMB = cykl w przód, RMB/Shift =
cykl wstecz, Ctrl = reset na 1. Kontroler: `view_line_font_clicked(line, button, control, shift)`.
Test: `test_controller_view_line_font_clicked`.

## 13. View Configuration

`AtariViewConfigWindow.cs` nie dotyczy fontów wierszy (sprawdzono) — konfiguracja
rozmiaru widoku/offsettów; fonty wierszy zmieniane są wyłącznie paskiem selektora.
Brak potrzeby zmian.

## 14. Test coverage

`crates/afm_gui/tests/test_phase21b3_line_fonts.rs` — 16 testów (model, rendering,
pages, persistence, dirty, undo, stary format). Kontroler: `test_controller_view_line_font_clicked`.

## 15. Golden Master results

Brak zmian w golden masterach. Wszystkie istniejące testy (w tym golden exporterów
i `.atrview` round-trip) nadal przechodzą.

## 16. Wszystkie znalezione problemy

| ID | Waga | Opis |
|---|---|---|
| LF-1 | HIGH | `set_line_font` martwy kod — brak ścieżki GUI → controller → state. |
| LF-2 | MEDIUM | `set_line_font` push undo — niezgodne z C# (nie jest krokiem undo). |
| LF-3 | LOW | Ładowanie nie normalizowało `line_fonts == 0 → 1` (C# to robi). |

## 17. Wszystkie poprawki

- `crates/afm_gui/src/state.rs`: usunięto `push_view_undo()` z `set_line_font`;
  dodano `cycle_view_line_font(line, backward)`.
- `crates/afm_core/src/codecs/atrview.rs`: normalizacja `0 → 1` przy ładowaniu `Lines`.
- `crates/afm_gui/src/controller.rs`: dodano `view_line_font_clicked` + synchronizację
  modelu `view_line_fonts`.
- `crates/afm_gui/ui/components/view_editor_panel.slint`: pasek selektora fontów
  (26 wierszy) + właściwość `line_fonts` + callback `line_font_clicked`.
- `crates/afm_gui/ui/main_window.slint`: właściwość `view_line_fonts` + callback
  `view_line_font_clicked` + podpięcie panelu.
- `crates/afm_gui/src/app.rs`: podpięcie callbacku.
- `crates/afm_gui/tests/test_phase21b3_line_fonts.rs`: 16 testów regresyjnych.

## 18. Problemy pozostawione bez zmian (poza zakresem)

- legacy `.vf2/.vfn/.dat`, inne eksportery, pozostałe pozycje FINAL RE-AUDIT.

## 19. Ograniczenia środowiska testowego

- Środowisko headless: fizyczne kliknięcie paska selektora nie było wykonane;
  ścieżka zweryfikowana programowo (Slint callback → controller → state → renderer).
- Obsługa przewijanego widoku (OffsetY) i Mode 5 (CellHeight=32) nie jest fizycznie
  weryfikowalna w headless; implementacja widoku jest stała 40×26.

## 20. Exact verification commands

```
cargo fmt --all -- --check        # czysto
cargo check --workspace           # czysto
cargo test --workspace            # 225 passed / 0 failed / 0 ignored
cargo clippy --workspace -- -D warnings  # czysto
timeout 3 cargo run -p afm_gui    # exit 124 (proces żyje)
```

## 21. Final verdict

**PHASE 21B-3 — PASS**

**225 passed / 0 failed / 0 ignored**
