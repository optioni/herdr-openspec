# live-refresh — tasks

Read `design.md` first: the Boundaries table, the Test Boundaries table, and the verification
matrix are the contract these tasks implement. **No task may invent a collaborator that table
does not name.**

**The outer loop is taken**, at the `ui::` composition tier through `run_loop`.
`ui::app::action_for` → `Dashboard::apply` → `run_loop`'s live tier → `Refresher` →
`Dashboard::adopt` → `Dashboard::sync_detail` → `ui::view::render` is a path no unit test
crosses, and "files paint, the CLI corrects" is a claim about what the **loop** shows in
successive frames rather than about what a function returns. Group 2 writes that test and
group 11 closes it.

**Group 1 comes before the acceptance test, and that is deliberate**, on the terms
`markdown-viewer`, `detail-view`, and `tasks-tab` all recorded: the acceptance test needs a
ninth `Dashboard` field, a thirteenth `Action` variant, two new modules, two new test
doubles, and a widened `run_loop` signature before it can compile. Group 1 is
**operational**, because a field, a variant, a signature, and two doubles are plumbing rather
than behaviour — `Action::Refresh` gets a `=> {}` arm and no `action_for` mapping, the two
new modules hold inert implementations only, and nothing observable changes. Group 2's RED is
an assertion failure against a pane that never updates, not a compile failure. A crate whose
test target does not build makes `cargo clippy --all-targets` and
`cargo llvm-cov --ignore-run-fail` unrunnable, and would leave groups 3 to 10 with no gate at
all.

**Every floor in this file is measured before it is raised.** Task 0.1 reads the real counts
off `main` and records them; every later `MIN`, `UI_MIN`, `WIDTHS_MIN`, `LIST_MIN`,
`SCAN_MIN`, `SLEEP_MIN`, and `testcount` minimum is *measured + this change's enumerated new
test functions*, with the arithmetic shown. **A realized count below a group's target means
the missing test is written, not the floor lowered.** This repository has already shipped a
check whose floor was asserted rather than measured and was therefore unpassable, and one
whose target (`WIDTHS_MIN=72`) was planned above what the change actually landed and had to
come down to a measured 67. That is why every target below is written as *measured +
enumerated*, and why the 80% coverage floor has not moved in fifteen changes and does not
move here.

**When a task says a check script was edited, that task writes the script to disk, shows the
edit landed, and runs it.** `tui-shell` recorded four edits to `DEPS` as prose in a checkbox
only; the script on disk was never changed, and `list-view` nearly halted on a check that did
not exist. Every block reproduced below is extracted to `$CHECKS/<LABEL>.sh` in task 0.1, and
every later run is of the extracted file. Blocks that are **not** reproduced below are
extracted byte-identically from a named archived file, and task 0.2 diffs each extraction
against its source so an extraction that silently produced the wrong bytes fails there rather
than passing a check that is not the one anybody reviewed. `DEPS` and `GRAPH-SNAP` are edited
by **exact string replacement** in task 6.2, which prints the resulting `diff` before running
them — the edits are specified below to the character, not described.

**Four checks cannot run green until the modules they guard exist**, and that is stated here
rather than discovered:

| Check | First green at | Why it is red before then |
|---|---|---|
| `READONLY-UI` with `EXTRA` | task 1.10 | `[ -f src/watch.rs ]` and `[ -f src/refresh.rs ]` guards, satisfied by task 1.5 |
| `WATCHSEAM` | task 6.6 | its positive control requires `src/watch.rs` to name `notify`, which arrives with the dependency in group 6 |
| `NOBLOCK` legs 2 and 3, watch controls | task 6.6 | their positive controls require `src/watch.rs` to name a clock and `try_recv` |
| `NOBLOCK` legs 1 and 3, refresh controls | task 7.5 | their positive controls require `src/refresh.rs` to name `mpsc`, `thread::spawn`, and `try_recv` |

`NOSLEEP` is **not** in that table: both its legs are green on unmodified `main`, because leg
2 covers `src/ui/` alone. Only its `SLEEP_MIN` floor moves, from 3 to 4, at group 6.

Task 0.3 runs all four and **records the expected guard failure verbatim**. A *different*
failure message from any of them is a defect in the block, not the expected guard, and is
fixed there rather than in the group that first runs it green. The unedited forms of the four
are not kept around as stand-ins: two versions of one check on disk is exactly how a run and
a record drift apart.

**`NOSLEEP` is not a blanket prohibition on sleeping, and that is measured rather than
assumed.** `main` already carries **three** `thread::sleep` calls — `src/cli.rs`'s
`a_program_that_reads_stdin_returns_rather_than_blocking` and `tests/cli.rs`'s two `try_wait`
polls — and all three are correct: each is the pause inside a `while Instant::now() <
deadline` loop, and `tests/cli.rs:141` carries the comment recording the very flake that
produced the rule. A blanket rule would have been RED on the unmodified tree and the only
ways out would have been deleting three correct tests or exempting two files. `NOTABSEAM`
shipped red on an unmodified tree in this repository once already; this is that lesson
applied before the fact.

## Test-module convention — read before writing any test

`cargo test` matches a filter against a test's **full path**, and a filter matching nothing
exits 0. Every filtered gate below therefore addresses tests through an explicit module path,
and is judged on a counted minimum rather than on the run's exit status. `testcount` takes
the filter **and** the minimum as two arguments, in that order — `changes-from-cli` once bound
a `5` to the filter and left the minimum empty, so the gate could not fail.

Every target below is **measured at `main` plus the number of NEW test functions the group
enumerates**, and the two are written out so the arithmetic can be checked rather than
trusted. A scenario that *modifies* an existing test adds nothing to the count, and
**seventeen** of this change's scenarios do exactly that, and `tasks-checklist`'s and
`detail-scroll`'s deltas modify landed tests only and add none.

| Group | File | Module | Filter | Measured | New | Target |
|---|---|---|---|---|---|---|
| 3 | `src/changes.rs` | `mod tests` | `changes::tests::` | 180 | 14 | 194 |
| 4 | `src/watch.rs` | `mod tests` | `watch::tests::` | 0 | 11 | 11 |
| 5 | `src/watch.rs` | `mod tests` | `watch::tests::` | 11 | 11 | 22 |
| 6 | `src/watch.rs` | `mod tests` | `watch::tests::` | 22 | 5 | 27 |
| 7 | `src/refresh.rs` | `mod tests` | `refresh::tests::` | 0 | 8 | 8 |
| 8 | `src/ui/app.rs` | `mod tests` | `ui::app::tests::` | 48 | 7 | 55 |
| 9 | `src/ui/driver.rs` | `mod tests` | `ui::driver::tests::` | 14 | 8 | 22 |
| 10 | `src/ui/list.rs` | `mod tests` | `ui::list::tests::` | 17 | 3 | 20 |
| 10 | `src/ui/view.rs` | `mod tests` | `ui::view::tests::` | 76 | 5 | 81 |
| 2, 11 | `src/ui/mod.rs` | `mod tests` → `mod live` | `ui::tests::live::` | 0 | 3 | 3 |

**`ui::tests::live::` is a new submodule of `src/ui/mod.rs`'s `mod tests`**, beside the
existing `read_artifact`, `detail`, `load`, and `start`. Its floor is asserted **only at group
11**, never between groups 2 and 11: `testcount` counts tests that *passed*, and the
acceptance test is deliberately red for that whole span, so an intermediate floor would be
unpassable by construction.

The **library** total is `673` measured plus `14 + 11 + 11 + 5 + 8 + 7 + 8 + 3 + 5 + 3 = 75`,
i.e. **748**; the `ui::` subtotal is `260` measured plus `7 + 8 + 3 + 5 + 3 = 26`, i.e.
**286** — `watch::tests::` and `refresh::tests::` are outside `src/ui/` and contribute to the
library total only. Both are asserted in group 14 and nowhere else.

**`make check` is not runnable unqualified between groups 2 and 11.** Group 2's acceptance
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
The literal, unqualified `make check` is run from task 11.3 onward.

---

## Command-level checks, written out once

Referenced by label from the tasks below. They live here rather than in a table cell because
an unescaped `|` cannot appear in a Markdown table and an escaped `\|` inside an ERE matches a
literal pipe, so the check would pass against the very code it exists to catch. Every one is
judged on **output emptiness, a counted minimum, or a specific error code**, never on a bare
pipeline exit status: `grep` exits 1 for no-match and 2 for a bad file, and `!` turns both
into a pass.

Extract each block to `$CHECKS/<LABEL>.sh` in task 0.1 and run the extracted, byte-identical
file thereafter, so the run and the record cannot drift. **Every block below was written and
run against the real tree at planning time** — green where stated, and red against each
planted violation named in task 0.4.

```sh
# WATCHSEAM — NEW in live-refresh. The filesystem-watch crate is confined to ONE module, on
# exactly MDSEAM's and NOSPAWN-GREP's single-file terms: `notify` is replaceable by editing
# src/watch.rs, and no other file in the crate — tests/ included — may reach for it.
#
# The pattern is PATH-QUALIFIED and TOKEN-SPECIFIC, and that is not caution, it is a
# measured requirement. A bare `EventKind` alternative — the obvious one to reach for, since
# it is notify's own event type — matches `KeyEventKind` and `MouseEventKind` in
# src/ui/app.rs at TWELVE sites of correct, unmodified code, so the check would have been RED
# on `main` before this change wrote a line. Verified at planning time by running the bare
# pattern. NOTABSEAM shipped red on an unmodified tree in this repository once already; this
# is that lesson applied before the fact rather than after.
#
# `notify` as a bare word is not in the pattern either: it is an ordinary English verb and
# would match a doc comment saying "notify the caller".
#
# This check FAILS with "src/watch.rs missing" until the module exists, and with "names no
# notify API" until group 6 adds the dependency. Both are the guards doing their job, not
# defects: without them, a renamed or gutted module makes the check report a clean tree.
set -u
WATCH="${WATCH:-src/watch.rs}"
MIN="${MIN:-20}"
fail() { echo "WATCHSEAM FAIL: $1" >&2; exit 1; }

[ -d src ] || fail "no src directory"
[ -f "$WATCH" ] || fail "$WATCH missing - the exclusion has nothing to exclude"

NOTIFY_RE='notify::|notify_types::|RecommendedWatcher|RecursiveMode|FsEventWatcher|INotifyWatcher|use notify\b|extern crate notify'

# Guard A — positive control, checked BEFORE the count: the allowed file must actually name
# the crate, or the exclusion protects nothing and a clean result means nothing.
grep -qE "$NOTIFY_RE" "$WATCH" || fail "$WATCH names no notify API - exclusion is vacuous"

# Guard B — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/watch.rs is still searched.
n=$(find src tests -name '*.rs' ! -path "$WATCH" | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

hits=$(find src tests -name '*.rs' ! -path "$WATCH" -print0 \
       | xargs -0 -I{} grep -nE "$NOTIFY_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "WATCHSEAM FAIL: the watch crate is named outside $WATCH:" >&2
                    echo "$hits" >&2; exit 1; }

# Leg 2 — the confined module is PLAIN DATA: it names no ratatui type, so a watched path
# never arrives at the view already styled and every function in it is assertable without a
# frame. Same terms as MDSEAM's second leg and NOTABSEAM. Comment-inclusive on purpose: a
# doc comment naming a ratatui type is itself the coupling this forbids, so say "the view".
r=$(grep -nE 'ratatui|Modifier|Style|Span|Rect|Frame|Buffer' "$WATCH" || true)
[ -z "$r" ] || { echo "WATCHSEAM FAIL: $WATCH names a ratatui type:" >&2; echo "$r" >&2; exit 1; }

# Leg 3 — the module spawns no process. NOSPAWN-GREP already forbids this tree-wide; this is
# the file-scoped restatement, because src/watch.rs is the new file where reaching for an
# `fswatch` subprocess would be plausible.
p=$(grep -nE 'process::Command|Command::new|Stdio' "$WATCH" || true)
[ -z "$p" ] || { echo "WATCHSEAM FAIL: $WATCH spawns a process:" >&2; echo "$p" >&2; exit 1; }

echo "WATCHSEAM OK: $n files searched (>= $MIN), notify only in $WATCH, no ratatui type there, no spawn"
```

