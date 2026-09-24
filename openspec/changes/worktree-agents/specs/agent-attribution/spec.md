## MODIFIED Requirements

### Requirement: Attribution is a pure, total function that refuses to guess

`agents::attribute` SHALL map live agents onto changes, and SHALL be a pure total function of
its five arguments:

```rust
pub struct Attribution {
    pub badges: std::collections::BTreeMap<String, AgentStatus>,
    pub panes: std::collections::BTreeMap<String, String>,
    pub unattributed: usize,
}

pub fn attribute(
    agents: &[Agent],
    repo: Option<&std::path::Path>,
    worktrees: &[&std::path::Path],
    change_names: &[&str],
    mapping: &std::collections::BTreeMap<String, String>,
) -> Attribution;
```

`panes` is `agent-launch`'s addition: for every change that has a badge, the `pane_id` of the
**same agent** whose status that badge shows. It SHALL be maintained in lockstep with `badges`,
so the two maps always hold exactly the same key set and never disagree about which agent a
change's row is talking about — `g` focuses the agent the badge is describing, or the badge and
the focus would be about different panes. `pane_id` is Herdr's only field this crate reads that
is present on **every** agent; `name` is present only on an agent someone named, which is why
the focus target is the pane and not the name even though Herdr resolves both.

`panes` is derived from the `Agent` values already passed in and added no argument.
`worktrees` is `worktree-agents`' addition and the only one: the OpenSpec roots of the
repository's worktree family, `ChangeSet::worktrees`' roots in order, which the repository-scope
requirement below admits alongside `repo`. It widens **which agents are in scope** and nothing
else — no tier reads it, and it is never a fourth source of evidence. Every scenario in this
capability that names no worktree passes an empty `worktrees`, and an empty `worktrees` SHALL
make `attribute` return exactly what the four-argument function returned.

It SHALL perform no filesystem, process, environment, network, or terminal I/O, SHALL read no
clock and no global state, SHALL spawn nothing, and SHALL never panic for any combination of
arguments, including empty slices, an empty map, a `repo` of `None` with a non-empty
`worktrees`, an agent with every optional field absent, and a `change_names` slice holding
duplicates.

An agent is first placed **in or out of the repository's scope** by the requirement below;
only an in-scope agent reaches a tier at all. Exactly **three** tiers then decide an in-scope
agent's fate, evaluated in this order and no other, and no fourth source of evidence SHALL
attribute an agent:

1. the plugin-local agent-name **mapping**, which records the name this plugin derived when it
   launched an agent;
2. an agent `name` equal, byte for byte, to a change name;
3. everything else, which is **counted** in `Attribution::unattributed` and assigned to no
   change.

An agent placed by tier 3 SHALL contribute **no** entry to `panes`, exactly as it contributes
none to `badges`. `g` on such a change therefore has no target and does nothing — the refusal to
guess extends to the focus key, which never picks "some agent nearby".

`Agent::terminal_title` SHALL NOT be read by `attribute`, on any path. A terminal title is a
summary written by whatever the agent is doing, not a change identifier, and treating it as one
is precisely the guess this design refuses.

`Agent::kind` — Herdr's `agent` field, the agent **kind**, `"claude"` on every live agent
measured against Herdr 0.8.2 — SHALL NOT be read by `attribute` on any path either. It is a
different field from `Agent::name`, and matching a change name against it would attribute every
change named `claude` to every live agent and nothing else to anything.

`Attribution` SHALL NOT implement `Default`, derived or hand-written, anywhere in the crate, and
every construction and destructuring of it SHALL name all **three** fields with no `..` rest, on
exactly `Agent`'s, `Listed`'s, and `AgentSnapshot`'s terms.

#### Scenario: An agent in the repository with no matching name is counted, never assigned

- **WHEN** `attribute` is called with one agent whose `cwd` is `/repo`, whose `name` is
  `Some("scratch-work")`, whose `pane_id` is `w8:p9`, and whose `status` is `Working`; `repo`
  `Some("/repo")`; `change_names` `["add-auth", "fix-basket"]`; and an empty mapping
- **THEN** `badges` is empty — neither `add-auth` nor `fix-basket` gains an entry
- **AND** `panes` is empty too, so `w8:p9` is reachable by no change's `g`
- **AND** `unattributed` is exactly `1`
- **AND** the same call with `change_names` reordered to `["fix-basket", "add-auth"]` returns an
  equal `Attribution`, so nothing was assigned by position

#### Scenario: The name tier reads `name` and never the agent kind

- **WHEN** `attribute` is called with one agent whose `cwd` is `/repo`, whose `kind` is
  `Some("claude")`, whose `name` is `None`, whose `pane_id` is `w8:p1`, and whose `status` is
  `Idle`; `repo` `Some("/repo")`; `change_names` `["claude"]` — a change whose name is exactly
  the agent kind; and an empty mapping
