# TASKSEAM — NEW in tasks-tab. src/ui/tasks.rs is a PLAIN-DATA grammar module on exactly
# MDSEAM's and NOTABSEAM's single-file terms: it names no ratatui type, so styling stays
# ui::view's job and every function in it can be asserted without a frame. This is the
# structural precondition that makes TASKWIDTHS possible without an exemption list.
#
# It searches src/ui/tasks.rs and NOTHING ELSE, for the reason NOTABSEAM records: extending
# the same whole-file sweep to a file whose correct prose names a ratatui type would be RED on
# an unmodified tree, and the only fixes would be to reword true comments or to exempt
# comments — which is how a confinement check rots into a rubber stamp.
#
# Known limit, stated rather than discovered later, and it applies to THIS file: the sweep is
# over the whole file, comments included. src/ui/tasks.rs's doc comments must therefore say
# "the view" rather than naming a ratatui type, exactly as src/ui/markdown.rs's and
# src/ui/detail.rs's must. Task 4.2 carries that as its own Red-when.
#
# This check FAILS with "missing" until group 4 creates the file; task 0.3 records that and
# task 4.4 is its first green run.
DT="${DT:-src/ui/tasks.rs}"
[ -f "$DT" ] || { echo "TASKSEAM FAIL: $DT missing" >&2; exit 1; }

# Positive control — the file must actually hold the grammar, or a clean result means
# nothing. Anchored on `pub fn progress_bar`, the function this check exists to keep honest.
grep -qE '^pub fn progress_bar' "$DT" \
  || { echo "TASKSEAM FAIL: positive control - $DT has no 'pub fn progress_bar'" >&2; exit 1; }

r=$(grep -nE 'ratatui|Modifier|Style|Span|Rect|Frame|Buffer' "$DT" || true)
[ -z "$r" ] || { echo "TASKSEAM FAIL: $DT names a ratatui type:" >&2; echo "$r" >&2; exit 1; }

# Second leg — the module never reaches the filesystem edge. crate::tasks::read is what would
# turn this pure module into an I/O one. NOIO-VIEW now names it too, tree-wide over the eight
# PURE files; this leg is the narrower, file-scoped restatement and also catches fs::read and
# read_to_string spelled locally.
b=$(grep -nE 'tasks::read|fs::read|read_to_string' "$DT" || true)
[ -z "$b" ] || { echo "TASKSEAM FAIL: $DT reaches the filesystem edge:" >&2; echo "$b" >&2; exit 1; }

# Third leg — the module RENDERS the existing parse rather than reimplementing it. Two things
# make this leg correct rather than merely present, both established by RUNNING it during
# planning review:
#
#  (a) It is CONDITIONAL on `pub fn lines` existing. Group 4 creates this file holding only
#      progress_bar, which is built from ui::list::progress_cell and crate::tasks::Progress and
#      legitimately calls no parser. An unconditional leg is RED at task 4.4 on a correct tree,
#      and the cheapest way out of that false red is to name tasks::parse in a comment — which
#      rubber-stamps the leg for the rest of the change. The parse arrives with `lines` in
#      group 5, so the leg arms itself exactly then.
#  (b) It matches a CALL SHAPE over a COMMENT-STRIPPED copy. Verified during planning: a file
#      whose doc comment said "Built on crate::tasks::parse" while its lines() hand-rolled
#      `for l in source.lines()` satisfied a bare whole-file `grep -qE 'tasks::parse'`. The
#      first and second legs above are deliberately comment-INCLUSIVE, because a comment naming
#      a ratatui type or a read is itself the thing they forbid; this leg is the opposite — a
#      comment naming the parse is not the parse — so it alone strips comments first.
if grep -qE '^pub fn lines' "$DT"; then
  DT="$DT" python3 -c '
import os, re, sys
raw = open(os.environ["DT"]).read()
code = "\n".join(l for l in raw.splitlines() if not l.lstrip().startswith("//"))
if not re.search(r"tasks::parse\s*\(", code):
    print("TASKSEAM FAIL: no `tasks::parse(` call outside comments - the parse is being "
          "reimplemented, or only mentioned in prose", file=sys.stderr)
    sys.exit(1)
print("TASKSEAM OK (parse leg): tasks::parse is called, not merely named")
' || exit 1
else
  echo "TASKSEAM: parse leg not armed - $DT has no 'pub fn lines' yet (expected until group 5)"
fi
echo "TASKSEAM OK: $DT names no ratatui type and reaches no filesystem edge"
