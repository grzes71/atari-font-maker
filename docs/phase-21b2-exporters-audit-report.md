# PHASE 21B-2 — EXPORTERS AUDIT & FIX (FULL REPORT)

**Data:** 2026-08-14
**Zakres:** BMP Mono, BMP Color, Binary View exporter — audyt i naprawa osiągalności z GUI.
**Punkt odniesienia:** C# `atari-fontmaker-master/` (specyfikacja), golden mastery `tests/fixtures/`.

---

## 1. Executive Summary

Wszystkie trzy zgłoszone problemy zostały **niezależnie potwierdzone jako realne**
i naprawione. Eksportery BMP Mono, BMP Color i Binary View są teraz osiągalne
z interfejsu użytkownika i zapisują do rzeczywistego pliku dane **bajt-identyczne
z C#** (BMP) lub zgodne z C# (Binary View). Dodano 13 nowych testów regresyjnych.

## 2. C# reference analysis

### 2.1 `ExportFontWindow` (font export)

`FormatTypes`: ImageBmpMono=0, ImageBmpColor=1, Assembler=2, Action=3, AtariBasic=4,
FastBasic=5, MADSdta=6, CDataArray=7, MadPascalArray=8, BinaryData=9, BasicListingFile=10.

**`SaveFontBMP(fontNr, filename, asColor)`**
- Szerokość obrazu: **256** px.
- Wysokość: **64** (Font 1/2/3/4), **128** (Font1+2 / Font3+4), **256** (All Fonts).
- Źródło: `BitmapFontBanks` (512×1024, 32bpp), próbkowane co **2 px**:
  `srcRect.X = x*2`, `srcRect.Y = y*2 + startFontIndex`.
- `startFontIndex = 128 * fontNr` (font 0..3); **+512** dla koloru (dolna połowa bitmapy).
- Zapisywane jako `Format24bppRgb` + `ImageFormat.Bmp` → BI_RGB, bottom-up, 24 bpp.
- Pojedynczy font obejmuje **normal + inverse** (128 wierszy atlasu), stąd BMP 64 wierszy
  zawiera normal (góra) i inverse (dół) — potwierdzone na golden masterze.

**`GetFontData(fontNr, withCompression)`** — surowe bajty zakresu (1024/2048/4096 B).

**`SaveBinaryData(fontNr, filename, withCompression)`** — zapis `.dat` (poza zakresem fazy).

### 2.2 `ExportViewWindow` (view export)

`FormatTypes`: BinaryData=0, Assembler=1, … MadPascalArray=7.

**`SaveAsBinaryData(fileName, region, transpose)`**
- Surowe bajty `AtariView.ViewBytes[x, y]` regionu, **bez nagłówka**, rozszerzenie `.dat`.
- Kolejność: wierszami (y zewnętrzna pętla, x wewnętrzna); `transpose` → kolumnami.
- Domyślnie region = cały widok (40×26 = 1040 B).
- Eksportuje **kody znaków**, nie fonty; `line_fonts` wpływają tylko na render.

### 2.3 `AtariFontRenderer`

- `BitmapFontBanks` 512×1024; górna połowa = mono, dolna = kolor.
- Każdy font: normal (64 wiersze) + inverse (64 wiersze), stąd 128 wierszy na font.
- Kolor renderowany w trybie `WhichColorMode` (4/5/10); Mode 4 i 5 współdzielą gałąź 2-bitową.

## 3. BMP Mono parity

| Kryterium | C# | Rust | Status |
|---|---|---|---|
| Dostępność w GUI | format 0 | indeks 8 w `ExportFontModal` | PASS |
| Wymiary | 256×(64/128/256) | 256×(64/128/256) | PASS |
| Bit depth | 24 | 24 (BI_RGB) | PASS |
| Próbkowanie | co 2 px | co 2 px | PASS |
| normal+inverse | tak | tak | PASS |
| Zapis do pliku | `ImageFormat.Bmp` | ręczny nagłówek + piksele (golden-zgodny) | PASS |
| Golden master | — | `font_default_mono.bmp` byte-for-byte | PASS |

Weryfikacja byte-for-byte z C#: test `test_export_font_bmp_mono_save_matches_golden`
(zapis przez GUI) oraz `test_export_font_bmp_mono_golden` (afm_core).

## 4. BMP Color parity

