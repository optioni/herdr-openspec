# SETTINGSWIDTHS — NEW in settings-window, on HELPWIDTHS' own pattern (script shape copied
# verbatim, pointed at a different file). Like src/ui/help.rs, src/ui/settings.rs is NOT a
# module where every public function is parameterised by a width — content_rows, fit, and the
# text-building helpers take no width at all — so this is NOT an every-#[test]-names-both-
# widths rule the way DETAILWIDTHS/LISTWIDTHS/MDWIDTHS/TASKWIDTHS/WIDTHS are. It is a COUNT
# AGAINST A FLOOR: the number of src/ui/settings.rs tests that assert at both 60 and 120
# columns, plus the vacuity leg every other width gate carries (missing file, or zero #[test]
# functions at all).
#
# See openspec/changes/settings-window/specs/responsive-layout/spec.md -> "The settings panel
# gets its own both-widths gate".
#
# Script shape copied from HELPWIDTHS, which was itself copied from DETAILWIDTHS: the same
# #[test]-splitting scan, the same doc-comment stripping, the same unsuffixed-literal limit (a
# `78u16` is invisible to `\b(\d+)\b`, so a width must be written unsuffixed to be seen).
#
# The floor moves only up: adding a further both-width test raises the count, and a NEW
# single-width test raises the total without lowering it, so neither can fail this gate. What
# fails it is a both-width test NARROWED to one width alone — the defect
# `tests/gate-controls.toml`'s "settingswidths-narrowed" control plants.
[ -f src/ui/settings.rs ] || { echo "SETTINGSWIDTHS FAIL: src/ui/settings.rs missing" >&2; exit 1; }
SETTINGS_MIN="${SETTINGS_MIN:-5}" python3 - <<'PY'
import re, sys, os
raw = open("src/ui/settings.rs").read()
src = "\n".join(l for l in raw.splitlines()
                if not l.lstrip().startswith("///") and not l.lstrip().startswith("//!"))
parts = re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", src)[1:]
if not parts:
    print("SETTINGSWIDTHS FAIL: src/ui/settings.rs holds no #[test] functions", file=sys.stderr)
    sys.exit(1)
floor = int(os.environ["SETTINGS_MIN"])
both = 0
for p in parts:
    nums = set(re.findall(r"\b(\d+)\b", p))
    if {"60", "120"} <= nums:
        both += 1
if both < floor:
    print(f"SETTINGSWIDTHS FAIL: only {both} src/ui/settings.rs tests name both 60 and 120 "
          f"columns, expected >= {floor}", file=sys.stderr)
    sys.exit(1)
print(f"SETTINGSWIDTHS OK: {both} of {len(parts)} src/ui/settings.rs tests name both 60 and 120 columns (>= {floor})")
PY
