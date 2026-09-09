## MODIFIED Requirements

### Requirement: The list region's interior holds one row per change, in the order the `ChangeSet` gives

`ui::list::rows(dashboard: &Dashboard, width: u16) -> Vec<Row>` SHALL be a pure total
function of its two arguments. It SHALL perform no filesystem, process, environment,
network, or terminal I/O, SHALL read no clock and no global state, and SHALL never panic
for any `Dashboard` value and any `u16` width, including `0`. It reaches the agent badge
through `Dashboard::attribution()`, which `agent-attribution` requires to be pure and derived
per call; that adds no I/O and no clock to this function.

`ui::view::render` SHALL draw the rows it returns into the **interior** of the list
region — the area below its heading row and its padding row and inside its two gutter
columns — starting at the interior's first row and first column, one `Row` per terminal row,
and SHALL draw nothing outside that interior. The two interiors the mandated frame widths
produce are fixed by `responsive-layout` and are **38 columns by 17 rows** at a 120x20 frame
(the wide layout's `Constraint::Length(40)` list part, less two gutter columns) and
**58 columns by 17 rows** at a 60x20 frame.

`pane-chrome` changes neither width and moves neither origin: it replaced the region's two
border columns with two gutter columns, which is the same arithmetic, and it spent one of the
two rows it freed — the frame's header row and the region's bottom border row — on the
region's own padding row. The interior therefore still begins at buffer row **2** at both
mandated frames, and gained exactly one row at the bottom. Every row index in this
capability's scenarios is unchanged except the last.

Rows SHALL be emitted in this order and no other:

1. one `Problem` row per entry of `dashboard.launch.problems`, in that vector's order;
2. one `Problem` row per entry of `dashboard.refresh.problems`, in that vector's order;
3. one `Problem` row per entry of `dashboard.changes.problems`, in that vector's order;
4. one `Section` row for the **active** section, only when that section's count is greater
   than zero;
5. the visible **active** changes, in `dashboard.changes.active`'s order, only when the
   active section is open — or, when that section's count is zero, the `Message` row or rows
   the empty-state requirement below names for the state the dashboard is in (one row for
   `No changes yet` and for `No active changes`, two for `No changes match` and its query
   line, three for the no-repository block, which replaces the whole list);
6. one `Section` row for the **archived** section, only when that section's count is greater
   than zero;
7. the visible **archived** changes, in `dashboard.changes.archived`'s order, only when the
   archived section is open.

**Sections.** `list-sections` replaces the single `Separator` row with two `Section` rows,
one per tier, each of which the reader can fold. `RowKind` SHALL become:

```rust
pub enum RowKind {
    Problem,
    Section { key: SectionKey, depth: u8, collapsed: bool },
    Item { index: usize },
    Message,
}
```

`RowKind::Separator` is removed; `Section` takes its place in the emission order and in
`ui::view`'s style table, where it SHALL map to the same `Role::ListSeparator` the separator
mapped to. That reuse is deliberate and is recorded rather than incidental: a palette role
names how a row should *look* — here, this list's one dim, non-informational grey — while a
`RowKind` names what a row *is*, and `view-palette` already records `AgentBadge(Unknown)`
sharing this same role with the separator. Renaming the role would restate four of
`view-palette`'s requirements for no change to a single rendered cell.

`depth` is `0` for both of this change's sections and is carried so that a later change can
nest a group under `archived` — by date, say — without reworking the row kind, the collapse
state, or the selection index. Nothing in this change reads a non-zero `depth`, and nothing
indents by it.

**A section's count** SHALL be:

- for the **active** section, the number of `dashboard.changes.active` entries the query
  matches;
- for the **archived** section, the number of `dashboard.changes.archived` entries the query
  matches — except when that tier is **unresolved**, meaning `changes.archived` is empty
  while `changes.archived_total` is greater than zero, in which case it is
  `changes.archived_total`.

A section header is emitted exactly when its count is greater than zero, so a repository with
nothing archived shows no archived header, exactly as it showed no separator, and a query
matching no archived change hides that header too. The count is what makes the fold honest:
a reader can see how much is behind a collapsed section before deciding to open it, which is
what the `archived_count` cap this change removes never told them.

The count is the number of changes the section **would show if it were open**, which for an
unfiltered pane is the tier's true total — twenty-two archived changes read `(22)` whether
the section is folded or not. It is never the number of rows actually drawn.

**A section's row grammar** is `[marker][space][glyph][space][label][space][(count)]`, passed
through the same `pad_or_truncate_right` every other row uses, so it measures exactly `width`
display columns at every width including `0`. The `glyph` is `▾` (U+25BE, BLACK DOWN-POINTING
SMALL TRIANGLE) when the section is open and `▸` (U+25B8, BLACK RIGHT-POINTING SMALL
TRIANGLE) when it is collapsed; the `label` is the literal `active` or `archived`; the count
is the decimal count in parentheses. A section header at 38 columns therefore reads
`  ▾ active (9)` followed by twenty-four spaces — the header is fourteen display columns —
and `  ▸ archived (22)`, seventeen columns, followed by twenty-one. No cell is dropped whole and no field is right-aligned: unlike a change row the
header is one label, and truncating it with `…` is the whole of its degradation.

`pane-chrome` replaces the `v`/`>` pair `list-sections` landed. That pair made the glyph and
the selection marker the **same character** in different columns, so a selected collapsed
section read `> > archived (22)` — two identical glyphs two columns apart, which a reader
parses as one repeated thing rather than as a cursor beside a fold state. The collision was
accepted at the time on the grounds that column `0` is the cursor on every row and column `2`
is the fold state on section rows alone; that argument is sound and is still why the columns
are unambiguous, but it asks the reader to hold a grammar in their head to read a glyph. Two
shapes that are not `>` cost nothing and ask nothing.

Both replacements SHALL measure **one display column**, which is what keeps every landed
column expectation in this capability true; `layout::columns` is the measure, and a section
header's total width is unchanged. Each is an unambiguous-width character in Unicode's East
Asian Width table, so the measure does not depend on a terminal's ambiguous-width setting.
The `+`/`-` pair remains rejected: `-` already leads a `[-]` progress cell in this same list.

**A section is open** when the reader has not collapsed it **or** `dashboard.filter.query`
is non-empty; `list-filtering` owns that force-open rule. A closed section emits its header
alone: its changes are not rows, are not addressable, and do not count toward
`RowKind::Item { index }`, which this capability already defines as an index into the
*visible* list rather than into `ChangeSet` — `list-sections` relies on that promise rather
than weakening it.

An **open** section whose tier is unresolved — the archived section just expanded, with the
refresh that resolves it still outstanding — emits its header, carrying the true count, and
no rows. No message row, no placeholder, and no problem is added for that state: it lasts one
file-tier refresh cycle, and `list-filtering`'s and `dashboard-loop`'s requirements are what
make the cycle happen.

A `Section` row SHALL carry no badge cell and no reserved badge column, at any width and
whatever `badges` holds, exactly as the separator did.

Launch problems lead, and that is `agent-launch`'s addition to an order `live-updates`
otherwise owns. They are the only rows in the list produced by a key the reader has **just
pressed**: a watcher that would not start is a standing condition and a `ChangeSet` problem is
a fact about the tree, but a failed launch is an answer to a question, and the pane's whole job
in the moment after `a` is to give it. `launch.problems` holds at most one entry, is replaced
wholesale by the next outcome or refusal, and is cleared by a success, so leading the list costs
at most one row and never accumulates.

Refresh problems precede change-set problems because the two have different lifetimes:
`refresh.problems` holds conditions that outlive a reload — today, a `notify` watcher that
would not start — while `ChangeSet::problems` is re-derived from the tree on every cycle and
`Dashboard::adopt` replaces it wholesale. A standing condition belongs above a transient one.
All three use the identical `! `-prefixed grammar and the identical `RowKind::Problem`, so
`ui::view` styles them the same way and none is addressable by `selected`.

The `repo.is_none()` early return SHALL be unchanged: a dashboard with no repository starts
no watcher, so it can carry no refresh problem, and — `agent-launch`'s addition to the same
argument — `start_collaborators` gives it `launch::none()` and `launch::decide` returns
`Decision::Nothing` for every key with no selected change, so it can carry no launch problem
either. The no-repository block stays exactly the
three rows the empty-state requirement names. It carries no badge either: with no repository
`attribution()` badges nothing, which the early return makes moot.

`rows` SHALL NOT sort, de-duplicate, or re-order either tier or either problem list:
`changes::from_files` already orders active changes name-ascending in byte order and archived
changes dated-newest-first, and a second ordering rule here would be a second place for the
two producers of `Change` to disagree.

**The agent badge.** `agent-attribution` claims the third column `SPEC.md` → List view
reserves. When `Dashboard::attribution().badges` holds an entry for a change's name, that
change's row SHALL carry a **one-column badge cell** between the name field and the progress
cell, separated from each by one space, so the full active grammar is
`[marker][space][name field][space][badge][space][progress]`. The badge character SHALL be
`w` for `Working`, `i` for `Idle`, `b` for `Blocked`, `d` for `Done`, and `?` for `Unknown` —
one ASCII column per status, collision-free against `>`, `!`, `…`, `-`, and `[`, every other
glyph this grammar already uses.

A change with **no** entry in `badges` SHALL carry no badge cell and no separating space, so
its row is byte-identical to the row the same dashboard produced before this change existed.
An agentless pane therefore renders exactly as it did.

Cells SHALL be dropped **whole**, never cut short, in a fixed order as the width falls:
first the badge cell and its separating space, when the name field would otherwise fall below
one column; then the progress cell and its separating space, when the name field would
**still** fall below one column; then — on an archived row — the date field and its separating
space, on the same condition; and only then does the row degenerate to
`[marker][space][name field]`. Dropping the newest and narrowest cell first is what leaves
every landed drop boundary where it was: once the badge is gone the row is
character-for-character the row this grammar already specified, so the widths at which the
progress cell and the date field drop are unchanged.

A badge cell SHALL be drawn on a **change row only**. A `Problem`, `Section`, or `Message`
row — including a launch problem, the `No changes yet`, `No active changes`,
`No changes match`, and no-repository rows — carries no badge cell and no reserved column, at
any width and whatever `badges` holds.

**Row numbering in this capability's scenarios** is the rendered **buffer** row, 0-based, so
row 0 is the region's heading row, row 1 is its padding row, and row 2 is the interior's
first row at the mandated frames. `pane-chrome` moved the interior's **last** index from 17
to 18 and moved nothing else; the convention and every other index below are unchanged. Every scenario below that asserts a row index uses that one convention, and every
scenario that asserts a selection marker states `selected` in its WHEN, because the marker
now follows a cursor that can rest on a section header.

#### Scenario: Active rows render at both mandated widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose
  `changes.active` is, in order, `add-token-refresh` at 4 of 9 tasks, `fix-empty-basket`
  at 7 of 7, and `migrate-ai-sdk-v7` at 0 of 0, with no archived changes, no problems of
  either kind, an empty filter query, `selected` **1** — the first change, since target 0 is
  now the active section header — and **no agents**, is rendered into a `TestBackend` at
  120x20 and again at 60x20
- **THEN** in both buffers row 2 is the active section header, exactly `  ▾ active (3)`
  padded to the interior width
- **AND** in the 120-column buffer the 38 cells of row 3, columns 1 through 38, spell
  exactly `> add-token-refresh              [4/9]`; row 4 spells
  `  fix-empty-basket               [7/7]`; and row 5 spells
  `  migrate-ai-sdk-v7                [-]`
- **AND** in the 60-column buffer the 58 cells of row 3, columns 1 through 58, spell
  exactly `> add-token-refresh                                  [4/9]`; row 4 spells
  `  fix-empty-basket                                   [7/7]`; and row 5 spells
  `  migrate-ai-sdk-v7                                    [-]`
- **AND** in both buffers every cell of rows 6 through 18 is a space, so exactly four rows
  were drawn and nothing was repeated into the remaining height
- **AND** the three change rows are byte-identical to the ones the same dashboard produced
  before `list-sections` existed at `selected` 0, and byte-identical to the ones it produced
  before `agent-attribution` added the badge cell: an empty `refresh.problems` costs no row,
  an empty `badges` costs no column, and the section header is the one row this change adds
- **AND** the same dashboard at `selected` **0** puts the `>` marker on the header row and a
  space in column 0 of all three change rows, so the marker follows the cursor onto a section
- **AND** the same dashboard with the active section **collapsed** renders row 2 as exactly
  `  ▸ active (3)` padded to the interior width, with no change name anywhere in either
  buffer, so the header's count is the tier's size and not the number of rows drawn

#### Scenario: A badged row carries its status between the name and the progress cell

- **WHEN** the same three-change dashboard is rendered at 120x20 and again at 60x20, this
  time carrying an in-scope `Working` agent named `add-token-refresh`, an in-scope `Blocked`
  agent named `fix-empty-basket`, and an in-scope agent named `migrate-ai-sdk-v7` whose status
  is `AgentStatus::Unknown` — what `agent-list` decodes an unrecognised or absent
  `agent_status` string to
- **THEN** in the 120-column buffer the 38 cells of row 3 spell exactly
  `> add-token-refresh            w [4/9]`; row 4 spells
  `  fix-empty-basket             b [7/7]`; and row 5 spells
  `  migrate-ai-sdk-v7              ? [-]`
- **AND** in the 60-column buffer each of the three rows is exactly 58 characters, the
  progress cell still ends in the interior's last column, and the badge sits two columns to
  the left of the progress cell's first column: interior column **51** (0-based, the 52nd cell
  of the interior) holds `w` in row 3 and `b` in row 4, whose progress cells are five columns
  wide, and interior column **53** holds `?` in row 5, whose `[-]` cell is three — the badge
  tracks the progress cell rather than occupying a fixed column, exactly as the name field
  already does
