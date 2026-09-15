## REMOVED Requirements

### Requirement: The inventory names every action the pane binds

**Reason**: `text-selection` adds `Action::Select`, and this requirement's sweep scenario
carried the bound-action count **in its own name**. A scenario header is a merge key exactly
as a requirement header is — `openspec validate --strict` refuses a MODIFIED block that
renames one — so it cannot be edited in place. It is re-stated below under a header that says
where the totals live, with the numeral removed from the scenario name, so that the next
change adding an action pays a MODIFIED rather than another REMOVED+ADDED. This is the second
time a count in a name has forced this; the first was `Dashboard`'s field count.

**Migration**: None. The sweep and the exemption set are unchanged in kind; only the two
totals move, from twenty-four swept and twenty-two bound to twenty-five and twenty-three.

## ADDED Requirements

### Requirement: The inventory names every action the pane binds, and the sweep's totals live in its body

A test in `tests/doc_contract.rs` SHALL **derive** the set of actions the pane binds by
executing `ui::app::action_for` and `ui::driver::mouse_action`, and SHALL require
`INVENTORY` to name exactly that set. It SHALL NOT read the source text of either
function: `action_for` is a pure total function of an `Event` and a `bool` and
`mouse_action` is a pure total function of a `Dashboard`, a `Rect`, and a `MouseEvent`,
so the set each produces is **computable by calling it**, and a derivation that calls
the function cannot disagree with the function the way a derivation that parses it can.
This is what `SPEC.md`'s mouse table is bound by today
(`tests/doc_contract.rs` -> `documented_mouse_actions`), one layer stronger: that check
parses a markdown table and compares it against parsed source, and this one compares a
`'static` value against a swept function.

The derivation SHALL be:

1. `fn action_name(action: Action) -> &'static str` maps each `Action` to a stable
   name by an **exhaustive** `match` with no wildcard arm, so a variant added later is a
   compile error in this test before it is a missing help row.
2. `action_for` is called for every `KeyCode::Char(c)` over the printable ASCII range
   `' '..='~'`, for `Backspace`, `Enter`, `Esc`, `Up`, `Down`, `Left`, `Right`, `Tab`,
   `Home`, `End`, `PageUp`, `PageDown`, and `Delete`, each under `KeyModifiers::NONE`,
   `SHIFT`, `CONTROL`, and `ALT`, and each under both values of `filtering`. Every
   returned action's name is collected.
3. `mouse_action` is called for every `MouseEventKind` the crate can receive, at every
   cell of a 120x40 frame and of a 60x20 frame, against a dashboard carrying active
   changes, archived changes, a selected change with several artifact tabs, and a
   foldable artifact — once with `help.open` false and once with it true. Every
   returned action's name is collected.
4. The union of the two sets, less the **closed** exemption set below, SHALL equal the
   set of `action_name(binding.action)` over every binding in `INVENTORY`.

The exemption set SHALL be exactly **two** names, asserted by name in the test rather
than by a predicate: `FilterPush`, which is typing rather than a binding — the
inventory says so in prose in its own group — and `Ignore`, which is the absence of a
binding. A third exemption SHALL require a spec change, so the list cannot quietly
absorb an action nobody documented.

The check SHALL fail in **both** directions: an action the pane produces and the
inventory does not name, and a name in the inventory no swept call produces. The second
direction is what catches a binding that was removed from `action_for` and left in the
help.

#### Scenario: An action added without a help row fails `cargo test`

- **WHEN** a new `Action` variant is added to `ui::app::Action` and bound to a key in
  `action_for`, and no `Binding` naming it is added to `INVENTORY`
- **THEN** `tests/doc_contract.rs` fails to compile at `action_name`'s exhaustive
  `match`, before any assertion runs
- **AND** when `action_name` is extended to name the variant but `INVENTORY` still does
  not, the test fails with a message naming the action and saying it is bound but not
  documented

#### Scenario: A binding removed from the driver and left in the help fails

- **WHEN** `INVENTORY` carries a `Binding` whose `action` is `Action::Refresh` and
  `action_for`'s `Char('r')` arm is deleted
