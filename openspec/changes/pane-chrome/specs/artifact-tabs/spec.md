## ADDED Requirements

### Requirement: The tab bar is the detail interior's first row, above a rule and a padding row

`ui::view::render` SHALL draw each `Tab` into the **first** row of the detail region's
interior at `interior.x + tab.x`, applying `palette::style(Role::TabActive)` to the selected
chip's cells and `palette::style(Role::TabInactive)` to every other chip's — including the
zero-artifact placeholder, which occupies the bar's position and is drawn on the bar's own
terms. Every column of a chip, its two padding columns included, SHALL carry that style, so
the painted span is exactly the chip's own span. The separating column between two chips
SHALL be left untouched, as SHALL every column no chip occupies, on the same terms
`detail-scroll` leaves the tail of a short markdown line untouched.

The change header no longer sits above it inside the interior: `detail-header` draws that
into the region's own heading row, and `responsive-layout`'s region shape puts a blank
padding row between the two. Below the bar the interior holds a **horizontal rule** — the
character `─` repeated across the interior's full width, drawn with
`palette::style(Role::RegionRule)` — then a second blank **padding row**, then the content
area.

The rule and the padding row exist for one measured reason: with the header, the bar, and the
artifact's first line of markdown on three adjacent rows, a reader parses the bar as content.
The rule separates the bar from the artifact it selects, and the padding row keeps the
artifact's own first line off the rule.

`ui::layout::split_detail(interior: Rect) -> (Rect, Rect, Rect)` SHALL split the detail
region's interior into a one-row **tab bar**, a one-row **rule**, and the content area below,
each the interior's full width and each carrying the interior's own `x` and `width`. It
SHALL NOT return a header rectangle: there is no header inside the interior to return one
for. Heights `0`, `1`, and `2` SHALL be branched on explicitly, exactly as `split_frame` does
and for the same measured reason:

| Interior height | Tab bar | Rule | Content | Padding row |
|---|---|---|---|---|
| `0` | zero-height at `interior.y` | zero-height at `interior.y` | zero-height at `interior.y` | none |
| `1` | one row at `interior.y` | zero-height at `interior.y + 1` | zero-height at `interior.y + 1` | none |
| `2` | one row at `interior.y` | one row at `interior.y + 1` | zero-height at `interior.y + 2` | none |
| `h >= 3` | one row at `interior.y` | one row at `interior.y + 1` | `h - 3` rows at `interior.y + 3` | one row at `interior.y + 2` |

The padding row is the row the pane gives up first. Below three interior rows there is no room
for both it and a content line, and a reader with two rows to spend wants the bar and the
rule, not air.

At the mandated frames the arithmetic lands as follows: the detail region's interior is
seventeen rows at a 20-row frame, beginning at buffer row 2, so the tab bar is buffer row
**2**, the rule is buffer row 3, the padding row is buffer row 4, and the content area is
buffer rows 5 through 18 — **fourteen** rows, exactly as many as the bordered layout gave it.
The rule and the two padding rows spend precisely what the removed frame header row and the
removed bottom border row gained; what the reader gets for it is separation rather than lines.

#### Scenario: The tab bar reaches the buffer at both mandated widths

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries the five tdd
  artifacts, with `detail.tab: 2`, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer row 2, columns 42 through 94, spell
  `" proposal   specs   design   tasks   planning-review "` — three spaces between chips,
  being each chip's own padding and the one unpainted separator column
- **AND** in the 60-column buffer row 2, columns 1 through 53, spell the same 53-column
  string
- **AND** in both buffers the eight cells spelling `" design "` report `Modifier::BOLD` set
  and the background and foreground `Role::TabActive` carries (`Color::Cyan` on
  `Color::Black`), and the ten cells spelling `" proposal "` report the background
  `Role::TabInactive` carries (`Color::DarkGray`) and no `BOLD`, so the selected chip is
  discriminated by colour and by weight together
- **AND** in both buffers the single column between two chips has no background set, so the
  chips do not merge into one field
- **AND** in both buffers row 3 is `─` repeated across the interior's width with
  `Modifier::DIM` set and no foreground, and every cell of rows 1 and 4 inside the interior's
  columns is a space with no background — the region's padding row and the content's

#### Scenario: The bar, the rule, and the content never leave the interior

- **WHEN** a `Dashboard` whose selected change carries twelve artifacts with 40-character
  ids and whose `detail.source` is thirty 200-character lines is rendered at 120x20 and at
  60x20 at `Route::Detail`
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 0 through 18
  is a space
- **AND** in the 120-column buffer every cell of columns 0, 39, and 41 in rows 0 through 18
  is a space and every cell of column 40 is the divider `│`
- **AND** in neither buffer does a cell of a gutter or divider column carry a chip
  background or a `─`, so both painted spans stopped inside the interior
- **AND** in both buffers the region's heading row still holds the change's name, so no chip
  wrapped upward

#### Scenario: `split_detail` is exact at its degenerate heights

- **WHEN** `split_detail` is called on interiors of the two mandated widths — 78 and 58 — at
  heights `0`, `1`, `2`, `3`, `4`, and `17`
- **THEN** at height `0` all three rects are zero-height at the interior's own `y`
- **AND** at height `1` the tab bar is one row at `interior.y` and the rule and content are
  zero-height, both at `interior.y + 1`
- **AND** at height `2` the tab bar and rule are one row each, at `interior.y` and
  `interior.y + 1`, and the content is zero-height at `interior.y + 2`
- **AND** at height `3` the content is zero-height at `interior.y + 3`, the padding row having
  claimed `interior.y + 2`
- **AND** at height `4` the content is exactly one row at `interior.y + 3`, and at height `17`
  it is fourteen rows there
- **AND** at every height all three rects carry the interior's own `x` and `width`

## REMOVED Requirements

### Requirement: The tab bar is drawn into the detail region's second interior row

**Reason**: The bar is the interior's **first** row now. `pane-chrome` moved the change
header out of the interior and into the region's heading row, so the row the bar used to sit
below is no longer inside the interior at all. Replaced by "The tab bar is the detail
interior's first row, above a rule and a padding row", which keeps the chip styling rule, the
untouched-separator rule, and the explicit degenerate-height branching, and adds the rule row
and the padding row below the bar.

**Migration**: `ui::layout::split_detail` returns `(tabs, rule, content)` rather than
`(header, tabs, content)`; a caller that took `.0` as the header row takes the region's
heading row from `responsive-layout`'s region shape instead, and `.0` is now the tab bar.
