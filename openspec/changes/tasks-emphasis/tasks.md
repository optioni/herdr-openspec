<!-- Ordering, with the file each group writes (`grep -rln` over the symbols named):

       1  src/tasks.rs
       2  src/ui/palette.rs
       3  src/ui/markdown.rs, src/ui/view.rs, src/ui/tasks.rs (the one Face literal at :159)
       4  src/ui/tasks.rs
       5  src/ui/app.rs and every ArtifactSection site in src/ and tests/
       6  src/ui/app.rs, src/ui/detail.rs, src/ui/view.rs (three fixtures and their helper)
       7  src/ui/tasks.rs, src/ui/detail.rs, src/ui/view.rs (nine progress_bar sites)
       8  src/ui/view.rs (tests only, but that is still a shared file)

     Groups 1 and 2 pass parallelism tests 1 and 2 against each other and against every other
     group: no shared file, and neither names the other's symbols — `LabelRole` is named by
     `src/ui/markdown.rs` (group 3), not by `src/ui/palette.rs`. They are rejected on test 3
     alone, for the reason `heading-sections` recorded and `openspec/config.yaml` backs:
     `make check` is a whole-tree gate, a concurrent failure would not stay attributable in a
     shared checkout, and this repository's rules forbid a worktree. No pair is marked
     `parallel-after`; every group is sequential, and that is a finding rather than an
     unexamined default.

     Groups 5, 6 and 7 are ordered by a real dependency, not by narrative: group 7's segmented
     gauge is fed from `ArtifactSection::progress`, which group 5 adds and group 6 populates.
     Planning review found the first draft had the gauge at group 5 passing an empty slice
     from the only path that renders it. -->

<!-- No group 0 acceptance test. The change binds no key and alters no state transition, so a
     `run_loop` row would drive an untouched key to observe a rendering the view tier observes
     directly (design.md -> Test Strategy). The view tier is the outermost tier that can fail
     here; groups 7 and 8 are where its rows land. -->

## 0. Measurements

<!-- kind: operational -->

Every number this plan uses, with the command that produced it, run at HEAD on 2026-09-14.

| Figure | Command | Result |
|---|---|---|
| `Face` literals spelling every field out | brace-matched scan over `src/**/*.rs` for `Face {` whose body holds no `..` | **1** — `src/ui/tasks.rs:159` (`heading_line`). `grep -rn "Face {" src/` gives **16** hits in all; the other 15 are the struct definition, doc comments, `impl` bodies, and 8 literals that use `..Face::plain()` or `..self.quoted_base()`. `tests/` constructs no `Face` |
| `ArtifactSection {` occurrences | `grep -rn "ArtifactSection {" src/ tests/ \| cut -d: -f1 \| sort \| uniq -c` | **78** total — 26 `src/ui/app.rs`, 19 `src/ui/detail.rs`, 14 `src/ui/driver.rs`, 14 `src/ui/view.rs`, 2 `tests/doc_contract.rs`, **3 `tests/gate-controls.toml`** |
| …of which the compiler forces | the 75 in `.rs` files, less the struct definition | **74** construction and pattern sites. The 3 in `gate-controls.toml` are planted-defect **strings** and are edited only if the plant itself must change |
| `ArtifactSection` spans `NODEFAULT-UI` scans | `SCAN_MIN=25 TYPES='ArtifactSection' /bin/sh scripts/gates/nodefault-ui.sh` | **72** spans, "none elides a field" |
| `progress_bar(` occurrences | `grep -rn "progress_bar(" src/ \| cut -d: -f1 \| sort \| uniq -c` | **51** — 39 `src/ui/tasks.rs`, 9 `src/ui/view.rs`, 3 `src/ui/detail.rs` |
| …split by `#[cfg(test)]` | count occurrences before and after the `#[cfg(test)]` marker per file | **2 production**, both in `src/ui/tasks.rs`; the other **49 are test sites**, including every one of the 9 in `view.rs` and the 3 in `detail.rs` |
| `bar_lines(` occurrences, same split | same, for `bar_lines(` | **2 production** in `src/ui/tasks.rs` (its definition and the call in `lines`) and **2** in `src/ui/detail.rs` (a doc-comment mention and the call at `:461`); 2 test sites |
| Gauge tests already carrying recorded literals | `grep -n "byte_identical\|does_not_move" src/ui/tasks.rs` | `full_grammar_is_byte_identical_to_pre_change_output` (`:1337`) and `the_bar_s_rendered_output_does_not_move` (`:1517`) — the pattern this change's own byte-identical assertions follow |
| View tests asserting a fold-header row | `grep -n "expected_header_at" src/ui/view.rs` | the helper at `:3974` and its call sites; the three tracked-tasks tests are at `:4488`, `:4540`, `:4659` |
| `#[test]` count, `src/ui/tasks.rs` | `grep -c "#\[test\]" src/ui/tasks.rs` | **29** (`TASKWIDTHS` floor is 22) |
| `#[test]` count, `src/ui/palette.rs` | `grep -c "#\[test\]" src/ui/palette.rs` | **4** |
| Archive task items / labelled / plain / compound / declined | `python3` scan over `openspec/changes/archive/*/tasks.md`, **first physical line of each item only**, matching what `tasks::parse` keeps (proposal.md -> Measurements) | 2563 / 2269 / 2196 / 73 / 3, with **0** items wrongly given a label |
| Groups per archived task file | `for f in openspec/changes/archive/*/tasks.md; do grep -c '^## ' "$f"; done \| sort -n` | n=38, min 5, median 11.5, max 22 |
| The 22-group worst case, and its gauge width | `archive/2026-09-06-agent-launch`: 22 groups, 81 items, cells `[81/81]` and `100%` | `g = 58 - 7 - 4 - 2 = 45` at the narrow interior, against the `2 * 22 = 44` floor — **one** column of headroom |

