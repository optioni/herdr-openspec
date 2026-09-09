## ADDED Requirements

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

The footer SHALL render the key hints `q quit`, `Enter detail`, and
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
solver, on the same terms and for the same measured reason as before: `Layout::vertical([
Min(0), Length(1)])` at height 1 gives the single row to the **footer**, so a naive split
renders `q quit` where the repository's name belongs. The explicit branch is: at height 0
`render` SHALL draw nothing; at height 1 it SHALL draw the **body** only, which is one row
and therefore exactly the routed region's heading row; at height 2 or more it SHALL use the
two-way split above, giving the body every row but the last. `render` SHALL NOT panic at any
frame size of at least one column by one row, and SHALL NOT panic at an interior of zero
columns or zero rows, which a one- or two-column frame produces.

The row this change frees is spent on content, not on air: at a 20-row frame the body grows
from eighteen rows to nineteen and the region's own former bottom border row is gone as
well, so a region's interior grows from sixteen rows to eighteen. That count is asserted by
"The routed region is emphasised and region interiors are left empty" rather than here.

#### Scenario: Body and footer occupy their rows at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose
  `agents.reachable` is `false` is rendered into a `TestBackend` at 60x20, and again at 120x20
- **THEN** in both buffers row 0 is the routed region's heading row — it spells `demo-repo`
  from column 1 — and the string `OpenSpec` appears in no cell of either buffer
- **AND** in both buffers row 19 begins with the exact string
  `q quit  Enter detail  Esc back` at column 0, and every remaining cell of row 19 is a
  space
- **AND** in both buffers no box-drawing character appears in column 0 or in column
  `width - 1` of any row, so the body occupies rows 0 through 18 with no bordered block in
  it and nothing is drawn in row 19 by the body

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

#### Scenario: A one-row frame renders the body's heading row and nothing else

- **WHEN** the same `Dashboard` is rendered at 60x1 and at 120x1
- **THEN** neither render panics
- **AND** row 0 spells `demo-repo` from column 1 in both — the routed region's heading row,
  which is the body's only row at this height
- **AND** no `q quit` appears in either buffer, so the footer was not drawn into the body's
  single row

#### Scenario: A two-row frame renders one body row and the footer

- **WHEN** the same `Dashboard` is rendered at 60x2 and at 120x2
- **THEN** neither render panics
- **AND** row 0 spells `demo-repo` from column 1 and row 1 begins `q quit` at column 0
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
- **AND** the 1x20 buffer's row 19 is a single space: `q quit` needs six columns, so the
  first hint is dropped whole rather than truncated to `q`
- **AND** the same holds with `agents.reachable` `true`, which adds no hint that could fit in
  one column and therefore changes no cell of the 1x20 buffer
- **AND** the 60x20 and 120x20 buffers both spell `demo-repo` from column 1 of row 0 and
  both begin row 19 with `q quit`, so the one-column result is a width branch rather than
  the heading and footer being absent everywhere

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


### Requirement: A region is a heading row above a gutter-padded interior

A region SHALL be drawn without a border. `ui::view::render_region` SHALL draw no
`Block`, no box-drawing character, and no title; it SHALL draw exactly one **heading row**
across the region's interior width, and nothing else.

A region's geometry SHALL be, for a region rectangle `area`:

| Part | Rectangle |
|---|---|
| left gutter | column `area.x`, every row of `area` |
| right gutter | column `area.x + area.width - 1`, every row of `area` |
| heading row | `Rect::new(area.x + 1, area.y, area.width - 2, 1)` |
| interior | `Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 1)` |

`ui::layout::interior(area)` SHALL return that interior: the origin advanced by one column
and one row and clamped to the rectangle's own right and bottom edges, the width reduced by
two saturating to zero, and the height reduced by **one** saturating to zero. The clamp is
not decoration — at a 1x1 or 0x0 rectangle it is the difference between `x: 0` and `x: 1`.
Only the height rule changes from the border arithmetic this function used before: a region
spends one row on its heading and none on a bottom border.

The gutters are why the mandated interior widths do not move. A bordered region's interior
was its area less two border columns; a borderless region's interior is its area less two
gutter columns, and the two are the same arithmetic. **38** and **58** for the list and
**78** and **58** for the detail are therefore unchanged by this change, and every landed
row-grammar, markdown, tasks, and detail test that names them stays true. What changes is
the interiors' **height**: sixteen rows at a 20-row frame becomes **eighteen**, two gained —
one from the frame's removed header row and one from the region's removed bottom border.

