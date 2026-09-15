## MODIFIED Requirements

### Requirement: Groups and items preserve document order, and no heading is discarded

The plugin SHALL emit groups in the order their headings appear and items in the order
their lines appear, SHALL emit a group for every heading it recognises — including one
whose text is empty and one holding no items — and SHALL NOT sort, merge, deduplicate,
or nest them. Two headings of the same text produce two groups; a level-3 heading
following a level-2 heading closes the level-2 group rather than nesting inside it.

A flat sequence is the contract because the detail view renders a scrolling list and can
indent by level, while a tree would force every consumer to flatten it back before
counting.

Non-task content SHALL be **retained** rather than discarded, attributed to exactly one
of two places and never to both: an item's own `body` when it continues that item, and
its group's `blocks` otherwise. Retention SHALL NOT change grouping — a block between two
items does not close a group, open one, or produce a third — so the group structure this
requirement describes is byte-identical to the one produced before retention existed, for
every input.

#### Scenario: A deeper heading closes the group above rather than nesting

- **WHEN** `parse` is given `## Outer`, `- [ ] a`, `### Inner`, `- [ ] b`
- **THEN** the result holds two groups in that order, at levels 2 and 3, holding one
  item each
- **AND** the level-2 group's items do not include `b`

#### Scenario: Repeated and empty headings are all kept

- **WHEN** `parse` is given `## Same`, `- [ ] a`, `## Same`, `- [ ] b`, `##`, `## Last`
- **THEN** the result holds four groups in document order with heading texts `Same`,
  `Same`, `` (empty), and `Last`
- **AND** the third and fourth groups hold no items

#### Scenario: Prose between items is dropped and does not split a group

- **WHEN** `parse` is given `## G`, `- [ ] a`, a blank line, `Some explanatory prose.`,
  and `- [ ] b`
- **THEN** the result holds one group holding exactly two items, `a` and `b`
- **AND** the prose is not an item, does not close group `G`, and does not open a group of
  its own — "dropped" here means dropped from the **item list**, which retention does not
  change

#### Scenario: Prose between items is retained as a block

- **WHEN** `parse` is given `## G`, `- [ ] a`, a blank line, `Some explanatory prose.`,
  and `- [ ] b`
- **THEN** that group's `blocks` hold one entry whose text is `Some explanatory prose.`,
  with a recorded position of `1`, placing it after item `a` and before item `b`
- **AND** item `a`'s `body` is empty, because the prose is separated from it by a blank
  line and is not indented past its bullet

### Requirement: Grouping never changes what is counted

For every input, the sum of the group progresses SHALL equal the flat count the
`task-checkboxes` rule produces for the same text. The plugin SHALL expose both entry
points — a flat `count` and a structured `parse` — and their agreement SHALL be asserted
directly rather than assumed, because `changes-from-files` calls the cheap flat one to
paint a list row while the detail view calls the structured one for the same change, and
a divergence would show two different numbers for one change on one screen.

Retaining bodies and blocks SHALL NOT change either number. `count` SHALL remain the
line-based rule `task-checkboxes` defines, unchanged in every respect, and a checkbox line
SHALL be recognised as an **item** wherever it appears — including inside a fenced block
that is otherwise an item's body or a group's block — so that `parse` and `count` cannot
disagree about what a checkbox is. A fence therefore does not shelter a checkbox from
either entry point, which is the existing `task-checkboxes` rule restated at the point it
is most likely to be forgotten, not a new one.

#### Scenario: A real change's task file counts the same both ways

- **WHEN** a corpus of task documents — including a real `tasks.md` copied from this
  repository's own archive, a CRLF document, one with fenced and commented checkboxes,
  one with no heading, and an empty one — is passed to both `count` and `parse`
- **THEN** for every document, `parse(text).progress()` equals `count(text)`
- **AND** the copied archived document's `completed` and `total` equal the pair the
  OpenSpec CLI's own rule produces for the same bytes, obtained independently of this
  crate. The equality alone is satisfied by an implementation that counts nothing, so the
  absolute pair is part of the requirement rather than a convenience of the test that
  happens to assert it

#### Scenario: Text with no heading still agrees

- **WHEN** `count` and `parse` are given a document of five task lines, two of them
  checked, and no heading at all
- **THEN** both report `Progress { completed: 2, total: 5 }`, the parse yielding all five
  in one unnamed group
- **AND** the `completed` half is asserted as well as the `total`, so an implementation
  that marks every item checked cannot pass

#### Scenario: Retention leaves every count in the archive unmoved

- **WHEN** every `tasks.md` under `openspec/changes/`, archived changes included, is
  passed to `parse` and to `count`
- **THEN** for every file, `parse(text).progress()` equals `count(text)`
- **AND** every file's `(completed, total)` pair equals the pair recorded before retention
  existed, asserted against a committed fixture of those pairs rather than against a
  second run of the same code, so an implementation that changed both sides together
  cannot pass

#### Scenario: A checkbox inside a fenced block is an item, not body text

- **WHEN** `parse` is given `## G`, `- [ ] a`, an indented fence opening, an indented
  `- [ ] trapped` line, and an indented fence closing
