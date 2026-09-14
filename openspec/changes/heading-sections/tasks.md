<!-- Ordering: measured with `grep -rn "ArtifactSection {" src/ tests/ | cut -d: -f1 | sort |
     uniq -c` and design.md -> Boundaries. Groups 1, 2, 3, 6 and 7 write `src/ui/app.rs`;
     groups 2, 4 and 6 write `src/ui/detail.rs`; group 5 writes `src/ui/tasks.rs` alone.
     Every pair therefore shares a file except pairs involving group 5. Group 5 passes
     parallelism tests 1 and 2 — nothing in groups 1-4 or 6-7 writes `src/ui/tasks.rs`, and
     its only ordering constraint is that group 6 consumes it — and is rejected on test 3
     alone: `make check` is a whole-tree gate, so a concurrent failure would not stay
     attributable in a shared checkout, and this repository's rules forbid a worktree. It was
     examined and left sequential; no pair is marked `parallel-after`. -->

<!-- No separate outer-loop group, and the reason is this repository's commit discipline
     rather than the absence of an outer tier. `run_loop` over a `TestBackend` IS the outer
     tier (design.md -> Test Strategy), and its three end-to-end rows are genuinely RED at
     HEAD. But they cannot pass until the tasks tab folds, which is group 6, and AGENTS.md
     requires every commit to pass `make check`. A group-0 outer loop would therefore sit red
     across five commits. The rows are instead the RED of group 6 — the group whose GREEN
     makes them pass — which keeps RED-before-GREEN intact with no red window. Planning
     review raised the original note's reasoning as unsound; this is the repair. -->

## 0. Measurements

<!-- kind: operational -->

Every number this plan uses, with the command that produced it, run at HEAD on 2026-09-14.

| Figure | Command | Result |
|---|---|---|
| Task groups per change | `for f in openspec/changes/archive/*/tasks.md; do grep -c '^## ' "$f"; done \| sort -n` | n=37, min 5, median 12, max 22 |
| Change spec files carrying `### Requirement:` | `grep -l '^### Requirement:' openspec/changes/archive/*/specs/*/spec.md \| wc -l` vs `ls ... \| wc -l` | 198 of 198 |
| Prose artifacts carrying `### Requirement:` at line start | `grep -l '^### Requirement:' openspec/changes/archive/*/{proposal,design,tasks}.md \| wc -l` | 0 |
| Fenced `#` lines in **spec-shaped** files, and how many are ATX-shaped | fence-tracking scan over the 198 archived change specs and 47 live specs | 15 fenced, **0** ATX-shaped (12 `#[derive(`, 3 `#[cfg(test)]`) |
| Fenced ATX-shaped lines in **archived `tasks.md`** | same scan over `openspec/changes/archive/*/tasks.md` | 1501 fenced, **1368 ATX-shaped**, over 37 files — the figure that makes fence tracking load-bearing (design.md -> D4) |
| `ArtifactSection {` sites to edit | `grep -rn "ArtifactSection {" src/ tests/ \| wc -l` = 67, of which 3 are the `tests/gate-controls.toml` planted defect and 1 is the struct definition | **63** to edit: app.rs 21, driver.rs 14, view.rs 13, detail.rs 13, doc_contract.rs 2. The `.toml` plant matches the struct's *header* line, so added fields leave it working |
| `noio-view.sh` / `colwidth.sh` `PURE` entries | `grep '^PURE=' scripts/gates/{noio-view,colwidth}.sh` | 10 / 9 — unchanged, because no new file is added under `src/ui/` |
| Landed tests the blank separator breaks | plant the separator in a `git archive HEAD` tree, then `cargo test --all-features` | `1315 passed; 5 failed` — the five named in task 4.3 |
| `DETAILWIDTHS` at HEAD, and under a planted width-free test | `/bin/sh scripts/gates/detailwidths.sh` | `OK: all 52 detail tests name both 58 and 78`; planted → `FAIL: … the_splitter_is_total_over_degenerate_input` |