- **AND** in each of those three rows the column on either side of the badge is a space
- **AND** at both widths each name field is two columns narrower than in the agentless
  rendering above, and no row's progress cell moved
- **AND** an unrecognised `agent_status` renders `?` rather than being dropped, so a newer
  Herdr never blanks the column

#### Scenario: An unattributed agent badges nothing

- **WHEN** the same three-change dashboard is rendered at 120x20 and at 60x20 carrying one
  in-scope `Working` agent whose `name` is `Some("scratch")` — matching no change — and one
  in-scope `Working` agent whose `name` is `None`
- **THEN** all three rows at both widths are byte-identical to the agentless rendering: no
  row gained a badge cell
- **AND** the same dashboard with the second agent **renamed** to `add-token-refresh` renders
  that row with a `w` badge at both widths, so the unbadged result above is the attribution
  refusing to guess rather than the badge column being unreachable
- **AND** the count those two agents produce is asserted where it is visible — the footer, in
  `responsive-layout`'s "The unattributed count is the footer's last hint at both widths" —
  not here: this scenario is verified by a `ui::list::tests::` test, which renders rows and
  has no footer to read

#### Scenario: A watch problem leads the list, above a change-set problem

- **WHEN** a `Dashboard` with one active change, one `launch.problems` entry
  `herdr pane split exited with code 1: no space to split`, one `refresh.problems` entry
  `filesystem watch unavailable for /r/openspec: No path was found`, and one
  `changes.problems` entry `openspec/changes unreadable: permission denied`, is rendered at
  120x20 and at 60x20
