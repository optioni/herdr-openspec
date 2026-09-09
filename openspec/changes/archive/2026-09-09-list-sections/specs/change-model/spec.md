## MODIFIED Requirements

### Requirement: Archived changes are file-sourced only, and the two lists stay separate

`ChangeSet` SHALL hold `active` and `archived` as two separate vectors, a repository-level
`problems` list, and an `archived_total: usize` — the number of archived changes the archive
directory holds — rather than one vector discriminated by `origin`.

The separation is a contract, not a convenience. `openspec list --json` lists active changes
only, and the OpenSpec CLI offers no way to address a change under `archive/` at all — an
`openspec instructions apply --change 2026-09-04-task-parsing` fails with "change not
found". Archived changes are therefore permanently file-sourced, and Phase 3's merge policy
layers CLI results over the `active` list alone while `archived` passes through untouched.
Keeping them in one vector would make that policy filter on a field instead of reading a
list, and would silently drop archived rows the first time the filter was written wrong.

A name MAY appear in both lists at once; the two entries are distinct changes and neither
suppresses the other.

`archived_total` is `list-sections`' addition and exists because `archived` is no longer
always populated: `change-enumeration` resolves the archived tier only when the archived
section is shown, and a collapsed section's header still has to say how many changes are
behind it. It SHALL be the count the archive **enumeration** produced, before any change was
built, so it is the same number under either scope. Two invariants therefore hold on every
`ChangeSet` either producer returns, and a **new** `#[cfg(test)]` function
`conformance::assert_set_invariants(set: &ChangeSet)` SHALL check both: `archived.len()` is
either `0` or exactly `archived_total`, and `archived_total` is never less than
`archived.len()`.

It SHALL be a new function beside `assert_invariants`, never a widening of it.
`assert_invariants` takes a `&Change` and destructures it exhaustively with no rest pattern,
which is this capability's **mechanism 2** — the `E0027` guard that makes adding a `Change`
field a compile error inside the shared conformance function. Changing its parameter to a
`ChangeSet` would destroy that guarantee for every landed call site.
`assert_set_invariants` SHALL destructure `ChangeSet` exhaustively for the same reason, so a
sixth `ChangeSet` field is a compile error there too. `changes::merge` SHALL carry the file result's `archived_total`
through untouched, on exactly the terms `archived` itself passes through, and
`changes::empty_set()` SHALL set it to `0`.

`archived_total` is **not** a field of `Change` and does not weaken the seven-field rule
above: it is a property of the set, like `problems`, and `ChangeSet` derives no `Default`
either, so adding it is a compile error at every construction site rather than a silent `0`
at one of them.

#### Scenario: An archived change keeps its file-derived values when the CLI arrives

- **WHEN** a repository holds an active change `add-auth` and an archived
  `2026-08-14-add-auth`
- **THEN** `active` holds one `Change` named `add-auth` and `archived` holds one `Change`
  named `add-auth` with `origin` `Archived { date: Some("2026-08-14") }`
- **AND** the two are separate values with different `dir` values, and neither list is
  filtered by the other
- **AND** `archived_total` is `1`, and `changes::merge` of that set with any `CliChanges`
  leaves it `1`

#### Scenario: A repository-level failure is recorded on the set, not on a change

- **WHEN** `openspec/changes/` exists but cannot be read
- **THEN** `ChangeSet::problems` holds exactly one entry naming the directory and the reason
- **AND** `active` and `archived` are both empty, no synthetic `Change` is invented to carry
  the message, and the archive directory beneath it is not attempted — reading
  `openspec/changes/archive/` under an unreadable parent fails with a permission error of
  its own, which would make a second, redundant problem
- **AND** `archived_total` is `0`, so an unreadable tree reports no archive rather than an
  archive whose size is unknown

#### Scenario: The two `archived_total` invariants hold under either scope

- **WHEN** a scratch repository whose archive holds twenty-two dated directories is
  enumerated once with the archived tier resolved and once with it unresolved, and
  `conformance::assert_set_invariants` is called on both results
- **THEN** the resolved set has `archived.len()` 22 and `archived_total` 22, and the
  unresolved set has `archived.len()` 0 and `archived_total` 22
- **AND** `assert_set_invariants` accepts both and rejects a hand-built `ChangeSet` whose
  `archived` holds three changes while `archived_total` is 22, so the invariant is a check
  rather than a comment
- **AND** `assert_invariants` still takes a `&Change` and every landed call site compiles
  unchanged, so mechanism 2's `E0027` guard is intact
