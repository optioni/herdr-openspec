## ADDED Requirements

### Requirement: Attribution is a pure, total function that refuses to guess

`agents::attribute` SHALL map live agents onto changes, and SHALL be a pure total function of
its four arguments:

```rust
pub struct Attribution {
    pub badges: std::collections::BTreeMap<String, AgentStatus>,
    pub unattributed: usize,
}

pub fn attribute(
    agents: &[Agent],
    repo: Option<&std::path::Path>,
    change_names: &[&str],
    mapping: &std::collections::BTreeMap<String, String>,
) -> Attribution;
```

It SHALL perform no filesystem, process, environment, network, or terminal I/O, SHALL read no
clock and no global state, SHALL spawn nothing, and SHALL never panic for any combination of
arguments, including empty slices, an empty map, a `repo` of `None`, an agent with every
optional field absent, and a `change_names` slice holding duplicates.

An agent is first placed **in or out of the repository's scope** by the requirement below;
only an in-scope agent reaches a tier at all. Exactly **three** tiers then decide an in-scope
agent's fate, evaluated in this order and no other, and no fourth source of evidence SHALL
attribute an agent:

1. the plugin-local agent-name **mapping**, which records the name this plugin derived when it
   launched an agent;
2. an agent `name` equal, byte for byte, to a change name;
3. everything else, which is **counted** in `Attribution::unattributed` and assigned to no
   change.

`Agent::terminal_title` SHALL NOT be read by `attribute`, on any path. A terminal title is a
summary written by whatever the agent is doing, not a change identifier, and treating it as one
is precisely the guess this design refuses.

`Agent::kind` — Herdr's `agent` field, the agent **kind**, `"claude"` on every live agent
measured against Herdr 0.8.2 — SHALL NOT be read by `attribute` on any path either. It is a
different field from `Agent::name`, and matching a change name against it would attribute every
change named `claude` to every live agent and nothing else to anything.

`Attribution` SHALL NOT implement `Default`, derived or hand-written, anywhere in the crate, and
every construction and destructuring of it SHALL name both fields with no `..` rest, on exactly
`Agent`'s, `Listed`'s, and `AgentSnapshot`'s terms.

#### Scenario: An agent in the repository with no matching name is counted, never assigned

- **WHEN** `attribute` is called with one agent whose `cwd` is `/repo`, whose `name` is
  `Some("scratch-work")`, and whose `status` is `Working`; `repo` `Some("/repo")`;
  `change_names` `["add-auth", "fix-basket"]`; and an empty mapping
- **THEN** `badges` is empty — neither `add-auth` nor `fix-basket` gains an entry
- **AND** `unattributed` is exactly `1`
- **AND** the same call with `change_names` reordered to `["fix-basket", "add-auth"]` returns an
  equal `Attribution`, so nothing was assigned by position

#### Scenario: The name tier reads `name` and never the agent kind

- **WHEN** `attribute` is called with one agent whose `cwd` is `/repo`, whose `kind` is
  `Some("claude")`, whose `name` is `None`, and whose `status` is `Idle`; `repo` `Some("/repo")`;
  `change_names` `["claude"]` — a change whose name is exactly the agent kind; and an empty
  mapping
- **THEN** `badges` is empty and `unattributed` is `1`
- **AND** the same call with the agent's `name` set to `Some("claude")` returns `badges` holding
  exactly `{"claude": Idle}` and `unattributed` `0`, so the two fields are distinguished rather
  than conflated

#### Scenario: A terminal title naming a change attributes nothing

- **WHEN** `attribute` is called with one agent whose `cwd` is `/repo`, whose `name` is `None`,
  and whose `terminal_title` is `Some("✳ add-auth: wiring the token refresh")`; `repo`
  `Some("/repo")`; `change_names` `["add-auth"]`; and an empty mapping
