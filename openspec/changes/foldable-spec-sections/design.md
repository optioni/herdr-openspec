## Context

The `specs` artifact is the only one in the `tdd` schema whose `generates` is a glob
(`specs/**/*.md`). `changes::resolve_artifact` already returns every match, sorted by raw
OS-string bytes, so `ArtifactRef::paths` is a `Vec<PathBuf>` of length three for a change
like `markdown-legibility`. `Dashboard::sync_detail` then flattens that list into one
`detail.source: String` joined by a newline, and `ui::detail::content_lines` renders the
result as one markdown document. OpenSpec spec files begin at `## MODIFIED Requirements`, so
the rendered tab is N indistinguishable blocks with no capability name anywhere in it — the
name exists only in the directory component, which nothing renders.

The constraint that shapes the whole design is that the detail region has **no cursor**.
`detail.scroll` is an offset, and `Dashboard::normalise_scroll` clamps it through
`layout::scroll_offset`, which returns `min(scroll, lines - height)` and therefore `0`
whenever the content fits the region. A collapsed foldable tab is a handful of header rows
and almost always fits. Any "act on the section at the scroll position" model can therefore
address only the first section in any pane taller than the section count — the ordinary case,
not an edge one. That is the design's central problem, and Decision 2 is its answer.

## Goals / Non-Goals

**Goals:**

- A multi-file artifact renders as named, individually foldable sections, collapsed by
  default, so opening the `specs` tab answers "which capabilities does this change touch?"
  in one glance.
- A single-file artifact renders exactly as it does today, byte for byte.
- One fold key across both regions, with no new binding to learn.
- A click folds a section on the same terms a click folds a list section.

**Non-Goals:**

- Changing `ui::markdown` itself. Soft-break reflow and glyph choices belong to the in-flight
  `markdown-legibility` change; this one is the join above the renderer. No file this change
  edits is a file that change edits, except `src/ui/detail.rs` (it touches `content_lines`'
  neighbours, not `content_lines`) — the two are independent and can land in either order.
- Editing, reordering, filtering, or persisting anything. The pane stays read-only and its
  only write stays `agent-names.toml`.
- Unifying the two regions on one cursor model for every artifact tab. Considered in
  Decision 2; deliberately declined.
- Folding the tracked-tasks tab. Decision 8.

## Boundaries

Everything this change touches is inside `src/ui/`, plus one role in the palette. No module
outside the view layer changes, `Change` and `ChangeSet` are untouched, and **no process
spawn is added anywhere** — this change names no `HerdrCli`, no `OpenspecCli`, and no
`process::Command`, so `NOSPAWN-GREP` and `LAUNCHSEAM` see nothing new.

| File | What changes | Pattern it follows |
|---|---|---|
| `src/ui/app.rs` | `Detail.source` → `sections: Vec<Section>`; new `Detail.expanded`; new `Section` type; `sync_detail` step 4; `apply`'s `ToggleSection` arm becomes route-dependent; `apply_click` gains two arms; `Target` gains two variants; `normalise_scroll` branches | `Sections { collapsed }` and `apply_toggle_section`, the list region's own fold, mirrored |
| `src/ui/detail.rs` | `content_lines` returns `Vec<ContentRow>` — a `markdown::Line` plus a `ContentKind` — instead of `Vec<markdown::Line>`; new `ContentRow`, `ContentKind`, `section_at` | `ui::list::rows` returning a `RowKind`, mirrored |
| `src/ui/view.rs` | the detail slice's offset is `viewport` or `scroll_offset` by foldability; the detail draw loop patches a `Role` over `style_for` by the row's `ContentKind` | the list region already derives its slice with `viewport` and already maps `RowKind` → `Role` in this file |
| `src/ui/driver.rs` | `mouse_action` resolves a click in the detail content area | its existing `Zone::ListRow` arm |
| `src/ui/layout.rs` | `Zone` gains `DetailRow { content, row }` and `Detail` narrows by the content area (`responsive-layout` delta) | `Zone::ListRow { interior, row }`, mirrored exactly |
| `src/ui/palette.rs` | two roles, `DetailSection` and `DetailSectionSelected`, joining the enum, the modifier table, and the uncoloured list (`view-palette` delta) | `ListRow` / `ListRowSelected` |
| `Makefile` | `NODEFAULT-UI` gains a **sixth** recipe line for `Section` with its own measured `SCAN_MIN` | the gate's existing five-subject shape, where every line carries its own floor |
| `SPEC.md` | the key-binding table, the mouse-binding table, the `Detail` field list | — |
| `tests/doc_contract.rs` | the documented mouse-binding list | its existing binding-list leg |

`src/ui/detail.rs`, `src/ui/view.rs`, `src/ui/layout.rs`, `src/ui/palette.rs`, and
`src/ui/app.rs` are all in the pure nine-file set, and stay pure: **this change adds no I/O
to any view.** Every new function is a total function of state and width. The one filesystem
edge involved, `ui::read_artifact`, is unchanged and still injected — `sync_detail` still
takes `&dyn Fn(&Path) -> Result<String, String>` and still names no `read_to_string`.

The change alters no seam module. `src/watch.rs`, `src/refresh.rs`, `src/agents.rs`,
`src/launch.rs`, and `src/open.rs` are untouched, the worker-thread count stays three, and
`NOBLOCK`'s subject list is unchanged.

## Contracts

`Detail`, `Section`, `Target`, `Role`, `Zone`, `ContentRow`, and `ContentKind` are crate-internal types with no external consumer. `content_lines`' return type changes, and its two production callers — `ui::view::render`'s detail-content draw and `Dashboard::normalise_scroll`, which needs only `.len()` — change with it, as do the two width-property tests that iterate its output. The
only consumer-facing surfaces are the **key bindings** and the **mouse bindings**, and both
change:

- `Space` at `Route::Detail` previously folded a *list* section. It now folds an *artifact*
  section, or is inert. **BREAKING**, per this repository's rule that any keybinding change
  is.
