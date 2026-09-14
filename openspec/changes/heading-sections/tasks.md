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

- [x] 5.1 CHARACTERIZE: Confirm the seven landed grammar tests in `src/ui/tasks.rs` are green and record them as the invariant this group must not move — `groups_headings_items`, `nested_indent`, `long_item_hanging_indent`, `unbreakable_word_hard_split`, `indent_dropped_whole`, `empty_group_keeps_heading`, `headingless_leading_group`.
- [x] 5.2 RED: Write `a_folded_group_and_an_unfolded_one_render_the_same_item_lines` against the not-yet-existing `bar_lines`/`items`, asserting each against **literals** — `items`, given `tasks::parse(..).groups[0].items`, returns exactly the two item rows with no bar, heading, or blank row; `bar_lines` returns the bar row and one blank, and the empty vector at a width where the bar is empty. Do not assert `items`' output equals a slice of `lines`' output: once `lines` calls `items` that comparison cannot fail.
- [x] 5.3 GREEN: Extract `bar_lines(progress, width)` and `items(&[tasks::Item], width)`, and rewrite `lines` to call both — `items(&group.items, width)` per group, so no group is re-serialised. `lines`' returned vector must be unchanged for every one of the seven fixtures.
- [x] 5.4 CHECK: Contract gate — re-inspect `ui::tasks`' published surface: `lines`' signature and returned vector unchanged, `bar_lines`/`items` `pub(crate)` with group 6 as their only new consumer, and the private `item_lines` untouched.

      Contract gate result: `lines`' signature is byte-for-byte unchanged (`pub fn
      lines(source: &str, progress: &crate::tasks::Progress, width: u16) ->
      Vec<crate::ui::markdown::Line>`) and its returned vector is unchanged for all
      seven characterization fixtures, which pass unedited. `bar_lines(&crate::tasks::Progress,
      u16) -> Vec<crate::ui::markdown::Line>` and `items(&[crate::tasks::Item], u16) ->
      Vec<crate::ui::markdown::Line>` are both `pub(crate)` with no consumer outside this
      module yet — group 6 will be their first. The private `item_lines` is untouched.

- [x] 5.5 VERIFY: `cargo test --all-features ui::tasks` — green with the seven characterization tests unedited — and `/bin/sh scripts/gates/taskwidths.sh` exits 0.

## 6. The tracked-tasks tab folds, seeds, and clamps

<!-- kind: behavior -->

The Decision 8 reversal (design.md -> D8), the fold seed (-> D5), and the move to the line
cursor (-> D10), in one group because each one's tests need the others: a seeded fold is only
observable once the tab renders headers, and the clamp only changes once the tab is foldable.
Tasks 6.1's three `run_loop` rows are this change's end-to-end evidence and are RED at HEAD.

- [x] 6.1 RED: Write the failing `run_loop` tests — `j_walks_the_groups_rather_than_scrolling_the_lines`, and the **cursor-rule** half of the landed `checklist_scroll_is_clamped` in `src/ui/driver.rs` (twenty items under **two** headings). Confirm each fails because the tab is not yet foldable.

      Scope correction, made during group 3. The offset-rule half — the same twenty items under
      **no** heading — already landed there: group 3's split gate made `twenty_task_source`'s
      single `## Tasks` heading a section `label`, which left one section, no header row, and a
      checklist body that clamped identically to the markdown one, collapsing the `assert_ne!`
      that is the test's whole point. The fixture is now headingless, which is this half
      verbatim, and both tests are green. Only the two-heading half remains for this group.

      Done as `checklist_scroll_is_clamped_to_the_cursor_when_the_file_splits`, a sibling of
      the landed offset-rule test rather than a second half inside it, so each clamp rule
      names its own fixture. **Deviation from the scenario's letter, recorded here and in the
      test's own doc comment:** it scripts thirty `j` presses, not twenty. The foldable body is
      twenty-five rows (bar, blank, two open headers, twenty items, one separator), so twenty
      presses land at exactly twenty and the clamp never binds — which would falsify the
      scenario's own "not to `20`" clause and its "the last row holds the checklist's last
      item" clause at once. Thirty presses make both true.

      `j_walks_the_groups_rather_than_scrolling_the_lines` asserts the scenario's literal
      `detail.scroll` of `4` after `j j Space j j`. Row `4` is the first group's **second item
      row**, not the second group's header, which sits at row `6` once that group's two item
      rows and the blank separator are drawn; the test asserts both, since the clause's real
      claim — the cursor walks the rendered list rather than a fixed section index — is what
      row `6` moving down demonstrates.
