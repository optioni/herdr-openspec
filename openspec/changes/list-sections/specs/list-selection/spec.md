## REMOVED Requirements

### Requirement: One change is selected, addressed by index into the visible list

**Reason**: `list-sections` makes section headers selectable, because a collapsed section's
header is its only row and there would otherwise be no way to reopen it. `selected` therefore
stops indexing changes and starts indexing **targets** — section headers and visible changes
in emission order — and the requirement's title, its `visible()` definition, its clamp, and
its "neither the separator row nor a problem row nor a message row is selectable" sentence
all change together. The ADDED requirement below restates every one of them, and keeps every
landed rule that did not change: `Next`/`Prev` move only at `Route::List`, they clamp at both
ends rather than wrapping, and the selected row carries `>` and `Modifier::BOLD`.

**Migration**: A reader sees `j`/`k` stop on two extra rows. In the crate, every construction
of a `Dashboard` that sets `selected` to address the *n*th change must account for the
section headers above it; `Dashboard::selected_change()` keeps its signature and returns
`None` when the cursor is on a header, which is the one call every consumer already goes
through.

## ADDED Requirements

### Requirement: The cursor addresses one target — a section header or a change — by index

`ui::app::Dashboard` SHALL carry a `selected: usize` field indexing the **visible targets**,
where a target is either a section header or a visible change:

```rust
pub enum Target {
    Section(SectionKey),
    /// An index into `Dashboard::visible()`.
    Change(usize),
}
```

`Dashboard::targets(&self) -> Vec<Target>` SHALL return them in exactly the order
`change-rows` emits their rows: the active section header when that section's count is
greater than zero, then the visible active changes when that section is open, then the
archived section header on the same condition, then the visible archived changes when that
section is open. It SHALL be a pure total function of `&self`, stored nowhere.

`Dashboard::visible(&self) -> Vec<&Change>` SHALL return the changes that are **shown**: the
query-matching entries of `changes.active` when the active section is open, followed by the
query-matching entries of `changes.archived` when the archived section is open. A closed
section's changes are not visible, so folding a section that is already resolved removes its
entries from `visible()` on the same frame rather than waiting for a refresh, and
`RowKind::Item { index }` keeps indexing exactly this list.

`ui::load` SHALL start `selected` at `0`.

`selected` SHALL be clamped to `targets().len().saturating_sub(1)` by `Dashboard::apply` on
every action that can change either the cursor or the target list, so no code path can leave
it addressing a target that is not shown. When the target list is empty, `selected` SHALL be
`0` and no row SHALL carry a selection marker.

The actions that move the cursor are `Next` and `Prev` — renamed from `SelectNext` and
`SelectPrev` by `markdown-viewer` — and they move it **only while `route` is `Route::List`**.
At `Route::Detail` the same two actions scroll the detail content instead, per
`detail-scroll`, and leave `selected` untouched.

Neither a problem row nor a message row SHALL be selectable: the cursor addresses sections
and changes, not rows. A **section header** is selectable, which is `list-sections`' change to
this rule and the reason `Space` has something to act on.

`Dashboard::selected_change(&self) -> Option<&Change>` SHALL return the change when the
cursor addresses a `Target::Change`, and `None` when it addresses a `Target::Section` or the
target list is empty. Every consumer that needs a change — `sync_detail`, `SelectTab`,
`NextTab`, and `agent-launch`'s four action keys — SHALL go on reaching it through that one
call, so a header cursor makes each of them inert rather than needing a rule of its own:
`launch::decide` already returns `Decision::Nothing` with no selected change, and no problem
is recorded.

`Action::OpenDetail` SHALL change nothing at all — not `route`, not `detail` — while the
cursor addresses a `Target::Section`. Moving to a detail region that has no change to show
would be a worse answer to `Enter` than doing nothing.

The row carrying the cursor SHALL carry `>` in the interior's first column and SHALL be drawn
with `Modifier::BOLD` set on every one of its cells, whether it is a change row or a section
header; every other row SHALL carry a space in that column and SHALL NOT have
`Modifier::BOLD` set.

#### Scenario: The first target is selected on startup at both widths

- **WHEN** a `Dashboard` built by `ui::load` over a scratch repository with three active
  changes and no archive is rendered at 120x20 and at 60x20
- **THEN** in both buffers the first interior row is the active section header, its first
  column is `>`, and every other interior row's first column is a space
- **AND** in both buffers every cell of the first interior row reports `Modifier::BOLD` set,
  and no cell of the second interior row does
