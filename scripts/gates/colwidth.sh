#!/bin/sh
set -u
PURE="src/ui/app.rs src/ui/detail.rs src/ui/list.rs src/ui/markdown.rs src/ui/tasks.rs src/ui/view.rs src/ui/driver.rs"
PAT='\.chars\(\)\.count\(\)|\.chars\(\)\.take\(|Vec<char>'
# Control 1: the sweep's OWN pattern must match a line holding all three forms.
probe='let v: Vec<char> = s.chars().collect(); s.chars().count(); s.chars().take(1);'
[ "$(printf '%s\n' "$probe" | grep -cE "$PAT")" = "1" ] \
  || { echo "COLWIDTH FAIL: self-test - the sweep pattern matches nothing" >&2; exit 1; }
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
