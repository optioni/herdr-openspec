## MODIFIED Requirements

### Requirement: `Dashboard::sync_detail` resolves the selected tab's content

`ui::app::Dashboard::sync_detail(&mut self, read: ArtifactReader)` SHALL, in this order:

0. Take and clear `refresh.reload`, holding the taken value as `force` for the rest of the
   call. Taking it here — rather than reading it later — is what makes one flag produce
   exactly one re-read, on every path out of the function including the empty-list one.
1. When `visible()` is empty, clear `detail.sections`, `detail.problems`, and
   `detail.expanded`, set `detail.tab` and `detail.scroll` to `0`, set `detail.loaded` to
   `None`, and return.
2. Clamp `detail.tab` against the selected change's `artifacts`: `0` when the list is empty,
   otherwise at most `artifacts.len() - 1`. This is the one place the `(change, tab)`
   invariant is restored, so a `/` filter edit that moves the selection to a change with
   fewer artifacts cannot leave the tab out of range for a frame.
3. Build the key `(selected change's `dir`, `detail.tab`)` and return unchanged when it
   equals `detail.loaded` **and `force` is false**. The key is the change's **directory**,
   never its name and never its index: an active `add-auth` and an archived `add-auth` share
   a name, and a filter edit changes which change an index addresses without changing the
   index. `force` is the second exit condition because the key does not change when a
   **file's content** does — an agent saving the very artifact on screen leaves the key
   identical, and without `force` the pane would keep showing the bytes it read at startup.
4. Otherwise re-read: clear `detail.problems`, then for each path of the selected artifact's
   `paths` **in the order `changes::from_files` resolved them**, call `read(path)`. Every
   `Err(e)` contributes no section and appends the problem `"<path>: <e>"`. Every `Ok(text)`
   contributes one or more `ArtifactSection` values by `artifact-folds`' derivation: an
   optional file section, an optional `None`-labelled preamble section, and one section per
   heading `ui::app::split_headings` returned, each carrying its own `depth` — or, when the file
   does not split, exactly one section carrying the whole text, which is every prose artifact
   and is unchanged from before this change.
5. Set `detail.scroll` to `0` and clear `detail.expanded` **exactly when the key changed**,
   and on that same condition **seed** `detail.expanded` per `artifact-folds` when the
   selected `ArtifactRef` carries `tracks_tasks == true` — every section whose subtree is
   incomplete, and no other. Then set `detail.loaded` to the key.

The split is decided from the text the reader returned and from `tracks_tasks`, and from
nothing else: no path, no artifact id, and no schema lookup. `artifact-folds` states the
gate. `sync_detail` SHALL call `ui::app::split_headings` at most once per successfully read path
per re-read — the row list is derived on every draw, but the section list is not.

Step 4 does not concatenate. The separating `\n` this requirement once inserted between two
files that would otherwise run together on one line is unnecessary once each file is its own
section: a section's body is rendered on its own lines under its own header, and a file that
ends without a newline can no longer collide with the next file's first line. An unsplit
section's `text` is therefore the reader's bytes unmodified, and a split file's sections
partition those same bytes, less the heading lines that became labels — which is what lets
`artifact-folds`' label rule and `ui::markdown` each see exactly what is on disk.

Resetting the scroll and the fold state on a key change and not on a forced reload is the
whole point of the split: a tab or change move starts at the top with every section
collapsed — or, on the tasks tab, with the unfinished groups open — while an agent's save
re-renders the document under a reader who is halfway down it, and inside a section they
opened, without throwing them back to line one, folding it shut, or re-seeding the folds
around a task that was just checked off. `layout::scroll_offset`, `layout::viewport`, and
`Dashboard::normalise_scroll` still clamp the preserved position against the new content's
length, so a document that shrank is corrected within one frame.

`sync_detail` SHALL be called by `ui::driver::run_loop` once per iteration, **before** the
draw and **after** the loop's live tier has adopted any result waiting for it, so the very
first frame shows content rather than a blank region that fills in on the second, and a
corrected change set is read in the same iteration that adopted it. It SHALL NOT be called
by any view.

`sync_detail` SHALL be total: it SHALL NOT panic for any dashboard state, any artifact list,
any `detail.tab`, any value of `refresh.reload`, any reader behaviour including one that
fails on every path, and any file content including one that is a single unterminated fence.