Nothing SHALL be drawn in a gutter column except the divider below. At `LayoutMode::Wide`
`ui::view` SHALL draw a **vertical rule** — the character `│` — down the detail region's
**left gutter** column for every row of the body, styled `palette::style(Role::RegionRule)`.
That column is `40` at the mandated 120-column frame. At `LayoutMode::Narrow` no vertical
rule SHALL be drawn at all, because there is only one region and nothing to separate it
from. The rule belongs to neither region: it is drawn by `render_body`, which is the one
place that knows both rectangles.

#### Scenario: A region draws a heading row and no border at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo`, holding three active
  changes, at `Route::List`, is rendered at 120x20 and at 60x20
- **THEN** no box-drawing character other than `│` appears in either buffer — no `┌`, no
  `┐`, no `└`, no `┘`, and no `─`
- **AND** in the 60-column buffer row 0 columns 1 through 58 are the list region's heading
  row and row 1 columns 1 through 58 are the first list row, so the interior begins one row
  below the heading
- **AND** in the 60-column buffer every cell of column 0 and of column 59 is a space in
  every row of the body, so both gutters are empty

#### Scenario: `interior` reserves one row, not two

- **WHEN** `layout::interior` is called with `Rect::new(0, 0, 60, 19)`, with
  `Rect::new(40, 0, 80, 19)`, with `Rect::new(0, 0, 1, 1)`, and with `Rect::new(0, 0, 0, 0)`
- **THEN** the results are `Rect::new(1, 1, 58, 18)`, `Rect::new(41, 1, 78, 18)`,
  `Rect::new(0, 0, 0, 0)`, and `Rect::new(0, 0, 0, 0)`
- **AND** the first two interiors are eighteen rows tall, two more than the sixteen the
  bordered arithmetic gave at the same frame height, and their widths and `x` origins are
  unchanged

#### Scenario: The vertical rule occupies the divider column above the breakpoint only

- **WHEN** the same `Dashboard` is rendered at 120x20, at 100x20, at 99x20, and at 60x20
- **THEN** in the 120-column buffer every cell of column 40 in rows 0 through 18 is `│`, and
  no other column of any row holds a `│`
- **AND** in the 100-column buffer every cell of column 40 in rows 0 through 18 is `│`, so
  the divider column is fixed by the list column's `Length(40)` rather than by the frame
  width
- **AND** in the 99-column and 60-column buffers the character `│` appears in no cell at all
- **AND** in the 120-column buffer row 19 — the footer — holds no `│`, so the rule is
  confined to the body

#### Scenario: A one- and two-column region degenerates without drawing over a gutter

- **WHEN** the same `Dashboard` is rendered at 1x20, at 2x20, and at 3x20
- **THEN** none of the three panics
- **AND** in the 1x20 and 2x20 buffers no heading text and no list row is drawn at all: the
  interior is zero columns wide, so there is nothing to draw into
- **AND** in the 3x20 buffer the heading row and every list row occupy column 1 alone, and
  columns 0 and 2 are spaces in every row

### Requirement: The list region's heading names the repository directory

The list region's heading row SHALL render the repository root's **final path component** —
its directory name — when a root was found, and the literal `no repository` when none was.
It SHALL be drawn left-aligned at the heading row's first column, which is the interior's
first column. When the root has no final component — the filesystem root `/` — the whole
display path SHALL be rendered instead, so the row is never empty when a root exists.

The absolute path this row used to carry is gone. It was measured at a 60-column split to
consume the row whole while naming, in its last component, the only part a reader uses; the
directory name is that component. A reader who needs the absolute path has the pane's own
`no repository` rows and the Herdr pane's own working directory, neither of which this row
duplicates.

**The `file mode` badge.** When `Dashboard::file_mode` is true — the `openspec` binary probe
resolved no usable binary, so the change list is file-sourced for the whole session — the
heading row SHALL draw the literal `file mode`, nine columns, **right-aligned** so its last
column is the heading row's last column, styled `palette::style(Role::FileMode)`: ratatui's
`DIM` modifier — and no other modifier — together with foreground `Color::Yellow`. The badge
stays dim because it names a *mode*, not a fault: file mode is a supported way to run, and a
badge competing with the repository's name for attention would say otherwise. It is coloured
because `DIM` alone is what an archived row's date, an inline code span, a block quote, and
an agent badge already are, and a badge that shares its whole style with four other things
names nothing.

