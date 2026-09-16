## MODIFIED Requirements

### Requirement: The inventory names every action the pane binds, and the sweep's totals live in its body

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
- **THEN** the first run yields **eight** names — `SelectNext`, `SelectPrev`, `ScrollDown`,
  `ScrollUp`, `SelectTab`, `Click`, `Select`, and `Ignore` — and the second yields `ScrollDown`,
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
