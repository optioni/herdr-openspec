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
  arithmetic, truncating rather than rounding, computed in `u128` so the product cannot
  overflow for any `usize` pair. It SHALL be absent entirely when `total == 0`.
- The **gauge** SHALL be a bare run of exactly `g` characters, `filled` of them `█`
  (U+2588) followed by `g - filled` of them `░` (U+2591), where `g` is whatever width
  remains after the other fields and their separating spaces, and
  `filled = g * completed / total` in integer arithmetic computed in `u128`, so the product
  cannot overflow for any `usize` pair and no value of `g`. It
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

**The arithmetic is `u128`, not a saturating `u64` multiply.** This requirement previously
specified a saturating multiply, which silently produced the wrong quotient rather than the
wrong magnitude: `u64::MAX.saturating_mul(g)` saturates to `u64::MAX`, and `u64::MAX /
u64::MAX == 1`, so at `Progress { completed: usize::MAX, total: usize::MAX }` the gauge
rendered **one** filled cell and the percent cell read **1%** for a change this capability
calls complete — at `g == 12`, and equally at the 68- and 48-column gauges this bar draws at
its own two mandated interior widths. `usize::MAX * 100` and `usize::MAX * g` both fit in
`u128` for every `u16` `g`, so widening removes the saturation rather than special-casing
around it, and the formula above is then literally true instead of contradicting the
completeness requirement below. For every `Progress` a repository of task files can produce
the quotient is unchanged, so no rendered output moves except at that one saturating input.

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
  whose one section holds nine task lines, is rendered at 120x20 and at 60x20
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

#### Scenario: A saturating `Progress` renders a full gauge and a full percentage

- **WHEN** `progress_bar` is called with `Progress { completed: usize::MAX, total: usize::MAX }`
  at widths `78` and `58`
- **THEN** each result's gauge holds no `░` at all and its percent cell reads `100%`
- **AND** against the previous saturating-`u64` arithmetic each gauge held exactly one `█`
  and the percent cell read `1%`, which is the discriminating comparison: this scenario fails
  against the implementation this requirement shipped with
- **AND** `progress_bar` with `Progress { completed: 4, total: 9 }` at the same two widths is
  byte-identical to before, so the widening moved exactly the saturating input and nothing else

### Requirement: The gauge is full exactly when the change is complete

`filled == g` SHALL hold if and only if `progress.is_complete()` — `total > 0 && completed
== total`, the same three-way split `Progress::is_complete` already mirrors from the
OpenSpec CLI. A partially complete change SHALL therefore never render a full gauge, and a
complete one SHALL never render a gauge with an empty cell in it.

`filled == 0` SHALL hold whenever `completed == 0`, so an untouched change renders an empty
gauge rather than a sliver.

Both properties SHALL hold for **every** `Progress` and every `g`, with no saturation regime
excepted. They did not, as shipped: the saturating `u64` multiply this capability previously
specified made `filled == 1` at `Progress { completed: usize::MAX, total: usize::MAX }` while
`is_complete()` was true, falsifying the "if and only if" at the header's 12-column budget and
at this bar's own 68- and 48-column gauges. The `u128` arithmetic above removes the exception
rather than documenting it, so this requirement needs no qualifying clause.

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

#### Scenario: The completeness property holds at the saturation boundary

- **WHEN** `gauge_of` is called with `Progress { completed: usize::MAX, total: usize::MAX }`
  at `g` of 12, 48, and 68 — the detail header's budget and the gauges `progress_bar` produces
  at the mandated interior widths `78` and `58`, which the scenario reaches by calling
  `progress_bar` at both as well, so the test names them and needs no `TASKWIDTHS` exemption
- **THEN** each result is exactly `g` characters and holds no `░` at all
- **AND** `progress_bar` at `78` and at `58` for the same `Progress` likewise holds no `░`
- **AND** each held exactly one `█` and `g - 1` `░` under the previous arithmetic, so this
  scenario is the one that fails against the shipped implementation
- **AND** `gauge_of(&Progress { completed: 0, total: usize::MAX }, 12)` holds no `█` at all,
  so widening did not break the `filled == 0` half

## ADDED Requirements

### Requirement: The gauge run is one implementation, shared with the detail header

The `█`/`░` run this capability already specifies SHALL be produced by a single function that
every renderer of it reaches, rather than being formatted a second time where a new renderer
needs it. `progress_bar` already calls `gauge_of` **twice** — once for the full form and once
for the percent-dropped one — so the function has two call sites today and gains a third; what
this change adds is a second *renderer*, the detail header, not a second call site. `ui::tasks::gauge_of(progress: &tasks::Progress, g: u16) -> String` SHALL be raised from
private to `pub(crate)` for that purpose, and `ui::detail::header_row` SHALL call it rather
than constructing a run of its own — the same rule that already makes
`ui::list::progress_cell` the crate's one progress cell, applied to the crate's one gauge.

This is the rule the change it belongs to exists to keep: a header gauge and a tab gauge
drawn from the same `Change::progress` SHALL NOT be able to disagree about how full that
change is, and the only way to guarantee that is for there to be one run to disagree about.

`gauge_of` SHALL become **total**, returning the empty string when `g == 0` or when
`progress.total == 0`. Only the second of those removes a division by zero; at `g == 0` the
function already returns the empty string, since `filled` is `0` and both push loops are empty,
so that arm fixes existing behaviour in place rather than changing it. Neither value is reachable from either
production call site — `progress_bar` returns before reaching it at `total == 0` and draws
no zero-width gauge, and `detail-header` draws no gauge cell at all at `total == 0` — so the
guard changes no rendered output anywhere. It is required because a `pub(crate)` function is
reachable from a call site this capability does not control, and a panicking one would be a
trap laid for the next caller rather than a contract.

