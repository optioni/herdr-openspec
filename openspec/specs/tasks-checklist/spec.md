# tasks-checklist Specification

## Purpose
The detail tab that renders a change's task file as a checklist instead of markdown: the
single decision — made from the selected artifact's `tracks_tasks` flag, never from its id or
filename — that swaps `ui::markdown`'s grammar for `ui::tasks`', and the line grammar that
results. Which of `ui::tasks`' three entry points draws it follows the tab's foldability, not
a second decision about its kind: an unsplit file is `ui::tasks::lines` whole, and a file that
split at its headings is `ui::tasks::bar_lines` above every section header with
`ui::tasks::group_body` inside each open group — `lines` calls `group_body` per group itself, so
a folded group and an unfolded one cannot disagree about an item line, and a group's heading is
drawn as its header row or as a heading line but never both and never neither. The grammar is
heading lines reproduced from their level, then, per group in document order, one `[✓]`/`[ ]`
glyph line per item — the same glyph `markdown-render` gives a task-list item, so the two
checkbox renderers agree by construction and what names this path is the progress-bar row
rather than the glyph — each item's own **body** beneath it at a hanging indent that falls
after the task number, and each of the group's **blocks** at column zero between the items they
sit between. An item's text is a fragment and goes through `ui::markdown::inline`; its body and
the group's blocks are documents and go through `ui::markdown::lines`, so fenced code, tables
and block quotes draw with the grammar every other tab already uses, and item text is faced
rather than shown as literal markers. A block is separated from the rows around it by exactly
one blank row and by none where there is no row to separate from. The parse's own indent,
whole-indent dropping as the width collapses — which drops an item's body with the prefix it
hangs from — and blank separators between groups are all preserved. It fixes `No tasks yet` for
a source that holds no items, followed by that source's own retained blocks, kept distinct from
`artifact-content`'s `No content yet` for a source that does not exist, and it carries the
read-only guarantee: no key toggles an item, no action mutates a `Change`, and no module the
dashboard reaches names a filesystem write API. The bar that leads the tab is
`tasks-progress-bar`'s.

## Requirements

### Requirement: The tracked-tasks tab renders a checklist, and every other tab does not

`ui::detail::content_lines` SHALL render the selected tab as a checklist — the grammar this
capability specifies — when, and only when, the selected change's `ArtifactRef` at
`detail.tab` carries `tracks_tasks == true`. Every other tab SHALL keep
`ui::markdown::lines` unchanged.

The decision SHALL be made in exactly one place, from `Change::artifacts[detail.tab]`, so
the drawn slice (`ui::view::render`) and the scroll clamp (`Dashboard::normalise_scroll`)
can never disagree about which grammar the tab holds. It decides the **grammar** only.
Whether the tab **folds** is a separate question with a separate answer —
`Detail::foldable()`, derived from the section list `sync_detail` built — and
`heading-sections` separates them deliberately: before it, `tracks_tasks` answered both, and
the tab was exempt from folding because of it. The decision SHALL NOT be made by
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

A tracked-tasks tab SHALL fold exactly when its file split into sections, which
`artifact-folds` gates on `tasks::count(text).total > 0`. A task file holding no items
therefore never folds, which is what keeps "A source holding no task lines renders `No tasks
yet`" true by construction: that requirement's "**no** heading line even where the source
carries headings" cannot be contradicted by a header row, because a file with no items is
never split into sections that could produce one.

#### Scenario: The tasks tab shows checkboxes and its siblings show markdown

- **WHEN** a `Dashboard` at `Route::Detail`, whose selected change carries the five `tdd`
  artifacts with `tracks_tasks` set at position 3 (`tasks`) and whose file holds
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n`, is
  rendered at 120x20 and at 60x20 with `detail.tab == 3`