- **THEN** the group holds two items, `a` and `trapped`
- **AND** `count` reports `Progress { completed: 0, total: 2 }` for the same text, so the
  two entry points agree about the trapped line

## ADDED Requirements

### Requirement: An item carries the body lines that follow its bullet

Every item SHALL carry a **body**: the lines following its bullet that continue it,
retained verbatim and in document order, with each line's leading indentation preserved
relative to the shallowest non-blank line in the body, so that a fenced block inside a
body survives as a fenced block.

A line SHALL continue the preceding item when it is blank, or when its indentation is
strictly greater than that item's own `indent`. The body SHALL end at the first line that
is a heading, is a checkbox line at any indent, or is non-blank with indentation at or
below the item's own `indent`. Trailing blank lines SHALL be stripped from the body, so an
item followed by a blank line and then a new item carries an empty body rather than a body
of one blank line.

A checkbox line SHALL never be body text, at any indent. A nested sub-task is indented past
its parent's bullet and would otherwise satisfy the continuation rule; it SHALL remain a
sibling item, as the existing indent requirement states, and its own body SHALL be
attributed to it rather than to its parent.

The body SHALL be the empty string when the item has none, so that every item carries the
field and no consumer distinguishes absent from empty.

#### Scenario: An item's continuation lines are retained as its body

- **WHEN** `parse` is given `- [x] 2.2 GREEN: Add the seventh trait method, its`,
  `      implementation writing the OSC 52 sequence, and the`, and
  `      arm. Entry and teardown order is untouched.`
- **THEN** the group holds exactly one item whose `text` is
  `2.2 GREEN: Add the seventh trait method, its`
- **AND** that item's `body` holds the two following lines in order, their relative
  indentation preserved

#### Scenario: A nested sub-task is a sibling item and takes its own body

- **WHEN** `parse` is given `- [x] parent`, `  continues the parent`, `  - [ ] child`, and
  `    continues the child`
- **THEN** the group holds two items, `parent` and `child`, with indents 0 and 2
- **AND** `parent`'s body is `continues the parent` and does not include either following
  line
- **AND** `child`'s body is `continues the child`

#### Scenario: A fenced block indented under an item is that item's body

- **WHEN** `parse` is given `- [ ] run the gate`, a fence opening indented six columns, a
  line reading `make check` indented six columns, and a fence closing indented six columns
- **THEN** the item's `body` holds all three lines in order, opening fence included
- **AND** the body's text parses as a fenced code block when handed to a markdown renderer

#### Scenario: A dedented line ends the body and is not attributed to the item

- **WHEN** `parse` is given `## G`, `  - [ ] indented item`, `    continues it`, and
  `A line at column zero`
- **THEN** the item's `body` is exactly `continues it`
- **AND** `A line at column zero` is a block of group `G`, not part of the item's body

#### Scenario: An item with nothing after it carries an empty body

- **WHEN** `parse` is given `## G`, `- [ ] a`, a blank line, and `- [ ] b`
- **THEN** both items carry an empty `body`
- **AND** neither body is a string of one blank line

### Requirement: A group carries the blocks that sit between its items

Every group SHALL carry an ordered list of **blocks**: the runs of non-blank content
inside that group which no item's body claimed, each retained verbatim with its own
indentation, and each recording the number of items that precede it in the group, so a
renderer can place it in document order without a second pass over the source.

A block SHALL be emitted for content before the group's first item as well as for content
between items and after the last one; the position recorded for content before the first
item SHALL be zero. Consecutive non-blank lines SHALL form one block; a blank line SHALL
end a block. A group with no such content SHALL carry an empty list.

Blocks SHALL NOT be emitted for content that an item's body already claimed, so every
retained line appears in exactly one place and a renderer that draws every item body and
every block reproduces each non-blank source line once.

#### Scenario: A group's lifecycle marker is retained as a block before its first item

- **WHEN** `parse` is given `## 2. Terminal ops`, `<!-- kind: behavior -->`, a blank line,
  and `- [x] 2.1 RED: write the failing test`
- **THEN** the group holds one item and one block
- **AND** the block's text is `<!-- kind: behavior -->` and its recorded position is `0`

#### Scenario: A fenced block at group level is retained between the items it sits between

- **WHEN** `parse` is given `- [ ] a`, a blank line, a fence opening at column zero, a
  line reading `make check`, a fence closing at column zero, a blank line, and `- [ ] b`
- **THEN** the group holds two items and one block
- **AND** the block's recorded position is `1`, placing it after item `a` and before `b`
- **AND** the block's text holds all three fence lines in order

#### Scenario: Every retained line appears exactly once

- **WHEN** every `tasks.md` under `openspec/changes/`, archived changes included, is passed
  to `parse`
- **THEN** for every file, concatenating every group's heading line, every item's own line,
  every item body, and every group block yields each non-blank source line exactly once
- **AND** no non-blank source line is absent from that concatenation, so retention is total
  and no line is claimed twice

#### Scenario: A group with no interstitial content carries no blocks

- **WHEN** `parse` is given `## G`, `- [ ] a`, and `- [ ] b`
- **THEN** the group's block list is empty
- **AND** the group still holds both items