- **THEN** `badges` is empty and `unattributed` is `1`
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
- **AND** call 1 returns `badges` empty and `unattributed` `0`: there is no agent to place
- **AND** call 2 returns `badges` empty and `unattributed` **`1`** — an in-scope agent that no
  tier could place is **counted**, and a repository with no changes at all does not make it
  vanish. An implementation that skipped the count whenever `change_names` was empty would pass
  a weaker form of this bullet and fail this one
- **AND** call 3 returns `badges` empty and `unattributed` `0`: with no repository there is no
  scope to attribute within
- **AND** call 4 returns `badges` empty and `unattributed` `0`, because an agent with no `cwd`
  is out of scope by the repository-scope requirement below
- **AND** call 5 returns `badges` holding exactly `{"alpha": Idle}` — one entry, not two — and
  `unattributed` `0`, so a duplicated change name attributes once

### Requirement: Every tier is scoped to the resolved repository

`herdr agent list` is **session-global**: measured against Herdr 0.8.2, it returned
byte-identical output from three different working directories and listed agents whose `cwd` lay
in a repository other than the current one. `SPEC.md` scopes only tier 3 to the repository;
every tier SHALL be scoped to it, or an agent renamed `add-auth` in an unrelated repository
would badge this repository's `add-auth` row.

An agent SHALL be **in scope** exactly when `repo` is `Some(root)` and the agent's `cwd` is
`Some(path)` and `path.starts_with(root)` — Rust's component-wise prefix test, which is true for
an equal path and false for a sibling whose textual prefix matches. An agent that is not in
scope SHALL be neither badged nor counted: it contributes nothing to `badges` and nothing to
`unattributed`, and is invisible to the pane.

An agent carrying no `cwd` SHALL be out of scope. `cwd` is not one of the seven fields Herdr's
own schema marks required, so its absence is an ordinary payload, not a fault; but an agent
whose working directory is unknown cannot be shown to be in this repository, and counting it
would be a guess.

Both sides of the comparison are canonical in production and SHALL NOT be canonicalized here:
`resolve::find_repo` canonicalizes the root it returns, and Herdr reports the operating
system's own working directory for the pane. `attribute` performs no filesystem I/O, so it
cannot canonicalize either side without breaking that.

#### Scenario: An agent in another repository is neither badged nor counted

- **WHEN** `attribute` is called with two agents, both `Working`: the first with `cwd`
  `/other/repo` and `name` `Some("add-auth")`, the second with `cwd` `/repo` and `name`
  `Some("nothing-like-a-change")`; `repo` `Some("/repo")`; `change_names` `["add-auth"]`; and an
  empty mapping
- **THEN** `badges` is empty — the out-of-scope agent's name matched a change and was still not
  attributed
- **AND** `unattributed` is exactly `1`, counting the in-scope agent alone and not the
  out-of-scope one
- **AND** the count therefore fails in both directions: it reads `0` if the poller supplied no
  agents and `2` if the scope test were removed

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

### Requirement: Tier 1 resolves an agent name through the plugin-local mapping

The plugin derives a Herdr-legal agent name from a change name and records the pair under
`HERDR_PLUGIN_STATE_DIR` whenever the derived name differs from the change name at all —
`plugin-state`'s contract, and the reason `2fa-support`, only thirteen characters, still becomes
`c-2fa-support` and still needs the record.

`attribute` SHALL therefore look an in-scope agent's `name` up in `mapping` first. When the
mapping holds that name **and** the change it names appears in `change_names`, the agent SHALL
be attributed to that change and the name tier SHALL NOT be consulted for it.

When the mapping holds the name but names a change that is **not** in `change_names` — a change
archived out of the visible window, renamed, or deleted since the launch — the agent SHALL fall
through to the name tier, and to the count if that fails too. A stale record SHALL NOT
strand an agent that the weaker tier can still place.

