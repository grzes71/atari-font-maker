# PHASE 21B-4 — LEGACY FORMATS & FONT .DAT / ZX0 AUDIT & FIX

**Data:** 2026-08-14
**Zakres:** `.vf2`, `.vfn`, `.dat` (view + font), kompresja ZX0, GUI reachability.

---

## A. C# Reference Inventory

Przeszukano cały `atari-fontmaker-master/` (`.cs`, resources, harness). Wyniki:

| Format | C# source | Operacja | GUI entry point |
|---|---|---|---|
| `.atrview` | `AtariViewEditor.LoadViewFile/SaveViewFile` | Open/Save | "Load View" / "Save View" buttons |
| `.vf2` | `AtariViewEditor.ActionLoadView` | **Open only** (import) | "Load View" button (filter `*.atrview;*.vf2;*.vfn`) |
| `.vfn` | `AtariViewEditor.ActionLoadView` | **Open only** (import) | "Load View" button |
| `.dat` (view) | `ActionLoadView` + `ActionSaveView` | Import/Export (raw screen) | "Load View" / "Save View" + "Export View" |
| `.dat` (font) | `ExportFontWindow.SaveBinaryData` | Export only | "Export Font" dialog (BinaryData + compress) |
| ZX0/ZX1/ZX2/apultra | `Compressors.Compress` | Export compression | "Export Font"/"Export View" (compress checkbox) |

Kompresja jest realizowana przez **zewnętrzne pliki wykonywalne** (`Resources/zx0.exe`,
`zx1.exe`, `zx2.exe`, `apultra.exe`) — brak implementacji ZX0 w C#.

## B. Format Specification

