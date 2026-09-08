# LISTWIDTHS — every #[test] in src/ui/list.rs names both 38 and 58, the two list-region
# INTERIORS the mandated 120- and 60-column frames produce. Carried forward from list-view
# BYTE-IDENTICALLY, invocation logic included: only LIST_MIN's default moves, 32 -> 39, per
# view-fidelity's re-measurement (design.md -> Decision 4: gate floors move up, because they
# are floors).
#
# Known limit inherited from WIDTHS, stated rather than discovered later: the number scan is
# `\b(\d+)\b`, which does NOT see a suffixed literal such as `38u16` or `38usize`. It fails
# closed — a test using only suffixed literals is reported as missing a width — so write the
# widths unsuffixed, or as a `const` the test also names bare.
[ -f src/ui/list.rs ] || { echo "LISTWIDTHS FAIL: src/ui/list.rs missing" >&2; exit 1; }
LIST_MIN="${LIST_MIN:-39}" python3 - <<'PY'
import re, sys, os
raw = open("src/ui/list.rs").read()
src = "\n".join(l for l in raw.splitlines()
                if not l.lstrip().startswith("///") and not l.lstrip().startswith("//!"))
parts = re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", src)[1:]
floor = int(os.environ["LIST_MIN"])
if len(parts) < floor:
    print(f"LISTWIDTHS FAIL: found {len(parts)} #[test] functions in src/ui/list.rs, expected >= {floor}",
          file=sys.stderr); sys.exit(1)
bad = []
for p in parts:
    name = re.search(r"fn\s+([a-z0-9_]+)", p)
    name = name.group(1) if name else "<unnamed>"
    nums = set(re.findall(r"\b(\d+)\b", p))
    if not ({"38", "58"} <= nums):
        bad.append(name)
if bad:
    print("LISTWIDTHS FAIL: these row-grammar tests do not name both 38 and 58: " + ", ".join(bad),
          file=sys.stderr); sys.exit(1)
print(f"LISTWIDTHS OK: all {len(parts)} row-grammar tests name both 38 and 58")
PY