- `j`, `k`, the arrows, and the wheel at a *foldable* artifact previously moved an offset.
  They now move a cursor whose viewport follows it. **BREAKING** by the same rule, though the
  observable difference at a collapsed foldable tab is that the keys work at all.
- A left click in the detail content area was `Action::Ignore` at every row. It now moves the
  detail cursor, and folds a section when it lands on a header. **Additive**: no click that
  did something before does something different now.

Every other binding, and every binding at a non-foldable artifact, is unchanged. No manifest
entry, no config key, no CLI subcommand, and no dependency changes. `Change`, `ChangeSet`,
`ArtifactRef`, and `Progress` are untouched, so `changes::from_files` and `changes::from_cli`
need no reconciliation — this change reads `ArtifactRef::paths`, which both producers already
fill identically, and writes nothing back.

## Persistence and Rollout

- **Migration:** none. No on-disk format, no state file, no schema.
- **Backfill:** none.
- **Seeding:** none — `expanded` is an empty `BTreeSet` at every construction site, which is
  precisely why the set names the open sections rather than the closed ones (Decision 4).
- **Cache invalidation:** the `(change directory, tab)` cache in `sync_detail` is unchanged.
  `expanded` is cleared on exactly the condition that already resets `detail.scroll` — a key
  change — and on no other, so a forced reload from the watcher does not fold anything shut.
- **Index rebuild:** none.
- **Authorization:** none. The pane is read-only and single-user; there is no caller to
  refuse.
- **Observability:** none added. A fold records no problem row and logs nothing, because
  neither failure mode exists — every path is already in memory.
- **Deployment:** `make build` and the existing `[[build]]` step. No manifest change, so
  `herdr plugin link .` needs no re-link.

## Test Boundaries

| Dependency | In acceptance test (`run_loop` over a `TestBackend`) | In unit tests |
|---|---|---|
| Terminal | replaced — `ratatui::backend::TestBackend` at 120x20, 120x40, 60x20, 60x40; `src/ui/terminal.rs` is never reached | replaced — pure functions take a `width: u16`, no backend at all |
| Artifact reader (`ui::read_artifact`) | replaced — an injected recording `&dyn Fn(&Path) -> Result<String, String>` returning fixture text and counting calls | replaced — the same recording closure |
| Filesystem | replaced everywhere the reader stands in; **real** only in `crate::testutil::ScratchDir` trees used by the existing `ui::load` startup scenarios this change re-asserts | replaced |
| `openspec` binary | not reached — this change adds no CLI call, and every dashboard under test is constructed directly or from a `ScratchDir` in file mode | not reached |
| Herdr socket (`HerdrCli`) | not reached — no launch, no focus, no pane call | not reached |
| Filesystem watcher (`FsEvents`) | replaced — the inert stub the existing loop tests already use | not reached |
| Refresh worker (`Refresher`) | replaced — the inert stub, plus a scripted `RefreshResult::Files`/`Merged` for the two adoption scenarios | replaced |
| Agent poller (`AgentPoll`) | replaced — the inert stub | not reached |
| Launcher (`Launcher`) | replaced — the inert stub | not reached |
| Clock | not reached — no new code reads one, and `NOBLOCK` still forbids it under `src/ui/` | not reached |
| Process spawn | not reached — asserted by `NOSPAWN-GREP`'s existing tree-wide sweep | not reached |
| Event source | replaced — the existing scripted `Vec<Event>` source | not reached |

No task may introduce a boundary this table does not name. In particular, no test in this
change may call `ui::read_artifact`, construct a `RealHerdrCli`, or enter raw mode.

## Test Strategy

Four tiers, in the order the fastest is preferred:

- **unit (pure)** — a function under test called directly with a constructed `Detail`,
  `Dashboard`, or path, asserted on its return value. `cargo test`.
- **view (TestBackend)** — `ui::view::render` into a `Buffer` at the two mandated frame
  widths (120 and 60) and the two mandated interiors (78 and 58), asserted on cells and
  styles. Styles are asserted by comparing against `palette::style(role)`, never a literal
  colour, per `PALETTE`.
- **loop (`run_loop` + TestBackend)** — the whole draw-then-wait loop driven by a scripted
  event source with every collaborator replaced by its inert stub. This is this change's
  **outer-loop acceptance tier**, and it is taken: `A fold reads no file` and
  `A foldable tab is clamped to its last line, not its last screenful` are only true of the
  assembled loop, because both are about what happens between a keypress and the next frame.
- **gate (script)** — a `scripts/gates/` program run by `make gates`, with a planted defect
  recorded in `tests/gate-controls.toml` and executed by `tests/gate_controls.rs`.