- [x] 6.2 RED: Write the failing rendering tests — `a_foldable_tasks_tab_draws_its_groups_as_fold_headers`, `the_progress_bar_leads_the_folded_task_groups`, `a_task_file_holding_no_items_does_not_split`, `a_missing_artifact_file_renders_no_content_yet_and_nothing_else`, `a_mostly_finished_task_file_opens_at_its_first_unfinished_group`, `the_tasks_tab_seeds_its_folds_once_on_the_key_change`, `a_completed_group_does_not_fold_shut_under_the_reader` — and rewrite the landed `tasks_tab_shows_checkboxes` in `src/ui/view.rs` and `the_tracked_tasks_tab_concatenates_rather_than_folding` in `src/ui/detail.rs`.

      Two of the seven — `a_task_file_holding_no_items_does_not_split` and
      `a_missing_artifact_file_renders_no_content_yet_and_nothing_else` — were **green on
      arrival**: they pin behaviour the reversal must preserve, which the `total > 0` half of
      the split gate (landed in group 3) makes true by construction. The other five and the
      two rewrites were RED for the right reason.

      **Deviation, recorded:** `tasks_tab_shows_checkboxes` drives the scenario's **two**-group
      source rather than the one-group source the scenario's WHEN names. A one-group task file
      splits into exactly one section, which `Detail::foldable` reports not-foldable by design
      (design.md -> D10 names that edge in as many words and this group is forbidden to add a
      predicate), so no `v 1. Setup` fold header could ever be drawn for it and the scenario's
      THEN is unsatisfiable as written. The rows asserted are the scenario's own, in the
      scenario's own order, and the tab-0 half — the same source read through `markdown::lines`
      with no bar row above it — is unchanged.
