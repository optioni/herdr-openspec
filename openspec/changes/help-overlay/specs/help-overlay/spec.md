## ADDED Requirements

### Requirement: `?` opens and closes a read-only help overlay

The pane SHALL bind `?` to `Action::ToggleHelp`, which opens the help overlay when it is
closed and closes it when it is open. The overlay SHALL be **read-only**: it renders,
and no key pressed while it is open changes anything the pane shows underneath it, reaches
a collaborator, spawns a process, touches the filesystem, or reads a clock.

The overlay SHALL be a **layer**, not a route. `Dashboard::route` SHALL remain an enum of
exactly `List` and `Detail`, and opening the overlay SHALL NOT change it. The precedent is
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
- **THEN** after the first, `help.open` is true and `help.scroll` is `0`
- **AND** after the second, `help.open` is false and `help.scroll` is `0`
- **AND** `route` is `Detail`, `detail.tab` is `2`, `detail.scroll` is `7`, and `selected`
  is `3` after both, so the reader returns to exactly the frame they left
- **AND** `changes`, `filter`, `quit`, `refresh`, `agents`, `agent_names`, `launch`, and
  `sections` are all unchanged, field for field, after both

#### Scenario: `Esc` closes the overlay before any other layer

- **WHEN** a `Dashboard` at `Route::Detail` whose `filter.query` is `add`, whose
  `filter.active` is true, and whose `help.open` is true is given four consecutive `Back`
  actions
- **THEN** after the first, `help.open` is false and `filter.active` is **still true**
  with the query still `add` — the overlay is the outermost layer and `Back` dismisses
  exactly one
- **AND** after the second, `filter.active` is false and `filter.query` is empty; after
  the third, `route` is `List`; after the fourth, nothing has changed
- **AND** `quit` is false after all four

### Requirement: The overlay answers seven actions and every other one is inert

While `help.open` is set, `Dashboard::apply` SHALL dispatch as follows, and this dispatch
SHALL take precedence over the route dispatch and over the filter dispatch alike:

| Action | Effect while the overlay is open |
|---|---|
| `Quit` | quit, exactly as when it is closed |
| `ToggleHelp` | close the overlay and reset `help.scroll` to `0` |
| `Back` | close the overlay and reset `help.scroll` to `0` |
| `Next`, `ScrollDown` | move `help.scroll` down one line |
| `Prev`, `ScrollUp` | move `help.scroll` up one line, saturating at `0` |
| every other action | change nothing at all |

"Every other action" is the closed remainder: `OpenDetail`, `SelectTab`, `NextTab`,
`PrevTab`, `FilterStart`, `FilterPush`, `FilterPop`, `Refresh`, `LaunchApply`,
`LaunchContinue`, `LaunchArchive`, `FocusAgent`, `ToggleSection`, `SelectNext`,
`SelectPrev`, `Click`, and `Ignore`. Seventeen actions, and none of them does anything
while the overlay is open. Seven answer — `Quit`, `ToggleHelp`, `Back`, `Next`, `Prev`,
`ScrollDown`, `ScrollUp` — and seventeen plus seven is the twenty-four `Action` carries after
this change, so the two lists are exhaustive between them with nothing counted twice.

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
  `help.open` true is given `LaunchApply`, `LaunchContinue`, `LaunchArchive`,
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

- **WHEN** a `Dashboard` with `help.open` true is given `Quit`
- **THEN** `quit` is true
- **AND** the same holds for a dashboard whose `help.open` is true and whose
  `filter.active` is also true, so no combination of layers traps the reader

#### Scenario: The overlay swallows the seventeen inert actions

- **WHEN** a `Dashboard` at `Route::List` with six active changes, `selected` `2`,
  `detail.tab` `1`, and `help.open` true is given `OpenDetail`, `SelectTab(3)`,
  `NextTab`, `PrevTab`, `FilterStart`, `FilterPush('a')`, `FilterPop`, `ToggleSection`,
  `SelectNext`, `SelectPrev`, and `Click(Target::Change(0))` in turn
- **THEN** `route` is still `List`, `selected` is still `2`, `detail.tab` is still `1`,
  `filter.query` is still empty, `filter.active` is still false, and `sections` is
  unchanged
- **AND** `help.open` is still true and `help.scroll` is still `0` after all eleven

#### Scenario: `j` and `k` scroll the overlay rather than the frame beneath

- **WHEN** a `Dashboard` at `Route::Detail` with `detail.scroll` `4`, `selected` `1`, and
  `help.open` true is given `Next`, `Next`, `Next`, then `Prev`
