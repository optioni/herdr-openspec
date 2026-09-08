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

The header row SHALL render, right-aligned so that its last column sits in the final
column, the repository root's display path when one was found, and the literal
`no repository` when none was. Exactly one blank column SHALL separate the `OpenSpec`
label from the shortened text at minimum.

**The `file mode` badge.** When `Dashboard::file_mode` is true — the `openspec` binary probe
resolved no usable binary, so the change list is file-sourced for the whole session — the
header SHALL draw the literal `file mode`, nine columns, immediately after the `OpenSpec`
label and one separating blank, in columns 9 through 17, styled with ratatui's `DIM`
modifier and no other. The badge is dim because it names a *mode*, not a fault: file mode is
a supported way to run, and a badge competing with the repository path for attention would
say otherwise.

The badge SHALL be dropped **whole**, never cut short, when the header width is below 18 —
the eight columns of `OpenSpec`, one blank, and the badge's nine — on exactly `change-rows`'
drop-whole rule. It is dropped **before** the path is shortened, not after: at a width that
cannot hold both, the reader can still learn the repository from the pane's contents, and a
half-drawn `file mo` would name nothing at all.

When `file_mode` is false the header SHALL be byte-identical to the header this requirement
already specified — no badge, no reserved columns, and the same `A`.

Let `A` be the header width minus 9 — the eight columns of `OpenSpec` plus one separating
blank — **floored at zero**, so widths below 9 do not underflow the unsigned subtraction.
When the badge is drawn, `A` SHALL instead be the header width minus **19** — the same nine,
plus the badge's nine and one further separating blank — floored at zero by the same rule.
When the text's `layout::columns` is at most `A` it SHALL be rendered whole. When it is
longer and `A` is at least 8, it SHALL be rendered as `…` followed by the **last `A - 1`
columns** of the text — the longest suffix ending on a grapheme-cluster boundary that
measures at most `A - 1`. When `A` is below 8 the text SHALL be omitted entirely and only
`OpenSpec` SHALL be drawn; when the width is below 8 the label itself SHALL be truncated to
the columns available. Shortening SHALL count **display columns**, not characters and not
bytes, and SHALL keep the tail — the repository's own directory name is what identifies it,
and the leading path components are what a reader can spare.

Because a cluster is dropped whole, the shortened text MAY measure one column less than the
space allotted to it. The row SHALL still be right-aligned against its own measured width,
so the last drawn column is the final column and any slack falls to the **left** of the
ellipsis, where the `OpenSpec` label's trailing blanks already are. The header SHALL never
draw past its last column at any width for any path.

The badge SHALL be drawn on the **frame** header, not on the detail region's change header.
The two are different claims: a missing binary is a fact about the process, true of every
change in the pane, while the detail header describes one change — and `SPEC.md` → Degraded
states already gives the per-change equivalent its own row, "Schema unknown to the CLI",
whose fall-back to file mode is per change and is named in the detail region instead.

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

#### Scenario: The badge is drawn dim after the label at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose `file_mode` is
  **true** is rendered at 60x20 and at 120x20
- **THEN** the header row's columns 9 through 17 spell `file mode` at both widths, and column
  8 is a space
- **AND** every one of those nine cells carries ratatui's `DIM` modifier, and the `OpenSpec`
  label's eight cells do not, so the badge is distinguishable from the label by style as well
  as by position
- **AND** the 60-column header spells `/tmp/demo-repo` in columns 46 through 59 and the
  120-column header in columns 106 through 119 — unchanged, because a fourteen-character path
  fits inside `A` at both widths either way

#### Scenario: A false flag renders the header that landed before this change

- **WHEN** the same `Dashboard` is rendered with `file_mode` **false** at 60x20 and at 120x20
- **THEN** neither buffer contains the substring `file mode` anywhere, in any row
- **AND** both buffers are byte-identical, cell for cell and style for style, to the ones the
  same dashboard produced before this change existed

#### Scenario: The badge takes its columns from the path, not from the label

