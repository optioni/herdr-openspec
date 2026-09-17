## REMOVED Requirements

### Requirement: The open help overlay captures the mouse

**Reason**: The requirement now governs two panels rather than one, and one row of its mapping
table becomes wrong rather than merely narrow: a click outside the band resolved to
`Action::ToggleHelp`, which with a second panel sharing the layer means *swap to the help
panel* rather than *close*. A header naming the help overlay alone would leave that row
looking correct.

**Migration**: Replaced by the ADDED requirement below. Every wheel row, every `Ignore` row,
the outside-the-frame rule, and the pointer-motion exemption carry forward unchanged; the
dismissal row becomes `Action::Back`, and the inside-the-band row gains the settings panel's
`Click(Target::Setting(i))`.

## ADDED Requirements

### Requirement: An open overlay captures the mouse

While `dashboard.overlay.panel` is `Some`, `ui::driver::mouse_action` SHALL resolve every mouse
event against the open panel's band and SHALL NOT consult `ui::layout::zone`. This
requirement **takes precedence** over "The wheel scrolls the region under the pointer" and
over "A left click selects a row, opens it, toggles a section, switches a tab, or arms a
selection" — the live header since `text-selection` — for as
long as the overlay is open; both of those resolve the pointer to a region, and while a
modal covers the body there is no region under the pointer to resolve it to.

`mouse_action` SHALL compute the band with `ui::layout::overlay_band(body, content_rows)`
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
| `Down(Left)` **inside** the band, help panel, rule rows included | `Action::Ignore` |
| `Down(Left)` on a **setting row** inside the band, settings panel | `Action::Click(Target::Setting(i))` |
| `Down(Left)` elsewhere inside the band, settings panel | `Action::Ignore` |
| `Down(Left)` **outside** the band but **inside the frame** — above it, below it, or on the footer row | `Action::Back` |
| `Down(Left)` **outside the frame** | `Action::Ignore` |
| `Down` of any other button, anywhere | `Action::Ignore` |
| `Moved` or `Drag`, anywhere | `Action::Ignore` |

The wheel SHALL scroll the overlay from **anywhere in the frame**, not only from over the
band. While a modal is open the reader's attention is the modal, and a wheel over the two
rows of footer or margin that are not the band doing nothing would read as a dead pointer.
`apply` routes both actions to `overlay.scroll` while the overlay is open, so the two regions
beneath cannot be scrolled by a gesture the reader cannot see the effect of.

A click **inside** the band SHALL do nothing **while the help panel is open**. That panel is
read-only and holds no control: no row is a button, no key row is clickable, and there is
nothing inside it a click could mean.

While the **settings** panel is open a click inside the band SHALL resolve to
`Action::Click(Target::Setting(i))` when it lands on either of a setting's two rows — its
value row or its source row — where `i` is that setting's index among the three. `apply` SHALL
move the row cursor to `i` and do nothing else: a click selects a setting, it does **not**
begin an edit, on exactly the list region's terms, where a first click selects a row and only
a second opens it. A click on the heading row, on either rule row, or on a row the panel left
blank SHALL be `Action::Ignore`.

The **wheel** SHALL be `Action::Ignore` while an edit is in progress, at every point in the
frame. Outside an edit it keeps its rows above, scrolling the panel from anywhere. This is the
same rule one step out as the click rule below: `Next`/`Prev` step the candidate value while an
edit is open, so a wheel roll anywhere on screen would otherwise change which agent kind is
about to be committed, with no pointer anywhere near the row.

A click SHALL NOT begin, commit, or cancel an edit. Editing is `Enter` and `Esc`, and a
pointing gesture that could commit a value by landing one row off is a gesture the reader
cannot undo — `agent_kind` is the setting that decides which process a later `a` starts.

A click **outside** the band SHALL dismiss, which is the convention every modal the reader
has met elsewhere follows. It SHALL resolve to `Action::Back` and **not** to
`Action::ToggleHelp`, which is this change's one correction to the landed table: with two
panels sharing one layer, `ToggleHelp` no longer means "close" — pressed while the settings
panel is open it *swaps to the help panel*, so a click outside the settings band would have
opened the help overlay instead of dismissing anything. `Back` closes whichever panel is open
and is the same action `Esc` already produces, so the click and the key agree by construction
rather than by two tables kept in step.

It dismisses and does nothing else in the same event: `Back` is one action, `run_loop` applies
one action per event, and the click that dismissed the overlay SHALL NOT also select the row
it landed on. A reader dismissing a modal is not also choosing what is under it.

While an **edit is in progress**, a click outside the band SHALL cancel the edit and leave the
panel open, because that is what `Back` does there. One gesture dismisses one layer, which is
the rule `Esc` already follows through the overlay, the filter, and the route.

A click **outside the frame** SHALL be `Action::Ignore`, not a dismissal, on exactly the
reasoning the wheel row already states: a point outside the frame is outside the band too, but
it is not a gesture the pane received. This matches the landed table's own `Ignore` row, which
names "or outside the frame" among the points it covers, and `mouse_action`'s existing
`Zone::Outside => Action::Ignore` arm on every event kind. An earlier draft of the table above
put "or outside the frame" on the **dismissal** row, which contradicted both — and contradicted
the wheel scenario three paragraphs below it, which returns `Ignore` at column 200 for that
exact reason. No scenario pinned either reading, so the disagreement would have reached
`mouse_action` as an implementer's coin-flip; the scenario below now pins it.

`Moved` and `Drag` SHALL keep costing no frame at all: `run_loop`'s existing
pointer-motion exemption is unchanged, and the overlay does not make a motion event
interesting.

