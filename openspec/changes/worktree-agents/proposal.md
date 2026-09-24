## Why

`worktree-changes` shows a change from the worktree an agent is working in, but the agent itself
stays invisible: attribution admits an agent only when its working directory lies under the
pane's own repository root, and a linked worktree — Herdr places one at
`<repo-parent>/.worktrees/<repo>-<branch>` — lies outside it. So a row can show a worktree's live
progress with no badge beside it, the footer's unattributed count omits the agent, and `g` has
nothing to focus. The other half is launching: `a`, `c`, and `s` always split a pane at the
pane's own root, so launching onto a row that shows a worktree's copy starts an agent in the
wrong checkout, on the wrong branch. `SPEC.md` records the first half as row 30 of the
degraded-states table, "a standing, accepted limitation".

## What Changes

- An agent is **in scope** when its working directory lies under the pane's repository root
  **or under the OpenSpec root of any worktree** in `ChangeSet::worktrees`. Every attribution
  tier — the recorded mapping, the name match, and the unattributed count — applies unchanged
  to the widened scope; nothing new is guessed.
- `agents::attribute` gains one parameter, the worktree roots; `Dashboard::attribution` passes
  them from the change set it already holds, so the scope follows the family per frame.
- `a`, `c`, and `s` on a row showing a worktree's copy split the new pane with `--cwd` at that
  worktree's OpenSpec root instead of the pane's own. `launch::Request::Launch` gains the root it
  runs in, filled by the dashboard from the selected change's `dir`; `launch::decide` is unchanged.
- `g` is unchanged: it focuses the pane of whichever agent the badge shows, wherever it runs.
- `SPEC.md` row 30 changes from "invisible, not pending any future change" to "in scope whenever
  the worktree is a member of the family", keeping its condition text.

Not **BREAKING**: no manifest, config, or keybinding change. A pane whose repository has no
worktree family behaves byte-identically.

## Non-Goals

- **No branch-name attribution** (PRD → Future #4): an agent in a worktree is still badged only
  by the plugin's recorded mapping or an exact name match, never because its branch is named like
  a change.
- **No new agent evidence of any kind.** A worktree agent no tier can place is counted, exactly
  as an unplaceable agent in the main checkout is.
- **No launching into a worktree the plugin creates.** `a`/`c`/`s` never run
  `herdr worktree create`; they launch where the selected row's copy already lives.
- **No change to the derived agent name**, so an agent already live for a change in one checkout
  still refuses a second launch for it in another, as today.
- No editing of OpenSpec files, no orchestration across changes, no change authoring, no Windows
  support.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-attribution`: the pure function's signature gains the worktree roots, and the
  repository-scope requirement admits an agent under any of them.
- `agent-launch`: call 1's `--cwd` is the root of the checkout the selected change's copy lives
  in, and a new requirement adds that root to `Request::Launch` and says who fills it.

## Impact

- `src/agents.rs` (`attribute`), `src/ui/app.rs` (`Dashboard::attribution`, the launch decision's
  `Go` branch), `src/launch.rs` (`Request::Launch`, `split_args`, the worker), `src/ui/driver.rs`
  (the `Request::Launch` patterns that name its fields).
- `SPEC.md` (attribution scope paragraph, the linked-worktree paragraph, row 30's description,
  the launch flow's `--cwd`), `AGENTS.md` (the attribution rule's "every tier is scoped to the
  resolved repository"), and `tests/degraded-coverage.toml` (row 30's proof).
- No new module, dependency, thread, gate, or process spawn.
- **Depends on `worktree-changes`**, which provides `ChangeSet::worktrees`; this change cannot be
  applied before it.
- **Roadmap:** unplanned work, the second half of the worktree gap `worktree-changes` opens; it
  gains a row beside that one when it archives.