- [x] 0.1 CHECK: Re-run the nine commands above and confirm each figure still holds. A figure that moved invalidates the task that cites it — the separator count sends you to 4.3, the fence count to D4, the `ArtifactSection` count to 2.2.

      Re-run result: eight of nine figures exact. The ninth moved — fenced ATX-shaped lines in
      archived `tasks.md` measured **1368**, not 1297; the fenced total (1501) is exact. The
      figure is cited only by D4, whose claim is that fence tracking is load-bearing, and a
      larger count strengthens it. No task is invalidated; the table row above is corrected.
      Baseline `cargo test --all-features`: green, 1320 lib tests.

## 1. The heading splitter

<!-- kind: behavior -->

`ui::app::split_headings` and `ui::app::is_spec_shaped`, per `specs/artifact-folds/spec.md`
-> "Headings split a file into nested sections". Sited in `src/ui/app.rs`, per design.md ->
D11: `DETAILWIDTHS` requires every test in `src/ui/detail.rs` to name both `58` and `78` and
has no exemption list, and these tests measure no width.

**RED probe, run at HEAD.** A temporary `tests/red_probe.rs`:

```rust
#[test]
fn split_headings_exists() {
    let s = herdr_openspec::ui::app::split_headings("## a\n");
    assert_eq!(s.len(), 1);
}
```

`cargo test --all-features --test red_probe` → **non-zero**, `error[E0425]: cannot find
function `split_headings` in module `herdr_openspec::ui::app``. Probe removed afterwards; it
is not part of the change.

- [x] 1.1 RED: Write failing unit tests named for the five splitter scenarios — `a_delta_spec_splits_into_operations_requirements_and_scenarios`, `a_task_file_splits_into_its_groups`, `a_heading_inside_a_fence_is_body_text`, `near_headings_are_not_headings`, `the_splitter_is_total_over_degenerate_input`. Confirm each fails on the missing function rather than on a malformed fixture.
- [x] 1.2 GREEN: Implement `HeadingSection { level, label, body }` and `split_headings`, with fence tracking and the ATX rule (at most three spaces of indent, one to six `#`, at least one following space, non-empty trimmed remainder).
- [x] 1.3 GREEN: Implement `is_spec_shaped`, asking `split_headings` for a level-3 heading whose label begins `Requirement:` rather than scanning the text a second time.
- [x] 1.4 RED→GREEN: Add the round-trip property — reassembling each section's heading line and `body` reproduces the input less its preamble — over all five fixtures plus a file with no trailing newline. Reassemble with `concat()` or `join("\n")`: `NOBLOCK`'s pattern matches a bare zero-argument `.join()`.
- [x] 1.5 REFACTOR: Fold the fence state and the heading match into one scan if two emerged, or record that none was needed. Count a `#` run with `trim_start_matches('#')` and byte arithmetic — `COLWIDTH` sweeps `src/ui/app.rs` whole-file for `.chars().count()`. **No refactor was needed:** the fence state and the ATX match were written as one `split_inclusive('\n')` pass from the first commit, and `marker_run`/`heading_of` count their runs with `trim_start_matches` and byte arithmetic. The one COLWIDTH failure the group did hit was a *doc comment* naming the forbidden pattern in prose, reworded rather than exempted.
- [x] 1.6 VERIFY: `cargo test --all-features ui::app` — green — and `/bin/sh scripts/gates/detailwidths.sh`, `colwidth.sh`, `noio-view.sh`, `noblock.sh` each exit 0, proving the width-free tests did not land in `src/ui/detail.rs` and that neither hazard above was hit.

## 2. `ArtifactSection` grows `label` and `depth`

<!-- kind: refactor -->

A mechanical widening with no behaviour change: every existing site becomes
`label: Some(..), depth: 0`. `ArtifactSection` is on the `NODEFAULT-UI` list precisely so this
fails to compile at each of its **67** construction sites rather than defaulting silently.

