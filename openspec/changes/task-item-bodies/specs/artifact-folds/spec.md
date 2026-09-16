## MODIFIED Requirements

### Requirement: A section header row names the file and shows its fold state

`ui::detail::content_lines` SHALL, when the selected artifact is foldable, walk
`detail.sections` in order and emit, for each section that is **visible**:

- when its `label` is `Some(label)`, one **header row** reading
  `<indent><glyph> <badge><label>`, where `badge` is present when and only when that section's
  `operation` is `Some(op)` and is then exactly two columns — the marker `+` for `Added`, `~`
  for `Modified`, `-` for `Removed`, followed by one space. A section whose `operation` is
  `None` SHALL emit no badge and reserve no column for one, so every row this change does not
  badge is byte-identical to the row it was, followed — when and only when that section's `progress` is
  `Some` — by right-aligned padding and the group's own **progress cell**, which SHALL be
  `ui::list::progress_cell(progress)` and SHALL NOT be a second formatting of the same pair.
  That is the crate's one progress cell, on exactly the terms `tasks-progress-bar`'s bar and
  `detail-header`'s header row already reach it, and drawing it here is what lets a reader
  fold a completed group without losing how far along it was. The cell SHALL be **dropped
  whole** when the row cannot hold the indent, the glyph, the separating space, at least one
  column of label, one separating space, and the cell itself — never truncated, and never
  allowed to push the label out. Here `indent` is two spaces per unit of the section's own
  `depth` and the glyph pair is **the one the list region's `active` and `archived` headers
  use**, so one fold reads the same in both regions. That pair is `▸` collapsed and `▾` open,
  and SHALL live at exactly one site — `ui::list::fold_glyph(collapsed: bool) -> char`,
  `pub(crate)`, called by both `ui::list`'s `section_row_text` and this capability's header
  row — and this capability SHALL take it from there rather than writing its own literal.
  `section_row_text` is private, so the extraction is what makes "one site" reachable at all;
  it joins `progress_cell`, `pad_or_truncate_right`, and `shorten_left` as helpers
  `ui::detail` already calls across that boundary. **The extraction is what proves
  agreement**, and no separate agreement test is required: both sites call the one function,
  so they cannot disagree, and the pair is pinned against its literals by `ui::list`'s own
  glyph test — a changed `fold_glyph` fails there. An assertion comparing the two function
  results would be tautological, and this repository does not keep tests that cannot fail.
  What a test SHALL NOT do is read the glyph out of `ui::list::rows`' drawn output at a known
  offset, which would depend on row-grammar layout rather than on the shared site. The whole row is passed through
  `ui::list::pad_or_truncate_right` at `width`; followed by
- when and only when the section is open — or its `label` is `None`, which is always open and
  never foldable — that section's body: `ui::markdown::lines(&section.text, body_width)`, or
  `tasks-checklist`'s group grammar when the selected artifact carries
  `tracks_tasks == true`, where `body_width` is `width - indent_cols` under the indent rule
  below and `width` itself whenever that rule draws at column zero; followed by
- one **blank row** when that body produced at least one row and a further visible section
  follows, so an open section's content is separated from the next header rather than running
  into it.

A tracked-tasks section's body is that section's items **and its retained content**, not its
items alone. `ui::detail::content_lines` SHALL obtain it by calling
`ui::tasks::group_body(group, body_width)` for each group `tasks::parse(&section.text)`
returned, in document order, so each item's own body rows and each group's blocks are drawn at
the positions `task-groups` records for them — at the same `body_width` the indent rule below
decides for every other body, never at `width` when that rule is indenting. It SHALL NOT
flatten the parse to a single item slice: flattening is what discarded every body and every
block, and a section whose text is one heading's body parses to one group, so the loop is one
iteration in the shape every real `tasks.md` produces and is written as a loop only so a
preamble holding more than one group cannot silently lose its second.

The rows this adds are **body rows**, not sections: they carry `ContentKind::Body` exactly as
every other body row does, are never fold targets, never carry a `SectionHeader` kind, and are
hidden by their own section's fold along with the item rows beside them. They are indented by
their section's `depth` on exactly the terms every other body row is, the indent rule below
making one decision for the whole call. This change adds no section, no header row, and no
fold level — an item's body folds with the section that holds it and with nothing finer.

A section is **visible** exactly when no preceding collapsed labelled section has smaller
`depth` and covers it: walking the list in order, a collapsed labelled section at depth `d`
hides every following section of depth strictly greater than `d`, header and body alike,
until the first section of depth at or below `d`. Indentation is therefore never the only
signal of nesting — a fold hides a whole subtree, which is what makes a two-level spec tab
navigable at the 58-column interior.

