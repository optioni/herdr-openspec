## MODIFIED Requirements

### Requirement: The checklist's line grammar

`ui::tasks` SHALL expose the checklist grammar as **three** functions, so that a folded tab
and an unfolded one render the same items through the same code:

- `ui::tasks::bar_lines(progress, groups: &[crate::tasks::Progress], width)` — the
  progress-bar line, `tasks-progress-bar`'s single line as one plain-faced segment, followed
  by one blank line; the **empty vector** when the bar renders as the empty string at that
  width. `groups` is one `Progress` per task group in document order, which
  `tasks-progress-bar` uses to segment the gauge; an **empty slice** SHALL render the bar
  exactly as it rendered before that change, which is what the non-foldable path and the
  detail header both pass;
- `ui::tasks::items(items: &[crate::tasks::Item], width)` — one or more lines per item, in
  order, as specified below, with **no** progress bar, **no** heading line, and **no** blank
  separator. It SHALL take **parsed items**, never a source string: `lines` already holds
  `tasks::Group` values and would have to re-serialise each group to call a string-taking
  form, which is the one thing this extraction exists to avoid. A folded tab reaches it
  through `tasks::parse(&section.text)` on a section body that carries no heading of its own,
  that heading having become the fold header;
- `ui::tasks::lines(source, progress, width)` — the whole-tab grammar, which SHALL be
  `bar_lines(progress, &per_group, width)`, where `per_group` is `group.progress()` for each
  `tasks::Group` `tasks::parse(source)` returned, in document order, followed by, for each
  such group in that same order:
  - when the group carries a `Heading`, one line whose text is that heading's `#` markers
    reproduced from its `level`, a space, and its `text` verbatim, carrying
    `Face { heading: Some(level), .. }` so `ui::view::style_for` bolds it with no new
    `Face`-to-`Style` mapping;
  - `items(&group.items, width)`;
  - one blank line after every group but the last.

`lines` SHALL be reached only for a **non-foldable** tracked-tasks tab — a task file holding
no items, which renders `No tasks yet`, and a task file with items but no heading at all,
which renders as one flat checklist exactly as it did before `heading-sections`. A foldable
one is rendered by `artifact-content`'s walk: `bar_lines` above every header, then
`items` inside each open section. The three functions SHALL NOT each reimplement the
item grammar: `lines` calls `items`, which is what makes "the folded and unfolded tabs
render an item identically" true by construction rather than by two assertions agreeing.

A group heading is therefore rendered **either** as a `Face { heading }` line, when the tab
is not foldable, **or** as `artifact-folds`' fold header row carrying the heading's text as
its label — never both, and never neither. The `#` markers are dropped on the folding path:
a fold glyph and an indent already say what the `##` markers said, and `artifact-folds`'
header grammar has no place for them.

An item's line SHALL be a prefix followed by its text. The prefix is `item.indent` spaces,
then the three-character glyph `[✓]` when `item.checked` and `[ ]` when it is not, then one
space.

An item's rendered **rows** SHALL be faced by exactly one of two rules, chosen by
`item.checked`:

- **A checked item is de-emphasised whole.** Every row it produces — its first row and every
  continuation row of a wrapped item — SHALL be one segment carrying
  `Face { muted: true, ..Face::plain() }`, its prefix and its text alike, and SHALL NOT be
  split at its label. A finished row reads as finished, `VERIFY:` included: leaving a bright
  label on a completed task is the exact complaint this change exists to answer, so the
  label's own role is **dropped** rather than dimmed alongside it.
- **An unchecked item is split at its label, when it has one.** `tasks::label_of(&item.text)`
  decides: on `None` the row is one `Face::plain()` segment exactly as before this change; on
  `Some(Label { start, len, role })` the item's **first** row SHALL carry exactly three
  segments — the prefix concatenated with `item.text[..start]` under `Face::plain()`, then
  `item.text[start..start + len]` under `Face { label: Some(role), ..Face::plain() }`, then
  the remainder of that row's text under `Face::plain()`. A **continuation** row of a wrapped
  item SHALL carry one `Face::plain()` segment: a label appears once, on the row it was
  written on.

