## MODIFIED Requirements

### Requirement: Attribution is a pure, total function that refuses to guess

`agents::attribute` SHALL map live agents onto changes, and SHALL be a pure total function of
its four arguments:

```rust
pub struct Attribution {
    pub badges: std::collections::BTreeMap<String, AgentStatus>,
    pub panes: std::collections::BTreeMap<String, String>,
    pub unattributed: usize,
}

pub fn attribute(
    agents: &[Agent],
    repo: Option<&std::path::Path>,
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

`attribute`'s signature SHALL NOT change: `panes` is derived from the `Agent` values already
passed in, and no fifth argument is added.

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

### Requirement: A change's badge is the highest-precedence status among its agents

Several agents can attribute to one change — a launched agent plus a manually renamed one, or
two panes on the same work. The badge SHALL be a single `AgentStatus`, chosen by the fixed
precedence `Blocked` > `Working` > `Idle` > `Done` > `Unknown`, highest first, and SHALL NOT
depend on the order the agents arrived in the payload.

`Blocked` leads because it is the only status asking for a person. `Unknown` trails because it
carries no information: an unrecognised or absent `agent_status` decodes to it, and a change
with one `Unknown` agent and one `Working` agent is being worked on.

`Attribution::panes` SHALL follow the same fold: the pane recorded for a change is the
`pane_id` of the agent whose status won, replaced whenever and only whenever the status is. Ties
SHALL be broken by keeping the **first** such agent in the slice's order, matching the strictly
greater comparison the fold already uses — so the choice is deterministic for a payload with
two `Working` agents on one change, and `g` focuses the same pane on every frame that payload
is polled.

#### Scenario: Precedence is total and order-independent

- **WHEN** `attribute` is called with `repo` `Some("/repo")`, an empty mapping, `change_names`
  `["alpha"]`, and five agents all at `cwd` `/repo` and all named `alpha`, carrying `Unknown`,
  `Done`, `Idle`, `Working`, and `Blocked` respectively, with `pane_id`s `w:p0` through `w:p4`
- **THEN** `badges` holds exactly `{"alpha": Blocked}` and `panes` holds exactly
  `{"alpha": "w:p4"}` — the pane of the `Blocked` agent, not of the first agent in the slice
- **AND** the same five agents in reverse order return an equal `Attribution`
- **AND** removing them one at a time from the front of the precedence — `Blocked`, then
  `Working`, then `Idle`, then `Done` — yields `Working`, then `Idle`, then `Done`, then
  `Unknown`, so every rank of the order is exercised rather than only its top, and `panes`
  follows each winner in turn

#### Scenario: A tie keeps the first agent's pane

- **WHEN** `attribute` is called with `repo` `Some("/repo")`, an empty mapping, `change_names`
  `["alpha"]`, and two agents at `cwd` `/repo`, both named `alpha`, both `Working`, with
  `pane_id`s `w:p1` and `w:p2` in that order
- **THEN** `badges` holds `{"alpha": Working}` and `panes` holds `{"alpha": "w:p1"}`
- **AND** the same two agents in the opposite order yield `{"alpha": "w:p2"}`, which is the
  documented consequence of a first-wins tie-break rather than an accident: the payload order is
  Herdr's and is stable across polls of an unchanged session

#### Scenario: One agent per change carries its own status unchanged

- **WHEN** `attribute` is called with `repo` `Some("/repo")`, an empty mapping, `change_names`
  `["a", "b", "c", "d", "e"]`, and five agents at `cwd` `/repo` named `a`, `b`, `c`, `d`, and
  `e`, carrying `Working`, `Idle`, `Blocked`, `Done`, and `Unknown` respectively, in panes
  `w:p1` through `w:p5`
- **THEN** `badges` holds exactly those five names mapped to those five statuses
- **AND** `panes` holds exactly those five names mapped to `w:p1` through `w:p5`
- **AND** `unattributed` is `0`

### Requirement: Attribution is derived per frame, keyed by name, and stored nowhere

`ui::app::Dashboard::attribution(&self) -> agents::Attribution` SHALL derive the attribution
from state the dashboard already carries — `repo`, `changes`, `agents.agents`, and
`agent_names` — and SHALL be a pure function of `&self`, beside `visible()`, `visible_len()`,
and `selected_change()`, which are derived on every call for the same reason.

The result SHALL NOT be stored on `Dashboard`, and a badge SHALL NOT be attached to a change by
index. `Dashboard::adopt` preserves the selection by the selected change's **name** because a
refresh can reorder the list; a badge attached by index would drift on exactly that refresh.
Keying `badges` by change name makes the drift unrepresentable rather than merely avoided. The
same holds for `panes` and therefore for `g`'s target: it is resolved by the selected change's
name at the moment the key is applied, from a map built in that same call, and is never a stored
pane id that a refresh could have staled.

`attribution()` SHALL be unaffected by the `/` filter: it is built from `changes.active` and
`changes.archived` in full, so filtering the visible list hides badged rows without changing
any other row's badge and without changing the count.

`Attribution` SHALL carry no problem text and SHALL produce none. An unreachable Herdr socket
yields an empty `AgentSnapshot::agents`, hence no badges, no panes, and a count of zero, which
is the silent standalone-TUI state `SPEC.md` → Degraded states requires — never a `!`-marked
problem row, and never an entry on `ChangeSet::problems`, which `adopt` replaces wholesale.
A **failed launch** is a different thing entirely and does produce a row; `agent-launch` puts it
on `Dashboard::launch.problems`, not here.

#### Scenario: A refresh that reorders the list moves the badge with its change

- **WHEN** a `Dashboard` whose `changes.active` is `beta` then `gamma`, whose `agents.agents`
  holds one `Working` agent at the repository root named `gamma` in pane `w8:p7`, and whose
  selection is on `gamma`, adopts a new `ChangeSet` whose `active` is `alpha`, `beta`, `gamma` —
  the same changes with one inserted above them
- **THEN** `attribution().badges` still holds exactly `{"gamma": Working}` after the adopt, and
  `attribution().panes` still holds exactly `{"gamma": "w8:p7"}`
- **AND** `selected_change()` is still `gamma`, so the selection, the badge, and `g`'s target
  agree because all three are resolved by name
- **AND** `alpha` and `beta` carry no badge and no pane, so the inserted row did not inherit
  either by index

#### Scenario: An unreachable socket yields no badge, no count, and no problem

- **WHEN** a `Dashboard` carrying two active changes and an `AgentSnapshot` whose `reachable` is
  false, whose `agents` is empty, and whose `problem` names an unreachable socket, is asked for
  `attribution()`
- **THEN** `badges` is empty, `panes` is empty, and `unattributed` is `0`
- **AND** `changes.problems`, `refresh.problems`, and `launch.problems` are all still empty, so
  nothing about the socket reached any problem list
- **AND** the rendered frame at 120x20 and at 60x20 is byte-identical to the frame the same
  dashboard produces with `agents.problem` set to `None`, so the recorded reason is carried and
  never drawn

#### Scenario: The `/` filter hides rows without changing the count

- **WHEN** a `Dashboard` with active changes `alpha` and `beta`, one in-scope agent named
  `alpha` (`Working`) in pane `w:p1` and two in-scope agents matching nothing, and
  `agents.reachable` `false`, is asked for `attribution()`,
  and then asked again with `filter.query` set to `beta`
- **THEN** both calls return an equal `Attribution` — `badges` `{"alpha": Working}`, `panes`
  `{"alpha": "w:p1"}`, and `unattributed` `2`
- **AND** the rendered 120x20 frame under the `beta` query shows no `alpha` row and still shows
  `2 unattributed` in the footer
- **AND** with `agents.reachable` set to `true` the two `attribution()` results are still equal
  to each other and to the pair above, and the 120x20 footer reads
  `q quit  Enter detail  Esc back  a/c/s launch  g focus  2 unattributed`: the filter changes
  neither the count nor the availability of the action keys
- **AND** `g` pressed under the `beta` query does nothing, because `beta` carries no badge — the
  filter hides `alpha`'s row and hiding a row does not make its agent focusable from another