Body rows SHALL be indented by their section's own `depth`, using the same `"  " * depth`
unit the header row carries, **when and only when the tab's content width can afford the
deepest such indent**. An indented body row is `indent` spaces followed by the row
`ui::markdown::lines` — or `tasks-checklist`'s grammar — produced at `width - indent_cols`,
so the body wraps inside the column it is drawn in rather than being prefixed and overflowing.

The decision SHALL be made **once per `content_lines` call**, from the content width and
`max_depth` — the greatest `depth` among the sections of `detail.sections` whose own `text` is
non-empty — and SHALL apply to every body row of that call or to none. `max_depth` SHALL be
read from `detail.sections` and never from the visible or expanded set: a tab's text column
must not widen as a deep section is collapsed and narrow again as it is opened, which would
make `Space` reflow the prose of every sibling that stayed open. The indent is a property of
the tab, not of the cursor's fold history. A per-section decision would indent a depth-1 body while leaving the
depth-3 body beside it at column zero, which reads worse than either extreme; one decision per
render is what makes the tab's left edge either consistently ragged or consistently aligned.

The floor SHALL be that the deepest indented body retains at least **64** display columns of
interior: body rows are indented when `width.saturating_sub(2 * max_depth) >= 64` and are
drawn at column zero otherwise. The subtraction SHALL saturate rather than wrap or panic —
`width` is a `u16` and `depth` a `usize`, and the width sweeps this capability already runs
from `0` reach every width below `2 * max_depth`. It follows from the floor that an indented
body's own `width - indent_cols` is at least 64 and therefore never zero, so the width-0 empty
path of `markdown-render`'s wrapping is unreachable under this rule. The constant is a measured trade rather than a derived one, and its derivation
is stated so a later reader can re-run it — the archive's deepest spec section is depth 3,
costing 6 columns, which leaves 72 columns at the 78-column wide interior and 52 at the
58-column narrow one. The floor is placed between those two so the wide layout gains the
alignment and the narrow layout keeps the text column.

That narrow-layout text column is the thing the previous rule protected, and the protection is
kept rather than overruled: indenting bodies would reduce the text column of exactly the
artifacts this capability exists to make readable, which remains true and remains the reason
the floor exists at all. What changed is the conclusion drawn from it — a cost the 58-column
interior cannot bear is not a cost the 78-column interior must also refuse. `markdown-render`'s
wrapping being parameterised by a width is what makes the narrower width free to pass.

A section at `depth` 0 SHALL be drawn at column zero whether or not the floor is met, its
indent being zero columns.

A **tracked-tasks** tab SHALL be indented on exactly these terms and SHALL have no exemption
of its own: `tasks-checklist`'s rows — its items, their own body rows, and its group blocks
alike — are a section's body like any other, and a tab whose groups sit at a non-zero depth
indents them. A tracked-tasks tab's sections are **not**
always at depth 0 — `depth` is `base + (level - min_level)`, `base` is 1 whenever the artifact
resolves to more than one path, and a task file that opens with a level-1 title puts every
`## ` group at depth 1 — so a rule resting on "every tracked-tasks section is at depth 0" would
be false for much of this repository's own history. One rule for every tab is what keeps the
fold grammar single, and it is stated here because the opposite was previously assumed.

The **progress-bar** rows `tasks-progress-bar` draws above every header SHALL NOT be indented:
they are leading body owned by no section, have no header to align beneath, and are already
padded to the full width. The **blank separator row** between sections SHALL NOT be indented
either, for the same padding reason — it is already exactly `width` blank columns, and
prefixing it would make it the one row that exceeds the region.

A `None`-labelled **preamble** section SHALL be indented by its own `depth` like any other
body. That depth is `base`, which is not necessarily 0: for a glob artifact the file section
is depth 0 and the preamble beneath it is depth 1, so the preamble's body sits flush under its
own file's header.

No **header** row's indent, glyph, badge, progress cell, or drop-whole order SHALL change, and
no body row SHALL become a fold target or acquire a `SectionHeader` kind.

This rule moves body rows sideways and, wherever a body wraps, produces **more** rows than it
did — the body is re-wrapped at the narrower `width - indent_cols`, not merely prefixed. Every
consumer of the row list re-derives it from the same `content_lines` call at the same width, so
the drawn frame, the scroll clamp, and the pointer resolvers cannot disagree about either the
indent or the row count.

The indent is rendered text, so `text-selection` copies it: a selection covering an indented
body row yields that row's leading spaces, exactly as a selection covering a **header** row
already yields the header's own `"  " * depth`. That is the consistent reading and SHALL be the
behaviour — the indent is not trailing padding, which that capability drops, but rendered
content, which it keeps. It follows that a double click inside the indent's own columns selects
nothing, those cells holding only whitespace, which `text-selection` already requires of any
whitespace cell.

When the selected artifact is **not** foldable, `content_lines` SHALL emit no header row and
no blank separator at all, and SHALL render the single section's `text` exactly as it
rendered it before this change.

