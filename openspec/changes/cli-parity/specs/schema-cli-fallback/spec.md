## MODIFIED Requirements

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

On the illegal-name variant `schema-artifacts` requires — the name could not be joined into
a path at all — it SHALL also NOT invoke the CLI, and SHALL record one problem naming the
rejected value. Asking `schema which` about a name that cannot be a directory segment is a
spawn whose only possible answer is an error, and the plugin would then have to reject the
`path` that answer named anyway. `cli-changes` already drops a change whose apply payload
reports such a `schemaName`, so this branch is reached only by a direct caller of
`resolve_cli_schema`; it is specified because the resolver is total over `LoadError` and
must not silently treat an unreachable variant as repairable.

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

#### Scenario: An illegal schema name is not repaired by the CLI

- **WHEN** `resolve_cli_schema` is called directly with the name `../../../../etc` against
  a scratch repository, and the fake has no `schema which` registration
- **THEN** it produces no schema and exactly one problem naming the rejected value
- **AND** no invocation is recorded at all, so a name that cannot be a path segment costs
  no process start
- **AND** nothing outside the scratch tree is read
