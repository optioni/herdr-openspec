## MODIFIED Requirements

### Requirement: `?` opens and closes a read-only help overlay

The pane SHALL bind `?` to `Action::ToggleHelp`, which opens the help overlay when it is
closed and closes it when it is open. The overlay SHALL be **read-only**: it renders,
and no key pressed while it is open changes anything the pane shows underneath it, reaches
a collaborator, spawns a process, touches the filesystem, or reads a clock.

The overlay SHALL be a **layer**, not a route. `Dashboard::route` SHALL remain an enum of
exactly `List` and `Detail`, and opening the overlay SHALL NOT change it.

`settings-window` gives that layer a **second panel**, and the help overlay becomes one of
two rather than the only one. The layer's state is `overlay: Overlay` carrying
`panel: Option<Panel>`, so what this requirement called `help.open` being true is
`overlay.panel` being `Some(Panel::Help)`; nothing else about it moves. `ToggleSettings`
pressed while the help panel is open SHALL swap the panel to `Settings` and reset
`overlay.scroll` to `0`, and `ToggleHelp` pressed while the settings panel is open SHALL swap
it back on the same terms. The two SHALL NOT stack: there is no representable state in which
both are open. The precedent is
`filter.active`, which is likewise a mode that changes what keys mean without being a
route, and the reason is the same: closing the overlay must return the reader to the
route they were on, which a third `Route` variant would have to remember separately.

`?` SHALL be accepted with `KeyModifiers::NONE` **and** with `KeyModifiers::SHIFT`. On a
US layout `?` is `Shift`+`/`, and terminals disagree about whether the shift modifier is
reported alongside the shifted character; accepting only `NONE` makes the key work on
some terminals and not others, which is the worst of the three outcomes. This is the same
pair `list-filtering`'s printable-character arm already matches on.

While `filter.active` is set, `?` SHALL type into the query like every other printable
character and SHALL NOT open the overlay. `list-filtering`'s rule is that only `Ctrl-C`
keeps a command meaning inside the filter, and `?` is not carved out of it: a reader
filtering for a change whose name contains `?` must be able to type it, and the help is
one `Esc` away.

#### Scenario: `?` toggles the overlay and its near misses do not

- **WHEN** `action_for` is called with `filtering` false and Presses of `Char('?')` with
  `KeyModifiers::NONE`, `Char('?')` with `SHIFT`, `Char('?')` with `CONTROL`,
  `Char('/')` with `SHIFT`, and a `Release` and a `Repeat` of `Char('?')`
- **THEN** the first two return `ToggleHelp` and the last four all return `Ignore` —
  `Char('/')` with `SHIFT` is not `?`, and `action_for`'s filter arm matches
  `KeyModifiers::NONE` only, so the filter key carrying a stray modifier falls to the
  wildcard rather than starting a filter; and a terminal reporting releases cannot toggle
  twice
- **AND** with `filtering` true, `Char('?')` with `NONE` and with `SHIFT` both return
  `FilterPush('?')`, so the key types into the query and the overlay does not open
- **AND** every other key's mapping is unchanged under both modes: the table
  `list-sections` asserted returns exactly the same actions, so `?` gaining a meaning
  moved no existing key

#### Scenario: The overlay opens and closes without moving the route

- **WHEN** a `Dashboard` at `Route::Detail` with `detail.tab` `2`, `detail.scroll` `7`,
  and `selected` `3` is given `ToggleHelp`, then `ToggleHelp` again
- **THEN** after the first, `overlay.panel` is `Some(Panel::Help)` and `overlay.scroll` is `0`
- **AND** after the second, `overlay.panel` is `None` and `overlay.scroll` is `0`
- **AND** `route` is `Detail`, `detail.tab` is `2`, `detail.scroll` is `7`, and `selected`
  is `3` after both, so the reader returns to exactly the frame they left
- **AND** `changes`, `filter`, `quit`, `refresh`, `agents`, `agent_names`, `launch`, and
  `sections` are all unchanged, field for field, after both

#### Scenario: `Esc` closes the overlay before any other layer

- **WHEN** a `Dashboard` at `Route::Detail` whose `filter.query` is `add`, whose
  `filter.active` is true, and whose `overlay.panel` is `Some(Panel::Help)` is given four consecutive `Back`
  actions
- **THEN** after the first, `overlay.panel` is `None` and `filter.active` is **still true**
  with the query still `add` — the overlay is the outermost layer and `Back` dismisses
  exactly one
- **AND** after the second, `filter.active` is false and `filter.query` is empty; after
  the third, `route` is `List`; after the fourth, nothing has changed
- **AND** `quit` is false after all four

#### Scenario: `?` and `,` swap panels rather than stacking them

