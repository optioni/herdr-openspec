## MODIFIED Requirements

### Requirement: A point in the frame resolves to exactly one zone

`ui::layout::zone(area: Rect, route: Route, column: u16, row: u16) -> Zone` SHALL map a
terminal coordinate to the part of the frame drawn there, deriving the geometry through the
same `split_frame`, `split_body`, `interior`, and `split_detail` the draw path uses and
holding no arithmetic of its own beyond a rectangle containment test. It SHALL be pure —
no filesystem, process, environment, network, or standard-I/O API — and total: every
`Rect`, every `Route`, and every `(column, row)` pair including `(0, 0)` and
`(u16::MAX, u16::MAX)` returns a `Zone` and none panics.

`Zone` SHALL have exactly six variants:

| Variant | Meaning |
|---|---|
| `ListRow { interior: Rect, row: u16 }` | A row of the list region's interior. `interior` is that interior's own rectangle and `row` is the offset of the addressed row below its first interior row |
| `List` | The list region, but not one of its interior rows — its border |
| `DetailTab { bar: Rect, column: u16 }` | The detail region's tab-bar row. `bar` is that row's own rectangle and `column` is the offset of the addressed column right of its first column |
| `DetailRow { content: Rect, row: u16 }` | A row of the detail region's **content area** — the rectangle `split_detail` returns third, below the header and tab-bar rows. `content` is that area's own rectangle and `row` is the offset of the addressed row below its first |
| `Detail` | The detail region, anywhere but the tab-bar row and the content area: its border and its header row |
| `Outside` | The frame's header row, its footer row, or a point outside the frame entirely |

`DetailRow` is this change's addition, and `Detail` narrows by exactly the content area to
make room for it. `artifact-folds` gives a multi-file artifact's content clickable section
header rows, and `mouse-input` resolves a press there to the row it landed on; without a
variant carrying the content area's own rectangle, `mouse_action` would have to recompute
`split_detail` at the call site, which is the second derivation `ListRow` and `DetailTab`
already exist to avoid.

`ListRow`, `DetailTab`, and `DetailRow` SHALL each carry the rectangle the zone was derived
from rather than only an offset, so the caller that resolves the offset to a row, a tab, or a
section uses the very geometry the hit test used. Recomputing the interior at the call site
would be a second derivation of the same rectangle, and the two could drift.

Resolving a `DetailRow` to a section, and deciding whether a press on it means anything at
all, SHALL NOT be `zone`'s work: `zone` answers where the pointer is, and
`ui::detail::section_at` answers what is drawn there. A content area of zero height
contributes no `DetailRow` at any point, and a row past the content area's last row is
`Outside` or `Detail` by the same containment test every other variant uses — never a
`DetailRow` with an out-of-range offset.

`zone` SHALL respect the route below the breakpoint exactly as `split_body` does: at
`LayoutMode::Narrow` only the routed region exists, so every point in the body resolves to
that region's zones and none to the other's. At `LayoutMode::Wide` both regions exist at
both routes, and the route changes nothing about the mapping.

`zone` SHALL name no ratatui widget, no crossterm type, and no mouse type: it takes two
integers, which is what keeps it in the pure view set and testable with no event at all.

#### Scenario: The zones tile the frame at 120 columns

- **WHEN** `zone` is called at a 120x40 frame, at `Route::List` and again at
  `Route::Detail`, for a point on the frame's header row, a point on its footer row, the
  list region's top-left border cell, the list interior's first row, the list interior's
  last row, the divider column 40's border cell, the detail interior's header row, the
  detail interior's tab-bar row, the detail interior's first content row, the detail
  interior's last content row, and column 200
- **THEN** the results are `Outside`, `Outside`, `List`, `ListRow` with `row` 0, `ListRow`
  with `row` equal to the interior's last index, `Detail`, `Detail`, `DetailTab` with
  `column` 0, `DetailRow` with `row` 0, `DetailRow` with `row` equal to the content area's
  last index, and `Outside`, at both routes
- **AND** every `ListRow`'s `interior` equals `interior(split_body(body, route).0.unwrap())`,
  every `DetailTab`'s `bar` equals `split_detail(interior(detail_area)).1`, and every
  `DetailRow`'s `content` equals `split_detail(interior(detail_area)).2`, each computed
  independently in the test
- **AND** the detail region's border cells and its header row still resolve to `Detail`, so
  the narrowing took exactly the content area and nothing else

#### Scenario: Below the breakpoint only the routed region has zones

- **WHEN** `zone` is called at a 60x20 frame at `Route::List` for the interior's first row,
  and then at `Route::Detail` for the same point
- **THEN** the first is a `ListRow` and the second is a `Detail`, `DetailTab`, or `DetailRow`
- **AND** no point anywhere in the 60-column body resolves to a list zone at `Route::Detail`,
  and none resolves to a detail zone at `Route::List`
- **AND** at `Route::Detail` the narrow frame's first content row resolves to `DetailRow` with
  `row` 0, so the new variant exists on both sides of the breakpoint

#### Scenario: The breakpoint is exact for the hit test too

- **WHEN** `zone` is called for the same body point at frame widths 99, 100, and 101 at
  `Route::Detail`
- **THEN** 99 resolves through the narrow layout and 100 and 101 through the wide one,
  agreeing with `layout::mode` at each width

#### Scenario: Degenerate frames resolve without panicking

- **WHEN** `zone` is called at frame areas `0x0`, `1x1`, `2x2`, `3x3`, and `120x2` — the
  heights `split_frame` branches on explicitly — for every `(column, row)` pair inside the
  area and for `(u16::MAX, u16::MAX)`, at both routes
- **THEN** every call returns a `Zone` and none panics
- **AND** a frame with no body resolves every point to `Outside`, since there is no region
- **AND** no call returns a `DetailRow` whose `content` has zero width or zero height, so a
  detail region too short to hold a content area contributes no clickable row at all

#### Scenario: The hit test agrees with what was drawn

- **WHEN** a dashboard with three active and three archived changes is rendered into a
  `TestBackend` at 120x40 and again at 60x20, and every cell of the resulting buffer is
  classified by `zone`
- **THEN** every cell `zone` reports as `ListRow` holds a character from `list::rows`' own
  output for that row, and every cell it reports as `List` or `Detail` at a region boundary
  holds a border character
- **AND** every cell `zone` reports as `DetailRow` holds a character from
  `ui::detail::content_lines`' own output for that row, resolved through the same offset the
  draw path used, so the new variant is checked against the drawn pixels rather than only
  against the geometry
- **AND** no cell of the drawn buffer is classified as belonging to a region the draw path
  did not draw

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
interior is blank on every frame, because nothing filled `detail.sections`, is what this change
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
  selected artifact is a single section of thirty lines each 200 characters long, is rendered at 60x20 and at
  120x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 1 through 18
  is a box-drawing character
- **AND** in the 120-column buffer every cell of columns 0, 39, 40, and 119 in rows 1
  through 18 is a box-drawing character, so a 38-column row neither ran into the divider nor
  into the detail region, and neither a 78-column header, a 78-column tab bar, nor a
  78-column markdown line ran into the divider or past the frame
- **AND** the same holds at `Route::Detail` at 60x20, where the detail region is the only
  one drawn
