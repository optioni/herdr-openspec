# tasks-tab — tasks

Read `design.md` first: the Boundaries table, the Test Boundaries table, and the verification
matrix are the contract these tasks implement. **No task may invent a collaborator that table
does not name.**

**The outer loop is taken**, at the `ui::` composition tier through `run_loop`.
`ui::app::action_for` → `Dashboard::apply` → `Dashboard::sync_detail` →
`ui::detail::content_lines` → `ui::tasks::lines` → `ui::view::render` →
`Dashboard::normalise_scroll` is a path no unit test crosses, and the read-only claim is a
claim about what the **loop** does to a real directory rather than what a pure function
returns. Group 2 writes that test and group 10 closes it.

**Group 1 comes before the acceptance test, and that is deliberate**, on the terms
`markdown-viewer` and `detail-view` both recorded: the acceptance test needs a new
`ArtifactRef` field, a widened `content_lines` signature, and a fixture helper before it can
compile. Group 1 is **operational**, because a field, a signature, and a test helper are
plumbing rather than behaviour — the *rule* that decides which artifact is marked is group
3's behaviour, and group 1 sets the field `false` everywhere with no rule at all. Group 2's
RED is an assertion failure against a detail region showing raw `- [ ]` source lines, not a
compile failure. A crate whose test target does not build makes
`cargo clippy --all-targets` and `cargo llvm-cov --ignore-run-fail` unrunnable, and would
leave groups 3 to 9 with no gate at all.

**Every floor in this file is measured before it is raised.** Task 0.1 reads the real counts
off `main` and records them; every later `MIN`, `UI_MIN`, `WIDTHS_MIN`, `DETAIL_MIN`,
`TASK_MIN`, `SCAN_MIN`, and `testcount` minimum is *measured + this change's enumerated new
test functions*. **A realized count below a group's target means the missing test is
written, not the floor lowered.** This repository has already shipped a check whose floor was
asserted rather than measured and was therefore unpassable, and one whose target
(`WIDTHS_MIN=72`) was planned above what the change actually landed and had to come down to
the measured 67 — the second is why every target below is written as *measured + enumerated*
with the arithmetic shown.

**When a task says a check script was edited, that task writes the script to disk and runs
it.** `tui-shell` recorded four edits to `DEPS` as prose in a checkbox only; the script on
disk was never changed, and `list-view` nearly halted on a check that did not exist. Every
block reproduced below is extracted to `$CHECKS/<LABEL>.sh` in task 0.1, and every later run
is of the extracted file. Blocks that are **not** reproduced below are extracted
byte-identically from a named archived file, and task 0.2 diffs each extraction against its
source so an extraction that silently produced the wrong bytes fails there rather than
passing a check that is not the one anybody reviewed.

**`DEPS` and `GRAPH-SNAP` change not at all, and that is a claim to prove, not to skip.**
This change adds no dependency — `ratatui::widgets::Gauge` is deliberately not used, see
design.md → Decisions 3 — so both are extracted byte-identically and both must pass
**unchanged**. A passing run of the unchanged scripts is the evidence that no dependency, no
feature, and no proc-macro crept in.

**Three checks cannot run at all until group 4 creates `src/ui/tasks.rs`**, and that is
stated here rather than discovered: the **edited** `NOIO-VIEW` (whose `PURE` set now names
eight files), `TASKSEAM`, and `TASKWIDTHS` all have a `[ -f ... ]` guard that fails on a
missing file — by design, so a renamed or deleted module is a loud failure rather than a
silent pass. Task 0.3 runs them and **records the expected guard failure**; task 4.4 is their
first green run. The *unedited* seven-file `NOIO-VIEW` is not kept around as a stand-in: two
versions of one check on disk is exactly how a run and a record drift apart.

## Test-module convention — read before writing any test

`cargo test` matches a filter against a test's **full path**, and a filter matching nothing
exits 0. Every filtered gate below therefore addresses tests through an explicit module path,
and is judged on a counted minimum rather than on the run's exit status.

Every target below is **measured at `main` plus the number of NEW test functions the group
enumerates**, and the two are written out so the arithmetic can be checked rather than
trusted. A scenario that *modifies* an existing test adds nothing to the count, and seven of
this change's scenarios do exactly that — a floor derived from "scenarios in the group"
rather than "new test functions in the group" would be unreachable.

| Group | File | Module | Filter | Measured | New | Target |
|---|---|---|---|---|---|---|
| 2, 10 | `src/ui/mod.rs` | `mod tests` → `mod detail` | `ui::tests::detail::` | 2 | 1 | 3 |
| 3 | `src/changes.rs` | `mod tests` | `changes::tests::` | 166 | 14 | 180 |
| 4 | `src/ui/tasks.rs` | `mod tests` | `ui::tasks::tests::` | 0 | 7 | 7 |
| 5 | `src/ui/tasks.rs` | `mod tests` | `ui::tasks::tests::` | 7 | 7 | 14 |
| 6 | `src/ui/detail.rs` | `mod tests` | `ui::detail::tests::` | 19 | 4 | 23 |
| 7 | `src/ui/view.rs` | `mod tests` | `ui::view::tests::` | 67 | 9 | 76 |
| 8 | `src/ui/app.rs` | `mod tests` | `ui::app::tests::` | 46 | 1 | 47 |
| 8 | `src/ui/driver.rs` | `mod tests` | `ui::driver::tests::` | 12 | 2 | 14 |
| 9 | `src/ui/app.rs` | `mod tests` | `ui::app::tests::` | 47 | 1 | 48 |

**`ui::tests::detail::` already holds two tests on `main`** — `markdown-viewer`'s
`a_markdown_document_renders_and_scrolls_through_the_loop` and `detail-view`'s own — so a
floor of 2 would be satisfied before this change writes a line. Group 2 adds a **third** test
function to that module.

**`src/changes.rs` holds 171 `#[test]` attributes but the `changes::tests::` filter matches
166**: five tests live in a different submodule inside the same file. Group 3's fourteen new
tests all go in `changes::tests::` submodules, which is what makes the 166 → 180 arithmetic
hold; a test placed elsewhere in that file would leave the floor unreachable.

The **library** total is `625` measured plus `1 + 14 + 7 + 7 + 4 + 9 + 1 + 2 + 1 = 46`, i.e.
**671**; the `ui::` subtotal is `226` measured plus `1 + 7 + 7 + 4 + 9 + 1 + 2 + 1 = 32`, i.e.
**258**. Both are asserted in group 13 and nowhere else.

**`make check` is not runnable unqualified between groups 2 and 10.** Group 2's acceptance
test is deliberately RED for that whole span, and `cargo llvm-cov` hard-fails on any test
failure and produces no report at all, `--no-fail-fast` included. At every intermediate
boundary, "the gate" therefore means these four commands, run separately:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features            # must fail on EXACTLY the one known acceptance test,
                                     # with the identical assertion message each time
