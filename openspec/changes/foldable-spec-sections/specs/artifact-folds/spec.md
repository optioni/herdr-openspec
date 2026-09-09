## ADDED Requirements

### Requirement: A multi-file artifact's content is a list of named sections

`ui::app::Detail` SHALL carry the selected tab's content as an ordered list of sections
rather than as one string:

```rust
pub struct Section {
    pub label: String,
    pub text: String,
}
```

with one `Section` per path the selected `ArtifactRef` resolved to, in the order
`changes::from_files` resolved them. `text` is that path's bytes as the injected reader
returned them; a path the reader failed on contributes **no section at all** and its reason
is recorded in `detail.problems`, per `artifact-content`.

`label` SHALL be derived from the path **relative to the change directory**, by exactly one
rule: when the file name is `spec.md` and the relative path has at least one parent
component, the label is the final component of that parent; otherwise the label is the file
name. So `specs/degraded-coverage/spec.md` labels `degraded-coverage`, `specs/notes.md`
labels `notes.md`, and a path that is not under the change directory at all labels with its
own file name. The derivation SHALL be total and SHALL NOT panic for any path, including an
empty one, a relative one, and one whose file name is not valid UTF-8 — a label that cannot
be derived SHALL be the empty string rather than a panic or a skipped section.

Labels SHALL NOT be required to be unique. Sections are addressed by **index** throughout —
in `detail.expanded`, in `Target::DetailSection`, and in every scenario below — so a glob
matching `a/x.md` and `b/x.md` yields two sections both labelled `x.md` that fold
independently.

An artifact SHALL be **foldable** exactly when it has more than one section. That predicate
is derived from `detail.sections.len() > 1` on every use and SHALL NOT be stored as a
field: an artifact whose second file is deleted between two refreshes stops being foldable
in the same frame that adopts the change, with no flag to keep in step.

#### Scenario: The three spec files of a change become three labelled sections

- **WHEN** a `Dashboard` whose selected artifact resolves to
  `[<dir>/specs/degraded-coverage/spec.md, <dir>/specs/markdown-render/spec.md,
  <dir>/specs/tasks-checklist/spec.md]` is synced with a reader returning
  `## MODIFIED Requirements\n` for every path
- **THEN** `detail.sections` holds three entries whose labels are `degraded-coverage`,
  `markdown-render`, and `tasks-checklist`, in that order
- **AND** every entry's `text` is `## MODIFIED Requirements\n`
- **AND** the artifact is foldable, because `detail.sections.len()` is `3`

#### Scenario: A single-file artifact is one section and is not foldable

- **WHEN** a `Dashboard` whose selected artifact resolves to `[<dir>/proposal.md]` is synced
  with a reader returning `# proposal\n`
- **THEN** `detail.sections` holds exactly one entry whose `text` is `# proposal\n`
- **AND** the artifact is not foldable
- **AND** `detail.expanded` is empty, and at 120x20 and at 60x20 the drawn content rows equal
  `ui::markdown::lines(&sections[0].text, width)` cell for cell, with no row prepended and no
  cell reporting `REVERSED` — which is what "unchanged from before this change" reduces to,
  since no pre-change binary is obtainable to compare against

#### Scenario: An artifact with no resolved paths has no sections

- **WHEN** a `Dashboard` whose selected artifact has an empty `paths` list is synced with a
  recording reader
- **THEN** `detail.sections` is empty, the artifact is not foldable, and the reader recorded
  zero calls
- **AND** `content_lines` returns exactly one line reading `No content yet`, per
  `artifact-content`

#### Scenario: An unreadable file drops its section and keeps its siblings

- **WHEN** a `Dashboard` whose selected artifact resolves to three paths is synced with a
  reader returning `Err("permission denied")` for the second and `Ok("# ok\n")` for the
  other two
- **THEN** `detail.sections` holds **two** entries, labelled for the first and third paths
- **AND** `detail.problems` holds exactly one entry containing the failing path and the text
  `permission denied`
- **AND** the artifact is still foldable, because two sections remain

#### Scenario: The label derivation is total over adversarial paths

- **WHEN** the label rule is applied to a change directory of `/repo/openspec/changes/c` and
  the paths `/repo/openspec/changes/c/specs/a/spec.md`,
  `/repo/openspec/changes/c/specs/a/b/spec.md`, `/repo/openspec/changes/c/specs/notes.md`,
  `/repo/openspec/changes/c/spec.md`, `/elsewhere/spec.md`, and the empty path
