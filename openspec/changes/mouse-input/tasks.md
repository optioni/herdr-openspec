<!-- Planning-time baseline: commit `a156f9a` on `main`, working tree clean except this
     change's own untracked artifacts (`git status --porcelain` names only
     `openspec/changes/mouse-input/`). Every measurement below was run at that commit. -->

**Known-flaky at HEAD, not caused by this change.** `cargo test --all-features` at `a156f9a`
failed twice in three runs — `ui::tests::wiring::g_focuses_the_agent_the_launch_started`
(both runs) and `ui::tests::wiring::an_unreachable_scratch_herdr_is_a_standalone_tui` (one
run) — while `cargo test --all-features --lib ui::tests::wiring:: -- --test-threads=1`
passed **27/27**. The two tests spawn scratch `herdr` programs and race under parallel
execution. Group 13 re-runs the suite; a failure confined to those two names is this
pre-existing flake, not a regression, and is a separate bug fix outside this change.

**Groups 1 through 8 are sequential, and none is marked `parallel-after`.** File overlap
orders only some pairs — group 1 and group 7 both write `src/ui/mod.rs`
(`grep -c 'impl TerminalOps for' src/ui/mod.rs` is `1`, the test `Recorder`), and groups 2,
3, 4 and 5 write four disjoint files. The criterion that orders every remaining pair is the
third one: **a failure must stay attributable**, and every group here closes on
`cargo test --all-features`, one whole-crate compile and one whole-suite run. Two concurrent
implementers in this tree would each see the other's half-written file as their own red.
This repository does dispatch code groups in parallel where that criterion holds
(`archive/2026-09-08-seam-resilience/tasks.md` groups 5, 6 and 7), so the sequential answer
is stated rather than assumed.

Group 9 (Documentation) is placed **before** group 10 rather than after the review, against
the schema's usual ordering, because group 10's two `tests/doc_contract.rs` tests read the
`SPEC.md` and `AGENTS.md` passages group 9 writes. It is not marked `parallel-after: 0` for
the same reason: 10 depends on it. Groups 11, 12 and 13 are whole-change gates and cannot
precede the work they gate.

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
| I (green + negative control) | `/bin/sh scripts/gates/noraw-grep.sh` | `NORAW OK: 36 files searched, mode functions only in src/ui/terminal.rs, CrosstermOps at 6 sites`, **exit 0** | Negative control run in a scratch copy of `src tests scripts`: `RAW_RE` extended with `\|EnableMouseCapture\|DisableMouseCapture` and `// EnableMouseCapture` appended to `src/ui/app.rs` → `NORAW FAIL: terminal-mode function outside src/ui/terminal.rs: src/ui/app.rs:5295`, **exit 1**; plant removed → **exit 0** again |
| J (green + negative control) | `/bin/sh scripts/gates/wired.sh` | `WIRED OK: thirteen names present …`, **exit 0** | Leg 2 forbids a branch in `pub fn run()`; group 7 adds a field expression, not a branch. Negative control is already checked in: `cargo test --test gate_controls` plants a real defect per gate script |
| K (must-change) | `awk '/^pub enum Action \{/{f=1;next} f&&/^\}/{exit} f&&/^    [A-Z]/{n++} END{print n}' src/ui/app.rs` | **18** | Must be `23` after group 5 |
| L (must-change) | `grep -c 'Startup {' src/ui/mod.rs` | **11** | Every site names every field; all 11 must name `mouse_problem` after group 7 |
| M (must-change) | `grep -rn 'impl TerminalOps for' src tests` | **3** sites: `src/ui/terminal.rs:85`, `src/ui/terminal.rs:212`, `src/ui/mod.rs:2186` | All 3 must implement the six-method trait after group 1 |
| N (baseline) | `cargo test --all-features --lib -- --test-threads=1` | **1158 passed**, exit 0 | The suite this change must leave green |

---

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

