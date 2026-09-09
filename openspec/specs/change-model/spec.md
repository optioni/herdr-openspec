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

### Requirement: Both producers of a change build every field, enforced at compile time

The plugin SHALL keep `changes::from_files` and `changes::from_cli` producing the same
`Change` by three mechanisms, none of which is a comment:

1. **No `Default`.** None of `Change`, `ChangeSet`, `ArtifactRef`, or `Origin` derives or
   implements `Default` **anywhere in the crate** — `impl Default for Change` is legal in
   any file, so the rule is stated over `src/`, not over one module — and no expression
   anywhere in `src/changes.rs` uses a `..` functional update or a `..` rest pattern in a
   literal or a pattern for any of those four types. Every construction site therefore
   names every field, and a new field is a compile error (`E0063`) at each of them.

   The `..` half is stated separately from the `Default` half because it is **not implied**
   by it: `Change { name, ..other }` compiles with no `Default` anywhere in the crate, and
   a check that looked only for `Default` would pass over it. Both halves are checked.

2. **A shared conformance function that destructures exhaustively.** A test-only
   `changes::conformance::assert_invariants(&Change)` SHALL open with an exhaustive `let
   Change { … }` pattern carrying **no** `..` rest pattern, so adding a field makes that one
   function fail to compile (`E0027`). Because it is the function *both* producers' test
   suites call, whoever adds a field is forced to state what the new field means for both
   sources rather than for the one they were working on.

3. **Every producer's tests call it.** Every `Change` a producer's test builds SHALL be
   passed through `assert_invariants`, which checks the source-independent invariants: a
   non-empty `name`; a non-empty `schema`; a `dir` whose final component equals `name` for
   an `Active` change and ends with `name` for an `Archived` one; an `ArtifactRef` with a
   non-empty `id` for every entry; and `problems` holding no empty string, so it is used
   only for human-readable text and never to carry data a field should hold. It SHALL NOT
   assert that artifact ids are unique — the landed `schema-artifacts` capability requires
   that a schema's artifact list is "never de-duplicated", so a schema declaring the same
   id twice legitimately produces two `ArtifactRef`s with one id.

Mechanisms 1 and 2 SHALL both be checked, and each SHALL be shown to catch what the other
misses. Neither alone is sufficient and neither substitutes for the other.

The set of construction sites the mechanisms bind is **every** site that builds a `Change`,
not only the two named producers. `changes::merge` builds merged `Change` values from a
pair of producer values and is a third such site; a field it filled by default would drift
exactly as a producer's would.

The plugin SHALL NOT satisfy this requirement with a `#[non_exhaustive]` attribute, which
constrains only other crates, nor by making fields private, which constrains nothing within
one module.

#### Scenario: Adding a field to `Change` fails to compile in the shared conformance function

- **WHEN** a field is added to `Change` and only `from_files`' construction site is updated
- **THEN** `cargo test --all-features` fails to compile, reporting that the pattern in
  `conformance::assert_invariants` does not mention the new field
- **AND** the error names the field, so the omission cannot be resolved by adding a rest
  pattern without deliberately deleting the enforcement

#### Scenario: Adding a field breaks both mechanisms, and each catches what the other misses

- **WHEN** a field is added to `Change` in a throwaway copy of the crate and nothing else
  is changed
- **THEN** the build fails with **both** `E0027` in `conformance::assert_invariants` and
  `E0063` at every construction site, which after `changes-from-cli` includes
  `from_files`', `from_cli`'s, and `merge`'s
- **AND** when mechanism 1 is then fully defeated in that copy — `Change` derives
  `Default` and every `Change` literal is filled from it, so **no `E0063` remains** — the
  build still fails with `E0027`, proving mechanism 2 catches what mechanism 1 misses
- **AND** when instead mechanism 2 is defeated by adding a `..` rest pattern to
  `assert_invariants`' pattern and no `Default` is added, so **no `E0027` remains**, the
  build still fails with `E0063`, proving mechanism 1 catches what mechanism 2 misses
- **AND** the assertions are on the specific error codes present **and absent**, not on
  "the build failed": a copy broken for an unrelated reason also fails, and would
  otherwise be read as evidence

#### Scenario: `Change` gaining a `Default` is caught by a source check

- **WHEN** the guarded source check for a `Default` implementation is run against a copy of
  `src/` in which, in turn, `Change` derives `Default` across a multi-line `#[derive(…)]`,
  `Change` carries a hand-written `impl std::default::Default for Change`, and
  `src/state.rs` — not `src/changes.rs` — carries an `impl Default for
  crate::changes::Origin`
- **THEN** the check exits non-zero in all three cases, because it searches every file
  under `src/` and matches a path-qualified `Default` as well as a bare one
- **AND** the same check exits non-zero when `src/changes.rs` does not exist, when fewer
  than eight `.rs` files are searched, and when the four types are not declared where it
  expects them, rather than passing on a missing or wrong file

#### Scenario: A rest pattern is caught even when a comment separates it from the comma

- **WHEN** the guarded source check for a `..` functional update or rest pattern is run
  against a copy of `src/changes.rs` carrying, in turn, `Change { name, ..other }` on one
  line; the same spread across lines; `let Change { name, .. }`; a functional update with a
  `// line comment` between the comma and the `..`; and one with a `/* block comment */`
  between them
