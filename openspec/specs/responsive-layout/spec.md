# responsive-layout Specification

## Purpose
The dashboard's outer frame and how it reflows with terminal width: a body of one or two
borderless regions and a footer of key hints dropped whole from the end rather than
truncated, with explicit branches for degenerate heights and one-column frames. There is no
header row — each region is instead a heading row, a blank padding row, and a
gutter-padded interior, and the routed region's heading is bold where the other's is dim,
the same distinction a bold border once carried. It owns the 100-column breakpoint that
decides whether the body holds a 40-column `Changes` region beside a `Detail` one, the two
separated by a single vertical rule that belongs to neither, or a single region showing only
the routed side; that content never bleeds across a gutter or that rule; and the two
interior widths — 78 at a 120-column frame and 58 at a 60-column one — that the detail-side
capabilities render into. What fills those interiors belongs to `change-rows`,
`detail-header`, `artifact-tabs`, and `artifact-content`.

## Requirements

### Requirement: The 100-column breakpoint decides one region or two

`ui::layout::WIDE_MIN_WIDTH` SHALL be `100`. `ui::layout::mode(width: u16)` SHALL return
`LayoutMode::Wide` when `width >= WIDE_MIN_WIDTH` and `LayoutMode::Narrow` otherwise, and
SHALL be a total function over every `u16`.

At `LayoutMode::Wide` the body SHALL be split horizontally into exactly three parts —
`Constraint::Length(40)` for the change list, `Constraint::Length(1)` for the divider, and
`Constraint::Min(0)` for the artifact detail — so that every column gained beyond 100 goes to
the detail side. Both regions SHALL be drawn, each as a heading row, a padding row, and a
gutter-padded interior, and neither as a bordered block. The list region takes
`Gutters::Both` and the detail region `Gutters::LeftOnly`. Their headings are not fixed
titles: the list region's heading names the repository directory and the detail region's
heading is the selected change's own header.

At `LayoutMode::Narrow` the body SHALL hold exactly one region occupying the whole body
width with `Gutters::Both`, drawn the same borderless way, and it SHALL be the list region
when the dashboard's route is `Route::List` and the detail region when it is
`Route::Detail`. The region that is not routed to SHALL NOT be drawn at all, and no divider
SHALL be drawn.

The literal titles `Changes` and `Detail` are gone with the borders that carried them. They
named the two halves of a split the reader can already see, and at a 60-column pane each
spent a row saying which of the two routes was showing — which the footer's `Enter detail` /
`Esc back` hints and the heading's own content already say.

The mode SHALL be derived from the frame area passed to `render` on every draw, and SHALL
NOT be stored on `Dashboard` or captured at startup, so a terminal resized across the
breakpoint changes layout on its next frame with no extra state.

#### Scenario: At 120 columns both regions are drawn with the divider at column 40

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  and one active change `alpha` is rendered at 120x20, and — as the contrasting control at
  the mandated narrow width — at 60x20
- **THEN** in the 120-column buffer row 0 spells `demo-repo` from column 1 and `alpha`'s own
  change header from column 42, so both regions drew their heading rows
- **AND** in the 120-column buffer every cell of column 40 in rows 0 through 18 is `│`, and
  columns 0, 39, and 41 are spaces in every one of those rows
- **AND** the strings `Changes` and `Detail` appear in no cell of either buffer
- **AND** the 60-column buffer holds no `│` at all, so the divider is a width branch rather
  than something drawn unconditionally

#### Scenario: At 60 columns only the routed region is drawn

- **WHEN** the same `Dashboard` with `route: Route::List` is rendered at 60x20, and — as
  the contrasting control at the mandated wide width — at 120x20
- **THEN** in the 60-column buffer row 0 spells `demo-repo` from column 1 and the change's
  own header appears in no row, because the detail region was not drawn
- **AND** in the 60-column buffer no cell holds `│`
- **AND** the 120-column buffer does hold the change's header at row 0 column 42, so the
  absence at 60 columns is the breakpoint and not the detail heading being missing

#### Scenario: At 60 columns the detail route replaces the list region

- **WHEN** the same `Dashboard` with `route: Route::Detail` is rendered at 60x20
- **THEN** row 0 spells the selected change's header from column 1
- **AND** the string `demo-repo` appears in no row of the buffer, because the list region —
  and with it the repository heading — is not drawn
- **AND** rendering the same dashboard at 120x20 still shows **both** `demo-repo` at row 0
  column 1 and the change's header at row 0 column 42, because the route selects emphasis
  rather than visibility above the breakpoint

#### Scenario: The breakpoint is exact at 99, 100, and 101 columns

- **WHEN** `ui::layout::mode` is called with `0`, `1`, `40`, `60`, `99`, `100`, `101`,
  `120`, and `u16::MAX`
- **THEN** it returns `Narrow` for `0`, `1`, `40`, `60`, and `99`, and `Wide` for `100`,
  `101`, `120`, and `u16::MAX`
- **AND** rendering the same `Dashboard` produces no `│` in the buffer at 60x20 and at
  99x20, and a full column of `│` at column 40 at 100x20, at 101x20, and at 120x20, so both
  mandated widths and all three boundary widths are rendered, not just computed

#### Scenario: The mode follows the current frame, not the startup size

- **WHEN** one `Terminal<TestBackend>` is created at 120x20, a frame is drawn, the backend
  is resized to 60x20, and a second frame is drawn from the **same** unchanged `Dashboard`
- **THEN** the first buffer holds a column of `│` at column 40 and the second holds no `│`
  at all
- **AND** `Dashboard` exposes no field naming a width, a layout mode, or a column count

### Requirement: The routed region is emphasised and region interiors are left empty

The region the dashboard's route names SHALL have its **heading row** drawn with
`Modifier::BOLD` set and `Modifier::DIM` clear; the other region, when drawn, SHALL have its
heading row drawn with `Modifier::DIM` set and `Modifier::BOLD` clear. This replaces the bold
border that carried the same claim before `pane-chrome` removed the borders.

Neither region's interior is left blank unconditionally any longer. The **list** region's
interior is owned by `change-rows`, with `list-selection` owning which slice is drawn. The
**detail** region's interior is owned by `artifact-tabs` (its first row), the horizontal rule
below that, a padding row, and `artifact-content` and `detail-scroll` (the content area
beneath), divided by `layout::split_detail`. The detail region's **change header** is no
longer part of its interior at all: `detail-header` draws it into the region's heading row,
two rows above.

The detail interior is blank on a frame **exactly when `Dashboard::visible()` is empty** — no
repository, no changes, or a `/` filter matching none — and its heading row is blank on
exactly the same condition. That is the whole of the blank case: with a change selected, the
region always carries a heading, a tab bar, a rule, and at least one content line, because
`ui::detail::content_lines` returns `No content yet` rather than nothing.

When the interior **is** blank, every cell of it is a space whose `Style` equals
`ratatui::buffer::Cell::default().style()`. The comparison is against `Cell::default().style()`
and **not** against `Style::default()`: `ratatui-crossterm` re-enables the `underline-color`
feature through its own defaults, so an untouched cell's style is
`fg(Reset).bg(Reset).underline_color(Reset)`, which equals neither `Style::default()` nor
`Style::reset()`. Comparing against the constructible value is what catches a style being
applied to a whole region where one row was meant. Every cell of a region's **padding row**
SHALL satisfy the same comparison on every frame, blank interior or not: nothing is ever
drawn there.

Content SHALL NOT bleed into a gutter or across the divider: no cell of a region's gutter
column, and no cell of the divider column, SHALL be overwritten by a list row, a heading, a
detail header, a tab cell, a rule, a problem line, or a markdown line, at either mandated
width. The wide layout's detail region has no right gutter, so its interior's last column is
the frame's last column and writing there is correct rather than a bleed.

