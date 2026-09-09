## Context

The pane draws clickable-looking rows and clickable-looking tab chips and answers none of
it. `ui::app::action_for` maps every `Event::Mouse` to `Action::Ignore`, and no capture
sequence is ever written, so no mouse event reaches the loop in the first place. Reaching
the eleventh archived change is eleven `j` presses; reaching the fourth artifact is knowing
it is the fourth.

Everything this change needs already exists in a testable shape. `layout` owns the frame
split (`split_frame`, `split_body`, `interior`, `split_detail`) as pure `Rect` arithmetic.
`list::rows` already emits a `RowKind` per drawn row, carrying `Section { key }` and
`Item { index }` — `list-sections` built exactly the addressing a click needs.
`detail::tab_bar` already reports each chip's own `x`, because `color-palette` needed the
painted cell and the reported cell to be one object. `run_loop` already copies the drawn
frame's `area` out of the `CompletedFrame` for `normalise_scroll`. The change is a hit test
over geometry that is already derived, plus two terminal-mode operations and five `Action`
variants.

The four sibling changes this proposal declared a dependency on have all landed and are
archived: `view-fidelity` (so widths are display columns, which a hit test over a CJK or
emoji name requires), `list-sections` (the section header target), `seam-resilience` (the
panic hook capture must release from), and `color-palette`.

## Goals / Non-Goals

**Goals:**

- Mouse capture entered and left with the rest of the terminal mode, in one mirrored order,
  released on normal return, error return, and panic alike, and **non-fatal** when the
  terminal refuses it.
- One pure hit test, `layout::zone`, from a frame area, a route, and a `(column, row)` pair
  to the part of the frame drawn there.
- Wheel scrolls the region under the pointer; click selects a row, opens the selected one,
  toggles a section header, or switches a tab.
- The mouse acts while filtering.
- Every target keeps its key; no key moves; nothing becomes mouse-only.

**Non-Goals:**

- No drag of any kind, no hover styling, no motion handling — every `Moved`, `Drag`, and
  `Up` maps to `Ignore`, and a pointer-motion event does not trigger a draw (Decision 12 —
  mapping it to `Ignore` is not by itself enough).
- No right-click, no context menu, no mouse-driven agent launch.
- No text selection of the pane's own; the terminal's is what capture displaces.
- No new dependency: crossterm's mouse types come through `ratatui::crossterm`, as every
  other crossterm type in the crate already does.
- No I/O in a view, no process spawn outside `cli`, no new module, no new worker thread.
- No change to the `Change` type, to `changes::from_files`, or to `changes::from_cli`.

## Boundaries

| Module | What changes | Pattern it follows |
|---|---|---|
| `src/ui/terminal.rs` | `TerminalOps` grows `enable_mouse`/`disable_mouse`; `TerminalGuard` enters and leaves them; `mouse_problem()` reports a refused capture | The existing four operations, verbatim — one crossterm call each in `CrosstermOps`, all ordering in the guard |
| `src/ui/layout.rs` | New `Zone` enum and `zone()` | `split_frame`/`split_body`/`interior` — pure `Rect` arithmetic, no ratatui widget, no crossterm type |
| `src/ui/list.rs` | New `row_at()`; the viewport-offset derivation extracted so `render_list` and `row_at` share it | `list::rows` — pure, total, parameterised by the interior |
| `src/ui/detail.rs` | New `tab_at()`, built from `tab_bar`'s own output | `tab_bar` reporting `Tab::x` so the painted cell and the reported cell are one object |
| `src/ui/app.rs` | Five `Action` variants; `apply` arms for them; `Next`/`Prev` become a route dispatch over the four region-explicit ones | `Action` + `Dashboard::apply` — pure over `&mut self` and the argument |
| `src/ui/driver.rs` | New `mouse_action()`; `run_loop` routes `Event::Mouse` to it with the drawn `area` | `run_loop` already holds `area`; the resolver is pure like everything else in the file |
| `src/ui/view.rs` | `render_list` calls the extracted offset helper instead of deriving it inline | No behaviour change; byte-identical frames |
| `src/ui/mod.rs` | `Startup` gains `mouse_problem: Option<String>`; `run` fills it from the guard; `run_wired` appends it to `refresh.startup` | `degraded-states`' injection of `env` and `npm_hook` onto `Startup` |

No process is spawned anywhere: `src/cli.rs` is untouched, and neither `mouse_action` nor
`zone` nor `row_at` nor `tab_at` names a spawn API, a filesystem API, an environment API, or
a clock. No file is added under `src/ui/`, so both view gates' `PURE` lists are unchanged and the new
code is swept from the day it lands — `noio-view.sh`'s nine files, and `colwidth.sh`'s
**eight**, which omits `src/ui/layout.rs` because that file holds the crate's one width
measure. `layout::zone` is therefore covered by `noio-view.sh` and, by construction, not by
`colwidth.sh`; it does no measuring, only `Rect` containment. No
worker thread is added; the crate stays at three.

The `Change` type is not altered. `changes::from_files` and `changes::from_cli` are not
touched, so the question of keeping them in agreement does not arise here.

Two gates need editing, and both edits are part of the change rather than a follow-up. The
second is a change to a *specified* invariant, so it carries its own delta under the
`quality-gates` capability rather than living only here:

