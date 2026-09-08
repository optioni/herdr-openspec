# tasks-progress-bar Specification

## Purpose
The single line that leads the tracked-tasks tab: a bare `█`/`░` gauge, then
`ui::list::progress_cell`'s `[<completed>/<total>]`, then a truncated integer percentage,
separated by one space each and filling exactly the interior width. It fixes that the gauge
is full if and only if the change is complete and empty whenever nothing is done, that the
fields degrade by being dropped whole in a fixed order — percentage, then gauge, then the
whole line — as the width narrows, and that a change with no tasks shows `[-]` alone rather
than an invented `0%`. The number it renders is the change's own `progress` field, shared with
the list row and the detail header, never a recount of the source the checklist beneath it
parses.

## Requirements

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
- **AND** the two results are **byte-identical** to the ones this requirement produced before
  display-column measurement, which is the discriminating claim: it fails if the gauge run's
  length was recomputed against a different measure, whereas comparing `layout::columns` to
  `chars().count()` for an all-width-1 fixture is a tautology and could not

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

### Requirement: The gauge is full exactly when the change is complete

`filled == g` SHALL hold if and only if `progress.is_complete()` — `total > 0 && completed
== total`, the same three-way split `Progress::is_complete` already mirrors from the
OpenSpec CLI. A partially complete change SHALL therefore never render a full gauge, and a
complete one SHALL never render a gauge with an empty cell in it.

`filled == 0` SHALL hold whenever `completed == 0`, so an untouched change renders an empty
gauge rather than a sliver.

#### Scenario: A one-task-short change never renders a full gauge

- **WHEN** `progress_bar` is called at width `78` and at width `58` with
  `Progress { completed: 99, total: 100 }`
- **THEN** at each width the gauge holds at least one `░`
- **AND** with `Progress { completed: 100, total: 100 }` at the same two widths the gauge
  holds no `░` at all and the percent cell reads `100%`
- **AND** with `Progress { completed: 0, total: 100 }` the gauge holds no `█` at all and
  the percent cell reads `0%`

#### Scenario: The property holds across a swept range of gauge widths

- **WHEN** `progress_bar` is called for every width from `0` to `120` inclusive, for each
  of `Progress { 0, 0 }`, `{ 0, 1 }`, `{ 1, 1 }`, `{ 1, 2 }`, `{ 3, 7 }`, and
  `{ 999, 1000 }`
- **THEN** no call panics, and no returned string exceeds its width in `char`s
- **AND** for every result carrying a gauge, the gauge holds no `░` exactly when that
  `Progress::is_complete()` is true
- **AND** the widths `58` and `78` are among those swept, so the mandated pair is asserted
  by this scenario as well as by its own

### Requirement: Cells are dropped whole as the bar narrows

A field too wide for the line SHALL be dropped **whole**, never cut short, in a fixed
order — the same degradation grammar `change-rows` and `detail-header` already use:

1. the **percent cell** first, reclaiming its separating space;
2. then the **gauge**, reclaiming its separating space, leaving the count cell alone;
3. and when even the count cell does not fit, the empty string.

The gauge SHALL be drawn only when at least one gauge column remains: a zero-width gauge is
dropped rather than rendered as two adjacent spaces.

`width == 0` SHALL return the empty string before any other branch.

#### Scenario: The three fields fall away in order as the width collapses

- **WHEN** `progress_bar` is called with `Progress { completed: 4, total: 9 }` at widths
  `78`, `58`, `11`, `10`, `7`, `6`, `5`, `4`, `1`, and `0`
- **THEN** no call panics and no result exceeds its width
- **AND** at `78`, `58`, and `11` the result carries a gauge, `[4/9]`, and `44%`
- **AND** at `10` and at `7` the result carries a gauge and `[4/9]` and no `%` at all — the
  percent cell was dropped whole rather than truncated to `4`
- **AND** at `6` and at `5` the result is exactly `[4/9]` — the gauge was dropped whole
  rather than rendered as a single ambiguous cell of leftover width
- **AND** at `4`, `1`, and `0` the result is the empty string, because the count cell is
  five characters and there is no shorter honest form of it

#### Scenario: No partial cell is ever emitted

- **WHEN** `progress_bar` is called for every width from `0` to `30` inclusive, and at `58`
  and `78`, with `Progress { completed: 4, total: 9 }`
- **THEN** every result either contains `[4/9]` in full or contains no `[` at all
- **AND** every result either ends in `%` preceded by a complete number or contains no `%`
  at all
- **AND** no result contains the two-character sequence of two spaces, which is what a
  zero-width gauge left in place would produce

### Requirement: A change with no tasks renders the count cell alone

When `progress.total == 0` the bar SHALL be exactly the three characters `[-]` at any width
of three or more, and the empty string below that. No gauge and no percent cell SHALL be
drawn: a gauge with no denominator would have to invent a fill, and `0%` for a change that
has no tasks says something false about it.

This is the same `[-]` the list row and the detail header already show for such a change,
because it is the same `ui::list::progress_cell` call.

#### Scenario: A change with no tasks shows `[-]` and nothing else

- **WHEN** `progress_bar` is called with `Progress { completed: 0, total: 0 }` at widths
  `78`, `58`, `4`, `3`, `2`, `1`, and `0`
- **THEN** the results at `78`, `58`, `4`, and `3` are all exactly `[-]`, unpadded
- **AND** the results at `2`, `1`, and `0` are the empty string
- **AND** no result contains `█`, `░`, or `%` at any width

#### Scenario: The no-tasks bar reaches the buffer at both frame widths

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change's tracked-tasks tab is
  selected, whose `progress` is `Progress { completed: 0, total: 0 }`, and whose
  `detail.source` is `# Plan\n\nprose only\n`, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer row 4 columns 41 through 43 read `[-]` and column 44 is
  a space belonging to the untouched rest of the row
- **AND** in the 60-column buffer row 4 columns 1 through 3 read `[-]`
- **AND** in each buffer row 6 reads `No tasks yet`, which is `tasks-checklist`'s state for
  a source holding no task **items** — `# Plan` is an ATX heading, so `tasks::parse` returns
  one group and zero items, and neither `# Plan` nor any other heading line appears
