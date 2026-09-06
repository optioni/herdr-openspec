## MODIFIED Requirements

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
