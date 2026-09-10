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

### Requirement: Nothing becomes mouse-only

Every action the mouse can produce SHALL remain reachable by key, and **no key SHALL change
its meaning because of a mouse binding**. `Action::SelectNext`, `Action::SelectPrev`,
`Action::ScrollDown`, and `Action::ScrollUp` are what `j`, `k`, and the arrows already reach
through `Action::Next` and `Action::Prev` at the matching route;
`Action::Click(Target::Section(_))` is what `Space` at `Route::List` already reaches;
`Action::Click(Target::Change(_))` is what `j`/`k` and `Enter` already reach;
`Action::SelectTab(i)` is what `1`–`9`, `[`, and `]` already reach;
`Action::Click(Target::DetailHeader { .. })` is what `Space` at `Route::Detail` reaches, and
`Action::Click(Target::DetailLine(_))` is what `j`/`k` and the arrows reach there.

The clause is narrowed from "no key SHALL change its meaning" to "no key SHALL change its
meaning **because of a mouse binding**", and the narrowing is this change's, stated rather
than left as a contradiction. `foldable-spec-sections` does change one key's meaning —
`Space` at `Route::Detail` folds an artifact section instead of a list section, marked
**BREAKING** in its own proposal — and it does so for reasons that have nothing to do with
the mouse. What this requirement guards is the property it was written for: that adding a
gesture never silently rebinds a key, and that the pane stays fully usable with no mouse at
all. Both remain true — every gesture above names the key that already reached it.

The pane SHALL therefore stay fully usable in a terminal that reports no mouse event at
all, including over SSH, with no feature reachable only by pointer.

#### Scenario: The key table is unchanged

- **WHEN** `action_for` is called with the full table of inputs `list-sections` asserted —
  every key and every near miss, under `filtering` false and again under `filtering` true
- **THEN** each returns exactly the action it returned before this change
- **AND** no new key is mapped: the two mapping tables gain no row

#### Scenario: Every mouse action has a key that produces the same effect

- **WHEN** for each of the four list-and-detail outcomes — advance the selection, retreat
  the selection, scroll the content down, scroll the content up — the dashboard is driven
  once by the mouse action and once by the corresponding key at the corresponding route
- **THEN** the two resulting `Dashboard` values are equal, field for field
- **AND** the same holds for a section toggle driven by `Action::Click(Target::Section(k))`
  against `Space`, and for a tab switch driven by a tab click against the matching digit key

### Requirement: `SPEC.md` names the mouse bindings and the drag-to-select cost

`SPEC.md` → Keys SHALL carry a mouse table naming each binding this capability defines —
the wheel over each region, the click on a change row, the second click, the click on a
list section header, the click on a tab cell, the click on an artifact-section header, and
the click on any other content row of a foldable artifact: **seven**, five before
`foldable-spec-sections` — and SHALL state that enabling mouse capture costs the terminal's
own drag-to-select, which requires holding `Option` on macOS or `Shift` on most Linux
terminals.

`AGENTS.md`'s terminal-seam rule SHALL name the two capture commands alongside the four
terminal-mode functions it already lists, so the confined set the `NORAW-GREP` gate enforces
and the set the document claims are the same set.

A doc-conformance test SHALL bind both claims to the files that determine them, so a binding
added, removed, or renamed in `ui::driver::mouse_action` without the document following
fails `cargo test`.

**What that test's subject actually is, stated because it is narrower than it reads:** the
leg extracts the backticked `Action::<Variant>` names from the table and compares that set
against `mouse_action`'s body. This change's two new gestures resolve to `Action::Click` and
`Action::Ignore`, both already named, so the leg is green before and after and **cannot fail
for the two new rows**. The rows are still required above; what verifies them is the
resolver's own scenarios, and the leg's own falsifiability SHALL be re-established by a
recorded negative control — delete one documented row, confirm the leg fires, restore it —
rather than inferred from its passing.

#### Scenario: The documented bindings match the resolver

- **WHEN** `tests/doc_contract.rs` reads `SPEC.md` → Keys' mouse table and compares it
  against the bindings `src/ui/driver.rs`'s resolver actually implements
- **THEN** every documented binding is implemented and every implemented binding is
  documented
- **AND** the check fails when the mouse table is absent, rather than passing vacuously

#### Scenario: The documented confined set matches the gate

- **WHEN** `tests/doc_contract.rs` reads the terminal-seam rule's list of confined function
  names from `AGENTS.md` and the `RAW_RE` pattern from `scripts/gates/noraw-grep.sh`
- **THEN** the two name the same six functions —  `enable_raw_mode`, `disable_raw_mode`,
  `EnterAlternateScreen`, `LeaveAlternateScreen`, `EnableMouseCapture`, and
  `DisableMouseCapture`
