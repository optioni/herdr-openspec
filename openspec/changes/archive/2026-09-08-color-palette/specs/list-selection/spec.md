## ADDED Requirements

### Requirement: The selected row's style is a palette role, not a modifier written at the call site

`ui::view::render` SHALL style each list row by asking `ui::palette` for the row's role
rather than constructing a `Style` of its own: `Role::ListRowSelected` for the row carrying
the selection, and `Role::ListRow`, `Role::ListProblem`, `Role::ListSeparator`, or
`Role::ListMessage` for every other row, chosen by its `RowKind`.

`Role::ListRowSelected` SHALL carry `Modifier::BOLD` and **no colour**, and `Role::ListRow`
SHALL carry neither, so the rendered result is exactly what this capability already requires:
the selected change's row carries `>` in the interior's first column and `Modifier::BOLD` on
every one of its cells, and every other row carries a space in that column and no `BOLD`.
Nothing about the selection's appearance changes; what changes is that a future edit to it
happens in one table rather than at a render call site.

The selected row keeps `BOLD` on **every** cell, its agent badge cell included:
`change-rows` requires the badge to be drawn as the row's own style patched with the badge's
colour, and the badge role carries no modifier, so the patch cannot clear the row's `BOLD`.

#### Scenario: The selected row is bold and uncoloured at both mandated widths

- **WHEN** a `Dashboard` built by `ui::load` over a scratch repository with three active
  changes is rendered at 120x20 and at 60x20
- **THEN** in both buffers every cell of the first interior row reports `Modifier::BOLD` set
  and no foreground, and no cell of the second interior row reports either
- **AND** the first interior row's first column is `>` and every other interior row's first
  column is a space

#### Scenario: A badged selected row keeps its bold under the badge colour

- **WHEN** a `Dashboard` with three active changes, the selected first one badged `Working`,
  is rendered at 120x20 and at 60x20
- **THEN** in both buffers the badge cell reports `Modifier::BOLD` set **and** the foreground
  `Role::AgentBadge(Working)` carries (`Color::Green`)
- **AND** every other cell of that row reports `BOLD` and no foreground
