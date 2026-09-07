## ADDED Requirements

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

**The Unicode promise the pane makes, stated rather than left to be discovered.** A grapheme
cluster occupies the columns ratatui gives it, and no line the view produces ever exceeds
the region it is drawn into. Where a terminal disagrees with ratatui — most commonly a
zero-width-joiner emoji sequence that the terminal composes into fewer columns than ratatui
budgets for it — the pane SHALL be the party that over-reserves: the row ends in trailing
blanks rather than overflowing its region. The pane SHALL NOT reorder bidirectional text,
SHALL NOT probe the terminal for its capabilities, and SHALL NOT tailor any measurement to a
locale.

No file under `src/ui/` other than `src/ui/layout.rs` SHALL measure a rendered string with a
`char` count. Concretely, no **production** line of `src/ui/app.rs`, `src/ui/detail.rs`,
`src/ui/list.rs`, `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, or
`src/ui/driver.rs` SHALL name `.chars().count()`, `.chars().take(`, or a
`Vec<char>`-producing `.chars().collect()`. Iterating characters for a purpose that is not
measurement — `ui::markdown`'s per-character scanner, for one — is untouched by this rule.

#### Scenario: `columns` agrees with what the buffer consumed

- **WHEN** for each of `abc`, `日本語`, `🎉`, `e` followed by U+0301 COMBINING ACUTE ACCENT,
  a family emoji joined by two zero-width joiners, `ｶ` followed by U+FF9E HALFWIDTH KATAKANA
  VOICED SOUND MARK, and the empty string, the string is written with `Buffer::set_string`
  at `(0, 0)` into a fresh 40x1 `Buffer` and the index of the first cell whose symbol is the
  buffer's reset symbol is taken
- **THEN** that index equals `layout::columns` of the same string for every one of them
- **AND** `columns("")` is `0`, and `columns` of a string holding only U+0007 BEL is `0`,
  because `styled_graphemes` drops control clusters exactly as `set_string` does

#### Scenario: `truncate_columns` never splits a cluster and never overruns

- **WHEN** `truncate_columns` is called with `日本語の変更` at every `max` from `0` through
  `14`, and with `abc🎉def` at every `max` from `0` through `10`
- **THEN** at every `max` the result's `columns` is at most `max`, the result is a prefix of
  the input, and slicing it back out of the input succeeds — so no call ever cut a cluster
- **AND** at `max` `3` for `日本語の変更` the result is `日` and measures `2`, one short of
  `max`, because the second cluster would have overrun
- **AND** at `max` `0` every result is `""`, and at a `max` at or above the input's own
  `columns` every result is the whole input

#### Scenario: Nothing under `src/ui/` measures in characters except the primitives

- **WHEN** the production lines of `src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/list.rs`,
  `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` — every
  line up to each file's `#[cfg(test)]` module — are searched for `.chars().count()`,
  `.chars().take(`, and a `Vec<char>` `.chars().collect()`
- **THEN** there is no match in any of the seven
- **AND** as a positive control `src/ui/layout.rs` names `cell_width` and
  `styled_graphemes`, so a search pattern that matched nothing because it was misspelled
  fails the check rather than passing it
- **AND** the check is a repository file under `scripts/gates/` named in the `Makefile`'s
  `gates:` recipe, so `tests/ci_workflow.rs`'s recipe-versus-directory assertion covers it
  and it cannot silently drop out of `make gates`

## MODIFIED Requirements

### Requirement: The header names the repository root, shortened from the left when narrow

The header row SHALL render, right-aligned so that its last column sits in the final
column, the repository root's display path when one was found, and the literal
`no repository` when none was. Exactly one blank column SHALL separate the `OpenSpec`
label from the shortened text at minimum.

**The `file mode` badge.** When `Dashboard::file_mode` is true — the `openspec` binary probe
resolved no usable binary, so the change list is file-sourced for the whole session — the
header SHALL draw the literal `file mode`, nine columns, immediately after the `OpenSpec`
label and one separating blank, in columns 9 through 17, styled with ratatui's `DIM`
modifier and no other. The badge is dim because it names a *mode*, not a fault: file mode is
a supported way to run, and a badge competing with the repository path for attention would
say otherwise.

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
- **AND** every one of those nine cells carries ratatui's `DIM` modifier, and the `OpenSpec`
  label's eight cells do not, so the badge is distinguishable from the label by style as well
  as by position
- **AND** the 60-column header spells `/tmp/demo-repo` in columns 46 through 59 and the
  120-column header in columns 106 through 119 — unchanged, because a fourteen-character path
  fits inside `A` at both widths either way

#### Scenario: A false flag renders the header that landed before this change

- **WHEN** the same `Dashboard` is rendered with `file_mode` **false** at 60x20 and at 120x20
- **THEN** neither buffer contains the substring `file mode` anywhere, in any row
- **AND** both buffers are byte-identical, cell for cell and style for style, to the ones the
  same dashboard produced before this change existed

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

- **WHEN** a `Dashboard` whose repository root is `/home/dev/日本語のリポジトリ名前です` —
  thirty-six characters and forty-eight display columns — is rendered at 60x20 and at
  120x20, and again with `file_mode` true
- **THEN** in every one of the four buffers the header row's last drawn column is the frame's
  final column and no cell beyond it is written
- **AND** in the 60-column, non-badged buffer the shortened text begins with `…` at a column
  no earlier than 9 and ends in the final column, and its `columns` is at most `A` — 51 —
  so a character count would have over-shortened by twelve columns and a byte count by
  many more
- **AND** in the 120-column buffer the whole path is drawn, its first column no earlier than
  column 50, and no ellipsis appears in that row
- **AND** rendering the same dashboard at 16x20, 18x20, 19x20, and 1x20 draws only the
  label or a truncation of it, writes nothing past the last column, and does not panic
