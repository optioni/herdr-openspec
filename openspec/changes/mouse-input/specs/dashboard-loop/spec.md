## MODIFIED Requirements

### Requirement: Key handling is a pure, total function over events

`ui::app::action_for(event: &Event, filtering: bool) -> Action` SHALL map a terminal event
and the current filter mode to one of exactly **twenty-three** actions — `Quit`,
`OpenDetail`, `Back`, `Next`, `Prev`, `SelectNext`, `SelectPrev`, `ScrollDown`, `ScrollUp`,
`Click(Target)`, `SelectTab(usize)`, `NextTab`, `PrevTab`, `FilterStart`,
`FilterPush(char)`, `FilterPop`, `Refresh`, `LaunchApply`, `LaunchContinue`, `LaunchArchive`,
`FocusAgent`, `ToggleSection`, `Ignore` — and SHALL be total: every `Event`
value, including mouse, paste, focus-gained, focus-lost, and resize events, maps to one of
them under either value of `filtering`, and none panics.

`action_for` SHALL map **no key** to the five actions `mouse-input` adds. `SelectNext`,
`SelectPrev`, `ScrollDown`, `ScrollUp`, and `Click(Target)` exist because a mouse event names
the region or the row it landed on, where a key does not; `ui::driver::mouse_action` is the
only producer of them, and `mouse-input` states what it produces. They are five flat variants
rather than two carrying a direction or a region, for the same reason the four launch
variants are flat: an enumerate-by-hand test must enumerate the same things the exhaustive
`match` does.

The count moved from thirteen to seventeen when `agent-launch` landed, not from nine to
thirteen: `HANDOFF.md`'s Phase 5 constraint 8 read the count off a stale doc comment in
`src/ui/app.rs` that still said "the nine outcomes" after four had been added. It moved to
**eighteen** with `ToggleSection`, `list-sections`' one addition, and moves to
**twenty-three** here, with `mouse-input`'s five.

`SelectTab`, `NextTab`, and `PrevTab` are `detail-view`'s additions; `artifact-tabs` states
their keys and their effect. They are route-agnostic in the same sense `Next` and `Prev`
are: the detail region is drawn at both routes above the breakpoint, so a tab press at the
list route is immediately visible. `Refresh` is `live-refresh`'s addition and is
route-agnostic in a stronger sense: it names no region at all. `LaunchApply`,
`LaunchContinue`, `LaunchArchive`, and `FocusAgent` are `agent-launch`'s additions and are
route-agnostic in that same stronger sense: they act on the **selected** change, which is the
same change at either route. `ToggleSection` is `list-sections`' addition and is
route-agnostic in the same way: it folds the section the cursor is in, which is the same
section at either route, and `list-selection` owns what it does. It moves no existing key —
`Space` was `Ignore` outside filter mode and `FilterPush(' ')` inside it, and it stays
`FilterPush(' ')` inside it.

They are **four flat variants** rather than one variant carrying a payload. A payloaded
`Launch(Intent)` would let `no_action_mutates_changes`' hand-written `variants` array carry one
intent and silently omit the other two, which is precisely the failure that test's own comment
warns against ("an enumerate-by-hand test would silently miss it"); four flat variants make the
exhaustive `match` and the array enumerate the same four things.

While `filtering` is **false** the mapping SHALL be:

| Input | Action |
|---|---|
| `KeyCode::Char('q')` with no modifiers | `Quit` |
| `KeyCode::Char('c')` with `KeyModifiers::CONTROL` | `Quit` |
| `KeyCode::Char('j')` or `KeyCode::Down` with no modifiers | `Next` |
| `KeyCode::Char('k')` or `KeyCode::Up` with no modifiers | `Prev` |
| `KeyCode::Char('1')`–`Char('9')` with no modifiers | `SelectTab(digit - 1)` |
| `KeyCode::Char(']')` with no modifiers | `NextTab` |
| `KeyCode::Char('[')` with no modifiers | `PrevTab` |
| `KeyCode::Char('/')` with no modifiers | `FilterStart` |
| `KeyCode::Char('r')` with no modifiers | `Refresh` |
| `KeyCode::Char('a')` with no modifiers | `LaunchApply` |
| `KeyCode::Char('c')` with **no** modifiers | `LaunchContinue` |
| `KeyCode::Char('s')` with no modifiers | `LaunchArchive` |
| `KeyCode::Char('g')` with no modifiers | `FocusAgent` |
| `KeyCode::Char(' ')` with no modifiers | `ToggleSection` |
| `KeyCode::Enter` with no modifiers | `OpenDetail` |
| `KeyCode::Esc` with no modifiers | `Back` |
| anything else, including `Char('Q')`, `Char('0')`, `Char('R')`, `Char('A')`, `Char('G')`, and `Char('q')`, `Char('r')`, `Char('a')`, `Char('s')`, or `Char('g')` with a modifier | `Ignore` |

