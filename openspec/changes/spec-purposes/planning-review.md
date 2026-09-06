## Reviewed Artifacts

- `proposal.md`
- `specs/quality-gates/spec.md`
- `specs/ci-workflow/spec.md`
- `design.md`
- `tasks.md`
- `notes/purposes-draft.md`, `notes/extracted-gates/` (planning inputs, not schema artifacts)

## Reviewed Against

- This repository HEAD: `f6b4c3f`
- Sibling repository (`optioni/openspec-schemas`) HEAD: Not applicable — this change edits no
  vendored file under `openspec/schemas/` or `.claude/agents/`.
- Working tree: clean apart from this change's own artifact directory. Every negative control
  run during review restored its subject; `git status --porcelain openspec/specs/` is empty.

The finding pass was delegated to **two independent reviewers**, neither a fork of the
planning session, each given the change directory and one slice: (A) proposal/specs/design
coherence and factual verification of every empirical claim; (B) tasks, and an audit of whether
each proposed check can fail at all. Both wrote findings incrementally to scratchpad files
rather than returning them only in a final message, per `HANDOFF.md` → Budget shape. Reviewer B
ran every shell block in `tasks.md` verbatim and attempted to break each one in `cp -R` copies.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md, proposal.md, tasks.md | The plan's whole durability argument rested on `tests/ci_workflow.rs`'s `every_gate_the_makefile_composes_runs_in_ci` "forcing" a CI step for any Makefile-composed gate. **It does not.** Read at `tests/ci_workflow.rs:227-258`, it never parses `check:`'s prerequisites — it compares against a hardcoded four-item literal, so adding `gates` to `check` leaves it green. This false claim was also the stated reason for taking no outer-loop acceptance test | Task 4.1 now **rewrites** that test to parse `check:`'s own prerequisite list out of the `Makefile`, which makes the guarantee real for this gate and the next one; design.md → Test Strategy states plainly that the landed test does not force it and that task 4.1 is what does | tasks.md task 4.1; design.md → Test Strategy |
| CRITICAL | tasks.md task 0.4 | `OPENSPEC-UNTOUCHED-SP` leg 2 grepped only for added/removed `### Requirement:` / `#### Scenario:` / `- **WHEN**` lines. The normative SHALL sentences are **plain prose**, so flipping `SHALL NOT` to `SHALL` inside a requirement produced no matching line and the gate returned **exit 0** — proven in a copy. The one gate carved out to permit spec edits could not see a requirement rewritten under cover of a Purpose edit | Leg 2 replaced with a strip-and-diff: strip the `## Purpose` section from the `BASE` blob and from the worktree file and require byte equality. Re-run at planning time — clean tree exit 0; the `SHALL NOT`→`SHALL` plant exit **1** printing the prose diff; a legitimate multi-line Purpose exit 0; one of the 26 deleted exit 1 | tasks.md task 0.4 |
| CRITICAL | specs/quality-gates, tasks.md group 2 | The contracted `gates` recipe **could not pass**: the extracted `DEPS` block opens `: "${WORK:?…}"` and `WORK` is consumed by leg **4**, not only leg 5, so `env -u WORK sh DEPS.sh` exits 1 and no task defaulted it | New task 2.3 defaults `WORK` inside the script with a cleanup `trap`; task 2.6's verification now runs the script through `env -u WORK` so the defect cannot come back unnoticed | tasks.md tasks 2.3, 2.6 |
| CRITICAL | tasks.md task 4.1 | `tests/ci_workflow.rs`'s `parser_preconditions_hold` asserts `sections.len() == 3`; task 4.3 adds a fourth job. The widen list omitted it, so group 4 would have gone red on a test nothing in the plan mentioned | Task 4.1 now names `parser_preconditions_hold` explicitly with the reason | tasks.md task 4.1 |
| CRITICAL | design.md | Every task pointer in design.md was stale — four cited a group 9 that does not exist, the planted-defect matrix was cited as group 6 when it is group 5, and the coverage and Herdr rows pointed at the fmt and clippy tasks. `tasks.md` was renumbered after `design.md` was written | All twelve pointers re-derived against the actual group numbering and re-checked by grep | design.md throughout |
| CRITICAL | design.md → Risks | The mitigation promised "task 9.5 diffs the archived result against the pre-change spec"; no such task existed under any number. Both deltas use `REMOVED`+`ADDED`, so the round trip is exactly what needs proving | Task 8.13 added: dry-run the archive round trip for both deltas, confirm each `REMOVED` header matches the live spec byte-for-byte and every scenario survives into its `ADDED` replacement | tasks.md task 8.13; design.md → Risks |
| WARNING | tasks.md → Gate floors | `NOSLEEP` was to be invoked at `SLEEP_MIN=5` against a **realized 6** — the precise "gate passing at a threshold nobody chose" failure the standing rule exists to prevent, and a fourth instance of it, inherited from `plugin-actions` | Floor corrected to `SLEEP_MIN=6 MIN=29` in both tables, and task 6.4 now records the inherited miss in `HANDOFF.md` so the next change does not re-inherit it | tasks.md → Gate floors, task 6.4; design.md → Boundaries |
| WARNING | tasks.md task 3.3 | An **ambient** `GRAPH_WRITE` in the environment makes `build-graph.sh` rewrite the snapshot and exit 0 with **zero assertions run**, and the planned `git diff --exit-code` stays clean because the rewrite is byte-identical. The gate would pass while proving nothing | Task 3.3 now takes the write mode from an explicit argument and has `make gates` clear the variable; task 3.5 asserts the run's output **contains** the `OK` line rather than only that it exits 0 | tasks.md tasks 3.3, 3.5 |
| WARNING | tasks.md task 5.1 | The plant `once_cell = "1"` appended to a copied `Cargo.toml` lands in `[dev-dependencies]` — the file's last table — which leg 2a correctly ignores, so the plant passed with exit 0 and would have been recorded as a gate that cannot fail | Task 5.1 now says "inside the `[dependencies]` table" and names the trap | tasks.md task 5.1 |
| WARNING | specs/quality-gates, tasks.md group 5 | Two scenarios — "Format gate fails and stops the run" and "Lint gate fails on a clippy warning" — had design matrix rows but **no task**. They are carried forward from `ci-pipeline` but `check`'s composition moved underneath them | Task 5.0 added, re-running both plants against the recomposed `check` | tasks.md task 5.0 |
| WARNING | tasks.md group 6 | Tasks 6.4 and 6.5 were written as corrections to text that **does not exist**: `AGENTSEAM` appears in neither `HANDOFF.md` nor `openspec/IMPLEMENTATION-ORDER.md` at `f6b4c3f`. The group also carried no lifecycle verbs and its only `CHECK` was last, which is how those two premises went unchecked | Task 6.0 `CHECK` added first, grepping each document for the text the later tasks claim to correct; 6.4 restated as an addition rather than a correction; 6.5 relabelled `VERIFY`; every task prefixed with its verb | tasks.md group 6 |
| WARNING | tasks.md task 4.1 | The stated measurement command `grep -c '#[test]'` returns **0**, not 11 — unescaped, the brackets are a character class. The "Measured at planning time" table had it right and the task body did not | Escaped in the task, with the trap named | tasks.md task 4.1 |
| WARNING | design.md → Contracts | `rules.design` requires the change to state whether the `Change` type is altered; it did not | Stated: not altered, no field added, removed or retyped, so the `from_files`/`from_cli` agreement question does not arise | design.md → Contracts |
| WARNING | design.md → Contracts | `rules.specs`' six mandated boundary states were neither covered nor declared inapplicable | Declared inapplicable with the reason — all six are states of the *running plugin*, and this change adds no runtime path — and the three analogous boundary states that **do** apply are named and mapped to their scenarios | design.md → Contracts |
| WARNING | design.md → Test Boundaries | `python3` became a `make check` dependency with no row, and `git`'s row was narrower than tasks 0.4 and 8.11 actually use it | Both rows added and widened | design.md → Test Boundaries |
| WARNING | tasks.md task 4.3 | The new `gates-full` job compiles but was specified with no checkout, toolchain, or cache preamble, which would violate `ci-workflow`'s live "Each job installs the toolchain it needs" and "Caching speeds a run up" requirements | Task 4.3 now specifies the same preamble the `coverage` job carries, and names the two live requirements it satisfies; task 4.4 re-reads the two composite-target requirements against the new steps | tasks.md tasks 4.3, 4.4 |
| SUGGESTION | tasks.md task 8.12 | The proc-macro portability question was deferred to a CI run when it is answerable locally: intersecting `cargo metadata`'s proc-macro target kinds with each triple's own `cargo tree` gives the **same eight names** on the macOS and the Linux triples | Task 8.12 now reproduces that intersection locally and uses the CI run as confirmation rather than as the only source | tasks.md task 8.12; design.md → Risks, Open Questions |
| SUGGESTION | design.md → Boundaries | `MDSEAM` appeared in `tasks.md`'s floors table and not in `design.md`'s | Row added, so the two tables agree gate for gate | design.md → Boundaries |

