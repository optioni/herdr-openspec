# artifact-tabs Specification

## Purpose
Owns the row of tabs across the top of the detail region: one cell per entry of
`Change::artifacts` in the schema's declared order, addressed strictly by position so
duplicate artifact ids stay two separately selectable tabs, drawn as chips — each label the
bare `<id>` padded one space on each side, carrying no digit at any position, though `1`-`9`
still select the first nine — windowed to whole cells that always
keep the selected tab on screen, and reduced to a single `no artifacts` placeholder when a
change declares none. It also defines where the bar sits — the second interior row of the
detail region, via `split_detail`'s header/tab-bar/content split — and the keys that move it:
`1`-`9`, `[` and `]` outside filter mode, inert rather than clamped on an out-of-range digit,
resetting the scroll only when the tab actually changed. Reading the selected tab's file is
`artifact-content`'s.

## Requirements

### Requirement: The tab bar is built from the schema's declared artifact order, addressed by position

`ui::detail::tab_bar(artifacts: &[changes::ArtifactRef], selected: usize, width: u16)
-> Vec<Tab>` SHALL build the tab bar from `Change::artifacts` — the list
`changes::from_files` already resolved from the schema's `generates` values, in the schema's
**declared order**. It SHALL NOT read a schema, open a directory, guess a path from an
artifact id, sort the list, or de-duplicate it: `schema-artifacts` requires that a schema
declaring the same artifact id at two positions keeps both, and two tabs with the same label
are therefore a legal, renderable state.

Every tab SHALL be addressed by its **position** in that list and never by its id. The
selected tab is `Dashboard::detail.tab`, a `usize` index into `artifacts`.

```rust
pub struct Tab {
    pub text: String,
    pub x: u16,
    pub index: Option<usize>,
    pub selected: bool,
}
```

`text` is the cell's label, `x` its column offset from the interior's first column, `index`
its position in `artifacts` (`None` for the zero-artifact placeholder below), and `selected`
whether it is the selected tab. `Tab` carries no `ratatui` type — `NOTABSEAM` forbids one in
`src/ui/detail.rs` — so `ui::view` is what maps `selected` to a style, exactly as it does for
`ui::list::Row`.

**The cell is a chip.** A tab's label SHALL be its bare `<id>` padded with exactly one space
on each side, so a cell `columns(id) + 2` wide is drawn and `ui::view` paints every one of
those columns a background. Consecutive chips SHALL be separated by exactly **one** column,
which no chip occupies and nothing paints, so two adjacent inactive chips show a visible edge
rather than one continuous field.

No label SHALL carry a leading digit. `1`–`9` still select a tab — `action_for` is unchanged
— but the bar no longer advertises them: a numbered chip is wider than the bar needs to be,
and a row of bare numbered labels two spaces apart reads as a sentence rather than as tabs.

Measured against this repository's five-artifact `tdd` schema the bar is **53 columns**: the
five chips are 10, 7, 8, 7, and 17 columns and four one-column spacers separate them. It
therefore fits both mandated interior widths, 78 and 58, with more slack than the
57-column numbered bar it replaces.

#### Scenario: The five tdd artifacts become five numbered tabs at both mandated widths

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
its subject is the same five artifacts, now drawn as unnumbered chips.

- **WHEN** `tab_bar` is called with artifact ids `proposal`, `specs`, `design`, `tasks`,
  `planning-review`, `selected: 0`, at width `78`, and again at width `58`
- **THEN** at both widths five tabs are returned, with texts `" proposal "`, `" specs "`,
  `" design "`, `" tasks "`, and `" planning-review "`, each beginning and ending in a space
- **AND** their `x` values are `0`, `11`, `19`, `28`, and `36`, so each chip starts exactly
  one column after the previous chip's last column
- **AND** the last chip's final column is 52, so the whole bar occupies 53 of the 58
  available columns at the narrow interior and 53 of the 78 at the wide one