- `scripts/gates/noraw-grep.sh`'s `RAW_RE` gains `EnableMouseCapture|DisableMouseCapture`,
  and its one-of-any positive control becomes per-name. Without the first the two new
  commands could be named anywhere in the crate; without the second the extended pattern
  would pass vacuously for the capture pair, since `src/ui/terminal.rs` already names
  `enable_raw_mode`. Specified by the `terminal-lifecycle` delta.
- `scripts/gates/wired.sh` gains a **body-scoped leg 5c**, not a fourteenth entry on leg 1.
  Leg 1 greps `code "$MOD"` — the whole production slice of `src/ui/mod.rs`
  (`scripts/gates/wired.sh:158-163`) — and `pub struct Startup<'a>` is declared in that slice
  at `src/ui/mod.rs:94`, well above the file's single line-anchored `#[cfg(test)]` at `:542`.
  The field declaration alone would satisfy a leg-1 name, leaving the gate green on a `run`
  that had stopped passing the value. This is the failure leg 5 already documents for
  `state::read` and answers by scoping to `$body`. Leg 5c does the same: `$body` must name
  `mouse_problem(`, and must not hardcode `mouse_problem: None`, mirroring leg 5's
  `state_dir: None` guard exactly. `openspec/specs/quality-gates/spec.md` states leg 1's size
  and the shape of these legs, so this change carries a `quality-gates` delta.

## Contracts

Every interface this change touches is internal to the crate; there is no external consumer,
no wire format, no pagination, and no streaming.

- **`TerminalOps` — breaking, one implementor and one test double.** The trait grows from
  four methods to six. `CrosstermOps` and the recording double in `src/ui/terminal.rs`'s own
  tests are the only implementors; both are updated here. `TerminalGuard::enter` keeps its
  `Result<Self, TerminalError>` signature — a refused capture is reported through
  `mouse_problem()`, not through the return type, so `ui::enter_if_terminal` and `ui::run`
  keep their shapes.
- **`Action` — additive.** Five variants: `SelectNext`, `SelectPrev`, `ScrollDown`,
  `ScrollUp`, `Click(Target)`. `Action` is `#[non_exhaustive]`-free and matched
  exhaustively in `Dashboard::apply` and in `no_action_mutates_changes`' hand-written
  array, both of which must enumerate the same set; the compiler enforces the first and the
  test's own comment demands the second.
- **`Startup` — breaking, six construction sites.** One new field, `mouse_problem:
  Option<String>`. Six, not the eleven a bare `grep -n 'Startup {' src/ui/mod.rs` reports:
  five of those eleven hits are `ProbedStartup {`, a different, test-local struct declared at
  `src/ui/mod.rs:2755`. The real sites are lines 386 (the one production site, in `run`),
  2724, 2776, 3065, 3690, and 3840 — `grep -n '[^d]Startup {' src/ui/mod.rs | grep -v Probed`
  reports exactly those six. `Startup` implements no `Default` and is constructed with every field
  named, so every site fails to compile until updated — which is the point.
- **`Zone`, `row_at`, `tab_at`, `mouse_action` — additive.** New surface, no existing
  caller.

## Persistence and Rollout

- **migration:** none. No stored state, no file format, no schema.
- **backfill:** none.
- **seeding:** none.
- **cache invalidation:** none. The `artifact-content` cache key stays `(change directory,
  tab)`; a tab click writes `detail.tab` exactly as a digit key does, so `sync_detail`
  invalidates on the same terms it already does.
- **index rebuild:** none.
- **authorization:** none. The plugin's write boundary is unchanged — its only write is
  still `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`, and no mouse gesture writes
  anything or starts a process.
- **observability:** one new problem row, `refresh.startup`'s last entry, when the terminal
  refuses mouse capture.
- **deployment:** `make build` and the existing pane entry. No manifest change, no config
  key, no `min_herdr_version` bump — capture is a terminal capability, not a Herdr one.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The terminal (raw mode, alternate screen, mouse capture) | replaced — the recording `TerminalOps` double; `CrosstermOps` is constructed only in `ui::run` (`src/ui/mod.rs:379`) and in `terminal::install_panic_hook` (`src/ui/terminal.rs:163`), and `NORAW-GREP` leg 2 confines the name to those two files | replaced — same double |
| The panic hook | not reached — installing one is process-global and `cargo test` runs tests in parallel threads of one process, so `install_panic_hook` is untested by construction (`src/ui/terminal.rs:150-157`). Its body is `restore_then_if`, which **is** driven through the recording double, including the new `disable_mouse` call | replaced — same double, via `restore_then` and `restore_then_if` |
| The terminal event stream (`EventSource`) | replaced — the scripted double `run_loop` already takes | replaced — `mouse_action` takes a `MouseEvent` value directly, with no source at all |
| The terminal backend (drawing) | replaced — `ratatui::backend::TestBackend` at 60x20 and 120x40 | replaced — same, for the view tests; the pure functions need no backend |
| The filesystem (artifact reads) | replaced — the injected `ArtifactReader` closure | replaced — same |
| The filesystem (change enumeration) | replaced — `Dashboard` values built in-process | replaced — same |
| The filesystem watcher (`FsEvents`) | replaced — the existing non-blocking double | not reached |
| The refresh worker (`Refresher`) | replaced — the existing non-blocking double | not reached |
| The `openspec` binary (`OpenspecCli`) | not reached — no CLI call is added or removed | not reached |
| The Herdr socket (`HerdrCli`, `AgentPoll`, `Launcher`) | replaced — the existing doubles; asserted **unreached** by any mouse gesture | not reached |
| `HERDR_PLUGIN_STATE_DIR` / the environment | replaced — the injected `&dyn Fn(&str) -> Option<String>` lookup | not reached |
| The clock | not reached — no new code reads one; `NOSLEEP` sweeps `src/ui/` for it | not reached |
| The process environment of `ui::run` | not reached — `mouse_problem` arrives on `Startup`, never read from the environment | not reached |
| `SPEC.md`, `AGENTS.md`, `scripts/gates/noraw-grep.sh` | real files, read by `tests/doc_contract.rs` | real files |

