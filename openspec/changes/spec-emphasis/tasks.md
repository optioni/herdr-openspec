# spec-emphasis — tasks

Group 0 (outer-loop acceptance) is **deleted**. design.md → Test Strategy says why: the pane's
only real collaborators are the terminal, which no test may touch, and the filesystem, which is
injected at `sync_detail` — so a "full stack" test would be a `TestBackend` render with a
closure reader, which is exactly what the view-render tier already is.

**Planning-time evidence.** Every check below was run against HEAD `d75335d`; exit statuses and
output lines are recorded beside each. Two findings shaped the plan:

- `cargo test specs::` exits **0** at HEAD with `running 0 tests` — a `cargo test` filter that
  matches nothing **passes**. No task may use a bare filter as a RED check; the RED checks below
  are `test -f` and `grep -q`, which genuinely fail, plus the named test's own failure.
- `role_of` **already exists** — `grep -n "fn role_of" src/tasks.rs` → exit 0,
  `205:fn role_of(run: &str) -> LabelRole {`. Group 1 widens an existing seam rather than
  extracting a new one, which is why it is a `refactor` group and not a `behavior` one.

**Parallelism: none, and that is a finding rather than an unexamined default.** Group 3
(`src/ui/palette.rs`) passes independence tests 1 and 2 against groups 1 and 2 — no shared file,
neither names the other's symbols — and group 7 (`src/ui/markdown.rs`) passes them against
groups 5 and 6. Both are **rejected on test 3**: `openspec/config.yaml:41` forbids a worktree
("Work in the main checkout on `main`"), so concurrent groups share one tree and one compile,
and `make check` is a whole-tree gate under which a concurrent failure does not stay
attributable to its own group. This is the same pair and the same rejection
`archive/2026-09-15-tasks-emphasis/tasks.md:12-19` recorded, citing `heading-sections` before
it. An earlier draft of this file marked groups 3 and 7 `parallel-after` and was wrong.

Counts used below, each with the command that produced it:

- **75** construction sites, **every one of which spells all fields out**, confirmed two ways:
  `TYPES='ArtifactSection' SCAN_MIN=1 /bin/sh scripts/gates/nodefault-ui.sh` →
  `75 literal/pattern spans scanned … none elides a field`, and
  `grep -rc "ArtifactSection {" src/` → `app.rs:29`, `detail.rs:19`, `driver.rs:14`,
  `view.rs:14` = 76, less `src/ui/app.rs:152`, which is the `pub struct` definition rather than
  a construction. So the per-file split is `app.rs` **28**, `detail.rs` 19, `driver.rs` 14,
  `view.rs` 14.
- **0** rest patterns, and `NODEFAULT-UI` half B forbids one — so the field cannot be elided
  anywhere. Two earlier drafts of this line were wrong: the first claimed 10 rest patterns,
  from a `\.\.\w` grep that matched range expressions (`(0..20)`, `0..500`) and no rest pattern
  at all; the second counted the struct definition as a construction site. Both errors ran in
  the direction that understates the implementer's work.
- `ls scripts/gates/ | wc -l` → **31**. This change adds no gate, so it must still be 31.
- `make gates` → exit **0** at HEAD, reporting `NOIO-VIEW OK: 10 pure files`,
  `COLWIDTH OK: ... nine pure view files`, `MDSEAM OK: 26 files searched`,
  `WIDTHS OK: all 149 view tests name both 60 and 120`.

## 1. Widen `tasks::role_of` to a shared table

<!-- kind: behavior -->

Classified `behavior`, not `refactor`: `an_unrecognised_run_is_none_to_the_table_and_other_to_the_label` asserts `None`, which does
not compile against today's `fn role_of(run: &str) -> LabelRole` (`src/tasks.rs:205`), so the
group has an honest RED state rather than a manufactured one.

- [ ] 1.1 RED: Write `the_table_is_reachable_on_its_own_and_label_of_agrees_with_it` and `an_unrecognised_run_is_none_to_the_table_and_other_to_the_label` in
      `src/tasks.rs`'s inline `mod tests`, from the two same-named scenarios in
      specs/task-labels. RED check at HEAD: `grep -rq "pub fn role_of" src/tasks.rs` → exit
      **1**, and `an_unrecognised_run_is_none_to_the_table_and_other_to_the_label` does not compile against the current signature.
- [ ] 1.2 GREEN: Make `role_of` `pub` and change its return to `Option<LabelRole>`, moving its
      `_ => LabelRole::Other` arm to `label_of`'s call site as `.unwrap_or(LabelRole::Other)`.
      The existing `tasks::tests::label_*` tests are the unchanged-behaviour anchor and must
      stay green and unedited.
