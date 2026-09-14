## MODIFIED Requirements

### Requirement: The detail region's first row names the selected change

`ui::detail::header_row(name: &str, schema: &str, progress: &tasks::Progress, width: u16)
-> String` SHALL return a string of exactly `width` **display columns**, as
`responsive-layout` defines them — a fixed-field, right-aligned
grammar in the same shape `change-rows` uses for a list row, and returning a plain `String`
with no `ratatui` type, on the same terms `ui::list` and `ui::markdown` return plain data and
let `ui::view` style it. Its signature SHALL NOT change: the gauge this requirement adds is
derived from the `progress` argument the row already takes, so `ui::view`'s one call site is
untouched.

The full form is `[name field][space][schema cell][space][gauge cell][space][progress cell]`,
where:

- the **progress cell** is `[<completed>/<total>]`, or the three characters `[-]` when
  `total` is zero, produced by the same `ui::list::progress_cell` the list rows use — one
  implementation, so the header and the row can never disagree about a change's progress;
- the **gauge cell** is a bare run of exactly **12** display columns, `filled` of them `█`
  (U+2588) followed by `12 - filled` of them `░` (U+2591), produced by the same
  `ui::tasks::gauge_of` the tracked-tasks tab's own bar uses — one implementation, so the
  header's gauge and the tab's gauge can never disagree about the same change. The run
  carries no surrounding brackets, for the reason `tasks-progress-bar` already gives: the
  progress cell beside it already carries a bracket pair. It carries no percent cell at any
  width: the percentage is the tracked-tasks tab's own field, and a header that already
  states `[4/9]` beside a gauge does not need the same fact a third time;
- the **schema cell** is the schema name in parentheses, e.g. `(tdd)`;
- the **name field** is `width - 3 - schema_columns - 12 - progress_columns` display
  columns, padded with trailing spaces when the name is shorter and truncated with a
  trailing `…` when it is longer, by the same `ui::list::pad_or_truncate_right` the row
  grammar uses.

The gauge's budget SHALL be a **fixed 12 columns** wherever the gauge is drawn at all, never
a share of the width and never a remainder: every column a wider frame brings SHALL go to the
name field, which is the field already being squeezed at the narrower of the two mandated
interiors. Twelve is chosen because it leaves a 32-column name field at the 58-column
interior — wider than the longest change name this repository has — while still resolving a
one-task move on a ten-task change to a visible cell. It is an accepted limit of any fixed
gauge that a one-task move on a change with more tasks than the gauge has cells renders no
visible step; the progress cell one space away states the exact pair, which is why the gauge
is an addition to that cell and not a replacement for it.

When `progress.total == 0` **no gauge cell and no separating space SHALL be drawn** at any
width, and the full form is `[name field][space][schema cell][space][progress cell]` with a
`[-]` progress cell — byte-identically to what this requirement produced before the gauge
was added. A gauge with no denominator would have to invent a fill, which is the same reason
`tasks-progress-bar` draws none for such a change.

A cell too wide for the row SHALL be dropped **whole**, never cut short, in this fixed order.
The gauge is placed **first** in that order, ahead of both cells that preceded it, so the
relative order of the schema and progress cells — and therefore every width band below the
full form — is exactly what it was before this change:

1. drop the **gauge cell**, reclaiming its separating space, when the name field would fall
   below one column;
2. then drop the **schema cell**, reclaiming its separating space, when the name field would
   still fall below one column;
3. then drop the **progress cell**, reclaiming its separating space, when the name field
   would still fall below one column, leaving the whole `width` to the name field;
4. at `width == 0` return the empty string.

The name is truncated only after all three cells have been dropped.

For a five-column schema cell, a twelve-column gauge, and a five-column progress cell the
four bands are therefore **`width >= 26`** (full form, name field `width - 25`),
**`13 <= width <= 25`** (gauge dropped, name field `width - 12`), **`7 <= width <= 12`**
(schema dropped too, name field `width - 6`), and **`1 <= width <= 6`** (all three dropped,
name field `width`). The lower three bands are the three this requirement already had, at the
same boundaries and producing the same strings: below 26 columns this change is not
observable. Those boundaries are named here because a scenario that only samples widths
inside one band cannot tell a correct implementation from one that never drops a cell.