- **AND** `index` is `Some(0)` through `Some(4)` in order, and only the first `Tab` has
  `selected` true

#### Scenario: A tenth artifact is labelled without a digit

The name is kept verbatim as the merge key. Its subject is now the stronger claim: **no**
position carries a digit, so the tenth is labelled exactly as the first.

- **WHEN** `tab_bar` is called with twelve artifacts whose ids are `a01` through `a12`,
  `selected: 0`, at width `78` and again at width `58`
- **THEN** every returned `text` is its artifact's bare id with one space on each side —
  `" a01 "`, `" a02 "`, and so on — and none begins with a digit or with `1 `
- **AND** at width `78` all twelve cells are returned, so the tenth, eleventh, and twelfth are
  present and are each exactly five columns wide, the same as the first — which the numbered
  grammar could not do
- **AND** at width `58` the window holds nine cells, because nine five-column chips and eight
  separators are 53 columns and a tenth would be 59, so the two widths differ and the
  narrower one is not silently vacuous

#### Scenario: Duplicate artifact ids remain two separately addressable tabs

- **WHEN** `tab_bar` is called with artifact ids `spec`, `spec`, `notes`, `selected: 1`, at
  width `78` and again at width `58`
- **THEN** three tabs are returned at both widths, with texts `" spec "`, `" spec "`, and
  `" notes "`
- **AND** the second has `index: Some(1)` and `selected` true while the first has
  `index: Some(0)` and `selected` false, so the two identically-labelled chips are
  distinguished by position rather than collapsed

#### Scenario: No artifacts is a single placeholder cell, not an empty bar

- **WHEN** `tab_bar` is called with an empty artifact slice, `selected: 0`, at width `78` and
  again at width `58`
- **THEN** exactly one `Tab` is returned at each width, with `text` `" no artifacts "`,
  `x: 0`, `index: None`, and `selected` false
- **AND** rendering it does not panic and writes only those fourteen columns
- **AND** at width `8` the same call returns one `Tab` whose `text` is exactly eight columns
  ending in `…`, because the placeholder is right-truncated by the same shared rule every
  other chip is, rather than overflowing a bar narrower than it

#### Scenario: A zero-width bar is empty and does not panic

- **WHEN** `tab_bar` is called at width `0` with five artifacts, and again with none
- **THEN** an empty vector is returned in both cases, and neither call panics

### Requirement: The tab bar is a window of whole cells that always contains the selected tab

At `width == 0` `tab_bar` SHALL return an empty vector, and that guard SHALL precede every
branch below, the zero-artifact placeholder and the oversized-cell exception included.

When `selected` is **not** a valid index into `artifacts`, the window SHALL be computed as if
`selected` were `0`, and no returned cell SHALL have `selected` true. This is a transient
state `Dashboard::sync_detail` clamps away before the next draw; it is defined here so the
function is total rather than "does not panic".

The **joined width** of the chips `start..=end` SHALL be the sum of their own widths plus
`end - start` separating columns — one per gap, not two. This is the one arithmetic the chip
grammar changes; every rule below is stated against it.

When the joined cells fit within `width`, `tab_bar` SHALL return every cell, starting at
column `0`.

When they do not, it SHALL return a **contiguous window** `start..=end` of whole cells,
chosen so that:

- `start` is the **smallest** index not greater than `selected` for which the chips
  `start..=selected` fit within `width` — so the window slides right only as far as keeping
  the selected tab visible requires, and a leftward move of the selection slides it back;
- `end` is the **largest** index not less than `selected` for which the chips `start..=end`
  fit within `width`.

Cells SHALL be dropped **whole**, never cut short, on the same terms `change-rows` drops a
list row's cells, and the window SHALL start at column `0` regardless of `start`. There is no
overflow marker and no ellipsis cell: `list-selection`'s viewport shows no scroll indicator
either, and inventing one here would be a second, undeclared grammar.

