# change-merge Specification

## Purpose
Reconciles the two producers of a change set into the one the pane renders: `changes::merge`
pairs active changes by name, layers the CLI's schema, progress and artifacts over the file
tier's while keeping the file tier's `dir`, passes archived changes through untouched, and
re-sorts the union so a CLI-only change lands in name order rather than at the end. Its hardest
rule is the artifact join, which is positional — never by path, which the two producers
canonicalize differently, and never by id, which a schema may legally repeat — with six stated
outcomes for empty, mismatched-length, and disagreeing lists, and the `tracks_tasks` flag
travelling with whichever whole list survives. Problems from both sides are kept in full rather
than dropped when a field is superseded. Producing either input is `change-enumeration`'s and
`cli-changes`' work; this is only the join.

## Requirements

### Requirement: The merge layers CLI results over the file result, by name

`changes::merge(files: ChangeSet, cli: CliChanges) -> ChangeSet` SHALL be a pure function —
no filesystem access, no CLI access — and SHALL be total: it returns a `ChangeSet`, never a
`Result`, and never panics.

Active changes SHALL be paired **by name**, against the file set's **active** list only.
Name is the only key either producer shares: `openspec list --json` reports a name and
nothing else that identifies a change, and both producers enumerate the same directory, so
a name is unique within each list. Pairing by name is the **change-level** join and is not
the positional join the next requirement specifies; that one applies to the artifacts
*within* a paired change. The two are different joins with different keys, and any prose
describing the merge as joined by position is wrong.

For a name present in both lists, the merged `Change` SHALL take:

| Field | From | Why |
|---|---|---|
| `name` | either — they are equal | |
| `dir` | the **file** change | the path every other file-sourced value was computed against; the merge layers onto the file result |
| `origin` | `Origin::Active` | `list --json` reports active changes only |
| `schema` | the **CLI** change | the CLI's `schemaName` is the authoritative answer, and correcting it is the point of the second producer |
| `artifacts` | the positional join below | |
| `progress` | the **CLI** change | `SPEC.md` → Dual-source model: the CLI arrives and corrects |
| `problems` | the file change's, then the CLI change's, then the join's | see below |

For a name present in the **CLI list only**, the merged `Change` SHALL be that CLI change
unchanged. For a name present in the **file list only**, the merged `Change` SHALL be that
file change unchanged, and no problem SHALL be recorded: the two reads are taken moments
apart and a change created or removed between them is a normal race, not a fault.

The merged `active` list SHALL be re-sorted by name in byte order after the union, so an
appended CLI-only change lands in position rather than at the end. The re-sort is also what
keeps the merge deterministic: the CLI-only remainder is drained from a `HashMap`, whose
iteration order is not stable, and sorting after the append is what prevents that order
from reaching the rendered list.

`archived` SHALL pass through byte-identical. `change-model` requires archived changes to be
permanently file-sourced, and `openspec list --json` filters `archive` out of its walk
(`dist/core/list.js:85-87`), so there is nothing to layer.

The merge SHALL NOT consult `files.archived` when deciding whether a CLI change is
CLI-only. A change archived **between** the worker's `list --json` call and its file walk —
two different instants — therefore appears in **both lists of the merged `ChangeSet`**: in
`active` from the CLI's stale answer, and in `archived` from the fresh file walk. This is
accepted, not defended against, and SHALL be carried as its own row of `SPEC.md` →
Degraded states, stated at the `ChangeSet` level the merge produces rather than at a
rendered frame, because the merge is a pure function and the set is where the condition is
observable. Suppressing the active row would require the merge to treat an archived
name as authority over a CLI answer, which inverts the dual-source model's own rule that
the CLI corrects the files; and the state lasts one refresh cycle, because the next
`list --json` no longer reports the change. Two rows briefly is a truer picture of a
repository mid-archive than one row silently chosen from the two producers' disagreement.

`ChangeSet::problems` SHALL be the file set's problems followed by `CliChanges::problems`.

#### Scenario: The CLI's schema, progress, and artifacts replace the file's

- **WHEN** the file `ChangeSet` holds one active change `alpha` with `schema`
  `stale-name`, an empty `artifacts` list, `progress { completed: 2, total: 3 }`, and
  `dir` `<repo>/openspec/changes/alpha`; and `CliChanges::active` holds `alpha` with
  `schema` `tdd`, two `ArtifactRef`s, and `progress { completed: 4, total: 9 }`
- **THEN** the merged active change has `schema` `tdd`, carries the two `ArtifactRef`s, and
  has `progress { completed: 4, total: 9 }`
- **AND** the string `stale-name` appears nowhere in the merged change's `schema`, so an
  implementation keeping the file's schema fails this scenario — the two producers are
  deliberately given **different** schema names, since equal ones would leave the change's
  headline claim unverified