Taken, per design.md → Test Strategy: the defect this change can most plausibly ship is a
resolver that is correct and never called, which is `live-refresh`'s own defect in a new
shape. Only `run_loop` can prove otherwise.

- [ ] 0.1 Add a `MouseEvent` constructor to `driver`'s existing scripted `EventSource`
      double, honouring design.md → Test Boundaries: events, backend, artifact reader, and
      all four collaborators replaced; the terminal never constructed.
- [ ] 0.2 RED: Write `driver::tests::a_mouse_event_moves_the_selection_through_the_loop`
      for `mouse-input` → "A mouse event is resolved through the loop and a key is not":
      drive `run_loop` at 120x40 with a left press on the second change row, then `q`, and
      assert the frame after the press carries the selection marker on that row.
- [ ] 0.3 Confirm it fails because the behaviour is missing, not the harness: the same test
      with `Char('j')` in place of the press must pass at HEAD.
      `cargo test --lib driver::tests::a_mouse_event` — expect RED.

## 1. Terminal capture lifecycle
<!-- kind: behavior -->

- [ ] 1.1 RED: Write failing tests in `src/ui/terminal.rs` for: `normal_lifetime_records_all_six`,
      `mouse_failure_still_returns_a_guard`, and the updated
      `normal_lifetime_is_enter_enter_leave_disable` (filtering the capture pair out of the
      recorded list, so the four-operation claim survives verbatim). Extend the `Recorder`
      double in `src/ui/terminal.rs` and the one in `src/ui/mod.rs:2186` to the six-method
      trait. `cargo test --lib terminal::` — expect RED.
- [ ] 1.2 GREEN: Add `enable_mouse` and `disable_mouse` to `TerminalOps`, implemented in
      `CrosstermOps` as one `ratatui::crossterm::execute!` of `EnableMouseCapture` /
      `DisableMouseCapture` each, mapping the error — no decision, no ordering, no state.
- [ ] 1.3 GREEN: `TerminalGuard::enter` calls `enable_mouse` after `enter_alternate` and
      stores a failure instead of returning it; `mouse_problem(&self) -> Option<String>`
      returns the `TerminalError`'s `Display` text. `Drop` and `restore_then` call
      `disable_mouse` first, unconditionally (per design.md → Decision 7), so the panic
      hook releases capture too.
- [ ] 1.4 REFACTOR: State whether the three `TerminalOps` implementors share enough to
      warrant extraction, or record that none was needed.
- [ ] 1.5 Run the group tests — `cargo test --lib terminal::` and
      `cargo test --lib ui::tests::` — no regressions.

## 2. The hit test — `layout::zone`
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing tests in `src/ui/layout.rs` for `responsive-layout`'s four
      pure scenarios: `zone::the_zones_tile_the_frame`,
      `zone::below_the_breakpoint_only_the_routed_region`,
      `zone::the_breakpoint_is_exact_for_zone`, `zone::degenerate_frames_resolve`. Derive
      each expected interior in the test from `split_frame`/`split_body`/`interior`/
      `split_detail` independently, never from `zone`'s own answer.
      `cargo test --lib layout::tests::zone` — expect RED.
- [ ] 2.2 GREEN: Add `pub enum Zone` with the five variants specs/responsive-layout names,
      and `pub fn zone(area, route, column, row) -> Zone` deriving its geometry through the
      existing splits and holding no arithmetic beyond containment.
- [ ] 2.3 CHECK: Confirm `layout.rs` still names no filesystem, process, environment,
      network, or standard-I/O API and no crossterm type —
      `/bin/sh scripts/gates/noio-view.sh` and `/bin/sh scripts/gates/colwidth.sh` both
      exit 0.
- [ ] 2.4 Run the group tests — `cargo test --lib layout::` — no regressions.

