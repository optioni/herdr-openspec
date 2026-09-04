# task-checkboxes Specification

## Purpose

Deciding which lines of a change's task file are tasks, whether each one is done, and
what the resulting counts are. The rule is not this crate's invention: it is the rule
`@fission-ai/openspec` applies in `dist/utils/task-progress.js`
(`TASK_LINE_PATTERN = /^\s*[-*]\s*\[([\sxX])\]\s*(.*)/`), reproduced here because the
plugin's file path and the CLI path must report the same `[completed/total]` for the
same change. The dual-source model layers CLI results over file results on every
refresh; two counting rules would make the pane show one number, then a different one,
for a change nobody touched. Where this specification looks over-precise — a
non-breaking space standing for an empty box, a byte-order mark before a bullet, a
checkbox inside a fenced code block — it is precise because the CLI is, and the
agreement is the requirement.

## Requirements

### Requirement: A task line is a `-` or `*` bullet carrying a one-character checkbox

The plugin SHALL treat a line as a task when, after any amount of leading whitespace,
it begins with a single `-` or `*`, followed by any amount of whitespace, followed by
`[`, exactly one character drawn from the checkbox alphabet (whitespace, `x`, or `X`),
and `]`. Everything after the closing bracket, with leading and trailing whitespace
trimmed, is the item's **text**, kept verbatim — including any `1.2`-style numbering
prefix, which the plugin never strips or interprets.

No other bullet marker qualifies. A `+` bullet and an ordered-list marker (`1.`) SHALL
NOT be task lines, because the CLI's pattern accepts neither, and a plugin that counted
them would report a total the CLI does not.

#### Scenario: The four canonical shapes are task lines

- **WHEN** `count` is given the four lines `- [ ] a`, `- [x] b`, `* [ ] c`, `* [x] d`
- **THEN** the result is `Progress { completed: 2, total: 4 }`
- **AND** `parse` yields four items whose texts are `a`, `b`, `c`, `d`

#### Scenario: Whitespace around the bullet and the box is optional

- **WHEN** `count` is given `-[x]done` on one line and `   *   [ ]   todo   ` on the next
- **THEN** the result is `Progress { completed: 1, total: 2 }`
- **AND** `parse` yields items whose texts are exactly `done` and `todo`, with no
  surrounding whitespace

#### Scenario: A `+` bullet and an ordered marker are not task lines

- **WHEN** `count` is given `+ [x] plus`, `1. [x] ordered`, and `1) [x] paren`
- **THEN** the result is `Progress { completed: 0, total: 0 }`

#### Scenario: A malformed box is not a task line

- **WHEN** `count` is given `- [] empty`, `- [-] dash`, `- [xx] two`, `- [x extra`, and
  `- ( ) round`
- **THEN** the result is `Progress { completed: 0, total: 0 }`

#### Scenario: A bullet with no checkbox and prose that mentions one are not task lines

- **WHEN** `count` is given `- an ordinary list item`, `text - [x] mid-line`, and
  `-- [x] two dashes`
- **THEN** the result is `Progress { completed: 0, total: 0 }`

#### Scenario: A numbering prefix and inline markup are kept verbatim in the text

- **WHEN** `parse` is given `- [x] 3.11 Run the group tests — `cargo test` green` and
  `- [ ] 10.5a **VERIFY:** coverage`
- **THEN** the two item texts are exactly `3.11 Run the group tests — `cargo test` green`
  and `10.5a **VERIFY:** coverage`
- **AND** no numbering prefix is parsed into a field of its own, no backtick or asterisk
  is resolved, and nothing is truncated

### Requirement: `x` or `X` marks an item done; every other legal box character does not

The plugin SHALL treat a checkbox as **checked** when its single character is `x` or
`X`, and as **unchecked** for every other character the box alphabet admits. `completed`
is the number of checked items and `total` is the number of task lines, so
`completed <= total` always holds.

#### Scenario: Both letter cases count as done

- **WHEN** `count` is given `- [x] lower` and `- [X] upper`
- **THEN** the result is `Progress { completed: 2, total: 2 }`

#### Scenario: A space, a tab, and a non-breaking space are all empty boxes

- **WHEN** `count` is given three lines whose boxes hold U+0020, U+0009, and U+00A0
  respectively
- **THEN** the result is `Progress { completed: 0, total: 3 }`
- **AND** `parse` reports `checked == false` for each of the three items

#### Scenario: A checked item's state survives grouping

- **WHEN** `parse` is given `## Group` followed by `- [X] done` and `- [ ] todo`
- **THEN** the group's two items report `checked == true` and `checked == false` in that
  order
- **AND** the group's progress is `Progress { completed: 1, total: 2 }`

### Requirement: Counting has no context exemptions

The plugin SHALL count every line matching the task pattern wherever it appears —
inside a fenced code block, inside an HTML comment, inside an indented block, or inside
a block quote — and SHALL NOT track fence state, comment state, or block-quote state
for counting purposes.

This is a deliberate agreement, not an omission. The CLI records that skipping fenced
checkboxes was tried and withdrawn, because every rule for deciding which fence is
"real" has an input where a stray or unbalanced fence swallows genuine tasks: counting a
documented example is a loud, visible false positive, while losing a real task is a
silent one. A plugin that applied the more "correct" rule would under-report exactly the
files the CLI over-reports, and the two numbers would disagree on this repository's own
change artifacts, which contain both fenced examples and HTML-comment preambles.

