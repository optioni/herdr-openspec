## MODIFIED Requirements

### Requirement: One module maps every semantic role to a `Style`, and it is the crate's only `Color`

`text-selection` appends `Selected`, the thirty-first role, carrying
`Modifier::REVERSED` and **no colour**: a selection must invert whatever the text
underneath already is — a heading, a code span, a delta badge — and a foreground of its
own would erase that. It is the crate's only reversed role, which is what keeps it
distinguishable from `SelectedRow` without either naming a colour.

`ui::palette` SHALL be a new module under `src/ui/`, holding one table from a **semantic
role** to a `ratatui::style::Style` and nothing else. It SHALL be a pure total module: no
filesystem, process, environment, network, or standard-I/O API, no clock, no global state,
and no panic for any input.

```rust
pub enum Role {
    FileMode,
    Footer,
    RegionHeading,
    RegionHeadingFocused,
    RegionRule,
    ListRow,
    ListRowSelected,
    ListProblem,
    ListSeparator,
    ListMessage,
    AgentBadge(crate::agents::AgentStatus),
    TabActive,
    TabInactive,
    DetailSection,
    DetailSectionSelected,
    Heading(u8),
    Strong,
    Emphasis,
    Code,
    Link,
    Quoted,
    Strikethrough,
    Muted,
    TaskEvidence,
    TaskChange,
    TaskConfirm,
    TaskLabel,
    DeltaAdded,
    DeltaModified,
    DeltaRemoved,
    Selected,
}

pub fn style(role: Role) -> Style;
```

The enum is reproduced here because changes keep altering its membership — four in a row now,
`spec-emphasis` being the fourth. It adds **three** variants and removes none: `DeltaAdded`,
`DeltaModified`, and `DeltaRemoved`, one per delta operation `spec-delta-badges` recognises.
They are appended after the four `task-labels` roles, at the end, for the placement reason
every addition since `foldable-spec-sections` has used: a delta badge is neither a markdown
construct nor a region's chrome, so it joins no earlier group and sits where the previous
change's own appended group ended.

Three variants rather than one parameterised `Delta(DeltaOp)` — which `AgentBadge` and
`Heading` would both be precedents for — because the parameterised form buys nothing here.
`AgentBadge` is parameterised to answer a status enum that `src/agents.rs` owns and may grow,
and `Heading` to answer a level outside `1..=6` with a value rather than a lookup miss. A
`DeltaOp` has exactly three values, all three are spelled out in this module's own table, and
naming them separately is what lets the modifier and colour tables above list them as rows
like every other role rather than as one row with a nested match.

`tasks-emphasis` made the previous set. It added **five** variants and removed none: `Muted`,
for a task item the reader has finished with, and the four `task-labels` roles
`TaskEvidence`, `TaskChange`, `TaskConfirm`, and `TaskLabel`. They are appended after
`Strikethrough`, where the markdown faces end, because a task label is not a markdown
construct and a completed row is not a face the parser can emit — the same placement argument
`foldable-spec-sections` used for `DetailSection`. `pane-chrome`
made the first set: `RegionHeading` and `RegionHeadingFocused` replace `RegionBorder` and
`RegionBorderFocused` — there is no border to style — and `RegionRule` is added for the
vertical divider and the detail region's horizontal rule. `HeaderTitle` and `HeaderPath` are
removed with the frame header row that carried them, and `DetailHeader` is removed because the
detail region's change header is now a region heading and takes the same two roles every other
region heading takes; keeping a third role identical to `RegionHeadingFocused` in everything
but its name would let the two drift for no reason a reader could see.

`DetailSection` and `DetailSectionSelected` are `foldable-spec-sections`' two additions:
`artifact-folds` gives a multi-file artifact's content a header row per file, and whether that
row is the one the cursor addresses is a distinction this module decides, not `ui::detail`'s
or `ui::view`'s. They sit after `TabInactive`, which is where the detail region's own chrome
roles end and the markdown faces begin — an artifact-section header is chrome drawn inside the
detail region's content area, not a construct the markdown parser emitted. There is no
detail-chrome role left for them to sit beside, `DetailHeader` having been removed above.

`Strikethrough` remains the variant that first forced this reproduction: the face's `Style` is
this module's to decide, not a render call site's. The exhaustive-`match` role list the
totality scenario iterates is what makes each addition a **compile error** until the table
answers it, which is the property that keeps the enum and
the table from drifting — the same property that forced `Strikethrough` into the table rather
than leaving it remembered, and that made each of `pane-chrome`'s three removals a compile
error at every call site that named one.