Each header row SHALL carry `ContentKind::SectionHeader { section, selected }`, where
`section` is its own index into `detail.sections` — not its position among the drawn rows —
and `selected` is true for exactly the header whose section the cursor is on or in.
`artifact-content` states that shape and `view-palette` states how `ui::view` turns it into a
`Style`: `Role::DetailSectionSelected` for the selected header, `Role::DetailSection` for
every other. `ui::detail` SHALL name neither role and no `ratatui` type — the header carries
a *kind*, not a style, on exactly the terms `ui::list::RowKind` already does.

No other row SHALL be restyled by the cursor: the cursor's feedback is which header is
emphasised, not a highlighted line running through prose. When the cursor addresses a problem
row — every problem row precedes every section — no header SHALL be `selected`.

A test asserting the **unselected** style SHALL assert the row's `kind`, not only its painted
cells. `Role::DetailSection` is plain `BOLD`, which `Role::Strong` and five other roles also
are, so a cell comparison alone would pass for a `**bold**` body span and could not fail if
the header lost its role entirely. The **selected** style is `BOLD | REVERSED`, which
`view-palette` requires to equal no other role's, so a cell comparison there is
discriminating and SHALL be asserted that way.

The progress cell SHALL be faced exactly as the rest of its header row is — the row carries
one `ContentKind::SectionHeader`, not a second kind for the cell — so a selected group's cell
is reversed along with its label and `view-palette` gains no role for it. A **completed**
group's cell SHALL NOT be styled differently from an incomplete one: the numbers already say
which is which, and a second signal there would be the per-group `kind` badge this change
deliberately declined.

Every header row SHALL measure at most `width` display columns at **every** width, measured
through `ui::layout::columns`, on exactly the terms `artifact-content` states for every
other line `content_lines` returns. A label too long for the width SHALL be truncated with
the same `…` rule every other row uses, and the indent, the glyph, and the separating space
SHALL be emitted before the label so that the depth and the fold state survive any
truncation. At a width below the indent's own columns the row degrades to truncated indent
rather than to a dropped glyph, and SHALL NOT panic. The drop-whole order as the row narrows
SHALL therefore be: the progress cell first, reclaiming its own separating padding; then the
label, truncated with the `…` rule; then the **badge**; then the glyph and the indent, in that
order — the same drop-whole discipline `tasks-progress-bar`'s bar and `detail-header`'s header
row already use, so a narrowing pane loses fields in one order everywhere.

The badge SHALL be emitted **with** the indent, the glyph and the separating space — before
the label — and SHALL therefore survive any truncation the label needs. It is fixed-width and
binary where the label degrades gracefully: a label cut to `Requirement: The tab bar is bui…`
still says what it is, while half a badge says nothing. It is dropped only below the width at
which the row can hold the indent, the glyph, its separating space, the badge, and at least
one column of label.

A badged header row SHALL carry **four** segments rather than one — the
`<indent><glyph> ` prefix, the badge, the label, and the blank columns that pad the row to its
width — so that the badge, the label and the padding may be faced apart. An unbadged header row
SHALL carry the one segment it carried before this change, **plus the progress cell's own
segment when it draws one** — five and two respectively on a row that also carries a cell,
since `header` appends the cell after deciding the badge.

That last clause is a correction, not a refinement: an earlier wording of this requirement said
an unbadged row carries "the one segment it carried before this change" full stop, which is
false for a tracked-tasks group header. Before this change `header` returned a single `String`
with the cell already formatted into it, so such a row was one segment; it is now two. **No
frame changes** — both segments are `Face::plain()`, so the row's text and every cell's style
are byte-identical either way — but the count is observable to a test, and the scenario below
that asserts "exactly one segment" reaches only rows with `progress: None`, so nothing caught
the overstatement. Found by the Change Review.

The padding is a segment of its own, and plain-faced, because the label's face reaches every
column of its own segment: a `Removed` label padded inside its own segment strikes the blank
columns after it, and a terminal draws that as a continuous rule from the word to the region's
edge rather than as a struck heading. `view-palette` -> "A monochrome reading of the frame is
unchanged" fixes the same fact from the other side — `CROSSED_OUT` on that requirement's label
cells "and on no other cell in the frame" — and the two cannot both hold at three segments. The
split is taken at `ui::list::truncate_right`, which returns `pad_or_truncate_right`'s two halves
separately and is the function `pad_or_truncate_right` is now written in terms of, so the
truncation rule stays written down once.
`ui::detail` SHALL name no `palette::Role` here either: the badge segment carries
`Face { delta: Some(op), .. }` and nothing else, exactly as the row carries a *kind* and not a
style. `view-palette` decides what each `DeltaOp` looks like, and `ui::view::style_for`
patches the row's own `ContentKind::SectionHeader` role over it — which is why the badge keeps
its colour on a selected header, `Role::DetailSectionSelected` carrying no foreground of its
own.

