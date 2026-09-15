## MODIFIED Requirements

### Requirement: The checklist's line grammar

`ui::tasks` SHALL expose the checklist grammar as **three** functions, so that a folded tab
and an unfolded one render the same items through the same code:

- `ui::tasks::bar_lines(progress, groups: &[crate::tasks::Progress], width)` — the
  progress-bar line, `tasks-progress-bar`'s single line as one plain-faced segment, followed
  by one blank line; the **empty vector** when the bar renders as the empty string at that
  width. `groups` is one `Progress` per task group in document order, which
  `tasks-progress-bar` uses to segment the gauge; an **empty slice** SHALL render the bar
  exactly as it rendered before that change.

  **Both callers SHALL pass a populated slice, and which one they are decides how they build
  it.** `lines` — the non-foldable path — derives it from its own `tasks::parse`. The
  **foldable** path is `artifact-content`'s walk in `ui::detail::content_lines`, and it is the
  path every real `tasks.md` takes; it SHALL pass the `progress` values `artifact-folds` now
  stores on `detail.sections`, in section order, skipping the sections carrying `None`. It
  SHALL NOT re-parse the file to build the slice: the number is already computed once per
  sync, and a second derivation is a second number that can disagree with the header cells
  drawn beside it.

  The consequence of skipping `None` is stated rather than left to be found: `artifact-folds`
  sets `progress` on **heading** sections only, so a split file's **preamble** — text before
  its first heading — contributes no span even when it holds items. Those items are still
  counted by the bar's own `progress`, which is the `Change`'s field, so the gauge's fill is
  unaffected; only the boundary marking omits them. A preamble holding task items is not a
  shape any schema's `tasks.md` produces, and the alternative — a span with no header row to
  match it — would mark a boundary the reader cannot see.

  The **detail header** is not a caller: `detail-header` draws its gauge through
  `ui::tasks::gauge_of` directly and never through this function;
- `ui::tasks::group_body(group: &crate::tasks::Group, width)` — every row a group contributes
  below its heading, in document order: its items, each item's own body rows, and its blocks
  interleaved at the positions `task-groups` records for them, with **no** progress bar,
  **no** heading line, and **no** blank separator between groups. It SHALL take a **parsed
  group**, never a source string: `lines` already holds `tasks::Group` values and would have
  to re-serialise each group to call a string-taking form, which is the one thing this
  extraction exists to avoid. A folded tab reaches it through `tasks::parse(&section.text)`
  on a section body that carries no heading of its own, that heading having become the fold
  header.

  This function was named `items` and took `&[crate::tasks::Item]`. It is **renamed** because
  its subject changed: a group's rows are no longer only its items, and a function called
  `items` that also draws fenced blocks would be a name that lies. Every caller — `lines`
  here, and `artifact-content`'s walk in `ui::detail::content_lines` — moves with it;
- `ui::tasks::lines(source, progress, width)` — the whole-tab grammar, which SHALL be
  `bar_lines(progress, &per_group, width)`, where `per_group` is `group.progress()` for each
  `tasks::Group` `tasks::parse(source)` returned, in document order, followed by, for each
  such group in that same order:
  - when the group carries a `Heading`, one line whose text is that heading's `#` markers
    reproduced from its `level`, a space, and its `text` verbatim, carrying
    `Face { heading: Some(level), .. }` so `ui::view::style_for` bolds it with no new
    `Face`-to-`Style` mapping;
  - `group_body(group, width)`;
  - one blank line after every group but the last.

`lines` SHALL be reached only for a **non-foldable** tracked-tasks tab, and **three** files
reach it, not two: a task file holding no items, which renders `No tasks yet`; a task file
with items but no heading at all; and a task file whose text **begins at its single `##`
heading**, which therefore has no preamble, contributes one section, and does not split. That
third file renders a `Face { heading }` line **and** label segments in one content area, so
the two faces can meet; `view-palette` states why that needs no licence. A foldable one is
rendered by `artifact-content`'s walk: `bar_lines` above every header, then `group_body`
inside each open section. The three functions SHALL NOT each reimplement the item grammar:
`lines` calls `group_body`, which is what makes "the folded and unfolded tabs render an item
identically" true by construction rather than by two assertions agreeing.

