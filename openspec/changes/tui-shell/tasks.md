# tui-shell — tasks

Read `design.md` first: the Boundaries table, the Test Boundaries table, and the
verification matrix are the contract these tasks implement. No task may invent a
collaborator that table does not name.

**The outer loop is taken.** design.md → Test Strategy says why: `main` → `ui::run` →
terminal check → guard → loop is a path no unit test crosses, and getting it wrong in the
direction that matters puts the developer's own terminal into raw mode from `cargo test`.
Group 0 writes that failing test first; group 8 closes it.

**No parallel groups.** Groups 2–8 form a dependency chain through `src/ui/`: `view`
needs `layout` and `app`, `driver` needs `view`, `run` needs all of them. Nothing here is
independent in the sense the schema requires.

## Test-module convention — read before writing any test

`cargo test` matches a filter against a test's **full path**, and a filter matching nothing
exits 0. Every filtered gate below therefore addresses tests through an explicit module
path, and is judged on a counted minimum rather than on the run's exit status:

| Group | File | Module | Filter |
|---|---|---|---|
| 2 | `src/ui/layout.rs` | `mod tests` | `ui::layout::tests::` |
| 3 | `src/ui/app.rs` | `mod tests` → `mod keys` | `ui::app::tests::` |
| 4 | `src/lib.rs` | `mod testutil` → `mod tests` | `testutil::tests::` |
| 4 | `src/ui/view.rs` | `mod tests` | `ui::view::tests::` |
| 5 | `src/ui/terminal.rs` | `mod tests` → `mod guard` | `ui::terminal::tests::` |
| 6 | `src/ui/driver.rs` | `mod tests` | `ui::driver::tests::` |
| 7 | `src/ui/mod.rs` | `mod tests` → `mod load` | `ui::tests::load::` |
| 8 | `src/ui/mod.rs` | `mod tests` → `mod start` | `ui::tests::start::` |
| 0, 8 | `tests/cli.rs` | — | none — an integration test's path is its bare function name, so `cli::` matches **nothing**; the `--test-cli` scope below runs the whole binary and counts it |

---

## Command-level checks, written out once

Referenced by label from the tasks below. They live here rather than in a table cell
because an unescaped `|` cannot appear in a Markdown table and an escaped `\|` inside an
ERE matches a literal pipe, so the check would pass against the very code it exists to
catch. Every one is judged on **output emptiness, a counted minimum, or a specific error
code**, never on a bare pipeline exit status: `grep` exits 1 for no-match and 2 for a bad
file, and `!` turns both into a pass.

Extract each block to `$CHECKS/<LABEL>.<ext>` in task 1.1 and run the extracted,
byte-identical file thereafter, so the run and the record cannot drift.

```sh
# NOSPAWN-GREP — `cli` is the only module in the crate that spawns a process.
# Carried forward from changes-from-cli with ONE deliberate edit, recorded in design.md ->
# Boundaries: Guard C's file-count minimum is now MIN="${MIN:-8}" instead of a hardcoded 8,
# so the same block asserts 8 before this change and 14 after it. A count that never moves
# stops discriminating as the crate grows.
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
# NOIO-VIEW — the four pure files of the render seam name no I/O API at all.
# Three files under src/ui/ are deliberately NOT searched, each for a stated reason:
# mod.rs holds ui::load and ui::run, terminal.rs holds the terminal seam, and event.rs
# holds CrosstermEvents, which reads the real event stream. Do not "fix" the list by
# adding them. A view test that needs a real directory is the signal this check exists to
# make impossible.
PURE="src/ui/app.rs src/ui/layout.rs src/ui/view.rs src/ui/driver.rs"
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
echo "NOIO-VIEW OK: 4 pure files carry no I/O API; positive control matched"
```

```sh
# NOCLI-SHELL — the dashboard shell never names the CLI seam. This is the mechanical form
# of the roadmap's "tui-shell depends on changes-from-files, not on the CLI".
CLI_RE='from_cli|OpenspecCli|HerdrCli|CliChanges|npm_prefix'
UI_MIN="${UI_MIN:-7}"

# Guard A — src/ui/ exists and holds Rust files. A count of zero would make the search
# vacuous. Parameterised for the same reason NOSPAWN-GREP's MIN is: the end state has
# exactly 7 files, and a later change that merges two of them would turn a real check into
# a spurious failure, which is how a check gets weakened into a rubber stamp.
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
# NODEFAULT-UI — Dashboard has no Default and no construction or destructuring elides a
# field. change-model's GATE-MECH1 covers Change/ChangeSet/ArtifactRef/Origin in
# src/changes.rs ONLY, so a new type in src/ui/app.rs is outside it. This is the same
# shape, scoped to Dashboard, and searches all of src/ because `impl Default for Dashboard`
# is legal in any file of the crate.
SRC="${SRC:-src}"
[ -d "$SRC" ] || { echo "NODEFAULT-UI FAIL: no such directory: $SRC" >&2; exit 1; }
[ -f "$SRC/ui/app.rs" ] || { echo "NODEFAULT-UI FAIL: $SRC/ui/app.rs missing" >&2; exit 1; }

# Positive control — the file must actually declare the type, or every search below is
# searching for a name that is not there and a clean result means nothing.
grep -q 'struct Dashboard' "$SRC/ui/app.rs" \
  || { echo "NODEFAULT-UI FAIL: positive control - $SRC/ui/app.rs has no 'struct Dashboard'" >&2
       exit 1; }

# Half A — no Default impl anywhere under src/, hand-written or derived. The derive form is
# caught by taking the two lines preceding each `struct Dashboard` and looking for Default
# inside a #[derive(...)] there.
a=$(find "$SRC" -name '*.rs' -print0 \
    | xargs -0 -I{} grep -nE 'impl[[:space:]]+Default[[:space:]]+for[[:space:]]+Dashboard' {} /dev/null 2>&1 || true)
b=$(find "$SRC" -name '*.rs' -print0 \
    | xargs -0 -I{} grep -B2 -n 'struct Dashboard' {} /dev/null 2>&1 \
    | grep 'derive' | grep 'Default' || true)
[ -z "$a$b" ] || { echo "NODEFAULT-UI FAIL: Dashboard has a Default:" >&2
                   printf '%s\n%s\n' "$a" "$b" >&2; exit 1; }

# Half B — no `..` inside a Dashboard literal or pattern. Matched on the same line, which is
# how rustfmt writes a short elision (`Dashboard { quit, .. }`); a multi-line elision is
# caught by the compile-time companion in app.rs's tests instead, which is the half a grep
# genuinely cannot do.
c=$(find "$SRC" -name '*.rs' -print0 \
    | xargs -0 -I{} grep -nE 'Dashboard[[:space:]]*\{[^}]*\.\.' {} /dev/null 2>&1 || true)
[ -z "$c" ] || { echo "NODEFAULT-UI FAIL: Dashboard literal or pattern elides a field:" >&2
                 echo "$c" >&2; exit 1; }
echo "NODEFAULT-UI OK: no Default for Dashboard, no elided field; positive control matched"
```

```sh
# NORAW-GREP — crossterm's terminal-mode functions appear ONLY in src/ui/terminal.rs, and
# CrosstermOps is constructed only by ui::run. Searches tests/ as well as src/, because the
# failure this prevents is a TEST putting the developer's own terminal into raw mode.
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
# GRAPH-SNAP — the resolved normal build graph equals the committed per-triple snapshot.
# Regenerate with: GRAPH_WRITE=1 sh GRAPH-SNAP.sh
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
allow="darling_macro derive_more-impl document-features indoc instability rustversion strum_macros thiserror-impl"
got=$(cargo tree -e normal --prefix none 2>/dev/null | grep '(proc-macro)' \
      | sed -e 's/ (\*)//' -e 's/ (proc-macro)//' | cut -d' ' -f1 | sort -u | tr '\n' ' ')
want=$(printf '%s\n' $allow | sort -u | tr '\n' ' ')
[ "$got" = "$want" ] || { echo "GRAPH-SNAP FAIL: proc-macro set is [$got], expected [$want]" >&2; exit 1; }

# Two named absences, each proving a default-feature decision is in effect rather than
# merely written down.
for absent in encoding_rs time; do
  if grep -qE "^$absent " "$TMP/all"; then
    echo "GRAPH-SNAP FAIL: $absent is in the normal build graph" >&2; exit 1
  fi
done
echo "GRAPH-SNAP OK: 4 triples match $SNAP; 8 proc-macro crates; no encoding_rs, no time"
```

```sh
# WIDTHS — every #[test] in src/ui/view.rs names both 60 and 120. A heuristic, and design.md
# -> Risks says so: it cannot prove an assertion is meaningful, only that both widths are
# present. It is the cheap half of the mandate; the per-scenario tasks are the other half.
#
# Known limits, stated rather than discovered later: a `60` in a comment satisfies it, and
# so does an unrelated `60` literal. It is a FLOOR. What proves the tests exist and run is
# `testcount --lib 'ui::view::tests::' 16` (task 4.6), not this.
[ -f src/ui/view.rs ] || { echo "WIDTHS FAIL: src/ui/view.rs missing" >&2; exit 1; }
python3 - <<'PY'
import re, sys
raw = open("src/ui/view.rs").read()
# Drop doc-comment and inner-doc lines before splitting: a `#[test]` inside a doc example
# would otherwise inflate the count and let a gutted file pass the >= 16 floor.
src = "\n".join(l for l in raw.splitlines()
                if not l.lstrip().startswith("///") and not l.lstrip().startswith("//!"))
# Split on a line-anchored attribute, so `#[test]` inside a string or a trailing comment
# does not create a part.
parts = re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", src)[1:]
if len(parts) < 16:
    print(f"WIDTHS FAIL: found {len(parts)} #[test] functions in src/ui/view.rs, expected >= 16",
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
# NOWAIVER — the coverage floor was not lowered, excluded, or annotated away.
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
# stray file anywhere else under openspec/ still fails.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'; } \
         | grep -v '^openspec/changes/tui-shell/' | sort -u || true)
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
# `--test-cli` exists because tests/cli.rs is an INTEGRATION test: its test paths are bare
# function names with no module prefix, so a filter like 'cli::' matches nothing at all and
# would be green forever. That scope runs the whole binary unfiltered and counts it.
# This block DEFINES a shell function; it must be SOURCED (`. $CHECKS/TESTCOUNT.sh`) in
# every shell that runs a gate, not executed with `sh`.
#   usage: testcount <scope> <filter> <minimum>   scope: --lib | --test-cli
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

**Carried forward unchanged** from `openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md`,
extracted byte-identically rather than retyped: `GATE-MECH1.py`, `NOJSON-SEAM`, and `DEPS`.
Task 1.1 extracts them from that file; task 1.5 makes and records the four edits `DEPS`
needs, in the extracted copy rather than in the archive.

---

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [x] 0.1 Set up the harness: no new harness is needed. `tests/cli.rs` already spawns the
      crate's own binary through `env!("CARGO_BIN_EXE_herdr-openspec")` and already polls to
      a deadline rather than sleeping. design.md → Test Boundaries names the crate's own
      binary as **real** in this tier and the terminal as **never reached**, because the
      spawn pipes stdout.

- [x] 0.2 RED: Add `tests/cli.rs::ui_without_a_terminal_exits_three` for
      plugin-build → "`ui` with stdout piped exits 3 without blocking". Spawn `ui` with
      stdout and stderr piped and **stdin attached to a pipe whose write end is left open**,
      so a wrongly blocking implementation hangs rather than reaching EOF. Poll `try_wait`
      to a ten-second deadline. Assert: exited before the deadline; `status.code() ==
      Some(3)`; stdout empty; stderr contains `herdr-openspec` and `not a terminal`.

