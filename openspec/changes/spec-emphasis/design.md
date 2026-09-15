## Context

The detail region already renders a delta spec as a tree of foldable sections: `heading-sections`
splits a spec-shaped file at its ATX headings, and `artifact-folds` draws one header row per
section with a fold glyph, an indent, and — since `tasks-emphasis` — a right-aligned progress
cell on a tracked-tasks group. What it does not do is say which of the three delta operations a
requirement belongs to. OpenSpec writes that into the text as a level-2 heading and the pane
spends it on nothing: `## ADDED Requirements` scrolls off the top and an added requirement and a
removed one then look identical.

Underneath, `tasks-emphasis` landed a lifecycle vocabulary — `crate::tasks::LabelRole`, four
positions over a token table — and wired it to the **checklist** path only. That table already
classifies `WHEN` as `Change` and `THEN` as `Confirm`, which are exactly the tokens a scenario
is built from. The vocabulary exists; the second consumer does not.

This change adds both, and the constraint that shapes it is that neither may be a second
implementation of something the crate already decides once.

## Goals / Non-Goals

**Goals:**

- A requirement's delta operation is visible on its own header row, as a coloured `+`/`~`/`-`
  marker, without scrolling back to the heading above it.
- A `REMOVED` requirement's heading reads as deleted; its body stays readable.
- A scenario's `WHEN`/`THEN`/`AND` are coloured by the lifecycle position they name, through
  the table `tasks-emphasis` already wrote.
- Every row this change does not badge is byte-identical to the row it was.

**Non-Goals:**

- A computed diff. The badge comes from the heading OpenSpec already writes and from nothing
  else — no body is compared against an archived original.
- Renaming the four `Task*` palette roles, which become misnomers here. See Decision 9.
- De-emphasising anything. The proposal's first draft did; the review reversed it.
- Any change to how a non-spec artifact renders.

## Boundaries

| Piece | Module | Pattern it follows |
|---|---|---|
| `DeltaOp`, `operation_of_heading`, `clause_of` | **`src/specs.rs`** (new) | `crate::tasks::label_of` — a pure total classifier over borrowed text, outside `src/ui/` |
| `role_of` (the token table, exposed) | `src/tasks.rs` | extraction of an existing private match, no behaviour change |
| `ArtifactSection::operation` | `src/ui/app.rs` | `ArtifactSection::progress`, added by `tasks-emphasis` and filled in `sync_detail` |
| badge segment on a header row | `src/ui/detail.rs` | the progress cell's drop-whole discipline, same function |
| `Face::delta`, clause labelling | `src/ui/markdown.rs` | `Face::label`, added by `tasks-emphasis` |
| `Role::Delta*`, `style_for` step 10 | `src/ui/palette.rs`, `src/ui/view.rs` | the four `Task*` roles and `style_for` step 9 |

`src/specs.rs` is a **new non-view module**, and that placement is the change's one structural
decision (Decision 1). No process spawn is added anywhere: this change names no
`process::Command`, no `Stdio`, and no `HerdrCli`/`OpenspecCli` handle, and `src/cli.rs` is
untouched. No view gains I/O — `src/specs.rs` is not under `src/ui/` and reaches no I/O API
itself, so `NOIO-VIEW`'s ten-file `PURE` list and `COLWIDTH`'s nine are **unchanged**, which is
the whole reason the module sits where it does.

The `Change` type is **not** altered. `changes::from_files` and `changes::from_cli` produce the
same `Change` values they did, and nothing this change adds is carried on one; the badge is
derived at render time from a section list, downstream of both paths, so the two cannot
disagree about it.

## Contracts

Every interface this change touches is internal to the crate — there is no wire format, no
persisted schema, and no consumer outside `src/`.

- **`crate::tasks::role_of`** — an **existing private** `fn role_of(run: &str) -> LabelRole`
  (`src/tasks.rs:205`, found by `grep -n "fn role_of" src/tasks.rs`) that becomes `pub` and
  returns `Option<LabelRole>`; its `_ => Other` arm moves to `label_of`'s call site as
  `.unwrap_or(LabelRole::Other)`. `label_of`'s observable behaviour is unchanged, asserted by a
  scenario that calls both over all thirteen tokens and compares. Consumers:
  `crate::specs::clause_of` (new) and `label_of` itself. This is a **widening of an existing
  seam**, not a new one — the table was already reached through exactly one function.
