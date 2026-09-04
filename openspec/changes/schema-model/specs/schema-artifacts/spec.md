## Purpose
Turning a named schema into the model the dashboard renders: the ordered artifact list
that is the detail view's tab order, and the one artifact holding a change's task
checklist. The list comes from `openspec/schemas/<name>/schema.yaml`, parsed as YAML
rather than scanned, in file order, carrying each artifact's `id` and its `generates`
value verbatim so `changes-from-files` can derive a path rather than guess one. The tasks
artifact is the one whose `generates` equals the schema's top-level `apply.tracks`, with
the artifact whose id is `tasks` as the fallback when no `apply` block declares what it
tracks — there is no `role` key in the OpenSpec schema format, whatever `SPEC.md` once
said. Nothing here fails closed: a schema that is not vendored, one that cannot be read,
one that is not a usable schema, and one whose individual entries are unusable each yield
a value naming the reason, and the three reasons are kept apart because a later change
asks the CLI about exactly one of them.

## ADDED Requirements

### Requirement: A vendored schema is loaded from `openspec/schemas/<name>/schema.yaml`

For a resolved schema name, the plugin SHALL read
`<repository>/openspec/schemas/<name>/schema.yaml` and produce an **ordered** list of
artifacts, each carrying its `id` and its `generates` value verbatim. The order SHALL be
the order the `artifacts:` sequence gives — never sorted, never de-duplicated, never
re-ordered by the `requires` edges — because that order is the detail view's tab order.

The value SHALL also carry the schema name that located the file, which is the directory
segment, not the file's own `name:` key. Those two are normally equal; when the file
declares a **non-blank string** `name:` that differs from the directory segment, the
difference SHALL be recorded as a problem, because a forked schema whose directory was
renamed still resolves by directory and the mismatch is otherwise invisible. A `name:` that
is absent, blank, or not a string is **not** a disagreement and SHALL record nothing — most
of this change's fixtures omit the key entirely, and treating absence as a mismatch would
put a spurious problem on almost every schema.

Only the **first YAML document** of the file is used, and the file is parsed as YAML rather
than scanned line by line — the vendored `tdd` schema's `apply.instruction` block scalar is
indented to the same column as an artifact's keys and contains markdown headings, fenced
code, and `key: value`-shaped prose, so a line scanner reads that content as structure.

Loading SHALL distinguish three reasons a schema did not load, because each has a different
consequence and a different degraded row:

- **not vendored** — no `schema.yaml` exists under `openspec/schemas/<name>/`. This is the
  normal state for a schema the CLI ships rather than the repository (`spec-driven` is not
  vendored here), and it is the one case `changes-from-cli` can repair by asking
  `openspec schema which <name> --json` for the schema's directory;
- **unreadable** — the path exists but its contents could not be read as UTF-8 text, whether
  because of an I/O error or a decode failure. A file of invalid UTF-8 belongs here and not
  under *invalid*: nothing about it is a YAML problem, and the consequence for
  `changes-from-cli` is the same as an I/O error's;
- **invalid** — the bytes were read as text and are not a usable schema.

Loading SHALL never panic and SHALL never return a bare error to the caller: the composed
resolution always yields a value carrying the name, the source, either the schema or one of
the three reasons, and the accumulated problems — those recorded while **selecting** the
name and those recorded while **loading** it, in that order. A composed result that carries
only the load step's problems has discarded the message telling a user their
`.openspec.yaml` is broken, which is the whole reason selection records one.

#### Scenario: The repository's own vendored `tdd` schema loads

- **WHEN** the schema named `tdd` is loaded from this repository's own tree — the real
  graft-vendored `openspec/schemas/tdd/schema.yaml`, read and never written
- **THEN** the artifact list is **exactly** `proposal`, `specs`, `design`, `tasks`,
  `planning-review`, in that order — the whole list, not a containment check, because a
  parser that loses `design` or `planning-review` (the two artifacts with the longest
  `instruction:` block scalars) satisfies every "contains" clause
- **AND** the schema's name is `tdd`
- **AND** the tasks artifact is `tasks` with `generates` `tasks.md`, selected through the
  file's real `apply.tracks: tasks.md` — the only place the tracks rule meets a real schema
- **AND** `specs`'s `generates` is the glob `specs/**/*.md`, carried verbatim rather than
  expanded
- **AND** no problem is recorded, which also pins that the file's `name: tdd` matching its
  directory records nothing

#### Scenario: Artifact order is the file's order, not alphabetical

