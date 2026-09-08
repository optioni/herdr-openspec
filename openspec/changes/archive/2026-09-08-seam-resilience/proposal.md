## Why

The render loop is sound; its four background collaborators are not resilient at the
edges. A post-`degraded-states` audit found eleven defects the user meets as silence: the
dual-source model may be dead on every repository but this one (the `openspec` CLI
resolves its root from the *process* cwd, which is not the workspace cwd the dashboard
resolved from, so `changes::from_cli`'s root guard discards every payload); it may be dead
one layer earlier still on most machines, because `openspec` is an `#!/usr/bin/env node`
shim installed beside the very `node` it needs, so the probe chain reaches steps 3 and 4
exactly when the child's inherited `PATH` cannot exec it — measured here: exit 127, empty
stdout, `env: node: No such file or directory`; a worker
thread that dies is called into forever with nothing shown; a wedged `herdr` socket
freezes the badges while the header still says `reachable`; a symlinked repository path
makes every agent badge vanish; a panic on a worker thread tears the terminal down under
a still-drawing loop; and the startup problem rows that name a missing binary are wiped
by the first watcher error. Each is a "never fail closed" violation that renders
*confidently wrong* content rather than degraded content.

This is **unplanned work**: the roadmap ended at `degraded-states`, which audited
`SPEC.md`'s degraded-states table row by row. These are failure modes no row describes —
worker death, child hang, and process working directory are collaborator *lifecycle*,
which the table never covered.

## What Changes

- **Measure first, then fix (S7).** Task 1 records the pane process's real
  `current_dir()` from a workspace outside this repository, **and** the exit code and stderr
  of a real spawn of the resolved binary — without that second half a 127 from S10 would be
  misread as "S7's fix did not work". The seam then carries the repository root as the
  child's working directory.
- **The resolved binary is spawned so its interpreter resolves (S10).** The composition root
  supplies a one-entry environment overlay prepending the resolved binary's own directory to
  the child's `PATH`. Verified on this machine: without it, exit 127 and
  `env: node: No such file or directory`; with it, exit 0.
- The watch narrows from the repository root to `openspec/` (S1), and an empty selection
  no longer spawns `openspec`.
- The panic hook restores the terminal only on the render thread (S2).
- `Refresher` and `Launcher` report worker death as a problem row, on `AgentPoll`'s
  existing model (S3).
- A poll with no answer for N intervals is surfaced, and refresh requests stop piling up
  behind a hung child (S5).
- Agent containment compares canonicalized paths (S8).
- Startup problems survive the first watcher error (U3).
- A launch in flight refuses a second one (S4), and `ui::run` joins the launcher before
  returning so `q` cannot orphan an unprompted agent (S6).
- The per-frame artifact read (S9) is recorded as a deliberate accepted cost in
  `design.md`, not silently left.

No keybinding, manifest, or config-format change: **not BREAKING**.

## Non-Goals

- No new keys, no manifest entry, no config key, no new dependency.
- No relaxation of the three architecture rules (no spawn outside `cli`, nothing under
  `src/ui/` blocks or reads a clock, views do no I/O).
- Not a rewrite of the launch flow, the attribution tiers, or the debounce.
- No change to `quality-gates`, `ci-workflow`, `degraded-coverage`, `cli-changes`,
  `change-merge`, the schema capabilities, or any pure rendering capability — four
  sibling changes own those.
- Nothing writes inside `openspec/`; no change authoring, no cross-change orchestration,
  no Windows support.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `subprocess-seam`: the seam gains a working directory (S7) and an environment overlay
  (S10) for the `openspec` program, both constructor-supplied and decision-free, plus a
  bounded wait for a child that never exits (S5).
- `watch-invalidation`: the recursive watch is rooted at `openspec/`, not the repository
  root (S1).
- `refresh-worker`: worker death is observable rather than swallowed, an empty
  selection short-circuits (S1, S3, S5), and the composition root supplies both the working
  directory and the `PATH` overlay so the CLI both runs and answers about the right
  repository (S7, S10).
- `agent-poller`: a poll outstanding across several intervals is reported (S5).
- `agent-launch`: `decide` refuses while a launch is in flight (S4), the launcher
  reports worker death (S3), and the process joins the launcher before exit (S6).
- `agent-attribution`: containment compares canonicalized paths (S8).
- `terminal-lifecycle`: the panic hook is a no-op off the render thread (S2).
- `live-updates`: startup problems are held separately from live problems so a watcher
  error cannot erase them (U3), and the per-frame artifact read is stated as accepted
  (S9).

## Impact

Code: `src/cli.rs`, `src/watch.rs`, `src/refresh.rs`, `src/agents.rs`, `src/launch.rs`,
`src/ui/mod.rs`, `src/ui/driver.rs`, `src/ui/app.rs`, `src/ui/terminal.rs`, `src/main.rs`.
`SPEC.md` — the "reports a repository root other than the one this plugin resolved" and
"one recursive watch on `openspec/`" statements both change, and new degraded rows land
for worker death and a hung child. No API, data model, external service, sibling
repository, or deployment manifest is affected.