- **THEN** `help.scroll` is `3` after the third and `2` after the fourth
- **AND** `detail.scroll` is still `4` and `selected` is still `1` after all four
- **AND** the same dashboard at `Route::List` given the same four actions moves
  `help.scroll` identically and leaves `selected` at `1`, so the overlay's scroll is
  route-agnostic where `Next` and `Prev` are not
- **AND** `Prev` applied to a dashboard whose `help.scroll` is `0` leaves it `0` rather
  than underflowing

#### Scenario: The overlay lists the agent keys when the socket is unreachable

- **WHEN** a `Dashboard` with `agents.reachable` **false** and `help.open` true is rendered at
  120x40 and at 60x20, and again with `agents.reachable` **true**
- **THEN** the two bands are byte-identical, cell for cell, style included, at both widths
- **AND** both hold the `Agents` group with all four of `a`, `c`, `s`, and `g` and their
  descriptions
- **AND** this is the scenario the change's accepted footer cost rests on: the footer drops
  `a/c/s launch` and `g focus` when the socket is unreachable and drops `g focus` at 60 columns
  when it is reachable, and the argument for accepting both is that the overlay lists them
  anyway. `INVENTORY` is `'static` and no render path consults `agents.reachable`, which is what
  makes that true rather than hoped for

### Requirement: The overlay is a full-width band, vertically centred in the body

`ui::help::render(frame, body, help)` SHALL draw the overlay into a rectangle
`ui::layout::help_band(body, content_rows)` computes, where `content_rows` is the number
of rows `ui::help`'s own row grammar produces for the inventory. The rectangle SHALL be:

- `x` and `width` equal to the **body's own**, so the band runs the full width of the
  region area;
- `height` equal to `min(content_rows + 2, body.height)` — the two being the band's top
  rule row and bottom rule row;
- `y` equal to `body.y + (body.height - height) / 2`, so the band is vertically centred
  and any odd remaining row falls below it.

It is a **band and not a box** for two reasons, both of which the repository has already
argued once. `pane-chrome` removed every border in the pane and replaced each with a
heading row and a rule; a boxed overlay would reintroduce the one construct that change
removed. And a box needs four corner glyphs — `┌`, `┐`, `└`, `┘` — every one of which is
East Asian **Ambiguous** and would widen the uncompensated CJK-locale exposure `SPEC.md`
records for the seven glyphs already drawn. A band spends `─` alone, which is already in
that set as the thematic break, and adds no glyph class at all.

The overlay SHALL be drawn over the **body** only. The footer row SHALL remain visible
and SHALL keep rendering its hints, so the reader can see `? help` and `q quit` while the
overlay is open and so the pane is never a frame with no way out named on it.

The band SHALL be drawn **after** the body, over whatever the list and detail regions
painted, and every interior cell it covers SHALL be painted — with a space where the row
has no text — so no character of the frame beneath shows through a gap.

#### Scenario: The band's geometry at both mandated widths

- **WHEN** a dashboard with `help.open` true is rendered into a `TestBackend` at 120x40
  and again at 60x20
- **THEN** at 120x40 the body is rows 0 through 38 and the footer is row 39; the band's
  `x` is `0` and its `width` is `120`
- **AND** at 60x20 the body is rows 0 through 18 and the footer is row 19; the band's
  `x` is `0` and its `width` is `60`
- **AND** at both, the band's height is `min(content_rows + 2, body.height)` and its
  first row is `body.y + (body.height - height) / 2`
- **AND** at both, row 39 and row 19 respectively still read the footer's hints, so the
  overlay covered the body and not the frame

#### Scenario: The band paints every cell it covers

- **WHEN** a dashboard carrying six active changes, an archived section, and a selected
  change whose artifact content is twenty lines of markdown is rendered at 120x40 with
  `help.open` false, and then the identical dashboard with `help.open` true
- **THEN** no cell inside the band's rectangle holds a character from any change name or
  artifact line present in the first buffer — asserted by collecting the band's rows as
  strings and requiring each change name and each artifact line to appear in none of them
- **AND** every cell **outside** the band's rectangle is byte-identical between the two
  buffers, style included, so opening the overlay changed nothing but the rows it covers

#### Scenario: The frame beneath is unchanged when the overlay closes

- **WHEN** a dashboard is rendered at 120x40, then given `ToggleHelp` and rendered, then
  given `ToggleHelp` again and rendered
- **THEN** the first and third buffers are byte-identical, cell for cell, style included

### Requirement: The overlay's row grammar is a rule row, groups, and a rule row

The band's first row SHALL be `─ Help ` followed by `─` to the band's right edge, with
`Help` in `palette::Role::RegionHeadingFocused` and every `─` in
`palette::Role::RegionRule`.

