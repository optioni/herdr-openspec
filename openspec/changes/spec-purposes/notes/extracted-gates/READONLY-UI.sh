# READONLY-UI — NEW in tasks-tab, EDITED by live-refresh with ONE change, applied here and
# written out rather than described: the searched set gains an EXTRA list, so src/watch.rs
# and src/refresh.rs are covered by the same sweep. The watcher module is the new place a
# write is plausible (a "touch a marker file" reflex), and the worker is the new place a
# background thread could write beside the tree it is reading. Everything else — the
# #[cfg(test)] stripper, the two controls, the path-qualified patterns — is byte-identical.
#
# No PRODUCTION code under src/ui/ or in EXTRA names a filesystem-write API. The dashboard
# reads openspec/ and never writes there: an agent may be editing tasks.md in another pane.
# NOIO-VIEW forbids every I/O API in the eight PURE files; this is the complementary half,
# and it is the ONLY check that reaches src/ui/mod.rs — the one file under src/ui/ permitted
# to touch the filesystem at all (ui::read_artifact, ui::load, ui::run).
#
# Two deliberate design choices, each forced by a shape that is REALLY in this tree and each
# verified against it at planning time rather than assumed:
#  1. `#[cfg(test)]` modules are STRIPPED before the search. src/ui/mod.rs legitimately
#     calls std::fs::create_dir_all — inside ui::tests::load, which builds a ScratchDir
#     repository to drive ui::load against. A whole-file sweep would be RED on the unmodified
#     tree, and the only fixes would be to delete a real test or to exempt a file, which is
#     how a confinement check rots into a rubber stamp. Every src/ui/*.rs file holds at most
#     ONE line-anchored `#[cfg(test)]` and its test module runs to EOF, verified at planning
#     time, so "cut at the first one" is exact rather than approximate. The same holds for
#     src/watch.rs and src/refresh.rs, whose own tests build ScratchDir trees.
#  2. Every pattern is PATH-QUALIFIED (`fs::rename`, not `rename`). A bare `rename` matches
#     src/ui/app.rs's doc comment "`Next` and `Prev` are renamed from ...", correct prose,
#     which would make the check RED on the unmodified tree for the second time.
#
# Runs GREEN on unmodified `main` with EXTRA empty — verified at planning time, together with
# a planted `std::fs::write` in ui::read_artifact that it caught — so task 0.3 runs it for
# real rather than recording a guard failure. With EXTRA set it fails with "EXTRA file
# src/watch.rs missing" until group 1 creates the modules.
UIDIR="${UIDIR:-src/ui}"
CONTROL="${CONTROL:-src/state.rs}"
UI_MIN="${UI_MIN:-10}"
EXTRA="${EXTRA:-}"
WRITE_RE='fs::write|File::create|OpenOptions|fs::remove_|fs::create_dir|fs::rename|fs::copy|set_permissions|fs::hard_link|fs::soft_link'
fail() { echo "READONLY-UI FAIL: $1" >&2; exit 1; }

[ -d "$UIDIR" ] || fail "no such directory: $UIDIR"
for f in $EXTRA; do
  [ -f "$f" ] || fail "EXTRA file $f missing"
  # Guard D — prod() below discards everything from a file's FIRST line-anchored #[cfg(test)]
  # to EOF. src/refresh.rs is required to hold a second one (worker_for_test); if it sits above
  # start(), the worker's whole body is invisible here and a std::fs::write in it is reported
  # green. Measured. A count is a check; a placement rule is a convention.
  c=$(grep -c '^#\[cfg(test)\]$' "$f" || true)
  [ "$c" -eq 1 ] || fail "$f holds $c line-anchored #[cfg(test)] attributes, expected exactly 1 - the sweep below the first one is silent"
done
[ -f "$CONTROL" ] || fail "$CONTROL missing - the positive control has nothing to match"

# Guard A — the searched set is real.
n=$(find "$UIDIR" -name '*.rs' | wc -l | tr -d ' ')
[ "$n" -ge "$UI_MIN" ] || fail "only $n files under $UIDIR (expected >= $UI_MIN)"

# The production-only slice of a file: everything above its first line-anchored #[cfg(test)].
prod() { awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print}' "$1"; }

# Guard B — positive control, checked BEFORE the sweep. The control file's PRODUCTION slice
# must match the pattern, or the pattern is broken and a clean result means nothing.
prod "$CONTROL" | grep -qE "$WRITE_RE" \
  || fail "positive control - $CONTROL's production slice names no write API"

# Guard C — the stripper actually strips. src/ui/mod.rs's TEST slice must name a write API
# that its production slice does not, or "strip #[cfg(test)]" is doing nothing and the
# exemption above is silently exempting the whole file.
[ -f "$UIDIR/mod.rs" ] || fail "$UIDIR/mod.rs missing"
awk 'BEGIN{p=0} /^#\[cfg\(test\)\]$/{p=1} p{print}' "$UIDIR/mod.rs" | grep -qE "$WRITE_RE" \
  || fail "the stripper is vacuous - $UIDIR/mod.rs's test slice names no write API"

hits=""
for f in $(find "$UIDIR" -name '*.rs' | sort) $EXTRA; do
  h=$(prod "$f" | grep -nE "$WRITE_RE" | sed "s|^|$f:|" || true)
  hits="$hits$h"
done
[ -z "$hits" ] || { echo "READONLY-UI FAIL: a write API in production code under $UIDIR:" >&2
                    echo "$hits" >&2; exit 1; }
echo "READONLY-UI OK: $n files under $UIDIR plus [$EXTRA], no write API in production code; both controls matched"
