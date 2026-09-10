## Why

In a Herdr split the pane is already framed and titled by Herdr itself, and the dashboard
then redraws that same chrome inside it: a `OpenSpec` header row under a pane border that
already says `OpenSpec`, and a bordered `Changes` box inside a pane that is already a box.
Measured live at a 60-column split, four of the frame's rows and four of its columns carry
no information at all, and the one row that could — the header — spends it on a literal
label plus an absolute repository path too long to read. Inside the detail region the
change header, the tab bar, and the artifact's first line of markdown are three adjacent
rows with nothing between them, so the reader parses the tab bar as content. On a selected
collapsed section the cursor marker and the fold glyph are the same character and render as
`> > archived (30)`.

This is unplanned work: the roadmap in `openspec/IMPLEMENTATION-ORDER.md` ends at Phase 6
(`degraded-states`), and every layout decision this change revisits was made in `tui-shell`
against a full-screen terminal rather than against the narrow split the pane actually
occupies.

## What Changes

- The frame loses its header row. `render` divides `frame.area()` into a body and a footer
  row; the repository's identity moves into the list region's own first row.
- Regions lose their borders. Each region keeps a one-column gutter on its left and right
  and spends its former top border row on a **heading row**, so every mandated interior
  width — 38 and 58 for the list, 78 and 58 for the detail — is unchanged, and the content
  area gains **one** row: two are freed (the frame header and the bottom border) and one is
  spent on the padding row, so a region's interior grows from sixteen rows to seventeen and
  still begins at buffer row 2.
- The list region's heading row names the repository's **directory name**, not its absolute
  path, with the `file mode` badge right-aligned and dropped whole when the row cannot hold
  both. The detail region's heading row stays the change header `detail-header` already
  specifies.
- The unfocused region's heading row is dim; the focused region's is not. This replaces the
  bold border that said the same thing.
- In the wide layout a single vertical rule separates the two regions, drawn in the detail
  region's left gutter.
- The section fold glyph becomes `▾` (open) and `▸` (collapsed), so it can never be
  confused with the `>` cursor marker that sits two columns left of it.
- The detail region gains separation, in **three** rows rather than two: a blank row between
  the change header and the tab bar, a horizontal rule beneath the tab bar, and a second
  blank row between that rule and the content, so the artifact's own first line never sits
  against the rule. The content area therefore begins at the interior's fourth row.

## Non-Goals

- No change to what the pane reads, writes, or spawns. This change is confined to the pure
  view layer and the geometry it is handed; `src/cli.rs`, `src/watch.rs`, `src/refresh.rs`,
  `src/agents.rs`, and `src/launch.rs` are untouched.
- No new keybinding, no new mouse gesture, and no key removed — **not BREAKING**: the
  plugin manifest, the plugin config format, and every binding are unchanged.
- No colour is added or changed beyond the roles this change renames and the two it adds;
  choosing what a role looks like stays `ui::palette`'s job.
- The false `is not vendored` problem row is **not** fixed here. It reverses a written
  decision in `change-merge` and is proposed separately.
- No editing of OpenSpec files, no orchestration across changes, no change authoring, no
  Windows support.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

Eleven, not six. The five at the end of this list carry no new behaviour of their own —
they are the capabilities whose existing scenarios state a row index, a viewport figure or an
interior height that the new geometry moves, and a delta that leaves them saying the old
number is a delta that contradicts this change. `design.md` → Risks counts the same eleven.

- `responsive-layout`: the frame is a body and a footer, not a header, a body and a footer;
  a region is a heading row, a padding row, and a gutter-padded interior, not a bordered
  block; the wide layout carries a vertical rule; `interior` reserves **two rows above and
  none below**, where the bordered arithmetic reserved one of each.
- `change-rows`: the list region's first row is the repository heading, and the section fold
  glyphs become `▾`/`▸`.
- `detail-header`: the change header is the detail region's heading row, dim when the region
  is not routed.
- `artifact-tabs`: the tab bar moves to the detail interior's **first** row — it was the
  second — under the region's own padding row, with a horizontal rule and a second padding
  row drawn beneath it.
- `mouse-input`: `zone` resolves points against the borderless geometry — a heading row is
  not a list row, and the tab bar's row moved.
- `view-palette`: `RegionBorder`/`RegionBorderFocused` become `RegionHeading`/
  `RegionHeadingFocused`, and `RegionRule` is added for the vertical and horizontal rules.
- `artifact-content`: its interior arithmetic says the header and tab bar take two rows of a
  sixteen-row interior; they take three of a seventeen-row one.
- `detail-scroll`: it states `interior`'s signature and its degenerate contract, both of
  which change, and every row index its scroll clamp is written against.
- `list-selection`: its viewport figures are computed from a sixteen-row interior.
- `list-filtering`: its filtered-viewport row indices move with the interior's first row.
- `tasks-checklist`: its progress-bar and group row indices move with the content area.

## Impact

- `src/ui/layout.rs` — `split_frame`, `split_body`, `interior`, `split_detail`, `zone`.
  **Four** signatures change, not three: `split_body` gains the divider column alongside the
  three `design.md` → Contracts already tabulates.
- `src/ui/view.rs` — `render`, `render_header` (removed), `render_region` (rewritten as a
  heading), `render_detail`, `render_detail_tabs`, plus the new rule drawing.
- `src/ui/list.rs` — `section_row_text`'s glyphs, and the heading row's own text.
- `src/ui/palette.rs` — the renamed and added roles.
- `src/ui/app.rs`, `src/ui/driver.rs`, `src/ui/mod.rs` — **compile errors, not edits of
  choice**: each destructures `split_body`'s pair (`app.rs:624`, `driver.rs:3516` and
  `:3526`, `mod.rs:758`) and none builds until the new signature lands. `app.rs`'s
  `normalise_scroll` additionally feeds `content.width` into `content_lines`, so the scroll
  clamp moves with the interior.
- `SPEC.md` — the frame description, the degraded-states row for the `file mode` badge, the
  documented mouse bindings, and § List view's fold-glyph examples and prose. Only the
  degraded-states row is machine-bound (`tests/degraded-coverage.toml`); the mouse-binding
  prose and the glyph examples drift **silently**, which is why they are listed here.
- `tests/degraded-coverage.toml` — the `openspec` binary not found row: both proof test
  names, its `why`, and its `covers` range, one of which names the deleted `render_header`.
- `openspec/specs/{responsive-layout,detail-header,artifact-tabs,view-palette}/spec.md` —
  each `## Purpose` states something this change falsifies (a bordered body, an emphasised
  border, a `bold` detail header, the removed `HeaderTitle`/`DetailHeader` roles).
  `tests/spec_purposes.rs` checks only that a Purpose exists and is not the archive
  placeholder, so nothing catches these.
- `AGENTS.md` — the mandated-width paragraph's justification (the widths themselves do not
  move).
- No dependency is added; no manifest, config format, or keybinding changes.
