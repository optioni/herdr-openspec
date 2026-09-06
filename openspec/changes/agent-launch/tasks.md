# agent-launch — implementation tasks

## How to read this file

Every floor here was **measured** before it was set, and the command that produced each
number is written beside it. A realized count below a group's target means the missing test
is written, not the floor lowered.

Every check is extracted to `$CHECKS/<LABEL>.sh` in task 0.2 and run from the extracted file
thereafter, never from a retyped copy. This change **adds one** check block (`LAUNCHSEAM`,
reproduced in full below), **edits three** (`NOIO-VIEW`, `WIRED`, `NOBLOCK`) by the exact string
replacements in task 0.5, and moves the invocation floor of **eight more**
(`NOSPAWN-GREP`, `NOLIT-CHANGE`, `AGENTSEAM`, `WATCHSEAM`, `NOSLEEP`, `WIDTHS`, `LISTWIDTHS`,
`EXTENDED`) plus one type list and two span floors (`NODEFAULT-UI`). `MDWIDTHS`,
`TASKWIDTHS`, and `DETAILWIDTHS` are unchanged but are invoked with their measured floors for
the first time; `agent-attribution` left all three bare, so they have been running at block
defaults of 23, 14, and 19 against realized counts of 24, 16, and 23.
`OPENSPEC-UNTOUCHED` is neither edited nor re-defaulted: every invocation here passes
`CHANGE=agent-launch` and re-derives `BASE=$(git rev-parse HEAD)` fresh.

`testcount` is sourced, not run: `. $CHECKS/TESTCOUNT.sh`, then
`testcount --lib '<filter>' <minimum>`. It takes the scope, the filter, **and** the minimum,
in that order, and is judged on a counted minimum because a `cargo test` filter matching
nothing exits 0. Every filter below that guards a **single** test is written out in full for
that reason.

**`make check` is not runnable unqualified between groups 2 and 12.** Group 2's acceptance
tests are deliberately RED for that whole span and `cargo llvm-cov` hard-fails on any failing
test, so the four gates are run individually there:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features            # must fail on EXACTLY the three known acceptance tests,
                                     # with the identical assertion messages each time
