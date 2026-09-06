## ADDED Requirements

### Requirement: The content area names the selected change's own problems above the tab's

Four rows of `SPEC.md` → Degraded states — a schema the CLI rejects, a schema that is
unreadable or invalid, a `generates` glob outside the supported subset, and a tasks file that
cannot be read — each end "and the reason is named". The reason is named on
`changes::Change::problems`, and **nothing in the crate reads that vector**: `ui::list` renders
`launch.problems`, `refresh.problems`, and `ChangeSet::problems`, and `ui::detail` renders
`Detail::problems`. A reason nobody can read is not named.

`ui::detail::content_lines` SHALL therefore prepend, above the `Detail::problems` lines it
already emits, one `! `-prefixed line per entry of the **selected change's** own `problems`
vector, in that vector's order, padded or truncated to the width by the same
`ui::list::pad_or_truncate_right` call the existing problem lines use, and carrying the same
plain face. The full content-area order SHALL be: the change's problems, then the tab's
problems, then the body.

The change's problems lead because their lifetimes differ, on exactly the argument
`change-rows` uses for refresh-before-changeset: a `Change` problem is a standing fact about
that change — its schema does not parse, its glob is unsupported — that no tab switch alters,
while a `Detail` problem is about the artifact file the reader is looking at right now and is
replaced by the next `sync_detail`. A standing condition sits above a transient one.

`content_lines` SHALL stay a pure total function of its three arguments, performing no
filesystem, process, environment, network, or terminal I/O, reading no clock and no global
state, and never panicking for any `Detail`, any `Option<&Change>`, and any `u16` width
including `0`.

A `change` of `None` SHALL contribute no lines: with no selected change the whole detail
region is blank, which `detail-view` already requires and this change SHALL NOT weaken.

The count of lines a change contributes SHALL NOT be capped, and the leading lines SHALL be
scrolled with the body rather than pinned: `detail-scroll`'s clamp is computed against the
full line count `content_lines` returns, so a change carrying more problems than the content
area is tall stays reachable with `j`.

#### Scenario: A change whose schema will not parse names the reason in the detail region

- **WHEN** a `Dashboard` whose selected change `alpha` carries
  `problems: ["openspec/schemas/broken/schema.yaml: mapping values are not allowed here"]`,
  an empty `Detail::problems`, and a `source` of `# Proposal` is rendered at the detail route
  into a `TestBackend` at 120x20 and again at 60x20
- **THEN** the first row of the detail region's content area, at both widths, begins
  `! openspec/schemas/broken/schema.yaml: mapping values are not allowed here`, truncated to
  the 78- and 58-column interiors respectively
- **AND** the row below it spells `# Proposal`
- **AND** removing the entry from `problems` returns both buffers to the ones the same
  dashboard produced with `# Proposal` on the first content row, so the line is the change's
  and not a constant

#### Scenario: A change problem and a tab problem are both shown, change first

- **WHEN** the same dashboard carries `problems: ["alpha: unsupported glob **/*.md"]` on the
  change **and** `Detail::problems: ["design.md: Permission denied (os error 13)"]`
- **THEN** the content area's first row names the glob and its second row names the
  permission error, at both widths
- **AND** the body follows on the third row
- **AND** neither line is `No content yet`: the tab-problem rule `detail-view` set — a known
  reason replaces the placeholder rather than joining it — is unchanged by this addition

#### Scenario: No selected change contributes no lines

- **WHEN** `content_lines` is called with `change` `None` and a `Detail` whose `problems` and
  `source` are both empty
- **THEN** it returns an empty vector
- **AND** the rendered detail region is entirely blank at both widths, byte-identical to the
  buffer the same dashboard produced before this change existed

#### Scenario: The line count drives the scroll clamp

- **WHEN** a change carrying twenty problem entries is selected at the detail route, whose
  content area is fourteen rows tall, and `j` is pressed forty times
- **THEN** the stored scroll offset is clamped against the full line count — twenty problem
  lines plus the body's — rather than against the body's alone
- **AND** the last problem entry is reachable on screen, which it would not be were the
  leading lines pinned outside the scrolled region
