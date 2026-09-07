## REMOVED Requirements

### Requirement: The tasks artifact is the one `apply.tracks` selects, else the one with id `tasks`

**Reason**: Its wrong-typed-`tracks` clause was a stated divergence from the OpenSpec CLI
that made the two producers report different progress for the same change, which
`SPEC.md` -> Dual-source model forbids. The rule is replaced, under a name that says a
*present* `tracks` selects, by the requirement added below. The replacement is a rename
rather than a modification because one of its scenarios asserts the opposite outcome and
must not survive under its old name.

**Migration**: None at runtime - no interface, manifest, configuration, or keybinding
changes. The scenario `A ``tracks`` value of the wrong type falls back to the id` is
replaced by `A ``tracks`` value of the wrong type yields no tasks artifact`; every other
scenario is carried over unchanged.

## ADDED Requirements

### Requirement: The tasks artifact is the one a present `apply.tracks` selects, else the one with id `tasks`

The artifact holding a change's task checklist SHALL be identified by the schema's top-level
`apply:` block:

- when `apply.tracks` is **present and not null** — whatever its type — the tasks artifact is
  the one whose **`generates` equals that value**, and there is no further fallback. A
  non-string value equals no `generates` value, so a present-but-wrong-typed `tracks` selects
  nothing;
- when `apply` is absent, is not a mapping, or `apply.tracks` is absent or explicitly `null`,
  the tasks artifact is the one whose **id is `tasks`**;
- when neither rule finds one, the schema SHALL load with **no** tasks artifact and record a
  problem, rather than failing to load.

A `tracks` value that is present but not a string SHALL record exactly one problem naming the
wrong-typed key, and SHALL NOT fall back to the artifact with id `tasks`.

The first two clauses are the rule the OpenSpec CLI itself applies in
`findTrackedTasksArtifact`, which branches on `tracks != null` and so treats a sequence or a
number as a declaration that matches nothing. **`SPEC.md`'s claim that the artifact carries a
`role: tasks` key is false** — no `role` key exists in the schema format, in the vendored `tdd`
schema, in the CLI's bundled `spec-driven` schema, or in the CLI's own definition of an
artifact. `SPEC.md` was corrected by an earlier change.

The wrong-typed clause is **parity, not divergence**, and replaces an earlier deliberate
divergence that fell back to the id. Verified against `@fission-ai/openspec` 1.12.0: the CLI's
schema validator types `apply.tracks` as `relativePathSchema('apply.tracks').nullable()
.optional()` (`dist/core/artifact-graph/types.js:34`), so a wrong-typed value fails validation
and `resolveSchema` throws; `resolveTrackedTasksGlob` swallows the throw and returns
`undefined` (`dist/utils/task-progress.js:70-83`); and `getTaskProgressDetailForChange`
substitutes `[<changeDir>/tasks.md]` for the empty file list. Selecting no tasks artifact
reproduces that outcome exactly, because `change-artifacts` already requires the plugin to fall
back to counting `<change dir>/tasks.md` when the tasks-artifact file list is empty. The
earlier fallback did not: with a wrong-typed `tracks` and an artifact `{id: tasks, generates:
tasks/**/*.md}`, the plugin counted the boxes under `tasks/` and the CLI counted `tasks.md`,
so the two producers reported different progress for the same change — the one thing the
dual-source model forbids.

One clause still **diverges** from the CLI deliberately, and is stated as a divergence rather
than as parity so that no later change inherits a false claim about the CLI:

- **The third clause itself.** The CLI has no "no tasks artifact" state to degrade into,
  because it rejects such schemas at parse time; the plugin loads them and counts
  `<change dir>/tasks.md`, which is the same number the CLI reports.

The consequence is that the plugin's *usable* is strictly wider than the CLI's, so a schema
can load here and be rejected there. `changes-from-cli` must not assume that the plugin
having loaded a schema means the CLI will accept it — `SPEC.md` → Degraded states already
carries the reverse case ("Schema unknown to the CLI"), and this is its mirror.

Note that `apply.tracks` names the artifact's `generates` **filename**, which is not the
same thing as the artifact's id: on the vendored `tdd` schema both happen to select the
artifact `tasks`, which is precisely why the scenarios below use fixtures where the two
rules disagree.

#### Scenario: `apply.tracks` selects an artifact whose id is not `tasks`

- **WHEN** a fixture schema declares artifacts `checklist` (generates `tasks.md`) and
  `tasks` (generates `notes.md`), with `apply.tracks: tasks.md`
- **THEN** the tasks artifact is `checklist`
- **AND** it is **not** `tasks`, which is the assertion an id-first implementation fails
- **AND** both artifacts remain in the ordered list, because the tasks artifact is still a
  tab

#### Scenario: An absent `apply` block falls back to the artifact with id `tasks`

- **WHEN** a fixture schema declares artifacts `proposal` (generates `proposal.md`) and
  `tasks` (generates `checklist.md`) and has no `apply:` block at all
- **THEN** the tasks artifact is `tasks`, whose `generates` is `checklist.md`
- **AND** no problem is recorded

#### Scenario: An `apply` block without `tracks`, and an explicit `tracks: null`, both fall back to the id

- **WHEN** a fixture schema declares an artifact with id `tasks` and an `apply:` block
  carrying only `requires`, and the same fixture is run again with `tracks: null` added
- **THEN** both runs select the artifact with id `tasks`
- **AND** the explicit-null run is asserted alongside the absent-key run in one test,
  because the two differ in the parse tree and an implementation branching on "the key is
  present" behaves differently for them
- **AND** a third run in the same test, with `apply:` itself a bare scalar rather than a
  mapping, also selects the artifact with id `tasks` without panicking — indexing a scalar
  node is the failure the sibling *invalid* requirement worries about, reached here from
  the other direction