The badge SHALL be dropped **whole**, never cut short, when the heading row cannot hold the
name, at least one separating blank, and the badge's nine columns together — `change-rows`'
drop-whole rule. It is dropped **before** the name is shortened, not after: at a width that
cannot hold both, the reader can still learn the mode from the pane's behaviour, and a
half-drawn `file mo` would name nothing at all. When `file_mode` is false the heading row
SHALL be byte-identical to the row this requirement specifies with no badge — no reserved
columns and no changed budget.

Let `A` be the heading row's width when no badge is drawn, and its width minus ten — the
badge's nine columns and one separating blank — floored at zero, when one is. When the
name's `layout::columns` is at most `A` it SHALL be rendered whole. When it is longer it
SHALL be shortened from the **left** by `change-rows`' shared `shorten_left` implementation:
`…` followed by the longest suffix ending on a grapheme-cluster boundary that measures at
most `A - 1` columns. A directory's own last characters are what distinguish it from its
siblings. When `A` is zero the name SHALL be omitted entirely.

Shortening SHALL count **display columns**, not characters and not bytes, and the heading
row SHALL never draw past its last column at any width for any repository name.

The badge SHALL be drawn on the **list** region's heading row, not on the detail region's.
The two are different claims: a missing binary is a fact about the process, true of every
change in the pane, while the detail heading describes one change — and `SPEC.md` → Degraded
states already gives the per-change equivalent its own row, "Schema unknown to the CLI",
whose fall-back to file mode is per change and is named in the detail region instead. Below
the breakpoint at `Route::Detail` the list region is not drawn at all and the badge is
therefore not drawn either; that is accepted, and is the same trade the routed-region rule
already makes for every list row.

#### Scenario: The heading names the directory, not the path, at both widths

- **WHEN** a `Dashboard` whose repository root is `/Users/dev/Code/herdr-openspec`, whose
  `file_mode` is `false`, at `Route::List`, is rendered at 120x20 and at 60x20
- **THEN** in both buffers row 0 spells exactly `herdr-openspec` from column 1, followed by
  spaces to the heading row's last column
- **AND** the string `/Users/dev/Code` appears in no cell of either buffer
- **AND** in the 120-column buffer the heading row ends at column 38 and column 39 is a
  space, so the heading stayed inside the list region's interior

#### Scenario: A name longer than the heading row keeps its tail

- **WHEN** a `Dashboard` whose repository root's final component is a 50-character
  directory name, whose `file_mode` is `false`, is rendered at 120x20
- **THEN** row 0 columns 1 through 38 spell `…` followed by the name's last 37 columns
- **AND** rendering the same dashboard at 60x20 spells the name whole from column 1, because
  50 columns fit in 58 — so the ellipsis at 120 is the narrower list column and not a
  constant

#### Scenario: The badge is right-aligned and dropped whole

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose `file_mode` is
  `true` is rendered at 120x20, at 60x20, at 20x20, and at 19x20
- **THEN** in the 120-column buffer row 0 spells `demo-repo` from column 1 and `file mode`
  in columns 30 through 38 — the heading row's last nine columns — and every cell of that
  badge reports `Modifier::DIM` set and foreground `Color::Yellow`
- **AND** in the 60-column buffer the badge occupies columns 50 through 58, the heading row's
  last nine, so it is right-aligned against the row rather than placed at a fixed column
- **AND** in the 20-column buffer — a heading row of 18 columns — `demo-repo` is nine
  columns and the badge needs ten more, so both are drawn: `demo-repo` from column 1 and
  `file mode` in columns 10 through 18
- **AND** in the 19-column buffer — a heading row of 17 columns — the badge is absent from
  every cell and `demo-repo` is drawn whole, so the badge was dropped whole rather than cut
- **AND** rendering the 120x20 case with `file_mode` `false` gives a row 0 byte-identical to
  the first scenario's, so the badge is additive

#### Scenario: No repository names itself in the heading

- **WHEN** a `Dashboard` whose `repo` is `None`, whose `searched_from` is
  `/tmp/not-a-repo/deep/here`, is rendered at 120x20 and at 60x20
- **THEN** row 0 spells exactly `no repository` from column 1 in both
- **AND** the list region's interior still holds the three rows `change-rows` specifies for
  that state, beginning at row 1