A row whose section carries **both** `operation: Some(op)` and `progress: Some(p)` SHALL draw
both: the badge in the prefix, after the glyph, and the progress cell right-aligned, with the
drop-whole order above deciding which yields first as the row narrows. The combination is
reachable — a tracked-tasks file that quotes `## ADDED Requirements` and `### Requirement: A` is
spec-shaped by `has_requirement_heading` *and* splits as a tracked-tasks file — though no such
file exists in this repository today
(`grep -rl '^### Requirement:' openspec/changes/*/tasks.md openspec/changes/archive/*/tasks.md`
returns nothing). It is specified for the reason `view-palette` gives for the unreachable
`muted` + `label` pair: totality is the contract, not the absence of a caller.

A `Removed` section's **label** segment SHALL additionally carry `Face { strikethrough: true,
.. }`, and its body SHALL NOT. The strike goes **with** the badge when the badge is dropped:
below the width that holds one, the row falls back to the pre-change single segment, which
carries no strike either. That is deliberate and follows from what the fallback means — the row
reverts to what it drew before this capability existed, face included, rather than keeping half
of a grammar whose marker is gone. A strike with no `-` beside it would say "deleted" with
nothing to say it about. Striking the heading is what marks the requirement as deleted;
striking hundreds of lines of body beneath it would make unreadable exactly the text a reader
opened the section to read. The strikethrough SHALL be the existing `Face` field and
`Role::Strikethrough` the existing role — this change adds neither — so a struck heading and a
`~~struck~~` markdown span are rendered by one mechanism.

#### Scenario: Folding one section shows its body and leaves its siblings shut

- **WHEN** the three-spec dashboard has `detail.expanded` set to hold `1` and is rendered at
  120x20 and at 60x20
- **THEN** the content area's first row reads `> degraded-coverage`, its second reads
  `v markdown-render`, and the rows below the second carry that file's rendered markdown
- **AND** the row following that file's last rendered line is blank, and the row after it
  reads `> tasks-checklist`
- **AND** at both widths every drawn row measures exactly the content area's width in
  display columns

#### Scenario: A fold hides a whole subtree

- **WHEN** the seven-section spec-glob dashboard above has `detail.expanded` holding `0` — the
  `degraded-coverage` file section — and is rendered at 120x40 and at 60x40
- **THEN** the content area's rows are `v degraded-coverage`, `  > ADDED Requirements`,
  `> markdown-render`, and `> tasks-checklist`, in that order
- **AND** with `expanded` holding `0` and `1` the rows become `v degraded-coverage`,
  `  v ADDED Requirements`, `    > Requirement: Alpha`, `    > Requirement: Beta`,
  `> markdown-render`, and `> tasks-checklist` — the scenario header stays hidden under its
  collapsed requirement, two levels below an open ancestor
- **AND** with `expanded` holding `1` alone, the `ADDED Requirements` header is not drawn at
  all, because its parent is collapsed

#### Scenario: The cursor's section header is the emphasised one

- **WHEN** the three-spec dashboard with every section collapsed and `detail.scroll` of `1`
  is rendered at 120x20 and at 60x20
- **THEN** the second header row's cells carry `palette::style(Role::DetailSectionSelected)`,
  which is the discriminating half, and `content_lines` reports `selected: true` on that row's
  `kind` and `false` on the first and third — the half that can fail when the role is lost
- **AND** with `detail.expanded` holding `1` and `detail.scroll` moved to a line inside that
  open section's body, the second header row still carries the selected style, because the
  cursor is *in* that section
- **AND** with a `detail.problems` of one entry and `detail.scroll` of `0` — addressing the
  problem row — no header row carries the selected style

#### Scenario: A narrow pane truncates the label and keeps the glyph

- **WHEN** the three-spec dashboard is rendered in the detail route at frame widths of 20
  and 15 — both below the 100-column breakpoint, with content areas of 18 and 13 columns
- **THEN** the first header row reads `> degraded-covera…` at 18 columns and
  `> degraded-c…` at 13
- **AND** a depth-2 header at those widths reads `    > Requirement…` and `    > Requir…`, so
  the four-column indent is emitted before the glyph and survives
- **AND** at every width from 0 through 20 no returned line exceeds that width in display
  columns, and none panics
- **AND** a section whose label is CJK is truncated in **columns**, so its header measures at
  most the content width even though its `chars().count()` is smaller

#### Scenario: A tracked-tasks tab's group headers carry their own progress

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true` and whose one path reads
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n`
  is synced and rendered at 120x20 and at 60x20
- **THEN** `detail.sections` holds two sections whose `progress` values are
  `Some(Progress { completed: 1, total: 2 })` and `Some(Progress { completed: 0, total: 1 })`
- **AND** at each width the `1. Setup` header row ends with `[1/2]` and the `2. Build` header
  row with `[0/1]`, each right-aligned against the content area's own last column
- **AND** both cells are byte-identical to `ui::list::progress_cell` called on those two
  values, so the row does not format its own
- **AND** with `detail.expanded` cleared both cells are still drawn, which is the point: a
  folded group still says how far along it is

#### Scenario: Every other artifact's section headers carry no progress cell

- **WHEN** the three-spec dashboard — a `specs` glob resolving to three files, `tracks_tasks`
  false — is synced and rendered at 120x20 and at 60x20
- **THEN** every section's `progress` is `None`
- **AND** every header row is byte-identical to the row this capability drew before this
  change, at both widths
- **AND** the same holds for the **file section** and the **preamble** of a split
  tracked-tasks file whose artifact resolved to more than one path: both carry
  `progress: None`, so only heading sections gain a cell

#### Scenario: The progress cell is dropped whole rather than truncated

- **WHEN** the two-group tracked-tasks dashboard above is rendered at every content-area width
  from `0` through `40`
- **THEN** at every width the header row measures at most that width in display columns and
  nothing panics
- **AND** at every width the row either holds the whole cell `[1/2]` or holds no `[`, no `]`,
  and no `/` at all — no partial cell is ever drawn
- **AND** there is a width in that range at which the cell is present and one at which it is
  absent, so the drop is exercised rather than assumed
- **AND** wherever the cell is dropped, at least one column of the label survives if the width
  admits one, so the cell yields to the label rather than the other way round

#### Scenario: A group holding no items still gets a header and a counted cell

- **WHEN** a tracked-tasks file reading
  `## 1. Notes\n\nprose only\n\n## 2. Build\n\n- [ ] 2.1 third\n` is synced and rendered at
  120x20
