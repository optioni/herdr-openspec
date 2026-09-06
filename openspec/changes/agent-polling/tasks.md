# agent-polling — implementation tasks

## How to read this file

Every floor here was **measured** before it was set, and the command that produced each
number is written beside it. A realized count below a group's target means the missing test
is written, not the floor lowered.

Every check is extracted to `$CHECKS/<LABEL>.sh` in task 0.1 and run from the extracted file
thereafter, never from a retyped copy. Blocks **reproduced in full below** are the two this
change adds; blocks **edited** are changed by the exact string replacements in task 0.4,
which prints the resulting `diff` before running them; everything else is extracted
byte-identically from a named archived file and diffed in task 0.2.

`testcount` is sourced, not run: `. $CHECKS/TESTCOUNT.sh`, then
`testcount --lib '<filter>' <minimum>`. It takes the scope, the filter, **and** the minimum,
in that order, and is judged on a counted minimum because a `cargo test` filter matching
nothing exits 0.

**`make check` is not runnable unqualified between groups 2 and 10** — see design.md →
Decisions 13. Group 2's acceptance tests are deliberately RED for that whole span, and
`cargo llvm-cov` hard-fails on any failing test, so the four gates are run individually there:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features            # must fail on EXACTLY the three known acceptance tests,
                                     # with the identical assertion message each time
