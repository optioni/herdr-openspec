## Context

`ui::detail::header_row` draws the detail region's heading: a name field, a `(schema)` cell,
and `ui::list::progress_cell`'s `[4/9]`, with cells dropped whole — schema first, then
progress — as the row narrows. `ui::tasks::progress_bar` draws a `█`/`░` gauge one region
below, but only on the tab the schema marks as tracking tasks. Reading `proposal.md` therefore
costs the reader the gauge, and getting it back costs a tab switch each way.

The constraint that shapes everything here: both functions are in the crate's **pure view
set**, ten files that name no filesystem, process, environment, network, or standard-I/O API.
This change adds no I/O, no dependency, no thread, and no process spawn. It is arithmetic and
two `&str`s, tested by calling functions and by rendering into a `TestBackend`.

The proposal left one question open — whether the bar replaces the numeric cell or sits beside
it. It was settled before this document, in favour of a **bare gauge cell beside** the numeric
one; the proposal's own § *Settled: the shape and the width split* records the decision and
the two alternatives rejected. This design implements that shape.

## Goals / Non-Goals

**Goals:**

- A `█`/`░` gauge for the selected change in the detail heading row, on every artifact tab.
- One gauge implementation in the crate, so the header's run and the tracked-tasks tab's run
  cannot disagree about the same change.
- No observable change below 26 columns, and none at all for a change with no tasks.
- `ui::tasks::progress_bar`'s output byte-identical at every width.

**Non-Goals:**

- Changing what `progress` means or where it comes from. The dual-source rule stands.
- The tracked-tasks tab's own body, which keeps its bar and its checklist grammar.
- The list region's per-row progress cells.
- Any new palette role or colour.
- Any interactivity: the gauge is not clickable and not a scrub target.
- A percent cell in the header. The heading already states `[4/9]`; a third statement of the
  same fact one space away is noise, and dropping it keeps the header's budget small enough
  that the name field is not squeezed at 58 columns.

## Boundaries

| Piece | Module | Pattern it follows |
|---|---|---|
| The gauge cell in the heading | `src/ui/detail.rs` (`header_row`) | Existing drop-whole cell grammar; already calls into `ui::list` for two of its three cells |
| The gauge run itself | `src/ui/tasks.rs` (`gauge_of`, raised to `pub(crate)`) | Exactly what `ui::list::progress_cell` already is: one `pub(crate)` formatter that several view files call rather than copy |
| The 12-column budget | `src/ui/detail.rs`, a named `const` | A module-local constant beside the grammar that uses it, like the rest of this file's arithmetic |
| Width measurement | `ui::layout::columns` | The crate's only measure; `COLWIDTH` sweeps `src/ui/detail.rs` for `.chars()`-based alternatives |
| Styling | `ui::view`, unchanged | The heading is one string styled whole under `Role::RegionHeadingFocused`/`Role::RegionHeading` |

Both edited files are in the pure view set. No file gains an I/O, process, clock, or
blocking-wait API. No process spawn is added anywhere, inside `src/cli.rs` or outside it —
this change never reaches the subprocess seam. `HerdrCli` is not named. No thread is started,
so `NOBLOCK`'s seam-module list is untouched.

`src/ui/detail.rs` calling `crate::ui::tasks::gauge_of` is a new edge between two pure view
files. It is the same kind of edge `src/ui/detail.rs` already has to `crate::ui::list`
(`progress_cell`, `pad_or_truncate_right`) and that `src/ui/tasks.rs` already has to
`crate::ui::list` and `crate::ui::markdown`. No gate constrains the shape of the graph inside
the pure set; the constraints are on what those files may *name*, and this names a pure
function over `&Progress`.

## Contracts

**`ui::detail::header_row` — signature unchanged, output changed.** It keeps
`(name: &str, schema: &str, progress: &tasks::Progress, width: u16) -> String`. The gauge is
derived from the `progress` argument the function already takes, so `ui::view::render`'s one
call site (`src/ui/view.rs:150`) needs **no edit at all**. That is the design's main
compatibility property: the only consumer of this function outside its own tests does not
change.

The *output* changes at `width >= 26` for a change with `total > 0`. Consumers affected: none
in production — `ui::view` draws whatever string it is handed.

