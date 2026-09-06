## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/agent-list/spec.md` (new capability)
- `specs/agent-poller/spec.md` (new capability)
- `specs/dashboard-loop/spec.md` (delta, 2 MODIFIED requirements)
- `specs/live-updates/spec.md` (delta, 1 MODIFIED requirement)
- `specs/subprocess-seam/spec.md` (delta, 1 ADDED requirement)
- `specs/watch-invalidation/spec.md` (delta, 1 MODIFIED requirement)

The finding pass was delegated to **four independent reviewers**, none of which wrote the
planning package and none of which was a fork of the planning session, sliced as
(A) capability coverage, scenario quality, and cross-artifact contradictions; (B) design
completeness, test boundaries, and whether each proposed check could fail at all;
(C) task alignment, lifecycle discipline, and `parallel-after` independence; and
(D) factual verification of every empirical claim, by running the command or reading the
source. Reviewers B and D worked against scratch copies of the tree and ran the checks; the
repository was not modified by any of them. This session merged the findings and made every
repair below.

## Reviewed Against

- This repository HEAD: `aec6cf54b0dc2bd44cd22d6bf8ad520f47e52847`
- Sibling repository HEAD: `Not applicable` — Herdr is consumed as an installed binary
  (0.8.2), not as a source dependency; its contract was verified by running it, and its own
  JSON schema was read from `herdr api schema --json`. `~/Code/openspec-schemas` supplies the
  vendored `tdd` schema and is not changed by this change.