- **THEN** the test fails with a message naming `Refresh` as documented but unreachable
- **AND** the message names both sides — the inventory row and the swept function — so
  the reader is not left to work out which of the two moved

#### Scenario: The sweep's totals and the exemption set are pinned

- **WHEN** the derivation above is run against the tree at the end of this change
- **THEN** the swept union holds exactly twenty-five action names — the full `Action`
  membership after `Select` is added
- **AND** `FilterPush` and `Ignore` are the only two removed by the exemption set, and
  the exemption set is asserted to hold exactly those two names and to have length two
- **AND** the remaining twenty-three equal the set `INVENTORY` names, so every action the
  pane can take from a key or a gesture has a row the reader can find

#### Scenario: The sweep covers the mouse under both overlay states

- **WHEN** step 3's sweep is run with `help.open` false and then with it true
- **THEN** the first run yields **seven** names — `SelectNext`, `SelectPrev`, `ScrollDown`,
  `ScrollUp`, `SelectTab`, `Click`, and `Ignore` — and the second yields `ScrollDown`,
  `ScrollUp`, `ToggleHelp`, and `Ignore`
- **AND** `SelectTab` is in the first set because `mouse_action` maps `Zone::DetailTab` through
  `detail::tab_at` to `Action::SelectTab` (`src/ui/driver.rs:235-245`), which is reachable only
  when the swept dashboard's selected change carries **several artifact tabs**. The fixture
  step 3 mandates therefore is not incidental: a dashboard whose changes carry no artifacts
  yields six names and passes an equality written against six, while silently removing the
  mouse's tab-switching and detail-header coverage from this whole check. A future session
  that finds this assertion red must widen the fixture, never narrow the expected set
- **AND** the union names `ToggleHelp`, so the click-outside dismissal `mouse-input`
  adds is bound by this check and not only by its own scenarios


#### Scenario: `Action::Select` has a `Mouse` row and no exemption

- **WHEN** the sweep is run against the tree at the end of this change
- **THEN** `Action::Select` appears in the swept union and in the set `INVENTORY` names
- **AND** `EXEMPT_ACTIONS` still holds exactly `FilterPush` and `Ignore` and still has length
  two, so the drag gesture is documented rather than exempted

## MODIFIED Requirements

### Requirement: The inventory's groups and order are fixed and readable

`INVENTORY` SHALL hold exactly **six** groups, in this order, with these titles and
these binding counts:

| # | Title | `scope` | Bindings | What it covers |
|---|---|---|---|---|
| 1 | `Changes` | `List` | 5 | the list region at `Route::List` |
| 2 | `Artifact` | `Detail` | 7 | the detail region at `Route::Detail` |
| 3 | `Agents` | `Any` | 4 | the four keys that reach Herdr |
| 4 | `Pane` | `Any` | 4 | the keys that name no region |
| 5 | `While filtering` | `Filter` | 5 | the keys that keep a command meaning inside `/` |
| 6 | `Mouse` | `Any` | 7 | the gestures `ui::driver::mouse_action` produces |

**Thirty-two** bindings in total, and the two counts that are not free are groups 5 and 6.

Group 5 SHALL hold **five**, not three, because `action_for`'s `filtering` table has exactly
**six** non-typing rows (`src/ui/app.rs:1213-1225`): `Ctrl-C` → `Quit`, `Backspace` →
`FilterPop`, `Enter` → `OpenDetail`, `Esc` → `Back`, `Up` → `Prev`, and `Down` → `Next`.
`Ctrl-C` has its row in `Pane`; the other five belong here. Three rows would have left `Up` and
`Down` undocumented **and invisible to the action-set check**, because `Prev` and `Next` are
each already named by group 1 — a set comparison cannot see a key that is missing when its
action is spoken for elsewhere.

Group 6 SHALL hold **seven** after `text-selection` adds `Action::Select`, whose single
row describes the drag and the press counting as one gesture family. It held **six**
before, for the same reason one layer over: `mouse_action` produces six
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
- **AND** their binding counts, in order, are 5, 7, 4, 4, 5, and 7, summing to 32
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