- **THEN** `1. Notes` is a section whose `progress` is
  `Some(Progress { completed: 0, total: 0 })`
- **AND** its header row ends with `[-]`, `ui::list::progress_cell`'s own form for a zero
  total
- **AND** `2. Build`'s row ends with `[0/1]`, so a prose group and an unstarted group are
  distinguishable on the header row alone

#### Scenario: The three operations draw three different markers

- **WHEN** `content_lines` renders, at the mandated 78-column detail interior and again at 58,
  a foldable `specs` tab whose sections are a file section and three collapsed requirement
  headers carrying `Some(Added)`, `Some(Modified)`, and `Some(Removed)`
- **THEN** at both widths the three header rows read `  ▸ + Requirement: …`,
  `  ▸ ~ Requirement: …`, and `  ▸ - Requirement: …`, the indent being the two columns of
  their depth
- **AND** the badge segment of each carries `Face { delta: Some(op), .. }` with `Added`,
  `Modified`, and `Removed` respectively, and the three faces are asserted to differ
- **AND** each row's fourth segment is its padding, plain-faced, so no row's badge or label face
  reaches the blank columns that fill the row to its width

#### Scenario: An unbadged header row is unchanged in every column

- **WHEN** `content_lines` renders, at 78 columns and again at 58, a foldable tab whose
  sections all carry `operation: None` — the file sections of a `design.md`, and the headings
  of a main spec under `## Requirements`
- **THEN** every header row is byte-identical to the row the same input produced before this
  change, with no badge and no reserved badge column
- **AND** each such row carries exactly one segment — these sections carry `progress: None`, so
  no progress-cell segment is appended either — and the four-segment shape is reached only by a
  badged row

#### Scenario: A removed requirement's heading is struck and its body is not

- **WHEN** `content_lines` renders, at 78 columns and again at 58, an **open** requirement
  section carrying `Some(Removed)` whose text is a paragraph and a `#### Scenario:` heading
- **THEN** the header row's label segment carries `Face { strikethrough: true, .. }` and its
  badge segment reads `- `
- **AND** that segment's text is the label alone, the row's padding being a separate plain-faced
  segment, so the strike ends with the word rather than running to the region's edge
- **AND** no body row carries `strikethrough`, so the removed requirement stays readable
- **AND** the same section carrying `Some(Added)` produces a label segment with
  `strikethrough` false, so the strike is the operation's and not every badged header's

#### Scenario: The label truncates before the badge is dropped

- **WHEN** `content_lines` renders a requirement header carrying `Some(Modified)` whose label
  is 200 characters, at 78 columns and again at 58
- **THEN** at both widths the row measures at most `width` display columns through
  `ui::layout::columns`, the badge `~ ` is present, and the label is truncated with the `…`
  rule
- **AND** the row still begins with its indent and glyph, so the badge joined the prefix that
  survives truncation rather than the label that does not

#### Scenario: The badge is dropped whole at a width that cannot hold it

- **WHEN** that same header row is rendered at every width from `0` through `20` inclusive
- **THEN** no width panics and every row measures at most that width
- **AND** there is a width at or below which the row carries no badge segment at all, and at
  every width above it the badge is present in full — the marker and its space together, never
  the marker alone

