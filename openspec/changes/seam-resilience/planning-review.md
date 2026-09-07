## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/agent-attribution/spec.md`
- `specs/agent-launch/spec.md`
- `specs/agent-poller/spec.md`
- `specs/live-updates/spec.md`
- `specs/refresh-worker/spec.md`
- `specs/subprocess-seam/spec.md`
- `specs/terminal-lifecycle/spec.md`
- `specs/watch-invalidation/spec.md`

**Delegation:** the schema's default is to dispatch four independent reviewers. This session
was invoked under a harness rule forbidding it from spawning subagents, so the finding pass
was performed in-session and is recorded as such rather than claimed as independent. The
factual-verification slice (reviewer D's job — check every empirical claim by running the
command) was executed literally: every number, grep result, and CLI capability asserted in
these artifacts was produced by a command run at HEAD, and the commands are written beside
the numbers in `tasks.md`. That slice found four of the six gaps below. The remaining slices
— capability coverage, scenario quality, design completeness, task alignment — are the ones a
same-session review is weakest at, and the Change Review group (task 10.1) dispatches an
independent reviewer over the finished implementation to cover them.

## Reviewed Against

- This repository HEAD: `f947ab262db392d258298e7bc71ec92848f7baa4`
- Sibling repository (`~/Code/openspec-schemas`) HEAD: Not applicable — this change edits no
  vendored schema or agent definition.
- Working tree: clean of tracked modifications. Five untracked change directories are present
  — `cli-parity/`, `doc-conformance/`, `gate-integrity/`, `seam-resilience/`, and
  `view-fidelity/` — the four sibling proposals being drafted in parallel plus this one. Only
  this change's own directory was written by this session.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/refresh-worker/spec.md` | The S7 requirement said `refresh::start` "SHALL construct its `OpenspecCli`" and its scenario asserted on what `refresh::start` "builds". Verified against the source: `refresh::start` receives an already-constructed `Option<Arc<dyn OpenspecCli>>`; the construction site is `cli::worker_cli`, called from `ui::start_collaborators`. The requirement named a site that cannot satisfy it. | Rewrote the requirement and its scenario to name `cli::worker_cli(resolution, cwd)` and `ui::start_collaborators`, and to state that `worker_cli_from_env` passes `None` because it is a `WIRED` positive control that cannot know a root. | `specs/refresh-worker/spec.md` → "The CLI-merge tier engages regardless…"; `design.md` → Boundaries, verification matrix row |
| CRITICAL | `design.md` | Decision 6 asserted that `launch::settle` needs no gate change, but the assertion covered `NOBLOCK` only. `NOSLEEP` was not checked, and `settle` sleeps. Read `scripts/gates/nosleep.sh`: leg 2 bans sleeps under `src/ui` alone, and leg 2b caps `src/watch.rs`, `src/refresh.rs`, and `src/agents.rs` at one each — `src/launch.rs` and `src/cli.rs` are named by neither, and leg 1 permits a sleep inside a deadline-bounded poll. The claim survives, but it was unverified when written. | Added the `NOSLEEP` analysis to Decision 6, and added task 2.7 and task 7.7, each requiring the leg-1 shape to be confirmed with a negative control. | `design.md` → Decision 6; `tasks.md` → 2.7, 7.7 |
| WARNING | `tasks.md` | Task 8.1's RED evidence was `grep -c 'startup' src/ui/app.rs` → claimed `0`. Run at HEAD it returns `3`: all three are the word "startup" in doc-comment prose. A check that is green at HEAD for the wrong reason would have been believed. | Replaced with the field-qualified `grep -rn 'refresh\.startup' src/ \| wc -l` → `0` and `grep -c 'in_flight' src/ui/app.rs` → `0` (exit 1), both run, and recorded why the bare form is wrong. | `tasks.md` → 8.1 |
| WARNING | `specs/live-updates/spec.md` | Two carried-over scenarios wrote `Outcome { … problem: None }` and `problem: Some(…)`. `degraded-states` widened that field to `problems: Vec<String>` (verified in `src/launch.rs`), so the landed spec text had already drifted from the type. Reproducing it verbatim in a MODIFIED requirement would have re-blessed the drift. | Corrected both literals to `problems` while reproducing the requirement. | `specs/live-updates/spec.md` → "The loop drives the live tier…" |
| WARNING | `specs/agent-attribution/spec.md` | The first draft of the S8 fix put canonicalization inside `attribute`. The landed requirement states outright that `attribute` performs no filesystem I/O and therefore cannot canonicalize either side — the fix would have contradicted the requirement it was amending. | Moved the canonicalization to the poller's side of the seam, behind an injected `&dyn Fn(&Path) -> Option<PathBuf>` on `config::env_lookup`'s established shape, leaving `attribute`'s signature and purity untouched. | `specs/agent-attribution/spec.md`; `design.md` → Decision 5 |
| WARNING | `specs/subprocess-seam/spec.md` | The S7 fix was initially framed as a choice between "give the seam a working directory" and "pass the root to `openspec`". Verified against the installed CLI (1.12.0): `openspec list --json` accepts only `--specs`, `--changes`, `--sort`, `--json`, and `--store <id>`, and a store is a *registered* repository, not an arbitrary path. There is no second mechanism. | Recorded the verification in the requirement and in Decision 2, and stated plainly that the seam's `current_dir` prohibition is therefore relaxed — narrowly, for `RealOpenspecCli` only — rather than leaving the relaxation implicit. | `specs/subprocess-seam/spec.md`; `design.md` → Decision 2 |
| SUGGESTION | `specs/watch-invalidation/spec.md` | The landed text justified `Selection::Only(∅)` as meaningful because "the worker still runs `openspec list --json`, which is how a change appearing or vanishing is noticed". That justification is false on its own terms: a change appearing or vanishing classifies `Repository`, never `Outside`. | Reversed the reading and recorded why the old justification does not hold, keeping `invalidate` itself unchanged so the short-circuit lives at the caller. | `specs/watch-invalidation/spec.md` → "A touched path is classified…" |

Mechanical cross-checks run at HEAD, all passing:

- 86 scenarios across the eight delta specs; every one appears in `design.md`'s verification
  matrix, with no duplicate scenario names (checked by script over the files).
- 38 matrix rows marked "new test"; every one is named in a `tasks.md` RED line.
- The 20 rows that are neither "new test" nor "carried over unchanged" each have a task —
  four `decide` scenarios through task 7.5's contract gate, two `AgentSnapshot` literals
  through task 5.4's, and the rest named explicitly.
- `openspec validate seam-resilience --strict` → `Change 'seam-resilience' is valid`.

## No Remaining Implementation-Blocking Gaps

None remain that require user input. One gap is deliberately left open **and is task 1**:
the pane process's real working directory has never been measured, and Decision 1 specifies
the measurement, the three possible outcomes, and what each licenses. It blocks group 3 alone
and not the change, which is why it is sequenced first rather than raised here as a question.

## Deferred Non-Blocking Notes

- `SPEC.md`'s degraded-states table and `tests/degraded-coverage.toml` are owned by the
  `degraded-coverage` capability, which a sibling change holds. Three rows become wrong here
  and two are missing. The resolution point is task group 9, which either makes the edits (if
  the sibling has landed) or writes them into this file as a hand-off (if it has not).
- The per-frame artifact read (S9) is accepted rather than fixed. Its resolution point is
  `design.md` → Decision 10 and the two cache scenarios in `live-updates`, which make the
  acceptance falsifiable rather than merely stated.
- The launcher is the only worker joined at exit. The refresh and poller workers stay
  detached, and the reason is recorded in Decision 6 rather than left as an omission.