The one exception, stated rather than discovered: when the anchor chip alone — the selected
one, or the `no artifacts` placeholder, which is drawn on the bar's own terms per the
requirement above — is wider than `width`, no whole-cell window exists. `tab_bar` SHALL then return that single chip —
padding included, since the padding is what the view paints — truncated to `width` columns
with a trailing `…` by the same `ui::list::pad_or_truncate_right` the header and the row
grammar use, with `x: 0` and `selected` true.

`tab_bar` SHALL be total: it SHALL NOT panic for any artifact list, any `selected` including
one past the end of the list, or any width.

#### Scenario: A twelve-artifact bar windows to keep the selected tab visible

- **WHEN** `tab_bar` is called with twelve artifacts whose ids are `artifact-01` through
  `artifact-12` — a 13-column chip each — at width `58` with `selected: 0`, then
  `selected: 3`, then `selected: 11`, and the same three at width `78`
- **THEN** at every call the returned cells are contiguous in position, the first has `x: 0`,
  and the last cell's final column is less than `width`
- **AND** at every call exactly one returned cell has `selected` true and its `index` equals
  the `selected` argument, so the selected tab is never scrolled off
- **AND** at `selected: 0` the window begins at index `0`; at `selected: 11` it ends at index
  `11`; and every 58-column window holds four chips while every 78-column window holds five,
  so the width is genuinely load-bearing
- **AND** no returned `text` ends in `…`, so every shown chip was shown whole

#### Scenario: The window slides back when the selection moves left again

- **WHEN** `tab_bar` is called with the same twelve artifacts at width `58` with
  `selected: 11`, and then again with `selected: 0`, and the pair repeated at width `78`
- **THEN** the second call's window begins at index `0` at both widths, so the window is a
  pure function of `(artifacts, selected, width)` and carries no hysteresis from the first

#### Scenario: A selected cell wider than the whole bar is truncated rather than dropped

- **WHEN** `tab_bar` is called with one artifact whose id is 200 characters long,
  `selected: 0`, at width `58` and again at width `78`
- **THEN** exactly one `Tab` is returned at each width, with `x: 0`, `selected` true,
  `index: Some(0)`, and `text` exactly `width` columns long ending in `…`
- **AND** each `text` begins with the chip's own leading space, so what is truncated is the
  padded chip and not a bare id
- **AND** neither call panics

#### Scenario: A selected index past the end of the list does not panic

- **WHEN** `tab_bar` is called with three artifacts and `selected: 7`, at width `58` and
  again at width `78`
- **THEN** neither call panics, and each returns cells whose `index` values are all within
  `0..3`
- **AND** no returned cell has `selected` true, since no cell holds that position
- **AND** the window is the one `selected: 0` would have produced — at these widths, all
  three chips starting at column `0` — so the out-of-range case has a defined result rather
  than merely an absence of panic

### Requirement: The tab bar is drawn into the detail region's second interior row

`ui::view::render` SHALL draw each `Tab` into the second row of the detail region's interior
at `interior.x + tab.x`, applying `palette::style(Role::TabActive)` to the selected chip's
cells and `palette::style(Role::TabInactive)` to every other chip's — including the
zero-artifact placeholder, which occupies the bar's position and is drawn on the bar's own
terms. Every column of a chip, its two padding columns included, SHALL carry that style, so
the painted span is exactly the chip's own span. The separating column between two chips
SHALL be left untouched, as SHALL every column no chip occupies, on the same terms
`detail-scroll` leaves the tail of a short markdown line untouched.

`ui::layout::split_detail(interior: Rect) -> (Rect, Rect, Rect)` SHALL split the detail
region's interior into a one-row header, a one-row tab bar, and the content area below,
each the interior's full width. Heights `0`, `1`, and `2` SHALL be branched on explicitly,
exactly as `split_frame` does and for the same measured reason: at height `1` the single row
belongs to the header and the other two rects are zero-height at the same `y`; at height `2`
the header and the tab bar each take one row and the content area is zero-height; at height
`h` above that the content area is `h - 2` rows starting at `interior.y + 2`.

