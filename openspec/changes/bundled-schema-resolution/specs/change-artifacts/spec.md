## ADDED Requirements

### Requirement: A schema the repository does not vendor is loaded from a supplied location

Every file-producer entry point that builds `Change` values SHALL take a read-only view of
the schema locations (`schema-cli-fallback` → "A located schema directory is remembered for
the worker's lifetime and shared with the file producer"). That covers the active and
archived tiers built by `from_files_with_listing`, and a worktree member's owned changes
built by `from_files_owned` and `overlay_family`.

When `schema::load(repo, name)` reports `LoadError::NotVendored` and the locations hold a
directory for `name`, the producer SHALL load the schema with
`schema::load_dir(<dir>, name)`. That is the same parser, the same per-enumeration cache by
name, and the same problem rendering every other load uses. On success the change's
`artifacts`, its tracked-tasks mark, and its task progress come from that schema exactly as
they would from a vendored one, and **no** "not vendored" problem is recorded. On failure,
the change records the one problem that `load_dir`'s error renders, naming the location's
directory and not the repository's `openspec/schemas/<name>`.

When the locations hold nothing for `name`, the producer SHALL record the "not vendored"
problem it records today. On every other repository-tier result (a vendored schema, an
unreadable or invalid vendored file, or an illegal name), the locations SHALL NOT be
consulted.

The producer SHALL invoke no process and SHALL write nothing, in the repository or in the
locations. A location only names where to read. Every byte, the archived tier's included, is
still read from disk, which is what keeps an archived change file-sourced as `change-model`
requires.

`changes::from_files(repo, scope)` SHALL keep its signature and supply no locations. A
caller with no `openspec` binary, which is file mode and every existing test, therefore sees
exactly the behaviour this capability already states, and SPEC.md's "Schema not vendored"
degraded-states row still holds there.

#### Scenario: An active change of a located schema has its artifacts and no problem

- **WHEN** a scratch repository declares `schema: spec-driven` in `openspec/config.yaml`,
  vendors nothing, and holds one active change `alpha`, and `from_files_with_listing` is
  called under `ArchivedScope::Names` with locations mapping `spec-driven` to a second scratch
  directory holding a real `schema.yaml` declaring `proposal`, `specs`, `design`, and `tasks`
- **THEN** `alpha`'s `artifacts` are those four ids in that order, with `tasks` marked as
  tracking tasks
- **AND** `alpha`'s `problems` is empty
- **AND** rendered through the detail region at 120 and at 60 columns, `alpha`'s tab bar
  lists the four ids and no `!`-marked row is drawn

#### Scenario: An archived change of a located schema has its artifacts

- **WHEN** the same repository and locations hold one archived change
  `2026-05-05-structured-player-actions` whose `tasks.md` counts 24 of 24, and
  `from_files_with_listing` is called under `ArchivedScope::Full`
- **THEN** that archived change carries the four artifact ids, progress 24 of 24, and no
  problem

#### Scenario: Without locations the not-vendored problem is unchanged

- **WHEN** the same repository is passed to `from_files(repo, ArchivedScope::Full)`
- **THEN** every change has an empty `artifacts` list and exactly one problem ending `is not
  vendored: no schema.yaml there` naming `<repo>/openspec/schemas/spec-driven/schema.yaml`
- **AND** rendered through the detail region, each change's tab bar reads `no artifacts` and
  one `!`-marked row names that path, which is the file-mode state SPEC.md's degraded-states
  table describes

#### Scenario: A location without a schema file names the location

- **WHEN** the locations map `spec-driven` to a directory that exists and holds no
  `schema.yaml`
- **THEN** `alpha` has an empty `artifacts` list and exactly one problem naming that
  directory, not `<repo>/openspec/schemas/spec-driven/schema.yaml`
- **AND** its task progress is still counted from `<change dir>/tasks.md`, as for any change
  whose schema did not load

#### Scenario: A vendored schema ignores the locations

- **WHEN** a repository vendors `openspec/schemas/tdd/schema.yaml` declaring five artifacts,
  its change declares `tdd`, and the locations map `tdd` to a directory whose `schema.yaml`
  declares two
- **THEN** the change carries the vendored schema's five artifact ids

#### Scenario: A worktree member's change of a located schema resolves

- **WHEN** a member worktree's own `openspec/config.yaml` declares `schema: spec-driven`,
  vendors nothing, and owns change `alpha`, and `from_files_owned` is called for it with
  locations mapping `spec-driven` to a directory holding a real `schema.yaml`
- **THEN** the member's `alpha` carries that schema's artifact ids and no problem containing
  `is not vendored`
