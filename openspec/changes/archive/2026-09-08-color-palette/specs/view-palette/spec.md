## ADDED Requirements

### Requirement: One module maps every semantic role to a `Style`, and it is the crate's only `Color`

`ui::palette` SHALL be a new module under `src/ui/`, holding one table from a **semantic
role** to a `ratatui::style::Style` and nothing else. It SHALL be a pure total module: no
filesystem, process, environment, network, or standard-I/O API, no clock, no global state,
and no panic for any input.

```rust
pub enum Role {
    HeaderTitle,
    HeaderPath,
    FileMode,
    Footer,
    RegionBorder,
    RegionBorderFocused,
    ListRow,
    ListRowSelected,
    ListProblem,
    ListSeparator,
    ListMessage,
    AgentBadge(crate::agents::AgentStatus),
    DetailHeader,
    TabActive,
    TabInactive,
    Heading(u8),
    Strong,
    Emphasis,
    Code,
    Link,
    Quoted,
}

pub fn style(role: Role) -> Style;
```

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
against, never a literal that test writes.

`ui::palette` SHALL be added to the pure view set both standing view gates already carry:
`NOIO-VIEW`'s `PURE` list (eight files becoming **nine**) and `COLWIDTH`'s (seven becoming
**eight**).

#### Scenario: The palette answers every role with a `Style`

- **WHEN** `palette::style` is called once for every `Role` variant, including
  `AgentBadge` for each of the five `agents::AgentStatus` values and `Heading` for levels
  `1` through `6`
- **THEN** every call returns a `Style` and none panics
- **AND** the role list the test iterates is built from an **exhaustive** `match role { … }`
  rather than hand-enumerated, so a `Role` added later fails to compile until it is added here
- **AND** no two of `ListProblem`, `FileMode`, `TabActive`, `TabInactive`, and the five
  `AgentBadge` styles are equal to one another, so each carries a distinction rather than
  repeating its neighbour
- **AND** the two deliberately shared pairs are asserted **equal** — `FileMode` with `Code`,
  and `AgentBadge(Unknown)` with `ListSeparator` — so the sharing is a recorded decision
  rather than a gap the distinctness assertion happens to step around

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
and this change SHALL add, remove, or alter **no modifier anywhere**. The consequence is
falsifiable rather than aspirational: in a captured `TestBackend` buffer compared by modifier
alone, every cell **outside the artifact tab-bar row** after this change carries exactly the
modifier it carried before it.

The tab-bar row is the stated exception, because `artifact-tabs` moves and relabels every
chip in it. Its own guarantee is narrower and is stated here rather than left out: the
selected chip's span remains that row's only `BOLD` span, so a monochrome reader still learns
which tab is current — what that reader loses is the `1`–`9` digits, which the chip grammar
drops deliberately and `action_for` still answers.

The modifier each role SHALL carry:

| Role | Modifiers |
|---|---|
| `HeaderTitle` | `BOLD` |
| `HeaderPath` | none |
| `FileMode` | `DIM` |
| `Footer` | none |
| `RegionBorder` | none |
| `RegionBorderFocused` | `BOLD` |
| `ListRow` | none |
| `ListRowSelected` | `BOLD` |
| `ListProblem` | none |
| `ListSeparator` | none |
| `ListMessage` | none |
| `AgentBadge(_)` | none |
| `DetailHeader` | `BOLD` |
| `TabActive` | `BOLD` |
| `TabInactive` | none |
| `Heading(_)` | `BOLD` |
| `Strong` | `BOLD` |
| `Emphasis` | `ITALIC` |
| `Code` | `DIM` |
| `Link` | `UNDERLINED` |
| `Quoted` | `DIM` |

#### Scenario: Each role's modifier set is exactly the table above

- **WHEN** `palette::style` is called for every `Role` variant and its `add_modifier` set is
  compared against the table
- **THEN** every role matches, and the nine roles that carry no modifier —
  `HeaderPath`, `Footer`, `RegionBorder`, `ListRow`, `ListProblem`, `ListSeparator`,
  `ListMessage`, `AgentBadge`, and `TabInactive` — carry none
- **AND** the assertion discriminates: `Emphasis` reports `ITALIC` and not `BOLD`

#### Scenario: A monochrome reading of the frame is unchanged

- **WHEN** a `Dashboard` carrying a repository in file mode, one problem row, three active
  changes of which one is badged `Working`, and a selected change whose `detail.source` is
  `## Heading\n\n**bold** and *italic* and `code` and [link](u)\n` is rendered at 120x20 and
  at 60x20
- **THEN** in both buffers the modifier of every cell **outside row 3, the tab bar** is
  exactly what the same dashboard produced before this change: `OpenSpec` and the detail header `BOLD`, the `file mode` badge
  `DIM`, the selected row's cells `BOLD`, the heading and `bold` `BOLD`, `italic` `ITALIC`,
  `code` `DIM`, and `link` `UNDERLINED`
- **AND** the problem row, the separator row, and the agent badge cell carry no modifier at
  all, exactly as before
- **AND** in the tab-bar row the selected chip's span is the only `BOLD` span, so the one
  excepted row still discriminates the current tab without colour

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

`HeaderTitle`, `HeaderPath`, `Footer`, `RegionBorder`, `RegionBorderFocused`, `ListRow`,
`ListRowSelected`, `ListMessage`, `DetailHeader`, `Strong`, `Emphasis`, and `Quoted` SHALL
carry **no** colour: each already carries a modifier that distinguishes it, and a colour
there would be decoration rather than information. `Quoted` in particular stays `DIM` and
uncoloured — a strikethrough face has no entry at all, because `markdown-render` has no
parser support for one yet.

