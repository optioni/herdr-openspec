## Context

`ui::markdown::lines` renders an artifact's markdown into faced, width-parameterised lines
for the detail region. It was built by `markdown-viewer` with
`pulldown_cmark::Options::empty()` — a deliberate scope bound, recorded honestly as a
degraded state: a construct the parser does not model renders as its literal source text.

The corpus this pane exists to display has since been measured. The proposal recorded 89 of
238 `.md` files holding a pipe table, totalling 3,764 table rows; re-measured at this
change's HEAD it is **118 of 309** —
`grep -lE '^\s*\|.*\|\s*$' $(git ls-files 'openspec/*.md' 'openspec/**/*.md') | wc -l`
against `git ls-files 'openspec/*.md' 'openspec/**/*.md' | wc -l`, both run on the tree this
plan was written against. Strikethrough appears zero times, real footnotes zero times, and
task-list items outside `tasks.md` zero times. The tables are where
this repository writes its `SHALL`-exact contracts — the degraded-states table, the gate
tables, the keys table, the palette's own role tables — so the one region meant for reading
prose currently shows pipe-and-dash syntax for the artifacts most worth reading.

Three landed changes set the constraints this design works inside:

- `view-fidelity` made every width in `src/ui/` a **display-column** measurement through the
  single pair `layout::columns` / `layout::truncate_columns`, and `COLWIDTH` sweeps the pure
  view files for any `.chars()`-based measure. A table's column allocation is width
  arithmetic, so it uses those two and adds no third measure.
- `color-palette` moved every `Style` behind `ui::palette`, the crate's only
  `ratatui::style::Color`. A new face is therefore a new `Role`, not a modifier written at a
  render call site. That spec's live text says in as many words that "a strikethrough face
  has no entry at all, because `markdown-render` has no parser support for one yet" — the
  sentence this change makes false and must correct.
- `gate-integrity` and `degraded-states` bound `SPEC.md`'s degraded-states table to
  `tests/degraded-coverage.toml`, where each row names a `proof` that must be a real,
  passing `#[test]` and a `covers` range that must resolve and be executed. Narrowing a row
  therefore means editing four sites in step — `SPEC.md`, the map, the `degraded-coverage`
  spec's own scenario, and the proof test — and `make check` fails if they disagree.

## Goals / Non-Goals

**Goals:**

- Render a GFM pipe table as aligned columns inside the detail region's interior, at every
  width, with a stated line grammar and a stated column-allocation rule.
- Never truncate a cell: a cell too wide for its column wraps inside it.
- Never exceed the region: every line a table emits measures at most the interior width.
- Render `~~strikethrough~~` as a face, through one new `Face` flag and one new palette role.
- Narrow `SPEC.md`'s degraded-states row to the two constructs that remain unmodelled, and
  re-point its executable proof at the narrowed claim.

**Non-Goals:**

- Footnotes and task-list items (measured zero; they stay in the degraded row).
- Any change to a construct that already renders.
- Truncation of table content at any width, under any later width or performance argument.
- A new dependency, a new module, a new key, a new manifest value, or a new config key.
- A second width measure, a second checkbox renderer, or a second `Face`-to-`Style` function.
- Horizontal scrolling, sortable tables, or any interaction: the pane stays read-only with
  exactly one scroll axis.

## Boundaries

The change modifies four capabilities — `markdown-render`, `view-palette`, `detail-scroll`,
and `degraded-coverage` — and touches four source files, all under `src/ui/`, and no module
outside it:

| File | What changes | Pattern it follows |
|---|---|---|
| `src/ui/markdown.rs` | `Options` gains two flags; `Block` gains a table kind; `emit_table` and its allocator are added; `Face` gains `strikethrough`; two `Tag`/`TagEnd` arms leave the wildcard | The module's existing fold-then-lay-out shape: `fold` produces width-independent `Block`s, `layout`/`emit_block` turn them into `Line`s at a width |
| `src/ui/palette.rs` | `Role::Strikethrough` and its one table entry | The existing role table; forced by the exhaustive `match` the totality test iterates |
| `src/ui/view.rs` | One `style_for` fold step | The existing `Style::patch` fold |
| `src/ui/tasks.rs` | `heading_line` names the seventh `Face` field | Nothing new — it is a compile error until it does |

