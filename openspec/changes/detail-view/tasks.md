# detail-view — tasks

Read `design.md` first: the Boundaries table, the Test Boundaries table, and the verification
matrix are the contract these tasks implement. **No task may invent a collaborator that table
does not name.**

**The outer loop is taken, at the `ui::` composition tier through `run_loop`.**
`ui::app::action_for` → `Dashboard::apply` → `Dashboard::sync_detail` → `ui::detail::*` →
`ui::view::render` → `Dashboard::normalise_scroll` is a path no unit test crosses, and it is
the thing this change exists to make work. Group 2 writes that test and group 10 closes it.

**Two groups come before the acceptance test, and that is deliberate**, on the same terms
`markdown-viewer` recorded: the acceptance test needs three new `Detail` fields, a new
`run_loop` parameter, and a recording reader double in `crate::testutil` before it can
compile. Group 0 measures the tree before anything is edited; group 1 is **operational**,
because a field, a signature, and a test double are plumbing rather than behaviour; group 2
is the behavioural acceptance group, and its RED is an assertion failure against a detail
region showing no header rather than a compile failure. A crate whose test target does not
build makes `cargo clippy --all-targets` and `cargo llvm-cov --ignore-run-fail` unrunnable,
and would leave groups 3 to 9 with no gate at all.

**Every floor in this file is measured before it is raised.** Task 0.1 reads the real counts
off `main` and records them; every later `MIN`, `UI_MIN`, `WIDTHS_MIN`, `LIST_MIN`, `MD_MIN`,
`DETAIL_MIN`, and `testcount` minimum is *measured + this change's addition*. This repository
has already shipped a check whose floor was asserted rather than measured and was therefore
unpassable.

**When a task says a check script was edited, that task writes the script to disk and runs
it.** `tui-shell` recorded four edits to `DEPS` as prose in a checkbox only; the script on
disk was never changed, and `list-view` nearly halted on a check that did not exist. Every
block reproduced below is extracted to `$CHECKS/<LABEL>.sh` in task 0.1, and every later run
is of the extracted file. Blocks that are **not** reproduced below are extracted
byte-identically from a named archived file, and task 0.2 diffs each extraction against its
source so an extraction that silently produced the wrong bytes fails there rather than
passing a check that is not the one anybody reviewed.

**`DEPS` and `GRAPH-SNAP` change not at all, and that is a claim to prove, not to skip.**
This change adds no dependency, so both are extracted byte-identically and both must pass
**unchanged**. A passing run of the unchanged scripts is the evidence that no dependency, no
feature, and no proc-macro crept in.

## Test-module convention — read before writing any test

`cargo test` matches a filter against a test's **full path**, and a filter matching nothing
exits 0. Every filtered gate below therefore addresses tests through an explicit module path,
and is judged on a counted minimum rather than on the run's exit status.

Every target below is **measured at `main` plus the number of NEW test functions the group
enumerates**, and the two are written out so the arithmetic can be checked rather than
trusted. A scenario that *modifies* an existing test adds nothing to the count, and several
of this change's scenarios do exactly that — a floor derived from "scenarios in the group"
rather than "new test functions in the group" would be unreachable.

| Group | File | Module | Filter | Measured | New | Target |
|---|---|---|---|---|---|---|
| 3 | `src/ui/layout.rs` | `mod tests` | `ui::layout::tests::` | 12 | 3 | 15 |
| 4, 5, 6 | `src/ui/detail.rs` | `mod tests` | `ui::detail::tests::` | 0 | 19 | 19 |
| 7, 8 | `src/ui/app.rs` | `mod tests` | `ui::app::tests::` | 30 | 15 | 45 |
| 9 | `src/ui/view.rs` | `mod tests` | `ui::view::tests::` | 57 | 10 | 67 |
| 11 | `src/ui/driver.rs` | `mod tests` | `ui::driver::tests::` | 10 | 2 | 12 |
| 11 | `src/ui/mod.rs` | `mod tests` → `mod load` | `ui::tests::load::` | 6 | 1 | 7 |
| 1 | `src/ui/mod.rs` | `mod tests` → `mod read_artifact` | `ui::tests::read_artifact::` | 0 | 2 | 2 |
| 2, 10 | `src/ui/mod.rs` | `mod tests` → `mod detail` | `ui::tests::detail::` | 1 | 1 | 2 |

**`ui::tests::detail::` already holds one test on `main`** — `markdown-viewer`'s
`a_markdown_document_renders_and_scrolls_through_the_loop`. A floor of 1 would therefore be
satisfied before this change writes a line. Group 2 adds a **second** test function to that
module, and group 10 **updates** the existing one, whose `scroll == 4` / `- line-04` /
`frames == 11` expectations this change invalidates.

The **library** total is `565` measured plus `3 + 19 + 15 + 10 + 2 + 1 + 2 + 1 = 53`, i.e.
**618**; the `ui::` subtotal is `172` measured plus the same 53, i.e. **225**. Both are
asserted in group 15 and nowhere else.

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
The literal, unqualified `make check` is run from task 10.4 onward.

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
# Carried forward from markdown-viewer with its executable logic BYTE-IDENTICAL. Only its
# MIN argument moves, from 17 to 18, and MIN is a parameter precisely so that is a change to
# a task's invocation rather than to the check.
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
# Carried forward from markdown-viewer with ONE deliberate edit, recorded in design.md ->
# Boundaries: PURE gains src/ui/detail.rs, so the set is SEVEN files rather than six.
# Three files under src/ui/ are still deliberately NOT searched, each for a stated reason:
# mod.rs holds ui::load, ui::read_artifact, and ui::run; terminal.rs holds the terminal seam;
# and event.rs holds CrosstermEvents, which reads the real event stream. Do not "fix" the
# list by adding them. A view test that needs a real directory is the signal this check
# exists to make impossible.
#
# This is the check that guards detail-view's central claim. The artifact read arrives as an
# injected `&dyn Fn(&Path) -> Result<String, String>`, which names none of the patterns
# below, so src/ui/app.rs and src/ui/driver.rs stay in the PURE set while carrying the
# reader through. READSEAM is the complementary half: it proves the single real binding is
# where design.md says it is.
PURE="src/ui/app.rs src/ui/detail.rs src/ui/layout.rs src/ui/list.rs src/ui/markdown.rs src/ui/view.rs src/ui/driver.rs"
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
echo "NOIO-VIEW OK: 7 pure files carry no I/O API; positive control matched"
```

```sh
# READSEAM — NEW in detail-view. The artifact read has exactly ONE binding under src/ui/,
# on the same terms src/cli.rs is the crate's only process spawner: `read_to_string` is
# named under src/ui/ only in src/ui/mod.rs, so the read can be replaced, faked, or moved
# behind a worker thread by editing one file. NOIO-VIEW is the complementary half: it proves
# the seven pure files name no I/O API at all.
#
# Known limit, stated rather than discovered later: this is a grep over whole files,
# comments included. src/ui/detail.rs's doc comment must therefore say "the reader" rather
# than naming read_to_string, exactly as src/ui/markdown.rs's must say "the view" rather
# than naming ratatui. That is a deliberate cost: an exemption for comments is how a
# confinement check rots into a rubber stamp.
UIDIR="${UIDIR:-src/ui}"
BINDING="${BINDING:-src/ui/mod.rs}"
UI_MIN="${UI_MIN:-7}"
fail() { echo "READSEAM FAIL: $1" >&2; exit 1; }

[ -d "$UIDIR" ] || fail "no such directory: $UIDIR"
[ -f "$BINDING" ] || fail "$BINDING missing - the exclusion has nothing to exclude"

# Guard B — positive control, checked BEFORE the count so a gutted binding is reported as
# vacuous rather than as a file-count shortfall. The allowed file must actually perform the
# read, or the exclusion protects nothing and a clean result means nothing.
grep -qE 'read_to_string' "$BINDING" \
  || fail "$BINDING names no read_to_string - exclusion is vacuous"

# Guard C — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/detail/mod.rs is still searched.
n=$(find "$UIDIR" -name '*.rs' ! -path "$BINDING" | wc -l | tr -d ' ')
[ "$n" -ge "$UI_MIN" ] || fail "searched only $n files under $UIDIR (expected >= $UI_MIN)"

