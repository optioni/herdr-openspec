# responsive-layout Specification

## Purpose
TBD - created by archiving change tui-shell. Update Purpose after archive.

## Requirements

### Requirement: The frame is a header row, a body, and a footer row

`ui::view::render(frame, &Dashboard)` SHALL be a pure function of its two arguments: it
SHALL perform no filesystem, process, environment, network, or terminal I/O, and SHALL
read no clock and no global state. It SHALL divide `frame.area()` vertically into exactly
three regions, in order — a header of `Constraint::Length(1)`, a body of
`Constraint::Min(0)`, and a footer of `Constraint::Length(1)` — and SHALL draw nothing
outside them.

The header SHALL render the literal `OpenSpec` at column 0 with
`Modifier::BOLD` set. The footer SHALL render the key hints `q quit`, `Enter detail`, and
`Esc back` in that order, separated by two spaces, starting at column 0, dropping hints
from the **end** when the remaining width cannot hold the next one whole.

The footer has two further forms, specified by `list-filtering` and restated here because
this requirement owns the row: while `dashboard.filter.active` is set the hints are
**replaced** by the prompt `/`, the query, and `_`, keeping its tail when it overflows;
while the filter is inactive with a non-empty query, `/` and the query become a fourth
hint placed **first** in the list above, dropped last rather than first. Every scenario
below renders a `Dashboard` with an empty, inactive filter, so the three-hint form is what
they assert.

Scenarios in this capability render a `Dashboard` whose `changes` is
`changes::empty_set()` unless they say otherwise. That state is no longer inert: with a
repository root present, `change-rows` renders a `No changes yet` message row into the list
region's interior, and the scenarios below are written so that none of them depends on that
interior being blank.

Degenerate heights SHALL be decided explicitly rather than delegated to the constraint
solver — measured, not assumed: `Layout::vertical([Length(1), Min(0), Length(1)])` at
height 1 gives the single row to the **footer**, so a naive split renders `q quit` where
`OpenSpec` belongs. The explicit branch is: at height 0 `render` SHALL draw nothing; at
height 1 it SHALL draw the header only; at height 2 it SHALL draw the header on row 0 and
the footer on row 1 and no body; at height 3 or more it SHALL use the three-way split
above. `render` SHALL NOT panic at any frame size of at least one column by one row, and
SHALL NOT panic at an interior of zero columns or zero rows, which a one- or two-column
frame produces.

#### Scenario: Header, body, and footer occupy their rows at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` is rendered into a
  `TestBackend` at 60x20, and again at 120x20
- **THEN** in both buffers row 0 column 0 through column 7 spells `OpenSpec`, and the cell
  at (0, 0) reports `Modifier::BOLD` set
- **AND** in both buffers row 19 begins with the exact string
  `q quit  Enter detail  Esc back` at column 0, and every remaining cell of row 19 is a
  space
- **AND** in both buffers row 1 is the top border of a bordered region and row 18 is its
  bottom border, so the body occupies rows 1 through 18 and nothing is drawn in row 0 or
  row 19 by the body

#### Scenario: A one-row frame renders the header and nothing else

- **WHEN** the same `Dashboard` is rendered at 60x1 and at 120x1
- **THEN** neither render panics
- **AND** row 0 spells `OpenSpec` at column 0 in both
- **AND** no box-drawing character and no `q quit` appears in either buffer, so neither the
  body nor the footer was drawn into the header's row

#### Scenario: A two-row frame renders the header and the footer with no body

- **WHEN** the same `Dashboard` is rendered at 60x2 and at 120x2
- **THEN** neither render panics
- **AND** row 0 spells `OpenSpec` and row 1 begins `q quit` at column 0
- **AND** no box-drawing character appears anywhere in either buffer, because the body
  received zero rows

#### Scenario: A one-column frame renders without panicking

- **WHEN** the same `Dashboard` is rendered at 1x1, at 1x20, at 2x20, and — as the
  contrasting controls at the two mandated widths — at 60x20 and 120x20
- **THEN** none of the five panics, including the two whose list region has an interior of
  zero columns