- **THEN** in each buffer the content area holds a progress-bar row, a blank row,
  `v 1. Setup`, `[✓] 1.1 first`, and `[ ] 1.2 second`, in that order — the group heading is
  now that group's own **fold header**, open because its subtree is incomplete, and the `##`
  markers are gone with the heading line
- **AND** the source carries **two** groups deliberately. One group splits into exactly one
  section, which `Detail::foldable` reports **not** foldable per design.md -> Decision 10, so
  no header row is drawn for it and the `v 1. Setup` row above would not exist. A fixture
  that means to exercise folding needs two sections, and this is the third scenario in this
  change to have been written with a one-group source before that was noticed
- **AND** the same `Dashboard` with `detail.tab == 0` renders the identical source through
  `markdown::lines` instead: the content area's first row reads `## 1. Setup` with **no
  progress-bar row, no blank row, and no fold header above it** — that artifact does not
  carry `tracks_tasks` and its text carries no `### Requirement:` heading, so it does not
  split. That is what discriminates the two paths now
  that `markdown-render` models task-list items and both paths render the item rows as
  `[✓] 1.1 first` and `[ ] 1.2 second`. The glyphs agreeing across the two paths is asserted
  here, not incidental: it is the structural answer to the drift `markdown-render` names.
- **AND** the tab bar in row 2 is byte-identical between the two renders, so switching tabs
  changed content and nothing else

#### Scenario: The tab is chosen by `tracks_tasks`, not by its id

- **WHEN** a `Dashboard` whose selected change carries two artifacts, `checklist` at
  position 0 with `tracks_tasks == true` and `tasks` at position 1 with
  `tracks_tasks == false`, and whose one section holds `- [x] done\n`, is rendered at
  120x20 and at 60x20
- **THEN** with `detail.tab == 0` the content area holds the checklist grammar — a
  progress-bar row and a `[✓] done` row
- **AND** with `detail.tab == 1` the content area holds a `[✓] done` row and **no
  progress-bar row**, even though that artifact's id is `tasks` — the markdown path renders
  the same glyph, so the absence of the bar is what names the path taken
- **AND** neither render panics and the tab bar shows both cells in both cases

#### Scenario: A schema naming no tasks artifact leaves every tab as markdown

- **WHEN** a `Dashboard` whose selected change carries three artifacts, none with
  `tracks_tasks == true`, and whose one section holds `- [ ] a\n`, is rendered at 120x20
  and at 60x20 with `detail.tab` at each of `0`, `1`, and `2`
- **THEN** no render shows a progress-bar row and every render shows `[ ] a`, the markdown
  path's own task-list rendering
- **AND** the tab bar still holds all three cells in every render, so no tab was removed

#### Scenario: A `detail.tab` past the end of the artifact list renders no checklist

- **WHEN** `content_lines` is called at width `78` and at width `58` for a `Detail` whose
  `tab` is `7` over a change carrying two artifacts, the second of which has
  `tracks_tasks == true`, with a source of `- [x] a\n`
- **THEN** neither call panics and both return the markdown rendering, because index `7`
  names no artifact and therefore names no tracked-tasks artifact
- **AND** the same holds for a change carrying no artifacts at all

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
`width - hang` and laid out at the hanging indent defined below.

This **retires** the rule that an item's text is unfaced, and with it the guarantee that
`Line::text()` is byte-identical to what this capability produced before. That guarantee
cannot survive facing and is deliberately given up: a fragment reading
``add the `Recorder` implementation`` renders as `add the Recorder implementation`
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
2,531 items carry a plain label and **0** carry an emphasised one.

A segment SHALL be omitted rather than emitted empty: an item whose text is exactly its label
produces one fewer segment than one with a remainder after it.

`ui::tasks` SHALL call `tasks::label_of` and SHALL NOT reimplement the recognition rule, on
exactly the terms it already calls `tasks::parse` rather than reimplementing the checkbox
rule, and SHALL call `ui::markdown::inline` and `ui::markdown::lines` rather than
reimplementing either. A heading line SHALL keep carrying `Face { heading: Some(level), .. }`
and SHALL be neither muted nor labelled.

