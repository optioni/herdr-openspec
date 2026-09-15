# spec-delta-badges Specification

## Purpose
Recognising the structural vocabulary OpenSpec itself writes into a delta spec: the three
delta-operation headings (`## ADDED Requirements` and its two siblings), and the keyword that
opens a scenario clause (`- **WHEN**`, `- **THEN**`, `- **AND**`). Two pure total functions
over borrowed text in `src/specs.rs`, plus the `DeltaOp` vocabulary; `artifact-folds` owns both the
**attribution walk** that turns the first into a per-section `operation` and the header row that
draws it, `markdown-render` faces a clause keyword from the second, and `view-palette` decides
what each looks like. This capability classifies one heading and one run; it never walks a
section list, because the list is `ArtifactSection`'s and that type is `artifact-folds`'.

It lives in `src/specs.rs` rather than under `src/ui/` for exactly the reason `task-labels`
lives in `src/tasks.rs`: recognising a heading is a fact about a spec's text and not about how
a frame is painted, and a new *pure-view* file would move a count that `view-palette` and
`responsive-layout` both bind and that three gate scripts carry as a `PURE` list — five sites
for a function that needs none of them.

This capability **classifies** and never styles, never reads a file, and never consults the
schema, `openspec/config.yaml`, or a change's `.openspec.yaml`. It recognises what OpenSpec's
own artifact instructions already mandate, which is why it needs no configuration to be right.

## Requirements

### Requirement: The three delta operations are recognised by one total function

`crate::specs` SHALL expose the delta-operation vocabulary and its recognition:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaOp {
    Added,
    Modified,
    Removed,
}

/// The delta operation the heading at `level` with label `label` names, or
/// `None` when it names none. `label` is the heading's remainder with
/// surrounding whitespace trimmed, exactly as `HeadingSection.label` carries it.
pub fn operation_of_heading(level: u8, label: &str) -> Option<DeltaOp>;
```

`operation_of_heading` SHALL return `Some` only when **both** hold:

1. `level` is exactly `2`. The operation headings OpenSpec writes are `##` headings, and a
   level check inside the function is what keeps the rule from being a caller's to remember.
2. `label` splits on ASCII whitespace into exactly **two** tokens, the second of which is
   exactly `Requirements`, and the first of which is exactly `ADDED`, `MODIFIED`, or
   `REMOVED` — matched **case-sensitively**.

Splitting on whitespace rather than comparing the whole string is deliberate and is the only
tolerance offered: `##  ADDED   Requirements` classifies, and `## Added Requirements`,
`## ADDED Requirement`, `## ADDED Requirements (2)`, and `### ADDED Requirements` do not.

`RENAMED Requirements` SHALL classify to `None`. OpenSpec's artifact instructions name it as a
fourth operation, and **zero** occurrences exist across `openspec/changes/archive/*/specs/*/spec.md`
at the time this was written. An unbadged requirement is the correct degradation for an
operation this crate has never seen rendered: it loses the badge and keeps every other thing
the detail region already draws.

`operation_of_heading` SHALL be **total**: it SHALL NOT panic for any `u8` and any `&str`,
including the empty string, a string of only whitespace, one of only ASCII whitespace variants,
and one whose first character is a multi-byte grapheme.

Measured across `openspec/changes/archive/*/specs/*/spec.md` at the time this was written,
the level-2 headings are exactly four kinds and nothing else: `MODIFIED Requirements` **134**,
`ADDED Requirements` **109**, `REMOVED Requirements` **15**, and `Purpose` **9**. The rule
therefore classifies 258 of 267 level-2 headings and declines the other 9, with no
misclassification available to it.

#### Scenario: Each of the three operation headings classifies to its own variant

- **WHEN** `specs::operation_of_heading` is called with level `2` and the labels
  `ADDED Requirements`, `MODIFIED Requirements`, and `REMOVED Requirements`
- **THEN** the results are `Some(DeltaOp::Added)`, `Some(DeltaOp::Modified)`, and
  `Some(DeltaOp::Removed)` respectively
- **AND** the assertion discriminates: `ADDED Requirements` reports `Added` and not `Modified`,
  so a function collapsing two variants could not pass

#### Scenario: Internal whitespace is tolerated and nothing else is

- **WHEN** `specs::operation_of_heading` is called with level `2` and the labels
  `ADDED   Requirements` and `MODIFIED\tRequirements`
- **THEN** both return `Some`, with `Added` and `Modified`
- **AND** the labels `Added Requirements`, `ADDED Requirement`, `ADDED Requirements (2)`,
  `ADDEDRequirements`, `Requirements ADDED`, and `ADDED` each return `None`

#### Scenario: Only a level-2 heading carries an operation

- **WHEN** `specs::operation_of_heading` is called with the label `ADDED Requirements` at each
  of the levels `0`, `1`, `3`, `4`, `5`, `6`, and `255`
- **THEN** every call returns `None`
- **AND** the same label at level `2` returns `Some(DeltaOp::Added)`, so the level is what
  discriminates and not the label

#### Scenario: A renamed operation and a main spec's heading both decline

- **WHEN** `specs::operation_of_heading` is called with level `2` and the labels
  `RENAMED Requirements`, `Requirements`, and `Purpose`
- **THEN** every call returns `None`
- **AND** no call panics, so an operation this crate has never rendered degrades to an
  unbadged requirement rather than to a failure

#### Scenario: The recognition is total over degenerate input

- **WHEN** `specs::operation_of_heading` is called at level `2` with the empty string, with
  `"   "`, with `"\t\n"`, with a 10000-character run of `A`, with `"日本語 Requirements"`,
  and with `"ADDED Requirements 日本語"`
