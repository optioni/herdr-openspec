## MODIFIED Requirements

### Requirement: The detail region draws the rendered markdown document

`ui::view::render` SHALL fill the detail region's **content area** — the rectangle
`layout::split_detail` returns below the header row and the tab-bar row `detail-header` and
`artifact-tabs` own — with `ui::detail::content_lines(&dashboard.detail, content.width)`,
drawing the slice that starts at
`layout::scroll_offset(lines.len(), dashboard.detail.scroll, content.height)` and runs for at
most `content.height` lines, one rendered line per terminal row, starting at the content
area's first row and first column, drawing each segment left to right with the
`ratatui::style::Style` its `Face` maps to and never writing past the interior's last column.
A rendered line shorter than the content area leaves the rest of its row untouched, because
`markdown-render` does not pad.

The content area's **width** equals the interior's, so the two mandated interior widths, 78
and 58, are also the two mandated wrapping widths and `markdown-render` is unaffected. Only
the **height** changes: a 16-row interior gives the content area 14 rows.

`content_lines` — not `markdown::lines` directly — is what both this draw and
`Dashboard::normalise_scroll` derive their line list from, so the drawn slice and the clamp
can never disagree about how many lines there are. `artifact-content` states what it
returns, including the `No content yet` line and the `!`-prefixed problem lines.

The `Face`-to-`Style` mapping SHALL live in `ui::view` and nowhere else, and SHALL be:
`heading` present or `strong` → `Modifier::BOLD`; `emphasis` → `Modifier::ITALIC`; `code`
→ `Modifier::DIM`; `link` → `Modifier::UNDERLINED`; `quoted` → `Modifier::DIM`. Flags
compose, so a bold link's cells carry `BOLD` and `UNDERLINED` together.

The region SHALL draw nothing at all — no header, no tab bar, no content — when `visible()`
is empty, leaving every interior cell a space whose `Style` equals
`ratatui::buffer::Cell::default().style()`, which is what `responsive-layout`'s
blank-interior scenario asserts. When a change **is** selected, the region SHALL always draw
its header and tab bar, and the content area SHALL always hold at least one line, because
`content_lines` returns `No content yet` rather than nothing.

The two mandated detail interiors are **78 columns by 16 rows** at a 120x20 frame and **58
columns by 16 rows** at a 60x20 frame in the detail route, giving content areas of 78x14 and
58x14. Every scenario below is exercised at both.

#### Scenario: The document fills the detail interior at both mandated widths

The scenario's name is kept verbatim from `markdown-viewer` because a delta's scenario
headers are its merge key; its subject is the same document, drawn two rows lower.

- **WHEN** a `Dashboard` at `Route::Detail`, whose selected change carries one artifact and
  whose `detail.source` is a bullet list of the twenty items `line-00` through `line-19`, is
  rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer row 4 columns 41 onward reads `- line-00` and row 17
  reads `- line-13`, so fourteen items are drawn into the content area
- **AND** in the 60-column buffer row 4 columns 1 onward reads `- line-00` and row 17 reads
  `- line-13`
- **AND** in both buffers row 2 holds the change header and row 3 the tab bar, so the
  markdown began below them rather than over them

#### Scenario: Faces reach the buffer as styles at both widths

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact and
  whose `detail.source` is `## Heading\n\n**bold** and *italic* and `code` and [link](u)\n`
  is rendered at 120x20 and at 60x20
- **THEN** in each buffer the cells of `## Heading` in the content area's first row report
  `Modifier::BOLD` set
- **AND** the cells of `bold` report `BOLD`, of `italic` report `ITALIC`, of `code` report
  `DIM`, and of `link` report `UNDERLINED`
- **AND** the assertion discriminates: a cell of the surrounding plain text reports none of
  those modifiers

#### Scenario: An empty source leaves the detail interior blank at both widths

