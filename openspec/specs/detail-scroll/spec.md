# detail-scroll Specification

## Purpose
TBD - created by archiving change markdown-viewer. Update Purpose after archive.

## Requirements

### Requirement: The detail region draws the rendered markdown document

`ui::view::render` SHALL fill the detail region's interior with
`ui::markdown::lines(&dashboard.detail.source, interior.width)`, drawing the slice that
starts at `layout::scroll_offset(lines.len(), dashboard.detail.scroll, interior.height)` and
runs for at most `interior.height` lines, one rendered line per terminal row, starting at the
interior's first row and first column, drawing each segment left to right with the
`ratatui::style::Style` its `Face` maps to and never writing past the interior's last column.
A rendered line shorter than the interior leaves the rest of its row untouched, because
`markdown-render` does not pad.

The `Face`-to-`Style` mapping SHALL live in `ui::view` and nowhere else, and SHALL be:
`heading` present or `strong` → `Modifier::BOLD`; `emphasis` → `Modifier::ITALIC`; `code`
→ `Modifier::DIM`; `link` → `Modifier::UNDERLINED`; `quoted` → `Modifier::DIM`. Flags
compose, so a bold link's cells carry `BOLD` and `UNDERLINED` together.

The region SHALL draw nothing when its interior has zero width or zero height, and nothing
when `detail.source` is empty — leaving, in the empty case, every interior cell a space
whose `Style` equals `ratatui::buffer::Cell::default().style()`, which is what
`responsive-layout`'s blank-interior scenario asserts.

The two mandated detail interiors are **78 columns by 16 rows** at a 120x20 frame and **58
columns by 16 rows** at a 60x20 frame in the detail route. Every scenario below is
exercised at both.

#### Scenario: The document fills the detail interior at both mandated widths

- **WHEN** a `Dashboard` whose `detail.source` is a bullet list of the twenty items
  `- line-00` through `- line-19`, whose `detail.scroll` is `0`, and whose `changes` holds
  the single active change `fix-empty-basket` with progress 7 of 7, is rendered at 120x20
  with `route: Route::List`, and at 60x20 with `route: Route::Detail`
- **THEN** in the 120-column buffer row 2, columns 41 through 49, reads `- line-00`, and
  row 17, columns 41 through 49, reads `- line-15` — sixteen rows of content, the
  seventeenth line not drawn
- **AND** in the 60-column buffer row 2, columns 1 through 9, reads `- line-00`, and row
  17, columns 1 through 9, reads `- line-15`
- **AND** in the 120-column buffer the list region is unaffected: row 2, columns 1 through
  38, reads `> fix-empty-basket               [7/7]`, the change-row grammar `change-rows`
  defines

#### Scenario: Faces reach the buffer as styles at both widths

- **WHEN** a `Dashboard` whose `detail.source` is
  `# Title` / `` / ``A **bold** and *italic* line with `code` and [a link](x).`` / `` /
  `> quoted` is rendered at 120x20 with `route: Route::Detail` and at 60x20 with
  `route: Route::Detail`
- **THEN** in both buffers the seven cells spelling `# Title` report `Modifier::BOLD` set.
  The assertion is on those cells and not on the whole row: `markdown-render` does not pad a
  line to the width, so the columns beyond the heading are never written and carry
  `Cell::default().style()`
- **AND** in both buffers the cells spelling `bold` report `BOLD`, the cells spelling
  `italic` report `ITALIC`, the cells spelling `code` report `DIM`, the cells spelling
  `a link` report `UNDERLINED`, and the cells spelling `> quoted` report `DIM`
- **AND** in both buffers the cells spelling `and` between them report none of those four
  modifiers, so the test discriminates rather than asserting a constant

#### Scenario: An empty source leaves the detail interior blank at both widths

- **WHEN** a `Dashboard` whose `detail.source` is empty and whose `detail.scroll` is `0` is
  rendered at 120x20 and at 60x20 with `route: Route::Detail`
- **THEN** in the 120-column buffer every cell of rows 2 through 17, columns 41 through
  118, is a space whose `Style` equals `Cell::default().style()`