- **`crate::specs`** — a new module, wholly additive. Consumers: `ui::app::sync_detail` and
  `ui::markdown::lines`.
- **`ArtifactSection`** — gains a fifth field. **Breaking at compile time** for every literal
  construction site, which is the point: `NODEFAULT-UI` scans this type set, so the field has no
  default and every site must answer it. Consumers: `ui::app`, and the `ui::detail` tests that
  build sections.
- **`markdown::Face`** — gains a tenth field. `Face` keeps deriving `Default`, so only sites
  that spell every field out are forced; `src/ui/tasks.rs`'s `heading_line` is the one such site.
- **`palette::Role`** — gains three variants. Breaking at compile time for the exhaustive
  `match` in `palette::style` and for the test that iterates the enum, both of which are meant
  to fail until the tables answer the new roles.

Error surface: none of the new functions can fail. Both return `Option`, and `None` is the
ordinary answer for "not a delta heading" / "not a clause keyword", never an error. There is no
pagination and no streaming.

## Persistence and Rollout

- **Migration:** none. Nothing is persisted.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none beyond what exists. `artifact-content`'s cache is keyed on
  `(change directory, tab)`, and `operation` is derived inside `sync_detail` on the same key
  change as `progress`, so it is filled when that cache fills and never per frame.
- **Index rebuild:** none.
- **Authorization:** none. The pane is read-only and this change adds no write; the plugin's
  own writes stay exactly `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`.
- **Observability:** none. No logging is added; a classification that declines renders an
  unbadged row, which is the observable.
- **Deployment:** none beyond `make build`. No manifest, config, or keybinding change, so
  `herdr plugin link .` needs no re-run and `tests/manifest.rs` is untouched.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem (`openspec/` tree) | replaced — `Dashboard::sync_detail` takes an injected `&dyn Fn(&Path) -> Result<String, String>`; tests pass a closure over in-memory strings | replaced, same injection; `crate::specs`' own tests take `&str` and touch no path |
| `openspec` binary (`OpenspecCli`) | replaced — a stub trait impl; this change never reaches it | not reached |
| Herdr socket (`HerdrCli`, agent poller, launcher) | replaced — stub trait impls, unchanged from today | not reached |
| Terminal (crossterm raw mode, alternate screen) | **never real** — `ratatui::backend::TestBackend` only; `src/ui/terminal.rs` is untouched and no test may enter raw mode, because `cargo test` spawns this binary | not reached |
| Clock / `Instant::now()` | not reached — this change reads no clock and adds no timing | not reached |
| Process spawn | **not reached anywhere**; asserted by `NOSPAWN-GREP` over the whole tree | not reached |
| `pulldown_cmark` | real, inside `src/ui/markdown.rs` only | real, same file |
| `ratatui` buffer | real `TestBackend` at 60 and 120 columns | real `Buffer` where a view test asserts cells; `crate::specs` and `crate::tasks` tests use none |
| `crate::tasks::role_of` | real — `clause_of` calls the production table, never a stub, which is the point of Decision 3 | real |

No task may invent a boundary this table does not name. In particular: **no test in this change
may read a real file**, and no test may construct a `Schema` for `crate::specs`, whose
signatures admit none.

## Test Strategy

Tiers, fastest first:

| Tier | What belongs in it | Command |
|---|---|---|
| Unit — pure parse | `crate::specs`, `crate::tasks` — `&str` in, plain data out | `cargo test specs::` / `cargo test tasks::` |
| Unit — pure view | `ui::markdown`, `ui::detail`, `ui::list`, `ui::palette` — state in, lines or `Style` out, at the mandated widths | `cargo test ui::markdown::` etc. |
| View render | a whole frame into `TestBackend` at 60 and 120 columns, asserting cells against `palette::style(role)` | `cargo test ui::view::` |
| Contract | `tests/*.rs` — doc claims bound to their computable second site | `cargo test --test doc_contract` |
| Gates | tree-wide greps with positive controls | `make gates` |

This change takes **no outer-loop acceptance test** beyond the view-render tier, and the reason
is the standing one for this crate: the pane's only real collaborators are the terminal, which
no test may touch, and the filesystem, which is injected at `sync_detail`. A "full stack" test
would therefore be a `TestBackend` render with a closure reader — which is exactly what the view
tier already is. The `ui::view` rows below are that tier and are the outermost evidence this
change has.