#### Scenario: A selected badged header keeps its badge colour

- **WHEN** the cursor is on a requirement section carrying `Some(Added)` and the frame is drawn
  at 78 columns and again at 58
- **THEN** the header row's kind is `ContentKind::SectionHeader { selected: true, .. }` and its
  cells carry `Modifier::REVERSED`
- **AND** the badge cell's foreground equals `palette::style(Role::DeltaAdded)`'s, because
  `Role::DetailSectionSelected` carries no foreground of its own to displace it

#### Scenario: A row carrying both a badge and a progress cell drops them in the stated order

- **WHEN** `header` renders a section carrying both `operation: Some(Added)` and
  `progress: Some(1/2)`, at depth 0 and again at depth 1, at **every** width from 0 through 40
- **THEN** at every width the row measures exactly that many columns, the totality
  `header_is_total_from_zero_through_twenty_columns` already asserts for the single-field row
- **AND** the badge's presence is **monotonic** in width: once a width draws the badge, every
  wider width draws it too, and likewise for the progress cell
- **AND** the progress cell yields **before** the badge — there is no width at which the cell is
  drawn and the badge is not, because the drop-whole order above puts the cell first
- **AND** the reservation is what makes that true: `label_area` counts the badge's own columns
  when `operation` is `Some`, so it cannot keep the cell at a width where `badged_pieces` then
  refuses. Without it the badge is drawn at 5–8 columns, absent at 9–10, and drawn again at 11 —
  measured, and the reason this scenario exists

#### Scenario: A badged header row is still addressed by its own section index

- **WHEN** a foldable `specs` tab holds a file section and three badged requirement sections,
  and `Space` is pressed with the cursor on the second requirement header at 78 columns and
  again at 58
- **THEN** that section's index toggles in `detail.expanded` and no other section's does
- **AND** the badge changes nothing about `section_at`'s lookup, the row still carrying
  `ContentKind::SectionHeader { section, .. }` with its own index into `detail.sections`

#### Scenario: A spec tab's bodies align under their headers at the wide interior

- **WHEN** a foldable `specs` **glob resolving to more than one file** — so the file section
  is depth 0 and `base` is 1 — whose first file holds `## MODIFIED Requirements`, a
  `### Requirement: One module` beneath it, and a `#### Scenario: The palette answers` beneath
  that — depths 1, 2, and 3 — is rendered with every section open at a content width of `78`
- **THEN** the scenario's body rows each begin with exactly six spaces, flush beneath the
  `#### Scenario:` header row's own indent
- **AND** the requirement's body rows begin with exactly four spaces and the operation
  heading's with two, each matching its own header
- **AND** no body row's text exceeds `78` display columns, the body having been wrapped at
  `78 - 6` rather than wrapped at `78` and then prefixed

#### Scenario: The same tab draws its bodies at column zero at the narrow interior

- **WHEN** the same tab is rendered at a content width of `58`
- **THEN** every body row begins at column zero, carrying no indent at any depth
- **AND** every header row keeps its own `"  " * depth` indent, unchanged from what it drew
  before this change
- **AND** the body rows are wrapped at the full `58` columns, so the narrow layout's text
  column is exactly what it was

#### Scenario: The indent is all-or-nothing across one render

- **WHEN** a foldable tab holding sections at depths 1, 2, and 3 is rendered at every content
  width from `0` through `120`
- **THEN** at each width either every body row is indented to its own depth or every body row
  is at column zero, and never a mixture
- **AND** the transition happens at exactly one width, `70`, which is `64 + 2 * 3` for that
  tab's maximum depth of 3
- **AND** collapsing the depth-3 section leaves every still-open body row's indent and wrap
  width unchanged, so the decision reads `detail.sections` and not the visible set — an
  implementation reading `visible_sections` passes every other scenario here
- **AND** no call panics and no row's text exceeds its width, including at every width below
  `2 * max_depth`, where the floor's subtraction saturates rather than panicking

#### Scenario: A shallower tab indents at a narrower width

- **WHEN** a foldable tab whose deepest section with a body is at depth `1` is rendered at
  content widths `67`, `66`, and `65`
- **THEN** its body rows are indented at `67` and `66` and are at column zero at `65`, the
  floor being `64 + 2 * 1 = 66`
- **AND** the same three widths leave a depth-3 tab's bodies at column zero throughout, its own
  floor being `70`, so the rule genuinely reads the tab's maximum depth rather than a fixed
  width

#### Scenario: A depth-0 tracked-tasks tab is unmoved at every width

- **WHEN** a foldable tracked-tasks tab whose sections are all at depth 0 — one resolved path,
  every group heading at one level — is rendered at content widths `120`, `78`, `58`, and `20`
- **THEN** every row it draws begins at the column it began at before the indent floor
  existed — the tab is unmoved **horizontally**, whatever rows its items' bodies and its
  groups' blocks add to it
