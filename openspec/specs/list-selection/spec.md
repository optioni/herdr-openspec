# list-selection Specification

## Purpose
Fixes what "the selected change" means: a single `usize` index into the *targets* list — the
`active` section header, the visible active changes, the `archived` section header, then the
visible archived changes, with the filter applied and a collapsed section contributing its
header alone — clamped on every action that could change either the index or the list, so it
can never address a row that is not shown, and never lands on a problem or message row, which
are not selectable at all. A section header **is** selectable, so that `Space` has something
to toggle and `j`/`k` can reach it; `selected_change()` answers `None` there, which is what
makes `Enter` and the launch keys inert on a header with no rule of their own. `Next` and `Prev` move it
only at the list route, clamping at both ends rather than wrapping; at the detail route the same
keys scroll instead. The second half is the viewport: `layout::viewport` derives the first drawn
row from the row count, the cursor's row, and the current interior height on every frame, so the
selection is always on screen and no scroll offset is stored on `Dashboard` where a resize could
stale it.

## Requirements

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

### Requirement: A click moves the cursor to the row it lands on

`Action::Click(Target::Change(i))` SHALL move `selected` to the position of
`Target::Change(i)` in `targets()` and reset `detail.tab` and `detail.scroll` to `0`,
exactly as a selection move by `j` or `k` does — and, exactly as those do, SHALL reset
neither when the cursor was already on that row.

A click SHALL never leave `selected` addressing a target that is not in `targets()`: an
action naming a target that is absent — a change filtered away, or a section folded, between
the draw and the event — SHALL change nothing at all rather than clamping to a neighbour.
Clamping would move the cursor somewhere the reader did not click, which is worse than
ignoring a click whose row is gone.

`Action::Click` SHALL change nothing but `selected`, `route`, `detail.tab`,
`detail.scroll`, `sections.collapsed`, and `refresh.requested` — the last only through the
blanket rule `apply` runs after **every** action, never written by a `Click` arm itself, and
so reachable only by a `Target::Section` click that opens an unresolved archive. It SHALL
reach no collaborator, spawn no process, touch no filesystem, and read no clock.

#### Scenario: A click on an unselected change moves the cursor and resets the tab

- **WHEN** a dashboard with four active changes has the cursor on the first change,
  `detail.tab` `3`, and `detail.scroll` `9`, and `Action::Click(Target::Change(2))` is
  applied
- **THEN** `selected` addresses `Target::Change(2)`, `detail.tab` is `0`, and
  `detail.scroll` is `0`
- **AND** `route`, `filter`, `changes`, `agents`, `agent_names`, and `launch` are unchanged

#### Scenario: A click on the already-selected change resets nothing

- **WHEN** the cursor is already on `Target::Change(2)` with `detail.tab` `3` and
  `detail.scroll` `9`, `route` is `Route::Detail`, and the same action is applied
- **THEN** `selected`, `detail.tab`, and `detail.scroll` are all unchanged

#### Scenario: A click naming a target that is gone changes nothing

- **WHEN** `Action::Click(Target::Change(7))` is applied to a dashboard whose `targets()`
  holds four entries, and then `Action::Click(Target::Section(SectionKey::Archived))` is
  applied to one with no archived changes at all
- **THEN** neither call changes any field of the dashboard
- **AND** neither panics

#### Scenario: A click reaches a row a collapsed section hides only when it is open

- **WHEN** the archived section is collapsed and `Action::Click(Target::Change(i))` is
  applied for an `i` belonging to an archived change
- **THEN** the target is absent from `targets()` and nothing changes
- **AND** after the section is opened, the same action moves the cursor to that row

### Requirement: A click on a section header toggles it exactly as `Space` does

`Action::Click(Target::Section(key))` SHALL fold `key`'s section when it is open and unfold
it when it is collapsed, and SHALL move `selected` to that section's header — the same two
effects `Action::ToggleSection` produces for a cursor already addressing that header, and
through the same code, so the two can never diverge.

Every consequence `Space` carries SHALL carry here too: opening an archived section whose
`changes.archived` is empty while `changes.archived_total` is greater than zero SHALL set
`refresh.requested`, through the same blanket rule `apply` runs after every action; a
resolved section SHALL cost no further cycle; and a section toggled by click SHALL survive
every `adopt` exactly as one toggled by key does.

A click on a header SHALL be inert when that header is not in `targets()` — a section whose
count is zero draws no header, so there is nothing to have clicked.

#### Scenario: Click and `Space` produce equal dashboards

- **WHEN** a dashboard with three active and three archived changes is driven once by
  `Action::Click(Target::Section(SectionKey::Archived))` and once by moving the cursor to
  the archived header and applying `Action::ToggleSection`
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** the same holds for the active section, and for a second toggle that unfolds each
  again

#### Scenario: Clicking open an unresolved archive requests the refresh that resolves it

- **WHEN** the archived section is collapsed, `changes.archived` is empty,
  `changes.archived_total` is `22`, and `Action::Click(Target::Section(SectionKey::Archived))`
  is applied
- **THEN** the section is open, `selected` addresses its header, and `refresh.requested` is
  `true`
- **AND** applying the same action again folds it, and `refresh.requested` is not set a
  second time by the fold

#### Scenario: A click on a header that is not drawn is inert

- **WHEN** a dashboard with active changes and no archived ones at all receives
  `Action::Click(Target::Section(SectionKey::Archived))`
- **THEN** no field changes, and `sections.collapsed` in particular is untouched

### Requirement: A second click on the selected change opens its detail

`Action::Click(Target::Change(i))` SHALL set `route` to `Route::Detail` and reset
`detail.scroll` to `0` when `selected` already addresses that target and `route` is
`Route::List` — exactly what `Action::OpenDetail` does — and SHALL change nothing when
`route` is already `Route::Detail`.

The rule SHALL be the same above and below the breakpoint. Above it the detail region is
drawn at both routes, so the visible effect of the second click is that `j`, `k`, and the
arrows begin scrolling the content instead of moving the selection; below it the second
click is what replaces the list region with the detail region.

A section header SHALL NOT open on a second click, on the same terms `Enter` on a header
does nothing: there is no detail for a section, and a second click folds it again instead.

#### Scenario: The second click opens and the third does nothing

- **WHEN** a dashboard at `Route::List` at 60x20 receives `Action::Click(Target::Change(1))`
  three times in a row
- **THEN** after the first, `selected` addresses that target and `route` is `Route::List`
- **AND** after the second, `route` is `Route::Detail` with `detail.scroll` `0`
- **AND** after the third, nothing has changed since the second

#### Scenario: The second click matches `Enter` exactly

- **WHEN** a dashboard whose cursor is already on `Target::Change(1)` is driven once by
  `Action::Click(Target::Change(1))` and once by `Action::OpenDetail`
- **THEN** the two resulting `Dashboard` values are equal, field for field

#### Scenario: Two clicks on a section header fold and unfold it

- **WHEN** `Action::Click(Target::Section(SectionKey::Active))` is applied twice
- **THEN** the section is folded and then unfolded, and `route` is `Route::List` throughout