#### Scenario: The routed region's border is bold and the other's is not

The scenario's name is kept verbatim because a delta's scenario headers are its merge key.
There is no border any more; the claim it made — the routed region is the emphasised one —
is now made by the heading row.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  and one active change is rendered at 120x20
- **THEN** the cell at column 1, row 0 reports `Modifier::BOLD` set
- **AND** the cell at column 42, row 0 reports `Modifier::BOLD` **not** set and
  `Modifier::DIM` set
- **AND** with `route: Route::Detail` and the same size, the two assertions swap, so the
  test discriminates rather than asserting a constant
- **AND** at 60x20 the single drawn region's heading cell at column 1, row 0 reports
  `Modifier::BOLD` set under **both** routes, because the region that is drawn is always
  the routed one below the breakpoint

#### Scenario: Interiors are blank at both widths

The scenario's name is kept verbatim because a delta's scenario headers are its merge key;
the detail interior is blank now because no change is selected, not because nothing may
write there.

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  `changes::empty_set()`, and an empty `detail` is rendered at 60x20 and at 120x20
- **THEN** in the 120-column buffer every cell in rows 2 through 18 and columns 42 through
  119 is a space whose `Style` equals `Cell::default().style()` — the detail region's
  interior is untouched because `visible()` is empty
- **AND** in the 120-column buffer rows 0 and 1 of columns 42 through 119 are spaces too,
  because the detail region's heading row is its change header and there is no change to
  name, and its padding row is never drawn
- **AND** in the 120-column buffer row 2, columns 1 through 38, begins `No changes yet`, so
  the list interior is written by `change-rows` rather than left blank, two rows below its
  heading
- **AND** in the 60-column buffer row 2, columns 1 through 58, begins `No changes yet`, and
  rows 3 through 18 of columns 1 through 58 are entirely spaces whose `Style` equals
  `Cell::default().style()`, so exactly one message row was drawn into a seventeen-row
  interior
- **AND** the same dashboard with **one** active change added is no longer blank in the
  detail region at 120x20: row 0 columns 42 onward holds that change's header, so the
  blankness asserted above is a property of the empty visible list rather than a constant

#### Scenario: Rows do not overwrite the borders at either width

The scenario's name is kept verbatim because a delta's scenario headers are its merge key.
What a row must not overwrite is now a gutter column and the divider between them.

- **WHEN** a `Dashboard` holding thirty active changes with names long enough to be
  truncated, whose selected change carries twelve artifacts with 40-character ids, and whose
  whose one section holds thirty lines each 200 characters long, is rendered at 60x20 and at
  120x20
- **THEN** in the 60-column buffer every cell of column 0 and column 59 in rows 0 through 18
  is a space
- **AND** in the 120-column buffer every cell of columns 0, 39, and 41 in rows 0 through 18
  is a space, and every cell of column 40 in those rows is `│` — so a 38-column row neither
  ran into the divider nor into the detail region, and neither a 78-column heading, a
  78-column tab bar, a 78-column rule, nor a 78-column markdown line ran into the divider or
  past the frame
- **AND** in the 120-column buffer column 119 does carry detail content, because the wide
  detail region has no right gutter, and no cell of any buffer lies past the frame's last
  column
- **AND** the same holds at `Route::Detail` at 60x20, where the detail region is the only one
  drawn, no `│` appears at all, and column 59 is a space because the narrow region takes
  `Gutters::Both`

### Requirement: The detail region's two mandated interior widths are 78 and 58

The detail region's interior width SHALL be **78** at a 120-column frame — the wide layout's
`Constraint::Min(0)` part, 79 columns, less its **one** left gutter column — and **58** at a
60-column frame in the detail route, where the region is the whole 60-column body less two
gutter columns. Both interiors SHALL be **17 rows** at a 20-row frame.

Only the height moves, and only by one. `pane-chrome` replaced the region's border columns
with gutter columns, which for the narrow region is the same arithmetic and for the wide
detail region is one gutter plus one column surrendered to the divider's right-hand blank —
so both mandated widths are unchanged and every landed expectation that names them stays
true. It removed the frame's header row and the region's bottom border row and spent one of
the two on the region's padding row, which is where the one extra interior row comes from.

These two widths remain frozen for `detail-view` and `tasks-tab` to inherit, exactly as
`change-rows`' 38 and 58 are. Every test of `ui::markdown` SHALL name both, and a source
check SHALL enforce that with a floor on the number of tests found, on the same terms and
with the same stated limits as the check over `src/ui/list.rs`.

Because the wide layout's detail part is `Min(0)`, 78 is the width at the mandated frame
size and not a constant of the layout: every column gained beyond 120 goes to the detail
region. Nothing SHALL depend on 78 other than the expectations of tests rendered at 120.

#### Scenario: The detail interior is 78 columns at 120 and 58 at 60

- **WHEN** `layout::split_frame` and `layout::split_body` are applied to `Rect::new(0, 0,
  120, 20)` with `Route::Detail`, and the resulting detail rectangle is passed to
  `layout::interior` with `Gutters::LeftOnly`
- **THEN** the interior is `Rect::new(42, 2, 78, 17)`
- **AND** the same applied to `Rect::new(0, 0, 60, 20)` with `Route::Detail` and
  `Gutters::Both` gives `Rect::new(1, 2, 58, 17)`
- **AND** at `Rect::new(0, 0, 60, 20)` with `Route::List` there is no detail rectangle at
  all, so the narrow list route has no detail interior to be 58 columns wide

#### Scenario: Every markdown test names both of its two interior widths

- **WHEN** every `#[test]` in `src/ui/markdown.rs` is inspected for the bare literals `78`
  and `58`, with a floor on the number of tests found
- **THEN** every one of them names both, and the floor is met, so a gutted file fails rather
  than passing with nothing to check
- **AND** the check is a floor and not a proof — a `78` in a comment satisfies it — and is
  paired with a counted filtered run, because `cargo test` exits 0 when a filter matches
  nothing
- **AND** the number scan does not see a suffixed literal such as `78u16`, so it fails
  closed and the widths are written unsuffixed

### Requirement: Display width is measured in terminal columns by one pair of primitives

Every measurement and every truncation under `src/ui/` SHALL be expressed in **terminal
display columns**, never in `char`s and never in bytes. Two pure total functions in
`ui::layout` SHALL be the crate's only implementation of that measure:

```rust
pub(crate) fn columns(text: &str) -> usize;
pub(crate) fn truncate_columns(text: &str, max: usize) -> &str;
```

`columns` SHALL return the number of terminal cells `ratatui::buffer::Buffer::set_string`
consumes for `text`, computed the way `set_string` itself computes it and not by an
independent table: it SHALL split `text` into grapheme clusters and sum each cluster's
width through ratatui's own public API — `ratatui::text::Span::styled_graphemes`, which
performs the split and drops clusters containing a control character, and the
`ratatui::buffer::CellWidth` trait, which yields each remaining cluster's cell width. This
is the whole of the argument for the choice: agreement with the buffer is the property being
bought, and any second measure — a hand-rolled table, or `unicode-width` called directly —
would agree with `set_string` only by coincidence of version and would miss ratatui's own
halfwidth-katakana adjustment and its control-character filter.

`truncate_columns(text, max)` SHALL return the longest **prefix of `text` ending on a
grapheme-cluster boundary** whose `columns` is at most `max`. It SHALL never split a
cluster, SHALL return `""` for `max == 0`, and SHALL return `text` whole when
`columns(text) <= max`. Because a cluster is dropped whole, the returned prefix MAY measure
`max - 1` columns where a two-column cluster would not fit; a caller that owes an exact
width SHALL pad the difference rather than assume the prefix filled it.

