# Planning Review — tab-open-focus

## Reviewed Artifacts

- `proposal.md`
- `specs/pane-open/spec.md` (the delta: 2 ADDED requirements, 1 MODIFIED)
- `design.md`
- `tasks.md`

## Reviewed Against

- This repository HEAD: `f4ebcb55e4143b19a8abc852434f9a1d694a12f1`
- Sibling repository HEAD: Not applicable — this change touches no sibling. (`openspec/schemas/tdd/`
  and `.claude/agents/` are graft-vendored from `openspec-schemas` and are not edited here.)
- Environment the measurements were taken on: `herdr 0.9.0`, `openspec 1.12.0` (nvm node v24.18.0)
- Working tree: clean apart from this change's own untracked directory. `git diff --stat src/ tests/`
  was empty before and after the review, including after reviewer D's plant-and-restore of
  `src/open.rs` for the negative control.

The finding pass was delegated to four `planning-reviewer` subagents, none of which wrote the
plan, sliced (A) capability coverage and scenario quality, (B) design completeness, test
boundaries, and whether each proposed check can fail, (C) task alignment and lifecycle, and
(D) factual verification of every empirical claim. They reported findings and edited nothing.
Reviewers A, B, and D between them verified **44 empirical claims** against the repository by
running the command or reading the source: 6 (A), 12 (B), and 26 (D, being its 10 assigned plus
16 it chose). Two were **false** — `tasks.md`'s "three existing tests" (five break) and "eight
Requirement-1 scenarios" (nine) — both repaired and listed as gaps below. Reviewer D also
retracted one finding after checking it, recording the retraction so no later reviewer re-raises
it: the "extended with the post-open focus that the empty pre-open id list produces" matrix row
is achievable, because registering `["pane","list"]` as `Err` then `Ok(labelled_listing())` gives
the first call its failure while leaving `before` empty.

Several artifacts were repaired while the review was in flight, so some first-pass findings from
B, C, and D arrived already fixed; those are not listed as gaps. Reviewer D's report explicitly
states it was written against the post-repair contents.

