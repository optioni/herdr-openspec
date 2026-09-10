# mouse-input Specification

## Purpose
Gives the pane a second input device without letting it become a second interface. It owns
`ui::driver::mouse_action` — a pure, total map from a `MouseEvent`, the frame `run_loop` has
just drawn, and the dashboard's own rows to one of the actions `Action` already carries —
and the rule that `ui::app::action_for` stays key-only, because a mouse event's meaning
depends on geometry a key mapper never receives. The wheel names the region under the
pointer rather than inheriting the route, which is what lets the wide layout's two regions
scroll independently for the first time; a left click names the row it landed on, opens the
row it is already on, toggles a list section header, switches a tab cell, or — in the detail
region's **content area**, when the selected artifact is foldable — moves the detail cursor to
the row it landed on and folds that row's section if it is a section header, through the very
code `Space` runs at the detail route. It fixes what stays inert: every other button, every
release, every drag, pointer motion, both horizontal wheel directions, the frame's chrome,
problem and message rows, the content area of a **non**-foldable artifact, a content row past
the last one drawn, and every point outside the frame.
Modifiers are read nowhere, and the filter mode is not an argument at all — a printable key
is ambiguous while filtering and a click is not, so both rules are structural rather than
branches a later change can get wrong.

Two constraints bound it, and both are load-bearing rather than decorative. **Nothing becomes
mouse-only**: every action the pointer can produce is reachable by a key that already exists,
no key changes meaning, and the pane stays fully usable over SSH in a terminal that reports no
mouse. And **the resolver is answerable to what was drawn**: it resolves against the frame just
drawn rather than a stored size, and the hit test, the row lookup, and the tab lookup each
derive their geometry from the very functions the draw path uses — `layout::zone` through the
frame splits, `list::row_at` through the offset derivation `render_list` shares with it, and
`detail::tab_at` from `tab_bar`'s own reported cells — so a click cannot land on a row the
reader is not looking at. `SPEC.md` → Keys carries the bindings and the one cost capture
imposes, the terminal's own drag-to-select, and a doc-conformance test binds that table to the
resolver so a binding added or renamed in one without the other fails `cargo test`.

## Requirements

### Requirement: Mouse events are resolved against the frame the reader is looking at

`ui::driver::mouse_action(dashboard: &Dashboard, area: Rect, mouse: &MouseEvent) -> Action`
SHALL map a mouse event to one of the actions `Action` already carries, using `area` — the
area of the frame `run_loop` has just drawn — as the geometry the pointer is resolved
against. It SHALL be pure and total: it performs no I/O, reads no clock, mutates nothing,
and returns an `Action` for every `MouseEvent` value, every `Rect` including a zero-sized
one, and every `Dashboard` value, without panicking.

`ui::app::action_for` SHALL remain unchanged and key-only: it continues to map every
`Event::Mouse` to `Action::Ignore`. The mouse is resolved by `mouse_action` instead,
because a mouse event's meaning depends on the frame geometry and the dashboard's own
rows, neither of which `action_for` receives.

`run_loop` SHALL call `mouse_action` for an `Event::Mouse` and `action_for` for every other
event, and SHALL apply the resulting action exactly as it applies a key's — one action per
event, through `Dashboard::apply`.

#### Scenario: A mouse event is resolved through the loop and a key is not

- **WHEN** `run_loop` is driven with a scripted source yielding a left-button press inside
  the list region's interior on the second change row, then `q`
- **THEN** the frame after the press shows the selection marker on that row
- **AND** `ui::app::action_for` called directly with the same `Event::Mouse` value returns
  `Action::Ignore` under `filtering` false and under `filtering` true, so the key mapper is
  unchanged and the loop's mouse handling is what moved the selection

#### Scenario: Resolution is total over adversarial geometry

- **WHEN** `mouse_action` is called with every `MouseEventKind` variant — `Down`, `Up`, and
  `Drag` of each of `Left`, `Right`, and `Middle`, `Moved`, `ScrollDown`, `ScrollUp`,
  `ScrollLeft`, and `ScrollRight` — at columns and rows `0`, `1`, `39`, `40`, `59`, `119`,
  and `65535`, against frame areas of `0x0`, `1x1`, `2x2`, `60x20`, and `120x40`, at both
  routes, with a dashboard holding no repository, one holding no changes, and one holding
  active and archived changes, each under `KeyModifiers::NONE`, `SHIFT`, `CONTROL`, and `ALT`
