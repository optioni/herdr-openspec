## MODIFIED Requirements

### Requirement: The header row is drawn into the detail region's first interior row

The scenario headers below are kept verbatim because a delta's scenario headers are its merge
key; the row this requirement names is no longer an interior row.

`ui::view::render` SHALL draw `header_row` into the detail region's **heading row** — the
region's own first row, one row above its interior — starting at that row's first column.
`pane-chrome` removed the region's border and gave the row it occupied to the region's
heading; the change header is what a detail region's heading names, exactly as the
repository's directory name is what the list region's heading names.

The header SHALL carry `Modifier::BOLD` when the dashboard's route is `Route::Detail` and
`Modifier::DIM` when it is not, taking both from `palette::style` and never constructing a
`Style` at the call site. That is `responsive-layout`'s routed-region rule applied to this
row: above the breakpoint both regions are drawn and the reader needs to know which one `j`
and `k` are driving. Below the breakpoint only the routed region is drawn, so a drawn detail
header is always the bold one.

The header SHALL name the change `list-selection`'s `Dashboard::selected` addresses in
`Dashboard::visible()`. When `visible()` is **empty** — no repository, no changes, or a `/`
filter matching none — the detail region SHALL draw no header, no tab bar, no rule, and no
content, leaving its heading row and every interior cell a space whose `Style` equals
`ratatui::buffer::Cell::default().style()`. There is no error screen and no placeholder: the
list region already names the empty state, and duplicating it in the detail region would say
the same thing twice.

The header SHALL be drawn whenever the detail region is drawn, which at the wide layout is
**both** routes — `responsive-layout` draws both regions at 100 columns or wider — and at the
narrow layout is the detail route only.

#### Scenario: The header names the selected change at both mandated widths

- **WHEN** a `Dashboard` at `Route::Detail` holding active changes `add-token-refresh` (4 of
  9, schema `tdd`) and `fix-empty-basket` (7 of 7), with `selected: 0`, is rendered at 120x20
  and at 60x20
- **THEN** in the 120-column buffer row 0, columns 41 through 118, is the 78-character
  `header_row("add-token-refresh", "tdd", 4 of 9, 78)`, beginning `add-token-refresh` and
  ending `(tdd) [4/9]`
- **AND** in the 60-column buffer row 0, columns 1 through 58, is the 58-character
  `header_row(…, 58)`, beginning `add-token-refresh` and ending `(tdd) [4/9]`
- **AND** every cell of the header row in both buffers reports `Modifier::BOLD` set, because
  the route is `Route::Detail` in both
- **AND** rendering the 120-column case at `Route::List` gives the identical 78 characters
  with `Modifier::DIM` set and `Modifier::BOLD` clear, so the emphasis discriminates rather
  than asserting a constant

#### Scenario: Moving the selection moves the header

- **WHEN** the same dashboard is rendered at 120x20, then `selected` is set to `1` and it is
  rendered again, at 120x20 and at 60x20
- **THEN** the first buffer's detail header names `add-token-refresh` and the later two name
  `fix-empty-basket` with `[7/7]`
- **AND** the assertion discriminates rather than asserting a constant, because the two
  headers differ in both the name field and the progress cell

#### Scenario: An archived change's header carries its stripped name and its own schema

- **WHEN** a `Dashboard` whose only change is the archived `2026-08-14-add-auth`, whose
  `Change::name` is `add-auth`, whose schema is `spec-driven`, and whose progress is 7 of 7,
  with `selected: 0`, is rendered at 120x20 and at 60x20
- **THEN** each buffer's detail header begins `add-auth`, not `2026-08-14-add-auth`, and
  carries `(spec-driven)` and `[7/7]`
- **AND** no date field appears in the header: the ten-column date field is `change-rows`'
  row grammar, not this one

#### Scenario: An empty visible list leaves the whole detail interior blank

- **WHEN** a `Dashboard` with `changes::empty_set()` is rendered at 120x20 at `Route::List`,
  and a second `Dashboard` holding one change `alpha` whose `filter.query` is `zzz` — so
  `visible()` is empty — is rendered at 120x20 and at 60x20 at `Route::Detail`
- **THEN** in every one of those buffers each cell of the detail region's heading row **and**
  of its interior is a space whose `Style` equals `ratatui::buffer::Cell::default().style()`
- **AND** rendering does not panic at either width

### Requirement: The detail header's cells are measured in display columns

`ui::detail::header_row` SHALL return a string of exactly `width` **display columns** as
`responsive-layout` defines them, not `width` `char`s, and every quantity in its grammar
SHALL be measured the same way: the schema cell's width, the progress cell's width, the name
field's `width - 2 - schema_columns - progress_columns` budget, and the three band boundaries
that follow from them. `ui::detail` SHALL reach the measure only through `layout::columns`
and `layout::truncate_columns` and SHALL name no `char` count of its own.