`header_row` SHALL be total: it SHALL NOT panic at any width, for any name including an empty
one and one longer than the row, for any schema name including an empty one, and for any
`Progress` including `{ completed: usize::MAX, total: usize::MAX }`.

`ui::detail` SHALL introduce no colour of its own for the gauge. The header row is one string
that `ui::view` styles whole, under the `Role::RegionHeadingFocused`/dim pair this capability
already fixes, so the gauge takes the heading's own emphasis and `view-palette` gains no role.

#### Scenario: The full header grammar at both mandated interior widths

- **WHEN** `header_row("detail-view", "tdd", Progress { completed: 4, total: 42 }, 78)` is
  called, and again at width `58`
- **THEN** the 78-column result is `detail-view` followed by trailing spaces filling a
  52-column name field, then a space, `(tdd)`, a space, a 12-column gauge, a space, and
  `[4/42]`, so its final character sits in column 77 and the whole string is 78 display
  columns wide
- **AND** the 58-column result is the same grammar with a 32-column name field, 58 display
  columns long, ending in the same `[4/42]`
- **AND** at both widths the gauge holds exactly one `█` and eleven `░`, because
  `12 * 4 / 42` truncates to 1, proving the fill is computed rather than copied from the
  tracked-tasks tab's own wider gauge
- **AND** both results end in the progress cell's `]`, so the cell is right-aligned to the
  interior's last column at either width

#### Scenario: A complete change renders a full gauge and an untouched one renders an empty gauge

- **WHEN** `header_row("fix-empty-basket", "tdd", Progress { completed: 7, total: 7 }, 78)`
  is called, and again at `58`, and then the same two calls with
  `Progress { completed: 0, total: 7 }`
- **THEN** each 7-of-7 result's gauge holds twelve `█` and no `░` at all, and ends `[7/7]`
- **AND** each 0-of-7 result's gauge holds twelve `░` and no `█` at all, and ends `[0/7]`
- **AND** with `Progress { completed: 6, total: 7 }` at the same two widths the gauge holds
  at least one `░`, so a one-task-short change never renders a full gauge in the header —
  the same `filled == g` iff `is_complete()` property `tasks-progress-bar` fixes, holding
  here because it is the same `gauge_of` call

#### Scenario: A change with no tasks still ends its row in the same column

- **WHEN** `header_row("migrate-ai-sdk-v7", "tdd", Progress { completed: 0, total: 0 }, w)`
  is called for `w` in `78`, `58`, `26`, `13`, `12`, `7`, `6`, `1`, and `0`
- **THEN** no result contains `█` or `░` at any width
- **AND** the results at `78` and `58` end with the three characters `[-]` in the row's
  final three columns, with a 68-column name field at 78 and a 48-column one at 58 — the
  name fields this requirement produced before the gauge existed, not ones narrowed by 13
  columns for a gauge that was never drawn
- **AND** each result is exactly `w` display columns wide, and the result at `w == 0` is the
  empty string

#### Scenario: A long name is truncated with an ellipsis, never overflowing the row

- **WHEN** `header_row` is called with a 200-character name, schema `tdd`, progress
  `Progress { completed: 4, total: 42 }`, at width `78` and again at width `58`
- **THEN** each result is exactly `width` display columns wide
- **AND** each result's name field ends with `…` and its schema cell, its 12-column gauge,
  and its progress cell are still present and intact, so the name yields before any of the
  three does

#### Scenario: The cells are dropped whole in order as the row narrows

- **WHEN** `header_row("add-token-refresh", "tdd", Progress { completed: 4, total: 9 }, w)`
  is called for `w` in `78`, `58`, `26`, `25`, `13`, `12`, `7`, `6`, `5`, `1`, and `0` — the
  two mandated interiors plus **both** boundaries of **all four** bands, so every branch of
  the degradation is entered and no band is sampled twice while another goes untouched
- **THEN** at every width the result is exactly `w` display columns wide
- **AND** at `78`, `58`, and `26` the result carries a 12-column gauge, `(tdd)`, and `[4/9]`,
  all three whole
- **AND** at `25` and `13` — the gauge-dropped band — the result contains neither `█` nor
  `░` while `(tdd)` and `[4/9]` are both still present and intact
- **AND** at `12` and `7` — the schema-dropped band — `(tdd)` does not appear at all while
  `[4/9]` is still present and intact
- **AND** at `6`, `5`, and `1` neither `(tdd)` nor `[4/9]` appears and the row is the name
  field alone
