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
}
pub struct Segment { pub text: String, pub face: Face }
pub struct Line { pub segments: Vec<Segment> }
pub fn lines(source: &str, width: u16) -> Vec<Line>;
```

`Face::plain()` SHALL be the all-`false`, `heading: None` value. `Line::text()` SHALL
return the line's segments concatenated, which is what a width assertion reads. Mapping a
`Face` to a `ratatui::style::Style` is `ui::view`'s work and SHALL NOT happen here, on the
same terms as `ui::list` returning plain `String`s.

Every returned `Line`'s `text()` SHALL be at most `width` **display columns**, measured by
`layout::columns` — the same measure `ui::list`, `ui::tasks`, `ui::detail`, and `ui::view`
use, and the measure `ratatui::buffer::Buffer::set_string` itself consumes. A `char` count is
no longer the unit and is no longer a documented limitation: a source holding wide or
combining characters is measured correctly rather than approximately. A line SHALL NOT be
padded to the width: a region draws segments left to right and leaves the rest of the row
untouched.

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
as to code; the two differ only in whether spaces are break opportunities.

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
  fenced code block, a block quote, a thematic break, and a link is rendered at widths
  0, 1, 2, 3, 10, 58, 78, and 200
- **THEN** at every width every returned line's `text()` has at most that many **display
  columns**, measured by `layout::columns`
- **AND** the rendering at 58 and at 78 both hold at least one line of each block kind, so
  the assertion is made against real content rather than an empty result

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
  combining accent, a 500-column run of `日本語` with no space, and a run of family emoji
  joined by zero-width joiners
- **THEN** none panics and each returns a `Vec` whose lines all respect the width in columns
- **AND** the 500-character token and the 500-column CJK run are each hard-split rather than
  emitted as one over-wide line
- **AND** the same calls at widths `1` and `2` also do not panic, and at width `1` a line
  holding only a two-column cluster is empty rather than two columns wide