- **AND** no body row gains an indent, depth 0 costing zero columns whether the floor is met
  or not
- **AND** that is a consequence of its depth alone and not of any tracked-tasks exemption,
  which the scenario below fixes from the other side

#### Scenario: A depth-1 tracked-tasks tab indents its items like any other tab

- **WHEN** a foldable tracked-tasks tab whose groups sit at depth 1 — the shape a task file
  that opens with a level-1 title produces, `min_level` being 1 — is rendered at the mandated
  `78`-column interior and again at `58`
- **THEN** at `78` every row of a group's body — item rows, their own body rows, and the
  group's blocks alike — begins with exactly two spaces and is wrapped at `76`, its floor
  being `64 + 2 * 1 = 66`
- **AND** at `58` every one of those rows begins at column zero and is wrapped at `58`,
  `58 - 2` being below the floor
- **AND** the group header rows keep their own `"  " * depth` indent at both widths, unchanged
- **AND** the progress-bar rows above every header are at column zero at both widths, owned by
  no section

#### Scenario: A selection over an indented body row copies the indent

- **WHEN** a depth-3 body row is drawn at a content width of `78`, where the floor is met, and
  again at `58`, where it is not, and the whole row is selected
- **THEN** at `78` the copied text carries the row's six leading spaces, the indent being
  rendered content rather than the trailing padding `text-selection` drops
- **AND** at `58` the copied text carries none, there being no indent to copy
- **AND** at `78` a double click inside the indent's own first six columns selects nothing,
  those cells holding only whitespace

#### Scenario: An open tracked-tasks section draws item bodies and group blocks

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true`, and whose one path reads a `## 1. Setup` heading, a
  `<!-- kind: behavior -->` line, a blank line, `- [ ] 1.1 RED: write the test`, and a
  continuation line indented six columns reading `covering the degraded path`, is synced and
  rendered at 120x20 and at 60x20 with the section open — its one path putting every section
  at depth 0, so the indent rule costs it no columns at either width
- **THEN** the content area holds a row whose text is `<!-- kind: behavior -->`, drawn at the
  section body's own column zero as that group's block
- **AND** it holds a row whose text, with the hanging indent stripped, is
  `covering the degraded path`, drawn beneath the item row as that item's body
- **AND** every one of those rows carries `ContentKind::Body`, none carries
  `ContentKind::SectionHeader`, and `section_at` resolves none of them to a section

#### Scenario: Collapsing a tracked-tasks section hides its bodies and blocks with its items

- **WHEN** the dashboard above is rendered at 120x20 and at 60x20 with `detail.expanded`
  cleared
- **THEN** the content area holds the progress-bar row, a blank row, and `> 1. Setup`, and no
  other row
- **AND** neither the block row nor the item's body row is drawn, both being hidden by the
  same fold that hides the item row
- **AND** no fold glyph is drawn on any item row, an item's body folding with its section
  rather than with a control of its own

### Requirement: `Space` toggles the artifact section the cursor is on or in

`Dashboard::apply(Action::ToggleSection)` at `Route::Detail` SHALL fold or unfold exactly one
artifact section: the one the detail cursor is **on or in**. `list-selection` states the
route split that sends the action here rather than to the list region, and states the list
route's own arm.

The section the cursor is in SHALL be derived from `ui::detail::content_lines`' own line
indices: the section whose header row is the last one drawn at or before `detail.scroll`.
Because a collapsed section's descendants are not drawn at all, that rule names the innermost
**visible** section containing the cursor, which is the one the reader sees emphasised. Every
problem row precedes every section and belongs to none, and so does every row of a
`None`-labelled preamble section and, on the tracked-tasks tab, the progress-bar row and its
blank line. So:

- when `detail.scroll` addresses a line at or after some header row, that header's section is
  toggled — its index removed from `detail.expanded` if present, inserted otherwise;
- when `detail.scroll` addresses a row before the first header row, when the selected artifact
  is not foldable, or when `detail.sections` is empty, `apply` SHALL change nothing at all and
  SHALL record no problem.

Toggling a section SHALL change only that section's own membership of `detail.expanded`. A
collapsed ancestor hides its descendants at render time rather than by clearing their
membership, so opening an ancestor restores exactly the fold state its subtree had — which is
what makes closing a requirement to look at its sibling and reopening it a cheap move rather
than a lossy one.

After a toggle, `detail.scroll` SHALL be set to the **header row index** of the section that
was toggled, recomputed against the line list the fold just produced. Collapsing a section
the cursor was inside would otherwise leave the cursor addressing lines that no longer exist,
and the per-frame clamp alone would land it somewhere unrelated; moving it to the header is
both the predictable answer and the position from which the next `Space` reopens the section.
This is `list-selection`'s rule for the list region, applied to the same key in the other
one.