hits=$(find "$UIDIR" -name '*.rs' ! -path "$BINDING" -print0 \
       | xargs -0 -I{} grep -nE 'read_to_string' {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "READSEAM FAIL: read_to_string outside $BINDING:" >&2
                    echo "$hits" >&2; exit 1; }

# Second leg — the binding is wired exactly once in production. `read_artifact` is named in
# src/ui/mod.rs (its definition and ui::run's use of it) and may be named in a test module,
# but it must NOT be named in any other src/ui/ file: run_loop and sync_detail take a
# `&dyn Fn`, never this function by name, or the injection would be decorative.
bad=$(find "$UIDIR" -name '*.rs' ! -path "$BINDING" -print0 \
      | xargs -0 -I{} grep -nE 'read_artifact' {} /dev/null 2>&1 || true)
[ -z "$bad" ] || { echo "READSEAM FAIL: read_artifact named outside $BINDING:" >&2
                   echo "$bad" >&2; exit 1; }
echo "READSEAM OK: $n files searched under $UIDIR (>= $UI_MIN), read_to_string and read_artifact only in $BINDING"
```

```sh
# NOCLI-SHELL — the dashboard shell never names the CLI seam. This is the mechanical form
# of the roadmap's "detail-view depends on list-view and markdown-viewer, and through them on
# changes-from-files, not on the CLI".
# Carried forward from markdown-viewer with its executable logic BYTE-IDENTICAL; only
# UI_MIN's invocation moves, 9 -> 10.
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
# destructuring elides a field. EDITED here, and this is the ONE landed check detail-view
# makes STRONGER rather than merely re-running. Both halves, the TYPES list, and every
# positive control are kept; what changes is HOW half B looks.
#
# WHY it changes: half B was a same-line grep — `$T[[:space:]]*\{[^}]*\.\.` — and this
# repository has already shipped a version of it that missed FIFTEEN `..base` struct-update
# elisions, because rustfmt put the `..` on a later line than the opening brace. `Detail`
# growing from two fields to five is exactly the shape rustfmt spreads over seven lines, so
# the blind spot would be wider after this change than before it. The old note said "a
# multi-line elision is caught by the compile-time companions in app.rs's tests instead" —
# that claim is FALSE and is retired here: the companion destructures ONE value, so it
# catches a field added to the type, never an elision at some other site. Half B is now a
# brace-matching pass that sees past a line break, and the companion is kept for what it
# genuinely does.
#
# change-model's GATE-MECH1 covers Change/ChangeSet/ArtifactRef/Origin in src/changes.rs
# ONLY, so a state type in src/ui/app.rs is outside it.
SRC="${SRC:-src}"
# `${TYPES-...}` and NOT `${TYPES:-...}`: with the colon, an explicitly empty TYPES silently
# falls back to the default and the emptiness guard below can never fire. The guard also
# rejects a whitespace-only value, which `[ -n ]` alone would accept and which would make
# the loop body run zero times while still printing OK.
TYPES="${TYPES-Dashboard Filter Detail}"
# The number of literal/pattern spans half B must FIND before a clean result means anything.
# Measured in task 0.1; a broken regex that matches nothing would otherwise print OK.
# Measured at planning time on unmodified `main`: 68 spans, 0 elisions. The default is 60,
# comfortably below that and comfortably above zero; the invocation raises it if a later
# change wants it tighter.
SCAN_MIN="${SCAN_MIN:-60}"
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

  # Half A — no Default impl anywhere under src/, hand-written or derived. The impl form
  # now also matches a PATH-QUALIFIED target (`impl Default for crate::ui::app::Detail`),
  # which the previous `for[[:space:]]+$T` form silently missed. The derive form is caught
  # by taking the two lines preceding each `struct <T>` and looking for Default inside a
  # #[derive(...)] there.
  a=$(find "$SRC" -name '*.rs' -print0 \
      | xargs -0 -I{} grep -nE "impl[[:space:]]+Default[[:space:]]+for[[:space:]]+([A-Za-z0-9_]+::)*$T([[:space:]]|\{|$)" {} /dev/null 2>&1 || true)
  b=$(find "$SRC" -name '*.rs' -print0 \
      | xargs -0 -I{} grep -B2 -nE "struct[[:space:]]+$T[[:space:]]*\{" {} /dev/null 2>&1 \
      | grep 'derive' | grep 'Default' || true)
  [ -z "$a$b" ] || { echo "NODEFAULT-UI FAIL: $T has a Default:" >&2
                     printf '%s\n%s\n' "$a" "$b" >&2; exit 1; }
done

# Half B — no `..` inside a <T> literal or pattern, SINGLE-LINE OR MULTI-LINE. Brace-matched
# from the opening `{` to its partner, so rustfmt's own seven-line spread of a five-field
# literal is seen. Line comments are stripped first, so a `..` inside prose is not reported;
# a `..` in a nested literal inside a <T> literal IS reported, which errs strict on purpose.
# A range expression (`0..10`, `a..=b`) is excluded by requiring the `..` to follow a brace,
# a comma, or whitespace.
SRC="$SRC" TYPES="$TYPES" SCAN_MIN="$SCAN_MIN" python3 - <<'PY'
import os, re, sys, pathlib

src = pathlib.Path(os.environ["SRC"])
types = os.environ["TYPES"].split()
scan_min = int(os.environ["SCAN_MIN"])
# Preceded by one of these keywords, a `<T> {` is a declaration or a signature, not a
# literal: `struct T {`, `impl T {`, `-> T {`, `enum`, `union`, `trait`, `impl Default for T {`.
SKIP_BEFORE = re.compile(r"(struct|enum|union|trait|impl|for|->)\s*$")
ELISION = re.compile(r"(?:^|[\s,{])\.\.(?!\.)")
LINE_COMMENT = re.compile(r"//.*$")

def strip_comments(text):
    return "\n".join(LINE_COMMENT.sub("", line) for line in text.splitlines())

scanned = 0
bad = []
files = sorted(src.rglob("*.rs"))
if not files:
    print(f"NODEFAULT-UI FAIL: no *.rs files under {src}", file=sys.stderr); sys.exit(1)
for path in files:
    text = strip_comments(path.read_text())
    for T in types:
        for m in re.finditer(r"(?<![A-Za-z0-9_])" + re.escape(T) + r"\s*\{", text):
            if SKIP_BEFORE.search(text[:m.start()]):
                continue
            scanned += 1
            depth, i, n = 0, m.end() - 1, len(text)
            while i < n:
                if text[i] == "{":
                    depth += 1
                elif text[i] == "}":
                    depth -= 1
                    if depth == 0:
                        break
                i += 1
            span = text[m.end():i]
            if ELISION.search(span):
                line = text[:m.start()].count("\n") + 1
                bad.append(f"{path}:{line}: {T} literal or pattern elides a field")
if scanned < scan_min:
    print(f"NODEFAULT-UI FAIL: half B found only {scanned} literal/pattern spans "
          f"(expected >= {scan_min}) - the scan is vacuous", file=sys.stderr)
    sys.exit(1)
if bad:
    print("NODEFAULT-UI FAIL: multi-line-aware half B:", file=sys.stderr)
    print("\n".join(bad), file=sys.stderr)
    sys.exit(1)
print(f"NODEFAULT-UI OK (half B): {scanned} literal/pattern spans scanned (>= {scan_min}), none elides a field")
PY
echo "NODEFAULT-UI OK: no Default for [$TYPES], no elided field; positive controls matched"
```

```sh
# NOLIT-CHANGE — `Change { … }` and `ChangeSet { … }` literals and patterns appear ONLY in
# src/changes.rs. Carried forward from markdown-viewer with its executable logic
# BYTE-IDENTICAL; only its MIN invocation moves, 17 -> 18, because src/ui/detail.rs is added.
# This change creates NO new Change or ChangeSet construction site: the tab-bar and header
# fixtures reach one through `changes::fixture::with_artifacts`, which is added INSIDE
# src/changes.rs precisely so this check does not have to be weakened.
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
# MDSEAM — the markdown parser is confined to ONE module, and that module names no ratatui
# type and no pulldown_cmark::html path. Carried forward from markdown-viewer with its
# executable logic BYTE-IDENTICAL; only its MIN invocation moves, 17 -> 18.
#
# It is re-run rather than edited because src/ui/detail.rs calls `ui::markdown::lines` — the
# module's public, plain-data entry point — and names pulldown_cmark nowhere. If a future
# implementer reaches for the parser directly from ui::detail, this is the check that stops
# them.
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
```

```sh
# WIDTHS — every #[test] in src/ui/view.rs names both 60 and 120. A heuristic, and design.md
# -> Risks says so: it cannot prove an assertion is meaningful, only that both widths are
# present. It is the cheap half of the mandate; the per-scenario tasks are the other half.
# Carried forward from markdown-viewer with its executable logic BYTE-IDENTICAL; only
# WIDTHS_MIN's invocation moves, 57 -> 72.
#
# Known limits, stated rather than discovered later: a `60` in a comment satisfies it, and
# so does an unrelated `60` literal. It is a FLOOR. What proves the tests exist and run is
# `testcount --lib 'ui::view::tests::' 72`, not this.
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
# DETAILWIDTHS — NEW in detail-view. Every #[test] in src/ui/detail.rs names both 58 and 78,
# the two DETAIL-region interiors the mandated 60- and 120-column frames produce (a
# 60-column body less two border columns, and the wide layout's Min(0) column of 80 less two
# border columns). Same script as LISTWIDTHS and MDWIDTHS pointed at a different file and the
# same pair MDWIDTHS uses; same stated limits; same pairing — it is a FLOOR, and
# `testcount --lib 'ui::detail::tests::' 22` is what proves the tests exist and run.
# Because this block is new here, its default IS this change's final floor: 22.
#
# Known limit inherited from WIDTHS: the number scan is `\b(\d+)\b`, which does NOT see a
# suffixed literal such as `78u16`. It fails closed — a test using only suffixed literals is
# reported as missing a width — so write the widths unsuffixed.
#
# It has NO exemption list, and design.md -> Boundaries records the module split that makes
# that possible: every public function in src/ui/detail.rs is parameterised by a width.
# `split_detail` lives in ui::layout (it is Rect geometry) and `sync_detail` lives in ui::app
# (it mutates state and has no width) precisely so that no test in this file has a legitimate
# reason to name neither width. An exemption list is how a width check rots into a rubber
# stamp.
[ -f src/ui/detail.rs ] || { echo "DETAILWIDTHS FAIL: src/ui/detail.rs missing" >&2; exit 1; }
DETAIL_MIN="${DETAIL_MIN:-22}" python3 - <<'PY'
import re, sys, os
raw = open("src/ui/detail.rs").read()
src = "\n".join(l for l in raw.splitlines()
                if not l.lstrip().startswith("///") and not l.lstrip().startswith("//!"))
parts = re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", src)[1:]
floor = int(os.environ["DETAIL_MIN"])
if len(parts) < floor:
    print(f"DETAILWIDTHS FAIL: found {len(parts)} #[test] functions in src/ui/detail.rs, expected >= {floor}",
          file=sys.stderr); sys.exit(1)
bad = []
for p in parts:
    name = re.search(r"fn\s+([a-z0-9_]+)", p)
    name = name.group(1) if name else "<unnamed>"
    nums = set(re.findall(r"\b(\d+)\b", p))
    if not ({"58", "78"} <= nums):
        bad.append(name)
if bad:
    print("DETAILWIDTHS FAIL: these detail tests do not name both 58 and 78: " + ", ".join(bad),
          file=sys.stderr); sys.exit(1)
print(f"DETAILWIDTHS OK: all {len(parts)} detail tests name both 58 and 78")
PY
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
# This change is the FIRST to add a read path into openspec/ (ui::read_artifact reads the
# artifact files changes::from_files resolved), so this check stops being a formality here:
# a `read_to_string` that was accidentally a `write` would show up as a modified tracked
# file, and a stray temporary as an untracked one.
#
# Exactly ONE exclusion: this change's own artifact directory, a hand-edited planning
# document rather than a code-path write. Excluded BY NAME, never by a broad prefix, so a
# stray file anywhere else under openspec/ still fails. Carried forward from markdown-viewer
# with that one path updated.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --ignored --exclude-standard -- ':(top)openspec/'; } \
         | grep -v '^openspec/changes/detail-view/' | sort -u || true)
