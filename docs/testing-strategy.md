# Testing & Golden-Master Strategy: C# → Rust + Slint

> **Purpose**: Define the reference testing framework, golden-master test vectors, and verification methodology to guarantee zero regressions during the migration of Atari FontMaker from C# (.NET 9 WinForms) to Rust + Slint.
>
> **Core Premise**: The legacy C# application lacks automated tests. To ensure functional equivalence, we define deterministic input-output oracles (characterization tests) before and during Rust implementation.

---

## 1. Testing Philosophy & Test Pyramid

```
                       ┌─────────────────────────┐
                       │     Manual & Visual     │
                       │    Slint GUI Testing    │
                       └────────────┬────────────┘
                                    │
                       ┌────────────▼────────────┐
                       │   End-to-End Workflows  │
                       │ (Load → Edit → Export)  │
                       └────────────┬────────────┘
                                    │
                       ┌────────────▼────────────┐
                       │  Golden-Master Fixtures │
                       │ (Byte-for-byte Parity)  │
                       └────────────┬────────────┘
                                    │
                       ┌────────────▼────────────┐
                       │   Headless Rust Units   │
                       │ (Bitmath, Codecs, Math) │
                       └─────────────────────────┘
```

1. **Deterministic Oracles**: Every algorithmic component (bit transformations, character decoders, software renderers, file exporters) must be verified against exact pre-computed output generated from C# reference fixtures.
2. **Headless-First**: 90% of the application's functionality (domain logic, transformations, file I/O, rendering calculations, export generation) can and must be tested headlessly without a display server.
3. **No Blind Trust**: Any discrepancy between C# implementation and standard specifications must be identified as either a legacy quirk to preserve or an explicit bug fix.

---

## 2. Inventory of Testable Functionalities & Test Vectors

### 2.1 Character Glyph Bit Manipulations & Transformations

**Target Component**: `AtariFont.cs`  
**Input Vectors**:
- Standard 1024-byte Atari font (`Default.fnt` — 128 characters × 8 bytes).
- All 128 character byte sequences (both normal `0..127` and inverted `128..255`).
- Synthetic edge cases: all `0x00`, all `0xFF`, diagonal patterns (`0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01`), checkerboard (`0xAA, 0x55...`).

| Operation | Observable Output (Oracle) | Verification Method |
|---|---|---|
| **Rotate Left (90° CCW)** | 8-byte array representing rotated glyph | Byte equality check against pre-calculated matrix |
| **Rotate Right (90° CW)** | 8-byte array representing rotated glyph | Byte equality check against pre-calculated matrix |
| **Mirror Horizontal** | 8-byte array with bits reversed per row: <br>• 1-bit mode (bit-reversal)<br>• 2-bit mode (pixel pair swap)<br>• 4-bit mode (nibble swap) | Byte equality check across all 3 color modes |
| **Mirror Vertical** | 8-byte array with row order reversed (`row[7-y]`) | Byte equality check |
| **Shift Up / Down** | 8-byte array with rows shifted by 1 (wrapped or cleared) | Byte equality check |
| **Shift Left / Right** | 8-byte array with bits shifted horizontally | Byte equality check across 1-bit, 2-bit, 4-bit |
| **Invert Character** | Bitwise NOT (`~byte`) | Exact bit inversion |
| **Clear Character** | 8 bytes of `0x00` | Zero check |
| **Font-Level Shift Left/Right** | 1024-byte array shifted by 8 bytes (1 character) | Array comparison across full font bank |
| **Delete & Shift Char** | 1024-byte array with character removed and tail shifted | Full bank comparison |

### 2.2 Color & Pixel Encodings

**Target Component**: `AtariFont.cs` / `Constants.cs`  
**Input Vectors**: All 256 possible byte values (`0x00..0xFF`).

| Encoding | Input | Observable Output (Oracle) |
|---|---|---|
| **DecodeMono** | 1 byte | `[u8; 8]` array of `0` or `1` |
| **EncodeMono** | `[u8; 8]` | 1 byte matching original |
| **DecodeColor2Bit** (Mode 4/5) | 1 byte | `[u8; 4]` array of 2-bit color indices (`0..3` mapped to palette indices) |
| **EncodeColor2Bit** (Mode 4/5) | `[u8; 4]` | 1 byte matching original |
| **DecodeColor4Bit** (Mode 10) | 1 byte | `[u8; 2]` array of 4-bit color indices (`0..15` mapped to palette indices) |
| **EncodeColor4Bit** (Mode 10) | `[u8; 2]` | 1 byte matching original |

### 2.3 Palette & Color Matching