- [x] 0.1 CHECK: Re-run every command above and confirm each figure still holds. A figure that
      moved invalidates the task that cites it — the `ArtifactSection` counts send you to 5.2,
      the `progress_bar` split to 7.3, the `expected_header_at` lines to 6.5, and the `Face`
      count to 3.3.
- [x] 0.2 CHECK: Confirm the baseline is green before any edit. Measured at HEAD on 2026-09-14
      with this change's artifacts committed: `cargo test --all-features` exits **0** with
      **1498 passed, 0 failed, 1 ignored** across **ten** test binaries — lib 1353, main 0,
      `ci_workflow` 21, `cli` 10, `coverage_prod` 19, `degraded_coverage` 10 (+1 ignored),
      `doc_contract` 73, `gate_controls` 5, `manifest` 4, `spec_purposes` 3 — and `make gates`
      exits **0**. Capture the result with a redirect to a file, not a pipe into `grep`: a
      pipeline drops the last four binaries' result lines and yields 1413, which is how this
      figure was wrong in the plan's first draft.
      **Known flake:** `gate_controls_catch_their_plants` fails with "the real working tree
      changed while running the gate controls" if another agent touches this checkout during
      the run — its `TreeDigest` includes directory mtimes. Re-run in a quiet tree before
      attributing it to a change.
- [x] 0.3 CHECK: Re-run the four RED checks below. Each was run at HEAD on 2026-09-14 and each
      returned **0**, so the behaviours this change adds are provably absent before group 1.
      Note the first uses `-rl`, not `-rc`: `grep -rc` prints a `path:0` line per file and so
      never reports `0` on its own.
      `grep -rl "label_of" src/ | wc -l` → 0;
      `grep -c "muted" src/ui/markdown.rs` → 0;
      `grep -cE "Role::Muted|TaskEvidence" src/ui/palette.rs` → 0;
      `grep -A6 "pub struct ArtifactSection" src/ui/app.rs | grep -c progress` → 0.
- [x] 0.4 CHECK: Confirm the four greps above are **absence-of-string** checks, not
      absence-of-behaviour ones, and that the behavioural RED is each group's own 1.1/2.1/…
      task. A grep cannot fail for the right reason; it is recorded as a starting condition.
- [x] 0.5 CHARACTERIZE: **Capture the baseline this change's byte-identical claims compare
      against, before any edit.** Write the HEAD output of `ui::tasks::items` and
      `ui::tasks::lines` over each group-4 fixture, and of `ui::markdown::lines` over the
      group-3 document, at widths 58 and 78, into `notes/head-output.md` in this change
      directory. Without this the later assertions have nothing recorded to compare against:
      by the time group 4 runs the pre-change function is gone, and a literal written then
      proves forward stability only. This is the repair of a tautology planning review found
      in 4.5, 7.7 and 8.4.

