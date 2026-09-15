## MODIFIED Requirements

### Requirement: The cursor addresses one target — a section header or a change — by index

`ui::app::Dashboard` SHALL carry a `selected: usize` field indexing the **visible targets**,
where a target is either a section header or a visible change:

```rust
pub enum Target {
    Section(SectionKey),
    /// An index into `Dashboard::visible()`.
    Change(usize),
    /// A section header row: its content-line index and its section index.
    DetailHeader { line: usize, section: usize },
}
```

`DetailHeader` is what remains of `foldable-spec-sections`' two additions: `text-selection`
**removes** `DetailLine`, because the press that used to move the detail cursor to a
clicked line is now the press that arms a text selection, and a variant no resolver emits
is a variant that rots. `Target` therefore carries **three** members, not four. The enum is
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


#### Scenario: `Target` carries three members and no resolver emits a fourth

- **WHEN** `ui::app::Target`'s membership is read
- **THEN** it holds exactly `Section`, `Change`, and `DetailHeader`
- **AND** `ui::driver::mouse_action` emits no `Target` outside that set at any point of any
  frame, under either overlay state
- **AND** nothing in the crate names `DetailLine`, so its removal is complete rather than
  merely unreachable
