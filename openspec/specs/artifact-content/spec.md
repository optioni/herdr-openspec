# artifact-content Specification

## Purpose
Governs what the detail region's body actually holds: `Dashboard::sync_detail` resolves the
selected `(change directory, tab)` pair into text by reading the artifact's files through a
single injected reader closure — one production binding, `ui::read_artifact`, so no view file
ever names a filesystem API — concatenating multiple paths in resolution order, caching on the
key so an unchanged selection re-reads nothing, and re-reading when a live refresh forces it
without throwing a mid-document reader back to line one. It also fixes what the content area
shows: problem rows for files that failed to read, then either the markdown body or, for the
schema's tracked-tasks artifact, the checklist body, and `No content yet` only when there is
neither content nor a reason. Which tabs exist is `artifact-tabs`', how far the body scrolls is
`detail-scroll`'s, and the two bodies' own grammars belong to `markdown-render` and
`tasks-checklist`.

## Requirements

### Requirement: The artifact read is an injected collaborator, confined to one binding

Reading an artifact file is **not** a view's job. That no view file names a filesystem API is
`dashboard-loop`'s requirement — its pure set grows to **eight** files with `src/ui/tasks.rs`
added, and the patterns it searches for grow by `tasks::read` — and is not restated here.
What **this** requirement adds is where the read does live, and that there is exactly one of
it.

The read SHALL arrive as an injected closure, in the same shape by which `resolve` takes the
`npm prefix -g` probe and `config` takes the process environment:

```rust
pub type ArtifactReader<'a> = &'a dyn Fn(&std::path::Path) -> Result<String, String>;
```

`ui::driver::run_loop` SHALL take one as a parameter and pass it to
`Dashboard::sync_detail`. The crate SHALL hold exactly one production binding,
`ui::read_artifact(path: &Path) -> Result<String, String>` in `src/ui/mod.rs`, which calls
`std::fs::read_to_string` and maps its error to the error's `Display` text. `ui::run` SHALL
be the only caller that passes it.

`read_to_string` SHALL appear under `src/ui/` only in `src/ui/mod.rs`, checked tree-wide with
a positive control asserting that `src/ui/mod.rs` does name it, so an exclusion that protects
nothing fails rather than passing.

`read_artifact` SHALL carry its own assertions rather than being covered only by the
composition that calls it, on the same terms `config::env_lookup` does: it is a one-line
binding to the real world, and a binding nothing asserts is untested residue.

The tracked-tasks tab SHALL add no second read and no second binding. `ui::tasks::lines`
receives the bytes `sync_detail` already placed in `detail.source` and parses them in memory;
`tasks::read`, the filesystem edge of the same module, SHALL be named nowhere under
`src/ui/`.

#### Scenario: The artifact read has exactly one binding under `src/ui/`

- **WHEN** every `*.rs` file under `src/ui/` is searched for `read_to_string`
- **THEN** every match is in `src/ui/mod.rs` and there is at least one
- **AND** the check is proven able to fail: run against a copy carrying a
  `std::fs::read_to_string` call in `src/ui/driver.rs`, it reports it
- **AND** it is proven able to fail the other way too: run against a copy whose
  `src/ui/mod.rs` names no `read_to_string`, it reports the exclusion as vacuous rather than
  reporting a clean tree

#### Scenario: `read_artifact` agrees with the standard library and names its failure

- **WHEN** `read_artifact` is called on a file written into a `ScratchDir` holding the text
  `# proposal\n`, and again on a path inside that directory that does not exist
- **THEN** the first returns `Ok` with a string equal to `std::fs::read_to_string` of the
  same path
- **AND** the second returns `Err` whose text is non-empty, and does not panic

### Requirement: `Dashboard::sync_detail` resolves the selected tab's content

`ui::app::Dashboard::sync_detail(&mut self, read: ArtifactReader)` SHALL, in this order:

0. Take and clear `refresh.reload`, holding the taken value as `force` for the rest of the
   call. Taking it here — rather than reading it later — is what makes one flag produce
   exactly one re-read, on every path out of the function including the empty-list one.
1. When `visible()` is empty, clear `detail.source` and `detail.problems`, set `detail.tab`
   and `detail.scroll` to `0`, set `detail.loaded` to `None`, and return.
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
4. Otherwise re-read: set `detail.source` to the concatenation of `read(path)` over the
   selected artifact's `paths` **in the order `changes::from_files` resolved them**,
   inserting a `\n` between two files when the preceding one does not already end in one, so
   two spec files cannot run together on one line. Every `Err(e)` contributes no text and
   appends the problem `"<path>: <e>"` to `detail.problems`, which is cleared first. Set
   `detail.scroll` to `0` **exactly when the key changed**, and set `detail.loaded` to the
   key.

Resetting the scroll on a key change and not on a forced reload is the whole point of the
split: a tab or change move starts at the top, while an agent's save re-renders the document
under a reader who is halfway down it without throwing them back to line one.
`layout::scroll_offset` and `Dashboard::normalise_scroll` still clamp the preserved offset
against the new content's length, so a document that shrank is corrected within one frame.

