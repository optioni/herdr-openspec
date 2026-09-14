## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/task-labels/spec.md` (new capability)
- `specs/tasks-checklist/spec.md`
- `specs/tasks-progress-bar/spec.md`
- `specs/artifact-folds/spec.md`
- `specs/markdown-render/spec.md`
- `specs/view-palette/spec.md`
- `specs/detail-scroll/spec.md`

The last two did not exist when the review began. Slice A found that both capabilities own a
sentence this change falsifies, and both deltas were written as repairs.

## Reviewed Against

- This repository HEAD: `53da335ff7c7897f676e384d94d7823aba3f5908` at dispatch; repairs landed
  as `aa59a5f`, `8f95f2e`, and `de25431`. Slice D re-checked its verdicts against `8f95f2e`.
- Sibling repositories: **Not applicable**. The change adds no dependency, touches no seam, and
  names no external contract. `openspec-schemas` is vendored by graft and is not edited here.
- Working tree: clean at dispatch apart from this change's own planning files, which were
  committed before the reviewers ran so that `OPENSPEC-UNTOUCHED` would not fail on an
  untracked file.

The finding pass was delegated to four `planning-reviewer` subagents, one slice each, run
simultaneously, given the change directory and told to report findings only. None edited a
file. This session verified each finding against the source before acting on it, merged them,
and wrote the repairs.

## Gaps Found and Fixed