- **AND** the string `not-a-repo` appears only in those interior rows and never in row 0

### Requirement: A region's heading style and the rules' style are palette roles

`ui::view` SHALL take every style it applies to a region's heading row and to either rule
from the palette, and SHALL construct no `Style` of its own.

A region's heading row SHALL be drawn with `palette::style(Role::RegionHeadingFocused)` when
the dashboard's route names that region, and with `palette::style(Role::RegionHeading)`
otherwise. `Role::RegionHeadingFocused` SHALL carry `Modifier::BOLD` and **no colour**;
`Role::RegionHeading` SHALL carry `Modifier::DIM` and **no colour**. The pair says which
region the keyboard is driving, which is exactly what the bold border said before; neither
carries a colour, because the distinction is between two regions of the same pane and a hue
would claim a meaning the region does not have.

The `file mode` badge keeps `Role::FileMode` and is drawn **over** the heading row's style
rather than under it, so the badge is dim and yellow whether or not the list region is the
routed one.

Both rules SHALL be drawn with `palette::style(Role::RegionRule)`, which SHALL carry
`Modifier::DIM` and no colour: a rule is a separator, and a separator that competes with the
text on either side of it has failed at its one job.

`Role::RegionBorder`, `Role::RegionBorderFocused`, `Role::HeaderTitle`, `Role::HeaderPath`,
and `Role::DetailHeader` SHALL be removed from `ui::palette::Role`, because nothing draws a
border, a frame header title, a frame header path, or a separately-styled detail header any
more. The removal is specified by `view-palette`, which owns the role table; this
requirement names it only so the two are not read as disagreeing.

#### Scenario: The routed region's heading is bold and the other's is dim

- **WHEN** a `Dashboard` with `route: Route::List` and one active change is rendered at
  120x20
- **THEN** the cell at column 1, row 0 reports `Modifier::BOLD` set and `Modifier::DIM` not
  set
- **AND** the cell at column 41, row 0 — the detail region's heading row — reports
  `Modifier::DIM` set and `Modifier::BOLD` not set
- **AND** with `route: Route::Detail` and the same size the two assertions swap, so the test
  discriminates rather than asserting a constant
- **AND** at 60x20 the single drawn region's heading cell at column 1, row 0 reports
  `Modifier::BOLD` set under **both** routes, because the region that is drawn is always the
  routed one below the breakpoint

#### Scenario: No heading or rule cell carries a colour

- **WHEN** the same dashboards are rendered at 120x20 and at 60x20 with `file_mode` `false`
- **THEN** no cell of either heading row and no cell of the vertical rule reports a
  foreground or a background, so the palette gave each a role and not a colour
- **AND** every cell of the vertical rule reports `Modifier::DIM` set
- **AND** with `file_mode` `true` the nine badge cells of the list heading row do report
  foreground `Color::Yellow`, so the absence of colour above is a property of the heading
  role rather than of the row

#### Scenario: Every colour literal still lives in the palette module alone

- **WHEN** `src/`, inline `#[cfg(test)]` modules included, is searched for
  `ratatui::style::Color`
- **THEN** `src/ui/palette.rs` is the only file that names it
- **AND** the render tests above assert a cell's colour by comparing it against
  `palette::style(role)` rather than against a literal

## MODIFIED Requirements

### Requirement: The 100-column breakpoint decides one region or two

`ui::layout::WIDE_MIN_WIDTH` SHALL be `100`. `ui::layout::mode(width: u16)` SHALL return
`LayoutMode::Wide` when `width >= WIDE_MIN_WIDTH` and `LayoutMode::Narrow` otherwise, and
SHALL be a total function over every `u16`.

At `LayoutMode::Wide` the body SHALL be split horizontally into exactly two regions —
`Constraint::Length(40)` for the change list on the left and `Constraint::Min(0)` for the
artifact detail on the right — so that every column gained beyond 100 goes to the detail
side. Both regions SHALL be drawn, each as a heading row above a gutter-padded interior and
neither as a bordered block. Their headings are not fixed titles: the list region's heading
names the repository directory and the detail region's heading is the selected change's own
header. The two are separated by the vertical rule in the detail region's left gutter.

