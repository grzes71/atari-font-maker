# GUI Layout Reconstruction Report — Atari FontMaker (C# WinForms → Rust + Slint)

> Analysis of the **visual layout only**, derived from the C# source of truth
> (`atari-fontmaker-master/`), correlated with the current Rust + Slint port
> (`crates/afm_gui/ui/`). This is a specification, **not** an implementation.
> No production code was changed.

Date: 2026-08-16
Status: COMPLETE (analysis)

---

## 1. Executive Summary

The original C# application (`FontMakerForm`) is **not** built from modern
layout containers. There is:

- **No `SplitContainer`**
- **No `TableLayoutPanel` / `FlowLayoutPanel` / `GroupBox` / `TabControl`**
- **No `Dock`** and (with one exception) **no `Anchor`**
- **No `AutoSize` layout chains** — only the tiny `AutoSize = true` on plain
  `Label`s

Instead, **every control is placed with absolute `Location` + `Size`** directly
on the form or inside one of a handful of `Panel` containers (each with
`BorderStyle = FixedSingle`). The "left/right split" seen in screenshots is
purely the **sum of absolute coordinates**: the left column ends at x ≈ 515 and
the view area begins at x = 520.

The window is **fixed-size**: `FormBorderStyle = FixedSingle`, `MaximizeBox =
false`. The user cannot resize the window; the only dynamic dimension change is
the **view width**, driven by the "32/40/48 Bytes" combo box (`comboBoxBytes`),
which changes `Form.Width` and repositions the scrollbars.

The code's own header comment divides the form into **6 sections**:

```text
+--------+------------------+----------+--------------------------------------+
| A      | B                | C        | D                                    |
| New    | Character Editor | GFX mode | View/Screen editor                   |
| Load...|                  | Recolor  | Paging                               |
+--------+------------------+----------|                                      |
| E                                    |                                      |
| Undo/Redo/Duplicate/MegaCopy/FontBank|                                      |
+--------------------------------------|                                      |
| F                                    |                                      |
| Font selector                        |                                      |
+--------------------------------------+--------------------------------------+
```

- **A** = `p_hh` — New/Load/Save/Save-As/Clear/About/Quit
- **B** = `p_xx` — Character Editor (8×8 pixel grid + transform buttons)
- **C** = `p_zz` — GFX mode, color palette, color sets, Recolor, Export font
- **D** = right side — 40×26 (or 48×26) Atari View editor + view controls/paging
- **E** = `p_status` — Show Duplicates, MegaCopy, Font Bank, font/copy-area ops
- **F** = `pictureBoxFontSelector` — 512×256 glyph atlas (2 fonts per bank)

The current Rust + Slint port uses a completely different structure: a
resizable window with a menu bar, a toolbar, a MegaCopy strip, a two-column
workspace, a palette bar, and a status bar. Functionality has been audited
before, but the **visual layout is a re-invention**, not a port.

---

## 2. Main Window Hierarchy

Reconstructed from `FontMakerForm.Designer.cs` (`Controls.Add` order) plus the
`Panel.Controls.Add` lists. Parentheses denote a child of the preceding
container.

```text
FontMakerForm  (Form, FixedSingle, MaximizeBox=false, ClientSize 1325×572 @design)
├── p_hh                     Panel, BorderStyle=FixedSingle          → Section A
│   ├── buttonNew            "New"
│   ├── buttonLoadFont1      "Load 1"   (↔ "Load 3")
│   ├── buttonLoadFont2      "Load 2"   (↔ "Load 4")
│   ├── buttonSaveFont1      "Save 1"   (↔ "Save 3")
│   ├── buttonSaveFont2      "Save 2"   (↔ "Save 4")
│   ├── buttonSaveFont1As    "as..."
│   ├── buttonSaveFont2As    "as..."
│   ├── buttonClearFont1     "Clear 1"  (↔ "Clear 3")
│   ├── buttonClearFont2     "Clear 2"  (↔ "Clear 4")
│   ├── buttonAbout          "About"
│   └── buttonQuit           "Quit"
│
├── p_xx                     Panel, FixedSingle                     → Section B
│   ├── label4               "Undo/Redo"
│   ├── label3               "Write Mode"
│   ├── Bevel3               Panel, FixedSingle
│   │   ├── pictureBoxCharacterEditor      160×160
│   │   ├── pictureBoxClipboardPreview     (hidden overlay)
│   │   └── labelCopyAreaInfo              (hidden)
│   ├── pictureBoxCharacterEditorColor1    color swatch (Tag=3)
│   ├── pictureBoxCharacterEditorColor2    color swatch (Tag=4)
│   ├── buttonRotateLeft      "ROL"
│   ├── buttonMirrorHorizontal "H MIR"
│   ├── buttonShiftLeft       "SHL"
│   ├── buttonShiftDown       "SHD"
│   ├── buttonRestoreDefault  "RES D"
│   ├── buttonRestoreSaved    "RES S"
│   ├── buttonCopy            "CPY"
│   ├── buttonRotateRight     "ROR"
│   ├── buttonMirrorVertical  "V MIR"
│   ├── buttonShiftRight      "SHR"
│   ├── buttonShiftUp         "SHU"
│   ├── buttonInverse         "INV"
│   ├── buttonClear           "CLR"
│   ├── buttonPaste           "PST"
│   ├── pictureBoxActionColor  active-color swatch
│   ├── labelEditCharInfo     "Font 1\n$00 #0"
│   ├── labelColor            "Color:"
│   ├── cmbColor9Menu         owner-drawn color combo (Mode 10 only, hidden)
│   ├── comboBoxWriteMode     "Rewrite"/"Insert"
│   ├── buttonUndo            (22×22 icon)
│   └── buttonRedo            (22×22 icon)
│
├── p_zz                     Panel, FixedSingle                     → Section C
│   ├── cmbColorMode          "Mode 4/5/10" combo
│   ├── comboBoxColorSets     "Project colors"/"Alt colors N"
│   ├── Bevel4                Panel, FixedSingle
│   │   └── pictureBoxPalette  90×90 (2×3 or 2×5 swatches)
│   ├── buttonShowColorSwitchSetup  (22×22 gear)
│   ├── buttonSwitchGraphicsMode    "Change GFX"
│   ├── buttonExportFont            "Export font"
│   └── buttonRecolor               "Recolor" (starts disabled)
│
├── p_status                 Panel                                → Section E
│   ├── label2                "Font Bank:"
│   ├── buttonFontDeleteCharShiftRight
│   ├── buttonFontDeleteCharShiftLeft
│   ├── buttonCopyAreaRotateRight / RotateLeft / Invert / VMirror / HMirror
│   ├── buttonCopyAreaShiftDown / ShiftUp / ShiftRight / ShiftLeft
│   ├── buttonFontShiftRightInsert / Rotate / LeftRotate / LeftInsert
│   ├── comboBoxPasteIntoFontNr    "1".."4"
│   ├── buttonPasteInPlace         "Paste to Font 1"
│   ├── checkBoxFontBank           (button-style, 12/34 image)
│   ├── checkBoxShowDuplicates     "Show Duplicates"
│   └── buttonMegaCopy             "Mega Copy" (CheckBox styled as button)
│
├── buttonTileSetEditor      "Tile Set Editor" (directly on form)
├── pictureBoxFontSelector   512×256                             → Section F
│   └── pictureBoxDuplicateIndicator (20×20, hidden, purple)
│
├── pictureBoxAtariView      768×416                              → Section D
├── pictureBoxCharacterSetSelector  15×416 (line-font strip)
├── pictureBoxAbout          512×128 (splash, hidden)
│
├── labelViewCharInfo        "Char: Font1 $00 #0"
├── labelOffsets             "TL: [0,0]-[39,25]"
├── labelSelectedArea        "Area:"
├── label5                   "Undo/Redo"
│
├── buttonExportView         "Export View"
├── buttonImportView         "Import View"
├── buttonClearView          "Clear View"
├── buttonLoadView           "Load View"
├── buttonSaveView           "Save View"
├── buttonConfigure          (28×28 gear, right of view buttons)
├── buttonEnterText          "Enter text"
├── buttonViewActions        "View Actions"
├── buttonFontAnalysis       "Analyse"
├── buttonViewUndo / buttonViewRedo   (22×22 icons)
│
├── panel1                   Panel, FixedSingle                   (paging block)
│   ├── labelPageSize        "40 x 26"
│   ├── buttonConfigurePage  (28×28 gear)
│   ├── label6               "Size:"
│   ├── label1               "Pages:"
│   ├── labelCurrentPageInfo "1 of 1"
│   ├── comboBoxPages
│   ├── buttonAddPage        "+"
│   ├── buttonDeletePage     "-"
│   └── buttonEditPage       "Edit"
│
├── checkBoxSkipChar0        "Skip char #0 on copy"
├── trackBarSkipCharX        0..255
├── checkBoxStayInPasteMode  "Stay in Paste Mode"
├── comboBoxBytes            "32 Bytes"/"40 Bytes"/"48 Bytes"
├── hScrollBar               (view horizontal scroll)
├── vScrollBar               (view vertical scroll)
│
├── panelColorSwitcher       153×97  (recolor popup, hidden)
├── panelColorSwitcherMode10 153×161 (recolor popup Mode 10, hidden)
├── pictureBoxFontSelectorMegaCopyImage / RubberBand / PasteCursor (overlays)
├── pictureBoxViewEditorMegaCopyImage / RubberBand / PasteCursor   (overlays)
└── lblInMegaCopyMode        "In MegaCopy Mode" (Bottom|Right, hidden)
```

