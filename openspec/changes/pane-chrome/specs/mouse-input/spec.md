## MODIFIED Requirements

### Requirement: The wheel scrolls the region under the pointer

A `MouseEventKind::ScrollDown` SHALL produce `Action::SelectNext` when the pointer is over
the list region and `Action::ScrollDown` when it is over the detail region;
`MouseEventKind::ScrollUp` SHALL produce `Action::SelectPrev` and `Action::ScrollUp` on the
same terms. Both SHALL produce `Action::Ignore` when the pointer is over the frame's footer
row or outside the frame entirely. There is no frame header row to ignore any more:
`pane-chrome` removed it, so every row of the frame but the last is a body row.

The region under the pointer is the whole region — its two gutter columns and its heading row
included, and, for the detail region, its tab bar, its rule, and the blank row above the bar
as well as its content area — not only the interior rows a click addresses. A wheel event in
a region's gutter scrolls that region rather than doing nothing, and so does one on the
vertical divider, which is the detail region's own left gutter.

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
  because the divider is the detail region's own gutter, and the last two return
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
| A problem row, a message row, an interior row past the last drawn row, a region's gutter or heading row, the divider, the detail region's spacer row, rule row, or content area, the frame footer, or outside the frame | `Action::Ignore` |

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
  left gutter, on the list region's heading row, on the detail region's heading row, on the
  detail region's rule row, on the detail content area, on the frame's
  footer, and at column 200
- **THEN** every call returns `Action::Ignore`
- **AND** applying `Action::Ignore` leaves the dashboard equal to what it was

#### Scenario: The other buttons and the non-press kinds are inert

- **WHEN** `mouse_action` is called over a drawn change row with `Down(Right)`,
  `Down(Middle)`, `Up(Left)`, `Drag(Left)`, and `Moved`
- **THEN** every call returns `Action::Ignore`
- **AND** in particular no press of any button reaches `Action::LaunchApply`,
  `Action::LaunchContinue`, `Action::LaunchArchive`, or `Action::FocusAgent`, so a
  mis-click cannot start or focus an agent

