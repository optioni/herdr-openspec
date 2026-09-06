# responsive-layout Specification

## Purpose
The dashboard's outer frame and how it reflows with terminal width: a one-row bold `OpenSpec`
header carrying the repository root right-aligned and shortened from the left, a bordered
body, and a footer of key hints dropped whole from the end rather than truncated, with
explicit branches for degenerate heights and one-column frames. It owns the 100-column
breakpoint that decides whether the body holds a 40-column `Changes` region beside a `Detail`
one or a single region showing only the routed side, which region's border is emphasised,
that content never bleeds across a border, and the two interior widths — 78 at a 120-column
frame and 58 at a 60-column one — that the detail-side capabilities render into. What fills
those interiors belongs to `change-rows`, `detail-header`, `artifact-tabs`, and
`artifact-content`.

## Requirements

### Requirement: The frame is a header row, a body, and a footer row

`ui::view::render(frame, &Dashboard)` SHALL be a pure function of its two arguments: it
SHALL perform no filesystem, process, environment, network, or terminal I/O, and SHALL
read no clock and no global state. It SHALL divide `frame.area()` vertically into exactly
three regions, in order — a header of `Constraint::Length(1)`, a body of
`Constraint::Min(0)`, and a footer of `Constraint::Length(1)` — and SHALL draw nothing
outside them.

The header SHALL render the literal `OpenSpec` at column 0 with
`Modifier::BOLD` set. The footer SHALL render the key hints `q quit`, `Enter detail`, and
`Esc back` in that order, separated by two spaces, starting at column 0, dropping hints
from the **end** when the remaining width cannot hold the next one whole.

`agent-launch` adds **two further hints, placed after `Esc back`**: when
`Dashboard::agents.reachable` is `true` the footer SHALL append `a/c/s launch` and then
`g focus`, joined by the same two-space separator. When it is `false` both SHALL be absent
entirely, so a pane with no reachable Herdr socket renders the footer this requirement
specified before `agent-launch` existed — which is the whole of `SPEC.md` → Degraded states'
"action keys hidden".

They are **one compound hint plus one**, not four separate ones, and that is a width decision
rather than a stylistic one: `q quit  Enter detail  Esc back  a apply  c continue  s archive`
is **62** columns, so at the mandated 60-column frame `fit_hints` would drop `s archive` and
`g focus` and offer the reader two of the four action keys with no indication that the other
two exist. `a/c/s launch  g focus` costs 23 columns including its separators, bringing the
footer to **53**, which fits the narrow frame whole. The keys themselves are documented in
`SPEC.md` → Keys and `README.md` → Keys; the footer's job is to say the feature is available
here and now, not to be the manual.

`agent-attribution` adds a **hint placed last**: when
`Dashboard::attribution().unattributed` is greater than zero, the footer SHALL append that
count, a single space, and the word `unattributed` — `1 unattributed`, `12 unattributed` —
after the action hints, joined by the same two-space separator. When the count is zero the hint
SHALL be absent entirely, so an agentless pane's footer is byte-identical to the footer this
requirement already specified. Being last means it is the **first** hint dropped as the width
falls, which is the correct priority: the key hints tell a reader how to drive the
pane, and the count tells them something they can act on later.

That priority has a measured consequence `agent-launch` makes explicit rather than leaving to be
discovered: the full reachable footer with a count is **69** columns, so at the mandated
60-column frame **the count is dropped and the action hints are kept**. Before this change the
count fitted at 60 (the footer was 46 columns); it still fits whenever the socket is
unreachable, because the two hints it now competes with are absent. The drop order is unchanged
— last hint first — and the hint list grew; nothing about the rule moved.

`SPEC.md` → Attributing an agent's tier 3 requires a count and forbids a row: an agent that
no tier attributed is reported here, in one shared cell, and never against a change. The
footer is the whole of that report — there is no per-agent listing, no expansion, and no key
that opens one. It is also never a `!`-marked problem row: an unattributed agent is a normal
state of a shared Herdr session, not a fault. A **failed launch** is not reported here at all:
`agent-launch` puts it on `Dashboard::launch.problems` and `change-rows` renders it as a
leading `!`-marked list row, because it is a fault and it is one the reader just caused.

