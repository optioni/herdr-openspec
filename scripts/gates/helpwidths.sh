# HELPWIDTHS — NEW in help-overlay. Unlike DETAILWIDTHS/LISTWIDTHS/MDWIDTHS/TASKWIDTHS/WIDTHS,
# src/ui/help.rs is NOT a module where every public function is parameterised by a width, so
# this is NOT an every-#[test]-names-both-widths rule. It is a COUNT AGAINST A FLOOR: the
# number of src/ui/help.rs tests that assert at both 60 and 120 columns, plus the vacuity leg
# every other width gate carries (missing file, or zero #[test] functions at all).
#
# See openspec/changes/help-overlay/specs/responsive-layout/spec.md -> "The overlay's
# both-widths rule is counted, not just stated". Several of help.rs's tests assert
# INVENTORY as data and name no width because there is no width in what they assert; two —
# "The grammar renders at 120 columns" and "at 60 columns" — are a deliberate one-width pair,
# because the spec states them as two scenarios; one drives `apply` and renders nothing at
# all. `settings-window`'s "The settings group renders at both mandated widths" raised the
# remaining both-width count from three to four — HELP_MIN's measured floor.
#
# Script shape copied from DETAILWIDTHS: the same #[test]-splitting scan, the same
# doc-comment stripping, the same unsuffixed-literal limit (a `78u16` is invisible to
# `\b(\d+)\b`, so a width must be written unsuffixed to be seen).
#
# The floor moves only up: adding a further both-width test raises the count, and a NEW
# single-width test raises the total without lowering it, so neither can fail this gate. What
# fails it is a both-width test NARROWED to one width alone — the defect
# `tests/gate-controls.toml`'s "helpwidths-narrowed" control plants.
[ -f src/ui/help.rs ] || { echo "HELPWIDTHS FAIL: src/ui/help.rs missing" >&2; exit 1; }
HELP_MIN="${HELP_MIN:-4}" python3 - <<'PY'
import re, sys, os
raw = open("src/ui/help.rs").read()
src = "\n".join(l for l in raw.splitlines()
                if not l.lstrip().startswith("///") and not l.lstrip().startswith("//!"))
parts = re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", src)[1:]
if not parts:
    print("HELPWIDTHS FAIL: src/ui/help.rs holds no #[test] functions", file=sys.stderr)
    sys.exit(1)
floor = int(os.environ["HELP_MIN"])
both = 0
for p in parts:
    nums = set(re.findall(r"\b(\d+)\b", p))
    if {"60", "120"} <= nums:
        both += 1
if both < floor:
    print(f"HELPWIDTHS FAIL: only {both} src/ui/help.rs tests name both 60 and 120 columns, "
          f"expected >= {floor}", file=sys.stderr)
    sys.exit(1)
print(f"HELPWIDTHS OK: {both} of {len(parts)} src/ui/help.rs tests name both 60 and 120 columns (>= {floor})")
PY