## Test Strategy

This repository's tiers, and the command that runs each:

- **Pure unit tests** over `layout`, `list`, `detail`, `app` — inline `#[cfg(test)]`
  modules. `cargo test --lib`.
- **View tests** rendering into a `ratatui::backend::TestBackend` at the mandated widths —
  60 and 120 columns for the frame, 38/58 for the list interior, 78/58 for the detail
  interior. `cargo test --lib`.
- **Loop/acceptance tests** driving `run_loop` and `run_wired` with a scripted
  `EventSource`, a `TestBackend`, and the four non-blocking collaborator doubles.
  `cargo test --lib`.
- **Contract tier** — `tests/doc_contract.rs`, `tests/degraded_coverage.rs`,
  `tests/gate_controls.rs`. `cargo test --test doc_contract` and friends.
- **Gates** — `make gates`.

**Every filtered command in the matrix below must run more than zero tests.** Measured:
`cargo test --lib the_full_key_table` at HEAD prints `running 0 tests … 1158 filtered out`
and **exits 0**. A row whose filter matches nothing is therefore a green row that proves
nothing, and this plan had three of them before review. Every command below either names an
existing test that the implementer must confirm still matches, or names a test a RED task
writes; a task that runs one and sees `0 passed` must treat that as a failure, not a pass.
The whole-suite runs in group 13 are what actually gate the change; the filters are for
iteration speed.

