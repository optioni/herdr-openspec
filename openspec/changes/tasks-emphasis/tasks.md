<!-- Ordering: measured with `grep -rln "<symbol>" src/ tests/`. Groups 1 and 2 write one
     file each and neither is written by any other group (`src/tasks.rs`, `src/ui/palette.rs`);
     groups 3, 4, 5, 6 and 7 write `src/ui/markdown.rs`+`src/ui/view.rs`, `src/ui/tasks.rs`,
     `src/ui/tasks.rs`+`src/ui/detail.rs`, `src/ui/app.rs`+every ArtifactSection site, and
     `src/ui/app.rs`+`src/ui/detail.rs`. Group 8 writes only tests.

     Groups 1 and 2 pass parallelism tests 1 and 2 against each other and against every other
     group: no shared file, and neither needs the other's code — `LabelRole` is named by
     `src/ui/markdown.rs` (group 3), not by `src/ui/palette.rs`. They are rejected on test 3
     alone, for the reason `heading-sections` recorded: `make check` is a whole-tree gate, a
     concurrent failure would not stay attributable in a shared checkout, and this
     repository's rules forbid a worktree. No pair is marked `parallel-after`; every group is
     sequential, and that is a finding rather than an unexamined default. -->

<!-- No group 0. The change binds no key and alters no state transition, so a `run_loop` row
     would drive an untouched key to observe a rendering the view tier observes directly
     (design.md -> Test Strategy). The view tier is the outermost tier that can fail here, and
     group 8 is where the cross-capability view rows land. -->

## 0. Measurements

<!-- kind: operational -->

Every number this plan uses, with the command that produced it, run at HEAD on 2026-09-14.

| Figure | Command | Result |
|---|---|---|
| `Face` literals spelling every field out | brace-matched scan over `src/**/*.rs` for `Face {` whose body holds no `..` | **1** — `src/ui/tasks.rs:159` (`heading_line`); the other five hits are the struct definition, two doc comments, and two `impl` bodies |
| `ArtifactSection {` construction and pattern sites | `grep -rn "ArtifactSection {" src/ tests/ \| wc -l` | **78** |
| `ArtifactSection` spans `NODEFAULT-UI` scans | `SCAN_MIN=25 TYPES='ArtifactSection' /bin/sh scripts/gates/nodefault-ui.sh` | **72** spans, "none elides a field" — so every one names every field |
| `progress_bar(` call sites | `grep -rn "progress_bar(" src/ \| cut -d: -f1 \| sort \| uniq -c` | **51** — 39 `src/ui/tasks.rs`, 9 `src/ui/view.rs`, 3 `src/ui/detail.rs` |
| `bar_lines(` call sites | same, for `bar_lines(` | **6** — 4 `src/ui/tasks.rs`, 2 `src/ui/detail.rs` |
| `#[test]` count, `src/ui/tasks.rs` | `grep -c "#\[test\]" src/ui/tasks.rs` | **29** (`TASKWIDTHS` floor is 22) |
| `#[test]` count, `src/ui/palette.rs` | `grep -c "#\[test\]" src/ui/palette.rs` | **4** |
| Archive task items / labelled / compound | `python3` scan over `openspec/changes/archive/*/tasks.md` (proposal.md -> Measurements) | 2563 / 2272 / 76, with **0** false positives |
| Groups per archived task file | `for f in openspec/changes/archive/*/tasks.md; do grep -c '^## ' "$f"; done \| sort -n` | n=37, min 5, median 12, max 22 |

- [ ] 0.1 CHECK: Re-run every command above and confirm each figure still holds. A figure that
      moved invalidates the task that cites it — the `ArtifactSection` count sends you to 6.2,
      the `bar_lines` count to 5.3, and the `Face` count to 3.3.
- [ ] 0.2 CHECK: Confirm the baseline is green before any edit. Measured at HEAD on
      2026-09-14, with this change's own artifacts committed: `cargo test --all-features` exits
      **0** with **1413 passed, 0 failed, 1 ignored** across six binaries, and `make gates`
      exits **0**. Re-run both and confirm they still hold; a figure that moved means the
      baseline is not this one.
- [ ] 0.3 CHECK: Re-run the four RED checks below. Each was run at HEAD on 2026-09-14 and each
      returned **0**, so the behaviours this change adds are provably absent before group 1:
      `grep -rc "label_of" src/` → 0; `grep -c "muted" src/ui/markdown.rs` → 0;
      `grep -cE "Role::Muted|TaskEvidence" src/ui/palette.rs` → 0;
      `grep -A6 "pub struct ArtifactSection" src/ui/app.rs | grep -c progress` → 0.

