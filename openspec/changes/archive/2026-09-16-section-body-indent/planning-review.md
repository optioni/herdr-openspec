## Reviewed Artifacts

- `proposal.md`
- `specs/artifact-folds/spec.md`
- `specs/artifact-content/spec.md` — **created during this review**; see CRITICAL-1
- `design.md`
- `tasks.md`

The finding pass was delegated to four independent reviewers, none of which wrote the planning
package and none a fork of the session that did, sliced (A) capability coverage, delta fidelity
and contradictions, (B) design completeness, test boundaries and check falsifiability, (C) task
alignment, lifecycle and ordering, (D) factual verification. Reviewer D planted a minimal
implementation of the rule in a scratch copy and ran the whole library suite, which is how the
set of existing tests this change breaks became a measurement rather than a prediction.

## Reviewed Against

- This repository at HEAD `c40a60aa66c17bd665345e3c7039f45543da1e1f`.
- Sibling repositories: **not applicable** — this change touches one crate and no external
  contract. `openspec-schemas` supplies the vendored schema but is not a contract this change
  moves.
- Working tree: clean apart from this change's own planning files. `design.md` was modified and
  `tasks.md` created before the review began; both are intentional and are named here so later
  implementation drift stays visible. `src/ui/detail.rs` and `src/ui/view.rs` were verified
  byte-identical to HEAD after reviewer D's planted-implementation experiment.

## Gaps Found and Fixed

### CRITICAL

| # | Source | Problem | Repair | Location |
|---|---|---|---|---|
| 1 | `proposal.md` | `artifact-content` states the reversed rule in its own scenario "A body row is never indented by its section's depth", proved by `src/ui/view.rs`'s test of the same name. Neither the capability nor the file was named anywhere. Archiving as planned would have left two live capability specs in direct contradiction. | Added `artifact-content` to Modified Capabilities and wrote the delta. Because OpenSpec identifies a scenario by its heading and a rename reads as a deletion, the original scenario is **kept and narrowed** to the 58-column interior, where it stays true, and a second scenario covers the 78-column case. Nothing is dropped and nothing is false. | `proposal.md` → Capabilities; `specs/artifact-content/spec.md` (new); `tasks.md` 1.2, 1.6 |
| 2 | `specs/artifact-folds/spec.md`, `proposal.md`, `design.md` D4 | "Every tracked-tasks section is at depth 0, so no row of a tasks tab moves" is false. `base` is `usize::from(paths.len() > 1)` and `depth` is `base + (level - min_level)`, so a multi-path artifact or a task file opening with a level-1 title puts groups at depth 1 — the shape **17 of this repository's 44 task files** already have. At depth 1 the floor is 66 and the 78-column interior clears it. The scenario meant to prove the tab unmoved pinned a depth-0 fixture, so it passed without reaching the case that matters. | The premise being false, no exemption had actually been chosen. Put to the user, who chose **one rule for every tab**. Deleted the claim from all three artifacts, stated the rule and its reasoning, and split the proof in two: a depth-0 tab is unmoved *by its depth*, and a new depth-1 scenario asserts items shift two columns at 78 and not at 58. | delta → tracked-tasks paragraph and two scenarios; `proposal.md` → What Changes; `design.md` → D4 |
| 3 | `proposal.md`, `design.md` | "`src/ui/detail.rs` only — no other module is touched" is false. | Corrected in both, naming the **three** existing tests the change breaks, measured by planting the implementation: `src/ui/view.rs`'s `a_body_row_is_never_indented_by_its_sections_depth`, and `src/ui/detail.rs`'s `a_badged_header_row_is_still_addressed_by_its_own_section_index` and `the_tracked_tasks_tab_concatenates_rather_than_folding`. | `proposal.md` → Impact; `design.md` → Modules touched; `tasks.md` 1.6 |

### WARNING

