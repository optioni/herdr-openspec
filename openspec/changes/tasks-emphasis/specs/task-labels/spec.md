## ADDED Requirements

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

Measured against `openspec/changes/archive/*/tasks.md` at the time this was written: 2563 task
items, of which **2272** carry a label under this rule and **291** do not. Of the 2272, 2196
are the plain `<RUN>:` form and **76** are compound (`CHANGE — rewrite in \`SPEC.md\`:` 35,
`RED then GREEN:` 9, `RED-by-addition:` 4, `CHECK (contract gate):` 3, `CHECK — contract
gate:` 2, `RED→GREEN:` 2, `CHANGE (only if …)` 1, and the rest singletons). **Zero** items
carry a leading uppercase run that is not a label, which is why step 4 is a colon test rather
than a keyword allow-list: an allow-list would have to be extended for every schema, and the
corpus says the shape alone is sufficient.

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