[ -z "$stray" ] || { echo "FAIL: wrote inside openspec/ outside this change:" >&2
                     echo "$stray" >&2; exit 1; }
echo "OPENSPEC-UNTOUCHED OK"
```

**Extracted byte-identically rather than retyped** — nine blocks, none of which changes. From
`openspec/changes/archive/2026-09-05-markdown-viewer/tasks.md`: `NORAW-GREP.sh`,
`LISTWIDTHS.sh`, `MDWIDTHS.sh`, `NOWAIVER.sh`, `TESTCOUNT.sh`, `DEPS.sh`, and
`GRAPH-SNAP.sh`. From `openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md`:
`GATE-MECH1.py` and `NOJSON-SEAM.sh`. Task 0.2 diffs every one of those extractions against
its source block, so an extraction that produced the wrong bytes fails there rather than
passing a check nobody reviewed.

**`NODEFAULT-UI` is NOT in that list any more**, and that is the one substantive check edit
this change makes: its half B is reproduced above in a brace-matching form that sees a
multi-line elision, because the same-line form has already missed fifteen of them in this
repository and `Detail` growing to five fields widens exactly that blind spot. Its `TYPES`
stays `Dashboard Filter Detail`.

Together: **eleven** blocks reproduced above (`NOSPAWN-GREP`, `NOIO-VIEW`, `READSEAM`,
`NOCLI-SHELL`, `NODEFAULT-UI`, `NOLIT-CHANGE`, `MDSEAM`, `NOTABSEAM`, `WIDTHS`,
`DETAILWIDTHS`, `OPENSPEC-UNTOUCHED`) and **nine** extracted from the archives, for **twenty**
files in `$CHECKS`.

---

## 0. Baseline, checks, and the scratchpad
<!-- kind: operational -->

- [x] 0.1 CHECK: Record the starting state before any edit, by **measuring**, never by
      copying a number from this file.
      - `git rev-parse HEAD` → `export BASE=<sha>`. Every `OPENSPEC-UNTOUCHED` run uses it.
        A diff against the index would pass over this change's own per-group commits.
        **Measured at planning time: `d3b798cac2281f533b079fcf7e6bd1be96b31cfc`.**
      - `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
        passed.*/\1/p' | awk '{t+=$1} END{print t+0}'` → the **lib** baseline.
        **Measured at planning time: 565**, inside a 581-test whole suite.
      - `find src -name '*.rs' ! -path 'src/cli.rs' | wc -l` → `NOSPAWN-GREP`'s starting file
        count. **Measured at planning time: 17**; the floor becomes 18.
      - `find src -name '*.rs' ! -path 'src/changes.rs' | wc -l` → `NOLIT-CHANGE`'s starting
        file count. **Measured at planning time: 17**; the floor becomes 18.
      - `find src -name '*.rs' ! -path 'src/ui/markdown.rs' | wc -l` → `MDSEAM`'s starting
        file count. **Measured at planning time: 17**; the floor becomes 18. All three of
        these are 17 today and 18 after, because `src/ui/detail.rs` is excluded by none of
        them — unlike `markdown-viewer`, where `MDSEAM`'s count did **not** move. Do not
        copy that change's asymmetry.
      - `find src/ui -name '*.rs' | wc -l` → `NOCLI-SHELL`'s `UI_MIN` baseline.
        **Measured at planning time: 9**; the floor becomes 10.
      - `find src/ui -name '*.rs' ! -path 'src/ui/mod.rs' | wc -l` → `READSEAM`'s `UI_MIN`
        baseline. **Measured at planning time: 8**; the floor becomes 9.
      - `find src tests -name '*.rs' ! -path 'src/ui/terminal.rs' | wc -l` → `NORAW-GREP`'s
        guard. **Measured at planning time: 19** (its floor is 16 and does not move).
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/view.rs` → `WIDTHS`' baseline.
        **Measured at planning time: 57**; the floor becomes 72.
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/list.rs` → `LISTWIDTHS`' baseline.
        **Measured at planning time: 17** (its floor is 17, exactly met today and unmoved).
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/markdown.rs` → `MDWIDTHS`' baseline.
        **Measured at planning time: 24** (its floor is 23 and does not move; this change
        adds no markdown test).
      - `grep -cE '^[ \t]*#\[test\][ \t]*$' src/ui/app.rs` → **30**;
        `src/ui/layout.rs` → **12**; `src/ui/driver.rs` → **10**; `src/ui/mod.rs` → **13**.
      - The three `src/ui/mod.rs` submodule counts, which the whole-file 13 hides and which
        three separate floors depend on. Measure them by running each filter:
        `cargo test --all-features --lib 'ui::tests::load::'` → **6**;
        `'ui::tests::detail::'` → **1** (`markdown-viewer`'s acceptance test, which already
        satisfies a floor of 1 — group 2's floor is therefore 2);
        `'ui::tests::start::'` → **6**. `ui::tests::read_artifact::` does not exist yet.
      - `cargo test --all-features --lib 'ui::' 2>&1 | sed -n 's/^test result: ok\.
        \([0-9]*\) passed.*/\1/p' | awk '{t+=$1} END{print t+0}'` → the `ui::` subtotal.
        **Measured at planning time: 172.**
      - `NODEFAULT-UI`'s new half B, run with `SCAN_MIN=1`, prints the number of
        literal/pattern spans it scanned. **Measured at planning time: 68 spans, 0
        elisions**; the default floor is 60.
      - `grep -rn 'Dashboard[[:space:]]*{' src/ | wc -l` → the `Dashboard` construction-site
        sweep group 1 must complete. **Measured at planning time: 46 grep hits, of which 29
        are real literal or pattern sites**, across six files: `src/lib.rs`, `src/ui/mod.rs`,
        `src/ui/list.rs`, `src/ui/driver.rs`, `src/ui/view.rs`, `src/ui/app.rs`. The other 17
        hits are `pub struct Dashboard {`, `impl Dashboard {`, and `-> Dashboard {`
        signatures.
      - `grep -rn 'Detail[[:space:]]*{' src/ | wc -l` → the `Detail` sweep.
        **Measured at planning time: 25 grep hits, of which 20 are real literal or pattern
        sites**, across the same six files. This number is recorded for orientation only:
        task 1.2's gate is the **compiler**, not this grep, because the grep cannot
        distinguish a doc-comment mention from a literal and only ever grows.
      - `cargo llvm-cov --summary-only` → TOTAL **line** coverage. The TOTAL row leads with
        the **region** count; read the line column, further right.
        **Measured at planning time: 98.04% over 12,267 lines.**
      - `export CHECKS=<scratchpad>/detail-view-checks` and
        `export WORK=<scratchpad>/detail-view-work`; `mkdir -p "$CHECKS" "$WORK"`.
        Extract every fenced block above to `$CHECKS/<LABEL>.sh` byte-identically — eleven of
        them — and extract `NORAW-GREP.sh`, `LISTWIDTHS.sh`, `MDWIDTHS.sh`,
        `NOWAIVER.sh`, `TESTCOUNT.sh`, `DEPS.sh`, and `GRAPH-SNAP.sh` from
        `openspec/changes/archive/2026-09-05-markdown-viewer/tasks.md`, and `GATE-MECH1.py`
        and `NOJSON-SEAM.sh` from
        `openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md`, all byte-identically.
        `DEPS` needs `WORK` set and refuses to run without it.
        Run the extracted files from here on, never a retyped copy. `TESTCOUNT.sh` is the
        exception: it only *defines* a function, so **source** it (`. $CHECKS/TESTCOUNT.sh`)
        in each shell that runs a gate.
      **Red when:** `BASE` is empty or not a commit (`OPENSPEC-UNTOUCHED` aborts on both), or
      a measured count differs from the planning-time figure recorded above, in which case
      the tree is not the one this plan was written against and the discrepancy is resolved
      before any edit.

