# PHASE 21B-11 — G-9 Export View Sub-Region Audit & Implementation Report

## 1. Executive Summary

| Phase | Target | Final Verdict | Dedicated Suite Tests | Workspace Tests | Clippy Warnings |
|---|---|---|---|---|---|
| **Phase 21B-11** | **G-9 Export View Sub-Region** | **PASS** | **22/22 PASS** | **100% PASS (All crates)** | **0 warnings** |

This phase completed the audit, implementation, and verification of **G-9 — Export View Sub-Region**, the final feature gap identified during the re-audit cycle. 

A rigorous analysis of the C# reference codebase (`atari-fontmaker-master/ExportViewWindow.cs`, `Configuration.cs`, `FontMakerForm.cs`) established the exact semantics for sub-region bounding, row-major vs column-major (`transpose`) ordering, number base formatting (`decimal` vs `hexadecimal`), text memo preview generation, clipboard copy, binary file export (`.dat`), active page isolation, and configuration persistence (`ExportViewRemember`).

---

## 2. C# Source References & Line Numbers

- **`ExportViewWindow.cs:11-23`**: Constants `EXPORT_WIDTH = 40`, `EXPORT_HEIGHT = 26`, `FullViewRegion = new(0, 0, 40, 26)`.
- **`ExportViewWindow.cs:32-42`**: `FormatTypes` enum (`BinaryData = 0`, `Assembler`, `Action`, `AtariBasic`, `FastBasic`, `MADSdta`, `CDataArray`, `MadPascalArray`).
- **`ExportViewWindow.cs:93-116`**: `LoadConfiguration` and `SaveConfiguration` for `RememberSelection`, `PreviousExportType`, `PreviousDataType`, `_exportRegion`, `_exportOffset`, `PreviousTransposeFlag`.
- **`ExportViewWindow.cs:176-204`**: Initialization and `UpdateRegionEdits()` formatting `labelDimensions.Text = $"({_exportRegion.X}, {_exportRegion.Y}) - ({_exportRegion.Width}, {_exportRegion.Height}) @ {(_exportRegion.Width * _exportRegion.Height)} bytes"`.
- **`ExportViewWindow.cs:347-381`**: `GetExportData(Rectangle exportRegion, bool transpose, bool withCompression)` iterating row-major when `!transpose` and column-major when `transpose`.
- **`ExportViewWindow.cs:390-573`**: `GenerateFileAsText` generating headers, 8 items per line, decimal/hex formatting, and language footers.
- **`ExportViewWindow.cs:744-798`**: Numeric spinners for `FromX`, `FromY`, `Width`, `Height` enforcing bounds `X + Width <= 40` and `Y + Height <= 26`.
- **`ExportViewWindow.cs:810-815`**: `buttonResetSelection_Click` resetting `_exportRegion = new Rectangle(0, 0, 40, 26)`.
- **`ExportViewWindow.cs:838-851`**: `ButtonCopyClipboard_Click` copying preview text to clipboard.
- **`ExportViewWindow.cs:853-916`**: `ButtonExport_Click` and `SaveAsBinaryData` writing raw bytes to `.dat` or text to `.txt`.
- **`Configuration.cs:134-141` & `166-181`**: Syncing `ExportViewRemember`, `ExportViewRegionX`, `Y`, `W`, `H`, `ExportViewTranspose`, `ExportViewExportType`, `ExportViewDataType`.

---

## 3. Region Semantics & Boundary Behavior

- **Full View**: 40 × 26 characters (1040 bytes total).
- **Sub-Region Representation**: Defined by `ViewExportRegion { rx, ry, rw, rh }`.
- **Coordinate Boundaries**:
  - `rx`: `0..=39`
  - `ry`: `0..=25`
  - `rw`: `1..=(40 - rx)`
  - `rh`: `1..=(26 - ry)`
- **Clamping Rules**:
  - When `rx` changes, if `rx + rw > 40`, `rw` is clamped to `40 - rx`.
  - When `ry` changes, if `ry + rh > 26`, `rh` is clamped to `26 - ry`.
  - When `rw` or `rh` are requested out of bounds, they are clamped to `max(1, remaining)`.
