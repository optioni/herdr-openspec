## ADDED Requirements

### Requirement: A point in the frame resolves to exactly one zone

`ui::layout::zone(area: Rect, route: Route, column: u16, row: u16) -> Zone` SHALL map a
terminal coordinate to the part of the frame drawn there, deriving the geometry through the
same `split_frame`, `split_body`, `interior`, and `split_detail` the draw path uses and
holding no arithmetic of its own beyond a rectangle containment test. It SHALL be pure —
no filesystem, process, environment, network, or standard-I/O API — and total: every
`Rect`, every `Route`, and every `(column, row)` pair including `(0, 0)` and
`(u16::MAX, u16::MAX)` returns a `Zone` and none panics.

`Zone` SHALL have exactly five variants:

| Variant | Meaning |
|---|---|
| `ListRow { interior: Rect, row: u16 }` | A row of the list region's interior. `interior` is that interior's own rectangle and `row` is the offset of the addressed row below its first interior row |
| `List` | The list region, but not one of its interior rows — its border |
| `DetailTab { bar: Rect, column: u16 }` | The detail region's tab-bar row. `bar` is that row's own rectangle and `column` is the offset of the addressed column right of its first column |
| `Detail` | The detail region, anywhere but the tab-bar row: its border, its header row, or its content area |
| `Outside` | The frame's header row, its footer row, or a point outside the frame entirely |

`ListRow` and `DetailTab` SHALL carry the rectangle the zone was derived from rather than
only an offset, so the caller that resolves the offset to a row or a tab uses the very
geometry the hit test used. Recomputing the interior at the call site would be a second
derivation of the same rectangle, and the two could drift.

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
  detail interior's tab-bar row, the detail interior's first content row, and column 200
- **THEN** the results are `Outside`, `Outside`, `List`, `ListRow` with `row` 0, `ListRow`
  with `row` equal to the interior's last index, `Detail`, `Detail`, `DetailTab` with
  `column` 0, `Detail`, and `Outside`, at both routes
- **AND** every `ListRow`'s `interior` equals `interior(split_body(body, route).0.unwrap())`
  and every `DetailTab`'s `bar` equals `split_detail(interior(detail_area)).1`, computed
  independently in the test

#### Scenario: Below the breakpoint only the routed region has zones

- **WHEN** `zone` is called at a 60x20 frame at `Route::List` for the interior's first row,
  and then at `Route::Detail` for the same point
- **THEN** the first is a `ListRow` and the second is a `Detail` or `DetailTab`
- **AND** no point anywhere in the 60-column body resolves to a list zone at `Route::Detail`,
  and none resolves to a detail zone at `Route::List`

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

#### Scenario: The hit test agrees with what was drawn

- **WHEN** a dashboard with three active and three archived changes is rendered into a
  `TestBackend` at 120x40 and again at 60x20, and every cell of the resulting buffer is
  classified by `zone`
- **THEN** every cell `zone` reports as `ListRow` holds a character from `list::rows`' own
  output for that row, and every cell it reports as `List` or `Detail` at a region boundary
  holds a border character
- **AND** no cell of the drawn buffer is classified as belonging to a region the draw path
  did not draw