The affected callers are the existing tests in **`src/ui/detail.rs`**, five of which assert
the pre-gauge grammar literally and go red the moment the gauge lands: `:878` asserts
`format!("{name_field} (tdd) [4/42]")` with a 65/45-column name field, `:927` asserts `(tdd)`
is present at `w >= 13` (now `w >= 26`), and `:951`'s `got.contains("() [1/2]")` is split by
the gauge between the two cells. Updating them is this change's own work, not collateral.

`src/ui/view.rs`'s two test expectations are **not** affected, and this is worth stating
because it is counter-intuitive: both build their expectation by calling the function under
test — `let expected = crate::ui::detail::header_row("add-token-refresh", "tdd", &progress, w)`
at `:5208`, and the same call inline at `:6750`. They discriminate the header's placement,
span, and style, never its grammar, so they stay green with or without a gauge. The corollary
binds this change's new view tests: they SHALL assert literal glyph counts and literal tails
(`five █`, ending `(tdd) █████░░░░░░░ [4/9]`) and never `assert_eq!(buffer, header_row(…))`,
which would reproduce exactly the tautology already sitting at those two sites.

**`ui::tasks::gauge_of` — private to `pub(crate)`, additive.** Its behaviour is unchanged for
every input `progress_bar` passes it. It gains a guard returning the empty string at `g == 0`
or `total == 0`, neither of which either production call site reaches. Error surface: none —
the function is total and returns a `String`, never a `Result`. No pagination, no streaming.
Additive, not breaking.

**`ui::tasks::progress_bar` — unchanged.** Byte-identical at every width for every `Progress`.

**The `Change` type is not altered.** No field is added, removed, or retyped, so
`changes::from_files` and `changes::from_cli` need no work to stay in agreement and
`changes::merge` is untouched. The gauge is a second rendering of the `progress` field both
paths already produce and `merge` already reconciles.

## Persistence and Rollout

- **Migration:** none. No persisted format changes.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. The `artifact-content` cache is keyed on
  `(change directory, tab)` and caches artifact *text*; the heading row is recomputed from
  `Change::progress` on every draw and was never cached.
- **Index rebuild:** none.
- **Authorization:** none. The pane is read-only and this change reads nothing new.
- **Observability:** none. The crate emits no metrics or logs.
- **Deployment:** `make build` produces the release binary the pane runs; a linked plugin
  picks up the new binary on the pane's next open. No manifest, config, action, or keybinding
  change, so `herdr plugin link` does not need re-running and `tests/manifest.rs` is untouched.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem (`std::fs`) | **real** in the contract tier — `tests/degraded_coverage.rs` reads `SPEC.md` (`:558`), `tests/degraded-coverage.toml` (`:562`), and every source file named by a `covers` range (`:222`, `:292`, `:329`, `:366`), and `tests/doc_contract.rs` reads the repository likewise (`:63`). Not reached in the unit or view tiers: `Dashboard` fixtures are constructed in memory and the injected artifact reader is a closure over a `&str` | not reached; `header_row` and `gauge_of` are functions of `&str`/`&Progress`/`u16` |
| `openspec` binary (`OpenspecCli`) | not reached — no `Change` in these tests is CLI-sourced, and no probe runs | not reached |
| Herdr socket (`HerdrCli`) | not reached — no agent poll, no launch, no pane open; `Dashboard` fixtures carry no agents | not reached |
| Terminal (crossterm raw mode, alternate screen, mouse capture) | **replaced** — `ratatui::backend::TestBackend`, never a real terminal; `cargo test` spawns this binary and a real terminal mode would corrupt the developer's session | not reached |
| Process environment (`std::env`) | not reached — no `env_lookup` is called on this path | not reached |
| Clock (`Instant::now`) | not reached — the render path reads no clock | not reached |
| Worker threads (`Refresher`, `AgentPoll`, `Launcher`, `FsEvents`) | not reached — no event loop is run; these tests call `ui::view::render` against a fixed `Dashboard` | not reached |
| `ui::layout::columns` / `truncate_columns` | **real** — the crate's one width measure is the subject, not a collaborator to replace | real |
| `ui::list::progress_cell` / `pad_or_truncate_right` | **real** — the point of the design is that there is one of each | real |
| `ui::tasks::gauge_of` | **real** — the point of the design is that there is one of it | real |
| `ui::palette::style` | **real** — style assertions compare buffer cells against `palette::style(role)`, never a colour literal, per `PALETTE` | real |