**No process spawn is added anywhere.** `src/cli.rs` remains the crate's only spawner and
this change does not touch it, nor `src/agents.rs`, `src/launch.rs`, or `src/open.rs`.
`NOSPAWN-GREP` and `LAUNCHSEAM` are unaffected.

**No I/O is added to a view.** All four files stay pure: `ui::markdown` still names no
`ratatui` type and no filesystem, process, environment, network, or standard-I/O API, and
`ui::palette` stays a constant table rather than a resolver. `NOIO-VIEW`'s nine-file `PURE`
list and `COLWIDTH`'s eight-file one are unchanged — no module is added or removed.

`pulldown_cmark` stays named only in `src/ui/markdown.rs` (`MDSEAM`), and
`ratatui::style::Color` only in `src/ui/palette.rs` (`PALETTE`). The table allocator measures
through `layout::columns` and wraps through `layout::truncate_columns`, so `COLWIDTH`'s sweep
for a `.chars()` measure stays clean.

The `Change` type is **not** altered. Neither `changes::from_files` nor `changes::from_cli`
is touched, and no agreement between them is at stake: this change is downstream of both,
operating on the text of an artifact after it has been read.

## Contracts

Two public shapes change, both **additively**, and both have exactly one consumer inside this
crate:

- `ui::markdown::Face` gains `pub strikethrough: bool`. Its consumers are
  `ui::view::style_for` and `ui::tasks::heading_line`. `Face` keeps deriving `Default`, so
  every `..Face::plain()` site compiles unchanged; `heading_line` spells its fields out and is
  a **compile error** until it names the new one, which is the point of putting the flag on
  the struct rather than beside it.
- `ui::palette::Role` gains `Strikethrough`. Its consumer is `ui::view::style_for`. The
  exhaustive `match` in `palette::style` and the exhaustive role list its totality test
  iterates both fail to compile until the variant is answered.

No error surface changes: `lines` is total and infallible, and stays so. There is no
pagination or streaming. No consumer outside the crate exists — the binary is the only
consumer, and it has no stable API.

Two behavioural contracts change for readers rather than callers, and both are stated in the
specs: a table's rendered line count no longer equals its source row count (a wrapped cell
makes one row several lines), and a table row line carries interior padding, unlike every
other line kind. `Dashboard::normalise_scroll` already clamps against **rendered** line count
via `content_lines`, so neither needs a loop change.

## Persistence and Rollout

- **Migration:** none. No stored state, no format, no schema.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. The `artifact-content` cache is keyed on
  `(change directory, tab)` and is invalidated by a tab switch, a selection change, or an
  adopted refresh; nothing about it depends on how the text is rendered.
- **Index rebuild:** none.
- **Authorization:** none. The pane is read-only and this change writes nothing.
- **Observability:** none. No logging surface exists in this crate.
- **Deployment:** `make build` rebuilds the release binary the pane runs; `herdr plugin link
  .` is unaffected. No manifest change, so no Herdr-side action.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem (`openspec/` tree, artifact files) | not reached — the view tier builds a `Dashboard` in memory and `detail.source` is set directly | not reached |