## 3. Row addressing — `list::row_at`
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests for `change-rows`' four scenarios:
      `list::tests::row_at::the_reported_row_follows_the_scrolled_slice`,
      `row_at::a_collapsed_section_reports_only_its_header`,
      `row_at::degenerate_interiors_report_nothing`, and, in `src/ui/view.rs`,
      `view::tests::every_drawn_row_is_reported_by_row_at` at 120x40 and 60x20.
      `cargo test --lib row_at` — expect RED.
- [ ] 3.2 GREEN: Extract the offset derivation `render_list` performs inline
      (`src/ui/view.rs:227-229`: `rows`, the `position(|r| r.selected)` cursor, and
      `viewport`) into one `list` function, and implement
      `row_at(dashboard, interior, row) -> Option<RowKind>` on top of it.
- [ ] 3.3 REFACTOR: Point `render_list` at the extracted function so the derivation exists
      once. The rendered buffers must stay byte-identical — every existing `ui::view` and
      `ui::list` test passes unchanged, which is the check.
- [ ] 3.4 Run the group tests — `cargo test --lib list:: view::` — no regressions.

## 4. Tab addressing — `detail::tab_at`
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests for `artifact-tabs`' four addressing scenarios:
      `detail::tests::tab_at::each_cell_answers_for_its_own_columns` (at the mandated 78 and
      58), `tab_at::a_windowed_bar_answers_for_drawn_cells`,
      `tab_at::the_placeholder_addresses_nothing`, `tab_at::a_wide_id_is_addressed_by_columns`.
      `cargo test --lib tab_at` — expect RED.
- [ ] 4.2 GREEN: Implement `tab_at(artifacts, selected, width, column) -> Option<usize>` from
      `tab_bar`'s own output, measuring each cell through `crate::ui::layout::columns`.
- [ ] 4.3 CHECK: `/bin/sh scripts/gates/detailwidths.sh` and
      `/bin/sh scripts/gates/colwidth.sh` both exit 0 — the new function is parameterised by
      width and counts no `char`s.
- [ ] 4.4 Run the group tests — `cargo test --lib detail::` — no regressions.

## 5. The five actions and `Dashboard::apply`
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing tests for `list-selection`'s ten scenarios and `detail-scroll`'s
      seven, named as design.md → Test Strategy lists them under `app::tests::click::` and
      `app::tests::scroll::`. The four "are the same move" and "produce equal dashboards"
      scenarios assert equality of two `Dashboard` values, not of one field.
      `cargo test --lib app::tests::click app::tests::scroll` — expect RED.
- [ ] 5.2 GREEN: Add `SelectNext`, `SelectPrev`, `ScrollDown`, `ScrollUp`, and
      `Click(Target)` to `Action` and handle each in `apply`. `Click` does nothing when its
      target is absent from `targets()` and never clamps to a neighbour.
- [ ] 5.3 REFACTOR: Reduce the `Next` and `Prev` arms to a route dispatch over the four
      region-explicit arms, per design.md → Decision 4, so the clamp-and-reset rule exists
      once.
- [ ] 5.4 CHECK: Contract gate — `Action` is enumerated in two places that must agree. Add
      the five variants to `no_action_mutates_changes`' hand-written array
      (`src/ui/app.rs:1977`) and confirm
      `awk '/^pub enum Action \{/{f=1;next} f&&/^\}/{exit} f&&/^    [A-Z]/{n++} END{print n}' src/ui/app.rs`
      reports **23** (check K, **18** at HEAD).
- [ ] 5.5 CHECK: `SCAN_MIN=206 TYPES='Dashboard Filter Detail Sections' /bin/sh scripts/gates/nodefault-ui.sh`
      exits 0 — no site elides a field on the four view-layer types.
- [ ] 5.6 Run the group tests — `cargo test --lib app::` — no regressions.

## 6. The resolver and the loop
<!-- kind: behavior -->