- [x] 0.3 Confirm it fails because the behaviour is missing, not because the harness is
      misconfigured. Run `cargo test --all-features --test cli ui_without_a_terminal_exits_three`
      and record the message verbatim. **Expected today:** the process blocks on stdin, so
      the deadline expires and the run fails on `exited` — not on a status mismatch. Both
      distinguishable outcomes are acceptable evidence; a pass is not.
      **Red when:** the assertion fails for the absent behaviour. **Wrongly green when:** the
      filter matched nothing — guarded by naming the test explicitly and confirming the run
      reports `1 filtered out`-style arithmetic consistent with one test having run.
      **Recorded:** `thread 'ui_without_a_terminal_exits_three' panicked at tests/cli.rs:39:5:
      process blocked on stdin instead of exiting immediately` — the deadline expired, as
      predicted. `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered
      out` confirms exactly one test ran under the filter.

---

## 1. Baseline, checks, and the `ratatui` dependency
<!-- kind: operational -->

- [x] 1.1 CHECK: Record the starting state before any edit.
      - `git rev-parse HEAD` → `export BASE=<sha>`. Every `OPENSPEC-UNTOUCHED` run uses it.
        A diff against the index would pass over this change's own per-group commits.
      - `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
        passed.*/\1/p'` → the **lib** baseline. On `main` at planning time: **389**.
      - `cargo test --all-features --test cli` count → **5** at planning time.
      - `cargo llvm-cov --summary-only` → record TOTAL line coverage. At planning time:
        **98.93%** over 7,635 lines, 82 uncovered.
      - `export CHECKS=<scratchpad>/tui-shell-checks` and `export WORK=<scratchpad>/tui-shell-work`;
        `mkdir -p "$WORK"`. Extract every fenced block above to `$CHECKS/<LABEL>.sh`
        byte-identically, and extract `GATE-MECH1.py`, `NOJSON-SEAM.sh`, and `DEPS.sh` from
        `openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md` byte-identically
        (`DEPS` is a `sh` block there, not a `python` one — `GATE-MECH1` is the Python one).
        Run the extracted files from here on, never a retyped copy. `TESTCOUNT.sh` is the
        exception: it only *defines* a function, so **source** it (`. $CHECKS/TESTCOUNT.sh`)
        in each shell that runs a gate rather than executing it.
      **Red when:** `BASE` is empty or not a commit (`OPENSPEC-UNTOUCHED` aborts on both), or
      the lib baseline is recorded higher than reality, which makes every later `TESTCOUNT`
      unsatisfiable rather than falsely green.
      **Recorded:** `BASE=e25017546cea99e4b606fcd74a404131e1ca582e`
      (`docs(tui-shell): plan the ratatui shell, event loop, and 100-column breakpoint`).
      Lib baseline **389**. `--test cli` count **5**. Coverage **98.93%** over 7,635 lines,
      82 uncovered — exact match to the planning-time figures. Checks pre-extracted in the
      scratchpad from planning review were **stale** (predated the review repairs to
      `NOCLI-SHELL`, `NOIO-VIEW`, `WIDTHS`, and `TESTCOUNT`); re-extracted byte-identically
      from the current `tasks.md` and archive instead.
      **Correction recorded here, load-bearing for every later "then `make check`" in
      groups 1–7:** `make check`'s `test` and `coverage` steps run the whole suite
      unfiltered, and group 0's outer-loop acceptance test (`ui_without_a_terminal_exits_three`)
      is deliberately RED until group 8 closes it — `cargo llvm-cov` also hard-fails on any
      test failure and produces no report at all, `--no-fail-fast` included. Neither tasks.md
      nor design.md reconciles this with "then `make check`" appearing at 1.7, 2.4, 3.4, 4.6,
      5.4, 6.4, and 7.5. Resolution: for those intermediate boundaries, `make check` is run
      as `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D
      warnings`, `cargo test --all-features` (confirmed to fail on **exactly** the one known
      acceptance test, with the identical panic message each time — a second failure would be
      a real regression), and `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` (which
      still reports a TOTAL despite the known test failure). The literal, unqualified
      `make check` is run at 8.6 onward, once the acceptance test is GREEN.

- [x] 1.2 CHECK: Run the checks that must **pass** on the tree as it stands, so this change
      starts from clean gates rather than inheriting broken ones: `NOSPAWN-GREP` (with the
      default `MIN=8`), `GATE-MECH1`, `NOJSON-SEAM`, and `OPENSPEC-UNTOUCHED`. Record each
      output verbatim. Expected: `NOSPAWN OK: 8 files checked under src (>= 8)`.
      **Red when:** any already fails, in which case this change is not the place to fix it
      and the finding is reported before any code is written.
      **Recorded:** `NOSPAWN OK: 8 files checked under src (>= 8), only src/cli.rs may
      spawn`; `GATE-MECH1 OK (half A): no Default for Change, ChangeSet, ArtifactRef, Origin
      in 9 files under src` and `(half B): 61 constructions in src/changes.rs, no rest
      pattern and no functional update`; `NOJSON-SEAM OK: serde_json used in src/changes.rs,
      absent from src/cli.rs`; `OPENSPEC-UNTOUCHED OK`.

- [x] 1.3 CHECK: Run the checks that must **fail** today, and record each message. They are
      the proof that the corresponding green runs later are earned rather than structural.
      - `NOIO-VIEW` → `src/ui/app.rs missing`.
      - `NOCLI-SHELL` → `src/ui missing`.
      - `NORAW-GREP` → `src/ui/terminal.rs missing`.
      - `WIDTHS` → `src/ui/view.rs missing`.
      - `NODEFAULT-UI` → `src/ui/app.rs missing`.
      - `GRAPH-SNAP` → `GRAPH-SNAP FAIL: aarch64-apple-darwin resolved only 16 packages` —
        the per-triple `lines >= 40` guard trips long before the proc-macro assertion, since
        the graph is sixteen packages today. Record the message the run actually prints, not
        the assertion it would have reached.
      - `NOSPAWN-GREP` with `MIN=14` → `searched only 8 files under src (expected >= 14)`.
      **Red when:** any of the seven **passes** today, which would mean it cannot discriminate.
      **Recorded:** all seven failed with exactly the predicted messages —
      `NOIO-VIEW FAIL: src/ui/app.rs missing`; `NOCLI-SHELL FAIL: src/ui missing`;
      `NORAW FAIL: src/ui/terminal.rs missing`; `WIDTHS FAIL: src/ui/view.rs missing`;
      `NODEFAULT-UI FAIL: src/ui/app.rs missing`;
      `GRAPH-SNAP FAIL: aarch64-apple-darwin resolved only 16 packages`;
      `NOSPAWN FAIL: searched only 8 files under src (expected >= 14)`.

- [x] 1.4 CHANGE: Add to `Cargo.toml`
      `ratatui = { version = "0.30.2", default-features = false, features = ["crossterm"] }`
      — version read from `cargo info ratatui` rather than remembered (it reported `0.30.2`
      as latest at planning time), defaults off, feature list explicit. Do **not** declare
      `crossterm`: design.md → Decisions argues why the re-export is used instead. Raise
      `rust-version` from `1.85` to `1.88`, which `ratatui` 0.30.2 and its three sibling
      crates require. Commit `Cargo.lock`.
      **Recorded:** `cargo info ratatui` reports latest **0.30.2**, `rust-version 1.85.0`
      for the crate itself (the beta shown as "version" carries the same floor); the four
      packages that actually sit at the raised floor are `ratatui`, `ratatui-core`,
      `ratatui-crossterm`, and `ratatui-widgets`, all `1.88.0`, confirmed via `cargo
      metadata`. `cargo build` succeeds; `Cargo.lock` updated and staged.
      **Incidental fix, unplanned but forced by this task:** raising `rust-version` to
      `1.88` changes clippy's MSRV-gated suggestions — `1.88` is the floor at which
      `if`-let-chains became suggestable — and `cargo clippy -- -D warnings` newly failed
      on two pre-existing nested-`if` sites in `src/schema.rs` (lines ~151 and ~275,
      `clippy::collapsible_if`) that were clean before this edit. Fixed by applying
      clippy's own suggested collapse into a `let … && let …` chain at both sites — a
      behaviour-preserving mechanical rewrite, verified by the full suite staying green.
      Confirmed pre-existing-clean by running `cargo clippy` against the unstaged tree
      (`git stash`) before this edit: 0 warnings.