- **THEN** `badges` is empty, `panes` is empty, and `unattributed` is `1`
- **AND** the same call with the agent's `name` set to `Some("claude")` returns `badges` holding
  exactly `{"claude": Idle}`, `panes` holding exactly `{"claude": "w8:p1"}`, and `unattributed`
  `0`, so the two fields are distinguished rather than conflated

#### Scenario: A terminal title naming a change attributes nothing

- **WHEN** `attribute` is called with one agent whose `cwd` is `/repo`, whose `name` is `None`,
  and whose `terminal_title` is `Some("✳ add-auth: wiring the token refresh")`; `repo`
  `Some("/repo")`; `change_names` `["add-auth"]`; and an empty mapping
- **THEN** `badges` is empty, `panes` is empty, and `unattributed` is `1`
- **AND** the identical call with `terminal_title` set to `None` returns an equal `Attribution`,
  so the title changed no outcome at all

#### Scenario: Every empty and absent input is total, not a panic

- **WHEN** `attribute` is called five times, each with `repo` `Some("/repo")` and an empty
  mapping unless stated: (1) an empty `agents` slice and `change_names` `["alpha"]`; (2) one
  `Working` agent at `cwd` `/repo` named `alpha` with an **empty** `change_names`; (3) that
  same agent with `change_names` `["alpha"]` but `repo` `None`; (4) one agent whose `name`,
  `kind`, `cwd`, and `terminal_title` are all `None`, with `change_names` `["alpha"]`; (5) one
  `Idle` agent at `cwd` `/repo` named `alpha` with `change_names` `["alpha", "alpha"]`
- **THEN** none of the five panics
- **AND** call 1 returns `badges` empty, `panes` empty, and `unattributed` `0`: there is no
  agent to place
- **AND** call 2 returns `badges` empty, `panes` empty, and `unattributed` **`1`** — an in-scope
  agent that no
  tier could place is **counted**, and a repository with no changes at all does not make it
  vanish. An implementation that skipped the count whenever `change_names` was empty would pass
  a weaker form of this bullet and fail this one
- **AND** call 3 returns `badges` empty, `panes` empty, and `unattributed` `0`: with no
  repository there is no scope to attribute within
- **AND** call 4 returns `badges` empty, `panes` empty, and `unattributed` `0`, because an agent
  with no `cwd` is out of scope by the repository-scope requirement below
- **AND** call 5 returns `badges` holding exactly `{"alpha": Idle}` and `panes` holding exactly
  one entry for `alpha` — one entry each, not two — and
  `unattributed` `0`, so a duplicated change name attributes once
- **AND** in every one of the five, `panes` is empty exactly when `badges` is: the two maps are
  built in one pass and cannot diverge on an empty or absent input any more than on a populated
  one

#### Scenario: The pane map and the badge map always hold the same keys

- **WHEN** `attribute` is called with any of the fixtures this capability's scenarios use — the
  three-tier fixture, the reordered fixture, the out-of-scope fixture, the no-repository
  fixture, and the five-agent precedence fixture
- **THEN** `badges.keys()` and `panes.keys()` are equal as sets on every one of them
- **AND** for every key, `panes[key]` is the `pane_id` of an agent whose `status` is
  `badges[key]`, so the two maps describe one agent rather than two


### Requirement: Every tier is scoped to the resolved repository

`herdr agent list` is **session-global**: measured against Herdr 0.8.2, it returned
byte-identical output from three different working directories and listed agents whose `cwd` lay
in a repository other than the current one. `SPEC.md` scopes only tier 3 to the repository;
every tier SHALL be scoped to it, or an agent renamed `add-auth` in an unrelated repository
would badge this repository's `add-auth` row.

An agent SHALL be **in scope** exactly when `repo` is `Some(root)`, the agent's `cwd` is
`Some(path)`, and either `path.starts_with(root)` or `path.starts_with(w)` for some `w` in
`worktrees` — Rust's component-wise prefix test, which is true for an equal path and false for a
sibling whose textual prefix matches. With `repo` `None` no agent is in scope, whatever
`worktrees` holds: a pane with no repository has no family either, and a stray list must not
invent one.