#### Scenario: The selected tab's file is read once and reused

- **WHEN** a `Dashboard` whose selected change carries artifacts `proposal` →
  `[/repo/p.md]` and `design` → `[/repo/d.md]`, with `detail.tab: 0` and `refresh.reload`
  false, is given `sync_detail` with a recording reader returning `# proposal` for any path,
  three times in a row
- **THEN** `detail.sections` holds one entry whose `text` is `# proposal`,
  `detail.problems` is empty, and `detail.loaded` is `Some((change dir, 0))`
- **AND** the reader recorded exactly **one** call, for `/repo/p.md`, so an unchanged
  selection re-reads nothing

#### Scenario: A forced reload re-reads the same key and keeps the scroll

- **WHEN** the same dashboard, already synced once and scrolled to `detail.scroll: 6`, has
  `refresh.reload` set and is synced again with a reader now returning `# proposal (edited)`
- **THEN** the reader recorded a **second** call for `/repo/p.md`
- **AND** `detail.sections` holds one entry whose `text` is `# proposal (edited)`
- **AND** `detail.scroll` is still `6` and `detail.expanded` is unchanged, because the key
  did not change
- **AND** `refresh.reload` is false afterwards, and a third sync with the flag still clear
  records no further call

#### Scenario: A tab move under a forced reload still resets the scroll

- **WHEN** a `Dashboard` scrolled to `6`, with `detail.expanded` holding `1`, has both
  `detail.tab` moved from `0` to `1` and `refresh.reload` set, and is then synced
- **THEN** the reader was called with `/repo/d.md`, `detail.scroll` is `0`, and
  `detail.expanded` is empty
- **AND** the reset is attributable to the key change rather than to the flag, because the
  previous scenario proves a forced reload alone preserves both

#### Scenario: Switching the tab re-reads, and so does switching the change

- **WHEN** the same dashboard is synced, then given `NextTab`, then synced again, then its
  `selected` is moved to a second change and it is synced a third time
- **THEN** the reader recorded three calls, for `/repo/p.md`, `/repo/d.md`, and the second
  change's first artifact path, in that order
- **AND** `detail.sections` holds the second change's text at the end, and `detail.scroll`
  is `0` and `detail.expanded` empty after each re-read

#### Scenario: Two changes with the same name are distinguished by directory

- **WHEN** a `Dashboard` holding an active change named `add-auth` at
  `/repo/openspec/changes/add-auth` and an archived change also named `add-auth` at
  `/repo/openspec/changes/archive/2026-08-14-add-auth`, each with one artifact resolving to
  a distinct path, is synced with `selected: 0`, then `selected` is set to `1` and it is
  synced again
- **THEN** the reader recorded two calls, one per directory
- **AND** `detail.sections` holds the archived change's text after the second sync, so
  keying on the name alone would have shown the wrong artifact

#### Scenario: A multi-file artifact is concatenated in path order with a separating newline

The scenario's name is kept verbatim because a delta's scenario headers are its merge key. It
pins the **absence** of a separator, which is what the section split replaced it with.

- **WHEN** a `Dashboard` whose selected artifact resolves to `[<dir>/specs/a/spec.md,
  <dir>/specs/b/spec.md]` is synced with a reader returning `# a` for the first (no trailing
  newline) and `# b\n` for the second
- **THEN** `detail.sections` holds two entries, labelled `Some("a")` and `Some("b")` in that
  order, both at `depth` `0`, whose `text` values are `# a` and `# b\n` — the reader's bytes
  verbatim, with no separator inserted and no newline added, and neither file splitting,
  because neither carries a `### Requirement:` heading
- **AND** rendering at 120x20 and at 60x20 in the detail route puts `> a` on the content
  area's first row and `> b` on its second, so the two files cannot run together on one line
  even though the first ends without a newline

#### Scenario: A split file is partitioned rather than copied

- **WHEN** a `Dashboard` whose selected artifact resolves to one path is synced with a reader
  returning
  `## ADDED Requirements\n\n### Requirement: Alpha\nAlpha text.\n\n#### Scenario: A works\n- **WHEN** a\n\n### Requirement: Beta\nBeta text.\n`
