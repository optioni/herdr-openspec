## MODIFIED Requirements

### Requirement: The progress bar's grammar

`ui::tasks::progress_bar(progress: &tasks::Progress, groups: &[tasks::Progress], width: u16)
-> String` SHALL produce the single line that leads the tracked-tasks tab's content, in the
full form

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
- The **gauge** SHALL be a bare run of exactly `g` characters, `filled` of them filled and
  `g - filled` of them empty, where `g` is whatever width remains after the other fields and
  their separating spaces, and `filled = g * completed / total` in integer arithmetic
  computed in `u128`, so the product cannot overflow for any `usize` pair and no value of
  `g`. It SHALL carry no surrounding brackets: the count cell already carries a bracket pair
  and a second one beside it reads as noise. Which **glyph** each of the `g` positions is
  drawn with is the segmentation requirement below; where that requirement does not segment —
  and in particular whenever `groups` is empty — the filled glyph SHALL be `█` (U+2588) and
  the empty one `░` (U+2591), exactly as before this change.

`groups` SHALL be one `Progress` per task group, in document order, as `tasks-checklist`
states each of its two callers derives them — and on the **foldable** path, which is the path
every real `tasks.md` takes, that slice SHALL be populated from `detail.sections`' own
`progress` values rather than left empty. It SHALL affect **only** which glyph each position is drawn
with — never `g`, never `filled`, never either cell, and never the drop-whole order. An
**empty slice** SHALL therefore produce a byte-identical line to the one this capability
produced before this change, at every width and for every `Progress`, which is what the
detail header and the non-foldable path both rely on.

The returned string SHALL be at most `width` **display columns**, measured by
`layout::columns` as `responsive-layout` defines it, and SHALL be exactly `width` columns in
the full form. Every character the bar can hold is one column wide — `█`, `░`, and the two
further shades the segmentation requirement introduces, `▓` (U+2593) and `▒` (U+2592), are
all East Asian Width **Ambiguous**, which `unicode-width`'s default, and therefore ratatui's,
resolves to 1, and the count and percent cells are ASCII — so this restatement changes no
rendered output at any width. The two new glyphs **widen** the accepted, uncompensated
CJK-locale exposure `SPEC.md` records rather than creating a new one: a terminal resolving
Ambiguous to two columns already painted this gauge at twice its width. It is made because the crate now has exactly one unit for a
rendered length, and a requirement still counting `char`s would be the one place a reader
could not tell which measure was meant. The cell-dropping order below is likewise unchanged
and now evaluated in columns.

`ui::tasks` SHALL reach the measure only through `layout::columns` and
`layout::truncate_columns`; `progress_bar` SHALL name no `char` count of its own.

The line SHALL be rendered as one segment carrying `Face::plain()`, so `ui::view::style_for`
needs no new `Face`-to-`Style` mapping and `ui::tasks` needs no `ratatui` type. Segmentation
SHALL be carried by the **glyphs**, not by a face: a segment per group would be a per-group
style the palette would then have to name, and the shade alternation already says the whole
of what a boundary means.

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

## ADDED Requirements

### Requirement: The gauge is segmented by group, proportionally, and only when a segment is legible

When `groups` holds **two or more** entries whose `total` is non-zero and the gauge is wide
enough, `ui::tasks` SHALL mark each group's span of the gauge by **alternating shade** rather
than by a separator character. The glyph pairs SHALL be:

| Position falls in | Filled | Empty |
|---|---|---|
| an even-indexed contributing group | `█` U+2588 | `░` U+2591 |
| an odd-indexed contributing group | `▓` U+2593 | `▒` U+2592 |

A **contributing group** is one whose `Progress::total` is greater than zero. A group holding
no items SHALL contribute no span and SHALL NOT consume an index, so two groups left adjacent
after empty ones are dropped still alternate — an empty group that took an index would give
two neighbours the same shade and erase the boundary between them.

Separator characters SHALL NOT be used. At the measured maximum of 22 groups, 21 separators
would consume 21 of the narrow interior's ~47 gauge columns, leaving the gauge itself less
than half the row.

**Spans are proportional to item count, by cumulative flooring.** With `n` contributing groups
whose totals are `t_0 … t_{n-1}` and `T` their sum, group `i`'s span SHALL be the half-open
range `[e_{i-1}, e_i)` where `e_i = floor(g * (t_0 + … + t_i) / T)` computed in `u128` and
`e_{-1} = 0`. The spans therefore partition the `g` positions exactly, with no position
unassigned and none assigned twice, for every `g` and every set of totals — which is a
property of cumulative flooring and SHALL be asserted as one rather than derived per fixture.
`T` SHALL be the **sum of the slice**, never `progress.total`: the bar's own `progress` is the
`Change`'s field, which `change-merge` may have replaced with the CLI's count, and the two are
allowed to disagree. A disagreement moves no rendered output here, because segmentation
chooses glyph variants and never how many positions are filled.

**Segmentation SHALL be skipped entirely** — every position drawn with the `█`/`░` pair — when
any of these holds:

- `groups` is empty, or holds fewer than two contributing groups;
- `T` is zero;
- `g < 2 * n`.

The last is the legibility floor, stated in columns rather than assumed: a one-column span
cannot be read as a shade run, so a gauge that cannot give every group two columns SHALL show
no boundaries at all rather than unreliable ones. At the measured worst case — 22 groups at
the narrow interior's ~47 gauge columns — `2 * 22 = 44 <= 47`, so the pane segments there, and
at the wide interior's ~67 it segments comfortably. A pane narrow enough to fail the test
degrades to the pre-change gauge, which is a supported rendering rather than a fallback.