#### Scenario: The wheel scrolls the overlay from every region

- **WHEN** a dashboard with `overlay.panel` `Some(Panel::Help)`, six active changes, and a twenty-line
  artifact is drawn at 120x40, and `mouse_action` is called with `ScrollDown` at column 10
  (over the list region), at column 80 (over the detail region), at column 40 (the divider
  column), at row 39 (the footer row), and at column 200 — past the frame's right edge
- **THEN** the first four return `Action::ScrollDown` and the fifth returns
  `Action::Ignore`, since a point outside the frame is outside the band too but is not a
  gesture the pane received
- **AND** applying any of the first four advances `overlay.scroll` and leaves `selected` and
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
  one before it — `overlay.panel` still `Some(Panel::Help)`, `overlay.scroll` unchanged

#### Scenario: A click outside the band dismisses it and selects nothing

- **WHEN** a dashboard with `overlay.panel` `Some(Panel::Help)`, `selected` `2`, and `route` `List` is drawn
  at 120x60 — where the band is 44 rows centred in a 59-row body, so rows 0 through 6 and
  rows 51 through 58 are outside it — and `mouse_action` is called with
  `Down(MouseButton::Left)` at row 4 over a change row, then at row 55, then at row 59
  (the footer row)
- **THEN** every one of the three returns `Action::Back` — **not** `Action::ToggleHelp`,
  which this requirement's own mapping table corrects: with two panels sharing one layer,
  `ToggleHelp` means *swap to help* rather than *close*
- **AND** applying the first sets `overlay.panel` `None`, `overlay.scroll` `0`, and leaves
  `selected` at `2` — the click that dismissed the overlay did not also move the cursor to
  the row under it
- **AND** a second identical click, now that the overlay is closed, returns
  `Action::Click(Target::Change(…))` per the click table this requirement took precedence
  over, so the precedence is conditional on `overlay.panel` being `Some` and not permanent
- **AND** a fourth call, at column 200 — past the frame's right edge — returns
  `Action::Ignore` and **not** `Action::Back`, so a click the pane never received does
  not dismiss the overlay, on the same terms the wheel scenario above establishes for
  `ScrollDown` at that column

#### Scenario: The band's edges are inside it

- **WHEN** the band at 120x60 occupies rows 7 through 50 and `mouse_action` is called with
  `Down(MouseButton::Left)` at rows 6, 7, 50, and 51, each at column 60
- **THEN** the calls at rows 6 and 51 return `Action::Back` and the calls at rows 7
  and 50 return `Action::Ignore`, so both rule rows belong to the band and the boundary is
  pinned from both sides

#### Scenario: Motion still costs no frame while the overlay is open

- **WHEN** a dashboard with `overlay.panel` `Some(Panel::Help)` is drawn and `mouse_action` is called with
  `Moved` and then with `Drag(MouseButton::Left)`, each inside the band and again outside
  it
- **THEN** every one of the four returns `Action::Ignore`
- **AND** `run_loop` skips the draw for each, exactly as it does with the overlay closed,
  so moving a pointer across an open overlay costs no frames at all

#### Scenario: Nothing in the overlay is mouse-only

- **WHEN** every gesture this requirement maps to a non-`Ignore` action is collected
- **THEN** it is exactly `ScrollDown`, `ScrollUp`, and the dismissing click
- **AND** each has a key that does the same thing: `j` and the down arrow for
  `ScrollDown`, `k` and the up arrow for `ScrollUp`, and `Esc` for the dismissal —
  `Action::Back` is exactly what `Esc` already produces, which is why the key and the
  gesture agree by construction
- **AND** the overlay is therefore fully operable with no mouse at all, which is the same
  guarantee "Nothing becomes mouse-only" already makes of every other gesture

#### Scenario: A click on a setting row selects it and begins no edit

- **WHEN** a dashboard with the settings panel open and the cursor on setting `0` is drawn at
  120x40, and `mouse_action` is called with `Down(Left)` on the **value** row of the third
  setting and again on its **source** row
- **THEN** both return `Action::Click(Target::Setting(2))`
- **AND** applying either moves the cursor to `2` and leaves no edit in progress
- **AND** a second identical click also leaves no edit in progress, so a click never opens one
- **AND** a click on the heading row, on either rule row, and on a blank row inside the band
  all return `Action::Ignore`
- **AND** the same five calls at 60x20 resolve identically

#### Scenario: A click outside the band dismisses whichever panel is open

- **WHEN** a dashboard with the **settings** panel open is drawn at 120x40 and `mouse_action`
  is called with `Down(Left)` one row above the band, one row below it, and on the footer row
- **THEN** all three return `Action::Back`
- **AND** applying any of them leaves `overlay.panel` `None` and leaves `selected`,
  `detail.scroll`, `route`, and `sections` unchanged, so the dismissal selected nothing
- **AND** the same three calls with the **help** panel open also return `Action::Back` and
  also close it, so one action dismisses both panels
- **AND** `Down(Left)` at column 200, outside the frame, returns `Action::Ignore` at both
  panels, on the reasoning the wheel row already states

#### Scenario: A click outside cancels an edit rather than closing the panel

- **WHEN** a dashboard with the settings panel open and an edit in progress on `agent_kind`
  with candidate `codex` is clicked one row below the band
- **THEN** the action is `Action::Back`, the edit is no longer in progress, and
  `overlay.panel` is still `Some(Panel::Settings)`
- **AND** the committed `agent_kind` is unchanged and no `settings.toml` was written
- **AND** a second such click then closes the panel

