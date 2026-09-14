# NOIO-VIEW — the PURE files of the render seam name no I/O API at all.
# Carried forward from detail-view, which added src/ui/tasks.rs, and then from
# color-palette (design.md -> Boundaries), which adds the colour table: PURE is NINE files
# rather than eight. The palette belongs in the swept set rather than exempted from it —
# it is a constant table, so it never reads NO_COLOR, never probes the terminal, and never
# branches on a colour capability, and this is the gate that proves it.
# Three files under src/ui/ are still deliberately NOT searched, each for a stated reason:
# mod.rs holds ui::load, ui::read_artifact, and ui::run; terminal.rs holds the terminal seam;
# and event.rs holds CrosstermEvents, which reads the real event stream. Do not "fix" the
# list by adding them. A view test that needs a real directory is the signal this check
# exists to make impossible.
#
# src/ui/tasks.rs belongs in PURE because it calls crate::tasks::parse — a pure function over
# a &str — and never crate::tasks::read, which is the filesystem edge. The artifact bytes
# still arrive through detail.source, filled by Dashboard::sync_detail from the injected
# reader before the draw. READSEAM is the complementary half: it proves the single real
# binding is where design.md says it is.
#
# This check FAILS with "src/ui/tasks.rs missing" until group 4 creates that file. That is
# the [ -f ] guard doing its job, not a defect: without it, a renamed or deleted module makes
# the check report a clean tree. Task 0.3 records the expected failure; task 4.4 is its first
# green run.
#
# Do NOT write either swept path as a contiguous literal anywhere in this file except on the
# PURE line below. The two are `src/ui/{help,palette}.rs` — written braced here precisely so
# this comment is not itself a match. `palette.sh`'s third leg greps THIS WHOLE FILE for each
# of those names, not the PURE assignment alone, so a mention in a comment satisfies it
# vacuously and the leg stops being able to see either file dropped from PURE. The failure is
# loud rather than silent — `tests/gate-controls.toml`'s `palette-unswept` and
# `palette-help-unswept` controls strike the name from PURE and require PALETTE to fail, so a
# stray comment mention turns up as a RED `cargo test --test gate_controls` — but the message
# names PALETTE, not the comment that broke it, which is why the rule is written down here.
# help-overlay adds the overlay's module below, taking PURE from nine files to TEN: its
# bindings and rendering are pure data and a pure draw function, on exactly the same terms as
# every other file already swept here.
PURE="src/ui/app.rs src/ui/detail.rs src/ui/help.rs src/ui/layout.rs src/ui/list.rs src/ui/markdown.rs src/ui/palette.rs src/ui/tasks.rs src/ui/view.rs src/ui/driver.rs"
# `tasks::read` is NEW in the pattern, and it is the second deliberate edit this change makes
# to this block. crate::tasks::read is the filesystem edge of the module ui::tasks renders
# from, and it matches NONE of the other alternatives - not `std::fs` (the call site writes
# `crate::tasks::read`), not `read_to_string` (that spelling lives inside src/tasks.rs, not at
# the call site). Planted in src/ui/detail.rs's production code during planning review, it was
# invisible to NOIO-VIEW, READSEAM, and READONLY-UI alike. Group 6 edits exactly that function,
# so that is the one file where reaching for it is plausible. `grep -nE 'tasks::read'` over the
# seven existing PURE files returns nothing on `main`, so adding it is green today.
IO_RE='std::fs|std::io|std::env|std::process|std::net|File::|read_to_string|tasks::read|state::read|state::record|launch::start|Command'

# Guard A — every searched file exists. grep exits 2 on a missing file; without this a
# renamed file makes the check report a clean tree.
for f in $PURE; do
  [ -f "$f" ] || { echo "NOIO-VIEW FAIL: $f missing" >&2; exit 1; }
done

# Guard B — positive control. src/ui/terminal.rs MUST name std::io. If the pattern were
# broken, or the tree gutted, this fails instead of the whole check passing vacuously.
[ -f src/ui/terminal.rs ] || { echo "NOIO-VIEW FAIL: src/ui/terminal.rs missing" >&2; exit 1; }
grep -qE 'std::io' src/ui/terminal.rs \
  || { echo "NOIO-VIEW FAIL: positive control - src/ui/terminal.rs does not name std::io" >&2
       exit 1; }

hits=$(grep -nE "$IO_RE" $PURE || true)
[ -z "$hits" ] || { echo "NOIO-VIEW FAIL: I/O API in a pure view file:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOIO-VIEW OK: 10 pure files carry no I/O API; positive control matched"
