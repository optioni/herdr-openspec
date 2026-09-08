## MODIFIED Requirements

### Requirement: The detail region draws the rendered markdown document

`ui::view::render` SHALL fill the detail region's **content area** — the rectangle
`layout::split_detail` returns below the header row and the tab-bar row `detail-header` and
`artifact-tabs` own — with
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
artifact is allocated against. Only the **height** changes: a 16-row interior gives the
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

- **WHEN** a `Dashboard` at `Route::Detail`, whose selected change carries one artifact not
  marked `tracks_tasks` and whose `detail.source` is a bullet list of the twenty items
  `line-00` through `line-19`, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer row 4 columns 41 onward reads `- line-00` and row 17
  reads `- line-13`, so fourteen items are drawn into the content area
- **AND** in the 60-column buffer row 4 columns 1 onward reads `- line-00` and row 17 reads
  `- line-13`
- **AND** in both buffers row 2 holds the change header and row 3 the tab bar, so the
  markdown began below them rather than over them

#### Scenario: Faces reach the buffer as styles at both widths

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact not
  marked `tracks_tasks` and whose `detail.source` is
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
  marked `tracks_tasks` and whose `detail.source` is a three-column table with a header row,
  a delimiter row, and three body rows — one of whose cells is long enough to wrap at 58 and
  not at 78 — is rendered at 120x20 and at 60x20
- **THEN** in each buffer the `|` characters of the delimiter row fall at exactly the same
  columns as those of every drawn row line, so the alignment survives the draw and is not a
  property of the line list alone
- **AND** in each buffer the header cells' cells report `Modifier::BOLD` while the pipe and
  padding cells report no modifier at all
- **AND** in the 60-column buffer the wrapping row occupies more rows than in the
  120-column buffer, and in neither does any table cell reach column 0, column 39, column 40,
  or the last column, so nothing was drawn over a border

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
- **AND** a fourth `Dashboard`, identical to the third but with its artifact marked
  `tracks_tasks`, also shows `No content yet` and no progress bar, so a marked tab with no
  file is still not blank and still not a checklist

#### Scenario: Content never overwrites the detail region's border

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries five artifacts with
  40-character ids, whose name is 200 characters long, and whose `detail.source` is thirty
  lines each 200 characters long, is rendered at 120x20 and at 60x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 1 through 18
  is a box-drawing character
- **AND** in the 120-column buffer every cell of columns 0, 39, 40, and 119 in rows 1
  through 18 is a box-drawing character
- **AND** no content is written above row 2 or below row 17 in either buffer
- **AND** the same holds with the selected tab marked `tracks_tasks` and the same source
  turned into thirty 200-character task lines, so neither the checklist's wrap nor the
  progress bar's gauge can reach the border
- **AND** the same holds again with the source replaced by a twelve-column table whose every
  cell is 200 characters long, so neither the pipe grammar nor the one-cell-per-line
  fallback it degrades to can reach the border

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
- **AND** every one of those renders is repeated with the selected tab marked
  `tracks_tasks`, where the one content row at 120x7 and 60x7 holds the progress bar rather
  than a task item, and 1x20 and 2x20 — where the interior is one or zero columns wide —
  still draw nothing and still do not panic
- **AND** every one of those renders is repeated once more with a table as the
  `detail.source`, where 1x20 and 2x20 still draw nothing and still do not panic
