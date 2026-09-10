<!-- Planning-time measurements. Every number below names the command that produced it.
     Originally taken at HEAD 027db45 on 2026-09-09 and RE-MEASURED at HEAD 08025d3 on
     2026-09-10, after `pane-chrome` landed and was archived. Where the two disagree the
     re-measured value stands and the earlier one is shown struck through, so a reader can
     see which numbers moved. See planning-review.md -> Reviewed Against. -->

## Where this stands

**Planning is refreshed and implementation has not started.** Every task below is unchecked
and that is accurate: the session that refreshed this plan stopped at the weekly usage limit
before group 0. Nothing in `src/` has been touched by this change.

What was done, in commits `f76be82`..`c571d35`: the pre-implementation drift refresh against
HEAD `08025d3`, after `pane-chrome` landed past the planning review's `c9820c6`. Five spec
deltas were re-based so archiving this change cannot revert `pane-chrome`, all 117 matrix
filters were repaired, and every baseline below was re-measured. See planning-review.md →
Drift Repair at HEAD `08025d3` for both findings and their verification.

**Resume at task 0.1.** No refresh is owed unless HEAD has moved again — check
planning-review.md → Reviewed Against against `git log` first, as the apply flow requires.

## Baseline re-measured at HEAD `08025d3`

| Check | Command | Result at HEAD |
|---|---|---|
| Spec scenarios in this change | `grep -rc '^#### Scenario:' openspec/changes/foldable-spec-sections/specs/*/spec.md \| awk -F: '{s+=$2} END {print s}'` | **122** (artifact-content 28, artifact-folds 18, detail-scroll 29, list-selection 15, mouse-input 13, responsive-layout 8, view-palette 11) |
| `Detail { … }` literal/pattern spans | `grep -rn 'Detail {' src/ \| grep -v 'struct Detail' \| wc -l` | **61** (was 60 at `027db45`) — every one must name the new field |
| `.source` references under `src/ui/` | `grep -rc '\.source\b' src/ui/*.rs \| grep -v ':0'` | app.rs 31, detail.rs 7, driver.rs 6, view.rs **4** (was 5), mod.rs 3, tasks.rs 1 — **52** in all, was 53 |
| Fold glyph pair in `src/ui/list.rs`'s `section_row_text` | `grep -n "if collapsed" src/ui/list.rs` | **`▸` collapsed, `▾` open** — `pane-chrome` D6 landed, so Decision 9's conditional is resolved and this change ships that pair |
| `Zone` variant count | `sed -n '/pub enum Zone/,/^}/p' src/ui/layout.rs` | **five** — unchanged by `pane-chrome`, so group 8 still adds the sixth |
| Roles carrying no modifier | `src/ui/palette.rs`'s own table test | **seven**, not nine — `pane-chrome` removed `HeaderPath` and `RegionBorder` from that list. Task 1.1 asserts seven |
| Pure view file counts | `/bin/sh scripts/gates/noio-view.sh; /bin/sh scripts/gates/colwidth.sh` | **nine** and **eight**, unchanged |
| R1 — the palette roles do not exist | `grep -rn 'Role::DetailSection' src/` | **exit 1 — RED** |
| R2 — `section_at` does not exist | `grep -rn 'fn section_at' src/ui/detail.rs` | **exit 1 — RED** |
| R3 — the click targets do not exist | `grep -rn 'Target::DetailLine\|Target::DetailHeader' src/` | **exit 1 — RED** |
| R4 — the fold field does not exist | `grep -n 'pub expanded' src/ui/app.rs` | **exit 1 — RED** |
| R5 — `Space` is route-blind today | `grep -n 'Action::ToggleSection => self.apply_toggle_section(),' src/ui/app.rs` | **exit 0**, still line 433 at `08025d3` — the unconditional arm group **7** replaces |
| Existing `nodefault-ui.sh` recipe lines | `grep -n SCAN_MIN Makefile` | lines **45–49**, five of them; group 4 adds a sixth |
| Matrix filters selecting zero tests | see design.md -> Test Strategy | **0 of 117** after the refresh; was about 37 before it |
| Hygiene gates | `make gates` | **exit 0** at `08025d3` with the refreshed artifacts committed: 53 `OK` lines, no failures. (At `027db45` it exited 2 on `OPENSPEC-UNTOUCHED` alone, naming this change's own then-untracked artifacts; committing them cleared it, as recorded.) |
| Library test count | `cargo test --lib -- --list` | **1224** at `08025d3` (was 1222 at `027db45`; `pane-chrome` added two). Not re-run as a full `cargo test --all-features` during the refresh — task 12.5 is where the whole suite is measured on an idle machine. |

**The `ui::tests::wiring` load flake, already diagnosed at HEAD.** `openspec/changes/markdown-legibility/design.md` (commit `027db45`) records the mechanism and the measurement, and this plan does not restate them: `crate::testutil::Stages` (`src/lib.rs:660-724`) bounds those 29 tests with a five-second wall-clock deadline and force-presses `q` on expiry, so a machine under load cuts a run short before the staged keys fire. **The signature separates a flake from a regression:** a deadline flake asserts an expected count against `0` with an *empty* log (`left: 0 / right: 4`, `calls: []`); a genuine defect produces a wrong call *list*. This change touches no file in that module's subject — no `HerdrCli`, no launcher, no poller — so a failure there with that signature is not this change's, and a failure anywhere else is. Re-measure on an idle machine before treating any of it as a regression.

**Sequencing.** `pane-chrome` has landed and is archived, which resolves the one scheduling
question planning-review.md left to the user: this change is additive onto its new shapes and
inherits its glyph decision (Decision 9). No sibling change is in flight against
`src/ui/layout.rs` or `src/ui/palette.rs`.

Groups 2, 3, 7, and 8 all write `src/ui/app.rs`, and groups 5 and 6 depend on
the types those groups add, so the chain 2 → 3 → 5 → 6 → 7 → 8 is genuinely sequential — one
file is shared mutable state, and a whole-crate `cargo test` cannot attribute a compile
failure between two agents editing it. Group 4 is an operational gate edit that depends only on `ArtifactSection` existing (group 2).
Only group 1 (`src/ui/palette.rs`, two enum variants and two match arms, needed by group 5
and by nothing before it) qualifies for `parallel-after: 0`.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

Taken, per design.md → Test Strategy: two scenarios are true only of the assembled loop —
a fold must read no file, and the per-frame clamp must leave the cursor on the last *line*
rather than the last *screenful*. Both are about what happens between a keypress and the next
frame, which no unit test sees.

- [ ] 0.1 Build the loop fixture in `src/ui/driver.rs`'s test module from `changes::fixture` values plus a recording reader, the inert `FsEvents`/`Refresher`/`AgentPoll`/`Launcher` stubs the module already uses, and a scripted event source. **No `ScratchDir`** — `grep -n ScratchDir src/ui/driver.rs` returns nothing today, and design.md → Test Boundaries grants a real filesystem only to the `ui::load` startup scenarios.
- [ ] 0.2 RED: Write `a_fold_reads_no_file` (artifact-folds) — script `Enter`, four `Char(' ')`, `q` at 120x40; assert the reader recorded one call per resolved path and none during the folds, and that the final buffer shows the resulting fold state.
- [ ] 0.3 RED: Write `a_foldable_tab_is_clamped_to_its_last_line` (detail-scroll) — script ten `Char('j')` then `q` at 120x40 and 60x40; assert `detail.scroll == 2` and that the third header row carries the selected style.
- [ ] 0.4 Confirm both fail because the behavior is missing, not because the fixture is wrong: the same fixture with the script reduced to `q` must draw one frame and return `Ok`.

## 1. Palette roles
<!-- kind: behavior -->

- [x] 1.1 RED: Extend `src/ui/palette.rs`'s three existing table tests for the two new roles — view-palette :: `The palette answers every role with a Style` (including the `DetailSectionSelected` inequality assertion), `Each role's modifier set is exactly the table above` (`BOLD | REVERSED`, not `BOLD` alone; the no-modifier count stays **seven** — `pane-chrome` removed `HeaderPath` and `RegionBorder` from that list, and the two new roles both carry a modifier), and `The coloured set is exactly the table above` (both report `fg: None`, `bg: None`). Place both variants after `TabInactive`, per the delta's own enum.
- [x] 1.2 GREEN: Add both variants to `Role` and both arms to `style` (design.md → Decision 11).
- [x] 1.3 VERIFY: `cargo test --lib ui::palette` green; `/bin/sh scripts/gates/palette.sh` OK — its vacuity leg must still find `Color` in this file; and view-palette :: `The palette module reaches no I/O and measures no width` still reports nine and eight. State that no refactor was needed, or name the one performed.

## 2. `ArtifactSection`, `Detail.sections`, and `sync_detail`
<!-- kind: behavior -->

- [ ] 2.1 RED: Write the label-rule tests for artifact-folds :: `The label derivation is total over adversarial paths` — the six paths the scenario names, asserting `a`, `b`, `notes.md`, `spec.md`, `spec.md`, and the empty string.
- [ ] 2.2 RED: Write artifact-folds :: `The three spec files of a change become three labelled sections`, `A single-file artifact is one section and is not foldable`, `An artifact with no resolved paths has no sections`, and `An unreadable file drops its section and keeps its siblings`.
- [ ] 2.3 GREEN: Add `pub struct ArtifactSection { pub label: String, pub text: String }` to `src/ui/app.rs` and the label function, per design.md → Decision 5.
- [ ] 2.4 GREEN: Replace `Detail.source: String` with `sections: Vec<ArtifactSection>` and rewrite `sync_detail` step 4 to push one section per successful read, with no separator inserted (artifact-content :: `A multi-file artifact is concatenated in path order with a separating newline`, whose body this change rewrites).
- [ ] 2.5 GREEN: Update the 61 `Detail { … }` spans the baseline counted and the `.source` references it counted — 31 in `app.rs` and 21 outside it, 52 across `src/ui/`. Re-run the two counting commands afterwards; both must report the new field and no `source`.
- [ ] 2.6 RED→GREEN: Re-assert on `sections` the five `sync_detail` scenarios that need no `expanded` — artifact-content :: `The selected tab's file is read once and reused`, `Switching the tab re-reads, and so does switching the change`, `Two changes with the same name are distinguished by directory`, `A multi-file artifact is concatenated in path order with a separating newline` (rewritten per the spec's new body), and `An unreadable file names its reason and does not lose its siblings` (extended with the not-foldable assertion). The four scenarios that assert `expanded` belong to group 3, which is where that field exists.
- [ ] 2.7 Run the group tests — no regressions, and state that no refactor was needed or name the one performed. Group 2 reshapes `Detail` across six files, so this is the least plausible silent case.

