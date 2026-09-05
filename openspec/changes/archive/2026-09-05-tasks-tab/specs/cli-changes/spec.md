## ADDED Requirements

### Requirement: The CLI producer marks the same tracked-tasks artifact

`cli_artifacts` SHALL set each `ArtifactRef`'s `tracks_tasks` from the schema it was given
by exactly the rule `change-artifacts` specifies for the file producer: `true` at the first
index of `Schema::artifacts` equal to `Schema::tasks`, `false` at every other index, and
`false` everywhere when `Schema::tasks` is `None`.

The two producers SHALL agree by construction — one published rule applied to whichever
`Schema` value each producer resolved — rather than by coincidence. Where the two resolve
**different** schemas for the same change, which is the whole point of
`schema-cli-fallback`, they may legitimately mark different positions; `change-merge`
specifies which of the two lists survives.

`tracks_tasks` SHALL NOT be read from any CLI payload. Neither `openspec list --json` nor
`openspec instructions apply --change <n> --json` reports which artifact tracks the tasks:
`contextFiles` is an id-to-paths map with no such marker, and apply's own `progress` field
is already discarded by this capability for a different reason. The marker is a property of
the schema, and the schema is what both producers already hold.

#### Scenario: The CLI producer marks position 3 for the `tdd` schema

- **WHEN** `from_cli` builds a change whose resolved schema is `tdd` — `proposal`, `specs`,
  `design`, `tasks`, `planning-review`, with `apply.tracks: tasks.md` — from a
  `contextFiles` payload holding keys for `proposal` and `tasks`
- **THEN** the five `ArtifactRef`s carry `tracks_tasks` values `false`, `false`, `false`,
  `true`, `false` in that order
- **AND** position 3 also carries the `tasks` key's paths verbatim, so the marking did not
  disturb the placement

#### Scenario: Both producers mark the same position for the same schema

- **WHEN** the same schema value is given to `change_artifacts` and to `cli_artifacts` for
  the same change, for each of: the `tdd` schema; a schema whose `apply.tracks` names an
  artifact whose id is not `tasks`; a schema with no `apply` block and an artifact with id
  `tasks`; and a schema naming no tasks artifact at all
- **THEN** the two produced lists carry identical `tracks_tasks` values at every position,
  in all four cases
- **AND** the comparison is over the flag alone, since the two producers' `paths` legitimately
  differ — the CLI's are canonicalized and the file producer's are not

#### Scenario: A CLI-only schema still marks its own tasks artifact

- **WHEN** the repository vendors no schema for a change, so the file producer's artifact
  list is empty, and `schema-cli-fallback` supplies a schema declaring `alpha` and `tasks`
- **THEN** the CLI list holds two entries and position 1 carries `tracks_tasks == true`
- **AND** no panic occurs for the file producer's empty list

#### Scenario: A duplicate id in a CLI-resolved schema marks only its first position

- **WHEN** the artifact-placement function is called directly with a schema declaring ids
  `zeta`, `alpha`, `zeta`, both `zeta` entries generating `tasks.md`, with
  `apply.tracks: tasks.md`, and a `contextFiles` holding one entry for `zeta`
- **THEN** three `ArtifactRef`s are produced, positions 0 and 2 carrying that entry's paths
  as this capability already requires
- **AND** only position 0 carries `tracks_tasks == true`