The properties this capability already fixes for the run SHALL hold at every `g`, the detail
header's 12 included: `filled == g` if and only if `progress.is_complete()`, `filled == 0`
whenever `completed == 0`, and otherwise `filled = g * completed / total` in integer
arithmetic with a saturating multiply so no `Progress` value can overflow it.

The first of those does **not** hold of the shipped implementation. The repair is specified by
the MODIFIED requirements above — the arithmetic widens to `u128`, which removes the saturation
that produced the wrong quotient — and this requirement inherits it rather than restating a
second mechanism. No existing test falsified the defect because the one sweep reaching
`usize::MAX` (`bar_measures_at_most_its_width_at_every_width`) asserts only that the bar fits
its width and never a fill count.

`progress_bar`'s own output SHALL be **unchanged** — byte-identical at every width — for
every `Progress` whose `completed * g` does not saturate, which is every `Progress` a
repository of task files can produce. Its grammar, its three-field degradation order, its
percent cell, and its `[-]` form for a change with no tasks are all untouched.

The one named exception is the saturating case the MODIFIED requirements above correct: at
`Progress { completed: usize::MAX, total: usize::MAX }` the bar's gauge changes from one
filled cell to a full run, and its percent cell from `1%` to `100%`, at every width that draws
them. That is the defect being fixed showing through, and it is the only input class whose
rendering moves.

#### Scenario: The bar's rendered output does not move

- **WHEN** `progress_bar` is called at every width from `0` through `130` with
  `Progress { completed: 4, total: 9 }`, `{ completed: 0, total: 0 }`,
  `{ completed: 2, total: 3 }`, `{ completed: 0, total: usize::MAX }`, and
  `{ completed: usize::MAX, total: usize::MAX }`
- **THEN** no call panics, and at every width every result's `layout::columns` is at most
  that width
- **AND** the results at `78` and at `58` for `Progress { completed: 4, total: 9 }` are
  exactly a 68-column and a 48-column gauge respectively, each followed by a space, `[4/9]`,
  a space, and `44%`, the 68-column gauge holding exactly 30 `█` and the 48-column gauge
  exactly 21 `█` — the literal expectations this capability landed with, restated here so
  the claim is that the bar did not move and not merely that it still runs
- **AND** the expected strings are built independently of `progress_bar` rather than by
  calling it, on the same terms the existing
  `full_grammar_is_byte_identical_to_pre_change_output` states, so the assertion cannot
  pass by construction
- **AND** `Progress { completed: usize::MAX, total: usize::MAX }` is the one excepted input:
  its gauge is now a full run and its percent cell reads `100%`, where they were a single
  filled cell and `1%`, and the scenario asserts the new values rather than treating the
  widening as a regression

#### Scenario: The header's gauge and the bar's gauge agree about the same change

- **WHEN** `gauge_of(&Progress { completed: 4, total: 9 }, 12)` is called directly, and
  `ui::detail::header_row("add-token-refresh", "tdd", &Progress { completed: 4, total: 9 },
  78)` is called
- **THEN** the header row contains the direct call's twelve-character result as a contiguous
  substring, bounded by a space on each side
- **AND** the same holds at width `58`, and for `Progress { completed: 7, total: 7 }` and
  `{ completed: 0, total: 7 }`, so the agreement is asserted over a full, an empty, and a
  partial gauge rather than at one point

#### Scenario: The gauge is full exactly when the change is complete, at the header's width too

- **WHEN** `gauge_of` is called with `g == 12` for each of
  `Progress { completed: 11, total: 12 }`, `{ completed: 12, total: 12 }`,
  `{ completed: 0, total: 12 }`, `{ completed: 99, total: 100 }`, and
  `{ completed: 100, total: 100 }`, and `progress_bar` is called with the same five at the
  mandated widths `78` and `58` — the route `gauge_full_only_when_complete` already takes, so
  the test names both interiors and needs no `TASKWIDTHS` exemption
- **THEN** each result is exactly twelve characters long
- **AND** each `progress_bar` result's gauge holds no `░` exactly when that
  `Progress::is_complete()` is true, so the property is asserted at the header's budget and at
  both of the bar's own gauges
- **AND** a result holds no `░` exactly when that `Progress::is_complete()` is true, so the
  11-of-12 and 99-of-100 runs each hold at least one `░` and the two complete ones hold none
- **AND** the 0-of-12 run holds no `█` at all
- **AND** `gauge_of(&Progress { completed: usize::MAX, total: usize::MAX }, g)` holds no `░`
  at `g` of 12, 48, and 68 — the header's budget and the bar's two mandated gauges. Under the
  shipped saturating arithmetic each returns a single `█` followed by `g - 1` `░`, so this
  clause is the one that fails before the widening and proves the repair landed

#### Scenario: The promoted function is total at both guard values

- **WHEN** `gauge_of(&Progress { completed: 4, total: 9 }, 0)` is called, and
  `gauge_of(&Progress { completed: 0, total: 0 }, 12)` is called
- **THEN** neither call panics and both return the empty string
- **AND** only the second is a behaviour change: the first already returns the empty string
  against the implementation this requirement is written for, so it is a characterization and
  the `total == 0` call is the one that fails
- **AND** `progress_bar`'s own results for `Progress { completed: 0, total: 0 }` at widths
  `78`, `58`, `3`, and `2` are still exactly `[-]`, `[-]`, `[-]`, and the empty string, so
  the guard did not reroute a path this capability already specifies