- **WHEN** a fixture schema lists artifacts in the order `zeta`, `alpha`, `middle`, where
  `zeta` declares `requires: [middle]` and `alpha` declares `requires: [zeta]`, followed by a
  fourth entry repeating the id `zeta` with `generates: zeta2.md`
- **THEN** the produced list is exactly four entries, `zeta`, `alpha`, `middle`, `zeta`, in
  file order
- **AND** it is not sorted (`alpha` first), not reversed (`zeta2` first), not the
  topological order of `requires` (`middle`, `zeta`, `alpha`), and not collapsed to three
  entries by de-duplicating on id — the three transformations the requirement forbids by
  name, none of which a fixture without edges or repeats can distinguish

#### Scenario: A schema that is not vendored is reported as such and not as broken

- **WHEN** `spec-driven` is loaded from a repository whose `openspec/schemas/` holds only
  `tdd`
- **THEN** no schema is produced and the reason is **not vendored**, naming the path
  `openspec/schemas/spec-driven/schema.yaml` that was searched
- **AND** the reason is distinguishable from *unreadable* and from *invalid* by its own
  variant rather than by matching text in a message, because `changes-from-cli` branches
  on exactly this case
- **AND** the composed resolution for the same repository records exactly one problem,
  naming that path — the load reason and the rendered problem are separate observables and
  both are asserted
- **AND** a repository with no `openspec/schemas/` directory at all yields the same **not
  vendored** reason rather than *unreadable*

#### Scenario: A schema directory with no `schema.yaml` is not vendored

- **WHEN** `openspec/schemas/half/` exists as a directory but contains no `schema.yaml`
- **THEN** the reason is **not vendored**, not *unreadable* — an empty schema directory is
  the same situation as an absent one for a consumer deciding whether to ask the CLI

#### Scenario: A `schema.yaml` that cannot be read is reported as unreadable

- **WHEN** `openspec/schemas/odd/schema.yaml` exists as a **directory**, so reading it
  fails with an I/O error rather than a parse error
- **THEN** the reason is **unreadable**, naming the path and carrying the I/O error's
  message
- **AND** the composed resolution records exactly one problem naming the path
- **AND** the call returns rather than panicking

#### Scenario: A composed resolution carries the selection's problems as well as the load's

- **WHEN** a change's `.openspec.yaml` is invalid YAML — so selection records a problem and
  falls through — and `openspec/config.yaml` declares `schema: absent`, which is not vendored
- **THEN** the composed result carries the name `absent`, the source *project*, the reason
  **not vendored**, and **two** problems, the selection's first
- **AND** this is the only scenario in which both halves contribute, and it is what rejects
  an implementation that builds the composed problem list from the load step alone

#### Scenario: The directory name and the file's `name:` key disagreeing is recorded

- **WHEN** `openspec/schemas/renamed/schema.yaml` declares `name: original` and one valid
  artifact
- **THEN** the schema loads with name `renamed` — the directory that located it
- **AND** exactly one problem is recorded, containing both `renamed` and `original`
- **AND** in the same test, a schema file carrying **no** `name:` key, one whose `name:` is
  blank, and one whose `name:` is a mapping each load with the directory's name and **no**
  problem, because an absent or unusable `name` is not a disagreement

#### Scenario: Only the first YAML document of the schema file is used

- **WHEN** a schema file holds two documents separated by `---`, the first declaring an
  artifact `first` and the second declaring an artifact `second`
- **THEN** the artifact list is exactly `first`

### Requirement: The tasks artifact is the one `apply.tracks` selects, else the one with id `tasks`

The artifact holding a change's task checklist SHALL be identified by the schema's top-level
`apply:` block:

- when `apply.tracks` is present and is a non-null string, the tasks artifact is the one
  whose **`generates` equals that value**, and there is no further fallback — if no artifact
  matches, the schema has no tasks artifact even if an artifact with id `tasks` exists;
- when `apply` is absent, or `apply.tracks` is absent or explicitly `null`, the tasks
  artifact is the one whose **id is `tasks`**;
- when neither rule finds one, the schema SHALL load with **no** tasks artifact and record a
  problem, rather than failing to load.

The first two clauses are the rule the OpenSpec CLI itself applies in
`findTrackedTasksArtifact`. **`SPEC.md`'s claim that the artifact carries a `role: tasks` key
is false** — no `role` key exists in the schema format, in the vendored `tdd` schema, in the
CLI's bundled `spec-driven` schema, or in the CLI's own definition of an artifact. `SPEC.md`
is corrected by this change.

Two clauses **diverge** from the CLI deliberately, and are stated as divergences rather than
as parity so that no later change inherits a false claim about the CLI:

- **A `tracks` value that is present but not a string.** The CLI branches on
  `tracks != null`, so a sequence is non-null: it searches for a matching `generates`, misses,
  and yields no tasks artifact — and upstream of that its schema validator types `tracks` as a
  relative-path string, so it rejects the whole schema before the search runs. The plugin
  instead treats a non-string `tracks` as no declaration and falls back to the id, recording a
  problem. A viewer that shows the conventional tasks tab plus a named problem is more useful
  than one that shows none.
- **The third clause itself.** The CLI has no "no tasks artifact" state to degrade into,
  because it rejects such schemas at parse time; the plugin loads them.

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

#### Scenario: A `tracks` value of the wrong type falls back to the id

- **WHEN** a fixture schema carries `apply.tracks` as a sequence rather than a string, and
  declares an artifact with id `tasks`
- **THEN** the tasks artifact is `tasks` and exactly one problem is recorded naming the
  wrong-typed key

### Requirement: An unusable artifact entry is skipped and named, not fatal

An entry in the `artifacts:` sequence SHALL be usable only when it is a mapping carrying a
non-blank string `id` and a non-blank string `generates` whose value is a relative path
containing no `..` segment and no NUL byte — the constraint the OpenSpec CLI applies to
`generates`, and one worth applying here for a reason of this crate's own, because
`changes-from-files` joins that value onto a change directory.

An entry failing any of those SHALL be **skipped**, with exactly one problem recorded naming
its zero-based position in the sequence, and the remaining artifacts SHALL still load. Other
keys the schema format carries — `description`, `template`, `instruction`, `requires` — are
not read and SHALL NOT make an entry unusable when absent or malformed, because the plugin
renders none of them and failing on a field it never uses would fail closed. This is
**deliberately more permissive than the CLI**, which requires both `description` and
`template` on every artifact: a schema whose entries omit them loads here and is rejected
there. That divergence is stated rather than implied, for the same reason as the two in the
tasks-artifact requirement above.

Skipping SHALL NOT be allowed to empty the list: a schema whose every entry is unusable is
*invalid*, not a schema with zero tabs.

#### Scenario: Entries missing `id` or `generates` are skipped and the rest survive

- **WHEN** a fixture schema's `artifacts:` sequence is: a valid `proposal`, an entry with
  `generates: orphan.md` and no `id`, an entry with `id: noname` and no `generates`, and a
  valid `tasks`
- **THEN** the artifact list is exactly `proposal`, `tasks`, in that order
- **AND** exactly two problems are recorded, one naming position 1 and one naming position 2
- **AND** the tasks artifact is still found, so one bad neighbour does not cost the tasks tab

#### Scenario: A blank `id` or `generates` is skipped like an absent one

- **WHEN** a fixture schema declares an entry with `id: "  "` and a valid `generates`, and
  another with a valid `id` and `generates: ""`, alongside one fully valid artifact
- **THEN** only the valid artifact is in the list and exactly two problems are recorded

#### Scenario: An entry that is not a mapping is skipped

- **WHEN** a fixture schema's `artifacts:` sequence holds a bare string `"proposal"` and a
  nested sequence, alongside one valid artifact
- **THEN** only the valid artifact is in the list, exactly two problems are recorded, and
  the call does not panic

#### Scenario: A `generates` that escapes the change directory is skipped

- **WHEN** a fixture schema declares artifacts whose `generates` values are, in turn,
  `../outside.md`, `/etc/passwd`, and `a/../../b.md`, alongside one valid artifact
- **THEN** only the valid artifact is in the list and exactly three problems are recorded
- **AND** in the same test `generates: specs/**/*.md` and `generates: sub/dir/file.md` are
  both accepted, so the rule rejects traversal rather than every value containing a
  separator or a punctuation character

#### Scenario: Unused keys being absent or malformed does not make an entry unusable

- **WHEN** a fixture schema declares one artifact carrying only `id` and `generates` — no
  `description`, no `template`, no `instruction`, no `requires` — and a second carrying a
  `requires` value that is a string rather than a sequence
- **THEN** both artifacts load and no problem is recorded, because the plugin reads
  neither key

#### Scenario: A schema whose every entry is unusable is invalid rather than empty

- **WHEN** a fixture schema's `artifacts:` sequence holds two entries, neither carrying an
  `id`
- **THEN** no schema is produced and the reason is **invalid**
- **AND** the result is not a schema with an empty artifact list, which would render a
  detail view with no tabs and no explanation