**Target Component**: `altirraPAL.pal` / `Helpers.cs`  
**Input Vectors**: 768-byte palette binary, arbitrary RGB test colors (`(0,0,0)`, `(255,255,255)`, `(74,120,240)`, `(180,50,30)`).

| Feature | Observable Output | Verification Method |
|---|---|---|
| **Palette Parsing** | 256 `(R, G, B)` triplets | Binary verification against Altirra reference |
| **FindClosest (Nearest Color)** | Palette index `0..255` (even indices only) | Euclidean distance in RGB space: $\sqrt{(R_1-R_2)^2 + (G_1-G_2)^2 + (B_1-B_2)^2}$ |

### 2.4 Software Pixel Buffer Rendering

**Target Component**: `AtariFontRenderer.cs`  
**Input Vectors**:
- 4096-byte font buffer.
- Color palettes in Mode 2, Mode 4, Mode 5, and Mode 10.
- Normal and inverted character flags.

| Rendering Path | Target Output | Golden-Master Artifact |
|---|---|---|
| **Full Font Bank Atlas (512×1024)** | 32-bit RGBA pixel buffer (2,097,152 bytes) | Raw RGBA dump / PNG reference image from C# GDI+ |
| **Single Character (16×16 px)** | 16×16 32-bit RGBA buffer | Pixel-by-pixel color equality |
| **Mode 4 vs Mode 5 Height** | Mode 4: 16px row; Mode 5: 32px row | Aspect ratio & pixel duplication check |
| **Inverted PF2/PF3 Color Switch** | In Mode 4/5, color index 3 on characters `128..255` renders as color index 4 | Exact RGBA color value check |

### 2.5 File Format Parsers & Serializers (Roundtrip Oracles)

**Target Component**: `AtrViewInfoJson.cs`, `TileSet.cs`, `JsonSupport.cs`  
**Input Fixtures**:
- `Resources/Default.fnt` (1024 B binary)
- `Resources/default.atrview` (JSON project file)
- Multi-page project file with customized tiles, custom dimensions (48×26), and Mode 10 colors.
- Hex-encoded byte strings (`Data`, `Chars`, `Lines`, `Colors`).

| Test Case | Procedure | Validation Oracle |
|---|---|---|
| **Binary `.fnt` Roundtrip** | Load → In-memory parse → Write | 100% byte-for-byte binary equality (1024 B) |
| **Dual Font `.fn2` Roundtrip** | Load → In-memory parse → Write | 100% byte-for-byte binary equality (2048 B) |
| **`.atrview` JSON Compatibility** | Parse legacy C# JSON → Serialize in Rust | Semantic JSON equivalence (all fields, dimensions, pages, tiles match) |
| **Hex Strings Integrity** | Decode hex string to `[u8]` → Re-encode | Exact uppercase hex string match |
| **Clipboard JSON Interchange** | Serialize `ClipboardJson` in Rust → Parse in C# format | Deserialization without missing or truncated fields |

### 2.6 Code & Data Exporters

**Target Component**: `ExportFontWindow.cs`, `ExportViewWindow.cs`  
**Input Vectors**: Sample 1-font and 4-font configurations, transposed and non-transposed views, compressed and uncompressed buffers.

| Exporter Format | Expected Structure | Verification Method |
|---|---|---|
| **Assembler (.txt)** | `.byte $xx, $yy...` with 16 bytes per line | String diff comparison against C# generated output |
| **Action! (.txt)** | `BYTE ARRAY font = [ $xx $yy... ]` | String diff comparison |
| **Atari BASIC (.txt)** | `10010 DATA 12,34,56...` with exact line numbering | String diff comparison |
| **FastBasic (.txt)** | `DATA font() BYTE = 12,34...` | String diff comparison |
| **MADS (.txt)** | `dta d'...'` or `dta $xx` | String diff comparison |
| **C Data Array (.txt)** | `const unsigned char font[] = { 0x.. };` | String diff comparison |
| **Mad Pascal Array (.txt)**| `const font: array [0..1023] of byte = ( ... );` | String diff comparison |
| **BASIC Listing (.lst)** | Binary BASIC format merged with `basicremfont.lst` | Byte-for-byte binary diff against C# `.lst` output |
| **Font Sheet BMP (.bmp)** | Monochromatic or colored Windows BMP file | BMP header + pixel payload comparison |

### 2.7 Undo/Redo State Machine

**Target Component**: `AtariFontUndoBuffer.cs`, `AtariViewUndoBuffer.cs`  
**Input Scenarios**:
1. Edit character → Undo → Verify state restored.
2. Edit character A → Edit character B → Undo → Redo → Verify state.
3. Perform 260 edits → Verify circular buffer discards oldest state past 250 limit.
4. Page 1 edit → Switch to Page 2 → Edit Page 2 → Undo → Verify Page 1 state untouched.