A group heading is therefore rendered **either** as a `Face { heading }` line, when the tab
is not foldable, **or** as `artifact-folds`' fold header row carrying the heading's text as
its label — never both, and never neither. The `#` markers are dropped on the folding path:
a fold glyph and an indent already say what the `##` markers said, and `artifact-folds`'
header grammar has no place for them.

An item's line SHALL be a prefix followed by its text. The prefix is `item.indent` spaces,
then the three-character glyph `[✓]` when `item.checked` and `[ ]` when it is not, then one
space.

**An item's text SHALL be rendered through `ui::markdown::inline`, not as literal text.** The
text is a fragment, so `markdown-render`'s fragment entry point is what renders it: inline
emphasis, strong, code spans, links, and strikethrough set their faces, and no leading `#`,
`-`, `>`, or digit run opens a block. The rendered rows SHALL be wrapped by that call at
`width - prefix_len` and laid out at the hanging indent below.

This **retires** the rule that an item's text is unfaced, and with it the guarantee that
`Line::text()` is byte-identical to what this capability produced before. That guarantee
cannot survive facing and is deliberately given up: a fragment reading
``add the `CrosstermOps` implementation`` renders as `add the CrosstermOps implementation`
with a `code`-faced segment, its backticks consumed. The reason the guarantee is not worth
keeping is the defect it would preserve — an item's text and its body are one sentence in the
source, and rendering the first row with literal backticks beside a body row with a styled
code span shows one sentence two ways.

An item's rendered **rows** SHALL be faced by exactly one of two rules, chosen by
`item.checked`:

- **A checked item is de-emphasised whole.** Every row it produces — its first row, every
  continuation row of a wrapped item, and every row of its **body** — SHALL be one segment
  carrying `Face { muted: true, ..Face::plain() }`, its prefix and its text alike, and SHALL
  NOT be split at its label or at any inline face. A finished row reads as finished,
  `VERIFY:` and code spans included: the inline faces are **dropped** rather than dimmed
  alongside it, on exactly the terms the label's own role already is, and for the same
  reason — a finished task is not where the reader's eye belongs.
- **An unchecked item is split at its label, when it has one.** `tasks::label_of(&item.text)`
  decides: on `None` the row carries whatever segments `ui::markdown::inline` returned for
  it; on `Some(Label { start, len, role })` the label SHALL be applied to the **leading plain
  segment** of the item's **first** row, splitting that one segment into the prefix
  concatenated with the text before the label under `Face::plain()`, the label itself under
  `Face { label: Some(role), ..Face::plain() }`, and the remainder of that segment under
  `Face::plain()`, with every later segment of the row left exactly as `inline` faced it. A
  **continuation** row of a wrapped item SHALL carry no label: a label appears once, on the
  row it was written on.

`tasks::label_of` returns byte offsets into the item's **plain** text, which no longer
address the rendered row once inline spans fold. The label SHALL therefore be applied only
when the first row's leading segment is `Face::plain()` and its text — after the prefix — is
long enough to contain `start + len` on character boundaries. When it is not, the item SHALL
render **unlabelled**, carrying the faces `inline` returned and no label anywhere. An
emphasised label (`- [ ] **RED**: …`) is the reachable case: its leading segment is
`strong`-faced, so it degrades to unlabelled. That is a **miss, never a wrong colour**, which
is the error direction `task-labels` already chose; measured over this repository's archive,
2,369 items carry a plain label and **0** carry an emphasised one.

A segment SHALL be omitted rather than emitted empty: an item whose text is exactly its label
produces one fewer segment than one with a remainder after it.