**CHECK, run at HEAD.** `SCAN_MIN=25 TYPES='ArtifactSection' /bin/sh scripts/gates/nodefault-ui.sh`
→ exit **0**. Its negative control is already recorded in `tests/gate-controls.toml` and run
by `cargo test --all-features gate_controls`.

- [x] 2.1 CHARACTERIZE: Run `cargo test --all-features` and record it green, so any later red is attributable to this change rather than inherited.
- [x] 2.2 REFACTOR: Change `label` to `Option<String>`, add `depth: usize`, and update all **63** construction and destructuring sites to `Some(..)` and `0`. Nothing else moves — `Detail::foldable` stays `sections.len() > 1`.
- [x] 2.3 CHECK: Contract gate — re-read design.md -> Contracts, confirm the only consumers are `sync_detail`, `content_lines`, `Detail::foldable`, and test fixtures, and that no serialized or persisted form exists.
  - Contract gate result: consumers confirmed by the compiler at exactly four sites — `sync_detail` (the one producer), `ui::detail::content_lines`, `Detail::foldable` (`sections.len() > 1`, untouched), and test fixtures; the type derives only `Debug, Clone, PartialEq, Eq`, so there is no serialized, wire, or persisted form.
- [x] 2.4 VERIFY: `cargo test --all-features` — green with the characterization tests unchanged — and the `NODEFAULT-UI` invocation above still exits 0.

## 3. Section construction in `sync_detail`

<!-- kind: behavior -->

`specs/artifact-folds/spec.md` -> "A multi-file artifact's content is a list of named sections"
and "A file splits at its headings only when it is a spec or a tracked task file";
`specs/artifact-content/spec.md` -> step 4.

- [x] 3.1 RED: Write failing `ui::app` tests for `a_prose_artifact_with_headings_does_not_split`, `a_spec_file_splits_and_a_task_file_splits`, `a_spec_file_whose_requirement_sits_inside_a_fence_does_not_split`, `a_spec_glob_nests_requirements_under_their_capability`, `a_preamble_becomes_an_unlabelled_section`, and `a_split_file_is_partitioned_rather_than_copied`. Drive each through `sync_detail` with a closure reader, per design.md -> Test Boundaries.
  - Four of the six failed; `a_prose_artifact_with_headings_does_not_split` and
    `a_spec_file_whose_requirement_sits_inside_a_fence_does_not_split` pass at HEAD by
    construction — nothing split yet — and stand as the regression guards for the gate's
    negative half.
  - `a_spec_glob_nests_requirements_under_their_capability` asserts **seven** sections, not
    the six the spec's prose counts: its own enumerated list names seven (one file section,
    four heading sections, two unsplit siblings). The list is implemented; the count word is
    a spec defect for Change Review.
- [x] 3.2 GREEN: Implement the split gate — `tracks_tasks && tasks::count(text).total > 0`, or `is_spec_shaped(text)` — and the file-section rule (a file section only when the artifact resolved to more than one path).
  - `is_spec_shaped` is not the call site: `artifact-content` allows one `split_headings` call
    per successfully read path, and `is_spec_shaped` makes one of its own. The predicate moved
    to a private `has_requirement_heading(&[HeadingSection])` that both callers share, so the
    rule is still written once.
- [x] 3.3 GREEN: Implement the preamble as a `None`-labelled section and the per-file depth normalisation `base + (level - min_level_in_that_file)`.
  - `split_headings` does not return the preamble, so `preamble_len` derives its length from
    the returned sections by walking them backwards — no second, fence-aware scan of the text.
- [x] 3.4 CHECK: Confirm the five landed file-axis tests still pass with no edit beyond group 2's mechanical widening — `the_three_spec_files_of_a_change_become_three_labelled_sections`, `an_unreadable_file_drops_its_section_and_keeps_its_siblings`, `an_artifact_with_no_resolved_paths_has_no_sections` and `the_label_derivation_is_total_over_adversarial_paths` in `src/ui/app.rs`, and `a_single_file_artifact_is_one_section_and_is_not_foldable` in `src/ui/view.rs`. They exist; do not write them again.
  - Confirmed, all five unedited: the four in `src/ui/app.rs` and
    `a_single_file_artifact_is_one_section_and_is_not_foldable` in `src/ui/view.rs`.
