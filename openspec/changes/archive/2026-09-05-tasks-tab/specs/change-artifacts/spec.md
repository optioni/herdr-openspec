## ADDED Requirements

### Requirement: The tracked-tasks artifact is marked at its schema position

`ArtifactRef` SHALL carry a third field, `tracks_tasks: bool`, and `change_artifacts` SHALL
set it `true` at exactly the position of the schema's tasks artifact and `false` at every
other position.

That position SHALL be the **first** index of `Schema::artifacts` whose entry equals
`Schema::tasks` — the artifact `schema-artifacts` already selected by the published rule:
the artifact whose `generates` equals the schema's top-level `apply.tracks`, falling back to
the artifact with id `tasks` when no `apply` block declares what it tracks, and yielding
none when a `tracks` value matches nothing. The plugin SHALL NOT re-derive that rule here
and SHALL NOT identify the artifact by comparing an id to the string `tasks` or a resolved
path's filename to `tasks.md`: a schema declaring `id: checklist` with
`generates: tasks.md` is the case both of those get wrong, and the OpenSpec CLI's own
`findTrackedTasksArtifact` takes the first match, which is why "first index" is normative
rather than incidental.

When `Schema::tasks` is `None`, or when the schema failed to load at all, **every**
`ArtifactRef` SHALL carry `tracks_tasks == false`. No artifact SHALL be marked as a
consolation: the change's `progress` still falls back to counting `<change dir>/tasks.md`,
as this capability already requires, but that fallback SHALL NOT mark a tab, because the
counted file is not an artifact the schema declares and no tab addresses it.

At most one `ArtifactRef` per change SHALL carry `tracks_tasks == true`, including for a
schema declaring the same artifact twice.

`Change`'s own seven fields SHALL be unchanged: this field lives on `ArtifactRef`, not on
`Change`, and `cli-changes`' "every one of `Change`'s seven fields comes from CLI data"
requirement is untouched by it.

#### Scenario: The `tdd` schema marks its `tasks` artifact and nothing else

- **WHEN** a change under the vendored `tdd` schema — artifacts `proposal`, `specs`,
  `design`, `tasks`, `planning-review`, with `apply.tracks: tasks.md` — is built from files
- **THEN** the `ArtifactRef` at position 3 carries `tracks_tasks == true`
- **AND** the four others carry `tracks_tasks == false`
- **AND** exactly one artifact in the list carries `true`

#### Scenario: `apply.tracks` wins over an artifact whose id is `tasks`

- **WHEN** a change's schema declares `id: checklist, generates: tasks.md` at position 0 and
  `id: tasks, generates: notes.md` at position 1, with `apply.tracks: tasks.md`
- **THEN** position 0 carries `tracks_tasks == true` and position 1 carries `false`
- **AND** the marked artifact's own `paths` still come from its `generates` value, so the
  tab renders `tasks.md` while being addressed as `checklist`

#### Scenario: An absent `apply` block falls back to the id `tasks`

- **WHEN** a change's schema declares `proposal` and `tasks` with no `apply:` block at all
- **THEN** the `tasks` position carries `tracks_tasks == true` and `proposal` carries
  `false`

#### Scenario: A schema naming no tasks artifact marks nothing

- **WHEN** a change's schema declares `alpha` and `beta`, has `apply.tracks: nothing.md`
  matching neither, and declares no artifact with id `tasks`
- **THEN** both `ArtifactRef`s carry `tracks_tasks == false`
- **AND** the change's `progress` still counts `<change dir>/tasks.md` by this capability's
  existing fallback, so the marking and the counting are decided separately
- **AND** the same holds when the schema failed to load entirely, where the artifact list is
  empty and therefore carries no marked entry

#### Scenario: A duplicate schema entry marks only its first position

- **WHEN** a change's schema declares ids `zeta`, `alpha`, `zeta` where both `zeta` entries
  carry the identical `generates: tasks.md`, and `apply.tracks: tasks.md`
- **THEN** position 0 carries `tracks_tasks == true` and positions 1 and 2 carry `false`
- **AND** the list is still three entries long and is neither de-duplicated nor truncated,
  as `schema-artifacts` requires

#### Scenario: A change with no artifacts carries no marked entry

- **WHEN** a change whose schema declares an empty artifact list is built from files
- **THEN** its `artifacts` is empty and no panic occurs while looking for a marked position
- **AND** its `progress` is still counted from `<change dir>/tasks.md`
