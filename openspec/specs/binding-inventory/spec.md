# binding-inventory Specification

## Purpose
The pane's keys and mouse gestures are written down **once**, as data, in
`ui::help::INVENTORY` — one `Binding` row per key or gesture, grouped by the route it acts
at — and that data is bound to the functions that actually produce the actions by
**executing** them, never by parsing their source.

This capability exists because a keybinding otherwise lives in four places that drift: the
`match` arm in `ui::app::action_for` or `ui::driver::mouse_action`, the help the reader
sees, `SPEC.md` → Keys, and `README.md` → Keys. `tests/doc_contract.rs` sweeps
`action_for` over every printable key and thirteen named codes at four modifier sets and
both filter modes, and `mouse_action` over every `MouseEventKind` at every cell of both
mandated frames, and compares what comes back against the inventory in **both**
directions: a key bound in the driver with no row fails, and a row no key reaches fails.
Each row's `input` is parsed back into a `(KeyCode, KeyModifiers)` pair and executed, so
an inventory naming the wrong key for every row cannot satisfy the action-set check.

Two actions are exempt, pinned by name and by count: `FilterPush`, which every printable
key produces while filtering and which no single row could name, and `Ignore`. A third
exemption costs a spec change.

## Requirements

### Requirement: The bindings are data, in one module, named once

`ui::help` SHALL be a new module under `src/ui/` holding the pane's bindings as a
`'static` value and the overlay that renders it, and nothing else. It SHALL be a pure
total module on exactly `ui::palette`'s terms: no filesystem, process, environment,
network, or standard-I/O API, no clock, no global mutable state, and no panic for any
input.

```rust
pub enum Scope { Any, List, Detail, Filter }

pub struct Binding {
    pub input: &'static str,
    pub description: &'static str,
    pub action: crate::ui::app::Action,
}

pub struct Group {
    pub title: &'static str,
    pub scope: Scope,
    pub bindings: &'static [Binding],
}

pub const INVENTORY: &[Group];
```

`input` is the key or gesture as the reader presses it — `j / ↓`, `Ctrl-C`, `1`–`9`,
`Wheel`, `Click`. `action` is the `Action` that input produces. **Both halves are
machine-bound**, by two different checks, and the reason the second one exists is worth
stating: an action-set comparison alone would pass against an `INVENTORY` that read
`k  scroll down` or `Ctrl-C  refresh`. Every action would still be named exactly once, the
group counts would still be right, and the overlay would still be **wrong about which key does
what** — which is the only thing a reader opens it for. A check that binds the goal's
bookkeeping and not the goal is the shape this repository calls an unfalsifiable guard.

`scope` SHALL be the route the bindings apply at, and SHALL sit on the **group**, not on
the binding. It is not decoration and SHALL NOT be collapsed away: `Space` folds a
**list** section at `Route::List` and a **content** section at `Route::Detail`, and `Esc`
leaves the detail route at `Route::Detail` and clears the query while filtering. An
inventory that flattens either is worse than none. Putting it on the group is what makes
that disambiguation cost one row in each of two groups rather than a qualifier repeated
on all thirty-one — within `Changes` every binding is a list binding, so a per-row tag
would say `list` five times and carry no information the heading does not. `Any` means
the group's bindings do the same thing at both routes — `Agents`, `Pane`, and `Mouse`,
whose gestures name their own region in their descriptions instead.

`Binding` and `Group` SHALL implement no `Default`, anywhere in the crate, and SHALL
join `NODEFAULT-UI`'s scanned type sets on exactly the terms `Filter`, `Refresh`, and
`Launch` are on them: every construction site names every field, with no `..` rest, so
a field added later is a compile error at each entry rather than a silent `""`.

#### Scenario: The inventory is const-evaluable, proved by a const item

- **WHEN** the test module declares `const _: &[Group] = ui::help::INVENTORY;`
- **THEN** it compiles, which is the observable second site: a const item is evaluated at
  compile time, so an `INVENTORY` that became a function call, allocated, or read a file would
  fail to compile here rather than pass a runtime assertion that has nothing to observe
- **AND** the purity greps over `src/ui/help.rs` are **not** asserted here: they are
  `dashboard-loop`'s `NOIO-VIEW` leg and `view-palette`'s `PALETTE` leg, both of which name the
  file in their own `PURE` lists, and restating them in this scenario would file a gate under
  the contract tier and count one guard twice

#### Scenario: Every binding names a field explicitly

