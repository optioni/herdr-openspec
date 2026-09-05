# change-artifacts Specification

## Purpose

Turning one change directory into the artifact paths a tab renders and the task count a
list row shows. Four already-landed capabilities meet here: `schema-selection` says which
schema applies to this change, `schema-artifacts` says which artifacts that schema declares
and which of them holds the tasks, `task-checkboxes` says which lines of a file count, and
`task-groups` reads a task file without writing to it. What is missing is the join —
resolving each artifact's `generates` value against this change's directory, and
summing the tasks artifact's files into one `Progress` by the same rule the OpenSpec CLI
uses, so the list row does not change its number when the CLI result lands.

## Requirements

### Requirement: Each change resolves its own schema, and the project file is read once

The plugin SHALL determine the schema for each change by the published `schema-selection`
ordering — the change's own `.openspec.yaml`, then the repository's
`openspec/config.yaml`, then the default `spec-driven` — so that a repository holding
changes under two schemas renders both correctly.

The repository's `openspec/config.yaml` SHALL be read from disk once per enumeration and its
contents reused for every change, rather than re-read per change. The already-published
`schema::read_file` and `schema::declared_name` exist in this crate for exactly that reason.
A loaded schema SHALL likewise be cached by name for the duration of one enumeration, in a
value owned by the call rather than a process-global, matching `resolve::BinCache`'s reason:
the test suite runs in parallel threads of one process and a shared cache would let the
first call decide the answer for all of them.

Problems recorded while selecting and loading a schema SHALL be recorded on the owning
change, not on the set, because a schema is a per-change fact.

#### Scenario: A change's own declaration wins over the project's

- **WHEN** a repository whose `openspec/config.yaml` declares `schema: tdd` vendors **both**
  `openspec/schemas/tdd/` and `openspec/schemas/probe/`, with artifact lists that differ, and
  holds `changes/a/` with no `.openspec.yaml` and `changes/b/.openspec.yaml` declaring
  `schema: probe`
- **THEN** change `a` has `schema` `tdd` and change `b` has `schema` `probe`
- **AND** each change's `artifacts` holds its own schema's ids, so the two lists differ —
  an implementation that resolves the name per change but loads one schema for the whole
  repository passes a `schema`-only assertion and fails this one

#### Scenario: A repository declaring no schema falls back to the default

- **WHEN** a repository has no `openspec/config.yaml`, a change with no `.openspec.yaml`,
  and vendors only `openspec/schemas/tdd/`
- **THEN** the change's `schema` is `spec-driven`
- **AND** its `artifacts` list is empty and one problem names `spec-driven` as not
  vendored, which is the state `changes-from-cli` later repairs by asking the CLI where
  that schema lives

#### Scenario: A schema that is not vendored leaves the artifact list empty

- **WHEN** a change declares `schema: outside-in-tdd`, which the repository does not vendor
- **THEN** the change's `artifacts` is empty and `problems` holds one entry naming the
  schema and the reason
- **AND** the change is still listed with its name, directory, and a task progress, rather
  than being dropped

### Requirement: Artifact paths come from `generates`, never from the artifact id

The plugin SHALL resolve each schema artifact's concrete paths by joining its `generates`
value onto the change directory, and SHALL NOT construct a path from the artifact's `id`.

`<id>.md` for a file artifact and `<id>/` for a directory artifact is what `generates`
happens to reduce to for the vendored `tdd` schema, where every artifact's filename is its
id. It is not the rule: a schema declaring `id: plan` with `generates:
implementation-plan.md` would resolve to a file that does not exist, and the tab would read
"No content yet" for an artifact that is written. `SPEC.md` is corrected accordingly by this
change.

A `generates` value reaching this capability is already guaranteed by `schema-artifacts` to
be non-blank, relative, free of `..` segments, and free of NUL bytes; artifacts failing that
check were skipped before the schema was returned. No path this capability builds SHALL
escape the change directory.

#### Scenario: An artifact whose filename differs from its id resolves correctly

