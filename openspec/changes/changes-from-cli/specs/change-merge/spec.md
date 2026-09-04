## ADDED Requirements

### Requirement: The merge layers CLI results over the file result, by name

`changes::merge(files: ChangeSet, cli: CliChanges) -> ChangeSet` SHALL be a pure function —
no filesystem access, no CLI access — and SHALL be total: it returns a `ChangeSet`, never a
`Result`, and never panics.

Active changes SHALL be paired **by name**. Name is the only key either producer shares:
`openspec list --json` reports a name and nothing else that identifies a change, and both
producers enumerate the same directory, so a name is unique within each list.

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
appended CLI-only change lands in position rather than at the end.

`archived` SHALL pass through byte-identical. `change-model` requires archived changes to be
permanently file-sourced, and `openspec list --json` filters `archive` out of its walk
(`dist/core/list.js:85-87`), so there is nothing to layer.

`ChangeSet::problems` SHALL be the file set's problems followed by `CliChanges::problems`.

#### Scenario: The CLI's schema, progress, and artifacts replace the file's

- **WHEN** the file `ChangeSet` holds one active change `alpha` with `schema`
  `outside-in-tdd`, an empty `artifacts` list, `progress { completed: 2, total: 3 }`, and
  `dir` `<repo>/openspec/changes/alpha`; and `CliChanges::active` holds `alpha` with
  `schema` `outside-in-tdd`, two `ArtifactRef`s, and `progress { completed: 4, total: 9 }`
- **THEN** the merged active change carries the two `ArtifactRef`s and
  `progress { completed: 4, total: 9 }`
- **AND** its `dir` is `<repo>/openspec/changes/alpha`, the file change's, unchanged

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

1. The CLI list is **empty** and the file list is not — take the **file** list, record no
   problem. The CLI produced no schema for this change, which is already reported as its
   own problem.
2. The file list is **empty** and the CLI list is not — take the **CLI** list, record no
   problem. This is the repaired case: the repository did not vendor the schema and
   `schema-cli-fallback` found it.
3. Both are empty — take either; the result is empty and no problem is recorded.
4. Both are non-empty and their **lengths differ** — take the **file** list and record one
   problem naming both lengths, because two different schemas were resolved and position is
   meaningless across them.
5. Both are non-empty and equal in length, and the ids at some index differ — take the
   **file** list and record one problem naming the **first** differing index and both ids.
6. Otherwise — take the **CLI** list.

Rules 4 and 5 prefer the file list because it is the one internally consistent with the
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

### Requirement: The merge produces the same `Change` type, enforced at compile time

`merge` SHALL construct every merged `Change` by naming all seven fields explicitly. It
SHALL NOT introduce a `Default` implementation on `Change`, `ChangeSet`, `ArtifactRef`, or
`Origin`, and SHALL NOT use a functional-update `..` expression or a `..` rest pattern in
any `Change` literal or pattern anywhere in `src/changes.rs`.

Both halves of `change-model`'s gate SHALL remain in force and SHALL both be checked, and
neither SHALL be relaxed to make CLI-side construction easier:

- **No `Default` and no `..`** — so a new field is `E0063` at every construction site,
  which now includes `from_cli`'s and `merge`'s. A `..other` functional update compiles
  with no `Default` anywhere, which is exactly why the `..` half is checked separately from
  the `Default` half rather than implied by it.
- **`conformance::assert_invariants`'s exhaustive `let Change { … }` with no rest
  pattern** — so a new field is `E0027` in the one function both producers' tests call.

Every `Change` this change's tests build, from either producer and from the merge, SHALL be
passed through `assert_invariants`.

#### Scenario: Adding a field breaks both halves independently

- **WHEN** a field is added to `Change` in a throwaway copy of the crate and nothing else
  is changed
- **THEN** the build fails reporting `E0063` at each construction site — `from_files`',
  `from_cli`'s, and `merge`'s — **and** `E0027` in `conformance::assert_invariants`
- **AND** when the same copy additionally derives `Default` on `Change` and fills the new
  field with `..Default::default()` at every construction site, the build still fails with
  `E0027`, proving the conformance half catches what the `Default` half misses
- **AND** when instead a `..` rest pattern is added to `assert_invariants`' pattern and no
  `Default` is added, the build still fails with `E0063`, proving the `Default`/`..` half
  catches what the conformance half misses

#### Scenario: A source check rejects a reintroduced `Default` or rest pattern

- **WHEN** the guarded source checks are run against a copy of `src/changes.rs` carrying,
  in turn, `#[derive(..., Default)]` on `Change`, `impl Default for Origin`, a
  `Change { name, ..other }` literal, and a `Change { name, .. }` pattern
- **THEN** each run exits non-zero
- **AND** both checks exit non-zero when `src/changes.rs` does not exist, and the
  rest-pattern check exits non-zero when it finds no `Change {` construction at all, so
  neither can pass vacuously
- **AND** both exit zero against the real file, which contains a `segment[..star]` slice
  index, so the rest-pattern check discriminates a slice index from a rest pattern

#### Scenario: Every merged value satisfies the shared invariants

- **WHEN** a merge is performed over a file set holding two active and three archived
  changes and a CLI result holding three active changes, one of which is CLI-only
- **THEN** `assert_invariants` passes for all seven resulting values
- **AND** it is the same function `from_files`' tests call, with no second copy of the
  invariants written for this producer