- [ ] 6.1 RED: Write failing tests for `mouse-input`'s remaining resolver scenarios and
      `dashboard-loop`'s two new ones, named as design.md → Test Strategy lists them under
      `driver::tests::`. `mouse_action_is_total` drives the full kind × coordinate × area ×
      route × dashboard cross product the spec enumerates.
      `cargo test --lib driver::tests` — expect RED.
- [ ] 6.2 GREEN: Implement `mouse_action(dashboard, area, mouse) -> Action` over
      `layout::zone`, `list::row_at`, and `detail::tab_at`. It takes no filter flag, per
      design.md → Decision 10.
- [ ] 6.3 GREEN: `run_loop` routes an `Event::Mouse` to `mouse_action` with the `area` it
      already copies out of the `CompletedFrame`, and every other event to `action_for`
      with `dashboard.filter.active`. One action per event; the quit check is unchanged.
- [ ] 6.4 CHECK: `app::tests::the_full_key_table_is_unchanged` and
      `app::tests::non_key_events_are_ignored` both pass — `action_for` still maps every
      `Event::Mouse` to `Ignore` and no key moved.
- [ ] 6.5 CHECK: `/bin/sh scripts/gates/noblock.sh`, `/bin/sh scripts/gates/nosleep.sh`, and
      `/bin/sh scripts/gates/nocli-shell.sh` all exit 0 — the resolver reads no clock,
      blocks on nothing, and names no `HerdrCli`.
- [ ] 6.6 Run the group tests — `cargo test --lib driver::` — no regressions.

## 7. The refused-capture row reaches the reader
<!-- kind: behavior -->

- [ ] 7.1 RED: Write failing tests for `terminal-lifecycle`'s two wiring scenarios:
      `ui::tests::wiring::a_refused_capture_is_named_last` and
      `wiring::a_successful_capture_adds_no_row`. The first asserts the probe's own reasons
      come first and the capture reason last, and that the loop still runs.
      `cargo test --lib ui::tests::wiring::a_refused_capture wiring::adds_no_row` — expect RED.
- [ ] 7.2 GREEN: Add `mouse_problem: Option<String>` to `Startup` and name it at all
      **11** construction sites (check L). `run_wired` appends it to
      `dashboard.refresh.startup` after `collaborators.problems`.
- [ ] 7.3 GREEN: `ui::run` binds the guard and passes `guard.mouse_problem()` as the field's
      value — a field expression, so `pub fn run()` still holds no branch and no loop.
- [ ] 7.4 CHECK: Contract gate — `Startup` has no `Default` and every site names every
      field. `SCAN_MIN=206 TYPES='Dashboard Filter Detail Sections' /bin/sh scripts/gates/nodefault-ui.sh`
      exits 0, and `grep -c 'Startup {' src/ui/mod.rs` still reports **11**.
- [ ] 7.5 CHECK: Persistence gate — no migration, backfill, cache invalidation, or index
      rebuild applies (design.md → Persistence and Rollout). Confirm the plugin's writes are
      still exactly `agent-names.toml`: `/bin/sh scripts/gates/readonly-ui.sh` exits 0.
- [ ] 7.6 Run the group tests — `cargo test --lib ui::tests::` — no regressions.

## 8. Gates
<!-- kind: operational -->

- [ ] 8.1 CHECK: Confirm the two capture commands are outside the confinement sweep today —
      `grep -n 'EnableMouseCapture' scripts/gates/noraw-grep.sh` gives no output, **exit 1**
      (check H).
- [ ] 8.2 CHANGE: Extend `noraw-grep.sh`'s `RAW_RE` to
      `enable_raw_mode|disable_raw_mode|EnterAlternateScreen|LeaveAlternateScreen|EnableMouseCapture|DisableMouseCapture`,
      and add a per-name positive control beside the existing one-of-any guard: the run fails
      unless `src/ui/terminal.rs` names **each** of the six, so the pattern cannot cover the
      capture pair vacuously.
