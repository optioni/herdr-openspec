## MODIFIED Requirements

### Requirement: A left click selects a row, opens it, toggles a section, or switches a tab

A `MouseEventKind::Down(MouseButton::Left)` SHALL be resolved by where it lands:

| Where the press lands | Action |
|---|---|
| A drawn change row in the list region's interior, other than the selected one | `Action::Click(Target::Change(i))` for that row's own `visible()` index |
| The drawn change row that already carries the cursor | `Action::Click(Target::Change(i))` for the same index |
| A drawn section-header row in the list region's interior | `Action::Click(Target::Section(key))` for that header's own key |
| A drawn tab cell in the detail region's tab-bar row | `Action::SelectTab(i)` for that cell's own artifact position |
| A drawn artifact-section header row in the detail region's content area, when the selected artifact is foldable | `Action::Click(Target::DetailHeader { line, section })` for that row's own content-line index and section index |
| Any other drawn row of the detail region's content area, when the selected artifact is foldable | `Action::Click(Target::DetailLine(line))` for that row's own content-line index |
| A problem row, a message row, an interior row past the last drawn row, a region's gutter, heading row or padding row, the divider, the detail region's rule row or content padding row, the detail content area when the selected artifact is **not** foldable, the detail content area below its last drawn line, the frame footer, or outside the frame | `Action::Ignore` |

`Target` SHALL gain exactly two variants for this:

```rust
Target::DetailLine(usize),
Target::DetailHeader { line: usize, section: usize },
```

The tracked-tasks tab is no longer excluded from those two rows. Before
`heading-sections` it was never foldable at any section count, so every press in its content
area resolved to `Action::Ignore`; now a task file carrying a heading and an item splits into
sections, the tab is foldable like any other, and a press on a group heading folds that group.
Nothing in this table changed to allow it — the table was already written in terms of
`Detail::foldable()`, and that predicate simply started answering `true` for one more tab.

Both carry indices **already resolved against the frame just drawn**. `mouse_action` has the
content area's width and can call `ui::detail::content_lines` and
`ui::detail::section_at`; `Dashboard::apply` has neither and SHALL NOT recompute either. That
is why the header variant carries its line index beside its section index rather than leaving
`apply` to derive one from the other.

`Dashboard::apply(Action::Click(target))` SHALL:

- do nothing at all when `target` is a `Target::Change` or a `Target::Section` that is not
  present in `targets()`;
- when `target` is `Target::Section(key)`, move the cursor to that header and fold the
  section if it is open, unfold it if it is collapsed — exactly what `Action::ToggleSection`
  at `Route::List` does for a cursor already on that header, so a click and a `Space` on the
  same header are indistinguishable in their effect;
- when `target` is `Target::Change(i)` and the cursor is **not** already on that row, move
  the cursor to it and reset `detail.tab` and `detail.scroll` to zero, exactly as a
  selection move by `j` or `k` does;
- when `target` is `Target::Change(i)`, the cursor is already on that row, and the route is
  `Route::List`, set the route to `Route::Detail` and reset `detail.scroll` to zero —
  exactly what `Enter` does — so a second click on a selected row opens it;
- when `target` is `Target::Change(i)`, the cursor is already on that row, and the route is
  already `Route::Detail`, change nothing;
- when `target` is `Target::DetailLine(line)`, set `detail.scroll` to `line` and change
  nothing else — not `route`, not `selected`, not `detail.tab`, not `detail.expanded` — so a
  click in the content area moves that region's cursor exactly as a click on a list row moves
  the list's, at **both** routes and without changing which region the keys address;
- when `target` is `Target::DetailHeader { line, section }`, set `detail.scroll` to `line`
  and then toggle `section` through the very code `Action::ToggleSection` at `Route::Detail`
  runs, so a click and a `Space` on the same artifact header can never diverge — including
  that arm's own rule, stated in `artifact-folds`, that `detail.scroll` ends on the toggled
  section's header row;
