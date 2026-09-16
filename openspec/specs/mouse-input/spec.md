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

Every action the mouse can produce SHALL remain reachable by key, and **no key SHALL change
its meaning because of a mouse binding**. `Action::SelectNext`, `Action::SelectPrev`,
`Action::ScrollDown`, and `Action::ScrollUp` are what `j`, `k`, and the arrows already reach
through `Action::Next` and `Action::Prev` at the matching route;
`Action::Click(Target::Section(_))` is what `Space` at `Route::List` already reaches;
`Action::Click(Target::Change(_))` is what `j`/`k` and `Enter` already reach;
`Action::SelectTab(i)` is what `1`–`9`, `[`, and `]` already reach;
and `Action::Click(Target::DetailHeader { .. })` is what `Space` at `Route::Detail` reaches.
`Action::Click(Target::DetailLine(_))` is gone: `text-selection` removes the variant, and the
detail cursor it moved is what `j`/`k` and the arrows always reached directly.

The clause is narrowed from "no key SHALL change its meaning" to "no key SHALL change its
meaning **because of a mouse binding**", and the narrowing is this change's, stated rather
than left as a contradiction. `foldable-spec-sections` does change one key's meaning —
`Space` at `Route::Detail` folds an artifact section instead of a list section, marked
**BREAKING** in its own proposal — and it does so for reasons that have nothing to do with
the mouse. What this requirement guards is the property it was written for: that adding a
gesture never silently rebinds a key, and that the pane stays fully usable with no mouse at
all. Both remain true — every gesture above names the key that already reached it.

The pane SHALL therefore stay fully usable in a terminal that reports no mouse event at
all, including over SSH, with no feature reachable only by pointer.

**`text-selection` adds the one exemption this requirement has ever had, and it is pinned
rather than reasoned about at each call site.** `Action::Select` has no key that produces
the same effect, and cannot: selecting a span of rendered text is a pointing gesture, and
the reader's only other route to it — the terminal's own `Shift`+drag — is a pointer gesture
too. What this requirement was written to protect is untouched: every *function of the pane*
stays reachable by key, and getting text out of the pane was not a function of the pane
before this change. The exemption SHALL be asserted **by name and by length**, exactly as
`tests/doc_contract.rs` asserts `EXEMPT_ACTIONS`, so a second mouse-only action costs a spec
change rather than passing under a predicate.

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

#### Scenario: `Action::Select` is the only mouse-only action, by name and count

- **WHEN** every action `ui::driver::mouse_action` can produce is swept and each is checked
  for a key at any route that produces the same `Dashboard` change
- **THEN** exactly one has none, and it is `Action::Select`
- **AND** the mouse-only set is asserted to hold that one name and to have length one, so a
  second mouse-only action fails `cargo test`
- **AND** every other gesture still names the key that already reached it, so the pane
  remains fully usable with no mouse in a terminal reporting none

#### Scenario: The pane is still complete without a pointer

- **WHEN** a dashboard is driven through a full session — list, filter, detail, tabs, folds,
  agent keys and the help overlay — using keys only
- **THEN** every route, every artifact and every fold state is reachable
- **AND** the only thing unavailable is copying text, which has no pane function behind it

### Requirement: `SPEC.md` names the mouse bindings and the drag-to-select cost

`SPEC.md` → Keys SHALL carry a mouse table naming each binding this capability defines —
the wheel over each region, the click on a change row, the second click, the click on a
list section header, the click on a tab cell, the click on an artifact-section header, and
the click on any other content row of a foldable artifact: **seven**, five before
`foldable-spec-sections` — and SHALL state that enabling mouse capture costs the terminal's
own drag-to-select **outside the detail content area**.

**The bypass sentence is corrected here, because this change measured it and it was wrong.**
`Option`+drag was observed **not** to restore selection under any mode set in Ghostty on
macOS, while `Shift`+drag restored it under every one. `SPEC.md` SHALL name `Shift`, SHALL
name the terminal it was measured on, and SHALL NOT claim the `Option` bypass at all — it is
an iTerm2 and Terminal.app convention this project has no measurement for. iTerm2,
Terminal.app and the Linux terminals are unmeasured and the document SHALL say so.

`SPEC.md` SHALL further record that inside the detail content area the cost no longer
applies, because the pane does the selecting itself, and that the mouse table gains a drag
row. The measured facts it SHALL NOT contradict: narrowing the enabled DEC mode set does
not restore plain drag-selection, `?1003` is load-bearing because drag motion is what the
selection consumes, and Herdr is not involved.

