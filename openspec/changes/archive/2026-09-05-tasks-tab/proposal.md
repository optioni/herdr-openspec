## Why

`herdr-openspec ui` renders every artifact tab as plain markdown, the tasks artifact
included. A `tasks.md` read that way is a wall of `- [ ]` source lines: the reader has to
count checkboxes by eye to answer the one question the pane exists to answer — how far
along is this change. `PRD.md` → Goal 2 ("see progress at a glance") is met in the list
row's `[4/9]` cell and nowhere else; the detail region, which is where a reader actually
looks at a change, shows the same information only as raw markup.

This is the Phase 4 `tasks-tab` row of `openspec/IMPLEMENTATION-ORDER.md`. Both its
dependencies have landed: `detail-view` (archived 2026-09-05) supplies the tab bar and
the content area, and `task-parsing` (archived 2026-09-04) supplies the parse — groups
under their headings, items with a checked state, and counts that reproduce the OpenSpec
CLI's own rule.

## What Changes

- **The tracked-tasks tab renders a checklist instead of markdown.** Groups under their
  headings, one `[x]`/`[ ]` glyph per item, the item's own text wrapped with a hanging
  indent, and a blank line between groups. Every other tab is unchanged, still rendered
  by `markdown-render`.
- **A progress bar leads that tab's content.** A bare gauge of `█`/`░` cells, then the
  change's progress cell, then a whole-number percentage — degrading by dropping whole
  cells the way `change-rows` and `detail-header` already do, down to nothing at zero
  width.
- **The count is not recomputed.** The bar renders `Change::progress`, the same value the
  detail header and the list row show, so the pane can never display two numbers for one
  change. No second counting rule is introduced anywhere.
- **`ArtifactRef` gains `tracks_tasks: bool`,** set at the one position the schema's
  `apply.tracks`-then-id-`tasks` rule names — `schema-model` already computes it and
  `changes` already discards it. This is how the view knows which tab is the tasks tab:
  by **position**, never by guessing an id or a filename. Both producers set it, and the
  positional artifact join carries it.
- **`ui::detail::content_lines` gains the selected `Change`** so it can make that one
  decision in one place, for both its callers — the draw and the scroll clamp.
- **A new plain-data grammar module, `src/ui/tasks.rs`,** on exactly `ui::detail`'s
  terms: no ratatui type, no I/O API, every public function parameterised by width.
  `NOIO-VIEW`'s pure set grows to **eight** files.
- **The tasks tab is read-only, and that is proved rather than asserted.** No key toggles
  an item, and a scripted run of every printable key over a real change directory leaves
  the tree byte-identical.
- **No new dependency, no manifest change, no configuration change, no keybinding
  change.** `ratatui::widgets::Gauge` is deliberately not used — the content area is a
  line list, and a widget would need its own rectangle and its own scroll rule.
  Not **BREAKING**: no existing key moves, and no interface a separate consumer depends
  on changes.
- **No PRD non-goal is crossed.** Nothing is written to any OpenSpec file, nothing is
  orchestrated across changes, nothing is authored, no Windows code is added.

## Non-Goals

- **`live-refresh`'s scope, entirely**: no `notify` watcher, no ~150ms debounce, no
  per-change invalidation, no worker thread, and no `r` force-refresh key. Content is
  still read when the selected change or tab changes and at no other moment.
- **Toggling, editing, or writing a checkbox.** Forbidden by `PRD.md` → Non-goals and by
  `SPEC.md` → Detail view: an agent may be editing `tasks.md` in another pane, and a
  write would race it. Rendering a checkbox makes toggling look natural, which is why
  this change carries a test that no key does it.
- **Any tab-bar change.** `detail-view` already renders a tab for every artifact the
  schema declares, this change replaces one tab's *content*, and the roadmap records
  that it adds no tab-bar code of its own. No tab is added, removed, reordered, or
  hidden.
- **Nesting, folding, collapsing, or filtering task items.** `tasks::parse` never nests;
  an item's `indent` is rendered, not interpreted as a tree.
- **Inline markdown inside an item's text.** An item's text is rendered as its literal
  source, the same treatment `markdown-render` gives a construct it does not model.
- **The CLI path.** No `openspec` binary is consulted; this change works with none
  installed at all. `changes::from_cli` is touched only to keep the two producers'
  `ArtifactRef` in agreement.