- **AND** at `w == 0` the result is the empty string
- **AND** wherever a result contains the substring `(tdd` it also contains the whole `(tdd)`,
  and wherever it contains `[4/` it also contains the whole `[4/9]`, and wherever it contains
  `█` or `░` it contains exactly twelve gauge characters, so no cell was ever cut short
  rather than dropped

#### Scenario: Below the full-form band the header is byte-identical to the pre-gauge grammar

- **WHEN** `header_row("add-token-refresh", "tdd", Progress { completed: 4, total: 9 }, w)`
  is called for every `w` from `0` through `25` inclusive
- **THEN** every result equals, byte for byte, the string the three-band grammar produced
  before the gauge cell was added — the name field padded or ellipsised to `w - 12`, `w - 6`,
  or `w` according to the band, with the same cells present
- **AND** this is the discriminating claim of the drop order: an implementation that placed
  the gauge anywhere but first in the order, or that reserved its 12 columns before deciding
  whether it fits, would change at least one of these 26 strings

#### Scenario: An empty schema name is a cell of two characters, not an absent one

- **WHEN** `header_row("alpha", "", Progress { completed: 1, total: 2 }, 78)` is called, and
  again at `58`
- **THEN** each result carries the two characters `()` where the schema cell sits, followed
  by a space, a 12-column gauge holding six `█` and six `░`, a space, and `[1/2]`
- **AND** each result is exactly `width` display columns wide, so an empty schema shortens the
  cell rather than removing it and the row's arithmetic still balances

### Requirement: The header row is drawn into the detail region's first interior row

The scenario headers below are kept verbatim because a delta's scenario headers are its merge
key; the row this requirement names is no longer an interior row.

`ui::view::render` SHALL draw `header_row` into the detail region's **heading row** — the
region's own first row, two rows above its interior, with the region's blank padding row
between them — starting at that row's first column.
`pane-chrome` removed the region's border and gave the row it occupied to the region's
heading; the change header is what a detail region's heading names, exactly as the
repository's directory name is what the list region's heading names.

The header SHALL carry `Modifier::BOLD` when the dashboard's route is `Route::Detail` and
`Modifier::DIM` when it is not, taking both from `palette::style` and never constructing a
`Style` at the call site. That is `responsive-layout`'s routed-region rule applied to this
row: above the breakpoint both regions are drawn and the reader needs to know which one `j`
and `k` are driving. Below the breakpoint only the routed region is drawn, so a drawn detail
header is always the bold one. The gauge cell takes that same pair with the rest of the row:
it is part of the one string this row draws, not a separately styled span.

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

The gauge SHALL reach the buffer on **every** artifact tab, not only the tracked-tasks one,
because this row is drawn from the selected `Change` and never from the selected tab. That is
the whole point of the cell: the tracked-tasks tab's own bar is unreachable while reading
`proposal.md`, and this row is not.

#### Scenario: The header names the selected change at both mandated widths

- **WHEN** a `Dashboard` at `Route::Detail` holding active changes `add-token-refresh` (4 of
  9, schema `tdd`) and `fix-empty-basket` (7 of 7), with `selected: 0`, is rendered at 120x20
  and at 60x20
- **THEN** in the 120-column buffer row 0, columns 42 through 119, is the 78-character
  `header_row("add-token-refresh", "tdd", 4 of 9, 78)`, beginning `add-token-refresh` and
  ending `(tdd) █████░░░░░░░ [4/9]` in the frame's own last column, the wide detail region
  having no right gutter, its name field 53 columns and its gauge holding five `█` because
  `12 * 4 / 9` truncates to 5
- **AND** in the 60-column buffer row 0, columns 1 through 58, is the 58-character
  `header_row(…, 58)`, beginning `add-token-refresh` and ending
  `(tdd) █████░░░░░░░ [4/9]` with a 33-column name field
- **AND** every cell of the header row in both buffers reports `Modifier::BOLD` set, because
  the route is `Route::Detail` in both, the gauge's own cells included
- **AND** rendering the 120-column case at `Route::List` gives the identical 78 characters
  with `Modifier::DIM` set and `Modifier::BOLD` clear, so the emphasis discriminates rather
  than asserting a constant

#### Scenario: Moving the selection moves the header

- **WHEN** the same dashboard is rendered at 120x20, then `selected` is set to `1` and it is
  rendered again, at 120x20 and at 60x20