- **AND** in the 60-column buffer every cell of rows 2 through 17, columns 1 through 58, is
  such a space
- **AND** nothing panics, so the production pane — where nothing sets `detail.source` until
  `detail-view` lands — is unchanged by this change

#### Scenario: Content never overwrites the detail region's border

- **WHEN** a `Dashboard` whose `detail.source` holds twenty lines each 200 characters long
  is rendered at 120x20 with `route: Route::Detail` and at 60x20 with
  `route: Route::Detail`
- **THEN** in the 120-column buffer every cell of columns 39, 40, and 119 in rows 1 through
  18 is a box-drawing character
- **AND** in the 60-column buffer every cell of columns 0 and 59 in rows 1 through 18 is a
  box-drawing character
- **AND** in both buffers no interior row is blank, so the assertion is made against a
  region that was actually written

#### Scenario: A degenerate detail interior draws nothing and does not panic

- **WHEN** the same non-empty `Dashboard` is rendered at 1x20, at 2x20, at 3x20, at 60x2,
  and at 60x3, in the detail route
- **THEN** none panics
- **AND** at 60x20 and at 120x20 — rendered in the same test as contrasting controls — the
  content is present, so the scenario proves a width branch rather than a feature that is
  simply absent

### Requirement: `Dashboard::detail` carries the markdown source and the scroll offset

`ui::app::Dashboard` SHALL carry a `detail: Detail` field, where
`pub struct Detail { pub source: String, pub scroll: usize }`.

`source` is the markdown the detail region shows. **Nothing in this change sets it**:
`ui::load` SHALL start it empty, and `detail-view` — which owns artifact resolution and the
tab bar — supplies it. `scroll` is the index of the first rendered line the region draws,
and is a **user-controlled position**, not derived geometry: it is the detail region's
counterpart to `list-selection`'s `selected`, not to `list-selection`'s derived `viewport`.

`Detail` SHALL NOT implement `Default` — neither derived nor hand-written, anywhere in the
crate — and every construction and every destructuring of it SHALL name both fields, with
no `..` rest, on exactly the terms `dashboard-loop` states for `Dashboard` and `Filter`.

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
  exhaustive pattern naming both fields and no `..`, and the `Dashboard` companion grows
  from seven fields to eight

#### Scenario: Startup leaves the detail empty and unscrolled

- **WHEN** `ui::load` is called over a scratch repository holding one change
- **THEN** the returned `Dashboard`'s `detail.source` is empty and `detail.scroll` is `0`
- **AND** the same holds when `ui::load` finds **no** `openspec/` directory above its
  starting path and takes its `RepoSearch::NotFound` arm, which is a second `Dashboard`
  construction site and therefore a second place the field can be got wrong
- **AND** rendering both dashboards at 120x20 and at 60x20 leaves the detail interior blank,
  so the production pane still shows an empty detail frame after this change

### Requirement: The drawn slice is derived on every draw from the current interior

`ui::layout::scroll_offset(lines: usize, scroll: usize, height: u16) -> usize` SHALL return
the index of the first line to draw, as a pure total function of its three arguments: `0`
when `height` is `0`, and otherwise `min(scroll, lines.saturating_sub(height))`. It SHALL be
derived on every draw rather than stored, exactly as `viewport` is, because the interior
height is a property of the current frame.

`ui::layout::interior(area: Rect) -> Rect` SHALL be the one place in the crate that computes
a bordered region's interior, and SHALL perform exactly the arithmetic
`ratatui::widgets::Block::bordered().inner` performs: the origin advanced by one column and
one row and **clamped to the rectangle's own right and bottom edges**, with the width and
height each reduced by two, saturating to zero. The clamp is not decoration — at
`Rect::new(0, 0, 0, 0)` it is the difference between `Rect { x: 0, y: 0, … }` and
`Rect { x: 1, y: 1, … }`, and `Rect` compares all four fields.

#### Scenario: `scroll_offset` is exact at its boundaries