- [ ] 8.3 CHANGE: Add `mouse_problem` to `wired.sh`'s leg 1 name list, so a `run` that stops
      threading the guard's reason into `Startup` fails rather than silently dropping the row.
- [ ] 8.4 CHANGE: Add a `[[control]]` to `tests/gate-controls.toml` planting
      `EnableMouseCapture` outside `src/ui/terminal.rs`, and one planting the removal of
      `mouse_problem` from `src/ui/mod.rs`'s production slice, each expecting its gate's own
      FAIL line.
- [ ] 8.5 VERIFY: `make gates` exits 0 (the change's artifacts must be `git add`ed first —
      `OPENSPEC-UNTOUCHED` fails on any untracked file under `openspec/`), and
      `cargo test --test gate_controls` exits 0 with every planted defect caught.

## 9. Documentation
<!-- kind: operational -->

- [ ] 9.1 Add in `SPEC.md`: § Keys (audience: anyone reading the design contract) — a mouse
      table naming the wheel over each region, the click on a change row, the second click,
      the click on a section header, and the click on a tab cell, plus one line stating that
      capture costs the terminal's own drag-to-select and how to override it. Nothing
      existing is stale; the section has described only keys since `tui-shell`, and the pane
      now has a second input device.
- [ ] 9.2 Add in `SPEC.md`: § Degraded states (audience: the same) — one row for a terminal
      that refuses mouse capture, naming the reason's position as `refresh.startup`'s last
      entry. Required by that section's own contract: every degraded state is a row, and
      `tests/degraded_coverage.rs` fails on a row with no proof.
- [ ] 9.3 Rewrite in `AGENTS.md`: the terminal-seam rule under § Architecture rules
      (audience: every future session) — its list of confined crossterm functions grows from
      four to six. Rewritten in place, not appended: the existing sentence becomes false the
      moment group 1 lands.
- [ ] 9.4 Rewrite in `AGENTS.md`: § Current repo state's sentence on the event loop
      (audience: the same) — it enumerates the keys and says nothing about a pointer.
      Replace the enumeration's closing clause rather than adding a paragraph beside it;
      net addition to that section is at most two lines.

## 10. Contract-tier bindings
<!-- kind: behavior -->

- [ ] 10.1 RED: Write `tests/doc_contract.rs::mouse_bindings_match_spec_md` and
      `::terminal_seam_names_match_the_gate` for `mouse-input`'s two documentation
      scenarios. Each must fail when its document passage is absent, not pass vacuously.
      `cargo test --test doc_contract` — expect RED before 10.2 and green after.
- [ ] 10.2 GREEN: Implement both, reading `SPEC.md`, `AGENTS.md`,
      `scripts/gates/noraw-grep.sh`, and `src/ui/driver.rs` as the second sites.
- [ ] 10.3 CHANGE: Add the refused-capture row to `tests/degraded-coverage.toml`, bound to
      `a_refused_capture_is_named_last`, and confirm
      `cargo test --test degraded_coverage` exits 0.
- [ ] 10.4 Run the group tests — `cargo test --test doc_contract --test degraded_coverage`
      — no regressions.

## 11. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 11.1 VERIFY: `cargo test --lib driver::tests::a_mouse_event` passes end to end.
- [ ] 11.2 REFACTOR: Clean up the scripted source's mouse constructor and the replaced
      collaborators if warranted, or record that none was needed.

## 12. Change Review
<!-- kind: operational -->

- [ ] 12.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session
      — against proposal.md, all eight spec files, design.md, and the diff. Concentration
      points for this change: that every one of the 73 spec scenarios names a test that
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
- [ ] 13.3 VERIFY: Coverage — `make coverage` exits 0 against both floors, the total and the
      production slice. If it falls short, add tests; never lower or waive a floor.
- [ ] 13.4 VERIFY: `openspec validate mouse-input --strict` reports the change valid.
