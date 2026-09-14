## MODIFIED Requirements

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
  `/beta  ? help  q quit  Enter detail  Esc back  a/c/s launch  g focus  2 unattributed` — 84
  columns, which fits 120 whole: the filter changes
  neither the count nor the availability of the action keys
- **AND** the **query leads that row**, because `render_footer` pushes `/<query>` onto the hint
  list before `FOOTER_HINTS` whenever the query is non-empty and the filter is not active. The
  landed spec omitted the prefix and claimed **69** for the same row, which is the unprefixed
  width; the prefixed one was **76**. This change corrects the omission rather than moving the
  wrong number by eight, under design.md → Decision 9's repair-in-passing rule — the arithmetic
  is `/beta` + the four key hints + the two action hints + the count, two spaces between each
- **AND** `g` pressed under the `beta` query does nothing, because `beta` carries no badge — the
  filter hides `alpha`'s row and hiding a row does not make its agent focusable from another
