## MODIFIED Requirements

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

- `ui::tasks::bar_lines(progress, width)` — the progress-bar line, `tasks-progress-bar`'s
  single line as one plain-faced segment, followed by one blank line; the **empty vector**
  when the bar renders as the empty string at that width;
- `ui::tasks::items(items: &[crate::tasks::Item], width)` — one or more lines per item, in
  order, as specified below, with **no** progress bar, **no** heading line, and **no** blank
  separator. It SHALL take **parsed items**, never a source string: `lines` already holds
  `tasks::Group` values and would have to re-serialise each group to call a string-taking
  form, which is the one thing this extraction exists to avoid. A folded tab reaches it
  through `tasks::parse(&section.text)` on a section body that carries no heading of its own,
  that heading having become the fold header;
- `ui::tasks::lines(source, progress, width)` — the whole-tab grammar, unchanged, which
  SHALL be `bar_lines` followed by, for each `tasks::Group` returned by `tasks::parse(source)`
  in document order:
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
  `[ ] 1.2 second`, each carrying `Face::plain()`, with no progress-bar line, no heading
  line, and no blank line
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
- **AND** the two heading lines carry `Face { heading: Some(2), .. }` and every other line
  carries `Face::plain()`

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