cargo llvm-cov --ignore-run-fail --fail-under-lines 80
```

A **second** failing test at any of those boundaries is a real regression, not the known one.
The literal, unqualified `make check` is run from task 10.3 onward.

---

## Command-level checks, written out once

Referenced by label from the tasks below. They live here rather than in a table cell because
an unescaped `|` cannot appear in a Markdown table and an escaped `\|` inside an ERE matches a
literal pipe, so the check would pass against the very code it exists to catch. Every one is
judged on **output emptiness, a counted minimum, or a specific error code**, never on a bare
pipeline exit status: `grep` exits 1 for no-match and 2 for a bad file, and `!` turns both
into a pass.

Extract each block to `$CHECKS/<LABEL>.sh` in task 0.1 and run the extracted, byte-identical
file thereafter, so the run and the record cannot drift.

```sh
# NOIO-VIEW — the PURE files of the render seam name no I/O API at all.
# Carried forward from detail-view with ONE deliberate edit, recorded in design.md ->
# Boundaries: PURE gains src/ui/tasks.rs, so the set is EIGHT files rather than seven.
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
PURE="src/ui/app.rs src/ui/detail.rs src/ui/layout.rs src/ui/list.rs src/ui/markdown.rs src/ui/tasks.rs src/ui/view.rs src/ui/driver.rs"
# `tasks::read` is NEW in the pattern, and it is the second deliberate edit this change makes
# to this block. crate::tasks::read is the filesystem edge of the module ui::tasks renders
# from, and it matches NONE of the other alternatives - not `std::fs` (the call site writes
# `crate::tasks::read`), not `read_to_string` (that spelling lives inside src/tasks.rs, not at
# the call site). Planted in src/ui/detail.rs's production code during planning review, it was
# invisible to NOIO-VIEW, READSEAM, and READONLY-UI alike. Group 6 edits exactly that function,
# so that is the one file where reaching for it is plausible. `grep -nE 'tasks::read'` over the
# seven existing PURE files returns nothing on `main`, so adding it is green today.
IO_RE='std::fs|std::io|std::env|std::process|std::net|File::|read_to_string|tasks::read|Command'

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
echo "NOIO-VIEW OK: 8 pure files carry no I/O API; positive control matched"
```

```sh
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
```

```sh
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
```

```sh
# READONLY-UI — NEW in tasks-tab. No PRODUCTION code under src/ui/ names a filesystem-write
# API. The dashboard reads openspec/ and never writes there: an agent may be editing tasks.md
# in another pane. NOIO-VIEW forbids every I/O API in the eight PURE files; this is the
# complementary half, and it is the ONLY check that reaches src/ui/mod.rs — the one file
# under src/ui/ permitted to touch the filesystem at all (ui::read_artifact, ui::load,
# ui::run). This change is the first to draw something that looks clickable, which is why the
# read-only claim stops being a formality here.
#
# Two deliberate design choices, each forced by a shape that is REALLY in this tree and each
# verified against it at planning time rather than assumed:
#  1. `#[cfg(test)]` modules are STRIPPED before the search. src/ui/mod.rs:486 legitimately
#     calls std::fs::create_dir_all — inside ui::tests::load, which builds a ScratchDir
#     repository to drive ui::load against. A whole-file sweep would be RED on the unmodified
#     tree, and the only fixes would be to delete a real test or to exempt a file, which is
#     how a confinement check rots into a rubber stamp. Every src/ui/*.rs file holds at most
#     ONE line-anchored `#[cfg(test)]` and its test module runs to EOF, verified at planning
#     time, so "cut at the first one" is exact rather than approximate.
#  2. Every pattern is PATH-QUALIFIED (`fs::rename`, not `rename`). A bare `rename` matches
#     src/ui/app.rs:35's doc comment "`Next` and `Prev` are renamed from ...", correct prose,
#     which would make the check RED on the unmodified tree for the second time.
#
# Unlike the three checks above, this one runs GREEN on unmodified `main` — verified at
# planning time, together with a planted `std::fs::write` in ui::read_artifact that it caught
# — so task 0.3 runs it for real rather than recording a guard failure.
UIDIR="${UIDIR:-src/ui}"
CONTROL="${CONTROL:-src/state.rs}"
UI_MIN="${UI_MIN:-10}"
WRITE_RE='fs::write|File::create|OpenOptions|fs::remove_|fs::create_dir|fs::rename|fs::copy|set_permissions|fs::hard_link|fs::soft_link'
fail() { echo "READONLY-UI FAIL: $1" >&2; exit 1; }

[ -d "$UIDIR" ] || fail "no such directory: $UIDIR"
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
for f in $(find "$UIDIR" -name '*.rs' | sort); do
  h=$(prod "$f" | grep -nE "$WRITE_RE" | sed "s|^|$f:|" || true)
  hits="$hits$h"
done
[ -z "$hits" ] || { echo "READONLY-UI FAIL: a write API in production code under $UIDIR:" >&2
                    echo "$hits" >&2; exit 1; }
echo "READONLY-UI OK: $n files under $UIDIR, no write API in production code; both controls matched"
```

```sh
# OPENSPEC-UNTOUCHED — no code path writes inside openspec/. Diffed against the BASE SHA
# captured in task 0.1, never against the index: this project commits per task group, so
# `git diff --exit-code` between working tree and index passes over the very change it
# exists to catch.
#
# `git diff` alone is NOT enough: it lists tracked paths only, and a file the plugin WRITES
# at runtime is untracked, so the one violation this check exists to catch would be
# invisible to it. The `ls-files --others` sweeps are the half that catch it, the second of
# them covering IGNORED untracked files, since a runtime write is exactly the kind of file
# someone adds a gitignore line for.
#
# This change draws a checkbox, which is what makes a stray write plausible for the first
# time: a reader who "just wants to toggle it" is one edit away, and group 9's scripted
# every-printable-key run over a real ScratchDir repository is the runtime half of the same
# claim. This grep is the source half.
#
# Exactly ONE exclusion: this change's own artifact directory, a hand-edited planning
# document rather than a code-path write. Excluded BY NAME, never by a broad prefix, so a
# stray file anywhere else under openspec/ still fails. Carried forward from detail-view
# with that one path updated.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --ignored --exclude-standard -- ':(top)openspec/'; } \
         | grep -v '^openspec/changes/tasks-tab/' | sort -u || true)
[ -z "$stray" ] || { echo "FAIL: wrote inside openspec/ outside this change:" >&2
                     echo "$stray" >&2; exit 1; }