`ui::tasks` SHALL call `tasks::label_of` and SHALL NOT reimplement the recognition rule, on
exactly the terms it already calls `tasks::parse` rather than reimplementing the checkbox
rule, and SHALL call `ui::markdown::inline` and `ui::markdown::lines` rather than
reimplementing either. A heading line SHALL keep carrying `Face { heading: Some(level), .. }`
and SHALL be neither muted nor labelled.

**An item's body SHALL be rendered through `ui::markdown::lines`** — the block entry point,
because a body is a document fragment that may hold fenced code, a table, a block quote, or a
nested list — at `width - prefix_len` columns, with every row it returns prefixed by
`prefix_len` spaces so the body aligns under the item's own text. The body SHALL be drawn
immediately after that item's own rows and before the next item or block. An item whose body
is empty SHALL contribute no body row and no blank row.

**A group's blocks SHALL be rendered through `ui::markdown::lines`** at the full `width`, with
no prefix, each drawn at the position `task-groups` recorded for it — before the group's first
item when that position is zero, and otherwise after that many items. A block SHALL be
separated from the rows above and below it by one blank row, so a fenced block does not run
into the item above it; a group carrying no blocks SHALL be byte-identical to the group it was
before blocks existed, blank rows included.

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
the glyph alone truncated by `ui::list::pad_or_truncate_right` at `width`. An item whose
prefix has degraded that far SHALL contribute **no body rows**, on the same drop-whole terms:
a body drawn at a hanging indent wider than the interior has no column to live in, and
dropping it whole is preferred to one character per row. No line's text SHALL exceed `width`
**display columns**, as `responsive-layout` defines them. The `[✓]`/`[ ]` glyph and
`item.indent`'s spaces measure exactly their character counts, so the drop-whole indent rule
above is unchanged; only an item's own text can differ between the two measures.

`width == 0` SHALL return an empty vector, matching `ui::markdown::lines`.

`ui::tasks` SHALL name no `ratatui` type, on exactly the terms `ui::detail` and
`ui::markdown` do not, and SHALL name no filesystem, process, environment, network, or
standard-I/O API. It SHALL call `tasks::parse` and SHALL NOT reimplement it: this
capability renders an existing parse and introduces no second checkbox rule.

#### Scenario: A folded group and an unfolded one render the same item lines

- **WHEN** `ui::tasks::group_body` is called at width `78` and at width `58` over
  `tasks::parse("- [x] 1.1 first\n- [ ] 1.2 second\n").groups[0]`, and
  `ui::tasks::lines` is called at the same two widths over
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n` with
  `Progress { completed: 1, total: 2 }`
- **THEN** `group_body` returns exactly two lines whose texts are `[✓] 1.1 first` and
  `[ ] 1.2 second`, the first carrying one `muted: true` segment and the second one
  `Face::plain()` segment, with no progress-bar line, no heading line, and no blank line
- **AND** `bar_lines` returns exactly two lines — `tasks-progress-bar`'s own bar and one
  blank — and the **empty vector** at a width where the bar renders as the empty string
- **AND** `lines`' own output at the same width is `bar_lines`' two lines, then the heading
  line `## 1. Setup` carrying `Face { heading: Some(2), .. }`, then those same two item
  texts — asserted against these **literals**, not against `lines`' output, because once
  `lines` calls `group_body` a comparison between the two could not fail

#### Scenario: An item's body is drawn under it at its hanging indent

- **WHEN** `ui::tasks::group_body` is called at width `78` and at width `58` over
  `tasks::parse("- [ ] 2.2 GREEN: add the method\n      writing the OSC 52 sequence, and\n      the arm.\n").groups[0]`
- **THEN** at both widths the first row begins `[ ] 2.2 ` and carries a label segment
  `GREEN:` under `Face { label: Some(LabelRole::Change), .. }`