- **AND** its `dir` is `<repo>/openspec/changes/alpha`, the file change's, unchanged

#### Scenario: Changes are paired by name, not by position

- **WHEN** the file set's `active` holds `alpha` and `zulu` in that order, and
  `CliChanges::active` holds `zulu` and `alpha` in that order, with each CLI entry carrying
  a schema naming itself (`zulu-schema`, `alpha-schema`)
- **THEN** the merged `alpha` carries `alpha-schema` and the merged `zulu` carries
  `zulu-schema`
- **AND** an implementation pairing by index produces the swap and fails this scenario,
  which is the assertion that makes the change-level key observable

#### Scenario: A change only the CLI reported is inserted in name order

- **WHEN** the file set's `active` holds `alpha` and `zulu`, and `CliChanges::active` holds
  `alpha`, `mike`, and `zulu`
- **THEN** the merged `active` is ordered `alpha`, `mike`, `zulu`
- **AND** `mike` is the CLI change unchanged, and no problem is recorded for it

#### Scenario: A change only the file producer saw survives the merge

- **WHEN** the file set's `active` holds `alpha` and `newly-created`, and
  `CliChanges::active` holds `alpha` only
- **THEN** the merged `active` holds both, ordered `alpha`, `newly-created`
- **AND** `newly-created` is byte-identical to the file change, and no problem is recorded

#### Scenario: An empty CLI result leaves the file result intact

- **WHEN** `CliChanges` carries an empty `active` and one problem naming an absent
  `openspec` binary, and the file set holds two active and three archived changes
- **THEN** the merged `active` and `archived` equal the file set's exactly
- **AND** `ChangeSet::problems` is the file set's problems followed by that one entry

#### Scenario: Archived changes pass through untouched

- **WHEN** the file set holds an archived change `add-auth` with
  `origin Archived { date: Some("2026-08-14") }`, and `CliChanges::active` happens to hold
  an active change also named `add-auth`
- **THEN** the merged `archived` list is byte-identical to the file set's
- **AND** the merged `active` holds the CLI's `add-auth` as a separate value, so the two
  lists neither filter nor suppress one another

#### Scenario: A change archived mid-cycle appears in both lists for one cycle

- **WHEN** the file set's `active` is empty, its `archived` holds `add-auth` dated
  `2026-08-14`, and `CliChanges::active` holds `add-auth` — the state produced by archiving
  a change between the worker's `list --json` call and its file walk
- **THEN** the merged `ChangeSet` holds `add-auth` in `active` **and** in `archived`
- **AND** no problem is recorded, because neither producer is faulty
- **AND** running the merge again with an empty `CliChanges::active`, the next cycle's
  answer, leaves `add-auth` in `archived` alone, so the state is self-correcting rather
  than sticky

### Requirement: Artifact lists are joined by position, never by path and never by id

The two producers' `artifacts` vectors SHALL be joined by **index**. The join SHALL NOT
look an entry up by path and SHALL NOT look one up by id.

Path is not a key because the two producers produce legitimately different strings for the
same artifact: the CLI canonicalizes every path it reports
(`fs.realpathSync.native`, `dist/utils/file-system.js:76-89`) and sorts a multi-file
artifact's matches on the canonical absolute path, while the file producer joins
`generates` onto the change directory without canonicalizing and sorts on the raw OS-string
bytes.

Id is not a key because `schema-artifacts` requires a schema's artifact list to be kept
verbatim and never de-duplicated, so a schema declaring the same id at two positions
produces two `ArtifactRef`s carrying it and an id-keyed lookup would collapse them.

The join SHALL apply exactly these rules, in order:

1. The CLI list is **empty** — take the **file** list, record no problem. The CLI produced
   no schema for this change, which is already reported as its own problem. Two empty lists
   fall under this rule and yield the empty result, so there SHALL be no separate
   both-empty branch: a branch returning exactly what the next rule returns is not a
   distinct rule, and reads as one.
2. The file list is **empty** and the CLI list is not — take the **CLI** list, record no
   problem. This is the repaired case: the repository did not vendor the schema and
   `schema-cli-fallback` found it.
3. Both are non-empty and their **lengths differ** — take the **file** list and record one
   problem naming both lengths, because two different schemas were resolved and position is
   meaningless across them.
4. Both are non-empty and equal in length, and the ids at some index differ — take the
   **file** list and record one problem naming the **first** differing index and both ids.
5. Otherwise — take the **CLI** list.

Rules 3 and 4 prefer the file list because it is the one internally consistent with the
merged change's `dir` and with the artifacts a reader can open on disk right now.

#### Scenario: Equal-length lists with equal ids take the CLI's paths, positionally

- **WHEN** the file list holds ids `proposal`, `specs`, `design` with paths
  `<repo>/openspec/changes/a/proposal.md`, none, and none; and the CLI list holds the same
  three ids with a canonicalized `/private/var/.../a/proposal.md`, two `specs` paths, and
  none
