# Tasks

Baseline recorded at HEAD `0437e05` before planning, re-measured at `1e658b5` during
planning review. Every commit between the two is documentation for other changes, so no
source figure below moved.

**The `ui::tests::wiring` failures are the load flake this repository has already
diagnosed, not a red baseline.** `openspec/changes/markdown-legibility/design.md` (commit
`027db45`) records the mechanism: `crate::testutil::Stages` (`src/lib.rs:660-724`) bounds
those 29 tests with a five-second wall-clock deadline and force-presses `q` on expiry, so a
machine under load cuts a run short before the staged keys fire. Measured here at
`1e658b5`: `cargo test --all-features` → **1220 passed, 2 failed**, both in that module;
`cargo test --all-features --lib ui::tests::wiring -- --test-threads=1` → **29 passed**.

**The signature separates a flake from a regression:** a deadline flake asserts an expected
count against `0` with an *empty* call log (`left: 0 / right: 4`, `calls: []`); a real
defect produces a wrong call *list*. This change touches no file in that module's subject —
no `HerdrCli`, no launcher, no poller — so a failure there with that signature is not this
change's, and a failure anywhere else is. Re-measure serially before treating any of it as a
regression.

**Sequencing.** No group qualifies for `parallel-after`, and this is a finding rather than an
omission. `src/ui/view.rs` is written by groups 1, 2, 3, 4 and 5; `src/ui/layout.rs` by
groups 1 and 6; `src/ui/list.rs` by groups 3 and 5. Every pair of behaviour groups shares at
least one file, and `cargo test --all-features` compiles the whole crate, so two agents
editing them could not attribute a compile failure between themselves. Group 7 shares no
source file but depends on every code group having landed, since it describes what they did.
The chain is therefore 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9, sequential throughout.

## 1. Frame and region geometry
<!-- kind: behavior -->

The three signature changes here are compile errors at every call site, so nothing below
builds until this group lands. See design.md → Decisions D1, D2, D3.