## 1. `tasks::label_of` — the recognition and the vocabulary

<!-- kind: behavior -->

Writes `src/tasks.rs` alone. The function takes a `&str` and returns plain data
(design.md -> Decision 1).

- [x] 1.1 RED: Write failing `tasks` unit tests named for the eight `task-labels` scenarios —
      `the_plain_and_compound_label_forms_are_both_recognised`,
      `a_task_number_is_skipped_and_does_not_become_part_of_the_label`,
      `unlabelled_tasks_are_recognised_as_unlabelled`,
      `the_recognition_is_total_over_degenerate_input`,
      `every_token_in_the_table_classifies_to_its_own_role`,
      `an_unrecognised_run_is_a_generic_label_not_a_miss`,
      `matching_is_case_sensitive_and_whole_run`, and
      `the_classification_reads_nothing_outside_its_argument`. Confirm each fails on the
      missing `label_of` rather than on a malformed fixture.
- [x] 1.2 GREEN: Implement `LabelRole`, `Label { start, len, role }`, and `label_of` per the
      five-step rule in `specs/task-labels/spec.md`, with byte arithmetic throughout.
- [x] 1.3 GREEN: Implement the classification table as an exact, case-sensitive match over the
      run, with `Other` as the fallback arm rather than a lookup miss.
- [x] 1.4 CHECK: Confirm `label_of` and `LabelRole` reach no schema, over a **comment-stripped**
      copy of the file: `grep -vE '^\s*(//|///)' src/tasks.rs | grep -nE 'schema::|Schema|config\.yaml|\.openspec\.yaml'`
      prints nothing. The unstripped form is already red at HEAD — `src/tasks.rs:328` is a
      comment naming `schema::read_file` — so it would fail for the wrong reason on an
      untouched tree.
- [x] 1.5 REFACTOR: Fold the number-skip and the run-scan into one pass if two emerged, or
      record that none was needed.
- [x] 1.6 VERIFY: `cargo test --all-features` — green — and `make gates` exits 0.

## 2. The five palette roles

<!-- kind: behavior -->

Writes `src/ui/palette.rs` alone. Sequential rather than parallel with group 1 for the reason
the ordering note above gives.

- [x] 2.1 RED: Write failing `ui::palette` tests for
      `each_roles_modifier_set_is_exactly_the_table` (five new rows; the no-modifier count
      moves 7 → 11), `the_coloured_set_is_exactly_the_table` (four new rows, plus the
      assertion that `TaskEvidence` is `LightRed` and **not** `Red`),
      `every_shared_style_is_licensed_and_the_unshared_roles_stay_unshared`, and
      `the_enums_membership_is_exactly_this_list`. Confirm each fails on the missing `Role`
      variants.
- [x] 2.2 GREEN: Add `Muted`, `TaskEvidence`, `TaskChange`, `TaskConfirm`, and `TaskLabel` to
      `Role` and to `style`, appended after `Strikethrough`, with the modifiers and colours
      `specs/view-palette/spec.md` states. Colour literals stay in this file's own tests.
- [x] 2.3 GREEN: Implement the shared-style test by **discarding the uncoloured roles first**,
      then grouping the rest by `Style` equality against the spec's five-group table. Grouping
      every role would produce three unenumerated plain-modifier groups that already exist at
      HEAD, which is the defect planning review found in the first draft of this task.
- [x] 2.4 GREEN: Implement `the_enums_membership_is_exactly_this_list` as an exhaustive `match`
      over `Role`, so a later variant added without updating `specs/view-palette`'s reproduced
      enum fails to compile rather than drifting.
- [x] 2.5 GREEN: Update this file's own prose — `table()`'s "thirty-one rows" doc comment and
      the module doc's "Two pairs share a style deliberately" — to the new count and the two
      licences (design.md -> Decision 7).
- [x] 2.6 VERIFY: `cargo test --all-features` — green — and
      `/bin/sh scripts/gates/palette.sh` exits 0, proving no colour literal escaped the file.

