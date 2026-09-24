## Context

After `worktree-changes`, `ChangeSet::worktrees` lists the canonical OpenSpec roots of every live
member of the repository's worktree family, and `worktrees::member_of(&worktrees, &dir)` answers
which member's copy a change is — by matching `dir` against each member's
`<root>/openspec/changes`, so a pane opened inside a worktree nested under the main checkout does
not mistake its own rows for the main checkout's. Two things still assume one checkout:

- `agents::attribute` admits an agent only when `cwd.starts_with(repo)`
  (`src/agents.rs:137-143`), so an agent working in a member — `<repo-parent>/.worktrees/…`, or
  anywhere else git put it — is neither badged nor counted. `SPEC.md`'s degraded-states row
  "An agent works in a **linked worktree** of this repository" records this.
- `launch::split_args(repo)` (`src/launch.rs:215`) always splits at `launch::Settings::repo`, the
  pane's own root, fixed when the launcher starts.

Both have the fact they need one hop away: the dashboard holds `changes.worktrees` on every frame.

## Goals / Non-Goals

**Goals:**

- An agent in a member worktree is in scope, placed by exactly the landed tiers, or counted.
- A launch onto a worktree row starts its agent in that worktree; a launch onto any other row is
  unchanged.
- With no family, attribution and launch are byte-identical to today.

**Non-Goals:**

- Branch-name attribution, or any new evidence tier.
- Creating worktrees, or choosing one for a row the pane shows from its own checkout.
- Changing the derived agent name or the live-name refusal.

## Boundaries

| Module | What changes | Pattern followed |
|---|---|---|
| `src/agents.rs` | `attribute` gains `worktrees: &[&Path]`; the scope test admits any root; its 40 test calls pass `&[]` (`grep -c 'attribute(' src/agents.rs` → 41 including the definition at `:116`) | the landed scope test, extended rather than re-shaped |
| `src/ui/app.rs` | `Dashboard::attribution` passes `changes.worktrees`' roots; the launch `Go` branch (`src/ui/app.rs:1743`) fills `Request::Launch::root` through `worktrees::member_of` | `attribution()` is already derived per frame from state the dashboard holds; `member_of` is the one derivation `worktree-changes` uses for the marker and the branch header |
| `src/launch.rs` | `Request::Launch` gains `root`; `decide` builds it `None`; `handle` passes `root.unwrap_or(&settings.repo)` to `split_args` | `settings-window`'s own extension of `Request` |
| `src/ui/driver.rs` | its four `Request::Launch { … }` patterns name the new field | compiler-driven |
| `SPEC.md`, `AGENTS.md`, `tests/degraded-coverage.toml` | scope paragraph, the two worktree/cwd degraded rows, launch flow's `--cwd`, the tested-modules bullet | documentation group |

No process is spawned anywhere new: the worktree roots arrive on the `ChangeSet`, already
canonical. No view gains I/O: `ui::list` and `ui::view` reach the badge through
`Dashboard::attribution()` as before. `Change` and `ChangeSet` are not altered by this change.

Every `Request::Launch` literal or pattern this change writes names all four fields:
`NODEFAULT-UI` (`Makefile:49`, `TYPES='Launch'`) scans every `Launch { … }` span and fails on a
`..`, and its floor sits exactly at the current 99 spans, so the Go branch fills `root` with a
full destructure, not `if let Request::Launch { root, .. }`.

## Contracts

- `agents::attribute` — breaking inside the crate: one production caller
  (`src/ui/app.rs:1965`) and 40 test calls in `src/agents.rs`. Additive in behaviour: an empty
  `worktrees` reproduces the four-argument result exactly.
- `launch::Request::Launch` — gains `root: Option<PathBuf>`; 34 construction or pattern sites
  (`grep -rn 'Request::Launch {' src` → 27 in `src/launch.rs`, 4 in `src/ui/driver.rs`, 3 in
  `src/ui/app.rs`), all named by the compiler. `launch::decide`'s seven-argument signature is
  untouched; its requirement is carried so that the `Request` quote, rule 7, and its per-intent
  scenario name the field.