- **AND** the 1x20 buffer's row 0 is the single character `O`, the first character of the
  truncated `OpenSpec` label, so a one-column frame still draws rather than silently
  skipping the header
- **AND** the 1x20 buffer's row 19 is a single space: `q quit` needs six columns, so the
  first hint is dropped whole rather than truncated to `q`
- **AND** the 60x20 and 120x20 buffers both spell `OpenSpec` in columns 0 through 7 and
  both begin row 19 with `q quit`, so the one-column result is a width branch rather than
  the header and footer being absent everywhere

#### Scenario: The footer drops whole hints rather than truncating one

- **WHEN** the same `Dashboard` is rendered at 18x20, at 20x20, at 60x20, and at 120x20
- **THEN** the 18-column footer row is exactly `q quit` followed by twelve spaces —
  `q quit  Enter detail` needs exactly 20 columns, so `Enter detail` and every hint after
  it are dropped whole rather than cut short
- **AND** the 20-column footer row is exactly `q quit  Enter detail`, filling the row with
  no trailing space, which pins the boundary from the other side
- **AND** the 60-column footer row is exactly `q quit  Enter detail  Esc back` — thirty
  characters — followed by thirty spaces, so all three hints fit at the mandated narrow
  width
- **AND** the 120-column footer row is the same thirty characters followed by ninety
  spaces

### Requirement: The 100-column breakpoint decides one region or two

`ui::layout::WIDE_MIN_WIDTH` SHALL be `100`. `ui::layout::mode(width: u16)` SHALL return
`LayoutMode::Wide` when `width >= WIDE_MIN_WIDTH` and `LayoutMode::Narrow` otherwise, and
SHALL be a total function over every `u16`.

At `LayoutMode::Wide` the body SHALL be split horizontally into exactly two regions —
`Constraint::Length(40)` for the change list on the left and `Constraint::Min(0)` for the
artifact detail on the right — so that every column gained beyond 100 goes to the detail
side. Both regions SHALL be drawn, each as a block with all four borders and a title:
`Changes` on the left, `Detail` on the right.

At `LayoutMode::Narrow` the body SHALL hold exactly one region occupying the whole body
width, drawn as a block with all four borders, titled `Changes` when the dashboard's route
is `Route::List` and `Detail` when it is `Route::Detail`. The region that is not routed to
SHALL NOT be drawn at all.

The mode SHALL be derived from the frame area passed to `render` on every draw, and SHALL
NOT be stored on `Dashboard` or captured at startup, so a terminal resized across the
breakpoint changes layout on its next frame with no extra state.

#### Scenario: At 120 columns both regions are drawn with the divider at column 40

- **WHEN** a `Dashboard` with `route: Route::List` is rendered at 120x20, and — as the
  contrasting control at the mandated narrow width — at 60x20
- **THEN** in the 120-column buffer row 1 holds `┌` at column 0, `┐` at column 39, `┌` at
  column 40, and `┐` at column 119
- **AND** row 1 columns 1 through 7 spell `Changes`, and row 1 columns 41 through 46 spell
  `Detail`
- **AND** row 18 holds `└` at column 0, `┘` at column 39, `└` at column 40, and `┘` at
  column 119
- **AND** the 60-column buffer holds exactly one `┌` in the whole buffer, so the second
  region is a width branch rather than something drawn unconditionally

#### Scenario: At 60 columns only the routed region is drawn

- **WHEN** the same `Dashboard` with `route: Route::List` is rendered at 60x20, and — as
  the contrasting control at the mandated wide width — at 120x20
- **THEN** in the 60-column buffer row 1 holds `┌` at column 0 and `┐` at column 59, and no
  other `┌` appears anywhere in the buffer
- **AND** row 1 columns 1 through 7 spell `Changes`
- **AND** the string `Detail` appears in no row of the 60-column buffer
- **AND** the 120-column buffer does contain `Detail`, at row 1 columns 41 through 46, so
  the absence at 60 columns is the breakpoint and not the title being missing

#### Scenario: At 60 columns the detail route replaces the list region