The footer has two further forms, specified by `list-filtering` and restated here because
this requirement owns the row: while `dashboard.filter.active` is set the hints are
**replaced** by the prompt `/`, the query, and `_`, keeping its tail when it overflows —
and the action hints and the unattributed count are replaced along with them, because the
prompt replaces the whole row rather than the three key hints specifically; while the filter is
inactive with a non-empty query, `/` and the query become a further hint placed **first** in the
list above, dropped last rather than first, with the action hints and then the count after the
three key hints as usual.

Scenarios in this capability render a `Dashboard` whose `changes` is
`changes::empty_set()` and whose `agents.reachable` is `false` unless they say otherwise. The
first is no longer inert: with a repository root present, `change-rows` renders a
`No changes yet` message row into the list region's interior, and the scenarios below are
written so that none of them depends on that interior being blank. The second is stated
explicitly for the first time here: every landed footer assertion in this capability was written
against a dashboard whose socket was unreachable, and pinning that is what keeps those exact
strings true rather than accidentally so.

Degenerate heights SHALL be decided explicitly rather than delegated to the constraint
solver — measured, not assumed: `Layout::vertical([Length(1), Min(0), Length(1)])` at
height 1 gives the single row to the **footer**, so a naive split renders `q quit` where
`OpenSpec` belongs. The explicit branch is: at height 0 `render` SHALL draw nothing; at
height 1 it SHALL draw the header only; at height 2 it SHALL draw the header on row 0 and
the footer on row 1 and no body; at height 3 or more it SHALL use the three-way split
above. `render` SHALL NOT panic at any frame size of at least one column by one row, and
SHALL NOT panic at an interior of zero columns or zero rows, which a one- or two-column
frame produces.

#### Scenario: Header, body, and footer occupy their rows at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose
  `agents.reachable` is `false` is rendered into a `TestBackend` at 60x20, and again at 120x20
- **THEN** in both buffers row 0 column 0 through column 7 spells `OpenSpec`, and the cell
  at (0, 0) reports `Modifier::BOLD` set
- **AND** in both buffers row 19 begins with the exact string
  `q quit  Enter detail  Esc back` at column 0, and every remaining cell of row 19 is a
  space
- **AND** in both buffers row 1 is the top border of a bordered region and row 18 is its
  bottom border, so the body occupies rows 1 through 18 and nothing is drawn in row 0 or
  row 19 by the body

#### Scenario: The action hints follow `Esc back` when the socket is reachable

- **WHEN** the same `Dashboard` with `agents.reachable` set to `true` and no agents is
  rendered at 60x20 and at 120x20
- **THEN** row 19 is exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus` — **53** characters — followed by
  seven spaces at 60 and sixty-seven spaces at 120
- **AND** rendering the identical dashboard with `agents.reachable` `false` produces row 19 of
  exactly `q quit  Enter detail  Esc back` followed by thirty spaces at 60, byte-identical to
  the row this capability specified before the action hints existed
- **AND** nothing outside row 19 differs between the two renders at either width, so the
  reachability flag moves the footer and nothing else

#### Scenario: The action hints are dropped whole, `g focus` first

- **WHEN** the reachable, agentless dashboard is rendered at 53x20, 52x20, 44x20, and 43x20
- **THEN** the 53-column row is exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus`, filling the row with no trailing
  space
- **AND** the 52-column row is exactly `q quit  Enter detail  Esc back  a/c/s launch` followed
  by eight spaces — `g focus` and its two-space separator need nine columns and only eight
  remain, so it is dropped whole rather than cut to `g focu`
- **AND** the 44-column row is exactly `q quit  Enter detail  Esc back  a/c/s launch`, filling
  the row, and the 43-column row is exactly `q quit  Enter detail  Esc back` followed by
  thirteen spaces
- **AND** at 43 columns the three key hints are all still present, so both action hints are
  dropped before any of them

#### Scenario: A one-row frame renders the header and nothing else

- **WHEN** the same `Dashboard` is rendered at 60x1 and at 120x1
- **THEN** neither render panics
- **AND** row 0 spells `OpenSpec` at column 0 in both
- **AND** no box-drawing character and no `q quit` appears in either buffer, so neither the
  body nor the footer was drawn into the header's row

