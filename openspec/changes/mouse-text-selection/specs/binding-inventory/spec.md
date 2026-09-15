## MODIFIED Requirements

### Requirement: The inventory's groups and order are fixed and readable

`INVENTORY` SHALL hold exactly **six** groups, in this order, with these titles and
these binding counts:

| # | Title | `scope` | Bindings | What it covers |
|---|---|---|---|---|
| 1 | `Changes` | `List` | 5 | the list region at `Route::List` |
| 2 | `Artifact` | `Detail` | 7 | the detail region at `Route::Detail` |
| 3 | `Agents` | `Any` | 4 | the four keys that reach Herdr |
| 4 | `Pane` | `Any` | 5 | the keys that name no region |
| 5 | `While filtering` | `Filter` | 5 | the keys that keep a command meaning inside `/` |
| 6 | `Mouse` | `Any` | 6 | the gestures `ui::driver::mouse_action` produces |

**Thirty-two** bindings in total, and the two counts that are not free are groups 5 and 6.

Group 5 SHALL hold **five**, not three, because `action_for`'s `filtering` table has exactly
**six** non-typing rows (`src/ui/app.rs:1213-1225`): `Ctrl-C` → `Quit`, `Backspace` →
`FilterPop`, `Enter` → `OpenDetail`, `Esc` → `Back`, `Up` → `Prev`, and `Down` → `Next`.
`Ctrl-C` has its row in `Pane`; the other five belong here. Three rows would have left `Up` and
`Down` undocumented **and invisible to the action-set check**, because `Prev` and `Next` are
each already named by group 1 — a set comparison cannot see a key that is missing when its
action is spoken for elsewhere.

Group 6 SHALL hold **six** for the same reason one layer over: `mouse_action` produces six
non-`Ignore` actions with the overlay closed — `SelectNext`, `SelectPrev`, `ScrollDown`,
`ScrollUp`, `Click`, and `SelectTab` — and a `Binding` carries exactly one `action`, so five
rows cannot name six actions. The order SHALL be the order a reader meets the pane in
— the list first, the detail second, the agents and pane keys after, and the two modal
groups last — not alphabetical and not the order `action_for`'s `match` happens to be
written in, which is an implementation artefact.

Group 5 SHALL state in its own bindings' descriptions that every other printable key
types into the query, which is the one behaviour the action sweep cannot name because
`FilterPush` is exempt from it. Group 6 carries `Scope::Any` because its six gestures do
not share one region — a wheel over the list and a wheel over the detail are different
bindings sharing one input string — so each of its descriptions SHALL name the region
the gesture acts on instead.

`INVENTORY` SHALL name `q` and `Ctrl-C` as **two** bindings in group 4 sharing the
action `Quit`, because a reader looking for how to leave will look for one or the other
and a row naming only one is a row that failed for half of them. The action-set check
above compares **sets**, so two bindings naming one action is not a duplicate it can
object to.

#### Scenario: The inventory's shape is asserted, not described

- **WHEN** `INVENTORY` is read
- **THEN** it holds six groups whose titles, in order, are `Changes`, `Artifact`,
  `Agents`, `Pane`, `While filtering`, and `Mouse`
- **AND** their `scope` values, in order, are `List`, `Detail`, `Any`, `Any`, `Filter`,
  and `Any`
- **AND** their binding counts, in order, are 5, 7, 4, 5, 5, and 6, summing to 32
- **AND** no group is empty, and no two groups share a title

#### Scenario: `Space` and `Esc` each appear under their route

- **WHEN** every binding whose `input` is `Space` is collected with the `scope` of the
  group holding it, and then every binding whose `input` contains `Esc`
- **THEN** there are exactly two `Space` bindings, one under a `Scope::List` group
  describing a list section and one under a `Scope::Detail` group describing a content
  section, and their descriptions differ
- **AND** there are exactly two `Esc` bindings, one under a `Scope::Detail` group
  describing the return to the change list and one under a `Scope::Filter` group
  describing clearing the query, and their descriptions differ
- **AND** both pairs name `Action::ToggleSection` and `Action::Back` respectively, so the
  set check above counts each action once while the reader sees both routes

#### Scenario: Both quit keys have a row

- **WHEN** every binding whose `action` is `Action::Quit` is collected
- **THEN** there are exactly two, with `input` values `q` and `Ctrl-C`
- **AND** both sit in the `Pane` group, whose `scope` is `Any`, and the action-set check
  still reports `Quit` once

#### Scenario: `m` has a row in the `Pane` group

- **WHEN** every binding whose `action` is `Action::ToggleMouse` is collected from `INVENTORY`
- **THEN** there is exactly one, with `input` `m`, sitting in the `Pane` group whose `scope` is
  `Any`
- **AND** its description names both directions — that it releases capture so the terminal can
  select text, and that pressing it again restores capture — because a row naming only the
  release leaves the reader with no way back
- **AND** the action-set check still reports `ToggleMouse` exactly once, with no new entry in
  `EXEMPT_ACTIONS`, whose length assertion of two is unchanged by this change
