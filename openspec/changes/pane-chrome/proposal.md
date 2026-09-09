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
  area gains two rows.
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
- The detail region gains separation: a blank row between the change header and the tab bar,
  and a horizontal rule between the tab bar and the content.

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

- `responsive-layout`: the frame is a body and a footer, not a header, a body and a footer;
  a region is a heading row plus a gutter-padded interior, not a bordered block; the wide
  layout carries a vertical rule; `interior` reserves one row above rather than two.
- `change-rows`: the list region's first row is the repository heading, and the section fold
  glyphs become `▾`/`▸`.
- `detail-header`: the change header is the detail region's heading row, dim when the region
  is not routed.
- `artifact-tabs`: the tab bar moves to the detail interior's **third** row, under a blank
  row, with a horizontal rule drawn beneath it.
- `mouse-input`: `zone` resolves points against the borderless geometry — a heading row is
  not a list row, and the tab bar's row moved.
- `view-palette`: `RegionBorder`/`RegionBorderFocused` become `RegionHeading`/
  `RegionHeadingFocused`, and `RegionRule` is added for the vertical and horizontal rules.

## Impact

- `src/ui/layout.rs` — `split_frame`, `split_body`, `interior`, `split_detail`, `zone`.
- `src/ui/view.rs` — `render`, `render_header` (removed), `render_region` (rewritten as a
  heading), `render_detail`, `render_detail_tabs`, plus the new rule drawing.
- `src/ui/list.rs` — `section_row_text`'s glyphs, and the heading row's own text.
- `src/ui/palette.rs` — the renamed and added roles.
- `SPEC.md` — the frame diagram, the degraded-states row for the `file mode` badge, and the
  documented mouse bindings; `tests/degraded_coverage.rs` and `tests/doc_contract.rs` bind
  both and will fail until they are updated.
- `AGENTS.md` — the mandated-width paragraph's justification (the widths themselves do not
  move).
- No dependency is added; no manifest, config format, or keybinding changes.