- Working tree: clean apart from `openspec/changes/agent-polling/`, this change's own
  artifact directory, which is untracked and intentionally included.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/agent-poller`, `design.md`, `tasks.md` | The acceptance test's **watcher** assertion could not fail. `refresh.problems` being empty was named as the discriminator, but `watch::start` returns an empty `problems` on success and `watch::none()` is wired with an empty one too — both arms satisfied it, so the outer-loop test built to catch an unwired collaborator was green with the watcher unwired. | The readiness predicate now writes one byte into the change directory after both scratch programs have logged, and the watcher's discriminator became the scratch `openspec` program's log reaching a **second** entry, which exists only if a real watcher reported the write. `refresh.problems` is retained as a supporting assertion and labelled as one. | `specs/agent-poller/spec.md` → "The real wiring polls…", "…fails when the poller is replaced…"; `design.md` → Test Strategy, Decisions 14, Test Boundaries; `tasks.md` 2.2, 10.3 |
| CRITICAL | `tasks.md` | Task 9.3's gate `grep -rn 'too_many_arguments' src/` asserts an empty result, but `src/changes.rs:966` already carries `#[allow(clippy::too_many_arguments)]` — the gate was red before and after the change. | Scoped to the diff: `git diff $BASE -- src/ tests/ \| grep '^+.*#\[allow'` is empty, with the reason for not using a tree-wide grep recorded. | `tasks.md` 9.4 |
| CRITICAL | `tasks.md` | `OPENSPEC-UNTOUCHED` was listed as extracted byte-identically, but its single exclusion is the hardcoded literal `^openspec/changes/live-refresh/`; run as prescribed it reports all ten of this change's own artifact files as stray writes and is red from task 0.3 onward. | Moved into the edited set with a two-part replacement parameterising the exclusion as `CHANGE` (default `agent-polling`) plus a guard rejecting a bogus or empty name and a non-existent directory. Widening to `^openspec/changes/` was considered and rejected as neutering the gate. Verified: unedited red on this change's files, edited green, red on a planted `openspec/specs/STRAY.tmp`, red on `CHANGE=nope`. | `tasks.md` → "Checks edited by this change", 0.3, 0.4, 0.6 |
| CRITICAL | `design.md`, `specs/dashboard-loop` | Two design rationales rested on a false measurement: clippy's `too_many_arguments` was said to fire at seven parameters. Measured on this crate and toolchain it fires at **eight**, and the crate's one `#[allow]` produces no warning when removed. A seventh `run_loop` parameter would not have tripped anything. | The decisions (a third field on `Live`, a `Startup` struct) are unchanged and now stand on cohesion. The false forcing constraint is retired in place, with the measurement recorded so a later change does not inherit it. | `design.md` → Boundaries, Decisions 8; `specs/dashboard-loop/spec.md` → "The loop draws before it waits"; `specs/agent-poller/spec.md` |
| WARNING | `tasks.md`, `specs/subprocess-seam` | `AGENTSEAM` leg 3 swept for `HerdrCli` alone. `agent_cli_via` returns `Arc<dyn HerdrCli>` and inference hides the type: a planted `crate::cli::agent_cli_via(...)` in `src/ui/list.rs` reported `AGENTSEAM OK`. | Leg 3's pattern is now `HerdrCli\|RealHerdrCli\|agent_cli_via` and `ALLOWED` names the three files that may reach the handle, `src/ui/mod.rs` included because the composition root legitimately calls it — `NOCLI-SHELL` independently forbids the trait's name across all of `src/ui/`, so neither check is weakened. Re-verified green, and red on both plants. | `tasks.md` → AGENTSEAM block; `specs/subprocess-seam/spec.md` |
| WARNING | `tasks.md` | `WIRED` had no guard that `prod()` actually cut a test module. With the seven required names present only inside `mod tests` and a single trailing space after `#[cfg(test)]`, an entirely unwired `start_collaborators` reported `WIRED OK`. | Added a Guard D analogue requiring exactly one line-anchored `#[cfg(test)]` in `src/ui/mod.rs`, matching what `NOBLOCK` already does for its seam modules. Re-verified red against that exact plant, and it is now one of `WIRED`'s six plants at task 10.4. | `tasks.md` → WIRED block, 10.4 |
| WARNING | `tasks.md` | Task 1.7's contract gate listed six paths and not `src/agents.rs`, which is untracked at that point, so `git diff` would have shown nothing for ten of the twelve new `pub` items. | Added the file to the path list, prefixed by `git add -N src/agents.rs`, with an explicit red-when clause for an empty diff on that file. | `tasks.md` 1.7 |
| WARNING | `design.md` | The verification matrix's Command column was not runnable: `testcount` needs `--lib` as its first argument; `NOCHANGELIT.sh` does not exist (`NOLIT-CHANGE.sh` does); `HOME=` would override the user's home directory; `READONLY-UI` was invoked without `EXTRA`, so it could not fail for what its row claimed; and `ui::app::tests::exhaustive 6` matched zero tests. | Every command corrected against the extracted scripts and against `tasks.md`, which was right in each case. | `design.md` → Test Strategy |
| WARNING | `specs/agent-poller` | `poller_for_test` was described inconsistently — one scenario had the test read the worker's answer on a raw receiver *and* `drain` return that same snapshot, which a single-consumer `mpsc` cannot do. | Pinned to `refresh::worker_for_test`'s established shape: the helper's poller yields nothing and the answers arrive on the raw receiver. The three scenarios needing `drain`'s own return value now use `agents::start` instead. | `specs/agent-poller/spec.md` → the seam requirement and its four scenarios |
| WARNING | `specs/agent-poller`, `tasks.md` | The `repo: None` branch of `start_collaborators` was unspecified and untested, leaving open whether the **poller** is inert with no repository — which would blind a standalone pane to agents that exist regardless of any OpenSpec repo. | Specified: the watcher and the worker are skipped without a repository, the poller runs unconditionally. Added a scenario and a third acceptance test. | `specs/agent-poller/spec.md` → the composition-root requirement and "A pane with no OpenSpec repository still polls for agents"; `tasks.md` 2.4, 10.1 |
| WARNING | `specs/agent-poller` | "Polling SHALL continue at the same interval after a failure of any kind" — design.md's Decision 5 and the change's central resilience claim — had no scenario and no task. The absent case was tested; the return from absent was not. | Added a scenario driving a counter-file scratch program that fails once and then succeeds, asserting both snapshots and a two-run log. | `specs/agent-poller/spec.md` → "An unreachable socket is recovered from, not backed off"; `design.md` matrix; `tasks.md` 7.1 |
| WARNING | `specs/agent-list`, `specs/dashboard-loop` | `AgentStatus` was required to have no `Default`, but every `NODEFAULT-UI` invocation covers only structs and the check's positive control is anchored on `struct <T> {`; and `Listed` was missing from the exhaustive-destructuring companions. | `AgentStatus` is now an explicit, reasoned exemption; `Listed` was added, taking the companions from six to seven. | `specs/agent-list/spec.md`; `specs/dashboard-loop/spec.md` → "`Dashboard` has no `Default`…" |
| WARNING | `specs/dashboard-loop` | The scenario "`Live` cannot be built without naming the poller" was verified by "the crate compiling at all", which cannot fail, and by a `NODEFAULT-UI` extension that is impossible: the check's control anchors on `struct T {` and `Live` is generic over a lifetime. | Replaced with a compile-time exhaustive-destructuring companion in `ui::driver`'s tests, and the reason `NODEFAULT-UI` cannot reach `Live` is stated. | `specs/dashboard-loop/spec.md`; `tasks.md` 9.1 |
| WARNING | `tasks.md` | Group 8's RED tasks could not go red: everything they assert landed in group 1, and nothing renders the field by design. | Reclassified `operational` with CHECK → CHANGE → VERIFY, per the schema's explicit instruction to label such work rather than invent a RED that cannot fail. The deterministic failing check is the planted `Default` and the planted elision. | `tasks.md` group 8 |
| WARNING | `tasks.md` | Groups 4, 5, 6, 8, and 9 ran RED → GREEN → VERIFY with no REFACTOR phase and no statement that none was needed. Group 5 also enumerated four tests against a floor of three, one of which asserted the value of a constant the implementation declares. | REFACTOR tasks (or explicit "none needed" records) added to all five; group 5 is three behavioural tests and the constant is pinned by `WIRED`'s positive control instead. | `tasks.md` 4.3, 5.1, 5.3, 6.3, 8.2, 9.3 |
| WARNING | `tasks.md` | Group 1's CHARACTERIZE pinned "the 752 landed tests are green", but task 1.5 splits `ui::run` — the one function the landed suite never drives, which is why `live-refresh` shipped it unwired. The characterization was unable to fail for the group's riskiest edit. | Recorded in 1.1 that `ui::run` is uncharacterized, that group 2's acceptance tests are its deferred characterization, and that 1.8's green `make check` is therefore not evidence about the split. | `tasks.md` 1.1 |
| WARNING | `tasks.md` | Group 12 had no CHECK before its CHANGE tasks, so its closing grep could be satisfied by a pattern that never matched anything. Its AGENTS.md target was also wrong — that file carries no "only clock binding" claim — and its second closing grep would have gone **red exactly when task 12.2 was done correctly**, since 12.2 requires `--json` and Herdr to appear on one line. | Added 12.1 as a before-grep over every target, retargeted the AGENTS.md task to the two-seam-module wording it actually carries, added SPEC.md's "the crate's only one" thread claim to the corrections, and deleted the inverted grep with the reason recorded in its place. | `tasks.md` group 12 |
| WARNING | `tasks.md` | The `NOLIT-CHANGE` plant that `specs/dashboard-loop` and `design.md` both require — a `Change {` literal in `src/agents.rs` — had no task; only a green run. | Added as task 8.4, alongside the `AGENTSEAM` implementation-time plants added as 10.5. | `tasks.md` 8.4, 10.5 |
| WARNING | `tasks.md` | The stated reason for having no `parallel-after` markers was the red acceptance test, but groups 5 and 6 share no file and neither depends on the other. | The reason was wrong and is corrected: `cargo test` builds the whole crate, so a half-written file in either group fails the other's `testcount` and the failure is not attributable. Shared compilation, not shared files, is the blocker. | `tasks.md` → "Group ordering" |
| WARNING | `tasks.md` | The `AGENTSEAM` searched-set row said 22 → 23; it is 22 both before and after, because `src/agents.rs` is added to the tree and excluded from the sweep in the same change. Task 0.1 halts on any mismatch, so the wrong figure would have stopped implementation on its first command. | Corrected, with the reason for the flat count written into the row. | `tasks.md` → "Measured at planning time" |
| WARNING | `design.md`, `tasks.md` | Two substantive decisions lived only in the checklist: the repair of `NODEFAULT-UI`'s half B (which changes what a landed gate means for every future change) and the suspension of unqualified `make check` across groups 2 to 10. | Both moved into `design.md` → Decisions 12 and 13; `tasks.md` now cites them. | `design.md` → Decisions; `tasks.md` → How to read this file, "Checks edited" |
| SUGGESTION | `specs/agent-list` | No scenario covered an entry in the `agents` array that is not a JSON object. | Added, alongside widening the skip rule to name it. | `specs/agent-list/spec.md` → "An entry that is not an object is skipped, not fatal" |
| SUGGESTION | `design.md` | `Agent` omits `terminal_id`, which Herdr's schema marks required, and the omission was unexplained; the parser's required set was also described as "the three identity fields Herdr's own schema marks required" when the schema marks seven. | Both corrected, with the omission recorded as Decision 11 so `agent-launch` inherits a reason rather than a surprise. | `design.md` → Decisions 11, Risks; `specs/agent-list/spec.md` |
| SUGGESTION | `proposal.md`, `specs/agent-poller` | The proposal said the arguments are "exactly `list`" (they are `["agent", "list"]`), and one sentence said `ui::HERDR_PROGRAM` where every other artifact says `cli::` — following it would have turned `WIRED` leg 4 red. | Both corrected. | `proposal.md`; `specs/agent-poller/spec.md` |
| SUGGESTION | `tasks.md` | The `NOSLEEP` edit was the only check change with no proven-red plant, and plant 7.5(d)'s recorded expectation named the wrong leg — renaming `try_recv` to `try_recv_x` leaves the substring and stays green. | Added task 7.6 as the `NOSLEEP` plant, and 7.5(d) now requires the token renamed rather than wrapped, with the measured result recorded. | `tasks.md` 7.5, 7.6 |
| SUGGESTION | `tasks.md`, `design.md` | The second `NODEFAULT-UI` invocation's `SCAN_MIN` was specified circularly; two `ui::driver` scenarios omitted the mandated 120/60 widths. | `SCAN_MIN` is now measured from the invocation's own OK line at implementation time, never above it and never below 1; both widths are named in task 9.1. | `tasks.md` 8.5, 9.1 |

