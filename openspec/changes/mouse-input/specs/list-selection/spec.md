## ADDED Requirements

### Requirement: A click moves the cursor to the row it lands on

`Action::Click(Target::Change(i))` SHALL move `selected` to the position of
`Target::Change(i)` in `targets()` and reset `detail.tab` and `detail.scroll` to `0`,
exactly as a selection move by `j` or `k` does — and, exactly as those do, SHALL reset
neither when the cursor was already on that row.

A click SHALL never leave `selected` addressing a target that is not in `targets()`: an
action naming a target that is absent — a change filtered away, or a section folded, between
the draw and the event — SHALL change nothing at all rather than clamping to a neighbour.
Clamping would move the cursor somewhere the reader did not click, which is worse than
ignoring a click whose row is gone.

`Action::Click` SHALL change nothing but `selected`, `route`, `detail.tab`,
`detail.scroll`, and `sections.collapsed`, and SHALL reach no collaborator, spawn no
process, touch no filesystem, and read no clock.

#### Scenario: A click on an unselected change moves the cursor and resets the tab

- **WHEN** a dashboard with four active changes has the cursor on the first change,
  `detail.tab` `3`, and `detail.scroll` `9`, and `Action::Click(Target::Change(2))` is
  applied
- **THEN** `selected` addresses `Target::Change(2)`, `detail.tab` is `0`, and
  `detail.scroll` is `0`
- **AND** `route`, `filter`, `changes`, `agents`, `agent_names`, and `launch` are unchanged

#### Scenario: A click on the already-selected change resets nothing

- **WHEN** the cursor is already on `Target::Change(2)` with `detail.tab` `3` and
  `detail.scroll` `9`, `route` is `Route::Detail`, and the same action is applied
- **THEN** `selected`, `detail.tab`, and `detail.scroll` are all unchanged

#### Scenario: A click naming a target that is gone changes nothing

- **WHEN** `Action::Click(Target::Change(7))` is applied to a dashboard whose `targets()`
  holds four entries, and then `Action::Click(Target::Section(SectionKey::Archived))` is
  applied to one with no archived changes at all
- **THEN** neither call changes any field of the dashboard
- **AND** neither panics

#### Scenario: A click reaches a row a collapsed section hides only when it is open

- **WHEN** the archived section is collapsed and `Action::Click(Target::Change(i))` is
  applied for an `i` belonging to an archived change
- **THEN** the target is absent from `targets()` and nothing changes
- **AND** after the section is opened, the same action moves the cursor to that row

### Requirement: A click on a section header toggles it exactly as `Space` does

`Action::Click(Target::Section(key))` SHALL fold `key`'s section when it is open and unfold
it when it is collapsed, and SHALL move `selected` to that section's header — the same two
effects `Action::ToggleSection` produces for a cursor already addressing that header, and
through the same code, so the two can never diverge.

Every consequence `Space` carries SHALL carry here too: opening an archived section whose
`changes.archived` is empty while `changes.archived_total` is greater than zero SHALL set
`refresh.requested`, through the same blanket rule `apply` runs after every action; a
resolved section SHALL cost no further cycle; and a section toggled by click SHALL survive
every `adopt` exactly as one toggled by key does.

A click on a header SHALL be inert when that header is not in `targets()` — a section whose
count is zero draws no header, so there is nothing to have clicked.

#### Scenario: Click and `Space` produce equal dashboards

- **WHEN** a dashboard with three active and three archived changes is driven once by
  `Action::Click(Target::Section(SectionKey::Archived))` and once by moving the cursor to
  the archived header and applying `Action::ToggleSection`
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** the same holds for the active section, and for a second toggle that unfolds each
  again

#### Scenario: Clicking open an unresolved archive requests the refresh that resolves it

- **WHEN** the archived section is collapsed, `changes.archived` is empty,
  `changes.archived_total` is `22`, and `Action::Click(Target::Section(SectionKey::Archived))`
  is applied
- **THEN** the section is open, `selected` addresses its header, and `refresh.requested` is
  `true`
- **AND** applying the same action again folds it, and `refresh.requested` is not set a
  second time by the fold

#### Scenario: A click on a header that is not drawn is inert

- **WHEN** a dashboard with active changes and no archived ones at all receives
  `Action::Click(Target::Section(SectionKey::Archived))`
- **THEN** no field changes, and `sections.collapsed` in particular is untouched

### Requirement: A second click on the selected change opens its detail

`Action::Click(Target::Change(i))` SHALL set `route` to `Route::Detail` and reset
`detail.scroll` to `0` when `selected` already addresses that target and `route` is
`Route::List` — exactly what `Action::OpenDetail` does — and SHALL change nothing when
`route` is already `Route::Detail`.

The rule SHALL be the same above and below the breakpoint. Above it the detail region is
drawn at both routes, so the visible effect of the second click is that `j`, `k`, and the
arrows begin scrolling the content instead of moving the selection; below it the second
click is what replaces the list region with the detail region.

A section header SHALL NOT open on a second click, on the same terms `Enter` on a header
does nothing: there is no detail for a section, and a second click folds it again instead.

#### Scenario: The second click opens and the third does nothing

- **WHEN** a dashboard at `Route::List` at 60x20 receives `Action::Click(Target::Change(1))`
  three times in a row
- **THEN** after the first, `selected` addresses that target and `route` is `Route::List`
- **AND** after the second, `route` is `Route::Detail` with `detail.scroll` `0`
- **AND** after the third, nothing has changed since the second

#### Scenario: The second click matches `Enter` exactly

- **WHEN** a dashboard whose cursor is already on `Target::Change(1)` is driven once by
  `Action::Click(Target::Change(1))` and once by `Action::OpenDetail`
- **THEN** the two resulting `Dashboard` values are equal, field for field

#### Scenario: Two clicks on a section header fold and unfold it

- **WHEN** `Action::Click(Target::Section(SectionKey::Active))` is applied twice
- **THEN** the section is folded and then unfolded, and `route` is `Route::List` throughout
