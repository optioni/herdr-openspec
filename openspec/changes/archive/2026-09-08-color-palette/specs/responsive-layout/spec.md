## MODIFIED Requirements

### Requirement: The header names the repository root, shortened from the left when narrow

The header row SHALL render, right-aligned so that its last column sits in the final
column, the repository root's display path when one was found, and the literal
`no repository` when none was. Exactly one blank column SHALL separate the `OpenSpec`
label from the shortened text at minimum.

**The `file mode` badge.** When `Dashboard::file_mode` is true — the `openspec` binary probe
resolved no usable binary, so the change list is file-sourced for the whole session — the
header SHALL draw the literal `file mode`, nine columns, immediately after the `OpenSpec`
label and one separating blank, in columns 9 through 17, styled with
`palette::style(Role::FileMode)`: ratatui's `DIM` modifier — and no other modifier — together
with foreground `Color::Yellow`. The badge stays dim because it names a *mode*, not a fault:
file mode is a supported way to run, and a badge competing with the repository path for
attention would say otherwise. It is coloured because `DIM` alone is what an archived row's
date, an inline code span, a block quote, and an agent badge already are, and a badge that
shares its whole style with four other things names nothing.

The `OpenSpec` label SHALL be drawn with `palette::style(Role::HeaderTitle)` and the
right-aligned path — or `no repository` — with `palette::style(Role::HeaderPath)`. Neither
carries a colour: the label already carries `BOLD` and the path is the row's only other
content, so the badge's yellow is the one new distinction this row gains.

The badge SHALL be dropped **whole**, never cut short, when the header width is below 18 —
the eight columns of `OpenSpec`, one blank, and the badge's nine — on exactly `change-rows`'
drop-whole rule. It is dropped **before** the path is shortened, not after: at a width that
cannot hold both, the reader can still learn the repository from the pane's contents, and a
half-drawn `file mo` would name nothing at all.

When `file_mode` is false the header SHALL be byte-identical to the header this requirement
already specified — no badge, no reserved columns, and the same `A`.

Let `A` be the header width minus 9 — the eight columns of `OpenSpec` plus one separating
blank — **floored at zero**, so widths below 9 do not underflow the unsigned subtraction.
When the badge is drawn, `A` SHALL instead be the header width minus **19** — the same nine,
plus the badge's nine and one further separating blank — floored at zero by the same rule.
When the text's `layout::columns` is at most `A` it SHALL be rendered whole. When it is
longer and `A` is at least 8, it SHALL be rendered as `…` followed by the **last `A - 1`
columns** of the text — the longest suffix ending on a grapheme-cluster boundary that
measures at most `A - 1`. When `A` is below 8 the text SHALL be omitted entirely and only
`OpenSpec` SHALL be drawn; when the width is below 8 the label itself SHALL be truncated to
the columns available. Shortening SHALL count **display columns**, not characters and not
bytes, and SHALL keep the tail — the repository's own directory name is what identifies it,
and the leading path components are what a reader can spare.

Because a cluster is dropped whole, the shortened text MAY measure one column less than the
space allotted to it. The row SHALL still be right-aligned against its own measured width,
so the last drawn column is the final column and any slack falls to the **left** of the
ellipsis, where the `OpenSpec` label's trailing blanks already are. The header SHALL never
draw past its last column at any width for any path.

The badge SHALL be drawn on the **frame** header, not on the detail region's change header.
The two are different claims: a missing binary is a fact about the process, true of every
change in the pane, while the detail header describes one change — and `SPEC.md` → Degraded
states already gives the per-change equivalent its own row, "Schema unknown to the CLI",
whose fall-back to file mode is per change and is named in the detail region instead.

The shortening rule SHALL be one shared implementation with `change-rows`' no-repository
block, which shortens `searched_from` by the same keep-the-tail rule against the list
region's interior width rather than the header's.

#### Scenario: A path that fits is right-aligned whole at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` — fourteen characters —
  is rendered at 60x20 and at 120x20
- **THEN** the 60-column header row spells `/tmp/demo-repo` in columns 46 through 59, and
  columns 8 through 45 are spaces