- **THEN** the list interior's row 0 begins `! herdr pane split exited` at both widths, its
  row 1 begins `! filesystem watch unavailable`, its row 2 begins
  `! openspec/changes unreadable`, its row 3 is the active section header `  ▾ active (1)`,
  and its row 4 is the change row — the one place this scenario counts from the interior's
  own first row rather than from the buffer's, as it always has
- **AND** the same dashboard with `launch.problems` emptied renders rows 0, 1, and 2
  byte-identically to the two-problem list this scenario specified before `agent-launch`
  existed, so a pane that has launched nothing gains no row
- **AND** both problem rows carry `RowKind::Problem` and neither carries `selected`
- **AND** each row's text is exactly the interior width in characters — 38 and 58 — padded
  with trailing spaces or truncated with a trailing `…` by the same
  `pad_or_truncate_right` every other row uses, never wrapped onto a second row
- **AND** at 38 columns the watch problem is visibly truncated with `…` while at 58 it is
  longer, so the truncation is a width branch rather than unconditional
- **AND** a problem row never carries a badge cell, at either width, whether or not the
  change below it does: only a change row is badged
- **AND** the same holds for a `Message` row: a dashboard with **no** changes at all and two
  in-scope unattributable agents renders `No changes yet` byte-identically to the agentless
  case at both widths, and a dashboard whose `/` query matches nothing renders its two message
  rows byte-identically too

#### Scenario: The row grammar places the marker, the name, and the progress cell

- **WHEN** `ui::list::rows` is called at widths 38 and 58 for the same three-change
  dashboard at `selected` 1, and the three `RowKind::Item` rows it returns are asserted —
  `rows()[1]`, `[2]` and `[3]`, below the active section header at `[0]`
- **THEN** every returned `Row`'s text is exactly the requested width in characters — 38
  and 58 respectively — so a row is padded rather than short