| `openspec` binary (`OpenspecCli`) | not reached | not reached |
| Herdr socket (`HerdrCli`, agent poll, launcher) | not reached | not reached |
| Terminal (crossterm raw mode, alternate screen) | **replaced** — `ratatui::backend::TestBackend` at 120x20 and 60x20, never the real terminal | not reached |
| `pulldown_cmark` | **real** — the parser is the subject; no fake exists or is wanted | **real** |
| `ratatui` `Buffer` / `Style` / `Modifier` | **real**, through `TestBackend` | **real** for `palette::style` and `style_for` assertions; `ui::markdown` names no `ratatui` type and asserts on `Face` alone |
| `ui::palette` | **real** — a render test compares a cell's style against `palette::style(role)` rather than a literal | **real** |
| Clock, threads, channels, watcher, refresh worker | not reached — this change adds nothing to the render loop | not reached |
| `scripts/gates/*.sh` (`MDSEAM`, `PALETTE`, `MDWIDTHS`, `COLWIDTH`, `NOIO-VIEW`, `NODEFAULT-UI`) | **real**, run by `make gates` and by `tests/gate_controls.rs` against a planted defect in a scratch copy of the tree | n/a |
| `tests/degraded-coverage.toml` ↔ `SPEC.md` binding | **real**, run by `tests/degraded_coverage.rs` inside `cargo test` | n/a |

## Test Strategy

Three tiers, all inside `cargo test` except the gate scripts, which `make gates` also runs:

- **unit** — `#[cfg(test)] mod tests` inside `src/ui/markdown.rs` and `src/ui/palette.rs`,
  asserting on `Line`/`Segment`/`Face` and on `Style` values. Every test in
  `src/ui/markdown.rs` must name both `58` and `78` **unsuffixed** (`MDWIDTHS`, which has no
  exemption list).
- **view** — `#[cfg(test)] mod tests` inside `src/ui/view.rs`, rendering into a `TestBackend`
  at **120x20 and 60x20** and asserting on buffer cells, their `Modifier`s, and their
  foregrounds compared against `palette::style`.
- **gates** — `scripts/gates/*.sh` under `make gates`, each bound to a planted defect in
  `tests/gate-controls.toml` and executed by `tests/gate_controls.rs`.