- [x] 1.5 CHANGE: Make four edits to the extracted `DEPS` copy, and record the diff:
      1. A fourth expected normal dependency: `ratatui`, defaults off, features exactly
         `crossterm`.
      2. An assertion that `crossterm` is **absent** from the declared set while **present**
         in the resolved graph.
      3. A fourth leg-5 removal case, for `ratatui`.
      4. **Delete leg 3 entirely**, together with its `EXPECTED_GRAPH` list and the
         per-triple loop that compares the four sets for equality. `GRAPH-SNAP` supersedes
         it, which is what design.md → Decisions decided ("the build graph moves from prose
         to a committed snapshot", "the proc-macro prohibition becomes an allowlist"). Leg 3
         as carried forward asserts a sixteen-package `EXPECTED_GRAPH`, that all four triples
         resolve identically, and that no `syn`/`quote`/`proc-macro2`/`ryu` is present — all
         three are falsified by adding `ratatui`, so leaving it in place would make task 1.7
         unsatisfiable. Keep the `TRIPLES` variable: leg 4 still uses it.
      Leg 5 for `ratatui` will correctly fail until group 8 — defer it with
      `DEPS_SKIP_LEG5=1` here and run it at task 9.6.
      **Recorded:** all four edits made in `$CHECKS/DEPS.sh` exactly as specified — leg 2's
      `want` dict gained `"ratatui":["crossterm"]` plus a positive assertion that
      `"crossterm" not in got`, and a new leg-2a-bis asserting `crossterm` present in
      `cargo tree -e normal`; leg 5 gained `needed ratatui "ui"`; leg 3 (the
      `EXPECTED_GRAPH` block and its per-triple equality loop) deleted outright, replaced
      with a one-line pointer to `GRAPH-SNAP.sh`, `TRIPLES` retained for leg 4.

- [x] 1.6 CHANGE: Generate the graph snapshot: `GRAPH_WRITE=1 sh $CHECKS/GRAPH-SNAP.sh`,
      producing `tests/fixtures/build-graph.txt`. Then run it **without** `GRAPH_WRITE` and
      confirm it passes. Record the per-triple package counts. At planning time, with only
      `ratatui` resolved in isolation, the four triples produced 62/62/63/63 packages and
      differed by exactly `linux-raw-sys`; the real numbers here are higher because this
      crate's existing three dependencies are in the same graph — record what the run
      reports, do not carry the planning-time numbers over.
      **Recorded:** `GRAPH-SNAP WROTE tests/fixtures/build-graph.txt`; re-run without
      `GRAPH_WRITE` → `GRAPH-SNAP OK: 4 triples match tests/fixtures/build-graph.txt; 8
      proc-macro crates; no encoding_rs, no time`. Per-triple counts: aarch64-apple-darwin
      **74**, x86_64-apple-darwin **74**, aarch64-unknown-linux-gnu **75**,
      x86_64-unknown-linux-gnu **75** — the two Linux triples each carry one more package
      than the two macOS triples, matching the design's named `linux-raw-sys` difference.

- [x] 1.7 VERIFY: Run `DEPS` with `DEPS_SKIP_LEG5=1` — legs 1, 2 and 4 must pass (leg 3 was
      deleted at task 1.5), output recorded verbatim, with leg 4 reporting `ratatui`, `ratatui-core`,
      `ratatui-crossterm`, and `ratatui-widgets` at the `1.88.0` floor. Then `make check`
      must still be green with the dependency added and no code using it yet — clippy
      included, since an unused dependency is not a clippy error. Commit.
      **Recorded:** `DEPS OK (leg 1a)` one bin target, edition 2024; `DEPS OK (leg 1b)`
      `scripts/build.sh` produces an executable; `DEPS OK (leg 2a)` exactly 4 normal deps,
      defaults off, features exact, crossterm not declared; `DEPS OK (leg 2a-bis)` crossterm
      present in the resolved graph; `DEPS OK (leg 2b)` `yaml-rust2`'s `features = []`
      spelled out; `DEPS OK (leg 2c)` `cargo build --locked`; `DEPS OK (leg 4)` floor `1.88`
      from `Cargo.toml`, at the floor: `darling, darling_core, darling_macro,
      herdr-openspec, instability, ratatui, ratatui-core, ratatui-crossterm,
      ratatui-widgets` — a superset of the four named in this task, which is expected and
      not a violation. `DEPS SKIPPED (leg 5)`. Then, per the 1.1 correction: `fmt-check`
      clean; `clippy -D warnings` clean; `cargo test --all-features` fails on exactly
      `ui_without_a_terminal_exits_three` (389 lib + 11 ci_workflow + 5-of-6 cli, the one
      known RED); `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` reports **98.93%**
      over 7,634 lines, 82 uncovered — unchanged from the 1.1 baseline (one line count moved
      by the clippy let-chain rewrite, coverage percentage identical), floor holds.
      **Red when:** the MSRV floor read from `Cargo.toml` is still `1.85` while a package
      declares `1.88`, which is exactly what leg 4 exists to catch.

---

## 2. `ui::layout` — the breakpoint and the frame split
<!-- kind: behavior -->

- [x] 2.1 RED: Create `src/ui/layout.rs` with an empty `mod tests` and write six failing
      tests in it, for responsive-layout's frame and breakpoint scenarios:
      - `mode_is_narrow_below_100` — `mode(0)`, `mode(1)`, `mode(40)`, `mode(60)`,
        `mode(99)` all `LayoutMode::Narrow`.
      - `mode_is_wide_at_100_and_above` — `mode(100)`, `mode(101)`, `mode(120)`,
        `mode(u16::MAX)` all `LayoutMode::Wide`. The 99/100 pair is the whole assertion:
        a test at 60 and 120 alone would pass with the constant set to 80.
      - `split_frame_gives_header_body_footer_at_normal_height` — for a `Rect` 60x20 and
        again 120x20: header is `y=0,height=1`; body is `y=1,height=18`; footer is
        `y=19,height=1`; and each region's `width` equals the input width.
      - `split_frame_degenerate_heights` — height 0 yields no header, no body, no footer;
        height 1 yields a header of height 1 and body and footer of height 0; height 2
        yields header `y=0`, footer `y=1`, body height 0. Asserted at width 60 and 120.
      - `split_body_wide_puts_the_divider_at_40` — at body `Rect { x:0, y:1, width:120,
        height:18 }`: list is `x=0,width=40`, detail is `x=40,width=80`. At width 100: list
        `x=0,width=40`, detail `x=40,width=60`.
      - `split_body_narrow_yields_one_region_for_the_route` — at width 60 with
        `Route::List`, list is the whole body and detail is `None`; with `Route::Detail`,
        detail is the whole body and list is `None`.
      **Red when:** `src/ui/layout.rs` does not compile because `mode`, `LayoutMode`,
      `split_frame`, and `split_body` do not exist. Confirm the failure is the missing
      behaviour, not a missing `pub mod ui;` — add the module declaration first so the
      compile error names the functions.
      **Design gap found and fixed here:** `split_body`'s narrow-mode scenario needs
      `Route`, which design.md → Boundaries assigns to `src/ui/app.rs` — group 3, which
      tasks.md sequences *after* this group, and the preamble states "view needs layout and
      app" (implying layout needs neither). This is a genuine forward reference the plan
      does not resolve. Fix: `Route`'s one-line definition (`pub enum Route { List, Detail
      }`, with `Debug, Clone, Copy, PartialEq, Eq`) is pulled forward into `src/ui/app.rs`
      now, minimally — nothing else from group 3 (`Dashboard`, `Action`, `action_for`,
      `apply`, or app.rs's own test module) is added yet. `Route` still lives exactly where
      design.md says; only its arrival commit moved earlier. `src/ui/mod.rs` created with
      `pub mod app;` and `pub mod layout;`; `pub mod ui;` added to `src/lib.rs`.
      **Recorded:** `error[E0432]: unresolved imports … no split_frame in ui::layout` (and
      the other three names) — confirmed missing-behaviour, not a missing `pub mod ui;`
      (that declaration was added first, as instructed).

- [x] 2.2 GREEN: Implement `WIDE_MIN_WIDTH: u16 = 100`, `LayoutMode`, `mode`,
      `split_frame`, and `split_body`. Handle heights 0, 1, and 2 with explicit branches
      rather than handing them to the constraint solver — design.md → Decisions and
      responsive-layout's requirement both say why: the solver's behaviour when constraints
      cannot all be satisfied is not part of its contract.
      **Recorded:** `split_frame` branches explicitly on height 0/1/2/else, each arm a
      `Rect` row helper; `split_body` calls `mode(area.width)` and, at `Wide`, uses
      `Layout::horizontal([Constraint::Length(40), Constraint::Min(0)])` (the solver is
      fine for the two-way horizontal split — only the three-way vertical split is
      excluded from it, per design.md). All 6 tests green on first implementation.

- [x] 2.3 REFACTOR: Fold any duplicated `Rect` arithmetic between the two split functions
      into one helper while the tests stay green, or record that none was duplicated.
      **Recorded:** folded the repeated `Rect { x: area.x, y, width: area.width, height }`
      construction inside `split_frame` into a local `row(y, height)` closure (four
      call-sites collapse to one-liners). No duplication existed *between* `split_frame`
      and `split_body` — the two use different constraint strategies by design (explicit
      branching vs. the solver) — so nothing was folded across them. All 6 tests stayed
      green.

- [x] 2.4 VERIFY: `testcount --lib 'ui::layout::tests::' 6`, then `make check`. Commit.
      **Recorded:** `TESTCOUNT OK: --lib filter 'ui::layout::tests::' ran 6 tests (>= 6)`.
      Per the 1.1 correction: `fmt-check` clean; `clippy -D warnings` clean; full suite —
      395 lib (389 + 6 new) + 11 `ci_workflow` + 5-of-6 `cli` (the one known RED,
      `ui_without_a_terminal_exits_three`, unchanged); `cargo llvm-cov --ignore-run-fail
      --fail-under-lines 80` → **98.94%** over 7,721 lines, 82 uncovered (`src/ui/layout.rs`
      itself at 100.00%), floor holds.

---

## 3. `ui::app` — `Dashboard`, `Route`, and key handling
<!-- kind: behavior -->

- [x] 3.1 RED: Create `src/ui/app.rs` with `mod tests { mod keys { … } }` and write eight
      failing tests, for dashboard-loop's key-handling scenarios:
      - `keys::quit_keys_and_their_near_misses` — Press `Char('q')` NONE → `Quit`; Press
        `Char('c')` CONTROL → `Quit`; Press `Char('Q')` SHIFT → `Ignore`; Press `Char('q')`
        CONTROL → `Ignore`; Press `Char('c')` NONE → `Ignore`.
      - `keys::only_press_kind_acts` — `Char('q')` with kind `Release` → `Ignore`; with kind
        `Repeat` → `Ignore`; with kind `Press` → `Quit`.
      - `keys::enter_and_esc_map_to_routes` — Press `Enter` → `OpenDetail`; Press `Esc` →
        `BackToList`; Press `Enter` with CONTROL → `Ignore`.
      - `keys::non_key_events_are_ignored` — `Resize(60,20)`, `FocusGained`, `FocusLost`,
        `Paste("q".into())`, and a `Mouse` event all → `Ignore`. The paste case is the one
        that matters: pasted text containing `q` must not close the pane.
      - `keys::action_for_is_total_over_a_keycode_sweep` — every `KeyCode` in an explicit
        list of at least twenty (`Backspace`, `Left`, `Right`, `Up`, `Down`, `Home`, `End`,
        `PageUp`, `PageDown`, `Tab`, `BackTab`, `Delete`, `Insert`, `F(1)`, `F(12)`,
        `Char('a')`, `Char('1')`, `Char('/')`, `Char('[')`, `Char(']')`, `Null`) returns
        without panicking, and every one except the four mapped keys returns `Ignore`.
        `Char('/')`, `Char('[')`, and `Char(']')` are named deliberately: `SPEC.md` → Keys
        assigns them behaviour in later changes, and they must be inert until then.
      - `keys::apply_quit_sets_the_flag` — a `Dashboard` at `Route::List`, `quit` false,
        given `Action::Quit`, has `quit` true and `route` still `List`.
      - `keys::apply_moves_between_routes` — `OpenDetail` then `BackToList` gives `Detail`
        then `List`, with `quit` false throughout.
      - `keys::back_to_list_at_list_is_a_no_op` — `BackToList` applied twice from `List`
        leaves `route == List` and `quit == false`, so a stray `Esc` at the root cannot
        close the pane.
      - `keys::dashboard_destructures_into_exactly_five_fields` — the compile-time half of
        the `Dashboard` gate: `let Dashboard { repo, searched_from, changes, route, quit } =
        &d;` with **no** `..`, then one assertion per binding. A sixth field added later
        fails to compile here, which is the half `NODEFAULT-UI`'s grep cannot do.
      **Red when:** `action_for`, `Action`, `Route`, and `Dashboard::apply` do not exist.
      **Recorded:** the bullet list names nine tests (the prose "eight failing tests" is a
      planning miscount, corrected here — the `Route` re-export note above already moved
      `Route` itself out of this RED step, so nine *new* app.rs tests remained to write, one
      of which is the compile-time `dashboard_destructures…` companion). Confirmed:
      `error[E0432]: unresolved imports … no action_for in ui::app` (and `Action`,
      `Dashboard`) — missing behaviour, not a harness issue.

- [x] 3.2 GREEN: Implement `Route`, `Action`, `Dashboard` (five fields, every one named at
      every construction site, **no** `Default` derive or impl), `action_for`, and
      `Dashboard::apply`. `action_for` matches on `Event::Key(k)` with
      `k.kind == KeyEventKind::Press` and falls through to `Ignore` for everything else.
      **Recorded:** all 9 tests green on first implementation. `action_for` uses a `let …
      else` guard for the non-`Key` case, a `kind != Press` early return, then one `match
      (key.code, key.modifiers)` over the four mapped combinations.

- [x] 3.3 REFACTOR: Extract the key-event match into a single `match` over
      `(code, modifiers)` if the first implementation nested conditionals, keeping tests
      green; otherwise record that no refactor was needed.
      **Recorded:** the first implementation already used a single `match (code,
      modifiers)` — no nested conditionals to extract. No refactor needed; all 9 tests
      stayed green.

- [x] 3.4 VERIFY: `testcount --lib 'ui::app::tests::' 9`. Then run `NODEFAULT-UI` — it must
      now **pass**, having failed at task 1.3 with `src/ui/app.rs missing`; record the
      positive-control line. Then `make check`. Commit.
      **Recorded:** `TESTCOUNT OK: --lib filter 'ui::app::tests::' ran 9 tests (>= 9)`.
      `NODEFAULT-UI OK: no Default for Dashboard, no elided field; positive control
      matched` — now passes, having failed at 1.3 with `src/ui/app.rs missing`. Per the 1.1
      correction: `fmt-check` clean (one nit auto-fixed by `cargo fmt`); `clippy -D
      warnings` clean; full suite — 404 lib (395 + 9) + 11 `ci_workflow` + 5-of-6 `cli` (the
      one known RED, unchanged); `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` →
      **98.92%** over 7,887 lines, 85 uncovered (`src/ui/app.rs` at 97.60%, `src/ui/layout.rs`
      still 100.00%), floor holds.

---

## 4. The `TestBackend` harness and `ui::view`
<!-- kind: behavior -->

- [x] 4.1 RED: Extend `crate::testutil` in `src/lib.rs` with the harness, and write three
      failing tests in `testutil::tests` for quality-gates' harness scenarios:
      - `render_at_touches_no_directory` — create a `testutil::ScratchDir` the test owns,
        take `testutil::snapshot` of **that directory alone** before and after
        `render_at(60, 20, &dashboard)` and `render_at(120, 20, &dashboard)`, and assert the
        two snapshots are equal; also assert each buffer's row 0 spells `OpenSpec` and its
        last row begins `q quit`, so the test fails if rendering is deleted rather than
        passing on two empty snapshots. **Do not snapshot `std::env::temp_dir()`**: this
        crate creates and destroys 203 `ScratchDir`s under it across parallel `cargo test`
        threads, so such a test is red on roughly six runs in ten for reasons that have
        nothing to do with rendering. The stronger claim — that a view file cannot perform
        I/O at all — is `NOIO-VIEW`, not a runtime observation.
      - `row_text_reads_a_whole_row` — `row_text(&buffer, 0)` returns a `String` whose
        length equals the buffer width, for widths 60 and 120.
      - `cell_reads_symbol_and_style` — `cell(&buffer, 0, 0)` returns the symbol and style
        of the top-left cell at both widths.
      **Red when:** `render_at`, `row_text`, and `cell` do not exist.
      **Design gap found and fixed here:** `render_at`'s own tests require `ui::view::render`
      to already produce real header and footer text ("row 0 spells `OpenSpec`", "last row
      begins `q quit`") — but `ui::view::render` is task 4.4's GREEN, two steps later. The
      same forward-reference shape as group 2/3's `Route` issue. Fix: `src/ui/view.rs` and
      `pub mod view;` are created now, with `render` implementing the header and footer
      *only* (both fully spec-correct, not stubs) — enough for these 3 tests to be honestly
      green. The body region (task 4.3/4.4's real subject — borders, titles, emphasis,
      interiors) is deliberately left undrawn until 4.3/4.4, so those 16 tests still get a
      genuine RED→GREEN cycle. Confirmed: `error[E0583]: file not found for module 'view'`
      — missing behaviour, not a harness issue.

- [x] 4.2 GREEN: Implement `testutil::render_at(width, height, &Dashboard) -> Buffer`
      building a `Terminal<TestBackend>`, drawing once through `ui::view::render`, and
      cloning the backend buffer; plus `row_text` and `cell`. All `#[cfg(test)]`, so nothing
      reaches the shipped binary.
      **Recorded:** all 3 tests green on first implementation (`render_at` via
      `TestBackend::new` + `Terminal::new` + `terminal.draw(...)` +
      `terminal.backend().buffer().clone()`; `row_text` concatenates `buffer[(x,y)].symbol()`
      per column; `cell` indexes `&buffer[(x,y)]`).

