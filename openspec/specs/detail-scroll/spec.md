# detail-scroll Specification

## Purpose
Answers where in a long artifact the reader is and which slice of it reaches the buffer: the
`detail.scroll` position as a user-controlled index, `j`/`k`/arrows moving it by a line at the
detail route while the same actions move the list selection at the list route, every route move
resetting it to the top, and the drawn window derived on every frame from the current content
area rather than from anything stored. What `detail.scroll` *means* is decided by whether the
selected artifact is foldable: at a single-section artifact it is a scroll **offset** and the
window comes from `layout::scroll_offset`, while at a foldable one it is a line **cursor** and
the window comes from `layout::viewport`, the helper the list region already uses — without a
cursor a fold could not be addressed at all, since `scroll_offset` clamps to `0` whenever the
content fits the region. The one piece of geometry the dashboard stores, `Detail::drawn_width`,
lives here too: a keypress taken between frames has to resolve `detail.scroll` through the same
width-dependent row list the last frame drew. It also fixes the drawing side —
lines painted below the tab-bar row, the rule, and the padding row, each segment styled by the
one `Face`-to-`Style` mapping in `ui::view`, never past a gutter, never panicking at a one-row
interior — and requires
`normalise_scroll` to clamp the stored offset against the same `content_lines` the draw used, so
a switch between the markdown and checklist bodies cannot leave an offset valid for one applied
to the other. The line lists themselves come from `markdown-render` and `tasks-checklist`.

## Requirements

### Requirement: The detail region draws the rendered markdown document

`ui::view::render` SHALL fill the detail region's **content area** — the rectangle
`layout::split_detail` returns below the tab-bar row `artifact-tabs` owns, the horizontal
rule beneath it, and the blank padding row beneath that — with
`ui::detail::content_lines(&dashboard.detail, dashboard.selected_change(), content.width)`,
drawing the slice that starts at
`layout::scroll_offset(lines.len(), dashboard.detail.scroll, content.height)` and runs for at
most `content.height` lines, one rendered line per terminal row, starting at the content
area's first row and first column, drawing each segment left to right with the
`ratatui::style::Style` its `Face` maps to and never writing past the interior's last column.
A rendered line shorter than the content area leaves the rest of its row untouched, because
neither `markdown-render` nor `tasks-checklist` pads. A **table row line** is padded to its
own table's total width, which `markdown-render` requires to be at most the content area's
width; the region draws it like any other line and the sentence above is unaffected.

The content area's **width** equals the interior's, so the two mandated interior widths, 78
and 58, are also the two mandated wrapping widths and the widths every table in a rendered
artifact is allocated against. Only the **height** changes: a 17-row interior gives the
content area 14 rows.

`content_lines` — not `markdown::lines` and not `ui::tasks::lines` directly — is what both
this draw and `Dashboard::normalise_scroll` derive their line list from, so the drawn slice
and the clamp can never disagree about how many lines there are, **including when the two
bodies produce different line counts for the same source**: a tracked-tasks tab's checklist
is a different length from the same file's markdown rendering, and passing the selected
change to both callers is what keeps the clamp honest across a tab switch. A table is a third
reason the two counts differ from the source's own line count — a wrapped cell makes one
source row several rendered lines — and it needs no new machinery, because the clamp already
counts rendered lines rather than source lines.
`artifact-content` states what `content_lines` returns, including the `No content yet` line,
the `!`-prefixed problem lines, and which of the two bodies applies.

The `Face`-to-`Style` mapping SHALL be `ui::view::style_for`, and its **source** SHALL be
`ui::palette` — the crate's one semantic-role table — rather than modifiers written out at
this call site. `style_for` SHALL remain the crate's only `Face`-to-`Style` function, and it
SHALL be **unchanged** by the checklist body.

