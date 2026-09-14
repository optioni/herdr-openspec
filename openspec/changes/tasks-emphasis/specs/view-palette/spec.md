## MODIFIED Requirements

### Requirement: Every role keeps the modifier the crate applied before this change

Colour SHALL be added **beside** the modifier a role already carried, never in place of it,
and `color-palette` added, removed, or altered **no modifier anywhere**. `pane-chrome` is the
first change that alters one: `RegionHeading` carries `DIM` where the `RegionBorder` it
replaces carried none, because an unfocused **heading** is text a reader can mistake for
content while an unfocused border was a line nobody read. `RegionHeadingFocused` keeps
`RegionBorderFocused`'s `BOLD` unchanged, and no other row of the table below moves. The consequence is
falsifiable rather than aspirational: in a captured `TestBackend` buffer compared by modifier
alone, every cell **outside the artifact tab-bar row** after that change carries exactly the
modifier it carried before it.

The tab-bar row is the stated exception, because `artifact-tabs` moves and relabels every
chip in it. Its own guarantee is narrower and is stated here rather than left out: the
selected chip's span remains that row's only `BOLD` span, so a monochrome reader still learns
which tab is current — what that reader loses is the `1`–`9` digits, which the chip grammar
drops deliberately and `action_for` still answers.

`tasks-emphasis` adds **five** rows to the table below and alters none, so the invariant holds
through it as well: nothing that carried a modifier before it carries a different one after,
and the only cells that gain one are a completed task item's own rows, which could carry
`Muted` only once a task item had a face to carry it.

`markdown-constructs` adds one row to the table below and alters none of the others, and
`foldable-spec-sections` adds two and alters none, so the invariant holds through both: no
cell that carried a modifier before either change carries a different one after, and the only
cells that gain a modifier are those the new construct or the new row produces — a
`~~struck~~` source for `CROSSED_OUT`, and a multi-file artifact's own section header rows
for the two below, neither of which could be drawn at all before. `pane-chrome`'s own single
alteration is stated above and is the one exception to "alters none".

The modifier each role SHALL carry:

| Role | Modifiers |
|---|---|
| `FileMode` | `DIM` |
| `Footer` | none |
| `RegionHeading` | `DIM` |
| `RegionHeadingFocused` | `BOLD` |
| `RegionRule` | `DIM` |
| `ListRow` | none |
| `ListRowSelected` | `BOLD` |
| `ListProblem` | none |
| `ListSeparator` | none |
| `ListMessage` | none |
| `AgentBadge(_)` | none |
| `TabActive` | `BOLD` |
| `TabInactive` | none |
| `DetailSection` | `BOLD` |
| `DetailSectionSelected` | `BOLD` + `REVERSED` |
| `Heading(_)` | `BOLD` |
| `Strong` | `BOLD` |
| `Emphasis` | `ITALIC` |
| `Code` | `DIM` |
| `Link` | `UNDERLINED` |
| `Quoted` | `DIM` |
| `Strikethrough` | `CROSSED_OUT` |
| `Muted` | `DIM` |
| `TaskEvidence` | none |
| `TaskChange` | none |
| `TaskConfirm` | none |
| `TaskLabel` | none |

`CROSSED_OUT` is chosen over a colour or a bracketing glyph for the same reason every other
row of this table carries a modifier: it is the terminal's own rendering of exactly this
meaning, it costs no columns, and a terminal that does not support it drops the attribute and
still shows the text — which is the right failure for a construct whose whole point is that
the text is still there.

`Muted`'s plain `DIM` deliberately equals the plain `DIM` that `RegionHeading`, `RegionRule`,
and `Quoted` already carry — plain-modifier equality is not a distinction this
table polices, on exactly the terms the `DetailSection` paragraph below states for plain
`BOLD`. It is a role of its own rather than a reuse of `Quoted` because a finished task and a
block quote are not the same thing, and a reader tracing why a row is dim should land on a
role that says so.

The four **task-label** roles carry **no** modifier, which is deliberate and is the one place
this table admits a monochrome reader loses something. What that reader loses is only the
*grouping* — which third of a lifecycle a label belongs to — and never the information: the
label is still the literal text `VERIFY:` on the row. A modifier there would have to be `BOLD`,
which would make the leading token of most rows in a task file bold and defeat the
de-emphasis this change exists to add.