#### Scenario: A two-row frame renders the header and the footer with no body

- **WHEN** the same `Dashboard` is rendered at 60x2 and at 120x2
- **THEN** neither render panics
- **AND** row 0 spells `OpenSpec` and row 1 begins `q quit` at column 0
- **AND** no box-drawing character appears anywhere in either buffer, because the body
  received zero rows

#### Scenario: A one-column frame renders without panicking

- **WHEN** the same `Dashboard` is rendered at 1x1, at 1x20, at 2x20, and — as the
  contrasting controls at the two mandated widths — at 60x20 and 120x20
- **THEN** none of the five panics, including the two whose list region has an interior of
  zero columns
- **AND** the 1x20 buffer's row 0 is the single character `O`, the first character of the
  truncated `OpenSpec` label, so a one-column frame still draws rather than silently
  skipping the header
- **AND** the 1x20 buffer's row 19 is a single space: `q quit` needs six columns, so the
  first hint is dropped whole rather than truncated to `q`
- **AND** the same holds with `agents.reachable` `true`, which adds no hint that could fit in
  one column and therefore changes no cell of the 1x20 buffer
- **AND** the 60x20 and 120x20 buffers both spell `OpenSpec` in columns 0 through 7 and
  both begin row 19 with `q quit`, so the one-column result is a width branch rather than
  the header and footer being absent everywhere

#### Scenario: The footer drops whole hints rather than truncating one

- **WHEN** the same `Dashboard`, with `agents.reachable` `false`, is rendered at 18x20, at
  20x20, at 60x20, and at 120x20
- **THEN** the 18-column footer row is exactly `q quit` followed by twelve spaces —
  `q quit  Enter detail` needs exactly 20 columns, so `Enter detail` and every hint after
  it are dropped whole rather than cut short
- **AND** the 20-column footer row is exactly `q quit  Enter detail`, filling the row with
  no trailing space, which pins the boundary from the other side
- **AND** the 60-column footer row is exactly `q quit  Enter detail  Esc back` — thirty
  characters — followed by thirty spaces, so all three hints fit at the mandated narrow
  width
- **AND** the 120-column footer row is the same thirty characters followed by ninety
  spaces

#### Scenario: The unattributed count is the footer's last hint at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo`, whose `changes` holds
  one active change `alpha`, whose `agents.agents` holds one in-scope agent named
  `nothing-like-a-change`, and whose `agents.reachable` is `false`, is rendered at 60x20 and at
  120x20
- **THEN** the 60-column footer row is exactly
  `q quit  Enter detail  Esc back  1 unattributed` — forty-six characters — followed by
  fourteen spaces
- **AND** the 120-column footer row is the same forty-six characters followed by
  seventy-four spaces
- **AND** the same dashboard with `agents.reachable` set to `true` gives a 120-column footer row
  of exactly `q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` — **69**
  characters — followed by fifty-one spaces, and a 60-column row of exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus` followed by seven spaces: the count
  needs sixteen further columns and only seven remain, so it is dropped whole. That is the same
  last-hint-first rule, applied to a longer list
- **AND** rendering the identical dashboard with `agents.agents` empty and `reachable` `false`
  produces a footer row of exactly `q quit  Enter detail  Esc back` and thirty spaces at 60
  columns, byte-identical to the row this capability specified before the count existed
- **AND** the count is a number of agents, not of changes: adding a second in-scope agent
  named `also-nothing` makes the hint read `2 unattributed` at 120 columns under either value
  of `reachable`, while the list region's rows are unchanged

#### Scenario: The count is reported with an empty change list

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo`, whose `changes` is
  `changes::empty_set()`, whose `agents.agents` holds two in-scope agents named
  `nothing-like-a-change` and `also-nothing`, and whose `agents.reachable` is `false`, is
  rendered at 60x20 and at 120x20
- **THEN** the footer row reads `q quit  Enter detail  Esc back  2 unattributed` at both
  widths
- **AND** the list region's interior holds exactly the single `No changes yet` message row
  `change-rows` specifies, byte-identical to the agentless rendering of the same dashboard —
  a `Message` row is never badged
- **AND** the same holds with a `/` query matching nothing: the two message rows are
  byte-identical and the count is unchanged, because the count is over agents and the filter
  is over changes
