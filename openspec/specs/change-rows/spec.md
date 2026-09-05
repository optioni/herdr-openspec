# change-rows Specification

## Purpose
TBD - created by archiving change list-view. Update Purpose after archive.

## Requirements

### Requirement: The list region's interior holds one row per change, in the order the `ChangeSet` gives

`ui::list::rows(dashboard: &Dashboard, width: u16) -> Vec<Row>` SHALL be a pure total
function of its two arguments. It SHALL perform no filesystem, process, environment,
network, or terminal I/O, SHALL read no clock and no global state, and SHALL never panic
for any `Dashboard` value and any `u16` width, including `0`.

`ui::view::render` SHALL draw the rows it returns into the **interior** of the `Changes`
region — the area inside the block's four borders — starting at the interior's first row
and first column, one `Row` per terminal row, and SHALL draw nothing outside that
interior. The two interiors the mandated frame widths produce are fixed by
`responsive-layout` and are **38 columns by 16 rows** at a 120x20 frame (the wide layout's
`Constraint::Length(40)` list column, less two border columns) and **58 columns by 16
rows** at a 60x20 frame.

Rows SHALL be emitted in this order and no other:

1. one `Problem` row per entry of `dashboard.refresh.problems`, in that vector's order;
2. one `Problem` row per entry of `dashboard.changes.problems`, in that vector's order;
3. the visible **active** changes, in `dashboard.changes.active`'s order, or — when that
   list is empty — the `Message` row or rows the empty-state requirement below names for
   the state the dashboard is in (one row for `No changes yet` and for
   `No active changes`, two for `No changes match` and its query line, three for the
   no-repository block, which replaces the whole list);
4. one `Separator` row, **only** when at least one visible archived change follows it;
5. the visible **archived** changes, in `dashboard.changes.archived`'s order.

Refresh problems precede change-set problems because the two have different lifetimes:
`refresh.problems` holds conditions that outlive a reload — today, a `notify` watcher that
would not start — while `ChangeSet::problems` is re-derived from the tree on every cycle and
`Dashboard::adopt` replaces it wholesale. A standing condition belongs above a transient one.
Both use the identical `! `-prefixed grammar and the identical `RowKind::Problem`, so
`ui::view` styles them the same way and neither is addressable by `selected`.

The `repo.is_none()` early return SHALL be unchanged: a dashboard with no repository starts
no watcher, so it can carry no refresh problem, and the no-repository block stays exactly the
three rows the empty-state requirement names.

`rows` SHALL NOT sort, de-duplicate, or re-order either tier or either problem list:
`changes::from_files` already orders active changes name-ascending in byte order and archived
changes dated-newest-first, and a second ordering rule here would be a second place for the
two producers of `Change` to disagree.

#### Scenario: Active rows render at both mandated widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose
  `changes.active` is, in order, `add-token-refresh` at 4 of 9 tasks, `fix-empty-basket`
  at 7 of 7, and `migrate-ai-sdk-v7` at 0 of 0, with no archived changes, no problems of
  either kind, an empty filter query, and `selected` 0, is rendered into a `TestBackend` at
  120x20 and again at 60x20
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
  `refresh` was added to `Dashboard`, so an empty `refresh.problems` costs no row

#### Scenario: A watch problem leads the list, above a change-set problem

- **WHEN** a `Dashboard` with one active change, one `refresh.problems` entry
  `filesystem watch unavailable for /r/openspec: No path was found`, and one
  `changes.problems` entry `openspec/changes unreadable: permission denied`, is rendered at
  120x20 and at 60x20
- **THEN** the list interior's row 0 begins `! filesystem watch unavailable` at both widths,
  its row 1 begins `! openspec/changes unreadable`, and its row 2 is the change row
- **AND** both problem rows carry `RowKind::Problem` and neither carries `selected`
- **AND** each row's text is exactly the interior width in characters — 38 and 58 — padded
  with trailing spaces or truncated with a trailing `…` by the same
  `pad_or_truncate_right` every other row uses, never wrapped onto a second row
- **AND** at 38 columns the watch problem is visibly truncated with `…` while at 58 it is
  longer, so the truncation is a width branch rather than unconditional

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

#### Scenario: A name too long for the field is truncated with an ellipsis

- **WHEN** a `Dashboard` whose only active change is
  `a-very-long-change-name-that-will-not-fit-here` — 46 characters — at 2 of 5 tasks is
  rendered at 120x20 and at 60x20
- **THEN** the 120-column buffer's row 2, columns 1 through 38, spells exactly
  `  a-very-long-change-name-that-… [2/5]`, so the name was cut to the field width less
  one and an `…` appended, and the progress cell was **not** truncated
- **AND** the 60-column buffer's row 2, columns 1 through 58, spells exactly
  `  a-very-long-change-name-that-will-not-fit-here     [2/5]`, with no `…` anywhere in
  that row, so the truncation at 38 columns is a width branch rather than unconditional

