## Why

The pane does not fully recognise `spec-driven`, the schema the OpenSpec CLI ships inside its
own package and no repository vendors. Run against a real `spec-driven` repository
(`~/Code/dungeons-and-dragons`, `openspec` 1.13.2 from Homebrew, where `openspec schema which
spec-driven --json` reports `source: "package"`), two things go wrong:

1. **Active changes** get the right tabs, because `from_cli`'s `schema which` tier repairs
   them. But every one of them still draws a `! <repo>/openspec/schemas/spec-driven/schema.yaml is
   not vendored: no schema.yaml there` problem row in its detail pane. The file producer records
   that message, and `change-merge` keeps every file-side problem. Its spec states this as
   an accepted consequence, and `a_file_side_message_survives_beside_a_corrected_artifact_list`
   pins it. It was invisible until `degraded-states` began drawing a change's problems as
   rows in its own detail pane.
2. **Archived changes** draw `no artifacts` plus that same row. Archived changes come only
   from the file producer, and the `schema which` tier exists only inside `from_cli`, so an
   archived `spec-driven` change can never have its schema found.

`spec-driven` is OpenSpec's default schema, so this affects nearly every repository that
doesn't vendor a schema of its own. The pane reports a fault for a schema the installed CLI
resolves without trouble.

## What Changes

- The schema **location** that `openspec schema which <name> --json` reports is learned once
  and remembered by the refresh worker for its whole lifetime, in the `CliCache` it already
  owns. Today the result is thrown away at the end of every `from_cli` call.
- The **file producer** resolves a not-vendored schema name through those remembered
  locations: the repository tier first, then a remembered directory loaded with the same
  `schema::load_dir`. Archived changes therefore get their artifacts. They stay
  *file-sourced*: every byte is still read from disk, and only the directory comes from the
  CLI.
- The worker's step 2 gains a **locate pass** before the merge. For every distinct schema
  name among the changes it built that the repository does not vendor and the worker has not
  located yet, it asks `schema which` once. When that teaches it a new location, it rebuilds
  the file set before merging. `from_cli`'s own fallback tier reads and fills the same
  locations, so `schema which` runs once per name per worker, not once per cycle.
- A remembered location whose directory no longer holds a `schema.yaml` (for example after a
  `brew upgrade` moves the Cellar path) is dropped and asked for again. A failed lookup is
  remembered for one cycle only, so a schema installed later is still found.
- The worktree overlay reads member changes with the same locations, so a `spec-driven`
  change owned by a linked worktree resolves too.
- The "not vendored" problem is therefore never produced for a schema the CLI can locate.
  The merge rule ("problems are kept in full, never dropped") **does not change**. Only its
  stated consequence does: the message it described no longer reaches the merge.
- `SPEC.md`'s Schema paragraph and its "Schema not vendored" degraded-states row are amended
  to describe both producers and the remaining file-mode case.

## Non-Goals

- **No change in file mode.** With no `openspec` binary there is nothing to ask, and the "not
  vendored" row stays. Guessing package paths next to a resolved binary is a separate
  decision.
- **No spawn on the render path or before the first frame.** Step 1, the fast file answer,
  still spawns nothing. So on a fresh pane, the first ~300ms of a `spec-driven` change can
  still show the row until step 2 replaces it.
- **No change to the merge's problem-union rule** and no message filtering anywhere. The fix
  is at the source.
- **No CLI data for archived changes.** `openspec instructions apply` cannot address an
  archived change, and archived progress and artifacts stay file-read.
- **No caching of `schema.yaml` contents across cycles.** Only the directory is remembered.
  The file is re-read each cycle, so editing a user-tier schema is still seen.
- Checked against the PRD non-goals: nothing here edits OpenSpec files, orchestrates across
  changes, authors changes, or touches Windows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `schema-cli-fallback`: the `schema which` tier's result is a **location** held for the
  worker's lifetime and shared by both producers. Successes are cached across cycles,
  failures for one cycle, and a stale location is evicted and asked for again. This
  **extends** "resolved at most once per name per call", which still holds for one
  `from_cli` call. The parser-problem paragraph of "A not-vendored schema is repaired" is
  amended, because the file producer now reads a located schema too.
- `change-artifacts`: a change's schema is resolved through supplied remembered locations
  when the repository does not vendor it, so an archived or file-sourced change of a
  CLI-located schema has its artifact list.
- `refresh-worker`: step 1 builds with the remembered locations. Step 2 runs the locate pass
  and rebuilds the file set when a location was learned, before merging. The idle re-check's
  member reads use the same locations.
- `change-merge`: the accepted-consequence paragraph of "Merged problems are kept in full,
  never dropped" is rewritten. A CLI-located schema no longer yields a file-side "not
  vendored" message, so nothing stale survives the merge. The rule itself is unchanged. Join
  rule 2's "repaired case" wording is narrowed, because that rule is now rarely reached.

## Impact

- **Unplanned work.** `IMPLEMENTATION-ORDER.md` gains a row. The roadmap planned
  `schema-cli-fallback` as a `from_cli`-only repair, before archived changes were browsable
  in full (`list-sections`) and before `degraded-states` drew per-change problems as rows.
  Together those made the gap visible.
- `src/changes.rs`: `CliCache` holds the locations; `from_files_with_listing`,
  `from_files_owned` and `overlay_family` take them read-only; a new locate function; and
  `resolve_cli_schema` consults and fills them. `from_files(repo, scope)` keeps its signature
  and passes no locations, so file mode and every existing caller are unchanged.
- `src/refresh.rs`: `worker_body`'s two steps and its idle re-check. The locate pass also
  drops the per-change `CliCache` entries of a newly located schema, so a change cached
  while its lookup failed does not keep a stale failure.
- Tests: the merge test pinning the stale message stays unchanged, because it is a property
  of the merge over synthetic strings. Every existing spawn-count assertion still holds,
  since misses never outlive one `from_cli_cached` call. New tests pin the per-worker
  counts.
- `SPEC.md` (Schema paragraph, degraded-states row), `tests/degraded-coverage.toml` (its
  `covers` line ranges in `src/changes.rs` will move), and `IMPLEMENTATION-ORDER.md`.
- No manifest, config-format, or keybinding change. Nothing is **BREAKING**.
