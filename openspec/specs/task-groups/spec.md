# task-groups Specification

## Purpose

Arranging the task lines of a change's task file into the document model the detail
view renders: groups introduced by the headings above them, items in document order
inside each group, and a count per group and for the file as a whole. Grouping is a
presentation layer over the counting rule in `task-checkboxes` and is required never to
change what that rule counts — a heading rule that could swallow or fabricate a task
line would put the file path and the CLI path back into disagreement, which is the one
thing the counting rule exists to prevent. This capability also owns the single
filesystem edge: reading a task file at a path a caller supplies, without writing
anything, anywhere.

## Requirements

### Requirement: An ATX heading at the start of a line opens a group

The plugin SHALL treat a line beginning at column zero with one to six `#` characters,
followed by a space or the end of the line, as a heading that closes the group before it
and opens a new one. The heading's **level** is the number of `#` characters and its
**text** is the remainder of the line, trimmed, kept verbatim — no closing `#` sequence
is stripped and no numbering prefix is interpreted.

The column-zero requirement is deliberately stricter than CommonMark, which admits up to
three leading spaces. An indented `#` inside a code block or a nested list is prose, and
promoting it to a group would fabricate structure the author did not write. Seven or
more `#` characters, and a `#` immediately followed by a non-space character, SHALL NOT
open a group.

Task lines appearing before the first heading SHALL form a group with **no** heading,
and that leading group SHALL be present only when it holds at least one item **or at
least one block**, so a file that opens with a title heading does not begin with an
empty unnamed group, while a file that opens with prose still carries that prose.

The block half of that condition is what makes retention **total**. Measured over this
repository's archive, **27 of 44** task files open with non-blank content above their
first heading — a lifecycle comment, a template banner, an ordering note — **736**
non-blank lines in all, every one of which the item-only condition discards. A rule that
retains every line inside a group and then drops the group cannot satisfy "Every retained
line appears exactly once", so the two clauses are written against each other here rather
than left to disagree.

A leading group carrying blocks but no items SHALL still hold `Progress { completed: 0,
total: 0 }` and SHALL NOT change what `count` reports for the same text, because a block
is not a checkbox. This is the one shape in which grouping is **not** byte-identical to
the pre-retention grouping, and it is named here rather than left as an exception a reader
has to infer from the retention paragraph in the requirement below.

#### Scenario: Two headings yield two groups holding their own items

- **WHEN** `parse` is given `## 1. First`, `- [x] a`, `- [ ] b`, `## 2. Second`,
  `- [ ] c`
- **THEN** the result holds two groups, with heading texts `1. First` and `2. Second`
  and levels 2 and 2
- **AND** the first group's items are `a` and `b` and the second group's item is `c`
- **AND** the group progresses are `Progress { completed: 1, total: 2 }` and
  `Progress { completed: 0, total: 1 }`

#### Scenario: Items before the first heading form an unnamed leading group

- **WHEN** `parse` is given `- [x] loose`, then `## Group`, then `- [ ] inside`
- **THEN** the result holds two groups, the first with no heading and the item `loose`,
  the second with heading text `Group` and the item `inside`

#### Scenario: A file opening with a heading has no empty leading group

- **WHEN** `parse` is given `# Implementation Tasks`, a blank line, `## 1. Group`, and
  `- [ ] a`
- **THEN** the result holds exactly two groups, the first with heading text
  `Implementation Tasks` at level 1 and no items, the second with heading text
  `1. Group` and one item
- **AND** no group with an absent heading appears anywhere in the result

#### Scenario: A closing hash sequence is kept, not stripped

- **WHEN** `parse` is given `## Group ##` followed by `- [ ] a`
- **THEN** the group's heading text is exactly `Group ##` at level 2
- **AND** no closing sequence rule exists to go wrong on a heading like `## C# ##`

#### Scenario: Indented, over-long, and unspaced hashes are not headings

- **WHEN** `parse` is given `   ## Indented`, `####### Seven`, and `#NoSpace`, each
  followed by `- [ ] a`
- **THEN** the result holds exactly one group, with no heading, holding three items
- **AND** the total count is `Progress { completed: 0, total: 3 }`

#### Scenario: A leading group of prose is emitted so its blocks survive

- **WHEN** `parse` is given `Intro prose.`, a blank line, `## G`, and `- [ ] a`
- **THEN** the result holds two groups: a headingless leading group with no items and one
  block whose text is `Intro prose.` and whose recorded position is `0`, then group `G`
  holding item `a`
- **AND** `parse(text).progress()` equals `count(text)`, both reporting
  `Progress { completed: 0, total: 1 }`, so emitting the leading group moved no count

#### Scenario: A file opening with a heading still has no empty leading group

- **WHEN** `parse` is given `# Implementation Tasks`, a blank line, `## 1. Group`, and
  `- [ ] a`
- **THEN** the result holds exactly two groups and no group with an absent heading, the
  pre-retention behaviour being unchanged wherever the leading run holds neither an item
  nor a block
- **AND** a document of blank lines alone yields no group at all

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

### Requirement: An item carries its checked state, its text, and its indent

Every item SHALL carry the checked state, the trimmed text defined by
`task-checkboxes`, and the **indent**: the number of whitespace characters preceding its
bullet, counted as characters rather than columns, so a tab counts as one. "Whitespace"
here is the same alphabet `task-checkboxes` defines and no other, so a byte-order mark
before a bullet contributes one to the indent rather than ending the scan. Nested items
SHALL remain siblings in the same group, distinguished only by that indent — the detail
view indents by it, and the count treats them exactly as the CLI does, which is as
ordinary tasks.

#### Scenario: A nested sub-task is a sibling item carrying its indent

