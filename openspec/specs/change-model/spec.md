# change-model Specification

## Purpose

The value every consumer of this plugin reads. A `Change` is one unit of OpenSpec work
reduced to what a dashboard needs — its name, where it lives, whether it is active or
archived, which schema applies to it, where each of its artifacts is on disk, how many of
its tasks are done, and everything that went wrong producing those answers.

The capability exists because there are **two** producers of that value and there will
never be one: `changes::from_files` walks the repository and paints the pane immediately,
and `changes::from_cli` (Phase 3) parses `openspec` JSON and corrects it a few hundred
milliseconds later. A field that one producer fills and the other silently leaves at a
default is not a cosmetic defect — it is a number or a path that changes by itself after
the pane has already been read. This capability therefore specifies the type *and* the
mechanism that makes such a field fail to compile rather than fail in a pane.

## Requirements

### Requirement: A change is one total value carrying identity, artifacts, and progress

The plugin SHALL represent one OpenSpec change as a `Change` value carrying exactly seven
fields: its `name`, the `dir` it lives in, its `origin` (active, or archived with the date
split off its directory name), the `schema` name that applies to it, its `artifacts` in the
schema's declared order, its task `progress`, and the `problems` recorded while producing
it.

Every field SHALL be total — a value the producer can always supply. `progress` is a
`Progress`, never an `Option<Progress>`, because the OpenSpec CLI reports a completed/total
pair for every change it lists, including one whose schema names no tasks artifact at all.
`artifacts` and `problems` are possibly-empty vectors rather than options, and `schema` is
always a name because schema selection is itself total.

`Change` SHALL NOT derive `Default`, and no field SHALL be filled in by a default. Rust
requires every field of a struct literal to be named unless a functional-update expression
supplies the rest, so the absence of `Default` is what makes adding a field a compile error
at every producer rather than a silent `None` at one of them.

#### Scenario: A fully written active change becomes one value

- **WHEN** `from_files` reads a repository whose `openspec/changes/add-auth/` holds
  `.openspec.yaml` declaring schema `tdd`, `proposal.md`, `design.md`, and a `tasks.md`
  holding four checked and five unchecked task lines
- **THEN** the resulting `Change` has `name` `add-auth`, `dir` ending
  `openspec/changes/add-auth`, `origin` `Active`, `schema` `tdd`, and
  `progress == Progress { completed: 4, total: 9 }`
- **AND** its `artifacts` are the `tdd` schema's five artifact ids in schema order, the
  three written ones carrying one path each and the two unwritten ones carrying none
- **AND** `problems` is empty

#### Scenario: An archived change carries the date split off its directory name

- **WHEN** `from_files` reads a repository holding
  `openspec/changes/archive/2026-08-14-add-auth/` with a `tasks.md` of three checked lines
- **THEN** the resulting `Change` has `name` `add-auth`, `origin`
  `Archived { date: Some("2026-08-14") }`, and `dir` ending
  `openspec/changes/archive/2026-08-14-add-auth`
- **AND** `progress == Progress { completed: 3, total: 3 }`, so `progress.is_complete()`
  is true

#### Scenario: A change whose schema did not load is still a complete value

- **WHEN** `from_files` reads a change whose `.openspec.yaml` declares schema
  `outside-in-tdd`, which is not vendored under `openspec/schemas/`, and whose directory
  holds a `tasks.md` with two checked and one unchecked task line
- **THEN** the `Change` has `schema` `outside-in-tdd`, an empty `artifacts` list, and
  `progress == Progress { completed: 2, total: 3 }` counted from `<dir>/tasks.md`
  through the fallback, rather than the `0/0` an implementation without the fallback
  produces
- **AND** `problems` holds one entry naming the schema and the reason it did not load
- **AND** no field is absent, no `Result` is returned, and nothing panics

### Requirement: `Change` carries no derived and no source-specific state

`Change` SHALL NOT carry a `status`, a completion ratio, a formatted progress string, a
last-modified timestamp, or a discriminant naming which producer built it.

`status` is excluded because `openspec list --json` computes it from the pair
(`totalTasks == 0` → `no-tasks`, `completedTasks == totalTasks` → `complete`, otherwise
`in-progress`) and `Progress::is_complete` already mirrors that split; a stored copy is a
second source of truth that one producer can get wrong. A last-modified timestamp is
excluded because no specified view renders one, and reproducing the CLI's recursive
maximum-mtime walk would be a divergence risk taken for nothing. A producer discriminant is
excluded because it would make it legal for a field to mean something different depending
on who built the value, which is exactly the divergence this capability exists to prevent.

#### Scenario: The three-way status split is derived, not stored

