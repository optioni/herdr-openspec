## MODIFIED Requirements

### Requirement: The visible slice follows the selection

`ui::layout::viewport(rows: usize, cursor: usize, height: u16) -> usize` SHALL return the
index of the first row to draw, as a pure total function of its three arguments. It SHALL
return `0` when `height` is `0` or when `rows` is at most `height`. Otherwise it SHALL
return `min(cursor.saturating_sub(height / 2), rows - height)`, so the selected row is
always inside the drawn slice, the slice never runs past the last row, and the value is
derived from the current frame on every draw rather than stored on `Dashboard`.

`ui::view::render` SHALL draw the rows from that offset, at most `height` of them, into the
interior. `cursor` SHALL be the index within the emitted row vector of the row carrying the
selection — not `selected` itself, since problem rows and message rows shift it. **Section
headers do not**: a header is both an emitted row and an addressable target, so within a
section-only list the two indices coincide, and they diverge exactly as far as the problem
and message rows above them.

Storing an offset on `Dashboard` is forbidden for the same reason `LayoutMode` is: the
interior height is a property of the current frame, and a stored offset would be stale
after a resize.

#### Scenario: A selection past the interior scrolls the slice at both widths

- **WHEN** a `Dashboard` holding thirty active changes named `change-00` through
  `change-29`, with `selected` **21** — target 0 is the active section header, so 21
  addresses `change-20` — is rendered at 120x20 and at 60x20 — an interior of seventeen rows
  in both
- **THEN** in both buffers the first interior row is the `change-12` row and the last is
  the `change-28` row, because the row vector is 31 long and `layout::viewport(31, 21, 17)`
  is `min(21 - 8, 31 - 17)` = 13, whose row is `change-12`
- **AND** in both buffers the `change-20` row carries the `>` marker and bold cells, so the
  selection is inside the drawn slice
- **AND** the same dashboard with the active section **collapsed** draws exactly one interior
  row, the header, with `selected` clamped to 0

#### Scenario: The last change is reachable and the slice stops at the end

- **WHEN** the same thirty-change dashboard with `selected` **30**, addressing `change-29`, is
  rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row is the `change-13` row and the last is
  the `change-29` row, because `layout::viewport(31, 30, 17)` clamps `30 - 8` to `31 - 17` = 14
- **AND** in both buffers the last interior row carries the `>` marker, and no interior row
  is blank, so the slice never runs past the final row

#### Scenario: The viewport is exact at its boundaries

- **WHEN** `ui::layout::viewport` is called with `(0, 0, 16)`, `(16, 15, 16)`, `(17, 0, 16)`,
  `(17, 8, 16)`, `(17, 9, 16)`, `(17, 16, 16)`, and `(30, 20, 0)`
- **THEN** it returns `0`, `0`, `0`, `0`, `1`, `1`, and `0` respectively
- **AND** rendering a seventeen-change dashboard with `selected` **10** at 120x20 and at
  60x20 shows `change-00` as the first interior row in both — eighteen rows with the header,
  `viewport(18, 10, 17)` = `min(10 - 8, 18 - 17)` = 1 — so the boundary is rendered and not
  only computed

  The unit calls above are unchanged: they pass `16` explicitly and pin `viewport`'s own
  arithmetic, which `pane-chrome` does not touch. What moved is the **interior height** the
  render path hands it — sixteen rows to seventeen, `responsive-layout`'s region shape — so
  every rendered expectation in this requirement is recomputed at `17` while every computed
  one stays at `16`.

#### Scenario: A resize changes the slice on the next frame

- **WHEN** one `Terminal<TestBackend>` is created at 120x20, the thirty-change dashboard
  with `selected` **21** is drawn, the backend is resized to 120x12, and a second frame is
  drawn from the **same** unchanged `Dashboard`
- **THEN** the first buffer's first interior row is the `change-12` row and the second
  buffer's is the `change-16` row, because the interior height fell from 17 to **9** —
  `viewport(31, 21, 9)` is `min(21 - 4, 31 - 9)` = 17. A 12-row frame gives a body of
  eleven rows and an interior of nine, where the bordered arithmetic gave eight; the
  half-height offset is `9 / 2` = 4, unchanged, so this scenario's rendered rows are the
  same as before this change and its stated arithmetic is not
- **AND** `Dashboard` exposes no field naming a scroll offset, a first visible row, or an
  interior height. `sections` is not such a field: it is a user decision, not derived
  geometry, which is why `list-sections` stores it and stores nothing else

### Requirement: `Space` toggles the section the cursor is on or in

`Action::ToggleSection` SHALL fold or unfold exactly one section: the one the cursor
addresses when `targets()[selected]` is a `Target::Section`, and otherwise the section the
addressed change belongs to — active for a change from `changes.active`, archived for one
from `changes.archived`. When the target list is empty, `ToggleSection` SHALL change nothing
at all and SHALL record no problem.

`Dashboard` SHALL carry the collapse state in a field, not derive it per frame:

```rust
pub struct Sections {
    pub collapsed: std::collections::BTreeSet<SectionKey>,
}

pub enum SectionKey {
    Active,
    Archived,
}
```

A section is collapsed exactly when its key is in the set, so a key added later — a date
group nested under `archived`, say — defaults to open without a migration. `Sections` SHALL
NOT implement `Default`, derived or hand-written, and every construction and destructuring
SHALL name its field with no `..` rest, on the same terms `dashboard-loop` states for
`Dashboard` and `list-filtering` states for `Filter`. `ui::load` SHALL start it with
`collapsed` holding exactly `SectionKey::Archived`: the archive is the tier this change
un-caps, and opening a session with twenty-two archived rows above the fold would replace one
bad default with another.

