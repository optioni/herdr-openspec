## MODIFIED Requirements

### Requirement: The content area renders the artifact, its problems, or `No content yet`

`ui::detail::content_lines(detail: &Detail, change: Option<&Change>, width: u16) ->
Vec<markdown::Line>` SHALL be the single line list both `ui::view::render` and
`Dashboard::normalise_scroll` derive the detail content from, so the drawn slice and the
clamp can never disagree. It SHALL return:

- one line per entry of `detail.problems`, each the text `"! <problem>"` passed through
  `ui::list::pad_or_truncate_right` at `width`, followed by
- the selected tab's **body**, which is:
  - `ui::tasks::lines(&detail.source, &change.progress, width)` — `tasks-checklist`'s
    grammar and `tasks-progress-bar`'s leading line — when `change` is `Some` and the
    `ArtifactRef` at `detail.tab` carries `tracks_tasks == true`, and
  - `ui::markdown::lines(&detail.source, width)`, unchanged, in every other case,
    including a `None` change, a `detail.tab` past the end of the artifact list, and a
    change carrying no artifacts at all;
- and, when `detail.problems` is empty **and** `detail.source` is empty, exactly one line
  reading `No content yet` — the state `SPEC.md`'s degraded-states table names for a missing
  artifact file — **passed through `ui::list::pad_or_truncate_right` at `width`**, on
  exactly the terms every problem row, task item, heading, and progress-bar line already is.

That last clause is the repair, not a restatement. `No content yet` was pushed as a bare
`String` while every neighbouring line went through the padding, and this requirement's own
promise — that no returned line exceeds `width` — was therefore false at every `width` below
14: at a 15-column narrow **frame** in the detail route, whose content area is 13 columns,
the rendered row read `│No content yet` and ate the region's right border. The literal is now
truncated with the same `…` rule as everything else, so at `width` 13 it reads
`No content y…`, at 12 `No content …`, at 11 `No content…`, and at 0 it is the empty string.
Every `width` in this paragraph is the **content area's**, which is the frame's less the
region's two border columns; the scenario below gives frame widths and says so.
The line SHALL still be a single `Segment` carrying `Face::plain()`.

The `change` argument is the **only** reason the tracked-tasks decision is made once rather
than at each of the two call sites; both callers SHALL pass `Dashboard::selected_change()`
and SHALL NOT decide the grammar themselves.

Both bodies SHALL return nothing for an empty `detail.source`, so the `No content yet` rule
above is unaffected by which body was selected: an artifact file that does not exist reads
`No content yet` whether or not it is the tracked-tasks artifact, and never reads
`tasks-checklist`'s `No tasks yet`.

When `detail.problems` is non-empty and `detail.source` is empty, the problems alone SHALL be
returned and `No content yet` SHALL NOT appear: the reason is known, and reporting both would
say two contradictory things about the same tab.

Every line SHALL carry plain faces except those `markdown::lines` and `ui::tasks::lines`
produce; `content_lines` SHALL name no `ratatui` type, on the same terms `ui::markdown`,
`ui::list`, and `ui::tasks` do not.

**Every line `content_lines` returns SHALL measure at most `width` display columns**, as
`responsive-layout` defines them, at **every** `width` — not only at the two mandated
interiors. A line that is exactly `width` columns is permitted; one column over is not. This
is stated as a total property rather than as two width cases because the two mandated widths
are both at or above 14 and could not see the `No content yet` overflow at all.

`ui::view::render` SHALL draw the slice of `content_lines` that starts at
`layout::scroll_offset(lines.len(), detail.scroll, content.height)` and runs for at most
`content.height` lines, into the content area `layout::split_detail` returns, starting at its
first row and first column, never writing past the interior's last column.

The per-segment draw loop SHALL advance its cursor by the **columns `set_string` consumed**,
`layout::columns(&segment.text)`, and not by the segment's `char` count. Under a `char`
count the cursor under-counted for any wide character, so the `x >= last_col` guard that
stops the row never fired and the segments after it were written past the region — into the
neighbouring region's border and beyond — because `Buffer::set_string` clips at the
**buffer's** right edge, never at the region's. The loop SHALL additionally clamp each
segment to the columns remaining, drawing `layout::truncate_columns(&segment.text, last_col
- x)` rather than the whole segment, so a single over-wide segment cannot escape the region
even at the moment the guard is about to fire. The guard SHALL remain, unchanged, as the
loop's exit condition.

#### Scenario: A missing artifact still shows its tab and reads `No content yet`

- **WHEN** a `Dashboard` whose selected change carries the five tdd artifacts, whose
  `detail.tab` is `1`, whose `detail.source` and `detail.problems` are empty, is rendered at
  120x20 and at 60x20 at `Route::Detail`
- **THEN** in the 120-column buffer row 4, columns 41 onward, begins `No content yet`
- **AND** in the 60-column buffer row 4, columns 1 onward, begins `No content yet`
- **AND** in both buffers row 3 still holds the full five-tab bar with `2 specs` bold, so a
  missing file removes neither the tab nor the header