- [x] 0.2 CHECK: Prove the extraction landed, rather than assuming it. For each of the **nine**
      files extracted from an archived tasks file, extract the same block a second time by an
      independent route and `diff` the two; every diff must be empty. Then `ls -1 "$CHECKS" |
      wc -l` and confirm **twenty** files are present — the eleven reproduced in this file
      (`NOSPAWN-GREP`, `NOIO-VIEW`, `READSEAM`, `NOCLI-SHELL`, `NODEFAULT-UI`,
      `NOLIT-CHANGE`, `MDSEAM`, `NOTABSEAM`, `WIDTHS`, `DETAILWIDTHS`,
      `OPENSPEC-UNTOUCHED`) plus the nine from the archives (`NORAW-GREP`, `LISTWIDTHS`,
      `MDWIDTHS`, `NOWAIVER`, `TESTCOUNT`, `DEPS`, `GRAPH-SNAP`, `GATE-MECH1.py`,
      `NOJSON-SEAM`). There is no overlap between the two lists. Record the exact count found
      and `ls -1 "$CHECKS"` verbatim.
      **Red when:** the count is not twenty, any diff is non-empty, or any script name a
      later task invokes is absent from that listing. A later task that runs a script which
      does not exist is exactly the failure `list-view` nearly hit.

- [x] 0.2b CHECK: Prove the "byte-identical executable logic" claim for the five blocks whose
      **comments** move but whose code does not — `NOSPAWN-GREP`, `NOCLI-SHELL`,
      `NOLIT-CHANGE`, `MDSEAM`, and `WIDTHS`. For each, extract `markdown-viewer`'s version
      to `$WORK/<LABEL>.prev.sh`, strip full-line comments from both it and
      `$CHECKS/<LABEL>.sh`, and `diff` the two; every diff must be empty.
      **Red when:** any diff shows an executable line. A silently-altered check that still
      calls itself carried forward is worse than an openly edited one.

- [x] 0.3 CHECK: Run the checks that must **pass** on the tree as it stands, so this change
      starts from clean gates rather than inheriting broken ones: `NOSPAWN-GREP` (default
      `MIN=8`), `NOCLI-SHELL` (default `UI_MIN=7`), `NORAW-GREP`, `NODEFAULT-UI` with
      `TYPES="Dashboard Filter Detail" SCAN_MIN=60` (its **edited** form — it must be green
      on `main` before this change touches anything, and its half B must report ≥60 spans
      scanned), `NOLIT-CHANGE` (default `MIN=15`), `MDSEAM`
      (default `MIN=16`), `WIDTHS` with `WIDTHS_MIN=57`, `LISTWIDTHS` with `LIST_MIN=17`,
      `MDWIDTHS` with `MD_MIN=23`, `NOWAIVER`, `GATE-MECH1`, `NOJSON-SEAM`,
      `OPENSPEC-UNTOUCHED`, and `GRAPH-SNAP`. Record each script's OK line verbatim.
      **Red when:** any of them fails on unmodified `main`, which would mean the baseline is
      not what `markdown-viewer`'s archive claims. `NODEFAULT-UI`'s edited half B failing here
      would mean the tree **already** carries an elision the old same-line form could not
      see — resolve that before any edit, and record it in `planning-review.md`.
      **Not run here:** `NOIO-VIEW` (its `PURE` list names `src/ui/detail.rs`, which does not
      exist yet — it is run for the first time in group 4), `READSEAM`, `NOTABSEAM`, and
      `DETAILWIDTHS`, all three of which name a file this change creates.

- [x] 0.4 CHECK: Run `DEPS` once on unmodified `main`, with `WORK` set and
      `DEPS_SKIP_LEG5=1`, and record its OK lines. This change adds no dependency, so this
      run is the **baseline** the identical run in group 12 is compared against, and the two
      must agree line for line.
      **Red when:** `DEPS` fails on `main`, or its output here differs from what
      `markdown-viewer`'s archive recorded, which would mean the dependency set moved between
      changes.

- [x] 0.5 Commit: `chore(detail-view): record the pre-change baseline and extract the checks`.
      Nothing under `src/` has been edited; the commit carries only this file's checkmarks
      and the recorded measurements.

## 1. The state, the signature, and the doubles
<!-- kind: operational -->

- [x] 1.1 Add the three `Detail` fields — `tab: usize`, `problems: Vec<String>`,
      `loaded: Option<(PathBuf, usize)>` — to `ui::app::Detail`, with the doc comments
      `design.md` → Contracts states. Add no behaviour: nothing reads or writes them yet.

- [x] 1.2 Sweep every `Detail` literal and pattern so the crate compiles. The **measured**
      grep count is 25 hits, of which 20 are real sites across six files (`src/lib.rs`,
      `src/ui/mod.rs`, `src/ui/list.rs`, `src/ui/driver.rs`, `src/ui/view.rs`,
      `src/ui/app.rs`). Every one names all five fields; **no `..` rest anywhere**, in a
      literal or in a pattern, on a single line or across several. Reach for a
      `..base.clone()` at none of them: that is precisely the shape rustfmt spreads over
      seven lines, and it is why `NODEFAULT-UI`'s half B was rewritten to brace-match.
      **Red when:** `cargo build --all-targets` reports an `E0063` that has not been fixed,
      **or** `SCAN_MIN=60 TYPES="Dashboard Filter Detail" sh $CHECKS/NODEFAULT-UI.sh` reports
      an elision. The gate here is the compiler and that check — deliberately **not** a grep
      count, which only ever grows and so can never go red.

- [x] 1.3 Extend the compile-time companion in `src/ui/app.rs`'s tests: the `Detail`
      destructuring test names **all five** fields with no `..`, and the `Dashboard` one
      continues to name all eight. Rename it — it is currently
      `detail_destructures_into_exactly_two_fields`, which becomes a lie the moment 1.1
      lands. Run `TYPES="Dashboard Filter Detail" SCAN_MIN=60 sh $CHECKS/NODEFAULT-UI.sh` and
      record its two OK lines.
      **Red when:** the companion still names two `Detail` fields, its name still says "two",
      or `NODEFAULT-UI` reports an elision. Note what the companion does **not** do: it
      destructures one value, so it catches a field added to the type and never an elision at
      another site. Half B is what covers that, which is why 1.2's Red-when names it.

- [x] 1.4 Add the three `Action` variants — `SelectTab(usize)`, `NextTab`, `PrevTab` — to
      `ui::app::Action`, and add a `_ => {}`-free arm for each in `Dashboard::apply` that
      does **nothing yet**, so the match stays exhaustive without `_`. Add no key binding in
      `action_for` yet: group 7 does that, RED-first.
      **Red when:** `apply`'s match gains a wildcard arm, which would let a later variant go
      unhandled silently.

- [x] 1.5 Add `pub type ArtifactReader<'a> = &'a dyn Fn(&std::path::Path) -> Result<String,
      String>;` to `src/ui/app.rs`, and add the `read: ArtifactReader<'_>` parameter to
      `ui::driver::run_loop`, positioned between `events` and `tick` as `design.md` →
      Contracts shows. The loop does **not** call it yet. Update **all ten** call sites — the
      one production caller in `src/ui/mod.rs`, `src/ui/mod.rs`'s acceptance test, and the
      eight in `src/ui/driver.rs`'s test module — each passing a trivial
      `|_: &std::path::Path| Ok(String::new())` closure for now.
      **Red when:** `cargo build --all-targets` still reports a call site with four
      arguments. The count is measured, not assumed: `grep -rn 'run_loop(' src/ | wc -l`.

- [x] 1.6 RED: Add `mod read_artifact` under `src/ui/mod.rs`'s `mod tests`, holding two
      tests against a real `crate::testutil::ScratchDir`: `ui::read_artifact` on a written
      file equals `std::fs::read_to_string` of the same path, and on a path inside that
      directory that does not exist returns an `Err` with non-empty text and no panic. This
      is the crate's third one-line binding to the real world, and it carries its own
      assertions on the same terms `config::env_lookup` does — written **here**, in the
      group that creates the binding, rather than nine groups later.
      **Red when:** the success case asserts a hard-coded string rather than agreement with
      `std::fs::read_to_string`. Agreement is the requirement; a literal would pass against a
      binding that silently normalised line endings.

