# NOLIT-CHANGE — `Change { … }` and `ChangeSet { … }` literals and patterns appear ONLY in
# src/changes.rs. Carried forward from markdown-viewer with its executable logic
# BYTE-IDENTICAL; only its MIN invocation moves, 17 -> 18, because src/ui/detail.rs is added.
# This change creates NO new Change or ChangeSet construction site: the tab-bar and header
# fixtures reach one through `changes::fixture::with_artifacts`, which is added INSIDE
# src/changes.rs precisely so this check does not have to be weakened.
#
# Known limits, stated rather than discovered later: the pattern also matches `impl Change {`
# and `struct Change {`, and it matches inside a doc comment — all three are FALSE POSITIVES
# that a future file outside src/changes.rs would trip, and the fix is to move the code, not
# to weaken the check. It is also dodged by `use crate::changes::Change as C;` followed by
# `C { … }` — a FALSE NEGATIVE, and the reason `GATE-MECH1`'s compile-time companion in
# `changes::conformance` remains the primary mechanism rather than this grep.
LIT_RE='(^|[^A-Za-z0-9_])Change(Set)?[[:space:]]*\{'
MIN="${MIN:-25}"

[ -f src/changes.rs ] || { echo "NOLIT-CHANGE FAIL: src/changes.rs missing" >&2; exit 1; }

# Guard — positive control: the allowed file must actually hold such a literal, or a broken
# pattern reads as a clean tree.
grep -qE 'ChangeSet[[:space:]]*\{' src/changes.rs \
  || { echo "NOLIT-CHANGE FAIL: positive control - src/changes.rs has no 'ChangeSet {'" >&2
       exit 1; }

# Guard — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/changes.rs is still searched.
n=$(find src -name '*.rs' ! -path 'src/changes.rs' | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || { echo "NOLIT-CHANGE FAIL: searched only $n files (expected >= $MIN)" >&2
                         exit 1; }

hits=$(find src -name '*.rs' ! -path 'src/changes.rs' -print0 \
       | xargs -0 -I{} grep -nE "$LIT_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NOLIT-CHANGE FAIL: a Change/ChangeSet literal outside src/changes.rs:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOLIT-CHANGE OK: $n files searched (>= $MIN), Change/ChangeSet literals only in src/changes.rs"