- **THEN** the joined list holds three entries with those ids in that order, carrying the
  CLI's paths at every position
- **AND** no problem is recorded, and the joined `specs` entry carries two paths where the
  file entry carried none

#### Scenario: A duplicate id is joined by index rather than collapsed

- **WHEN** both lists hold ids `zeta`, `alpha`, `zeta` in that order, and the CLI's entry
  at index 2 carries a path the entry at index 0 does not
- **THEN** the joined list holds three entries with ids `zeta`, `alpha`, `zeta`
- **AND** index 0 and index 2 carry their own CLI paths rather than the same one, which an
  id-keyed lookup could not produce
- **AND** no problem is recorded

#### Scenario: An empty file list takes the CLI's list, which is the repaired case

- **WHEN** the file list is empty — the repository vendors no schema for this change — and
  the CLI list holds five ids
- **THEN** the joined list is the CLI's five entries
- **AND** no problem is recorded

#### Scenario: An empty CLI list keeps the file's list

- **WHEN** the file list holds five ids and the CLI list is empty
- **THEN** the joined list is the file's five entries
- **AND** no problem is recorded, because the CLI's own failure is already reported on the
  change

#### Scenario: Differing lengths keep the file list and name both counts

- **WHEN** the file list holds five ids and the CLI list holds three
- **THEN** the joined list is the file's five entries
- **AND** exactly one problem is recorded naming `5` and `3`

#### Scenario: A differing id at one index keeps the file list and names the index

- **WHEN** both lists hold four entries, agreeing at indices 0, 1, and 3, and holding
  `design` and `plan` respectively at index 2
- **THEN** the joined list is the file's four entries
- **AND** exactly one problem is recorded naming index `2`, `design`, and `plan`
- **AND** the problem names the first differing index only, even when two indices differ

#### Scenario: Two empty lists join to an empty list

- **WHEN** both lists are empty
- **THEN** the joined list is empty and no problem is recorded

### Requirement: Merged problems are kept in full, never dropped

The merged `Change::problems` SHALL be the file change's problems, then the CLI change's
problems, then the join's problem when it produced one — with exactly equal strings
collapsed to their first occurrence and the remaining order preserved.

No entry SHALL be dropped on the grounds that the CLI superseded the field it describes.
`problems` is human-readable diagnostic text, and the file producer records messages the
CLI cannot — a malformed `.openspec.yaml`, an unreadable `tasks.md` — which a reader stops
learning about the moment the merge discards them.

The consequence is accepted and stated: a change whose schema the repository does not
vendor but `schema-cli-fallback` repaired keeps the file producer's "is not vendored"
message alongside a fully populated artifact list. The message is still true — the
repository does not vendor it — and losing an unrelated message is the worse failure.

#### Scenario: A file-side message survives beside a corrected artifact list

- **WHEN** the file change carries `problems` holding one entry ending
  `is not vendored: no schema.yaml there` and an empty `artifacts` list, and the CLI change
  carries five `ArtifactRef`s and no problems
- **THEN** the merged change carries the five `ArtifactRef`s
- **AND** its `problems` still holds that one entry

#### Scenario: Duplicate messages from both producers are collapsed

- **WHEN** the file change and the CLI change each carry `problems` holding the byte-equal
  string `openspec/schemas/x/schema.yaml is not vendored: no schema.yaml there`, and the
  file change additionally carries a second, different message first
- **THEN** the merged `problems` holds exactly two entries, in the order the file change
  listed them
- **AND** the duplicate appears once

#### Scenario: A join problem is appended after both producers' problems

- **WHEN** the file change carries one problem, the CLI change carries one problem, and the
  join records a length disagreement
- **THEN** the merged `problems` holds three entries in that order, with the join's last

### Requirement: The merge is a third construction site, bound by the same gate

`merge` SHALL construct every merged `Change` by naming all seven fields explicitly, and
SHALL be bound by `change-model`'s two-producer gate exactly as the two producers are —
that requirement, modified by this change, is where the gate's mechanisms, their scope, and
their checks are specified, and this capability adds no second, differently-worded copy of
them.

Nothing in the merge SHALL be a reason to relax either mechanism. Every field of a merged
`Change` is supplied from one of the two inputs, so there is no field the merge cannot
name.

Every `Change` this change's tests build — from either producer and from the merge — SHALL
be passed through `changes::conformance::assert_invariants`.

#### Scenario: Every merged value satisfies the shared invariants

- **WHEN** a merge is performed over a file set holding two active and three archived
  changes and a CLI result holding three active changes, one of which is CLI-only
- **THEN** `assert_invariants` passes for all six resulting values
- **AND** it is the same function `from_files`' tests call, with no second copy of the
  invariants written for this producer

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
