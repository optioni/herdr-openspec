# Apply-time findings — settings-window

Written at the start of the apply phase, before any implementation task ran. Three
defects in the planning package were **repaired in place** (recorded below with what
changed and why); two pre-existing drifts in the tree are **recorded only**, because
they belong to landed changes rather than to this one and an implementer will trip on
them otherwise.

`openspec validate settings-window --strict` passes after the three repairs.

## Repaired in the delta specs

### 1. `specs/mouse-input/` — two carried-forward scenarios asserted the corrected action

The requirement's own mapping table, `design.md` → Decision 8, and the two new scenarios
("A click outside the band dismisses whichever panel is open", "A click outside cancels an
edit rather than closing the panel") all say a click outside the band resolves to
`Action::Back`. Three bullets carried forward from the REMOVED requirement still said
`Action::ToggleHelp`:

- "A click outside the band dismisses it and selects nothing" — both its main `THEN` and
  its column-200 bullet;
- "The band's edges are inside it" — its only `THEN`;
- "Nothing in the overlay is mouse-only" — named `?` as a dismissal key, which it is not
  any more: `?` swaps panels, `Esc` dismisses.

All three now say `Back`. Task 8.3 is the task that performs the code change; without
this repair it would have landed a spec whose scenarios contradicted its own table.

### 2. `specs/binding-inventory/` — the shape scenario still counted six groups

"The inventory's shape is asserted, not described" sits inside the MODIFIED requirement
whose body mandates **seven** groups and **thirty-seven** bindings, and asserted six
groups, counts `5, 7, 4, 4, 5, 7` and a sum of `32`. A MODIFIED block replaces the whole
requirement, scenarios included, so this would have landed in the live spec contradicting
the table three paragraphs above it. Now seven groups, `5, 7, 4, 5, 5, 7, 4`, summing to
37, with `Scope::Settings` in the scope list.

### 3. `specs/settings-window/` — the band-scroll scenario was not satisfiable as written

"The band scrolls only when the cursor would leave it" required all three of:

1. the first visible row is `0` while the cursor's rows are visible;
2. the window advances *only once the cursor's value row would fall below the band, and by
   the least it can*;
3. the window at every step is the one `ui::layout::viewport` returns.

`ui::layout::viewport` (`src/ui/layout.rs:108`) is
`cursor.saturating_sub(height / 2).min(rows - height)` — it **centres** the cursor. At the
scenario's own 120x8 frame (band height 7, interior 5, content 7) it returns `0`, `1`, `2`
for the three settings, so it advances to `1` while the second setting's value row (content
row 3) is still visible in the window `0..=4`. Bullet 2 was therefore false against bullet
3, and bullet 3 is the one both `tasks.md` 4.5 and `dashboard-loop` bind ("the primitive the
foldable detail path already uses").

Bullet 2 is replaced with the two properties that *are* true of `viewport` and are still
falsifiable: the cursor's value row is inside the window at every step, and the window never
exceeds `content_rows - interior_height`. Bullet 3 now says the assertion is an **equality
against `viewport`**, and names the centring behaviour explicitly so the next reader does
not re-derive this.

## Recorded, not repaired — pre-existing drift the implementer will meet

Both are sentences `dashboard-loop`'s delta **carries forward** from the live spec, in a
block this change must copy anyway. They were already false at HEAD; this change neither
introduces nor fixes them, and repairing them would widen its diff into two landed
capabilities. They are written down here because a task that trusts either number will fail
to compile.

- **`ui::app::Refresh` carries four fields, not three.** `dashboard-loop` says "exactly
  **three** fields: `requested: bool`, `reload: bool`, and `problems: Vec<String>`".
  `src/ui/app.rs` has `requested`, `reload`, `startup`, `problems` — `seam-resilience` added
  `startup` without carrying the count back. Every `Refresh { … }` literal names all four.
- **`agents::AgentSnapshot` carries four fields, not three.** `dashboard-loop` says
  "exactly **three**: `agents`, `reachable`, and `problem`". `src/agents.rs` has `agents`,
  `reachable`, `stalled`, `problem` — `seam-resilience` added `stalled`. Every
  `AgentSnapshot { … }` literal names all four.

Neither is machine-bound, which is why both drifted. `Dashboard`'s own field count **is**
bound, by the compile-time companion `dashboard-loop` names, and that one is correct at
sixteen today and must reach seventeen here.