**This change does take an outer-loop acceptance test.** It is the view-tier scenario "A
table reaches the buffer aligned and inside the region", and it is written **first**, RED,
before any parser or allocator work: it is the only test that proves the whole path —
`Options` flag, fold, allocation, wrap, alignment, padding, face, palette, draw — reaches a
real buffer without touching a border. Every unit test below sits inside it.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| markdown-render :: A paragraph is word-wrapped, differently at the two mandated widths | `paragraph_wraps_at_58_and_78` (unchanged; regression guard that the new `Options` flags do not reflow prose) | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::paragraph_wraps` |
| markdown-render :: An empty source and a zero width each produce no lines | `empty_source_and_zero_width_produce_no_lines` (unchanged) | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::empty_source` |
| markdown-render :: No line exceeds the width it was given | `no_line_exceeds_the_width_it_was_given`, its `composite_fixture` extended with a three-column table and a struck run | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::no_line_exceeds` |
| markdown-render :: A wide-character document wraps by columns at both mandated widths | `a_wide_character_document_wraps_by_columns_at_both_mandated_widths` (unchanged) | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_wide_character` |
| markdown-render :: Rendering is total over arbitrary input | `lines_is_total_over_arbitrary_input`, its `pathological` array extended with the six new adversarial sources | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::lines_is_total` |
| markdown-render :: A block quote prefixes every one of its lines | `a_block_quote_prefixes_every_line` (unchanged) | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_block_quote` |
| markdown-render :: A thematic break fills the interior at both widths | `a_thematic_break_fills_the_width` (unchanged) | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_thematic_break` |
| degraded-coverage :: A footnote and a task-list item each render as literal source | the same `unmodelled_constructs_render_as_source`, plus `tests/degraded_coverage.rs` proving `SPEC.md`'s narrowed row and the map's `condition` still match byte-for-byte | unit + contract | pulldown_cmark real, `SPEC.md` real | `cargo test --all-features degraded_coverage` |
| markdown-render :: A footnote and a task-list item still render as their literal source text | `unmodelled_constructs_render_as_source`, narrowed to two sources; its `ui::tasks::lines` carve-out leg kept | unit | pulldown_cmark real, `ui::tasks` real | `cargo test --all-features ui::markdown::tests::unmodelled_constructs` |
| markdown-render :: A table that fits renders as aligned columns at both mandated widths | `a_table_that_fits_renders_as_aligned_columns` (replaces `a_table_renders_as_literal_source_rows`) | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_table_that_fits` |
| markdown-render :: A cell wider than its column wraps rather than being truncated | `a_wide_cell_wraps_within_its_column` | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_wide_cell_wraps` |
| markdown-render :: A table too wide for the region spends its columns on the narrow ones | `a_wide_table_allocates_max_min_fairly` | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_wide_table_allocates` |
| markdown-render :: Alignment markers pad the cell on the side they name | `alignment_markers_pad_the_side_they_name` | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::alignment_markers` |
| markdown-render :: A ragged table renders every declared column and drops no header column | `a_ragged_table_keeps_its_declared_columns` | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_ragged_table` |
| markdown-render :: A region too narrow for the pipe grammar renders one cell per line | `a_narrow_region_renders_one_cell_per_line` | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_narrow_region` |
| markdown-render :: A struck run carries the face and composes with the others | `a_struck_run_carries_the_face_and_composes` | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_struck_run_carries` |
| markdown-render :: A struck run split across a wrap keeps its face on both lines | `a_struck_run_split_across_a_wrap_keeps_its_face` | unit | pulldown_cmark real | `cargo test --all-features ui::markdown::tests::a_struck_run_split` |
| view-palette :: The palette answers every role with a `Style` | `the_palette_answers_every_role`, its exhaustive `match` extended; `Strikethrough` asserted unequal to every other style | unit | ratatui `Style` real | `cargo test --all-features ui::palette::tests::the_palette_answers` |
| view-palette :: The confinement gate catches a `Color` named outside the palette | `scripts/gates/palette.sh` plus its three recorded controls in `tests/gate-controls.toml` (unchanged; re-run to prove the new role adds no `Color` outside the palette) | gates | real scripts, scratch tree copy | `make gates && cargo test --all-features gate_controls` |
| view-palette :: The palette module reaches no I/O and measures no width | `scripts/gates/noio-view.sh` and `scripts/gates/colwidth.sh` (unchanged; both counts unmoved) | gates | real scripts | `make gates` |
| view-palette :: Each role's modifier set is exactly the table above | `each_roles_modifier_set_is_exactly_the_table`, gaining the `Strikethrough`/`CROSSED_OUT` row and the `not DIM` discrimination | unit | ratatui `Modifier` real | `cargo test --all-features ui::palette::tests::each_roles_modifier` |
| view-palette :: A monochrome reading of the frame is unchanged | `a_monochrome_reading_of_the_frame_is_unchanged`, gaining the appended `~~struck~~` leg | view | TestBackend real, palette real | `cargo test --all-features ui::view::tests::a_monochrome_reading` |
| view-palette :: The coloured set is exactly the table above | `the_coloured_set_is_exactly_the_table`, `Strikethrough` asserted `fg: None, bg: None` | unit | ratatui `Style` real | `cargo test --all-features ui::palette::tests::the_coloured_set` |
| view-palette :: An out-of-range heading level does not panic | `an_out_of_range_heading_level_does_not_panic` (unchanged) | unit | ratatui `Style` real | `cargo test --all-features ui::palette::tests::an_out_of_range` |
| view-palette :: Faces reach the buffer as coloured styles at both mandated widths | `faces_reach_the_buffer_as_coloured_styles`, its source gaining ` and ~~struck~~` | view | TestBackend real, palette real | `cargo test --all-features ui::view::tests::faces_reach_the_buffer_as_coloured` |
| view-palette :: Heading foreground wins over a code span inside it | `heading_foreground_wins_over_a_code_span`, gaining the struck-bold-link face | unit | palette real | `cargo test --all-features ui::view::tests::heading_foreground_wins` |
| view-palette :: A plain face is the default style | `a_plain_face_is_the_default_style`, gaining the `strikethrough == false` assertion | unit + view | palette real, TestBackend real | `cargo test --all-features ui::view::tests::a_plain_face` |
| detail-scroll :: The document fills the detail interior at both mandated widths | `the_document_fills_the_detail_interior` (unchanged regression guard) | view | TestBackend real | `cargo test --all-features ui::view::tests::the_document_fills` |
| detail-scroll :: Faces reach the buffer as styles at both widths | `faces_reach_the_buffer_as_styles`, its source gaining ` and ~~struck~~` and a `CROSSED_OUT` assertion | view | TestBackend real, palette real | `cargo test --all-features ui::view::tests::faces_reach_the_buffer_as_styles` |
| detail-scroll :: A table reaches the buffer aligned and inside the region | `a_table_reaches_the_buffer_aligned` — **the outer-loop acceptance test, written first** | view | TestBackend real, palette real, pulldown_cmark real | `cargo test --all-features ui::view::tests::a_table_reaches_the_buffer` |
| detail-scroll :: An empty source leaves the detail interior blank at both widths | `an_empty_source_leaves_the_detail_interior_blank` (unchanged) | view | TestBackend real | `cargo test --all-features ui::view::tests::an_empty_source_leaves` |
| detail-scroll :: Content never overwrites the detail region's border | `content_never_overwrites_the_detail_border`, gaining the twelve-column 200-character-cell table leg | view | TestBackend real | `cargo test --all-features ui::view::tests::content_never_overwrites` |
| detail-scroll :: A degenerate detail interior draws nothing and does not panic | `a_degenerate_detail_interior_draws_nothing`, gaining the table-source repetition | view | TestBackend real | `cargo test --all-features ui::view::tests::a_degenerate_detail_interior` |

