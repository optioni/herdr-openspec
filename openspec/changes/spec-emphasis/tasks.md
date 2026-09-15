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

- [x] 1.1 RED: Write `the_table_is_reachable_on_its_own_and_label_of_agrees_with_it` and `an_unrecognised_run_is_none_to_the_table_and_other_to_the_label` in
      `src/tasks.rs`'s inline `mod tests`, from the two same-named scenarios in
      specs/task-labels. RED check at HEAD: `grep -rq "pub fn role_of" src/tasks.rs` → exit
      **1**, and `an_unrecognised_run_is_none_to_the_table_and_other_to_the_label` does not compile against the current signature.
- [x] 1.2 GREEN: Make `role_of` `pub` and change its return to `Option<LabelRole>`, moving its
      `_ => LabelRole::Other` arm to `label_of`'s call site as `.unwrap_or(LabelRole::Other)`.
      The existing `tasks::tests::label_*` tests are the unchanged-behaviour anchor and must
      stay green and unedited.
- [x] 1.3 REFACTOR: Clean up while green, or state that none was needed.
- [x] 1.4 VERIFY: `cargo test tasks::` green with the pre-existing `label_*` tests unedited, and
      both new tests passing — the equivalence design.md → Contracts claims is now asserted by a
      test rather than by a signature grep.

## 2. `crate::specs` — the delta and clause classifiers

<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests in `src/specs.rs` for `each_of_the_three_operation_headings_classifies_to_its_own_variant`,
      `internal_whitespace_is_tolerated_and_nothing_else_is`, `only_a_level_2_heading_carries_an_operation`, `a_renamed_operation_and_a_main_specs_heading_both_decline`,
      `the_recognition_is_total_over_degenerate_input`, `the_three_measured_keywords_classify_as_specified`, `the_wider_testing_vocabulary_classifies_through_the_same_table`, and
      `a_run_outside_the_table_is_not_a_clause`, from **eight** of the ten same-named scenarios in specs/spec-delta-badges.
      RED check at HEAD: `test -f src/specs.rs` → exit **1**.
      **Corrected during implementation, twice.** An earlier draft of this line listed a ninth
      name, `the_classification_reads_nothing_outside_its_argument`, and called the set "the nine
      same-named scenarios". That scenario (`specs/spec-delta-badges/spec.md:181`) delegates
      itself in its own text — "the check lives in `tests/doc_contract.rs` and **not** inside
      `src/specs.rs`, because a check written inside the file it sweeps contains its own needles
      and can never pass" — and `specs/doc-conformance/spec.md:39` states it from the owning
      side as "The production slice of `src/specs.rs` carries no I/O or schema name", which is
      the sentence group 8's test name transcribes. Writing it here was impossible, not merely
      redundant. Separately, the draft omitted a scenario that *is* this group's: see 2.1a.
      Both defects were in the reviewed package; the implementer declined the ninth name on the
      scenario's own authority and the gate found the omission.
- [x] 2.1a RED: Write `the_clause_recognition_is_total_over_degenerate_input` in `src/specs.rs`,
      from `specs/spec-delta-badges/spec.md:200` — `clause_of` on the empty string, on `"   "`,
      on a 10000-character run of `A`, on `"日本語"`, on `"AND 日本語"`, and on a string whose
      first character is a multi-byte grapheme; no call panics, every call returns `None`, `AND`
      being matched whole so `"AND 日本語"` is not it. The planning review added this scenario as
      a NIT repair — "`clause_of` was required to be total but had no degenerate-input scenario,
      while its sibling `operation_of_heading` had one — two functions in one capability held to
      different standards for the same property" — and did not add it to 2.1's list, so the
      repair landed in the spec and in no task. It mirrors
      `the_recognition_is_total_over_degenerate_input`, which is what the scenario's closing
      clause asks for: one standard for one property across both siblings.
