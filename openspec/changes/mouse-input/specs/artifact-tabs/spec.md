## ADDED Requirements

### Requirement: A column of the tab bar resolves to the tab drawn there

`ui::detail::tab_at(artifacts: &[ArtifactRef], selected: usize, width: u16, column: u16) ->
Option<usize>` SHALL return the artifact position of the tab cell `tab_bar` places over
`column` — an offset from the bar's own first column — and `None` when that column holds no
addressable cell.

A column holds a cell exactly when it falls within `[cell.x, cell.x + columns(cell.text))`
for a `Tab` whose `index` is `Some`. The one separating column between two chips belongs to
neither and SHALL return `None`; so SHALL a column past the last drawn cell, a column
covered only by the `no artifacts` placeholder (whose `index` is `None`), and every column
when `width` is `0`.

`tab_at` SHALL be built from `tab_bar`'s own output rather than from a second placement
calculation, so the cell a click lands on and the cell painted there are the same cell —
the same reason `Tab` reports its `x` rather than letting the view derive one. It SHALL
measure a cell's width through `crate::ui::layout::columns`, never a `char` count, so a
bar whose artifact ids carry a CJK or emoji character targets the cell the reader sees.

`tab_at` SHALL be pure and total: no I/O, no clock, and no panic for any artifact slice
including an empty one, any `selected` including one past the end, any `width`, and any
`column`.

#### Scenario: Each of the five tdd cells answers for its own columns

- **WHEN** `tab_at` is called for every column from `0` to `width` against the five `tdd`
  artifacts with `selected` `0`, at the mandated interior widths 78 and 58
- **THEN** every column inside a cell's painted span returns that cell's own index
- **AND** every separating column between two cells returns `None`
- **AND** every column past the last drawn cell returns `None`
- **AND** the set of columns returning `Some(i)` is exactly the span `tab_bar` reported for
  cell `i`, at both widths

#### Scenario: A windowed bar answers for the cells it actually drew

- **WHEN** a twelve-artifact change has `selected` `9`, so `tab_bar` windows to a run that
  does not start at `0`, and `tab_at` is called for every column at width 78
- **THEN** no column returns an index outside the drawn window
- **AND** the first drawn cell's own columns return that cell's index, which is not `0`

#### Scenario: The placeholder and the degenerate widths address nothing

- **WHEN** `tab_at` is called against an empty artifact slice at width 78, then against the
  five `tdd` artifacts at width `0`, then with `selected` past the end of the list, then at
  `column` `65535`
- **THEN** every call returns `None` without panicking
- **AND** a `selected` past the end still returns `Some` for the columns of the cells the
  bar drew, since windowing treats it as `0` and the cells are addressable regardless of
  which one is marked

#### Scenario: A wide artifact id is addressed by its display columns

- **WHEN** an artifact id containing a two-column character is placed in the bar and
  `tab_at` is called for every column of its cell
- **THEN** both columns of that character return the cell's own index
- **AND** the first column of the next cell returns the next cell's index, so no column is
  attributed to the wrong tab

### Requirement: A click on a tab cell switches to it at both routes

A left press inside a drawn tab cell SHALL produce `Action::SelectTab(i)` for that cell's
own artifact position, at `Route::List` and at `Route::Detail` alike above the breakpoint,
and at `Route::Detail` below it — wherever the tab bar is drawn. It SHALL NOT change the
route: above the breakpoint the content is already visible, and below it the bar is only
drawn at the detail route.

Applying it SHALL do exactly what the matching digit key does: set `detail.tab` to `i` when
`i` is within the selected change's artifact list, and reset `detail.scroll` to `0` exactly
when `detail.tab` changed value.

#### Scenario: A tab click and its digit key are indistinguishable

- **WHEN** a change carrying the five `tdd` artifacts is selected at 120x40 with
  `detail.tab` `0` and `detail.scroll` `7`, and the dashboard is driven once by a left press
  inside the fourth cell and once by `Char('4')`
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** both leave `route` at `Route::List`, so clicking a tab does not open the detail

#### Scenario: A tab click on the already-selected cell resets nothing

- **WHEN** `detail.tab` is `2`, `detail.scroll` is `5`, and a left press lands inside the
  third cell
- **THEN** `detail.tab` stays `2` and `detail.scroll` stays `5`, exactly as pressing `3`
  does
