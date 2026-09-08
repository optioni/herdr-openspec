## ADDED Requirements

### Requirement: The detail header's style is a palette role, not a modifier written at the call site

`ui::view::render` SHALL draw `ui::detail::header_row` with
`palette::style(Role::DetailHeader)` rather than constructing a `Style` at the call site.

`Role::DetailHeader` SHALL carry `Modifier::BOLD` and **no colour**, so the rendered result
is exactly what this capability already requires: every cell of the detail region's first
interior row reports `Modifier::BOLD` set, the same emphasis the frame header's `OpenSpec`
label carries. The change header names a change; the row directly below it — the artifact
tab bar — is where this change spends its colour, and a coloured header competing with the
chips would blunt exactly the distinction the chips exist to draw.

#### Scenario: The detail header is bold and uncoloured at both mandated widths

- **WHEN** a `Dashboard` at `Route::Detail` holding active changes `add-token-refresh` (4 of
  9, schema `tdd`) and `fix-empty-basket` (7 of 7), with `selected: 0`, is rendered at 120x20
  and at 60x20
- **THEN** in the 120-column buffer row 2, columns 41 through 118, is the 78-character
  `header_row("add-token-refresh", "tdd", 4 of 9, 78)`, and in the 60-column buffer row 2,
  columns 1 through 58, is the 58-character form
- **AND** every cell of the header row in both buffers reports `Modifier::BOLD` set and no
  foreground and no background
- **AND** the tab-bar row directly below it does carry a background, so the two rows are
  distinguishable and the header was not left unstyled by accident