`Char('c')` is the one key with two rows. Bare `c` is `LaunchContinue`; `c` with
`KeyModifiers::CONTROL` is `Quit`, and remains so under both values of `filtering`. The two are
distinguished by the modifier alone, which the mapping already matches on, so no key is
overloaded ambiguously and `Ctrl-C` never launches.

`action_for` SHALL NOT take the socket's reachability as a parameter and SHALL NOT consult it.
Whether an action key is *offered* is `agent-launch`'s decision, made in `Dashboard::apply`
against `agents.reachable`; the key-to-action mapping stays a pure function of the event and the
filter mode, so a socket that comes and goes never changes what a key means.

While `filtering` is **true** the mapping SHALL be the one `list-filtering` states, in which
printable characters type into the query and only `Ctrl-C` quits. `1`–`9`, `[`, `]`, `r`, `a`,
`c`, `s`, and `g` are printable characters and are therefore query characters there, with no
exception carved out for any of them — and so is `' '`, which `list-filtering` states
explicitly because a space in a text field is the one printable character a reader might
expect to keep a command meaning.

`action_for` SHALL act only on key events whose `kind` is `KeyEventKind::Press`. A key event
with kind `Repeat` or `Release` SHALL map to `Ignore` under either value of `filtering`, so
a terminal that reports release events does not quit twice, navigate on a release, type a
character twice, refresh twice, or **launch a second agent**.

`Dashboard::apply(&mut self, action: Action)` SHALL:

- set `quit` on `Quit`;
- on `OpenDetail`, clear `filter.active` and change nothing else when `filter.active` is
  set — accepting a filter is not opening a detail — and otherwise set `route` to `Detail`
  and reset `detail.scroll` to `0`;
- on `Back`, dismiss exactly one layer, in this order: filter mode with its query when
  `filter.active` is set; else a non-empty `filter.query`; else `route` back to `List`,
  resetting `detail.scroll` to `0`; else nothing at all, so a stray `Esc` at the root cannot
  close the pane. `launch.problems` SHALL NOT be one of the layers: no key dismisses a launch
  problem, and the next launch outcome is what replaces it;
- on `Next` and `Prev`, move and clamp `selected` per `list-selection` when `route` is
  `List`, and move `detail.scroll` by one line per `detail-scroll` when `route` is `Detail`,
  never both; and, at `Route::List` only, reset `detail.tab` and `detail.scroll` to `0`
  exactly when `selected` changed value. `Next` and `Prev` SHALL be exactly a route
  dispatch over the four region-explicit actions below and SHALL hold no arithmetic of
  their own: `Next` does what `SelectNext` does at `Route::List` and what `ScrollDown` does
  at `Route::Detail`, and `Prev` does what `SelectPrev` and `ScrollUp` do on the same terms,
  so a key and a wheel over the same region can never diverge;
- on `SelectNext` and `SelectPrev`, move and clamp `selected` per `list-selection` and reset
  `detail.tab` and `detail.scroll` to `0` exactly when `selected` changed value, **at either
  route** — these name the list region rather than inheriting the route, which is what lets
  the wide layout's two regions scroll independently;
- on `ScrollDown` and `ScrollUp`, move `detail.scroll` by one line per `detail-scroll`, **at
  either route**, changing `selected`, `detail.tab`, and `route` not at all;
- on `Click(target)`, do nothing when `target` is absent from `targets()`; otherwise act per
  `mouse-input` — a `Target::Section(key)` toggles that section and moves the cursor to its
  header exactly as `ToggleSection` does, and a `Target::Change(i)` moves the cursor to that
  row (resetting `detail.tab` and `detail.scroll` to `0` when it moved), or, when the cursor
  is already there and `route` is `List`, sets `route` to `Detail` and resets `detail.scroll`
  to `0`. It SHALL reach no collaborator, spawn no process, touch no filesystem, and read no
  clock;
- on `SelectTab`, `NextTab`, and `PrevTab`, move `detail.tab` per `artifact-tabs`, resetting
  `detail.scroll` to `0` exactly when `detail.tab` changed value;