- **AND** each row is column 0 a selection marker (`>` for the selected change, a space
  otherwise), column 1 a space, then a name field, then one space, then the progress cell
  right-aligned so its final character occupies the interior's last column
- **AND** the progress cell is `[<completed>/<total>]` when `total` is greater than zero
  and the three characters `[-]` when `total` is zero, so `migrate-ai-sdk-v7`'s cell ends
  in the same column as `add-token-refresh`'s
- **AND** when the change carries a badge the same holds with one space, one badge
  character, and one further space inserted between the name field and the progress cell,
  and every returned row is still exactly the requested width

#### Scenario: A name too long for the field is truncated with an ellipsis

- **WHEN** a `Dashboard` whose only active change is
  `a-very-long-change-name-that-will-not-fit-here` — 46 characters — at 2 of 5 tasks is
  rendered at 120x20 and at 60x20, with no agents
- **THEN** the 120-column buffer's row 3, columns 1 through 38, spells exactly
  `  a-very-long-change-name-that-… [2/5]`, so the name was cut to the field width less
  one and an `…` appended, and the progress cell was **not** truncated
- **AND** the 60-column buffer's row 3, columns 1 through 58, spells exactly
  `  a-very-long-change-name-that-will-not-fit-here     [2/5]`, with no `…` anywhere in
  that row, so the truncation at 38 columns is a width branch rather than unconditional
- **AND** rendering the same dashboard again with an in-scope `Working` agent carrying that
  46-character name is impossible — Herdr rejects a name past 32 characters with
  `invalid_agent_name` — so such a change is badged only through the mapping tier, and doing
  so at 38 columns truncates the name two columns earlier, to
  `  a-very-long-change-name-tha… w [2/5]`

#### Scenario: A field too narrow for both drops the progress cell whole

- **WHEN** `ui::list::rows` is called for a dashboard holding one active change named `alpha`
  at 4 of 9 tasks carrying a `Working` badge, with `selected` **1** so the cursor is on the
  change rather than on the active section header above it, at widths 12, 11, 10, 9, 8, 1,
  and 0, and — as the contrasting controls at the mandated interiors — at 38 and 58; every
  assertion below is about the **change** row, `rows()[1]`
- **THEN** at width 12 the row is exactly `> a… w [4/9]` — the name field is
  two columns and the badge still fits; at width 11 it is exactly `> … w [4/9]`
- **AND** at width 10 the badge and its separating space are dropped **whole** and the row is
  exactly `> a… [4/9]`; at width 9 it is exactly `> … [4/9]`; and at width 8 it is exactly
  `> alpha `, the progress cell dropped whole in its turn
- **AND** those three rows are character-for-character the rows the same widths produced
  before the badge existed, so the badge dropping first left every landed boundary in place
- **AND** at width 1 the row is exactly `>` and at width 0 the row is the empty string,
  and neither panics
- **AND** at widths 38 and 58 the row still carries both the badge and the `[4/9]` cell, so
  each drop is a width branch
- **AND** the same holds with a `refresh.problems` entry present: its row degrades by the
  same `pad_or_truncate_right` rule at widths 1 and 0, and neither panics

### Requirement: The list region names every empty and degraded body state

When `dashboard.repo` is `None` the list region's interior SHALL hold exactly three rows
and no change rows: `No OpenSpec repository found`, then `searched from:`, then
`dashboard.searched_from`'s display path shortened to the interior width by the same
keep-the-tail rule the header uses — whole when it fits, otherwise `…` followed by the
longest suffix ending on a grapheme-cluster boundary that measures at most *width − 1*
**display columns**, then padded back to exactly `width` columns. This is `SPEC.md` →
Degraded states, row "No `openspec/` found while walking up".

The two functions that implement that rule are `ui::list::shorten_left` — un-padded, shared
with `responsive-layout`'s header, which right-aligns it within its own remaining space — and
`ui::list::shorten_left_row`, which is `shorten_left` padded to the full row. Both SHALL
measure and shorten in display columns.

When `dashboard.repo` is `Some`, the interior SHALL hold:

- exactly one `Message` row reading `No changes yet` when **both section counts are zero**
  and the filter query is empty;
- exactly one `Message` row reading `No changes match` followed by one row holding `/` and
  the query when both section counts are zero and the query is non-empty;
- one `Message` row reading `No active changes` in place of the active rows when the
  **active** section's count is zero and the archived section's is greater than zero,
  followed by the archived section header and — when that section is open — its rows;
  `SPEC.md` → Degraded states, row "No active changes: empty state; archived changes remain
  browsable".

Each state is keyed on the section **counts** `change-rows`' emission requirement defines,
never on how many rows are drawn. That distinction is `list-sections`' correction and it is
load-bearing in both directions. A **collapsed but populated** section contributes no visible
changes, so a visibility-keyed rule would render `  ▸ active (9)` immediately followed by
`No active changes`, and would render `No changes yet` above `  ▸ archived (28)` in a
repository whose only changes are archived — the same dishonesty the cap this change removes
was condemned for. A section whose count is zero emits no header, so no state can show a
header and its own contradiction together.

Every entry of `dashboard.changes.problems` SHALL be rendered as a leading row: `!`, a
space, then the problem text, truncated with `…` to the interior width. This is where the
degraded-states rows that record a reason on `ChangeSet::problems` — an unreadable
`openspec/changes/` or `archive/` — become visible; nothing else in the plugin renders
them.

A `Problem` row's text field is the interior width less its two-column `! ` prefix; a
`Message` row has no prefix and its field is the whole width. Both SHALL be padded with
spaces when short and truncated from the right with `…` when long, by the same rule the
name field uses, and both SHALL follow the same drop-whole rule the change rows do: below
three columns a `Problem` row is `! ` truncated to the first `width` **display columns**, and
at every width — including `0`, `1`, and `2` — every row of every kind, `Section`,
`Problem`, `Section`, and `Message` included, SHALL be **exactly** `width` **display
columns**.