```sh
# NOBLOCK — NEW in live-refresh. The render path never blocks on anything but the terminal
# event source, and it never reads a clock. This is hazard 2 ("the watcher must not block the
# draw") and hazard 1 ("no timing-based assertion") made structural.
#
# Two legs, each with its own positive control, and each pattern verified against the real
# tree at planning time rather than assumed:
#
#  1. src/ui/driver.rs's PRODUCTION slice names no channel, thread, or blocking-wait API.
#     run_loop reaches the watcher and the worker only through the two trait objects
#     ui::driver::Live carries, whose every method is non-blocking by contract, so there is
#     no expression in that file that can wait on a worker.
#     `\.join\(\)` — with EMPTY parentheses — not a bare `\.join\(`: Path::join and
#     str::join both take an argument and are ordinary, correct code; a JoinHandle's join
#     takes none. A bare pattern would be a false red waiting to happen the first time the
#     loop built a path.
#  2. NO file under src/ui/ reads a clock, tests included. This one is deliberately
#     whole-file rather than production-only: a TEST that reads the clock is precisely the
#     timing flake this change exists not to reintroduce, and the debounce is a pure state
#     machine taking `now` as a PARAMETER, so no view test has any reason to call
#     Instant::now(). Measured at planning time: zero hits under src/ui on `main`.
#
#     BECAUSE IT IS COMMENT-INCLUSIVE, no doc comment under src/ui/ may spell a clock path
#     either. Verified at planning time: a comment reading "the crate's one `Instant::now()`
#     lives in `watch::RealFsEvents::drain`" — exactly the sentence tasks 9.4 and 13.2 push an
#     implementer toward — turns this leg RED on otherwise-correct code. Say "the clock" or "a
#     clock", never Instant::now(). Same rule MDSEAM imposes on src/ui/markdown.rs's prose
#     about ratatui, and NOTABSEAM shipped red on an unmodified tree in this repository once
#     already for exactly this reason. Prose without `::` is safe.
#
#  3. The two SEAM modules honour their own non-blocking contract. Legs 1 and 2 only ever look
#     under src/ui/, so a blocking `drain` or a blocking `take_result` — the two functions
#     run_loop calls on EVERY FRAME — are invisible to them. That is the gap this leg closes:
#     the traits' "every method is non-blocking" is a doc comment, and a doc comment is not a
#     check. A `drain` written `self.rx.recv_timeout(Duration::from_millis(150))` and a
#     `take_result` written `self.rx.recv_timeout(Duration::from_millis(400)).ok()` satisfy
#     NOCLI-SHELL, both legs above, NOSLEEP, WATCHSEAM, every trait signature, and every
#     frame-and-poll count this change asserts — while delaying each draw by half a second.
#     Guards D and E are what keep leg 3 from being dodged by moving code around rather than by
#     fixing it: D counts each seam module's line-anchored #[cfg(test)] attributes (a second one
#     above `start` hides the worker's whole body from prod()), and E requires `take_result` to
#     be declared above the spawn (below it, a blocking recv lands in the worker half and the
#     leg reports OK). Both measured against stub modules at planning time.
#
# #[cfg(test)] modules are stripped for legs 1 and 3 the way READONLY-UI strips them, because
# src/ui/driver.rs's own tests may legitimately name a channel in a double. Every src/ui/*.rs
# file holds at most ONE line-anchored #[cfg(test)] whose module runs to EOF — verified at
# planning time.
#
# That fact is NOT free for the two new modules, and Guard D below is why. src/refresh.rs is
# REQUIRED to hold a second #[cfg(test)] item, worker_for_test. prod() discards everything from
# the FIRST one to EOF, so a worker_for_test placed above start() hides the worker's entire body
# from this check and from READONLY-UI: measured, a std::fs::write in the worker's start path is
# then reported green, and this check's own spawn control false-reds on a correct tree. A COUNT
# is a check; a placement rule is a convention, so Guard D counts.
set -u
UIDIR="${UIDIR:-src/ui}"
UI_MIN="${UI_MIN:-11}"
fail() { echo "NOBLOCK FAIL: $1" >&2; exit 1; }

[ -d "$UIDIR" ] || fail "no such directory: $UIDIR"
[ -f "$UIDIR/driver.rs" ] || fail "$UIDIR/driver.rs missing - the subject is gone"
[ -f src/refresh.rs ] || fail "src/refresh.rs missing - the positive control has nothing to match"
[ -f src/watch.rs ] || fail "src/watch.rs missing - the positive control has nothing to match"

n=$(find "$UIDIR" -name '*.rs' | wc -l | tr -d ' ')
[ "$n" -ge "$UI_MIN" ] || fail "only $n files under $UIDIR (expected >= $UI_MIN)"

prod() { awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print}' "$1"; }

# Guard D — each seam module holds EXACTLY ONE line-anchored #[cfg(test)]. See the header.
for f in src/watch.rs src/refresh.rs; do
  c=$(grep -c '^#\[cfg(test)\]$' "$f" || true)
  [ "$c" -eq 1 ] || fail "$f holds $c line-anchored #[cfg(test)] attributes, expected exactly 1 - prod() truncates at the first and every sweep below it is silent"
done

BLOCK_RE='\.recv\(|recv_timeout|try_recv|\.join\(\)|JoinHandle|thread::spawn|thread::sleep|mpsc|Mutex|RwLock|Condvar'
# Leg 3's own pattern is wider than leg 1's: it must also see the ITERATOR forms, which block
# without naming `recv` at all. `self.rx.iter()` and `for r in &self.rx` are both blocking
# receives and both were measured to pass a recv-only pattern.
#
# It is anchored on a receiver-shaped binding (`…rx`) rather than on a bare `\.iter\(\)`:
# `Vec::iter()` is ordinary code that any of these modules may use, and a bare pattern would be
# a false red the first time the worker iterated a path list. `rx` is this crate's own naming
# convention for a Receiver, established by src/cli.rs's existing test.
BLOCK3_RE='\.recv\(|recv_timeout|\.join\(\)|park_timeout|thread::park|rx\.iter\(\)|for +[a-z_]+ +in +&?[a-z_.]*rx'

# Guard A — positive control for leg 1: src/refresh.rs's production slice MUST name a
# channel and a thread spawn, or the worker is not a worker and the pattern is broken.
prod src/refresh.rs | grep -qE 'mpsc' \
  || fail "positive control - src/refresh.rs's production slice names no mpsc"
prod src/refresh.rs | grep -qE 'thread::spawn' \
  || fail "positive control - src/refresh.rs's production slice does not spawn a thread"

h=$(prod "$UIDIR/driver.rs" | grep -nE "$BLOCK_RE" || true)
[ -z "$h" ] || { echo "NOBLOCK FAIL (leg 1): $UIDIR/driver.rs blocks or owns a thread:" >&2
                 echo "$h" >&2; exit 1; }
echo "NOBLOCK OK (leg 1): $UIDIR/driver.rs's production slice names no channel, thread, or blocking wait"

CLOCK_RE='Instant::now|SystemTime::now|\.elapsed\(\)|Instant::'

# Guard B — positive control for leg 2: src/watch.rs MUST read the clock, since it holds the
# crate's one binding from the debounce's injected `now` to the real one. If it does not,
# either the pattern is broken or the debounce grew a hidden clock somewhere else.
grep -qE "$CLOCK_RE" src/watch.rs \
  || fail "positive control - src/watch.rs names no clock; the one real clock binding is missing"

c=$(find "$UIDIR" -name '*.rs' -print0 | xargs -0 -I{} grep -nE "$CLOCK_RE" {} /dev/null 2>&1 || true)
[ -z "$c" ] || { echo "NOBLOCK FAIL (leg 2): a clock is read under $UIDIR:" >&2
                 echo "$c" >&2; exit 1; }
echo "NOBLOCK OK (leg 2): $n files under $UIDIR read no clock, tests included; both controls matched"

# Leg 3 — the two seam modules honour their own non-blocking contract.
# src/watch.rs may not block on a channel AT ALL: RealFsEvents::drain is a try_recv loop and
# nothing in that module runs off the render path.
w=$(prod src/watch.rs | grep -nE "$BLOCK3_RE" || true)
[ -z "$w" ] || { echo "NOBLOCK FAIL (leg 3): src/watch.rs blocks on a channel:" >&2
                 echo "$w" >&2; exit 1; }
prod src/watch.rs | grep -q 'try_recv' \
  || fail "positive control - src/watch.rs's production slice names no try_recv; drain is not draining"

# src/refresh.rs MAY block, but only inside the worker body, which is everything after the
# single thread::spawn in the production slice. Refresher::take_result is declared before it
# and must be try_recv.
# awk is used ONLY to CUT the file at its single thread::spawn; the matching is grep's, because
# an ERE passed to awk through -v is re-escaped as a string literal and awk rejects it
# ("illegal primary in regular expression") — measured, and it fails OPEN, printing to stderr
# while the leg reports OK. One tool, one job.
#
# The cut SKIPS COMMENT LINES. Measured: a module doc comment on line 1 reading "the worker is
# started by a single `thread::spawn` in `start`" — the natural sentence to write, and one
# SPEC.md is being edited to say — would otherwise cut at line 1 and leave nothing to search.
# `^[^/]*thread::spawn` requires the token to be preceded on its line by no `/`, which excludes
# `//`, `///`, and `//!`.
# Guard E — take_result is DECLARED BEFORE the spawn, so the textual half this leg searches is
# the render-path half. Without it the rule is a convention rather than a check: moving
# take_result below start() puts a blocking recv into the worker half and the leg reports OK.
# Measured. specs/refresh-worker states the ordering; this is what enforces it.
tr_line=$(prod src/refresh.rs | grep -n 'fn take_result' | head -1 | cut -d: -f1)
sp_line=$(prod src/refresh.rs | grep -nE '^[^/]*thread::spawn' | head -1 | cut -d: -f1)
[ -n "$tr_line" ] || fail "src/refresh.rs's production slice declares no take_result"
[ -n "$sp_line" ] || fail "src/refresh.rs's production slice has no thread::spawn outside a comment"
[ "$tr_line" -lt "$sp_line" ] \
  || fail "src/refresh.rs declares take_result at line $tr_line, below its thread::spawn at line $sp_line; leg 3 would then search the worker half instead of the render-path half"

head3=$(prod src/refresh.rs | awk '/^[^\/]*thread::spawn/{exit} {print}')
b=$(printf '%s\n' "$head3" | grep -nE "$BLOCK3_RE" || true)
[ -z "$b" ] || { echo "NOBLOCK FAIL (leg 3): src/refresh.rs blocks before its thread::spawn," >&2
                 echo "i.e. on the render path rather than in the worker:" >&2
                 echo "$b" >&2; exit 1; }
prod src/refresh.rs | grep -q 'try_recv' \
  || fail "positive control - src/refresh.rs's production slice names no try_recv; take_result blocks"

echo "NOBLOCK OK (leg 3): src/watch.rs never blocks; src/refresh.rs blocks only after thread::spawn"
```

**A known limit of `NOBLOCK` leg 1, stated rather than left as an overclaim.** Its patterns are
token-based, so a blocking receive reached through a type alias — `type Rx = …; for _ in
rx.iter() {}` — names no `mpsc`, no `.recv(`, and no `.join()`, and passes. Verified at
planning time. Leg 1's OK line therefore claims only what it checked; the trait contract and
leg 3 are what carry that case.

```sh
# NOSLEEP — NEW in live-refresh. Hazard 1, mechanised: no test in this crate sleeps a fixed
# interval and then asserts that something has already happened. Every sleep in the tree must
# sit inside a DEADLINE-BOUNDED POLL, which is the only shape that stays green on a cold or
# loaded machine.
#
# This is deliberately NOT "no sleep anywhere". Measured at planning time, `main` already
# carries THREE sleeps — src/cli.rs's `a_program_that_reads_stdin_returns_rather_than_blocking`
# and tests/cli.rs's two `try_wait` polls — and all three are correct: each is the pause
# inside a `while Instant::now() < deadline` loop, and tests/cli.rs:141 even carries the
# comment recording the flake that produced the rule ("Poll to a deadline rather than
# sleeping a fixed interval"). A blanket prohibition would be RED on the unmodified tree,
# and the only ways out would be to delete three correct tests or to exempt two files —
# which is how a check rots into a rubber stamp.
#
# `yield_now` is deliberately NOT in the pattern: it has no duration, so a loop around it is
# still a condition poll and cannot make an assertion premature. Group 6's real-watcher test
# uses it.
#
# Two legs, and a self-contained negative control that runs on EVERY invocation rather than
# only at plant time.
#   env: MIN  minimum number of .rs files searched (guards a vacuous search)
#        SLEEP_MIN  minimum number of sleep sites the scan must FIND (guards a scan that
#                   matched nothing and reported a clean tree)
set -u
MIN="${MIN:-20}"
SLEEP_MIN="${SLEEP_MIN:-3}"
[ -d src ] && [ -d tests ] || { echo "NOSLEEP FAIL: run from the crate root" >&2; exit 1; }
n=$(find src tests -name '*.rs' | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || { echo "NOSLEEP FAIL: searched only $n files (expected >= $MIN)" >&2; exit 1; }

MIN="$MIN" SLEEP_MIN="$SLEEP_MIN" python3 - <<'PY'
import os, re, subprocess, sys

SLEEP = re.compile(r"thread::sleep|sleep_ms|park_timeout|std::thread::park\b")
# A span is "deadline-bounded" when it names a deadline AND loops. Both halves matter:
# `deadline` alone could be a variable nothing loops on, and a bare `while` could spin
# forever.
DEADLINE = re.compile(r"\bdeadline\b")
LOOP = re.compile(r"\b(while|loop)\b")

def spans(text):
    """Split a Rust file at line-anchored #[test] attributes, the same splitter WIDTHS,
    LISTWIDTHS, MDWIDTHS, DETAILWIDTHS and TASKWIDTHS use. Index 0 is the pre-test
    prelude (production code); every later part is one test function PLUS EVERYTHING
    DEFINED AFTER IT up to the next #[test]. That limit is stated rather than discovered:
    a sleeping helper written below a correct deadline-bounded test inherits that test's
    verdict. Leg 2's absolute ban over src/ui is what closes it where this change's own
    tests live; elsewhere the limit stands and a reviewer is the backstop."""
    return re.split(r"(?m)^[ \t]*#\[test\][ \t]*$", text)

def offenders(path, text):
    out = []
    for i, part in enumerate(spans(text)):
        if not SLEEP.search(part):
            continue
        name = re.search(r"fn\s+([a-z0-9_]+)", part)
        name = name.group(1) if name else ("<prelude>" if i == 0 else "<unnamed>")
        ok = bool(DEADLINE.search(part)) and bool(LOOP.search(part))
        out.append((path, name, ok))
    return out

# --- the self-contained negative control, run on EVERY invocation ------------------
control_bad = "#[test]\nfn c() { std::thread::sleep(d); assert!(happened); }\n"
control_good = ("#[test]\nfn c() { let deadline = Instant::now() + d;\n"
                "  while Instant::now() < deadline { if done() { break } "
                "std::thread::sleep(d); } assert!(happened); }\n")
bad = offenders("<control>", control_bad)
good = offenders("<control>", control_good)
if not (len(bad) == 1 and bad[0][2] is False):
    print("NOSLEEP FAIL: the negative control was not reported - the scan is broken",
          file=sys.stderr); sys.exit(1)
if not (len(good) == 1 and good[0][2] is True):
    print("NOSLEEP FAIL: the positive control was reported - the scan rejects a correct poll",
          file=sys.stderr); sys.exit(1)

files = subprocess.run(["find", "src", "tests", "-name", "*.rs"],
                       capture_output=True, text=True, check=True).stdout.split()
found, bad_sites = [], []
for f in sorted(files):
    for path, name, ok in offenders(f, open(f).read()):
        found.append(f"{path}::{name}")
        if not ok:
            bad_sites.append(f"{path}::{name}")

floor = int(os.environ["SLEEP_MIN"])
if len(found) < floor:
    print(f"NOSLEEP FAIL: found {len(found)} sleep sites, expected >= {floor}. "
          "A scan that matches nothing reports a clean tree; if a sleep was legitimately "
          "removed, lower SLEEP_MIN deliberately in the invocation.", file=sys.stderr)
    sys.exit(1)
if bad_sites:
    print("NOSLEEP FAIL: sleep outside a deadline-bounded poll: " + ", ".join(bad_sites),
          file=sys.stderr)
    sys.exit(1)
print(f"NOSLEEP OK (leg 1): {len(found)} sleep sites, all inside a deadline-bounded poll: "
      + ", ".join(found))
PY
[ $? -eq 0 ] || exit 1

# --- leg 2: an ABSOLUTE prohibition where it is satisfiable. Every file of the render seam
# names no sleep at all: its tests drive scripted doubles and an injected `now`, so there is
# nothing to wait for.
#
# It deliberately does NOT cover src/watch.rs. That module's one real-watcher test polls
# `drain` to a deadline with a 10ms sleep between iterations — the shape tests/cli.rs already
# uses here and the shape leg 1 accepts. Banning it there would force a `yield_now` busy-spin
# that holds a core for the whole window and, on a loaded two-core runner, competes for CPU
# with the notify thread producing the event it waits for. A sleep INSIDE a deadline-bounded
# poll cannot make an assertion premature, because the condition is re-tested after it; that
# is the whole point, and leg 1 is what governs it.
ABS="src/ui"
missing=""
for p in $ABS; do [ -e "$p" ] || missing="$missing $p"; done
[ -z "$missing" ] || { echo "NOSLEEP FAIL (leg 2):$missing missing" >&2; exit 1; }
h=$(find $ABS -name '*.rs' -print0 2>/dev/null \
    | xargs -0 -I{} grep -nE 'thread::sleep|sleep_ms|park_timeout' {} /dev/null 2>&1 || true)
[ -z "$h" ] || { echo "NOSLEEP FAIL (leg 2): a sleep under $ABS:" >&2; echo "$h" >&2; exit 1; }
echo "NOSLEEP OK (leg 2): no sleep at all under $ABS"

# --- leg 2b: the two seam modules are NOT under leg 2, because src/watch.rs legitimately holds
# the one deadline-bounded 10ms poll. They are capped instead: AT MOST ONE sleep each. Leg 1's
# span splitter attributes a sleeping helper to the preceding test, so a 300ms settle() written
# below group 6's correct deadline-bounded test would inherit its verdict and pass — measured.
# The cap is the plan's own claim ("exactly one sleep, in the one real-watcher test") made
# checkable, and it costs nothing because no other test in either module waits for anything.
for f in src/watch.rs src/refresh.rs; do
  [ -f "$f" ] || { echo "NOSLEEP FAIL (leg 2b): $f missing" >&2; exit 1; }
  k=$(grep -c 'thread::sleep\|sleep_ms\|park_timeout' "$f" || true)
  [ "$k" -le 1 ] || { echo "NOSLEEP FAIL (leg 2b): $f names a sleep $k times; the seam modules get at most the one deadline-bounded poll" >&2; exit 1; }
done
echo "NOSLEEP OK (leg 2b): src/watch.rs and src/refresh.rs name at most one sleep each"
```

```sh
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
# This change WATCHES openspec/, which is what makes a stray write plausible: a watcher is one
# edit away from a marker file, a lock file, or a "last seen" stamp beside the tree it reads.
# Group 11's scripted every-printable-key run, over a real ScratchDir repository with a REAL
# notify watcher open on it, is the runtime half of the same claim. This grep is the source
# half, and it is the half that sees an UNTRACKED runtime write, which `git diff` alone cannot.
#
# Exactly ONE exclusion: this change's own artifact directory, a hand-edited planning
# document rather than a code-path write. Excluded BY NAME, never by a broad prefix, so a
# stray file anywhere else under openspec/ still fails. Carried forward from tasks-tab
# with that one path updated.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --ignored --exclude-standard -- ':(top)openspec/'; } \
         | grep -v '^openspec/changes/live-refresh/' | sort -u || true)
[ -z "$stray" ] || { echo "FAIL: wrote inside openspec/ outside this change:" >&2
                     echo "$stray" >&2; exit 1; }
echo "OPENSPEC-UNTOUCHED OK"
```

### The two edited-in-place blocks, specified to the character

`DEPS.sh` and `GRAPH-SNAP.sh` are extracted **byte-identically** from
`openspec/changes/archive/2026-09-05-markdown-viewer/tasks.md` in task 0.1, and edited in
task 6.2 by exact string replacement. The replacements are written out here so the edit is
reviewable before it is made; task 6.2 applies them, prints
`diff -u $WORK/<LABEL>.src $CHECKS/<LABEL>.sh`, and runs the result.

**`DEPS.sh`, edit 1** — leg 2a's `want` dict. Replace exactly:

```
want={"serde_json":["std"],
      "toml":["display","parse","serde","std"],
      "yaml-rust2":[],
      "ratatui":["crossterm"],
      "pulldown-cmark":[]}
```

with exactly:

```
want={"serde_json":["std"],
      "toml":["display","parse","serde","std"],
      "yaml-rust2":[],
      "ratatui":["crossterm"],
      "pulldown-cmark":[],
      "notify":["macos_fsevent"]}
```

**`DEPS.sh`, edit 2** — a new leg 2e, inserted immediately after leg 2d's closing
`' || { echo "DEPS FAIL: leg 2d" >&2; exit 1; }` line:

```sh
# --- leg 2e: NEW in live-refresh. notify's defaults are off in MANIFEST terms AND its one
# feature is named explicitly. Both halves are load-bearing and the second is not cosmetic:
# `default-features = false` ALONE does not compile on macOS — notify's fsevent module is
# gated on `not(feature = "macos_kqueue")`, not on the presence of `macos_fsevent`, so an
# empty feature list yields `error[E0432]: unresolved import fsevent_sys`. Measured, not
# assumed. The graph half of the same claim is GRAPH-SNAP's kqueue/kqueue-sys absences.
grep -q 'notify = .*default-features = false' Cargo.toml \
  || { echo "DEPS FAIL: leg 2e notify does not declare default-features = false" >&2; exit 1; }
grep -q 'notify = .*features = \["macos_fsevent"\]' Cargo.toml \
  || { echo "DEPS FAIL: leg 2e notify does not name macos_fsevent explicitly" >&2; exit 1; }
# No debouncer crate. The hand-rolled watch::Debounce is the decision (design.md -> Decisions
# 12) and this is its manifest-level proof: notify-debouncer-mini costs five further
# packages, tempfile among them, as NORMAL dependencies of the shipped binary.
if grep -qE 'notify-debouncer' Cargo.toml; then
  echo "DEPS FAIL: leg 2e a notify-debouncer crate is declared" >&2; exit 1
fi
echo "DEPS OK (leg 2e): notify declares defaults off and macos_fsevent; no debouncer crate"
```

**`DEPS.sh`, edit 3** — leg 5's experiment list. Replace exactly:

```
needed pulldown-cmark "ui::markdown"
```

with exactly:

```
needed pulldown-cmark "ui::markdown"
needed notify "watch::RealFsEvents"
```

**`GRAPH-SNAP.sh`, edit 1** — the macOS/Linux delta assertion. Replace exactly:

```
# The macOS pair and the Linux pair each agree; macOS and Linux differ by exactly
# linux-raw-sys. Asserted as a NAMED difference, because the two are no longer identical.
```

with exactly:

```
# The macOS pair and the Linux pair each agree; macOS and Linux differ by exactly FOUR named
# packages. Asserted as a NAMED difference, because the two are no longer identical.
# Before live-refresh the difference was linux-raw-sys alone, which rustix pulls in only on
# Linux. notify adds three more, all target-scoped in its own Cargo.toml: fsevent-sys on
# macOS (the FSEvents backend), inotify and inotify-sys on Linux. This is the one hard-coded
# string in the whole check, and this change is the first to make it false.
```

and replace exactly:

```
[ "$d" = "linux-raw-sys " ] \
  || { echo "GRAPH-SNAP FAIL: macOS/Linux differ by [$d], expected [linux-raw-sys ]" >&2; exit 1; }
```

with exactly:

```
[ "$d" = "fsevent-sys inotify inotify-sys linux-raw-sys " ] \
  || { echo "GRAPH-SNAP FAIL: macOS/Linux differ by [$d], expected [fsevent-sys inotify inotify-sys linux-raw-sys ]" >&2; exit 1; }
```

**`GRAPH-SNAP.sh`, edit 2** — the named-absence list. Replace exactly:

```
for absent in encoding_rs time getopts pulldown-cmark-escape; do
```

with exactly:

```
for absent in encoding_rs time getopts pulldown-cmark-escape kqueue kqueue-sys; do
```

and replace exactly:

```
echo "GRAPH-SNAP OK: four triples match the snapshot; proc-macro set exact; four named absences hold"
```

with exactly:

```
echo "GRAPH-SNAP OK: four triples match the snapshot; proc-macro set exact; six named absences hold"
```

**Extracted byte-identically rather than retyped** — **twenty-one** blocks, nineteen of which
never change; the remaining two (`DEPS.sh`, `GRAPH-SNAP.sh`) are extracted byte-identically
here and **then** edited in task 6.2, so task 0.2 diffs the extraction and task 6.2 diffs the
edit. From `openspec/changes/archive/2026-09-05-detail-view/tasks.md`: `NOSPAWN-GREP.sh`,
`READSEAM.sh`, `NOCLI-SHELL.sh`, `NODEFAULT-UI.sh` (its **edited**, brace-matching form —
that is the version on record and the one that sees a multi-line elision), `NOLIT-CHANGE.sh`,
`MDSEAM.sh`, `NOTABSEAM.sh`, `WIDTHS.sh`, and `DETAILWIDTHS.sh`. From
`openspec/changes/archive/2026-09-05-markdown-viewer/tasks.md`: `NORAW-GREP.sh`,
`LISTWIDTHS.sh`, `MDWIDTHS.sh`, `NOWAIVER.sh`, `TESTCOUNT.sh`, `DEPS.sh`, and
`GRAPH-SNAP.sh` (the last two are extracted byte-identically and **then** edited in task 6.2;
task 0.2 diffs the extraction, task 6.2 diffs the edit). From
`openspec/changes/archive/2026-09-05-tasks-tab/tasks.md`: `NOIO-VIEW.sh`, `TASKSEAM.sh`, and
`TASKWIDTHS.sh`. From `openspec/changes/archive/2026-09-04-changes-from-cli/tasks.md`:
`GATE-MECH1.py` and `NOJSON-SEAM.sh`.

Together: **five** blocks reproduced above (`WATCHSEAM`, `NOBLOCK`, `NOSLEEP`,
`READONLY-UI`, `OPENSPEC-UNTOUCHED`) and **twenty-one** extracted from the archives, for
**twenty-six** files in `$CHECKS` — twenty-five gates plus `TESTCOUNT.sh`, which is sourced
(`. $CHECKS/TESTCOUNT.sh`) rather than run.

**`GATE-MECH1.py`'s label is on its block's *second* line, behind a shebang.** An extractor
that reads only a block's first line silently drops it. Scan the first **two** lines for the
label, and treat a missing `$CHECKS/GATE-MECH1.py` at task 0.3 as an extraction defect rather
than as a missing check.

---

## 0. Baseline, checks, and the scratchpad
<!-- kind: operational -->

- [x] 0.1 CHECK: Record the starting state before any edit, by **measuring**, never by
      copying a number from this file.
      - `git rev-parse HEAD` → `export BASE=<sha>`. Every `OPENSPEC-UNTOUCHED` run uses it.
        A diff against the index would pass over this change's own per-group commits.
        **Measured at planning time: `f9b42e8d11ead35049297451996d77e70869bd2f`.** Note that
        this is **not** the commit the first planning pass measured: `f9b42e8`
        ("chore(openspec): put the environment rules in config.yaml instead of every prompt")
        landed on `main` while this package was being written, touching
        `openspec/config.yaml` and nothing else. That is why `BASE` is re-read from
        `git rev-parse HEAD` here rather than trusted from this file: with the older sha,
        `OPENSPEC-UNTOUCHED` reports `openspec/config.yaml` as a stray write on the very first
        run. It changed no source file and no test, so every measured count below stands. Prefer
        `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh` at each invocation over relying on an
        exported variable: the check fails closed in a fresh shell, but the natural recovery —
        re-exporting from the *current* `HEAD` — silently defeats it, since every commit this
        change makes would then be inside the baseline.
      - `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
        passed.*/\1/p' | awk '{t+=$1} END{print t+0}'` → the **lib** baseline.
        **Measured at planning time: 673**, inside a 694-test whole suite (673 lib + 16
        `tests/ci_workflow.rs` + 5 `tests/cli.rs`).
      - `find src -name '*.rs' ! -path 'src/cli.rs' | wc -l` → `NOSPAWN-GREP`'s starting file
        count. **Measured: 19**; the floor becomes **21**, because both new modules are
        excluded by none of these.
      - `find src -name '*.rs' ! -path 'src/changes.rs' | wc -l` → `NOLIT-CHANGE`'s starting
        count. **Measured: 19**; the floor becomes **21**.
      - `find src -name '*.rs' ! -path 'src/ui/markdown.rs' | wc -l` → `MDSEAM`'s starting
        count. **Measured: 19**; the floor becomes **21**.
      - `find src tests -name '*.rs' | wc -l` → `NOSLEEP`'s guard. **Measured: 22**; the floor
        becomes **24**.
      - `find src tests -name '*.rs' ! -path 'src/watch.rs' | wc -l` → `WATCHSEAM`'s guard.
        It will be **23** once `src/watch.rs` exists; the floor is 23.
      - `find src/ui -name '*.rs' | wc -l` → `NOCLI-SHELL`'s, `READONLY-UI`'s, and
        `NOBLOCK`'s `UI_MIN`. **Measured: 11**; all three floors stay **11**, because this
        change adds no file under `src/ui/`. That is the point of Decisions 1.
      - `find src/ui -name '*.rs' ! -path 'src/ui/mod.rs' | wc -l` → `READSEAM`'s `UI_MIN`.
        **Measured: 10**; the floor stays **10**.
      - `find src tests -name '*.rs' ! -path 'src/ui/terminal.rs' | wc -l` → `NORAW-GREP`'s
        guard. **Measured: 21** (its floor is 16 and does not move).
      - `grep -cE '^[ \t]*#\[test\][ \t]*$'` per file → `src/ui/view.rs` **76** (`WIDTHS`'
        floor becomes **81**); `src/ui/detail.rs` **23** (`DETAILWIDTHS` floor stays 23);
        `src/ui/list.rs` **17** (`LISTWIDTHS` floor becomes **20**); `src/ui/markdown.rs`
        **24** — its `MDWIDTHS` floor is **raised from tasks-tab's 23 to 24**, because a floor
        below the tree's own current value is already satisfied and therefore cannot fail;
        this change adds no markdown test, so 24 is exactly met and unmoved thereafter;
        `src/ui/tasks.rs` **16** (`TASKWIDTHS` floor stays **16**, exactly met); `src/ui/app.rs` **48**; `src/ui/driver.rs` **14**; `src/ui/mod.rs` **18**;
        `src/changes.rs` **185**; `src/ui/layout.rs` **15**. `src/watch.rs` and
        `src/refresh.rs` do not exist yet.
      - The `src/ui/mod.rs` submodule counts, which the whole-file 18 hides. Measure by
        running each filter: `'ui::tests::load::'` → **7**; `'ui::tests::detail::'` → **3**;
        `'ui::tests::start::'` → **6**; `'ui::tests::read_artifact::'` → **2**;
        `'ui::tests::live::'` → **0**, since the module does not exist.
      - `cargo test --all-features --lib 'changes::tests::'` → **180**, against 185 `#[test]`
        attributes in that file: five tests live in another submodule. The floor becomes 194
        and group 3's fourteen new tests must all land in `changes::tests::`.
      - `cargo test --all-features --lib 'ui::' …` → the `ui::` subtotal.
        **Measured: 260.** Also `'ui::app::tests::'` **48**, `'ui::driver::tests::'` **14**,
        `'ui::view::tests::'` **76**, `'ui::list::tests::'` **17**.
      - `NODEFAULT-UI`'s half B, run with `SCAN_MIN=1`, prints the number of literal/pattern
        spans it scanned. **Measured at planning time: 107 spans, 0 elisions**; the invocation
        below raises `SCAN_MIN` from tasks-tab's 80 to **90**, comfortably below 107 with room
        for a span-reducing refactor, and far above zero.
      - `grep -rn 'Dashboard[[:space:]]*{' src/ | wc -l` → **70 grep hits**, spread over
        `src/lib.rs` (2), `src/ui/app.rs` (30), `src/ui/driver.rs` (9), `src/ui/list.rs` (6),
        `src/ui/mod.rs` (7), and `src/ui/view.rs` (16). **The grep is orientation only. The
        gate for task 1.3 is the compiler (`E0063`)**, which cannot be fooled by the struct
        definition, a doc comment, or a `&Dashboard {` pattern — an earlier plan in this
        repository quoted a grep count as a construction count and was wrong by three.
      - `grep -rn 'run_loop(' src/ | wc -l` → **19 hits, 18 of them call sites** (the
        definition in `src/ui/driver.rs` is not a call; 13 calls in that file's tests and 5 in
        `src/ui/mod.rs`, one of them `ui::run`'s production call). All eighteen move in group 1.
      - `cargo llvm-cov --summary-only` → TOTAL **line** coverage. The TOTAL row leads with
        the **region** count; read the line column, further right. **Measured at planning
        time: 97.52% over 16,151 lines** (regions 97.12%, functions 96.90% — do not quote
        either of those as the gated figure).
      - `export CHECKS=<scratchpad>/live-refresh-checks` and
        `export WORK=<scratchpad>/live-refresh-work`; `mkdir -p "$CHECKS" "$WORK"`.
        Extract every fenced block above to `$CHECKS/<LABEL>.sh` byte-identically — five of
        them — and extract the twenty-one named archived blocks to their own files, scanning
        each block's first **two** lines for its label so `GATE-MECH1.py`'s shebang does not
        hide it.

- [x] 0.2 CHECK: Prove every **extracted** block is byte-identical to its source. For each of
      the twenty-one, re-extract the source block into `$WORK/<LABEL>.src` by the same means
      and `diff -u "$WORK/<LABEL>.src" "$CHECKS/<LABEL>.sh"` — an empty diff for all
      twenty-one, or stop. Keep `$WORK/DEPS.src` and `$WORK/GRAPH-SNAP.src`: task 6.2 diffs its edits against
      them. An extraction that silently produced the wrong bytes would otherwise pass a check
      that is not the one anybody reviewed. Record the twenty-one diff results.

- [x] 0.3 CHECK: Run every check against **unmodified `main`** and record each result
      verbatim, so the change starts from a known state rather than an assumed one.
      - Expected **green** at their current floors, each printing its OK line:
        `MIN=19 sh $CHECKS/NOSPAWN-GREP.sh`; `UI_MIN=10 sh $CHECKS/READSEAM.sh`;
        `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`;
        `TYPES="Dashboard Filter Detail" SCAN_MIN=90 sh $CHECKS/NODEFAULT-UI.sh`;
        `MIN=19 sh $CHECKS/NOLIT-CHANGE.sh`; `MIN=19 sh $CHECKS/MDSEAM.sh`;
        `sh $CHECKS/NOTABSEAM.sh`; `sh $CHECKS/TASKSEAM.sh`; `sh $CHECKS/NOIO-VIEW.sh`;
        `sh $CHECKS/NORAW-GREP.sh`; `WIDTHS_MIN=76 sh $CHECKS/WIDTHS.sh`;
        `LIST_MIN=17 sh $CHECKS/LISTWIDTHS.sh`; `MD_MIN=24 sh $CHECKS/MDWIDTHS.sh`;
        `DETAIL_MIN=23 sh $CHECKS/DETAILWIDTHS.sh`; `TASK_MIN=16 sh $CHECKS/TASKWIDTHS.sh`;
        `sh $CHECKS/NOWAIVER.sh`; `sh $CHECKS/NOJSON-SEAM.sh`;
        `python3 $CHECKS/GATE-MECH1.py src`; `UI_MIN=11 sh $CHECKS/READONLY-UI.sh` (with
        `EXTRA` unset); `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`.
        **Verified green at planning time, all twenty, on `f9b42e8`.**
      - Expected green on `main` in their **unedited** form, run here to record that:
        `WORK=$WORK sh $CHECKS/DEPS.sh` — `WORK` is **required** and the script's own
        `: "${WORK:?…}"` guard fails without it — and `sh $CHECKS/GRAPH-SNAP.sh`. Task 6.2
        edits both and task 6.6 re-runs the edited pair.
      - Expected **green on `main` as it stands**, because `NOSLEEP` leg 2 covers `src/ui/`
        alone: `MIN=22 SLEEP_MIN=3 sh $CHECKS/NOSLEEP.sh`, whose leg-1 line must list exactly
        the **three** measured sleep sites by name. Run it **unpiped**: `sh … | head` reports
        the pipe's status, not the script's, and would hide a failure.
      - Expected **red, with the exact message named**, because the modules do not exist yet:
        `MIN=23 sh $CHECKS/WATCHSEAM.sh` → `WATCHSEAM FAIL: src/watch.rs missing - the
        exclusion has nothing to exclude`; `UI_MIN=11 sh $CHECKS/NOBLOCK.sh` → `NOBLOCK FAIL:
        src/refresh.rs missing - the positive control has nothing to match`;
        `UI_MIN=11 EXTRA="src/watch.rs src/refresh.rs" sh $CHECKS/READONLY-UI.sh` →
        `READONLY-UI FAIL: EXTRA file src/watch.rs missing`. **All three verified at planning
        time.** Record the three messages. **A different failure message from any of the three
        is a defect in the block, not the expected guard**, and must be fixed here rather than
        in the group that first runs it green.
      - Red-when: any check in the first list fails on unmodified `main`. That is a defect in
        the extraction or the invocation, and it is fixed here — `NOTABSEAM` shipped red on an
        unmodified tree once already.

- [x] 0.4 CHECK: Prove each of the three NEW checks and the two EDITED checks can actually see
      what it guards, by planting a violation, running the check, and reverting. Each plant is
      a working-tree edit reverted immediately; `git status --porcelain src/` must be empty
      after each revert. Every plant below was **run at planning time** with the result stated.
      - `NOSLEEP` leg 1 (**runnable now**): add a `#[test] fn planted_sleep()` to
        `src/ui/layout.rs`'s test module whose body is
        `std::thread::sleep(Duration::from_millis(200)); assert!(true);`. Expect
        `NOSLEEP FAIL: sleep outside a deadline-bounded poll: src/ui/layout.rs::planted_sleep`.
        **Verified: red on the plant, green again after the revert.** Note the invocation
        form: run the script **unpiped**, since `sh … | head` reports the pipe's status, not
        the script's.
      - `NOSLEEP` leg 1's **known limit**, planted so it is a recorded fact rather than a
        surprise: put a sleeping helper `fn wait_a_bit() { thread::sleep(200ms) }` in
        `src/changes.rs` **after** a correct deadline-bounded `#[test]`, and confirm the check
        reports **OK**, attributing the sleep to the preceding test. A span is one test plus
        everything defined after it. Record the result; do not "fix" it by widening leg 2,
        which is what forced a busy-spin in an earlier draft of this plan.
      - `NOSLEEP`'s self-controls (**runnable now, and run on every invocation**): they are
        inside the script, so no plant is needed — but confirm the script fails when its own
        `SLEEP` regex is broken, by running a copy with `SLEEP` set to a pattern matching
        nothing. Expect `NOSLEEP FAIL: the negative control was not reported - the scan is
        broken`.
      - `WATCHSEAM` (deferred to **6.6**): add `// use notify::Event;` to `src/ui/driver.rs`.
        Expect `WATCHSEAM FAIL: the watch crate is named outside src/watch.rs` naming that
        file and line. **Verified at planning time against stub modules.**
      - `WATCHSEAM` ratatui leg (deferred to **6.6**): add `// a ratatui Style` as a comment
        in `src/watch.rs`. Expect FAIL — the comment-inclusive sweep is the point, and this
        plant is what proves the module's doc comments must say "the view".
        **Verified at planning time.**
      - `NOBLOCK` leg 1 (deferred to **7.5**): add `use std::sync::mpsc;` above `pub const
        TICK` in `src/ui/driver.rs`. Expect `NOBLOCK FAIL (leg 1)` naming that line.
        **Verified at planning time.**
      - `NOBLOCK` leg 2 (deferred to **7.5**): add `// let _ = std::time::Instant::now();` to
        `src/ui/app.rs`. Expect `NOBLOCK FAIL (leg 2): a clock is read under src/ui`.
        **Verified at planning time**, and so is the corollary that makes it a hazard rather
        than a nicety: a doc comment in `src/ui/driver.rs` reading "the crate's one
        `Instant::now()` lives in `watch::RealFsEvents::drain`" — the sentence tasks 9.4 and
        13.2 push toward — is **also** red. Prose without `::` is safe; say "the clock".
      - `NOBLOCK` leg 3, watch half (deferred to **6.6**): add
        `let _ = rx.recv_timeout(d);` inside `src/watch.rs`'s **production** slice. Expect
        `NOBLOCK FAIL (leg 3): src/watch.rs blocks on a channel`. Also delete `try_recv` from
        that slice and expect `NOBLOCK FAIL: positive control - src/watch.rs's production
        slice names no try_recv`. **Both verified at planning time against stub modules.**
      - `NOBLOCK` leg 3, refresh half (deferred to **7.5**): add
        `let _ = rx.recv_timeout(d);` **before** `src/refresh.rs`'s `thread::spawn` and expect
        `NOBLOCK FAIL (leg 3): src/refresh.rs blocks before its thread::spawn`; then move the
        same call **after** the spawn and expect **OK**, which is the deliberate exemption for
        the worker body rather than a miss. **Both verified at planning time.**
      - `READONLY-UI`'s EXTRA leg (deferred to **1.10**, the first task at which both
        modules exist): add `fn w() { let _ = std::fs::write("x","y"); }` to `src/watch.rs`'s
        production code. Expect FAIL naming `src/watch.rs` and the line. **Verified at
        planning time against a stub module.**
      - `READONLY-UI`, second plant (**runnable now**): add `std::fs::remove_file(p)` inside a
        `#[cfg(test)]` module of `src/ui/view.rs`. Expect **OK** — the stripper is supposed to
        ignore it — and record that this is the deliberate exemption, not a miss.
      - `NODEFAULT-UI` with a fourth type (deferred to **1.10**): add `#[derive(Default)]`
        above `struct Refresh`. Expect `NODEFAULT-UI FAIL: Refresh has a Default`.
        **Verified at planning time against a stub struct: the script's `for T in $TYPES` loop
        accepts a fourth name and its positive control requires `struct Refresh` to exist.**
      - `NOLIT-CHANGE` (deferred to **1.10**): add a `Change { … }` literal to
        `src/refresh.rs`. Expect FAIL naming that file — the new module is inside the searched
        set.
      - `GRAPH-SNAP`'s edited delta assertion (deferred to **6.6**): after the edit and the
        snapshot regeneration, revert **only** the four-name string to `linux-raw-sys ` and
        expect `GRAPH-SNAP FAIL: macOS/Linux differ by [fsevent-sys inotify inotify-sys
        linux-raw-sys ], expected [linux-raw-sys ]`, then restore. This is the control that
        proves the edit is load-bearing rather than decorative.
      - `DEPS` leg 2e (deferred to **6.6**): change `features = ["macos_fsevent"]` to
        `features = []` in `Cargo.toml`. Expect `DEPS FAIL: leg 2e notify does not name
        macos_fsevent explicitly`, then restore. On macOS this plant **also** breaks the
        build, which is itself the measured evidence for Decisions 13.

- [x] 0.5 VERIFY: `git status --porcelain` shows only `openspec/changes/live-refresh/`; the
      four gate commands from the preamble all pass on unmodified `main`; and
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh` prints OK. Commit the artifacts.

---

## 1. Plumbing the acceptance test needs
<!-- kind: operational -->

A field, a variant, two inert modules, a widened signature, and two test doubles. No rule and
no behaviour: `Action::Refresh` gets a `=> {}` arm and **no** `action_for` mapping,
`watch::none()` and `refresh::none()` return implementations that answer nothing, and
`run_loop` receives a `Live` it does not yet read. Nothing observable changes and there is no
honest RED to manufacture. The evidence is that the crate compiles, every existing test still
passes, and `GATE-MECH1` still holds.

- [x] 1.1 CHECK: `python3 $CHECKS/GATE-MECH1.py src`, `cargo test --all-features --lib
      'ui::app::tests::'`, and `cargo test --all-features --lib 'ui::driver::tests::'` all
      green before any edit, so the "no regression" claim below has a baseline.

- [x] 1.2 CHANGE: Add `pub struct Refresh { pub requested: bool, pub reload: bool, pub
      problems: Vec<String> }` to `src/ui/app.rs`, beside `Filter` and `Detail`. Do **not**
      derive or implement `Default` for it: `NODEFAULT-UI` forbids it, and the compile error
      at each construction site is the mechanism that keeps the nine-field `Dashboard` from
      being half-adopted. Its doc comment states what each field means, not how it is
      computed, and — per `NOBLOCK` leg 2 — names no clock type.

- [x] 1.3 CHANGE: Add `pub refresh: Refresh` to `Dashboard`, after `detail`. Name it at every
      construction site. Do **not** reach for `..Default::default()` or a `..base` rest.
      Red-when: the crate compiles without every site naming the field — that would mean a
      `Default` or a `..` slipped in. **The gate is `E0063`, not the grep**: the 70 grep hits
      recorded in task 0.1 include the struct definition, doc comments, and `&Dashboard {`
      patterns, and are orientation only.

- [x] 1.4 CHANGE: Add `Refresh` to `ui::app::Action` as the thirteenth variant, with a
      `Action::Refresh => {}` arm in `Dashboard::apply` and **no** arm in `action_for` — the
      key mapping and the flag are group 8's behaviour. Extend
      `ui::app::tests::no_action_mutates_changes`'s exhaustive array to thirteen and its count
      assertion from twelve to thirteen; it must stay green, since a no-op arm mutates
      nothing.

- [x] 1.5 CHANGE: Create `src/watch.rs` and `src/refresh.rs`, declare `pub mod watch;` and
      `pub mod refresh;` in `src/lib.rs`, and populate them with the **inert** halves only:
      - `src/watch.rs`: `pub struct WatchError(pub String)` with a `Display`, the `FsEvents`
        trait, `pub fn none() -> Box<dyn FsEvents>` returning an implementation whose `drain`
        is `Ok(None)` and whose `pending_in` is `None`, and `pub fn poll_timeout(tick:
        Duration, pending_in: Option<Duration>) -> Duration` returning `tick`
        unconditionally — a stub, not the rule. Its doc comments name no `ratatui` type and no
        clock: `WATCHSEAM` and `NOBLOCK` are whole-file greps with comments deliberately
        included, and this repository has shipped a comment-triggered red before.
      - `src/refresh.rs`: `pub enum RefreshResult { Files(ChangeSet), Merged(ChangeSet) }`,
        the `Refresher` trait, and `pub fn none() -> Box<dyn Refresher>` returning an
        implementation that records nothing and answers `None`. It names **no**
        `OpenspecCli` and no thread yet. Its doc comments say "the worker thread" and never
        spell `thread::spawn`: `NOBLOCK` leg 3 arms its "everything after this is the worker"
        cut on that token, and while the cut ignores comment lines, a second defence costs
        nothing and the same rule already applies to `src/watch.rs`'s prose about `ratatui`
        and a clock. Both files hold **exactly one** line-anchored `#[cfg(test)]` — Guard D
        in `NOBLOCK` and `READONLY-UI` counts them, because `prod()` truncates at the first.
      - `src/changes.rs`: `pub enum Selection { All, Only(BTreeSet<String>) }`, with no
        `union` — group 3's behaviour.
      Red-when: `src/refresh.rs` names `CliChanges` in a public type. `NOCLI-SHELL` does not
      search that file, but `RefreshResult` reaching `src/ui/driver.rs` while carrying a
      `CliChanges` would make that check red in `driver.rs`, and the fix would be to weaken
      it. `RefreshResult` carries `ChangeSet`.

- [x] 1.6 CHANGE: Add `pub struct Live<'a> { pub fs: &'a mut dyn crate::watch::FsEvents, pub
      refresher: &'a mut dyn crate::refresh::Refresher }` to `src/ui/driver.rs` and widen
      `run_loop` to `(terminal, dashboard, events, live, read, tick)` — `live` fourth, so
      `read` and `tick` keep their positions. The body **ignores** `live` for now. Update all
      **eighteen** call sites: the thirteen in `src/ui/driver.rs`'s tests and the four in
      `src/ui/mod.rs`'s tests pass `Live { fs: &mut *watch::none(), refresher: &mut
      *refresh::none() }`; `ui::run` passes the same for now.
      Red-when: `run_loop` grows a seventh parameter. Six is below clippy's
      `too_many_arguments` threshold; a seventh would trip `-D warnings` and the fix would be
      an `#[allow]`, which is how a lint gets waived rather than satisfied.

- [x] 1.7 CHANGE: Add the two `#[cfg(test)]` doubles to `crate::testutil`, beside `Script` and
      `RecordingReader`, both **synchronous and thread-free**, both recording into a
      `RefCell`:
      - `ScriptedFs` — a queue of `Result<Option<Vec<PathBuf>>, WatchError>` for `drain`, a
        queue of `Option<Duration>` for `pending_in`, and `drains()` / `pendings()` accessors.
        Exhausting either queue yields `Ok(None)` / `None` forever rather than panicking: an
        `FsEvents` that errors once the script runs out would end tests for the wrong reason.
      - `RecordingRefresher` — a queue of `Option<RefreshResult>` for `take_result`, plus
        `requests() -> Vec<Selection>` and `takes() -> usize`. It is what makes
        "exactly one request, carrying `Selection::All`" assertable at all.
      Red-when: either double spawns a thread, sleeps, or reads a clock. Neither `NOSLEEP`
      leg 2 nor `NOBLOCK` leg 2 searches `src/lib.rs`, so this one is **not** mechanically
      caught — it is enforced by review, and task 12.1's reviewer is pointed at it explicitly.
      Stating the gap is the point: a Red-when that claims a check it does not have is worse
      than none.

- [x] 1.8 CHANGE: `ui::load` sets `refresh: Refresh { requested: true, reload: false,
      problems: Vec::new() }` in **both** its `Found` and `NotFound` arms. Extend
      `ui::tests::load::a_scratch_repository_is_loaded_from_files` and
      `no_repository_above_the_start` with the `refresh` assertion. Both stay green.

- [x] 1.9 CHECK: Contract gate — re-read `design.md` → Contracts and confirm the signatures on
      disk match it exactly: `Refresh`'s three fields, `Dashboard`'s nine, `Detail`'s
      **five** (unchanged), `Action`'s thirteen, `run_loop`'s six parameters, `Live`'s two,
      `FsEvents`' two methods, `Refresher`'s two, and `RefreshResult`'s two variants. Confirm
      `Change`'s seven fields and `ChangeSet`'s three are untouched, so `change-model`'s
      two-producer gate is unmoved.

- [x] 1.10 CHECK: The checks that arm at this group, and their deferred plants:
      `MIN=21 sh $CHECKS/NOSPAWN-GREP.sh`; `MIN=21 sh $CHECKS/NOLIT-CHANGE.sh` plus its
      planted `Change { … }` in `src/refresh.rs` (expect FAIL naming that file, then revert);
      `MIN=21 sh $CHECKS/MDSEAM.sh`; `MIN=24 SLEEP_MIN=3 sh $CHECKS/NOSLEEP.sh` — still
      three sites and still green, at the raised **file** floor of 24 now that two modules
      exist; `TYPES="Dashboard Filter Detail Refresh" SCAN_MIN=90 sh
      $CHECKS/NODEFAULT-UI.sh` plus its planted `#[derive(Default)]` above `struct Refresh`
      (expect FAIL, then revert);
      `UI_MIN=11 EXTRA="src/watch.rs src/refresh.rs" sh $CHECKS/READONLY-UI.sh` — green with
      the extended set for the first time — plus its planted
      `fn w() { let _ = std::fs::write("x","y"); }` in `src/watch.rs`'s production code
      (expect FAIL naming that file, then revert).
      `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh` and `UI_MIN=10 sh $CHECKS/READSEAM.sh` at their
      **unchanged** floors — that they do not move is this group's evidence that nothing was
      added under `src/ui/`. `git status --porcelain src/` empty after every revert.

- [x] 1.11 VERIFY: `cargo fmt --all -- --check`;
      `cargo clippy --all-targets --all-features -- -D warnings`;
      `cargo test --all-features` — **fully green**, this being the last boundary before the
      acceptance test goes red; `python3 $CHECKS/GATE-MECH1.py src`;
      `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::app::tests::' 48`,
      `testcount --lib 'ui::driver::tests::' 14`, and `testcount --lib 'changes::tests::' 180`
      — all unchanged, because this group adds no test.
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 2. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [x] 2.1 Set up the harness and the replaced collaborators named in design.md → Test
      Boundaries: a `crate::testutil::ScratchDir` repository holding `openspec/config.yaml`
      (`schema: tdd`), `openspec/schemas/tdd/schema.yaml` declaring the five artifacts with
      `apply.tracks: tasks.md`, and one change `alpha` whose `tasks.md` counts **4 of 9**.
      Drive it through `ui::load`, the **real** `ui::read_artifact`, a scripted `EventSource`,
      a `RecordingRefresher`, a `ScriptedFs`, and a `TestBackend`. Nothing else is real: no
      `openspec` binary, no `herdr` socket, no terminal, no thread, and no clock.

- [x] 2.2 RED: Write `ui::tests::live::files_paint_then_the_cli_corrects` — the failing
      end-to-end test for **live-updates → "A full live run leaves the change tree
      byte-identical"** and **"A result is adopted before the frame that shows it"**, carrying
      the dual-source rendering assertion with it. At 120x20 and again at 60x20, in two stages
      over one `Dashboard` and one `TestBackend` (the two-stage shape `tasks-tab` established,
      because a `TestBackend` keeps only its last buffer and the claim is about two different
      frames):
      - **Stage 1** — a `RecordingRefresher` whose `take_result` is always `None`, and a
        script of one `Char('q')`. Assert `LoopSummary { frames: 1, polls: 1 }`; assert the
        list row for `alpha` ends `[4/9]` — **files painted, before any CLI result existed**;
        assert `refresher.requests() == vec![Selection::All]` — the startup request went out;
        assert `dashboard.refresh.requested` is now false.
      - **Stage 2** — `dashboard.quit = false`, a `RecordingRefresher` scripted with
        `Some(Files(set@4/9))` then `Some(Merged(set@7/9))`, and a script of two `Ok(None)`
        timeouts then `Char('q')`. Assert the final list row for `alpha` ends `[7/9]` — **the
        CLI corrected it**; assert `refresher.takes() == 3`, one per iteration.
      - **Both stages** — a `testutil::snapshot` taken before stage 1 and after stage 2 is
        byte-identical, and a discriminating control rewrites one byte of `alpha`'s `tasks.md`
        between two further snapshots and asserts they **differ**.

- [x] 2.3 Confirm it fails **because the behaviour is missing**, not because the harness is
      misconfigured: the failure must be stage 1's `requests()` assertion — the loop does not
      read `refresh.requested` yet, so no request is made — and the snapshot and progress
      assertions must already **pass**, since nothing writes today. Record the exact assertion
      message; every intermediate boundary from here to group 11 must reproduce it verbatim,
      and a *different* message means the harness broke rather than the behaviour arriving.

- [x] 2.4 REFACTOR: None is possible while the test is red; state that explicitly here rather
      than leaving the lifecycle step unaccounted for. The harness cleanup happens at task
      11.2, once the test is green and a refactor can be shown not to change its result.

- [x] 2.5 VERIFY: the four gate commands from the preamble. `cargo test --all-features` fails
      on exactly one test, `ui::tests::live::files_paint_then_the_cli_corrects`.
      `cargo llvm-cov --ignore-run-fail --fail-under-lines 80` passes.
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 3. Per-change CLI invalidation
<!-- kind: behavior -->

- [x] 3.1 RED: Write fourteen failing tests in `changes::tests::`, named from their scenarios:
      `selection_union_all_absorbs_only`, `selection_union_of_two_onlys`,
      `only_reruns_the_named_change`, `only_still_runs_an_uncached_change`,
      `cached_progress_comes_from_the_fresh_list`, `an_unlisted_cached_change_is_evicted`,
      `cached_change_keeps_its_artifacts_and_schema`, `cached_change_keeps_its_problems`,
      `a_list_failure_leaves_the_cache_untouched`,
      `garbage_apply_payload_leaves_the_others_intact`,
      `a_rejected_schema_is_cached_like_any_other`, `all_reruns_every_change`,
      `from_cli_is_from_cli_cached_with_all`, and `from_cli_cached_writes_nothing`.
      `a_rejected_schema_is_cached_like_any_other` covers the mandatory boundary case
      `openspec/config.yaml` names ("a schema the CLI rejects") and which this change makes
      newly load-bearing: `SPEC.md` → Degraded states records that `learning-tool` declares
      `outside-in-tdd`, which the installed CLI rejects, and the new cache has to decide
      whether such a change is re-asked about every cycle forever or cached like any other.
      It is cached, and `r` is the way out.
      Red-when: `only_reruns_the_named_change` asserts `!calls.contains(beta)` rather than an
      **exact equality** on the recorded argument vector. A `!contains` passes when nothing
      ran at all, and an assertion that everything reloaded would pass either way and prove
      nothing — this is the test the whole per-change claim rests on.
      Second Red-when: `only_still_runs_an_uncached_change` is omitted. Without it a reader
      concludes `Only` means "never call the others", which is false on a cold cache and is
      the reason a first cycle is never degraded by an unlucky selection.

- [x] 3.2 GREEN: Implement `Selection::union`, `CliCache` (with `Default`, which clippy's
      `new_without_default` would demand anyway and which `change-model`'s gate does not
      cover — recorded in design.md → Decisions 16 so it is not read as a breach), and
      `changes::from_cli_cached` per `specs/refresh-worker/spec.md`. Then **redefine**
      `from_cli` as `from_cli_cached(cli, repo, &Selection::All, &mut CliCache::default())`.
      Red-when: `from_cli`'s body is duplicated rather than delegated. Two CLI producers is
      the divergence `change-model`'s two-producer gate exists to prevent, one level up, and
      the evidence that it did not happen is that **every landed `cli-changes` and
      `schema-cli-fallback` test stays green with no edit**.
      Second Red-when: a cached change's `progress` comes from the cache. It comes from the
      fresh `list --json` entry, which is what makes a mis-classified path harmless.

- [x] 3.3 CHECK: `changes::merge` and `join_artifacts` need **no** change — they operate on
      whole `Change` values and a cached one is built at the same construction site by the
      same rule. Confirm by reading them, and confirm every landed `change-merge` test passes
      untouched. If a change turns out to be needed, that is a divergence from `design.md` →
      Decisions 7 and must be recorded there before it is made.

- [x] 3.4 REFACTOR: If `from_cli_cached`'s per-change loop ended up with two near-copies of
      the "build a `Change`" expression — one fresh, one from cache — collapse them onto one
      helper, so `cli-changes`' "every one of `Change`'s seven fields comes from CLI data"
      keeps exactly one construction site. Otherwise state that no refactor was needed.

- [x] 3.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'changes::tests::' 194`;
      `python3 $CHECKS/GATE-MECH1.py src`; `MIN=21 sh $CHECKS/NOLIT-CHANGE.sh`;
      `sh $CHECKS/NOJSON-SEAM.sh`; `MIN=21 sh $CHECKS/NOSPAWN-GREP.sh`; the four gate
      commands, still failing on exactly the one known acceptance test with the recorded
      message. `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 4. Classifying a touched path
<!-- kind: behavior -->

- [x] 4.1 RED: Write eleven failing tests in `watch::tests::`, named from their scenarios:
      `a_change_file_invalidates_only_that_change`,
      `a_nested_change_file_invalidates_only_that_change`,
      `the_change_directory_itself_is_a_repository_touch`,
      `the_changes_directory_is_a_repository_touch`,
      `archive_and_schema_are_repository_touches`,
      `a_path_outside_the_repository_is_outside`,
      `a_relative_path_is_outside`, `an_empty_path_is_outside`,
      `a_mixed_batch_unions_its_classifications`,
      `a_repository_path_in_a_batch_absorbs_the_rest`, and
      `a_batch_of_only_outside_paths_is_an_empty_only`.
      Red-when: `a_change_file_invalidates_only_that_change` asserts only that `alpha` is in
      the result. It must assert the **whole set** is exactly `{"alpha"}` against a
      repository whose changes are `alpha` **and** `beta`, or it proves nothing about `beta`.
      Second Red-when: `a_batch_of_only_outside_paths_is_an_empty_only` asserts
      `Selection::All`. It is `Only(∅)` — an empty batch must not escalate to a full CLI
      reload, and the two are different values with different costs.

- [x] 4.2 GREEN: Implement `watch::Touch`, `watch::classify`, and `watch::invalidate` per
      `specs/watch-invalidation/spec.md`. Both are pure: no filesystem, no `canonicalize`, no
      clock, so a path that no longer exists classifies exactly as one that does.
      Red-when: the classification calls `Path::canonicalize`. It would turn a pure function
      into a filesystem edge, make a removed file classify differently from a present one, and
      break the `openspec/`-was-deleted degraded path.
      Second Red-when: an unrecognised path under `openspec/` maps to `Outside`. It maps to
      `Repository` — a wrong `Repository` costs one `list --json` on a cycle that was
      happening anyway; a wrong `Outside` silently stops the pane updating.

- [x] 4.3 REFACTOR: If `classify` reads as a chain of `starts_with` calls rather than as one
      relative-path match, restructure it around `Path::strip_prefix` and a component walk;
      otherwise state that no refactor was needed.

- [x] 4.4 CHECK: `sh $CHECKS/NOIO-VIEW.sh` — unchanged and green: the classifier lives in
      `src/watch.rs`, outside `src/ui/`, which is what keeps the pure set at eight.
      `MIN=21 sh $CHECKS/NOSPAWN-GREP.sh`. `MIN=24 SLEEP_MIN=3 sh $CHECKS/NOSLEEP.sh`.

- [x] 4.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'watch::tests::' 11`; the four
      gate commands, still failing on exactly the one known acceptance test.
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 5. The debounce, with `now` as a parameter
<!-- kind: behavior -->

**This group is where hazard 1 is answered, and it is answered by a signature rather than by
a convention.** `Debounce::push(paths, now)`, `take_due(now)`, and `pending_in(now)` each take
the instant as an argument. Every test below captures `Instant::now()` **once**, as `t0`, and
asserts against `t0 + Duration::from_millis(149)` and `t0 + Duration::from_millis(150)`. No
test in this group may call `Instant::now()` more than once, sleep, or wait.

- [x] 5.1 RED: Write eleven failing tests in `watch::tests::`, named from their scenarios:
      `nothing_is_due_before_the_window`, `the_batch_is_due_at_the_window`,
      `taking_a_due_batch_empties_the_state`, `a_later_push_extends_the_window`,
      `pending_in_counts_down`, `pending_in_is_none_when_empty`,
      `repeated_paths_are_coalesced`, `a_continuous_writer_cannot_defer_forever`,
      `debounce_window_is_150_milliseconds`,
      `poll_timeout_is_the_tick_when_nothing_is_pending`, and
      `poll_timeout_is_the_remaining_window_and_never_zero`.
      Red-when: any of them calls `thread::sleep` or captures `Instant::now()` a second time.
      `NOSLEEP` leg 2 catches the sleep; the second `Instant::now()` is caught by review and by
      `NOBLOCK` leg 2 only if it strays under `src/ui/`, so state it here as the group's own
      discipline.
      Second Red-when: `poll_timeout_is_the_remaining_window_and_never_zero` asserts only the
      `min`. It must also assert `poll_timeout(250ms, Some(0ms)) == 1ms`, or the floor that
      bounds a spin is unpinned.
      Third Red-when: `pending_in_counts_down` omits the past-the-end case. It must assert
      `pending_in(t0 + 400ms) == Some(0ms)` **and not panic** — `Instant - Instant` panics
      when its argument is the later of the two, so the obvious `self.end - now` is green
      against every other assertion in this group and panics the pane the first time
      `pending_in` is reached after the window closed. `Instant::saturating_duration_since` is
      the API.
      Fourth Red-when: `a_continuous_writer_cannot_defer_forever` is omitted, or asserts only
      that the extension happened. Without a `DEBOUNCE_MAX` cap this is a *sliding* window: an
      agent saving more often than every 150ms — this change's own motivating case — defers
      the batch forever and the pane never updates while it is most useful. The test must push
      at `t0`, `t0 + 100ms`, … through `t0 + 1100ms` and assert `take_due(t0 + 1000ms)` is
      `Some(_)`.

- [x] 5.2 GREEN: Implement `watch::DEBOUNCE` (150ms), `watch::DEBOUNCE_MAX` (1 second),
      `watch::Debounce`, and the real `watch::poll_timeout` per
      `specs/watch-invalidation/spec.md`. `push` sets the window's end to
      `min(now + DEBOUNCE, first_push + DEBOUNCE_MAX)`; `pending_in` uses
      `Instant::saturating_duration_since`. `Debounce` coalesces
      without duplicates in a deterministic order — a `BTreeSet<PathBuf>` is the obvious
      shape, and the reason it matters is measured: one file save on macOS produced **six**
      FSEvents events across four `EventKind`s, and the pane must run the CLI once for it.
      Red-when: `Debounce` stores an `Instant::now()` of its own. It stores the deadline it
      was **given**; the one real clock call arrives in group 6, in `RealFsEvents::drain`.

- [x] 5.3 REFACTOR: Clean up while the twenty-one tests stay green, or state that none was
      needed.

- [x] 5.4 CHECK: `MIN=24 SLEEP_MIN=3 sh $CHECKS/NOSLEEP.sh` — still exactly three sleep sites
      in the tree, all deadline-bounded, and none in `src/watch.rs`. This is the group where
      that number would have grown if the debounce had read the clock itself: every test here
      captures `Instant::now()` **once** as `t0` and asserts against `t0 + 149ms` and
      `t0 + 150ms`. The fourth site arrives in group 6, with the one real-watcher poll.

- [x] 5.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'watch::tests::' 22`; the four
      gate commands, still failing on exactly the one known acceptance test.
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 6. The real watcher, and the dependency
<!-- kind: behavior -->

This is the one group that touches `Cargo.toml`, `Cargo.lock`, and
`tests/fixtures/build-graph.txt`, and it edits `DEPS` and `GRAPH-SNAP` in the same commit, so
no intermediate state leaves a manifest and a snapshot disagreeing.

- [x] 6.1 CHANGE: Add to `Cargo.toml`, after `pulldown-cmark`:
      `notify = { version = "8.2.0", default-features = false, features = ["macos_fsevent"] }`.
      **Confirm the version against the registry first** — `cargo info notify` — rather than
      taking 8.2.0 from this file; it was the latest **stable** at planning time
      (`cargo info` printed `8.2.0 (latest 9.0.0-rc.5)`, and the 9.0.0 line is a pre-release
      declaring MSRV exactly 1.88). If the registry now offers a newer stable, adopt it, and
      re-measure the graph delta rather than reusing the numbers below.
      Red-when: `default-features = false` is written with no feature list. **Measured: that
      does not compile on macOS** — `error[E0432]: unresolved import fsevent_sys`, because
      notify's fsevent module is gated on `not(feature = "macos_kqueue")`. This is the single
      most likely wrong edit in the whole change.

- [x] 6.2 CHANGE: Edit `$CHECKS/DEPS.sh` and `$CHECKS/GRAPH-SNAP.sh` **on disk** by the exact
      string replacements written out above — **three** in `DEPS`, and **four** in
      `GRAPH-SNAP` across its two labelled edits — then
      print `diff -u "$WORK/DEPS.src" "$CHECKS/DEPS.sh"` and
      `diff -u "$WORK/GRAPH-SNAP.src" "$CHECKS/GRAPH-SNAP.sh"` and confirm each diff shows
      **exactly** those hunks and nothing else. `tui-shell` recorded four `DEPS` edits as
      prose in a checkbox and never changed the file; this task's evidence is the two diffs.

- [x] 6.3 CHANGE: Regenerate the graph snapshot **on disk**:
      `GRAPH_WRITE=1 sh $CHECKS/GRAPH-SNAP.sh`, then
      `git diff --stat tests/fixtures/build-graph.txt`. Expected, and **measured at planning
      time in a scratch copy**: `310 → 332` lines, a `22 0` numstat — five added lines per
      macOS triple (`fsevent-sys v4.1.0`, `notify v8.2.0`, `notify-types v2.1.0`,
      `same-file v1.0.6`, `walkdir v2.5.0`) and six per Linux triple (the same five with
      `fsevent-sys` replaced by `inotify v0.11.5` and `inotify-sys v0.1.8`), nothing removed.
      Read the diff and confirm each line. Note the build graph legitimately **differs between
      macOS and Linux** and both are in the fixture, one section per triple; regenerating on
      either platform produces all four sections, because `cargo tree --target <triple>`
      resolves without building.
      Red-when: the diff removes a line, or the numstat is not `22 0`. Either means the
      resolution moved for a reason other than this addition, and it is investigated rather
      than committed.

- [x] 6.4 RED: Write five failing tests in `watch::tests::`, named from their scenarios:
      `no_fs_events_never_yields`, `start_on_a_missing_path_degrades_and_names_the_reason`,
      `start_on_a_real_directory_yields_the_touched_path`,
      `a_started_watcher_writes_nothing`, and `watch_error_display_names_the_reason`.
      `start_on_a_real_directory_yields_the_touched_path` is **the one test in this change
      that opens a real watcher and waits for an event**. It polls `drain` in a loop bounded
      by a deadline **ten seconds** in the future, sleeping **10 milliseconds** between polls
      — the same shape `tests/cli.rs`'s two `try_wait` polls already use in this crate, whose
      own comment records the flake that produced the rule. A sleep *inside* a
      deadline-bounded poll cannot make an assertion premature, because the condition is
      re-tested after every sleep; a `yield_now` spin would instead hold a core for the whole
      window and, on a loaded two-core runner, compete for CPU with the `notify` thread
      producing the event it waits for. `NOSLEEP` leg 1 accepts this span — it names
      `deadline` and `while` — and leg 2 does not cover `src/watch.rs`. Its failure message
      names the written path and says the filesystem may not support change notification, so a
      genuine platform limitation is distinguishable from a defect. It compares the received path by
      **suffix** (`alpha/tasks.md`), because macOS FSEvents returns canonicalized paths and
      `/var/folders/…` arrives as `/private/var/folders/…` — measured at planning time.
      Red-when: the test sleeps a fixed interval and **then** asserts an event arrived, with
      no loop and no deadline. That is the exact defect this change exists not to reintroduce,
      and `NOSLEEP` leg 1 fails it.
      Second Red-when: `start_on_a_missing_path_degrades` asserts only that a problem is
      non-empty. It must assert the returned `FsEvents` is the **inert** one — `drain` is
      `Ok(None)` forever — or the degraded path is untested and only its message is.

- [x] 6.5 GREEN: Implement `watch::RealFsEvents` and `watch::start` per
      `specs/watch-invalidation/spec.md`: a `notify::RecommendedWatcher` constructed with an
      `std::sync::mpsc::Sender<notify::Result<Event>>` (an `EventHandler` **unconditionally**,
      with no feature enabled — verified against notify 8.2.0's source), watching the root
      with `RecursiveMode::Recursive`, feeding a `Debounce`. `drain` holds the crate's **one**
      `Instant::now()` call, its fourth one-line binding to the real world beside
      `cli::npm_prefix`, `config::env_lookup`, and `ui::read_artifact`. `start` returns
      `(Box<dyn FsEvents>, Vec<String>)` and never a `Result`.
      Red-when: `RealFsEvents::drain` calls `rx.recv()` or `rx.recv_timeout(..)`. It calls
      `try_recv` in a loop until the channel is empty; a blocking receive on the render path
      is the failure mode this whole change is shaped around.
      Second Red-when: `drain` returns `Ok(Some(vec![]))` for an empty batch. An empty batch
      and no batch are the same fact, and two spellings of it invite a caller to request a
      refresh for nothing.
      Third Red-when: `RealFsEvents::pending_in` calls `Instant::now()`. It returns the
      `Option<Duration>` `drain` cached from `debounce.pending_in(now)`, using the same `now`
      `drain` already captured — otherwise `src/watch.rs` has **two** clock bindings and
      `SPEC.md` → Architecture's "one binding" claim, which task 13.2 writes, is false. Sound
      because `run_loop` calls `drain` and then `pending_in` in the same iteration.

- [x] 6.6 CHECK: The checks that arm at this group, plus their deferred plants from task 0.4:
      - `MIN=23 sh $CHECKS/WATCHSEAM.sh` — green for the first time. Then plant
        `// use notify::Event;` in `src/ui/driver.rs` (expect FAIL naming it) and
        `// a ratatui Style` in `src/watch.rs` (expect FAIL), reverting each.
      - `WORK=$WORK sh $CHECKS/DEPS.sh` — the **edited** script, all six legs green including
        the new 2e. Then plant `features = []` for `notify` in `Cargo.toml` (expect
        `DEPS FAIL: leg 2e`) and revert.
      - `sh $CHECKS/GRAPH-SNAP.sh` — the **edited** script against the regenerated fixture.
        Then revert **only** the four-name delta string to `linux-raw-sys ` and expect
        `GRAPH-SNAP FAIL: macOS/Linux differ by [fsevent-sys inotify inotify-sys
        linux-raw-sys ], expected [linux-raw-sys ]`, then restore. That control is what proves
        the edit is load-bearing rather than decorative.
      - `MIN=24 SLEEP_MIN=4 sh $CHECKS/NOSLEEP.sh` — the floor rises from 3 to **4**, because
        the real-watcher poll adds the fourth site. Leg 1 must list it by name and judge it
        deadline-bounded; leg 2 still covers `src/ui/` only and stays green; and **leg 2b**
        arms here, capping `src/watch.rs` and `src/refresh.rs` at one sleep each. Plant a
        second — a `fn settle() { thread::sleep(300ms) }` helper below the real-watcher test,
        the shape leg 1's span splitter attributes to the *preceding* test and passes — and
        expect `NOSLEEP FAIL (leg 2b): src/watch.rs names a sleep 2 times`, then revert.
        **Verified at planning time.**
      - `UI_MIN=11 sh $CHECKS/NOBLOCK.sh` — legs 2 and 3's **watch-side** controls arm here for
        the first time, because `src/watch.rs` now names a clock and `try_recv`. Then plant
        `let _ = rx.recv_timeout(d);` in its production slice (expect
        `NOBLOCK FAIL (leg 3): src/watch.rs blocks on a channel`) and delete `try_recv` from it
        (expect the positive-control failure), reverting each. The refresh-side controls stay
        red until group 7.
      - `git status --porcelain src/ Cargo.toml` empty after every revert.

- [x] 6.7 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'watch::tests::' 27`;
      `cargo build --locked`; the four gate commands, still failing on exactly the one known
      acceptance test. `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh` — `Cargo.toml`,
      `Cargo.lock`, and `tests/fixtures/build-graph.txt` are all outside `openspec/`, so it
      still prints OK. Commit.

---

## 7. The worker thread
<!-- kind: behavior -->

**Every assertion in this group is made after receiving a result**, which is a real
happens-before edge: the worker sends its result only after all of that cycle's CLI calls have
returned, so the recorded call list is complete at the moment the result arrives. No assertion
is made after a sleep, and no negative assertion is made across a thread boundary — the
per-change invalidation rule was proven single-threaded in group 3, precisely so it need not
be proven here.

- [x] 7.1 RED: Write eight failing tests in `refresh::tests::`, named from their scenarios:
      `no_refresher_never_yields`, `start_without_a_binary_is_inert`,
      `start_without_a_repo_is_inert`, `the_worker_answers_with_files_then_merged`,
      `a_failing_cli_still_sends_the_files_result`,
      `drain_and_fold_unions_queued_requests`,
      `dropping_the_refresher_disconnects_the_channel`, and `the_worker_writes_nothing`.
      Every wait is `rx.recv_timeout(Duration::from_secs(10))` with an `.expect` naming what
      was being waited for — a condition with a generous deadline, never a sleep, and never a
      poll of the non-blocking `take_result`, which has no legal pacing inside
      `src/refresh.rs`. The receiver is reached through the `#[cfg(test)]` constructor
      `refresh::worker_for_test`, because `Box<dyn Refresher>` exposes only `take_result` and
      cannot be downcast.
      Red-when: `dropping_the_refresher_disconnects_the_channel` asserts a **thread count**.
      On Linux both `notify`'s and a plain `mpsc` worker's shutdown are signalled without
      being joined, so a thread count read immediately after a drop is racy while a
      disconnected channel is not. Measured against notify 8.2.0's source: macOS's
      `FsEventWatcher::drop` joins; Linux's `INotifyWatcher::drop` discards its `JoinHandle`.
      Second Red-when: that same test asserts `is_err()` rather than
      `matches!(…, Err(RecvTimeoutError::Disconnected))`. `Err(Timeout)` is an `Err` too, and
      it is **exactly** the failure — a leaked worker — the test exists to catch, so `is_err()`
      would pass precisely when the claim is false, ten seconds later.
      Third Red-when: the folding rule is proved through the live worker. The obvious threaded
      version — send `Only({a})`, `Only({b})`, `All`, then read the fake's calls "after the
      last `Merged` result" — **cannot fail**: the union is `All`, so "nothing outside the
      folded selection" is a tautology; `from_cli_cached` applies to every change with no
      cache entry, so the worker's first cycle is unconditionally a full apply; and *which*
      result is the last depends purely on interleaving, so establishing it needs a timeout
      followed by a negative assertion — the one shape this change forbids. Instead,
      `drain_and_fold_unions_queued_requests` calls the extracted
      `drain_and_fold(first, &rx)` **directly, on the test's own thread**, over a receiver
      into which two further selections have been sent and whose sender has been dropped.

- [x] 7.2 GREEN: Implement `refresh::Worker`, `refresh::start`, and the `#[cfg(test)]`
      `refresh::worker_for_test` per `specs/refresh-worker/spec.md`: two `mpsc` channels, one
      `thread::spawn`, an `Arc<dyn OpenspecCli>`, a `CliCache` owned for the thread's
      lifetime, and per request `from_files` → send `Files` → `from_cli_cached` → `merge` →
      send `Merged`. Fold queued requests through the **named private**
      `drain_and_fold(first, &rx)` before starting a cycle — named because group 7.1's test
      drives it directly. Return when either channel disconnects. `worker_for_test` returns
      `(Box<dyn Refresher>, Receiver<RefreshResult>, Receiver<()>)`: the **second** element is
      the worker's result receiver, handed to the test instead of being stored on the
      `Refresher` (whose `take_result` therefore always yields `None` in these tests), and the
      **third** receives from a channel whose `Sender` the worker owns and drops only when its
      body returns. `worker_for_test` is declared **after** `start` and above `mod tests`, and
      `take_result` **before** `start` — `NOBLOCK` Guards D and E and `READONLY-UI`'s Guard D
      enforce both, because `prod()` truncates at a file's first line-anchored `#[cfg(test)]`
      and leg 3 cuts at its `thread::spawn`.
      Red-when: the worker calls `Command::new`. It reaches the program **only** through the
      trait object; `NOSPAWN-GREP` fails otherwise, and `src/cli.rs` stays the crate's one
      spawn site.
      Second Red-when: the worker sends only the merged result. The `Files`-then-`Merged`
      split is what keeps the file read off the render path *and* what makes "files paint,
      the CLI corrects" assertable; collapsing it would either delay the pane by a Node start
      or push `std::fs` into `src/ui/driver.rs`.
      Third Red-when: `Refresher::take_result` uses `recv` or `recv_timeout`. It is
      `try_recv`. `NOBLOCK` legs 1 and 2 never look at this file — leg 3 is what makes "every
      method of both traits is non-blocking" a check rather than a doc comment, and
      `take_result` is called on **every frame**.

- [x] 7.3 REFACTOR: If the worker's body reads as one long function, extract the
      drain-and-fold step and the one-cycle step into named private functions; otherwise state
      that no refactor was needed.

- [x] 7.4 CHECK: Contract gate — re-read `specs/refresh-worker/spec.md` and confirm neither
      `Refresher` nor `RefreshResult` names `CliChanges`, `OpenspecCli`, or `from_cli`, so
      `src/ui/driver.rs` can hold a `&mut dyn Refresher` while `NOCLI-SHELL` stays green
      unweakened. That check is the structural proof the CLI is off the render path.

- [x] 7.5 CHECK: The checks that arm at this group, plus their deferred plants from task 0.4:
      - `UI_MIN=11 sh $CHECKS/NOBLOCK.sh` — **all three** legs green for the first time,
        since `src/refresh.rs` now names `mpsc`, `thread::spawn`, and `try_recv`. Then plant,
        reverting each: `use std::sync::mpsc;` above `pub const TICK` in `src/ui/driver.rs`
        (expect `NOBLOCK FAIL (leg 1)`); `// let _ = std::time::Instant::now();` in
        `src/ui/app.rs` (expect `NOBLOCK FAIL (leg 2)`); `let _ = rx.recv_timeout(d);`
        **before** `src/refresh.rs`'s `thread::spawn` (expect
        `NOBLOCK FAIL (leg 3): src/refresh.rs blocks before its thread::spawn`); the same call
        **after** the spawn (expect **OK** — the deliberate exemption for the worker body, not
        a miss); `try_recv` deleted from that file's production slice (expect the
        positive-control failure); `for r in &self.rx { … }` and `self.rx.iter()` in
        `take_result` (expect `NOBLOCK FAIL (leg 3)` for each — both block and neither names
        `recv`); a module doc comment on line 1 reading "the worker is started by a single
        `thread::spawn` in `start`" **together with** a blocking `take_result` (expect FAIL —
        the cut ignores comment lines, and without that it would be disarmed for the whole
        file); `take_result` moved **below** `start` (expect the Guard E failure); and a
        `#[cfg(test)] pub fn worker_for_test` placed **above** `start` alongside a
        `std::fs::write` in `start`'s body (expect Guard D's failure from **both** `NOBLOCK`
        and `READONLY-UI` — without it `READONLY-UI` reports that write **green**).
        **All ten verified at planning time against stub modules.**
      - `UI_MIN=11 EXTRA="src/watch.rs src/refresh.rs" sh $CHECKS/READONLY-UI.sh` — green,
        re-run at its final floor; its plant was exercised at task 1.10.
      - `MIN=24 SLEEP_MIN=4 sh $CHECKS/NOSLEEP.sh` — four sites, `src/refresh.rs` contributing
        none: every wait here is a `recv_timeout`.
      - `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh` — green and unmoved, which is the evidence that
        the worker stayed outside `src/ui/`.
      - `MIN=21 sh $CHECKS/NOSPAWN-GREP.sh` — the worker spawns a **thread**, not a process.
      - `git status --porcelain src/` empty after every revert.