The **fill count SHALL NOT move.** Segmentation SHALL be expressed as a glyph substitution over
the run `gauge_of(progress, g)` already returns: each position keeps whether it is filled or
empty and changes only which of the two shades it is drawn with. Every property the gauge
already carries therefore holds unchanged and **by construction** rather than by a second
assertion — `filled == g` if and only if `progress.is_complete()`, `filled == 0` whenever
`completed == 0`, and the `u128` arithmetic the requirement above fixes. There SHALL be no
second fill computation anywhere in the crate, and `gauge_of` SHALL keep the signature and the
output `header-progress-bar` gave it, so `detail-header`'s own gauge is untouched by this
change in every respect.

#### Scenario: Two groups of unequal size get spans proportional to their item counts

- **WHEN** `ui::tasks::progress_bar` is called at the two mandated interior widths, `78` and
  `58`, with `Progress { completed: 3, total: 12 }` and `groups`
  `[Progress { completed: 3, total: 9 }, Progress { completed: 0, total: 3 }]`
- **THEN** at each width the gauge run is `g` columns, of which the first `floor(g * 9 / 12)`
  are drawn with the `█`/`░` pair and the remainder with `▓`/`▒`
- **AND** the number of **filled** positions — `█` and `▓` counted together — is exactly
  `floor(g * 3 / 12)` at each width, the same count the unsegmented gauge produces
- **AND** calling the same function with an **empty** `groups` slice returns a line that
  differs only in that every `▓` is a `█` and every `▒` is a `░`

#### Scenario: An empty group contributes no span and consumes no index

- **WHEN** `progress_bar` is called at width `78` with `Progress { completed: 0, total: 4 }`
  and `groups` `[{0,2}, {0,0}, {0,2}]` — a group holding no items between two that do
- **THEN** the gauge's first half is drawn with `░` and its second half with `▒`, the two
  contributing groups taking indices `0` and `1`
- **AND** every one of the `g` positions is drawn, and none is drawn with a shade belonging to
  the empty group
- **AND** the same call with `groups` `[{0,2}, {0,2}]` produces a byte-identical gauge, so
  dropping the empty group is what the rule does rather than merely what it permits

#### Scenario: Segmentation is skipped below the legibility floor

- **WHEN** `progress_bar` is called with `Progress { completed: 5, total: 22 }` and 22
  contributing groups of one item each, at every width from `0` through `130`
- **THEN** at every width where a gauge is drawn and `g < 44`, the run holds no `▓` and no `▒`
- **AND** at every width where `g >= 44` the run holds at least one glyph of each pair, so the
  floor is a boundary the sweep crosses rather than a condition that never fires
- **AND** at width `58` — the narrow mandated interior — `g` is at least `44` and the bar is
  segmented, which is the measured worst case the floor was chosen against

#### Scenario: A single group is never segmented

- **WHEN** `progress_bar` is called at widths `78` and `58` with
  `Progress { completed: 1, total: 2 }` and `groups` holding exactly one entry, `{1,2}`
- **THEN** the gauge holds no `▓` and no `▒`
- **AND** the line is byte-identical to the same call with an empty `groups` slice, because
  one group has no boundary to mark

#### Scenario: Segmentation is total and partitions the run exactly

- **WHEN** `progress_bar` is called at every width from `0` through `130` with each of:
  `Progress { completed: 0, total: 0 }` and an empty slice; `{ usize::MAX, usize::MAX }` with
  two groups each `{ usize::MAX / 2, usize::MAX / 2 }`; 40 contributing groups of one item
  each; one group whose `total` is `usize::MAX` beside one whose `total` is `1`; and a slice
  of 100 groups all holding zero items
- **THEN** no call panics
- **AND** at every width the returned line measures at most that width in display columns
- **AND** wherever a gauge is drawn, its four glyphs' counts sum to exactly `g`, so no position
  is unassigned or assigned twice
- **AND** at every width the count of **filled** positions — `█` and `▓` together — is equal
  to the count of `█` in the same call made with an **empty** `groups` slice, so the
  substitution provably preserves the fill rather than being asserted to by construction
- **AND** for `{ usize::MAX, usize::MAX }` every position is a filled glyph, `█` or `▓`, and
  the percent cell reads `100%`, so segmentation did not reintroduce the saturation defect the
  requirement above repairs

#### Scenario: A real tasks tab renders a segmented gauge into the frame

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true` and whose one path reads
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n` is
  synced and rendered at 120x20 and at 60x20
- **THEN** at each width the progress-bar row holds at least one `▓` or `▒`, so the pane
  actually draws a segmented gauge rather than only `progress_bar` being able to
- **AND** the first group's span is drawn with the `█`/`░` pair and the second with `▓`/`▒`,
  their widths in the ratio `tasks-checklist`'s two sections' own `progress` totals give — 2
  items against 1
- **AND** the same dashboard whose artifact does **not** track tasks draws no progress-bar row
  at all, and no `▓` or `▒` appears anywhere in either buffer
- **AND** this is the one scenario in this capability that renders through
  `ui::detail::content_lines` rather than calling `progress_bar` directly. Every other
  segmentation scenario passes a hand-built slice, so all of them would pass against a build
  whose production caller passed an empty one — which is the defect this scenario exists to
  catch, and did catch, in planning review