| Kryterium | C# | Rust | Status |
|---|---|---|---|
| Dostępność w GUI | format 1 | indeks 9 w `ExportFontModal` | PASS |
| Źródło danych | dolna połowa `BitmapFontBanks` | dolna połowa atlasu (+512) | PASS |
| Tryb koloru | `WhichColorMode` 4/5/10 | `active_color_mode` → Mono/Mode4/Mode5/Mode10 | PASS |
| Mode 4 == Mode 5 | tak | tak (wspólna gałąź 2-bitowa) | PASS |
| Mode 10 ≠ Mode 4 | tak | tak (inna geometria 4-bitowa) | PASS |
| Paleta | `CachedColors` z projektu | `renderer.cached_colors` z projektu | PASS |
| Zmiana rejestru | zmienia raster | zmienia raster | PASS |
| Golden master | — | `font_default_color.bmp` byte-for-byte | PASS |

Kluczowe: eksport koloru używa **tej samej palety co renderer** (`project.colors` +
`Palette::default_altirra()`), nie innego źródła. `export_font_bmp_bytes` prze-renderowuje
atlas w bieżącym trybie kolorów przed eksportem (odpowiednik cache `BitmapFontBanks`).

## 5. Binary View parity

| Kryterium | C# | Rust | Status |
|---|---|---|---|
| Dostępność w GUI | format 0 | indeks 7 w `ExportViewModal` | PASS |
| Rozmiar | 40×26 = 1040 B | 40×26 = 1040 B | PASS |
| Kolejność | wierszami (transpose → kolumnami) | wierszami / kolumnami | PASS |
| Aktywna strona | `AtariView.ViewBytes` (bieżąca strona) | `project.view_bytes` (bieżąca strona) | PASS |
| `Nulls` | nie dotyczy (surowe kody) | nie dotyczy | PASS |
| `line_fonts` | nie wpływają na bajty | nie wpływają | PASS |
| Rozszerzenie | `.dat` | `.dat` | PASS |

## 6. GUI reachability matrix

| Funkcja | C# | afm_core | GuiState | Controller | Slint | File/Clipboard | Status |
|---|---|---|---|---|---|---|---|
| BMP Mono | ✔ | ✔ `export_font_bmp` | ✔ `export_font_bmp_bytes` | ✔ `export_font_do_save` idx 8 | ✔ dropdown | ✔ `std::fs::write` | **PASS** |
| BMP Color | ✔ | ✔ `export_font_bmp(as_color)` | ✔ `export_font_bmp_bytes` | ✔ idx 9 | ✔ dropdown | ✔ | **PASS** |
| Binary View | ✔ | ✔ `export_view_binary` (nowy) | ✔ `export_view_binary_bytes` | ✔ idx 7 | ✔ dropdown | ✔ | **PASS** |

Przed fazą wszystkie trzy pozycje miały status **FAIL** (funkcja istniała w afm_core
lub wcale; brak ścieżki GUI → file).

## 7. File I/O verification

Zweryfikowano programowo (nie tylko preview), z injected fake dialogiem (`TestFileDialogs`)
i **prawdziwym** eksporterem zapisującym przez `std::fs::write`:

- `test_export_font_bmp_mono_save_matches_golden` — zapisany plik == `font_default_mono.bmp` (byte-for-byte).
- `test_export_font_bmp_color_save_matches_golden` — zapisany plik == `font_default_color.bmp` (byte-for-byte).
- `test_export_view_binary_save_writes_raw_bytes` — plik istnieje, 1040 B, kolejność wierszowa.
- `test_export_view_binary_save_transposed` — plik istnieje, kolejność kolumnowa.
- `test_export_cancel_keeps_dialog_open` — anulowanie dialogu nie zapisuje pliku i zostawia modal otwarty.

Native `rfd` dialog runtime nie był klikany fizycznie (headless) — patrz sekcja 14.

## 8. Clipboard verification

- Format tekstowy (ASM/Action/…/LST): `test_export_copy_clipboard_sets_clipboard` —
  zawartość `ClipboardProvider` równa preview (rzeczywista treść).
- BMP Mono/Color: C# **wyłącza** przycisk Copy; Rust no-op z komunikatem —
  `test_export_bmp_copy_clipboard_noop` (clipboard pozostaje pusty).
