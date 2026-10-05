## MODIFIED Requirements

### Requirement: Merged problems are kept in full, never dropped

The merged `Change::problems` SHALL be the file change's problems, then the CLI change's
problems, then the join's problem when it produced one — with exactly equal strings
collapsed to their first occurrence and the remaining order preserved.

No entry SHALL be dropped on the grounds that the CLI superseded the field it describes.
`problems` is human-readable diagnostic text, and the file producer records messages the
CLI cannot — a malformed `.openspec.yaml`, an unreadable `tasks.md` — which a reader stops
learning about the moment the merge discards them.

The consequence is accepted and stated: whatever the file change carries survives the merge,
even beside an artifact list the CLI corrected. Dropping a stale message is therefore the
**producer's** job, never the merge's. A schema the repository does not vendor but the CLI
can locate no longer produces a file-side "is not vendored" message at all, because the
refresh worker's locate pass gives the file producer that schema's directory before the merge
runs (`change-artifacts` → "A schema the repository does not vendor is loaded from a supplied
location"). The message reaches a merge only when no location could be learned, and there it
is true and is kept. Losing an unrelated message would be the worse failure.

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
   problem. The repository did not vendor the schema and `schema-cli-fallback` found it,
   but the file producer had no location for it. Once the refresh worker's locate pass has
   run, the file producer resolves such a schema too. This rule is then reached only when
   the two producers resolved different schema names for one change and the file
   producer's name could not be loaded. One example is a `.openspec.yaml` edited between
   the CLI's answer and the file walk.
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