- **WHEN** three `Change` values carry `Progress { 0, 0 }`, `Progress { 3, 3 }`, and
  `Progress { 1, 3 }`
- **THEN** applying the CLI's own rule to each — `total == 0` is `no-tasks`,
  `is_complete()` is `complete`, otherwise `in-progress` — yields `no-tasks`, `complete`,
  and `in-progress` respectively
- **AND** a source inspection of the `Change` definition finds no field named `status`, no
  ratio, no percentage, no formatted string, no `lastModified`, and no producer
  discriminant, so the split has exactly one source of truth

#### Scenario: The same repository read twice produces equal values, even after a touch

- **WHEN** `from_files` reads a repository, an unrelated file inside one change directory
  has its modification time advanced, and `from_files` reads it again
- **THEN** the two `ChangeSet` values compare equal with `==`
- **AND** the second read is byte-for-byte the same value as the first, which a `Change`
  carrying a `lastModified` field could not be — that is the assertion, since a plain
  double read of an untouched tree would pass for any deterministic implementation

### Requirement: A change's artifacts are the schema's, in the schema's order, resolved to paths

The plugin SHALL represent each of a change's artifacts as an `ArtifactRef` carrying the
schema artifact's `id` and the concrete `paths` that artifact resolves to on disk, in the
order the schema declares its artifacts.

`paths` SHALL be empty when nothing is written yet. An empty `paths` is the "No content
yet" state of `SPEC.md` → Degraded states, not an error and not an omitted tab: the tab is
built from the schema's artifact list and is shown whether or not its file exists.

`ArtifactRef` SHALL NOT carry the artifact's `generates` value, its description, or its
rendered content. The schema is the owner of the first two and every consumer of a `Change`
already holds it; the third is `markdown-viewer`'s, read at render time.

Artifact ids SHALL NOT be assumed unique. The landed `schema-artifacts` capability requires
a schema's artifact list to be kept verbatim and "never de-duplicated", so a schema
declaring the same id twice produces two `ArtifactRef`s carrying it. The consequence is a
contract for Phase 3: a consumer joining this list against the CLI's `contextFiles` joins by
**position** in the schema's declared order, which both producers reproduce, and never by
id, which is not a key.

#### Scenario: Tab order follows the schema, not the filesystem

- **WHEN** `from_files` reads a change under the `tdd` schema whose only written artifacts
  are `tasks.md` and `proposal.md`
- **THEN** `artifacts` holds five `ArtifactRef`s with ids `proposal`, `specs`, `design`,
  `tasks`, `planning-review` in that order
- **AND** `proposal` and `tasks` each carry one path, and `specs`, `design`, and
  `planning-review` each carry none

#### Scenario: A missing artifact file is an empty path list, not a missing tab

- **WHEN** a change directory holds no `design.md`
- **THEN** the `design` `ArtifactRef` is present with an empty `paths`
- **AND** no problem is recorded, because an unwritten artifact is the normal state of a
  change in flight

### Requirement: Both producers of a change build every field, enforced at compile time

The plugin SHALL keep `changes::from_files` and `changes::from_cli` producing the same
`Change` by three mechanisms, none of which is a comment:

1. **No `Default`.** Neither `Change` nor `ArtifactRef` derives or implements `Default`, and
   no producer uses a functional-update expression, so every construction site names every
   field and a new field is a compile error at each of them.
2. **A shared conformance function that destructures exhaustively.** A test-only
   `changes::conformance::assert_invariants(&Change)` SHALL open with an exhaustive `let
   Change { … }` pattern carrying **no** `..` rest pattern, so adding a field makes that one
   function fail to compile. Because it is the function *both* producers' test suites call,
   whoever adds a field is forced to state what the new field means for both sources rather
   than for the one they were working on.
3. **Every producer's tests call it.** Every `Change` a producer's test builds SHALL be
   passed through `assert_invariants`, which checks the source-independent invariants: a
   non-empty `name`; a non-empty `schema`; a `dir` whose final component equals `name` for
   an `Active` change and ends with `name` for an `Archived` one; an `ArtifactRef` with a
   non-empty `id` for every entry; and `problems` holding no empty string, so it is used
   only for human-readable text and never to carry data a field should hold. It SHALL NOT
   assert that artifact ids are unique — the landed `schema-artifacts` capability requires
   that a schema's artifact list is "never de-duplicated", so a schema declaring the same
   id twice legitimately produces two `ArtifactRef`s with one id.

The plugin SHALL NOT satisfy this requirement with a `#[non_exhaustive]` attribute, which
constrains only other crates, nor by making fields private, which constrains nothing within
one module.

#### Scenario: Adding a field to `Change` fails to compile in the shared conformance function