- [x] 3.5 VERIFY: `cargo test --all-features ui::app` — green, and the reader's recorded call count is unchanged for every pre-existing fixture.
  - `cargo test --all-features ui::app`: 151 passed, 0 failed. Every landed `calls()` assertion
    passes unedited — 0, 1, 1, 2, 0, 1, 0 at `src/ui/app.rs` lines 2306, 7693, 7720, 7763,
    8013, 8120, 8197, 8289 — so one call per resolved path per re-read still holds.
  - **Handed to group 6, not fixed here:** `ui::driver::tests::checklist_scroll_is_clamped` and
    `tab_move_resets_and_reclamps` are red. Both share `twenty_task_source` — `## Tasks\n` and
    twenty items — which now splits, so `content_lines`' tracked-tasks branch concatenates
    section texts that no longer carry the `## Tasks` line: 23 rows become 22 and the clamp
    goes 9 → 8. Task 6.3 (delete that exemption branch) is the repair. Task 6.1 already plans
    to rewrite the first; **6.6's claim that `tab_move_resets_and_reclamps` passes unedited is
    false** — it fails on the same shared fixture, at its `scroll == 9` precondition.

## 4. The fold walk: visibility, indent, and separators

<!-- kind: behavior -->

`specs/artifact-folds/spec.md` -> "A section header row names the file and shows its fold
state". The markdown path only; the tracked-tasks path is group 6.

- [x] 4.1 RED: Write failing `ui::detail` and `ui::view` tests for `a_fold_hides_a_whole_subtree`, `a_delta_spec_tab_opens_as_its_operation_headings_alone`, and `a_body_row_is_never_indented_by_its_sections_depth`, and extend the landed `a_narrow_pane_truncates_the_label_and_keeps_the_glyph` with the depth-2 and depth-3 indent cases. Render at 120 and 60 columns.
- [x] 4.2 GREEN: Implement the visibility walk — a collapsed labelled section at depth `d` hides every following section of depth greater than `d` until the first at or below `d` — and the `"  " * depth` header indent emitted before the glyph.
- [x] 4.3 GREEN: Implement the blank separator after a non-empty open body that a further visible section follows, and skip header emission for a `None` label. Then run `cargo test --all-features` and update every failing landed assertion. **Exactly five failures are expected**, measured by planting the separator in a `git archive HEAD` tree: `ui::detail::tests::a_foldable_tabs_body_is_headers_and_an_open_sections_markdown_beneath_its_own`, `ui::view::tests::folding_one_section_shows_its_body_and_leaves_its_siblings_shut`, `ui::driver::tests::a_fold_reads_no_file`, `ui::app::tests::space_opens_the_section_under_the_cursor_and_leaves_its_siblings_shut`, and `ui::app::tests::space_inside_an_open_section_folds_it_and_moves_the_cursor_to_its_header`. A sixth means the separator rule is wider than specified; fewer than five means it did not fire. The last of the five is **semantic** — `expanded` becomes `{0,1}` instead of `{0,2}` because the extra row shifts which section `detail.scroll` resolves to — so it is not found by grepping for adjacency.

      Measured: **exactly five**, the five named, with no sixth. The semantic one was
      repaired by moving its cursor rather than its expectation: its third block parks
      `detail.scroll` on the *third* header row, which the separator moved from row 3 to
      row 4, so the fixture is now `foldable_dashboard({0}, 4)` and `expanded` stays
      `{0,2}`. Asserting `{0,1}` at the unmoved `scroll` of 3 would have passed too, but
      would have silently changed the block's subject from a sibling header to the open
      section's own.
