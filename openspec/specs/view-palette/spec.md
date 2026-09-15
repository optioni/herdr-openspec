# view-palette Specification

## Purpose
Holds the crate's one semantic-role table: `ui::palette`, a pure total module under `src/ui/`
mapping a `Role` — a region's heading, bold when that region is routed and dim otherwise, the
rule that separates the two wide-layout regions and underlines the detail region's tab bar, a
`file mode` badge, a list row and its degraded kinds, an agent badge per `AgentStatus`, an
active or inactive tab chip, and the markdown faces — to a `ratatui::style::Style`, and
holding nothing else. It fixes three things
that were previously written out at render call sites: which modifier each role carries, which
roles gain a colour and which deliberately carry none, and that a colour is always one of
`ratatui::style::Color`'s sixteen **named** ANSI variants, never an `Rgb` triple, an
`Indexed`, or a `Reset` — so the reader's own terminal theme decides what `Red` looks like, a
16-colour terminal renders the pane correctly, and the pane does not fight the theme of the
panes beside it in the same Herdr workspace. The palette probes no terminal, reads no
`NO_COLOR`, and branches on no capability: it declares an index and lets the terminal answer.
`src/ui/palette.rs` is therefore the only file under `src/` permitted to name a `Color` at all,
on the same terms `src/cli.rs` is the crate's only process spawner and `src/ui/markdown.rs` its
only `pulldown_cmark` caller, enforced the same tree-wide-grep-with-a-positive-control way.
Applying a role to a span is `ui::view`'s — including `style_for`, which folds a
`markdown::Face`'s several flags onto one `Style` in a fixed order — and which span carries
which role belongs to the drawing capability that owns it.

## Requirements

### Requirement: One module maps every semantic role to a `Style`, and it is the crate's only `Color`

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

### Requirement: Colour is a named ANSI index, never an RGB triple