echo "OPENSPEC-UNTOUCHED OK"
```

**Extracted byte-identically rather than retyped** — eighteen blocks, none of which changes.
From `openspec/changes/archive/2026-09-05-detail-view/tasks.md`: `NOSPAWN-GREP.sh`,
`READSEAM.sh`, `NOCLI-SHELL.sh`, `NODEFAULT-UI.sh` (its **edited**, brace-matching form —
that is the version on record and the one that sees a multi-line elision), `NOLIT-CHANGE.sh`,
`MDSEAM.sh`, `NOTABSEAM.sh`, `WIDTHS.sh`, and `DETAILWIDTHS.sh`. From
`openspec/changes/archive/2026-09-05-markdown-viewer/tasks.md`: `NORAW-GREP.sh`,
`LISTWIDTHS.sh`, `MDWIDTHS.sh`, `NOWAIVER.sh`, `TESTCOUNT.sh`, `DEPS.sh`, and
`GRAPH-SNAP.sh`. From `openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md`:
`GATE-MECH1.py` and `NOJSON-SEAM.sh`. Task 0.2 diffs every one of those extractions against
its source block, so an extraction that produced the wrong bytes fails there rather than
passing a check nobody reviewed.

Together: **five** blocks reproduced above (`NOIO-VIEW`, `TASKSEAM`, `TASKWIDTHS`,
`READONLY-UI`, `OPENSPEC-UNTOUCHED`) and **eighteen** extracted from the archives, for
**twenty-three** files in `$CHECKS`.

---

## 0. Baseline, checks, and the scratchpad
<!-- kind: operational -->

- [x] 0.1 CHECK: Record the starting state before any edit, by **measuring**, never by
      copying a number from this file.
      - `git rev-parse HEAD` → `export BASE=<sha>`. Every `OPENSPEC-UNTOUCHED` run uses it.
        A diff against the index would pass over this change's own per-group commits.
        **Measured at planning time: `85a5631dfcfb0c02c4091ccabb3232c194ab61f2`.** Prefer
        `BASE=85a5631 sh $CHECKS/OPENSPEC-UNTOUCHED.sh` at each invocation over relying on an
        exported variable: the check fails closed in a fresh shell, but the natural recovery —
        re-exporting from the *current* `HEAD` — silently defeats it, since every commit this
        change makes would then be inside the baseline.
      - `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
        passed.*/\1/p' | awk '{t+=$1} END{print t+0}'` → the **lib** baseline.
        **Measured at planning time: 625**, inside a 641-test whole suite (625 lib + 11
        `tests/ci_workflow.rs` + 5 `tests/cli.rs`).
      - `find src -name '*.rs' ! -path 'src/cli.rs' | wc -l` → `NOSPAWN-GREP`'s starting file
        count. **Measured at planning time: 18**; the floor becomes 19.
      - `find src -name '*.rs' ! -path 'src/changes.rs' | wc -l` → `NOLIT-CHANGE`'s starting
        file count. **Measured at planning time: 18**; the floor becomes 19.
      - `find src -name '*.rs' ! -path 'src/ui/markdown.rs' | wc -l` → `MDSEAM`'s starting
        file count. **Measured at planning time: 18**; the floor becomes 19. All three are 18
        today and 19 after, because `src/ui/tasks.rs` is excluded by none of them.
      - `find src/ui -name '*.rs' | wc -l` → `NOCLI-SHELL`'s and `READONLY-UI`'s `UI_MIN`
        baseline. **Measured at planning time: 10**; both floors become 11.
      - `find src/ui -name '*.rs' ! -path 'src/ui/mod.rs' | wc -l` → `READSEAM`'s `UI_MIN`
        baseline. **Measured at planning time: 9**; the floor becomes 10.
      - `find src tests -name '*.rs' ! -path 'src/ui/terminal.rs' | wc -l` → `NORAW-GREP`'s
        guard. **Measured at planning time: 20** (its floor is 16 and does not move).
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/view.rs` → `WIDTHS`' baseline.
        **Measured at planning time: 67**; the floor becomes 76.
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/detail.rs` → `DETAILWIDTHS`' baseline.
        **Measured at planning time: 19**; the floor becomes 23.
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/list.rs` → `LISTWIDTHS`' baseline.
        **Measured at planning time: 17** (its floor is 17, exactly met today and unmoved —
        this change adds no list test).
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/markdown.rs` → `MDWIDTHS`' baseline.
        **Measured at planning time: 24** (its floor is 23 and does not move; this change
        adds no markdown test).
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/app.rs` → **46**;
        `src/ui/driver.rs` → **12**; `src/ui/mod.rs` → **17**; `src/changes.rs` → **171**;
        `src/ui/layout.rs` → **15**. `src/ui/tasks.rs` does not exist yet.
      - The `src/ui/mod.rs` submodule counts, which the whole-file 17 hides and which three
        separate floors depend on. Measure by running each filter:
        `cargo test --all-features --lib 'ui::tests::load::'` → **7**;
        `'ui::tests::detail::'` → **2** (already satisfying a floor of 2 — group 2's floor is
        therefore 3); `'ui::tests::start::'` → **6**;
        `'ui::tests::read_artifact::'` → **2**.
      - `cargo test --all-features --lib 'changes::tests::'` → **166**, against 171 `#[test]`
        attributes in that file: five tests live in another submodule. The floor becomes 180
        and group 3's fourteen new tests must all land in `changes::tests::`.
      - `cargo test --all-features --lib 'ui::' 2>&1 | sed -n 's/^test result: ok\.
        \([0-9]*\) passed.*/\1/p' | awk '{t+=$1} END{print t+0}'` → the `ui::` subtotal.
        **Measured at planning time: 226.**
      - `NODEFAULT-UI`'s half B, run with `SCAN_MIN=1`, prints the number of literal/pattern
        spans it scanned. **Measured at planning time: 97 spans, 0 elisions**; the invocation
        below raises `SCAN_MIN` from detail-view's 60 to **80**, comfortably below 97 with
        room for task 7.4's span-reducing refactor, and far above zero.
      - `grep -rn 'ArtifactRef[[:space:]]*{' src/ | wc -l` → the `ArtifactRef` sweep group 1
        must complete. **Measured at planning time: 36 grep hits, of which only 32 are real
        constructions**, all in `src/changes.rs`. The other four are the struct definition
        (`src/changes.rs:29`), a doc comment (`src/changes.rs:194`), a `-> ArtifactRef {`
        signature (`src/changes.rs:3567`), and a doc comment in `src/ui/detail.rs` saying the
        module builds no such literal of its own. **The grep is orientation only. The gate for
        task 1.2 is the compiler (`E0063`)**, which cannot be fooled by a comment — an earlier
        draft of this plan quoted the grep's 35 as the construction count and was wrong by
        three.
      - `grep -rn 'content_lines(' src/ | wc -l` → **9 hits, 8 of them calls**: the
        definition in `src/ui/detail.rs:170` is not a call; the calls are
        `src/ui/detail.rs:524, 534, 552, 569, 591, 592` (six, in that file's tests),
        `src/ui/view.rs:109`, and `src/ui/app.rs:263`. All eight move in group 1.
      - `cargo llvm-cov --summary-only` → TOTAL **line** coverage. The TOTAL row leads with
        the **region** count; read the line column, further right. **Measured at planning
        time: 97.81% over 14,178 lines** (regions 97.39%, functions 96.62% — do not quote
        either of those as the gated figure).
      - `export CHECKS=<scratchpad>/tasks-tab-checks` and
        `export WORK=<scratchpad>/tasks-tab-work`; `mkdir -p "$CHECKS" "$WORK"`.
        Extract every fenced block above to `$CHECKS/<LABEL>.sh` byte-identically — five of
        them — and extract the eighteen named archived blocks to their own files.

- [x] 0.2 CHECK: Prove every **extracted** block is byte-identical to its source. For each of
      the eighteen, re-extract the source block into `$WORK/<LABEL>.src` by the same means and
      `diff -u "$WORK/<LABEL>.src" "$CHECKS/<LABEL>.sh"` — an empty diff for all eighteen, or
      stop. An extraction that silently produced the wrong bytes would otherwise pass a check
      that is not the one anybody reviewed. Record the eighteen diff results.

- [x] 0.3 CHECK: Run every check against **unmodified `main`** and record each result
      verbatim, so the change starts from a known state rather than an assumed one.
      - Expected **green** at their current floors, each printing its OK line:
        `MIN=18 sh $CHECKS/NOSPAWN-GREP.sh`; `UI_MIN=9 sh $CHECKS/READSEAM.sh`;
        `UI_MIN=10 sh $CHECKS/NOCLI-SHELL.sh`;
        `TYPES="Dashboard Filter Detail" SCAN_MIN=80 sh $CHECKS/NODEFAULT-UI.sh`;
        `MIN=18 sh $CHECKS/NOLIT-CHANGE.sh`; `MIN=18 sh $CHECKS/MDSEAM.sh`;
        `sh $CHECKS/NOTABSEAM.sh`; `sh $CHECKS/NORAW-GREP.sh`;
        `WIDTHS_MIN=67 sh $CHECKS/WIDTHS.sh`; `LIST_MIN=17 sh $CHECKS/LISTWIDTHS.sh`;
        `MD_MIN=23 sh $CHECKS/MDWIDTHS.sh`; `DETAIL_MIN=19 sh $CHECKS/DETAILWIDTHS.sh`;
        `sh $CHECKS/NOWAIVER.sh`; `sh $CHECKS/DEPS.sh`; `sh $CHECKS/GRAPH-SNAP.sh`;
        `sh $CHECKS/NOJSON-SEAM.sh`; `python3 $CHECKS/GATE-MECH1.py src`;
        `UI_MIN=10 sh $CHECKS/READONLY-UI.sh`; `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`.
      - Expected **red, with the exact message `<file> missing`**, because group 4 has not
        created `src/ui/tasks.rs` yet: `sh $CHECKS/NOIO-VIEW.sh` (edited),
        `sh $CHECKS/TASKSEAM.sh`, `TASK_MIN=7 sh $CHECKS/TASKWIDTHS.sh`. Record the three
        messages. **A different failure message from any of the three is a defect in the
        block, not the expected guard**, and must be fixed here rather than in group 4.
      - Also confirm the **edited** `NOIO-VIEW`'s new `tasks::read` alternative is green on
        the seven files that already exist: `grep -nE 'tasks::read' src/ui/app.rs
        src/ui/detail.rs src/ui/layout.rs src/ui/list.rs src/ui/markdown.rs src/ui/view.rs
        src/ui/driver.rs` must print nothing. **Verified at planning time: no hits**, so the
        added pattern costs nothing on `main` and only guards what arrives later.
      - Red-when: any check in the first list fails on unmodified `main`. That is a defect in
        the extraction or the invocation, and it is fixed here — `NOTABSEAM` shipped red on an
        unmodified tree once already.

- [x] 0.4 CHECK: Prove each of the three NEW checks and the one EDITED check can actually see
      what it guards, by planting a violation, running the check, and reverting. Each plant is
      a working-tree edit reverted immediately; `git status --porcelain src/` must be empty
      after each revert.
      - `READONLY-UI`: add `let _ = std::fs::write(path, "x");` inside `ui::read_artifact` in
        `src/ui/mod.rs`. Expect FAIL naming `src/ui/mod.rs` and the line.
        **Verified at planning time: green on `main`, red on this plant, green again after
        the revert.**
      - `READONLY-UI`, second plant: add `std::fs::remove_file(p)` inside a `#[cfg(test)]`
        module of `src/ui/view.rs`. Expect **OK** — the stripper is supposed to ignore it —
        and record that this is the deliberate exemption, not a miss.
      - `NOIO-VIEW` (after group 4 creates the file, deferred to task 4.4): add
        `use std::fs;` to `src/ui/tasks.rs`. Expect FAIL naming that file.
      - `TASKSEAM`, ratatui leg (deferred to 4.4): add `// the view maps this to a Style` as a
        comment in `src/ui/tasks.rs`. Expect FAIL — the comment-inclusive sweep is the point,
        and this plant is what proves the Red-when in task 4.2 is real.
      - `TASKSEAM`, filesystem leg (deferred to 4.4): add a `crate::tasks::read(p)` call.
        Expect FAIL on that leg. Plant the **same** call in `src/ui/detail.rs`'s production
        code and expect the edited `NOIO-VIEW` to catch it there — that is the pattern this
        change added, and group 6 edits exactly that file.
      - `TASKSEAM`, parse leg (deferred to **5.5**, not 4.4): the leg is armed only when
        `src/ui/tasks.rs` declares `pub fn lines`, which arrives in group 5. Two plants, both
        at 5.5: replace the `tasks::parse(...)` call with a hand-rolled `source.lines()` loop
        while a doc comment still *names* `crate::tasks::parse` — expect FAIL, because the leg
        strips comments; and restore the call — expect OK. **Verified at planning time: the
        comment-only form passed a naive whole-file grep, which is why the leg strips comments
        and matches a call shape.**
      - `NOLIT-CHANGE` (deferred to 4.4): add a `Change { … }` literal to `src/ui/tasks.rs`.
        Expect FAIL naming that file — the new module is inside the searched set.
      - `TASKWIDTHS` (deferred to 4.4): change one test's `58` to `59`. Expect FAIL naming
        that test function.

- [x] 0.5 VERIFY: `git status --porcelain` shows only `openspec/changes/tasks-tab/`; the
      four gate commands from the preamble all pass on unmodified `main`; and
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh` prints OK. Commit the artifacts.

---

## 1. Plumbing the acceptance test needs
<!-- kind: operational -->

A field, a signature, and a test helper. No rule and no rendering: group 1 sets
`tracks_tasks` to `false` at every site and `content_lines` ignores its new argument, so
nothing observable changes and there is no honest RED to manufacture. The evidence is that
the crate compiles, every existing test still passes, and `GATE-MECH1` still holds.

- [x] 1.1 CHECK: `python3 $CHECKS/GATE-MECH1.py src` and
      `cargo test --all-features --lib 'changes::tests::'` both green before any edit, so the
      "no regression" claim below has a baseline.

- [x] 1.2 CHANGE: Add `pub tracks_tasks: bool` to `changes::ArtifactRef`, after `paths`.
      Set it `false` at every one of the **32** construction sites in `src/changes.rs` —
      including `change_artifacts`, `cli_artifacts`, and `fixture::with_artifacts`. Do not
      count them with a grep: `grep -c 'ArtifactRef {'` reports 36 and includes the struct
      definition, two doc comments, and a return-type signature. Do **not** reach for
      `..Default::default()` or a `..base` rest: `GATE-MECH1` forbids both for this type, and
      the compile error (`E0063`) at each site is the mechanism that keeps the two producers
      in agreement. The doc comment on the field states what it means, not how it is computed.
      Red-when: the crate compiles without every site naming the field — that would mean a
      `Default` or a `..` slipped in.

- [x] 1.3 CHANGE: Add `pub(crate) fn track_tasks_at(change: Change, index: usize) -> Change`
      to `changes::fixture` (`#[cfg(test)]`), returning `change` with `artifacts[index]`'s
      `tracks_tasks` set `true` and every other left `false`; an `index` past the end marks
      nothing and does not panic. It lives in `src/changes.rs` beside `with_artifacts` for the
      reason that function does: `NOLIT-CHANGE` forbids a `Change` or `ArtifactRef` literal
      outside that file, and a view test that needed one would otherwise force the check to be
      weakened.

- [x] 1.4 CHANGE: Widen `ui::detail::content_lines` to
      `(detail: &Detail, change: Option<&crate::changes::Change>, width: u16)`, **ignoring**
      `change` for now (`let _ = change;` is not needed — an unused parameter named `_change`
      would hide the group-6 edit; name it `change` and add `#[allow(unused_variables)]` only
      if clippy demands it, removing the attribute in group 6).
      Update all **eight** call sites: `src/ui/view.rs:109`'s `render_detail_content` and
      `src/ui/app.rs:263`'s `normalise_scroll` pass
      `dashboard.selected_change()` / `self.selected_change()`; the six calls in
      `src/ui/detail.rs`'s tests (lines 524, 534, 552, 569, 591, 592) pass `None`.
      Red-when: `normalise_scroll` fails to borrow-check. The fix is to end the immutable
      borrow by computing `total` in its own statement before assigning `self.detail.scroll`,
      **not** to clone the change or to reorder the function's steps.

- [x] 1.5 CHECK: Contract gate — re-read `design.md` → Contracts and confirm the three
      signatures on disk match it exactly, and that `Change`'s **seven** fields are unchanged,
      so `cli-changes`' "every one of `Change`'s seven fields comes from CLI data" requirement
      is untouched.

- [x] 1.6 VERIFY: `cargo fmt --all -- --check`;
      `cargo clippy --all-targets --all-features -- -D warnings`;
      `cargo test --all-features` — **fully green**, this being the last boundary before the
      acceptance test goes red; `python3 $CHECKS/GATE-MECH1.py src`;
      `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'changes::tests::' 166` and
      `testcount --lib 'ui::detail::tests::' 19` — unchanged, because this group adds no test.
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 2. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [x] 2.1 Set up the harness and the replaced collaborators named in design.md → Test
      Boundaries: a `crate::testutil::ScratchDir` repository holding
      `openspec/config.yaml` (`schema: tdd`), `openspec/schemas/tdd/schema.yaml` declaring the
      five artifacts with `apply.tracks: tasks.md`, and one change directory holding a
      `tasks.md` with two checked and three unchecked items under two headings. Drive it
      through `ui::load`, the **real** `ui::read_artifact`, a scripted `EventSource`, and a
      `TestBackend`. Nothing else is real: no `openspec` binary, no `herdr` socket, no
      terminal.

- [x] 2.2 RED: Write `ui::tests::detail::tasks_tab_is_read_only` — the failing end-to-end test
      for **tasks-checklist → "Every printable key leaves the change tree byte-identical"**,
      carrying the headline rendering assertion with it. At 120x20 and again at 60x20: take a
      `testutil::snapshot` of the scratch tree; run `run_loop` with a script that selects the
      tracked-tasks tab, then presses every ASCII printable character from `!` to `~`, then
      `Enter`, `Esc`, `Backspace`, `Tab`, the four arrows, and finally `Ctrl-C`; assert the
      final buffer's content area holds the progress bar ending `[2/5] 40%` on its first row,
      a blank row, and `[x]`/`[ ]` glyph rows below; assert the post-run snapshot is
      byte-identical to the pre-run one; and assert re-reading `tasks.md` still gives
      `Progress { completed: 2, total: 5 }`.

- [x] 2.3 Confirm it fails **because the behaviour is missing**, not because the harness is
      misconfigured: the failure must be the rendering assertion — the buffer holds
      `- [x] ...` source lines rather than a bar and glyph rows — and the snapshot and
      progress assertions must already **pass**, since nothing writes today. Record the exact
      assertion message; every intermediate boundary from here to group 10 must reproduce it
      verbatim, and a *different* message means the harness broke rather than the behaviour
      arriving.

- [x] 2.4 REFACTOR: None is possible while the test is red; state that explicitly here rather
      than leaving the lifecycle step unaccounted for. The harness cleanup happens at task
      10.2, once the test is green and a refactor can be shown not to change its result.

- [x] 2.5 VERIFY: the four gate commands from the preamble. `cargo test --all-features` fails
      on exactly one test, `ui::tests::detail::tasks_tab_is_read_only`.
      `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` passes.
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 3. The tracked-tasks flag, in both producers
<!-- kind: behavior -->

- [x] 3.1 RED: Write fourteen failing tests in `changes::tests::`, named from their scenarios:
      `tracks_tasks_tdd`, `tracks_tasks_prefers_tracks`, `tracks_tasks_id_fallback`,
      `tracks_tasks_none_marked`, `tracks_tasks_duplicate_first_only`,
      `tracks_tasks_empty_list` (change-artifacts, six);
      `cli_tracks_tasks_tdd`, `both_producers_mark_the_same_position`,
      `cli_only_schema_marks`, `cli_duplicate_first_only` (cli-changes, four);
      `join_takes_cli_flag`, `join_cli_flag_moves_the_tab`, `join_rejected_keeps_file_flag`,
      `join_never_two_marked` (change-merge, four).
      Red-when: `both_producers_mark_the_same_position` passes before 3.2 lands — it would,
      trivially, with every flag `false` on both sides, so it must assert a `true` at a named
      position and not merely that the two vectors are equal.

- [x] 3.2 GREEN: Implement `pub(crate) fn tasks_index(schema: &schema::Schema) ->
      Option<usize>` in `src/changes.rs`: the **first** index of `schema.artifacts` whose
      entry equals `schema.tasks`, `None` when `schema.tasks` is `None`. Call it from
      `change_artifacts` and from `cli_artifacts`, and from nowhere else — one rule, two
      producers.
      Red-when: the position is found by comparing ids rather than whole `Artifact` values;
      `tracks_tasks_prefers_tracks` is the test that catches it.

- [x] 3.3 CHECK: `join_artifacts` needs **no code change** — it returns whole `ArtifactRef`
      values and the flag rides with them. Confirm that by reading it, and confirm the four
      `join_*` tests pass without touching the function. If a change turns out to be needed,
      that is a divergence from `design.md` → Decisions 1 and must be recorded there before
      it is made.

- [x] 3.4 REFACTOR: If `change_artifacts` and `cli_artifacts` ended up with two copies of the
      "mark at `tasks_index`" loop, collapse them onto one helper; otherwise state that no
      refactor was needed.

- [x] 3.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'changes::tests::' 180`;
      `python3 $CHECKS/GATE-MECH1.py src`; `MIN=18 sh $CHECKS/NOLIT-CHANGE.sh`;
      `sh $CHECKS/NOJSON-SEAM.sh`; the four gate commands, still failing on exactly the one
      known acceptance test with the recorded message.
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 4. The progress bar
<!-- kind: behavior -->

- [x] 4.1 RED: Create `src/ui/tasks.rs` holding only its module doc comment and
      `pub fn progress_bar(progress: &crate::tasks::Progress, width: u16) -> String`
      returning `String::new()`, declare `pub mod tasks;` in `src/ui/mod.rs`, and write seven
      failing tests in `ui::tasks::tests::`, named from their scenarios: `bar_full_grammar`,
      `percent_truncates`, `gauge_full_only_when_complete`, `gauge_property_sweep`,
      `bar_drops_fields_whole`, `bar_never_partial`, `no_tasks_bar_is_the_count_cell`.
      Every one names both `58` and `78` unsuffixed — `TASKWIDTHS` has no exemption list, and
      the two sweep tests name the pair explicitly among their swept widths.
      Red-when: a test asserts only that no panic occurred. A stub returning `String::new()`
      never panics, so such a test is green by construction and pins nothing.

- [x] 4.2 GREEN: Implement `progress_bar` per `specs/tasks-progress-bar/spec.md`: the count
      cell from `ui::list::progress_cell` (**not** a second `format!` of the same pair), the
      truncating percentage with a saturating multiply, the `█`/`░` gauge whose fill is
      `g * completed / total` with a saturating multiply, and the drop-whole order — percent,
      then gauge, then nothing. Width arithmetic in `i64`, as `ui::detail::header_row` already
      does, so no `u16` subtraction can underflow.
      Red-when: the module's doc comment names a `ratatui` type (`Style`, `Modifier`, `Span`,
      `Rect`, `Frame`, `Buffer`) or the string `ratatui`, or names `from_cli`. `TASKSEAM`
      and `NOCLI-SHELL` are whole-file greps with comments deliberately included — say "the
      view" and "the CLI producer". This has been red on an unmodified tree in this repository
      before.

- [x] 4.3 REFACTOR: Extract the field-dropping ladder if it reads as three near-copies;
      otherwise state that no refactor was needed.

- [x] 4.4 CHECK: The three checks that could not run before this file existed, now green for
      the first time, **and** their deferred planted-violation controls from task 0.4:
      `sh $CHECKS/NOIO-VIEW.sh` (must print `8 pure files`); `sh $CHECKS/TASKSEAM.sh` — which
      at this group prints `parse leg not armed` **and then** its OK line, because
      `pub fn lines` arrives in group 5 and the leg is conditional on it; and
      `TASK_MIN=7 sh $CHECKS/TASKWIDTHS.sh`.
      Then plant and revert each of this group's deferred violations — a `use std::fs;` in
      `src/ui/tasks.rs` (NOIO-VIEW), a `// the view maps this to a Style` comment (TASKSEAM),
      a `crate::tasks::read(p)` call in `src/ui/tasks.rs` (TASKSEAM) and the same call in
      `src/ui/detail.rs`'s production code (NOIO-VIEW's new pattern), a `Change { … }` literal
      in `src/ui/tasks.rs` (NOLIT-CHANGE), and a `58` changed to `59` (TASKWIDTHS) —
      confirming each check reports the expected failure and `git status --porcelain src/` is
      empty after every revert. The parse leg's two plants are group 5's, at task 5.5.
      Also re-run at their raised floors: `MIN=19 sh $CHECKS/NOSPAWN-GREP.sh`;
      `UI_MIN=10 sh $CHECKS/READSEAM.sh`; `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`;
      `MIN=19 sh $CHECKS/NOLIT-CHANGE.sh`; `MIN=19 sh $CHECKS/MDSEAM.sh`;
      `UI_MIN=11 sh $CHECKS/READONLY-UI.sh`.