- [x] 1.7 GREEN: Add `ui::read_artifact` to `src/ui/mod.rs` — the crate's one
      `std::fs::read_to_string` for artifacts, mapping the error to its `Display` text — and
      wire `ui::run` to pass `&read_artifact` to `run_loop`.
      **Red when:** `read_artifact` is named anywhere under `src/ui/` but `src/ui/mod.rs`;
      `run_loop` and `sync_detail` take a `&dyn Fn`, never this function by name, or the
      injection is decorative. `UI_MIN=9 sh $CHECKS/READSEAM.sh` is the mechanical form and
      is run in 1.9.

- [x] 1.8 Add `crate::testutil::RecordingReader` to `src/lib.rs`'s `#[cfg(test)] pub(crate)
      mod testutil`: constructed from a list of `(path, Result<String, String>)` entries plus
      a default result, it records every call's path in order and exposes `calls()` and
      `paths()`. It performs **no I/O**, so it can be used from any test module without a
      scratch directory. Because `&dyn Fn` is not a `&mut` receiver, it records into a
      `RefCell` and every call site wraps it — `let read = |p: &Path| rec.read(p);` then
      `&read`. This is the double every group from 8 onward drives `sync_detail` with, and it
      is what makes "was not read again" assertable at all.

- [x] 1.9 Add `changes::fixture::with_artifacts(change: Change, artifacts: &[(&str,
      &[&str])]) -> Change` **inside** `src/changes.rs`'s existing `#[cfg(test)] pub(crate)
      mod fixture`, returning the change with its `artifacts` replaced by `ArtifactRef`
      values built from the given ids and path strings, in the given order, with **no
      de-duplication**. Every fixture in groups 4 through 10 reaches an artifact-carrying
      `Change` through this function.
      **Red when:** the helper lives anywhere but `src/changes.rs`, which would put a
      `Change {` or `ArtifactRef {` literal outside the file `NOLIT-CHANGE` guards.

- [x] 1.10 VERIFY: `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features
      -- -D warnings`; `cargo test --all-features` — **all green**, since no behaviour
      changed beyond the new binding and its two tests; `. $CHECKS/TESTCOUNT.sh` then
      `testcount --lib 'ui::tests::read_artifact::' 2` and `testcount --lib 'ui::' 174`
      (the measured 172 plus this group's two).
      Then `UI_MIN=9 sh $CHECKS/READSEAM.sh` — its first run, now that the binding exists —
      and `sh $CHECKS/NOIO-VIEW.sh` is **not** yet runnable (`src/ui/detail.rs` does not
      exist until group 4).
      **Red when:** any test fails, either count falls short, or `READSEAM` reports
      `read_to_string` outside `src/ui/mod.rs`.

- [x] 1.11 Commit: `refactor(detail-view): add the Detail fields, the tab actions, and the
      injected reader`.

## 2. The outer-loop acceptance test — RED until group 10
<!-- kind: behavioural -->

- [x] 2.1 RED: Add a **second** test function to `src/ui/mod.rs`'s existing
      `mod tests` → `mod detail`, beside `markdown-viewer`'s
      `a_markdown_document_renders_and_scrolls_through_the_loop` (which group 10 updates,
      since this change invalidates its `scroll == 4` / `- line-04` expectations). It drives
      `run_loop` over a
      `TestBackend` at **both** 120x20 and 60x20 with a `RecordingReader` and a `Script` of
      key presses. The dashboard holds two active changes; the selected one carries the five
      tdd artifacts, whose second path reads as a twenty-item bullet list. The script is:
      `]`, then ten `j`, then `q`. Assert, on the **final** buffer and the **final** state:
      - the detail region's first interior row is the selected change's header, ending in its
        progress cell;
      - its second row begins `1 proposal  2 specs`;
      - its content area's first row reads `- line-06` and its last drawn row `- line-19`;
      - `dashboard.detail.tab` is `1` and `dashboard.detail.scroll` is `6` — **not** `10`,
        which is what makes this test stay red until `normalise_scroll` uses the content
        area's fourteen rows;
      - the reader recorded exactly **two** calls: the first artifact's path on the first
        iteration, the second artifact's path after the `]`, and nothing more across the ten
        `j` presses.
      **Red when:** it passes. It must fail here on the header assertion; the failure message
      is recorded verbatim so groups 3 to 9 can tell the known failure from a new one. The
      test loops over both widths inside one function, so the *first* width to fail is the
      one reported — record that message, not an imagined pair.

- [x] 2.2 CHECK: Run the four-command gate. `cargo test --all-features` must fail on
      **exactly one** test — the new one, in `ui::tests::detail::` — and `cargo llvm-cov
      --ignore-run-fail --fail-under-lines 80` must still produce a report.
      **Red when:** a second test fails, or `llvm-cov` produces no report, which would mean
      the crate's test target does not build. In particular
      `a_markdown_document_renders_and_scrolls_through_the_loop` must still **pass** here:
      nothing in group 2 changes `normalise_scroll`, so its `scroll == 4` is still correct
      until group 10.

- [x] 2.3 Commit: `test(detail-view): the failing outer-loop acceptance test`.

## 3. `layout::split_detail`
<!-- kind: behavioural -->

- [x] 3.1 RED: Add tests in `src/ui/layout.rs`'s `mod tests` for scenario 28 — `split_detail`
      at interiors of width **78** and **58**, at heights 0, 1, 2, 3, and 16 — asserting all
      four fields of all three returned rects, including that each carries the interior's own
      `x` and `width`.
      **Red when:** the tests pass before `split_detail` exists, which they cannot; the RED is
      a compile failure, and the task is complete only once that failure names
      `split_detail`.

- [x] 3.2 GREEN: Implement `split_detail` in `src/ui/layout.rs`, branching on heights 0, 1,
      and 2 explicitly rather than handing them to the constraint solver, exactly as
      `split_frame` does and for the same measured reason.

- [x] 3.3 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::layout::tests::' 15`. The
      floor is the **measured** 12 plus this group's 3.
      **Red when:** fewer than 15 tests run, which is what a renamed module or a typo'd
      filter looks like.

- [x] 3.4 Commit: `feat(detail-view): split the detail interior into header, tabs, and content`.

## 4. `ui::detail::header_row`
<!-- kind: behavioural -->

- [x] 4.1 RED: Create `src/ui/detail.rs` with a `mod tests` holding scenarios 36–40, every
      test naming both **78** and **58**: the full grammar at both widths; the `[-]` cell for
      a change with no tasks; a 200-character name truncated with `…` while both cells stay
      intact; the drop-whole-in-order walk across widths 78, 58, 30, 26, 25, 20, 19, 18, 5,
      2, 1, and 0; and an empty schema rendering `()` rather than vanishing. Register the
      module in `src/ui/mod.rs`. Assert the **exact string**, character for character, not a
      `contains`. The width list is the two mandated interiors plus **both** boundaries of
      **all three** degradation bands — `78`, `58`, `13`, `12`, `7`, `6`, `5`, `1`, `0` — and
      that list is not negotiable: with a five-character schema cell and a five-character
      progress cell the bands are `w >= 13`, `7 <= w <= 12`, and `1 <= w <= 6`, so a list
      that samples only widths above 13 makes the "the schema cell is dropped" clause
      **vacuously true** and passes against an implementation that never drops it.
      **Red when:** any test asserts only a length or a substring, or the width list omits
      12 or 7. A row grammar asserted by length passes against the wrong row.

- [x] 4.2 GREEN: Implement `header_row`, raising `ui::list::progress_cell` and
      `ui::list::pad_or_truncate_right` to `pub(crate)` and calling them rather than copying
      them. `src/ui/detail.rs` names no ratatui type.

- [x] 4.3 CHECK: Run `sh $CHECKS/NOIO-VIEW.sh` — its first run, now that
      `src/ui/detail.rs` exists — and `sh $CHECKS/NOTABSEAM.sh`. Record both OK lines.
      `NOTABSEAM` is a **whole-file** sweep, comments included: `src/ui/detail.rs`'s module
      doc comment must therefore say "the view" and must not name `Style`, `Modifier`,
      `Span`, `Rect`, `Frame`, or `Buffer`, exactly as `src/ui/markdown.rs`'s must.
      **Red when:** either fails, or `NOIO-VIEW` reports fewer than seven searched files.
      A `NOTABSEAM` failure whose only hit is a doc comment is fixed by rewording the
      comment, never by exempting comments from the check.

- [x] 4.4 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::detail::tests::' 5`;
      `DETAIL_MIN=5 sh $CHECKS/DETAILWIDTHS.sh`. The floor rises with each of groups 5 and 6
      and reaches 19.
      **Red when:** any test in the new file names only one of the two widths.

- [x] 4.5 Commit: `feat(detail-view): the change header's fixed-field grammar`.

## 5. `ui::detail::tab_bar`
<!-- kind: behavioural -->