- Agent badges, the unattributed-agent footer, action keys, and the `file mode` badge —
  `agent-attribution`, `agent-launch`, and `degraded-states`.

## Capabilities

### New Capabilities

- `tasks-checklist`: the tracked-tasks tab's line grammar — the group heading line, the
  checkbox glyph and hanging-indent wrap per item, the blank-line separation, the
  `No tasks yet` state for a document yielding no task items, and the read-only guarantee
  that no key mutates an item or the tree.
- `tasks-progress-bar`: the gauge grammar — the `█`/`░` run, the shared progress cell,
  the percentage, the drop-whole degradation order, the zero-total and degenerate-width
  cases, and the property that the gauge is full exactly when `Progress::is_complete()`.

### Modified Capabilities

- `change-model`: `ArtifactRef` carries a third field, `tracks_tasks`, and a marked artifact
  with no resolved file is still a shown tab reading `No content yet`.
- `change-artifacts`: `ArtifactRef` gains `tracks_tasks`, set by the file producer at the
  first schema position the tasks-artifact rule names, and false everywhere else.
- `cli-changes`: the CLI producer sets `tracks_tasks` from the same schema by the same
  rule, so the two producers still emit the same value.
- `change-merge`: the positional artifact join carries `tracks_tasks` with its artifact.
- `artifact-content`: `content_lines` takes the selected `Change` and returns the
  checklist grammar for the tracked-tasks tab, `markdown::lines` for every other.
- `detail-scroll`: the scrolled line list for the tracked-tasks tab is the checklist, and
  the clamp is computed against it by the same `content_lines` call the draw uses.
- `dashboard-loop`: the pure view-file set grows to **eight** with `src/ui/tasks.rs`, and the
  search that guards it gains `tasks::read` — the one filesystem call a checklist renderer
  would plausibly reach for, which none of the existing patterns matches.

## Impact

- **Code:** `src/ui/tasks.rs` (new — plain data, no ratatui, no I/O), `src/ui/detail.rs`,
  `src/ui/view.rs`, `src/ui/app.rs`, `src/ui/mod.rs`, `src/changes.rs` (`ArtifactRef`'s
  new field, both producers, and a test-only fixture helper), and `src/ui/list.rs`
  (nothing beyond the already-`pub(crate)` helpers). `ArtifactRef`'s new field touches
  **32** construction sites in `src/changes.rs`, all compile-enforced — a naive
  `grep -c 'ArtifactRef {'` reports 36, of which the struct definition, a doc comment, a
  `-> ArtifactRef {` signature, and one doc comment in `src/ui/detail.rs` are not
  constructions; `content_lines`' new parameter touches **8** call sites (two production,
  six in `src/ui/detail.rs`'s tests).
- **Docs:** `SPEC.md` → Detail view, Degraded states (one row rewritten, three added),
  Module map, Unit-tested modules, and View tests; `AGENTS.md` → Current repo state and two
  Architecture rules (the pure set, and the detail-region width rule that names which
  modules assert 58 and 78).
- **Checks:** `NOIO-VIEW`'s pure set gains `src/ui/tasks.rs` and `OPENSPEC-UNTOUCHED`'s
  one excluded path moves — both edited **on disk** and re-run; `TASKSEAM`,
  `TASKWIDTHS`, and `READONLY-UI` are added, each with a planted-violation control;
  `NOSPAWN-GREP`, `NOCLI-SHELL`, `NOLIT-CHANGE`, `MDSEAM`, `READSEAM`, `NOTABSEAM`,
  `WIDTHS`, and `DETAILWIDTHS` keep their executable logic byte-identical and move only
  a parameter in the invocation; and `NODEFAULT-UI`, `NORAW-GREP`, `LISTWIDTHS`,
  `MDWIDTHS`, `NOWAIVER`, `GATE-MECH1`, `NOJSON-SEAM`, `TESTCOUNT`, `DEPS`, and
  `GRAPH-SNAP` are carried forward byte-identically. `DEPS` and `GRAPH-SNAP` passing
  **unchanged** is this change's proof that no dependency crept in.
- **No dependency, manifest, or configuration change:** `Cargo.toml`, `Cargo.lock`,
  `tests/fixtures/build-graph.txt`, `herdr-plugin.toml`, the `config.toml` format, the
  state-file format, and the exit statuses are untouched. No re-link, no re-install.
- **Data model, jobs, external services, sibling repositories:** none.