Two pairs of roles SHALL share a style, deliberately rather than by oversight. `FileMode` and
`Code` are both `DIM` + `Yellow`, and they cannot meet: one is drawn in the frame header, the
other only inside the detail region's content area. `AgentBadge(Unknown)` and `ListSeparator`
are both `DarkGray`, and they do share the list region — that is the point, because `DarkGray`
is this palette's one "no information" grey and an unknown agent status and a divider rule are
both exactly that. Neither pair is a distinction the reader must draw, so neither is a
`DIM`-style overload.

`Heading(l)` for an `l` outside `1..=6` SHALL return the same `Style` as `Heading(6)`:
`markdown-render` produces only `1..=6`, and the function is total rather than panicking on a
value the parser cannot emit.

#### Scenario: The coloured set is exactly the table above

- **WHEN** `palette::style` is called for every `Role` variant and each result's `fg` and `bg`
  are inspected
- **THEN** exactly the roles in the table above report a `Some` foreground or background, with
  the named variant the table gives
- **AND** every other role reports `fg: None` and `bg: None`

#### Scenario: An out-of-range heading level does not panic

- **WHEN** `palette::style(Role::Heading(0))`, `Role::Heading(7)`, and `Role::Heading(255)`
  are called
- **THEN** none panics and each returns the same `Style` as `Role::Heading(6)`

### Requirement: `ui::view` takes every style it applies from the palette

`ui::view` SHALL construct no `Style` of its own: every span it writes to the buffer SHALL be
`palette::style(role)` for the role that span carries, or a fixed composition of such styles.
The mapping from a drawn span to its role SHALL be:

- the frame header's `OpenSpec` label → `HeaderTitle`;
- the `file mode` badge → `FileMode`;
- the right-aligned repository path or `no repository` → `HeaderPath`;
- the footer row, in all three of its forms → `Footer`. It is named rather than left as a
  bare `Style::default()` so the requirement below — that `ui::view` constructs no `Style` of
  its own — is true of the whole file rather than of the functions this change happened to
  visit;
- a region's border → `RegionBorderFocused` when that region is the routed one, else
  `RegionBorder`;
- a list row → `ListRowSelected` when `Row::selected`, else `ListProblem`, `ListSeparator`,
  or `ListMessage` by its `RowKind`, else `ListRow`;
- a badged change row's badge cell → the row's own style patched with
  `AgentBadge(status)`, so a badge on the selected row is coloured **and** bold;
- the detail region's change header → `DetailHeader`;
- an artifact tab chip → `TabActive` when `Tab::selected`, else `TabInactive`;
- a rendered content segment → `style_for(&segment.face)`, below.

`ui::view::style_for(face: &markdown::Face) -> Style` SHALL compose the palette's face roles
by folding them onto `Style::default()` with `Style::patch` in this fixed order:

1. `Quoted`, when `face.quoted`;
2. `Link`, when `face.link`;
3. `Code`, when `face.code`;
4. `Emphasis`, when `face.emphasis`;
5. `Strong`, when `face.strong`;
6. `Heading(level)`, when `face.heading` is `Some(level)`.

Because `patch` lets the later value win, modifiers accumulate — a bold link's cells carry
`BOLD` and `UNDERLINED` together, exactly as before — while the **foreground** of a span
carrying several coloured faces is decided by the last one in that order. The precedence is
therefore heading over code over link, stated here rather than left to be discovered: a
heading line reads as one colour even where it contains a code span or a link, which is the
point of colouring the heading at all.

`style_for` SHALL be total: no `Face` value panics, and `Face::plain()` SHALL map to
`Style::default()`.

#### Scenario: Faces reach the buffer as coloured styles at both mandated widths

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change carries one artifact not
  marked `tracks_tasks` and whose `detail.source` is
  `# Title\n\n## Heading\n\n**bold** and *italic* and `code` and [link](u)\n` is rendered at
  120x20 and at 60x20
- **THEN** in each buffer the cells of `# Title` report `BOLD` set and the foreground
  `Role::Heading(1)` carries (`Magenta`), and the cells of `## Heading` report `BOLD` set and
  the foreground `Role::Heading(2)` carries (`Cyan`)
- **AND** the cells of `code` report `DIM` and the foreground `Role::Code` carries (`Yellow`),
  and the cells of `link` report `UNDERLINED` and the foreground `Role::Link` carries
  (`Blue`)
- **AND** the cells of `bold` report `BOLD` with no foreground, and of `italic` `ITALIC` with
  no foreground, so the uncoloured roles are discriminated from the coloured ones

#### Scenario: Heading foreground wins over a code span inside it

- **WHEN** `style_for` is called on a `Face` with `heading: Some(2)` and `code: true`, and on
  one with `code: true` and `link: true`
- **THEN** the first reports the foreground `Role::Heading(2)` carries — the heading's — with
  `BOLD` and `DIM` both set, so no modifier was lost to the precedence rule
- **AND** the second reports the foreground `Role::Code` carries — which follows the link in
  the fold order — with `DIM` and `UNDERLINED` both set
- **AND** neither assertion names a `Color` literal: both compare against `palette::style`,
  because `style_for` lives in `src/ui/view.rs`, which the confinement gate searches

#### Scenario: A plain face is the default style

- **WHEN** `style_for(&Face::plain())` is called
- **THEN** it returns `Style::default()`, with no modifier, no foreground, and no background
- **AND** rendering a plain-text document at 120x20 and 60x20 leaves every content cell's
  style equal to `ratatui::buffer::Cell::default().style()`