## 3. `Face` gains two fields, and `style_for` composes nine roles

<!-- kind: behavior -->

Writes `src/ui/markdown.rs`, `src/ui/view.rs`, and the one `Face` literal in
`src/ui/tasks.rs`.

- [x] 3.1 RED: Write a failing `ui::markdown` test
      `the_markdown_path_sets_neither_new_face_field` at widths 58 and 78 over the
      multi-construct document the scenario names, asserting `muted: false` and `label: None`
      on every segment.
- [x] 3.2 RED: Write a failing `ui::view` test
      `the_two_new_face_fields_compose_in_their_stated_positions` calling `style_for` on the
      four `Face` values the scenario names, including the unreachable `muted` + `label`
      combination.
- [x] 3.3 GREEN: Add `muted: bool` and `label: Option<crate::tasks::LabelRole>` to `Face`, keep
      `Default`, and update the **one** construction site that spells every field out
      (`src/ui/tasks.rs:159`, per 0.1). The compiler names any site this count missed.
- [x] 3.4 GREEN: Extend `style_for` to the nine-step fold — `Muted` first, `face.label` last —
      per `specs/view-palette/spec.md`.
- [x] 3.5 CHECK: Contract gate — re-read design.md -> Contracts, confirm `Face`'s two new
      fields reach no consumer outside `src/`, and that no serialized or persisted form exists.
- [x] 3.6 CHECK: Run `/bin/sh scripts/gates/mdseam.sh`, `noio-view.sh`, and `colwidth.sh` and
      confirm each exits 0 — `ui::markdown` naming `crate::tasks::LabelRole` must widen none.
- [x] 3.7 VERIFY: `cargo test --all-features` — green. The whole suite, not a module filter:
      this group edits `src/ui/tasks.rs`, which a `ui::markdown ui::view` filter would not run.

## 4. The checklist item grammar: split at the label, mute a finished row

<!-- kind: behavior -->

Writes `src/ui/tasks.rs`. `TASKWIDTHS` has no exemption list, so **every** test added here
names both `58` and `78` even where its interesting widths are elsewhere.

- [x] 4.1 RED: Write failing `ui::tasks` tests
      `a_labelled_unchecked_item_splits_into_three_segments`,
      `a_checked_item_is_de_emphasised_whole_label_included`,
      `a_wrapped_labelled_item_labels_only_its_first_row`, and
      `a_label_split_across_a_wrap_degrades_to_unlabelled`. Each names 58 and 78; the last also
      sweeps 12, 14 and 16.
- [x] 4.2 RED: Extend the two existing face assertions —
      `a_folded_group_and_an_unfolded_one_render_the_same_item_lines` and
      `groups_headings_items_and_separators_at_both_mandated_widths` — to the new fields.
- [x] 4.3 GREEN: Implement the two-rule facing in `item_lines`: a checked item is one
      `muted: true` segment covering the whole row; an unchecked one calls
      `tasks::label_of(&item.text)` and splits into at most three segments, omitting any empty.
- [x] 4.4 GREEN: Implement the wrap degradation — when `start + len` exceeds the first row's own
      text length, render one `Face::plain()` segment (design.md -> Decision 9).
- [x] 4.5 CHECK: Assert every fixture's `Line::text()` equals the literal recorded in
      `notes/head-output.md` at 0.5, written into the test as a string literal — never
      recomputed from the function under test, which could not fail. This assertion carries
      the claim that the change moved no character, and it follows the pattern
      `full_grammar_is_byte_identical_to_pre_change_output` (`src/ui/tasks.rs:1337`) already
      uses.
- [x] 4.6 REFACTOR: Extract the segment-building if `item_lines` grew a second copy of the
      prefix arithmetic, or record that none was needed.
- [x] 4.7 VERIFY: `cargo test --all-features` — green — and
      `/bin/sh scripts/gates/taskwidths.sh`, `taskseam.sh` and `colwidth.sh` each exit 0.

## 5. `ArtifactSection` gains a `progress` field

<!-- kind: refactor -->

Writes `src/ui/app.rs` and the 74 compiler-forced sites across `src/` and `tests/`. Structure
only: every site takes `None`, so no rendered output moves and the characterization tests stay
unchanged.