- **WHEN** a change's schema declares an artifact `id: plan` with `generates:
  implementation-plan.md`, and the change directory holds `implementation-plan.md`
- **THEN** the `plan` `ArtifactRef` carries the one path ending `implementation-plan.md`
- **AND** no path ending `plan.md` is produced

#### Scenario: A non-glob `generates` naming something that is not a regular file resolves to nothing

- **WHEN** a schema declares `generates: notes` and the change directory holds a
  *directory* named `notes`
- **THEN** the `ArtifactRef` carries no paths
- **AND** no problem is recorded, matching the OpenSpec CLI, which resolves a non-glob
  pattern by testing for a regular file and yields nothing otherwise

#### Scenario: A symbolic link to a regular file is a resolved artifact

- **WHEN** a change directory holds `proposal.md` as a symbolic link pointing at a readable
  regular file
- **THEN** the `proposal` `ArtifactRef` carries the path as constructed, not the link's
  target
- **AND** a `proposal.md` that is a dangling symbolic link resolves to no paths and records
  no problem

### Requirement: A `generates` value is a glob exactly when it contains `*`, `?`, or `[`

The plugin SHALL classify a `generates` value as a glob if and only if it contains one of
the characters `*`, `?`, or `[`, reproducing the OpenSpec CLI's own `isGlobPattern` test
character for character.

The consequence worth stating is the one that surprises: `{` is **not** in that set, so a
value such as `specs/{alpha,zeta}/spec.md` is treated as a literal relative path, is tested
for being a regular file under that exact name, and resolves to nothing. That is what the
CLI does, and reproducing it is what keeps a schema author's mistake looking the same in
both tools rather than only in one.

#### Scenario: A plain filename is not a glob

- **WHEN** `generates` is `tasks.md`
- **THEN** it is resolved as a single path and not as a pattern

#### Scenario: A brace expression is not a glob and resolves to nothing

- **WHEN** `generates` is `specs/{alpha,zeta}/spec.md` and the change directory holds
  `specs/alpha/spec.md` and `specs/zeta/spec.md`
- **THEN** the `ArtifactRef` carries no paths
- **AND** the result is identical to the OpenSpec CLI's for the same tree

#### Scenario: Each of the three metacharacters makes a value a glob

- **WHEN** `generates` is `specs/**/*.md`, `notes/file?.md`, or `notes/[ab].md`
- **THEN** each is classified as a glob rather than as a literal path

### Requirement: A supported glob resolves to every matching regular file, in ascending path order

The plugin SHALL resolve a glob `generates` value over the change directory when its shape
falls inside a deliberately small supported subset:

- every directory segment is a literal containing none of `*`, `?`, `[`, except that the
  **last** directory segment may be exactly `**`; and
- the final segment is either a literal filename, or a literal prefix, exactly one `*`, and
  a literal suffix, with either part possibly empty.

`**` SHALL match zero or more directory levels, so `specs/**/*.md` matches a file directly
inside `specs/` as well as one nested below it — the behaviour of the CLI's own matcher.

Resolution SHALL yield regular files only, tested through symbolic links. It SHALL skip
every entry whose name begins with `.`, matching the CLI's matcher, whose dot option is left
at its default. It SHALL NOT descend into a directory reached through a symbolic link, so
no walk can loop; the CLI instead detects a linked cycle and throws, which this plugin may
not do because it never fails closed. Results SHALL be ordered by full path ascending in
byte order and SHALL contain no duplicate.

#### Scenario: A nested spec tree resolves in path order

- **WHEN** `generates` is `specs/**/*.md` and the change directory holds
  `specs/Beta/spec.md`, `specs/alpha/spec.md`, `specs/alpha/nested/deep.md`, and
  `specs/top.md`
- **THEN** the `ArtifactRef` carries all four paths, ordered `specs/Beta/spec.md`,
  `specs/alpha/nested/deep.md`, `specs/alpha/spec.md`, `specs/top.md`
- **AND** the order is byte order on the full path, so `Beta` precedes `alpha`

#### Scenario: A glob matching nothing is an empty path list, not a problem

- **WHEN** `generates` is `specs/**/*.md` and the change directory holds no `specs/`
  directory at all
- **THEN** the `ArtifactRef` carries no paths and records no problem
- **AND** the same holds when `specs/` exists and is empty

#### Scenario: Dot entries and non-files are skipped

- **WHEN** `generates` is `specs/**/*.md` and `specs/` holds `.hidden.md`, a
  `.hidden-cap/spec.md`, a directory named `looks-like.md`, and one real `alpha/spec.md`
- **THEN** only `specs/alpha/spec.md` is carried
- **AND** no problem is recorded for any of the skipped entries

#### Scenario: A directory symbolic link is not descended into

- **WHEN** `generates` is `specs/**/*.md` and `specs/` holds a real `alpha/spec.md` and a
  symbolic link `loop/` pointing back at `specs/`
- **THEN** only `specs/alpha/spec.md` is carried, and the call terminates
- **AND** a file reached only through the link is absent, which is a knowing divergence
  from the CLI's matcher recorded in design.md and corrected by the CLI path

#### Scenario: A prefix-and-suffix file pattern is supported

- **WHEN** `generates` is `specs/spec-*.md` and `specs/` holds `spec-a.md`, `spec-b.md`, and
  `other.md`
- **THEN** the `ArtifactRef` carries `specs/spec-a.md` and `specs/spec-b.md` in that order

### Requirement: An unsupported glob shape records a problem and resolves to no files

A glob `generates` value whose shape falls outside the supported subset SHALL yield no paths
and SHALL record exactly one problem on the owning change, naming the artifact id and the
pattern verbatim.

Degrading visibly rather than guessing is the point. Two of the unsupported shapes resolve
to nothing in the OpenSpec CLI as well — a `?` in a directory segment is one — and for the
rest the CLI's answer is a superset the CLI path supplies when it arrives. Silently
producing a wrong file list for a tasks artifact would put a task count on screen that the
CLI then changes, which is the one symptom the dual-source model exists to avoid.

#### Scenario: A wildcard inside a directory segment is unsupported

- **WHEN** `generates` is `specs/*/spec.md`, `specs/?eta/spec.md`, or `specs/[az]*/spec.md`,
  over a tree that holds `specs/zeta/spec.md`
- **THEN** the `ArtifactRef` carries no paths and the change records one problem naming the
  artifact and the pattern
- **AND** for `specs/?eta/spec.md` the empty result matches the OpenSpec CLI's own, which
  also resolves it to nothing

#### Scenario: More than one wildcard in the filename segment is unsupported

- **WHEN** `generates` is `specs/*-*.md`
- **THEN** no paths are produced and one problem names the artifact and the pattern

#### Scenario: A `**` that is not the last directory segment is unsupported

- **WHEN** `generates` is `specs/**/nested/*.md`, or `specs/**/nested/**/*.md`
- **THEN** no paths are produced and one problem names the artifact and the pattern
- **AND** the change is still listed with every other artifact resolved normally

### Requirement: Task progress sums the tasks artifact's files and falls back to `tasks.md`

The plugin SHALL compute a change's `progress` by resolving the schema's tasks artifact to a
file list, counting each file with the published `task-checkboxes` rule, and summing the
results.

When that file list is **empty** — because the schema declares no tasks artifact, because
the schema did not load at all, or because the artifact's glob matched nothing — the plugin
SHALL fall back to the single path `<change dir>/tasks.md` and count that.

The fallback is not a convenience; it is the OpenSpec CLI's own behaviour, and omitting it
would make the file path report `0/0` for a change the CLI reports `4/9` for. Its own task
counter builds the target list, substitutes `[<changeDir>/tasks.md]` when that list is
empty, and sums, swallowing every schema error on the way.

An unreadable task file SHALL contribute zero to the sum and record its problem on the
change, exactly as `task-groups` specifies; an absent one SHALL contribute zero and
record nothing.

#### Scenario: A single tasks file gives the change's progress

- **WHEN** a change under the `tdd` schema, whose tasks artifact generates `tasks.md`, holds
  a `tasks.md` with four checked and five unchecked task lines
- **THEN** `progress == Progress { completed: 4, total: 9 }`
- **AND** the same pair is what `openspec list --json` reports for that change

#### Scenario: A glob-shaped tasks artifact sums across its files

- **WHEN** a change's schema declares a tasks artifact generating `**/tasks.md`, and the
  change directory holds `tasks.md` at 1/2, `sub/tasks.md` at 2/3, and
  `sub/deeper/tasks.md` at 0/1
- **THEN** `progress == Progress { completed: 3, total: 6 }`
- **AND** the sum is over the resolved file list, in the same way the CLI accumulates its
  own

#### Scenario: A schema naming no tasks artifact still counts `tasks.md`

- **WHEN** a change's schema declares artifacts but no `apply.tracks` and no artifact with
  id `tasks`, and the change directory holds a `tasks.md` with one checked and one
  unchecked line
- **THEN** `progress == Progress { completed: 1, total: 2 }`
- **AND** the same holds when the schema failed to load entirely, and when the tasks
  artifact's glob matched no file

#### Scenario: A change with no tasks file at all is zero, not complete

- **WHEN** a change directory holds no `tasks.md` and its schema's tasks artifact resolves
  to nothing
- **THEN** `progress == Progress { completed: 0, total: 0 }` and `progress.is_complete()`
  is false
- **AND** no problem is recorded, because an unwritten task file is the normal state of a
  change that has not reached implementation

#### Scenario: An unreadable tasks file is zero plus one named problem

- **WHEN** a change directory holds a *directory* named `tasks.md`
- **THEN** `progress == Progress { completed: 0, total: 0 }` and the change records exactly
  one problem naming the path
- **AND** the change is still listed, with every other artifact resolved normally

### Requirement: Artifact resolution and counting read and never write

Resolving an artifact's paths and counting a change's tasks SHALL open no file for writing,
create no directory, and change no modification time, including for the directories a glob
walk descends into and the files it counts.

#### Scenario: A change directory is byte-identical after resolution and counting

- **WHEN** a change directory holding a nested `specs/` tree, a `tasks.md`, and an
  unreadable artifact is resolved and counted, with a snapshot of every directory entry,
  every file's bytes, and every modification time taken immediately before and after
- **THEN** the two snapshots compare equal
- **AND** a `tasks.md` that did not exist before the count still does not exist afterwards,
  even though the fallback named that exact path

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