Two refinements were taken from D beyond the gap table: the `LAUNCHSEAM` note in task 3.5 (its
`prod()` slice truncates at the first `#[cfg(test)]`, so the recorded plant at EOF could not fire
it — re-planted above `src/open.rs:423` it does, meaning both gate legs guard the production
slice), and the `scrubbed()`/`MIN_SPAWN_SITES` constraint `every_run_pipes_and_scrubs_herdr`
(`tests/cli.rs:460`) imposes on the new acceptance test, now named in task 0.2.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/pane-open/spec.md`, `design.md` | Nothing could falsify the difference logic. Every scenario either started from an empty pre-open listing or ended with `before` and `after` equal, so an implementation hardcoding `opened_pane(&[], &after)` passed all 24 matrix rows — the same wiring-assumption class of defect this change exists to fix. | Added a `run`-level scenario: pre-open listing carries `w8:pG`, its focus fails at exit 2, the open falls through, the post-open listing carries `w8:pG` then `w8:pH`, and the fifth call must focus `w8:pH`. | Spec Req 1, scenario "The post-open focus names the newly opened pane, not the one already listed"; design matrix row |
| WARNING | `specs/pane-open/spec.md`, `design.md` | `existing_pane` accepts `"pane_id":""` (`src/open.rs:116` calls `as_str()` with no emptiness filter) while the live spec already promises "no focus call is made with an empty or non-string pane id" — a promise with no proving fixture at HEAD. The post-open focus would have been a **second** site issuing `plugin pane focus ""`. | The single extractor requires a **non-empty** `pane_id`, which closes the existing gap and the new site at once. Added an empty-id input to the extractor scenario and an `existing_pane` clause. | Spec Req 2 and its "No post-open match chooses nothing" scenario; design Decision 7 |
| WARNING | `design.md` → Test Strategy | Two matrix rows stayed green with the whole post-open step deleted. `FakeCli` panics only on an *unregistered* call, never on an unused registration, so asserting `outcome == Ok` or `warnings.is_empty()` proved nothing. | Both rows now assert on `fake.calls()` — the recorded argument vectors — and the reason text. Added task 3.2 requiring the same of every group-3 test. | Design matrix rows for "A post-open focus failure warns…" and "A successful open is silent"; `tasks.md` 3.2 |
| WARNING | `tasks.md` 3.4 | Named three existing tests the new call sequence invalidates; **five** break. `no_match_opens_instead` (`src/open.rs:522`) asserts `calls.len() == 2`, which becomes 3; `a_dead_focused_pane_opens_against_a_live_one` (`src/open.rs:1102`) asserts `warnings: Vec::new()` but its `pane_listing` helper labels entries `"zsh"` (`src/open.rs:592`), so the post-open listing matches nothing and warns. Found independently by C and D. | Listed all five with the reason each breaks. The second is repaired by registering a second labelled `pane list` answer so silence is preserved, never by weakening its assertion (D's refinement). | `tasks.md` 3.4 |
| WARNING | `tasks.md` 2.1 | Task 2.1 told the implementer to leave the `existing_pane` tests unedited, but `no_match_opens_instead` is in that set **and** must change in group 3 — the two tasks contradicted each other. The count "nine" was also a grep-line count including two comments; five test functions call it. | Scoped "leave unedited" to the *matcher* assertions only, named the group that changes its call counts, and gave the count with the command that produces it. | `tasks.md` 2.1 |
| WARNING | `design.md` → Decision 4, `tasks.md` 0.1/4.3 | The counter-reset affordance had no consumer: the only two-invocation test keeps the plain stub, and the one new test makes a single invocation. As an unused helper it would fail `cargo clippy -D warnings` on `dead_code`. Found independently by B and C. | Replaced with a second helper, `stub_herdr_sequenced`, leaving `stub_herdr` untouched and building no reset. | `design.md` Decision 4; `tasks.md` 0.1, 4.3 |
| WARNING | `tasks.md` 5.3 | The row recipe omitted `verdict`, which `openspec/specs/degraded-coverage/spec.md` requires as one of exactly six keys and which `tests/degraded_coverage.rs:126` rejects the parse without — so task 5.4 would have failed on rows task 5.3 produced. | Named all six keys, `verdict = "implemented"`, and required `condition` to match `SPEC.md`'s first cell verbatim. | `tasks.md` 5.3 |
| WARNING | `proposal.md` → Impact | Impact listed two files; the plan edits six, including `tests/degraded-coverage.toml`, which is contract tier (`tests/degraded_coverage.rs` runs inside `make check`). | Listed all six with the contract-tier one called out. | `proposal.md` Impact |
| SUGGESTION | `tasks.md` 5.3a | The four existing `open` rows' `covers` ranges are **already wrong at HEAD** — `src/open.rs:268-280` lands on `placement_for` and the `Report` doc comment, not the listing block — and `validate_covers` (`tests/degraded_coverage.rs:352-392`) structurally cannot detect range drift. This change shifts them further. | Added a task to re-anchor all four while the file is open, noting why no gate will catch it. | `tasks.md` 5.3a |
| SUGGESTION | `specs/pane-open/spec.md` | The "no dashboard pane identified" warning had no upstream `CliError` and no specified wording, so its test could only assert a count. Found by A and B. | The requirement now mandates it name the workspace id and the condition; the scenario asserts that. | Spec Req 1 table row 3 and the empty-listing scenario |
| SUGGESTION | `design.md` → Test Boundaries | Two collaborators were unstated: group 5's `cargo test --test degraded_coverage` reads the real repository tree, and task 3.5 shells out to the real `make gates`. | Added both rows, marked real and read-only. | `design.md` Test Boundaries |
| SUGGESTION | `tasks.md`, `design.md` | Miscounts and a wrong symbol: 1.1 said eight Requirement-2 scenarios (seven), 3.1 said eight Requirement-1 scenarios (nine after the CRITICAL repair), design prose said "twenty-two of the twenty-three", and 3.4 named a test `open_is_silent_on_success` that does not exist — the symbol is `successful_open_is_silent` (`src/open.rs:1153`). | All corrected against per-requirement counts of 9 / 7 / 8 = 24, verified by command. | `tasks.md` 1.1, 3.1, 3.4, count block; `design.md` Test Strategy |
| SUGGESTION | `tasks.md` group 3, group 4 | Group 3 (behavior) had no REFACTOR step and no statement that none was needed, and used the operational `CHANGE:` marker for test-repair work that belongs to GREEN. Group 4 opened with `CHANGE:` before `VERIFY:`. | Relabelled to the behavior lifecycle and added the REFACTOR step; group 3 is now contiguous 3.1–3.7. | `tasks.md` groups 3 and 4 |
| SUGGESTION | `tasks.md` 3.4 | The `listing_failure_warns_and_still_opens` repair only holds if a second `Ok(labelled)` is registered — with only the `Err` registered, `FakeCli`'s repeat-last makes the post-open listing fail too and no focus follows. | Stated the second registration explicitly. | `tasks.md` 3.4 |
| SUGGESTION | `tasks.md` 4.1 | The comment at `tests/cli.rs:361` ("so four lines total") goes stale with the assertion the task corrects. | Folded the comment fix into the same task. | `tasks.md` 4.1 |
| SUGGESTION | `design.md` → Boundaries | `split_target` was cited as the pattern `opened_pane` follows, but it is production code governed by **no** requirement and it contradicts the live "`open` splits the pane the action was invoked from" requirement by substituting a live pane from the listing. | Added a caveat that it is cited as a shape, never as a spec precedent. See Deferred below. | `design.md` Boundaries |

## No Remaining Implementation-Blocking Gaps

None remain. `openspec validate tab-open-focus --strict` reports the change valid. All 24 spec
scenarios appear in the design verification matrix (25 rows — the headline scenario carries both
a unit and an acceptance row), verified by set comparison in both directions rather than by
counting. Every design decision has an owning task, every group carries exactly one valid kind
marker with its evidence task first, and `tasks.md` (238 lines) remains shorter than the
`design.md` (241 lines) it implements.

No PRD non-goal is crossed: no OpenSpec file is edited by the plugin, no cross-change
orchestration, no change authoring, no Windows. No code path in the plan writes inside
`openspec/` — the plugin's only runtime write stays `agent-names.toml` under
`HERDR_PLUGIN_STATE_DIR`. No process spawn is added outside `cli`, proved by a negative control
run at planning time (planting `std::process::Command::new` in `src/open.rs` turned `NOSPAWN OK`
into `NOSPAWN FAIL`; restoring the file returned it to `OK`).

## Deferred Non-Blocking Notes

- **`split_target` is production code governed by no requirement.**
  `grep -rn 'split_target' openspec/specs/ openspec/changes/` matches only this change's
  `design.md`. `src/open.rs:152` substitutes a live pane from the listing where the live
  requirement "`open` splits the pane the action was invoked from" still mandates the context's
  own `focused_pane_id` and says it "SHALL be omitted when the context carries no focused pane
  id". Out of scope here — this change neither relies on nor worsens it. Resolution point:
  recorded in `design.md` → Boundaries so the next reader does not take the live spec as
  authoritative for the split vector; it warrants a separate change.
- **The four existing `open` rows' `covers` ranges are wrong at HEAD.** Repaired opportunistically
  by task 5.3a because this change edits the same file. The underlying weakness — `validate_covers`
  cannot detect a range that has drifted off the code it names — is not addressed here and would
  need its own change to the `degraded-coverage` capability.