Both functions SHALL be pure and total: no filesystem, process, environment, network, or
standard-I/O work, no clock, no global state, and no panic for any `&str` and any `usize`.
Both live in `ui::layout` — the module `responsive-layout` already owns, and which already
names a `ratatui` type — so the crate's pure view set stays at the **eight** files
`dashboard-loop` enumerates and no ninth file is added to it. This placement is a
consequence of that count, not a claim that text measurement is `Rect` geometry.

**The Unicode promise the pane makes, stated rather than left to be discovered, and stated
with its limit.** A grapheme cluster occupies the columns ratatui gives it, and no line the
view produces ever exceeds — **as ratatui measures it** — the region it is drawn into. That
qualification is the promise's boundary and is deliberate: the pane's arithmetic and
`Buffer::set_string` are the same measure by construction, so within the buffer the bound is
exact, but a terminal is free to paint a cluster in a different number of cells than ratatui
budgeted and the pane has no way to know.

The known divergence runs in one direction, and it is the opposite of the intuitive one. A
zero-width-joiner emoji sequence is **one** grapheme cluster to `graphemes(true)`, and
ratatui measures it at 2 columns — the same as a terminal that composes it. A terminal that
does **not** compose it paints each constituent emoji instead, six columns for a
three-person family, and the row overruns. The pane SHALL NOT attempt to detect or
compensate for this: it is a property of the terminal's font and shaping, invisible to a
process writing bytes to a pty, and any compensation would have to guess. It is recorded
here as an accepted limit of the promise rather than a defect, and it is the one case in
which a line can exceed its region after this change.

Correspondingly, the pane SHALL NOT reorder bidirectional text, SHALL NOT probe the terminal
for its capabilities, and SHALL NOT tailor any measurement to a locale — which carries its
own instance of the same limit, since several of the box-drawing and arrow characters the
artifacts already use are East Asian **Ambiguous** and a CJK-locale terminal renders them at
2 columns where `unicode-width`'s default, and therefore ratatui's, says 1.