The whole gate set runs as one command: `make check`.

## Decisions

**Decision 1 — `Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH`, and nothing else.**
Alternatives: `Options::all()`, or the GFM bundle. Rejected because every flag turned on is a
construct this module must then render correctly — `ENABLE_FOOTNOTES` and `ENABLE_TASKLISTS`
would silently start emitting `FootnoteReference` and `TaskListMarker` events into a `_ => {}`
wildcard, which is exactly how a construct **vanishes** rather than degrading to literal text.
The two flags are named individually, and the wildcard's own comment is updated to say which
variants the new options can now produce.

**Decision 2 — a table is a third `Block` kind, not a second entry point.**
`Block` currently discriminates a thematic break with `is_rule: bool`. That becomes
`kind: BlockKind { Flow, Rule, Table(Table) }`, where `Table` carries the parsed alignments
and a `Vec<Row>` of cells, each cell a `Vec<Run>`. Alternative: keep `is_rule` and add
`table: Option<Table>` beside it, which makes two mutually exclusive fields both
constructible at once. Alternative: lay tables out in `fold`, which would make `fold`
width-dependent and break the module's one real invariant — `fold` produces width-independent
data, `layout` turns it into lines at a width. The enum is the cheapest shape that keeps that
invariant and makes the impossible state unrepresentable.

**Decision 3 — the pipe grammar keeps its outer pipes.**
`| a | b |`, overhead `3n + 1`. Alternative: drop the outer pipes (`a | b`), saving four
columns and lowering the narrow-fallback threshold. Rejected: with wrapped cells, the outer
pipes are what tell the reader that a continuation line belongs to the table at all, and the
saving is four columns out of 58. The delimiter line is `-` repeated `w[j] + 2` per column so
its pipes fall exactly under the row lines', which is what makes "the columns are aligned" an
assertion a test can make about character offsets rather than about intent.

**Decision 4 — max-min fair column allocation, not proportional.**
Given `avail = width - (3n + 1)` and natural widths `nat[j]`, allocate `min(nat[j], c)` for
the largest `c` that fits, then hand the remainder out by ascending index. Alternative:
proportional shrink (`w[j] = nat[j] * avail / sum(nat)`), which squeezes a 4-column `Gate`
column to 1 to feed a 60-column prose column — the opposite of useful, since the short columns
are the labels a reader scans by. Alternative: fixed equal columns, which wastes the region on
a table of one long and three short columns. Max-min fairness gives every column its natural
width until the budget runs out, then caps only the greedy ones; it is the standard answer and
it is four lines of code.

