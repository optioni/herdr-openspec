## ADDED Requirements

### Requirement: The help overlay is a full-body-width band the layout computes

`ui::layout` SHALL gain one function, and it SHALL be the only place the overlay's
geometry is decided:

```rust
pub fn help_band(body: Rect, content_rows: usize) -> Rect;
```

It SHALL be pure and total: no I/O, no clock, no panic for any `Rect` including a
zero-sized one and one at `u16::MAX`, and no arithmetic that can overflow or underflow —
every subtraction saturating, on exactly the terms `interior` and `scroll_offset` already
hold to.

It SHALL return a rectangle whose `x` and `width` are the **body's own**, whose `height`
is `min(content_rows + 2, body.height)`, and whose `y` is
`body.y + (body.height - height) / 2`. The band therefore runs the full width of the body,
is vertically centred in it, and gives any odd remaining row to the space below it.

The overlay SHALL NOT participate in the 100-column breakpoint. `split_body` decides one
region or two and the overlay covers whichever it produced; there is no wide form and no
narrow form of the band, and `LayoutMode` is not consulted. That is a deliberate
simplification with a stated cost: at 120 columns the band's descriptions sit in a single
column with the right half of the row empty, where a two-column reflow would have halved
its height. Single-column is chosen because the band is scrollable — `help-overlay`
requires it — so height is not the constraint the reflow would have relieved, and a
breakpoint-dependent overlay would need its own geometry tests at both widths for a
layout no other requirement in this capability asks for.

The band SHALL cover the **body only**. `ui::view::render` splits the frame into a body
and a footer row, and the overlay is drawn into the body after the regions are; the footer
row SHALL render its hints unchanged while the overlay is open, so `? help` and `q quit`
are both visible from inside it.

`ui::layout::columns` and `ui::layout::truncate_columns` SHALL remain the crate's only
display-width measure, and `src/ui/help.rs` SHALL use them for the key column and for
every truncation. `COLWIDTH`'s `PURE` list SHALL gain `src/ui/help.rs`, taking it from
eight files to **nine**, and `NOIO-VIEW`'s from nine to **ten**.

`ui::help`'s own both-widths rule SHALL be **mechanized**, not merely mandated. Every other
view module in the crate has a width gate of its own — `detailwidths.sh`, `listwidths.sh`,
`mdwidths.sh`, `taskwidths.sh`, and `widths.sh` (hard-coded to `src/ui/view.rs`) at
`Makefile:39,40,42,64,66` — and without one, `ui::help` would be the only module carrying a
"60 and 120, both widths, every time" mandate with nothing counting whether it is kept. A
`scripts/gates/helpwidths.sh` SHALL be added on `detailwidths.sh`'s pattern, composed into the
`gates:` recipe, its floor measured when the module's tests are written rather than guessed,
and bound to its own planted defect in `tests/gate-controls.toml` like every other gate.

#### Scenario: The overlay's both-widths rule is counted, not just stated

- **WHEN** `scripts/gates/helpwidths.sh` is run against the tree at the end of this change
- **THEN** it exits zero and reports the number of `src/ui/help.rs` tests asserting at both 60
  and 120 columns, against a floor measured from that tree
- **AND** it exits non-zero against a copy in which a `ui::help` test asserting only at 120 is
  added, and against one in which the module's tests are removed — the second being the vacuity
  leg every other width gate carries
- **AND** `tests/gate-controls.toml` binds it to a planted defect, so a `helpwidths.sh` neutered
  to `exit 0` fails `cargo test` rather than passing `make gates` quietly

#### Scenario: The band's rectangle at both mandated widths

- **WHEN** `help_band` is called with the body a 120x40 frame produces — `x` 0, `y` 0,
  `width` 120, `height` 39 — and `content_rows` of 42
- **THEN** it returns `x` 0, `width` 120, `height` 39, and `y` 0: the content needs 44
  rows and the body holds 39, so the band fills it