- **WHEN** a `Dashboard` whose repository root is
  `/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here` — seventy
  characters — and whose `file_mode` is true is rendered at 60x20
- **THEN** the header row's columns 9 through 17 spell `file mode`
- **AND** the shortened path occupies columns 19 through 59 — `A` is 41 rather than 51 — and
  begins with the ellipsis, so ten more leading characters were spared than without the badge
- **AND** column 18 is a space, so the badge and the path never abut
- **AND** the same dashboard at 120x20 spells the whole seventy-character path with no
  ellipsis, the badge still in columns 9 through 17

#### Scenario: A header too narrow for the badge drops it whole

- **WHEN** the same seventy-character, file-mode `Dashboard` is rendered at 17x20, at 18x20,
  and at 60x20
- **THEN** the 17-column header row does not contain `file mode`, nor any prefix of it: it is
  exactly `OpenSpec` followed by nine spaces
- **AND** the 18-column header row spells `OpenSpec`, a space, then `file mode` in columns 9
  through 17, so 18 is the exact width at which the badge appears and 17 the one at which it
  does not
- **AND** the 60-column header carries both the badge and the ellipsis-prefixed path, so the
  17-column omission is a width branch and not the feature being absent
- **AND** no ellipsis appears anywhere in the 17-column buffer's header row

#### Scenario: A wide-character path is shortened by columns and stays inside the header

- **WHEN** a `Dashboard` whose repository root is
  `/home/dev/workspaces/日本語のリポジトリ名前がとても長いディレクトリ` — forty-four
  characters and **sixty-seven display columns** — is rendered at 60x20 and at 120x20, and
  again with `file_mode` true. The fixture is chosen to exceed `A` at the narrow width both
  with the badge (41) and without it (51), so both branches actually shorten; a
  wide-character path short enough to fit would leave every assertion below unreachable
- **THEN** in every one of the four buffers the header row's last drawn column is the frame's
  final column and no cell beyond it is written
- **AND** in the 60-column, non-badged buffer the shortened text begins with `…` at a column
  no earlier than 9 and ends in the final column, and its `columns` is at most `A` — 51
- **AND** a `char`-counted shortening of the same path would have kept its last 50
  **characters**, which measure far more than 51 columns and would have run past the frame —
  so the scenario distinguishes the two measures rather than merely exercising one
- **AND** in the 60-column, badged buffer the badge occupies columns 9 through 17, column 18
  is blank, and the shortened path's `columns` is at most `A` — 41 — so the badge took its
  columns from the path exactly as the unbadged rule says
- **AND** in the 120-column buffer the whole sixty-seven-column path is drawn, its first
  column no earlier than column 50, and no ellipsis appears in that row
- **AND** rendering the same dashboard at 16x20, 18x20, 19x20, and 1x20 draws only the
  label or a truncation of it, writes nothing past the last column, and does not panic

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

### Requirement: Display width is measured in terminal columns by one pair of primitives

Every measurement and every truncation under `src/ui/` SHALL be expressed in **terminal
display columns**, never in `char`s and never in bytes. Two pure total functions in
`ui::layout` SHALL be the crate's only implementation of that measure:

```rust
pub(crate) fn columns(text: &str) -> usize;
pub(crate) fn truncate_columns(text: &str, max: usize) -> &str;
```

`columns` SHALL return the number of terminal cells `ratatui::buffer::Buffer::set_string`
consumes for `text`, computed the way `set_string` itself computes it and not by an
independent table: it SHALL split `text` into grapheme clusters and sum each cluster's
width through ratatui's own public API — `ratatui::text::Span::styled_graphemes`, which
performs the split and drops clusters containing a control character, and the
`ratatui::buffer::CellWidth` trait, which yields each remaining cluster's cell width. This
is the whole of the argument for the choice: agreement with the buffer is the property being
bought, and any second measure — a hand-rolled table, or `unicode-width` called directly —
would agree with `set_string` only by coincidence of version and would miss ratatui's own
halfwidth-katakana adjustment and its control-character filter.

