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
`progress.total == 0` rather than dividing by zero. Neither value is reachable from either
production call site — `progress_bar` returns before reaching it at `total == 0` and draws
no zero-width gauge, and `detail-header` draws no gauge cell at all at `total == 0` — so the
guard changes no rendered output anywhere. It is required because a `pub(crate)` function is
reachable from a call site this capability does not control, and a panicking one would be a
trap laid for the next caller rather than a contract.

The properties this capability already fixes for the run SHALL hold at every `g`, the detail
header's 12 included: `filled == g` if and only if `progress.is_complete()`, `filled == 0`
whenever `completed == 0`, and otherwise `filled = g * completed / total` in integer
arithmetic with a saturating multiply so no `Progress` value can overflow it.

The first of those does **not** hold of the shipped implementation, and this change SHALL
repair it rather than weaken the claim. `filled = completed.saturating_mul(g) / total`
saturates at `Progress { completed: usize::MAX, total: usize::MAX }`, and
`u64::MAX / u64::MAX == 1`, so a change this capability calls complete renders a gauge with
**one** filled cell — at `g == 12`, and equally at the 68- and 48-column gauges the bar
itself draws at its two mandated interior widths. `gauge_of` SHALL therefore return a run of
`g` filled cells whenever `progress.is_complete()`, before computing the quotient. This is a
fix to an already-shipped requirement of this capability, not a new one: the sentence
"`filled == g` SHALL hold if and only if `progress.is_complete()`" is live text, and no
existing test falsified it because the one sweep reaching `usize::MAX`
(`bar_measures_at_most_its_width_at_every_width`) asserts only that the bar fits its width
and never a fill count.

`progress_bar`'s own output SHALL be **unchanged** — byte-identical at every width — for
every `Progress` whose `completed * g` does not saturate, which is every `Progress` a
repository of task files can produce. Its grammar, its three-field degradation order, its
percent cell, and its `[-]` form for a change with no tasks are all untouched.

The one named exception is the saturating case the completeness repair above corrects: at
`Progress { completed: usize::MAX, total: usize::MAX }` the bar's gauge changes from one
filled cell to a full run at every width that draws a gauge. That is the defect being fixed
showing through, and it is the only input class whose rendering moves.

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
  its gauge is now a full run at every width that draws one, where it was a single filled
  cell, and the scenario asserts the new value rather than treating the change as a
  regression

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
  `{ completed: 100, total: 100 }`
- **THEN** each result is exactly twelve characters long
- **AND** a result holds no `░` exactly when that `Progress::is_complete()` is true, so the
  11-of-12 and 99-of-100 runs each hold at least one `░` and the two complete ones hold none
- **AND** the 0-of-12 run holds no `█` at all
- **AND** `gauge_of(&Progress { completed: usize::MAX, total: usize::MAX }, g)` holds no `░`
  at `g` of 12, 48, and 68 — the header's budget and the bar's two mandated gauges. Without
  the completeness short-circuit each of these returns a single `█` followed by `g - 1` `░`,
  so this clause is the one that fails against the shipped implementation and proves the
  repair landed

#### Scenario: The promoted function is total at both guard values

- **WHEN** `gauge_of(&Progress { completed: 4, total: 9 }, 0)` is called, and
  `gauge_of(&Progress { completed: 0, total: 0 }, 12)` is called
- **THEN** neither call panics and both return the empty string
- **AND** `progress_bar`'s own results for `Progress { completed: 0, total: 0 }` at widths
  `78`, `58`, `3`, and `2` are still exactly `[-]`, `[-]`, `[-]`, and the empty string, so
  the guard did not reroute a path this capability already specifies