The indexing guarantee that sentence used to carry is narrowed here, deliberately, because
display columns are the honest unit and a `char` index is not one: a caller may take the
first `n` **columns** of a row with `layout::truncate_columns` for any `n <= width` and get a
whole prefix back, but SHALL NOT index a row by `char` or by byte and assume `width` of them
exist. For an all-ASCII row — every row this repository produces today — the two are the same
count, which is why no landed assertion moves.

The no-repository block SHALL replace **every** other row, problem rows included. That is
not a conflict to resolve at render time: `ui::load` produces `changes::empty_set()`
whenever `find_repo` reports `NotFound`, so a `Dashboard` with `repo: None` and a
non-empty `changes.problems` is not a value the composition root can build. Stating the
precedence anyway keeps `rows` total over every `Dashboard` value a test can construct.

`ui::view::render` SHALL still draw the list region — its heading row, its padding row, and
its interior — in every one of these states. No state SHALL replace the frame with an error
screen. The region carries no border as of `pane-chrome`, so what a reader sees framing an
empty state is the repository's own name on the heading row two rows above it.

#### Scenario: No repository names the directory searched, at both widths

- **WHEN** a `Dashboard` whose `repo` is `None`, whose `searched_from` is
  `/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here` — seventy
  characters — and whose `changes` is `changes::empty_set()`, is rendered at 120x20 and at
  60x20
- **THEN** the 120-column buffer's interior rows 2, 3, and 4 at columns 1 through 38 begin
  `No OpenSpec repository found`, `searched from:`, and
  `…os/a-rather-long-repository-name-here` respectively
- **AND** the 60-column buffer's interior rows 2, 3, and 4 at columns 1 through 58 begin
  `No OpenSpec repository found`, `searched from:`, and
  `…kspaces/openspec-demos/a-rather-long-repository-name-here`
- **AND** in both buffers row 0 spells `no repository` from the region's first interior
  column and row 1 is entirely spaces, so the empty state renders below the region's own
  heading and padding rows rather than replacing the frame

#### Scenario: A repository with no changes at all

- **WHEN** a `Dashboard` whose `repo` is `Some("/tmp/demo-repo")`, whose `changes` is
  `changes::empty_set()`, and whose filter query is empty is rendered at 120x20 and at
  60x20
- **THEN** in both buffers the first interior row begins `No changes yet`
- **AND** neither buffer contains `-- archived`, `No active changes`, or `No changes match`

#### Scenario: No active changes with archived ones still browsable

- **WHEN** a `Dashboard` whose `changes.active` is empty, whose `changes.archived` holds
  `add-auth` dated `2026-08-14` at 7 of 7, whose `archived_total` is 1, and whose archived
  section is **open**, is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row begins `No active changes`, the second is
  exactly `  ▾ archived (1)` padded to the interior width, and the third is the `add-auth` row
- **AND** neither buffer contains `No changes yet`, so the two empty states are
  distinguished rather than sharing one message
- **AND** no active section header is emitted, because that section's count is zero

#### Scenario: A collapsed but non-empty archive is not "no changes yet"

- **WHEN** a `Dashboard` whose `changes.active` is empty, whose `changes.archived` is empty,
  whose `archived_total` is 28, and whose archived section is **collapsed**, is rendered at
  120x20 and at 60x20
- **THEN** in both buffers the first interior row begins `No active changes` and the second is
  exactly `  ▸ archived (28)` padded to the interior width
- **AND** neither buffer contains `No changes yet`: the archived section's count is 28, not
  zero, even though it contributes no visible change and no row
- **AND** neither buffer contains `No changes match`, because the query is empty

#### Scenario: A collapsed active section shows its header and no message row

- **WHEN** a `Dashboard` with nine active changes, no archive, and the **active** section
  collapsed is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row is exactly `  ▸ active (9)` padded to the
  interior width
- **AND** neither buffer contains `No active changes`, `No changes yet`, or any change name:
  the message rows are keyed on the section's count of nine, not on its zero visible rows
- **AND** expanding the same section renders `  ▾ active (9)` followed by the nine rows, so
  the absence of a message row is the fold rather than the changes being gone

#### Scenario: Repository-level problems are named above the rows

- **WHEN** a `Dashboard` whose `changes.problems` holds the single entry
  `openspec/changes: Permission denied (os error 13)` and whose `changes.active` holds
  `fix-empty-basket` at 7 of 7 is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer interior row 2 at columns 1 through 38 spells exactly
  `! openspec/changes: Permission denied…` — the two-character `! ` prefix, then the
  problem text's first thirty-five characters, then `…` — and interior row 3 is the
  `fix-empty-basket` row
- **AND** in the 60-column buffer interior row 2 at columns 1 through 58 spells
  `! openspec/changes: Permission denied (os error 13)` followed by spaces, with no `…`, so
  the truncation at 38 columns is a width branch
- **AND** in both buffers the region is still the list region — its heading row above its
  rows — and not an error screen

### Requirement: Rows are confined to the list region

`ui::view::render` SHALL write no cell outside the list region's interior when
drawing rows — its heading row, its padding row, its two gutter columns, and the divider
column beside it included. At `LayoutMode::Wide` the detail region's interior SHALL remain
blank —
every cell a space whose `Style` equals `ratatui::buffer::Cell::default().style()` — and at
`LayoutMode::Narrow` with `route: Route::Detail` no row SHALL be drawn at all, because the
list region is not drawn at that width and route.

A list longer than the interior height SHALL NOT overflow it: rows beyond the height are
not drawn, and `list-selection` governs which slice is shown.

#### Scenario: The detail region stays blank while the list fills

- **WHEN** the three-active-change dashboard is rendered at 120x20
- **THEN** every cell in rows 2 through 18 and columns 42 through 119 is a space whose
  `Style` equals `Cell::default().style()`