- [x] 4.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::tasks::tests::' 7`; the four
      gate commands, still failing on exactly the one known acceptance test.
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 5. The checklist grammar
<!-- kind: behavior -->

- [x] 5.1 RED: Write seven failing tests in `ui::tasks::tests::`, named from their scenarios:
      `groups_headings_items`, `nested_indent`, `long_item_hanging_indent`,
      `unbreakable_word_hard_split`, `indent_dropped_whole`, `empty_group_keeps_heading`,
      `headingless_leading_group`. Each names both `58` and `78` unsuffixed.
      Red-when: `long_item_hanging_indent` asserts only "no line exceeds the width". An empty
      vector satisfies that forever; it must also assert the strict line-count inequality
      between 58 and 78 and the four-space continuation prefix.

- [x] 5.2 GREEN: Implement `pub fn lines(source: &str, progress: &crate::tasks::Progress,
      width: u16) -> Vec<crate::ui::markdown::Line>` per `specs/tasks-checklist/spec.md` →
      "The checklist's line grammar": the bar and its blank line (both omitted when the bar is
      empty), then `crate::tasks::parse(source)`'s groups — heading line with
      `Face { heading: Some(level), .. }` spelled out field by field, item lines with the
      indent, the three-character glyph, a space, and the word-wrapped text at a hanging
      indent, a blank line between groups but not after the last — and `No tasks yet` when
      there are no groups. `width == 0` returns an empty vector.
      Red-when: `crate::tasks::read` is called instead of `crate::tasks::parse`. `TASKSEAM`'s
      second leg catches it; `NOIO-VIEW`'s pattern does not, which is why that leg exists.

