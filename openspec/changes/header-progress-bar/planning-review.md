## Reviewed Artifacts

- `proposal.md`
- `specs/detail-header/spec.md` (3 MODIFIED requirements)
- `specs/tasks-progress-bar/spec.md` (2 MODIFIED, 1 ADDED)
- `design.md`
- `tasks.md`

## Reviewed Against

- This repository HEAD: `0bdcb62` at dispatch; repairs landed across `4ecb073`, `0ca6e51`,
  `a682bca`, `fb8a147`.
- Sibling repositories: **Not applicable.** `~/Code/openspec-schemas` supplies the vendored
  `tdd` schema and the agent definitions, and this change alters neither.
- Working tree: clean. The planning files are intentionally committed — `OPENSPEC-UNTOUCHED`
  fails on untracked files under `openspec/`, so artifacts must land before `make gates` runs.

The finding pass was delegated to four `planning-reviewer` subagents, one slice each, none of
which wrote the planning package: (A) capability coverage and cross-artifact contradictions,
(B) design completeness, test boundaries, and falsifiability, (C) task alignment and lifecycle
discipline, (D) factual verification against the codebase. Every finding below was reproduced
against the source by this session before repair; two were reproduced and found **wrong in the
reviewer's favour**, which is recorded in the notes.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | tasks.md, design.md | Twelve of the nineteen scenarios already had passing tests at HEAD, bound by a ``/// `<capability>` :: "<scenario>"`` doc comment. The plan wrote new tests under new names, making every RED check vacuous — a new name reports `0 matched` whether or not the behavior exists — and leaving existing tests red and unowned at "no regressions". Found by C, corroborated independently by B. | Every one of the twelve is now a **rewrite in place**, keeping its name and doc comment. The verification matrix names the existing test and its file:line for each. | tasks.md groups 2 and 4; design.md → Test Strategy |
| CRITICAL | tasks.md | The `covers` baseline had to be captured before any edit but sat in a group positioned after the implementation groups, so the check it gates was green by construction. | New group 0 captures every pre-implementation baseline; re-anchoring moved to group 3, directly after the only two groups that shift line numbers. | tasks.md groups 0 and 3 |
| WARNING | tasks.md | The RED helper reported RED when a filter matched **nothing**, indistinguishable from a test that matched and failed. | Replaced with `redfail`, which requires a match *and* a failure. Verified on all three branches, including a planted wrong expectation. | tasks.md → The RED protocol |
| WARNING | tasks.md, design.md | Nine of the twelve existing tests pass unchanged against a gauge-bearing header, so an expectation edit alone leaves them green before *and* after. The plan labelled them all RED. | Tasks now separate **RED** (3 tests, literal expectations), **RED-by-addition** (6, fail only once the scenario's new gauge claim is added), and **CHARACTERIZE** (3, cannot honestly fail and say so). | tasks.md groups 2 and 4 |
| WARNING | specs/tasks-progress-bar | `filled == g` iff `is_complete()` is false as shipped: `saturating_mul` saturates and `u64::MAX / u64::MAX == 1`, so a complete change renders one filled cell — at the header's 12 columns and at the bar's own 68 and 48. `percent_of` carries the identical defect, reading `1%`. | Both widened to `u128`, which removes the saturation rather than special-casing around it and makes the live formula literally true. | specs/tasks-progress-bar → MODIFIED; design.md → Decision 11 |
| WARNING | specs/tasks-progress-bar | The delta was ADDED-only while changing two shipped requirements, so `archive` would have dropped the scenarios those requirements still carry. | Two `## MODIFIED Requirements` blocks added, each carrying the full requirement and every existing scenario. | specs/tasks-progress-bar |
| WARNING | specs/detail-header, design.md, tasks.md | The adversarial scenario asserted the documented band boundaries across four `Progress` values, but they derive from a five-column progress cell: `{0, MAX}` renders 24 columns and `{MAX, MAX}` 43, moving every boundary. No correct implementation could satisfy it. | Band clause scoped to `Progress { 4, 9 }`; the wide-cell values keep the width-exactness, no-panic and drop-whole claims. | specs/detail-header → measured-in-columns requirement |
| WARNING | design.md, tasks.md | `src/ui/view.rs`'s two `header_row` expectations are computed **by calling the function under test**, so they discriminate placement and style but never grammar. The plan claimed they name the old strings and scheduled an edit. Found by B, independently by D. | Corrected in Contracts and Risks; the task is now a CHECK that the file needs no such edit. New view tests are required to assert literal glyph counts, never `assert_eq!(buffer, header_row(…))`. | design.md → Contracts, Risks; tasks.md 4.4 |
| WARNING | design.md | The contract tier was absent from Test Strategy, and the Filesystem boundary read "not reached" while `tests/degraded_coverage.rs` reads `SPEC.md`, the TOML and the covered source files from disk. | Fourth tier added; the Filesystem row now says real in the contract tier and not reached in the others. | design.md → Test Strategy, Test Boundaries |
| WARNING | design.md | Risks named `chars().count() < columns()` as the guard against a `.chars()`-based width. It does not discriminate: correct gives `68 < 78`, char-counting `78 < 88`, both pass. | The guards are now named as the exact `columns(&got) == 78 / == 58` assertions and `COLWIDTH`. | design.md → Risks |
| WARNING | tasks.md, design.md, specs | Four planned tests could not satisfy `DETAILWIDTHS`/`TASKWIDTHS`, which require every `#[test]` in the two edited files to contain the literals `78` and `58`, with no exemption list and stripping `///`. `make gates` would have failed at the last task. | Each given a real assertion at both widths rather than an exemption. The byte-identical sweep now also asserts the row **differs** at 78 and 58, which makes it discriminating where it was not. | specs (both), design.md → Test Strategy, tasks.md 2.4a |
| WARNING | tasks.md | Neither live capability's `## Purpose` was scheduled for rewrite. Delta specs carry no Purpose block and `openspec archive` never rewrites one from a delta, so both would ship stale — this repository has already paid for that twice (`9b63ca6`, `a156f9a`). | Tasks 6.1 and 6.2 rewrite both, naming the files and the archive-time reason. | tasks.md group 6 |
| WARNING | proposal.md, specs/tasks-progress-bar | "two call sites" was wrong: `progress_bar` already calls `gauge_of` twice, so it gains a third. The change adds a second **renderer**. | Reworded in both; the counts table states three sites with two inside `progress_bar`. | proposal.md → Capabilities; specs/tasks-progress-bar |
| WARNING | tasks.md | The view tier was stated as 133 tests; it is 132, which `make gates` reports independently. | Corrected. | tasks.md 4.5 |
| WARNING | proposal.md | The chosen shape falsified two of the proposal's own claims: `progress_bar` is not the function gaining a call site, and `responsive-layout` needs no delta since the mandated interiors do not move. | Both corrected, with the reason recorded rather than silently dropped. | proposal.md → Capabilities, Settled |

Two findings came from reproducing a reviewer rather than from a reviewer:

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| WARNING | tasks.md | Slice C reported every integration target green under its gauge plant. Reproducing it showed `degraded_coverage` failing — `covers entry src/ui/detail.rs:245-251 holds no line of code`. The hazard is **nondeterministic**: a one-line shift left it silent at `10 passed`, the real ten-line gauge makes it fail loudly naming an unrelated degraded-states row. | Group 3's account rewritten around both measurements, and moved to sit directly after the groups that shift lines so the confusing failure does not land in group 4's run. | tasks.md group 3 |
| WARNING | specs/tasks-progress-bar | Slice D listed three width-gate violators; a fourth — the saturation-boundary scenario added mid-review — had the same defect. | Repaired the same way, routing through `progress_bar` at 78 and 58. | specs/tasks-progress-bar |

Slice D's closing suggestions, all verified and applied:

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| SUGGESTION | tasks.md | `src/ui/detail.rs:265-272` is written in **two** `covers` arrays (`tests/degraded-coverage.toml:76` and `:116`) and the baseline's `sort -u` collapses them, so four distinct ranges are five lines to edit. Updating one site would leave the verification diff non-empty. | Both line numbers named in the task. | tasks.md 3.1 |
| SUGGESTION | tasks.md | The longest-change-name command did not produce the number recorded beside it: two `ls` arguments emit a `dirname:` header line that wins the numeric sort, returning `26 openspec/changes/archive/:`. The fact was right; the command was not. | Command corrected and re-run — it now returns `22 foldable-spec-sections`. | tasks.md counts table |
| SUGGESTION | design.md, specs/tasks-progress-bar, tasks.md | The totality argument attributed a division by zero to the `g == 0` arm. `gauge_of(p, 0)` already returns the empty string, since `filled` is `0` and both push loops are empty; only `total == 0` divides. | Stated precisely in all three: the `g == 0` arm codifies existing behaviour and is a characterization, the `total == 0` arm is the behaviour change. | design.md → Decision 6; the ADDED requirement and its guard scenario; tasks.md 1.1a |
| SUGGESTION | tasks.md | Task 2.1 named the 78-column empty-schema name field but not the 58-column one. | Both stated, with the arithmetic. | tasks.md 2.1 |
| SUGGESTION | tasks.md | `gate_controls_catch_their_plants` digests the whole tree before and after, so a concurrent write fails it spuriously — which is what slice D hit with four review agents live against this working tree. | Noted at the verification task so the implementer does not chase it as a defect. | tasks.md 7.5 |

## No Remaining Implementation-Blocking Gaps

None. The package validates `--strict`, every one of the 27 scenarios has a verification-matrix
row cross-checked by name, the measured red set is recorded with the command that produced it,
and both width gates pass at HEAD.

All four slices returned their verdicts. Slice A confirmed the capability-to-delta mapping, that
`responsive-layout` needs no delta (its 78/58 requirement fixes region **geometry**, and the
gauge moves no rectangle), that the palette requirement needs none either (its scenario asserts
the buffer against `header_row`'s own output, so it tracks the new grammar automatically), and
that all four artifacts agree on the drop order, the budget, the boundaries, and the
`total == 0` rule. Slice A also re-derived every figure the scenarios assert verbatim and found
them unchanged by the `u128` widening.

Three empirical claims underpin the plan and were each re-run by this session rather than taken
on report: the red set (`1308 passed; 3 failed` in the lib target under a design-conformant
gauge plant, plus one integration failure), the `redfail` helper's three branches, and every
column figure the specs assert — nine name-field widths, six gauge fills, and all four band
boundaries.

## Deferred Non-Blocking Notes

- **East Asian Ambiguous width.** `█` and `░` resolve to one column under `unicode-width`'s
  default but to two in a CJK-locale terminal, where the heading row will overrun its region.
  Accepted and uncompensated, as `SPEC.md` already records for the glyphs
  `markdown-constructs` and `markdown-legibility` introduced and for this same gauge one region
  below. This change widens a standing exposure rather than creating a new kind; the resolution
  point is whenever that standing exposure is addressed, for all of its sites at once.
- **The degraded-states row is deliberately not reworded.** *A marked tab's artifact resolves to
  no file…* claims the header "still shows the counted pair", which stays true with a gauge
  beside it. Its text is the merge key binding it to `tests/degraded-coverage.toml`'s
  `condition`, so rewording costs an identical edit at two further sites for no gain. Task 6.5
  records the decision so a future reader does not mistake it for an oversight.
- **A 12-column gauge cannot resolve a one-task move on a change with more than 12 tasks.**
  Inherent to any fixed gauge; the exact pair sits one space away, which is why the gauge is an
  addition to that cell and not a replacement. Revisit only if the budget itself is revisited.