Container order in `Controls.Add` (z-order, reverse of paint order) is
`comboBoxBytes … p_hh, p_zz, …, p_status, …, panel1` — all direct children of
the form.

---

## 3. Main Split Geometry (Left vs Right)

**There is no splitter.** The "split" is the sum of absolute coordinates.

| Fact | Value | Source |
|---|---|---|
| Split mechanism | None (absolute coordinates) | `Designer.cs` |
| Left column width | 0 … ~515 (effectively 520) | `p_status` at `(-1,238) 515×54`; `p_zz` right edge = 514 |
| View area start | x = 520 (line strip), x = 536 (Atari view) | `pictureBoxCharacterSetSelector` (520), `pictureBoxAtariView` (536) |
| Gap between columns | 520 − 514 = 6 px (visual margin) | — |
| Orientation | Left column / right view (vertical seam) | — |
| Splitter movable | **No** (fixed form) | `FormBorderStyle=FixedSingle`, `MaximizeBox=false` |
| Dock/Anchor | None (except `lblInMegaCopyMode` = Bottom\|Right) | `Designer.cs` |
| What stretches | **Nothing** — fixed size | `MaximizeBox=false` |
| Right side own layout | Yes — absolute controls below/around the view | section 5–7 |

Window width is dynamic (not user-resizable), driven by `comboBoxBytes`:

| comboBoxBytes | Form.Width | ClientWidth* | vScrollBar at | hScrollBar.Width |
|---|---|---|---|---|
| 32 Bytes | 1082 | 1066 | (1049, 0) | 545 |
| 40 Bytes (default) | 1210 | 1194 | (1177, 0) | 673 |
| 48 Bytes | 1341 | 1325 | (1306, 0) | 801 |

\* ClientWidth = Width − 16 (FixedSingle chrome). The Designer's
`ClientSize = 1325×572` therefore corresponds to the **48-byte** geometry even
though the app's default at startup is **40 bytes** (from `default.atrview`,
`"FortyBytes":"1"`).

Client height is constant **572** in all modes. The dead field
`AppWidth = {1210−128, 1210, 1353+128}` is declared but never used.

---

## 4. Left-side Layout

Three fixed-size bordered panels sit in a row at the top (y = 0 … 234), a status
panel spans the full width just below (y = 238 … 292), a Tile-Set-Editor button
sits between the first panel and the font selector, and the font selector fills
the bottom (y = 298 … 554).

```text
x:   0    105  113       401 409     513 515
     ┌─────┬───────────────┬───────┬────┐
  0  │ A   │ B             │ C     │    │
     │p_hh │ p_xx          │ p_zz  │    │  height 234
     └─────┴───────────────┴───────┴────┘
     ┌──────────────────────────────────┐
 238 │ E (p_status) 515×54              │
     └──────────────────────────────────┘
     ┌─────┐
 181?│Tile │  ← buttonTileSetEditor (3,181) 104×51
     └─────┘
     ┌──────────────────────────────────┐
 298 │ F (pictureBoxFontSelector) 512×256│
     │                                  │
 554 └──────────────────────────────────┘
```

### 4.1 Section A — `p_hh` (File / Project)

- **Type**: `Panel`, `BorderStyle=FixedSingle`
- **Location/Size**: (1, 0), 105 × 177
- **Dock/Anchor**: none
- **Contents** (all absolute, inside panel):
  - `buttonNew` "New" — (7, 8), 90×20
  - `buttonLoadFont1` "Load 1" — (3, 35), 50×20
  - `buttonLoadFont2` "Load 2" — (52, 35), 50×20
  - `buttonSaveFont1` "Save 1" — (3, 61), 50×20
  - `buttonSaveFont2` "Save 2" — (52, 61), 50×20
  - `buttonSaveFont1As` "as..." — (3, 80), 50×20
  - `buttonSaveFont2As` "as..." — (52, 80), 50×20
  - `buttonClearFont1` "Clear 1" — (3, 106), 50×20
  - `buttonClearFont2` "Clear 2" — (52, 106), 50×20
  - `buttonAbout` "About" — (8, 130), 89×20
  - `buttonQuit` "Quit" — (8, 154), 89×20
- Dynamic: font-bank toggle renames Load/Save/Clear "1/2" ↔ "3/4".

### 4.2 Section C — `p_zz` (GFX / Colors)

- **Type**: `Panel`, `BorderStyle=FixedSingle`
- **Location/Size**: (409, 0), 105 × 234
- **Contents**:
  - `buttonSwitchGraphicsMode` "Change GFX" — (1, 6), 99×25
  - `cmbColorMode` — (2, 33), 98×21 ("Mode 4 (5 cols)", "Mode 5 (5 cols)", "Mode 10 (9 cols)")
  - `Bevel4` Panel FixedSingle — (5, 58), 92×92
    - `pictureBoxPalette` — (0, 0), 90×90
  - `comboBoxColorSets` — (5, 154), 92×21
  - `buttonRecolor` "Recolor" — (5, 177), 64×25 (initially `Enabled=false`)
  - `buttonShowColorSwitchSetup` — (72, 179), 22×22 gear (initially `Enabled=false`)
  - `buttonExportFont` "Export font" — (5, 204), 92×25