## 3. `Detail.expanded` and the fold reset
<!-- kind: behavior -->

- [ ] 3.1 RED: Write artifact-folds :: `A tab move forgets the fold, a forced reload does not` — three dashboards, one per path (tab move, forced reload, adopted refresh).
- [ ] 3.2 GREEN: Add `expanded: std::collections::BTreeSet<usize>` to `Detail`, clear it in `sync_detail` on exactly the condition that resets `detail.scroll`, and leave it untouched in `adopt` (design.md → Decision 4).
- [ ] 3.3 GREEN: Update the same 61 spans for the second new field, and extend the four scenarios that assert `expanded` — artifact-content :: `A forced reload re-reads the same key and keeps the scroll`, `A tab move under a forced reload still resets the scroll`, `An artifact with no resolved paths reads nothing at all`, and `An empty visible list clears the detail` — plus detail-scroll :: `Startup leaves the detail empty and unscrolled`.
- [ ] 3.4 CHECK: Persistence gate — confirm no migration, backfill, cache invalidation, or index rebuild applies (design.md → Persistence and Rollout), and that the plugin's writes are still exactly `agent-names.toml`. Run `/bin/sh scripts/gates/readonly-ui.sh`, then plant a `std::fs::write` in `src/ui/app.rs`, confirm it exits non-zero, remove it, confirm it goes quiet. An unchanged `grep -c 'agent-names'` would not catch a *new* write and is not the check.
- [ ] 3.5 Run the group tests — no regressions, and state that no refactor was needed or name the one performed.