- **WHEN** a `Dashboard` at `Route::Detail` with `overlay.panel` `Some(Panel::Help)` and
  `overlay.scroll` `6` is given `ToggleSettings`, then `ToggleHelp`, then `ToggleHelp`
- **THEN** `overlay.panel` is `Some(Panel::Settings)`, then `Some(Panel::Help)`, then `None`
- **AND** `overlay.scroll` is `0` after each of the first two, since the two panels have
  unrelated row counts and a carried position would land the reader at an arbitrary row
- **AND** `route`, `selected`, `detail`, `filter`, and `sections` are unchanged after all three

## REMOVED Requirements

### Requirement: The overlay answers seven actions and ignores every other one

**Reason**: The header states a count this change changes. `settings-window` adds
`ToggleSettings`, which the help panel answers by swapping to the settings panel, taking the
answered set from seven to eight and the `Action` total from twenty-five to twenty-six. The
requirement's own closing scenario asserts that arithmetic, so a header left at seven would
contradict a scenario inside it.

**Migration**: Replaced by the ADDED requirement below. The eighteen-member inert list is
carried forward unchanged, member for member; the answered table gains one row; `help.open`
becomes `overlay.panel` and `help.scroll` becomes `overlay.scroll`. The `Quit` exception and
the blanket-rule paragraph are unchanged.

## ADDED Requirements

### Requirement: The help panel answers eight actions and ignores every other one

While `overlay.panel` is `Some(Panel::Help)`, `Dashboard::apply` SHALL dispatch as follows, and this dispatch
SHALL take precedence over the route dispatch and over the filter dispatch alike:

| Action | Effect while the overlay is open |
|---|---|
| `Quit` | quit, exactly as when it is closed |
| `ToggleHelp` | close the overlay and reset `overlay.scroll` to `0` |
| `ToggleSettings` | swap the panel to `Settings` and reset `overlay.scroll` to `0` |
| `Back` | close the overlay and reset `overlay.scroll` to `0` |
| `Next`, `ScrollDown` | move `overlay.scroll` down one line |
| `Prev`, `ScrollUp` | move `overlay.scroll` up one line, saturating at `0` |
| every other action | change nothing at all |

"Every other action" is the closed remainder: `OpenDetail`, `SelectTab`, `NextTab`,
`PrevTab`, `FilterStart`, `FilterPush`, `FilterPop`, `Refresh`, `LaunchApply`,
`LaunchContinue`, `LaunchArchive`, `FocusAgent`, `ToggleSection`, `SelectNext`,
`SelectPrev`, `Click`, `Select`, and `Ignore`. Eighteen actions — `Select` was missing from
this enumeration while the count already said eighteen, a slip `text-selection` left behind
and this change repairs in passing rather than reproduces — and none of them does anything
while the help panel is open. **Eight** answer — `Quit`, `ToggleHelp`, `ToggleSettings`,
`Back`, `Next`, `Prev`, `ScrollDown`, `ScrollUp` — and eighteen plus eight is the
**twenty-six** `Action` carries after `settings-window`, so the two lists are exhaustive
between them with nothing counted twice. The inert list is unchanged, member for member:
`settings-window` added one action and it answers, so nothing moved from one list to the other.

`ToggleSettings` answers rather than being inert, and it is the one answered action that
neither closes nor scrolls: it **swaps** the panel. That is not a hole in "read-only".
Swapping panels renders a different set of rows and reaches no collaborator, spawns no
process, touches no filesystem, and reads no clock, which is exactly what the read-only rule
names; and a `,` that did nothing from inside the help would be the same dead key the `Quit`
exception exists to avoid.

`LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent` being inert is the
load-bearing half of "read-only": `a`, `c`, `s`, and `g` reach Herdr and start a process,
and a reader who opened the help to find out what `a` does must be able to press it
without launching an agent. `Refresh` being inert is the same argument one step down —
it starts no process but it does start a CLI cycle.

`Quit` is the one exception, and it is deliberate: `q` and `Ctrl-C` SHALL close the pane
from inside the overlay exactly as from outside it. A modal that traps the reader is a
worse failure than one that lets a quit through, and both keys have a row in the
inventory's `Pane` group saying so.

The blanket rule `apply` runs after **every** action — setting `refresh.requested` when
`Dashboard::needs_archived_refresh()` holds — SHALL continue to run while the overlay is
open, unchanged. It is not an action's effect and the overlay does not suppress it.

#### Scenario: The agent keys launch nothing while the overlay is open

- **WHEN** a `Dashboard` with `agents.reachable` true, a selected change `add-auth`, and
  `overlay.panel` `Some(Panel::Help)` is given `LaunchApply`, `LaunchContinue`, `LaunchArchive`,
  `FocusAgent`, and `Refresh` in turn