- **THEN** no call panics
- **AND** every call returns `None`

### Requirement: A scenario clause's keyword is classified through the task-label table

`crate::specs` SHALL expose the clause vocabulary and its recognition:

```rust
/// What a bold run at the head of a list item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clause {
    /// A keyword naming its own lifecycle position.
    Opens(crate::tasks::LabelRole),
    /// A continuation, carrying the position of the clause above it.
    Continues,
}

/// `None` when `run` is not a clause keyword.
pub fn clause_of(run: &str) -> Option<Clause>;
```

`clause_of` SHALL classify by this rule, applied to the **whole** run with no trimming, no
case folding, and no prefix matching:

1. `AND` SHALL return `Some(Clause::Continues)`.
2. Otherwise, the run SHALL be looked up in the `task-labels` token table through
   `crate::tasks::role_of`, and a hit SHALL return `Some(Clause::Opens(role))`. So `WHEN`
   returns `Opens(Change)` and `THEN` returns `Opens(Confirm)`, and `GIVEN`, `ARRANGE`, `ACT`,
   and `ASSERT` classify identically to the conventions they belong to.
3. Every other run SHALL return `None`.

The table SHALL NOT be copied. `crate::tasks::role_of` is the crate's one lifecycle-token
table, and this capability reaches it rather than restating it — two implementations of one
fact drift, and a spec's `WHEN` and a task's `WHEN` are the same fact.

The **recognition** is deliberately *not* shared with `tasks::label_of`. That function's rule 4
requires a colon at or after the keyword, and a scenario clause has none, so `label_of` would
fire on exactly the clauses that happen to contain a colon further along — a worse failure than
never firing. Relaxing rule 4 is refused: `task-labels` measured **zero** false positives across
2563 task items precisely because of it. One table, two recognitions, and the spec for each
says so.

`clause_of` SHALL be **total** and SHALL NOT panic for any `&str`.

Measured across `openspec/specs/*/spec.md` and every archived delta at the time this was
written, the scenario clause vocabulary is exactly **three** tokens and nothing else:
`AND` **6701**, `WHEN` **3877**, `THEN` **3877**. `WHEN` and `THEN` are equal to the item —
one of each per scenario — which is what makes `AND`'s inheritance the only rule with any
work to do.

#### Scenario: The three measured keywords classify as specified

- **WHEN** `specs::clause_of` is called on `WHEN`, `THEN`, and `AND`
- **THEN** the results are `Some(Clause::Opens(LabelRole::Change))`,
  `Some(Clause::Opens(LabelRole::Confirm))`, and `Some(Clause::Continues)`
- **AND** `WHEN` and `THEN` report different roles, so a function collapsing them could not pass

#### Scenario: The wider testing vocabulary classifies through the same table

- **WHEN** `specs::clause_of` is called on `GIVEN`, `ARRANGE`, `ACT`, `ASSERT`, and `RED`
- **THEN** the results are `Opens(Evidence)`, `Opens(Evidence)`, `Opens(Change)`,
  `Opens(Confirm)`, and `Opens(Evidence)`
- **AND** each agrees with `crate::tasks::role_of` called on the same run, asserted by calling
  both in the test, so the two cannot disagree without failing

#### Scenario: A run outside the table is not a clause

- **WHEN** `specs::clause_of` is called on `and`, `When`, `WHENEVER`, `OR`, `BUT`, `IF`,
  `Requirement`, and the empty string
- **THEN** every call returns `None`
- **AND** `and` and `When` return `None` while `AND` and `WHEN` do not, so matching is
  case-sensitive and whole-run

#### Scenario: The classification reads nothing outside its argument

- **WHEN** `tests/doc_contract.rs` reads the **production slice** of `src/specs.rs` — the file
  cut at its first line-anchored `#[cfg(test)]`, by the helper
  `production_slice_cuts_before_cfg_test` already proves — and searches it for the I/O and
  schema needles `std::fs`, `std::io`, `std::env`, `std::process`, `std::net`, `File::`,
  `read_to_string`, `Command`, `schema::`, `Schema`, `config.yaml`, and `.openspec.yaml`
- **THEN** none occurs, and the check fails naming the needle and the line if one does
- **AND** the check lives in `tests/doc_contract.rs` and **not** inside `src/specs.rs`, because
  a check written inside the file it sweeps contains its own needles and can never pass — which
  is what an earlier draft of this scenario specified
- **AND** it is not a `scripts/gates/` script either: `NOIO-VIEW`'s `PURE` list is the
  render-seam's, and adding a non-view module to it would move the "ten pure files" figure that
  `AGENTS.md`, `SPEC.md`, `view-palette` and `responsive-layout` all carry — the five-site cost
  Decision 1 exists to avoid — while a thirty-second gate script would move the "thirty-one"
  count `openspec/specs/quality-gates/spec.md` states in seven places
- **AND** the check is falsifiable, proved by the standing mechanism: planting `use std::fs;`
  above the `#[cfg(test)]` line makes it fail, and removing the plant makes it pass

#### Scenario: The clause recognition is total over degenerate input

- **WHEN** `specs::clause_of` is called on the empty string, on `"   "`, on a 10000-character
  run of `A`, on `"日本語"`, on `"AND 日本語"`, and on a string whose first character is a
  multi-byte grapheme
- **THEN** no call panics
- **AND** every call returns `None`, `AND` being matched whole and `"AND 日本語"` not being it
- **AND** the obligation is the same one `operation_of_heading` carries above, so the two
  sibling functions in this capability are held to one standard for one property
