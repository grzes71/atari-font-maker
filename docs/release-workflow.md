# Release Workflow

This document describes the automated release pipeline implemented in
`.github/workflows/release.yml`.

## Trigger

The workflow runs in two ways:

1. **Automatically** when a Pull Request is **closed and merged**
   (`pull_request` with `types: [closed]`). Closing a PR **without** merging
   (`merged == false`) does **not** create a release.

2. **Manually** via `workflow_dispatch`. The manual run has a
   `dry_run` input (boolean, default `true`):

   | `dry_run` | Behaviour |
   |---|---|
   | `true` (default) | Computes the version, prints the release notes and builds both archives, but does **not** create a tag or GitHub Release. |
   | `false` | Runs the full pipeline and publishes the release. |

## Versioning (Conventional Commits + SemVer)

The version is computed by `scripts/determine_version.py`.

Commit types:

| Prefix | Effect (without breaking change) |
|---|---|
| `feat:` | **MINOR** bump |
| `fix:` | **PATCH** bump |
| `perf:` | **PATCH** bump |
| `refactor:` | **PATCH** bump |
| `docs:` | **PATCH** bump |
| `test:` | **PATCH** bump |
| `chore:` | **PATCH** bump |
| `ci:` | **PATCH** bump |
| `build:` | **PATCH** bump |
| `style:` | **PATCH** bump |
| (no prefix) | **PATCH** bump, listed under *Other changes* |

Breaking changes trigger a **MAJOR** bump:

* `feat!:` or `fix!:` in the subject, or
* `BREAKING CHANGE:` / `BREAKING-CHANGE:` in the commit body.

Only the highest-precedence change in the analyzed range wins:
`MAJOR` > `MINOR` > `PATCH`.

## Commit range

The script finds the latest SemVer tag (e.g. `v1.4.2`) and analyzes the commits
in `previous-tag..merge-commit-sha`. For a merged PR the target commit is
`github.event.pull_request.merge_commit_sha`, which works for squash, merge and
rebase merges. For manual runs the target is the current `HEAD`.

Because the workflow uses a `concurrency` group (`afm-release`) with
`cancel-in-progress: false`, two PRs merged at the same time are processed
sequentially: the second run sees the tag created by the first and computes the
next version (e.g. `v1.5.0`, then `v1.5.1`).

## First release

If the repository has no SemVer tag yet, the initial version is `v0.1.0`
(matching `Cargo.toml`). The existing commit history is listed in the release
notes but does not bump the version.

## Release notes

The generated release notes group commits into:

* `💥 Breaking Changes`
* `🚀 Features`
* `🐛 Fixes`
* `⚡ Performance`
* `🔧 Other`

## Targets and artifacts

| Target | Runner | Archive |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | `atari-fontmaker-vX.Y.Z-linux-x86_64.tar.gz` |
| `x86_64-pc-windows-gnu` (MinGW cross-build) | `ubuntu-latest` | `atari-fontmaker-vX.Y.Z-windows-x86_64.zip` |

The Linux archive contains `atari-fontmaker`, a default `FontMaker.json` and a
`README.txt`. The Windows archive contains `atari-fontmaker.exe`,
`FontMaker.json` and `README.txt`.

## Quality gate

A release is only published after all of these pass on Linux:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

followed by both release builds.

## Dependencies installed by the workflow

* **Linux GUI:** `libfontconfig1-dev`, `libxkbcommon-dev`, `libx11-dev`,
  `libx11-xcb-dev`, `libxcb1-dev`, `libxcursor-dev`, `libgl1-mesa-dev`,
  `libegl1-mesa-dev` (Slint winit + femtovg/OpenGL).
* **Windows cross-build:** `gcc-mingw-w64-x86-64`, `g++-mingw-w64-x86-64`,
  `binutils-mingw-w64-x86-64` (provides `x86_64-w64-mingw32-gcc`,
  `x86_64-w64-mingw32-dlltool`, `x86_64-w64-mingw32-ar`) and `zip`.

## Permissions

The workflow uses minimal permissions:

```yaml
permissions:
  contents: write
```

## Idempotency

Before publishing, `create-release` checks whether the release already exists
(`gh release view vX.Y.Z`). Re-running the workflow for the same merge will not
create duplicate `vX.Y.Z` tags or releases.
