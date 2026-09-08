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
  whose widest cell is wider than the narrow interior**, and **a struck run** is rendered
  at widths 0, 1, 2, 3, 10, 58, 78, and 200
- **THEN** at every width every returned line's `text()` has at most that many **display
  columns**, measured by `layout::columns`
- **AND** the rendering at 58 and at 78 both hold at least one line of each block kind, the
  table's and the struck run's included, so the assertion is made against real content
  rather than an empty result
- **AND** the two widths at which the table cannot hold its pipe grammar — where the
  region is narrower than `4n + 1` columns for the table's `n` columns — are among those
  swept, so the degenerate path is measured rather than assumed

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
  and **a table whose single cell is a 500-column CJK run with no space**
- **THEN** none panics and each returns a `Vec` whose lines all respect the width in columns
- **AND** the 500-character token and the 500-column CJK run are each hard-split rather than
  emitted as one over-wide line
- **AND** the same calls at widths `1` and `2` also do not panic, and at width `1` a line
  holding only a two-column cluster is empty rather than two columns wide

### Requirement: Block quotes, thematic breaks, and constructs the parser does not model

A block quote's every line, continuations included, SHALL be prefixed `> ` and carry
`quoted` true, with its content wrapped to `width - 2`. A nested quote SHALL be prefixed
`> > `.

A thematic break SHALL be a line of `-` repeated to fill the width exactly, carrying
`Face::plain()` — the same ASCII rule `change-rows`' archived separator uses, rather than a
box-drawing character.

The parser SHALL be configured with
`pulldown_cmark::Options::ENABLE_TABLES | pulldown_cmark::Options::ENABLE_STRIKETHROUGH`,
and with **no other option**. `Options::empty()` is no longer the configuration: tables are
what this repository writes its `SHALL`-exact contracts in, and rendering them as pipe-and-dash
source in the one region meant for reading prose was a scope bound of `markdown-viewer`, not a
lasting decision.

**Footnotes and task-list items remain unmodelled.** `ENABLE_FOOTNOTES` and
`ENABLE_TASKLISTS` SHALL stay off, and such source SHALL continue to render as its literal
text — one line per source line, by the soft-break rule below — rather than being dropped or
mangled. This is the narrowed degraded state named in `SPEC.md`'s degraded-states table,
whose row this change rewords from "a table, a footnote, strikethrough, a task-list item" to
"a footnote, a task-list item" and whose `tests/degraded-coverage.toml` proof this change
re-points at the narrowed claim.

The reason the row narrows rather than closing is measured, not stylistic: across the 238
`.md` files this pane renders, footnote definitions appear zero times and task-list items
outside `tasks.md` appear zero times, while pipe tables appear in 89 of them. A second
checkbox renderer beside `ui::tasks`' own, for a case that does not occur, is how two
renderers drift apart.

#### Scenario: A block quote prefixes every one of its lines

- **WHEN** a quote whose text is `alpha bravo charlie delta echo foxtrot golf hotel india
  juliett kilo lima mike november oscar papa` is rendered at 58 and at 78
- **THEN** at both widths every produced line's `text()` begins `> ` and carries `quoted`
  true on every segment
- **AND** at 58 the quote wraps at 56 columns of content rather than 58, so its first line
  is `> alpha bravo charlie delta echo foxtrot golf hotel india` — 57 characters — and no
  line is wider than the interior
- **AND** at both widths a nested quote's lines begin `> > `

#### Scenario: A thematic break fills the interior at both widths

- **WHEN** a document holding `---` between two paragraphs is rendered at 58 and at 78
- **THEN** at 58 the break's `text()` is 58 `-` characters and at 78 it is 78 of them
- **AND** at both widths that line carries `Face::plain()`, and a blank line separates it
  from the paragraph above and the paragraph below

#### Scenario: A footnote and a task-list item still render as their literal source text