- [x] 5.3 GREEN: Implement the private plain-text wrapper `design.md` → Decisions 8 argues
      for: word-wrap at spaces, hard-split a word longer than the column, and never lose a
      tail. It stays private to `src/ui/tasks.rs`; do **not** make `ui::markdown`'s `Run` or
      its folder public to reuse `wrap_prose` — that widens `MDSEAM`'s confined module for a
      caller with no faces.

- [x] 5.4 REFACTOR: Clean up while the fourteen tests stay green, or state that none was
      needed.

- [x] 5.5 CHECK: `sh $CHECKS/TASKSEAM.sh` — its parse leg is **armed for the first time**
      here, because `pub fn lines` now exists, so the run must print
      `TASKSEAM OK (parse leg)` rather than `parse leg not armed`. Then its two deferred
      plants from task 0.4: replace the `tasks::parse(...)` call with a hand-rolled
      `source.lines()` loop while leaving a doc comment that names `crate::tasks::parse`
      (expect FAIL — the leg strips comments and matches a call shape), and restore it (expect
      OK). `git status --porcelain src/` empty after the revert.

- [x] 5.6 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::tasks::tests::' 14`;
      `TASK_MIN=14 sh $CHECKS/TASKWIDTHS.sh`; `sh $CHECKS/NOIO-VIEW.sh`; the four gate
      commands, still failing on exactly the one known acceptance test.
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 6. The dispatch in `content_lines`
<!-- kind: behavior -->

