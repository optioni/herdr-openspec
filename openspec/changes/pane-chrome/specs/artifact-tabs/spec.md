## MODIFIED Requirements

### Requirement: The tab bar is drawn into the detail region's second interior row

`ui::view::render` SHALL draw each `Tab` into the second row of the detail region's interior
at `interior.x + tab.x`, applying `palette::style(Role::TabActive)` to the selected chip's
cells and `palette::style(Role::TabInactive)` to every other chip's — including the
zero-artifact placeholder, which occupies the bar's position and is drawn on the bar's own
terms. Every column of a chip, its two padding columns included, SHALL carry that style, so
the painted span is exactly the chip's own span. The separating column between two chips
SHALL be left untouched, as SHALL every column no chip occupies, on the same terms
`detail-scroll` leaves the tail of a short markdown line untouched.

The row it sits in is the interior's second, and that is still true after `pane-chrome`
though the interior's contents around it changed. The change header no longer occupies the
interior's first row at all: `detail-header` draws it into the region's heading row, one row
above the interior. The interior's **first** row is now a **blank spacer**, its second is the
tab bar, and its third is a **horizontal rule**. Nothing is drawn into the spacer, and the
rule is the character `─` repeated across the interior's full width, drawn with
`palette::style(Role::RegionRule)`.

The two rows exist for one measured reason: with the header, the bar, and the artifact's
first line of markdown on three adjacent rows, a reader parses the bar as content. A blank
row above the bar separates it from the change it belongs to and a rule below it separates it
from the artifact it selects, which is exactly the grouping the bar has.

`ui::layout::split_detail(interior: Rect) -> (Rect, Rect, Rect)` SHALL split the detail
region's interior into a one-row **tab bar**, a one-row **rule**, and the content area below,
each the interior's full width and each carrying the interior's own `x` and `width`. It
SHALL NOT return a header rectangle: there is no header inside the interior to return one
for. Heights `0`, `1`, and `2` SHALL be branched on explicitly, exactly as `split_frame` does
and for the same measured reason:

| Interior height | Tab bar | Rule | Content | Spacer |
|---|---|---|---|---|
| `0` | zero-height at `interior.y` | zero-height at `interior.y` | zero-height at `interior.y` | none |
| `1` | one row at `interior.y` | zero-height at `interior.y + 1` | zero-height at `interior.y + 1` | none |
| `2` | one row at `interior.y` | one row at `interior.y + 1` | zero-height at `interior.y + 2` | none |
| `h >= 3` | one row at `interior.y + 1` | one row at `interior.y + 2` | `h - 3` rows at `interior.y + 3` | one row at `interior.y` |

The spacer is the row the pane gives up first. Below three interior rows there is no room for
both it and a content line, and a reader with two rows to spend wants the bar and the rule,
not air; the bar therefore moves up to the interior's first row at heights `1` and `2`. That
is the one height at which "the second interior row" in this requirement's name is not where
the bar is drawn, and it is stated here rather than left to be discovered.

At the mandated frames the arithmetic lands as follows: the detail region's interior is
eighteen rows at a 20-row frame, beginning at buffer row 1, so the spacer is buffer row 1,
the tab bar is buffer row **2**, the rule is buffer row 3, and the content area is buffer
rows 4 through 18 — fifteen rows. The content area's first row is buffer row 4 at both
mandated widths, exactly as it was before `pane-chrome`, and it gained one row at the bottom.

#### Scenario: The tab bar reaches the buffer at both mandated widths

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries the five tdd
  artifacts, with `detail.tab: 2`, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer row 2, columns 41 through 93, spell
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
- **AND** in both buffers the row above the bar is entirely spaces with no background, and
  the row below it is `─` repeated across the interior's width with `Modifier::DIM` set and
  no foreground, so the spacer and the rule are drawn where the table above places them

#### Scenario: The tab bar never overwrites a border or the rows around it

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
what the bar must not overwrite is now the region's gutter columns and the divider.

- **WHEN** a `Dashboard` whose selected change carries twelve artifacts with 40-character
  ids is rendered at 120x20 and at 60x20 at `Route::Detail`
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 0 through 18
  is a space
- **AND** in the 120-column buffer every cell of columns 0, 39, and 119 in rows 0 through 18
  is a space and every cell of column 40 is the divider `│`
- **AND** in both buffers no cell of a gutter or divider column carries a chip background, so
  the painted span stopped inside the interior
- **AND** in both buffers the region's heading row still holds the change's name, so no chip
  wrapped upward

#### Scenario: `split_detail` is exact at its degenerate heights

- **WHEN** `split_detail` is called on interiors of the two mandated widths — 78 and 58 — at
  heights `0`, `1`, `2`, `3`, and `18`
- **THEN** at height `0` all three rects are zero-height at the interior's own `y`
- **AND** at height `1` the tab bar is one row at `interior.y` and the rule and content are
  zero-height, both at `interior.y + 1`
- **AND** at height `2` the tab bar and rule are one row each, at `interior.y` and
  `interior.y + 1`, and the content is zero-height at `interior.y + 2`
- **AND** at height `3` the tab bar is at `interior.y + 1`, the rule at `interior.y + 2`, and
  the content is zero-height at `interior.y + 3`, the spacer having claimed `interior.y`
- **AND** at height `18` the content area is fifteen rows starting three rows below the
  interior's `y`
- **AND** at every height all three rects carry the interior's own `x` and `width`