- [x] 2.2 GREEN: Add `pub mod specs;` to `src/lib.rs` and implement `DeltaOp`,
      `operation_of_heading(level, label)`, `Clause`, and `clause_of(run)`, with `clause_of`
      calling `crate::tasks::role_of` and never restating the token table.
      RED check at HEAD: `grep -rq "DeltaOp" src/` → exit **1**.
      **Sequencing correction.** `pub mod specs;` has to land in the **RED** commit, not this
      one: without it `src/specs.rs` is a file rustc never compiles, and `cargo test specs::`
      answers `running 0 tests` and **exits 0** — the precise false pass this file's own
      Planning-time evidence warns against. With the declaration present, the RED is honest and
      names its cause: `error[E0432]: unresolved imports super::Clause, super::DeltaOp,
      super::clause_of, super::operation_of_heading`.
- [x] 2.3 REFACTOR: Clean up while green, or state that none was needed.
- [x] 2.4 CHECK: The token table must not be copied. Scope the sweep to the **production
      slice** — everything above `mod tests` — exactly as `src/tasks.rs`'s own
      `the_classification_reads_nothing_outside_its_argument` already does:
      `awk '/^mod tests/{exit} {print}' src/specs.rs | grep -nE '"(RED|GREEN|VERIFY|CHARACTERIZE|ARRANGE|ACT|ASSERT)"'`
      must print nothing, and `grep -q "crate::tasks::role_of" src/specs.rs` must exit 0.
      Unscoped, this check is guaranteed to fail on a correct implementation: 2.1's
      `the_wider_testing_vocabulary_classifies_through_the_same_table` is required by specs/spec-delta-badges to call `clause_of` on
      `GIVEN`, `ARRANGE`, `ACT`, `ASSERT`, and `RED`, so those literals must appear in the test
      module. Measured on the established analogue:
      `grep -cE '"(RED|GREEN|VERIFY|CHARACTERIZE|ARRANGE|ACT|ASSERT)"' src/tasks.rs` → **6**.
- [x] 2.5 Run the group tests — `cargo test specs:: && cargo test tasks::`, both green, and
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
- [ ] 3.3 GREEN: Extend the modifier-table and colour-table tests to the three new rows, update
      the modifier scenario's count from eleven to **fourteen**, and add the three variant names
      to `the_enums_membership_is_exactly_this_list` — both its transcribed `vec!` of names
      (`src/ui/palette.rs:798-825`) and its among-them loop (`:830-838`). Only `variant()`'s
      match is compile-forced; the `vec!` fails at runtime and is easy to miss.
- [ ] 3.4 CHECK: `/bin/sh scripts/gates/palette.sh` → exit 0, with the colour literals confined
      to `src/ui/palette.rs`'s own tests. Negative control run at planning time: appending
      `fn _p() { let _ = ratatui::style::Color::Red; }` to `src/ui/list.rs` made it exit **1**,
      and removing the plant returned it to exit **0**.
- [ ] 3.5 Run the group tests — `cargo test ui::palette::` green, no regressions; state
      whether a refactor was needed.

## 4. `Face::delta` and `style_for` step 10

<!-- kind: behavior -->

Depends on 2 (for `DeltaOp`) and 3 (for the roles).

- [ ] 4.1 RED: Write failing tests `every_segment_lines_returns_carries_no_delta` in `ui::markdown` and an extension of
      `a plain face is the default style` in `ui::view`, from the same-named specs/markdown-render
      and specs/view-palette scenarios. Each answers to its **own** width gate:
      `every_segment_lines_returns_carries_no_delta` is in `src/ui/markdown.rs` and names 58 and
      78 (`MDWIDTHS`); the `ui::view` extension keeps the 60 and 120 it already has (`WIDTHS`).
      `DETAILWIDTHS` sweeps only `src/ui/detail.rs` and neither of these — rewriting the
      `ui::view` test to 58/78 would redden `make gates`.