Its last row SHALL be `─` repeated to the band's width, in `palette::Role::RegionRule`,
carrying the position indicator below when the content does not fit.

Between them, the band's **interior** — `height - 2` rows — SHALL render a window onto
the row sequence the inventory produces, which is, in order and for each group:

1. a **group heading** row: the group's `title`, and, when the group's `scope` is not
   `Any`, a space and the scope in parentheses — `Changes (list route)`,
   `Artifact (detail route)`, `While filtering (filter mode)` — in
   `palette::Role::RegionHeadingFocused`;
2. one **binding** row per binding: two spaces, then `input` padded with spaces to the
   key column, then two spaces, then `description`. `input` SHALL be in
   `palette::Role::Strong` and `description` in `palette::Role::ListRow`;
3. a **blank** row after every group but the last.

The **key column** SHALL be the display width of the widest `input` in the whole
inventory, measured by `ui::layout::columns` and never by a `char` count, so the
descriptions of every group align in one column. A binding row whose text exceeds the
band's width SHALL be truncated by `ui::layout::truncate_columns`, never sliced by byte
or by `char`.

For the inventory `binding-inventory` mandates — six groups, thirty-one bindings —
`content_rows` is therefore **42**: thirty-one binding rows, six heading rows, and five
blanks.

#### Scenario: The grammar renders at 120 columns

- **WHEN** a dashboard with `help.open` true is rendered at 120x40 and the band's rows
  are read out of the buffer
- **THEN** the band's first row begins `─ Help ` and every remaining column of it is `─`
- **AND** the first interior row reads `Changes (list route)`, padded with spaces to the
  band's width
- **AND** the next five interior rows each begin with two spaces, carry an `input` from
  the `Changes` group, and carry that binding's `description` beginning at the same
  column as every other binding row's description in the buffer
- **AND** the row after those five is entirely spaces, and the one after it reads
  `Artifact (detail route)`
- **AND** the `Agents`, `Pane`, and `Mouse` headings carry no parenthesised scope,
  because their groups' `scope` is `Any`

#### Scenario: The grammar renders at 60 columns

- **WHEN** the same dashboard is rendered at 60x20
- **THEN** the band's first row begins `─ Help ` and is 60 columns wide
- **AND** the interior rows follow the same grammar, with the descriptions beginning at
  the same column as at 120, because the key column is measured from the inventory and
  not from the frame
- **AND** no interior row exceeds 60 columns, and any row whose text would have is
  truncated rather than wrapped, so the buffer's row 19 is still the footer

#### Scenario: The key column is measured in display columns

- **WHEN** the key column is computed over an inventory whose widest `input` is
  `Backspace` at nine columns
- **THEN** every binding row's description begins at column `2 + 9 + 2`, which is `13`
- **AND** the computation calls `ui::layout::columns` and the truncation calls
  `ui::layout::truncate_columns`, so `src/ui/help.rs` carries no `.chars().count()`,
  no `.chars().take(`, and no `Vec<char>` and passes `COLWIDTH`'s sweep

### Requirement: The overlay scrolls when the body cannot hold it

When `content_rows` exceeds the band's interior height, the band SHALL show a **window**
onto the rows, and `help.scroll` SHALL be the first visible row's index.

The offset actually drawn SHALL be recomputed on every draw by
`ui::layout::scroll_offset(content_rows, help.scroll, interior_height)` — the crate's
existing clamp, whose parameter order is `(lines, scroll, height)` — so a `help.scroll`
left out of range by a resize is bounded before the frame is painted rather than after it,
and a held `j` cannot run the window past the last row.

`Dashboard` SHALL gain a **second** normaliser, `normalise_help_scroll(frame_area)`, and
`ui::driver::run_loop` SHALL call it once per frame beside `normalise_scroll(area)`. It
SHALL NOT be folded into `normalise_scroll`, and the reason is a measured contradiction
rather than tidiness: `normalise_scroll` returns early when the detail region is not drawn
— which `detail-scroll` requires of it in as many words — and the detail region is not
drawn at `Route::List` below the breakpoint, which is precisely the 60x20 fixture the
held-key scenario below uses. Sharing the function would leave `help.scroll` unclamped in
the one case the scenario exists to pin. The two also read different geometry: the band is
computed from the **body**, the detail clamp from the detail region's content area.

`normalise_help_scroll` SHALL clamp `help.scroll` whenever the overlay is open, at either
route and at either side of the breakpoint, and SHALL change nothing when it is closed.
`help.scroll` is a user-controlled position and not derived geometry, and it SHALL NOT be
stored as a row count or a page number.

