# agent-attribution — implementation tasks

## How to read this file

Every floor here was **measured** before it was set, and the command that produced each
number is written beside it. A realized count below a group's target means the missing test
is written, not the floor lowered.

Every check is extracted to `$CHECKS/<LABEL>.sh` in task 0.2 and run from the extracted file
thereafter, never from a retyped copy. This change **adds one** check block (`EXTENDED`,
reproduced in full below), **edits two** (`NOIO-VIEW`, `WIRED`) by the exact string
replacements in task 0.5, and moves the invocation floor of **two more** (`LISTWIDTHS`,
`WIDTHS`) plus one type list and one span floor (`NODEFAULT-UI`). `OPENSPEC-UNTOUCHED` is
neither edited nor re-defaulted: it is already parameterised over `$CHANGE`, and every
invocation here passes `CHANGE=agent-attribution` explicitly.

`testcount` is sourced, not run: `. $CHECKS/TESTCOUNT.sh`, then
`testcount --lib '<filter>' <minimum>`. It takes the scope, the filter, **and** the minimum,
in that order, and is judged on a counted minimum because a `cargo test` filter matching
nothing exits 0. Every filter below that guards a **single** test is written out in full for
that reason: `ui::list::tests::a_badged` matches four functions and would be satisfied by any
one of them.

**`make check` is not runnable unqualified between groups 2 and 8.** Group 2's acceptance
test is deliberately RED for that whole span and `cargo llvm-cov` hard-fails on any failing
test, so the four gates are run individually there:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features            # must fail on EXACTLY the one known acceptance test,
                                     # with the identical assertion message each time