The worktree roots are `worktree-agents`' addition. A linked worktree of the repository —
Herdr places one at `<repo-parent>/.worktrees/<repo>-<branch>`, outside the root — is the same
repository's work, and `worktree-changes` already shows its changes on this pane's rows, so an
agent working there belongs to this pane's scope. Only a root the family lists widens the scope:
when no family was read (no `git`, file mode, a failed `worktree list`) `worktrees` is empty and a
worktree-shaped `cwd` is out of scope exactly as before. A directory **beneath** a listed root is
in scope because it lies under that root — including, when the main checkout is a member, a
worktree git no longer lists that sits inside it — exactly as any directory beneath the pane's
own root already is. Each root is a member's **OpenSpec
root**, on the same terms `root` is the directory holding `openspec/`, so a member whose OpenSpec
root is a subdirectory of its checkout admits agents under that subdirectory only. An agent that is not in
scope SHALL be neither badged nor counted: it contributes nothing to `badges` and nothing to
`unattributed`, and is invisible to the pane.

An agent carrying no `cwd` SHALL be out of scope. `cwd` is not one of the seven fields Herdr's
own schema marks required, so its absence is an ordinary payload, not a fault; but an agent
whose working directory is unknown cannot be shown to be in this repository, and counting it
would be a guess.

Both sides of the comparison SHALL be canonical by the time they reach `attribute`, and
`attribute` SHALL NOT canonicalize either: it performs no filesystem I/O, so it cannot.
`resolve::find_repo` already canonicalizes the root it returns. Herdr, however, reports the
working directory **verbatim as the pane process was given it**, which is not canonical
whenever the repository is reached through a symbolic link — `/tmp` resolving to
`/private/tmp` on macOS, a symlinked home directory, a path under `/Volumes`. In that state
the component-wise prefix test fails for **every** agent, and because the scope test precedes
the unattributed count, the agents are not even reported as a number: every badge and the
footer count vanish with no problem row and no other signal.

Canonicalization SHALL therefore happen once, on the poller's own side of the seam, before a
snapshot reaches `attribute`: `agents::AgentSnapshot`'s agents SHALL carry a `cwd` that has
been passed through the same canonicalizing step `resolve::find_repo` applies to the root.
The step SHALL be an injected `&dyn Fn(&Path) -> Option<PathBuf>` on exactly
`config::env_lookup`'s and `resolve::openspec_bin`'s npm-hook terms, so the rule is driven by
a test without a real symlink, and its one production binding SHALL live outside `src/ui/`
alongside the poller. A path that cannot be canonicalized — it no longer exists, or the
process cannot resolve it — SHALL be kept **verbatim** rather than dropped: a stale directory
is still better evidence than none, and dropping it would reintroduce the same silent
disappearance from the other direction.

`attribute` SHALL remain a pure, total function performing no filesystem I/O; the
canonicalization is a property of the values it is handed, never of the function. The worktree
roots arrive canonical as well: the refresh worker canonicalizes every member path before it
reaches `ChangeSet::worktrees` (`worktree-overlay`).

#### Scenario: An agent in another repository is neither badged nor counted

- **WHEN** `attribute` is called with two agents, both `Working`: the first with `cwd`
  `/other/repo` and `name` `Some("add-auth")`, the second with `cwd` `/repo` and `name`
  `Some("nothing-like-a-change")`; `repo` `Some("/repo")`; `change_names` `["add-auth"]`; and an
  empty mapping
- **THEN** `badges` is empty — the out-of-scope agent's name matched a change and was still not
  attributed
- **AND** `unattributed` is exactly `1`, counting the in-scope agent alone and not the
  out-of-scope one
- **AND** removing the scope test is still discriminating here, though not through the count:
  this fixture's out-of-scope agent shares its name with the one change on screen, so without
  the scope test it would be placed by the **name tier** rather than counted — `badges` would
  become `{"add-auth": Working}` instead of staying empty, and `unattributed` would stay `1`.
  The count-moves-to-`2` direction is proven by `agent-poller`'s outer-loop scenario below,
  whose out-of-scope agent is deliberately named to match **no** change on screen for exactly
  this reason — a repair recorded in planning-review.md after task 8.3 found the same collision
  in that scenario's own fixture

#### Scenario: A subdirectory is inside the repository and a sibling prefix is not

- **WHEN** `attribute` is called with `repo` `Some("/repo")`, an empty mapping, `change_names`
  `[]`, and three `Idle` agents whose `cwd` values are `/repo`, `/repo/openspec/changes/alpha`,
  and `/repo-other`
- **THEN** `unattributed` is exactly `2`: the agent at the root and the agent in the
  subdirectory are in scope, and the sibling `/repo-other` is not
- **AND** the sibling's exclusion is a component-wise comparison rather than a string prefix,
  which a textual `starts_with` on the display string would have admitted

#### Scenario: No repository means no badges and no count

- **WHEN** `attribute` is called with `repo` `None`, three agents whose `cwd` values are all
  `/repo`, `change_names` `["add-auth"]`, and a mapping holding `c-add-auth -> add-auth`