`src/ui/palette.rs` SHALL be the **only file under `src/`** that names
`ratatui::style::Color` or a `Color::` variant. The scope is `src/` and not the whole crate
because that is what the gate searches; `grep -rn 'Color::' tests` returns nothing today and
the integration tier renders no frame, so a `tests/` leg would guard nothing and read as
enforced when it is not. Every other file styles by asking the palette
for a role. This is the same confinement `pulldown_cmark` has to `src/ui/markdown.rs` and a
process-spawn API has to `src/cli.rs`, and it SHALL be enforced the same way: a tree-wide
grep with a positive control, extracted as `scripts/gates/palette.sh`, composed into the
`Makefile`'s `gates:` recipe, and bound to a recorded planted defect in
`tests/gate-controls.toml`.

The gate SHALL fail when its own exclusion is **vacuous** — when `src/ui/palette.rs` is
missing, or exists but names no `Color` — before it reports a clean tree, so a gutted palette
is reported as a broken control rather than as a pass.

The gate SHALL also fail when `src/ui/palette.rs` is absent from `scripts/gates/noio-view.sh`'s
or `scripts/gates/colwidth.sh`'s `PURE` list. Both hard-code that list and check only that the
files they name exist, so a forgotten edit would leave the new module unswept while both
scripts still print `OK` — a gap no other check in the tree can see.

The same SHALL hold for `src/ui/help.rs`, `help-overlay`'s addition: the palette gate SHALL
fail when **either** of the two pure-view modules is missing from **either** `PURE` list. The
argument is the one above, one module on: `src/ui/help.rs` is a pure view file that styles by
role like every other, and a `PURE` list that has quietly stopped naming it is the one failure
the two sweeps themselves cannot report.

Each of the gate's three failure modes — a `Color` outside the palette, a non-ANSI colour
inside it, and a vacuous exclusion — SHALL have its **own** planted control in
`tests/gate-controls.toml`, on `gate-integrity`'s "executed, not attested" standard. One
script may carry several controls; `scripts/gates/wired.sh` already does.

**How a test asserts a colour.** The gate searches `src/`, and this crate's view tests live
in `#[cfg(test)]` modules **inside** `src/ui/view.rs`, `src/ui/list.rs`, and
`src/ui/detail.rs`. No test outside `src/ui/palette.rs` may therefore name a `Color` literal.
A render test SHALL assert a cell's colour by comparing it against the palette —
`assert_eq!(cell.style().fg, palette::style(Role::AgentBadge(AgentStatus::Working)).fg)` — and
the literal table SHALL be asserted once, in `src/ui/palette.rs`'s own tests, where "The
coloured set is exactly the table above" already lives and is the falsifiable half. Every
scenario in this change that names a colour is naming the palette entry a test compares
against, never a literal that test writes. The same rule governs a **modifier**: a render
test asserting that a section header is the emphasised one SHALL compare the cell's style
against `palette::style(Role::DetailSectionSelected)`, never against a `Modifier` it writes
itself.

`ui::palette` and `ui::help` SHALL both be in the pure view set both standing view gates
carry. `view-palette` left those lists at `NOIO-VIEW`'s **nine** files and `COLWIDTH`'s
**eight**, adding no module and moving neither count; measured at HEAD `08025d3`:
`NOIO-VIEW OK: 9 pure files` and `COLWIDTH OK: … the eight pure view files`.
`help-overlay` adds exactly one module, `src/ui/help.rs`, and moves both counts by one:
`NOIO-VIEW`'s `PURE` list becomes **ten** files and `COLWIDTH`'s **nine**, and both scripts'
reported counts move with them.

`ui::help` SHALL name no `ratatui::style::Color` and no `Color::` variant, and SHALL take
every style it applies from `palette::style(Role::…)`. It introduces **no new `Role`**: its
rule rows are `RegionRule`, its `Help` title and its group headings are
`RegionHeadingFocused`, its `input` cells are `Strong`, its `description` cells are
`ListRow`, and its scroll indicator is `ListSeparator`. Reusing five existing roles rather
than minting `HelpTitle`, `HelpGroup`, `HelpKey`, and `HelpText` is this requirement's own
"colour is added only where it carries a distinction a modifier cannot", applied to roles:
four new roles identical in every respect but their names to four existing ones would let the
two sets drift for no reason a reader could see, which is exactly why `pane-chrome` removed
`DetailHeader`.

#### Scenario: The palette answers every role with a `Style`

- **WHEN** `palette::style` is called once for every `Role` variant, including
  `AgentBadge` for each of the five `agents::AgentStatus` values, `Heading` for levels
  `1` through `6`, `Strikethrough`, `DetailSection`, and `DetailSectionSelected`
- **THEN** every call returns a `Style` and none panics
- **AND** the role list the test iterates is built from an **exhaustive** `match role { … }`
  rather than hand-enumerated, so a `Role` added later fails to compile until it is added here
  — which is exactly how `Strikethrough` was forced into this table rather than remembered,
  and how the two new roles are
