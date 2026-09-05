# markdown-viewer — tasks

Read `design.md` first: the Boundaries table, the Test Boundaries table, and the verification
matrix are the contract these tasks implement. **No task may invent a collaborator that table
does not name.**

**The outer loop is taken, at the `ui::` composition tier through `run_loop`.**
`ui::app::action_for` → `Dashboard::apply` → `ui::markdown::lines` → `ui::view::render` →
`Dashboard::normalise_scroll` is a path no unit test crosses, and it is the thing this change
exists to make work. Group 2 writes that test and group 9 closes it.

**Two groups come before the acceptance test, and that departs from `list-view`'s order on
purpose.** `list-view`'s group 0 added only a test file, so measuring afterwards was harmless
and the RED could be written against API that already existed. Neither holds here: the
acceptance test needs a `Dashboard` field to put a markdown source in, and it needs the
scripted `EventSource` double that is **private** to `src/ui/driver.rs`'s test module. Group 0
therefore measures the tree before anything is edited; group 1 — an **operational** group,
because a type and a test helper are plumbing rather than behaviour — adds those two pieces
and nothing else; group 2 is the behavioural acceptance group, and its RED is an assertion
failure against a blank interior rather than a compile failure. A crate whose test target does
not build makes `cargo clippy --all-targets` and `cargo llvm-cov --ignore-run-fail`
unrunnable, and would leave groups 3 to 8 with no gate at all.

**Every floor in this file is measured before it is raised.** Task 0.1 reads the real counts
off `main` and records them; every later `MIN`, `UI_MIN`, `WIDTHS_MIN`, `LIST_MIN`, `MD_MIN`,
and `testcount` minimum is *measured + this change's addition*. This repository has already
shipped a check whose floor was asserted rather than measured and was therefore unpassable.

**When a task says a check script was edited, that task writes the script to disk and runs
it.** `tui-shell` recorded four edits to `DEPS` as prose in a checkbox only; the script on
disk was never changed, and `list-view` nearly halted on a check that did not exist. Every
block below — carried forward, edited, or new — is reproduced in full and extracted to
`$CHECKS/<LABEL>.sh` in task 0.1, and every later run is of the extracted file.

## Test-module convention — read before writing any test

`cargo test` matches a filter against a test's **full path**, and a filter matching nothing
exits 0. Every filtered gate below therefore addresses tests through an explicit module path,
and is judged on a counted minimum rather than on the run's exit status.

| Group | File | Module | Filter | Target count |
|---|---|---|---|---|
| 4, 5 | `src/ui/markdown.rs` | `mod tests` | `ui::markdown::tests::` | 23 |
| 6 | `src/ui/app.rs` | `mod tests` → `mod keys` | `ui::app::tests::` | 30 |
| 6, 7 | `src/ui/layout.rs` | `mod tests` | `ui::layout::tests::` | 12 |
| 7 | `src/ui/view.rs` | `mod tests` | `ui::view::tests::` | 57 |
| 8 | `src/ui/driver.rs` | `mod tests` | `ui::driver::tests::` | 10 |
| 8 | `src/ui/mod.rs` | `mod tests` → `mod load` | `ui::tests::load::` | 6 |
| 2, 9 | `src/ui/mod.rs` | `mod tests` → `mod detail` | `ui::tests::detail::` | 1 |

**`make check` is not runnable unqualified between groups 2 and 9.** Group 2's acceptance test
is deliberately RED for that whole span, and `cargo llvm-cov` hard-fails on any test failure
and produces no report at all, `--no-fail-fast` included. At every intermediate boundary, "the
gate" therefore means these four commands, run separately:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features            # must fail on EXACTLY the one known acceptance test,
                                     # with the identical assertion message each time
cargo llvm-cov --ignore-run-fail --fail-under-lines 80
```

A **second** failing test at any of those boundaries is a real regression, not the known one.
The literal, unqualified `make check` is run from task 9.2 onward.

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
# NOSPAWN-GREP — `cli` is the only module in the crate that spawns a process.
# Carried forward from list-view BYTE-IDENTICALLY. Only its MIN argument moves, from 16 to
# 17, and MIN is a parameter precisely so that is a change to a task's invocation rather
# than to the check.
SRC="${SRC:-src}"
MIN="${MIN:-8}"
fail() { echo "NOSPAWN FAIL: $1" >&2; exit 1; }
SPAWN_RE='process::Command|Command::new|Stdio'

# Guard A — the tree and the one allowed spawner exist. A renamed, split, or moved seam
# (src/cli/mod.rs, say) must be a deliberate update to this block, never a silent stop.
[ -d "$SRC" ] || fail "no such directory: $SRC"
[ -f "$SRC/cli.rs" ] || fail "$SRC/cli.rs missing - the exclusion has nothing to exclude"

# Guard B — the excluded file actually spawns. Without this, a tree where the seam was
# gutted (or never written) passes, and the exclusion protects nothing.
grep -qE 'process::Command|Command::new' "$SRC/cli.rs" \
  || fail "$SRC/cli.rs names no spawn API - exclusion is vacuous"

# Guard C — the searched set is the crate's real module set, not an empty list. grep exits
# 2 on a missing file and `!` would pass that; this is the test -f-shaped guard.
# The exclusion is BY PATH, not by base name: `! -name 'cli.rs'` would silently exempt a
# future src/ui/cli.rs.
n=$(find "$SRC" -name '*.rs' ! -path "$SRC/cli.rs" | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files under $SRC (expected >= $MIN)"

# The check itself. Judged on OUTPUT EMPTINESS, never on a pipeline's exit status.
# The trailing /dev/null makes grep's argument list unconditionally non-empty, so the
# GNU-xargs empty-input case cannot make grep read stdin and hang.
hits=$(find "$SRC" -name '*.rs' ! -path "$SRC/cli.rs" -print0 \
       | xargs -0 -I{} grep -nE "$SPAWN_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NOSPAWN FAIL: spawn API outside $SRC/cli.rs:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOSPAWN OK: $n files checked under $SRC (>= $MIN), only $SRC/cli.rs may spawn"

# MODULE-SCOPED, stricter, and unweakened: these five files name no process API at all,
# spawning or not. The [ -f ... ] guards are load-bearing for the same exit-code-2 reason.
for f in src/resolve.rs src/config.rs src/state.rs src/schema.rs src/tasks.rs; do
  [ -f "$f" ] || { echo "MODULE-SCOPED FAIL: $f missing" >&2; exit 1; }
done
m=$(grep -nE 'std::process|Command|Stdio' src/resolve.rs src/config.rs src/state.rs \
    src/schema.rs src/tasks.rs || true)
[ -z "$m" ] || { echo "MODULE-SCOPED FAIL:" >&2; echo "$m" >&2; exit 1; }
```

```sh
# NOIO-VIEW — the PURE files of the render seam name no I/O API at all.
# Carried forward from list-view with ONE deliberate edit, recorded in design.md ->
# Boundaries: PURE gains src/ui/markdown.rs, so the set is SIX files rather than five.
# Three files under src/ui/ are still deliberately NOT searched, each for a stated reason:
# mod.rs holds ui::load and ui::run, terminal.rs holds the terminal seam, and event.rs
# holds CrosstermEvents, which reads the real event stream. Do not "fix" the list by
# adding them. A view test that needs a real directory is the signal this check exists to
# make impossible.
PURE="src/ui/app.rs src/ui/layout.rs src/ui/list.rs src/ui/markdown.rs src/ui/view.rs src/ui/driver.rs"
IO_RE='std::fs|std::io|std::env|std::process|std::net|File::|read_to_string|Command'

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
echo "NOIO-VIEW OK: 6 pure files carry no I/O API; positive control matched"
```

```sh
# NOCLI-SHELL — the dashboard shell never names the CLI seam. This is the mechanical form
# of the roadmap's "markdown-viewer depends on tui-shell and changes-from-files, not on the
# CLI".
# Carried forward from list-view BYTE-IDENTICALLY; only UI_MIN's invocation moves, 8 -> 9.
CLI_RE='from_cli|OpenspecCli|HerdrCli|CliChanges|npm_prefix'
UI_MIN="${UI_MIN:-7}"

# Guard A — src/ui/ exists and holds Rust files. A count of zero would make the search
# vacuous. Parameterised for the same reason NOSPAWN-GREP's MIN is: a later change that
# merges two files would turn a real check into a spurious failure, which is how a check
# gets weakened into a rubber stamp.
[ -d src/ui ] || { echo "NOCLI-SHELL FAIL: src/ui missing" >&2; exit 1; }
n=$(find src/ui -name '*.rs' | wc -l | tr -d ' ')
[ "$n" -ge "$UI_MIN" ] || { echo "NOCLI-SHELL FAIL: only $n files under src/ui (expected >= $UI_MIN)" >&2
                    exit 1; }

# Guard B — positive control. src/changes.rs MUST name OpenspecCli, or the pattern is
# broken and a clean result means nothing.
grep -qE 'OpenspecCli' src/changes.rs \
  || { echo "NOCLI-SHELL FAIL: positive control - src/changes.rs does not name OpenspecCli" >&2
       exit 1; }

hits=$(find src/ui -name '*.rs' -print0 | xargs -0 -I{} grep -nE "$CLI_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NOCLI-SHELL FAIL: the shell names the CLI seam:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOCLI-SHELL OK: $n files under src/ui name no CLI seam; positive control matched"
```

```sh
# NODEFAULT-UI — Dashboard, Filter, and Detail have no Default, and no construction or
# destructuring elides a field. change-model's GATE-MECH1 covers Change/ChangeSet/
# ArtifactRef/Origin in src/changes.rs ONLY, so a state type in src/ui/app.rs is outside it.
# Carried forward from list-view with ONE deliberate edit, recorded in design.md ->
# Boundaries: the TYPES default gains Detail, so markdown-viewer's third state type is
# covered by the SAME mechanism rather than by a second, drifting check. Both halves and
# every positive control are unchanged in substance.
SRC="${SRC:-src}"
# `${TYPES-...}` and NOT `${TYPES:-...}`: with the colon, an explicitly empty TYPES silently
# falls back to the default and the emptiness guard below can never fire. The guard also
# rejects a whitespace-only value, which `[ -n ]` alone would accept and which would make
# the loop body run zero times while still printing OK.
TYPES="${TYPES-Dashboard Filter Detail}"
[ -d "$SRC" ] || { echo "NODEFAULT-UI FAIL: no such directory: $SRC" >&2; exit 1; }
[ -f "$SRC/ui/app.rs" ] || { echo "NODEFAULT-UI FAIL: $SRC/ui/app.rs missing" >&2; exit 1; }
case "$TYPES" in *[![:space:]]*) ;;
  *) echo "NODEFAULT-UI FAIL: TYPES is empty - the loop would run zero times" >&2; exit 1;; esac

for T in $TYPES; do
  # Positive control — the file must actually declare the type, or every search below is
  # searching for a name that is not there and a clean result means nothing. ANCHORED on
  # both sides: an unanchored `struct $T` is satisfied by `struct FilterState` after a
  # rename, so the control would pass while every leg below searched for a name that is
  # no longer there.
  grep -qE "struct[[:space:]]+$T[[:space:]]*\{" "$SRC/ui/app.rs" \
    || { echo "NODEFAULT-UI FAIL: positive control - $SRC/ui/app.rs has no 'struct $T {'" >&2
         exit 1; }

  # Half A — no Default impl anywhere under src/, hand-written or derived. The derive form
  # is caught by taking the two lines preceding each `struct <T>` and looking for Default
  # inside a #[derive(...)] there.
  a=$(find "$SRC" -name '*.rs' -print0 \
      | xargs -0 -I{} grep -nE "impl[[:space:]]+Default[[:space:]]+for[[:space:]]+$T" {} /dev/null 2>&1 || true)
  b=$(find "$SRC" -name '*.rs' -print0 \
      | xargs -0 -I{} grep -B2 -nE "struct[[:space:]]+$T[[:space:]]*\{" {} /dev/null 2>&1 \
      | grep 'derive' | grep 'Default' || true)
  [ -z "$a$b" ] || { echo "NODEFAULT-UI FAIL: $T has a Default:" >&2
                     printf '%s\n%s\n' "$a" "$b" >&2; exit 1; }

  # Half B — no `..` inside a <T> literal or pattern. Matched on the same line, which is
  # how rustfmt writes a short elision (`Dashboard { quit, .. }`); a multi-line elision is
  # caught by the compile-time companions in app.rs's tests instead, which is the half a
  # grep genuinely cannot do.
  # Leading non-identifier guard, for the same reason: `SomeFilter { .. }` must not be
  # reported as a `Filter` elision.
  c=$(find "$SRC" -name '*.rs' -print0 \
      | xargs -0 -I{} grep -nE "(^|[^A-Za-z0-9_])$T[[:space:]]*\{[^}]*\.\." {} /dev/null 2>&1 || true)
  [ -z "$c" ] || { echo "NODEFAULT-UI FAIL: $T literal or pattern elides a field:" >&2
                   echo "$c" >&2; exit 1; }
done
echo "NODEFAULT-UI OK: no Default for [$TYPES], no elided field; positive controls matched"
```

