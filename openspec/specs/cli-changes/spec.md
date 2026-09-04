# cli-changes Specification

## Purpose
TBD - created by archiving change changes-from-cli. Update Purpose after archive.

## Requirements

### Requirement: Two commands produce the CLI's view of the active changes

`changes::from_cli` SHALL take a `&dyn cli::OpenspecCli` and the repository root and SHALL
reach the `openspec` program **only** through that trait object. It SHALL name no
process-spawn API: `cli` remains the crate's one spawning module, and `from_cli` is its
first real consumer.

It SHALL run exactly two kinds of invocation **of its own**, with these exact argument
vectors:

1. `["list", "--json"]` — once per call.
2. `["instructions", "apply", "--change", <name>, "--json"]` — once per change the first
   call reported, in the order the result is emitted in.

The only other invocation it may make is `["schema", "which", <name>, "--json"]`, which
`schema-cli-fallback` specifies and bounds. No other vector SHALL be run.

It SHALL NOT run `openspec status --change <name> --json`. That command carries
`schemaName`, `changeRoot`, and per-artifact existing paths computed by the *same*
`resolveArtifactOutputs` the apply payload's `contextFiles` uses
(`dist/core/artifact-graph/instruction-loader.js:224` and
`dist/commands/workflow/instructions.js:273`, `@fission-ai/openspec@1.11.0`), so it adds a
second 200–400ms Node start per change for information already in hand. Its `artifacts`
array is additionally **topologically sorted by build order**, not the schema's declared
order (`instruction-loader.js:258-261`), so it is not even the right source for the
positional join.

`from_cli` SHALL return a `CliChanges` value carrying an `active: Vec<Change>` and a
`problems: Vec<String>`. It SHALL NOT return a `Result`, SHALL NOT panic, and SHALL NOT
`unwrap` on any input, matching `from_files`' total shape. It SHALL NOT carry an
`archived` list: `openspec list --json` filters the `archive` directory out
(`dist/core/list.js:85-87`) and the CLI can address no change beneath it, so archived
changes stay file-sourced as `change-model` requires.

Each produced `Change` SHALL be built by naming every field explicitly — no `Default`, no
functional-update `..` — and every value a test builds SHALL be passed through
`changes::conformance::assert_invariants`, the same function `from_files`' tests call.

#### Scenario: A two-change repository drives exactly three invocations

- **WHEN** a fake `OpenspecCli` answers `["list", "--json"]` with a payload naming changes
  `zulu` and `alpha` (in that order, most-recently-modified first) and answers each
  `["instructions", "apply", "--change", <n>, "--json"]` with a well-formed payload naming
  a schema the scratch repository **does** vendor, so no `schema which` invocation is due
- **THEN** the fake records exactly three invocations, all on the `OpenspecCli` side, in
  the order `["list","--json"]`, then the two apply vectors
- **AND** no vector beginning `status` is recorded, and no vector carries any argument
  beyond the ones listed above

#### Scenario: No status invocation is made even when a change's apply call fails

- **WHEN** the apply call for `zulu` is registered as `Err(CliError::Failed)` and the fake
  has **no** registration for any `status` vector
- **THEN** `from_cli` returns without panicking, so no `status` invocation was attempted —
  the fake panics on an unregistered pair, which is what makes this assertion discriminate

#### Scenario: `from_cli` names no process-spawn API

- **WHEN** the tree-wide architectural check is run over `src/`, excluding `src/cli.rs` by
  path
- **THEN** it reports no match, so the new code in `src/changes.rs` reaches the program
  only through the trait object
- **AND** the check still fails when `src/cli.rs` is absent, when `src/cli.rs` itself names
  no spawn API, and when the set of files searched is smaller than the crate's module count

### Requirement: The list payload is an envelope, and progress comes from it

`openspec list --json` SHALL be parsed as the object
`{"changes": [ … ], "root": {"path": <string>, "source": <string>}}` — **not** as a bare
array. Each element of `changes` carries `name` (string), `completedTasks` (number),
`totalTasks` (number), `lastModified` (string), and `status` (string), all five always
present (`dist/core/list.js:119-125`).

`name` SHALL be required to be a **non-empty** string and `completedTasks` / `totalTasks`
non-negative integers; an entry failing any of those is skipped with one problem. An empty
`name` is rejected rather than carried, because `conformance::assert_invariants` requires a
non-empty `name` and would panic on the value instead of degrading.