Focused commands below name a `cargo test` filter; `make check` runs every gate and is the
final verification.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| artifact-folds :: The three spec files of a change become three labelled sections | assert `detail.sections` labels and texts after one `sync_detail` | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sections` |
| artifact-folds :: A single-file artifact is one section and is not foldable | assert one section, and that the rendered rows equal `ui::markdown::lines(&sections[0].text, width)` cell for cell with no row prepended — the property "unchanged from before" reduces to, since no pre-change binary is obtainable | view | reader replaced, terminal replaced | `cargo test --lib ui::view::tests::fold` |
| artifact-folds :: An artifact with no resolved paths has no sections | assert empty `sections` and a zero call count | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sections` |
| artifact-folds :: An unreadable file drops its section and keeps its siblings | assert two sections and one problem | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sections` |
| artifact-folds :: The label derivation is total over adversarial paths | table-driven assert over six paths | unit (pure) | none | `cargo test --lib ui::app::tests::label` |
| artifact-folds :: The specs tab opens as a list of capability names | assert the first three content rows at both widths | view | reader replaced, terminal replaced | `cargo test --lib ui::view::tests::fold` |
| artifact-folds :: A tab move forgets the fold, a forced reload does not | three dashboards, assert `expanded` after each path | unit (pure) | reader replaced, refresher replaced | `cargo test --lib ui::app::tests::fold_reset` |
| artifact-folds :: An index past the end folds shut rather than panicking | render with `expanded` holding `7` over two sections | view | terminal replaced | `cargo test --lib ui::view::tests::fold` |
| artifact-folds :: Folding one section shows its body and leaves its siblings shut | assert the row sequence at both widths | view | terminal replaced | `cargo test --lib ui::view::tests::fold` |
| artifact-folds :: The cursor's section header is the emphasised one | assert cell styles against `palette::style(Role::…)` for three cursor positions | view | terminal replaced | `cargo test --lib ui::view::tests::fold` |
| artifact-folds :: A narrow pane truncates the label and keeps the glyph | assert header text at 18 and 13 columns; sweep 0..=20 for the width property | unit (pure) | none | `cargo test --lib ui::detail::tests::header` |
| artifact-folds :: Each drawn header row resolves to its own index | `section_at` over the drawn frame's own row list and offset, four rows, twice | unit (pure) | none | `cargo test --lib ui::detail::tests::section_at` |
| artifact-folds :: Resolution is total and inert where it should be | `section_at` over adversarial `Rect`s and rows | unit (pure) | none | `cargo test --lib ui::detail::tests::section_at` |
| artifact-folds :: `Space` opens the section under the cursor and leaves its siblings shut | apply `ToggleSection`, assert state, then render | unit (pure) + view | terminal replaced | `cargo test --lib ui::app::tests::toggle_detail` |
| artifact-folds :: `Space` inside an open section folds it and moves the cursor to its header | apply, assert `expanded` and `scroll`, then render | unit (pure) + view | terminal replaced | `cargo test --lib ui::app::tests::toggle_detail` |
| artifact-folds :: `Space` on a problem row is inert | ten applies, assert field-for-field equality | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::toggle_detail` |
| artifact-folds :: `Space` is inert on a non-foldable artifact | ten applies on each of two dashboards | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::toggle_detail` |
| artifact-folds :: A fold reads no file | `run_loop` with a recording reader and a six-event script; assert the call count | loop | every collaborator replaced | `cargo test --lib ui::driver::tests::fold` |
| artifact-content :: The selected tab's file is read once and reused | existing test, re-asserted on `sections` | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: A forced reload re-reads the same key and keeps the scroll | existing test, extended with `expanded` | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: A tab move under a forced reload still resets the scroll | existing test, extended with `expanded` | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: Switching the tab re-reads, and so does switching the change | existing test, re-asserted on `sections` | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: Two changes with the same name are distinguished by directory | existing test, re-asserted on `sections` | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: A multi-file artifact is concatenated in path order with a separating newline | rewritten: assert two sections with verbatim text, then render both header rows | unit (pure) + view | reader replaced, terminal replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: An unreadable file names its reason and does not lose its siblings | existing test, extended with the not-foldable assertion | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: An artifact with no resolved paths reads nothing at all | existing test, extended with `expanded` | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: A tab out of range for the newly selected change is clamped before the read | existing test, unchanged | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: An empty visible list clears the detail | existing test, extended with `expanded` | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::sync_detail` |
| artifact-content :: The loop syncs before it draws | existing loop test, unchanged | loop | every collaborator replaced | `cargo test --lib ui::driver::tests::sync` |
| artifact-content :: A missing artifact still shows its tab and reads `No content yet` | existing view test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::view::tests::no_content` |
| artifact-content :: `No content yet` does not eat the border at a narrow frame | existing view test, unchanged | view | terminal replaced | `cargo test --lib ui::view::tests::no_content` |
| artifact-content :: A read failure is named above the content at both widths | existing view test, extended with the no-header assertion | view | terminal replaced | `cargo test --lib ui::view::tests::problems` |
| artifact-content :: The rendered markdown fills the content area, not the whole interior | existing view test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::view::tests::content_area` |
| artifact-content :: The tracked-tasks tab renders the checklist body instead | existing view test, unchanged | view | terminal replaced | `cargo test --lib ui::view::tests::tasks_tab` |
| artifact-content :: A wide-character document stays inside the detail region | existing view test, extended with a CJK-labelled foldable case | view | terminal replaced | `cargo test --lib ui::view::tests::wide` |
| artifact-content :: `content_lines` is total and width-parameterised | existing table, extended to nine `Detail` values | unit (pure) | none | `cargo test --lib ui::detail::tests::content_lines_total` |
| artifact-content :: No `content_lines` line exceeds its width at any width | existing 0..=130 sweep, extended to the foldable values | unit (pure) | none | `cargo test --lib ui::detail::tests::content_lines_width` |
| artifact-content :: A foldable tab's body is headers, and an open section's markdown beneath its own | assert the returned line list against `markdown::lines` at 78 and 58 | unit (pure) | none | `cargo test --lib ui::detail::tests::content_lines_fold` |
| artifact-content :: A non-foldable tab is byte-identical to today | assert the returned rows' `line` values equal `markdown::lines` exactly and every `kind` is `Body` | unit (pure) + view | terminal replaced | `cargo test --lib ui::detail::tests::content_lines_fold` |
| artifact-content :: The tracked-tasks tab concatenates rather than folding | assert equality with `tasks::lines` over the concatenation | unit (pure) | none | `cargo test --lib ui::detail::tests::content_lines_fold` |
| detail-scroll :: `Detail` has no `Default` and no site elides a field | `NODEFAULT-UI` with `TYPES` extended by `Section`, plus the compile-time destructuring companions | gate + unit (pure) | none | `make gates` / `cargo test --lib names_agent_names_at_every_site` |
| detail-scroll :: Startup leaves the detail empty and unscrolled | existing test, extended with `expanded` | unit (pure) | filesystem real (`ScratchDir`) | `cargo test --lib ui::tests::load` |
| detail-scroll :: At the detail route the content scrolls by one line at both widths | existing test, re-asserted on `sections` | unit (pure) + view | terminal replaced | `cargo test --lib ui::app::tests::scroll` |
| detail-scroll :: At a collapsed foldable tab the same keys walk the section list | two `Next`s, assert `scroll` and the emphasised header at 120x40 and 60x40 | view | terminal replaced | `cargo test --lib ui::view::tests::fold_cursor` |
| detail-scroll :: At the list route the same actions still move the selection | existing test, unchanged | view | terminal replaced | `cargo test --lib ui::app::tests::scroll` |
| detail-scroll :: Scrolling stops at the top | existing test, extended with the foldable case | unit (pure) + view | terminal replaced | `cargo test --lib ui::app::tests::scroll` |
| detail-scroll :: While filtering, `j` and `k` still type into the query | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::app::tests::filter` |
| detail-scroll :: Every route move resets the scroll | existing test, extended with a fourth dashboard asserting `expanded` survives | unit (pure) + view | terminal replaced | `cargo test --lib ui::app::tests::route_reset` |
| detail-scroll :: `Enter` at the detail route moves nothing and keeps the scroll | existing test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::app::tests::route_reset` |
| detail-scroll :: `Enter` from the list route still opens at the top | existing test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::app::tests::route_reset` |
| detail-scroll :: `Enter` while filtering still dismisses the filter and resets nothing | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::app::tests::route_reset` |
| detail-scroll :: Scrolling past the end is normalised on the next frame | existing loop test, re-asserted on `sections` | loop | every collaborator replaced | `cargo test --lib ui::driver::tests::normalise` |
| detail-scroll :: A foldable tab is clamped to its last line, not its last screenful | `run_loop` at 120x40 and 60x40 with the same script; assert `scroll == 2` and the emphasis | loop | every collaborator replaced | `cargo test --lib ui::driver::tests::normalise` |
| detail-scroll :: A resize renormalises the offset on the next frame | existing test, extended with the foldable case | unit (pure) | none | `cargo test --lib ui::app::tests::normalise` |
| detail-scroll :: The narrow list route leaves the stored offset alone | existing test, re-asserted on `sections` | unit (pure) | none | `cargo test --lib ui::app::tests::normalise` |
| detail-scroll :: The wheel scrolls the detail region at the list route | existing test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::app::tests::wheel` |
| detail-scroll :: The wheel moves the cursor at a foldable tab | two `ScrollDown`s then two `ScrollUp`s, assert `scroll` and the emphasis | view | terminal replaced | `cargo test --lib ui::app::tests::wheel` |
| detail-scroll :: `ScrollUp` stops at the top | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::app::tests::wheel` |
| detail-scroll :: A held wheel is clamped by the frame, not by `apply` | existing test, unchanged | unit (pure) + view | terminal replaced | `cargo test --lib ui::app::tests::wheel` |
| detail-scroll :: `Next` at the detail route and `ScrollDown` are the same move | existing equality test, extended with the foldable case | unit (pure) | none | `cargo test --lib ui::app::tests::wheel` |
| list-selection :: `Space` on a header folds and unfolds that section | existing test, pinned to `Route::List` | view | terminal replaced | `cargo test --lib ui::app::tests::toggle_section` |
| list-selection :: `Space` inside a section folds it and moves the cursor to its header | existing test, pinned to `Route::List` | view | terminal replaced | `cargo test --lib ui::app::tests::toggle_section` |
| list-selection :: `Space` at the detail route leaves the list alone | apply at `Route::Detail`; assert list fields unchanged and the 120x20 list rows byte-identical | view | terminal replaced | `cargo test --lib ui::app::tests::toggle_route` |
| list-selection :: `Space` at the detail route is inert on a non-foldable artifact | ten applies, assert field-for-field equality | unit (pure) | reader replaced | `cargo test --lib ui::app::tests::toggle_route` |
| list-selection :: An empty list makes `Space` inert | existing test, pinned to `Route::List` | unit (pure) | none | `cargo test --lib ui::app::tests::toggle_section` |
| list-selection :: Opening an unresolved archive requests a refresh | existing test, pinned to `Route::List` | unit (pure) | none | `cargo test --lib ui::app::tests::toggle_section` |
| list-selection :: A refresh does not undo a fold | existing test, unchanged | view | refresher replaced, terminal replaced | `cargo test --lib ui::app::tests::adopt` |
| list-selection :: A refresh keeps the cursor on the same change | existing test, unchanged | unit (pure) | refresher replaced | `cargo test --lib ui::app::tests::adopt` |
| mouse-input :: A click selects a change row and a second click opens it | existing test, unchanged | view | terminal replaced | `cargo test --lib ui::driver::tests::mouse` |
| mouse-input :: A click on a section header folds it exactly as `Space` does | existing test, with the `Space` comparison pinned to `Route::List` | view | terminal replaced | `cargo test --lib ui::driver::tests::mouse` |
| mouse-input :: A click on an archived header opens an unresolved archive and requests its refresh | existing test, unchanged | view | terminal replaced | `cargo test --lib ui::driver::tests::mouse` |
| mouse-input :: A click on an artifact-section header folds it exactly as `Space` does | draw at 120x40, press row 1, assert the action and the field-for-field equality; repeat at `Route::List` | view | terminal replaced | `cargo test --lib ui::driver::tests::mouse_fold` |
| mouse-input :: A click in an open section's body moves the detail cursor and folds nothing | press a body row, assert `DetailLine`, then a following `ToggleSection` | view | terminal replaced | `cargo test --lib ui::driver::tests::mouse_fold` |
| mouse-input :: A click on a non-foldable tab's content is inert | three presses, assert `Ignore` and an unchanged dashboard | view | terminal replaced | `cargo test --lib ui::driver::tests::mouse_fold` |
| mouse-input :: A click on a tab cell switches to that artifact | existing test, unchanged | view | terminal replaced | `cargo test --lib ui::driver::tests::mouse` |
| mouse-input :: Clicks that address nothing are inert | existing test, extended with the below-last-line press | view | terminal replaced | `cargo test --lib ui::driver::tests::mouse` |
| mouse-input :: The other buttons and the non-press kinds are inert | existing test, extended to a header row | unit (pure) | none | `cargo test --lib ui::driver::tests::mouse_total` |
| view-palette :: The palette answers every role with a `Style` | existing exhaustive-`match` totality test, extended with the two new roles and the `DetailSectionSelected` inequality assertion | unit (pure) | none | `cargo test --lib ui::palette::tests` |
| view-palette :: The confinement gate catches a `Color` named outside the palette | existing planted controls, re-run | gate (script) | none | `make gates` / `cargo test --test gate_controls` |
| view-palette :: The palette module reaches no I/O and measures no width | existing `NOIO-VIEW` and `COLWIDTH` legs, re-run | gate (script) | none | `make gates` |
| view-palette :: Each role's modifier set is exactly the table above | existing table comparison, extended with two rows and the `BOLD \| REVERSED` discrimination | unit (pure) | none | `cargo test --lib ui::palette::tests` |
| view-palette :: A monochrome reading of the frame is unchanged | existing modifier-only buffer comparison, extended with the three-file case asserting exactly one `REVERSED` row | view | terminal replaced | `cargo test --lib ui::view::tests::monochrome` |
| view-palette :: The coloured set is exactly the table above | existing `fg`/`bg` sweep, extended with the two new roles reporting `None`/`None` | unit (pure) | none | `cargo test --lib ui::palette::tests` |
| view-palette :: An out-of-range heading level does not panic | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::palette::tests` |
| view-palette :: Faces reach the buffer as coloured styles at both mandated widths | existing test, re-asserted on a single section, plus the no-`REVERSED` assertion | view | terminal replaced | `cargo test --lib ui::view::tests::faces` |
| view-palette :: A section header's role is selected by its kind, not by its face | assert the selected row's cells against `palette::style`, the unselected rows against their `kind`, and that `Role::` appears nowhere in `src/ui/detail.rs` | view + gate (grep) | terminal replaced | `cargo test --lib ui::view::tests::fold_role` |
| view-palette :: Heading foreground wins over a code span inside it | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::view::tests::style_for` |
| view-palette :: A plain face is the default style | existing test, unchanged | unit (pure) + view | terminal replaced | `cargo test --lib ui::view::tests::style_for` |
| responsive-layout :: The zones tile the frame at 120 columns | existing tiling table, extended with the first and last content rows and the `DetailRow.content` independent derivation | unit (pure) | none | `cargo test --lib ui::layout::tests::zone` |
| responsive-layout :: Below the breakpoint only the routed region has zones | existing test, extended with the narrow `DetailRow` leg | unit (pure) | none | `cargo test --lib ui::layout::tests::zone` |
| responsive-layout :: The breakpoint is exact for the hit test too | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::layout::tests::zone` |
| responsive-layout :: Degenerate frames resolve without panicking | existing sweep, extended with the zero-width/zero-height `DetailRow` assertion | unit (pure) | none | `cargo test --lib ui::layout::tests::zone` |
| responsive-layout :: The hit test agrees with what was drawn | existing whole-buffer classification, extended to check `DetailRow` cells against `content_lines`' output at the drawn offset | view | terminal replaced | `cargo test --lib ui::layout::tests::zone_drawn` |
| artifact-content :: The artifact read has exactly one binding under `src/ui/` | existing `READSEAM` gate legs, re-run | gate (script) | none | `make gates` |
| artifact-content :: `read_artifact` agrees with the standard library and names its failure | existing test, unchanged | unit (pure) | filesystem real (`ScratchDir`) | `cargo test --lib ui::tests::read_artifact` |
| artifact-content :: A change whose schema will not parse names the reason in the detail region | existing test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::view::tests::problems` |
| artifact-content :: A change problem and a tab problem are both shown, change first | existing test, extended to assert both rows carry `ContentKind::Problem` | view | terminal replaced | `cargo test --lib ui::view::tests::problems` |
| artifact-content :: No selected change contributes no lines | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::detail::tests::content_lines` |
| artifact-content :: The line count drives the scroll clamp | existing test, re-asserted on the row count | unit (pure) | none | `cargo test --lib ui::app::tests::normalise` |
| detail-scroll :: The document fills the detail interior at both mandated widths | existing test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::view::tests::content_area` |
| detail-scroll :: Faces reach the buffer as styles at both widths | existing test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::view::tests::faces` |
| detail-scroll :: A table reaches the buffer aligned and inside the region | existing test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::view::tests::table` |
| detail-scroll :: An empty source leaves the detail interior blank at both widths | existing test, re-asserted on an empty section list | view | terminal replaced | `cargo test --lib ui::view::tests::no_content` |
| detail-scroll :: Content never overwrites the detail region's border | existing test, extended with a foldable artifact whose labels are CJK | view | terminal replaced | `cargo test --lib ui::view::tests::border` |
| detail-scroll :: A degenerate detail interior draws nothing and does not panic | existing test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::view::tests::degenerate` |
| detail-scroll :: `scroll_offset` is exact at its boundaries | existing test, unchanged — the helper itself does not change | unit (pure) | none | `cargo test --lib ui::layout::tests::scroll_offset` |
| detail-scroll :: `interior` agrees with a bordered block's own inner rectangle | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::layout::tests::interior` |
| detail-scroll :: A scroll offset past the end still draws the last screenful | existing test, re-asserted on `sections`; the foldable counterpart is `A foldable tab is clamped to its last line` above | view | terminal replaced | `cargo test --lib ui::view::tests::scroll` |
| responsive-layout :: The routed region's border is bold and the other's is not | existing test, unchanged | view | terminal replaced | `cargo test --lib ui::view::tests::border` |
| responsive-layout :: Interiors are blank at both widths | existing test, re-asserted on `sections` | view | terminal replaced | `cargo test --lib ui::view::tests::blank` |
| responsive-layout :: Rows do not overwrite the borders at either width | existing test, unchanged | view | terminal replaced | `cargo test --lib ui::view::tests::border` |
| list-selection :: The first target is selected on startup at both widths | existing test, extended to assert the four-variant `Target` and that `targets()` still yields only the two list variants | view | terminal replaced | `cargo test --lib ui::app::tests::targets` |
| list-selection :: `j`, `k`, and the arrows move the cursor over headers and changes | existing test, pinned to `Route::List` | view | terminal replaced | `cargo test --lib ui::app::tests::targets` |
| list-selection :: The cursor clamps at both ends rather than wrapping | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::app::tests::targets` |
| list-selection :: The cursor crosses the archived header into the archived rows | existing test, unchanged | view | terminal replaced | `cargo test --lib ui::app::tests::targets` |
| list-selection :: A collapsed section's changes are neither visible nor addressable | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::app::tests::targets` |
| list-selection :: Navigation over an empty visible list is inert | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::app::tests::targets` |
| list-selection :: `Enter` on a section header does nothing | existing test, unchanged | unit (pure) | none | `cargo test --lib ui::app::tests::targets` |
| mouse-input :: The key table is unchanged | existing test, **narrowed**: `action_for`'s mapping is unchanged by the mouse, but `Space`'s *effect* at `Route::Detail` changes in `apply`, which this scenario does not cover and `ui::app::tests::toggle_route` does | unit (pure) | none | `cargo test --lib ui::app::tests::action_for` |
| mouse-input :: Every mouse action has a key that produces the same effect | existing test, extended with the two new gestures and the keys that already reach them | unit (pure) | none | `cargo test --lib ui::driver::tests::mouse_key_parity` |
| mouse-input :: The documented bindings match the resolver | existing `mouse_bindings_match_spec_md` leg, plus the negative control this change adds — delete a documented row, confirm it fires, restore it | contract (`cargo test`) | none | `cargo test --test doc_contract` |
| mouse-input :: The documented confined set matches the gate | existing leg, unchanged | contract (`cargo test`) | none | `cargo test --test doc_contract` |

## Decisions

### Decision 1 — `detail.source: String` becomes `detail.sections: Vec<Section>`

**Chosen:** replace the flat string with an ordered list of `{ label, text }`, one per
successfully read path.

**Alternatives:** (a) keep `source: String` and add `Vec<(String, Range<usize>)>` naming each
file's byte range within it. Rejected: two fields that must agree, with no type-level reason
they do — a re-read that fills one and not the other is a class of bug the chosen shape
cannot express. (b) Keep `source` and re-derive the boundaries by re-reading the paths in
`content_lines`. Rejected outright: `content_lines` is a pure view function and may not touch
the filesystem.

**Consequence:** `Detail` goes from five fields to six (with `expanded`), and `Section` joins
the `NODEFAULT-UI` type list so every construction site names both its fields.

### Decision 2 — the detail cursor: reinterpret `detail.scroll`, and derive the offset with `layout::viewport`

This is the change's central decision, and the one the proposal flagged as open.

**The problem.** `normalise_scroll` clamps through `layout::scroll_offset(lines, scroll,
height) = min(scroll, lines - height)`, which is `0` for every `scroll` when `lines <=
height`. Three collapsed sections in a forty-row pane are three lines, so `scroll` is pinned
at `0` and `Space` could only ever address the first section. This is not an edge case; it is
what the feature looks like most of the time.

**Chosen:** `detail.scroll` keeps its name and its meaning — "the reader's position in the
rendered line list", which `detail-scroll` already describes as "the detail region's
counterpart to `list-selection`'s `selected`". What changes is what the region does with it:

- foldable artifact → the drawn offset is `layout::viewport(lines, scroll, height)`, and
  `normalise_scroll` clamps `scroll` to `lines - 1`;
- otherwise → `layout::scroll_offset`, and the clamp is `lines - height`, exactly as today.

`layout::viewport` is not new. It is the helper `list-selection` already derives the list
region's visible slice from, with the identical signature `(rows, cursor, height)`. So the
detail region is not gaining a cursor mechanism; it is gaining the *list region's* cursor
mechanism, and the two regions end up with one model each drawn from the same two helpers
that already sit side by side in `src/ui/layout.rs`.

**Alternatives:**

- **(a) A separate `Detail.cursor: usize` beside `scroll`.** Rejected: two positions, one
  meaningful at a time, with no rule saying which — and `Detail`'s existing doc comment
  already calls `scroll` the counterpart of `selected`, so the second field would be a second
  name for the thing that field already is.
- **(b) Give *every* artifact tab a cursor, unifying the two regions completely.** This is
  the tidiest end state and was seriously considered. Rejected for scope: it would rewrite
  `detail-scroll`'s "A scroll past the end draws the last screenful", "Scrolling stops at the
  top", and the normalisation scenarios for content that has nothing to fold, changing how
  `j` feels in `proposal.md` and `design.md` — unrequested behaviour change in the tabs the
  reader spends most time in. Left as a future unification if the two models prove confusing;
  Decision 3 is what makes that future change a one-line predicate edit.
- **(c) No cursor: fold whatever section's header is topmost in the viewport.** Rejected —
  this is exactly the defect above. It would ship a feature that works only in a pane shorter
  than the section count.
- **(d) A section index plus a new key (`Tab`, `n`/`p`, or `z`).** Rejected: it adds a
  binding to learn, it cannot scroll within an open section, and it leaves `j`/`k` doing
  nothing useful at a collapsed tab.

**Consequence:** at a collapsed foldable tab `j` and `k` walk the section list one header at
a time, which is the interaction the feature is for; inside an open section they walk its
rendered lines, so a four-hundred-line spec still reads normally. `viewport` centres the
cursor, so the first few `j` presses move the cursor down a tall pane before the content
starts moving — the same feel the list region already has.

### Decision 3 — foldable is derived, never stored

`sections.len() > 1`, computed at every use. **Alternative:** a `foldable: bool` on `Detail`,
set by `sync_detail`. Rejected: an artifact whose second file is deleted between two frames
would need the flag updated in step with `sections`, and there is no reason to invite that
skew when the predicate is a comparison. It also makes Decision 2's alternative (b) a
one-line change later, since every branch already asks one question in one place.

### Decision 4 — `expanded` names the open sections, inverting the list's `collapsed`

**Chosen:** `expanded: BTreeSet<usize>`, empty meaning all collapsed.

**Alternative:** `collapsed: BTreeSet<usize>` for symmetry with `Sections { collapsed }`.
Rejected: the two regions have opposite defaults — the list's sections default open, the
detail's default closed — so a `collapsed` set would have to be *seeded* with `0..n` at every
construction site and re-seeded on every key change and every adopted refresh, and a section
count that grew would silently default the new section open. The rule the two share is the
one worth sharing: **the set names the exception to the default**, so a freshly constructed
value always carries an empty set and no construction path can get the seeding wrong.

### Decision 5 — the label is the capability directory, not the file name or the relative path

`specs/degraded-coverage/spec.md` → `degraded-coverage`. The rule: when the file name is
`spec.md` and the relative path has a parent component, take that parent's final component;
otherwise take the file name.

**Alternatives:** (a) the file name — every row would read `spec.md`, which is the current
problem restated. (b) The full relative path (`specs/degraded-coverage`) — honest for any
glob, but the `specs/` prefix repeats on every row and costs seven columns of a
fifty-eight-column region. Rejected on that measurement; the tab bar already says `specs`.

Labels are **not** required to be unique, and sections are addressed by index everywhere, so
a glob matching `a/x.md` and `b/x.md` yields two `x.md` rows that fold independently rather
than a collision to resolve.

### Decision 6 — `Space` becomes route-dependent rather than gaining a sibling key

**Chosen:** `Action::ToggleSection` matches on `self.route`, exactly as `Action::Next` and
`Action::Prev` already do at `src/ui/app.rs:349-356`.

**Alternative:** a second key (`z`, or `Enter` on a header) so `Space` keeps folding list
sections at both routes. Rejected: it is two fold keys for one concept, and the behaviour it
preserves is itself wrong — `Space` at `Route::Detail` currently moves rows in the list
region, which in the narrow layout is not even drawn. Marked **BREAKING** because this
repository marks every keybinding change so, not because a reader is likely to miss it.

`Enter` was specifically **not** reused: `detail-scroll` states that `Enter` at the detail
route is a deliberate no-op and that rebinding it would be breaking, and that argument has
not changed.

### Decision 7 — click targets carry indices already resolved against the drawn frame

`Target::DetailLine(usize)` and `Target::DetailHeader { line, section }`.

`Dashboard::apply` has no width, so it cannot call `content_lines` and cannot itself tell a
header row from a body row or a line index from a section index. `mouse_action` has both the
dashboard and the frame area, so it resolves and the action carries the answer. **Alternative:**
give `apply` the width. Rejected: `apply` is a pure state transition over `Action`, and
threading geometry into it would make every other action's arm carry an argument it does not
use — `mouse_action` was introduced in `mouse-input` precisely so geometry stays out of
`action_for` and `apply`.

The header variant carries `line` beside `section` for the same reason: deriving one from the
other needs `content_lines`, and only `mouse_action` can call it.

**The membership guard is scoped to the list targets, and the detail targets are exempt.**
`apply_click` opens with `let Some(index) = self.targets().iter().position(|t| *t == target)
else { return; };` (`src/ui/app.rs:486-489`), and `targets()` yields only `Target::Section`
and `Target::Change`. Left as it is, that guard returns before either new arm is ever
reached, and the click would be silently inert — found in planning review, and it is a real
defect rather than a stylistic note. The guard exists for a specific reason that does not
apply here: a list target is an *index into a list that may have changed* between the draw
and the event — a change filtered away, a section folded — so it must be re-validated.
A `DetailLine` or `DetailHeader` carries indices `mouse_action` resolved against the frame
just drawn, and the per-frame `normalise_scroll` clamps whatever the list has since become,
so re-validating against a list `apply` cannot recompute (it has no width) would be both
impossible and pointless. `apply_click` therefore matches on the target first and applies the
`targets()` guard only in the `Section` and `Change` arms.

### Decision 8 — the tracked-tasks tab is never foldable

The tasks artifact's `generates` is a literal path in every schema this repository ships, so
a multi-section tasks tab is unreachable today. Were a schema to declare a glob there,
`tasks-checklist`'s progress bar counts the change's whole `progress`, which would disagree
with any per-section fold. Concatenating in path order preserves exactly today's behaviour
for that one tab and costs one condition in `content_lines`.

### Decision 9 — the header glyphs are `>` and `v`, reused from the list region

`list-selection` already renders `  > archived (2)` collapsed and `  v archived (2)` open.
Using the same two glyphs means a fold reads the same in both regions. **Alternative:** `▸`
and `▾`. Rejected here: they are East Asian **Ambiguous**, the class `SPEC.md` names as
painted at two columns by a CJK-locale terminal where `unicode-width` says one, and this
change has no reason to widen that exposure. The in-flight `markdown-legibility` change is
where that trade-off is being argued; if it lands and moves the list's glyphs, these follow
it there rather than diverging here.

### Decision 10 — cursor feedback is the section header's emphasis, not a highlighted line

**Chosen:** the header row of the section the cursor is on or in carries
`Role::DetailSectionSelected`; no other row is restyled.

**Alternative:** highlight the cursor's own line, as the list region marks its selected row.
Rejected: at an open section that paints a reversed bar through the middle of rendered prose,
and the cursor's *only* semantic use is which section `Space` acts on — which is exactly what
the header emphasis says. The wording is deliberately `list-selection`'s own: "the section the
cursor is on or in".

### Decision 11 — the two roles are modifier-only

`DetailSection` is `BOLD`; `DetailSectionSelected` is `BOLD | REVERSED`. `view-palette`
requires that colour be added only where it carries a distinction a modifier cannot, and a
modifier carries this one. `REVERSED` is not currently used by any role, so the two are
distinguishable from every existing role as well as from each other.

### Decision 12 — a header row carries a *kind*, and `ui::view` alone turns it into a `Role`

Found in planning review, and it is a hole rather than a refinement: under the original plan
the header row's style **could not reach the buffer at all**. `content_lines` returned
`Vec<markdown::Line>`, and the detail content area is styled by exactly one path —
`ui::view::style_for(&segment.face)`, the crate's only `Face`-to-`Style` mapping.
`markdown::Face` is seven markdown-construct flags (`heading`, `strong`, `emphasis`, `code`,
`link`, `quoted`, `strikethrough`); a section header is not a markdown construct, and
`REVERSED` appears nowhere in `src/`. There was no mechanism for a row to say "I am a header".

**Chosen:** `content_lines` returns `Vec<ContentRow>`, where a `ContentRow` is a
`markdown::Line` plus a `ContentKind` of `Problem`, `Body`, or
`SectionHeader { section, selected }`. The kind is plain data naming no `ratatui` type and no
`palette::Role`; `ui::view` maps it to `DetailSection` or `DetailSectionSelected` and patches
that over the segment's own `style_for`.

This is not a new pattern. `ui::list::rows` already returns a `RowKind` whose own doc comment
says "Plain data: no ratatui type, so `ui::view` alone decides what a badge looks like", and
`ui::view` already holds the `RowKind` → `Role` match beside the one this change adds. The
detail region simply gains the arrangement the list region has had since `color-palette`.

**Alternatives:**

- **(a) Extend `markdown::Face` with a `section_header` flag and `style_for` with an eighth
  fold step.** Rejected on two counts. It edits `src/ui/markdown.rs`, the one file the
  in-flight `markdown-legibility` change owns, turning two independent changes into a
  conflict — and this design's Non-Goals promise the opposite. It also puts a
  non-markdown concept inside the type whose whole definition is "a markdown segment's
  styling", where the next reader of `Face` would rightly not expect it.
- **(b) `content_lines` returns `Vec<(Option<Role>, Line)>`.** Rejected narrowly: it works and
  it is smaller, but it puts `palette::Role` inside `ui::detail`, which is the boundary
  `ui::list` deliberately does not cross. Two regions would then answer the "who decides what
  a row looks like" question differently, and the one that already has an argued answer would
  be the one overruled.
- **(c) Draw header rows in a second pass in `ui::view`, re-deriving which rows they are.**
  Rejected: a second derivation of the row list is exactly what `ListRow`-carries-its-own-`Rect`
  and `section_at`-takes-the-row-list exist to prevent.

**Consequence, and it is the reason this decision is recorded rather than folded into
Decision 10:** `section_at` becomes a **lookup** — it indexes the caller's own row list at
`offset + row` and reads the kind — rather than a parallel computation that has to agree with
the draw. And the unselected header's assertion changes shape: `Role::DetailSection` is plain
`BOLD`, which `Role::Strong` and five other roles also are, so a cell comparison alone would
pass for any bold body span and could not fail if the header lost its role. The falsifiable
assertion for the unselected case is on the row's `kind`; the selected case keeps a cell
comparison, because `BOLD | REVERSED` equals no other role.

## Risks / Trade-offs

- **Two scroll models in one region, chosen by an artifact's file count** → The predicate is
  one derived comparison (Decision 3) named in one place, and the non-foldable branch is
  today's code path untouched, asserted byte-for-byte by
  `A non-foldable tab is byte-identical to today`. Decision 2's alternative (b) remains
  available as a later unification.
- **`Space` changes meaning at the detail route without warning** → Marked **BREAKING** in
  the proposal, recorded in `SPEC.md`'s key-binding table, and the old behaviour was itself a
  defect (it moved rows in a region the narrow layout does not draw). No migration is
  possible for a keypress; the mitigation is that the new meaning is the one the key already
  has in the other region.
- **`Detail` gaining a field touches every construction site** → That is `NODEFAULT-UI`
  working as designed: the compiler names each site. Counted, not estimated, during
  implementation.
- **`content_lines` is called twice per frame — once by `render`, once by `normalise_scroll`
  — and now does more work per call** → It was already called twice, and the added work is
  one `BTreeSet::contains` and one string format per section, over a list whose length is a
  change's spec-file count (three, in the largest change in this repository). No cache is
  added; `seam-resilience`'s `(change directory, tab)` cache still bounds the filesystem read,
  which is the cost that mattered.
- **A CJK label could overflow a narrow header row** → Every header row goes through
  `ui::list::pad_or_truncate_right`, which measures in display columns, and the width property
  is asserted over `0..=130` rather than at the two mandated widths, which is the sweep that
  can actually see it.
- **`markdown-legibility` and this change both touch `src/ui/detail.rs`** → Different
  functions (`content_lines`' emission versus `ui::markdown`'s output), and neither depends on
  the other's spec. Whichever lands second resolves a textual conflict in one file, with no
  behavioural interaction. Decision 12 is what keeps this true: had the header carried a
  `Face` flag, both changes would edit `src/ui/markdown.rs` and the claim would be false.
- **`content_lines`' return type changes, and every caller and iterating test changes with
  it** → Two production callers (`ui::view::render`, `Dashboard::normalise_scroll`) and the
  two width-property tests, all named in Contracts and tasked in group 5. The compiler finds
  each; none is a silent-drift site.

## Migration Plan

None needed. No persisted state, no on-disk format, no protocol, and no manifest entry
changes; `expanded` is per-session and starts empty at every construction site. Deploy is
`make build` and the existing `[[build]]` step, and rollback is reverting the commit — a
reader who has a section open when the binary is replaced simply gets a freshly collapsed tab
on the next launch.

## Visual Design

Not applicable. This change modifies a terminal view, and no HTML or email design source
exists for it. The rendered grammar is specified in `specs/artifact-folds/spec.md` under
"A section header row names the file and shows its fold state" and asserted against a
`TestBackend` buffer, which is this repository's equivalent of a design source.

## Open Questions

None. The cursor question the proposal raised is settled by Decision 2; the glyph choice
defers to `markdown-legibility` by Decision 9 without blocking on it.