- [x] 4.3 RED: Create `src/ui/view.rs` with `mod tests` and write sixteen failing tests, one
      per responsive-layout scenario. Each names the exact cells it asserts, at each width:
      - `frame_rows_at_60_and_120` — at 60x20 and 120x20: `row_text(buf,0)[0..8] ==
        "OpenSpec"`; `cell(buf,0,0).style` has `Modifier::BOLD`; `row_text(buf,19)` starts
        `q quit  Enter detail  Esc back` and the remainder is all spaces; `cell(buf,0,1)`
        is `┌` and `cell(buf,0,18)` is `└`.
      - `one_row_frame_draws_header_only` — at 60x1 and 120x1: no panic;
        `row_text(buf,0)[0..8] == "OpenSpec"`; the whole buffer contains no `┌` and no
        `q quit`.
      - `two_row_frame_draws_no_body` — at 60x2 and 120x2: `row_text(buf,0)[0..8] ==
        "OpenSpec"`; `row_text(buf,1)[0..6] == "q quit"`; no box-drawing character anywhere.
      - `one_column_frame_does_not_panic` — at 1x1, 1x20, **60x20**, and 120x20: no panic;
        `row_text(buf,0) == "O"` and `row_text(buf,19) == " "` at 1x20; and at both 60x20
        and 120x20 `row_text(buf,0)[0..8] == "OpenSpec"` and `row_text(buf,19)[0..6] ==
        "q quit"`, so the one-column result is a width branch rather than the header and
        footer being absent everywhere.
      - `footer_drops_whole_hints` — at 18x20: `row_text(buf,19) == "q quit" + 12 spaces`.
        At 20x20: `row_text(buf,19) == "q quit  Enter detail"` exactly, no trailing space.
        At **60x20**: the full 30-character string then 30 spaces. At 120x20: the same 30
        characters then 90 spaces.
      - `wide_draws_two_regions_divided_at_40` — at 120x20: `cell(buf,0,1)=='┌'`,
        `cell(buf,39,1)=='┐'`, `cell(buf,40,1)=='┌'`, `cell(buf,119,1)=='┐'`;
        `row_text(buf,1)[1..8]=="Changes"`; `row_text(buf,1)[41..47]=="Detail"`;
        `cell(buf,0,18)=='└'`, `cell(buf,39,18)=='┘'`, `cell(buf,40,18)=='└'`,
        `cell(buf,119,18)=='┘'`. Contrasted at 60x20, where only one `┌` exists.
      - `narrow_draws_only_the_list_region` — at 60x20 with `Route::List`:
        `cell(buf,0,1)=='┌'`, `cell(buf,59,1)=='┐'`, the count of `┌` in the whole buffer is
        exactly 1, `row_text(buf,1)[1..8]=="Changes"`, and no row contains `Detail`.
        Contrasted at 120x20, where the count is 2.
      - `narrow_detail_route_replaces_the_list_region` — at 60x20 with `Route::Detail`:
        `row_text(buf,1)[1..7]=="Detail"` and no row contains `Changes`. At 120x20 with the
        same dashboard: **both** `Changes` at columns 1..8 and `Detail` at columns 41..47.
      - `breakpoint_is_exact_at_the_boundary` — the `┌` count in the whole buffer is 1 at
        60x20 and at 99x20, and 2 — the second at column 40 — at 100x20, at 101x20, and at
        120x20. Five renders, covering both mandated widths and all three boundary widths.
        101 is asserted here at the view tier and not only at the layout tier, because the
        spec scenario names it as a rendered width.
      - `resizing_the_backend_changes_the_next_frame` — one `Terminal<TestBackend>` at
        120x20, draw, `backend_mut().resize(60,20)`, draw again from the **same** dashboard:
        first buffer has two `┌`, second has one.
      - `routed_region_border_is_bold` — at 120x20 with `Route::List`: `cell(buf,0,1).style`
        has `BOLD`, `cell(buf,40,1).style` does not. With `Route::Detail` at 120x20: the two
        swap, so the test discriminates rather than asserting a constant. At 60x20 under
        **both** routes: `cell(buf,0,1).style` has `BOLD`, because the region drawn below
        the breakpoint is always the routed one.
      - `region_interiors_are_blank` — at 120x20: every cell in rows 2..=17, columns 1..=38
        and columns 41..=118, has symbol `" "` **and** a `Style` equal to
        `ratatui::buffer::Cell::default().style()`. At 60x20: rows 2..=17, columns 1..=58,
        same two assertions. Compare against `Cell::default().style()`, **not**
        `Style::default()`: `ratatui-crossterm` re-enables `underline-color` through its own
        defaults, so an untouched cell's style is `fg(Reset).bg(Reset).underline_color(Reset)`
        and an `assert_eq!(…, Style::default())` is red with no bug behind it.
      - `header_path_right_aligned_whole` — repo `/tmp/demo-repo` (14 chars): at 60x20
        `row_text(buf,0)[46..60]=="/tmp/demo-repo"` and `[8..46]` is all spaces; at 120x20
        `[106..120]` is the path.
      - `header_path_shortened_from_the_left` — repo
        `/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here` (70 chars):
        at 60x20 `row_text(buf,0)[9..60] ==
        "…/openspec-demos/a-rather-long-repository-name-here"`; at 120x20 `[50..120]` is the
        whole path and the buffer contains no `…`.
      - `header_omits_the_path_when_too_narrow` — the same 70-char repo at 16x20:
        `row_text(buf,0) == "OpenSpec" + 8 spaces`, no `…`. At **60x20** the shortened,
        ellipsis-prefixed path is in columns 9..60; at 120x20 the full path is in columns
        50..120 — so the omission is a width branch and not the feature being absent.
      - `header_says_no_repository` — `repo: None`: at 60x20 `row_text(buf,0)[47..60] ==
        "no repository"`; at 120x20 `[107..120]`; and neither buffer contains the
        `searched_from` path, because that empty state is `list-view`'s.
      **Red when:** `ui::view::render` does not exist. Confirm each failure names a missing
      function or a wrong cell, never a panic inside `TestBackend` construction.
      **Recorded:** 9 of 16 passed immediately (the header/footer-only scenarios, already
      implemented at 4.2) and 7 failed on a body-region assertion — `assertion left == right
      / left: " " / right: "┌"` and similar — confirming the missing behaviour is the body,
      not a harness or compile problem. Failing: `narrow_draws_only_the_list_region`,
      `narrow_detail_route_replaces_the_list_region`, `frame_rows_at_60_and_120`,
      `breakpoint_is_exact_at_the_boundary`, `routed_region_border_is_bold`,
      `wide_draws_two_regions_divided_at_40`, `resizing_the_backend_changes_the_next_frame`.

- [x] 4.4 GREEN: Implement `ui::view::render`, composing `layout::split_frame` and
      `layout::split_body`, the header (label, then the shortening rule from
      responsive-layout's requirement, counting **characters** not bytes), the footer's
      whole-hint drop rule, and the bordered regions with their titles and focus style.
      Region interiors are left untouched.
      **Recorded:** added `render_body`/`render_region` (the missing piece from 4.3),
      composing `layout::split_body` with `Block::bordered().title(..).border_style(..)` —
      `border_style`, not `style`, is what the planning review's finding 11 warns a naive
      implementation would use instead. All 16 tests green on first implementation.

- [x] 4.5 REFACTOR: Extract the header-shortening rule into a private pure function taking
      `(&str, u16) -> Option<String>` so it is readable independently of the frame, keeping
      tests green.
      **Recorded:** already factored this way from 4.2's implementation (`shorten_for_header(text:
      &str, header_width: u16) -> Option<String>`) — no further extraction needed; all tests
      stayed green.

- [x] 4.6 VERIFY: `testcount --lib 'ui::view::tests::' 16` and
      `testcount --lib 'testutil::tests::' 3`, then run `WIDTHS` — it must now **pass**,
      reporting all 16 view tests naming both 60 and 120, having failed at task 1.3. Every
      one of the sixteen names both widths after the repairs above; there is **no exemption
      list**, and the check has no mechanism for one. `make check`. Commit.
      **Recorded:** `TESTCOUNT OK: --lib filter 'ui::view::tests::' ran 16 tests (>= 16)`;
      `TESTCOUNT OK: --lib filter 'testutil::tests::' ran 3 tests (>= 3)`. `WIDTHS` first
      **failed**: `these view tests do not name both 60 and 120: frame_rows_at_60_and_120,
      one_row_frame_draws_header_only, two_row_frame_draws_no_body,
      one_column_frame_does_not_panic, breakpoint_is_exact_at_the_boundary` — those five used
      `60u16`/`120u16`/`100u16` literals, and `\b\d+\b` does not match a digit run fused to a
      type suffix (no word boundary between a digit and a following letter). Fixed by
      dropping the `u16` suffixes (Rust infers the element type from `render_at`'s parameter
      regardless); all 16 tests stayed green. Re-run: `WIDTHS OK: all 16 view tests name both
      60 and 120`. Per the 1.1 correction: `fmt-check` clean (two nits auto-fixed); `clippy -D
      warnings` clean; full suite — 423 lib (404 + 3 + 16) + 11 `ci_workflow` + 5-of-6 `cli`
      (the one known RED, unchanged); `cargo llvm-cov --ignore-run-fail --fail-under-lines 80`
      → **98.92%** over 8,259 lines, 89 uncovered (`src/ui/view.rs` at 99.02%, `src/ui/app.rs`
      97.60%, `src/ui/layout.rs` 100.00%), floor holds.

---

## 5. `ui::terminal` — the terminal seam
<!-- kind: behavior -->

- [x] 5.1 RED: Create `src/ui/terminal.rs` with `mod tests { mod guard { … } }`, including a
      `Recorder` double that appends each call's name to a `RefCell<Vec<&'static str>>` and
      can be configured to fail any subset of the four operations. Write eight failing tests
      for terminal-lifecycle's scenarios:
      - `guard::normal_lifetime_is_enter_enter_leave_disable` — recorded list is exactly
        `["enable_raw","enter_alternate","leave_alternate","disable_raw"]`.
      - `guard::enable_raw_failure_attempts_nothing_further` — recorded list is exactly
        `["enable_raw"]` and the returned error's `op` is `"enable_raw"`.
      - `guard::alternate_screen_failure_unwinds_raw_mode` — recorded list is exactly
        `["enable_raw","enter_alternate","disable_raw"]` and the error's `op` is
        `"enter_alternate"`.
      - `guard::teardown_errors_do_not_panic_and_both_are_attempted` — both teardown
        operations configured to fail; the drop completes and the list ends
        `["leave_alternate","disable_raw"]`.
      - `guard::a_panic_still_restores` — `std::panic::catch_unwind` around a closure that
        enters a guard and panics; the result is `Err` and the recorded list is the full
        mirrored four.
      - `guard::restore_then_restores_before_delegating` — `restore_then(&rec, &mut || …)`
        records exactly `["leave_alternate","disable_raw","previous_hook"]`.
      - `guard::restore_then_delegates_even_when_both_restores_fail` — same list, with both
        restore operations failing.
      - `guard::terminal_error_names_the_failing_op_and_detail` — a `TerminalError`'s
        `Display` names both its `op` and its `detail`, so a start failure reaching stderr
        at exit status 1 is diagnosable.
      **Red when:** `TerminalOps`, `TerminalGuard`, `TerminalError`, and `restore_then` do
      not exist. The `Recorder` must be written first and must compile against the trait,
      so confirm the failure is the missing production types and not the double.
      **Recorded:** `error[E0432]: unresolved imports … no restore_then in ui::terminal`
      (and `TerminalError`, `TerminalGuard`, `TerminalOps`) — the `Recorder` double itself
      compiled cleanly against the trait signatures named in its `impl TerminalOps for
      Recorder` block; only the production types were missing.

