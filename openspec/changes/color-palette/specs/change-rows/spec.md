## ADDED Requirements

### Requirement: A `Row` carries its badge cell's column, and the view colours that one cell

`ui::list::Row` SHALL gain one field naming where its agent badge sits, so `ui::view` can
paint that single column without re-deriving the row grammar's arithmetic:

```rust
pub struct BadgeCell {
    pub x: u16,
    pub status: crate::agents::AgentStatus,
}

pub struct Row {
    pub text: String,
    pub kind: RowKind,
    pub selected: bool,
    pub badge: Option<BadgeCell>,
}
```

`badge` SHALL be `Some` exactly when the row's `text` carries a badge cell — a change row
whose name is in `Dashboard::attribution().badges` and whose width was wide enough that the
badge cell was not dropped — and `None` in every other case, including every `Problem`,
`Separator`, and `Message` row, at any width and whatever `badges` holds.

`BadgeCell::x` SHALL be the badge character's **column offset from the interior's first
column**, so `text[x]` is that character: `name_field_width + 3` on an active row
(`[marker][space][name field][space][badge]`) and `name_field_width + 14` on an archived one
(`[marker][space][date field: 10][space][name field][space][badge]`). `BadgeCell::status`
SHALL be the `agents::AgentStatus` the badge character was derived from, carried rather than
re-parsed from the glyph.

Adding this field SHALL move **no cell**. The `text` of every row, badged or not, at every
width, SHALL be byte-identical to the `text` the same `Dashboard` produced before this
change: `BadgeCell` reports where the grammar already put the badge and never decides where
it goes. `Row` SHALL carry no `ratatui` type, so the row grammar stays plain data on exactly
`change-rows`' landed terms.

`ui::view::render` SHALL draw a badged row by writing the whole row with the row's own style
first, then re-writing the single character at `interior.x + badge.x` with that same style
**patched** by `palette::style(Role::AgentBadge(badge.status))`. The badge cell therefore
keeps every modifier its row carries — a badge on the selected row is bold **and** coloured —
and gains only the status colour. When `badge.x` is not less than the interior's width the
cell SHALL be skipped rather than clamped, so no badge is ever drawn over a border.

#### Scenario: A badged active row reports the column its badge occupies, at both mandated widths

- **WHEN** `ui::list::rows` is called for a `Dashboard` whose repository root is
  `/tmp/demo-repo`, whose `changes.active` holds `add-token-refresh` at 4 of 9 tasks with a
  `Working` badge and `fix-empty-basket` at 7 of 7 with no badge, at width `38` and again at
  width `58`
- **THEN** the first row's `badge` is `Some(BadgeCell { x, status: Working })` and
  the character occupying display column `x` of `row.text` is `w` at both widths
- **AND** the second row's `badge` is `None`
- **AND** both rows' `text` values are byte-identical to the values the same dashboard
  produced before this change

#### Scenario: A badged archived row reports the column its badge occupies

- **WHEN** `ui::list::rows` is called for a `Dashboard` whose only change is the archived
  `add-auth`, dated `2026-08-14`, at 7 of 7 tasks with a `Blocked` badge, at width `38` and
  again at width `58`
- **THEN** the row's `badge` is `Some(BadgeCell { x, status: Blocked })` and the character at
  `x` is `b` at both widths
- **AND** `x` is one column further right than the name field's **last** column — that is,
  `name_field_width + 14` from the interior's first column — and equals the badge column an
  **active** row of the same width reports, so the ten-column date field is accounted for
  without the two grammars disagreeing

#### Scenario: A dropped badge cell reports no badge

- **WHEN** `ui::list::rows` is called for a `Dashboard` holding one active change named
  `demo` at 4 of 9 tasks carrying a `Working` badge, at widths `12`, `11`, `10`, `9`, `1`,
  and `0`
- **THEN** at widths `12` and `11`, where the badge cell still fits, `badge` is `Some` and
  the character at its `x` is `w`
- **AND** at widths `10`, `9`, `1`, and `0`, where the badge cell is dropped whole, `badge`
  is `None`, so the field never points at a column the row does not have

#### Scenario: No non-change row carries a badge

- **WHEN** `ui::list::rows` is called for a `Dashboard` carrying one launch problem, one
  refresh problem, one archived change producing a separator, and a filter query matching
  nothing — and separately for a `Dashboard` with no repository — at widths `38` and `58`
- **THEN** every returned row whose `kind` is `Problem`, `Separator`, or `Message` has
  `badge: None`
- **AND** no `Message` row of the no-repository block carries a badge, whatever
  `attribution().badges` holds

#### Scenario: The badge cell reaches the buffer coloured and the rest of the row does not

- **WHEN** a `Dashboard` with three active changes — the selected first one badged `Working`,
  the second badged `Blocked`, the third unbadged — is rendered at 120x20 and at 60x20
- **THEN** in both buffers the cell holding `w` reports the foreground
  `Role::AgentBadge(Working)` carries (`Color::Green`) **and**
  `Modifier::BOLD`, because it sits on the selected row
- **AND** the cell holding `b` reports the foreground `Role::AgentBadge(Blocked)` carries
  (`Color::LightRed`) and no `BOLD`
- **AND** the cells on either side of each badge — the two separating spaces — report no
  foreground at all, so exactly one column was painted

### Requirement: A problem row is drawn in the palette's problem colour

`ui::view::render` SHALL draw every row whose `kind` is `RowKind::Problem` with
`palette::style(Role::ListProblem)` — foreground `Color::Red`, no modifier — and every row
whose `kind` is `RowKind::Separator` with `palette::style(Role::ListSeparator)` — foreground
`Color::DarkGray`, no modifier.

This is the one distinction the landed styling could not draw: a `!`-marked problem row is
styled exactly like the change rows around it, and "something is wrong" is the signal in this
pane most worth picking out of a frame. All three problem sources — launch, refresh, and
change-set — SHALL be drawn identically, exactly as `change-rows` already requires of their
grammar and their `RowKind`.

No problem or separator row SHALL become selectable, and neither SHALL gain a modifier: the
colour is added beside a style that carried none.

#### Scenario: Problem rows are red and change rows are not, at both mandated widths

- **WHEN** a `Dashboard` carrying one launch problem, one refresh problem, one change-set
  problem, three active changes, and one archived change is rendered at 120x20 and at 60x20
- **THEN** in both buffers every cell of the three `!`-prefixed rows reports the foreground
  `Role::ListProblem` carries (`Color::Red`) and no modifier
- **AND** every cell of the separator row reports the foreground `Role::ListSeparator`
  carries (`Color::DarkGray`) and no modifier
- **AND** the change rows below report no foreground at all, and the selected one still
  reports `Modifier::BOLD`, so the colour discriminates the problem rows rather than tinting
  the region

#### Scenario: An empty-state message row is not a problem row

- **WHEN** a `Dashboard` over `changes::empty_set()` with a repository root is rendered at
  120x20 and at 60x20, and a second whose filter query matches nothing is rendered at the
  same two sizes
- **THEN** the `No changes yet`, `No changes match`, and `/`-query rows report no foreground
  at all in any buffer
- **AND** the three rows of the no-repository block likewise report none, so an empty pane is
  not painted as a broken one