Width discipline: every `#[test]` in `src/ui/detail.rs` must name both `58` and `78`
(`DETAILWIDTHS`, no exemption list), and the same in `src/ui/markdown.rs`. `crate::specs`'
tests are width-free and therefore belong in `src/specs.rs`, which no `*WIDTHS` gate sweeps —
that is why the classifier does not live under `src/ui/`.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| Each of the three operation headings classifies to its own variant | `specs::tests::each_of_the_three_operation_headings_classifies_to_its_own_variant` | Unit — pure parse | none (all real, `&str` only) | `cargo test specs::` |
| Internal whitespace is tolerated and nothing else is | `specs::tests::internal_whitespace_is_tolerated_and_nothing_else_is` | Unit — pure parse | none | `cargo test specs::` |
| Only a level-2 heading carries an operation | `specs::tests::only_a_level_2_heading_carries_an_operation` | Unit — pure parse | none | `cargo test specs::` |
| A renamed operation and a main spec's heading both decline | `specs::tests::a_renamed_operation_and_a_main_specs_heading_both_decline` | Unit — pure parse | none | `cargo test specs::` |
| The recognition is total over degenerate input | `specs::tests::the_recognition_is_total_over_degenerate_input` | Unit — pure parse | none | `cargo test specs::` |
| Requirements are attributed to the operation heading above them | `ui::app::tests::requirements_are_attributed_to_the_operation_heading_above_them` | Unit — pure view | reader replaced (closure) | `cargo test ui::app::` |
| A requirement above every operation heading carries none | `ui::app::tests::a_requirement_above_every_operation_heading_carries_none` | Unit — pure view | reader replaced | `cargo test ui::app::` |
| A main spec's requirements are entirely unbadged | `ui::app::tests::a_main_specs_requirements_are_entirely_unbadged` | Unit — pure view | reader replaced | `cargo test ui::app::` |
| Only a level-3 `Requirement:` heading is attributed | `ui::app::tests::only_a_level_3_requirement_heading_is_attributed` | Unit — pure view | reader replaced | `cargo test ui::app::` |
| A non-spec artifact is attributed nothing | `ui::app::tests::a_non_spec_artifact_is_attributed_nothing` | Unit — pure view | reader replaced | `cargo test ui::app::` |
| The three measured keywords classify as specified | `specs::tests::the_three_measured_keywords_classify_as_specified` | Unit — pure parse | `tasks::role_of` real | `cargo test specs::` |
| The wider testing vocabulary classifies through the same table | `specs::tests::the_wider_testing_vocabulary_classifies_through_the_same_table` | Unit — pure parse | `tasks::role_of` real | `cargo test specs::` |
| A run outside the table is not a clause | `specs::tests::a_run_outside_the_table_is_not_a_clause` | Unit — pure parse | `tasks::role_of` real | `cargo test specs::` |
| The classification reads nothing outside its argument | `specs::tests::the_classification_reads_nothing_outside_its_argument` + `NOSCHEMA` leg of `make gates` | Unit + Gates | none | `cargo test specs:: && make gates` |
| Every token in the table classifies to its own role | existing `tasks::tests::every_token_in_the_table_classifies_to_its_own_role`, unchanged | Unit — pure parse | none | `cargo test tasks::` |
| An unrecognised run is a generic label, not a miss | existing `tasks::tests::an_unrecognised_run_is_a_generic_label_not_a_miss`, unchanged | Unit — pure parse | none | `cargo test tasks::` |
| Matching is case-sensitive and whole-run | existing `tasks::tests::matching_is_case_sensitive_and_whole_run`, unchanged | Unit — pure parse | none | `cargo test tasks::` |
| The classification reads nothing outside its argument | existing `tasks::tests::the_classification_the_classification_reads_nothing_outside_its_argument_outside_its_argument`, unchanged | Unit — pure parse | none | `cargo test tasks::` |
| The table is reachable on its own and `label_of` agrees with it | `tasks::tests::the_table_is_reachable_on_its_own_and_label_of_agrees_with_it` | Unit — pure parse | none | `cargo test tasks::` |
| An unrecognised run is `None` to the table and `Other` to the label | `tasks::tests::an_unrecognised_run_is_none_to_the_table_and_other_to_the_label` | Unit — pure parse | none | `cargo test tasks::` |
| The three spec files of a change become three labelled sections | existing `ui::app` test, re-run with the new field | Unit — pure view | reader replaced | `cargo test ui::app::` |
| A spec glob nests requirements under their capability | existing test, unchanged | Unit — pure view | reader replaced | `cargo test ui::app::` |
| A preamble becomes an unlabelled section | existing test, unchanged | Unit — pure view | reader replaced | `cargo test ui::app::` |
| A single-file artifact is one section and is not foldable | existing test, unchanged | Unit — pure view | reader replaced | `cargo test ui::app::` |
| An artifact with no resolved paths has no sections | existing test, unchanged | Unit — pure view | reader replaced | `cargo test ui::app::` |
| An unreadable file drops its section and keeps its siblings | existing test, unchanged | Unit — pure view | reader replaced (failing closure) | `cargo test ui::app::` |
| The label derivation is total over adversarial paths | existing test, unchanged | Unit — pure view | none | `cargo test ui::app::` |
| A delta spec's requirement sections carry their operation and nothing else does | `ui::app::tests::a_delta_specs_requirement_sections_carry_their_operation_and_nothing_else_does` | Unit — pure view | reader replaced | `cargo test ui::app::` |
| A tracked-tasks tab's sections carry progress and no operation | `ui::app::tests::a_tracked_tasks_tabs_sections_carry_progress_and_no_operation` | Unit — pure view | reader replaced | `cargo test ui::app::` |
| Folding one section shows its body and leaves its siblings shut | existing `ui::detail` test, unchanged | Unit — pure view | none | `cargo test ui::detail::` |
| A fold hides a whole subtree | existing test, unchanged | Unit — pure view | none | `cargo test ui::detail::` |
| The cursor's section header is the emphasised one | existing test, unchanged | Unit — pure view | none | `cargo test ui::detail::` |
| A narrow pane truncates the label and keeps the glyph | existing test, unchanged | Unit — pure view | none | `cargo test ui::detail::` |
| A tracked-tasks tab's group headers carry their own progress | existing test, unchanged | Unit — pure view | none | `cargo test ui::detail::` |
| Every other artifact's section headers carry no progress cell | existing test, unchanged | Unit — pure view | none | `cargo test ui::detail::` |
| The progress cell is dropped whole rather than truncated | existing test, unchanged | Unit — pure view | none | `cargo test ui::detail::` |
| A group holding no items still gets a header and a counted cell | existing test, unchanged | Unit — pure view | none | `cargo test ui::detail::` |
| The three operations draw three different markers | `ui::detail::tests::the_three_operations_draw_three_different_markers` | Unit — pure view | none | `cargo test ui::detail::` |
| An unbadged header row is unchanged in every column | `ui::detail::tests::an_unbadged_header_row_is_unchanged_in_every_column` | Unit — pure view | none | `cargo test ui::detail::` |
| A removed requirement's heading is struck and its body is not | `ui::detail::tests::a_removed_requirements_heading_is_struck_and_its_body_is_not` | Unit — pure view | none | `cargo test ui::detail::` |
| The label truncates before the badge is dropped | `ui::detail::tests::the_label_truncates_before_the_badge_is_dropped` | Unit — pure view | none | `cargo test ui::detail::` |
| The badge is dropped whole at a width that cannot hold it | `ui::detail::tests::the_badge_is_dropped_whole_at_a_width_that_cannot_hold_it` | Unit — pure view | none | `cargo test ui::detail::` |
| A selected badged header keeps its badge colour | `ui::view::tests::a_selected_badged_header_keeps_its_badge_colour` | View render | `TestBackend` real, reader replaced | `cargo test ui::view::` |
| A badged header row is still addressed by its own section index | `ui::detail::tests::a_badged_header_row_is_still_addressed_by_its_own_section_index` | Unit — pure view | none | `cargo test ui::detail::` |
| A paragraph is word-wrapped, differently at the two mandated widths | existing `ui::markdown` test, unchanged | Unit — pure view | `pulldown_cmark` real | `cargo test ui::markdown::` |
| An empty source and a zero width each produce no lines | existing test, unchanged | Unit — pure view | `pulldown_cmark` real | `cargo test ui::markdown::` |
| No line exceeds the width it was given | existing test, unchanged | Unit — pure view | `pulldown_cmark` real | `cargo test ui::markdown::` |
| A wide-character document wraps by columns at both mandated widths | existing test, unchanged | Unit — pure view | `pulldown_cmark` real | `cargo test ui::markdown::` |
| Rendering is total over arbitrary input | existing test, extended to assert no panic with clause input | Unit — pure view | `pulldown_cmark` real | `cargo test ui::markdown::` |
| The markdown path sets neither new face field | existing test, extended to assert `delta: None` | Unit — pure view | `pulldown_cmark` real | `cargo test ui::markdown::` |
| A scenario's three clauses are coloured by position | `ui::markdown::tests::a_scenarios_three_clauses_are_coloured_by_position` | Unit — pure view | `pulldown_cmark`, `specs::clause_of` real | `cargo test ui::markdown::` |
| `AND` inherits the clause above it and resets at a heading | `ui::markdown::tests::and_inherits_the_clause_above_it_and_resets_at_a_heading` | Unit — pure view | `pulldown_cmark`, `specs::clause_of` real | `cargo test ui::markdown::` |
| Only a run opening a list item is a keyword | `ui::markdown::tests::only_a_run_opening_a_list_item_is_a_keyword` | Unit — pure view | `pulldown_cmark` real | `cargo test ui::markdown::` |
| Every segment `lines` returns carries no delta | `ui::markdown::tests::every_segment_lines_returns_carries_no_delta` | Unit — pure view | `pulldown_cmark` real | `cargo test ui::markdown::` |
| The narrowed seam holds | `MDSEAM` + `NOCRATETASKS` legs of `make gates` | Gates | none | `make gates` |
| Each role's modifier set is exactly the table above | `ui::palette::tests::each_role_s_modifier_set_is_exactly_the_table_above`, extended by three rows | Unit — pure view | none | `cargo test ui::palette::` |
| A monochrome reading of the frame is unchanged | existing `ui::view` test, extended with a badged section | View render | `TestBackend` real | `cargo test ui::view::` |
| The five new roles leave every existing cell's modifier where it was | existing test, unchanged | View render | `TestBackend` real | `cargo test ui::view::` |
| The coloured set is exactly the table above | `ui::palette::tests::the_coloured_set_is_exactly_the_table_and_every_colour_is_a_named_ansi_index`, extended by three rows | Unit — pure view | none | `cargo test ui::palette::` |
| An out-of-range heading level does not panic | existing test, unchanged | Unit — pure view | none | `cargo test ui::palette::` |
| Every shared style is licensed, and the unshared roles stay unshared | `ui::palette::tests::every_shared_style_is_licensed_and_the_unshared_roles_stay_unshared`, extended | Unit — pure view | none | `cargo test ui::palette::` |
| A task label and a problem row are distinguishable in one frame | existing test, unchanged | View render | `TestBackend` real | `cargo test ui::view::` |
| Faces reach the buffer as coloured styles at both mandated widths | existing test, unchanged | View render | `TestBackend` real | `cargo test ui::view::` |
| A section header's role is selected by its kind, not by its face | existing test, unchanged | View render | `TestBackend` real | `cargo test ui::view::` |
| Heading foreground wins over a code span inside it | existing test, unchanged | Unit — pure view | none | `cargo test ui::view::` |
| A plain face is the default style | existing test, extended — `Face::plain()` now carries `delta: None` | Unit — pure view | none | `cargo test ui::view::` |
| The two new face fields compose in their stated positions | existing test, unchanged | Unit — pure view | none | `cargo test ui::view::` |
| A checklist row reaches the buffer with its label coloured | existing test, unchanged | View render | `TestBackend` real | `cargo test ui::view::` |
| The palette answers every role with a `Style` | existing exhaustive-`match` test, forced to grow by the three variants | Unit — pure view | none | `cargo test ui::palette::` |
| The confinement gate catches a `Color` named outside the palette | `PALETTE` gate + its planted defect in `tests/gate-controls.toml` | Gates + Contract | none | `make gates && cargo test --test gate_controls` |
| The palette module reaches no I/O and measures no width | `NOIO-VIEW` and `COLWIDTH` legs, counts unchanged at ten and nine | Gates | none | `make gates` |
| The enum's membership is exactly this list | existing exhaustive-`match` test, extended to name the three new variants | Unit — pure view | none | `cargo test ui::palette::` |
| The three delta roles carry their colour and no modifier | `ui::palette::tests::the_three_delta_roles_carry_their_colour_and_no_modifier` | Unit — pure view | none | `cargo test ui::palette::` |
| The full set of shared coloured styles is still exactly five groups | `ui::palette::tests::every_shared_style_is_licensed_and_the_unshared_roles_stay_unshared` (pairwise, same test as above) | Unit — pure view | none | `cargo test ui::palette::` |
| A badged header row's colours survive the row's own role | `ui::view::tests::a_badged_header_rows_colours_survive_the_rows_own_role` | View render | `TestBackend` real, reader replaced | `cargo test ui::view::` |