cargo llvm-cov --ignore-run-fail --fail-under-lines 80
```

A **second** failing test at any of those boundaries is a real regression, not a known one.
`--ignore-run-fail` ignores a failing test **run**, never a failing compile, which is why
group 1 lands the inert structure first — see design.md → Test Strategy. The literal,
unqualified `make check` is run from task 8.4 onward.

## Measured at planning time

Every figure below came from the command shown, run on `main` at
`4e673690db3f8bc4d90480570ce1f43c3b904b5a`. **That sha is recorded for comparison only.**
Task 0.1 re-derives `BASE=$(git rev-parse HEAD)` fresh — this repository is shared with other
live sessions and its history was rewritten once, so a hardcoded sha is a gate that silently
includes this change's own commits in its baseline.

| Figure | Command | Value |
|---|---|---|
| Library tests | `cargo test --all-features --lib -- --list \| grep -c ': test$'` | **805** |
| Line coverage | `cargo llvm-cov --summary-only \| tail -1` | **97.18%**, 20,890 lines, 589 uncovered |
| Region count (**not** a line count) | same TOTAL row | 34,201 |
| `src/ui/*.rs` | `find src/ui -name '*.rs' \| wc -l` | **11**, unchanged by this change |
| `NOSPAWN-GREP` / `NOLIT-CHANGE` set | `find src -name '*.rs' ! -path 'src/changes.rs' \| wc -l` | **22**, unchanged |
| `AGENTSEAM` set | `find src tests -name '*.rs' ! -path src/cli.rs ! -path src/agents.rs ! -path src/ui/mod.rs \| wc -l` | **22**, unchanged |
| `NOSLEEP` set | `find src tests -name '*.rs' \| wc -l` | **25**, unchanged |
| `#[test]` in `src/ui/list.rs` | `grep -c '^[[:space:]]*#\[test\][[:space:]]*$' src/ui/list.rs` | **20** → 25 |
| `#[test]` in `src/ui/view.rs` | `grep -c '^[[:space:]]*#\[test\][[:space:]]*$' src/ui/view.rs` | **82** → 88 |
| `NODEFAULT-UI` half-B spans, `src/ui/app.rs` run | `SCAN_MIN=200 TYPES='Dashboard Filter Detail Refresh' sh $CHECKS/NODEFAULT-UI.sh` | **165** from its own FAIL line; floor raised 90 → **165** |
| `NODEFAULT-UI` half-B spans, `src/agents.rs` run | `HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot' SCAN_MIN=90 sh $CHECKS/NODEFAULT-UI.sh` | **86**; floor set to **86** |
| `state::read` under `src/ui/` | `grep -rn 'state::read\|crate::state' src/ui/` | **0 matches**, so adding it to `NOIO-VIEW` is green today |
| `Startup { … }` literals | `grep -rn 'Startup {' src/` | **2** — `ui::run` and the one shared `run_wired_at` test helper |
| `ui::load(` call sites | `grep -rn 'load(' src/ui/mod.rs` | **1** production (`run_wired`) and **13** test — `tests::load` ×9, `tests::live` ×3, `tests::detail` ×1 |
| clippy's `too_many_arguments` threshold | a 7-param and an 8-param function under `-D warnings` in a scratch crate | silent at 7, `too many arguments (8/7)` at 8 |
| Herdr agent list scope | `herdr agent list` from the repo, from an unrelated repo, and from `/` | **byte-identical** output; both listed agents' `cwd` in a *different* repository |
| Herdr name validation | `herdr agent start 'Illegal Name!' --kind claude --pane w9:p9999` | `invalid_agent_name`, exit 1, before pane lookup |
| Herdr worktree layout | `herdr worktree list` | linked worktrees at `<repo-parent>/.worktrees/<repo>-<branch>`, **outside** the root |

## Test-count floors — measured plus this change's enumerated new tests

| Group | Filter | Measured | New | Target |
|---|---|---|---|---|
| 3 | `agents::tests::attribute::` | 0 | 17 | 17 |
| 4 | `ui::app::tests::` | 59 | 4 | 63 |
| 5 | `ui::list::tests::` | 20 | 5 | 25 |
| 6 | `ui::view::tests::` | 82 | 6 | 88 |
| 7 | `ui::tests::load::` | 7 | 2 | 9 |
| 2, 8 | `ui::tests::wiring::` | 3 | 1 | 4 |

Library total: `805 + 17 + 4 + 5 + 6 + 2 + 1` = **840**, asserted in group 12 and nowhere else.

**Fourteen landed tests are *modified* rather than added**, and a modified test moves no count,
so no `testcount` floor can see one. `EXTENDED` is what does: eight of the fourteen are named
in its pair list with the token their extension must add, and the other six are the
`ui::tests::load::` and `ui::tests::wiring::` call sites, which fail to **compile** until they
are amended.

**`ui::tests::wiring::`'s floor is asserted only at group 8**, never between groups 2 and 8:
`testcount` counts tests that *passed*, and the new one is deliberately red for that span.

## Group ordering — examined; one parallel pair, the rest sequential

Groups 3 through 7 each write a different file (`src/agents.rs`, `src/ui/app.rs`,
`src/ui/list.rs`, `src/ui/view.rs`, `src/ui/mod.rs`), so criterion 1 holds for every pair —
with one exception recorded rather than glossed: task 4.2 touches **every** `Dashboard`
construction site in the crate, including the two inside `src/ui/mod.rs`, which is group 7's
file. **The other two criteria do not hold anywhere.** Group 4 needs `agents::attribute`;
groups 5, 6, and 7 each need a real `Dashboard::attribution()`. The one otherwise-independent
pair — 5 and 6, after 4 — still fails criterion 3: `cargo test` builds the whole crate, so a
half-written `src/ui/view.rs` fails group 5's `testcount` and vice versa, and neither failure
is attributable to the group that caused it.

**Group 10 is the exception and is marked parallel after group 0.** It edits `SPEC.md` and
`AGENTS.md` and nothing else — no source file, no test, no check script — and no gate in this
plan reads either document, so a failure in group 10 cannot be mistaken for a failure in any
other group, and vice versa. Its content is fixed by the spec deltas before implementation
starts, so it depends on no code.

---

## Command-level checks this change adds and edits

Every block below was written and run against the real tree at planning time, with its exit
status and output recorded under it.

```sh
# EXTENDED — NEW in agent-attribution. Every landed test this change EXTENDS rather than
# replaces actually gained its extension. An aggregate `testcount` floor is satisfied by the
# NEW tests alone, so skipping every extension leaves the whole suite green; this check is what
# makes the fourteen modified tests real rather than aspirational.
#
# One line per pair: <file>:<test fn>:<token the extension must add>. The span is the named
# function's body, cut at the next line-anchored `    fn ` at the same indent, so a token
# appearing in a NEIGHBOURING test does not satisfy this one. Adding a pair is how a later
# change records an extension; deleting one is a deliberate act a reviewer can see.
[ -f src/ui/list.rs ] || { echo "EXTENDED FAIL: src/ui/list.rs missing" >&2; exit 1; }
[ -f src/ui/view.rs ] || { echo "EXTENDED FAIL: src/ui/view.rs missing" >&2; exit 1; }
PAIRS="${PAIRS:-src/ui/list.rs:active_row_grammar_at_38_and_58:badge
src/ui/list.rs:every_row_is_exactly_the_requested_width:badge
src/ui/list.rs:a_long_name_is_truncated_with_an_ellipsis:badge
src/ui/list.rs:refresh_and_change_problems_in_order:badge
src/ui/list.rs:the_four_message_states_are_distinct:badge
src/ui/view.rs:the_prompt_replaces_the_hints_while_filtering:unattributed
src/ui/view.rs:an_accepted_query_leads_the_hint_list:unattributed
src/ui/view.rs:agents_change_no_pixel:assert_ne!}"
PAIRS="$PAIRS" python3 - <<'PY' || exit 1
import os, re, sys
pairs = [l for l in os.environ["PAIRS"].splitlines() if l.strip()]
if len(pairs) < 8:
    print(f"EXTENDED FAIL: only {len(pairs)} pairs given (expected >= 8) - the list was gutted",
          file=sys.stderr); sys.exit(1)
bad, missing_fn = [], []
for line in pairs:
    path, fn, token = line.split(":", 2)
    src = open(path).read()
    m = re.search(r"(?m)^([ \t]*)fn %s\(" % re.escape(fn), src)
    if not m:
        missing_fn.append(f"{path}::{fn}"); continue
    indent = m.group(1)
    rest = src[m.end():]
    nxt = re.search(r"(?m)^%sfn [a-z0-9_]+\(" % re.escape(indent), rest)
    body = rest[: nxt.start()] if nxt else rest
    if token not in body:
        bad.append(f"{path}::{fn} does not name {token!r}")
if missing_fn:
    print("EXTENDED FAIL: named test not found (renamed or deleted): " + ", ".join(missing_fn),
          file=sys.stderr); sys.exit(1)
if bad:
    print("EXTENDED FAIL: a landed test was not extended:\n  " + "\n  ".join(bad),
          file=sys.stderr); sys.exit(1)
print(f"EXTENDED OK: all {len(pairs)} landed tests carry their extension token")
PY
```

**Measured at planning time.** On `main`: `exit=1`, `EXTENDED FAIL: a landed test was not
extended:` followed by all eight pairs — the expected RED, since none of the extensions exists
yet. Against a pair list whose tokens are already present in each span (`add-token-refresh`,
`width`, `…`, `problem`, `No changes`, `/add_`, `q quit`, `assert_eq!`): `exit=0`,
`EXTENDED OK: all 8 landed tests carry their extension token`. Span isolation proved: with
`the_separator_fills_the_width` paired against `add-token-refresh`, a token that exists in a
neighbouring test in the same file, `exit=1` — the cut isolates the body. Both guards fire: a
renamed function gives `named test not found (renamed or deleted)`, and a one-entry list gives
`only 1 pairs given (expected >= 8) - the list was gutted`.

**`NOIO-VIEW.sh`** (extracted from `.../2026-09-05-tasks-tab/tasks.md`), one replacement:

```
-IO_RE='std::fs|std::io|std::env|std::process|std::net|File::|read_to_string|tasks::read|Command'
+IO_RE='std::fs|std::io|std::env|std::process|std::net|File::|read_to_string|tasks::read|state::read|Command'
```

`crate::state::read` matches none of the other alternatives — not `std::fs` (the call site
writes `crate::state::read`), not `read_to_string` (that spelling lives inside `src/state.rs`,
not at the call site) — which is exactly why `tasks::read` was added by `tasks-tab`.

**Measured at planning time.** Unedited and edited, both green on `main` (`exit=0`,
`NOIO-VIEW OK: 8 pure files carry no I/O API; positive control matched`). Edited, against a
`src/ui/app.rs` carrying
`fn zz_plant(d: &std::path::Path) { let _ = crate::state::read(Some(d)); }` appended to the
file: `exit=1`, `NOIO-VIEW FAIL: I/O API in a pure view file: src/ui/app.rs:2823:fn zz_plant(…)`.
Plant removed, `exit=0` again. The unedited block misses the same plant, which is what makes
the edit load-bearing rather than decorative.

**`WIRED.sh`** (extracted from `.../2026-09-06-agent-polling/tasks.md`), five replacements:

1. after `CLI="${CLI:-src/cli.rs}"` insert `STATE="${STATE:-src/state.rs}"`;
2. after the `[ -f "$CLI" ] || fail …` line insert
   `[ -f "$STATE" ] || fail "$STATE missing - the mapping-read control has nothing to match"`;
3. after the `HERDR_PROGRAM` positive control insert
   `grep -qE '^pub fn read\(' "$STATE" || fail "positive control - $STATE defines no 'pub fn read('"`;
4. leg 1's name list gains `'state::read'` as an eighth entry;
5. immediately **before** leg 4, insert a new **leg 5** over the body leg 2 already cut:

```sh
# Leg 5 — NEW in agent-attribution. `run` resolves the state directory and threads the result
# into Startup. Leg 1 sees `state::read` wherever it appears, and it will appear inside `load`,
# which fifteen tests drive; the untested link is the VALUE `run` passes, and a shipped binary
# passing `None` has a permanently dead tier-1 mapping with every other gate green.
printf '%s\n' "$body" | grep -q 'state::state_dir(' \
  || fail "leg 5: 'pub fn run()' does not resolve state::state_dir( - Startup.state_dir would be a literal"
if printf '%s\n' "$body" | grep -q 'state_dir: None'; then
  fail "leg 5: 'pub fn run()' hardcodes 'state_dir: None' - the mapping tier would be dead in the shipped binary"
fi
```

and the closing `echo` becomes `WIRED OK: eight names present in $MOD; run resolves
state::state_dir; …`.

**Measured at planning time**, all four states, using a scratch copy of `src/` carrying the
intended wiring. On `main`: `exit=1`, `WIRED FAIL: leg 1: src/ui/mod.rs's production slice does
not name state::read`. On the wired copy: `exit=0`, `WIRED OK: eight names present in
src/ui/mod.rs; run resolves state::state_dir; 'pub fn run()' is 22 lines with no branch and no
loop; no "herdr" literal under src/ui`. On the wired copy with `state_dir: state_dir.as_deref()`
replaced by `state_dir: None`: `exit=1`, `WIRED FAIL: leg 5: 'pub fn run()' hardcodes
'state_dir: None'`. On the wired copy with `state::read` deleted from `load`: `exit=1`, leg 1.
On the wired copy with `src/state.rs`'s `pub fn read(` renamed: `exit=1`,
`WIRED FAIL: positive control - src/state.rs defines no 'pub fn read('`.

**Invocation floors that move, with no edit to the block:**

- `LISTWIDTHS`: `LIST_MIN` **→ 25**, passed explicitly. Measured: at 17 (the block default),
  `exit=0`, `LISTWIDTHS OK: all 20 row-grammar tests name both 38 and 58`; at 25, `exit=1`,
  `LISTWIDTHS FAIL: found 20 #[test] functions in src/ui/list.rs, expected >= 25`.
- `WIDTHS`: `WIDTHS_MIN` **→ 88**, passed explicitly. Measured: at 82, `exit=0`,
  `WIDTHS OK: all 82 view tests name both 60 and 120`; at 88, `exit=1`,
  `WIDTHS FAIL: found 82 #[test] functions in src/ui/view.rs, expected >= 88`. `live-refresh`
  last set this floor to 81 and `agent-polling`'s gate pass invoked both blocks **bare**, so
  they ran at their defaults of 16 and 17; passing the value explicitly is what makes the floor
  real again.
- `NODEFAULT-UI`: half A's second invocation gains `Attribution` in `TYPES`; half B's
  `SCAN_MIN` **90 → 165** for the `src/ui/app.rs` run and **→ 86** for the `src/agents.rs` run.
  Measured: with `Attribution` in `TYPES` and `HOMEFILE=src/agents.rs`, `exit=1`,
  `NODEFAULT-UI FAIL: positive control - src/agents.rs has no 'struct Attribution {'` — the
  expected RED, first green at task 3.4. At `SCAN_MIN=200` on the app run, `exit=1`,
  `half B found only 165 literal/pattern spans`; at `SCAN_MIN=90` on the agents run, `exit=1`,
  `half B found only 86`.
- `OPENSPEC-UNTOUCHED`: **no edit and no re-default.** Measured with
  `BASE=$BASE CHANGE=agent-attribution` on `main`: `exit=0`, `OPENSPEC-UNTOUCHED OK`; with an
  empty `openspec/specs/STRAY.tmp` planted, `exit=1`,
  `FAIL: wrote inside openspec/ outside this change: openspec/specs/STRAY.tmp`; with
  `CHANGE=nope`, `exit=1`,
  `FAIL: no such change directory: openspec/changes/nope - the exclusion is vacuous`.

---

## 0. Baseline, checks, and the scratchpad
<!-- kind: operational -->

- [x] 0.1 CHECK: Record the starting state by **measuring**, never by copying a number from
      this file. `export BASE=$(git rev-parse HEAD)`; then run each command in the "Measured
      at planning time" table and record its output verbatim.
      **Red when:** `BASE` is empty or not a commit, or any measured figure differs from the
      table. Prefer `BASE=<sha> CHANGE=agent-attribution sh $CHECKS/OPENSPEC-UNTOUCHED.sh` at
      each invocation over an exported variable: re-exporting from the current `HEAD` after a
      fresh shell silently puts this change's own commits inside the baseline.

- [x] 0.2 CHANGE: Write `EXTENDED.sh` to `$CHECKS` from the block above, and extract the other
      twenty-eight files from the archived tasks files that last reproduced each in full:
      `AGENTSEAM.sh`, `WIRED.sh` from `.../2026-09-06-agent-polling/tasks.md`; `WATCHSEAM.sh`,
      `READONLY-UI.sh`, `NOBLOCK.sh`, `NOSLEEP.sh`, `OPENSPEC-UNTOUCHED.sh` from
      `.../2026-09-06-live-refresh/tasks.md`; `NOSPAWN-GREP.sh`, `READSEAM.sh`,
      `NOCLI-SHELL.sh`, `NOLIT-CHANGE.sh`, `MDSEAM.sh`, `NOTABSEAM.sh`, `WIDTHS.sh`,
      `DETAILWIDTHS.sh`, `NODEFAULT-UI.sh` from `.../2026-09-05-detail-view/tasks.md`;
      `NORAW-GREP.sh`, `LISTWIDTHS.sh`, `MDWIDTHS.sh`, `NOWAIVER.sh`, `TESTCOUNT.sh`,
      `DEPS.sh`, `GRAPH-SNAP.sh` from `.../2026-09-05-markdown-viewer/tasks.md`;
      `NOIO-VIEW.sh`, `TASKSEAM.sh`, `TASKWIDTHS.sh` from `.../2026-09-05-tasks-tab/tasks.md`;
      `GATE-MECH1.py`, `NOJSON-SEAM.sh` from `.../2026-09-04-changes-from-cli/tasks.md`.
      **Red when:** `ls -1 "$CHECKS" | wc -l` is not 29, or `$CHECKS/GATE-MECH1.py` is absent
      because the extractor read only each block's first line — its label sits on the block's
      **second** line behind a shebang. Use a scratch directory this session owns: another live
      session in this repository may be writing to the same shared path.

- [x] 0.3 CHANGE: Re-apply `agent-polling`'s four accumulated edits, which live in that
      change's tasks file as string replacements rather than in any reproduced block:
      `NODEFAULT-UI.sh`'s `HOMEFILE` parameterisation, its `|| exit 1` repair, and its OK-line
      suffix; `NOSLEEP.sh`'s leg 2b gaining `src/agents.rs`; `NOBLOCK.sh`'s leg 3 gaining
      `src/agents.rs`; and `OPENSPEC-UNTOUCHED.sh`'s `$CHANGE` parameterisation. Print each
      resulting `diff` before running.
      **Red when:** `NODEFAULT-UI.sh` still names `$SRC/ui/app.rs` in its positive control, or
      `OPENSPEC-UNTOUCHED.sh` still holds the literal `^openspec/changes/live-refresh/`.

- [x] 0.4 CHECK: Run all twenty-eight extracted gates against the unmodified tree at `BASE`,
      at the invocations this change uses, and record each exit status and its first line
      verbatim.
      **Expected:** `EXTENDED` red on all eight pairs; `WIRED` red on leg 1;
      `LIST_MIN=25 LISTWIDTHS` red with `found 20`; `WIDTHS_MIN=88 WIDTHS` red with `found 82`;
      `TYPES='Agent Listed AgentSnapshot Attribution' NODEFAULT-UI` red on its positive
      control; every other gate green.

- [x] 0.5 CHANGE: Apply this change's own edits — `NOIO-VIEW.sh`'s one replacement and
      `WIRED.sh`'s five, exactly as written above — printing each resulting `diff` before
      running.
      **Expected after this task:** `NOIO-VIEW` green; `WIRED` still red, now on **leg 1**
      rather than on a guard. A guard failure here is a defect in the edit.

- [x] 0.6 CHECK: Prove `NOIO-VIEW`'s edit can fail. Plant
      `fn zz_plant(d: &std::path::Path) { let _ = crate::state::read(Some(d)); }` at the end of
      `src/ui/app.rs`, run `sh $CHECKS/NOIO-VIEW.sh`, record exit **1** and the FAIL line;
      run the **unedited** extraction against the same plant and record exit **0**; remove the
      plant and record exit **0** for the edited block.
      **Red when:** the edited check is green under the plant, or the unedited one is red —
      either means the edit is not the thing catching it.

- [x] 0.7 CHECK: Prove `OPENSPEC-UNTOUCHED`'s exclusion is still scoped. Plant an empty
      `openspec/specs/STRAY.tmp`, run with `CHANGE=agent-attribution` and record exit **1**
      naming that path; remove it; then run with `CHANGE=nope` and record exit **1** naming the
      vacuous exclusion.

- [x] 0.8 VERIFY: `git status --porcelain` shows no change under `openspec/` outside
      `openspec/changes/agent-attribution/`, and
      `BASE=$BASE CHANGE=agent-attribution sh $CHECKS/OPENSPEC-UNTOUCHED.sh` is green. Commit
      nothing in this group — it writes only to `$CHECKS`.

## 1. The skeleton every later group compiles against
<!-- kind: refactor -->

Structure only: inert signatures, no behaviour. The evidence is that the 805 landed tests stay
green while the tree grows the API groups 2 to 7 fill in. See design.md → Test Strategy for why
this precedes the outer-loop RED.

- [x] 1.1 CHARACTERIZE: `. $CHECKS/TESTCOUNT.sh; testcount --lib '' 805` and record the count,
      so the landed suite is green before any file is touched.

- [x] 1.2 REFACTOR: Add the inert shapes. `src/agents.rs`: `Attribution` with its two fields
      and an `attribute` returning an empty one. `src/ui/app.rs`: `Dashboard::agent_names:
      crate::state::Mapping` and an `attribution()` that calls `attribute` with the change
      names it derives. `src/ui/mod.rs`: `Startup::state_dir: Option<&'a Path>`, `load`'s third
      parameter (unread), and `run` passing `state::state_dir(&config::env_lookup())`.

- [x] 1.3 REFACTOR: Fix every site the new shapes break, counted rather than estimated: the
      **2** `Startup { … }` literals, the **14** `ui::load(` call sites (1 production, 13
      test), and every `Dashboard { … }` literal in the crate — `grep -rn 'Startup {' src/`,
      `grep -rn 'load(' src/ui/mod.rs`, and `NODEFAULT-UI`'s 165-span count are the three
      commands that enumerate them.

- [x] 1.4 CHECK: `sh $CHECKS/WIRED.sh` must now be **green** for the first time, including
      leg 5. Then plant `state_dir: None` in `run`'s `Startup` literal, re-run, record exit
      **1** naming leg 5, and revert; then delete `state::read` from `load`, re-run, record
      exit **1** naming leg 1, and revert.
      **Red when:** either plant is green — leg 5 exists precisely because no test reaches the
      value `run` passes.

- [x] 1.5 VERIFY: `testcount --lib '' 805` — the landed suite is unchanged — then
      `cargo clippy --all-targets --all-features -- -D warnings` and
      `cargo fmt --all -- --check`. Commit.

## 2. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

The headline scenario is `agent-poller` → "A polled agent reaches a rendered badge and a
rendered count". Its assertions are chosen so each can fail, and the footer's in both
directions; see design.md → Test Strategy.

- [x] 2.1 CHANGE: Extend `ui::tests::wiring`'s harness, honouring design.md → Test Boundaries:
      a `ScratchDir` repository holding active changes `2fa-support` and `alpha`, each with a
      `tasks.md` counting 4 of 9; a second `ScratchDir` state directory holding
      `agent-names.toml` with `[names]` and `c-2fa-support = "2fa-support"`; and a
      `herdr_script` parameterised on the **canonicalized** repository root and on the agent
      list it prints.
      **Red when:** the script embeds `scratch.path()` rather than
      `testutil::canonical(scratch.path())` — on macOS `/var` is a symlink to `/private/var`,
      so the uncanonicalized form makes the containment test pass where production fails.

- [x] 2.2 RED: Write `ui::tests::wiring::a_polled_agent_reaches_a_rendered_badge`, driving
      `run_wired` at 120x20 and again at 60x20 with a four-agent payload — `c-2fa-support`
      `working` in the repository, `alpha` `blocked` in the repository, one unnamed `claude`
      `idle` in the repository, and one named `alpha` `working` at `/definitely/elsewhere` —
      and a readiness predicate that waits for the scratch `herdr` log's first entry and
      **writes nothing**. Assert the `2fa-support` row's badge column holds `w`, the `alpha`
      row's holds `b`, and the footer row is exactly
      `q quit  Enter detail  Esc back  1 unattributed` padded to the width.

- [x] 2.3 CHECK: Confirm it fails because the behaviour is missing, not because the harness is
      misconfigured: the run must reach `Ok(dashboard)` with `agents.reachable` true and
      `agents.agents` holding four agents, and fail on the badge and footer assertions alone.
      **Red when:** it fails on `StartError`, on a deadline with an empty `herdr` log, or on a
      panic — any of those is a harness fault. Record the exact assertion message; task 8.1
      requires the identical one.

- [x] 2.4 VERIFY: `cargo test --all-features` fails on **exactly** this one test. Commit.

## 3. `agents::attribute` — the three tiers
<!-- kind: behavior -->

- [x] 3.1 RED: Write the seventeen tests of `agents::tests::attribute::`, one per
      `agent-attribution` spec scenario in the first five requirements:
      `an_unmatched_in_scope_agent_is_counted`, `the_name_tier_never_reads_the_kind`,
      `a_terminal_title_attributes_nothing`, `every_empty_input_is_total`,
      `an_agent_in_another_repository_is_invisible`, `containment_is_component_wise`,
      `no_repository_attributes_nothing`, `the_mapping_resolves_a_derived_name`,
      `the_mapping_outranks_the_name`, `a_stale_mapping_falls_through`,
      `an_empty_mapping_leaves_the_name_tier_working`,
      `both_tiers_of_the_change_list_are_attributable`, `matching_is_exact`,
      `a_name_past_the_cap_is_mapping_only`, `an_unnamed_agent_is_counted`,
      `precedence_is_total_and_order_independent`, and `one_agent_per_change_keeps_its_status`.
      **Red when:** any of them builds a `Change` value or touches a filesystem — `attribute`
      takes `&[&str]` and a `BTreeMap` per design.md → Boundaries, and
      `MIN=22 sh $CHECKS/NOLIT-CHANGE.sh` is what catches the first.

- [x] 3.2 GREEN: Implement `attribute`: scope every agent by
      `repo.is_some() && cwd.is_some() && cwd.starts_with(root)`, then the mapping tier, then
      byte-equal name match, then the count; fold several agents on one change by the fixed
      precedence `Blocked > Working > Idle > Done > Unknown`.

- [x] 3.3 REFACTOR: Extract the precedence to one total `rank` function beside
      `decode_entry`'s status `match`, so the order is written once — or record that the
      implementation already has exactly one.

- [x] 3.4 VERIFY: `testcount --lib 'agents::tests::attribute::' 17`, then `MIN=22 sh
      $CHECKS/AGENTSEAM.sh`, `MIN=22 sh $CHECKS/NOLIT-CHANGE.sh`, `MIN=22 sh
      $CHECKS/NOSPAWN-GREP.sh`, and
      `HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' SCAN_MIN=86 sh
      $CHECKS/NODEFAULT-UI.sh` — the last must now be **green** for the first time. Then plant
      `#[derive(Default)]` on `Attribution`, re-run it, record exit **1**, and revert. Commit.

## 4. `Dashboard::attribution()`
<!-- kind: behavior -->

- [ ] 4.1 RED: Write `ui::app::tests::attribution_derives_the_three_tiers`,
      `attribution_follows_adopt_by_name`, `attribution_ignores_the_filter`, and
      `dashboard_names_agent_names_at_every_site` — the last an exhaustive destructuring of
      `Dashboard` with no `..` rest — and extend the landed
      `ui::app::tests::dashboard_is_clone_and_eq_with_agents` to carry a non-empty
      `agent_names`.

- [ ] 4.2 GREEN: Give `attribution()` its real body: build the change-name slice from
      `changes.active` and `changes.archived` in full — never the filtered `visible()` — and
      call `agents::attribute` with `repo`, `agents.agents`, and `agent_names.names`.

- [ ] 4.3 CHECK: Contract gate — re-inspect `Dashboard`'s published shape against design.md →
      Contracts and confirm the eleventh field broke every construction site at compile time,
      and that `Change`, `ChangeSet`, and `Action` are untouched so `from_files` and `from_cli`
      need no new agreement and `no_action_mutates_changes`' exact count holds at thirteen.

- [ ] 4.4 VERIFY: `testcount --lib 'ui::app::tests::' 63`,
      `testcount --lib 'ui::app::tests::attribution_' 3`, and
      `testcount --lib 'ui::app::tests::dashboard_names_agent_names_at_every_site' 1`, then
      `SCAN_MIN=165 TYPES='Dashboard Filter Detail Refresh' sh $CHECKS/NODEFAULT-UI.sh`,
      `sh $CHECKS/NOIO-VIEW.sh`, and `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`. Commit.

## 5. The badge cell in `ui::list`
<!-- kind: behavior -->

- [ ] 5.1 RED: Write `ui::list::tests::a_badged_active_row_at_both_widths`,
      `a_badged_archived_row_at_both_widths`, `an_unattributed_agent_badges_nothing`,
      `a_badged_row_drops_the_badge_first`, and `a_badged_archived_row_drops_the_badge_first`,
      asserting the exact strings `change-rows`' delta gives at 38 and the badge's 0-based
      column index at 58. **In the same task**, extend the five landed row-grammar tests
      `EXTENDED`'s pair list names — `active_row_grammar_at_38_and_58`,
      `every_row_is_exactly_the_requested_width`, `a_long_name_is_truncated_with_an_ellipsis`,
      `refresh_and_change_problems_in_order`, and `the_four_message_states_are_distinct` — with
      their badge forms, keeping every existing assertion, so every new assertion in this group
      is observed red before 5.2 makes it pass. Each test names both 38 and 58 **unsuffixed**,
      or `LISTWIDTHS` reports it missing a width.

- [ ] 5.2 CHECK: Confirm all ten failures are the missing badge, not a broken fixture: each
      names the badge column or the badge character in its diff, and no test fails to compile.

- [ ] 5.3 GREEN: Give `active_style_row` and `archived_row_text` an optional badge character,
      offered only together with the progress cell and dropped whole with its separating space
      before it, so a row with no badge is byte-identical to today's.

- [ ] 5.4 VERIFY: `testcount --lib 'ui::list::tests::' 25`, plus the four single-test floors
      `testcount --lib 'ui::list::tests::a_badged_active_row_at_both_widths' 1`,
      `…a_badged_archived_row_at_both_widths' 1`,
      `…an_unattributed_agent_badges_nothing' 1`, and
      `…a_badged_archived_row_drops_the_badge_first' 1`. Then
      `LIST_MIN=25 sh $CHECKS/LISTWIDTHS.sh` — green for the first time — and
      `sh $CHECKS/NOIO-VIEW.sh`. Commit.

## 6. The footer count in `ui::view`
<!-- kind: behavior -->

- [ ] 6.1 RED: Write `ui::view::tests::badged_rows_render_at_both_widths`,
      `the_unattributed_count_is_the_last_hint`, `the_count_drops_before_the_key_hints`,
      `the_count_survives_a_filter`, `the_count_is_reported_with_an_empty_list`, and
      `an_unreachable_socket_renders_the_agentless_pane`. **In the same task**, extend the
      three landed view tests `EXTENDED`'s pair list names —
      `the_prompt_replaces_the_hints_while_filtering` and `an_accepted_query_leads_the_hint_list`
      with the count forms, and `agents_change_no_pixel` with the absent-`cwd` case and the
      `assert_ne!` control that moves one agent into the repository — so every new assertion is
      observed red first. Each test names both 60 and 120 unsuffixed, or `WIDTHS` reports it
      missing a width.

- [ ] 6.2 CHECK: Confirm the nine failures are the missing count and the missing badge, not a
      broken fixture, and that `agents_change_no_pixel`'s `assert_ne!` is the one failing for
      the opposite reason — it is red because the badge does not exist yet.

- [ ] 6.3 GREEN: Append `<n> unattributed` as the last entry of the hint list `render_footer`
      builds, only when the count is non-zero, so `fit_hints` drops it first; `render_footer`
      takes the `Dashboard` rather than the `Filter` alone, since it now needs both.

- [ ] 6.4 VERIFY: `testcount --lib 'ui::view::tests::' 88` plus the five single-test floors for
      the new functions, then `WIDTHS_MIN=88 sh $CHECKS/WIDTHS.sh` — green for the first time,
      and the first run of this gate at a real floor — plus `sh $CHECKS/EXTENDED.sh`, green for
      the first time, and `sh $CHECKS/NOIO-VIEW.sh`. Commit.

## 7. The composition root reads the mapping
<!-- kind: behavior -->

- [ ] 7.1 RED: Write `ui::tests::load::load_reads_the_agent_name_mapping` and
      `ui::tests::load::a_none_state_dir_is_an_empty_mapping`, over real `ScratchDir` trees on
      `state::tests::`' landed terms — one holding a well-formed `agent-names.toml`, one holding
      invalid TOML whose problem must survive onto `Mapping::problems` — and assert the mapping
      through `attribution()` as well as through the field. **In the same task**, extend the
      three landed `ui::tests::wiring::` tests and the four landed `ui::tests::load::` tests
      with the assertions the `agent-poller` and `dashboard-loop` deltas add — the unreachable
      run's no-badge, no-count, and state-directory byte-identity claims; the no-repository
      run's empty `agent_names` and no-count claim; and `load`'s own empty-`agent_names` and
      state-directory-untouched claims — so every new assertion is observed red first.

- [ ] 7.2 GREEN: Make `load` read `state::read(state_dir)` onto `agent_names` on both branches.
      Delete `Startup`'s `too_many_arguments` sentence and state cohesion in its place, per
      design.md → Contracts.

- [ ] 7.3 CHECK: Confirm the two new `ui::tests::load::` failures name the empty mapping rather
      than a missing fixture, and that the extended wiring assertions were red for the same
      reason.

- [ ] 7.4 CHECK: Contract gate — re-inspect `Startup`, `load`, and `render_footer` against
      design.md → Contracts, and confirm `run` still holds no branch, no loop, and no field
      selection.

- [ ] 7.5 VERIFY: `testcount --lib 'ui::tests::load::' 9` plus the two single-test floors, then
      `sh $CHECKS/WIRED.sh`, `sh $CHECKS/NOIO-VIEW.sh`, and `UI_MIN=11 EXTRA='src/watch.rs
      src/refresh.rs src/agents.rs' sh $CHECKS/READONLY-UI.sh`. Commit.

## 8. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 8.1 VERIFY: `testcount --lib 'ui::tests::wiring::' 4` — all four pass, and the assertion
      message recorded in task 2.3 no longer appears.

- [ ] 8.2 CHECK: Run the six plants named in `agent-poller`'s "The wiring test fails when the
      poller is replaced by the inert double", one at a time, recording the exit status and the
      failing assertions verbatim for each, and reverting after each: `agents::none()`,
      `watch::none()`, `refresh::none()`, `load` passing `Mapping::default()`, `list::rows`
      ignoring `attribution()`, and `render_footer` ignoring `attribution()`.
      **Red when:** any plant leaves the suite green, or two plants fail the identical *set* of
      assertions — three of the six necessarily share the footer assertion, and the badge
      assertions are what must separate them.

- [ ] 8.3 CHECK: Prove the footer assertion fails in the **other** direction too: delete the
      `cwd.starts_with(root)` scope test in `agents::attribute`, run
      `testcount --lib 'ui::tests::wiring::' 4`, record that the footer read `2 unattributed`
      rather than `1`, and revert.
      **Red when:** the count is still `1` — the out-of-scope agent is not reaching the
      function, and the scenario is not testing what it claims.

- [ ] 8.4 VERIFY: `make check` — the first unqualified run since group 2. Commit.

## 9. The architecture checks, all green together
<!-- kind: operational -->

- [ ] 9.1 CHECK: Run all twenty-eight gates at their landed invocations, in one pass, and
      record every exit status and OK line: `MIN=22 NOSPAWN-GREP`; `MIN=22 NOLIT-CHANGE`;
      `UI_MIN=11 NOCLI-SHELL`; `NOIO-VIEW`; `READSEAM`; `MDSEAM`; `NOTABSEAM`; `TASKSEAM`;
      `MIN=24 WATCHSEAM`; `MIN=22 AGENTSEAM`; `NOBLOCK`; `SLEEP_MIN=5 MIN=25 NOSLEEP`;
      `UI_MIN=11 EXTRA='src/watch.rs src/refresh.rs src/agents.rs' READONLY-UI`; `WIRED`;
      `EXTENDED`; both `NODEFAULT-UI` invocations (`SCAN_MIN=165`, and
      `HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' SCAN_MIN=86`);
      `WIDTHS_MIN=88 WIDTHS`; `DETAILWIDTHS`; `LIST_MIN=25 LISTWIDTHS`; `MDWIDTHS`;
      `TASKWIDTHS`; `NORAW-GREP`; `NOWAIVER`; `NOJSON-SEAM`; `GATE-MECH1.py`; `WORK=... DEPS`;
      `GRAPH-SNAP`; `BASE=$BASE CHANGE=agent-attribution OPENSPEC-UNTOUCHED`.
      **Red when:** any is not green, including `DEPS` and `GRAPH-SNAP`, which this change does
      not edit because it adds no dependency.

- [ ] 9.2 CHECK: Re-measure both `NODEFAULT-UI` span counts from their own FAIL lines at a
      deliberately high `SCAN_MIN`, and confirm the landed floors of 165 and 86 are at or below
      them.
      **Red when:** either realized count is below its floor — a literal was replaced by
      something the scan cannot see.

- [ ] 9.3 VERIFY: `git status --porcelain` shows no change under `openspec/` outside this
      change's own directory. Commit any gate-invocation notes.

## 10. SPEC.md and AGENTS.md corrections
<!-- kind: operational -->
<!-- parallel-after: 0 -->

- [ ] 10.1 CHECK: Grep each target passage before editing and record what is there today:
      `SPEC.md` → "Attributing an agent to a change" (three numbered tiers, scope stated for
      tier 3 alone), → "List view" (the paragraph reserving the third column), → Degraded
      states (no row for an absent or out-of-repository `cwd`, none for a worktree); and
      `AGENTS.md` → the "Do not attribute an agent to a change on weak evidence" paragraph.
      **Red when:** a passage is not where this task says — the edit below would then be
      appending a second entry beside a rule rather than rewriting it.

- [ ] 10.2 Rewrite in `SPEC.md`: Herdr integration → Attributing an agent (audience: every
      later change and every agent planning one) — state that `herdr agent list` is
      session-global and that **every** tier is scoped to the repository, not tier 3 alone;
      that `cwd` is optional and an agent without one is out of scope; and that tier 1 is
      consulted before tier 2 and falls through when it names a change that no longer exists.
      Replaces the current text, which scopes only tier 3 and states no precedence.

- [ ] 10.3 Rewrite in `SPEC.md`: User interface → List view (audience: `degraded-states` and
      any later change touching the row grammar) — replace the paragraph reserving the third
      column with the badge's actual grammar, its glyphs, and its position in the drop order,
      show it in the mock, and add the footer's `<n> unattributed` hint. Name the collision
      `degraded-states` inherits: it plans a per-change problem indicator in the same column on
      an archived row, and must place it elsewhere or specify a precedence.

- [ ] 10.4 Add to `SPEC.md`: Degraded states (audience: as above) — one row for an agent whose
      `cwd` is absent or outside the repository (neither badged nor counted), and one for an
      agent working in a **linked worktree** of this repository, which Herdr places outside the
      root and which this design deliberately does not attribute.

- [ ] 10.5 Rewrite in `AGENTS.md`: Architecture rules, the existing "Do not attribute an agent
      to a change on weak evidence" paragraph (audience: every future session) — keep the rule
      and add the two facts a future change would otherwise re-derive: the agent list is
      session-global, so repository scope is the plugin's job and not Herdr's; and the tier-2
      match is on `name`, never on `agent`, which is the agent kind. Rewrite in place; add no
      second entry.

- [ ] 10.6 VERIFY: `BASE=$BASE CHANGE=agent-attribution sh $CHECKS/OPENSPEC-UNTOUCHED.sh` is
      green — `SPEC.md` and `AGENTS.md` are outside `openspec/`, and
      `openspec/IMPLEMENTATION-ORDER.md` is deliberately **not** edited here, since nothing
      writes inside `openspec/` except this change's own directory; the roadmap row is
      `openspec archive`'s to correct. Commit.

## 11. Change Review
<!-- kind: operational -->

- [ ] 11.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against
      proposal.md, every spec scenario, design.md, and tasks.md, given the diff and the
      artifacts only. Require it to write findings to a scratchpad file as it goes rather than
      returning them only in a final message.
      **Concentration points, per `openspec/config.yaml`:** nothing spawns outside `cli`;
      views perform no I/O; nothing writes inside `openspec/`; every external dependency has an
      absent case; **attribution must not guess — the test proving an in-repository agent with
      no matching name is counted rather than assigned must exist and must be able to fail**;
      `from_files` and `from_cli` still produce the same `Change`; view tests run at 60 and
      120; agent names are capped at 32 characters and match `[a-z][a-z0-9_-]{0,31}`.

- [ ] 11.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run every affected test and gate.

- [ ] 11.3 VERIFY: No blocking or unowned finding remains, and every artifact that a fix
      contradicted has been refreshed. Commit.

## 12. Lint & Verify
<!-- kind: operational -->

- [ ] 12.1 CHECK: Inspect the intended verification commands and affected tiers, and confirm
      each one can fail: every `testcount` invocation carries a minimum above the count that
      existed on `main`, every single-test filter names its function in full, and every gate
      carries its explicit floor rather than a block default.

- [ ] 12.2 VERIFY: `cargo fmt --all -- --check` — clean.

- [ ] 12.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings.
      This crate has no separate type checker; `cargo clippy` compiles and is that step.

- [ ] 12.4 VERIFY: `cargo test --all-features` — green, and
      `cargo test --all-features --lib -- --list | grep -c ': test$'` reports **840**.

- [ ] 12.5 VERIFY: `cargo llvm-cov --fail-under-lines 80`, and record the **line** percentage
      from the TOTAL row's line columns — never the region count, which leads that row.

- [ ] 12.6 VERIFY: `make check` — the single gate, green. If it fails, report the failing
      sub-command rather than a summary.

- [ ] 12.7 VERIFY: `openspec validate agent-attribution --strict`, with
      `export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"` in the shell. Commit.