#### Scenario: A field too narrow for both drops the progress cell whole

- **WHEN** `ui::list::rows` is called for a single selected active change named `alpha` at
  4 of 9 tasks, at widths 10, 9, 8, 1, and 0, and — as the contrasting controls at the
  mandated interiors — at 38 and 58
- **THEN** at width 10 the row is exactly `> a… [4/9]`; at width 9 it is exactly
  `> … [4/9]`; and at width 8 it is exactly `> alpha ` — the progress cell is dropped
  **whole**, never cut short, as soon as the name field would fall below one column
- **AND** at width 1 the row is exactly `>` and at width 0 the row is the empty string,
  and neither panics
- **AND** at widths 38 and 58 the row still carries the `[4/9]` cell, so the drop is a
  width branch
- **AND** the same holds with a `refresh.problems` entry present: its row degrades by the
  same `pad_or_truncate_right` rule at widths 1 and 0, and neither panics

### Requirement: Archived changes sit below a separator and carry their date

An archived change's row SHALL carry, after the marker and its following space, a
**ten-column date field** holding the `YYYY-MM-DD` string the archive directory name was
prefixed with, then one space, then the name field and the progress cell exactly as an
active row has them. When the change's `Origin::Archived` carries `date: None` the ten
columns SHALL be spaces, so an undated entry's name still begins in the same column as a
dated one's.

An archived row's fields SHALL be dropped **whole**, in a fixed order, as the width
falls: first the progress cell, when the name field would otherwise fall below one
column; then the ten-column date field and its following space, when the name field
would *still* fall below one column; and only then does the row degenerate to the active
grammar's `[marker][space][name field]`. Below two columns the row is the first `width`
characters of `> `. Concretely, for a change named `add-auth` at 7 of 7 dated
`2026-08-14`: at width 20 the row is `> 2026-08-14 … [7/7]`; at 19 it is
`> 2026-08-14 add-a…`, the progress cell dropped; at 14 it is `> 2026-08-14 …`; at 13 it
is `> add-auth   `, the date dropped; at 3 it is `> …`; at 1 it is `>`; at 0 it is empty.
Every one of those rows SHALL be exactly its width in characters.

The separator row SHALL be two spaces, then the literal `-- archived `, then `-`
characters filling the interior to its full width; when the interior is narrower than the
fourteen characters of `  -- archived ` it SHALL be that prefix truncated to the width.
The separator SHALL be emitted only when at least one archived row follows it, so a
repository with no archived changes — or a filter that matches none — shows no dangling
rule.

#### Scenario: The separator and archived rows render at both mandated widths

- **WHEN** a `Dashboard` with one active change `fix-empty-basket` at 7 of 7, and archived
  changes `add-auth` dated `2026-08-14` at 7 of 7 followed by `legacy-cleanup` with
  `date: None` at 3 of 3, is rendered at 120x20 and at 60x20
- **THEN** the 120-column buffer's interior rows read, in order at columns 1 through 38:
  `> fix-empty-basket               [7/7]`, then
  `  -- archived ------------------------`, then
  `  2026-08-14 add-auth            [7/7]`, then
  `             legacy-cleanup      [3/3]`
- **AND** the 60-column buffer's interior rows read, in order at columns 1 through 58:
  `> fix-empty-basket                                   [7/7]`, then
  `  -- archived --------------------------------------------`, then
  `  2026-08-14 add-auth                                [7/7]`, then
  `             legacy-cleanup                          [3/3]`
- **AND** in both buffers the undated row's name begins in the same interior column as the
  dated row's, so the absent date is ten spaces rather than a shift

#### Scenario: An archived row drops the progress cell, then the date, as the width falls

- **WHEN** `ui::list::rows` is called for a single selected archived change `add-auth` at
  7 of 7 dated `2026-08-14`, at widths 20, 19, 14, 13, 3, 1, and 0, and — as the
  contrasting controls at the mandated interiors — at 38 and 58
- **THEN** the rows are exactly `> 2026-08-14 … [7/7]`, `> 2026-08-14 add-a…`,
  `> 2026-08-14 …`, `> add-auth   `, `> …`, `>`, and the empty string, in that order
- **AND** each of those rows is exactly the requested width in characters, so no branch
  is off by one — the drop-the-progress branch reclaims the separating space as well as
  the cell
- **AND** at 38 and 58 the row still carries both the date field and the `[7/7]` cell, so
  each drop is a width branch

#### Scenario: No archived changes means no separator

- **WHEN** the three-active-change dashboard above, whose `changes.archived` is empty, is
  rendered at 120x20 and at 60x20