An unusable mapping file yields an empty or partial map (`plugin-state`), and `attribute` SHALL
be indifferent to how the map was obtained: with an empty map every agent is decided by the
name tier and the count alone.

#### Scenario: A derived name is resolved through the mapping

- **WHEN** `attribute` is called with one `Working` agent whose `cwd` is `/repo` and whose
  `name` is `Some("c-2fa-support")`; `repo` `Some("/repo")`; `change_names`
  `["2fa-support", "add-auth"]`; and a mapping holding `c-2fa-support -> 2fa-support`
- **THEN** `badges` holds exactly `{"2fa-support": Working}` and `unattributed` is `0`
- **AND** the identical call with an **empty** mapping returns `badges` empty and
  `unattributed` `1`, because `c-2fa-support` is not itself a change name — so the mapping is
  what did the work, not the name tier

#### Scenario: The mapping wins when both tiers could match

- **WHEN** `attribute` is called with one `Blocked` agent whose `cwd` is `/repo` and whose
  `name` is `Some("alpha")`; `repo` `Some("/repo")`; `change_names` `["alpha", "beta"]`; and a
  mapping holding `alpha -> beta`
- **THEN** `badges` holds exactly `{"beta": Blocked}` — the recorded launch, which is direct
  evidence, outranks the name coincidence
- **AND** `badges` holds no entry for `alpha` and `unattributed` is `0`

#### Scenario: A mapping naming a change that no longer exists falls through

- **WHEN** `attribute` is called with one `Idle` agent whose `cwd` is `/repo` and whose `name`
  is `Some("alpha")`; `repo` `Some("/repo")`; `change_names` `["alpha"]`; and a mapping holding
  `alpha -> long-since-archived`
- **THEN** `badges` holds exactly `{"alpha": Idle}`, the name tier having caught the fall
- **AND** the same call with the agent's `name` changed to `Some("c-gone")` and the mapping to
  `c-gone -> long-since-archived` returns `badges` empty and `unattributed` `1`, so the
  fall-through ends in the count rather than in an invented change

#### Scenario: An empty mapping leaves the name tier working

- **WHEN** `attribute` is called with an **empty** mapping — the value `plugin-state`'s
  `state::read` produces from an absent, empty, or malformed `agent-names.toml` alike —
  alongside one `Working` agent named `alpha` at `cwd` `/repo`, `repo` `Some("/repo")`, and
  `change_names` `["alpha"]`
- **THEN** `badges` holds exactly `{"alpha": Working}` and `unattributed` is `0`
- **AND** `attribute` returns no problem text of its own: `Attribution` carries no problem
  field, it takes a `BTreeMap` rather than a `Mapping`, and the file's own problems stay on
  `state::Mapping` where `plugin-state` put them. How the map was obtained is not this
  function's concern, and no test of it touches a filesystem — `dashboard-loop`'s
  "An unusable mapping file is an empty mapping with a named problem" is where the real
  `state::read` is driven

### Requirement: Tier 2 matches an agent name against a change name exactly

An in-scope agent whose `name` is `Some(n)`, which the mapping did not resolve, SHALL be
attributed to the change named exactly `n` when `change_names` contains it. Comparison SHALL be
byte equality: no case folding, no trimming, no prefix or substring match. `herdr agent rename`
is therefore a deliberate, exact way for a person to opt an agent in.

Both tiers of the change list SHALL be attributable — active changes and the archived changes
inside the configured window — because a change can be archived while an agent that was working
on it is still alive.

An agent whose `name` is `None` SHALL fall straight to the count. Herdr omits `name` entirely
when it is unset, which is the state of every agent nobody has renamed.

A change name that is not a legal Herdr agent name (`[a-z][a-z0-9_-]{0,31}`) is unreachable by
this tier, because no live agent can carry such a name; it is reachable through the mapping
tier alone. This is a consequence of Herdr's own validation, confirmed against Herdr 0.8.2,
which rejects an illegal name with `invalid_agent_name` before it does anything else.