- **AND** with `agents.reachable` set to `true` the list region's interior is unchanged, cell
  for cell, at both widths: the action hints live in the footer and never in the list

#### Scenario: The count is dropped whole before the three key hints

- **WHEN** the one-unattributed-agent dashboard with `agents.reachable` `false` is rendered at
  46x20 and at 45x20
- **THEN** the 46-column footer row is exactly
  `q quit  Enter detail  Esc back  1 unattributed`, filling the row with no trailing space
- **AND** the 45-column footer row is exactly `q quit  Enter detail  Esc back` followed by
  fifteen spaces — the count and its two-space separator need sixteen columns and only
  fifteen remain, so it is dropped whole rather than cut to `1 unattribute`
- **AND** at 45 columns the three key hints are all still present, so the count is dropped
  before any of them
- **AND** the same dashboard with `agents.reachable` `true` pins the boundary one hint list
  further out: the 69-column row is exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` with no trailing
  space, and the 68-column row is exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus` followed by fifteen spaces — the
  count dropped whole, both action hints kept

#### Scenario: The filter prompt replaces the count along with the hints

- **WHEN** the one-unattributed-agent dashboard with `agents.reachable` `false` is rendered at
  60x20 and at 120x20 with `filter.active` set and `filter.query` `be`, and then again with
  `filter.active` cleared and the same query kept
- **THEN** the active-filter footer row is exactly `/be_` followed by spaces at both widths,
  and the string `unattributed` appears nowhere in it
- **AND** the accepted-query footer row is exactly
  `/be  q quit  Enter detail  Esc back  1 unattributed` at both widths, so the query leads
  the list and the count still trails it
- **AND** both forms are byte-identical to what `list-filtering` specifies once the same
  dashboard's `agents.agents` is emptied, so the count is additive rather than a rewrite of
  either form
- **AND** with `agents.reachable` `true` the active-filter row is still exactly `/be_` followed
  by spaces at both widths, and the strings `a/c/s launch` and `g focus` appear nowhere in it:
  the prompt replaces the **whole** row, action hints included
- **AND** with `agents.reachable` `true` the accepted-query row is exactly
  `/be  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` — 74 characters
  — at 120, and exactly `/be  q quit  Enter detail  Esc back  a/c/s launch  g focus` — 58
  characters — followed by two spaces at 60, so the query still leads and the count is still
  the first thing dropped

### Requirement: The 100-column breakpoint decides one region or two

`ui::layout::WIDE_MIN_WIDTH` SHALL be `100`. `ui::layout::mode(width: u16)` SHALL return
`LayoutMode::Wide` when `width >= WIDE_MIN_WIDTH` and `LayoutMode::Narrow` otherwise, and
SHALL be a total function over every `u16`.

At `LayoutMode::Wide` the body SHALL be split horizontally into exactly two regions —
`Constraint::Length(40)` for the change list on the left and `Constraint::Min(0)` for the
artifact detail on the right — so that every column gained beyond 100 goes to the detail
side. Both regions SHALL be drawn, each as a block with all four borders and a title:
`Changes` on the left, `Detail` on the right.

At `LayoutMode::Narrow` the body SHALL hold exactly one region occupying the whole body
width, drawn as a block with all four borders, titled `Changes` when the dashboard's route
is `Route::List` and `Detail` when it is `Route::Detail`. The region that is not routed to
SHALL NOT be drawn at all.

The mode SHALL be derived from the frame area passed to `render` on every draw, and SHALL
NOT be stored on `Dashboard` or captured at startup, so a terminal resized across the
breakpoint changes layout on its next frame with no extra state.

#### Scenario: At 120 columns both regions are drawn with the divider at column 40

- **WHEN** a `Dashboard` with `route: Route::List` is rendered at 120x20, and — as the
  contrasting control at the mandated narrow width — at 60x20
- **THEN** in the 120-column buffer row 1 holds `┌` at column 0, `┐` at column 39, `┌` at
  column 40, and `┐` at column 119
- **AND** row 1 columns 1 through 7 spell `Changes`, and row 1 columns 41 through 46 spell
  `Detail`
- **AND** row 18 holds `└` at column 0, `┘` at column 39, `└` at column 40, and `┘` at
  column 119