- when `target` is `Target::DetailLine` or `Target::DetailHeader` and the selected artifact
  is not foldable, change nothing at all: `mouse_action` does not emit either variant there,
  and `apply` SHALL be inert rather than trusting it, on the same terms it checks
  `targets()` for the other two.

A `MouseEventKind::Down` of `MouseButton::Right` or `MouseButton::Middle` SHALL produce
`Action::Ignore`: there is no context menu, and no mouse gesture starts a process.

#### Scenario: A click selects a change row and a second click opens it

- **WHEN** a dashboard with four active changes is drawn at 120x40 at `Route::List` with
  the cursor on the active header, and a left press lands on the third drawn row (the
  second change)
- **THEN** `mouse_action` returns `Action::Click(Target::Change(1))`
- **AND** applying it sets `selected` to that row's target index, `detail.tab` to `0`, and
  `detail.scroll` to `0`, leaving the route `Route::List`
- **AND** a second left press on the same row returns the same action, and applying it sets
  the route to `Route::Detail` with `detail.scroll` at `0` and `selected` unchanged
- **AND** a third left press on the same row returns the same action, and applying it
  changes nothing at all

#### Scenario: A click on a section header folds it exactly as `Space` does

- **WHEN** a dashboard with three active and three archived changes is drawn at 120x40 and
  a left press lands on the `active` header row
- **THEN** `mouse_action` returns `Action::Click(Target::Section(SectionKey::Active))`
- **AND** applying it collapses the active section and leaves `selected` addressing that
  header
- **AND** the resulting `Dashboard` is equal, field for field, to one produced by moving the
  cursor to that header and applying `Action::ToggleSection` at `Route::List`
- **AND** a second click on the same header unfolds it again

#### Scenario: A click on an archived header opens an unresolved archive and requests its refresh

- **WHEN** a dashboard whose archived section is collapsed, whose `changes.archived` is
  empty, and whose `changes.archived_total` is `22` is drawn at 120x40, and a left press
  lands on the `archived` header row
- **THEN** applying the returned action opens the section
- **AND** `dashboard.refresh.requested` is `true`, so the click that opened the archive is
  what asks for its resolution, on the same terms `Space` does

#### Scenario: A click on an artifact-section header folds it exactly as `Space` does

- **WHEN** a dashboard whose selected artifact resolves to three spec files, every section
  collapsed, is drawn at 120x40 at `Route::Detail`, and a left press lands on the second
  content row
- **THEN** `mouse_action` returns `Action::Click(Target::DetailHeader { line: 1, section: 1 })`
- **AND** applying it puts `1` in `detail.expanded` and leaves `detail.scroll` at `1`
- **AND** the resulting `Dashboard` is equal, field for field, to one produced by setting
  `detail.scroll` to `1` and applying `Action::ToggleSection` at `Route::Detail`
- **AND** a second click on the same row folds it again, and `sections.collapsed` and
  `selected` are unchanged throughout
- **AND** the same press at `Route::List` on a 120x40 frame — where the wide layout draws
  both regions — returns and applies the same action, so the detail region's headers are
  clickable at either route

#### Scenario: A click in an open section's body moves the detail cursor and folds nothing

- **WHEN** the same dashboard with `detail.expanded` holding `0` is drawn at 120x40 and a
  left press lands on the row carrying that section's third rendered body line
- **THEN** `mouse_action` returns `Action::Click(Target::DetailLine(l))` for that row's own
  content-line index
- **AND** applying it sets `detail.scroll` to `l` and leaves `detail.expanded`, `route`,
  `selected`, and `detail.tab` unchanged
- **AND** a `ToggleSection` action given immediately afterwards folds section `0`, because
  the click left the cursor inside it

#### Scenario: A click on a non-foldable tab's content is inert

- **WHEN** a dashboard whose selected artifact resolves to one path holding a twenty-item
  list is drawn at 120x40 at `Route::Detail`, and left presses land on the first, fifth, and
  last drawn content rows
- **THEN** every call returns `Action::Ignore`
- **AND** the dashboard is unchanged after applying all three, so a single-file artifact's
  content is exactly as click-inert as it was before this change

