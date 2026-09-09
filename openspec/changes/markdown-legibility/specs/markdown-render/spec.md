## MODIFIED Requirements

### Requirement: Markdown source becomes plain-data lines, parameterised by the interior width

`ui::markdown::lines(source: &str, width: u16) -> Vec<Line>` SHALL turn a markdown string
into the lines a region `width` columns wide would show, as a **pure total transformation**:
it SHALL perform no filesystem, process, environment, network, or standard-I/O work, read
no clock and no global state, and SHALL NOT panic for any `&str` and any `u16`.

The types SHALL be plain data carrying no `ratatui` type:

```rust
pub struct Face {
    pub heading: Option<u8>,   // 1..=6 for a heading line, None otherwise
    pub strong: bool,
    pub emphasis: bool,
    pub code: bool,
    pub link: bool,
    pub quoted: bool,
    pub strikethrough: bool,
}
pub struct Segment { pub text: String, pub face: Face }
pub struct Line { pub segments: Vec<Segment> }
pub fn lines(source: &str, width: u16) -> Vec<Line>;
```

`strikethrough` is this change's one new field. `Face` SHALL keep deriving `Default` — it is
a value with a meaningful zero, not a state type `NODEFAULT-UI` gates — so `Face::plain()`
SHALL remain the all-`false`, `heading: None` value and SHALL now include
`strikethrough: false`. Every construction site that spells the fields out rather than
writing `..Face::plain()` SHALL name the new field: `src/ui/tasks.rs`'s `heading_line` is the
one such site in the crate, and it is a **compile-time** forcing site, which is why the field
is added to the struct rather than tracked in a parallel enum.

`Line::text()` SHALL return the line's segments concatenated, which is what a width assertion
reads. Mapping a `Face` to a `ratatui::style::Style` is `ui::view`'s work and SHALL NOT
happen here, on the same terms as `ui::list` returning plain `String`s; which `Style` a face
role carries is `view-palette`'s.

Every returned `Line`'s `text()` SHALL be at most `width` **display columns**, measured by
`layout::columns` — the same measure `ui::list`, `ui::tasks`, `ui::detail`, and `ui::view`
use, and the measure `ratatui::buffer::Buffer::set_string` itself consumes. A `char` count is
no longer the unit and is no longer a documented limitation: a source holding wide or
combining characters is measured correctly rather than approximately. A line SHALL NOT be
padded to the width: a region draws segments left to right and leaves the rest of the row
untouched. A **table row line** is the one line kind that carries interior padding, and it is
not an exception to that sentence: it is padded to its own table's total width — the sum of
the allocated column widths plus the pipe-and-space overhead — which is at most `width` and
is frequently less, and never to the region's width.

`ui::markdown` SHALL reach that measure through `layout::columns` and
`layout::truncate_columns` only, and SHALL still name **no** `ratatui` type of its own —
those two functions are the seam, exactly as `ui::list::pad_or_truncate_right` already is for
`ui::detail`. The confinement `markdown-render` already requires of `pulldown_cmark` is
unaffected.

Wrapping SHALL be greedy at ASCII spaces, and a token that does not fit the width available
to it — even on a line of its own — SHALL be **hard-split** at a **grapheme-cluster
boundary**, taking the longest prefix that fits the columns available and continuing the
remainder on the next line, so a URL or an unbroken identifier can never produce a line
wider than the region and no line ever ends in half a cluster. Where a cluster is dropped
whole the resulting line MAY measure one column less than the width; that is a short line,
not a wrapped-wrong one, and no padding is added to hide it. This applies to prose as well
as to code; the two differ only in whether spaces are break opportunities. A table cell's
content wraps by this same rule, in the columns its own column was allocated.

A block that carries a prefix or an indent — a list marker, a nesting indent, a block
quote's `> ` — SHALL lay its content out in `width.saturating_sub(prefix)` columns, where
`prefix` is that prefix's own `layout::columns`. When that leaves zero columns, the block
emits its prefix truncated to the width **in columns** and no content, rather than
underflowing or panicking. The hanging indent a continuation line carries SHALL be as many
spaces as the first line's marker measured in columns, so a wide-character list marker's
continuations still align under its content.