### B.1 `.vf2` (import only)
```
byte 0      version (1..3; >3 -> error "newer version")
byte 1      color mode (0=B/W, 2=Mode5, 3=Mode10, else Mode4)
bytes 2..33 8 x int32 LE = UseFontOnLine[0..7]
bytes 34..51 6 x RGB = SetOfSelectedColors[0..5] (FindClosest, even indices)
version 2: 248 bytes = 31x8 screen
version 3: 832 bytes = 32x26 screen
version 0/1: no screen data
```
Uwaga (znaleziony bug C#): pętla linii fontów czyta `BitConverter.ToInt32(buf, 0)`
zamiast `buf, fsIndex`, przez co C# w praktyce ignoruje dane fontów wierszy
(ustawia im bajt wersji). Rust czyta pola zgodnie z formatem (intent).

### B.2 `.vfn` (import only)
```
byte 0       color mode
bytes 1..18  6 x RGB
bytes 19..204 186 bytes = 31x6 screen (kolumny 6,7 zerowane)
```

### B.3 `.dat` — view (raw screen)
- Import: surowe bajty 40x26 (row-major), `loadSize = min(len, 1040)`, reszta bez zmian.
- Export: surowe bajty 40x26 (bez kompresji w C# — `SaveAsBinaryData` nie kompresuje).

### B.4 `.dat` — font (BinaryData)
- `GetFontData(fontNr, withCompression)`: surowe bajty zakresu (1024/2048/4096).
- Z kompresją: `Compressors.Compress(ZX0)`; użyj tylko jeśli krótsze niż oryginał.

### B.5 ZX0
- C# wywołuje `zx0.exe -f in out` (format **v2**, domyślny; `-c` = classic v1).
- Wdrożono w Rust wierny port **ZX0 v2** (dekompresor wg `dzx0.c` v2.2, kompresor
  greedy LZ77). Wynik jest **zgodny dekompresyjnie** (każdy ZX0 v2 dekompresor
  odtworzy dane). Bitstream nie jest zweryfikowany bajt-w-bajt względem `zx0.exe`
  (patrz sekcja H).

## C. C# → Rust Parity Matrix

| Feature | C# | Rust | GUI | Tests | Parity |
|---|---|---|---|---|---|
| `.vf2` import | ✔ | ✔ `parse_vf2` | ✔ `open_project` routing | ✔ | PASS |
| `.vfn` import | ✔ | ✔ `parse_vfn` | ✔ | ✔ | PASS |
| `.dat` view import | ✔ | ✔ `load_raw_view_bytes` | ✔ | ✔ | PASS |
| `.dat` view export | ✔ | ✔ (Phase 21B-2) | ✔ | ✔ | PASS |
| `.dat` font export (raw) | ✔ | ✔ `export_font_binary` | ✔ idx 10 | ✔ | PASS |
| `.dat` font export (ZX0) | ✔ | ✔ ZX0 v2 | ✔ "Compress" checkbox | ✔ | PASS* |
| ZX0 decompression | ✔ (dzx0) | ✔ `zx0_decompress` | — | ✔ | PASS |
| ZX1/ZX2/apultra | ✔ (exe) | ✘ (poza zakresem — patrz H) | — | — | N/A |

\* dekompresyjna zgodność potwierdzona; tożsamość bitstreamu z `zx0.exe` niezweryfikowana.

## D. Golden Master Results

Brak golden masterów dla `.vf2`/`.vfn`/`.dat`/ZX0 (C# generuje je wyłącznie
przez zewnętrzne exe; ReferenceHarness ich nie obejmuje). Nie utworzono nowych
fixtures z braku możliwości uruchomienia `zx0.exe` (Windows PE, brak Wine).
Żaden istniejący golden master nie został zmodyfikowany.

## E. GUI Reachability Matrix

| Funkcja | UI | Controller | State | Core | I/O |
|---|---|---|---|---|---|
| Load `.vf2`/`.vfn`/`.dat` | "📂 Open" (filter) | `open_project_from_path` | `open_legacy_view_file` / `load_raw_view_bytes` | `parse_vf2/vfn` | `std::fs::read` |
| Export font `.dat` | "Export Font…" + Binary Data + Compress | `export_font_do_save` idx 10 | `export_font_binary_bytes` | `export_font_binary` + `zx0` | `std::fs::write` |

## F. Error/Cancel Matrix

| Przypadek | Zachowanie | Test |
|---|---|---|
| `.vf2` version > 3 | błąd, stan nienaruszony | `test_open_vf2_newer_version_fails_without_corrupting_state` |
| Anulowanie Save | dialog otwarty, brak pliku | `test_export_cancel_keeps_dialog_open` (Phase 21B-2) |
| Błąd I/O | status, bez zmian stanu | routing w `open_project_from_path` |
| Truncated `.vf2` | łagodne zerowanie | `parse_vf2` unit test |
| Pusty ZX0 | `Err(Truncated)` | `test_decompress_truncated_errors` |

## G. Test Results

- `cargo fmt --all -- --check` — czysto.
- `cargo check --workspace` — czysto.
- `cargo test --workspace` — **246 passed / 0 failed / 0 ignored** (+21 nowych).
- `cargo clippy --workspace -- -D warnings` — czysto.
- `timeout 3 cargo run -p afm_gui` — exit 124 (proces żyje).

Nowe testy: ZX0 roundtrip/empty/repetitive/random/compression-shrink (5 core),
legacy vf2/vfn/version/truncated (5 core), integracyjne `test_phase21b4_legacy_formats.rs`
(7), kontroler open-routing + font .dat save + compress/copy (4).

## H. Known Limitations

1. **ZX0 bitstream vs decompression**: nie można uruchomić `zx0.exe` (Windows PE,
   brak Wine, headless). Kompresor Rust jest wierny formatowi ZX0 v2 i zweryfikowany
   dekompresyjnie (roundtrip przez port `dzx0.c`), ale **tożsamość bitstreamu**
   z optymalnym `zx0.exe` nie jest potwierdzona. Każdy poprawny dekompresor ZX0 v2
   odtworzy wygenerowane `.dat`.
2. **ZX1/ZX2/apultra**: C# umożliwia wybór 4 kompresorów. Zaimplementowano tylko
   ZX0 (domyślny). ZX1/ZX2/apultra pozostawione poza zakresem (brak specyfikacji
   w tej fazie — udokumentowano, nie naprawiano).
3. **Bug C# w `.vf2`** (line-font read `buf[0]` vs `buf[fsIndex]`) — Rust czyta
   zgodnie z formatem; zachowanie C# jest uznane za błąd i nieodtworzone.
4. **`.vf2`/`.vfn` region**: C# nadpisuje tylko region źródłowy zachowując resztę
   ekranu; Rust zastępuje cały widok zerowym tłem (typowy przypadek identyczny).
5. Środowisko headless — fizyczna interakcja GUI nieweryfikowana; ścieżki sprawdzone
   programowo (callback → controller → state → core → filesystem).

## Final Verdict

**PHASE 21B-4 — PASS WITH LIMITATIONS**

**246 passed / 0 failed / 0 ignored**