**This change takes the outer-loop acceptance test.** Two behaviours are unreachable from
any unit: that a mouse event read from the source reaches `mouse_action` at all (the defect
`live-refresh` shipped, in its own shape — a resolver that is never called), and that the
area it is resolved against is the frame just drawn rather than a stale one. Both are driven
through `run_loop` with a scripted source.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A mouse event is resolved through the loop and a key is not | `driver::tests::a_mouse_event_moves_the_selection_through_the_loop` | Acceptance | replaced: events, backend, reader, fs, refresher, agents, launcher | `cargo test --lib driver::tests::a_mouse_event` |
| Resolution is total over adversarial geometry | `driver::tests::mouse_action_is_total` — the full kind × coordinate × area × route × dashboard cross product | Unit | none | `cargo test --lib mouse_action_is_total` |
| The two regions scroll independently at 120 columns | `driver::tests::the_two_regions_scroll_independently` | Unit + view | replaced: backend | `cargo test --lib scroll_independently` |
| The wheel acts over a border and not over the chrome | `driver::tests::the_wheel_acts_over_a_border_and_not_the_chrome` | Unit | none | `cargo test --lib over_a_border` |
| At 60 columns only the routed region answers the wheel | `driver::tests::at_60_columns_only_the_routed_region_answers` | Unit | none | `cargo test --lib only_the_routed_region_answers` |
| Horizontal wheel events do nothing | `driver::tests::horizontal_wheel_events_do_nothing` | Unit | none | `cargo test --lib horizontal_wheel` |
| A click selects a change row and a second click opens it | `driver::tests::a_click_selects_and_a_second_click_opens` | Unit + view | replaced: backend | `cargo test --lib a_second_click_opens` |
| A click on a section header folds it exactly as `Space` does | `driver::tests::a_header_click_equals_space` — equality of two `Dashboard` values | Unit | none | `cargo test --lib a_header_click_equals_space` |
| A click on an archived header opens an unresolved archive and requests its refresh | `driver::tests::a_header_click_requests_the_archive_refresh` | Unit | none | `cargo test --lib requests_the_archive_refresh` |
| A click on a tab cell switches to that artifact | `driver::tests::a_tab_click_switches_the_tab` | Unit + view | replaced: backend | `cargo test --lib a_tab_click_switches` |
| Clicks that address nothing are inert | `driver::tests::clicks_that_address_nothing_are_inert` | Unit | none | `cargo test --lib address_nothing_are_inert` |
| The other buttons and the non-press kinds are inert | `driver::tests::the_other_buttons_are_inert` — asserts every call returns `Action::Ignore`, and in particular none of `LaunchApply`, `LaunchContinue`, `LaunchArchive`, `FocusAgent`. Not "the launcher received nothing": `mouse_action` is pure and takes no `Launcher`, so that assertion would be true before the function existed | Unit | none | `cargo test --lib the_other_buttons_are_inert` |
| A click selects while the filter is open | `driver::tests::a_click_acts_while_filtering` | Unit | none | `cargo test --lib acts_while_filtering` |
| The wheel scrolls while the filter is open | `driver::tests::the_wheel_acts_while_filtering` | Unit | none | `cargo test --lib the_wheel_acts_while_filtering` |
| The key table is unchanged | the four tests that carry the table today, passing **unmodified**: `action_for_is_total_over_a_keycode_sweep` (`src/ui/app.rs:1906`), `quit_keys_and_their_near_misses` (`:1701`), `navigation_and_filter_keys_are_distinguished` (`:2120`), `space_maps_to_toggle_section_outside_filter_mode_and_types_inside_it` (`:2672`) | Unit | none | `cargo test --lib app::tests::` |
| Every mouse action has a key that produces the same effect | `driver::tests::every_mouse_action_has_an_equal_key` — six paired `Dashboard` equalities | Unit | none | `cargo test --lib has_an_equal_key` |
| The documented bindings match the resolver | `tests/doc_contract.rs::mouse_bindings_match_spec_md` | Contract | real: `SPEC.md`, `src/ui/driver.rs` | `cargo test --test doc_contract mouse_bindings` |
| The documented confined set matches the gate | `tests/doc_contract.rs::terminal_seam_names_match_the_gate` | Contract | real: `AGENTS.md`, `scripts/gates/noraw-grep.sh` | `cargo test --test doc_contract terminal_seam_names` |
| A name hidden in a block comment no longer satisfies the gate | `scripts/gates/wired.sh`'s stripper control, carried over unchanged | Gate | real: the tree | `make gates` |
| Deleting the panic-hook call fails the gate | `tests/gate-controls.toml`'s existing `wired` control, carried over unchanged | Gate | real: the tree | `cargo test --test gate_controls` |
| Deleting the mouse-capture reason from `run` fails the gate | new `[[control]]` in `tests/gate-controls.toml` planting the removal of `mouse_problem` from `ui::run` | Gate | real: the tree | `cargo test --test gate_controls` |
| Hardcoding the mouse-capture reason in `run` fails the gate | new `[[control]]` planting `mouse_problem: None,` in `ui::run`, expecting `wired.sh` leg 5c's FAIL | Gate | real: the tree | `cargo test --test gate_controls` |
| A renamed definition fails in the defining file | `scripts/gates/wired.sh`'s positive control, carried over unchanged | Gate | real: the tree | `make gates` |
| The real implementation is the only place naming a terminal-mode function | `scripts/gates/noraw-grep.sh`, plus its planted-defect control | Gate | real: the tree | `make gates` |
| No test constructs the real terminal implementation | `scripts/gates/noraw-grep.sh` leg 2 | Gate | real: the tree | `make gates` |
| Normal lifetime records the four operations mirrored | `terminal::tests::guard::normal_lifetime_is_enter_enter_leave_disable` (`src/ui/terminal.rs:228`) — updated to filter the capture pair out | Unit | replaced: `TerminalOps` double | `cargo test --lib normal_lifetime` |
| Normal lifetime records all six operations mirrored | `terminal::tests::guard::normal_lifetime_records_all_six` | Unit | replaced: `TerminalOps` double | `cargo test --lib records_all_six` |
| Raw mode fails and nothing else is attempted | `terminal::tests::guard::enable_raw_failure_attempts_nothing_further` (`src/ui/terminal.rs:245`), extended with the `enable_mouse` assertion | Unit | replaced: double | `cargo test --lib enable_raw_failure` |
| The alternate screen fails and raw mode is unwound | `terminal::tests::guard::alternate_screen_failure_unwinds_raw_mode` (`src/ui/terminal.rs:254`), extended with the `enable_mouse` assertion | Unit | replaced: double | `cargo test --lib alternate_screen_failure` |
| Mouse capture fails and the guard is still returned | `terminal::tests::guard::mouse_failure_still_returns_a_guard` | Unit | replaced: double | `cargo test --lib mouse_failure` |
| Unwinding past the guard still restores | `terminal::tests::guard::a_panic_still_restores` — updated to the six-entry list | Unit | replaced: `TerminalOps` double | `cargo test --lib a_panic_still_restores` |
| The hook restores before the previous hook runs | `terminal::tests::guard::restore_then_restores_before_delegating` — updated to lead with `disable_mouse` | Unit | replaced: double | `cargo test --lib restore_then` |
| A panic on a worker thread restores nothing | existing test, asserted **unchanged** at `["previous_hook"]` — the check that capture gained no off-thread exception | Unit | replaced: double | `cargo test --lib worker_thread` |
| A panic on the render thread still restores | `terminal::tests::guard::a_panic_on_the_render_thread_still_restores` — updated to lead with `disable_mouse` | Unit | replaced: double | `cargo test --lib a_panic_on_the_render_thread` |
| The refusal touches no terminal operation | `ui::tests::enter_if_terminal_*` — the `Ok` arm's list becomes three entries; the empty-list assertion is unchanged | Unit | replaced: double | `cargo test --lib enter_if_terminal` |
| Teardown errors are swallowed rather than panicking in Drop | `terminal::tests::guard::teardown_errors_do_not_panic_and_both_are_attempted` (`src/ui/terminal.rs:266`), extended to all three teardown operations | Unit | replaced: double | `cargo test --lib teardown_errors` |
| A refused capture becomes a leading problem row, below the probe's own | `ui::tests::wiring::a_refused_capture_is_named_last` | Acceptance | replaced: terminal, events, backend, reader, collaborators | `cargo test --lib a_refused_capture` |
| A successful capture adds no row | `ui::tests::wiring::a_successful_capture_adds_no_row` | Acceptance | replaced: as above | `cargo test --lib adds_no_row` |
| The zones tile the frame at 120 columns | `layout::tests::zone::the_zones_tile_the_frame` | Unit | none | `cargo test --lib zone::the_zones_tile` |
| Below the breakpoint only the routed region has zones | `layout::tests::zone::below_the_breakpoint_only_the_routed_region` | Unit | none | `cargo test --lib only_the_routed_region` |
| The breakpoint is exact for the hit test too | `layout::tests::zone::the_breakpoint_is_exact_for_zone` | Unit | none | `cargo test --lib the_breakpoint_is_exact_for_zone` |
| Degenerate frames resolve without panicking | `layout::tests::zone::degenerate_frames_resolve` | Unit | none | `cargo test --lib degenerate_frames_resolve` |
| The hit test agrees with what was drawn | `view::tests::the_hit_test_agrees_with_the_drawn_buffer` — every cell of a rendered `TestBackend` buffer classified by `zone` | View | replaced: backend | `cargo test --lib agrees_with_the_drawn_buffer` |
| Every drawn row is reported by the row it occupies | `view::tests::every_drawn_row_is_reported_by_row_at` | View | replaced: backend | `cargo test --lib reported_by_row_at` |
| The reported row follows the scrolled slice | `list::tests::row_at::the_reported_row_follows_the_scrolled_slice` | Unit | none | `cargo test --lib follows_the_scrolled_slice` |
| A collapsed section reports its header and nothing behind it | `list::tests::row_at::a_collapsed_section_reports_only_its_header` | Unit | none | `cargo test --lib reports_only_its_header` |
| Degenerate interiors report nothing | `list::tests::row_at::degenerate_interiors_report_nothing` | Unit | none | `cargo test --lib degenerate_interiors` |
| Each of the five tdd cells answers for its own columns | `detail::tests::tab_at::each_cell_answers_for_its_own_columns` | Unit | none | `cargo test --lib answers_for_its_own_columns` |
| A windowed bar answers for the cells it actually drew | `detail::tests::tab_at::a_windowed_bar_answers_for_drawn_cells` | Unit | none | `cargo test --lib a_windowed_bar_answers` |
| The placeholder and the degenerate widths address nothing | `detail::tests::tab_at::the_placeholder_addresses_nothing` | Unit | none | `cargo test --lib the_placeholder_addresses_nothing` |
| A wide artifact id is addressed by its display columns | `detail::tests::tab_at::a_wide_id_is_addressed_by_columns` | Unit | none | `cargo test --lib a_wide_id_is_addressed` |
| A tab click and its digit key are indistinguishable | `driver::tests::a_tab_click_equals_its_digit_key` | Unit | none | `cargo test --lib equals_its_digit_key` |
| A tab click on the already-selected cell resets nothing | `driver::tests::a_tab_click_on_the_selected_cell_resets_nothing` | Unit | none | `cargo test --lib on_the_selected_cell_resets_nothing` |
| A click on an unselected change moves the cursor and resets the tab | `app::tests::click::moves_the_cursor_and_resets_the_tab` | Unit | none | `cargo test --lib click::moves_the_cursor` |
| A click on the already-selected change resets nothing | `app::tests::click::the_selected_change_resets_nothing` | Unit | none | `cargo test --lib the_selected_change_resets_nothing` |
| A click naming a target that is gone changes nothing | `app::tests::click::an_absent_target_changes_nothing` | Unit | none | `cargo test --lib an_absent_target_changes_nothing` |
| A click reaches a row a collapsed section hides only when it is open | `app::tests::click::a_collapsed_section_hides_its_rows_when_clicked` | Unit | none | `cargo test --lib hides_its_rows_when_clicked` |
| Click and `Space` produce equal dashboards | `app::tests::click::click_and_space_produce_equal_dashboards` | Unit | none | `cargo test --lib produce_equal_dashboards` |
| Clicking open an unresolved archive requests the refresh that resolves it | `app::tests::click::clicking_open_an_unresolved_archive_requests_a_refresh` | Unit | none | `cargo test --lib unresolved_archive_requests` |
| A click on a header that is not drawn is inert | `app::tests::click::an_undrawn_header_is_inert` | Unit | none | `cargo test --lib an_undrawn_header_is_inert` |
| The second click opens and the third does nothing | `app::tests::click::the_second_click_opens_and_the_third_does_nothing` | Unit | none | `cargo test --lib the_third_does_nothing` |
| The second click matches `Enter` exactly | `app::tests::click::the_second_click_matches_enter` | Unit | none | `cargo test --lib matches_enter` |
| Two clicks on a section header fold and unfold it | `app::tests::click::two_header_clicks_fold_and_unfold` | Unit | none | `cargo test --lib fold_and_unfold` |
| The wheel scrolls the detail region at the list route | `app::tests::scroll::scroll_down_scrolls_at_the_list_route` | Unit + view | replaced: backend | `cargo test --lib scrolls_at_the_list_route` |
| `ScrollUp` stops at the top | `app::tests::scroll::scroll_up_stops_at_the_top` | Unit | none | `cargo test --lib scroll_up_stops_at_the_top` |
| A held wheel is clamped by the frame, not by `apply` | `app::tests::scroll::a_held_wheel_is_clamped_by_the_frame` | View | replaced: backend | `cargo test --lib clamped_by_the_frame` |
| `Next` at the detail route and `ScrollDown` are the same move | `app::tests::scroll::next_at_detail_equals_scroll_down` | Unit | none | `cargo test --lib next_at_detail_equals` |
| The wheel moves the selection at the detail route | `app::tests::scroll::select_next_moves_at_the_detail_route` | Unit | none | `cargo test --lib moves_at_the_detail_route` |
| A clamped move resets nothing | `app::tests::scroll::a_clamped_select_resets_nothing` | Unit | none | `cargo test --lib a_clamped_select_resets_nothing` |
| `Next` at the list route and `SelectNext` are the same move | `app::tests::scroll::next_at_list_equals_select_next` | Unit | none | `cargo test --lib next_at_list_equals` |
| Pointer motion does not cost a frame | `driver::tests::pointer_motion_does_not_draw` — drives `run_loop` with N `Moved` events and asserts `LoopSummary.frames` | Acceptance | replaced: events, backend, reader, collaborators | `cargo test --lib pointer_motion_does_not_draw` |
| A click after motion still resolves against the drawn frame | `driver::tests::a_click_after_motion_resolves_against_the_frame` | Acceptance | replaced: events, backend, reader, collaborators | `cargo test --lib a_click_after_motion` |
| A resize between the draw and the click costs one frame, not a panic | `driver::tests::a_resize_before_a_click_costs_one_frame` | Acceptance | replaced: events, backend, reader, collaborators | `cargo test --lib a_resize_before_a_click` |
| The loop routes mouse and key events to different mappers | `driver::tests::the_loop_routes_mouse_and_key_to_different_mappers` | Acceptance | replaced: as above | `cargo test --lib different_mappers` |
| The four action keys map, and their near misses do not | `app::tests::the_four_action_keys_map_and_their_near_misses_do_not` — carried over, unchanged, re-run | Unit | none | `cargo test --lib the_four_action_keys_map` |
| A launch action reaches no collaborator and starts no work | existing test, carried over unchanged | Unit | none | `cargo test --lib` |
| A refused launch records the reason and produces no request | existing test, carried over unchanged | Unit | none | `cargo test --lib` |
| An unreachable socket makes the four keys change nothing | existing test, carried over unchanged | Unit | none | `cargo test --lib` |
| Both quit keys quit and neither near-miss does | `app::tests::quit_keys_and_their_near_misses` — carried over unchanged | Unit | none | `cargo test --lib quit_keys_and_their_near_misses` |
| A released quit key does not quit | existing test, carried over unchanged | Unit | none | `cargo test --lib` |
| Enter and Esc move between the two routes | existing test, carried over unchanged | Unit | none | `cargo test --lib` |
| `Esc` dismisses one layer at a time | existing test, carried over unchanged | Unit | none | `cargo test --lib` |
| Navigation and filter keys are distinguished from near misses | existing test, carried over unchanged | Unit | none | `cargo test --lib` |
| Non-key events are ignored without panicking | `app::tests::non_key_events_are_ignored` — extended with the new `AND` about `action_for` staying key-only | Unit | none | `cargo test --lib non_key_events_are_ignored` |
| `Space` maps to `ToggleSection` outside filter mode and types inside it | `app::tests::space_maps_to_toggle_section_outside_filter_mode_and_types_inside_it` — its count wording updated | Unit | none | `cargo test --lib space_maps_to_toggle_section` |

