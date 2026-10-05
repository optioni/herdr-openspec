## ADDED Requirements

### Requirement: A located schema directory is remembered for the worker's lifetime and shared with the file producer

`changes::CliCache` SHALL hold, beside its per-change entries, a map from schema name to
the **directory** a successful `["schema", "which", <name>, "--json"]` reported. That map is
the schema **locations**. The cache SHALL also hold a record of the names whose lookup left no
usable location in the current cycle, each with the one problem that outcome rendered. That
record is the **misses**. Both are owned by the `CliCache` value, never a `static`, for
`resolve::BinCache`'s reason. The refresh worker owns one `CliCache` for its whole lifetime,
so a location learned once is kept for as long as the pane runs.

Only a directory SHALL be remembered, never a parsed schema. Every load reads
`<dir>/schema.yaml` afresh through `schema::load_dir`, so an edit to a user-tier schema is
seen on the next cycle exactly as an edit to a vendored one is.

A location SHALL be remembered only when it is **usable**: its directory holds a regular
`schema.yaml`. A `schema which` payload naming an unusable directory SHALL still produce the
problem this capability already requires for it, and SHALL leave no location behind, so the
next cycle asks again.

When `from_cli_cached`'s fallback tier reaches a name the repository does not vendor, it
SHALL, in order:

1. load from the remembered location when one is usable, invoking no process;
2. otherwise, when the misses hold the name, record that miss's problem, invoking no
   process;
3. otherwise invoke `schema which` as this capability already requires. It remembers the
   location on a usable success. On **every** other outcome it records a miss holding the one
   problem that outcome renders. That covers a `CliError`, an unusable payload, and a payload
   naming a directory with no regular `schema.yaml`.

The misses SHALL be cleared at exactly two points: when a locate pass (`refresh-worker` →
"Step 2 locates every not-vendored schema before it merges") begins, and when
`from_cli_cached` returns. A worker cycle runs the locate pass and then `from_cli_cached`, so
a name whose lookup failed is asked about at most once per cycle and asked again on the next
one. A schema installed while the pane runs is therefore found without a restart. Two
`from_cli_cached` calls with no locate pass between them never share a miss, so
`refresh-worker`'s "A change whose schema the CLI rejects is cached like any other" holds
unchanged: a third, `Selection::All` call still re-asks.

The repository tier SHALL still win. `schema::load(repo, name)` is called first, and a
remembered location is consulted only when that call reports `LoadError::NotVendored`, so
vendoring a schema while the pane runs takes effect on the next cycle with no eviction step.

`changes::from_cli(cli, repo)` SHALL keep starting from an empty `CliCache`. Every scenario
this capability already states about one `from_cli` call therefore holds unchanged.

The file producer SHALL read the locations and SHALL NOT write them. Only `from_cli_cached`
and the locate pass fill the map, and both run on the worker thread.

#### Scenario: A remembered location is loaded without a spawn

- **WHEN** a `CliCache` already maps `spec-driven` to a scratch directory holding a real
  `schema.yaml` declaring four artifacts, the repository vendors no `spec-driven`, and
  `from_cli_cached` is called over `list --json` reporting `alpha`, `mike`, and `zulu`, each of
  whose apply payloads reports `schemaName: "spec-driven"`
- **THEN** all three changes carry the four artifact ids and no problem
- **AND** no `schema which` invocation is recorded

#### Scenario: A successful lookup fills the cache for the next call

- **WHEN** `from_cli_cached` is called twice with the same, initially empty, `CliCache`, over
  one change whose apply payload reports `schemaName: "spec-driven"`, and the fake answers
  `schema which spec-driven` with a directory holding a real `schema.yaml`
- **THEN** both calls' changes carry that schema's artifact ids
- **AND** exactly one `schema which` invocation is recorded across both calls

#### Scenario: An unusable directory is named and not remembered

- **WHEN** the fake answers `schema which spec-driven` with a directory that exists and holds
  no `schema.yaml`, and `from_cli_cached` is called twice with the same `CliCache`
- **THEN** each call's change has an empty `artifacts` list and one problem naming that
  directory and ending `is not vendored: no schema.yaml there`
- **AND** two `schema which` invocations are recorded, one per call, because nothing was
  remembered

#### Scenario: A miss does not outlive the call that consumed it

- **WHEN** a locate pass has recorded a miss for `spec-driven`, `from_cli_cached` has then run
  once with the same `CliCache`, and `from_cli_cached` is called a second time with no locate
  pass in between, with the fake still failing `schema which spec-driven`
- **THEN** the second call invokes `schema which spec-driven` once more
- **AND** two `schema which` invocations are recorded in total, the locate pass's and the
  second call's

#### Scenario: A miss recorded earlier in the cycle is named without a second spawn

- **WHEN** a locate pass has run over one change declaring `spec-driven`, the fake answered
  its `schema which spec-driven` with `Err(CliError::Failed)` with exit code 1, and
  `from_cli_cached` is then called with the same `CliCache` over that change
- **THEN** the change carries exactly one problem naming `openspec schema which spec-driven
  --json` and exit code 1, byte-equal to the problem a fresh `from_cli` would render for the
  same failure
- **AND** exactly one `schema which` invocation is recorded in total

#### Scenario: A schema vendored after it was located wins over the location

- **WHEN** a `CliCache` maps `spec-driven` to a scratch directory declaring four artifacts,
  and the repository then vendors its own `openspec/schemas/spec-driven/schema.yaml`
  declaring two
- **THEN** `from_cli_cached` gives each `spec-driven` change the repository's two artifact
  ids
- **AND** no `schema which` invocation is recorded

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
`CachedSchemaLoad`. They SHALL NOT be discarded. When the refresh worker has located the
schema, the file producer reads the same `schema.yaml` through the same `load_dir` and
records byte-equal messages, and `change-merge`'s duplicate collapsing leaves one copy of
each. When no location has been learned, the file producer never read the file at all, and
this producer is the **only** one that can report them.

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

#### Scenario: A parser problem read by both producers appears once after the merge

- **WHEN** the schema of the previous scenario has been located, so the file producer's
  `alpha` and the CLI producer's `alpha` each carry that one parser message, and the two are
  merged
- **THEN** the merged `alpha`'s `problems` hold that parser message exactly once

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
