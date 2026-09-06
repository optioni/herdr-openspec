## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/agent-launch/spec.md`, `specs/agent-list/spec.md`, `specs/agent-poller/spec.md`,
  `specs/artifact-content/spec.md`, `specs/dashboard-loop/spec.md`,
  `specs/degraded-coverage/spec.md`, `specs/live-updates/spec.md`,
  `specs/openspec-binary/spec.md`, `specs/plugin-config/spec.md`,
  `specs/plugin-state/spec.md`, `specs/quality-gates/spec.md`,
  `specs/responsive-layout/spec.md`

## Reviewed Against

- This repository HEAD: `89cb3b2`
- Sibling repository: Not applicable — `~/Code/openspec-schemas` is vendored, not modified.
- Working tree: clean apart from this change's own `openspec/changes/degraded-states/`
  directory.

The finding pass was delegated to **three independent reviewers**, none of them a fork of the
planning session, each given the change directory and one slice: (1) cross-artifact coherence
and delta-spec fidelity; (2) whether each proposed check could fail at all, with every command
in the plan actually run; (3) scope discipline, repository rules, and the honesty of the
audit's verdicts against the source. A fourth slice — factual verification of empirical claims
— was folded into slice 2, which re-ran all twenty-two gates at the plan's floors and
confirmed every measured number.