## 1. `tasks::label_of` — the recognition and the vocabulary

<!-- kind: behavior -->

Writes `src/tasks.rs` alone. Deepest dependency, no collaborators: the function takes a `&str`
and returns plain data (design.md -> Decision 1).

- [ ] 1.1 RED: Write failing `tasks` unit tests named for the eight `task-labels` scenarios —
      `the_plain_and_compound_label_forms_are_both_recognised`,
      `a_task_number_is_skipped_and_does_not_become_part_of_the_label`,
      `unlabelled_tasks_are_recognised_as_unlabelled`,
      `the_recognition_is_total_over_degenerate_input`,
      `every_token_in_the_table_classifies_to_its_own_role`,
      `an_unrecognised_run_is_a_generic_label_not_a_miss`,
      `matching_is_case_sensitive_and_whole_run`, and
      `the_classification_reads_nothing_outside_its_argument`. Confirm each fails on the
      missing `label_of` rather than on a malformed fixture.
- [ ] 1.2 GREEN: Implement `LabelRole`, `Label { start, len, role }`, and `label_of`, following
      the five-step rule in `specs/task-labels/spec.md` exactly. Count the task number and the
      uppercase run with byte arithmetic and `trim_start_matches`-style scanning — `src/tasks.rs`
      is outside `COLWIDTH`'s `PURE` list, but the crate's one width measure is
      `ui::layout::columns` and this function measures no width at all.
- [ ] 1.3 GREEN: Implement the classification table as an exact, case-sensitive match over the
      run, with `Other` as the fallback arm rather than a lookup miss.
- [ ] 1.4 CHECK: Run
      `grep -nE 'schema::|Schema|config\.yaml|\.openspec\.yaml' src/tasks.rs` and confirm it
      prints nothing, proving the non-goal that this capability consults no schema.
- [ ] 1.5 REFACTOR: Fold the number-skip and the run-scan into one pass if two emerged, or
      record that none was needed.
- [ ] 1.6 VERIFY: `cargo test --all-features tasks::` — green — and `make gates` exits 0.

## 2. The five palette roles

<!-- kind: behavior -->

Writes `src/ui/palette.rs` alone. Sequential rather than parallel with group 1 for the reason
the ordering note above gives.

- [ ] 2.1 RED: Write failing `ui::palette` tests for
      `each_roles_modifier_set_is_exactly_the_table` (extended to five new rows, with the
      no-modifier count moving 7 → 11), `the_coloured_set_is_exactly_the_table` (four new rows,
      plus the assertion that `TaskEvidence` is `LightRed` and **not** `Red`), and
      `every_shared_style_is_licensed_and_the_unshared_roles_stay_unshared`. Confirm each fails
      on the missing `Role` variants.
- [ ] 2.2 GREEN: Add `Muted`, `TaskEvidence`, `TaskChange`, `TaskConfirm`, and `TaskLabel` to
      `Role` and to `style`, with the modifiers and colours `specs/view-palette/spec.md` states.
      Colour literals stay in this file's own tests, which is where the second, independent
      transcription lives.
- [ ] 2.3 GREEN: Implement the shared-style test by grouping every `Role` variant, every
      `AgentStatus`, and heading levels 1–6 by `Style` equality, and requiring each group of
      size greater than one to be one of the groups the spec enumerates.
- [ ] 2.4 CHANGE: Update the `table()` doc comment's "thirty-one rows" to the new count and the
      module doc's "Two pairs share a style deliberately" to the two licences
      (design.md -> Decision 7). These are the file's own prose, not a separate document.
- [ ] 2.5 VERIFY: `cargo test --all-features ui::palette` — green — and
      `/bin/sh scripts/gates/palette.sh` exits 0, proving no colour literal escaped the file.

## 3. `Face` gains two fields, and `style_for` composes nine roles

<!-- kind: behavior -->

Writes `src/ui/markdown.rs` and `src/ui/view.rs`.

- [ ] 3.1 RED: Write a failing `ui::markdown` test
      `the_markdown_path_sets_neither_new_face_field` at widths 58 and 78 over the
      multi-construct document the scenario names, asserting `muted: false` and `label: None`
      on every segment, and that the output is byte-identical to the pre-change result.
- [ ] 3.2 RED: Write a failing `ui::view` test
      `the_two_new_face_fields_compose_in_their_stated_positions` calling `style_for` on the
      four `Face` values the scenario names, including the unreachable `muted` + `label`
      combination.