- set `filter.active` and `route: List` on `FilterStart`, resetting `detail.scroll` to `0`
  because that too is a route move, push on `FilterPush`, pop on `FilterPop`, clamping
  `selected` after each;
- set `refresh.requested` on `Refresh` and change nothing else at all — not `changes`, not
  `selected`, not `route`, not `detail`, not `filter`, not `quit` — reaching no collaborator
  and starting no work, so `apply` stays a pure function of `&mut self` and its argument;
- on `ToggleSection`, fold or unfold exactly one section per `list-selection` — the one the
  cursor addresses, or the one the addressed change belongs to — and move `selected` to that
  section's header. It SHALL change nothing else: not `changes`, not `route`, not `detail`,
  not `filter`, not `quit`, not `agents`, not `agent_names`, and not `launch`, and it SHALL
  reach no collaborator;
- on `LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent`, map the action to the
  corresponding `launch::Intent`, call `launch::decide` with the selected change's name, the
  focus pane `attribution().panes` holds for it, `agents.reachable`, and the `name`s of the
  live agents, and write the result: `Decision::Nothing` changes nothing at all;
  `Decision::Refuse(reason)` replaces `launch.problems` with that one entry and leaves
  `launch.pending` alone; `Decision::Go(request)` sets `launch.pending` to `Some(request)` and
  clears `launch.problems`. **None of the four SHALL change `changes`, `selected`, `route`,
  `detail`, `filter`, `quit`, `refresh`, `agents`, or `agent_names`, and none SHALL reach a
  collaborator, spawn a process, touch the filesystem, or read a clock** — `apply` stays a pure
  function of `&mut self` and its argument, and `run_loop` is what turns `launch.pending` into a
  request to a collaborator that lives outside `src/ui/` entirely;
- change nothing on `Ignore`.

After applying **any** action, `apply` SHALL set `refresh.requested` to true when
`Dashboard::needs_archived_refresh()` holds — `list-sections`' rule, stated in full by
`list-selection`. Writing it once for every action rather than for a named subset is what
keeps a future section-opening key from forgetting it; it is the one place `apply` sets a
flag an action did not name, and it sets no other.

`apply` SHALL never panic, SHALL never leave `selected` addressing a target that is not
visible, and SHALL never leave `detail.scroll` unbounded for more than one frame — the
normalisation `detail-scroll` requires of `ui::driver::run_loop` is what bounds it. It MAY
leave `detail.tab` out of range for the selected change after a filter edit; `sync_detail`
is what restores that invariant, before the next draw rather than after it.

#### Scenario: The four action keys map, and their near misses do not

- **WHEN** `action_for` is called with `filtering` false and Presses of `Char('a')`,
  `Char('c')`, `Char('s')`, `Char('g')`, `Char('c')` with `CONTROL`, `Char('A')` with `SHIFT`,
  `Char('G')` with `SHIFT`, `Char('a')` with `CONTROL`, and `Char('s')` with `ALT`
- **THEN** the first four return `LaunchApply`, `LaunchContinue`, `LaunchArchive`, and
  `FocusAgent`; the fifth returns `Quit`; and the last four return `Ignore`
- **AND** with `filtering` true the same nine return `FilterPush('a')`, `FilterPush('c')`,
  `FilterPush('s')`, `FilterPush('g')`, `Quit`, `FilterPush('A')`, `FilterPush('G')`,
  `Ignore`, and `Ignore` — so `Ctrl-C` is the only one of the nine that still quits and none of
  the four launches while a filter is open
- **AND** a `Release` and a `Repeat` of each of `Char('a')`, `Char('c')`, `Char('s')`, and
  `Char('g')` return `Ignore` under both modes, so a terminal reporting releases cannot launch
  a second agent

#### Scenario: A launch action reaches no collaborator and starts no work

- **WHEN** a `Dashboard` with `agents.reachable` `true` and a selected change `add-auth` is
  given `LaunchApply`
- **THEN** `launch.pending` is
  `Some(Request::Launch { change: "add-auth", agent: "add-auth", intent: Apply })` and
  `launch.problems` is empty
- **AND** `changes`, `selected`, `route`, `detail`, `filter`, `quit`, `refresh`, `agents`, and
  `agent_names` are all unchanged, field for field