Every `Color` the palette names SHALL be one of `ratatui::style::Color`'s **named** ANSI
variants — `Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `Gray`, `DarkGray`,
`LightRed`, `LightGreen`, `LightYellow`, `LightBlue`, `LightMagenta`, `LightCyan`, `White`.
`Color::Rgb`, `Color::Indexed`, and `Color::Reset` SHALL NOT appear.

The reason is stated rather than inferred: the reader's own terminal theme decides what
`Red` looks like, a 16-colour terminal renders the pane correctly, and the pane does not
fight the theme of the panes beside it in the same Herdr workspace.

The pane SHALL NOT probe the terminal for colour support, SHALL NOT read `NO_COLOR` or any
other environment variable, and SHALL NOT branch on a capability: it declares an ANSI index
and lets the terminal answer. Reading the environment from a view file is forbidden by
`NOIO-VIEW` in any case, so this is a property the existing gate already enforces.

#### Scenario: No RGB, indexed, or reset colour is named

- **WHEN** `src/ui/palette.rs` is searched for `Color::Rgb`,
  `Color::Indexed`, and `Color::Reset`
- **THEN** none of the three appears
- **AND** `palette::style` returns, for every role that carries one, a foreground or
  background equal to one of the sixteen named variants

#### Scenario: The palette reads nothing from the environment

- **WHEN** `src/ui/palette.rs` is searched for `std::env`, `NO_COLOR`, and `var(`
- **THEN** none appears, so setting `NO_COLOR` changes nothing about what the palette returns

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
| `DeltaAdded` | none |
| `DeltaModified` | none |
| `DeltaRemoved` | none |

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

The three **delta** roles carry no modifier either, and they are the one group in this table
where that costs a monochrome reader **nothing**. Their span is the badge marker itself — `+`,
`~`, or `-` — so the distinction is already in the glyph, and colour is doing what this table
everywhere else asks it to do: arriving strictly beside a signal that survives without it. The
badge was specified as a glyph rather than as a recoloured heading for exactly this reason
(`spec-emphasis` -> proposal.md -> Review Decisions, 1), which is what lets these three be
added without weakening the guarantee the scenario below asserts.

A modifier here would also be wrong on its own terms. `BOLD` would compete with
`Role::DetailSection`, which every badged row already carries, and `CROSSED_OUT` is spoken for:
a `Removed` requirement's **label** segment carries `Strikethrough` — the existing role, on the
existing field — while its badge segment carries `DeltaRemoved`, so the two say different
things in the same row and must not wear the same attribute.

`spec-emphasis` adds **three** rows — `DeltaAdded`, `DeltaModified`, `DeltaRemoved`, each
carrying no modifier — and alters none. The only cells that gain a modifier are a `Removed`
requirement's own **heading label**, which gains `CROSSED_OUT` beside the `BOLD` its header row
already carried (`artifact-folds` -> "A section header row names the file and shows its fold
state"). That cell could not be drawn before this change, so no cell that existed has moved.

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
- **THEN** every role matches, and the **fourteen** roles that carry no modifier — `Footer`,
  `ListRow`, `ListProblem`, `ListSeparator`, `ListMessage`, `AgentBadge`, `TabInactive`,
  `TaskEvidence`, `TaskChange`, `TaskConfirm`, `TaskLabel`, `DeltaAdded`, `DeltaModified`, and
  `DeltaRemoved` — carry none, `Muted` having joined the modifier-carrying side and the four
  label roles and three delta roles the other
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
- **AND** a `specs` tab whose sections carry `operation: Some(Removed)` renders `CROSSED_OUT`
  on exactly that requirement's **heading label** cells and on no other cell in the frame — not
  on its badge, not on its body, and not on a sibling requirement carrying `Some(Added)`. This
  is the one cell `spec-emphasis` adds a modifier to, and the fixture above — single-section,
  not spec-shaped — cannot reach it, which is why this clause names its own dashboard
- **AND** the same dashboard whose selected artifact resolves to **three** files instead of
  one renders `REVERSED` on exactly one row — the cursor's own section header — and on no
  cell anywhere else, so a single-file artifact's frame is untouched by the two new roles

#### Scenario: The five new roles leave every existing cell's modifier where it was

- **WHEN** the `color-palette` monochrome fixture above — a repository in file mode, one
  problem row, three active changes one of which is badged `Working`, and a selected change
  whose single-section content is `## Heading\n\n**bold** and *italic* and `code` and
  [link](u)\n` — is rendered at 120x20 and at 60x20
- **THEN** no cell of the detail region's content area carries `DIM` except the `code` span's,
  which `Role::Code` has always carried — that fixture's selected artifact does not track
  tasks, so no cell carries any of the five new roles. The **byte-for-byte** half of this
  claim is carried by "A monochrome reading of the frame is unchanged", whose per-cell
  modifier assertions across both regions at both widths this change leaves **unmodified**;
  that is what makes it falsifiable, and it is stated here rather than promised as a
  comparison against a pre-change buffer, which this repository has no mechanism to record
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
| `DeltaAdded` | foreground `Green` |
| `DeltaModified` | foreground `Yellow` |
| `DeltaRemoved` | foreground `LightRed` |

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

The three **delta** roles take a colour on the same grounds and one stronger one: they colour a
one-character marker, and a marker is the shortest span in this palette. `+`, `~`, and `-` are
distinguishable without colour — which is the whole reason `spec-emphasis` chose a glyph — so
colour here is redundancy rather than the only signal, and the three are the palette's clearest
case of colour arriving strictly beside something that survives without it.

The hues are the ones a reader has already learned from every diff they have read: `Green` for
added, `Yellow` for changed, and red for removed. `DeltaRemoved` takes `LightRed` and **not**
`Red`, for exactly the reason `TaskEvidence` does: a reader scans the pane for exactly one red
thing, and `Red` means "a problem" — `ListProblem` and nothing else. A removed requirement is
not a problem; it is the ordinary content of a delta spec, and 15 of them exist across the
archive. Spending the problem colour on it would make the scan unreliable for a row that is
never an error.

Roles MAY share a style, but only under one of three stated licences, and a share outside all three
is a defect rather than a decision. **The rule ranges over the roles that carry a colour**,
which is the set the table above enumerates. Plain-modifier equality among uncoloured roles is
not policed and never was — `Footer`, `ListRow`, and `ListMessage` are all `Style::default()`;
`RegionHeadingFocused`, `ListRowSelected`, `DetailSection`, and `Strong` are all plain `BOLD`;
`RegionHeading`, `RegionRule`, `Quoted`, and now `Muted` are all plain `DIM` — and the
requirement above says so in as many words. Stating the range is this change's repair of a
sentence that read as a claim about every role while three such groups already existed.

1. **They cannot meet.** Two roles that are never drawn in the same region may share a style,
   because no reader is ever asked to tell them apart. `FileMode` and `Code` are both
   `DIM` + `Yellow` under this licence: one is drawn in the list region's heading row, the
   other only inside the detail region's content area.
2. **They mean the same thing.** `DarkGray` is this palette's one "no information" grey, and
   every role wearing it means exactly that. `AgentBadge(Unknown)`, `ListSeparator`, and — as
   of `tasks-emphasis` — `TaskLabel` all carry it: an unknown agent status, a divider rule, and
   a label the crate declines to classify are the same statement three times.

This replaces the enumeration this requirement previously carried ("exactly two pairs"), which
`tasks-emphasis` would otherwise have had to grow with no principle to grow it by. The
enumeration is restated as a consequence rather than a rule: among the coloured roles, the
full set of shared styles SHALL be exactly these five groups, and no other coloured pair SHALL
be equal:

| Shared `Style` | Roles | Licence |
|---|---|---|
| `DIM` + `Yellow` | `FileMode`, `Code` | 1 |
| `DarkGray` foreground | `ListSeparator`, `AgentBadge(Unknown)`, `TaskLabel` | 2 |
| `Green` foreground, no modifier | `AgentBadge(Working)`, `TaskChange`, `DeltaAdded` | 1 and 3 |
| `Blue` foreground, no modifier | `AgentBadge(Done)`, `TaskConfirm` | 1 |
| `LightRed` foreground, no modifier | `AgentBadge(Blocked)`, `TaskEvidence`, `DeltaRemoved` | 1 and 3 |

`DeltaModified`'s `Yellow` is a style **no** other role carries and joins no group: `FileMode`
and `Code` are `DIM` + `Yellow` and `Heading(5)` is `BOLD` + `Yellow`, so all three differ from
it by a modifier. It is a colour reuse and not a style share, exactly as `Heading(3)` and
`Link` are for `Blue`.

`DeltaAdded` and `DeltaRemoved` share a style with roles they **can** meet, and are licensed
under a third licence stated here for the first time:

3. **They cannot be confused.** Two roles may share a style when each span's own text carries
   its full meaning without colour **and** the two never occupy the same row. A `+` marker in a
   header row's prefix and a `WHEN` at the head of a body list item satisfy both: the marker
   says "added" and the word says "when" whatever colour either is painted, and no row holds
   both.

The licence is worded narrowly on purpose. "Colour is redundant here" would license almost any
share in this table — nearly every span carries a modifier or self-describing text — and would
reproduce the failure this requirement already records for the enumeration it replaced: no
principle to grow it by. Both conjuncts are load-bearing, and a future share satisfying only
one is a defect rather than a decision.

**The reader cost this licence accepts, stated rather than hidden:** on a delta spec tab the
green `+` badge and every green `WHEN` in the open bodies are the same hue, so scanning that tab
by colour for "the added things" does not work. `WHEN` occurs 3877 times across the corpus, so
this is the common case and not a corner. What survives is the marker column, which is where a
reader looks for the operation and where no clause keyword is ever drawn.

An earlier draft of this requirement licensed the same two shares under **licence 1**, claiming
a task label is drawn "only in the detail region on the tracked-tasks tab". `spec-emphasis`'
own `markdown-render` delta falsifies that: a clause keyword sets `Face::label` on **every**
markdown source with no spec-shape test (design.md -> Decision 11), so a `- **WHEN**` in a
delta spec's body is drawn on the **specs** tab, the same tab the badge is on, in the same
frame. This requirement has been wrong in exactly this way before — it already records that
`TaskChange`'s `Green` and `TaskConfirm`'s `Blue` "**can** appear in one frame's content area —
an earlier draft of this requirement claimed they could not, and planning review falsified it".
Twice now the error has been a "cannot meet" asserted without a scenario that renders both, and
the scenario below is what makes the third claim falsifiable rather than merely careful.

`Heading(3)`, `Heading(4)`, `Heading(6)`, and `Link` are deliberately **absent** from that
table even though they carry `Blue`, `Green`, `DarkGray`, and `Blue` respectively: each also
carries a modifier the label roles do not, so none is an equal `Style` and none is a share.
They are still a **colour** reuse, and the paragraph below is about that weaker relation,
which a reader sees and a `Style` comparison does not.

Each `AgentBadge` pairing in those groups is licence 1, the badges being list-region only; each
`Delta*`-against-`Task*` pairing is licence 3, the two being reachable in one frame on a specs
tab. The `DarkGray` trio is licence 2. The three
that matter are worth spelling out because the obvious objection is the one this change was
asked to answer. `TaskEvidence` takes `LightRed` and **not** `Red`, and the reason is recorded
in the form the implementation left it in rather than the form this requirement first stated.

The first draft said `Red` would have been an unlicensed share because `ListProblem` is drawn
in the detail region, the same region a task label is drawn in. That is **false**:
`ui::view::detail_row_role` answers `ContentKind::Problem` with no role, so a detail-region
problem row is `Style::default()`, and `ListProblem`'s red is reached only from `row_role`, in
the list region. Both colours are therefore licensed under licence 1, and the choice is a
choice rather than a forced move. `LightRed` is taken and recorded for two reasons that
survive the correction: a label and an `AgentBadge(Blocked)` can never meet, where a label and
a `ListProblem` row **can** — at 120 columns both regions are drawn in one frame — so
`LightRed` is the share that costs a reader nothing; and a reader scans the pane for exactly
one red thing, so leaving plain `Red` to mean "a problem" and nothing else keeps that scan
reliable. `AgentBadge(Blocked)`, which `LightRed` does collide with, is drawn only in the list
region. `TaskChange`'s `Green` and `TaskConfirm`'s `Blue` reuse `Heading(4)`'s and `Heading(3)`'s
**colours**, and the two **can** appear in one frame's content area — an earlier draft of this
requirement claimed they could not, and planning review falsified it against
`src/ui/app.rs:1527`. A tasks file whose text begins at its single `##` heading has no
preamble, so `contributions == 1`, so it does not split, so the tab is **not foldable**, so
`ui::detail::content_lines` reaches `ui::tasks::lines` — whose `heading_line` emits
`Face { heading }` on the heading row while `items` emits label segments on the rows below it.
`src/ui/detail.rs`'s own comment already named that case.

No licence is required, because neither pair is a **share**: `Heading(3)` and `Heading(4)`
carry `BOLD` and the label roles carry no modifier, so the four are four distinct `Style`s and
the rule above — which ranges over `Style` equality — does not reach them. A reader who sees
only hue sees a reused colour; a reader who sees the row sees a bolded heading against an
unbolded leading token, on different rows. That is why `Heading(3)` and `Heading(4)` are
asserted **alone** in their groups by the scenario below: a table that dropped `BOLD` from
either would turn a colour reuse into an unlicensed share, and that assertion is what catches
it. `Link` never arises at all: `ui::tasks` emits no link face for any input.

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
  heading levels 1 through 6; the results carrying **no** foreground and no background are
  discarded; and the rest are grouped by equal `Style`
- **THEN** the groups of size greater than one are exactly the five the table above lists,
  member for member, and no other coloured group has size greater than one
- **AND** the discarded, uncoloured roles are exactly `Footer`, `RegionHeading`,
  `RegionHeadingFocused`, `RegionRule`, `ListRow`, `ListRowSelected`, `ListMessage`,
  `DetailSection`, `DetailSectionSelected`, `Strong`, `Emphasis`, `Quoted`, `Strikethrough`,
  and `Muted` — asserted by name, so a role that silently loses its colour is caught here
  rather than passing as "uncoloured and therefore out of scope"
- **AND** `Heading(3)` and `Heading(4)` are each alone in their group, because `BOLD`
  separates them from `TaskConfirm` and `TaskChange`; a table that dropped the `BOLD` from
  either heading would fail this assertion
- **AND** `TaskEvidence`'s group holds `AgentBadge(Blocked)` and **not** `ListProblem`, which
  is the distinction between a licensed share and an unlicensed one for this change

#### Scenario: A task label and a problem row are distinguishable in one frame

The scenario's name is kept verbatim because a delta's scenario headers are its merge key. Its
subject is corrected: an earlier draft asserted the problem row's cells carry `ListProblem`'s
foreground **at both widths**, which the implementation falsified. `ui::view::detail_row_role`
answers `ContentKind::Problem` with **no** role, so a problem row drawn inside the detail
region carries `Style::default()`; `ListProblem`'s red is reached only from `row_role`, in the
**list** region. At `Route::Detail` and 60 columns the list region is not drawn at all, so the
120-column frame is the only one in which both constructs appear.

- **WHEN** a `Dashboard` at `Route::Detail` whose `changes.problems` carries one entry — a
  **list**-region problem row — and whose tracked-tasks artifact holds the unchecked item
  `- [ ] 1.1 RED: write the test` is rendered at 120x20 and at 60x20
- **THEN** in the 120-column frame, where both regions are drawn, the list region's problem
  row's cells carry `palette::style(Role::ListProblem)`'s foreground and the `RED:` cells carry
  `palette::style(Role::TaskEvidence)`'s
- **AND** those two foregrounds are not equal, so the two constructs that can appear in one
  frame are distinguishable by colour and not only by shape
- **AND** in the 60-column frame the list region is not drawn, so no cell carries
  `ListProblem`'s foreground at all, and the `RED:` cells still carry `TaskEvidence`'s — which
  is the assertion that would fail if the narrow layout ever drew both regions without this
  scenario being revisited
- **AND** a problem row drawn in the **detail** region — `detail.problems` — carries neither
  foreground, because `content_lines` gives it `ContentKind::Problem` and `ui::view` maps that
  kind to no role. Asserted here rather than left implicit, since it is the fact this
  scenario's first draft got wrong
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
