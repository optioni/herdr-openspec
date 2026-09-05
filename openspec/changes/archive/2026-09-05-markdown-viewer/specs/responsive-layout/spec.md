## MODIFIED Requirements

### Requirement: The routed region is emphasised and region interiors are left empty

The region the dashboard's route names SHALL have its border drawn with `Modifier::BOLD`
set; the other region, when drawn, SHALL have its border drawn without it.

Neither region's interior is left blank unconditionally any longer. The **list** region's
interior is owned by `change-rows`, with `list-selection` owning which slice is drawn. The
**detail** region's interior is owned by `markdown-render` and `detail-scroll`: it holds the
rendered markdown of `Dashboard::detail.source`, and it is blank exactly when that source is
empty. Since nothing in the crate sets `detail.source` yet — `detail-view` supplies it — the
production pane's detail interior is still blank on every frame, and every cell of it is a
space whose `Style` equals `ratatui::buffer::Cell::default().style()`. The comparison is
against `Cell::default().style()` and **not** against `Style::default()`: `ratatui-crossterm`
re-enables the `underline-color` feature through its own defaults, so an untouched cell's
style is `fg(Reset).bg(Reset).underline_color(Reset)`, which equals neither
`Style::default()` nor `Style::reset()`. Comparing against the constructible value is what
catches a `Block::style` being set where `Block::border_style` was meant.

Content SHALL NOT bleed across a border: no cell of a region's border column or border row
SHALL be overwritten by a list row or by a markdown line, at either mandated width.

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

The scenario's name is kept verbatim from `tui-shell` because a delta's scenario headers are
its merge key; the detail interior is blank now because its source is empty, not because
nothing may write there.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  `changes::empty_set()`, and an empty `detail.source` is rendered at 60x20 and at 120x20
- **THEN** in the 120-column buffer every cell in rows 2 through 17 and columns 41 through
  118 is a space whose `Style` equals `Cell::default().style()` — the detail region is
  untouched because its source is empty
- **AND** in the 120-column buffer row 2, columns 1 through 38, begins `No changes yet`, so
  the list interior is written by `change-rows` rather than left blank
- **AND** in the 60-column buffer row 2, columns 1 through 58, begins `No changes yet`, and
  rows 3 through 17 of columns 1 through 58 are entirely spaces whose `Style` equals
  `Cell::default().style()`, so exactly one message row was drawn

#### Scenario: Rows do not overwrite the borders at either width

- **WHEN** a `Dashboard` holding thirty active changes with names long enough to be
  truncated, and a `detail.source` of thirty lines each 200 characters long, is rendered at
  60x20 and at 120x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 1 through 18
  is a box-drawing character
- **AND** in the 120-column buffer every cell of columns 0, 39, 40, and 119 in rows 1
  through 18 is a box-drawing character, so a 38-column row neither ran into the divider nor
  into the detail region, and a 78-column markdown line neither ran into the divider nor
  past the frame

## ADDED Requirements

### Requirement: The detail region's two mandated interior widths are 78 and 58

The detail region's interior width SHALL be **78** at a 120-column frame — the wide layout's
`Constraint::Min(0)` column, 80 columns, less two border columns — and **58** at a 60-column
frame in the detail route, where the region is the whole 60-column body less two border
columns. Both interiors SHALL be **16 rows** at a 20-row frame.

These two widths are frozen here for `detail-view` and `tasks-tab` to inherit, exactly as
`change-rows`' 38 and 58 were frozen by `list-view`. Every test of `ui::markdown` SHALL name
both, and a source check SHALL enforce that with a floor on the number of tests found, on
the same terms and with the same stated limits as the check over `src/ui/list.rs`.

Because the wide layout's detail column is `Min(0)`, 78 is the width at the mandated frame
size and not a constant of the layout: every column gained beyond 120 goes to the detail
region. Nothing SHALL depend on 78 other than the expectations of tests rendered at 120.

#### Scenario: The detail interior is 78 columns at 120 and 58 at 60

- **WHEN** `layout::split_frame` and `layout::split_body` are applied to `Rect::new(0, 0,
  120, 20)` with `Route::Detail`, and the resulting detail rectangle is passed to
  `layout::interior`
- **THEN** the interior is `Rect::new(41, 2, 78, 16)`
- **AND** the same applied to `Rect::new(0, 0, 60, 20)` with `Route::Detail` gives
  `Rect::new(1, 2, 58, 16)`
- **AND** at `Rect::new(0, 0, 60, 20)` with `Route::List` there is no detail rectangle at
  all, so the narrow list route has no detail interior to be 58 columns wide

#### Scenario: Every markdown test names both of its two interior widths

- **WHEN** every `#[test]` in `src/ui/markdown.rs` is inspected for the bare literals `78`
  and `58`, with a floor on the number of tests found
- **THEN** every one of them names both, and the floor is met, so a gutted file fails rather
  than passing with nothing to check
- **AND** the check is a floor and not a proof — a `78` in a comment satisfies it — and is
  paired with a counted filtered run, because `cargo test` exits 0 when a filter matches
  nothing
- **AND** the number scan does not see a suffixed literal such as `78u16`, so it fails
  closed and the widths are written unsuffixed