- **THEN** the labels are `a`, `b`, `notes.md`, `spec.md` — the change directory itself is
  not a parent *component* inside the relative path, so the file name is used — `spec.md`,
  and the empty string, in that order
- **AND** no call panics

### Requirement: Sections start collapsed, and the fold state resets with the tab

`ui::app::Detail` SHALL carry the fold state in a field:

```rust
pub expanded: std::collections::BTreeSet<usize>,
```

holding the indices of the sections that are **open**. A section is collapsed exactly when
its index is **not** in the set, so the empty set means every section is collapsed and no
seeding is needed on any construction path. This inverts `list-selection`'s `Sections
{ collapsed }`, deliberately: the list's sections default open and the set names the
exception, the detail's default closed and the set names the exception, so in each case a
freshly constructed value carries an empty set.

`Dashboard::sync_detail` SHALL clear `expanded` **exactly when the `(change directory, tab)`
key changed** — the same condition that resets `detail.scroll`, and on the same terms. A
forced reload of an unchanged key SHALL leave `expanded` alone, so an agent saving a spec
file the reader has open does not fold it shut underneath them.

`Dashboard::adopt` SHALL NOT touch `expanded`, so a live refresh never folds or unfolds
anything on its own; the reset happens only through `sync_detail`'s key change.

`expanded` SHALL be per-session and SHALL NOT be persisted. The plugin's own writes stay
exactly `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`.

Indices in `expanded` SHALL NOT be required to address an existing section. Rendering and
toggling SHALL both read the set by index and ignore an index at or past
`detail.sections.len()`, so a section list that shrank between two frames degrades to a
closed section rather than a panic.

#### Scenario: The specs tab opens as a list of capability names

- **WHEN** a `Dashboard` whose selected change carries the three spec files above is synced
  and rendered at 120x20 and at 60x20 in the detail route
- **THEN** the content area's first three rows read `> degraded-coverage`,
  `> markdown-render`, and `> tasks-checklist`, each padded to the content width
- **AND** no row carries any text from inside any of the three files
- **AND** `detail.expanded` is empty

#### Scenario: A tab move forgets the fold, a forced reload does not

- **WHEN** a `Dashboard` at the foldable tab with `expanded` holding `1` has `detail.tab`
  moved to another tab and is synced, and then moved back and synced again
- **THEN** `detail.expanded` is empty after the return, so the tab reopened collapsed
- **AND** a second `Dashboard` in the same starting state given `refresh.reload` and synced
  **without** a tab move still has `expanded` holding `1`, and its `detail.scroll` is
  unchanged as well
- **AND** a third `Dashboard` in the same starting state that adopts a
  `RefreshResult::Files` and then a `RefreshResult::Merged` still has `expanded` holding `1`

#### Scenario: An index past the end folds shut rather than panicking

- **WHEN** a `Dashboard` whose `detail.expanded` holds `0`, `1`, and `7` is rendered against
  a `detail.sections` of length `2`, at 120x20 and at 60x20
- **THEN** two header rows are drawn, both open, and nothing panics
- **AND** `content_lines` returns no line attributable to a section index `7`

### Requirement: A section header row names the file and shows its fold state

`ui::detail::content_lines` SHALL, when the selected artifact is foldable, emit for each
section in order:

- one **header row** reading `<glyph> <label>`, where `<glyph>` is `>` when the section is
  collapsed and `v` when it is open — the same two glyphs `list-selection` already uses for
  the `active` and `archived` headers, so one fold reads the same in both regions — passed
  through `ui::list::pad_or_truncate_right` at `width`; followed by
- when and only when the section is open, that section's body: `ui::markdown::lines(&section.text, width)`.

When the selected artifact is **not** foldable, `content_lines` SHALL emit no header row at
all and SHALL render the single section's `text` exactly as it renders `detail.source`
today.

Each header row SHALL carry `ContentKind::SectionHeader { section, selected }`, where
`section` is its own index and `selected` is true for exactly the header whose section the
cursor is on or in. `artifact-content` states that shape and `view-palette` states how
`ui::view` turns it into a `Style`: `Role::DetailSectionSelected` for the selected header,
`Role::DetailSection` for every other. `ui::detail` SHALL name neither role and no `ratatui`
type — the header carries a *kind*, not a style, on exactly the terms `ui::list::RowKind`
already does.

No other row SHALL be restyled by the cursor: the cursor's feedback is which header is
emphasised, not a highlighted line running through prose. When the cursor addresses a problem
row — every problem row precedes every section — no header SHALL be `selected`.

A test asserting the **unselected** style SHALL assert the row's `kind`, not only its painted
cells. `Role::DetailSection` is plain `BOLD`, which `Role::Strong` and five other roles also
are, so a cell comparison alone would pass for a `**bold**` body span and could not fail if
the header lost its role entirely. The **selected** style is `BOLD | REVERSED`, which
`view-palette` requires to equal no other role's, so a cell comparison there is
discriminating and SHALL be asserted that way.

Every header row SHALL measure at most `width` display columns at **every** width, measured
through `ui::layout::columns`, on exactly the terms `artifact-content` states for every
other line `content_lines` returns. A label too long for the width SHALL be truncated with
the same `…` rule every other row uses, and the glyph and its separating space SHALL be
emitted before the label so that the fold state survives any truncation.

#### Scenario: Folding one section shows its body and leaves its siblings shut

- **WHEN** the three-spec dashboard has `detail.expanded` set to hold `1` and is rendered at
  120x20 and at 60x20
- **THEN** the content area's first row reads `> degraded-coverage`, its second reads
  `v markdown-render`, and the rows below the second carry that file's rendered markdown
- **AND** the row following that file's last rendered line reads `> tasks-checklist`
- **AND** at both widths every drawn row measures exactly the content area's width in
  display columns

#### Scenario: The cursor's section header is the emphasised one

- **WHEN** the three-spec dashboard with every section collapsed and `detail.scroll` of `1`
  is rendered at 120x20 and at 60x20
- **THEN** the second header row's cells carry `palette::style(Role::DetailSectionSelected)`,
  which is the discriminating half, and `content_lines` reports `selected: true` on that row's
  `kind` and `false` on the first and third — the half that can fail when the role is lost
- **AND** with `detail.expanded` holding `1` and `detail.scroll` moved to a line inside that
  open section's body, the second header row still carries the selected style, because the
  cursor is *in* that section
- **AND** with a `detail.problems` of one entry and `detail.scroll` of `0` — addressing the
  problem row — no header row carries the selected style

#### Scenario: A narrow pane truncates the label and keeps the glyph

- **WHEN** the three-spec dashboard is rendered in the detail route at frame widths of 20
  and 15 — both below the 100-column breakpoint, with content areas of 18 and 13 columns
- **THEN** the first header row reads `> degraded-covera…` at 18 columns and
  `> degraded-c…` at 13
- **AND** at every width from 0 through 20 no returned line exceeds that width in display
  columns, and none panics
- **AND** a section whose label is CJK is truncated in **columns**, so its header measures at
  most the content width even though its `chars().count()` is smaller

### Requirement: A section header row resolves to its own section index

`ui::detail::section_at(rows: &[ContentRow], offset: usize, content: Rect, row: u16) ->
Option<usize>` SHALL return the `section` of the `ContentKind::SectionHeader` drawn at `row`
of the content area `layout::split_detail` returns, and `None` for every other row — a
problem row, a row inside an open section's body, a row past the last drawn row, and every
row when the artifact is not foldable.

It SHALL be a **lookup**, not a second derivation: it indexes `rows` at `offset + row` and
reads that row's `kind`. Taking the already-computed row list and the already-computed
offset as arguments is what makes "a click and the pixels can never disagree" true by
construction rather than by two computations agreeing — the caller passes the very list and
offset the draw used.

It SHALL be pure and total: every `Detail`, every `Rect` including a zero-width and
zero-height one, and every `row` including `u16::MAX` returns without panicking. It SHALL
name no `ratatui` widget and no mouse type, on the terms `ui::list::row_at` already meets.

#### Scenario: Each drawn header row resolves to its own index

- **WHEN** the three-spec dashboard with every section collapsed is drawn at 120x40, and
  `section_at` is called with that frame's own row list and offset for content rows `0`, `1`,
  `2`, and `3`
- **THEN** it returns `Some(0)`, `Some(1)`, `Some(2)`, and `None`
- **AND** with `detail.expanded` holding `0`, the row carrying the first line of that
  section's body returns `None`, and the row carrying the second header returns `Some(1)`

#### Scenario: Resolution is total and inert where it should be

- **WHEN** `section_at` is called against a non-foldable dashboard's row list, against an
  empty row list, against a zero-width content area, against a zero-height one, with an
  `offset` past the end of the list, and with `row` of `u16::MAX`
- **THEN** every call returns `None` and none panics
- **AND** against a dashboard whose `detail.problems` holds two entries, content rows `0`
  and `1` return `None` and row `2` returns `Some(0)`

### Requirement: `Space` toggles the artifact section the cursor is on or in

`Dashboard::apply(Action::ToggleSection)` at `Route::Detail` SHALL fold or unfold exactly one
artifact section: the one the detail cursor is **on or in**. `list-selection` states the
route split that sends the action here rather than to the list region, and states the list
route's own arm.

The section the cursor is in SHALL be derived from `ui::detail::content_lines`' own line
indices: section `i` owns its header row and, when open, every body line between that header
and the next header, or to the end of the list for the last section. Every problem row
precedes every section and belongs to none. So:

- when `detail.scroll` addresses a line owned by section `i`, `i` is toggled — its index
  removed from `detail.expanded` if present, inserted otherwise;
- when `detail.scroll` addresses a problem row, when the selected artifact is not foldable,
  or when `detail.sections` is empty, `apply` SHALL change nothing at all and SHALL record no
  problem.

After a toggle, `detail.scroll` SHALL be set to the **header row index** of the section that
was toggled, recomputed against the line list the fold just produced. Collapsing a section
the cursor was inside would otherwise leave the cursor addressing lines that no longer exist,
and the per-frame clamp alone would land it somewhere unrelated; moving it to the header is
both the predictable answer and the position from which the next `Space` reopens the section.
This is `list-selection`'s rule for the list region, applied to the same key in the other
one.

`apply` SHALL reach no collaborator, spawn no process, touch no filesystem, and read no clock
while handling `ToggleSection` at either route. In particular it SHALL NOT re-read any
artifact: a fold changes which lines are rendered from text already in `detail.sections`, and
`sync_detail` is not involved.

Opening a section SHALL NOT set `refresh.requested`. Unlike the archived list section, whose
rows may not be resolved yet, every section's `text` was read when the tab was, so a fold
needs no data.

#### Scenario: `Space` opens the section under the cursor and leaves its siblings shut

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to three spec
  files, with `detail.expanded` empty and `detail.scroll` `1`, is given a `ToggleSection`
  action
- **THEN** `detail.expanded` holds exactly `1`, and `detail.scroll` is still `1` — the
  header row index of the section that opened, which did not move because the sections above
  it did not change height
- **AND** rendering at 120x40 and at 60x40 shows `> degraded-coverage`, then
  `v markdown-render`, then that file's rendered markdown, then `> tasks-checklist`
- **AND** `sections.collapsed`, `selected`, `refresh.requested`, and every other field of the
  `Dashboard` are unchanged

#### Scenario: `Space` inside an open section folds it and moves the cursor to its header

- **WHEN** the same dashboard with `detail.expanded` holding `0` and `detail.scroll` set to a
  line inside that open section's body is given a `ToggleSection` action
- **THEN** `detail.expanded` is empty and `detail.scroll` is `0`, the folded section's header
  row
- **AND** rendering at 120x40 and at 60x40 shows exactly three header rows, all collapsed
- **AND** with `detail.expanded` holding `0` and `detail.scroll` addressing the **third**
  header row — whose index depends on how many lines the open first section contributed — the
  same action opens the third section and leaves the first open, so the section acted on is
  the one the cursor is in and not a fixed one

#### Scenario: `Space` on a problem row is inert

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to three spec
  files, one of which the reader failed on, and whose `detail.scroll` is `0` — the problem
  row — is given ten `ToggleSection` actions
- **THEN** the `Dashboard` is equal, field for field, to what it was before the ten
- **AND** none panics and no further problem is recorded
- **AND** moving `detail.scroll` to `1` and repeating the action toggles the first section,
  so the inertness was attributable to the row and not to the presence of a problem

#### Scenario: `Space` is inert on a non-foldable artifact

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to one path is
  given ten `ToggleSection` actions, and a second whose artifact resolves to none is given
  ten more
- **THEN** both `Dashboard` values are equal, field for field, to what they were before
- **AND** neither spawns a process, touches the filesystem, nor calls the artifact reader

#### Scenario: A fold reads no file

- **WHEN** `run_loop` is driven over a `TestBackend` at 120x40 with a recording reader, a
  dashboard whose selected artifact resolves to three spec files, and an event script of
  `Enter`, four `Char(' ')` presses, and `Char('q')`
- **THEN** the reader recorded exactly **one** call per resolved path, all of them during the
  first sync, and none during the four folds
- **AND** the run returns `Ok(..)` and the final buffer shows the fold state the four presses
  produced
