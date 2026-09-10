## MODIFIED Requirements

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
}

pub fn style(role: Role) -> Style;
```

The enum is reproduced here because two changes in a row alter its membership. `pane-chrome`
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

`ui::palette` SHALL be in the pure view set both standing view gates already carry:
`NOIO-VIEW`'s `PURE` list of **nine** files and `COLWIDTH`'s of **eight**. This change adds
no module and moves neither count. Measured at HEAD `08025d3`: `NOIO-VIEW OK: 9 pure files`
and `COLWIDTH OK: … the eight pure view files`.

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
- **AND** the two deliberately shared pairs are asserted **equal** — `FileMode` with `Code`,
  and `AgentBadge(Unknown)` with `ListSeparator` — so the sharing is a recorded decision
  rather than a gap the distinctness assertion happens to step around, and `Strikethrough` is
  asserted **unequal** to every other role's style, so it joins neither pair by accident
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

#### Scenario: The palette module reaches no I/O and measures no width

- **WHEN** `make gates` runs on a tree carrying `src/ui/palette.rs`
- **THEN** `NOIO-VIEW` reports nine pure files carrying no I/O API
- **AND** `COLWIDTH` reports eight pure view files carrying no `char`-count measurement

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

`CROSSED_OUT` is chosen over a colour or a bracketing glyph for the same reason every other
row of this table carries a modifier: it is the terminal's own rendering of exactly this
meaning, it costs no columns, and a terminal that does not support it drops the attribute and
still shows the text — which is the right failure for a construct whose whole point is that
the text is still there.

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
- **THEN** every role matches, and the seven roles that carry no modifier —
  `Footer`, `ListRow`, `ListProblem`, `ListSeparator`, `ListMessage`, `AgentBadge`, and
  `TabInactive` — carry none, the two new roles having joined the modifier-carrying side and
  left that count at seven
- **AND** the assertion discriminates: `Emphasis` reports `ITALIC` and not `BOLD`,
  `Strikethrough` reports `CROSSED_OUT` and not `DIM`, and `DetailSectionSelected` reports
  `BOLD | REVERSED` and not `BOLD` alone

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

`Footer`, `RegionHeading`, `RegionHeadingFocused`, `RegionRule`, `ListRow`, `ListRowSelected`,
`ListMessage`, `DetailSection`, `DetailSectionSelected`, `Strong`, `Emphasis`, `Quoted`, and
`Strikethrough` SHALL carry **no** colour: each already carries a modifier that distinguishes
it, and a colour there would be decoration rather than information. `Quoted` in particular stays `DIM` and uncoloured. `Strikethrough` joins that
list rather than gaining an entry of its own: `CROSSED_OUT` already says the whole of what
the face means, and the obvious candidate colour — `DarkGray` — is this palette's one "no
information" grey, which struck text emphatically is not, since the reader is being shown
what it says as well as that it is struck. `DetailSection` and `DetailSectionSelected` join
it for the same reason: `BOLD` and `BOLD | REVERSED` already carry the whole distinction
between a fold header and the fold header the keys address, and this change's own design
records that adding a colour there would be decoration.

Two pairs of roles SHALL share a style, deliberately rather than by oversight. `FileMode` and
`Code` are both `DIM` + `Yellow`, and they cannot meet: one is drawn in the list region's
heading row, the other only inside the detail region's content area. `AgentBadge(Unknown)`
and `ListSeparator`
are both `DarkGray`, and they do share the list region — that is the point, because `DarkGray`
is this palette's one "no information" grey and an unknown agent status and a divider rule are
both exactly that. Neither pair is a distinction the reader must draw, so neither is a
`DIM`-style overload. `Strikethrough` SHALL be a **third** style equal to no other role's, and
`DetailSectionSelected` a **fourth**, so the shared set stays exactly those two pairs.

`Heading(l)` for an `l` outside `1..=6` SHALL return the same `Style` as `Heading(6)`:
`markdown-render` produces only `1..=6`, and the function is total rather than panicking on a
value the parser cannot emit.

#### Scenario: The coloured set is exactly the table above

- **WHEN** `palette::style` is called for every `Role` variant and each result's `fg` and `bg`
  are inspected
- **THEN** exactly the roles in the table above report a `Some` foreground or background, with
  the named variant the table gives
- **AND** every other role reports `fg: None` and `bg: None`, `Strikethrough`,
  `DetailSection`, and `DetailSectionSelected` among them

#### Scenario: An out-of-range heading level does not panic

- **WHEN** `palette::style(Role::Heading(0))`, `Role::Heading(7)`, and `Role::Heading(255)`
  are called
- **THEN** none panics and each returns the same `Style` as `Role::Heading(6)`

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
carry this distinction — `Face` is seven markdown-construct flags and a section header is not
a markdown construct — which is why the role is selected by kind here rather than folded into
`style_for`. `style_for` itself is unchanged: it still takes a `Face`, still composes exactly
the seven face roles below, and gains no eighth.

`ui::view::style_for(face: &markdown::Face) -> Style` SHALL compose the palette's face roles
by folding them onto `Style::default()` with `Style::patch` in this fixed order:

1. `Quoted`, when `face.quoted`;
2. `Strikethrough`, when `face.strikethrough`;
3. `Link`, when `face.link`;
4. `Code`, when `face.code`;
5. `Emphasis`, when `face.emphasis`;
6. `Strong`, when `face.strong`;
7. `Heading(level)`, when `face.heading` is `Some(level)`.

`Strikethrough` is inserted at position 2 — `markdown-constructs`' only edit to the order —
precisely because it carries **no** foreground: wherever it sits it cannot take a colour away
from a role that has one, so it is placed early, beside the other uncoloured,
always-composing face.

Because `patch` lets the later value win, modifiers accumulate — a bold link's cells carry
`BOLD` and `UNDERLINED` together, and a struck bold link's carry `CROSSED_OUT` as well —
while the **foreground** of a span carrying several coloured faces is decided by the last one
in that order. The precedence is therefore heading over code over link, stated here rather
than left to be discovered: a heading line reads as one colour even where it contains a code
span or a link, which is the point of colouring the heading at all.

`style_for` SHALL be total: no `Face` value panics, and `Face::plain()` SHALL map to
`Style::default()`.

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
