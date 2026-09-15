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
    pub muted: bool,                            // de-emphasised whole
    pub label: Option<crate::tasks::LabelRole>, // a task's leading label
    pub delta: Option<crate::specs::DeltaOp>,   // a delta badge's marker
}
pub struct Segment { pub text: String, pub face: Face }
pub struct Line { pub segments: Vec<Segment> }
pub fn lines(source: &str, width: u16) -> Vec<Line>;
```

`delta` is `spec-emphasis`' one new field, joining `muted` and `label` from `tasks-emphasis`
and `strikethrough` from `markdown-legibility`, all three added on the same terms. `Face`
SHALL keep deriving `Default` — it is a value with a meaningful zero, not a state type
`NODEFAULT-UI` gates — so `Face::plain()` SHALL remain the all-`false`, `heading: None`,
`label: None`, `delta: None` value. Every construction site
that spells the fields out rather than writing `..Face::plain()` SHALL name the new field:
`src/ui/tasks.rs`'s `heading_line` is the one such site in the crate, and it is a
**compile-time** forcing site, which is why the fields are added to the struct rather than
tracked in a parallel enum.

`ui::markdown` SHALL return `muted: false` on every segment it emits, for every source and
every width. It SHALL set `label` on exactly one construct — a clause keyword, per "A scenario
clause's keyword carries its lifecycle role" below — and `None` on every other segment.
`spec-emphasis` is what changed this sentence: `tasks-checklist` was `label`'s only writer when
the field was added, and a second writer on the markdown path is the whole of that change's
rendered half. The `muted` and `label` fields
exist because `Face` is the crate's one carrier of "what this run of text is", and
`tasks-checklist` needs to say two things about a run that no markdown construct says —
that a whole row is finished, and that a leading token is a lifecycle label. Putting them
here rather than inventing a second segment type is what keeps `ui::detail::content_lines`
returning one line type whether its body came from the markdown path or the checklist path.

`label`'s type is `crate::tasks::LabelRole`, which is **not** a view type and is **not** a
`ratatui` type, so it widens neither the `MDSEAM` confinement — `pulldown_cmark` stays named
only here — nor the "names no `ratatui` type" rule above. `ui::markdown` SHALL NOT call
`tasks::label_of` or any other function of `crate::tasks`; it names the type and nothing
else. `NOIO-VIEW` is unaffected for the same reason: `LabelRole` is a plain enum and reaches
no I/O API.

`spec-emphasis` adds `delta`, on exactly those terms: `crate::specs::DeltaOp` is a plain enum
naming no view type, no `ratatui` type, and no I/O API. It is set by `ui::detail`'s badge
segment and never by this module — no markdown construct is a delta operation — so `lines`
SHALL leave it `None` on every segment it returns.

The prohibition above is **narrowed, not lifted**: `ui::markdown` SHALL NOT call any function
of `crate::tasks`, and SHALL be permitted to call `crate::specs::clause_of` and no other
function of `crate::specs`. The two are not the same risk. `crate::tasks` holds `tasks::read`,
a filesystem edge that `NOIO-VIEW` names explicitly, so a call into that module is one
`use` away from a gate failure; `crate::specs` has no I/O at all, holds no such sibling, and
exists precisely so that this classification is reachable from a pure view file. The narrowing
is what makes the clause requirement below implementable without a second segment type.

Which `Style` each new field produces is `view-palette`'s, not this capability's, exactly as
it already is for `strikethrough`.

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
- **AND** every segment of every line carries `Face::plain()` — `muted: false` and
  `label: None` among its fields — and the result is byte-identical to the one this
  requirement produced before display-column measurement, because every character in it
  measures one column

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

#### Scenario: The markdown path sets neither new face field

- **WHEN** `ui::markdown::lines` is called at widths 58 and 78 over a document holding a
  heading, a paragraph, a bullet list, a fenced code block, a block quote, a link, a struck
  run, a table, a checked and an unchecked task-list item, and the literal paragraph
  `VERIFY: this is prose, not a task`
- **THEN** every segment of every returned line carries `muted: false`, `label: None`, and
  `delta: None` — the document holds no `- **WHEN**` bullet, so no clause keyword is reached
- **AND** the `VERIFY:` paragraph in particular carries `label: None`, so recognising a
  *task* label is `tasks-checklist`'s job on the checklist path and never the markdown
  renderer's — a proposal that opens with the word `VERIFY:` is not styled as a task. This
  clause is what `spec-emphasis` narrowed: the markdown path now sets `label` on a clause
  keyword, and this scenario's document deliberately contains none, so the assertion survives
  the change unweakened rather than being deleted by it
- **AND** the result at both widths is byte-identical, segment for segment, to the same call
  before `tasks-emphasis`, so neither its two fields nor `spec-emphasis`' third moved any
  rendered output for a document carrying no clause

## ADDED Requirements

### Requirement: A scenario clause's keyword carries its lifecycle role

`ui::markdown::lines` SHALL set `Face::label` on a **strong** run that opens a list item and is
a clause keyword, so that a spec's `- **WHEN**` is coloured by the lifecycle position it names.

A run SHALL be a clause keyword when **all** of:

1. It is emitted by a `Strong` inline — the `**…**` the author already wrote. The renderer
   SHALL NOT invent emphasis where the source has none.
2. It is the **first** inline content of a list item, with nothing but the item's marker and
   its hanging indent before it. A bold run later in the clause SHALL NOT be a keyword, which
   is what keeps `- **WHEN** the **schema** declares four artifacts` styling one run and not
   two.
3. `crate::specs::clause_of` on the run's **trimmed** text returns `Some`.

The role SHALL then be:

- `Some(Clause::Opens(role))` — the run carries `Face { label: Some(role), .. }`, and `role`
  SHALL be remembered as the **current clause position**.
- `Some(Clause::Continues)` — the run carries the remembered position, so `- **AND**` is drawn
  in the colour of the `WHEN` or `THEN` above it. A continuation reached with no remembered
  position SHALL carry `LabelRole::Other` rather than no label at all, so a stray leading
  `- **AND**` degrades to the generic label colour and never panics.

The remembered position SHALL be **reset to none at every heading**, at any level. A heading is
the boundary between one scenario and the next, so an `AND` under a new `#### Scenario:` can
never inherit from the scenario above it. It SHALL NOT be reset by a paragraph, a blank line,
or a nested list, because a scenario's clauses are frequently separated by continuation lines.