- **AND** with the body a 60x20 frame produces — `height` 19 — and the same
  `content_rows`, it returns `x` 0, `width` 60, `height` 19, and `y` 0
- **AND** with a 120x60 frame's body — `height` 59 — and the same `content_rows`, it
  returns `height` 44 and `y` 7, so the band is centred with the odd row below it

#### Scenario: The band is total over degenerate and extreme rectangles

- **WHEN** `help_band` is called with a zero-width body, a zero-height body, a 1x1 body, a
  120x1 body, a 120x2 body, a body at `u16::MAX` width and height, and `content_rows` of
  `0`, `1`, `42`, and `usize::MAX` against each
- **THEN** no call panics, overflows, or underflows
- **AND** every returned rectangle lies entirely inside the body it was given: its `x` and
  `y` are at least the body's, and its right and bottom edges do not exceed the body's
- **AND** a zero-height body returns a zero-height rectangle, which `ui::help::render`
  answers by drawing nothing

#### Scenario: The overlay does not move the breakpoint

- **WHEN** a dashboard with `help.open` true is rendered at 120x40 and at 60x20, and
  `split_body` is called for each
- **THEN** `split_body` returns the two-region result at 120 and the one-region result at
  60, byte-identical to what it returns for the same dashboard with `help.open` false
- **AND** the band's `x` and `width` equal the body's at both, so the overlay spans both
  regions and the divider column at 120 rather than sitting inside one of them

## MODIFIED Requirements

### Requirement: The frame is a body and a footer row

`ui::view::render(frame, &Dashboard)` SHALL be a pure function of its two arguments: it
SHALL perform no filesystem, process, environment, network, or terminal I/O, and SHALL
read no clock and no global state. It SHALL divide `frame.area()` vertically into exactly
**two** regions, in order — a body of `Constraint::Min(0)` and a footer of
`Constraint::Length(1)` — and SHALL draw nothing outside them.

There SHALL be no header row. `pane-chrome` removes it: in a Herdr split the pane is
already titled by Herdr, and the literal `OpenSpec` label this requirement used to mandate
at row 0 column 0 restated that title one row below it while spending the row that could
have named the repository. The repository's identity moves into the list region's own
heading row, specified by "The list region's heading names the repository directory"; no
part of the frame draws the literal `OpenSpec` any more.

The footer SHALL render the key hints `? help`, `q quit`, `Enter detail`, and
`Esc back` in that order, separated by two spaces, starting at column 0, dropping hints
from the **end** when the remaining width cannot hold the next one whole.

`? help` is `help-overlay`'s addition and is placed **first**, which is the only position
that works: hints are dropped from the end, and the one key that reveals every other key
must be the last hint standing rather than the first one lost. It costs eight columns with
its separator and takes the base footer from **30** to **38**.

`agent-launch` adds **two further hints, placed after `Esc back`**: when
`Dashboard::agents.reachable` is `true` the footer SHALL append `a/c/s launch` and then
`g focus`, joined by the same two-space separator. When it is `false` both SHALL be absent
entirely, so a pane with no reachable Herdr socket renders the footer this requirement
specified before `agent-launch` existed — which is the whole of `SPEC.md` → Degraded states'
"action keys hidden".

They are **one compound hint plus one**, not four separate ones, and that is a width decision
rather than a stylistic one: `? help  q quit  Enter detail  Esc back  a apply  c continue  s archive`
is **70** columns, so at the mandated 60-column frame `fit_hints` would drop `s archive` and
`g focus` and offer the reader two of the four action keys. `a/c/s launch  g focus` costs 23
columns including its separators, bringing the footer to **61**. The keys themselves are
documented in `SPEC.md` → Keys and `README.md` → Keys; the footer's job is to say the feature
is available here and now, not to be the manual.