- [x] 5.1 CHARACTERIZE: Run `cargo test --all-features` and record it green **at the head of
      this group**, after groups 1–4 have landed, so a red here is attributable to the field
      rather than to an earlier group. This is not 0.2, which measured the pre-change tree.
- [x] 5.2 REFACTOR: Add `progress: Option<crate::tasks::Progress>` to `ArtifactSection` and
      update all **74** compiler-forced sites (per 0.1) to `progress: None`. `NODEFAULT-UI`
      requires every span to name every field, so none may be elided with `..`. The 3
      occurrences in `tests/gate-controls.toml` are planted-defect strings; leave them unless
      the plant stops matching.
- [x] 5.3 CHANGE: Update `openspec/specs/detail-scroll/spec.md`'s companion obligation via this
      change's own `specs/detail-scroll/spec.md` delta — the live spec names `ArtifactSection`'s
      "all **three**" fields and this makes it four. Add the compile-time companion test the
      delta requires, destructuring all four with no `..`.
- [x] 5.4 CHECK: Contract gate — re-read design.md -> Contracts, confirm the only consumers are
      `sync_detail`, `content_lines`, `Detail::foldable`, and test fixtures, and that
      `Detail::foldable` is still `sections.len() > 1`.
- [x] 5.5 VERIFY: Run the unchanged tests — `cargo test --all-features` green — and
      `SCAN_MIN=25 TYPES='ArtifactSection' /bin/sh scripts/gates/nodefault-ui.sh` exits 0 with
      a span count at or above the 72 recorded in 0.1.

## 6. Per-group progress on the fold header row

<!-- kind: behavior -->

Writes `src/ui/app.rs` (`sync_detail`), `src/ui/detail.rs` (`header`), and three `ui::view`
fixtures with their shared helper.

- [x] 6.1 RED: Write failing `ui::app` tests
      `a_tracked_tasks_tabs_group_headers_carry_their_own_progress` and
      `a_group_holding_no_items_still_gets_a_header_and_a_counted_cell`, driving `sync_detail`
      with a closure reader and asserting on `detail.sections`.
- [x] 6.2 RED: Write the **view halves** of those same two scenarios in `ui::view`, rendering
      at 120x20 and 60x20 and asserting the drawn cells. design.md's matrix tiers both as
      "unit (pure) + view"; the unit half alone would not show the cell reaching a buffer.
- [x] 6.3 RED: Write failing `ui::view` test
      `every_other_artifacts_section_headers_carry_no_progress_cell` — `ui::view`, not
      `ui::detail`, per the matrix — and failing `ui::detail` test
      `the_progress_cell_is_dropped_whole_rather_than_truncated`, the latter sweeping content
      widths 0..=40 and naming 58 and 78 as the contrasted pair for `DETAILWIDTHS`.
- [x] 6.4 GREEN: In `sync_detail`, set `progress` to `tasks::parse(&section.text).progress()`
      for the heading sections of a split file whose `ArtifactRef` carries
      `tracks_tasks == true`, and leave every other section `None`.
- [x] 6.5 GREEN: In `ui::detail::header`, draw the right-aligned cell from
      `ui::list::progress_cell` when the section carries a `progress`, dropped whole in the
      order `specs/artifact-folds/spec.md` states: the cell, then the label, then the glyph and
      the indent.
- [x] 6.6 CHANGE: Amend the three existing `ui::view` fixtures that assert tracked-tasks
      fold-header rows — `a_foldable_tasks_tab_draws_its_groups_as_fold_headers` (`:4488`),
      `the_progress_bar_leads_the_folded_task_groups` (`:4540`), and
      `a_mostly_finished_task_file_opens_at_its_first_unfinished_group` (`:4659`) — and give
      their shared helper `expected_header_at` (`:3974`) a progress argument. These go red the
      moment 6.5 lands; without this task the group reports green on a red tree.
- [x] 6.7 CHECK: Assert the drawn cell is byte-identical to `ui::list::progress_cell` called on
      the same value, so the row provably does not format its own.
- [x] 6.8 REFACTOR: Extract the right-align arithmetic if `header` grew a second copy of
      `pad_or_truncate_right`'s job, or record that none was needed.