`REVERSED` is chosen for the same reason and is the table's first use of it. A section header
is a fold control, and reversing it is how a terminal says "this is the one the keys address"
without spending a column on a marker glyph or borrowing a colour that would then mean two
things. `DetailSection`'s plain `BOLD` deliberately equals the plain `BOLD` that
`RegionHeadingFocused`, `ListRowSelected`, `TabActive`, `Heading(_)`, and `Strong` already
carry — plain-`BOLD` equality is not a distinction this table polices, and the two shared
*style* pairs named in the colour requirement are unaffected because neither new role carries
a colour.

#### Scenario: Each role's modifier set is exactly the table above

- **WHEN** `palette::style` is called for every `Role` variant and its `add_modifier` set is
  compared against the table
- **THEN** every role matches, and the **eleven** roles that carry no modifier — `Footer`,
  `ListRow`, `ListProblem`, `ListSeparator`, `ListMessage`, `AgentBadge`, `TabInactive`,
  `TaskEvidence`, `TaskChange`, `TaskConfirm`, and `TaskLabel` — carry none, `Muted` having
  joined the modifier-carrying side and the four label roles the other
- **AND** the assertion discriminates: `Emphasis` reports `ITALIC` and not `BOLD`,
  `Strikethrough` reports `CROSSED_OUT` and not `DIM`, `DetailSectionSelected` reports
  `BOLD | REVERSED` and not `BOLD` alone, and `Muted` reports `DIM` and not `CROSSED_OUT`

#### Scenario: A monochrome reading of the frame is unchanged

- **WHEN** a `Dashboard` carrying a repository in file mode, one problem row, three active
  changes of which one is badged `Working`, and a selected change whose single-section
  content is `## Heading\n\n**bold** and *italic* and `code` and [link](u)\n` is rendered at
  120x20 and at 60x20
- **THEN** in both buffers the modifier of every cell **outside row 2, the tab bar** is
  exactly what the same dashboard produced before `color-palette` once `pane-chrome`'s own
  three modifier changes are applied — the removed frame header row, the routed region's
  heading `BOLD` and the unrouted one's `DIM`, and the rules' `DIM`: the `file mode` badge
  `DIM`, the selected row's cells `BOLD`, the heading and `bold` `BOLD`, `italic` `ITALIC`,
  `code` `DIM`, and `link` `UNDERLINED`
- **AND** the problem row, the separator row, and the agent badge cell carry no modifier at
  all, exactly as before
- **AND** in the tab-bar row the selected chip's span is the only `BOLD` span, so the one
  excepted row still discriminates the current tab without colour
- **AND** the same source with `~~struck~~` appended renders that word's cells with
  `CROSSED_OUT` and leaves every other cell's modifier unchanged, so the new role adds a
  modifier only where the new construct appears
- **AND** the same dashboard whose selected artifact resolves to **three** files instead of
  one renders `REVERSED` on exactly one row — the cursor's own section header — and on no
  cell anywhere else, so a single-file artifact's frame is untouched by the two new roles

#### Scenario: The five new roles leave every existing cell's modifier where it was

- **WHEN** the `color-palette` monochrome fixture above — a repository in file mode, one
  problem row, three active changes one of which is badged `Working`, and a selected change
  whose single-section content is `## Heading\n\n**bold** and *italic* and `code` and
  [link](u)\n` — is rendered at 120x20 and at 60x20
- **THEN** every cell's modifier is byte-for-byte what the same dashboard produced before
  `tasks-emphasis`, because that fixture's selected artifact does not track tasks and so no
  cell carries any of the five new roles
- **AND** the same dashboard whose selected artifact tracks tasks and whose file holds one
  checked and one unchecked item renders `DIM` on exactly the checked item's own rows, and on
  no cell anywhere else that did not already carry it
- **AND** in that same frame no cell carries a modifier the four label roles could have added,
  since they add none

### Requirement: Colour is added only where it carries a distinction a modifier cannot

The palette SHALL assign a foreground or background colour to exactly these roles, and to no
other:

| Role | Colour |
|---|---|
| `FileMode` | foreground `Yellow` |
| `ListProblem` | foreground `Red` |
| `ListSeparator` | foreground `DarkGray` |
| `AgentBadge(Working)` | foreground `Green` |
| `AgentBadge(Idle)` | foreground `Cyan` |
| `AgentBadge(Blocked)` | foreground `LightRed` |
| `AgentBadge(Done)` | foreground `Blue` |
| `AgentBadge(Unknown)` | foreground `DarkGray` |
| `TabActive` | foreground `Black`, background `Cyan` |
| `TabInactive` | background `DarkGray` |
| `Heading(1)` | foreground `Magenta` |
| `Heading(2)` | foreground `Cyan` |
| `Heading(3)` | foreground `Blue` |
| `Heading(4)` | foreground `Green` |
| `Heading(5)` | foreground `Yellow` |
| `Heading(6)` | foreground `DarkGray` |
| `Code` | foreground `Yellow` |
| `Link` | foreground `Blue` |
| `TaskEvidence` | foreground `LightRed` |
| `TaskChange` | foreground `Green` |
| `TaskConfirm` | foreground `Blue` |
| `TaskLabel` | foreground `DarkGray` |

`Footer`, `RegionHeading`, `RegionHeadingFocused`, `RegionRule`, `ListRow`, `ListRowSelected`,
`ListMessage`, `DetailSection`, `DetailSectionSelected`, `Strong`, `Emphasis`, `Quoted`,
`Strikethrough`, and `Muted` SHALL carry **no** colour: each already carries a modifier that distinguishes
it, and a colour there would be decoration rather than information. `Quoted` in particular stays `DIM` and uncoloured. `Strikethrough` joins that
list rather than gaining an entry of its own: `CROSSED_OUT` already says the whole of what
the face means, and the obvious candidate colour — `DarkGray` — is this palette's one "no
information" grey, which struck text emphatically is not, since the reader is being shown
what it says as well as that it is struck. `DetailSection` and `DetailSectionSelected` join
it for the same reason: `BOLD` and `BOLD | REVERSED` already carry the whole distinction
between a fold header and the fold header the keys address, and this change's own design
records that adding a colour there would be decoration. `Muted` joins it too, and for the
sharpest version of the reason: it is the role that says *stop looking here*, and a colour is
the opposite instruction.

The four **task-label** roles are the inverse case and take a colour precisely because no
modifier distinguishes them. `TaskEvidence`, `TaskChange`, and `TaskConfirm` are the three
positions of a testing lifecycle — evidence, then the change, then the confirmation — which
`task-labels` defines without reference to any schema, and `TaskLabel` is the generic role a
recognised but unclassified token falls back to. Four hues rather than one per keyword is the
whole point: `VERIFY`, `THEN`, and `ASSERT` are one position under three conventions, and
colouring them separately would be a rainbow nobody learns. `TaskLabel`'s `DarkGray` is this
palette's "no information" grey used for exactly that: a label the crate recognises as a label
and classifies no further.

Roles MAY share a style, but only under one of two stated licences, and a share outside both
is a defect rather than a decision:

1. **They cannot meet.** Two roles that are never drawn in the same region may share a style,
   because no reader is ever asked to tell them apart. `FileMode` and `Code` are both
   `DIM` + `Yellow` under this licence: one is drawn in the list region's heading row, the
   other only inside the detail region's content area.
2. **They mean the same thing.** `DarkGray` is this palette's one "no information" grey, and
   every role wearing it means exactly that. `AgentBadge(Unknown)`, `ListSeparator`, and — as
   of `tasks-emphasis` — `TaskLabel` all carry it: an unknown agent status, a divider rule, and
   a label the crate declines to classify are the same statement three times.

This replaces the enumeration this requirement previously carried ("exactly two pairs"), which
`tasks-emphasis` would otherwise have had to grow by four with no principle to grow it by. The
enumeration is restated as a consequence rather than a rule: the full set of shared styles
SHALL be `FileMode`/`Code`, the `DarkGray` trio above, `TaskEvidence` with
`AgentBadge(Blocked)`, `TaskChange` with `AgentBadge(Working)` and `Heading(4)`, `TaskConfirm`
with `AgentBadge(Done)`, `Heading(3)` and `Link`, and `Muted` with `RegionHeading`,
`RegionRule` and `Quoted`.