Collapse state is a **user decision** and SHALL survive every refresh: `Dashboard::adopt`
replaces `changes` and SHALL NOT touch `sections`. It is per-session and SHALL NOT be
persisted — nothing new is written under `HERDR_PLUGIN_STATE_DIR`, which stays exactly
`agent-names.toml`.

After a toggle, `selected` SHALL address that section's **header**. Collapsing a section the
cursor was inside would otherwise leave the cursor pointing at a change that is no longer
shown, and clamping alone would land it somewhere unrelated; moving it to the header is both
the predictable answer and the position from which the next `Space` reopens the section.

`Dashboard::apply` SHALL, after applying **any** action, set `refresh.requested` to true when
`needs_archived_refresh()` holds — the archived section is open, `changes.archived` is empty,
and `changes.archived_total` is greater than zero. It SHALL NOT clear the flag, and the rule
SHALL be written once for every action rather than for a named subset, so a future key that
opens a section cannot forget it. That is the whole mechanism by which an expanded archived
section becomes resolved: `dashboard-loop`'s loop step **2** turns the flag into a request
carrying `Dashboard::archived_scope()`, and `refresh-worker` answers it. Step 3, the
watch-invalidate path, carries the same scope for the same reason.

`apply` SHALL reach no collaborator, spawn no process, touch no filesystem, and read no clock
while handling `ToggleSection`, exactly as it does for every other action.

#### Scenario: `Space` on a header folds and unfolds that section

- **WHEN** a `Dashboard` with one active change, two resolved archived changes,
  `archived_total` 2, both sections open, and `selected` 2 — the archived header — is given a
  `ToggleSection` action, then another
- **THEN** after the first, `sections.collapsed` holds exactly `SectionKey::Archived`,
  `visible()` holds the active change alone, and `selected` is 2, still the archived header
- **AND** after the second, `sections.collapsed` is empty, `visible()` holds all three
  changes, and `selected` is still 2
- **AND** rendering at 120x20 and at 60x20 between the two actions shows `  ▸ archived (2)`
  and no archived name, and after the second shows `  ▾ archived (2)` with both names

#### Scenario: `Space` inside a section folds it and moves the cursor to its header

- **WHEN** the same dashboard with `selected` 4 — the second archived change — is given a
  `ToggleSection` action
- **THEN** `sections.collapsed` holds `SectionKey::Archived`, and `selected` is 2, addressing
  `Target::Section(Archived)`
- **AND** `selected_change()` is `None`, and rendering at 120x20 and at 60x20 puts the `>`
  marker on the archived header row in both
- **AND** the same action given at `selected` 1 — the active change — collapses the
  **active** section instead and leaves `selected` 0, so the section acted on is the one the
  cursor is in and not a fixed one

#### Scenario: An empty list makes `Space` inert

- **WHEN** a `Dashboard` whose `changes` is `changes::empty_set()` is given ten
  `ToggleSection` actions
- **THEN** `sections.collapsed` is unchanged after all ten, `selected` is `0`,
  `refresh.requested` is unchanged, and none panics
- **AND** `changes`, `route`, `detail`, `filter`, `quit`, `agents`, `agent_names`, and
  `launch` are all byte-identical to their values before the ten actions

#### Scenario: Opening an unresolved archive requests a refresh

- **WHEN** a `Dashboard` whose `changes.archived` is empty, whose `archived_total` is 22,
  whose archived section is collapsed, and whose `refresh.requested` is false has its cursor
  put on the archived header and is given a `ToggleSection` action
- **THEN** `sections.collapsed` is empty and `refresh.requested` is true
- **AND** `Dashboard::archived_scope()` is `ArchivedScope::Full`
- **AND** giving the **reverse** toggle from a resolved, open archive — twenty-two entries in
  `changes.archived` — leaves `refresh.requested` false, because a fold needs no data and
  `archived_scope()` becoming `Names` costs nothing until the next cycle
- **AND** `apply` spawned no process, started no thread, and read no file: the flag is a state
  value, and `run_loop` is what turns it into a request

#### Scenario: A refresh does not undo a fold

- **WHEN** a `Dashboard` with its archived section collapsed adopts a `RefreshResult::Files`
  and then a `RefreshResult::Merged` carrying a wholly different `ChangeSet`
- **THEN** `sections.collapsed` still holds exactly `SectionKey::Archived` after both
- **AND** rendering at 120x20 and at 60x20 after each shows the archived header collapsed,
  so a live update never reopens what the reader folded

#### Scenario: A refresh keeps the cursor on the same change

- **WHEN** a `Dashboard` with two active changes, two archived changes, `archived_total` 2,
  **both sections open**, and `selected` 4 — addressing the archived change `add-auth` —
  adopts a `RefreshResult::Files` and then a `RefreshResult::Merged`, each carrying a
  `ChangeSet` in which one further active change has appeared **above** `add-auth`
- **THEN** `selected_change()` names `add-auth` after both adoptions, and `selected` is 5 —
  the `targets()` index of `add-auth` in the new list, not its `visible()` position of 3
- **AND** the same dashboard whose selected change has **disappeared** from the adopted set
  leaves `selected` within `targets().len()`, with `selected_change()` either `None` or a
  change that is actually shown, and never an index past the end
- **AND** adopting a set with the archived section **collapsed** leaves `selected` addressing
  a target that exists, because `clamp_selection` clamps against `targets().len()` and not
  against `visible_len()`

