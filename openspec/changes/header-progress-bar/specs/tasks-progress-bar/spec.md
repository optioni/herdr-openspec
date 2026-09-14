## ADDED Requirements

### Requirement: The gauge run is one implementation, shared with the detail header

The `█`/`░` run this capability already specifies SHALL be produced by a single function that
both call sites reach, rather than being formatted a second time where a second caller needs
it. `ui::tasks::gauge_of(progress: &tasks::Progress, g: u16) -> String` SHALL be raised from
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

The properties this capability already fixes for the run SHALL continue to hold at every `g`,
the detail header's 12 included: `filled == g` if and only if `progress.is_complete()`,
`filled == 0` whenever `completed == 0`, and `filled = g * completed / total` in integer
arithmetic with a saturating multiply so no `Progress` value can overflow it.

`progress_bar`'s own output SHALL be **unchanged** — byte-identical at every width for every
`Progress` — by this change. Its grammar, its three-field degradation order, its percent
cell, and its `[-]` form for a change with no tasks are all untouched: the only edit this
capability takes is a visibility keyword and a guard on a branch neither of its call sites
reaches.

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

#### Scenario: The promoted function is total at both guard values

- **WHEN** `gauge_of(&Progress { completed: 4, total: 9 }, 0)` is called, and
  `gauge_of(&Progress { completed: 0, total: 0 }, 12)` is called
- **THEN** neither call panics and both return the empty string
- **AND** `progress_bar`'s own results for `Progress { completed: 0, total: 0 }` at widths
  `78`, `58`, `3`, and `2` are still exactly `[-]`, `[-]`, `[-]`, and the empty string, so
  the guard did not reroute a path this capability already specifies
