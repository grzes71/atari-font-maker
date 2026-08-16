# Atari FontMaker — AI Agent Instructions

## Project Overview

This repository is a migration of the existing Atari FontMaker application
from C#/.NET to Rust + Slint.

The original C# implementation is located in:

    atari-fontmaker-master/

The target implementation will be written in Rust with a Slint GUI.

The migration must preserve the observable behavior and functionality of the
original C# application unless an intentional behavioral change is explicitly
requested by the user.

---

# Authoritative Documentation

Before performing any migration work, read:

    docs/architecture.md
    docs/migration-plan.md
    docs/testing-strategy.md

If available, also read:

    docs/reference-harness-audit.md

These documents describe the current architecture, migration strategy and
testing strategy.

Do not invent a different architecture without first identifying the conflict
and explaining why a change is necessary.

When documentation conflicts with the actual C# implementation, treat the
actual source code as the source of truth for existing behavior.

Do not silently modify the documentation to hide inconsistencies.

---

# Migration Philosophy

This is a behavioral migration, not a line-by-line translation.

DO NOT:

    C# class → Rust struct
    C# method → Rust method
    C# GUI control → Slint control

simply because the original code uses those constructs.

Instead:

1. Understand the behavior of the existing implementation.
2. Identify its responsibilities and dependencies.
3. Reimplement the behavior using idiomatic Rust.
4. Preserve externally observable behavior.
5. Verify the result against the C# reference implementation.

Prefer idiomatic Rust over code that resembles C#.

---

# Legacy C# Application

The directory:

    atari-fontmaker-master/

contains the original application.

Treat it as the reference implementation.

Do not modify the legacy application unless the task explicitly concerns
the legacy implementation or the reference testing infrastructure.

In particular, do not:

- refactor the legacy application,
- rename legacy classes,
- "clean up" legacy code,
- change algorithms,
- change file formats,
- change GUI behavior,
- fix unrelated bugs.

If a suspicious behavior is discovered, document it rather than silently
changing it.

---

# Reference Harness

The C# Reference Harness is located in:

    tools/ReferenceHarness/

It is used to extract and verify behavior from the legacy C# implementation.

The generated reference data is stored in:

    tests/fixtures/

Golden-master data generated from the legacy implementation is authoritative
for migration compatibility.

Do not manually modify golden-master files.

If a golden master appears to be incorrect:

1. investigate the reason,
2. verify the behavior against the legacy C# implementation,
3. modify the Reference Harness if necessary,
4. regenerate the fixture,
5. document the reason for the change.

Never modify a golden master simply to make a Rust test pass.

A golden master is evidence of the behavior of the legacy implementation,
not a specification invented by the AI agent.

---

# Golden-Master Testing

For migrated functionality, prefer comparison against the legacy
implementation whenever practical.

The expected relationship is:

    C# implementation
          ↓
    Reference Harness
          ↓
    Golden Master
          ↓
    Rust implementation
          ↓
    comparison

A migrated module is not considered correct merely because its Rust tests
pass.

Where a golden-master test exists, the Rust implementation should produce
the same observable result as the C# implementation.

For binary formats, compare binary data.

For textual exports where exact output is part of the format, compare exact
text output.

For structured formats such as JSON, compare semantic content where
appropriate rather than insignificant formatting differences.

---

# Scope Control

Work only on the scope explicitly requested by the user.

When migrating one module:

- do not migrate unrelated modules,
- do not redesign the whole application,
- do not migrate the GUI unless requested,
- do not modify unrelated tests,
- do not change unrelated documentation.

If another problem is discovered, report it instead of expanding the scope
automatically.

---

# Migration Order

Follow the order defined in:

    docs/migration-plan.md

Do not skip migration phases without explaining why.

Prefer migrating functionality in small, independently testable units.

The preferred workflow is:

    Analyze
       ↓
    Implement
       ↓
    Test
       ↓
    Compare with C#
       ↓
    Fix discrepancies
       ↓
    Verify
       ↓
    Proceed to next module

---

# Rust Code

The target implementation uses Rust.

Prefer:

- safe Rust,
- idiomatic ownership and borrowing,
- Result<T, E> for recoverable errors,
- Option<T> where appropriate,
- explicit error handling,
- small focused modules,
- clear data ownership,
- deterministic behavior,
- testable business logic.

Avoid:

- unnecessary unsafe code,
- global mutable state,
- unnecessary singletons,
- C#-style class hierarchies,
- excessive abstraction,
- unnecessary dependencies,
- cloning data without a reason.

Do not introduce unsafe Rust unless there is a concrete technical reason.
If unsafe code is necessary, document why.

---

# GUI — Slint

The target GUI uses Slint.

Keep GUI code separate from core application logic.

The general architectural boundary should be:

    Slint GUI
        ↓
    Application layer
        ↓
    Core/domain logic
        ↓
    Data / file formats / I/O

Do not put substantial business logic into `.slint` files.

Slint should primarily handle:

- presentation,
- user interaction,
- UI state,
- visual components,
- user input.

Rust should handle:

- business logic,
- algorithms,
- file formats,
- parsing,
- data processing,
- application state where appropriate,
- external I/O.

Do not start GUI migration while the corresponding core functionality
is still unstable unless explicitly requested.