**Decision 5 — a cell wraps; it is never truncated.**
Already argued in the proposal and recorded there as a **non-goal** rather than only a
decision, so a later width or performance argument cannot reintroduce it quietly. The detail
region has exactly one scroll axis and no horizontal recovery, so a cut cell is content
destroyed. `change-rows` truncates because its grammar is fixed and the name alone identifies
the row; a table cell missing its predicate is a decoration.

**Decision 6 — a header cell carries `strong`, rather than gaining a face of its own.**
The header row reads bold through the existing `Strong` role. Alternative: a `table_header`
face plus a `Role::TableHeader`. Rejected: it would be a second way to say `BOLD`, and
`view-palette` would gain a role that is by construction equal to one it already has — which
its own distinctness scenario would then have to carve an exception around.

**Decision 7 — below `4n + 1` columns, one cell per line.**
The pipe grammar cannot carry fewer than one content column per column, so a threshold is
unavoidable; the all-widths sweep runs at 0, 1, 2, 3, and 10, so the degenerate path is
exercised whether or not it is designed. Alternatives: a `header: value` stacked form (a
genuine second grammar with its own alignment, wrapping, and test surface, for a case that
only fires below nine columns); dropping columns until it fits (destroys content, and
Decision 5 forbids it); rendering the table as literal source (the source text is not
available at layout time — `fold` has already consumed the events — so it would have to be
re-synthesised, at which point it has the same width problem). One cell per line preserves
every character, needs no header lookup, reuses `wrap_prose` unchanged, and is four lines.

**Decision 8 — ragged rows: pad short, drop surplus.**
`n` comes from the delimiter row's alignment list, which is what fixes the delimiter line's
shape. A short row's missing columns render as `w[j]` spaces; a long row's surplus cells have
no column to be drawn in and are dropped. Alternative: widen the table to the longest row,
which would leave the header and delimiter lines shorter than the body and destroy the
alignment the grammar exists for. GitHub itself drops the surplus, so the pane agrees with the
renderer the author was writing for.

**Decision 9 — `strikethrough` is a `Face` field and `Role::Strikethrough` is `CROSSED_OUT`
and uncoloured, folded second.**
A field rather than an enum because faces compose and `Face` is already a struct of flags for
that reason. `CROSSED_OUT` rather than a colour because it is the terminal's own rendering of
exactly this meaning, costs no columns, and degrades to plain text on a terminal that lacks
it — the right failure for a construct whose point is that the text is still legible. No
colour, because the obvious candidate `DarkGray` is this palette's one "no information" grey
and struck text is not that. Folded at position 2, immediately after `Quoted`: carrying no
foreground, it cannot displace a coloured role's colour wherever it sits, so it is placed
beside the other uncoloured always-composing face rather than inserted into the coloured
precedence chain, which stays exactly heading over code over link.

**Decision 10 — the degraded-states row is narrowed, and keeps its `unproven` verdict.**
Four sites move together, because the binding `degraded-states` and `gate-integrity` built
makes any three of them a red build: `SPEC.md`'s row and its rendering-grammar prose both drop
"a table" and "strikethrough" and keep "a footnote, a task-list item";
`tests/degraded-coverage.toml`'s `condition` is edited to match byte-for-byte (the checker
compares them), its `why` is reworded, and its `covers` range is re-pointed at the new
`Options` line; the `degraded-coverage` capability's own scenario is renamed from "A footnote,
strikethrough, and a table each render as literal source" to name only what stays literal; and
`unmodelled_constructs_render_as_source` drops its two departed sources while keeping its
`ui::tasks` carve-out leg. The `verdict` stays `unproven` rather than moving to
`spec-corrected` or `implemented`: those two mean, respectively, that the audit found the row
itself wrong and that the row described behaviour that did not exist. Neither is true of what
**remains** in this row — footnotes and task-list items rendered as literal source before this
change and still do — and the verdict describes the row that remains, not the subjects that
left it. Deleting the row instead was considered and rejected: two constructs still degrade,
and a table with no row for them would be a claim that nothing does.