A separate four-way audit of `SPEC.md`'s degraded-states table against the running plugin was
run **before** planning began; its findings are design.md → Context and are re-checked by task
group 1 rather than trusted.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md, tasks.md | The change's own outer tests were unsound: `start_collaborators` reaches the binary probe through `worker_cli_from_env` → `openspec_bin_from_env`, which reads the real `PATH` and spawns the real `npm`. Landed outer tests dodge this by naming a usable `openspec_bin`; the file-mode scenarios exist to drive the case where nothing resolves, so they would have consulted the developer's machine | `Startup` carries the environment lookup and the `npm prefix -g` hook; `run` is the one caller passing the real bindings | design.md → Decision 14; `specs/openspec-binary/spec.md` (new requirement, 2 scenarios); `specs/agent-poller/spec.md`; tasks 3.1, 3.2, 3.7 |
| CRITICAL | specs/degraded-coverage/spec.md | A new capability's delta with no `## Purpose` makes `openspec archive` write the `TBD - created by archiving change` placeholder, which `tests/spec_purposes.rs` fails on — inside `make check` | `## Purpose` written into the delta | `specs/degraded-coverage/spec.md` |
| CRITICAL | design.md, tasks.md, specs | Three different verdict vocabularies (five in design, four in tasks, three in the spec), so 21 of 44 rows had no legal verdict and the checker would have failed on them | One vocabulary of five — `confirmed`, `unproven`, `spec-corrected`, `repaired`, `implemented` — everywhere | `specs/degraded-coverage/spec.md`; tasks 1.3, 1.5, 11.3, 11.5 |
| CRITICAL | specs/degraded-coverage/spec.md | `every_row_has_an_audit_entry` bound a `cargo test` to `openspec/changes/degraded-states/notes/audit.md`; `openspec archive` moves that path, so the test would break permanently on archive | The machine-checked half moves into the coverage map as a `verdict` key in `tests/`, which archiving does not move; `notes/audit.md` stays narrative | `specs/degraded-coverage/spec.md`; design.md matrix |
| CRITICAL | tasks.md | Task 2.4 wrote a new leg into `$C/WIRED.sh`, which lives under `openspec/changes/archive/` — a path nothing in this repository may write to, and the PRD non-goal this change is meant to prove | `WIRED` is extracted to `scripts/gates/wired.sh` in group 2, and the leg is added there | tasks 2.4–2.7 |
| CRITICAL | tasks.md | `NODEFAULT-UI` takes one `SCAN_MIN` across four type sets; the `Refresh` set measures below the default while the `Dashboard` set measures 126, so one default either passes vacuously or fails legitimately | Per-subject floors, carried on the multi-subject gate's own recipe line, with Decision 6 amended to permit exactly that | design.md → Decision 6; `specs/quality-gates/spec.md`; `specs/dashboard-loop/spec.md`; `specs/live-updates/spec.md`; tasks 0.5, 12.7 |
| CRITICAL | tasks.md | `13.4`/`12.4` re-derived `BASE` as the current `HEAD`, which makes `OPENSPEC-UNTOUCHED` diff `HEAD` against a clean tree — the vacuous form its own header warns about | `BASE` is captured **once** at 0.1 and re-verified to resolve, never re-derived; the `BASE` leg is run with `CHANGE=degraded-states` | tasks 0.1, 14.5 |
| CRITICAL | tasks.md | The signing sweep is a loop over `$BASE..HEAD`; with `BASE == HEAD` it runs zero times and reports success | The commit count is asserted above zero before the loop | tasks 17.9 |
| CRITICAL | specs/degraded-coverage/spec.md | Condition 5 required a `view` proof's **file** to be under `src/ui/` and to name `TestBackend` — unsatisfiable for a proof belonging in `detail.rs` or `markdown.rs`, which `NOTABSEAM`/`MDSEAM` forbid from naming a `ratatui` type, and too coarse in the three files that do | Condition 5 checks the named function's **own body** | `specs/degraded-coverage/spec.md`; design.md matrix |
| CRITICAL | design.md | The verification command for `live-updates :: Refresh has no Default` was a bare `nodefault-ui.sh`, whose default `TYPES` never examines `Refresh` | The matrix names the `Refresh` type-set invocation | design.md matrix |
| WARNING | design.md | Row 31 was classified `confirmed` while this change's own `openspec-binary` delta says the vector nothing reads is dropped — the same state as row 27, classified `unproven` | Row 31 reclassified `unproven`; tally now 16 / 21 / 5 / 1 / 1 | design.md → Context |
| WARNING | proposal.md | Claimed the badge "needs" `BinResolution::problems`; `worker_cli_from_env` already returns an `Option`, so found-ness is available today | Claim removed; surfacing `problems` is stated as an adjacent fix for rows 27 and 31 | proposal.md; design.md → Context |
| WARNING | proposal.md, design.md | Rendering `Config::problems` and `BinResolution::problems` is work beyond the roadmap row and was not flagged as such | Flagged explicitly, with the reason (two rows become provable; no later change can do it) | proposal.md |
| WARNING | design.md | Decision 7 overrode `HANDOFF.md`'s recorded "worth doing as its own change, not smuggled into a hygiene pass" without acknowledging the contradiction | The override is stated, with what changed since (`WIRED` rotted; the roadmap ends here) and that the user's brief asked for the judgment | design.md → Decision 7 |
| WARNING | design.md, tasks.md | `OPENSPEC-UNTOUCHED` was excluded outright, though its two `git ls-files` legs need no `BASE` — and it is the only mechanical guard on the "nothing writes inside `openspec/`" non-goal | Split: the tree-only legs are extracted; the `BASE` leg stays per-change | design.md → Decision 9; `specs/quality-gates/spec.md`; tasks 12.3, 14.5 |
| WARNING | tasks.md | The three dependency-gate clauses `spec-purposes` parked for a follow-up were closed by nothing, and this change is the follow-up | Closed here, with planted-defect proofs | `specs/quality-gates/spec.md` (new requirement); tasks 12.6 |
| WARNING | tasks.md | Plants were named for 3 of 8 tests in group 8, 2 of 5 in group 7, and the group-9 plant left its own assertion green | One plant per test, enumerated; the absence-asserting tests get plants that **add** what must not be there; group 9's plant makes the argv log non-empty | tasks 7.2, 8.4, 9.5 |
| WARNING | tasks.md | Every "`git status --porcelain` is empty" checkpoint sat before its group's commit, so it could never be empty | Replaced with `git diff --name-only` naming only the group's intended files; the one true clean-tree assertion is 13.4, after the planted-defect sweep | tasks 2.7, 7.3, 8.6, 9.6, 11.4, 12.11, 13.4 |
| WARNING | tasks.md | `12.3`'s positive-control plant removed the file, tripping each gate's `test -f` guard before the control it was meant to exercise | Group 13 splits into three uniform sweeps — control-file removal, floor-plus-one, subject plant — each with its own purpose | tasks 13.1–13.3 |
| WARNING | tasks.md | `11.4`'s floor arithmetic was per task group; the gates count `#[test]` per **file**, so three floors would have been raised by zero | Arithmetic restated per file, with the new tests enumerated by name and file | tasks 12.5 |
| WARNING | tasks.md | "each gate's header names the measured plant" is false for about twenty of the scripts | Where no plant is recorded, one is derived, run, and **written into the script's header** | tasks 13.3 |
| WARNING | tasks.md | `badge_drops_whole_below_eighteen_columns` at 17/18/60 would fail `WIDTHS`, which requires both mandated widths | 120 added to that test | tasks 4.1 |
| WARNING | tasks.md | Three whole-buffer-equality tests had no discriminating control; the rule named only the three that already had one | Controls required for all six, named per task | tasks 3.6, 4.2, 8.5 |
| WARNING | tasks.md | `13.4` ran `OPENSPEC-UNTOUCHED` with no `CHANGE`, whose default names an archived change that no longer exists | `CHANGE=degraded-states` set | tasks 14.5 |
| WARNING | tasks.md | Gate floor variables were labelled "(internal)"; three are `LIST_MIN` / `MD_MIN` / `TASK_MIN` with defaults 17 / 23 / 14 against measured 29 / 24 / 16 | Named with their defaults, with an instruction to confirm each name before editing | tasks 0.4 |
| WARNING | specs/quality-gates/spec.md | "no other file restates any of their commands" is false the moment 25 verbatim copies remain in the archive | Narrowed to **live** files; archived planning notes are a record, not a second definition | `specs/quality-gates/spec.md` |
| WARNING | specs/dashboard-loop/spec.md | Carried-over scenario still said "all **twelve** fields" and "the twelfth field" against prose saying thirteen | Both corrected | `specs/dashboard-loop/spec.md` |
| WARNING | design.md | "15 task groups"; the audit cited as "task group 2"; the gate extraction as "groups 10–12" | 18 groups; group 1; groups 12–13 | design.md → Context, Goals, Risks, matrix |
| SUGGESTION | tasks.md | Deleting the vestigial `#[allow]` sat inside an operational group about markdown proofs | Its own `kind: refactor` group, CHARACTERIZE → REFACTOR → VERIFY | tasks group 10 |
| SUGGESTION | design.md | Test Boundaries said Git is "not touched", but the plan drives real git in four operational checks | Restated: not touched **by any test**; the operational checks are commands, not collaborators | design.md → Test Boundaries |
| SUGGESTION | design.md | `notify` was scoped real only to "acceptance tests that assert a second CLI cycle", while the matrix drives it real in two more | Scope widened to every scenario naming a watcher outcome | design.md → Test Boundaries |
| SUGGESTION | specs/degraded-coverage/spec.md | The `tier` enum could not express the `outer` tier the design calls load-bearing | `outer` added, and the deviation from the roadmap's "a view test" stated | `specs/degraded-coverage/spec.md` |
| SUGGESTION | tasks.md | `11.8` recorded a timing with no threshold, so it could not fail | 15 seconds, against ~3.5 s measured | tasks 12.10 |
| SUGGESTION | tasks.md | `2.1`'s RED claim did not distinguish `startup_dir` from the existing `startup_cwd` | Distinguished, with the landed function's location | tasks 2.1 |
| SUGGESTION | SPEC.md | Row 30's "pending a change that reads `herdr worktree list`" is a forward reference to a change that will not exist | Added as the eighth `SPEC.md` correction | design.md → Decision 12; tasks 14.2 |
| SUGGESTION | tasks.md | Nothing updated `openspec/IMPLEMENTATION-ORDER.md`, whose `degraded-states` row says "Nothing here should be new behaviour" | Documentation task added | tasks 16.4 |
| SUGGESTION | design.md | Decision 3's trade-off below 100 columns (list and detail are separate routes) was unstated | Stated as accepted | design.md → Decision 12; tasks 14.3 |