```sh
# NORAW-GREP — crossterm's terminal-mode functions appear ONLY in src/ui/terminal.rs, and
# CrosstermOps is constructed only by ui::run. Searches tests/ as well as src/, because the
# failure this prevents is a TEST putting the developer's own terminal into raw mode.
# Carried forward from list-view BYTE-IDENTICALLY, invocation included: its file-count
# guard is >= 16 and the tree holds 18 today, 19 after this change.
RAW_RE='enable_raw_mode|disable_raw_mode|EnterAlternateScreen|LeaveAlternateScreen'

[ -f src/ui/terminal.rs ] || { echo "NORAW FAIL: src/ui/terminal.rs missing" >&2; exit 1; }

# Guard — positive control: the allowed file must actually name them, or the exclusion
# protects nothing.
grep -qE "$RAW_RE" src/ui/terminal.rs \
  || { echo "NORAW FAIL: src/ui/terminal.rs names no terminal-mode function - vacuous" >&2
       exit 1; }

# Guard — the searched set is real. src + tests together are at least 16 files today.
n=$(find src tests -name '*.rs' ! -path 'src/ui/terminal.rs' | wc -l | tr -d ' ')
[ "$n" -ge 16 ] || { echo "NORAW FAIL: searched only $n files (expected >= 16)" >&2; exit 1; }

hits=$(find src tests -name '*.rs' ! -path 'src/ui/terminal.rs' -print0 \
       | xargs -0 -I{} grep -nE "$RAW_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NORAW FAIL: terminal-mode function outside src/ui/terminal.rs:" >&2
                    echo "$hits" >&2; exit 1; }

# Second leg — CrosstermOps is named only where it is defined and where ui::run wires it.
# Any other site, especially under tests/, would mean a test can reach a real terminal.
sites=$(find src tests -name '*.rs' -print0 | xargs -0 -I{} grep -nE 'CrosstermOps' {} /dev/null 2>&1 || true)
bad=$(printf '%s\n' "$sites" | grep -v '^src/ui/terminal.rs:' | grep -v '^src/ui/mod.rs:' || true)
[ -z "$bad" ] || { echo "NORAW FAIL: CrosstermOps named outside terminal.rs and mod.rs:" >&2
                   echo "$bad" >&2; exit 1; }
c=$(printf '%s\n' "$sites" | grep -c 'CrosstermOps' || true)
[ "$c" -ge 2 ] || { echo "NORAW FAIL: CrosstermOps named $c times (expected >= 2: definition and wiring)" >&2
                    exit 1; }
echo "NORAW OK: $n files searched, mode functions only in src/ui/terminal.rs, CrosstermOps at $c sites"
```

```sh
# NOLIT-CHANGE — `Change { … }` and `ChangeSet { … }` literals and patterns appear ONLY in
# src/changes.rs. Carried forward from list-view BYTE-IDENTICALLY; only its MIN invocation
# moves, 16 -> 17, because src/ui/markdown.rs is added. This change creates NO new Change or
# ChangeSet construction site.
#
# Known limits, stated rather than discovered later: the pattern also matches `impl Change {`
# and `struct Change {`, and it matches inside a doc comment — all three are FALSE POSITIVES
# that a future file outside src/changes.rs would trip, and the fix is to move the code, not
# to weaken the check. It is also dodged by `use crate::changes::Change as C;` followed by
# `C { … }` — a FALSE NEGATIVE, and the reason `GATE-MECH1`'s compile-time companion in
# `changes::conformance` remains the primary mechanism rather than this grep.
LIT_RE='(^|[^A-Za-z0-9_])Change(Set)?[[:space:]]*\{'
MIN="${MIN:-15}"

[ -f src/changes.rs ] || { echo "NOLIT-CHANGE FAIL: src/changes.rs missing" >&2; exit 1; }

# Guard — positive control: the allowed file must actually hold such a literal, or a broken
# pattern reads as a clean tree.
grep -qE 'ChangeSet[[:space:]]*\{' src/changes.rs \
  || { echo "NOLIT-CHANGE FAIL: positive control - src/changes.rs has no 'ChangeSet {'" >&2
       exit 1; }

# Guard — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/changes.rs is still searched.
n=$(find src -name '*.rs' ! -path 'src/changes.rs' | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || { echo "NOLIT-CHANGE FAIL: searched only $n files (expected >= $MIN)" >&2
                         exit 1; }

hits=$(find src -name '*.rs' ! -path 'src/changes.rs' -print0 \
       | xargs -0 -I{} grep -nE "$LIT_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NOLIT-CHANGE FAIL: a Change/ChangeSet literal outside src/changes.rs:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOLIT-CHANGE OK: $n files searched (>= $MIN), Change/ChangeSet literals only in src/changes.rs"
```

```sh
# MDSEAM — NEW in markdown-viewer. The markdown parser is confined to ONE module, on the
# same terms as src/cli.rs being the crate's only process spawner: pulldown_cmark is named
# only in src/ui/markdown.rs, so the parser can be replaced by editing one file. Its second
# leg is the other half of the same seam: that module returns PLAIN DATA and therefore names
# no ratatui type — styling is ui::view's job, exactly as it is for ui::list's row grammar.
# Its third leg proves the `html` default feature is off in SOURCE terms.
#
# NOTE on the MIN baseline: MDSEAM excludes src/ui/markdown.rs, which is the file this change
# ADDS. Its searched count is therefore 17 both before and after — unlike NOSPAWN-GREP's and
# NOLIT-CHANGE's, which go 16 -> 17. Measured in task 0.1; do not copy the wrong number.
#
# Known limit, stated rather than discovered later: all three legs are greps over whole
# files, comments included. The module's doc comment must therefore say "the view" rather
# than naming ratatui, and no other module may mention pulldown_cmark even in prose. That is
# a deliberate cost: an exemption for comments is how a confinement check rots into a rubber
# stamp, and design.md -> Decisions records it.
MD="${MD:-src/ui/markdown.rs}"
MIN="${MIN:-16}"
fail() { echo "MDSEAM FAIL: $1" >&2; exit 1; }

[ -d src ] || fail "no src directory"
[ -f "$MD" ] || fail "$MD missing - the exclusion has nothing to exclude"

# Guard B — positive control: the allowed file must actually name the parser, or the
# exclusion protects nothing and a clean result means nothing. Checked BEFORE the count, so
# a gutted markdown.rs is reported as vacuous rather than as a file-count shortfall.
grep -qE 'pulldown_cmark' "$MD" || fail "$MD names no pulldown_cmark - exclusion is vacuous"

# Guard C — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/markdown.rs is still searched.
n=$(find src -name '*.rs' ! -path "$MD" | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

hits=$(find src -name '*.rs' ! -path "$MD" -print0 \
       | xargs -0 -I{} grep -nE 'pulldown_cmark|pulldown-cmark' {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "MDSEAM FAIL: the markdown parser is named outside $MD:" >&2
                    echo "$hits" >&2; exit 1; }

# Second leg — the confined module names no ratatui type. Judged on output emptiness too.
r=$(grep -nE 'ratatui|Modifier|Style|Span' "$MD" || true)
[ -z "$r" ] || { echo "MDSEAM FAIL: $MD names a ratatui type:" >&2; echo "$r" >&2; exit 1; }

# Third leg — the html feature is observably off in SOURCE terms: no file anywhere names a
# pulldown_cmark::html path. The graph half of the same claim is GRAPH-SNAP's named
# absences, and the manifest half is DEPS legs 2b and 2d.
h=$(find src -name '*.rs' -print0 | xargs -0 -I{} grep -nE 'pulldown_cmark::html|cmark::html' {} /dev/null 2>&1 || true)
[ -z "$h" ] || { echo "MDSEAM FAIL: a pulldown_cmark::html path is named:" >&2; echo "$h" >&2; exit 1; }
echo "MDSEAM OK: $n files searched (>= $MIN), pulldown_cmark only in $MD, no ratatui type there, no html path"
```

```sh
# WIDTHS — every #[test] in src/ui/view.rs names both 60 and 120. A heuristic, and design.md
# -> Risks says so: it cannot prove an assertion is meaningful, only that both widths are
# present. It is the cheap half of the mandate; the per-scenario tasks are the other half.
# Carried forward from list-view BYTE-IDENTICALLY; only WIDTHS_MIN's invocation moves,
# 47 -> 57.
#
# Known limits, stated rather than discovered later: a `60` in a comment satisfies it, and
# so does an unrelated `60` literal. It is a FLOOR. What proves the tests exist and run is
# `testcount --lib 'ui::view::tests::' 57`, not this.
[ -f src/ui/view.rs ] || { echo "WIDTHS FAIL: src/ui/view.rs missing" >&2; exit 1; }
WIDTHS_MIN="${WIDTHS_MIN:-16}" python3 - <<'PY'
import re, sys, os
raw = open("src/ui/view.rs").read()
# Drop doc-comment and inner-doc lines before splitting: a `#[test]` inside a doc example
# would otherwise inflate the count and let a gutted file pass the floor.
src = "\n".join(l for l in raw.splitlines()
                if not l.lstrip().startswith("///") and not l.lstrip().startswith("//!"))
# Split on a line-anchored attribute, so `#[test]` inside a string or a trailing comment
# does not create a part.
parts = re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", src)[1:]
floor = int(os.environ["WIDTHS_MIN"])
if len(parts) < floor:
    print(f"WIDTHS FAIL: found {len(parts)} #[test] functions in src/ui/view.rs, expected >= {floor}",
          file=sys.stderr); sys.exit(1)
bad = []
for p in parts:
    name = re.search(r"fn\s+([a-z0-9_]+)", p)
    name = name.group(1) if name else "<unnamed>"
    nums = set(re.findall(r"\b(\d+)\b", p))
    if not ({"60", "120"} <= nums):
        bad.append(name)
if bad:
    print("WIDTHS FAIL: these view tests do not name both 60 and 120: " + ", ".join(bad),
          file=sys.stderr); sys.exit(1)
print(f"WIDTHS OK: all {len(parts)} view tests name both 60 and 120")
PY
```

```sh
# LISTWIDTHS — every #[test] in src/ui/list.rs names both 38 and 58, the two list-region
# INTERIORS the mandated 120- and 60-column frames produce. Carried forward from list-view
# BYTE-IDENTICALLY, invocation included: its floor is 17 and this change adds no list test.
#
# Known limit inherited from WIDTHS, stated rather than discovered later: the number scan is
# `\b(\d+)\b`, which does NOT see a suffixed literal such as `38u16` or `38usize`. It fails
# closed — a test using only suffixed literals is reported as missing a width — so write the
# widths unsuffixed, or as a `const` the test also names bare.
[ -f src/ui/list.rs ] || { echo "LISTWIDTHS FAIL: src/ui/list.rs missing" >&2; exit 1; }
LIST_MIN="${LIST_MIN:-17}" python3 - <<'PY'
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
```

```sh
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
```

```sh
# NOWAIVER — the coverage floor was not lowered, excluded, or annotated away.
# Carried forward from list-view BYTE-IDENTICALLY.
grep -q -- '--fail-under-lines 80' Makefile \
  || { echo "NOWAIVER FAIL: Makefile no longer names --fail-under-lines 80" >&2; exit 1; }
bad=$(grep -rnE 'coverage\(off\)|--ignore-filename-regex|--exclude|fail-under-lines ([0-7][0-9]?|[0-9])\b' \
      Makefile .github/workflows src tests 2>/dev/null || true)
[ -z "$bad" ] || { echo "NOWAIVER FAIL: a coverage waiver entered the tree:" >&2
                   echo "$bad" >&2; exit 1; }
echo "NOWAIVER OK: floor is 80, no exclusion, no coverage attribute"
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
# Exactly ONE exclusion: this change's own artifact directory, a hand-edited planning
# document rather than a code-path write. Excluded BY NAME, never by a broad prefix, so a
# stray file anywhere else under openspec/ still fails. Carried forward from list-view with
# that one path updated.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --ignored --exclude-standard -- ':(top)openspec/'; } \
         | grep -v '^openspec/changes/markdown-viewer/' | sort -u || true)
[ -z "$stray" ] || { echo "FAIL: wrote inside openspec/ outside this change:" >&2
                     echo "$stray" >&2; exit 1; }
echo "OPENSPEC-UNTOUCHED OK"
```

```sh
# TESTCOUNT — a `cargo test` filter that matches NOTHING exits 0. Verified in a previous
# change: `cargo test --all-features this_name_does_not_exist` prints
# "0 passed; ... filtered out" and returns 0. Every VERIFY that is a filtered run therefore
# passes if the module is renamed, if the tests are never written, or if the filter has a
# typo. The gate is a counted minimum taken from the group's own RED.
#
# The SCOPE parameter is load-bearing: without it the helper sums every test binary, so a
# whole-suite gate passes against a lib baseline with zero new lib tests.
#
# The `[ $? -eq 0 ]` after the `case` reads cargo's status, not the case's: the branch's
# last command is an assignment whose value comes from a command substitution, and both
# `case` and the assignment propagate that status. Re-verified under /bin/sh, bash, zsh, and
# dash during this change's planning review.
#
# This block DEFINES a shell function; it must be SOURCED (`. $CHECKS/TESTCOUNT.sh`) in
# every shell that runs a gate, not executed with `sh`.
#   usage: testcount <scope> <filter> <minimum>   scope: --lib | --test-cli
# Carried forward from list-view BYTE-IDENTICALLY.
testcount() {
  scope=$1; filter=$2; min=$3
  case "$scope" in
    --lib)      out=$(cargo test --all-features --lib "$filter" 2>&1) ;;
    --test-cli) out=$(cargo test --all-features --test cli 2>&1) ;;
    *) echo "TESTCOUNT FAIL: bad scope '$scope'" >&2; return 1 ;;
  esac
  [ $? -eq 0 ] || { printf '%s\n' "$out" >&2; return 1; }
  n=$(printf '%s\n' "$out" | sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' \
      | awk '{t+=$1} END{print t+0}')
  [ "$n" -ge "$min" ] || {
    echo "TESTCOUNT FAIL: $scope filter '$filter' ran $n tests, expected >= $min" >&2
    return 1; }
  echo "TESTCOUNT OK: $scope filter '$filter' ran $n tests (>= $min)"
}
```

```sh
# DEPS — the argued dependency set, the one binary target, the MSRV floor, the html-feature
# decision, and the genuinely-needed experiment. Reads `cargo metadata` JSON through python3
# rather than adding a crate to do it.
#
# Carried forward from list-view with FOUR deliberate edits, every one of them APPLIED HERE
# AND WRITTEN OUT rather than described: (1) leg 2a's want dict gains "pulldown-cmark": [],
# and "exactly four" becomes "exactly five"; (2) leg 2b's manifest-text clause is generalised
# to a loop over BOTH crates that must spell out `features = []`; (3) a NEW leg 2d proves the
# html and getopts defaults are off in manifest terms, complementing MDSEAM's source leg and
# GRAPH-SNAP's graph leg; (4) leg 5 gains a fifth removal experiment for pulldown-cmark.
# tui-shell recorded its own four edits as prose in a checkbox and never changed the file;
# this block IS the file.
#   env: WORK  scratch directory (required, guarded); leg 5 copies the crate into it,
#              because `cargo build` REWRITES Cargo.lock during resolution before it
#              reaches the compile error the leg waits for, so an edit-and-restore in
#              place would silently discard the resolution this change verified.
#        DEPS_SKIP_LEG5=1 skips the five full builds.
set -u
: "${WORK:?DEPS FAIL: WORK (a scratch directory) must be set}"
[ -d "$WORK" ] || { echo "DEPS FAIL: WORK=$WORK is not a directory" >&2; exit 1; }
[ -f Cargo.toml ] && [ -d src ] || { echo "DEPS FAIL: run from the crate root" >&2; exit 1; }
TRIPLES='aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu'