**An item's body SHALL be rendered through `ui::markdown::lines`** — the block entry point,
because a body is a document fragment that may hold fenced code, a table, a block quote, or a
nested list — at `width - hang` columns, with every row it returns prefixed by
`hang` spaces, the same hanging indent the item's own wrapped text uses, so an item's text and
its body form one continuous text column beneath the task number. The body SHALL be drawn
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

**The hanging indent SHALL fall after the item's task number, not under it.** The hang is
`prefix_len + tasks::task_number_len(&item.text)` columns: the prefix, plus the width of the
leading task number `task-labels` already skips when it looks for a label. An item carrying no
task number has a `task_number_len` of zero and hangs at `prefix_len`, exactly as before.

The item's text SHALL be word-wrapped to `width - hang` columns, with continuation
lines indented by `hang` spaces so they align under the first line's text after its number —
a hanging indent, the same shape `markdown-render` gives a list item. A single word longer
than the available text column SHALL be hard-split at that column rather than overflowing the
interior or being dropped, so a long path never silently loses its tail.

Hanging after the number rather than under it keeps the number column clear, so a reader
scanning for `2.3` reads a column of numbers rather than a column of numbers interleaved with
wrapped prose. It **departs from the source file's own convention** — a `tasks.md` continuation
line is conventionally indented to six columns, which lands under the number, because the
source prefix is `- [x] ` and the rendered one is `[x] `. The rendered prefix is a different
width from the source's, so reproducing the source's column would align with nothing on
screen; this capability aligns with what it actually draws. Measured over this repository's
archive, **2,842 of 2,842** items carry a task number, of width 4 (2,168), 5 (663), or 6 (11)
characters including its trailing space, so the hang is 8 columns for three items in four and
never more than 10.

The number hang SHALL be **dropped whole** before the prefix is: when
`prefix_len + task_number_len` would leave no text column, the hang falls back to
`prefix_len`, and only then does the existing prefix degradation below apply. So a width that
can hold the glyph and some text never loses the text to the number's indent.

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
- **AND** the body rows follow it, each beginning with exactly eight spaces — four for the
  prefix and four for the task number `2.2 ` — their text reflowed at `width - 8` and holding
  `writing the OSC 52 sequence, and the arm.`
- **AND** this fixture's forty-one reflowed characters fit **one** body row at both widths —
  a 50-column body at `58` and a 70-column one at `78` — so the fixture pins the hang, the
  label, and the reflowed text, and cannot itself demonstrate a width-dependent row count
- **AND** the same item carrying a body long enough to wrap at both widths produces strictly
  more body rows at `58` than at `78`, so the body genuinely reflows rather than being
  reproduced line for line

#### Scenario: The hanging indent falls after the task number

- **WHEN** `ui::tasks::group_body` is called at width `78` and at width `58` over three
  unchecked items long enough to wrap at both widths, whose texts begin `1.1 `, `10.11a `, and
  with no number at all
- **THEN** at both widths the first item's continuation rows begin with exactly eight spaces,
  the second's with exactly eleven, and the third's with exactly four
- **AND** in each case the first row's text at column `hang` is the same character the
  continuation rows begin with, so the text column is continuous down the item
- **AND** the numbers themselves appear only on each item's first row, no continuation row
  repeating or re-indenting under one

#### Scenario: The number hang is dropped before the prefix is

- **WHEN** `ui::tasks::group_body` is called at widths `78`, `58`, `20`, `12`, `10`, `8`, `6`,
  `5`, `4`, and `0` over a single unchecked item whose text is `1.1 ` followed by forty
  characters of space-separated words
- **THEN** no call panics and no returned line's text exceeds its width
- **AND** at the widths where `[ ] 1.1 ` leaves at least one text column, continuation rows
  begin with eight spaces