- [x] 6.1 RED: Write four failing tests in `ui::detail::tests::`, each naming both `58` and
      `78`: `marked_tab_returns_the_checklist_body`, `unmarked_tab_returns_the_markdown_body`,
      `tab_past_the_end`, `content_lines_total`. They reach a `Change` through
      `changes::fixture::with_artifacts` and `changes::fixture::track_tasks_at` — never an
      `ArtifactRef {` literal of this module's own, which `NOLIT-CHANGE` forbids.
      Red-when: `unmarked_tab_returns_the_markdown_body` passes before 6.2 lands. It would,
      since everything is markdown today, so it must be paired with the marked case in a way
      that makes the pair discriminating — assert the marked body's first line is the bar.

- [x] 6.2 GREEN: In `ui::detail::content_lines`, read the `change` argument added in group 1:
      when `change.and_then(|c| c.artifacts.get(detail.tab)).is_some_and(|a| a.tracks_tasks)`,
      extend with `crate::ui::tasks::lines(&detail.source, &change.progress, width)`;
      otherwise with `crate::ui::markdown::lines(&detail.source, width)`. The problems lines
      above and the `No content yet` rule below are **unchanged** — both bodies return nothing
      for an empty source, which is what keeps that rule intact. Remove any
      `#[allow(unused_variables)]` group 1 added.
      Red-when: the decision is duplicated into `ui::view` or `Dashboard::normalise_scroll`.
      `specs/artifact-content` requires it in exactly one place, and two copies is how the
      draw and the clamp come to disagree.

- [x] 6.3 CHECK: Contract gate — `content_lines`' behaviour is what `detail-scroll`,
      `responsive-layout`, and `artifact-content` all describe. Re-read those three specs in
      `openspec/specs/` and confirm the only statement this change makes false is the
      signature spelling, which `specs/detail-scroll/spec.md` and
      `specs/artifact-content/spec.md` in this change already correct.

- [x] 6.4 REFACTOR: `content_lines` now has three sequential concerns — problems, body,
      fallback. Extract the body selection into a named private helper if the function no
      longer reads as one thing; otherwise state that no refactor was needed.