- **AND** `selected_change()` is `None`, and `targets()` is
  `[Section(Active), Change(0), Change(1), Change(2)]`

#### Scenario: `j`, `k`, and the arrows move the cursor over headers and changes

- **WHEN** a `Dashboard` at `Route::List` with three active changes, no archive, and
  `selected` 0 is given, in turn, the action for a Press of `Char('j')`, then a Press of
  `Down`, then a Press of `Char('k')`, then a Press of `Up`
- **THEN** `selected` is 1, then 2, then 1, then 0
- **AND** `selected_change()` is `Some("add-token-refresh")` at `selected` 1 and `None` at
  `selected` 0, so the cursor stepped off the header onto the first change
- **AND** rendering the dashboard at 120x20 and at 60x20 after the second action puts the
  `>` marker on the third interior row in both buffers, and `Modifier::BOLD` on that row's
  cells rather than the first's
- **AND** `detail.scroll` is `0` throughout, so the list route's keys never touched the
  detail offset

#### Scenario: The cursor clamps at both ends rather than wrapping

- **WHEN** a `Dashboard` at `Route::List` with three active changes and no archived changes
  is given five consecutive `Next` actions and then five consecutive `Prev` actions
- **THEN** `selected` is 3 after the five `Next` actions — never 4 and never 0 — and 0
  after the five `Prev` actions, because the four targets are the header and the three
  changes
- **AND** rendering at 120x20 and at 60x20 after the five `Next` actions puts the marker on
  the fourth interior row in both, so the clamp is visible and not merely arithmetic

#### Scenario: The cursor crosses the archived header into the archived rows

- **WHEN** a `Dashboard` at `Route::List` with one active change `fix-empty-basket`, two
  archived changes `add-auth` and `legacy-cleanup`, `archived_total` 2, and both sections
  open is given four `Next` actions
- **THEN** `selected` is 4, `targets()[4]` is `Target::Change(2)`, and `selected_change()` is
  `Some("legacy-cleanup")`
- **AND** `selected` 2 addresses `Target::Section(Archived)` and `selected_change()` is
  `None` there, so the archived header is a stop rather than a row stepped over
- **AND** rendering at 120x20 and at 60x20 at `selected` 4 puts the `>` marker on the
  `legacy-cleanup` row in both, and no other row's cells are bold

#### Scenario: A collapsed section's changes are neither visible nor addressable

- **WHEN** the same five-target dashboard has its archived section collapsed
- **THEN** `visible()` holds `fix-empty-basket` alone, `targets()` is
  `[Section(Active), Change(0), Section(Archived)]`, and `selected` clamps to at most 2
- **AND** `selected` 2 addresses `Target::Section(Archived)` and `selected_change()` is
  `None`
- **AND** rendering at 120x20 and at 60x20 shows three interior rows and neither `add-auth`
  nor `legacy-cleanup` anywhere, so a collapsed section costs no row and no index

#### Scenario: Navigation over an empty visible list is inert

- **WHEN** a `Dashboard` at `Route::List` whose `changes` is `changes::empty_set()` is given
  a `Next` action and then a `Prev` action
- **THEN** `selected` is 0 after both, `targets()` is empty, `selected_change()` is `None`,
  and neither panics
- **AND** rendering at 120x20 and at 60x20 shows the `No changes yet` message row with a
  space — not `>` — in the interior's first column, and no bold cell anywhere in the
  interior

#### Scenario: `Enter` on a section header does nothing

- **WHEN** a `Dashboard` at `Route::List` with three active changes and `selected` 0 — the
  active section header — is given a Press of `Enter`
- **THEN** `route` is still `Route::List`, `detail.tab` and `detail.scroll` are both `0`, and
  `quit` is false
- **AND** the same dashboard with `selected` 1 given the same key sets `route` to
  `Route::Detail`, so the inertness is the header rather than the key being unbound
- **AND** Presses of `a`, `c`, `s`, and `g` at `selected` 0 leave `launch.pending` `None` and
  `launch.problems` empty, because `selected_change()` is `None` and `launch::decide` returns
  `Decision::Nothing`

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
section becomes resolved: `dashboard-loop`'s step 3 turns the flag into a request carrying
`Dashboard::archived_scope()`, and `refresh-worker` answers it.

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
- **AND** rendering at 120x20 and at 60x20 between the two actions shows `  > archived (2)`
  and no archived name, and after the second shows `  v archived (2)` with both names

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