- [x] 7.6 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'refresh::tests::' 8`; the four
      gate commands, still failing on exactly the one known acceptance test.
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 8. The `r` key, `adopt`, and the forced reload
<!-- kind: behavior -->

- [x] 8.1 RED: Write seven failing tests in `ui::app::tests::`, named from their scenarios:
      `r_maps_to_refresh_outside_filter_mode`, `refresh_sets_requested_and_changes_nothing_else`,
      `adopt_keeps_the_selection_by_name`, `adopt_clamps_when_the_change_is_gone`,
      `adopt_resolves_the_name_against_the_filtered_list`,
      `a_forced_reload_keeps_the_scroll`, and
      `a_tab_move_under_a_forced_reload_resets_the_scroll`.
      Red-when: `a_forced_reload_keeps_the_scroll` asserts only that the reader was called
      again. It must **also** assert `detail.scroll` is unchanged, and its sibling must assert
      the tab move resets it — the pair is what attributes the reset to the key change rather
      than to the flag. A single test cannot distinguish the two.
      Second Red-when: `adopt_keeps_the_selection_by_name` uses a change set whose new list
      has the same length and order. It must **shift** — a change inserted alphabetically
      above the selected one — or an implementation that preserved the index would pass.

- [x] 8.2 CHANGE: Extend the landed `ui::app::tests::` tests this change's scenarios modify,
      without adding to the count. **Key tests:** `a_released_quit_key_does_not_quit` gains
      `Char('r')` Release and Repeat; `navigation_and_filter_keys_*` gains `r`, `R` with
      SHIFT, and `r` with CONTROL; `non_key_events_are_ignored` gains `Paste("r")`;
      `enter_and_esc_move_between_the_routes` gains a `refresh.requested` assertion; and
      **both** `no_action_mutates_changes` and `tasks-checklist`'s landed
      `no_action_mutates_a_task_item` sweep an exhaustive **thirteen**-variant array and
      assert the count is thirteen. **`sync_detail` tests:** the landed read-once test asserts
      `refresh.reload` is false throughout; the unreadable-file test asserts `detail.problems`
      is cleared before a **forced** re-read of an unchanged key; and the empty-visible-list
      test asserts `refresh.reload` is false after a sync that took the empty-list early
      return, so the flag cannot survive an empty list and force a spurious re-read on the
      next frame. **`ui::tests::load::` tests:** `startup_leaves_the_detail_empty_and_unscrolled`
      gains `refresh.reload` false at both construction sites. Their exact names on disk may
      differ; match by the scenario they carry, not by the name written here.

- [x] 8.3 GREEN: Implement the `(KeyCode::Char('r'), KeyModifiers::NONE) => Action::Refresh`
      arm in `action_for` — **in the non-filtering match only**, so `list-filtering`'s
      unchanged rule keeps `r` typing itself while filtering — the
      `Action::Refresh => self.refresh.requested = true` arm in `apply`, `Dashboard::adopt`,
      and `sync_detail`'s `force` per `specs/live-updates/spec.md` and
      `specs/artifact-content/spec.md`.
      Red-when: `sync_detail` resets `detail.scroll` whenever it re-reads. It resets it
      exactly when the **key** changed, which is what lets an agent's save re-render the
      document a reader is halfway down without throwing them to line one.
      Second Red-when: `adopt` resolves the previous name against `changes.active` rather than
      `visible()`. `selected` indexes the visible list, and a `/` filter may be active.

- [x] 8.4 REFACTOR: `sync_detail` now has four sequential concerns — take the flag, clamp the
      tab, decide whether to read, read. Extract the decision into a named private helper if
      the function no longer reads as one thing; otherwise state that no refactor was needed.

- [x] 8.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::app::tests::' 55`;
      `TYPES="Dashboard Filter Detail Refresh" SCAN_MIN=90 sh $CHECKS/NODEFAULT-UI.sh`;
      `sh $CHECKS/NOIO-VIEW.sh`; `UI_MIN=11 sh $CHECKS/NOBLOCK.sh`; the four gate commands,
      still failing on exactly the one known acceptance test.
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 9. The loop's live tier
<!-- kind: behavior -->