## 4. The `NODEFAULT-UI` gate learns `ArtifactSection`
<!-- kind: operational -->

Split out of group 2 because it edits build config, which this schema classes as operational
and which may not share a group with behavior work. It runs **after** group 3 for a second
reason, measured at planning time: the gate reports `half B found only 0 literal/pattern
spans - the scan is vacuous` until real `ArtifactSection { … }` literals exist, so the floor
cannot be measured before the construction sites are written.

**Why the type is not called `Section`.** Measured against a scratch copy of `src/` and
`scripts/`: `SCAN_MIN=1 TYPES='Section'` exits **1** with nine false hits — `src/ui/driver.rs:231`,
`src/ui/list.rs:721,731,1755,1795,1813,1919`, `src/ui/view.rs:214,244` — because half B's
pattern is `(?<![A-Za-z0-9_])Section\s*\{` and `:` is not a word character, so
`list::RowKind::Section {` matches. `TYPES='ArtifactSection'` matches none of them. The name
is load-bearing, not cosmetic.

- [ ] 4.1 CHECK: Run `SCAN_MIN=1 TYPES='ArtifactSection' /bin/sh scripts/gates/nodefault-ui.sh` and record the span count it reports. `SCAN_MIN` is per invocation, so `ArtifactSection` gets its own floor rather than hiding inside line 45's 206 (measured at `08025d3`: lines 45–49 carry 206, 53, 81, 111, 26).
- [ ] 4.2 CHANGE: Add a **sixth** `nodefault-ui.sh` line to the `Makefile`'s `gates:` recipe — `SCAN_MIN=<4.1's count> TYPES='ArtifactSection'` — beside the five that already carry their own floors.
- [ ] 4.3 VERIFY: Plant `impl Default for Section` in `src/ui/app.rs`, confirm the new line exits non-zero naming `ArtifactSection`, remove it, confirm it goes quiet. A separate line is what makes this falsifiable — folded into line 45, a `ArtifactSection` scan matching zero spans would still have printed OK under that line's larger floor.
- [ ] 4.4 VERIFY: Record the plant in `tests/gate-controls.toml` if the existing `nodefault-ui` control does not already cover the `ArtifactSection` leg, and confirm `cargo test --test gate_controls` green.

## 5. `content_lines` header rows, bodies, and `section_at`
<!-- kind: behavior -->

- [ ] 5.1 RED: Write the header-row grammar tests at the two mandated interiors, 78 and 58 — artifact-folds :: `A narrow pane truncates the label and keeps the glyph`, plus the 0..=20 width sweep the scenario names.
- [ ] 5.2 RED: Write artifact-content :: `A foldable tab's body is headers, and an open section's markdown beneath its own`, `A non-foldable tab is byte-identical to today`, and `The tracked-tasks tab concatenates rather than folding`.
- [ ] 5.3 RED: Write artifact-folds :: `Each drawn header row resolves to its own index` and `Resolution is total and inert where it should be`.
- [ ] 5.4 GREEN: Change `content_lines`' return type to `Vec<ContentRow>` — a `markdown::Line` plus a `ContentKind` of `Problem`, `Body`, or `SectionHeader { section, selected }` — and update its two production callers, `ui::view::render`'s detail draw and `Dashboard::normalise_scroll` (which needs only `.len()`), per design.md → Decision 12. `ui::detail` names no `Role` and no `ratatui` type; confirm with `grep -n 'Role::' src/ui/detail.rs` returning nothing.
- [ ] 5.5 GREEN: Emit header rows and per-section bodies, gated on `sections.len() > 1`, with the tracked-tasks branch concatenating (design.md → Decisions 3 and 8). The glyph pair is **whatever `src/ui/list.rs`'s `section_row_text` uses**, per Decision 9 — `pane-chrome` landed, so that is `▸` collapsed and `▾` open. Read it from that function rather than writing a literal, and assert the two agree rather than asserting a character.
- [ ] 5.6 GREEN: Implement `ui::detail::section_at(rows, offset, row)` as a **lookup** into the caller's own row list (design.md → Decision 12). It takes **no** `Rect` — `scripts/gates/notabseam.sh` greps this whole file for one — and `offset + row` saturates.
- [ ] 5.7 GREEN: Extend the two existing width properties — artifact-content :: `content_lines` is total and width-parameterised (seven `Detail` values to nine) and `No content_lines line exceeds its width at any width` (the 0..=130 sweep over the foldable values).
- [ ] 5.8 CHECK: `/bin/sh scripts/gates/notabseam.sh`, `/bin/sh scripts/gates/colwidth.sh`, and `/bin/sh scripts/gates/noio-view.sh` OK — no `.chars()` measurement and no I/O added to a pure view file. Plant `label.chars().count()` in `src/ui/detail.rs`, confirm `COLWIDTH` exits non-zero, remove it, confirm it goes quiet.
- [ ] 5.9 CHECK: `/bin/sh scripts/gates/detailwidths.sh` OK — every new test in `ui::detail` names both 58 and 78, which is what the gate counts.
- [ ] 5.10 Run the group tests — no regressions, and state that no refactor was needed or name the one performed.

## 6. The drawn slice and the header styling
<!-- kind: behavior -->

- [ ] 6.1 RED: Write view-palette :: `A section header's role is selected by its kind, not by its face`. The selected row is asserted against `palette::style`; the unselected rows against their `kind`, since plain `BOLD` cannot discriminate them from `Role::Strong`.
- [ ] 6.2 RED: Write artifact-folds :: `The specs tab opens as a list of capability names`, `Folding one section shows its body and leaves its siblings shut`, `The cursor's section header is the emphasised one`, and `An index past the end folds shut rather than panicking`, all at 120x20 and 60x20.
- [ ] 6.3 RED: Write the four clauses the extended scenarios gain — artifact-content :: `A read failure is named above the content at both widths`'s no-header-row clause and `A wide-character document stays inside the detail region`'s CJK-labelled foldable case, and view-palette :: `A monochrome reading of the frame is unchanged`'s exactly-one-`REVERSED`-row leg and `Faces reach the buffer as coloured styles at both mandated widths`'s no-`REVERSED` clause.
- [ ] 6.4 GREEN: In `ui::view::render`, choose `layout::viewport` for a foldable artifact and `layout::scroll_offset` otherwise (design.md → Decision 2), and map each row's `ContentKind` to `DetailSectionSelected`, `DetailSection`, or no role, patching it over `style_for(&segment.face)` (Decisions 10 and 12). This is the only place the mapping lives, beside the `RowKind` → `Role` match already in this file.
- [ ] 6.5 GREEN: Re-assert on `sections` every existing view scenario the matrix marks so — sixteen rows across `artifact-content`, `detail-scroll`, and `responsive-layout`, listed in design.md → Test Strategy. Mechanical field renames, not new assertions, which is why they sit here and 6.1–6.3's new clauses do not.
- [ ] 6.6 CHECK: `/bin/sh scripts/gates/palette.sh` OK — every style assertion compares against `palette::style(role)` and no colour literal appears outside `src/ui/palette.rs`'s own tests.
- [ ] 6.7 Run the group tests — no regressions, and state that no refactor was needed or name the one performed.