The keyword's `strong` face SHALL be left **set**. The clause role is added beside the author's
bold, never in place of it: `view-palette` patches the label colour over `Role::Strong`'s
modifier, so a keyword is bold *and* coloured, and a monochrome reading of the frame is exactly
what it was before this change. This is the same "colour beside the modifier" rule
`view-palette` states for every other role.

This SHALL apply to **every** markdown source the renderer is given, with no spec-shape test.
`lines` is parameterised by text and width and knows nothing about which artifact it is
drawing; adding that knowledge would mean threading the tab's identity through a pure
function for no gain. The vocabulary is narrow enough that this is safe: a bold run opening a
list item that is exactly a lifecycle token is a scenario clause wherever it appears, and
`markdown-legibility`'s task-list items carry their marker before any strong run and so are
untouched.

#### Scenario: A scenario's three clauses are coloured by position

- **WHEN** `lines` renders, at the mandated 78-column detail interior and again at 58, the
  source `- **WHEN** the schema declares four artifacts\n- **THEN** the tab bar shows four\n- **AND** the first is active\n`
- **THEN** the three keyword segments carry `Face::label` values `Some(Change)`,
  `Some(Confirm)`, and `Some(Confirm)` respectively
- **AND** each of those segments also carries `strong: true`, so the colour was added beside
  the author's bold and not in place of it
- **AND** the clause text following each keyword carries `label: None`, so only the keyword is
  coloured

#### Scenario: `AND` inherits the clause above it and resets at a heading

- **WHEN** `lines` renders, at 78 columns and again at 58, a source holding
  `#### Scenario: a\n\n- **WHEN** x\n- **AND** y\n\n#### Scenario: b\n\n- **AND** z\n`
- **THEN** the first `AND` carries `Some(Change)`, inherited from the `WHEN` above it
- **AND** the second `AND` carries `Some(Other)`, the heading having reset the remembered
  position, so inheritance never crosses a scenario boundary

#### Scenario: Only a run opening a list item is a keyword

- **WHEN** `lines` renders, at 78 columns and again at 58, the sources `**WHEN** not in a list`,
  `- the **WHEN** clause is described`, `- **Note** this is prose`, and `- **when** lowercase`
- **THEN** no segment in any of the four carries a `Face::label` that is `Some`
- **AND** the bold runs still carry `strong: true`, so declining to classify changed nothing
  else about how they render

#### Scenario: Every segment `lines` returns carries no delta

- **WHEN** `lines` renders a source holding a heading, a paragraph, a table, a code block, a
  block quote, a task-list item, and a scenario clause, at 78 columns and again at 58
- **THEN** every segment of every returned line carries `delta: None`
- **AND** `Face::plain()` equals a `Face` with `delta: None`, so no markdown construct can set
  the badge field that only `ui::detail` writes

#### Scenario: The narrowed seam holds

- **WHEN** `src/ui/markdown.rs` is searched for `crate::tasks::` and for `crate::specs::`
- **THEN** no call to any function of `crate::tasks` occurs — the module names `LabelRole` and
  nothing else of it
- **AND** the only function of `crate::specs` it calls is `clause_of`
- **AND** `pulldown_cmark` is still named only in this file, and this file still names no
  `ratatui` type, so neither `MDSEAM` nor the view-type rule was widened