- [ ] 1.3 REFACTOR: Clean up while green, or state that none was needed.
- [ ] 1.4 VERIFY: `cargo test tasks::` green with the pre-existing `label_*` tests unedited, and
      both new tests passing — the equivalence design.md → Contracts claims is now asserted by a
      test rather than by a signature grep.

## 2. `crate::specs` — the delta and clause classifiers

<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing tests in `src/specs.rs` for `each_of_the_three_operation_headings_classifies_to_its_own_variant`,
      `internal_whitespace_is_tolerated_and_nothing_else_is`, `only_a_level_2_heading_carries_an_operation`, `a_renamed_operation_and_a_main_specs_heading_both_decline`,
      `the_recognition_is_total_over_degenerate_input`, `the_three_measured_keywords_classify_as_specified`, `the_wider_testing_vocabulary_classifies_through_the_same_table`, `a_run_outside_the_table_is_not_a_clause`,
      and `the_classification_reads_nothing_outside_its_argument`, from the nine same-named scenarios in specs/spec-delta-badges.
      RED check at HEAD: `test -f src/specs.rs` → exit **1**.
- [ ] 2.2 GREEN: Add `pub mod specs;` to `src/lib.rs` and implement `DeltaOp`,
      `operation_of_heading(level, label)`, `Clause`, and `clause_of(run)`, with `clause_of`
      calling `crate::tasks::role_of` and never restating the token table.
      RED check at HEAD: `grep -rq "DeltaOp" src/` → exit **1**.
- [ ] 2.3 CHECK: The token table must not be copied. Scope the sweep to the **production
      slice** — everything above `mod tests` — exactly as `src/tasks.rs`'s own
      `the_classification_the_classification_reads_nothing_outside_its_argument_outside_its_argument` already does:
      `awk '/^mod tests/{exit} {print}' src/specs.rs | grep -nE '"(RED|GREEN|VERIFY|CHARACTERIZE|ARRANGE|ACT|ASSERT)"'`
      must print nothing, and `grep -q "crate::tasks::role_of" src/specs.rs` must exit 0.
      Unscoped, this check is guaranteed to fail on a correct implementation: 2.1's
      `the_wider_testing_vocabulary_classifies_through_the_same_table` is required by specs/spec-delta-badges to call `clause_of` on
      `GIVEN`, `ARRANGE`, `ACT`, `ASSERT`, and `RED`, so those literals must appear in the test
      module. Measured on the established analogue:
      `grep -cE '"(RED|GREEN|VERIFY|CHARACTERIZE|ARRANGE|ACT|ASSERT)"' src/tasks.rs` → **6**.
- [ ] 2.4 REFACTOR: Clean up while green, or state that none was needed.
- [ ] 2.5 Run the group tests — `cargo test specs:: && cargo test tasks::`, both green, and
      confirm the run reports a non-zero test count for `specs::` rather than the
      `running 0 tests` HEAD result recorded above.

## 3. Three `Delta*` palette roles

<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests `the_three_delta_roles_carry_their_colour_and_no_modifier` and an extension of the existing
      `every_shared_style_is_licensed_and_the_unshared_roles_stay_unshared` pairwise test, from
      the specs/view-palette scenarios "The three delta
      roles carry their colour and no modifier" and "The full set of shared coloured styles is
      still exactly five groups". RED check at HEAD: `grep -rq "DeltaAdded" src/ui/palette.rs`
      → exit **1**.
- [ ] 3.2 GREEN: Add `DeltaAdded`, `DeltaModified`, `DeltaRemoved` to `Role` after `TaskLabel`,
      and answer them in `style` with `Green`, `Yellow`, and `LightRed`, no modifier. The
      exhaustive `match` makes this a compile error until done.
- [ ] 3.3 GREEN: Extend the modifier-table and colour-table tests to the three new rows, and
      update the modifier scenario's count from eleven to **fourteen** uncoloured-modifier roles
      per specs/view-palette.
- [ ] 3.4 CHECK: `/bin/sh scripts/gates/palette.sh` → exit 0, with the colour literals confined
      to `src/ui/palette.rs`'s own tests. Negative control run at planning time: appending
      `fn _p() { let _ = ratatui::style::Color::Red; }` to `src/ui/list.rs` made it exit **1**,
      and removing the plant returned it to exit **0**.
- [ ] 3.5 Run the group tests — `cargo test ui::palette::` green, no regressions.

## 4. `Face::delta` and `style_for` step 10

<!-- kind: behavior -->

Depends on 2 (for `DeltaOp`) and 3 (for the roles).

