## Context

`tests/degraded-coverage.toml` binds each row of `SPEC.md`'s degraded-states table to a proving
test and to a `covers` range naming the production code that implements it. Two checks read it:

- `tests/degraded_coverage.rs`, inside `cargo test`, checks the map's **shape** — conditions
  match the table, proofs resolve and are tests, tiers agree, ranges are in bounds.
- `scripts/coverage-prod.py`, under `make coverage`, checks that each range is **instrumented
  and hot**.

The second is unreachable while the suite is red, because `make coverage` runs the suite itself
and `check` is `fmt-check lint gates test coverage`. The first runs always — but its range rule
is one line that is non-empty and does not start with `//`, which a bare signature or a struct
field satisfies.

So the range that broke during `settings-window` — `src/ui/app.rs:830-843`, `pub struct
Dashboard`'s field declarations — passed the always-on check for the wrong reason and was never
reached by the check that would have caught it, for seven task groups.

## Goals / Non-Goals

**Goals.** Make a mis-bound range fail on every run, red suite or green. Report it in seconds
rather than after a multi-minute suite. Rebind every range the strengthened rule rejects.

**Non-Goals.** Moving either coverage floor, or their values. Running the floors on a red suite.
Touching `tests/doc_contract.rs`' mouse guard. Changing what any degraded-states row claims.
Adding a known-weak-rows list or a ratchet.

## Boundaries

No module under `src/` is touched, no view is added, no process spawn is introduced, and the
`Change` type is untouched — so `from_files`/`from_cli` agreement is not in question. The change
lives entirely in the checking tier and the prose that documents it: `Makefile`,
`.github/workflows/ci.yml`, `tests/degraded_coverage.rs`, `tests/degraded-coverage.toml`,
`tests/ci_workflow.rs`, `SPEC.md`, `AGENTS.md`, `README.md`, and `openspec/config.yaml` —
the last being the only edit inside `openspec/` outside the change's own directory, required
by `doc-conformance`'s machine-checked rule that every `check:` prerequisite be named in the
injected context block.

`covers-check` follows `gates`' own pattern — a named phony target, composed into `check`, run
individually by CI — but it is **not** a file under `scripts/gates/`. That directory's contract,
asserted in both directions by `tests/ci_workflow.rs`, is one recipe line per script and one
script per line; `covers-check` reads a test fixture and a coverage contract rather than
sweeping source for a hygiene property, and adding it there would also move the gate-script
count that `openspec/specs/quality-gates/spec.md` pins by equality.

## Contracts

No interface a separate consumer depends on changes. `herdr-plugin.toml`, the config format, and
every keybinding are untouched; the plugin binary's behaviour is identical before and after.
Additive, not breaking. The only consumers are this repository's own `make` targets and CI job
steps, both listed under Impact in the proposal.

## Persistence and Rollout

Migration: none. Backfill: none. Seeding: none. Cache invalidation: none. Index rebuild: none.
Authorization: none. Observability: none beyond the new target's own failure message.
Deployment: none — nothing ships in the binary.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The filesystem | real — the checker reads `tests/degraded-coverage.toml` and files under `src/` | real, against the tree itself. `crate::testutil::ScratchDir` is `#[cfg(test)] pub(crate)` in `src/lib.rs` and is **unreachable from `tests/`**; each integration test that needs one reimplements it, and `degraded_coverage.rs` has none. `validate_covers` also rejects paths outside `src/` and resolves them against `manifest_dir()`, so a synthetic source would never be read — fixtures are ranges of the real tree |
| The `openspec` binary | not involved | not involved |
| The Herdr socket | not involved | not involved |
| The terminal | not involved — no view is added | not involved |
| `cargo llvm-cov` | real, in the existing `make coverage` leg only | not involved: the structural rule reads no report, which is the point |
| `make` / the shell | **not spawned.** `tests/ci_workflow.rs` parses `Makefile`, `ci.yml`, `SPEC.md` and `AGENTS.md` as text — 21 tests in 0.00s, no `Command::new` anywhere. The only test that runs `make` is `tests/gate_controls.rs`, and only `make gates`, which never re-enters cargo | not involved |

## Test Strategy

This change takes **no outer-loop acceptance test**, and not only because it adds no runtime
behaviour. The composite cannot be driven from inside `cargo test` at all: `make check`'s `test`
target is `cargo test --all-features`, so a test in `tests/ci_workflow.rs` that ran real
`make check` on a tree copy would re-enter the whole suite in the copy, which would run
`ci_workflow` again, which would copy again — unbounded, each level dragging in
`gate_controls`' 65 tree copies and a cold `clippy` and `llvm-cov`. `gate_controls`' own
precedent is safe only because `make gates` never calls cargo.

So the two `make check`-ordering scenarios are verified **once, by hand, at apply time**, and
the plan says so rather than promising machinery it cannot have (tasks 2.10 and 2.11). What
remains permanently checked is the ordering itself, asserted against the `Makefile`'s parsed
prerequisite list by `tests/ci_workflow.rs` — text, not execution, and honest about being so.