`AGENTS.md`'s terminal-seam rule SHALL name the two capture commands alongside the four
terminal-mode functions it already lists, so the confined set the `NORAW-GREP` gate enforces
and the set the document claims are the same set.

A doc-conformance test SHALL bind both claims to the files that determine them, so a binding
added, removed, or renamed in `ui::driver::mouse_action` without the document following
fails `cargo test`.

**What that test's subject actually is, stated because it is narrower than it reads:** the
leg extracts the backticked `Action::<Variant>` names from the table and compares that set
against `mouse_action`'s body, as a **set equality**.

**This change inverts the note that stood here.** `foldable-spec-sections` recorded that its
two new gestures resolved to `Action::Click` and `Action::Ignore`, both already named, so the
leg was green before and after and could not fail for its new rows. That is **not** true here:
`text-selection`'s gesture resolves to `Action::Select`, a name the table does not carry, so
the leg goes **red** the moment `mouse_action` can produce it and stays red until
`SPEC.md` → Keys' `| Gesture | Action |` table names it. The leg is therefore this change's
own proof rather than something needing a separate negative control — though the control
`foldable-spec-sections` recorded still stands for the rows it added.

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

#### Scenario: The documented bypass names what was measured

- **WHEN** `tests/doc_contract.rs` reads `SPEC.md` → Keys' mouse table and its drag-to-select
  paragraph
- **THEN** the paragraph names `Shift` and names the terminal it was measured on
- **AND** the literal `Option` appears nowhere in it as a claimed bypass, so the falsified
  sentence cannot be reintroduced without failing `cargo test`
- **AND** the mouse table carries a drag row whose action is `Action::Select`

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
| `Down(Left)` **outside** the band but **inside the frame** — above it, below it, or on the footer row | `Action::ToggleHelp` |
| `Down(Left)` **outside the frame** | `Action::Ignore` |
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
  at 120x60 — where the band is 44 rows centred in a 59-row body, so rows 0 through 6 and
  rows 51 through 58 are outside it — and `mouse_action` is called with
  `Down(MouseButton::Left)` at row 4 over a change row, then at row 55, then at row 59
  (the footer row)
- **THEN** every one of the three returns `Action::ToggleHelp`
- **AND** applying the first sets `help.open` false, `help.scroll` `0`, and leaves
  `selected` at `2` — the click that dismissed the overlay did not also move the cursor to
  the row under it
- **AND** a second identical click, now that the overlay is closed, returns
  `Action::Click(Target::Change(…))` per the click table this requirement took precedence
  over, so the precedence is conditional on `help.open` and not permanent
- **AND** a fourth call, at column 200 — past the frame's right edge — returns
  `Action::Ignore` and **not** `Action::ToggleHelp`, so a click the pane never received does
  not dismiss the overlay, on the same terms the wheel scenario above establishes for
  `ScrollDown` at that column

#### Scenario: The band's edges are inside it

- **WHEN** the band at 120x60 occupies rows 7 through 50 and `mouse_action` is called with
  `Down(MouseButton::Left)` at rows 6, 7, 50, and 51, each at column 60
- **THEN** the calls at rows 6 and 51 return `Action::ToggleHelp` and the calls at rows 7
  and 50 return `Action::Ignore`, so both rule rows belong to the band and the boundary is
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

### Requirement: A left click selects a row, opens it, toggles a section, switches a tab, or arms a selection

A `MouseEventKind::Down(MouseButton::Left)` SHALL be resolved by where it lands:

| Where the press lands | Action |
|---|---|
| A drawn change row in the list region's interior, other than the selected one | `Action::Click(Target::Change(i))` for that row's own `visible()` index |
| The drawn change row that already carries the cursor | `Action::Click(Target::Change(i))` for the same index |
| A drawn section-header row in the list region's interior | `Action::Click(Target::Section(key))` for that header's own key |
| A drawn tab cell in the detail region's tab-bar row | `Action::SelectTab(i)` for that cell's own artifact position |
| A drawn artifact-section header row in the detail region's content area, when the selected artifact is foldable | `Action::Click(Target::DetailHeader { line, section })` for that row's own content-line index and section index |
| Any other drawn row of the detail region's content area, foldable or not | `Action::Select` at its arming phase, for that row's own content-line index and display column |
| A problem row, a message row, an interior row past the last drawn row, a region's gutter, heading row or padding row, the divider, the detail region's rule row or content padding row, the detail content area below its last drawn line, the frame footer, or outside the frame | `Action::Ignore` |