- **WHEN** a `Dashboard` with `route: Route::Detail` is rendered at 60x20
- **THEN** row 1 columns 1 through 6 spell `Detail`
- **AND** the string `Changes` appears in no row of the buffer
- **AND** rendering the same dashboard at 120x20 still shows **both** `Changes` at row 1
  column 1 and `Detail` at row 1 column 41, because the route selects emphasis rather than
  visibility above the breakpoint

#### Scenario: The breakpoint is exact at 99, 100, and 101 columns

- **WHEN** `ui::layout::mode` is called with `0`, `1`, `40`, `60`, `99`, `100`, `101`,
  `120`, and `u16::MAX`
- **THEN** it returns `Narrow` for `0`, `1`, `40`, `60`, and `99`, and `Wide` for `100`,
  `101`, `120`, and `u16::MAX`
- **AND** rendering the same `Dashboard` produces exactly one `┌` in the buffer at 60x20
  and at 99x20, and exactly two — the second at column 40 — at 100x20, at 101x20, and at
  120x20, so both mandated widths and all three boundary widths are rendered, not just
  computed

#### Scenario: The mode follows the current frame, not the startup size

- **WHEN** one `Terminal<TestBackend>` is created at 120x20, a frame is drawn, the backend
  is resized to 60x20, and a second frame is drawn from the **same** unchanged `Dashboard`
- **THEN** the first buffer holds two `┌` characters and the second holds one
- **AND** `Dashboard` exposes no field naming a width, a layout mode, or a column count

### Requirement: The routed region is emphasised and region interiors are left empty

The region the dashboard's route names SHALL have its border drawn with
`Modifier::BOLD` set; the other region, when drawn, SHALL have its border drawn without it.

The **list** region's interior is no longer left blank: `change-rows` owns every cell of it
and `list-selection` owns which slice is drawn. The **detail** region's interior SHALL
still be left blank by this capability — `markdown-viewer` and `detail-view` fill it; until
they land, every cell of it SHALL be a space whose `Style` equals
`ratatui::buffer::Cell::default().style()`, so a later change adding content is a visible
diff rather than an overwrite of something already there. The comparison is against
`Cell::default().style()` and **not** against `Style::default()`: `ratatui-crossterm`
re-enables the `underline-color` feature through its own defaults, so an untouched cell's
style is `fg(Reset).bg(Reset).underline_color(Reset)`, which equals neither
`Style::default()` nor `Style::reset()`. Comparing against the constructible value is what
catches a `Block::style` being set where `Block::border_style` was meant.

Row content SHALL NOT bleed across a border: no cell of a region's border column or border
row SHALL be overwritten by a row, at either mandated width.

#### Scenario: The routed region's border is bold and the other's is not

- **WHEN** a `Dashboard` with `route: Route::List` is rendered at 120x20
- **THEN** the cell at column 0, row 1 reports `Modifier::BOLD` set
- **AND** the cell at column 40, row 1 reports `Modifier::BOLD` **not** set
- **AND** with `route: Route::Detail` and the same size, the two assertions swap, so the
  test discriminates rather than asserting a constant
- **AND** at 60x20 the single drawn region's border cell at column 0, row 1 reports
  `Modifier::BOLD` set under **both** routes, because the region that is drawn is always
  the routed one below the breakpoint

#### Scenario: Interiors are blank at both widths

The scenario's name is kept verbatim from `tui-shell` because a delta's scenario headers
are its merge key; only the **detail** interior is blank now.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  and `changes::empty_set()` is rendered at 60x20 and at 120x20
- **THEN** in the 120-column buffer every cell in rows 2 through 17 and columns 41 through
  118 is a space whose `Style` equals `Cell::default().style()` — the detail region is
  still untouched
- **AND** in the 120-column buffer row 2, columns 1 through 38, begins `No changes yet`, so
  the list interior is written by `change-rows` rather than left blank
- **AND** in the 60-column buffer row 2, columns 1 through 58, begins `No changes yet`, and
  rows 3 through 17 of columns 1 through 58 are entirely spaces whose `Style` equals
  `Cell::default().style()`, so exactly one message row was drawn

#### Scenario: Rows do not overwrite the borders at either width