- **AND** the 120-column header row spells `/tmp/demo-repo` in columns 106 through 119

#### Scenario: A path too long for the narrow header is shortened from the left

- **WHEN** a `Dashboard` whose repository root is
  `/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here` — seventy
  characters — is rendered at 60x20 and at 120x20
- **THEN** the 60-column header row spells
  `…/openspec-demos/a-rather-long-repository-name-here` in columns 9 through 59, so the
  first character of the shortened text is the ellipsis and the last is the final `e` of
  the directory name
- **AND** the 120-column header row spells the whole seventy-character path in columns 50
  through 119, with no ellipsis anywhere in the buffer — which stays true only because
  that `Dashboard`'s `changes` is `changes::empty_set()` with a repository root present,
  so the list interior holds the fourteen-character `No changes yet` and needs no
  ellipsis of its own

#### Scenario: A header too narrow for any path shows only the label

- **WHEN** the same seventy-character `Dashboard` is rendered at 16x20, at 60x20, and at
  120x20
- **THEN** the 16-column header row is exactly `OpenSpec` followed by eight spaces, and no
  ellipsis appears in that row
- **AND** the 60-column header row carries the shortened, ellipsis-prefixed path in columns
  9 through 59, and the 120-column header row carries the full path in columns 50 through
  119 — so the 16-column omission is a width branch and not the feature being absent
- **AND** no ellipsis appears anywhere in the 16-column buffer: its list interior is
  fourteen columns wide and `No changes yet` is exactly fourteen characters, so the body
  neither truncates nor overflows

#### Scenario: No repository found is named in the header at both widths

- **WHEN** a `Dashboard` built with no repository root — the value `ui::load` produces when
  `resolve::find_repo` reports `NotFound`, with `searched_from` `/tmp/searched-from` — is
  rendered at 60x20 and at 120x20
- **THEN** the 60-column header row spells `no repository` in columns 47 through 59
- **AND** the 120-column header row spells `no repository` in columns 107 through 119
- **AND** neither **header row** names the directory the search started from: row 0 of
  each buffer does not contain `/tmp/searched-from`. The **body** now does, and that is
  `change-rows`' no-repository block — the landed form of this scenario asserted the
  string was absent from the whole buffer, which `list-view` makes false; the assertion
  is narrowed to row 0, which is what the requirement was ever about

#### Scenario: The badge is drawn dim after the label at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose `file_mode` is
  **true** is rendered at 60x20 and at 120x20
- **THEN** the header row's columns 9 through 17 spell `file mode` at both widths, and column
  8 is a space
- **AND** every one of those nine cells carries ratatui's `DIM` modifier **and** the
  foreground `Role::FileMode` carries (`Color::Yellow`), and the `OpenSpec` label's eight
  cells carry `Modifier::BOLD` and no
  foreground at all, so the badge is distinguishable from the label by colour as well as by
  weight and position
- **AND** the drawn path's cells carry neither a modifier nor a foreground, so the yellow is
  confined to the badge's own nine columns
- **AND** the 60-column header spells `/tmp/demo-repo` in columns 46 through 59 and the
  120-column header in columns 106 through 119 — unchanged, because a fourteen-character path
  fits inside `A` at both widths either way

#### Scenario: A false flag renders the header that landed before this change

- **WHEN** the same `Dashboard` is rendered with `file_mode` **false** at 60x20 and at 120x20
- **THEN** neither buffer contains the substring `file mode` anywhere, in any row
- **AND** both **header rows** are identical, cell for cell and style for style, to the ones
  the same dashboard produced before this change existed — no cell of row 0 carries a
  foreground, so the palette added colour to the badge and to nothing else on this row

#### Scenario: The badge takes its columns from the path, not from the label

- **WHEN** a `Dashboard` whose repository root is
  `/home/dev/workspaces/openspec-demos/a-rather-long-repository-name-here` — seventy
  characters — and whose `file_mode` is true is rendered at 60x20
- **THEN** the header row's columns 9 through 17 spell `file mode`
- **AND** the shortened path occupies columns 19 through 59 — `A` is 41 rather than 51 — and
  begins with the ellipsis, so ten more leading characters were spared than without the badge
