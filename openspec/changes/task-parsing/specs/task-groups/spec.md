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

## ADDED Requirements

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
and that leading group SHALL be present only when it holds at least one item, so a file
that opens with a title heading does not begin with an empty unnamed group.

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

### Requirement: Groups and items preserve document order, and no heading is discarded

The plugin SHALL emit groups in the order their headings appear and items in the order
their lines appear, SHALL emit a group for every heading it recognises — including one
whose text is empty and one holding no items — and SHALL NOT sort, merge, deduplicate,
or nest them. Two headings of the same text produce two groups; a level-3 heading
following a level-2 heading closes the level-2 group rather than nesting inside it.

A flat sequence is the contract because the detail view renders a scrolling list and can
indent by level, while a tree would force every consumer to flatten it back before
counting.

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