**75 rows, one per scenario in the five delta specs.** 32 are inherited scenarios whose tests this change does not touch, re-run unchanged; the rest are new or extended.

## Visual Design

This change modifies a user-facing view, and **no design source exists** for it — the pane is a
terminal UI with no HTML, Figma, or asset origin, and this repository has never carried a
`design/` directory. The section is stated and skipped rather than invented. The visual contract
is instead the row grammar written into `artifact-folds`' spec and the two colour tables in
`view-palette`'s, both of which are asserted by tests rather than by a picture.

## Decisions

### Decision 1: `crate::specs` is a new module outside `src/ui/`

The two classifiers could live in `src/ui/specs.rs` (beside their callers), in `src/tasks.rs`
(beside the table they share), or in a new `src/specs.rs`.

Chosen: **a new `src/specs.rs`**, outside `src/ui/`. This is `task-labels`' own argument
applied again. A new file *under* `src/ui/` would move the pure-view file count that
`view-palette` and `responsive-layout` both bind in prose and that `noio-view.sh`,
`colwidth.sh`, and `palette.sh` each carry as a `PURE` list — five sites to edit for a function
that needs none of the properties those gates protect. It would also drag the file into
`DETAILWIDTHS`-style width sweeps that a width-free classifier has nothing to say to.