- [x] 5.2 GREEN: Implement `TerminalError`, `TerminalOps`, `TerminalGuard` (with `Drop`),
      `restore_then`, `install_panic_hook`, and `CrosstermOps`. `CrosstermOps`'s four
      methods each call exactly one `ratatui::crossterm` function and map its error; no
      branch, no ordering, no state. `install_panic_hook` captures
      `std::panic::take_hook()` and installs a hook whose body is
      `restore_then(&CrosstermOps, &mut || previous(info))`.
      **Recorded:** all 8 tests green after also adding a minimal `Debug` impl for
      `TerminalGuard` (needed by `Result::expect_err` in two tests; `#[derive(Debug)]`
      does not apply since the struct holds a `&dyn TerminalOps` trait object).
      `CrosstermOps::enter_alternate`/`leave_alternate` use
      `ratatui::crossterm::execute!(stdout(), EnterAlternateScreen)` /
      `LeaveAlternateScreen`; `enable_raw`/`disable_raw` call the corresponding bare
      functions. All four map their error into `TerminalError` with the operation's name.

- [x] 5.3 REFACTOR: If `Drop` and `restore_then` duplicate the leave-then-disable sequence,
      have `Drop` call `restore_then` with a no-op continuation, keeping tests green.
      **Recorded:** implemented this way from the start of 5.2 rather than as a later
      extraction (`Drop::drop` is `restore_then(self.ops, &mut || {})`) — no separate
      refactor step was needed. All 8 tests green throughout.

- [x] 5.4 VERIFY: `testcount --lib 'ui::terminal::tests::' 8`. Then run `NORAW-GREP`. It
      **still fails at this point**, and on its *file-count* guard rather than on anything
      about raw mode: at the end of this group `src/` holds 14 `.rs` files and `tests/` holds
      2, so the searched set is 15 and the guard reads
      `NORAW FAIL: searched only 15 files (expected >= 16)` — `ui/driver.rs` and
      `ui/event.rs` do not exist until group 6. Record that exact message and re-run the
      check in full at task 9.4, where the count is 17 and the `CrosstermOps` leg reports 2
      sites. **Do not weaken the threshold**: the guard is doing its job. `make check`. Commit.
      **Recorded:** `TESTCOUNT OK: --lib filter 'ui::terminal::tests::' ran 8 tests (>= 8)`.
      `NORAW-GREP` → `NORAW FAIL: searched only 15 files (expected >= 16)` — exactly the
      predicted message; threshold left unweakened. Per the 1.1 correction: `fmt-check`
      clean; `clippy -D warnings` clean; full suite — 431 lib (423 + 8) + 11 `ci_workflow` +
      5-of-6 `cli` (the one known RED, unchanged); `cargo llvm-cov --ignore-run-fail
      --fail-under-lines 80` → **98.53%** over 8,424 lines, 124 uncovered
      (`src/ui/terminal.rs` at 78.79% — `CrosstermOps` and `install_panic_hook` are the
      named, argued-uncoverable residue from design.md → Test Strategy), floor holds well
      clear of 80.

---

## 6. `ui::event` and `ui::driver` — the event loop
<!-- kind: behavior -->