- [x] 6.9 VERIFY: `cargo test --all-features` — green — and
      `/bin/sh scripts/gates/detailwidths.sh`, `colwidth.sh`, `noio-view.sh` and `readseam.sh`
      each exit 0.

## 7. The segmented gauge, wired to the path that renders it

<!-- kind: behavior -->

Writes `src/ui/tasks.rs`, `src/ui/detail.rs`, and the nine `progress_bar` sites in
`src/ui/view.rs`. It follows groups 5 and 6 because its slice is fed from
`ArtifactSection::progress`.

- [x] 7.1 RED: Write the failing render row first —
      `a_real_tasks_tab_renders_a_segmented_gauge_into_the_frame` in `ui::view`, at 120x20 and
      60x20 over the two-group fixture, asserting at least one `▓` or `▒` in the progress-bar
      row and the two spans in the ratio the sections' own totals give. The fixture SHALL be
      built by calling `sync_detail` with a closure reader, **not** by hand-populating
      `detail.sections`: `monochrome_dashboard` (`src/ui/view.rs:7836`) hand-builds its
      sections and so cannot fail on a wiring defect between `sync_detail` and
      `content_lines` — which is the class of defect planning review found here. This is the
      only scenario in the change that renders a segmented gauge through `content_lines`; the
      other five pass hand-built slices and would all pass against an unwired build.
- [x] 7.2 RED: Write failing `ui::tasks` tests
      `two_groups_of_unequal_size_get_spans_proportional_to_their_item_counts`,
      `an_empty_group_contributes_no_span_and_consumes_no_index`,
      `segmentation_is_skipped_below_the_legibility_floor`,
      `a_single_group_is_never_segmented`, and
      `segmentation_is_total_and_partitions_the_run_exactly`. Each names 58 and 78; three sweep
      0..=130.
- [x] 7.3 GREEN: Add the `groups: &[tasks::Progress]` parameter to `progress_bar` and
      `bar_lines` and implement segmentation as a **glyph substitution** over the run
      `gauge_of` already returns (design.md -> Decision 3). `gauge_of` does not move.
- [x] 7.4 GREEN: Wire both production callers. `ui::tasks::lines` passes `group.progress()` per
      parsed group. `ui::detail::content_lines`' foldable branch — `src/ui/detail.rs:461`, the
      path every real `tasks.md` takes — passes `detail.sections`' own `progress` values in
      order, skipping `None`, and SHALL NOT re-parse the file to build the slice.
- [x] 7.5 CHANGE: Update the remaining `progress_bar` sites, including the **nine in
      `src/ui/view.rs`** (per 0.1), to pass an empty slice, which the spec requires to
      reproduce the previous output byte for byte.
- [x] 7.6 CHECK: Contract gate — `progress_bar` and `bar_lines` both appear in design.md ->
      Contracts with named consumers. Re-inspect both signatures against that table and confirm
      every consumer is named and the empty-slice compatibility claim holds.
- [x] 7.7 CHECK: Re-run the two literal-carrying gauge tests —
      `full_grammar_is_byte_identical_to_pre_change_output` (`src/ui/tasks.rs:1337`) and
      `the_bar_s_rendered_output_does_not_move` (`:1517`) — unmodified except for the new
      empty-slice argument, and confirm both stay green. They already hold recorded literals,
      which is what makes them falsifiable; `bar_measures_at_most_its_width_at_every_width` is
      a property sweep recording nothing and is **not** evidence for this claim.
- [x] 7.8 REFACTOR: Fold the span arithmetic and the substitution into one pass if two emerged,
      or record that none was needed.
- [x] 7.9 VERIFY: `cargo test --all-features` — green — and
      `/bin/sh scripts/gates/taskwidths.sh`, `detailwidths.sh` and `taskseam.sh` each exit 0.

## 8. The cross-capability view rows

<!-- kind: operational -->

Writes test code in `src/ui/view.rs`. These rows span every group above, so none of them can be
RED inside a group of its own — which is why this group is `operational` and its lifecycle is
CHECK → CHANGE → VERIFY rather than a manufactured RED. Its evidence is that the rows pass
against the assembled change and would fail against any one group reverted.

