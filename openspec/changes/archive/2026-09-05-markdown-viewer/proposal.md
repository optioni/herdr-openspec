## Why

`herdr-openspec ui` now paints a real change list, but the detail region beside it is
still an empty bordered frame. `PRD.md` → Goal 1 ("read a change without leaving Herdr")
and the first Reading story are unmet until an artifact's markdown can be turned into
cells and scrolled. Nothing in the crate can render markdown at all today.

This is the Phase 4 `markdown-viewer` row of `openspec/IMPLEMENTATION-ORDER.md`, whose
single dependency is `tui-shell` (archived 2026-09-04). It is the sibling of `list-view`
(archived 2026-09-05), so nothing blocks it.

## What Changes

- **A markdown transformation.** `ui::markdown::lines(source, width)` turns a markdown
  string into plain-data lines of faced segments — headings, paragraphs, bullet and
  ordered lists, code blocks, block quotes, thematic breaks, emphasis, strong, inline
  code, and links — word-wrapped to the interior width it is given. It imports no
  ratatui and performs no I/O; `ui::view` maps a face to a `Modifier`, exactly as
  `ui::list` returns plain strings and lets the view style them.
- **The detail region draws it, with scrolling.** `Dashboard` gains a `detail: Detail`
  field carrying the markdown source and a scroll offset; `ui::view` draws the slice the
  current interior height allows, clamped on every draw; `ui::driver` normalises the
  stored offset against the frame just drawn, so scrolling past the end cannot run away.
- **BREAKING — `j` / `k` and the arrows scroll the detail content at `Route::Detail`.**
  They still move the list selection at `Route::List`. `SPEC.md` → Keys binds them
  unconditionally to the list selection today, and `list-view` deferred the collision to
  "the first change with a competing claim on those keys", predicting `detail-view`.
  That change is this one. `Action::SelectNext` / `SelectPrev` are renamed `Next` /
  `Prev`, since the action is route-agnostic and only its effect is not.
- **The crate's fifth dependency, `pulldown-cmark` 0.13.4**, `default-features = false,
  features = []`, argued in `design.md` → Decisions. It adds exactly two packages
  (`pulldown-cmark`, `unicase`), no proc-macro, and its MSRV of 1.71.1 leaves the crate's
  1.88 floor untouched.
- **No PRD non-goal is crossed.** Nothing is written to any OpenSpec file; nothing is
  orchestrated across changes; nothing is authored; no Windows code is added. The viewer
  is read-only, and no `openspec` binary is consulted.

## Non-Goals

- **`detail-view`'s scope, entirely**: no change header, no artifact tab bar, no
  `1`–`9` / `[` / `]` tab switching, no "No content yet" state, and no resolution of
  *which* artifact is shown. Nothing populates `Detail::source` in this change; the
  production pane's detail region is unchanged, and `detail-view` supplies the source.
- The tasks tab's grouped-checkbox rendering — `tasks-tab`.
- Markdown the CommonMark core does not model: tables, footnotes, strikethrough, and
  task-list extensions stay off, and such source renders as its literal text rather than
  being dropped. Deferred, with the resolution point recorded in `design.md`.
- Syntax highlighting, colour, mouse support, text selection, link following, and
  horizontal scrolling.
- Any change to `Change`, `ChangeSet`, `changes::from_files`, or `changes::from_cli`.

## Capabilities

### New Capabilities

- `markdown-render`: markdown source to width-parameterised plain-data lines of faced
  segments — every block and inline construct, the wrapping rule, and totality over
  arbitrary input at any width.
- `detail-scroll`: the detail region's content, the scroll offset on `Dashboard`, the
  keys that move it, the clamp derived on every draw, and the driver's normalisation.

### Modified Capabilities

- `dashboard-loop`: `Dashboard` gains an eighth field and a third gated state type;
  `Action`'s two navigation variants are renamed and their effect becomes route-
  dependent; the pure-view file set grows to six.
- `list-selection`: the selection moves on `Next` / `Prev` **at `Route::List`**, which
  the landed requirement leaves unqualified.
- `responsive-layout`: the detail region's interior is no longer unconditionally blank,
  and its two mandated interior widths — 78 and 58 — are frozen for `detail-view` and
  `tasks-tab` to inherit.
- `plugin-build`: the argued dependency set gains `pulldown-cmark`, with its features,
  its MSRV, its two resolved packages, and its removal experiment.

## Impact

- **Code:** `src/ui/markdown.rs` (new), `src/ui/app.rs`, `src/ui/layout.rs`,
  `src/ui/view.rs`, `src/ui/driver.rs`, `src/ui/mod.rs`, `src/ui/list.rs` (its four
  test-fixture `Dashboard` literals only — no behaviour change), and `src/lib.rs`, whose
  `testutil` gains the scripted `EventSource` double lifted out of `src/ui/driver.rs`'s
  private test module. `Dashboard`'s new field touches **fourteen** literal and pattern sites
  across six files, since `..` is forbidden. Plus `Cargo.toml`, `Cargo.lock`, and
  `tests/fixtures/build-graph.txt`.
- **Docs:** `SPEC.md` → Detail view, Keys, Responsive layout, Module map, Degraded
  states, and Unit-tested modules; `AGENTS.md` → Current repo state and Architecture
  rules.
- **Checks:** `NOIO-VIEW`, `NODEFAULT-UI`, `DEPS`, and `GRAPH-SNAP` are edited **on
  disk** and re-run; `MDWIDTHS` and `MDSEAM` are added; `NOSPAWN-GREP`, `NOCLI-SHELL`,
  `NORAW-GREP`, `NOLIT-CHANGE`, `WIDTHS`, `LISTWIDTHS`, `NOWAIVER`, `GATE-MECH1`,
  `NOJSON-SEAM`, `TESTCOUNT`, and `OPENSPEC-UNTOUCHED` are carried forward.
- **No manifest and no configuration change:** `herdr-plugin.toml`, the `config.toml`
  format, the state-file format, and the exit statuses are untouched. No re-link, no
  re-install.
- **Data model, jobs, external services, sibling repositories:** none.
