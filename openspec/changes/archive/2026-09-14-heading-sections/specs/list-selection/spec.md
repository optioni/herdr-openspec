## MODIFIED Requirements

### Requirement: `Space` toggles the section the cursor is on or in

`Action::ToggleSection` SHALL be interpreted by `Dashboard::apply` according to the current
route, exactly as `Action::Next` and `Action::Prev` already are and for the same reason — the
action is route-agnostic and only its effect is not:

- at `Route::List` it SHALL fold or unfold exactly one **list** section, as stated below;
- at `Route::Detail` it SHALL fold or unfold exactly one **artifact** section, as
  `artifact-folds` states.

This is **BREAKING**. Before this change `ToggleSection` folded a list section at either
route, so `Space` at the detail route reached past the region the reader was looking at and
moved rows in the other one — in the narrow layout, a region not drawn at all. Nothing else
about the key changes: `action_for` still maps `Char(' ')` to `ToggleSection` outside filter
mode and still types a space into the query inside it, and no second fold key is introduced.

At `Route::List`, `ToggleSection` SHALL fold or unfold exactly one section: the one the
cursor addresses when `targets()[selected]` is a `Target::Section`, and otherwise the section
the addressed change belongs to — active for a change from `changes.active`, archived for one
from `changes.archived`. When the target list is empty, it SHALL change nothing at all and
SHALL record no problem.

At `Route::Detail`, `ToggleSection` SHALL act on `detail.expanded` and SHALL NOT itself touch
`sections`, `selected`, or `refresh.requested` — the blanket archived-tier rule below, which
runs after **any** action and whose condition concerns the list's archived tier rather than
any fold, is the one stated exception. When the selected artifact is not foldable —
one section or none — it SHALL change nothing at all and SHALL record no problem, on exactly
the terms the empty target list makes it inert at the list route.

`Dashboard` SHALL carry the list's collapse state in a field, not derive it per frame:

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

After a toggle **at the list route**, `selected` SHALL address that section's **header**.
Collapsing a section the cursor was inside would otherwise leave the cursor pointing at a
change that is no longer shown, and clamping alone would land it somewhere unrelated; moving
it to the header is both the predictable answer and the position from which the next `Space`
reopens the section. `artifact-folds` states the detail route's counterpart, which moves
`detail.scroll` to the folded section's header for the same reason.

`Dashboard::apply` SHALL, after applying **any** action, set `refresh.requested` to true when
`needs_archived_refresh()` holds — the archived section is open, `changes.archived` is empty,
and `changes.archived_total` is greater than zero. It SHALL NOT clear the flag, and the rule
SHALL be written once for every action rather than for a named subset, so a future key that
opens a section cannot forget it. That is the whole mechanism by which an expanded archived
section becomes resolved: `dashboard-loop`'s loop step **2** turns the flag into a request
carrying `Dashboard::archived_scope()`, and `refresh-worker` answers it. Step 3, the
watch-invalidate path, carries the same scope for the same reason.

`apply` SHALL reach no collaborator, spawn no process, touch no filesystem, and read no clock
while handling `ToggleSection` at either route, exactly as it does for every other action.

#### Scenario: `Space` on a header folds and unfolds that section

- **WHEN** a `Dashboard` at `Route::List` with one active change, two resolved archived
  changes, `archived_total` 2, both sections open, and `selected` 2 — the archived header —
  is given a `ToggleSection` action, then another
- **THEN** after the first, `sections.collapsed` holds exactly `SectionKey::Archived`,
  `visible()` holds the active change alone, and `selected` is 2, still the archived header
- **AND** after the second, `sections.collapsed` is empty, `visible()` holds all three
  changes, and `selected` is still 2
- **AND** rendering at 120x20 and at 60x20 between the two actions shows `  ▸ archived (2)`
  and no archived name, and after the second shows `  ▾ archived (2)` with both names

#### Scenario: `Space` inside a section folds it and moves the cursor to its header

- **WHEN** the same dashboard at `Route::List` with `selected` 4 — the second archived change
  — is given a `ToggleSection` action
- **THEN** `sections.collapsed` holds `SectionKey::Archived`, and `selected` is 2, addressing
  `Target::Section(Archived)`
- **AND** `selected_change()` is `None`, and rendering at 120x20 and at 60x20 puts the `>`
  marker on the archived header row in both
- **AND** the same action given at `selected` 1 — the active change — collapses the
  **active** section instead and leaves `selected` 0, so the section acted on is the one the
  cursor is in and not a fixed one

#### Scenario: `Space` at the detail route leaves the list alone

- **WHEN** the same dashboard, its `route` set to `Route::Detail` and its selected artifact
  resolving to three spec files, is given a `ToggleSection` action
- **THEN** `sections.collapsed` is unchanged, `selected` is unchanged, and
  `refresh.requested` is unchanged
- **AND** `detail.expanded` holds the index of the section the detail cursor was on
- **AND** at 120x20 — the width band where both regions are drawn — the list region's rows
  are byte-identical before and after, so the key reached only the region the route names

#### Scenario: `Space` at the detail route is inert on a non-foldable artifact

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to one path
  **holding prose** — a file that carries no `### Requirement:` heading and is not the
  tracked-tasks artifact, so `heading-sections`' gate leaves it unsplit and it is therefore
  the one section that makes the artifact non-foldable — is given ten `ToggleSection` actions
- **THEN** the `Dashboard` is equal, field for field, to what it was before the ten
- **AND** none panics, and no problem is recorded anywhere
- **AND** the same holds for an artifact resolving to no path at all
- **AND** a `Dashboard` whose selected artifact resolves to one path holding a **spec** file
  is **not** covered by this scenario: that file splits into heading sections and is
  foldable, so `Space` acts. "Resolves to one path" stopped implying "one section" when
  `heading-sections` landed, and the fixture says which kind of file it means for that
  reason

#### Scenario: An empty list makes `Space` inert

- **WHEN** a `Dashboard` at `Route::List` whose `changes` is `changes::empty_set()` is given
  ten `ToggleSection` actions
- **THEN** `sections.collapsed` is unchanged after all ten, `selected` is `0`,
  `refresh.requested` is unchanged, and none panics
- **AND** `changes`, `route`, `detail`, `filter`, `quit`, `agents`, `agent_names`, and
  `launch` are all byte-identical to their values before the ten actions

#### Scenario: Opening an unresolved archive requests a refresh

- **WHEN** a `Dashboard` at `Route::List` whose `changes.archived` is empty, whose
  `archived_total` is 22, whose archived section is collapsed, and whose `refresh.requested`
  is false has its cursor put on the archived header and is given a `ToggleSection` action
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
