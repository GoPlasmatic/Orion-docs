#!/usr/bin/env bash
# docs-lint — structural and link integrity linter for Orion documentation.
set -uo pipefail
cd "$(dirname "$0")"

export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_SYSTEM=/dev/null
export PYTHONDONTWRITEBYTECODE=1

if python3 -c "import pathlib" >/dev/null 2>&1; then
  PYTHON=python3
elif [ -x /usr/bin/python3 ]; then
  PYTHON=/usr/bin/python3
else
  PYTHON=python3
fi

DOCS2_PHASE=4
# Set DOCS_CHECK_LLMS=0 while migrating SUMMARY.md across phases; Phase 7 sets it to 1.
DOCS_CHECK_LLMS="${DOCS_CHECK_LLMS:-1}"

fail=0
err() { printf 'docs-lint: %s\n' "$*" >&2; fail=1; }

## 1. {{#include}} targets resolve.
while IFS= read -r hit; do
  file=${hit%%:*}
  rest=${hit#*:}
  line=${rest%%:*}
  body=${rest#*:}
  for t in $(printf '%s' "$body" | grep -oE '\{\{#include [^}]+\}\}' | sed -E 's/\{\{#include +([^:}]+).*/\1/'); do
    [ -f "$(dirname "$file")/$t" ] || err "$file:$line: include target missing: $t"
  done
done < <(grep -rn '{{#include ' src --include='*.md' 2>/dev/null)

## 2. Relative .md links point at files that exist.
while IFS= read -r hit; do
  file=${hit%%:*}
  rest=${hit#*:}
  line=${rest%%:*}
  body=${rest#*:}
  for l in $(printf '%s' "$body" | grep -oE '\]\([^)#[:space:]]+\.md' | sed 's/^](//'); do
    case "$l" in http://*|https://*|/*) continue ;; esac
    [ -f "$(dirname "$file")/$l" ] || err "$file:$line: dead link: $l"
  done
done < <(grep -rnE '\]\([^)]*\.md[#)]' src --include='*.md' 2>/dev/null)

## 3. Hard site links (README.md, llms.txt) resolve to a live page or a redirect in book.toml.
for f in README.md src/llms.txt; do
  [ -f "$f" ] || continue
  while IFS= read -r url; do
    p=${url#docs.goplasmatic.io/}
    p=${p%%#*}
    case "$p" in *.html) ;; *) continue ;; esac
    if [ ! -f "src/${p%.html}.md" ] && ! grep -qF "\"/${p}\"" book.toml; then
      err "$f: site link has no page and no redirect: $url"
    fi
  done < <(grep -oE 'docs\.goplasmatic\.io/[A-Za-z0-9_./#-]+' "$f" | sort -u)
done

## 4. No hand-maintained magic numbers.
if git grep -I -nE '46 tools|6,000' -- src README.md >/dev/null 2>&1; then
  git grep -I -nE '46 tools|6,000' -- src README.md >&2
  err 'hand-maintained magic number (46 tools / 6,000+)'
fi

## 5. The function count is stated only in the functions reference.
stray=$(git grep -I -lE '24 functions|24 built-in' -- src 2>/dev/null | grep -v 'reference/functions' || true)
[ -z "$stray" ] || err "function count '24' stated outside reference/functions: $stray"

## 6. No internal review IDs in user docs.
hits=$(git grep -I -nE '\((K|R|F|N|S)[0-9]+(, ?(K|R|F|N|S)[0-9]+)*\)' -- src \
       | grep -v 'removed in 1.0 (K4)' || true)
if [ -n "$hits" ]; then
  printf '%s\n' "$hits" >&2
  err 'internal review IDs in user docs'
fi

## 7. No Rust internals outside architecture / design-notes.
if [ "$DOCS2_PHASE" -ge 3 ]; then
  if git grep -I -nE 'Arc<RwLock|tokio::sync::mpsc|apply_guards|CatchPanicLayer|arena-mode' -- src ':!src/concepts/design-notes.md' ':!src/concepts/architecture.md' >/dev/null 2>&1; then
    git grep -I -nE 'Arc<RwLock|tokio::sync::mpsc|apply_guards|CatchPanicLayer|arena-mode' -- src ':!src/concepts/design-notes.md' ':!src/concepts/architecture.md' >&2
    err 'Rust internals outside concepts/architecture.md or concepts/design-notes.md'
  fi
fi

## 8, 9, 10. llms.txt synchronization with SUMMARY.md.
if [ "$DOCS2_PHASE" -ge 4 ] && [ "$DOCS_CHECK_LLMS" -eq 1 ]; then
  while IFS= read -r p; do
    html="${p%.md}.html"
    grep -qF "docs.goplasmatic.io/${html}" src/llms.txt \
      || err "llms.txt: no entry for SUMMARY chapter $p"
  done < <(grep -oE '\(\./[A-Za-z0-9_./-]+\.md\)' src/SUMMARY.md | sed 's/^(\.\///; s/)$//')

  while IFS= read -r entry; do
    title=${entry%%](*}; title=${title#*[}
    url=${entry#*](}; url=${url%)}
    p=${url#https://docs.goplasmatic.io/}
    p=${p%%#*}
    case "$p" in *.html) ;; *) continue ;; esac
    chapter=$(grep -F "](./${p%.html}.md)" src/SUMMARY.md || true)
    if [ -z "$chapter" ]; then
      err "llms.txt: entry for a page with no SUMMARY chapter: $p"
      continue
    fi
    chapter_title=${chapter%%](*}; chapter_title=${chapter_title#*[}
    [ "$title" = "$chapter_title" ] \
      || err "llms.txt: entry titled '$title' but SUMMARY calls $p '$chapter_title'"
  done < <(grep -oE '^- \[[^]]+\]\(https://docs\.goplasmatic\.io/[A-Za-z0-9_./#-]+\)' src/llms.txt)
fi

## 11. Mindmap node links resolve to a real page or redirect.
for f in $(grep -rl 'orion-mindmap' src --include='*.md' 2>/dev/null); do
  while IFS= read -r hit; do
    line=${hit%%:*}
    body=${hit#*:}
    for l in $(printf '%s' "$body" | grep -oE '"link" *: *"[^"]+"' | sed -E 's/.*"link" *: *"([^"#]+).*/\1/'); do
      case "$l" in http://*|https://*) continue ;; esac
      if [ ! -f "src/${l%.html}.md" ] && ! grep -qF "\"/${l}\"" book.toml; then
        err "$f:$line: mindmap link has no page: $l"
      fi
    done
  done < <(grep -n '"link" *: *"' "$f" 2>/dev/null)
done

## 13. The Required column in field tables is lowercase (yes / no).
req_case=$("$PYTHON" - <<'PY'
import pathlib
bad = []
for f in sorted(pathlib.Path('src').rglob('*.md')):
    hdr = idx = None
    for n, l in enumerate(f.read_text().split('\n'), 1):
        if not l.startswith('|'):
            hdr = idx = None
            continue
        cells = [c.strip() for c in l.strip().strip('|').split('|')]
        if hdr is None:
            hdr = cells
            low = [c.lower().strip('`* ') for c in cells]
            idx = low.index('required') if 'required' in low else None
            continue
        if set(''.join(cells)) <= set('-: '):
            continue
        if idx is not None and idx < len(cells) and cells[idx] in ('Yes', 'No'):
            bad.append(f"{f}:{n}: Required column says '{cells[idx]}', want lower case")
print('\n'.join(bad))
PY
)
if [ -n "$req_case" ]; then
  printf '%s\n' "$req_case" >&2
  err 'capitalised Yes/No in a Required column'
fi

## 14. No decorative emoji in src markdown files.
emoji_re='(*UTF)[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}\x{2B00}-\x{2BFF}\x{FE0F}]'
emoji_hits=$(git grep -I -nP "$emoji_re" -- 'src/**/*.md' 'src/*.md' 2>&1)
case $? in
0)
  printf '%s\n' "$emoji_hits" >&2
  err 'emoji or dingbat in src — use an SVG mark'
  ;;
1) ;; # clean
*)
  printf '%s\n' "$emoji_hits" >&2
  err 'the emoji check could not run (git grep failed)'
  ;;
esac

## 15. Every SUMMARY chapter carries a unique `<!-- description: ... -->` (105-165 chars).
desc_report=$("$PYTHON" - <<'DESCPY'
import pathlib, re, collections

summary = pathlib.Path('src/SUMMARY.md').read_text()
chapters = [m[1] for m in re.findall(r'\[([^\]]+)\]\(\./([A-Za-z0-9_./-]+\.md)\)', summary)]

problems, seen = [], collections.defaultdict(list)
for rel in chapters:
    p = pathlib.Path('src') / rel
    if not p.exists():
        continue
    first = p.read_text().split('\n', 1)[0]
    m = re.match(r'^<!--\s*description:\s*(.*?)\s*-->$', first)
    if not m:
        problems.append(f"src/{rel}: no '<!-- description: ... -->' on line 1")
        continue
    d = m.group(1)
    n = len(d)
    if not (105 <= n <= 165):
        problems.append(f"src/{rel}: description is {n} chars, want 105-165")
    seen[d].append(rel)

for d, files in seen.items():
    if len(files) > 1:
        problems.append(f"duplicate description shared by {', '.join(files)}")

print('\n'.join(problems))
DESCPY
)
if [ -n "$desc_report" ]; then
  printf '%s\n' "$desc_report" >&2
  err 'page description missing, mis-sized or duplicated (seo.mjs reads these)'
fi

## 16. Chapter titles are unique.
title_report=$("$PYTHON" - <<'TITLEPY'
import collections, pathlib, re

summary = pathlib.Path('src/SUMMARY.md').read_text()
paths = [m[1] for m in re.findall(r'\[([^\]]+)\]\(\./([A-Za-z0-9_./-]+\.md)\)', summary)]
seen = collections.defaultdict(list)
for rel in paths:
    p = pathlib.Path('src') / rel
    if not p.exists():
        continue
    h1 = next((line[2:].strip() for line in p.read_text().splitlines()
               if line.startswith('# ')), None)
    if h1:
        seen[h1.casefold()].append((rel, h1))
for entries in seen.values():
    if len(entries) > 1:
        print('duplicate H1:', ', '.join(f'{p} ({h})' for p, h in entries))
TITLEPY
)
if [ -n "$title_report" ]; then
  printf '%s\n' "$title_report" >&2
  err 'duplicate chapter titles'
fi

## 17. Relative Markdown links carrying a fragment resolve to a real heading.
fragment_report=$("$PYTHON" - <<'FRAGPY'
import pathlib, re, urllib.parse

root = pathlib.Path('src')

def anchors(path):
    found, counts, fenced = set(), {}, False
    for line in path.read_text().splitlines():
        if line.startswith('```'):
            fenced = not fenced
            continue
        if fenced:
            continue
        match = re.match(r'^(#{1,6})\s+(.+?)\s*#*$', line)
        if not match:
            continue
        slug = re.sub(r'<[^>]+>', '', match.group(2))
        slug = re.sub(r'[`*~]', '', slug).lower()
        slug = re.sub(r'[^\w\- ]', '', slug).replace(' ', '-')
        duplicate = counts.get(slug, 0)
        counts[slug] = duplicate + 1
        if duplicate:
            slug = f'{slug}-{duplicate}'
        found.add(slug)
    return found

for source in sorted(root.rglob('*.md')):
    for match in re.finditer(r'\]\(([^)\s]+\.md)#([^)\s]+)\)', source.read_text()):
        target = (source.parent / match.group(1)).resolve()
        fragment = urllib.parse.unquote(match.group(2))
        if target.exists() and fragment not in anchors(target):
            print(f'{source}: missing fragment #{fragment} in {target}')
FRAGPY
)
if [ -n "$fragment_report" ]; then
  printf '%s\n' "$fragment_report" >&2
  err 'broken relative fragment links'
fi

## 18. Executable tutorials state the Orion version they were tested with.
tutorial_report=$("$PYTHON" - <<'TUTORIALPY'
import pathlib, re

cargo = pathlib.Path('drift/Cargo.toml').read_text()
tags = set(re.findall(r'git = "https://github\.com/GoPlasmatic/Orion", tag = "v([^"]+)"', cargo))
if len(tags) != 1:
    print(f'drift/Cargo.toml: expected one Orion tag across the orion-* dependencies, found {sorted(tags)}')
    raise SystemExit
version = tags.pop()
candidates = [
    'get-started/quickstart.md',
    'get-started/install.md',
    'get-started/first-service.md',
    'get-started/postgres-orders-api.md',
    'get-started/examples.md',
    'guides/ci-cd.md',
    'guides/ai-authoring.md',
]
for rel in candidates:
    path = pathlib.Path('src') / rel
    if not path.exists():
        continue
    text = path.read_text()
    match = re.search(r'Tested with Orion (\S+)\.', text)
    if not match:
        print(f'{path}: no "Tested with Orion <version>." line')
    elif match.group(1) != version:
        print(f'{path}: tested with {match.group(1)}, workspace version is {version}')
TUTORIALPY
)
if [ -n "$tutorial_report" ]; then
  printf '%s\n' "$tutorial_report" >&2
  err 'tutorial test metadata missing or stale'
fi

## 19. Retired editorial terms and unsafe quickstart shortcuts do not return.
forbidden=$(grep -rnE 'CI-CD|The Console \(Orion UI\)|raw\.githubusercontent\.com/GoPlasmatic/Orion/main/examples/quickstart\.sh *\| *bash' \
  src --include='*.md' 2>/dev/null || true)
if [ -n "$forbidden" ]; then
  printf '%s\n' "$forbidden" >&2
  err 'deprecated terminology or pipe-to-shell quickstart'
fi

## 20. Vendored JSON examples and openapi.json parse cleanly.
json_report=$("$PYTHON" - <<'JSONPY'
import json, pathlib

paths = [pathlib.Path('openapi.json')]
paths.extend(pathlib.Path('examples').rglob('*.json'))
for path in sorted(paths):
    try:
        json.loads(path.read_text())
    except Exception as exc:
        print(f'{path}: {exc}')
JSONPY
)
if [ -n "$json_report" ]; then
  printf '%s\n' "$json_report" >&2
  err 'invalid JSON in a copyable repository example'
fi

## 21. Page structure and metadata conformance.
DOCS_STRUCTURE_LEVEL=enforce

if ! "$PYTHON" structure_lint.py "$DOCS_STRUCTURE_LEVEL"; then
  err 'page structure does not match DOCUMENTATION_STANDARD.md'
fi

if [ "$fail" -eq 0 ]; then
  echo 'docs-lint: OK'
else
  exit 1
fi
