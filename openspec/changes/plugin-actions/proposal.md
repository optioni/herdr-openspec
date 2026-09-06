## Why

The dashboard can only be opened by typing a `herdr plugin pane open` command by hand.
`README.md` already advertises two action-menu entries — **OpenSpec: dashboard** and
**OpenSpec: dashboard (tab)** — that the manifest does not declare, and the manifest's
lone `dashboard` pane inherits the plugin root as its working directory, so a
GitHub-installed plugin would open a dashboard that resolves no `openspec/` at all.
This is Phase 6's `plugin-actions` row in `openspec/IMPLEMENTATION-ORDER.md`.

## What Changes

- Two new binary subcommands, `open` and `open-tab`, each **open or focus** the
  dashboard for the workspace the action was invoked from: list panes, focus an
  existing dashboard pane, otherwise open a new one. Neither renders; neither needs a
  terminal.
- A new `src/open.rs` — the crate's fifth `HerdrCli` consumer, reaching the `herdr`
  program only through the trait object, spawning nothing itself.
- `herdr-plugin.toml` gains both `[[actions]]` entries and the `dashboard-tab` pane.
  **BREAKING** — a manifest change.
- The plugin passes `--cwd` from Herdr's injected invocation context, so the pane
  opens on the workspace's repository rather than on the plugin root.
- A `tests/manifest.rs` contract gate, inside `make check`, that ties the manifest,
  the Cargo bin target, and `README.md`'s advertised action titles together.
- `README.md` drops the "these action-menu entries arrive with a later change"
  caveat; `SPEC.md` is corrected against Herdr 0.8.2 measured live.

## Non-Goals

- No toggle-off, no close-on-repeat, no cross-workspace switching.
- No event hooks, no `[[startup]]` hooks, no keybinding installation.
- No change to `min_herdr_version` or `platforms` — `repo-foundation` owns both.
- No new dependency, no second binary target, no dashboard behaviour change.
- Not `spec-purposes`' work: `DEPS` and `GRAPH-SNAP` stay red and stay its.

## Capabilities

### New Capabilities

- `pane-open`: the `open` / `open-tab` subcommands — argument classification, the
  invocation context, the open-or-focus decision, the exact `herdr` argument vectors,
  exit statuses, and every degraded path.

### Modified Capabilities

- `plugin-manifest`: two `[[panes]]` entries and two `[[actions]]` entries replace
  "exactly one `[[panes]]`"; the requirement "Actions and the tab pane are
  deliberately absent" is removed, and the contract check becomes a Rust test.
- `plugin-build`: the binary accepts `ui`, `open`, and `open-tab` rather than `ui`
  alone; exit statuses gain the `open` family's.

## Impact

`src/open.rs` (new), `src/lib.rs` (`parse`, `usage`), `src/main.rs` (dispatch),
`tests/manifest.rs` (new), `tests/cli.rs`, `herdr-plugin.toml`, `README.md`, `SPEC.md`,
`AGENTS.md`.

Gates: `LAUNCHSEAM`'s script body gains an `ENTRY` parameter and a second invocation;
`LAUNCHSEAM` and `AGENTSEAM` take a deliberate `ALLOWED` edit and `READONLY-UI` an `EXTRA`
edit; seven file-count floors are re-measured. The roster stays at 30 — no gate is added.
`DEPS` and `GRAPH-SNAP` are red on `main` before this change starts and stay
`spec-purposes`'. No `openspec/` write outside this change's own directory.