- [ ] 4.1a RED: In the same step assert `style_for` over all three operations —
      `the_three_delta_roles_carry_their_colour_and_no_modifier`'s sibling in `ui::view`, from
      specs/view-palette → "`style_for` maps each `DeltaOp` to its own role". Without it group 4
      has no test that step 10 exists at all, and a slip mapping `Modified` to `DeltaAdded`
      passes every other check in this plan.
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
- [ ] 5.5 Run the group tests — `cargo test ui::app::` green, `cargo test` green overall;
      state whether a refactor was needed.

## 6. The badge on a section header row

<!-- kind: behavior -->

Depends on 4 (for `Face::delta`) and 5 (for `operation`).

- [ ] 6.1 RED: Write failing tests `the_three_operations_draw_three_different_markers`,
      `an_unbadged_header_row_is_unchanged_in_every_column`,
      `a_removed_requirements_heading_is_struck_and_its_body_is_not`,
      `the_label_truncates_before_the_badge_is_dropped`,
      `the_badge_is_dropped_whole_at_a_width_that_cannot_hold_it`, and
      `a_badged_header_row_is_still_addressed_by_its_own_section_index` in `ui::detail`, from the
      same-named specs/artifact-folds scenarios. Every one must name both `58` and `78`
      (`DETAILWIDTHS`).
- [ ] 6.1a RED: In the same step write the two **render** tests in `ui::view` —
      `a_selected_badged_header_keeps_its_badge_colour` and
      `a_badged_header_rows_colours_survive_the_rows_own_role` — naming `60` and `120`
      (`WIDTHS`, which sweeps `src/ui/view.rs` and requires those two widths, not 58/78). They
      belong in this group's RED because the badge does not exist until 6.2/6.3: run after them
      they could not fail, which is a characterization test wearing a `behavior` marker.
- [ ] 6.2 GREEN: Emit the badge in `ui::detail::header` as `<indent><glyph> <badge><label>`,
      two columns, with the badge in the survives-truncation prefix and dropped whole below the
      width that holds prefix plus one column of label — per design.md → Decision 5.
- [ ] 6.3 GREEN: Split a badged header row into three segments and set
      `Face { delta: Some(op) }` on the badge and `Face { strikethrough: true }` on a
      `Removed` row's label. An unbadged row keeps its single segment.
- [ ] 6.4 REFACTOR: `header` goes from a one-segment row to three with a drop-whole prefix
      rule; clean that up while green, or state that none was needed.
- [ ] 6.5 CHECK: `/bin/sh scripts/gates/detailwidths.sh` and `/bin/sh scripts/gates/widths.sh`
      → exit 0. At HEAD `widths.sh` reports `all 149 view tests name both 60 and 120`; the
      count rises and must not regress to a test naming one width.
- [ ] 6.6 Run the group tests — `cargo test ui::detail:: ui::view::` green, no regressions.

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
      `grep -n "crate::tasks::" src/ui/markdown.rs` finds no function **call**. It does return
      four hits the implementer should expect: `LabelRole` as a field type, two doc-comment
      mentions, and `crate::tasks::Progress` at `:2400` inside the test module — none is a call,
      and the production slice ends at `:1270`. And `grep -n "crate::specs::" src/ui/markdown.rs` prints only lines
      naming `clause_of`. Both are `grep -n`, not `grep -c`: a count names nothing, and the
      check's whole content is *which* symbols appear.
- [ ] 7.5 Run the group tests — `cargo test ui::markdown::` green; state whether a refactor
      was needed.

## 8. The new module's purity, checked inside `cargo test`

<!-- kind: behavior -->

Depends on 2. Per specs/doc-conformance → "A non-view pure module's freedom from I/O is checked
inside `cargo test`", and design.md → Decision 1: outside `src/ui/`, `src/specs.rs` is swept by
no gate, so the property Decision 3 rests on has no check without this group.

- [ ] 8.1 RED: Write the eleventh claim in `tests/doc_contract.rs` —
      `the_production_slice_of_src_specs_rs_carries_no_io_or_schema_name` — reading the slice
      above `src/specs.rs`'s first line-anchored `#[cfg(test)]` through the existing helper
      (`production_slice_cuts_before_cfg_test`) and failing on any of `std::fs`, `std::io`,
      `std::env`, `std::process`, `std::net`, `File::`, `read_to_string`, `Command`, `schema::`,
      `Schema`, `config.yaml`, `.openspec.yaml`. RED check at HEAD: `test -f src/specs.rs` →
      exit **1**, so the claim cannot yet read its subject.