- **AND** the 60-column buffer holds exactly one `┌` in the whole buffer, so the second
  region is a width branch rather than something drawn unconditionally

#### Scenario: At 60 columns only the routed region is drawn

- **WHEN** the same `Dashboard` with `route: Route::List` is rendered at 60x20, and — as
  the contrasting control at the mandated wide width — at 120x20
- **THEN** in the 60-column buffer row 1 holds `┌` at column 0 and `┐` at column 59, and no
  other `┌` appears anywhere in the buffer
- **AND** row 1 columns 1 through 7 spell `Changes`
- **AND** the string `Detail` appears in no row of the 60-column buffer
- **AND** the 120-column buffer does contain `Detail`, at row 1 columns 41 through 46, so
  the absence at 60 columns is the breakpoint and not the title being missing

#### Scenario: At 60 columns the detail route replaces the list region

- **WHEN** a `Dashboard` with `route: Route::Detail` is rendered at 60x20
- **THEN** row 1 columns 1 through 6 spell `Detail`
- **AND** the string `Changes` appears in no row of the buffer
- **AND** rendering the same dashboard at 120x20 still shows **both** `Changes` at row 1
  column 1 and `Detail` at row 1 column 41, because the route selects emphasis rather than
  visibility above the breakpoint

#### Scenario: The breakpoint is exact at 99, 100, and 101 columns

- **WHEN** `ui::layout::mode` is called with `0`, `1`, `40`, `60`, `99`, `100`, `101`,
  `120`, and `u16::MAX`
- **THEN** it returns `Narrow` for `0`, `1`, `40`, `60`, and `99`, and `Wide` for `100`,
  `101`, `120`, and `u16::MAX`
- **AND** rendering the same `Dashboard` produces exactly one `┌` in the buffer at 60x20
  and at 99x20, and exactly two — the second at column 40 — at 100x20, at 101x20, and at
  120x20, so both mandated widths and all three boundary widths are rendered, not just
  computed

#### Scenario: The mode follows the current frame, not the startup size

- **WHEN** one `Terminal<TestBackend>` is created at 120x20, a frame is drawn, the backend
  is resized to 60x20, and a second frame is drawn from the **same** unchanged `Dashboard`
- **THEN** the first buffer holds two `┌` characters and the second holds one
- **AND** `Dashboard` exposes no field naming a width, a layout mode, or a column count

### Requirement: The routed region is emphasised and region interiors are left empty

The region the dashboard's route names SHALL have its border drawn with `Modifier::BOLD`
set; the other region, when drawn, SHALL have its border drawn without it.

Neither region's interior is left blank unconditionally any longer. The **list** region's
interior is owned by `change-rows`, with `list-selection` owning which slice is drawn. The
**detail** region's interior is owned by `detail-header` (its first row), `artifact-tabs`
(its second row), and `artifact-content` and `detail-scroll` (the content area below them),
divided by `layout::split_detail`.

The detail interior is blank on a frame **exactly when `Dashboard::visible()` is empty** — no
repository, no changes, or a `/` filter matching none. That is the whole of the blank case
from `detail-view` onward: with a change selected, the region always carries at least a
header, a tab bar, and one content line, because `ui::detail::content_lines` returns
`No content yet` rather than nothing. The earlier statement that the production pane's detail
interior is blank on every frame, because nothing set `detail.source`, is what this change
retires: `detail-view` is the change that was named there as the one that would.

When the interior **is** blank, every cell of it is a space whose `Style` equals
`ratatui::buffer::Cell::default().style()`. The comparison is against `Cell::default().style()`
and **not** against `Style::default()`: `ratatui-crossterm` re-enables the `underline-color`
feature through its own defaults, so an untouched cell's style is
`fg(Reset).bg(Reset).underline_color(Reset)`, which equals neither `Style::default()` nor
`Style::reset()`. Comparing against the constructible value is what catches a `Block::style`
being set where `Block::border_style` was meant.

Content SHALL NOT bleed across a border: no cell of a region's border column or border row
SHALL be overwritten by a list row, a detail header, a tab cell, a problem line, or a
markdown line, at either mandated width.

#### Scenario: The routed region's border is bold and the other's is not

