## Why

An audit differential-fuzzed the checkbox counter against the real `@fission-ai/openspec`
`TASK_LINE_PATTERN` over 60,000 adversarial lines and found **zero** disagreements, and
proved every `expect`/`unwrap` in the dual-source production halves unreachable. What it
did find are seven small parity and hygiene gaps plus one wrong sentence in `AGENTS.md`.
Two of them matter to a user: a change whose schema carries a wrong-typed `apply.tracks`
is shown a task count the OpenSpec CLI would never report, contradicting a rule `SPEC.md`
states outright; and a CLI-supplied `schemaName` is joined into a filesystem path with
none of the validation the file-sourced name gets, through a guard that exists solely for
that join.

This is **unplanned work**. The roadmap ends at Phase 6 (`degraded-states`); it planned
for building the dual-source model, not for auditing it once built.

## What Changes

- **Close C1.** A present-but-wrong-typed `apply.tracks` (`tracks: 42`, `tracks: [a]`)
  stops falling back to the artifact with id `tasks`. It is treated as a `tracks` value
  matching nothing: no tasks artifact, one named problem, and the task count falls back to
  `<change dir>/tasks.md` — which is exactly what `openspec list --json` reports for such a
  schema (verified against 1.12.0). This reverses a divergence `schema-artifacts`
  currently states deliberately.
- **Close C2.** Two guards, one for each half of the asymmetry. `schema::load` checks its
  `name` with `is_legal_name` before joining it into `<repo>/openspec/schemas/<name>` and
  returns a new `LoadError` variant when it fails, so the join every caller shares is
  guarded once. And `parse_apply` rejects an illegal CLI `schemaName` outright, so the value
  never becomes `Change::schema` or a cache key either; the trimmed form is stored, matching
  what the file path already does.
- **Close C3.** `cli_error_problem` stops discarding every diagnostic the seam carried.
  `CliError::NotStarted`'s `reason` is appended always, and `CliError::Failed`'s `stderr`
  contributes its first non-blank line whenever it is not blank. The `Failed` half is the
  half that matters: the nvm-installed `openspec` is a `#!/usr/bin/env node` shim, and with
  `node` off `PATH` it exits **127** with `env: node: No such file or directory` on stderr
  (measured). That is a `Failed`, not a `NotStarted` — `env` did start — and the probe chain
  lands on that shim precisely when the failure is possible, because `openspec`'s symlink
  shares the nvm `bin` directory with `node`. Today it renders as an unactionable
  `openspec list --json exited with code 127`. The carried line skips an informational
  `Note:` banner, because `openspec schema which` — the only command the schema fallback tier
  runs — writes one to stderr on **every** invocation; without the skip this change would
  improve one tier's rows and make the other's strictly worse.
- **Close C5.** Delete the dead first branch of `join_artifacts`, which the next branch
  fully subsumes, and fold the corresponding rule 3 out of `change-merge`'s published
  six-rule list so code and spec still enumerate the same rules.
- **Close C7.** Move the `first_document`/`schema_key` doc block onto the function it
  documents, so the load-bearing `!is_badvalue()` rationale sits on the code it protects.
- **Document C4.** Glob resolution knowingly does not follow symlinked directories while
  the CLI's fast-glob sets `followSymbolicLinks: true`. Measured against 1.12.0, that
  divergence is narrower than it first looks — it exists only where the link resolves
  *inside* the change directory; outside it the CLI fails closed instead. The narrow case
  becomes an explicit degraded-states row, because `SPEC.md` currently claims invalid UTF-8
  is "the one case" of a knowing file-vs-CLI disagreement.
- **Document C6.** A change archived between the worker's `list --json` call and its file
  walk appears in both lists of the merged `ChangeSet` for one cycle. `change-merge`
  already mandates that the two lists neither filter nor suppress one another; the race
  becomes a named, self-correcting degraded-states row rather than a code change.
- **Fix C8.** `AGENTS.md` says the change-level merge joins "by position". It joins **by
  name**; only artifacts within a change join by position. The sentence is corrected.

Not **BREAKING**: no manifest, config-format, or keybinding change.

## Non-Goals

- No new dependency, no new module, no change to the two architectural seams.
- No change to checkbox counting itself — the fuzz found it exact, and it stays exact.
- Not the general documentation-drift sweep: only the one `AGENTS.md` sentence describing
  this capability. A sibling change owns the rest.
- Nothing outside the eight listed findings, and no delta on a capability another
  in-flight change owns (`quality-gates`, `ci-workflow`, `degraded-coverage`,
  `subprocess-seam`, `watch-invalidation`, `refresh-worker`, `agent-*`,
  `terminal-lifecycle`, or the rendering capabilities).
- Still read-only, still no change authoring, still no cross-change orchestration, still
  no Windows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `schema-artifacts`: a wrong-typed `apply.tracks` yields no tasks artifact instead of
  falling back to the id, reversing a stated divergence into parity (C1); and a new
  requirement makes `schema::load` reject a name it cannot legally join (C2).
- `schema-cli-fallback`: a CLI-supplied schema name is subject to the same legality guard
  as a file-declared one before any path is joined from it (C2); its failure requirement
  inherits C3's stderr rule, since it shares `cli_error_problem` (C3).
- `cli-changes`: `parse_apply` rejects an illegal `schemaName` rather than only an empty
  one (C2); every diagnostic the seam carried — a `NotStarted` reason, and a non-blank
  `Failed` stderr — reaches the problem row (C3).
- `change-artifacts`: the symlinked-directory divergence is restated as a named, accepted
  divergence rather than an aside (C4).
- `change-merge`: the published six-rule artifact join becomes five, folding the both-empty
  rule into the CLI-empty one it duplicates (C5); the active/archived same-name race is
  stated where the pass-through rule is stated (C6).

## Impact

- Code: `src/schema.rs` (`tasks_artifact`, `load`, a new `LoadError` variant,
  `load_error_problem`, doc placement), `src/changes.rs` (`parse_apply`,
  `schema_load_problem`, `resolve_cli_schema_uncached`, `cli_error_problem`,
  `join_artifacts`).
- Documents: `SPEC.md` (Resolution chain wording, two new degraded-states rows, and three
  existing rows corrected — the no-tasks-artifact condition, the invalid-UTF-8 "one case"
  claim, and the non-zero-exit row, whose "the reason is unavailable to the plugin"
  justification is measurably false for an exec failure),
  `AGENTS.md` (the merge sentence), `tests/degraded-coverage.toml` (a proof per new row) and
  `tests/degraded_coverage.rs`'s `MIN_ROWS` floor — both satisfying `degraded-coverage`'s
  existing requirement rather than changing it.
- No manifest, no configuration, no keybinding, no dependency, no data model.
