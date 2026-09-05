# list-selection Specification

## Purpose
TBD - created by archiving change list-view. Update Purpose after archive.

## Requirements

### Requirement: One change is selected, addressed by index into the visible list

`ui::app::Dashboard` SHALL carry a `selected: usize` field indexing the **visible list** —
the visible active changes followed by the visible archived changes, in the order
`change-rows` emits them, with `list-filtering`'s query applied. `ui::load` SHALL start it
at `0`.

`selected` SHALL be clamped to `visible.len().saturating_sub(1)` by `Dashboard::apply` on
every action that can change either the selection or the visible list, so no code path can
leave it addressing a change that is not shown. When the visible list is empty, `selected`
SHALL be `0` and no row SHALL carry a selection marker.

The actions that move the selection are `Next` and `Prev` — renamed from `SelectNext` and
`SelectPrev` by `markdown-viewer` — and they move it **only while `route` is `Route::List`**.
At `Route::Detail` the same two actions scroll the detail content instead, per
`detail-scroll`, and leave `selected` untouched. The landed wording left the route
unqualified, which was true only while the detail region had nothing to scroll.

Neither the separator row nor a problem row nor a message row SHALL be selectable: the
index addresses changes, not rows.

The selected change's row SHALL carry `>` in the interior's first column and SHALL be drawn
with `Modifier::BOLD` set on every one of its cells; every other row SHALL carry a space in
that column and SHALL NOT have `Modifier::BOLD` set.

#### Scenario: The first change is selected on startup at both widths

- **WHEN** a `Dashboard` built by `ui::load` over a scratch repository with three active
  changes is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row's first column is `>` and every other
  interior row's first column is a space
- **AND** in both buffers every cell of the first interior row reports `Modifier::BOLD`
  set, and no cell of the second interior row does

#### Scenario: `j`, `k`, and the arrows move the selection

- **WHEN** a `Dashboard` at `Route::List` with three active changes and `selected` 0 is
  given, in turn, the action for a Press of `Char('j')`, then a Press of `Down`, then a
  Press of `Char('k')`, then a Press of `Up`
- **THEN** `selected` is 1, then 2, then 1, then 0
- **AND** rendering the dashboard at 120x20 and at 60x20 after the second action puts the
  `>` marker on the third interior row in both buffers, and `Modifier::BOLD` on that row's
  cells rather than the first's
- **AND** `detail.scroll` is `0` throughout, so the list route's keys never touched the
  detail offset

#### Scenario: Selection clamps at both ends rather than wrapping

- **WHEN** a `Dashboard` at `Route::List` with three active changes and no archived changes
  is given four consecutive `Next` actions and then four consecutive `Prev` actions
- **THEN** `selected` is 2 after the four `Next` actions — never 3 and never 0 — and 0
  after the four `Prev` actions
- **AND** rendering at 120x20 and at 60x20 after the four `Next` actions puts the marker on
  the third interior row in both, so the clamp is visible and not merely arithmetic

#### Scenario: Selection crosses the separator into the archived rows

- **WHEN** a `Dashboard` at `Route::List` with one active change `fix-empty-basket` and two
  archived changes `add-auth` and `legacy-cleanup` is given two `Next` actions
- **THEN** `selected` is 2, addressing `legacy-cleanup`
- **AND** rendering at 120x20 and at 60x20 puts the `>` marker on the `legacy-cleanup` row
  in both, and the separator row's first column is a space and its cells are not bold, so
  the separator was stepped over rather than selected

#### Scenario: Navigation over an empty visible list is inert

- **WHEN** a `Dashboard` at `Route::List` whose `changes` is `changes::empty_set()` is given
  a `Next` action and then a `Prev` action
- **THEN** `selected` is 0 after both, and neither panics
- **AND** rendering at 120x20 and at 60x20 shows the `No changes yet` message row with a
  space — not `>` — in the interior's first column, and no bold cell anywhere in the
  interior
### Requirement: The visible slice follows the selection

`ui::layout::viewport(rows: usize, cursor: usize, height: u16) -> usize` SHALL return the
index of the first row to draw, as a pure total function of its three arguments. It SHALL
return `0` when `height` is `0` or when `rows` is at most `height`. Otherwise it SHALL
return `min(cursor.saturating_sub(height / 2), rows - height)`, so the selected row is
always inside the drawn slice, the slice never runs past the last row, and the value is
derived from the current frame on every draw rather than stored on `Dashboard`.

`ui::view::render` SHALL draw the rows from that offset, at most `height` of them, into the
interior. `cursor` SHALL be the index within the emitted row vector of the row carrying the
selection — not `selected` itself, since problem rows, the separator, and message rows
shift it.

Storing an offset on `Dashboard` is forbidden for the same reason `LayoutMode` is: the
interior height is a property of the current frame, and a stored offset would be stale
after a resize.

#### Scenario: A selection past the interior scrolls the slice at both widths

- **WHEN** a `Dashboard` holding thirty active changes named `change-00` through
  `change-29`, with `selected` 20, is rendered at 120x20 and at 60x20 — an interior of
  sixteen rows in both
- **THEN** in both buffers the first interior row is the `change-12` row and the last is
  the `change-27` row, because `layout::viewport(30, 20, 16)` is `min(20 - 8, 30 - 16)` = 12
- **AND** in both buffers the `change-20` row carries the `>` marker and bold cells, so the
  selection is inside the drawn slice

#### Scenario: The last change is reachable and the slice stops at the end

- **WHEN** the same thirty-change dashboard with `selected` 29 is rendered at 120x20 and at
  60x20
- **THEN** in both buffers the first interior row is the `change-14` row and the last is
  the `change-29` row, because `layout::viewport(30, 29, 16)` clamps `29 - 8` to `30 - 16` = 14
- **AND** in both buffers the last interior row carries the `>` marker, and no interior row
  is blank, so the slice never runs past the final row

#### Scenario: The viewport is exact at its boundaries

- **WHEN** `ui::layout::viewport` is called with `(0, 0, 16)`, `(16, 15, 16)`, `(17, 0, 16)`,
  `(17, 8, 16)`, `(17, 9, 16)`, `(17, 16, 16)`, and `(30, 20, 0)`
- **THEN** it returns `0`, `0`, `0`, `0`, `1`, `1`, and `0` respectively
- **AND** rendering a seventeen-change dashboard with `selected` 9 at 120x20 and at 60x20
  shows `change-01` as the first interior row in both, so the boundary is rendered and not
  only computed

#### Scenario: A resize changes the slice on the next frame

- **WHEN** one `Terminal<TestBackend>` is created at 120x20, the thirty-change dashboard
  with `selected` 20 is drawn, the backend is resized to 120x12, and a second frame is drawn
  from the **same** unchanged `Dashboard`
- **THEN** the first buffer's first interior row is the `change-12` row and the second
  buffer's is the `change-16` row, because the interior height fell from 16 to 8
- **AND** `Dashboard` exposes no field naming a scroll offset, a first visible row, or an
  interior height
