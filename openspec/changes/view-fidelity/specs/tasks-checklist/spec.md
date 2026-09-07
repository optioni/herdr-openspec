## ADDED Requirements

### Requirement: The checklist's lines are measured in display columns at every width

Every line `ui::tasks::lines` produces — the progress-bar line, the blank separators, each
heading line, each task item line and its wrapped continuations, and the `No tasks yet`
line — SHALL measure at most `width` **display columns** as `responsive-layout` defines
them, at **every** `width`, not only at the two mandated interiors of 78 and 58.

`ui::tasks` SHALL reach that measure only through `layout::columns` and
`layout::truncate_columns`, and SHALL continue to name no `ratatui` type. The item wrap SHALL
break at a **grapheme-cluster boundary**, so a task item holding a wide character, an emoji,
or a combining mark can never produce a line wider than the region and never ends in half a
cluster. The `[x]`/`[ ]` glyph, its separating space, and the hanging indent under it are
ASCII and measure exactly their character counts; the indent a continuation carries SHALL be
as many spaces as the glyph and its space measure in columns.

A task item holding a single grapheme cluster wider than the columns available to it SHALL
drop that cluster rather than emit a line wider than the region, on exactly
`markdown-render`'s rule for the same case.

#### Scenario: A checklist of wide-character items fits at both mandated widths

- **WHEN** `ui::tasks::lines` is called at widths 78 and 58 with a source holding one heading
  `## 日本語の見出し` and five items, among them `- [x] 日本語のタスク` repeated to 200
  display columns, `- [ ] 🎉 celebrate`, and an item holding a family emoji joined by two
  zero-width joiners, against `Progress { completed: 1, total: 5 }`
- **THEN** at each width every returned line's `layout::columns` is at most that width
- **AND** at least one wrapped continuation line measures the width or one less, so the wrap
  fills the region rather than stopping early
- **AND** slicing every drawn item line back out of the source at its own byte offsets
  succeeds, so no line ends in half a cluster
- **AND** the `[x]` and `[ ]` glyphs still begin at the same interior columns they do for an
  ASCII checklist, because the glyph budget did not move

#### Scenario: No checklist line exceeds its width at any width

- **WHEN** `ui::tasks::lines` is called at **every** width from `0` through `130` against
  each of: the wide-character source above; a twenty-item ASCII checklist; a source holding
  one heading and no items; the empty string; and a source holding a NUL character — each
  against `Progress { completed: 4, total: 9 }` and again against
  `Progress { completed: 0, total: 0 }`
- **THEN** no call panics at any width for any combination
- **AND** at every width every returned line's `layout::columns` is at most that width
- **AND** the `Progress { completed: 0, total: 0 }` runs include widths `0` through `12`,
  the range in which `No tasks yet` is longer than the region

## MODIFIED Requirements

### Requirement: A source holding no task lines renders `No tasks yet`

When `tasks::parse(source)` yields no **items** — no group holds one, so
`Tasks::progress().total == 0` — the checklist SHALL render the progress-bar line, its blank
line, and exactly one further line reading `No tasks yet`, and SHALL render **no** heading
line even where the source carries headings.

That `No tasks yet` line SHALL be passed through `ui::list::pad_or_truncate_right` at
`width`, on exactly the terms every task item, heading line, and problem row already is. It
was pushed as a bare `String` while every neighbouring line went through the padding, so at
any width below 13 it overran the region: at a 13-column narrow frame in the detail route the
rendered row read `│No tasks yet` and ate the region's right border. The literal is now
truncated with the same `…` rule as everything else — at width 12 it reads `No tasks ye…`,
at width 0 it is the empty string — and measures exactly `width` columns in the fitting case.
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

- **WHEN** the same prose-only `Dashboard` is rendered at 15x20, at 14x20, at 13x20, at
  2x20, and at 1x20
- **THEN** at 13x20 the content area's third row reads `No tasks y…` or shorter and the
  frame's right border column is a box-drawing character, not the letter `t` — the audit's
  `│No tasks yet` row is gone
- **AND** at every one of the five widths the row's `layout::columns` is at most the interior
  width, no buffer writes a cell past its last column, and none of the five renders panics
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