| # | Source | Problem | Repair |
|---|---|---|---|
| 1 | `design.md`, `tasks.md` | `cargo test --lib ui::detail ui::view` is rejected by cargo — one positional `TESTNAME` only — so the row covering the sixteen carried scenarios had **no runnable command**, selecting 0 tests. | `--` inserted in both; selections counted and recorded (63 / 156 / 219 / 260). |
| 2 | delta | The carried body bullet still said the body is produced at `width` while the new paragraph said `width - indent_cols`. Both normative. | Bullet amended to name `body_width` and both cases. |
| 3 | delta | "the sections that have a body" was undefined, and the difference is observable: an implementation reading `visible_sections` passed every scenario. | Defined in the delta as `detail.sections` with non-empty `text`, never the visible set, with the reason (`Space` must not reflow siblings). |
| 4 | delta | The headline scenario's stated input — one file — cannot produce its stated depths 1/2/3; one path gives `base` 0 and depths 0/1/2, floor 68, four spaces. An implementer would have "corrected" the expectation and silently lost the depth-3 case, desynchronising the `70` the sibling scenario pins. | Restated as a `specs` glob resolving to more than one file. |
| 5 | `tasks.md` | Group 3's only `CHECK` came *after* its change task, inverting the operational lifecycle. | Split into 3.1 CHECK / 3.2 CHANGE / 3.3 VERIFY, with both greps shown to match at HEAD so the verification can fail. |
| 6 | `openspec/specs/artifact-folds/spec.md` | The capability's `## Purpose` states "bodies drawn unindented at the full content width". A `## MODIFIED Requirements` delta cannot reach a Purpose, and `tests/spec_purposes.rs` only checks it is non-empty — so nothing would ever catch it. | Scheduled as an explicit in-place edit in group 3, with the reason it cannot be a delta. |
| 7 | `tasks.md` | "the 63 existing tests and the 16 carried scenarios stay green" is false — one carried scenario's own test compares a depth-1 body row's text exactly. | Rewritten; the test is scheduled in 1.6 and the exception is stated in `design.md`. |
| 8 | `tasks.md` | A third existing test also fails, found only by planting the implementation. | Added to 1.6 and to both Impact sections. |
| 9 | `design.md` | Neither the blank `separator_row` nor `ui::tasks::bar_lines` was mentioned in the design. `separator_row` is already padded to the full width, so prefixing it would make it the one row exceeding the region — and being blank, the defect is invisible. | Both stated in Decision 1 and normatively in the delta. |
| 10 | `design.md`, `proposal.md` | The text-selection consequence was examined nowhere: `span_text` copies each row's rendered text, so an indented body row now copies its leading spaces, and a double click in the indent selects nothing. No existing test uses a fixture deeper than depth 0, so nothing would have caught it. | New Decision 5 accepts the copied indent, on the ground that a **header** row's own indent already copies the same way; stated in the delta; new scenario added to drive it. |
| 11 | `design.md`, delta | The floor was written `width - 2 * max_depth >= 64` unguarded. `width` is `u16` and this capability's sweeps run from 0, so that panics in the debug build `cargo test` uses. | Saturating form in both, plus the type note and the fact that the *other* subtraction is safe only as a consequence of the floor. |
| 12 | `openspec/changes/task-item-bodies/` | Both changes delta the same requirement and each carries the other's untouched original, so whichever archives second reverts the first. The ordering note lived only in `section-body-indent`, which archives and disappears first. | Mirrored the refresh obligation into `task-item-bodies/proposal.md` → Impact, naming the stale sentence it still carries. |
| 13 | `proposal.md`, `design.md` | Both quoted 231 files / 3,757 headers / 2,812 at depth 3, stale against the archive. | Re-measured and replaced in both: 236 / 3,830. |
| 14 | `tasks.md` | "2,872 at depth 3" conflated ATX heading level with `depth`; five single-capability changes have `base` 0, putting their `####` at depth 2. | Corrected to **2,776 of 3,830** at depth 3, with the distribution and the distinction stated. |
| 15 | `proposal.md`, `design.md` | "`DETAILWIDTHS` satisfied by construction" is false for three of the planned tests — one names only 78, one only 58, the sweep names neither. | Reworded in both to say the literals are added deliberately, per the task that does it. |
| 16 | `tasks.md` | The narrow-interior test was listed under RED but is green at HEAD, since every body row already sits at column zero. An implementer told to "confirm it fails" would have distorted the fixture until it did. | Moved to 1.1 as CHARACTERIZE with a byte-identity assertion, which is also immune to `ui::markdown`'s own hanging indent. |
| 17 | `design.md` | Test Boundaries named only external seams, omitting the two in-crate collaborators Decision 1's correctness rests on. | Added `ui::markdown::lines` and `ui::tasks::items` as **Real** at the reduced width. |
| 18 | `design.md` | Test Strategy said the change "moves rows within it". It also *adds* rows, since the body is re-wrapped narrower — and `rows.len()` feeds the scroll clamp and the pointer resolvers. | Rewritten, naming the four call sites that all pass the same width, which is what makes the no-outer-loop conclusion hold. |
| 19 | delta, `design.md` | A `None`-labelled preamble is not always depth 0 — for a glob artifact it is `base`, i.e. 1 — and nothing said what happens to it. | Stated in both: a preamble is a body and is indented by its own depth. |

### SUGGESTION

All eight were applied rather than deferred: the mislabelled `RED` gate task became `CHECK`;
two tasks that restated `design.md`'s reasoning were trimmed to a citation; the coverage task
now names `make coverage`; "body-bearing" is stated as the byte test `!text.is_empty()` rather
than the narrower "could draw a body"; the two fixtures that do not yet exist are named in a
task so the implementer does not reach for one giving the wrong floor; `colwidth-sweep-help`
and `detailwidths-missing` joined the recorded control list; Decision 3 notes that a
single-capability change's `max_depth` is 2, not 3, which is why the floor is not written as
the constant 70; and the all-or-nothing scenario gained the clause pinning fold-state
independence, the one thing Decision 2 decides that nothing previously asserted.

## No Remaining Implementation-Blocking Gaps

None remain. The one decision that genuinely required user input — whether the tracked-tasks
tab is exempt from the indent, which had been settled in the artifacts on a false premise — was
put to the user and answered: **one rule for every tab, no exemption**. Every artifact now
reflects that, and two scenarios fix it from both sides.

`openspec validate section-body-indent --strict` passes, as does `task-item-bodies` after its
edit.

## Deferred Non-Blocking Notes

- **Scenario renaming is not expressible in an OpenSpec delta.** A `MODIFIED` block must carry
  every scenario heading the live spec has, so `artifact-content`'s scenario was narrowed and
  paired rather than renamed. Recorded here because the next change that needs to retire a
  scenario outright will meet the same constraint; the resolution point is that change's own
  design, not this one.
- **`task-item-bodies` must re-diff its `artifact-folds` delta** against the archived
  requirement before its tasks are written. Recorded in that change's own `proposal.md` →
  Impact, so it survives this change's archive.
- **Coverage was not evaluated at planning time** and cannot be: the gate is meaningful only
  once the code exists. It runs in group 4.
