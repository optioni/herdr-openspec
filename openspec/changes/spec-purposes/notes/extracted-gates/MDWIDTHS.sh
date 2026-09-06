# MDWIDTHS — NEW in markdown-viewer. Every #[test] in src/ui/markdown.rs names both 58 and
# 78, the two DETAIL-region interiors the mandated 60- and 120-column frames produce (a
# 60-column body less two border columns, and the wide layout's Min(0) column of 80 less two
# border columns). Same script as LISTWIDTHS pointed at a different file and a different
# pair; same stated limits; same pairing — it is a FLOOR, and
# `testcount --lib 'ui::markdown::tests::' 23` is what proves the tests exist and run.
# Because this block is new here, its default IS this change's final floor: 23.
#
# Known limit inherited from WIDTHS: the number scan is `\b(\d+)\b`, which does NOT see a
# suffixed literal such as `78u16`. It fails closed — a test using only suffixed literals is
# reported as missing a width — so write the widths unsuffixed.
#
# It has NO exemption list, and design.md -> Boundaries records the module split that makes
# that possible: every public function in src/ui/markdown.rs is parameterised by a width.
# `scroll_offset` and `interior` live in ui::layout and `normalise_scroll` lives in ui::app
# precisely so that no test in this file has a legitimate reason to name neither width. An
# exemption list is how a width check rots into a rubber stamp.
[ -f src/ui/markdown.rs ] || { echo "MDWIDTHS FAIL: src/ui/markdown.rs missing" >&2; exit 1; }
MD_MIN="${MD_MIN:-23}" python3 - <<'PY'
import re, sys, os
raw = open("src/ui/markdown.rs").read()
src = "\n".join(l for l in raw.splitlines()
                if not l.lstrip().startswith("///") and not l.lstrip().startswith("//!"))
parts = re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", src)[1:]
floor = int(os.environ["MD_MIN"])
if len(parts) < floor:
    print(f"MDWIDTHS FAIL: found {len(parts)} #[test] functions in src/ui/markdown.rs, expected >= {floor}",
          file=sys.stderr); sys.exit(1)
bad = []
for p in parts:
    name = re.search(r"fn\s+([a-z0-9_]+)", p)
    name = name.group(1) if name else "<unnamed>"
    nums = set(re.findall(r"\b(\d+)\b", p))
    if not ({"58", "78"} <= nums):
        bad.append(name)
if bad:
    print("MDWIDTHS FAIL: these markdown tests do not name both 58 and 78: " + ", ".join(bad),
          file=sys.stderr); sys.exit(1)
print(f"MDWIDTHS OK: all {len(parts)} markdown tests name both 58 and 78")
PY