### 4.3 Section E — `p_status` (Status / Font ops / MegaCopy)

- **Type**: `Panel` (no border)
- **Location/Size**: (−1, 238), 515 × 54
- **Contents**:
  - `checkBoxShowDuplicates` "Show Duplicates" — (8, 6)
  - `buttonMegaCopy` "Mega Copy" — (159, 2), 216×21 (`CheckBox`, `Appearance=Button`, centered)
  - `label2` "Font Bank:" — (404, 6)
  - `checkBoxFontBank` — (469, 1), 46×22 (button-style; image 12↔34)
  - Icon button row at y = 27, all 24×24, ordered left→right:
    1. `buttonFontShiftLeftInsert` (4)
    2. `buttonFontShiftLeftRotate` (28)
    3. `buttonFontDeleteCharShiftRight` (52)
    4. `buttonFontDeleteCharShiftLeft` (76)
    5. `buttonFontShiftRightRotate` (100)
    6. `buttonFontShiftRightInsert` (124)
    7. `buttonCopyAreaShiftLeft` (159)
    8. `buttonCopyAreaShiftRight` (183)
    9. `buttonCopyAreaShiftUp` (207)
    10. `buttonCopyAreaShiftDown` (231)
    11. `buttonCopyAreaHMirror` (255)
    12. `buttonCopyAreaVMirror` (279)
    13. `buttonCopyAreaInvert` (303)
    14. `buttonCopyAreaRotateLeft` (327)
    15. `buttonCopyAreaRotateRight` (351)
  - `buttonPasteInPlace` "Paste to Font 1" — (378, 27), 104×24
  - `comboBoxPasteIntoFontNr` "1".."4" — (483, 29), 30×21
- Dynamic: `buttonPasteInPlace.Text` becomes "Paste to Font N"; copy-area buttons
  (`buttonCopyArea*`, `buttonPasteInPlace`, `comboBoxPasteIntoFontNr`) are
  disabled unless a copy area is selected.

### 4.4 `buttonTileSetEditor`

- Direct child of the form (not inside `p_hh`).
- Location/Size: (3, 181), 104 × 51; text "Tile Set Editor".

### 4.5 Section F — `pictureBoxFontSelector`

- Direct child of the form. Location/Size: **(2, 298), 512 × 256**.
- Child: `pictureBoxDuplicateIndicator` at (0, 9), 20×20, hidden.
- Full detail in section 8.

---

## 5. Right-side Layout (View + controls)

Everything is absolute; the right side occupies x ≥ 520.

### 5.1 View rendering surface

- `pictureBoxAtariView` — **Location (536, 0), Size 768 × 416**.
  - 768 = 48 chars × 16 px; 416 = 26 lines × 16 px (CellHeight 16; 32 in Mode 5).
  - Rendering clears to `AtariPalette[SetOfSelectedColors[1]]` (background), then
    blits each character cell 16×16 (NearestNeighbor) from `BitmapFontBanks`.
  - Visible width = `GetActualViewWidth()` = 32 / 40 / 48 chars; the rest of the
    768-wide surface is left cleared.
- `pictureBoxCharacterSetSelector` — **Location (520, 0), Size 15 × 416**.
  - The 1–4 "font per line" strip immediately to the **left** of the view.
  - White background, black text drawn with the form font at `(4, 2 + line*CellHeight)`,
    plus a horizontal separator line at `y = 24 * CellHeight`.

### 5.2 Scrollbars

- `hScrollBar` — (520, 417), height 17; width 673 (40-byte) / 545 / 801.
  `LargeChange = 1`; `Maximum = max(0, viewWidth − visibleWidth)`.
- `vScrollBar` — 17 × 416; x depends on mode: 1177 (40B) / 1049 (32B) / 1306 (48B).
  `Maximum = Height − (CellHeight == 16 ? 26 : 13)`.

### 5.3 Info labels row (y = 436)

- `labelViewCharInfo` "Char: Font1 $00 #0" — (520, 436)
- `labelOffsets` "TL: [0,0]-[39,25]" — (711, 436)
- `labelSelectedArea` "Area:" — (863, 436)

### 5.4 View action buttons (y = 454)

Left → right, each 86×23:

- `buttonExportView` "Export View" — (520, 454)
- `buttonImportView` "Import View" — (613, 454)
- `buttonClearView` "Clear View" — (706, 454)
- `buttonLoadView` "Load View" — (799, 454)
- `buttonSaveView` "Save View" — (892, 454)
- `buttonConfigure` (gear) — (990, 452), 28×28

### 5.5 Lower controls

- `buttonEnterText` "Enter text" — (520, 481), 86×23 (enabled only in MegaCopy)
- `label5` "Undo/Redo" — (810, 483)
- `checkBoxSkipChar0` "Skip char #0 on copy" — (898, 483)
- `buttonViewUndo` — (820, 499), 22×22
- `buttonViewRedo` — (843, 499), 22×22
- `trackBarSkipCharX` — (892, 500), 284×34, 0..255, TickFrequency 10
- `buttonViewActions` "View Actions" — (520, 508), 86×23
- `checkBoxStayInPasteMode` "Stay in Paste Mode" — (812, 527)
- `buttonFontAnalysis` "Analyse" — (520, 535), 85×23
- `comboBoxBytes` "32/40/48 Bytes" — (810, 546), 67×21
- `lblInMegaCopyMode` — (1139, 547), Anchor **Bottom|Right**, bold 14.25pt,
  `SystemColors.ActiveCaption` background, hidden

### 5.6 Paging block — `panel1`

- `Panel`, `BorderStyle=FixedSingle`, **Location (613, 483), Size 191 × 88**.
- Contents:
  - `label1` "Pages:" — (7, 9)
  - `labelCurrentPageInfo` "1 of 1" — (53, 9)
  - `buttonEditPage` "Edit" — (131, 3), 53×21
  - `comboBoxPages` — (7, 33), 121×21
  - `buttonAddPage` "+" — (131, 33), 25×21
  - `buttonDeletePage` "−" — (159, 33), 25×21
  - `label6` "Size:" — (8, 62)
  - `labelPageSize` "40 x 26" — (41, 62)
  - `buttonConfigurePage` (gear) — (156, 57), 28×28

### 5.7 Floating overlays (all on the form, hidden by default)

- `panelColorSwitcher` — (544, 8), 153×97 (mode 4/5 recolor: two 65×56 ListBoxes
  "BAK/PF0/PF1/PF2", two color swatches 49×17)
- `panelColorSwitcherMode10` — (696, 8), 153×161 (mode 10: two 65×121 ListBoxes
  "Color 0..8", two 65×17 swatches); at runtime `FormCreate` aligns its location
  to `panelColorSwitcher.Location`