`Target` SHALL carry exactly **one** variant for this, `text-selection` having removed the
other:

```rust
Target::DetailHeader { line: usize, section: usize },
```

The tracked-tasks tab is no longer excluded from those two rows. Before
`heading-sections` it was never foldable at any section count, so every press in its content
area resolved to `Action::Ignore`; now a task file carrying a heading and an item splits into
sections, the tab is foldable like any other, and a press on a group heading folds that group.
Nothing in this table changed to allow it — the table was already written in terms of
`Detail::foldable()`, and that predicate simply started answering `true` for one more tab.

Both carry indices **already resolved against the frame just drawn**. `mouse_action` has the
content area's width and can call `ui::detail::content_lines` and
`ui::detail::section_at`; `Dashboard::apply` has neither and SHALL NOT recompute either. That
is why the header variant carries its line index beside its section index rather than leaving
`apply` to derive one from the other.

`Dashboard::apply(Action::Click(target))` SHALL:

- do nothing at all when `target` is a `Target::Change` or a `Target::Section` that is not
  present in `targets()`;
- when `target` is `Target::Section(key)`, move the cursor to that header and fold the
  section if it is open, unfold it if it is collapsed — exactly what `Action::ToggleSection`
  at `Route::List` does for a cursor already on that header, so a click and a `Space` on the
  same header are indistinguishable in their effect;
- when `target` is `Target::Change(i)` and the cursor is **not** already on that row, move
  the cursor to it and reset `detail.tab` and `detail.scroll` to zero, exactly as a
  selection move by `j` or `k` does;
- when `target` is `Target::Change(i)`, the cursor is already on that row, and the route is
  `Route::List`, set the route to `Route::Detail` and reset `detail.scroll` to zero —
  exactly what `Enter` does — so a second click on a selected row opens it;
- when `target` is `Target::Change(i)`, the cursor is already on that row, and the route is
  already `Route::Detail`, change nothing;
- when `target` is `Target::DetailHeader { line, section }`, set `detail.scroll` to `line`
  and then toggle `section` through the very code `Action::ToggleSection` at `Route::Detail`
  runs, so a click and a `Space` on the same artifact header can never diverge — including
  that arm's own rule, stated in `artifact-folds`, that `detail.scroll` ends on the toggled
  section's header row;
- when `target` is `Target::DetailHeader` and the selected artifact is not foldable, change
  nothing at all: `mouse_action` does not emit it there, and `apply` SHALL be inert rather
  than trusting it, on the same terms it checks `targets()` for the other two. A **press** in
  that same non-foldable content area is not inert — it arms a selection, per
  `text-selection` — because a single-section artifact is a whole rendered document.

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
  cursor to that header and applying `Action::ToggleSection` at `Route::List`
- **AND** a second click on the same header unfolds it again

#### Scenario: A click on an archived header opens an unresolved archive and requests its refresh

- **WHEN** a dashboard whose archived section is collapsed, whose `changes.archived` is
  empty, and whose `changes.archived_total` is `22` is drawn at 120x40, and a left press
  lands on the `archived` header row
- **THEN** applying the returned action opens the section
- **AND** `dashboard.refresh.requested` is `true`, so the click that opened the archive is
  what asks for its resolution, on the same terms `Space` does

#### Scenario: A click on an artifact-section header folds it exactly as `Space` does

- **WHEN** a dashboard whose selected artifact resolves to three spec files, every section
  collapsed, is drawn at 120x40 at `Route::Detail`, and a left press lands on the second
  content row
- **THEN** `mouse_action` returns `Action::Click(Target::DetailHeader { line: 1, section: 1 })`
- **AND** applying it puts `1` in `detail.expanded` and leaves `detail.scroll` at `1`
- **AND** the resulting `Dashboard` is equal, field for field, to one produced by setting
  `detail.scroll` to `1` and applying `Action::ToggleSection` at `Route::Detail`
- **AND** a second click on the same row folds it again, and `sections.collapsed` and
  `selected` are unchanged throughout
- **AND** the same press at `Route::List` on a 120x40 frame — where the wide layout draws
  both regions — returns and applies the same action, so the detail region's headers are
  clickable at either route

#### Scenario: A click in an open section's body arms a selection and folds nothing