- [x] 4.4 RED→GREEN: Extend the two landed width properties — `content_lines_total` and `no_content_lines_line_exceeds_its_width_at_any_width`, both in `src/ui/detail.rs` — with the seven-section spec-glob and preamble fixtures over widths 0 through 130, asserting also that every `SectionHeader`'s carried `section` addresses an entry of `detail.sections`.
- [x] 4.5 REFACTOR: Extract the visibility walk if the header emission and the `selected` pass ended up scanning twice, or record that they did not.

      No further extraction was needed. The walk was already extracted during 4.2 as
      `ui::detail::visible_sections`, because the separator rule needs to look **ahead** —
      a blank row follows an open body only when a further *visible* section does — which
      is a question about the visible list rather than about the next index. The `selected`
      pass scans the emitted row list for `ContentKind::SectionHeader`, not
      `detail.sections`, so it never re-applies the visibility rule and does not scan twice.
- [x] 4.6 VERIFY: `cargo test --all-features` — green, all five repaired — and `/bin/sh scripts/gates/colwidth.sh` and `notabseam.sh` exit 0.

## 5. `ui::tasks` — `bar_lines`, `items`, and `lines`

<!-- kind: behavior -->

`specs/tasks-checklist/spec.md` -> "The checklist's line grammar". `lines` keeps its signature
and its output; `bar_lines(progress, width)` and `items(&[tasks::Item], width)` are new `pub(crate)`
functions it comes to call, per
design.md -> D9. The new function is `items`, not `item_lines`: `src/ui/tasks.rs:264` already
has a private per-item `item_lines`.

- [ ] 5.1 CHARACTERIZE: Confirm the seven landed grammar tests in `src/ui/tasks.rs` are green and record them as the invariant this group must not move — `groups_headings_items`, `nested_indent`, `long_item_hanging_indent`, `unbreakable_word_hard_split`, `indent_dropped_whole`, `empty_group_keeps_heading`, `headingless_leading_group`.
- [ ] 5.2 RED: Write `a_folded_group_and_an_unfolded_one_render_the_same_item_lines` against the not-yet-existing `bar_lines`/`items`, asserting each against **literals** — `items`, given `tasks::parse(..).groups[0].items`, returns exactly the two item rows with no bar, heading, or blank row; `bar_lines` returns the bar row and one blank, and the empty vector at a width where the bar is empty. Do not assert `items`' output equals a slice of `lines`' output: once `lines` calls `items` that comparison cannot fail.
- [ ] 5.3 GREEN: Extract `bar_lines(progress, width)` and `items(&[tasks::Item], width)`, and rewrite `lines` to call both — `items(&group.items, width)` per group, so no group is re-serialised. `lines`' returned vector must be unchanged for every one of the seven fixtures.
- [ ] 5.4 CHECK: Contract gate — re-inspect `ui::tasks`' published surface: `lines`' signature and returned vector unchanged, `bar_lines`/`items` `pub(crate)` with group 6 as their only new consumer, and the private `item_lines` untouched.
- [ ] 5.5 VERIFY: `cargo test --all-features ui::tasks` — green with the seven characterization tests unedited — and `/bin/sh scripts/gates/taskwidths.sh` exits 0.

## 6. The tracked-tasks tab folds, seeds, and clamps

<!-- kind: behavior -->

The Decision 8 reversal (design.md -> D8), the fold seed (-> D5), and the move to the line
cursor (-> D10), in one group because each one's tests need the others: a seeded fold is only
observable once the tab renders headers, and the clamp only changes once the tab is foldable.
Tasks 6.1's three `run_loop` rows are this change's end-to-end evidence and are RED at HEAD.

- [ ] 6.1 RED: Write the failing `run_loop` tests — `j_walks_the_groups_rather_than_scrolling_the_lines`, and the **cursor-rule** half of the landed `checklist_scroll_is_clamped` in `src/ui/driver.rs` (twenty items under **two** headings). Confirm each fails because the tab is not yet foldable.

      Scope correction, made during group 3. The offset-rule half — the same twenty items under
      **no** heading — already landed there: group 3's split gate made `twenty_task_source`'s
      single `## Tasks` heading a section `label`, which left one section, no header row, and a
      checklist body that clamped identically to the markdown one, collapsing the `assert_ne!`
      that is the test's whole point. The fixture is now headingless, which is this half
      verbatim, and both tests are green. Only the two-heading half remains for this group.