The scenario's name is kept verbatim; its subject moves from "an empty `source`" to "no
change selected", because with a change selected the region is never blank — `No content
yet` is drawn instead, which is `artifact-content`'s requirement and `SPEC.md`'s
degraded-states row.

- **WHEN** a `Dashboard` over `changes::empty_set()` is rendered at 120x20 at `Route::List`
  and a `Dashboard` whose only change is filtered out is rendered at 60x20 at
  `Route::Detail`
- **THEN** every cell of the detail region's interior in each buffer is a space whose `Style`
  equals `ratatui::buffer::Cell::default().style()`
- **AND** a third `Dashboard` with a change selected and an empty `detail.source` rendered at
  the same two sizes shows `No content yet` in the content area's first row and is therefore
  **not** blank, so the two states are distinguished rather than conflated

#### Scenario: Content never overwrites the detail region's border

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries five artifacts with
  40-character ids, whose name is 200 characters long, and whose `detail.source` is thirty
  lines each 200 characters long, is rendered at 120x20 and at 60x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 1 through 18
  is a box-drawing character
- **AND** in the 120-column buffer every cell of columns 0, 39, 40, and 119 in rows 1
  through 18 is a box-drawing character
- **AND** no content is written above row 2 or below row 17 in either buffer

#### Scenario: A degenerate detail interior draws nothing and does not panic

The frame height a detail interior row costs is four — one frame-header row, one frame-footer
row, and the region's two border rows — so the interior first has one row at a frame height of
**5**, two at **6**, and three at **7**. Those are the heights this scenario samples; 3 and 4
give an interior of zero rows and would exercise only the earliest guard.

- **WHEN** a `Dashboard` with a change selected and a non-empty `detail.source` is rendered
  at 120x4, 120x5, 120x6, 120x7, 60x5, 60x6, 60x7, 1x20, and 2x20
- **THEN** no render panics at any of them
- **AND** at 120x4 and its narrow counterpart the interior has zero rows and nothing at all is
  drawn inside the region
- **AND** at 120x5 and 60x5 the header row **is** drawn and no tab cell and no markdown line
  appears anywhere in the frame
- **AND** at 120x6 and 60x6 the header row and the tab bar are drawn and no markdown line
  appears
- **AND** at 120x7 and 60x7 exactly one content row is drawn, holding the source's first
  rendered line

### Requirement: `Dashboard::detail` carries the markdown source and the scroll offset

`ui::app::Dashboard` SHALL carry a `detail: Detail` field, where

```rust
pub struct Detail {
    pub source: String,
    pub scroll: usize,
    pub tab: usize,
    pub problems: Vec<String>,
    pub loaded: Option<(std::path::PathBuf, usize)>,
}
```

`source` is the markdown the detail region shows. From `detail-view` onward it **is** set:
`ui::load` still starts it empty, and `Dashboard::sync_detail` — driven by
`ui::driver::run_loop` with `artifact-content`'s injected reader — fills it from the selected
change's selected artifact. `scroll` is the index of the first rendered line the region
draws, and is a **user-controlled position**, not derived geometry: it is the detail region's
counterpart to `list-selection`'s `selected`, not to `list-selection`'s derived `viewport`.
`tab`, `problems`, and `loaded` are `artifact-tabs`' and `artifact-content`'s, and are
specified there.

`Detail` SHALL NOT implement `Default` — neither derived nor hand-written, anywhere in the
crate — and every construction and every destructuring of it SHALL name **all five** fields,
with no `..` rest, on exactly the terms `dashboard-loop` states for `Dashboard` and `Filter`.

#### Scenario: `Detail` has no `Default` and no site elides a field

- **WHEN** every `*.rs` file under `src/` is searched, for the type name `Detail`, for
  `impl Default for Detail`, for a `Default` inside the `#[derive(...)]` immediately
  preceding `struct Detail`, and for a `..` appearing inside a `Detail { … }` literal or
  pattern