- **Reset Selection / Full View**: Restores `(0, 0, 40, 26)`.

---

## 4. Transpose Semantics

- **`transpose == false` (Row-Major)**:
  - Outer loop iterates row `y` from `ry` to `ry + rh - 1`.
  - Inner loop iterates column `x` from `rx` to `rx + rw - 1`.
  - Byte index: `y * 40 + x`.
- **`transpose == true` (Column-Major)**:
  - Outer loop iterates column `x` from `rx` to `rx + rw - 1`.
  - Inner loop iterates row `y` from `ry` to `ry + rh - 1`.
  - Byte index: `y * 40 + x`.

### Reference Deterministic Grid Check (Prompt Item 17)
Given `cell(x, y) = (y * 40 + x) % 256` and region `x=10, y=5, width=3, height=2`:
- Row 5: `(10,5)=210`, `(11,5)=211`, `(12,5)=212`
- Row 6: `(10,6)=250`, `(11,6)=251`, `(12,6)=252`
- **`transpose == false`**: `[210, 211, 212, 250, 251, 252]`
- **`transpose == true`**: `[210, 250, 211, 251, 212, 252]`
- Verified by automated regression tests `test_export_region_reference_deterministic_grid` and `test_export_region_reference_deterministic_grid_transposed`.

---

## 5. Export Format Matrix

| Format ID | Name | Extension | Output Type | Comment Header | Line Layout | Hex Format |
|---|---|---|---|---|---|---|
| 0 | Assembler | `.txt` | Text | `\t; Size: {N} bytes` | `\t.BYTE d0,d1,d2...` (8/line) | `$XX` |
| 1 | Action! | `.txt` | Text | `; Size: {N} bytes` | `PROC VIEW=*()\r\n[\r\nd0 d1 ...\n]\nMODULE\n` | `$XX` |
| 2 | Atari BASIC | `.txt` | Text | `10000 REM *** DATA VIEW ***\r\n10001 REM Size: {N} bytes` | `10010 DATA d0,d1...` (+10/line) | `$XX` |
| 3 | FastBasic | `.txt` | Text | `` ` Size: {N} bytes `` | `data view() byte = d0,d1...` | `$XX` |
| 4 | MADS .dta | `.txt` | Text | `\t; Size: {N} bytes` | `\tdta d0,d1,d2...` | `$XX` |
| 5 | C Data Array | `.txt` | Text | `// Size: {N} bytes` | `{\n\td0,d1,d2...\n}` | `0xXX` |
| 6 | Mad-Pascal Array | `.txt` | Text | `// Size: {N} bytes` | `data: array [0..N-1] of byte = (\n\td0,d1...\n);\n` | `$XX` |
| 7 | Binary Data | `.dat` | Binary | None (Raw bytes) | Exact `rw * rh` bytes | Raw binary |

---

## 6. GUI Reachability & User Controls

The `ExportViewModal` (`export_view_modal.slint`) now provides full GUI controls matching C#:
- **From X SpinBox**: range `0..39`
- **From Y SpinBox**: range `0..25`
- **Width SpinBox**: range `1..40`
- **Height SpinBox**: range `1..26`
- **Full View Button**: resets coordinates to `(0, 0, 40, 26)`
- **Dimensions Badge**: live updates `(X, Y) - (W, H) @ N bytes`
- **Format ComboBox**: 8 formats (Assembler, Action!, Atari BASIC, FastBasic, MADS .dta, C Data Array, Mad-Pascal Array, Binary Data)
- **Data Type ComboBox**: Decimal vs Hexadecimal
- **Transpose CheckBox**: Columns first toggle
- **Live Preview Memo**: displays real-time formatted source code
- **Copy to Clipboard Button**: copies preview memo to system clipboard
- **Save to File Button**: native file dialog saving `.txt` or `.dat`
- **Close Button (✕)**: dismisses modal and persists configuration if `Remember` is enabled

---

## 7. Active Page, Persistence & Dirty Tracking

