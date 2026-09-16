# task-labels Specification

## Purpose
Recognises the lifecycle label at the head of a task item — `RED:`, `VERIFY:`,
`CHANGE — rewrite in `SPEC.md`:` — and classifies it into one of three positions of a
testing lifecycle, or into a generic fallback. One pure total function over a `&str`,
`crate::tasks::label_of`, returning where the label starts, how many bytes it covers, and
which position it names; `tasks-checklist` splits an item's first row at those offsets and
`view-palette` decides what each position looks like.

It lives in `src/tasks.rs` rather than under `src/ui/` because recognising a label is a fact
about a task's text and not about how a frame is painted — a new pure-view file would move a
count that `view-palette` and `responsive-layout` both bind and that three gate scripts carry
as a `PURE` list, five sites for a function that needs none of them.

The vocabulary is a **general testing vocabulary**, not one workflow's task prefixes: this
capability reads no schema, no `openspec/config.yaml`, and no change's `.openspec.yaml`, so
GIVEN/WHEN/THEN and ARRANGE/ACT/ASSERT classify identically to RED/GREEN/VERIFY and an
unrecognised run degrades to the generic role rather than to no label at all. Four roles
rather than one per keyword, because seven hues is a rainbow nobody learns and the
distinction a reader wants is which third of a group's lifecycle a row belongs to.

## Requirements

### Requirement: A task item's leading label is recognised by one total function

`crate::tasks` SHALL expose the label vocabulary and its recognition:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRole {
    Evidence,
    Change,
    Confirm,
    Other,
}

/// Where `text`'s leading label starts, how many bytes it covers, and the role
/// it classifies to. `start` skips any task number; `len` covers the label token
/// itself and never the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Label {
    pub start: usize,
    pub len: usize,
    pub role: LabelRole,
}

/// `None` when `text` carries no label.
pub fn label_of(text: &str) -> Option<Label>;
```

It SHALL live in `src/tasks.rs`, **not** under `src/ui/`. The classification is a fact about
a task's text, not about how a frame is painted, and a new file under `src/ui/` would move the
pure-view file count that `view-palette` and `responsive-layout` both bind — a cost this
capability has no reason to pay.

`label_of` SHALL be **total**: it SHALL NOT panic for any `&str`, including the empty string,
a string of only whitespace, invalid-looking but valid UTF-8, a string of only uppercase
letters, and a string whose first character is a multi-byte grapheme. It SHALL never return a `start` or a
`start + len` that is not a character boundary of `text`, so a caller may split `text` at
both without checking.

`label_of` SHALL recognise a label by this rule, applied to the item's **already-trimmed**
text as `task-groups` produces it:

1. **Skip an optional task number**: a run of ASCII digits and `.` characters, optionally
   followed by one ASCII lowercase letter, followed by exactly one space. `1.1 `, `10.2a ` and
   `7 ` are skipped; `1.1` with no following space is not, and neither is `a1 `.
2. **Take the leading run** of two or more ASCII uppercase letters (`A`–`Z`). A run of one
   letter is not a label, which is what keeps `Commit: …` and `Run \`cargo test\` …` unlabelled.
3. **Require the run to be a whole word**: the character immediately after it SHALL be absent,
   or SHALL NOT be an ASCII letter or an ASCII digit. `REDdish` is therefore not a label and
   `RED-by-addition:` is.
4. **Require a colon**: a `:` SHALL occur somewhere at or after the end of the run. This is
   what separates a labelled task from prose that happens to open with an acronym.
5. `start` SHALL be the byte offset of the run's first letter — after the skipped number, not
   at it — and `len` SHALL cover the run plus one **immediately following** `:` when there is
   one. So `VERIFY: …` gives `start` 0 and `len` 7, `1.1 RED: …` gives `start` 4 and `len` 4,
   and `CHANGE — rewrite in \`SPEC.md\`: …` gives `start` 0 and `len` 6. The task number is
   deliberately outside the label: a reader wants `1.1` to stay legible, and styling it with
   the label would make the whole row's leading third one colour.

Measured against `openspec/changes/archive/*/tasks.md` at the time this was written, scanning
**the first physical line of each item only** — which is what `tasks::parse` keeps: 2563 task
items, of which **2269** carry a label under this rule and **294** do not. Of the 2269, 2196
are the plain `<RUN>:` form and **73** are compound (`RED then GREEN:` 9, `RED-by-addition:` 4,
`RED→GREEN:` 3, `CHECK (contract gate):` 3, `CHECK — contract gate:` 2, and the
`CHANGE — …` family, of which `CHANGE — rewrite in \`SPEC.md\`:` is 11). **Three** items
carry a leading uppercase run this rule declines — `8.3 VERIFY — …` and `8.3a VERIFY — …` in
`archive/2026-09-04-subprocess-seam/tasks.md`, and `13.4 DEFERRED to archive time — …` in
`archive/2026-09-05-detail-view/tasks.md` — each because its colon sits on a continuation line
`tasks::parse` discards. They render unlabelled.