Its modifiers SHALL be: `heading` present or `strong` → `Modifier::BOLD`; `emphasis` →
`Modifier::ITALIC`; `code` → `Modifier::DIM`; `link` → `Modifier::UNDERLINED`; `quoted` →
`Modifier::DIM`; and `strikethrough` → `Modifier::CROSSED_OUT`, which is the whole of what
this change adds here. Flags compose, so a bold link's cells carry `BOLD` and `UNDERLINED`
together and a struck bold link's carry `CROSSED_OUT` as well. A checklist heading line
reaches the buffer bold through the same `heading` mapping, and every other checklist line is
plain, so no new mapping is added for it and `ui::tasks` never sets `strikethrough`.

Three of those faces SHALL additionally carry a **foreground colour**: `heading` its level's
colour, `code` `Color::Yellow`, and `link` `Color::Blue`. `strong`, `emphasis`, `quoted`, and
`strikethrough` SHALL carry none — each already carries a modifier that distinguishes it.
`view-palette` states the fold order that composes several faces onto one span and the
foreground precedence — heading over code over link — that decides the colour when a span
carries more than one; this requirement adds no second rule.

The region SHALL draw nothing at all — no header, no tab bar, no rule, no content — when `visible()`
is empty, leaving every interior cell a space whose `Style` equals
`ratatui::buffer::Cell::default().style()`, which is what `responsive-layout`'s
blank-interior scenario asserts. When a change **is** selected, the region SHALL always draw
its header, tab bar and rule, and the content area SHALL always hold at least one line, because
`content_lines` returns `No content yet` rather than nothing.

The two mandated detail interiors are **78 columns by 17 rows** at a 120x20 frame and **58
columns by 16 rows** at a 60x20 frame in the detail route, giving content areas of 78x14 and
58x14. Every scenario below is exercised at both.

#### Scenario: The document fills the detail interior at both mandated widths

The scenario's name is kept verbatim from `markdown-viewer` because a delta's scenario
headers are its merge key; its subject is the same document, drawn two rows lower.

- **WHEN** a `Dashboard` at `Route::Detail`, whose selected change carries one artifact not
  marked `tracks_tasks` and whose one section holds a bullet list of the twenty items
  `line-00` through `line-19`, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer row 5 columns 42 onward reads `- line-00` and row 18
  reads `- line-13`, so fourteen items are drawn into the content area — the same fourteen
  the bordered layout drew, two rows lower and one column right
- **AND** in the 60-column buffer row 5 columns 1 onward reads `- line-00` and row 18 reads
  `- line-13`
- **AND** in both buffers row 0 holds the change header, row 2 the tab bar, row 3 the rule,
  and rows 1 and 4 are blank, so the markdown began below all four rather than over them

#### Scenario: Faces reach the buffer as styles at both widths

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact not
  marked `tracks_tasks` and whose one section holds
  `## Heading\n\n**bold** and *italic* and `code` and [link](u) and ~~struck~~\n` is rendered
  at 120x20 and at 60x20
- **THEN** in each buffer the cells of `## Heading` in the content area's first row report
  `Modifier::BOLD` set, and additionally the foreground `Role::Heading(2)` carries
  (`Color::Cyan`)
- **AND** the cells of `bold` report `BOLD`, of `italic` report `ITALIC`, of `code` report
  `DIM`, of `link` report `UNDERLINED`, and of `struck` report `CROSSED_OUT` — every modifier
  that existed before this change exactly as before it
- **AND** the cells of `code` additionally report the foreground `Role::Code` carries
  (`Color::Yellow`) and those of `link` the foreground `Role::Link` carries (`Color::Blue`),
  while those of `bold`, `italic`, and `struck` report no foreground at all
- **AND** the assertion discriminates: a cell of the surrounding plain text reports none of
  those modifiers and no foreground

#### Scenario: A table reaches the buffer aligned and inside the region

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact not
  marked `tracks_tasks` and whose one section holds a three-column table with a header row,
  a delimiter row, and three body rows — one of whose cells is long enough to wrap at 58 and
  not at 78 — is rendered at 120x20 and at 60x20