## 7. `Space` becomes route-dependent, and the clamp branches
<!-- kind: behavior -->

- [ ] 7.1 RED: Write artifact-folds :: `Space opens the section under the cursor and leaves its siblings shut`, `Space inside an open section folds it and moves the cursor to its header`, `Space on a problem row is inert`, and `Space is inert on a non-foldable artifact`.
- [ ] 7.2 RED: Write list-selection :: `Space at the detail route leaves the list alone` and `Space at the detail route is inert on a non-foldable artifact`; pin the four existing list-route `Space` scenarios to `Route::List`; and pin mouse-input :: `A click on a section header folds it exactly as Space does`, whose `ToggleSection` comparison goes red at 7.8 unless its route is pinned here.
- [ ] 7.3 RED: Write detail-scroll :: `At a collapsed foldable tab the same keys walk the section list` and `The wheel moves the cursor at a foldable tab`; extend `A resize renormalises the offset on the next frame`, `Scrolling stops at the top`, and `Next at the detail route and ScrollDown are the same move` with their foldable legs; and write `Every route move resets the scroll`'s fourth dashboard, proving `expanded` survives a route move.
- [ ] 7.4 GREEN: Replace `src/ui/app.rs:433`'s unconditional arm (baseline check R5) with a `match self.route`, and implement the detail-route toggle including its cursor-to-header rule.
- [ ] 7.5 GREEN: Branch `normalise_scroll` between `scroll.min(lines - 1)` and `layout::scroll_offset` on foldability, and guard the three route-move arms so none clears `detail.expanded`.
- [ ] 7.6 CHECK: Contract gate — re-read `SPEC.md`'s key-binding table and confirm the documented meaning of `Space` at each route matches `apply`. **No computable second site exists**: `tests/doc_contract.rs`'s one binding leg, `mouse_bindings_match_spec_md`, locates its table by `| Gesture | Action |` and its own parser control asserts `Err` on a `| Key | Action |` table, which is exactly SPEC.md's key table's shape. The behaviour itself is proved by `ui::app::tests::…toggle_route`; this task is a read.
- [ ] 7.7 CHECK: `/bin/sh scripts/gates/noblock.sh` OK — the toggle reaches no collaborator, spawns nothing, and reads no clock.
- [ ] 7.8 Run the group tests — no regressions, and state that no refactor was needed or name the one performed.