Findings deliberately **not** repaired, with the reason:

- **`WIRED` leg 1 sees names, not values**, so a call whose result is dropped passes it. The
  fix is not a better grep — it is the acceptance test, which is why design.md → Decisions 7
  says neither half suffices alone. Leg 1's failure message was rewritten to claim only what
  it checked rather than "a collaborator is unwired".
- **`WIRED` leg 2 misses a keyword-free decision** (`unwrap_or_else`, `?` on an `Option`).
  Recorded as a known limit inside the block rather than chased with a wider pattern that
  would false-red on ordinary code.
- **`AGENTSEAM` leg 2 forbids the word `Frame` in comments.** Kept comment-inclusive, on
  `WATCHSEAM`'s and `NOTABSEAM`'s established terms; the constraint is stated in the block.

## No Remaining Implementation-Blocking Gaps

None remain. All four CRITICALs are repaired and each repair was re-verified by running the
affected check against the plant it now catches. `openspec validate agent-polling --strict`
reports the change valid, the six spec files carry **62** scenarios, and `design.md`'s
verification matrix carries **62** rows whose scenario names match the specs' exactly in both
directions.

## Deferred Non-Blocking Notes

- **`terminal_id` is not parsed into `Agent`.** Deferred to `agent-launch` if it turns out to
  need one; the reason and the cost of adding it are recorded in `design.md` → Decisions 11.
- **`AGENTSEAM`'s `ALLOWED` list will need `src/launch.rs`.** That is a deliberate edit
  `agent-launch` makes, and the block says so at its point of definition.
- **`Agent.name` is the field `agent-attribution` will key on, and no parse test covers a name
  that is long, empty, or outside Herdr's `[a-z][a-z0-9_-]{0,31}` pattern.** Attribution is a
  named non-goal here, so the constraint belongs to the change that reads the field; recorded
  so it does not inherit an assumption that `name` is well-formed.
- **`SPEC.md` → Degraded states will need a further row when `agent-attribution` renders the
  agent column.** This change scopes the existing non-zero-exit row and adds the `herdr` one;
  the "agent column and action keys hidden" row is already true and stays untouched.
