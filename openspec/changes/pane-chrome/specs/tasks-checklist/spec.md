## MODIFIED Requirements

### Requirement: The tracked-tasks tab renders a checklist, and every other tab does not

`ui::detail::content_lines` SHALL render the selected tab as a checklist — the grammar this
capability specifies — when, and only when, the selected change's `ArtifactRef` at
`detail.tab` carries `tracks_tasks == true`. Every other tab SHALL keep
`ui::markdown::lines` unchanged.

The decision SHALL be made in exactly one place, from `Change::artifacts[detail.tab]`, so
the drawn slice (`ui::view::render`) and the scroll clamp (`Dashboard::normalise_scroll`)
can never disagree about which grammar the tab holds. The decision SHALL NOT be made by
comparing the artifact's `id` to the string `tasks`, and SHALL NOT be made by inspecting a
resolved path's filename: `change-artifacts` and `cli-changes` set `tracks_tasks` from the
schema's `apply.tracks`-then-id-`tasks` rule, and a schema declaring `id: checklist` with
`generates: tasks.md` is the case an id comparison gets wrong.

When no artifact of the selected change carries `tracks_tasks == true` — a schema whose
`apply.tracks` matches nothing and which declares no artifact with id `tasks`, a schema
that failed to load, or a change with no artifacts at all — **no tab** renders the
checklist and every tab renders as markdown. No tab is added to, removed from, reordered
in, or hidden from the tab bar by this capability: `artifact-tabs` owns the bar and this
change adds no code to it.

#### Scenario: The tasks tab shows checkboxes and its siblings show markdown

- **WHEN** a `Dashboard` at `Route::Detail`, whose selected change carries the five `tdd`
  artifacts with `tracks_tasks` set at position 3 (`tasks`) and whose `detail.source` is
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n`, is rendered at 120x20 and at 60x20
  with `detail.tab == 3`
- **THEN** in each buffer the content area holds a progress-bar row, a blank row,
  `## 1. Setup`, `[x] 1.1 first`, and `[ ] 1.2 second`, in that order
- **AND** the same `Dashboard` with `detail.tab == 0` renders the identical source through
  `markdown::lines` instead: the content area's first row reads `## 1. Setup` with no
  progress-bar row above it, and the item rows read `- [x] 1.1 first` and
  `- [ ] 1.2 second` with their source bullets intact
- **AND** the tab bar in row 2 is byte-identical between the two renders, so switching tabs
  changed content and nothing else

#### Scenario: The tab is chosen by `tracks_tasks`, not by its id

- **WHEN** a `Dashboard` whose selected change carries two artifacts, `checklist` at
  position 0 with `tracks_tasks == true` and `tasks` at position 1 with
  `tracks_tasks == false`, and whose `detail.source` is `- [x] done\n`, is rendered at
  120x20 and at 60x20
- **THEN** with `detail.tab == 0` the content area holds the checklist grammar — a
  progress-bar row and a `[x] done` row
- **AND** with `detail.tab == 1` the content area holds `- [x] done` and no progress-bar
  row, even though that artifact's id is `tasks`
- **AND** neither render panics and the tab bar shows both cells in both cases

#### Scenario: A schema naming no tasks artifact leaves every tab as markdown

- **WHEN** a `Dashboard` whose selected change carries three artifacts, none with
  `tracks_tasks == true`, and whose `detail.source` is `- [ ] a\n`, is rendered at 120x20
  and at 60x20 with `detail.tab` at each of `0`, `1`, and `2`
- **THEN** no render shows a progress-bar row and every render shows `- [ ] a` verbatim
- **AND** the tab bar still holds all three cells in every render, so no tab was removed

#### Scenario: A `detail.tab` past the end of the artifact list renders no checklist

- **WHEN** `content_lines` is called at width `78` and at width `58` for a `Detail` whose
  `tab` is `7` over a change carrying two artifacts, the second of which has
  `tracks_tasks == true`, with a source of `- [x] a\n`
- **THEN** neither call panics and both return the markdown rendering, because index `7`
  names no artifact and therefore names no tracked-tasks artifact
- **AND** the same holds for a change carrying no artifacts at all

### Requirement: A source holding no task lines renders `No tasks yet`

When `tasks::parse(source)` yields no **items** — no group holds one, so
`Tasks::progress().total == 0` — the checklist SHALL render the progress-bar line, its blank
line, and exactly one further line reading `No tasks yet`, and SHALL render **no** heading
line even where the source carries headings.