A segment SHALL be omitted rather than emitted empty: an item whose text is exactly its label
produces two segments, not three, and one whose label starts at offset `0` with an empty
prefix — unreachable, since the prefix always holds at least the glyph — would produce two.
`Line::text()` SHALL therefore be **byte-identical** to what this capability produced before
this change for every item, checked or not: this change splits rows into segments and faces
them, and moves **no character**.

The label SHALL be looked up against `item.text`, never against the rendered row, so a wrap
that falls inside the label cannot half-style it: when `start + len` exceeds the first row's
own text length, the item SHALL be treated as carrying no label at all and SHALL render as one
`Face::plain()` segment. That case is reachable only at a width narrow enough to split
`CHARACTERIZE:` itself, and degrading it to unlabelled is preferred to emitting a label
segment whose text is `CHARACT`.

`ui::tasks` SHALL call `tasks::label_of` and SHALL NOT reimplement the recognition rule, on
exactly the terms it already calls `tasks::parse` rather than reimplementing the checkbox
rule. A heading line SHALL keep carrying `Face { heading: Some(level), .. }` and SHALL be
neither muted nor labelled.

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
column: `item.indent` spaces first, leaving `[✓] ` alone; and when even that does not fit,
the glyph alone truncated by `ui::list::pad_or_truncate_right` at `width`. No line's text
SHALL exceed `width` **display columns**, as `responsive-layout` defines them — the unit
this change makes uniform across the crate, replacing the `char` count this requirement
carried. The `[✓]`/`[ ]` glyph and `item.indent`'s spaces measure exactly
their character counts, so the drop-whole indent rule above is unchanged; only an item's
own text can differ between the two measures.

`width == 0` SHALL return an empty vector, matching `ui::markdown::lines`.

`ui::tasks` SHALL name no `ratatui` type, on exactly the terms `ui::detail` and
`ui::markdown` do not, and SHALL name no filesystem, process, environment, network, or
standard-I/O API. It SHALL call `tasks::parse` and SHALL NOT reimplement it: this
capability renders an existing parse and introduces no second checkbox rule.

#### Scenario: A folded group and an unfolded one render the same item lines

- **WHEN** `ui::tasks::items` is called at width `78` and at width `58` over
  `tasks::parse("- [x] 1.1 first\n- [ ] 1.2 second\n").groups[0].items`, and
  `ui::tasks::lines` is called at the same two widths over
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n` with
  `Progress { completed: 1, total: 2 }`
- **THEN** `items` returns exactly two lines whose texts are `[✓] 1.1 first` and
  `[ ] 1.2 second`, the first carrying one `muted: true` segment and the second one
  `Face::plain()` segment, with no progress-bar line, no heading line, and no blank line
- **AND** `bar_lines` returns exactly two lines — `tasks-progress-bar`'s own bar and one
  blank — and the **empty vector** at a width where the bar renders as the empty string
- **AND** `lines`' own output at the same width is `bar_lines`' two lines, then the heading
  line `## 1. Setup` carrying `Face { heading: Some(2), .. }`, then those same two item
  texts — asserted against these **literals**, not against `lines`' output, because once
  `lines` calls `items` a comparison between the two could not fail

#### Scenario: A foldable tasks tab draws its groups as fold headers

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true`, whose change's `progress` is `Progress { completed: 1, total: 3 }`,
  and whose one path reads
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n`
  is synced and rendered at 120x20 and at 60x20
- **THEN** the content area holds the progress-bar row, a blank row, `v 1. Setup`,
  `[✓] 1.1 first`, `[ ] 1.2 second`, a blank row, `v 2. Build`, and `[ ] 2.1 third`, in that
  order — both groups open, because both subtrees are incomplete
- **AND** no row reads `## 1. Setup` or `## 2. Build`: the `#` markers are gone with the
  heading lines they belonged to
- **AND** with `detail.expanded` cleared the content area holds the progress-bar row, a blank
  row, `> 1. Setup`, and `> 2. Build`, and no item row at all

#### Scenario: Groups, headings, items, and separators at both mandated widths