#### Scenario: The tab bar reaches the buffer at both mandated widths

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries the five tdd
  artifacts, with `detail.tab: 2`, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer row 3, columns 41 through 93, spell
  `" proposal   specs   design   tasks   planning-review "` — three spaces between chips,
  being each chip's own padding and the one unpainted separator column
- **AND** in the 60-column buffer row 3, columns 1 through 53, spell the same 53-column
  string
- **AND** in both buffers the eight cells spelling `" design "` report `Modifier::BOLD` set
  and the background and foreground `Role::TabActive` carries (`Color::Cyan` on
  `Color::Black`), and the ten cells spelling `" proposal "` report the background
  `Role::TabInactive` carries (`Color::DarkGray`) and no `BOLD`, so the selected chip is
  discriminated by colour and by weight together
- **AND** in both buffers the single column between two chips has no background set, so the
  chips do not merge into one field

#### Scenario: The tab bar never overwrites a border or the rows around it

- **WHEN** a `Dashboard` whose selected change carries twelve artifacts with 40-character
  ids is rendered at 120x20 and at 60x20 at `Route::Detail`
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 1 through 18
  is a box-drawing character
- **AND** in the 120-column buffer every cell of columns 0, 39, 40, and 119 in rows 1
  through 18 is a box-drawing character
- **AND** in both buffers no cell of a border column carries a chip background, so the
  painted span stopped inside the interior
- **AND** in both buffers the header row above the tab bar still holds the change's name, so
  no chip wrapped upward

#### Scenario: `split_detail` is exact at its degenerate heights

- **WHEN** `split_detail` is called on interiors of the two mandated widths — 78 and 58 — at
  heights `0`, `1`, `2`, `3`, and `16`
- **THEN** at height `0` all three rects are zero-height at the interior's own `y`
- **AND** at height `1` the header is one row at `interior.y` and the tab bar and content are
  zero-height, both at `interior.y + 1`
- **AND** at height `2` the header and tab bar are one row each, at `interior.y` and
  `interior.y + 1`, and the content is zero-height at `interior.y + 2`
- **AND** at heights `3` and `16` the content area is `height - 2` rows starting two rows
  below the interior's `y`
- **AND** at every height all three rects carry the interior's own `x` and `width`

### Requirement: `1`–`9`, `[`, and `]` switch the artifact tab

`ui::app::action_for` SHALL, while `filtering` is **false**, map a Press of `KeyCode::Char`
`'1'` through `'9'` with no modifiers to `Action::SelectTab(n - 1)`, a Press of `Char(']')`
with no modifiers to `Action::NextTab`, and a Press of `Char('[')` with no modifiers to
`Action::PrevTab`. `Char('0')` SHALL map to `Ignore`: tab addressing is 1-based, so there is
no zeroth tab.

While `filtering` is **true** these keys SHALL continue to type themselves into the query as
`FilterPush`, because `list-filtering` makes every printable character a query character and
this change does not carve exceptions out of it.

`Dashboard::apply` SHALL:

- on `SelectTab(i)`, set `detail.tab` to `i` when `i` is a valid index into the selected
  change's `artifacts`, and change nothing at all otherwise — a `7` pressed on a change with
  five artifacts is inert, not clamped, because clamping would move the tab to a position the
  user did not ask for;
- on `NextTab`, raise `detail.tab` by one, clamped to the last artifact's index, and change
  nothing when the selected change has no artifacts or none is selected;
- on `PrevTab`, lower `detail.tab` by one, clamped at zero;
- reset `detail.scroll` to `0` exactly when `detail.tab` changed value, so switching tabs
  starts the new document at its top while a `]` at the last tab leaves the reading position
  alone;
- on `Next` and `Prev` at `Route::List`, reset both `detail.tab` and `detail.scroll` to `0`
  exactly when `selected` changed value, so moving to another change opens its first tab and
  a clamped no-op at either end of the list does not.

