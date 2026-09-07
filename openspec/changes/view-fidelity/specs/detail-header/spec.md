## ADDED Requirements

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
schema and progress cells off the row and, at the wide layout, over the detail region's own
border.

Because the field is padded by `ui::list::pad_or_truncate_right`, which `change-rows`
requires to return exactly `width` columns in both arms, the header row measures exactly
`width` columns even when a wide cluster is dropped whole and the truncated name lands one
column short. The tab bar `artifact-tabs` draws on the following row is unaffected: its cells
are artifact ids, which the schema fixes.

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

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change is named
  `日本語の変更名前です`, whose schema is `tdd`, and whose `progress` is
  `Progress { completed: 4, total: 9 }` is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer the detail region's first interior row spans columns 41
  through 118 and ends with `[4/9]`, and column 119 holds the frame's right border
- **AND** in the 60-column buffer that row spans columns 1 through 58 and column 59 holds the
  border
- **AND** in the 120-column buffer column 39 holds the list block's right border and column
  40 the detail block's left border, unchanged from the same render with an ASCII name

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

## MODIFIED Requirements

### Requirement: The detail region's first row names the selected change

`ui::detail::header_row(name: &str, schema: &str, progress: &tasks::Progress, width: u16)
-> String` SHALL return a string of exactly `width` **display columns**, as
`responsive-layout` defines them — a fixed-field, right-aligned
grammar in the same shape `change-rows` uses for a list row, and returning a plain `String`
with no `ratatui` type, on the same terms `ui::list` and `ui::markdown` return plain data and
let `ui::view` style it.

The full form is `[name field][space][schema cell][space][progress cell]`, where:

- the **progress cell** is `[<completed>/<total>]`, or the three characters `[-]` when
  `total` is zero, produced by the same `ui::list::progress_cell` the list rows use — one
  implementation, so the header and the row can never disagree about a change's progress;
- the **schema cell** is the schema name in parentheses, e.g. `(tdd)`;
- the **name field** is `width - 2 - schema_columns - progress_columns` display columns, padded with trailing
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
  display columns wide
- **AND** the 58-column result is the same grammar with a 45-column name field, 58 display columns
  long, ending in the same `[4/42]`
- **AND** both results end in the progress cell's `]`, so the cell is right-aligned to the
  interior's last column at either width

#### Scenario: A change with no tasks still ends its row in the same column

- **WHEN** `header_row("migrate-ai-sdk-v7", "tdd", Progress { completed: 0, total: 0 }, 78)`
  is called, and again at `58`
- **THEN** each result ends with the three characters `[-]` in the row's final three columns,
  exactly where a `[4/42]` would have ended
- **AND** each result is exactly `width` display columns wide

#### Scenario: A long name is truncated with an ellipsis, never overflowing the row

- **WHEN** `header_row` is called with a 200-character name, schema `tdd`, progress `[4/42]`,
  at width `78` and again at width `58`
- **THEN** each result is exactly `width` display columns wide
- **AND** each result's name field ends with `…` and its schema and progress cells are still
  present and intact, so the name yields before either cell does

#### Scenario: The cells are dropped whole in order as the row narrows

- **WHEN** `header_row("add-token-refresh", "tdd", Progress { completed: 4, total: 9 }, w)`
  is called for `w` in `78`, `58`, `13`, `12`, `7`, `6`, `5`, `1`, and `0` — the two mandated
  interiors plus **both** boundaries of **all three** bands, so every branch of the
  degradation is entered and no band is sampled twice while another goes untouched
- **THEN** at every width the result is exactly `w` display columns wide
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
- **AND** each result is exactly `width` display columns wide, so an empty schema shortens the
  cell rather than removing it and the row's arithmetic still balances
