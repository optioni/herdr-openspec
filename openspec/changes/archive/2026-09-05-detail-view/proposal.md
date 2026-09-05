## Why

`herdr-openspec ui` paints a real change list and can render a markdown document with
scrolling, but nothing decides *which* document. `Dashboard::detail.source` is set by no
code path in the crate, so the production pane's detail region is still an empty bordered
frame. `PRD.md` → Goal 1 ("read a change without leaving Herdr") and the first Reading
story stay unmet until picking a change and a tab shows that artifact's text.

This is the Phase 4 `detail-view` row of `openspec/IMPLEMENTATION-ORDER.md`. Both its
dependencies have landed: `list-view` (archived 2026-09-05) supplies the selection, and
`markdown-viewer` (archived 2026-09-05) supplies the renderer and the scroll.

## What Changes

- **A change header in the detail region.** The selected change's name, its schema, and
  its progress cell, in the detail interior's first row, degrading by dropping whole
  cells the way `change-rows` already does.
- **An artifact tab bar.** Built from `Change::artifacts` — the schema's declared order,
  ids **not** de-duplicated — addressed by **position**, never by id. `1`–`9` select the
  first nine positions; `[` and `]` step one tab, clamped at both ends. The bar is a
  window of whole cells that always contains the selected tab.
- **Content resolved for the selected tab.** The artifact's resolved paths are read and
  concatenated in path order, then rendered by `markdown-render` and scrolled by
  `detail-scroll`. An artifact with no resolved paths renders `No content yet`; a read
  that fails names its reason rather than showing nothing.
- **The read is an injected collaborator, not a view.** `ui::driver::run_loop` gains a
  `&dyn Fn(&Path) -> Result<String, String>` reader and calls `Dashboard::sync_detail`
  once per iteration, before the draw. The single real binding, `ui::read_artifact`,
  lives in `src/ui/mod.rs` beside `ui::load` — the same shape by which `resolve` takes
  `npm prefix -g` and `config` takes the process environment. **No view file gains an
  I/O API**, and `NOIO-VIEW`'s pure set grows to seven files rather than shrinking.
- **The detail interior is split into three rows-groups.** `layout::split_detail` gives
  a header row, a tab-bar row, and the content area below; the scroll clamp and
  `normalise_scroll` are computed against the **content** area's height, not the whole
  interior. The interior's two mandated widths, 58 and 78, are unchanged.
- **No new dependency, no manifest change, no configuration change.** No `openspec`
  binary is consulted: this change depends on `changes-from-files` only.
- **Not BREAKING, and here is why the keybinding rule does not fire.** `1`–`9`, `[`, and `]`
  all mapped to `Action::Ignore` before this change, at every route and in both filter
  modes; nothing that worked before behaves differently. No existing binding is moved,
  removed, or re-pointed. The plugin manifest and the `config.toml` format are untouched.
- **No PRD non-goal is crossed.** Nothing is written to any OpenSpec file, nothing is
  orchestrated across changes, nothing is authored, no Windows code is added.

## Non-Goals

- **`tasks-tab`'s scope, entirely**: the tasks artifact is rendered as plain markdown
  here, like every other tab. Grouped checkbox items and the progress bar are the next
  change.
- **`live-refresh`'s scope, entirely**: no `notify` watcher, no debounce, no worker
  thread, no `r` key, and no CLI correction. Content is read when the selected
  change or tab changes, and at no other moment.
- Re-opening the `j` / `k` binding. `markdown-viewer` bound `j` / `k` and the arrows to
  detail scrolling at `Route::Detail` and renamed `Action::SelectNext` / `SelectPrev` to
  `Next` / `Prev`; `openspec/IMPLEMENTATION-ORDER.md` records that this change does not
  re-open it.
- Agent badges, the unattributed-agent footer, action keys, and the `file mode` badge —
  `agent-attribution`, `agent-launch`, and `degraded-states`.