**Zero** items are given a label that is not one. That asymmetry is why step 4 is a colon test
rather than a keyword allow-list: the rule's errors are misses, never wrong colours, and an
allow-list would have to be extended for every schema while still missing `DEFERRED`.

#### Scenario: The plain and compound label forms are both recognised

- **WHEN** `tasks::label_of` is called on `VERIFY: cargo test is green`, on
  `CHANGE — rewrite in \`SPEC.md\`: the module map`, on `RED then GREEN: write both`, on
  `RED-by-addition: add the row`, and on `CHECK (contract gate): re-read design.md`
- **THEN** each returns `Some` with `start` `0`, and `len` `7`, `6`, `3`, `3`, and `5`
  respectively — the run plus a colon only where the colon immediately follows it
- **AND** the roles are `Confirm`, `Change`, `Evidence`, `Evidence`, and `Evidence`

#### Scenario: A task number is skipped and does not become part of the label

- **WHEN** `tasks::label_of` is called on `1.1 RED: write the failing test`, on
  `10.2a GREEN: implement it`, and on `7 VERIFY: make check`
- **THEN** each returns `Some` with `len` `4`, `6`, and `7` respectively — `RED:`, `GREEN:`,
  and `VERIFY:` — and the roles `Evidence`, `Change`, and `Confirm`
- **AND** `start` is `4`, `6`, and `2` respectively, pointing at the `R`, the `G`, and the `V`
  and never at the digit, so a caller splitting there leaves the number in the unstyled prefix

#### Scenario: Unlabelled tasks are recognised as unlabelled

- **WHEN** `tasks::label_of` is called on `Commit: the parser and its tests`, on
  `Run \`cargo test --all-features\``, on `Rewrite in \`SPEC.md\`: the module map`, on
  `REDdish text`, on `A: short`, and on `CI must stay green`
- **THEN** every call returns `None`
- **AND** the reasons are, in order: a one-letter run; a one-letter run; no leading uppercase
  run at all; the run is not a whole word; a one-letter run; and no colon anywhere in the text

#### Scenario: The recognition is total over degenerate input

- **WHEN** `tasks::label_of` is called on the empty string, on `"   "`, on `":"`, on `"::::"`,
  on `"ABC"` with no colon, on `"ABC:"`, on `"1.1 "`, on `"1.1"`, on a 10000-character run of
  `A` followed by `:`, and on `"日本語: text"`
- **THEN** no call panics
- **AND** the results are `None`, `None`, `None`, `None`, `None`,
  `Some(Label { start: 0, len: 4, role: Other })`, `None`, `None`,
  `Some(Label { start: 0, len: 10001, role: Other })`, and `None`
- **AND** for every `Some`, both `start` and `start + len` are character boundaries of the
  input, checked by calling `text.split_at` at each

### Requirement: The vocabulary is three lifecycle positions, not one word per colour

`LabelRole` SHALL classify a recognised run by **exact, case-sensitive** match against this
table, and everything unmatched SHALL be `Other`:

| Role | Tokens |
|---|---|
| `Evidence` | `RED`, `CHARACTERIZE`, `CHECK`, `GIVEN`, `ARRANGE` |
| `Change` | `GREEN`, `REFACTOR`, `CHANGE`, `WHEN`, `ACT` |
| `Confirm` | `VERIFY`, `THEN`, `ASSERT` |
| `Other` | every other recognised run |

The table already **is** a function of its own — `fn role_of(run: &str) -> LabelRole` in
`src/tasks.rs`, private, with `Other` as its fallback arm. It SHALL become reachable to a second
consumer, which changes its visibility and its return type and nothing else:

```rust
/// The lifecycle position `run` names, or `None` when the table holds no row
/// for it. The table itself, without `label_of`'s recognition rules around it.
pub fn role_of(run: &str) -> Option<LabelRole>;
```

`role_of` SHALL stay the **one** site of the table above — it already is, and `label_of`
already reaches the table only through it — so this is a widening of an existing seam and not a
new one. The two changes are that it becomes `pub`, and that its `_ => LabelRole::Other`
fallback arm becomes `_ => None`; `label_of` then absorbs the fallback at its own call site as
`role_of(run).unwrap_or(LabelRole::Other)`, which SHALL leave `label_of`'s observable behaviour
byte-identical for every input.

`spec-delta-badges` is the second consumer and the reason this requirement changed: a spec's
`WHEN` and a task's `WHEN` are the same fact, and two implementations of one fact drift.