**Decision 11 — gate floors move up, they are not overridden on the `Makefile` line.**
`MDWIDTHS`'s `MD_MIN` default (currently 26) rises to the re-measured test count in
`scripts/gates/mdwidths.sh` itself, per `AGENTS.md`'s rule that a gate's floor is its own
script default. If `NODEFAULT-UI`'s view-set `SCAN_MIN` moves, that one **is** edited on the
`Makefile` line, because it is the project's one recorded multi-subject exception. Every floor
change is measured on the finished tree and recorded in the task that makes it, never guessed.

## Risks / Trade-offs

- **A table's rendered line count no longer equals its source row count** → Accepted and
  specified. `Dashboard::normalise_scroll` already clamps against `content_lines`' rendered
  count rather than a source count, so no loop change is needed; the detail-scroll spec states
  it explicitly so a later reader does not treat the divergence as a bug.
- **A wide table at 58 columns produces very tall rows** — a 60-column prose cell in a
  four-column table wraps to a dozen lines → Accepted. The alternative is truncation, which
  Decision 5 and the proposal's non-goals both forbid. The reader has a vertical scroll and
  no horizontal one, so height is the axis to spend.
- **`ENABLE_TABLES` changes how existing prose parses** — a paragraph that happens to hold
  pipes and a `---` line could now be read as a table → Mitigated by the unchanged
  `paragraph_wraps_at_58_and_78` and `exactly_one_blank_line_separates_blocks` regression
  guards, and by the composite-fixture sweep, which renders prose and a table in the same
  document. GFM's own table detection requires a delimiter row, so the false-positive surface
  is narrow.
- **`CROSSED_OUT` is not universally supported by terminals** → Accepted, and it is the
  stated reason for choosing it: an unsupported attribute is dropped and the text still reads,
  where an unsupported colour would be approximated and a bracketing glyph would cost columns
  and lie about the source.
- **The allocator is new arithmetic on the width-critical path** → Mitigated by the
  all-widths sweep (0, 1, 2, 3, 10, 58, 78, 200), by the totality scenario's six new
  adversarial tables including a forty-column one and a 500-column CJK cell, and by the
  view-tier border scenario's twelve-column 200-character-cell table, which is the assertion
  that nothing reaches a border.
- **Three files must be edited in step for the degraded row** → Mitigated by the mechanism
  that already exists: `tests/degraded_coverage.rs` fails `make check` when `SPEC.md` and
  `tests/degraded-coverage.toml` disagree, so a half-done narrowing is a red build rather than
  a silent drift.

## Migration Plan

None needed. The change is additive to two in-crate types with no external consumer, alters
no stored state, no file format, no manifest value, and no keybinding. Deploy is `make build`;
rollback is `git revert`, and nothing outside the working tree records that the change
happened.

Ordering matters only inside the change and is fixed by the tasks: the acceptance test first
(RED), then the parser and layout work, then the palette and view arms, then the documents and
the degraded-coverage binding, then the gate floors re-measured on the finished tree.

## Open Questions

None. Every question this change raised — the grammar, the allocation rule, the narrow
fallback, the ragged-row rule, the strikethrough style, the verdict on the narrowed degraded
row — is answered above as a decision with its alternatives recorded.

One **sequencing constraint** rather than a question: this change modifies `view-palette`,
which `color-palette` introduces and has not yet archived. Its delta is written against
`color-palette`'s spec text, so `color-palette` must be archived before this change is
archived. `openspec validate --specs --strict` on the merged tree is what will catch it if
that ordering is broken.