- **THEN** `badges` is empty and `unattributed` is `0`
- **AND** nothing panics: a pane that never found a repository has no scope to attribute within,
  and its list has no rows to badge

#### Scenario: A symlinked repository path still badges its agents

- **WHEN** the poller's canonicalizing hook is a test closure mapping `/tmp/repo` to
  `/private/tmp/repo` and every other path to itself, Herdr reports one `Working` agent whose
  `cwd` is `/tmp/repo` and whose `name` is `Some("add-auth")`, and the resolved root is the
  canonical `/private/tmp/repo` with `change_names` `["add-auth"]`
- **THEN** the snapshot reaching `attribute` carries `cwd` `/private/tmp/repo`, and `badges` is
  `{"add-auth": Working}`
- **AND** without the canonicalizing step the same fixture yields empty `badges` **and**
  `unattributed` `0` — the whole-session disappearance this scenario exists to catch — so the
  test is discriminating in both directions
- **AND** the hook is an injected closure, so no symbolic link is created on disk and the
  scenario runs identically on both supported platforms

#### Scenario: An unresolvable working directory is kept verbatim, not dropped

- **WHEN** the canonicalizing hook returns `None` for an agent's `cwd` `/repo/gone`, the
  resolved root is `/repo`, and `change_names` is `[]`
- **THEN** the snapshot carries `cwd` `/repo/gone` unchanged and the agent is counted:
  `unattributed` is `1`
- **AND** an agent whose directory has since been deleted is therefore still reported as a
  number rather than silently vanishing, which is the same failure this requirement's
  canonicalization exists to remove

#### Scenario: An agent in a member worktree is in scope and placed by the ordinary tiers

- **WHEN** `attribute` is called with `repo` `Some("/r")`, `worktrees` `["/w/feat"]`,
  `change_names` `["x"]`, a mapping `c-x -> x`, and three agents: a `Working` agent at `cwd`
  `/w/feat` named `c-x` in pane `w1:p1`, a `Working` agent at `/w/feat/openspec/changes/x` named
  `scratch`, and a `Blocked` agent at `/w/feat-other` named `x` in pane `w1:p3`
- **THEN** `badges` is exactly `{"x": Working}`, placed through the mapping tier by the first
  agent, and `panes["x"]` is `w1:p1`
- **AND** `unattributed` is exactly `1` — the second agent, in scope and placed by no tier
- **AND** the third agent, whose name matches `x` byte for byte, is neither badged nor counted:
  admitting it would turn the badge `Blocked` and the pane `w1:p3`, since `Blocked` outranks
  `Working`, and `/w/feat-other` shares only a textual prefix with the member root, so the test is
  component-wise

#### Scenario: Without a family a worktree-shaped path stays out of scope

- **WHEN** the same call is made with `worktrees` empty
- **THEN** `badges` is empty and `unattributed` is `0`, equal to the result the four-argument
  function returned for the same agents
- **AND** with `repo` `None` and `worktrees` `["/w/feat"]` the result is also empty and `0`

#### Scenario: A member's OpenSpec root bounds its scope

- **WHEN** `attribute` is called with `repo` `Some("/r")`, `worktrees` `["/w/feat/sub"]`,
  `change_names` `["x"]`, an empty mapping, and two `Idle` agents named `x`, at `/w/feat/sub/x`
  in pane `w1:p1` and at `/w/feat` in pane `w1:p2`
- **THEN** `badges` is exactly `{"x": Idle}` with `panes["x"]` `w1:p1`, and `unattributed` is `0`
- **AND** swapping which agent is at which path swaps nothing into scope: the agent at `/w/feat`
  is out of scope, so with only that agent `badges` is empty and `unattributed` is `0`

#### Scenario: The dashboard passes the family it holds

- **WHEN** `ui::list::rows` is called at widths 38 and 58 for a `Dashboard` whose repository root
  is `/r`, whose `changes.worktrees` holds `(/w/feat, "feat")`, whose one active change `x` has
  `dir` `/w/feat/openspec/changes/x`, with `selected` 1, and which holds one `Working` agent at
  `cwd` `/w/feat` named `x`
- **THEN** `x`'s row carries the `w` badge followed by the `@` marker
- **AND** the same dashboard with `changes.worktrees` emptied yields a row with no badge, so the
  badge came from the family and not from the fixture

#### Scenario: The footer counts an unplaced agent in a member worktree

- **WHEN** the same `Dashboard`, with its one agent renamed `scratch`, is rendered at 120x20 and
  at 60x20
- **THEN** the footer's last hint is `1 unattributed`
- **AND** with `changes.worktrees` emptied the footer carries no unattributed count, so the count
  follows the family

