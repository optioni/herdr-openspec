## ADDED Requirements

### Requirement: A not-vendored schema is repaired through `openspec schema which`

When resolving a change's schema, `from_cli` SHALL first call `schema::load(repo, name)`.
On `Ok` it SHALL use that schema and SHALL NOT invoke the CLI.

On `Err(LoadError::NotVendored)` — and **only** on that variant — it SHALL run
`["schema", "which", <name>, "--json"]`, read the resulting object's `path` as the
schema's **directory**, and load it with `schema::load_dir(<path>, <name>)`, the same
parser the repository tier uses. `schema-artifacts` already names this as the one load
failure `changes-from-cli` can repair, and it is what reaches the two tiers a disk-only
read cannot see: `$XDG_DATA_HOME/openspec/schemas/<name>` (reported as `source: "user"`)
and the CLI package's own `schemas/<name>` (reported as `source: "package"`), the tier that
holds `spec-driven`, which no repository vendors.

On `Err(LoadError::Unreadable)` or `Err(LoadError::Invalid)` it SHALL NOT invoke the CLI:
the file is present and broken, and the CLI's tier 1 would read the same bytes. It SHALL
record one problem naming the path and the reason, and produce no schema.

The resolved `path` SHALL be used as given. The CLI reports the schema's directory, not the
`schema.yaml` file (`dist/commands/schema.js:16-38` stores `path.join(<schemasDir>, name)`
and only *tests* `<dir>/schema.yaml` for existence), which is why `schema::load_dir` — which
joins `schema.yaml` itself — is the right entry point and `schema::load` is not.

The payload's `source` and `shadows` keys SHALL be ignored. `source` names which tier
answered, which changes nothing about how the directory is read, and `shadows` lists the
lower-priority locations, which this plugin does not consult.

`schema::load` and `schema::load_dir` return a `ParsedSchema` carrying both the schema and
a `problems` list — one entry per artifact entry the parser skipped, plus a
directory-name-versus-declared-`name:` mismatch. Those problems SHALL be recorded on every
`Change` whose schema resolved through them, exactly as `from_files` records them through
`CachedSchemaLoad`. They SHALL NOT be discarded: on a schema the repository does not vendor,
the file producer never read the file at all, so this producer is the **only** one that can
report them and `change-merge`'s duplicate collapsing has nothing to collapse against.

Because the schema is resolved at most once per name per call, a per-name problem list
SHALL be attached to **each** change using that name, not only the first — otherwise a
message appears on whichever change happened to be enumerated first.

#### Scenario: A parser problem from the CLI-named schema reaches every change using it

- **WHEN** two changes report `schemaName: "spec-driven"`, the repository vendors no
  `spec-driven`, and the `schema which` payload names a scratch directory whose
  `schema.yaml` declares three artifacts of which one entry has no usable `id`, so
  `schema::load_dir` returns `Ok` carrying two artifacts and one problem
- **THEN** both changes carry two `ArtifactRef`s
- **AND** both changes' `problems` hold that one parser message, so it is not attached to
  only the first change to use the cached entry

#### Scenario: A schema absent from the repository is loaded from the directory the CLI names

- **WHEN** a change's apply payload reports `schemaName: "spec-driven"`, the repository
  vendors no `openspec/schemas/spec-driven/`, and the fake answers
  `["schema", "which", "spec-driven", "--json"]` with
  `{"name":"spec-driven","source":"package","path":"<scratch>/pkg/spec-driven","shadows":[]}`
  where `<scratch>/pkg/spec-driven/schema.yaml` is a real file declaring artifacts
  `proposal`, `tasks` in that order
- **THEN** the produced `Change` has `schema` `spec-driven` and two `ArtifactRef`s with
  ids `proposal` and `tasks` in that order
- **AND** the schema file was read from disk with no process spawned, because the fake
  answered from a registration and the directory it named holds a real `schema.yaml`
- **AND** no problem is recorded

#### Scenario: A vendored schema never reaches the CLI

- **WHEN** the repository vendors `openspec/schemas/tdd/schema.yaml` and a change's apply
  payload reports `schemaName: "tdd"`, and the fake has **no** registration for any
  `schema which` vector
- **THEN** `from_cli` returns without panicking and the change carries the `tdd` schema's
  artifacts
- **AND** the recorded invocations contain no vector beginning `schema`, which the fake's
  refusal to answer an unregistered pair is what makes verifiable

#### Scenario: An unreadable vendored schema is not repaired by the CLI

- **WHEN** `openspec/schemas/odd/schema.yaml` exists as a **directory**, so
  `schema::load` reports `Unreadable`, a change's apply payload reports
  `schemaName: "odd"`, and the fake has no `schema which` registration
- **THEN** the produced `Change` has `schema` `odd`, an empty `artifacts` list, and exactly
  one problem naming the path and the I/O error
- **AND** no `schema which` invocation is recorded, so an already-present broken file is
  not re-asked of the CLI

#### Scenario: An invalid vendored schema is not repaired by the CLI

- **WHEN** `openspec/schemas/bad/schema.yaml` exists and holds bytes that are not a usable
  schema, so `schema::load` reports `Invalid`, and the fake has no `schema which`
  registration
- **THEN** the change carries `schema` `bad`, an empty `artifacts` list, and one problem
  naming the path and the parse reason
- **AND** no `schema which` invocation is recorded

### Requirement: Every failure of the fallback tier degrades and names itself

Each of the following SHALL produce no schema for the affected change — an empty
`artifacts` list rather than a missing change — and SHALL record exactly one problem naming
the schema name and what failed. None SHALL panic, and none SHALL abort the other changes.