At `LayoutMode::Narrow` the body SHALL hold exactly one region occupying the whole body
width, drawn the same borderless way, and it SHALL be the list region when the dashboard's
route is `Route::List` and the detail region when it is `Route::Detail`. The region that is
not routed to SHALL NOT be drawn at all, and no vertical rule SHALL be drawn.

The literal titles `Changes` and `Detail` are gone with the borders that carried them. They
named the two halves of a split the reader can already see, and at a 60-column pane each
spent a row saying which of the two routes was showing — which the footer's `Enter detail` /
`Esc back` hints and the heading's own content already say.

The mode SHALL be derived from the frame area passed to `render` on every draw, and SHALL
NOT be stored on `Dashboard` or captured at startup, so a terminal resized across the
breakpoint changes layout on its next frame with no extra state.

#### Scenario: At 120 columns both regions are drawn with the divider at column 40

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  and one active change `alpha` is rendered at 120x20, and — as the contrasting control at
  the mandated narrow width — at 60x20
- **THEN** in the 120-column buffer row 0 spells `demo-repo` from column 1 and `alpha` from
  column 41, so both regions drew their heading rows
- **AND** in the 120-column buffer every cell of column 40 in rows 0 through 18 is `│`, and
  columns 0, 39, and 119 are spaces in every one of those rows
- **AND** the strings `Changes` and `Detail` appear in no cell of either buffer
- **AND** the 60-column buffer holds no `│` at all, so the divider is a width branch rather
  than something drawn unconditionally

#### Scenario: At 60 columns only the routed region is drawn

- **WHEN** the same `Dashboard` with `route: Route::List` is rendered at 60x20, and — as
  the contrasting control at the mandated wide width — at 120x20
- **THEN** in the 60-column buffer row 0 spells `demo-repo` from column 1 and the change's
  own header appears in no row, because the detail region was not drawn
- **AND** in the 60-column buffer no cell holds `│`
- **AND** the 120-column buffer does hold the change's header at row 0 column 41, so the
  absence at 60 columns is the breakpoint and not the detail heading being missing

#### Scenario: At 60 columns the detail route replaces the list region

- **WHEN** the same `Dashboard` with `route: Route::Detail` is rendered at 60x20
- **THEN** row 0 spells the selected change's header from column 1
- **AND** the string `demo-repo` appears in no row of the buffer, because the list region —
  and with it the repository heading — is not drawn
- **AND** rendering the same dashboard at 120x20 still shows **both** `demo-repo` at row 0
  column 1 and the change's header at row 0 column 41, because the route selects emphasis
  rather than visibility above the breakpoint

#### Scenario: The breakpoint is exact at 99, 100, and 101 columns

- **WHEN** `ui::layout::mode` is called with `0`, `1`, `40`, `60`, `99`, `100`, `101`,
  `120`, and `u16::MAX`
- **THEN** it returns `Narrow` for `0`, `1`, `40`, `60`, and `99`, and `Wide` for `100`,
  `101`, `120`, and `u16::MAX`
- **AND** rendering the same `Dashboard` produces no `│` in the buffer at 60x20 and at
  99x20, and a full column of `│` at column 40 at 100x20, at 101x20, and at 120x20, so both
  mandated widths and all three boundary widths are rendered, not just computed

#### Scenario: The mode follows the current frame, not the startup size

- **WHEN** one `Terminal<TestBackend>` is created at 120x20, a frame is drawn, the backend
  is resized to 60x20, and a second frame is drawn from the **same** unchanged `Dashboard`
- **THEN** the first buffer holds a column of `│` at column 40 and the second holds no `│`
  at all
- **AND** `Dashboard` exposes no field naming a width, a layout mode, or a column count

### Requirement: The routed region is emphasised and region interiors are left empty

The region the dashboard's route names SHALL have its **heading row** drawn with
`Modifier::BOLD` set and `Modifier::DIM` clear; the other region, when drawn, SHALL have its
heading row drawn with `Modifier::DIM` set and `Modifier::BOLD` clear. This replaces the bold
border that carried the same claim before `pane-chrome` removed the borders.

Neither region's interior is left blank unconditionally any longer. The **list** region's
interior is owned by `change-rows`, with `list-selection` owning which slice is drawn. The
**detail** region's interior is owned by `artifact-tabs` (its second row, under a blank
first row), the horizontal rule below it, and `artifact-content` and `detail-scroll` (the
content area beneath that), divided by `layout::split_detail`. The detail region's **change
header** is no longer part of its interior at all: `detail-header` draws it into the
region's heading row, one row above.