- **THEN** in each buffer the `|` characters of the delimiter row fall at exactly the same
  columns as those of every drawn row line, so the alignment survives the draw and is not a
  property of the line list alone
- **AND** in each buffer the header cells' cells report `Modifier::BOLD` while the pipe and
  padding cells report no modifier at all
- **AND** in the 60-column buffer the wrapping row occupies more rows than in the
  120-column buffer
- **AND** no table cell is drawn on a gutter or divider column, stated **per buffer** because
  the two layouts have different ones: in the 120-column buffer none reaches column 0, 39, 40,
  or 41; in the 60-column buffer none reaches column 0 or 59. Column 119 is **not** in that
  list — the wide detail region takes `Gutters::LeftOnly`, so its interior's last column is
  the frame's last column and a table cell may legitimately reach it. Columns 39, 40 and 41
  are ordinary interior content at 60 columns — below the breakpoint `layout::split_body`
  gives the detail region the whole body — and a table wrapping at 58 necessarily covers them,
  so asserting the wide layout's divider columns against the narrow buffer would leave this
  test permanently red against a correct implementation

#### Scenario: An empty source leaves the detail interior blank at both widths

The scenario's name is kept verbatim; its subject moves from "an empty `source`" to "no
change selected", because with a change selected the region is never blank — `No content
yet` is drawn instead, which is `artifact-content`'s requirement and `SPEC.md`'s
degraded-states row.

- **WHEN** a `Dashboard` over `changes::empty_set()` is rendered at 120x20 at `Route::List`
  and a `Dashboard` whose only change is filtered out is rendered at 60x20 at
  `Route::Detail`
- **THEN** every cell of the detail region's heading row, padding row and interior in each
  buffer is a space whose `Style` equals `ratatui::buffer::Cell::default().style()`
- **AND** a third `Dashboard` with a change selected and no sections at all rendered at
  the same two sizes shows `No content yet` in the content area's first row and is therefore
  **not** blank, so the two states are distinguished rather than conflated
- **AND** a fourth `Dashboard`, identical to the third but with its artifact marked
  `tracks_tasks`, also shows `No content yet` and no progress bar, so a marked tab with no
  file is still not blank and still not a checklist

#### Scenario: Content never overwrites the detail region's border

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
there is no border now, and what content must not overwrite is a gutter column or the divider.

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries five artifacts with
  40-character ids, whose name is 200 characters long, and whose one section holds thirty
  lines each 200 characters long, is rendered at 120x20 and at 60x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 0 through 18
  is a space
- **AND** in the 120-column buffer every cell of columns 0, 39, and 41 in rows 0 through 18
  is a space and every cell of column 40 is `│`, while column 119 does carry content because
  the wide detail region has no right gutter
- **AND** no content is written above row 5 or below row 18 in either buffer
- **AND** the same holds with the selected tab marked `tracks_tasks` and the same source
  turned into thirty 200-character task lines, so neither the checklist's wrap nor the
  progress bar's gauge can reach a gutter
- **AND** the same holds again with the source replaced by a twelve-column table whose every
  cell is 200 characters long — which the pipe grammar fits at both interiors, spending each
  exactly — and once more with a fifteen-column one, whose `4n + 1` of 61 the 78-column
  interior clears and the 58-column one does not, so **both** the pipe grammar and the
  one-cell-per-line fallback it degrades to are measured against the gutters

  Corrected during Change Review: the first draft claimed the twelve-column fixture
  exercised the fallback. It does not — `3n + 1` is 37, so `avail` is 41 and 21, both at
  least `n`, and the pipe grammar is used at both widths. The fifteen-column table is what
  makes the sentence true, and the two are kept side by side because the twelve-column one
  is the tightest pipe-grammar case there is.

#### Scenario: A degenerate detail interior draws nothing and does not panic

The frame height a detail interior row costs is three — one frame-footer row and the region's
heading and padding rows — so the interior first has one row at a frame height of **4**, two
at **5**, three at **6**, and four at **7**. `split_detail` spends the first three on the tab
bar, the rule and the content padding row, so the **content area** first has one row at a
frame height of **7**. Those are the heights this scenario samples; 2 and 3 give an interior
of zero rows and would exercise only the earliest guard.

- **WHEN** a `Dashboard` with a change selected and a non-empty section list is rendered
  at 120x3, 120x4, 120x5, 120x6, 120x7, 60x4, 60x5, 60x6, 60x7, 1x20, and 2x20
- **THEN** no render panics at any of them
- **AND** at 120x3 and its narrow counterpart the interior has zero rows and nothing at all is
  drawn inside the region, though the heading row **is** drawn
- **AND** at 120x4 and 60x4 the tab bar is drawn and no rule and no markdown line appears
  anywhere in the frame
- **AND** at 120x5 and 60x5 the tab bar and the rule are drawn and no markdown line appears
- **AND** at 120x6 and 60x6 the same two are drawn and still no markdown line, the interior's
  third row being the content padding row
- **AND** at 120x7 and 60x7 exactly one content row is drawn, holding the source's first
  rendered line
- **AND** every one of those renders is repeated with the selected tab marked
  `tracks_tasks`, where the one content row at 120x7 and 60x7 holds the progress bar rather
  than a task item, and 1x20 and 2x20 — where the interior is one or zero columns wide —
  still draw nothing and still do not panic
- **AND** every one of those renders is repeated once more with a table as the
  `detail.sections`, where 1x20 and 2x20 still draw nothing and still do not panic

### Requirement: Switching to and from the tracked-tasks tab renormalises the scroll

`Dashboard::sync_detail` already resets `detail.scroll` to zero whenever the
`(change directory, tab)` key changes, so a tab switch never lands mid-document. Because the
two bodies produce different line counts for the same source, `Dashboard::normalise_scroll`
SHALL compute its clamp from the **same** `content_lines` call the draw uses, with the same
`selected_change()` argument, so a scroll offset valid for one body is never applied against
the other's line count.

`run_loop` SHALL call `sync_detail` before the draw and `normalise_scroll` after it, in the
order `dashboard-loop` already specifies; this change adds no loop step and no new call.

#### Scenario: Scrolling the checklist is clamped against the checklist's own length

- **WHEN** `run_loop` runs against a `TestBackend` of 120x20 and again of 60x20, with a
  scripted source delivering twenty Presses of `Char('j')` then `Char('q')`, over a
  `Dashboard` at `Route::Detail` whose selected change's only artifact is marked
  `tracks_tasks`, whose `progress` is `Progress { completed: 0, total: 20 }`, and whose file
  reads as twenty unchecked task lines under one heading
- **THEN** the run ends with `dashboard.detail.scroll` clamped to the checklist's own length
  less the content area's fourteen rows, not to `20`
- **AND** the final buffer's content area's last row holds the checklist's last item, so the
  clamp used the body that was actually drawn
- **AND** the clamped value differs from the value the same source's markdown rendering
  would give, so the scenario discriminates between the two bodies

#### Scenario: A tab move away from the checklist resets and reclamps

- **WHEN** the same run continues with a Press of `Char('1')` — selecting a different,
  unmarked artifact — followed by ten more Presses of `Char('j')`
- **THEN** `detail.scroll` is `0` immediately after the tab move, because `sync_detail`'s
  key changed
- **AND** after the ten further presses it is clamped against the markdown body's line count
  rather than the checklist's
- **AND** no render in the run panics at either width

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

`Detail` SHALL carry exactly these **five** fields after `live-refresh` too. That change's
forced-reload flag deliberately lives on `ui::app::Refresh` rather than here: `Dashboard`
gains one field either way, and putting it on `Refresh` leaves `Detail`'s five — and every
`Detail { … }` literal in the crate — untouched. `live-updates` states the flag's contract
and `artifact-content` states what `sync_detail` does with it.

`Detail` SHALL NOT implement `Default` — neither derived nor hand-written, anywhere in the
crate — and every construction and every destructuring of it SHALL name **all five** fields,
with no `..` rest, on exactly the terms `dashboard-loop` states for `Dashboard`, `Filter`,
and (from `live-refresh`) `Refresh`.

#### Scenario: `Detail` has no `Default` and no site elides a field

- **WHEN** every `*.rs` file under `src/` is searched, for the type name `Detail`, for
  `impl Default for Detail`, for a `Default` inside the `#[derive(...)]` immediately
  preceding `struct Detail`, and for a `..` appearing inside a `Detail { … }` literal or
  pattern