- **AND** no process was spawned, no file was read or written, and no clock was read — `apply`
  is a pure function of `&mut self` and its argument, and the dashboard value it produces is
  `Clone` and `PartialEq` as before

#### Scenario: A refused launch records the reason and produces no request

- **WHEN** a `Dashboard` with `agents.reachable` `true`, a selected change `2fa-support`, and a
  live agent named `c-2fa-support` is given `LaunchApply` three times in a row
- **THEN** `launch.pending` is `None` after every one of the three
- **AND** `launch.problems` holds exactly **one** entry after all three, naming
  `c-2fa-support` and `g` — replaced wholesale each time, never grown

#### Scenario: An unreachable socket makes the four keys change nothing

- **WHEN** a `Dashboard` with `agents.reachable` `false` and a selected change is given
  `LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent` in turn
- **THEN** `launch.pending` is `None` and `launch.problems` is empty after all four
- **AND** the whole dashboard is equal, field for field, to the one before the four actions

#### Scenario: Both quit keys quit and neither near-miss does

- **WHEN** `action_for` is called with `filtering` false and a Press of `Char('q')` with no
  modifiers, a Press of `Char('c')` with `CONTROL`, a Press of `Char('Q')` with `SHIFT`, a
  Press of `Char('q')` with `CONTROL`, and a Press of `Char('c')` with no modifiers
- **THEN** the first two return `Quit`, the next two return `Ignore`, and the fifth returns
  `LaunchContinue` — bare `c` is `agent-launch`'s key and was `Ignore` before it
- **AND** with `filtering` true the same five events return `FilterPush('q')`, `Quit`,
  `FilterPush('Q')`, `Ignore`, and `FilterPush('c')`, so exactly one of them still quits

#### Scenario: A released quit key does not quit

- **WHEN** `action_for` is called with `Char('q')` carrying kind `Release`, then with
  `Char('q')` carrying kind `Repeat`, then with `Char('q')` carrying kind `Press`, each
  under `filtering` false and again under `filtering` true
- **THEN** under `filtering` false the first two return `Ignore` and the third returns
  `Quit`
- **AND** under `filtering` true the first two return `Ignore` and the third returns
  `FilterPush('q')`, so a release cannot type a character either
- **AND** the same holds for `Char('1')`, `Char(']')`, and `Char('r')`: a `Release` or
  `Repeat` of any of them returns `Ignore` under both modes, so a terminal reporting releases
  cannot switch tabs twice or refresh twice

#### Scenario: Enter and Esc move between the two routes

- **WHEN** a `Dashboard` at `Route::List` with an empty, inactive filter and
  `detail.scroll` of `0` is given the actions for a Press of `Enter`, then a Press of `Esc`,
  then a second Press of `Esc`
- **THEN** its route is `Detail`, then `List`, then still `List`
- **AND** `quit` is false after all three, so `Esc` at the root does not close the pane
- **AND** `detail.scroll` is `0` after each, since both route moves reset it
- **AND** `detail.tab` is unchanged by all three, because a route move is not a change move
- **AND** `refresh.requested` is unchanged by all three, because a route move is not a refresh
- **AND** `launch.pending` and `launch.problems` are unchanged by all three, because `Esc` is
  not a layer that dismisses a launch problem

#### Scenario: `Esc` dismisses one layer at a time

- **WHEN** a `Dashboard` at `Route::Detail` whose `filter.query` is `add`, whose
  `filter.active` is true, and whose `detail.scroll` is `3` is given four consecutive `Back`
  actions
- **THEN** after the first, `filter.active` is false and `filter.query` is empty, and
  `detail.scroll` is still `3` — dismissing the filter layer is not leaving the route; after
  the second, `route` is `List` and `detail.scroll` is `0`; after the third and fourth,
  nothing has changed and `quit` is still false
- **AND** a second `Dashboard` at `Route::List` whose `filter.query` is `add` with
  `filter.active` **false** reaches an empty query on its first `Back` and changes nothing
  on its second
- **AND** a third `Dashboard` carrying one `launch.problems` entry still carries it after four
  consecutive `Back` actions

#### Scenario: Navigation and filter keys are distinguished from near misses

- **WHEN** `action_for` is called with `filtering` false and Presses of `Char('j')`,
  `Down`, `Char('k')`, `Up`, `Char('/')`, `Char('J')` with `SHIFT`, `Down` with `CONTROL`,
  and `Char('/')` with `CONTROL`
- **THEN** the first five return `Next`, `Next`, `Prev`, `Prev`, and `FilterStart`, and the
  last three return `Ignore`