## 8. Clicking a section
<!-- kind: behavior -->

- [ ] 8.1 RED: Write mouse-input :: `A click on an artifact-section header folds it exactly as Space does`, `A click in an open section's body moves the detail cursor and folds nothing`, and `A click on a non-foldable tab's content is inert` — each at **120x40 and 60x40**, per `openspec/config.yaml` → `rules.tasks`, since the narrow one-region layout is exactly the geometry `Zone`'s split changes.
- [ ] 8.2 RED: Extend mouse-input :: `Clicks that address nothing are inert` with a press below the last drawn line, and `The other buttons and the non-press kinds are inert` with a press on a header row.
- [ ] 8.3 RED: Extend `ui::layout::zone`'s four existing tests for the sixth variant — responsive-layout :: `The zones tile the frame at 120 columns` (the first and last content rows, and `DetailRow.content` derived independently), `Below the breakpoint only the routed region has zones` (the narrow `DetailRow` leg), `Degenerate frames resolve without panicking` (no zero-sized `DetailRow`), and `The hit test agrees with what was drawn` (`DetailRow` cells checked against `content_lines`' output at the drawn offset).
- [ ] 8.4 GREEN: Add `Zone::DetailRow { content, row }` and narrow `Zone::Detail` by the content area in `src/ui/layout.rs`, mirroring `Zone::ListRow { interior, row }`.
- [ ] 8.5 GREEN: Add `Target::DetailLine(usize)` and `Target::DetailHeader { line, section }` and resolve them in `mouse_action`.
- [ ] 8.6 GREEN: Restructure `apply_click` so the `targets()` membership guard applies only in the `ArtifactSection` and `Change` arms (design.md → Decision 7). Left at the top of the function it returns before either new arm is reached and every detail click is silently inert — 8.1's tests are what catch that. Route the header arm through the very code the detail-route `ToggleSection` runs.
- [ ] 8.7 CHECK: `mouse_action`'s existing totality test (`Resolution is total over adversarial geometry`) still passes with the new zone — it sweeps every kind at seven columns, five areas, and both routes, so a new variant that panics is caught there.
- [ ] 8.8 CHECK: Contract gate — update `SPEC.md`'s mouse-binding table and confirm `mouse_bindings_match_spec_md` agrees. Its subject is the backticked `Action::` variant set, not the gesture rows, so it cannot fail for this change's new rows (both resolve to `Action::Click` / `Action::Ignore`, already documented): delete one documented row, confirm it fires, restore it, and record both halves — otherwise the check is believed and guards nothing.
- [ ] 8.9 Run the group tests — no regressions, and state that no refactor was needed or name the one performed.