- **THEN** every call returns an `Action` and none panics
- **AND** for every one of those inputs, the action returned under `SHIFT`, `CONTROL`, and
  `ALT` equals the action returned under `NONE`
- **AND** no call mutates the dashboard, which is passed by shared reference

### Requirement: The wheel scrolls the region under the pointer

A `MouseEventKind::ScrollDown` SHALL produce `Action::SelectNext` when the pointer is over
the list region and `Action::ScrollDown` when it is over the detail region;
`MouseEventKind::ScrollUp` SHALL produce `Action::SelectPrev` and `Action::ScrollUp` on the
same terms. Both SHALL produce `Action::Ignore` when the pointer is over the frame's footer
row or outside the frame entirely. There is no frame header row to ignore any more:
`pane-chrome` removed it, so every row of the frame but the last is a body row.

The region under the pointer is the whole region — its gutter columns, its heading row and
its padding row included, and, for the detail region, its tab bar, its rule, its content
padding row and its content area — not only the interior rows a click addresses. A wheel
event in a region's gutter scrolls that region rather than doing nothing, and one on the
**divider column**, which lies in neither region's area, scrolls the detail region:
`responsive-layout`'s zone requirement is where that assignment is made and argued.

`ScrollLeft` and `ScrollRight` SHALL produce `Action::Ignore`: the pane scrolls in one
dimension only.

Because the wheel names its region rather than inheriting the route, the wide layout's two
regions scroll independently for the first time: a wheel over the list moves the list
selection while `Route::Detail` is current, and a wheel over the detail scrolls the content
while `Route::List` is.

#### Scenario: The two regions scroll independently at 120 columns

- **WHEN** a dashboard at `Route::Detail` with six active changes and a twenty-line
  artifact is drawn at 120x40, and `mouse_action` is called with `ScrollDown` at column 10
  (the list region) and again at column 80 (the detail region)
- **THEN** the first returns `Action::SelectNext` and the second returns
  `Action::ScrollDown`
- **AND** applying the first advances `dashboard.selected` and leaves `detail.scroll` at
  zero, while applying the second advances `detail.scroll` and leaves `selected` unchanged
- **AND** the same two calls at `Route::List` return the same two actions, so the route
  does not decide which region the wheel acts on

#### Scenario: The wheel acts over a border and not over the chrome

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
the chrome is now the footer alone, and what was a border column is now a gutter.

- **WHEN** `mouse_action` is called at 120x40 with `ScrollUp` at the list region's own
  left gutter column (column 0), then at the list region's heading row (row 0), then at the
  divider column 40, then at the frame's footer row (row 39), then at column 200 — past the
  frame's right edge
- **THEN** the first two return `Action::SelectPrev`, the third returns `Action::ScrollUp`
  because `zone` gives the divider column to the detail region, and the last two return
  `Action::Ignore`

#### Scenario: At 60 columns only the routed region answers the wheel

- **WHEN** a dashboard at `Route::List` is drawn at 60x20 and `mouse_action` is called with
  `ScrollDown` at column 30, row 10, and then the same call is made at `Route::Detail`
- **THEN** the first returns `Action::SelectNext` and the second returns
  `Action::ScrollDown`
- **AND** neither returns the other's action, because below the breakpoint the region that
  is drawn is the routed one and there is no second region to be over

#### Scenario: Horizontal wheel events do nothing

- **WHEN** `mouse_action` is called with `ScrollLeft` and then `ScrollRight` over the list
  region, over the detail region, and over the footer, at both mandated widths
- **THEN** every call returns `Action::Ignore`

### Requirement: A left click selects a row, opens it, toggles a section, or switches a tab

A `MouseEventKind::Down(MouseButton::Left)` SHALL be resolved by where it lands:

| Where the press lands | Action |
|---|---|
| A drawn change row in the list region's interior, other than the selected one | `Action::Click(Target::Change(i))` for that row's own `visible()` index |
| The drawn change row that already carries the cursor | `Action::Click(Target::Change(i))` for the same index |
| A drawn section-header row in the list region's interior | `Action::Click(Target::Section(key))` for that header's own key |
| A drawn tab cell in the detail region's tab-bar row | `Action::SelectTab(i)` for that cell's own artifact position |
| A problem row, a message row, an interior row past the last drawn row, a region's gutter, heading row or padding row, the divider, the detail region's rule row, content padding row or content area, the frame footer, or outside the frame | `Action::Ignore` |

`Dashboard::apply(Action::Click(target))` SHALL:

- do nothing at all when `target` is not present in `targets()`;
- when `target` is `Target::Section(key)`, move the cursor to that header and fold the
  section if it is open, unfold it if it is collapsed — exactly what `Action::ToggleSection`
  does for a cursor already on that header, so a click and a `Space` on the same header are
  indistinguishable in their effect;
- when `target` is `Target::Change(i)` and the cursor is **not** already on that row, move
  the cursor to it and reset `detail.tab` and `detail.scroll` to zero, exactly as a
  selection move by `j` or `k` does;
- when `target` is `Target::Change(i)`, the cursor is already on that row, and the route is
  `Route::List`, set the route to `Route::Detail` and reset `detail.scroll` to zero —
  exactly what `Enter` does — so a second click on a selected row opens it;
- when `target` is `Target::Change(i)`, the cursor is already on that row, and the route is
  already `Route::Detail`, change nothing.

A `MouseEventKind::Down` of `MouseButton::Right` or `MouseButton::Middle` SHALL produce
`Action::Ignore`: there is no context menu, and no mouse gesture starts a process.

#### Scenario: A click selects a change row and a second click opens it

- **WHEN** a dashboard with four active changes is drawn at 120x40 at `Route::List` with
  the cursor on the active header, and a left press lands on the third drawn row (the
  second change)
- **THEN** `mouse_action` returns `Action::Click(Target::Change(1))`
- **AND** applying it sets `selected` to that row's target index, `detail.tab` to `0`, and
  `detail.scroll` to `0`, leaving the route `Route::List`
- **AND** a second left press on the same row returns the same action, and applying it sets
  the route to `Route::Detail` with `detail.scroll` at `0` and `selected` unchanged
- **AND** a third left press on the same row returns the same action, and applying it
  changes nothing at all

#### Scenario: A click on a section header folds it exactly as `Space` does

- **WHEN** a dashboard with three active and three archived changes is drawn at 120x40 and
  a left press lands on the `active` header row
- **THEN** `mouse_action` returns `Action::Click(Target::Section(SectionKey::Active))`
- **AND** applying it collapses the active section and leaves `selected` addressing that
  header
- **AND** the resulting `Dashboard` is equal, field for field, to one produced by moving the
  cursor to that header and applying `Action::ToggleSection`
- **AND** a second click on the same header unfolds it again

#### Scenario: A click on an archived header opens an unresolved archive and requests its refresh

- **WHEN** a dashboard whose archived section is collapsed, whose `changes.archived` is
  empty, and whose `changes.archived_total` is `22` is drawn at 120x40, and a left press
  lands on the `archived` header row
- **THEN** applying the returned action opens the section
- **AND** `dashboard.refresh.requested` is `true`, so the click that opened the archive is
  what asks for its resolution, on the same terms `Space` does

#### Scenario: A click on a tab cell switches to that artifact

- **WHEN** a change carrying the five `tdd` artifacts is selected, the dashboard is drawn at
  120x40 with `detail.tab` at `0` and a non-zero `detail.scroll`, and a left press lands
  inside the third tab cell's own painted columns
- **THEN** `mouse_action` returns `Action::SelectTab(2)`
- **AND** applying it sets `detail.tab` to `2` and `detail.scroll` to `0`
- **AND** a press on the one separating column between two cells returns `Action::Ignore`
- **AND** a press on the tab bar of a change with no artifacts — where the bar holds only
  the `no artifacts` placeholder — returns `Action::Ignore`

#### Scenario: Clicks that address nothing are inert

- **WHEN** a dashboard whose `changes.problems` holds one entry and whose visible list is
  empty is drawn at 120x40, and left presses land on the problem row, on the `No changes
  yet` message row, on an interior row below the last drawn row, on the list region's
  left gutter, on the list region's heading row, on its padding row, on the detail region's
  heading row, on the detail region's rule row, on its content padding row, on the detail
  content area, on the frame's footer, and at column 200
- **THEN** every call returns `Action::Ignore`
- **AND** applying `Action::Ignore` leaves the dashboard equal to what it was

#### Scenario: The other buttons and the non-press kinds are inert

- **WHEN** `mouse_action` is called over a drawn change row with `Down(Right)`,
  `Down(Middle)`, `Up(Left)`, `Drag(Left)`, and `Moved`
- **THEN** every call returns `Action::Ignore`
- **AND** in particular no press of any button reaches `Action::LaunchApply`,
  `Action::LaunchContinue`, `Action::LaunchArchive`, or `Action::FocusAgent`, so a
  mis-click cannot start or focus an agent