Every one of those is licence 1, and the three that matter are worth spelling out because the
obvious objection is the one this change was asked to answer. `TaskEvidence` takes `LightRed`
and **not** `Red` specifically so that it does not collide with `ListProblem`, which is drawn
in the **detail** region — the same region a task label is drawn in — and so would have been a
share with no licence at all. `AgentBadge(Blocked)`, which `LightRed` does collide with, is
drawn only in the list region. `TaskChange`'s `Green` and `TaskConfirm`'s `Blue` meet
`Heading(4)` and `Heading(3)` only in principle: a heading face reaches the detail region's
content area only on the markdown path or on a **non-foldable** tracked-tasks tab, and a
non-foldable tracked-tasks tab is by `artifact-folds`' own definition one whose file carries no
heading at all. A label and a heading face therefore cannot appear in one frame's content area.
`Link` cannot meet them for a blunter reason: `ui::tasks` emits no link face for any input.

`Strikethrough` SHALL remain a style equal to no other role's, and `DetailSectionSelected`
likewise, so the two roles whose whole job is to be unmistakable stay unshared.

`Heading(l)` for an `l` outside `1..=6` SHALL return the same `Style` as `Heading(6)`:
`markdown-render` produces only `1..=6`, and the function is total rather than panicking on a
value the parser cannot emit.

#### Scenario: The coloured set is exactly the table above

- **WHEN** `palette::style` is called for every `Role` variant and each result's `fg` and `bg`
  are inspected
- **THEN** exactly the roles in the table above report a `Some` foreground or background, with
  the named variant the table gives
- **AND** every other role reports `fg: None` and `bg: None`, `Strikethrough`,
  `DetailSection`, `DetailSectionSelected`, and `Muted` among them
- **AND** `TaskEvidence` reports `LightRed` and **not** `Red`, so the one colour choice this
  change was asked to justify is the one an assertion would catch being reverted

#### Scenario: An out-of-range heading level does not panic

- **WHEN** `palette::style(Role::Heading(0))`, `Role::Heading(7)`, and `Role::Heading(255)`
  are called
- **THEN** none panics and each returns the same `Style` as `Role::Heading(6)`

#### Scenario: Every shared style is licensed, and the unshared roles stay unshared

- **WHEN** `palette::style` is called for every `Role` variant, every `AgentStatus`, and
  heading levels 1 through 6, and the results are grouped by equal `Style`
- **THEN** every group of size greater than one is one of the groups this requirement
  enumerates, and no other group has size greater than one
- **AND** `Strikethrough` and `DetailSectionSelected` are each alone in their group
- **AND** `TaskEvidence`'s group holds `AgentBadge(Blocked)` and **not** `ListProblem`, which
  is the distinction between a licensed share and an unlicensed one for this change

#### Scenario: A task label and a problem row are distinguishable in one frame

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one problem and
  whose tracked-tasks artifact holds the unchecked item `- [ ] 1.1 RED: write the test` is
  rendered at 120x20 and at 60x20
- **THEN** at each width the problem row's cells carry `palette::style(Role::ListProblem)`'s
  foreground and the `RED:` cells carry `palette::style(Role::TaskEvidence)`'s
- **AND** those two foregrounds are not equal, so the two constructs that share the detail
  region are distinguishable by colour and not only by shape
- **AND** neither assertion writes a colour literal: both compare against `palette::style`, as
  every render test outside `src/ui/palette.rs`'s own tests does

### Requirement: `ui::view` takes every style it applies from the palette

`ui::view` SHALL construct no `Style` of its own: every span it writes to the buffer SHALL be
`palette::style(role)` for the role that span carries, or a fixed composition of such styles.
The mapping from a drawn span to its role SHALL be:

- a region's heading row → `RegionHeadingFocused` when that region is the routed one, else
  `RegionHeading`. That covers the list region's repository name and the detail region's
  change header alike;
- the `file mode` badge → the heading row's own style patched with `FileMode`, so the badge is
  dim and yellow whether or not the list region is the routed one;
- the vertical divider and the detail region's horizontal rule → `RegionRule`;
- the footer row, in all three of its forms → `Footer`. It is named rather than left as a
  bare `Style::default()` so the requirement below — that `ui::view` constructs no `Style` of
  its own — is true of the whole file rather than of the functions this change happened to
  visit;
- a list row → `ListRowSelected` when `Row::selected`, else `ListProblem`, `ListSeparator`,
  or `ListMessage` by its `RowKind`, else `ListRow`;
- a badged change row's badge cell → the row's own style patched with
  `AgentBadge(status)`, so a badge on the selected row is coloured **and** bold;