- **THEN** `detail.sections` holds four entries, and reassembling them — each section's
  heading line rebuilt from its `depth` and `label`, followed by its `text` — reproduces the
  reader's bytes exactly
- **AND** no section's `text` contains a heading line, and no two sections' `text` values
  overlap
- **AND** the reader recorded exactly one call, and a second `sync_detail` on the unchanged
  key records none and re-splits nothing

#### Scenario: The tasks tab seeds its folds once, on the key change

- **WHEN** a `Dashboard` whose selected artifact carries `tracks_tasks == true` and resolves
  to one path is synced with a reader returning
  `## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [ ] b\n`
- **THEN** `detail.expanded` holds exactly `1`
- **AND** a second sync under `refresh.reload`, with the reader now returning the same file
  with `b` checked, leaves `detail.expanded` holding `1`
- **AND** moving `detail.tab` away and back and syncing twice more leaves `detail.expanded`
  empty, because the re-seed found no incomplete subtree
- **AND** the same dashboard whose artifact does **not** carry `tracks_tasks` has an empty
  `detail.expanded` after every one of those syncs

#### Scenario: An unreadable file names its reason and does not lose its siblings

- **WHEN** a `Dashboard` whose selected artifact resolves to two paths is synced with a
  reader that returns `Err("permission denied")` for the first and `Ok("# b\n")` for the
  second
- **THEN** `detail.sections` holds exactly one entry, labelled for the second path, whose
  `text` is `# b\n`
- **AND** `detail.problems` holds exactly one entry, which contains the failing path and the
  text `permission denied`
- **AND** the artifact is **not** foldable, because one section remains, so no header row is
  drawn for the file that did survive
- **AND** a second sync after a tab move and back clears the previous `problems` before
  recording again, so a transient failure does not accumulate
- **AND** the same holds across a forced reload: a `refresh.reload` that re-reads an
  unchanged key clears `detail.problems` first, so a file whose permissions were repaired
  stops being reported the moment the next refresh arrives

#### Scenario: An artifact with no resolved paths reads nothing at all

- **WHEN** a `Dashboard` whose selected artifact has an **empty** `paths` list is synced with
  a recording reader
- **THEN** `detail.sections`, `detail.problems`, and `detail.expanded` are all empty and
  `detail.loaded` is `Some((change dir, tab))`
- **AND** the reader recorded **zero** calls, so "no content yet" costs no filesystem access

#### Scenario: A tab out of range for the newly selected change is clamped before the read

- **WHEN** a `Dashboard` whose selected change carries five artifacts, with `detail.tab: 4`,
  has its `filter.query` set so that `visible()` becomes a single different change carrying
  two artifacts, and is then synced
- **THEN** `detail.tab` is `1` and the reader was called with that change's second artifact's
  path
- **AND** no panic occurs, and `detail.loaded` names the new change's directory

#### Scenario: An empty visible list clears the detail

- **WHEN** a `Dashboard` holding one change is synced, and then its `filter.query` is set to
  a string matching no change and it is synced again
- **THEN** after the second sync `detail.sections`, `detail.problems`, and `detail.expanded`
  are empty, `detail.tab` and `detail.scroll` are `0`, and `detail.loaded` is `None`
- **AND** the same holds for a `Dashboard` built over `changes::empty_set()`, which is synced
  without the reader being called at all
- **AND** a `refresh.reload` set on such a dashboard is still cleared by the sync, so the flag
  cannot survive an empty list and force a spurious re-read on the next frame

#### Scenario: The loop syncs before it draws

- **WHEN** `run_loop` is driven over a `Terminal<TestBackend>` at 120x20 with a dashboard
  whose selected change's first artifact reads as `# proposal\n`, an event script of a single
  `q` press, a recording reader, and an inert live tier
- **THEN** it returns `Ok(LoopSummary { frames: 1, polls: 1 })`
- **AND** the buffer's detail content area holds `# proposal` on its first row, so the very
  first frame carried the content rather than a blank region
- **AND** the reader recorded exactly one call

### Requirement: The content area renders the artifact, its problems, or `No content yet`

`ui::detail::content_lines(detail: &Detail, change: Option<&Change>, width: u16) ->
Vec<ContentRow>` SHALL be the single row list both `ui::view::render` and
`Dashboard::normalise_scroll` derive the detail content from, so the drawn slice and the
clamp can never disagree, where

