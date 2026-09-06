# NOTABSEAM — NEW in detail-view. src/ui/detail.rs is a PLAIN-DATA grammar module on exactly
# MDSEAM's single-file terms: it names no ratatui type, so styling stays ui::view's job and
# every function in it can be asserted without a frame. This is the structural precondition
# that makes DETAILWIDTHS possible without an exemption list.
#
# It searches src/ui/detail.rs and NOTHING ELSE, and that is deliberate rather than lazy.
# Extending the same whole-file sweep to src/ui/list.rs was written and REJECTED during
# planning: list.rs:2 says "with no ratatui styling" and list.rs:22 says "`ui::view` applies
# `Modifier::BOLD` to the selected one", both correct prose, so the leg would have been RED
# on the unmodified tree and the only fixes would have been to reword true comments or to
# exempt comments — which is how a confinement check rots into a rubber stamp. list.rs's
# purity is not this change's claim, and this change adds no ratatui import to it.
#
# Known limit, stated rather than discovered later, and it applies to THIS file: the sweep is
# over the whole file, comments included. src/ui/detail.rs's doc comment must therefore say
# "the view" rather than naming a ratatui type, exactly as src/ui/markdown.rs's must. A task
# in group 4 carries that as its own Red-when.
DT="${DT:-src/ui/detail.rs}"
[ -f "$DT" ] || { echo "NOTABSEAM FAIL: $DT missing" >&2; exit 1; }

# Positive control — the file must actually hold the grammar, or a clean result means
# nothing. Anchored on `pub fn tab_bar`, the function this check exists to keep honest.
grep -qE '^pub fn tab_bar' "$DT" \
  || { echo "NOTABSEAM FAIL: positive control - $DT has no 'pub fn tab_bar'" >&2; exit 1; }

r=$(grep -nE 'ratatui|Modifier|Style|Span|Rect|Frame|Buffer' "$DT" || true)
[ -z "$r" ] || { echo "NOTABSEAM FAIL: $DT names a ratatui type:" >&2; echo "$r" >&2; exit 1; }
echo "NOTABSEAM OK: $DT names no ratatui type; positive control matched"