- **WHEN** a `Dashboard` with `route: Route::List` is rendered at 120x20
- **THEN** the cell at column 0, row 1 reports `Modifier::BOLD` set
- **AND** the cell at column 40, row 1 reports `Modifier::BOLD` **not** set
- **AND** with `route: Route::Detail` and the same size, the two assertions swap, so the
  test discriminates rather than asserting a constant
- **AND** at 60x20 the single drawn region's border cell at column 0, row 1 reports
  `Modifier::BOLD` set under **both** routes, because the region that is drawn is always
  the routed one below the breakpoint

#### Scenario: Interiors are blank at both widths

The scenario's name is kept verbatim from `tui-shell` because a delta's scenario headers are
its merge key; the detail interior is blank now because no change is selected, not because
nothing may write there.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  `changes::empty_set()`, and an empty `detail` is rendered at 60x20 and at 120x20
- **THEN** in the 120-column buffer every cell in rows 2 through 17 and columns 41 through
  118 is a space whose `Style` equals `Cell::default().style()` — the detail region is
  untouched because `visible()` is empty
- **AND** in the 120-column buffer row 2, columns 1 through 38, begins `No changes yet`, so
  the list interior is written by `change-rows` rather than left blank
- **AND** in the 60-column buffer row 2, columns 1 through 58, begins `No changes yet`, and
  rows 3 through 17 of columns 1 through 58 are entirely spaces whose `Style` equals
  `Cell::default().style()`, so exactly one message row was drawn
- **AND** the same dashboard with **one** active change added is no longer blank in the
  detail region at 120x20: row 2 columns 41 onward holds that change's header, so the
  blankness asserted above is a property of the empty visible list rather than a constant

#### Scenario: Rows do not overwrite the borders at either width

- **WHEN** a `Dashboard` holding thirty active changes with names long enough to be
  truncated, whose selected change carries twelve artifacts with 40-character ids, and whose
  `detail.source` is thirty lines each 200 characters long, is rendered at 60x20 and at
  120x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 1 through 18
  is a box-drawing character
- **AND** in the 120-column buffer every cell of columns 0, 39, 40, and 119 in rows 1
  through 18 is a box-drawing character, so a 38-column row neither ran into the divider nor
  into the detail region, and neither a 78-column header, a 78-column tab bar, nor a
  78-column markdown line ran into the divider or past the frame
- **AND** the same holds at `Route::Detail` at 60x20, where the detail region is the only
  one drawn
### Requirement: The header names the repository root, shortened from the left when narrow

The header row SHALL render, right-aligned so that its last character sits in the final
column, the repository root's display path when one was found, and the literal
`no repository` when none was. Exactly one blank column SHALL separate the `OpenSpec`
label from the shortened text at minimum.

Let `A` be the header width minus 9 — the eight columns of `OpenSpec` plus one separating
blank — **floored at zero**, so widths below 9 do not underflow the unsigned subtraction.
When the text's character count is at most `A` it SHALL be rendered whole. When it is
longer and `A` is at least 8, it SHALL be rendered as `…` followed by its last `A - 1`
characters. When `A` is below 8 the text SHALL be omitted entirely and only `OpenSpec`
SHALL be drawn; when the width is below 8 the label itself SHALL be truncated to the
columns available. Shortening SHALL count characters, not bytes, and SHALL keep the tail —
the repository's own directory name is what identifies it, and the leading path components
are what a reader can spare.

The shortening rule SHALL be one shared implementation with `change-rows`' no-repository
block, which shortens `searched_from` by the same keep-the-tail rule against the list
region's interior width rather than the header's.

#### Scenario: A path that fits is right-aligned whole at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` — fourteen characters —
  is rendered at 60x20 and at 120x20
- **THEN** the 60-column header row spells `/tmp/demo-repo` in columns 46 through 59, and
  columns 8 through 45 are spaces
- **AND** the 120-column header row spells `/tmp/demo-repo` in columns 106 through 119

#### Scenario: A path too long for the narrow header is shortened from the left

- **WHEN** a `Dashboard` whose repository root is
  `/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here` — seventy
  characters — is rendered at 60x20 and at 120x20