```rust
pub struct ContentRow {
    pub line: crate::ui::markdown::Line,
    pub kind: ContentKind,
}

pub enum ContentKind {
    Problem,
    Body,
    SectionHeader { section: usize, selected: bool },
}
```

The return type changes from `Vec<markdown::Line>` because a `markdown::Face` cannot express
a section header: `Face` is seven markdown-construct flags, and a header row is not a
markdown construct. `kind` is **plain data carrying no `ratatui` type and no
`palette::Role`**, on exactly the terms `ui::list::RowKind` already is, so `ui::view` alone
decides what a row looks like. `ContentRow` and `ContentKind` are derived view data rather
than state types and SHALL NOT join the `NODEFAULT-UI` type list, on the same terms
`ui::list::RowKind` does not.

`kind` SHALL be `Problem` for a problem row — **both** sources of them, the selected change's
own `Change::problems` and the tab's `detail.problems`, which the requirement below stacks in
that order — `SectionHeader` for a header row, and `Body` for every other row, including every
line of an open section's rendered body, every line of a `None`-labelled preamble section,
every blank separator row, the tracked-tasks progress-bar row and its blank line, and every
line of a non-foldable artifact's body.

`selected` SHALL be true for exactly the one header whose section the cursor is on or in, and
false on every header when the cursor addresses a problem row, a preamble row, or a
progress-bar row, or when there are no sections. Because both problem sources precede every
section and are counted in the same row list, a change that gained a problem between two
frames shifts every section's row index by one, and `detail.scroll` — an index into that same
list — follows the shift rather than the section. That is the same behaviour a problem row
already gives the scroll offset today, and it is corrected within one frame by
`Dashboard::normalise_scroll`.

Each returned row's `line` SHALL be:

- one line per entry of `detail.problems`, each the text `"! <problem>"` passed through
  `ui::list::pad_or_truncate_right` at `width`, followed by
- when `change` is `Some` and the `ArtifactRef` at `detail.tab` carries
  `tracks_tasks == true`, `tasks-progress-bar`'s single line and one blank line beneath it,
  both omitted entirely when the bar renders as the empty string at that width, followed by
- the selected tab's **body**, which is:
  - when the artifact is **foldable** (more than one section), `artifact-folds`' walk over
    `detail.sections`: a header row per visible labelled section, each open section's body
    beneath its own header, and a blank separator row after a non-empty open body that a
    further visible section follows;
  - and otherwise the single section's whole `text` rendered as one body, with no header row
    and no separator;
- where a section's body — and a non-foldable artifact's whole body — is rendered by
  `tasks-checklist`'s items-only grammar when the tracked-tasks condition above holds, and by
  `ui::markdown::lines(&section.text, width)` in every other case: a `None` change, a
  `detail.tab` past the end of the artifact list, and a change carrying no artifacts at all
  among them;
- and, when `detail.problems` is empty **and** `detail.sections` is empty, exactly one line
  reading `No content yet` — the state `SPEC.md`'s degraded-states table names for a missing
  artifact file — **passed through `ui::list::pad_or_truncate_right` at `width`**, on
  exactly the terms every problem row, task item, section header, and progress-bar
  line already is.

The tracked-tasks body is **no longer exempt from folding**. The exemption this requirement
carried — "deliberately not foldable, at any section count" — rested on two claims, and
`heading-sections` retires both: a multi-section tasks tab is no longer unreachable, because
sections now come from the file's own headings rather than from a glob, and the progress bar
no longer sits inside the folded region. It is emitted **above** every header, as leading
body owned by no section, so it reports the change's whole `progress` while the groups
beneath it fold — a bar that counts the change and a fold that hides a group are not in
disagreement, they answer different questions. `artifact-folds` states the reversal in full.

That `No content yet` clause is a repair, not a restatement. `No content yet` was pushed as
a bare `String` while every neighbouring line went through the padding, and this
requirement's own promise — that no returned line exceeds `width` — was therefore false at
every `width` below 14: at a 15-column narrow **frame** in the detail route, whose content
area is 13 columns, the rendered row read `│No content yet` and ate the region's right
border, as the region still had one. The literal is truncated with the same `…` rule as
everything else, so at `width` 13 it reads `No content y…`, at 12 `No content …`, at 11
`No content…`, and at 0 it is the empty string. Every `width` in this paragraph is the
**content area's**, which at the narrow layout is the frame's less the region's two **gutter**
columns — the same arithmetic the two border columns gave before `pane-chrome`, so every
truncation point below is unchanged; the scenario below gives frame widths and says so.
The line SHALL still be a single `Segment` carrying `Face::plain()`.

