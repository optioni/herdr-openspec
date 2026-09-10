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
| `List` | The list region, but not one of its interior rows — its gutters, its heading row, or its padding row |
| `DetailTab { bar: Rect, column: u16 }` | The detail region's tab-bar row. `bar` is that row's own rectangle and `column` is the offset of the addressed column right of its first column |
| `DetailRow { content: Rect, row: u16 }` | A row of the detail region's **content area** — the rectangle `split_detail` returns third, below the tab-bar row, the rule, and the content padding row. `content` is that area's own rectangle and `row` is the offset of the addressed row below its first |
| `Detail` | The detail region, anywhere but the tab-bar row and the content area: its gutter, its heading row, its padding row, the rule below the tab bar, or the content padding row — and the divider column beside it |
| `Outside` | The frame's footer row, or a point outside the frame entirely |

`Outside` no longer covers a frame header row, because there is no longer one. Every row of
the frame but the last is now a body row and resolves to a region's zone whenever a region is
drawn there.

The **divider column** is in neither region's area, so its zone is decided rather than
derived: it SHALL resolve to `Detail`. It is one column, the reader who lands on it meant one
of the two regions, and the detail is the region whose content scrolls under a wheel — so
giving it to the detail makes a near-miss do something rather than nothing. The choice is
arbitrary in the sense that `List` would also be defensible; it is written down here so it is
one answer rather than an accident.

`DetailRow` is `foldable-spec-sections`' addition, and `Detail` narrows by exactly the
content area to make room for it. `artifact-folds` gives a multi-file artifact's content
clickable section header rows, and `mouse-input` resolves a press there to the row it landed
on; without a variant carrying the content area's own rectangle, `mouse_action` would have to
recompute `split_detail` at the call site, which is the second derivation `ListRow` and
`DetailTab` already exist to avoid.

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
  `Route::Detail`, for a point on the frame's footer row, the list region's heading row, the
  list region's padding row, the list region's left gutter, the list interior's first row,
  the list interior's last row, the divider column 40, the detail region's heading row, the
  detail interior's tab-bar row, the rule row below it, the content padding row, the detail
  interior's first content row, the detail interior's last content row, and column 200
- **THEN** the results are `Outside`, `List`, `List`, `List`, `ListRow` with `row` 0,
  `ListRow` with `row` equal to the interior's last index, `Detail`, `Detail`, `DetailTab`
  with `column` 0, `Detail`, `Detail`, `DetailRow` with `row` 0, `DetailRow` with `row` equal
  to the content area's last index, and `Outside`, at both routes
- **AND** every `ListRow`'s `interior` equals
  `interior(split_body(body, route).0.unwrap(), Gutters::Both)`, every `DetailTab`'s `bar`
  equals `split_detail(interior(detail_area, Gutters::LeftOnly)).0`, and every `DetailRow`'s
  `content` equals `split_detail(interior(detail_area, Gutters::LeftOnly)).2`, each computed
  independently in the test
- **AND** row 0 of the frame resolves to a region rather than to `Outside`, because the
  frame has no header row for it to belong to
- **AND** the detail region's gutter, heading row, padding row, rule row, and content padding
  row still resolve to `Detail`, so the narrowing took exactly the content area and nothing
  else

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
  to be over
- **AND** no call returns a `DetailRow` whose `content` has zero width or zero height, so a
  detail region too short to hold a content area contributes no clickable row at all

#### Scenario: The hit test agrees with what was drawn

- **WHEN** a dashboard with three active and three archived changes is rendered into a
  `TestBackend` at 120x40 and again at 60x20, and every cell of the resulting buffer is
  classified by `zone`
- **THEN** every cell `zone` reports as `ListRow` holds a character from `list::rows`' own
  output for that row, and every cell it reports as `List` or `Detail` in a gutter column
  holds a space, and the divider column holds `│`
- **AND** every cell `zone` reports as `DetailRow` holds a character from
  `ui::detail::content_lines`' own output for that row, resolved through the same offset the
  draw path used, so the new variant is checked against the drawn pixels rather than only
  against the geometry
- **AND** no cell of the drawn buffer is classified as belonging to a region the draw path
  did not draw

### Requirement: The routed region is emphasised and region interiors are left empty

The region the dashboard's route names SHALL have its **heading row** drawn with
`Modifier::BOLD` set and `Modifier::DIM` clear; the other region, when drawn, SHALL have its
heading row drawn with `Modifier::DIM` set and `Modifier::BOLD` clear. This replaces the bold
border that carried the same claim before `pane-chrome` removed the borders.

Neither region's interior is left blank unconditionally any longer. The **list** region's
interior is owned by `change-rows`, with `list-selection` owning which slice is drawn. The
**detail** region's interior is owned by `artifact-tabs` (its first row), the horizontal rule
below that, a padding row, and `artifact-content` and `detail-scroll` (the content area
beneath), divided by `layout::split_detail`. The detail region's **change header** is no
longer part of its interior at all: `detail-header` draws it into the region's heading row,
two rows above.