Not `src/tasks.rs`: a delta-operation heading is not a fact about tasks, and putting it there
would make the module's name a lie for the sake of avoiding one new file. `src/tasks.rs` also
holds `tasks::read`, the filesystem edge `NOIO-VIEW` names — and `ui::markdown` must call into
this new module, which is exactly why it must not be that one (Decision 3).

Cost, accepted: adding a module moves `SPEC.md`'s module map, which `tests/doc_contract.rs`
binds. That is one known site with a failing test that names both sides of the disagreement.

### Decision 2: the badge is a glyph carrying a colour, not a colour alone

Considered: recolouring the requirement heading by operation (zero columns), a `+`/`~`/`-`
marker (two columns), and a full gutter marker on every row of the subtree (four columns, at
every width).

Chosen: **the two-column marker**. `src/ui/palette.rs` carries a standing guarantee — "colour
is added strictly beside the modifier a role already carried, so a monochrome reading of the
frame loses nothing" — and a colour-only badge would make `ADDED` and `REMOVED` byte-identical
without colour, breaking exactly that. The gutter was rejected on price: four columns on every
row, at the 58-column narrow interior, spent on a fact that only changes at a requirement
boundary.

This is what lets the three new palette roles carry no modifier without weakening the
monochrome guarantee — the glyph *is* the modifier-equivalent — which is stated in
`view-palette` and is Decision 2's real payoff.