- [x] 6.1 RED: Create `src/ui/event.rs` (trait, `EventError`, `CrosstermEvents`) and
      `src/ui/driver.rs` with `mod tests`, including a `Script` double that returns queued
      `Result<Option<Event>, EventError>` values, records the `timeout` of every call, and
      returns `Err(EventError("script exhausted".into()))` once empty — never `Ok(None)`
      forever, which would turn a loop bug into a hung test. Also a `FailingBackend`
      implementing `ratatui::backend::Backend` whose `draw` returns an error. Write six
      failing tests for dashboard-loop's loop scenarios:
      - `first_frame_precedes_the_first_poll` — empty script at 120x20; `run_loop` returns
        `Err(LoopError::Events(_))`; the buffer nevertheless has `row_text(buf,0)[0..8] ==
        "OpenSpec"` and `┌` at (0,1) and (40,1).
      - `timeouts_are_not_events` — three `Ok(None)` then Press `q` at 60x20; returns
        `Ok(LoopSummary { frames: 4, polls: 4 })`; `dashboard.quit` true; every recorded
        timeout equals the `tick` the caller passed (a value deliberately different from
        `driver::TICK`, so a hard-coded constant fails).
      - `ctrl_c_ends_the_loop` — one Press `Char('c')` with CONTROL at 60x20; returns
        `Ok(LoopSummary { frames: 1, polls: 1 })`; `quit` true.
      - `ignored_input_redraws_and_continues` — Press `Char('Q')`, then `Resize(60,20)`,
        then Press `q`, at 60x20; returns `frames: 3, polls: 3`; route still `List`.
      - `route_change_shows_in_the_next_frame` — Press `Enter` then Press `q` at 60x20;
        returns `frames: 2, polls: 2`; the final buffer's `row_text(buf,1)[1..7] ==
        "Detail"` and no row contains `Changes`.
      - `a_draw_failure_stops_before_polling` — `FailingBackend` with a script that would
        supply `q`; returns `Err(LoopError::Draw(_))` carrying the backend error's text; the
        script recorded **zero** calls.
      **Red when:** `run_loop`, `LoopSummary`, `LoopError`, `EventSource`, and `EventError`
      do not exist.
      **Recorded:** `pub mod event; pub mod driver;` added first. `src/ui/event.rs` created
      with only a module doc comment (no types yet), and `src/ui/driver.rs` with only the
      test module — the `Script` and `FailingBackend` doubles reference `EventSource`,
      `EventError` (from the still-empty `event.rs`) and `run_loop`, `LoopSummary`,
      `LoopError` (from `driver.rs` itself), none of which exist yet. Confirmed:
      `error[E0432]: unresolved imports … no run_loop in ui::driver` and a second `no
      EventSource in ui::event` — both production sides missing, exactly as predicted.

- [x] 6.2 GREEN: Implement `EventSource`, `EventError`, `CrosstermEvents` (poll then read,
      two lines, no branch of its own beyond the poll result), `TICK`, `LoopSummary`,
      `LoopError`, and `run_loop`: draw, count the frame, poll, count the poll, apply the
      action, break on `quit`.
      **Recorded:** 5 of 6 tests green immediately; `route_change_shows_in_the_next_frame`
      failed on a byte-index panic — `&row_text(buf, 1)[1..7]` sliced mid-character, since
      column 0 is the (3-byte) `┌` border. Same hazard already worked around in group 4's
      view tests via a char-based `cols()` helper; fixed here the same way
      (`row_text(buf,1).chars().skip(1).take(6).collect()`). All 6 green after the fix.

- [x] 6.3 REFACTOR: If the frame/poll counters and the break condition read awkwardly,
      restructure the loop body while keeping the six tests green; otherwise record that no
      refactor was needed.
      **Recorded:** `cargo clippy --all-targets --all-features -- -D warnings` reported
      nothing on the loop body — no restructure needed. All 6 tests stayed green.

- [x] 6.4 VERIFY: `testcount --lib 'ui::driver::tests::' 6`, then `make check`. Commit.
      **Recorded:** `TESTCOUNT OK: --lib filter 'ui::driver::tests::' ran 6 tests (>= 6)`.
      Per the 1.1 correction: `fmt-check` clean; `clippy -D warnings` clean; full suite —
      437 lib (431 + 6) + 11 `ci_workflow` + 5-of-6 `cli` (the one known RED, unchanged);
      `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` → **98.15%** over 8,637
      lines, 160 uncovered (`src/ui/event.rs` at 0.00% — `CrosstermEvents` is named,
      argued-uncoverable residue, same reasoning as `CrosstermOps`), floor holds well
      clear of 80.

---

## 7. `ui::load` and `changes::empty_set` — startup state from files
<!-- kind: behavior -->

- [x] 7.1 RED: Add `mod tests { mod load { … } }` to `src/ui/mod.rs` and write four failing
      tests for dashboard-loop's startup scenarios, each building a `ScratchDir` tree:
      - `load::a_scratch_repository_is_loaded_from_files` — a tree with
        `openspec/changes/alpha/proposal.md` and `openspec/changes/alpha/tasks.md`
        containing two checked and one unchecked item; call `ui::load` from a subdirectory
        two levels below the root with a `Config` whose `archived_count` is 5. Assert
        `repo == Some(testutil::canonical(root))`, `changes.active.len() == 1`,
        `changes.active[0].name == "alpha"`, progress `2/3`, `route == Route::List`,
        `quit == false`.
      - `load::no_repository_above_the_start` — a fresh `ScratchDir`; **first** walk its
        ancestors and assert none holds an `openspec` directory, failing with a message
        naming the offending ancestor, exactly as `resolve`'s own `NotFound` test does.
        Then assert `repo == None`, `searched_from` equals what `resolve::find_repo`
        reported, and `changes.active`, `.archived`, and `.problems` are all empty.
      - `load::archived_count_from_config_is_honoured` — a tree with seven dated archive
        directories and a `Config` whose `archived_count` is 3; assert
        `changes.archived.len() == 3`. This is what proves `load` passes the configured
        value through rather than a literal.
      - `load::loading_writes_nothing` — `testutil::snapshot` of the whole scratch tree
        before and after `ui::load`; assert equality. Full recursive snapshot, not shallow:
        the tree is small and every byte and mtime matters here.
      **Red when:** `ui::load` and `changes::empty_set` do not exist.
      **Recorded:** `error[E0425]: cannot find function 'load' in module 'super::super'`
      (three call sites) — missing behaviour, not a harness issue.

- [x] 7.2 GREEN: Implement `changes::empty_set()` in `src/changes.rs`, naming all three
      `ChangeSet` fields explicitly with no `Default` and no `..`, and `ui::load` in
      `src/ui/mod.rs`, branching on `resolve::find_repo`'s two variants.
      **Recorded:** all 4 tests green on first implementation. For the `Found` branch,
      `Dashboard::searched_from` is the canonicalized `start` (`RepoSearch::Found` carries
      only `root`, not the original start point, so `ui::load` canonicalizes `start`
      itself — no test pins this field's value in the found case, only in the not-found
      one, where it is `find_repo`'s own reported `searched_from`).

- [x] 7.3 CHECK: Run `GATE-MECH1` — `empty_set` is a new construction site inside the file
      that gate searches, so it must still report no `Default` for `Change`, `ChangeSet`,
      `ArtifactRef`, or `Origin`, and no rest pattern or functional update in
      `src/changes.rs`. Record the new construction count, which is one higher than the
      value `changes-from-cli` recorded.
      **Recorded:** `GATE-MECH1 OK (half A)` unchanged (16 files now, `ui/*` added).
      `GATE-MECH1 OK (half B): 63 constructions` — **two** higher than the task-1.2 baseline
      of 61, not one. Reproduced and understood: the counting regex matches `ChangeSet\s*\{`
      textually, which fires on a function's `-> ChangeSet {` return-type-and-brace shape as
      well as on an actual struct literal — `pub fn empty_set() -> ChangeSet {` contributes
      one hit for the signature and one for the literal body, both genuinely new. The
      predicted "+1" assumed only the literal counts; the check's own regex does not
      distinguish the two shapes (a property already true of the baseline — `from_files`'s
      and `merge`'s own `-> ChangeSet {` signatures already contribute this way, unremarked
      at task 1.2). Not a defect: `hits_b` (the rest-pattern/functional-update half) is
      unaffected, and half A's `Default` search is orthogonal to this count.

- [x] 7.4 REFACTOR: None expected — `load` is a two-arm match. State explicitly that no
      refactor was needed, or make one and say what.
      **Recorded:** `cargo clippy --all-targets --all-features -- -D warnings` reported
      nothing — no refactor needed. All 4 tests stayed green.

- [x] 7.5 VERIFY: `testcount --lib 'ui::tests::load::' 4`, then `make check`. Commit.
      **Recorded:** `TESTCOUNT OK: --lib filter 'ui::tests::load::' ran 4 tests (>= 4)`.
      Per the 1.1 correction: `fmt-check` clean; `clippy -D warnings` clean; full suite —
      441 lib (437 + 4) + 11 `ci_workflow` + 5-of-6 `cli` (the one known RED, unchanged);
      `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` → **98.11%** over 8,748
      lines, 165 uncovered, floor holds well clear of 80.

---

## 8. `ui::run` and `main` — the wiring, and outer-loop GREEN
<!-- kind: behavior -->

- [x] 8.1 RED: Add `mod tests { mod start { … } }` to `src/ui/mod.rs` and write three
      failing tests for terminal-lifecycle's refusal scenario:
      - `start::not_a_terminal_records_no_call` — `enter_if_terminal(false, &recorder)`
        returns `Err(StartError::NotATerminal)` and the recorder's list is **empty**. The
        emptiness is the discriminating assertion: a guard entered and immediately dropped
        would also return an error to a careless caller, and would record four calls.
      - `start::a_terminal_enters_the_guard` — `enter_if_terminal(true, &recorder)` returns
        `Ok` and the list is `["enable_raw","enter_alternate"]`, so the `false` case is a
        branch rather than a function that never does anything.
      - `start::a_failing_enable_raw_becomes_start_error_terminal` —
        `enter_if_terminal(true, &failing)` returns `Err(StartError::Terminal(e))` whose
        `op` is `"enable_raw"`.
      **Red when:** `enter_if_terminal` and `StartError` do not exist.
      **Recorded:** `error[E0433]: failed to resolve: could not find 'StartError' in
      'super'` and `error[E0425]: cannot find function 'enter_if_terminal' in module
      'super::super'` — missing behaviour, not a harness issue.

- [x] 8.2 GREEN: Implement `StartError`, `enter_if_terminal`, and `ui::run`. `run`'s body,
      in order: `enter_if_terminal(std::io::stdout().is_terminal(), &CrosstermOps)?`;
      `install_panic_hook()`; `config::load_from_env()`; `std::env::current_dir()`;
      `load(&cwd, &config)`; `Terminal::new(CrosstermBackend::new(std::io::stdout()))`;
      `run_loop(&mut terminal, &mut dashboard, &mut CrosstermEvents, TICK)`; drop the guard.
      Keep it to wiring with no branch of its own beyond `?`, which is what design.md →
      Risks accepts as uncovered.
      **Recorded:** all 3 tests green on first implementation. `From<std::io::Error>` and
      `From<LoopError>` for `StartError` let `current_dir()?`, `Terminal::new(...)?`, and
      `run_loop(...)?` each convert with a bare `?`, keeping `run`'s body to the named
      straight-line sequence plus the guard drop at the end of scope.

- [x] 8.3 CHANGE: Update `src/main.rs` — `Invocation::Ui` calls `ui::run()` and maps
      `Ok(())` to exit 0, `Err(StartError::NotATerminal)` to a stderr message naming
      `herdr-openspec` and `not a terminal` plus exit 3, and any other `Err` to its
      `Display` text plus exit 1. Delete `lib::banner` and its unit test; leave
      `lib::parse`, `lib::usage`, and `lib::rejection_text` untouched, so the status-2 path
      is unchanged.
      **Recorded:** done as specified. `StartError::NotATerminal`'s own `Display` reads just
      `"not a terminal"` — `main.rs` prepends `"herdr-openspec: "` itself for the exit-3
      message, so the process name lives in the one place `plugin-build` owns the exit
      surface, not duplicated into the library type. `lib::banner` and its test deleted;
      `parse`, `usage`, `rejection_text` untouched, confirmed by `unknown_subcommand`,
      `extra_arguments_after_ui`, and `no_subcommand` staying green unmodified.

- [x] 8.4 CHANGE: Replace `tests/cli.rs::ui_prints_placeholder_banner` and
      `ui_holds_open_until_stdin_closes` — both now assert removed behaviour — with
      `failing_statuses_are_distinct`: run the binary four ways with stdout piped and stdin
      at EOF (`ui`, `wat`, `ui --tab`, no arguments), assert statuses 3, 2, 2, 2, assert the
      three status-2 runs print usage listing `ui` and name their rejected token, and assert
      every run's stdout is empty. `ui_without_a_terminal_exits_three` from group 0 stays.
      **Recorded:** done as specified. `unknown_subcommand`, `extra_arguments_after_ui`, and
      `no_subcommand` are left in place, unmentioned by this task and covering the same
      shapes individually — the net count (6 tests before this task: 5 baseline + group 0's
      addition, minus the 2 replaced, plus 1 new) lands back at 5, matching 8.5's target.

- [x] 8.5 VERIFY (outer loop closes): Run
      `cargo test --all-features --test cli ui_without_a_terminal_exits_three` — it must now
      **pass**, having failed at task 0.3 with the recorded message. That is a filtered run,
      so read its `filtered out` arithmetic and confirm exactly one test ran; a filter that
      matched nothing exits 0. Then `testcount --test-cli '' 5` for the whole binary, and
      `testcount --lib 'ui::tests::start::' 3`. The `5` is the same as the HEAD baseline, so
      it is not by itself a guard on task 8.4 having happened; what makes 8.4 non-skippable
      is that 8.3 deletes `lib::banner`, which leaves the surviving
      `ui_prints_placeholder_banner` failing the run outright.
      **Red when:** the binary still blocks on stdin, or exits 0 or 2 instead of 3.
      **Recorded — the outer loop closes:** `test ui_without_a_terminal_exits_three ... ok`;
      `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out` — exactly
      one test ran under the filter, and it passed, having failed since task 0.3.
      `TESTCOUNT OK: --test-cli filter '' ran 5 tests (>= 5)`;
      `TESTCOUNT OK: --lib filter 'ui::tests::start::' ran 3 tests (>= 3)`.

- [x] 8.6 REFACTOR + VERIFY: No refactor is expected — `run` is straight-line wiring and
      `main`'s dispatch is a three-arm match. State explicitly that none was needed, or make
      one and say what. Then `make check`, then `cargo llvm-cov --summary-only` and record
      the TOTAL against the 98.93% baseline from task 1.1. The floor is 80 and is not moving; this is
      recorded so a large unexplained drop is visible rather than discovered later. Commit.
      **Recorded:** `cargo clippy --all-targets --all-features -- -D warnings` reported
      nothing on `ui::run` or `main` — no refactor needed. **`make check` — the literal,
      unqualified command, no longer needing the 1.1 substitution now that the acceptance
      test is green — exits 0**: `fmt-check` clean, `lint` clean, `test` 443 lib + 11
      `ci_workflow` + 5 `cli`, all passing, `coverage` passes the 80% floor.
      `cargo llvm-cov --summary-only` → **TOTAL 97.82%** over 8,822 lines, 192 uncovered —
      down from the 98.93%/7,635-line/82-uncovered baseline, as design.md → Risks predicted
      ("the recorded total will move a little more than the residue list alone suggests"):
      the newly-uncoverable residue is `ui::terminal::CrosstermOps` and
      `install_panic_hook`, `ui::event::CrosstermEvents`, `ui::run`'s body after its
      terminal check, `main.rs`'s exit-1 arm (unreachable without a real terminal), and the
      test-only `FailingBackend`'s seven `Backend` methods `Terminal` never calls — every
      one named in that section in advance. The floor (80) is unmoved and cleared by 17.82
      points.

---

## 9. Architectural and dependency checks
<!-- kind: operational -->

- [x] 9.1 CHECK: Re-read design.md → Test Boundaries and confirm no test written in groups
      2–8 introduced a collaborator it does not name — in particular that no view test
      opened a directory and no test constructed `CrosstermOps`.
      **Recorded:** confirmed by grep — `CrosstermOps` is named only in `src/ui/terminal.rs`
      (definition and `install_panic_hook`'s body) and `src/ui/mod.rs` (`ui::run`'s wiring),
      never under `tests/` or in any `#[cfg(test)]` module. No `std::fs::`/`ScratchDir`
      reference exists in `src/ui/view.rs`, `layout.rs`, `app.rs`, or `driver.rs`. Both
      checked directly, ahead of running the mechanical checks below.

- [x] 9.2 VERIFY: `NOSPAWN-GREP` with `MIN=14` — must **pass**, having failed at task 1.3
      with `searched only 8 files`. Record the file count it reports.
      **Recorded:** `NOSPAWN OK: 15 files checked under src (>= 14), only src/cli.rs may
      spawn`.

- [x] 9.3 VERIFY: `NOIO-VIEW`, `NOCLI-SHELL`, and `NODEFAULT-UI` — all three must **pass**,
      having failed at task 1.3 with `src/ui/app.rs missing`, `src/ui missing`, and
      `src/ui/app.rs missing` respectively. Record the positive-control line from each.
      **Recorded:** `NOIO-VIEW OK: 4 pure files carry no I/O API; positive control matched`;
      `NOCLI-SHELL OK: 7 files under src/ui name no CLI seam; positive control matched`;
      `NODEFAULT-UI OK: no Default for Dashboard, no elided field; positive control
      matched`.

- [x] 9.4 VERIFY: `NORAW-GREP` in full — must now **pass** both legs, including the
      `CrosstermOps` site count of at least 2, which task 5.4 recorded as still failing.
      **Recorded — real finding, fixed here:** first run **failed** its second leg:
      `NORAW FAIL: CrosstermOps named outside terminal.rs and mod.rs:
      src/ui/event.rs:23`. `src/ui/event.rs`'s doc comment on `CrosstermEvents` named
      `CrosstermOps` in prose (cross-referencing design.md's Test Strategy), which the grep
      cannot distinguish from a real construction site. Fixed by rewording the comment to
      describe the same fact without the identifier. Re-run: `NORAW OK: 17 files searched,
      mode functions only in src/ui/terminal.rs, CrosstermOps at 6 sites` — the file count
      (17) matches the group-5.4 prediction for this point in the plan.

- [x] 9.5 VERIFY: `NOSPAWN-GREP`, `NOIO-VIEW`, `NOCLI-SHELL`, `NORAW-GREP`, and
      `NODEFAULT-UI` negative controls. Copy `src/` and `tests/` to `$WORK/neg-N/` and plant
      one violation per copy, confirming each check goes **red**:
      1. `let _ = std::process::Command::new("ls");` in `src/ui/driver.rs` → `NOSPAWN-GREP`
         and `NOIO-VIEW` both fail.
      2. `let _ = std::fs::read_to_string("/etc/hosts");` in `src/ui/view.rs` → `NOIO-VIEW`
         fails.
      3. `fn _x(_: &dyn crate::cli::OpenspecCli) {}` in `src/ui/app.rs` → `NOCLI-SHELL`
         fails.
      4. `let _ = ratatui::crossterm::terminal::enable_raw_mode();` in `tests/cli.rs` →
         `NORAW-GREP` fails. This is the control that matters most: it is the exact mistake
         that would put a developer's terminal into raw mode.
      5. `src/ui/terminal.rs` deleted → `NOIO-VIEW` and `NORAW-GREP` both fail on their
         guards rather than reporting a clean tree.
      6. `src/changes.rs`'s `OpenspecCli` mention removed → `NOCLI-SHELL` fails on its
         positive control.
      7. `impl Default for Dashboard { … }` added to `src/ui/app.rs` → `NODEFAULT-UI` half A
         fails.
      8. `let Dashboard { quit, .. } = d;` added to `src/ui/view.rs` → `NODEFAULT-UI` half B
         fails.
      9. `#[derive(Debug, Default)]` on `struct Dashboard` → `NODEFAULT-UI` half A's derive
         leg fails, which is the case a plain `impl Default` search would miss.
      **Red when:** any control passes, which would mean the check cannot discriminate.
      Never edit the real tree for this. The four grep checks read no `$WORK` themselves —
      they run against whatever directory is the working directory, or against `$SRC` where
      they take one — so the protection is that `cp -R src tests "$WORK/neg-N/"` fails
      loudly when `$WORK` is unset, which is why task 1.1 exports it.
      **Recorded:** all nine planted violations, each run from inside its own
      `$WORK/neg-N/` copy (real tree never edited), discriminated exactly as specified —
      `NOSPAWN FAIL: spawn API outside src/cli.rs` and (same copy) `NOIO-VIEW FAIL: I/O API
      in a pure view file` for #1; `NOIO-VIEW FAIL` for #2; `NOCLI-SHELL FAIL: the shell
      names the CLI seam` for #3; `NORAW FAIL: terminal-mode function outside
      src/ui/terminal.rs` for #4; `NOIO-VIEW FAIL: src/ui/terminal.rs missing` and (same
      copy) `NORAW FAIL: src/ui/terminal.rs missing` for #5; `NOCLI-SHELL FAIL: positive
      control - src/changes.rs does not name OpenspecCli` for #6; `NODEFAULT-UI FAIL:
      Dashboard has a Default` (the plain-`impl` leg) for #7; `NODEFAULT-UI FAIL: Dashboard
      literal or pattern elides a field` for #8; `NODEFAULT-UI FAIL: Dashboard has a
      Default` (the derive leg, confirmed by the reported hit being the `#[derive(...,
      Default, ...)]` line rather than an `impl` line) for #9.

- [x] 9.6 VERIFY: `DEPS` in full, including leg 5 for all four crates — removing `ratatui`
      from a throwaway copy's manifest must now fail the build, which it could not before
      group 4. Then `GRAPH-SNAP` and `NOJSON-SEAM` and `NOWAIVER`.
      **Recorded:** `DEPS OK` legs 1a/1b/2a/2a-bis/2b/2c/4/5 (toml, yaml-rust2, serde_json,
      **ratatui**, and the not-declared guard) — all pass, working tree unchanged including
      `Cargo.lock`. `GRAPH-SNAP OK: 4 triples match tests/fixtures/build-graph.txt; 8
      proc-macro crates; no encoding_rs, no time`. `NOJSON-SEAM OK`. `NOWAIVER OK: floor is
      80, no exclusion, no coverage attribute`.

- [x] 9.7 VERIFY: `OPENSPEC-UNTOUCHED` against `$BASE`. The only permitted paths under
      `openspec/` are this change's own artifact directory. Any other path, tracked or
      untracked, fails — the untracked sweep is what would catch a runtime write.
      **Recorded:** `OPENSPEC-UNTOUCHED OK` against
      `BASE=e25017546cea99e4b606fcd74a404131e1ca582e`.

- [x] 9.8 VERIFY: `make check`. Commit.
      **Recorded:** `make check` exits 0 — `fmt-check`, `lint`, `test` (443 lib + 11
      `ci_workflow` + 5 `cli`), and `coverage` (**97.74%**/**97.82%** lines over 8,822
      lines, 192 uncovered) all pass; floor (80) cleared.