## 9. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 9.1 VERIFY: Confirm `a_fold_reads_no_file` and `a_foldable_tab_is_clamped_to_its_last_line` pass end to end at both sizes.
- [ ] 9.2 REFACTOR: Fold the group-0 fixture into the module's existing loop-test helpers if it duplicates one; otherwise state that it does not. Then re-run `cargo test --lib ui::driver::tests` and confirm both acceptance tests are still green.

## 10. Change Review
<!-- kind: operational -->

- [ ] 10.1 CHECK: Dispatch `outside-in-tdd-reviewer` — a fresh agent, not a fork of the implementing session — against proposal.md, all **seven** spec files, design.md, tasks.md, and the diff. Point it first at: whether any of the **122** scenarios has a test that could not go red; whether `sections.len() > 1` is asked in one place or several; and whether the two `Detail` fields are named at all **61** spans.
- [ ] 10.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 10.3 VERIFY: Confirm no blocking or unowned finding remains.

## 11. Documentation
<!-- kind: operational -->

- [ ] 11.1 Rewrite in `SPEC.md`: the `Detail` field list and the key-binding table's `Space` row (audience: this repository's future changes) — `Detail` carries sections and a fold set, and `Space` is route-dependent. The mouse-binding table is task 8.7's, which must edit it in the same group or `mouse_bindings_match_spec_md` goes red at 8.8. Replaces the current text; nothing is appended beside it.
- [ ] 11.2 Rewrite in `CLAUDE.md`: the paragraph describing the detail region's content, and the one listing the pure view set's width rule (audience: every agent session). It currently says the tab's content is "read through an injected reader … and resolved once per `(change directory, tab)`" with no mention of sections; correct it to name the per-file sections and the one derived foldability predicate, and delete the now-false claim that `j`/`k` "scroll the detail content" unconditionally. Net change must be near zero — this corrects two existing sentences rather than adding a third.
- [ ] 11.3 Add one row to `openspec/IMPLEMENTATION-ORDER.md`'s Phase 6 table, marking this change unplanned post-roadmap work as `doc-conformance` is (audience: whoever reads the roadmap next), so the roadmap does not silently stop recording what shipped.

