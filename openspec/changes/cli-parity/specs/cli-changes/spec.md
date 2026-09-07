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
  problem names the vector, the exit code, **and the first non-blank line of `stderr` when
  `stderr` is not blank**.
- **`list --json` output is not JSON, or not the expected envelope** — `active` is empty
  and one problem names the payload as unusable.
- **A change's apply call failed, for any reason** — that change is absent from `active`
  and one problem names the change, the vector, the exit code, and the same conditional
  `stderr` line. A schema the CLI rejects is the empty-`stderr` case: the CLI answers exit 1
  with its diagnostic on **stdout** and a **0-byte stderr** (`dist/cli/index.js:624-627`,
  measured), and `subprocess-seam`'s `CliError::Failed` carries stderr only, so for that
  failure the problem SHALL name the change, the vector, and the exit code and nothing more.
- **A change's apply output is not JSON, or lacks `schemaName`, `changeDir`, or
  `contextFiles`** — that change is absent from `active` and one problem names it.
- **A change's apply output reports a `schemaName` that is not a legal schema name** — that
  change is absent from `active` and one problem names the change and the rejected value.
  `parse_apply` SHALL apply `schema-selection`'s `is_legal_name` guard, not merely a
  non-empty check.

`is_legal_name` trims before validating, so `parse_apply` SHALL store the **trimmed** value
and never the padded one. `schema_key` already trims a file-declared name for exactly this
reason — accept and use must agree on one form, or a padded segment reaches the schema path
join. An empty `schemaName` remains covered by the existing missing-field branch — the
`required_non_empty_str` filter that branch relies on does not trim, so a whitespace-only
value such as `"   "` is not empty and instead reaches the new guard, which rejects it — the
two branches' messages differ; both outcomes drop the change, and the two messages are not
required to be equal.

**Both** carried diagnostics SHALL reach the problem text: a `NotStarted` reason always, and
a `Failed` stderr whenever it is not blank. Neither is dropped, and the rule is one rule
rather than two: *report every diagnostic the seam actually carried*.

The earlier rule — that a `Failed` stderr is never worth reporting because `openspec` writes
its diagnostics to stdout — is true only of `openspec`'s **own** diagnostics. It is false for
a failure of the **shim**, which is reachable by construction on this project's reference
machine. `~/.nvm/versions/node/<v>/bin/openspec` is a symlink to a `.js` file whose first
line is `#!/usr/bin/env node`. Run with `node` off `PATH`, it exits **127** with an empty
stdout and `env: node: No such file or directory` on **stderr** (measured). That is
`CliError::Failed`, not `NotStarted` — `env` started, so `Command::output()` returns `Ok`
with a non-zero status — and the probe chain lands on that shim precisely when the failure
is possible: `openspec`'s symlink lives in the same nvm `bin` directory as `node`, so
whenever that directory is off `PATH`, `resolve::step2_path` misses and `step3_nvm` finds
the shim anyway.

Dropping stderr therefore turns the one self-diagnosing failure in the crate into
`openspec list --json exited with code 127` — a row naming a number the user cannot act on.

What is carried is the **first non-blank line that is not an informational `Note:` banner**,
**trimmed**, and nothing else. Three clauses, each earning its place:

- **First line, not the whole stream** — a problem renders as one row of the list region, so a
  multi-line diagnostic would be truncated by the view or break the row grammar.
- **Trimmed** — the line is joined onto an existing sentence, and padding from the child's own
  formatting would show up inside it.
- **Not a `Note:` banner** — a line whose trimmed form begins `Note: ` SHALL be skipped, and
  the next non-blank line considered in its place. This is not a hypothetical: `openspec
  schema which` writes `Note: Schema commands are experimental and may change.` to **stderr on
  every invocation, success and failure alike** (`dist/commands/schema.js:388-391`, a
  `preAction` hook; measured on 1.12.0 at exit 0 and exit 1). Without this clause, the one
  command `schema-cli-fallback` runs would append that banner to every failure it reports,
  replacing a row that says nothing with a row that appears to explain the failure and does
  not. Skipping a genuine diagnostic that happened to begin `Note: ` costs nothing this rule
  was ever going to deliver, because a line that announces itself as a note is not a
  diagnosis.