- **THEN** the check exits non-zero in all five cases. The last two are the ones a
  comment-naive scan misses, and they compile and are formatter-clean
- **AND** the check exits zero against the real `src/changes.rs`, which contains a
  `segment[..star]` slice index it must not fire on, so the green run is itself
  discriminating rather than merely permissive

#### Scenario: Every value a producer builds satisfies the shared invariants

- **WHEN** `from_files` produces a `ChangeSet` from a repository holding two active and
  three archived changes
- **THEN** `assert_invariants` passes for all five values
- **AND** the same function is the one `changes-from-cli` calls for every value
  `from_cli` builds and every value `merge` returns, with no second copy of the invariants
  written for either

### Requirement: Archived changes are file-sourced only, and the two lists stay separate

`ChangeSet` SHALL hold `active` and `archived` as two separate vectors, a repository-level
`problems` list, and an `archived_total: usize` — the number of archived changes the archive
directory holds — rather than one vector discriminated by `origin`.

The separation is a contract, not a convenience. `openspec list --json` lists active changes
only, and the OpenSpec CLI offers no way to address a change under `archive/` at all — an
`openspec instructions apply --change 2026-09-04-task-parsing` fails with "change not
found". Archived changes are therefore permanently file-sourced, and Phase 3's merge policy
layers CLI results over the `active` list alone while `archived` passes through untouched.
Keeping them in one vector would make that policy filter on a field instead of reading a
list, and would silently drop archived rows the first time the filter was written wrong.

A name MAY appear in both lists at once; the two entries are distinct changes and neither
suppresses the other.

`archived_total` is `list-sections`' addition and exists because `archived` is no longer
always populated: `change-enumeration` resolves the archived tier only when the archived
section is shown, and a collapsed section's header still has to say how many changes are
behind it. It SHALL be the count the archive **enumeration** produced, before any change was
built, so it is the same number under either scope. Two invariants therefore hold on every
`ChangeSet` either producer returns, and a **new** `#[cfg(test)]` function
`conformance::assert_set_invariants(set: &ChangeSet)` SHALL check both: `archived.len()` is
either `0` or exactly `archived_total`, and `archived_total` is never less than
`archived.len()`.

It SHALL be a new function beside `assert_invariants`, never a widening of it.
`assert_invariants` takes a `&Change` and destructures it exhaustively with no rest pattern,
which is this capability's **mechanism 2** — the `E0027` guard that makes adding a `Change`
field a compile error inside the shared conformance function. Changing its parameter to a
`ChangeSet` would destroy that guarantee for every landed call site.
`assert_set_invariants` SHALL destructure `ChangeSet` exhaustively for the same reason, so a
sixth `ChangeSet` field is a compile error there too. `changes::merge` SHALL carry the file result's `archived_total`
through untouched, on exactly the terms `archived` itself passes through, and
`changes::empty_set()` SHALL set it to `0`.

`archived_total` is **not** a field of `Change` and does not weaken the seven-field rule
above: it is a property of the set, like `problems`, and `ChangeSet` derives no `Default`
either, so adding it is a compile error at every construction site rather than a silent `0`
at one of them.

#### Scenario: An archived change keeps its file-derived values when the CLI arrives

- **WHEN** a repository holds an active change `add-auth` and an archived
  `2026-08-14-add-auth`
- **THEN** `active` holds one `Change` named `add-auth` and `archived` holds one `Change`
  named `add-auth` with `origin` `Archived { date: Some("2026-08-14") }`
- **AND** the two are separate values with different `dir` values, and neither list is
  filtered by the other
- **AND** `archived_total` is `1`, and `changes::merge` of that set with any `CliChanges`
  leaves it `1`

#### Scenario: A repository-level failure is recorded on the set, not on a change

- **WHEN** `openspec/changes/` exists but cannot be read
- **THEN** `ChangeSet::problems` holds exactly one entry naming the directory and the reason
- **AND** `active` and `archived` are both empty, no synthetic `Change` is invented to carry
  the message, and the archive directory beneath it is not attempted — reading
  `openspec/changes/archive/` under an unreadable parent fails with a permission error of
  its own, which would make a second, redundant problem
- **AND** `archived_total` is `0`, so an unreadable tree reports no archive rather than an
  archive whose size is unknown

#### Scenario: The two `archived_total` invariants hold under either scope

- **WHEN** a scratch repository whose archive holds twenty-two dated directories is
  enumerated once with the archived tier resolved and once with it unresolved, and
  `conformance::assert_set_invariants` is called on both results
- **THEN** the resolved set has `archived.len()` 22 and `archived_total` 22, and the
  unresolved set has `archived.len()` 0 and `archived_total` 22
- **AND** `assert_set_invariants` accepts both and rejects a hand-built `ChangeSet` whose
  `archived` holds three changes while `archived_total` is 22, so the invariant is a check
  rather than a comment
- **AND** `assert_invariants` still takes a `&Change` and every landed call site compiles
  unchanged, so mechanism 2's `E0027` guard is intact

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