- [x] 6.3 GREEN: Delete `content_lines`' tracked-tasks exemption branch; emit `bar_lines` above the section walk and render each open section's body with `items`.
- [x] 6.4 GREEN: Implement the seed in `sync_detail` — the subtree walk (a section plus every following section of strictly greater depth, to the first at or below its own) and insertion of those whose `tasks::count` over the concatenated subtree text reports `completed < total`, on the key change only.
- [x] 6.5 CHECK: Confirm `normalise_scroll` branches on `Detail::foldable()` and not on `tracks_tasks` — it already does, at `src/ui/app.rs:998`, so this is a confirmation and not an edit. Change it only if the confirmation fails.

      Confirmed, no edit made. `normalise_scroll` reads `self.detail.foldable()`
      (`src/ui/app.rs:1211` after this group's insertions) and the whole function body
      names `tracks_tasks` zero times.
- [x] 6.6 CHECK: Confirm the landed `marked_tab_renders_checklist_body` (`src/ui/view.rs`), `marked_tab_returns_the_checklist_body` (`src/ui/detail.rs`) and `prose_only_reads_no_tasks_yet` (`src/ui/view.rs`) still pass unedited — the `total > 0` half of the split gate exists to keep all three true by construction.

      `tab_move_resets_and_reclamps` (`src/ui/driver.rs`) was **removed from this list** during
      group 3. The claim that it passes unedited was wrong: it shares `twenty_task_source` with
      `checklist_scroll_is_clamped` and failed on its own `scroll == 9` precondition for the same
      reason. It was repaired with that fixture in group 3 and is green; confirm it still is,
      but it is not an unedited test.

      Confirmed. All three named tests are byte-identical to their `bc5a876` text — compared
      by extracting each function body from `git show bc5a876:<file>` and from the working
      tree, not by eyeballing the diff — and all three pass. `tab_move_resets_and_reclamps`
      passes too.
- [x] 6.7 CHECK: Persistence gate — confirm nothing is written to disk, that `expanded` stays per-session, and that the only cache is the existing `(change directory, tab)` key; record that no migration, backfill, invalidation, or index rebuild applies.

      Confirmed. The group's whole production diff is `ui::detail::content_lines`' dispatch,
      `ui::app::seed_expanded`, and four lines of `sync_detail`; grepping the added lines for
      a write, a file handle, a serialiser, or the state directory matches nothing.
      `detail.expanded` is a `BTreeSet<usize>` on `Detail`, in memory, discarded with the
      process; the plugin's own writes stay exactly `agent-names.toml` under
      `HERDR_PLUGIN_STATE_DIR`. The only cache is `detail.loaded`, still the
      `(change directory, tab)` pair, unchanged in shape and in the condition that resets it.
      No migration, backfill, invalidation, or index rebuild applies.
- [x] 6.8 VERIFY: `cargo test --all-features` — green, the three `run_loop` rows included — and `/bin/sh scripts/gates/readonly-ui.sh` and `noblock.sh` exit 0.

      `cargo test --all-features` green: 1346 lib tests (up from 1337) plus every integration
      tier — 21, 10, 19, 10, 73, 5, 4, 3 — 0 failed anywhere.
      `readonly-ui.sh`, `noblock.sh`, `detailwidths.sh`, `taskwidths.sh`, `colwidth.sh`,
      `noio-view.sh` and `palette.sh` each exit 0, and `make check` exits 0.

## 7. `Space`, the cursor, and the mouse over nested sections

<!-- kind: behavior -->

`specs/artifact-folds/spec.md` -> "`Space` toggles the artifact section the cursor is on or
in" and `specs/mouse-input/spec.md`. The `Space` arm already resolves through `content_lines`'
own `selected` flag, so this group is mostly new coverage plus the preamble case.

- [x] 7.1 RED: Write failing tests for `space_on_a_preamble_row_is_inert`, `closing_an_ancestor_preserves_its_subtrees_folds`, `a_click_on_a_task_groups_header_folds_that_group`, and `a_click_on_a_nested_scenario_header_folds_only_that_scenario`.
  - The first two landed in `src/ui/app.rs`, the last two in `src/ui/driver.rs`. All four passed against the group-6 tree on first run, so each was proved falsifiable against a **planted** defect instead (none committed): `detail_cursor_section` falling back to `Some(0)`; `toggle_detail_section` clearing its two following siblings on a close; and `detail_row_click` carrying the drawn row in place of the section index. Each plant turned the matching test red on its own assertion.
- [x] 7.2 GREEN: Make `Space` inert for a cursor above the first header row — the progress-bar row, its blank line, and a preamble body row — and confirm a toggle changes only that section's own membership of `expanded`.
  - **No production change was needed**, exactly as design.md -> D2 and D1 predicted. `content_lines` sets no header's `selected` flag when no header row sits at or before the cursor, so `detail_cursor_section` already returns `None` above the first one; and `toggle_detail_section` already touches one index of `expanded`, leaving a closed ancestor's descendants their membership (D1). No branch was added.
- [x] 7.3 CHECK: Confirm the landed `a_fold_reads_no_file` (`src/ui/driver.rs`) passes on its **read-count** claim — the reader records exactly one call per resolved path and none during the folds. Its row assertions were already repaired in 4.3; do not re-diagnose them here.
  - Passes, and the claim is live rather than vacuous: planting `sync_detail` to leave `detail.loaded` at `None` made the recorder log all three paths **six** times instead of once, and the read-count assertion is what went red.
- [x] 7.4 CHECK: Confirm the landed inertness tests still pass — `space_on_a_problem_row_is_inert` and `space_is_inert_on_a_non_foldable_artifact` in `src/ui/app.rs`, and `a_detail_header_click_equals_space` and `a_click_on_a_non_foldable_tab_is_inert` in `src/ui/driver.rs`.
  - All four pass unedited.
- [x] 7.5 CHECK: Confirm `list-selection`'s seven carried scenarios still hold unedited, and that its one changed fixture — `Space` at the detail route inert on a **prose** one-path artifact — is what the landed test now builds. The delta exists only because "resolves to one path" stopped implying "one section".
  - All eight pass unedited, and `space_at_the_detail_route_is_inert_on_a_non_foldable_artifact` builds one `Some("a")`-labelled section over `"one\n"` — prose, carrying no `### Requirement:` heading and on no tracked-tasks artifact, so the splitter leaves it unsplit and `foldable()` is false. That is the changed fixture the delta describes.
- [x] 7.6 VERIFY: `cargo test --all-features ui::app ui::driver` — green.
  - Run as `cargo test --all-features --lib -- ui::app ui::driver` (the literal two-argument form is rejected by `cargo test`, which takes a single TESTNAME): **241 passed, 0 failed**. `make check` exits 0 with the full suite at 1350 lib tests.

**Deviations found in this group's own planning artifacts** — reported rather than coded around:

1. `specs/artifact-folds/spec.md` -> "`Space` on a preamble row is inert" calls the preamble rows "any row from the progress bar through the last line of `Intro prose.`". On the tracked-tasks tab `Intro prose.` draws **no row at all**: a `None`-labelled section's body goes through `ui::tasks::items`, which renders task items and nothing else, and that text holds none. The preamble rows there are exactly the bar row and its blank line. The test asserts the real set and binds the finding.
2. `specs/mouse-input/spec.md` -> "A click on a task group's header folds that group" says a press on the progress-bar row or its blank line returns `Action::Ignore`. The requirement **table** directly above it sends every other drawn row of a foldable content area to `Action::Click(Target::DetailLine(line))`, and that is what `detail_row_click` does — the scenario contradicts its own table. The test asserts the table's answer and additionally proves the press folds nothing and leaves `Space` inert from where it lands, which is the property the scenario was reaching for.

## 8. Change Review

<!-- kind: operational -->

- [x] 8.1 CHECK: Dispatch an `outside-in-tdd-reviewer` subagent — not a fork of this session — against proposal.md, all six delta specs, design.md, and tasks.md, given only the artifacts and the diff.
- [x] 8.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, note each SUGGESTION, and re-run the affected tests.
  - **CRITICAL — a one-section split lost its heading row.** A file passing the split gate but yielding exactly one section (one resolved path, empty preamble, one heading) had its heading consumed into `ArtifactSection::label`; `Detail::foldable()` is then `false`, so `content_lines` took its non-foldable branch and drew the section *text* with no header row. The heading vanished from the screen, against `artifact-folds`' byte-identity sentence and `tasks-checklist`' "never both, and **never neither**".
  - **Repair, in `sync_detail` only:** a file's split is adopted only when it would yield more than one section — `splits && (base > 0 || usize::from(has_preamble) + headings.len() > 1)`. Otherwise the single unsplit section the pre-change path produced is contributed, so byte-identity is restored by construction rather than by a second exemption inside `content_lines` (design.md -> D3's own argument). `Detail::foldable()` stays `sections.len() > 1` (D10) and no call site moved. The `base > 0` clause keeps a one-heading file inside a multi-path artifact splitting, where its file section already draws the heading as a header row.
  - Covered by `a_single_heading_task_file_is_not_split_and_keeps_its_heading` and `a_single_requirement_spec_file_is_not_split_and_keeps_its_heading` in `src/ui/app.rs`, both asserting the shape, the verbatim text, not-foldable, and rows byte-identical to `ui::tasks::lines`/`ui::markdown::lines` at 78 and 58 — the two fixtures earlier repairs had removed, which is why the defect survived to review.
