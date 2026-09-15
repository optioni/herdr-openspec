## MODIFIED Requirements

### Requirement: The vocabulary is three lifecycle positions, not one word per colour

`LabelRole` SHALL classify a recognised run by **exact, case-sensitive** match against this
table, and everything unmatched SHALL be `Other`:

| Role | Tokens |
|---|---|
| `Evidence` | `RED`, `CHARACTERIZE`, `CHECK`, `GIVEN`, `ARRANGE` |
| `Change` | `GREEN`, `REFACTOR`, `CHANGE`, `WHEN`, `ACT` |
| `Confirm` | `VERIFY`, `THEN`, `ASSERT` |
| `Other` | every other recognised run |

The table SHALL be reachable as a function of its own, so a second consumer classifies against
it rather than against a copy of it:

```rust
/// The lifecycle position `run` names, or `None` when the table holds no row
/// for it. The table itself, without `label_of`'s recognition rules around it.
pub fn role_of(run: &str) -> Option<LabelRole>;
```

`role_of` SHALL be the **one** site of the table above, and `label_of` SHALL classify a run it
has recognised by calling it — `role_of(run).unwrap_or(LabelRole::Other)` — rather than by
matching the tokens a second time. `spec-delta-badges` is the second consumer and the reason
this requirement changed: a spec's `WHEN` and a task's `WHEN` are the same fact, and two
implementations of one fact drift.

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