- **AND** no two of `ListProblem`, `FileMode`, `TabActive`, `TabInactive`, and the five
  `AgentBadge` styles are equal to one another, so each carries a distinction rather than
  repeating its neighbour
- **AND** `RegionHeading`, `RegionHeadingFocused`, and `RegionRule` each return a `Style`, and
  the enum names no `RegionBorder`, `RegionBorderFocused`, `HeaderTitle`, `HeaderPath`, or
  `DetailHeader`
- **AND** the **five** deliberately shared coloured groups are asserted **equal** member for
  member — `FileMode` with `Code`; `AgentBadge(Unknown)` with `ListSeparator` **and**
  `TaskLabel`; `AgentBadge(Working)` with `TaskChange`; `AgentBadge(Done)` with
  `TaskConfirm`; and `AgentBadge(Blocked)` with `TaskEvidence` — so each share is a recorded
  decision rather than a gap the distinctness assertion happens to step around, and
  `Strikethrough` is asserted **unequal** to every other role's style, so it joins no group by
  accident. The bullet above stays true as written because it names only `ListProblem`,
  `FileMode`, `TabActive`, `TabInactive`, and the five `AgentBadge` styles, none of which this
  change makes equal to another; the four new coloured roles join `AgentBadge` styles rather
  than each other
- **AND** `DetailSectionSelected` is asserted **unequal** to every other role's style,
  `Strikethrough` included, so the emphasised header is distinguishable from every other span
  the frame can draw

#### Scenario: The confinement gate catches a `Color` named outside the palette

- **WHEN** `scripts/gates/palette.sh` is run against the repository tree
- **THEN** it exits `0` and its output names the file count it searched
- **AND** when the line `use ratatui::style::Color;` is planted in `src/ui/view.rs` it exits
  non-zero with a message naming `src/ui/view.rs`
- **AND** when `src/ui/palette.rs` is emptied of every `Color` mention it exits non-zero
  reporting the exclusion as vacuous rather than reporting a clean tree
- **AND** when `src/ui/palette.rs` is removed from `scripts/gates/noio-view.sh`'s `PURE` list
  it exits non-zero naming that script, so the two standing view gates cannot silently stop
  sweeping the new module
- **AND** the same holds for `src/ui/help.rs`: removing it from either script's `PURE` list
  makes the gate exit non-zero naming that script, so neither pure-view module can drop out of
  either sweep

#### Scenario: The palette module reaches no I/O and measures no width

The scenario's name is kept verbatim from `view-palette` because a delta's scenario headers are
its merge key; its subject widens from one pure-view module to two.

- **WHEN** `make gates` runs on a tree carrying `src/ui/palette.rs` and `src/ui/help.rs`
- **THEN** `NOIO-VIEW` reports **ten** pure files carrying no I/O API
- **AND** `COLWIDTH` reports **nine** pure view files carrying no `char`-count measurement
- **AND** both counts include `src/ui/palette.rs` and `src/ui/help.rs`, and both gates fail
  when either file is absent from their list rather than reporting a clean tree over the rest

#### Scenario: The enum's membership is exactly this list

- **WHEN** `palette::style` is called for every `Role` variant the enum above names
- **THEN** each returns a `Style`, and the three variants this change adds — `DeltaAdded`,
  `DeltaModified`, and `DeltaRemoved` — are among them, as are the five `tasks-emphasis`
  added before them
- **AND** the enum names no variant this requirement's reproduction omits, checked by an
  exhaustive `match` over `Role` in the test that would fail to compile if a variant were
  added without this block being updated
- **AND** that exhaustive `match` is the mechanism, not a hand-counted total: a count would go
  stale silently, and this requirement exists precisely because the enum's membership keeps
  moving

#### Scenario: The three delta roles carry their colour and no modifier

- **WHEN** `palette::style` is called for `Role::DeltaAdded`, `Role::DeltaModified`, and
  `Role::DeltaRemoved`
- **THEN** their foregrounds are `Green`, `Yellow`, and `LightRed`, and each `add_modifier`
  set is empty
- **AND** the assertion discriminates: `DeltaAdded` reports `Green` and not `LightRed`, so a
  table collapsing two operations onto one colour could not pass
- **AND** `DeltaRemoved` reports `LightRed` and **not** `Red`, so `ListProblem`'s red stays the
  pane's one problem colour

#### Scenario: The full set of shared coloured styles is still exactly five groups

- **WHEN** every coloured `Role` is compared pairwise against every other for `Style` equality
- **THEN** the equal groups are exactly the five the table above names, now including
  `DeltaAdded` in the `Green` group and `DeltaRemoved` in the `LightRed` group
- **AND** `DeltaModified` is equal to no other role's `Style`, `FileMode`, `Code`, and
  `Heading(5)` each differing from it by a modifier