The load-bearing half is the **strengthened rule**, not the new target: it runs inside `cargo
test` on every run, red suite or green, and it alone would have caught the `settings-window`
regression on the first task group. `covers-check` is the early-reporting half — it turns a
defect buried in a 1600-test run into a named gate that fails in seconds, before the suite.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| The map covers the table at HEAD | existing test, unchanged | unit | filesystem | `cargo test --test degraded_coverage` |
| A row added to SPEC without a proof fails the build | existing test, unchanged | unit | ScratchDir | `cargo test --test degraded_coverage` |
| A renamed test fails the binding rather than passing vacuously | existing test, unchanged | unit | ScratchDir | `cargo test --test degraded_coverage` |
| A `view` tier pointing at a test that renders nothing fails | existing test, unchanged | unit | ScratchDir | `cargo test --test degraded_coverage` |
| An empty table is a failure, not a vacuous pass | existing test, unchanged | unit | ScratchDir | `cargo test --test degraded_coverage` |
| A `proof` that is not a test fails the binding | existing test, unchanged | unit | ScratchDir | `cargo test --test degraded_coverage` |
| A `covers` range that does not resolve fails the binding | existing test, unchanged | unit | ScratchDir | `cargo test --test degraded_coverage` |
| `unproven` gains a consequence rather than a new verdict | existing test, unchanged | unit | filesystem | `cargo test --test degraded_coverage` |
| A range that is only a signature or only struct fields is rejected | new test: both HEAD shapes planted as fixtures, each asserted rejected and naming the row's `condition` | unit | ScratchDir | `cargo test --test degraded_coverage` |
| A range that is only a signature or only struct fields is rejected | negative control: a retained `legacy_holds_code` predicate is asserted to **accept** both ranges, so the strengthening is shown to be what catches them | unit | the tree itself | `cargo test --test degraded_coverage` |
| A range of real statements is accepted whatever it starts with | new test: a range starting at a `fn` line but including its guard body is accepted; the `fn` line alone is rejected | unit | the tree itself | `cargo test --test degraded_coverage` |
| A range that is only a signature or only struct fields is rejected | scanner control: a fixture holding a `{` inside a string literal, asserted not to desync field recognition — the failure mode is vacuous acceptance | unit | the tree itself | `cargo test --test degraded_coverage` |
| An uncovered degraded path fails the coverage run | existing behaviour, unchanged | integration | `cargo llvm-cov` | `make coverage` |
| Deleting the test that drives a degraded path is caught | existing behaviour, unchanged | integration | `cargo llvm-cov` | `make coverage` |
| The range check cannot pass vacuously | existing tests in `tests/coverage_prod.rs`, unchanged | unit | fixture maps | `cargo test --test coverage_prod` |
| The structural half reports while the suite is red | manual, once, at apply time (task 2.10) — the composite cannot run inside `cargo test` without unbounded recursion. Permanently checked: `covers-check` precedes `test` in the parsed prerequisite list | integration (manual) + unit | `Makefile` text | task 2.10; `cargo test --test ci_workflow` |
| All gates pass on a clean tree | existing test extended to name `covers-check` in the parsed prerequisite order — `Makefile` text, not a run | unit | `Makefile` text | `cargo test --test ci_workflow` |
| Format gate fails and stops the run | existing test extended: `covers-check` among the commands named as not run | unit | `Makefile` text | `cargo test --test ci_workflow` |
| Lint gate fails on a clippy warning | existing test, unchanged | unit | `Makefile` text | `cargo test --test ci_workflow` |
| The hygiene gates fail before the test and coverage runs | existing test, unchanged | unit | `Makefile` text | `cargo test --test ci_workflow` |
| The command table names the gates tier that exists | existing test extended: six prerequisites, each named in `SPEC.md` → Gates | unit | `SPEC.md`, `Makefile` | `cargo test --test ci_workflow` |
| The binding check runs although the suite is red | manual, once, at apply time (task 2.10), for the same recursion reason | integration (manual) | real `make`, working tree | task 2.10 |
| All five gates run on each runner | existing `ci_workflow` test extended: the `check` job's step list names `make covers-check` between `make gates` and `make test` | unit | `ci.yml` text | `cargo test --test ci_workflow` |
| The matrix names both runners and no others | existing test, unchanged — carried by the REMOVED+ADDED pair, not altered by it | unit | `ci.yml` text | `cargo test --test ci_workflow` |
| A failing gate fails the run rather than being skipped | existing test, unchanged | unit | `ci.yml` text | `cargo test --test ci_workflow` |
| One runner's failure does not cancel the other | existing test, unchanged (`fail-fast: false`) | unit | `ci.yml` text | `cargo test --test ci_workflow` |
| The coverage job is Linux-only and runs the gate once | existing test, unchanged — the MODIFIED block moves one prose count, no behaviour | unit | `ci.yml` text | `cargo test --test ci_workflow` |
| The coverage job installs what `make coverage` needs | existing test, unchanged | unit | `ci.yml` text | `cargo test --test ci_workflow` |
| The production floor reaches CI without a workflow edit | existing test, unchanged | unit | `ci.yml` text | `cargo test --test ci_workflow` |
| The floors are not moved ahead of the suite | manual, once, at apply time (task 2.11): a below-floor tree, `make covers-check` exits 0 leaving no report, `make coverage` exits non-zero | integration (manual) | real `make` | task 2.11 |

