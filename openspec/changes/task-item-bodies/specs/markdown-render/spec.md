## ADDED Requirements

### Requirement: A fragment renders as one paragraph through a second entry point

`ui::markdown::inline(text: &str, width: u16) -> Vec<Line>` SHALL render `text` as a
**single paragraph**, on exactly the terms `lines` renders a paragraph it found in a
document: the same inline faces, the same word wrapping at `width`, the same hard split for
a word longer than the interior, and the same `Line`/`Segment`/`Face` plain-data types. It
SHALL be a pure total transformation on the same terms as `lines` — no filesystem, process,
environment, network, or standard-I/O work, no clock, no global state, and no panic for any
`&str` and any `u16`.

`inline` exists because an item's text is a **fragment**, not a document. Handing
`# not a heading` or `1. first` to the block parser reinterprets the fragment and eats its
marker, which a renderer of task text must not do. Measured over this repository's archive,
**0 of 2,751** item texts would be reinterpreted today; the function is therefore justified
structurally rather than by a live defect, and this sentence records that so a later reader
does not mistake the measurement for a motivating bug.

`inline` SHALL recognise **no block construct**. A leading `#`, `-`, `*`, `+`, `>`, a fence
opening, a digit run followed by `.` or `)`, and a line of `-`/`=`/`_`/`*` that would
otherwise be a thematic break or a setext underline SHALL each render as **literal text** in
the paragraph. No heading face SHALL be set by `inline` on any segment it returns: every
returned `Face` SHALL carry `heading: None`.

Inline constructs SHALL be recognised exactly as `lines` recognises them inside a paragraph:
emphasis, strong, inline code, links, strikethrough, and inline HTML, each setting the same
`Face` field. A **soft** break SHALL fold into a single space and the paragraph SHALL reflow
at `width`; a **hard** break SHALL start a rendered line, matching the rule `lines` already
carries.

`inline` SHALL leave `muted`, `label`, and `delta` at their `Face::plain()` values on every
segment it returns, exactly as `lines` does — no markdown construct is a completion state, a
task label, or a delta operation, and setting those remains the caller's job.

`inline` SHALL return an **empty vector** at `width == 0`, matching `lines`, and an empty
vector for a `text` that is empty or entirely whitespace.

`inline` SHALL live in `src/ui/markdown.rs` and SHALL NOT widen the parser's option set,
which stays exactly `ENABLE_TABLES | ENABLE_STRIKETHROUGH | ENABLE_TASKLISTS`. It is a
second entry point into the parser already confined to that file, so the `MDSEAM` gate's
subject does not move.

#### Scenario: A fragment's inline faces are set and its text is unchanged

- **WHEN** `inline` is called at width `78` and at width `58` with
  `2.2 GREEN: add the `` `CrosstermOps` `` implementation, **bolded**, and *stressed*`
- **THEN** at both widths the concatenated text of every returned segment holds
  `CrosstermOps`, `bolded`, and `stressed` with their markers removed
- **AND** the `CrosstermOps` segment carries `face.code`, the `bolded` segment
  `face.strong`, and the `stressed` segment `face.emphasis`
- **AND** every returned segment carries `face.heading == None`

#### Scenario: A leading block marker is literal text, not a block

- **WHEN** `inline` is called at width `78` and at width `58` with each of `# not a heading`,
  `- not a bullet`, `> not a quote`, `1. not an ordered list`, and `--- not a rule`
- **THEN** at both widths each fragment's rendered text begins with its own marker, that
  marker present character for character
- **AND** no returned segment carries a `heading` face, and no list marker, quote prefix, or
  thematic-break run is introduced
- **AND** handing the same five fragments to `lines` produces different output for at least
  four of them, so the scenario distinguishes the two entry points rather than passing
  vacuously

#### Scenario: A fragment wraps and hard-splits exactly as a paragraph does

- **WHEN** `inline` is called at width `78` and at width `58` with a fragment long enough to
  wrap at both, and separately with a single unbroken word longer than `58` columns
- **THEN** the wrapped fragment produces strictly more lines at `58` than at `78`, and no
  line exceeds its width in display columns as `responsive-layout` measures them
- **AND** the unbroken word is hard-split at the interior rather than overflowing it or
  being dropped
- **AND** the same fragment wrapped as a lone paragraph by `lines` at the same width yields
  the same line texts, so the two entry points cannot drift apart on wrapping

#### Scenario: Empty, whitespace, and zero-width inputs return nothing

- **WHEN** `inline` is called with the empty string, with a string of three spaces, and with
  any text at `width == 0`
- **THEN** each call returns an empty vector
- **AND** no call panics, matching `lines` at the same three inputs