The `change` argument is the **only** reason the tracked-tasks decision is made once rather
than at each of the two call sites; both callers SHALL pass `Dashboard::selected_change()`
and SHALL NOT decide the grammar themselves.

Both non-foldable bodies SHALL return nothing for an empty section list, so the
`No content yet` rule above is unaffected by which body was selected: an artifact file that
does not exist reads `No content yet` whether or not it is the tracked-tasks artifact, and
never reads `tasks-checklist`'s `No tasks yet`. The tracked-tasks tab's own progress-bar row
SHALL NOT change that: `bar_lines` is emitted only when `detail.sections` is non-empty, so an
artifact file that does not exist still reads `No content yet` alone.

When `detail.problems` is non-empty and `detail.sections` is empty, the problems alone SHALL
be returned and `No content yet` SHALL NOT appear: the reason is known, and reporting both
would say two contradictory things about the same tab.

Every row's `line` SHALL carry plain faces except those `markdown::lines`, `ui::tasks::lines`,
and `ui::tasks::items` produce; a section header's emphasis is carried by its `kind`,
never by a `Face`, and a blank separator row carries `Face::plain()` like every other row this
function adds. `content_lines` SHALL name no `ratatui` type and no `palette::Role`, on the
same terms `ui::markdown`, `ui::list`, and `ui::tasks` do not — which `NOTABSEAM` enforces
over the whole of `src/ui/detail.rs`. `split_headings` and `is_spec_shaped` live in
`src/ui/app.rs` and are swept by `NOIO-VIEW`, `COLWIDTH`, `NOBLOCK`, and `READONLY-UI`
instead.

**Every `line` `content_lines` returns SHALL measure at most `width` display columns**, as
`responsive-layout` defines them, at **every** `width` — not only at the two mandated
interiors. A line that is exactly `width` columns is permitted; one column over is not. This
is stated as a total property rather than as two width cases because the two mandated widths
are both at or above 14 and could not see the `No content yet` overflow at all. A header
row's depth indent is inside that budget, not beside it: at a width below the indent's own
columns the row degrades to truncated indent rather than overflowing.

`ui::view::render` SHALL draw the slice of `content_lines` that starts at
`layout::viewport(rows.len(), detail.scroll, content.height)` when the selected artifact is
foldable and at `layout::scroll_offset(rows.len(), detail.scroll, content.height)`
otherwise, and runs for at most `content.height` rows, into the content area
`layout::split_detail` returns, starting at its first row and first column, never writing
past the interior's last column. The two offsets are `detail-scroll`'s, which states why a
foldable tab follows a cursor and a non-foldable one clamps an offset. The tracked-tasks tab
now reaches the **foldable** arm whenever its file split, which is the whole of what
`heading-sections` changes about this paragraph. `view-palette` states how a row's `kind`
becomes the `Style` its cells carry.

#### Scenario: A missing artifact still shows its tab and reads `No content yet`

- **WHEN** a `Dashboard` whose selected change carries the five tdd artifacts, whose
  `detail.tab` is `1`, whose `detail.sections` and `detail.problems` are empty, is rendered at
  120x20 and at 60x20 at `Route::Detail`
- **THEN** in the 120-column buffer row 5, columns 42 onward, begins `No content yet`
- **AND** in the 60-column buffer row 5, columns 1 onward, begins `No content yet`
- **AND** in both buffers row 2 still holds the full five-tab bar with `2 specs` bold, so a
  missing file removes neither the tab nor the header
- **AND** the same holds with `detail.tab` set to `3`, the tracked-tasks position: a missing
  tasks artifact reads `No content yet` and shows no progress bar
- **AND** in both buffers that row measures exactly the interior width — 78 and 58 — because
  the literal is now padded like every line around it
#### Scenario: `No content yet` does not eat the border at a narrow frame

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
what the literal must not eat is now the region's right gutter column.

- **WHEN** the same section-less `Dashboard` at `Route::Detail` is rendered at 15x20, at
  14x20, at 13x20, at 2x20, and at 1x20