- **WHEN** `scripts/gates/nodefault-ui.sh` is run over `src/ui/help.rs`'s type set
- **THEN** it reports at least one construction site for `Binding` and at least one for
  `Group`, and neither type implements `Default` anywhere in the crate
- **AND** the gate exits non-zero against a copy of the tree in which a `Binding`
  literal is written with a `..Default::default()` rest

### Requirement: Every key row's `input` is executed against the driver

The action-set check above binds *which actions exist*. This one binds *which key produces
which action*, and without it the inventory could name the wrong key for every row and stay
green.

For every binding in a group whose `scope` is not `Any`-with-a-mouse-gesture — that is, every
binding whose `input` names a **key** rather than `Wheel` or `Click` — the check SHALL:

1. parse `input` into one or more `(KeyCode, KeyModifiers)` pairs, so `j / ↓` yields
   `Char('j')` with `NONE` and `Down` with `NONE`, `1`–`9` yields all nine digits, and
   `Ctrl-C` yields `Char('c')` with `CONTROL`;
2. derive the filter mode from the group's `scope`: `Filter` means `filtering` true, every
   other scope means false;
3. assert `action_name(action_for(press(code, mods), filtering)) == action_name(binding.action)`
   for **every** pair parsed — compared by the **variant name** `action_name` already returns
   for the action-set check above, not by value.

The comparison is by name because one `Binding` carries one `action` and a row may parse to
several pairs whose actions differ in their **payload**. `1`–`9` is the case that forces it:
`action_for` maps `Char(c @ '1'..='9')` to `Action::SelectTab((c - b'1') as usize)`
(`src/ui/app.rs:1350`), so the nine digits yield `SelectTab(0)` through `SelectTab(8)` and no
single `binding.action` can equal all nine. A value comparison would fail eight of the nine by
construction, and the only ways to make it pass would be to shrink the row to the digit `1`
alone — leaving eight keys undocumented — or to split group 2 into fifteen bindings, which
contradicts the fixed count of seven this spec mandates below.

Comparing names costs this check nothing it was relying on. What it exists to catch is a row
naming the **wrong key** — `k  scroll down`, `Ctrl-C  refresh` — and those fail on the name
alone, since `Prev` and `Refresh` are different names. A row that named the right key and the
wrong tab **index** is not a failure mode the inventory can have: the row's `input` is the
range `1`–`9` and its description says which tab each digit selects, so there is no index in
the data to get wrong.

The parse SHALL be **total over the inventory**: the check SHALL assert that every non-mouse
`input` parsed to at least one pair, and SHALL fail naming the row when one did not. A spelling
the parser does not recognise must fail loudly rather than be skipped — a silently skipped row
is the same unfalsifiable guard this requirement exists to remove, one level down.

The parser SHALL be small and SHALL NOT become a second key table: it recognises a single
character, a `X / Y` pair, a `Ctrl-<c>` form, a `<a>`–`<b>` digit range, and the named keys
`Enter`, `Esc`, `Space`, `Backspace`, `↑`, and `↓`. Anything else is an error, not a guess.

#### Scenario: A row naming the wrong key fails

- **WHEN** the `Binding` whose `input` is `r` has its `input` changed to `k` while its `action`
  stays `Action::Refresh`
- **THEN** the check fails naming the row, the key it claims, and the action `action_for`
  actually returns for that key — `Prev`, not `Refresh`; the two differ by **name**, which is
  what the per-row comparison reads, so this failure does not depend on any payload
- **AND** the action-set check above still **passes** on that same tree, since `Refresh` is
  still named exactly once, which is precisely why this second check exists

#### Scenario: Every non-mouse row parses and agrees at HEAD

- **WHEN** the check is run against the tree at the end of this change
- **THEN** every binding in the `Changes`, `Artifact`, `Agents`, `Pane`, and `While filtering`
  groups parses to at least one `(KeyCode, KeyModifiers)` pair
- **AND** every parsed pair, evaluated under its group's filter mode, returns exactly that
  binding's `action`
- **AND** the `While filtering` group's five rows are evaluated with `filtering` **true**, which
  is what makes `Esc` → `Back`, `Backspace` → `FilterPop`, `Enter` → `OpenDetail`, `↑` → `Prev`,
  and `↓` → `Next` the assertions rather than the `filtering` false table's answers

#### Scenario: An unparseable spelling fails rather than skipping

- **WHEN** a binding's `input` is changed to `the any key`
- **THEN** the check fails naming that row as unparseable
- **AND** it does not pass by treating an unrecognised spelling as a mouse row or as zero pairs

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