- [x] 9.1 RED: Write eight failing tests in `ui::driver::tests::`, named from their scenarios:
      `the_startup_request_precedes_the_first_wait`,
      `a_result_is_adopted_before_the_frame` (at 120x20 **and** 60x20),
      `an_fs_batch_becomes_one_selection`, `a_watch_error_is_recorded_once`,
      `the_wait_shortens_to_the_debounce_deadline`,
      `the_wait_is_the_tick_when_nothing_is_pending`,
      `a_files_result_and_a_merged_result_are_both_adopted`, and
      `an_inert_live_tier_takes_nothing_and_requests_nothing`.
      Red-when: `a_result_is_adopted_before_the_frame` asserts `frames: 2`. It must assert
      `LoopSummary { frames: 1, polls: 1 }` **and** that the single frame's buffer already
      shows the corrected `[7/9]` — two frames would mean the result was applied after the
      draw, which is the ordering bug this test exists to catch.
      Second Red-when: `an_fs_batch_becomes_one_selection` asserts
      `requests().contains(&Selection::Only(…))`. It must be an **exact equality** on the
      whole recorded vector — `[All, Only({"alpha"})]` — or an implementation that requested
      `All` for every batch passes.
      Third Red-when: `a_watch_error_is_recorded_once` asserts only that a problem exists. It
      must assert `refresh.problems.len() == 1` after **two** consecutive `Err` drains, **and**
      `LoopSummary { frames: 4, polls: 4 }` — a watcher failing on every poll must neither grow
      an unbounded list nor end the loop. This test is also where
      `watch-invalidation`'s "`openspec/` is removed while the watcher runs" clause "neither
      `drain` nor the worker returns an error that ends it" is verified: deterministically,
      with a `ScriptedFs`, rather than by opening a real watcher on a removed tree and draining
      it for some elapsed period, which would be a negative assertion bounded by wall-clock
      time.