- **THEN** at 15x20 — a frame of 15, so a content area of 13 — the content area's first row
  reads `No content y…` and the frame's right gutter column is a space, not the letter `t`:
  the audit's `│No content yet` row is gone
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
  denied"]` and whose `detail.sections` is a single section holding `# b\n` is rendered at
  120x20 and at 60x20 at `Route::Detail`
- **THEN** in each buffer the content area's first row begins `! /repo/specs/a/spec.md:` and
  its second row begins `# b`
- **AND** `No content yet` appears in neither buffer
- **AND** each problem row is exactly the interior width — 78 and 58 — so it neither
  overwrites a gutter nor leaves a partial cell
- **AND** no header row is drawn, because one surviving section is not foldable
#### Scenario: The rendered markdown fills the content area, not the whole interior

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact,
  **not** marked `tracks_tasks`, resolving to a single section holding twenty lines reading
  `- line-00` through `- line-19` is rendered at 120x20 and at 60x20
- **THEN** in each buffer the header row and the tab row are unchanged and the content
  area's first row holds `- line-00`
- **AND** the content area's last drawn row is `- line-13`, because the **17**-row interior
  gives the content area 14 rows, so the tab bar, the rule, and the padding row took **three**
  rows from the markdown rather than being drawn over it. The change header is no longer one
  of them: it is the region's heading row, outside the interior entirely
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
  artifact, resolving to a single section holding a paragraph of forty repetitions of
  `日本語`, a `# 🎉 見出し` heading, and a bullet holding a family emoji joined by
  zero-width joiners, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer every cell of columns 39 and 41 is a space and every cell
  of column 40 is the divider `│`, unchanged from the same render with an ASCII document —
  the audit's escaped row is gone
- **AND** in the 60-column buffer every cell of column 0 and column 59 is a space
- **AND** in both buffers every drawn content row's own `layout::columns` is at most the
  content area's width, and no cell outside the content area was written
- **AND** the same holds for a **foldable** artifact of three sections whose labels are CJK,
  with one section open, so a header row is measured in columns like every other row
#### Scenario: `content_lines` is total and width-parameterised

- **WHEN** `content_lines` is called at width `78` and at width `58`, and with `change` set
  to `None`, to a change with no artifacts, to a change whose artifact at `detail.tab` is
  marked, and to one whose is not, over: an empty `Detail`; one with problems only; one with
  a single section only; one with both; one whose section is a 200-character paragraph; one
  whose section is a 200-column CJK paragraph; one whose `tab` is past the end of the
  artifact list; one holding three sections with `expanded` empty; and one holding three
  sections with `expanded` holding `0`, `1`, `2`, and `7`; one holding a seven-section spec
  glob whose depths run `0, 1, 2, 3, 2, 0, 0`; and one holding a `None`-labelled preamble
  section followed by two depth-0 task groups
- **THEN** no call panics at either width for any combination
- **AND** the wrapped paragraph produces strictly more lines at `58` than at `78`, so the
  width genuinely reaches both bodies
- **AND** no returned line's `layout::columns` exceeds the width it was called with
- **AND** every `ContentKind::SectionHeader`'s `section` addresses an entry of
  `detail.sections`, at every width, so a hidden subtree never shifts a header's carried
  index off its own section
#### Scenario: No `content_lines` line exceeds its width at any width

- **WHEN** `content_lines` is called at **every** width from `0` through `130`, for each of
  the same eleven `Detail` values and four `change` values above, and additionally for an
  empty `Detail` with a `None` change — the `No content yet` case
- **THEN** no call panics, and at every width every returned line's `layout::columns` is at
  most that width
- **AND** the `No content yet` case is included at widths `0` through `13`, the range in
  which the literal is longer than the region — the range no mandated-width test can reach,
  which is why this property is stated over all widths rather than adding a narrow case to
  the mandated pair
- **AND** the foldable cases are included at widths `0` through `13`, the range in which a
  header row's glyph, its separating space, and its label are together longer than the region
- **AND** the seven-section spec-glob case is included over that same range, where a depth-3
  header's six columns of indent alone exceed the region — the row degrades to truncated
  indent rather than to a dropped glyph, and does not panic
#### Scenario: A foldable tab's body is headers, and an open section's markdown beneath its own