- **THEN** the string `-- archived` appears in no row of either buffer
- **AND** adding a single archived change to the same dashboard and rendering again at both
  widths makes it appear, so the absence is the emission rule and not the string being
  unrenderable

### Requirement: The list region names every empty and degraded body state

When `dashboard.repo` is `None` the list region's interior SHALL hold exactly three rows
and no change rows: `No OpenSpec repository found`, then `searched from:`, then
`dashboard.searched_from`'s display path shortened to the interior width by the same
keep-the-tail rule the header uses — whole when it fits, otherwise `…` followed by its last
*width − 1* characters. This is `SPEC.md` → Degraded states, row "No `openspec/` found
while walking up".

When `dashboard.repo` is `Some`, the interior SHALL hold:

- exactly one `Message` row reading `No changes yet` when no change is visible and the
  filter query is empty;
- exactly one `Message` row reading `No changes match` followed by one row holding `/` and
  the query when no change is visible and the query is non-empty;
- one `Message` row reading `No active changes` in place of the active rows when no active
  change is visible but at least one archived change is, followed by the separator and the
  archived rows — `SPEC.md` → Degraded states, row "No active changes: empty state;
  archived changes remain browsable".

Every entry of `dashboard.changes.problems` SHALL be rendered as a leading row: `!`, a
space, then the problem text, truncated with `…` to the interior width. This is where the
degraded-states rows that record a reason on `ChangeSet::problems` — an unreadable
`openspec/changes/` or `archive/` — become visible; nothing else in the plugin renders
them.

A `Problem` row's text field is the interior width less its two-column `! ` prefix; a
`Message` row has no prefix and its field is the whole width. Both SHALL be padded with
spaces when short and truncated from the right with `…` when long, by the same rule the
name field uses, and both SHALL follow the same drop-whole rule the change rows do: below
three columns a `Problem` row is the first `width` characters of `! `, and at every width
— including `0`, `1`, and `2` — every row of every kind SHALL be **exactly** `width`
characters, so a caller may index them without a bounds check.

The no-repository block SHALL replace **every** other row, problem rows included. That is
not a conflict to resolve at render time: `ui::load` produces `changes::empty_set()`
whenever `find_repo` reports `NotFound`, so a `Dashboard` with `repo: None` and a
non-empty `changes.problems` is not a value the composition root can build. Stating the
precedence anyway keeps `rows` total over every `Dashboard` value a test can construct.

`ui::view::render` SHALL still draw a bordered `Changes` region in every one of these
states. No state SHALL replace the frame with an error screen.

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
- **AND** in both buffers row 1 still holds `┌` at the region's first column and the title
  `Changes`, so the empty state renders inside the frame rather than replacing it

#### Scenario: A repository with no changes at all

- **WHEN** a `Dashboard` whose `repo` is `Some("/tmp/demo-repo")`, whose `changes` is
  `changes::empty_set()`, and whose filter query is empty is rendered at 120x20 and at
  60x20
- **THEN** in both buffers the first interior row begins `No changes yet`
- **AND** neither buffer contains `-- archived`, `No active changes`, or `No changes match`

#### Scenario: No active changes with archived ones still browsable

- **WHEN** a `Dashboard` whose `changes.active` is empty and whose `changes.archived` holds
  `add-auth` dated `2026-08-14` at 7 of 7 is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row begins `No active changes`, the second is
  the archived separator, and the third is the `add-auth` row
- **AND** neither buffer contains `No changes yet`, so the two empty states are
  distinguished rather than sharing one message

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
- **AND** in both buffers the region is still the bordered `Changes` block, not an error
  screen

### Requirement: Rows are confined to the list region

`ui::view::render` SHALL write no cell outside the `Changes` region's interior when
drawing rows. At `LayoutMode::Wide` the `Detail` region's interior SHALL remain blank —
every cell a space whose `Style` equals `ratatui::buffer::Cell::default().style()` — and at
`LayoutMode::Narrow` with `route: Route::Detail` no row SHALL be drawn at all, because the
list region is not drawn at that width and route.

A list longer than the interior height SHALL NOT overflow it: rows beyond the height are
not drawn, and `list-selection` governs which slice is shown.

#### Scenario: The detail region stays blank while the list fills

- **WHEN** the three-active-change dashboard is rendered at 120x20
- **THEN** every cell in rows 2 through 17 and columns 41 through 118 is a space whose
  `Style` equals `Cell::default().style()`
- **AND** the same dashboard rendered at 60x20 draws its rows in columns 1 through 58 and
  no cell of column 0 or column 59 in rows 2 through 17 is anything but a border character

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
- **THEN** in both buffers exactly sixteen interior rows are drawn — rows 2 through 17 —
  and rows 0, 1, 18, and 19 hold no change name
- **AND** in both buffers the first interior row is the `change-00` row and the sixteenth
  is the `change-15` row, so the region shows the first sixteen and stops