The detail interior is blank on a frame **exactly when `Dashboard::visible()` is empty** — no
repository, no changes, or a `/` filter matching none — and its heading row is blank on
exactly the same condition. That is the whole of the blank case: with a change selected, the
region always carries a heading, a tab bar, a rule, and at least one content line, because
`ui::detail::content_lines` returns `No content yet` rather than nothing — one section header
row and its body, or a single section's body alone, whichever the selected artifact resolves
to.

When the interior **is** blank, every cell of it is a space whose `Style` equals
`ratatui::buffer::Cell::default().style()`. The comparison is against `Cell::default().style()`
and **not** against `Style::default()`: `ratatui-crossterm` re-enables the `underline-color`
feature through its own defaults, so an untouched cell's style is
`fg(Reset).bg(Reset).underline_color(Reset)`, which equals neither `Style::default()` nor
`Style::reset()`. Comparing against the constructible value is what catches a style being
applied to a whole region where one row was meant. Every cell of a region's **padding row**
SHALL satisfy the same comparison on every frame, blank interior or not: nothing is ever
drawn there.

Content SHALL NOT bleed into a gutter or across the divider: no cell of a region's gutter
column, and no cell of the divider column, SHALL be overwritten by a list row, a heading, a
detail header, a tab cell, a rule, a problem line, or a markdown line, at either mandated
width. The wide layout's detail region has no right gutter, so its interior's last column is
the frame's last column and writing there is correct rather than a bleed.

#### Scenario: The routed region's border is bold and the other's is not

The scenario's name is kept verbatim because a delta's scenario headers are its merge key.
There is no border any more; the claim it made — the routed region is the emphasised one —
is now made by the heading row.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  and one active change is rendered at 120x20
- **THEN** the cell at column 1, row 0 reports `Modifier::BOLD` set
- **AND** the cell at column 42, row 0 reports `Modifier::BOLD` **not** set and
  `Modifier::DIM` set
- **AND** with `route: Route::Detail` and the same size, the two assertions swap, so the
  test discriminates rather than asserting a constant
- **AND** at 60x20 the single drawn region's heading cell at column 1, row 0 reports
  `Modifier::BOLD` set under **both** routes, because the region that is drawn is always
  the routed one below the breakpoint

#### Scenario: Interiors are blank at both widths

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
the detail interior is blank now because no change is selected, not because nothing may
write there.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  `changes::empty_set()`, and an empty `detail` is rendered at 60x20 and at 120x20
- **THEN** in the 120-column buffer every cell in rows 2 through 18 and columns 42 through
  119 is a space whose `Style` equals `Cell::default().style()` — the detail region's
  interior is untouched because `visible()` is empty
- **AND** in the 120-column buffer rows 0 and 1 of columns 42 through 119 are spaces too,
  because the detail region's heading row is its change header and there is no change to
  name, and its padding row is never drawn
- **AND** in the 120-column buffer row 2, columns 1 through 38, begins `No changes yet`, so
  the list interior is written by `change-rows` rather than left blank, two rows below its
  heading
- **AND** in the 60-column buffer row 2, columns 1 through 58, begins `No changes yet`, and
  rows 3 through 18 of columns 1 through 58 are entirely spaces whose `Style` equals
  `Cell::default().style()`, so exactly one message row was drawn into a seventeen-row
  interior
- **AND** the same dashboard with **one** active change added is no longer blank in the
  detail region at 120x20: row 0 columns 42 onward holds that change's header, so the
  blankness asserted above is a property of the empty visible list rather than a constant

#### Scenario: Rows do not overwrite the borders at either width

The scenario's name is kept verbatim because a delta's scenario headers are its merge key.
What a row must not overwrite is now a gutter column and the divider between them.

- **WHEN** a `Dashboard` holding thirty active changes with names long enough to be
  truncated, whose selected change carries twelve artifacts with 40-character ids, and whose
  selected artifact resolves to a **single** section of thirty lines each 200 characters
  long, is rendered at 60x20 and at 120x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 0 through 18
  is a space
- **AND** in the 120-column buffer every cell of columns 0, 39, and 41 in rows 0 through 18
  is a space, and every cell of column 40 in those rows is `│` — so a 38-column row neither
  ran into the divider nor into the detail region, and neither a 78-column heading, a
  78-column tab bar, a 78-column rule, nor a 78-column markdown line ran into the divider or
  past the frame
- **AND** in the 120-column buffer column 119 does carry detail content, because the wide
  detail region has no right gutter, and no cell of any buffer lies past the frame's last
  column
- **AND** the same holds at `Route::Detail` at 60x20, where the detail region is the only one
  drawn, no `│` appears at all, and column 59 is a space because the narrow region takes
  `Gutters::Both`
- **AND** the same dashboard whose selected artifact resolves to **three** files instead of
  one, so its content area opens as three section header rows, satisfies every assertion
  above unchanged: a header row is truncated by the same rule a markdown line is, so it
  reaches no gutter, no divider column, and no cell past the frame's last column
