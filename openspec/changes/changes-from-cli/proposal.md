## Why

The pane paints from disk today. `SPEC.md` → Dual-source model promises a second
producer that arrives a few hundred milliseconds later and *corrects* it: schema-correct
artifact paths for a schema the repository does not vendor, and the CLI's own task counts.
`subprocess-seam` landed the traits; nothing crosses them yet. This is the Phase 3
`changes-from-cli` row of `openspec/IMPLEMENTATION-ORDER.md`, and `live-refresh` (Phase 4)
depends on it.

## What Changes

- **`changes::from_cli(&dyn OpenspecCli, repo)`** — parse `openspec list --json` and, per
  change, `openspec instructions apply --change <n> --json` into the *same* `Change` type
  `from_files` produces. Progress comes from `list --json`'s `completedTasks`/`totalTasks`,
  not from the apply payload's `progress`, which is a different computation (see design).
- **Re-sort the CLI's active list by name in byte order.** `list --json` defaults to
  most-recently-modified first, and its own `--sort name` is `localeCompare`, not byte
  order, so the flag cannot be used.
- **`changes::merge(files, cli)`** — layer CLI results over file results, joining each
  change's artifact lists by **position** in the schema's declared order, never by path
  (the CLI realpaths its `contextFiles`; the file producer does not) and never by id
  (`schema-artifacts` forbids de-duplicating ids). `archived` passes through untouched.
- **The schema CLI-fallback tier** — when `schema::load` reports `NotVendored`, ask
  `openspec schema which <name> --json` for the schema's **directory** and read
  `schema.yaml` from there through the existing `schema::load_dir`.
- **Degrade per change and as a whole:** absent CLI, a schema the CLI rejects, malformed
  JSON, a CLI whose reported repository root is not ours — each falls back to the file
  result and records a problem.
- **Roadmap correction:** the row names `openspec status --change <n> --json` as a third
  command. It is dropped — it supplies nothing `instructions apply` does not, and doubles
  the per-change Node startup the PRD names as a risk. Argued in design.md.
- **Dependency:** add `serde_json` (`default-features = false`, `features = ["std"]`), the
  parser `SPEC.md`'s stack already names.

Nothing spawns outside `cli`. Not **BREAKING**: no manifest, config-format, or keybinding
change.

## Non-Goals

- No writing anywhere under `openspec/` — PRD non-goal, and a task proves the tree is
  byte-identical.
- No worker thread, no debounce, no caching across calls — `live-refresh` owns those.
- No change to `subprocess-seam`'s `CliError` (its `Failed` carries stderr only; the CLI
  writes its error JSON to stdout, so the reason is unavailable — accepted, argued in
  design).
- No new view, no rendering, no orchestration, no change authoring, no Windows.

## Capabilities

### New Capabilities
- `cli-changes`: the CLI producer — which commands run, what is parsed from each, the
  byte-order re-sort, where progress comes from, the repository-root guard, and how each
  failure degrades.
- `change-merge`: layering CLI results over file results, including the positional
  artifact join and the untouched `archived` list.
- `schema-cli-fallback`: repairing a `NotVendored` schema through
  `openspec schema which <name> --json`.

### Modified Capabilities
- `plugin-build`: the argued dependency set gains `serde_json`; the enumerated normal
  build graph gains `serde_json`, `itoa`, `memchr`, and `zmij`.

## Impact

`src/changes.rs` (new public functions and their parsers), `Cargo.toml`, `Cargo.lock`.
`src/cli.rs`, `src/schema.rs`, `src/resolve.rs`, `src/tasks.rs` are consumed unchanged.
`SPEC.md` gains eight corrections found against the CLI's own source (listed in
planning-review.md); `openspec/IMPLEMENTATION-ORDER.md`'s Phase 3 row loses the third
command. No manifest, no config format, no keys, no external service.