- Binary View: j.w. — `test_export_binary_view_copy_clipboard_noop`.

## 9. Golden Master results

| Golden | Wynik |
|---|---|
| `exports/font_default_mono.bmp` | PASS (afm_core + GUI byte-for-byte) |
| `exports/font_default_color.bmp` | PASS (afm_core + GUI byte-for-byte) |
| `exports/font_*.txt`, `view_*.txt`, `font_default.lst` | PASS (bez zmian) |

Żaden golden master **nie został zmieniony**.

## 10. Regression tests

`crates/afm_gui/tests/test_phase21b2_exporters.rs` (10 testów):
- BMP Mono: pusty font (normal+inverse stacking), pojedynczy piksel, pełny znak, wymiary wg selekcji.
- BMP Color: Mode 4/5/10, zmiana rejestru PF0 zmienia raster.
- Binary View: pusty ekran, różne znaki + transpose, aktywna strona, `line_fonts` nie wpływają na bajty.

`crates/afm_gui/src/controller.rs` (unit, +3): BMP copy no-op, Binary copy no-op, cancel dialogu.

`crates/afm_core/tests/test_exporters.rs` (+3): `export_view_binary` wierszowo / transponowany / klamping OOB.

## 11. Wszystkie znalezione problemy

| ID | Waga | Opis |
|---|---|---|
| E-1 | HIGH | BMP Mono nieosiągalny z GUI (brak opcji, brak zapisu). |
| E-2 | HIGH | BMP Color nieosiągalny z GUI. |
| E-3 | HIGH | Binary View — brak funkcji core + brak opcji + brak zapisu. |

## 12. Wszystkie wykonane poprawki

- `crates/afm_core/src/exporters/view_text.rs`: dodano `export_view_binary`.
- `crates/afm_core/src/exporters/mod.rs`: reeksport `export_view_binary`.
- `crates/afm_gui/src/state.rs`: dodano `export_font_bmp_bytes` (re-render atlasu + eksport)
  i `export_view_binary_bytes`.
- `crates/afm_gui/src/controller.rs`: indeksy formatu fontu 8=BMP Mono/9=BMP Color,
  indeks widoku 7=Binary Data; `font_export_file_meta` zwraca `FontN.bmp`/`Bitmap (*.bmp)`;
  gałęzie zapisu binarnego w `export_font_do_save`/`export_view_do_save`; no-op Copy dla
  formatów raster/binarnych; `font_selection_name` wg `MakeFilenamePartFromFontSelectionNr`.
- `crates/afm_gui/ui/components/export_font_modal.slint`: dodano „BMP Mono", „BMP Color".
- `crates/afm_gui/ui/components/export_view_modal.slint`: dodano „Binary Data".
- Testy: nowy plik integracyjny + testy unit controller + testy core.

## 13. Problemy pozostawione bez zmian (poza zakresem)

- Fontowy eksport binarny `.dat` (`FormatTypes.BinaryData = 9` w C#) i kompresja ZX0.
- Wybór regionu widoku myszą + offset (C# `_exportRegion`/`_exportOffset`) — Rust eksportuje
  pełny region 40×26, co jest domyślnym zachowaniem C# (`RememberSelection=false`).
- View line-font editing, legacy `.vf2/.vfn/.dat` — poza zakresem fazy.

## 14. Ograniczenia środowiska testowego

- Środowisko headless: fizyczne kliknięcia w GUI (Slint + native `rfd`) nie były wykonane.
  Weryfikację wykonano programowo: callbacki Slint → controller → state → eksporter →
  `std::fs::write` z injected `TestFileDialogs` (prawdziwy eksporter, nie mock).
- Kolejność pozycji list formatów różni się od C# (BMP/Binary na końcu listy zamiast
  pozycji 0/1) — decyzja o minimalnej zmianie, aby nie przesuwać istniejących indeksów/testów.

## 15. Exact verification commands

```
cargo fmt --all -- --check        # czysto
cargo check --workspace           # czysto
cargo test --workspace            # 208 passed / 0 failed / 0 ignored
cargo clippy --workspace -- -D warnings  # czysto
timeout 3 cargo run -p afm_gui    # exit 124 (proces żyje)
```

## 16. Final verdict

**PHASE 21B-2 — PASS**

**208 passed / 0 failed / 0 ignored**
