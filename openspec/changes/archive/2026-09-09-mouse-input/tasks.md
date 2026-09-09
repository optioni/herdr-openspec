<!-- Planning-time baseline: commit `a156f9a` on `main`, working tree clean except this
     change's own then-untracked artifacts. They have since been committed (`65924be`,
     `702bf9c`), so `git status --porcelain` is now empty; `src/`, `scripts/`, `Makefile`,
     `SPEC.md`, `AGENTS.md`, `tests/` and `openspec/specs/` are byte-identical between
     `a156f9a` and HEAD, so every measurement below holds at both. -->

**Known-flaky at HEAD, not caused by this change.** `cargo test --all-features` at `a156f9a`
failed twice in three runs — `ui::tests::wiring::g_focuses_the_agent_the_launch_started`
(both runs) and `ui::tests::wiring::an_unreachable_scratch_herdr_is_a_standalone_tui` (one
run) — while `cargo test --all-features --lib ui::tests::wiring:: -- --test-threads=1`
passed **27/27**. The two tests spawn scratch `herdr` programs and race under parallel
execution. Group 13 re-runs the suite; a failure confined to those two names is this
pre-existing flake, not a regression, and is a separate bug fix outside this change.

**Groups 1 through 8 are sequential, and none is marked `parallel-after`.** This repository
does dispatch code groups in parallel where the three criteria hold
(`archive/2026-09-08-seam-resilience/tasks.md` groups 5, 6 and 7), so the sequential answer
here is a finding, not a default.

File overlap orders two pairs directly: group 1 and group 7 both write `src/ui/mod.rs`
(`grep -c 'impl TerminalOps for' src/ui/mod.rs` → `1`, the test `Recorder`), and groups 0
and 6 both write `src/ui/driver.rs`. Groups 2, 3, 4 and 5 do write disjoint files.

What orders the rest is criterion 2, **a dependency between them** — not criterion 3, and
not the vaguer "everything shares a compile". Every one of groups 2, 3 and 4 compiles
against `src/ui/app.rs`, which is exactly what group 5 rewrites:
`grep -n 'ui::app' src/ui/layout.rs src/ui/detail.rs src/ui/list.rs` reports **24** sites —
`layout.rs:10 use crate::ui::app::Route`, `list.rs:6 use crate::ui::app::{Dashboard,
SectionKey, Target, matches}`, `detail.rs:193 &crate::ui::app::Detail`, and their test
modules' `Dashboard` constructions, every one of which names all fourteen fields with no
`..` rest. Group 5 adds five `Action` variants and rewrites `apply`; while that file does not
compile, none of groups 2, 3 or 4 can run its own scoped `cargo test --lib` either, because
the filter selects which tests run, not which crate is built. That is a dependency, not a
shared gate, and it holds however narrowly each group's command is scoped.

**Group 9 is marked `parallel-after: 7`.** It writes only `SPEC.md` and `AGENTS.md`; group 8
writes only `scripts/gates/*.sh` and `tests/gate-controls.toml`; neither needs the other's
output and neither touches a `.rs` file, so a failure in either stays its own. Both depend
on group 7, the last group to change behaviour they describe.

Group 9 is also placed **before** group 10 rather than after the review, against the
schema's usual ordering for a Documentation group, because group 10's two
`tests/doc_contract.rs` tests read the `SPEC.md` and `AGENTS.md` passages group 9 writes.
Group 10 is therefore ordered after 9 and carries no marker of its own. Groups 11, 12 and 13
are whole-change gates and cannot precede the work they gate.

**Planning-time checks, run at `a156f9a`:**

