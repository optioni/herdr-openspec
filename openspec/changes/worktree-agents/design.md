## Context

After `worktree-changes`, `ChangeSet::worktrees` lists the canonical OpenSpec roots of every live
member of the repository's worktree family, and a row whose change lies under one of them shows
that member's copy with an `@` marker. Two things still assume one checkout:

- `agents::attribute` admits an agent only when `cwd.starts_with(repo)`
  (`src/agents.rs:137-143`), so an agent working in a member — `<repo-parent>/.worktrees/…`, or
  anywhere else git put it — is neither badged nor counted. `SPEC.md` row 30 records this.
- `launch::split_args(repo)` (`src/launch.rs:215`) always splits at `launch::Settings::repo`, the
  pane's own root, fixed when the launcher starts.

Both are one-line decisions today, and both have the fact they need one hop away: the dashboard
holds `changes.worktrees` on every frame.

## Goals / Non-Goals

**Goals:**

- An agent in a member worktree is in scope, placed by exactly the landed tiers, or counted.
- A launch onto a worktree row starts its agent in that worktree.
- With no family, attribution and launch are byte-identical to today.

**Non-Goals:**

- Branch-name attribution, or any new evidence tier.
- Creating worktrees, or choosing one for a row the pane shows from its own checkout.
- Changing the derived agent name or the live-name refusal.

## Boundaries

| Module | What changes | Pattern followed |
|---|---|---|
| `src/agents.rs` | `attribute` gains `worktrees: &[&Path]`; the scope test admits any root; its 40 test calls pass `&[]` (`grep -c 'attribute(' src/agents.rs` minus the definition) | the landed scope test, extended rather than re-shaped |
| `src/ui/app.rs` | `Dashboard::attribution` passes `changes.worktrees`' roots; the launch `Go` branch fills `Request::Launch::root` from the selected change's `dir` | `attribution()` is already derived per frame from state the dashboard holds |
| `src/launch.rs` | `Request::Launch` gains `root`; `decide` builds it `None`; the worker passes `root.unwrap_or(settings.repo)` to `split_args` | `settings-window`'s own extension of `Request` with `Resolve` |
| `src/ui/driver.rs` | its `Request::Launch { .. }` patterns name the new field | compiler-driven |
| `SPEC.md`, `AGENTS.md`, `tests/degraded-coverage.toml` | scope paragraph, row 30, launch flow's `--cwd` | documentation group |

No process is spawned anywhere new: the worktree roots arrive on the `ChangeSet`, already
canonical. No view gains I/O: `ui::list` and `ui::view` reach the badge through
`Dashboard::attribution()` as before. `Change` is not altered; `ChangeSet` is not altered by this
change either.

## Contracts

- `agents::attribute` — breaking inside the crate: one production caller
  (`src/ui/app.rs:1965`) and 40 test calls in `src/agents.rs`, all passing an empty slice except
  the new scope tests. Additive in behaviour: an empty `worktrees` reproduces the four-argument
  result exactly.
- `launch::Request::Launch` — gains `root: Option<PathBuf>`; 34 construction or pattern sites
  (`grep -rn 'Request::Launch {' src` → 27 in `src/launch.rs`, 4 in `src/ui/driver.rs`, 3 in
  `src/ui/app.rs`). `launch::decide`'s seven-argument signature is untouched.
- No manifest key, config key, CLI argument, or keybinding moves.

## Persistence and Rollout

- **migration** — none.
- **backfill** — none. `agent-names.toml` records `(derived name → change name)`, never a
  checkout, so a mapping recorded before this change still places an agent that now runs in a
  worktree.
- **seeding** — none.
- **cache invalidation** — none; attribution is derived per frame and stored nowhere.
- **index rebuild** — none.
- **authorization** — none. The widened scope admits only roots git reported as this
  repository's own worktrees; `herdr agent list` stays session-global and every other repository's
  agents stay out of scope.
- **observability** — the footer's unattributed count may rise, by exactly the unplaced agents in
  member worktrees.
- **deployment** — `make build`; no manifest change.

## Test Boundaries

| Dependency | In the launch-worker tests | In unit and view tests |
|---|---|---|
| Filesystem | not reached — `split_args` renders a path it is given | not reached — `attribute` and the dashboard read in-memory values |
| `herdr` binary | a scratch `#!/bin/sh` program logging argument vectors, as `agent-launch`'s landed worker tests use | not reached |
| Herdr agent list | not reached | replaced — hand-built `Agent` values on `dashboard.agents` |
| `git` binary | not reached — the family arrives on `ChangeSet::worktrees` | not reached — `changes::fixture::with_worktrees` builds it |
| `openspec` binary | not reached — the resolved path is a `Settings` value | not reached |
| Terminal | not reached | replaced — `TestBackend` at 120x20 and 60x20 |
| Launcher worker thread | real, through the landed worker test harness | replaced — the dashboard's `launch.pending` is asserted before any worker sees it |
| Clock | none | none |

## Test Strategy

