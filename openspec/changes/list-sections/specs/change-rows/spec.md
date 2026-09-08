## MODIFIED Requirements

### Requirement: The list region's interior holds one row per change, in the order the `ChangeSet` gives

`ui::list::rows(dashboard: &Dashboard, width: u16) -> Vec<Row>` SHALL be a pure total
function of its two arguments. It SHALL perform no filesystem, process, environment,
network, or terminal I/O, SHALL read no clock and no global state, and SHALL never panic
for any `Dashboard` value and any `u16` width, including `0`. It reaches the agent badge
through `Dashboard::attribution()`, which `agent-attribution` requires to be pure and derived
per call; that adds no I/O and no clock to this function.

`ui::view::render` SHALL draw the rows it returns into the **interior** of the `Changes`
region — the area inside the block's four borders — starting at the interior's first row
and first column, one `Row` per terminal row, and SHALL draw nothing outside that
interior. The two interiors the mandated frame widths produce are fixed by
`responsive-layout` and are **38 columns by 16 rows** at a 120x20 frame (the wide layout's
`Constraint::Length(40)` list column, less two border columns) and **58 columns by 16
rows** at a 60x20 frame.

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
display columns at every width including `0`. The `glyph` is `v` when the section is open and
`>` when it is collapsed; the `label` is the literal `active` or `archived`; the count is the
decimal count in parentheses. A section header at 38 columns therefore reads
`  v active (9)` followed by twenty-five spaces, and `  > archived (22)` followed by
twenty-one. No cell is dropped whole and no field is right-aligned: unlike a change row the
header is one label, and truncating it with `…` is the whole of its degradation.

The `>` glyph and the `>` selection marker are the same character in different columns —
a selected collapsed section reads `> > archived (22)`. That collision is accepted rather
than overlooked: column `0` is the cursor on **every** row of this list and column `2` is the
fold state on section rows alone, so neither is ambiguous once the grammar is read, and the
alternative glyph pair `+`/`-` was rejected only because `proposal.md` names `v` and `>`.

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

#### Scenario: Active rows render at both mandated widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose
  `changes.active` is, in order, `add-token-refresh` at 4 of 9 tasks, `fix-empty-basket`
  at 7 of 7, and `migrate-ai-sdk-v7` at 0 of 0, with no archived changes, no problems of
  either kind, an empty filter query, `selected` 0, and **no agents**, is rendered into a
  `TestBackend` at 120x20 and again at 60x20
- **THEN** in the 120-column buffer the 38 cells of row 2, columns 1 through 38, spell
  exactly `> add-token-refresh              [4/9]`; row 3 spells
  `  fix-empty-basket               [7/7]`; and row 4 spells
  `  migrate-ai-sdk-v7                [-]`
- **AND** in the 60-column buffer the 58 cells of row 2, columns 1 through 58, spell
  exactly `> add-token-refresh                                  [4/9]`; row 3 spells
  `  fix-empty-basket                                   [7/7]`; and row 4 spells
  `  migrate-ai-sdk-v7                                    [-]`
- **AND** in both buffers interior row 1 is the active section header — exactly
  `  v active (3)` padded to the interior width — so the three change rows begin at interior
  row 2
- **AND** in both buffers every cell of interior rows 5 through 17 is a space, so exactly
  four rows were drawn and nothing was repeated into the remaining height
- **AND** the three change rows are byte-identical to the ones the same dashboard produced
  before `list-sections` existed, and byte-identical to the ones it produced before
  `agent-attribution` added the badge cell: an empty `refresh.problems` costs no row, an
  empty `badges` costs no column, and the section header is the one row this change adds
- **AND** the same dashboard with the active section **collapsed** renders interior row 1 as
  exactly `  > active (3)` padded to the interior width, with no change name anywhere in
  either buffer, so the header's count is the tier's size and not the number of rows drawn

#### Scenario: A badged row carries its status between the name and the progress cell

- **WHEN** the same three-change dashboard is rendered at 120x20 and again at 60x20, this
  time carrying an in-scope `Working` agent named `add-token-refresh`, an in-scope `Blocked`
  agent named `fix-empty-basket`, and an in-scope agent named `migrate-ai-sdk-v7` whose status
  is `AgentStatus::Unknown` — what `agent-list` decodes an unrecognised or absent
  `agent_status` string to
- **THEN** in the 120-column buffer the 38 cells of row 2 spell exactly
  `> add-token-refresh            w [4/9]`; row 3 spells
  `  fix-empty-basket             b [7/7]`; and row 4 spells
  `  migrate-ai-sdk-v7              ? [-]`
- **AND** in the 60-column buffer each of the three rows is exactly 58 characters, the
  progress cell still ends in the interior's last column, and the badge sits two columns to
  the left of the progress cell's first column: interior column **51** (0-based, the 52nd cell
  of the interior) holds `w` in row 2 and `b` in row 3, whose progress cells are five columns
  wide, and interior column **53** holds `?` in row 4, whose `[-]` cell is three — the badge
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
  `! openspec/changes unreadable`, and its row 3 is the change row
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
  dashboard
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
- **THEN** the 120-column buffer's row 2, columns 1 through 38, spells exactly
  `  a-very-long-change-name-that-… [2/5]`, so the name was cut to the field width less
  one and an `…` appended, and the progress cell was **not** truncated
- **AND** the 60-column buffer's row 2, columns 1 through 58, spells exactly
  `  a-very-long-change-name-that-will-not-fit-here     [2/5]`, with no `…` anywhere in
  that row, so the truncation at 38 columns is a width branch rather than unconditional