- [x] 9.2 CHANGE: Extend the landed `ui::driver::tests::` and `ui::tests::detail::` tests to
      pass a real inert `Live` rather than the placeholder from group 1, without adding to
      the count. Every landed frame count, poll count, buffer assertion, and reader call count
      must be **unchanged** — that is the evidence for `live-updates` → "An inert live tier
      leaves the loop exactly as it was", and it is the state a machine with no `openspec`
      binary and no working watcher runs in.

- [x] 9.3 GREEN: Implement `run_loop`'s three live-tier steps and the
      `watch::poll_timeout(tick, live.fs.pending_in())` wait, per
      `specs/live-updates/spec.md` → "The loop drives the live tier without ever waiting on
      it". Steps 1 to 3 precede `sync_detail`; nothing else in the loop moves.
      Red-when: the loop drains results in a `while let Some(r) = take_result()` loop. It takes
      **at most one per iteration**; a drain loop is one dropped frame away from a spin and
      makes the `takes() == 3` assertion meaningless.
      Second Red-when: `run_loop` calls `changes::from_files` when a batch arrives. That would
      put `std::fs` into a pure view file and fail `NOIO-VIEW` — the change would have failed
      at its one job. The worker sends the file result.

- [x] 9.4 CHECK: `run_loop` gains no clock, no channel, and no thread: confirm by
      `git diff` over `src/ui/driver.rs`'s production code, and by
      `UI_MIN=11 sh $CHECKS/NOBLOCK.sh`.