- [ ] 4.1 RED: Write failing tests `every_segment_lines_returns_carries_no_delta` in `ui::markdown` and an extension of
      `a plain face is the default style` in `ui::view`, from the same-named specs/markdown-render
      and specs/view-palette scenarios. Both name widths 58 and 78 per `DETAILWIDTHS`.
- [ ] 4.2 GREEN: Add `delta: Option<crate::specs::DeltaOp>` to `Face`, keeping its `Default`
      derive, and name the new field at `src/ui/tasks.rs`'s `heading_line` — the crate's one
      site that spells every field out (`grep -n "heading_line" src/ui/tasks.rs` → `286`).
- [ ] 4.3 GREEN: Add step 10 to `ui::view::style_for`, patching `DeltaAdded`/`DeltaModified`/
      `DeltaRemoved` last, per specs/view-palette → "`ui::view` takes every style it applies
      from the palette".
- [ ] 4.4 REFACTOR: Clean up while green, or state that none was needed.
- [ ] 4.5 Run the group tests — `cargo test ui::markdown:: ui::view::` green.

## 5. `ArtifactSection::operation` and its attribution walk

<!-- kind: behavior -->

Depends on 2 (for `DeltaOp`).

- [ ] 5.1 RED: Write failing tests `requirements_are_attributed_to_the_operation_heading_above_them`, `a_requirement_above_every_operation_heading_carries_none`,
      `a_main_specs_requirements_are_entirely_unbadged`, `only_a_level_3_requirement_heading_is_attributed`, `a_non_spec_artifact_is_attributed_nothing`,
      `a_delta_specs_requirement_sections_carry_their_operation_and_nothing_else_does`, and `a_tracked_tasks_tabs_sections_carry_progress_and_no_operation` in `ui::app`, from
      the same-named scenarios in specs/spec-delta-badges and specs/artifact-folds.
- [ ] 5.2 GREEN: Add `operation: Option<crate::specs::DeltaOp>` to `ArtifactSection` and fill it
      in `sync_detail` by one forward walk, beside where `progress` is computed. Reuse the
      existing level-3 `Requirement:` predicate rather than writing it twice — per design.md →
      Decision 4.
- [ ] 5.3 GREEN: Answer the new field at all **75** construction sites counted above
      (`app.rs` 28, `detail.rs` 19, `driver.rs` 14, `view.rs` 14) — none uses a rest pattern and
      `NODEFAULT-UI` forbids one, so every site must name it. The compiler enumerates them; `NODEFAULT-UI` is why no default is
      added to shortcut this.
- [ ] 5.4 CHECK: `/bin/sh scripts/gates/nodefault-ui.sh` with `ArtifactSection`'s own `SCAN_MIN`
      as the Makefile passes it → exit 0, confirming the type still carries no `Default`.
- [ ] 5.5 Run the group tests — `cargo test ui::app::` green, `cargo test` green overall.

## 6. The badge on a section header row

<!-- kind: behavior -->

Depends on 4 (for `Face::delta`) and 5 (for `operation`).

- [ ] 6.1 RED: Write failing tests `the_three_operations_draw_three_different_markers`,
      `an_unbadged_header_row_is_unchanged_in_every_column`, `a_removed_requirements_heading_is_struck_and_its_body_is_not`,
      `the_label_truncates_before_the_badge_is_dropped`, `the_badge_is_dropped_whole_at_a_width_that_cannot_hold_it`, and
      `a_badged_header_row_is_still_addressed_by_its_own_section_index` in `ui::detail`, from the same-named specs/artifact-folds
      scenarios. Every one must name both `58` and `78` or `DETAILWIDTHS` fails.
- [ ] 6.2 GREEN: Emit the badge in `ui::detail::header` as `<indent><glyph> <badge><label>`,
      two columns, with the badge in the survives-truncation prefix and dropped whole below the
      width that holds prefix plus one column of label — per design.md → Decision 5.
- [ ] 6.3 GREEN: Split a badged header row into three segments and set
      `Face { delta: Some(op) }` on the badge and `Face { strikethrough: true }` on a
      `Removed` row's label. An unbadged row keeps its single segment.
- [ ] 6.4 CHECK: `/bin/sh scripts/gates/detailwidths.sh` and `/bin/sh scripts/gates/widths.sh`
      → exit 0. At HEAD `widths.sh` reports `all 149 view tests name both 60 and 120`; the
      count rises and must not regress to a test naming one width.
- [ ] 6.5 Run the group tests — `cargo test ui::detail::` green, no regressions.

## 7. Clause keywords on the markdown path

<!-- kind: behavior -->

- [ ] 7.1 RED: Write failing tests `a_scenarios_three_clauses_are_coloured_by_position`, `and_inherits_the_clause_above_it_and_resets_at_a_heading`, and
      `only_a_run_opening_a_list_item_is_a_keyword` in `ui::markdown`, from the same-named
      specs/markdown-render scenarios.