## Decisions

**1. The hit-test type is `Zone`, not `Target`.** The proposal named it `Target`; that name
was taken by `list-sections`, which landed after the proposal was written and uses
`ui::app::Target` for "one addressable row the cursor can land on". Two `Target` types with
different meanings in the same layer would be worse than a rename. `Zone` names what it
returns: the part of the frame drawn at a point. *Alternative:* `layout::Target` alongside
`app::Target`, rejected because every call site would have to disambiguate; `Hit`, rejected
because it says the pointer landed on something without saying on what.

**2. `Zone::ListRow` and `Zone::DetailTab` carry the rectangle they were derived from.**
The resolver needs the list interior (for the viewport offset) and the tab bar (for the
cell columns). Carrying them means the derivation happens once, inside `zone`. *Alternative:*
plain `u16` offsets with the caller recomputing the interior through `split_frame` →
`split_body` → `interior`, rejected because the two derivations could drift — a second
`split_body` call passed the wrong route, say — and the failure would be a click landing on
the wrong row, which is exactly the class of defect this change is supposed to eliminate.

**3. Five flat `Action` variants, not two payloaded ones.** `SelectNext`, `SelectPrev`,
`ScrollDown`, `ScrollUp`, `Click(Target)` rather than `Scroll(Region, Direction)`.
`agent-launch` argued this in its own design and `dashboard-loop`'s spec records the
reason: `no_action_mutates_changes` enumerates the variants by hand, and a payload lets it
carry one value and silently omit the others. `Click` carries `Target` because a click's
subject is genuinely data — the row — not a fixed choice from a small set.