- **THEN** the 60-column header row spells
  `…/openspec-demos/a-rather-long-repository-name-here` in columns 9 through 59, so the
  first character of the shortened text is the ellipsis and the last is the final `e` of
  the directory name
- **AND** the 120-column header row spells the whole seventy-character path in columns 50
  through 119, with no ellipsis anywhere in the buffer — which stays true only because
  that `Dashboard`'s `changes` is `changes::empty_set()` with a repository root present,
  so the list interior holds the fourteen-character `No changes yet` and needs no
  ellipsis of its own

#### Scenario: A header too narrow for any path shows only the label

- **WHEN** the same seventy-character `Dashboard` is rendered at 16x20, at 60x20, and at
  120x20
- **THEN** the 16-column header row is exactly `OpenSpec` followed by eight spaces, and no
  ellipsis appears in that row
- **AND** the 60-column header row carries the shortened, ellipsis-prefixed path in columns
  9 through 59, and the 120-column header row carries the full path in columns 50 through
  119 — so the 16-column omission is a width branch and not the feature being absent
- **AND** no ellipsis appears anywhere in the 16-column buffer: its list interior is
  fourteen columns wide and `No changes yet` is exactly fourteen characters, so the body
  neither truncates nor overflows

#### Scenario: No repository found is named in the header at both widths

- **WHEN** a `Dashboard` built with no repository root — the value `ui::load` produces when
  `resolve::find_repo` reports `NotFound`, with `searched_from` `/tmp/searched-from` — is
  rendered at 60x20 and at 120x20
- **THEN** the 60-column header row spells `no repository` in columns 47 through 59
- **AND** the 120-column header row spells `no repository` in columns 107 through 119
- **AND** neither **header row** names the directory the search started from: row 0 of
  each buffer does not contain `/tmp/searched-from`. The **body** now does, and that is
  `change-rows`' no-repository block — the landed form of this scenario asserted the
  string was absent from the whole buffer, which `list-view` makes false; the assertion
  is narrowed to row 0, which is what the requirement was ever about
### Requirement: The detail region's two mandated interior widths are 78 and 58

The detail region's interior width SHALL be **78** at a 120-column frame — the wide layout's
`Constraint::Min(0)` column, 80 columns, less two border columns — and **58** at a 60-column
frame in the detail route, where the region is the whole 60-column body less two border
columns. Both interiors SHALL be **16 rows** at a 20-row frame.

These two widths are frozen here for `detail-view` and `tasks-tab` to inherit, exactly as
`change-rows`' 38 and 58 were frozen by `list-view`. Every test of `ui::markdown` SHALL name
both, and a source check SHALL enforce that with a floor on the number of tests found, on
the same terms and with the same stated limits as the check over `src/ui/list.rs`.

Because the wide layout's detail column is `Min(0)`, 78 is the width at the mandated frame
size and not a constant of the layout: every column gained beyond 120 goes to the detail
region. Nothing SHALL depend on 78 other than the expectations of tests rendered at 120.

#### Scenario: The detail interior is 78 columns at 120 and 58 at 60

- **WHEN** `layout::split_frame` and `layout::split_body` are applied to `Rect::new(0, 0,
  120, 20)` with `Route::Detail`, and the resulting detail rectangle is passed to
  `layout::interior`
- **THEN** the interior is `Rect::new(41, 2, 78, 16)`
- **AND** the same applied to `Rect::new(0, 0, 60, 20)` with `Route::Detail` gives
  `Rect::new(1, 2, 58, 16)`
- **AND** at `Rect::new(0, 0, 60, 20)` with `Route::List` there is no detail rectangle at
  all, so the narrow list route has no detail interior to be 58 columns wide

#### Scenario: Every markdown test names both of its two interior widths

- **WHEN** every `#[test]` in `src/ui/markdown.rs` is inspected for the bare literals `78`
  and `58`, with a floor on the number of tests found
- **THEN** every one of them names both, and the floor is met, so a gutted file fails rather
  than passing with nothing to check
- **AND** the check is a floor and not a proof — a `78` in a comment satisfies it — and is
  paired with a counted filtered run, because `cargo test` exits 0 when a filter matches
  nothing
- **AND** the number scan does not see a suffixed literal such as `78u16`, so it fails
  closed and the widths are written unsuffixed
