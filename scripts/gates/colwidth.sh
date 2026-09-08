#!/bin/sh
set -u
PURE="src/ui/app.rs src/ui/detail.rs src/ui/list.rs src/ui/markdown.rs src/ui/tasks.rs src/ui/view.rs src/ui/driver.rs"
PAT='\.chars\(\)\.count\(\)|\.chars\(\)\.take\(|Vec<char>'
# Control 1: the sweep's OWN pattern must independently match EACH of the three
# alternatives, one probe per form. A single combined probe line holding all three
# forms is the wrong test: `grep -c` counts matching LINES, not matches, so a
# combined probe returns 1 as soon as ANY ONE alternative still matches, and stays
# silently green while the other two rot out of the pattern.
probe_count='s.chars().count();'
[ "$(printf '%s\n' "$probe_count" | grep -cE "$PAT")" = "1" ] \
  || { echo "COLWIDTH FAIL: self-test - the sweep pattern no longer matches .chars().count()" >&2; exit 1; }
probe_take='s.chars().take(1);'
[ "$(printf '%s\n' "$probe_take" | grep -cE "$PAT")" = "1" ] \
  || { echo "COLWIDTH FAIL: self-test - the sweep pattern no longer matches .chars().take(" >&2; exit 1; }
probe_vec_char='let v: Vec<char> = s.chars().collect();'
[ "$(printf '%s\n' "$probe_vec_char" | grep -cE "$PAT")" = "1" ] \
  || { echo "COLWIDTH FAIL: self-test - the sweep pattern no longer matches Vec<char>" >&2; exit 1; }
# Control 2: the measure being protected exists, so the exemption is not vacuous.
[ -f src/ui/layout.rs ] || { echo "COLWIDTH FAIL: src/ui/layout.rs missing" >&2; exit 1; }
grep -q 'cell_width' src/ui/layout.rs && grep -q 'styled_graphemes' src/ui/layout.rs \
  || { echo "COLWIDTH FAIL: positive control - src/ui/layout.rs names no cell_width/styled_graphemes" >&2; exit 1; }
bad=0
for f in $PURE; do
  [ -f "$f" ] || { echo "COLWIDTH FAIL: $f missing" >&2; exit 1; }
  n=$(awk '/^#\[cfg\(test\)\]/{exit} {print}' "$f" | grep -cE "$PAT")
  if [ "$n" != "0" ]; then
    echo "COLWIDTH FAIL: $f has $n char-count measurement(s) in production code" >&2
    awk '/^#\[cfg\(test\)\]/{exit} {print NR": "$0}' "$f" | grep -E "$PAT" >&2
    bad=$((bad+1))
  fi
done
[ "$bad" = "0" ] || exit 1
echo "COLWIDTH OK: no char-count measurement in the seven pure view files"