#### Scenario: An active and an archived change are both attributable by name

- **WHEN** `attribute` is called with two `Working` agents at `cwd` `/repo` named `alpha` and
  `2026-08-14-legacy`; `repo` `Some("/repo")`; `change_names` `["alpha", "legacy"]` — the
  archived change's name with its date prefix already stripped, as `changes::from_files`
  produces it; and an empty mapping
- **THEN** `badges` holds exactly `{"alpha": Working}` and `unattributed` is `1`
- **AND** the same call with the second agent renamed to `legacy` returns `badges` holding both
  `{"alpha": Working, "legacy": Working}` and `unattributed` `0`, so the archived tier is
  attributable and the date prefix is not part of the name

#### Scenario: Matching is exact, not fuzzy

- **WHEN** `attribute` is called with `repo` `Some("/repo")`, an empty mapping, `change_names`
  `["add-auth"]`, and four `Idle` agents at `cwd` `/repo` named `Add-Auth`, `add-auth-2`,
  `add`, and ` add-auth`
- **THEN** `badges` is empty and `unattributed` is `4`
- **AND** adding a fifth agent named exactly `add-auth` makes `badges` hold
  `{"add-auth": Idle}` while `unattributed` stays `4`, so the exactness is the rule and not the
  name being unmatchable

#### Scenario: A change name past Herdr's cap is reachable only through the mapping

- **WHEN** `attribute` is called with `repo` `Some("/repo")`, `change_names`
  `["a-change-name-that-runs-well-past-thirty-two-characters"]` — 54 characters, which
  `plugin-state` derives to the 32-character `a-change-name-that-runs-pas-<hash>` form — and
  one `Working` agent at `cwd` `/repo` whose `name` is that derived 32-character name, first
  with an empty mapping and then with a mapping holding derived-name → change-name
- **THEN** the empty-mapping call returns `badges` empty and `unattributed` `1`: no live agent
  can carry a name Herdr would reject, so the name tier can never reach such a change
- **AND** the mapped call returns `badges` holding exactly that 54-character change name and
  `unattributed` `0`
- **AND** the mapped call's key is the **change** name, not the agent name, so the badge lands
  on the row `change-rows` draws

#### Scenario: An unnamed agent falls straight to the count

- **WHEN** `attribute` is called with one `Done` agent at `cwd` `/repo` whose `name` is `None`
  and whose `kind` is `Some("claude")`; `repo` `Some("/repo")`; `change_names` `["alpha"]`; and
  a mapping holding `alpha -> alpha`
- **THEN** `badges` is empty and `unattributed` is `1`
- **AND** neither the mapping nor the change list was consulted with a `None` name, so no
  entry was invented for a nameless agent

### Requirement: A change's badge is the highest-precedence status among its agents

Several agents can attribute to one change — a launched agent plus a manually renamed one, or
two panes on the same work. The badge SHALL be a single `AgentStatus`, chosen by the fixed
precedence `Blocked` > `Working` > `Idle` > `Done` > `Unknown`, highest first, and SHALL NOT
depend on the order the agents arrived in the payload.

`Blocked` leads because it is the only status asking for a person. `Unknown` trails because it
carries no information: an unrecognised or absent `agent_status` decodes to it, and a change
with one `Unknown` agent and one `Working` agent is being worked on.

#### Scenario: Precedence is total and order-independent

- **WHEN** `attribute` is called with `repo` `Some("/repo")`, an empty mapping, `change_names`
  `["alpha"]`, and five agents all at `cwd` `/repo` and all named `alpha`, carrying `Unknown`,
  `Done`, `Idle`, `Working`, and `Blocked` respectively
- **THEN** `badges` holds exactly `{"alpha": Blocked}`
- **AND** the same five agents in reverse order return an equal `Attribution`
- **AND** removing them one at a time from the front of the precedence — `Blocked`, then
  `Working`, then `Idle`, then `Done` — yields `Working`, then `Idle`, then `Done`, then
  `Unknown`, so every rank of the order is exercised rather than only its top