- **THEN** the first buffer's detail header names `add-token-refresh` with a gauge of five
  `█` and seven `░`, and the later two name `fix-empty-basket` with `[7/7]` and a gauge of
  twelve `█` and no `░`
- **AND** the assertion discriminates rather than asserting a constant, because the two
  headers differ in the name field, the gauge run, and the progress cell alike

#### Scenario: The gauge is present on an artifact tab that is not the tracked-tasks one

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change is `add-token-refresh` (4
  of 9, schema `tdd`), with the tab selected being `proposal` — an artifact `schema-artifacts`
  does not mark as tracking tasks — is rendered at 120x20 and at 60x20
- **THEN** each buffer's heading row carries the 12-column gauge and `[4/9]`
- **AND** no row of either buffer's content area holds `ui::tasks::progress_bar`'s output —
  no percent cell appears anywhere in the frame — so the gauge reached the reader by the
  header and not by the tracked-tasks tab having been selected
- **AND** switching to the tracked-tasks tab and rendering again leaves the heading row's
  gauge byte-identical while the content area now also holds the tab's own wider bar, the
  two agreeing about the same change because both are `gauge_of` over the same `progress`

#### Scenario: An archived change's header carries its stripped name and its own schema

- **WHEN** a `Dashboard` whose only change is the archived `2026-08-14-add-auth`, whose
  `Change::name` is `add-auth`, whose schema is `spec-driven`, and whose progress is 7 of 7,
  with `selected: 0`, is rendered at 120x20 and at 60x20
- **THEN** each buffer's detail header begins `add-auth`, not `2026-08-14-add-auth`, and
  carries `(spec-driven)`, a gauge of twelve `█`, and `[7/7]`
- **AND** the 78-column header's name field is 45 columns and the 58-column header's is 25,
  the 13-column schema cell having taken from the name field and not from the gauge
- **AND** no date field appears in the header: the ten-column date field is `change-rows`'
  row grammar, not this one

#### Scenario: An empty visible list leaves the whole detail interior blank

- **WHEN** a `Dashboard` with `changes::empty_set()` is rendered at 120x20 at `Route::List`,
  and a second `Dashboard` holding one change `alpha` whose `filter.query` is `zzz` — so
  `visible()` is empty — is rendered at 120x20 and at 60x20 at `Route::Detail`
- **THEN** in every one of those buffers each cell of the detail region's heading row, its
  padding row, **and** its interior is a space whose `Style` equals
  `ratatui::buffer::Cell::default().style()`
- **AND** no `█` or `░` appears anywhere in any of those buffers
- **AND** rendering does not panic at either width

### Requirement: The detail header's cells are measured in display columns

`ui::detail::header_row` SHALL return a string of exactly `width` **display columns** as
`responsive-layout` defines them, not `width` `char`s, and every quantity in its grammar
SHALL be measured the same way: the schema cell's width, the gauge cell's 12 columns, the
progress cell's width, the name field's `width - 3 - schema_columns - 12 - progress_columns`
budget, and the four band boundaries that follow from them. `ui::detail` SHALL reach the
measure only through `layout::columns` and `layout::truncate_columns` and SHALL name no
`char` count of its own.

The progress cell is ASCII by construction and the schema cell is a schema name in
parentheses — `tdd` here, and every schema name the CLI has ever reported is ASCII — so for
this repository the band boundaries `width >= 26`, `13 <= width <= 25`, `7 <= width <= 12`,
and `1 <= width <= 6` are unchanged by the measure. The **name field** is the cell that can
carry a wide character, because a change directory name is arbitrary, and it is where the
measure genuinely changes: a CJK change name previously consumed roughly twice the columns
its budget allowed, pushing the schema and progress cells off the row and, at the wide
layout, past the frame's last column — which at the wide layout is the divider column beside
it, the detail region having no right gutter of its own.

The **gauge cell** cannot carry a wide character in the sense the name field can: it is a run
of exactly two possible characters, both of which measure one column. `█` (U+2588) and `░`
(U+2591) are East Asian Width **Ambiguous**, which `unicode-width`'s default — and therefore
`ratatui::buffer::Buffer::set_string`'s — resolves to one, so a twelve-character run is a
twelve-column one and the row's arithmetic balances. In a CJK-locale terminal resolving
Ambiguous to two columns the gauge paints double-width and the header row overruns its
region. That is the same accepted, uncompensated exposure `SPEC.md` already records for the
glyphs `markdown-constructs` and `markdown-legibility` introduced, and for this very gauge as
`tasks-progress-bar` already draws it one region below; this change widens the standing
exposure rather than creating a new kind of one, and compensates for it no more than the
crate already does anywhere else.