- [x] 8.4 CHANGE: Close the review's coverage gap — the derivation of a **multi-path** tracked-tasks artifact, and `seed_expanded`'s subtree walk at a **non-zero** `base`, were asserted only by transcription in `ui::detail`'s `the_tracked_tasks_tab_concatenates_rather_than_folding` and executed by nothing: every `sync_detail`-driven tracked-tasks fixture in the suite resolved to exactly one path. `a_two_path_tracked_tasks_artifact_nests_its_groups_and_seeds_them_all` in `src/ui/app.rs` now drives a two-path artifact through `sync_detail` with a closure reader, asserting the `(label, depth)` shape, each section's own `text`, and the seeded `expanded` set against literals. Test-only; no production line moved. This is the path the 8.2 CRITICAL survived seven gated groups on.
  - Proved falsifiable against two planted defects, both reverted and neither committed: (1) `seed_expanded` reading a section's own `text` instead of its subtree — the seed set drops to `{1, 3}`, since a file section's own text is empty; (2) `base` forced to `0` for a multi-path artifact — the shape collapses to two unsplit sections, because the one-heading files then fail the split gate's `base > 0 || contributions > 1`.
  - One fixture divergence from the transcription, derived rather than assumed: a group's `text` is `"\n- [ ] a\n"`, not `"- [ ] a\n"` — the body is the verbatim bytes between heading lines (design.md -> D6), so the blank line after the heading belongs to it. The hand-built `ui::detail` fixture omits it; that file is a `content_lines` unit test over a synthetic `Detail`, so nothing is wrong there. The label rule diverges too — it strips the parent directory only for `spec.md` — so the test runs both path pairs: `a/tasks.md`/`b/tasks.md` (labelling `tasks.md` twice, which also exercises "labels need not be unique") and `a/spec.md`/`b/spec.md` (labelling `a`/`b`, binding literally to the transcribed shape).
