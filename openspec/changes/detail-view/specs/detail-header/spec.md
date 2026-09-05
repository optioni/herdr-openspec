## ADDED Requirements

### Requirement: The detail region's first row names the selected change

`ui::detail::header_row(name: &str, schema: &str, progress: &tasks::Progress, width: u16)
-> String` SHALL return a string of exactly `width` characters — a fixed-field, right-aligned
grammar in the same shape `change-rows` uses for a list row, and returning a plain `String`
with no `ratatui` type, on the same terms `ui::list` and `ui::markdown` return plain data and
let `ui::view` style it.

The full form is `[name field][space][schema cell][space][progress cell]`, where:

- the **progress cell** is `[<completed>/<total>]`, or the three characters `[-]` when
  `total` is zero, produced by the same `ui::list::progress_cell` the list rows use — one
  implementation, so the header and the row can never disagree about a change's progress;
- the **schema cell** is the schema name in parentheses, e.g. `(tdd)`;
- the **name field** is `width - 2 - schema_len - progress_len` columns, padded with trailing
  spaces when the name is shorter and truncated with a trailing `…` when it is longer, by the
  same `ui::list::pad_or_truncate_right` the row grammar uses — which this change raises from
  private to `pub(crate)` rather than copying, so the crate keeps exactly one
  right-truncation implementation.

A cell too wide for the row SHALL be dropped **whole**, never cut short, in this fixed order,
which mirrors `change-rows`' order for the same reason (the identity of the row is the name,
so the name is what survives):

1. drop the **schema cell**, reclaiming its separating space, when the name field would fall
   below one column;
2. then drop the **progress cell**, reclaiming its separating space, when the name field
   would still fall below one column, leaving the whole `width` to the name field;
3. at `width == 0` return the empty string.

The name is truncated only after both cells have been dropped.

For a five-character schema cell and a five-character progress cell the three bands are
therefore **`width >= 13`** (full form, name field `width - 12`), **`7 <= width <= 12`**
(schema dropped, name field `width - 6`), and **`1 <= width <= 6`** (both dropped, name field
`width`). Those boundaries are named here because a scenario that only samples widths inside
one band cannot tell a correct implementation from one that never drops a cell.

`header_row` SHALL be total: it SHALL NOT panic at any width, for any name including an empty
one and one longer than the row, for any schema name including an empty one, and for any
`Progress`.

#### Scenario: The full header grammar at both mandated interior widths

- **WHEN** `header_row("detail-view", "tdd", Progress { completed: 4, total: 42 }, 78)` is
  called, and again at width `58`
- **THEN** the 78-column result is `detail-view` followed by spaces out to column 64, then
  ` (tdd) [4/42]`, so its final character sits in column 77 and the whole string is 78
  characters long
- **AND** the 58-column result is the same grammar with a 45-column name field, 58 characters
  long, ending in the same `[4/42]`
- **AND** both results end in the progress cell's `]`, so the cell is right-aligned to the
  interior's last column at either width

#### Scenario: A change with no tasks still ends its row in the same column

- **WHEN** `header_row("migrate-ai-sdk-v7", "tdd", Progress { completed: 0, total: 0 }, 78)`
  is called, and again at `58`
- **THEN** each result ends with the three characters `[-]` in the row's final three columns,
  exactly where a `[4/42]` would have ended
- **AND** each result is exactly `width` characters long

#### Scenario: A long name is truncated with an ellipsis, never overflowing the row

- **WHEN** `header_row` is called with a 200-character name, schema `tdd`, progress `[4/42]`,
  at width `78` and again at width `58`
- **THEN** each result is exactly `width` characters long
- **AND** each result's name field ends with `…` and its schema and progress cells are still
  present and intact, so the name yields before either cell does

#### Scenario: The cells are dropped whole in order as the row narrows

- **WHEN** `header_row("add-token-refresh", "tdd", Progress { completed: 4, total: 9 }, w)`
  is called for `w` in `78`, `58`, `13`, `12`, `7`, `6`, `5`, `1`, and `0` — the two mandated
  interiors plus **both** boundaries of **all three** bands, so every branch of the
  degradation is entered and no band is sampled twice while another goes untouched
- **THEN** at every width the result is exactly `w` characters long
- **AND** at `78`, `58`, and `13` the result carries both `(tdd)` and `[4/9]` whole
- **AND** at `12` and `7` — the schema-dropped band — `(tdd)` does not appear at all while
  `[4/9]` is still present and intact
- **AND** at `6`, `5`, and `1` neither `(tdd)` nor `[4/9]` appears and the row is the name
  field alone
- **AND** at `w == 0` the result is the empty string
- **AND** wherever a result contains the substring `(tdd` it also contains the whole `(tdd)`,
  and wherever it contains `[4/` it also contains the whole `[4/9]`, so no cell was ever cut
  short rather than dropped

#### Scenario: An empty schema name is a cell of two characters, not an absent one

- **WHEN** `header_row("alpha", "", Progress { completed: 1, total: 2 }, 78)` is called, and
  again at `58`
- **THEN** each result carries the two characters `()` where the schema cell sits, followed
  by a space and `[1/2]`
- **AND** each result is exactly `width` characters long, so an empty schema shortens the
  cell rather than removing it and the row's arithmetic still balances

### Requirement: The header row is drawn into the detail region's first interior row

`ui::view::render` SHALL draw `header_row` into the first row of the detail region's
interior, starting at the interior's first column, with `Modifier::BOLD` — the same emphasis
the frame header's `OpenSpec` label carries, and the crate's only other bold-by-default row.

The header SHALL name the change `list-selection`'s `Dashboard::selected` addresses in
`Dashboard::visible()`. When `visible()` is **empty** — no repository, no changes, or a `/`
filter matching none — the detail region SHALL draw no header, no tab bar, and no content,
leaving every interior cell a space whose `Style` equals
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
- **THEN** in the 120-column buffer row 2, columns 41 through 118, is the 78-character
  `header_row("add-token-refresh", "tdd", 4 of 9, 78)`, beginning `add-token-refresh` and
  ending `(tdd) [4/9]`
- **AND** in the 60-column buffer row 2, columns 1 through 58, is the 58-character
  `header_row(…, 58)`, beginning `add-token-refresh` and ending `(tdd) [4/9]`
- **AND** every cell of the header row in both buffers reports `Modifier::BOLD` set

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
- **THEN** in every one of those buffers each cell of the detail region's interior is a space
  whose `Style` equals `ratatui::buffer::Cell::default().style()`
- **AND** rendering does not panic at either width