`truncate_columns(text, max)` SHALL return the longest **prefix of `text` ending on a
grapheme-cluster boundary** whose `columns` is at most `max`. It SHALL never split a
cluster, SHALL return `""` for `max == 0`, and SHALL return `text` whole when
`columns(text) <= max`. Because a cluster is dropped whole, the returned prefix MAY measure
`max - 1` columns where a two-column cluster would not fit; a caller that owes an exact
width SHALL pad the difference rather than assume the prefix filled it.

Both functions SHALL be pure and total: no filesystem, process, environment, network, or
standard-I/O work, no clock, no global state, and no panic for any `&str` and any `usize`.
Both live in `ui::layout` — the module `responsive-layout` already owns, and which already
names a `ratatui` type — so the crate's pure view set stays at the **eight** files
`dashboard-loop` enumerates and no ninth file is added to it. This placement is a
consequence of that count, not a claim that text measurement is `Rect` geometry.

**The Unicode promise the pane makes, stated rather than left to be discovered, and stated
with its limit.** A grapheme cluster occupies the columns ratatui gives it, and no line the
view produces ever exceeds — **as ratatui measures it** — the region it is drawn into. That
qualification is the promise's boundary and is deliberate: the pane's arithmetic and
`Buffer::set_string` are the same measure by construction, so within the buffer the bound is
exact, but a terminal is free to paint a cluster in a different number of cells than ratatui
budgeted and the pane has no way to know.

The known divergence runs in one direction, and it is the opposite of the intuitive one. A
zero-width-joiner emoji sequence is **one** grapheme cluster to `graphemes(true)`, and
ratatui measures it at 2 columns — the same as a terminal that composes it. A terminal that
does **not** compose it paints each constituent emoji instead, six columns for a
three-person family, and the row overruns. The pane SHALL NOT attempt to detect or
compensate for this: it is a property of the terminal's font and shaping, invisible to a
process writing bytes to a pty, and any compensation would have to guess. It is recorded
here as an accepted limit of the promise rather than a defect, and it is the one case in
which a line can exceed its region after this change.

Correspondingly, the pane SHALL NOT reorder bidirectional text, SHALL NOT probe the terminal
for its capabilities, and SHALL NOT tailor any measurement to a locale — which carries its
own instance of the same limit, since several of the box-drawing and arrow characters the
artifacts already use are East Asian **Ambiguous** and a CJK-locale terminal renders them at
2 columns where `unicode-width`'s default, and therefore ratatui's, says 1.

