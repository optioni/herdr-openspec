## MODIFIED Requirements

### Requirement: The progress bar's grammar

`ui::tasks::progress_bar(progress: &tasks::Progress, width: u16) -> String` SHALL produce
the single line that leads the tracked-tasks tab's content, in the full form

```
<gauge> <count cell> <percent cell>
```

with exactly one space between the three fields and no padding on either end, so a region
draws it left to right and leaves the rest of its row untouched, exactly as
`markdown-render` does.

- The **count cell** SHALL be `ui::list::progress_cell(progress)` — `[<completed>/<total>]`,
  or the three characters `[-]` when `total == 0`. It SHALL be that one function and not a
  second formatting of the same pair, so the bar, the detail header (`detail-header`), and
  the list row (`change-rows`) can never disagree about a change's progress.
- The **percent cell** SHALL be `<n>%` where `n` is `completed * 100 / total` in integer
  arithmetic, truncating rather than rounding, computed with a saturating multiply so no
  `Progress` value can overflow it. It SHALL be absent entirely when `total == 0`.
- The **gauge** SHALL be a bare run of exactly `g` characters, `filled` of them `█`
  (U+2588) followed by `g - filled` of them `░` (U+2591), where `g` is whatever width
  remains after the other fields and their separating spaces, and
  `filled = g * completed / total` in integer arithmetic with a saturating multiply. It
  SHALL carry no surrounding brackets: the count cell already carries a bracket pair and a
  second one beside it reads as noise.

The returned string SHALL be at most `width` **display columns**, measured by
`layout::columns` as `responsive-layout` defines it, and SHALL be exactly `width` columns in
the full form. Every character the bar can hold is one column wide — `█` and `░` are
East Asian Width **Ambiguous**, which `unicode-width`'s default, and therefore ratatui's,
resolves to 1, and the count and percent cells are ASCII — so this restatement changes no
rendered output at any width. It is made because the crate now has exactly one unit for a
rendered length, and a requirement still counting `char`s would be the one place a reader
could not tell which measure was meant. The cell-dropping order below is likewise unchanged
and now evaluated in columns.

`ui::tasks` SHALL reach the measure only through `layout::columns` and
`layout::truncate_columns`; `progress_bar` SHALL name no `char` count of its own.

The line SHALL be rendered as one segment carrying `Face::plain()`, so `ui::view::style_for`
needs no new `Face`-to-`Style` mapping and `ui::tasks` needs no `ratatui` type.

The progress the bar renders SHALL be the selected `Change`'s own `progress` field — the
value `change-artifacts` computed and `change-merge` may have replaced with the CLI's — and
SHALL NOT be recounted from the rendered source. `tasks-checklist` renders the parse; this
capability renders the count, and the two SHALL NOT be two different numbers for the same
change on the same frame.

#### Scenario: The full grammar at both mandated interior widths

- **WHEN** `progress_bar` is called with `Progress { completed: 4, total: 9 }` at width
  `78` and at width `58`
- **THEN** the 78-column result is exactly 78 display columns: a 68-column gauge, a space,
  `[4/9]`, a space, and `44%`
- **AND** the 58-column result is exactly 58 display columns: a 48-column gauge, a space,
  `[4/9]`, a space, and `44%`
- **AND** the 68-column gauge holds exactly 30 `█` and 38 `░`, and the 48-column
  gauge exactly 21 `█` and 27 `░`, so the fill is `g * completed / total` truncated
- **AND** both results' `layout::columns` equals their `chars().count()`, which is what
  proves the gauge glyphs measure one column each and the restatement changed no output

#### Scenario: The bar reaches the buffer at both mandated frame widths

- **WHEN** a `Dashboard` at `Route::Detail`, whose selected change's tracked-tasks tab is
  selected, whose `progress` is `Progress { completed: 4, total: 9 }`, and whose
  `detail.source` holds nine task lines, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer row 4 columns 41 through 118 hold the 78-column bar,
  ending `[4/9] 44%`
- **AND** in the 60-column buffer row 4 columns 1 through 58 hold the 58-column bar,
  ending `[4/9] 44%`
- **AND** in each buffer row 5 is blank in the content area and row 6 holds the first
  checklist line, so the bar and its separator took two rows from the checklist rather than
  being drawn over it

#### Scenario: The percentage truncates rather than rounds

- **WHEN** `progress_bar` is called at width `78` and at width `58` with
  `Progress { completed: 2, total: 3 }`, then `{ completed: 1, total: 3 }`, then
  `{ completed: 0, total: 7 }`, then `{ completed: 7, total: 7 }`
- **THEN** the percent cells read `66%`, `33%`, `0%`, and `100%` respectively at both widths
- **AND** `66%` rather than `67%` proves the truncation, and the full form is still exactly
  `width` display columns at every one of them

#### Scenario: The bar measures at most its width at every width

- **WHEN** `progress_bar` is called at every width from `0` through `130` with
  `Progress { completed: 4, total: 9 }`, `{ completed: 0, total: 0 }`,
  `{ completed: 0, total: usize::MAX }`, and `{ completed: usize::MAX, total: usize::MAX }`
- **THEN** no call panics at any width for any of the four
- **AND** at every width every result's `layout::columns` is at most that width, and equals
  it whenever the full form was returned