- **THEN** there is no match
- **AND** the search is the same parameterised check that covers `Dashboard`, `Filter`, and
  `Refresh`, run over the type list `Dashboard Filter Detail Refresh`, rather than a second
  drifting check. `Refresh` is `live-refresh`'s addition to that list; the check's own
  `TYPES` parameter is what makes adding it a change to an invocation rather than to the
  check
- **AND** it is paired with a positive control asserting that `src/ui/app.rs` **does**
  contain `struct Detail {`, and the check is proven able to fail against a copy carrying
  `impl Default for Detail { … }` and against a copy carrying `let Detail { source, .. }`
- **AND** a compile-time companion exists: a test destructures a `Detail` with an
  exhaustive pattern naming all five fields and no `..`, the `Dashboard` companion
  continues to name all **nine** — eight before `live-refresh`, plus `refresh` — and a
  fourth companion destructures a `Refresh` naming all three

#### Scenario: Startup leaves the detail empty and unscrolled

- **WHEN** `ui::load` is called over a scratch repository holding one change
- **THEN** the returned `Dashboard`'s `detail.sections` is empty, `detail.problems` is empty,
  and `detail.scroll`, `detail.tab`, and `detail.loaded` are `0`, `0`, and `None`
- **AND** the same holds when `ui::load` finds **no** `openspec/` directory above its
  starting path and takes its `RepoSearch::NotFound` arm, which is a second `Dashboard`
  construction site and therefore a second place the field can be got wrong