## Test Strategy

Four tiers, all inside `cargo test` (`make test`):

- **Unit** — inline `#[cfg(test)]` in `src/ui/detail.rs` and `src/ui/tasks.rs`. Pure calls,
  string assertions, sweeps over widths. Every test names both mandated interiors, 78 and 58.
- **View** — rendering a `Dashboard` into a `TestBackend` at **120x20 and 60x20**, asserting
  buffer rows, columns, and styles. Note the tier is not the file: two view-tier tests for
  this capability live in `src/ui/detail.rs`, not `src/ui/view.rs`.
- **Contract** — `cargo test --test degraded_coverage` and `--test doc_contract`. These read
  the repository from disk, which is why the Test Boundaries table marks the filesystem real
  for this tier and only this one.
- **Gate** — `make gates`, unchanged, run over the edited files: `COLWIDTH` (no `.chars()`
  width in the pure set), `NOIO-VIEW`, `PALETTE`, `MDSEAM`, `NOSPAWN-GREP`.

**This change takes no outer-loop acceptance test of its own.** The repository's outermost
tier for a view change *is* the `TestBackend` render at the two mandated frame widths — there
is no higher loop short of driving a real terminal, which the architecture forbids because
`cargo test` spawns this binary.

**Twelve of the nineteen scenarios already have passing tests at HEAD.** The repository binds
a scenario to a test by snake-casing the scenario header and carrying a
`/// \`<capability>\` :: "<scenario header>"` doc comment. For those twelve the work is to
**rewrite the existing test in place**, keeping its name and its doc comment, so the RED
evidence is honest: an expectation edited to the post-gauge value genuinely fails against the
unchanged `header_row`, reporting `1 failed`. Inventing a second test per scenario would
leave a duplicate pair, one permanently red. The `Verification` column below says `rewrite
<name>` or `new` for every row, and the `Command` column names the test that actually exists.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| The full header grammar at both mandated interior widths | rewrite `the_full_header_grammar_at_both_mandated_interior_widths` (`src/ui/detail.rs:868`) — name field 65/45 → 52/32, expected tail gains the gauge | Unit | real `columns`, `progress_cell`, `gauge_of` | `cargo test --lib ui::detail::tests::the_full_header_grammar_at_both_mandated_interior_widths` |
| A complete change renders a full gauge and an untouched one renders an empty gauge | **new** in `src/ui/detail.rs` | Unit | real `gauge_of` | `cargo test --lib ui::detail::tests::a_complete_change_renders_a_full_gauge` |
| A change with no tasks still ends its row in the same column | rewrite `a_change_with_no_tasks_still_ends_its_row_in_the_same_column` (`src/ui/detail.rs:885`) — extend to nine widths and assert no `█`/`░`; its existing claims survive unchanged per Decision 7 | Unit | real `progress_cell` | `cargo test --lib ui::detail::tests::a_change_with_no_tasks_still_ends_its_row` |
| A long name is truncated with an ellipsis, never overflowing the row | rewrite `a_long_name_is_truncated_with_an_ellipsis_never_overflowing_the_row` (`src/ui/detail.rs:898`) — its `find(" (tdd)")` name-field slice changes meaning and must be re-derived | Unit | real `pad_or_truncate_right` | `cargo test --lib ui::detail::tests::a_long_name_is_truncated_with_an_ellipsis` |
| The cells are dropped whole in order as the row narrows | rewrite `the_cells_are_dropped_whole_in_order_as_the_row_narrows` (`src/ui/detail.rs:919`) — the `w >= 13` band assertion becomes `w >= 26`, and widths 26 and 25 join the sample | Unit | real `columns` | `cargo test --lib ui::detail::tests::the_cells_are_dropped_whole_in_order` |
| Below the full-form band the header is byte-identical to the pre-gauge grammar | **new** in `src/ui/detail.rs`; its 26 expected strings are captured from HEAD's `header_row` **before** the implementation lands and written as literals, never recomputed | Unit | real `pad_or_truncate_right` | `cargo test --lib ui::detail::tests::below_the_full_form_band_the_header_is_byte_identical` |
| An empty schema name is a cell of two characters, not an absent one | rewrite `an_empty_schema_name_is_a_cell_of_two_characters_not_an_absent_one` (`src/ui/detail.rs:950`) — `contains("() [1/2]")` is split by the gauge and must become the full expected row | Unit | real `gauge_of` | `cargo test --lib ui::detail::tests::an_empty_schema_name_is_a_cell_of_two_characters` |
| The header names the selected change at both mandated widths | rewrite `the_header_names_the_selected_change_at_both_mandated_widths` (`src/ui/view.rs:5191`) — assert the literal tail and five `█`, not `assert_eq!(buf, header_row(…))` | View | `TestBackend`, real `palette::style` | `cargo test --lib ui::view::tests::the_header_names_the_selected_change` |
| Moving the selection moves the header | rewrite `moving_the_selection_moves_the_header` (`src/ui/view.rs:5230`) — add the gauge run to the three-way difference | View | `TestBackend` | `cargo test --lib ui::view::tests::moving_the_selection_moves_the_header` |
| The gauge is present on an artifact tab that is not the tracked-tasks one | **new** in `src/ui/view.rs` — the headline scenario of the change | View | `TestBackend`, real `gauge_of` | `cargo test --lib ui::view::tests::the_gauge_is_present_on_an_artifact_tab` |
| An archived change's header carries its stripped name and its own schema | rewrite `an_archived_change_s_header_carries_its_stripped_name_and_its_own_schema` (`src/ui/view.rs:5257`) — name fields 45/25, twelve `█` | View | `TestBackend` | `cargo test --lib ui::view::tests::an_archived_change_s_header_carries_its_stripped_name` |
| An empty visible list leaves the whole detail interior blank | rewrite `an_empty_visible_list_leaves_the_whole_detail_interior_blank` (`src/ui/view.rs:5279`) — add the no-`█`/`░`-anywhere clause | View | `TestBackend` | `cargo test --lib ui::view::tests::an_empty_visible_list_leaves_the_whole_detail_interior_blank` |
| A CJK change name keeps the header inside its region at both mandated widths | rewrite `a_cjk_change_name_keeps_the_header_inside_its_region_at_both_mandated_widths` (`src/ui/detail.rs:2162`) — `(tdd)` is no longer immediately before the cell | Unit | real `columns`, `truncate_columns` | `cargo test --lib ui::detail::tests::a_cjk_change_name_keeps_the_header_inside_its_region` |
| The header reaches the buffer without crossing the region border | rewrite `the_header_reaches_the_buffer_without_crossing_the_region_border` (**`src/ui/detail.rs:2191`**, not `view.rs`) — its `tail = " (tdd) [4/9]"` becomes the gauge-bearing tail | View | `TestBackend` | `cargo test --lib ui::detail::tests::the_header_reaches_the_buffer_without_crossing` |
| The header is total over adversarial names at every width | rewrite `header_row_is_total_over_adversarial_names_at_every_width` (`src/ui/detail.rs:2244`) — cross the five names with four `Progress`, band clause scoped to `{4, 9}` | Unit | real `columns` | `cargo test --lib ui::detail::tests::header_row_is_total_over_adversarial_names` |
| The full grammar at both mandated interior widths | **unchanged** — `bar_full_grammar` (`src/ui/tasks.rs:355`) and `full_grammar_is_byte_identical_to_pre_change_output` (`:1214`) stay green; carried into the MODIFIED block only because a delta replaces the whole requirement | Unit | real `gauge_of`, `progress_cell` | `cargo test --lib ui::tasks::tests::bar_full_grammar` |
| The bar reaches the buffer at both mandated frame widths | **unchanged** — `progress_bar_in_the_buffer` (`src/ui/view.rs:6007`) stays green | View | `TestBackend` | `cargo test --lib ui::view::tests::progress_bar_in_the_buffer` |
| The percentage truncates rather than rounds | **unchanged** — `percent_truncates` (`src/ui/tasks.rs:384`) stays green; its fixtures are 2/3, 1/3, 0/7, 7/7, none of which saturate | Unit | real `percent_of` | `cargo test --lib ui::tasks::tests::percent_truncates` |
| The bar measures at most its width at every width | **unchanged** — `bar_measures_at_most_its_width_at_every_width` (`src/ui/tasks.rs:1171`) stays green; it reaches `usize::MAX` but asserts only that the bar fits its width, which is why it never caught the saturation defect | Unit | real `progress_bar` | `cargo test --lib ui::tasks::tests::bar_measures_at_most_its_width` |
| A saturating `Progress` renders a full gauge and a full percentage | **new** in `src/ui/tasks.rs` — fails against the shipped saturating arithmetic (one `█`, `1%`) | Unit | real `gauge_of`, `percent_of` | `cargo test --lib ui::tasks::tests::a_saturating_progress_renders_a_full_gauge` |
| A one-task-short change never renders a full gauge | **unchanged** — `gauge_full_only_when_complete` (`src/ui/tasks.rs:428`) stays green | Unit | real `gauge_of` | `cargo test --lib ui::tasks::tests::gauge_full_only_when_complete` |
| The property holds across a swept range of gauge widths | **unchanged** — `gauge_property_sweep` (`src/ui/tasks.rs:462`) stays green; its six fixtures top out at 999/1000 | Unit | real `gauge_of` | `cargo test --lib ui::tasks::tests::gauge_property_sweep` |
| The completeness property holds at the saturation boundary | **new** in `src/ui/tasks.rs` — the clause that fails before the widening | Unit | real `gauge_of` | `cargo test --lib ui::tasks::tests::the_completeness_property_holds_at_the_saturation_boundary` |
| The bar's rendered output does not move | **new** in `src/ui/tasks.rs`, expectations built independently of `progress_bar` per `full_grammar_is_byte_identical_to_pre_change_output`'s stated discipline | Unit | real `gauge_of`, `progress_cell` | `cargo test --lib ui::tasks::tests::the_bar_s_rendered_output_does_not_move` |
| The header's gauge and the bar's gauge agree about the same change | **new** in `src/ui/tasks.rs` | Unit | real `gauge_of`, real `header_row` | `cargo test --lib ui::tasks::tests::the_header_s_gauge_and_the_bar_s_gauge_agree` |
| The gauge is full exactly when the change is complete, at the header's width too | **new** in `src/ui/tasks.rs`; its `usize::MAX` clause is the one that fails against the shipped implementation | Unit | real `gauge_of` | `cargo test --lib ui::tasks::tests::the_gauge_is_full_exactly_when_the_change_is_complete` |
| The promoted function is total at both guard values | **new** in `src/ui/tasks.rs` | Unit | real `gauge_of`, real `progress_bar` | `cargo test --lib ui::tasks::tests::the_promoted_function_is_total_at_both_guard_values` |