### Decision 3: one table, two recognitions

The clause keywords `WHEN` and `THEN` are already rows in `tasks::label_of`'s table. Reusing
`label_of` wholesale was the obvious move and is **wrong**: its rule 4 requires a colon at or
after the keyword, and a scenario clause has none. It would fire on exactly the clauses that
happen to contain a colon further along — worse than never firing, because the styling would
look arbitrary.

Relaxing rule 4 was rejected outright. `task-labels` measured **zero** false positives across
2563 task items, and that asymmetry — the rule's errors are misses, never wrong colours — is
the property the colon test buys.

Chosen: **share the classification, split the recognition.** `tasks::role_of` already *is* the
table's one site and `label_of` already reaches it only through that function, so the work is to
widen it — `pub`, and `Option<LabelRole>` so a caller that has not already decided the run is a
label can decline — not to extract it. `specs::clause_of` then looks a bold run up in that same
table. Two implementations of one fact drift, and a spec's `WHEN` and a
task's `WHEN` are one fact.

This forced the narrowing of `markdown-render`'s "SHALL NOT call any function of `crate::tasks`"
into "…and MAY call `crate::specs::clause_of`". The two are different risks: `crate::tasks`
holds `tasks::read`, one `use` away from a `NOIO-VIEW` failure; `crate::specs` has no I/O at all
and exists precisely to be callable from a pure view file.

### Decision 4: `operation` is a stored field, derived once per key change