- [ ] 8.2 GREEN: Assert the slice is non-empty before searching it, so the claim cannot pass
      vacuously against a file it failed to read or cut at the wrong place.
- [ ] 8.3 CHECK: Run the negative control and record both halves. Insert `use std::fs;` above
      `src/specs.rs`'s `#[cfg(test)]` line → `cargo test --test doc_contract` must FAIL naming
      the needle and the line; remove the plant → must pass. A green-at-HEAD check without this
      is an unfalsifiable guard, which is worse than no guard.
- [ ] 8.4 CHECK: Confirm no gate moved — `ls scripts/gates/ | wc -l` still **31**, and
      `make gates` still reports `NOIO-VIEW OK: 10 pure files`. This claim exists in the test
      tier precisely so neither count moves.
- [ ] 8.5 Run the group tests — `cargo test --test doc_contract` green; state whether a refactor
      was needed.

## 9. Change Review

<!-- kind: operational -->

- [ ] 9.1 CHECK: Dispatch an independent reviewer against proposal, specs, design, and tasks.
- [ ] 9.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING, re-run affected tests.
- [ ] 9.3 VERIFY: Confirm no blocking or unowned finding remains.

## 10. Documentation

<!-- kind: operational -->

- [ ] 10.1 CHECK: `cargo test --test doc_contract` → must be RED on **two** counts once
      `pub mod specs;` exists: `module_map_matches_lib_rs` (the table at `SPEC.md:81`) and
      `tested_modules_names_every_module` (the `### Unit-tested modules` list at `SPEC.md:1098`).
      Both read `src/lib.rs`'s `pub mod` set; naming only the first understates what is red.
- [ ] 10.2 CHANGE: Add the `specs` row to `SPEC.md`'s Module map (audience: anyone tracing where
      a classifier lives) — "Recognise a delta spec's operation headings and a scenario clause's
      keyword". It replaces nothing; it is the row a new module owes that table.
- [ ] 10.3 CHANGE: Add `specs::operation_of_heading` and `specs::clause_of` to `SPEC.md`'s
      `### Unit-tested modules` list (`SPEC.md:1098`) — delta-operation headings and scenario
      clause keywords, classified from `&str` with no filesystem edge. The check requires a
      bounded `specs::` token, so the module name alone is not enough.
- [ ] 10.4 CHANGE: Update `AGENTS.md` → Architecture rules (audience: agents editing this crate)
      to name `src/specs.rs` beside `src/tasks.rs` as pure classification outside `src/ui/`, and
      record that `ui::markdown` may call `specs::clause_of` and no other function of that
      module. This corrects the current text's implication that no pure view file calls out.
- [ ] 10.5 CHANGE: Update `AGENTS.md`'s contract-tier sentence from "ten further claims" to
      **eleven**, naming the `src/specs.rs` purity claim. The count is prose and nothing binds
      it, which is exactly how `quality-gates`' script count drifted three behind unnoticed.
- [ ] 10.6 VERIFY: `cargo test --test doc_contract` green.

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
- [ ] 11.5a VERIFY: `make coverage` — both floors hold. `src/specs.rs` is a new **production**
      file and the production-slice floor is **96%** (`scripts/coverage-prod.py:84`), well above
      the 80% total, so every arm of `operation_of_heading` and `clause_of` needs a test. The
      eleven scenarios in specs/spec-delta-badges supply them; this task is the forewarning, so
      a shortfall surfaces here rather than as an unexplained red at `make check`.
- [ ] 11.6 VERIFY: `make check` as the single gate; if it fails, name the failing sub-command.
- [ ] 11.7 VERIFY: `openspec validate spec-emphasis --strict` — valid.