- `pictureBoxViewEditorMegaCopyImage` — (560, 296), 105×105, hidden
- `pictureBoxViewEditorRubberBand` — cyan hollow rect, hidden
- `pictureBoxViewEditorPasteCursor` — yellow hollow rect, hidden
- `pictureBoxAbout` — 512×128 splash shown at `pictureBoxAtariView.Left`
  (= 536), initially at (536, 136), auto-hides after 5 s

---

## 6. Character Editor (`p_xx`, Section B)

### 6.1 Container

- `Panel`, `BorderStyle=FixedSingle`, **Location (113, 0), Size 289 × 234**.
- The `Character Editor` and `Font Selector` are **independent**: the editor is
  the bordered `p_xx` panel; the font selector is a separate `PictureBox` far
  below. They are not one container.

### 6.2 Pixel grid

- `Bevel3` Panel (FixedSingle) at (63, 5), 162×162.
- `pictureBoxCharacterEditor` at (0, 0), **160 × 160** = 8×8 pixels @ **20 px**.
- Mono mode: each pixel is a 20×20 cell filled with foreground
  (`BrushCache[0]`) or background (`BrushCache[1]`) plus a 1×1 white dot at each
  cell corner (grid marker).
- Mode 4/5: 4 columns × 8 rows, cell = `CharXWidth × 20` (40×20 normal,
  20×20 in Mode 5).
- Mode 10: 2 columns × 8 rows, cell = `CharXWidth*2 × 20` (80×20).
- Mouse: `ry = e.Y/20`, `rx = e.X/20` (mono) or `e.X/CharXWidth` (color).
- Overlays (children of `Bevel3`, hidden): `pictureBoxClipboardPreview` (0,15),
  160×145; `labelCopyAreaInfo` "Copy Area:" (0,0).

### 6.3 Left button column (x = 8, width 49, height 20 each)

| y | Button | Text |
|---|---|---|
| 6 | `pictureBoxCharacterEditorColor1` (49×17 swatch) | color 1 (Tag 3) |
| 26 | `buttonRotateLeft` | ROL |
| 47 | `buttonMirrorHorizontal` | H MIR |
| 68 | `buttonShiftLeft` | SHL |
| 89 | `buttonShiftDown` | SHD |
| 110 | `buttonRestoreDefault` | RES D |
| 131 | `buttonRestoreSaved` | RES S |
| 152 | `buttonCopy` | CPY |

### 6.4 Right button column (x = 232, width 49, height 20 each)

| y | Button | Text |
|---|---|---|
| 6 | `pictureBoxCharacterEditorColor2` (49×17 swatch) | color 2 (Tag 4) |
| 26 | `buttonRotateRight` | ROR |
| 47 | `buttonMirrorVertical` | V MIR |
| 68 | `buttonShiftRight` | SHR |
| 89 | `buttonShiftUp` | SHU |
| 110 | `buttonInverse` | INV |
| 131 | `buttonClear` | CLR |
| 152 | `buttonPaste` | PST |

### 6.5 Below the grid (y ≈ 170–227)

- `pictureBoxActionColor` — (115, 172), 49×17 (active color)
- `labelColor` "Color:" — (73, 173)
- `cmbColor9Menu` — (114, 170), 77×23, owner-drawn, hidden (Mode 10 only)
- `comboBoxWriteMode` — (114, 200), 77×21 ("Rewrite", "Insert")
- `label4` "Undo/Redo" — (3, 188)
- `label3` "Write Mode" — (63, 197)
- `labelEditCharInfo` "Font 1\n$00 #0" — (212, 188), 69×29
- `buttonUndo` — (12, 204), 22×22
- `buttonRedo` — (35, 204), 22×22

Dynamic: in **MegaCopy mode** `pictureBoxCharacterEditor` is hidden and
`pictureBoxClipboardPreview` becomes visible; all 18 buttons in
`ActionListNormalModeOnly` are disabled. In **Mode 10** the two color swatches
and the action color are hidden and `cmbColor9Menu` is shown.

---

## 7. Color / Palette Area (Section C detail)

`p_zz` (409,0) 105×234 holds all color functionality. The palette itself:

- `pictureBoxPalette` 90×90 inside `Bevel4` (92×92, FixedSingle).
- Renders `numLinesOfColors = 3` (Mono/4/5) or `5` (Mode 10) rows × 2 columns:
  - Swatch at `(b*45, a*18)`, size **45×22**.
  - Colors: `BrushCache[b + a*2]`.
  - Labels (mono/4/5): LUM, BAK−00, PF0−01, PF1−10, PF2−11, PF3−11.
  - Labels (mode 10): LUM, 0, 1, 2, 3, 4, 5, 6, 7, 8.
- Click maps to register: `wh = e.X/45 + (e.Y/18)*2`; opens
  `AtariColorSelectorForm` (Shift+click = restore defaults).
- `ColorSets` combo lists "Project colors" + "Alt colors 1..5".

The `AtariColorSelectorForm` (the picker dialog):

- `ClientSize` 284×280, `FixedDialog`.
- `ImagePalette` — (8, 0), 145×272: 128×256 px palette, 8 cols × 16 rows of
  16×16 cells (every 2nd palette color per row), hex row/column labels, white
  15×15 selection rectangle.
- `ImageSelected` — (159, 72), 105×121: selected (top) / hovered (bottom) color.
- `LabelOldColor` / `LabelActualColor` text.

---

## 8. Font Selector — Detailed Analysis (Section F)

### 8.1 Geometry

- Control: `pictureBoxFontSelector` (a plain `PictureBox`, **not** a custom
  control with `OnPaint`; it displays a pre-rendered `Bitmap`).
- Location/Size: **(2, 298), 512 × 256**.
- Constants: `FONT_SELECTOR_WIDTH = 512`, `FONT_SELECTOR_HEIGHT = 256`.

### 8.2 Grid

- **32 columns × 16 rows** of **16×16 px** cells = 512 cells = **2 fonts** of
  256 chars each (one "font bank": fonts 1+2, or 3+4).
- Glyph scale: 8×8 font pixel → 2× zoom (16 px).
- Cell → character mapping: `rx = e.X/16`, `ry = e.Y/16`,
  `SelectedCharacterIndex = rx + ry*32`. Index 0–255 = first font of bank,
  256–511 = second font.

### 8.3 Layout inside the atlas (source bitmap `BitmapFontBanks`, 512×1024)

- Top 512 rows = mono; bottom 512 rows = color.
- Within each 512-row half, per bank:
  - rows 0–3 (y 0–63): Font N **normal**, $00–$7F (128 glyphs)
  - rows 4–7 (y 64–127): Font N **inverse**, $80–$FF
  - rows 8–11 (y 128–191): Font N+1 **normal**
  - rows 12–15 (y 192–255): Font N+1 **inverse**
- The displayed 512×256 source rectangle is chosen from
  `Constants.WhereAreTheFontBanksComingFrom[bank + (InColorMode ? 2 : 0)]`.
  (Note: entries 1–3 declare oversized `Height` values — 512/768/1024 — but
  `DrawImage` clips them, so the visual result is still the correct 256-row band.)

### 8.4 Selection / overlays