- `schema which` could not start the program (`CliError::NotStarted`).
- `schema which` exited non-zero (`CliError::Failed`). The CLI writes `{"error": …,
  "available": [ … ]}` to **stdout** and exits 1 for an unknown schema
  (`dist/commands/schema.js:450-464`), and `subprocess-seam`'s `Failed` carries stderr
  only, so the problem SHALL name the schema, the vector, and the exit code and SHALL NOT
  claim to report the CLI's own message.
- The output is not JSON, is not an object, or has no `path`, or a `path` that is not a
  string, or a `path` that is empty.
- The directory the `path` names holds no `schema.yaml` — `load_dir` reports `NotVendored`
  a second time. The tier SHALL stop there rather than asking again.
- The `schema.yaml` in that directory is unreadable or is not a usable schema.

The parser SHALL require the whole of stdout to be one JSON document, and SHALL NOT skip
leading non-JSON lines. The CLI writes its `Note: Schema commands are experimental and may
change.` line to **stderr** (`dist/commands/schema.js:388-391`), and `subprocess-seam`
already guarantees stderr never reaches an `Ok` value, so no stripping is needed; a parser
that tolerated a leading noise line would silently accept a future release that moved the
note to stdout, hiding a real change in the contract instead of degrading visibly.

#### Scenario: An unknown schema name degrades that change alone

- **WHEN** three changes are reported, the middle one's apply payload names
  `schemaName: "outside-in-tdd"`, the repository vendors no such schema, and
  `["schema", "which", "outside-in-tdd", "--json"]` answers
  `Err(CliError::Failed { code: Some(1), stderr: "" })`
- **THEN** all three changes appear in `active`, and the middle one has `schema`
  `outside-in-tdd`, an empty `artifacts` list, and one problem naming the schema, the
  vector, and exit code `1`
- **AND** the other two changes carry their full artifact lists and no problem

#### Scenario: A `path` naming a directory with no `schema.yaml` stops the tier

- **WHEN** `schema which` answers a well-formed object whose `path` names a scratch
  directory that exists but holds no `schema.yaml`
- **THEN** the change carries an empty `artifacts` list and exactly one problem naming the
  joined `schema.yaml` path as not present
- **AND** exactly one `schema which` invocation is recorded for that name, so the tier did
  not retry

#### Scenario: An unstartable `openspec` during the fallback degrades that change

- **WHEN** a change's schema is not vendored and
  `["schema", "which", <name>, "--json"]` answers `Err(CliError::NotStarted)`
- **THEN** that change carries an empty `artifacts` list and exactly one problem naming the
  schema and the program that could not be started
- **AND** nothing panics and the remaining changes are unaffected

#### Scenario: An unusable `schema.yaml` at the CLI-named path degrades that change

- **WHEN** `schema which` answers a well-formed object whose `path` names a scratch
  directory in which `schema.yaml` exists but holds bytes that are not a usable schema, and
  again one in which `schema.yaml` exists as a **directory**
- **THEN** each yields an empty `artifacts` list and exactly one problem naming the joined
  `schema.yaml` path and the reason — invalid in the first case, unreadable in the second
- **AND** exactly one `schema which` invocation is recorded in each case

#### Scenario: A malformed `schema which` payload is a problem, not a panic

- **WHEN** `schema which` answers, in turn, `Ok("")`, `Ok("null")`, `Ok("{\"path\": 7}")`,
  and `Ok("{\"name\":\"x\"}")`
- **THEN** each yields an empty `artifacts` list and exactly one problem naming the schema
  and the unusable payload
- **AND** nothing panics in any of the four cases

#### Scenario: A leading non-JSON line is not tolerated

- **WHEN** `schema which` answers
  `Ok("Note: Schema commands are experimental and may change.\n{\"path\":\"<dir>\"}")`
- **THEN** the payload is rejected as unusable and one problem is recorded
- **AND** the schema is not loaded from `<dir>`, so a future release moving the note to
  stdout degrades visibly rather than being silently absorbed

### Requirement: A schema is resolved at most once per name per call

`from_cli` SHALL cache schema resolution by name for the duration of one call, so a
repository whose twelve changes all declare `tdd` performs at most one `schema which`
invocation for `tdd` — not twelve. The cache SHALL be owned by the call, never a `static`:
`resolve::BinCache`'s reason applies unchanged, in that the suite runs this crate's tests
in parallel threads of one process and a process-global cache would let the first test
decide the answer for every other.

The cache SHALL record failures as well as successes, so a schema whose `schema which` call
failed is not retried once per change.

#### Scenario: Three changes sharing one unvendored schema ask the CLI once

- **WHEN** `list --json` reports `alpha`, `mike`, and `zulu`, each of whose apply payloads
  reports `schemaName: "spec-driven"`, the repository vendors no `spec-driven`, and the
  fake answers `["schema", "which", "spec-driven", "--json"]` with a directory holding a
  real `schema.yaml`
- **THEN** all three changes carry that schema's artifacts
- **AND** the recorded invocations hold exactly one `["schema", "which", "spec-driven",
  "--json"]` entry, and five invocations in total

#### Scenario: A failed lookup is cached rather than retried per change

- **WHEN** the same three changes each report `schemaName: "outside-in-tdd"` and the
  `schema which` call for it answers `Err(CliError::Failed)`
- **THEN** all three changes carry an empty `artifacts` list and one problem each
- **AND** exactly one `["schema", "which", "outside-in-tdd", "--json"]` invocation is
  recorded

#### Scenario: Two different schema names are asked for separately

- **WHEN** two changes report `schemaName: "spec-driven"` and `schemaName:
  "other-schema"` respectively, neither vendored, and both `schema which` calls answer
  directories holding real, distinct `schema.yaml` files
- **THEN** each change carries its own schema's artifact ids
- **AND** exactly two `schema which` invocations are recorded, one per name