- **AND** at a width below that but where `[ ] ` still leaves a text column, continuation rows
  begin with exactly four spaces and the item's text is still rendered — the number hang was
  dropped whole and the prefix was not
- **AND** at `0` the returned vector is empty

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
  four spaces, the item carrying no task number, so the hang is the prefix alone
- **AND** the same item written with a leading `1.1 ` hangs at eight spaces instead, so the
  number's contribution is asserted against its absence rather than on its own
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
- **AND** at every width of the **glyph-only** band — where the indent is gone but `[✓] `
  still fits, here `9`, `8`, `7`, `6`, and `5` — the item renders exactly what the same item
  carrying **no** body renders, the body being dropped with the indent rather than hung from
  the four columns the glyph leaves. Measured before this rule was implemented, width `5`
  drew the body as fourteen one-character rows, which is the outcome dropping whole exists to
  avoid, and is why the two bands are one rule rather than the truncated-glyph case alone
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

### Requirement: A source holding no task lines renders `No tasks yet`

When `tasks::parse(source)` yields no **items** — no group holds one, so
`Tasks::progress().total == 0` — the checklist SHALL render the progress-bar line, its blank
line, one further line reading `No tasks yet`, and then each group's own retained **blocks**,
and SHALL render **no** heading line even where the source carries headings.

The blocks are drawn because a document with zero items is precisely the document whose every
non-blank line is a block: `task-groups`' leading headingless group is emitted when it carries
blocks and no items, so a prose-only `tasks.md` parses to exactly one group holding exactly
its prose. Discarding it here would throw away the whole file on the one path whose reason for
existing is that the file has nothing else in it. The row count is therefore no longer fixed
at one — the previous text said "exactly one further line", true only while a group with no
items had nothing left to draw — and what is fixed is the **order**: the bar, its blank line,
`No tasks yet`, then the blocks. Each block is drawn by `ui::tasks::group_body` over a group
whose `items` slice is empty, which is the same function and the same block grammar the
item-bearing path uses, so the two cannot drift apart.

No heading line is drawn on this path, unchanged: `group_body` renders a group's items and
blocks and never its heading, and the heading loop is not reached at all.

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

When `detail.sections` is **empty**, `artifact-content`'s existing rule governs and
`No content yet` SHALL be rendered instead: an artifact file that does not exist is a
different state from one that exists and holds no tasks, and the two SHALL NOT be
conflated. `No tasks yet` and `No content yet` SHALL never both appear for the same tab.

#### Scenario: A prose-only tasks file reads `No tasks yet`

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change's tracked-tasks tab is
  selected, whose one section holds `# Plan\n\nNothing checkable here.\n`, and whose
  change carries `Progress { completed: 0, total: 0 }`, is rendered at 120x20 and at 60x20
- **THEN** in each buffer the content area's first row holds `[-]` and its third row reads
  `No tasks yet`
- **AND** `No content yet` appears in neither buffer
- **AND** `# Plan` appears in neither buffer, even though `tasks::parse` returned one group
  for that heading: the trigger is zero items, and a document with no items renders no
  heading lines
- **AND** in each buffer that third row measures exactly the interior width — 78 and 58 —
  because the literal is now padded like every line around it

#### Scenario: A prose-only document draws its blocks beneath `No tasks yet`

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over a source holding
  a heading and prose but no checkbox line at all, whose leading group therefore carries one
  block and no items
- **THEN** at both widths the rows are the progress-bar line, a blank line, `No tasks yet`,
  **one blank line**, and then the block's own rows, in that order — the blank separating the
  status row from the content beneath it
- **AND** the block's text reaches the screen rather than being discarded with the items the
  document does not have
- **AND** no heading line is drawn at either width, the source's heading notwithstanding

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
  did not resolve to — and whose `detail.sections` and `detail.problems` are both empty, is
  rendered at 120x20 and at 60x20
