# Tasks

Baseline recorded at HEAD `0437e05` before planning. **`cargo test --all-features` is not
reliably green on this machine right now**: 4–5 tests under `ui::tests::wiring` fail, a
different set on each run (`5 failed` then `4 failed`), and each passes in isolation
(`cargo test --all-features --lib g_with_no_agent_renders_the_same_buffer` → `1 passed`).
Two other Claude sessions are working in this same checkout concurrently. Establish a green
baseline before task 1.1 and do not attribute a wiring failure to this change.

## 1. Frame and region geometry
<!-- kind: behavior -->

The three signature changes here are compile errors at every call site, so nothing below
builds until this group lands. See design.md → Decisions D1, D2, D3.

- [ ] 1.1 RED: add `ui::layout::tests::interior_reserves_two_rows_and_the_named_gutters`
  asserting `interior(Rect::new(0,0,60,19), Gutters::Both) == Rect::new(1,2,58,17)`,
  `interior(Rect::new(0,0,40,19), Gutters::Both) == Rect::new(1,2,38,17)`, and
  `interior(Rect::new(41,0,79,19), Gutters::LeftOnly) == Rect::new(42,2,78,17)`.
  Check: `grep -c 'Gutters' src/ui/layout.rs` → `0`, exit 1 at HEAD, so the type does not
  exist and the test cannot compile, let alone pass.
- [ ] 1.2 GREEN: add `pub enum Gutters { Both, LeftOnly }` and change `interior` to
  `interior(area: Rect, gutters: Gutters) -> Rect`, advancing the origin by the left gutter
  and **two** rows, reducing width by `gl + gr` and height by two, each saturating, keeping
  the existing origin clamp. Verify: 1.1 passes.
- [ ] 1.3 RED: add `ui::layout::tests::split_frame_is_body_then_footer` asserting
  `split_frame` returns `(body, footer)` with body `Rect::new(0,0,w,h-1)` at `h >= 2`, the
  body alone at `h == 1`, and both zero-height at `h == 0`.
  Check: `grep -n 'pub fn split_frame' src/ui/layout.rs` → `-> (Rect, Rect, Rect)` at HEAD,
  so the three-tuple destructuring in the test does not compile.
- [ ] 1.4 GREEN: drop the header rect from `split_frame` and update `ui::view::render`'s
  destructuring. Verify: 1.3 passes.
- [ ] 1.5 RED: add `ui::layout::tests::the_wide_body_splits_into_list_divider_detail`
  asserting `split_body` at `Rect::new(0,0,120,19)` gives list `Rect::new(0,0,40,19)`,
  divider column `40`, and detail `Rect::new(41,0,79,19)`, and that below the breakpoint
  there is no divider. Verify: fails at HEAD — `split_body` returns two `Option<Rect>` and
  names no divider column.
- [ ] 1.6 GREEN: add the `Constraint::Length(1)` divider part between the two regions per
  design.md → D4/D5. Verify: 1.5 passes.
- [ ] 1.7 RED: add `ui::layout::tests::split_detail_is_tabs_rule_content` covering interior
  heights `0`, `1`, `2`, `3`, `4`, and `17` against `artifact-tabs`' table, including that at
  height `17` the content area is fourteen rows at `interior.y + 3`. Verify: fails at HEAD —
  `split_detail` returns a header rect first.
- [ ] 1.8 GREEN: change `split_detail` to `(tabs, rule, content)` on that table. Verify: 1.7
  passes.
- [ ] 1.9 REFACTOR: run `cargo clippy --all-targets --all-features -- -D warnings` and
  `cargo fmt --all -- --check`; fix what they name and nothing else.

## 2. The palette's roles
<!-- kind: behavior -->

- [ ] 2.1 RED: extend `ui::palette::tests` to assert `RegionHeading` carries `DIM` and no
  colour, `RegionHeadingFocused` carries `BOLD` and no colour, and `RegionRule` carries `DIM`
  and no colour, and that the exhaustive `match` names no `RegionBorder`, `HeaderTitle`,
  `HeaderPath`, or `DetailHeader`.
  Check: `grep -c 'RegionBorder' src/ui/palette.rs` → `9`, exit 0 at HEAD, so the removal
  assertion fails and the three new variants do not exist.
- [ ] 2.2 GREEN: remove the five roles, add the three, and update the modifier table per
  `view-palette`'s delta. Every call site that named a removed role is a compile error; fix
  each to the role `view-palette`'s draw-span mapping gives it. Verify: 2.1 passes.
- [ ] 2.3 Verify the confinement gate still holds: `bash scripts/gates/palette.sh` exits 0 and
  names the file count it searched. It exits 0 at HEAD, so this pins an invariant rather than
  proving new behaviour, and `tests/gate_controls.rs` is what proves it can fail.

## 3. The list region's heading and its padding row
<!-- kind: behavior -->

- [ ] 3.1 RED: add `ui::view::tests::the_heading_names_the_directory_not_the_path` rendering a
  dashboard whose root is `/Users/dev/Code/herdr-openspec` at 120x20 and 60x20 and asserting
  row 0 spells `herdr-openspec` from column 1 and `/Users/dev/Code` appears in no cell.
  Check: `grep -c '"OpenSpec"' src/ui/view.rs` → `5`, exit 0 at HEAD — the literal is still
  drawn and the path is still right-aligned, so the assertion fails on both halves.
- [ ] 3.2 GREEN: delete `render_header` and `shorten_for_header`; rewrite `render_region` to
  draw a heading row and no `Block`, taking the directory name through
  `ui::list::shorten_left`. Verify: 3.1 passes and
  `grep -c 'fn render_header' src/ui/view.rs` → `0` (it is `1` at HEAD).
