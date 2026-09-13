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
`ListRow`, and its scroll indicator is `ListSeparator`. Reusing six existing roles rather
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