**4. `Next`/`Prev` become a route dispatch over the four region-explicit actions.** Rather
than duplicating the selection and scroll arithmetic, `apply(Next)` at `Route::List` calls
the same helper `apply(SelectNext)` does. This is the REFACTOR step of the group that adds
them, and it is what the two "are the same move" scenarios assert. *Alternative:* leaving
the arithmetic in the `Next` arm and writing it again in `SelectNext`, rejected — two
copies of a clamp-and-reset rule is exactly how a key and a wheel come to disagree.

**5. The wheel names the region; the click names the row.** A wheel event does not change
the route, so a wheel over the list at `Route::Detail` moves the selection and leaves `j`
and `k` scrolling the content. *Alternative:* having the wheel also focus the region it
scrolls, rejected as scope the proposal does not ask for and as surprising — a scroll is a
look, not a choice.

**6. A refused mouse capture is non-fatal and named.** `SPEC.md`'s "never fail closed" rule
makes refusing to start wrong: the pane is fully usable by key without capture.
`TerminalGuard::enter` therefore records the error instead of returning it, and the reason
reaches the reader as the last entry of `refresh.startup`. *Alternative A:* fatal — rejected
outright by the invariant. *Alternative B:* silent — rejected because this project's
recurring defect is a condition that is true and invisible, which is what `degraded-states`
existed to close. It is appended **last** rather than first because it explains the least:
a missing binary changes what the pane can show, a refused capture only withdraws a second
way to reach what the keys already reach, and it must not push a probe reason off the top of
a short list.

