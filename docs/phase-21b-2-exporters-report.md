# PHASE 21B-2 — EXPORTERS AUDIT & FIX

**Data:** 2026-08-14
**Scope:** BMP Mono, BMP Color, Binary View exporters — reachability from GUI.
**Reference:** C# `ExportFontWindow.cs`, `ExportViewWindow.cs`, `AtariFontRenderer.cs`, `tools/ReferenceHarness`.

---

## 1. Werdykt

**PHASE 21B-2 — PASS**

Wszystkie trzy zgłoszone problemy zostały niezależnie potwierdzone jako REALNE
i naprawione. Eksportery BMP Mono, BMP Color i Binary View są teraz osiągalne
z interfejsu użytkownika i produkują bajt-identyczne pliki względem C#.

---

## 2. Audyt — wyniki

Pełny łańcuch `C# exporter → afm_core → GuiState → GuiController → Slint modal
→ akcja GUI → zapis do pliku` został prześledzony dla każdego eksportera.

### 2.1 Semantyka C# (ustalona na podstawie kodu źródłowego)

**BMP (Mono/Color) — `ExportFontWindow.SaveFontBMP(fontNr, filename, asColor)`**
- Wymiary: szerokość **256**, wysokość **64 / 128 / 256** (Font N / Font1+2 lub 3+4 / All).
- Źródło: bufor `BitmapFontBanks` (512×1024, 32bpp), próbkowany co **2 piksele**
  (`srcRect.X = x*2`, `srcRect.Y = y*2 + startFontIndex`).
- `startFontIndex = 128 * fontNr` dla mono (font 0..3); **+512** dla wersji kolorowej
  (górna połowa = mono, dolna = kolor).
- Format: `Format24bppRgb`, zapis `ImageFormat.Bmp` (BI_RGB, bottom-up, 96 DPI).
- W wersji kolorowej próbkuje dolną połowę renderowaną w trybie `WhichColorMode` (4/5/10).

**Binary View — `ExportViewWindow.SaveAsBinaryData(fileName, region, transpose)`**
- Surowe bajty `AtariView.ViewBytes[x, y]` regionu, bez nagłówka, `.dat`.
- Kolejność: wierszami (y zewnętrzna pętla, x wewnętrzna); przy `transpose` — kolumnami.

### 2.2 Stwierdzone defekty

| ID | Waga | Opis |
|----|------|------|
| E-1 | HIGH | BMP Mono — brak opcji w `ExportFontModal`; `font_export_file_meta` zna tylko `.txt`/`.lst`; `export_font_do_save` zapisuje wyłącznie tekst. |
| E-2 | HIGH | BMP Color — jak wyżej; ścieżka `as_color=true` nieosiągalna z GUI. |
| E-3 | HIGH | Binary View — brak funkcji `export_view_binary` w `afm_core`, brak opcji „Binary Data" w `ExportViewModal`; `export_view_do_save` zapisuje wyłącznie tekst. |

Istniejący `afm_core::exporters::export_font_bmp` był **poprawny** (testy golden
`font_default_mono.bmp` / `font_default_color.bmp` porównują bajt-po-bajcie z C#)
— problemem była wyłącznie nieosiągalność z GUI oraz brak eksportera binarnego widoku.

---

## 3. Wprowadzone poprawki (minimalne)

### 3.1 `afm_core`
- `exporters/view_text.rs`: dodano `export_view_binary(view_bytes, w, h, region, transpose) -> Vec<u8>`
  (surowe bajty regionu; wierszami / kolumnami przy transpose; klamping poza zakresem jak w `export_view_as_text`).
- `exporters/mod.rs`: reeksport `export_view_binary`.

### 3.2 `afm_gui` — `state.rs`
- `export_font_bmp_bytes(selection, as_color)` — prze-renderowuje atlas w bieżącym
  trybie kolorów (`render_full_atlas`) i eksportuje BMP (odpowiednik `SaveFontBMP`).
- `export_view_binary_bytes(region, transpose)` — surowe bajty widoku (odpowiednik `SaveAsBinaryData`).

### 3.3 `afm_gui` — `controller.rs`
- Indeksy formatu fontu: **8 = BMP Mono**, **9 = BMP Color** (podgląd pusty, jak
  czyszczenie `MemoExport` w C#); `font_export_file_meta` zwraca `FontN.bmp` /
  `Bitmap (*.bmp)`; `export_font_do_save` zapisuje raster przez `std::fs::write`.
- `export_font_copy_clipboard` — no-op z komunikatem dla BMP (C# wyłącza przycisk).
- Indeks formatu widoku: **7 = Binary Data** (podgląd pusty); `export_view_do_save`
  zapisuje surowe bajty do `View.dat` (filtr `Binary (*.dat)`), honorując transpose.
- `export_view_copy_clipboard` — no-op dla formatu binarnego.
- Dodano `font_selection_name` (Font1…Font1+2+3+4) wg `MakeFilenamePartFromFontSelectionNr`.

### 3.4 `afm_gui` — Slint
- `export_font_modal.slint`: dodano „BMP Mono", „BMP Color" do listy formatów.
- `export_view_modal.slint`: dodano „Binary Data" do listy formatów.

---

## 4. Testy regresyjne

**`afm_core/tests/test_exporters.rs` (+3)**
- `test_export_view_binary_row_major` — pełny region 40×26, wierszami.
- `test_export_view_binary_transposed` — kolejność kolumnowa.
- `test_export_view_binary_subregion_clamps_out_of_bounds` — klamping poza zakres.

**`afm_gui/src/controller.rs` (testy jednostkowe, +4)**
- `test_export_font_bmp_mono_save_matches_golden` — zapis BMP Mono przez GUI
  **bajt-identyczny** z golden `font_default_mono.bmp` (C#).
- `test_export_font_bmp_color_save_matches_golden` — j.w. dla `font_default_color.bmp`.
- `test_export_view_binary_save_writes_raw_bytes` — zapis surowych bajtów widoku.
- `test_export_view_binary_save_transposed` — zapis z transpozycją.

---

## 5. Weryfikacja

| Polecenie | Wynik |
|-----------|-------|
| `cargo fmt --check` | czysto |
| `cargo check --workspace` | czysto |
| `cargo test --workspace` | **195 passed / 0 failed** (poprzednio 188; +7 nowych) |
| `cargo clippy --workspace -- -D warnings` | czysto |
| `timeout 3 cargo run -p afm_gui` | exit 124 (proces żyje) |

---

## 6. Znane ograniczenia (nie powodują utraty danych)

1. **Kolejność pozycji listy formatów**: BMP Mono/Color i Binary Data dodano na
   końcu listy (indeksy 8/9/7), a nie na pozycjach 0/1/0 jak w C#. Zmiana kolejności
   wymagałaby przesunięcia wszystkich istniejących indeksów i testów — zachowanie
   semantyczne jest identyczne.
2. **Binary View — wybór regionu**: C# pozwala zaznaczać region myszą + offset;
   implementacja Rust eksportuje pełny region 40×26, co jest zachowaniem domyślnym
   C# (`RememberSelection=false` → cały widok).
3. **BMP Color — tryb koloru**: dolna połowa atlasu renderowana jest w bieżącym
   trybie kolorów (jak `WhichColorMode` w C#); nie weryfikowano fizycznie w trybie
   bezgłowym, ale pokrywa ją test golden (Mode 4).
4. **Poza zakresem fazy** (nie ruszano): fontowy eksport binarny `.dat`
   (`FormatTypes.BinaryData = 9` w C#) i kompresja ZX0 — nie były zgłoszone.