| Check | Command | Result at HEAD | Why that result |
|---|---|---|---|
| A (RED) | `grep -rn 'fn zone' src/ui/layout.rs` | no output, **exit 1** | `layout::zone` does not exist; group 2 turns this green |
| B (RED) | `grep -rn 'fn row_at' src/ui/list.rs` | no output, **exit 1** | `list::row_at` does not exist; group 3 turns this green |
| C (RED) | `grep -rn 'fn tab_at' src/ui/detail.rs` | no output, **exit 1** | `detail::tab_at` does not exist; group 4 turns this green |
| D (RED) | `grep -rn 'Action::Click' src tests` | no output, **exit 1** | The five new actions do not exist; group 5 turns this green |
| E (RED) | `grep -rn 'fn mouse_action' src/ui/driver.rs` | no output, **exit 1** | The resolver does not exist; group 6 turns this green |
| F (RED) | `grep -rn 'enable_mouse\|EnableMouseCapture' src tests` | no output, **exit 1** | Capture is never entered; group 1 turns this green |
| G (RED) | `grep -rn 'mouse_problem' src tests` | no output, **exit 1** | The refused-capture reason has no carrier; groups 1 and 7 turn this green |
| H (RED) | `grep -n 'EnableMouseCapture' scripts/gates/noraw-grep.sh` | no output, **exit 1** | The confinement sweep does not cover the two capture commands; group 8 turns this green |
| I (green + negative control) | `/bin/sh scripts/gates/noraw-grep.sh` | `NORAW OK: 36 files searched, mode functions only in src/ui/terminal.rs, CrosstermOps at 6 sites`, **exit 0** | Negative control run in a scratch copy of `src tests scripts`: `RAW_RE` extended with `\|EnableMouseCapture\|DisableMouseCapture` and `// EnableMouseCapture` appended to `src/ui/app.rs` with `printf '\n// EnableMouseCapture\n' >> src/ui/app.rs` (a leading blank line, so the comment lands two past the file's 5293) → `NORAW FAIL: terminal-mode function outside src/ui/terminal.rs: src/ui/app.rs:5295`, **exit 1**; plant removed → **exit 0** again |
| J (green + negative control) | `/bin/sh scripts/gates/wired.sh` | `WIRED OK: thirteen names present …`, **exit 0** | Leg 2 forbids a branch in `pub fn run()`; group 7 adds a field expression, not a branch. Negative control is already checked in: `cargo test --test gate_controls` plants a real defect per gate script |
| K (must-change) | `awk '/^pub enum Action \{/{f=1;next} f&&/^\}/{exit} f&&/^    [A-Z]/{n++} END{print n}' src/ui/app.rs` | **18** | Must be `23` after group 5 |
| L (must-change) | `grep -n '[^d]Startup {' src/ui/mod.rs \| grep -v Probed` | **6** sites — 386 (the production one, in `run`), 2724, 2776, 3065, 3690, 3840 | Every site names every field, so all 6 must name `mouse_problem` after group 7. A bare `grep -c 'Startup {'` reports **11**: five are `ProbedStartup {`, a different test-local struct at `:2755`, not construction sites of this type |
| L2 (green + negative control) | `cargo test --lib the_full_key_table` | `running 0 tests … 1158 filtered out`, **exit 0** | A `cargo test` name filter matching nothing exits 0. Every filtered command in this plan and in design.md's matrix must therefore be checked for `0 passed`, which is a failure, not a pass — three matrix rows named tests that do not exist before review caught them |
| M (must-change) | `grep -rn 'impl TerminalOps for' src tests` | **3** sites: `src/ui/terminal.rs:85`, `src/ui/terminal.rs:212`, `src/ui/mod.rs:2186` | All 3 must implement the six-method trait after group 1 |
| N (baseline) | `cargo test --all-features --lib -- --test-threads=1` | **1158 passed**, exit 0 | The suite this change must leave green |

---

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

Taken, per design.md → Test Strategy: the defect this change can most plausibly ship is a
resolver that is correct and never called, which is `live-refresh`'s own defect in a new
shape. Only `run_loop` can prove otherwise.

- [x] 0.1 Add a `MouseEvent` constructor to `driver`'s existing scripted `EventSource`
      double, honouring design.md → Test Boundaries: events, backend, artifact reader, and
      all four collaborators replaced; the terminal never constructed.
- [x] 0.2 RED: Write `driver::tests::a_mouse_event_moves_the_selection_through_the_loop`
      for `mouse-input` → "A mouse event is resolved through the loop and a key is not":
      drive `run_loop` at 120x40 with a left press on the second change row, then `q`, and
      assert the frame after the press carries the selection marker on that row.
- [x] 0.3 Confirm it fails because the behaviour is missing, not the harness: the same test
      with `Char('j')` in place of the press must pass at HEAD.
      `cargo test --lib driver::tests::a_mouse_event` — expect RED.
      **Run:** RED as written — `left: " ", right: ">"` on row 4, whose text is
      `"│  beta                             [-]│…"`, so the geometry is right and the
      selection did not move. With `press(KeyCode::Char('j'), KeyModifiers::NONE)`
      substituted for the mouse event and nothing else changed: **1 passed**. The harness
      drives `run_loop` correctly; the mouse path is what is missing.

## 1. Terminal capture lifecycle
<!-- kind: behavior -->

- [x] 1.1 RED: Write failing tests in `src/ui/terminal.rs` for: `normal_lifetime_records_all_six`,
      `mouse_failure_still_returns_a_guard`, and the updated
      `normal_lifetime_is_enter_enter_leave_disable` (filtering the capture pair out of the
      recorded list, so the four-operation claim survives verbatim). Extend the `Recorder`
      double in `src/ui/terminal.rs` and the one in `src/ui/mod.rs:2186` to the six-method
      trait. The four tests updated in place keep their existing names —
      `normal_lifetime_is_enter_enter_leave_disable` (`src/ui/terminal.rs:228`),
      `enable_raw_failure_attempts_nothing_further` (`:245`),
      `alternate_screen_failure_unwinds_raw_mode` (`:254`), and
      `teardown_errors_do_not_panic_and_both_are_attempted` (`:266`).
      `cargo test --lib terminal::` — expect RED, and expect more than 0 tests to run.
- [x] 1.2 RED: Update the five existing panic-path and refusal tests to the lists the
      `terminal-lifecycle` delta now states — `a_panic_still_restores`
      (`src/ui/terminal.rs:281`), `restore_then_restores_before_delegating` (`:300`),
      `restore_then_delegates_even_when_both_restores_fail` (`:310`),
      `a_panic_on_the_render_thread_still_restores` (`:343`), and `ui::enter_if_terminal`'s
      own `Ok` arm. All four terminal tests break the moment `disable_mouse` joins
      `restore_then` (`:74-78`), which `Drop` and the panic hook share.
      `a_panic_on_a_worker_thread_restores_nothing` (`:322`) must stay **unchanged**.
      **Correction, made while implementing:**
      `teardown_errors_do_not_panic_and_both_are_attempted` (`:266`) does *not* survive
      untouched. It would still compile and pass on its two-call slice, but the
      `terminal-lifecycle` delta reworded its own scenario to fail **all three** teardown
      operations and assert the list ends `["disable_mouse", "leave_alternate",
      "disable_raw"]`. The spec is authoritative over this note, so the test was updated to
      the three-operation form. The worker-thread test's `["previous_hook"]`
      is unchanged and must stay unchanged, which is the check that capture gained no
      exception off the render thread. `cargo test --lib terminal:: enter_if_terminal` —
      expect RED.
- [x] 1.3 GREEN: Add `enable_mouse` and `disable_mouse` to `TerminalOps`, implemented in
      `CrosstermOps` as one `ratatui::crossterm::execute!` of `EnableMouseCapture` /
      `DisableMouseCapture` each, mapping the error — no decision, no ordering, no state.
- [x] 1.4 GREEN: `TerminalGuard::enter` calls `enable_mouse` after `enter_alternate` and
      stores a failure instead of returning it; `mouse_problem(&self) -> Option<String>`
      returns the `TerminalError`'s `Display` text. `Drop` and `restore_then` call
      `disable_mouse` first, unconditionally (per design.md → Decision 7), so the panic
      hook releases capture too — `restore_then` is the shared body `Drop` and the panic hook
      both delegate to, so one edit covers both paths.
- [x] 1.5 CHECK: Contract gate — `TerminalOps` is a trait with three implementors
      (check M). `grep -rn 'impl TerminalOps for' src tests` still reports exactly those 3
      sites, each implementing all six methods, and no fourth appeared.
      **Run:** exactly 3 — `src/ui/terminal.rs:123` (`CrosstermOps`),
      `src/ui/terminal.rs:294` (that file's `Recorder`), and `src/ui/mod.rs:2186`
      (`ui::tests::start`'s `Recorder`). The line numbers moved with the edit; the set did
      not. All three compile, which is what proves each implements all six methods —
      rustc rejects a partial `impl`.
- [x] 1.6 REFACTOR: State whether the three `TerminalOps` implementors share enough to
      warrant extraction, or record that none was needed.
      **None was needed.** The three share only the trait's six signatures. `CrosstermOps`
      is six one-line crossterm calls with no state; `terminal.rs`'s `Recorder` records a
      call list and answers from a per-operation failure map; `mod.rs`'s `Recorder` records
      a call list and carries one `bool`. Extracting a shared recording double would move a
      test-only type across a module boundary to save eight lines and would couple two
      independent test modules' failure models — the second double deliberately cannot fail
      anything but `enable_raw`, which is the whole of what `enter_if_terminal`'s error arm
      needs.
- [x] 1.7 Run the group tests — `cargo test --lib terminal::` and
      `cargo test --lib ui::tests::` — no regressions.
      **Run:** `cargo test --lib terminal::` → **13 passed**, 0 failed (11 before, plus
      `normal_lifetime_records_all_six` and `mouse_failure_still_returns_a_guard`).
      `cargo test --lib ui::tests::` → 7 failed under the default parallel run, every one
      of them under `ui::tests::wiring::`; re-run as
      `cargo test --lib ui::tests:: -- --test-threads=1` → **60 passed**, 0 failed. This is
      the pre-existing flake recorded at the top of this file, wider than the two names
      measured at `a156f9a` but the same class and the same module — the wiring tests spawn
      scratch `herdr` programs and race under parallel execution.

## 2. The hit test — `layout::zone`
<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests in `src/ui/layout.rs` for `responsive-layout`'s four
      **pure** scenarios (its fifth, "The hit test agrees with what was drawn", is a view
      test and lands in group 3.1 beside the other one): `zone::the_zones_tile_the_frame`,
      `zone::below_the_breakpoint_only_the_routed_region`,
      `zone::the_breakpoint_is_exact_for_zone`, `zone::degenerate_frames_resolve`. Derive
      each expected interior in the test from `split_frame`/`split_body`/`interior`/
      `split_detail` independently, never from `zone`'s own answer.
      `cargo test --lib layout::tests::zone` — expect RED.
- [x] 2.2 GREEN: Add `pub enum Zone` with the five variants specs/responsive-layout names,
      and `pub fn zone(area, route, column, row) -> Zone` deriving its geometry through the
      existing splits and holding no arithmetic beyond containment.
- [x] 2.3 CHECK: Confirm `layout.rs` still names no filesystem, process, environment,
      network, or standard-I/O API and no crossterm type — `/bin/sh scripts/gates/noio-view.sh`
      exits 0. Not `colwidth.sh`: its `PURE` list is eight files and omits `src/ui/layout.rs`
      (`grep -n 'PURE=' scripts/gates/colwidth.sh`), so it would never read this file.
      **Run:** `NOIO-VIEW OK: 9 pure files carry no I/O API; positive control matched`,
      **exit 0**. `colwidth.sh`'s `PURE` is confirmed to be the eight files without
      `src/ui/layout.rs`. `zone` names no crossterm type and no ratatui widget — only
      `Rect` and `Position`, both already used by this file.
- [x] 2.4 Run the group tests — `cargo test --lib layout::` — no regressions, and record
      that no refactor was needed.
      **Run:** `cargo test --lib layout::` → **21 passed**, 0 failed (17 before, plus the
      four `zone::` tests). **No refactor was needed:** `zone` adds no arithmetic of its
      own — it calls the four existing splits and `Rect::contains`, and the two offsets it
      reports are the definitions of `ListRow::row` and `DetailTab::column` rather than a
      second derivation of anything.

## 3. Row addressing — `list::row_at`
<!-- kind: behavior -->

- [x] 3.1 RED: Write failing tests for `change-rows`' four scenarios:
      `list::tests::row_at::the_reported_row_follows_the_scrolled_slice`,
      `row_at::a_collapsed_section_reports_only_its_header`,
      `row_at::degenerate_interiors_report_nothing`, and, in `src/ui/view.rs`,
      `view::tests::every_drawn_row_is_reported_by_row_at` and
      `view::tests::the_hit_test_agrees_with_the_drawn_buffer` (responsive-layout's fifth
      scenario — it classifies every cell of a rendered buffer by `layout::zone`, so it
      lands here with the other view test rather than in group 2's pure set), both at 120x40
      and 60x20. `cargo test --lib row_at the_hit_test_agrees` — expect RED.
- [x] 3.2 GREEN: Extract the offset derivation `render_list` performs inline
      (`src/ui/view.rs:227-229`: `rows`, the `position(|r| r.selected)` cursor, and
      `viewport`) into one `list` function, and implement
      `row_at(dashboard, interior, row) -> Option<RowKind>` on top of it.
- [x] 3.3 REFACTOR: Point `render_list` at the extracted function so the derivation exists
      once. The rendered buffers must stay byte-identical — every existing `ui::view` and
      `ui::list` test passes unchanged, which is the check.
      **Done:** `render_list`'s three inline lines became one `list::drawn_rows` call. Every
      pre-existing `ui::view` and `ui::list` test passed unmodified, which is the check.
- [x] 3.4 Run the group tests — `cargo test --lib list:: view::` — no regressions.
      **Run:** the command as written is rejected by cargo (`unexpected argument 'view::'` —
      `cargo test` takes one positional filter), so it was run as
      `cargo test --lib -- list:: view::`, which passes both filters through to the test
      harness: **169 passed**, 0 failed (164 before, plus the three `row_at::` tests and the
      two `hit_test::` view tests).

## 4. Tab addressing — `detail::tab_at`
<!-- kind: behavior -->

- [x] 4.1 RED: Write failing tests for `artifact-tabs`' four addressing scenarios:
      `detail::tests::tab_at::each_cell_answers_for_its_own_columns` (at the mandated 78 and
      58), `tab_at::a_windowed_bar_answers_for_drawn_cells`,
      `tab_at::the_placeholder_addresses_nothing`, `tab_at::a_wide_id_is_addressed_by_columns`.
      `cargo test --lib tab_at` — expect RED.
- [x] 4.2 GREEN: Implement `tab_at(artifacts, selected, width, column) -> Option<usize>` from
      `tab_bar`'s own output, measuring each cell through `crate::ui::layout::columns`.
- [x] 4.3 CHECK: `/bin/sh scripts/gates/detailwidths.sh` and
      `/bin/sh scripts/gates/colwidth.sh` both exit 0 — the new function is parameterised by
      width and counts no `char`s.
      **Run:** `DETAILWIDTHS OK: all 42 detail tests name both 58 and 78`, exit 0, and
      `COLWIDTH OK: no char-count measurement in the eight pure view files`, exit 0. The
      gate caught real work: all four new tests initially named only 78 (and named it
      `78u16`, which the gate's `\b(\d+)\b` scan deliberately does not see), so each was
      rewritten to loop over an unsuffixed `[u16; 2] = [78, 58]`. Two of the four —
      `a_windowed_bar_answers_for_drawn_cells` and `the_placeholder_addresses_nothing` —
      are stated by `artifact-tabs` at width 78 alone and now assert at both; 58 windows
      harder, so neither claim is weakened.
- [x] 4.4 Run the group tests — `cargo test --lib detail::` — no regressions, and record
      that no refactor was needed.
      **Run:** `cargo test --lib detail::` → **46 passed**, 0 failed (42 before, plus the
      four `tab_at::` tests). **No refactor was needed:** `tab_at` is one `find_map` over
      `tab_bar`'s own output; there is no second placement calculation to fold away, which
      is the property `artifact-tabs` asks for.

## 5. The five actions and `Dashboard::apply`
<!-- kind: behavior -->

- [x] 5.1 RED: Write failing tests for `list-selection`'s ten scenarios and `detail-scroll`'s
      seven, named as design.md → Test Strategy lists them under `app::tests::click::` and
      `app::tests::scroll::`. The four "are the same move" and "produce equal dashboards"
      scenarios assert equality of two `Dashboard` values, not of one field.
      `cargo test --lib app::tests::click app::tests::scroll` — expect RED.
- [x] 5.2 GREEN: Add `SelectNext`, `SelectPrev`, `ScrollDown`, `ScrollUp`, and
      `Click(Target)` to `Action` and handle each in `apply`. `Click` does nothing when its
      target is absent from `targets()` and never clamps to a neighbour.
- [x] 5.3 REFACTOR: Reduce the `Next` and `Prev` arms to a route dispatch over the four
      region-explicit arms, per design.md → Decision 4, so the clamp-and-reset rule exists
      once.
      **Done:** the arithmetic moved into two private helpers, `Dashboard::select_by(step)`
      and `Dashboard::scroll_by(step)`. `Next`/`Prev` are now two-arm route dispatches
      naming them, and `SelectNext`/`SelectPrev`/`ScrollDown`/`ScrollUp` name the same two.
      `next_at_list_equals_select_next` and `next_at_detail_equals_scroll_down` are what
      hold that true.
- [x] 5.4 CHECK: Contract gate — `Action` is enumerated in two places that must agree. Add
      the five variants to `no_action_mutates_changes`' hand-written array
      (`src/ui/app.rs:1977`) **and** to its `assert_eq!(variants.len(), 18, …)` literal and
      prose at `:2030`, which is the third site that must agree. Then confirm
      `awk '/^pub enum Action \{/{f=1;next} f&&/^\}/{exit} f&&/^    [A-Z]/{n++} END{print n}' src/ui/app.rs`
      reports **23** (check K, **18** at HEAD).
      **Run:** `23`. All three sites were updated in the same commit, and rustc found two of
      them for me: `assert_known_variant`'s wildcard-free match failed to compile with
      `SelectNext … Click(Target) not covered`, which is exactly the signal that test's own
      comment says it exists to give.
- [x] 5.5 CHECK: `SCAN_MIN=206 TYPES='Dashboard Filter Detail Sections' /bin/sh scripts/gates/nodefault-ui.sh`
      exits 0 — no site elides a field on the four view-layer types.
      **Run:** `NODEFAULT-UI OK (half B): 208 literal/pattern spans scanned (>= 206), none
      elides a field` and `NODEFAULT-UI OK: no Default for [Dashboard Filter Detail
      Sections], no elided field; positive controls matched`, **exit 0**. The count rose
      from 206 to 208; the `SCAN_MIN` floor on the `Makefile` line is unchanged, since it is
      a floor.
- [x] 5.6 Run the group tests — `cargo test --lib app::` — no regressions.
      **Run:** **113 passed**, 0 failed (96 before, plus the ten `click::` and seven
      `scroll::` tests).

## 6. The resolver and the loop
<!-- kind: behavior -->

- [x] 6.1 RED: Write failing tests for `mouse-input`'s remaining resolver scenarios and
      `dashboard-loop`'s **four** new ones — including
      `pointer_motion_does_not_draw` and `a_click_after_motion_resolves_against_the_frame`,
      which assert `LoopSummary::frames`, the only observable that distinguishes a drawn
      frame from a skipped one — plus `artifact-tabs`' two click scenarios
      (`a_tab_click_equals_its_digit_key`, `a_tab_click_on_the_selected_cell_resets_nothing`),
      named as design.md → Test Strategy lists them under `driver::tests::`. `mouse_action_is_total` drives the full kind × coordinate × area ×
      route × dashboard cross product the spec enumerates.
      `cargo test --lib driver::tests` — expect RED.
- [x] 6.2 GREEN: Implement `mouse_action(dashboard, area, mouse) -> Action` over
      `layout::zone`, `list::row_at`, and `detail::tab_at`. It takes no filter flag, per
      design.md → Decision 10.
- [x] 6.3 GREEN: `run_loop` routes an `Event::Mouse` to `mouse_action` with the `area` it
      already copies out of the `CompletedFrame`, and every other event to `action_for`
      with `dashboard.filter.active`. One action per event; the quit check is unchanged.
- [x] 6.4 CHECK: The four tests that actually carry the key table today pass **unmodified** —
      `app::tests::action_for_is_total_over_a_keycode_sweep` (`src/ui/app.rs:1906`),
      `quit_keys_and_their_near_misses` (`:1701`),
      `navigation_and_filter_keys_are_distinguished` (`:2120`), and
      `space_maps_to_toggle_section_outside_filter_mode_and_types_inside_it` (`:2672`), plus
      `non_key_events_are_ignored` (`:1877`), which already asserts `Event::Mouse` → `Ignore`.
      Editing any of them to accommodate this change is the failure this task exists to
      catch. `cargo test --lib app::tests::` — expect green, having run more than 0 tests.
      **Run:** **113 passed**, 0 failed — well above zero, so the filter matched.
      `git diff` over `src/ui/app.rs` for this group is **4 lines**, all of them one test
      rename (see 6.6); not one of the five named tests was touched by this group, and none
      of them was touched by group 5 either — group 5's only edits to existing tests were
      `no_action_mutates_changes`' three enumeration sites, which the design names as its
      own subject.
- [x] 6.5 GREEN: `run_loop` skips the draw — and the `sync_detail` and `normalise_scroll`
      around it — for a `MouseEventKind::Moved` or `Drag(_)` event, carries the previous
      frame's `area` forward, and does not count the skipped iteration in
      `LoopSummary::frames` (per design.md -> Decision 12). Covered by 6.1's RED tests
      `pointer_motion_does_not_draw` and `a_click_after_motion_resolves_against_the_frame`.
- [x] 6.6 CHECK: `/bin/sh scripts/gates/noblock.sh`, `/bin/sh scripts/gates/nosleep.sh`, and
      `/bin/sh scripts/gates/nocli-shell.sh` all exit 0 — the resolver reads no clock,
      blocks on nothing, and names no `HerdrCli`.
      **Run:** `NOBLOCK OK` (all three legs, 12 files), `NOSLEEP OK` (all three legs),
      `NOCLI-SHELL OK: 12 files under src/ui name no CLI seam`, every one **exit 0**.
      **`NOCLI-SHELL` fired first, on a false positive, and a test was renamed rather than
      the gate loosened.** Its `CLI_RE` is `from_cli|OpenspecCli|HerdrCli|CliChanges|npm_prefix`
      with no word boundary, so `from_cli` matched inside
      `app::tests::click::a_collapsed_section_hides_its_rows_from_clicks` — the name
      design.md's matrix gives that test. The test is now
      `a_collapsed_section_hides_its_rows_when_clicked` and design.md's matrix row follows
      it. The alternative — anchoring `CLI_RE`'s `from_cli` on a word boundary — is the
      better repair of the two and is deliberately **not** made here: this change carries a
      `quality-gates` delta for `wired.sh` and `noraw-grep.sh` and none for
      `nocli-shell.sh`, and editing a specified gate with no delta authorising it is the
      exact drift the planning review caught in `wired.sh`. Recorded as a finding: any
      future identifier beginning `from_cli` — `from_client`, say — will trip this gate the
      same way.
- [x] 6.7 CHECK: `dashboard-loop`'s `An ignored key redraws and keeps waiting` scenario's
      existing test passes **unmodified** — the motion exemption must not have become a
      general ignore-means-no-draw rule.
      **Run:** `driver::tests::ignored_input_redraws_and_continues` — **1 passed**, unmodified
      (`git diff` shows no edit to it). `pointer_motion_does_not_draw`'s third leg asserts the
      same property from the other side: twenty ignored `Char('z')` presses report
      `frames: 21`, not `1`.
- [x] 6.8 Run the group tests — `cargo test --lib driver::` — no regressions, and record
      that no refactor was needed beyond 6.5's own extraction.
      **Run:** **69 passed**, 0 failed (49 before, plus this group's twenty). **No refactor
      was needed beyond 6.5's own:** `run_loop` gained two locals — `area`, carried across a
      skipped draw, and `draw`, cleared for exactly one iteration by a motion event — and
      the draw/sync/normalise trio moved inside `if draw`. `mouse_action` is one `match`
      over the event kind and the zone; there is no arithmetic in it to fold away, since
      `layout::zone`, `list::row_at`, and `detail::tab_at` own all of it.

## 7. The refused-capture row reaches the reader
<!-- kind: behavior -->

- [x] 7.1 RED: Write failing tests for `terminal-lifecycle`'s two wiring scenarios:
      `ui::tests::wiring::a_refused_capture_is_named_last` and
      `wiring::a_successful_capture_adds_no_row`. The first asserts the probe's own reasons
      come first and the capture reason last, and that the loop still runs.
      `cargo test --lib ui::tests::wiring::a_refused_capture wiring::adds_no_row` — expect RED.
- [x] 7.2 GREEN: Add `mouse_problem: Option<String>` to `Startup` and name it at all
      **6** construction sites (check L) — not the 11 a bare `grep -c 'Startup {'` reports. `run_wired` appends it to
      `dashboard.refresh.startup` after `collaborators.problems`.
- [x] 7.3 GREEN: `ui::run` binds the guard and passes `guard.mouse_problem()` as the field's
      value — a field expression, so `pub fn run()` still holds no branch and no loop.
- [x] 7.4 CHECK: Contract gate — every `Startup` construction site names the new field.
      `grep -c 'mouse_problem:' src/ui/mod.rs` reports **6** (it is `0` at HEAD), one per site
      of check L. `nodefault-ui.sh` is deliberately not the check here: it loops only over its
      `$TYPES` argument and never reads `Startup`; that `Startup` has no `Default` is enforced
      by rustc, since no site elides a field.
      **Run, and a correction to the check itself.** `grep -c 'mouse_problem:' src/ui/mod.rs`
      reports **15**, not 6, and 6 was never reachable: the predicted number counted only the
      `Startup` construction sites, while the pattern also matches the field's own
      declaration and every test-harness site. The 15 break down as: 1 the `Startup` field
      declaration (`:123`); **6** `Startup` construction sites — `:410` (the production one,
      in `run`, `guard.mouse_problem()`), `:2763`, `:2821`, `:3111`, `:3737`, `:3888` — which
      is exactly check L's six; 1 the `ProbedStartup` test-harness field declaration
      (`:2800`); and 7 `ProbedStartup` literals (`:3926`, `:3954`, `:4133`, `:4178`, `:4223`,
      `:4341`, `:4399`), five carried over and two written by 7.1.
      **The check that actually moved and actually holds is rustc's**, which the task text
      already names: `Startup` implements no `Default` and every site names every field, so
      adding the field failed to compile at three sites the edit had not reached — `:3111`,
      `:3737`, and `:3888` — and named each of them. That is the contract gate working;
      the grep was a proxy for it and a miscounted one.
- [x] 7.5 CHECK: Persistence gate — no migration, backfill, cache invalidation, or index
      rebuild applies (design.md → Persistence and Rollout). Confirm the plugin's writes are
      still exactly `agent-names.toml`: `/bin/sh scripts/gates/readonly-ui.sh` exits 0.
      **Run:** `READONLY-UI OK: 12 files under src/ui plus [src/watch.rs src/refresh.rs
      src/agents.rs src/launch.rs src/open.rs], no write API in production code; both
      controls matched; File::options, DirBuilder, and create_new controls matched`,
      **exit 0**. No migration, backfill, seeding, cache invalidation, or index rebuild
      applies: this change stores nothing and the `artifact-content` cache key is
      unchanged.
- [x] 7.6 Run the group tests — `cargo test --lib ui::tests::` — no regressions, and record
      that no refactor was needed.
      **Run:** `cargo test --lib ui::tests:: -- --test-threads=1` → **62 passed**, 0 failed
      (60 before, plus this group's two). Single-threaded for the pre-existing
      `ui::tests::wiring::` flake recorded at the top of this file. **No refactor was
      needed:** `run_wired` gained a four-line `if let` after the line that seeds
      `refresh.startup`, and `run` gained one field expression; the test harness gained one
      field on `ProbedStartup` rather than a seventh positional parameter on
      `run_wired_at`, which is the shape `degraded-states` already established there.

## 8. Gates
<!-- kind: operational -->

- [x] 8.1 CHECK: Confirm the two capture commands are outside the confinement sweep today —
      `grep -n 'EnableMouseCapture' scripts/gates/noraw-grep.sh` gives no output, **exit 1**
      (check H). **Run:** no output, exit 1, as recorded.
- [x] 8.2 CHANGE: Extend `noraw-grep.sh`'s `RAW_RE` to
      `enable_raw_mode|disable_raw_mode|EnterAlternateScreen|LeaveAlternateScreen|EnableMouseCapture|DisableMouseCapture`,
      and replace the one-of-any positive control with a per-name one: for each of the six
      names, the run fails unless the name matches `RAW_RE` **and** appears in
      `src/ui/terminal.rs`. One-of-any is satisfied by `enable_raw_mode` alone, so it would
      cover the capture pair vacuously in both directions.
- [x] 8.3 CHECK: Confirm leg 1 cannot carry this name. `pub struct Startup<'a>` is at
      `src/ui/mod.rs:94` and the file's only line-anchored `#[cfg(test)]` at `:542`, so the
      struct is inside `code "$MOD"`'s slice: `sed -n '94p;542p' src/ui/mod.rs` shows both.
      A leg-1 entry would be satisfied by the field declaration alone.
      **Run, and reproduced rather than reasoned about.** `pub struct Startup<'a>` is at
      `src/ui/mod.rs:94` and the file's only line-anchored `#[cfg(test)]` is now at `:560`
      (`:542` at planning time — this change's own edits moved it), so the struct is still
      well inside `code "$MOD"`'s slice. Proved executably in a scratch copy of `src scripts
      tests`: with `mouse_problem: guard.mouse_problem(),` replaced by `mouse_problem: None,`
      in `ui::run` — the exact defect — and `mouse_problem` added as a **fourteenth leg-1
      name** with leg 5c deleted, `wired.sh` printed `WIRED OK` and **exited 0**. The same
      plant against the real script fails: `WIRED FAIL: leg 5c: 'pub fn run()' does not name
      mouse_problem(`, **exit 1**. Reviewer D's finding, reproduced end to end.
- [x] 8.4 CHANGE: Add a body-scoped **leg 5c** to `wired.sh`, mirroring leg 5: `pub fn run()`'s
      body must name `mouse_problem(` and must not hardcode `mouse_problem: None`. Add a
      positive control anchored on `^    pub fn mouse_problem(` in `src/ui/terminal.rs`, on
      Guard A's terms, so a rename fails in the defining file. Leg 1's thirteen names are
      unchanged, so `wired.sh:246`'s `thirteen names present` message stays true; extend it
      to mention leg 5c rather than restating a count.
- [x] 8.5 CHANGE: Add four `[[control]]` entries to `tests/gate-controls.toml`, each a
      single exact-substring find/replace, which is all that file's format supports:
      (a) `EnableMouseCapture` planted in `src/ui/list.rs`, expecting `NORAW-GREP`'s
      confinement FAIL; (b) `mouse_problem: guard.mouse_problem(),` → `mouse_problem: None,`
      in `src/ui/mod.rs`, expecting leg 5c's FAIL; (c) `EnableMouseCapture` stripped from
      `noraw-grep.sh`'s own `RAW_RE`; and (d) `DisableMouseCapture` stripped from
      `src/ui/terminal.rs`. (c) and (d) are the two directions of the per-name vacuity leg
      `specs/terminal-lifecycle/spec.md` states, and (a) alone proves neither.
- [x] 8.6 VERIFY: `make gates` exits 0 (the change's artifacts must be `git add`ed first —
      `OPENSPEC-UNTOUCHED` fails on any untracked file under `openspec/`), and
      `cargo test --test gate_controls` exits 0 with every planted defect caught, and record
      that no refactor was needed.
      **Run:** `make gates` → **exit 0**, every script green including
      `NORAW OK: 36 files searched` and `WIRED OK: … run threads the guard's mouse_problem(
      (leg 5c) …`. `cargo test --test gate_controls` → **4 passed**, 0 failed; all **57**
      controls (53 before, plus this group's four) caught their plants.
      **Two pre-existing width gates went red first, and both were repairs to group 3's own
      tests rather than to a gate.** `LISTWIDTHS` requires every `#[test]` in
      `src/ui/list.rs` to name both 38 and 58, and the three `row_at::` tests named only 38;
      `WIDTHS` requires every test in `src/ui/view.rs` to name both 60 and 120, and the two
      `hit_test::` tests wrote `(120u16, 40u16)` — the same suffixed-literal trap
      `DETAILWIDTHS` caught in group 4, since all three gates scan `\b(\d+)\b` and do not
      see `120u16`. All five tests now loop over an unsuffixed pair of the mandated widths.
      This is a gap in the plan, not only in the work: groups 3 and 4 both close on a
      `cargo test` filter, and nothing before this task runs `make gates`; group 4's own
      3-line check happened to name `detailwidths.sh` and so caught its half, while group 3
      named no width gate at all.
      **No refactor was needed** to the two gate scripts beyond the specified edits.

## 9. Documentation
<!-- kind: operational -->
<!-- parallel-after: 7 -->

- [x] 9.1 CHECK: Read the two `SPEC.md` sections and the two `AGENTS.md` passages these
      tasks rewrite, and confirm what is stale before writing: § Keys describes a
      keyboard-only pane, § Degraded states has no capture row,
      `grep -c 'EnableMouseCapture' AGENTS.md` is `0`, and `grep -n 'seven further claims'
      AGENTS.md` finds the doc-conformance count group 10 makes nine.
      **Run, all four confirmed stale before writing:** `SPEC.md` § Keys (`:517`) is a
      fourteen-row table of keys with no mention of a pointer; § Degraded states (`:851`)
      has no capture row; `grep -c 'EnableMouseCapture' AGENTS.md` → **0**, and its
      terminal-seam rule lists four names; `grep -n 'seven further claims' AGENTS.md` →
      `:216`. `SPEC.md` § Doc-conformance checks (`:1052`) carries the matching seven-item
      list and was updated with them, which 9.2-9.6 did not name but which group 10's own
      tests read alongside `AGENTS.md`.
- [x] 9.2 Add in `SPEC.md`: § Keys (audience: anyone reading the design contract) — a mouse
      table naming the wheel over each region, the click on a change row, the second click,
      the click on a section header, and the click on a tab cell, plus one line stating that
      capture costs the terminal's own drag-to-select and how to override it. Nothing
      existing is stale; the section has described only keys since `tui-shell`, and the pane
      now has a second input device.
- [x] 9.3 Add in `SPEC.md`: § Degraded states (audience: the same) — one row for a terminal
      that refuses mouse capture, naming the reason's position as `refresh.startup`'s last
      entry. Required by that section's own contract: every degraded state is a row, and
      `tests/degraded_coverage.rs` fails on a row with no proof.
- [x] 9.4 Rewrite in `AGENTS.md`: the terminal-seam rule under § Architecture rules
      (audience: every future session) — its list of confined crossterm functions grows from
      four to six. Rewritten in place, not appended: the existing sentence becomes false the
      moment group 1 lands.
- [x] 9.5 Rewrite in `AGENTS.md`: § Current repo state's sentence on the event loop
      (audience: the same) — it enumerates the keys and says nothing about a pointer.
      Replace the enumeration's closing clause rather than adding a paragraph beside it;
      net addition to that section is at most two lines.
- [x] 9.6 Rewrite in `AGENTS.md`: § Quality gates' doc-conformance sentence (audience: the
      same) — it calls `tests/doc_contract.rs` "seven further claims" and enumerates them;
      group 10 adds two. Correct the count and the enumeration in place rather than
      appending; this is a net-zero edit.
- [x] 9.7 VERIFY: The passages group 10 binds now exist —
      `grep -c 'EnableMouseCapture' AGENTS.md` is at least `1` and `SPEC.md` § Keys holds a
      mouse table. Group 10's tests are the real verification and run there.
      **Run:** `grep -c 'EnableMouseCapture' AGENTS.md` → **1**; `SPEC.md` § Keys holds a
      nine-row mouse table, each row naming its `Action` variant in backticks, which is the
      extraction rule `specs/mouse-input/spec.md` states for
      `mouse_bindings_match_spec_md`. `cargo test --test doc_contract` → **55 passed** with
      the passages rewritten, so nothing that already existed was broken by the edits.

## 10. Contract-tier bindings
<!-- kind: operational -->

Classified operational, not behavior: group 9 writes the passages these tests bind, so a
RED between writing the test and implementing it could only be manufactured with a stub —
which the schema forbids. The evidence is a negative control instead.

- [x] 10.1 CHECK: Write `tests/doc_contract.rs::mouse_bindings_match_spec_md` and
      `::terminal_seam_names_match_the_gate`, then prove each can fail: in a scratch copy of
      the tree, delete `SPEC.md` → Keys' mouse table and show the first exits non-zero
      naming the absent table; restore it and show it goes quiet. Repeat by removing
      `DisableMouseCapture` from `AGENTS.md`'s confined list for the second. Record both
      halves' exit statuses in this file.
      **Run, in a scratch copy of the tree (`src tests scripts SPEC.md AGENTS.md openspec` and
      the manifests), `CARGO_TARGET_DIR` pointed at the real `target/`:**
      - `mouse_bindings_match_spec_md`: with `SPEC.md` → Keys' mouse table deleted →
        **FAILED**, exit non-zero, naming the absent table:
        `SPEC.md -> Keys holds no mouse table (no \`| Gesture | Action |\` header row)`.
        `SPEC.md` restored → **1 passed**, exit 0.
      - `terminal_seam_names_match_the_gate`: with `DisableMouseCapture` removed from
        `AGENTS.md`'s confined list → **FAILED**, exit non-zero, printing both sides —
        `AGENTS.md names {…5 names…} … while noraw-grep.sh's RAW_RE searches for {…6 names…}`.
        `AGENTS.md` restored → **1 passed**, exit 0.
      Four parser controls are checked in beside them, so neither extraction can rot into a
      vacuous one: `documented_mouse_actions_fails_on_a_missing_table` (an absent table, an
      absent section, and a table naming no variant are each an `Err`),
      `mouse_action_body_is_cut_from_the_production_slice` (the cut stops at the function's
      own closing brace and never reaches the inline test module, which names every `Action`
      variant the crate has), `gate_raw_names_parses_and_fails_loudly`, and
      `documented_seam_names_takes_identifiers_only` (a hyphenated change name in the same
      parenthetical is not an identifier and is not taken).
- [x] 10.2 CHANGE: Land both tests, reading `SPEC.md`, `AGENTS.md`,
      `scripts/gates/noraw-grep.sh`, and `src/ui/driver.rs` as the second sites.
- [x] 10.3 CHANGE: Add the refused-capture row to `tests/degraded-coverage.toml`, bound to
      `a_refused_capture_is_named_last`. Prove the binding is real the same way: reword the
      row's `condition` and show `cargo test --test degraded_coverage` fails.
      **Run:** the row is bound to **two** proofs, not one —
      `a_refused_capture_is_named_last` and `a_successful_capture_adds_no_row` — since the
      `SPEC.md` row states both the position of the reason and that a successful capture
      adds nothing. Negative control, in a scratch copy: `refuses` reworded to `declines` in
      `tests/degraded-coverage.toml` only → **8 of 10 failed**, the first naming
      `SPEC.md row "The terminal refuses mouse capture (\`enable_mouse\` fails)" has no
      tests/degraded-coverage.toml [[row]] entry`; restored → **10 passed**, 1 ignored,
      exit 0.
      **Seven pre-existing `covers` ranges had drifted and were repaired.** The test checks
      that each range holds a line of code, and this change's own edits shifted line numbers
      in five files. Rather than repair only the one range the test happened to report
      first, every entry was re-derived mechanically: for each `file:a-b`, the exact text at
      `baa22c7:file` lines `a..b` was located in the current file, and the range rewritten
      to where that same block now sits — `src/ui/detail.rs:210-216`→`245-251`,
      `218-223`→`253-258`, `230-237`→`265-272` (two sites), `src/ui/view.rs:56-58`→`55-57`,
      `src/ui/app.rs:641-654`→`721-734`, and `src/ui/mod.rs:343-348`→`360-365`. Every one
      still covers the same code it was written for, not merely *some* code. This is a
      standing hazard the row-level check cannot see: a range that drifts onto unrelated
      code passes it.
- [x] 10.4 VERIFY: `cargo test --test doc_contract --test degraded_coverage` exits 0 — no
      regressions.
      **Run:** `doc_contract` → **61 passed**, 0 failed (55 before, plus the two bindings and
      their four parser controls). `degraded_coverage` → **10 passed**, 0 failed, 1 ignored
      (unchanged count; the new row joins the existing table-driven test rather than adding
      one).

## 11. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [x] 11.1 VERIFY: `cargo test --lib driver::tests::a_mouse_event` passes end to end.
      **Run:** **1 passed**, 0 failed. The same test that was RED at group 0 — unmodified
      since — now passes: a left press on the second change row, read from the scripted
      source, reaches `mouse_action` through `run_loop` and moves the selection, while
      `action_for` still maps the identical event to `Ignore` under both filter modes.
- [x] 11.2 REFACTOR: Clean up the scripted source's mouse constructor and the replaced
      collaborators if warranted, or record that none was needed.
      **One extraction was warranted and made; nothing further was.** The acceptance test's
      own setup became `driver::tests::drive_one_event(event)`, which builds the dashboard,
      the `TestBackend`, the `Script`, and the four inert collaborators, drives `run_loop`
      with `event` then `q`, and returns the last frame's buffer — written that way at group
      0 precisely so task 0.3's harness control could substitute a key for the press with
      nothing else changed.
      `testutil::mouse` needs no cleanup: it is four lines beside `press`, takes the kind and
      a coordinate pair, and fixes `modifiers` to `NONE` on purpose — the tests that pin the
      modifier rule (`mouse_action_is_total`) construct their own `MouseEvent` rather than
      reaching for a second helper, so the constructor never grew a parameter nothing else
      uses.
      The replaced collaborators were left alone deliberately: group 6's three loop-driving
      tests each build their own `watch::none()`/`refresh::none()`/`agents::none()`/
      `launch::none()` quartet rather than sharing one, because two of them need a different
      dashboard and a different reader, and folding the quartet into a helper would leave a
      builder with more parameters than the four lines it replaced.

## 12. Change Review
<!-- kind: operational -->

- [x] 12.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session
      — against proposal.md, all eight spec files, design.md, and the diff. Concentration
      points for this change: that every one of the 84 spec scenarios names a test that
      would go red if its behaviour were deleted; that `mouse_action` is genuinely reached
      by `run_loop` rather than only unit-tested; that `Action`'s two enumeration sites
      (`apply`'s match and `no_action_mutates_changes`' array) agree; that no mouse gesture
      reaches the launcher; and that the two documents and the gate script name the same six
      confined functions.
      **Dispatched:** the repository's `outside-in-tdd-reviewer` agent, in its own session
      rather than a fork of this one, against `proposal.md`, all eight spec files,
      `design.md`, `tasks.md`, `planning-review.md`, and `git diff baa22c7..HEAD`. It
      reported findings and edited nothing. Concentration points 2, 3, 4 and 5 all hold; it
      re-ran every gate and every tier itself and found them green. Its two substantive
      findings were both established **by mutation** — it changed the production code, showed
      the whole suite stayed green, and named the missing assertion — which is the standard
      this project's own review asks for and is why they are acted on rather than argued
      with.
- [x] 12.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line
      reason, note SUGGESTIONs, and re-run affected tests.

      **CRITICAL — `dashboard-loop`'s "the area used SHALL be the one just drawn" was not
      pinned. Fixed.** `a_resize_before_a_click_costs_one_frame` chose the point `(100, 4)`,
      which at 120x40 is the detail region's *content area* and resolves to `Action::Ignore`
      via `Zone::Detail` — the same answer the 60x20 frame gives via `Zone::Outside`. The
      test therefore held under a `run_loop` that had pinned the startup frame's geometry
      forever. Reproduced here before fixing: mutating `area = completed.area;` to
      `if area == Rect::ZERO { area = completed.area; }` left **all 69 driver tests and all
      1213 lib tests green**. The test now presses `(62, 3)` — the third `tdd` tab cell,
      painted at columns 60-67 of the 120-column frame and outside the 60-column one — so
      stale geometry yields `SelectTab(2)` and moves `detail.tab`, fresh geometry yields
      `Ignore`. It asserts `detail.tab` is unmoved, `LoopSummary { frames: 3, polls: 3 }`,
      and derives the cell's own column rather than trusting the number. Verified RED against
      the mutation (`left: 2, right: 0`) and GREEN with it reverted.

      **WARNING — the wheel over the detail region's tab bar was not pinned. Fixed.**
      `mouse-input` states the region is the whole region, "its header row and its **tab
      bar** as well as its content area". Deleting `Zone::DetailTab { .. }` from both wheel
      arms left **all 1213 tests green** (only the recorded `wiring::` flake failed).
      `the_wheel_acts_over_a_border_and_not_the_chrome` now asserts `ScrollUp` **and**
      `ScrollDown` at four points of the detail region — its own border column, its header
      row, its tab-bar row, and its content area. Verified RED against the mutation and
      GREEN with it reverted.

      **WARNING — leg 5c's *name* half had no checked-in plant. Fixed.** `tests/gate-controls.toml`
      carried only `wired-mouse-problem-hardcoded`, which drives leg 5c's *literal* half
      (`mouse_problem: None`); design.md → Test Strategy names a control for the name half
      too, and task 8.5 narrowed five rows to four without recording the drop. Added
      `wired-mouse-problem-renamed`, planting `mouse_problem: capture_reason(),` — which
      matches neither `None` nor `mouse_problem(` — expecting `WIRED FAIL: leg 5c`. Controls
      go from 57 to **58**, all passing.

      **WARNING — `proposal.md` still claimed `WIRED`'s list grows to fourteen. Fixed.** Both
      its Modified Capabilities bullet and its Impact bullet were left over from the draft the
      planning review corrected; `design.md`, the `quality-gates` delta and the shipped
      `wired.sh` all do the opposite. Rewritten to state leg 5c and that leg 1 stays at
      thirteen, with the reason.

      **SUGGESTIONs — four acted on, one recorded.**
      - *design.md's matrix named three tests that do not exist* —
        `unwinding_still_restores`, `restore_then_delegates_last`,
        `render_thread_restores`. This is the same defect the planning review's fourth
        CRITICAL caught (a matrix filter matching zero tests exits 0 and proves nothing),
        reappearing in the three rows group 1 touched: task 1.1 deliberately kept the
        existing test names and the matrix was never updated to follow. Repointed at
        `a_panic_still_restores`, `restore_then_restores_before_delegating`, and
        `a_panic_on_the_render_thread_still_restores`, and each filter re-run to confirm it
        matches more than zero: **1**, **2**, **1 passed**.
      - *`clicks_that_address_nothing_are_inert` omitted the frame's header row*, which the
        left-click table names alongside the footer. `(10, 0)` added.
      - *`non_key_events_are_ignored` asserted neither `Paste("1")` nor `Paste("a")`*, though
        `dashboard-loop`'s scenario names both with their own AND clauses ("does not switch
        tabs", "**does not launch an agent**"). Pre-existing, but this change's MODIFIED
        `dashboard-loop` delta re-asserts that scenario, so its proof is in scope. Both
        added. **This edits one of the five tests task 6.4 requires to pass unmodified**, and
        the tension is worth naming: 6.4's subject is that no key test was *weakened* to
        accommodate the mouse, and design.md's own matrix row for this scenario already
        directs that it be "extended". Four of the five are untouched; the fifth is
        strengthened by two assertions that pass equally before and after this change.
      - *Decision 12's skipped draw opens a window the design did not name*: `drive_live_tier`
        still runs on a skipped-draw iteration, so an adopted `ChangeSet` can change
        `targets()` while the frame on screen predates the adopt. Named in `run_loop` and in
        design.md → Decision 12, with its bound — `Action::Click` names a `Target`, never an
        index — and its residue: a target that still exists and now names a different change
        does move the cursor. No code change; the reviewer confirmed the frame accounting
        itself is correct.
      - *Coverage sits exactly on the production floor* — 96.00% (4371/4553) against a 96%
        floor, no margin, with `src/ui/terminal.rs` at 32/81 as design.md predicted for the
        two new deliberately-uncovered `CrosstermOps` bindings. Recorded, not acted on: the
        floor is met and the rule is to add tests rather than lower it, but the next
        uncovered production line anywhere in the crate fails the gate.

      **The reviewer's concentration-point-1 verdict: four scenarios of the 84 had no proof
      that would go red. Three are the findings above. The fourth is closed here.**
      `quality-gates` → "Hardcoding the mouse-capture reason in `run` fails the gate" carries
      three clauses. The first is proved by the checked-in `wired-mouse-problem-hardcoded`
      control. The third — that leg **1** stays green against that same planted tree, which is
      the entire reason leg 5c is body-scoped — existed only as 8.3's scratch transcript and
      as prose in the control's `why`. It is now
      `tests/gate_controls.rs::leg_one_cannot_see_the_defect_leg_five_c_catches`: it reads the
      real `src/ui/mod.rs`, applies the real plant in memory, restates leg 1's own predicate
      and leg 5c's two halves, and asserts leg 1 is green while both halves of leg 5c fire.
      It also asserts the stronger form — leg 1 stays green on a `run` naming `mouse_problem`
      **nowhere at all** — which turns out to be *forced* rather than merely observed:
      `ui::run` constructs a `Startup`, so the declaration cannot move below `#[cfg(test)]`
      without the crate failing to compile. Both attempted negative controls proved that
      rather than falsifying it (`E0422: cannot find struct ... Startup`, and `E0609` for a
      renamed field), and the test says so in place rather than claiming a falsifiability it
      does not have.
      **The second clause is accepted, not proved:** that `cargo test --all-features` stays
      green against the planted tree needs a full suite run on a copied tree per invocation.
      Reproduced by hand at 8.3 and recorded there. It is also the weaker claim — it is a
      property of how the tests construct their own `Startup`, not of the plant.

      **Re-run after the fixes:** `cargo test --all-features --lib -- --test-threads=1` →
      **1213 passed**, 0 failed (the test *count* is unchanged — every repair strengthened an
      existing test rather than adding one, which is the point);
      `cargo test --test gate_controls` → **4 passed**, 58 controls; clippy and `cargo fmt`
      clean; `openspec validate mouse-input --strict` valid.
- [x] 12.3 VERIFY: Confirm no blocking or unowned finding remains.
      **Confirmed.** The reviewer reported 1 CRITICAL, 4 WARNINGs and 5 SUGGESTIONs, and
      answered concentration point 1 with four unproven scenarios. Every one is now either
      fixed or accepted with a stated reason, and none is unowned:
      - CRITICAL: fixed, verified RED against the mutation and GREEN reverted.
      - WARNINGs 1-4: all fixed (the tab-bar wheel, the fifth gate control, `proposal.md`'s
        stale "fourteen", and its remainder).
      - SUGGESTIONs: four fixed (the three stale matrix names, the frame header row, the two
        pastes, Decision 12's unnamed window); one — coverage sitting exactly on the 96%
        production floor — recorded rather than acted on, since the floor is met and this
        project's rule is to add tests rather than lower a floor. It is a standing hazard for
        the next change, not a finding against this one.
      - Concentration point 1's fourth gap: closed by
        `leg_one_cannot_see_the_defect_leg_five_c_catches`; its one remaining clause is
        accepted with the reason above.
      Concentration points 2, 3, 4 and 5 the reviewer verified directly and found holding.

## 13. Lint & Verify
<!-- kind: operational -->

- [x] 13.1 CHECK: Inspect the intended verification commands and affected tiers — `make check`
      runs format, lint, gates, test, and coverage; the contract tier runs inside `cargo test`.
      **Inspected:** `check: fmt-check lint gates test coverage`. The contract tier —
      `tests/manifest.rs`, `tests/degraded_coverage.rs`, `tests/doc_contract.rs`,
      `tests/gate_controls.rs`, `tests/ci_workflow.rs`, `tests/spec_purposes.rs`,
      `tests/coverage_prod.rs`, `tests/cli.rs` — runs inside `test`. Every tier this change
      touches is reached by that one target; nothing needed a separate invocation.
- [x] 13.2 VERIFY: `make check` exits 0, with the failing sub-command named if it does not.
      **`make check` exits 0** — verified in a clean checkout of this change's own HEAD
      (`git worktree add` at the tip commit, `diff -r src` against the working tree reporting
      no difference): format, lint, `make gates`, `cargo test --all-features` (**1213
      passed**, 0 failed), and coverage all green.

      **It does NOT exit 0 in the development working directory, and the reason is
      environmental rather than this change's.** There, `ui::tests::wiring::` fails 1-4 tests
      per run and takes ~35s instead of ~3.8s. Measured rather than assumed:

      | tree | result |
      |---|---|
      | baseline `baa22c7` | 27/27, 3.7-3.8s, **5 consecutive runs** |
      | group 6 `905bee4`, clean worktree | 27/27, 3.70s |
      | group 7 `e0a1b0a`, clean worktree | 29/29, 3.75s |
      | group 10 `ce0992b`, clean worktree | 29/29, 3.77s |
      | HEAD, clean worktree | 29/29, 3.77s, twice |
      | HEAD, this working directory | 1-4 failed, ~35s, **5 consecutive runs** |

      Identical source — `diff -r src` is silent — so it is neither the code nor the commit.

      **The cause is where the test executable lives, and nothing else.** Two hypotheses were
      tried and falsified before the real one: it is *not* the two stale
      `./target/release/herdr-openspec ui` panes that had been running here since Monday and
      Tuesday (killed; the failures continued unchanged), and it is *not* a stale build
      artifact (forcing a rebuild produced the same result). The isolating experiment is one
      variable: with `CARGO_TARGET_DIR` pointed outside `~/Code` and **the working directory
      unchanged**, the full suite is **1213 passed, 0 failed, 8.38s**, twice; with the
      repository's own `target/`, it is ~39s with one to four `ui::tests::wiring::` failures.
      Sharper still: copying the *fast* binary into `target/debug/` makes it slow (35.6s, 3
      failed) and copying the *slow* binary to `/private/tmp` makes it fast (3.87s, 29/29), so
      the executable's **location** decides the outcome and its content does not.
      `mdutil -s /` reports indexing enabled and `mdfind` confirms Spotlight has indexed
      `target/debug/deps/herdr_openspec-*`; `/private/tmp` is excluded by default. The wiring
      tests spawn scratch programs and are bounded by `testutil::UntilReady`'s **5-second**
      deadline, so the per-exec scanning cost pushes their predicates past it — which is also
      why a failing run takes 35s rather than 4s: each failure burns its full deadline.

      Measured cost of this change itself, in clean worktrees: baseline **1158 passed in
      4.69s**, HEAD **1213 passed in 8.56s**, from `ui::driver` (0.20s → 7.67s) and
      `ui::view` (0.40s → 6.13s) — the totality cross product and the every-cell hit-test
      sweep. Real, and worth knowing, but not the failure's cause: skipping both of those
      tests made the failing runs *worse*, not better.

      A developer seeing this locally can exclude `target/` from Spotlight
      (`touch target/.metadata_never_index`, or add it in System Settings → Spotlight →
      Privacy). CI is unaffected: GitHub runners index nothing.

      **This is a correction to a claim made twice during implementation.** Groups 1 and 6
      recorded these failures as the pre-existing flake this file's own header describes. That
      was too quick. The header's flake could not be reproduced at `baa22c7` in five
      consecutive runs of the same module, so either it was always this same environmental
      effect or it is rarer than recorded; either way the header's diagnosis should not be
      relied on by a future change. The honest statement is narrower: **the tree is green; this
      working directory is not, for a reason outside the tree.**
- [x] 13.3 VERIFY: Coverage is the `coverage` sub-command of 13.2's `make check`
      (`Makefile`: `check: fmt-check lint gates test coverage`), enforced at both floors —
      the total and the production slice — and is not re-run separately. If it falls short,
      add tests; never lower or waive a floor.
      **Run, inside 13.2's `make check`:** `COVERAGE-PROD OK: production 96.00% (4371/4553)
      >= floor 96%; test-module 95.41%`, and `cargo llvm-cov --fail-under-lines 80` reporting
      a **96.07%** total. Both floors met, neither lowered, no waiver added. Recorded as a
      standing hazard rather than a finding (Change Review, SUGGESTION 5): the production slice
      sits **exactly** on its floor with no margin, so the next uncovered production line
      anywhere in the crate fails this gate. `src/ui/terminal.rs` is at 32/81, the two new
      `CrosstermOps` capture bindings having joined `enter_alternate`/`leave_alternate` as
      deliberately-uncovered seam bindings exactly as design.md → Risks predicted.
- [x] 13.4 VERIFY: `openspec validate mouse-input --strict` reports the change valid.
      **Run:** `Change 'mouse-input' is valid`, exit 0 — before archiving, and again after
      every Change Review repair to `proposal.md` and `design.md`. After archiving,
      `openspec validate --specs --strict` reports **44 passed, 0 failed** over the whole
      spec set, so the eighteen added and six modified requirements landed clean.
- [x] 13.5 CHECK: The new `mouse-input` capability needs a written `## Purpose` before it can
      be archived. `openspec archive` writes the placeholder `TBD - created by archiving
      change <x>`, on which `tests/spec_purposes.rs` fails `cargo test` — HEAD's tip commit
      `a156f9a docs(specs): archive list-sections and repair the Purpose paragraphs` is this
      exact trap firing on the previous change. Write the paragraph into
      `openspec/specs/mouse-input/spec.md` immediately after archiving and confirm
      `cargo test --test spec_purposes` exits 0.
      **The trap fired exactly as predicted.** `openspec archive mouse-input --yes` moved the
      change to `archive/2026-09-09-mouse-input`, applied **+18 / ~6** across nine capability
      specs, and seeded `openspec/specs/mouse-input/spec.md` with `TBD - created by archiving
      change mouse-input`. `cargo test --test spec_purposes` → **FAILED**,
      `every_capability_has_a_written_purpose`. The paragraph was written immediately after —
      two paragraphs in the house style the sibling capabilities use: what the capability owns
      (the resolver, the key-only mapper it does not disturb, and what stays inert), then the
      two constraints that bound it, that nothing becomes mouse-only and that the resolver is
      answerable to the frame actually drawn. Re-run → **3 passed**, 0 failed, including
      `an_archived_placeholder_fails_the_test`, which is the control proving the check is not
      vacuous.