- [x] 8.1 CHECK: Write `ui::view` tests
      `the_five_new_roles_leave_every_existing_cell_s_modifier_where_it_was`,
      `a_task_label_and_a_problem_row_are_distinguishable_in_one_frame`, and
      `a_checklist_row_reaches_the_buffer_with_its_label_coloured`, each rendering at 120x20 and
      60x20. Run them and record the result.
- [x] 8.2 CHECK: Confirm each can fail, by reverting one group's production change in a scratch
      copy and recording which row goes red. A row no reverted group can redden is testing
      something other than this change.
      **Measured.** Three reverts, each run against the three rows:
      group 2 (`TaskEvidence` returns `Style::default()`) reddens
      `a_task_label_and_a_problem_row…` and `a_checklist_row…`;
      group 3 (`style_for` stops folding `face.label`) reddens the same two;
      group 4 (`item_lines` stops muting a checked item) reddens
      `the_five_new_roles…` and `a_checklist_row…`. Every row is reddened by at
      least one revert, and no row survives every revert.
- [x] 8.3 CHANGE: Fix whatever 8.1 or 8.2 reddens, in the group that owns it, and record which
      group and why here.
      **What reddened was an artifact, not a group's code.** 8.1's
      `a_task_label_and_a_problem_row_are_distinguishable_in_one_frame` could not
      pass as specified: `ui::view::detail_row_role` answers
      `ContentKind::Problem` with no role, so a **detail**-region problem row is
      `Style::default()`, and `ListProblem`'s red is reached only from
      `row_role`, in the **list** region — which at `Route::Detail` and 60
      columns is not drawn at all. The scenario, `design.md` -> Decision 6, and
      `proposal.md`'s resolved question 1 all asserted the opposite. All three
      are corrected in place; no production code moved, because the code was
      right and the claim about it was wrong. `TaskEvidence` still takes
      `LightRed`, now for the reason that survives the correction: a label and
      an `AgentBadge(Blocked)` can never meet, while a label and a `ListProblem`
      row **can**, at 120 columns where both regions are drawn.
- [x] 8.4 CHECK: Confirm `a_monochrome_reading_of_the_frame_is_unchanged` (`src/ui/view.rs:7928`)
      and the three-section fixture's own tests stay green **with their assertions unmodified**.
      They already assert per-cell literals, so leaving them untouched is the falsifiable form
      of "no rendered text moved outside the tasks tab"; re-rendering and comparing against a
      fresh buffer is not, and there is no recorded-buffer mechanism in this repository.
- [x] 8.5 VERIFY: `cargo test --all-features` — green — and
      `/bin/sh scripts/gates/palette.sh` and `readonly-ui.sh` each exit 0. `palette.sh`
      subsumes a hand-written grep for colour literals, so none is written here.

## 9. Change Review

<!-- kind: operational -->

- [x] 9.1 CHECK: Dispatch an independent `outside-in-tdd-reviewer` against proposal.md, all
      seven spec deltas, design.md, and tasks.md, given the diff and the artifacts only — not
      this session's reasoning. Name the repository's concentration points in the brief:
      nothing spawns outside `cli`; views do no I/O; the plugin writes nothing inside
      `openspec/`; view tests run at 60 and 120; the six `*WIDTHS` gates carry no exemption
      list.
- [x] 9.2 CHECK: Ask the reviewer specifically whether any assertion here can fail — the
      byte-identical comparisons in 4.5, 7.7 and 8.4 must compare against recorded literals,
      and every scenario must have a production path that reaches it. Planning review found
      five segmentation scenarios that would have passed against a build rendering none.
- [x] 9.3 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run affected tests.
      **No CRITICAL. Three WARNINGs, all fixed; two SUGGESTIONs, one fixed and
      one rejected against the evidence.**
      W1 — `src/ui/palette.rs`' production arm and its own test still carried
      the justification 8.3 falsified everywhere else. 8.3 said "all three are
      corrected in place" and named the scenario, design.md and proposal.md,
      missing the two code sites a future reader hits first. Both rewritten to
      the surviving reason.
      W2 — `ui::detail::content_lines`' doc comment read
      `bar_lines(&change.progress, &[], width)`, documenting the exact
      empty-slice defect planning review caught and 7.1 exists to prevent. The
      bulk rewriter that added `&[]` to the test call sites had edited a doc
      comment too. Replaced with the section-derived slice it actually passes.
      W3 — `the_classification_reads_nothing_outside_its_argument` could not
      fail: its two legs were a signature restatement and the determinism of
      any pure function, while the scenario's discriminating THEN lived only in
      1.4's one-time grep. It now runs that grep inside `cargo test` over an
      `include_str!` of this module's own production slice, with a positive
      control, and is proven to redden against a planted
      `crate::schema::Schema` in `label_of`.
      S1 — rejected on the evidence: the reviewer reported the
      `tests/degraded-coverage.toml` re-anchor as unrecorded, but commit
      `e57cbc0` carries a dedicated paragraph on it.
      S2 — fixed: `specs/detail-scroll/spec.md` stated **78** sites "measured
      with" a command that returns **81** after the change. Restated as the
      before-and-after pair it is, with the 74-of-78 compiler-forced split.