A `name` appearing more than once in `changes` SHALL keep the **first** occurrence and
record one problem naming the duplicate. The real CLI enumerates directory entries and
cannot emit a duplicate, but `merge` pairs by name and a second entry would silently
overwrite the first.

A change's `progress` SHALL be `Progress { completed: completedTasks, total: totalTasks }`
from this payload, and SHALL NOT be taken from the apply payload's
`progress {total, complete, remaining}`. The two are different computations: `list` resolves
the tracked-tasks artifact's `generates` through the same glob expansion the file producer
uses and **sums every matched file**, falling back to `<changeDir>/tasks.md`
(`dist/utils/task-progress.js:120-133`), while apply resolves `apply.tracks` as a single
path with no globbing (`dist/commands/workflow/instructions.js:278-327`). They agree only
when `apply.tracks` is a plain filename. `SPEC.md` → Dual-source model requires both of
this plugin's producers to report the same number for the same change, and the file
producer reproduces `list`'s rule; taking apply's would break that agreement for exactly
the schemas whose `tracks` is a glob.

`lastModified` and `status` SHALL be discarded rather than stored: `change-model` forbids
both fields on `Change`.

#### Scenario: Progress is read from the list payload's pair

- **WHEN** `list --json` reports `alpha` with `completedTasks: 4` and `totalTasks: 9`, and
  the apply payload for `alpha` reports `progress: {total: 0, complete: 0, remaining: 0}`
- **THEN** the produced `Change` for `alpha` has `progress == Progress { completed: 4,
  total: 9 }`
- **AND** the value `0/0` appears nowhere in the result, so the apply payload's own
  progress was not consulted

#### Scenario: A bare array is rejected rather than parsed

- **WHEN** `list --json` answers with a bare JSON array of change objects, with no
  enclosing object
- **THEN** `from_cli` returns an empty `active` list and exactly one problem naming the
  `openspec list --json` payload as unusable
- **AND** nothing panics

#### Scenario: A change entry missing a required field is skipped, not fatal

- **WHEN** `list --json` reports three entries, of which the middle one has no
  `totalTasks` key and another has a `name` that is a number rather than a string
- **THEN** the one well-formed entry produces a `Change`, the two malformed entries
  produce no `Change`, and exactly two problems are recorded, each naming the entry's
  zero-based position in the `changes` array
- **AND** the surviving change is still fully populated, so one bad entry does not
  discard the payload

#### Scenario: An empty change list is a supported empty state

- **WHEN** `list --json` answers `{"changes": [], "root": {"path": <repo>, "source":
  "nearest"}}`
- **THEN** `active` is empty, `problems` is empty, and no apply invocation is recorded

#### Scenario: An entry whose name is the empty string is skipped

- **WHEN** `list --json` reports two entries, one with `"name": ""` and one well-formed
- **THEN** only the well-formed entry produces a `Change`, and exactly one problem names
  the empty name at its position
- **AND** nothing panics, which an implementation that carried the empty name into
  `conformance::assert_invariants` would not manage

#### Scenario: A repeated name keeps the first entry and names the duplicate

- **WHEN** `list --json` reports `alpha` with `1/2`, then `mike`, then `alpha` again with
  `9/9`
- **THEN** `active` holds one `alpha`, carrying `progress { completed: 1, total: 2 }` from
  the first occurrence, and one `mike`
- **AND** exactly one problem names `alpha` as reported twice

### Requirement: The CLI's active list is re-sorted by name in byte order

`from_cli` SHALL sort its `active` list by `name` ascending, comparing **bytes**, before
returning it. This is `change-enumeration`'s "Active changes are ordered by name ascending,
in byte order" requirement, which states the obligation on this producer explicitly; the
scenarios below are where it is verified for the CLI side.

`from_cli` SHALL NOT pass `--sort name` to obtain the order. That flag exists, but it sorts
with `a.name.localeCompare(b.name)` (`dist/core/list.js:111-116`), which is a locale
collation — case- and accent-folding, and machine-dependent — whereas the file producer
sorts Rust `String` values, which is byte order. Delegating the sort would make the two
producers disagree on any repository holding a change whose name differs from another's
only by case, by a leading underscore, or by a non-ASCII character, and disagreement is the
one visible failure of the dual-source model.

#### Scenario: The most-recently-modified default order is replaced by byte order

- **WHEN** `list --json` answers with changes in its own default order — `zulu`, `mike`,
  `alpha`, newest first — and each apply call succeeds