- [ ] 7.2 GREEN: Set `Face::label` on a `Strong` run that is the first inline of a list item and
      that `specs::clause_of` accepts, holding the current position and resetting it at every
      heading. Leave `strong` set — the colour is added beside the author's bold.
- [ ] 7.3 GREEN: Extend the existing `The markdown path sets neither new face field` test to
      assert `delta: None` too, keeping its document free of any `- **WHEN**` bullet so the
      assertion survives unweakened.
- [ ] 7.4 CHECK: `/bin/sh scripts/gates/mdseam.sh` and `/bin/sh scripts/gates/mdwidths.sh`
      → exit 0 (at HEAD:
      `MDSEAM OK: 26 files searched (>= 25), pulldown_cmark only in src/ui/markdown.rs`), and
      `grep -n "crate::tasks::" src/ui/markdown.rs` finds no function call — only the
      `LabelRole` type — and `grep -n "crate::specs::" src/ui/markdown.rs` prints only lines
      naming `clause_of`. Both are `grep -n`, not `grep -c`: a count names nothing, and the
      check's whole content is *which* symbols appear.
- [ ] 7.5 Run the group tests — `cargo test ui::markdown::` green.

## 8. The badge reaches the buffer

<!-- kind: behavior -->

Depends on 6. This is the outermost evidence this change has — see design.md → Test Strategy.

- [ ] 8.1 RED: Write failing tests `a_selected_badged_header_keeps_its_badge_colour` and
      `a_badged_header_rows_colours_survive_the_rows_own_role` in `ui::view`, rendering into `TestBackend` at 60 and 120
      columns, from the same-named specs/artifact-folds and specs/view-palette scenarios.
- [ ] 8.2 GREEN: Confirm no `ui::view` change is needed beyond step 10 from 4.3 — the existing
      loop already patches the row's kind role over each segment's `style_for`. If a change is
      needed, that is a finding: record it, because design.md → Decision 8 claims it is not.
- [ ] 8.3 CHECK: Assert the badge cell's foreground against `palette::style(Role::DeltaAdded)`
      and never against a `Color` literal; `/bin/sh scripts/gates/palette.sh` → exit 0 proves it.
- [ ] 8.4 Run the group tests — `cargo test ui::view::` green.

## 9. Change Review

<!-- kind: operational -->

- [ ] 9.1 CHECK: Dispatch an independent reviewer against proposal, specs, design, and tasks.
- [ ] 9.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING, re-run affected tests.
- [ ] 9.3 VERIFY: Confirm no blocking or unowned finding remains.

## 10. Documentation

<!-- kind: operational -->

- [ ] 10.1 CHECK: `cargo test --test doc_contract module_map_matches_lib_rs` → must be RED once
      `pub mod specs;` exists and `SPEC.md`'s Module map has no row for it. That test reads
      `src/lib.rs`'s `pub mod` set against the table at `SPEC.md:81`.
- [ ] 10.2 CHANGE: Add the `specs` row to `SPEC.md`'s Module map (audience: anyone tracing where
      a classifier lives) — "Recognise a delta spec's operation headings and a scenario clause's
      keyword". It replaces nothing; it is the row a new module owes that table.
- [ ] 10.3 CHANGE: Update `AGENTS.md` → Architecture rules (audience: agents editing this crate)
      to name `src/specs.rs` beside `src/tasks.rs` as pure classification outside `src/ui/`, and
      record that `ui::markdown` may call `specs::clause_of` and no other function of that
      module. This corrects the current text's implication that no pure view file calls out.
- [ ] 10.4 VERIFY: `cargo test --test doc_contract` green.

## 11. Lint & Verify

<!-- kind: operational -->

- [ ] 11.1 CHECK: Confirm `ls scripts/gates/ | wc -l` still reports **31** — this change adds no
      gate, so a changed count means one was added without the spec sentence
      `tests/ci_workflow.rs` binds.
- [ ] 11.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 11.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings.
- [ ] 11.4 VERIFY: `make gates` — exit 0, with `NOIO-VIEW` still reporting **10** pure files and
      `COLWIDTH` still **nine** pure view files. Both counts must be unchanged: `src/specs.rs`
      is outside `src/ui/` precisely so they do not move (design.md → Decision 1).
- [ ] 11.5 VERIFY: `cargo test --all-features` — green, contract tier included.
- [ ] 11.6 VERIFY: `make check` as the single gate; if it fails, name the failing sub-command.
- [ ] 11.7 VERIFY: `openspec validate spec-emphasis --strict` — valid.