- [ ] 3.3 RED then GREEN: the `file mode` badge, right-aligned and dropped whole below its
  budget, per `responsive-layout`'s badge scenario at 120, 60, 20 and 19 columns.
- [ ] 3.4 RED then GREEN: the padding row — assert every cell of row 1 inside the interior's
  columns is a space whose `Style` equals `Cell::default().style()`, and that the first list
  row is row 2 at both widths.
- [ ] 3.5 RED then GREEN: the divider and its two blank columns — column 40 is `│`, columns
  39 and 41 are spaces, column 0 is a space, and column 119 carries detail content.
- [ ] 3.6 Re-baseline `ui::list` and `ui::view`'s row expectations against the seventeen-row
  interior: `cargo test --all-features ui::list ui::view`. Only indices that name the
  interior's **last** row should move.

## 4. The detail region's heading, rule, and padding row
<!-- kind: behavior -->

- [ ] 4.1 RED: assert the change header is buffer row 0 at columns 42–119 (120x20) and 1–58
  (60x20), bold at `Route::Detail` and dim at `Route::List`. Verify: fails at HEAD, where the
  header is row 2 at columns 41–118 and is always bold.
- [ ] 4.2 GREEN: draw `detail::header_row` into the region's heading row with
  `RegionHeadingFocused`/`RegionHeading`. Verify: 4.1 passes.
- [ ] 4.3 RED then GREEN: the tab bar at buffer row 2, the `─` rule at row 3 in `RegionRule`,
  the padding row at row 4, and content from row 5 — fourteen rows to row 18.
- [ ] 4.4 Verify the content area did not change size: `cargo test --all-features ui::view
  ui::layout` — every "fourteen-row content area" expectation in `detail-scroll` must pass
  **unmodified**. If one needs editing, the arithmetic is wrong; re-read design.md → D2.
- [ ] 4.5 Verify the mandated widths did not move: `bash scripts/gates/detailwidths.sh`,
  `bash scripts/gates/mdwidths.sh`, and `bash scripts/gates/taskwidths.sh` each exit 0 and
  report all tests naming both `58` and `78`. All three exit 0 at HEAD; they pin an invariant
  this change must not break.

## 5. The fold glyphs
<!-- kind: behavior -->

- [ ] 5.1 RED: assert `section_row_text` yields `  ▾ active (3)` open and `  ▸ archived (30)`
  collapsed, that a selected collapsed section reads `> ▸ archived (30)` and holds no `> >`,
  and that `layout::columns` of each row is exactly the requested width at 38, 58, 17, 16, 5,
  1 and 0.
  Check: `grep -c '▾\|▸' src/ui/list.rs` → `0`, exit 1 at HEAD.
- [ ] 5.2 GREEN: swap the glyph pair in `section_row_text`. Verify: 5.1 passes and
  `bash scripts/gates/colwidth.sh` exits 0 — the glyphs go through `layout::columns` like
  every other cell.
- [ ] 5.3 Update every landed expectation naming `  v ` or `  > ` in `src/ui/list.rs`,
  `src/ui/view.rs` and `src/ui/app.rs`. Verify: `grep -rn '`  v \|`  > ' src/ui/` returns
  nothing.

## 6. The hit test
<!-- kind: behavior -->

- [ ] 6.1 RED: extend `ui::layout::tests` so `zone` returns `List` for the heading and padding
  rows, `Detail` for the divider column 40, and `ListRow`/`DetailTab` at the new offsets, and
  so row 0 resolves to a region rather than `Outside`. Verify: fails at HEAD, where row 0 is
  the frame header and resolves to `Outside`.
- [ ] 6.2 GREEN: update `zone` to derive through the new `split_frame`, `interior` and
  `split_detail`, adding the divider-column branch per `responsive-layout`'s zone requirement.
- [ ] 6.3 Verify the drawn/hit-test agreement still holds at both widths:
  `cargo test --all-features the_hit_test_agrees_with_the_drawn_buffer` and
  `every_drawn_row_is_reported_by_row_at`.

## 7. Documentation sites bound inside `cargo test`
<!-- kind: operational -->

- [ ] 7.1 Update `SPEC.md`'s frame description and diagram, its degraded-states row for the
  `file mode` badge, and its documented terminal-seam names. Verify:
  `cargo test --all-features --test doc_contract --test degraded_coverage` passes — both fail
  and name each side while the docs disagree with the code.
- [ ] 7.2 Update `AGENTS.md` — the mandated-width paragraph's justification (the widths
  themselves do not move), the pure-view-set description, and the current-repo-state
  paragraph. Verify: `cargo test --all-features --test doc_contract`.
- [ ] 7.3 Update `README.md` only if a documented key or binding changed. It did not, so the
  expected result is no edit; confirm with `cargo test --all-features --test manifest`.

## 8. Change Review
<!-- kind: operational -->

- [ ] 8.1 Dispatch the outside-in TDD reviewer against this change's artifacts and diff.
- [ ] 8.2 Resolve every finding, or record why it is declined, in `planning-review.md`.

## 9. Final verification
<!-- kind: operational -->

- [ ] 9.1 `make check` — the single gate. If it fails, name the failing sub-command
  (`fmt`, `clippy`, `gates`, `test`, or `coverage`) and fix that, not the gate.
- [ ] 9.2 Confirm the plugin's own writes are unchanged: `git status --short openspec/` shows
  no modification under `openspec/specs/` or `openspec/changes/archive/` caused by running the
  pane. Note that `make gates` currently reports untracked directories under
  `openspec/changes/` belonging to two **other** in-flight sessions; that is not this change's
  doing and must not be resolved by deleting them.
- [ ] 9.3 Build and look at it: `make build`, then open the pane in a Herdr split at 60
  columns and in a dedicated tab, and compare against the prototype named in design.md.
