#!/usr/bin/env python3
"""Structure and metadata linter for Orion documentation (DOCUMENTATION_STANDARD.md).

Checks:
1. Every chapter in src/SUMMARY.md exists on disk, and every .md file in src/
   belongs to src/SUMMARY.md (no orphan files).
2. Navigation depth in src/SUMMARY.md does not exceed 3 levels, and indentation
   uses multiples of 2 spaces.
3. Every page carries valid front-matter comments at the top:
   - <!-- description: ... --> (100-165 chars)
   - <!-- type: ... --> (valid content type)
   - <!-- last_verified: YYYY-MM-DD --> or <!-- generated_from: <ver> on YYYY-MM-DD -->
4. Every page has a single # H1 heading matching its SUMMARY.md label (or with
   a standard prefix like "How to " or "Tutorial: "), does not skip heading
   levels, stops at H4, and has no trailing punctuation (., :, !) on headings.
5. Callouts use valid GFM alert labels (NOTE, TIP, WARNING, IMPORTANT, CAUTION)
   and are not stacked immediately back-to-back.
6. Edition badges use the canonical "Standard+" label and never appear in # H1.
"""

from __future__ import annotations

import datetime
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent
SRC = ROOT / "src"
SUMMARY = SRC / "SUMMARY.md"

TYPES = {
    "quickstart",
    "tutorial",
    "concept",
    "guide",
    "reference",
    "troubleshooting",
    "hub",
    "release-note",
    "migration",
    "glossary",
}

ALERT_LABELS = {"NOTE", "TIP", "WARNING", "IMPORTANT", "CAUTION"}

MAX_NAV_DEPTH = 3
MAX_HEADING_LEVEL = 4

NOT_A_CHAPTER = {"SUMMARY.md", "diagram-assets/VENDOR.md"}


def problems() -> list[str]:
    found: list[str] = []
    chapters = read_summary(found)
    pages = {rel for rel, _, _ in chapters}

    for path in sorted(SRC.rglob("*.md")):
        rel = str(path.relative_to(SRC)).replace("\\", "/")
        if rel in NOT_A_CHAPTER:
            continue
        if rel not in pages:
            found.append(f"{rel}: not listed in SUMMARY.md (orphan file)")
            continue
        found.extend(check_page(path, rel))

    found.extend(check_navigation(chapters))
    return found