The scenario replaces `markdown-viewer`'s "A table renders as its literal source text, one
row per line", whose subject has left the unmodelled set; its own subject is the two
constructs that remain in it.

- **WHEN** the two sources `See it here[^1].` / `` / `[^1]: The note.` and `- [ ] an item` /
  `- [x] a done item` are each rendered at 58 and at 78
- **THEN** at both widths each source's non-blank rendered lines' `text()` values equal its
  own source lines verbatim, in order, so nothing is folded into its neighbour and nothing is
  dropped
- **AND** at both widths every segment of those lines carries `Face::plain()`, since neither
  extension is on and the lines are ordinary paragraph text — in particular no segment
  carries `strikethrough`, which discriminates this requirement from the strikethrough
  requirement below
- **AND** the same task-list source dispatched through `ui::tasks::lines` — what
  `ui::detail::content_lines` reaches for a tracked-tasks tab — renders the checklist grammar
  instead, so the row's own "on a tab **other** than the tracked-tasks one" carve-out is
  observed rather than asserted

## ADDED Requirements

### Requirement: A table renders as aligned columns sized to the region

With `Options::ENABLE_TABLES` on, a GitHub-flavoured pipe table SHALL be laid out as one
block of aligned columns rather than as literal source. It SHALL be separated from its
neighbouring blocks by exactly one blank `Line`, on the block-separation rule below, and it
SHALL carry no block prefix of its own.

Let `n` be the number of columns the table's **header row** declares, `w[j]` the columns
allocated to column `j`, and `total = 3n + 1 + sum(w)` — one `|` per column boundary plus a
leading and trailing one, and one padding space on each side of every cell. The **line
grammar** is:

- a **row line** is `|`, then for each column ` `, the cell's content for that line laid out
  in exactly `w[j]` columns, ` `, and a closing `|`;
- a **delimiter line**, emitted once, immediately after the header row's last line, is `|`,
  then for each column `-` repeated `w[j] + 2` times, then `|` — so it is exactly `total`
  columns and its pipes fall under the row lines' pipes;
- no other line kind exists: there is no top rule, no bottom rule, and no per-row rule.

Every line a table emits SHALL measure exactly `total` display columns, and `total` SHALL be
at most `width`. The pipes, the padding spaces, the alignment padding, and the delimiter line
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

**Ragged rows.** A body row with fewer cells than the header SHALL render its missing columns
as `w[j]` spaces. A body row with more cells than the header SHALL drop the surplus cells:
`n` is the header's, the delimiter line's shape is fixed by it, and a row wider than the
table has no column to be drawn in. A table whose header declares no columns at all SHALL
emit nothing.

**When the pipe grammar does not fit.** When `avail < n` — equivalently when
`width < 4n + 1`, one content column per column being the least the grammar can carry — the
table SHALL instead render **one cell per line**, in row-major order, each wrapped to the
full `width` by the prose rule, header cells still carrying `strong`, and an empty cell
emitting nothing. No pipe, no padding, and no delimiter line is emitted in this form. It
preserves every character and never exceeds the region, which is what a table in a
three-column region can still be asked for.

#### Scenario: A table that fits renders as aligned columns at both mandated widths

- **WHEN** the table `| Gate | Runner |` / `|---|---|` / `| Format | cargo fmt |` /
  `| Lint | cargo clippy |` is rendered at 58 and at 78
- **THEN** at both widths exactly four non-blank lines are produced, whose `text()` values
  are `| Gate | Runner        |`, `|------|---------------|`, `| Format | cargo fmt |`
  shaped to the same column widths, and the lint row likewise — every line measuring the same
  `total`, and `total` being the natural widths plus `3n + 1` rather than the region width
- **AND** at both widths every line's `layout::columns` is equal to every other's, and each
  is strictly less than the region width, so the table is sized to its content and not
  stretched
- **AND** at both widths the two header cells' segments carry `strong` true while every pipe,
  padding space, and delimiter-line segment carries `Face::plain()`