- [ ] 3.3 GREEN: Add `muted: bool` and `label: Option<crate::tasks::LabelRole>` to `Face`, keep
      `Default`, and update the **one** construction site that spells every field out
      (`src/ui/tasks.rs:159`, per 0.1). The compiler names any site this count missed.
- [ ] 3.4 GREEN: Extend `style_for` to the nine-step fold — `Muted` first, `face.label` last —
      per `specs/view-palette/spec.md`.
- [ ] 3.5 CHECK: Contract gate — re-read design.md -> Contracts, confirm the five moved
      signatures still name every consumer, and that `Face`'s two new fields reach no consumer
      outside `src/`. No serialized or persisted form exists.
- [ ] 3.6 CHECK: Run `/bin/sh scripts/gates/mdseam.sh`, `noio-view.sh`, and `colwidth.sh` and
      confirm each exits 0 — `ui::markdown` naming `crate::tasks::LabelRole` must widen none of
      the three.
- [ ] 3.7 VERIFY: `cargo test --all-features ui::markdown ui::view` — green.

## 4. The checklist item grammar: split at the label, mute a finished row

<!-- kind: behavior -->

Writes `src/ui/tasks.rs`. `TASKWIDTHS` has no exemption list, so **every** test added here names
both `58` and `78` even where its interesting widths are elsewhere (design.md -> Test Strategy).

- [ ] 4.1 RED: Write failing `ui::tasks` tests
      `a_labelled_unchecked_item_splits_into_three_segments`,
      `a_checked_item_is_de_emphasised_whole_label_included`,
      `a_wrapped_labelled_item_labels_only_its_first_row`, and
      `a_label_split_across_a_wrap_degrades_to_unlabelled`. Each names 58 and 78; the last also
      sweeps 12, 14 and 16. Confirm each fails on the missing behaviour.
- [ ] 4.2 RED: Extend the two existing face assertions —
      `a_folded_group_and_an_unfolded_one_render_the_same_item_lines` and
      `groups_headings_items_and_separators_at_both_mandated_widths` — to the new fields, so a
      checked row's `muted: true` is asserted on the path that already existed.
- [ ] 4.3 GREEN: Implement the two-rule facing in `item_lines`: a checked item is one
      `muted: true` segment covering the whole row; an unchecked one calls
      `tasks::label_of(&item.text)` and splits into at most three segments, omitting any that
      would be empty.
- [ ] 4.4 GREEN: Implement the wrap degradation — when `start + len` exceeds the first row's own
      text length, render one `Face::plain()` segment (design.md -> Decision 9).
- [ ] 4.5 CHECK: Assert `Line::text()` is byte-identical to the pre-change output for every
      fixture in this group, checked and unchecked alike. This is the claim that the change
      moved no character, and it is the one a reviewer will want run.
- [ ] 4.6 REFACTOR: Extract the segment-building if `item_lines` grew a second copy of the
      prefix arithmetic, or record that none was needed.
- [ ] 4.7 VERIFY: `cargo test --all-features ui::tasks` — green — and
      `/bin/sh scripts/gates/taskwidths.sh`, `taskseam.sh` and `colwidth.sh` each exit 0.

## 5. The segmented gauge

<!-- kind: behavior -->

Writes `src/ui/tasks.rs` and `src/ui/detail.rs`'s two `bar_lines` call sites.

- [ ] 5.1 RED: Write failing `ui::tasks` tests
      `two_groups_of_unequal_size_get_spans_proportional_to_their_item_counts`,
      `an_empty_group_contributes_no_span_and_consumes_no_index`,
      `segmentation_is_skipped_below_the_legibility_floor`,
      `a_single_group_is_never_segmented`, and
      `segmentation_is_total_and_partitions_the_run_exactly`. Each names 58 and 78; three sweep
      0..=130. Confirm each fails on the missing parameter rather than on a compile error in the
      fixture.
- [ ] 5.2 GREEN: Add the `groups: &[tasks::Progress]` parameter to `progress_bar` and
      `bar_lines`, and implement segmentation as a **glyph substitution** over the run
      `gauge_of` already returns (design.md -> Decision 3). `gauge_of` itself does not move.
- [ ] 5.3 CHANGE: Update the 6 `bar_lines` call sites and the 51 `progress_bar` ones (per 0.1):
      `ui::tasks::lines` passes `group.progress()` per parsed group; every other site passes an
      empty slice, which the spec requires to reproduce the previous output byte for byte.
