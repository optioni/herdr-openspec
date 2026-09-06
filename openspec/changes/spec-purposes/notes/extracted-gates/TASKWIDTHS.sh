# TASKWIDTHS — NEW in tasks-tab. Every #[test] in src/ui/tasks.rs names both 58 and 78, the
# two DETAIL-region interiors the mandated 60- and 120-column frames produce (a 60-column
# body less two border columns, and the wide layout's Min(0) column of 80 less two border
# columns). Same script as WIDTHS, LISTWIDTHS, MDWIDTHS, and DETAILWIDTHS pointed at a
# different file and the same pair DETAILWIDTHS uses; same stated limits; same pairing — it
# is a FLOOR, and `testcount --lib 'ui::tasks::tests::' 14` is what proves the tests exist
# and run.
#
# Known limits, stated rather than discovered later: a `58` in a comment satisfies it, and so
# does an unrelated `58` literal. The number scan is `\b(\d+)\b`, which does NOT see a
# suffixed literal such as `78u16` — it fails closed, reporting such a test as missing a
# width, so write the widths unsuffixed.
#
# It has NO exemption list, and design.md -> Boundaries records what makes that possible:
# every public function in src/ui/tasks.rs is parameterised by a width. The width-sweep tests
# (progress_bar over 0..=120 and over 0..=30) are the ones an exemption would be reached for;
# their spec scenarios name 58 and 78 explicitly as swept values precisely so no exemption is
# needed. An exemption list is how a width check rots into a rubber stamp.
#
# This check FAILS with "missing" until group 4 creates the file; task 0.3 records that.
[ -f src/ui/tasks.rs ] || { echo "TASKWIDTHS FAIL: src/ui/tasks.rs missing" >&2; exit 1; }
TASK_MIN="${TASK_MIN:-14}" python3 - <<'PY'
import re, sys, os
raw = open("src/ui/tasks.rs").read()
src = "\n".join(l for l in raw.splitlines()
                if not l.lstrip().startswith("///") and not l.lstrip().startswith("//!"))
parts = re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", src)[1:]
floor = int(os.environ["TASK_MIN"])
if len(parts) < floor:
    print(f"TASKWIDTHS FAIL: found {len(parts)} #[test] functions in src/ui/tasks.rs, expected >= {floor}",
          file=sys.stderr); sys.exit(1)
bad = []
for p in parts:
    name = re.search(r"fn\s+([a-z0-9_]+)", p)
    name = name.group(1) if name else "<unnamed>"
    nums = set(re.findall(r"\b(\d+)\b", p))
    if not ({"58", "78"} <= nums):
        bad.append(name)
if bad:
    print("TASKWIDTHS FAIL: these tasks tests do not name both 58 and 78: " + ", ".join(bad),
          file=sys.stderr); sys.exit(1)
print(f"TASKWIDTHS OK: all {len(parts)} tasks tests name both 58 and 78")
PY