**7. Teardown disables capture unconditionally.** Writing the disable sequence to a terminal
that never enabled it is inert. *Alternative:* a `mouse: bool` on the guard gating the call,
rejected because its false arm would be a production branch no test can observe from
outside, and because the panic hook's `restore_then` has no guard to consult and would need
its own answer anyway.

**8. `mouse_action` lives in `ui::driver`, not `ui::app`.** It needs `list::rows` and
`detail::tab_bar`, and `driver` is where the frame's `area` already lives — `run_loop`
copies it out of the `CompletedFrame` for `normalise_scroll`, so the resolver's one input
that nothing else has is already in hand there. Not for module-cycle reasons: Rust permits
mutual references between modules of one crate, and this crate already has such a cycle
(`app.rs:557` -> `ui::detail::content_lines`, `detail.rs:34` -> `ui::list::progress_cell`,
`list.rs:6` -> `use crate::ui::app::…`). *Alternative:* `ui::app`, rejected because the
resolver would then need the frame area threaded in from `driver` anyway, which is the
argument for putting it where the area is; a new `src/ui/mouse.rs`, rejected because it
would grow both view gates' `PURE` lists and change three gate scripts for no gain.

**9. `run_loop` resolves against the frame just drawn.** The area is already copied out of
the `CompletedFrame` for `normalise_scroll`; the resolver reuses it. A resize between the
draw and the click costs one mis-targeted event — the same one-frame window
`normalise_scroll` already accepts. *Alternative:* storing a size on `Dashboard`, rejected
because `Dashboard` deliberately carries no width, no layout mode, and no frame.

**10. The mouse ignores filter mode.** A printable key is ambiguous while filtering and is
resolved as a character; a click is not ambiguous. `mouse_action` therefore takes no filter
flag at all, which makes the rule structural rather than a branch someone can get wrong.

**12. A pointer-motion event does not trigger a draw.** Measured, not assumed: crossterm
0.29.0's `EnableMouseCapture` writes `?1000h ?1002h ?1003h ?1015h ?1006h`
(`crossterm-0.29.0/src/event.rs:321-335`), and `?1003h` is *any-event tracking — report all
motion events*. `run_loop` draws at the top of every iteration
(`src/ui/driver.rs:106-135`), so with capture on, merely moving the pointer across the pane
would re-render a full frame per motion event — including recomputing `detail::content_lines`
for the whole document. Resolving the event to `Action::Ignore` cannot prevent that: the
draw happens before the event is even read.

`run_loop` therefore skips the draw — and the `sync_detail` and `normalise_scroll` around it
— for a `MouseEventKind::Moved` or `Drag(_)` event, carrying the previous frame's `area`
forward so the next click still resolves against what is on screen, and not counting a
skipped frame in `LoopSummary.frames`. Nothing else changes: an ignored **key** still
redraws, which `dashboard-loop`'s existing "An ignored key redraws and keeps waiting"
scenario pins and this change must not break.

**The exemption's own cost, named during Change Review rather than discovered later.**
`drive_live_tier` still runs on a skipped-draw iteration, so an adopted `ChangeSet` can
change `targets()` while the frame on screen predates that adopt, and a click read at the end
of that same iteration is resolved against rows the reader is not looking at. Without the
exemption this window does not exist, because an adopt is always followed by a draw before
the next event is read. It is bounded by the rule Risks below already states — `Action::Click`
names a `Target`, never a row index, and `apply` does nothing when that target is absent from
`targets()` — but that mitigation covers only the *absent*-target case: a target that still
exists and now names a different change does move the cursor. Accepted at the width of one
adopt landing between a pointer motion and a click, and corrected by the very next frame,
which is the same one-frame window Decision 9 already accepts for a resize.