- **WHEN** a `Dashboard` whose selected artifact resolves to three spec files, with
  `detail.expanded` holding `1`, has `content_lines` called at widths 78 and 58
- **THEN** the first line is a header row for section `0` with the collapsed glyph, the
  second is a header row for section `1` with the open glyph, the lines after it are
  `ui::markdown::lines(&sections[1].text, width)` in order, then **one blank row** — that
  body is non-empty and a further visible section follows it — and the line after that is a
  header row for section `2` with the collapsed glyph
- **AND** at both widths no returned line exceeds `width` display columns
- **AND** with `detail.expanded` empty the returned list is exactly three header rows
#### Scenario: A non-foldable tab is byte-identical to today

- **WHEN** a `Dashboard` whose selected artifact resolves to one path holding a twenty-item
  markdown list has `content_lines` called at widths 78 and 58
- **THEN** every returned row's `line` equals the corresponding `ui::markdown::lines(&sections[0].text, width)`
  entry exactly, every `kind` is `Body`, and no header row is prepended
- **AND** rendering it at 120x20 and at 60x20 writes those same lines into the content area
  with no cell reporting `REVERSED` and no row carrying a role
#### Scenario: The tracked-tasks tab concatenates rather than folding

The scenario's name is kept verbatim because a delta's scenario headers are its merge key. It
now pins the **reversal**: the tab folds, and concatenation is gone.

- **WHEN** a `Dashboard` whose selected artifact carries `tracks_tasks == true` and resolves
  to two paths reading `## 1. Setup\n- [ ] a\n` and `## 2. Build\n- [ ] b\n` has
  `content_lines` called at widths 78 and 58
- **THEN** the returned list is the progress-bar row, a blank row, and then four header rows
  with their bodies — `v <first path's label>`, `  v 1. Setup`, `[ ] a`, a blank row,
  `v <second path's label>`, `  v 2. Build`, `[ ] b` — because both files split and both
  subtrees are incomplete, so the seed opened every section
- **AND** with the first file's item **checked** instead, the seed leaves section `0`
  collapsed and the drawn rows are the bar, a blank row, `> <first path's label>`,
  `v <second path's label>`, `  v 2. Build`, and `[ ] b`: `  > 1. Setup` is **not** drawn at
  all, because a collapsed depth-0 file section hides its own group
- **AND** no line of the concatenation `## 1. Setup\n- [x] a\n## 2. Build\n- [ ] b\n`
  appears as a heading line, at either width: the two group headings became labels
- **AND** at both widths no returned line exceeds `width` display columns

#### Scenario: A missing artifact file renders `No content yet` and nothing else

- **WHEN** a `Dashboard` whose selected change carries one artifact resolving to no path at
  all is rendered at 120x20 and at 60x20 in the detail route
- **THEN** the content area's first row reads `No content yet`, padded to the content width
- **AND** no header row, no progress-bar row, and no problem row is drawn
- **AND** at frame widths of 15, 14, 13, and 1 the row reads `No content y…`, `No content …`,
  `No content…`, and the truncation the `…` rule gives at that width, and none panics

#### Scenario: The progress bar leads the folded task groups

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true`, whose change's `progress` is `Progress { completed: 1, total: 3 }`,
  and whose file reads
  `## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [ ] b\n- [ ] c\n` is rendered at 120x20 and at
  60x20
- **THEN** the content area's first row is the progress-bar row and its second is blank
- **AND** its third row reads `> 1. Done` and its fourth `v 2. Doing`, followed by `[ ] b`
  and `[ ] c`
- **AND** the bar reports the change's own `progress`, `1/3`, while group `1. Done` is folded
  — the two are not required to agree, and the scenario asserts the bar is unaffected by the
  fold state by rendering the same dashboard with `detail.expanded` empty and comparing the
  first row cell for cell

#### Scenario: A body row is never indented by its section's depth

- **WHEN** the seven-section spec-glob dashboard is rendered at 120x40 and at 60x40 with
  `detail.expanded` holding every index
- **THEN** the rows of `Requirement: Alpha`'s body — a depth-2 section — begin at column
  zero of the content area, with no leading spaces the source did not carry
- **AND** its header row begins with exactly four spaces
- **AND** every drawn row measures exactly the content area's width in display columns