- [x] 8.3 VERIFY: Confirm no blocking or unowned finding remains, and record the triage in the change directory.
  - Triage recorded in `change-review.md` beside this file. One CRITICAL, fixed in 8.2. Two WARNINGs, both stale source doc comments, **routed to group 9** as 9.7 and 9.8 rather than accepted — a doc comment stating the opposite of the function beneath it is what group 9 exists to prevent. Two SUGGESTIONs: one folded into 9.4 (a second stale `SPEC.md` site), one **promoted** to 8.4 rather than noted, because it was the same species of gap the CRITICAL survived on. Nothing blocking and nothing unowned remains.

## 9. Documentation

<!-- kind: operational -->

- [x] 9.1 CHECK: Confirm the net size before editing — 9.3 and 9.4 rewrite rather than add, 9.5 is the only addition and is under ten lines, and 9.2's three Purpose blocks are rewrites. Nothing else in the maintained set is made stale by this change.
  - **Re-checked, and the plan's claim holds with two corrections.** Net size: 9.2, 9.3, 9.4,
    9.7 and 9.8 rewrite; 9.5 is the only addition, four lines. 9.4 is two sites as the Change
    Review found. **The quoted passage 9.3 names is not in `AGENTS.md`.** "never foldable, at
    any section count" appears in `SPEC.md:580` — inside 9.4's own content-description site —
    and in `openspec/specs/artifact-folds/spec.md:22`, which is 9.2's. `AGENTS.md`'s stale
    passages are different ones and were rewritten instead: its detail-region description
    ("one **section per resolved file**", "foldable per-file sections") and its tracked-tasks
    sentence, which described the unfolded body as the whole grammar.
  - One further site was found stale and **left alone deliberately**:
    `openspec/specs/artifact-content/spec.md:307` ("The tracked-tasks body is deliberately
    **not** foldable, at any section count") sits inside the requirement this change's own
    delta MODIFIES, so `openspec archive` rewrites it. Only the Purpose blocks ship stale,
    which is what 9.2 is for. `README.md` and the other capability Purposes were swept for
    the same phrases and carry none.
- [x] 9.2 Rewrite three `## Purpose` blocks in `openspec/specs/` (audience: every future change): `artifact-folds` (its Purpose states "one per resolved file" and carries the Decision 8 claim this change reverses verbatim), `artifact-content` ("one section per resolved file"), and `tasks-checklist` ("swaps `ui::markdown::lines` for `ui::tasks::lines`" — the foldable path uses `items`). A delta carries no Purpose block and `openspec archive` never rewrites one, so an untouched Purpose ships stale; this repository has already paid for that twice. `detail-scroll`'s, `mouse-input`'s and `list-selection`'s were read and stay true.
  - `artifact-folds`: the opening now states both section sources (per resolved file, and
    per ATX heading inside a spec-shaped or tracked task file), the `depth` field and the
    subtree-hiding rule, the `None`-labelled preamble, and the header's indent; the
    "needs no seeding" sentence gains the tracked-tasks exception; and the Decision 8 claim
    is rewritten as a **reversal**, with the bar-above-every-header argument that retires it.
  - `artifact-content`: "one section per resolved file" becomes both sources, and "more than
    one path is foldable" becomes more than one **section**, with the split gate's
    "only when it would yield more than one section" clause.
  - `tasks-checklist`: the swap is now `ui::markdown`'s grammar for `ui::tasks`', with which
    entry point draws it following foldability — `lines` whole when unsplit, `bar_lines` plus
    `items` per open group when split — and the "never both, never neither" rule for a
    group's heading.
- [x] 9.3 Rewrite in `CLAUDE.md` (audience: every future session): the passage stating the tracked-tasks tab "is never foldable, at any section count" and that its body concatenates. It is now false, and a session reading it would route around folding rather than use it.
  - Two passages in `AGENTS.md`, not the one the task line quotes (see 9.1): the detail
    region's content description now names both section sources, the `depth`, the
    collapsed-by-default rule and the split gate; and the tracked-tasks sentence now says the
    tab folds once its file splits, names the reversal, and names `bar_lines`/`items` and the
    incomplete-subtree seed.
- [x] 9.4 Rewrite in `SPEC.md` (audience: every future change): the detail region's content description — sections may come from headings, carry a `depth`, and a fold hides a subtree. **Two sites, not one:** the content description near `SPEC.md:566`, and the module-map row at `SPEC.md:96`, which still calls the dashboard's stored state "the detail region's per-file sections and its fold set". Sections are no longer per-file. Found by the Change Review; the second site is covered by none of this group's other edits.
  - `SPEC.md:564-582`: rewritten as three paragraphs — the two section sources and the flat
    `depth` encoding, the header grammar with its indent and the tracked-tasks seeding
    exception, and foldability with Decision 8's reversal stated as a reversal.
  - `SPEC.md:96`: the module-map row's stored state is now "the detail region's section list —
    one section per resolved file and, inside a spec-shaped or tracked task file, one per
    heading, each carrying a `depth` — and its fold set".
- [x] 9.5 Add to `CLAUDE.md` (≤4 lines): under `src/ui/`, where a pure helper lives decides which gates it must satisfy — `DETAILWIDTHS` and the other five `*WIDTHS` gates have no exemption list, so a width-free function cannot live in the file they sweep, and a new module moves a pure-view count two capability specs bind. Non-obvious, and the next change adding a pure helper pays the same discovery cost without it.
  - Added as a bullet in `AGENTS.md` -> Architecture rules, beside the `COLWIDTH` rule: the
    six `*WIDTHS` gates carry no exemption list, so a width-free function's tests belong in a
    file they do not sweep, and a new module moves the pure-view count two capability specs
    bind. Four lines.
- [x] 9.7 Rewrite `content_lines`' own doc comment in `src/ui/detail.rs` (roughly lines 391–401), which still states the pre-reversal rule and cites Decision 8 as live: "the concatenation of every section's text, when … `tracks_tasks == true` (`artifact-folds` -> Decision 8: this tab is never foldable, at any section count)". That is the opposite of what the function does twelve lines below it, where the dispatch is `if detail.foldable()`, `bar_lines` above the section walk, and `items` inside each open tracked-tasks section. WARNING from the Change Review. This is the contract documentation of the one function this change reversed, and no other task named a source doc comment.
  - `content_lines`' doc comment now dispatches on `Detail::foldable` rather than on the
    artifact's kind: the foldable branch (bar first as leading body, then the header/body/
    separator walk, `items` or `markdown::lines` per open section), the non-foldable branch
    over the concatenation, and Decision 8 named as **reversed** rather than as live.
- [x] 9.8 Split the doc block above `seed_expanded` in `src/ui/app.rs` (roughly lines 321–348). `preamble_len`'s documentation — "The byte length of `text`'s preamble …", including the backwards-walk argument that is the non-obvious part of this change's derivation — runs into `seed_expanded`'s with no break, so the compiler binds the whole run to `seed_expanded` and `preamble_len` carries no documentation at all. Move the first block down to `preamble_len` at its own definition. WARNING from the Change Review.
  - `preamble_len`'s block — the backwards-walk argument and the totality clause — moved
    intact to its own definition; `seed_expanded` keeps only its own. Both now carry
    documentation the compiler binds to the right function.
- [x] 9.6 VERIFY: `cargo test --all-features --test doc_contract` and `--test spec_purposes` — green, confirming no documented binding, module-map row, confined-seam name, or capability Purpose drifted.
  - `cargo test --all-features --test doc_contract --test spec_purposes`: **73 passed, 0
    failed** and **3 passed, 0 failed**. `cargo fmt --all` applied, `make check` exits 0 with
    the full suite green and both coverage floors holding (production 96.35% >= 96%).

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