- **Specs quoting `Request::Launch` that are not carried:** `dashboard-loop` → "Key handling is a
  pure, total function…" (434 lines) and "The loop draws before it waits…" (218), and
  `live-updates` → "The loop drives the live tier…" (319), each quote a three-field literal in one
  scenario. Every one of those scenarios launches a change from the pane's own checkout, so the
  literal they mean is the `root: None` one; carrying 971 lines to append one field to three
  literals is the oversized-carry risk the schema warns about, and each would be re-derived when
  those capabilities next move.
- No manifest key, config key, CLI argument, or keybinding moves.

## Persistence and Rollout

- **migration** — none.
- **backfill** — none. `agent-names.toml` records `(derived name → change name)`, never a
  checkout, and `state::record` writes only when the two differ (`src/state.rs:292-295`), so a
  mapping recorded before this change still places an agent that now runs in a worktree.
- **seeding** — none.
- **cache invalidation** — none; attribution is derived per frame and stored nowhere.
- **index rebuild** — none.
- **authorization** — none. The widened scope admits only roots git reported as this
  repository's own worktrees; `herdr agent list` stays session-global and every other repository's
  agents stay out of scope.
- **observability** — the footer's unattributed count may rise, by exactly the unplaced agents in
  member worktrees.
- **deployment** — `make build`; no manifest change.
- **archive** — the archive step owns three edits no delta can make, each inside `openspec/`
  outside this change's directory, which only an archive writes: this change's row in
  `openspec/IMPLEMENTATION-ORDER.md` beside `worktree-changes`'; `agent-attribution`'s Purpose
  ("every tier scoped to the resolved repository"); and `agent-poller`'s out-of-scope
  parenthetical, which the Test Strategy section leaves uncarried.

## Test Boundaries

| Dependency | In the launch-worker tests | In unit and view tests |
|---|---|---|
| Filesystem | **real** — a `ScratchDir` state directory, because `run_request` calls `state::record` | not reached — `attribute` and the dashboard read in-memory values |
| `herdr` binary | **replaced** — `cli::FakeCli`'s `HerdrCli` side, keyed on exact argument vectors, driven through `launch`'s `handle` (not `run_request`, so the `root`-or-`repo` choice is covered wherever it lives) with `Settings::repo` `/r`; `split_ok`'s hardcoded `REPO` gains a root-parameterised variant | not reached |
| Herdr agent list | not reached | **replaced** — hand-built `Agent` values on `dashboard.agents` |
| `git` binary | not reached — the family arrives on `ChangeSet::worktrees` | not reached — `changes::fixture::with_worktrees` builds it |
| `openspec` binary | not reached — the resolved path is a `Settings` value | not reached |
| Terminal | not reached | **replaced** — `TestBackend` at 120x20 and 60x20 for the footer; `ui::list::rows` at 38 and 58 for the badge |
| Launcher worker thread | not reached — `handle` is called directly | not reached — `launch.pending` is asserted before any worker sees it |
| Clock | none | none |

## Test Strategy

Tiers: **unit** (`cargo test --lib agents::`, `… launch::`, `… ui::app`) and **view**
(`cargo test --lib ui::list`, `… ui::view`). This change takes **no outer-loop acceptance
test**: `worktree-changes` already drives `run_wired` end to end with a worktree family on screen,
and the two decisions here — a scope test and a `--cwd` value — are fully observable one layer
in, through `attribute`'s return value and `FakeCli`'s recorded calls. A `run_wired` test here
would exercise the loop, not the rule.