- an artifact tab chip → `TabActive` when `Tab::selected`, else `TabInactive`;
- a detail content row whose `ContentKind` is `SectionHeader { selected: true }` →
  `DetailSectionSelected`, and one whose kind is `SectionHeader { selected: false }` →
  `DetailSection`, patched **over** the segment's own `style_for(&segment.face)` so a header
  row's emphasis wins over the plain face its text carries;
- every other rendered content segment — `ContentKind::Problem` and `ContentKind::Body` —
  → `style_for(&segment.face)`, below, exactly as before this change.

That penultimate clause is `foldable-spec-sections`' one addition to this mapping, and it is
the reason this requirement is reproduced here. `ui::detail::content_lines` returns a
`ContentKind` per row and names no `Role`, on exactly the terms `ui::list` returns a `RowKind`
and names none: **`ui::view` alone decides what a row looks like.** A `markdown::Face` cannot
carry this distinction — a section header is not a construct any segment's face describes, and
it is a property of the **row**, not of a run within it — which is why the role is selected by
kind here rather than folded into `style_for`.

`tasks-emphasis` adds two face fields and therefore two composing roles, taking `style_for`
from seven to **nine**. The test of where a distinction belongs is unchanged and is what
decides both: a *row-wide* distinction the renderer knows and the text does not is a
`ContentKind`, and a distinction about **a run of text** is a `Face`. A finished task item and
a lifecycle label are both facts about runs of text — `muted` happens to cover every run on
its row, but nothing about the rule turns on that — so both are faces.

`ui::view::style_for(face: &markdown::Face) -> Style` SHALL compose the palette's face roles
by folding them onto `Style::default()` with `Style::patch` in this fixed order:

1. `Muted`, when `face.muted`;
2. `Quoted`, when `face.quoted`;
3. `Strikethrough`, when `face.strikethrough`;
4. `Link`, when `face.link`;
5. `Code`, when `face.code`;
6. `Emphasis`, when `face.emphasis`;
7. `Strong`, when `face.strong`;
8. `Heading(level)`, when `face.heading` is `Some(level)`;
9. `TaskEvidence`, `TaskChange`, `TaskConfirm`, or `TaskLabel`, when `face.label` is
   `Some(role)`, selected by that `LabelRole`.

`Strikethrough` is inserted at position 3 — `markdown-constructs`' only edit to the order —
precisely because it carries **no** foreground: wherever it sits it cannot take a colour away
from a role that has one, so it is placed early, beside the other uncoloured,
always-composing faces.

`Muted` is placed **first** for the same reason and one more: it carries no foreground either,
so it takes no colour from anything, and putting it first means every later role's colour wins
over it rather than being suppressed by position. `face.label` is placed **last** so that a
label's colour is the one a reader sees, ahead of every markdown face a label segment could in
principle also carry.

The two new steps SHALL NOT be assumed mutually exclusive by the implementation even though
`tasks-checklist` never emits both: a checked item's segment carries `label: None` and a label
segment carries `muted: false`, so the combination is unreachable in production, and
`style_for` SHALL nonetheless answer it — `DIM` plus the label's foreground — rather than
`debug_assert`ing against a caller it does not control. Totality is the contract, not the
absence of a caller.

Because `patch` lets the later value win, modifiers accumulate — a bold link's cells carry
`BOLD` and `UNDERLINED` together, and a struck bold link's carry `CROSSED_OUT` as well —
while the **foreground** of a span carrying several coloured faces is decided by the last one
in that order. The precedence is therefore heading over code over link, stated here rather
than left to be discovered: a heading line reads as one colour even where it contains a code
span or a link, which is the point of colouring the heading at all.

`style_for` SHALL be total: no `Face` value panics, and `Face::plain()` SHALL map to
`Style::default()` — which SHALL stay true with the two new fields at their `false`/`None`
zero, so every existing rendering is byte-identical unless a new field is set.

#### Scenario: Faces reach the buffer as coloured styles at both mandated widths

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact not
  marked `tracks_tasks`, resolving to a single section holding
  `# Title\n\n## Heading\n\n**bold** and *italic* and `code` and [link](u) and ~~struck~~\n`,
  is rendered at 120x20 and at 60x20
- **THEN** in each buffer the cells of `# Title` report `BOLD` set and the foreground
  `Role::Heading(1)` carries (`Magenta`), and the cells of `## Heading` report `BOLD` set and
  the foreground `Role::Heading(2)` carries (`Cyan`)
- **AND** the cells of `code` report `DIM` and the foreground `Role::Code` carries (`Yellow`),
  and the cells of `link` report `UNDERLINED` and the foreground `Role::Link` carries
  (`Blue`)