*Alternative A:* accept the redraws and correct the non-goal. Rejected — the pane sits in a
Herdr split the reader moves a pointer across constantly to reach other panes, and a
continuous re-render for pointer travel that is not even aimed at the pane is a cost with no
benefit at all. *Alternative B:* emit button-event tracking only, writing
`?1000h ?1002h ?1015h ?1006h` by hand instead of `EnableMouseCapture`. Rejected: crossterm
offers no narrower command, so this puts raw escape literals in `CrosstermOps`, breaks the
"one crossterm call per method, no decision" rule the seam has kept since `tui-shell`, and
leaves `EnableMouseCapture` named nowhere — which would make the `NORAW-GREP` extension this
change adds guard a name the crate does not use. *Alternative C:* skip the draw whenever the
action is `Ignore`. Rejected: it silently rewrites the pinned key scenario above, for no gain
over the narrow motion rule.

**13. Mouse modifiers are ignored.** `MouseEvent` carries a `modifiers: KeyModifiers`
field (crossterm 0.29.0 `src/event.rs:777-786`) that nothing in this design reads: a
`Shift+Down(Left)` resolves exactly as a bare one, and `Ctrl+ScrollDown` exactly as a bare
wheel. This is deliberate and is the opposite of `action_for`'s rule, which is meticulous
about modifiers because a modified key is usually a *different* key. A modified click is
still a click on the same row, and inventing a second meaning for it would make a target
reachable only by pointer — which the "nothing becomes mouse-only" requirement forbids.

The one case that argued for the opposite is the drag-select escape hatch: a reader holding
`Shift` (or `Option`) to select text. That override is the **terminal's**, applied before
the sequence is ever sent — a terminal honouring it forwards no mouse event at all, so
there is nothing for the resolver to ignore. A terminal that forwarded it instead would
move the cursor under the reader's selection whether the resolver read the modifier or not,
since the alternative — refusing every modified press — would break the readers whose
terminals do intercept it and who therefore never send one. *Alternative:* map any modified
press or wheel to `Ignore`. Rejected for that reason, and recorded here rather than left as
an unstated assumption `mouse_action_is_total` would silently pin either way.

**11. Problem and message rows stay unaddressable.** `row_at` reports their `RowKind`
faithfully — they are what is drawn there — and `mouse_action` is what refuses to act on
them. Keeping the refusal in the resolver rather than in the row grammar means
`change-rows` gains no notion of clickability and `RowKind` keeps its existing four
variants.

## Risks / Trade-offs

- **Drag-to-select is lost while the pane is focused** → Stated in the proposal as the
  change's one accepted cost, documented in `SPEC.md` → Keys, and mitigated by the
  terminal's own override: `Option` on macOS, `Shift` on most Linux terminals. Not
  mitigable in-pane without reimplementing selection, which the proposal excludes.
- **A terminal that reports mouse events the pane did not ask for, or reports them with a
  different coordinate origin** → `mouse_action` is total over every coordinate including
  `u16::MAX`, and every unrecognised point resolves to `Outside` and then to
  `Action::Ignore`. The worst case is a wasted redraw.
- **A click lands on a row that changed between the draw and the event** (a refresh adopted
  a new `ChangeSet` in the same iteration) → `Action::Click` names a `Target`, not a row
  index, and `apply` does nothing when that target is absent from `targets()`. It never
  clamps to a neighbour, so the failure is an ignored click rather than a wrong selection.
- **A held wheel produces a burst of events, each forcing a draw** → Accepted, and the same
  bound a held `j` already has: `sync_detail`'s `(change directory, tab)` cache means a
  scroll re-reads nothing, and the clamp is applied per frame by `normalise_scroll`. A wheel
  burst is the reader asking for frames; pointer *motion* is not, which is why only motion
  is exempted from the draw (Decision 12).
- **Any-event tracking cannot be turned off independently of capture** → `EnableMouseCapture`
  is all-or-nothing in crossterm 0.29.0, so the pane receives motion events it never wants.
  Mitigated in the loop rather than at the terminal, per Decision 12, and bound by a frame-count
  assertion rather than left as a claim.
- **`Action` grows to twenty-three variants, and `no_action_mutates_changes` enumerates
  them by hand** → The five new variants are added to that array in the same task group
  that adds them to the enum, and the exhaustive `match` in `apply` fails to compile until
  each is handled.
- **Coverage:** the change adds pure, fully-drivable functions and no untestable binding
  beyond `CrosstermOps`' two new one-line methods, which join `enter_alternate` and
  `leave_alternate` as deliberately-uncovered seam bindings. The production-slice floor is
  expected to rise, not fall; if it does fall, the answer is more tests, never a lowered
  floor.

## Migration Plan

No deploy order, no migration, no backfill, no rollback machinery. The change is one
binary; `make build` replaces it and the pane picks it up on its next open. Rolling back is
reverting the commits. The one user-visible regression — drag-to-select — is reversed by
reverting, not by a flag: a runtime toggle for capture was considered and rejected, because
it would need a `config.toml` key, a second startup path, and a degraded state of its own,
for a preference the terminal's `Option`/`Shift` override already serves.

## Open Questions

None. The four sibling changes this depended on are archived, the crossterm mouse API is
confirmed present in the vendored 0.29.0 (`MouseEventKind`, `MouseButton`,
`EnableMouseCapture`, `DisableMouseCapture`, all reachable through `ratatui::crossterm`),
and the two gate edits are named above rather than left to be discovered.

## Visual Design

Not applicable. This change modifies a terminal view, and no visual design source exists
for it — the pane's grammar is specified in text by `change-rows`, `artifact-tabs`, and
`responsive-layout`, and this change adds no new drawn element at all: it draws nothing new,
it only makes what is already drawn addressable by pointer.