`role_of` returns `Option` where `label_of` returns `Other`, and the difference is load-bearing.
`label_of` has already decided the run *is* a label by the time it classifies, so an
unrecognised token is a label of unknown position. `clause_of` has decided nothing, and needs
`None` to mean "this bold run is not a clause keyword at all" — which is what keeps a
`- **Note**` bullet in a spec unstyled. Collapsing the two would style every bold run at the
head of every list item in the tree.

`role_of` SHALL be **total** and SHALL NOT panic for any `&str`, including the empty string.

The table SHALL be a **general testing vocabulary**, not the active schema's task prefixes.
The `tdd` schema's own instruction states the three lifecycles are parallel — RED before GREEN
for behavior, CHARACTERIZE before REFACTOR for refactors, CHECK before CHANGE for operational
work — so the vocabulary is three positions wearing different names, and GIVEN/WHEN/THEN and
ARRANGE/ACT/ASSERT are the same three positions under two further conventions. A schema using
those SHALL be styled identically, with nothing read from `openspec/config.yaml`, from the
change's `.openspec.yaml`, or from the loaded schema. This capability SHALL NOT consult the
schema at all.

Four roles rather than seven-plus is the point: seven hues is a rainbow nobody learns, and the
distinction a reader actually wants is which third of a task group's lifecycle a row belongs
to. `RED` and `GREEN` landing on red and green is a consequence of the positions, not the
reason for them.

`Other` SHALL NOT be a failure. An unrecognised run is a label the reader wrote deliberately
(`NOTE:`, `TODO:`, a schema this crate has never seen), and it is styled as a label — just
not as one of the three positions.

#### Scenario: Every token in the table classifies to its own role

- **WHEN** `tasks::label_of` is called on each of `RED:`, `CHARACTERIZE:`, `CHECK:`, `GIVEN:`,
  `ARRANGE:`, `GREEN:`, `REFACTOR:`, `CHANGE:`, `WHEN:`, `ACT:`, `VERIFY:`, `THEN:`, and
  `ASSERT:`, each followed by ` do the thing`
- **THEN** the first five classify to `Evidence`, the next five to `Change`, and the last three
  to `Confirm`
- **AND** the assertion discriminates: `RED` reports `Evidence` and not `Change`, and `CHANGE`
  reports `Change` and not `Evidence`, so a table that collapsed two roles could not pass

#### Scenario: An unrecognised run is a generic label, not a miss

- **WHEN** `tasks::label_of` is called on `NOTE: see design.md`, on `TODO: later`, and on
  `HANDOFF: the next session picks this up`
- **THEN** each returns `Some` with the role `Other` and a `len` covering the run and its
  colon — `5`, `5`, and `8`
- **AND** none returns `None`, so an unknown vocabulary degrades to the generic role rather
  than to no label

#### Scenario: Matching is case-sensitive and whole-run

- **WHEN** `tasks::label_of` is called on `Red: lower`, on `RE D: spaced`, and on
  `REDGREEN: joined`
- **THEN** `Red: lower` returns `None` — a one-letter uppercase run
- **AND** `RE D: spaced` returns `Some` with the role `Other`, the run being `RE`
- **AND** `REDGREEN: joined` returns `Some` with the role `Other`, because the run is matched
  whole and `REDGREEN` is in no row of the table

#### Scenario: The classification reads nothing outside its argument

- **WHEN** `src/tasks.rs` is searched for the schema-reading names `schema::`, `Schema`,
  `config.yaml`, and `.openspec.yaml`
- **THEN** none occurs in `label_of` or in `LabelRole`'s definition
- **AND** `label_of` is called in a test with no `Schema` value constructed anywhere in scope,
  proving the signature admits none

#### Scenario: The table is reachable on its own and `label_of` agrees with it

- **WHEN** `tasks::role_of` is called on each of `RED`, `CHARACTERIZE`, `CHECK`, `GIVEN`,
  `ARRANGE`, `GREEN`, `REFACTOR`, `CHANGE`, `WHEN`, `ACT`, `VERIFY`, `THEN`, and `ASSERT`
- **THEN** the first five return `Some(Evidence)`, the next five `Some(Change)`, and the last
  three `Some(Confirm)`
- **AND** for every one of the thirteen, `tasks::label_of` called on that run followed by
  `: do the thing` reports the same role, asserted in the same test, so the two cannot
  disagree without failing

#### Scenario: An unrecognised run is `None` to the table and `Other` to the label

- **WHEN** `tasks::role_of` is called on `NOTE`, `TODO`, `HANDOFF`, `REDGREEN`, and the empty
  string
- **THEN** every call returns `None`
- **AND** `tasks::label_of` called on `NOTE: see design.md` returns `Some` with the role
  `Other`, so the recognised-but-unclassified run still reaches the generic role and only the
  bare table declines

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