#### Scenario: A `tracks` value matching nothing yields no tasks artifact even when an id `tasks` exists

- **WHEN** a fixture schema declares artifacts `tasks` (generates `tasks.md`) and `design`
  (generates `design.md`), with `apply.tracks: nowhere.md`
- **THEN** the schema loads, both artifacts are in the list, and there is **no** tasks
  artifact
- **AND** exactly one problem is recorded naming `nowhere.md`
- **AND** it is not `tasks` — an implementation that falls back to the id whenever the
  `tracks` lookup misses fails this scenario, and that fallback is exactly what the CLI's
  `find` does not do

#### Scenario: A schema with neither a `tracks` match nor an id `tasks` loads without one

- **WHEN** a fixture schema declares only `proposal` and `design` and has no `apply:` block
- **THEN** the schema loads with both artifacts and no tasks artifact
- **AND** exactly one problem is recorded
- **AND** the artifact list is still usable, because a schema with no task checklist is a
  degraded tab bar and not an error screen

#### Scenario: `tracks` matches on `generates`, not on a filename suffix

- **WHEN** a fixture schema declares artifacts `a` (generates `sub/tasks.md`) and `b`
  (generates `tasks.md`), with `apply.tracks: tasks.md`
- **THEN** the tasks artifact is `b`
- **AND** it is not `a`, so an implementation comparing only the final path component fails

#### Scenario: A `tracks` value of the wrong type yields no tasks artifact

- **WHEN** a fixture schema carries `apply.tracks` as a sequence rather than a string, and
  declares an artifact with id `tasks` generating `tasks.md`
- **THEN** the schema loads, the artifact list is unchanged, and there is **no** tasks
  artifact
- **AND** exactly one problem is recorded naming the wrong-typed key
- **AND** the same holds when `tracks` is the integer `42` and when it is a mapping, all
  three asserted in one test, because a `Yaml` node's type is the only thing that differs
  between them
- **AND** no artifact is selected, and in particular not `tasks` — the assertion the
  previous id-fallback implementation fails and this one passes

#### Scenario: A wrong-typed `tracks` counts `tasks.md`, the same pair the CLI reports

This scenario asserts `change-artifacts`' `<change dir>/tasks.md` progress fallback from
this capability's side, because the parity claim above is about the *pair the CLI reports*
and is meaningless without it. It is verified in `changes`' tests, not `schema`'s.

- **WHEN** a change's schema carries `apply.tracks: 42` and an artifact `{id: tasks,
  generates: tasks/**/*.md}`, and the change directory holds `tasks/a.md` with two checked
  boxes, `tasks/b.md` with two unchecked boxes, and no `tasks.md`
- **THEN** the change's `progress` is `Progress { completed: 0, total: 0 }`, reached through
  `change-artifacts`' `<change dir>/tasks.md` fallback
- **AND** it is **not** `Progress { completed: 2, total: 4 }`, which is what counting the
  `tasks/` files would give and what `openspec list --json` does not report for this schema
- **AND** no `ArtifactRef` in the change carries `tracks_tasks`, which is the field the
  detail region reads to decide whether a tab renders the checklist grammar

### Requirement: A schema name is rejected before it is joined into a filesystem path

`schema::load(repo, name)` SHALL check `name` with the same `is_legal_name` guard
`schema-selection` applies to a declared name, **before** joining it into
`<repo>/openspec/schemas/<name>`. A name the guard rejects SHALL yield a distinct
`LoadError` variant naming the offending value, and no path SHALL be constructed from it,
no directory read, and no file opened.

The guard SHALL be inside `schema::load` rather than only at each call site. It exists
solely to protect this one join, and a guard applied by one caller and not another is how
the asymmetry arises: the file-sourced name is checked in `source_contribution` while the
CLI-sourced `schemaName` reaches the same join unchecked.

The new variant SHALL be distinct from `NotVendored`. `schema-cli-fallback` repairs
`NotVendored` by asking `openspec schema which`, and asking the CLI about a name that
cannot be a directory segment is a pointless spawn whose answer could only be an error.

`schema::load_dir(dir, name)` SHALL NOT apply the guard: it joins only `schema.yaml` onto a
directory it is handed, and uses `name` for the declared-`name:` comparison alone.

#### Scenario: A traversing name is rejected before any filesystem access

- **WHEN** `schema::load` is called with `name` `../../../../etc` against a scratch
  repository
- **THEN** it returns the illegal-name error naming `../../../../etc`
- **AND** the error is **not** `NotVendored`, which is what an unguarded `load` returns for
  the same input, so the variant is the falsifier rather than any absence-of-I/O claim
- **AND** the scratch tree is byte-identical afterwards, asserted by snapshotting it, on the
  same terms as the sibling read-only requirement

#### Scenario: Every shape `is_legal_name` rejects is rejected here too

- **WHEN** `schema::load` is called in turn with `""`, `"   "`, `"."`, `".."`, `"a/b"`,
  `"a\\b"`, an absolute path, and a name containing a NUL byte
- **THEN** each returns the illegal-name error, and none returns `NotVendored`, which is the
  answer an unguarded `load` gives for at least `""` and `"   "`
- **AND** the legal name `spec-driven.v2`, which contains a dot but is not `.` or `..`, is
  **not** rejected and reaches the normal not-vendored answer, so the guard is not merely
  rejecting everything

#### Scenario: A legal name still loads exactly as before

- **WHEN** `schema::load` is called with `tdd` against this repository
- **THEN** the vendored `tdd` schema loads with its artifacts in file order and the guard is
  invisible in the result