28 findings: 5 CRITICAL, 15 WARNING, 8 SUGGESTION. Every CRITICAL is repaired. The table gives
one row per gap that changed an artifact.

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `tasks.md` 5.3, `specs/tasks-checklist`, `design.md` Contracts | **The segmented gauge would never have been drawn.** Any `tasks.md` with two headings splits, so the tab is foldable, so its bar comes from `src/ui/detail.rs:461` — and the plan told that call site to pass an empty slice. All five segmentation scenarios build their own slice and call `progress_bar` directly, so the change would have shipped green with the feature in no frame at any width. Found independently by slices A, B and C. | The foldable branch passes `detail.sections`' own `progress` values, skipping `None`, and never re-parses. Groups reordered so the field lands before the gauge. A render-tier scenario now asserts `▓`/`▒` reach a buffer, built through `sync_detail` rather than by hand-populating `detail.sections`. | `specs/tasks-checklist` → three-function paragraph; `specs/tasks-progress-bar` → "A real tasks tab renders a segmented gauge into the frame"; `tasks.md` groups 5–7; `design.md` → Contracts |
| CRITICAL | `specs/view-palette` | **"Every shared style is licensed" could not pass.** Grouping all 36 roles by `Style` yields three unenumerated plain-modifier groups that already exist at HEAD, and the enumeration paired the uncoloured label roles with `Heading(3)`/`Heading(4)`/`Link`, which carry modifiers and so are not equal styles at all. | The rule now ranges over the **coloured** roles, where the shared set is exactly five groups, given as a table. The scenario discards uncoloured roles first and asserts the discarded set by name. | `specs/view-palette` → "Colour is added only where it carries a distinction a modifier cannot" |
| CRITICAL | `proposal.md`, `design.md`, `specs/task-labels`, `tasks.md` | **The corpus figures measured text the renderer never sees.** `tasks::parse` keeps only a task item's first physical line; the scan read the joined multi-line text. Three items whose colon sits on a continuation line were counted as labels, so "zero items carry a leading uppercase run that is not a label" — the sentence justifying step 4 — was false. | Figures restated as 2563 / 2269 / 2196 / 73 / 3 over first lines only, with the three offenders named. The argument is restated in the direction the corpus supports: the rule's errors are **misses**, never miscolourings, and **zero** items are given a label that is not one. | `proposal.md` → Measurements; `specs/task-labels` → the measured paragraph; `design.md` → Context and Decision 12; `tasks.md` → group 0 |
| CRITICAL | `design.md` Risks, `specs/view-palette`, `specs/tasks-checklist` | **"A non-foldable tracked-tasks tab carries no heading" was false** — and it was the argument licensing `TaskChange`/`Green` against `Heading(4)` and `TaskConfirm`/`Blue` against `Heading(3)`. A tasks file beginning at its single `##` heading has no preamble, so `contributions == 1`, so it does not split; `src/ui/detail.rs:517-519`'s own comment already named the case. | The colours stand on a different footing: the headings carry `BOLD` and the label roles carry none, so they are distinct `Style`s and the shared-style rule never reaches them. The scenario asserts both headings **alone** in their groups, which is what would catch a future table dropping the `BOLD`. `tasks-checklist` now names all three files that reach `lines`, not two. | `specs/view-palette` → the colour-reuse paragraph; `specs/tasks-checklist` → the `lines` clause; `design.md` → Risks |
| CRITICAL | `tasks.md` 0.2 | **The green baseline was wrong by 85 tests and four binaries** — 1413 across six, because piping `cargo test` into `grep` drops the last four binaries' result lines. | Restated as 1498 passed / 0 failed / 1 ignored across ten binaries, with the per-binary breakdown and an instruction to capture by redirect rather than pipe. The `gate_controls` mtime flake a concurrent agent causes is recorded beside it. | `tasks.md` → group 0 |
| WARNING | `proposal.md` | The Capabilities list omitted `markdown-render`, whose delta existed, and `detail-scroll`, whose spec this change falsifies. | Both named, with the reason each is touched. | `proposal.md` → Capabilities |
| WARNING | — | `openspec/specs/detail-scroll/spec.md` fixes `ArtifactSection`'s field list at "all **three**". | New delta reproducing the requirement with four fields and a scenario binding the compile-time companion. | `specs/detail-scroll/spec.md` (new) |
| WARNING | `specs/view-palette` | The live spec reproduces the 22-variant `Role` enum in a requirement this change did not modify — whose own prose says it is reproduced *because* changes keep altering its membership. | That requirement added to the delta with the five variants in place, its "two shared pairs" bullet widened to five groups, and a scenario binding membership to an exhaustive `match`. | `specs/view-palette` → "One module maps every semantic role to a `Style`" |
| WARNING | `tasks.md` 3.7, 5.6, 7.7 | **Every group's VERIFY ran a module filter narrower than the files that group edits**, so three groups would have reported green on a red tree. | Every group VERIFY is now `cargo test --all-features`, with the reason stated where it is not obvious. | `tasks.md` groups 1–8 |
| WARNING | `tasks.md` group 7 | Three existing `ui::view` fixtures assert tracked-tasks fold-header rows through a shared `expected_header_at` helper; the progress cell breaks all three and no task amended them. | Task 6.6 amends the three named fixtures and gives the helper a progress argument. | `tasks.md` 6.6 |
| WARNING | `tasks.md` 4.5, 8.4 | **The byte-identical assertions compared against nothing recorded.** By the time group 4 runs the pre-change function is gone, so a literal written then proves forward stability only — the assertion carrying this change's central claim was a tautology. | Group 0 captures the HEAD output of the group-3 and group-4 fixtures at 58 and 78 into `notes/head-output.md` before any edit. Where no recorded-buffer mechanism exists, the claim is restated as "the existing per-cell view assertions stay green unmodified". | `tasks.md` 0.5, 4.5, 7.7, 8.4; `design.md` → Test Strategy |
| WARNING | `design.md` matrix | `cargo test tasks::tests::label`, the command for eight rows, matches zero tests — it exits 0 at HEAD with "73 filtered out", green before and after with the behaviour absent. | Replaced with `cargo test tasks::` in all eight rows. | `design.md` → Test Strategy |
| WARNING | `tasks.md` 1.4 | The check was **already red at HEAD**: `src/tasks.rs:328` is a comment naming `schema::read_file`, so it would fail for the wrong reason on an untouched tree. | Runs over a comment-stripped copy. | `tasks.md` 1.4 |
| WARNING | `design.md` Test Boundaries | The table said the filesystem is not reached, while task 10.4 runs `--test doc_contract`, which reads `SPEC.md` and `AGENTS.md` off the tree. | Row added. | `design.md` → Test Boundaries |
| WARNING | `specs/tasks-progress-bar`, `proposal.md`, `design.md` | The gauge does **not** get `width - 11`; it gets `width` less both data-dependent cells and two spaces. The floor argument's headroom is one column, not three. | Arithmetic restated, with the archive's real worst case named: `agent-launch`, 22 groups, 81 items, `58 - 7 - 4 - 2 = 45` against a floor of 44. | `specs/tasks-progress-bar` → the legibility floor; `proposal.md` → What Changes; `design.md` → Decision 5 |
| WARNING | `tasks.md` group 8 | Task 8.2 admitted its GREEN was empty, so 8.1 could not be RED — a behavior group whose lifecycle could not run. | Re-marked `operational` with a CHECK → CHANGE → VERIFY lifecycle, and 8.2 now requires reverting one group to prove each row can fail. | `tasks.md` group 8 |
| WARNING | `tasks.md` group 5 | No contract gate, though it moves two signatures design.md lists with named consumers. | Contract gate added as 7.6. | `tasks.md` 7.6 |
| WARNING | `proposal.md` | The Why section carried first-draft counts (1,989 labelled; `VERIFY` 557) contradicting the Measurements table. | Replaced with the measured figures and a note marking the old ones superseded. | `proposal.md` → Why |
| WARNING | `proposal.md`, `specs/task-labels` | Several corpus sub-figures wrong: groups per file `n=37, median 12` (actually 38 and 11.5); `CHANGE — rewrite in \`SPEC.md\`:` 35 (actually 11); `RED→GREEN:` 2 (actually 3); the token table was the plain-form tally presented as the labelled set and omitted `DEFERRED`. | All corrected; the token table is now two rows, one per population. | `proposal.md` → Measurements; `specs/task-labels` |
| WARNING | `design.md` matrix | Three `artifact-folds` rows were tiered `view` while the tasks wrote them as `ui::app`/`ui::detail` tests. | Tasks 6.1–6.3 split into unit and view halves matching the matrix. | `tasks.md` group 6 |
| SUGGESTION | `tasks.md` 0.3 | `grep -rc` prints a `path:0` line per file and never reports `0`. | Changed to `grep -rl … \| wc -l`, with the reason noted. | `tasks.md` 0.3 |
| SUGGESTION | `tasks.md` 5.2 | The "78 sites" figure included three planted-defect **strings** in `tests/gate-controls.toml` and the struct definition. | Split into 78 occurrences of which **74** are compiler-forced. | `tasks.md` group 0, 5.2 |
| SUGGESTION | `design.md` Decision 3 | "By construction" was argued, not asserted. | The `0..=130` sweep now compares the filled-glyph count against the same call with an empty slice, and Decision 3 says what the phrase does and does not cover. | `specs/tasks-progress-bar`; `design.md` → Decision 3 |
| SUGGESTION | `design.md` Contracts | `progress_bar`'s consumer list said "`bar_lines` only" — true in production, but twelve test sites move with the signature. | Both stated. | `design.md` → Contracts |
| SUGGESTION | `tasks.md` group 0 | "the other five hits" for `Face {` — there are 16 hits, so fifteen others. | Corrected; the load-bearing figure (exactly one literal spells every field out) was confirmed. | `tasks.md` → group 0 |
| SUGGESTION | `tasks.md` 2.4, 5.3 | `CHANGE:` is the operational lifecycle's verb, used inside behavior groups. | Relabelled `GREEN:` where the task is part of a behavior group's implementation. | `tasks.md` group 2 |
| SUGGESTION | `tasks.md` 11.9 | Non-verification work appended after `make check`. | Moved into the Change Review group as 9.4. | `tasks.md` 9.4 |
| SUGGESTION | `tasks.md` ordering comment | The file map omitted `src/ui/tasks.rs` from group 3 and `src/ui/view.rs` from groups 5 and 8 — the evidence the parallelism decision rests on. | Rewritten as a per-group list. | `tasks.md` → ordering comment |

