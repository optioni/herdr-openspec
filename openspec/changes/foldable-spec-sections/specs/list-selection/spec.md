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

At `Route::Detail`, `ToggleSection` SHALL act on `detail.expanded` and SHALL NOT touch
`sections`, `selected`, or `refresh.requested`. When the selected artifact is not foldable —
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
- **AND** rendering at 120x20 and at 60x20 between the two actions shows `  > archived (2)`
  and no archived name, and after the second shows `  v archived (2)` with both names

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

- **WHEN** a `Dashboard` at `Route::Detail` whose selected artifact resolves to one path is
  given ten `ToggleSection` actions
- **THEN** the `Dashboard` is equal, field for field, to what it was before the ten
- **AND** none panics, and no problem is recorded anywhere
- **AND** the same holds for an artifact resolving to no path at all

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

### Requirement: The cursor addresses one target — a section header or a change — by index

`ui::app::Dashboard` SHALL carry a `selected: usize` field indexing the **visible targets**,
where a target is either a section header or a visible change:

```rust
pub enum Target {
    Section(SectionKey),
    /// An index into `Dashboard::visible()`.
    Change(usize),
    /// An index into `ui::detail::content_lines`' own row list.
    DetailLine(usize),
    /// A section header row: its content-line index and its section index.
    DetailHeader { line: usize, section: usize },
}
```

`DetailLine` and `DetailHeader` are `foldable-spec-sections`' two additions, and the enum is
reproduced here so one capability declares it rather than two declaring it differently. They
address the **detail** region and are therefore **not** returned by `targets()`, which
enumerates the list region's rows and nothing else; `mouse-input` states how they are
resolved and why `apply_click`'s `targets()` membership guard is scoped to the two list
variants.

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

**Every write to `selected` SHALL be in the `targets()` index space, and the two landed sites
that write it from `visible()` SHALL be corrected.** `Dashboard::adopt` reselects by name with
`self.visible().iter().position(…)` and assigns the result directly; `Dashboard::clamp_selection`
clamps against `visible_len()`. Under this requirement `visible()` position `0` is `targets()`
index `1` whenever an active header precedes it, and `2` for an archived change with both
headers above it. Both sides are `usize`, so nothing in the type system distinguishes them and
the cursor would land one or two targets above where the reader left it on **every** adopted
refresh — every watch event and every `r`.

`adopt` SHALL therefore resolve the found `visible()` position through `targets()`, selecting
the index whose `Target` is `Target::Change(pos)`, and `clamp_selection` SHALL clamp against
`targets().len().saturating_sub(1)` for both of its callers. Because no compiler check covers
this, the proving assertion is named here rather than left to a render: a refresh scenario
SHALL assert that `selected_change()` names the **same change** across a `Files` and a
`Merged` adoption. An assertion on `sections.collapsed` or on the rendered header passes with
the defect present.

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
