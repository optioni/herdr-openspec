## ADDED Requirements

### Requirement: `ScrollDown` and `ScrollUp` scroll the detail content at either route

`Dashboard::apply(Action::ScrollDown)` SHALL add one to `detail.scroll` and
`Action::ScrollUp` SHALL subtract one, saturating at zero, **regardless of `route`** —
unlike `Next` and `Prev`, which do the same thing only at `Route::Detail`. The wheel names
the region it is over, so the wide layout's detail region scrolls while the list route is
current.

Both SHALL change nothing else: not `selected`, not `detail.tab`, not `route`, not
`filter`, not `changes`, not `agents`, not `agent_names`, not `launch`.

The upper bound SHALL stay where `detail-scroll` already puts it — the draw-time clamp in
`render_detail` and the per-frame `Dashboard::normalise_scroll` — not in `apply`, so a wheel
held down cannot run the stored offset arbitrarily far ahead any more than a held `j` can.

`Action::Next` at `Route::Detail` SHALL be exactly `Action::ScrollDown` and `Action::Prev`
at `Route::Detail` exactly `Action::ScrollUp`, through one shared implementation, so a key
and a wheel over the same region can never disagree about what one line means.

#### Scenario: The wheel scrolls the detail region at the list route

- **WHEN** a dashboard at `Route::List` at 120x40, with a forty-line artifact selected, is
  given `Action::ScrollDown` three times
- **THEN** `detail.scroll` is `3` and `selected` is unchanged
- **AND** the drawn detail region shows the content advanced by three lines while the list
  region still shows the same selected row
- **AND** `route` is still `Route::List`

#### Scenario: `ScrollUp` stops at the top

- **WHEN** `Action::ScrollUp` is applied to a dashboard whose `detail.scroll` is `0`, at
  both routes
- **THEN** `detail.scroll` stays `0` and nothing else changes

#### Scenario: A held wheel is clamped by the frame, not by `apply`

- **WHEN** `Action::ScrollDown` is applied five hundred times to a dashboard whose selected
  artifact renders twelve lines, and the frame is then drawn at 120x40
- **THEN** the drawn content shows the last screenful rather than a blank region
- **AND** after `normalise_scroll`, `detail.scroll` is clamped to the same value a held `j`
  at the detail route leaves behind

#### Scenario: `Next` at the detail route and `ScrollDown` are the same move

- **WHEN** a dashboard at `Route::Detail` is driven once by `Action::Next` and once by
  `Action::ScrollDown`
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** the same holds for `Action::Prev` against `Action::ScrollUp`

### Requirement: `SelectNext` and `SelectPrev` move the list selection at either route

`Dashboard::apply(Action::SelectNext)` and `Action::SelectPrev` SHALL move and clamp
`selected` per `list-selection` and reset `detail.tab` and `detail.scroll` to `0` exactly
when `selected` changed value — **regardless of `route`**, unlike `Next` and `Prev`, which
do this only at `Route::List`.

They SHALL change nothing else, and in particular SHALL NOT change `route`: a wheel over
the list region while the detail route is current moves the selection and leaves `j` and
`k` scrolling the content, because nothing about a wheel says the reader wants to change
which region the keys address.

`Action::Next` at `Route::List` SHALL be exactly `Action::SelectNext` and `Action::Prev` at
`Route::List` exactly `Action::SelectPrev`, through one shared implementation.

#### Scenario: The wheel moves the selection at the detail route

- **WHEN** a dashboard at `Route::Detail` at 120x40 with six active changes, `detail.tab`
  `2`, and `detail.scroll` `9` is given `Action::SelectNext`
- **THEN** `selected` has advanced by one, `detail.tab` is `0`, and `detail.scroll` is `0`
- **AND** `route` is still `Route::Detail`, so the detail region now shows the newly
  selected change

#### Scenario: A clamped move resets nothing

- **WHEN** `Action::SelectNext` is applied to a dashboard whose cursor is already on the
  last target, with `detail.tab` `2` and `detail.scroll` `9`
- **THEN** `selected`, `detail.tab`, and `detail.scroll` are all unchanged
- **AND** the same holds for `Action::SelectPrev` at the first target

#### Scenario: `Next` at the list route and `SelectNext` are the same move

- **WHEN** a dashboard at `Route::List` is driven once by `Action::Next` and once by
  `Action::SelectNext`
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** the same holds for `Action::Prev` against `Action::SelectPrev`