- **Active Page**: View export always samples the currently active page in `project.view_bytes`. Switching pages immediately invalidates and updates the export data.
- **Project File (`.atrview`)**: Sub-region coordinates are UI/exporter state and are **never** serialized to `.atrview`.
- **Configuration File (`FontMaker.json`)**: If `ExportViewRemember` is true, region coordinates `(X, Y, W, H)`, format, data type, and transpose are remembered across sessions.
- **Dirty State**: Exporting, previewing, changing regions, or copying to clipboard does **not** dirty the project state (`is_dirty` remains unchanged).

---

## 8. Bugs Found and Fixed During G-9 Implementation

### BUG-G9-1: Hardcoded Full-View Region in Controller
- **Severity**: HIGH
- **C# Behavior**: `ExportViewWindow` allows selecting arbitrary sub-regions `(X, Y, W, H)` and exports only that sub-region in binary or text formats.
- **Rust Behavior**: `compute_view_export_text` and `export_view_do_save` hardcoded `ViewExportRegion::full_standard()`.
- **Root Cause**: Sub-region properties and controller state fields were missing.
- **Fix**: Added `export_view_rx`, `ry`, `rw`, `rh` to `GuiController`, dynamic calculation in `current_view_export_region()`, and wired UI callbacks.
- **Regression Test**: `test_export_region_middle`, `test_export_region_1x1`, `test_export_region_reference_deterministic_grid`.

### BUG-G9-2: Missing Region Controls in Slint `ExportViewModal`
- **Severity**: HIGH
- **C# Behavior**: `ExportViewWindow` has `numericFromX`, `numericFromY`, `numericWidth`, `numericHeight`, `buttonResetSelection`, and `labelDimensions`.
- **Rust Behavior**: `ExportViewModal.slint` had no controls for region selection.
- **Root Cause**: Incomplete Slint modal component.
- **Fix**: Implemented full region controls layout with SpinBoxes, Full View reset button, dimensions badge, and Slint property/callback bindings.
- **Regression Test**: `test_phase21b11_export_region.rs` (all 22 tests).

---

## 9. Adversarial Test Suite: `test_phase21b11_export_region.rs`

22 comprehensive tests covering all requirements:
1. `test_export_region_full_view`: PASS
2. `test_export_region_1x1`: PASS
3. `test_export_region_1xn_horizontal_strip`: PASS
4. `test_export_region_nx1_vertical_column`: PASS
5. `test_export_region_middle`: PASS
6. `test_export_region_top_left_corner`: PASS
7. `test_export_region_bottom_right_corner`: PASS
8. `test_export_region_boundary_clamping`: PASS
9. `test_export_region_reference_deterministic_grid`: PASS
10. `test_export_region_reference_deterministic_grid_transposed`: PASS
11. `test_export_region_transpose_off_vs_on`: PASS
12. `test_export_region_binary_data`: PASS
13. `test_export_region_assembler_format`: PASS
14. `test_export_region_action_format`: PASS
15. `test_export_region_atari_basic_format`: PASS
16. `test_export_region_all_remaining_text_formats`: PASS
17. `test_export_region_decimal_vs_hexadecimal`: PASS
18. `test_export_region_clipboard_copy`: PASS
19. `test_export_region_preview_matches_save_and_clipboard`: PASS
20. `test_export_region_active_page_isolation`: PASS
21. `test_export_region_does_not_dirty_project`: PASS
22. `test_export_region_configuration_remember_roundtrip`: PASS

---

## 10. Verification Commands & Results

```bash
cargo fmt --all -- --check
# Result: Exit code 0 (clean formatting)

cargo check --workspace
# Result: Exit code 0 (clean build)

cargo test --workspace
# Result: Exit code 0 (all test suites pass across entire workspace)

cargo clippy --workspace -- -D warnings
# Result: Exit code 0 (0 warnings)

timeout 3 cargo run -p afm_gui
# Result: Exit code 124 (clean binary launch and timeout shutdown)
```

---

## 11. Final Verdict

```text
PHASE 21B-11 — PASS

Tests: 22 passed / 0 failed (Phase 21B-11 suite)
       All workspace unit, integration, and golden tests PASS
New tests: 22
HIGH findings: 0 (2 fixed during implementation)
MEDIUM findings: 0
LOW findings: 0
Unverified: 0

Report:
docs/phase-21b11-g9-export-region-audit-report.md
```