- **Selection cursor**: `pictureBoxFontSelectorRubberBand` — a 20×20 **hollow**
  rectangle (2 px red border, region-shaped), positioned at
  `cell origin − 2` (i.e. 2 px outside the 16×16 cell). Default 20×20; in
  MegaCopy it grows with the selection.
- **Duplicate indicator**: `pictureBoxDuplicateIndicator` — purple 20×20 child
  at (0,9); shown when "Show Duplicates" is on and a duplicate exists; moved to
  the duplicate cell at `(rx*16−2, ry*16−2)`.
- **Paste cursor**: `pictureBoxFontSelectorPasteCursor` — green hollow rect.
- **MegaCopy image**: `pictureBoxFontSelectorMegaCopyImage` — 105×105.

### 8.5 Interaction summary

- Plain left-click: select glyph → update `labelEditCharInfo` ("Font N\n$XX #N"),
  redraw editor, check duplicates.
- Font bank toggle (`checkBoxFontBank`): switch bank 1+2 ↔ 3+4, renames
  Load/Save/Clear buttons.
- `Show Duplicates` checkbox: timer-driven `FindDuplicateChar()`.

---

## 9. MegaCopy

MegaCopy is **not a panel**; it is a mode toggled by the `buttonMegaCopy`
checkbox (button-styled, "Mega Copy", in `p_status` at (159,2), 216×21).

When enabled (`MegaCopy_Click`):

- `buttonEnterText.Enabled = true`, `lblInMegaCopyMode.Visible = true`.
- `pictureBoxCharacterEditor` hidden; `pictureBoxClipboardPreview` shown.
- Recolor controls disabled/hidden; Show Duplicates disabled.
- `ActionListNormalModeOnly` buttons (all 18 char/font-shift buttons) disabled.
- Rubber bands reset: view rubber band placed at `pictureBoxAtariView.Location`
  with size 20×20, font-selector rubber band hidden.
- `ConfigureClipboardActionButtons()` enables copy-area transform buttons,
  `buttonPasteInPlace` and `comboBoxPasteIntoFontNr`.
- On exit, the rubber-band state is restored and `SimulateSafeLeftMouseButtonClick()`
  re-selects the character.

`lblInMegaCopyMode` ("In MegaCopy Mode") is anchored Bottom|Right at
(1139, 547).

---

## 10. Functional Groups

Mapping of logical groups → concrete controls (all absolute):

| Logical group | C# realization |
|---|---|
| File / Project | `p_hh` panel (New, Load 1/2, Save 1/2, as..., Clear 1/2, About, Quit) |
| Character editor | `p_xx` panel (`Bevel3` + 160×160 canvas + 16 transform buttons + color/write-mode/undo) |
| GFX / Color | `p_zz` panel (Change GFX, color mode, palette, colorsets, Recolor, Export font) |
| Font operations strip | `p_status` icon row (shift/delete/rotate font, copy-area ops, Paste in Place) |
| Duplicates / MegaCopy / Bank | `p_status` top row (`Show Duplicates`, `Mega Copy`, `Font Bank`) |
| Font selector | `pictureBoxFontSelector` (512×256) |
| View editor | `pictureBoxAtariView` (768×416) + `pictureBoxCharacterSetSelector` (15×416) |
| View scroll | `hScrollBar`, `vScrollBar` |
| View file ops | Export/Import/Clear/Load/Save View + Configure gear (y=454) |
| Paging | `panel1` (Pages, Edit, +, −, Size, configure) |
| Text / undo / skip | Enter text, View Undo/Redo, Skip char, trackbar, Stay in Paste |
| Analysis | `buttonFontAnalysis` "Analyse" |
| View width | `comboBoxBytes` 32/40/48 Bytes |
| Recolor popup | `panelColorSwitcher` / `panelColorSwitcherMode10` (hidden) |
| About splash | `pictureBoxAbout` |
| Tile set | `buttonTileSetEditor` (form-level, left column) |

There are **no group boxes** in the main form; panels (`p_hh`, `p_xx`, `p_zz`,
`p_status`, `Bevel3`, `Bevel4`, `panel1`) with `FixedSingle` borders are the
visual grouping mechanism.

---

## 11. Detailed Geometry Table

All coordinates are client-area coordinates from `FontMakerForm.Designer.cs`
(design-time = 48-byte geometry, 1325×572 client). Runtime defaults to 40-byte
geometry (client width 1194) — only scrollbar/right-edge positions differ.

### 11.1 Top-level containers & big surfaces

| Element | Parent | X | Y | W | H | Dock | Anchor | Note |
|---|---|---:|---:|---:|---:|---|---|---|
| Form `FontMakerForm` | — | — | — | 1325 | 572 | — | — | FixedSingle; MaximizeBox=false; runtime Width 1082/1210/1341 |
| `p_hh` | form | 1 | 0 | 105 | 177 | — | — | FixedSingle, Section A |
| `p_xx` | form | 113 | 0 | 289 | 234 | — | — | FixedSingle, Section B |
| `p_zz` | form | 409 | 0 | 105 | 234 | — | — | FixedSingle, Section C |
| `p_status` | form | −1 | 238 | 515 | 54 | — | — | Section E |
| `buttonTileSetEditor` | form | 3 | 181 | 104 | 51 | — | — | |
| `pictureBoxFontSelector` | form | 2 | 298 | 512 | 256 | — | — | Section F |
| `pictureBoxCharacterSetSelector` | form | 520 | 0 | 15 | 416 | — | — | line-font strip |
| `pictureBoxAtariView` | form | 536 | 0 | 768 | 416 | — | — | 48×26 @16px |
| `hScrollBar` | form | 520 | 417 | 673* | 17 | — | — | *=545/801 by mode |
| `vScrollBar` | form | 1306* | 0 | 17 | 416 | — | — | *=1177/1049 by mode |
| `panel1` | form | 613 | 483 | 191 | 88 | — | — | FixedSingle, paging |
| `panelColorSwitcher` | form | 544 | 8 | 153 | 97 | — | — | hidden |
| `panelColorSwitcherMode10` | form | 696 | 8 | 153 | 161 | — | — | hidden |
| `pictureBoxAbout` | form | 536 | 136 | 512 | 128 | — | — | hidden splash |
| `lblInMegaCopyMode` | form | 1139 | 547 | (auto) | (auto) | — | **B\|R** | hidden |

### 11.2 Inside `p_hh`

| Element | X | Y | W | H | Text |
|---|---:|---:|---:|---:|---|
| `buttonNew` | 7 | 8 | 90 | 20 | New |
| `buttonLoadFont1` | 3 | 35 | 50 | 20 | Load 1 |
| `buttonLoadFont2` | 52 | 35 | 50 | 20 | Load 2 |
| `buttonSaveFont1` | 3 | 61 | 50 | 20 | Save 1 |
| `buttonSaveFont2` | 52 | 61 | 50 | 20 | Save 2 |
| `buttonSaveFont1As` | 3 | 80 | 50 | 20 | as... |
| `buttonSaveFont2As` | 52 | 80 | 50 | 20 | as... |
| `buttonClearFont1` | 3 | 106 | 50 | 20 | Clear 1 |
| `buttonClearFont2` | 52 | 106 | 50 | 20 | Clear 2 |
| `buttonAbout` | 8 | 130 | 89 | 20 | About |
| `buttonQuit` | 8 | 154 | 89 | 20 | Quit |