Because the field is padded by `ui::list::pad_or_truncate_right`, which `change-rows`
requires to return exactly `width` columns in both arms, the header row measures exactly
`width` columns even when a wide cluster is dropped whole and the truncated name lands one
column short. The tab bar `artifact-tabs` draws is unaffected: its cells are artifact ids,
which the schema fixes. `pane-chrome` moves the tab bar from the row directly below this one
to two rows below it, with the region's padding row between; nothing about this measure
depends on which.

`header_row` SHALL remain total: no panic at any width, for any name — including an empty
one, one longer than the row, one holding wide characters, emoji, combining marks, or a
zero-width-joiner sequence — for any schema name, and for any `Progress`.

#### Scenario: A CJK change name keeps the header inside its region at both mandated widths

- **WHEN** `header_row("日本語の変更名前です", "tdd", Progress { completed: 4, total: 9 },
  78)` and the same call at width `58` are made — a name of ten characters and twenty
  display columns
- **THEN** each result's `layout::columns` is exactly 78 and exactly 58
- **AND** each ends with `[4/9]` and carries `(tdd)`, a space, a twelve-column gauge, and a
  space immediately before it, so neither cell was pushed off the row and the gauge sits
  between them rather than at either end
- **AND** each result's `chars().count()` is strictly less than its `layout::columns`, which
  is what proves the padding was computed in columns rather than characters — a claim the
  gauge does not weaken, since its twelve characters are also twelve columns and so
  contribute equally to both counts

#### Scenario: The header reaches the buffer without crossing the region border

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
what the header must not cross is now the region's right gutter column.

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change is named
  `日本語の変更名前です`, whose schema is `tdd`, and whose `progress` is
  `Progress { completed: 4, total: 9 }` is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer the detail region's heading row spans columns 42
  through 119 and ends with `[4/9]` in the frame's last column, preceded by a twelve-column
  gauge holding five `█` and seven `░`
- **AND** in the 60-column buffer that row spans columns 1 through 58 and column 59 is a
  space, the narrow region taking `Gutters::Both`
- **AND** in the 120-column buffer columns 39 and 41 are spaces and column 40 holds the
  divider `│`, unchanged from the same render with an ASCII name — so no gauge character
  bled left across the divider, which is the failure a gauge measured in `char`s beside a
  wide name would produce

#### Scenario: The header is total over adversarial names at every width

- **WHEN** `header_row` is called at every width from `0` through `130`, with schema `tdd`
  and each of `Progress { completed: 4, total: 9 }`, `{ completed: 0, total: 0 }`,
  `{ completed: 0, total: usize::MAX }`, and `{ completed: usize::MAX, total: usize::MAX }`,
  for each of: a 200-column CJK name; a family emoji joined by two zero-width joiners; `e`
  followed by five combining accents; a name holding a NUL; and the empty string
- **THEN** no call panics at any width for any of the twenty combinations
- **AND** at every width at or above 1 every result's `layout::columns` is exactly that
  width, and at width `0` every result is the empty string
- **AND** for `Progress { completed: 4, total: 9 }` specifically, at widths 26, 25, 13, 12,
  7, 6, and 1 the cells drop in the documented order for every one of the five names, so all
  four band boundaries hold for wide content as well as for ASCII. The band clause is scoped
  to that fixture because the boundaries are derived from a **five**-column progress cell:
  `Progress { completed: 0, total: usize::MAX }` renders a 24-column cell and
  `{ completed: usize::MAX, total: usize::MAX }` a 43-column one, which move every boundary
  — at width 26 the first is already in the schema-dropped band and the second is down to
  the name field alone. Asserting the documented boundaries across all four values would be
  a requirement no correct implementation could satisfy
- **AND** for the two wide-cell `Progress` values the claims are the width-exactness and
  no-panic ones above, plus the drop-whole property: wherever a result contains `[` it
  contains the whole progress cell
- **AND** no result of a `total == 0` call contains `█` or `░` at any width, so the
  no-gauge rule survives every adversarial name rather than only the ASCII fixture