One existing test is deliberately **not** in this table: `src/ui/view.rs:6726`
`the_detail_header_is_bold_and_uncoloured_at_both_mandated_widths`, which proves the fourth
live `detail-header` requirement. It asserts every cell of the header row carries BOLD and no
foreground, which the gauge does not change (Decision 8), and it builds its expectation by
calling `header_row`, so it is invariant to the grammar. That requirement therefore takes no
delta and needs no task.

Whole-suite gate: `make check`.

## Decisions

**Decision 1 — A bare gauge cell beside the numeric cell, not `progress_bar` swapped in.**
`progress_bar` already *contains* the numeric cell (`gauge ␣ [n/m] ␣ pct%`), so calling it
from the header would print `[4/9]` twice in one row. The coherent "beside" shape is therefore
the bare run. *Alternative rejected:* replacing `progress_cell` with `progress_bar` in the
header. It is a smaller diff and inherits `progress_bar`'s degradation wholesale, but it puts
a percent cell in a heading that already states the exact pair, and it makes the header's
grammar depend on a second function's drop order rather than owning its own. *Alternative
rejected:* a second heading row carrying a full-width bar. Most legible, but it costs one
content row at every width and pulls `src/ui/layout.rs`'s viewport arithmetic into a change
whose proposal scoped it as a cell.