- **WHEN** a field is added to `Change` and only `from_files`' construction site is updated
- **THEN** `cargo test --all-features` fails to compile, reporting that the pattern in
  `conformance::assert_invariants` does not mention the new field
- **AND** the error names the field, so the omission cannot be resolved by adding a rest
  pattern without deliberately deleting the enforcement

#### Scenario: `Change` gaining a `Default` is caught by a source check

- **WHEN** the guarded source check for a `Default` implementation on `Change` is run
  against a `src/changes.rs` in which `Change` derives `Default`
- **THEN** the check exits non-zero
- **AND** the same check exits non-zero when `src/changes.rs` does not exist, rather than
  passing on a missing file

#### Scenario: Every value a producer builds satisfies the shared invariants

- **WHEN** `from_files` produces a `ChangeSet` from a repository holding two active and
  three archived changes
- **THEN** `assert_invariants` passes for all five values
- **AND** the same function is the one `changes-from-cli` calls for every value
  `from_cli` builds, with no second copy of the invariants written for that producer

### Requirement: Archived changes are file-sourced only, and the two lists stay separate

`ChangeSet` SHALL hold `active` and `archived` as two separate vectors plus a repository-level
`problems` list, rather than one vector discriminated by `origin`.

The separation is a contract, not a convenience. `openspec list --json` lists active changes
only, and the OpenSpec CLI offers no way to address a change under `archive/` at all — an
`openspec instructions apply --change 2026-09-04-task-parsing` fails with "change not
found". Archived changes are therefore permanently file-sourced, and Phase 3's merge policy
layers CLI results over the `active` list alone while `archived` passes through untouched.
Keeping them in one vector would make that policy filter on a field instead of reading a
list, and would silently drop archived rows the first time the filter was written wrong.

A name MAY appear in both lists at once; the two entries are distinct changes and neither
suppresses the other.

#### Scenario: An archived change keeps its file-derived values when the CLI arrives

- **WHEN** a repository holds an active change `add-auth` and an archived
  `2026-08-14-add-auth`
- **THEN** `active` holds one `Change` named `add-auth` and `archived` holds one `Change`
  named `add-auth` with `origin` `Archived { date: Some("2026-08-14") }`
- **AND** the two are separate values with different `dir` values, and neither list is
  filtered by the other

#### Scenario: A repository-level failure is recorded on the set, not on a change

- **WHEN** `openspec/changes/` exists but cannot be read
- **THEN** `ChangeSet::problems` holds exactly one entry naming the directory and the reason
- **AND** `active` and `archived` are both empty, no synthetic `Change` is invented to carry
  the message, and the archive directory beneath it is not attempted — reading
  `openspec/changes/archive/` under an unreadable parent fails with a permission error of
  its own, which would make a second, redundant problem

### Requirement: Producing changes never fails and never panics

`changes::from_files` SHALL return a `ChangeSet` and never a `Result`. Every failure —
an unreadable directory, an unloadable schema, an unreadable task file, an artifact pattern
the file path does not support — SHALL degrade to an empty or partial value plus a
human-readable entry on the nearest `problems` list, matching the total shape of
`config::load`, `state::read`, `resolve::openspec_bin`, and `tasks::read`.

No entry point of this module SHALL `unwrap`, `expect`, index a slice, or panic on any
input, including a directory entry whose name is not valid UTF-8. Such an entry SHALL be
skipped, and one problem naming it SHALL be recorded on `ChangeSet::problems` — the listing
is a repository-level fact, so the problem belongs to the set rather than to a change that
was never built.

The requirement is written for Linux, where such a name is creatable. On macOS with APFS it
is not: `mkdir` rejects an invalid UTF-8 byte sequence with `EILSEQ`, so a filesystem test
of this behaviour would fail on the reference machine for a reason that has nothing to do
with the plugin. The verification is therefore a unit test over the name-decoding step
itself, given an `OsString` built from invalid bytes, rather than a directory tree.

#### Scenario: A directory entry whose name is not valid UTF-8 is skipped, not fatal

- **WHEN** the name-decoding step is given an `OsString` built from bytes that are not valid
  UTF-8, alongside one built from a normal name
- **THEN** the normal name is returned and the undecodable one is not
- **AND** exactly one problem naming the undecodable entry is produced, and nothing panics

#### Scenario: Every degradation lands on a problems list rather than in a return type

- **WHEN** a repository is read whose `openspec/changes/archive/` is unreadable, one active
  change's schema is not vendored, and another active change's `tasks.md` is a directory
  where a file was expected
- **THEN** `from_files` returns a `ChangeSet` value rather than an error, holding both
  active changes
- **AND** each of the three failures appears exactly once — on `ChangeSet::problems` for
  the unreadable archive directory, and on the owning `Change::problems` for the other two
