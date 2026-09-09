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

- [ ] 10.1 CHECK: Write `tests/doc_contract.rs::mouse_bindings_match_spec_md` and
      `::terminal_seam_names_match_the_gate`, then prove each can fail: in a scratch copy of
      the tree, delete `SPEC.md` → Keys' mouse table and show the first exits non-zero
      naming the absent table; restore it and show it goes quiet. Repeat by removing
      `DisableMouseCapture` from `AGENTS.md`'s confined list for the second. Record both
      halves' exit statuses in this file.
- [ ] 10.2 CHANGE: Land both tests, reading `SPEC.md`, `AGENTS.md`,
      `scripts/gates/noraw-grep.sh`, and `src/ui/driver.rs` as the second sites.
- [ ] 10.3 CHANGE: Add the refused-capture row to `tests/degraded-coverage.toml`, bound to
      `a_refused_capture_is_named_last`. Prove the binding is real the same way: reword the
      row's `condition` and show `cargo test --test degraded_coverage` fails.
- [ ] 10.4 VERIFY: `cargo test --test doc_contract --test degraded_coverage` exits 0 — no
      regressions.

## 11. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 11.1 VERIFY: `cargo test --lib driver::tests::a_mouse_event` passes end to end.
- [ ] 11.2 REFACTOR: Clean up the scripted source's mouse constructor and the replaced
      collaborators if warranted, or record that none was needed.

## 12. Change Review
<!-- kind: operational -->

- [ ] 12.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session
      — against proposal.md, all eight spec files, design.md, and the diff. Concentration
      points for this change: that every one of the 84 spec scenarios names a test that
      would go red if its behaviour were deleted; that `mouse_action` is genuinely reached
      by `run_loop` rather than only unit-tested; that `Action`'s two enumeration sites
      (`apply`'s match and `no_action_mutates_changes`' array) agree; that no mouse gesture
      reaches the launcher; and that the two documents and the gate script name the same six
      confined functions.
- [ ] 12.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line
      reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 12.3 VERIFY: Confirm no blocking or unowned finding remains.

## 13. Lint & Verify
<!-- kind: operational -->

- [ ] 13.1 CHECK: Inspect the intended verification commands and affected tiers — `make check`
      runs format, lint, gates, test, and coverage; the contract tier runs inside `cargo test`.
- [ ] 13.2 VERIFY: `make check` exits 0, with the failing sub-command named if it does not.
      A failure confined to `ui::tests::wiring::g_focuses_the_agent_the_launch_started` or
      `wiring::an_unreachable_scratch_herdr_is_a_standalone_tui` is the pre-existing flake
      recorded at the top of this file; re-run those two with `-- --test-threads=1` to
      confirm before treating it as a regression.
- [ ] 13.3 VERIFY: Coverage is the `coverage` sub-command of 13.2's `make check`
      (`Makefile`: `check: fmt-check lint gates test coverage`), enforced at both floors —
      the total and the production slice — and is not re-run separately. If it falls short,
      add tests; never lower or waive a floor.
- [ ] 13.4 VERIFY: `openspec validate mouse-input --strict` reports the change valid.
- [ ] 13.5 CHECK: The new `mouse-input` capability needs a written `## Purpose` before it can
      be archived. `openspec archive` writes the placeholder `TBD - created by archiving
      change <x>`, on which `tests/spec_purposes.rs` fails `cargo test` — HEAD's tip commit
      `a156f9a docs(specs): archive list-sections and repair the Purpose paragraphs` is this
      exact trap firing on the previous change. Write the paragraph into
      `openspec/specs/mouse-input/spec.md` immediately after archiving and confirm
      `cargo test --test spec_purposes` exits 0.