The keys SHALL work at **both** routes. At the wide layout the detail region is drawn at the
list route too, so a tab press there is immediately visible; at the narrow layout the state
still moves and is visible as soon as `Enter` opens the detail.

#### Scenario: The digit keys select tabs and near misses do not

- **WHEN** `action_for` is called with `filtering` false and Presses of `Char('1')`,
  `Char('5')`, `Char('9')`, `Char('0')`, `Char('1')` with `CONTROL`, and `Char('!')` with
  `SHIFT`
- **THEN** the first three return `SelectTab(0)`, `SelectTab(4)`, and `SelectTab(8)`, and the
  last three return `Ignore`
- **AND** with `filtering` true the same six events return `FilterPush('1')`,
  `FilterPush('5')`, `FilterPush('9')`, `FilterPush('0')`, `Ignore`, and `FilterPush('!')`

#### Scenario: The bracket keys step one tab and near misses do not

- **WHEN** `action_for` is called with `filtering` false and Presses of `Char(']')`,
  `Char('[')`, `Char(']')` with `CONTROL`, and `Char('}')` with `SHIFT`
- **THEN** the first two return `NextTab` and `PrevTab` and the last two return `Ignore`
- **AND** with `filtering` true `Char(']')` and `Char('[')` return `FilterPush(']')` and
  `FilterPush('[')`

#### Scenario: Stepping is clamped at both ends and does not wrap

- **WHEN** a `Dashboard` whose selected change carries three artifacts and whose `detail.tab`
  is `0` is given `PrevTab`, then `NextTab` four times, then `PrevTab` four times
- **THEN** `detail.tab` is `0` after the first, `1`, `2`, `2`, `2` after the next four, and
  `1`, `0`, `0`, `0` after the last four
- **AND** it never becomes `3` and never wraps to `2` from `0`

#### Scenario: An out-of-range digit is inert

- **WHEN** a `Dashboard` whose selected change carries three artifacts and whose `detail.tab`
  is `1` and `detail.scroll` is `5` is given `SelectTab(6)`, and then `SelectTab(2)`
- **THEN** after the first, `detail.tab` is still `1` and `detail.scroll` is still `5`
- **AND** after the second, `detail.tab` is `2` and `detail.scroll` is `0`

#### Scenario: Switching tabs resets the scroll and staying put does not

- **WHEN** a `Dashboard` whose selected change carries three artifacts, whose `detail.tab` is
  `2` and whose `detail.scroll` is `7`, is given `NextTab`, and a second identical dashboard
  is given `PrevTab`
- **THEN** the first still has `detail.tab` `2` and `detail.scroll` `7`, because the clamped
  step changed nothing
- **AND** the second has `detail.tab` `1` and `detail.scroll` `0`

#### Scenario: Moving the selection resets the tab and the scroll, and a clamped move does not

- **WHEN** a `Dashboard` at `Route::List` holding three visible changes, with `selected: 0`,
  `detail.tab: 2`, and `detail.scroll: 9`, is given `Next`, and a second dashboard identical
  but for `selected: 0` at the top of the list is given `Prev`
- **THEN** the first has `selected: 1`, `detail.tab: 0`, and `detail.scroll: 0`
- **AND** the second still has `selected: 0`, `detail.tab: 2`, and `detail.scroll: 9`,
  because the clamped move changed no change

#### Scenario: Tab keys act at both routes

- **WHEN** a `Dashboard` at `Route::List` whose selected change carries three artifacts and
  whose `detail.tab` is `0` is given `SelectTab(2)`, and a second at `Route::Detail` is given
  the same
- **THEN** both have `detail.tab` `2` and neither has changed `route`
- **AND** rendering the first at 120x20 shows the third artifact's chip — `" <id> "`, padded,
  with no leading digit — carrying the active chip's style in the detail region's tab row, so
  the wide layout makes the list-route press immediately visible. The digit keys still select
  a tab; the bar no longer advertises them, which is `artifact-tabs`' chip grammar and changes
  nothing about `action_for`
