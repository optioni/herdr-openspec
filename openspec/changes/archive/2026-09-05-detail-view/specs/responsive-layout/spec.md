## MODIFIED Requirements

### Requirement: The routed region is emphasised and region interiors are left empty

The region the dashboard's route names SHALL have its border drawn with `Modifier::BOLD`
set; the other region, when drawn, SHALL have its border drawn without it.

Neither region's interior is left blank unconditionally any longer. The **list** region's
interior is owned by `change-rows`, with `list-selection` owning which slice is drawn. The
**detail** region's interior is owned by `detail-header` (its first row), `artifact-tabs`
(its second row), and `artifact-content` and `detail-scroll` (the content area below them),
divided by `layout::split_detail`.

The detail interior is blank on a frame **exactly when `Dashboard::visible()` is empty** — no
repository, no changes, or a `/` filter matching none. That is the whole of the blank case
from `detail-view` onward: with a change selected, the region always carries at least a
header, a tab bar, and one content line, because `ui::detail::content_lines` returns
`No content yet` rather than nothing. The earlier statement that the production pane's detail
interior is blank on every frame, because nothing set `detail.source`, is what this change
retires: `detail-view` is the change that was named there as the one that would.

When the interior **is** blank, every cell of it is a space whose `Style` equals
`ratatui::buffer::Cell::default().style()`. The comparison is against `Cell::default().style()`
and **not** against `Style::default()`: `ratatui-crossterm` re-enables the `underline-color`
feature through its own defaults, so an untouched cell's style is
`fg(Reset).bg(Reset).underline_color(Reset)`, which equals neither `Style::default()` nor
`Style::reset()`. Comparing against the constructible value is what catches a `Block::style`
being set where `Block::border_style` was meant.

Content SHALL NOT bleed across a border: no cell of a region's border column or border row
SHALL be overwritten by a list row, a detail header, a tab cell, a problem line, or a
markdown line, at either mandated width.

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
its merge key; the detail interior is blank now because no change is selected, not because
nothing may write there.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  `changes::empty_set()`, and an empty `detail` is rendered at 60x20 and at 120x20
- **THEN** in the 120-column buffer every cell in rows 2 through 17 and columns 41 through
  118 is a space whose `Style` equals `Cell::default().style()` — the detail region is
  untouched because `visible()` is empty
- **AND** in the 120-column buffer row 2, columns 1 through 38, begins `No changes yet`, so
  the list interior is written by `change-rows` rather than left blank
- **AND** in the 60-column buffer row 2, columns 1 through 58, begins `No changes yet`, and
  rows 3 through 17 of columns 1 through 58 are entirely spaces whose `Style` equals
  `Cell::default().style()`, so exactly one message row was drawn
- **AND** the same dashboard with **one** active change added is no longer blank in the
  detail region at 120x20: row 2 columns 41 onward holds that change's header, so the
  blankness asserted above is a property of the empty visible list rather than a constant

#### Scenario: Rows do not overwrite the borders at either width

- **WHEN** a `Dashboard` holding thirty active changes with names long enough to be
  truncated, whose selected change carries twelve artifacts with 40-character ids, and whose
  `detail.source` is thirty lines each 200 characters long, is rendered at 60x20 and at
  120x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 1 through 18
  is a box-drawing character
- **AND** in the 120-column buffer every cell of columns 0, 39, 40, and 119 in rows 1
  through 18 is a box-drawing character, so a 38-column row neither ran into the divider nor
  into the detail region, and neither a 78-column header, a 78-column tab bar, nor a
  78-column markdown line ran into the divider or past the frame
- **AND** the same holds at `Route::Detail` at 60x20, where the detail region is the only
  one drawn