A single grapheme cluster wider than the whole region SHALL be dropped rather than emitted:
at `width` 1 a two-column cluster produces an empty line for that position, never a line
that measures 2. This is the one case in which content is lost rather than wrapped, and it
is preferred to overflowing the region, which is the failure this capability exists to
prevent.

`lines` SHALL return an empty `Vec` when `source` is empty and when `width` is `0`.

The two interior widths every scenario below is exercised at are **78** and **58**. They are
frozen by `responsive-layout`, which owns the layout that produces them; this capability
consumes them rather than defining them.

#### Scenario: A paragraph is word-wrapped, differently at the two mandated widths

- **WHEN** `lines` is called with the single paragraph `alpha bravo charlie delta echo
  foxtrot golf hotel india juliett kilo lima mike november oscar papa` at width 58 and
  again at width 78
- **THEN** at 58 it returns exactly two lines whose `text()` values are
  `alpha bravo charlie delta echo foxtrot golf hotel india` and
  `juliett kilo lima mike november oscar papa`
- **AND** at 78 it returns exactly two lines whose `text()` values are
  `alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike` — 78
  columns exactly, so the boundary is inclusive — and `november oscar papa`
- **AND** every segment of every line carries `Face::plain()`, and the result is
  byte-identical to the one this requirement produced before display-column measurement,
  because every character in it measures one column

#### Scenario: An empty source and a zero width each produce no lines

- **WHEN** `lines("", 58)`, `lines("", 78)`, `lines("# Proposal\n", 0)`, and
  `lines("", 0)` are called
- **THEN** each returns an empty `Vec`
- **AND** nothing panics, so a detail region that has been squeezed to zero columns is a
  no-op rather than a crash

#### Scenario: No line exceeds the width it was given

- **WHEN** a document holding a heading, a paragraph, a bullet list, a nested list, a
  fenced code block, a block quote, a thematic break, a link, **a three-column pipe table
  whose widest cell is wider than the narrow interior**, **a struck run**, **a checked and
  an unchecked task-list item**, and **a task-list item nested inside a block quote** is
  rendered at widths 0, 1, 2, 3, 10, 58, 78, and 200
- **THEN** at every width every returned line's `text()` has at most that many **display
  columns**, measured by `layout::columns`
- **AND** the rendering at 58 and at 78 both hold at least one line of each block kind, the
  table's, the struck run's and both task-list items' included, so the assertion is made
  against real content rather than an empty result
- **AND** the task-list item's four-column prefix is swept below its own width — 1, 2 and 3
  are all under it — so `width.saturating_sub(prefix)` is measured at underflow rather than
  assumed total, which is the case a fixture holding only 58 and 78 cannot reach
- **AND** the swept widths include several below `4n + 1` for the fixture's three-column
  table — 1, 2, 3, and 10 all are — so the degenerate one-cell-per-line path is measured
  rather than assumed

#### Scenario: A wide-character document wraps by columns at both mandated widths

- **WHEN** a document whose paragraph is forty repetitions of `日本語` and whose bullet list
  holds `- 🎉 celebrate` and an item wrapped in a family emoji joined by zero-width joiners
  is rendered at widths 58 and 78
- **THEN** at each width every returned line's `layout::columns` is at most that width
- **AND** at least one line measures exactly the width or exactly one less, so the wrap is
  filling the region rather than stopping early
- **AND** at 58 the paragraph produces strictly more lines than the same document with each
  `日本語` replaced by three ASCII letters, because the wide form consumes twice the columns
- **AND** slicing every returned line's `text()` back out of the source at its own byte
  offsets succeeds, so no line ends in half a cluster

#### Scenario: Rendering is total over arbitrary input