The **position indicator** SHALL be drawn into the band's bottom rule row when, and only
when, `content_rows > interior_height`: the text `<first>-<last>/<total>`, where `first`
is the offset plus one, `last` is `first + interior_height - 1`, and `total` is
`content_rows`, placed so its final character is one column in from the band's right
edge, in `palette::Role::ListSeparator`.

It SHALL carry **no arrow glyphs**. `▲` and `▼` are East Asian Ambiguous and would widen
the uncompensated CJK-locale exposure `SPEC.md` records, for information the numbers
already carry: `1-37/42` says both that there is more below and exactly how much.

When the band is too **narrow** to hold an indicator — a band whose width is under the
indicator's own display width plus two — the indicator SHALL be omitted and the bottom
rule SHALL be drawn whole. The content is still reachable by scrolling; a clipped
indicator would not be.

#### Scenario: The overlay scrolls at both mandated sizes

- **WHEN** a dashboard with `help.open` true is rendered at 120x40, where the body is 39
  rows, the band is 39 rows, and its interior is 37 against 42 content rows
- **THEN** the bottom rule row's final columns read `1-37/42`, ending one column in from
  column 119
- **AND** after ten `Next` actions and a redraw the offset is **clamped to 5** — 42
  content rows less a 37-row interior — so the interior's first row is content row 6 and
  the indicator reads `6-42/42`, not the `11-47/42` an unclamped offset of ten would give
- **AND** at 60x20 the body is 19 rows, the band is 19, its interior is 17, and the
  indicator reads `1-17/42`, ending one column in from column 59

#### Scenario: A held key cannot run the window off the end

- **WHEN** a dashboard with `help.open` true is given two hundred consecutive `Next`
  actions at 60x20, redrawing after each
- **THEN** the last interior row is always content row 42 once the window has reached the
  end, and never a blank row past it
- **AND** `help.scroll` is clamped on every frame by
  `ui::layout::scroll_offset(42, help.scroll, 17)` to a maximum of 25, so it is never used
  unbounded — and it is `normalise_help_scroll` that applies it, which is why this
  60x20 `Route::List` fixture clamps at all where `normalise_scroll` would have returned
  early
- **AND** two hundred consecutive `Prev` actions from there return the window to content
  row 1 and leave `help.scroll` at `0` rather than underflowing

#### Scenario: No indicator when the content fits

- **WHEN** a dashboard with `help.open` true is rendered at 120x60, where the body is 59
  rows and the band is `42 + 2 = 44` rows with an interior of 42 against 42 content rows
- **THEN** the band's bottom row is `─` repeated to the band's width with no digits in it
- **AND** every one of the 42 content rows is present in the buffer, so the whole
  inventory is visible in one frame at a tall pane

### Requirement: The overlay degrades rather than panicking at any frame size

`ui::help::render` SHALL be total over every `Rect`, including zero-width, zero-height,
and one-cell rectangles, and SHALL panic for none of them. A pane can be resized to one
row while the overlay is open, and a Herdr split can be narrower than any content the
inventory holds.

- A body of zero width or zero height SHALL render **nothing** and return.
- A body of one row SHALL render the top rule row alone — `─ Help ` truncated to the
  width — with no interior and no bottom rule.
- A body of two rows SHALL render the top rule and the bottom rule with no interior
  between them, and no indicator, since an interior height of zero names no window.
- A body narrower than `─ Help ` SHALL render that row truncated by
  `ui::layout::truncate_columns`, never sliced by byte.

In none of these SHALL the reader be trapped: `q`, `Ctrl-C`, `Esc`, and `?` all still
close the overlay or the pane, because they are handled in `apply` and not in the view.

#### Scenario: Degenerate frames render without panicking

- **WHEN** a dashboard with `help.open` true is rendered at 1x1, 1x2, 2x1, 0x0 — where
  the backend permits it — 120x1, 120x2, 120x3, and 5x10
- **THEN** no render panics
- **AND** at 120x2 the body is one row and holds the top rule alone, truncated to 120
  columns, with the footer on row 1
- **AND** at 120x3 the body is two rows and holds the top rule and the bottom rule with
  no interior and no indicator
- **AND** at 5x10 the top row reads the first five columns of `─ Help `, truncated by
  display column

#### Scenario: The reader is never trapped in a degenerate frame

- **WHEN** a dashboard with `help.open` true at a 1x1 frame is given `Quit`, and a second
  such dashboard is given `Back`, and a third `ToggleHelp`
- **THEN** the first sets `quit`, and the second and third set `help.open` to false
- **AND** none of the three panics, and none depends on anything the view computed, since
  `apply` never receives a `Rect`