- **AND** at both widths the delimiter line's `|` characters fall at the same column offsets
  as the row lines', which is the assertion that the columns are actually aligned

#### Scenario: A cell wider than its column wraps rather than being truncated

- **WHEN** a two-column table whose second column holds
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
  are `l`, `c`, and `r`, and whose one body row holds `x`, `x`, and `x` in columns four
  columns wide is rendered at 58 and at 78
- **THEN** at both widths the left-aligned cell renders `x   `, the centred one ` x  `, and
  the right-aligned one `   x`, between their padding spaces
- **AND** at both widths a wrapped cell in each column is padded on the same side on **every**
  one of its lines, not on its first alone

#### Scenario: A ragged table renders every declared column and drops no header column

- **WHEN** a three-column table with one body row of two cells and one body row of five cells
  is rendered at 58 and at 78
- **THEN** at both widths every emitted line measures the same `total` and holds exactly four
  `|` characters, so neither short nor long row deformed the table
- **AND** at both widths the short row's third column is `w[2]` spaces and the long row's
  fourth and fifth cells appear nowhere in the rendering
- **AND** a table whose header declares no columns emits no line at all at either width, and
  neither case panics

#### Scenario: A region too narrow for the pipe grammar renders one cell per line

- **WHEN** the two-column `| Gate | Runner |` table is rendered at widths 0, 1, 2, 3, 4, 8,
  and 9 — `4n + 1` being 9 for this table — and, for contrast, at 58 and at 78
- **THEN** at widths 1 through 8 no line holds a `|` character, each non-empty cell's text
  appears on its own line or wrapped across its own lines, and every line measures at most the
  width
- **AND** at width 9 and above the pipe grammar is used, so the threshold is exact rather than
  approximate, and at 58 and 78 the aligned form is produced
- **AND** at width 0 no line is produced at all, and no width panics

### Requirement: Strikethrough sets a face on its text

With `Options::ENABLE_STRIKETHROUGH` on, `~~x~~` SHALL set `strikethrough` on the enclosed
text's segments. It SHALL compose with the other inline faces rather than replacing them, on
the same terms `strong` and `emphasis` already compose, and SHALL survive a wrap: where a
struck run is split across a line boundary, **both** resulting lines' segments carry it.

It is specified and implemented at **zero measured occurrences** in this repository's own
corpus, and that is deliberate rather than an oversight: the pane renders whatever repository
it is pointed at, and the whole cost is one `Options` flag, one `bool`, and one palette role.
`view-palette` states which `Style` the face maps to.

#### Scenario: A struck run carries the face and composes with the others

- **WHEN** the paragraph `A ~~struck~~ word, ~~**struck bold**~~, and `~~x~~` in code.` is
  rendered at 58 and at 78 — short enough to fit on one line at both, so a wrap cannot split
  the phrases
- **THEN** at both widths a segment whose `text` is `struck` carries `strikethrough` true and
  every other flag false
- **AND** at both widths a segment whose `text` is `struck bold` carries `strikethrough`
  **and** `strong` true together
- **AND** at both widths the `~~x~~` inside the code span carries `code` true and
  `strikethrough` **false**, because a code span's content is verbatim, and the `~` characters
  are present in its text
- **AND** at both widths no segment's `text` contains a `~` outside that code span, so the
  markers are consumed rather than rendered

#### Scenario: A struck run split across a wrap keeps its face on both lines

- **WHEN** a paragraph whose entire text is struck — `~~alpha bravo charlie delta echo
  foxtrot golf hotel india juliett kilo lima mike november oscar papa~~` — is rendered at 58
  and at 78
- **THEN** at both widths the result is two lines and every segment of **both** carries
  `strikethrough` true
- **AND** at both widths the two lines' `text()` values are the same two strings the
  paragraph-wrap scenario names, so the face costs no columns