- **WHEN** the same dashboard with `detail.expanded` holding `0` is drawn at 120x40 and a
  left press lands on the row carrying that section's third rendered body line
- **THEN** `mouse_action` returns `Action::Select` at its arming phase for that row's own
  content-line index and column, and no `Target::DetailLine` — the variant is gone
- **AND** applying it leaves `detail.scroll`, `detail.expanded`, `route`, `selected`, and
  `detail.tab` unchanged, so the cursor no longer jumps to the clicked line
- **AND** a second press at that same cell selects the word under it, per `text-selection`

#### Scenario: A non-foldable tab's content is selectable too

- **WHEN** a dashboard whose selected artifact resolves to one path holding a twenty-item
  list is drawn at 120x40 at `Route::Detail`, and left presses land on the first, fifth, and
  last drawn content rows
- **THEN** every call returns `Action::Select` at its arming phase, not `Action::Ignore`
- **AND** a second press at any of those cells selects the word under it
- **AND** this is a deliberate departure: `detail_row_click` short-circuited on
  `!detail.foldable()` and returned `Action::Ignore`, which selection must **not** inherit —
  a single-section artifact is a whole rendered document and exactly the thing a reader
  wants to copy out of

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
  content area — which holds no foldable artifact, because no change is selected — on the
  frame's footer, and at column 200
- **THEN** every call returns `Action::Ignore`
- **AND** applying `Action::Ignore` leaves the dashboard equal to what it was
- **AND** a press on a detail content row **below** the last drawn line of a foldable
  artifact returns `Action::Ignore` too, so an empty region under three header rows is not a
  fourth section

#### Scenario: The other buttons and the non-press kinds are inert

- **WHEN** `mouse_action` is called over a drawn change row with `Down(Right)`,
  `Down(Middle)`, `Up(Left)`, `Drag(Left)`, and `Moved`, and again over a drawn
  artifact-section header row with the same five
- **THEN** every call returns `Action::Ignore`
- **AND** in particular no press of any button reaches `Action::LaunchApply`,
  `Action::LaunchContinue`, `Action::LaunchArchive`, or `Action::FocusAgent`, so a
  mis-click cannot start or focus an agent

#### Scenario: A click on a task group's header folds that group

- **WHEN** a dashboard at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true` and whose file reads
  `## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [ ] b\n` is drawn at 120x40 and at 60x40,
  and a left press lands on the `> 1. Done` header row
- **THEN** `mouse_action` returns `Action::Click(Target::DetailHeader { line, section })` for
  that row's own content-line index and a `section` of `0`
- **AND** applying it opens that group, sets `detail.scroll` to the group's header row, and
  leaves `route`, `selected`, and `detail.tab` unchanged
- **AND** a left press on the progress-bar row or on its blank line returns `Action::Select`
  at its arming phase for that row's own index and column, per the table above: both are
  drawn rows of a content area, and the table sends every drawn row that is not a header
  there. Neither belongs to a section, so applying it arms a selection and
  folds nothing, and `Space` from where it lands is inert
- **AND** the same two presses against a dashboard whose task file holds items but no heading
  — which does not split, so the tab is not foldable — both return `Action::Select` at its
  arming phase, **not** `Action::Ignore`: a headless tracked-tasks file still draws real
  content rows, and design.md -> Decision 8 forbids selection inheriting
  `detail_row_click`'s `!foldable()` short-circuit, so the resolver treats a non-foldable
  tab's rows exactly as it treats a foldable one's body rows. This clause was carried from
  the pre-selection requirement, where `Ignore` was correct because nothing else could
  answer; it is corrected here rather than left to contradict this same delta's
  `specs/text-selection` scenario "A non-foldable tab's content is selectable too"

#### Scenario: A click on a nested scenario header folds only that scenario

- **WHEN** a dashboard at `Route::Detail` whose selected artifact resolves to one delta spec
  path, with `detail.expanded` holding the indices of the operation heading and its first
  requirement, is drawn at 120x40 and at 60x40, and a left press lands on the
  `    > Scenario: A works` row
- **THEN** `mouse_action` returns `Action::Click(Target::DetailHeader { line, section })`
  whose `section` is that scenario's own index into `detail.sections`, not its position among
  the drawn rows
- **AND** applying it opens that scenario and leaves every other section's membership of
  `detail.expanded` exactly as it was
- **AND** a left press on one of that scenario's body rows returns `Action::Select` at its
  arming phase, which folds nothing and moves no cursor