- **AND** `refresh.reload` is **false** at both construction sites, so nothing forces a
  re-read before the loop's first ordinary sync; `refresh.requested` is true at both, which
  is `dashboard-loop`'s clause and not `Detail`'s
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

`ui::layout::interior(area: Rect, gutters: Gutters) -> Rect` SHALL be the one place in the
crate that computes a region's interior, on the arithmetic `responsive-layout`'s region shape
states: the origin advanced by the left gutter's column and **two** rows and **clamped to the
rectangle's own right and bottom edges**, with the width reduced by the gutter columns and the
height by two, each saturating to zero. The clamp is not decoration — at
`Rect::new(0, 0, 0, 0)` it is the difference between `Rect { x: 0, y: 0, … }` and
`Rect { x: 1, y: 1, … }`, and `Rect` compares all four fields.

It no longer agrees with `ratatui::widgets::Block::bordered().inner`, and that is the point:
a bordered block reserves a row at the **bottom** for its border, while a region reserves a
second row at the **top** for its padding row. The two agree on `x` and on width for
`Gutters::Both`, and differ by one on `y` and by one on height.

`ui::layout::split_detail(interior: Rect) -> (Rect, Rect, Rect)` SHALL be the one place in
the crate that divides that interior into the tab-bar row, the rule row, and the content
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
  seventeen, which is what `detail-view` changed about the caller — the content area is
  fourteen rows under `pane-chrome` exactly as it was before it

#### Scenario: `interior` agrees with a bordered block's own inner rectangle

The scenario's name is kept verbatim because a delta's scenario headers are its merge key. It
now pins the **disagreement** deliberately, so a future edit that quietly restores the
bordered arithmetic fails here rather than silently losing the padding row.

