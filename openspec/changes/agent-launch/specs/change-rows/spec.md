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
4. the visible **active** changes, in `dashboard.changes.active`'s order, or — when that
   list is empty — the `Message` row or rows the empty-state requirement below names for
   the state the dashboard is in (one row for `No changes yet` and for
   `No active changes`, two for `No changes match` and its query line, three for the
   no-repository block, which replaces the whole list);
5. one `Separator` row, **only** when at least one visible archived change follows it;
6. the visible **archived** changes, in `dashboard.changes.archived`'s order.

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

A badge cell SHALL be drawn on a **change row only**. A `Problem`, `Separator`, or `Message`
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
- **AND** in both buffers every cell of interior rows 5 through 17 is a space, so exactly
  three rows were drawn and nothing was repeated into the remaining height
- **AND** the buffers are byte-identical to the ones the same dashboard produced before
  `refresh` was added to `Dashboard`, and byte-identical to the ones it produced before
  `agent-attribution` added the badge cell: an empty `refresh.problems` costs no row, and an
  empty `badges` costs no column

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