- **THEN** in each buffer the content area's first row reads `No content yet`
- **AND** no progress-bar row and no `No tasks yet` row appears, even though the change's
  `progress` is non-zero and the detail header one row above still shows `[4/9]`
- **AND** this is the one place the tab's content and the change's progress legitimately
  disagree; `SPEC.md`'s degraded-states table carries a row for it

#### Scenario: A read failure on the tasks tab names its reason and renders no checklist

- **WHEN** the same `Dashboard` has `detail.problems ==
  ["/repo/openspec/changes/x/tasks.md: permission denied"]` and no sections at all
  and is rendered at 120x20 and at 60x20
- **THEN** in each buffer the content area's first row begins
  `! /repo/openspec/changes/x/tasks.md:` and is exactly the interior width — 78 and 58
- **AND** neither `No content yet` nor `No tasks yet` nor a progress-bar row appears, so
  the tab says one thing about itself rather than two

### Requirement: The tasks tab is read-only and writes nothing

No key SHALL toggle, check, uncheck, insert, delete, or reorder a task item, and no code
path reachable from the dashboard SHALL open a file under `openspec/` for writing, create
a directory there, or change a modification time there. Rendering a checkbox is what makes
toggling one look natural; `PRD.md` → Non-goals forbids it because an agent may be editing
`tasks.md` in another pane and a write would race it.

`live-refresh` makes that race concrete rather than hypothetical: the dashboard now holds an
open `notify` watch on `openspec/` while an agent writes to it, so the read-only guarantee is
under more pressure here than anywhere else. It is unchanged in substance — the watcher, the
worker, and every `Refresher` result are reads — and the source-level sweep it is proved by
now covers `src/watch.rs`, `src/refresh.rs`, `src/agents.rs`, and `src/launch.rs` alongside
`src/ui/`. `src/agents.rs` joined the searched set with `agent-polling`'s poller; this
requirement's prose still named only the first two and is corrected here.

**`agent-launch` is where this requirement is under the most pressure it has ever been**, and
the boundary it draws is the load-bearing sentence of that change: `a` / `c` / `s` start a
coding agent whose whole purpose is to edit files inside `openspec/`. The plugin still writes
nothing there. What it does is start a **separate process** in a **separate pane**, and every
byte that process writes it writes as itself, under its own working directory, through its own
tools — the plugin's causal contribution ends at `herdr agent prompt`. The boundary is the
process boundary, and the evidence is mechanical rather than argumentative: the launch tier's
only write is `state::record` into `HERDR_PLUGIN_STATE_DIR`, and the acceptance scenarios take a
byte-identical snapshot of the whole repository tree across a run in which an agent was
launched.

`ui::app::action_for` SHALL map no key to a task-mutating action, and `Dashboard::apply`
SHALL gain no arm that edits a `Change`, an `ArtifactRef`, or a `Progress`. `live-refresh`
added exactly **one** `Action` variant, `Refresh`, whose `apply` arm sets
`refresh.requested` and touches nothing else; `agent-launch` adds exactly **four** —
`LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent` — whose `apply` arms write
only `Dashboard::launch`, a one-shot `Option<launch::Request>` and a `Vec<String>` of at most
one entry. **None of the four reaches a collaborator, spawns a process, opens a file, or reads
a clock**: they are the first four actions whose *eventual* effect is outward, and the whole
reason `apply` can stay pure is that what they produce is **data**. `ui::driver::run_loop` hands
that data to `launch::Launcher`, whose real implementation lives in `src/launch.rs`, outside
`src/ui/` entirely, and which is the only thing in the crate that turns it into a process.

The set of keys the dashboard responds to SHALL
be exactly the set `dashboard-loop`, `list-selection`, `list-filtering`, `detail-scroll`,
`artifact-tabs`, `live-updates`, and `agent-launch` specify. `live-updates`' addition is `r`,
which `SPEC.md` → Keys has documented as "Force refresh" since `repo-foundation`;
`agent-launch`'s additions are `a`, `c`, `s`, and `g`, which `SPEC.md` → Keys and `README.md`
→ Keys have both documented since `repo-foundation` and which this change is the one to make
true.

