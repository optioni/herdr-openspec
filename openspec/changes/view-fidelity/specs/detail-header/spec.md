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