### Requirement: The mouse acts while filtering

`mouse_action` SHALL NOT take the filter mode as an argument and SHALL resolve a mouse event
identically whether `dashboard.filter.active` is true or false. A printable key is ambiguous
while filtering — it may be a command or a character — and is resolved as a character; a
click is not ambiguous and keeps its meaning.

`run_loop` SHALL therefore pass no filter flag when resolving a mouse event, while
continuing to pass `dashboard.filter.active` to `action_for` for a key.

#### Scenario: A click selects while the filter is open

- **WHEN** a dashboard with six active changes and an active filter whose query is `s` is
  drawn at 120x40, and a left press lands on the second matching change row
- **THEN** the returned action is the same `Action::Click` the same press returns with the
  filter closed
- **AND** applying it moves the cursor to that row and leaves `filter.active` true and
  `filter.query` equal to `s` — the click neither cancels nor accepts the filter

#### Scenario: The wheel scrolls while the filter is open

- **WHEN** the same dashboard receives `ScrollDown` over the list region and then over the
  detail region
- **THEN** the two actions are `Action::SelectNext` and `Action::ScrollDown`, identical to
  the actions the same two events produce with the filter closed

### Requirement: Nothing becomes mouse-only

Every action the mouse can produce SHALL remain reachable by key, and no key SHALL change
its meaning. `Action::SelectNext`, `Action::SelectPrev`, `Action::ScrollDown`, and
`Action::ScrollUp` are what `j`, `k`, and the arrows already reach through `Action::Next`
and `Action::Prev` at the matching route; `Action::Click(Target::Section(_))` is what
`Space` already reaches; `Action::Click(Target::Change(_))` is what `j`/`k` and `Enter`
already reach; `Action::SelectTab(i)` is what `1`–`9`, `[`, and `]` already reach.

The pane SHALL therefore stay fully usable in a terminal that reports no mouse event at
all, including over SSH, with no feature reachable only by pointer.

#### Scenario: The key table is unchanged

- **WHEN** `action_for` is called with the full table of inputs `list-sections` asserted —
  every key and every near miss, under `filtering` false and again under `filtering` true
- **THEN** each returns exactly the action it returned before this change
- **AND** no new key is mapped: the two mapping tables gain no row

#### Scenario: Every mouse action has a key that produces the same effect

- **WHEN** for each of the four list-and-detail outcomes — advance the selection, retreat
  the selection, scroll the content down, scroll the content up — the dashboard is driven
  once by the mouse action and once by the corresponding key at the corresponding route
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** the same holds for a section toggle driven by `Action::Click(Target::Section(k))`
  against `Space`, and for a tab switch driven by a tab click against the matching digit key

### Requirement: `SPEC.md` names the mouse bindings and the drag-to-select cost

`SPEC.md` → Keys SHALL carry a mouse table naming each binding this capability defines —
the wheel over each region, the click on a change row, the second click, the click on a
section header, and the click on a tab cell — and SHALL state that enabling mouse capture
costs the terminal's own drag-to-select, which requires holding `Option` on macOS or
`Shift` on most Linux terminals.

`AGENTS.md`'s terminal-seam rule SHALL name the two capture commands alongside the four
terminal-mode functions it already lists, so the confined set the `NORAW-GREP` gate enforces
and the set the document claims are the same set.

A doc-conformance test SHALL bind both claims to the files that determine them, so a binding
added, removed, or renamed in `ui::driver::mouse_action` without the document following
fails `cargo test`.

#### Scenario: The documented bindings match the resolver

- **WHEN** `tests/doc_contract.rs` reads `SPEC.md` → Keys' mouse table and compares it
  against the bindings `src/ui/driver.rs`'s resolver actually implements
- **THEN** every documented binding is implemented and every implemented binding is
  documented
- **AND** the check fails when the mouse table is absent, rather than passing vacuously

#### Scenario: The documented confined set matches the gate

- **WHEN** `tests/doc_contract.rs` reads the terminal-seam rule's list of confined function
  names from `AGENTS.md` and the `RAW_RE` pattern from `scripts/gates/noraw-grep.sh`
- **THEN** the two name the same six functions —  `enable_raw_mode`, `disable_raw_mode`,
  `EnterAlternateScreen`, `LeaveAlternateScreen`, `EnableMouseCapture`, and
  `DisableMouseCapture`
