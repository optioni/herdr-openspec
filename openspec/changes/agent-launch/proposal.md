## Why

The pane can read every change and can say which agent is on which change. It still cannot
*start* one. A reader who has decided what to work on leaves the dashboard, opens a pane,
changes directory, launches an agent, and types the `/opsx:*` command by hand — the one step
`PRD.md` → Goal 4 promises in a single keystroke, and the one claim `README.md`'s Keys table
has been making since Phase 1 without being true.

This is `openspec/IMPLEMENTATION-ORDER.md` → Phase 5's `agent-launch` row, the last of that
phase, depending on `agent-attribution`, `detail-view`, and `plugin-config` — all archived.

It is also the **first change in which a keypress causes an outward, side-effecting action**.
Every landed `Action` is read-only. The design's whole burden is keeping "views are pure" and
"the plugin never writes inside `openspec/`" true while a key starts a process that will do
neither.

## What Changes

- **BREAKING — four new keybindings.** (`herdr-plugin.toml` is untouched and
  `min_herdr_version` stays `0.7.0`; see Impact for why that is a deliberate, recorded choice
  rather than an oversight.) `a` / `c` / `s` launch an agent onto the selected change
  with `/opsx:apply`, `/opsx:continue`, and `/opsx:archive`; `g` focuses the agent already
  attributed to it. `Action` goes from **thirteen** variants to **seventeen** — not from nine,
  as `HANDOFF.md` → constraint 8 assumed.