- [x] 9.5 REFACTOR: If `run_loop`'s body now reads as seven steps rather than one loop,
      extract the live-tier prelude into a named private function taking `&mut Dashboard` and
      `&mut Live`; otherwise state that no refactor was needed. The function must not acquire
      the ability to block by being extracted — `NOBLOCK` searches the whole production slice,
      not just `run_loop`.

- [x] 9.6 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::driver::tests::' 22`;
      `sh $CHECKS/NOIO-VIEW.sh`; `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`; the four gate
      commands, still failing on exactly the one known acceptance test.
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 10. The rendered buffer
<!-- kind: behavior -->

- [x] 10.1 RED: Write three failing tests in `ui::list::tests::`, each naming both `38` and
      `58`: `refresh_problems_lead_the_rows`, `refresh_and_change_problems_in_order`, and
      `a_refresh_problem_row_degrades_at_narrow_widths`; and five in `ui::view::tests::`,
      each rendering at **both** 60 and 120 columns and asserting named cells:
      `a_watch_problem_is_the_first_row`, `refresh_problems_precede_change_problems`,
      `no_refresh_problem_draws_no_extra_row`, `the_corrected_progress_reaches_the_buffer`,
      and `a_removed_repo_shows_the_problem_row` — the last covering the view half of
      `watch-invalidation`'s "`openspec/` is removed while the watcher runs".
      Red-when: a test renders and asserts only that the buffer is non-empty. `quality-gates`
      → "View behaviour is verified against a `TestBackend` buffer at both widths" rejects
      that explicitly: it stays green with the behaviour deleted.
      Second Red-when: `no_refresh_problem_draws_no_extra_row` asserts only that some row
      exists. It must assert the buffer is **byte-identical** to the same dashboard rendered
      with the field absent — the evidence that the common case gained no row.

