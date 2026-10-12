## Why

After a Herdr server restart (a reboot, `herdr server stop`, an update without `--handoff`)
every OpenSpec dashboard pane comes back as a bare shell: Herdr restores the layout, the
pane id, the `OpenSpec` label, and the saved directory, but not the process, and it does not
re-run the `launch_argv` it stored for the pane (measured on Herdr 0.9.1 —
`notes/measurements.md` M2). The user must close each dead shell and reopen the dashboard by
hand, in every workspace. Worse, the "OpenSpec: dashboard" action then *focuses* that dead
shell instead of opening a dashboard, because `open` recognises a dashboard by its label
alone (M4). Coding agents come back on their own after a restart; the dashboard beside them
should too.

This is unplanned work: `openspec/IMPLEMENTATION-ORDER.md` predates Herdr's `[[startup]]`
plugin hook (added in Herdr 0.7.5) and the measurements above, and no row anticipates
session restore.

## What Changes

- **A `restore` subcommand**, run by Herdr as a `[[startup]]` hook once after it restores a
  session. It lists every pane in the session and, for each **stale dashboard pane** — one
  labelled `OpenSpec` whose only foreground process is the pane's own shell — opens a fresh
  dashboard beside it and closes the stale shell, so the new dashboard takes the old pane's
  exact slot in its tab (M3). A pane whose foreground process is anything else — a live
  dashboard after a live handoff, or a command the user started — is left alone.
- **`open`/`open-tab` replace a stale dashboard instead of focusing it.** When the matching
  pane in the workspace is stale, the action replaces it in place (the same operation as
  `restore`) and focuses the replacement, rather than focusing a dead shell.
- **BREAKING (plugin manifest):** `herdr-plugin.toml` gains one `[[startup]]` entry,
  `command = ["./target/release/herdr-openspec", "restore"]`, and `min_herdr_version` rises
  from `0.7.0` to `0.7.5`, the release that introduced `[[startup]]`. Herdr's install preview
  lists the startup command, so users see it before it runs.
- `SPEC.md`, `README.md`, and the subcommand usage text name the fourth subcommand and the
  restore behaviour.

## Non-Goals

- **Restoring dashboard state.** The replacement starts fresh: no selection, open tab, fold
  state, or scroll position carries over. Nothing new is written to `HERDR_PLUGIN_STATE_DIR`;
  the plugin's writes stay exactly `agent-names.toml` and `settings.toml`.
- **Restoring anything but dashboard panes.** Agents restore through Herdr's own native
  session restore; other plugins' panes and ordinary shells are untouched.
- **Restoring a dashboard the user closed.** A closed pane is not in the saved layout, so
  there is nothing to restore.
- **Pretending to be an agent.** Herdr's agent resume-command route (Herdr 0.10.0+) would
  need the binary on `PATH` and would surface the dashboard as an agent in `herdr agent
  list` — and in this plugin's own agent badges. Rejected.
- **A supervised or retrying daemon.** `restore` is one-shot, as Herdr documents startup
  hooks to be.
- Nothing here edits OpenSpec files, orchestrates changes, authors changes, or adds Windows
  support (PRD non-goals).

## Capabilities

### New Capabilities

- `dashboard-restore`: the `restore` subcommand — how a stale dashboard pane is recognised
  from `herdr pane list` and `herdr pane process-info`, how it is replaced in its own slot,
  what is left alone, and how failures are reported without stopping the sweep.

### Modified Capabilities

- `pane-open`: the subcommand set grows from three to four (`restore`), and "An
  already-open dashboard is focused, never duplicated" changes — a matching pane that is
  stale is replaced in place and the replacement focused, rather than the stale pane
  focused.
- `plugin-manifest`: the manifest declares exactly one `[[startup]]` entry, and
  `min_herdr_version` is `0.7.5`.

## Impact

- **Code:** `src/open.rs` (the stale-pane classifier, the replace operation, the `restore`
  entry point; it stays the crate's one-shot `HerdrCli` consumer and starts no thread),
  `src/main.rs` (subcommand dispatch and usage text). No change under `src/ui/`, and no new
  `HerdrCli` consumer file, so the `LAUNCHSEAM` five-file list does not move.
- **Manifest:** `herdr-plugin.toml` (`[[startup]]`, `min_herdr_version`).
- **Contract tier:** `tests/manifest.rs` (manifest transcription), `tests/doc_contract.rs`
  where it transcribes the manifest or the subcommand list.
- **Docs:** `SPEC.md` → Herdr integration (Manifest, subcommands, restore), `README.md`
  (subcommands, minimum Herdr version, restore behaviour), `AGENTS.md`/`CLAUDE.md` current
  repo state.
- **External:** Herdr's `herdr pane list`, `herdr pane process-info`, `herdr plugin pane
  open`, and `herdr pane close` CLI surfaces, all reached through the existing `HerdrCli`
  seam. Users on Herdr 0.7.0–0.7.4 must upgrade.