# --- leg 1: exactly one bin target, and build.sh actually produces it ---------
cargo metadata --no-deps --format-version 1 | python3 -c '
import json,sys
m=json.load(sys.stdin); p=m["packages"][0]
bins=[t for t in p["targets"] if "bin" in t["kind"]]
assert p["name"]=="herdr-openspec", p["name"]
assert len(bins)==1 and bins[0]["name"]=="herdr-openspec", bins
assert p["edition"]=="2024", p["edition"]
print("DEPS OK (leg 1a): exactly one bin target, herdr-openspec, edition 2024")
' || { echo "DEPS FAIL: leg 1a" >&2; exit 1; }
# The deletion and the EXIT STATUS are both load-bearing: build.sh has a real failure
# path (`error: cargo not found`, exit 1) and a leftover binary from any earlier build
# satisfies an existence check regardless of what the script did.
rm -f target/release/herdr-openspec
/bin/sh scripts/build.sh >/dev/null 2>&1 \
  || { echo "DEPS FAIL: leg 1b scripts/build.sh exited non-zero" >&2; exit 1; }
[ -x target/release/herdr-openspec ] \
  || { echo "DEPS FAIL: leg 1b no executable at target/release/herdr-openspec" >&2; exit 1; }
echo "DEPS OK (leg 1b): scripts/build.sh exited 0 and produced an executable binary"

# --- leg 2: the declared set, read from RESOLVED metadata, not manifest text --
cargo metadata --no-deps --format-version 1 | python3 -c '
import json,sys
want={"serde_json":["std"],
      "toml":["display","parse","serde","std"],
      "yaml-rust2":[],
      "ratatui":["crossterm"],
      "pulldown-cmark":[]}
deps=[d for d in json.load(sys.stdin)["packages"][0]["dependencies"] if d["kind"] is None]
got={d["name"]:d for d in deps}
assert sorted(got)==sorted(want), f"normal deps are {sorted(got)}, expected {sorted(want)}"
assert "crossterm" not in got, "crossterm is declared directly - it must be reached through ratatui only"
for n,f in want.items():
    assert got[n]["uses_default_features"] is False, f"{n} uses default features"
    assert sorted(got[n]["features"])==sorted(f), "%s features %s != %s" % (n, got[n]["features"], f)
print("DEPS OK (leg 2a): exactly", len(deps), "normal deps, defaults off, features exact")
' || { echo "DEPS FAIL: leg 2a" >&2; exit 1; }
# `cargo metadata` reports [] for both `features = []` and an omitted key and cannot
# tell them apart; the requirement is about what the manifest says on its face, so
# this clause is read from the TEXT. Both empty-feature crates are checked, in a loop,
# so adding a third is one word rather than a copied line.
for c in yaml-rust2 pulldown-cmark; do
  grep -q "$c = .*features = \[\]" Cargo.toml \
    || { echo "DEPS FAIL: leg 2b $c does not spell out features = [] in Cargo.toml" >&2; exit 1; }
done
echo "DEPS OK (leg 2b): yaml-rust2 and pulldown-cmark spell out features = [] in Cargo.toml"
# leg 2a-bis: crossterm is ABSENT from the declared set (asserted above) and PRESENT in the
# resolved graph. Both halves are needed: absent-and-absent would mean the backend is gone,
# and declared-and-present is the two-crossterm-versions hazard the design forbids.
cargo tree -e normal 2>/dev/null | grep -qE '(^|[^A-Za-z0-9_-])crossterm v' \
  || { echo "DEPS FAIL: leg 2a-bis crossterm is not in the resolved normal graph" >&2; exit 1; }
echo "DEPS OK (leg 2a-bis): crossterm undeclared but resolved, reached through ratatui"
cargo build --locked >/dev/null 2>&1 \
  || { echo "DEPS FAIL: leg 2c cargo build --locked" >&2; exit 1; }
echo "DEPS OK (leg 2c): cargo build --locked"
# --- leg 2d: NEW. pulldown-cmark's own defaults are off, in MANIFEST terms. The source
# half of the same claim is MDSEAM's third leg (no pulldown_cmark::html path anywhere) and
# the graph half is GRAPH-SNAP's named absences (no getopts, no pulldown-cmark-escape).
# Asserted three ways because each alone is dodgeable: a manifest can say one thing while a
# feature is re-enabled transitively, a graph can be clean while dead code names the module,
# and a source sweep says nothing about what is built.
grep -q 'pulldown-cmark = .*default-features = false' Cargo.toml \
  || { echo "DEPS FAIL: leg 2d pulldown-cmark does not declare default-features = false" >&2; exit 1; }
cargo metadata --no-deps --format-version 1 | python3 -c '
import json,sys
d=[x for x in json.load(sys.stdin)["packages"][0]["dependencies"] if x["name"]=="pulldown-cmark"]
assert len(d)==1, d
assert d[0]["uses_default_features"] is False
assert d[0]["features"]==[], d[0]["features"]
assert d[0]["kind"] is None, "pulldown-cmark must be a normal dependency, not dev or build"
print("DEPS OK (leg 2d): pulldown-cmark is a normal dependency with no features and no defaults")
' || { echo "DEPS FAIL: leg 2d" >&2; exit 1; }

# --- leg 3: DELETED. Superseded by GRAPH-SNAP, which compares the resolved graph
# against tests/fixtures/build-graph.txt per triple and enforces a proc-macro ALLOWLIST.
# TRIPLES is kept: leg 4 still uses it.

# --- leg 4: MSRV, compared against Cargo.toml's OWN rust-version -------------
# A check carrying its own literal floor keeps enforcing the old value when the
# crate's rust-version moves, and the requirement is a claim about the relationship.
# The at-the-floor set is PRINTED, not asserted: a fixed expected set rots the moment any
# dependency bumps its own rust-version. The assertion is only that the set is non-empty
# (so the intersection matched something) and that nothing is above the floor.
{ for t in $TRIPLES; do cargo tree -e normal --target "$t" 2>/dev/null; done; } \
  | grep -oE '[A-Za-z0-9_-]+ v[0-9][^ ]*' | sed 's/ v/\t/' | sort -u > "$WORK/graph-pairs.tsv"
cargo metadata --format-version 1 | WORK="$WORK" python3 -c '
import json,sys,re,os
pairs=set()
for line in open(os.environ["WORK"]+"/graph-pairs.tsv"):
    n,v=line.rstrip("\n").split("\t"); pairs.add((n,v.split()[0]))
m=json.load(sys.stdin)
floor=None
for p in m["packages"]:
    if p["name"]=="herdr-openspec": floor=p["rust_version"]
assert floor, "the crate declares no rust-version"
def key(v):
    # Zero-pad to three components: "1.85" and "1.85.0" are the same floor, and a
    # bare tuple comparison would rank (1,85,0) above (1,85) and report a false
    # violation for every package declaring the patch digit.
    n=[int(x) for x in re.findall(r"\d+", v)[:3]]
    return tuple(n+[0]*(3-len(n)))
over=[]; at=[]
for p in m["packages"]:
    if (p["name"], p["version"]) not in pairs: continue
    rv=p.get("rust_version")
    if not rv: continue
    if key(rv)>key(floor): over.append((p["name"],p["version"],rv))
    elif key(rv)==key(floor): at.append(p["name"])
assert not over, f"packages above the {floor} floor: {over}"
assert at, "no package sits at the floor - the intersection matched nothing"
print(f"DEPS OK (leg 4): floor {floor} from Cargo.toml; at the floor: {sorted(set(at))}")
' || { echo "DEPS FAIL: leg 4" >&2; exit 1; }

# --- leg 5: each dependency is genuinely needed, tested in a COPY ------------
[ "${DEPS_SKIP_LEG5:-0}" = "1" ] && { echo "DEPS SKIPPED (leg 5) by DEPS_SKIP_LEG5=1"; exit 0; }
before=$(git status --porcelain -- Cargo.toml Cargo.lock)
needed() { # $1 crate name, $2 the module whose failure is expected
  d="$WORK/deps-$1"; rm -rf "$d"; mkdir -p "$d"
  cp -R Cargo.toml Cargo.lock src tests rustfmt.toml "$d/"
  # The guard: removing a crate the manifest does not carry must be a FAILURE OF THE
  # CHECK, never counted as a pass — otherwise the leg silently succeeds against a
  # manifest that never declared it.
  grep -q "^$1 = " "$d/Cargo.toml" \
    || { echo "DEPS FAIL: leg 5 '$1' is not declared in Cargo.toml - nothing to remove" >&2; exit 1; }
  grep -v "^$1 = " "$d/Cargo.toml" > "$d/Cargo.toml.new" && mv "$d/Cargo.toml.new" "$d/Cargo.toml"
  if ( cd "$d" && cargo build >/dev/null 2>&1 ); then
    echo "DEPS FAIL: leg 5 the crate still builds without $1 - it is not genuinely needed" >&2
    exit 1
  fi
  echo "DEPS OK (leg 5/$1): removing it breaks the build ($2 depends on it)"
}
needed toml "config and state"
needed yaml-rust2 "schema"
needed serde_json "changes::from_cli"
needed ratatui "ui"
needed pulldown-cmark "ui::markdown"
# The guard itself, exercised: a crate the manifest does not carry must be reported as
# a failure of the check. Run in a subshell so its `exit 1` does not end this script.
if ( needed notacrate "nothing" ) >/dev/null 2>&1; then
  echo "DEPS FAIL: leg 5's not-declared guard did not fire" >&2; exit 1
fi
echo "DEPS OK (leg 5 guard): removing an undeclared crate is reported as a failure"
after=$(git status --porcelain -- Cargo.toml Cargo.lock)
[ "$before" = "$after" ] \
  || { echo "DEPS FAIL: leg 5 changed Cargo.toml or Cargo.lock in the working tree" >&2; exit 1; }
echo "DEPS OK: the working tree is unchanged, Cargo.lock included"
```

```sh
# GRAPH-SNAP — the resolved normal build graph equals the committed per-triple snapshot.
# Regenerate with: GRAPH_WRITE=1 sh GRAPH-SNAP.sh
#
# Carried forward from tui-shell with ONE deliberate edit, APPLIED HERE AND WRITTEN OUT: the
# named-absence list grows from `encoding_rs time` to `encoding_rs time getopts
# pulldown-cmark-escape`, so pulldown-cmark's own default features are proven off in GRAPH
# terms as well as in manifest terms (DEPS legs 2b and 2d) and source terms (MDSEAM leg 3).
# The snapshot file itself is regenerated in task 3.3 and its diff reviewed in task 3.4.
#
# NOTE on the named absences' negative control: appending a stray line to the SNAPSHOT only
# trips `diff -u`, because the absence loop reads the REGENERATED graph, never $SNAP. The
# control that exercises the loop is task 10.2 item 17: turn on the `html` feature in a copy,
# regenerate the snapshot there (GRAPH_WRITE=1) so the diff passes, then run the check.
SNAP=tests/fixtures/build-graph.txt
TRIPLES="aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu"
TMP=$(mktemp -d) || exit 1
trap 'rm -rf "$TMP"' EXIT

for t in $TRIPLES; do
  # Strip cargo's "(*)" repeat marker and "(proc-macro)" tag, drop this crate's own line,
  # and sort. `--target <triple>` once per triple, never `--target all`, which reports
  # optional resolutions cargo never builds.
  cargo tree -e normal --target "$t" --prefix none 2>/dev/null \
    | sed -e 's/ (\*)$//' -e 's/ (proc-macro)$//' -e '/^$/d' \
    | grep -v '^herdr-openspec ' | sort -u > "$TMP/$t"
  lines=$(wc -l < "$TMP/$t" | tr -d ' ')
  [ "$lines" -ge 40 ] || { echo "GRAPH-SNAP FAIL: $t resolved only $lines packages" >&2; exit 1; }
  { echo "## $t"; cat "$TMP/$t"; } >> "$TMP/all"
done

if [ -n "${GRAPH_WRITE:-}" ]; then
  mkdir -p "$(dirname "$SNAP")"; cp "$TMP/all" "$SNAP"; echo "GRAPH-SNAP WROTE $SNAP"; exit 0
fi

[ -f "$SNAP" ] || { echo "GRAPH-SNAP FAIL: $SNAP missing" >&2; exit 1; }
diff -u "$SNAP" "$TMP/all" || { echo "GRAPH-SNAP FAIL: graph differs from the snapshot" >&2; exit 1; }

# The macOS pair and the Linux pair each agree; macOS and Linux differ by exactly
# linux-raw-sys. Asserted as a NAMED difference, because the two are no longer identical.
diff -q "$TMP/aarch64-apple-darwin" "$TMP/x86_64-apple-darwin" >/dev/null \
  || { echo "GRAPH-SNAP FAIL: the two macOS triples disagree" >&2; exit 1; }
diff -q "$TMP/aarch64-unknown-linux-gnu" "$TMP/x86_64-unknown-linux-gnu" >/dev/null \
  || { echo "GRAPH-SNAP FAIL: the two Linux triples disagree" >&2; exit 1; }
d=$(diff "$TMP/aarch64-apple-darwin" "$TMP/aarch64-unknown-linux-gnu" \
    | grep -E '^[<>]' | sed 's/^[<>] //' | cut -d' ' -f1 | sort -u | tr '\n' ' ')
[ "$d" = "linux-raw-sys " ] \
  || { echo "GRAPH-SNAP FAIL: macOS/Linux differ by [$d], expected [linux-raw-sys ]" >&2; exit 1; }

# Proc-macro allowlist, as a SET EQUALITY: a ninth arriving fails, and an eighth vanishing
# fails too, so the list cannot rot into a description of whatever happens to be there.
# pulldown-cmark adds none, which is part of why it was chosen.
allow="darling_macro derive_more-impl document-features indoc instability rustversion strum_macros thiserror-impl"
got=$(cargo tree -e normal --prefix none 2>/dev/null | grep '(proc-macro)' \
      | sed -e 's/ (\*)//' -e 's/ (proc-macro)//' | cut -d' ' -f1 | sort -u | tr '\n' ' ')
want=$(printf '%s\n' $allow | sort -u | tr '\n' ' ')
[ "$got" = "$want" ] || { echo "GRAPH-SNAP FAIL: proc-macro set is [$got], expected [$want]" >&2; exit 1; }

