## ADDED Requirements

### Requirement: A worktree change's header names its branch

`ui::detail::branched_header_row(name: &str, branch: &str, schema: &str, progress:
&tasks::Progress, width: u16) -> String` SHALL return a string of exactly `width` display
columns, pure and total on `header_row`'s own terms, with no `ratatui` type. `header_row` itself
SHALL NOT change in signature or output.

The **branch cell** is `@` followed by `branch` cut to at most **16** display columns by
`ui::list::truncate_right`'s rule — when it is longer, its first 15 columns and a trailing `…` —
so the cell is at most 17 columns wide and `b` below names its actual width. `@` is the same glyph `change-rows` draws as a worktree
row's marker, so the list and the header name one fact with one character.

The result SHALL be:

- when `width - (b + 1)` is at least the width at which `header_row(name, schema, progress, ·)`
  draws its **full** form — every one of its cells present, which for a five-column schema cell,
  a twelve-column gauge, and a five-column progress cell is 26, and without a gauge (a change of
  `total == 0`) is 3 + the schema cell + the progress cell — then
  `header_row(name, schema, progress, width - (b + 1))` with a space and the branch cell inserted
  immediately after its name field, so the branch sits between the name and the schema;
- otherwise exactly `header_row(name, schema, progress, width)`: the branch cell is dropped
  **whole**, first of every cell and before the gauge, so a header too narrow to hold it is
  byte-identical to the header of a change that came from no worktree.

The branch is therefore never cut short once drawn and never costs the name a column unless
every other cell already fits. It takes no colour of its own: `ui::view` styles the whole row as
it already does.

`ui::view` SHALL call `branched_header_row` exactly when
`worktrees::member_of(&dashboard.changes.worktrees, &change.dir)` is `Some`, passing that
member's label, and `header_row` otherwise — the one derivation `worktree-overlay` defines, so
the header and the list's `@` marker cannot disagree. Every other rule this capability states
for the header row — where it is drawn, the bold-or-dim palette role it takes, and that none is
drawn when the visible list is empty — applies to `branched_header_row`'s output unchanged. The call site stays in `ui::view`, which already holds the `Dashboard`;
`ui::detail` reads no `ChangeSet`.

#### Scenario: A worktree change's header at both mandated interior widths

- **WHEN** `branched_header_row("detail-view", "feat", "tdd", Progress { completed: 4, total:
  42 }, 78)` is called, and again at `58`
- **THEN** the 78-column result is `detail-view` padded to a 46-column name field, then
  ` @feat (tdd) `, a 12-column gauge holding one `█` and eleven `░`, and ` [4/42]`, exactly 78
  display columns wide
- **AND** the 58-column result is the same grammar with a 26-column name field, exactly 58
  display columns wide
- **AND** each result with the six characters ` @feat` removed equals `header_row("detail-view",
  "tdd", Progress { completed: 4, total: 42 }, width - 6)`

#### Scenario: The branch is dropped whole before the gauge

- **WHEN** `branched_header_row("add-token-refresh", "feat", "tdd", Progress { completed: 4,
  total: 9 }, w)` is called for `w` in `78`, `58`, `32`, `31`, `26`, `13`, `7`, `1`, and `0`
- **THEN** at `78`, `58`, and `32` the result contains `@feat` whole and a 12-column gauge
- **AND** at every `w` from `31` down to `0` the result equals `header_row("add-token-refresh",
  "tdd", Progress { completed: 4, total: 9 }, w)` byte for byte, so at `31` and `26` the gauge is
  still drawn while the branch is gone
- **AND** at every width the result is exactly `w` display columns wide, and wherever it contains
  `@f` it contains the whole `@feat`

#### Scenario: A long branch name is capped, and a detached head shows its commit

- **WHEN** `branched_header_row("alpha", "feature/a-very-long-branch-name", "tdd", Progress {
  completed: 1, total: 2 }, 78)` is called, and again with branch `"a1b2c3d"`, and again at `58`
- **THEN** the first result's branch cell is exactly `@feature/a-very-…` — `@` and sixteen
  columns — and the name field is 78 − 25 − 18 = 35 columns
- **AND** the second result's branch cell is exactly `@a1b2c3d`
- **AND** every result is exactly its width in display columns

#### Scenario: The view draws the branch only for a worktree copy

- **WHEN** a `Dashboard` routed to detail, with `changes.worktrees` holding `(/w/feat, "feat")`,
  selects an active change whose `dir` is `/w/feat/openspec/changes/x`, and is rendered at 120x20
  and at 60x20, and then selects a change whose `dir` lies under the pane's own root
- **THEN** the first two frames' detail heading rows contain `@feat`, and the second pair's
  contain no `@` at all and equal the frames the same dashboard drew before `worktree-changes`
- **AND** with the repository root at `/r/.worktrees/feat`, `changes.worktrees` holding
  `(/r, "main")`, and a selected change whose `dir` is `/r/.worktrees/feat/openspec/changes/x`,
  the heading row contains no `@` at either width