- **AND** the body rows follow it, each beginning with exactly four spaces, their text
  reflowed at `width - 4` and holding `writing the OSC 52 sequence, and the arm.`
- **AND** the 58-column call produces strictly more body rows than the 78-column call, so
  the body genuinely reflows rather than being reproduced line for line

#### Scenario: A fenced block in an item's body renders as code, not as vanished text

- **WHEN** `ui::tasks::group_body` is called at width `78` and at width `58` over an item
  reading `- [ ] run the gate` whose body is a fence opening, `make check`, and a fence
  closing, each indented six columns in the source
- **THEN** at both widths a row exists whose text, with the hanging indent stripped, is
  `make check`, and every segment of that row carries `face.code`
- **AND** no row's text contains a backtick, the fence delimiters having been consumed by
  the renderer rather than printed
- **AND** the same body handed to `ui::markdown::lines` at `width - 4` produces the same
  row texts, so the body path and the ordinary markdown path cannot drift

#### Scenario: A group's block renders between the items it sits between

- **WHEN** `ui::tasks::group_body` is called at width `78` and at width `58` over a group
  holding `- [ ] a`, then a column-zero fenced block reading `make check`, then `- [ ] b`
- **THEN** at both widths the rows are, in order: `[ ] a`, a blank row, a row whose text is
  `make check` with every segment carrying `face.code`, a blank row, and `[ ] b`
- **AND** the block's rows begin at column zero, carrying no hanging indent, because a block
  belongs to the group and not to the item above it
- **AND** a group holding the same two items and no block returns exactly two rows with no
  blank row between them, so the blank separators belong to the block rather than to the
  items

#### Scenario: An item's inline markdown is faced rather than shown as markers

- **WHEN** `ui::tasks::group_body` is called at width `78` and at width `58` over
  ``tasks::parse("- [ ] 1.1 RED: add the `Recorder` arm and **assert** it\n").groups[0]``
- **THEN** at both widths the row carries a `Recorder` segment with `face.code` and an
  `assert` segment with `face.strong`
- **AND** the row's text contains no backtick and no asterisk
- **AND** the row still carries `RED:` under `Face { label: Some(LabelRole::Evidence), .. }`,
  so facing the text did not cost the label

#### Scenario: An emphasised label degrades to unlabelled rather than mis-coloured

- **WHEN** `ui::tasks::group_body` is called at width `78` and at width `58` over
  `tasks::parse("- [ ] **RED**: write the failing test\n").groups[0]`
- **THEN** at both widths no segment of the row carries a `label`
- **AND** the leading text segment carries `face.strong`, the emphasis having been honoured
- **AND** the same item written `- [ ] RED: write the failing test` does carry a label
  segment at both widths, so the degradation is driven by the emphasis rather than being
  unconditional

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
  `Face::plain()` segment, no item text here holding a label or an inline construct

#### Scenario: A nested item reproduces its own indent

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `- [ ] parent\n  - [x] child\n    - [ ] grandchild\n` with
  `Progress { completed: 1, total: 3 }`
- **THEN** at each width the three item lines read `[ ] parent`, `  [✓] child`, and
  `    [ ] grandchild`, so the source's own two- and four-space indents are reproduced
- **AND** the list is flat: no line is dropped, merged, or re-ordered, because
  `tasks::parse` never nests
- **AND** no line is attributed to `parent`'s body, a checkbox line never being body text

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

#### Scenario: A body is dropped whole with the prefix it hangs from

- **WHEN** `ui::tasks::lines` is called at widths `78`, `58`, `12`, `6`, `5`, `4`, `3`,
  `2`, `1`, and `0` over the source `      - [x] alpha\n        a body line\n` (an indent of
  six) with `Progress { completed: 1, total: 1 }`
- **THEN** no call panics and no returned line's text exceeds its width
- **AND** at `78` and `58` the body row follows the item row at the hanging indent
- **AND** at a width where even `[✓] ` leaves no text column, the item contributes exactly
  one row and **no** body row, the body having been dropped whole with the prefix rather
  than wrapped into zero columns