Three new tests are **regression guards**, not RED evidence: *`decide` never fills the root*
(the compiler forces `None`), *Without a family a worktree-shaped path stays out of scope* (true
under the old rule; its `repo None` arm guards D3), and the refusal half of *A refusal is
unchanged by the root, and `g` reaches the worktree agent*. Group RED is shown by the
discriminating tests against a stub that adds only the new signature and field.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| An agent in a member worktree is in scope and placed by the ordinary tiers | `attribute` over the three agents with `worktrees` `["/w/feat"]` | unit | none | `cargo test --lib agents::` |
| Without a family a worktree-shaped path stays out of scope | same fixture, `worktrees` empty, and `repo` `None` with a non-empty list; equality with the empty-list result (regression guard) | unit | none | `cargo test --lib agents::` |
| A member's OpenSpec root bounds its scope | `attribute` with root `/w/feat/sub`, both placements | unit | none | `cargo test --lib agents::` |
| The dashboard passes the family it holds | `rows()` at 38 and 58, with and without `changes.worktrees` | unit | none | `cargo test --lib ui::list` |
| The footer counts an unplaced agent in a member worktree | render at 120x20 and 60x20, with and without `changes.worktrees` | view | `TestBackend` | `cargo test --lib ui::view` |
| An agent in the repository with no matching name is counted, never assigned | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| The name tier reads `name` and never the agent kind | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| A terminal title naming a change attributes nothing | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| Every empty and absent input is total, not a panic | carried unchanged — existing test re-run with `&[]`; the `repo` `None` with a non-empty `worktrees` case is *Without a family…*'s | unit | none | `make test` |
| The pane map and the badge map always hold the same keys | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| An agent in another repository is neither badged nor counted | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| A subdirectory is inside the repository and a sibling prefix is not | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| No repository means no badges and no count | carried unchanged — existing test re-run with `&[]` | unit | none | `make test` |
| A symlinked repository path still badges its agents | carried unchanged — existing test re-run | unit | injected canonicalizer | `make test` |
| An unresolvable working directory is kept verbatim, not dropped | carried unchanged — existing test re-run | unit | injected canonicalizer | `make test` |
| The worktree's root is filled from the selected row | `Dashboard::apply(Launch(Apply))` with a worktree row and with a base row | unit | none | `cargo test --lib ui::app` |
| A pane inside a nested worktree launches its own rows in place | same, with repository root `/r/.worktrees/feat` and member `/r` | unit | none | `cargo test --lib ui::app` |
| A refusal is unchanged by the root, and `g` reaches the worktree agent | `a` then `g` with a live agent at `/w/feat`, then `g` with the family emptied | unit | none | `cargo test --lib ui::app` |
| `decide` never fills the root | `decide` for `Apply`, `Continue`, `Archive`, whole-literal equality (regression guard) | unit | none | `cargo test --lib launch::` |
| An unreachable socket makes every action key inert | carried unchanged — existing `decide` test re-run; any expected `Request::Launch` literal gains `root: None` | unit | none | `make test` |
| File mode refuses the three launch keys and names the missing binary | carried unchanged — existing `decide` test re-run; any expected `Request::Launch` literal gains `root: None` | unit | none | `make test` |
| Focus is exempt from file mode | carried unchanged — existing `decide` test re-run; any expected `Request::Launch` literal gains `root: None` | unit | none | `make test` |
| No selected change means no launch, and no agent means no focus | carried unchanged — existing `decide` test re-run; any expected `Request::Launch` literal gains `root: None` | unit | none | `make test` |
| A derived name already live in the session is refused before any Herdr call | carried unchanged — existing `decide` test re-run; any expected `Request::Launch` literal gains `root: None` | unit | none | `make test` |
| A second press while a launch is in flight is refused, not queued | carried unchanged — existing `decide` test re-run; any expected `Request::Launch` literal gains `root: None` | unit | none | `make test` |
| Focus still works while a launch is in flight | carried unchanged — existing `decide` test re-run; any expected `Request::Launch` literal gains `root: None` | unit | none | `make test` |
| Every combination is total | carried unchanged — existing `decide` test re-run; any expected `Request::Launch` literal gains `root: None` | unit | none | `make test` |
| Each launch intent carries its own change and derived name | existing test's expected literals gain `root: None`, as the carried scenario now states | unit | none | `cargo test --lib launch::` |
| The three calls appear in order with the split's own pane id | carried unchanged — existing worker test re-run (`root: None`) | unit | `FakeCli`, `ScratchDir` state dir | `make test` |
| Each intent sends its own `/opsx:*` command and nothing else | carried unchanged — existing worker test re-run | unit | `FakeCli`, `ScratchDir` state dir | `make test` |
| An archived change launches on the same terms as an active one | existing test's expected literal gains `root: None` | unit | none | `cargo test --lib ui::app` |
| A launch onto a worktree copy splits in that worktree | `handle` with `root: Some("/w/feat")` and with `None`, comparing `FakeCli`'s recorded calls | unit | `FakeCli`, `ScratchDir` state dir | `cargo test --lib launch::` |