- **THEN** `active` is ordered `alpha`, `mike`, `zulu`
- **AND** the apply invocations were made in the payload's own order, so the sort is
  applied to the result rather than to the invocation sequence

#### Scenario: Case and digits order by byte, not by locale

- **WHEN** `list --json` reports `Beta`, `alpha`, `10-late`, and `2-early`
- **THEN** `active` is ordered `10-late`, `2-early`, `Beta`, `alpha` — the same order
  `change-enumeration` requires of the file producer
- **AND** `Beta` precedes `alpha`, which a `localeCompare` sort would reverse, so the
  assertion discriminates against having passed `--sort name`

#### Scenario: The argument vector carries no sort flag

- **WHEN** any `from_cli` call is made against a fake
- **THEN** the recorded `list` vector is exactly `["list", "--json"]`
- **AND** it contains no `--sort` and no `name`, so the sort could not have been delegated

### Requirement: A change's artifacts are placed by schema position from `contextFiles`

For each change, `from_cli` SHALL resolve the change's schema — by the name the apply
payload reports as `schemaName`, through `schema-cli-fallback` — and SHALL build one
`ArtifactRef` per schema artifact, **in the schema's declared order**, whose `paths` are
`contextFiles[<that artifact's id>]` when the key is present and empty otherwise.

An artifact matching no file SHALL be **omitted** from `contextFiles` entirely rather than
present with an empty array (`dist/commands/workflow/instructions.js:270-277`: the writes
are guarded by `if (outputs.length > 0)`). A missing key is therefore the normal
"No content yet" state and SHALL NOT record a problem.

Artifacts SHALL be placed by walking the schema's declared list and reading `contextFiles`
by id. This id lookup is **within one producer** — the CLI's own id-keyed payload onto the
CLI's own positional list — and is not the cross-producer join, which `change-merge`
specifies as positional. Where a schema declares the same id at two positions, both
positions SHALL receive that id's `contextFiles` entry, since the payload cannot
distinguish them; this is unreachable through a real CLI, which rejects a duplicate-id
schema outright (`dist/core/schema.js:29-30, 40-48`, `Duplicate artifact ID`), and is
specified so the placement function is total.

Artifact `paths` SHALL be taken verbatim from `contextFiles`, absolute and already
canonicalized by the CLI (`fs.realpathSync.native`, `dist/utils/file-system.js:76-89`), and
SHALL NOT be re-resolved, re-sorted, or made relative.

#### Scenario: An omitted `contextFiles` key becomes an empty path list at its position

- **WHEN** the change's schema is `tdd`, declaring `proposal`, `specs`, `design`, `tasks`,
  `planning-review` in that order, and `contextFiles` holds keys for `proposal`, `specs`,
  and `tasks` only
- **THEN** the `Change` has five `ArtifactRef`s with those five ids in that order
- **AND** `design` and `planning-review` each carry an empty `paths`, and no problem is
  recorded for either

#### Scenario: A multi-file artifact keeps the CLI's own list, in the CLI's own order

- **WHEN** `contextFiles["specs"]` holds two absolute paths ending
  `specs/cap-one/spec.md` and `specs/cap-two/spec.md`
- **THEN** the `specs` `ArtifactRef` carries exactly those two paths, in that order, as
  absolute paths
- **AND** neither is joined onto the change directory, shortened, or re-sorted

#### Scenario: A `contextFiles` key naming no schema artifact is ignored

- **WHEN** `contextFiles` holds a key `legacy` that the resolved schema does not declare
- **THEN** the produced `artifacts` hold exactly the schema's ids and no entry named
  `legacy`
- **AND** exactly one problem is recorded naming `legacy` as reported by the CLI but not
  declared by the schema, so the divergence is visible rather than silently dropped

#### Scenario: A duplicate schema id gives both positions the same paths

- **WHEN** the artifact-placement function is called directly with a schema declaring ids
  `zeta`, `alpha`, `zeta` and a `contextFiles` holding one entry for `zeta`
- **THEN** three `ArtifactRef`s are produced, at positions 0, 1, 2, with ids `zeta`,
  `alpha`, `zeta`
- **AND** positions 0 and 2 carry that entry's paths and position 1 carries none, so the
  list is neither de-duplicated nor truncated

### Requirement: Every one of `Change`'s seven fields comes from CLI data

`from_cli` SHALL supply all seven fields of `Change` without a default and without a
functional update, from these sources:

| Field | Source |
|---|---|
| `name` | the list payload's `name` |
| `dir` | the apply payload's `changeDir` |
| `origin` | `Origin::Active` by construction — `openspec list --json` filters `archive` out of its walk, so nothing it reports is archived |
| `schema` | the apply payload's `schemaName` |
| `artifacts` | the resolved schema's declared order plus `contextFiles`, per the requirement above |
| `progress` | the list payload's `completedTasks` / `totalTasks` |
| `problems` | accumulated while producing the value |

There is therefore no field this producer cannot supply, and no argument for relaxing
`change-model`'s gate on its account.

`changeDir` SHALL be required to have a final path component **equal to the change's
name**; a payload whose `changeDir` does not is a per-change failure recording one problem
naming both, not a `Change`. The real CLI joins the change directory from the name and
always satisfies this, but `conformance::assert_invariants` requires it of every `Active`
value and would panic rather than degrade — so the check belongs at the boundary, where a
malformed payload is still data rather than a bug.

#### Scenario: A `changeDir` whose final component is not the change name is rejected

- **WHEN** the apply payload for `alpha` reports
  `changeDir: "<repo>/openspec/changes/beta"`
- **THEN** `alpha` is absent from `active` and exactly one problem names both `alpha` and
  that directory
- **AND** nothing panics, and the other changes are unaffected

#### Scenario: Every produced change is `Active` and satisfies the shared invariants

- **WHEN** `from_cli` runs against a payload holding three well-formed changes
- **THEN** every produced `Change` has `origin == Origin::Active`
- **AND** every one passes `changes::conformance::assert_invariants`, the same function
  `from_files`' tests call, with no second copy of the invariants written for this producer

### Requirement: Every CLI failure degrades to the file result and names itself

`from_cli` SHALL never fail closed. Each failure below SHALL leave the affected changes out
of `active` — so `change-merge` keeps the file result for them — and SHALL record exactly
one human-readable problem naming what failed.

- **The `openspec` program could not be started** (`CliError::NotStarted`) — `active` is
  empty and one problem names the program and the `list --json` vector. This is
  `SPEC.md` → Degraded states' "`openspec` binary not found" reaching this module.
- **`list --json` exited non-zero** (`CliError::Failed`) — `active` is empty and one
  problem names the vector and the exit code.
- **`list --json` output is not JSON, or not the expected envelope** — `active` is empty
  and one problem names the payload as unusable.
- **A change's apply call failed, for any reason** — that change is absent from `active`
  and one problem names the change and the vector. A schema the CLI rejects is exactly this
  case: the CLI answers exit 1 with its diagnostic on **stdout** and an **empty stderr**
  (`dist/cli/index.js:624-627`), and `subprocess-seam`'s `CliError::Failed` carries stderr
  only, so the recorded problem SHALL name the change, the argument vector, and the exit
  code, and SHALL NOT claim to report the CLI's own message.
- **A change's apply output is not JSON, or lacks `schemaName`, `changeDir`, or
  `contextFiles`** — that change is absent from `active` and one problem names it.

A per-change failure SHALL NOT abort the remaining changes.

#### Scenario: An absent `openspec` binary yields an empty result and one problem

- **WHEN** the fake answers `["list", "--json"]` with
  `Err(CliError::NotStarted { program: "openspec", args: ["list", "--json"], reason: … })`
- **THEN** `active` is empty and `problems` holds exactly one entry naming `openspec` and
  the `list --json` invocation
- **AND** no apply invocation is recorded, and nothing panics

#### Scenario: A schema the CLI rejects removes one change and keeps the others

- **WHEN** `list --json` reports `alpha`, `mike`, and `zulu`, and the apply call for `zulu`
  answers `Err(CliError::Failed { code: Some(1), stderr: "" })` while the other two succeed
- **THEN** `active` holds `alpha` and `mike` only, in that order
- **AND** `problems` holds exactly one entry naming `zulu`, the vector
  `["instructions", "apply", "--change", "zulu", "--json"]`, and exit code `1`
- **AND** that entry does not assert any reason for the failure, because the CLI wrote its
  diagnostic to stdout, which the seam discards on a non-zero exit

#### Scenario: Malformed JSON from a single apply call is contained to that change

- **WHEN** the apply call for `mike` answers `Ok("{ this is not json")` and the other two
  succeed