- [ ] 11.4 Rename the deleted field in the twelve spec sites this change does not otherwise touch (audience: every future reader of those specs) — `openspec/specs/tasks-checklist/spec.md` 42/58/69/230/238/272/283, `tasks-progress-bar` 84/201, and `live-updates` 244 describe `detail.source`, which will not exist. Their behaviour is unchanged, so this is a prose rename rather than a delta: carrying five behaviourally-identical requirements as MODIFIED blocks would be several hundred lines of copy for a field name. Verify with `grep -rn 'detail\.source' openspec/specs/` returning nothing.
- [ ] 11.5 Write `## Purpose` for the new `artifact-folds` capability and rewrite three now-false ones (audience: `openspec validate --specs --strict` and every future reader). `openspec archive` writes the `TBD - created by archiving change` placeholder that `tests/spec_purposes.rs` rejects, and a delta carries requirements rather than a Purpose, so archiving cannot fix these on its own. The three to correct: `openspec/specs/detail-scroll/spec.md` (it names `layout::scroll_offset` as the drawn-window derivation and `j`/`k` as moving it by a line at the detail route — both now conditional on foldability), `openspec/specs/artifact-content/spec.md` (it says the reader concatenates multiple paths in resolution order), and `openspec/specs/mouse-input/spec.md` (its click list omits the detail content area). Follow `archive/2026-09-09-list-sections/tasks.md` 9.6's shape.

## 12. Lint & Verify
<!-- kind: operational -->

- [ ] 12.1 CHECK: Confirm the affected tiers are the four design.md → Test Strategy names, and that no test added in this change enters raw mode, spawns a process, or calls `ui::read_artifact`.
- [ ] 12.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 12.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 12.4 VERIFY: `make gates` — every gate OK. `OPENSPEC-UNTOUCHED` requires the change's artifacts committed first; if it still fails, the failure must name a file this change did not create.
- [ ] 12.5 VERIFY: `cargo test --all-features` on an idle machine — green. A failure inside `ui::tests::wiring` carrying the empty-log deadline signature is the known load flake; re-run it alone to confirm, and record which it was. A failure anywhere else, or one inside that module with a non-empty call list, is this change's.
- [ ] 12.6 VERIFY: `cargo llvm-cov --fail-under-lines 80`, and the production-slice floor from the same run — the change adds view-layer code, which is the gated slice.
- [ ] 12.7 VERIFY: `make check` as the single gate; if it fails, name the failing sub-command.
- [ ] 12.8 VERIFY: `openspec validate foldable-spec-sections --strict`.
- [ ] 12.9 VERIFY: Every `cargo test --lib` filter in design.md → Test Strategy selects at least one test. Extract them and check each against `cargo test --lib -- --list`; **zero matches is a failure**, because a filter that matches nothing exits `0` and reads as a passing verification. This is the check whose absence let about thirty-seven dead filters survive a planning review that recorded the defect class as repaired (design.md → Test Strategy, "How to read the Command column"). Report the count checked and the count matched.