**`help-overlay` changes one measured consequence of that decision and repairs its
rationale.** At **61** columns the reachable footer no longer fits the mandated 60-column
frame: `g focus` is now dropped there, where before this change it fitted at 53. The original
argument against four separate action hints was that dropping two of them would leave the
reader with "no indication that the other two exist" — and that argument no longer holds,
because `? help` is now the first hint on the row and the overlay it opens lists `g` under
`Agents` with its own description. The footer is the always-visible minimum and the overlay is
the full list; a hint dropped at a narrow width is now a hint the reader can still find, which
is the whole reason this change exists. The compound `a/c/s launch` is kept as it is: it is
still the form that fits the most keys into the fewest columns, and nothing about the drop
order moved.

`agent-attribution` adds a **hint placed last**: when
`Dashboard::attribution().unattributed` is greater than zero, the footer SHALL append that
count, a single space, and the word `unattributed` — `1 unattributed`, `12 unattributed` —
after the action hints, joined by the same two-space separator. When the count is zero the hint
SHALL be absent entirely, so an agentless pane's footer is byte-identical to the footer this
requirement already specified. Being last means it is the **first** hint dropped as the width
falls, which is the correct priority: the key hints tell a reader how to drive the
pane, and the count tells them something they can act on later.

That priority has a measured consequence `agent-launch` makes explicit rather than leaving to be
discovered: the full reachable footer with a count is **77** columns, so at the mandated
60-column frame **the count is dropped and `g focus` with it, and the remaining action hint is
kept**. The count still fits whenever the socket is unreachable — that footer is **54**
columns — because the two hints it competes with are absent then. The drop order is unchanged
— last hint first — and the hint list grew twice; nothing about the rule moved.

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
solver, on the same terms and for the same measured reason as before: `Layout::vertical([
Min(0), Length(1)])` at height 1 gives the single row to the **footer**, so a naive split
renders `? help` where the repository's name belongs. The explicit branch is: at height 0
`render` SHALL draw nothing; at height 1 it SHALL draw the **body** only, which is one row
and therefore exactly the routed region's heading row; at height 2 or more it SHALL use the
two-way split above, giving the body every row but the last. `render` SHALL NOT panic at any
frame size of at least one column by one row, and SHALL NOT panic at an interior of zero
columns or zero rows, which a one- or two-column frame produces.

The row this change frees is spent on content, not on air: at a 20-row frame the body grows
from eighteen rows to nineteen and the region's own former bottom border row is gone as
well, so a region's interior grows from sixteen rows to seventeen. That count is asserted by
"The routed region is emphasised and region interiors are left empty" rather than here.

#### Scenario: Body and footer occupy their rows at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose
  `agents.reachable` is `false` is rendered into a `TestBackend` at 60x20, and again at 120x20
- **THEN** in both buffers row 0 is the routed region's heading row — it spells `demo-repo`
  from column 1 — and the string `OpenSpec` appears in no cell of either buffer
- **AND** in both buffers row 19 begins with the exact string
  `? help  q quit  Enter detail  Esc back` at column 0 — thirty-eight characters — and every
  remaining cell of row 19 is a space
- **AND** in both buffers no box-drawing character appears in column 0 or in column
  `width - 1` of any row, so the body occupies rows 0 through 18 with no bordered block in
  it and nothing is drawn in row 19 by the body

#### Scenario: The action hints follow `Esc back` when the socket is reachable

- **WHEN** the same `Dashboard` with `agents.reachable` set to `true` and no agents is
  rendered at 60x20 and at 120x20
- **THEN** row 19 at 120 is exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch  g focus` — **61** characters —
  followed by fifty-nine spaces
- **AND** row 19 at 60 is exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch` — **52** characters — followed by
  eight spaces: the full form needs 61 columns, so `g focus` is dropped whole, which is
  `help-overlay`'s one measured regression at the narrow width and the one the overlay itself
  answers