- **AND** the same dashboard rendered at 60x20 draws its rows in columns 1 through 58 and
  every cell of column 0 and column 59 in rows 0 through 18 is a space

#### Scenario: The narrow detail route draws no rows

- **WHEN** the three-active-change dashboard, with `route: Route::Detail`, is rendered at
  60x20
- **THEN** the strings `add-token-refresh`, `fix-empty-basket`, and `migrate-ai-sdk-v7`
  appear in no row of the buffer
- **AND** the same dashboard rendered at 120x20 does show all three, because above the
  breakpoint both regions are drawn

#### Scenario: More changes than rows do not overflow the region

- **WHEN** a `Dashboard` holding thirty active changes named `change-00` through
  `change-29`, each at 1 of 2 tasks, with `selected` 0, is rendered at 120x20 and at 60x20
- **THEN** in both buffers exactly seventeen interior rows are drawn — rows 2 through 18 —
  and rows 0, 1, and 19 hold no change name
- **AND** in both buffers the first interior row is the `change-00` row and the seventeenth
  is the `change-16` row, so the region shows the first seventeen and stops

### Requirement: Every cell of the row grammar is measured in display columns

Every field of every row this capability defines — the name field, an archived row's date
field, the badge cell, the progress cell, the `! `-prefixed problem row, the `Message` rows,
and the no-repository block's shortened `searched_from` — SHALL be laid out, padded, and
truncated in **display columns** as `responsive-layout` defines them, never in `char`s and
never in bytes. `ui::list` SHALL reach that measure only through `layout::columns` and
`layout::truncate_columns`.

`ui::list::pad_or_truncate_right(text, width)` — the crate's one right-truncation
implementation, shared with `ui::detail`'s header, tab bar, and problem-line grammar — SHALL
return a string measuring **exactly `width` display columns** in both of its arms:

- when `layout::columns(text) <= width`, `text` followed by `width - layout::columns(text)`
  spaces;
- when it is longer and `width` is at least 1, `layout::truncate_columns(text, width - 1)`,
  then `…`, then **as many further spaces as are needed to reach exactly `width`
  columns**. That trailing pad is new and load-bearing: `truncate_columns` drops a grapheme
  cluster whole, so the prefix measures `width - 1` or `width - 2`, and without the pad a row
  whose name ends in a wide cluster would be one column short of the interior and leave a
  stale cell behind it;
- when `width` is `0`, the empty string.

`ui::list::shorten_left` — the keep-the-tail rule the no-repository block and
`responsive-layout`'s list-region heading share — SHALL likewise keep the longest **suffix ending on a
grapheme-cluster boundary** that measures at most `width - 1` columns, prefixed with `…`. It
SHALL NOT pad, exactly as it does not today; the heading row places it at its own first
column and leaves the remainder blank. `ui::list::shorten_left_row`, the padded form the no-repository block's
third row uses, SHALL pad the result back to exactly `width` display columns — a third
measuring site, named here because it is easy to miss beside its un-padded sibling.

The cell-drop order is unchanged and is now evaluated in columns: the badge cell and its
space go first, then the progress cell and its space, then an archived row's date field and
its space, each when the name field would otherwise fall below **one column**. The progress
cell (`[<completed>/<total>]` or `[-]`) and the badge (one ASCII column) are ASCII by
construction and measure exactly their character counts; the date field is ten ASCII columns.
Only the name field and the message rows can carry non-ASCII content, and only they can
change width under this rule.

The two mandated interior widths SHALL remain **38** and **58**, and every row-grammar test
SHALL continue to assert both. What changes is the unit each assertion is stated in: a row's
length SHALL be asserted as `layout::columns(row.text) == width`, not as
`row.text.chars().count() == width`.

No row SHALL write past the interior's last column at any width, for any change name, in any
repository — including a name holding wide characters, emoji, combining marks, or a
zero-width-joiner sequence.

#### Scenario: A CJK change name stays inside the list region at both mandated widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose
  `changes.active` is the single change `日本語の変更名前です` — ten characters, twenty
  display columns — at 4 of 9 tasks, with no archived changes, no problems, an empty query,
  `selected` 0 and no agents, is rendered at 120x20 and at 60x20
- **THEN** in the 120-column buffer interior row 2 measures exactly 38 columns, spells the
  ten-character name from interior column 2, and ends with `[4/9]` in the interior's last
  five columns
- **AND** in the 120-column buffer the list region's right gutter at column 39, the divider
  `│` at column 40, and every cell of the detail region's heading row are
  exactly what the same dashboard renders with the ASCII name `add-token-refresh` — the
  overwrite the audit measured is gone
- **AND** in the 60-column buffer interior row 2 measures exactly 58 columns and its `[4/9]`
  cell ends in the interior's last column, with the frame's right gutter at column 59 still
  a space

#### Scenario: An emoji change name at 58 columns does not overwrite the border

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
what a name must not overwrite is now the region's right gutter column.

- **WHEN** a `Dashboard` whose single active change is named `emoji-🎉-change` at 4 of 9 is
  rendered at 60x20 and at 120x20, once with no agent and once carrying an in-scope
  `Working` agent for it
- **THEN** in all four buffers the row measures exactly its interior width — 58 and 38 — the
  progress cell ends in the interior's last column, and the frame's right gutter column is
  a space, unchanged from the ASCII-named render
- **AND** in the badged renders the badge sits two columns left of the progress cell's first
  column, on exactly the landed rule, because the badge and progress cells are ASCII and
  their column budgets did not move

#### Scenario: A wide name is truncated whole and padded back to the full width

- **WHEN** `ui::list::rows` is called at widths 38 and 58 for a change named
  `日本語の変更名前です日本語の変更名前です日本語の変更名前です` — thirty characters, sixty
  display columns — at 4 of 9