- [x] 6.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::detail::tests::' 23`;
      `DETAIL_MIN=23 sh $CHECKS/DETAILWIDTHS.sh`; `sh $CHECKS/NOTABSEAM.sh`;
      `sh $CHECKS/NOIO-VIEW.sh`; the four gate commands, still failing on exactly the one
      known acceptance test. `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 7. The rendered buffer
<!-- kind: behavior -->

- [x] 7.1 RED: Write nine new failing tests in `ui::view::tests::`, each rendering at **both**
      60 and 120 columns and asserting named cells: `tasks_tab_shows_checkboxes`,
      `tasks_tab_chosen_by_flag`, `no_marked_artifact_renders_markdown`,
      `prose_only_reads_no_tasks_yet`, `missing_tasks_artifact_no_content_yet`,
      `tasks_tab_read_failure`, `progress_bar_in_the_buffer`, `no_tasks_bar_in_the_buffer`,
      `marked_tab_renders_checklist_body`.
      Red-when: a test renders and asserts only that the buffer is non-empty.
      `quality-gates` → "View behaviour is verified against a `TestBackend` buffer at both
      widths" rejects that explicitly: it stays green with the behaviour deleted.

- [x] 7.2 CHANGE: Update the **seven** existing view tests this change's scenarios modify,
      without adding to the count: `missing_artifact_no_content_yet` (gains the marked tab-3
      case), `markdown_fills_content_area`, `document_fills_interior`,
      `faces_reach_the_buffer` (each has its artifact explicitly **un**marked, so the test
      states its assumption rather than relying on a default),
      `empty_source_blank_interior` (gains a fourth marked-but-empty dashboard),
      `content_never_overwrites_border` (gains a marked 200-character-task-line case), and
      `degenerate_interior` (every size repeated with the tab marked). Their exact names on
      disk may differ; match by the scenario they carry, not by the name written here.

- [x] 7.3 CHECK: `ui::view::style_for` is **unchanged** — no new `Face` variant and no new
      modifier mapping. Confirm by reading it and by `git diff` over that function. A
      checklist heading reaches the buffer bold through the existing `heading` arm; a new arm
      would mean the grammar invented a face `design.md` → Decisions 11 says it does not.

- [x] 7.4 REFACTOR: Fold the repeated "build a dashboard with a marked tab and this source"
      setup into one test helper if it appears more than three times; otherwise state that no
      refactor was needed.

- [x] 7.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::view::tests::' 76`;
      `WIDTHS_MIN=76 sh $CHECKS/WIDTHS.sh`; `sh $CHECKS/NOIO-VIEW.sh`; the four gate commands,
      still failing on exactly the one known acceptance test.
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 8. The scroll clamp across two bodies
<!-- kind: behavior -->

- [x] 8.1 RED: Write one failing test in `ui::app::tests::` —
      `normalise_scroll_clamps_against_the_drawn_body` — and two in `ui::driver::tests::` —
      `checklist_scroll_is_clamped` and `tab_move_resets_and_reclamps` — from
      `specs/detail-scroll` → "Switching to and from the tracked-tasks tab renormalises the
      scroll". Both driver tests run at 120x20 **and** 60x20.
      Red-when: `checklist_scroll_is_clamped` asserts a clamped value that the markdown body
      would also produce. The scenario requires the two to **differ**, and the test must
      assert that difference explicitly, or it cannot tell which body the clamp used.

- [x] 8.2 GREEN: No production change is expected — group 1 already routed
      `selected_change()` into `normalise_scroll`'s `content_lines` call, and group 6 made
      that call dispatch. If a change **is** needed, it is a divergence from `design.md` →
      Risks and must be recorded there before it is made.

- [x] 8.3 CHECK: `run_loop` gains no step and no new call: `sync_detail` before the draw,
      `normalise_scroll` after it, exactly as `dashboard-loop` specifies. Confirm by
      `git diff` over `src/ui/driver.rs`'s production code showing test-module changes only.

- [x] 8.4 REFACTOR: State that no refactor was needed, or name one — this group is expected
      to add tests and no production code, so a refactor here would be a signal that group 1
      or group 6 left something half-wired.

- [x] 8.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::app::tests::' 47`;
      `testcount --lib 'ui::driver::tests::' 14`;
      `TYPES="Dashboard Filter Detail" SCAN_MIN=80 sh $CHECKS/NODEFAULT-UI.sh`; the four gate
      commands, still failing on exactly the one known acceptance test.
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 9. The read-only proof
<!-- kind: behavior -->

- [x] 9.1 RED: Write one failing test in `ui::app::tests::` —
      `no_action_mutates_changes` — applying **every** `Action` variant in turn to a
      `Dashboard` whose selected change carries a marked artifact, asserting `changes` is `==`
      to its prior value after each, and asserting the variant count is exactly twelve so a
      later toggle action cannot be added without this test failing.
      Red-when: the test enumerates variants by hand and silently misses one added later. Use
      an exhaustive `match` over a constructed array whose length the test also asserts, so a
      new variant is a compile error rather than a silent gap.

- [x] 9.2 GREEN: No production change is expected — no `Action` is added and
      `Dashboard::apply` gains no arm. If one is needed, this change has crossed a PRD
      non-goal and must stop.

- [x] 9.3 CHECK: `UI_MIN=11 sh $CHECKS/READONLY-UI.sh` green, and its two planted-violation
      controls re-run and reverted: the `std::fs::write` in `ui::read_artifact` (expect FAIL)
      and the `std::fs::remove_file` inside a `#[cfg(test)]` module (expect OK, the
      deliberate exemption). `git status --porcelain src/` empty after both.

- [x] 9.4 REFACTOR: State that no refactor was needed, or name one — as in group 8, this
      group is expected to add tests and no production code.

- [x] 9.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::app::tests::' 48`; the four
      gate commands, still failing on exactly the one known acceptance test.
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 10. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [x] 10.1 VERIFY: `cargo test --all-features --lib 'ui::tests::detail::'` — three tests,
      all green, including `tasks_tab_is_read_only`, which has been red since group 2.
      Confirm the two pre-existing tests in that module still pass unchanged; if either
      needed its expectations updated, name the change and why here.

- [x] 10.2 REFACTOR: Clean up the group-2 harness — the ScratchDir builder and the scripted
      key list — if it duplicates `ui::tests::load::`'s existing tree builder; otherwise state
      that no refactor was needed.