- **AND** column 18 is a space, so the badge and the path never abut
- **AND** the same dashboard at 120x20 spells the whole seventy-character path with no
  ellipsis, the badge still in columns 9 through 17

#### Scenario: A header too narrow for the badge drops it whole

- **WHEN** the same seventy-character, file-mode `Dashboard` is rendered at 17x20, at 18x20,
  and at 60x20
- **THEN** the 17-column header row does not contain `file mode`, nor any prefix of it: it is
  exactly `OpenSpec` followed by nine spaces
- **AND** the 18-column header row spells `OpenSpec`, a space, then `file mode` in columns 9
  through 17, so 18 is the exact width at which the badge appears and 17 the one at which it
  does not
- **AND** the 60-column header carries both the badge and the ellipsis-prefixed path, so the
  17-column omission is a width branch and not the feature being absent
- **AND** no ellipsis appears anywhere in the 17-column buffer's header row

#### Scenario: A wide-character path is shortened by columns and stays inside the header

- **WHEN** a `Dashboard` whose repository root is
  `/home/dev/workspaces/日本語のリポジトリ名前がとても長いディレクトリ` — forty-four
  characters and **sixty-seven display columns** — is rendered at 60x20 and at 120x20, and
  again with `file_mode` true. The fixture is chosen to exceed `A` at the narrow width both
  with the badge (41) and without it (51), so both branches actually shorten; a
  wide-character path short enough to fit would leave every assertion below unreachable
- **THEN** in every one of the four buffers the header row's last drawn column is the frame's
  final column and no cell beyond it is written
- **AND** in the 60-column, non-badged buffer the shortened text begins with `…` at a column
  no earlier than 9 and ends in the final column, and its `columns` is at most `A` — 51
- **AND** a `char`-counted shortening of the same path would have kept its last 50
  **characters**, which measure far more than 51 columns and would have run past the frame —
  so the scenario distinguishes the two measures rather than merely exercising one
- **AND** in the 60-column, badged buffer the badge occupies columns 9 through 17, column 18
  is blank, and the shortened path's `columns` is at most `A` — 41 — so the badge took its
  columns from the path exactly as the unbadged rule says
- **AND** in the 120-column buffer the whole sixty-seven-column path is drawn, its first
  column no earlier than column 50, and no ellipsis appears in that row
- **AND** rendering the same dashboard at 16x20, 18x20, 19x20, and 1x20 draws only the
  label or a truncation of it, writes nothing past the last column, and does not panic

## ADDED Requirements

### Requirement: A region's border style is a palette role

`ui::view::render_region` SHALL take the border's style from the palette:
`palette::style(Role::RegionBorderFocused)` for the region the dashboard's route names, and
`palette::style(Role::RegionBorder)` for the other region when it is drawn. It SHALL
construct no `Style` of its own.

`Role::RegionBorderFocused` SHALL carry `Modifier::BOLD` and **no colour**, and
`Role::RegionBorder` SHALL carry neither, so the rendered result is exactly what "The routed
region is emphasised and region interiors are left empty" already requires. The border frames
the pane rather than saying anything about it; colouring it would tint every frame for no
distinction. The style SHALL reach `Block::border_style` and not `Block::style`, so a blank
interior's cells still equal `ratatui::buffer::Cell::default().style()`.

#### Scenario: The routed region's border takes its style from the palette at both widths

- **WHEN** a `Dashboard` at `Route::List` is rendered at 120x20, and a second at
  `Route::Detail` is rendered at 120x20 and at 60x20
- **THEN** in the first buffer every border cell of the `Changes` region reports
  `Modifier::BOLD` set and the `Detail` region's border cells do not, and in the second the
  two are swapped, so the assertion discriminates rather than asserting a constant
- **AND** no border cell in any buffer reports a foreground or a background, so the palette
  gave the border a role and not a colour
- **AND** every cell of a blank region interior still equals
  `ratatui::buffer::Cell::default().style()`, so the style reached `border_style` rather than
  `style`