- **THEN** there is no match
- **AND** the search is the same parameterised check that covers `Dashboard` and `Filter`,
  run over the type list `Dashboard Filter Detail`, rather than a second drifting check
- **AND** it is paired with a positive control asserting that `src/ui/app.rs` **does**
  contain `struct Detail {`, and the check is proven able to fail against a copy carrying
  `impl Default for Detail { … }` and against a copy carrying `let Detail { source, .. }`
- **AND** a compile-time companion exists: a test destructures a `Detail` with an
  exhaustive pattern naming all five fields and no `..`, and the `Dashboard` companion
  continues to name all eight

#### Scenario: Startup leaves the detail empty and unscrolled

- **WHEN** `ui::load` is called over a scratch repository holding one change
- **THEN** the returned `Dashboard`'s `detail.source` is empty, `detail.problems` is empty,
  and `detail.scroll`, `detail.tab`, and `detail.loaded` are `0`, `0`, and `None`
- **AND** the same holds when `ui::load` finds **no** `openspec/` directory above its
  starting path and takes its `RepoSearch::NotFound` arm, which is a second `Dashboard`
  construction site and therefore a second place the field can be got wrong
- **AND** rendering the `NotFound` dashboard at 120x20 and at 60x20 leaves the detail
  interior blank, because nothing is selected
- **AND** rendering the loaded dashboard at 120x20 shows that change's header and tab bar
  with `No content yet` below them, because a change **is** selected and nothing has been
  read yet — which is exactly the state `run_loop`'s first `sync_detail` replaces

### Requirement: The drawn slice is derived on every draw from the current interior

`ui::layout::scroll_offset(lines: usize, scroll: usize, height: u16) -> usize` SHALL return
the index of the first line to draw, as a pure total function of its three arguments: `0`
when `height` is `0`, and otherwise `min(scroll, lines.saturating_sub(height))`. It SHALL be
derived on every draw rather than stored, exactly as `viewport` is, because the interior
height is a property of the current frame. From `detail-view` onward the `height` it is
given for the detail region is the **content area's**, not the interior's.

`ui::layout::interior(area: Rect) -> Rect` SHALL be the one place in the crate that computes
a bordered region's interior, and SHALL perform exactly the arithmetic
`ratatui::widgets::Block::bordered().inner` performs: the origin advanced by one column and
one row and **clamped to the rectangle's own right and bottom edges**, with the width and
height each reduced by two, saturating to zero. The clamp is not decoration — at
`Rect::new(0, 0, 0, 0)` it is the difference between `Rect { x: 0, y: 0, … }` and
`Rect { x: 1, y: 1, … }`, and `Rect` compares all four fields.

`ui::layout::split_detail(interior: Rect) -> (Rect, Rect, Rect)` SHALL be the one place in
the crate that divides that interior into the header row, the tab-bar row, and the content
area, so `ui::view::render` and `Dashboard::normalise_scroll` cannot derive different content
heights from the same frame. `artifact-tabs` states its degenerate-height contract.

#### Scenario: `scroll_offset` is exact at its boundaries

- **WHEN** `scroll_offset` is called with `(0, 0, 14)`, `(14, 0, 14)`, `(14, 9, 14)`,
  `(15, 0, 14)`, `(15, 1, 14)`, `(15, 2, 14)`, `(20, 6, 14)`, `(20, 99, 14)`, and
  `(20, 6, 0)`
- **THEN** it returns `0`, `0`, `0`, `0`, `1`, `1`, `6`, `6`, and `0` respectively
- **AND** the value is never greater than `lines.saturating_sub(height)`, so the drawn
  slice never runs past the last line
- **AND** the heights are the **content area's** fourteen rows rather than the interior's
  sixteen, which is what `detail-view` changed about the caller

#### Scenario: `interior` agrees with a bordered block's own inner rectangle