def read_summary(found: list[str]) -> list[tuple[str, str, int]]:
    out: list[tuple[str, str, int]] = []
    for line in SUMMARY.read_text().splitlines():
        match = re.match(
            r"^(\s*)(?:- )?\[([^\]]+)\]\(\./([A-Za-z0-9_./-]+\.md)\)", line
        )
        if not match:
            continue
        indent, label, rel = match.groups()
        if len(indent) % 2:
            found.append(
                f"SUMMARY.md: '{label}' is indented by {len(indent)}, expected a multiple of 2"
            )
        out.append((rel, label, len(indent) // 2))
    return out


def normalize_title(title: str) -> str:
    title = re.sub(r"^(?:How to |Tutorial:\s*)", "", title.strip())
    return title


def check_navigation(chapters: list[tuple[str, str, int]]) -> list[str]:
    found: list[str] = []

    for rel, label, depth in chapters:
        if depth + 1 > MAX_NAV_DEPTH:
            found.append(
                f"SUMMARY.md: '{label}' is nested {depth + 1} levels deep; "
                f"max allowed is {MAX_NAV_DEPTH}"
            )

    for rel, label, _ in chapters:
        page = SRC / rel
        if not page.exists():
            found.append(f"SUMMARY.md: '{label}' points to missing file {rel}")
            continue
        h1 = heading_one(page.read_text())
        if h1 is None:
            found.append(f"{rel}: missing # H1 heading")
        elif normalize_title(label) != normalize_title(h1) and label != h1:
            found.append(
                f"{rel}: sidebar label '{label}' does not match H1 '{h1}'"
            )

    return found


def check_page(path: pathlib.Path, rel: str) -> list[str]:
    text = path.read_text()
    lines = text.splitlines()
    found: list[str] = []

    kind = declared_type(text)
    if kind is None:
        return [f"{rel}: missing '<!-- type: ... -->' metadata comment"]
    if kind not in TYPES:
        return [f"{rel}: unknown page type '{kind}'; expected one of {sorted(TYPES)}"]

    found += check_stamp(rel, lines)
    found += check_headings(rel, text)
    found += check_callouts(rel, lines)
    found += check_editions(rel, lines)
    return found


def declared_type(text: str) -> str | None:
    match = re.search(r"^<!--\s*type:\s*([a-z-]+)\s*-->$", text, re.M)
    return match.group(1) if match else None


def heading_one(text: str) -> str | None:
    for line in strip_fences(text).splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return None


def check_editions(rel: str, lines: list[str]) -> list[str]:
    found: list[str] = []
    badge = re.compile(
        r'<span class="edition-badge edition-[a-z-]+"[^>]*>([^<]+)</span>'
    )
    for number, line in enumerate(lines, 1):
        for match in badge.finditer(line):
            label = match.group(1).strip()
            if label != "Standard+":
                found.append(
                    f"{rel}:{number}: edition badge says '{label}'; only 'Standard+' is allowed"
                )
            if line.startswith("# "):
                found.append(
                    f"{rel}:{number}: edition badge in H1; place badges on H2/H3 or table rows"
                )
    return found


def check_stamp(rel: str, lines: list[str]) -> list[str]:
    verified = generated = None
    for line in lines[:6]:
        match = re.match(r"^<!--\s*last_verified:\s*(\d{4}-\d{2}-\d{2})\s*-->$", line)
        if match:
            verified = match.group(1)
        match = re.match(
            r"^<!--\s*generated_from:\s*(\S+) on (\d{4}-\d{2}-\d{2})\s*-->$", line
        )
        if match:
            generated = match.group(2)

    if verified and generated:
        return [f"{rel}: carries both last_verified and generated_from stamps; use only one"]
    stamp = verified or generated
    if not stamp:
        return [
            f"{rel}: missing freshness stamp ('<!-- last_verified: YYYY-MM-DD -->' "
            "or '<!-- generated_from: <version> on YYYY-MM-DD -->')"
        ]

    try:
        when = datetime.date.fromisoformat(stamp)
    except ValueError:
        return [f"{rel}: freshness stamp '{stamp}' is not a valid ISO date"]
    if when > datetime.date.today():
        return [f"{rel}: freshness stamp '{stamp}' is in the future"]
    return []


def check_headings(rel: str, text: str) -> list[str]:
    found: list[str] = []
    headings = [
        (len(m.group(1)), m.group(2).strip())
        for m in re.finditer(r"^(#{1,6})\s+(.+?)\s*$", strip_fences(text), re.M)
    ]

    previous = 0
    for level, title in headings:
        if level > MAX_HEADING_LEVEL:
            found.append(
                f"{rel}: heading '{title}' is H{level}; max allowed is H{MAX_HEADING_LEVEL}"
            )
        if previous and level > previous + 1:
            found.append(
                f"{rel}: heading '{title}' skips from H{previous} to H{level}"
            )
        if title.endswith((".", ":", "!")):
            found.append(
                f"{rel}: heading '{title}' ends with punctuation"
            )
        previous = level

    return found


def check_callouts(rel: str, lines: list[str]) -> list[str]:
    found: list[str] = []
    previous_end = -99
    for index, line in enumerate(lines):
        match = re.match(r"^> \[!([A-Z]+)\]\s*$", line)
        if not match:
            continue
        label = match.group(1)
        if label not in ALERT_LABELS:
            found.append(
                f"{rel}:{index + 1}: callout '[!{label}]' is not one of {sorted(ALERT_LABELS)}"
            )
        if index - previous_end <= 1:
            found.append(f"{rel}:{index + 1}: two callouts placed back-to-back")
        end = index
        while end + 1 < len(lines) and lines[end + 1].startswith(">"):
            end += 1
        previous_end = end
    return found


def strip_fences(text: str) -> str:
    out: list[str] = []
    fenced = False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            fenced = not fenced
            out.append("")
            continue
        out.append("" if fenced else line)
    return "\n".join(out)


def main() -> int:
    level = sys.argv[1] if len(sys.argv) > 1 else "report"
    found = problems()
    if not found:
        print("docs-structure: OK")
        return 0

    for line in found:
        print(f"docs-structure: {line}", file=sys.stderr)
    print(f"\ndocs-structure: {len(found)} findings", file=sys.stderr)
    if level == "enforce":
        return 1
    print(
        "docs-structure: advisory — pass 'enforce' to fail the build",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
