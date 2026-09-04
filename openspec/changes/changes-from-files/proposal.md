## Why

Everything downstream of Phase 2 renders a *change*, and nothing in the crate produces
one. `resolve` finds the repository, `schema` says which artifacts a change has and which
one holds its tasks, and `tasks` counts checkboxes in a file it is handed — but no code
walks `openspec/changes/`, decides what a change is, or joins those three answers into a
value the list view can paint. `tui-shell` depends on this change rather than on the CLI,
which is the roadmap edge that keeps "never fail closed" honest: the dashboard has to be
useful with no `openspec` binary present at all.

This is the Phase 2 `changes-from-files` row of `openspec/IMPLEMENTATION-ORDER.md`.

## What Changes

- A new `changes` module producing `ChangeSet { active, archived, problems }` from one
  repository root and the configured `archived_count`. Pure transformation plus filesystem
  reads: no terminal, no subprocess, no writes.
- Active changes are the *directories* directly under `openspec/changes/` other than the
  one named exactly `archive`, with no marker file required — reproducing
  `openspec list --json`'s rule verbatim, including its inclusion of dot-directories and
  its exclusion of symlinked directories, because that command is what `changes-from-cli`
  parses and the two lists must hold the same names.
- Archived changes come from `openspec/changes/archive/`, splitting the CLI's own
  `^\d{4}-\d{2}-\d{2}-` prefix off the directory name. Dated entries sort newest first;
  an entry with no prefix keeps its whole name and sorts after every dated one, following
  `resolve::nvm_candidates`' existing rule for an unparseable version directory. The first
  `archived_count` survive.
- Artifact paths resolve from the schema artifact's `generates` value, not from its id —
  `<id>.md` / `<id>/` is what `generates` reduces to for the vendored `tdd` schema, and it
  is wrong for any schema whose artifact id differs from its filename. A non-glob value is
  one file; a glob resolves through a deliberately small supported subset, with anything
  outside it recorded as a problem and an empty file list rather than a guess.
- Task progress reproduces the CLI's `getTaskProgressDetailForChange`, including the part
  that is easy to miss: when the tasks artifact resolves to **no** files — a glob that
  matched nothing, a schema that failed to load, a schema naming no tasks artifact — the
  CLI falls back to the single path `<change dir>/tasks.md` and still reports a count.
- `Change` carries no derived state: no `status`, no ratio, no formatted string, no
  provenance discriminant, and no `lastModified`. The CLI derives `status` from the
  completed/total pair, and `Progress` already answers it.
- `SPEC.md` is corrected in four sections the implementation proves wrong or unstated: the
  artifact-path rule and the shape of `contextFiles`; the ordering of both change lists and
  the fields `Change` deliberately does not carry; two new degraded-state rows plus one row
  whose parenthetical defers a decision this change has to make; and the Fixtures section,
  which promises checked-in fixture repositories that no change in the roadmap can use. The
  two roadmap rows are corrected to match.
- No new dependency. The glob subset is hand-written rather than pulling `glob` or
  `globset` — argued in design.md → Decisions.

Nothing here is **BREAKING**: the plugin manifest, the `config.toml` format, and the
keybindings are untouched, and the crate has no external consumer.

## Non-Goals

- **No CLI, no subprocess, and no deferred hook.** `openspec list --json`,
  `openspec status`, `contextFiles`, and the merge policy are all `changes-from-cli`'s, in
  Phase 3. Unlike `resolve::openspec_bin`, whose fourth step sits *inside* one ordered
  chain and therefore needed an injected binding, the two producers of `Change` are two
  whole functions; the seam between them is the function boundary, and putting a deferred
  CLI hook inside `from_files` would be the divided reader the dual-source model rejects.
- **No rendering.** No `ui` code, no `TestBackend`, no 60/120-column tests. The list view,
  the archived separator, filtering, and the tabs are Phase 4's.
- **No writing.** Nothing under `openspec/` is created, modified, or touched, including
  during tests.
- **No agent attribution and no launching.** `Change` carries no agent field; Phase 5 maps
  agents onto changes from outside the type.
- **No watching or caching between calls.** `live-refresh` owns invalidation.
- No orchestration across changes, no change authoring, no Windows support.

## Capabilities

### New Capabilities

- `change-model`: the `Change` and `ChangeSet` values every consumer of this plugin reads,
  and the rule that keeps their two producers — `from_files` here, `from_cli` in Phase 3 —
  from diverging.
- `change-enumeration`: which directories under `openspec/changes/` are changes, which
  under `archive/` are archived changes, how the date prefix is split off, how each list is
  ordered, and how `archived_count` truncates the second one.
- `change-artifacts`: resolving one change's schema, turning each schema artifact's
  `generates` value into concrete file paths, and producing the change's task progress by
  the CLI's own rule.

### Modified Capabilities

None. `plugin-config` already specifies `archived_count` and this change only consumes it;
`schema-selection` and `schema-artifacts` are used exactly as published; `task-checkboxes`'
`Progress` is used unchanged; and `plugin-build`'s argued dependency set is unaffected
because no dependency is added.

## Impact

- **Code:** new `src/changes.rs`; one `pub mod changes;` line in `src/lib.rs`. No other
  module changes. `Cargo.toml`, `Cargo.lock`, `Makefile`, `herdr-plugin.toml`, and
  `.github/workflows/ci.yml` are untouched.
- **Tests:** unit tests inside `src/changes.rs`, building real repository trees under
  `crate::testutil::ScratchDir` — the only mechanism that reaches an empty directory, a
  symlink, and an unreadable directory, none of which git can store. `testutil::snapshot`
  proves the trees are byte-identical afterwards.
- **Docs:** `SPEC.md` → Data layer (Artifact files, Changes, Archived changes), → Degraded
  states (two rows added, one corrected), → Fixtures (rewritten to describe the two fixture
  mechanisms the crate actually uses, since neither a file-reading change nor a no-I/O view
  change can use a checked-in fixture repository); `openspec/IMPLEMENTATION-ORDER.md` → the
  Phase 2 `changes-from-files` row and the Phase 3 `changes-from-cli` row it hands
  obligations to; `AGENTS.md` → Current repo state.
- **Downstream:** `changes-from-cli` produces the same type and inherits two rules recorded
  here — re-sort the active list by name, and join the two artifact lists by position rather
  than by path or by id; `tui-shell`, `list-view`, and `detail-view` render
  `ChangeSet`; `tasks-tab` reads the tasks artifact's resolved paths; `live-refresh`
  re-invokes `from_files` and maps a touched path back to a change through `Change::dir`;
  `agent-attribution` joins on `Change::name`; `degraded-states` surfaces every `problems`
  list. No sibling repository, deployment manifest, or external service is affected.
