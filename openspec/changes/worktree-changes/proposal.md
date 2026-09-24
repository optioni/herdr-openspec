## Why

An agent working in a **linked git worktree** of the repository is invisible to the pane. The
dashboard reads exactly one `openspec/` tree — the one under the directory it resolved at
startup — so a change created inside a worktree never appears, and a change created on the main
checkout and then worked on in a worktree sits at its stale main-checkout progress (`0/12`)
while the worktree's own `tasks.md` reads `7/12`. The more an agent is isolated in a worktree —
which is exactly how Herdr's `worktree create` and the superpowers workflow run parallel work —
the less the pane shows about it. `SPEC.md` records the agent half of this as an accepted
limitation (row 30); the change half has never been written down at all.

## What Changes

- The pane discovers the repository's **worktree family** by running
  `git worktree list --porcelain -z` against its own repository root, skipping prunable and
  bare entries, and maps each member to its own OpenSpec root (the member's top level joined to
  the same relative path the pane's own root has inside its top level).
- For each member other than the pane's own root, it derives which change directories that
  member has **touched** since it forked: `git merge-base` against the pane's own `HEAD`, then
  `git diff-tree` from that base to the member's `HEAD`, plus `git status` for uncommitted and
  untracked work — with rename detection, optional locks, and fsmonitor off, so the pane never
  writes `.git/index` and never contends for `index.lock` with an agent committing in that
  worktree. A worktree with unrelated history (an orphan `gh-pages`) simply owns nothing.
- Those touched changes are **overlaid** onto the pane's own change set: a member's copy of an
  active change **replaces** the pane's copy of the same name, or is **added** when the pane has
  none; a change the member **archived** leaves the active list and appears in the archived list;
  a member's archived directory the pane's archive lacks is added. A change nobody touched shows
  the pane's own copy, so a stale or long-lived worktree resurrects nothing and hides nothing.
- Two members touching the **same** change is reported as a problem row naming both; the first
  member in `git worktree list` order — the main checkout, then by path — wins.
- A change shown from a worktree carries a one-column **worktree marker** in its list row, and
  the detail header names the worktree's **branch** (or its short `HEAD` when detached). One pure
  function, `worktrees::member_of`, decides which member's copy a row is by matching the change's
  directory against each member's `openspec/changes`, so a pane opened inside a worktree nested
  under the main checkout never mistakes its own rows for the main checkout's.
- The refresh worker computes the overlay on every cycle and, while idle, **re-checks the
  family every two seconds**, sending a fresh set only when the overlay changed — so a worktree
  created after the pane opened, and an agent ticking tasks inside one, reach the pane without
  watching any path outside `openspec/`.
- A third subprocess trait, `GitCli`, joins `OpenspecCli` and `HerdrCli` in `src/cli.rs`, with
  one binding naming the bare program `git`. A new pure module, `src/worktrees.rs`, parses git's
  output and names no process or filesystem API.
- `SPEC.md`'s degraded-states table gains rows for no `git`, a member whose git query fails, a
  prunable member, two members on one change, and file mode (which starts no worker and so shows
  no worktree changes).

Not **BREAKING**: no manifest key, no config key, and no keybinding changes. The list row's
grammar gains a cell only on worktree-sourced rows; every other row is byte-identical.

## Non-Goals

- **No agent visibility.** An agent whose working directory is inside a worktree stays
  out of scope for attribution, and `a`/`c`/`s` still launch in the pane's own root. Both are
  the follow-up change `worktree-agents`, which depends on this one.
- **No branch-name attribution** (PRD → Future #4).
- **No CLI correction for worktree copies.** They are file-sourced, exactly as archived changes
  already are; the file path counts tasks by the CLI's own rule.
- **No worktree changes in file mode.** File mode starts no refresh worker, and changing that is
  a separate argument about file mode, not about worktrees.
- **No write of any kind**, git's included: no `git worktree prune`, no index refresh, no fetch.
- No editing of OpenSpec files, no orchestration across changes, no change authoring, no Windows
  support — the PRD non-goals are untouched. Showing a change from a worktree is still reading.

## Capabilities

### New Capabilities

- `worktree-overlay`: discovering the worktree family, deriving each member's touched changes
  since its merge-base, overlaying them onto the pane's change set, the conflict rule, the
  idle re-check, and every degraded path — no `git`, a failing member, a prunable member, file
  mode.

### Modified Capabilities

- `subprocess-seam`: "Two traits carry the two programs" becomes three (a rename), the
  recording fake implements the third and keys it separately, and a new requirement names the
  one binding for the real `git` program and confines the `GitCli` handle.
- `change-model`: `ChangeSet` gains a `worktrees` field listing the family members the set was
  built against, with `assert_set_invariants`, `merge`, and `empty_set` carrying it.
- `refresh-worker`: `refresh::start` gains the git handle; each cycle's results are overlaid;
  the worker re-checks the family while idle and answers with an unsolicited `Files` result only
  when the overlay changed.
- `change-rows`: the row grammar gains the worktree marker cell, dropped whole before the agent
  badge.
- `detail-header`: a new `branched_header_row` draws a worktree-sourced change's branch cell,
  dropped whole before the gauge; `header_row` itself is untouched.
- `doc-conformance`: `src/worktrees.rs`' freedom from I/O and from every CLI handle becomes the
  seventeenth `tests/doc_contract.rs` claim, on the terms of the three pure modules before it.

## Impact

- `src/cli.rs` (`GitCli`, `RealGitCli`, `GIT_PROGRAM`, `git_cli_via`), new `src/worktrees.rs`,
  `src/changes.rs` (`ChangeSet::worktrees`, a touched-only file producer, the overlay),
  `src/refresh.rs` (worker body and `start`), `src/ui/mod.rs` (`Startup::git`,
  `start_collaborators`), `src/ui/list.rs`, `src/ui/detail.rs`, `src/ui/view.rs`, `src/lib.rs`.
- Gates: `NOCLI-SHELL`'s pattern gains `GitCli`; the new module joins the module map and the
  tested-modules list and becomes the seventeenth doc-contract claim; no new gate script, no new
  thread (the worker already exists), no new dependency. `LAUNCHSEAM` gains a third invocation
  confining the git handle, and `WIRED` gains the git binding. The suite already runs `git`
  (`tests/gate_controls.rs`); two real-repository tests are added, and `git` is named in
  `AGENTS.md` → Environment and `README.md` → Development.
- `SPEC.md` (module map, degraded-states table, the worktree paragraph beside row 30),
  `AGENTS.md`, `README.md` if it describes what the pane shows, and
  `tests/degraded-coverage.toml` for every new degraded row.
- **Roadmap:** unplanned work. `openspec/IMPLEMENTATION-ORDER.md` has no row for it; the roadmap
  predates Herdr's worktree workspaces, and `SPEC.md` recorded the gap as a limitation rather
  than a planned change. It gains a row when this change archives.