#### Scenario: A checkbox inside a fenced code block counts

- **WHEN** `count` is given a document holding ` ```text `, `- [x] example`, ` ``` `,
  and then `- [ ] real`
- **THEN** the result is `Progress { completed: 1, total: 2 }`

#### Scenario: A checkbox inside an HTML comment counts

- **WHEN** `count` is given `<!--`, `- [x] commented out`, `-->`, and `- [ ] real`
- **THEN** the result is `Progress { completed: 1, total: 2 }`

#### Scenario: A block-quoted checkbox does not count, because the quote marker is not a bullet

- **WHEN** `count` is given `> - [x] quoted`
- **THEN** the result is `Progress { completed: 0, total: 0 }`
- **AND** this is the CLI's outcome too: `>` is not in the bullet alphabet, so the
  agreement holds without a block-quote rule existing

### Requirement: The checkbox whitespace alphabet is the CLI's, plus the byte-order mark

The whitespace admitted before a bullet, between a bullet and its box, inside an empty
box, and after the box SHALL be the set the CLI's `\s` matches: Unicode `White_Space`
together with U+FEFF, and excluding U+0085. This differs from Rust's
`char::is_whitespace` at exactly two code points, and both differences SHALL be resolved
in the CLI's favour:

- **U+FEFF** (byte-order mark / zero-width no-break space) is whitespace here and is not
  whitespace to `char::is_whitespace`. A UTF-8 BOM at the head of a `tasks.md` sits
  immediately before that file's first character, so without this the first task line of
  a BOM-prefixed file would be invisible to the plugin and visible to the CLI.
- **U+0085** (next line) is whitespace to `char::is_whitespace` and is not whitespace to
  the CLI. It is admitted by neither rule here.

#### Scenario: A byte-order mark before the first bullet does not hide the task

- **WHEN** `count` is given a document whose first character is U+FEFF, immediately
  followed by `- [x] first`, and whose second line is `- [ ] second`
- **THEN** the result is `Progress { completed: 1, total: 2 }`

#### Scenario: A next-line character before a bullet is not indentation

- **WHEN** `count` is given a line whose first character is U+0085, immediately followed
  by `- [x] task`
- **THEN** the result is `Progress { completed: 0, total: 0 }`

#### Scenario: A next-line character inside the box is not an empty box

- **WHEN** `count` is given a line whose box holds U+0085
- **THEN** the result is `Progress { completed: 0, total: 0 }`

### Requirement: Line endings and trailing carriage returns never change a count

The plugin SHALL split a task file on U+000A alone and SHALL discard a single trailing
U+000D from each resulting line before applying the task pattern, so that a file saved
with CRLF endings and the same file saved with LF endings produce identical counts and
identical item text. A final line with no terminator SHALL be scanned like any other.

#### Scenario: A CRLF document counts and reads the same as its LF twin

- **WHEN** `count` and `parse` are given `## G\r\n- [x] a\r\n- [ ] b\r\n` and then the
  same document with `\n` endings
- **THEN** both counts are `Progress { completed: 1, total: 2 }`
- **AND** both parses yield the group heading `G` and item texts `a` and `b`, with no
  carriage return in any of the three strings

#### Scenario: The last line counts without a trailing newline

- **WHEN** `count` is given `- [x] only`, with no terminating newline
- **THEN** the result is `Progress { completed: 1, total: 1 }`

#### Scenario: An empty document has no tasks and is not an error

- **WHEN** `count` and `parse` are given the empty string, and then a document of blank
  lines and prose with no checkbox at all
- **THEN** both counts are `Progress { completed: 0, total: 0 }`
- **AND** both parses yield no groups and no problems

### Requirement: `Progress` is the counting shape both sources of a change produce

`Progress` SHALL carry `completed` and `total` as counts and nothing else, so that
`changes-from-files` (which obtains them by counting checkboxes) and `changes-from-cli`
(which obtains them from `openspec list --json`'s `completedTasks` and `totalTasks`) can
produce the identical value for the same change with no conversion and no third state.

`Progress` SHALL support addition, because the tasks artifact of a schema may be a
directory glob resolving to several files and the CLI sums them into one pair. A change
with no tasks SHALL NOT be reported as complete: `is_complete` is true only when `total`
is greater than zero and `completed` equals `total`, matching the CLI's own three-way
`No tasks` / `n/m tasks` / `✓ Complete` distinction.

#### Scenario: A fully checked file is complete

- **WHEN** `count` is given three lines, all `- [x]`
- **THEN** the result is `Progress { completed: 3, total: 3 }` and `is_complete` is true

#### Scenario: A file with no tasks is not complete

- **WHEN** `count` is given a document with no task line
- **THEN** the result is `Progress { completed: 0, total: 0 }` and `is_complete` is
  false

#### Scenario: Progress values from several files sum

- **WHEN** `Progress { completed: 1, total: 3 }` is added to
  `Progress { completed: 2, total: 2 }`
- **THEN** the result is `Progress { completed: 3, total: 5 }`
- **AND** the same sum is reached by `+=` on a mutable accumulator starting at
  `Progress { completed: 0, total: 0 }`

#### Scenario: A file-derived count and a CLI-shaped count compare equal

- **WHEN** `count` is given a document holding four task lines of which one is checked,
  and a `Progress` is built directly from the pair `(completed: 1, total: 4)` a CLI
  response would supply
- **THEN** the two values are equal