The progress cell is ASCII by construction and the schema cell is a schema name in
parentheses — `tdd` here, and every schema name the CLI has ever reported is ASCII — so for
this repository the band boundaries `width >= 13`, `7 <= width <= 12`, and `1 <= width <= 6`
are unchanged. The **name field** is the cell that can carry a wide character, because a
change directory name is arbitrary, and it is where the measure genuinely changes: a CJK
change name previously consumed roughly twice the columns its budget allowed, pushing the
schema and progress cells off the row and, at the wide layout, into the detail region's own
right gutter.

Because the field is padded by `ui::list::pad_or_truncate_right`, which `change-rows`
requires to return exactly `width` columns in both arms, the header row measures exactly
`width` columns even when a wide cluster is dropped whole and the truncated name lands one
column short. The tab bar `artifact-tabs` draws is unaffected: its cells are artifact ids,
which the schema fixes. `pane-chrome` moves the tab bar from the row directly below this one
to two rows below it, with a blank row between; nothing about this measure depends on which.

`header_row` SHALL remain total: no panic at any width, for any name — including an empty
one, one longer than the row, one holding wide characters, emoji, combining marks, or a
zero-width-joiner sequence — for any schema name, and for any `Progress`.

#### Scenario: A CJK change name keeps the header inside its region at both mandated widths

- **WHEN** `header_row("日本語の変更名前です", "tdd", Progress { completed: 4, total: 9 },
  78)` and the same call at width `58` are made — a name of ten characters and twenty
  display columns
- **THEN** each result's `layout::columns` is exactly 78 and exactly 58
- **AND** each ends with `[4/9]` and carries `(tdd)` immediately before it, so neither cell
  was pushed off the row
- **AND** each result's `chars().count()` is strictly less than its `layout::columns`, which
  is what proves the padding was computed in columns rather than characters

#### Scenario: The header reaches the buffer without crossing the region border

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
what the header must not cross is now the region's right gutter column.

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change is named
  `日本語の変更名前です`, whose schema is `tdd`, and whose `progress` is
  `Progress { completed: 4, total: 9 }` is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer the detail region's heading row spans columns 41
  through 118 and ends with `[4/9]`, and column 119 is a space
- **AND** in the 60-column buffer that row spans columns 1 through 58 and column 59 is a
  space
- **AND** in the 120-column buffer column 39 is a space and column 40 holds the divider `│`,
  unchanged from the same render with an ASCII name

#### Scenario: The header is total over adversarial names at every width

- **WHEN** `header_row` is called at every width from `0` through `130`, with schema `tdd`
  and `Progress { completed: 4, total: 9 }`, for each of: a 200-column CJK name; a family
  emoji joined by two zero-width joiners; `e` followed by five combining accents; a name
  holding a NUL; and the empty string
- **THEN** no call panics at any width for any of the five
- **AND** at every width at or above 1 every result's `layout::columns` is exactly that
  width, and at width `0` every result is the empty string
- **AND** at widths 13, 12, 7, 6, and 1 the cells drop in the documented order for every one
  of the five names, so the band boundaries hold for wide content as well as for ASCII

### Requirement: The detail header's style is a palette role, not a modifier written at the call site

`ui::view::render` SHALL draw `ui::detail::header_row` with
`palette::style(Role::RegionHeadingFocused)` when the dashboard's route is `Route::Detail`
and `palette::style(Role::RegionHeading)` when it is not, rather than constructing a `Style`
at the call site.

`Role::DetailHeader` is removed. It carried `Modifier::BOLD` and no colour, which is exactly
what `Role::RegionHeadingFocused` carries; keeping a second role that differs from the
region-heading role in nothing but its name would let the detail region's heading and the
list region's heading drift apart for no reason a reader could see. The removal is specified
by `view-palette`, which owns the role table.

`Role::RegionHeadingFocused` SHALL carry `Modifier::BOLD` and **no colour** and
`Role::RegionHeading` SHALL carry `Modifier::DIM` and no colour, so a drawn detail header
reports one of the two and never a foreground. The change header names a change; the tab bar
below it is where this capability's neighbour spends its colour, and a coloured header
competing with the chips would blunt exactly the distinction the chips exist to draw.

#### Scenario: The detail header is bold and uncoloured at both mandated widths

- **WHEN** a `Dashboard` at `Route::Detail` holding active changes `add-token-refresh` (4 of
  9, schema `tdd`) and `fix-empty-basket` (7 of 7), with `selected: 0`, is rendered at 120x20
  and at 60x20
- **THEN** in the 120-column buffer row 0, columns 41 through 118, is the 78-character
  `header_row("add-token-refresh", "tdd", 4 of 9, 78)`, and in the 60-column buffer row 0,
  columns 1 through 58, is the 58-character form
- **AND** every cell of the header row in both buffers reports `Modifier::BOLD` set and no
  foreground and no background
- **AND** the tab-bar row two rows below it does carry a background, so the two rows are
  distinguishable and the header was not left unstyled by accident
- **AND** rendering the 120-column case at `Route::List` reports `Modifier::DIM` set and
  still no foreground and no background, so the role swap changes the modifier alone