#### Scenario: Every printable key leaves the change tree byte-identical

- **WHEN** `run_loop` runs against a `TestBackend` of 120x20 and again of 60x20, over a
  `Dashboard` loaded by `ui::load` from a real `crate::testutil::ScratchDir` repository
  holding one change whose `tasks.md` has two checked and three unchecked items, with the
  tracked-tasks tab selected, driven by a scripted source delivering a Press of every
  ASCII printable character from `!` to `~`, then `Enter`, `Esc`, `Backspace`, `Tab`, and
  the four arrows, and finally `Ctrl-C`
- **THEN** the run terminates and `crate::testutil::snapshot` over that scratch directory
  is byte-identical to the snapshot taken before the run, including every file's bytes,
  size, and modification time
- **AND** re-reading `tasks.md` after the run yields `Progress { completed: 2, total: 5 }`,
  the same pair as before it
- **AND** the assertion discriminates: the same snapshot comparison fails when a single
  byte of `tasks.md` is deliberately rewritten between the two snapshots
- **AND** the ASCII sweep now includes `r`, which `live-updates` maps to `Action::Refresh`
  outside filter mode — so this scenario is also the proof that a refresh, like every other
  key, writes nothing. `live-updates`' own
  `ui::tests::live::a_live_watcher_over_the_tree_writes_nothing` makes the same claim with a
  **real** `notify` watch open on the tree throughout
- **AND** the sweep also includes `a`, `c`, `s`, and `g`, which `agent-launch` maps to its four
  actions, and the run is driven with `agents.reachable` **true** and a **recording** launcher
  double so all four reach `launch::decide` and produce real requests rather than being
  short-circuited — the snapshot claim is worth nothing if the keys did nothing. The recording
  launcher received requests; the tree is still byte-identical

#### Scenario: No action mutates a task item

- **WHEN** every `ui::app::Action` variant is applied in turn to a `Dashboard` whose
  selected change carries a tracked-tasks artifact, whose `agents.reachable` is `true`, and
  whose visible list is non-empty
- **THEN** the `Dashboard`'s `changes` field is `==` to its value before the application
  for every variant, so no action edits a `Change`, an `ArtifactRef`, or a `Progress`
- **AND** the `Action` enum holds exactly the **seventeen** variants `dashboard-loop`,
  `list-filtering`, `artifact-tabs`, `live-updates`, and `agent-launch` specify, with no
  toggle, check, edit, save, or write variant among them. `live-updates`' `Refresh` is the
  thirteenth, and it sets a `bool` on `Dashboard::refresh`; `agent-launch`'s `LaunchApply`,
  `LaunchContinue`, `LaunchArchive`, and `FocusAgent` are the fourteenth through seventeenth,
  and each writes only `Dashboard::launch`. **None of the five reaches a collaborator, performs
  any I/O, or starts any work of its own**: the launch actions produce a `launch::Request`
  value, and `run_loop` — not `apply` — is what hands it to something that can act on it
- **AND** the count is bumped from thirteen to seventeen **deliberately**, as this scenario's
  own subject rather than as a repair to a failing assertion: the number exists so a mutating
  action cannot be added silently, and moving it is the moment to state what the four new ones
  do and do not touch
- **AND** the variant count is asserted against an array the test builds through an
  exhaustive `match`, so an eighteenth variant is a compile error rather than a silent gap in
  the sweep
- **AND** after each of the four launch variants the dashboard's `launch.pending` is `Some` or
  its `launch.problems` is non-empty, which is what proves the fixture reached the decision
  rather than satisfying "changes is unchanged" vacuously through an unreachable socket