- **WHEN** `lines` is called at widths 58 and 78 with each of: a lone `#`, an unclosed
  fence ```` ```sh ```` with no closing fence, an unterminated `[link](`, a line of 500
  `x` characters with no space, a string of only newlines, a string of only spaces, a
  document ending mid-emphasis `**bold`, a source holding a NUL character and a
  combining accent, a 500-column run of `日本語` with no space, a run of family emoji
  joined by zero-width joiners, **an unterminated `~~struck`**, **a header-only table with
  no delimiter row**, **a table whose body rows carry more cells than its header**, **a
  table whose body rows carry fewer cells than its header**, **a table of forty columns**,
  **a table whose single cell is a 500-column CJK run with no space**, **a task-list item
  whose text is a 500-character token with no space**, **a bare `- [x]` with no text after
  the marker**, and **a task-list item nested three levels deep inside block quotes**
- **THEN** none panics and each returns a `Vec` whose lines all respect the width in columns
- **AND** the 500-character token and the 500-column CJK run are each hard-split rather than
  emitted as one over-wide line
- **AND** the same calls at widths `1` and `2` also do not panic, and at width `1` a line
  holding only a two-column cluster is empty rather than two columns wide

### Requirement: Lists carry markers, numbering, and a hanging indent

A bullet item SHALL be marked `• ` and an ordered item `<n>. `, numbered from the list's own
start value rather than from 1. Nesting SHALL indent two columns per level. A wrapped item's
continuation lines SHALL be indented to the item's **text** column — the marker's width plus
its nesting indent — so a wrapped item reads as one item.

Items SHALL NOT be separated by a blank line, whether the source list is tight or loose;
exactly one blank line SHALL follow the list, per the block-separation rule.

#### Scenario: Bullet items carry their marker and wrap under their text column

- **WHEN** a list whose single item is `- alpha bravo charlie delta echo foxtrot golf hotel
  india juliett kilo lima mike november oscar papa` is rendered at 58 and at 78
- **THEN** at both widths the first line begins `• alpha` and every continuation line
  begins with exactly two spaces followed by a non-space
- **AND** at 58 the first line's `text()` is
  `• alpha bravo charlie delta echo foxtrot golf hotel india` — 57 display columns,
  because the
  marker costs the item two columns of text — and at 78 it is
  `• alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima`
- **AND** at both widths every segment carries `Face::plain()`, so the marker is not a face

#### Scenario: An ordered list numbers from its own start value

- **WHEN** a list written `7. seven` / `8. eight` / `9. nine` is rendered at 58 and at 78
- **THEN** at both widths the three lines' `text()` values are `7. seven`, `8. eight`, and
  `9. nine`, in that order
- **AND** at both widths a second list written `1. one` / `1. one again` renders as
  `1. one` and `2. one again`, so the numbering is sequential from the start value rather
  than a copy of the source digits

#### Scenario: A nested list indents two columns per level

- **WHEN** a list of `- outer`, then a nested `- inner`, then a doubly nested `- deepest`
  is rendered at 58 and at 78
- **THEN** at both widths the three lines' `text()` values are `• outer`, `  • inner`, and
  `    • deepest`
- **AND** at both widths a wrapped `• inner` item's continuation lines begin with exactly
  four spaces, so the hanging indent counts the nesting as well as the marker

### Requirement: A table renders as aligned columns sized to the region

With `Options::ENABLE_TABLES` on, a GitHub-flavoured pipe table SHALL be laid out as one
block of aligned columns rather than as literal source. It SHALL be separated from its
neighbouring blocks by exactly one blank `Line`, on the block-separation rule below, and it
SHALL carry no block prefix of its own.

Let `n` be the number of columns the table's **header row** declares, `w[j]` the columns
allocated to column `j`, and `total = 3n + 1 + sum(w)` — one `|` per column boundary plus a
leading and trailing one, and one padding space on each side of every cell. The **line
grammar** is:

- a **row line** is a leading `│`, then **for each column** the four parts ` `, the cell's
  content for that line laid out in exactly `w[j]` columns, ` `, and a `│` — so a row line
  holds `n + 1` separators in total and `│ a │ b │` is the two-column form;
- a **delimiter line**, emitted once, immediately after the header row's last line, is a row
  line's own shape with every padding space and every content column replaced by `─`, its
  leading separator by `├`, its trailing separator by `┤`, and every interior separator by
  `┼` — so it is exactly `total` columns and its junctions fall under the row lines'
  separators by construction rather than by a second arithmetic;
- no other line kind exists: there is no top rule, no bottom rule, and no per-row rule.

Every line the **pipe grammar** emits SHALL measure exactly `total` display columns, and
`total` SHALL be at most `width`. The narrow fallback below emits no pipe line at all and is
bound by `width` rather than by `total`; the two forms are stated separately for that reason. The separators, the padding spaces, the alignment padding, and the delimiter line
SHALL carry `Face::plain()`. A **header** cell's content SHALL carry `strong` true in
addition to whatever inline faces it contains, so the header row reads bold through the
existing `Strong` role and no new face or role is introduced for it.

**Column widths.** `nat[j]` is the greatest `layout::columns` of any cell in column `j`, at
least `1`. With `avail = width.saturating_sub(3n + 1)`:

- when `avail >= sum(nat)`, `w[j] = nat[j]` — a table that fits keeps its natural columns and
  the table is narrower than the region;
- otherwise the allocation SHALL be **max-min fair**: `w[j] = min(nat[j], c)` for the largest
  integer `c` with `sum(min(nat[j], c)) <= avail`, and the `avail - sum(w)` columns left over
  SHALL be handed out one at a time, by ascending column index, to columns still short of
  their natural width. Every `w[j]` SHALL be at least `1`.

The rule's consequence is the one that matters and is stated rather than derived: a table
that does not fit spends its columns on the **narrow** columns first, so a table of one prose
column beside three short ones does not squeeze the short ones to nothing.

**A cell is never truncated.** A cell whose content exceeds `w[j]` SHALL **wrap** within its
own column by the prose rule above, its continuation lines laid out in the same column
position with the other columns blank on those lines. A row's line count is therefore the
greatest line count of any of its cells, at least one, and the rendered line count and the
source row count legitimately diverge. Truncation is right where the grammar is fixed and the
name alone identifies the thing — `change-rows` truncates for exactly that reason — and wrong
here, where the detail region has no horizontal scroll to recover what a cut discards.

**Alignment.** A column the delimiter row marks `:---` or leaves unmarked SHALL be padded on
the right, `---:` on the left, and `:--:` on both with the odd column going to the right. The
alignment applies to every line of a wrapped cell, not to its first alone.

**Ragged rows are the parser's to normalise, not this module's.** Measured against
pulldown-cmark 0.13.4 with `ENABLE_TABLES`, a body row with fewer cells than the delimiter row
declares arrives already padded with empty cells, and one with more arrives already truncated
— `fold` never observes a ragged row. This module SHALL therefore add no padding or dropping
logic of its own, and the requirement states the property as a **regression guard on the
parser**: every emitted row line holds `n + 1` pipes whatever the source row's cell count.
Should a future parser version stop normalising, that guard is what fails.

`emit_table` SHALL nonetheless be **total** for a table carrying no columns, returning no line
rather than dividing by zero. That is a `fold`-level invariant rather than an observable
rendering: no `&str` source produces `Tag::Table([])` — `||` and `|-|` both yield one column —
so it is stated as a guard and deliberately given no scenario of its own.

**A table inside a container.** A table nested in a block quote or a list item SHALL lay its
own columns out in the columns its container leaves — `width` less the container's prefix, on
exactly the rule every other block follows — and every emitted line SHALL carry that prefix,
continuation lines included. `total` is then measured against the reduced width, not the
region's, so the "at most `width`" promise holds through a container. When the prefix leaves
too few columns for the pipe grammar, the fallback below applies inside the container.

**When the pipe grammar does not fit.** When `avail < n` — equivalently when
`width < 4n + 1`, one content column per column being the least the grammar can carry — the
table SHALL instead render **one cell per line**, in row-major order, each wrapped to the
full `width` by the prose rule, header cells still carrying `strong`, and an empty cell
emitting nothing. No separator, no padding, and no delimiter line is emitted in this form. It
preserves every character and never exceeds the region, which is what a table in a
three-column region can still be asked for.

#### Scenario: A table that fits renders as aligned columns at both mandated widths

- **WHEN** the table `| Gate | Runner |` / `|---|---|` / `| Format | cargo fmt |` /
  `| Lint | cargo clippy |` is rendered at 58 and at 78
- **THEN** at both widths exactly four non-blank lines are produced, whose `text()` values are
  exactly

  ```text
  │ Gate   │ Runner       │
  ├────────┼──────────────┤
  │ Format │ cargo fmt    │
  │ Lint   │ cargo clippy │
  ```

  each measuring 25 columns. The literals are the allocation rule's own output and are
  derived, not chosen: `nat = [6, 12]` (`Format` and `cargo clippy` are the widest cells),
  the table fits at both widths so `w = nat`, and `total = 3n + 1 + sum(w) = 7 + 18 = 25`
- **AND** at both widths every line's `layout::columns` is equal to every other's — which is
  the assertion that the box-drawing glyphs cost the arithmetic nothing, since a delimiter
  line built from `─` must measure exactly what a row line built from spaces does — and each
  is strictly less than the region width, so the table is sized to its content and not
  stretched
- **AND** at both widths the two header cells' segments carry `strong` true while every
  separator, padding space, and delimiter-line segment carries `Face::plain()`
- **AND** at both widths the delimiter line's `├`, `┼`, and `┤` characters fall at the same
  column offsets as the row lines' `│`, which is the assertion that the columns are actually
  aligned

#### Scenario: A cell wider than its column wraps rather than being truncated

- **WHEN** a two-column table whose header is `| key | value |`, whose delimiter row is
  `|---|---|`, and whose one body row holds `k` and
  `alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima mike november
  oscar papa` is rendered at 58 and at 78
- **THEN** at both widths that row occupies more than one line, and the concatenation of that
  column's cell content across those lines, with single spaces restored at the wrap points,
  equals the cell's source text — every word present, none cut mid-token except by the
  hard-split rule
- **AND** at both widths the first column's cell appears on the row's first line and its
  continuation lines carry `w[0]` spaces in that column instead, so continuations align under
  the wrapped cell rather than under the row
- **AND** at both widths every line of the row measures the same `total` and no line exceeds
  the region

#### Scenario: A table too wide for the region spends its columns on the narrow ones

- **WHEN** a four-column table whose natural widths are 4, 4, 4, and 60 is rendered at 58 and
  at 78
- **THEN** at 58 the three short columns keep their natural width of 4 and the wide one takes
  what is left, and at 78 the same holds with a wider fourth column, so the max-min rule is
  observed at both widths rather than a proportional squeeze
- **AND** at both widths every allocated width is at least 1 and every line measures exactly
  `3n + 1 + sum(w)`, which is at most the region width
- **AND** at both widths no cell's text is missing from the concatenated rendering, so
  nothing was truncated to make it fit

#### Scenario: Alignment markers pad the cell on the side they name

- **WHEN** a three-column table whose delimiter row is `|:---|:--:|---:|`, whose header cells
  are `left`, `cent`, and `rght` — four columns each, which is what makes `nat[j] = 4` and so
  what makes the padding observable at all — and whose one body row holds `x`, `x`, and `x`
  is rendered at 58 and at 78
- **THEN** at both widths `w = [4, 4, 4]`, and the body row's left-aligned cell renders
  `x   `, the centred one ` x  ` (the odd column going to the right), and the right-aligned
  one `   x`, between their padding spaces
- **AND** the assertion discriminates: the three cells' rendered text differs from one
  another, which a one-column-wide fixture could not show and which fails against an
  implementation that ignores `Alignment` entirely
- **AND** at both widths a wrapped cell in each column is padded on the same side on **every**
  one of its lines, not on its first alone

#### Scenario: A ragged table renders every declared column and drops no header column

The scenario is a **regression guard on pulldown-cmark's own normalisation**, not on logic
this module adds: measured against 0.13.4, a short row arrives already padded with empty cells
and a long one already truncated, so `fold` never sees a ragged row. It is written to fail if
a future parser version stops doing that.

- **WHEN** a three-column table with one body row of two cells and one body row of five cells
  is rendered at 58 and at 78
- **THEN** at both widths every emitted line measures the same `total` and holds exactly four
  separators — `│` on a row line, `├`/`┼`/`┤` on the delimiter line — so neither short nor
  long row deformed the table
- **AND** at both widths the short row's third column is `w[2]` spaces and the long row's
  fourth and fifth cells appear nowhere in the rendering
- **AND** the event stream the parser produced for that source is asserted directly — three
  `TableCell` events per body row — so the scenario says which component holds the property
  rather than crediting it to `emit_table`

#### Scenario: A region too narrow for the pipe grammar renders one cell per line

- **WHEN** the two-column `| Gate | Runner |` table is rendered at every width from 0 through
  10 — `4n + 1` being 9 for this table, so the threshold falls inside the sweep — and, for
  contrast, at 58 and at 78
- **THEN** at widths 1 through 8 no line holds a `│` character, each non-empty cell's text
  appears on its own line or wrapped across its own lines, and every line measures at most the
  width
- **AND** at width 9 and above the pipe grammar is used, so the threshold is exact rather than
  approximate, and at 58 and 78 the aligned form is produced
- **AND** at width 0 no line is produced at all, and no width panics

### Requirement: Block quotes, thematic breaks, and constructs the parser does not model

A block quote's every line, continuations included, SHALL be prefixed `│ ` and carry
`quoted` true, with its content wrapped to `width - 2`. A nested quote SHALL be prefixed
`│ │ `.

A thematic break SHALL be a line of `─` (U+2500) repeated to fill the width exactly,
carrying `Face::plain()`.

This **reverses** `markdown-viewer`'s rule that the break use the same ASCII character
`change-rows`' section headers use for their fold glyphs "rather than a box-drawing
character". The reversal is deliberate and its cost is named rather than discovered: `│`,
`─`, `├`, `┼`, `┤`, and `•` are East Asian **Ambiguous**, the class `SPEC.md` already
records as painted at two columns by a CJK-locale terminal where `unicode-width` — and so
`layout::columns`, and so every width computation in this crate — says one. This change
widens that standing, uncompensated exposure from an artifact's own content to the pane's
own chrome, and adds no compensation, for the reason `SPEC.md` already gives: any
compensation is a guess that breaks the terminal it guessed wrong for. `change-rows`'
fold glyphs are **not** changed and stay ASCII.

The parser SHALL be configured with `pulldown_cmark::Options::ENABLE_TABLES |
pulldown_cmark::Options::ENABLE_STRIKETHROUGH | pulldown_cmark::Options::ENABLE_TASKLISTS`,
and with **no other option**.

**A task-list item is modelled.** `Event::TaskListMarker(checked)` SHALL render as the
three-character glyph `[✓]` when `checked` and `[ ]` when it is not, followed by one space,
in place of the list item's own bullet marker — so a task-list item's prefix costs four
columns, two more than a `• ` item's two. Its continuation lines SHALL hang under its **own**
text column, four columns in, by the list rule above: the hanging indent is derived from the
marker actually rendered, never from the bullet marker the item would have carried. `✓` (U+2713) is East Asian **Neutral** and measures one
column, unlike the box-drawing glyphs above; it carries no width caveat.

Turning the option on **without** this render path is what the change may not do:
`TaskListMarker` would fall into `fold`'s `_ => {}` wildcard and the checkbox would
**vanish** from the rendered line rather than degrade to literal text, and no gate in this
repository can see that. The option and its arm SHALL therefore land together.

This **reverses** `markdown-viewer`'s measured decision that task-list items stay
unmodelled — "a second checkbox renderer beside `ui::tasks`' own, for a case that does not
occur, is how two renderers drift apart". The measurement it rested on still holds: task-list
items outside `tasks.md` occur nowhere in the corpus this pane renders. The drift risk it
names is answered structurally instead, by `tasks-checklist` moving to the **same** `[✓]`
glyph in this change, so the two renderers agree by construction and a future divergence is
a test failure rather than a silent inconsistency. `design.md` records the argument.

**Footnotes remain unmodelled.** `ENABLE_FOOTNOTES` SHALL stay off, and such source SHALL
continue to render as its literal text rather than being dropped or mangled. This is the
narrowed degraded state named in `SPEC.md`'s degraded-states table, whose row this change
rewords from "a footnote, a task-list item" to "a footnote" and whose
`tests/degraded-coverage.toml` proof this change re-points at the narrowed claim.

#### Scenario: A block quote prefixes every one of its lines

- **WHEN** a quote whose text is `alpha bravo charlie delta echo foxtrot golf hotel india
  juliett kilo lima mike november oscar papa` is rendered at 58 and at 78
- **THEN** at both widths every produced line's `text()` begins `│ ` and carries `quoted`
  true on every segment
- **AND** at 58 the quote wraps at 56 columns of content rather than 58, so its first line
  is `│ alpha bravo charlie delta echo foxtrot golf hotel india` — 57 display columns — and
  no line is wider than the interior
- **AND** at both widths a nested quote's lines begin `│ │ `

#### Scenario: A thematic break fills the interior at both widths

- **WHEN** a document holding `---` between two paragraphs is rendered at 58 and at 78
- **THEN** at 58 the break's `text()` is 58 `─` characters and at 78 it is 78 of them, and
  its `layout::columns` is 58 and 78 respectively, so the glyph costs exactly one column
- **AND** at both widths that line carries `Face::plain()`, and a blank line separates it
  from the paragraph above and the paragraph below

#### Scenario: A table renders as its literal source text, one row per line

The scenario's name is kept verbatim from `markdown-viewer` because a delta's scenario
headers are its merge key and OpenSpec has no scenario-level rename — the same reason
`detail-scroll` keeps two names whose subjects moved. Its subject narrows again, to the
**one** construct that remains unmodelled, and the two that have left the set — the table it
is named for, and now the task-list item — appear in it as the **discriminating controls**:
the scenario fails if either still renders literally.

- **WHEN** the source `See it here[^1].` / `` / `[^1]: The note.` is rendered at 58 and at 78
- **THEN** at both widths its non-blank rendered lines' `text()` values equal its own source
  lines verbatim, in order, so nothing is dropped and nothing is reinterpreted
- **AND** at both widths every segment of those lines carries `Face::plain()`, since
  `ENABLE_FOOTNOTES` is off and the lines are ordinary paragraph text — in particular no
  segment carries `strikethrough`, which discriminates this requirement from the
  strikethrough requirement below
- **AND** at both widths the source `- [ ] an item` / `- [x] a done item` renders as
  `[ ] an item` and `[✓] a done item` — the checkbox glyph in place of the bullet marker,
  the literal `[ ]`/`[x]` text gone — which is what makes this scenario fail if the
  task-list narrowing is not real
- **AND** a task-list item long enough to wrap at 58 — `- [ ] alpha bravo charlie delta echo
  foxtrot golf hotel india juliett kilo lima mike november oscar papa` — produces
  continuation lines beginning with exactly **four** spaces followed by a non-space, at 58
  and at 78. This clause exists because the hanging indent is computed from the marker's own
  width at item-open time, before the checkbox marker is known; an implementation that sets
  the first line's prefix and not the continuation's leaves them indented two columns, and no
  single-line fixture and no width sweep can see it — a shorter indent never overruns
- **AND** the same task-list source dispatched through `ui::tasks::lines` — what
  `ui::detail::content_lines` reaches for a tracked-tasks tab — renders the checklist grammar
  instead, and its item glyph is the **same** `[✓]`, so the two renderers are asserted to
  agree rather than assumed to
- **AND** a GFM table and a `~~struck~~` span in the same fixture render as aligned columns
  and as a struck face rather than as literal text, which is what makes this scenario fail if
  the table narrowing is not real

### Requirement: Blocks are separated by one blank line and source line structure is preserved

The requirement's name is kept verbatim because a delta's requirement headers are its merge
key; its second half — the block-separation rule — is unchanged, and its first half is
reversed.

A **soft break** — a newline inside a paragraph — SHALL be folded into a single space, and
the paragraph SHALL be wrapped as one unit at the width available to it. A **hard break** —
a line ending in two spaces or a backslash — SHALL still start a new rendered line.

This **reverses** `markdown-viewer`'s rule that a soft break starts a new line, whose
argument was that "re-flowing a paragraph would destroy exactly the shapes this pane exists
to read: an unmodelled table's rows, and prose an author hard-wrapped on purpose". Both
halves of that argument are answered rather than ignored. An unmodelled table's rows are no
longer at stake: pipe tables have been modelled since `markdown-viewer` itself turned
`ENABLE_TABLES` on, and a table's rows now arrive as `TableRow` events, not as soft-broken
paragraph text. Prose hard-wrapped on purpose keeps an opt-out that is **explicit** rather
than incidental — the hard break — which is what distinguishes a line the author meant to end
from a line their editor's fill column ended. The incidental case is the one that was
producing the damage: an artifact wrapped at one width, rendered in a region of another,
alternates full-width and orphan lines for its whole length.

Folding SHALL apply wherever the prose rule applies — a paragraph, a list item, a block
quote, a heading, and a table cell — and SHALL NOT apply to a verbatim block, whose lines
are already reproduced verbatim and hard-split rather than reflowed.

Exactly one blank `Line` — one with no segments — SHALL separate two adjacent blocks. There
SHALL be no blank line before the first block and none after the last, so a document never
opens or closes with dead rows.

#### Scenario: A soft break starts a new line rather than being folded

The scenario's name is kept verbatim because a delta's scenario headers are its merge key
and OpenSpec has no scenario-level rename. Its assertions are the inverse of the ones it
carried, which is the point: this scenario failing against the old implementation is what
proves the reversal landed.

- **WHEN** a paragraph written as `first line` / `second line` on two source lines is
  rendered at 58 and at 78 — widths at which both fit on one line together
- **THEN** at both widths exactly one line is produced and its `text()` is
  `first line second line`, with exactly one space at the join and no trailing space
- **AND** at both widths the same paragraph written with `first line  ` — two trailing
  spaces, a **hard** break — produces two lines, `first line` and `second line`, so the
  author's explicit break survives while the incidental one does not
- **AND** a paragraph of sixty words written one word per source line, rendered at 58 and at
  78, produces lines that are each within one word of the width rather than sixty lines of
  one word, and produces **fewer** lines at 78 than at 58 — the assertion that the reflow is
  width-driven rather than a fixed regrouping
- **AND** a fenced code block whose lines are `alpha` and `bravo` still produces two lines at
  both widths, so folding did not leak into the verbatim path

#### Scenario: Exactly one blank line separates blocks, with none leading or trailing

- **WHEN** a document of `# One`, a paragraph, a bullet list, a fenced code block, and a
  closing paragraph — with two blank source lines between two of them — is rendered at 58
  and at 78
- **THEN** at both widths the first returned line is the heading and the last is the
  closing paragraph's last line, neither of them blank
- **AND** at both widths no two consecutive returned lines are both blank, and exactly one
  blank line sits between each adjacent pair of blocks


## ADDED Requirements

### Requirement: Every glyph the renderer emits measures one display column

Every non-space character `ui::markdown` emits that is not drawn from the source document —
the bullet `•`, the quote prefix `│`, the thematic break `─`, the table separators `│`, `├`,
`┼`, `┤`, and the checkbox glyph `✓` — SHALL measure exactly one column under
`layout::columns`, which is `unicode-width`'s measure and therefore the one
`ratatui::buffer::Buffer::set_string` itself consumes. The renderer's width arithmetic SHALL
treat each as one column and SHALL NOT special-case any of them.

This is asserted rather than assumed because it is the single property the whole glyph change
rests on: were any of them two columns under that measure, every line grammar built on it
would overrun its region silently. It is **not** a claim about what a terminal paints — six of
the eight are East Asian Ambiguous and a CJK-locale terminal paints them at two columns, which
this capability accepts uncompensated on `SPEC.md`'s standing rule and states in the
thematic-break requirement above.

#### Scenario: Each emitted glyph measures one column, and the set is complete

- **WHEN** each of `•`, `│`, `─`, `├`, `┼`, `┤`, and `✓` is passed to `layout::columns`
- **THEN** each returns exactly `1`
- **AND** rendering a document holding a bullet list, a nested bullet list, a block quote, a
  nested block quote, a thematic break, a two-column table, and a task-list item at 58 and at
  78 produces no line whose `layout::columns` exceeds the width
- **AND** at both widths the set of non-space characters appearing in that rendering but not
  in its source is exactly those seven glyphs, so a glyph added later without a width
  assertion fails this scenario rather than passing unnoticed
