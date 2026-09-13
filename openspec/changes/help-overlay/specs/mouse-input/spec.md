## ADDED Requirements

### Requirement: The open help overlay captures the mouse

While `dashboard.help.open` is set, `ui::driver::mouse_action` SHALL resolve every mouse
event against the overlay's band and SHALL NOT consult `ui::layout::zone`. This
requirement **takes precedence** over "The wheel scrolls the region under the pointer" and
over "A left click selects a row, opens it, toggles a section, or switches a tab" for as
long as the overlay is open; both of those resolve the pointer to a region, and while a
modal covers the body there is no region under the pointer to resolve it to.

`mouse_action` SHALL compute the band with `ui::layout::help_band(body, content_rows)`
from the same `area` it already receives — the area of the frame just drawn — so the
overlay's mouse target is the rectangle the reader is actually looking at, on exactly the
terms `mouse-input` already established for every other gesture. It SHALL remain pure,
total, and free of I/O and clocks.

The mapping SHALL be:

| Where the event lands, while the overlay is open | Action |
|---|---|
| `ScrollDown`, anywhere in the frame | `Action::ScrollDown` |
| `ScrollUp`, anywhere in the frame | `Action::ScrollUp` |
| `ScrollLeft` or `ScrollRight`, anywhere | `Action::Ignore` |
| `Down(Left)` **inside** the band, its two rule rows included | `Action::Ignore` |
| `Down(Left)` **outside** the band — above it, below it, on the footer row, or outside the frame | `Action::ToggleHelp` |
| `Down` of any other button, anywhere | `Action::Ignore` |
| `Moved` or `Drag`, anywhere | `Action::Ignore` |

The wheel SHALL scroll the overlay from **anywhere in the frame**, not only from over the
band. While a modal is open the reader's attention is the modal, and a wheel over the two
rows of footer or margin that are not the band doing nothing would read as a dead pointer.
`apply` routes both actions to `help.scroll` while the overlay is open, so the two regions
beneath cannot be scrolled by a gesture the reader cannot see the effect of.

A click **inside** the band SHALL do nothing. The overlay is read-only and holds no
control: no row is a button, no key row is clickable, and there is nothing inside it a
click could mean.

A click **outside** the band SHALL close the overlay, which is the convention every modal
the reader has met elsewhere follows. It closes it and does nothing else in the same
event: `Action::ToggleHelp` is one action, `run_loop` applies one action per event, and
the click that dismissed the overlay SHALL NOT also select the row it landed on. A reader
dismissing a modal is not also choosing what is under it.

`Moved` and `Drag` SHALL keep costing no frame at all: `run_loop`'s existing
pointer-motion exemption is unchanged, and the overlay does not make a motion event
interesting.

#### Scenario: The wheel scrolls the overlay from every region

- **WHEN** a dashboard with `help.open` true, six active changes, and a twenty-line
  artifact is drawn at 120x40, and `mouse_action` is called with `ScrollDown` at column 10
  (over the list region), at column 80 (over the detail region), at column 40 (the divider
  column), at row 39 (the footer row), and at column 200 — past the frame's right edge
- **THEN** the first four return `Action::ScrollDown` and the fifth returns
  `Action::Ignore`, since a point outside the frame is outside the band too but is not a
  gesture the pane received
- **AND** applying any of the first four advances `help.scroll` and leaves `selected` and
  `detail.scroll` unchanged
- **AND** the same five calls with `ScrollUp` return `Action::ScrollUp` and
  `Action::Ignore` on the same terms, and `ScrollLeft` and `ScrollRight` return
  `Action::Ignore` at every one of the five
- **AND** at 60x20 the same calls resolve identically, so the breakpoint does not change
  what the wheel does while the overlay is open

#### Scenario: A click inside the band does nothing

- **WHEN** the same dashboard is drawn at 120x40 and `mouse_action` is called with
  `Down(MouseButton::Left)` at the band's top rule row, at its first interior row, at its
  last interior row, at its bottom rule row, at its leftmost column, and at its rightmost
  column
- **THEN** every one of the six returns `Action::Ignore`
- **AND** applying `Action::Ignore` leaves the dashboard equal, field for field, to the
  one before it — `help.open` still true, `help.scroll` unchanged

#### Scenario: A click outside the band dismisses it and selects nothing

- **WHEN** a dashboard with `help.open` true, `selected` `2`, and `route` `List` is drawn
  at 120x60 — where the band is 41 rows centred in a 59-row body, so rows 0 through 8 and
  rows 50 through 58 are outside it — and `mouse_action` is called with
  `Down(MouseButton::Left)` at row 4 over a change row, then at row 55, then at row 59
  (the footer row)
- **THEN** every one of the three returns `Action::ToggleHelp`
- **AND** applying the first sets `help.open` false, `help.scroll` `0`, and leaves
  `selected` at `2` — the click that dismissed the overlay did not also move the cursor to
  the row under it
- **AND** a second identical click, now that the overlay is closed, returns
  `Action::Click(Target::Change(…))` per the click table this requirement took precedence
  over, so the precedence is conditional on `help.open` and not permanent

#### Scenario: The band's edges are inside it

- **WHEN** the band at 120x60 occupies rows 9 through 49 and `mouse_action` is called with
  `Down(MouseButton::Left)` at rows 8, 9, 49, and 50, each at column 60
- **THEN** the calls at rows 8 and 50 return `Action::ToggleHelp` and the calls at rows 9
  and 49 return `Action::Ignore`, so both rule rows belong to the band and the boundary is
  pinned from both sides

#### Scenario: Motion still costs no frame while the overlay is open

- **WHEN** a dashboard with `help.open` true is drawn and `mouse_action` is called with
  `Moved` and then with `Drag(MouseButton::Left)`, each inside the band and again outside
  it
- **THEN** every one of the four returns `Action::Ignore`
- **AND** `run_loop` skips the draw for each, exactly as it does with the overlay closed,
  so moving a pointer across an open overlay costs no frames at all

#### Scenario: Nothing in the overlay is mouse-only

- **WHEN** every gesture this requirement maps to a non-`Ignore` action is collected
- **THEN** it is exactly `ScrollDown`, `ScrollUp`, and the dismissing click
- **AND** each has a key that does the same thing: `j` and the down arrow for
  `ScrollDown`, `k` and the up arrow for `ScrollUp`, and `Esc` and `?` for the dismissal
- **AND** the overlay is therefore fully operable with no mouse at all, which is the same
  guarantee "Nothing becomes mouse-only" already makes of every other gesture