- **WHEN** `interior` with `Gutters::Both` and `ratatui::widgets::Block::bordered().inner` are
  both applied to `Rect::new(0, 0, 40, 19)`, `Rect::new(0, 0, 60, 19)`, `Rect::new(0, 0, 2, 2)`,
  `Rect::new(0, 0, 1, 1)`, and `Rect::new(0, 0, 0, 0)`
- **THEN** on each non-degenerate rectangle the two agree on `x`, on `width`, and on
  `height` — both subtract two rows — and disagree on `y` by exactly one, `interior` giving
  the larger `y`. The bordered arithmetic spent its two rows one above and one below; this
  one spends both above, which moves the origin and leaves the height alone
- **AND** `interior(Rect::new(0, 0, 40, 19), Gutters::Both)` is `Rect::new(1, 2, 38, 17)`
  while the block's inner is `Rect::new(1, 1, 38, 17)`, compared as whole `Rect` values — all
  four fields, not width and height alone
- **AND** the two degenerate rectangles yield zero width and zero height rather than
  underflowing, with `Rect::new(0, 0, 1, 1)` giving `Rect { x: 1, y: 1, width: 0, height: 0 }`
  and `Rect::new(0, 0, 0, 0)` giving `Rect { x: 0, y: 0, width: 0, height: 0 }` — the origin
  clamp, which both share

#### Scenario: A scroll offset past the end still draws the last screenful

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact, whose
  whose one section holds the twenty-item list, and whose `detail.scroll` is `99`, is rendered at
  120x20 and at 60x20
- **THEN** in the 120-column buffer the content area's first row reads `- line-06` and its
  last drawn row reads `- line-19`
- **AND** in the 60-column buffer the same two rows read the same, so the fourteen-row
  content area — not the seventeen-row interior — is what the clamp used
- **AND** no row above the content area and no row below row 18 carries markdown

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

**An action that does not move `route` SHALL NOT reset `detail.scroll`, and each of the three
arms above SHALL guard on that.** `OpenDetail` SHALL reset the scroll only when
`self.route != Route::Detail` — that is, only when it is about to change the route — matching
`Back`'s and `FilterStart`'s existing `before != after` guards, which is the rule this
sentence generalises rather than a new one. `OpenDetail` alone did not guard: it assigned the
route and zeroed the scroll unconditionally whenever the filter was inactive.

That was observable, not theoretical. In the wide layout — every width at or above the
100-column breakpoint — `responsive-layout` draws **both** regions regardless of route, so at
`Route::Detail` an `Enter` moves nothing on screen and its only visible effect was to throw
the reader back to line one of a `design.md` they were forty lines into. The requirement's own
wording already said "every action that moves `route`"; the implementation fired on an action
that moved none, and this clause is what closes that gap. The intended behaviour, stated
plainly: **`Enter` at the detail route is a no-op.** It is not rebound, it is not given a new
meaning such as "reload" or "scroll to top", and nothing else about the key changes — a
keybinding change would be **BREAKING** and this is deliberately not one.

`Back` at `Route::List` with no filter layer to dismiss remains a no-op that resets nothing,
which the same guard already gives it.

#### Scenario: At the detail route the content scrolls by one line at both widths

- **WHEN** a `Dashboard` whose one section holds the twenty-item list, whose `route` is
  `Route::Detail`, and whose `detail.scroll` is `0` is given a `Next` action, then a second
  `Next`
- **THEN** `detail.scroll` is `1`, then `2`, and `selected` is unchanged throughout
- **AND** rendering after the second action at 120x20 puts `- line-02` at row 5, columns 42
  through 50, and `- line-15` at row 18
- **AND** rendering after the second action at 60x20 puts `- line-02` at row 5, columns 1
  through 9, and `- line-15` at row 18

  The landed text named rows 2 and 17 and the line `line-17`, which predate `detail-view`'s
  own header and tab bar: a fourteen-row content area holding `line-02` first ends at
  `line-15`, not `line-17`. `pane-chrome` corrects the arithmetic while it is moving the two
  row indices anyway, rather than copying a stale expectation forward.