- **AND** the cells of `bold` report `BOLD` with no foreground, of `italic` `ITALIC` with
  no foreground, and of `struck` `CROSSED_OUT` with no foreground, so the uncoloured roles are
  discriminated from the coloured ones
- **AND** no cell in either buffer reports `REVERSED`, because a single-section artifact draws
  no header row

#### Scenario: A section header's role is selected by its kind, not by its face

- **WHEN** the same dashboard's artifact resolves to three spec files, the cursor is on the
  second, and it is rendered at 120x20 and at 60x20
- **THEN** in each buffer the second header row's cells equal
  `palette::style(Role::DetailSectionSelected)` and are the only cells reporting `REVERSED`
- **AND** the first and third header rows' cells equal `palette::style(Role::DetailSection)`,
  and `content_lines` reports `SectionHeader { selected: false }` for both — the assertion
  that can fail, since `DetailSection`'s plain `BOLD` is indistinguishable from `Role::Strong`'s
  by cell comparison alone
- **AND** every segment of every body row still equals `style_for(&segment.face)` with no role
  patched over it, so the header mapping reaches header rows and nothing else
- **AND** `grep -n 'Role::' src/ui/detail.rs` returns nothing, so the role selection lives in
  `ui::view` and the kind alone crosses the boundary

#### Scenario: Heading foreground wins over a code span inside it

- **WHEN** `style_for` is called on a `Face` with `heading: Some(2)` and `code: true`, on
  one with `code: true` and `link: true`, and on one with `strikethrough: true`,
  `strong: true`, and `link: true`
- **THEN** the first reports the foreground `Role::Heading(2)` carries — the heading's — with
  `BOLD` and `DIM` both set, so no modifier was lost to the precedence rule
- **AND** the second reports the foreground `Role::Code` carries — which follows the link in
  the fold order — with `DIM` and `UNDERLINED` both set
- **AND** the third reports `CROSSED_OUT`, `BOLD`, and `UNDERLINED` all set and the foreground
  `Role::Link` carries, so an uncoloured strikethrough neither loses its own modifier nor
  displaces the link's colour
- **AND** no assertion names a `Color` literal: all compare against `palette::style`,
  because `style_for` lives in `src/ui/view.rs`, which the confinement gate searches

#### Scenario: A plain face is the default style

- **WHEN** `style_for(&Face::plain())` is called
- **THEN** it returns `Style::default()`, with no modifier, no foreground, and no background
- **AND** `Face::plain()`'s `strikethrough` is `false`, so the new field does not change what
  a plain face maps to
- **AND** rendering a plain-text document at 120x20 and 60x20 leaves every content cell's
  style equal to `ratatui::buffer::Cell::default().style()`

#### Scenario: The two new face fields compose in their stated positions

- **WHEN** `ui::view::style_for` is called on `Face { muted: true, ..Face::plain() }`, on
  `Face { label: Some(LabelRole::Evidence), ..Face::plain() }`, on
  `Face { label: Some(LabelRole::Other), ..Face::plain() }`, and on
  `Face { muted: true, label: Some(LabelRole::Confirm), ..Face::plain() }`
- **THEN** the first equals `palette::style(Role::Muted)`, the second
  `palette::style(Role::TaskEvidence)`, and the third `palette::style(Role::TaskLabel)`
- **AND** the fourth carries `DIM` **and** `palette::style(Role::TaskConfirm)`'s foreground,
  the unreachable combination answered rather than refused
- **AND** `style_for(&Face::plain())` is still `Style::default()`, so the zero value did not
  move

#### Scenario: A checklist row reaches the buffer with its label coloured

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true` and whose file reads
  `## 1. Setup\n\n- [ ] 1.1 RED: write it\n- [x] 1.2 VERIFY: it passes\n` is rendered at
  120x20 and at 60x20
- **THEN** at each width the `RED:` cells carry `palette::style(Role::TaskEvidence)`'s
  foreground and no `DIM`
- **AND** every cell of the `[✓] 1.2 VERIFY: it passes` row carries `DIM`, and no cell of it
  carries `palette::style(Role::TaskConfirm)`'s foreground — the completed row's label is
  de-emphasised rather than dimmed-but-coloured
- **AND** no assertion in this scenario names a colour literal: each compares against
  `palette::style(role)`