- **AND** rendering the identical dashboard with `agents.reachable` `false` produces row 19 of
  exactly `? help  q quit  Enter detail  Esc back` followed by twenty-two spaces at 60
- **AND** nothing outside row 19 differs between the two renders at either width, so the
  reachability flag moves the footer and nothing else

#### Scenario: The action hints are dropped whole, `g focus` first

- **WHEN** the reachable, agentless dashboard is rendered at 61x20, 60x20, 52x20, and 51x20
- **THEN** the 61-column row is exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch  g focus`, filling the row with no
  trailing space
- **AND** the 60-column row is exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch` followed
  by eight spaces — `g focus` and its two-space separator need nine columns and only eight
  remain, so it is dropped whole rather than cut to `g focu`
- **AND** the 52-column row is exactly `? help  q quit  Enter detail  Esc back  a/c/s launch`,
  filling the row, and the 51-column row is exactly
  `? help  q quit  Enter detail  Esc back` followed by thirteen spaces
- **AND** at 51 columns all four key hints are still present, `? help` among them, so both
  action hints are dropped before any of them and the help key is never one of the dropped

#### Scenario: A one-row frame renders the body's heading row and nothing else

- **WHEN** the same `Dashboard` is rendered at 60x1 and at 120x1
- **THEN** neither render panics
- **AND** row 0 spells `demo-repo` from column 1 in both — the routed region's heading row,
  which is the body's only row at this height
- **AND** neither `? help` nor `q quit` appears in either buffer, so the footer was not drawn
  into the body's single row

#### Scenario: A two-row frame renders one body row and the footer

- **WHEN** the same `Dashboard` is rendered at 60x2 and at 120x2
- **THEN** neither render panics
- **AND** row 0 spells `demo-repo` from column 1 and row 1 begins `? help` at column 0
- **AND** no list row is drawn anywhere in either buffer, because the body received one row
  and the heading row consumed it

#### Scenario: A one-column frame renders without panicking

- **WHEN** the same `Dashboard` is rendered at 1x1, at 1x20, at 2x20, and — as the
  contrasting controls at the two mandated widths — at 60x20 and 120x20
- **THEN** none of the five panics, including the two whose list region has an interior of
  zero columns
- **AND** the 1x20 buffer's row 0 is a single space: the region's one column is its left
  gutter, its interior is zero columns wide, and the heading row is therefore truncated to
  nothing rather than drawn over the gutter
- **AND** the 1x20 buffer's row 19 is a single space: `? help` needs six columns, so the
  first hint is dropped whole rather than truncated to `?`
- **AND** the same holds with `agents.reachable` `true`, which adds no hint that could fit in
  one column and therefore changes no cell of the 1x20 buffer
- **AND** the 60x20 and 120x20 buffers both spell `demo-repo` from column 1 of row 0 and
  both begin row 19 with `? help`, so the one-column result is a width branch rather than
  the heading and footer being absent everywhere

#### Scenario: The footer drops whole hints rather than truncating one

- **WHEN** the same `Dashboard`, with `agents.reachable` `false`, is rendered at 27x20, at
  28x20, at 60x20, and at 120x20
- **THEN** the 27-column footer row is exactly `? help  q quit` followed by thirteen spaces —
  `? help  q quit  Enter detail` needs exactly 28 columns, so `Enter detail` and every hint
  after it are dropped whole rather than cut short
- **AND** the 28-column footer row is exactly `? help  q quit  Enter detail`, filling the row
  with no trailing space, which pins the boundary from the other side
- **AND** the 60-column footer row is exactly `? help  q quit  Enter detail  Esc back` —
  thirty-eight characters — followed by twenty-two spaces, so all four hints fit at the
  mandated narrow width
- **AND** the 120-column footer row is the same thirty-eight characters followed by
  eighty-two spaces