# Four named absences, each proving a default-feature decision is in effect rather than
# merely written down. encoding_rs would arrive with yaml-rust2's defaults, time with
# ratatui's all-widgets, and getopts and pulldown-cmark-escape with pulldown-cmark's own
# `getopts` and `html` defaults.
for absent in encoding_rs time getopts pulldown-cmark-escape; do
  if grep -qE "^$absent " "$TMP/all"; then
    echo "GRAPH-SNAP FAIL: $absent is in the normal build graph" >&2; exit 1
  fi
done
echo "GRAPH-SNAP OK: four triples match the snapshot; proc-macro set exact; four named absences hold"
```

**Carried forward unchanged**, extracted byte-identically rather than retyped:
`GATE-MECH1.py` and `NOJSON-SEAM.sh` from
`openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md`.

---

## 0. Baseline, checks, and the scratchpad
<!-- kind: operational -->

- [x] 0.1 CHECK: Record the starting state before any edit, by **measuring**, never by
      copying a number from this file.
      - `git rev-parse HEAD` → `export BASE=<sha>`. Every `OPENSPEC-UNTOUCHED` run uses it.
        A diff against the index would pass over this change's own per-group commits.
        **Measured at planning time: `4f93fa70fec2512b449e5f6299066b1b7eff8c64`.**
      - `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
        passed.*/\1/p' | awk '{t+=$1} END{print t+0}'` → the **lib** baseline.
        **Measured at planning time: 516**, inside a 532-test whole suite.
      - `find src -name '*.rs' ! -path 'src/cli.rs' | wc -l` → `NOSPAWN-GREP`'s starting file
        count. **Measured at planning time: 16**; the floor becomes 17.
      - `find src -name '*.rs' ! -path 'src/changes.rs' | wc -l` → `NOLIT-CHANGE`'s starting
        file count. **Measured at planning time: 16**; the floor becomes 17.
      - `find src -name '*.rs' ! -path 'src/ui/markdown.rs' | wc -l` → **`MDSEAM`'s** starting
        file count, which is a *different* number from the two above: `MDSEAM` excludes the
        file this change adds, so its count is **17 today and 17 after**.
        **Measured at planning time: 17**, and its floor is 17 throughout.
      - `find src/ui -name '*.rs' | wc -l` → `NOCLI-SHELL`'s `UI_MIN` baseline.
        **Measured at planning time: 8**; the floor becomes 9.
      - `find src tests -name '*.rs' ! -path 'src/ui/terminal.rs' | wc -l` → `NORAW-GREP`'s
        guard. **Measured at planning time: 18** (its floor is 16 and does not move).
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/view.rs` → `WIDTHS`' baseline.
        **Measured at planning time: 47**; the floor becomes 57.
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/list.rs` → `LISTWIDTHS`' baseline.
        **Measured at planning time: 17** (its floor is 17, exactly met today and unmoved).
      - `grep -rn 'Dashboard[[:space:]]*{' src/ | wc -l` → the `Dashboard` construction-site
        sweep group 1 must complete. **Measured at planning time: 28 grep hits, of which 14
        are real literal or pattern sites**, across **six** files: `src/lib.rs` 1,
        `src/ui/mod.rs` 2, `src/ui/list.rs` 4, `src/ui/app.rs` 3, `src/ui/view.rs` 3,
        `src/ui/driver.rs` 1. The other 14 hits are `pub struct Dashboard {`,
        `impl Dashboard {`, and `-> Dashboard {` signatures.
      - `cargo llvm-cov --summary-only` → TOTAL **line** coverage. The TOTAL row leads with
        the **region** count; read the line column, further right.
        **Measured at planning time: 98.26% over 10,493 lines.**
      - `export CHECKS=<scratchpad>/markdown-viewer-checks` and
        `export WORK=<scratchpad>/markdown-viewer-work`; `mkdir -p "$CHECKS" "$WORK"`.
        Extract every fenced block above — `DEPS.sh` and `GRAPH-SNAP.sh` included, which this
        file writes out in their **edited** form rather than inheriting — to
        `$CHECKS/<LABEL>.sh` byte-identically, and extract `GATE-MECH1.py` and
        `NOJSON-SEAM.sh` from
        `openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md`, byte-identically.
        `DEPS` needs `WORK` set and refuses to run without it.
        Run the extracted files from here on, never a retyped copy. `TESTCOUNT.sh` is the
        exception: it only *defines* a function, so **source** it (`. $CHECKS/TESTCOUNT.sh`)
        in each shell that runs a gate.
      **Red when:** `BASE` is empty or not a commit (`OPENSPEC-UNTOUCHED` aborts on both), or
      a measured count differs from the planning-time figure recorded above, in which case
      the tree is not the one this plan was written against and the discrepancy is resolved
      before any edit.

- [x] 0.2 CHECK: Run the checks that must **pass** on the tree as it stands, so this change
      starts from clean gates rather than inheriting broken ones: `NOSPAWN-GREP` (default
      `MIN=8`), `NOCLI-SHELL` (default `UI_MIN=7`), `NORAW-GREP`, `NODEFAULT-UI` with
      `TYPES="Dashboard Filter"` (its edited default names `Detail`, which does not exist
      until group 1, so the pre-change run must pass the old list explicitly), `NOLIT-CHANGE`,
      `WIDTHS` (default floor 16), `LISTWIDTHS`, `NOWAIVER`, `GATE-MECH1`, `NOJSON-SEAM`,
      `GRAPH-SNAP`, and `OPENSPEC-UNTOUCHED`. Record each output verbatim. **`GRAPH-SNAP` is
      in this list deliberately: it must pass today**, on the pre-dependency snapshot, which
      is what makes task 3.3's regeneration a reviewable change rather than an unexplained
      one.
      Four checks are **expected to fail** here, each for a stated reason, and each failure is
      recorded as the pre-change baseline rather than as a problem:
      - `MDSEAM` and `MDWIDTHS` — `src/ui/markdown.rs` does not exist yet.
      - `NOIO-VIEW` — its six-file `PURE` list names `src/ui/markdown.rs`.
      - `DEPS` (with `WORK` set and `DEPS_SKIP_LEG5=1`) — leg 2a's want dict names
        `pulldown-cmark`, which is not yet declared: expect `normal deps are ['ratatui',
        'serde_json', 'toml', 'yaml-rust2'], expected ['pulldown-cmark', 'ratatui',
        'serde_json', 'toml', 'yaml-rust2']`.
      **Red when:** any of the twelve expected-to-pass checks fails, in which case this change
      is not the place to fix it.
      **Wrongly green when:** an extraction typo made a block a no-op — guarded by 0.3.

- [x] 0.3 CHECK: Prove the **existence guards** and **positive controls** of every extracted
      check can fail, before trusting any of them. Against throwaway copies under `$WORK`:
      - `NOSPAWN-GREP` with `SRC=$WORK/empty` → `no such directory`.
      - `NOSPAWN-GREP` with `MIN=17` on the current tree → `searched only 16 files under src
        (expected >= 17)`, which is exactly the floor this change will make true.
      - `NOLIT-CHANGE` with `MIN=17` on the current tree → `searched only 16 files
        (expected >= 17)` — the same shape.
      - `NOCLI-SHELL` with `UI_MIN=9` on the current tree → `only 8 files under src/ui
        (expected >= 9)` — the same shape.
      - `MDSEAM` in a copy that has an **empty** `src/ui/markdown.rs`, run with **`MIN=99`**
        → the message must be `src/ui/markdown.rs names no pulldown_cmark - exclusion is
        vacuous`, **not** a file-count shortfall, which is what proves the positive control is
        evaluated before the count. `MIN=17` would not prove it: the copy already has 17
        searched files and would pass the count anyway.
      - `NOIO-VIEW`, `MDWIDTHS`, `LISTWIDTHS`, `WIDTHS`, `NORAW-GREP`, `NODEFAULT-UI`,
        `NOLIT-CHANGE`, `MDSEAM` run in a copy with the named file deleted → each reports
        `<file> missing`, never a clean tree.
      - `NODEFAULT-UI` with `TYPES=""` **and** with `TYPES=" "` → both report
        `TYPES is empty - the loop would run zero times`. Both matter: the `:-` form of the
        default silently rewrites an empty `TYPES` back to the default (so the guard would be
        unreachable), and `[ -n ]` alone accepts a single space.
      - `NODEFAULT-UI` with no argument at all on the **current** tree → the `Detail` positive
        control fires (`app.rs has no 'struct Detail {'`). This is both the proof that the
        edited default is read and the pre-change baseline for the type group 1 adds.
      - `NODEFAULT-UI` with `TYPES=Dashboard` in a copy whose `pub struct Dashboard` is
        renamed `pub struct DashboardState` → the positive control **fires** rather than
        passing on the substring, which is why both `struct $T` greps are anchored with a
        following `{`.
      - `OPENSPEC-UNTOUCHED` with `BASE=` empty and with `BASE=deadbeef` → each aborts.
      - `WIDTHS` with `WIDTHS_MIN=99` → `found 47 … expected >= 99`.
      - `MDWIDTHS` with `MD_MIN=99` in the copy with an empty `src/ui/markdown.rs` → `found 0
        … expected >= 99`, proving the floor is read; and with no argument at all, confirm the
        reported floor is this change's 23 rather than a copied 17.
      - `DEPS` with `WORK` unset → `WORK (a scratch directory) must be set`; and with
        `WORK=$WORK/not-a-dir` → `is not a directory`.
      - `GRAPH-SNAP` in a copy whose `tests/fixtures/build-graph.txt` is deleted →
        `GRAPH-SNAP FAIL: tests/fixtures/build-graph.txt missing`.
      Record each message verbatim.
      **Red when:** any guard reports success. A guard that cannot fail is worth nothing, and
      this repository has shipped three of them.

- [x] 0.4 CHECK: Confirm the intermediate-gate substitution stated at the top of this file is
      real: it will be exercised from group 2 onward, so verify **now**, on a green tree, that
      `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` reports a TOTAL rather than
      producing no report, and record the percentage. Then confirm the claim that matters:
      `cargo test --all-features` currently reports **zero** failures, so any failure seen in
      groups 2–8 is either the one known acceptance test or a regression.
      **Red when:** it produces no report, in which case every intermediate gate in groups 3–8
      must drop its coverage step and say so here instead of silently skipping it.

- [x] 0.5 VERIFY: Commit the plan-time baseline record (this file's checkbox state only; no
      source change yet). Conventional Commits:
      `docs(markdown-viewer): record measured check baselines`.

---

## 1. `Detail` and the test double — the scaffolding the acceptance test needs
<!-- kind: operational -->

This group is **operational, not behavioural, and deliberately so**: it adds one struct, one
`Dashboard` field, and one test helper — plumbing, not behaviour. A manufactured RED for a
type or a test double would be the schema's own "tests demanded for plumbing" anti-pattern.
Its evidence is the compiler and the construction-site sweep. Every behaviour the new field
carries is group 6's.

- [x] 1.1 CHECK: Complete the `Dashboard` construction-site sweep **before editing anything**.
      Run `grep -rn 'Dashboard[[:space:]]*{' src/` and classify every hit as a literal, a
      destructuring pattern, or a signature/declaration. Record the list. The planning-time
      measurement is **28 hits, 14 of them real sites**, across six files:
      - `src/lib.rs` — 1, `testutil::tests::empty_dashboard`.
      - `src/ui/mod.rs` — 2, `load`'s `Found` and `NotFound` arms. **Both** must be edited;
        the `NotFound` arm is the no-repository path and is easy to miss.
      - `src/ui/list.rs` — **4**, its row-grammar test fixtures. This file is otherwise
        untouched by this change and is named in design.md → Boundaries for exactly this
        reason.
      - `src/ui/app.rs` — 3: the `mod tests` `dashboard_at` helper, a second test helper, and
        the `let Dashboard { … }` **pattern** in
        `dashboard_destructures_into_exactly_seven_fields`, which a "literal" sweep by eye
        loses.
      - `src/ui/view.rs` — 3, its `mod tests` helpers.
      - `src/ui/driver.rs` — 1, its `mod tests` `dashboard` helper.
      Also record `crate::ui::driver::tests`' visibility: `mod tests` there carries **no**
      visibility modifier, so `Script` is unreachable from `crate::ui::tests::detail`, a
      sibling module. Marking the item `pub(crate)` does not make the path legal.
      **Red when:** the measured hit count or the per-file distribution differs from the
      figures above — stop and record it rather than editing blind, since `..` is forbidden
      and a missed site is a compile error at best and a silent default at worst.

- [x] 1.2 CHANGE: Add `pub struct Detail { pub source: String, pub scroll: usize }` to
      `src/ui/app.rs`, deriving `Debug, Clone, PartialEq, Eq` and **not** `Default`, and add
      `detail: Detail` as `Dashboard`'s eighth field. **Nothing reads the field**: no render,
      no action, no clamp. Update all fourteen sites from 1.1 in this same commit, each taking
      `Detail { source: String::new(), scroll: 0 }`, so every landed assertion keeps its
      meaning and `ui::load` starts empty and unscrolled on **both** of its arms. Rename
      `dashboard_destructures_into_exactly_seven_fields` to `…_eight_fields` and add the
      eighth field to its pattern.

- [x] 1.3 CHANGE: Lift the scripted `EventSource` double from `src/ui/driver.rs`'s test module
      to `crate::testutil` — the `Script` struct with its `new`, `calls`, and `timeouts`
      methods, its `EventSource` impl, and the `press(code, modifiers)` helper — and have
      `src/ui/driver.rs`'s tests import them from there. `testutil` is already
      `#[cfg(test)] pub(crate) mod` and already holds `render_at`, `row_text`, and `cell`, so
      this is a move into the module that exists for exactly this. Do **not** duplicate the
      double: two scripted sources with slightly different exhaustion behaviour is how a loop
      test starts hanging.
      **Red when:** `src/ui/driver.rs` keeps a second copy, or `Script` is left in a private
      module and only its items marked `pub(crate)` — that does not compile from a sibling
      module, and 1.1 recorded why.