---

## 3. Automated vs. GUI-Required Test Classification

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Automated Headless Tests                        │
│                           (~90% of Codebase)                           │
│                                                                        │
│  • All Glyph Transformations (Rotate, Mirror, Shift, Invert, Clear)    │
│  • Bit Encodings & Mode 2/4/5/10 Decoders                              │
│  • Palette Parsing & Distance Math                                     │
│  • Software Rendering into RGBA buffers                                │
│  • File Serialization / Deserialization (.fnt, .atrview, .json)        │
│  • Compression Engine (ZX0 / apultra)                                  │
│  • Exporters (ASM, BASIC, C, Action!, MADS, BMP)                       │
│  • Undo / Redo Stacks & State Transitions                              │
└────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                     GUI & Visual Tests (Slint)                         │
│                           (~10% of Codebase)                           │
│                                                                        │
│  • Rubberband selection visual drag & drop                             │
│  • Mouse stroke drawing continuous drag in character editor            │
│  • Keyboard shortcut dispatching (Ctrl+Z, Ctrl+C, R, M, I)             │
│  • Window layout responsiveness & High-DPI rendering                   │
│  • System native file dialog triggers (via rfd)                        │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Golden-Master Test Data Architecture

We will organize reference test fixtures under a dedicated `tests/fixtures/` directory:

```
tests/
├── fixtures/
│   ├── fonts/
│   │   ├── Default.fnt             # Standard Atari 1024-byte font
│   │   └── dual_sample.fn2         # 2048-byte dual font
│   ├── projects/
│   │   ├── default.atrview         # Standard project fixture
│   │   ├── mode10_sample.atrview   # Mode 10 multi-color project
│   │   └── multi_page.atrview      # Multi-page project with custom tiles
│   ├── golden_renders/
│   │   ├── font_bank_mono.raw      # 512x1024 RGBA raw pixel dump
│   │   ├── font_bank_mode4.raw     # Mode 4 colored font bank
│   │   └── font_bank_mode10.raw    # Mode 10 colored font bank
│   └── golden_exports/
│       ├── sample_font_asm.txt     # Reference Assembler export
│       ├── sample_font_basic.txt   # Reference BASIC export
│       ├── sample_font_c.txt       # Reference C array export
│       └── sample_view_action.txt  # Reference Action! view export
└── integration/
    ├── test_transforms.rs          # Bitwise glyph transformation tests
    ├── test_codecs.rs              # Serialization roundtrip tests
    ├── test_render.rs              # Image comparison tests
    ├── test_exports.rs             # Export string diff tests
    └── test_undo.rs                # State machine tests
```

---

## 5. Execution Workflow for Golden-Master Verification

### Step 1: Characterization Fixture Extraction
Before writing new modules, extract baseline reference data:
1. Load `Default.fnt` and execute every single transform in C#. Save the resulting 8-byte array fixtures to `tests/fixtures/transforms_golden.json`.
2. Generate export text files for all formats from `default.atrview` and store under `tests/fixtures/golden_exports/`.
3. Render full font sheets from `default.atrview` in mono, mode 4, and mode 10; save as uncompressed RGBA pixel dumps.

### Step 2: Continuous Rust Parity Testing
During implementation, run automated validation:

```bash
# Run all unit and golden-master integration tests
cargo test --all-targets

# Test bit-exact glyph transformations
cargo test --test test_transforms

# Test exact export syntax parity
cargo test --test test_exports

# Test render pixel buffer matching
cargo test --test test_render
```

### Step 3: Visual & Interactive Inspection (Slint)
For GUI-specific behaviors:
1. Verify rubberband selection overlay coordinates.
2. Verify continuous drawing on mouse-down drag across character cells.
3. Test undo/redo button enable/disable state responsiveness.

---

## 6. Summary: Key Regression Safeguards

| Feature / Area | Primary Risk | Safeguard Method |
|---|---|---|
| **Bitwise Mirror/Rotate** | Subtle endianness/bit shift mismatch | Comprehensive table-driven tests for all 128 glyphs |
| **Mode 4/5 Inverted PF2/PF3** | Wrong color applied to inverted characters | Golden pixel buffer check on chars `0..127` vs `128..255` |
| **`.atrview` Backward Compatibility** | Incompatible JSON schema on older versions | Serialization roundtrip on v1911, v2007, and v2023 files |
| **BASIC Listing `.lst` Export** | Corrupted binary BASIC bytecode | Byte-for-byte binary diff against `basicremfont.lst` output |
| **Multi-page View Undo** | Undoing on Page 2 corrupting Page 1 | Isolated per-page state machine tests |