- **AND** a sixth group appearing fails the test, so a future role sharing a style without a
  stated licence is caught rather than merged in silently

#### Scenario: A badged header row's colours survive the row's own role

- **WHEN** a frame is drawn at 120 columns with the detail region showing a `specs` tab whose
  selected section is a requirement carrying `Some(Added)`, and again at 60 columns in the
  detail route
- **THEN** the badge cell's foreground equals `palette::style(Role::DeltaAdded)`'s foreground
  at both widths
- **AND** that cell also carries `Modifier::REVERSED` from `Role::DetailSectionSelected`, which
  carries no foreground of its own and so cannot displace the badge's colour
- **AND** no colour literal appears anywhere in the test, the assertion comparing against
  `palette::style` rather than against `Color::Green`

#### Scenario: A delta badge and a clause keyword are the same style in one frame

- **WHEN** a frame is drawn at 120 columns, and again at 60 in the detail route, showing a
  `specs` tab whose sections are `## ADDED Requirements`, an **open** `### Requirement: A`, and
  a body holding `- **WHEN** the schema declares four artifacts`
- **THEN** the badge cell's `Style` and the `WHEN` keyword cells' `Style` are **equal** at both
  widths — both `Green`, both carrying whatever modifier their row contributes
- **AND** the assertion is an equality, deliberately: it documents the collision licence 3
  accepts rather than asserting a distinction that does not exist, so a future change that
  separates the two hues fails here and must revisit the licence
- **AND** both cells' foregrounds are compared against `palette::style(Role::DeltaAdded)` and
  `palette::style(Role::TaskChange)` respectively, never against a `Color` literal
- **AND** the two spans are distinguishable by position rather than by style: the badge is in
  the header row's prefix and the keyword opens a body list item, which is the whole content of
  licence 3's second conjunct

#### Scenario: `style_for` maps each `DeltaOp` to its own role

- **WHEN** `ui::view::style_for` is called on `Face { delta: Some(op), ..Face::plain() }` for
  each of `Added`, `Modified`, and `Removed`
- **THEN** the three results equal `palette::style(Role::DeltaAdded)`,
  `palette::style(Role::DeltaModified)`, and `palette::style(Role::DeltaRemoved)` respectively
- **AND** the three differ from one another, so a step-10 implementation mapping two operations
  onto one role could not pass — every other scenario in this change renders only `Added`, and
  a slip mapping `Modified` to `DeltaAdded` would otherwise ship green


#### Scenario: `Selected` reverses and colours nothing

- **WHEN** `palette::style(Role::Selected)` is read
- **THEN** it carries `Modifier::REVERSED`
- **AND** its `fg` and `bg` are both `None`, so the cell underneath keeps its own colour and
  only the two are swapped
- **AND** it is not equal to `palette::style(Role::SelectedRow)`, so a selected change row and
  a selected span never look alike

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
   `Some(role)`, selected by that `LabelRole`;
10. `DeltaAdded`, `DeltaModified`, or `DeltaRemoved`, when `face.delta` is `Some(op)`,
    selected by that `DeltaOp`.

`spec-emphasis` adds step 10, taking `style_for` from nine to **ten**. It is placed last, after
`face.label`, and the two are never both `Some` in production — a badge segment carries the
marker and nothing else, a clause keyword carries no badge — so the order between them is a
totality statement rather than a precedence decision. `style_for` SHALL answer the unreachable
combination with the delta colour rather than `debug_assert`ing against a caller it does not
control, on exactly the terms the paragraph below states for `muted` and `label`.

A delta badge is a `Face` and not a `ContentKind` by the same test the paragraph above applies:
the badge is a **run of text** within the header row — two columns of it — while the row's
`SectionHeader` kind covers the whole row including its label. Making it a kind would have
forced the row to be two rows or the kind to carry a sub-range, and the reason the badge can be
a face at all is that `artifact-folds` splits a badged header row into four segments — the
prefix, the badge, the label, and the plain-faced padding that fills the row.

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

**`text-selection` adds one entry to that mapping, and it composes rather than replaces.** A
cell inside the selected span SHALL be drawn as its own role's style **patched** with
`palette::style(Role::Selected)` — the "fixed composition of such styles" this requirement
already permits — never as `Role::Selected` alone. A selected heading stays a heading and a
selected code span stays code; only the foreground and background swap. Replacing the style
would erase every distinction the content area exists to draw.

#### Scenario: A selected cell keeps its own role and gains the reversal

- **WHEN** a dashboard whose selection covers part of a level-2 heading row and part of an
  inline code span is rendered at 120x20
- **THEN** each selected cell's style equals its unselected style patched with
  `palette::style(Role::Selected)`
- **AND** the heading cells still carry the heading role's modifiers and the code cells the
  code role's, so neither was flattened
- **AND** every unselected cell is byte-identical to the same frame with no selection