- **WHEN** `scroll_offset` is called with `(0, 0, 16)`, `(16, 0, 16)`, `(16, 9, 16)`,
  `(17, 0, 16)`, `(17, 1, 16)`, `(17, 2, 16)`, `(20, 4, 16)`, `(20, 99, 16)`, and
  `(20, 4, 0)`
- **THEN** it returns `0`, `0`, `0`, `0`, `1`, `1`, `4`, `4`, and `0` respectively
- **AND** the value is never greater than `lines.saturating_sub(height)`, so the drawn
  slice never runs past the last line

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

- **WHEN** a `Dashboard` whose `detail.source` is the twenty-item list and whose
  `detail.scroll` is `99` is rendered at 120x20 and at 60x20 in the detail route
- **THEN** in the 120-column buffer row 2 reads `- line-04` and row 17 reads `- line-19`
- **AND** in the 60-column buffer row 2 reads `- line-04` and row 17 reads `- line-19`
- **AND** no row below row 17 and no row above row 2 of the interior carries content, so
  the slice stopped at the end rather than drawing past it

### Requirement: `j`, `k`, and the arrows scroll the detail content at the detail route

`Action::Next` and `Action::Prev` — renamed from `SelectNext` and `SelectPrev`, because the
action is route-agnostic and only its effect is not — SHALL be interpreted by
`Dashboard::apply` according to the current route: at `Route::List` they move and clamp
`selected`, per `list-selection`; at `Route::Detail` they move `detail.scroll` by one line,
with `Prev` saturating at `0` and `Next` saturating at `usize::MAX`, the upper bound being
enforced by the draw-time clamp and the frame normalisation below.

`action_for` SHALL be unchanged in shape: it maps `Char('j')` and `Down` to `Next` and
`Char('k')` and `Up` to `Prev` while `filtering` is false, and while `filtering` is true
`j` and `k` still type themselves into the query and only the arrows navigate — the layering
`list-filtering` states is untouched.

**Every action that moves `route`** SHALL reset `detail.scroll` to `0`, so a change opened
twice opens at the top both times. That is three arms of `apply`: `OpenDetail` when it sets
`Route::Detail`, `Back` when it returns to `Route::List`, and `FilterStart`, which
`list-filtering` also defines as moving the route to `List`. Dismissing a filter layer is
**not** a route move and SHALL leave `detail.scroll` alone.

#### Scenario: At the detail route the content scrolls by one line at both widths

- **WHEN** a `Dashboard` whose `detail.source` is the twenty-item list, whose `route` is
  `Route::Detail`, and whose `detail.scroll` is `0` is given a `Next` action, then a second
  `Next`
- **THEN** `detail.scroll` is `1`, then `2`, and `selected` is unchanged throughout
- **AND** rendering after the second action at 120x20 puts `- line-02` at row 2, columns 41
  through 49, and `- line-17` at row 17
- **AND** rendering after the second action at 60x20 puts `- line-02` at row 2, columns 1
  through 9, and `- line-17` at row 17

#### Scenario: At the list route the same actions still move the selection

- **WHEN** a `Dashboard` with three active changes, a non-empty `detail.source`, `route` of
  `Route::List`, and `detail.scroll` of `0` is given two `Next` actions
- **THEN** `selected` is `2` and `detail.scroll` is still `0`
- **AND** rendering at 120x20 and at 60x20 puts the `>` marker on the third list row in
  both, so the contrast with the detail route is observed on screen and not only in state

#### Scenario: Scrolling stops at the top

- **WHEN** a `Dashboard` at `Route::Detail` whose `detail.source` is the twenty-item list and
  whose `detail.scroll` is `0` is given four consecutive `Prev` actions
- **THEN** `detail.scroll` is `0` after each, and nothing panics
- **AND** rendering at 120x20 and at 60x20 still puts `- line-00` on the interior's first
  row in both

#### Scenario: While filtering, `j` and `k` still type into the query

- **WHEN** `action_for` is called with `filtering` true and Presses of `Char('j')`,
  `Char('k')`, `Down`, and `Up`
