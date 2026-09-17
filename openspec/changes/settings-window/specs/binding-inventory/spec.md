## MODIFIED Requirements

### Requirement: The inventory's groups and order are fixed and readable

`INVENTORY` SHALL hold exactly **seven** groups, in this order, with these titles and
these binding counts:

| # | Title | `scope` | Bindings | What it covers |
|---|---|---|---|---|
| 1 | `Changes` | `List` | 5 | the list region at `Route::List` |
| 2 | `Artifact` | `Detail` | 7 | the detail region at `Route::Detail` |
| 3 | `Agents` | `Any` | 4 | the four keys that reach Herdr |
| 4 | `Pane` | `Any` | 5 | the keys that name no region |
| 5 | `While filtering` | `Filter` | 5 | the keys that keep a command meaning inside `/` |
| 6 | `Mouse` | `Any` | 7 | the gestures `ui::driver::mouse_action` produces |
| 7 | `While settings is open` | `Settings` | 4 | the keys the settings panel reinterprets |

**Thirty-seven** bindings in total, and the three counts that are not free are groups 5, 6,
and 7.

Group 4 SHALL hold **five**, not four, because `settings-window` adds `,` → `ToggleSettings`
to the keys that name no region, beside `q`, `Ctrl-C`, `r`, and `?`.

Group 7 SHALL hold **four** and is `settings-window`'s addition: `Enter` → `OpenDetail`,
`Esc` → `Back`, `j / ↓` → `Next`, and `k / ↑` → `Prev`. Every one of its four actions is
already named by an earlier group, which is exactly why the group is needed and exactly why
it cannot be caught by the action-set check below: a set comparison cannot see a key whose
*meaning changes* while its action stays the same. `Enter` opens a change at group 1 and
begins or commits an edit here; `j` moves the list selection there and moves the row cursor
or steps the candidate here. A reader who opens the help from inside the settings panel and
finds only "Open the selected change" has been told something false about the key in front of
them.

`Scope` SHALL therefore gain a fifth variant, `Settings`, whose group-heading suffix is
`settings panel`, beside `Any`, `List`, `Detail`, and `Filter`. It is a scope and not a
seventh `Any` group for the same reason `Filter` is one: the bindings act this way only while
that mode is on, and the heading is where the reader learns the condition.

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
- **THEN** it holds seven groups whose titles, in order, are `Changes`, `Artifact`,
  `Agents`, `Pane`, `While filtering`, `Mouse`, and `While settings is open`
- **AND** their `scope` values, in order, are `List`, `Detail`, `Any`, `Any`, `Filter`,
  `Any`, and `Settings`
- **AND** their binding counts, in order, are 5, 7, 4, 5, 5, 7, and 4, summing to 37
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

#### Scenario: The settings group is present and names the reinterpreted keys

- **WHEN** `INVENTORY` is inspected at the end of this change
- **THEN** it holds seven groups, the seventh titled `While settings is open` with scope
  `Scope::Settings` and exactly four bindings
- **AND** their actions are `OpenDetail`, `Back`, `Next`, and `Prev`, and each description
  names what the key does **in the panel** rather than what it does at a route
- **AND** the `Pane` group holds five bindings, the fifth `,` → `ToggleSettings`
- **AND** the seven group titles and their order are asserted by equality against a literal
  list, so a group inserted or reordered fails rather than passing on a count alone

#### Scenario: The settings group renders at both mandated widths

- **WHEN** the help panel is rendered at 120x40 and at 60x20
- **THEN** both frames hold the `While settings is open` heading with its `settings panel`
  suffix, and all four of its rows
- **AND** no row exceeds the band's width at either, measured by `ui::layout::columns`

## REMOVED Requirements

### Requirement: The inventory names every action the pane binds, and the sweep's totals live in its body

**Reason**: This change takes the driver sweep from two overlay states to three — `None`,
`Some(Panel::Help)`, and `Some(Panel::Settings)` — which renames the scenario "The sweep covers
the mouse under both overlay states". A MODIFIED block may not rename a scenario, and a
same-named REMOVED/ADDED pair is rejected, so the requirement is replaced under a header that
names what it now covers.

**Migration**: Replaced by the ADDED requirement below, carrying every sentence and every
scenario forward with three edits: the swept union moves from twenty-five names to twenty-six
and the compared set from twenty-three to twenty-four, step 3 sweeps three overlay states
instead of two, and the dismissal action the sweep observes becomes `Back` rather than
`ToggleHelp`. The two-name exemption set is unchanged.

## ADDED Requirements

### Requirement: The inventory names every action the pane binds, and the sweep's totals and overlay states live in its body

