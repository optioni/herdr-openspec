## Why

The pane cannot see the agents working beside it. Phase 5's first row opens that channel:
poll `herdr agent list` at roughly one second and parse the result, so `agent-attribution`
has live agents to map onto changes and `agent-launch` has a socket it knows is reachable.
An unreachable socket is a **supported state, not an error** — the plugin runs as a
standalone TUI with agent features simply absent.

This is `openspec/IMPLEMENTATION-ORDER.md` → Phase 5's `agent-polling` row, depending on
`subprocess-seam` and `tui-shell`, both archived.

## What Changes

- A new top-level `src/agents.rs` — outside `src/ui/`, exactly as `src/watch.rs` and
  `src/refresh.rs` are — holding the `Agent` value, the parse of `herdr agent list`'s
  payload, the non-blocking `AgentPoll` seam, and the crate's **second** worker thread.
- `herdr agent list` is spawned through the existing `cli::HerdrCli` trait and nowhere else,
  with exactly the argument vector `["agent", "list"]` — **no `--json` flag**, which Herdr 0.8.2
  rejects with exit status 2.
- `ui::driver::Live` gains a **third field**, not a seventh `run_loop` parameter, and the
  loop's wake-up becomes the soonest of the watcher's and the poller's own deadlines.
- `Dashboard` gains an `agents` field carrying the latest poll's outcome. **Nothing renders
  it in this change**: every landed buffer assertion stays byte-identical.
- `ui::run` is split so its wiring is driven by a test rather than only read by a reviewer —
  the defect class `live-refresh` shipped and its Change Review caught.
- `SPEC.md` is corrected against Herdr 0.8.2's real output (see Impact).

## Non-Goals

- **No attribution.** Mapping an agent to a change, the badge, and the footer count are
  `agent-attribution`.
- **No launching or focusing.** `a` / `c` / `s` / `g` are `agent-launch`; `Action` gains no
  variant here and no keybinding changes, so this change is **not BREAKING**.
- **No rendering.** No view reads the new state yet.
- **No event hook.** Polling is deliberate; the confirmed plugin event names carry no
  agent-status event.
- PRD non-goals hold: nothing is written to OpenSpec files, no change is authored, no batch
  is orchestrated, and no Windows path is added.

## Capabilities

### New Capabilities
- `agent-list`: spawning `herdr agent list` through the seam and parsing its envelope into
  agent values, including every absent, unknown, and unusable case.
- `agent-poller`: the non-blocking `AgentPoll` seam, the ~1s cadence, the second worker
  thread, the unreachable-socket standing state, and the composition root that wires it.

### Modified Capabilities
- `dashboard-loop`: `Dashboard` gains a tenth field; `Live` gains a third; the non-blocking
  source check gains a third seam module.
- `live-updates`: the loop's per-iteration order gains the poller's step, and the live
  tier's "reaches no Herdr socket" clause is narrowed to the three files it names.
- `watch-invalidation`: `watch::soonest` joins `poll_timeout` so one tick serves two pollers.
- `subprocess-seam`: one binding names the real `herdr` program, as `npm_prefix` does.

## Impact

- **Code**: new `src/agents.rs`; `src/cli.rs` (one binding); `src/watch.rs` (`soonest`);
  `src/ui/driver.rs` (`Live`, the loop's fourth live step); `src/ui/app.rs` (`Dashboard`);
  `src/ui/mod.rs` (`run` split into a testable composition root); `src/lib.rs` (module
  declaration and two test doubles).
- **No new dependency.** `serde_json` already ships.
- **`SPEC.md` corrections**, all confirmed against Herdr 0.8.2 running live: `herdr agent
  list` returns a `{"id":…,"result":{"agents":[…],"type":"agent_list"}}` **envelope**, not a
  bare array; it carries a `name` field the spec omits — the field attribution tier 2 needs,
  while `agent` is the agent *kind*; only seven fields are required and the rest may be
  absent; an unreachable socket exits **1** with a JSON error on **stderr**, so the
  degraded-states row claiming a CLI's reason is unavailable is true of `openspec` and false
  of `herdr`; and the crate gains a **second** clock binding, which `SPEC.md` and `AGENTS.md`
  both currently call the only one.