That `No tasks yet` line SHALL be passed through `ui::list::pad_or_truncate_right` at
`width`, on exactly the terms every task item, heading line, and problem row already is. It
was pushed as a bare `String` while every neighbouring line went through the padding, so at
any `width` below 12 it overran the region: at a 13-column narrow **frame** in the detail
route, whose content area is 11 columns, the rendered row read `│No tasks yet` and ate the
region's right border, as the region still had one. The literal is now truncated with the
same `…` rule as everything
else — at `width` 11 it reads `No tasks y…`, at 0 it is the empty string — and measures
exactly `width` columns in the fitting case, `width` 12 included, where the twelve-column
literal fits whole and is padded by nothing. Every `width` here is the **content area's**,
the frame's less the region's two **gutter** columns — the same arithmetic the two border
columns gave before `pane-chrome`, so every truncation point here is unchanged.
The line SHALL remain a single `Segment` carrying `Face::plain()`.

The trigger is items, not groups, and that distinction is load-bearing rather than pedantic:
the landed `task-groups` capability requires `parse` to emit a group for **every** heading it
recognises, including one holding no items, so a prose file opening with `# Plan` returns one
group and zero items. A trigger stated over groups would never fire for exactly the documents
this state exists for. It does not conflict with "A heading with no items still renders its
heading" above, which governs a document that holds items *somewhere*: there, the empty
group is one group among several and its heading carries information about a section not yet
written; here, there is nothing to be a section of.

When `detail.source` is **empty**, `artifact-content`'s existing rule governs and
`No content yet` SHALL be rendered instead: an artifact file that does not exist is a
different state from one that exists and holds no tasks, and the two SHALL NOT be
conflated. `No tasks yet` and `No content yet` SHALL never both appear for the same tab.

#### Scenario: A prose-only tasks file reads `No tasks yet`

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change's tracked-tasks tab is
  selected, whose `detail.source` is `# Plan\n\nNothing checkable here.\n`, and whose
  change carries `Progress { completed: 0, total: 0 }`, is rendered at 120x20 and at 60x20
- **THEN** in each buffer the content area's first row holds `[-]` and its third row reads
  `No tasks yet`
- **AND** `No content yet` appears in neither buffer
- **AND** `# Plan` appears in neither buffer, even though `tasks::parse` returned one group
  for that heading: the trigger is zero items, and a document with no items renders no
  heading lines
- **AND** in each buffer that third row measures exactly the interior width — 78 and 58 —
  because the literal is now padded like every line around it

#### Scenario: `No tasks yet` does not eat the border at a narrow frame

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
what the literal must not eat is now the region's right gutter column.

- **WHEN** the same prose-only `Dashboard` is rendered at 15x20, at 14x20, at 13x20, at
  2x20, and at 1x20
- **THEN** at 13x20 — a frame of 13, so a content area of 11 — the content area's third row
  reads `No tasks y…` and the frame's right gutter column is a space, not the
  letter `t`: the audit's `│No tasks yet` row is gone
- **AND** at 14x20 the content area is 12 columns and the row reads `No tasks yet` **whole**,
  with no ellipsis, because the literal is exactly twelve columns — the boundary at which
  truncation begins, sampled on both sides
- **AND** at 15x20 the row reads `No tasks yet` followed by one padding space, so the padded
  arm is exercised too
- **AND** at every one of the five widths the row's `layout::columns` is at most the content
  area's width, no buffer writes a cell past its last column, and none of the five renders
  panics
- **AND** at 1x20 and 2x20 the interior is one or zero columns wide and nothing is drawn in
  it, exactly as `detail-scroll`'s degenerate-width scenarios already require

#### Scenario: A missing tasks artifact still reads `No content yet`

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change's tracked-tasks tab is
  selected, whose `progress` is `Progress { completed: 4, total: 9 }` — non-zero, because
  `change-artifacts`' `tasks.md` fallback counted a file the marked artifact's `generates`
  did not resolve to — and whose `detail.source` and `detail.problems` are both empty, is
  rendered at 120x20 and at 60x20
- **THEN** in each buffer the content area's first row reads `No content yet`
- **AND** no progress-bar row and no `No tasks yet` row appears, even though the change's
  `progress` is non-zero and the detail header one row above still shows `[4/9]`
- **AND** this is the one place the tab's content and the change's progress legitimately
  disagree; `SPEC.md`'s degraded-states table carries a row for it

#### Scenario: A read failure on the tasks tab names its reason and renders no checklist

- **WHEN** the same `Dashboard` has `detail.problems ==
  ["/repo/openspec/changes/x/tasks.md: permission denied"]` and an empty `detail.source`
  and is rendered at 120x20 and at 60x20
- **THEN** in each buffer the content area's first row begins
  `! /repo/openspec/changes/x/tasks.md:` and is exactly the interior width — 78 and 58
- **AND** neither `No content yet` nor `No tasks yet` nor a progress-bar row appears, so
  the tab says one thing about itself rather than two