`apply` SHALL reach no collaborator, spawn no process, touch no filesystem, and read no clock
while handling `ToggleSection` at either route. In particular it SHALL NOT re-read any
artifact and SHALL NOT re-split any file: a fold changes which lines are rendered from text
already in `detail.sections`, and `sync_detail` is not involved.

Opening a section SHALL NOT itself set `refresh.requested`. Unlike the archived list section,
whose rows may not be resolved yet, every section's `text` was read when the tab was, so a
fold needs no data.

The blanket rule `list-selection` states — `Dashboard::apply` sets `refresh.requested` after
**any** action when the archived tier needs resolving — is the stated exception, and is not a
detail-route toggle doing anything. It runs after every action alike, its condition is about
the *list*'s archived tier and never about a fold, and exempting it here is what keeps this
clause true of the code rather than of the fixtures that happen to leave that condition
false.

#### Scenario: `Space` opens the section under the cursor and leaves its siblings shut

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to three spec
  files, with `detail.expanded` empty and `detail.scroll` `1`, is given a `ToggleSection`
  action
- **THEN** `detail.expanded` holds exactly `1`, and `detail.scroll` is still `1` — the
  header row index of the section that opened, which did not move because the sections above
  it did not change height
- **AND** rendering at 120x40 and at 60x40 shows `> degraded-coverage`, then
  `v markdown-render`, then that file's rendered markdown, then a blank row, then
  `> tasks-checklist`
- **AND** `sections.collapsed`, `selected`, `refresh.requested`, and every other field of the
  `Dashboard` are unchanged

#### Scenario: `Space` inside an open section folds it and moves the cursor to its header

- **WHEN** the same dashboard with `detail.expanded` holding `0` and `detail.scroll` set to a
  line inside that open section's body is given a `ToggleSection` action
- **THEN** `detail.expanded` is empty and `detail.scroll` is `0`, the folded section's header
  row
- **AND** rendering at 120x40 and at 60x40 shows exactly three header rows, all collapsed
- **AND** with `detail.expanded` holding `0` and `detail.scroll` addressing the **third**
  header row — whose index depends on how many lines the open first section contributed — the
  same action opens the third section and leaves the first open, so the section acted on is
  the one the cursor is in and not a fixed one

#### Scenario: Closing an ancestor preserves its subtree's folds

- **WHEN** the seven-section spec-glob dashboard with `detail.expanded` holding `0`, `1`, and
  `2` has the cursor moved to the `degraded-coverage` header row and is given two
  `ToggleSection` actions
- **THEN** after the first `detail.expanded` holds `1` and `2` — the descendants kept their
  membership — and the content area shows `> degraded-coverage`, `> markdown-render`, and
  `> tasks-checklist` alone
- **AND** after the second the rendered rows are byte-identical to what they were before the
  two actions, at 120x40 and at 60x40

#### Scenario: `Space` on a problem row is inert

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to three spec
  files, one of which the reader failed on, and whose `detail.scroll` is `0` — the problem
  row — is given ten `ToggleSection` actions
- **THEN** the `Dashboard` is equal, field for field, to what it was before the ten
- **AND** none panics and no further problem is recorded
- **AND** moving `detail.scroll` to `1` and repeating the action toggles the first section,
  so the inertness was attributable to the row and not to the presence of a problem

#### Scenario: `Space` on a preamble row is inert

- **WHEN** the three-section task dashboard whose first entry is a `None`-labelled preamble is
  given ten `ToggleSection` actions with `detail.scroll` addressing a preamble row — which,
  on that tab, are the progress-bar row, its blank line, and the row `Intro prose.` itself now
  draws: a `None`-labelled section's body on a tracked-tasks tab goes through
  `ui::tasks::group_body`, which draws that section's blocks and item bodies beside its items,
  so the preamble's prose is its leading group's position-0 block and occupies a row of its own.
  What makes the action inert is unchanged and is the point of the scenario: a body row is not
  a fold target whatever drew it
- **THEN** the `Dashboard` is equal, field for field, to what it was before the ten
- **AND** moving `detail.scroll` onto the `1. Setup` header and repeating the action toggles
  that section

#### Scenario: `Space` is inert on a non-foldable artifact

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to one path
  holding prose is given ten `ToggleSection` actions, and a second whose artifact resolves to
  none is given ten more
- **THEN** both `Dashboard` values are equal, field for field, to what they were before
- **AND** neither spawns a process, touches the filesystem, nor calls the artifact reader

#### Scenario: A fold reads no file

- **WHEN** `run_loop` is driven over a `TestBackend` at 120x40 with a recording reader, a
  dashboard whose selected artifact resolves to three spec files, and an event script of
  `Enter`, four `Char(' ')` presses, and `Char('q')`
- **THEN** the reader recorded exactly **one** call per resolved path, all of them during the
  first sync, and none during the four folds
- **AND** the run returns `Ok(..)` and the final buffer shows the fold state the four presses
  produced