- [x] 10.2 GREEN: Extend `ui::list::rows` to emit `dashboard.refresh.problems` before
      `dashboard.changes.problems`, both through the existing `problem_row_text` grammar and
      both carrying `RowKind::Problem`. The `repo.is_none()` early return is **unchanged**.
      Red-when: a new `RowKind` variant is added for refresh problems. `change-rows` requires
      one kind, so `ui::view` styles them identically and neither is addressable by
      `selected`; a second variant would need a second styling arm and a second selection rule.

- [x] 10.3 CHECK: `ui::view::style_for` and `ui::view::render_list` are **unchanged** — no new
      `Face`, no new modifier mapping, no new row kind. Confirm by reading them and by
      `git diff` over those functions.

- [x] 10.4 REFACTOR: Fold the repeated "build a dashboard with a refresh problem and this
      change set" setup into one test helper if it appears more than three times; otherwise
      state that no refactor was needed.

- [x] 10.5 VERIFY: `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::list::tests::' 20`;
      `testcount --lib 'ui::view::tests::' 81`; `LIST_MIN=20 sh $CHECKS/LISTWIDTHS.sh`;
      `WIDTHS_MIN=81 sh $CHECKS/WIDTHS.sh`; `sh $CHECKS/NOIO-VIEW.sh`; the four gate commands,
      still failing on exactly the one known acceptance test.
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 11. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [x] 11.1 CHANGE: Extend `ui::tests::live::` with its second and third tests, at 120x20 and
      again at 60x20, driving `run_loop` over the same real `ScratchDir` repository with a
      script that presses `r`, then every ASCII printable character from `!` to `~`, then
      `Enter`, `Esc`, `Backspace`, `Tab`, the four arrows, and finally `Ctrl-C`:
      - `r_forces_a_refresh_through_the_loop` — with a **`ScriptedFs` whose `drain` always
        returns `Ok(None)`**. Assert `refresher.requests()` is **exactly**
        `[Selection::All, Selection::All]`: the startup request and the `r`-driven one, and
        nothing else.
      - `a_live_watcher_over_the_tree_writes_nothing` — with a **real** `watch::start` over
        the tree. Assert **only** that the tree is byte-identical across the run and that
        re-reading `tasks.md` still gives the same `Progress`, with the discriminating
        one-byte control proving the comparison can fail. This is the runtime half of
        `OPENSPEC-UNTOUCHED`'s source-level claim, with a live watcher open on the tree.
      Splitting them is deliberate, not tidiness: an exact equality on a vector a **live** OS
      watcher can append to is protected only by the run finishing inside the 150ms debounce
      window — a budget nothing states, nothing asserts, and `cargo llvm-cov`'s instrumentation
      (task 14.5, 2–5× slower) erodes. The day a stray event under the scratch root becomes
      due mid-run, a third request appears and the equality goes red: a test passing because a
      race resolved favourably, which is this change's own stated hazard.
      Red-when: the two claims are recombined into one test "to save a fixture".