- [x] 9.4 CHECK: Before archiving, re-extract the `markdown-render`, `detail-scroll`, and four
      `view-palette` requirement blocks from `openspec/specs/<capability>/spec.md` and compare
      by **phrase** against this change's deltas. `spec-emphasis` modifies `markdown-render`
      and `view-palette` too, and a `MODIFIED` block carries the whole requirement, so
      whichever change archives second silently discards the first's edits
      (design.md -> Risks).
      **Done, and the hazard is one-directional today.**
      `openspec/changes/spec-emphasis/` holds a `proposal.md` and **no** `specs/`
      directory, so it has nothing yet for this change to discard. The
      obligation therefore falls on whoever writes its deltas: extract from the
      live specs **after** this change archives.
      `git log 53da335..HEAD -- openspec/specs/` is empty, so no live spec moved
      while this change was implemented and the deltas are still written against
      current text. Every `### Requirement:` header in all seven deltas resolves:
      the nine MODIFIED ones match a live requirement verbatim, and the three
      that do not are the ADDED blocks (`task-labels` ×2 and
      `tasks-progress-bar`'s segmentation rule). Compared scenario by scenario,
      **no** MODIFIED block drops a scenario the live requirement carries; each
      only adds — `artifact-folds` 4, `tasks-checklist` 4, `view-palette` 6
      across its four blocks, `markdown-render` 1, `detail-scroll` 1.
- [x] 9.5 VERIFY: Confirm no blocking or unowned finding remains.

## 10. Documentation

<!-- kind: operational -->

- [x] 10.1 Add in `SPEC.md`: the East Asian Ambiguous paragraph (audience: future implementers)
      — one sentence naming the gauge's two new shades `▓` and `▒` as joining that exposure.
      The markdown renderer's "seven glyphs, six Ambiguous" count does **not** move, since this
      change adds no markdown glyph; rewrite the surrounding sentence only if it reads as a
      total for the pane rather than for the renderer.
- [x] 10.2 Rewrite in `AGENTS.md`: the paragraph beginning "Six of those seven glyphs"
      (audience: every agent session) — it repeats `SPEC.md`'s count and must say the same
      thing after 10.1. Rewrite in place; do not append beside it.
- [x] 10.3 Rewrite in `AGENTS.md`: the sentence stating the crate has "**one** gauge run beside
      its **one** progress cell" (audience: every agent session) — it stays true and gains a
      third consumer, the fold header row. Correct it in place rather than adding a rule.
- [x] 10.4 VERIFY: `cargo test --all-features --test doc_contract` — green — confirming no
      documented claim this change touched drifted from the file that determines it.

## 11. Lint & Verify

<!-- kind: operational -->

- [x] 11.1 CHECK: Inspect the intended verification commands and affected tiers — `make check`
      is the single gate and runs format, lint, gates, test, and coverage.
- [x] 11.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [x] 11.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors. This
      is the crate's type checker for this purpose; `cargo check` is subsumed by it.
- [x] 11.4 VERIFY: `make gates` — exits 0, every script included.
- [x] 11.5 VERIFY: `cargo test --all-features` — green.
- [x] 11.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — passes, and the production-slice
      floor from `scripts/coverage-prod.py` passes with it.
- [x] 11.7 VERIFY: `openspec validate tasks-emphasis --strict` — valid.
- [x] 11.8 VERIFY: `make check` — exits 0 as a whole. If it fails, name the failing
      sub-command here rather than the composite.