- [x] 1.1 RED: add `ui::layout::tests::interior_reserves_two_rows_and_the_named_gutters`
  asserting `interior(Rect::new(0,0,60,19), Gutters::Both) == Rect::new(1,2,58,17)`,
  `interior(Rect::new(0,0,40,19), Gutters::Both) == Rect::new(1,2,38,17)`, and
  `interior(Rect::new(41,0,79,19), Gutters::LeftOnly) == Rect::new(42,2,78,17)`.
  Check: `grep -c 'Gutters' src/ui/layout.rs` → `0`, exit 1 at HEAD, so the type does not
  exist and the test cannot compile, let alone pass.
  Named `interior_reserves_two_rows_and_the_gutters_its_gutters_names` instead (matching
  design.md's Verification matrix, which states "the names in the Verification column are
  the contract, not a suggestion" — the matrix row for this scenario names that exact test).
  Also covers the two degenerate cases (`(0,0,1,1)`, `(0,0,0,0)`) `responsive-layout`'s own
  scenario names alongside the three above.
- [x] 1.2 GREEN: add `pub enum Gutters { Both, LeftOnly }` and change `interior` to
  `interior(area: Rect, gutters: Gutters) -> Rect`, advancing the origin by the left gutter
  and **two** rows, reducing width by `gl + gr` and height by two, each saturating, keeping
  the existing origin clamp. Verify: 1.1 passes.
- [x] 1.3 RED: add `ui::layout::tests::split_frame_is_body_then_footer` asserting
  `split_frame` returns `(body, footer)` with body `Rect::new(0,0,w,h-1)` at `h >= 2`, the
  body alone at `h == 1`, and both zero-height at `h == 0`.
  Check: `grep -n 'pub fn split_frame' src/ui/layout.rs` → `-> (Rect, Rect, Rect)` at HEAD,
  so the three-tuple destructuring in the test does not compile.
- [x] 1.4 GREEN: drop the header rect from `split_frame` and update `ui::view::render`'s
  destructuring. Verify: 1.3 passes.
- [x] 1.5 RED: add `ui::layout::tests::the_wide_body_splits_into_list_divider_detail`
  asserting `split_body` at `Rect::new(0,0,120,19)` gives list `Rect::new(0,0,40,19)`,
  divider column `40`, and detail `Rect::new(41,0,79,19)`, and that below the breakpoint
  there is no divider. Verify: fails at HEAD — `split_body` returns two `Option<Rect>` and
  names no divider column.
- [x] 1.6 GREEN: add the `Constraint::Length(1)` divider part between the two regions per
  design.md → D4/D5. Verify: 1.5 passes.
- [x] 1.7 RED: add `ui::layout::tests::split_detail_is_tabs_rule_content` covering interior
  heights `0`, `1`, `2`, `3`, `4`, and `17` against `artifact-tabs`' table, including that at
  height `17` the content area is fourteen rows at `interior.y + 3`. Verify: fails at HEAD —
  `split_detail` returns a header rect first.
  Named `split_detail_is_exact_at_its_degenerate_heights` instead (matching design.md's
  Verification matrix name for `artifact-tabs`'s own scenario of the same title).
- [x] 1.8 GREEN: change `split_detail` to `(tabs, rule, content)` on that table. Verify: 1.7
  passes.
- [x] 1.9 REFACTOR: run `cargo clippy --all-targets --all-features -- -D warnings` and
  `cargo fmt --all -- --check`; fix what they name and nothing else. If nothing needs
  restructuring, record "no refactor was needed" rather than leaving the step unmarked.
  Clippy was clean throughout. `cargo fmt` reformatted two spots in the new `zone` test
  module (an import list line wrap and one call broken across lines); applied via
  `cargo fmt --all` and reverified clean before the GREEN commit was made (the fix landed
  inside that commit rather than a separate one, since it was applied before this group's
  RED/GREEN history was reconstructed for the commit log). No structural refactor was
  needed beyond that formatting fix.
- [x] 1.10 VERIFY: `cargo test --all-features --lib ui::layout` — green. This group changes
  four signatures crate-wide and the earlier draft closed on the linters alone, which cannot
  see a wrong `Rect`.
  19 passed, 1 failed: `ui::layout::tests::zone::the_zones_tile_the_frame` fails at its very
  first case, `(0, 0)` under `Route::List`, expecting `Zone::Outside` (the old frame-header
  contract) where the new geometry correctly gives `Zone::List` — `responsive-layout`'s own
  scenario explicitly requires this ("row 0 of the frame resolves to a region rather than
  Outside, because the frame has no header row for it to belong to"). This is squarely group
  6's re-baselining task (6.1/6.2): the same test's later cases also expect the divider
  column (120, x=40) to resolve to `Detail`, which `zone` does not yet implement (that branch
  is explicitly deferred to task 6.2, per `responsive-layout`'s divider-column requirement).
  Fixing only the first assertion would just move the failure to the next one without
  completing group 6's task, so it is left red and flagged here rather than partially patched.

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
- [ ] 2.3 REFACTOR: no role is left whose only difference from another is its name — D7 removes
  `DetailHeader` for exactly that reason. Record "no refactor was needed" if none applies.
- [ ] 2.4 VERIFY: `cargo test --all-features --lib ui::palette` — green. The earlier draft
  closed this group on the gate alone, which checks confinement rather than the table.
- [ ] 2.5 VERIFY the confinement gate still holds: `bash scripts/gates/palette.sh` exits 0 and
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
- [ ] 3.3 RED: the `file mode` badge, right-aligned and dropped whole below its budget, per
  `responsive-layout`'s badge scenario at 120, 60, **21 and 20** columns — the drop rule's own
  boundary, since an 18-column heading cannot hold `demo-repo`, a blank and the badge's nine.
  Check: `cargo test --all-features --lib file_mode_badge_is_dim_after_the_label` → `1 passed`
  at HEAD, asserting the badge sits after the `OpenSpec` label this change deletes, so the
  rewritten test fails until 3.4 lands.
- [ ] 3.4 GREEN: draw the badge into the heading row's right edge. Verify: 3.3 passes, and
  `tests/degraded-coverage.toml`'s `openspec binary not found` row is updated in the same
  commit — both proof names, its `why`, and its `covers` range, which is `src/ui/view.rs:293-301`
  and spans the deleted `render_header`. Verify:
  `cargo test --all-features --test degraded_coverage`.
- [ ] 3.5 RED: the padding row — assert every cell of row 1 inside the interior's columns is a
  space whose `Style` equals `Cell::default().style()`, and that the first list row is row 2 at
  both widths. Check: at HEAD row 1 is the list region's top border, so the style assertion
  fails.
- [ ] 3.6 GREEN: leave row 1 unpainted in `render_region`. Verify: 3.5 passes.
- [ ] 3.7 RED: the divider and its two blank columns — column 40 is `│`, columns 39 and 41 are
  spaces, column 0 is a space, and column 119 carries detail content. **The test must assert at
  60 as well as at 120**: `scripts/gates/widths.sh` requires every `#[test]` in `src/ui/view.rs`
  to name both widths (`:32`), so a 120-only test fails the gate on arrival. At 60 the assertion
  is that no divider column exists below the breakpoint.
  Check: `grep -c '│' src/ui/view.rs` → `0` at HEAD.
- [ ] 3.8 GREEN: draw the divider in `render_body`. Verify: 3.7 passes.
- [ ] 3.9 Re-baseline `ui::list` and `ui::view`'s row expectations against the seventeen-row
  interior: `cargo test --all-features --lib ui::list` and `cargo test --all-features --lib
  ui::view`, run separately — `cargo test` takes one filter and two bare ones exit 1. Rows that
  name the interior's **last** row move, and so do the first rows at `src/ui/view.rs:2849` and
  `:2863`; the earlier claim that only last-row indices move was wrong.
- [ ] 3.10 Verify the width gates still hold over the two files this group rewrites:
  `bash scripts/gates/widths.sh` and `bash scripts/gates/listwidths.sh` each exit 0. At HEAD
  they report 122 view tests against a floor of 114 and 47 row-grammar tests against 39, so
  this group has **eight** view tests of headroom and task 3.2 deletes roughly that many.

## 4. The detail region's heading, rule, and padding row
<!-- kind: behavior -->

- [ ] 4.1 RED: assert the change header is buffer row 0 at columns 42–119 (120x20) and 1–58
  (60x20), bold at `Route::Detail` and dim at `Route::List`. Verify: fails at HEAD, where the
  header is row 2 at columns 41–118 and is always bold.
- [ ] 4.2 GREEN: draw `detail::header_row` into the region's heading row with
  `RegionHeadingFocused`/`RegionHeading`. Verify: 4.1 passes.
- [ ] 4.3 RED: the tab bar at buffer row 2, the `─` rule at row 3 in `RegionRule`, the padding
  row at row 4, and content from row 5 — fourteen rows to row 18. Check:
  `cargo test --all-features --lib the_tab_bar_is_the_second_interior_row` → passes at HEAD
  against the bar's current row, so the rewritten expectation fails.
- [ ] 4.4 GREEN: change the detail draw path to `split_detail`'s `(tabs, rule, content)`.
  Verify: 4.3 passes.
- [ ] 4.5 Verify the content area did not change **size**: it is fourteen rows before and
  after, and it **moves** from rows 4–17 to rows 5–18. The expectations live in
  `src/ui/mod.rs:843` and `:1132`, which neither `ui::view` nor `ui::layout` selects, so run
  `cargo test --all-features --lib ui::tests`. Their row indices change; their row **count**
  must not. If a count needs editing, the arithmetic is wrong; re-read design.md → D2.
- [ ] 4.6 Verify the mandated widths did not move: `bash scripts/gates/detailwidths.sh`,
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
  `src/ui/view.rs` and `src/ui/app.rs`.
  Check at HEAD: `grep -rn '"  v \|"  > ' src/ui/ | wc -l` → **28**. Verify after: the same
  command returns **0**. The quoting matters and the earlier draft got it wrong: the fixtures
  write these glyphs inside double-quoted string literals, so the backtick form
  `grep -rn '\`  v \|\`  > ' src/ui/` returns **0 at HEAD** — green before the work starts and
  green if the work is skipped.
- [ ] 5.4 REFACTOR: the glyph pair is written once, in `section_row_text`, and read from there
  by every other site — `foldable-spec-sections` → Decision 9 depends on that being true, since
  it adopts whatever pair the list region carries. Record "no refactor was needed" if none
  applies.
- [ ] 5.5 VERIFY: `cargo test --all-features --lib ui::list` and
  `cargo test --all-features --lib ui::view`, run separately — green.
- [ ] 5.6 VERIFY: `bash scripts/gates/colwidth.sh` exits 0 — the new glyphs go through
  `layout::columns` like every other cell.

## 6. The hit test
<!-- kind: behavior -->

- [ ] 6.1 RED: extend `ui::layout::tests` so `zone` returns `List` for the heading and padding
  rows, `Detail` for the divider column 40, and `ListRow`/`DetailTab` at the new offsets, and
  so row 0 resolves to a region rather than `Outside`. Verify: fails at HEAD, where row 0 is
  the frame header and resolves to `Outside`.
- [ ] 6.2 GREEN: update `zone` to derive through the new `split_frame`, `interior` and
  `split_detail`, adding the divider-column branch per `responsive-layout`'s zone requirement.
- [ ] 6.3 Update the two landed `src/ui/driver.rs` assertions the new geometry falsifies:
  `:3646` expects `(10, 0)` to be `Action::Ignore` where row 0 is now the list region's
  heading, and `:4303` expects the tab bar at `y == 3` where it becomes 2. Neither file was
  named in the earlier task list.
- [ ] 6.4 Verify the drawn/hit-test agreement still holds at both widths:
  `cargo test --all-features --lib the_hit_test_agrees_with_the_drawn_buffer` and
  `cargo test --all-features --lib every_drawn_row_is_reported_by_row_at`, run separately.

## 7. Scenarios with no owning task above
<!-- kind: behavior -->

Four scenarios added by the deltas were verified by no task, and three were tasked at the
wrong tier. See design.md → Verification matrix for the test name each must take.

- [ ] 7.1 RED then GREEN: `responsive-layout` → "A name longer than the heading row keeps its
  tail" and "No repository names itself in the heading" — the heading row's own degraded
  states, neither reachable through task 3.1's happy path.
- [ ] 7.2 RED then GREEN: `responsive-layout` → "The routed region's heading is bold and the
  other's is dim". Task 4.1 asserts this for the **detail** heading only; the list heading's
  own bold/dim pair has no task, and the two are drawn by different call sites.
- [ ] 7.3 RED then GREEN: `responsive-layout` → "A one-, two-, and three-column frame
  degenerates without drawing over a gutter".
- [ ] 7.4 RED then GREEN: the drawn-buffer half of "Body and footer occupy their rows at both
  widths", "A one-row frame renders the body's heading row and nothing else", and "A two-row
  frame renders one body row and the footer". Task 1.3 asserts `split_frame`'s tuple only; the
  render half includes rewriting `src/ui/view.rs:944 one_row_frame_draws_header_only`, whose
  name is falsified by this change.

## 8. Documentation sites
<!-- kind: operational -->

Only the first of these fails a test when it goes stale. The other three drift in silence,
which is why each is a named line here rather than a note — see design.md → Test Strategy.

- [ ] 8.1 CHECK: record which documentation claims this change actually falsifies.
  `cargo test --all-features --test doc_contract --test degraded_coverage` → **10 and 61
  passed at HEAD**, so neither is red yet, and `degraded_coverage` is the one that will go.
  Note that the earlier task list named `doc_contract`'s **terminal-seam names** leg: this
  change adds and removes no crossterm terminal-mode function, so that leg has no subject
  here and must not be edited.
- [ ] 8.2 CHANGE: `SPEC.md` — the frame description (audience: this repository's future
  changes); the degraded-states row for the `file mode` badge, which says the badge sits
  "immediately after the `OpenSpec` label" that this change deletes (`:887`); the mouse
  table's ignored-gesture row, which lists "the header row" and "a border" (`:562`); and
  § List view's three example blocks and its sentence "its glyph is `v` when open and `>`
  when collapsed" (`:388-407`). Net change is a rewrite of existing text, not an addition.
- [ ] 8.3 CHANGE: `AGENTS.md` — the mandated-width paragraph's justification (the widths
  themselves do not move, but they are now gutters rather than borders), the pure-view-set
  description, and the current-repo-state paragraph.
- [ ] 8.4 CHANGE: the `## Purpose` of `openspec/specs/responsive-layout`, `detail-header`,
  `artifact-tabs` and `view-palette` (audience: `openspec validate --specs --strict` and every
  future reader). Each states something this change falsifies — a bordered body and an
  emphasised border, a `bold` detail header, the tab bar as the interior's *second* row via a
  `header/tab-bar/content` split, and the removed `HeaderTitle`/`DetailHeader` roles.
  `tests/spec_purposes.rs` checks only that a Purpose exists and is not the archive
  placeholder, so nothing catches these; a delta carries requirements rather than a Purpose,
  so archiving cannot fix them either.
- [ ] 8.5 CHECK: `README.md` needs no edit — no documented key or binding changed. Confirm
  rather than assume: `cargo test --all-features --test manifest`.
- [ ] 8.6 VERIFY: `cargo test --all-features --test doc_contract --test degraded_coverage`
  passes, and `grep -rn '  v \|  > ' SPEC.md` returns nothing.

## 9. Change Review
<!-- kind: operational -->

- [ ] 9.1 CHECK: assemble the review input — this change's artifacts and the full diff against
  `0437e05`.
- [ ] 9.2 CHANGE: dispatch the outside-in TDD reviewer. It must be **a fresh agent, not a fork
  of the implementing session**: a fork inherits the assumptions under audit and will confirm
  them.
- [ ] 9.3 VERIFY: every finding is resolved in the artifact that owns it, or declined with a
  reason recorded. Implementation-review findings go in the change's own review record, not
  into `planning-review.md`, which is the log of the review that ran *before* implementation.

## 10. Final verification
<!-- kind: operational -->

- [ ] 10.1 CHECK: confirm the affected tiers are the four design.md → Test Strategy names, and
  that no test added by this change enters raw mode, spawns a process, or calls
  `ui::read_artifact`.
- [ ] 10.2 VERIFY: **the verification matrix is real.** Run every command in design.md →
  Verification matrix and require each to run at least one test. A `--lib` filter matching
  nothing exits **0** with `0 passed`, so a mis-typed or never-written test name is otherwise
  indistinguishable from a passing one. This check is what makes that table evidence:

  ```sh
  grep -oE 'cargo test --all-features --lib [a-z0-9_]+' openspec/changes/pane-chrome/design.md \
    | sed 's/.*--lib //' | sort -u | while read -r t; do
      n=$(cargo test --all-features --lib "$t" 2>/dev/null | grep -oE '[0-9]+ passed' | head -1)
      [ "${n%% *}" -ge 1 ] 2>/dev/null || echo "MATCHES NOTHING: $t"
    done
  ```

  Expected output: empty. Every name printed is a row of the matrix that proves nothing.
- [ ] 10.3 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 10.4 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 10.5 VERIFY: `make gates` — every gate OK. `OPENSPEC-UNTOUCHED` requires this change's
  artifacts committed first; if it still fails, the failure must name a file this change did
  not create.
- [ ] 10.6 VERIFY: `cargo test --all-features` on an idle machine. A failure inside
  `ui::tests::wiring` carrying the empty-call-log deadline signature is the known flake
  recorded in the baseline above; re-run it with `--test-threads=1` to confirm, and record
  which it was. A failure anywhere else, or one in that module with a non-empty call list, is
  this change's.
- [ ] 10.7 VERIFY: `cargo llvm-cov --fail-under-lines 80`, and the production-slice floor from
  the same run — this change adds view-layer code, which is the gated slice.
- [ ] 10.8 VERIFY: the plugin's own writes are unchanged. `git status --short openspec/` cannot
  attribute this on its own: two **other** in-flight sessions have untracked directories under
  `openspec/changes/`, and deleting them is not the fix. Use the gate that has a subject and a
  planted control instead — `bash scripts/gates/readonly-ui.sh` exits 0, and
  `cargo test --all-features --test gate_controls` proves it can fail.
- [ ] 10.9 VERIFY: `make check` as the single gate. If it fails, name the failing sub-command
  (`fmt`, `clippy`, `gates`, `test`, or `coverage`) and fix that, not the gate.
- [ ] 10.10 VERIFY: `openspec validate pane-chrome --strict`.
- [ ] 10.11 Build and look at it: `make build`, then open the pane in a Herdr split at 60
  columns and in a dedicated tab, and compare against the prototype named in design.md.