Alternative: derive the operation inside `content_lines`, per frame, by scanning back up the
section list from each header row.

Chosen: **a fifth field on `ArtifactSection`, filled in `sync_detail`**, exactly as
`tasks-emphasis` filled `progress`. `seam-resilience` -> Decision 10 bounds per-frame work to
the `artifact-content` cache's key change; a backwards scan per header row per frame would be
O(sections²) on a held key, for a value that cannot change between two reads of the same file.
The forward walk is O(sections), runs where `progress` already runs, and reuses the section list
that is already in hand.

`Option<DeltaOp>` and not a fourth `DeltaOp::None` variant, for `progress`'s reason: `None`
means "not a delta requirement, take no badge column", where a fourth variant would have to draw
as *something* on every heading in the tree.

### Decision 5: the badge joins the prefix that survives truncation

`artifact-folds` already fixes a drop-whole order as a row narrows: progress cell, then the
label truncated with `…`, then the glyph, then the indent. The badge could slot in before the
label's truncation or after it.

Chosen: **after** — the badge is emitted with the indent and the glyph, and is dropped only once
the label is down to one column. The reason is that the badge is binary and the label degrades
gracefully: `Requirement: The tab bar is bui…` still says what it is, while half a badge says
nothing. The existing spec sentence already states the principle ("the indent, the glyph and its
separating space — emitted first — survive any truncation the label needs"); the badge is
fixed-width and meaning-bearing in exactly the way the glyph is, so it belongs on that side.

### Decision 6: `REMOVED` strikes the heading's label segment only

Considered: no strikethrough (glyph and colour alone), the heading row, and the whole subtree.

Chosen: **the heading row's label segment.** Striking the body would make unreadable exactly the
text a reader opened the section to read — a removed requirement is kept in a delta *so that it
can be read*. No strikethrough at all was the runner-up and was rejected because `CROSSED_OUT`
is the terminal's own rendering of this precise meaning and costs no columns.

The badge segment is deliberately **not** struck: it carries `DeltaRemoved`, the label carries
`Strikethrough`, and the two say different things in one row. This is also why the three delta
roles carry no modifier — `CROSSED_OUT` is spoken for here.

### Decision 7: three `Role` variants, not one parameterised `Delta(DeltaOp)`

`AgentBadge(AgentStatus)` and `Heading(u8)` are both precedents for parameterising. Neither
reason applies: `AgentBadge` answers a status enum another module owns and may grow, and
`Heading` answers levels outside `1..=6` with a value rather than a lookup miss. `DeltaOp` has
exactly three values, all three are spelled out in `palette`'s own tables, and three plain
variants let the modifier and colour tables list them as ordinary rows instead of one row with a
nested match inside it.

### Decision 8: the badge is a `Face`, not a `ContentKind`

`view-palette` already states the test: a *row-wide* distinction the renderer knows and the text
does not is a `ContentKind`; a distinction about **a run of text** is a `Face`. The badge is two
columns of a row whose remaining columns are the label, so it is a run. Making it a kind would
have forced either a second row or a kind carrying a sub-range.

This is what obliges a badged header row to carry **three** segments rather than one. The
payoff is that `ui::view`'s existing loop needs no new branch: it already patches the row's kind
role over each segment's `style_for`, and `Role::DetailSection`/`DetailSectionSelected` carry no
foreground, so the badge's colour survives on a selected row for free.

### Decision 9: the four `Task*` palette roles are **not** renamed

Once a spec's clauses reach `TaskChange` and `TaskConfirm`, those names are misnomers: they are
lifecycle-*position* roles with two consumers, only one of which is tasks. The rename is
correct.

Deferred anyway. `view-palette` has **five** requirements; the four this change modifies are
203, 197, 169, and 146 lines (the fifth, "Colour is a named ANSI index", is 29 lines and is left
alone). A `MODIFIED` requirement must carry full content — so renaming four roles costs a
**715**-line rewrite of specs, with no behaviour in it, on top of a change that already rewrites those same
four requirements for three genuine additions. It would bury this change's content in churn and
make the diff unreviewable in one sitting.

Recorded here rather than dropped: the role *meanings* are already right ("the Change position
of a testing lifecycle" describes a scenario's `WHEN` exactly), so only the `Task` prefix is
wrong, and nothing about this change makes the rename harder later.

### Decision 10: `AND` inherits, and a heading resets the inheritance

`AND` is the most common token by far — 6701 against 3877 each for `WHEN` and `THEN` — and it is
in no row of the lifecycle table. Three options: classify it `Other` (grey), give it its own
role, or let it carry the position of the clause above it.

Chosen: **inherit**. `- **AND**` continues the preceding clause by definition, so it *is* that
position; grey would say "unclassified" about the one token whose meaning is fully determined by
context. `Clause::Continues` carries no role of its own, which keeps the table untouched.

The state resets at **every heading, at any level**, because a heading is the boundary between
one scenario and the next — an `AND` under a fresh `#### Scenario:` must never inherit across it.
It deliberately does *not* reset at a blank line or paragraph, since this repository's scenarios
routinely wrap clauses across continuation lines. A `Continues` reached with nothing remembered
degrades to `LabelRole::Other` rather than panicking or dropping the label.

### Decision 11: `lines` applies the clause rule to every source, with no spec-shape test

`ui::markdown::lines(source, width)` is parameterised by text and width and knows nothing about
which artifact it is drawing. Threading the tab's identity in — so that clauses were styled only
in a spec — would put view state into a pure function for no gain.

Safe because the rule is narrow on three axes at once: the run must be `Strong`, it must be the
*first* inline of a list item, and it must be exactly a table token. `markdown-legibility`'s
task-list items place their `[ ]` marker before any strong run and so are untouched, and prose
bullets opening with a bold non-keyword (`- **Note**`) classify to `None`.

### Decision 12: `DeltaRemoved` is `LightRed`, not `Red`

`Red` is `ListProblem` and nothing else. A reader scans the pane for exactly one red thing, and
a removed requirement is not a problem — it is the ordinary content of a delta spec: **15**
`## REMOVED Requirements` heading blocks across the archive, holding **18** removed requirements
between them. `LightRed` shares a style with `AgentBadge(Blocked)` (list region only) and
`TaskEvidence` (tracked-tasks tab only), both licence-1 "cannot meet" shares, and costs nothing.

This repeats `tasks-emphasis`' own reasoning for `TaskEvidence`, including the correction it
recorded: `ListProblem`'s red is reached only from `row_role` in the list region, so `Red` would
have been *licensed* here too. It is still declined, because the licence says a share is
allowed, not that it is free for the reader.

## Risks / Trade-offs

- **Adding `src/specs.rs` moves `SPEC.md`'s module map and the doc-contract test** → the failure
  is loud and names both sides; the task list edits the map in the same group that adds the
  module.
- **`ArtifactSection` gaining a field breaks all 75 construction sites, 19 of them in
  `src/ui/detail.rs`'s tests** → deliberate. `NODEFAULT-UI` scans this type set precisely so
  that the compiler, not a reviewer, finds them.
- **The clause rule applies to every markdown source, so a non-spec document with a
  `- **WHEN**` bullet gets coloured** → accepted and narrow. Measured, the token appears as a
  leading bold run only in specs; and the failure mode is a coloured keyword in a proposal,
  not a wrong rendering.
- **`role_of`'s extraction changes `label_of`'s implementation** → a scenario calls both over
  all thirteen tokens and compares, so a divergence fails rather than ships.
- **The badge's glyphs `+`, `~`, `-` are ASCII and measure one column** → unlike the six
  Ambiguous-width glyphs `markdown-legibility` accepted, these add **no** new CJK-locale
  exposure. Worth stating because the obvious cheap alternative (`▲`/`△`/`▽`) would have.
- **Two more columns of indent on requirement rows at the 58-column interior** → bounded: the
  badge is on header rows only, never on body rows, and is dropped whole before the glyph.

## Migration Plan

None required, and the reasons are worth stating rather than omitting. Nothing is persisted, so
there is no data to migrate or backfill. There is no deploy order: the crate is one binary that
Herdr launches per pane, and `make build` replaces it wholesale. Rollback is `git revert` plus
`make build` — no state survives a version change, because the only thing the plugin writes is
`agent-names.toml` under `HERDR_PLUGIN_STATE_DIR` and this change does not touch it. A pane
running the old binary beside one running the new renders the same repository correctly, badges
being derived at render time from files neither writes.

## Open Questions

**None remain.** The proposal opened four; all four are answered and recorded in its Review
Decisions section, and the fifth that surfaced while answering them is settled in Decision 3
above. The one thing deliberately left undecided is *when* the `Task*` role rename happens
(Decision 9) — it is a follow-up change, not an open question for this one.