No file under `src/ui/` other than `src/ui/layout.rs` SHALL measure a rendered string with a
`char` count. Concretely, no **production** line of `src/ui/app.rs`, `src/ui/detail.rs`,
`src/ui/list.rs`, `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, or
`src/ui/driver.rs` SHALL name `.chars().count()`, `.chars().take(`, or a
`Vec<char>`-producing `.chars().collect()`. Those seven are the whole of the rule's reach,
not a sample of it: `src/ui/mod.rs` and `src/ui/terminal.rs` are the only other files under
`src/ui/`, neither renders, and neither holds such a measurement in production code today.
Iterating characters for a purpose that is not measurement — `ui::markdown`'s per-character
scanner, for one — is untouched by this rule.

**Two measuring sites the pattern above cannot see, named so they are not missed.**
`ui::markdown`'s `split_at_char`, which cuts a run at a `char_indices` offset for the
hard-split, and any future `char_indices`-based cut, are column budgets expressed in
characters that no `.chars()` pattern matches. They SHALL be rewritten to cut in columns
along with the rest; what proves it is `markdown-render`'s wide-character and 500-column-CJK
scenarios, not the sweep. A source check that cannot see a violation is recorded as a limit
rather than relied on.

**The footer is inside this rule.** `responsive-layout`'s own footer requirement and
`list-filtering`'s both state their hint budgets and their keep-the-tail truncation of the
filter prompt in characters, against all-ASCII hint literals whose stated counts stay exactly
true. The **query** in that footer is the reader's own typed text and is not ASCII-bound, so
the footer's budget arithmetic and its tail truncation SHALL be measured with `layout::columns`
and cut with `layout::truncate_columns` like every other field. No hint literal's length
changes and no landed footer assertion moves.

#### Scenario: `columns` agrees with what the buffer consumed

- **WHEN** for each of `abc`, `日本語`, `🎉`, `e` followed by U+0301 COMBINING ACUTE ACCENT,
  a family emoji joined by two zero-width joiners, `ｶ` followed by U+FF9E HALFWIDTH KATAKANA
  VOICED SOUND MARK, a string holding only U+0007 BEL, and the empty string, the string is
  written into a fresh 40x1 `Buffer` with
  `Buffer::set_stringn(0, 0, s, usize::MAX, Style::default())` and the `x` of the `(u16, u16)`
  that call **returns** is taken — the cursor position `set_stringn` advanced to, which is by
  definition the number of cells it consumed
- **THEN** that `x` equals `layout::columns` of the same string for every one of the eight
- **AND** the oracle is `set_stringn`'s **return value** and SHALL NOT be "the index of the
  first blank cell": `set_stringn` calls `Cell::reset()` on the trailing cells of every
  multi-column cluster, and a reset cell is byte-identical to an untouched one, so a
  first-blank scan reports `1` for `日本語`, for `🎉`, and for the family emoji, and a
  measurement built to satisfy it would be wrong in exactly the direction this change exists
  to fix. `set_string` itself returns `()` and cannot serve as the oracle
- **AND** `columns("")` is `0` and `columns` of the BEL string is `0`, because
  `styled_graphemes` drops control clusters exactly as `set_stringn` does

#### Scenario: `truncate_columns` never splits a cluster and never overruns

- **WHEN** `truncate_columns` is called with `日本語の変更` at every `max` from `0` through
  `14`, with `abc🎉def` at every `max` from `0` through `10`, and with
  `ab` + U+0007 BEL + `日本語` at every `max` from `0` through `10`
- **THEN** at every `max` the result's `columns` is at most `max`, the result is a **prefix of
  the input as bytes**, and re-slicing the input at the result's own length succeeds — so no
  call ever cut a cluster and none panicked
- **AND** at `max` `3` for `日本語の変更` the result is `日` and measures `2`, one short of
  `max`, because the second cluster would have overrun
- **AND** at `max` `0` every result is `""`, and at a `max` at or above the input's own
  `columns` every result is the whole input
- **AND** the BEL case does not panic at any `max`, which is what pins the byte-offset rule
  below: `styled_graphemes` **drops** the control cluster, so an implementation that derives
  its cut point by summing the yielded symbols' `len()` computes an offset shifted by the
  dropped byte and slices mid-character. `truncate_columns` SHALL derive its cut point from
  each symbol's own position within the original `&str` — its byte offset, not a running sum
  of returned lengths — so a dropped cluster shifts nothing

#### Scenario: Nothing under `src/ui/` measures in characters except the primitives

- **WHEN** the production lines of `src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/list.rs`,
  `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` — every
  line up to each file's `#[cfg(test)]` module — are searched for `.chars().count()`,
  `.chars().take(`, and a `Vec<char>` `.chars().collect()`
- **THEN** there is no match in any of the seven
- **AND** the check first runs **its own sweep pattern** against a line it synthesises
  holding each of the three forms, and fails when that self-test does not match all three.
  This is the positive control, and it is the sweep's pattern rather than a second one: a
  control that greps `layout.rs` for `cell_width` proves only that `cell_width` is spelled
  right, and would let a corrupted sweep regex print `COLWIDTH OK` over a tree full of
  violations. `NOSPAWN-GREP`'s shape — run the same pattern against something that must
  match — is the model
- **AND** the check additionally requires `src/ui/layout.rs` to name `cell_width` and
  `styled_graphemes`, so the measure it is protecting is present and the exemption is not
  vacuous
- **AND** the check is a repository file under `scripts/gates/` named in the `Makefile`'s
  `gates:` recipe, so `tests/ci_workflow.rs`'s recipe-versus-directory assertion covers it
  and it cannot silently drop out of `make gates`

### Requirement: A point in the frame resolves to exactly one zone

`ui::layout::zone(area: Rect, route: Route, column: u16, row: u16) -> Zone` SHALL map a
terminal coordinate to the part of the frame drawn there, deriving the geometry through the
same `split_frame`, `split_body`, `interior`, and `split_detail` the draw path uses and
holding no arithmetic of its own beyond a rectangle containment test. It SHALL be pure —
no filesystem, process, environment, network, or standard-I/O API — and total: every
`Rect`, every `Route`, and every `(column, row)` pair including `(0, 0)` and
`(u16::MAX, u16::MAX)` returns a `Zone` and none panics.

`Zone` SHALL have exactly five variants:

| Variant | Meaning |
|---|---|
| `ListRow { interior: Rect, row: u16 }` | A row of the list region's interior. `interior` is that interior's own rectangle and `row` is the offset of the addressed row below its first interior row |
| `List` | The list region, but not one of its interior rows — its gutters, its heading row, or its padding row |
| `DetailTab { bar: Rect, column: u16 }` | The detail region's tab-bar row. `bar` is that row's own rectangle and `column` is the offset of the addressed column right of its first column |
| `Detail` | The detail region, anywhere but the tab-bar row: its gutter, its heading row, its padding row, the rule below the tab bar, the content padding row, or its content area — and the divider column beside it |
| `Outside` | The frame's footer row, or a point outside the frame entirely |

`Outside` no longer covers a frame header row, because there is no longer one. Every row of
the frame but the last is now a body row and resolves to a region's zone whenever a region is
drawn there.

The **divider column** is in neither region's area, so its zone is decided rather than
derived: it SHALL resolve to `Detail`. It is one column, the reader who lands on it meant one
of the two regions, and the detail is the region whose content scrolls under a wheel — so
giving it to the detail makes a near-miss do something rather than nothing. The choice is
arbitrary in the sense that `List` would also be defensible; it is written down here so it is
one answer rather than an accident.

`ListRow` and `DetailTab` SHALL carry the rectangle the zone was derived from rather than
only an offset, so the caller that resolves the offset to a row or a tab uses the very
geometry the hit test used. Recomputing the interior at the call site would be a second
derivation of the same rectangle, and the two could drift.

`zone` SHALL respect the route below the breakpoint exactly as `split_body` does: at
`LayoutMode::Narrow` only the routed region exists, so every point in the body resolves to
that region's zones and none to the other's. At `LayoutMode::Wide` both regions exist at
both routes, and the route changes nothing about the mapping.

`zone` SHALL name no ratatui widget, no crossterm type, and no mouse type: it takes two
integers, which is what keeps it in the pure view set and testable with no event at all.

#### Scenario: The zones tile the frame at 120 columns

- **WHEN** `zone` is called at a 120x40 frame, at `Route::List` and again at
  `Route::Detail`, for a point on the frame's footer row, the list region's heading row, the
  list region's padding row, the list region's left gutter, the list interior's first row,
  the list interior's last row, the divider column 40, the detail region's heading row, the
  detail interior's tab-bar row, the rule row below it, the content padding row, the detail
  interior's first content row, and column 200
- **THEN** the results are `Outside`, `List`, `List`, `List`, `ListRow` with `row` 0,
  `ListRow` with `row` equal to the interior's last index, `Detail`, `Detail`, `DetailTab`
  with `column` 0, `Detail`, `Detail`, `Detail`, and `Outside`, at both routes
- **AND** every `ListRow`'s `interior` equals
  `interior(split_body(body, route).0.unwrap(), Gutters::Both)` and every `DetailTab`'s `bar`
  equals `split_detail(interior(detail_area, Gutters::LeftOnly)).0`, computed independently
  in the test
- **AND** row 0 of the frame resolves to a region rather than to `Outside`, because the
  frame has no header row for it to belong to

#### Scenario: Below the breakpoint only the routed region has zones

- **WHEN** `zone` is called at a 60x20 frame at `Route::List` for the interior's first row,
  and then at `Route::Detail` for the same point
- **THEN** the first is a `ListRow` and the second is a `Detail` or `DetailTab`
- **AND** no point anywhere in the 60-column body resolves to a list zone at `Route::Detail`,
  and none resolves to a detail zone at `Route::List`

#### Scenario: The breakpoint is exact for the hit test too

- **WHEN** `zone` is called for the same body point at frame widths 99, 100, and 101 at
  `Route::Detail`
- **THEN** 99 resolves through the narrow layout and 100 and 101 through the wide one,
  agreeing with `layout::mode` at each width

#### Scenario: Degenerate frames resolve without panicking

- **WHEN** `zone` is called at frame areas `0x0`, `1x1`, `2x2`, `3x3`, and `120x2` — the
  heights `split_frame` branches on explicitly — for every `(column, row)` pair inside the
  area and for `(u16::MAX, u16::MAX)`, at both routes
- **THEN** every call returns a `Zone` and none panics
- **AND** a frame with no body resolves every point to `Outside`, since there is no region
  to be over

#### Scenario: The hit test agrees with what was drawn

- **WHEN** a dashboard with three active and three archived changes is rendered into a
  `TestBackend` at 120x40 and again at 60x20, and every cell of the resulting buffer is
  classified by `zone`
- **THEN** every cell `zone` reports as `ListRow` holds a character from `list::rows`' own
  output for that row, and every cell it reports as `List` or `Detail` in a gutter column
  holds a space, and the divider column holds `│`
- **AND** no cell of the drawn buffer is classified as belonging to a region the draw path
  did not draw

### Requirement: The frame is a body and a footer row

`ui::view::render(frame, &Dashboard)` SHALL be a pure function of its two arguments: it
SHALL perform no filesystem, process, environment, network, or terminal I/O, and SHALL
read no clock and no global state. It SHALL divide `frame.area()` vertically into exactly
**two** regions, in order — a body of `Constraint::Min(0)` and a footer of
`Constraint::Length(1)` — and SHALL draw nothing outside them.

There SHALL be no header row. `pane-chrome` removes it: in a Herdr split the pane is
already titled by Herdr, and the literal `OpenSpec` label this requirement used to mandate
at row 0 column 0 restated that title one row below it while spending the row that could
have named the repository. The repository's identity moves into the list region's own
heading row, specified by "The list region's heading names the repository directory"; no
part of the frame draws the literal `OpenSpec` any more.

The footer SHALL render the key hints `q quit`, `Enter detail`, and
`Esc back` in that order, separated by two spaces, starting at column 0, dropping hints
from the **end** when the remaining width cannot hold the next one whole.

`agent-launch` adds **two further hints, placed after `Esc back`**: when
`Dashboard::agents.reachable` is `true` the footer SHALL append `a/c/s launch` and then
`g focus`, joined by the same two-space separator. When it is `false` both SHALL be absent
entirely, so a pane with no reachable Herdr socket renders the footer this requirement
specified before `agent-launch` existed — which is the whole of `SPEC.md` → Degraded states'
"action keys hidden".

They are **one compound hint plus one**, not four separate ones, and that is a width decision
rather than a stylistic one: `q quit  Enter detail  Esc back  a apply  c continue  s archive`
is **62** columns, so at the mandated 60-column frame `fit_hints` would drop `s archive` and
`g focus` and offer the reader two of the four action keys with no indication that the other
two exist. `a/c/s launch  g focus` costs 23 columns including its separators, bringing the
footer to **53**, which fits the narrow frame whole. The keys themselves are documented in
`SPEC.md` → Keys and `README.md` → Keys; the footer's job is to say the feature is available
here and now, not to be the manual.

`agent-attribution` adds a **hint placed last**: when
`Dashboard::attribution().unattributed` is greater than zero, the footer SHALL append that
count, a single space, and the word `unattributed` — `1 unattributed`, `12 unattributed` —
after the action hints, joined by the same two-space separator. When the count is zero the hint
SHALL be absent entirely, so an agentless pane's footer is byte-identical to the footer this
requirement already specified. Being last means it is the **first** hint dropped as the width
falls, which is the correct priority: the key hints tell a reader how to drive the
pane, and the count tells them something they can act on later.

That priority has a measured consequence `agent-launch` makes explicit rather than leaving to be
discovered: the full reachable footer with a count is **69** columns, so at the mandated
60-column frame **the count is dropped and the action hints are kept**. Before this change the
count fitted at 60 (the footer was 46 columns); it still fits whenever the socket is
unreachable, because the two hints it now competes with are absent. The drop order is unchanged
— last hint first — and the hint list grew; nothing about the rule moved.

`SPEC.md` → Attributing an agent's tier 3 requires a count and forbids a row: an agent that
no tier attributed is reported here, in one shared cell, and never against a change. The
footer is the whole of that report — there is no per-agent listing, no expansion, and no key
that opens one. It is also never a `!`-marked problem row: an unattributed agent is a normal
state of a shared Herdr session, not a fault. A **failed launch** is not reported here at all:
`agent-launch` puts it on `Dashboard::launch.problems` and `change-rows` renders it as a
leading `!`-marked list row, because it is a fault and it is one the reader just caused.

The footer has two further forms, specified by `list-filtering` and restated here because
this requirement owns the row: while `dashboard.filter.active` is set the hints are
**replaced** by the prompt `/`, the query, and `_`, keeping its tail when it overflows —
and the action hints and the unattributed count are replaced along with them, because the
prompt replaces the whole row rather than the three key hints specifically; while the filter is
inactive with a non-empty query, `/` and the query become a further hint placed **first** in the
list above, dropped last rather than first, with the action hints and then the count after the
three key hints as usual.

Scenarios in this capability render a `Dashboard` whose `changes` is
`changes::empty_set()` and whose `agents.reachable` is `false` unless they say otherwise. The
first is no longer inert: with a repository root present, `change-rows` renders a
`No changes yet` message row into the list region's interior, and the scenarios below are
written so that none of them depends on that interior being blank. The second is stated
explicitly for the first time here: every landed footer assertion in this capability was written
against a dashboard whose socket was unreachable, and pinning that is what keeps those exact
strings true rather than accidentally so.

Degenerate heights SHALL be decided explicitly rather than delegated to the constraint
solver, on the same terms and for the same measured reason as before: `Layout::vertical([
Min(0), Length(1)])` at height 1 gives the single row to the **footer**, so a naive split
renders `q quit` where the repository's name belongs. The explicit branch is: at height 0
`render` SHALL draw nothing; at height 1 it SHALL draw the **body** only, which is one row
and therefore exactly the routed region's heading row; at height 2 or more it SHALL use the
two-way split above, giving the body every row but the last. `render` SHALL NOT panic at any
frame size of at least one column by one row, and SHALL NOT panic at an interior of zero
columns or zero rows, which a one- or two-column frame produces.

The row this change frees is spent on content, not on air: at a 20-row frame the body grows
from eighteen rows to nineteen and the region's own former bottom border row is gone as
well, so a region's interior grows from sixteen rows to seventeen. That count is asserted by
"The routed region is emphasised and region interiors are left empty" rather than here.

#### Scenario: Body and footer occupy their rows at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose
  `agents.reachable` is `false` is rendered into a `TestBackend` at 60x20, and again at 120x20
- **THEN** in both buffers row 0 is the routed region's heading row — it spells `demo-repo`
  from column 1 — and the string `OpenSpec` appears in no cell of either buffer
- **AND** in both buffers row 19 begins with the exact string
  `q quit  Enter detail  Esc back` at column 0, and every remaining cell of row 19 is a
  space
- **AND** in both buffers no box-drawing character appears in column 0 or in column
  `width - 1` of any row, so the body occupies rows 0 through 18 with no bordered block in
  it and nothing is drawn in row 19 by the body

#### Scenario: The action hints follow `Esc back` when the socket is reachable

- **WHEN** the same `Dashboard` with `agents.reachable` set to `true` and no agents is
  rendered at 60x20 and at 120x20
- **THEN** row 19 is exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus` — **53** characters — followed by
  seven spaces at 60 and sixty-seven spaces at 120
- **AND** rendering the identical dashboard with `agents.reachable` `false` produces row 19 of
  exactly `q quit  Enter detail  Esc back` followed by thirty spaces at 60, byte-identical to
  the row this capability specified before the action hints existed
- **AND** nothing outside row 19 differs between the two renders at either width, so the
  reachability flag moves the footer and nothing else

#### Scenario: The action hints are dropped whole, `g focus` first

- **WHEN** the reachable, agentless dashboard is rendered at 53x20, 52x20, 44x20, and 43x20
- **THEN** the 53-column row is exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus`, filling the row with no trailing
  space
- **AND** the 52-column row is exactly `q quit  Enter detail  Esc back  a/c/s launch` followed
  by eight spaces — `g focus` and its two-space separator need nine columns and only eight
  remain, so it is dropped whole rather than cut to `g focu`
- **AND** the 44-column row is exactly `q quit  Enter detail  Esc back  a/c/s launch`, filling
  the row, and the 43-column row is exactly `q quit  Enter detail  Esc back` followed by
  thirteen spaces
- **AND** at 43 columns the three key hints are all still present, so both action hints are
  dropped before any of them

#### Scenario: A one-row frame renders the body's heading row and nothing else

- **WHEN** the same `Dashboard` is rendered at 60x1 and at 120x1
- **THEN** neither render panics
- **AND** row 0 spells `demo-repo` from column 1 in both — the routed region's heading row,
  which is the body's only row at this height
- **AND** no `q quit` appears in either buffer, so the footer was not drawn into the body's
  single row

#### Scenario: A two-row frame renders one body row and the footer

- **WHEN** the same `Dashboard` is rendered at 60x2 and at 120x2
- **THEN** neither render panics
- **AND** row 0 spells `demo-repo` from column 1 and row 1 begins `q quit` at column 0
- **AND** no list row is drawn anywhere in either buffer, because the body received one row
  and the heading row consumed it

#### Scenario: A one-column frame renders without panicking

- **WHEN** the same `Dashboard` is rendered at 1x1, at 1x20, at 2x20, and — as the
  contrasting controls at the two mandated widths — at 60x20 and 120x20
- **THEN** none of the five panics, including the two whose list region has an interior of
  zero columns
- **AND** the 1x20 buffer's row 0 is a single space: the region's one column is its left
  gutter, its interior is zero columns wide, and the heading row is therefore truncated to
  nothing rather than drawn over the gutter
- **AND** the 1x20 buffer's row 19 is a single space: `q quit` needs six columns, so the
  first hint is dropped whole rather than truncated to `q`
- **AND** the same holds with `agents.reachable` `true`, which adds no hint that could fit in
  one column and therefore changes no cell of the 1x20 buffer
- **AND** the 60x20 and 120x20 buffers both spell `demo-repo` from column 1 of row 0 and
  both begin row 19 with `q quit`, so the one-column result is a width branch rather than
  the heading and footer being absent everywhere

#### Scenario: The footer drops whole hints rather than truncating one

- **WHEN** the same `Dashboard`, with `agents.reachable` `false`, is rendered at 18x20, at
  20x20, at 60x20, and at 120x20
- **THEN** the 18-column footer row is exactly `q quit` followed by twelve spaces —
  `q quit  Enter detail` needs exactly 20 columns, so `Enter detail` and every hint after
  it are dropped whole rather than cut short
- **AND** the 20-column footer row is exactly `q quit  Enter detail`, filling the row with
  no trailing space, which pins the boundary from the other side
- **AND** the 60-column footer row is exactly `q quit  Enter detail  Esc back` — thirty
  characters — followed by thirty spaces, so all three hints fit at the mandated narrow
  width
- **AND** the 120-column footer row is the same thirty characters followed by ninety
  spaces

#### Scenario: The unattributed count is the footer's last hint at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo`, whose `changes` holds
  one active change `alpha`, whose `agents.agents` holds one in-scope agent named
  `nothing-like-a-change`, and whose `agents.reachable` is `false`, is rendered at 60x20 and at
  120x20
- **THEN** the 60-column footer row is exactly
  `q quit  Enter detail  Esc back  1 unattributed` — forty-six characters — followed by
  fourteen spaces
- **AND** the 120-column footer row is the same forty-six characters followed by
  seventy-four spaces
- **AND** the same dashboard with `agents.reachable` set to `true` gives a 120-column footer row
  of exactly `q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` — **69**
  characters — followed by fifty-one spaces, and a 60-column row of exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus` followed by seven spaces: the count
  needs sixteen further columns and only seven remain, so it is dropped whole. That is the same
  last-hint-first rule, applied to a longer list
- **AND** rendering the identical dashboard with `agents.agents` empty and `reachable` `false`
  produces a footer row of exactly `q quit  Enter detail  Esc back` and thirty spaces at 60
  columns, byte-identical to the row this capability specified before the count existed
- **AND** the count is a number of agents, not of changes: adding a second in-scope agent
  named `also-nothing` makes the hint read `2 unattributed` at 120 columns under either value
  of `reachable`, while the list region's rows are unchanged

#### Scenario: The count is reported with an empty change list

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo`, whose `changes` is
  `changes::empty_set()`, whose `agents.agents` holds two in-scope agents named
  `nothing-like-a-change` and `also-nothing`, and whose `agents.reachable` is `false`, is
  rendered at 60x20 and at 120x20
- **THEN** the footer row reads `q quit  Enter detail  Esc back  2 unattributed` at both
  widths
- **AND** the list region's interior holds exactly the single `No changes yet` message row
  `change-rows` specifies, byte-identical to the agentless rendering of the same dashboard —
  a `Message` row is never badged
- **AND** the same holds with a `/` query matching nothing: the two message rows are
  byte-identical and the count is unchanged, because the count is over agents and the filter
  is over changes
- **AND** with `agents.reachable` set to `true` the list region's interior is unchanged, cell
  for cell, at both widths: the action hints live in the footer and never in the list

#### Scenario: The count is dropped whole before the three key hints

- **WHEN** the one-unattributed-agent dashboard with `agents.reachable` `false` is rendered at
  46x20 and at 45x20
- **THEN** the 46-column footer row is exactly
  `q quit  Enter detail  Esc back  1 unattributed`, filling the row with no trailing space
- **AND** the 45-column footer row is exactly `q quit  Enter detail  Esc back` followed by
  fifteen spaces — the count and its two-space separator need sixteen columns and only
  fifteen remain, so it is dropped whole rather than cut to `1 unattribute`
- **AND** at 45 columns the three key hints are all still present, so the count is dropped
  before any of them
- **AND** the same dashboard with `agents.reachable` `true` pins the boundary one hint list
  further out: the 69-column row is exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` with no trailing
  space, and the 68-column row is exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus` followed by fifteen spaces — the
  count dropped whole, both action hints kept

#### Scenario: The filter prompt replaces the count along with the hints

- **WHEN** the one-unattributed-agent dashboard with `agents.reachable` `false` is rendered at
  60x20 and at 120x20 with `filter.active` set and `filter.query` `be`, and then again with
  `filter.active` cleared and the same query kept
- **THEN** the active-filter footer row is exactly `/be_` followed by spaces at both widths,
  and the string `unattributed` appears nowhere in it
- **AND** the accepted-query footer row is exactly
  `/be  q quit  Enter detail  Esc back  1 unattributed` at both widths, so the query leads
  the list and the count still trails it
- **AND** both forms are byte-identical to what `list-filtering` specifies once the same
  dashboard's `agents.agents` is emptied, so the count is additive rather than a rewrite of
  either form
- **AND** with `agents.reachable` `true` the active-filter row is still exactly `/be_` followed
  by spaces at both widths, and the strings `a/c/s launch` and `g focus` appear nowhere in it:
  the prompt replaces the **whole** row, action hints included
- **AND** with `agents.reachable` `true` the accepted-query row is exactly
  `/be  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` — 74 characters
  — at 120, and exactly `/be  q quit  Enter detail  Esc back  a/c/s launch  g focus` — 58
  characters — followed by two spaces at 60, so the query still leads and the count is still
  the first thing dropped

### Requirement: A region is a heading row, a padding row, and a gutter-padded interior

A region SHALL be drawn without a border. `ui::view::render_region` SHALL draw no `Block`,
no box-drawing character, and no title. Every region SHALL be composed of exactly three
parts, in this order down its area:

| Part | Rectangle | Drawn |
|---|---|---|
| heading row | `Rect::new(x + gl, area.y, iw, 1)` | the region's own heading |
| padding row | `Rect::new(x + gl, area.y + 1, iw, 1)` | never — it is blank by construction |
| interior | `Rect::new(x + gl, area.y + 2, iw, area.height - 2)` | the region's content |

where `gl` and `gr` are the region's left and right **gutter** columns, `x` is `area.x`, and
`iw` is `area.width - gl - gr`. Gutter columns are never drawn into by any region.

`ui::layout::interior(area: Rect, gutters: Gutters) -> Rect` SHALL return that interior:
the origin advanced by `gl` columns and **two** rows and clamped to the rectangle's own
right and bottom edges, the width reduced by `gl + gr` saturating to zero, and the height
reduced by two saturating to zero. The clamp is not decoration — at a 1x1 or 0x0 rectangle
it is the difference between `x: 0` and `x: 1`.

```rust
pub enum Gutters { Both, LeftOnly }
```

`Gutters::Both` SHALL give `gl = 1, gr = 1` and `Gutters::LeftOnly` `gl = 1, gr = 0`. The
list region and the narrow layout's single region take `Both`; the **wide** layout's detail
region takes `LeftOnly`, for the arithmetic reason the divider requirement below states.
There is no `RightOnly` and no `Neither`: no region in this layout wants one, and a variant
nothing constructs is a variant nothing tests.

The **padding row** is why one rule covers both regions. The list region wants a blank row
between the repository's name and its first change; the detail region wants one between the
change's header and its tab bar. They are the same row at the same offset, so they are one
part of one shape rather than two special cases in two draw paths.

The gutters are why the mandated interior **widths** do not move. A bordered region's
interior was its area less two border columns; a borderless region's interior is its area
less its gutter columns, and for `Gutters::Both` the two are the same arithmetic. **38** and
**58** for the list and **78** and **58** for the detail are therefore unchanged by this
change, and every landed row-grammar, markdown, tasks, and detail test that names them stays
true.

The interior's **height** grows by exactly one row at every frame height: two rows are freed
(the frame's header row and the region's bottom border row) and one is spent on the padding
row. At a 20-row frame a region's interior is **17 rows**, up from sixteen, and its first row
is buffer row **2** — the very row a bordered region's interior began at. That is not a
coincidence to be relied on loosely: it is stated so that a scenario elsewhere asserting
"buffer row 2 is the interior's first row" is known to be still true rather than accidentally
so.

#### Scenario: A region draws a heading, a blank row, and no border at both widths

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo`, holding three active
  changes, at `Route::List`, is rendered at 120x20 and at 60x20
- **THEN** no box-drawing character other than `│` appears in either buffer — no `┌`, no
  `┐`, no `└`, no `┘`, and no `─` outside the detail region's own rule row
- **AND** in the 60-column buffer row 0 columns 1 through 58 are the region's heading row,
  every cell of row 1 columns 1 through 58 is a space, and row 2 columns 1 through 58 is the
  first list row
- **AND** in the 60-column buffer every cell of column 0 and of column 59 is a space in every
  row of the body, so both gutters are empty
- **AND** in the 60-column buffer row 18 holds a drawn list row, so the interior's last row is
  content rather than a border

#### Scenario: `interior` reserves two rows and the gutters its `Gutters` names

- **WHEN** `layout::interior` is called with `(Rect::new(0, 0, 60, 19), Gutters::Both)`,
  `(Rect::new(0, 0, 40, 19), Gutters::Both)`, `(Rect::new(41, 0, 79, 19), Gutters::LeftOnly)`,
  `(Rect::new(0, 0, 1, 1), Gutters::Both)`, and `(Rect::new(0, 0, 0, 0), Gutters::Both)`
- **THEN** the results are `Rect::new(1, 2, 58, 17)`, `Rect::new(1, 2, 38, 17)`,
  `Rect::new(42, 2, 78, 17)`, `Rect::new(1, 1, 0, 0)`, and `Rect::new(0, 0, 0, 0)`
- **AND** the `1x1` case keeps the **origin clamp**: `x` is `min(0 + 1, 0 + 1)` = 1 and `y`
  is `min(0 + 2, 0 + 1)` = 1, so the origin lands on the rectangle's own right and bottom
  edges rather than staying at zero, and only the width and height saturate. `detail-scroll`
  states the same value for the same call
- **AND** the first three are seventeen rows tall, one more than the sixteen the bordered
  arithmetic gave at the same frame height, and each begins at row 2 exactly as it did
- **AND** the `LeftOnly` interior's last column is `119`, the frame's own last column, while
  each `Both` interior leaves its area's last column untouched

### Requirement: A one-column divider separates the two regions, with a blank column each side

At `LayoutMode::Wide` `ui::view::render_body` SHALL draw a **vertical divider** — the
character `│` — down one column for every row of the body, styled
`palette::style(Role::RegionRule)`. The divider column belongs to **neither** region: it is
drawn by `render_body`, which is the one place that knows both rectangles, and no region's
area contains it.

At `LayoutMode::Narrow` no divider SHALL be drawn at all, because there is only one region
and nothing to divide it from.

The wide layout's body SHALL therefore be split horizontally into **three** parts, not two —
`Constraint::Length(40)` for the list, `Constraint::Length(1)` for the divider, and
`Constraint::Min(0)` for the detail — so that at a 120-column frame the columns are:

| Column(s) | What |
|---|---|
| `0` | the list region's left gutter |
| `1`–`38` | the list region's interior — **38** columns |
| `39` | the list region's right gutter |
| `40` | the divider `│` |
| `41` | the detail region's left gutter |
| `42`–`119` | the detail region's interior — **78** columns |

A divider with a blank column on **both** sides costs **five** chrome columns, and
`38 + 78` leaves exactly **four** at a 120-column frame. One outer gutter therefore cannot
be had, and it is the **trailing** one that goes: the detail region takes `Gutters::LeftOnly`
and its interior runs to the frame's own last column. The leading gutter is kept because
column 0 carries the list's selection marker on every row, where a flush edge would read as
part of the grammar; the trailing edge is reached only by a right-aligned cell — the detail
header's progress and schema cells, and a markdown line that happens to fill the width.

Taking the fifth column from an **interior** instead was measured and rejected: the literal
`78` appears 162 times under `src/ui/` and is hard-coded in three `scripts/gates/` scripts,
so a 77-column detail interior would rewrite roughly 145 hand-computed test expectations for
a column of whitespace.

#### Scenario: The divider has a blank column on each side at 120 columns

- **WHEN** a `Dashboard` with `route: Route::List`, a repository root of `/tmp/demo-repo`,
  and one active change `alpha` is rendered at 120x20
- **THEN** every cell of column 40 in rows 0 through 18 is `│` and reports `Modifier::DIM`
  and no foreground
- **AND** every cell of column 39 and of column 41 in rows 0 through 18 is a space, so the
  divider is not flush against either region's content
- **AND** every cell of column 0 in rows 0 through 18 is a space, so the leading gutter
  survived
- **AND** row 19 — the footer — holds no `│`, so the divider is confined to the body

#### Scenario: The divider column is a width branch, not a constant

- **WHEN** the same `Dashboard` is rendered at 120x20, at 100x20, at 99x20, and at 60x20
- **THEN** in the 120-column and 100-column buffers every cell of column 40 in rows 0
  through 18 is `│`, so the divider column is fixed by the list's `Length(40)` rather than by
  the frame width
- **AND** in the 99-column and 60-column buffers the character `│` appears in no cell at all
- **AND** in the 100-column buffer the detail region's interior is `Rect::new(42, 2, 58, 17)`,
  so every column gained beyond 100 still goes to the detail side

#### Scenario: A one-, two-, and three-column frame degenerates without drawing over a gutter

- **WHEN** the same `Dashboard` is rendered at 1x20, at 2x20, and at 3x20
- **THEN** none of the three panics
- **AND** in the 1x20 and 2x20 buffers no heading text and no list row is drawn at all: the
  interior is zero columns wide, so there is nothing to draw into
- **AND** in the 3x20 buffer the heading row and every list row occupy column 1 alone, and
  columns 0 and 2 are spaces in every row
- **AND** none of the three holds a `│`, because all three are below the breakpoint

### Requirement: The list region's heading names the repository directory

The list region's heading row SHALL render the repository root's **final path component** —
its directory name — when a root was found, and the literal `no repository` when none was.
It SHALL be drawn left-aligned at the heading row's first column, which is the interior's
first column. When the root has no final component — the filesystem root `/` — the whole
display path SHALL be rendered instead, so the row is never empty when a root exists.

The absolute path this row used to carry is gone. It was measured at a 60-column split to
consume the row whole while naming, in its last component, the only part a reader uses; the
directory name is that component. A reader who needs the absolute path has the pane's own
`no repository` rows and the Herdr pane's own working directory, neither of which this row
duplicates.

**The `file mode` badge.** When `Dashboard::file_mode` is true — the `openspec` binary probe
resolved no usable binary, so the change list is file-sourced for the whole session — the
heading row SHALL draw the literal `file mode`, nine columns, **right-aligned** so its last
column is the heading row's last column, styled `palette::style(Role::FileMode)`: ratatui's
`DIM` modifier — and no other modifier — together with foreground `Color::Yellow`. The badge
stays dim because it names a *mode*, not a fault: file mode is a supported way to run, and a
badge competing with the repository's name for attention would say otherwise. It is coloured
because `DIM` alone is what an archived row's date, an inline code span, a block quote, and
an agent badge already are, and a badge that shares its whole style with four other things
names nothing.

The badge SHALL be dropped **whole**, never cut short, when the heading row cannot hold the
name, at least one separating blank, and the badge's nine columns together — `change-rows`'
drop-whole rule. It is dropped **before** the name is shortened, not after: at a width that
cannot hold both, the reader can still learn the mode from the pane's behaviour, and a
half-drawn `file mo` would name nothing at all. When `file_mode` is false the heading row
SHALL be byte-identical to the row this requirement specifies with no badge — no reserved
columns and no changed budget.

Let `A` be the heading row's width when no badge is drawn, and its width minus ten — the
badge's nine columns and one separating blank — floored at zero, when one is. When the
name's `layout::columns` is at most `A` it SHALL be rendered whole. When it is longer it
SHALL be shortened from the **left** by `change-rows`' shared `shorten_left` implementation:
`…` followed by the longest suffix ending on a grapheme-cluster boundary that measures at
most `A - 1` columns. A directory's own last characters are what distinguish it from its
siblings. When `A` is zero the name SHALL be omitted entirely.

The shortening branch is reachable **only when the badge was dropped**, and that is a
consequence of the drop rule rather than a second rule: a badge survives only where the name,
one blank, and nine columns all fit, which is exactly `name <= A`, which is the whole-name
branch. So `A` is the full heading width in every case that shortens, and the `width - 10`
form matters only for the cases that do not shorten. Stated here because the two rules are
written in separate paragraphs and read as though they compose.

Shortening SHALL count **display columns**, not characters and not bytes, and the heading
row SHALL never draw past its last column at any width for any repository name.

The badge SHALL be drawn on the **list** region's heading row, not on the detail region's.
The two are different claims: a missing binary is a fact about the process, true of every
change in the pane, while the detail heading describes one change — and `SPEC.md` → Degraded
states already gives the per-change equivalent its own row, "Schema unknown to the CLI",
whose fall-back to file mode is per change and is named in the detail region instead. Below
the breakpoint at `Route::Detail` the list region is not drawn at all and the badge is
therefore not drawn either; that is accepted, and is the same trade the routed-region rule
already makes for every list row.

#### Scenario: The heading names the directory, not the path, at both widths

- **WHEN** a `Dashboard` whose repository root is `/Users/dev/Code/herdr-openspec`, whose
  `file_mode` is `false`, at `Route::List`, is rendered at 120x20 and at 60x20
- **THEN** in both buffers row 0 spells exactly `herdr-openspec` from column 1, followed by
  spaces to the heading row's last column
- **AND** the string `/Users/dev/Code` appears in no cell of either buffer
- **AND** in the 120-column buffer the heading row ends at column 38 and column 39 is a
  space, so the heading stayed inside the list region's interior

#### Scenario: A name longer than the heading row keeps its tail

- **WHEN** a `Dashboard` whose repository root's final component is a 50-character
  directory name, whose `file_mode` is `false`, is rendered at 120x20
- **THEN** row 0 columns 1 through 38 spell `…` followed by the name's last 37 columns
- **AND** rendering the same dashboard at 60x20 spells the name whole from column 1, because
  50 columns fit in 58 — so the ellipsis at 120 is the narrower list column and not a
  constant

#### Scenario: The badge is right-aligned and dropped whole

- **WHEN** a `Dashboard` whose repository root is `/tmp/demo-repo` and whose `file_mode` is
  `true` is rendered at 120x20, at 60x20, at 21x20, and at 20x20
- **THEN** in the 120-column buffer row 0 spells `demo-repo` from column 1 and `file mode`
  in columns 30 through 38 — the heading row's last nine columns — and every cell of that
  badge reports `Modifier::DIM` set and foreground `Color::Yellow`
- **AND** in the 60-column buffer the badge occupies columns 50 through 58, the heading row's
  last nine, so it is right-aligned against the row rather than placed at a fixed column
- **AND** in the 21-column buffer — a heading row of 19 columns — `demo-repo`'s nine columns,
  one separating blank, and the badge's nine fit exactly: `demo-repo` occupies columns 1
  through 9, column 10 is blank, and `file mode` occupies columns 11 through 19
- **AND** in the 20-column buffer — a heading row of 18 columns — the badge is absent from
  every cell and `demo-repo` is drawn whole, so the badge was dropped whole rather than cut,
  and the pair 21/20 is the drop rule's own boundary: one column narrower than the rule
  needs is the first width at which the badge goes
- **AND** rendering the 120x20 case with `file_mode` `false` gives a row 0 byte-identical to
  the first scenario's, so the badge is additive

#### Scenario: No repository names itself in the heading

- **WHEN** a `Dashboard` whose `repo` is `None`, whose `searched_from` is
  `/tmp/not-a-repo/deep/here`, is rendered at 120x20 and at 60x20
- **THEN** row 0 spells exactly `no repository` from column 1 in both
- **AND** the list region's interior still holds the three rows `change-rows` specifies for
  that state, beginning at row 2 — row 1 is the region's padding row and is blank
- **AND** the string `not-a-repo` appears only in those interior rows and never in row 0

### Requirement: A region's heading style and the rules' style are palette roles

`ui::view` SHALL take every style it applies to a region's heading row and to either rule
from the palette, and SHALL construct no `Style` of its own.

A region's heading row SHALL be drawn with `palette::style(Role::RegionHeadingFocused)` when
the dashboard's route names that region, and with `palette::style(Role::RegionHeading)`
otherwise. `Role::RegionHeadingFocused` SHALL carry `Modifier::BOLD` and **no colour**;
`Role::RegionHeading` SHALL carry `Modifier::DIM` and **no colour**. The pair says which
region the keyboard is driving, which is exactly what the bold border said before; neither
carries a colour, because the distinction is between two regions of the same pane and a hue
would claim a meaning the region does not have.

The `file mode` badge keeps `Role::FileMode` and is drawn **over** the heading row's style
rather than under it, so the badge is dim and yellow whether or not the list region is the
routed one.

Both rules SHALL be drawn with `palette::style(Role::RegionRule)`, which SHALL carry
`Modifier::DIM` and no colour: a rule is a separator, and a separator that competes with the
text on either side of it has failed at its one job.

`Role::RegionBorder`, `Role::RegionBorderFocused`, `Role::HeaderTitle`, `Role::HeaderPath`,
and `Role::DetailHeader` SHALL be removed from `ui::palette::Role`, because nothing draws a
border, a frame header title, a frame header path, or a separately-styled detail header any
more. The removal is specified by `view-palette`, which owns the role table; this
requirement names it only so the two are not read as disagreeing.

#### Scenario: The routed region's heading is bold and the other's is dim

- **WHEN** a `Dashboard` with `route: Route::List` and one active change is rendered at
  120x20
- **THEN** the cell at column 1, row 0 reports `Modifier::BOLD` set and `Modifier::DIM` not
  set
- **AND** the cell at column 42, row 0 — the detail region's heading row's first column,
  column 41 being that region's left gutter and blank — reports
  `Modifier::DIM` set and `Modifier::BOLD` not set
- **AND** with `route: Route::Detail` and the same size the two assertions swap, so the test
  discriminates rather than asserting a constant
- **AND** at 60x20 the single drawn region's heading cell at column 1, row 0 reports
  `Modifier::BOLD` set under **both** routes, because the region that is drawn is always the
  routed one below the breakpoint

#### Scenario: No heading or rule cell carries a colour

- **WHEN** the same dashboards are rendered at 120x20 and at 60x20 with `file_mode` `false`
- **THEN** no cell of either heading row and no cell of the vertical divider at column 40
  reports a foreground or a background, so the palette gave each a role and not a colour
- **AND** every cell of the vertical divider and of the detail region's horizontal rule
  reports `Modifier::DIM` set
- **AND** with `file_mode` `true` the nine badge cells of the list heading row do report
  foreground `Color::Yellow`, so the absence of colour above is a property of the heading
  role rather than of the row

#### Scenario: Every colour literal still lives in the palette module alone

- **WHEN** `src/`, inline `#[cfg(test)]` modules included, is searched for
  `ratatui::style::Color`
- **THEN** `src/ui/palette.rs` is the only file that names it
- **AND** the render tests above assert a cell's colour by comparing it against
  `palette::style(role)` rather than against a literal