`degraded-coverage`'s scenario *An out-of-scope agent and a worktree agent are both invisible*
stays true and is not carried: its fixture has no worktree family, so its worktree-shaped `cwd` is
not a member and stays out of scope. Its test stays the proof of the row "A live agent's `cwd` is
absent, or outside the resolved repository root" (whose condition this change rewords); the
linked-worktree row gains *The dashboard passes the family it holds* as its proof. `agent-poller`
→ its out-of-scope parenthetical ("one whose `cwd` is absent, or lies outside the resolved
repository root") defers to `agent-attribution`'s definition and its scenario stays true; it is
not carried either, and the documentation group names it for the next change that moves that
capability.

## Decisions

**D1 — Widen the scope; add no tier.**
The alternative the PRD lists — attribute a worktree agent by its branch name — is new evidence,
and `SPEC.md` forbids attributing on weak evidence. Scope is not evidence: it only decides which
agents the existing tiers may look at, and git's worktree list is a fact about the repository, not
a guess about an agent. An agent this plugin launched is placed wherever it runs: through its
recorded mapping when its derived name differs from the change name, and through the name tier
when the two are equal, since `state::record` records nothing then.

**D2 — Keep `repo` and add `worktrees`, rather than replacing both with one `roots` slice.**
A single slice would rewrite every landed scenario's `repo` wording and make "no repository" an
empty slice indistinguishable from "a repository with no root", which the landed no-repository
scenario distinguishes. Adding one parameter leaves every landed scenario true as written with an
empty list.

**D3 — `repo: None` admits nothing, whatever `worktrees` holds.**
A family is only ever read for a resolved repository, so a non-empty list beside `None` is a
state no producer creates; treating it as empty keeps `attribute` total without inventing scope.

**D4 — The dashboard fills `root` through `worktrees::member_of`; `decide` does not.**
`decide`'s seven arguments are the launch policy, and a checkout is not policy. `member_of` is the
same function `worktree-changes` calls for the `@` marker and the branch header, so the list, the
header, and the launch cannot disagree about which checkout a row is — including the nested layout
where a prefix test over member roots would claim the pane's own rows for the main checkout.

**D5 — The OpenSpec root, not the worktree's top level, is the launch `--cwd` and the scope root.**
The base already launches at the directory holding `openspec/` and scopes to it. A member whose
OpenSpec root is a subdirectory of its checkout is treated identically, so the two checkouts obey
one rule.

## Risks / Trade-offs

- [An agent in a member worktree working on something unrelated raises the unattributed count] →
  intended: it is this repository's work, and a count is what the pane already shows for an
  unplaced agent in the main checkout.
- [A worktree removed while its agent still runs] → the next cycle drops it from the family, the
  agent falls out of scope, and its badge disappears — the same as an agent that `cd`s out of the
  repository today.
- [A worktree removed within the up-to-two-second window before the family catches up, and `a`
  pressed on its row] → `pane split --cwd <gone>` is sent. What Herdr does with a missing `--cwd`
  is **unmeasured**: if it fails, the landed "`pane split` fails" row applies; if it silently falls
  back to another directory, the agent starts elsewhere. It was not measured at planning time
  because doing so splits a pane in the user's live Herdr session; task 2.4 measures it in a
  throwaway workspace, with the user's go-ahead, and records the outcome here.
- [Two agents on one change in two checkouts] → `rank`'s existing fold shows the higher-precedence
  status; `worktree-changes` already reports two worktrees owning one change as a problem row.
- [A carried MODIFIED block reverts landed work] → the four carried blocks were extracted from
  `openspec/specs/` at this HEAD by script and diffed; re-extract before implementing if any
  change archives first.

## Migration Plan

None. Apply after `worktree-changes`; rollback is `git revert`.

## Open Questions

None blocking. What Herdr does with a `--cwd` that no longer exists is an open measurement, owned
by task 2.4.