---

## 10. Documentation
<!-- kind: operational -->

- [x] 10.1 CHECK: Re-read `SPEC.md` and list every statement this change made false, before
      editing. Expected set, each corrected in 10.2: the stack line implies a direct
      `crossterm` dependency; the Keys table omits `Ctrl-C`; the module map's `ui` row omits
      terminal lifecycle and the event loop; nothing anywhere covers "stdout is not a
      terminal"; the "Unit-tested modules" list omits `ui`; the Responsive layout section
      gives no column widths, while this change freezes them; the List view mock's rows are
      48 characters wide and cannot fit the 38-column interior those widths imply; the
      Responsive layout section scopes `Enter`/`Esc` to the narrow case only; and the
      Fixtures section says there are exactly two mechanisms while this change adds a third.
      **Red when:** a statement on this list turns out to be already true, or a false
      statement is found that is not on it — in either case the list is corrected here
      before 10.2 edits anything.
      **Recorded:** re-read confirmed all nine statements false exactly as listed, and no
      further false statement was found beyond them.

- [x] 10.2 CHANGE: Correct `SPEC.md` — audience: every future change reading the design
      contract; durable reason: `AGENTS.md` requires the spec to be corrected by the change
      that disproves it, not left to drift. Nine edits, all **rewrites in place** except the
      two additions named below, adding roughly 30 lines net:
      - **Stack** (§ Overview): say `crossterm` is reached through `ratatui`'s re-export and
        is not a declared dependency.
      - **Architecture → module map**: rewrite the `ui` row to name views, layout, key
        handling, terminal lifecycle, and the event loop.
      - **User interface → Keys**: add a `Ctrl-C` row alongside `q`, and say `Esc` at the
        list root is inert rather than a quit.
      - **Degraded states**: add a short paragraph *below* the table, headed so it is
        visibly not a row: **no terminal is not a degraded state**. The section opens "Every
        condition renders usable content rather than an error screen", and `PRD.md` →
        Success criteria repeats it; adding `stdout is not a terminal` as a table row would
        make both false. It is a precondition failure, not a degradation — there is nothing
        to render into — so it is recorded outside the table, naming exit status 3 and the
        message, and saying why it is outside. `PRD.md` is therefore not edited by this
        change, which is the point of putting it outside.
      - **Testing → Unit-tested modules**: add `ui::layout`, `ui::app`, `ui::view`,
        `ui::driver`, `ui::terminal`, and `ui::load`, naming the `TestBackend` and
        `TerminalOps` doubles as what stands in for the terminal.
      - **User interface → Responsive layout**: record the two body constraints this change
        freezes — `Length(40)` for the change list, `Min(0)` for the artifact detail, so
        every column beyond 100 goes to detail — and state that `Enter`/`Esc` move the route
        at **every** width, selecting which region is emphasised above the breakpoint and
        which region is visible below it. Both are contract decisions `list-view`,
        `detail-view`, and `agent-launch` inherit, and neither is in `SPEC.md` today.
      - **User interface → List view**: the mock's widest row,
        `> add-token-refresh    [4/9]  > claude - working`, is 48 characters and cannot fit
        the 38-column interior `Length(40)` gives it. Annotate the mock as illustrating
        *content*, not width, and note that `list-view` owns how a row is shortened for a
        38-column interior. Decide it here rather than leaving `list-view` to discover it.
      - **Testing → Fixtures**: "Two mechanisms" becomes three — add
        `tests/fixtures/build-graph.txt`, a committed snapshot of tool output compared
        against a live `cargo tree` run, which is neither a `ScratchDir` tree nor an
        `include_str!` corpus.
      **Recorded:** all nine edits made (`git diff --stat SPEC.md`: +51/-9, 42 net — a bit
      over the estimated "roughly 30" since the Responsive layout and Degraded states
      additions grew somewhat in the writing, both still single, focused additions in the
      places named). The "no terminal is not a degraded state" note is its own `###`
      heading, matching every other subsection's heading level in the file (an `####` would
      have been the literal single-level-deeper reading of "below the table", but breaks
      the document's own heading convention with no compensating clarity). `PRD.md` left
      untouched, confirmed by `git status`.

