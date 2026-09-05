# markdown-render Specification

## Purpose
TBD - created by archiving change markdown-viewer. Update Purpose after archive.

## Requirements

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
}
pub struct Segment { pub text: String, pub face: Face }
pub struct Line { pub segments: Vec<Segment> }
pub fn lines(source: &str, width: u16) -> Vec<Line>;
```

`Face::plain()` SHALL be the all-`false`, `heading: None` value. `Line::text()` SHALL
return the line's segments concatenated, which is what a width assertion reads. Mapping a
`Face` to a `ratatui::style::Style` is `ui::view`'s work and SHALL NOT happen here, on the
same terms as `ui::list` returning plain `String`s.

Every returned `Line`'s `text()` SHALL be at most `width` characters, counted in `char`s —
the same measure `ui::list` uses, and the same documented limitation for wide and combining
characters. A line SHALL NOT be padded to the width: a region draws segments left to right
and leaves the rest of the row untouched.

Wrapping SHALL be greedy at ASCII spaces, and a token that does not fit the width available
to it — even on a line of its own — SHALL be **hard-split** at exactly that many characters
and continued on the next line, so a URL or an unbroken identifier can never produce a line
wider than the region. This applies to prose as well as to code; the two differ only in
whether spaces are break opportunities.

A block that carries a prefix or an indent — a list marker, a nesting indent, a block
quote's `> ` — SHALL lay its content out in `width.saturating_sub(prefix)` columns. When
that leaves zero columns, the block emits its prefix truncated to the width and no content,
rather than underflowing or panicking.

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
  characters exactly, so the boundary is inclusive — and `november oscar papa`
- **AND** every segment of every line carries `Face::plain()`

#### Scenario: An empty source and a zero width each produce no lines

- **WHEN** `lines("", 58)`, `lines("", 78)`, `lines("# Proposal\n", 0)`, and
  `lines("", 0)` are called
- **THEN** each returns an empty `Vec`
- **AND** nothing panics, so a detail region that has been squeezed to zero columns is a
  no-op rather than a crash

#### Scenario: No line exceeds the width it was given

- **WHEN** a document holding a heading, a paragraph, a bullet list, a nested list, a
  fenced code block, a block quote, a thematic break, and a link is rendered at widths
  0, 1, 2, 3, 10, 58, 78, and 200
- **THEN** at every width every returned line's `text()` has at most that many characters
- **AND** the rendering at 58 and at 78 both hold at least one line of each block kind, so
  the assertion is made against real content rather than an empty result

#### Scenario: Rendering is total over arbitrary input

- **WHEN** `lines` is called at widths 58 and 78 with each of: a lone `#`, an unclosed
  fence ```` ```sh ```` with no closing fence, an unterminated `[link](`, a line of 500
  `x` characters with no space, a string of only newlines, a string of only spaces, a
  document ending mid-emphasis `**bold`, and a source holding a NUL character and a
  combining accent
- **THEN** none panics and each returns a `Vec` whose lines all respect the width
- **AND** the 500-character token is hard-split rather than emitted as one over-wide line

### Requirement: Headings carry their level and their marker

A heading SHALL be rendered as `#` repeated `level` times, a space, and the heading's text,
with the whole line's segments carrying `heading: Some(level)`. The level is carried in the
marker rather than in a colour, because this crate has no colour policy and an `H2` and an
`H3` styled identically are indistinguishable in a plain terminal.

A heading too long for the width SHALL wrap by the paragraph rule, with continuation lines
indented by `level + 1` spaces so the text of a wrapped heading stays in one column.

#### Scenario: The six heading levels render with their markers at both widths

- **WHEN** a document holding `# One`, `## Two`, `### Three`, `#### Four`, `##### Five`,
  and `###### Six` is rendered at 58 and at 78
- **THEN** at both widths the non-blank lines' `text()` values are `# One`, `## Two`,
  `### Three`, `#### Four`, `##### Five`, and `###### Six` in that order
- **AND** at both widths those six lines carry `heading: Some(1)` through
  `heading: Some(6)` respectively on every segment, and `strong`, `emphasis`, `code`,
  `link`, and `quoted` are all false

#### Scenario: A heading too long for the interior wraps under its own text column

- **WHEN** `## alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima
  mike november` is rendered at 58 and at 78
- **THEN** at both widths the first line begins `## alpha` and every continuation line
  begins with exactly three spaces followed by a non-space, so the wrapped text aligns
  under the first line's text rather than under its marker
- **AND** at both widths every one of those lines carries `heading: Some(2)`

### Requirement: Emphasis, strong, inline code, and links become faces on segments

Within a block, `*x*` SHALL set `emphasis`, `**x**` SHALL set `strong`, `` `x` `` SHALL set
`code`, and `[text](dest)` SHALL set `link` on the link's **text**. Nested constructs SHALL
compose: text inside both `**` and `*` carries `strong` and `emphasis` together, rather
than one replacing the other.

A link's destination SHALL NOT be rendered. A detail region 58 columns wide cannot afford a
URL beside every link, the pane cannot follow one, and dropping it keeps the wrapping rule
free of a second, differently-wrapping field.

An **image** — `![alt](dest)`, which CommonMark's core models and which this pane cannot
display — SHALL render its alt text with `link` set, and SHALL render the four characters
`[img]` with `link` set when the alt text is empty. It SHALL NOT be dropped: an image with
no alt text and no rendering would vanish from the pane with nothing to show that anything
was there.

Where a faced run is split across a wrap boundary, **both** resulting lines' segments SHALL
carry the face, so styling never falls off at a line break.

#### Scenario: Inline constructs become separate segments carrying their faces

- **WHEN** the paragraph ``This **is** a *very* nice `example()` call for
  [the design](design.md) doc.`` — short enough that the whole sentence fits on one
  line at both mandated widths, so a wrap boundary cannot split the link phrase — is
  rendered at 58 and at 78
- **THEN** at both widths there is a segment whose `text` is `is` with `strong` true and
  every other flag false, a segment whose `text` is `very` with `emphasis` true, a
  segment whose `text` is `example()` with `code` true, and a segment whose `text`
  is `the design` with `link` true
- **AND** at both widths no segment's `text` contains `design.md`, so the destination is
  not rendered
- **AND** at both widths the concatenated `text()` of all lines contains
  `This is a very nice example() call for the design doc.`

#### Scenario: Nested emphasis composes rather than replacing

- **WHEN** the paragraph `***both*** and [**bold link**](x)` is rendered at 58 and at 78
- **THEN** at both widths the segment whose `text` is `both` carries `strong` **and**
  `emphasis` true together
- **AND** at both widths the segment whose `text` is `bold link` carries `link` **and**
  `strong` true together

#### Scenario: An image renders its alt text rather than vanishing

- **WHEN** the paragraph `Before ![a diagram](x.png) and ![](y.png) after` is rendered at 58
  and at 78
- **THEN** at both widths a segment whose `text` is `a diagram` carries `link` true, and a
  segment whose `text` is `[img]` carries `link` true
- **AND** at both widths no segment's `text` contains `x.png` or `y.png`, and the
  concatenated text of all lines contains both `a diagram` and `[img]`, so neither image
  was dropped

#### Scenario: A faced run split across a wrap keeps its face on both lines

- **WHEN** a paragraph whose entire text is bold — `**alpha bravo charlie delta echo
  foxtrot golf hotel india juliett kilo lima mike november oscar papa**` — is rendered at
  58 and at 78
- **THEN** at both widths the result is two lines, and every segment of **both** lines
  carries `strong` true
- **AND** at both widths the two lines' `text()` values are the same two strings the
  paragraph-wrap scenario names, so the face costs no columns

### Requirement: Lists carry markers, numbering, and a hanging indent

A bullet item SHALL be marked `- ` and an ordered item `<n>. `, numbered from the list's own
start value rather than from 1. Nesting SHALL indent two columns per level. A wrapped item's
continuation lines SHALL be indented to the item's **text** column — the marker's width plus
its nesting indent — so a wrapped item reads as one item.

Items SHALL NOT be separated by a blank line, whether the source list is tight or loose;
exactly one blank line SHALL follow the list, per the block-separation rule.

#### Scenario: Bullet items carry their marker and wrap under their text column

- **WHEN** a list whose single item is `- alpha bravo charlie delta echo foxtrot golf hotel
  india juliett kilo lima mike november oscar papa` is rendered at 58 and at 78
- **THEN** at both widths the first line begins `- alpha` and every continuation line
  begins with exactly two spaces followed by a non-space
- **AND** at 58 the first line's `text()` is
  `- alpha bravo charlie delta echo foxtrot golf hotel india` — 57 characters, because the
  marker costs the item two columns of text — and at 78 it is
  `- alpha bravo charlie delta echo foxtrot golf hotel india juliett kilo lima`
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
- **THEN** at both widths the three lines' `text()` values are `- outer`, `  - inner`, and
  `    - deepest`
- **AND** at both widths a wrapped `- inner` item's continuation lines begin with exactly
  four spaces, so the hanging indent counts the nesting as well as the marker

### Requirement: Code blocks and raw HTML are reproduced verbatim and hard-split

Every line of a fenced or indented code block SHALL be reproduced **verbatim**, as a single
segment with `code` true, with no word wrapping, no re-indenting, and no rendering of the
fence lines or the info string. A code line longer than the width SHALL be **hard-split** at
exactly the width and continued on the next line, rather than word-wrapped or clipped:
losing the tail of a command in a spec-reading pane is worse than a ragged split, and a
split preserves every character.

A raw HTML block or inline HTML SHALL be rendered the same way — verbatim, with `code` true,
hard-split — rather than dropped, so nothing in an artifact disappears from the pane.

#### Scenario: A fenced code block's lines are reproduced verbatim

- **WHEN** a document holding ```` ```sh ````, `cargo test --all-features`, `  indented`,
  and ```` ``` ```` is rendered at 58 and at 78
- **THEN** at both widths exactly two code lines are produced, whose `text()` values are
  `cargo test --all-features` and `  indented` — the leading spaces preserved
- **AND** at both widths every segment of those two lines carries `code` true and every
  other flag false
- **AND** at both widths no line's `text()` contains ```` ``` ```` or `sh`, so the fence
  and the info string are not rendered

#### Scenario: A code line longer than the interior is hard-split rather than word-wrapped

- **WHEN** a fenced block holding one line of 130 `x` characters is rendered at 58 and at
  78
- **THEN** at 58 it produces three code lines of 58, 58, and 14 characters
- **AND** at 78 it produces two code lines of 78 and 52 characters
- **AND** at both widths every character of the original line is present, in order, when
  the lines are concatenated

#### Scenario: An indented code block renders the same as a fenced one

- **WHEN** a document whose code block is written as four-space-indented lines rather than
  fenced is rendered at 58 and at 78
- **THEN** at both widths the produced code lines' `text()` values equal those the fenced
  form produces for the same code
- **AND** at both widths those lines carry `code` true

#### Scenario: A raw HTML block renders verbatim rather than being dropped

- **WHEN** a document holding `<details><summary>Notes</summary>` on its own line is
  rendered at 58 and at 78
- **THEN** at both widths a line whose `text()` is `<details><summary>Notes</summary>` is
  present and carries `code` true
- **AND** at both widths nothing in the document has been silently discarded: the
  concatenation of every line's `text()` contains `<details>`

### Requirement: Block quotes, thematic breaks, and constructs the parser does not model

A block quote's every line, continuations included, SHALL be prefixed `> ` and carry
`quoted` true, with its content wrapped to `width - 2`. A nested quote SHALL be prefixed
`> > `.

A thematic break SHALL be a line of `-` repeated to fill the width exactly, carrying
`Face::plain()` — the same ASCII rule `change-rows`' archived separator uses, rather than a
box-drawing character.

The parser SHALL be configured with `pulldown_cmark::Options::empty()`. Tables, footnotes,
strikethrough, and task-list items are therefore **not** modelled, and such source SHALL
render as its literal text — one line per source line, by the soft-break rule below —
rather than being dropped or mangled. This is the degraded state named in `SPEC.md`'s
degraded-states table.

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

#### Scenario: A table renders as its literal source text, one row per line

- **WHEN** a three-row GitHub-flavoured table — `| Gate | Runner |`,
  `|---|---|`, `| Format | cargo fmt |` — is rendered at 58 and at 78
- **THEN** at both widths three lines are produced whose `text()` values are those three
  source rows verbatim, so no row is folded into its neighbour and nothing is dropped
- **AND** at both widths every segment carries `Face::plain()`, since the extension is off
  and the rows are ordinary paragraph text

### Requirement: Blocks are separated by one blank line and source line structure is preserved

A **soft break** — a newline inside a paragraph — SHALL start a new rendered line rather
than being folded into a space. Re-flowing a paragraph would destroy exactly the shapes
this pane exists to read: an unmodelled table's rows, and prose an author hard-wrapped on
purpose. A **hard break** SHALL do the same.

Exactly one blank `Line` — one with no segments — SHALL separate two adjacent blocks. There
SHALL be no blank line before the first block and none after the last, so a document never
opens or closes with dead rows.

#### Scenario: A soft break starts a new line rather than being folded

- **WHEN** a paragraph written as `first line` / `second line` on two source lines is
  rendered at 58 and at 78 — widths at which both would fit on one line together
- **THEN** at both widths two lines are produced, `first line` and `second line`
- **AND** at both widths no line's `text()` is `first line second line`

#### Scenario: Exactly one blank line separates blocks, with none leading or trailing

- **WHEN** a document of `# One`, a paragraph, a bullet list, a fenced code block, and a
  closing paragraph — with two blank source lines between two of them — is rendered at 58
  and at 78
- **THEN** at both widths the first returned line is the heading and the last is the
  closing paragraph's last line, neither of them blank
- **AND** at both widths no two consecutive returned lines are both blank, and exactly one
  blank line sits between each adjacent pair of blocks

### Requirement: The markdown parser is confined to one module and imports no view type

`pulldown_cmark` SHALL be named only in `src/ui/markdown.rs`, so the parser can be replaced
by editing one file, on the same terms as `src/cli.rs` being the crate's only process
spawner. `src/ui/markdown.rs` SHALL name no `ratatui` type and no filesystem, process,
environment, network, or standard-I/O API: it is a pure-data module on the pure side of the
render seam, and joins the set `dashboard-loop` names.

The `pulldown-cmark` dependency SHALL be declared with `default-features = false` and
`features = []`, so neither the `html` renderer nor the `getopts` command-line parser its
defaults carry enters the build.

#### Scenario: `pulldown_cmark` is named only in `src/ui/markdown.rs`

- **WHEN** every `*.rs` file under `src/` other than `src/ui/markdown.rs` is searched for
  `pulldown_cmark` and `pulldown-cmark`
- **THEN** there is no match
- **AND** it is paired with a positive control asserting that `src/ui/markdown.rs` **does**
  name `pulldown_cmark`, and it fails when that file is absent, so a search that matched
  nothing because it searched nothing fails instead of passing
- **AND** the check is proven able to fail: run against a copy of `src/` carrying
  `use pulldown_cmark::Parser;` in `src/ui/view.rs`, it reports the file and line

#### Scenario: The markdown module names no ratatui type and no I/O API

- **WHEN** `src/ui/markdown.rs` is searched for `ratatui`, `Style`, `Modifier`, `Span`,
  and for `std::fs`, `std::io`, `std::env`, `std::process`, `std::net`, `File::`,
  `read_to_string`, and `Command`
- **THEN** there is no match for any of them
- **AND** `src/ui/markdown.rs` is one of the six files `dashboard-loop`'s pure-view-file
  check searches, so this is enforced by the same mechanism rather than a second one
- **AND** the `html` feature is observably off: no source file names `pulldown_cmark::html`,
  the manifest spells out `features = []`, and neither `pulldown-cmark-escape` nor
  `getopts` appears in the resolved build graph
