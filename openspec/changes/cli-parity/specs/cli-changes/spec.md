## MODIFIED Requirements

### Requirement: Every CLI failure degrades to the file result and names itself

`from_cli` SHALL never fail closed. Each failure below SHALL leave the affected changes out
of `active` — so `change-merge` keeps the file result for them — and SHALL record exactly
one human-readable problem naming what failed.

- **The `openspec` program could not be started** (`CliError::NotStarted`) — `active` is
  empty and one problem names the program, the `list --json` vector, **and the `reason` the
  seam carried**. This is `SPEC.md` → Degraded states' "`openspec` binary not found"
  reaching this module.
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
- **A change's apply output reports a `schemaName` that is not a legal schema name** — that
  change is absent from `active` and one problem names the change and the rejected value.
  `parse_apply` SHALL apply `schema-selection`'s `is_legal_name` guard, not merely a
  non-empty check.

`is_legal_name` trims before validating, so `parse_apply` SHALL store the **trimmed** value
and never the padded one. `schema_key` already trims a file-declared name for exactly this
reason — accept and use must agree on one form, or a padded segment reaches the schema path
join. An empty or whitespace-only `schemaName` remains covered by the existing missing-field
branch, whose message differs; both outcomes drop the change, and the two messages are not
required to be equal.

A `NotStarted` reason SHALL be carried into the problem text; a `Failed` stderr SHALL NOT.
The two are not symmetric and the asymmetry is the point: a `Failed` stderr is empty because
`openspec` writes its diagnostic to stdout, so there is nothing to report, while a
`NotStarted` reason is the **operating system's** message about the spawn itself — it
distinguishes a binary deleted between probe and run from one with a bad interpreter line,
and it is the only diagnostic an `openspec` failure ever supplies.

`schemaName` is rejected rather than sanitized, and rejected at the parse rather than at the
use, because the value does not only reach `schema::load`'s path join: it also becomes the
`Change::schema` string the detail header renders and the key of the per-call schema cache.
A guard at the join alone would leave a traversing name on screen and in the cache.
`schema-artifacts` requires `schema::load` to guard the join as well; the two together are
defence in depth, and neither is a substitute for the other.

A per-change failure SHALL NOT abort the remaining changes.

#### Scenario: An absent `openspec` binary yields an empty result and one problem

- **WHEN** the fake answers `["list", "--json"]` with
  `Err(CliError::NotStarted { program: "openspec", args: ["list", "--json"], reason: "No such file or directory (os error 2)" })`
- **THEN** `active` is empty and `problems` holds exactly one entry naming `openspec`, the
  `list --json` invocation, and the text `No such file or directory (os error 2)`
- **AND** no apply invocation is recorded, and nothing panics

#### Scenario: Two different spawn failures produce two different problems

- **WHEN** the same `list --json` call is run twice, once answering
  `Err(CliError::NotStarted { reason: "No such file or directory (os error 2)" })` and once
  `Err(CliError::NotStarted { reason: "Exec format error (os error 8)" })`
- **THEN** the two recorded problems differ
- **AND** an implementation that discards `reason` produces two byte-identical strings and
  fails this scenario, which is the whole content of the finding

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

#### Scenario: An apply payload whose `schemaName` would escape the schema directory is refused

- **WHEN** `list --json` reports `alpha` and `mike`, and `alpha`'s apply call answers a
  well-formed payload carrying `"schemaName": "../../../../etc"`, a valid `changeDir`, and
  a valid `contextFiles` object
- **THEN** `alpha` is absent from `active` and exactly one problem names `alpha` and the
  string `../../../../etc`
- **AND** `mike` is produced normally, so one poisoned payload does not empty the result
- **AND** no `schema which` invocation is recorded for `../../../../etc`, which is the
  observable that distinguishes the guard from its absence
- **AND** the string `../../../../etc` appears in no produced `Change`'s `schema` field

#### Scenario: Every other shape `is_legal_name` rejects is refused the same way

- **WHEN** an apply payload's `schemaName` is in turn `"   "`, `"."`, `".."`, `"a/b"`,
  `"a\\b"`, and an absolute path
- **THEN** each drops its change from `active` with exactly one problem naming the change and
  the rejected value
- **AND** a `schemaName` of `spec-driven`, and one of `spec-driven.v2`, are both accepted,
  so the guard rejects the escaping shapes rather than everything
- **AND** a `schemaName` of `" tdd "` is accepted and the produced `Change` carries the
  trimmed `tdd`, so no padded segment can reach a path join
- **AND** a `schemaName` of `""` also drops its change, through the existing missing-field
  branch rather than the guard, which the test asserts by message rather than by outcome

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