- **AND** Presses of `Char('1')`, `Char('9')`, `Char(']')`, and `Char('[')` return
  `SelectTab(0)`, `SelectTab(8)`, `NextTab`, and `PrevTab`, while `Char('0')`, `Char('{')`,
  and `Char(']')` with `CONTROL` return `Ignore`
- **AND** a Press of `Char('r')` returns `Refresh`, while `Char('R')` with `SHIFT` and
  `Char('r')` with `CONTROL` both return `Ignore`

#### Scenario: Non-key events are ignored without panicking

- **WHEN** `action_for` is called with `Event::Resize(60, 20)`, `Event::FocusGained`,
  `Event::FocusLost`, `Event::Paste("q".to_string())`, `Event::Paste("1".to_string())`,
  `Event::Paste("r".to_string())`, `Event::Paste("a".to_string())`, and a mouse event, each
  under `filtering` false and again under `filtering` true
- **THEN** each returns `Ignore` under both
- **AND** in particular a paste whose text is the single character `q` neither quits nor
  types into the query, a paste whose text is `1` does not switch tabs, a paste whose text
  is `r` does not refresh, and a paste whose text is `a` **does not launch an agent**, so
  pasted content cannot close the pane, edit the filter, move the tab, start a CLI cycle, or
  spawn a process
- **AND** the mouse event still returns `Ignore` from `action_for` under both modes after
  `mouse-input` lands: the mouse is resolved by `ui::driver::mouse_action`, which takes the
  frame geometry `action_for` never receives, so this mapper stays key-only

#### Scenario: `Space` maps to `ToggleSection` outside filter mode and types inside it

- **WHEN** `action_for` is called with `filtering` false and a Press of `Char(' ')`, then with
  `filtering` true and the same Press, then with `filtering` false and a Press of `Char(' ')`
  carrying `KeyModifiers::CONTROL`, and finally with a `Release` and a `Repeat` of
  `Char(' ')` under both modes
- **THEN** the first returns `ToggleSection`, the second returns `FilterPush(' ')`, and the
  last three under each mode return `Ignore`
- **AND** every other key's mapping is unchanged: the same table of inputs
  `agent-launch` asserted returns exactly the same actions under both modes, so `Space`
  gaining a meaning moved no existing key
- **AND** `KeyCode::Char(' ')` is the only new row in the `filtering` false table, and
  `filtering` true gains no row at all

## ADDED Requirements

### Requirement: The loop resolves a mouse event against the frame it just drew

`ui::driver::run_loop` SHALL keep the `area` of the frame it has just drawn — the value it
already copies out of the `CompletedFrame` for `Dashboard::normalise_scroll` — and SHALL
pass it to `ui::driver::mouse_action` when the event it reads is an `Event::Mouse`. Every
other event SHALL continue to go to `ui::app::action_for` with `dashboard.filter.active`.

The area used SHALL be the one just drawn, never a stored size and never the size at
startup: a mouse event is a reply to a frame the reader is looking at, and resolving it
against anything else would target rows that are not on screen. A resize between the draw
and the click therefore costs at most one mis-targeted event, which the next frame
corrects — the same one-frame window `normalise_scroll` already accepts for the scroll
clamp.

`run_loop` SHALL apply exactly one action per event and SHALL break on `dashboard.quit`
exactly as it already does, so a mouse event can neither apply two actions nor bypass the
quit check.

#### Scenario: A resize between the draw and the click costs one frame, not a panic

- **WHEN** `run_loop` is driven at 120x40 with a scripted source yielding
  `Event::Resize(60, 20)`, then a left press at column 100 — a column that existed in the
  frame before the resize and does not exist after it — then `q`
- **THEN** the loop completes without panicking
- **AND** the press is resolved against the 60-column frame drawn after the resize, so it
  falls outside the frame and returns `Action::Ignore`

#### Scenario: The loop routes mouse and key events to different mappers

- **WHEN** `run_loop` is driven with a scripted source yielding a `ScrollDown` over the
  detail region, then `Char('j')`, then `q`, on a dashboard at `Route::List` above the
  breakpoint
- **THEN** after the wheel event `detail.scroll` is `1` and `selected` is unchanged
- **AND** after the `j` event `selected` has advanced and `detail.scroll` is back to `0`,
  because a selection move resets it — so the wheel reached `ScrollDown` and the key reached
  `Next`, and neither took the other's path
