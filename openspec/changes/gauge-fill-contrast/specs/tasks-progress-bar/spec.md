## MODIFIED Requirements

### Requirement: The gauge is segmented by group, proportionally, and only when a segment is legible

When `groups` holds **two or more** entries whose `total` is non-zero and the gauge is wide
enough, `ui::tasks` SHALL mark each group's span of the gauge by **alternating glyph** rather
than by a separator character. The glyph pairs SHALL be:

| Position falls in | Filled | Empty |
|---|---|---|
| an even-indexed contributing group | `█` U+2588 FULL BLOCK | `⢕` U+2895 BRAILLE DOTS-1358 |
| an odd-indexed contributing group | `▒` U+2592 MEDIUM SHADE | `⠌` U+280C BRAILLE DOTS-34 |

**The filled half and the empty half SHALL be drawn from different character families** — block
elements when filled, braille patterns when empty — and that, not a difference in lightness, is
what marks where the fill ends. This is the requirement's central change and the reason it was
made: under the previous table the fill boundary (`▓`→`▒`) was a one-step shade change, exactly
the same visual weight as the group boundaries beside it (`█`→`▓`, `░`→`▒`), so every edge in
the bar competed equally and the one a reader actually wants at a glance — how far along the
change is — was the hardest to find. A block-to-dots transition cannot be confused with a
boundary between two blocks or between two dot patterns.

Because the two halves are categorically distinct, **both** halves may carry boundaries without
either blurring the fill edge. `▒` in the filled half SHALL NOT be read as a partially filled
position: nothing in the empty half is a shade, so there is no lightness scale for it to sit on.

The two alternations SHALL be **in phase** — an even-indexed group is `█` where filled and `⢕`
where empty, an odd-indexed group `▒` and `⠌` — so a group straddling the fill boundary keeps
one identity on both sides of it rather than reading as two groups.

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

**Segmentation SHALL be skipped entirely** — every position drawn with the `█`/`░` pair
`gauge_of` itself returns, braille appearing nowhere — when any of these holds:

- `groups` is empty, or holds fewer than two contributing groups;
- `T` is zero;
- `g < 2 * n`.

The last is the legibility floor, stated in columns rather than assumed: a one-column span
cannot be read as a glyph run, so a gauge that cannot give every group two columns SHALL show
no boundaries at all rather than unreliable ones.

`░` U+2591 therefore survives this change and `▓` U+2593 does not. `gauge_of`'s own pair stays
`█`/`░`, which is both the unsegmented bar and — since it is passed no groups and is segmented
by nothing — `detail-header`'s twelve-column gauge. The rule that results is worth stating
plainly, because it is what keeps two vocabularies from becoming a drift: **block shades are
the unsegmented vocabulary, and braille is the boundary vocabulary.** Braille appears exactly
where and when there are boundaries to mark, and nowhere else in the crate.

`g` is **not** `width - 11`. It is `width` less `columns(progress_cell)`, less
`columns(percent_cell)`, less the two separating spaces — and both cells are data-dependent,
so the deduction grows with the change's own task count. The archive's worst case is therefore
tighter than a fixed 11 predicts: `archive/2026-09-06-agent-launch` carries **22** groups and
**81** items, so its cells are `[81/81]` and `100%` — 7 and 4 columns — and at the 58-column
interior `g` is `58 - 7 - 4 - 2 = 45` against a floor of `2 * 22 = 44`. It segments, with
**one** column of headroom rather than three. A change with more groups, or a wider count cell
at the same group count, falls below the floor and degrades to the pre-change gauge — which is
a supported rendering, not a fallback.

The **fill count SHALL NOT move.** Segmentation SHALL be expressed as a glyph substitution over
the run `gauge_of(progress, g)` already returns: each position keeps whether it is filled or
empty and changes only which of the two shades it is drawn with. Every property the gauge
already carries therefore holds unchanged and **by construction** rather than by a second
assertion — `filled == g` if and only if `progress.is_complete()`, `filled == 0` whenever
`completed == 0`, and the `u128` arithmetic the requirement above fixes. There SHALL be no
second fill computation anywhere in the crate, and `gauge_of` SHALL keep the signature and the
output `header-progress-bar` gave it, so `detail-header`'s own gauge is untouched by this
change in every respect.

Every glyph named in the table above SHALL measure **one** display column as
`ui::layout::columns` reports it. Their East Asian Width properties differ and the consequence
SHALL be recorded rather than discovered: `█` and `▒` are **Ambiguous** and `⢕` and `⠌` are
**Neutral** (UCD 16.0.0), so in a terminal configured for a CJK locale the filled half may paint
at two columns per position while the empty half paints at one, leaving the bar
**mis-proportioned** — a 53%-complete change reading as roughly 69%. That is an accepted,
uncompensated exposure on exactly the terms `markdown-render` already records for its own
glyphs, and it is not a regression: the previous table mixed Ambiguous `█`, `▓`, `▒` with
**Neutral** `░`, so the same terminal already mis-proportioned the bar *and* drew its empty half
ragged. This change removes the raggedness and keeps the mis-proportion. Correcting it would
require every glyph in one width class, which in practice means an all-braille bar whose filled
half is dotted and therefore no longer reads as solid — the defect this change exists to
repair.

#### Scenario: Two groups of unequal size get spans proportional to their item counts