The detail interior is blank on a frame **exactly when `Dashboard::visible()` is empty** — no
repository, no changes, or a `/` filter matching none — and its heading row is blank on
exactly the same condition. That is the whole of the blank case: with a change selected, the
region always carries a heading, a blank row, a tab bar, a rule, and at least one content
line, because `ui::detail::content_lines` returns `No content yet` rather than nothing.

When the interior **is** blank, every cell of it is a space whose `Style` equals
`ratatui::buffer::Cell::default().style()`. The comparison is against `Cell::default().style()`
and **not** against `Style::default()`: `ratatui-crossterm` re-enables the `underline-color`
feature through its own defaults, so an untouched cell's style is
`fg(Reset).bg(Reset).underline_color(Reset)`, which equals neither `Style::default()` nor
`Style::reset()`. Comparing against the constructible value is what catches a style being
applied to a whole region where one row was meant.

Content SHALL NOT bleed into a gutter or across the divider: no cell of a region's left or
right gutter column SHALL be overwritten by a list row, a heading, a detail header, a tab
cell, a rule, a problem line, or a markdown line, at either mandated width. The one cell any
of them may write in a gutter is the vertical rule, which `render_body` draws into the detail
region's left gutter and which no region writes.

#### Scenario: The routed region's border is bold and the other's is not

The scenario's name is kept verbatim because a delta's scenario headers are its merge key.
There is no border any more; the claim it made — the routed region is the emphasised one —
is now made by the heading row.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  and one active change is rendered at 120x20
- **THEN** the cell at column 1, row 0 reports `Modifier::BOLD` set
- **AND** the cell at column 41, row 0 reports `Modifier::BOLD` **not** set and
  `Modifier::DIM` set
- **AND** with `route: Route::Detail` and the same size, the two assertions swap, so the
  test discriminates rather than asserting a constant
- **AND** at 60x20 the single drawn region's heading cell at column 1, row 0 reports
  `Modifier::BOLD` set under **both** routes, because the region that is drawn is always
  the routed one below the breakpoint

#### Scenario: Interiors are blank at both widths

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
the detail interior is blank now because no change is selected, not because nothing may
write there.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  `changes::empty_set()`, and an empty `detail` is rendered at 60x20 and at 120x20
- **THEN** in the 120-column buffer every cell in rows 1 through 18 and columns 41 through
  118 is a space whose `Style` equals `Cell::default().style()` — the detail region's
  interior is untouched because `visible()` is empty
- **AND** in the 120-column buffer row 0 columns 41 through 118 are spaces too, because the
  detail region's heading row is its change header and there is no change to name
- **AND** in the 120-column buffer row 1, columns 1 through 38, begins `No changes yet`, so
  the list interior is written by `change-rows` rather than left blank, one row below its
  heading
- **AND** in the 60-column buffer row 1, columns 1 through 58, begins `No changes yet`, and
  rows 2 through 18 of columns 1 through 58 are entirely spaces whose `Style` equals
  `Cell::default().style()`, so exactly one message row was drawn into an eighteen-row
  interior
- **AND** the same dashboard with **one** active change added is no longer blank in the
  detail region at 120x20: row 0 columns 41 onward holds that change's header, so the
  blankness asserted above is a property of the empty visible list rather than a constant

#### Scenario: Rows do not overwrite the borders at either width

The scenario's name is kept verbatim because a delta's scenario headers are its merge key.
What a row must not overwrite is now a gutter column and the divider drawn in one of them.

- **WHEN** a `Dashboard` holding thirty active changes with names long enough to be
  truncated, whose selected change carries twelve artifacts with 40-character ids, and whose
  `detail.source` is thirty lines each 200 characters long, is rendered at 60x20 and at
  120x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 0 through 18
  is a space
- **AND** in the 120-column buffer every cell of columns 0, 39, and 119 in rows 0 through 18
  is a space, and every cell of column 40 in those rows is `│` — so a 38-column row neither
  ran into the divider nor into the detail region, and neither a 78-column heading, a
  78-column tab bar, a 78-column rule, nor a 78-column markdown line ran into the divider or
  past the frame
- **AND** the same holds at `Route::Detail` at 60x20, where the detail region is the only one
  drawn and no `│` appears at all

### Requirement: The detail region's two mandated interior widths are 78 and 58