- A new `launch` module, outside `src/ui/`, holding the whole policy as pure functions plus the
  crate's **third** worker thread. Three sequential Herdr calls, the second and third depending
  on the first's output: `pane split --cwd <repo> --direction right --no-focus` → `agent start
  <derived name> --kind <kind> --pane <pane id from call 1>` → `agent prompt <derived name>
  "/opsx:<command> <change>"`. `g` is a fourth, `agent focus <pane id>`. All four go through
  `cli::HerdrCli`; nothing spawns outside `src/cli.rs`.
- The agent name is `state::agent_name`'s derivation, capped at 32 characters and matching
  `[a-z][a-z0-9_-]{0,31}` — Herdr 0.8.2 rejects anything else with `invalid_agent_name` before
  it looks at the pane. `state::record` writes the derived-name → change mapping under
  `HERDR_PLUGIN_STATE_DIR` whenever the two differ, and the successful outcome updates
  `Dashboard::agent_names` in memory so attribution finds the new agent on the next frame.
- `Dashboard` gains a twelfth field, `launch`, carrying the one-shot request `apply` produced
  and the last outcome's failure. `Live` gains a fourth field, the launcher.
  `Attribution` gains a third field, `panes`, so `g` has a target.
- The footer gains `a/c/s launch` and `g focus`, present only when `Dashboard.agents.reachable`
  — the standing condition lives there, never on `ChangeSet::problems`.
- A failed launch renders as a leading `!`-marked list row naming the reason Herdr gave.
- `SPEC.md` is corrected against Herdr 0.8.2 measured live (see Impact); `README.md`'s Keys
  table and `SPEC.md` → Keys are updated together.

## Non-Goals

- **No orchestration.** One key, one agent, one change. No launch-all, no queue, no dependency
  ordering, no "next change" — `PRD.md` → Non-goals reserves that for the orchestrator agents.
- **No authoring.** Only `/opsx:apply`, `/opsx:continue`, and `/opsx:archive` are bound: the
  three commands that act on a change that already exists. `/opsx:new` and `/opsx:propose` are
  deliberately unbound.
- **No writing inside `openspec/`.** The plugin writes exactly one file, `agent-names.toml`,
  under its own state directory. The agent it starts will write inside `openspec/`; the plugin
  does not, and the boundary is the process it starts.
- **The plugin never closes a pane it created.** A failed `agent start` leaves the split pane in
  place and names its id, rather than killing a process that may be mid-start.
- **No new key to dismiss the launch problem**, no confirmation prompt, no launch menu, no
  per-agent listing, no `herdr worktree list`, no manifest change, no new dependency.
- **No Windows path**, no editing of OpenSpec files, no change to `Change`, `ChangeSet`,
  `from_files`, or `from_cli`.

## Capabilities

### New Capabilities
- `agent-launch`: the three-call launch flow and its argument vectors, the derived agent name
  and its recording, the refusals that reach Herdr not at all, the per-call failure paths, and
  `g`'s focus target across the attribution tiers.

### Modified Capabilities
- `dashboard-loop`: `Action` reaches seventeen variants, `action_for` maps four more keys,
  `Dashboard` gains a twelfth field, and the loop hands the pending request to a collaborator
  and drains its outcome.
- `agent-attribution`: `Attribution` gains `panes`, the pane of the agent whose status won the
  precedence fold, so the badge and `g`'s target are the same agent by construction.
- `agent-poller`: `AgentSnapshot::reachable` is read for the first time — by the footer and by
  the launch decision — and `Live`, `Collaborators`, and `start_collaborators` each grow by one.
- `responsive-layout`: the footer's hint list gains two conditional action hints ahead of the
  unattributed count, which therefore no longer fits at 60 columns when the socket is reachable.
- `list-filtering`: the same hint list, which that capability states exhaustively.
- `change-rows`: the row emission order gains launch problems ahead of refresh problems.
- `live-updates`: the loop gains a dispatch step and an outcome step, the leading-problem-row
  requirement gains a third source, and the live tier gains its first write — confined to the
  state directory.
- `tasks-checklist`: the exact `Action` variant count moves from thirteen to seventeen, and the
  read-only requirement states where the plugin's writes end and a launched agent's begin.
- `subprocess-seam`: the Herdr handle becomes reachable from four files rather than three.

## Impact

- **Code**: `src/launch.rs` (new — policy, argv, parsing, the worker); `src/ui/app.rs`
  (`Action`'s four variants, `Dashboard::launch`, the four `apply` arms); `src/ui/driver.rs`
  (`Live`'s fourth field, two loop steps); `src/ui/mod.rs` (`start_collaborators`'s fourth
  parameter); `src/ui/list.rs` (the launch problem rows); `src/ui/view.rs` (the footer hints);
  `src/agents.rs` (`Attribution::panes`); `src/lib.rs` (test doubles). No new file under
  `src/ui/`, no new dependency, no manifest or config-format change.
- **The manifest floor is left at `0.7.0` deliberately.** Every subcommand used here predates
  0.8.2, but two *shapes* this change depends on were measured on 0.8.2 and were previously
  believed otherwise: `pane split` requires `--direction`, and it returns an envelope rather
  than the bare id `SPEC.md`'s flow diagram showed. A Herdr that returned a bare id would fail
  every launch at call 1. That is covered as a degraded state — `launch::pane_id` returns `Err`,
  the launch stops before an agent is started, and the reason is rendered — rather than by a
  version bump, which would be a manifest change and therefore a second BREAKING claim on a
  hypothesis nobody has measured.
- **`SPEC.md` corrections, measured live against Herdr 0.8.2** (the two prior Phase 5 changes
  each found `SPEC.md` wrong about this socket, and so does this one): `pane split` **requires**
  `--direction` (exit 2, `usage: herdr pane split …` without it) and returns an **envelope**,
  `{"id":"cli:pane:split","result":{"pane":{…,"pane_id":…},"type":"pane_info"}}`, not a bare id;
  with no pane argument it splits the **focused** pane, which is the dashboard's own when its key
  was pressed; `agent start` and `agent prompt` take the **derived agent name**, not the change
  name, which `SPEC.md` → Attributing an agent already flags as this change's correction to make;
  `agent prompt` takes its text as a **positional** argument and is not given `--wait`;
  `agent focus` accepts a pane id or an agent name but **not** a terminal id; every one of these
  reports failure as a JSON envelope on **stderr** with exit **1** (`invalid_agent_name`,
  `agent_name_taken`, `agent_pane_not_found`, `agent_not_found`), a usage error with exit **2**;
  and Herdr 0.8.2 accepts exactly **22** agent kinds. Six new Degraded-states rows follow.
- **A landed doc defect is corrected**: `src/cli.rs`'s `CliError` doc comment predicts
  "`agent-launch` four more" invocations through one `RealHerdrCli`, and "all eight failures".
  There are **five** — `pane split`, `agent start`, `agent prompt`, `agent focus`, and the
  `agent list` `agent-polling` already drives — and therefore **nine** failures. A second landed
  doc defect goes with it: `src/ui/app.rs`'s `Action` enum still opens "The nine outcomes",
  which has been wrong since `live-refresh` made it thirteen.
- **Inherited, not fixed here, and the inherited description is itself wrong.** `DEPS` and
  `GRAPH-SNAP` have both been red on `main` since `live-refresh`, but for **two different
  reasons**, and only one is what `HANDOFF.md` records. `DEPS` fails on leg 2a because the
  `notify` dependency was never added to its want-list — as recorded. `GRAPH-SNAP` **passes**
  its snapshot diff: `tests/fixtures/build-graph.txt` **was** regenerated (commit `574b87d`,
  and it holds `notify v8.2.0`, `fsevent-sys`, `inotify`, `inotify-sys`). It fails four legs
  later, on a hardcoded macOS/Linux platform-difference assertion that `notify`'s backends
  invalidated: `GRAPH-SNAP FAIL: macOS/Linux differ by [fsevent-sys inotify inotify-sys
  linux-raw-sys ], expected [linux-raw-sys ]`.
  **The decision, stated here rather than deferred:** this change declines the repair.
  `HANDOFF.md` assigns it here ("**Decision: `agent-launch` owns the repair** — regenerate
  `tests/fixtures/build-graph.txt` and author a want-list matching the real dependency set"),
  but that scope is wrong twice over — the snapshot needs no regenerating, and the fix
  `GRAPH-SNAP` actually needs is a widened platform-difference list, which `HANDOFF.md` does not
  mention. This change adds no dependency and edits neither script, so folding in a repair whose
  scope is misstated would bless a gate nobody had re-derived. planning-review.md records the
  exact failure text of both so a reviewer does not read either as a regression, and the repair
  is handed on with its corrected scope.
