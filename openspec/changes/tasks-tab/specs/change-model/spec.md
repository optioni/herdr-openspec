## MODIFIED Requirements

### Requirement: A change's artifacts are the schema's, in the schema's order, resolved to paths

The plugin SHALL represent each of a change's artifacts as an `ArtifactRef` carrying the
schema artifact's `id`, the concrete `paths` that artifact resolves to on disk, and
`tracks_tasks` — whether this position is the one the schema's tasks-artifact rule names — in
the order the schema declares its artifacts.

`paths` SHALL be empty when nothing is written yet. An empty `paths` is the "No content
yet" state of `SPEC.md` → Degraded states, not an error and not an omitted tab: the tab is
built from the schema's artifact list and is shown whether or not its file exists. That
holds for a marked position too: a marked artifact with no resolved file reads
`No content yet`, not a checklist and not `No tasks yet`.

`tracks_tasks` SHALL be a derived marker, not a second source of truth: it is `true` at the
first index equal to `Schema::tasks` and `false` everywhere else, per `change-artifacts` for
the file producer and `cli-changes` for the CLI producer. At most one entry per change SHALL
carry `true`. It is on `ArtifactRef` rather than on `Change` so the positional artifact join
`change-merge` specifies carries it with no rule of its own, and so `Change`'s seven fields
are unchanged.

`ArtifactRef` SHALL NOT carry the artifact's `generates` value, its description, or its
rendered content. The schema is the owner of the first two and every consumer of a `Change`
already holds it; the third is `markdown-viewer`'s, read at render time. `tracks_tasks` is
not an exception to that rule: it is one bit of the schema's *decision*, not the schema's
data, and a consumer that held the `Schema` could not recompute it without re-deriving a
rule that already ran.

Artifact ids SHALL NOT be assumed unique. The landed `schema-artifacts` capability requires
a schema's artifact list to be kept verbatim and "never de-duplicated", so a schema
declaring the same id twice produces two `ArtifactRef`s carrying it. The consequence is a
contract for Phase 3: a consumer joining this list against the CLI's `contextFiles` joins by
**position** in the schema's declared order, which both producers reproduce, and never by
id, which is not a key. The same reasoning is why `tracks_tasks` marks a **position** rather
than an id.

#### Scenario: Tab order follows the schema, not the filesystem

- **WHEN** `from_files` reads a change under the `tdd` schema whose only written artifacts
  are `tasks.md` and `proposal.md`
- **THEN** `artifacts` holds five `ArtifactRef`s with ids `proposal`, `specs`, `design`,
  `tasks`, `planning-review` in that order
- **AND** `proposal` and `tasks` each carry one path, and `specs`, `design`, and
  `planning-review` each carry none
- **AND** the `tasks` entry carries `tracks_tasks == true` and the other four `false`

#### Scenario: A missing artifact file is an empty path list, not a missing tab

- **WHEN** a change directory holds no `design.md`
- **THEN** the `design` `ArtifactRef` is present with an empty `paths`
- **AND** no problem is recorded, because an unwritten artifact is the normal state of a
  change in flight
- **AND** the same holds for a change directory holding no `tasks.md`: the `tasks` entry is
  present, carries an empty `paths`, and still carries `tracks_tasks == true`, because the
  marker is a property of the schema and not of what is written