- [ ] 6.2 RED: Write the failing rendering tests — `a_foldable_tasks_tab_draws_its_groups_as_fold_headers`, `the_progress_bar_leads_the_folded_task_groups`, `a_task_file_holding_no_items_does_not_split`, `a_missing_artifact_file_renders_no_content_yet_and_nothing_else`, `a_mostly_finished_task_file_opens_at_its_first_unfinished_group`, `the_tasks_tab_seeds_its_folds_once_on_the_key_change`, `a_completed_group_does_not_fold_shut_under_the_reader` — and rewrite the landed `tasks_tab_shows_checkboxes` in `src/ui/view.rs` and `the_tracked_tasks_tab_concatenates_rather_than_folding` in `src/ui/detail.rs`.
- [ ] 6.3 GREEN: Delete `content_lines`' tracked-tasks exemption branch; emit `bar_lines` above the section walk and render each open section's body with `items`.
- [ ] 6.4 GREEN: Implement the seed in `sync_detail` — the subtree walk (a section plus every following section of strictly greater depth, to the first at or below its own) and insertion of those whose `tasks::count` over the concatenated subtree text reports `completed < total`, on the key change only.
- [ ] 6.5 CHECK: Confirm `normalise_scroll` branches on `Detail::foldable()` and not on `tracks_tasks` — it already does, at `src/ui/app.rs:998`, so this is a confirmation and not an edit. Change it only if the confirmation fails.
- [ ] 6.6 CHECK: Confirm the landed `marked_tab_renders_checklist_body` (`src/ui/view.rs`), `marked_tab_returns_the_checklist_body` (`src/ui/detail.rs`) and `prose_only_reads_no_tasks_yet` (`src/ui/view.rs`) still pass unedited — the `total > 0` half of the split gate exists to keep all three true by construction.

      `tab_move_resets_and_reclamps` (`src/ui/driver.rs`) was **removed from this list** during
      group 3. The claim that it passes unedited was wrong: it shares `twenty_task_source` with
      `checklist_scroll_is_clamped` and failed on its own `scroll == 9` precondition for the same
      reason. It was repaired with that fixture in group 3 and is green; confirm it still is,
      but it is not an unedited test.
- [ ] 6.7 CHECK: Persistence gate — confirm nothing is written to disk, that `expanded` stays per-session, and that the only cache is the existing `(change directory, tab)` key; record that no migration, backfill, invalidation, or index rebuild applies.
- [ ] 6.8 VERIFY: `cargo test --all-features` — green, the three `run_loop` rows included — and `/bin/sh scripts/gates/readonly-ui.sh` and `noblock.sh` exit 0.

## 7. `Space`, the cursor, and the mouse over nested sections

<!-- kind: behavior -->

`specs/artifact-folds/spec.md` -> "`Space` toggles the artifact section the cursor is on or
in" and `specs/mouse-input/spec.md`. The `Space` arm already resolves through `content_lines`'
own `selected` flag, so this group is mostly new coverage plus the preamble case.

- [ ] 7.1 RED: Write failing tests for `space_on_a_preamble_row_is_inert`, `closing_an_ancestor_preserves_its_subtrees_folds`, `a_click_on_a_task_groups_header_folds_that_group`, and `a_click_on_a_nested_scenario_header_folds_only_that_scenario`.
- [ ] 7.2 GREEN: Make `Space` inert for a cursor above the first header row — the progress-bar row, its blank line, and a preamble body row — and confirm a toggle changes only that section's own membership of `expanded`.
- [ ] 7.3 CHECK: Confirm the landed `a_fold_reads_no_file` (`src/ui/driver.rs`) passes on its **read-count** claim — the reader records exactly one call per resolved path and none during the folds. Its row assertions were already repaired in 4.3; do not re-diagnose them here.
- [ ] 7.4 CHECK: Confirm the landed inertness tests still pass — `space_on_a_problem_row_is_inert` and `space_is_inert_on_a_non_foldable_artifact` in `src/ui/app.rs`, and `a_detail_header_click_equals_space` and `a_click_on_a_non_foldable_tab_is_inert` in `src/ui/driver.rs`.
- [ ] 7.5 CHECK: Confirm `list-selection`'s seven carried scenarios still hold unedited, and that its one changed fixture — `Space` at the detail route inert on a **prose** one-path artifact — is what the landed test now builds. The delta exists only because "resolves to one path" stopped implying "one section".
- [ ] 7.6 VERIFY: `cargo test --all-features ui::app ui::driver` — green.