The detail region's interior width SHALL be **78** at a 120-column frame — the wide layout's
`Constraint::Min(0)` column, 80 columns, less two **gutter** columns — and **58** at a
60-column frame in the detail route, where the region is the whole 60-column body less two
gutter columns. Both interiors SHALL be **18 rows** at a 20-row frame.

Only the height moves. `pane-chrome` replaced each region's two border columns with two
gutter columns, which is the same arithmetic, so both mandated widths are unchanged and
every landed expectation that names them stays true; it removed the frame's header row and
the region's bottom border row, which is where the two extra interior rows come from.

These two widths remain frozen for `detail-view` and `tasks-tab` to inherit, exactly as
`change-rows`' 38 and 58 are. Every test of `ui::markdown` SHALL name both, and a source
check SHALL enforce that with a floor on the number of tests found, on the same terms and
with the same stated limits as the check over `src/ui/list.rs`.

Because the wide layout's detail column is `Min(0)`, 78 is the width at the mandated frame
size and not a constant of the layout: every column gained beyond 120 goes to the detail
region. Nothing SHALL depend on 78 other than the expectations of tests rendered at 120.

#### Scenario: The detail interior is 78 columns at 120 and 58 at 60

- **WHEN** `layout::split_frame` and `layout::split_body` are applied to `Rect::new(0, 0,
  120, 20)` with `Route::Detail`, and the resulting detail rectangle is passed to
  `layout::interior`
- **THEN** the interior is `Rect::new(41, 1, 78, 18)`
- **AND** the same applied to `Rect::new(0, 0, 60, 20)` with `Route::Detail` gives
  `Rect::new(1, 1, 58, 18)`
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

### Requirement: A point in the frame resolves to exactly one zone

`ui::layout::zone(area: Rect, route: Route, column: u16, row: u16) -> Zone` SHALL map a
terminal coordinate to the part of the frame drawn there, deriving the geometry through the
same `split_frame`, `split_body`, `interior`, and `split_detail` the draw path uses and
holding no arithmetic of its own beyond a rectangle containment test. It SHALL be pure —
no filesystem, process, environment, network, or standard-I/O API — and total: every
`Rect`, every `Route`, and every `(column, row)` pair including `(0, 0)` and
`(u16::MAX, u16::MAX)` returns a `Zone` and none panics.

`Zone` SHALL have exactly five variants:

| Variant | Meaning |
|---|---|
| `ListRow { interior: Rect, row: u16 }` | A row of the list region's interior. `interior` is that interior's own rectangle and `row` is the offset of the addressed row below its first interior row |
| `List` | The list region, but not one of its interior rows — its gutters, its heading row, or the divider column beside it |
| `DetailTab { bar: Rect, column: u16 }` | The detail region's tab-bar row. `bar` is that row's own rectangle and `column` is the offset of the addressed column right of its first column |
| `Detail` | The detail region, anywhere but the tab-bar row: its gutters, its heading row, the blank row above the tab bar, the rule below it, or its content area |
| `Outside` | The frame's footer row, or a point outside the frame entirely |

`Outside` no longer covers a frame header row, because there is no longer one. Every row of
the frame but the last is now a body row and resolves to a region's zone whenever a region is
drawn there.

The divider column belongs to the region whose gutter it is — the detail region's left gutter
at `LayoutMode::Wide` — so a click on the rule resolves to `Detail` and a wheel over it
scrolls the detail region. It is one column and the reader who lands on it meant one of the
two regions; giving it to the one it is drawn inside is the answer that needs no special case.

`ListRow` and `DetailTab` SHALL carry the rectangle the zone was derived from rather than
only an offset, so the caller that resolves the offset to a row or a tab uses the very
geometry the hit test used. Recomputing the interior at the call site would be a second
derivation of the same rectangle, and the two could drift.

`zone` SHALL respect the route below the breakpoint exactly as `split_body` does: at
`LayoutMode::Narrow` only the routed region exists, so every point in the body resolves to
that region's zones and none to the other's. At `LayoutMode::Wide` both regions exist at
both routes, and the route changes nothing about the mapping.

`zone` SHALL name no ratatui widget, no crossterm type, and no mouse type: it takes two
integers, which is what keeps it in the pure view set and testable with no event at all.

#### Scenario: The zones tile the frame at 120 columns

