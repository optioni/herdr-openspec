## REMOVED Requirements

### Requirement: The overlay answers seven actions and every other one is inert

**Reason**: `text-selection` adds `Action::Select` to the inert remainder, and this
requirement's swallow scenario carried that count in its own name, which is a merge key. It is
re-stated below with the numeral removed from the scenario name, for the reason
`binding-inventory`'s delta gives at greater length.

**Migration**: None. The seven answered actions are unchanged; the inert remainder moves from
seventeen to eighteen, and the two still sum to the whole of `Action`.

## ADDED Requirements

### Requirement: The overlay answers seven actions and ignores every other one

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
`ScrollDown`, `ScrollUp` — and eighteen plus seven is the twenty-four `Action` carries after
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

#### Scenario: The overlay swallows every inert action

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


#### Scenario: `Action::Select` is inert while the overlay is open

- **WHEN** `Dashboard::apply` is called with `Action::Select` at any phase while `help.open`
  is set
- **THEN** nothing changes: no selection is made, extended, or cleared, and nothing is copied
- **AND** the inert list holds eighteen actions and the answered list seven, summing to the
  twenty-five `Action` carries after this change
- **AND** the mouse resolver never produces it there anyway, because a drag over the band
  already resolves to `Action::Ignore`

## MODIFIED Requirements

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

For the inventory `binding-inventory` mandates — six groups, thirty-two bindings —
`content_rows` is therefore **43**: thirty-two binding rows, six heading rows, and five
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
already carry: `1-37/43` says both that there is more below and exactly how much.

When the band is too **narrow** to hold an indicator — a band whose width is under the
indicator's own display width plus two — the indicator SHALL be omitted and the bottom
rule SHALL be drawn whole. The content is still reachable by scrolling; a clipped
indicator would not be.

#### Scenario: The overlay scrolls at both mandated sizes

- **WHEN** a dashboard with `help.open` true is rendered at 120x40, where the body is 39
  rows, the band is 39 rows, and its interior is 37 against 43 content rows
- **THEN** the bottom rule row's final columns read `1-37/43`, ending one column in from
  column 119
- **AND** after ten `Next` actions and a redraw the offset is **clamped to 6** — 43
  content rows less a 37-row interior — so the interior's first row is content row 6 and
  the indicator reads `7-43/43`, not the `11-47/43` an unclamped offset of ten would give
- **AND** at 60x20 the body is 19 rows, the band is 19, its interior is 17, and the
  indicator reads `1-17/43`, ending one column in from column 59

#### Scenario: A held key cannot run the window off the end

- **WHEN** a dashboard with `help.open` true is given two hundred consecutive `Next`
  actions at 60x20, redrawing after each
- **THEN** the last interior row is always content row 43 once the window has reached the
  end, and never a blank row past it
- **AND** `help.scroll` is clamped on every frame by
  `ui::layout::scroll_offset(43, help.scroll, 17)` to a maximum of 26, so it is never used
  unbounded — and it is `normalise_help_scroll` that applies it, which is why this
  60x20 `Route::List` fixture clamps at all where `normalise_scroll` would have returned
  early
- **AND** two hundred consecutive `Prev` actions from there return the window to content
  row 1 and leave `help.scroll` at `0` rather than underflowing

#### Scenario: No indicator when the content fits

- **WHEN** a dashboard with `help.open` true is rendered at 120x60, where the body is 59
  rows and the band is `43 + 2 = 45` rows with an interior of 43 against 43 content rows
- **THEN** the band's bottom row is `─` repeated to the band's width with no digits in it
- **AND** every one of the 43 content rows is present in the buffer, so the whole
  inventory is visible in one frame at a tall pane