**Decision 2 — The gauge is dropped *first*, ahead of both existing cells.** Placing the new
cell at the head of the drop order means the schema and progress cells keep their relative
positions and every band below the full form produces byte-identically what it produced
before. That converts "did this change break the narrow header?" into a test that compares 26
literal strings rather than a judgment call. *Alternative rejected:* dropping the gauge
between the schema and progress cells, which would have renumbered the existing bands for no
gain.

**Decision 3 — A fixed 12-column budget; the name field absorbs the remainder.** The gauge
reads identically at 58 and at 120 columns, and every column a wider frame brings goes to the
name — the field already being squeezed at the narrower mandated interior. *Alternative
rejected:* a name-field minimum with the gauge taking the remainder, which gives a 47-column
gauge at 78 columns but leaves a long change name truncated at 120 where there is plainly room
for it. *Alternative rejected:* a proportional split, which scales both fields but introduces
rounding the header has never had and turns every width assertion into a computed expectation
rather than a literal string — losing exactly the property Decision 2 buys.

**Decision 4 — Twelve.** At the 58-column interior it leaves a 32-column name field, wider
than the longest change name this repository has (`foldable-spec-sections`, 22). At 78 it
leaves 52. It resolves a one-task move on a ten-task change to a visible cell
(`12*3/10 = 3`, `12*4/10 = 4`). The constant lives as a named `const` in `src/ui/detail.rs`
beside the grammar that spends it, not in `src/ui/layout.rs`: it is a fact about this row's
composition, not about the frame.

