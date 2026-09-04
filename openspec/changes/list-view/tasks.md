# list-view — tasks

Read `design.md` first: the Boundaries table, the Test Boundaries table, and the
verification matrix are the contract these tasks implement. **No task may invent a
collaborator that table does not name.**

**The outer loop is taken, at the `ui::` composition tier.** `ui::load` →
`changes::from_files` → `ui::list::rows` → `ui::view::render` is a path no unit test
crosses, and it is the thing this change exists to make work. Group 0 writes that test
first — against the API that already exists today, so it **compiles and fails on an
assertion** rather than failing to build — and group 8 closes it.

**Every floor in this file is measured before it is raised.** Task 1.1 reads the real
counts off `main` and records them; every later `MIN`, `UI_MIN`, `WIDTHS_MIN`, `LIST_MIN`,
and `testcount` minimum is *measured + this change's addition*. This repository has already
shipped a check whose floor was asserted rather than measured and was therefore unpassable.

## Test-module convention — read before writing any test

`cargo test` matches a filter against a test's **full path**, and a filter matching nothing
exits 0. Every filtered gate below therefore addresses tests through an explicit module
path, and is judged on a counted minimum rather than on the run's exit status.

| Group | File | Module | Filter | Target count |
|---|---|---|---|---|
| 2 | `src/ui/app.rs` | `mod tests` → `mod keys` | `ui::app::tests::` | 22 |
| 3 | `src/ui/layout.rs` | `mod tests` | `ui::layout::tests::` | 9 |
| 4 | `src/ui/list.rs` | `mod tests` | `ui::list::tests::` | 17 |
| 4 | `src/changes.rs` | `mod fixture` → `mod tests` | `changes::fixture::tests::` | 2 |
| 5, 6 | `src/ui/view.rs` | `mod tests` | `ui::view::tests::` | 47 |
| 7 | `src/ui/driver.rs` | `mod tests` | `ui::driver::tests::` | 8 |
| 0, 7, 8 | `src/ui/mod.rs` | `mod tests` → `mod load` | `ui::tests::load::` | 5 |

**`make check` is not runnable unqualified between groups 0 and 8.** Group 0's acceptance
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

A **second** failing test at any of those boundaries is a real regression, not the known
one. The literal, unqualified `make check` is run from task 8.2 onward.

---

## Command-level checks, written out once

Referenced by label from the tasks below. They live here rather than in a table cell
because an unescaped `|` cannot appear in a Markdown table and an escaped `\|` inside an
ERE matches a literal pipe, so the check would pass against the very code it exists to
catch. Every one is judged on **output emptiness, a counted minimum, or a specific error
code**, never on a bare pipeline exit status: `grep` exits 1 for no-match and 2 for a bad
file, and `!` turns both into a pass.

Extract each block to `$CHECKS/<LABEL>.sh` in task 1.1 and run the extracted,
byte-identical file thereafter, so the run and the record cannot drift.

```sh
# NOSPAWN-GREP — `cli` is the only module in the crate that spawns a process.
# Carried forward from tui-shell BYTE-IDENTICALLY. Only its MIN argument moves, from 8 to
# 16, and MIN is a parameter precisely so that is a change to a task's invocation rather
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
# Carried forward from tui-shell with ONE deliberate edit, recorded in design.md ->
# Boundaries: PURE gains src/ui/list.rs, so the set is five files rather than four.
# Three files under src/ui/ are still deliberately NOT searched, each for a stated reason:
# mod.rs holds ui::load and ui::run, terminal.rs holds the terminal seam, and event.rs
# holds CrosstermEvents, which reads the real event stream. Do not "fix" the list by
# adding them. A view test that needs a real directory is the signal this check exists to
# make impossible.
PURE="src/ui/app.rs src/ui/layout.rs src/ui/list.rs src/ui/view.rs src/ui/driver.rs"
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
echo "NOIO-VIEW OK: 5 pure files carry no I/O API; positive control matched"
```