## 8. Change Review

<!-- kind: operational -->

- [ ] 8.1 CHECK: Dispatch an `outside-in-tdd-reviewer` subagent — not a fork of this session — against proposal.md, all six delta specs, design.md, and tasks.md, given only the artifacts and the diff.
- [ ] 8.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, note each SUGGESTION, and re-run the affected tests.
- [ ] 8.3 VERIFY: Confirm no blocking or unowned finding remains, and record the triage in the change directory.

## 9. Documentation

<!-- kind: operational -->

- [ ] 9.1 CHECK: Confirm the net size before editing — 9.3 and 9.4 rewrite rather than add, 9.5 is the only addition and is under ten lines, and 9.2's three Purpose blocks are rewrites. Nothing else in the maintained set is made stale by this change.
- [ ] 9.2 Rewrite three `## Purpose` blocks in `openspec/specs/` (audience: every future change): `artifact-folds` (its Purpose states "one per resolved file" and carries the Decision 8 claim this change reverses verbatim), `artifact-content` ("one section per resolved file"), and `tasks-checklist` ("swaps `ui::markdown::lines` for `ui::tasks::lines`" — the foldable path uses `items`). A delta carries no Purpose block and `openspec archive` never rewrites one, so an untouched Purpose ships stale; this repository has already paid for that twice. `detail-scroll`'s, `mouse-input`'s and `list-selection`'s were read and stay true.
- [ ] 9.3 Rewrite in `CLAUDE.md` (audience: every future session): the passage stating the tracked-tasks tab "is never foldable, at any section count" and that its body concatenates. It is now false, and a session reading it would route around folding rather than use it.
- [ ] 9.4 Rewrite in `SPEC.md` (audience: every future change): the detail region's content description — sections may come from headings, carry a `depth`, and a fold hides a subtree.
- [ ] 9.5 Add to `CLAUDE.md` (≤4 lines): under `src/ui/`, where a pure helper lives decides which gates it must satisfy — `DETAILWIDTHS` and the other five `*WIDTHS` gates have no exemption list, so a width-free function cannot live in the file they sweep, and a new module moves a pure-view count two capability specs bind. Non-obvious, and the next change adding a pure helper pays the same discovery cost without it.
- [ ] 9.6 VERIFY: `cargo test --all-features --test doc_contract` and `--test spec_purposes` — green, confirming no documented binding, module-map row, confined-seam name, or capability Purpose drifted.

## 10. Lint & Verify

<!-- kind: operational -->

- [ ] 10.1 CHECK: Inspect the intended verification commands and affected tiers — the pure `ui::*` unit tests, the 60/120-column view tests, the `run_loop` tests, `make gates`, and the two coverage floors.
- [ ] 10.2 VERIFY: `make lint` — 0 errors
- [ ] 10.3 VERIFY: `make fmt-check` — clean
- [ ] 10.4 VERIFY: `make gates` — every gate exits 0, `NOIO-VIEW` still reporting 10 pure files, `COLWIDTH` nine, and `DETAILWIDTHS` all detail tests naming both widths
- [ ] 10.5 VERIFY: `make test` — green
- [ ] 10.6 VERIFY: `make coverage` — both floors hold, the total and the production slice
- [ ] 10.7 VERIFY: `make check` — the single gate, green; name the failing sub-command if it is not
- [ ] 10.8 VERIFY: `openspec validate heading-sections --strict` — valid