- **THEN** at each width the row's `layout::columns` is exactly that width
- **AND** the name field ends with `…`, the character before the ellipsis is a whole CJK
  character rather than a cut one, and slicing the drawn name back out of the change's own
  name succeeds
- **AND** where the truncation landed one column short, the shortfall appears as a space
  between the ellipsis and the following separator rather than as a missing cell at the end
  of the row

#### Scenario: Rows are total over adversarial names at every width

- **WHEN** `ui::list::rows` is called at every width from `0` through `130` for a dashboard
  whose active changes are named, in order: a 200-column CJK string; a family emoji joined
  by two zero-width joiners; `e` followed by five combining accents; a lone U+FF9E halfwidth
  katakana sound mark; a name holding a NUL; and the empty string — each at 4 of 9, with an
  archived change carrying a date, and with one badged
- **THEN** no call panics at any width
- **AND** at every width every returned row's `layout::columns` is at most that width, and
  every non-`Section` change row's is exactly that width
- **AND** at widths `0`, `1`, and `2` the rows are empty or a bare marker, and no row's text
  is longer than the width in columns

#### Scenario: The no-repository block shortens its search path by columns

- **WHEN** a `Dashboard` with no repository root whose `searched_from` is
  `/home/dev/workspaces/日本語のディレクトリ名前がとても長い場合の例` — forty-three
  characters and **sixty-five display columns**, chosen so that it exceeds the wider of the
  two interiors (58) and therefore shortens at both, which a shorter wide-character path
  would not — is rendered at 120x20 and at 60x20
- **THEN** in each buffer the three-row no-repository block is drawn, its shortened path row
  measures **exactly** the interior width in columns — 38 and 58 — and begins with `…`
- **AND** at each width the drawn suffix's own `layout::columns` is at most the interior less
  the ellipsis's one column, and slicing it back out of `searched_from` succeeds, so the
  keep-the-tail cut landed on a cluster boundary
- **AND** a `char`-counted shortening would have kept the last 37 and 57 **characters** —
  70 and 110 columns — so the two measures are distinguishable at both widths and the
  scenario discriminates between them
- **AND** neither buffer writes a cell past the list region's interior, and the block is
  still exactly three rows with no badge and no marker

### Requirement: Archived changes sit below their section header and carry their date

An archived change's row SHALL carry, after the marker and its following space, a
**ten-column date field** holding the `YYYY-MM-DD` string the archive directory name was
prefixed with, then one space, then the name field, the badge cell, and the progress cell
exactly as an active row has them. When the change's `Origin::Archived` carries `date: None`
the ten columns SHALL be spaces, so an undated entry's name still begins in the same column
as a dated one's.

An archived change is badged on exactly the same terms as an active one: a change can be
archived while an agent that was working on it is still alive, and `agent-attribution` builds
its `badges` from both tiers of the change list.

An archived row's fields SHALL be dropped **whole**, never cut short, in a fixed order as
the width falls: first the badge cell and its following space, when the name field would
otherwise fall below one column; then the progress cell and its following space, when the
name field would **still** fall below one column; then the ten-column date field and its
following space, on the same condition again; and only then does the row degenerate to the
active grammar's `[marker][space][name field]`, with no progress cell ever offered. Below two columns
the row is `> ` truncated to the first `width` **display columns**. Concretely, for an **unbadged** change named
`add-auth` at 7 of 7 dated `2026-08-14`: at width 20 the row is `> 2026-08-14 … [7/7]`; at 19
it is `> 2026-08-14 add-a…`, the progress cell dropped; at 14 it is `> 2026-08-14 …`; at 13
it is `> add-auth   `, the date dropped; at 3 it is `> …`; at 1 it is `>`; at 0 it is empty.
Every one of those rows SHALL be exactly its width in **display columns**, which for this
all-ASCII worked example is the same row it has always been. The **badged** form of the
same change needs two further columns: at width 22 the row is `> 2026-08-14 … b [7/7]`, and
at width 21 and below the badge is gone and every row above is reproduced unchanged.

Every one of those rows is character-for-character the row the same change produced before
`list-sections`: this requirement changes what sits **above** the archived rows, never the
rows themselves.

The archived rows SHALL be preceded by the archived **section header** the emission-order
requirement above defines — `  ▸ archived (22)` when collapsed, `  ▾ archived (22)` when
open, padded or truncated to exactly the interior width — and SHALL be emitted only when that
section is open. The header SHALL be emitted only when the section's count is greater than
zero, so a repository with nothing archived, or a query that matches no archived change,
shows no dangling header, exactly as it showed no dangling rule. A section row is never
badged and is never a change.

#### Scenario: The section header and archived rows render at both mandated widths

- **WHEN** a `Dashboard` with one active change `fix-empty-basket` at 7 of 7, archived
  changes `add-auth` dated `2026-08-14` at 7 of 7 followed by `legacy-cleanup` with
  `date: None` at 3 of 3, `archived_total` 2, **both** sections open, `selected` **1** — the
  active change, since target 0 is the active section header — and no agents, is rendered at
  120x20 and at 60x20
- **THEN** the 120-column buffer's interior rows read, in order at columns 1 through 38:
  `  ▾ active (1)`, then
  `> fix-empty-basket               [7/7]`, then
  `  ▾ archived (2)`, then
  `  2026-08-14 add-auth            [7/7]`, then
  `             legacy-cleanup      [3/3]` — the two header rows padded with spaces to the
  full interior width
- **AND** the 60-column buffer's interior rows read the same five rows padded to 58 columns,
  the three change rows byte-identical to the ones this dashboard produced before
  `list-sections`
- **AND** in both buffers the undated row's name begins in the same interior column as the
  dated row's, so the absent date is ten spaces rather than a shift
- **AND** neither buffer contains the string `-- archived`

#### Scenario: An archived change carries a badge in the same column as an active one

