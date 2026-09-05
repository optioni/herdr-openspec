## MODIFIED Requirements

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