### Requirement: A `schema.yaml` that is not a usable schema yields the invalid reason

A schema file SHALL be treated as *invalid* — no schema produced, and the **invalid** reason
carrying both the path and the underlying cause — when the bytes are not valid YAML, when the
file holds no document at all, when the first document is not a mapping, when `artifacts:` is
absent, when `artifacts:` is not a sequence, or when `artifacts:` is an empty sequence. The
composed resolution turns that reason into exactly one problem; the scenarios below assert
the reason, because that is what the parsing step returns, and the composition's
problem-for-each-reason rule is asserted once, in the *unreadable* and *not vendored*
scenarios above, rather than restated for every cause.

Because the file is parsed as YAML rather than scanned, a schema written in flow style, one
using anchors and aliases, and one using CRLF line endings SHALL all load, and their
artifacts SHALL be indistinguishable from block-style ones.

#### Scenario: Invalid YAML is reported with the parser's own message

- **WHEN** a schema file uses a tab character for indentation
- **THEN** the reason is **invalid**, naming the path, and carrying the parser's own message
  so the offending line can be found
- **AND** the composed resolution renders that reason as exactly one problem
- **AND** the call returns rather than panicking

#### Scenario: An empty, comment-only, or non-mapping document is invalid

- **WHEN** a schema file is, in turn, completely empty, only a comment line, a bare scalar
  document `just text`, and a top-level sequence
- **THEN** each yields the **invalid** reason, naming its own cause
- **AND** none of them panics on indexing a document that is not a mapping

#### Scenario: An absent, non-sequence, or empty `artifacts:` key is invalid

- **WHEN** a schema file carries a valid `name:` and `version:` but, in turn, no `artifacts`
  key, `artifacts: nope`, and `artifacts: []`
- **THEN** each yields the **invalid** reason, naming its own cause
- **AND** the `artifacts: []` case is asserted explicitly, because a schema with zero tabs
  is not a state the detail view can render

#### Scenario: Flow style, anchors, and CRLF all parse

- **WHEN** a schema file declares
  `artifacts: [{id: a, generates: a.md}, {id: tasks, generates: tasks.md}]` on one line
- **THEN** two artifacts load in that order and the tasks artifact is `tasks`
- **AND** in the same test a schema declaring an anchored mapping and referencing it with an
  alias inside `artifacts:` loads that artifact, and a schema whose lines end `\r\n` loads
  identically to the same schema with `\n`
- **AND** these three are what distinguish a YAML parser from a line scanner: each is legal
  YAML that an indentation-and-colon scanner reads wrongly or not at all

#### Scenario: A schema file whose block scalars mimic structure still parses correctly

- **WHEN** a fixture schema declares two artifacts and an `apply:` block whose
  `instruction: |` block scalar contains, at the same indentation an artifact's keys use,
  the lines `- id: fake`, `generates: fake.md`, and `tracks: fake.md`
- **THEN** the artifact list is exactly the two real artifacts, with no `fake` entry
- **AND** the tasks artifact is chosen from the real `apply.tracks` value, not from the one
  inside the block scalar
- **AND** this fixture is modelled on the vendored `tdd` schema, whose real
  `apply.instruction` content sits at the same column as `generates:` does under an artifact

### Requirement: Schema loading reads and never writes, and spawns no process

Loading a schema SHALL be a read-only operation over the repository tree, creating,
modifying, removing, and touching nothing — including when the schema directory or the file
is absent. `src/schema.rs` SHALL name no process API at all, spawning or otherwise: the
`cli` seam does not exist until `subprocess-seam`, and this module is a pure transformation
over bytes read from disk.

#### Scenario: A repository tree is byte-identical after loading

- **WHEN** a scratch fixture holding `openspec/config.yaml`, `openspec/schemas/tdd/schema.yaml`,
  `openspec/changes/x/tasks.md`, an **empty** `openspec/specs/`, and `README.md` is
  snapshotted — every path **including every directory**, every file's bytes, and every
  entry's modification time, read through `std::fs::Metadata` — and the schema is then
  resolved twice
- **THEN** a second snapshot equals the first exactly
- **AND** a third run naming a schema that is **not** vendored leaves the snapshot equal
  too, so a miss creates no directory

#### Scenario: The schema module names no process API

- **WHEN** `src/schema.rs` is searched for `std::process`, `Command`, and `Stdio`
- **THEN** none is present
- **AND** the search is guarded so that a missing `src/schema.rs` fails the check rather
  than passing it, since `grep` exits 2 for a missing file and a negated exit 2 reads as
  success