- **WHEN** `interior` and `ratatui::widgets::Block::bordered().inner` are both applied to
  `Rect::new(0, 1, 40, 18)`, `Rect::new(40, 1, 80, 18)`, `Rect::new(0, 1, 60, 18)`,
  `Rect::new(0, 0, 2, 2)`, `Rect::new(0, 0, 1, 1)`, and `Rect::new(0, 0, 0, 0)`
- **THEN** the two agree on every one of them, compared as whole `Rect` values — all four
  fields, not width and height alone
- **AND** the two degenerate rectangles yield zero width and zero height rather than
  underflowing, with `Rect::new(0, 0, 1, 1)` giving `Rect { x: 1, y: 1, width: 0, height: 0 }`
  and `Rect::new(0, 0, 0, 0)` giving `Rect { x: 0, y: 0, width: 0, height: 0 }` — the origin
  clamp, which is the one place a hand-rolled interior and ratatui's disagree

#### Scenario: A scroll offset past the end still draws the last screenful

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact, whose
  `detail.source` is the twenty-item list, and whose `detail.scroll` is `99`, is rendered at
  120x20 and at 60x20
- **THEN** in the 120-column buffer the content area's first row reads `- line-06` and its
  last drawn row reads `- line-19`
- **AND** in the 60-column buffer the same two rows read the same, so the fourteen-row
  content area — not the sixteen-row interior — is what the clamp used
- **AND** no row above the content area and no row below row 17 carries markdown

### Requirement: The stored scroll offset is normalised against the frame just drawn

`Dashboard::normalise_scroll(&mut self, frame_area: Rect)` SHALL be a pure total function of
its two arguments that recomputes the detail region from `frame_area` through
`layout::split_frame`, `layout::split_body`, `layout::interior`, and `layout::split_detail`,
and assigns `detail.scroll = layout::scroll_offset(lines, detail.scroll, content.height)` for
the line count `ui::detail::content_lines` produces at that content area's width. It SHALL
change nothing when the detail region is not drawn — the narrow list route — or when the
content area has zero width or zero height.

`ui::driver::run_loop` SHALL call it once per iteration, with the `area` of the
`CompletedFrame` the draw returned, so a held key cannot leave `detail.scroll` arbitrarily
far past the end and require as many presses to come back. `ui::view::render` SHALL remain a
pure function of `&Dashboard`: it clamps for **display** and never writes the stored value,
which is why the normalisation lives in the loop and not in the view.

#### Scenario: Scrolling past the end is normalised on the next frame

- **WHEN** `run_loop` runs against a `TestBackend` of 120x20 and a scripted event source
  delivering ten Presses of `Char('j')` and then a Press of `Char('q')`, over a `Dashboard`
  at `Route::Detail` whose selected change carries one artifact whose file reads as the
  twenty-item list, with a reader supplying it
- **THEN** the run ends with `dashboard.detail.scroll` equal to `6`, not `10`: twenty lines
  in a fourteen-row content area allow an offset of at most six
- **AND** the same holds at 60x20, so the normalisation is not a property of the wide layout
- **AND** the final buffer's content area reads `- line-06` on its first row and `- line-19`
  on its last

#### Scenario: A resize renormalises the offset on the next frame

- **WHEN** a `Dashboard` at `Route::Detail` whose `detail.scroll` is `6` over the twenty-item
  list has `normalise_scroll` called with a 120x20 area, then with a 120x40 area, then with a
  60x20 area
- **THEN** the offset is `6` after the first, `0` after the second — a 34-row content area
  holds every line — and `6` again after the third
- **AND** no call panics

#### Scenario: The narrow list route leaves the stored offset alone

- **WHEN** a `Dashboard` at `Route::List` whose `detail.scroll` is `9` has `normalise_scroll`
  called with a 60x20 area
- **THEN** `detail.scroll` is still `9`, because no detail region was drawn to normalise
  against
- **AND** with a 120x20 area — where the wide layout draws the detail region at the list
  route too — it is normalised to `6`