- [x] 11.2 VERIFY: `cargo test --all-features --lib 'ui::tests::live::'` — three tests, all
      green, including `files_paint_then_the_cli_corrects`, which has been red since group 2.
      Confirm the other four `ui::tests::` submodules still pass unchanged; if any needed its
      expectations updated, name the change and why here.

- [x] 11.3 REFACTOR: Clean up the group-2 harness — the `ScratchDir` builder and the scripted
      key list — if it duplicates `ui::tests::load::`'s or `ui::tests::detail::`'s existing
      tree builder; otherwise state that no refactor was needed.

- [x] 11.4 VERIFY: the literal, unqualified **`make check`** — its first run since group 1 —
      exits 0. If it fails, name the failing sub-command rather than summarising.
      `. $CHECKS/TESTCOUNT.sh`; `testcount --lib 'ui::tests::live::' 3`.
      `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`. Commit.

---

## 12. Change Review
<!-- kind: operational -->

- [ ] 12.1 CHECK: Dispatch an independent reviewer — a fresh subagent that did **not** write
      the implementation, given only `proposal.md`, the **nine** spec files, `design.md`, this
      file, and the diff. It reports findings and changes nothing. Point it first at the
      concentration points `openspec/config.yaml` names for this repository — nothing spawns
      outside `cli`; views do no I/O; nothing writes inside `openspec/`; every external
      dependency has an absent case; both producers emit the same type; view tests run at 60
      **and** 120; then the three `openspec/config.yaml` names that this change does not
      obviously touch but must still be checked — agent attribution must not guess (nothing
      here attributes anything), Herdr agent names are capped at 32 characters (no name is
      derived here), and logic must stay out of the spawn implementations and `main` (both are
      untouched) — and then at this change's own four hazards and two defect classes:
      - **Hazard 1**: audit **every** wait, sleep, `Instant`, `Duration`, and deadline in the
        diff. Any assertion that something has *already* happened after a fixed interval is a
        CRITICAL, whatever its deadline.
      - **Hazard 2**: find any expression on the render path that can block. Check that
        `NOBLOCK`'s patterns actually match the shapes in the diff.
      - **Hazard 3**: confirm the eight-file pure set neither shrank nor gained an exemption,
        and that the artifact read still goes through the injected reader.
      - **Hazard 4**: confirm the per-change invalidation test asserts an **exact** call
        vector, not a `!contains`, and that a test proves an unrelated change was not
        re-asked about.
      - **Defect class (a)**: any check structurally incapable of failing — a filter matching
        nothing, a floor already satisfied, a bound argument in the wrong position.
      - **Defect class (b)**: any check that cannot see what it guards in the formatting this
        tree really uses — multi-line struct literals, `..base` in helpers, doc comments,
        macro-generated code. Extract and run **every** check against a planted violation.
      - **Concurrency claims specifically**: any test that could pass because an interleaving
        happened to resolve favourably.
      The reviewer writes findings incrementally to a scratchpad file as it goes, not only in
      its final message.

- [ ] 12.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run the affected tests.

- [ ] 12.3 VERIFY: No blocking or unowned finding remains; every accepted WARNING carries its
      reason in `planning-review.md`. Commit.

---

## 13. Documentation
<!-- kind: operational -->

- [ ] 13.1 Rewrite in `SPEC.md`: **Data layer → Refresh** (audience: anyone implementing
      against the contract). It is currently three lines and describes none of the mechanism.
      Replace with: the one recursive `notify` watch on `openspec/`; the 150ms debounce as a
      pure state machine taking `now` as a parameter; the classification of a touched path to
      a `Selection`; the worker answering one request with the file result then the merged
      one; `r` forcing `Selection::All`, which is also what startup issues; and the statement
      that the loop's wait shortens to the debounce deadline while `TICK` stays 250ms.
      Durable because it is the section a future reader would implement a synchronous refresh
      from.

- [ ] 13.2 Rewrite in `SPEC.md`: **Architecture** (audience: the same) — the paragraph
      enumerating the crate's one-line bindings to the real world currently names three
      (`cli::npm_prefix`, `config::env_lookup`, `ui::read_artifact`). Add the fourth, the
      `Instant::now()` call in `watch::RealFsEvents::drain`, and state why it is one: the
      debounce takes `now` as a parameter, so the clock has exactly one call site and no view
      test can reach it. Add the two new module rows to the **Module map** — `watch`
      (filesystem watching, debounce, and per-change invalidation) and `refresh` (the worker
      thread and the `Refresher` seam) — the second of which the map does not have at all.

- [ ] 13.3 Rewrite in `SPEC.md`: **Keys** (audience: the same) — the `r` row currently reads
      "Force refresh" with no filter-mode note, while every other command key's row or the
      paragraph below it says what filtering does to it. State that `r` types itself while
      filtering, like every other printable key.

- [ ] 13.4 Add to `SPEC.md`: **Degraded states**, four rows (audience: the same) — a `notify`
      watcher that will not start (the pane runs unwatched, `r` still refreshes, and the
      reason is a `!`-marked row); `openspec/` removed while the pane is open (an empty list
      with the read failure named, the loop still drawing); a CLI cycle that fails after the
      file result was already sent (the pane keeps the file-sourced numbers and names the
      failure); and a change whose artifacts are one cycle stale because its path was
      classified as another's (its **progress** is still correct, because that comes from the
      fresh `list --json`, and `r` corrects the rest). The fourth is the row that stops a
      future reader "fixing" the cache by making progress come from it.

- [ ] 13.5 Rewrite in `SPEC.md`: **Stack** (audience: the same) — the sentence naming `notify`
      already exists, but with no version and no feature. Give it the same treatment
      `pulldown-cmark` has: the version, `default-features = false, features =
      ["macos_fsevent"]`, and a pointer to this change's design.md for the argument, so a
      later change needing a watcher does not re-open it.

- [ ] 13.6 Rewrite in `SPEC.md`: **Testing and quality gates → Unit-tested modules**
      (audience: the same) — the bulleted list names every unit-tested module and stops at
      `ui`. Add two bullets in the list's existing style: `watch::classify`,
      `watch::invalidate`, `watch::Debounce`, and `watch::poll_timeout` — pure functions over
      values and an **injected instant**, with `watch::start` and `RealFsEvents` the
      filesystem edge, tested against a real `ScratchDir` and against a path that does not
      exist; and `refresh::start`, `refresh::none`, and the worker — the crate's one thread,
      tested through a `#[cfg(test)]` constructor with every assertion made after a
      `recv_timeout` returned an item. Also add one sentence to **View tests** recording that
      no `ui::` test starts a thread, opens a watcher, or reads a clock, and that the two
      new doubles in `crate::testutil` are synchronous by construction.

- [ ] 13.7 Rewrite in `AGENTS.md`: **Current repo state**, the **dependency count**, and
      **one new Architecture rule** (audience: every future session). (a) The landed-changes
      enumeration currently ends at `tasks-tab`; add `live-refresh`, and add two sentences on
      what the pane now does — the watcher, the worker, and `r`. (b) "five third-party
      dependencies" becomes **six**, with `notify` named. (c) A new Architecture-rules bullet:
      **the render path blocks on nothing but the terminal, and reads no clock** — naming
      `src/watch.rs` and `src/refresh.rs` as the two modules outside `src/ui/` that hold the
      watcher and the worker, the non-blocking `FsEvents`/`Refresher` contracts, and the
      `NOBLOCK` and `NOSLEEP` checks. Rewrite all three in place; do not append a second entry
      beside the existing ones.

- [ ] 13.8 CHECK: Deferred to archive time, not doable now —
      `openspec/IMPLEMENTATION-ORDER.md`'s `live-refresh` row and the "Notes on the ordering"
      section must be confirmed to still describe what was built, and corrected if not. In
      particular the roadmap's `degraded-states` row depends on `live-refresh` and its "audit
      every row against the running plugin" scope now has four more rows to audit.
      `OPENSPEC-UNTOUCHED` forbids writing anywhere under `openspec/` outside this change's
      own directory, so this is recorded here as a deferred obligation the way `list-view`
      (10.4b), `markdown-viewer` (11.8), `detail-view` (13.4), and `tasks-tab` (12.6) did, and
      is discharged by `openspec archive`.

- [ ] 13.9 VERIFY: `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh` — the doc edits are to
      `SPEC.md` and `AGENTS.md`, both outside `openspec/`, so it still prints OK. Commit.

---

## 14. Lint & Verify
<!-- kind: operational -->

- [ ] 14.1 CHECK: Inspect the intended verification commands and affected tiers: the four
      `make` gates, the **twenty-six** extracted files (twenty-five gates plus `TESTCOUNT.sh`,
      which is sourced rather than run), and the **thirteen** `testcount` invocations across
      **eight** distinct filters — `changes::tests::`, `watch::tests::` (three times, at 11,
      22, and 27), `refresh::tests::`, `ui::app::tests::` (twice), `ui::driver::tests::`,
      `ui::list::tests::`, `ui::view::tests::`, and `ui::tests::live::`. Name any that this
      change's groups did not run at their final floors.

- [ ] 14.2 VERIFY: `make lint` — `cargo clippy --all-targets --all-features -- -D warnings`,
      0 errors, no `#[allow]` added anywhere.

- [ ] 14.3 VERIFY: `make fmt-check` — `cargo fmt --all -- --check`, clean. Rust has no
      separate type checker; `cargo clippy --all-targets` in 14.2 type-checks every target,
      including the test target, which is the equivalent step.

- [ ] 14.4 VERIFY: `make test` — `cargo test --all-features`, green.

- [ ] 14.5 VERIFY: `make coverage` — `cargo llvm-cov --fail-under-lines 80`, passing with no
      exclusion and no threshold change. Quote the TOTAL row's **line** column, not the region
      column it leads with (planning-time baseline: **97.52% over 16,151 lines**). Then
      `sh $CHECKS/NOWAIVER.sh`. If the figure fell, name which new module is under-covered and
      write the missing test — **the floor does not move**, and it has not moved in fifteen
      changes.

- [ ] 14.6 VERIFY: Every check at its final floor, each printing its OK line verbatim:
      `MIN=21 sh $CHECKS/NOSPAWN-GREP.sh`; `sh $CHECKS/NOIO-VIEW.sh`;
      `UI_MIN=10 sh $CHECKS/READSEAM.sh`; `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`;
      `TYPES="Dashboard Filter Detail Refresh" SCAN_MIN=90 sh $CHECKS/NODEFAULT-UI.sh`;
      `MIN=21 sh $CHECKS/NOLIT-CHANGE.sh`; `MIN=21 sh $CHECKS/MDSEAM.sh`;
      `sh $CHECKS/NOTABSEAM.sh`; `sh $CHECKS/TASKSEAM.sh`; `MIN=23 sh $CHECKS/WATCHSEAM.sh`;
      `sh $CHECKS/NORAW-GREP.sh`; `UI_MIN=11 sh $CHECKS/NOBLOCK.sh`;
      `MIN=24 SLEEP_MIN=4 sh $CHECKS/NOSLEEP.sh`;
      `UI_MIN=11 EXTRA="src/watch.rs src/refresh.rs" sh $CHECKS/READONLY-UI.sh`;
      `WIDTHS_MIN=81 sh $CHECKS/WIDTHS.sh`; `LIST_MIN=20 sh $CHECKS/LISTWIDTHS.sh`;
      `MD_MIN=24 sh $CHECKS/MDWIDTHS.sh`; `DETAIL_MIN=23 sh $CHECKS/DETAILWIDTHS.sh`;
      `TASK_MIN=16 sh $CHECKS/TASKWIDTHS.sh`; `python3 $CHECKS/GATE-MECH1.py src`;
      `sh $CHECKS/NOJSON-SEAM.sh`; `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh`.

- [ ] 14.7 VERIFY: `WORK=$WORK sh $CHECKS/DEPS.sh` and `sh $CHECKS/GRAPH-SNAP.sh` — both the
      **edited** scripts, both green, with `DEPS` leg 5 now running six removal experiments
      and `GRAPH-SNAP` asserting the four-name macOS/Linux delta against a 332-line fixture.
      `cargo build --locked`. `git diff --stat $BASE -- Cargo.toml Cargo.lock
      tests/fixtures/build-graph.txt` shows exactly the three files and the `22 0` numstat on
      the fixture; `git diff --name-only $BASE -- herdr-plugin.toml` is empty.

- [ ] 14.8 VERIFY: The library and `ui::` totals, asserted here and nowhere else:
      `cargo test --all-features --lib 2>&1 | sed -n 's/^test result: ok\. \([0-9]*\)
      passed.*/\1/p' | awk '{t+=$1} END{print t+0}'` → **748** (673 measured + 75 new); and
      `cargo test --all-features --lib 'ui::' …` → **286** (260 measured + 26 new — the
      `watch::` and `refresh::` tests are outside `src/ui/` and count only toward the library
      total). A count below either target means an enumerated test was not written — write it,
      do not lower the number.

- [ ] 14.9 VERIFY: `export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"` and
      `openspec validate live-refresh --strict` — the `openspec` binary is nvm-installed here
      and is not on a non-login shell's default `PATH`.