- [ ] 5.4 CHECK: Re-run the three carried `tasks-progress-bar` sweeps —
      `the_bar_measures_at_most_its_width_at_every_width`, the saturating-`Progress` test, and
      the drop-whole order test — with **both** an empty slice and a populated one, and confirm
      the empty-slice results are byte-identical to HEAD's.
- [ ] 5.5 REFACTOR: Fold the span arithmetic and the substitution into one pass if two emerged,
      or record that none was needed.
- [ ] 5.6 VERIFY: `cargo test --all-features ui::tasks ui::detail` — green — and
      `/bin/sh scripts/gates/taskwidths.sh`, `detailwidths.sh` and `taskseam.sh` each exit 0.

## 6. `ArtifactSection` gains a `progress` field

<!-- kind: refactor -->

Writes `src/ui/app.rs` and the 78 construction and pattern sites across `src/` and `tests/`.
Structure only: every site takes `None`, so no rendered output moves and the characterization
tests stay unchanged.

- [ ] 6.1 CHARACTERIZE: Run `cargo test --all-features` and record it green, so a later red in
      this group is attributable to the field rather than inherited.
- [ ] 6.2 REFACTOR: Add `progress: Option<crate::tasks::Progress>` to `ArtifactSection` and
      update all **78** sites (per 0.1) to `progress: None`. `NODEFAULT-UI` scans 72 spans and
      requires each to name every field, so none may be elided with `..`.
- [ ] 6.3 CHECK: Contract gate — re-read design.md -> Contracts, confirm the only consumers are
      `sync_detail`, `content_lines`, `Detail::foldable`, and test fixtures, and that
      `Detail::foldable` is still `sections.len() > 1`.
- [ ] 6.4 VERIFY: Run the unchanged tests — `cargo test --all-features` green — and
      `SCAN_MIN=25 TYPES='ArtifactSection' /bin/sh scripts/gates/nodefault-ui.sh` exits 0 with a
      span count at or above the 72 recorded in 0.1.

## 7. Per-group progress on the fold header row

<!-- kind: behavior -->

Writes `src/ui/app.rs` (`sync_detail`) and `src/ui/detail.rs` (`header`).

- [ ] 7.1 RED: Write failing `ui::app` tests
      `a_tracked_tasks_tabs_group_headers_carry_their_own_progress` and
      `a_group_holding_no_items_still_gets_a_header_and_a_counted_cell`, driving `sync_detail`
      with a closure reader and asserting on `detail.sections`. Confirm each fails on the
      field's `None` value rather than on a missing fixture.
- [ ] 7.2 RED: Write failing `ui::detail` tests
      `every_other_artifacts_section_headers_carry_no_progress_cell` and
      `the_progress_cell_is_dropped_whole_rather_than_truncated`, the latter sweeping content
      widths 0..=40 and naming 58 and 78 as the contrasted pair, since `DETAILWIDTHS` sweeps
      this file with no exemption list.
- [ ] 7.3 GREEN: In `sync_detail`, set `progress` to `tasks::parse(&section.text).progress()`
      for the heading sections of a split file whose `ArtifactRef` carries
      `tracks_tasks == true`, and leave every other section `None`.
- [ ] 7.4 GREEN: In `ui::detail::header`, draw the right-aligned cell from
      `ui::list::progress_cell` when the section carries a `progress`, dropped whole in the
      order `specs/artifact-folds/spec.md` states: the cell first, then the label, then the
      glyph and the indent.
- [ ] 7.5 CHECK: Assert the drawn cell is byte-identical to `ui::list::progress_cell` called on
      the same value, so the row provably does not format its own.
- [ ] 7.6 REFACTOR: Extract the right-align arithmetic if `header` grew a second copy of
      `pad_or_truncate_right`'s job, or record that none was needed.
- [ ] 7.7 VERIFY: `cargo test --all-features ui::app ui::detail` — green — and
      `/bin/sh scripts/gates/detailwidths.sh`, `colwidth.sh`, `noio-view.sh` and `readseam.sh`
      each exit 0.

## 8. The cross-capability view rows

<!-- kind: behavior -->

Writes test code in `src/ui/view.rs` only. This is the outermost tier that can fail here
(design.md -> Test Strategy); every collaborator below is replaced per design.md -> Test
Boundaries.