Three reviewer claims were **not** adopted as reported, having been checked against the source
first: slice A's reading that the four-vs-five roles contradiction was live (it was already
repaired but for one stale back-reference); its suggestion to group the palette by colour alone
(which would have merged `Heading` roles that `BOLD` legitimately separates — grouping coloured
roles by full `Style` is the correct form); and slice C's report that the `detail-scroll`
compile-time companion already exists (no `let ArtifactSection` destructuring exists at HEAD, so
the delta creates it rather than amending it).

## No Remaining Implementation-Blocking Gaps

None remain. All five CRITICALs are repaired, every WARNING is either repaired or recorded
below, and `openspec validate tasks-emphasis --strict` passes. The package is seven capability
deltas, 73 scenarios, every one carrying a verification-matrix row — checked by script, not by
eye.

Two things a reviewer should know were deliberately settled rather than left open:

- **The three Open Questions in `proposal.md` were answered by the user before any spec was
  written**, and are recorded there under *Resolved before specs were written* with the
  reasoning, so the decision can be disagreed with rather than re-derived.
- **The change adds five palette roles, not the four the proposal first said.** De-emphasising
  a completed item needs `Muted`; reusing `Quoted` would make a finished task and a block quote
  the same thing.

## Deferred Non-Blocking Notes

- **The `kind` marker stays out of scope**, with its `task-groups` delta unwritten. The
  proposal records why (a badge did not earn a cell on a heading row that now also carries a
  progress pair) and that a later change may take it. Resolution point: `proposal.md` →
  *Resolved before specs were written*, item 3.
- **Three archived task items render unlabelled** because their colon lands on a continuation
  line `tasks::parse` discards. They are named in `specs/task-labels/spec.md`, and the
  behaviour is specified rather than accidental: the rule's errors are misses. Resolution
  point: that spec's measured paragraph.
- **The gauge's legibility floor has one column of headroom at the archive's worst case.** A
  change with more than 22 groups, or a wider count cell at 22, degrades to the unsegmented
  gauge — a supported rendering, specified in `specs/tasks-progress-bar`. Resolution point:
  that capability's skip rule.
- **`spec-emphasis` modifies `markdown-render` and `view-palette` too**, so whichever change
  archives second silently discards the first's `MODIFIED` blocks. Resolution point:
  `tasks.md` 9.4, which requires re-extracting all six blocks by phrase before archiving, and
  `design.md` → Risks, which states the obligation in the other direction for whoever writes
  `spec-emphasis`'s specs.
- **`tests/gate_controls.rs` flakes when another agent touches this checkout during a run** —
  its `TreeDigest` includes directory mtimes. Observed once by slice D; an isolated re-run
  passed. Recorded in `tasks.md` 0.2 so a future red is not attributed to this change. It is
  also empirical support for the ordering note's refusal to mark any group `parallel-after`.
