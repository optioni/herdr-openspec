## ADDED Requirements

### Requirement: The positional join carries the tracked-tasks flag with its artifact

`tracks_tasks` SHALL travel with the `ArtifactRef` the positional join selects, and SHALL
NOT be merged, OR-ed, or reconciled field by field. The join's six rules already decide
which whole list survives; the flag is a field of that list's entries and is taken with
them.

Concretely: where rule 6 takes the CLI list, the **CLI's** `tracks_tasks` values survive —
the same tier that already replaces the change's schema, progress, and paths. Where rules 1,
4, and 5 take the file list, the **file's** values survive. Where rule 2 takes the CLI list
because the file list was empty, the CLI's do.

A merged list SHALL therefore still carry at most one entry with `tracks_tasks == true`,
because each producer's list already does and the join never mixes the two.

The one visible consequence, stated so it is a decision rather than a discovery: when the
two producers resolved **different** schemas for a change and the join takes the CLI list,
the tab that renders a checklist can move to a different position from the one the file
tier marked. That is correct — it is the same tier shift that already moves the tab bar's
labels and the change's progress — and it is the reason the flag is not recomputed after the
join from a schema the merged value no longer carries.

#### Scenario: Equal-length lists with equal ids take the CLI's flag

- **WHEN** the file list holds ids `proposal`, `specs`, `tasks` with `tracks_tasks` at
  position 2, and the CLI list holds the same three ids with `tracks_tasks` at position 2
- **THEN** the joined list carries `tracks_tasks == true` at position 2 and `false` at 0 and 1
- **AND** no problem is recorded

#### Scenario: A CLI list marking a different position wins with its own flag

- **WHEN** both lists hold ids `checklist`, `notes`, `tasks` in that order, the file list
  marking position 2 and the CLI list marking position 0 — the two tiers resolved different
  schemas for the same change
- **THEN** the joined list is the CLI's, carrying `tracks_tasks == true` at position 0 only
- **AND** no problem is recorded, because the ids agree at every index and rule 6 applies

#### Scenario: A rejected CLI list leaves the file's flag in place

- **WHEN** the file list holds five ids with `tracks_tasks` at position 3 and the CLI list
  holds three ids
- **THEN** the joined list is the file's five entries with `tracks_tasks == true` at
  position 3
- **AND** exactly one problem is recorded naming `5` and `3`, as this capability already
  requires
- **AND** the same holds when the lists are equal in length but disagree on an id, where
  rule 5 keeps the file list and its flag

#### Scenario: A merged list never carries two marked entries

- **WHEN** the join is exercised over every one of its six rules with lists whose marked
  positions differ
- **THEN** in every result at most one entry carries `tracks_tasks == true`
- **AND** no result carries a marked entry that neither input list carried