cargo llvm-cov --ignore-run-fail --fail-under-lines 80
```

A **fourth** failing test at any of those boundaries is a real regression, not a known one.
`--ignore-run-fail` ignores a failing test **run**, never a failing compile, which is why
group 1 lands the inert structure first — see design.md → Test Strategy. The literal,
unqualified `make check` is run from task 12.4 onward.

## Measured at planning time

Every figure below came from the command shown, run on `main` at
`df36bcbb4e32f0df66a6952f7c77a5e650f51c6c`. **That sha is recorded for comparison only.**
Task 0.1 re-derives `BASE=$(git rev-parse HEAD)` fresh — this repository is shared with other
live sessions and its history was rewritten once, so a hardcoded sha is a gate that silently
includes this change's own commits in its baseline.

| Figure | Command | Value |
|---|---|---|
| Library tests | `cargo test --all-features --lib -- --list \| grep -c ': test$'` | **840** |
| Line coverage | `cargo llvm-cov --summary-only \| tail -1` | **97.14%**, 22,257 lines, 637 uncovered |
| Region count (**not** a line count) | same TOTAL row | 36,413 |
| `src/ui/*.rs` | `find src/ui -name '*.rs' \| wc -l` | **11**, unchanged by this change |
| `NOSPAWN-GREP` / `NOLIT-CHANGE` set | `find src -name '*.rs' ! -path 'src/changes.rs' \| wc -l` | **22** → 23 |
| `AGENTSEAM` set | `find src tests -name '*.rs' ! -path src/cli.rs ! -path src/agents.rs ! -path src/ui/mod.rs \| wc -l` | **22** → 23 |
| `LAUNCHSEAM` set | the same with `! -path src/launch.rs` added | — → **22** |
| `NOSLEEP` set | `find src tests -name '*.rs' \| wc -l` | **25** → 26 |
| `WATCHSEAM` set | the same with `! -path src/watch.rs` — the block excludes its own subject | **24** → 25 |
| `GATE-MECH1` set | `find src -name '*.rs' \| wc -l` | **23** → 24; its floor is hardcoded at 8, so no edit is needed |
| `#[test]` in `src/ui/view.rs` | `grep -c '^[[:space:]]*#\[test\][[:space:]]*$' src/ui/view.rs` | **88** → 94 |
| `#[test]` in `src/ui/list.rs` | same, `src/ui/list.rs` | **25** → 29 |
| `#[test]` in `src/ui/app.rs` | same, `src/ui/app.rs` | **63** → 72 |
| `#[test]` in `src/ui/driver.rs` | same, `src/ui/driver.rs` | **29** → 34 |
| `#[test]` in `src/ui/markdown.rs` / `tasks.rs` / `detail.rs` | same | **24** / **16** / **23**, unchanged — the `MDWIDTHS` / `TASKWIDTHS` / `DETAILWIDTHS` floors |
| `NODEFAULT-UI` half-B spans, `src/ui/app.rs` run | `SCAN_MIN=9999 TYPES='Dashboard Filter Detail Refresh' sh $CHECKS/NODEFAULT-UI.sh` | **174** |
| `NODEFAULT-UI` half-B spans, `src/agents.rs` run | `SCAN_MIN=9999 HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' sh …` | **100**; floor raised 86 → **100** |
| `NODEFAULT-UI` half-B spans, `src/launch.rs` run | not measurable at `BASE` — `Outcome` does not exist; measured against the planning-time stub at **6**, and re-measured for real in task 5.4 |
| `Dashboard { … }` literals | the app run's own span count | **174** |
| `Attribution {` literals | `grep -rn 'Attribution {' src/` | **5** hits, of which only **2** are literals — both in `src/agents.rs`'s production slice; the other three are the `struct` declaration and two `-> Attribution {` return types. **0** test literals: `#[cfg(test)]` starts at `src/agents.rs:490` and `src/ui/app.rs:537`, below every hit |
| `Live {` literals | `grep -rn 'Live {' src/` | **41** — 1 production (`src/ui/mod.rs:158`), 32 in `src/ui/driver.rs`, 8 in `src/ui/mod.rs` |
| `start_collaborators` sites | `grep -rn 'start_collaborators' src/` | **4** — the definition, one call, and two doc-comment mentions |
| `Startup { … }` literals | `grep -rn 'Startup {' src/` | **2** — `ui::run` and the shared `run_wired_at` helper |
| `Command::new` | `grep -rn 'Command::new' src/ tests/` | **3** — `src/cli.rs:66` (doc comment), `src/cli.rs:74` (the one spawn), `tests/cli.rs:13` |
| `"claude"` under `src/ui/` | `grep -rn '"claude"' src/ui/` | **10**, every one inside a `#[cfg(test)]` slice; **0** after leg-4-style stripping |
| Herdr `pane split` argv | `herdr pane split --cwd /tmp --no-focus` | exit **2**, `usage: herdr pane split … --direction right\|down …` — `--direction` is required |
| Herdr `pane split` payload | `herdr pane split --cwd <p> --direction right --no-focus` | exit 0, `{"id":"cli:pane:split","result":{"pane":{…,"pane_id":"wD:pJ",…},"type":"pane_info"}}`; with no pane argument it split the **focused** pane |
| Herdr `agent start` payload | `herdr agent start hpo-probe --kind claude --pane wD:pJ` | exit 0, `{"id":"cli:agent:start","result":{"agent":{…,"name":"hpo-probe",…},"argv":["claude"],"type":"agent_started"}}` |
| Herdr `agent prompt` payload | `herdr agent prompt hpo-probe "/help"` | exit 0, `{"id":"cli:agent:prompt","result":{"agent":{…},"type":"agent_prompted"}}`; returned immediately with no `--wait` |
| Herdr name validation | `herdr agent start 'Agent-Launch' --kind claude --pane NOSUCH:pane` | exit **1**, `{"error":{"code":"invalid_agent_name","message":"agent name must start with a lowercase letter and contain only lowercase letters, digits, '-' or '_' (1-32 characters)"}}` on **stderr**, before the pane lookup |
| Herdr name collision | `herdr agent start hpo-probe …` a second time | exit **1**, `{"error":{"code":"agent_name_taken","message":"agent name hpo-probe is already used; candidates: …"}}` on stderr |
| Herdr `agent focus` targets | `herdr agent focus w8:p1` (the already-focused pane, so focus does not move) and `herdr agent focus term_65a34df386c314`; `herdr agent get` behaves identically and was used for the name form | pane id **ok**, name **ok**, terminal id `agent_not_found` exit 1 |
| Herdr agent kinds | `herdr agent start --help` | **22** kinds accepted by `--kind` |

| Herdr usage errors are not JSON | `herdr agent start goodname --kind nosuchkind --pane NOSUCH:pane` | exit **2**, plain text `unsupported interactive agent kind: nosuchkind` — an **argument** error, unlike the exit-1 domain errors above, which is why stderr is carried verbatim and never parsed |

Every Herdr row above was measured against the installed **0.8.2** on the reference machine.
Each probe pane and probe agent it created was closed before this file was written;
`herdr pane list` and `herdr agent list` afterwards showed none of them.

## Test-count floors — measured plus this change's enumerated new tests

| Group | Filter | Measured | New | Target |
|---|---|---|---|---|
| 3, 4, 5 | `launch::tests::` | 0 | 36 | 36 |
| 6 | `agents::tests::attribute::` | 17 | 2 | 19 |
| 7 | `ui::app::tests::` | 63 | 9 | 72 |
| 8 | `ui::driver::tests::` | 29 | 5 | 34 |
| 9 | `ui::list::tests::` | 25 | 4 | 29 |
| 10 | `ui::view::tests::` | 88 | 6 | 94 |
| 11 | `ui::tests::load::` | 9 | 1 | 10 |
| 11 | `cli::tests::` | 40 | 1 | 41 |
| 2, 12 | `ui::tests::wiring::` | 4 | 3 | 7 |

Library total: `840 + 36 + 2 + 9 + 5 + 4 + 6 + 1 + 1 + 3` = **907**, asserted in group 16 and
nowhere else. `launch::tests::`'s 36 break down as `decide::` 7, `argv::` 5, `prompt::` 2,
`pane_id::` 3, `run_request::` 10, `focus::` 2, `seam::` 4, and 3 shape tests.

**Thirty-eight landed tests are *modified* rather than added**, and a modified test moves no
count, so no `testcount` floor can see one. `EXTENDED` is what does: its pair list carries
`agent-attribution`'s eight landed pairs plus thirty-eight new ones, each naming the token its
extension must add, span-isolated to the named function's body. All thirty-eight were run
against `main` at planning time and all thirty-eight were **red**; the eight landed ones were
green, which is how the run is known to be discriminating rather than uniformly failing.

**`ui::tests::wiring::`'s floor is asserted only at group 12**, never between groups 2 and 12:
`testcount` counts tests that *passed*, and the three new ones are deliberately red for that
span.

## Group ordering — examined; one parallel group, the rest sequential

Groups 3, 4, and 5 all write `src/launch.rs`, so criterion 1 fails for every pair among them.
Groups 6 through 11 each write a different file (`src/agents.rs`, `src/ui/app.rs`,
`src/ui/driver.rs`, `src/ui/list.rs`, `src/ui/view.rs`, `src/ui/mod.rs`), so criterion 1 holds
for those pairs — **but the other two criteria do not hold anywhere**. Group 7 needs
`launch::decide` from group 3 and `Attribution::panes` from group 6; group 8 needs group 7's
`launch.pending`; groups 9, 10, and 11 each need group 7's twelfth field. And criterion 3
fails across the board: `cargo test` builds the whole crate, so a half-written `src/ui/view.rs`
fails group 9's `testcount` and vice versa, and neither failure is attributable to the group
that caused it.

**Group 14 is the exception and is marked parallel after group 0.** It edits `SPEC.md`,
`README.md`, and `AGENTS.md` and nothing else — no source file, no test, no check script —
and no gate in this plan reads any of the three, so a failure in group 14 cannot be mistaken
for a failure in any other group, and vice versa. Its content is fixed by the spec deltas
before implementation starts, so it depends on no code.

The two shipped **doc-comment** corrections this change also makes are deliberately **not** in
group 14, and that placement is what keeps the marker honest: `src/ui/app.rs`'s "The nine
outcomes" is corrected in group 7, which is where the number becomes seventeen and which owns
that file, and `src/cli.rs`'s "four more" in group 11, which owns that one. Left in group 14 they
would have made it share two files with groups 7 and 11 and depend on group 7's variant count —
three parallelism criteria failed at once.

The two shipped **doc-comment** corrections this change also makes are deliberately **not** in
group 14, and that placement is the reason the marker is honest: `src/ui/app.rs`'s "The nine
outcomes" is corrected in group 7, which is where the number becomes seventeen and which owns
that file, and `src/cli.rs`'s "four more" in group 11, which owns that one. Left in group 14 they
would have made it share two files with groups 7 and 11 and depend on group 7's variant count —
three parallelism criteria failed at once.

---

## Command-level checks this change adds and edits

Every block below was written and run against the real tree at planning time, with its exit
status and output recorded under it.

```sh
# LAUNCHSEAM — NEW in agent-launch. The launcher is confined to ONE module, on exactly
# AGENTSEAM's, WATCHSEAM's, and MDSEAM's single-file terms: the Herdr program is reachable by
# editing src/cli.rs, src/agents.rs, src/ui/mod.rs, and src/launch.rs, and no other file in
# the crate — tests/ included — may reach for it.
#
# Leg 1 is the file-scoped restatement of NOSPAWN-GREP, and it is the whole point of this
# check: src/launch.rs is the file whose JOB is to open a pane, so reaching for
# std::process::Command here — rather than asking the session through the injected handle —
# is the single most plausible way this change goes wrong. A launcher that spawns its own
# child is untestable and unobservable, and every other gate stays green while it does it.
# Leg 2 is AGENTSEAM leg 2's rule: the confined module is PLAIN DATA, so an Outcome never
# arrives at the view already styled. COMMENT-INCLUSIVE on purpose — a doc comment naming a
# view type is itself the coupling this forbids, so say "the view", never `Frame`.
# Leg 3 confines the Herdr handle itself. The pattern is HerdrCli|RealHerdrCli|agent_cli_via,
# not HerdrCli alone, and that is measured rather than cautious: agent_cli_via returns
# Arc<dyn HerdrCli> and inference hides the type, so a bare HerdrCli pattern reported OK on a
# tree carrying crate::cli::agent_cli_via(Path::new("/bin/herdr")) planted in src/ui/list.rs.
# src/ui/mod.rs is in ALLOWED because the composition root legitimately calls agent_cli_via;
# it is still forbidden to name HerdrCli itself by NOCLI-SHELL, which sweeps all of src/ui/,
# so the two checks compose without either one being weakened.
#
# Legs 1 and 2 read the PRODUCTION slice only, so a test that names Command::new to prove the
# launcher does NOT reach for it is not itself a violation. That makes Guard D below
# load-bearing rather than decorative: with two line-anchored #[cfg(test)] attributes the
# first one still truncates, but with ZERO (an attribute with a trailing space, say) prod()
# keeps the whole file and the legs get STRICTER, never weaker — the dangerous direction is
# a stray `#[cfg(test)]` ABOVE the real code, which the count guard is what catches.
set -u
LAUNCH="${LAUNCH:-src/launch.rs}"
ALLOWED="${ALLOWED-src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs}"
# Measured on the agent-launch tree: src/ + tests/ hold 26 *.rs files, four of them ALLOWED,
# so leg 3 searches 22. The floor is the realized count, not a round number below it: this
# repository only ever adds files, so a drop below 22 means a file was deleted or the find
# expression rotted, and either way leg 3's clean result would mean nothing.
MIN="${MIN:-22}"
fail() { echo "LAUNCHSEAM FAIL: $1" >&2; exit 1; }

[ -d src ] || fail "no src directory"
[ -d tests ] || fail "no tests directory - leg 3 would search half the crate"
[ -f "$LAUNCH" ] || fail "$LAUNCH missing - the subject is gone"
case "$ALLOWED" in *[![:space:]]*) ;;
  *) fail "ALLOWED is empty - leg 3 would forbid the declaration itself";; esac

# Guard A — positive control, checked BEFORE every leg: the module must actually define the
# launcher's entry point, or this is not the launcher and a clean result means nothing.
# ANCHORED on a definition form, never a bare substring: an unanchored `start` is satisfied
# by the doc comment on `none()` that mentions it.
grep -qE '^pub fn start\(' "$LAUNCH" \
  || fail "positive control - $LAUNCH defines no 'pub fn start('"

# Guard D — $LAUNCH holds EXACTLY ONE line-anchored #[cfg(test)], since prod() truncates at
# the first and legs 1 and 2 are blind below it. Measured elsewhere in this repository: with a
# single TRAILING SPACE after the attribute the anchored count is 0, and a file whose real
# code sits below a mis-spelled attribute is searched in full — safe — while a stray attribute
# ABOVE the real code hides everything. WIRED carries this guard for src/ui/mod.rs; LAUNCHSEAM
# needs it for the same reason.
c=$(grep -c '^#\[cfg(test)\]$' "$LAUNCH" || true)
[ "$c" -eq 1 ] || fail "$LAUNCH holds $c line-anchored #[cfg(test)] attributes, expected exactly 1 - legs 1 and 2 are blind below the first one"

prod() { awk 'BEGIN{p=1} /^#\[cfg\(test\)\]$/{p=0} p{print FILENAME":"FNR": "$0}' "$1"; }

# Guard B — the searched set is real. Excluded BY PATH, never by base name, so a future
# src/ui/launch.rs is still searched.
pruned=""
for f in $ALLOWED; do
  [ -f "$f" ] || fail "ALLOWED names $f, which does not exist - the exclusion is vacuous"
  pruned="$pruned ! -path $f"
done
# shellcheck disable=SC2086
n=$(find src tests -name '*.rs' $pruned | wc -l | tr -d ' ')
[ "$n" -ge "$MIN" ] || fail "searched only $n files (expected >= $MIN)"

# Leg 1 — the launcher spawns no process. It asks the session, through the handle.
p=$(prod "$LAUNCH" | grep -E 'process::Command|Command::new|Stdio' || true)
[ -z "$p" ] || { echo "LAUNCHSEAM FAIL (leg 1): $LAUNCH spawns a process:" >&2
                 echo "$p" >&2
                 echo "reach the session through Arc<dyn HerdrCli> instead - a child this module owns is unobservable" >&2
                 exit 1; }

# Leg 2 — the launcher names no view type, comments included.
r=$(prod "$LAUNCH" | grep -E 'ratatui|Frame|Rect|Buffer|Style' || true)
[ -z "$r" ] || { echo "LAUNCHSEAM FAIL (leg 2): $LAUNCH names a view type:" >&2
                 echo "$r" >&2
                 echo "Outcome is plain data - the view styles it, this module does not" >&2
                 exit 1; }

# Leg 3 — the Herdr handle is reached only from the allowed files.
HANDLE_RE='HerdrCli|RealHerdrCli|agent_cli_via'
# shellcheck disable=SC2086
hits=$(find src tests -name '*.rs' $pruned -print0 \
       | xargs -0 -I{} grep -nE "$HANDLE_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "LAUNCHSEAM FAIL (leg 3): the Herdr handle is reached outside:$ALLOWED" >&2
                    echo "$hits" >&2; exit 1; }

# Guard C — the exclusion is not vacuous in the other direction either: src/cli.rs must
# define the constructor leg 3 is confining, or leg 3 is excluding a file that says nothing.
# ANCHORED on a definition form for the same reason as Guard A.
grep -qE '^pub fn agent_cli_via\(' src/cli.rs \
  || fail "positive control - src/cli.rs defines no 'pub fn agent_cli_via('"

echo "LAUNCHSEAM OK: $n files searched (>= $MIN); no spawn and no view type in $LAUNCH's production slice; Herdr handle only in:$ALLOWED"
```

**Measured at planning time.** Against a scratch copy of `src/` + `tests/` carrying a
plausible `src/launch.rs` stub: `exit=0`, `LAUNCHSEAM OK: 22 files searched (>= 22); no spawn
and no view type in src/launch.rs's production slice; Herdr handle only in:src/cli.rs
src/agents.rs src/ui/mod.rs src/launch.rs`. Against unmodified `main`: `exit=1`,
`LAUNCHSEAM FAIL: src/launch.rs missing - the subject is gone` — the expected RED, since the
module does not exist yet. Five plants, each observed red and reverted:
`std::process::Command::new("herdr")` in `src/launch.rs` → `exit=1`, `LAUNCHSEAM FAIL (leg 1)`;
`use ratatui::layout::Rect;` in `src/launch.rs` → `exit=1`, `LAUNCHSEAM FAIL (leg 2)`;
`crate::cli::agent_cli_via(p)` in `src/ui/list.rs` → `exit=1`, `LAUNCHSEAM FAIL (leg 3)`;
`MIN=23` → `exit=1`, `LAUNCHSEAM FAIL: searched only 22 files (expected >= 23)`;
`pub fn start(` renamed → `exit=1`, `LAUNCHSEAM FAIL: positive control`. Every other failure
path was swept too — Guard A rename, two `#[cfg(test)]` attributes, Guard C rename, an
`ALLOWED` naming a missing file, an empty `ALLOWED`, and a missing `tests/` — and **all exit
1**; there is no print-FAIL-exit-0 path in the block.

**`NOIO-VIEW.sh`** (extracted from `.../2026-09-05-tasks-tab/tasks.md`), one replacement:

```
-IO_RE='std::fs|std::io|std::env|std::process|std::net|File::|read_to_string|tasks::read|state::read|Command'
+IO_RE='std::fs|std::io|std::env|std::process|std::net|File::|read_to_string|tasks::read|state::read|state::record|launch::start|Command'
```

`crate::state::record` matches none of the other alternatives — not `std::fs` (the write lives
in `src/state.rs`, not at the call site), not `state::read` — and nothing in the block matches
`launch::start`.

**Measured at planning time.** Edited, against the real `src/`: `exit=0`,
`NOIO-VIEW OK: 8 pure files carry no I/O API; positive control matched`. Edited, against a
copy carrying `fn zz(a:&str,b:&str){let _=crate::state::record(None,a,b);}` appended to
`src/ui/app.rs`: `exit=1`, `NOIO-VIEW FAIL: I/O API in a pure view file:`. Edited, against a
copy carrying `crate::launch::start(cli, r, k, s)` appended to `src/ui/driver.rs`: `exit=1`,
same first line. The **unedited** block misses both plants (`exit=0` under each), which is what
makes the edit load-bearing rather than decorative. Edited, against the scratch wired tree
whose `src/ui/driver.rs` names `crate::launch::Launcher`: `exit=0` — the trait name is not the
starter and is correctly not matched.

**`WIRED.sh`** (extracted from `.../2026-09-06-agent-polling/tasks.md`, carrying
`agent-attribution`'s five replacements), four further replacements:

1. after `STATE="${STATE:-src/state.rs}"` insert `LAUNCH="${LAUNCH:-src/launch.rs}"`;
2. after the `[ -f "$STATE" ] || fail …` line insert
   `[ -f "$LAUNCH" ] || fail "$LAUNCH missing - leg 1's ninth name would point at nothing"`;
3. after the `pub fn read\(` positive control insert
   `grep -qE '^pub fn start\(' "$LAUNCH" || fail "positive control - $LAUNCH defines no 'pub fn start('"`;
4. leg 1's name list gains `'launch::start'` as its ninth entry, and immediately **before**
   leg 4 a new **leg 6** is inserted:

```sh
# Leg 6 — NEW in agent-launch. The launcher is started with the CONFIGURED agent kind, not a
# literal. Two halves, and both are needed:
#   (i)  $MOD's production slice must NAME config.agent_kind. Searched over the whole
#        production slice rather than `run`'s body, because the value is threaded in
#        start_collaborators, which `run` never sees.
#   (ii) the "claude" literal appears in NO production slice under $UIDIR. agent_kind already
#        defaults to "claude" in src/config.rs, so a start_collaborators that spells the
#        default itself can still name config.agent_kind elsewhere in the file and pass half
#        (i) while shipping a dashboard that ignores the operator's setting.
# SCOPED exactly like leg 4: under $UIDIR only, stripping at the first line-anchored
# #[cfg(test)]. Measured: the literal appears TEN times under src/ui/ today, every one inside
# a test slice, and ZERO times after stripping.
code "$MOD" | grep -q 'config.agent_kind' \
  || fail "leg 6: $MOD's production slice does not name config.agent_kind - the launcher would run some other agent than the configured one"
k=$(find "$UIDIR" -name '*.rs' -print0 \
    | xargs -0 -I{} sh -c 'awk "BEGIN{p=1} /^#\\[cfg\\(test\\)\\]\$/{p=0} p{print FILENAME\":\"FNR\": \"\$0}" "$1"' _ {} \
    | grep -E '"claude"' || true)
[ -z "$k" ] || { echo "WIRED FAIL (leg 6): the \"claude\" literal appears under $UIDIR:" >&2
                 echo "$k" >&2
                 echo "pass config.agent_kind instead - the default lives in src/config.rs, not here" >&2
                 exit 1; }
```

and the closing echo becomes `WIRED OK: nine names present in $MOD; run resolves
state::state_dir; $MOD names config.agent_kind; 'pub fn run()' is $lines lines with no branch
and no loop; no "herdr" and no "claude" literal under $UIDIR`.

**Measured at planning time**, seven states, using a scratch copy of `src/` carrying the
intended wiring. On `main`: `exit=1`,
`WIRED FAIL: src/launch.rs missing - leg 1's ninth name would point at nothing`. On the wired
copy: `exit=0`, the nine-name OK line above. With `launch::start` removed from
`src/ui/mod.rs`: `exit=1`, `WIRED FAIL: leg 1: … does not name launch::start`. With
`config.agent_kind.clone()` replaced by `"claude".to_string()`: `exit=1`,
`WIRED FAIL: leg 6: … does not name config.agent_kind` — half (i) fires first; with
`config.agent_kind` still named elsewhere in the slice, half (ii) fires instead:
`WIRED FAIL (leg 6): the "claude" literal appears under src/ui:` naming
`src/ui/mod.rs:145`. With `const ZZ: &str = "claude";` at the top of `src/ui/view.rs`:
`exit=1`, leg 6 half (ii). On the untouched wired copy, whose ten `"claude"` occurrences are
all test-side: `exit=0` — leg 6 does not trip on them. With `src/launch.rs`'s `pub fn start(`
renamed: `exit=1`, `WIRED FAIL: positive control - src/launch.rs defines no 'pub fn start('`.

**`NOBLOCK.sh`** (extracted from `.../2026-09-06-live-refresh/tasks.md`, carrying
`agent-polling`'s `src/agents.rs` additions), four replacements. `src/launch.rs` is the crate's
**fourth** seam module and leg 3 is the only leg that can see inside it — leg 1 is scoped to
`src/ui/driver.rs` and leg 2 to `src/ui/`:

1. Guard D's list becomes
   `for f in src/watch.rs src/refresh.rs src/agents.rs src/launch.rs; do`;
2. beside the existing `src/agents.rs` existence guard, insert
   `[ -f src/launch.rs ] || fail "src/launch.rs missing - leg 3 has a fourth seam module to sweep"`;
3. after the `src/agents.rs` leg-3 arm, a fourth arm mirroring it exactly — Guard E taking
   `grep -n 'fn drain' | tail -1` (never `head -1`: `src/launch.rs` declares `fn drain` three
   times and the trait's abstract signature necessarily precedes every spawn), the `awk` cut at
   the single non-comment `thread::spawn`, the `BLOCK3_RE` search over the half above it, and
   the `try_recv` positive control;
4. the leg-3 echo becomes `NOBLOCK OK (leg 3): src/watch.rs never blocks; src/refresh.rs,
   src/agents.rs and src/launch.rs block only after thread::spawn`.

**Measured at planning time**, seven states against a scratch copy carrying a `src/launch.rs`
stub. Edited, against the real `src/` (no `src/launch.rs`): `exit=1`, Guard D. Edited, against
the wired copy: `exit=0`, the new leg-3 OK line. Four plants on the wired copy, each `exit=1`:
`Launcher::drain` using `recv_timeout(150ms)` instead of `try_recv` → leg 3,
`NOBLOCK FAIL (leg 3): src/launch.rs blocks before its thread::spawn`; the concrete `fn drain`
moved below `thread::spawn` → Guard E, `declares drain at line 145, below its thread::spawn at
line 133`; a second line-anchored `#[cfg(test)]` → Guard D, `holds 2 … expected exactly 1`;
`try_recv` deleted → the positive control. The **unedited** block is `exit=0` under all four,
which is what makes the edit load-bearing. A `head -1` variant of replacement 3 reports OK under
the moved-`drain` plant, which is why it is `tail -1`.

**`EXTENDED.sh`**'s pair list, passed explicitly as `PAIRS`, is
`agent-attribution`'s eight plus these **thirty-eight**, forty-six in all. The count is written
out because it is load-bearing twice: the block's own guard fires only below eight, and every
invocation additionally asserts the OK line reports forty-six, so a pair silently dropped to
match a stale number is caught rather than absorbed:

```
src/ui/app.rs:no_action_mutates_changes:LaunchApply
src/ui/app.rs:quit_keys_and_their_near_misses:LaunchContinue
src/ui/app.rs:esc_dismisses_one_layer_at_a_time:launch
src/ui/app.rs:enter_and_esc_map_to_routes:launch
src/ui/app.rs:non_key_events_are_ignored:Paste("a"
src/ui/app.rs:dashboard_is_clone_and_eq_with_agents:launch
src/ui/app.rs:attribution_follows_adopt_by_name:panes
src/ui/app.rs:attribution_ignores_the_filter:panes
src/ui/view.rs:agents_change_no_pixel:g focus
src/ui/view.rs:the_unattributed_count_is_the_last_hint:a/c/s launch
src/ui/view.rs:the_count_drops_before_the_key_hints:g focus
src/ui/view.rs:the_count_survives_a_filter:g focus
src/ui/view.rs:the_count_is_reported_with_an_empty_list:reachable
src/ui/view.rs:frame_rows_at_60_and_120:reachable
src/ui/view.rs:footer_drops_whole_hints:reachable
src/ui/view.rs:an_unreachable_socket_renders_the_agentless_pane:launch
src/ui/view.rs:a_prompt_longer_than_the_footer_keeps_its_tail:reachable
src/ui/view.rs:one_column_frame_does_not_panic:reachable
src/ui/list.rs:refresh_and_change_problems_in_order:launch
src/ui/list.rs:refresh_problems_lead_the_rows:launch
src/ui/list.rs:row_order_is_problems_then_active_then_separator_then_archived:launch
src/ui/driver.rs:an_inert_live_tier_takes_nothing_and_requests_nothing:launch::none
src/ui/driver.rs:ctrl_c_ends_the_loop:launcher
src/ui/driver.rs:an_unreachable_socket_is_not_a_problem_row:launch.problems
src/ui/driver.rs:timeouts_are_not_events:launcher
src/ui/view.rs:the_prompt_replaces_the_hints_while_filtering:reachable
src/ui/view.rs:an_accepted_query_leads_the_hint_list:reachable
src/agents.rs:an_unmatched_in_scope_agent_is_counted:panes
src/agents.rs:the_name_tier_never_reads_the_kind:panes
src/agents.rs:a_terminal_title_attributes_nothing:panes
src/agents.rs:every_empty_input_is_total:panes
src/agents.rs:precedence_is_total_and_order_independent:panes
src/agents.rs:one_agent_per_change_keeps_its_status:panes
src/ui/mod.rs:the_real_wiring_polls_a_scratch_herdr:pane split
src/ui/mod.rs:a_polled_agent_reaches_a_rendered_badge:a/c/s launch
src/ui/mod.rs:an_unreachable_scratch_herdr_is_a_standalone_tui:launch
src/ui/mod.rs:no_repository_still_polls_for_agents:g focus
src/ui/mod.rs:tasks_tab_is_read_only:launcher
```

**Measured at planning time.** The full forty-six-pair list against `main`: `exit=1`,
`EXTENDED FAIL: a landed test was not extended:` followed by **exactly** the thirty-eight new
pairs and none of the eight landed ones — so the run is discriminating rather than uniformly
red, and every landed test named above exists under exactly that name (a renamed one would
have produced `named test not found` instead). Two tokens were rejected during that run and
replaced: `attribution_ignores_the_filter:reachable` and `agents_change_no_pixel:reachable`
were both already green, because both spans construct an `AgentSnapshot` and therefore already
spell `reachable`; they are `panes` and `g focus` above. The block's own
`len(pairs) < 8` guard still fires if the list is gutted, and each invocation additionally
asserts the OK line reports **46**.

**Invocation floors that move, with no edit to the block:**

- `NOSPAWN-GREP`: `MIN` **22 → 23**. `NOLIT-CHANGE`: `MIN` **22 → 23**. `AGENTSEAM`: `MIN`
  **22 → 23** and `ALLOWED` gains `src/launch.rs`. `WATCHSEAM`: `MIN` **24 → 25**. `NOSLEEP`:
  `MIN` **25 → 26**, `SLEEP_MIN` stays **5**.
- `WIDTHS`: `WIDTHS_MIN` **88 → 94**. `LISTWIDTHS`: `LIST_MIN` **25 → 29**. Both passed
  explicitly.
- `READSEAM`: `UI_MIN` **→ 10**, its measured realized count, passed explicitly for the first
  time — bare it runs at the block default of 7. `MDSEAM`: `MIN` **→ 23**, the file count after
  `src/launch.rs` lands; bare it runs at 16. Both have been invoked bare since `detail-view`,
  which is the same defect the widths gates had.
- `MDWIDTHS`, `TASKWIDTHS`, `DETAILWIDTHS`: unchanged, but invoked as `MD_MIN=24`,
  `TASK_MIN=16`, and `DETAIL_MIN=23` — their measured realized counts. Invoked bare they run
  at 23, 14, and 19, which are below the tree's real counts and therefore bite nothing.
- `NODEFAULT-UI`: the app run's `TYPES` gains `Launch` and its `SCAN_MIN` moves **165 → 174**
  (re-measured, then raised again in task 7.4 once the twelfth field's literals land); the
  agents run's `SCAN_MIN` moves **86 → 100**; and a **third** run is added,
  `HOMEFILE=src/launch.rs TYPES='Outcome'`, whose `SCAN_MIN` is measured in task 5.4 from its
  own FAIL line at `SCAN_MIN=9999` and set to at least **20** — `launch::tests::` builds an
  `Outcome` in most of its thirty-six tests, so a realized count below twenty means the scan
  is not seeing them.
- `OPENSPEC-UNTOUCHED`: **no edit and no re-default.** Measured with
  `BASE=$(git rev-parse HEAD) CHANGE=agent-launch` after the change directory existed:
  `exit=0`, `OPENSPEC-UNTOUCHED OK`; the `[ -d ]` guard passed, so the exclusion is not
  vacuous.
- `GATE-MECH1.py`: **no edit.** Its file-count floor is hardcoded at 8 (line 98) rather than
  parameterised, and the tree holds 23 files today and 24 after this change.

---

## 0. Baseline, checks, and the scratchpad
<!-- kind: operational -->

- [x] 0.1 CHECK: Record the starting state by **measuring**, never by copying a number from
      this file. `export BASE=$(git rev-parse HEAD)`; then run each command in the "Measured
      at planning time" table and record its output verbatim.
      **Red when:** `BASE` is empty or not a commit, or any measured figure differs from the
      table. Prefer `BASE=$(git rev-parse HEAD) CHANGE=agent-launch sh
      $CHECKS/OPENSPEC-UNTOUCHED.sh` at each invocation over an exported variable:
      re-exporting from the current `HEAD` after a fresh shell silently puts this change's own
      commits inside the baseline.

- [x] 0.2 CHANGE: Write `LAUNCHSEAM.sh` to `$CHECKS` from the block above, and extract the
      other twenty-nine files from the archived tasks files that last reproduced each in full:
      `EXTENDED.sh` from `.../2026-09-06-agent-attribution/tasks.md`; `AGENTSEAM.sh`,
      `WIRED.sh` from `.../2026-09-06-agent-polling/tasks.md`; `WATCHSEAM.sh`,
      `READONLY-UI.sh`, `NOBLOCK.sh`, `NOSLEEP.sh`, `OPENSPEC-UNTOUCHED.sh` from
      `.../2026-09-06-live-refresh/tasks.md`; `NOSPAWN-GREP.sh`, `READSEAM.sh`,
      `NOCLI-SHELL.sh`, `NOLIT-CHANGE.sh`, `MDSEAM.sh`, `NOTABSEAM.sh`, `WIDTHS.sh`,
      `DETAILWIDTHS.sh`, `NODEFAULT-UI.sh` from `.../2026-09-05-detail-view/tasks.md`;
      `NORAW-GREP.sh`, `LISTWIDTHS.sh`, `MDWIDTHS.sh`, `NOWAIVER.sh`, `TESTCOUNT.sh`,
      `DEPS.sh`, `GRAPH-SNAP.sh` from `.../2026-09-05-markdown-viewer/tasks.md`;
      `NOIO-VIEW.sh`, `TASKSEAM.sh`, `TASKWIDTHS.sh` from `.../2026-09-05-tasks-tab/tasks.md`;
      `GATE-MECH1.py`, `NOJSON-SEAM.sh` from `.../2026-09-04-changes-from-cli/tasks.md`.
      **Red when:** `ls -1 "$CHECKS" | wc -l` is not 30, or `$CHECKS/GATE-MECH1.py` is absent
      because the extractor read only each block's first line — its label sits on the block's
      **second** line behind a shebang. Use a scratch directory this session owns.

- [x] 0.3 CHANGE: Re-apply the accumulated edits that live in earlier changes' tasks files as
      string replacements rather than in any reproduced block: `agent-polling`'s
      `NODEFAULT-UI.sh` `HOMEFILE` parameterisation, its `|| exit 1` repair, and its OK-line
      suffix; `NOSLEEP.sh`'s leg 2b gaining `src/agents.rs`; `NOBLOCK.sh`'s leg 3 gaining
      `src/agents.rs`; `OPENSPEC-UNTOUCHED.sh`'s `$CHANGE` parameterisation; and
      `agent-attribution`'s `NOIO-VIEW.sh` `state::read` replacement and `WIRED.sh` five.
      Print each resulting `diff` before running.
      **Red when:** `NODEFAULT-UI.sh` still names `$SRC/ui/app.rs` in its positive control, or
      `WIRED.sh` holds no `leg 5`.

- [x] 0.4 CHECK: Run all thirty extracted gates against the unmodified tree at `BASE`, at the
      invocations this change uses, and record each exit status and its first line verbatim.
      **Expected:** `LAUNCHSEAM` red on its missing subject; `WIRED` and `NOBLOCK` red on the
      same;
      `EXTENDED` red on exactly the thirty-eight new pairs; `MIN=23 NOSPAWN-GREP`,
      `MIN=23 NOLIT-CHANGE`, `MIN=23 AGENTSEAM`, `MIN=25 WATCHSEAM`, and `MIN=26 NOSLEEP` red
      on their raised file counts; `WIDTHS_MIN=94 WIDTHS` red with `found 88`;
      `LIST_MIN=29 LISTWIDTHS` red with `found 25`; the third `NODEFAULT-UI` run red on its
      positive control; **`DEPS` and `GRAPH-SNAP` red for two different inherited reasons**, with
      each one's exact text recorded so a reviewer does not mistake either for this change's
      doing — `DEPS` on leg 2a
      (`AssertionError: normal deps are ['notify', …], expected [...]` then `DEPS FAIL: leg 2a`),
      and `GRAPH-SNAP` **past** its snapshot diff on a hardcoded platform assertion
      (`GRAPH-SNAP FAIL: macOS/Linux differ by [fsevent-sys inotify inotify-sys linux-raw-sys ],
      expected [linux-raw-sys ]`) — `tests/fixtures/build-graph.txt` **was** regenerated by
      `live-refresh` (commit `574b87d`), contrary to what `HANDOFF.md` records; and
      every other gate green.

- [x] 0.5 CHANGE: Apply this change's own edits — `NOIO-VIEW.sh`'s one replacement,
      `WIRED.sh`'s four, and `NOBLOCK.sh`'s four, exactly as written above — printing each
      resulting `diff` before running.
      **Expected after this task:** `NOIO-VIEW` green; `WIRED` and `NOBLOCK` both still red, each
      on its `src/launch.rs` guard rather than on a leg.

- [x] 0.6 CHECK: Prove `NOIO-VIEW`'s edit can fail. Plant
      `fn zz(a:&str,b:&str){let _=crate::state::record(None,a,b);}` at the end of
      `src/ui/app.rs` and `fn zz2(){let _=crate::launch::start;}` at the end of
      `src/ui/driver.rs`, run `sh $CHECKS/NOIO-VIEW.sh`, record exit **1** and the FAIL line
      for each; run the **unedited** extraction against the same plants and record exit **0**;
      remove the plants and record exit **0** for the edited block.
      **Red when:** the edited check is green under a plant, or the unedited one is red.

- [x] 0.7 CHECK: Prove `OPENSPEC-UNTOUCHED`'s exclusion is still scoped. Plant an empty
      `openspec/specs/STRAY.tmp`, run with `CHANGE=agent-launch` and record exit **1** naming
      that path; remove it; then run with `CHANGE=nope` and record exit **1** naming the
      vacuous exclusion.

- [x] 0.8 VERIFY: `git status --porcelain` shows no change under `openspec/` outside
      `openspec/changes/agent-launch/`, and
      `BASE=$(git rev-parse HEAD) CHANGE=agent-launch sh $CHECKS/OPENSPEC-UNTOUCHED.sh` is
      green. Commit nothing in this group — it writes only to `$CHECKS`.

## 1. The skeleton every later group compiles against
<!-- kind: refactor -->

Structure only: inert signatures, no behaviour. The evidence is that the 840 landed tests stay
green while the tree grows the API groups 2 to 11 fill in. See design.md → Test Strategy for
why this precedes the outer-loop RED.

- [x] 1.1 CHARACTERIZE: `. $CHECKS/TESTCOUNT.sh; testcount --lib '' 840` and record the count,
      so the landed suite is green before any file is touched.

- [x] 1.2 REFACTOR: Add `src/launch.rs` and `pub mod launch;` with the inert shapes from
      design.md → Contracts: `Intent`, `Request`, `Decision`, `Outcome`, a `decide` returning
      `Decision::Nothing`, a `pane_id` returning `Err`, the `Launcher` trait, `none()`, and a
      `start` that spawns a worker answering nothing.

- [x] 1.3 REFACTOR: Add the inert shapes elsewhere: `Attribution::panes` (always empty);
      `ui::app::Launch` and `Dashboard::launch`; `Action`'s four variants mapped by
      `action_for` but with `apply` arms that do nothing; `Live::launcher`;
      `Collaborators::launcher`; `start_collaborators`'s fourth parameter (unread), and
      `start_collaborators` calling `launch::start` with `config.agent_kind` unconditionally, so
      `WIRED` leg 1's ninth name and leg 6 half (i) are satisfied by the skeleton rather than
      eleven groups later.

- [x] 1.4 REFACTOR: Fix every site the new shapes break, counted rather than estimated: the
      **5** `Attribution {` literals, the **41** `Live {` literals, and every `Dashboard { … }`
      literal in the crate — `grep -rn 'Attribution {' src/`, `grep -rn 'Live {' src/`, and
      `NODEFAULT-UI`'s 174-span count are the three commands that enumerate them. Rename
      `ui::driver::tests::live_destructures_into_exactly_three_fields` to `…_four_fields` and
      `ui::app::tests::dashboard_destructures_into_exactly_ten_fields` to `…_twelve_fields`;
      the second name has been stale at eleven since `agent-attribution`.

- [x] 1.5 CHECK: `MIN=22 sh $CHECKS/LAUNCHSEAM.sh` and `sh $CHECKS/WIRED.sh` must both be
      **green** for the first time, `WIRED` including leg 6. Then plant
      `"claude".to_string()` in place of `config.agent_kind` in `start_collaborators`, re-run
      `WIRED`, record exit **1** naming leg 6, and revert; then plant
      `std::process::Command::new("x")` in `src/launch.rs`, re-run `LAUNCHSEAM`, record exit
      **1** naming leg 1, and revert.
      **Red when:** either plant is green.

- [x] 1.6 VERIFY: `testcount --lib '' 840` — the landed suite is unchanged — then
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo fmt --all -- --check`. Commit.

## 2. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

The headline scenario is `agent-launch` → "Pressing `a` splits a pane, starts an agent, and
sends the prompt". Its assertions are chosen so each can fail and no two plants fail the same
set; see design.md → Test Strategy.

- [x] 2.1 RED: Extend `ui::tests::wiring`'s harness — test-side code only, under
      `#[cfg(test)]`, touching no production file — honouring design.md → Test Boundaries:
      a `ScratchDir` repository holding the active change `2fa-support`; a second `ScratchDir`
      state directory, empty; a `Config` whose `agent_kind` is `codex`; a `herdr_script`
      parameterised on the **canonicalized** repository root that logs every argument vector
      and answers `pane split`, `agent start`, `agent prompt`, `agent focus`, and `agent list`;
      and `testutil::Stages`, a staged event source yielding timeouts until each stage's
      predicate holds, pressing that stage's key, and finishing with `q`.
      **Red when:** the script embeds `scratch.path()` rather than
      `testutil::canonical(scratch.path())` — on macOS `/var` is a symlink to `/private/var`,
      so the uncanonicalized form makes the containment test pass where production fails.

- [x] 2.2 RED: Write `ui::tests::wiring::a_keypress_launches_an_agent`, driving `run_wired` at
      120x20 and again at 60x20: wait for the log's first `agent list` entry, press `a`, wait
      for **three entries that are not `agent list`**, press `q`. Assert those three are the
      launch calls in order, `agent start`'s
      `--pane` equal to the pane id `pane split` printed, `--kind codex`,
      **Red when:** any predicate or assertion counts total log entries. The poller writes
      `agent list` to the same log every second, so an absolute count fires early and the
      ordering assertion then fails for a reason unrelated to the launcher. the scratch state
      directory's `agent-names.toml` holding `c-2fa-support = "2fa-support"`, the returned
      dashboard's `agent_names.names` holding the same pair, `launch.pending` `None`,
      `launch.problems` empty, the footer carrying `a/c/s launch  g focus`, and a
      `testutil::snapshot` pair over the repository showing byte-identity.

- [x] 2.3 RED: Write `ui::tests::wiring::g_focuses_the_agent_the_launch_started`, with
      `agent_kind` **`gemini`** rather than `codex` so the two runs disagree and a hardcoded
      kind fails one of them — the scratch
      `herdr`'s `agent list` branch begins reporting the started agent once `agent start` has
      been called, and the stages are `a`, then wait for that agent to appear in a poll, then
      `g`, then wait for a **fourth non-`agent list`** entry, then `q` — and
      `ui::tests::wiring::an_unreachable_socket_leaves_every_key_inert`, driven with a `herdr`
      path that does not exist and stages `a`, `g`, `q`.

- [x] 2.4 CHECK: Confirm all three fail because the behaviour is missing, not because the
      harness is misconfigured: each run must reach `Ok(dashboard)`, and the first two must
      fail on the log, the mapping, and the footer assertions alone.
      **Red when:** any fails on `StartError`, on a deadline with an empty `herdr` log, or on a
      panic — all three are harness faults. Record the exact assertion messages; task 12.1
      requires the identical ones.

- [x] 2.5 VERIFY: `cargo test --all-features` fails on **exactly** these three tests. Commit.

## 3. `launch::decide`, the argument vectors, and `pane_id`
<!-- kind: behavior -->

- [x] 3.1 RED: Write `launch::tests::decide::`'s seven tests — `an_unreachable_socket_makes_
      every_key_inert`, `no_change_means_no_launch_and_no_agent_means_no_focus`,
      `each_intent_carries_its_own_change_and_name`, `a_live_derived_name_is_refused`,
      `every_combination_is_total`, `decide_reads_nothing_but_its_arguments`, and
      `the_derived_name_is_state_agent_name` — one per `agent-launch` spec scenario in the
      first requirement.
      **Red when:** any of them builds a `Change` value or touches a filesystem; `decide`
      takes an `Option<&str>` and a `&[&str]` per design.md → Boundaries, and
      `MIN=23 sh $CHECKS/NOLIT-CHANGE.sh` is what catches the first.

- [x] 3.2 RED: Write `launch::tests::argv::`'s five — `split_args_are_exact`,
      `start_args_are_exact`, `prompt_args_are_exact`, `focus_args_are_exact`, and
      `a_non_utf8_repo_root_is_lossy` — and `launch::tests::prompt::`'s two,
      `each_intent_has_its_own_opsx_command` and `the_prompt_text_is_one_argument`,
      asserting the exact vectors design.md → Contracts gives, including the non-UTF-8 root's
      lossy rendering and the prompt text being one element containing exactly one space.

- [x] 3.3 RED: Write `launch::tests::pane_id::`'s three —
      `a_well_formed_envelope_yields_the_pane_id`, `every_unusable_payload_is_an_error` (the
      eight-input sweep), and `an_error_envelope_is_reported_by_code_and_message`.

- [x] 3.4 GREEN: Implement `decide` in the order the spec fixes — reachability, focus, the
      selection, the collision, then `Go` — plus the four argv builders, `prompt_text`, and
      `pane_id`.

- [x] 3.5 REFACTOR: Extract the derived-name call to one place beside `decide`, so
      `state::agent_name` is named once in the module — or record that the implementation
      already names it once.

- [x] 3.6 VERIFY: `testcount --lib 'launch::tests::decide::' 7`,
      `testcount --lib 'launch::tests::argv::' 5`, `testcount --lib 'launch::tests::prompt::' 2`,
      `testcount --lib 'launch::tests::pane_id::' 3`, then `MIN=23 sh $CHECKS/NOSPAWN-GREP.sh`,
      `MIN=23 sh $CHECKS/NOLIT-CHANGE.sh`, and `MIN=22 sh $CHECKS/LAUNCHSEAM.sh`. Commit.

## 4. `launch::run_request` — three calls and every failure path
<!-- kind: behavior -->

- [x] 4.1 RED: Write `launch::tests::run_request::`'s ten and `launch::tests::focus::`'s two,
      one per `agent-launch` spec scenario in the third, fourth, and fifth requirements, over a
      recording `HerdrCli` fake answering by argument vector and — for the four that record —
      a real `testutil::ScratchDir` state directory.
      Names: `the_three_calls_appear_in_order_with_the_splits_pane_id`,
      `an_unusable_payload_stops_before_agent_start`, `a_failed_split_leaves_nothing_behind`,
      `a_failed_start_leaves_the_pane_and_names_it`,
      `a_failed_prompt_leaves_a_recorded_agent`, `a_failed_recording_does_not_undo_the_start`,
      `a_name_past_the_cap_is_truncated_hashed_and_recorded`,
      `a_legal_but_derived_name_is_recorded_too`, `an_unchanged_name_writes_no_file`,
      `a_collision_herdr_sees_is_reported_with_its_reason`, `a_focus_request_is_one_call`, and
      `a_failed_focus_is_reported`.

- [x] 4.2 CHECK: Confirm every failure is the missing behaviour: each names the argument vector
      or the outcome field in its diff, and none fails to compile or to find its scratch
      directory.

- [x] 4.3 GREEN: Implement `run_request`: derive the name, split, parse, start, record, prompt,
      stopping at the first failure and carrying `CliError`'s stderr verbatim. `Focus` is one
      call. **No `pane close` is issued on any path** (design.md → Decisions 3).

- [x] 4.4 CHECK: Concentration point — the 32-character cap. Confirm
      `a_name_past_the_cap_is_truncated_hashed_and_recorded` asserts the argv name is ≤ 32
      characters **and** matches `[a-z][a-z0-9_-]{0,31}` **and** equals `state::agent_name`'s
      output, and that `an_unchanged_name_writes_no_file` asserts the state directory is
      byte-identical rather than merely that no error was returned.

- [x] 4.5 VERIFY: No refactor was needed — `run_request` is one linear sequence with one early
      return per call. `testcount --lib 'launch::tests::run_request::' 10`,
      `testcount --lib 'launch::tests::focus::' 2`, then
      `UI_MIN=11 EXTRA='src/watch.rs src/refresh.rs src/agents.rs src/launch.rs' sh
      $CHECKS/READONLY-UI.sh` — green for the first time — and a planted
      `std::fs::write(p, "x")` in `src/launch.rs` re-run against it, recorded red and
      reverted. Commit.

## 5. The launcher seam and the crate's third worker thread
<!-- kind: behavior -->

- [x] 5.1 RED: Write `launch::tests::seam::`'s four — `the_inert_launcher_answers_nothing`,
      `the_real_launcher_answers_on_a_later_drain`, `dropping_the_launcher_stops_its_worker`,
      and `the_launcher_writes_only_the_state_directory` — plus the three shape tests
      (`Outcome`'s exhaustive destructuring and the exhaustive matches over `Intent` and
      `Request`).
      **Red when:** any of them calls `thread::sleep`. `NOSLEEP` will **not** catch that on its
      own — its leg 1 accepts a deadline-bounded sleep anywhere and its `SLEEP_MIN` is a
      lower bound, so a sixth site passes — so this is a review point rather than a gate: the
      in-flight claim is proved with a gated fake blocking on a channel the test releases and a
      `yield_now` deadline poll, per design.md → Test Strategy, and a `thread::sleep` here would
      be an elapsed-time assertion about a worker thread.

- [x] 5.2 GREEN: Give `RealLauncher` its body — `request` sends, `drain` `try_recv`s — and
      `worker_body` its loop, written **below** the single `thread::spawn`.

- [x] 5.3 CHECK: Contract gate — re-inspect `src/launch.rs`'s published shape against
      design.md → Contracts, and confirm `Launcher` carries no `pending_in`, so
      `watch::soonest` still takes two arguments and no landed loop test's timeout changes.

- [x] 5.4 VERIFY: No refactor was needed — the seam is `refresh::start`'s shape, copied.
      `testcount --lib 'launch::tests::seam::' 4` and
      `testcount --lib 'launch::tests::' 36`; `sh $CHECKS/NOBLOCK.sh` with leg 3, Guard D, and
      Guard E extended to `src/launch.rs`; `SLEEP_MIN=5 MIN=26 sh $CHECKS/NOSLEEP.sh`;
      `MIN=25 sh $CHECKS/WATCHSEAM.sh`; and
      `SCAN_MIN=9999 HOMEFILE=src/launch.rs TYPES='Outcome' sh $CHECKS/NODEFAULT-UI.sh` to
      read the realized span count from its FAIL line, then re-run at that figure (at least
      **20**) and record it. Then plant `#[derive(Default)]` on `Outcome`, re-run, record exit
      **1**, and revert. Commit.

## 6. `Attribution::panes`
<!-- kind: behavior -->

- [ ] 6.1 RED: Write `agents::tests::attribute::the_pane_map_and_the_badge_map_agree` and
      `::a_tie_keeps_the_first_agents_pane`. **In the same task**, extend the six landed
      `attribute` tests `EXTENDED`'s pair list names — `an_unmatched_in_scope_agent_is_counted`,
      `the_name_tier_never_reads_the_kind`, `a_terminal_title_attributes_nothing`,
      `every_empty_input_is_total`, `precedence_is_total_and_order_independent`, and
      `one_agent_per_change_keeps_its_status` — with their `panes` assertions, so every new
      assertion is observed red before 6.2 makes it pass.

- [ ] 6.2 GREEN: Fill `panes` in the same fold that fills `badges`, updated by the same
      strictly-greater comparison so a tie keeps the first agent.

- [ ] 6.3 VERIFY: No refactor was needed — `panes` is filled inside the fold that already
      exists. `testcount --lib 'agents::tests::attribute::' 19`,
      `testcount --lib 'agents::tests::attribute::the_pane_map_and_the_badge_map_agree' 1`,
      `testcount --lib 'agents::tests::attribute::a_tie_keeps_the_first_agents_pane' 1`, then
      `MIN=23 sh $CHECKS/AGENTSEAM.sh` with `ALLOWED` gaining `src/launch.rs`, and
      `SCAN_MIN=100 HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' sh
      $CHECKS/NODEFAULT-UI.sh`. Commit.

## 7. `Action`'s four variants and `Dashboard::apply`
<!-- kind: behavior -->

- [ ] 7.1 RED: Write `ui::app::tests::`'s nine —
      `the_four_action_keys_map_and_their_near_misses_do_not`,
      `a_launch_action_reaches_no_collaborator_and_starts_no_work`,
      `a_refused_launch_records_the_reason_and_produces_no_request`,
      `an_unreachable_socket_makes_the_four_keys_change_nothing`,
      `dashboard_names_launch_at_every_site`, `focus_resolves_the_pane_for_each_attribution_
      tier`, `g_on_an_unattributed_change_does_nothing`,
      `an_archived_change_launches_on_the_same_terms`, and
      `launch_intent_maps_one_to_one_from_the_action`. **In the same task**, extend the **eight**
      landed tests `EXTENDED`'s pair list names in `src/ui/app.rs` — including
      `no_action_mutates_changes` and the two `attribution_*` tests, whose `panes` extensions
      belong here rather than in group 6 because they assert through `Dashboard::attribution()`.

- [ ] 7.2 CHECK: Confirm the failures are the missing arms, and in particular that
      `no_action_mutates_changes` fails on its **exhaustive match** first — a new variant is a
      compile error there before it is an array-length mismatch.

- [ ] 7.3 GREEN: Give the four `apply` arms their bodies: map the action to an `Intent`, gather
      the selected change's name, `attribution().panes`, `agents.reachable`, and the live
      agent names, call `launch::decide`, and write `launch.pending` or `launch.problems`.
      Bump `no_action_mutates_changes`' count assertion from **13** to **17** and add the four
      names to its exhaustive match and its `variants` array, in one edit whose commit message
      says why the number moved.

- [ ] 7.4 CHECK: Contract gate — re-inspect `Dashboard`'s published shape against design.md →
      Contracts and confirm the twelfth field broke every construction site at compile time,
      and that `Change`, `ChangeSet`, `from_files`, and `from_cli` are untouched. Then
      **re-measure** the app run's span count —
      `SCAN_MIN=9999 TYPES='Dashboard Filter Detail Refresh Launch' sh $CHECKS/NODEFAULT-UI.sh`
      — read the realized figure from its FAIL line, and use it in 7.5 and 13.1.
      **Red when:** the realized figure is not above 174; the twelfth field adds literals to a
      type that had 174 spans, so a floor left at 174 bites nothing.

- [ ] 7.4b CHANGE: Correct `src/ui/app.rs`'s `Action` doc comment — "The nine outcomes" becomes
      seventeen — in the same commit that moves the count, so the comment and the assertion move
      together.

- [ ] 7.5 VERIFY: The four `apply` arms each gather the same five values, so extract that
      gathering into one private helper if they read as four copies; otherwise record that no
      refactor was needed. `testcount --lib 'ui::app::tests::' 72`,
      `testcount --lib 'ui::app::tests::no_action_mutates_changes' 1`, and the eight other
      single-test floors; then
      `SCAN_MIN=<the figure 7.4 measured, above 174> TYPES='Dashboard Filter Detail Refresh
      Launch' sh $CHECKS/NODEFAULT-UI.sh` — green for the first time — plus
      `sh $CHECKS/NOIO-VIEW.sh` and `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`. Commit.

## 8. The loop's dispatch and drain
<!-- kind: behavior -->

- [ ] 8.1 RED: Write `ui::driver::tests::`'s five —
      `a_pending_launch_request_is_handed_over_exactly_once`,
      `a_launch_outcome_updates_the_mapping_and_replaces_the_problem`,
      `a_quit_on_the_same_event_as_a_launch_dispatches_nothing`,
      `a_launch_failure_is_recorded_once_and_the_loop_continues`, and
      `live_cannot_be_built_without_naming_the_launcher` — over new
      `testutil::RecordingLauncher` and `testutil::ScriptedLauncher` doubles. **In the same
      task**, extend the **four** landed driver tests `EXTENDED`'s pair list names.

- [ ] 8.2 GREEN: Add step 1 (take `launch.pending`, hand it to `live.launcher.request`) and
      step 6 (drain, insert `named` into `agent_names.names`, replace `launch.problems` with
      `problem`) to `drive_live_tier`, in the positions live-updates' delta fixes.

- [ ] 8.3 VERIFY: No refactor was needed — the two steps are two statements inside
      `drive_live_tier`. `testcount --lib 'ui::driver::tests::' 34` plus the five single-test
      floors,
      then `sh $CHECKS/NOBLOCK.sh` and `sh $CHECKS/NOIO-VIEW.sh`. Commit.

## 9. The launch problem rows in `ui::list`
<!-- kind: behavior -->

- [ ] 9.1 RED: Write `ui::list::tests::a_launch_problem_is_the_lists_first_row`,
      `::launch_refresh_and_change_problems_in_order`, `::no_launch_problem_draws_no_extra_row`,
      and `::a_launch_problem_row_carries_no_badge`, asserting the exact strings
      `change-rows`' and `live-updates`' deltas give at 38 and 58. **In the same task**, extend
      the three landed list tests `EXTENDED`'s pair list names. Each test names both 38 and 58
      **unsuffixed**, or `LISTWIDTHS` reports it missing a width.

- [ ] 9.2 GREEN: Emit one `RowKind::Problem` row per `launch.problems` entry, ahead of the
      refresh problems, through the same `pad_or_truncate_right` every other row uses.

- [ ] 9.3 VERIFY: No refactor was needed — the launch rows reuse the loop the refresh rows
      already use. `testcount --lib 'ui::list::tests::' 29` plus the four single-test floors,
      then `LIST_MIN=29 sh $CHECKS/LISTWIDTHS.sh` — green for the first time. Commit.

## 10. The action hints in `ui::view`
<!-- kind: behavior -->

- [ ] 10.1 RED: Write `ui::view::tests::the_action_hints_follow_esc_back_when_reachable`,
      `::the_action_hints_are_dropped_whole_g_focus_first`,
      `::an_unreachable_socket_hides_both_hints`,
      `::the_count_is_dropped_before_the_action_hints`, `::the_action_hints_survive_a_filter`,
      and `::a_launch_problem_renders_above_a_watch_problem`. **In the same task**, extend the
      **twelve** landed view tests `EXTENDED`'s pair list names. Each test names both 60 and 120
      unsuffixed, or `WIDTHS` reports it missing a width.

- [ ] 10.2 CHECK: Confirm the failures are the missing hints and not a broken fixture, and that
      the landed tests whose extension pins `agents.reachable` to `false` fail for the
      opposite reason — they are red because the field is now read.

- [ ] 10.3 GREEN: Insert `a/c/s launch` and `g focus` into the hint list `render_footer`
      builds, after `Esc back` and before the count, only when `dashboard.agents.reachable`.

- [ ] 10.4 VERIFY: No refactor was needed — the two hints are two `push` calls into the list
      `render_footer` already builds. `testcount --lib 'ui::view::tests::' 94` plus the six
      single-test floors,
      then `WIDTHS_MIN=94 sh $CHECKS/WIDTHS.sh` — green for the first time — plus
      `MD_MIN=24 sh $CHECKS/MDWIDTHS.sh`, `TASK_MIN=16 sh $CHECKS/TASKWIDTHS.sh`, and
      `DETAIL_MIN=23 sh $CHECKS/DETAILWIDTHS.sh`, each at its measured floor for the first
      time. Commit.

## 11. The composition root starts the launcher
<!-- kind: behavior -->

- [ ] 11.1 RED: Write `ui::tests::load::load_initialises_an_empty_launch_tier` and
      `cli::tests::a_prompt_argument_with_a_space_survives_the_seam`. **In the same task**,
      extend the five landed `ui::tests::` tests `EXTENDED`'s pair list names — the four
      wiring tests and `tasks_tab_is_read_only`, the last driven with `agents.reachable` true
      and a recording launcher so `a`/`c`/`s`/`g` really reach `decide` while the tree stays
      byte-identical.

- [ ] 11.2 GREEN: Give `start_collaborators` the two behaviours group 1's skeleton left out:
      thread the state directory into `launch::start`, and use `launch::none()` on the
      no-repository arm. Correct `src/cli.rs`'s `CliError` doc comment — "`agent-launch` four
      more" becomes five, named, and "all eight failures" becomes nine.

- [ ] 11.3 CHECK: Contract gate — re-inspect `Startup`, `Collaborators`,
      `start_collaborators`, and `run_wired` against design.md → Contracts, and confirm `run`
      still holds no branch, no loop, and no field selection.

- [ ] 11.4 VERIFY: No refactor was needed — `start_collaborators` gains one `match` arm beside
      the one it already has. `testcount --lib 'ui::tests::load::' 10`,
      `testcount --lib 'cli::tests::' 41`, then `sh $CHECKS/WIRED.sh`,
      `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`, `MIN=22 sh $CHECKS/LAUNCHSEAM.sh`, and
      `sh $CHECKS/READSEAM.sh`. Commit.

## 12. Acceptance Test — Outer Loop GREEN
<!-- kind: operational -->

- [ ] 12.1 VERIFY: `testcount --lib 'ui::tests::wiring::' 7` — all seven pass, and the
      assertion messages recorded in task 2.4 no longer appear.

- [ ] 12.2 CHECK: Run the seven plants design.md → Test Strategy names, one at a time, recording
      the exit status and the failing assertions verbatim for each and reverting after each:
      `agents::none()`, `launch::none()`, `run_loop` not dispatching `launch.pending`,
      `run_request` passing a constant `--pane w0:p0`, `state::record` not called,
      `drive_live_tier` dropping `Outcome::named`, and `run_request` setting `Outcome::named`
      **before** `agent start` rather than after — the seventh is invisible to the acceptance
      tier (its launch succeeds either way) and is caught by
      `testcount --lib 'launch::tests::run_request::' 10` alone.
      **Red when:** any plant leaves the suite green, or two plants fail the identical *set* of
      assertions — the table in design.md → Test Strategy names what separates each pair.

- [ ] 12.3 CHECK: Re-run the six plants `agent-poller`'s "The wiring test fails when the poller
      is replaced by the inert double" names, against the now-amended landed wiring tests, and
      record that each is still red.

- [ ] 12.4 VERIFY: `make check` — the first unqualified run since group 2. Commit.

## 13. The architecture checks, all green together
<!-- kind: operational -->

- [ ] 13.1 CHECK: Run all thirty gates at their landed invocations, in one pass, and record
      every exit status and OK line: `MIN=23 NOSPAWN-GREP`; `MIN=23 NOLIT-CHANGE`;
      `UI_MIN=11 NOCLI-SHELL`; `NOIO-VIEW`; `UI_MIN=10 READSEAM`; `MIN=23 MDSEAM`; `NOTABSEAM`;
      `TASKSEAM`;
      `MIN=25 WATCHSEAM`; `MIN=23 ALLOWED='src/cli.rs src/agents.rs src/launch.rs
      src/ui/mod.rs' AGENTSEAM`; `MIN=22 LAUNCHSEAM`; `NOBLOCK`; `SLEEP_MIN=5 MIN=26 NOSLEEP`;
      `UI_MIN=11 EXTRA='src/watch.rs src/refresh.rs src/agents.rs src/launch.rs' READONLY-UI`;
      `WIRED`; `PAIRS='<the forty-six>' EXTENDED`; all three `NODEFAULT-UI` invocations;
      `WIDTHS_MIN=94 WIDTHS`; `DETAIL_MIN=23 DETAILWIDTHS`; `LIST_MIN=29 LISTWIDTHS`;
      `MD_MIN=24 MDWIDTHS`; `TASK_MIN=16 TASKWIDTHS`; `NORAW-GREP`; `NOWAIVER`; `NOJSON-SEAM`;
      `GATE-MECH1.py`; `WORK=… DEPS`; `GRAPH-SNAP`;
      `BASE=$(git rev-parse HEAD) CHANGE=agent-launch OPENSPEC-UNTOUCHED`.
      **Red when:** any is not green **except** `DEPS` and `GRAPH-SNAP`, whose failure text
      must be byte-identical to what task 0.4 recorded at `BASE` — including `GRAPH-SNAP`'s,
      which is a hardcoded platform-difference assertion rather than the stale snapshot
      `HANDOFF.md` describes. This change adds no
      dependency and edits neither script, and an identical failure is the evidence that it
      did not make them worse.

- [ ] 13.2 CHECK: Re-measure all three `NODEFAULT-UI` span counts from their own FAIL lines at
      `SCAN_MIN=9999`, and confirm the landed floors are at or below them.
      **Red when:** any realized count is below its floor — a literal was replaced by something
      the scan cannot see.

- [ ] 13.3 CHECK: Confirm every gate that reads a floor was passed one **explicitly** in 13.1
      and none ran at a block default. Read each script's own `${VAR:-N}` line and compare:
      `WIDTHS_MIN`, `LIST_MIN`, `MD_MIN`, `TASK_MIN`, `DETAIL_MIN`, `MIN` (NOSPAWN-GREP,
      NOLIT-CHANGE, AGENTSEAM, WATCHSEAM, LAUNCHSEAM, MDSEAM, NOSLEEP), `UI_MIN` (NOCLI-SHELL,
      READONLY-UI, READSEAM), `SCAN_MIN`, `SLEEP_MIN`, and `PAIRS`.
      **Red when:** any invocation in 13.1 omits a floor the script defines — that is the defect
      `agent-polling` shipped for `WIDTHS` and `LISTWIDTHS` and `agent-attribution` left in place
      for `MDWIDTHS`, `TASKWIDTHS`, `DETAILWIDTHS`, `READSEAM`, and `MDSEAM`.

- [ ] 13.4 VERIFY: `git status --porcelain` shows no change under `openspec/` outside this
      change's own directory. Commit any gate-invocation notes.

## 14. SPEC.md, README.md, and AGENTS.md corrections
<!-- kind: operational -->
<!-- parallel-after: 0 -->

- [ ] 14.1 CHECK: Grep each target passage before editing and record what is there today:
      `SPEC.md` → "Launch flow" (three lines, `<change>` where the derived name belongs),
      → "Keys" (the `a`/`c`/`s`/`g` rows and the hidden-action-keys sentence), → "Degraded
      states" (one row for an unreachable socket, none for a failed launch); `README.md` →
      "Keys" (lines 38–61) and → Configuration's `agent_kind` row; and `AGENTS.md` →
      Architecture rules 1 and 7. The two shipped **doc comments** this change also corrects are
      **not** this group's: `src/ui/app.rs`'s belongs to task 7.4b and `src/cli.rs`'s to task
      11.2, so this group touches no source file and its `parallel-after` marker stays true.
      **Red when:** a passage is not where this task says.

- [ ] 14.2 CHANGE: Rewrite in `SPEC.md`: Herdr integration → Launch flow (audience: `plugin-actions`
      and every later change) — the three argument vectors as measured, `--direction` as
      required rather than optional, the split targeting the focused pane, the envelope
      `pane split` returns, the **derived agent name** in `agent start` and `agent prompt`
      (which "Attributing an agent" already flags as this change's correction), `agent prompt`'s
      positional text and the absence of `--wait`, `agent focus`'s accepted targets, and the
      error shape every one of them uses. Replaces the current three-line block.

- [ ] 14.3 CHANGE: Add to `SPEC.md`: Degraded states (audience: `degraded-states` and every later
      change) — six rows: a failed `pane split`; a failed `agent start` leaving the pane
      un-closed and named; a failed `agent prompt` leaving a recorded, running agent; a derived
      name already live, refused before any call; a failed `state::record` that does not undo
      the start; and `g` on a change with no attributed agent. Add a **seventh** for a
      `pane split` payload that is not the measured envelope — the shape a Herdr older than the
      manifest floor's `0.7.0` might return — which stops the launch before an agent is started.
      Amend the existing unreachable-socket row to say the action keys and their footer hints
      are both hidden.

- [ ] 14.4 CHANGE: Rewrite in `SPEC.md` → Keys and `README.md` → Keys **together** (audience:
      every operator reading the documented reference, and every later change touching a
      keybinding), since they are the
      documented reference and this change is **BREAKING** per `openspec/config.yaml` →
      `rules.proposal`: the `a`/`c`/`s`/`g` rows gain what each does on a change with no agent
      and while filtering, and both files gain the footer's `a/c/s launch  g focus` form.
      Correct `README.md`'s Configuration row for `agent_kind` to name `g` as unaffected.

- [ ] 14.5 CHANGE: Rewrite in `AGENTS.md`: Architecture rule 1 (audience: every future session) — the
      spawn rule gains the launcher as its second Herdr consumer and names `LAUNCHSEAM`
      alongside the tree-wide grep. Rewrite rule 7 in place to name `src/launch.rs` as the
      fourth seam module. Add **one** new rule, at most five lines: the plugin starts a process
      that will write inside `openspec/`, and the boundary is the process — the plugin's own
      writes are `agent-names.toml` and nothing else. Delete nothing else; the net addition is
      under ten lines.

- [ ] 14.7 VERIFY: `BASE=$(git rev-parse HEAD) CHANGE=agent-launch sh
      $CHECKS/OPENSPEC-UNTOUCHED.sh` is green — `SPEC.md`, `README.md`, and `AGENTS.md` are
      outside `openspec/`, and `openspec/IMPLEMENTATION-ORDER.md` is deliberately **not**
      edited here; the roadmap row is `openspec archive`'s to correct. Commit.

## 15. Change Review
<!-- kind: operational -->

- [ ] 15.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against
      proposal.md, every spec scenario, design.md, and tasks.md, given the diff and the
      artifacts only. Require it to write findings to a scratchpad file as it goes rather than
      returning them only in a final message.
      **Concentration points, per `openspec/config.yaml`:** nothing spawns outside `cli`;
      views perform no I/O; **nothing writes inside `openspec/` — and this is the change where
      a keypress starts a process that will, so the boundary must be the process and the
      evidence must be a snapshot**; every external dependency has an absent case; attribution
      must not guess; `from_files` and `from_cli` still produce the same `Change`; view tests
      run at 60 and 120; **agent names are capped at 32 characters and match
      `[a-z][a-z0-9_-]{0,31}`, and the over-cap test must exist and must be able to fail**.

- [ ] 15.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run every affected test and gate.

- [ ] 15.3 VERIFY: No blocking or unowned finding remains, and every artifact that a fix
      contradicted has been refreshed. Commit.

## 16. Lint & Verify
<!-- kind: operational -->

- [ ] 16.1 CHECK: Inspect the intended verification commands and affected tiers, and confirm
      each one can fail: every `testcount` invocation carries a minimum above the count that
      existed on `main`, every single-test filter names its function in full, and every gate
      carries its explicit floor rather than a block default.

- [ ] 16.2 VERIFY: `cargo fmt --all -- --check` — clean.

- [ ] 16.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings.
      This crate has no separate type checker; `cargo clippy` compiles and is that step.

- [ ] 16.4 VERIFY: `cargo test --all-features` — green, and
      `cargo test --all-features --lib -- --list | grep -c ': test$'` reports **907**.

- [ ] 16.5 VERIFY: `cargo llvm-cov --fail-under-lines 80`, and record the **line** percentage
      from the TOTAL row's line columns — never the region count, which leads that row.

- [ ] 16.6 VERIFY: `make check` — the single gate, green. If it fails, report the failing
      sub-command rather than a summary.

- [ ] 16.7 VERIFY: `openspec validate agent-launch --strict`, with
      `export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"` in the shell. Commit.
