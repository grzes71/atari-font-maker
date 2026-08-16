#!/usr/bin/env python3
"""Determine the next SemVer version and generate release notes.

The version is computed from Conventional Commits found between the latest
SemVer tag (e.g. ``v1.4.2``) and the target commit (the merged PR's merge
commit SHA, or HEAD for manual runs).

SemVer rules:
  * ``feat!`` / ``fix!`` or a ``BREAKING CHANGE:`` / ``BREAKING-CHANGE:``
    footer in the commit body  ->  MAJOR bump.
  * ``feat:`` (no breaking)     ->  MINOR bump.
  * any other commit            ->  PATCH bump.

Commits without a Conventional Commit prefix are grouped under
"Other changes" and still trigger a PATCH bump.

When the repository has no SemVer tag yet, the initial version is used
(``--first-version``, matching ``Cargo.toml``) without bumping.

Usage:
    python3 scripts/determine_version.py \
        --sha <commit> --notes-out release_notes.md [--dry-run]

The computed version is printed to stdout (e.g. ``1.5.0``).
"""

import argparse
import re
import subprocess
import sys

CONVENTIONAL_RE = re.compile(
    r"^(?P<type>[a-zA-Z][a-zA-Z0-9]*)(?:\((?P<scope>[^)]*)\))?(?P<bang>!)?:\s*(?P<subject>.*)$"
)

BREAKING_BODY_RE = re.compile(
    r"(?:^|\n)\s*(?:BREAKING[ -]CHANGE|BREAKING-CHANGE):\s*", re.MULTILINE
)

FEATURES_SECTION = "🚀 Features"
FIXES_SECTION = "🐛 Fixes"
PERFORMANCE_SECTION = "⚡ Performance"
BREAKING_SECTION = "💥 Breaking Changes"
OTHER_SECTION = "🔧 Other"


def run(cmd: str) -> str:
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(
            f"Command failed ({result.returncode}): {cmd}\n{result.stderr}"
        )
    return result.stdout


def semver_key(tag: str):
    m = re.match(r"^v?(\d+)\.(\d+)\.(\d+)$", tag.strip())
    if not m:
        return None
    return (int(m.group(1)), int(m.group(2)), int(m.group(3)))


def parse_commits(commit_range: str):
    """Return a list of (sha, subject, body) for non-merge commits, oldest first."""
    if not commit_range:
        return []
    log = run(
        'git log --no-merges --reverse --pretty=format:"%H%x1f%s%x1f%b%x1e" ' + commit_range
    )
    commits = []
    for block in log.split("\x1e"):
        block = block.strip()
        if not block:
            continue
        parts = block.split("\x1f", 2)
        if len(parts) < 2 or not parts[0].strip():
            continue
        sha = parts[0].strip()
        subject = parts[1].strip()
        body = parts[2] if len(parts) > 2 else ""
        commits.append((sha, subject, body))
    return commits


def classify(commits):
    features, fixes, performance, breaking, other = [], [], [], [], []
    has_breaking = False
    has_feature = False
    has_any = False

    for _sha, subject, body in commits:
        m = CONVENTIONAL_RE.match(subject)
        is_breaking = bool(BREAKING_BODY_RE.search(body))
        if m:
            ctype = m.group("type").lower()
            if m.group("bang"):
                is_breaking = True
            if is_breaking:
                breaking.append(subject)
                has_breaking = True
            elif ctype == "feat":
                features.append(subject)
                has_feature = True
            elif ctype == "fix":
                fixes.append(subject)
            elif ctype == "perf":
                performance.append(subject)
            else:
                other.append(subject)
        else:
            # Non-conventional commit -> PATCH, listed under Other changes.
            other.append(subject)
        has_any = True

    return {
        "features": features,
        "fixes": fixes,
        "performance": performance,
        "breaking": breaking,
        "other": other,
        "has_breaking": has_breaking,
        "has_feature": has_feature,
        "has_any": has_any,
    }


def bump_version(base, result):
    major, minor, patch = base
    if result["has_breaking"]:
        return (major + 1, 0, 0)
    if result["has_feature"]:
        return (major, minor + 1, 0)
    if result["has_any"]:
        return (major, minor, patch + 1)
    return (major, minor, patch)


def build_notes(result):
    lines = ["## What's Changed", ""]

    def add_section(title, items):
        if not items:
            return
        lines.append(f"### {title}")
        lines.append("")
        for item in items:
            lines.append(f"- {item}")
        lines.append("")

    add_section(BREAKING_SECTION, result["breaking"])
    add_section(FEATURES_SECTION, result["features"])
    add_section(FIXES_SECTION, result["fixes"])
    add_section(PERFORMANCE_SECTION, result["performance"])
    add_section(OTHER_SECTION, result["other"])

    if not any(
        [result["breaking"], result["features"], result["fixes"], result["performance"], result["other"]]
    ):
        lines.append("- (no commits)")
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sha", default="HEAD", help="Target commit SHA")
    parser.add_argument("--first-version", default="0.1.0")
    parser.add_argument("--notes-out", default="release_notes.md")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    tags = []
    for line in run("git tag -l").splitlines():
        line = line.strip()
        key = semver_key(line)
        if key is not None:
            tags.append((key, line))
    tags.sort()

    if tags:
        base = tags[-1][0]
        prev_tag = tags[-1][1]
        commit_range = f"{prev_tag}..{args.sha}"
    else:
        # No previous SemVer tag: use the initial version unchanged.
        base = None
        commit_range = args.sha

    commits = parse_commits(commit_range)
    result = classify(commits)

    if base is not None:
        version_tuple = bump_version(base, result)
    else:
        version_tuple = semver_key(args.first_version) or (0, 1, 0)

    version = f"{version_tuple[0]}.{version_tuple[1]}.{version_tuple[2]}"
    notes = build_notes(result)

    with open(args.notes_out, "w", encoding="utf-8") as f:
        f.write(notes)

    print(version)
    if args.dry_run:
        print(f"[dry-run] version: v{version}", file=sys.stderr)
        print(f"[dry-run] commits analyzed: {len(commits)}", file=sys.stderr)
        sys.stderr.write(notes)


if __name__ == "__main__":
    main()