```sh
# NOCLI-SHELL — the dashboard shell never names the CLI seam. This is the mechanical form
# of the roadmap's "list-view depends on changes-from-files, not on the CLI".
# Carried forward from tui-shell BYTE-IDENTICALLY; only UI_MIN's invocation moves, 7 -> 8.
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
# NODEFAULT-UI — Dashboard and Filter have no Default, and no construction or destructuring
# elides a field. change-model's GATE-MECH1 covers Change/ChangeSet/ArtifactRef/Origin in
# src/changes.rs ONLY, so a state type in src/ui/app.rs is outside it.
# Carried forward from tui-shell with ONE deliberate edit, recorded in design.md ->
# Boundaries: the single hardcoded type name becomes the parameterised list
# TYPES="Dashboard Filter", so list-view's second state type is covered by the SAME
# mechanism rather than by a second, drifting check. Both halves and both positive
# controls are unchanged in substance.
SRC="${SRC:-src}"
# `${TYPES-...}` and NOT `${TYPES:-...}`: with the colon, an explicitly empty TYPES silently
# falls back to the default and the emptiness guard below can never fire. The guard also
# rejects a whitespace-only value, which `[ -n ]` alone would accept and which would make
# the loop body run zero times while still printing OK.
TYPES="${TYPES-Dashboard Filter}"
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
# Carried forward from tui-shell BYTE-IDENTICALLY, invocation included: its file-count
# guard is >= 16 and the tree holds 17 today, 18 after this change.
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
# NOLIT-CHANGE — NEW in list-view. `Change { … }` and `ChangeSet { … }` literals and
# patterns appear ONLY in src/changes.rs. design.md -> Boundaries: this change's view
# fixtures are built by constructors inside that file, so change-model's GATE-MECH1 (no
# Default, no `..`, searched in src/changes.rs ONLY) keeps covering every construction site
# in the crate. Without this check, a fixture built in src/ui/view.rs would be a
# construction site the existing gate cannot see, and nobody would notice for a phase.
# Verified to pass on main before any edit (task 1.2).
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
# WIDTHS — every #[test] in src/ui/view.rs names both 60 and 120. A heuristic, and design.md
# -> Risks says so: it cannot prove an assertion is meaningful, only that both widths are
# present. It is the cheap half of the mandate; the per-scenario tasks are the other half.
# Carried forward from tui-shell with ONE deliberate edit: the floor, hardcoded at 16, is
# now read from WIDTHS_MIN with that same default, so raising it is a change to a task's
# invocation rather than to the check. Nothing else moves.
#
# Known limits, stated rather than discovered later: a `60` in a comment satisfies it, and
# so does an unrelated `60` literal. It is a FLOOR. What proves the tests exist and run is
# `testcount --lib 'ui::view::tests::' 47` (task 6.5), not this.
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
# LISTWIDTHS — NEW in list-view. Every #[test] in src/ui/list.rs names both 38 and 58, the
# two list-region INTERIORS the mandated 120- and 60-column frames produce (Length(40) less
# two border columns, and a 60-column body less two border columns). Same heuristic, same
# stated limits, and the same pairing as WIDTHS: it is a FLOOR, and
# `testcount --lib 'ui::list::tests::' 17` is what proves the tests exist and run.
# Unlike WIDTHS, whose default is tui-shell's 16 and must be passed explicitly, this
# block is new here, so its default IS this change's final floor: 17.
#
# Known limit inherited from WIDTHS, stated rather than discovered later: the number scan is
# `\b(\d+)\b`, which does NOT see a suffixed literal such as `38u16` or `38usize`. It fails
# closed — a test using only suffixed literals is reported as missing a width — so write the
# widths unsuffixed, or as a `const` the test also names bare.
#
# It has NO exemption list, and design.md -> Decisions records the module split that makes
# that possible: every function in src/ui/list.rs is parameterised by a width. `viewport`
# lives in ui::layout and `matches`/`visible`/`visible_len` live in ui::app precisely so
# that no test in this file has a legitimate reason to name neither width. An exemption
# list is how a width check rots into a rubber stamp.
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
# NOWAIVER — the coverage floor was not lowered, excluded, or annotated away.
# Carried forward from tui-shell BYTE-IDENTICALLY.
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
# captured in task 1.1, never against the index: this project commits per task group, so
# `git diff --exit-code` between working tree and index passes over the very change it
# exists to catch.
#
# `git diff` alone is NOT enough: it lists tracked paths only, and a file the plugin WRITES
# at runtime is untracked, so the one violation this check exists to catch would be
# invisible to it. The `ls-files --others` sweep is the half that catches it.
#
# Exactly ONE exclusion: this change's own artifact directory, a hand-edited planning
# document rather than a code-path write. Excluded BY NAME, never by a broad prefix, so a
# stray file anywhere else under openspec/ still fails. Carried forward from tui-shell with
# that one path updated.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
# ONE deliberate edit to the carried-forward block, recorded in planning-review.md: a third
# sweep for IGNORED untracked files. `--others --exclude-standard` alone hides anything a
# `.gitignore` covers, and a runtime write is exactly the kind of file someone adds a
# gitignore line for. The whole point of this check is to catch a write nobody declared.
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --ignored --exclude-standard -- ':(top)openspec/'; } \
         | grep -v '^openspec/changes/list-view/' | sort -u || true)
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
# This block DEFINES a shell function; it must be SOURCED (`. $CHECKS/TESTCOUNT.sh`) in
# every shell that runs a gate, not executed with `sh`.
#   usage: testcount <scope> <filter> <minimum>   scope: --lib | --test-cli
# Carried forward from tui-shell BYTE-IDENTICALLY.
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
# DEPS — carried forward from changes-from-cli with the FOUR edits tui-shell's task 1.5
# recorded only as prose, applied here and written out, so no later change has to
# re-derive them from an archived checkbox: (1) ratatui in leg 2's want dict, (2) the
# crossterm undeclared-but-resolved pair, (3) a leg-5 case for ratatui, (4) leg 3
# deleted in favour of GRAPH-SNAP. list-view itself adds NO dependency, so this block
# must pass unchanged; it is re-run, not modified.
# DEPS — the argued dependency set, the one binary target, the resolved graph, the MSRV
# floor, and the genuinely-needed experiment. Reads `cargo metadata` JSON through
# python3 rather than adding a crate to do it, as subprocess-seam's DEPS does.
#   env: WORK  scratch directory (required, guarded); leg 5 copies the crate into it,
#              because `cargo build` REWRITES Cargo.lock during resolution before it
#              reaches the compile error the leg waits for, so an edit-and-restore in
#              place would silently discard the resolution this change verified.
#        DEPS_SKIP_LEG5=1 skips the three full builds.
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
      "ratatui":["crossterm"]}
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
# this clause is read from the TEXT.
grep -q 'yaml-rust2 = .*features = \[\]' Cargo.toml \
  || { echo "DEPS FAIL: leg 2b yaml-rust2 does not spell out features = [] in Cargo.toml" >&2; exit 1; }
echo "DEPS OK (leg 2b): yaml-rust2's empty feature list is written out in Cargo.toml"
# leg 2a-bis: crossterm is ABSENT from the declared set (asserted above) and PRESENT in the
# resolved graph. Both halves are needed: absent-and-absent would mean the backend is gone,
# and declared-and-present is the two-crossterm-versions hazard the design forbids.
cargo tree -e normal 2>/dev/null | grep -qE '(^|[^A-Za-z0-9_-])crossterm v' \
  || { echo "DEPS FAIL: leg 2a-bis crossterm is not in the resolved normal graph" >&2; exit 1; }
echo "DEPS OK (leg 2a-bis): crossterm undeclared but resolved, reached through ratatui"
cargo build --locked >/dev/null 2>&1 \
  || { echo "DEPS FAIL: leg 2c cargo build --locked" >&2; exit 1; }
echo "DEPS OK (leg 2c): cargo build --locked"

# --- leg 3: DELETED. Superseded by GRAPH-SNAP, which compares the resolved graph
# against tests/fixtures/build-graph.txt per triple and enforces a proc-macro ALLOWLIST.
# tui-shell's design.md -> Decisions made that call: leg 3's sixteen-package
# EXPECTED_GRAPH, its all-four-triples-identical assertion, and its blanket
# no-proc-macro rule are all falsified by ratatui, so leaving it here would make this
# check permanently unsatisfiable. TRIPLES is kept: leg 4 still uses it.

# --- leg 4: MSRV, compared against Cargo.toml's OWN rust-version -------------
# A check carrying its own literal floor keeps enforcing the old value when the
# crate's rust-version moves, and the requirement is a claim about the relationship.
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

**Carried forward unchanged**, extracted byte-identically rather than retyped:
`GATE-MECH1.py` and `NOJSON-SEAM.sh` from
`openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md`, and `GRAPH-SNAP.sh` from
`openspec/changes/archive/2026-09-04-tui-shell/tasks.md`.

**`DEPS` is written out above rather than extracted**, and that is a deliberate departure
from three changes' convention. `tui-shell` made four edits to its own extracted copy and
recorded them **only as prose in a checkbox**; the archived block therefore fails on today's
tree (`normal deps are ['ratatui', 'serde_json', 'toml', 'yaml-rust2'], expected
['serde_json', 'toml', 'yaml-rust2']`), which would halt this change at task 1.2 with
nowhere to go. The four edits are applied here and written out so no later change has to
re-derive them from an archived checkbox. `list-view` itself adds no dependency, so the
block is re-run, not further modified — and `GRAPH-SNAP` likewise.

---

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [ ] 0.1 Set up the harness: no new harness is needed. `crate::testutil::ScratchDir`,
      `write_with_mode`, `render_at`, `row_text`, and `cell` all exist and are exactly what
      design.md → Test Boundaries names — the filesystem **real** through `ScratchDir`, the
      rendering surface **replaced** by `TestBackend`, the terminal **never reached**, the
      `openspec` binary **absent by construction**. The test goes in
      `src/ui/mod.rs`'s `mod tests` → `mod load`, **not** in `src/ui/view.rs`: it is a
      composition test over `ui::load` + `ui::view::render`, and `NOIO-VIEW` must keep
      proving that `view.rs` and `list.rs` name no filesystem API.

- [ ] 0.2 RED: Add `ui::tests::load::a_scratch_repository_renders_its_change_rows` for
      change-rows → "Active rows render at both mandated widths", composed with
      dashboard-loop → "A scratch repository is loaded from disk with no binary present".
      Build a `ScratchDir` repository holding
      `openspec/changes/add-token-refresh/tasks.md` with 4 checked of 9,
      `openspec/changes/fix-empty-basket/tasks.md` with 7 of 7,
      `openspec/changes/migrate-ai-sdk-v7/proposal.md` and no tasks file, and
      `openspec/changes/archive/2026-08-14-add-auth/tasks.md` with 7 of 7. Call
      `ui::load(root, &Config { archived_count: 5, ..Config::default() })`, then
      `testutil::render_at(120, 20, &dashboard)` and `testutil::render_at(60, 20, &dashboard)`.
      Assert, at **both** widths, the interior rows by exact string:
      - 120x20, row 2 columns 1..=38: `> add-token-refresh              [4/9]`
      - 120x20, row 3 columns 1..=38: `  fix-empty-basket               [7/7]`
      - 120x20, row 4 columns 1..=38: `  migrate-ai-sdk-v7                [-]`
      - 120x20, row 5 columns 1..=38: `  -- archived ------------------------`
      - 120x20, row 6 columns 1..=38: `  2026-08-14 add-auth            [7/7]`
      - 60x20, row 2 columns 1..=58: `> add-token-refresh                                  [4/9]`
      - 60x20, row 3 columns 1..=58: `  fix-empty-basket                                   [7/7]`
      - 60x20, row 4 columns 1..=58: `  migrate-ai-sdk-v7                                    [-]`
      - 60x20, row 5 columns 1..=58: `  -- archived --------------------------------------------`
      - 60x20, row 6 columns 1..=58: `  2026-08-14 add-auth                                [7/7]`
      Read columns by **character index**, never byte offset — `…` and the box-drawing
      borders are multi-byte. Use only API that exists today, so the test **compiles**.

- [ ] 0.3 Confirm it fails because the behaviour is missing, not because the harness is
      misconfigured. Run `cargo test --all-features --lib
      ui::tests::load::a_scratch_repository_renders_its_change_rows` and record the message
      verbatim. **Expected today:** the assertion on the 120-column row 2 fails, comparing
      the expected row against 38 spaces — the interior is blank because `tui-shell`
      asserts it is.
      **Red when:** the first row assertion fails against an all-spaces actual.
      **Wrongly green when:** the filter matched nothing — guarded by confirming the run
      reports exactly one test ran under the filter, not `0 passed; 0 failed`.
      **Also wrongly green when:** the fixture's tasks files are miscounted, so re-read the
      recorded actual and confirm it is spaces rather than a *different* row.

---

## 1. Baseline, checks, and the scratchpad
<!-- kind: operational -->

- [ ] 1.1 CHECK: Record the starting state before any edit, by **measuring**, never by
      copying a number from this file.
      - `git rev-parse HEAD` → `export BASE=<sha>`. Every `OPENSPEC-UNTOUCHED` run uses it.
        A diff against the index would pass over this change's own per-group commits.
      - `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
        passed.*/\1/p'` → the **lib** baseline (measured on `main` at planning time as part
        of a 465-test suite; record the real number).
      - `find src -name '*.rs' ! -path 'src/cli.rs' | wc -l` → `NOSPAWN-GREP`'s and
        `NOLIT-CHANGE`'s starting file count. **Measured at planning time: 15.**
      - `find src/ui -name '*.rs' | wc -l` → `NOCLI-SHELL`'s `UI_MIN` baseline.
        **Measured at planning time: 7.**
      - `find src tests -name '*.rs' ! -path 'src/ui/terminal.rs' | wc -l` →
        `NORAW-GREP`'s guard. **Measured at planning time: 17** (its floor is 16).
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/view.rs` → `WIDTHS`' baseline.
        **Measured at planning time: 17** (the shipped floor is 16).
      - `cargo llvm-cov --summary-only` → TOTAL line coverage. **98.01% over 8,836 lines**
        at planning time.
      - `export CHECKS=<scratchpad>/list-view-checks` and
        `export WORK=<scratchpad>/list-view-work`; `mkdir -p "$WORK"`. Extract every fenced
        block above — `DEPS.sh` included, which this file writes out rather than inheriting
        — to `$CHECKS/<LABEL>.sh` byte-identically, and extract `GATE-MECH1.py` and
        `NOJSON-SEAM.sh` from
        `openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md` and `GRAPH-SNAP.sh`
        from `openspec/changes/archive/2026-09-04-tui-shell/tasks.md`, byte-identically.
        `DEPS` needs `WORK` set and refuses to run without it.
        Run the extracted files from here on, never a retyped copy. `TESTCOUNT.sh` is the
        exception: it only *defines* a function, so **source** it (`. $CHECKS/TESTCOUNT.sh`)
        in each shell that runs a gate.
      **Red when:** `BASE` is empty or not a commit (`OPENSPEC-UNTOUCHED` aborts on both),
      or a measured count comes back **below** the planning-time figure recorded above, in
      which case the tree is not the one this plan was written against and the discrepancy
      is resolved before any edit.

- [ ] 1.2 CHECK: Run the checks that must **pass** on the tree as it stands, so this change
      starts from clean gates rather than inheriting broken ones: `NOSPAWN-GREP` (default
      `MIN=8`), `NOCLI-SHELL` (default `UI_MIN=7`), `NORAW-GREP`, `NODEFAULT-UI` with
      `TYPES=Dashboard` (its edited form must still pass on the pre-change tree),
      `NOLIT-CHANGE`, `NOWAIVER`, `GATE-MECH1`, `NOJSON-SEAM`, `DEPS` (with `WORK` set and
      `DEPS_SKIP_LEG5=1` — leg 5 is four full builds and nothing in this change touches a
      dependency, so it runs once, at 9.1), `GRAPH-SNAP`, and `OPENSPEC-UNTOUCHED`. Record
      each output verbatim. `DEPS`'s expected legs on today's tree, reproduced during
      planning: `leg 1a`, `leg 1b`, `leg 2a` (four normal deps), `leg 2b`, `leg 2a-bis`
      (crossterm undeclared but resolved), `leg 2c`, and `leg 4` (floor 1.88). `NOIO-VIEW` and `LISTWIDTHS` are
      **expected to fail** here (`src/ui/list.rs` does not exist yet) — record those two
      failures as the pre-change baseline, not as a problem.
      **Red when:** any of the eleven fails, in which case this change is not the place to
      fix it.
      **Wrongly green when:** an extraction typo made a block a no-op — guarded by 1.3.

- [ ] 1.3 CHECK: Prove the **existence guards** and **positive controls** of every extracted
      check can fail, before trusting any of them. Against a throwaway copy under `$WORK`:
      - `NOSPAWN-GREP` with `SRC=$WORK/empty` → `no such directory`.
      - `NOSPAWN-GREP` with `MIN=16` on the current tree → `searched only 15 files under src
        (expected >= 16)`, which is exactly the floor this change will make true.
      - `NOCLI-SHELL` with `UI_MIN=8` on the current tree → `only 7 files under src/ui
        (expected >= 8)` — the same shape.
      - `NOIO-VIEW`, `LISTWIDTHS`, `WIDTHS`, `NORAW-GREP`, `NODEFAULT-UI`, `NOLIT-CHANGE`
        run in a copy with the named file deleted → each reports `<file> missing`, never a
        clean tree.
      - `NODEFAULT-UI` with `TYPES=""` **and** with `TYPES=" "` → both report
        `TYPES is empty - the loop would run zero times`. Both matter: the `:-` form of the
        default silently rewrites an empty `TYPES` back to the default (so the guard would
        be unreachable), and `[ -n ]` alone accepts a single space (so the loop would run
        zero times and still print OK). Reproduced during planning against the `:-`/`[ -n ]`
        form, which is why the block above uses `${TYPES-…}` and a `case` guard.
      - `NODEFAULT-UI` with `TYPES=NoSuchType` → the positive control fires
        (`app.rs has no 'struct NoSuchType {'`), proving the control is not decorative.
      - `NODEFAULT-UI` with `TYPES=Dashboard` in a copy whose `pub struct Dashboard` is
        renamed `pub struct DashboardState` → the positive control **fires** rather than
        passing on the substring, which is why both `struct $T` greps are anchored with a
        following `{`. Use `Dashboard`, not `Filter`: `struct Filter` does not exist until
        group 2, so a `Filter`-based control here would be unperformable.
      - `NOLIT-CHANGE` in a copy whose `src/changes.rs` has its `ChangeSet {` literals
        renamed → the positive control fires.
      - `OPENSPEC-UNTOUCHED` with `BASE=` empty and with `BASE=deadbeef` → each aborts.
      - `WIDTHS` with `WIDTHS_MIN=99` → `found 17 … expected >= 99`, proving the new
        parameter is read rather than ignored.
      - `LISTWIDTHS` with `LIST_MIN=99` after creating an empty `src/ui/list.rs` in the copy
        → `found 0 … expected >= 99`, proving the floor is read; and with no argument at all,
        confirm the reported floor is this change's 17 rather than a copied 16.
      Record each message verbatim.
      **Red when:** any guard reports success. A guard that cannot fail is worth nothing,
      and this repository has shipped three of them.

- [ ] 1.4 CHECK: Confirm the intermediate-gate substitution stated at the top of this file
      is real: run `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` on the tree with
      group 0's acceptance test red, and confirm it **reports a TOTAL** rather than
      producing no report. Record the percentage.
      **Red when:** it produces no report, in which case every intermediate gate in groups
      2–7 must drop its coverage step and say so here instead of silently skipping it.

- [ ] 1.5 Commit the plan-time baseline record (this file's checkbox state only; no source
      change yet). Conventional Commits: `docs(list-view): record measured check baselines`.

---

## 2. `ui::app` — state, the visible list, and the key map
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing tests in `src/ui/app.rs`'s `mod tests` → `mod keys` for:
      - `matches_is_case_insensitive_substring_on_the_name` — `matches("add-token-refresh",
        "add")`, `("ADD-TOKEN-REFRESH", "add")`, `("add-token-refresh", "ADD")`,
        `("add-token-refresh", "")` all true; `("fix-empty-basket", "add")` false. Covers
        list-filtering → "Matching ignores case".
      - `visible_applies_the_query_to_both_tiers` — the five-change fixture (three active,
        two archived), query `add` → exactly `["add-token-refresh", "add-auth"]` in that
        order; query `` → all five in `active`-then-`archived` order; query `zzz` → empty.
        Covers list-filtering → "A query narrows both tiers at both widths" at the unit
        tier.
      - `quit_keys_and_their_near_misses` — **rewritten**: the existing five events under
        `filtering: false` unchanged, plus the same five under `filtering: true` returning
        `FilterPush('q')`, `Quit`, `FilterPush('Q')`, `Ignore`, `FilterPush('c')`.
      - `only_press_kind_acts` — **rewritten**: Release, Repeat, Press under both modes.
      - `enter_and_esc_map_to_routes` — **rewritten** for the new signature.
      - `non_key_events_are_ignored` — **rewritten**: the same five events under both modes.
      - `action_for_is_total_over_a_keycode_sweep` — **rewritten**: the twenty-plus-code
        sweep run under both modes; under `filtering: true` every `Char` maps to
        `FilterPush` and every non-`Char` outside the table maps to `Ignore`.
      - `navigation_and_filter_keys_are_distinguished` — `Char('j')`, `Down`, `Char('k')`,
        `Up`, `Char('/')` → `SelectNext`, `SelectNext`, `SelectPrev`, `SelectPrev`,
        `FilterStart`; `Char('J')`+SHIFT, `Down`+CONTROL, `Char('/')`+CONTROL → `Ignore`.
      - `slash_starts_filter_mode_and_routes_to_list` — from `Route::Detail`, `FilterStart`
        leaves `filter.active` true, `filter.query` empty, `route` `List`.
      - `filter_mode_types_printable_characters` — `q`, `j`, `/` typed → query `qj/`,
        `quit` false.
      - `ctrl_c_quits_from_filter_mode` — `Ctrl-C` under `filtering: true` → `Quit`, and
        applying it sets `quit`.
      - `backspace_deletes_and_is_inert_on_empty` — query `ad` → `a` → `` → `` with
        `filter.active` still true after all three.
      - `enter_accepts_the_filter_without_opening_detail` — `filter.active` false,
        `filter.query` still `add`, `route` still `List`.
      - `esc_dismisses_one_layer_at_a_time` — from `Route::Detail` + active filter `add`,
        four `Back`s give (inactive, empty query), then `List`, then no change, then no
        change, `quit` false throughout; plus the inactive-query variant.
      - `select_next_and_prev_clamp` — three changes: four `SelectNext` → 2; four
        `SelectPrev` → 0.
      - `selection_clamps_when_the_filter_shrinks_the_list` — five visible, `selected` 4,
        then `FilterStart` + `a`, `d`, `d` → `selected` 1; three `FilterPop` → five visible,
        `selected` still 1.
      - `apply_never_leaves_selected_out_of_range` — over `changes::empty_set()` and over
        the five-change fixture, apply every `Action` variant in turn and assert
        `selected == 0 || selected < visible_len()` after each; `SelectNext`/`SelectPrev`
        over the empty set leave `selected` 0 and do not panic.
      - `dashboard_destructures_into_exactly_seven_fields` — **rewritten** from five.
      - `filter_destructures_into_exactly_two_fields` — new companion.
      - `apply_quit_sets_the_flag`, `apply_moves_between_routes`,
        `back_to_list_at_list_is_a_no_op` — carried, adjusted for the rename.
      Covers: dashboard-loop's six key scenarios, list-filtering's first four and
      "Shrinking the visible list clamps the selection", list-selection's
      "`j`, `k`, and the arrows move the selection", "Selection clamps at both ends" and
      "Navigation over an empty visible list is inert" at the unit tier.
      **Red when:** the crate does not compile because `Filter`, `Dashboard::selected`,
      `Action::SelectNext`, and the two-argument `action_for` do not exist. That is an
      honest RED for a typed language; confirm the compiler names those items and not a
      typo in the test module.

- [ ] 2.2 GREEN: Add `pub struct Filter { pub query: String, pub active: bool }` to
      `src/ui/app.rs`, deriving `Debug, Clone, PartialEq, Eq` and **not** `Default`. Add
      `selected: usize` and `filter: Filter` to `Dashboard`. Extend `Action` to the nine
      variants in design.md → Contracts, renaming `BackToList` to `Back`.

- [ ] 2.2b GREEN: Update **every** `Dashboard` literal in the crate in this same commit.
      `..` is forbidden by `NODEFAULT-UI`, so each must name the two new fields or the
      crate does not compile and roughly thirty landed tests go with it. Find them first —
      `grep -rn 'Dashboard[[:space:]]*{' src/` — which returns **14 lines** on `main`, of
      which **seven are sites** and seven are not (five `-> Dashboard {` signatures, one
      `pub struct Dashboard {`, one `impl Dashboard {`). The seven sites, measured:
      - `src/ui/mod.rs:97` and `:105` — `load`'s two literals. Group 7 owns their values,
        but they must compile now, so give them `selected: 0` and an empty inactive
        `Filter` here.
      - `src/ui/app.rs:97` — `mod tests`' `dashboard_at` helper (a literal).
      - `src/ui/app.rs:260` — `let Dashboard { … }` in
        `dashboard_destructures_into_exactly_five_fields`. A **pattern**, not a literal, and
        the one site a "literal" sweep loses: `NODEFAULT-UI` half B forbids `..` in a
        pattern too, and task 2.1 renames this test to `…_seven_fields`.
      - `src/ui/view.rs:145` — `mod tests`' `dashboard` helper.
      - `src/ui/driver.rs:86` — `mod tests`' `dashboard` helper.
      - `src/lib.rs:216` — `testutil::tests::empty_dashboard`.
      The four test helpers take `selected: 0` and an empty inactive `Filter`, so every
      landed assertion keeps its meaning.
      **Red when:** the grep returns a site outside those seven — stop and record it rather
      than editing blind. **Wrongly green when:** the sweep is done by eye on the literal
      form alone and `app.rs:260`'s pattern is missed; it fails to compile, so the error is
      loud, but the fix belongs here rather than as a surprise in 2.8.

- [ ] 2.3 GREEN: Implement `pub fn matches(name: &str, query: &str) -> bool` —
      `name.to_ascii_lowercase().contains(&query.to_ascii_lowercase())`, true for an empty
      query — and `Dashboard::visible(&self) -> Vec<&Change>` /
      `Dashboard::visible_len(&self) -> usize`, active-then-archived with `matches` applied
      to `Change::name` and nothing else.

- [ ] 2.4 GREEN: Implement `action_for(event, filtering)` as the two tables in
      dashboard-loop and list-filtering, `KeyEventKind::Press` only, total over every event
      under both modes.

- [ ] 2.5 GREEN: Implement `Dashboard::apply` as the state machine in dashboard-loop →
      "Key handling is a pure, total function over events", with a single private
      `clamp_selection` helper called after every action that can change the visible list or
      the index — `SelectNext`, `SelectPrev`, `FilterPush`, `FilterPop`, and the two `Back`
      layers that clear a query.

- [ ] 2.6 REFACTOR: Fold the two `Back` query-clearing layers and `clamp_selection` so the
      clamp is written once; if no duplication remains after 2.5, state that here rather
      than inventing a change.

- [ ] 2.7 CHECK (contract gate): three interfaces move in this group — `action_for`'s
      signature, `Action`'s variants, and `Dashboard`'s field list. Grep the tree for
      **all three**: `action_for(`, `Action::`, and `Dashboard[[:space:]]*{`. The third is
      load-bearing and easy to forget: neither `src/ui/view.rs` nor `src/lib.rs` names
      `action_for` or `Action::`, so a gate that looked only for the call site would miss
      two of the six literal sites entirely. Expected: `action_for(` and `Action::` in
      `src/ui/app.rs` and `src/ui/driver.rs` only; `Dashboard {` at the six sites 2.2b
      lists; **nothing** under `tests/`. Record the full site list.
      **Red when:** a site outside those files appears, meaning a consumer this change did
      not plan for.

- [ ] 2.8 VERIFY: `. $CHECKS/TESTCOUNT.sh` then `testcount --lib 'ui::app::tests::' 22`.
      Then run `NODEFAULT-UI` (default `TYPES="Dashboard Filter"`) — it must **pass**, and
      its `Filter` positive control must now find `struct Filter`. Then the four
      intermediate-gate commands from the top of this file; `cargo test --all-features` must
      fail on **exactly** group 0's acceptance test.
      **Red when:** the count is below 22, `NODEFAULT-UI`'s `Filter` control fires, or a
      second test fails. A *compile* failure here means 2.2b missed a `Dashboard` literal;
      go back to its grep rather than patching the reported site alone.
      **Wrongly green when:** `testcount` is executed with `sh` instead of sourced, so the
      function is undefined and the shell's error is mistaken for a pass — guarded by
      confirming the `TESTCOUNT OK:` line appears.
      Commit: `feat(list-view): add Filter, selection, and the mode-aware key map`.

---

## 3. `ui::layout::viewport` — the visible slice
<!-- kind: behavior -->
<!-- parallel-after: 1 -->

- [ ] 3.1 RED: Write failing tests in `src/ui/layout.rs`'s `mod tests` for:
      - `viewport_is_zero_when_everything_fits` — `(0, 0, 16)`, `(16, 15, 16)`,
        `(17, 0, 16)`, and `(30, 20, 0)` all return `0`.
      - `viewport_centres_and_clamps` — `(30, 20, 16)` → 12; `(30, 29, 16)` → 14;
        `(30, 20, 8)` → 16; `(30, 0, 16)` → 0.
      - `viewport_boundaries_are_exact` — the seven-call table from list-selection → "The
        viewport is exact at its boundaries": `(0,0,16)`, `(16,15,16)`, `(17,0,16)`,
        `(17,8,16)`, `(17,9,16)`, `(17,16,16)`, `(30,20,0)` → `0,0,0,0,1,1,0`.
      Covers list-selection → "The viewport is exact at its boundaries" at the unit tier.
      **Red when:** `ui::layout::viewport` does not exist and the module fails to compile.

- [ ] 3.2 GREEN: Implement `pub fn viewport(rows: usize, cursor: usize, height: u16) -> usize`
      exactly as list-selection states: `0` when `height == 0` or `rows <= height as usize`,
      else `min(cursor.saturating_sub(height as usize / 2), rows - height as usize)`.

- [ ] 3.3 REFACTOR: None expected — the function is four lines with no duplication. State
      that rather than inventing a change.

- [ ] 3.4 VERIFY: `testcount --lib 'ui::layout::tests::' 9`, then the four intermediate-gate
      commands. `cargo test --all-features` must still fail on exactly group 0's test.
      **Red when:** the count is below 9, which is 6 measured at `main` plus this group's 3.
      Commit: `feat(list-view): add layout::viewport, the derived scroll offset`.

---

## 4. `ui::list` — the row grammar
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests in a new `#[cfg(test)] pub(crate) mod fixture` →
      `mod tests` inside `src/changes.rs`, naming builders that do not exist yet:
      - `every_builder_satisfies_the_change_invariants` — `fixture::active(...)` and
        `fixture::archived(...)` outputs pass `conformance::assert_invariants`, so an
        `Active` change's `dir` really ends in its name and an `Archived` one's ends with it.
      - `the_five_change_fixture_is_ordered_active_then_archived` — `fixture::set(...)`
        preserves the two vectors' order and does not sort.
      **Red when:** the crate does not compile because `fixture::active`, `fixture::archived`,
      and `fixture::set` are undefined. This is deliberately the *first* task of the group:
      every later RED in it needs the fixture.

- [ ] 4.2 GREEN: Implement the three builders in that module, naming **every** field of
      `Change` and `ChangeSet` explicitly — no `..`, no `Default` — so `GATE-MECH1` and
      `NOLIT-CHANGE` both keep covering them. `active(name, completed, total)` builds an
      `Origin::Active` change whose `dir` ends in `name`; `archived(date, name, completed,
      total)` builds an `Origin::Archived { date }` change whose `dir` ends with `name`.

- [ ] 4.3 RED: Write failing tests in `src/ui/list.rs`'s `mod tests`. **Every one calls
      `rows` at both 38 and 58**, which `LISTWIDTHS` enforces mechanically:
      - `active_row_grammar_at_38_and_58` — the three-change fixture at 38 gives
        `> add-token-refresh              [4/9]`,
        `  fix-empty-basket               [7/7]`,
        `  migrate-ai-sdk-v7                [-]`; at 58 gives
        `> add-token-refresh                                  [4/9]`,
        `  fix-empty-basket                                   [7/7]`,
        `  migrate-ai-sdk-v7                                    [-]`.
      - `progress_cell_is_dash_when_total_is_zero` — at 38 and 58 the `[-]` cell's last
        character is in the final column, the same column as `[4/9]`'s.
      - `every_row_is_exactly_the_requested_width` — every `Row.text` from every fixture in
        this module is `width` characters at 38 and at 58, counted by `chars()`.
      - `a_long_name_is_truncated_with_an_ellipsis` — the 46-character name at 38 gives
        `  a-very-long-change-name-that-… [2/5]`; at 58 gives
        `  a-very-long-change-name-that-will-not-fit-here     [2/5]` with no `…`.
      - `a_narrow_width_drops_the_progress_cell_whole` — `alpha` at 4/9 selected: width 10 →
        `> a… [4/9]`; width 9 → `> … [4/9]`; width 8 → `> alpha `; width 3 → `> …`; width 2
        → `> `; width 1 → `>`; width 0 → ``; widths 38 and 58 still carry `[4/9]`. Widths 3
        and 2 are the boundary the spec's "below two columns" clause names, and the one a
        1-and-0-only test walks past.
      - `archived_narrow_widths_drop_progress_then_date` — `add-auth` at 7 of 7 dated
        `2026-08-14`, selected, at widths 20, 19, 14, 13, 3, 1, 0 → exactly
        `> 2026-08-14 … [7/7]`, `> 2026-08-14 add-a…`, `> 2026-08-14 …`, `> add-auth   `,
        `> …`, `>`, ``; and at 38 and 58 the full row with both the date and `[7/7]`. Assert
        each row's **character count equals the width** as well as its text: the
        drop-the-progress branch reclaims the separating space too, so the name field is
        `width - 13` there and `width - 14 - len(progress)` otherwise, and an off-by-one is
        invisible at 38 and 58.
      - `archived_rows_carry_a_ten_column_date_field` — at 38
        `  2026-08-14 add-auth            [7/7]`; at 58
        `  2026-08-14 add-auth                                [7/7]`.
      - `an_undated_archived_row_uses_ten_spaces` — at 38
        `             legacy-cleanup      [3/3]`; at 58
        `             legacy-cleanup                          [3/3]`; and the name begins in
        the same character index as the dated row's at both widths.
      - `the_separator_fills_the_width` — at 38 `  -- archived ------------------------`;
        at 58 `  -- archived --------------------------------------------`; and at width 6
        the first six characters of `  -- archived `.
      - `the_separator_is_emitted_only_when_archived_rows_follow` — no archived changes at
        38 and 58 → no `Separator` row; adding one → exactly one.
      - `row_order_is_problems_then_active_then_separator_then_archived` — the `RowKind`
        sequence at 38 and at 58 for a fixture carrying one problem, two active, and two
        archived.
      - `rows_preserve_the_change_set_order` — a deliberately unsorted fixture
        (`zeta`, `alpha`, `mid`) comes back in that order at 38 and 58: `rows` sorts nothing.
      - `the_selected_flag_marks_exactly_the_selected_change` — `selected` 3 over five
        visible: exactly one `Row.selected` is true, it is the fourth `Item`, and the
        `Separator` and `Problem` rows are never selected, at 38 and 58.
      - `the_no_repository_block_is_three_rows` — `repo: None`, `searched_from` the
        seventy-character path: at 38 the three rows are `No OpenSpec repository found`,
        `searched from:`, `…os/a-rather-long-repository-name-here`; at 58 the third is
        `…kspaces/openspec-demos/a-rather-long-repository-name-here`; and no `Item` row is
        emitted at either width.
      - `the_four_message_states_are_distinct` — at 38 and 58: empty set + empty query →
        `No changes yet`; empty active + one archived → `No active changes` + separator +
        row; non-empty query with no match → `No changes match` then `/zzz`; a matching
        query → no `Message` row at all.
      - `problem_rows_are_truncated_to_the_width` — the 49-character problem at 38 gives
        `! openspec/changes: Permission denied…`; at 58 gives
        `! openspec/changes: Permission denied (os error 13)` padded with seven spaces.
      - `rows_never_panic_at_any_width` — every fixture in this module rendered at
        `0, 1, 2, 8, 9, 10, 13, 14, 38, 58, 120, u16::MAX`, asserting only that each row's
        character count equals the width.
      Covers change-rows' first three requirements at the unit tier.
      **Red when:** `src/ui/list.rs` does not exist.

- [ ] 4.4 GREEN: Create `src/ui/list.rs` with `RowKind`, `Row`, and
      `pub fn rows(dashboard: &Dashboard, width: u16) -> Vec<Row>`, and register it in
      `src/ui/mod.rs`. Write the width arithmetic as an explicit **sequence of
      subtractions** from `width` — marker, date field (archived only), separator space,
      progress cell — so `agent-attribution` adds one more subtraction rather than rewriting
      it. Names truncate from the right, the no-repository path from the left.

- [ ] 4.5 REFACTOR: Extract the pad-or-truncate-to-a-field helper used by the name field,
      the problem row, and the message rows, so there is exactly one implementation of
      right-truncation in the file.

- [ ] 4.6 VERIFY: `testcount --lib 'ui::list::tests::' 17` and
      `testcount --lib 'changes::fixture::tests::' 2`. Then run `LISTWIDTHS` (default
      `LIST_MIN=17`) — it must now **pass**, having failed at 1.2 for the stated reason —
      and `NOIO-VIEW`, which must now pass with `5 pure files`. Then `NOLIT-CHANGE` and
      `GATE-MECH1`, both of which must still pass with the fixture module added. Then the
      four intermediate-gate commands.
      **Red when:** `LISTWIDTHS` names a test that omits 38 or 58; `NOLIT-CHANGE` reports a
      literal outside `src/changes.rs`, meaning the fixture was written in the wrong file;
      or `NOIO-VIEW` reports an I/O API in `list.rs`.
      Commit: `feat(list-view): add ui::list — the row grammar at 38 and 58 columns`.

---

## 5. `ui::view` — drawing the rows, the slice, and the selection
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing tests in `src/ui/view.rs`'s `mod tests`, **every one rendering
      at both 60 and 120** (`WIDTHS` enforces it):
      - `list_rows_render_at_60_and_120` — the exact 38- and 58-column strings from 4.3 as
        buffer rows 2, 3, 4 at columns 1..=38 and 1..=58, plus interior rows 5..=17 all
        spaces at both widths.
      - `a_long_name_is_truncated_at_38_but_not_at_58` — row 2 at 120 ends `…` before the
        progress cell; at 60 the full name appears and `…` appears nowhere in the buffer.
      - `separator_and_archived_rows_render_at_both_widths` — the four-row sequence at both.
      - `no_archived_changes_means_no_separator` — `-- archived` absent at both; adding an
        archived change makes it appear at both.
      - `no_repository_names_the_directory_searched` — rows 2, 3, 4 at both, plus row 1
        still holding `┌` and the title `Changes`.
      - `a_repository_with_no_changes_says_so` — row 2 begins `No changes yet` at both;
        `-- archived`, `No active changes`, and `No changes match` absent at both.
      - `no_active_changes_keeps_archived_browsable` — the three-row sequence at both;
        `No changes yet` absent at both.
      - `repository_problems_are_named_above_the_rows` — row 2 at 120 is
        `! openspec/changes: Permission denied…`; at 60 the untruncated form with no `…`;
        row 3 is the `fix-empty-basket` row at both.
      - `the_detail_region_stays_blank_while_the_list_fills` — at 120 every cell in rows
        2..=17, columns 41..=118 is a space with `Cell::default().style()`; at 60 no cell of
        column 0 or 59 in rows 2..=17 is anything but a border character.
      - `the_narrow_detail_route_draws_no_rows` — at 60 with `Route::Detail` the three names
        are absent; at 120 all three appear.
      - `more_changes_than_rows_do_not_overflow` — thirty changes, `selected` 0: rows 2..=17
        hold `change-00`..`change-15` at both widths; rows 0, 1, 18, 19 hold no change name.
      - `the_selected_row_carries_the_marker_and_bold` — interior row 0 column 1 is `>`,
        every cell of that row is BOLD, and no cell of interior row 1 is, at both widths.
      - `navigation_moves_the_marker` — after two `SelectNext`, `>` and BOLD move to the
        third interior row at both widths.
      - `selection_clamps_at_both_ends_on_screen` — after four `SelectNext` over three
        changes, the marker is on the third row at both widths.
      - `selection_crosses_the_separator` — one active, two archived, two `SelectNext`: the
        marker is on the `legacy-cleanup` row and the separator row's first column is a
        space and its cells are not bold, at both widths.
      - `an_empty_visible_list_draws_no_marker` — `No changes yet` row's first column is a
        space and no interior cell is bold, at both widths.
      - `a_selection_past_the_interior_scrolls_the_slice` — thirty changes, `selected` 20:
        first interior row is `change-12`, last is `change-27`, and `change-20`'s row is
        marked and bold, at both widths.
      - `the_last_change_is_reachable` — `selected` 29: first is `change-14`, last is
        `change-29`, the last is marked, no interior row is blank, at both widths.
      - `the_viewport_boundary_is_rendered` — seventeen changes, `selected` 9: first
        interior row is `change-01` at both widths.
      - `resizing_changes_the_slice_on_the_next_frame` — one `Terminal<TestBackend>` at
        120x20 then resized to 120x12, same `Dashboard`: first interior row moves from
        `change-12` to `change-16`; the 60-column control renders the same dashboard
        separately so the test still names both mandated widths.
      - `rows_do_not_overwrite_the_borders` — at 60 every cell of columns 0 and 59 in rows
        1..=18 is a box-drawing character; at 120 the same for columns 0, 39, 40, 119.
      Covers change-rows' four requirements and list-selection's two at the view tier.
      **Red when:** the interiors are blank, so every row assertion fails against spaces.

- [ ] 5.2 GREEN: Implement `render_list(frame, interior, dashboard)` in `src/ui/view.rs`:
      call `list::rows(dashboard, interior.width)`, find the index of the selected row,
      call `layout::viewport(rows.len(), cursor, interior.height)`, and `set_string` each
      row of the slice at the interior's first column, applying `Modifier::BOLD` to the
      selected row and `Style::default()` to the rest. Draw nothing when the interior has
      zero width or zero height.

- [ ] 5.3 GREEN: Call `render_list` from `render_body` for the list region only, using
      `Block::inner` on the region's area so the borders are never overwritten.

- [ ] 5.4 REFACTOR: Extract the tests' two interior geometries into one helper
      (`interior_cols(buf, y)` returning the 38 or 58 characters for the buffer's width) so
      the *expectations* are written out per test but the geometry is derived once.

- [ ] 5.5 VERIFY: `testcount --lib 'ui::view::tests::' 38` — the 17 measured at `main` plus
      this group's 21 — then `WIDTHS` with `WIDTHS_MIN=38`, which must pass. Then the four
      intermediate-gate commands.
      **Red when:** `WIDTHS` names a new test that omits 60 or 120, or the count is below 38.
      Commit: `feat(list-view): draw change rows, the slice, and the selection`.

---

## 6. `ui::view` — the footer's filter forms and the rewritten shell scenarios
<!-- kind: behavior -->

- [ ] 6.1 RED: Write failing tests in `src/ui/view.rs`'s `mod tests`, each at 60 and 120:
      - `the_prompt_replaces_the_hints_while_filtering` — row 19 at 60 is exactly `/add_`
        plus fifty-five spaces; at 120 `/add_` plus one hundred and fifteen; `q quit`,
        `Enter detail`, and `Esc back` absent from row 19 at both.
      - `an_accepted_query_leads_the_hint_list` — row 19 at 60 is exactly
        `/add  q quit  Enter detail  Esc back` plus twenty-four spaces; at 120 the same
        thirty-six characters plus eighty-four; and the same dashboard with an empty query
        gives `q quit  Enter detail  Esc back` at both.
      - `a_prompt_longer_than_the_footer_keeps_its_tail` — a seventy-character query: at 60
        row 19 is sixty characters ending `_` and beginning `a`, not `/`; at 120 it is `/`
        plus seventy `a`s plus `_` plus forty-eight spaces.
      - `a_query_narrows_both_tiers` — the accepted query `add` over the five-change
        fixture: the three exact interior rows at 38 and at 58, and `fix-empty-basket`,
        `migrate-ai-sdk-v7`, `legacy-cleanup` absent at both.
      - `matching_ignores_case` — `ADD` and `Add` produce buffers equal to `add`'s at both
        widths.
      - `a_query_matching_only_an_archived_change` — `auth`: `No active changes`, separator,
        then the marked `add-auth` row, at both; `No changes match` and `No changes yet`
        absent at both.
      - `a_query_matching_nothing_names_itself` — `zzz`: `No changes match` then `/zzz`, no
        `-- archived`, no change name, no `>` in the interior's first column, at both.
      - `shrinking_the_visible_list_clamps_the_marker` — after `/`, `a`, `d`, `d` from
        `selected` 4, the marker is on the `add-auth` row at both widths.
      - `slash_starts_filter_mode_and_the_list_is_shown` — from `Route::Detail`, applying
        `FilterStart` then rendering shows the `Changes` region at both widths.
      - `region_interiors_are_blank` — **rewritten** for responsive-layout →
        "Interiors are blank at both widths": at 120 the detail interior (rows 2..=17,
        columns 41..=118) is entirely spaces with `Cell::default().style()`; at 120 and 60
        the list interior's row 2 begins `No changes yet` and its rows 3..=17 are entirely
        spaces with that same style.
      - `one_column_frame_does_not_panic` — **extended** with a 2x20 render, so a
        zero-column region interior is exercised; the existing 1x1, 1x20, 60x20, and 120x20
        assertions are unchanged.
      - `header_says_no_repository` — **rewritten** for responsive-layout → "No repository
        found is named in the header at both widths". Its landed final assertion,
        `!buffer_contains(&buf, "/tmp/searched-from")`, is now **false by design**: the
        body's no-repository block names that path on purpose. Narrow it to row 0 — assert
        the *header row* of each buffer does not contain it, and additionally assert that
        interior row 4 **does**, so the test still discriminates instead of merely being
        weakened. The two `no repository` column assertions are unchanged.
      Covers list-filtering's three requirements at the view tier and responsive-layout's
      two rewritten scenarios.
      **Red when:** the footer still renders the three hints while filtering, and the
      rewritten interior test still finds a blank list interior.

- [ ] 6.2 GREEN: Implement the footer's two extra forms in `render_footer`, taking the
      `Filter` as an argument: the prompt form (hints replaced, tail kept) and the
      leading-hint form (`/` + query prepended to the hint list, same drop-from-the-end
      rule). The empty-inactive form is byte-for-byte what it is today.

- [ ] 6.3 REFACTOR: Extract `shorten_left(text, width)` — whole when it fits, else `…` plus
      the last `width - 1` characters, empty at width 0 — and refactor
      `shorten_for_header` onto it, so the crate holds exactly one left-truncation and one
      right-truncation implementation. The header's landed scenarios must stay green
      unchanged; if any moves, the refactor is wrong.

- [ ] 6.4 VERIFY: The eight `responsive-layout` tests this change does **not** alter
      must still pass, unmodified, and their tests are the evidence for those seven matrix
      rows: run `cargo test --all-features --lib ui::view::tests::frame_rows_at_60_and_120
      ui::view::tests::one_row_frame_draws_header_only` and the rest by name —
      `two_row_frame_draws_no_body`, `footer_drops_whole_hints`,
      `routed_region_border_is_bold`, `header_path_right_aligned_whole`,
      `header_path_shortened_from_the_left`, and `header_omits_the_path_when_too_narrow` —
      and confirm each ran and passed. Two of them survive only for a reason worth
      recording rather than rediscovering: `header_path_shortened_from_the_left` and
      `header_omits_the_path_when_too_narrow` both assert no `…` appears anywhere in the
      buffer, and they stay green because the body renders `No changes yet`, which is
      exactly fourteen characters and fits the 14-column interior a 16-column frame leaves.
      **Red when:** any of the eight fails, which would mean this change altered a landed
      requirement it claims not to.
      **Wrongly green when:** a name is misspelled and the filter matches nothing — count
      the tests the run reports, do not read its exit status.

- [ ] 6.5 VERIFY: `testcount --lib 'ui::view::tests::' 47`, then `WIDTHS` with
      `WIDTHS_MIN=47` — this is the change's final view floor. Then the four
      intermediate-gate commands.
      **Red when:** the count is below 47, or `WIDTHS` names a test omitting a width.
      **Wrongly green when:** `WIDTHS_MIN` is left at its 16 default, which would pass with
      the tests deleted — guarded by passing the value explicitly and recording the
      `WIDTHS OK: all 47 …` line.
      Commit: `feat(list-view): render the filter prompt and rewrite the shell scenarios`.

---

## 7. `ui::driver` and `ui::load` — the mode reaches the key map
<!-- kind: behavior -->

- [ ] 7.1 RED: Write failing tests for:
      - `ui::driver::tests::filter_mode_is_passed_to_action_for` — drive `run_loop` at 60x20
        with a script of Press `Char('/')`, Press `Char('q')`, Press `Char('c')`+CONTROL.
        Assert `Ok(LoopSummary { frames: 3, polls: 3 })`, `dashboard.filter.query == "q"`,
        and that the loop did **not** quit on the `q`. Also render the same dashboard at
        120x20 through `testutil::render_at` so the test names both mandated widths.
      - `ui::tests::load::` — the module holds four landed tests plus group 0's acceptance
        test. Extend the two that inspect a returned `Dashboard`
        (`a_scratch_repository_is_loaded_from_files` and `no_repository_above_the_start`) to
        assert `selected == 0` and
        `filter == Filter { query: String::new(), active: false }`. The other two
        (`archived_count_from_config_is_honoured`, `loading_writes_nothing`) inspect
        `changes` and the tree and need no change.
      Covers dashboard-loop → "Startup state is read from files only" and list-filtering →
      "`q` types a character while filtering and does not quit" at the loop tier.
      **Red when:** `run_loop` still calls the one-argument `action_for` and the crate does
      not compile, or `load` does not set the two fields.

- [ ] 7.2 GREEN: Change `run_loop`'s single call site to
      `action_for(&event, dashboard.filter.active)`, and `ui::load`'s two `Dashboard`
      literals to name `selected: 0` and `filter: Filter { query: String::new(), active:
      false }`.

- [ ] 7.3 REFACTOR: None expected. State that rather than inventing a change.

- [ ] 7.4 VERIFY: `testcount --lib 'ui::driver::tests::' 8`.
      **Do not** run `testcount --lib 'ui::tests::load::' 5` here: group 0's acceptance test
      lives under that filter and is still RED until 8.1, so `testcount` would fail on its
      exit guard (and, even past it, `test result: FAILED.` does not match the `sed` that
      reads the count). Instead run
      `cargo test --all-features --lib ui::tests::load:: 2>&1` and confirm the summary reads
      **4 passed; 1 failed**, with the one failure being exactly
      `a_scratch_repository_renders_its_change_rows`. The counted form runs at 8.1, once the
      acceptance test is green.
      **Red when:** the driver count is short, or the load module reports any failure other
      than the single known one — a second failure means 7.2 broke a landed test.
      Commit: `feat(list-view): pass the filter mode through the loop and startup state`.

---

## 8. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 8.1 VERIFY: Confirm group 0's acceptance test now passes end to end.
      `cargo test --all-features --lib
      ui::tests::load::a_scratch_repository_renders_its_change_rows` — green, with exactly
      one test run under the filter.
      Then, now that the module is fully green, the counted form 7.4 had to defer:
      `testcount --lib 'ui::tests::load::' 5`.
      **Red when:** any of the ten exact row strings differs, or the count is below 5.
      Record the diff verbatim; a mismatch here is a row-grammar bug, not a fixture bug,
      until 8.2 proves otherwise.

- [ ] 8.2 VERIFY: The literal, unqualified `make check` — the first time in this change it
      is runnable. All four gates green at the unchanged 80% floor.
      **Red when:** any sub-command fails; name the failing one rather than reporting a
      summary.

- [ ] 8.3 REFACTOR: Clean up the acceptance test's fixture setup if it duplicates the
      `ScratchDir` helpers the other `load` tests already use; otherwise state that none was
      needed.
      Commit: `test(list-view): close the outer loop — a real repository renders real rows`.

---

## 9. Architectural and dependency checks
<!-- kind: operational -->

- [ ] 9.1 CHECK: Run every check against the finished tree and record each output verbatim:
      `NOSPAWN-GREP` with `MIN=16`, `NOIO-VIEW`, `NOCLI-SHELL` with `UI_MIN=8`,
      `NORAW-GREP`, `NODEFAULT-UI`, `NOLIT-CHANGE` with `MIN=16`, `WIDTHS` with
      `WIDTHS_MIN=47`, `LISTWIDTHS` with `LIST_MIN=17`, `NOWAIVER`, `GATE-MECH1`,
      `NOJSON-SEAM`, `DEPS` (full, **without** `DEPS_SKIP_LEG5` — this is the one run that
      exercises leg 5's four removal experiments), and `GRAPH-SNAP`.
      **Red when:** any fails. `NOSPAWN-GREP` at `MIN=16` and `NOCLI-SHELL` at `UI_MIN=8`
      both **failed** at task 1.3 by construction, so their passing here is evidence the
      file was added rather than evidence the floor is decorative.
      **Wrongly green when:** `DEPS` or `GRAPH-SNAP` is skipped on the reasoning that no
      dependency changed — run them; the point is that nothing changed.

- [ ] 9.2 CHECK: Prove each check can still go **red** against a planted violation, in a
      throwaway copy under `$WORK`, then remove it. Extract and run the same `$CHECKS`
      scripts, never a retyped variant:
      1. `let _ = std::process::Command::new("ls");` in `src/ui/list.rs` → `NOSPAWN-GREP`
         and `NOIO-VIEW` both fail, naming the file and line.
      2. `use std::fs;` in `src/ui/list.rs` → `NOIO-VIEW` fails.
      3. `let _ = crate::changes::from_cli;` in `src/ui/list.rs` → `NOCLI-SHELL` fails.
      4. `impl Default for Filter { … }` in `src/ui/app.rs` → `NODEFAULT-UI` fails naming
         `Filter`, proving the type list is read rather than only `Dashboard`.
      5. `let Filter { query, .. } = f;` in `src/ui/app.rs` → `NODEFAULT-UI` half B fails.
      6. `let d = Dashboard { quit: false, .. };`-shaped elision → half B fails for
         `Dashboard` too.
      7. A `ChangeSet { active: vec![], archived: vec![], problems: vec![] }` literal in
         `src/ui/view.rs` → `NOLIT-CHANGE` fails, naming the file and line.
      8. A new view test in `src/ui/view.rs` naming only one width, written across **three
         lines** with `#[test]` alone on its own line:
         ```
             #[test]
             fn only_at_60() { let _ = render_at(60, 20, &d); }
         ```
         → `WIDTHS` fails naming `only_at_60`. The line break is load-bearing and was
         reproduced during planning: both width checks split on a **line-anchored**
         `^[ \t]*#\[test\][ \t]*$`, so the same plant written as one line creates no part
         and the check passes — a false negative control on the two checks this change adds.
      9. The same shape in `src/ui/list.rs`, `#[test]` again on its own line:
         ```
             #[test]
             fn only_at_38() { let _ = rows(&d, 38); }
         ```
         → `LISTWIDTHS` fails naming `only_at_38`.
      10. `--fail-under-lines 70` in `Makefile` → `NOWAIVER` fails.
      11. `enable_raw_mode()` in `src/ui/view.rs` → `NORAW-GREP` fails.
      Every plant is a **source-text** edit in a throwaway copy that is never compiled —
      items 5, 6, and 7 are not valid Rust without a `Default`, and that is fine: these
      checks are greps, and the property under test is that the grep sees the text.
      Record each failure message verbatim. Delete `$WORK`'s copy afterwards and confirm
      `git status --porcelain` is clean of it.
      **Red when:** any planted violation is **not** caught. A check that stays green
      against its own violation is the defect class this repository has shipped three times.

- [ ] 9.3 CHECK: Confirm no test in the change reaches a collaborator the design's Test
      Boundaries table does not name. Grep `src/ui/list.rs` and `src/ui/view.rs` for
      `ScratchDir`, `temp_dir`, `std::fs`, and `Command` — expected: **no hits in either**.
      Grep `src/ui/mod.rs` for `render_at` — expected: exactly the acceptance test.
      **Red when:** a view test opens a directory, which is the signal that logic leaked
      into the view.

- [ ] 9.4 VERIFY: `OPENSPEC-UNTOUCHED` against `$BASE`. The only permitted paths under
      `openspec/` are `openspec/changes/list-view/`.
      **Red when:** any other path appears — including an untracked one, which is the half
      `git diff` alone would miss.
      Commit: `test(list-view): run the architectural check suite and its negative controls`.

---

## 10. Documentation
<!-- kind: operational -->

- [ ] 10.1 Rewrite in `SPEC.md`: **User interface → List view** (audience: every future
      change that renders a row). Replace the 48-character mock with the real 38-column
      rendering and state the row grammar in prose — marker, name field, right-aligned
      progress cell, and the archived row's ten-column date field — because the landed mock
      cannot occur at the width `tui-shell` froze, and `agent-attribution` needs to know
      which field it subtracts from. Also annotate the mock's third column as
      `agent-attribution`'s and `degraded-states`', with the explicit statement that
      `list-view` reserves no width for it, and annotate the unattributed-agents footer
      sentence as `agent-attribution`'s so its absence today is not read as an omission.
      This **replaces** the existing mock and its two-sentence caveat; net growth roughly
      ten lines, all of it the grammar that was previously undecided.

- [ ] 10.2 Rewrite in `SPEC.md`: **User interface → Keys** (audience: the same). Correct
      **five rows** and one sentence, in place. A reader consults this table one row at a
      time, so a qualification written only on the `/` row does not reach someone looking up
      `q`; each affected row carries its own:
      - `/` gains the filter-mode semantics (printable keys type, `Backspace` deletes,
        `Enter` accepts, `Esc` cancels, `Ctrl-C` still quits) and that `/` moves the route
        to the list.
      - `q | Quit` gains "except while filtering, where it types a `q`" — the landed
        unqualified row is what correction 4 calls the contradiction, so the fix belongs
        here and not only on the `/` row.
      - `Enter | Open change detail` gains "or accepts the filter, while filtering".
      - `Esc | Back to list` gains the layered dismissal — corrected in the **row**, not
        only in the sentence below the table, because leaving the two disagreeing inside one
        section is worse than leaving both stale.
      - `j`/`k`/arrows gain what they navigate: the list selection, clamped rather than
        wrapping, with the list scrolling to keep it visible — **and** that the four part
        company in filter mode, where the arrows still navigate and `j` / `k` type
        themselves. The landed row groups them as one behaviour, which this change makes
        false.
      - and the sentence "`Esc` at the list root … is inert rather than a quit" is rewritten
        as the layered dismissal, which keeps it true while naming the layers.

- [ ] 10.2b Rewrite in `SPEC.md`: three sentences elsewhere that this change falsifies
      (audience: every future change; each is a one-line edit, no net growth):
      - **User interface → List view:** "Progress comes from the CLI when available and from
        checkbox counts otherwise" — false of what the pane renders, because `NOCLI-SHELL`
        forbids `src/ui/` naming the CLI seam. Qualify it: the rendered list is file-sourced
        until `live-refresh` wires the correction in; the dual-source model is still the
        design.
      - **User interface → Responsive layout:** "`Enter` and `Esc` move the route at
        **every** width" — the sentence `tui-shell` froze *for `list-view` to inherit*, and
        the one this change falsifies in filter mode, where `Enter` accepts and `Esc`
        cancels without touching the route, and where `/` now also moves it. Correct in
        place; leave the `Length(40)` / `Min(0)` half of the freeze untouched.
      - **Architecture → Module map, the `ui` row:** "Views, layout, key handling, terminal
        lifecycle, and the event loop" gains the row grammar and the dashboard's own state,
        which are now the bulk of the module.
      - **Testing and quality gates → Unit-tested modules:** add `ui::list` to the `ui`
        bullet, which lists `ui::layout`, `ui::app`, `ui::view`, `ui::driver`,
        `ui::terminal`, and `ui::load`.
      - **Fixtures:** narrow "every view change performs no I/O at all" to "every view
        *test*". It is already false of `tui-shell`'s own `ui::tests::load::` tests, and this
        change's acceptance test opens a `ScratchDir` on purpose. Add the clause that keeps
        the seam's claim true: `render` takes a value, never a path, and `NOIO-VIEW` proves
        it.

- [ ] 10.3 Add to `SPEC.md`: **Degraded states** (audience: `degraded-states`, which audits
      this table row by row). One new row for a filter matching no change, and one clause
      added to the existing `openspec/changes/`-unreadable row naming *where* the reason on
      `ChangeSet::problems` is rendered — previously unassigned to any change, which would
      have made `degraded-states`' "confirm, do not add" principle false.

- [ ] 10.4 Rewrite in `AGENTS.md`: **Current repo state** (audience: every session).
      Replace the sentence "The body regions themselves are still empty bordered frames —
      `list-view`, `markdown-viewer`, and `detail-view` fill them next" with what the list
      now shows, and add one line to **Architecture rules** naming the two interior widths
      (38 and 58) and that row-grammar tests assert both — a durable constraint a future
      change would otherwise rediscover by breaking it. Net: one sentence rewritten, one
      line added; nothing appended beside a rule it supersedes.

- [ ] 10.4b Record, in this file and in `planning-review.md`, the one edit that must be made
      at **archive** time rather than now: `openspec/IMPLEMENTATION-ORDER.md`'s `list-view`
      row does not mention repository-level `ChangeSet::problems`, which this change
      deliberately renders (design.md → Decisions says why). It cannot be edited here —
      `OPENSPEC-UNTOUCHED` would fail, and correctly so — and `openspec/config.yaml`'s
      archive guidance covers correcting a row that "split, merged, or moved" but not one
      that widened, so nothing would otherwise prompt it. Naming it here is what makes the
      archive step catch it. The same row's **Spec refs** cell reads "User interface → List
      view" and needs "Degraded states" alongside it, for the same reason.

- [ ] 10.5 CHECK (contract gate): Re-read `SPEC.md` → Manifest and the `config.toml` format
      and confirm neither changed. This change touches no manifest key and no configuration
      key, so the documented formats must still match `herdr-plugin.toml` and
      `src/config.rs` byte for byte.
      **Red when:** either drifted, which would mean this change altered a format it claims
      not to.

- [ ] 10.6 VERIFY: `OPENSPEC-UNTOUCHED` again — 10.1–10.4 edit files outside `openspec/`,
      so the result must be identical to 9.4's.
      **Red when:** a documentation edit landed under `openspec/` outside this change's own
      directory.
      Commit: `docs(list-view): correct SPEC.md's list mock, keys, and degraded states`.

---

## 11. Change Review
<!-- kind: operational -->

- [ ] 11.1 CHECK: Dispatch an **independent** reviewer — a fresh subagent, never a fork of
      the implementing session — against `proposal.md`, all five spec files, `design.md`,
      `tasks.md`, and the diff. Give it the artifacts and the diff, not this session's
      reasoning. Concentration points, in order:
      1. **Tests that cannot fail.** For each of the 56 spec scenarios, name the test that
         would go red if the behaviour were deleted. Re-run 9.2's planted violations
         independently rather than trusting the recorded output.
      2. **Both widths, genuinely.** Every view test names 60 and 120 and every row test
         names 38 and 58 — but does each *assert* at both, or does one width appear only in
         a comment or an unused literal? Read the assertions.
      3. **The render seam.** No view test opens a directory; `NOIO-VIEW`'s five-file set is
         the real set; the acceptance test's real filesystem is confined to `ui::load`.
      4. **The `openspec/` write invariant.** `OPENSPEC-UNTOUCHED` was run against a base
         SHA including untracked files, not against the index.
      5. **Floors that measure.** Every `testcount`, `MIN`, `UI_MIN`, `WIDTHS_MIN`, and
         `LIST_MIN` is *measured baseline + this change's addition*, and each was proven to
         fail before the group that satisfies it.
      6. **Drift.** Any contract the implementation changed without refreshing `design.md`,
         the specs, or `SPEC.md`.
      7. **Leftovers.** `dbg!`, `println!`, `todo!`, `TODO`, `FIXME`, commented-out code,
         and hardcoded widths that should have come from the interior.

- [ ] 11.2 CHANGE: Fix every CRITICAL. Resolve or consciously accept each WARNING with a
      one-line reason recorded in `planning-review.md`. Note each SUGGESTION. Re-run the
      affected tests and any check whose input changed.

- [ ] 11.3 VERIFY: Confirm no blocking or unowned finding remains, and that the finding
      counts are recorded in `planning-review.md` alongside the planning-time findings.
      Commit: `review(list-view): address Change Review findings`.

---

## 12. Lint & Verify
<!-- kind: operational -->

- [ ] 12.1 CHECK: Inspect the intended verification commands and affected tiers. The gate is
      `make check`; the tiers are `--lib` unit and view tests only — this change adds no
      `tests/cli.rs` case and the binary's behaviour is unchanged.

- [ ] 12.2 VERIFY: `cargo fmt --all -- --check` — clean.

- [ ] 12.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings.
      (This repository has no separate type checker: `cargo clippy` builds the crate, so a
      type error fails here.)

- [ ] 12.4 VERIFY: `cargo test --all-features` — green. Then the counted floors, so a
      renamed module cannot pass as a green run: `testcount --lib 'ui::app::tests::' 22`,
      `testcount --lib 'ui::layout::tests::' 9`, `testcount --lib 'ui::list::tests::' 17`,
      `testcount --lib 'ui::view::tests::' 47`, `testcount --lib 'ui::driver::tests::' 8`,
      `testcount --lib 'ui::tests::load::' 5`,
      `testcount --lib 'changes::fixture::tests::' 2`, and
      `testcount --lib '' <measured baseline + 67>`. Record the final lib total.

- [ ] 12.5 VERIFY: `make coverage` — `cargo llvm-cov --fail-under-lines 80` at the unchanged
      floor. Record the TOTAL percentage and the uncovered-line count, and compare against
      1.1's 98.01% over 8,836 lines: a fall of more than about one point is a finding, not a
      pass.

- [ ] 12.6 VERIFY: `make check` as the single gate — all four in order. If it fails, name
      the failing sub-command rather than reporting a summary.

- [ ] 12.7 VERIFY: Re-run the whole architectural suite one last time against the final
      tree: `NOSPAWN-GREP` with `MIN=16`, `NOIO-VIEW`, `NOCLI-SHELL` with `UI_MIN=8`,
      `NORAW-GREP`, `NODEFAULT-UI`, `NOLIT-CHANGE` with `MIN=16`, `WIDTHS` with
      `WIDTHS_MIN=47`, `LISTWIDTHS` with `LIST_MIN=17`, `NOWAIVER`, `GATE-MECH1`,
      `NOJSON-SEAM`, `DEPS` (with `DEPS_SKIP_LEG5=1`; 9.1 ran it in full), `GRAPH-SNAP`,
      and `OPENSPEC-UNTOUCHED` against `$BASE`.

- [ ] 12.8 VERIFY: `openspec validate list-view --strict` — valid. The `openspec` binary is
      nvm-installed: `export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"` first.
      Commit: `docs(list-view): record group 12 lint & verify evidence — apply complete`.