Tiers: **unit** (`cargo test --lib agents`, `… launch`, `… ui::app`) and **view**
(`cargo test --lib ui::list`, `… ui::view`, at 60 and 120 columns). This change takes **no
outer-loop acceptance test**: `worktree-changes` already drives `run_wired` end to end with a
worktree family on screen, and the two decisions here — a scope test and a `--cwd` value — are
fully observable one layer in, through `attribute`'s return value and the scratch `herdr`
program's argument log. A `run_wired` test here would exercise the loop, not the rule.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| An agent in a member worktree is in scope and placed by the ordinary tiers | `attribute` over the three agents with `worktrees` `["/w/feat"]` | unit | none | `cargo test --lib agents` |
| Without a family a worktree-shaped path stays out of scope | same fixture, `worktrees` empty, and `repo` `None` with a non-empty list; equality with the empty-list result | unit | none | `cargo test --lib agents` |
| A member's OpenSpec root bounds its scope | `attribute` with root `/w/feat/sub` | unit | none | `cargo test --lib agents` |
| The dashboard passes the family it holds | render at 120x20 and 60x20 with and without `changes.worktrees` | view | `TestBackend` | `cargo test --lib ui::list` |
| An agent in the repository with no matching name is counted, never assigned | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| The name tier reads `name` and never the agent kind | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| A terminal title naming a change attributes nothing | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| Every empty and absent input is total, not a panic | carried unchanged — existing test re-run with `&[]`; the `repo` `None` with a non-empty `worktrees` case is *Without a family a worktree-shaped path stays out of scope*'s | unit | none | `make test` |
| The pane map and the badge map always hold the same keys | existing test extended with the member-worktree fixture | unit | none | `cargo test --lib agents` |
| An agent in another repository is neither badged nor counted | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| A subdirectory is inside the repository and a sibling prefix is not | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| No repository means no badges and no count | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| A symlinked repository path still badges its agents | carried unchanged — existing test re-run | unit | injected canonicalizer | `make test` |
| An unresolvable working directory is kept verbatim, not dropped | carried unchanged — existing test re-run | unit | injected canonicalizer | `make test` |
| The worktree's root is filled from the selected row | `Dashboard::apply(Launch(Apply))` with a worktree row and with a base row | unit | none | `cargo test --lib ui::app` |
| A refusal is unchanged by the root | same, with a live agent named `x` | unit | none | `cargo test --lib ui::app` |
| `decide` never fills the root | `decide` for `Apply`, `Continue`, `Archive` | unit | none | `cargo test --lib launch` |
| The three calls appear in order with the split's own pane id | carried unchanged — existing worker test re-run (`root: None`) | unit | scratch `herdr`, worker thread | `make test` |
| Each intent sends its own `/opsx:*` command and nothing else | carried unchanged — existing worker test re-run | unit | scratch `herdr`, worker thread | `make test` |
| An archived change launches on the same terms as an active one | existing test's expected literal gains `root: None` | unit | none | `cargo test --lib ui::app` |
| A launch onto a worktree copy splits in that worktree | worker run with `root: Some("/w/feat")` and with `None`, comparing the logs | unit | scratch `herdr`, worker thread | `cargo test --lib launch` |

`degraded-coverage`'s scenario *An out-of-scope agent and a worktree agent are both invisible*
stays true and is not carried: its fixture has no worktree family, so its worktree-shaped `cwd` is
not a member and stays out of scope. Its test,
`ui::list::tests::out_of_scope_and_worktree_agents_are_invisible`, stays row 29's proof; row 30
gains *The dashboard passes the family it holds* as its proof, since row 30's text now describes
the member case.

## Decisions

**D1 — Widen the scope; add no tier.**
The alternative the PRD lists — attribute a worktree agent by its branch name — is new evidence,
and `SPEC.md` forbids attributing on weak evidence. Scope is not evidence: it only decides which
agents the existing tiers may look at, and git's worktree list is a fact about the repository, not
a guess about an agent. A launched agent is placed through its recorded mapping wherever it runs,
which covers every agent this plugin started.

**D2 — Keep `repo` and add `worktrees`, rather than replacing both with one `roots` slice.**
A single slice would read more simply, but it would rewrite every landed scenario's `repo`
wording and make "no repository" an empty slice indistinguishable from "a repository with no
root", which the landed no-repository scenario distinguishes. Adding one parameter leaves every
landed scenario true as written with an empty list.

**D3 — `repo: None` admits nothing, whatever `worktrees` holds.**
A family is only ever read for a resolved repository, so a non-empty list beside `None` is a
state no producer creates; treating it as empty keeps `attribute` total without inventing scope.

**D4 — The dashboard fills `root`; `decide` does not.**
`decide`'s seven arguments are the launch policy, and a checkout is not policy. Filling `root` in
the `Go` branch — from the selected change's `dir` against `changes.worktrees`, the same test
`ui::list` uses for the marker — keeps one place deciding "which checkout is this row" and leaves
`decide` and its landed scenarios untouched.

**D5 — The OpenSpec root, not the worktree's top level, is the launch `--cwd` and the scope root.**
The base already launches at the directory holding `openspec/` and scopes to it. A member whose
OpenSpec root is a subdirectory of its checkout is treated identically, so the two checkouts obey
one rule.

## Risks / Trade-offs

- [An agent in a member worktree working on something unrelated now raises the unattributed
  count] → intended: it is this repository's work, and a count is what the pane already shows for
  an unplaced agent in the main checkout.
- [A worktree removed while its agent still runs] → the next cycle drops it from the family, the
  agent falls out of scope, and its badge disappears — the same as an agent that `cd`s out of the
  repository today.
- [Two agents on one change in two checkouts] → `rank`'s existing fold shows the higher-precedence
  status; `worktree-changes` already reports two worktrees owning one change as a problem row.
- [A carried MODIFIED block reverts landed work] → the three carried blocks were extracted from
  `openspec/specs/` at this HEAD by script and diffed; re-extract if anything archives first,
  `worktree-changes` included.

## Migration Plan

None. Apply after `worktree-changes`; rollback is `git revert`.

## Open Questions

None.