- [x] 10.3 CHANGE: Rewrite `AGENTS.md`'s "Current repo state" paragraph in place — it says
      the dashboard is not implemented and `ui` prints a placeholder banner, which this
      change makes false. Replace, do not append. Also add one rule to "Architecture rules",
      rewriting rather than extending the existing "Views do no I/O" bullet: name
      `src/ui/terminal.rs` as the only file permitted to name a crossterm terminal-mode
      function, with the reason (a test that reaches a real terminal corrupts the
      developer's session, and `cargo test` spawns this binary). Net change: roughly neutral
      — the placeholder sentence goes, the raw-mode rule arrives.
      **Recorded:** both edits made in place (replaced, not appended). `git diff --stat
      AGENTS.md`: +36/-21, net +15 — more than "roughly neutral" predicted, because the
      replacement paragraph names the concrete landed behaviour (raw mode/alternate-screen
      lifecycle, the event loop's quit and route keys, the breakpoint, the still-empty body
      regions, and the terminal refusal) rather than a one-line summary; judged worth the
      extra length for a paragraph whose job is to tell the next reader what actually
      exists.

- [x] 10.4 CHECK: Confirm `openspec/IMPLEMENTATION-ORDER.md`'s Phase 4 `tui-shell` row still
      describes what was built. It names crossterm setup and teardown, the event loop, quit
      handling, the breakpoint, and the `TestBackend` harness — all five landed. Correct it
      only if something moved; record explicitly that nothing did, if nothing did.
      **Recorded:** re-read the row and its Mermaid dependency edges (`changes-from-files
      --> tui-shell`, `tui-shell --> list-view`, `tui-shell --> markdown-viewer`,
      `tui-shell --> agent-polling`) and the "`tui-shell` depends on `changes-from-files`,
      not on the CLI" paragraph below the diagram. All five scope items and all four edges
      match exactly what landed. **Nothing moved; no correction made.**

- [x] 10.5 VERIFY: `OPENSPEC-UNTOUCHED` again — 10.4 may have edited a file under
      `openspec/`. If it did, the check will fail on it, and the exclusion must be extended
      **by name** for exactly that path with a recorded reason, matching how
      `changes-from-cli` handled the same situation. If 10.4 changed nothing, the check
      passes unchanged. Commit.
      **Recorded:** 10.4 changed nothing, so no exclusion was needed. `OPENSPEC-UNTOUCHED
      OK` against the unmodified exclusion list. `make check` re-run after the
      documentation edits: exits 0, coverage unchanged at 97.82% (docs-only, no source
      touched).

---

## 11. Change Review
<!-- kind: operational -->

- [x] 11.1 Dispatch the finding pass to a reviewer that did not write this implementation,
      giving it only the change artifacts and the diff — never this session's reasoning.
      Do not fork the implementing session. Ask it to assign CRITICAL / WARNING / SUGGESTION
      and to change nothing.
      **Recorded:** dispatched to a fresh `outside-in-tdd-reviewer` subagent (not a fork),
      given the base SHA, the planning artifact paths, and the concentration-point brief
      below; it read the diff and planning docs itself and independently re-ran every
      architectural check, `make check`, and `cargo llvm-cov` rather than trusting the
      recorded numbers in this file.

- [x] 11.2 Concentration points to put in the reviewer's brief, this repository's first:
      - **A view test that opens a directory**, or a `Dashboard` built by reading disk in a
        `view.rs` test. That is logic leaking across the render seam.
      - **A view scenario asserted at one width only**, or asserted with the same assertion
        at both, which covers the breakpoint no better than one width does.
      - **A buffer assertion that names no cell** — "the buffer is non-empty", "no panic
        occurred", a `contains` over the whole buffer where a column range was specified.
      - **A filtered `cargo test` gate without a counted minimum.** A filter matching
        nothing exits 0.
      - **A check judged on a pipeline's exit status.** `grep` exits 1 for no-match and 2
        for a bad file, and `!` turns both into a pass.
      - **Any `src/ui/` file naming the CLI seam or a spawn API**, and any test constructing
        `CrosstermOps`.
      - **A `Dashboard` construction that does not name all five fields.**
      - **Leftovers**: placeholder text in a region interior, debug printing inside a
        render, a `dbg!`, commented-out layout experiments.
      **Recorded:** none of the eight concentration points produced a hit — the reviewer
      confirmed each mechanically (re-ran NOIO-VIEW/NOCLI-SHELL/NORAW-GREP/NODEFAULT-UI/
      WIDTHS, grepped for `dbg!`/`println!`/`todo!`/`TODO`/`FIXME`, checked every view test
      builds its `Dashboard` in memory). It also independently verified the five
      self-reported deviations recorded in groups 2, 4, 1, 7, and 9 above, and confirmed
      each sound by re-deriving it rather than trusting the note.

- [x] 11.3 Fix every CRITICAL. Resolve or consciously accept each WARNING with a one-line
      reason. Note SUGGESTIONs. Re-run every affected test and every affected check, and
      record the findings and their disposition in this file beneath this task.

      **Findings: 0 CRITICAL, 5 WARNING, 6 SUGGESTION.**

      **WARNING — all fixed:**
      1. `ui::driver::tests::a_draw_failure_stops_before_polling` asserted only
         `matches!(result, Err(LoopError::Draw(_)))`, never inspecting the payload the spec
         requires ("carrying the backend error's text"). **Fixed:** now asserts the payload
         contains `"FailingBackend always fails to draw"`.
      2. `ui::driver::TICK`'s value (spec: "SHALL be 250 milliseconds") was never asserted,
         only its use-site. **Fixed:** added `tick_is_250_milliseconds`, mirroring how
         `ui::layout::tests` pins `WIDE_MIN_WIDTH`.
      3. `StartError`'s `Display` impl and both `From` impls were entirely uncovered, which
         falsified `quality-gates`' coverage scenario's claim that the uncoverable residue
         is confined to a named, smaller set (`TerminalError`'s own `Display` *was* already
         tested, making the gap inconsistent within the change too). **Fixed:** three new
         tests in `ui::tests::start` cover `Display` for all three `StartError` variants and
         both `From` conversions; also added a `Debug`-format test for `TerminalGuard`
         (`ui::terminal::tests::guard::a_guard_formats_for_debug`), the other pure-but-untested
         surface the reviewer found. Corrected `specs/quality-gates/spec.md`'s coverage
         scenario to name what genuinely remains uncoverable: `main.rs`'s exit-0 arm
         (alongside the already-named exit-1 — neither reachable without a real terminal)
         and `ui::load`'s `canonicalize` fallback (a race no deterministic test constructs).
      4. `render`'s height-0 branch (`responsive-layout`: "at height 0 render SHALL draw
         nothing") had no view-tier test — only `split_frame`'s own height-0 case was
         covered, at the layout tier. **Fixed:** added
         `ui::view::tests::zero_height_frame_draws_nothing` (17th view test, still naming
         both 60 and 120 for `WIDTHS`); a 0-height `TestBackend` has no cells to read, so the
         assertion is "did not panic", the strongest claim a zero-cell buffer admits.
      5. Five sites hand-constructed `ChangeSet { active: vec![], archived: vec![], problems:
         vec![] }` outside `src/changes.rs` (`ui/app.rs`, `ui/view.rs`, `ui/driver.rs`,
         `lib.rs`'s `testutil::tests`, `ui/mod.rs`'s `ui::tests::load`) even though
         `changes::empty_set()` existed from group 7 onward — the exact construction site the
         design decision (`src/changes.rs` → `empty_set`'s doc comment; design.md → Decisions)
         exists to keep inside `GATE-MECH1`'s no-`Default`/no-`..` gate. None elided a field,
         so this was a WARNING, not CRITICAL — the invariant just wasn't mechanically
         enforced at those five sites. **Fixed:** all five now call `crate::changes::empty_set()`.

      **SUGGESTION — disposition:**
      1. A private, identically-shaped `fn empty_set() -> ChangeSet` in `src/changes.rs`'s
         own `mod merge` test module shadowed the new public one. **Fixed:** deleted; its ten
         call sites resolve to the public `empty_set()` through the existing `use super::*;`
         chain (`mod tests` → `mod merge`), confirmed by re-running `changes::tests::merge::*`.
      2. design.md's verification-matrix row for `render_at_touches_no_directory` prescribed
         snapshotting `std::env::temp_dir()`, which the actual (spec-correct) implementation
         does not do. **Fixed:** row corrected to describe the owned-`ScratchDir` approach and
         why, cross-referencing planning-review finding 2.
      3. Same file, the `quit_keys_and_their_near_misses` matrix row said `testcount --lib
         'ui::app::tests::' 8`; the count is 9 everywhere else. **Fixed:** typo corrected.
      4. `ui::view::tests::header_path_right_aligned_whole` used `.trim()` (accepts any
         Unicode whitespace) where the spec says "columns 8 through 45 are spaces". **Fixed:**
         matched the stricter form the sibling assertion at `region_interiors_are_blank`
         already uses (`.chars().all(|c| c == ' ')`).
      5. `split_frame`/`split_body` are `pub` (required — `ui::view` is a sibling module) but
         `split_body` was absent from design.md's Contracts code block and Boundaries table
         row. **Fixed:** both added to both places.
      6. The seven command-level checks (`NOIO-VIEW`, `NOCLI-SHELL`, `NORAW-GREP`,
         `NODEFAULT-UI`, `WIDTHS`, `GRAPH-SNAP`, `NOWAIVER`) exist only as fenced blocks in
         `tasks.md`, extracted to a scratchpad at implementation time — nothing in `Makefile`,
         CI, or `tests/` runs them, so a later change could violate any of them with every
         committed gate still green. **Accepted, not fixed:** this is this repository's
         established convention (`NOSPAWN-GREP` has been carried the same way across three
         prior changes) and is explicitly out of this change's scope — wiring seven checks
         into CI is a decision for a change that owns that surface, not a side effect of this
         one. Recorded here so the decision is deliberate rather than silent drift.

      **Re-verification after fixes:** full lib suite 443 → 449 tests (6 new: 1 driver
      `tick_is_250_milliseconds`, 1 view `zero_height_frame_draws_nothing`, 3 `StartError`
      `Display`/`From` tests, 1 `TerminalGuard` `Debug`-format test); `cargo fmt --all --
      --check` clean; `cargo clippy --all-targets --all-features -- -D warnings` clean;
      `openspec validate tui-shell
      --strict` → `Change 'tui-shell' is valid` (confirms the `specs/quality-gates/spec.md`
      edit didn't break the delta spec); `make check` exits 0; `cargo llvm-cov
      --fail-under-lines 80` → **98.01%** over 8,836 lines, 176 uncovered — up from 97.82%
      before this group (`ui/mod.rs` 78.78% → 93.36%, `ui/terminal.rs` 80.66% → 83.66%), floor
      cleared by 18.01 points.

---

## 12. Lint & Verify
<!-- kind: operational -->

- [x] 12.1 CHECK: Inspect the verification commands and the tiers they touch. `make check`
      runs format, lint, test, and coverage in order and stops at the first failure; the
      command-level checks are separate and are run explicitly below, because `make check`
      does not know about them.
      **Recorded:** confirmed against `Makefile`'s `check: fmt-check lint test coverage`
      target. Tasks 12.2–12.6 below reproduce those four tiers individually (so a failure
      is attributable to a specific sub-command, per the group's own point); 12.7 covers
      the eleven command-level checks `make check` does not run at all; 12.9 is the
      composite gate itself, last.

- [x] 12.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
      **Recorded:** `Finished` with no warnings — 0 errors.

- [x] 12.3 VERIFY: `cargo fmt --all -- --check` — clean.
      **Recorded:** exit 0, no diff.

- [x] 12.4 VERIFY: `cargo check --all-targets --all-features` — 0 errors. Rust has no
      separate type checker; this is the type-check step, run separately from clippy so a
      clippy lint failure is distinguishable from a type error.
      **Recorded:** `Finished` — 0 errors.

- [x] 12.5 VERIFY: `cargo test --all-features` — green. Then `testcount --lib '' 443` for
      the whole library: the task-1.1 baseline of 389, minus the deleted `banner` test,
      plus 55 new tests (6 layout + 9 app + 3 testutil + 16 view + 8 terminal + 6 driver +
      4 load + 3 start). And `testcount --test-cli '' 5`.
      **Recorded:** all green — 449 lib + 11 `ci_workflow` + 5 `cli`, 0 failed.
      `TESTCOUNT OK: --lib filter '' ran 449 tests (>= 443)` — 6 above the plan's own
      prediction, because group 11's Change Review fixed 5 WARNING findings by adding 6
      tests (1 driver, 1 view — bringing view to 17, not the planned 16 — 3 `StartError`,
      1 `TerminalGuard`) that were not anticipated at planning time; every count-based gate
      in this file (`testcount`, `WIDTHS`'s 17-test report) reflects the actual, larger
      number, not the stale prediction. `TESTCOUNT OK: --test-cli filter '' ran 5 tests
      (>= 5)`.

- [x] 12.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — the floor is unchanged, with no
      exclusion and no waiver; `NOWAIVER` proves the second half. Record the TOTAL.
      **Recorded:** exit 0. **TOTAL 98.01%** over 8,836 lines, 176 uncovered — up from
      97.82% before group 11's coverage fixes and from the 98.93%/7,635-line/82-uncovered
      task-1.1 baseline, the gap between the two being the argued, now-accurately-named
      residue in `specs/quality-gates/spec.md`. Floor (80) cleared by 18.01 points.
      `NOWAIVER OK`.

- [x] 12.7 VERIFY: Re-run every command-level check one final time against the finished
      tree: `NOSPAWN-GREP` with `MIN=14`, `NOIO-VIEW`, `NOCLI-SHELL`, `NORAW-GREP`,
      `NODEFAULT-UI`, `GATE-MECH1`, `NOJSON-SEAM`, `DEPS` (all legs), `GRAPH-SNAP`,
      `WIDTHS`, `NOWAIVER`, and `OPENSPEC-UNTOUCHED`.
      **Recorded:** all twelve pass. `NOSPAWN OK: 15 files (>= 14)`; `NOIO-VIEW OK`;
      `NOCLI-SHELL OK: 7 files`; `NORAW OK: 17 files searched … CrosstermOps at 6 sites`;
      `NODEFAULT-UI OK`; `GATE-MECH1 OK` half A (16 files) and half B — **61 constructions**,
      back down from group-9's 63: deleting the shadowing test-local `empty_set()` helper in
      group 11 removed its own two matching sites (signature + body), landing exactly back
      on the pre-`tui-shell` baseline even though the production `empty_set()` is a
      genuinely new, additional construction site — a coincidence of the count, not a sign
      anything reverted; `NOJSON-SEAM OK`; `DEPS OK` all legs including leg 5 for all four
      crates; `GRAPH-SNAP OK`; `WIDTHS OK: all 17 view tests name both 60 and 120`;
      `NOWAIVER OK`; `OPENSPEC-UNTOUCHED OK` against
      `BASE=e25017546cea99e4b606fcd74a404131e1ca582e`.

- [x] 12.8 VERIFY: `openspec validate tui-shell --strict` — with
      `PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"`, since the CLI is nvm-installed
      and is not on the default `PATH`.
      **Recorded:** `Change 'tui-shell' is valid`.

- [x] 12.9 VERIFY: `make check` — the single composite gate, exit 0. It runs last, after
      group 10's documentation edits and group 11's review fixes, so the finished tree is
      the tree the gate sees. If it fails, name the failing sub-command rather than
      summarising.
      **Recorded:** `make check` exits 0 — `fmt-check`, `lint`, `test` (465 tests total
      across all three test binaries plus the lib), `coverage` (98.01%, floor 80 cleared)
      all pass, in order, on the finished tree. Apply complete.