---

# Atari File Formats

Compatibility with existing Atari-related formats is a primary requirement.

Treat the following as compatibility-sensitive:

- .fnt
- .fn2
- .atrview
- exporter formats
- Atari-specific binary data

Do not change binary layouts, byte ordering, bit packing, character encoding,
or textual export formats without explicit approval.

When uncertain, inspect the C# implementation and existing golden masters.

---

# Algorithms

Algorithms related to Atari font manipulation, rendering, color handling,
encoding and exporting are compatibility-critical.

Examples include:

- glyph transformations,
- bit shifts,
- rotations,
- mirroring,
- inversion,
- color encoding,
- palette matching,
- rendering,
- exporters,
- Undo/Redo.

Do not "improve" an algorithm during migration unless the user explicitly
requests such a change.

The first Rust implementation should reproduce the existing behavior.

Optimization can be performed later as a separate task.

---

# GUI Behavior

The GUI is not the primary source of truth for core functionality.

When possible, test the underlying behavior independently of the GUI.

GUI migration should preserve observable behavior including:

- commands,
- keyboard shortcuts,
- selection behavior,
- editing operations,
- undo/redo,
- menus,
- dialogs,
- application state,
- rendering behavior.

Do not assume that a visually similar Slint implementation is behaviorally
equivalent.

---

# Dependencies

Do not add a dependency merely because it is convenient.

Before adding a Rust crate:

1. determine whether the standard library is sufficient,
2. check whether an existing project dependency already provides the required
   functionality,
3. consider portability to Windows and Linux,
4. consider maintenance and licensing,
5. explain significant new dependencies.

Keep the dependency set reasonably small.

---

# Cross-Platform Requirements

The target application must support:

- Windows
- Linux

Do not introduce unnecessary platform-specific code.

When platform-specific functionality is unavoidable, isolate it behind a
small and clearly defined abstraction.

Do not assume that behavior available on Windows is automatically available
on Linux.

---

# Testing Requirements

Every migrated non-trivial module should have automated tests.

At minimum, consider:

- unit tests,
- integration tests,
- golden-master/reference tests.

Run the relevant tests after modifying code.

For Rust code, use appropriate Cargo commands such as:

    cargo check
    cargo test

and, where appropriate:

    cargo clippy

Do not claim that a migration is complete without actually running the
relevant tests.

---

# Handling Discrepancies

If Rust and C# produce different results:

DO NOT immediately change the Rust implementation to match a guessed result.

Instead:

1. reproduce the discrepancy,
2. identify the exact input,
3. identify the exact C# output,
4. identify the exact Rust output,
5. inspect both implementations,
6. determine which behavior is authoritative,
7. fix the Rust implementation if the C# behavior is correct,
8. update the reference infrastructure only if the C# reference was wrong.

Document non-obvious discrepancies.

---

# Error Handling

Do not silently ignore errors.

Avoid patterns equivalent to:

    unwrap()
    expect()

in production code unless the invariant is genuinely guaranteed.

Tests may use unwrap/expect where appropriate.

Production error handling should provide useful context to the caller.

---

# Code Quality

Prefer simple, readable code over clever code.

Do not perform large refactorings during a migration step.

Avoid speculative abstractions.

Do not optimize prematurely.

Correctness and behavioral compatibility have priority over performance
during the initial migration.

Performance optimization should be a separate, measurable task.

---

# Git

Keep changes focused.

Prefer small commits corresponding to logical migration steps.

Do not rewrite unrelated history.

Do not commit generated build artifacts.

Do not commit temporary files.

Golden-master fixtures are part of the repository when explicitly generated
as part of the reference testing strategy.

---

# Agent Workflow

Before implementing a migration task:

1. Read the relevant documentation.
2. Identify the exact module/functionality being migrated.
3. Inspect the corresponding C# implementation.
4. Inspect relevant reference fixtures and tests.
5. Identify dependencies.
6. State the intended scope internally before making changes.

During implementation:

1. Make the smallest reasonable change.
2. Keep the architecture consistent with the migration plan.
3. Add or update tests.
4. Run the relevant tests.

After implementation:

1. Run `cargo check`.
2. Run relevant `cargo test` tests.
3. Run the Reference Harness comparison when applicable.
4. Run `cargo clippy` when appropriate.
5. Review the diff for unrelated changes.

---

# Do Not

Never:

- rewrite the entire application in one step,
- translate C# line-by-line,
- migrate GUI and business logic simultaneously without a reason,
- change behavior for "improvement",
- modify golden masters to hide failures,
- remove legacy C# code prematurely,
- introduce dependencies without justification,
- make unrelated refactorings,
- assume undocumented behavior,
- claim tests passed without actually running them.

---

# Current Migration Status

The project currently contains:

- `docs/architecture.md` — architecture analysis
- `docs/migration-plan.md` — migration plan
- `docs/testing-strategy.md` — testing strategy
- `tools/ReferenceHarness/` — C# reference harness
- `tests/fixtures/` — reference/golden-master data

The Rust implementation is not yet considered complete.

The legacy C# application remains the behavioral reference until the migration
is explicitly declared complete.

When beginning a new migration phase, consult `docs/migration-plan.md` first.