- **WHEN** `zone` is called at a 120x40 frame, at `Route::List` and again at
  `Route::Detail`, for a point on the frame's footer row, the list region's heading row, the
  list region's left gutter, the list interior's first row, the list interior's last row, the
  divider column 40, the detail region's heading row, the detail interior's blank first row,
  the detail interior's tab-bar row, the rule row below it, the detail interior's first
  content row, and column 200
- **THEN** the results are `Outside`, `List`, `List`, `ListRow` with `row` 0, `ListRow`
  with `row` equal to the interior's last index, `Detail`, `Detail`, `Detail`, `DetailTab`
  with `column` 0, `Detail`, `Detail`, and `Outside`, at both routes
- **AND** every `ListRow`'s `interior` equals `interior(split_body(body, route).0.unwrap())`
  and every `DetailTab`'s `bar` equals `split_detail(interior(detail_area)).0`, computed
  independently in the test
- **AND** row 0 of the frame resolves to a region rather than to `Outside`, because the
  frame has no header row for it to belong to

#### Scenario: Below the breakpoint only the routed region has zones

- **WHEN** `zone` is called at a 60x20 frame at `Route::List` for the interior's first row,
  and then at `Route::Detail` for the same point
- **THEN** the first is a `ListRow` and the second is a `Detail` or `DetailTab`
- **AND** no point anywhere in the 60-column body resolves to a list zone at `Route::Detail`,
  and none resolves to a detail zone at `Route::List`

#### Scenario: The breakpoint is exact for the hit test too

- **WHEN** `zone` is called for the same body point at frame widths 99, 100, and 101 at
  `Route::Detail`
- **THEN** 99 resolves through the narrow layout and 100 and 101 through the wide one,
  agreeing with `layout::mode` at each width

#### Scenario: Degenerate frames resolve without panicking

- **WHEN** `zone` is called at frame areas `0x0`, `1x1`, `2x2`, `3x3`, and `120x2` — the
  heights `split_frame` branches on explicitly — for every `(column, row)` pair inside the
  area and for `(u16::MAX, u16::MAX)`, at both routes
- **THEN** every call returns a `Zone` and none panics
- **AND** a frame with no body resolves every point to `Outside`, since there is no region
  to be over

#### Scenario: The hit test agrees with what was drawn

- **WHEN** a dashboard with three active and three archived changes is rendered into a
  `TestBackend` at 120x40 and again at 60x20, and every cell of the resulting buffer is
  classified by `zone`
- **THEN** every cell `zone` reports as `ListRow` holds a character from `list::rows`' own
  output for that row, and every cell it reports as `List` or `Detail` in a gutter column
  holds either a space or the divider's `│`
- **AND** no cell of the drawn buffer is classified as belonging to a region the draw path
  did not draw

## REMOVED Requirements

### Requirement: The frame is a header row, a body, and a footer row

**Reason**: The frame no longer has a header row. In a Herdr split the pane is already
titled by Herdr, so the literal `OpenSpec` this requirement mandated at row 0 column 0
restated the pane's own title one row below it. Replaced by "The frame is a body and a
footer row", which carries every footer rule of this requirement unchanged.

**Migration**: None for a reader; the footer is byte-identical. Callers of
`ui::layout::split_frame` receive a two-tuple `(body, footer)` rather than a three-tuple
`(header, body, footer)`, and every body rectangle begins one row higher.

### Requirement: The header names the repository root, shortened from the left when narrow

**Reason**: There is no header row to name it in. Replaced by "The list region's heading
names the repository directory", which keeps the `file mode` badge, the drop-whole rule, the
display-column measure, and the shared `shorten_left` implementation, and changes what is
named — the directory rather than the absolute path — and where the badge sits — right-
aligned on the heading row rather than in columns 9 through 17.

**Migration**: A reader who wants the absolute path reads it from the Herdr pane's own
working directory. `ui::view::shorten_for_header` and `ui::view::render_header` are removed;
`ui::view::render_region` draws the heading.

### Requirement: A region's border style is a palette role

**Reason**: There is no border to style. Replaced by "A region's heading style and the
rules' style are palette roles", which keeps the discriminating-test rule, the no-colour
rule, and the blank-interior comparison against `Cell::default().style()`.

**Migration**: `Role::RegionBorder` and `Role::RegionBorderFocused` are replaced by
`Role::RegionHeading` and `Role::RegionHeadingFocused`; `Role::RegionRule` is added.