- [x] 5.1 RED: Add scenarios 17–25 to `src/ui/detail.rs`'s `mod tests`, every test naming
      both **78** and **58**, every fixture built through `changes::fixture::with_artifacts`:
      the five tdd artifacts with their exact `x` offsets 0, 12, 21, 31, 40; duplicate ids
      staying two addressable tabs; a tenth artifact labelled without a digit; the empty list
      returning one `no artifacts` cell; width 0 returning an empty vector; the twelve-tab
      window at three selections and both widths, asserting contiguity, that exactly one cell
      is selected and its `index` matches, and that the 78-column window holds strictly more
      cells than the 58-column one; the window sliding back on a leftward move; a 200-character
      id truncated with `…` to exactly the width; and `selected: 7` over three artifacts not
      panicking.
      **Red when:** the twelve-tab test asserts only "does not panic". The contiguity, the
      always-selected-visible property, and the strictly-more-cells-at-78 comparison are what
      make it a test rather than a smoke check.

- [x] 5.2 GREEN: Implement `tab_bar` and the `Tab` type per `design.md` → Contracts: labels,
      two-space separation, the `start`/`end` window, the single-oversized-cell exception,
      and the zero-artifact placeholder.

- [x] 5.3 REFACTOR: If `header_row` and `tab_bar` grew a shared width-arithmetic helper,
      name it and keep it private. Do **not** move either public function's width parameter
      away; `DETAILWIDTHS` has no exemption list and depends on every public function taking
      one.

- [x] 5.4 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::detail::tests::' 14`;
      `DETAIL_MIN=14 sh $CHECKS/DETAILWIDTHS.sh`; `MIN=18 sh $CHECKS/NOLIT-CHANGE.sh` — the first run at the
      new floor, proving the fixtures went through `changes::fixture` rather than through a
      literal.
      **Red when:** `NOLIT-CHANGE` reports a `Change {` or `ChangeSet {` in
      `src/ui/detail.rs`.

- [x] 5.5 Commit: `feat(detail-view): the artifact tab bar and its window`.

## 6. `ui::detail::content_lines`
<!-- kind: behavioural -->

- [x] 6.1 RED: Add scenario 16 and its companions to `src/ui/detail.rs`'s `mod tests`, every
      test naming both **78** and **58**: an empty `Detail` returning exactly one
      `No content yet` line; problems only; a source only; both, with the problems first;
      and a 200-character paragraph producing strictly more lines at 58 than at 78. Assert on
      `Line::text()`, so the assertion is about characters rather than about a segment count.
      **Red when:** the both-present case asserts only a line count. The **order** —
      problems, then markdown — is the requirement.

- [x] 6.2 GREEN: Implement `content_lines`, calling `ui::markdown::lines` for the source and
      `ui::list::pad_or_truncate_right` for each `"! <problem>"` line.

- [x] 6.3 CHECK: `MIN=18 sh $CHECKS/MDSEAM.sh` — `src/ui/detail.rs` calls
      `ui::markdown::lines` and must name `pulldown_cmark` nowhere; and `NOTABSEAM`, since
      `markdown::Line` is plain data and must not have dragged a ratatui type in.
      **Red when:** either reports a hit, or `MDSEAM` reports fewer than 18 searched files.

- [x] 6.4 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::detail::tests::' 19`;
      `sh $CHECKS/DETAILWIDTHS.sh` at its default floor of 19 — the value it keeps for the
      rest of this change, and the sum of groups 4 (5 tests), 5 (9), and 6 (5).
      **Red when:** the count is below 19, or any detail test names only one width.

- [x] 6.5 Commit: `feat(detail-view): the detail content's line list`.

## 7. The tab keys
<!-- kind: behavioural -->

- [x] 7.1 RED: Add scenarios 29–35 (state half) and 50–53 (their extensions) to
      `src/ui/app.rs`'s `mod tests`: `1`–`9` mapping to `SelectTab(n-1)` and `0`, a
      modifier'd digit, and `!` mapping to `Ignore`; `]` and `[` mapping to `NextTab` and
      `PrevTab` while `}` and a modifier'd `]` do not; the same eight events under
      `filtering` true mapping to `FilterPush` of the same character; a `Release` and a
      `Repeat` of `1` and `]` mapping to `Ignore` under both modes; clamped stepping at both
      ends without wrapping; an out-of-range digit leaving both `tab` and `scroll` untouched;
      the scroll resetting exactly when `tab` changed; and `Next`/`Prev` at `Route::List`
      resetting `tab` and `scroll` exactly when `selected` changed.
      **Red when:** the "staying put does not reset" half is missing from either the
      `NextTab`-at-the-end test or the clamped-`Prev`-at-the-top test. Both halves are the
      requirement; a test asserting only the reset passes against an implementation that
      always resets.

- [x] 7.2 GREEN: Add the bindings to `action_for` and the three arms to `apply`, plus
      `Dashboard::selected_change()`. `apply` performs no clamp of `tab` against the artifact
      count beyond what `SelectTab`'s validity check and `NextTab`'s step need —
      `sync_detail` owns the invariant, per `design.md` → Decisions 4.

- [x] 7.3 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::app::tests::' 37` — the
      measured 30 plus this group's seven new test functions. The floor reaches 45 after
      group 8's eight.
      **Red when:** fewer than 37 tests run.

- [x] 7.4 Commit: `feat(detail-view): bind 1-9, [ and ] to the artifact tab`.

## 8. `Dashboard::sync_detail`
<!-- kind: behavioural -->

- [x] 8.1 RED: Add scenarios 4–11 to `src/ui/app.rs`'s `mod tests`, every one driven with a
      `RecordingReader` and **no filesystem**: read once and reuse across three syncs
      (asserting `calls() == 1`); a tab switch and a change switch each re-reading, with the
      recorded paths in order; two changes sharing the name `add-auth` distinguished by
      `dir`; two paths concatenated in order with exactly one separating newline, proven both
      for a first file that ends in a newline and for one that does not; an `Err` on the
      first path recording one problem containing that path and its reason while the second
      path's text still lands; an artifact with **no** paths recording zero calls; a tab out
      of range after a filter edit clamped before the read; and an empty visible list
      clearing all five fields.
      **Red when:** any of these asserts only the resulting `source`. The **call count** is
      the assertion that proves the `loaded` key works, and it is the one a naive
      implementation that re-reads every iteration fails.

- [x] 8.2 GREEN: Implement `sync_detail` in the five-step order `design.md` → Contracts
      states, cloning the `dir` and the selected artifact's `paths` before mutating.

- [x] 8.3 CHECK: `sh $CHECKS/NOIO-VIEW.sh` and `UI_MIN=9 sh $CHECKS/READSEAM.sh`.
      `src/ui/app.rs` now carries the whole read-decision and must still name no I/O API and
      no `read_artifact`.
      **Red when:** either reports a hit. A `use std::fs` that crept into `app.rs` is exactly
      what this pair exists to catch.

- [x] 8.4 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::app::tests::' 45`.
      **Red when:** fewer than 45 tests run.

- [x] 8.5 Commit: `feat(detail-view): resolve the selected tab's content through the injected reader`.

## 9. The detail region's render
<!-- kind: behavioural -->

- [x] 9.1 RED (a): Add **ten new** test functions to `src/ui/view.rs`'s `mod tests`, every
      one naming both **60** and **120** and asserting **actual cell content** rather than a
      shape — scenarios 12, 13, 14, 25, 26, 34 (view half), 60, 61, 62, and 63:
      the header at both widths with its exact 78- and 58-character strings; the selection
      move discriminating between two different headers; the archived change's stripped name
      and its own schema; the empty visible list leaving every interior cell a space whose
      `Style` equals `Cell::default().style()`, **paired in the same test** with the same
      dashboard plus one change, which is *not* blank; the five-tab bar's exact string with
      `3 design` bold and `1 proposal` not; a `SelectTab(2)` at `Route::List` visible in the
      tab row at 120; the border sweep at both widths with twelve 40-character tab ids and
      200-character lines; `No content yet` in the content area's first row with the tab bar
      still intact above it; a problem row reading `! /repo/…` above `# b` with
      `No content yet` absent; and the twenty-item list filling rows 4 through 17 with
      `- line-00` through `- line-13`.
      **Red when:** a blank-interior test has no discriminating companion assertion in the
      same test. A buffer assertion that renders nothing and asserts nothing-in-particular
      passes forever.