- **THEN** it returns `FilterPush('j')`, `FilterPush('k')`, `Next`, and `Prev`
- **AND** applying those four to a `Dashboard` at `Route::Detail` with an active filter
  leaves `filter.query` as `jk` and moves `detail.scroll` to `1` and back to `0`, so the
  filter layer still wins over the scroll layer for printable keys

#### Scenario: Every route move resets the scroll

- **WHEN** a `Dashboard` at `Route::Detail` whose `detail.source` is the twenty-item list and
  whose `detail.scroll` is `3` is given a `Back` action, and then an `OpenDetail` action
- **THEN** `detail.scroll` is `0` after the `Back` and still `0` after the `OpenDetail`
- **AND** a second `Dashboard` in the same state given a `FilterStart` action instead has
  `route` `List` and `detail.scroll` `0`, while a third at `Route::Detail` with
  `filter.active` true and `detail.scroll` `3` given a `Back` — which dismisses the filter
  layer and not the route — still has `detail.scroll` `3`
- **AND** rendering the first dashboard at 120x20 and at 60x20 after the `OpenDetail` puts
  `- line-00` on the interior's first row in both

### Requirement: The stored scroll offset is normalised against the frame just drawn

`Dashboard::normalise_scroll(&mut self, frame_area: Rect)` SHALL be a pure total function of
its two arguments that recomputes the detail region from `frame_area` through
`layout::split_frame`, `layout::split_body`, and `layout::interior`, and assigns
`detail.scroll = layout::scroll_offset(lines, detail.scroll, interior.height)` for the line
count `markdown::lines` produces at that interior's width. It SHALL change nothing when the
detail region is not drawn — the narrow list route — or when the interior has zero width or
zero height.

`ui::driver::run_loop` SHALL call it once per iteration, with the `area` of the
`CompletedFrame` the draw returned, so a held key cannot leave `detail.scroll` arbitrarily
far past the end and require as many presses to come back. `ui::view::render` SHALL remain a
pure function of `&Dashboard`: it clamps for **display** and never writes the stored value,
which is why the normalisation lives in the loop and not in the view.

#### Scenario: Scrolling past the end is normalised on the next frame

- **WHEN** `run_loop` runs against a `TestBackend` of 120x20 and a scripted event source
  delivering ten Presses of `Char('j')` and then a Press of `Char('q')`, over a `Dashboard`
  at `Route::Detail` whose `detail.source` is the twenty-item list
- **THEN** the loop ends with `dashboard.detail.scroll` equal to `4`, not `10`
- **AND** the final buffer's row 2, columns 41 through 49, reads `- line-04` and row 17
  reads `- line-19`
- **AND** the same script against a `TestBackend` of 60x20 ends with `detail.scroll` of `4`
  and the same two rows at columns 1 through 9
- **AND** the summary reports `frames: 11, polls: 11`, so the normalisation did not add or
  skip a draw

#### Scenario: A resize renormalises the offset on the next frame

The pair `terminal.draw(render)` then `normalise_scroll(area)` is exactly what one iteration
of `run_loop` does, and this scenario drives that pair directly rather than through
`run_loop`: the loop borrows the `Terminal` mutably for its whole run, so no scripted event
source can resize the backend from inside it.

- **WHEN** the twenty-item `Dashboard` at `detail.scroll` of `4` is drawn into a
  `Terminal<TestBackend>` of 120x20 and normalised against that frame's area, the backend is
  then resized to 120x30, and the pair is repeated
- **THEN** after the first pair `detail.scroll` is `4`, and after the second it is `0`,
  because a 26-row interior holds every one of the twenty lines
- **AND** the second buffer's row 2, columns 41 through 49, reads `- line-00`, so the
  renormalisation is visible rather than only stored

#### Scenario: The narrow list route leaves the stored offset alone

- **WHEN** `normalise_scroll` is called with a `Dashboard` at `Route::List` whose
  `detail.scroll` is `7` and a `frame_area` of 60x20 — a width at which the detail region
  is not drawn at all
- **THEN** `detail.scroll` is still `7`
- **AND** with the same dashboard and a `frame_area` of 120x20, where the detail region
  **is** drawn, `detail.scroll` becomes `4`, so the early return is a real branch and not
  the only behaviour