- Any change to `Change`, `ChangeSet`, `changes::from_files`, `changes::from_cli`,
  `schema::*`, or `tasks::*`.
- Horizontal scrolling of the tab bar by mouse, tab reordering, closing a tab, and
  remembering a per-change tab across selection moves.

## Capabilities

### New Capabilities

- `detail-header`: the detail region's first row — change name, schema, progress cell,
  the fixed-field grammar, the drop-whole degradation order, and the no-change state.
- `artifact-tabs`: the tab bar built from the schema's ordered artifact list, positional
  addressing with duplicate ids intact, the `1`–`9` / `[` / `]` keys, the window that
  keeps the selected tab visible, and the zero-artifact and more-than-nine boundaries.
- `artifact-content`: the injected reader, the (change, tab) key that decides when to
  re-read, multi-path concatenation in path order, `No content yet`, and a read failure
  named rather than swallowed.

### Modified Capabilities

- `dashboard-loop`: `Action` gains `SelectTab`, `NextTab`, and `PrevTab`; `Detail` gains
  three fields; `run_loop` gains the injected reader and syncs before it draws; the pure
  view-file set grows to seven.
- `detail-scroll`: the scrolled region becomes the detail interior's **content** area
  rather than the whole interior, and the scrolled line list becomes
  `ui::detail::content_lines` rather than `markdown::lines` alone; a change or tab move
  resets the offset the way a route move already does.
- `responsive-layout`: the detail region's interior is no longer blank on the production
  pane; it is blank exactly when no change is selected.

## Impact

- **Code:** `src/ui/detail.rs` (new — the detail region's grammar, plain data, no
  ratatui, no I/O), `src/ui/app.rs`, `src/ui/layout.rs`, `src/ui/view.rs`,
  `src/ui/driver.rs`, `src/ui/mod.rs`, `src/ui/list.rs` (`progress_cell` and
  `pad_or_truncate_right` become `pub(crate)` — no behaviour change), `src/changes.rs` (a
  test-only `fixture::with_artifacts`), and `src/lib.rs` (`testutil` gains a recording
  artifact-reader double). `Detail`'s three new fields touch **twenty** literal and
  pattern sites and `Dashboard`'s literals **twenty-nine**, since `..` is forbidden;
  `run_loop`'s new parameter touches **ten** call sites. Six existing tests change their
  expectations, named in `tasks.md` group 9 and group 10.
- **Docs:** `SPEC.md` → Architecture (the render seam), Responsive layout, Detail view,
  Keys, Degraded states, Module map, Unit-tested modules, and View tests; `AGENTS.md` →
  Current repo state and Architecture rules.
- **Checks:** `NOIO-VIEW` (its pure set gains `src/ui/detail.rs`) and `OPENSPEC-UNTOUCHED`
  (its one excluded path) are edited **on disk** and re-run; `READSEAM`, `NOTABSEAM`, and
  `DETAILWIDTHS` are added, each with planted-violation controls; `NOSPAWN-GREP`,
  `NOCLI-SHELL`, `NOLIT-CHANGE`, `MDSEAM`, and `WIDTHS` keep their executable logic
  byte-identical and move only a parameter in the invocation; and `NODEFAULT-UI`,
  `NORAW-GREP`, `LISTWIDTHS`, `MDWIDTHS`, `NOWAIVER`, `GATE-MECH1`, `NOJSON-SEAM`,
  `TESTCOUNT`, `DEPS`, and `GRAPH-SNAP` are carried forward byte-identically — `DEPS` and
  `GRAPH-SNAP` must pass **unchanged**, which is this change's proof that no dependency
  crept in.
- **No dependency, manifest, or configuration change:** `Cargo.toml`, `Cargo.lock`,
  `tests/fixtures/build-graph.txt`, `herdr-plugin.toml`, the `config.toml` format, the
  state-file format, and the exit statuses are untouched. No re-link, no re-install.
- **Data model, jobs, external services, sibling repositories:** none.