- [x] 9.2 RED (b): **Update**, do not add, the six existing tests this change invalidates.
      They are named here because a group that says "add scenarios" and then finds a red
      suite has discovered them rather than planned for them:
      1. `src/ui/view.rs` `the_detail_document_fills_the_interior_at_60_and_120` — asserts
         detail row 2 is `- line-00` and row 17 is `- line-15`; becomes row 4 and
         `- line-13`, because the content area is fourteen rows (scenario 64).
      2. `src/ui/view.rs` `the_detail_region_stays_blank_while_the_list_fills` — three real
         changes at `Route::List`, asserting every detail interior cell blank; at 120 the
         wide layout now draws that change's header and tab bar, so the assertion becomes
         "blank only where `visible()` is empty" (scenarios 66, 78).
      3–5. `src/ui/view.rs`'s three tests reached through the `detail_dashboard` helper,
         whose `changes` is `fixture::set(vec![], vec![], vec![])` — an empty visible list,
         which after 9.4 draws nothing at all. The helper gains a change so the tests keep
         exercising what they were written for (scenarios 65, 67, 73).
      6. `src/ui/app.rs`'s three `assert_eq!(…, 4)` scroll literals become `6`, for the same
         fourteen-row reason. These are edits to existing tests, so they add nothing to
         `ui::app::tests::`' count.
      Scenarios 64, 65, 66, 67, 68, 73, 78, and 79 are these six tests' subjects; none of
      them is a new test function, and 9.5's floor of 67 is what that fact is derived from.
      **Red when:** the suite is red for any test not on this list, which would mean a
      seventh was missed. Record each updated test's before and after expectation.

- [x] 9.3 GREEN: Rewrite `ui::view::render_detail` to draw, through `layout::split_detail`:
      nothing at all when `Dashboard::visible()` is empty; otherwise the bold header row, the
      tab row with the selected cell bold, and the `content_lines` slice
      `layout::scroll_offset` selects against the **content area's** height. Never write past
      the interior's last column.

- [x] 9.4 REFACTOR: `render_detail` is now three drawing steps; extract each into a private
      helper if it improves readability, keeping every one a pure function of a `Rect` and
      `&Dashboard`.

- [x] 9.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::view::tests::' 67`;
      `WIDTHS_MIN=67 sh $CHECKS/WIDTHS.sh`; `sh $CHECKS/NOIO-VIEW.sh`.
      The floor is the measured 57 plus 9.1's **ten new** test functions. 9.2's six updates
      add nothing to it, and a floor derived from "scenarios in this group" would be
      unreachable.
      **Red when:** fewer than 67 tests run, or any view test names only one of 60 and 120.

- [x] 9.6 Commit: `feat(detail-view): draw the header, the tab bar, and the content area`.

## 10. `normalise_scroll` over the content area — the acceptance test goes green
<!-- kind: behavioural -->

- [x] 10.1 RED: Scenarios 75, 76, and 11, plus the update to `markdown-viewer`'s acceptance
      test. In `src/ui/app.rs`'s test module, **update** the existing `normalise_scroll`
      tests: at 120x20 the offset for a twenty-line source becomes `6`, at 120x40 it is `0`,
      and at 60x20 it is `6`; the narrow list route still leaves a stored `9` alone while the
      wide list route normalises it to `6`. In `src/ui/driver.rs`'s test module, add **two
      new** tests for `run_loop` calling `sync_detail` **before** the draw — a recording
      reader whose single call precedes a single frame, and the failing-backend case where
      the reader recorded one call and the event source zero. In `src/ui/mod.rs`'s
      `mod tests` → `mod detail`, update
      `a_markdown_document_renders_and_scrolls_through_the_loop`: its `scroll == 4` becomes
      `6`, its `- line-04` becomes `- line-06`, and its dashboard gains a selected change (it
      currently uses `changes::empty_set()`, which after 9.3 draws nothing at all).
      **Red when:** the resize test asserts only that the value changed. The three exact
      values — 6, 0, 6 — are what prove the content area's height, not the interior's, was
      used.

- [x] 10.2 GREEN: Change `Dashboard::normalise_scroll` to derive the content area through
      `layout::split_detail` and to count `ui::detail::content_lines` rather than
      `markdown::lines`, and change `run_loop` to call `dashboard.sync_detail(read)` at the
      top of each iteration, before the draw.

- [x] 10.3 CHECK: The group 2 acceptance test is now **green**, and so is the updated
      `markdown-viewer` one beside it. Run the module alone and record its output:
      `cargo test --all-features --lib 'ui::tests::detail::'` — **two** tests, both passing.
      **Red when:** either still fails, or the new one passes for the wrong reason — re-read
      its five assertions and confirm the `scroll == 6` and `calls() == 2` ones are among
      those that pass.

- [x] 10.4 VERIFY: `make check` — the literal, unqualified composite, runnable again from
      here on. Then `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::tests::detail::' 2` —
      **two**, because `mod detail` already held one test on `main` and a floor of 1 would be
      satisfied before this change wrote a line; and `testcount --lib 'ui::app::tests::' 45`.
      **Red when:** `make check` fails; report the failing sub-command by name rather than a
      summary.

- [x] 10.5 Commit: `feat(detail-view): normalise the scroll against the content area`.

## 11. The read binding and the startup path
<!-- kind: behavioural -->

- [x] 11.1 RED: **Update** the six existing tests in `ui::tests::load` for scenarios 51–54
      and 70: `ui::load` leaves all five `Detail` fields empty; the `NotFound` arm does too;
      rendering the loaded dashboard at 120x20 now shows the change's header and tab bar with
      `No content yet` below it, while rendering the `NotFound` one still leaves the detail
      interior blank. `startup_leaves_the_detail_empty_and_unscrolled` currently asserts the
      120-column detail interior is blank for a dashboard that **has** a change, which stops
      being true at 9.3 — that is the one to rewrite most carefully.
      **Red when:** the count of tests in `ui::tests::load::` rises here. These are updates;
      the one new test function is 11.2's.

- [x] 11.2 RED: Add **one new** test function to `ui::tests::load`, for scenario 54's second
      half: a `snapshot` of a real `ScratchDir` repository taken before and after `ui::load`
      **followed by** `Dashboard::sync_detail` driven with the real `ui::read_artifact`
      binding, byte-identical.
      **Red when:** it drives `sync_detail` with the `RecordingReader` instead of the real
      binding. The point is that the **real read** writes nothing; a fake reader proves
      nothing about the filesystem.

- [x] 11.3 GREEN: Make whatever minimal change these require. `ui::load` should already
      satisfy them after group 1's sweep; if it does not, the sweep was incomplete.

- [x] 11.4 CHECK: `BASE=<sha> sh $CHECKS/OPENSPEC-UNTOUCHED.sh`, and
      `UI_MIN=9 sh $CHECKS/READSEAM.sh`.
      **Red when:** `OPENSPEC-UNTOUCHED` reports a stray path, or `READSEAM` reports
      `read_to_string` outside `src/ui/mod.rs` — group 1's `mod read_artifact` tests name
      `std::fs::read_to_string` and live in `src/ui/mod.rs`, which is why it is allowed
      there and nowhere else.

- [x] 11.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::tests::load::' 7`;
      `testcount --lib 'ui::tests::read_artifact::' 2`; `testcount --lib
      'ui::driver::tests::' 12`; `make check`.
      **Red when:** any count falls short, or `make check` fails.

- [x] 11.6 Commit: `test(detail-view): the startup path and the written-nothing proof`.

## 12. The architectural check suite and its negative controls
<!-- kind: operational -->

- [ ] 12.1 CHECK: Run the whole suite at this change's floors, in one shell, recording every
      OK line verbatim: `MIN=18 sh $CHECKS/NOSPAWN-GREP.sh`; `sh $CHECKS/NOIO-VIEW.sh`;
      `UI_MIN=9 sh $CHECKS/READSEAM.sh`; `sh $CHECKS/NOTABSEAM.sh`; `UI_MIN=10 sh
      $CHECKS/NOCLI-SHELL.sh`; `TYPES="Dashboard Filter Detail" SCAN_MIN=60 sh
      $CHECKS/NODEFAULT-UI.sh`;
      `sh $CHECKS/NORAW-GREP.sh`; `MIN=18 sh $CHECKS/NOLIT-CHANGE.sh`; `MIN=18 sh
      $CHECKS/MDSEAM.sh`; `WIDTHS_MIN=67 sh $CHECKS/WIDTHS.sh`; `LIST_MIN=17 sh
      $CHECKS/LISTWIDTHS.sh`; `MD_MIN=23 sh $CHECKS/MDWIDTHS.sh`; `DETAIL_MIN=19 sh
      $CHECKS/DETAILWIDTHS.sh`; `sh $CHECKS/NOWAIVER.sh`; `python3 $CHECKS/GATE-MECH1.py`;
      `sh $CHECKS/NOJSON-SEAM.sh`; `BASE=<sha> sh $CHECKS/OPENSPEC-UNTOUCHED.sh`.
      **Red when:** any one fails, or any reports a searched-file count below its floor.

- [ ] 12.2 CHECK: Run `DEPS` (with `WORK` set, `DEPS_SKIP_LEG5=1`) and `GRAPH-SNAP`,
      **unedited**, and diff their output against what task 0.4 and task 0.3 recorded on
      `main`. Both must be identical line for line.
      **Red when:** either differs. This change adds no dependency; a changed dependency
      graph here means one crept in.