`sync_detail` SHALL be called by `ui::driver::run_loop` once per iteration, **before** the
draw and **after** the loop's live tier has adopted any result waiting for it, so the very
first frame shows content rather than a blank region that fills in on the second, and a
corrected change set is read in the same iteration that adopted it. It SHALL NOT be called
by any view.

`sync_detail` SHALL be total: it SHALL NOT panic for any dashboard state, any artifact list,
any `detail.tab`, any value of `refresh.reload`, or any reader behaviour including one that
fails on every path.

#### Scenario: The selected tab's file is read once and reused

- **WHEN** a `Dashboard` whose selected change carries artifacts `proposal` →
  `[/repo/p.md]` and `design` → `[/repo/d.md]`, with `detail.tab: 0` and `refresh.reload`
  false, is given `sync_detail` with a recording reader returning `# proposal` for any path,
  three times in a row
- **THEN** `detail.source` is `# proposal`, `detail.problems` is empty, and
  `detail.loaded` is `Some((change dir, 0))`
- **AND** the reader recorded exactly **one** call, for `/repo/p.md`, so an unchanged
  selection re-reads nothing

#### Scenario: A forced reload re-reads the same key and keeps the scroll

- **WHEN** the same dashboard, already synced once and scrolled to `detail.scroll: 6`, has
  `refresh.reload` set and is synced again with a reader now returning `# proposal (edited)`
- **THEN** the reader recorded a **second** call for `/repo/p.md`
- **AND** `detail.source` is `# proposal (edited)`
- **AND** `detail.scroll` is still `6`, because the key did not change
- **AND** `refresh.reload` is false afterwards, and a third sync with the flag still clear
  records no further call

#### Scenario: A tab move under a forced reload still resets the scroll

- **WHEN** a `Dashboard` scrolled to `6` has both `detail.tab` moved from `0` to `1` and
  `refresh.reload` set, and is then synced
- **THEN** the reader was called with `/repo/d.md` and `detail.scroll` is `0`
- **AND** the reset is attributable to the key change rather than to the flag, because the
  previous scenario proves a forced reload alone preserves the offset

#### Scenario: Switching the tab re-reads, and so does switching the change

- **WHEN** the same dashboard is synced, then given `NextTab`, then synced again, then its
  `selected` is moved to a second change and it is synced a third time
- **THEN** the reader recorded three calls, for `/repo/p.md`, `/repo/d.md`, and the second
  change's first artifact path, in that order
- **AND** `detail.source` holds the second change's text at the end, and `detail.scroll` is
  `0` after each re-read

#### Scenario: Two changes with the same name are distinguished by directory

- **WHEN** a `Dashboard` holding an active change named `add-auth` at
  `/repo/openspec/changes/add-auth` and an archived change also named `add-auth` at
  `/repo/openspec/changes/archive/2026-08-14-add-auth`, each with one artifact resolving to
  a distinct path, is synced with `selected: 0`, then `selected` is set to `1` and it is
  synced again
- **THEN** the reader recorded two calls, one per directory
- **AND** `detail.source` holds the archived change's text after the second sync, so keying
  on the name alone would have shown the wrong artifact

#### Scenario: A multi-file artifact is concatenated in path order with a separating newline

- **WHEN** a `Dashboard` whose selected artifact resolves to `[/repo/specs/a/spec.md,
  /repo/specs/b/spec.md]` is synced with a reader returning `# a` for the first (no trailing
  newline) and `# b\n` for the second
- **THEN** `detail.source` is `# a\n# b\n`
- **AND** with a reader returning `# a\n` for the first instead, `detail.source` is
  `# a\n# b\n` as well, so a file that already ends in a newline gains no second one

#### Scenario: An unreadable file names its reason and does not lose its siblings

- **WHEN** a `Dashboard` whose selected artifact resolves to two paths is synced with a
  reader that returns `Err("permission denied")` for the first and `Ok("# b\n")` for the
  second
- **THEN** `detail.source` is `# b\n`
- **AND** `detail.problems` holds exactly one entry, which contains the failing path and the
  text `permission denied`
- **AND** a second sync after a tab move and back clears the previous `problems` before
  recording again, so a transient failure does not accumulate
- **AND** the same holds across a forced reload: a `refresh.reload` that re-reads an
  unchanged key clears `detail.problems` first, so a file whose permissions were repaired
  stops being reported the moment the next refresh arrives

#### Scenario: An artifact with no resolved paths reads nothing at all

- **WHEN** a `Dashboard` whose selected artifact has an **empty** `paths` list is synced with
  a recording reader
- **THEN** `detail.source` and `detail.problems` are both empty and `detail.loaded` is
  `Some((change dir, tab))`
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
- **THEN** after the second sync `detail.source` and `detail.problems` are empty,
  `detail.tab` and `detail.scroll` are `0`, and `detail.loaded` is `None`
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
  artifact file.

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

