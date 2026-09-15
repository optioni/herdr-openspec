## ADDED Requirements

### Requirement: The footer names a released mouse capture

When `Dashboard::mouse_capture` is `false` the footer SHALL render the literal
`mouse off (m)` — **thirteen** columns — at column 0, **before** `? help`, followed by the
same two-space separator every other hint uses. When `mouse_capture` is `true` the footer
SHALL be byte-identical to the footer the existing requirement specifies, with no reserved
columns and no changed budget, so the badge is purely additive.

**It is placed first for the reason `? help` is, and the reason is stronger here.** Hints are
dropped from the end, so a badge appended after `Esc back` would be the first thing lost at
exactly the widths where the pane is hardest to read — and a reader looking at a mouse that
has stopped responding, with no badge to explain it, has been handed a bug rather than a
mode. The badge SHALL therefore never be dropped while `mouse_capture` is `false`, even when
every other hint has been dropped for width.

**It carries its own key, which no other hint does.** `mouse off` alone names a state and
offers no way out of it; the reader who pressed `m` by accident, or who has finished copying,
needs the way back in the one place they are already looking. The four extra columns are
bought deliberately.

**The width cost is stated rather than discovered.** The badge and its separator cost
**fifteen** columns. The base footer goes from 38 to 53, the reachable footer from 61 to 76,
and the reachable footer carrying an unattributed count from 77 to 92. At the mandated
60-column frame a released capture therefore drops `g focus` from a reachable footer, and
that is accepted on the same terms the capability already accepts dropping `s archive`: the
keys stay documented in `SPEC.md` → Keys, `README.md` → Keys, and the help overlay, and the
footer's job is to say the feature exists, not to be its manual.

The badge SHALL be styled `palette::style(Role::MouseOff)`, a role of this change's own. It
SHALL carry ratatui's `DIM` modifier, because — like `file mode` — it names a *mode the
reader chose*, not a fault, and a badge competing with the pane's content for attention would
say otherwise. It SHALL NOT share its full style with `Role::FileMode`: both badges can be on
screen at once, one in the list region's heading row and one in the footer, and two badges
that look identical while meaning unrelated things is the collision this placement exists to
avoid.

The badge SHALL be drawn on the **footer**, not on the list region's heading row where
`file mode` sits. The two are different claims and, decisively, different lifetimes: below the
100-column breakpoint at `Route::Detail` the list region is not drawn at all, and the detail
route is exactly where a reader releases capture in order to copy a requirement out of a spec.
A heading-row badge would be invisible at the moment it is most needed. The footer is drawn at
every width and at every route.

#### Scenario: The badge is drawn first and is additive

- **WHEN** a `Dashboard` whose `agents.reachable` is `false` and whose `mouse_capture` is
  `false` is rendered at 120x20 and at 60x20
- **THEN** in both buffers the footer row spells `mouse off (m)` in columns 0 through 12,
  columns 13 and 14 are spaces, and `? help` begins at column 15
- **AND** every cell of the badge reports `Modifier::DIM` set
- **AND** rendering the same dashboard with `mouse_capture` `true` gives a footer row
  byte-identical to the one the existing footer requirement specifies, with `? help` at
  column 0

#### Scenario: The badge survives a width that drops every hint

- **WHEN** a `Dashboard` whose `mouse_capture` is `false` is rendered at 14x20 — one column
  narrower than the badge and its separator together
- **THEN** the footer row spells `mouse off (m)` in columns 0 through 12 and draws no hint
- **AND** the footer row never draws past its last column
- **AND** at 13x20 the badge is still drawn whole, and at 12x20 the footer row draws no cell
  past column 11, so the badge is truncated by the frame rather than overflowing it

#### Scenario: A released capture drops `g focus` at the mandated narrow width

- **WHEN** a `Dashboard` whose `agents.reachable` is `true`, with no unattributed agents, and
  whose `mouse_capture` is `false` is rendered at 60x20
- **THEN** the footer spells `mouse off (m)`, `? help`, `q quit`, `Enter detail`, `Esc back`,
  and `a/c/s launch`, in that order
- **AND** `g focus` is absent from every cell, because 76 columns do not fit in 60 and hints
  drop from the end
- **AND** the same dashboard with `mouse_capture` `true` still renders `g focus`, so the drop
  is the badge's cost and not a regression in the hint budget

#### Scenario: Both badges can be on screen at once and are distinguishable

- **WHEN** a `Dashboard` whose `file_mode` is `true` and whose `mouse_capture` is `false`, at
  `Route::List`, is rendered at 120x20
- **THEN** the list region's heading row carries `file mode` right-aligned and the footer row
  carries `mouse off (m)` at column 0
- **AND** the two do not occupy the same row at any width
- **AND** `palette::style(Role::MouseOff)` and `palette::style(Role::FileMode)` are not equal,
  so a reader can tell which badge is which without reading the text