- [ ] 12.3 CHECK: Planted-violation controls. For each, copy the tree into `$WORK`, plant the
      violation there, run the check against the copy, confirm it **fails** with the expected
      message, and delete the copy. Never plant in the working tree.
      1. `std::fs::read_to_string` inside `src/ui/detail.rs` → `NOIO-VIEW` fails.
      2. `std::fs::read_to_string` inside `src/ui/driver.rs` → `NOIO-VIEW` and `READSEAM`
         both fail.
      3. `src/ui/mod.rs` with its `read_to_string` removed → `READSEAM` fails as **vacuous**,
         not as clean.
      4. `read_artifact` named in `src/ui/driver.rs` → `READSEAM`'s second leg fails.
      5. `src/ui/detail.rs` deleted → `NOIO-VIEW`, `NOTABSEAM`, and `DETAILWIDTHS` each fail
         on the missing file, by name.
      6. `use ratatui::style::Style;` in `src/ui/detail.rs` → `NOTABSEAM` fails.
      7. `pub fn tab_bar` renamed in `src/ui/detail.rs` → `NOTABSEAM`'s positive control
         fails, rather than the sweep passing against a file that no longer holds the grammar.
      8. A `#[test]` in `src/ui/detail.rs` naming `58` but not `78` → `DETAILWIDTHS` names
         that test.
      9. A `#[test]` in `src/ui/detail.rs` naming `78u16` and `58u16` only → `DETAILWIDTHS`
         names it, proving the suffixed-literal limit fails **closed**. Confirmed empirically
         at planning time: `re.findall(r'\b(\d+)\b', '78u16')` returns `[]`.
      10. `impl Default for Detail { … }` → `NODEFAULT-UI` half A fails.
      11. `impl Default for crate::ui::app::Detail { … }` — the **path-qualified** form →
          half A fails too. The pre-edit `for[[:space:]]+$T` pattern did not match this;
          confirm the edited one does, and record both results.
      12. `let Detail { source, .. } = d;` on one line → half B fails.
      13. **The blind spot this change closes.** Plant a rustfmt-shaped multi-line
          struct-update literal in a test helper:
          `Detail { source: s, scroll: 0, ..base.clone() }` written across seven lines, the
          `..base.clone()` on a line of its own. Confirm three things and record all three:
          (a) `cargo fmt --check` accepts it, so it is not merely a formatting violation;
          (b) the **old** same-line half B — `$T[[:space:]]*\{[^}]*\.\.` — reports
          **nothing**, which is the fifteen-elision failure this repository already had;
          (c) the **new** brace-matching half B reports it with its file and line. Also
          confirm the compile-time companion in `src/ui/app.rs` stays **green** against it,
          which is why the claim that the companion covers multi-line elisions was retired.
      14. `for i in 0..10` inside a function that also builds a `Detail` literal → half B
          reports **nothing**, so a range expression is not a false positive.
      15. Half B run with a deliberately broken type name (`TYPES=Nonexistent`) → the
          positive control fires before the scan, and with `TYPES="Dashboard Filter Detail"`
          against a copy holding no `.rs` files at all → the `SCAN_MIN` guard reports the scan
          as vacuous rather than printing OK.
      16. A `Change { … }` literal in `src/ui/detail.rs` → `NOLIT-CHANGE` fails.
      17. `use pulldown_cmark::Parser;` in `src/ui/detail.rs` → `MDSEAM` fails.
      18. `Command::new("ls")` in `src/ui/detail.rs` → `NOSPAWN-GREP` fails.
      19. `OpenspecCli` named in `src/ui/detail.rs` → `NOCLI-SHELL` fails.
      20. `--fail-under-lines 70` in a copied `Makefile` → `NOWAIVER` fails.
      21. A file written under `openspec/` in the copy → `OPENSPEC-UNTOUCHED` fails, and it
          fails for an **untracked** file too, which is the half `git diff` alone misses.
          Confirm the same tree passes `git diff --exit-code`, so the two are distinguished.
      **Red when:** any control passes. A check that cannot fail is not a check, and this
      practice caught 3 CRITICALs in `tui-shell`, 4 in `list-view`, and 5 in
      `markdown-viewer`. Control 13 is the one this change exists to add: it is the only one
      whose *expected* outcome includes a check reporting nothing.

- [ ] 12.4 CHECK: Confirm `$WORK` holds no leftover copy and
      `git status --porcelain` is clean but for this change's own files.
      **Red when:** a planted violation survived into the working tree.

- [ ] 12.5 Commit: `test(detail-view): run the architectural check suite and its negative controls`.

## 13. Documentation
<!-- kind: operational -->

- [ ] 13.1 Apply the **ten** `SPEC.md` corrections `design.md` → Decisions lists, each
      recorded in `planning-review.md` with its before and after text: (1) the detail
      region's three-way row split; (2) the above-nine and zero-artifact tab behaviour;
      (3) the no-change-selected blank state; (4) the Keys table's `1`–`9` / `[` / `]` row
      gaining route, `0`, and filter-mode wording; (5) the Responsive-layout paragraph
      gaining the fourteen-row content area; (6) a new Degraded-states row for an artifact
      file that exists and cannot be read, plus the `no artifacts` rendering consequence on
      the two existing schema rows; (7) a new Degraded-states row for a detail region with no
      change selected; (8) the Architecture → render seam paragraph naming `ui::read_artifact`
      as the crate's third one-line binding, and the Module map's `ui` row; (9) the
      Unit-tested-modules list naming `ui::detail` and correcting `ui::load` to `ui::mod`'s
      `load` and `read_artifact`; and (10) the View-tests paragraph naming `ui::detail`
      alongside `ui::markdown` and `ui::view` as a module asserting the 58/78 pair.
      **Red when:** a correction is described but not applied. `git diff SPEC.md` must show
      all ten, and `planning-review.md` must carry a before/after pair for each.

- [ ] 13.2 Update `AGENTS.md` → Current repo state (the detail region now carries a header, a
      tab bar, and resolved content) and → Architecture rules (the pure-view file set is
      seven; `src/ui/mod.rs` is the crate's only artifact-read binding; `src/ui/detail.rs`'s
      two mandated interior widths are 58 and 78).

- [ ] 13.3 CHECK: Re-read `SPEC.md` → Detail view, Keys, Responsive layout, and Degraded
      states against the implementation, line by line, and confirm each sentence is now true
      of the code. A contract gate, in the shape `openspec/config.yaml` requires of any group
      touching a documented format.
      **Red when:** any sentence is still false. Fix `SPEC.md`, not the checkbox.

- [ ] 13.4 DEFERRED to archive time — do **not** do it here. `openspec/IMPLEMENTATION-ORDER.md`'s
      `detail-view` row and the note about `tasks-tab` inheriting the tab bar can only be
      edited at archive time, because `OPENSPEC-UNTOUCHED` forbids writing anywhere under
      `openspec/` outside this change's own directory. Recorded here the way `list-view`
      (10.4b) and `markdown-viewer` (11.8) recorded theirs. Likewise `README.md`'s keymap,
      still deferred to the end of Phase 4 per commit `cdcc160`, now also owing `1`–`9`,
      `[`, and `]`.

- [ ] 13.5 VERIFY: `make check`; `BASE=<sha> sh $CHECKS/OPENSPEC-UNTOUCHED.sh`.
      **Red when:** either fails.

- [ ] 13.6 Commit: `docs(detail-view): correct SPEC.md's detail view, keys, and degraded states`.

## 14. Change Review
<!-- kind: review -->

- [ ] 14.1 Dispatch the `outside-in-tdd-reviewer` subagent over the finished change, with
      `proposal.md`, `design.md`, the six spec files, and this file as the contract. It
      reports findings only and edits no code.

- [ ] 14.2 Address every CRITICAL and MAJOR finding, each with its own commit. Record each
      finding and its resolution in `planning-review.md` → Change Review.
      **Red when:** a finding is closed with an argument rather than a change or a recorded,
      justified rejection.

- [ ] 14.3 Commit: `review(detail-view): address Change Review findings`.

## 15. Final validation
<!-- kind: operational -->

- [ ] 15.1 VERIFY: `make check` — the single gate. If it fails, name the failing
      sub-command (`fmt-check`, `lint`, `test`, or `coverage`) rather than reporting a
      summary.

- [ ] 15.2 VERIFY: `cargo llvm-cov --summary-only` and record the TOTAL **line** figure —
      the column further right, not the leading region count. It must be at or above 80%,
      and a drop of more than two points from the measured 98.04% baseline is investigated
      before this change is called done.

- [ ] 15.3 VERIFY: `. $CHECKS/TESTCOUNT.sh`; then, with the argument order the helper
      actually takes — `testcount <scope> <filter> <minimum>` —
      `testcount --lib 'ui::' 225`, `testcount --lib '' 618`, and
      `testcount --test-cli '' 5`. The empty filter is **load-bearing**: `testcount
      --test-cli 5` would bind `5` to the *filter* argument and leave the minimum empty,
      which passes with zero tests under zsh and errors under bash — a gate that is inert on
      one shell and red on another. The three numbers are the measured 172, 565, and 5 plus
      this change's 53 new library tests and no new integration test.
      **Red when:** any count falls short. A filter that matches nothing exits 0.

- [ ] 15.4 VERIFY: `export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"` then
      `openspec validate detail-view --strict`.

- [ ] 15.5 Commit: `docs(detail-view): record group 15 lint and verify evidence — apply complete`.