- [ ] 8.1 RED: Write failing `ui::view` tests
      `the_five_new_roles_leave_every_existing_cell_s_modifier_where_it_was`,
      `a_task_label_and_a_problem_row_are_distinguishable_in_one_frame`, and
      `a_checklist_row_reaches_the_buffer_with_its_label_coloured`, each rendering into a
      `TestBackend` at 120x20 and 60x20.
- [ ] 8.2 GREEN: No production code is expected here — groups 1 through 7 supply it. If a test
      fails, the defect is in one of those groups; fix it there and record which.
- [ ] 8.3 CHECK: Confirm no assertion in this group writes a colour literal: each compares
      against `palette::style(role)`. Run
      `grep -nE 'Color::(Red|Green|Blue|LightRed|DarkGray)' src/ui/view.rs` and confirm it
      prints nothing.
- [ ] 8.4 CHECK: Confirm the carried view rows are byte-identical — render the `color-palette`
      monochrome fixture and the three-spec fixture at both widths and compare against HEAD's
      buffers, which is the claim that this change moved no rendered text outside the tasks tab.
- [ ] 8.5 VERIFY: `cargo test --all-features ui::view` — green — and
      `/bin/sh scripts/gates/palette.sh` and `readonly-ui.sh` each exit 0.

## 9. Change Review

<!-- kind: operational -->

- [ ] 9.1 CHECK: Dispatch an independent `outside-in-tdd-reviewer` against proposal.md, all six
      spec deltas, design.md, and tasks.md, given the diff and the artifacts only — not this
      session's reasoning. Name the repository's own concentration points in the brief: nothing
      spawns outside `cli`; views do no I/O; the plugin writes nothing inside `openspec/`;
      view tests run at 60 and 120; and the six `*WIDTHS` gates carry no exemption list.
- [ ] 9.2 CHECK: Ask the reviewer specifically whether any test here can fail — in particular
      whether the byte-identical assertions in 4.5, 5.4 and 8.4 compare against a recorded
      literal rather than against the function's own fresh output, which would be tautological.
- [ ] 9.3 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 9.4 VERIFY: Confirm no blocking or unowned finding remains.

## 10. Documentation

<!-- kind: operational -->

- [ ] 10.1 Add in `SPEC.md`: the East Asian Ambiguous paragraph (audience: future implementers)
      — one sentence naming the gauge's two new shades `▓` and `▒` as joining that exposure.
      The markdown renderer's "seven glyphs, six Ambiguous" count does **not** move, because
      this change adds no markdown glyph; rewrite the surrounding sentence only if it reads as
      a total for the pane rather than for the renderer.
- [ ] 10.2 Rewrite in `AGENTS.md`: the paragraph beginning "Six of those seven glyphs"
      (audience: every agent session) — it repeats `SPEC.md`'s count and must say the same
      thing after 10.1. Rewrite in place; do not append a second sentence beside it.
- [ ] 10.3 Rewrite in `AGENTS.md`: the sentence stating the crate has "**one** gauge run beside
      its **one** progress cell" (audience: every agent session) — it stays true and gains a
      third consumer, the fold header row. Correct it in place rather than adding a rule; a
      future change rendering a change's progress in a fourth place must still call these two.
- [ ] 10.4 VERIFY: `cargo test --all-features --test doc_contract` — green — confirming no
      documented claim this change touched drifted from the file that determines it.

## 11. Lint & Verify

<!-- kind: operational -->

- [ ] 11.1 CHECK: Inspect the intended verification commands and affected tiers — `make check`
      is the single gate and runs format, lint, gates, test, and coverage.
- [ ] 11.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 11.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors. This
      is the crate's type checker for this purpose; `cargo check` is subsumed by it.
- [ ] 11.4 VERIFY: `make gates` — exits 0, every script included.
- [ ] 11.5 VERIFY: `cargo test --all-features` — green.
- [ ] 11.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — passes, and the production-slice
      floor from `scripts/coverage-prod.py` passes with it.
- [ ] 11.7 VERIFY: `make check` — exits 0 as a whole. If it fails, name the failing
      sub-command here rather than the composite.
- [ ] 11.8 VERIFY: `openspec validate tasks-emphasis --strict` — valid.
- [ ] 11.9 CHECK: Before archiving, re-extract the `markdown-render` and three `view-palette`
      requirement blocks from `openspec/specs/<capability>/spec.md` and compare by **phrase**
      against the copies in this change's deltas. `spec-emphasis` modifies both capabilities
      and a `MODIFIED` block carries the whole requirement, so whichever change archives second
      silently discards the first's edits (design.md -> Risks).
