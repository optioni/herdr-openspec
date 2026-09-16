## ADDED Requirements

### Requirement: The task-number skip is exposed on its own

`crate::tasks::task_number_len(text: &str) -> usize` SHALL return the byte length of the
leading task number `label_of` skips — a non-empty run of ASCII digits and `.` characters,
optionally followed by one ASCII lowercase letter, then exactly one space — and `0` when
`text` carries none.

It SHALL be the **same** skip `label_of` performs, reached through one shared helper rather
than a second copy of the rule, on exactly the terms `specs::clause_of` calls
`tasks::role_of` rather than restating its token table. Two implementations of one rule
drift, and this rule is now read from two places: `label_of`, to find where a label starts,
and `ui::tasks`, to decide where an item's wrapped text and its body hang.

`task_number_len` SHALL be **pure and total**: every `&str` returns a value, the value SHALL
never exceed `text.len()`, and the returned length SHALL always fall on a character boundary
of `text`, so a caller may `split_at` it without checking.

The function SHALL NOT be reached by `label_of`'s callers as a substitute for `Label.start`:
`start` remains the label's own offset and is `task_number_len`'s value only when the text
carries a label immediately after its number.

#### Scenario: A numbered item reports its number's width

- **WHEN** `task_number_len` is given `1.1 CHECK: move the count`, `10.11a GREEN: ship it`,
  and `2. RED: write it`
- **THEN** it returns `4`, `7`, and `3` respectively
- **AND** for each, `text.split_at(n)` succeeds and the prefix is exactly the number with its
  trailing space

#### Scenario: An unnumbered item reports zero

- **WHEN** `task_number_len` is given `CHECK: move the count`, `Commit the parser`, the empty
  string, and a string of one space
- **THEN** it returns `0` for each
- **AND** no call panics

#### Scenario: The exposed skip agrees with the one `label_of` performs

- **WHEN** every item text in this repository's archive is passed to both `task_number_len`
  and `label_of`
- **THEN** for every text that `label_of` returns `Some(Label { start, .. })` for, `start`
  equals `task_number_len(text)`
- **AND** the agreement is asserted over the corpus rather than over a handful of literals, so
  a second copy of the skip rule that diverged on an unusual number could not pass

#### Scenario: A malformed number is not a number

- **WHEN** `task_number_len` is given `1.1CHECK: no space`, `1.1ab GREEN: two letters`, and
  `.  leading dot then two spaces`
- **THEN** it returns `0` for the first two, the rule requiring exactly one trailing space and
  at most one lowercase letter
- **AND** the third returns `2`, a run of `.` being a digits-and-dots run followed by one
  space, which is the existing rule applied rather than a new exception