#### Scenario: The dashboard names no write API

- **WHEN** every file under `src/ui/`, **plus `src/watch.rs`, `src/refresh.rs`,
  `src/agents.rs`, and `src/launch.rs`**, is
  searched for a filesystem-write API — `fs::write`, `File::create`, `OpenOptions`,
  `fs::remove_`, `fs::create_dir`, `fs::rename`, `fs::copy`, `set_permissions`,
  `fs::hard_link`, and `fs::soft_link` — over each file's production slice, with
  `#[cfg(test)]` modules stripped
- **THEN** there is no match
- **AND** the four extra modules are in the searched set because `live-refresh`,
  `agent-polling`, and `agent-launch` put the watcher, the worker, the poller, and the launcher
  there: a watcher is one edit away from a marker file, a worker one edit away from writing
  beside the tree it reads, and the launcher is the module with an actual reason to write —
  none of which would have been visible to a sweep scoped to `src/ui/`
- **AND** `src/launch.rs` passes it while genuinely persisting the agent-name mapping, which is
  the point: it writes only by calling `state::record`, and every write API this check names
  lives in `src/state.rs`, inside the state directory `plugin-state` confines it to. A launcher
  that reached for `fs::write` itself — to log, to mark, to cache — would be caught here
- **AND** it is paired with a positive control asserting that `src/state.rs`'s production
  slice **does** name a write API, and with a second control proving the `#[cfg(test)]`
  stripper is not vacuous — `src/ui/mod.rs`'s test slice must name a write API its
  production slice does not
- **AND** the check is proven able to fail against a copy carrying
  `let _ = std::fs::write(path, "x");` inside `ui::read_artifact`, against one carrying
  the same call in `src/watch.rs`'s production code, and against one carrying it in
  `src/launch.rs`'s production code

#### Scenario: A launched agent's writes are not the plugin's writes

- **WHEN** `ui::run_wired` is driven over a real scratch repository, a real scratch state
  directory, and a scratch `herdr` program, and an `a` press launches an agent end to end —
  the three Herdr calls are logged, the mapping is recorded, and the returned dashboard's
  `agent_names` carries the pair
- **THEN** `crate::testutil::snapshot` over the repository tree is byte-identical to the
  snapshot taken before the run, including every file's bytes, size, and modification time
- **AND** the only file written anywhere by the plugin is `agent-names.toml` inside the scratch
  **state** directory, proven by a second snapshot pair over that directory showing exactly one
  file appearing
- **AND** the boundary is stated rather than implied: what the plugin started would, against a
  real Herdr and a real agent, go on to edit `openspec/`; the scratch program starts nothing, so
  this scenario measures the plugin's own writes and nothing else, which is exactly the claim
  `PRD.md` → Non-goals makes

### Requirement: The checklist's lines are measured in display columns at every width

Every line `ui::tasks::lines` produces — the progress-bar line, the blank separators, each
heading line, each task item line and its wrapped continuations, and the `No tasks yet`
line — SHALL measure at most `width` **display columns** as `responsive-layout` defines
them, at **every** `width`, not only at the two mandated interiors of 78 and 58.

`ui::tasks` SHALL reach that measure only through `layout::columns` and
`layout::truncate_columns`, and SHALL continue to name no `ratatui` type. The item wrap SHALL
break at a **grapheme-cluster boundary**, so a task item holding a wide character, an emoji,
or a combining mark can never produce a line wider than the region and never ends in half a
cluster. The `[✓]`/`[ ]` glyph, its separating space, and the hanging indent under it measure
exactly their character counts — `✓` (U+2713) is East Asian **Neutral** and one column, so the
glyph is three columns exactly as `[x]` was, and no arithmetic in this capability changes; the
indent a continuation carries SHALL be as many spaces as the glyph and its space measure in
columns.

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
- **AND** the `[✓]` and `[ ]` glyphs still begin at the same interior columns they do for an
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