- [x] 10.3 VERIFY: the literal, unqualified **`make check`** — its first run since group 1 —
      exits 0. If it fails, name the failing sub-command rather than summarising.
      `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::tests::detail::' 3`.
      `sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 11. Change Review
<!-- kind: operational -->

- [x] 11.1 CHECK: Dispatch an independent reviewer — a fresh subagent that did **not** write
      the implementation, given only `proposal.md`, the **nine** spec files, `design.md`, this
      file, and the diff. It reports findings and changes nothing. Point it first at the
      concentration points `openspec/config.yaml` names for this repository — nothing spawns
      outside `cli`; views do no I/O; nothing writes inside `openspec/`; every external
      dependency has an absent case; both producers emit the same type; view tests run at 60
      **and** 120 — and at the two defect classes this change was planned against: a check
      that cannot fail, and a check that cannot see what it guards in the formatting this
      codebase really uses.

- [x] 11.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run the affected tests.

- [x] 11.3 VERIFY: No blocking or unowned finding remains; every accepted WARNING carries its
      reason in `planning-review.md`. Commit.

---

## 12. Documentation
<!-- kind: operational -->

- [x] 12.1 Rewrite in `SPEC.md`: Degraded states, the "Schema loads with no tasks artifact"
      row (audience: anyone implementing against the contract) — it currently claims "the
      tasks tab is absent", which contradicts `detail-view`'s tab-per-declared-artifact rule
      and the roadmap note that `tasks-tab` adds no tab-bar code. Replace it with: no artifact
      is marked, so every tab renders as markdown and none renders the checklist; no tab is
      added, removed, or hidden. Durable because it is the row a future reader would implement
      a tab-removal branch from.

- [x] 12.2 Rewrite in `SPEC.md`: Degraded states, the "markdown source holds a construct the
      parser does not model" row (audience: the same) — it lists "a task-list item" among the
      constructs rendered as literal source text, unqualified by tab, which this change makes
      false on the tracked-tasks tab. Scope the row to every tab **other** than that one, the
      same scoping the twin sentence in Detail view already carries. Durable because it is the
      row a future reader would use to argue the tasks tab should render its source verbatim.

- [x] 12.3 Add to `SPEC.md`: Degraded states, two rows (audience: the same) — a tasks file
      that exists and yields no task **items** renders `No tasks yet`, distinct from
      `No content yet`, with no heading line even where the source carries headings; and a
      marked tab whose artifact resolves to no file renders `No content yet` while the header
      still shows the pair the `tasks.md` fallback counted. The second is the one place the
      tab's content and the change's progress legitimately disagree, and it is the row that
      stops a future reader "fixing" the divergence.

- [x] 12.4 Rewrite in `SPEC.md`: User interface → Detail view, the tasks-tab paragraph, plus
      the Module map `ui` row, Testing → Unit-tested modules, and View tests (audience: the
      same) — name `ui::tasks`, state that the tab is identified by **position** from the
      schema's tracked-tasks artifact rather than by id, and that the bar renders
      `Change::progress` so the pane shows one number per change. Net addition to `SPEC.md` is
      under ten lines beyond the row rewrites above; the tasks-tab paragraph is rewritten in
      place rather than appended to.

- [x] 12.5 Rewrite in `AGENTS.md`: **two** Architecture-rules bullets plus Current repo state
      (audience: every future session). (a) "Views do no I/O" — the pure set becomes **eight**
      files with `src/ui/tasks.rs` named. (b) "The detail region's two mandated interior widths
      are 78 and 58 … Every test in `ui::markdown` and `ui::detail` asserts both" — add
      `ui::tasks`, a third width-parameterised pure module with its own exemption-free gate;
      missing this one was a planning-review finding, and it is the kind of rule that goes
      stale silently because nothing executes it. (c) Current repo state — both its
      landed-changes enumeration, which currently ends at `detail-view`, and its running
      description, which gains one sentence on what the tasks tab now renders. Rewrite all
      three in place; do not append a second entry beside the existing ones.

- [x] 12.6 CHECK: Deferred to archive time, not doable now —
      `openspec/IMPLEMENTATION-ORDER.md`'s `tasks-tab` row and the note under "Notes on the
      ordering" must be confirmed to still describe what was built, and corrected if not.
      `OPENSPEC-UNTOUCHED` forbids writing anywhere under `openspec/` outside this change's
      own directory, so this is recorded here as a deferred obligation the way `list-view`
      (10.4b), `markdown-viewer` (11.8), and `detail-view` (13.4) did, and is discharged by
      `openspec archive`.

- [x] 12.7 VERIFY: `sh $CHECKS/OPENSPEC-UNTOUCHED.sh` — the doc edits are to `SPEC.md` and
      `AGENTS.md`, both outside `openspec/`, so it still prints OK. Commit.

---

## 13. Lint & Verify
<!-- kind: operational -->

- [ ] 13.1 CHECK: Inspect the intended verification commands and affected tiers: the four
      `make` gates, the twenty-three extracted checks, and the **nine** `testcount`
      invocations across **seven** distinct filters (`changes::tests::`, `ui::tasks::tests::`
      twice, `ui::detail::tests::`, `ui::view::tests::`, `ui::app::tests::` twice,
      `ui::driver::tests::`, `ui::tests::detail::`). Name any that this change's groups did
      not run at their final floors.

- [ ] 13.2 VERIFY: `make lint` — `cargo clippy --all-targets --all-features -- -D warnings`,
      0 errors.

- [ ] 13.3 VERIFY: `make fmt-check` — `cargo fmt --all -- --check`, clean. Rust has no
      separate type checker; `cargo clippy --all-targets` in 13.2 type-checks every target,
      including the test target, which is the equivalent step.

- [ ] 13.4 VERIFY: `make test` — `cargo test --all-features`, green.

- [ ] 13.5 VERIFY: `make coverage` — `cargo llvm-cov --fail-under-lines 80`, passing with no
      exclusion and no threshold change. Quote the TOTAL row's **line** column, not the region
      column it leads with (planning-time baseline: 97.81% over 14,178 lines). Then
      `sh $CHECKS/NOWAIVER.sh`.

- [ ] 13.6 VERIFY: Every check at its final floor, each printing its OK line verbatim:
      `MIN=19 sh $CHECKS/NOSPAWN-GREP.sh`; `sh $CHECKS/NOIO-VIEW.sh`;
      `UI_MIN=10 sh $CHECKS/READSEAM.sh`; `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`;
      `TYPES="Dashboard Filter Detail" SCAN_MIN=80 sh $CHECKS/NODEFAULT-UI.sh`;
      `MIN=19 sh $CHECKS/NOLIT-CHANGE.sh`; `MIN=19 sh $CHECKS/MDSEAM.sh`;
      `sh $CHECKS/NOTABSEAM.sh`; `sh $CHECKS/TASKSEAM.sh`; `sh $CHECKS/NORAW-GREP.sh`;
      `UI_MIN=11 sh $CHECKS/READONLY-UI.sh`; `WIDTHS_MIN=76 sh $CHECKS/WIDTHS.sh`;
      `LIST_MIN=17 sh $CHECKS/LISTWIDTHS.sh`; `MD_MIN=23 sh $CHECKS/MDWIDTHS.sh`;
      `DETAIL_MIN=23 sh $CHECKS/DETAILWIDTHS.sh`; `TASK_MIN=14 sh $CHECKS/TASKWIDTHS.sh`;
      `python3 $CHECKS/GATE-MECH1.py src`; `sh $CHECKS/NOJSON-SEAM.sh`;
      `BASE=85a5631 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Also
      `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::tests::read_artifact::' 2` — unchanged
      by this change, and asserted because `specs/artifact-content`'s read-binding requirement
      is one this change edits the text of without changing its verification.

- [ ] 13.7 VERIFY: `sh $CHECKS/DEPS.sh` and `sh $CHECKS/GRAPH-SNAP.sh` — both **unchanged**
      scripts, both green. This is the evidence that no dependency, feature, or proc-macro
      crept in, and `Cargo.toml`, `Cargo.lock`, and `tests/fixtures/build-graph.txt` are
      untouched. `git diff --name-only $BASE -- Cargo.toml Cargo.lock
      tests/fixtures/build-graph.txt herdr-plugin.toml` must be empty.

- [ ] 13.8 VERIFY: The library and `ui::` totals, asserted here and nowhere else:
      `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
      passed.*/\1/p' | awk '{t+=$1} END{print t+0}'` → **671** (625 measured + 46 new); and
      `cargo test --all-features --lib 'ui::' ...` → **258** (226 measured + 32 new). A count
      below either target means an enumerated test was not written — write it, do not lower
      the number.

- [ ] 13.9 VERIFY: `export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"` and
      `openspec validate tasks-tab --strict` — the `openspec` binary is nvm-installed here and
      is not on a non-login shell's default `PATH`.