#### Scenario: At the list route the same actions still move the selection

- **WHEN** a `Dashboard` with three active changes, a non-empty `detail.sections`, `route` of
  `Route::List`, and `detail.scroll` of `0` is given two `Next` actions
- **THEN** `selected` is `2` and `detail.scroll` is still `0`
- **AND** rendering at 120x20 and at 60x20 puts the `>` marker on the third list row in
  both, so the contrast with the detail route is observed on screen and not only in state

#### Scenario: Scrolling stops at the top

- **WHEN** a `Dashboard` at `Route::Detail` whose one section holds the twenty-item list and
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

- **WHEN** a `Dashboard` at `Route::Detail` whose one section holds the twenty-item list and
  whose `detail.scroll` is `3` is given a `Back` action, and then an `OpenDetail` action
- **THEN** `detail.scroll` is `0` after the `Back` and still `0` after the `OpenDetail`
- **AND** a second `Dashboard` in the same state given a `FilterStart` action instead has
  `route` `List` and `detail.scroll` `0`, while a third at `Route::Detail` with
  `filter.active` true and `detail.scroll` `3` given a `Back` — which dismisses the filter
  layer and not the route — still has `detail.scroll` `3`
- **AND** rendering the first dashboard at 120x20 and at 60x20 after the `OpenDetail` puts
  `- line-00` on the interior's first row in both

#### Scenario: `Enter` at the detail route moves nothing and keeps the scroll

- **WHEN** a `Dashboard` at `Route::Detail` with `filter.active` false, whose `detail.sections`
  is the twenty-item list and whose `detail.scroll` is `7`, is given an `OpenDetail` action,
  and then a second and a third
- **THEN** `route` is `Route::Detail` and `detail.scroll` is `7` after each of the three
- **AND** `selected`, `filter`, and every other field of the `Dashboard` are unchanged, so
  the action is a no-op in state and not only in the scroll field
- **AND** rendering at 120x20 before and after the three actions produces byte-identical
  buffers, which is the width band where both regions are drawn and the route move was
  invisible; rendering at 60x20 likewise produces byte-identical buffers, with `- line-07`
  on the content area's first row in each

#### Scenario: `Enter` from the list route still opens at the top

- **WHEN** a `Dashboard` at `Route::List` with `filter.active` false, whose `detail.sections`
  is the twenty-item list and whose `detail.scroll` is `7` — a value left behind by an
  earlier session at the detail route — is given an `OpenDetail` action
- **THEN** `route` is `Route::Detail` and `detail.scroll` is `0`, because the action moved
  the route
- **AND** rendering at 120x20 and at 60x20 puts `- line-00` on the content area's first row
  in both, so the guard narrowed the reset to real route moves and did not remove it

#### Scenario: `Enter` while filtering still dismisses the filter and resets nothing

- **WHEN** a `Dashboard` at `Route::Detail` with `filter.active` true, a query of `add`, and
  `detail.scroll` of `7` is given an `OpenDetail` action
- **THEN** `filter.active` is false, `filter.query` is still `add`, `route` is still
  `Route::Detail`, and `detail.scroll` is still `7`
- **AND** the same dashboard at `Route::List` given the same action has `filter.active`
  false, `route` still `Route::List`, and `detail.scroll` still `7`, so accepting a filter
  is not a route move at either route

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

### Requirement: `ScrollDown` and `ScrollUp` scroll the detail content at either route

`Dashboard::apply(Action::ScrollDown)` SHALL add one to `detail.scroll` and
`Action::ScrollUp` SHALL subtract one, saturating at zero, **regardless of `route`** —
unlike `Next` and `Prev`, which do the same thing only at `Route::Detail`. The wheel names
the region it is over, so the wide layout's detail region scrolls while the list route is
current.

