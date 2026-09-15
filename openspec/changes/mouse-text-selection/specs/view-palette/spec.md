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