**Decision 5 — Promote `gauge_of` rather than copy the run.** `src/ui/detail.rs` could format
twelve characters itself in three lines. It must not, for the same reason it already calls
`ui::list::progress_cell` instead of formatting `[{}/{}]`: two implementations of one fact can
drift, and this fact — how full a change is — is the one the change exists to show in two
places at once. Promotion costs one visibility keyword.

**Decision 11 — the saturation defect is repaired by widening to `u128`, not by a
completeness short-circuit.** `gauge_of` and `percent_of` both compute
`x.saturating_mul(k) / total` in `u64`. At `Progress { completed: usize::MAX, total: usize::MAX }`
the multiply saturates to `u64::MAX` and `u64::MAX / u64::MAX == 1`, so a complete change
renders a one-cell gauge beside `1%` — at the header's 12 columns and at the bar's own 68- and
48-column gauges. Saturation produces the wrong *quotient* here, not merely a clamped
magnitude, which is why the existing "so no `Progress` value can overflow it" clause did not
protect the property. `usize::MAX * 100` and `usize::MAX * g` both fit in `u128` for every
`u16` `g`, so widening removes the saturation entirely. *Alternative rejected:* short-circuiting
`gauge_of` on `is_complete()`. It repairs the gauge but leaves `percent_of` reading `1%` beside
a now-full gauge — a bar that contradicts itself at the same input — and it contradicts the
live formula clause `filled = g * completed / total` rather than making it true, so it would
need a MODIFIED block saying the formula no longer holds. The widening needs one saying how the
product is computed, and every ordinary value is provably unchanged (4/9 at g=68 → 30 either
way; 3/10 at g=12 → 3; 4/42 at g=12 → 1). *Alternative rejected:* qualifying the property with
a saturation regime, which weakens shipped text to accommodate a defect.

**Decision 6 — `gauge_of` becomes total.** It currently divides by `total` with no guard,
safe only because its one caller returns before reaching it at `total == 0`. A `pub(crate)`
function is reachable from a call site it does not control, so it gains a guard returning the
empty string at `g == 0` or `total == 0`. Neither production call site reaches either value,
so no rendered output moves — the guard is a contract for the next caller, not a behaviour
change. *Alternative rejected:* documenting the precondition and leaving the panic, which
makes the promotion a trap.

**Decision 7 — No gauge at all when `total == 0`, and no space reserved for one.**
`header_row` branches on `total == 0` *before* computing the budget, so a change with no tasks
renders exactly the row it renders today, at every width — not a 13-column-narrower name field
beside an absent gauge. This matches `tasks-progress-bar`'s existing rule and its reason: a
gauge with no denominator would have to invent a fill.

**Decision 8 — No new palette role.** The heading is one string that `ui::view` styles whole
under `Role::RegionHeadingFocused`/`Role::RegionHeading`. The gauge inherits that pair. The
existing scenario *The detail header is bold and uncoloured at both mandated widths* asserts
every cell of the row reports BOLD and no foreground — still true with the gauge in it, which
is why that requirement takes no delta. *Alternative rejected:* a coloured gauge, which would
compete with the tab chips below, the one place this region spends colour on purpose.