- **THEN** `active` holds `alpha` and `zulu`, and exactly one problem names `mike`
- **AND** the two surviving changes carry their full artifact lists, so the parse failure
  did not poison the shared schema resolution

#### Scenario: An apply payload missing `contextFiles` is a per-change failure

- **WHEN** the apply call for `alpha` answers a well-formed JSON object carrying
  `changeName`, `changeDir`, and `schemaName` but no `contextFiles` key
- **THEN** `alpha` is absent from `active` and exactly one problem names it
- **AND** no `Change` is produced with an empty artifact list in its place, because an
  empty list would be indistinguishable from a change whose artifacts are genuinely
  unwritten

#### Scenario: A non-zero exit from `list --json` yields an empty result and one problem

- **WHEN** the fake answers `["list", "--json"]` with
  `Err(CliError::Failed { code: Some(1), stderr: "" })` — the shape the CLI produces when
  the working directory holds no `openspec/` at all, or when the repository root cannot be
  resolved, since it writes its diagnostic to stdout and exits 1
- **THEN** `active` is empty and `problems` holds exactly one entry naming the
  `list --json` vector and exit code `1`
- **AND** no apply invocation is recorded, and the entry claims no reason for the failure,
  because stderr was empty

#### Scenario: Empty stdout from `list --json` is a parse failure, not an empty repository

- **WHEN** `list --json` answers `Ok("")`
- **THEN** `active` is empty and exactly one problem names the payload as unusable
- **AND** the problem is distinguishable from the "no active changes" state, which records
  no problem at all

### Requirement: A CLI answering for a different repository is discarded whole

`from_cli` SHALL compare the `root.path` the `list --json` envelope reports against the
repository root it was given. When the two do not name the same directory, it SHALL return
an empty `active` and exactly one problem naming both paths, so the merge keeps the file
result entirely.

The guard is necessary rather than defensive. `resolve::find_repo` walks up from the
*invocation context's workspace working directory*, while the OpenSpec CLI resolves its
own root by walking up from the **process working directory** — and `subprocess-seam`
requires the real implementation never to set `current_dir`. When a plugin process is
started with a working directory outside, or in a sibling of, the repository the dashboard
resolved, the CLI answers truthfully about a different repository, and merging that answer
would replace one repository's rows with another's.

Comparison SHALL be made on canonicalized paths where the filesystem can canonicalize both,
falling back to comparing the paths as given when it cannot, so a symbolic link in either
path is not read as a disagreement.

A `root` key that is absent or whose `path` is not a string SHALL be treated as a
disagreement, not as consent.

#### Scenario: A mismatched root discards the whole CLI result

- **WHEN** `from_cli` is given a repository root `<scratch>/repo-a` and `list --json`
  answers an envelope whose `root.path` is `<scratch>/repo-b`, with two well-formed changes
- **THEN** `active` is empty and `problems` holds exactly one entry naming both
  `<scratch>/repo-a` and `<scratch>/repo-b`
- **AND** no apply invocation is recorded, so the guard runs before the per-change calls

#### Scenario: A symlinked repository root is not a disagreement

- **WHEN** the repository root given to `from_cli` is a symbolic link that resolves to the
  directory `list --json` reports as `root.path`
- **THEN** the result is accepted and the changes are produced normally
- **AND** no problem is recorded

#### Scenario: An envelope with no `root` is treated as a disagreement

- **WHEN** `list --json` answers `{"changes": [ … one well-formed change … ]}` with no
  `root` key at all
- **THEN** `active` is empty and exactly one problem names the missing root
- **AND** no apply invocation is recorded

### Requirement: Producing changes from the CLI writes nothing

`from_cli` SHALL create, modify, and delete no file and no directory. It reads the
repository only through `schema::load` / `schema::load_dir`, and reaches the `openspec`
program only through `OpenspecCli`, whose real implementation `subprocess-seam` already
proves writes nothing.

#### Scenario: A full `from_cli` run leaves the tree byte-identical

- **WHEN** a scratch tree holding an `openspec/schemas/` directory, an
  `openspec/changes/` directory, and an empty directory is snapshotted — every path,
  every file's bytes, and every entry's modification time — and a `from_cli` call covering
  a successful change, a change whose apply call failed, a schema resolved through the CLI
  fallback tier, and a malformed payload is then performed against it
- **THEN** a second snapshot equals the first exactly, with no entry added, removed, or
  modified
- **AND** a snapshot of the test process's own working directory, taken around the same
  call, is likewise unchanged