- **WHEN** the same five-row dashboard is rendered at 120x20 and at 60x20 carrying one
  in-scope `Blocked` agent named `add-auth`
- **THEN** the 120-column buffer's archived `add-auth` row spells exactly
  `  2026-08-14 add-auth          b [7/7]`, and the `legacy-cleanup` row and the active
  `fix-empty-basket` row are byte-identical to the agentless rendering
- **AND** the 60-column buffer's `add-auth` row is exactly 58 characters with interior
  column 51 (0-based) holding `b` and interior columns 50 and 52 holding spaces — the same
  three columns a badged active row uses at that width
- **AND** both section-header rows are byte-identical at both widths to the agentless
  rendering, so no badge column was reserved on a row that cannot carry one

#### Scenario: A query against an unresolved archive counts from `archived_total`

- **WHEN** a `Dashboard` whose `changes.archived` is empty, whose `archived_total` is 28,
  whose archived section is collapsed, and whose active tier holds two changes matching
  nothing, is given a non-empty query of `zzz` and rendered at 120x20 and at 60x20 **before**
  the refresh that query requests has answered
- **THEN** in both buffers the first interior row begins `No changes match`, the second holds
  `/zzz`, and the third is exactly `  ▾ archived (28)` padded to the interior width
- **AND** no row below the header is drawn, and no `! `-prefixed problem row appears
- **AND** rendering the same dashboard once the twenty-eight archived changes have arrived,
  none of which matches `zzz`, emits **no** archived header at all — that section's count is
  then zero — leaving the two message rows alone, so the `(28)` above is the one-cycle
  unresolved window rather than a lasting count

#### Scenario: A collapsed archived section shows its count and no rows

- **WHEN** the same dashboard with the archived section **collapsed** is rendered at 120x20
  and at 60x20
- **THEN** in both buffers interior row 2 is exactly `  ▸ archived (2)` padded to the interior
  width, and the strings `add-auth` and `legacy-cleanup` appear in no row
- **AND** interior rows 0 and 1 — the active header and the `fix-empty-basket` row — are
  byte-identical to the open rendering, so folding one section moves nothing above it
- **AND** the same dashboard whose `changes.archived` is empty while `archived_total` is 22
  renders interior row 2 as exactly `  ▸ archived (22)` padded to the interior width, so an
  unresolved tier's header counts from `archived_total` rather than from the rows it holds

#### Scenario: An expanded but unresolved archived section shows its header alone

- **WHEN** a `Dashboard` whose `changes.archived` is empty, whose `archived_total` is 22, and
  whose archived section is **open** is rendered at 120x20 and at 60x20
- **THEN** in both buffers the archived header row is exactly `  ▾ archived (22)` padded to
  the interior width
- **AND** no row below it is drawn, and neither buffer contains `No changes match`,
  `No active changes`, or any `! `-prefixed problem row: an outstanding refresh is not a
  degraded state
- **AND** the same dashboard with the twenty-two archived changes present renders the header
  identically and twenty-two rows below it, of which the interior's remaining height draws
  as many as fit

#### Scenario: An archived row drops the progress cell, then the date, as the width falls

- **WHEN** `ui::list::rows` is called for a dashboard holding one archived change `add-auth`
  at 7 of 7 dated `2026-08-14` carrying a `Blocked` badge, with no active changes, the
  archived section open, and `selected` **1** so the cursor is on the change rather than on
  the archived section header above it, at widths 22, 21, 20, 19, 14, 13, 3, 1, and 0, and —
  as the contrasting controls at the mandated interiors — at 38 and 58; every assertion below
  is about the **change** row, `rows()[2]` — the active section's count is zero, so this
  dashboard emits the `No active changes` message row at `rows()[0]` and the archived section
  header at `rows()[1]` before it
- **THEN** the rows are exactly `> 2026-08-14 … b [7/7]`, `> 2026-08-14 a… [7/7]`,
  `> 2026-08-14 … [7/7]`, `> 2026-08-14 add-a…`, `> 2026-08-14 …`, `> add-auth   `, `> …`,
  `>`, and the empty string, in that order
- **AND** the rows at widths 20 and below are character-for-character the rows the same
  widths produced for the same change before the badge existed, so the badge dropping first
  left the progress-cell and date-field boundaries exactly where they were
- **AND** each of those rows is exactly the requested width in characters, so no branch
  is off by one — every drop reclaims the separating space as well as the cell
- **AND** at 38 and 58 the row still carries the date field, the badge, and the `[7/7]`
  cell, so each drop is a width branch

#### Scenario: A section header degrades by truncation at every width

- **WHEN** `ui::list::rows` is called for a dashboard whose archived section is collapsed over
  twenty-two archived changes, at widths 17, 16, 5, 1, and 0, and at 38 and 58
- **THEN** the archived header row is exactly `  ▸ archived (22)` at width 17, exactly
  `  ▸ archived (2…` at 16, exactly `  ▸ …` at 5, exactly ` ` at 1, and the empty string at 0
- **AND** every one of those rows measures exactly the requested width in display columns,
  and none panics
- **AND** at 38 and 58 the row is `  ▸ archived (22)` padded with spaces, so the truncation is
  a width branch

#### Scenario: No archived changes means no archived header

- **WHEN** the three-active-change dashboard above, whose `changes.archived` is empty and
  whose `archived_total` is `0`, is rendered at 120x20 and at 60x20
- **THEN** the strings `archived` and `-- archived` appear in no row of either buffer
- **AND** adding a single archived change to the same dashboard, with `archived_total` `1`,
  and rendering again at both widths makes `  ▾ archived (1)` appear, so the absence is the
  emission rule and not the string being unrenderable
- **AND** the same dashboard with an accepted query of `zzz-no-match` shows neither header at
  either width, because both sections' counts are zero and the two `No changes match` message
  rows replace them