No file under `src/ui/` other than `src/ui/layout.rs` SHALL measure a rendered string with a
`char` count. Concretely, no **production** line of `src/ui/app.rs`, `src/ui/detail.rs`,
`src/ui/list.rs`, `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, or
`src/ui/driver.rs` SHALL name `.chars().count()`, `.chars().take(`, or a
`Vec<char>`-producing `.chars().collect()`. Those seven are the whole of the rule's reach,
not a sample of it: `src/ui/mod.rs` and `src/ui/terminal.rs` are the only other files under
`src/ui/`, neither renders, and neither holds such a measurement in production code today.
Iterating characters for a purpose that is not measurement — `ui::markdown`'s per-character
scanner, for one — is untouched by this rule.

**Two measuring sites the pattern above cannot see, named so they are not missed.**
`ui::markdown`'s `split_at_char`, which cuts a run at a `char_indices` offset for the
hard-split, and any future `char_indices`-based cut, are column budgets expressed in
characters that no `.chars()` pattern matches. They SHALL be rewritten to cut in columns
along with the rest; what proves it is `markdown-render`'s wide-character and 500-column-CJK
scenarios, not the sweep. A source check that cannot see a violation is recorded as a limit
rather than relied on.

**The footer is inside this rule.** `responsive-layout`'s own footer requirement and
`list-filtering`'s both state their hint budgets and their keep-the-tail truncation of the
filter prompt in characters, against all-ASCII hint literals whose stated counts stay exactly
true. The **query** in that footer is the reader's own typed text and is not ASCII-bound, so
the footer's budget arithmetic and its tail truncation SHALL be measured with `layout::columns`
and cut with `layout::truncate_columns` like every other field. No hint literal's length
changes and no landed footer assertion moves.

#### Scenario: `columns` agrees with what the buffer consumed

- **WHEN** for each of `abc`, `日本語`, `🎉`, `e` followed by U+0301 COMBINING ACUTE ACCENT,
  a family emoji joined by two zero-width joiners, `ｶ` followed by U+FF9E HALFWIDTH KATAKANA
  VOICED SOUND MARK, a string holding only U+0007 BEL, and the empty string, the string is
  written into a fresh 40x1 `Buffer` with
  `Buffer::set_stringn(0, 0, s, usize::MAX, Style::default())` and the `x` of the `(u16, u16)`
  that call **returns** is taken — the cursor position `set_stringn` advanced to, which is by
  definition the number of cells it consumed
- **THEN** that `x` equals `layout::columns` of the same string for every one of the eight
- **AND** the oracle is `set_stringn`'s **return value** and SHALL NOT be "the index of the
  first blank cell": `set_stringn` calls `Cell::reset()` on the trailing cells of every
  multi-column cluster, and a reset cell is byte-identical to an untouched one, so a
  first-blank scan reports `1` for `日本語`, for `🎉`, and for the family emoji, and a
  measurement built to satisfy it would be wrong in exactly the direction this change exists
  to fix. `set_string` itself returns `()` and cannot serve as the oracle
- **AND** `columns("")` is `0` and `columns` of the BEL string is `0`, because
  `styled_graphemes` drops control clusters exactly as `set_stringn` does

#### Scenario: `truncate_columns` never splits a cluster and never overruns

- **WHEN** `truncate_columns` is called with `日本語の変更` at every `max` from `0` through
  `14`, with `abc🎉def` at every `max` from `0` through `10`, and with
  `ab` + U+0007 BEL + `日本語` at every `max` from `0` through `10`
- **THEN** at every `max` the result's `columns` is at most `max`, the result is a **prefix of
  the input as bytes**, and re-slicing the input at the result's own length succeeds — so no
  call ever cut a cluster and none panicked
- **AND** at `max` `3` for `日本語の変更` the result is `日` and measures `2`, one short of
  `max`, because the second cluster would have overrun
- **AND** at `max` `0` every result is `""`, and at a `max` at or above the input's own
  `columns` every result is the whole input
- **AND** the BEL case does not panic at any `max`, which is what pins the byte-offset rule
  below: `styled_graphemes` **drops** the control cluster, so an implementation that derives
  its cut point by summing the yielded symbols' `len()` computes an offset shifted by the
  dropped byte and slices mid-character. `truncate_columns` SHALL derive its cut point from
  each symbol's own position within the original `&str` — its byte offset, not a running sum
  of returned lengths — so a dropped cluster shifts nothing

#### Scenario: Nothing under `src/ui/` measures in characters except the primitives

- **WHEN** the production lines of `src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/list.rs`,
  `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` — every
  line up to each file's `#[cfg(test)]` module — are searched for `.chars().count()`,
  `.chars().take(`, and a `Vec<char>` `.chars().collect()`
- **THEN** there is no match in any of the seven
- **AND** the check first runs **its own sweep pattern** against a line it synthesises
  holding each of the three forms, and fails when that self-test does not match all three.
  This is the positive control, and it is the sweep's pattern rather than a second one: a
  control that greps `layout.rs` for `cell_width` proves only that `cell_width` is spelled
  right, and would let a corrupted sweep regex print `COLWIDTH OK` over a tree full of
  violations. `NOSPAWN-GREP`'s shape — run the same pattern against something that must
  match — is the model
- **AND** the check additionally requires `src/ui/layout.rs` to name `cell_width` and
  `styled_graphemes`, so the measure it is protecting is present and the exemption is not
  vacuous
- **AND** the check is a repository file under `scripts/gates/` named in the `Makefile`'s
  `gates:` recipe, so `tests/ci_workflow.rs`'s recipe-versus-directory assertion covers it
  and it cannot silently drop out of `make gates`