A test in `tests/doc_contract.rs` SHALL **derive** the set of actions the pane binds by
executing `ui::app::action_for` and `ui::driver::mouse_action`, and SHALL require
`INVENTORY` to name exactly that set. It SHALL NOT read the source text of either
function: `action_for` is a pure total function of an `Event` and a `bool` and
`mouse_action` is a pure total function of a `Dashboard`, a `Rect`, and a `MouseEvent`,
so the set each produces is **computable by calling it**, and a derivation that calls
the function cannot disagree with the function the way a derivation that parses it can.
`SPEC.md`'s mouse table is bound the same executed way (`doc-conformance` ->
"`SPEC.md`'s mouse table is bound by executing `mouse_action`, row by row"). The contrast
that stood here is **superseded**: it read "one layer stronger: that check parses a markdown
table and compares it against parsed source, and this one compares a `'static` value against
a swept function", which was true only while the mouse table was bound by parsing. It is not
a layer weaker now, and the two are no longer distinguished by that. What still separates
them is **subject**, not strength: this requirement compares a `'static` value against the
set of action **names** the sweep produces, while the mouse table's own check compares each
documented **row** against the gesture, overlay state, zone and outcome the sweep observed.
A binding missing from `INVENTORY` fails here; a row of `SPEC.md` describing a binding the
pane does not have fails there.

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
   foldable artifact — once with `overlay.panel` `None`, once with `Some(Panel::Help)`,
   and once with `Some(Panel::Settings)`. Every returned action's name is collected.
   `settings-window` took this from two overlay states to **three**, and the third is not
   redundant: it is the only state in which `mouse_action` produces
   `Click(Target::Setting(_))`.
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
- **THEN** the swept union holds exactly twenty-six action names — the full `Action`
  membership after `ToggleSettings` is added
- **AND** `FilterPush` and `Ignore` are the only two removed by the exemption set, and
  the exemption set is asserted to hold exactly those two names and to have length two
- **AND** the remaining twenty-four equal the set `INVENTORY` names, so every action the
  pane can take from a key or a gesture has a row the reader can find

#### Scenario: The sweep covers the mouse under all three overlay states

- **WHEN** step 3's sweep is run with `overlay.panel` `None`, then `Some(Panel::Help)`, then
  `Some(Panel::Settings)`
- **THEN** the first run yields **eight** names — `SelectNext`, `SelectPrev`, `ScrollDown`,
  `ScrollUp`, `SelectTab`, `Click`, `Select`, and `Ignore`; the second yields `ScrollDown`,
  `ScrollUp`, `Back`, and `Ignore`; and the third yields `ScrollDown`, `ScrollUp`, `Back`,
  `Click`, and `Ignore`
- **AND** `SelectTab` is in the first set because `mouse_action` maps `Zone::DetailTab` through
  `detail::tab_at` to `Action::SelectTab` (`src/ui/driver.rs:235-245`), which is reachable only
  when the swept dashboard's selected change carries **several artifact tabs**. The fixture
  step 3 mandates therefore is not incidental: a dashboard whose changes carry no artifacts
  yields six names and passes an equality written against six, while silently removing the
  mouse's tab-switching and detail-header coverage from this whole check. A future session
  that finds this assertion red must widen the fixture, never narrow the expected set
- **AND** the second and third runs name `Back` and **not** `ToggleHelp`, which pins
  `mouse-input`'s one correction: with two panels sharing a layer, a click outside the band
  dismisses with `Back`, where `ToggleHelp` would have swapped panels instead of closing
- **AND** `ToggleHelp` is still in the overall union, produced by `?` rather than by any
  gesture, so no mouse row is owed for it
- **AND** the third run is the only one naming `Click` alongside `Back`, which is the setting
  row's own selection gesture

#### Scenario: `Action::Select` has a `Mouse` row and no exemption

- **WHEN** the sweep is run against the tree at the end of this change
- **THEN** `Action::Select` appears in the swept union and in the set `INVENTORY` names
- **AND** `EXEMPT_ACTIONS` still holds exactly `FilterPush` and `Ignore` and still has length
  two, so the drag gesture is documented rather than exempted

#### Scenario: `ToggleSettings` has a `Pane` row and no exemption

- **WHEN** the sweep is run against the tree at the end of this change
- **THEN** `ToggleSettings` appears in the swept union and in the set `INVENTORY` names
- **AND** `EXEMPT_ACTIONS` still holds exactly `FilterPush` and `Ignore` and still has length
  two, so the new action is documented rather than exempted
- **AND** deleting the `,` row from `INVENTORY` fails the test with a message naming
  `ToggleSettings` as bound but not documented