### 11.3 Inside `p_xx`

| Element | X | Y | W | H | Text |
|---|---:|---:|---:|---:|---|
| `Bevel3` | 63 | 5 | 162 | 162 | (FixedSingle) |
| `pictureBoxCharacterEditor` | 0* | 0* | 160 | 160 | 8×8 @20px |
| `pictureBoxClipboardPreview` | 0* | 15* | 160 | 145 | hidden |
| `labelCopyAreaInfo` | 0* | 0* | 100 | 13 | hidden |
| `pictureBoxCharacterEditorColor1` | 8 | 6 | 49 | 17 | swatch |
| `pictureBoxCharacterEditorColor2` | 232 | 6 | 49 | 17 | swatch |
| `buttonRotateLeft` | 8 | 26 | 49 | 20 | ROL |
| `buttonMirrorHorizontal` | 8 | 47 | 49 | 20 | H MIR |
| `buttonShiftLeft` | 8 | 68 | 49 | 20 | SHL |
| `buttonShiftDown` | 8 | 89 | 49 | 20 | SHD |
| `buttonRestoreDefault` | 8 | 110 | 49 | 20 | RES D |
| `buttonRestoreSaved` | 8 | 131 | 49 | 20 | RES S |
| `buttonCopy` | 8 | 152 | 49 | 20 | CPY |
| `buttonRotateRight` | 232 | 26 | 49 | 20 | ROR |
| `buttonMirrorVertical` | 232 | 47 | 49 | 20 | V MIR |
| `buttonShiftRight` | 232 | 68 | 49 | 20 | SHR |
| `buttonShiftUp` | 232 | 89 | 49 | 20 | SHU |
| `buttonInverse` | 232 | 110 | 49 | 20 | INV |
| `buttonClear` | 232 | 131 | 49 | 20 | CLR |
| `buttonPaste` | 232 | 152 | 49 | 20 | PST |
| `pictureBoxActionColor` | 115 | 172 | 49 | 17 | active color |
| `labelColor` | 73 | 173 | (auto) | 13 | Color: |
| `cmbColor9Menu` | 114 | 170 | 77 | 23 | hidden (Mode 10) |
| `comboBoxWriteMode` | 114 | 200 | 77 | 21 | Rewrite/Insert |
| `label4` | 3 | 188 | (auto) | 13 | Undo/Redo |
| `label3` | 63 | 197 | 48 | 29 | Write Mode |
| `labelEditCharInfo` | 212 | 188 | 69 | 29 | Font 1\n$00 #0 |
| `buttonUndo` | 12 | 204 | 22 | 22 | icon |
| `buttonRedo` | 35 | 204 | 22 | 22 | icon |

\* coordinates relative to `Bevel3`.

### 11.4 Inside `p_zz`

| Element | X | Y | W | H | Text |
|---|---:|---:|---:|---:|---|
| `buttonSwitchGraphicsMode` | 1 | 6 | 99 | 25 | Change GFX |
| `cmbColorMode` | 2 | 33 | 98 | 21 | Mode 4/5/10 |
| `Bevel4` | 5 | 58 | 92 | 92 | (FixedSingle) |
| `pictureBoxPalette` | 0* | 0* | 90 | 90 | 2×3 / 2×5 swatches |
| `comboBoxColorSets` | 5 | 154 | 92 | 21 | Project/Alt colors |
| `buttonRecolor` | 5 | 177 | 64 | 25 | Recolor |
| `buttonShowColorSwitchSetup` | 72 | 179 | 22 | 22 | gear |
| `buttonExportFont` | 5 | 204 | 92 | 25 | Export font |

\* relative to `Bevel4`.

### 11.5 Inside `p_status` (top row + icon row)

| Element | X | Y | W | H |
|---|---:|---:|---:|---:|
| `checkBoxShowDuplicates` | 8 | 6 | (auto) | 17 |
| `buttonMegaCopy` | 159 | 2 | 216 | 21 |
| `label2` (Font Bank:) | 404 | 6 | (auto) | 13 |
| `checkBoxFontBank` | 469 | 1 | 46 | 22 |
| `buttonFontShiftLeftInsert` | 4 | 27 | 24 | 24 |
| `buttonFontShiftLeftRotate` | 28 | 27 | 24 | 24 |
| `buttonFontDeleteCharShiftRight` | 52 | 27 | 24 | 24 |
| `buttonFontDeleteCharShiftLeft` | 76 | 27 | 24 | 24 |
| `buttonFontShiftRightRotate` | 100 | 27 | 24 | 24 |
| `buttonFontShiftRightInsert` | 124 | 27 | 24 | 24 |
| `buttonCopyAreaShiftLeft` | 159 | 27 | 24 | 24 |
| `buttonCopyAreaShiftRight` | 183 | 27 | 24 | 24 |
| `buttonCopyAreaShiftUp` | 207 | 27 | 24 | 24 |
| `buttonCopyAreaShiftDown` | 231 | 27 | 24 | 24 |
| `buttonCopyAreaHMirror` | 255 | 27 | 24 | 24 |
| `buttonCopyAreaVMirror` | 279 | 27 | 24 | 24 |
| `buttonCopyAreaInvert` | 303 | 27 | 24 | 24 |
| `buttonCopyAreaRotateLeft` | 327 | 27 | 24 | 24 |
| `buttonCopyAreaRotateRight` | 351 | 27 | 24 | 24 |
| `buttonPasteInPlace` | 378 | 27 | 104 | 24 |
| `comboBoxPasteIntoFontNr` | 483 | 29 | 30 | 21 |

### 11.6 Right side (form level)

| Element | X | Y | W | H | Text |
|---|---:|---:|---:|---:|---|
| `labelViewCharInfo` | 520 | 436 | (auto) | 13 | Char: Font1 $00 #0 |
| `labelOffsets` | 711 | 436 | (auto) | 13 | TL: [0,0]-[39,25] |
| `labelSelectedArea` | 863 | 436 | (auto) | 13 | Area: |
| `buttonExportView` | 520 | 454 | 86 | 23 | Export View |
| `buttonImportView` | 613 | 454 | 86 | 23 | Import View |
| `buttonClearView` | 706 | 454 | 86 | 23 | Clear View |
| `buttonLoadView` | 799 | 454 | 86 | 23 | Load View |
| `buttonSaveView` | 892 | 454 | 86 | 23 | Save View |
| `buttonConfigure` | 990 | 452 | 28 | 28 | gear |
| `buttonEnterText` | 520 | 481 | 86 | 23 | Enter text |
| `label5` | 810 | 483 | (auto) | 13 | Undo/Redo |
| `checkBoxSkipChar0` | 898 | 483 | (auto) | 17 | Skip char #0 on copy |
| `buttonViewUndo` | 820 | 499 | 22 | 22 | icon |
| `buttonViewRedo` | 843 | 499 | 22 | 22 | icon |
| `trackBarSkipCharX` | 892 | 500 | 284 | 34 | 0..255 |
| `buttonViewActions` | 520 | 508 | 86 | 23 | View Actions |
| `checkBoxStayInPasteMode` | 812 | 527 | (auto) | 17 | Stay in Paste Mode |
| `buttonFontAnalysis` | 520 | 535 | 85 | 23 | Analyse |
| `comboBoxBytes` | 810 | 546 | 67 | 21 | 32/40/48 Bytes |