cargo llvm-cov --ignore-run-fail --fail-under-lines 80
```

A **fourth** failing test at any of those boundaries is a real regression, not a known one.
The literal, unqualified `make check` is run from task 10.7 onward.

## Measured at planning time

Every figure below came from the command shown, run on `main` at
`aec6cf54b0dc2bd44cd22d6bf8ad520f47e52847`. **That sha is recorded for comparison only.**
Task 0.1 re-derives `BASE=$(git rev-parse HEAD)` fresh — this repository is shared with other
live sessions and its history was rewritten once, so a hardcoded sha is a gate that silently
includes this change's own commits in its baseline.

| Figure | Command | Value |
|---|---|---|
| Library tests | `cargo test --all-features --lib -- --list \| grep -c ': test$'` | **752** |
| Line coverage | `cargo llvm-cov --summary-only \| tail -1` | **97.16%**, 18,898 lines, 537 uncovered |
| Region count (**not** a line count) | same TOTAL row | 31,485 |
| `src/ui/*.rs` | `find src/ui -name '*.rs' \| wc -l` | **11** |
| `NOSPAWN-GREP` set | `find src -name '*.rs' ! -path 'src/cli.rs' \| wc -l` | **21** → 22 |
| `NOLIT-CHANGE` set | `find src -name '*.rs' ! -path 'src/changes.rs' \| wc -l` | **21** → 22 |
| `WATCHSEAM` set | `find src tests -name '*.rs' ! -path 'src/watch.rs' \| wc -l` | **23** → 24 |
| `AGENTSEAM` set | `find src tests -name '*.rs' ! -path src/cli.rs ! -path src/agents.rs ! -path src/ui/mod.rs \| wc -l` | **22** on `main` and **22** after (`src/agents.rs` is both added and excluded) |
| `NOSLEEP` set | `find src tests -name '*.rs' \| wc -l` | **24** → 25 |
| `NODEFAULT-UI` half-B spans | `SCAN_MIN=90 TYPES='Dashboard Filter Detail Refresh' sh $CHECKS/NODEFAULT-UI.sh` | **165** scanned; floor stays **90** |
| Crate-wide `#[allow]` count | `grep -rn '#\[allow' src/ tests/ \| wc -l` | **1**, `clippy::too_many_arguments` on `changes::build_change` |
| clippy's `too_many_arguments` threshold | a 7-param and an 8-param function compiled under `-D warnings` | fires at **8**, not 7 — see design.md → Boundaries |
| `herdr agent list` cost | ten timed runs through `subprocess.run` | **8.0 ms** median (7.3 min, 9.4 max) |

## Test-count floors — measured plus this change's enumerated new tests

| Group | Filter | Measured | New | Target |
|---|---|---|---|---|
| 3 | `agents::tests::parse::` | 0 | 13 | 13 |
| 4 | `agents::tests::poll::` | 0 | 6 | 6 |
| 5 | `cli::tests::` | 37 | 3 | 40 |
| 6 | `watch::tests::` | 27 | 4 | 31 |
| 7 | `agents::tests::seam::` | 0 | 3 | 3 |
| 7 | `agents::tests::worker::` | 0 | 7 | 7 |
| 8 | `ui::app::tests::` | 55 | 4 | 59 |
| 8 | `ui::view::tests::` | 81 | 1 | 82 |
| 9 | `ui::driver::tests::` | 22 | 7 | 29 |
| 1, 10 | `testutil::tests::` | 8 | 2 | 10 |
| 2, 10 | `ui::tests::wiring::` | 0 | 3 | 3 |

Library total: `752 + 13 + 6 + 3 + 4 + 3 + 7 + 4 + 1 + 7 + 2 + 3` = **805**, asserted in
group 14 and nowhere else. Thirteen scenarios *modify* landed `ui::driver::tests::` tests
(each only to build a three-field `Live`) and add nothing to any count.

**`ui::tests::wiring::` is a new submodule** of `src/ui/mod.rs`'s `mod tests`, beside the
existing `read_artifact`, `detail`, `load`, `start`, and `live`. Its floor is asserted **only
at group 10**, never between groups 2 and 10: `testcount` counts tests that *passed*, and all
three of its tests are deliberately red for that span.

## Group ordering — examined, and sequential

Groups 3, 4, and 7 all write `src/agents.rs`; groups 8, 9, and 10 all write files under
`src/ui/`. Groups 5 (`src/cli.rs`) and 6 (`src/watch.rs`) share no file and depend on each
other in neither direction — 5's `agent_cli_via` is first consumed at 10.1 and 6's `soonest`
at 9.2 — so the first two criteria hold for that pair. **The third does not, and that is why
no group is marked parallel:** `cargo test` builds the whole crate, so a half-written
`src/watch.rs` fails group 5's `testcount` and a half-written `src/cli.rs` fails group 6's,
and neither failure is attributable to the group that caused it. The blocker is shared
compilation, not shared files.

---

## Command-level checks added by this change

Both were written and run against a real tree at planning time — green on a scratch copy of
`src/` carrying the stub module and the split `run`, and red against every planted violation
listed with them.

```sh
# AGENTSEAM — NEW in agent-polling. The agent poller is confined to ONE module, on exactly
# WATCHSEAM's, MDSEAM's, and NOSPAWN-GREP's single-file terms: the Herdr program is reachable
# by editing src/agents.rs and src/cli.rs, and no other file in the crate — tests/ included —
# may reach for it.
#
# Leg 1 is the file-scoped restatement of NOSPAWN-GREP, because src/agents.rs is the new file
# where reaching for a `herdr` subprocess directly would be most plausible.
# Leg 2 is WATCHSEAM leg 2's rule verbatim: the confined module is PLAIN DATA, so a polled
# agent never arrives at the view already styled. COMMENT-INCLUSIVE on purpose — a doc comment
# naming a view type is itself the coupling this forbids, so say "the view", never `Frame`.
# Leg 3 confines the Herdr handle itself. The pattern is HerdrCli|RealHerdrCli|agent_cli_via,
# not HerdrCli alone, and that is measured rather than cautious: agent_cli_via returns
# Arc<dyn HerdrCli> and inference hides the type, so a bare HerdrCli pattern reported OK on a
# tree carrying crate::cli::agent_cli_via(Path::new("/bin/herdr")) planted in src/ui/list.rs.
# src/ui/mod.rs is in ALLOWED because the composition root legitimately calls agent_cli_via;
# it is still forbidden to name HerdrCli itself by NOCLI-SHELL, which sweeps all of src/ui/,
# so the two checks compose without either one being weakened. agent-launch adds
# src/launch.rs to ALLOWED as a deliberate edit; a silent extra namer is what this catches.
set -u
AGENTS="${AGENTS:-src/agents.rs}"
ALLOWED="${ALLOWED-src/cli.rs src/agents.rs src/ui/mod.rs}"
MIN="${MIN:-20}"
fail() { echo "AGENTSEAM FAIL: $1" >&2; exit 1; }

[ -d src ] || fail "no src directory"
[ -f "$AGENTS" ] || fail "$AGENTS missing - the subject is gone"
case "$ALLOWED" in *[![:space:]]*) ;;
  *) fail "ALLOWED is empty - leg 3 would forbid the declaration itself";; esac

# Guard A — positive control, checked BEFORE every leg: the module must actually name the
# Herdr seam trait, or this is not the poller and a clean result means nothing.
grep -q 'HerdrCli' "$AGENTS" || fail "$AGENTS names no HerdrCli - the subject is not the poller"

# Guard B — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/agents.rs is still searched.
pruned=""
for f in $ALLOWED; do
  [ -f "$f" ] || fail "ALLOWED names $f, which does not exist - the exclusion is vacuous"
  pruned="$pruned ! -path $f"
done
# shellcheck disable=SC2086
n=$(find src tests -name '*.rs' $pruned | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

# Leg 1 — the module spawns no process.
p=$(grep -nE 'process::Command|Command::new|Stdio' "$AGENTS" || true)
[ -z "$p" ] || { echo "AGENTSEAM FAIL (leg 1): $AGENTS spawns a process:" >&2
                 echo "$p" >&2; exit 1; }

# Leg 2 — the module names no view type, comments included.
r=$(grep -nE 'ratatui|Modifier|Style|Span|Rect|Frame|Buffer' "$AGENTS" || true)
[ -z "$r" ] || { echo "AGENTSEAM FAIL (leg 2): $AGENTS names a ratatui type:" >&2
                 echo "$r" >&2; exit 1; }

# Leg 3 — the Herdr handle is reached only from the allowed files.
HANDLE_RE='HerdrCli|RealHerdrCli|agent_cli_via'
# shellcheck disable=SC2086
hits=$(find src tests -name '*.rs' $pruned -print0 \
       | xargs -0 -I{} grep -nE "$HANDLE_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "AGENTSEAM FAIL (leg 3): the Herdr handle is reached outside:$ALLOWED" >&2
                    echo "$hits" >&2; exit 1; }

# Guard C — the exclusion is not vacuous in the other direction either: src/cli.rs must
# declare the trait, or leg 3 is excluding a file that says nothing.
# ANCHORED on both sides. An unanchored `trait HerdrCli` is satisfied by
# `trait HerdrClient` after a rename, so the control would pass while leg 3 searched for a
# name that is no longer declared anywhere. Measured: the unanchored form stayed green
# against exactly that rename.
grep -qE 'trait[[:space:]]+HerdrCli[[:space:]]*:' src/cli.rs \
  || fail "positive control - src/cli.rs declares no 'trait HerdrCli:'"

echo "AGENTSEAM OK: $n files searched (>= $MIN); no spawn and no view type in $AGENTS; Herdr handle only in:$ALLOWED"
```

```sh
# WIRED — NEW in agent-polling. The composition root actually starts what it composes, and
# the residue left in `run` is small enough that reading it is a proof.
#
# WHY THIS EXISTS: live-refresh shipped a `ui::run` that called neither watch::start nor
# refresh::start. Every test passed, because unit and view tests drove the seams directly and
# nothing exercised the function that composes them. This check is the source-level half of
# the answer; the behavioural half is ui::tests::wiring's acceptance test, and NEITHER is
# sufficient alone — a grep proves a name appears, not that its result reaches `Live`, and a
# test proves the wiring at the moment it ran, not that a later edit kept it.
#
# Leg 2 is the one that keeps this honest over time. It requires `pub fn run`'s own body to
# hold no branch and no loop, so the untested residue cannot grow a condition under which a
# collaborator is silently dropped. Everything with a decision in it lives in run_wired, which
# the acceptance test drives.
set -u
MOD="${MOD:-src/ui/mod.rs}"
CLI="${CLI:-src/cli.rs}"
UIDIR="${UIDIR:-src/ui}"
fail() { echo "WIRED FAIL: $1" >&2; exit 1; }

[ -f "$MOD" ] || fail "$MOD missing - the subject is gone"
[ -f "$CLI" ] || fail "$CLI missing - the positive controls have nothing to match"
[ -d "$UIDIR" ] || fail "$UIDIR missing - leg 4 has nothing to search"

# Guard D — $MOD holds EXACTLY ONE line-anchored #[cfg(test)], since prod() truncates at the
# first and every leg below it is silent. Measured: with the names present only inside
# `mod tests` and a single TRAILING SPACE after the attribute (so the anchored count is 0 and
# prod() keeps the whole file), an entirely unwired start_collaborators reported WIRED OK.
# NOBLOCK carries this guard for its seam modules; WIRED needs it for the same reason.
c=$(grep -c '^#\[cfg(test)\]$' "$MOD" || true)
[ "$c" -eq 1 ] || fail "$MOD holds $c line-anchored #[cfg(test)] attributes, expected exactly 1 - every leg below the first one is silent"

prod() { awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print}' "$1"; }
# CODE, not text. Every leg below searches the production slice with comments STRIPPED, and
# that is a measured requirement rather than tidiness. src/ui/mod.rs's doc comment on `run`
# names `watch::start` and `crate::refresh::start` in prose; measured, a plant that replaced
# the real `refresh::start(...)` call with `refresh::none()` left that comment in place and
# leg 1 stayed GREEN on a genuinely unwired composition root - the exact defect this check
# exists to catch. Stripping `//` to end-of-line can also truncate a `//` inside a string
# literal, which only makes a must-be-present leg stricter and a must-be-absent leg no
# weaker, so it fails in the safe direction.
code() { prod "$1" | sed 's://.*::'; }

# Guard A — positive controls live in the OTHER file, so a renamed binding fails HERE rather
# than leaving leg 1 searching for a name that is no longer defined anywhere. Each is anchored
# on a definition form, never a bare substring: an unanchored `agent_cli_via` is satisfied by
# a doc comment mentioning it.
grep -qE '^pub fn agent_cli_via\(' "$CLI" \
  || fail "positive control - $CLI defines no 'pub fn agent_cli_via('"
grep -qE '^pub fn worker_cli_from_env\(' "$CLI" \
  || fail "positive control - $CLI defines no 'pub fn worker_cli_from_env('"
grep -qE '^pub const HERDR_PROGRAM' "$CLI" \
  || fail "positive control - $CLI defines no 'pub const HERDR_PROGRAM'"

# Leg 1 — every collaborator the loop needs is started BY NAME in the production slice.
for n in run_wired start_collaborators 'watch::start' 'refresh::start' 'agents::start' \
         worker_cli_from_env agent_cli_via; do
  code "$MOD" | grep -q -- "$n" \
    || fail "leg 1: $MOD's production slice does not name $n - the name is gone; leg 1 sees names, not values, so a call whose result is dropped still passes here and is the acceptance test's job"
done

# Leg 2 — `pub fn run`'s own body holds no branch and no loop. The body is cut from the
# production slice between its signature line and the next column-zero `}`.
# KNOWN LIMIT, stated rather than left to be discovered: the pattern is keyword-based, so a
# keyword-free decision (`x.unwrap_or_else(|| ...)`, `?` on an Option, a match hidden in a
# closure) passes. Measured. Leg 2's claim is only what it checked; the residue's real
# guarantee is its length plus the acceptance test below it.
body=$(code "$MOD" | awk '/^pub fn run\(\)/{f=1} f{print} f && /^\}$/{exit}')
[ -n "$body" ] || fail "leg 2: could not find 'pub fn run()' in $MOD's production slice"
printf '%s\n' "$body" | grep -q '^}$' \
  || fail "leg 2: 'pub fn run()' has no closing brace at column zero - the cut ran to EOF"
b=$(printf '%s\n' "$body" | grep -nE '(^|[^A-Za-z0-9_])(if|match|for|while|loop|else)([^A-Za-z0-9_]|$)' || true)
[ -z "$b" ] || { echo "WIRED FAIL (leg 2): 'pub fn run()' holds a branch or a loop:" >&2
                 echo "$b" >&2
                 echo "everything with a decision in it belongs in run_wired, which is tested" >&2
                 exit 1; }

# Leg 3 — the residue delegates: run calls run_wired, rather than naming it in a comment.
printf '%s\n' "$body" | grep -qE 'run_wired\(' \
  || fail "leg 3: 'pub fn run()' does not call run_wired("

# Leg 4 — the composition root does not hardcode the program name. The "herdr" literal
# appears nowhere under src/ui/, so `run` must reach cli::HERDR_PROGRAM rather than spelling
# the program itself and bypassing the one place it is written down.
#
# SCOPED TO src/ui/ deliberately, and that is a measured scope rather than a cautious one: a
# tree-wide form is RED on unmodified `main`, because src/config.rs and src/state.rs both
# build `.join("herdr")` for the plugin's configuration and state directories, which is
# correct, unrelated code. A tree-wide leg would have had to exempt two files on its first
# run, which is how a check rots into a rubber stamp.
h=$(find "$UIDIR" -name '*.rs' -print0 \
    | xargs -0 -I{} sh -c 'awk "BEGIN{p=1} /^#\\[cfg\\(test\\)\\]\$/{p=0} p{print FILENAME\":\"FNR\": \"\$0}" "$1"' _ {} \
    | grep -E '"herdr"' || true)
[ -z "$h" ] || { echo "WIRED FAIL (leg 4): the \"herdr\" literal appears under $UIDIR:" >&2
                 echo "$h" >&2
                 echo "reach cli::HERDR_PROGRAM instead - it is the one place the name lives" >&2
                 exit 1; }

lines=$(printf '%s\n' "$body" | wc -l | tr -d ' ')
echo "WIRED OK: seven names present in $MOD; 'pub fn run()' is $lines lines with no branch and no loop; no \"herdr\" literal under $UIDIR"
```

## Checks edited by this change — the exact replacements

Applied in task 0.4 by **exact string replacement**, each printing its `diff` before any run.

**`NOBLOCK.sh`** (extracted from `openspec/changes/archive/2026-09-06-live-refresh/tasks.md`),
four replacements:

1. after
   `[ -f src/watch.rs ] || fail "src/watch.rs missing - the positive control has nothing to match"`
   insert
   `[ -f src/agents.rs ] || fail "src/agents.rs missing - leg 3 has a third seam module to sweep"`
2. replace `for f in src/watch.rs src/refresh.rs; do`
   with `for f in src/watch.rs src/refresh.rs src/agents.rs; do` (Guard D over three files)
3. after Guard A's `src/refresh.rs`'s `thread::spawn` control, insert the same pair for
   `src/agents.rs` — `prod src/agents.rs | grep -qE 'mpsc'` and
   `prod src/agents.rs | grep -qE 'thread::spawn'`, each failing with a
   `positive control - src/agents.rs …` message
4. before leg 3's final `echo`, insert `src/agents.rs`'s own block: Guard E taking
   `prod src/agents.rs | grep -n 'fn drain' | tail -1` as `dr_line` and the first
   non-comment `thread::spawn` as `sp_line`, requiring `dr_line < sp_line`; the `awk` cut at
   that spawn; `grep -nE "$BLOCK3_RE"` over the head half; and the
   `prod src/agents.rs | grep -q 'try_recv'` control. Then replace leg 3's `echo` text with
   `"NOBLOCK OK (leg 3): src/watch.rs never blocks; src/refresh.rs and src/agents.rs block only after thread::spawn"`

`tail -1`, not `head -1`: `src/agents.rs` declares `fn drain` three times — the trait's
abstract signature, `NoAgentPoll`'s impl, and `RealAgentPoll`'s impl — and the first
necessarily precedes every spawn. Measured under `head -1`, moving `RealAgentPoll`'s `drain`
below `start` reported OK; under `tail -1` it reported FAIL.

**`NODEFAULT-UI.sh`** (extracted from
`openspec/changes/archive/2026-09-05-detail-view/tasks.md`), three replacements:

1. after `SRC="${SRC:-src}"` insert `HOMEFILE="${HOMEFILE:-$SRC/ui/app.rs}"`, and replace the
   **two** later uses of `$SRC/ui/app.rs` — the existence guard and the positive control's two
   occurrences on one line — with `$HOMEFILE`, so three textual occurrences change in total.
   **Not** named `HOME`: overriding the user's home directory breaks every tool the check
   shells out to.
2. replace `SRC="$SRC" TYPES="$TYPES" SCAN_MIN="$SCAN_MIN" python3 - <<'PY'`
   with the same line plus ` || exit 1`.
3. replace the final `echo` with the same text plus ` in $HOMEFILE`.

Replacement 2 is a **repair of a landed defect** — see design.md → Decisions 12 for what it
changes about every future run of this gate.

**`NOSLEEP.sh`** (extracted from `.../2026-09-06-live-refresh/tasks.md`), one replacement:
leg 2b's `for f in src/watch.rs src/refresh.rs; do` becomes
`for f in src/watch.rs src/refresh.rs src/agents.rs; do`, and its `echo` names the third file.
`SLEEP_MIN` stays **4**: this change's waits are `recv_timeout` against a deadline inside
`src/agents.rs`'s test module and `std::thread::yield_now` inside `testutil::UntilReady`,
neither of which is a sleep.

**`OPENSPEC-UNTOUCHED.sh`** (extracted from `.../2026-09-06-live-refresh/tasks.md`), two
replacements — and this one is **not optional bookkeeping**: run as extracted it is RED from
task 0.3 onward, because its single exclusion is the hardcoded literal
`grep -v '^openspec/changes/live-refresh/'` and its `ls-files --others` sweep therefore reports
every file of this change's own artifact directory as a stray write.

1. replace `         | grep -v '^openspec/changes/live-refresh/' | sort -u || true)`
   with `         | grep -v "^openspec/changes/$CHANGE/" | sort -u || true)`
2. immediately **after** the `ROOT=$(git rev-parse --show-toplevel) ...` line, insert
   `CHANGE="${CHANGE:-agent-polling}"`, a `case` rejecting a name with any character outside
   `[A-Za-z0-9._-]` or an empty one, and
   `[ -d "$ROOT/openspec/changes/$CHANGE" ] || { echo "FAIL: no such change directory: openspec/changes/$CHANGE - the exclusion is vacuous" >&2; exit 1; }`.
   The guard must sit after `ROOT` is assigned, not before it.

Parameterising is the repair; widening the exclusion to `^openspec/changes/` would neuter the
gate for every other in-flight change in a shared repository. Verified at planning time:
unedited it reported this change's ten artifact files as stray writes; edited it is green, red
on a planted `openspec/specs/STRAY.tmp`, and red on `CHANGE=nope`.

**Extracted byte-identically rather than retyped** — **twenty-two** blocks, none of which
changes. From `.../2026-09-06-live-refresh/tasks.md`: `WATCHSEAM.sh`, `READONLY-UI.sh`. From
`.../2026-09-05-detail-view/tasks.md`: `NOSPAWN-GREP.sh`, `READSEAM.sh`, `NOCLI-SHELL.sh`,
`NOLIT-CHANGE.sh`, `MDSEAM.sh`, `NOTABSEAM.sh`, `WIDTHS.sh`, `DETAILWIDTHS.sh`. From
`.../2026-09-05-markdown-viewer/tasks.md`: `NORAW-GREP.sh`, `LISTWIDTHS.sh`, `MDWIDTHS.sh`,
`NOWAIVER.sh`, `TESTCOUNT.sh`, `DEPS.sh`, `GRAPH-SNAP.sh`. From
`.../2026-09-05-tasks-tab/tasks.md`: `NOIO-VIEW.sh`, `TASKSEAM.sh`, `TASKWIDTHS.sh`. From
`.../2026-09-04-changes-from-cli/tasks.md`: `GATE-MECH1.py`, `NOJSON-SEAM.sh`.
`NOBLOCK`, `NODEFAULT-UI`, `NOSLEEP`, and `OPENSPEC-UNTOUCHED` are extracted from those same
files and **then** edited in task 0.4, so task 0.2 diffs the extraction and task 0.4 diffs the
edit.

Together: **two** blocks reproduced above plus **twenty-six** extracted, for **twenty-eight**
files in `$CHECKS` — twenty-seven gates plus `TESTCOUNT.sh`, which is sourced rather than run.
`GATE-MECH1.py`'s label sits on its block's *second* line behind a shebang; scan the first two
lines for it.

`DEPS` and `GRAPH-SNAP` are **not** edited: this change adds no dependency, so both must be
green unchanged, and a failure there is a real finding.

---

## 0. Baseline, checks, and the scratchpad
<!-- kind: operational -->

- [x] 0.1 CHECK: Record the starting state by **measuring**, never by copying a number from
      this file. `export BASE=$(git rev-parse HEAD)`; then run each command in the "Measured
      at planning time" table and record its output verbatim.
      **Red when:** `BASE` is empty or not a commit, or any measured figure differs from the
      table. Prefer `BASE=<sha> sh $CHECKS/OPENSPEC-UNTOUCHED.sh` at each invocation over an
      exported variable: re-exporting from the current `HEAD` after a fresh shell silently puts
      this change's own commits inside the baseline.

- [x] 0.2 CHECK: Extract all twenty-eight files to `$CHECKS`, then prove the extraction landed.
      For each of the twenty-six extracted from an archived tasks file, extract the same block
      a second time by an independent route and `diff` the two; every diff must be empty.
      **Red when:** `ls -1 "$CHECKS" | wc -l` is not 28, any diff is non-empty, or
      `$CHECKS/GATE-MECH1.py` is absent because the extractor read only each block's first line.

- [x] 0.3 CHECK: Run all twenty-seven gates against the unmodified tree at `BASE` and record
      each exit status and its first line of output verbatim.
      **Expected:** `AGENTSEAM` red with `src/agents.rs missing - the subject is gone`; `WIRED`
      red with `positive control - src/cli.rs defines no 'pub fn agent_cli_via('`;
      `OPENSPEC-UNTOUCHED` red listing this change's own artifact files; every other gate green
      at its **landed** invocation. A *different* failure message from any of the three is a
      defect in the block, not the expected guard, and is fixed here.

- [x] 0.4 CHANGE: Apply the four edits above to `NOBLOCK.sh`, `NODEFAULT-UI.sh`, `NOSLEEP.sh`,
      and `OPENSPEC-UNTOUCHED.sh` by exact string replacement, printing each resulting `diff`
      before running. Then re-run all four at their landed invocations.
      **Expected:** `NOBLOCK` red with `src/agents.rs missing - leg 3 has a third seam module
      to sweep`; `NODEFAULT-UI` and `OPENSPEC-UNTOUCHED` green.
      **Task-file correction (found during implementation):** `NOSLEEP` is **also** red here,
      with `NOSLEEP FAIL (leg 2b): src/agents.rs missing` — its own edit adds `src/agents.rs`
      to leg 2b's `for f in …; do [ -f "$f" ] || fail "$f missing"` loop, the same
      existence-guard shape `NOBLOCK`'s edit adds, and that guard fires for the same reason
      `NOBLOCK`'s does: the file does not exist until task 1.2. The original "the other three
      green" was written without accounting for that guard; it is a task-file bookkeeping
      correction, not a behavior or contract change, and does not affect `OPENSPEC-UNTOUCHED`,
      `git status --porcelain` outside `$CHECKS`, or any other verification.

- [x] 0.5 CHECK: Prove `NODEFAULT-UI`'s repair actually repaired it. Plant
      `fn zz(d: Dashboard) -> Dashboard { Dashboard {\n quit: true,\n ..d\n} }` at the end of
      `src/ui/app.rs`'s production slice, run the edited check, and record exit **1**; run the
      **unedited** extraction against the same plant and record exit **0**; remove the plant and
      confirm the edited check is green again.
      **Red when:** the edited check exits 0 under the plant, or the unedited one exits 1 —
      either means the defect design.md → Decisions 12 describes is not the one that was there.

- [x] 0.6 CHECK: Prove `OPENSPEC-UNTOUCHED`'s repair is not a widening. Plant an empty
      `openspec/specs/STRAY.tmp`, run the edited check and record exit **1** naming that path;
      remove it; then run with `CHANGE=nope` and record exit **1** naming the vacuous exclusion.
      **Red when:** either plant is green — the exclusion has been widened past this change's own
      directory.

- [x] 0.7 VERIFY: `git status --porcelain` shows no change under `openspec/` outside
      `openspec/changes/agent-polling/`, and `BASE=$BASE sh $CHECKS/OPENSPEC-UNTOUCHED.sh` is
      green. Commit nothing in this group — it writes only to `$CHECKS`.

## 1. The skeleton every later group compiles against
<!-- kind: refactor -->

Structure only: inert signatures, no behaviour. The evidence is that the 752 landed tests stay
green while the tree grows the API groups 2 to 10 fill in.

- [x] 1.1 CHARACTERIZE: Run `. $CHECKS/TESTCOUNT.sh; testcount --lib '' 752` and record the
      count, so the 752 landed tests are green before any file is touched.
      **This characterization deliberately does not cover `ui::run`**, which task 1.5 splits:
      the landed suite never drives it, which is precisely why `live-refresh` shipped it
      unwired. Group 2's acceptance tests are that function's first characterization, and they
      are red until group 10 — so 1.8's green `make check` is evidence about the other nine
      files this group touches, and not about the split.

- [x] 1.2 REFACTOR: Add `pub mod agents;` to `src/lib.rs` and create `src/agents.rs` with the
      full shape from design.md → Contracts: `POLL_INTERVAL`, `AgentStatus`, `Agent`, `Listed`,
      `AgentSnapshot`, `parse_list` returning `Err("not implemented")`, `poll_once` returning a
      constant `AgentSnapshot { agents: vec![], reachable: true, problem: None }` **whatever the
      CLI answers**, the `AgentPoll` trait, `NoAgentPoll`, `none()`, `RealAgentPoll` with an
      always-`None` `drain`, and `start` spawning a worker that answers nothing. No `Default` on
      `Agent`, `Listed`, or `AgentSnapshot`. The constant is deliberately wrong in every
      direction so group 4's six tests are red on behaviour rather than on a stub that already
      guessed right.

- [x] 1.3 REFACTOR: Add `cli::HERDR_PROGRAM` and `cli::agent_cli_via` to `src/cli.rs`, the
      latter returning a handle over a **fixed** program path so group 5's tests are red, and
      `watch::soonest` to `src/watch.rs` returning `None` unconditionally.

- [x] 1.4 REFACTOR: Add the third field `agents: &'a mut dyn crate::agents::AgentPoll` to
      `ui::driver::Live` and the tenth field `agents: agents::AgentSnapshot` to
      `ui::app::Dashboard`, initialised at both of `ui::load`'s branches to
      `AgentSnapshot { agents: Vec::new(), reachable: false, problem: None }` written as a
      literal. Update every landed construction site the compiler names — no `..` rest anywhere.

- [x] 1.5 REFACTOR: Add `ui::Startup`, `ui::Collaborators`, `ui::start_collaborators`, and
      `ui::run_wired` per design.md → Contracts, and reduce `ui::run` to the terminal guard, the
      panic hook, `config::load_from_env`, `current_dir`, the real `Terminal`, the `Startup`
      literal, and one `run_wired` call.

- [x] 1.6 REFACTOR: Add `testutil::ScriptedAgents` — an `AgentPoll` double returning a scripted
      `Vec<Option<AgentSnapshot>>` from `drain` and a scripted `Vec<Option<Duration>>` from
      `pending_in`, recording both call counts — and `testutil::UntilReady`, an `EventSource`
      that calls `std::thread::yield_now()` and returns `Ok(None)` until a caller-supplied
      `&dyn Fn() -> bool` has held for a 50ms settle window or a 5s deadline passes, then one
      `q` press, recording its timeouts as `Script` does. The clock lives here and nowhere under
      `src/ui/`.

- [x] 1.7 CHECK: Contract gate — re-inspect the published interface. `git add -N src/agents.rs`
      first so the new file is visible to `git diff`, then
      `git diff $BASE -- src/lib.rs src/agents.rs src/cli.rs src/watch.rs src/ui/driver.rs
      src/ui/app.rs src/ui/mod.rs | grep '^+pub '` and confirm every added `pub` item is one
      design.md → Contracts names, and that no landed `pub` signature changed.
      **Red when:** the diff is empty for `src/agents.rs` — that means the file is still
      untracked and the gate saw nothing.

- [x] 1.8 VERIFY: `testcount --lib '' 754` — the 752 landed tests plus `testutil::tests::`'s two
      new ones from 1.6 — then `cargo fmt --all -- --check`, `cargo clippy --all-targets
      --all-features -- -D warnings`, and `make check`. Commit.

## 2. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

This is the group that exists because `live-refresh` shipped an inert live tier with every test
green. It drives `ui::run_wired` — the composition root itself — not the components it composes.

- [ ] 2.1 RED (harness): Set up what design.md → Test Boundaries names — a
      `testutil::ScratchDir` repository holding `alpha` with a `tasks.md` counting 4 of 9 and a
      vendored `tdd` schema (the helpers `ui::tests::live` already has); a scratch `#!/bin/sh`
      `herdr` program that appends its arguments to a log file and prints the one-agent
      `agent_list` envelope from `specs/agent-list/spec.md`; and a scratch `#!/bin/sh`
      `openspec` program that appends its arguments to a second log and answers `list --json`,
      named through `Config::openspec_bin` so the probe chain stops at step 1.

- [ ] 2.2 RED: Write `ui::tests::wiring::the_real_wiring_polls_a_scratch_herdr` for the scenario
      "The real wiring polls a scratch Herdr and adopts what it says", at 120x20 and 60x20. Its
      `UntilReady` predicate waits for both logs, then writes one byte to
      `openspec/changes/alpha/proposal.md`, then waits for the `openspec` log to reach **two**
      entries. Assert `agents.reachable` with one agent at `pane_id` `w8:p1`; the `herdr` log
      recording `agent list`; the `openspec` log holding two or more runs; `refresh.problems`
      empty; and the `alpha` row reading `[4/9]`.

- [ ] 2.3 RED: Write `ui::tests::wiring::an_unreachable_scratch_herdr_is_a_standalone_tui` for
      "An unreachable scratch Herdr leaves the pane a working standalone TUI", with a `herdr`
      path that does not exist and a predicate naming only the `openspec` log and writing
      nothing. This is where the byte-identity snapshot pair and its one-byte discriminating
      control live, since this run writes nothing of its own.

- [ ] 2.4 RED: Write `ui::tests::wiring::no_repository_still_polls_for_agents` for "A pane with
      no OpenSpec repository still polls for agents", over a `ScratchDir` holding no `openspec/`
      directory, asserting `agents.reachable` true and the `openspec` log **empty**.

- [ ] 2.5 Confirm all three fail for the right reason: run them and record each assertion
      message.
      **Red when:** the failure is a compile error, a missing fixture, or an `UntilReady`
      deadline with no assertion message — each means the harness is misconfigured rather than
      the behaviour missing. The expected message names `reachable` as `false`.

- [ ] 2.6 VERIFY: `cargo test --all-features` fails on **exactly** these three tests, and
      `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` is green. Commit.

## 3. `agents::parse_list`
<!-- kind: behavior -->

- [ ] 3.1 RED: Write the thirteen `agents::tests::parse::` tests named in design.md → Test
      Strategy, covering the reference payload, the empty array, a named agent, every status
      string plus three unrecognised forms, an entry with only the required identity fields, a
      skipped entry in three variants, a non-object entry in three variants, a `cwd` outside the
      repository, non-JSON text, four wrong shapes, and Herdr's own error envelope. Confirm each
      fails on the stub's `Err("not implemented")`, not on a fixture typo.

- [ ] 3.2 GREEN: Implement `parse_list` over `serde_json::Value`, following
      `changes::parse_list`'s navigation style: the envelope, then `result.agents`, then one
      `Agent` per entry with `pane_id`/`tab_id`/`workspace_id` required and everything else
      optional, and one `problems` line per skipped entry naming the fault and the index.

- [ ] 3.3 REFACTOR: Extract the per-entry decode into its own function if 3.2 left `parse_list`
      doing two jobs; otherwise record that no refactor was needed.

- [ ] 3.4 VERIFY: `testcount --lib 'agents::tests::parse::' 13`, then the four gates
      individually (`cargo llvm-cov --ignore-run-fail`). Commit.

## 4. `agents::poll_once`
<!-- kind: behavior -->

- [ ] 4.1 RED: Write the six `agents::tests::poll::` tests: the argument-vector equality on
      `FakeCli::calls()`, a `Failed` error, a `NotStarted` error, an unparsable success, a
      partial payload that is still reachable, and a clean success. All six fail against 1.2's
      constant snapshot, which answers `reachable: true` with no agents whatever the CLI said.

- [ ] 4.2 GREEN: Implement `poll_once`'s four-way mapping per `specs/agent-list/spec.md` → "A
      failed run is an unreachable socket carrying the program's own reason", joining
      `Listed::problems` into `problem` when non-empty.

- [ ] 4.3 REFACTOR: Fold the two error arms into one `CliError` formatter if 4.2 duplicated the
      message shape; otherwise record that no refactor was needed.

- [ ] 4.4 VERIFY: `testcount --lib 'agents::tests::poll::' 6`, then `MIN=22 sh
      $CHECKS/AGENTSEAM.sh` — it must now be **green**, since `src/agents.rs` exists and names
      `HerdrCli`. Record its OK line. Commit.

## 5. `cli::agent_cli_via` and `cli::HERDR_PROGRAM`
<!-- kind: behavior -->

- [ ] 5.1 RED: Write the three `cli::tests::` tests, all three about what the binding *does*
      rather than what it is called: `agent_cli_via` against a scratch `#!/bin/sh` printing
      `hello`, against one exiting `3` with `boom` on stderr, and against a path that does not
      exist. They fail against 1.3's fixed-program stub. No test asserts the value of
      `HERDR_PROGRAM`: a test restating a constant the implementation also declares proves only
      that the string was typed twice, and `WIRED`'s positive control is what pins it.

- [ ] 5.2 GREEN: Implement `agent_cli_via` as one line over `RealHerdrCli::new(program)`, with
      `HERDR_PROGRAM` the only place the literal `herdr` is written as a program name.

- [ ] 5.3 REFACTOR: No refactor is expected — the function is one expression. Record that.

- [ ] 5.4 CHECK: Contract gate — `src/cli.rs` still names no `serde_json` and no parse
      (`grep -n 'serde_json' src/cli.rs` is empty), so the seam gained a binding and no logic.

- [ ] 5.5 VERIFY: `testcount --lib 'cli::tests::' 40`, then `MIN=22 sh $CHECKS/NOSPAWN-GREP.sh`
      and `sh $CHECKS/NOJSON-SEAM.sh`. Commit.

## 6. `watch::soonest`
<!-- kind: behavior -->

- [ ] 6.1 RED: Write the four `watch::tests::soonest_*` tests covering all four presence
      combinations, commutativity, and the two `poll_timeout(250ms, soonest(...))` compositions
      from `specs/watch-invalidation/spec.md` → "One tick serves two pollers". Confirm they fail
      against the group-1 stub's unconditional `None`.

- [ ] 6.2 GREEN: Implement `soonest` as a pure minimum over two `Option<Duration>`, reading no
      clock and taking no `Instant`.

- [ ] 6.3 REFACTOR: No refactor is expected — the function is one `match`. Record that.

- [ ] 6.4 VERIFY: `testcount --lib 'watch::tests::' 31`, then `MIN=24 sh $CHECKS/WATCHSEAM.sh`.
      Commit.

## 7. The poller: schedule, worker, and the seam
<!-- kind: behavior -->

- [ ] 7.1 RED: Write the three `agents::tests::seam::` tests (the inert poller, `POLL_INTERVAL`,
      and the no-`Default` companions) and the seven `agents::tests::worker::` tests (first
      drain polls immediately, a poll in flight suppresses the next, a started poller yields the
      scratch program's agents, a failing scratch program is unreachable, a failed poll is
      recovered from, a dead worker is reported once, dropping the poller stops the thread). Add
      `poller_for_test` **inside** `mod tests`, shaped exactly as `refresh::worker_for_test` is
      — its poller's `drain` yields nothing and the answers arrive on a raw receiver — so Guard D
      still sees exactly one line-anchored `#[cfg(test)]`.

- [ ] 7.2 GREEN: Implement `RealAgentPoll::drain` per design.md → Decisions 2 — one clock read,
      `try_recv`, next-due set from that same `now`, at most one request in flight, `pending_in`
      cached — and `worker_body` as a `recv` loop calling `poll_once`.

- [ ] 7.3 REFACTOR: Confirm `fn drain`'s last declaration still precedes the single
      `thread::spawn`, and that the worker body sits below it; move nothing else.

- [ ] 7.4 VERIFY: `testcount --lib 'agents::tests::seam::' 3` and
      `testcount --lib 'agents::tests::worker::' 7`, then `sh $CHECKS/NOBLOCK.sh` — it must now
      be **green** on all three legs. Record its three OK lines. Commit.

- [ ] 7.5 CHECK: Prove `NOBLOCK`'s new coverage can fail. Plant each of (a)
      `self.result_rx.recv_timeout(POLL_INTERVAL)` in `RealAgentPoll::drain`, (b) the
      `impl AgentPoll for RealAgentPoll` block moved below `agents::start`, (c) a second
      line-anchored `#[cfg(test)]` above `start`, and (d) every `try_recv` renamed to a
      different identifier; run the check after each, record the FAIL line, and revert. All four
      were run against a stub module at planning time and reported, in order: leg 3 blocking
      before the spawn, Guard E's line comparison, Guard D's count, and the `try_recv` positive
      control. Plant (d) must rename the token, not wrap it — `try_recv_x` still contains
      `try_recv` and stays green, which was measured.

- [ ] 7.6 CHECK: Prove `NOSLEEP`'s leg 2b extension can fail. Plant two `thread::sleep` calls in
      `src/agents.rs`, run `SLEEP_MIN=4 MIN=25 sh $CHECKS/NOSLEEP.sh`, record exit **1** naming
      `src/agents.rs names a sleep 2 times`, and revert. Then run it clean and record green.
      Commit.

## 8. `Dashboard.agents` and the proof that nothing renders it
<!-- kind: operational -->

Group 1 already landed the field, and nothing renders it *by design*, so no test here can
honestly start red. These are guards written against structure that already exists — the
lifecycle the schema names for that is CHECK → CHANGE → VERIFY, and the deterministic failing
check is 8.4's plant rather than an invented RED.

- [ ] 8.1 CHECK: Write `ui::view::tests::agents_change_no_pixel` — the same `Dashboard` rendered
      at 120x20 and 60x20 with the initial snapshot, a two-agent reachable snapshot, and an
      unreachable snapshot carrying a long `problem`, asserting the three buffers are identical
      cell for cell at each width — and the four `ui::app::tests::` tests (the ten-field
      exhaustive destructure, the `Agent`, `Listed`, and `AgentSnapshot` companions, and
      `dashboard_is_clone_and_eq_with_agents`). Run them and record that they pass, which is the
      finding: the field is inert and stays inert.

- [ ] 8.2 CHANGE: If any of the five is red, fix `src/ui/app.rs` or `src/ui/view.rs` until it is
      green — a red result here means group 1 left the field rendered or defaulted. Otherwise
      record that no change was needed.

- [ ] 8.3 CHECK: Plant `#[derive(Default)]` above `struct Agent` and, separately, a multi-line
      `..s` inside an `AgentSnapshot` literal; run
      `HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot' SCAN_MIN=<measured>
      sh $CHECKS/NODEFAULT-UI.sh` after each, record exit **1** and the FAIL line, and revert.
      Both were run against a stub module at planning time and reported half A and half B
      respectively.

- [ ] 8.4 CHECK: Plant a `Change {` literal in `src/agents.rs`, run
      `MIN=22 sh $CHECKS/NOLIT-CHANGE.sh`, record exit **1**, and revert — the plant
      `specs/dashboard-loop/spec.md` requires and the only one that proves the file-count raise
      from 21 to 22 covers the new module.

- [ ] 8.5 VERIFY: `testcount --lib 'ui::app::tests::' 59` and `testcount --lib
      'ui::view::tests::' 82`, then `SCAN_MIN=90 TYPES='Dashboard Filter Detail Refresh' sh
      $CHECKS/NODEFAULT-UI.sh` and the second invocation above. Measure the second invocation's
      span count from its own OK line and set its `SCAN_MIN` to that measured value — never
      above it, and never below 1. Commit.

## 9. `Live`'s third field and the loop's fourth live step
<!-- kind: behavior -->

- [ ] 9.1 RED: Write the seven new `ui::driver::tests::` tests —
      `the_wait_takes_the_soonest_of_two_deadlines` (exact equality on three recorded timeouts),
      `a_snapshot_reaches_the_frame_and_survives_adopt` and
      `an_unreachable_socket_is_not_a_problem_row` (both at 120x20 **and** 60x20), the
      exhaustive `Live` destructuring companion, and three covering the per-frame `drain` call
      count, step 4's independence from step 3, and the wholesale replacement of
      `dashboard.agents`. Amend the thirteen landed `ui::driver::tests::` tests to build a
      three-field `Live` with `agents::none()`.

- [ ] 9.2 GREEN: Add step 4 to `drive_live_tier` and change the wait to
      `watch::poll_timeout(tick, watch::soonest(live.fs.pending_in(), live.agents.pending_in()))`.

- [ ] 9.3 REFACTOR: Keep `drive_live_tier` one function unless step 4 pushed it past the point
      where each step is readable; otherwise record that no refactor was needed.

- [ ] 9.4 CHECK: Contract gate — `run_loop`'s parameter count is still six, and this change adds
      no `#[allow]`: `git diff $BASE -- src/ tests/ | grep '^+.*#\[allow'` is empty. A tree-wide
      grep would be red before and after, since `src/changes.rs:966` already carries one.

- [ ] 9.5 VERIFY: `testcount --lib 'ui::driver::tests::' 29`, then `sh $CHECKS/NOBLOCK.sh`,
      `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`, and `sh $CHECKS/NOIO-VIEW.sh`. Commit.

## 10. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 10.1 GREEN: Fill in `start_collaborators` and `run_wired` per design.md → Contracts, with
      the poller started unconditionally and the watcher and worker only when a repository was
      found, so all three group-2 tests pass end to end.

- [ ] 10.2 VERIFY: `testcount --lib 'ui::tests::wiring::' 3` and `testcount --lib
      'ui::tests::live::' 3`, and confirm every landed `ui::` buffer assertion is unchanged.

- [ ] 10.3 CHECK: Run the three wiring plants and record each red result verbatim, reverting
      after each: `agents::none()` for `agents::start` must fail on `agents.reachable`;
      `refresh::none()` for `refresh::start` must fail on the scratch `openspec` log being
      empty; `watch::none()` for `watch::start` must fail on that log stopping at **one** entry.
      **Red when:** any plant leaves the tests green — the assertion is not discriminating and
      the group-2 test is the `live-refresh` defect reproduced rather than caught. The watcher
      plant is the one that matters most: `refresh.problems` is empty under both arms, so it is
      a supporting assertion only.

- [ ] 10.4 VERIFY: `sh $CHECKS/WIRED.sh` — it must now be **green**. Then run its six plants
      (drop each of the three `*::start` calls, add a branch to `run`, hardcode `"herdr"` in
      `run`, and move the seven names into `mod tests` behind a mis-anchored `#[cfg(test)]`),
      record each FAIL line, and revert. All six were run against a scratch tree at planning
      time and reported leg 1 three times, leg 2, leg 4, and Guard D.

- [ ] 10.5 CHECK: Prove `AGENTSEAM`'s legs can fail at implementation time, not only at planning
      time: plant `Command::new` in `src/agents.rs`, a doc comment naming `Frame` there, and a
      `crate::cli::agent_cli_via(...)` call in `src/ui/list.rs`; record the three FAIL lines and
      revert each.

- [ ] 10.6 REFACTOR: Fold the two scratch-program builders and the scratch repository helper
      into one place in `ui::tests::wiring` if 2.1 left them duplicated; otherwise record that
      no refactor was needed.

- [ ] 10.7 VERIFY: `make check` unqualified — the first run of the literal composite target
      since group 1. Commit.

## 11. The architecture checks, all green together
<!-- kind: operational -->

- [ ] 11.1 CHECK: Run all twenty-seven gates at their landed invocations, in one pass, and
      record every exit status and OK line: `MIN=22 NOSPAWN-GREP`; `MIN=22 NOLIT-CHANGE`;
      `UI_MIN=11 NOCLI-SHELL`; `NOIO-VIEW`; `READSEAM`; `MDSEAM`; `NOTABSEAM`; `TASKSEAM`;
      `MIN=24 WATCHSEAM`; `MIN=22 AGENTSEAM`; `NOBLOCK`; `SLEEP_MIN=4 MIN=25 NOSLEEP`;
      `UI_MIN=11 EXTRA='src/watch.rs src/refresh.rs src/agents.rs' READONLY-UI`; `WIRED`;
      both `NODEFAULT-UI` invocations; `WIDTHS`; `DETAILWIDTHS`; `LISTWIDTHS`; `MDWIDTHS`;
      `TASKWIDTHS`; `NORAW-GREP`; `NOWAIVER`; `NOJSON-SEAM`; `GATE-MECH1.py`; `WORK=... DEPS`;
      `GRAPH-SNAP`; `BASE=$BASE OPENSPEC-UNTOUCHED`.
      **Red when:** any is not green, including `DEPS` and `GRAPH-SNAP`, which this change does
      not edit because it adds no dependency.

- [ ] 11.2 CHECK: Prove `READONLY-UI`'s extension is load-bearing, not decorative. Plant
      `std::fs::write` in `src/agents.rs`'s **production** slice, run with `src/agents.rs` in
      `EXTRA` (expect exit 1) and again without it (expect exit 0), then revert. Both were run at
      planning time and produced exactly that pair.

- [ ] 11.3 VERIFY: `make check`, then `BASE=$BASE sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

## 12. Documentation
<!-- kind: operational -->

Four corrections, every one of which this change proved false by running Herdr 0.8.2 live.
Net effect on `SPEC.md` is about +16 lines and −5; on `AGENTS.md`, two rewritten sentences.

- [ ] 12.1 CHECK: Before editing, record what each target currently says:
      `grep -n 'agent_status\|terminal_title' SPEC.md`; `grep -n "crate's only clock binding"
      SPEC.md`; `grep -n 'one thread' SPEC.md`; `grep -n 'exits non-zero' SPEC.md`;
      `grep -n 'src/watch.rs. and .src/refresh.rs' AGENTS.md`. Every one must match something —
      a pattern that already matches nothing is a target that has already moved, and 12.6's
      after-grep would then prove nothing.

- [ ] 12.2 Rewrite in `SPEC.md`: "Herdr integration → Agent status by polling" (audience:
      every later change reading the design contract) — replace the bare field list with the
      real envelope `{"id":…,"result":{"agents":[…],"type":"agent_list"}}`, name `name` beside
      `agent`, say that `agent` is the agent **kind**, record that only `pane_id`, `tab_id`,
      `workspace_id`, `terminal_id`, `focused`, `revision`, and `agent_status` are required and
      the rest may be absent, and state that **no `--json` flag exists** (Herdr 0.8.2 exits 2).
      This replaces a field list that would have had `agent-attribution` matching change names
      against the string `claude`.

- [ ] 12.3 Rewrite in `SPEC.md`: "Degraded states" (audience: same) — scope the row "A CLI
      command exits non-zero | the reason is unavailable to the plugin" to the **`openspec`**
      CLI, and add a row for `herdr`, which exits 1 with an empty stdout and a JSON error
      envelope on stderr, so `CliError::Failed`'s stderr does carry the reason.

- [ ] 12.4 Rewrite in `SPEC.md`: "Architecture" and "Testing and quality gates → Unit-tested
      modules" (audience: same) — the sentence calling `watch::RealFsEvents::drain`'s
      `Instant::now()` "the crate's only clock binding" is now false; there are **two**. The
      "the crate's only one" thread claim about `refresh` is now false; there are **two**. Also
      update the module-map row for `agents` to name the poll and the parse, not attribution
      alone, and add `agents` to the unit-tested module list.

- [ ] 12.5 Rewrite in `AGENTS.md`: "Architecture rules" → the render-path bullet (audience:
      every agent session) — it names `src/watch.rs` and `src/refresh.rs` as the two seam
      modules and says "neither seam module's own production code blocks". Rewrite in place to
      name three, and add one line stating that the poller lives outside `src/ui/` because
      `NOCLI-SHELL` forbids naming `HerdrCli` there. Delete the two-module wording rather than
      appending beside it. **`AGENTS.md` carries no "only clock binding" claim** — that sentence
      is `SPEC.md`'s alone, and 12.4 owns it.

- [ ] 12.6 VERIFY: `grep -n "only clock binding\|one real clock" SPEC.md` returns nothing;
      `grep -n 'the crate.s only one' SPEC.md` returns nothing; `grep -n 'src/watch.rs. and
      .src/refresh.rs' AGENTS.md` returns nothing. Do **not** grep for the absence of `--json`
      near "Herdr" in `SPEC.md`: 12.2 requires that pair of words to appear together, so such a
      gate would be red exactly when the task was done correctly. Commit.

## 13. Change Review
<!-- kind: operational -->

- [ ] 13.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against
      `proposal.md`, all six spec files, `design.md`, `tasks.md`, and `git diff $BASE`. Give it
      the concentration points from `openspec/config.yaml` → `rules.tasks` and, additionally:
      every verification command must be able to fail and to see what it guards; no test may
      assert on a double's own return value; the acceptance test's three wiring assertions must
      each be separately discriminating; and `run`'s untested residue must hold no decision.

- [ ] 13.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note each SUGGESTION, and re-run the affected tests and gates.

- [ ] 13.3 VERIFY: No blocking or unowned finding remains, and `make check` is green. Commit.

## 14. Lint & Verify
<!-- kind: operational -->

- [ ] 14.1 CHECK: Inspect the intended verification commands and affected tiers — the four gates
      below, the twenty-seven `$CHECKS` gates from task 11.1, and the eleven `testcount` floors.

- [ ] 14.2 VERIFY: `cargo fmt --all -- --check` — clean.

- [ ] 14.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings. Rust
      has no separate type checker; clippy's run is the type check.

- [ ] 14.4 VERIFY: `cargo test --all-features` — green, and `testcount --lib '' 805`.

- [ ] 14.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` — green, and record the **line**
      percentage and line total from the TOTAL row, not the region count. The floor is not
      lowered and no exclusion is added; if coverage falls short, tests are added.

- [ ] 14.6 VERIFY: `make check` as the single composite gate; if it fails, name the failing
      sub-command rather than summarising.

- [ ] 14.7 VERIFY: `openspec validate agent-polling --strict` — valid.