- **WHEN** `ui::tasks::lines` is called directly — the non-foldable path — at width `78` and
  at width `58` over the source
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n`
  with `Progress { completed: 1, total: 3 }`
- **THEN** at each width the lines' texts are, in order: the progress bar, an empty line,
  `## 1. Setup`, `[✓] 1.1 first`, `[ ] 1.2 second`, an empty line, `## 2. Build`, and
  `[ ] 2.1 third`
- **AND** exactly one blank line separates the two groups and none follows the last
- **AND** the two heading lines carry `Face { heading: Some(2), .. }`, the `[✓] 1.1 first`
  row carries one segment with `muted: true`, and the two unchecked rows each carry one
  `Face::plain()` segment, no item text here holding a label

#### Scenario: A nested item reproduces its own indent

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `- [ ] parent\n  - [x] child\n    - [ ] grandchild\n` with
  `Progress { completed: 1, total: 3 }`
- **THEN** at each width the three item lines read `[ ] parent`, `  [✓] child`, and
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
- **AND** at `78` and `58` the item line begins with six spaces then `[✓] alpha`
- **AND** at a width where the six-space indent leaves no text column, the item line begins
  `[✓]` at column zero — the indent was dropped whole rather than partially
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
- **THEN** at each width `[✓] loose` appears before `## 1. Later` with no heading line
  above it
- **AND** exactly one blank line separates the two groups

#### Scenario: A labelled unchecked item splits into three segments

- **WHEN** `ui::tasks::items` is called at width `78` and at width `58` over
  `tasks::parse("- [ ] 1.1 RED: write the failing test\n- [ ] Commit: the parser\n").groups[0].items`
- **THEN** the first item's row carries exactly three segments — `[ ] 1.1 ` under
  `Face::plain()`, `RED:` under `Face { label: Some(LabelRole::Evidence), ..Face::plain() }`,
  and ` write the failing test` under `Face::plain()`
- **AND** the second item's row carries exactly **one** `Face::plain()` segment, `Commit:`
  being a one-letter uppercase run and so no label at all
- **AND** at both widths each row's `Line::text()` is byte-identical to what this capability
  returned before this change, so the split moved no character

#### Scenario: A checked item is de-emphasised whole, label included

- **WHEN** `ui::tasks::items` is called at width `78` and at width `58` over
  `tasks::parse("- [x] 1.1 VERIFY: make check is green\n").groups[0].items`
- **THEN** the row carries exactly **one** segment, whose text is the whole row and whose
  face is `Face { muted: true, ..Face::plain() }`
- **AND** that segment's `label` is `None`, so the completed row carries no label role for
  `ui::view` to colour — the de-emphasis is not a dimmed `VERIFY:` but no `VERIFY:` role at
  all
- **AND** the same text with `[ ]` instead of `[x]` returns three segments with
  `label: Some(LabelRole::Confirm)` on the middle one, so the two paths are asserted against
  each other and a rule that muted both, or neither, could not pass

#### Scenario: A wrapped labelled item labels only its first row

- **WHEN** `ui::tasks::items` is called at width `58` over a single unchecked item whose text
  is `1.1 GREEN: ` followed by twenty words of eight characters each, so the item wraps to
  more than one row
- **THEN** the first row carries three segments, the middle one `GREEN:` under
  `Face { label: Some(LabelRole::Change), .. }`
- **AND** every continuation row carries exactly one `Face::plain()` segment, with
  `label: None` and `muted: false`
- **AND** the concatenation of every row's `text()`, with the hanging indent stripped, holds
  the item's whole text, so no character was lost to the split

#### Scenario: A label split across a wrap degrades to unlabelled

- **WHEN** `ui::tasks::items` is called over a single unchecked item whose text is
  `1.1 CHARACTERIZE: record the baseline`, at a width where the first row's text column ends
  inside `CHARACTERIZE:` — width `12`, `14`, and `16`
- **THEN** at each of those widths every row carries exactly one `Face::plain()` segment and
  no segment carries a `label`
- **AND** nothing panics, and each row's `text()` is byte-identical to what this capability
  returned before this change
- **AND** at widths `58` and `78`, where the label fits on the first row whole, the same item
  does split into three segments, so the degradation is width-driven rather than unconditional
