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
the numbers in `tasks.md`. That slice found six of the ten gaps below. The S10 gap arrived
from a peer session rather than from this review, and was **reproduced here before being
accepted** — the symlink, the shebang, the 127, and the exit-0 with the directory prepended
were each re-run on this machine (Decision 1b carries the commands and their output). The remaining slices
— capability coverage, scenario quality, design completeness, task alignment — are the ones a
same-session review is weakest at, and the Change Review group (task 10.1) dispatches an
independent reviewer over the finished implementation to cover them.

## Reviewed Against

- This repository HEAD: `f947ab262db392d258298e7bc71ec92848f7baa4` at the time of the review.
  HEAD has since advanced to `8113eea` through three commits from a parallel session
  (`994a6d1`, `6816a97`, `8113eea`), all of which touch `openspec/changes/` only:
  `git diff --stat f947ab2..HEAD -- src/ tests/ Makefile scripts/` is empty, so every
  code-level number, grep result, and gate floor recorded here still holds.
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
| CRITICAL | `specs/subprocess-seam/spec.md` | **S10, reported by a peer session and reproduced here.** The seam's blanket prohibition — which this change's own draft re-stated verbatim at line 8 — forbids altering the child's environment by name. But `openspec` is not a self-contained binary: the resolved path is a symlink to `openspec.js` whose first line is `#!/usr/bin/env node`, and `node` lives in the same directory. Reproduced independently: `env -i PATH=/usr/bin:/bin <nvmbin>/openspec list --json` → exit `127`, empty stdout, `env: node: No such file or directory`; with that directory prepended → exit `0`, `1.12.0`. Landing the fix later would have reopened a requirement this change had just re-affirmed. | Relaxed the environment clause in the **same paragraph** as the `current_dir` clause under one shared rationale — both were written under an assumption (directories coincide; executables are self-contained) that measurement falsified. `RealOpenspecCli` gains a constructor-supplied environment overlay: sets exactly what it is given, clears nothing, removes nothing, derives nothing. | `specs/subprocess-seam/spec.md` → "The real implementations spawn and return stdout…"; `design.md` → Decision 2 |
| CRITICAL | `specs/refresh-worker/spec.md` | S10 has the same *symptom* as S7 and a different *mechanism*: there the child runs and reports a root the plugin rejects; here it never runs and there is no payload to reject. Folding them together would have made the S7 fix look broken on any machine whose probe reached step 3 or 4 — which is, structurally, exactly the machines where `PATH` lacks the nvm bin directory. | Added a separate requirement carrying the overlay rule (one `PATH` entry: the resolved binary's own parent, prepended to the inherited value, read through the injected lookup, applied for every probe step), with four scenarios including the 127 degrade. Kept `cli-changes`' stderr-on-`Failed` work out of scope, as instructed. | `specs/refresh-worker/spec.md` → "The resolved `openspec` binary is spawned…" |
| WARNING | `tasks.md` | Task 1's measurement recorded only the two directories. A 127 would have been read as "the S7 fix did not work", since the pane looks identical in both defects. | Extended group 1 to spawn the resolved binary for real from inside the pane process and record its exit code, stdout, and stderr, plus a classification step (1.6) that names which defect the result is. | `tasks.md` → 1.3, 1.4, 1.6, 1.7 |
| SUGGESTION | `tasks.md` | Task 2.5's proposed check asserted `grep -c 'PATH' src/cli.rs` → `0` "outside test bodies". Run at HEAD it returns `8`: six doc comments and two test literals. The check as written would have been red on an untouched tree. | Rewrote it to record the eight HEAD hits with their line numbers and to assert the overlay adds no ninth outside a test. | `tasks.md` → 2.5 |
| SUGGESTION | `specs/watch-invalidation/spec.md` | The landed text justified `Selection::Only(∅)` as meaningful because "the worker still runs `openspec list --json`, which is how a change appearing or vanishing is noticed". That justification is false on its own terms: a change appearing or vanishing classifies `Repository`, never `Outside`. | Reversed the reading and recorded why the old justification does not hold, keeping `invalidate` itself unchanged so the short-circuit lives at the caller. | `specs/watch-invalidation/spec.md` → "A touched path is classified…" |

Mechanical cross-checks run at HEAD, all passing:

- 93 scenarios across the eight delta specs; every one appears in `design.md`'s verification
  matrix, with no duplicate scenario names (checked by script over the files).
- 45 matrix rows marked "new test"; every one is named in a `tasks.md` RED line.
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
- Carrying the child's stderr on the `CliError::Failed` problem row — which is what makes a
  127 self-diagnosing to the reader — belongs to the `cli-parity` sibling change and is
  deliberately not duplicated here. This change's own 127 scenario asserts the row names the
  command and the exit code, which is what the landed rule already promises.
- The launcher is the only worker joined at exit. The refresh and poller workers stay
  detached, and the reason is recorded in Decision 6 rather than left as an omission.