- **AND** the same holds with `detail.tab` set to `3`, the tracked-tasks position: a missing
  tasks artifact reads `No content yet` and shows no progress bar
- **AND** in both buffers that row measures exactly the interior width — 78 and 58 — because
  the literal is now padded like every line around it

#### Scenario: `No content yet` does not eat the border at a narrow frame

- **WHEN** the same empty-source `Dashboard` at `Route::Detail` is rendered at 15x20, at
  14x20, at 13x20, at 2x20, and at 1x20
- **THEN** at 15x20 — a frame of 15, so a content area of 13 — the content area's first row
  reads `No content y…` and the frame's right border column is a box-drawing character, not
  the letter `t`: the audit's `│No content yet` row is gone
- **AND** at 14x20 the row reads `No content …` (a content area of 12) and at 13x20
  `No content…` (a content area of 11), each exactly its content area's width in columns —
  the frame is two columns wider than the region it holds, which is why the frame widths and
  the truncation points differ by two
- **AND** at every one of the five widths the row's `layout::columns` is at most the content
  area's width
- **AND** at 2x20 and 1x20 the content area is zero or one column wide and the row is empty
  or a single `…`
- **AND** no buffer writes a cell past its last column and none of the five renders panics

#### Scenario: A read failure is named above the content at both widths

- **WHEN** a `Dashboard` whose `detail.problems` is `["/repo/specs/a/spec.md: permission
  denied"]` and whose `detail.source` is `# b\n` is rendered at 120x20 and at 60x20 at
  `Route::Detail`
- **THEN** in each buffer the content area's first row begins `! /repo/specs/a/spec.md:` and
  its second row begins `# b`
- **AND** `No content yet` appears in neither buffer
- **AND** each problem row is exactly the interior width — 78 and 58 — so it neither
  overwrites the border nor leaves a partial cell

#### Scenario: The rendered markdown fills the content area, not the whole interior

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact,
  **not** marked `tracks_tasks`, and whose `detail.source` is twenty lines reading
  `- line-00` through `- line-19` is rendered at 120x20 and at 60x20
- **THEN** in each buffer the header row and the tab row are unchanged and the content
  area's first row holds `- line-00`
- **AND** the content area's last drawn row is `- line-13`, because the 16-row interior gives
  the content area 14 rows, so the header and the tab bar took two rows from the markdown
  rather than being drawn over it

#### Scenario: The tracked-tasks tab renders the checklist body instead

- **WHEN** the same `Dashboard` has its one artifact marked `tracks_tasks == true`, a
  `progress` of `Progress { completed: 0, total: 0 }`, and is rendered at 120x20 and at
  60x20
- **THEN** in each buffer the content area's first row holds the progress bar `[-]`, its
  second row is blank, and its third row reads `No tasks yet` — the twenty `- line-NN`
  bullets hold no task lines
- **AND** the header row and the tab row are byte-identical to the markdown render, so only
  the body changed

#### Scenario: A wide-character document stays inside the detail region

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one unmarked
  artifact, and whose `detail.source` is a document holding a paragraph of forty repetitions
  of `日本語`, a `# 🎉 見出し` heading, and a bullet holding a family emoji joined by
  zero-width joiners, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer every cell of column 39 is the list block's right border
  and every cell of column 40 is the detail block's left border, unchanged from the same
  render with an ASCII document — the audit's escaped row is gone
- **AND** in the 60-column buffer every cell of column 0 and column 59 is a border character
- **AND** in both buffers every drawn content row's own `layout::columns` is at most the
  content area's width, and no cell outside the content area was written

#### Scenario: `content_lines` is total and width-parameterised

- **WHEN** `content_lines` is called at width `78` and at width `58`, and with `change` set
  to `None`, to a change with no artifacts, to a change whose artifact at `detail.tab` is
  marked, and to one whose is not, over: an empty `Detail`; one with problems only; one with
  a source only; one with both; one whose source is a 200-character paragraph; one whose
  source is a 200-column CJK paragraph; and one whose `tab` is past the end of the artifact
  list
- **THEN** no call panics at either width for any combination
- **AND** the wrapped paragraph produces strictly more lines at `58` than at `78`, so the
  width genuinely reaches both bodies
- **AND** no returned line's `layout::columns` exceeds the width it was called with

#### Scenario: No `content_lines` line exceeds its width at any width

- **WHEN** `content_lines` is called at **every** width from `0` through `130`, for each of
  the same seven `Detail` values and four `change` values above, and additionally for an
  empty `Detail` with a `None` change — the `No content yet` case
- **THEN** no call panics, and at every width every returned line's `layout::columns` is at
  most that width
- **AND** the `No content yet` case is included at widths `0` through `13`, the range in
  which the literal is longer than the region — the range no mandated-width test can reach,
  which is why this property is stated over all widths rather than adding a narrow case to
  the mandated pair