#### Scenario: One agent per change carries its own status unchanged

- **WHEN** `attribute` is called with `repo` `Some("/repo")`, an empty mapping, `change_names`
  `["a", "b", "c", "d", "e"]`, and five agents at `cwd` `/repo` named `a`, `b`, `c`, `d`, and
  `e`, carrying `Working`, `Idle`, `Blocked`, `Done`, and `Unknown` respectively
- **THEN** `badges` holds exactly those five names mapped to those five statuses
- **AND** `unattributed` is `0`

### Requirement: Attribution is derived per frame, keyed by name, and stored nowhere

`ui::app::Dashboard::attribution(&self) -> agents::Attribution` SHALL derive the attribution
from state the dashboard already carries — `repo`, `changes`, `agents.agents`, and
`agent_names` — and SHALL be a pure function of `&self`, beside `visible()`, `visible_len()`,
and `selected_change()`, which are derived on every call for the same reason.

The result SHALL NOT be stored on `Dashboard`, and a badge SHALL NOT be attached to a change by
index. `Dashboard::adopt` preserves the selection by the selected change's **name** because a
refresh can reorder the list; a badge attached by index would drift on exactly that refresh.
Keying `badges` by change name makes the drift unrepresentable rather than merely avoided.

`attribution()` SHALL be unaffected by the `/` filter: it is built from `changes.active` and
`changes.archived` in full, so filtering the visible list hides badged rows without changing
any other row's badge and without changing the count.

`Attribution` SHALL carry no problem text and SHALL produce none. An unreachable Herdr socket
yields an empty `AgentSnapshot::agents`, hence no badges and a count of zero, which is the
silent standalone-TUI state `SPEC.md` → Degraded states requires — never a `!`-marked problem
row, and never an entry on `ChangeSet::problems`, which `adopt` replaces wholesale.

#### Scenario: A refresh that reorders the list moves the badge with its change

- **WHEN** a `Dashboard` whose `changes.active` is `beta` then `gamma`, whose `agents.agents`
  holds one `Working` agent at the repository root named `gamma`, and whose selection is on
  `gamma`, adopts a new `ChangeSet` whose `active` is `alpha`, `beta`, `gamma` — the same
  changes with one inserted above them
- **THEN** `attribution().badges` still holds exactly `{"gamma": Working}` after the adopt
- **AND** `selected_change()` is still `gamma`, so the selection and the badge agree because
  both are resolved by name
- **AND** `alpha` and `beta` carry no badge, so the inserted row did not inherit one by index

#### Scenario: An unreachable socket yields no badge, no count, and no problem

- **WHEN** a `Dashboard` carrying two active changes and an `AgentSnapshot` whose `reachable` is
  false, whose `agents` is empty, and whose `problem` names an unreachable socket, is asked for
  `attribution()`
- **THEN** `badges` is empty and `unattributed` is `0`
- **AND** `changes.problems` and `refresh.problems` are both still empty, so nothing about the
  socket reached either problem list
- **AND** the rendered frame at 120x20 and at 60x20 is byte-identical to the frame the same
  dashboard produces with `agents.problem` set to `None`, so the recorded reason is carried and
  never drawn

#### Scenario: The `/` filter hides rows without changing the count

- **WHEN** a `Dashboard` with active changes `alpha` and `beta`, one in-scope agent named
  `alpha` (`Working`) and two in-scope agents matching nothing, is asked for `attribution()`,
  and then asked again with `filter.query` set to `beta`
- **THEN** both calls return an equal `Attribution` — `badges` `{"alpha": Working}` and
  `unattributed` `2`
- **AND** the rendered 120x20 frame under the `beta` query shows no `alpha` row and still shows
  `2 unattributed` in the footer
