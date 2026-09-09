## ADDED Requirements

### Requirement: The row a point lands on is the row drawn there

`ui::list::row_at(dashboard: &Dashboard, interior: Rect, row: u16) -> Option<RowKind>` SHALL
return the `RowKind` of the row `ui::view::render_list` draws at `interior.y + row`, and
`None` when that terminal row holds no drawn row — an interior shorter than the row offset,
a zero-width or zero-height interior, or a row past the end of what `list::rows` emitted.

`row_at` and `render_list` SHALL derive the first drawn row's index the same way, through
one shared function rather than two copies of the same three lines: the rows are
`list::rows(dashboard, interior.width)`, the cursor is the position of the row whose
`selected` is set (or `0` when none is), and the offset is
`layout::viewport(rows.len(), cursor, interior.height)`. A second derivation could drift
from the first by a resize, a filter edit, or a fold, and a click would then land on a row
the reader is not looking at.

`row_at` SHALL be pure and total: no I/O, no clock, no mutation, and no panic for any
`Dashboard`, any `Rect`, and any `row`.

`RowKind::Problem` and `RowKind::Message` rows SHALL remain unaddressable. `row_at` reports
them faithfully — they are what is drawn there — and the caller is what refuses to act on
them, so the row grammar keeps its one problem kind and gains no notion of clickability.

#### Scenario: Every drawn row is reported by the row it occupies

- **WHEN** a dashboard whose `changes.problems` holds one entry, with three active and three
  archived changes, is rendered at 120x40 and at 60x20, and `row_at` is called for every
  row offset from `0` to the interior's height
- **THEN** for each offset within the drawn rows, `row_at`'s `RowKind` is the kind of the
  row `list::rows` placed at that same offset, and the text drawn into the buffer at that
  terminal row equals that row's own `text`
- **AND** for every offset past the last drawn row, `row_at` is `None`

#### Scenario: The reported row follows the scrolled slice

- **WHEN** the same dashboard is given more changes than the interior has rows, the cursor is
  moved to the last change, and the frame is drawn
- **THEN** `row_at(dashboard, interior, 0)` reports the kind of the first row of the
  **scrolled** slice, not the first row `list::rows` emitted
- **AND** the offset it used equals `layout::viewport(rows.len(), cursor, interior.height)`
  computed independently in the test

#### Scenario: A collapsed section reports its header and nothing behind it

- **WHEN** the archived section is collapsed and the frame is drawn at 120x40
- **THEN** `row_at` reports the archived header's own `RowKind::Section` at the header's row
- **AND** no offset reports a `RowKind::Item` whose index belongs to an archived change, so a
  click cannot reach a row that is not drawn

#### Scenario: Degenerate interiors report nothing

- **WHEN** `row_at` is called with interiors of `0x0`, `1x0`, `0x1`, and `1x1`, at row
  offsets `0`, `1`, and `65535`, against a dashboard with no repository, one with no
  changes, and one with changes
- **THEN** every call returns a value without panicking
- **AND** every call against a zero-width or zero-height interior returns `None`, matching
  `render_list`, which draws nothing there