- **WHEN** `ui::tasks::progress_bar` is called at the two mandated interior widths, `78` and
  `58`, with `Progress { completed: 3, total: 12 }` and `groups`
  `[Progress { completed: 3, total: 9 }, Progress { completed: 0, total: 3 }]`
- **THEN** at each width the gauge run is `g` columns, of which the first `floor(g * 9 / 12)`
  are drawn with the `█`/`⢕` pair and the remainder with `▒`/`⠌`
- **AND** the number of **filled** positions — `█` and `▒` counted together — is exactly
  `floor(g * 3 / 12)` at each width, the same count the unsegmented gauge produces
- **AND** calling the same function with an **empty** `groups` slice returns a line holding
  only `█` and `░`, with the same filled count and no braille anywhere

#### Scenario: An empty group contributes no span and consumes no index

- **WHEN** `progress_bar` is called at width `78` with `Progress { completed: 0, total: 4 }`
  and `groups` `[{0,2}, {0,0}, {0,2}]` — a group holding no items between two that do
- **THEN** the gauge's first half is drawn with `⢕` and its second half with `⠌`, the two
  contributing groups taking indices `0` and `1`
- **AND** every one of the `g` positions is drawn, and none is drawn with a glyph belonging to
  the empty group
- **AND** the same call with `groups` `[{0,2}, {0,2}]` produces a byte-identical gauge, so
  dropping the empty group is what the rule does rather than merely what it permits

#### Scenario: Segmentation is skipped below the legibility floor

- **WHEN** `progress_bar` is called with `Progress { completed: 5, total: 22 }` and 22
  contributing groups of one item each, at every width from `0` through `130`
- **THEN** at every width where a gauge is drawn and `g < 44`, the run holds no braille glyph
  and no `▒`, every position being `█` or `░`
- **AND** at every width where `g >= 44` the run holds at least one glyph of each pair, so the
  floor is a boundary the sweep crosses rather than a condition that never fires
- **AND** at width `58` — the narrow mandated interior — `g` is at least `44` and the bar is
  segmented, which is the measured worst case the floor was chosen against

#### Scenario: A single group is never segmented

- **WHEN** `progress_bar` is called at widths `78` and `58` with
  `Progress { completed: 1, total: 2 }` and `groups` holding exactly one entry, `{1,2}`
- **THEN** the gauge holds no `▒` and no braille glyph, every position being `█` or `░`
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
- **AND** at every width the count of **filled** positions — `█` and `▒` together — is equal
  to the count of `█` in the same call made with an **empty** `groups` slice, so the
  substitution provably preserves the fill rather than being asserted to by construction
- **AND** at every width no position is drawn with a block glyph and a braille glyph both, and
  the two families never interleave: every braille position lies at or after every block one,
  so the fill boundary is a single transition rather than a scatter
- **AND** for `{ usize::MAX, usize::MAX }` every position is a filled glyph, `█` or `▒`, and
  the percent cell reads `100%`, so segmentation did not reintroduce the saturation defect the
  requirement above repairs

#### Scenario: A real tasks tab renders a segmented gauge into the frame

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true` and whose one path reads
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n` is
  synced and rendered at 120x20 and at 60x20
- **THEN** at each width the progress-bar row holds at least one `▒` or braille glyph, so the
  pane actually draws a segmented gauge rather than only `progress_bar` being able to
- **AND** the first group's span is drawn with the `█`/`⢕` pair and the second with `▒`/`⠌`,
  their widths in the ratio `tasks-checklist`'s two sections' own `progress` totals give — 2
  items against 1
- **AND** the same dashboard whose artifact does **not** track tasks draws no progress-bar row
  at all, and no `▒` and no braille glyph appears anywhere in either buffer
- **AND** this is the one scenario in this capability that renders through
  `ui::detail::content_lines` rather than calling `progress_bar` directly. Every other
  segmentation scenario passes a hand-built slice, so all of them would pass against a build
  whose production caller passed an empty one — which is the defect this scenario exists to
  catch, and did catch, in planning review

#### Scenario: The fill boundary is the only change of character family

- **WHEN** `progress_bar` is called at the two mandated interior widths, `78` and `58`, with
  `Progress { completed: 25, total: 47 }` and the eleven-group slice
  `[{4,4},{4,4},{3,3},{5,5},{8,8},{1,3},{0,4},{0,3},{0,4},{0,3},{0,6}]` — this repository's
  own `mouse-text-selection` at the moment this change was written
- **THEN** at each width the gauge holds exactly one position at which a block glyph is
  followed by a braille glyph, and no position at which a braille glyph is followed by a block
  one
- **AND** that transition falls at index `floor(g * 25 / 47)`, the same index at which `█`
  becomes `░` in the same call made with an **empty** `groups` slice
- **AND** the gauge holds at least one `█`, one `▒`, one `⢕`, and one `⠌`, so the scenario
  exercises all four glyphs rather than passing on a slice that reaches only two

#### Scenario: A group straddling the fill boundary keeps one identity

- **WHEN** `progress_bar` is called at widths `78` and `58` with
  `Progress { completed: 1, total: 4 }` and `groups` `[{1,2}, {0,2}]`, the first group being
  partly complete so its span crosses the fill boundary
- **THEN** at each width group 0's span holds `█` before the boundary and `⢕` after it, both
  being the even-indexed pair
- **AND** group 1's span holds only `⠌`, the odd-indexed empty glyph
- **AND** no position of group 0's span is drawn with `▒` or `⠌`, so the two alternations are
  in phase and the straddling group does not read as two groups
