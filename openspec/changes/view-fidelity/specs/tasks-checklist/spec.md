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

### Requirement: The checklist's line grammar

For a tracked-tasks tab whose source is `source` and whose change carries `progress`,
`ui::tasks::lines(source, progress, width)` SHALL produce, in order:

1. the progress-bar line — `tasks-progress-bar`'s single line, as one plain-faced segment
   — followed by one blank line, both omitted entirely when the bar renders as the empty
   string at that width;
2. for each `tasks::Group` returned by `tasks::parse(source)`, in document order:
   - when the group carries a `Heading`, one line whose text is that heading's `#` markers
     reproduced from its `level`, a space, and its `text` verbatim, carrying
     `Face { heading: Some(level), .. }` so `ui::view::style_for` bolds it with no new
     `Face`-to-`Style` mapping;
   - one or more lines per `tasks::Item`, as specified below;
   - one blank line after every group but the last.

An item's line SHALL be a prefix followed by its text. The prefix is `item.indent` spaces,
then the three-character glyph `[x]` when `item.checked` and `[ ]` when it is not, then one
space. Every line SHALL carry `Face::plain()` except a heading line.

`item.indent` is `task-parsing`'s own count of the whitespace **characters** preceding the
bullet, not a column count, and this capability reproduces it as that many spaces without
reinterpreting it. The consequence, stated so it is a decision rather than a surprise: a
tab-indented item renders with **one** space of indent, because a tab is one character. That
matches the parse rather than second-guessing it, and re-deriving a column width here would
be a second indentation rule beside the one `task-parsing` already publishes.

The item's text SHALL be word-wrapped to `width - prefix_len` columns, with continuation
lines indented by `prefix_len` spaces so they align under the first line's text — a hanging
indent, the same shape `markdown-render` gives a list item. A single word longer than the
available text column SHALL be hard-split at that column rather than overflowing the
interior or being dropped, so a long path never silently loses its tail.

The indent SHALL be **dropped whole** when the prefix would not leave at least one text
column: `item.indent` spaces first, leaving `[x] ` alone; and when even that does not fit,
the glyph alone truncated by `ui::list::pad_or_truncate_right` at `width`. No line's text
SHALL exceed `width` characters, counted in `char`s.

`width == 0` SHALL return an empty vector, matching `ui::markdown::lines`.

`ui::tasks` SHALL name no `ratatui` type, on exactly the terms `ui::detail` and
`ui::markdown` do not, and SHALL name no filesystem, process, environment, network, or
standard-I/O API. It SHALL call `tasks::parse` and SHALL NOT reimplement it: this
capability renders an existing parse and introduces no second checkbox rule.

#### Scenario: Groups, headings, items, and separators at both mandated widths

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over the source
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n`
  with `Progress { completed: 1, total: 3 }`
- **THEN** at each width the lines' texts are, in order: the progress bar, an empty line,
  `## 1. Setup`, `[x] 1.1 first`, `[ ] 1.2 second`, an empty line, `## 2. Build`, and
  `[ ] 2.1 third`
- **AND** exactly one blank line separates the two groups and none follows the last
- **AND** the two heading lines carry `Face { heading: Some(2), .. }` and every other line
  carries `Face::plain()`

#### Scenario: A nested item reproduces its own indent

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `- [ ] parent\n  - [x] child\n    - [ ] grandchild\n` with
  `Progress { completed: 1, total: 3 }`
- **THEN** at each width the three item lines read `[ ] parent`, `  [x] child`, and
  `    [ ] grandchild`, so the source's own two- and four-space indents are reproduced
- **AND** the list is flat: no line is dropped, merged, or re-ordered, because
  `tasks::parse` never nests

#### Scenario: A long item wraps with a hanging indent at both widths

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over a single
  unchecked item whose text is 200 characters of space-separated words, with
  `Progress { completed: 0, total: 1 }`
- **THEN** at each width no line's text exceeds that width
- **AND** the first item line begins `[ ] ` and every continuation line begins with exactly
  four spaces, aligning under the first line's text
- **AND** the 58-column call produces strictly more lines than the 78-column call, so the
  width genuinely reaches the wrap

#### Scenario: An unbreakable word is hard-split rather than lost

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over a single
  checked item whose text is one 300-character run with no spaces
- **THEN** at each width the concatenation of the item's lines, with the glyph prefix and
  the hanging indent removed, reproduces the 300 characters exactly
- **AND** no line's text exceeds the width it was called with

#### Scenario: The indent is dropped whole as the width collapses

- **WHEN** `ui::tasks::lines` is called at widths `78`, `58`, `12`, `6`, `5`, `4`, `3`,
  `2`, `1`, and `0` over the source `      - [x] alpha\n` (an indent of six) with
  `Progress { completed: 1, total: 1 }`
- **THEN** no call panics and no returned line's text exceeds its width
- **AND** at `78` and `58` the item line begins with six spaces then `[x] alpha`
- **AND** at a width where the six-space indent leaves no text column, the item line begins
  `[x]` at column zero — the indent was dropped whole rather than partially
- **AND** at `0` the returned vector is empty

#### Scenario: A heading with no items still renders its heading

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `## 1. Empty\n\nsome prose\n\n## 2. Full\n\n- [ ] only\n` with
  `Progress { completed: 0, total: 1 }`
- **THEN** at each width both headings appear, `## 1. Empty` carries no item line beneath
  it, and `## 2. Full` carries `[ ] only`
- **AND** the prose line does not appear, because `tasks::parse` discards it

#### Scenario: A headingless leading group renders without a heading line

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `- [x] loose\n\n## 1. Later\n\n- [ ] grouped\n` with
  `Progress { completed: 1, total: 2 }`
- **THEN** at each width `[x] loose` appears before `## 1. Later` with no heading line
  above it
- **AND** exactly one blank line separates the two groups