#### Scenario: The unattributed count is the footer's last hint at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo`, whose `changes` holds
  one active change `alpha`, whose `agents.agents` holds one in-scope agent named
  `nothing-like-a-change`, and whose `agents.reachable` is `false`, is rendered at 60x20 and at
  120x20
- **THEN** the 60-column footer row is exactly
  `? help  q quit  Enter detail  Esc back  1 unattributed` — fifty-four characters — followed
  by six spaces
- **AND** the 120-column footer row is the same fifty-four characters followed by
  sixty-six spaces
- **AND** the same dashboard with `agents.reachable` set to `true` gives a 120-column footer row
  of exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` — **77**
  characters — followed by forty-three spaces, and a 60-column row of exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch` followed by eight spaces: the count
  needs sixteen further columns and `g focus` nine, and only eight remain after the launch
  hint, so **both** are dropped whole. That is the same last-hint-first rule, applied to a
  longer list
- **AND** rendering the identical dashboard with `agents.agents` empty and `reachable` `false`
  produces a footer row of exactly `? help  q quit  Enter detail  Esc back` and twenty-two
  spaces at 60 columns
- **AND** the count is a number of agents, not of changes: adding a second in-scope agent
  named `also-nothing` makes the hint read `2 unattributed` at 120 columns under either value
  of `reachable`, while the list region's rows are unchanged

#### Scenario: The count is reported with an empty change list

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo`, whose `changes` is
  `changes::empty_set()`, whose `agents.agents` holds two in-scope agents named
  `nothing-like-a-change` and `also-nothing`, and whose `agents.reachable` is `false`, is
  rendered at 60x20 and at 120x20
- **THEN** the footer row reads `? help  q quit  Enter detail  Esc back  2 unattributed` at
  both widths — fifty-four characters, which still fits the mandated narrow frame
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
  54x20 and at 53x20
- **THEN** the 54-column footer row is exactly
  `? help  q quit  Enter detail  Esc back  1 unattributed`, filling the row with no trailing
  space
- **AND** the 53-column footer row is exactly `? help  q quit  Enter detail  Esc back`
  followed by fifteen spaces — the count and its two-space separator need sixteen columns and
  only fifteen remain, so it is dropped whole rather than cut to `1 unattribute`
- **AND** at 53 columns all four key hints are still present, so the count is dropped
  before any of them
- **AND** the same dashboard with `agents.reachable` `true` pins the boundary one hint list
  further out: the 77-column row is exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` with no
  trailing space, and the 76-column row is exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch  g focus` followed by fifteen spaces —
  the count dropped whole, both action hints kept

#### Scenario: The filter prompt replaces the count along with the hints

- **WHEN** the one-unattributed-agent dashboard with `agents.reachable` `false` is rendered at
  60x20 and at 120x20 with `filter.active` set and `filter.query` `be`, and then again with
  `filter.active` cleared and the same query kept
- **THEN** the active-filter footer row is exactly `/be_` followed by spaces at both widths,
  and the string `unattributed` appears nowhere in it
- **AND** the accepted-query footer row is exactly
  `/be  ? help  q quit  Enter detail  Esc back  1 unattributed` — fifty-nine characters — at
  both widths, so the query leads the list and the count still trails it
- **AND** both forms are byte-identical to what `list-filtering` specifies once the same
  dashboard's `agents.agents` is emptied, so the count is additive rather than a rewrite of
  either form
- **AND** with `agents.reachable` `true` the active-filter row is still exactly `/be_` followed
  by spaces at both widths, and the strings `a/c/s launch` and `g focus` appear nowhere in it:
  the prompt replaces the **whole** row, action hints included
- **AND** with `agents.reachable` `true` the accepted-query row is exactly
  `/be  ? help  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` — 82
  characters — at 120, and exactly
  `/be  ? help  q quit  Enter detail  Esc back  a/c/s launch` — 57
  characters — followed by three spaces at 60, so the query still leads, the count is still
  the first thing dropped, and `g focus` is the second