**Confirmed clean, and worth recording because a reviewer re-checked them rather than assuming:**
the verification matrix is exactly 1:1 with the 73 spec scenarios (no missing, no orphan, no
duplicate — re-verified after every repair); the 44-row verdict partition covers 1..44 with no
duplicate or omission; every "measured at HEAD" number in task 0.4 matches the gate's own `OK`
line when re-run; `WIRED` is red at HEAD on leg 2 with exactly the quoted line; count floors do
fire at plus one; `git cat-file commit 89cb3b2 | grep '^gpgsig'` reports signed and `%G?`
reports `N`, as `HANDOFF.md` warns; no scenario was dropped or renamed in any of the six
MODIFIED delta blocks; and no PRD non-goal or `AGENTS.md` architecture rule is crossed.

The row 23 defect was **independently verified in the source** by a reviewer who did not write
the plan (`src/launch.rs:300-314` — `record_problem` is discarded on the both-fail path), as
were the row 18 and row 28 corrections. That matters more than the count: this change exists to
prove claims, and its own central claim was checked the same way.

## No Remaining Implementation-Blocking Gaps

None. Every CRITICAL is repaired in the artifact that owns it, and every WARNING is either
repaired or, where accepted, stated as accepted with its reason in design.md.

Two judgments are recorded here rather than resolved, because they are the user's to overturn
and neither blocks implementation:

1. **The gate extraction is work beyond the roadmap row**, and `HANDOFF.md` recorded the
   opposite intent ("worth doing as its own change"). The user's brief for this change asked
   for that judgment to be made here, and design.md → Decision 7 makes it in the open. If the
   user would rather it were a separate Phase 6 row, groups 12 and 13 lift out cleanly — they
   touch no `src/` file — and the roadmap permits adding a row.
2. **Rendering `Config::problems` and `BinResolution::problems`** is likewise beyond the
   roadmap row, and unlike the per-change indicator it is not pre-assigned to this change by
   `SPEC.md`. It is kept because two table rows are otherwise true only on a vector nobody
   reads, and no later change can make them observable. Flagged in proposal.md rather than
   folded in silently.

## Deferred Non-Blocking Notes

None. This is the final change of the roadmap, so nothing downstream can pick anything up, and
the plan is written on that basis: the two items `HANDOFF.md` left open (`WIRED` red; the
unextracted gates), the three parked dependency clauses, the vestigial `#[allow]`, the stale
`README`/`IMPLEMENTATION-ORDER` claims, and `SPEC.md`'s two forward references to changes that
will never exist are all closed inside this change rather than recorded as follow-ups.
