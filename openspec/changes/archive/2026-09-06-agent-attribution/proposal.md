## Why

`agent-polling` opened the channel: the pane now knows which agents are alive in the Herdr
session, and nothing reads that knowledge. A reader looking at the change list still cannot
tell which change an agent beside them is working on.

This is `openspec/IMPLEMENTATION-ORDER.md` → Phase 5's `agent-attribution` row, depending on
`agent-polling` and `changes-from-files`, both archived.

Attribution is the one piece of this plugin most likely to guess wrong, so its whole design is
a refusal to guess: three tiers, and an agent that satisfies none of them is **counted, never
assigned**. A live measurement forces one correction on `SPEC.md` before a line is written —
`herdr agent list` is **session-global**, returning byte-identical output from three different
working directories and listing agents in repositories other than the current one. Every tier
must therefore be scoped to this repository, which `SPEC.md` states for tier 3 alone.

## What Changes

- `agents::attribute` — a pure, total function from live agents, the repository root, the
  change names, and the plugin-local agent-name mapping to per-change badges and one
  unattributed count. Tier 2 matches on an agent's **`name`**, never on `agent`, which is the
  agent *kind* (`"claude"` on every live agent measured).
- Every tier is scoped to the repository: an agent whose `cwd` is not inside the resolved
  repository root is neither badged nor counted, and an agent carrying no `cwd` cannot be
  proven to be in the repository and is treated the same way.
- The change row gains a one-column **agent badge** between the name field and the progress
  cell, dropped whole **before** the progress cell as the width falls, so every landed drop
  boundary stays where it is. A row with no attributed agent carries no badge cell and renders
  byte-identically to today.
- The footer gains a trailing `<n> unattributed` hint, present only when the count is non-zero.
- `Dashboard` gains an eleventh field, `agent_names`, holding the mapping read from
  `HERDR_PLUGIN_STATE_DIR`; `Startup` gains a fourth field, `state_dir`, so the read stays
  injected and a test drives it. `Dashboard::attribution()` derives the badges per frame,
  keyed by change **name** — never stored, never attached by index.
- `SPEC.md` is corrected against the measured CLI (see Impact).
- No key, no manifest entry, and no configuration key changes. **Not BREAKING.**

## Non-Goals

- **No launching or focusing.** `a` / `c` / `s` / `g`, `herdr pane split`, and writing the
  mapping are `agent-launch`; `Action` gains no variant.
- **No second Herdr call.** `herdr worktree list` is not consulted, so an agent working in a
  linked worktree of this repository is out of scope — see design.md → Decisions 4.
- **No guessing from a terminal title.** A title is a summary, not a change id.
- **No change to `Change`.** Attribution is derived beside the change list, never stored on it,
  so `from_files` and `from_cli` need no new agreement.
- **No new problem row.** An unreachable socket stays the silent standalone-TUI state.
- PRD non-goals hold: nothing is written to OpenSpec files, no change is authored, no batch is
  orchestrated, and no Windows path is added.

## Capabilities

### New Capabilities
- `agent-attribution`: the three tiers and their precedence, the repository scope every tier
  inherits, the deliberate non-attribution case reported as a count, and the badge status
  chosen when several agents share one change.

### Modified Capabilities
- `change-rows`: the row grammar gains the badge cell and its position in the drop order, on
  both the active and the archived row.
- `responsive-layout`: the footer's hint list gains a conditional trailing count.
- `dashboard-loop`: `Dashboard` gains an eleventh field and the derived `attribution()`.
- `agent-poller`: `Startup` gains `state_dir`, the composition root reads the mapping, the
  outer-loop wiring test now asserts a rendered badge and a rendered count, and the
  "nothing renders it" requirement — true of exactly one change — names what reads the snapshot
  now and through what.
- `list-filtering`: the footer's hint list, which that capability states exhaustively, gains the
  count as its trailing entry.

## Impact

- **Code**: `src/agents.rs` (`Attribution`, `attribute`); `src/ui/app.rs` (`Dashboard`'s
  eleventh field, `attribution()`); `src/ui/list.rs` (the badge cell); `src/ui/view.rs` (the
  footer hint); `src/ui/mod.rs` (`Startup::state_dir`, `load`'s third parameter); `src/lib.rs`
  (test doubles). No new module, no new file under `src/ui/`, no new thread.
- **No new dependency.** `state::read` and `serde_json` already ship.
- **`SPEC.md` corrections**, measured against Herdr 0.8.2 running live: `herdr agent list` is
  **session-global** — verified byte-identical from three working directories, listing agents
  in a different repository — so every attribution tier is scoped to the repository, not tier 3
  alone; `cwd` is **not** one of Herdr's seven required fields and is absent from an entry that
  has none; the List view's third column now has a defined grammar and a defined drop position;
  and Herdr places linked worktrees **outside** the repository root, which the design records
  as a known limitation rather than silently mis-counting.
- **A landed doc defect is corrected**: `src/ui/mod.rs`'s `Startup` doc comment still says
  seven parameters are "clippy's `too_many_arguments` threshold exactly". Measured on this
  crate and toolchain, the lint fires at **eight** — a seven-argument function is silent and an
  eight-argument one errors `too many arguments (8/7)`. `agent-polling` retired the claim in its
  design → Boundaries and repeated it verbatim in its own design → Decisions 8, so the false
  sentence survives in two places; this change deletes the shipped copy, which is the one a
  future author reads.