Both SHALL change nothing else: not `selected`, not `detail.tab`, not `route`, not
`filter`, not `changes`, not `agents`, not `agent_names`, not `launch`.

The upper bound SHALL stay where `detail-scroll` already puts it — the draw-time clamp in
`render_detail` and the per-frame `Dashboard::normalise_scroll` — not in `apply`, so a wheel
held down cannot run the stored offset arbitrarily far ahead any more than a held `j` can.

`Action::Next` at `Route::Detail` SHALL be exactly `Action::ScrollDown` and `Action::Prev`
at `Route::Detail` exactly `Action::ScrollUp`, through one shared implementation, so a key
and a wheel over the same region can never disagree about what one line means.

#### Scenario: The wheel scrolls the detail region at the list route

- **WHEN** a dashboard at `Route::List` at 120x40, with a forty-line artifact selected, is
  given `Action::ScrollDown` three times
- **THEN** `detail.scroll` is `3` and `selected` is unchanged
- **AND** the drawn detail region shows the content advanced by three lines while the list
  region still shows the same selected row
- **AND** `route` is still `Route::List`

#### Scenario: `ScrollUp` stops at the top

- **WHEN** `Action::ScrollUp` is applied to a dashboard whose `detail.scroll` is `0`, at
  both routes
- **THEN** `detail.scroll` stays `0` and nothing else changes

#### Scenario: A held wheel is clamped by the frame, not by `apply`

- **WHEN** `Action::ScrollDown` is applied five hundred times to a dashboard whose selected
  artifact renders twelve lines, and the frame is then drawn at 120x40
- **THEN** the drawn content shows the last screenful rather than a blank region
- **AND** after `normalise_scroll`, `detail.scroll` is clamped to the same value a held `j`
  at the detail route leaves behind

#### Scenario: `Next` at the detail route and `ScrollDown` are the same move

- **WHEN** a dashboard at `Route::Detail` is driven once by `Action::Next` and once by
  `Action::ScrollDown`
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** the same holds for `Action::Prev` against `Action::ScrollUp`

### Requirement: `SelectNext` and `SelectPrev` move the list selection at either route

`Dashboard::apply(Action::SelectNext)` and `Action::SelectPrev` SHALL move and clamp
`selected` per `list-selection` and reset `detail.tab` and `detail.scroll` to `0` exactly
when `selected` changed value — **regardless of `route`**, unlike `Next` and `Prev`, which
do this only at `Route::List`.

They SHALL change nothing else, and in particular SHALL NOT change `route`: a wheel over
the list region while the detail route is current moves the selection and leaves `j` and
`k` scrolling the content, because nothing about a wheel says the reader wants to change
which region the keys address.

`Action::Next` at `Route::List` SHALL be exactly `Action::SelectNext` and `Action::Prev` at
`Route::List` exactly `Action::SelectPrev`, through one shared implementation.

#### Scenario: The wheel moves the selection at the detail route

- **WHEN** a dashboard at `Route::Detail` at 120x40 with six active changes, `detail.tab`
  `2`, and `detail.scroll` `9` is given `Action::SelectNext`
- **THEN** `selected` has advanced by one, `detail.tab` is `0`, and `detail.scroll` is `0`
- **AND** `route` is still `Route::Detail`, so the detail region now shows the newly
  selected change

#### Scenario: A clamped move resets nothing

- **WHEN** `Action::SelectNext` is applied to a dashboard whose cursor is already on the
  last target, with `detail.tab` `2` and `detail.scroll` `9`
- **THEN** `selected`, `detail.tab`, and `detail.scroll` are all unchanged
- **AND** the same holds for `Action::SelectPrev` at the first target

#### Scenario: `Next` at the list route and `SelectNext` are the same move

- **WHEN** a dashboard at `Route::List` is driven once by `Action::Next` and once by
  `Action::SelectNext`
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** the same holds for `Action::Prev` against `Action::SelectPrev`