- **AND** rendering the same dashboard again with an in-scope `Working` agent carrying that
  46-character name is impossible — Herdr rejects a name past 32 characters with
  `invalid_agent_name` — so such a change is badged only through the mapping tier, and doing
  so at 38 columns truncates the name two columns earlier, to
  `  a-very-long-change-name-tha… w [2/5]`

#### Scenario: A field too narrow for both drops the progress cell whole

- **WHEN** `ui::list::rows` is called for a single selected active change named `alpha` at
  4 of 9 tasks carrying a `Working` badge, at widths 12, 11, 10, 9, 8, 1, and 0, and — as the
  contrasting controls at the mandated interiors — at 38 and 58
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


## REMOVED Requirements

### Requirement: Archived changes sit below a separator and carry their date

**Reason**: The single, unaddressable `-- archived ----` separator becomes a foldable section
header carrying a glyph, a label, and a count. The requirement's title, its separator grammar,
and its emission rule are all restated by the ADDED requirement below; the archived row's own
grammar — the ten-column date field, the badge cell, the progress cell, and the fixed order in
which they drop as the width falls — is carried across unchanged, character for character.

**Migration**: None for a reader. The rows themselves are byte-identical; the row above them
changes from `  -- archived --------` to `  > archived (22)` or `  v archived (22)`, and
`Space` now folds it. `RowKind::Separator` is gone from the crate's public row vocabulary,
replaced by `RowKind::Section { key, depth, collapsed }`; every match on it is a compile
error rather than a silent fallthrough, since the enum is matched exhaustively at each site.

## ADDED Requirements

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
requirement above defines — `  > archived (22)` when collapsed, `  v archived (22)` when
open, padded or truncated to exactly the interior width — and SHALL be emitted only when that
section is open. The header SHALL be emitted only when the section's count is greater than
zero, so a repository with nothing archived, or a query that matches no archived change,
shows no dangling header, exactly as it showed no dangling rule. A section row is never
badged and is never a change.

#### Scenario: The section header and archived rows render at both mandated widths

- **WHEN** a `Dashboard` with one active change `fix-empty-basket` at 7 of 7, archived
  changes `add-auth` dated `2026-08-14` at 7 of 7 followed by `legacy-cleanup` with
  `date: None` at 3 of 3, `archived_total` 2, **both** sections open, and no agents, is
  rendered at 120x20 and at 60x20
- **THEN** the 120-column buffer's interior rows read, in order at columns 1 through 38:
  `  v active (1)`, then
  `> fix-empty-basket               [7/7]`, then
  `  v archived (2)`, then
  `  2026-08-14 add-auth            [7/7]`, then
  `             legacy-cleanup      [3/3]` — the two header rows padded with spaces to the
  full interior width
- **AND** the 60-column buffer's interior rows read the same five rows padded to 58 columns,
  the three change rows byte-identical to the ones this dashboard produced before
  `list-sections`
- **AND** in both buffers the undated row's name begins in the same interior column as the
  dated row's, so the absent date is ten spaces rather than a shift
- **AND** neither buffer contains the string `-- archived`

#### Scenario: A collapsed archived section shows its count and no rows

- **WHEN** the same dashboard with the archived section **collapsed** is rendered at 120x20
  and at 60x20
- **THEN** in both buffers interior row 2 is exactly `  > archived (2)` padded to the interior
  width, and the strings `add-auth` and `legacy-cleanup` appear in no row
- **AND** interior rows 0 and 1 — the active header and the `fix-empty-basket` row — are
  byte-identical to the open rendering, so folding one section moves nothing above it
- **AND** the same dashboard whose `changes.archived` is empty while `archived_total` is 22
  renders interior row 2 as exactly `  > archived (22)` padded to the interior width, so an
  unresolved tier's header counts from `archived_total` rather than from the rows it holds

#### Scenario: An expanded but unresolved archived section shows its header alone

- **WHEN** a `Dashboard` whose `changes.archived` is empty, whose `archived_total` is 22, and
  whose archived section is **open** is rendered at 120x20 and at 60x20
- **THEN** in both buffers the archived header row is exactly `  v archived (22)` padded to
  the interior width
- **AND** no row below it is drawn, and neither buffer contains `No changes match`,
  `No active changes`, or any `! `-prefixed problem row: an outstanding refresh is not a
  degraded state
- **AND** the same dashboard with the twenty-two archived changes present renders the header
  identically and twenty-two rows below it, of which the interior's remaining height draws
  as many as fit

#### Scenario: An archived row drops the progress cell, then the date, as the width falls

- **WHEN** `ui::list::rows` is called for a single selected archived change `add-auth` at
  7 of 7 dated `2026-08-14` carrying a `Blocked` badge, at widths 22, 21, 20, 19, 14, 13, 3,
  1, and 0, and — as the contrasting controls at the mandated interiors — at 38 and 58
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
- **THEN** the archived header row is exactly `  > archived (22)` at width 17, exactly
  `  > archived (2…` at 16, exactly `  > …` at 5, exactly ` ` at 1, and the empty string at 0
- **AND** every one of those rows measures exactly the requested width in display columns,
  and none panics
- **AND** at 38 and 58 the row is `  > archived (22)` padded with spaces, so the truncation is
  a width branch

#### Scenario: No archived changes means no archived header

- **WHEN** the three-active-change dashboard above, whose `changes.archived` is empty and
  whose `archived_total` is `0`, is rendered at 120x20 and at 60x20
- **THEN** the strings `archived` and `-- archived` appear in no row of either buffer
- **AND** adding a single archived change to the same dashboard, with `archived_total` `1`,
  and rendering again at both widths makes `  v archived (1)` appear, so the absence is the
  emission rule and not the string being unrenderable
- **AND** the same dashboard with an accepted query of `zzz-no-match` shows neither header at
  either width, because both sections' counts are zero and the two `No changes match` message
  rows replace them