## Findings examined and **not** taken

- **"`plugin-build`'s live spec names three gate clauses neither script lands"** — the `notify`
  manifest-text `default-features = false` check, the `kqueue`/`kqueue-sys` named absences, and
  the `notify-debouncer-*` absence. Verified: all three are real gaps between that requirement
  and the two scripts. **Not taken here.** They are pre-existing and orthogonal: the scripts
  have never carried them, no leg regressed, and adding three assertions would widen this change
  from "repair the two red gates and make them durable" into "audit the dependency requirement
  end to end". Recorded below as a deferred note with its resolution point named.
- **"The ci-workflow delta leaves the composite-target requirement stale"** — examined. That
  requirement says the composite `check` is a local convenience and CI invokes targets
  individually; adding a `Gates` step and a `gates-full` job is exactly that pattern, so the
  requirement is satisfied as written rather than stale. Task 4.4 re-reads it against the real
  steps, which is the check that would catch it if this reading is wrong.
- **"`AGENTSEAM` should be repaired"** (the change's own brief). Verified green at HEAD at its
  landed `MIN=23`: `find src tests -name '*.rs'` = 28, less the five `ALLOWED` = 23. The `22`
  in the brief is `plugin-actions`' task 0.3 baseline, measured before that change's own files
  existed. **Neither a missing-test case nor an over-set floor.** The floor is correct; only its
  recorded arithmetic is wrong, and that correction is written into `HANDOFF.md` at task 6.4.

## No Remaining Implementation-Blocking Gaps

None. Every CRITICAL was repaired in the artifact that owns it and the repair re-verified by
running the affected check; every WARNING was either repaired or, where declined, examined
above with the reason. `openspec validate spec-purposes --strict` reports the change valid.

Both reviewers independently confirmed every figure in `tasks.md` → "Measured at planning time"
and "Gate floors" by re-running the stated command, including the coverage baseline (97.05%
over 25,674 lines), the 940 library tests, the 26/40 validate split, and each floor's
arithmetic re-checked in a `cp -R` copy with a probe file added to `tests/`. One figure was
wrong — `SLEEP_MIN` — and it is fixed above.

## Deferred Non-Blocking Notes

- **`plugin-build`'s three unlanded gate clauses** (the `notify` manifest-text
  `default-features = false` assertion, the `kqueue`/`kqueue-sys` named absences, and the
  `notify-debouncer-*` absence). Resolution point: `degraded-states` is the last Phase 6 change
  and touches neither script, so this belongs to the follow-up that brings the remaining
  twenty-eight extracted gates into `scripts/gates/` — recorded in `HANDOFF.md` at task 6.3
  alongside that follow-up, not left floating.
- **The other twenty-eight extracted gates, and their hand-maintained `MIN` floors.** All are
  green, so none blocks this change; but they rot on exactly the terms `DEPS` and `GRAPH-SNAP`
  did. Named as a Non-Goal in `design.md` and written into `HANDOFF.md` at task 6.3 with the
  reason a hand-maintained floor is worth deriving rather than transcribing.