- **AND** at `0` the returned vector is empty

#### Scenario: A heading with no items still renders its heading

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `## 1. Empty\n\nsome prose\n\n## 2. Full\n\n- [ ] only\n` with
  `Progress { completed: 0, total: 1 }`
- **THEN** at each width both headings appear, `## 1. Empty` carries no item line beneath
  it, and `## 2. Full` carries `[ ] only`
- **AND** a row reading `some prose` appears beneath `## 1. Empty`, the prose being that
  group's block rather than discarded content
- **AND** that row is not attributed to any item, group `1. Empty` holding none

#### Scenario: A headingless leading group renders without a heading line

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `- [x] loose\n\n## 1. Later\n\n- [ ] grouped\n` with
  `Progress { completed: 1, total: 2 }`
- **THEN** at each width `[✓] loose` appears before `## 1. Later` with no heading line
  above it
- **AND** exactly one blank line separates the two groups

#### Scenario: A labelled unchecked item splits into three segments

- **WHEN** `ui::tasks::group_body` is called at width `78` and at width `58` over
  `tasks::parse("- [ ] 1.1 RED: write the failing test\n- [ ] Commit: the parser\n").groups[0]`
- **THEN** the first item's row carries exactly three segments — `[ ] 1.1 ` under
  `Face::plain()`, `RED:` under `Face { label: Some(LabelRole::Evidence), ..Face::plain() }`,
  and ` write the failing test` under `Face::plain()`
- **AND** the second item's row carries exactly **one** `Face::plain()` segment, `Commit:`
  being a one-letter uppercase run and so no label at all
- **AND** at both widths each row's `Line::text()` equals the item's source text with its
  prefix prepended, neither item carrying an inline construct whose markers facing would
  consume

#### Scenario: A checked item is de-emphasised whole, label included

- **WHEN** `ui::tasks::group_body` is called at width `78` and at width `58` over
  ``tasks::parse("- [x] 1.1 VERIFY: `make check` is green\n").groups[0]``
- **THEN** the row carries exactly **one** segment, whose text is the whole row and whose
  face is `Face { muted: true, ..Face::plain() }`
- **AND** that segment's `label` is `None` and its `code` is `false`, so the completed row
  carries neither a label role nor a code face for `ui::view` to colour — the de-emphasis is
  not a dimmed `VERIFY:` beside a bright code span but neither role at all
- **AND** the same text with `[ ]` instead of `[x]` returns a label segment with
  `label: Some(LabelRole::Confirm)` and a `make check` segment with `face.code`, so the two
  paths are asserted against each other and a rule that muted both, or neither, could not
  pass

#### Scenario: A wrapped labelled item labels only its first row

- **WHEN** `ui::tasks::group_body` is called at width `58` over a single unchecked item whose
  text is `1.1 GREEN: ` followed by twenty words of eight characters each, so the item wraps
  to more than one row
- **THEN** the first row carries a `GREEN:` segment under
  `Face { label: Some(LabelRole::Change), .. }`
- **AND** every continuation row carries segments with `label: None` and `muted: false`
- **AND** the concatenation of every row's `text()`, with the hanging indent stripped, holds
  the item's whole text, so no character was lost to the split

#### Scenario: A label split across a wrap degrades to unlabelled

- **WHEN** `ui::tasks::group_body` is called over a single unchecked item whose text is
  `1.1 CHARACTERIZE: record the baseline`, at a width where the first row's text column ends
  inside `CHARACTERIZE:` — width `12`, `14`, and `16`
- **THEN** at each of those widths no segment carries a `label`
- **AND** nothing panics, and the concatenation of every row's text, with the hanging indent
  stripped, reproduces the item's whole text
- **AND** at widths `58` and `78`, where the label fits on the first row whole, the same item
  does carry a label segment, so the degradation is width-driven rather than unconditional