- **THEN** `launch.pending` is `None` and `launch.problems` is empty after all five
- **AND** `refresh.requested` is false after all five, unless
  `needs_archived_refresh()` holds, in which case it is true after every one of them and
  for that reason alone
- **AND** the whole dashboard but for that one flag is equal, field for field, to the one
  before the five actions
- **AND** `apply` reaches no collaborator by construction — it takes `&mut self` and an
  `Action` and holds no handle — so this scenario's evidence is the field-for-field equality
  above and the `NOIO-VIEW` sweep over `src/ui/app.rs`, not a spy that could never fire

#### Scenario: Both quit keys still quit from inside the overlay

- **WHEN** a `Dashboard` with `overlay.panel` `Some(Panel::Help)` is given `Quit`
- **THEN** `quit` is true
- **AND** the same holds for a dashboard whose `overlay.panel` is `Some(Panel::Help)` and whose
  `filter.active` is also true, so no combination of layers traps the reader

#### Scenario: The overlay swallows every inert action

- **WHEN** a `Dashboard` at `Route::List` with six active changes, `selected` `2`,
  `detail.tab` `1`, and `overlay.panel` `Some(Panel::Help)` is given `OpenDetail`, `SelectTab(3)`,
  `NextTab`, `PrevTab`, `FilterStart`, `FilterPush('a')`, `FilterPop`, `ToggleSection`,
  `SelectNext`, `SelectPrev`, and `Click(Target::Change(0))` in turn
- **THEN** `route` is still `List`, `selected` is still `2`, `detail.tab` is still `1`,
  `filter.query` is still empty, `filter.active` is still false, and `sections` is
  unchanged
- **AND** `overlay.panel` is still `Some(Panel::Help)` and `overlay.scroll` is still `0` after all eleven

#### Scenario: `j` and `k` scroll the overlay rather than the frame beneath

- **WHEN** a `Dashboard` at `Route::Detail` with `detail.scroll` `4`, `selected` `1`, and
  `overlay.panel` `Some(Panel::Help)` is given `Next`, `Next`, `Next`, then `Prev`
- **THEN** `overlay.scroll` is `3` after the third and `2` after the fourth
- **AND** `detail.scroll` is still `4` and `selected` is still `1` after all four
- **AND** the same dashboard at `Route::List` given the same four actions moves
  `overlay.scroll` identically and leaves `selected` at `1`, so the overlay's scroll is
  route-agnostic where `Next` and `Prev` are not
- **AND** `Prev` applied to a dashboard whose `overlay.scroll` is `0` leaves it `0` rather
  than underflowing

#### Scenario: The overlay lists the agent keys when the socket is unreachable

- **WHEN** a `Dashboard` with `agents.reachable` **false** and `overlay.panel` `Some(Panel::Help)` is rendered at
  120x40 and at 60x20, and again with `agents.reachable` **true**
- **THEN** the two bands are byte-identical, cell for cell, style included, at both widths
- **AND** both hold the `Agents` group with all four of `a`, `c`, `s`, and `g` and their
  descriptions
- **AND** this is the scenario the change's accepted footer cost rests on: the footer drops
  `a/c/s launch` and `g focus` when the socket is unreachable and drops `g focus` at 60 columns
  when it is reachable, and the argument for accepting both is that the overlay lists them
  anyway. `INVENTORY` is `'static` and no render path consults `agents.reachable`, which is what
  makes that true rather than hoped for

#### Scenario: `Action::Select` is inert while the overlay is open

- **WHEN** `Dashboard::apply` is called with `Action::Select` at any phase while `overlay.panel`
  is `Some(Panel::Help)`
- **THEN** nothing changes: no selection is made, extended, or cleared, and nothing is copied
- **AND** the inert list holds eighteen actions and the answered list eight, summing to the
  twenty-six `Action` carries after `settings-window`
- **AND** the mouse resolver never produces it there anyway, because a drag over the band
  already resolves to `Action::Ignore`

#### Scenario: `,` swaps to the settings panel from inside the help

- **WHEN** a `Dashboard` at `Route::Detail` with `overlay.panel` `Some(Panel::Help)` and
  `overlay.scroll` `9` is given `ToggleSettings`
- **THEN** `overlay.panel` is `Some(Panel::Settings)` and `overlay.scroll` is `0`
- **AND** `route` is still `Detail` and `selected`, `detail`, `filter`, and `sections` are
  unchanged, so the swap moved no part of the frame beneath
- **AND** no collaborator was reached, no process spawned, and no file read, so the swap is
  read-only in the same sense the rest of this requirement is

