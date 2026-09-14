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
| WARNING | tasks.md | Whether the `covers` slide is silent or loud depends on **where** the shifted range lands, not how far it moves — and both this session and slice C initially generalised from a single plant in opposite directions. Simulated over the real file, `src/ui/detail.rs:245-251` lands on code (passing while naming the wrong code) at shifts of 1, 5, 15, 21 and 25, and inside a doc-comment block (failing loudly) only at about 10. Silence is the likelier outcome and the more dangerous one. | Group 3 rewritten around the simulation, instructing unconditional re-anchoring against the baseline shas, and moved to sit directly after the only two groups that shift lines. | tasks.md group 3 |
| WARNING | specs/tasks-progress-bar | Slice D listed three width-gate violators; a fourth — the saturation-boundary scenario added mid-review — had the same defect. | Repaired the same way, routing through `progress_bar` at 78 and 58. | specs/tasks-progress-bar |

Slice C's closing findings, both structural, verified against
`.claude/agents/apply-orchestrator.md` before repair:

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | tasks.md | The implementing group's exit gate could not pass. `the_header_reaches_the_buffer_without_crossing_the_region_border` lives in `src/ui/detail.rs:2191` and is one of the three measured reds, but its repair sat two groups later — and Step 4 requires a behavior group's tests to all pass before the next starts, handing the failure to a fresh implementer with no task explaining it. | Merged into the implementing group as task 2.2. | tasks.md group 2 |
| CRITICAL | tasks.md | The view-tier group ran *after* the implementation, so its three tasks labelled RED could not be red — the literal they instruct is what a gauge-bearing header already produces. | The view tier is now part of group 2, before 2.11 lands the gauge. The alternative, an `acceptance-red` outer-loop group, was reconsidered and declined on the record: it manages wiring risk, and this change adds no wiring. | tasks.md group 2; design.md → Test Strategy |
| WARNING | tasks.md | The red-set table stated a fourth failure (`every_table_row_has_a_proof`) as measured fact. It is not: it depends on how many lines the implementation inserts. | Count corrected to three, with the fourth marked insertion-count-dependent. | tasks.md → measured red set |

Slice D's closing suggestions, all verified and applied:

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| SUGGESTION | tasks.md | `src/ui/detail.rs:265-272` is written in **two** `covers` arrays (`tests/degraded-coverage.toml:76` and `:116`) and the baseline's `sort -u` collapses them, so four distinct ranges are five lines to edit. Updating one site would leave the verification diff non-empty. | Both line numbers named in the task. | tasks.md 3.1 |
| SUGGESTION | tasks.md | The longest-change-name command did not produce the number recorded beside it: two `ls` arguments emit a `dirname:` header line that wins the numeric sort, returning `26 openspec/changes/archive/:`. The fact was right; the command was not. | Command corrected and re-run — it now returns `22 foldable-spec-sections`. | tasks.md counts table |
| SUGGESTION | design.md, specs/tasks-progress-bar, tasks.md | The totality argument attributed a division by zero to the `g == 0` arm. `gauge_of(p, 0)` already returns the empty string, since `filled` is `0` and both push loops are empty; only `total == 0` divides. | Stated precisely in all three: the `g == 0` arm codifies existing behaviour and is a characterization, the `total == 0` arm is the behaviour change. | design.md → Decision 6; the ADDED requirement and its guard scenario; tasks.md 1.1a |
| SUGGESTION | tasks.md | Task 2.1 named the 78-column empty-schema name field but not the 58-column one. | Both stated, with the arithmetic. | tasks.md 2.1 |
| SUGGESTION | tasks.md | `gate_controls_catch_their_plants` digests the whole tree before and after, so a concurrent write fails it spuriously — which is what slice D hit with four review agents live against this working tree. | Noted at the verification task so the implementer does not chase it as a defect. | tasks.md 7.5 |

## Implementation-Time Corrections

Three claims in the reviewed package turned out to be wrong once the tests were written. None
changes the scope, the contracts, the drop order, the budget, or any spec scenario; all three are
corrections to what the package asserted about the **existing test baseline**, and each is
recorded at the task that hit it as well as here.

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| WARNING | tasks.md, design.md | "Twelve of the nineteen scenarios already have passing tests … carrying a ``/// `<capability>` :: "<scenario>"`` doc comment" overstates the baseline. At HEAD only **four** tests carried the doc comment (three in `src/ui/detail.rs`, one in `src/ui/view.rs`); the other nine were bound by snake-cased name alone. The rewrite-in-place decision is unaffected — a second test per scenario would collide by name either way — but the review's stated reason for it was half true. | Both artifacts corrected. Group 2 added the missing doc comment to each of the nine as part of its rewrite, so all twelve are now bound by name *and* by comment. | tasks.md preamble; design.md → Test Strategy |
| WARNING | tasks.md 2.5 | `below_the_full_form_band_the_header_is_byte_identical` was labelled CHARACTERIZE, "cannot honestly be RED". Its second clause — asserting the row **differs** from its pre-gauge string at 78 and 58, which review itself added so the test would discriminate and satisfy `DETAILWIDTHS` — necessarily fails before 2.11, because until then `header_row` *is* the pre-gauge grammar. Measured: `assertion left != right failed: width 78`. | Task relabelled: its first clause characterizes, its second is RED, and it was recorded as RED. The "cannot be RED" framing predates the clause review later added and was never reconciled with it. | tasks.md 2.5 |
| WARNING | tasks.md 2.13, design.md | 2.13's contract gate asked to confirm `src/ui/view.rs:5208` "needed no edit", contradicting 2.7, which rewrites the very test that line sits inside. design.md's Contracts paragraph reads the same way. The two are not in tension on the facts — the test is invariant to the gauge — but invariance is *why* it is rewritten, not why it is exempt. | 2.13's list reduced to `:6750` and `:6729`, which are genuinely unedited; design.md now distinguishes "not broken" from "not edited". The contract gate proper is `src/ui/view.rs:150`, the production call site, confirmed untouched in the diff. | tasks.md 2.13; design.md → Contracts |

A fourth observation needed no repair: the group 2 implementer saw `src/ui/detail.rs` briefly
revert two lines to an unformatted state between two `fmt` checks and attributed it to a
concurrent write in the shared working tree. No second agent was editing those files, the
condition did not recur, and the committed tree is `cargo fmt --all -- --check` clean.

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