**Decision 9 — `responsive-layout` takes no delta.** The proposal listed it as modified, under
a shape where the bar might have needed a width split from the layout. Under Decisions 1 and 3
the mandated 78- and 58-column interiors do not move, the gauge's budget is `detail-header`'s
constant, and the display-column rule the gauge inherits is already `responsive-layout`'s
standing requirement. A delta would restate existing text. The proposal was corrected to say
so rather than left to disagree with this document.

**Decision 10 — `header_row`'s signature does not change.** The gauge comes from the
`progress` argument already passed. `ui::view::render` is therefore untouched by this change
except in its tests' expectations, which is the strongest available evidence that the change
is confined to the grammar.

## Risks / Trade-offs

- **A CJK-locale terminal resolving East Asian Ambiguous to two columns paints `█`/`░`
  double-width, overrunning the heading row past its region** → Accepted and recorded, not
  mitigated. This is the standing exposure `SPEC.md` already documents for the glyphs
  `markdown-constructs` and `markdown-legibility` introduced, and for this same gauge as the
  tracked-tasks tab already draws it one region below. This change widens the exposure to one
  more row; it does not create a new kind of it. Compensating here and nowhere else would be
  worse than the honest, recorded gap.
- **A fixed 12-column gauge cannot resolve a one-task move on a change with more than 12
  tasks** → The `[4/42]` cell one space away states the exact pair, which is why the gauge is
  an addition to that cell rather than a replacement for it. Any fixed gauge has this limit;
  a remainder-width gauge would only move the threshold, at the cost rejected in Decision 3.
- **Five existing tests in `src/ui/detail.rs` assert the pre-gauge grammar and go red when
  the gauge lands** → Expected and in scope: the verification matrix above assigns each one a
  `rewrite` row, so none is left unowned. The scenario *Below the full-form band the header is
  byte-identical to the pre-gauge grammar* is the guard that those rewrites were confined to
  the widths that should have moved. `src/ui/view.rs`'s expectations are computed by calling
  `header_row` and stay green either way, which is why no task edits that file's tests.
- **A new view test written in the house style would be a tautology** → `src/ui/view.rs:5208`
  and `:6750` both assert the buffer equals `header_row(…)`, the function under test. Copying
  that shape for the three new view tests would produce three more that cannot fail on the
  gauge, so the matrix requires literal glyph counts and literal tails instead.
- **A `Progress` at the saturation boundary renders a gauge and a percentage that contradict
  `is_complete()`** → Repaired at the root rather than documented, per Decision 11. This is a
  defect in already-shipped behaviour, reached at `g` of 12, 48 and 68, that no existing test
  caught because the one `usize::MAX` sweep asserts only that the bar fits its width.
- **A `.chars()`-based width slipping into the new arithmetic would be silently correct on
  ASCII fixtures and wrong elsewhere** → The guards are `COLWIDTH`, which sweeps
  `src/ui/detail.rs`, and the CJK scenarios' `layout::columns(&got) == 78` / `== 58`
  assertions. Not their `chars().count() < columns()` clause, which does **not**
  discriminate: for the CJK fixture a correct implementation returns 68 chars against 78
  columns and a char-counting one returns 78 against 88, and both satisfy the strict
  inequality. That clause is evidence the wide name survived truncation, nothing more —
  `openspec/specs/tasks-progress-bar/spec.md` already calls the `columns`-versus-`chars`
  comparison "a tautology" for an all-width-1 fixture, and the 12-column gauge adds equally
  to both sides, so it neither strengthens nor weakens the scenario.
- **Coverage could drift below the 80% line floor or the production-slice floor** → The new
  code is a handful of branches, every one of which a listed scenario enters. `make coverage`
  is in `make check`. The floor is never lowered.

## Migration Plan

None needed. No persisted data, no schema, no manifest, no config, no keybinding, and no
public API outside the crate. The change is a rebuild: `make build`, and the pane picks up
the new binary on its next open. Rollback is `git revert` of the commits — nothing outside the
binary is written or migrated, so a revert is complete on its own.

## Open Questions

None. The proposal's one open question — bar-replaces-cell versus bar-beside-cell, and the
width split — was settled before this document and is recorded in the proposal's
§ *Settled: the shape and the width split* and in Decisions 1 through 3 above.