---

## 12. Dynamic / Responsive Behavior

1. **No user resizing.** `FixedSingle` + `MaximizeBox=false`. The only Anchor is
   `lblInMegaCopyMode` (Bottom|Right) — effectively decorative.
2. **View width switching** (`comboBoxBytes`): changes `Form.Width` to
   1082 / 1210 / 1341 and repositions `vScrollBar` / resizes `hScrollBar`.
3. **Color mode switching** (`cmbColorMode`, `SwitchGfxMode`): toggles `InMode5`
   (`CellHeight` 16↔32, `CharXWidth` 40↔20, `ViewHeight` 26↔13), redraws palette
   (2×3 ↔ 2×5), swaps `pictureBoxActionColor`/color swatches for `cmbColor9Menu`
   in Mode 10, and re-renders the font atlas (mono ↔ color).
4. **Font bank** (`checkBoxFontBank`): renames Load/Save/Clear 1/2↔3/4, swaps
   atlas band (mono/color), re-selects character.
5. **MegaCopy** (`buttonMegaCopy`): hides character editor, shows clipboard
   preview, disables normal-mode buttons, enables copy-area ops, shows
   `lblInMegaCopyMode`.
6. **Recolor** (`buttonShowColorSwitchSetup`): shows `panelColorSwitcher` or
   `panelColorSwitcherMode10` at (544,8) / (696,8).
7. **About splash**: `pictureBoxAbout` aligned to `pictureBoxAtariView.Left`,
   auto-hidden after 5 s.
8. **Mouse wheel**: changes selected character (Ctrl = ×32 step) or color
   (Shift), or tile in MegaCopy (Alt).

---

## 13. Current Rust vs C# Comparison

Current layout source: `crates/afm_gui/ui/main_window.slint` + `components/*.slint`.

| Element | Original C# | Rust/Slint current | Divergence |
|---|---|---|---|
| Window | FixedSingle, fixed size (Width 1210 @40B) | Resizable; min 1200×820, preferred 1480×900 | **HIGH** — resizable, different size |
| Title | "Atari FontMaker vX - f1/f2/f3/f4" | "Atari FontMaker [Rust + Slint]" | LOW |
| Menu bar | none (buttons in panels) | `MenuBar` (2 rows of buttons) | **HIGH** — new element |
| Toolbar | none | `Toolbar` (New/Open/Save/Undo/Redo/modes/Export/…) | **HIGH** — new element |
| MegaCopy bar | `p_status` strip + "Mega Copy" checkbox | dedicated `MegaCopy Toolbar` strip | **HIGH** — different location/form |
| Main split | absolute; left 520px / right view | `HorizontalLayout`; left 532px fixed + right stretch | MEDIUM — similar intent, different mechanics |
| Section A (File) | `p_hh` 105×177 bordered panel | MenuBar rows + Toolbar | **HIGH** |
| Section B (Char editor) | `p_xx` 289×234: 160×160 grid (20px cells), 2 button columns | `CharEditorPanel`: 240×240 grid (30px cells), transform grid, recolor combos | **HIGH** |
| Section C (Colors) | `p_zz` 105×234: palette 90×90 2×3 swatches, GFX, Export font | `PaletteBar` (horizontal register strip at bottom) | **HIGH** — different position/layout |
| Font Selector | 512×256 PictureBox, no title, 16px cells, hollow red rubber band | `FontSelectorPanel` with title, bank button, 512×256 atlas, filled cyan 16px cursor | **HIGH** — extra chrome + different cursor |
| View editor | 768×416 (48×26 @16), 15px line strip, scrollbars, 5 view buttons + gear | `ViewEditorPanel` 640×416 viewport, 22px line strip, page nav in header, no scrollbars, no view buttons | **HIGH** |
| View controls | Export/Import/Clear/Load/Save View + gear (y=454); paging `panel1`; Enter text; View undo/redo; Skip char; Stay in Paste; Analyse; 32/40/48 Bytes | Header row (page prev/next/add/del/rename/undo/redo); no view file buttons; no width combo | **HIGH** |
| MegaCopy indicator | `lblInMegaCopyMode` bottom-right | MegaCopy toolbar toggle | **HIGH** |
| Tile Set Editor | button "Tile Set Editor" (3,181) | MenuBar/Toolbar "TileSet" | **HIGH** |
| Status bar | none (labels inline) | `StatusBar` | **HIGH** — new element |
| Sub-windows | modeless/dialog WinForms forms | Slint modal `PopupWindow`s | MEDIUM — parity is functional, layout differs |

**Summary**: the port reproduces **functionality** but re-invented the **visual
layout**. The most obvious divergences the user already flagged are confirmed:
Font Selector has extra header/title chrome and a different cursor; the color
palette moved from the top-left `p_zz` panel to a bottom `PaletteBar`; the
character editor grid is 30 px/cell instead of 20 px; the view is 640×416 instead
of 768×416 with missing scrollbars and view-file buttons.

---

## 14. Recommended Slint Layout Model (derived from C# — NOT implemented)

Proposed component tree, mapping each C# block to an idiomatic Slint mechanism:

```text
MainWindow (Window, fixed min/pref size ≈ 1210×572 + chrome)
├── RootVertical (VerticalLayout, spacing 0)
│   ├── TopRow (HorizontalLayout, fixed height 234)
│   │   ├── FilePanel          ← p_hh   (VerticalLayout, FixedSingle border)
│   │   ├── CharEditorPanel    ← p_xx   (VerticalLayout + custom 160×160 grid)
│   │   └── ColorPanel         ← p_zz   (VerticalLayout + 90×90 palette grid)
│   ├── StatusPanel            ← p_status (VerticalLayout: top row + icon row)
│   ├── TileSetButton + FontSelectorPanel ← Section F (512×256 Image + overlays)
│   └── (right side is NOT a sibling row — see below)
└── (overlaid) ViewArea (absolute, x≥520)   ← Section D
    ├── LineFontStrip (15×416)
    ├── AtariView (768×416 Image)
    ├── HScrollBar / VScrollBar
    ├── ViewInfoRow / ViewButtonsRow / PagingPanel / …
```

Because the original is **all absolute and fixed-size**, two viable approaches:

1. **Faithful (recommended for pixel parity)**:
   - Fixed window (preferred = 40-byte geometry: client ≈ 1194×572).
   - A single `Rectangle` root; place every block with explicit `x/y/width/height`
     (Slint supports absolute positioning). Use `Rectangle { x:…; y:…; width:…; height:…; border_width: 1px; }`
     for the bordered panels and `Image` for the three raster surfaces
     (char editor, font atlas, atari view) plus small `Rectangle` overlays for
     rubber bands/cursors.
   - Custom `component`s where interaction is needed: `CharEditor`, `FontSelector`,
     `AtariView`, `Palette`, each rendering from an `image` property produced by
     the Rust side (mirroring the C# `Helpers.GetImage` + `Graphics` model).

2. **Fluid (if resizing is desired)** — but this **diverges** from the original:
   - `HorizontalLayout` with left column width ≈ 520 and right `ViewEditorPanel`
     stretching; left column = `VerticalLayout` of File/Editor/Color row, Status
     row, FontSelector.

Mechanism mapping:

| C# element | Slint mechanism |
|---|---|
| `p_hh`, `p_xx`, `p_zz`, `p_status`, `Bevel3`, `Bevel4`, `panel1` | `Rectangle` (border_width 1px) or `VerticalLayout` inside a bordered `Rectangle` |
| Left/right "split" | Either fixed absolute positions, or `HorizontalLayout` (left 520px, right stretch) if fluid |
| `pictureBoxCharacterEditor` 160×160 | custom component drawing 8×8 @20px; or `Image` + `TouchArea` |
| `pictureBoxFontSelector` 512×256 | `Image` (atlas) + overlay `Rectangle` for selection/duplicate/paste cursors + `TouchArea` |
| `pictureBoxAtariView` 768×416 | `Image` (view buffer) + `TouchArea` + overlay rectangles |
| `pictureBoxCharacterSetSelector` 15×416 | `Rectangle` + per-line `Text` (1..4) |
| `hScrollBar`/`vScrollBar` | `ScrollView` or custom `Slider`/`ScrollBar` |
| 16/22/24/28-px buttons | `Button` with fixed `width`/`height` |
| `comboBoxBytes`, `cmbColorMode`, `comboBoxColorSets`, `comboBoxWriteMode` | `ComboBox` |
| `trackBarSkipCharX` | `Slider` (0..255, tick) |
| Hidden popups (`panelColorSwitcher…`) | `PopupWindow` or conditionally-visible `Rectangle` overlay |
| `lblInMegaCopyMode` | `Text` overlay anchored bottom-right |

---

## 15. List of Significant Divergences (C# vs current Slint)

1. **HIGH** — Window is resizable in Slint; original is fixed-size (`FixedSingle`).
2. **HIGH** — MenuBar + Toolbar + StatusBar do not exist in the original; the
   original puts everything in bordered panels.
3. **HIGH** — Font Selector: Slint adds a title row and a "Banks 1 & 2 / 3 & 4"
   toggle button; the original has no title, no button — just the 512×256 atlas
   with a hollow red 20×20 selection rubber band (Slint uses a filled 16×16 cyan
   cursor). Duplicate indicator (purple) is missing in Slint.
4. **HIGH** — Character editor grid: 30 px/cell in Slint vs 20 px/cell in C#
   (and C# uses 1×1 white corner dots as grid markers). Button arrangement
   differs (Slint uses a 4×2 transform grid; C# uses two vertical columns
   ROL/H MIR/SHL/SHD/RES D/RES S/CPY and ROR/V MIR/SHR/SHU/INV/CLR/PST).
5. **HIGH** — Color palette: C# has it top-left in `p_zz` (90×90, 2×3 swatches);
   Slint has a bottom `PaletteBar` with 6 register rectangles.
6. **HIGH** — Atari View: C# surface is 768×416 (up to 48 chars wide) with
   h/v scrollbars; Slint viewport is 640×416 with no scrollbars. Line-font strip
   is 15px in C#, 22px in Slint.
7. **HIGH** — View file ops (Export/Import/Clear/Load/Save View + gear) and the
   view-width combo (32/40/48 Bytes) are absent from the Slint view panel; the
   width combo is the original's only layout-changing control.
8. **MEDIUM** — Paging: C# uses a dedicated bordered `panel1` (Edit, +, −, Size,
   configure gear) below the view; Slint puts page nav inline in the view header.
9. **MEDIUM** — MegaCopy: C# toggles a button in `p_status` and overlays the
   "In MegaCopy Mode" banner; Slint uses a dedicated toolbar strip.
10. **LOW** — Missing visual details: purple duplicate indicator, red/cyan/green/
    yellow rubber-band colors, `pictureBoxAbout` splash, `buttonTileSetEditor`
    position, disabled-state semantics of copy-area buttons.

---

## 16. Elements Not Unambiguously Determined / Caveats

1. **Runtime DPI** — `AutoScaleDimensions = (6F, 13F)` (96 DPI). All coordinates
   above are 96-DPI logical pixels. On HiDPI the form scales; the exact scaled
   geometry was not measured.
2. **`WhereAreTheFontBanksComingFrom` rectangles 1–3** declare `Height`
   512/768/1024 (cumulative, likely a copy-paste bug) but behave correctly due
   to `DrawImage` clipping. Visual effect is correct; the bug is latent.
3. **`AppWidth` field** (`{1082, 1210, 1481}`) is dead code — never read.
4. **Window chrome width** — `ClientWidth = Width − 16` was inferred from
   `vScrollBar` coordinates (1177+17=1194 for Width 1210); not directly measured.
5. **Exact pixel fonts / icon bitmaps** — icon images (`buttonUndo`, `buttonRedo`,
   `buttonConfigure`, `imageListFontShift`, `imageListFont1234`) come from
   `resources`; their exact visual appearance was not extracted.
6. **Splash (`pictureBoxAbout`) image content** — a resource bitmap, not inspected.
7. **Sub-windows' internal layouts** were only partially inventoried (form-level
   sizes and key controls). They are dialogs and out of scope for the main-window
   layout parity, but each was confirmed to be a separate `Form`.
8. **Screen-shot alignment** — no screenshot was available in this session; the
   report is code-derived only.

---

## 17. Verification Note

No production Rust/Slint code was modified. The analysis is based on:

- `FontMakerForm.Designer.cs` (full read) — primary geometry source
- `FontMakerForm.cs` (full read) — dynamic behavior, MegaCopy, width switching
- `General.cs`, `CharacterEditor.cs`, `AtariViewEditor.cs`, `Colors.cs`,
  `FontSelector.cs`, `AtariView.cs`, `AtariFontRenderer.cs`, `Constants.cs`,
  `Configuration.cs`, `AtariColorSelector(.Designer).cs`
- Sub-window Designer files (form-level geometry)
- `default.atrview`, `FontMaker.json`
- `crates/afm_gui/ui/main_window.slint` and `components/*.slint` (current port)

---

## 18. Completion Checklist

- [x] `FontMakerForm.Designer.cs` analyzed (all Location/Size/Dock/Anchor/Controls order)
- [x] `FontMakerForm.cs` analyzed (dynamic UI changes, MegaCopy, width switching)
- [x] Custom rendering analyzed (FontSelector atlas, CharacterEditor grid, AtariView blit, palette)
- [x] Container hierarchy established (no SplitContainer; absolute positioning)
- [x] Main left/right split established (fixed x ≈ 520 seam)
- [x] Font Selector layout established (512×256, 32×16 cells @16px, 2 fonts/bank)
- [x] Character Editor layout established (p_xx, 160×160 @20px, two button columns)
- [x] View layout established (768×416, 15px strip, scrollbars, controls)
- [x] View controls layout established (y=436/454/481/499/508/535/546 rows)
- [x] Functional grouping established (sections A–F)
- [x] Current Slint compared (`main_window.slint` + components)
- [x] Report created at `docs/gui-layout-reconstruction-report.md`
