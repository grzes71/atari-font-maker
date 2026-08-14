# Atari FontMaker (Rust Port)

![Atari FontMaker Screenshot](images/1.JPG) <!-- Update this path with a screenshot of your Rust app -->

## What is it?
This is a modern **Rust port** of the original [Atari FontMaker](https://github.com/matosimi/atari-fontmaker) created by matosimi and RetroCoder. 

Atari FontMaker is a powerful tool used for creating and editing Atari 8-bit/5200 fonts and font-based graphics. The original tool was written in Delphi (dating back to 2003) and later ported to C# for Windows. This Rust port aims to bring the extensive functionality of the classic tool to a modern, memory-safe, and natively cross-platform environment.

* **Original C# Repository:** [matosimi/atari-fontmaker](https://github.com/matosimi/atari-fontmaker)
* **Original Delphi Sources:** [SourceForge](https://sourceforge.net/projects/atari-fontmaker/)
* **Project History:** [matosimi.websupport.sk](http://matosimi.websupport.sk/atari/atari-fontmaker/)

---

## Getting Started

### Prerequisites
To build this project, you will need the [Rust toolchain](https://rustup.rs/) installed on your system.

### Installation
Clone the repository and run the project using Cargo:
```bash
git clone [https://github.com/your-username/atari-fontmaker-rust.git](https://github.com/your-username/atari-fontmaker-rust.git)
cd atari-fontmaker-rust
cargo build --release
cargo run --release

```

---

## Core Features

This Rust port aims to provide feature parity with the 1.6.x versions of the original FontMaker, including:

* **Multi-Font Editing:** Load, edit, and view up to four fonts simultaneously.
* **Graphics Modes:** Support for hi-res 2-color graphics mode (8×8 pixels) and low-res 4/5/9-color modes (Mode 2, Mode 4/5, Mode 10).
* **Advanced Character Operations:** Shift, rotate, mirror, invert, and clear characters with a deep Undo/Redo buffer (up to 2048 operations). Includes a built-in Duplicate Character finder.
* **Mega Copy Mode:** An advanced multi-character clipboard that allows copying areas, shifting/rotating grouped pixels, pasting into new locations, and managing "soft-sprites".
* **Tile Set Editor:** Manage up to 256 8x8 character tiles to easily build complex maps and screens.
* **View Editor & Pager:** Test your creations on a configurable canvas (up to 1024x1024). Manage multiple pages to simulate animations or different game screens.
* **Extensive Exporters:** Export your fonts and views to Binary, Assembler (MADS), Action!, Atari Basic, FastBasic, C, or MadPascal.
* **Modern Compression:** Built-in support for exporting data compressed with ZX0, ZX1, ZX2, and apultra.

---

## File Formats

### `.fnt` Format

The main output of Atari FontMaker. It is a raw binary file, 1024 bytes long without a header. It can be inserted into your project using MADS pseudo-instructions:

```assembly
          .align $400
myFont    ins 'myFont.fnt'

```

### `.atrview` Format

A custom JSON-based format used to save the contents of the view window, font data, color palettes, and tile set information. Because it is JSON, it can be easily read or modified in any text editor.

### Clipboard Format

Atari FontMaker uses a JSON format for clipboard data, allowing you to easily copy/paste character definitions or tiles between different instances of the application or save them in a text file for later.

---

## Keyboard Controls

### General Editing

* `,` / `.` – Previous / Next character
* **Mousewheel** – Previous / Next character
* **CTRL + Mousewheel** – Previous / Next character row
* **SHIFT + Mousewheel** – Next / Previous drawing color
* **0-8** – Select drawing color
* **R / Shift+R** – Rotate character left / right
* **M / Shift+M** – Mirror horizontal / vertical
* **I** – Invert character
* **B** – Switch font bank (1+2) or (3+4)
* **ESC** - Close dialog / exit paste mode

### View & Clipboard Management

* **Ctrl + C / Ctrl + V** – Copy / Paste
* **Ctrl + Z / Ctrl + Y** – Undo / Redo font changes
* **Ctrl + Shift + Z / Ctrl + Shift + Y** – Undo / Redo view changes
* **Ctrl + M** – Switch between Normal and Mega Copy mode
* **Ctrl + Tab / Ctrl + Shift + Tab** – Flip to Next / Previous page
* **Ctrl + 0-9** – Quickly select a page (1-10) in the View Editor
* **ALT + Mousewheel** – Select next/previous tile (Mega Copy / Tile mode)

---

## Credits & Acknowledgements

* **Original Authors:** [matosimi](https://github.com/matosimi) and RetroCoder.
* Thank you to the Atari 8-bit community for years of continuous feedback and testing on the original versions that defined the feature set for this port.
* Rust port developed by **grzes71**.