## Decisions

**1. The structural rule lives in Rust, not Python — one implementation, not two.**
`covers-check` runs the existing `tests/degraded_coverage.rs` binary alone
(`cargo test --all-features --test degraded_coverage`) rather than a new script. The
alternative — a `--covers-only` mode on `scripts/coverage-prod.py` — would put the same rule in
two languages, and this repository's standing rule is that two implementations of one fact
drift. It would also need its own positive control. Running one test binary early costs a
compile it was going to pay anyway.

**2. The rule is syntactic, and conservative by choice.** It approximates what llvm-cov will
instrument without a report. It can reject a range a report would have instrumented; the author
widens the range and moves on. It is never allowed to *accept* a range on the grounds that the
file compiles. The hotness half stays in `make coverage`, where a real report exists. Rejected
alternative: `cargo llvm-cov --ignore-run-fail` to get a report from a red suite — it still runs
the whole suite, so it buys accuracy at the cost of the seconds-versus-minutes property that is
half the point, and a partial run's instrumentation set is itself partial.

**3. The floors do not move.** A red suite means paths legitimately did not execute, so the 80%
and 96% floors would fail for reasons unrelated to the change under way. A gate that fires for
reasons the reader learns to discount is worse than one that fires late. Stated as its own
scenario so a later change cannot quietly promote them.

**4. Every rejected row is rebound in this change; no ratchet.** `EXTENDED` is this
repository's own worked example of a pinned per-change list outliving its intent and becoming a
permanently red step nobody may compose into `make check`. A `KNOWN_WEAK_ROWS` list would take
the same path. The audit is the bulk of the work and that is accepted.

**5. `NODEFAULT-UI`'s type-set count is corrected from five to nine in the same edit.** The
`make check` requirement is MODIFIED here, and a MODIFIED block replaces the requirement
verbatim on archive. The live text says five; the `Makefile` carries nine. `settings-window`'s
change review caught exactly this class of defect twice, in requirements it was re-authoring for
other reasons — so it is corrected rather than reproduced.

## Risks / Trade-offs

- **The syntactic rule will have false rejections.** A range of nothing but a `match` arm's
  patterns, or a long multi-line call split so that some line is a lone delimiter, may be
  rejected. Mitigated by Decision 2's direction of error: the fix is always to widen the range,
  which makes the binding stronger rather than weaker.
- **The ordering guarantee is unverified after apply.** The two `make check`-ordering scenarios
  are manual plants performed once during implementation (tasks 2.10, 2.11); nothing durable
  re-checks that `covers-check` still runs before `test`, because the composite cannot run
  inside `cargo test`. What remains checked is the parsed prerequisite **order** in
  `tests/ci_workflow.rs` — text, not execution. A change that reordered `check:` and updated
  that vector would pass. This is a real residue and is recorded rather than papered over: the
  alternative was a matrix row asserting a test no task could write.
- **The audit touches several capabilities' proving tests.** Rebinding a range may reveal that a
  row's `proof` watches something other than what its `why` claims. Where that happens the row's
  `why` and `proof` are the contract and the range moves to match them; a row whose proof is
  genuinely wrong is reported and left for its own change rather than silently re-aimed.
- **`covers-check` adds a compile to `check` before `test`.** In practice it is the same
  compilation `test` performs moments later and is cached, so the cost is close to zero on a
  warm tree and one test binary's link on a cold one.
- **The rule finds shape, not aboutness — and aboutness is the larger defect.** Measured at
  HEAD, the rule flags 1 of 74 ranges. It rejects the `settings-window` regression shape
  (`app.rs:830-843` at `bfe7e62`, struct fields) — but `fold_glyph` under "No `openspec/` found"
  holds `if collapsed { '▸' } else { '▾' }` and `detail_row_role` holds a `match`, so both pass
  the rule while being bound to code unrelated to their row. A gate cannot ask whether a range
  is *about* its condition. Group 3 is therefore a human audit of all 74, and the rule is its
  floor rather than its substitute. Stated here because the opposite reading — "the gate is
  green, so the bindings are right" — is precisely the failure this change exists to correct.

## Migration

None. No data, no on-disk format, and no consumer outside this repository.

## Open Questions

None. The two that would have been — what runs on a red suite, and whether rejected rows are
fixed now or ratcheted — were decided before this document was written and are recorded as
Decisions 3 and 4.
