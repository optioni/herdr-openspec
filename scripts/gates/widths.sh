# WIDTHS — every #[test] in src/ui/view.rs names both 60 and 120. A heuristic, and design.md
# -> Risks says so: it cannot prove an assertion is meaningful, only that both widths are
# present. It is the cheap half of the mandate; the per-scenario tasks are the other half.
# Carried forward from markdown-viewer with its executable logic BYTE-IDENTICAL; only
# WIDTHS_MIN's invocation moves, 57 -> 72 -> 101 (view-fidelity, per design.md -> Decision 4:
# gate floors move up, because they are floors).
#
# Known limits, stated rather than discovered later: a `60` in a comment satisfies it, and
# so does an unrelated `60` literal. It is a FLOOR. What proves the tests exist and run is
# `testcount --lib 'ui::view::tests::' 101`, not this.
[ -f src/ui/view.rs ] || { echo "WIDTHS FAIL: src/ui/view.rs missing" >&2; exit 1; }
WIDTHS_MIN="${WIDTHS_MIN:-113}" python3 - <<'PY'
import re, sys, os
raw = open("src/ui/view.rs").read()
# Drop doc-comment and inner-doc lines before splitting: a `#[test]` inside a doc example
# would otherwise inflate the count and let a gutted file pass the floor.
src = "\n".join(l for l in raw.splitlines()
                if not l.lstrip().startswith("///") and not l.lstrip().startswith("//!"))
# Split on a line-anchored attribute, so `#[test]` inside a string or a trailing comment
# does not create a part.
parts = re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", src)[1:]
floor = int(os.environ["WIDTHS_MIN"])
if len(parts) < floor:
    print(f"WIDTHS FAIL: found {len(parts)} #[test] functions in src/ui/view.rs, expected >= {floor}",
          file=sys.stderr); sys.exit(1)
bad = []
for p in parts:
    name = re.search(r"fn\s+([a-z0-9_]+)", p)
    name = name.group(1) if name else "<unnamed>"
    nums = set(re.findall(r"\b(\d+)\b", p))
    if not ({"60", "120"} <= nums):
        bad.append(name)
if bad:
    print("WIDTHS FAIL: these view tests do not name both 60 and 120: " + ", ".join(bad),
          file=sys.stderr); sys.exit(1)
print(f"WIDTHS OK: all {len(parts)} view tests name both 60 and 120")
PY