- [x] 1.4 VERIFY: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --
      -D warnings` (0 warnings — the proof that the fourteen-site sweep found every site),
      `cargo test --all-features` (**green**: this group adds no test and breaks none), and
      `cargo llvm-cov --fail-under-lines 80`. Then `sh $CHECKS/NODEFAULT-UI.sh` with **no
      argument** — its edited default `TYPES="Dashboard Filter Detail"` must now pass, having
      failed at 0.3 by construction.
      **Red when:** `cargo test` is not green. Group 1 is the last boundary before the known
      red test, so a failure here is unambiguously a regression.
      Commit: `refactor(markdown-viewer): add Detail state and lift the scripted event double`.

---

## 2. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [x] 2.1 RED: Add
      `ui::tests::detail::a_markdown_document_renders_and_scrolls_through_the_loop` in
      `src/ui/mod.rs`'s `mod tests` → a new `mod detail`, for detail-scroll → "The document
      fills the detail interior at both mandated widths" composed with → "At the detail route
      the content scrolls by one line at both widths".
      No new harness is needed: `crate::testutil` now holds `Script`, `press`, `render_at`,
      `row_text`, and `cell` — exactly what design.md → Test Boundaries names, with the
      rendering surface **replaced** by `TestBackend`, the event stream **replaced** by the
      scripted double, and the terminal and the filesystem **never reached**. The test lives
      in `src/ui/mod.rs`, **not** in `src/ui/view.rs`: it is a composition test over
      `run_loop`, and `NOIO-VIEW` must keep proving that `view.rs` and `markdown.rs` name no
      filesystem API.
      Build a `Dashboard` with `repo: Some(PathBuf::from("/tmp/demo-repo"))`,
      `searched_from` the same path, `changes: changes::empty_set()`, `route: Route::Detail`,
      `quit: false`, `selected: 0`, an empty inactive `Filter`, and a `detail.source` built in
      a loop as the twenty bullet items `- line-00` … `- line-19`
      (`(0..20).map(|i| format!("- line-{i:02}\n")).collect()`), with `detail.scroll: 0`.
      Drive `driver::run_loop` over a `TestBackend` of 120x20 with a script of **two** Presses
      of `Char('j')` then a Press of `Char('q')`, and again over a `TestBackend` of 60x20 with
      the same script. Assert, at **both** widths, on the final buffer:
      - 120x20, row 2, columns 41..=49: `- line-02`
      - 120x20, row 17, columns 41..=49: `- line-17`
      - 60x20, row 2, columns 1..=9: `- line-02`
      - 60x20, row 17, columns 1..=9: `- line-17`
      - and, at both widths, `dashboard.detail.scroll == 2` and `summary.frames == 3`.
      Read columns by **character index**, never byte offset — the box-drawing borders are
      multi-byte. Derive the two interiors in the test from `layout::split_frame` +
      `split_body` + `Block::bordered().inner` rather than hardcoding them, so the test fails
      loudly if the breakpoint moves; at 120 the detail interior is `Rect::new(41, 2, 78, 16)`
      and at 60, in the detail route, `Rect::new(1, 2, 58, 16)`.
      Use only API that exists after group 1, so the test **compiles**.

- [x] 2.2 CHECK: Confirm it fails because the behaviour is missing, not because the harness is
      misconfigured. Run `cargo test --all-features --lib
      ui::tests::detail::a_markdown_document_renders_and_scrolls_through_the_loop` and record
      the message verbatim. **Expected today:** the assertion on the 120-column row 2 fails,
      comparing `- line-02` against nine spaces — the detail interior is blank because
      `responsive-layout` asserts it is and nothing draws there yet.
      **Red when:** the first row assertion fails against an all-spaces actual.
      **Wrongly green when:** the filter matched nothing — guarded by confirming the run
      reports exactly one test ran under the filter, not `0 passed; 0 failed`.
      **Also wrongly green when:** the fixture's source string is malformed, so re-read the
      recorded actual and confirm it is spaces rather than a *different* row.

- [x] 2.3 VERIFY: the four intermediate-gate commands. `cargo test --all-features` must fail
      on **exactly one** test — the new acceptance test — and `cargo clippy --all-targets`
      must be clean.
      Commit: `test(markdown-viewer): add the detail acceptance test, red`.

---

## 3. The `pulldown-cmark` dependency
<!-- kind: operational -->

- [x] 3.1 CHECK: Record the evidence this group changes, before changing it. Run
      `WORK=$WORK DEPS_SKIP_LEG5=1 sh $CHECKS/DEPS.sh` and record leg 2a's failure verbatim —
      `normal deps are ['ratatui', 'serde_json', 'toml', 'yaml-rust2'], expected
      ['pulldown-cmark', 'ratatui', 'serde_json', 'toml', 'yaml-rust2']`. Run
      `sh $CHECKS/GRAPH-SNAP.sh` and record that it **passes** on the pre-dependency snapshot.
      Then confirm the version this plan pins is still current:
      `cargo info pulldown-cmark` → at planning time `0.13.4`, `rust-version: 1.71.1`, default
      features `getopts` and `html`.
      **Red when:** `cargo info` reports a version later than 0.13.4 — in which case use it,
      re-run the MSRV leg below, and record the difference here rather than silently pinning
      the planned one.

- [x] 3.2 CHANGE: Add to `Cargo.toml`'s `[dependencies]`, keeping the file's existing
      one-line-per-crate style:
      `pulldown-cmark = { version = "0.13.4", default-features = false, features = [] }`.
      Do **not** raise `rust-version`: 1.71.1 is far below the crate's 1.88 floor, which
      `ratatui` sets. Then, **as the very next cargo command**, run `cargo build --locked` and
      record that it **fails** — the lock does not yet carry the new package. The ordering is
      load-bearing: any intervening `cargo tree` or `cargo metadata` rewrites `Cargo.lock`
      during resolution and the failure would not be observable. Then `cargo build` (which
      rewrites the lock), then `cargo build --locked` again, which must now succeed. Record
      all three.
      Then confirm the resolution matches what was measured at planning time: `cargo tree -e
      normal --prefix none` gains **exactly two** packages, `pulldown-cmark v0.13.4` and
      `unicase v2.9.0`, and no proc-macro. (`bitflags` and `memchr`, which `pulldown-cmark`
      also needs, are already in the graph at exactly the resolved versions.)
      **Red when:** a third package appears, or a proc-macro does — either means the feature
      list is not what this change argued, and the design's dependency decision must be
      re-opened rather than the check relaxed.

- [x] 3.3 CHANGE: Regenerate the build-graph snapshot with the extracted script —
      `GRAPH_WRITE=1 sh $CHECKS/GRAPH-SNAP.sh` — never by hand-editing
      `tests/fixtures/build-graph.txt`.

- [x] 3.4 CHECK: Review the snapshot's diff before committing it:
      `git diff --numstat -- tests/fixtures/build-graph.txt` must read exactly `8 0`, and
      `git diff -- tests/fixtures/build-graph.txt` must show `+pulldown-cmark v0.13.4` and
      `+unicase v2.9.0` under each of the four `## <triple>` headings and nothing else.
      **Red when:** any line is removed, or a package other than those two is added, or the
      four sections do not each gain both. A wholesale rewrite of the file is the failure mode
      this task exists to catch: it would mean the generator ran against a different toolchain
      or a different feature resolution.

- [x] 3.5 VERIFY: `sh $CHECKS/GRAPH-SNAP.sh` — passes against the regenerated snapshot, with
      the proc-macro set still exactly the eight on the allowlist and **four** named absences
      now holding (`encoding_rs`, `time`, `getopts`, `pulldown-cmark-escape`).
      Then `WORK=$WORK DEPS_SKIP_LEG5=1 sh $CHECKS/DEPS.sh` — legs 1a, 1b, 2a (five normal
      deps), 2b (both crates spell out `features = []`), 2a-bis, 2c, 2d, and 4 all pass. Leg 4
      must report the floor as `1.88` and **print** the at-the-floor set; record it verbatim
      rather than asserting a fixed list, and confirm only that `pulldown-cmark` is **not** in
      it and nothing is above the floor. At planning time that set was `ratatui`,
      `ratatui-core`, `ratatui-crossterm`, `ratatui-widgets`, `darling`, `darling_core`,
      `darling_macro`, `instability`, and `herdr-openspec` itself.
      **Red when:** leg 4 reports a package above the floor, which would mean the MSRV claim
      in `plugin-build` is false and the dependency decision must be revisited.
      **Wrongly green when:** `GRAPH-SNAP` is run before 3.3's regeneration, where it would
      fail, or after it without reading 3.4's diff, where a wholesale rewrite would pass.

- [x] 3.6 CHECK (contract gate): Re-read `SPEC.md` → Overview → Stack and
      `openspec/specs/plugin-build/spec.md`'s dependency table, and confirm the delta in
      `specs/plugin-build/spec.md` matches what was actually resolved: five normal
      dependencies, `pulldown-cmark` at `0.13.4` with no features and defaults off, two added
      packages, no proc-macro, floor unchanged at 1.88, and the at-the-floor set reported
      rather than asserted.
      **Red when:** the resolved reality differs from the delta spec in any of those six
      particulars — fix the spec here, not at review time.

- [x] 3.7 VERIFY: the four intermediate-gate commands; `cargo test --all-features` still fails
      on exactly the one known acceptance test, with the identical message.
      Commit: `build(markdown-viewer): add pulldown-cmark 0.13.4 and regenerate the graph`.

---

