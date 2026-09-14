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
in production — `ui::view` draws whatever string it is handed. The affected callers are the
existing tests in `src/ui/detail.rs` and `src/ui/view.rs` whose expectations name the old
strings, which this change updates as part of its own work rather than leaving red.

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
| Filesystem (`std::fs`) | not reached — `Dashboard` fixtures are constructed in memory, and the injected artifact reader is a closure over a `&str` | not reached; `header_row` and `gauge_of` are functions of `&str`/`&Progress`/`u16` |
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

Three tiers, all inside `cargo test` (`make test`):

- **Unit** — inline `#[cfg(test)]` in `src/ui/detail.rs` and `src/ui/tasks.rs`. Pure calls,
  string assertions, sweeps over widths. Every test names both mandated interiors, 78 and 58.
- **View** — inline `#[cfg(test)]` in `src/ui/view.rs`, rendering a `Dashboard` into a
  `TestBackend` at **120x20 and 60x20**, asserting buffer rows, columns, and styles.
- **Gate** — `make gates`, unchanged, run over the edited files: `COLWIDTH` (no `.chars()`
  width in the pure set), `NOIO-VIEW`, `PALETTE`, `MDSEAM`, `NOSPAWN-GREP`.

**This change takes no outer-loop acceptance test of its own.** The repository's outermost
tier for a view change *is* the `TestBackend` render at the two mandated frame widths — there
is no higher loop short of driving a real terminal, which the architecture forbids because
`cargo test` spawns this binary. The view rows below are that outer loop.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| The full header grammar at both mandated interior widths | `header_row` at 78 and 58; assert full string, 52/32-column name fields, one `█` and eleven `░` | Unit | real `columns`, `progress_cell`, `gauge_of` | `cargo test --lib ui::detail::tests::full_header_grammar` |
| A complete change renders a full gauge and an untouched one renders an empty gauge | `header_row` at 78/58 for 7-of-7, 0-of-7, 6-of-7; count `█` and `░` | Unit | real `gauge_of` | `cargo test --lib ui::detail::tests::header_gauge_fill` |
| A change with no tasks still ends its row in the same column | `header_row` 0-of-0 at nine widths; assert no `█`/`░`, 68/48-column name fields, `[-]` last | Unit | real `progress_cell` | `cargo test --lib ui::detail::tests::header_no_tasks` |
| A long name is truncated with an ellipsis, never overflowing the row | 200-char name at 78 and 58; assert width, trailing `…`, all three cells intact | Unit | real `pad_or_truncate_right` | `cargo test --lib ui::detail::tests::header_long_name` |
| The cells are dropped whole in order as the row narrows | `header_row` at 78, 58, 26, 25, 13, 12, 7, 6, 5, 1, 0; assert band membership and no partial cell | Unit | real `columns` | `cargo test --lib ui::detail::tests::header_cells_drop_whole` |
| Below the full-form band the header is byte-identical to the pre-gauge grammar | `header_row` for every width 0..=25 against 26 literal expected strings | Unit | real `pad_or_truncate_right` | `cargo test --lib ui::detail::tests::header_unchanged_below_full_band` |
| An empty schema name is a cell of two characters, not an absent one | `header_row("alpha", "", 1-of-2, 78/58)`; assert `()`, 56-column name field, six `█` | Unit | real `gauge_of` | `cargo test --lib ui::detail::tests::header_empty_schema` |
| The header names the selected change at both mandated widths | Render 120x20 and 60x20; assert row 0 cols 42..119 and 1..58, five `█`, BOLD set; re-render at `Route::List` for DIM | View | `TestBackend`, real `palette::style` | `cargo test --lib ui::view::tests::detail_header_names_selection` |
| Moving the selection moves the header | Render, set `selected: 1`, re-render at both widths; assert name, gauge run, and cell all three differ | View | `TestBackend` | `cargo test --lib ui::view::tests::detail_header_follows_selection` |
| The gauge is present on an artifact tab that is not the tracked-tasks one | Render with `proposal` tab selected; assert heading gauge present and no `%` anywhere in frame; switch to tracked-tasks tab and assert heading gauge byte-identical | View | `TestBackend`, real `gauge_of` | `cargo test --lib ui::view::tests::detail_header_gauge_on_every_tab` |
| An archived change's header carries its stripped name and its own schema | Render archived `2026-08-14-add-auth`; assert `add-auth`, `(spec-driven)`, twelve `█`, 45/25-column name fields, no date field | View | `TestBackend` | `cargo test --lib ui::view::tests::detail_header_archived_change` |
| An empty visible list leaves the whole detail interior blank | Render `empty_set()` and a filter matching none; assert every cell default-styled space and no `█`/`░` in frame | View | `TestBackend` | `cargo test --lib ui::view::tests::detail_header_empty_visible_list` |
| A CJK change name keeps the header inside its region at both mandated widths | `header_row("日本語の変更名前です", …)` at 78/58; assert `columns` exact, cell order, `chars().count() < columns()` | Unit | real `columns`, `truncate_columns` | `cargo test --lib ui::detail::tests::header_cjk_name` |
| The header reaches the buffer without crossing the region border | Render CJK-named change at 120x20 and 60x20; assert span, five `█`, col 59 space, cols 39/41 spaces and col 40 `│` | View | `TestBackend` | `cargo test --lib ui::view::tests::detail_header_cjk_within_region` |
| The header is total over adversarial names at every width | Five names × four `Progress` × widths 0..=130; assert no panic, exact `columns`, band order at 26/25/13/12/7/6/1, no gauge at `total == 0` | Unit | real `columns` | `cargo test --lib ui::detail::tests::header_total_over_adversarial` |
| The bar's rendered output does not move | `progress_bar` over five `Progress` × widths 0..=130; plus literal 78/58 expectations for 4-of-9 (30 `█` of 68, 21 of 48) | Unit | real `gauge_of`, `progress_cell` | `cargo test --lib ui::tasks::tests::progress_bar_output_unchanged` |
| The header's gauge and the bar's gauge agree about the same change | `gauge_of(p, 12)` as a substring of `header_row(…)`, space-bounded, at 78/58 for 4-of-9, 7-of-7, 0-of-7 | Unit | real `gauge_of`, real `header_row` | `cargo test --lib ui::tasks::tests::gauge_agrees_with_header` |
| The gauge is full exactly when the change is complete, at the header's width too | `gauge_of(…, 12)` for five `Progress`; assert length 12 and `no ░` iff `is_complete()` | Unit | real `gauge_of` | `cargo test --lib ui::tasks::tests::gauge_full_iff_complete_at_twelve` |
| The promoted function is total at both guard values | `gauge_of(p, 0)` and `gauge_of(0-of-0, 12)`; plus `progress_bar` 0-of-0 at 78, 58, 3, 2 | Unit | real `gauge_of`, real `progress_bar` | `cargo test --lib ui::tasks::tests::gauge_is_total_at_guards` |

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
- **Existing test expectations in `src/ui/detail.rs` and `src/ui/view.rs` name the old header
  strings and will go red** → Expected and in scope: updating them is this change's work, not
  collateral. The scenario *Below the full-form band the header is byte-identical to the
  pre-gauge grammar* is the guard that the updates were confined to the widths that should
  have moved.
- **A `.chars()`-based width slipping into the new arithmetic would be silently correct on
  ASCII fixtures and wrong elsewhere** → `COLWIDTH` sweeps `src/ui/detail.rs`; the CJK
  scenarios assert `chars().count() < columns()`, which a char-counting implementation fails.
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