`ui::view::render` SHALL draw the slice of `content_lines` that starts at
`layout::scroll_offset(lines.len(), detail.scroll, content.height)` and runs for at most
`content.height` lines, into the content area `layout::split_detail` returns, starting at its
first row and first column, never writing past the interior's last column.

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

#### Scenario: `content_lines` is total and width-parameterised

- **WHEN** `content_lines` is called at width `78` and at width `58`, and with `change` set
  to `None`, to a change with no artifacts, to a change whose artifact at `detail.tab` is
  marked, and to one whose is not, over: an empty `Detail`; one with problems only; one with
  a source only; one with both; one whose source is a 200-character paragraph; and one whose
  `tab` is past the end of the artifact list
- **THEN** no call panics at either width for any combination
- **AND** the wrapped paragraph produces strictly more lines at `58` than at `78`, so the
  width genuinely reaches both bodies
- **AND** no returned line's text exceeds the width it was called with

### Requirement: The content area names the selected change's own problems above the tab's

Four rows of `SPEC.md` → Degraded states — a schema the CLI rejects, a schema that is
unreadable or invalid, a `generates` glob outside the supported subset, and a tasks file that
cannot be read — each end "and the reason is named". The reason is named on
`changes::Change::problems`, and **nothing in the crate reads that vector**: `ui::list` renders
`launch.problems`, `refresh.problems`, and `ChangeSet::problems`, and `ui::detail` renders
`Detail::problems`. A reason nobody can read is not named.

`ui::detail::content_lines` SHALL therefore prepend, above the `Detail::problems` lines it
already emits, one `! `-prefixed line per entry of the **selected change's** own `problems`
vector, in that vector's order, padded or truncated to the width by the same
`ui::list::pad_or_truncate_right` call the existing problem lines use, and carrying the same
plain face. The full content-area order SHALL be: the change's problems, then the tab's
problems, then the body.

The change's problems lead because their lifetimes differ, on exactly the argument
`change-rows` uses for refresh-before-changeset: a `Change` problem is a standing fact about
that change — its schema does not parse, its glob is unsupported — that no tab switch alters,
while a `Detail` problem is about the artifact file the reader is looking at right now and is
replaced by the next `sync_detail`. A standing condition sits above a transient one.

`content_lines` SHALL stay a pure total function of its three arguments, performing no
filesystem, process, environment, network, or terminal I/O, reading no clock and no global
state, and never panicking for any `Detail`, any `Option<&Change>`, and any `u16` width
including `0`.

A `change` of `None` SHALL contribute no lines: with no selected change the whole detail
region is blank, which `detail-view` already requires and this change SHALL NOT weaken.

The count of lines a change contributes SHALL NOT be capped, and the leading lines SHALL be
scrolled with the body rather than pinned: `detail-scroll`'s clamp is computed against the
full line count `content_lines` returns, so a change carrying more problems than the content
area is tall stays reachable with `j`.

#### Scenario: A change whose schema will not parse names the reason in the detail region

- **WHEN** a `Dashboard` whose selected change `alpha` carries
  `problems: ["openspec/schemas/broken/schema.yaml: mapping values are not allowed here"]`,
  an empty `Detail::problems`, and a `source` of `# Proposal` is rendered at the detail route
  into a `TestBackend` at 120x20 and again at 60x20
- **THEN** the first row of the detail region's content area, at both widths, begins
  `! openspec/schemas/broken/schema.yaml: mapping values are not allowed here`, truncated to
  the 78- and 58-column interiors respectively
- **AND** the row below it spells `# Proposal`
- **AND** removing the entry from `problems` returns both buffers to the ones the same
  dashboard produced with `# Proposal` on the first content row, so the line is the change's
  and not a constant

#### Scenario: A change problem and a tab problem are both shown, change first

- **WHEN** the same dashboard carries `problems: ["alpha: unsupported glob **/*.md"]` on the
  change **and** `Detail::problems: ["design.md: Permission denied (os error 13)"]`
- **THEN** the content area's first row names the glob and its second row names the
  permission error, at both widths
- **AND** the body follows on the third row
- **AND** neither line is `No content yet`: the tab-problem rule `detail-view` set — a known
  reason replaces the placeholder rather than joining it — is unchanged by this addition

#### Scenario: No selected change contributes no lines

- **WHEN** `content_lines` is called with `change` `None` and a `Detail` whose `problems` and
  `source` are both empty
- **THEN** it returns an empty vector
- **AND** the rendered detail region is entirely blank at both widths, byte-identical to the
  buffer the same dashboard produced before this change existed

#### Scenario: The line count drives the scroll clamp

- **WHEN** a change carrying twenty problem entries is selected at the detail route, whose
  content area is fourteen rows tall, and `j` is pressed forty times
- **THEN** the stored scroll offset is clamped against the full line count — twenty problem
  lines plus the body's — rather than against the body's alone
- **AND** the last problem entry is reachable on screen, which it would not be were the
  leading lines pinned outside the scrolled region