- **WHEN** `parse` is given `## G`, `- [x] parent`, `  - [ ] child`, `\t- [ ] tabbed`
- **THEN** the group holds three items in that order with indents 0, 2, and 1
- **AND** the group progress is `Progress { completed: 1, total: 3 }`

#### Scenario: Indent does not affect membership of the preceding group

- **WHEN** `parse` is given `## G`, `        - [ ] deeply indented`, `## H`
- **THEN** the deeply indented item belongs to group `G`, not to group `H`

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

### Requirement: Reading a task file degrades rather than failing

The plugin SHALL read a task file from a path a caller supplies and SHALL never return
an error, panic, or fail closed. An **absent** file SHALL yield an empty result with no
problem recorded — the CLI treats a missing `tasks.md` as zero tasks, and a change whose
tasks artifact has not been written yet is an ordinary state, not a fault. Every other
read failure — a path naming a directory, a permission error, an I/O error, or contents
that are not valid UTF-8 — SHALL yield an empty result carrying exactly one
human-readable problem naming the path, so `degraded-states` can surface it.

Invalid UTF-8 belongs with the read failures rather than with the parse results, for the
same reason `schema` places it there: nothing about it is a structural problem, and the
consequence for the caller is identical to an I/O error's.

This is the one place the plugin's result is knowingly allowed to differ from the CLI's.
Node decodes with replacement characters and so still reports a count for a file holding
invalid bytes, while Rust's UTF-8 read fails; the plugin SHALL report zero tasks and a
named problem rather than decoding lossily. A file whose bytes are corrupt is exactly the
case the dual-source model exists for — the CLI path arrives and corrects the count,
`Tasks::problems` says why the file path had none — and lossy decoding would instead
silently assign a count to bytes nobody can read.

#### Scenario: An absent file is zero tasks and no problem

- **WHEN** `read` is given a path inside an existing directory at which no file exists
- **THEN** the result holds no groups, a progress of `Progress { completed: 0, total: 0 }`,
  and no problems

#### Scenario: A directory where a file was expected is one named problem

- **WHEN** `read` is given the path of an existing directory
- **THEN** the result holds no groups and exactly one problem whose text contains that
  path

#### Scenario: A file of invalid UTF-8 is one named problem, not a parse result

- **WHEN** `read` is given a file whose bytes are not valid UTF-8
- **THEN** the result holds no groups and exactly one problem whose text contains that
  path

#### Scenario: A readable file is parsed exactly as its text would be

- **WHEN** a file holding `## G\n- [x] a\n- [ ] b\n` is written into a scratch directory
  and `read` is given its path
- **THEN** the result equals `parse` applied to the same text, problems included

### Requirement: Task parsing and reading write nothing

Neither parsing nor reading SHALL create, modify, remove, or touch any filesystem entry,
and neither SHALL spawn a process. The dashboard reads `openspec/` while an agent may be
editing `tasks.md` in another pane, so a write from this path would corrupt work in
progress rather than merely misreport it.

#### Scenario: A task tree is byte-identical after reading

- **WHEN** a fixture tree holding `openspec/changes/x/tasks.md`, an empty
  `openspec/specs/` directory, and `README.md` is snapshotted — entries, bytes, and
  modification times — and `read` is called twice on the `tasks.md` path, on the
  directory path, and on a path that does not exist
- **THEN** a second snapshot equals the first
- **AND** the non-existent path and its parent directories are still absent

#### Scenario: The module names no process API

- **WHEN** `src/tasks.rs` is searched for `std::process`, `Command`, `spawn`, `output`,
  and `status`
- **THEN** no occurrence is found, and the crate's only process references remain
  `crate::pid`'s single documented call

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
end a block **except inside an open fence**. A group with no such content SHALL carry an
empty list.

The fence exception is not a refinement — without it this capability's own purpose fails.
A fenced block is opened by a line whose first non-whitespace run is three or more `` ` ``
or `~` characters and closed by the next line whose run matches it or by the end of the
group, and every line between them, blank ones included, belongs to the block that opened
it. Measured over this repository's archive, **46 of 112** column-zero fenced blocks
contain a blank line; under a blank-terminates-always rule each of those is shredded into
two or more blocks whose delimiters no longer pair, so `ui::markdown::lines` reflows the
code as prose and drops the `` ``` `` rows entirely — turning the 156-block defect this
change exists to fix into a differently-broken rendering of 46 of them. An unterminated
fence SHALL run to the end of its group rather than being abandoned, so the rule is total.

Blocks SHALL NOT be emitted for content that an item's body already claimed, so every
retained line appears in exactly one place and a renderer that draws every item body and
every block reproduces each non-blank source line once.

An item's **body** takes the same fence exception, and for the same reason: a blank line
inside a fence opened within a body does not end that body, even though the continuation
rule above would otherwise end it at the first line at or below the item's indent. A fence
opened inside a body SHALL run to the body's own end.

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

#### Scenario: A fenced block survives the blank line inside it

- **WHEN** `parse` is given `## G`, `- [ ] a`, a blank line, a column-zero fence opening,
  `make check`, a blank line, `make coverage`, a fence closing, a blank line, and `- [ ] b`
- **THEN** the group holds two items and exactly **one** block, whose text holds all five
  fence lines in order including the blank one between the two commands
- **AND** that block's text handed to `ui::markdown::lines` renders both commands with
  `face.code` and no backtick, which a block split at the blank line could not do
- **AND** the block's recorded position is `1`

#### Scenario: A group with no interstitial content carries no blocks

- **WHEN** `parse` is given `## G`, `- [ ] a`, and `- [ ] b`
- **THEN** the group's block list is empty
- **AND** the group still holds both items