A stderr that is blank, or that holds nothing but `Note:` banners, appends nothing — so
`openspec`'s own stdout-diagnostic failures read exactly as they did.

A `NotStarted` reason is the **operating system's** message about the spawn itself — it
distinguishes a binary deleted between probe and run from one with an unusable interpreter
line — and it is carried unconditionally.

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

#### Scenario: An exec failure of the `openspec` shim reports its own stderr

- **WHEN** the fake answers `["list", "--json"]` with
  `Err(CliError::Failed { code: Some(127), stderr: "env: node: No such file or directory\n" })`
  — the exact shape measured by running the nvm-installed `openspec` shim with `node` off
  `PATH`
- **THEN** `active` is empty and `problems` holds exactly one entry naming the
  `list --json` vector, exit code `127`, and the text `env: node: No such file or directory`
- **AND** an implementation that discards `stderr` produces a row naming only the code and
  fails this scenario
- **AND** the trailing newline is not in the recorded problem, because the first non-blank
  line is taken trimmed

#### Scenario: A multi-line stderr contributes only its first non-blank line, trimmed

- **WHEN** a `Failed` carries `stderr` `"\n\n  first line  \nsecond line\nthird line"`
- **THEN** the recorded problem **ends with** `code 1: first line` — asserted as an exact
  whole-string equality, not a `contains`
- **AND** it holds neither `second line` nor `third line`, and no doubled space, so an
  implementation that takes the first non-blank line **without** trimming it fails this
  scenario. That is the only scenario in this capability where the trim is observable, so
  dropping the assertion leaves the trim unproven
- **AND** the problem is a single line, so it renders as one row of the list region

#### Scenario: A `Note:` banner is skipped and the next line carried

- **WHEN** a `Failed` carries `stderr`
  `"Note: Schema commands are experimental and may change.\nreal diagnosis here"`
- **THEN** the recorded problem ends with `real diagnosis here` and contains neither `Note:`
  nor `experimental`
- **AND** with the banner alone as the whole of `stderr` — the shape `openspec schema which`
  actually produces — the problem is byte-identical to the empty-`stderr` one, so the banner
  contributes nothing rather than an empty fragment
- **AND** the skip is by the trimmed line's `Note: ` prefix, asserted by a third run whose
  stderr is `"   Note: padded banner\nkept"`, which also carries `kept`

#### Scenario: A whitespace-only stderr appends nothing

- **WHEN** a `Failed` carries `stderr` `"   \n\t\n"`
- **THEN** the recorded problem is byte-identical to the one produced for an empty `stderr`
- **AND** it ends at the exit code, with no trailing separator left dangling

#### Scenario: A schema the CLI rejects removes one change and keeps the others

- **WHEN** `list --json` reports `alpha`, `mike`, and `zulu`, and the apply call for `zulu`
  answers `Err(CliError::Failed { code: Some(1), stderr: "" })` while the other two succeed
- **THEN** `active` holds `alpha` and `mike` only, in that order
- **AND** `problems` holds exactly one entry naming `zulu`, the vector
  `["instructions", "apply", "--change", "zulu", "--json"]`, and exit code `1`
- **AND** that entry appends no reason, because the fake's `stderr` is empty — the CLI wrote
  its diagnostic to stdout, which the seam discards on a non-zero exit

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
- **AND** no apply invocation is recorded, and the entry appends no reason, because stderr
  was empty

#### Scenario: Empty stdout from `list --json` is a parse failure, not an empty repository

- **WHEN** `list --json` answers `Ok("")`
- **THEN** `active` is empty and exactly one problem names the payload as unusable
- **AND** the problem is distinguishable from the "no active changes" state, which records
  no problem at all