#### Scenario: A click on a tab cell switches to that artifact

- **WHEN** a change carrying the five `tdd` artifacts is selected, the dashboard is drawn at
  120x40 with `detail.tab` at `0` and a non-zero `detail.scroll`, and a left press lands
  inside the third tab cell's own painted columns
- **THEN** `mouse_action` returns `Action::SelectTab(2)`
- **AND** applying it sets `detail.tab` to `2` and `detail.scroll` to `0`
- **AND** a press on the one separating column between two cells returns `Action::Ignore`
- **AND** a press on the tab bar of a change with no artifacts — where the bar holds only
  the `no artifacts` placeholder — returns `Action::Ignore`

#### Scenario: Clicks that address nothing are inert

- **WHEN** a dashboard whose `changes.problems` holds one entry and whose visible list is
  empty is drawn at 120x40, and left presses land on the problem row, on the `No changes
  yet` message row, on an interior row below the last drawn row, on the list region's
  left gutter, on the list region's heading row, on its padding row, on the detail region's
  heading row, on the detail region's rule row, on its content padding row, on the detail
  content area — which holds no foldable artifact, because no change is selected — on the
  frame's footer, and at column 200
- **THEN** every call returns `Action::Ignore`
- **AND** applying `Action::Ignore` leaves the dashboard equal to what it was
- **AND** a press on a detail content row **below** the last drawn line of a foldable
  artifact returns `Action::Ignore` too, so an empty region under three header rows is not a
  fourth section

#### Scenario: The other buttons and the non-press kinds are inert

- **WHEN** `mouse_action` is called over a drawn change row with `Down(Right)`,
  `Down(Middle)`, `Up(Left)`, `Drag(Left)`, and `Moved`, and again over a drawn
  artifact-section header row with the same five
- **THEN** every call returns `Action::Ignore`
- **AND** in particular no press of any button reaches `Action::LaunchApply`,
  `Action::LaunchContinue`, `Action::LaunchArchive`, or `Action::FocusAgent`, so a
  mis-click cannot start or focus an agent

#### Scenario: A click on a task group's header folds that group

- **WHEN** a dashboard at `Route::Detail` whose selected artifact carries
  `tracks_tasks == true` and whose file reads
  `## 1. Done\n\n- [x] a\n\n## 2. Doing\n\n- [ ] b\n` is drawn at 120x40 and at 60x40,
  and a left press lands on the `> 1. Done` header row
- **THEN** `mouse_action` returns `Action::Click(Target::DetailHeader { line, section })` for
  that row's own content-line index and a `section` of `0`
- **AND** applying it opens that group, sets `detail.scroll` to the group's header row, and
  leaves `route`, `selected`, and `detail.tab` unchanged
- **AND** a left press on the progress-bar row or on its blank line returns
  `Action::Click(Target::DetailLine(line))` for that row's own index, per the table above:
  both are drawn rows of a foldable content area, and the table sends every drawn row that is
  not a header there. Neither belongs to a section, so applying it moves `detail.scroll` and
  folds nothing, and `Space` from where it lands is inert
- **AND** the same two presses against a dashboard whose task file holds items but no heading
  — which does not split, so the tab is not foldable — both return `Action::Ignore`

#### Scenario: A click on a nested scenario header folds only that scenario

- **WHEN** a dashboard at `Route::Detail` whose selected artifact resolves to one delta spec
  path, with `detail.expanded` holding the indices of the operation heading and its first
  requirement, is drawn at 120x40 and at 60x40, and a left press lands on the
  `    > Scenario: A works` row
- **THEN** `mouse_action` returns `Action::Click(Target::DetailHeader { line, section })`
  whose `section` is that scenario's own index into `detail.sections`, not its position among
  the drawn rows
- **AND** applying it opens that scenario and leaves every other section's membership of
  `detail.expanded` exactly as it was
- **AND** a left press on one of that scenario's body rows returns
  `Action::Click(Target::DetailLine(line))`, which moves `detail.scroll` and folds nothing