- **WHEN** a `Dashboard` holding thirty active changes with names long enough to be
  truncated is rendered at 60x20 and at 120x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 1 through
  18 is a box-drawing character
- **AND** in the 120-column buffer every cell of columns 0, 39, 40, and 119 in rows 1
  through 18 is a box-drawing character, so a 38-column row neither ran into the divider
  nor into the detail region

### Requirement: The header names the repository root, shortened from the left when narrow

The header row SHALL render, right-aligned so that its last character sits in the final
column, the repository root's display path when one was found, and the literal
`no repository` when none was. Exactly one blank column SHALL separate the `OpenSpec`
label from the shortened text at minimum.

Let `A` be the header width minus 9 — the eight columns of `OpenSpec` plus one separating
blank — **floored at zero**, so widths below 9 do not underflow the unsigned subtraction.
When the text's character count is at most `A` it SHALL be rendered whole. When it is
longer and `A` is at least 8, it SHALL be rendered as `…` followed by its last `A - 1`
characters. When `A` is below 8 the text SHALL be omitted entirely and only `OpenSpec`
SHALL be drawn; when the width is below 8 the label itself SHALL be truncated to the
columns available. Shortening SHALL count characters, not bytes, and SHALL keep the tail —
the repository's own directory name is what identifies it, and the leading path components
are what a reader can spare.

The shortening rule SHALL be one shared implementation with `change-rows`' no-repository
block, which shortens `searched_from` by the same keep-the-tail rule against the list
region's interior width rather than the header's.

#### Scenario: A path that fits is right-aligned whole at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` — fourteen characters —
  is rendered at 60x20 and at 120x20
- **THEN** the 60-column header row spells `/tmp/demo-repo` in columns 46 through 59, and
  columns 8 through 45 are spaces
- **AND** the 120-column header row spells `/tmp/demo-repo` in columns 106 through 119

#### Scenario: A path too long for the narrow header is shortened from the left

- **WHEN** a `Dashboard` whose repository root is
  `/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here` — seventy
  characters — is rendered at 60x20 and at 120x20
- **THEN** the 60-column header row spells
  `…/openspec-demos/a-rather-long-repository-name-here` in columns 9 through 59, so the
  first character of the shortened text is the ellipsis and the last is the final `e` of
  the directory name
- **AND** the 120-column header row spells the whole seventy-character path in columns 50
  through 119, with no ellipsis anywhere in the buffer — which stays true only because
  that `Dashboard`'s `changes` is `changes::empty_set()` with a repository root present,
  so the list interior holds the fourteen-character `No changes yet` and needs no
  ellipsis of its own

#### Scenario: A header too narrow for any path shows only the label

- **WHEN** the same seventy-character `Dashboard` is rendered at 16x20, at 60x20, and at
  120x20
- **THEN** the 16-column header row is exactly `OpenSpec` followed by eight spaces, and no
  ellipsis appears in that row
- **AND** the 60-column header row carries the shortened, ellipsis-prefixed path in columns
  9 through 59, and the 120-column header row carries the full path in columns 50 through
  119 — so the 16-column omission is a width branch and not the feature being absent
- **AND** no ellipsis appears anywhere in the 16-column buffer: its list interior is
  fourteen columns wide and `No changes yet` is exactly fourteen characters, so the body
  neither truncates nor overflows

#### Scenario: No repository found is named in the header at both widths

- **WHEN** a `Dashboard` built with no repository root — the value `ui::load` produces when
  `resolve::find_repo` reports `NotFound`, with `searched_from` `/tmp/searched-from` — is
  rendered at 60x20 and at 120x20
- **THEN** the 60-column header row spells `no repository` in columns 47 through 59
- **AND** the 120-column header row spells `no repository` in columns 107 through 119
- **AND** neither **header row** names the directory the search started from: row 0 of
  each buffer does not contain `/tmp/searched-from`. The **body** now does, and that is
  `change-rows`' no-repository block — the landed form of this scenario asserted the
  string was absent from the whole buffer, which `list-view` makes false; the assertion
  is narrowed to row 0, which is what the requirement was ever about