## 4. `ui::markdown` — the fold, paragraphs, headings, and inline faces
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests in a new `src/ui/markdown.rs`'s `mod tests`. **Every one
      renders at both 58 and 78 and writes both widths unsuffixed**, because `MDWIDTHS` reads
      them and fails closed on `58u16`:
      - `paragraph_wraps_at_58_and_78` — the sixteen-word paragraph `alpha bravo charlie delta
        echo foxtrot golf hotel india juliett kilo lima mike november oscar papa`; at 58
        exactly two lines, `alpha bravo charlie delta echo foxtrot golf hotel india` and
        `juliett kilo lima mike november oscar papa`; at 78 exactly two lines, `alpha bravo
        charlie delta echo foxtrot golf hotel india juliett kilo lima mike` — **78 characters
        exactly**, asserted as a length as well as a string, because the inclusive boundary is
        the thing most likely to be off by one — and `november oscar papa`. Every segment
        `Face::plain()`.
      - `empty_source_and_zero_width_produce_no_lines` — `lines("", 58)`, `lines("", 78)`,
        `lines("# Proposal\n", 0)`, `lines("", 0)` all empty.
      - `no_line_exceeds_the_width_it_was_given` — the composite fixture document (heading,
        paragraph, bullet list, nested list, fenced code, block quote, thematic break, link)
        at widths 0, 1, 2, 3, 10, 58, 78, 200; every line's `text().chars().count()` at most
        the width; and at 58 and 78 at least one line of each block kind present, so the
        assertion is not made against an empty result.
      - `lines_is_total_over_arbitrary_input` — a lone `#`, an unclosed ```` ```sh ````, an
        unterminated `[link](`, 500 `x` characters with no space, only newlines, only spaces,
        `**bold` unterminated, and a source holding `\0` and a combining accent; each at 58
        and 78; nothing panics, every line respects the width, and the 500-character token is
        hard-split.
      - `the_six_heading_levels_carry_their_markers` — `# One` … `###### Six` at 58 and 78;
        the six non-blank `text()` values exact; `heading: Some(1..=6)` on every segment;
        `strong`, `emphasis`, `code`, `link`, `quoted` all false.
      - `a_long_heading_wraps_under_its_text_column` — `## alpha bravo … november` at 58 and
        78; first line begins `## alpha`; every continuation line begins with exactly three
        spaces then a non-space; `heading: Some(2)` on all of them.
      - `inline_constructs_become_faced_segments` — ``Why **this** change *really* needs
        `Options::empty()` — see [the design](design.md).`` at 58 and 78; a `this` segment
        with only `strong`; a `really` segment with only `emphasis`; an `Options::empty()`
        segment with only `code`; a `the design` segment with only `link`; **no** segment
        containing `design.md`; and the concatenated text containing the full sentence without
        the destination.
      - `nested_emphasis_composes` — `***both*** and [**bold link**](x)` at 58 and 78; `both`
        carries `strong` **and** `emphasis`; `bold link` carries `link` **and** `strong`.
      - `an_image_renders_its_alt_text` — `Before ![a diagram](x.png) and ![](y.png) after` at
        58 and 78; an `a diagram` segment with `link` true; an `[img]` segment with `link`
        true; no segment containing `x.png` or `y.png`. This is the scenario that keeps an
        alt-less image from vanishing silently.
      - `a_faced_run_split_across_a_wrap_keeps_its_face` — the whole sixteen-word paragraph
        wrapped in `**` at 58 and 78; two lines at each width, identical to
        `paragraph_wraps_at_58_and_78`'s strings, every segment of **both** lines `strong`.
      - `a_soft_break_starts_a_new_line` — `first line\nsecond line` at 58 and 78 → two lines;
        **no** line equal to `first line second line`.
      - `exactly_one_blank_line_separates_blocks` — the composite document with two blank
        source lines in the middle, at 58 and 78; first line the heading and last line
        non-blank; no two consecutive blank lines; exactly one blank between each adjacent
        pair of blocks.
      Covers markdown-render's first two requirements, its inline requirement, and the
      soft-break/blank-line requirement.
      **Red when:** the crate does not compile because `ui::markdown` does not exist. That is
      an honest RED for a typed language; confirm the compiler names the module and not a typo
      in the test module. This group's RED is a compile failure and group 2's is not — the
      acceptance test is the one that must stay assertion-shaped, because it is the one that
      is red across seven groups.

- [ ] 4.2 GREEN: Create `src/ui/markdown.rs` and register it in `src/ui/mod.rs`. Implement
      `Face` (deriving `Debug, Clone, Copy, PartialEq, Eq, Default`, with `Face::plain()`
      returning the default — design.md → Decisions records why `Face` is the one type in `ui`
      that may derive `Default`), `Segment`, `Line`, `Line::text()`, and
      `lines(source, width)`.
      Structure the implementation as **fold then lay out**: walk
      `pulldown_cmark::Parser::new_ext(source, Options::empty())` into a `Vec` of blocks
      carrying their own indent, marker, and inline runs; then lay each block out at the
      width. The two halves are separately readable, and the width appears only in the second,
      which is what makes `MDWIDTHS`' no-exemption rule satisfiable.
      **The module's doc comment must not contain the word `ratatui`, and no other module may
      name `pulldown_cmark` even in prose** — `MDSEAM` is a whole-file grep and design.md
      records that limit as a deliberate cost.

- [ ] 4.3 GREEN: Implement the wrapping engine as one function with two break modes — break at
      spaces (prose) and break anywhere (code and raw HTML) — taking the width, a first-line
      indent, and a continuation indent. A token longer than the available width is hard-split
      at exactly that many characters under **both** modes, which is what keeps a
      500-character URL from producing an over-wide line. A block's content width is
      `width.saturating_sub(prefix)`, and when that is zero the block emits its prefix
      truncated to the width and no content — the branch widths 1, 2, and 3 in 4.1's totality
      test exercise. Count characters with `chars().count()`, matching `ui::list`; the
      wide-character limitation is stated in design.md → Risks.

- [ ] 4.4 VERIFY: `. $CHECKS/TESTCOUNT.sh` then
      `testcount --lib 'ui::markdown::tests::' 12` — the twelve tests this group adds, run and
      counted, so a renamed module cannot pass as a green run. Then `sh $CHECKS/MDSEAM.sh`
      (default `MIN=16`; the tree has 17) and `MD_MIN=12 sh $CHECKS/MDWIDTHS.sh` — both must
      now **pass**, having failed at task 0.2 by construction, which is evidence the file was
      added rather than evidence the floors are decorative.
      **Red when:** either still fails, or the count is below 12.

- [ ] 4.5 VERIFY: the four intermediate-gate commands; still exactly one known failure.
      Commit: `feat(markdown-viewer): render paragraphs, headings, and inline faces`.

---

## 5. `ui::markdown` — lists, code, quotes, rules, and unmodelled constructs
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing tests in `src/ui/markdown.rs`'s `mod tests`, each at 58 and 78:
      - `bullet_items_carry_their_marker_and_hanging_indent` — the sixteen-word item; at 58
        the first line is `- alpha bravo charlie delta echo foxtrot golf hotel india`
        (57 characters, asserted as a length too); at 78 it is `- alpha bravo charlie delta
        echo foxtrot golf hotel india juliett kilo lima` (75 characters); continuation lines
        begin with exactly two spaces then a non-space; every segment `Face::plain()`.
      - `an_ordered_list_numbers_from_its_start_value` — `7. seven` / `8. eight` / `9. nine`
        render as those three strings; and `1. one` / `1. one again` renders as `1. one` and
        `2. one again`, so the numbering is sequential from the start value rather than a copy
        of the source digits.
      - `a_nested_list_indents_two_columns_per_level` — `- outer`, `  - inner`,
        `    - deepest`; and a wrapped `- inner` item's continuation lines begin with exactly
        four spaces.
      - `a_fenced_code_block_is_verbatim` — ```` ```sh ```` / `cargo test --all-features` /
        `  indented` / ```` ``` ````; exactly two code lines, `cargo test --all-features` and
        `  indented` with its leading spaces; `code` true and every other flag false; no line
        containing ```` ``` ```` or `sh`.
      - `a_long_code_line_is_hard_split` — 130 `x` characters in a fence; at 58 three lines of
        58, 58, 14; at 78 two lines of 78 and 52; concatenation equals the original.
      - `an_indented_code_block_matches_the_fenced_form` — the four-space-indented form
        produces the same `text()` values as the fenced form for the same code, with `code`
        true.
      - `a_raw_html_block_renders_verbatim` — `<details><summary>Notes</summary>` present as a
        line with `code` true; the concatenation contains `<details>`.
      - `a_block_quote_prefixes_every_line` — the sixteen-word quote; every line begins `> `
        with `quoted` true on every segment; at 58 the first line is
        `> alpha bravo charlie delta echo foxtrot golf hotel india` (57 characters); a nested
        quote's lines begin `> > `.
      - `a_thematic_break_fills_the_width` — `---` between two paragraphs; 58 `-` characters
        at 58 and 78 at 78; `Face::plain()`; one blank line above and below.
      - `a_table_renders_as_literal_source_rows` — `| Gate | Command |`, `|---|---|`,
        `| Format | cargo fmt |` render as those three strings, one per line, every segment
        `Face::plain()`.
      - `line_text_concatenates_its_segments` — a mixed-face line at 58 and 78, where `text()`
        equals the segments' `text` joined in order and equals the same string at both widths
        for a line short enough not to wrap. This is the test that pins `Line::text()`, which
        every other test's assertions run through.
      **Red when:** each assertion fails against the group-4 implementation, which renders
      these blocks as plain paragraph text or not at all. Record the first failure per test
      verbatim, and confirm it is a **wrong string** rather than a panic: a panic here would
      mean group 4's fold is not total and the fix belongs in group 4.

- [ ] 5.2 GREEN: Extend the fold with `Tag::List`, `Tag::Item`, `Tag::CodeBlock` (both
      `Fenced` and `Indented`), `Tag::BlockQuote`, `Event::Rule`, `Event::Html`, and
      `Event::InlineHtml`, with the markers, indents, and faces the specs state. Keep the
      default arm of the event match rendering an event's own text rather than dropping it,
      which is what `lines_is_total_over_arbitrary_input` holds.

- [ ] 5.3 CHECK: Confirm the fold handles every `pulldown_cmark::Event` variant explicitly or
      through a stated default, by reading the match arms against the crate's `Event` enum.
      Record which variants reach the default arm and what it does with each.
      **Red when:** a variant is silently discarded — a discarded `Event::Text` is content
      vanishing from a reader's pane, which is the failure this whole change exists to avoid.

- [ ] 5.4 VERIFY: `testcount --lib 'ui::markdown::tests::' 23`, `sh $CHECKS/MDWIDTHS.sh` with
      **no argument** (its default floor is this change's 23), and `sh $CHECKS/MDSEAM.sh`.
      **Red when:** the count is below 23, or any markdown test fails to name both widths.

- [ ] 5.5 VERIFY: the four intermediate-gate commands; still exactly one known failure.
      Commit: `feat(markdown-viewer): render lists, code blocks, quotes, and rules`.

---

## 6. `ui::app` and `layout::interior` — route-dependent navigation and scroll state
<!-- kind: behavior -->

`layout::interior` lands **here**, not in group 7, because `normalise_scroll` is its first
caller and shipping it a group early with its test a group late would be untested saturating
arithmetic and a green-on-arrival RED in group 7 — the defect class group 12 hunts for.

- [ ] 6.1 RED: Write a failing test in `src/ui/layout.rs`'s `mod tests`:
      - `interior_agrees_with_a_bordered_block` — `interior` and
        `ratatui::widgets::Block::bordered().inner` over `Rect::new(0,1,40,18)`,
        `Rect::new(40,1,80,18)`, `Rect::new(0,1,60,18)`, `Rect::new(0,0,2,2)`,
        `Rect::new(0,0,1,1)`, `Rect::new(0,0,0,0)`; equal on every one, compared as **whole
        `Rect` values**, all four fields. The degenerate cases are the point: measured against
        ratatui 0.30.2, `Rect::new(0,0,1,1)` gives `Rect { x: 1, y: 1, width: 0, height: 0 }`
        and `Rect::new(0,0,0,0)` gives `Rect { x: 0, y: 0, width: 0, height: 0 }`, because
        `inner` clamps the advanced origin to the rectangle's own right and bottom edges. A
        hand-rolled interior that only saturates the width and height passes a width-and-height
        comparison and fails this one.
      **Red when:** the crate does not compile because `layout::interior` does not exist.

- [ ] 6.2 GREEN: Implement `layout::interior(area: Rect) -> Rect` with the clamp 6.1 pins, and
      replace `src/ui/view.rs`'s single `Block::bordered().inner(area)` call with it — a
      behaviour-preserving refactor, pinned by 6.1.

- [ ] 6.3 RED: Write failing tests in `src/ui/app.rs`'s `mod tests` → `mod keys`:
      - `detail_destructures_into_exactly_two_fields` — an exhaustive `let Detail { source,
        scroll }` with no `..`.
      - `next_and_prev_scroll_at_the_detail_route` — a `Dashboard` at `Route::Detail` with the
        twenty-item source and `detail.scroll` 0, given two `Next` actions → `detail.scroll` 1
        then 2, `selected` unchanged.
      - `next_and_prev_select_at_the_list_route` — three active changes, a non-empty
        `detail.source`, `Route::List`, two `Next` → `selected` 2 and `detail.scroll` still 0.
      - `scroll_stops_at_the_top` — four `Prev` at `detail.scroll` 0 → 0 each time, no panic.
      - `filter_mode_types_j_and_k_while_arrows_scroll` — `action_for` under `filtering` true
        for `Char('j')`, `Char('k')`, `Down`, `Up` → `FilterPush('j')`, `FilterPush('k')`,
        `Next`, `Prev`; applying all four at `Route::Detail` with an active filter leaves
        `filter.query` as `jk` and `detail.scroll` 1 then 0.
      - `every_route_move_resets_the_scroll` — from `Route::Detail` with `scroll` 3: a `Back` →
        `scroll` 0 and `route` `List`; then an `OpenDetail` → `scroll` still 0; a second
        dashboard given `FilterStart` instead → `route` `List` and `scroll` 0; a third at
        `Route::Detail` with `filter.active` true and `scroll` 3 given a `Back` — which
        dismisses the filter layer, not the route — → `scroll` still 3.
      - `normalise_scroll_clamps_against_the_frame` — the twenty-item source, `scroll` 99,
        `normalise_scroll(Rect::new(0, 0, 120, 20))` → 4; and at `Route::Detail` with
        `Rect::new(0, 0, 60, 20)` → 4.
      - `normalise_scroll_is_inert_when_the_detail_region_is_not_drawn` — `Route::List` with
        `scroll` 7 and `Rect::new(0, 0, 60, 20)` → still 7; the same dashboard with
        `Rect::new(0, 0, 120, 20)` → 4, in the **same test**, so the early return is a real
        branch rather than the only behaviour.
      And **rewrite** for the rename and the route branch:
      `quit_keys_and_their_near_misses`, `only_press_kind_acts`, `enter_and_esc_map_to_routes`
      (extended: `detail.scroll` is 0 after each), `esc_dismisses_one_layer_at_a_time`
      (extended: `scroll` survives the filter layer and resets on the route layer),
      `navigation_and_filter_keys_are_distinguished`, `non_key_events_are_ignored`,
      `action_for_is_total_over_a_keycode_sweep`,
      `select_next_and_prev_clamp` → `next_and_prev_clamp`,
      `apply_never_leaves_selected_out_of_range`, and
      `dashboard_destructures_into_exactly_eight_fields` (already renamed in 1.2).
      **Red when:** the crate does not compile because `Action::Next`, `Action::Prev`, and
      `Dashboard::normalise_scroll` do not exist. Confirm the compiler names those three items
      and not a typo.

- [ ] 6.4 GREEN: Rename `Action::SelectNext` → `Action::Next` and `Action::SelectPrev` →
      `Action::Prev` throughout the crate, in one commit, so no intermediate state has a
      half-renamed enum. The sites are **measured, not guessed**: `src/ui/app.rs` holds every
      non-test site — the two enum variants, two `apply` arms, and two `action_for` arms — and
      `src/ui/view.rs`'s test module holds five more. `src/ui/driver.rs` names **no** variant:
      `run_loop` passes an opaque `action` to `dashboard.apply(action)`. The compiler finds
      anything this list misses; record any site it finds that this list does not name.

- [ ] 6.5 GREEN: Branch `apply`'s `Next` and `Prev` on `self.route` — move and clamp
      `selected` at `Route::List`, move `detail.scroll` by one saturating at `Route::Detail`,
      never both. Reset `detail.scroll` to 0 on **every** arm that moves the route:
      `OpenDetail` when it sets `Route::Detail`, `Back` when it returns to `Route::List`, and
      `FilterStart`, which also sets `Route::List`. Dismissing a filter layer is **not** a
      route move and leaves `detail.scroll` alone.

- [ ] 6.6 GREEN: Implement `Dashboard::normalise_scroll(&mut self, frame_area: Rect)` using
      `layout::split_frame`, `layout::split_body`, and the `layout::interior` landed in 6.2,
      early-returning when the detail region is `None` or its interior has zero width or zero
      height.
      **Red when:** `app.rs` ends up constructing a `ratatui::widgets::Block` to compute an
      interior — that is the duplication `layout::interior` exists to prevent, and design.md →
      Decisions rejects it by name.

- [ ] 6.7 VERIFY: `testcount --lib 'ui::app::tests::' 30` and
      `testcount --lib 'ui::layout::tests::' 10`, then `sh $CHECKS/NODEFAULT-UI.sh` with no
      argument — it must still pass with `Detail` now carrying a destructuring companion.
      **Red when:** either count is short, or `NODEFAULT-UI` reports a `Default` or an elided
      field for any of the three types.

- [ ] 6.8 VERIFY: the four intermediate-gate commands; still exactly one known failure.
      Commit: `feat(markdown-viewer): scroll the detail route with j, k, and the arrows`.

---

## 7. `layout::scroll_offset` and `ui::view` — the slice and the drawing
<!-- kind: behavior -->

- [ ] 7.1 RED: Write failing tests in `src/ui/layout.rs`'s `mod tests`:
      - `scroll_offset_is_exact_at_its_boundaries` — the nine tabled triples `(0,0,16)→0`,
        `(16,0,16)→0`, `(16,9,16)→0`, `(17,0,16)→0`, `(17,1,16)→1`, `(17,2,16)→1`,
        `(20,4,16)→4`, `(20,99,16)→4`, `(20,4,0)→0`.
      - `the_detail_interior_is_78_at_120_and_58_at_60` — `split_frame` + `split_body` +
        `interior` at `Rect::new(0,0,120,20)` with `Route::Detail` → `Rect::new(41,2,78,16)`;
        at `Rect::new(0,0,60,20)` with `Route::Detail` → `Rect::new(1,2,58,16)`; at
        `Rect::new(0,0,60,20)` with `Route::List` → no detail rectangle at all.
      **Red when:** the crate does not compile because `scroll_offset` does not exist.
      **Wrongly green when:** `the_detail_interior_is_78_at_120_and_58_at_60` passes on
      arrival — it will, because `interior` landed in group 6 and `split_frame` / `split_body`
      landed in `tui-shell`. That is expected and stated: it is a **freeze** of a derived fact,
      not a new behaviour, and its value is that it reddens if the breakpoint or the
      constraints ever move. Do not mistake it for the group's RED; `scroll_offset` is.

- [ ] 7.2 GREEN: Implement `layout::scroll_offset(lines, scroll, height)` — `0` when `height`
      is `0`, otherwise `min(scroll, lines.saturating_sub(height))`.

- [ ] 7.3 RED: Write failing tests in `src/ui/view.rs`'s `mod tests`. **Every one renders at
      both 60 and 120 and writes both widths unsuffixed**, because `WIDTHS` reads them:
      - `the_detail_document_fills_the_interior_at_60_and_120` — the twenty-item source,
        `scroll` 0, and a `changes` holding the single active change `fix-empty-basket` at 7
        of 7; 120x20 with `Route::List` → row 2 columns 41..=49 `- line-00`, row 17
        `- line-15`, and row 2 columns 1..=38 exactly
        `> fix-empty-basket               [7/7]`; 60x20 with `Route::Detail` → row 2 columns
        1..=9 `- line-00`, row 17 `- line-15`.
      - `faces_reach_the_buffer_as_styles` — `# Title` / blank / ``A **bold** and *italic*
        line with `code` and [a link](x).`` / blank / `> quoted`; at both widths the seven
        cells spelling `# Title` report `BOLD` (**not** the whole row — `markdown-render` does
        not pad, so the columns past the heading are never written); the `bold` cells report
        `BOLD`, the `italic` cells `ITALIC`, the `code` cells `DIM`, the `a link` cells
        `UNDERLINED`, and the `> quoted` cells `DIM`; and the `and` cells between them report
        **none** of those, which is the negative control that makes the test discriminate.
        Five positive assertions for a five-clause mapping: a mapping clause with no assertion
        can be deleted with every test still green.
      - `an_empty_detail_source_leaves_the_interior_blank` — at 120x20 every cell of rows
        2–17, columns 41–118, and at 60x20 every cell of rows 2–17, columns 1–58, a space
        whose `Style` equals `Cell::default().style()`.
      - `detail_content_never_overwrites_the_border` — twenty 200-character lines; at 120
        every cell of columns 39, 40, 119 in rows 1–18 box-drawing; at 60 every cell of
        columns 0 and 59; and no interior row blank at either width.
      - `a_degenerate_detail_interior_draws_nothing` — 1x20, 2x20, 3x20, 60x2, 60x3 render
        without panicking, with 60x20 and 120x20 in the same test as contrasting controls that
        **do** show content.
      - `a_scroll_past_the_end_draws_the_last_screenful` — `scroll` 99; at both widths row 2
        `- line-04` and row 17 `- line-19`.
      - `scrolling_moves_the_detail_content` — two `Next` actions applied, then rendered; at
        both widths row 2 `- line-02` and row 17 `- line-17`.
      - `the_list_route_still_moves_the_marker_with_detail_content_present` — three changes, a
        non-empty source, `Route::List`, two `Next`; at both widths the `>` marker on the
        third list row and the detail content unmoved at `- line-00`.
      - `scrolling_stops_at_the_top_on_screen` — four `Prev` at `scroll` 0; at both widths row
        2 still `- line-00`.
      - `route_moves_reset_the_scroll_on_screen` — `scroll` 3, a `Back`, an `OpenDetail`,
        rendered; at both widths row 2 `- line-00`.
      And **extend** two landed tests: `region_interiors_are_blank` — its `Dashboard` now names
      an explicitly empty `detail.source`, and the detail-interior assertion is unchanged,
      which is what proves this change did not quietly start drawing there; and
      `rows_do_not_overwrite_the_borders` — its `Dashboard` gains a thirty-line,
      200-character `detail.source`, so the border assertion covers markdown as well as rows.
      **Red when:** each fails against a detail region that draws nothing.

- [ ] 7.4 GREEN: Implement `render_detail` in `src/ui/view.rs` — early-return on zero width,
      zero height, or an empty source; `markdown::lines(&source, interior.width)`;
      `layout::scroll_offset(lines.len(), dashboard.detail.scroll, interior.height)`; then one
      row per line, segments left to right, each with `style_for(&segment.face)`, stopping at
      the interior's last column. Implement `style_for` as the crate's only `Face`-to-`Style`
      mapping, exactly as `detail-scroll` states it: `heading` present or `strong` → `BOLD`,
      `emphasis` → `ITALIC`, `code` → `DIM`, `link` → `UNDERLINED`, `quoted` → `DIM`, composing
      when flags compose.
      **Red when:** a segment is drawn at a column computed from a byte offset rather than a
      character count — the same multi-byte trap `ui::list` already documents.

- [ ] 7.5 VERIFY: `testcount --lib 'ui::layout::tests::' 12` and
      `testcount --lib 'ui::view::tests::' 57`, then `sh $CHECKS/WIDTHS.sh` with
      `WIDTHS_MIN=57` and `sh $CHECKS/LISTWIDTHS.sh` (default floor 17, unchanged).
      **Red when:** either count is short, or a view test names only one width.

- [ ] 7.6 VERIFY: the four intermediate-gate commands; still exactly one known failure — the
      acceptance test, which is still red because `run_loop` does not yet normalise.
      **Wrongly green when:** the acceptance test starts passing here. It must not: 2.1's
      script scrolls only twice, well within range, so if it passes at 7.6 the normalisation
      is untested and group 8's RED will be vacuous. If it does pass, extend 2.1's script to
      ten `j` presses **before** proceeding, and record the change.
      Commit: `feat(markdown-viewer): draw the markdown document into the detail region`.

---

## 8. `ui::driver` and `ui::load` — the frame normalises the stored offset
<!-- kind: behavior -->

- [ ] 8.1 RED: Write failing tests in `src/ui/driver.rs`'s `mod tests`:
      - `scrolling_past_the_end_is_normalised_on_the_next_frame` — the twenty-item source at
        `Route::Detail`, a script of ten `Char('j')` Presses then `Char('q')`, over a
        `TestBackend` of 120x20 and again of 60x20; the loop ends with `detail.scroll` **4**,
        not 10; the final buffer's row 2 reads `- line-04` and row 17 `- line-19` at both
        widths; and the summary is `frames: 11, polls: 11`, so the normalisation neither added
        nor skipped a draw.
      - `a_resize_renormalises_the_offset` — **not** through `run_loop`, which borrows the
        `Terminal` for its whole run so no scripted source can resize the backend from inside
        it. Drive the pair `terminal.draw(|f| view::render(f, &dashboard))` then
        `dashboard.normalise_scroll(area)` directly — which is exactly one iteration of the
        loop — over one `Terminal<TestBackend>`: at 120x20 starting from `detail.scroll` 4 it
        stays 4; resize the backend to 120x30 and repeat, and it becomes 0, because a 26-row
        interior holds all twenty lines; the second buffer's row 2, columns 41..=49, reads
        `- line-00`.
      And in `src/ui/mod.rs`'s `mod tests` → `mod load`:
      - `startup_leaves_the_detail_empty_and_unscrolled` — `ui::load` over a `ScratchDir`
        repository with one change, **and** over a starting path with no `openspec/` above it
        (the `NotFound` arm, a second `Dashboard` construction site); `detail.source` empty and
        `detail.scroll` 0 in both; and rendering both at 120x20 and 60x20 leaves the detail
        interior blank, so the production pane is unchanged by this change.
      **Red when:** `scrolling_past_the_end_is_normalised_on_the_next_frame` reports
      `detail.scroll` 10 rather than 4 — the stored offset ran away because nothing normalises
      it yet. Record the number verbatim; a 4 here would mean the clamp is happening somewhere
      it should not.

- [ ] 8.2 GREEN: In `run_loop`, capture the draw's `CompletedFrame::area` and call
      `dashboard.normalise_scroll(area)` once per iteration, after the draw and before the
      poll. `CompletedFrame` borrows the terminal, so copy the `Rect` out — it is `Copy` —
      before the borrow ends. `render` keeps its `&Dashboard` signature: the view clamps for
      display and never writes the stored value, which is the render seam this change must not
      dissolve.
      **Red when:** the implementation reaches for `&mut Dashboard` in `render` — design.md →
      Decisions rejects that by name, and `NOIO-VIEW` would not catch it.

- [ ] 8.3 VERIFY: `testcount --lib 'ui::driver::tests::' 10` and
      `testcount --lib 'ui::tests::load::' 6`.

- [ ] 8.4 VERIFY: the four intermediate-gate commands; still exactly one known failure.
      Commit: `feat(markdown-viewer): normalise the stored scroll against the drawn frame`.

---

## 9. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 9.1 VERIFY: `. $CHECKS/TESTCOUNT.sh` then
      `testcount --lib 'ui::tests::detail::' 1` — one test, green, at both widths, with no
      change to the test since 2.1. Record the run. The counted form is used rather than a
      bare filtered `cargo test`, which exits 0 when a filter matches nothing.
      **Red when:** the test needed editing to pass. An acceptance test edited to match the
      implementation is not an acceptance test; if an expectation was genuinely wrong, record
      the correction and its reason here rather than changing it silently.

- [ ] 9.2 VERIFY: `make check` — the literal, unqualified gate, for the first time since
      group 1. All four sub-commands in order. If it fails, name the failing sub-command
      rather than reporting a summary.

- [ ] 9.3 VERIFY: `testcount --lib '' 564` — the measured 516 baseline plus this change's 48
      library tests. Record the final lib total and the whole-suite total.
      **Red when:** the total is below 564, which would mean a group's tests were never
      written or a module was renamed out from under a filter.
      Commit: `test(markdown-viewer): the detail acceptance test is green`.

---

## 10. Architectural and dependency checks
<!-- kind: operational -->

- [ ] 10.1 CHECK: Run every check against the finished tree and record each output verbatim:
      `NOSPAWN-GREP` with `MIN=17`, `NOIO-VIEW`, `NOCLI-SHELL` with `UI_MIN=9`, `NORAW-GREP`,
      `NODEFAULT-UI` (no argument — the edited default `Dashboard Filter Detail`),
      `NOLIT-CHANGE` with `MIN=17`, `MDSEAM` with `MIN=17`, `WIDTHS` with `WIDTHS_MIN=57`,
      `LISTWIDTHS` (default 17), `MDWIDTHS` (default 23), `NOWAIVER`, `GATE-MECH1`,
      `NOJSON-SEAM`, `DEPS` (full, **without** `DEPS_SKIP_LEG5` — this is the one run that
      exercises leg 5's five removal experiments and the undeclared-crate guard), and
      `GRAPH-SNAP`.
      **Red when:** any fails. `NOSPAWN-GREP` at `MIN=17`, `NOLIT-CHANGE` at `MIN=17`,
      `NOCLI-SHELL` at `UI_MIN=9`, `MDSEAM`, `MDWIDTHS`, and `NOIO-VIEW` all **failed** at
      task 0.2 or 0.3 by construction, so their passing here is evidence the file was added
      rather than evidence the floors are decorative.
      **Wrongly green when:** `DEPS` is run with `DEPS_SKIP_LEG5=1` — leg 5 is the only place
      the new dependency's necessity is proven, and skipping it here would leave the
      `plugin-build` scenario unverified.

- [ ] 10.2 CHECK: Prove each check can still go **red** against a planted violation, in a
      throwaway copy under `$WORK`, then remove it. Extract and run the same `$CHECKS`
      scripts, never a retyped variant:
      1. `let _ = std::process::Command::new("ls");` in `src/ui/markdown.rs` → `NOSPAWN-GREP`
         and `NOIO-VIEW` both fail, naming the file and line.
      2. `use std::fs;` in `src/ui/markdown.rs` → `NOIO-VIEW` fails.
      3. `let _ = crate::changes::from_cli;` in `src/ui/markdown.rs` → `NOCLI-SHELL` fails.
      4. `use pulldown_cmark::Parser;` in `src/ui/view.rs` → `MDSEAM` fails, naming the file
         and line. This is the control that proves the confinement is real and not a
         convention nobody re-checks.
      5. `use ratatui::style::Modifier;` in `src/ui/markdown.rs` → `MDSEAM`'s second leg
         fails.
      6. `let _ = pulldown_cmark::html::push_html;` in `src/ui/markdown.rs` → `MDSEAM`'s third
         leg fails.
      7. `impl Default for Detail { … }` in `src/ui/app.rs` → `NODEFAULT-UI` fails naming
         `Detail`, proving the edited type list is read rather than only the old two.
      8. `let Detail { source, .. } = d;` in `src/ui/app.rs` → `NODEFAULT-UI` half B fails for
         `Detail`.
      9. `let d = Dashboard { quit: false, .. };`-shaped elision → half B fails for
         `Dashboard` too.
      10. A `ChangeSet { active: vec![], archived: vec![], problems: vec![] }` literal in
          `src/ui/markdown.rs` → `NOLIT-CHANGE` fails, naming the file and line.
      11. A new view test in `src/ui/view.rs` naming only one width, written across **three
          lines** with `#[test]` alone on its own line:
          ```
              #[test]
              fn only_at_60() { let _ = render_at(60, 20, &d); }
          ```
          → `WIDTHS` fails naming `only_at_60`. The line break is load-bearing: all three
          width checks split on a **line-anchored** `^[ \t]*#\[test\][ \t]*$`, so the same
          plant written as one line creates no part and the check passes.
      12. The same shape in `src/ui/markdown.rs`, `#[test]` again on its own line:
          ```
              #[test]
              fn only_at_58() { let _ = lines("x", 58); }
          ```
          → `MDWIDTHS` fails naming `only_at_58`.
      13. The same shape in `src/ui/list.rs` naming only 38 → `LISTWIDTHS` fails.
      14. `--fail-under-lines 70` in `Makefile` → `NOWAIVER` fails.
      15. `enable_raw_mode()` in `src/ui/view.rs` → `NORAW-GREP` fails.
      16. **Two plants for `DEPS`'s two manifest legs, because the first exits before the
          second runs.** (a) `features = ["html"]` on the `pulldown-cmark` line → leg 2a fails
          on the feature comparison, and the script stops there. (b) the `features = []` key
          **removed entirely** while `default-features = false` stays → `cargo metadata`
          reports `[]` either way, so leg 2a *passes* and **leg 2b** fails on the manifest
          text, which is the only violation leg 2b exists for. Run both with
          `DEPS_SKIP_LEG5=1`.
      17. **The named-absence control, in two steps, because a stray line appended to
          `$SNAP` only trips `diff -u`.** In the copy, set `features = ["html"]` on
          `pulldown-cmark`, run `cargo build` so the lock resolves
          `pulldown-cmark-escape`, run `GRAPH_WRITE=1 sh $CHECKS/GRAPH-SNAP.sh` so the
          snapshot matches the new graph and the diff leg passes, then run
          `sh $CHECKS/GRAPH-SNAP.sh` → it must fail with
          `pulldown-cmark-escape is in the normal build graph`. Without this two-step, the
          four named absences have no negative control anywhere in the plan.
      Every plant is a **source-text** edit in a throwaway copy that is never compiled —
      items 8, 9, and 10 are not valid Rust without a `Default`, and that is fine: these
      checks are greps, and the property under test is that the grep sees the text. Items 16
      and 17 do run `cargo`, in the copy only.
      Record each failure message verbatim. Delete `$WORK`'s copies afterwards and confirm
      `git status --porcelain` is clean of them.
      **Red when:** any planted violation is **not** caught. A check that stays green against
      its own violation is the defect class this repository has shipped three times.

- [ ] 10.3 CHECK: Confirm no test in the change reaches a collaborator the design's Test
      Boundaries table does not name. Grep `src/ui/markdown.rs` and `src/ui/view.rs` for
      `ScratchDir`, `temp_dir`, `std::fs`, and `Command` — expected: **no hits in either**.
      Grep `src/ui/mod.rs` for `run_loop` — expected: exactly the acceptance test.
      **Red when:** a view or markdown test opens a directory, which is the signal that logic
      leaked into the view.

- [ ] 10.4 VERIFY: `OPENSPEC-UNTOUCHED` against `$BASE`. The only permitted paths under
      `openspec/` are `openspec/changes/markdown-viewer/`.
      **Red when:** any other path appears — including an untracked one, which is the half
      `git diff` alone would miss.
      Commit: `test(markdown-viewer): run the architectural check suite and its negative controls`.

---

## 11. Documentation
<!-- kind: operational -->

- [ ] 11.1 CHECK: Re-read `SPEC.md` → Manifest and the `config.toml` format and confirm
      neither changed. This change touches no manifest key and no configuration key, so the
      documented formats must still match `herdr-plugin.toml` and `src/config.rs` byte for
      byte.
      **Red when:** either drifted, which would mean this change altered a format it claims
      not to.

- [ ] 11.2 CHANGE: Rewrite in `SPEC.md`: **User interface → Detail view** (audience:
      `detail-view`, `tasks-tab`, and every future change that renders into that region).
      Three edits, in place:
      - Replace "Every other tab is a markdown viewer built on `pulldown-cmark`, supporting
        headings, lists, code blocks, emphasis, and links" with the rendering grammar: what
        each construct becomes on screen, that soft breaks are preserved rather than
        re-flowed, that code and raw HTML are hard-split rather than clipped, that a link's
        destination is not printed and an image renders its alt text, and that constructs the
        parser does not model render as their literal text.
      - Annotate each sentence of the section with the change that owns it — the header, the
        tab bar, and the tab-switching keys are `detail-view`'s; the tasks tab is
        `tasks-tab`'s; the markdown viewer and its scrolling are this change's — so
        `markdown-viewer`'s scope is legible from `SPEC.md` alone and `detail-view`'s absence
        is not read as an omission.
      - State the two mandated detail interiors: 78 columns by 16 rows at a 120x20 frame, 58
        by 16 at a 60x20 frame in the detail route, as `list-view` stated 38 and 58 for the
        list.

- [ ] 11.3 CHANGE: Rewrite in `SPEC.md`: **User interface → Responsive layout** (audience: the
      same). The section gives the `Length(40)` / `Min(0)` constraints and never the interiors
      they produce. Add the detail interiors beside the list ones, with the note that 78 is a
      property of the mandated 120-column frame rather than a constant of the layout, since
      the detail column is `Min(0)` and every column gained beyond 100 goes to it.

- [ ] 11.4 CHANGE: Rewrite in `SPEC.md`: **User interface → Keys** (audience: every future
      change and every reader looking up one row). Two edits:
      - The `j` / `k` / arrows row reads "Move the list selection, clamped at both ends rather
        than wrapping; the list scrolls to keep it visible", which this change makes false at
        `Route::Detail`. Add the route split: the list selection at the list route, the detail
        content by one line at the detail route, with the filter-mode layering unchanged. A
        reader consults this table one row at a time, so the qualification goes in the row
        itself.
      - The `1`–`9` / `[` / `]` row is annotated as `detail-view`'s, so a reader of this table
        does not expect tab switching to work in the change that fills the detail region.

- [ ] 11.5 CHANGE: Rewrite in `SPEC.md`: four further sentences this change falsifies or
      leaves incomplete (audience: every future change; each is a one- or two-line edit):
      - **Overview → Stack** — the `pulldown-cmark` entry carries no version and no feature
        decision, while `yaml-rust2`'s carries a pointer to where its choice is argued. Give
        `pulldown-cmark` the same pointer to this change's `design.md`.
      - **Architecture → Module map, the `ui` row** — "Views (the change-row grammar
        included), layout, the dashboard's own state …" gains the markdown rendering, which is
        now the largest single thing in the module.
      - **Testing and quality gates → Unit-tested modules** — the `ui` bullet lists
        `ui::layout`, `ui::app`, `ui::list`, `ui::view`, `ui::driver`, `ui::terminal`, and
        `ui::load`. Add `ui::markdown`, with its own width check.
      - **Testing and quality gates → View tests** — "at both 60 and 120 columns so the
        responsive breakpoint is genuinely covered" is true of frame widths and silent about
        the interior widths two capabilities now assert. Name all three pairs: 60/120 for
        frames, 38/58 for list interiors, 58/78 for detail interiors, so a future change does
        not have to rediscover which pair applies to which module.

- [ ] 11.6 CHANGE: Add to `SPEC.md`: **Degraded states** (audience: `degraded-states`, which
      audits this table row by row in Phase 6). Two edits:
      - One new row: markdown the viewer does not model — tables, footnotes, strikethrough,
        raw HTML — renders as its literal source text, one line per source line, rather than
        being dropped or mangled. It is reachable today, so leaving it off the table would
        make `degraded-states`' "confirm, do not add" principle false.
      - Annotate the existing "Artifact file missing → Tab is still shown and renders 'No
        content yet'" row as `detail-view`'s, since this change deliberately does not
        implement it and an auditor would otherwise score it as missing.

- [ ] 11.7 CHANGE: Rewrite in `AGENTS.md`: **Current repo state** and **Architecture rules**
      (audience: every session). In the first, replace the sentence "the detail region is
      still an empty bordered frame; `markdown-viewer` and `detail-view` fill it next" with
      what the region now does and what still fills it — the viewer and its scrolling are in
      place and `detail-view` supplies the source. In the second, add two durable constraints
      a future change would otherwise rediscover by breaking them: `pulldown_cmark` is named
      only in `src/ui/markdown.rs` and that module names no ratatui type; and the detail
      region's two mandated interiors are 78 and 58 columns, with every `ui::markdown` test
      asserting both. Net: one sentence rewritten, two lines added; nothing appended beside a
      rule it supersedes.

- [ ] 11.8 CHANGE: Record, in this file and in `planning-review.md`, the edits that must be
      made at **archive** time rather than now, because `OPENSPEC-UNTOUCHED` would fail —
      correctly — if they were made here, and `openspec/config.yaml`'s archive guidance covers
      a row that "split, merged, or moved" but not one that gained scope:
      - `openspec/IMPLEMENTATION-ORDER.md`'s `markdown-viewer` row reads "`pulldown-cmark` to
        ratatui text — headings, lists, code blocks, emphasis, links — with scrolling". It
        does not mention the dependency delta on `plugin-build`, which this change carries the
        way `schema-model`'s row records its own; nor the keybinding change; nor block quotes,
        thematic breaks, images, and raw HTML. Its **Spec refs** cell reads "User interface →
        Detail view" and needs "Degraded states" alongside it.
      - The same file's note "**`markdown-viewer` and `list-view` are siblings.** Nothing
        connects them until `detail-view`" is still true, but the open question `list-view`
        deferred — whether `j` / `k` rebind to the detail pane — is **answered here**, one row
        earlier than it predicted. Record that, so `detail-view` does not re-open it.
      - `openspec/specs/plugin-build/spec.md`'s MSRV scenario named four at-the-floor packages
        as though the list were exhaustive; it is nine. This change's delta corrects the
        wording to "reported rather than asserted", and the correction reaches the live spec
        only at archive time.
      Naming these here is what makes the archive step catch them.

- [ ] 11.9 VERIFY: `OPENSPEC-UNTOUCHED` again — 11.2–11.7 edit files outside `openspec/`, so
      the result must be identical to 10.4's.
      **Red when:** a documentation edit landed under `openspec/` outside this change's own
      directory.
      Commit: `docs(markdown-viewer): correct SPEC.md's detail view, keys, and degraded states`.

---

## 12. Change Review
<!-- kind: operational -->

- [ ] 12.1 CHECK: Dispatch an **independent** reviewer — a fresh subagent, never a fork of the
      implementing session — against `proposal.md`, all six spec files, `design.md`,
      `tasks.md`, and the diff. Give it the artifacts and the diff, not this session's
      reasoning. Require it to write its findings incrementally to a file as it goes, not only
      in its final message. Concentration points, in order:
      1. **Tests that cannot fail.** For each of the 68 spec scenarios, name the test that
         would go red if the behaviour were deleted. Re-run 10.2's seventeen planted
         violations independently rather than trusting the recorded output.
      2. **Both widths, genuinely.** Every view test names 60 and 120, every markdown test
         names 58 and 78, every row test names 38 and 58 — but does each *assert* at both, or
         does one width appear only in a comment or an unused literal? Read the assertions.
      3. **The render seam.** No view or markdown test opens a directory; `NOIO-VIEW`'s
         six-file set is the real set; `render` still takes `&Dashboard`; the normalisation
         lives in the loop.
      4. **The dependency.** `Cargo.toml`, `Cargo.lock`, and `tests/fixtures/build-graph.txt`
         agree; the graph diff is exactly eight added lines; `DEPS` and `GRAPH-SNAP` on disk
         carry the edits this file describes — **read the extracted scripts**, since the
         defect this repository has actually shipped is a described-but-unmade edit.
      5. **The `openspec/` write invariant.** `OPENSPEC-UNTOUCHED` was run against a base SHA
         including untracked and ignored files, not against the index.
      6. **Floors that measure.** Every `testcount`, `MIN`, `UI_MIN`, `WIDTHS_MIN`,
         `LIST_MIN`, and `MD_MIN` is *measured baseline + this change's addition*, and each
         was proven to fail before the group that satisfies it. `MDSEAM`'s baseline is 17
         before **and** after, unlike the two `MIN`s that go 16 → 17.
      7. **Drift.** Any contract the implementation changed without refreshing `design.md`,
         the specs, or `SPEC.md` — the `Face` flag set and the `style_for` mapping especially,
         since both are stated in two places, and every clause of the mapping must have an
         assertion.
      8. **Leftovers.** `dbg!`, `println!`, `todo!`, `TODO`, `FIXME`, commented-out code, and
         hardcoded widths that should have come from the interior.

- [ ] 12.2 CHANGE: Fix every CRITICAL. Resolve or consciously accept each WARNING with a
      one-line reason recorded in `planning-review.md`. Note each SUGGESTION. Re-run the
      affected tests and any check whose input changed.

- [ ] 12.3 VERIFY: Confirm no blocking or unowned finding remains, and that the finding counts
      are recorded in `planning-review.md` alongside the planning-time findings.
      Commit: `review(markdown-viewer): address Change Review findings`.

---

## 13. Lint & Verify
<!-- kind: operational -->

- [ ] 13.1 CHECK: Inspect the intended verification commands and affected tiers. The gate is
      `make check`; the tiers are `--lib` unit and view tests only — this change adds no
      `tests/cli.rs` case and the binary's behaviour is unchanged.

- [ ] 13.2 VERIFY: `cargo fmt --all -- --check` — clean.

- [ ] 13.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings.
      (This repository has no separate type checker: `cargo clippy` builds the crate, so a
      type error fails here.)

- [ ] 13.4 VERIFY: `cargo test --all-features` — green. Then the counted floors, so a renamed
      module cannot pass as a green run: `testcount --lib 'ui::markdown::tests::' 23`,
      `testcount --lib 'ui::app::tests::' 30`, `testcount --lib 'ui::layout::tests::' 12`,
      `testcount --lib 'ui::list::tests::' 17`, `testcount --lib 'ui::view::tests::' 57`,
      `testcount --lib 'ui::driver::tests::' 10`, `testcount --lib 'ui::tests::load::' 6`,
      `testcount --lib 'ui::tests::detail::' 1`,
      `testcount --lib 'changes::fixture::tests::' 2`, and `testcount --lib '' 564`.
      Record the final lib total and the whole-suite total.

- [ ] 13.5 VERIFY: `make coverage` — `cargo llvm-cov --fail-under-lines 80` at the unchanged
      floor. Record the TOTAL **line** percentage — the column to the right of the region
      count — and the uncovered-line count, and compare against 0.1's 98.26% over 10,493
      lines: a fall of more than about one point is a finding, not a pass.
      **Red when:** the floor is met only because an exclusion was added — `NOWAIVER` is what
      proves it was not.

- [ ] 13.6 VERIFY: `make check` as the single gate — all four in order. If it fails, name the
      failing sub-command rather than reporting a summary.

- [ ] 13.7 VERIFY: Re-run the whole architectural suite one last time against the final tree:
      `NOSPAWN-GREP` with `MIN=17`, `NOIO-VIEW`, `NOCLI-SHELL` with `UI_MIN=9`, `NORAW-GREP`,
      `NODEFAULT-UI` (no argument), `NOLIT-CHANGE` with `MIN=17`, `MDSEAM` with `MIN=17`,
      `WIDTHS` with `WIDTHS_MIN=57`, `LISTWIDTHS`, `MDWIDTHS`, `NOWAIVER`, `GATE-MECH1`,
      `NOJSON-SEAM`, `DEPS` (with `DEPS_SKIP_LEG5=1`; 10.1 ran it in full), `GRAPH-SNAP`, and
      `OPENSPEC-UNTOUCHED` against `$BASE`.

- [ ] 13.8 VERIFY: `openspec validate markdown-viewer --strict` — valid. The `openspec` binary
      is nvm-installed: `export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"` first.
      Commit: `docs(markdown-viewer): record group 13 lint & verify evidence — apply complete`.
