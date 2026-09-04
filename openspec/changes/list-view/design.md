## Context

Ten changes have landed. `herdr-openspec ui` opens a real dashboard — terminal lifecycle,
draw-then-wait loop, the 100-column breakpoint, a `TestBackend` harness — and `ui::load`
already reads the whole repository from files, with no `openspec` binary present. What the
pane shows in the body is two empty bordered frames whose interiors `responsive-layout`
asserts are blank.

This change fills the left one. It is the Phase 4 `list-view` row of
`openspec/IMPLEMENTATION-ORDER.md`, whose single dependency is `tui-shell` (archived
2026-09-04). `main` is green at 465 tests and 98.01% line coverage over 8,836 lines.

Constraints that shape everything below:

- **A Herdr split pane is frequently 40–60 columns** (`PRD.md` → Constraints), which is why
  the layout is not a fixed split. The two mandated frame widths produce list-region
  interiors of **58 columns** (60-wide frame, narrow, whole body) and **38 columns**
  (120-wide frame, wide, `Constraint::Length(40)` less two borders). Both interiors are
  **16 rows** at a 20-row frame. Every row-grammar decision below is made against 38.
- **`SPEC.md`'s List view mock is 48 characters wide.** `tui-shell` recorded that it
  illustrates content rather than width and deferred the width decision here. This change
  takes it and corrects `SPEC.md`.
- **Views are pure functions from state to a ratatui frame and perform no I/O.** A view
  test that needs a real directory means logic leaked into the view. `NOIO-VIEW` is the
  mechanical form of that, and its pure-file set grows by one here.
- **This change depends on `changes-from-files`, not on the CLI path.** The list must
  render with no `openspec` binary present at all. `NOCLI-SHELL` is the mechanical form.
- **Nothing writes inside `openspec/`.** The list is read-only because an agent may be
  editing `tasks.md` in another pane (`PRD.md` → Non-goals).
- **The 80% coverage floor is enforced and never waived** (`quality-gates`).

## Goals / Non-Goals

**Goals**

1. The list region shows every active change with its progress, then a separator, then the
   archived changes, at 38 and 58 interior columns, with no row overflowing its region.
2. A selection moves with `j` / `k` and the arrows, clamped at both ends, and the drawn
   slice follows it.
3. `/` opens a filter in which printable keys type instead of commanding, and only `Ctrl-C`
   still quits.
4. Every empty and degraded body state renders content inside the frame: no repository, no
   changes, no *active* changes, no match, and repository-level problems.
5. Nothing in `src/ui/` names a process-spawn API or the CLI seam, and the **five** pure
   files name no I/O API at all — each checked mechanically, each with a positive control,
   each proven able to fail against a planted violation.

**Non-Goals**

- The detail side: no artifact tab bar, no markdown, no "No content yet" —
  `markdown-viewer`, `detail-view`, `tasks-tab`.
- Agent badges and the footer's unattributed-agent count — `agent-attribution`. No column
  width is reserved for them here.
- Per-change `Change::problems`, which shares SPEC.md's third mock column with the badge —
  `degraded-states`.
- The CLI path, the watcher, the debounce, the worker thread, the `r` key — `live-refresh`.
- Mouse support, colour themes, configurable keys, a `?` help overlay.
- Any change to `Change`, `ChangeSet`, `changes::from_files`, or `changes::from_cli`.

## Boundaries

| Module | Change | Pattern followed |
|---|---|---|
| `src/ui/list.rs` (new) | `Row`, `RowKind`, `rows` — every function takes an interior width | Pure total transformation, like `ui::layout` — plain data in, plain data out, no ratatui styling |
| `src/ui/layout.rs` | `viewport` joins `mode`, `split_frame`, `split_body` | Pure geometry, unchanged in shape |
| `src/ui/view.rs` | `render_body` gains `render_list`; `render_footer` gains the two filter forms; `shorten_for_header` refactored onto a shared `shorten_left` | The render seam's pure side, unchanged in shape |
| `src/ui/app.rs` | `Filter`; `Dashboard` gains `selected` and `filter`; `Action` gains five variants and renames one, for nine in all; `action_for` gains a parameter; `apply` becomes a small state machine; `matches`, `Dashboard::visible`, `Dashboard::visible_len` | Pure total transformation, like `tasks::parse` |
| `src/ui/driver.rs` | One call site: `action_for(&event, dashboard.filter.active)` | Unchanged shape |
| `src/ui/mod.rs` | `load` names the two new fields | Composition root, unchanged shape |
| `src/changes.rs` | `#[cfg(test)] pub(crate) mod fixture` — the crate's only `Change` / `ChangeSet` literals outside the producers | The existing `#[cfg(test)] pub(crate) mod conformance` beside it |
| `SPEC.md`, `AGENTS.md` | Corrections listed under Decisions | `tui-shell`'s group 10 |

**No process spawn is added.** `src/cli.rs` remains the only module naming
`process::Command`, `Command::new`, or `Stdio`, and `src/ui/` names none of them.
`NOSPAWN-GREP` is carried forward with its `MIN` raised from the **measured** 15 files at
`main` to 16, because `src/ui/list.rs` is added. `MIN` is a parameter precisely so this is
an edit to a task's argument rather than to the check.

**Every new view is in `ui` and is a pure function of state.** `ui::list` takes a
`&Dashboard` and a `u16`; `ui::view::render` takes a `&mut Frame` and a `&Dashboard`.
Neither takes a path, a `Config`, a clock, or a CLI. `NOIO-VIEW`'s pure set becomes
`app.rs`, `layout.rs`, `list.rs`, `view.rs`, `driver.rs`; the three files deliberately
outside it are unchanged and each keeps its stated reason (`mod.rs` holds `load` and `run`,
`terminal.rs` holds the terminal seam, `event.rs` holds `CrosstermEvents`).

**`Change` is unaltered.** No field is added, removed, or retyped, so the `from_files` /
`from_cli` agreement is untouched and neither producer needs a matching edit. The only new
construction sites are the test fixtures, and they live inside `src/changes.rs` so
`change-model`'s `GATE-MECH1` (no `Default`, no `..`) already covers them. `NOLIT-CHANGE`
is added to prove that placement holds rather than being a convention nobody re-checks: a
`Change {` or `ChangeSet {` literal anywhere else under `src/` fails it.

**`Dashboard`'s gate widens rather than weakens.** `NODEFAULT-UI` becomes a loop over the
type list `Dashboard Filter`, keeping both halves (no `Default`; no `..` in a literal or
pattern) and both positive controls. Its compile-time companion in `app.rs`'s tests grows
from five fields to seven and gains a second test for `Filter`'s two.

## Contracts

New and changed public API on the library crate. Every consumer is inside this repository.

```rust
// src/ui/list.rs (new) — content, always parameterised by an interior width
pub enum RowKind {
    Problem,
    Item { index: usize },   // index into the visible list, not into ChangeSet
    Separator,
    Message,
}
pub struct Row { pub text: String, pub kind: RowKind, pub selected: bool }
pub fn rows(dashboard: &Dashboard, width: u16) -> Vec<Row>;

// src/ui/layout.rs — geometry, alongside mode / split_frame / split_body
pub fn viewport(rows: usize, cursor: usize, height: u16) -> usize;

// src/ui/app.rs
pub struct Filter { pub query: String, pub active: bool }   // no Default, ever
pub enum Action {
    Quit, OpenDetail, Back,                 // Back replaces BackToList
    SelectNext, SelectPrev,
    FilterStart, FilterPush(char), FilterPop,
    Ignore,
}
pub struct Dashboard {
    pub repo: Option<PathBuf>, pub searched_from: PathBuf, pub changes: ChangeSet,
    pub route: Route, pub quit: bool,
    pub selected: usize, pub filter: Filter,          // new
}
pub fn action_for(event: &Event, filtering: bool) -> Action;   // signature change
pub fn matches(name: &str, query: &str) -> bool;                // case-insensitive substring
impl Dashboard {
    pub fn visible(&self) -> Vec<&Change>;   // active-then-archived, query applied
    pub fn visible_len(&self) -> usize;      // what `apply` clamps `selected` against
}

// src/changes.rs (test-only)
#[cfg(test)] pub(crate) mod fixture {
    pub(crate) fn active(name: &str, completed: usize, total: usize) -> Change;
    pub(crate) fn archived(date: Option<&str>, name: &str, completed: usize, total: usize) -> Change;
    pub(crate) fn set(active: Vec<Change>, archived: Vec<Change>, problems: Vec<String>) -> ChangeSet;
}
```

**Additive, except three internal renames/signature changes** — `Action::BackToList` →
`Action::Back`, `action_for` gaining a parameter, and `Dashboard` gaining two fields. All
three are internal to this crate; `src/ui/driver.rs` is the only non-test call site and it
is updated in the same group. **Not breaking** in the sense `proposal.md` marks: the plugin
manifest, the `config.toml` format, and the exit statuses are untouched, and every key bound
here (`j`, `k`, arrows, `/`) is already documented in `SPEC.md` → Keys.

**Downstream consumers.** `detail-view` extends `Dashboard` and `render` further;
`agent-attribution` adds the badge to the row grammar; `live-refresh` replaces `changes`
between frames; `degraded-states` audits the message rows. Each extends rather than
replaces, exactly as they do `tui-shell`'s contracts.

## Persistence and Rollout

- **Migration:** none. No stored data, no schema, no on-disk format.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. `resolve::BinCache` is not consulted by this change.
- **Index rebuild:** none.
- **Authorization:** none. The plugin is a local read-only process; it opens no socket and
  writes nothing outside `HERDR_PLUGIN_STATE_DIR`, and this change writes nothing at all.
- **Observability:** none. A TUI has no stream to log to while it owns the screen.
- **Deployment impact:** a developer with the plugin linked runs `make build` to see rows
  instead of an empty frame. The manifest, the pane definitions, and `config.toml` are
  untouched, so no re-link and no re-install. No dependency is added, so `Cargo.toml`,
  `Cargo.lock`, and `tests/fixtures/build-graph.txt` are unchanged and `DEPS` /
  `GRAPH-SNAP` are re-run rather than edited.

## Test Boundaries

Every collaborator this change touches, in every tier. "Replaced" always names what
replaces it. **No task may introduce a collaborator absent from this table.**

| Dependency | In the acceptance test (`ui::tests::load::a_scratch_repository_renders_its_change_rows`) | In unit and view tests (`--lib`) | In command-level checks |
|---|---|---|---|
| The **filesystem** | **real**, through `crate::testutil::ScratchDir` under `std::env::temp_dir()` — this is the one test in the change that opens a directory, and it does so through `ui::load`, never through a view | **never reached by a view test.** Every test in `src/ui/view.rs` and `src/ui/list.rs` builds its `ChangeSet` in memory through `changes::fixture` and touches no directory. A view test that needed a real directory would mean logic leaked into the view, and `NOIO-VIEW` makes that unrepresentable. `ui::tests::load::`'s four existing tests keep their real `ScratchDir` | real, in `OPENSPEC-UNTOUCHED` and the graph checks |
| The **rendering surface** | **replaced** by `ratatui::backend::TestBackend`, at 60x20 and 120x20 | **replaced** by `TestBackend`, at 60 and 120 columns for every view scenario, plus 1, 2, 16, 18, 20, 99, 100, 101 and a 120x12 resize for boundary scenarios | **replaced** by `WIDTHS` (view tier) and `LISTWIDTHS` (row-grammar tier, 38 and 58) |
| The **terminal** (raw mode, alternate screen) | **never reached** — no `TerminalOps` is constructed | **never reached** — no test in this change constructs `CrosstermOps` or names a crossterm mode function | **replaced** by `NORAW-GREP`, carried forward unchanged |
| The **event stream** | not reached | **replaced**: `action_for` is called directly with constructed `Event` values; `ui::driver`'s one new test uses the existing scripted `EventSource` double | — |
| The **`openspec` binary** | **absent by construction** — `ui::load` takes no `OpenspecCli`, and the acceptance fixture writes no binary | **absent by construction**; `ui::list` and `ui::view` take no CLI argument | **replaced** by `NOCLI-SHELL`, whose `UI_MIN` rises from the measured 7 to 8 |
| The **Herdr socket / `herdr` binary** | not reached | not reached — Phase 5 | — |
| The **process environment** | **replaced** by the `Config` value the test constructs; `ui::load` takes a `&Config` and never reads the environment itself | not read by any pure module in this change | — |
| The **wall clock** | not read | not read — `viewport` and `rows` take their inputs as arguments | — |
| The **process's own binary** | not used | not used — this change adds no `tests/cli.rs` case; the binary's behaviour is unchanged | — |
| **`cargo`, `git`, `python3`, and the POSIX shell utilities `find`, `grep`, `xargs`, `sort`, `sed`, `awk`, `cut`, `tr`, `wc`, `diff`, `printf`, `mktemp`, `cp`, `rm`, `mkdir`** | — | — | **real**, in the command-level checks only (`NOSPAWN-GREP`, `NOIO-VIEW`, `NOCLI-SHELL`, `NORAW-GREP`, `NODEFAULT-UI`, `NOLIT-CHANGE`, `GATE-MECH1`, `NOJSON-SEAM`, `DEPS`, `GRAPH-SNAP`, `WIDTHS`, `LISTWIDTHS`, `NOWAIVER`, `OPENSPEC-UNTOUCHED`, `TESTCOUNT`). The enumeration is exhaustive on purpose: a new tool in a check block is then a reviewable addition here rather than a silent one |
| **crates.io** | — | — | **real**, only in so far as `DEPS` and `GRAPH-SNAP` re-resolve the unchanged lockfile |
| The **`openspec` CLI as a planning tool** (nvm-installed, not on the default `PATH`) | not reached | not reached | **real**, in exactly one place: `openspec validate list-view --strict` in the final group. It reads the change's own artifacts and writes nothing to the crate. This is a distinct row from "the `openspec` binary" above, which is about the *plugin* consulting it at runtime and stays absent by construction |

## Test Strategy

Three tiers, all already in this repository.

- **Unit / view (`cargo test --all-features --lib`)** — everything. `ui::list`'s row
  grammar, filter predicate and viewport as plain-data unit tests at widths 38 and 58;
  `ui::app`'s `action_for` and `apply` as pure key tests under both filter modes;
  `ui::view`'s rendering through `TestBackend` at 60 and 120; and the one composition test
  in `ui::tests::load::`.
- **Command-level checks** — the architectural invariants a Rust test cannot express, each
  with a positive control and an existence guard, each **extracted to `$CHECKS/<LABEL>.sh`
  in task 1.1 and proven able to fail against a planted violation** in task 9.2 (with its
  existence guards and positive controls proven to fire earlier, in task 1.3).
- **Gates** — `make check`, unchanged, at the unchanged 80% floor.

**The outer loop is taken, at the `ui::` composition tier rather than through the binary.**
The end-to-end risk here is not process wiring — `main` → `ui::run` is unchanged and already
covered — it is that a real repository on disk turns into real cells on screen. `ui::load` →
`changes::from_files` → `ui::list::rows` → `ui::view::render` is a path no single unit test
crosses. Group 0 writes `ui::tests::load::a_scratch_repository_renders_its_change_rows`
first, RED, and group 8 closes it. That test uses a real `ScratchDir` **and** a
`TestBackend`; it lives in `src/ui/mod.rs`, deliberately **not** in `src/ui/view.rs`, and it
does not weaken the render seam: `render` still receives a `Dashboard` value, never a path,
and `NOIO-VIEW` proves `view.rs` and `list.rs` name no filesystem API. A `tests/cli.rs`
case would add nothing — the binary refuses to start without a terminal and this change does
not alter that.

### Why each behaviour sits where it does

- **Row text is asserted twice, at two tiers, on purpose.** `ui::list::rows` returns a
  `String` per row, so the grammar is asserted directly at widths 38 and 58 with no buffer
  in the way; `ui::view` then asserts the same strings *as cells* at 60 and 120, which is
  what proves they were placed at the interior's first column and row and not somewhere
  else. Asserting only the first would let a mis-placed `set_string` pass; asserting only
  the second makes every grammar failure surface as a buffer diff.
- **`viewport` is a free function in `ui::layout`, not a method on `Dashboard`.** It takes three numbers and returns one,
  so its boundaries (`rows <= height`, `height == 0`, the clamp at `rows - height`) are
  asserted as a table rather than by rendering seven dashboards.
- **The selected row's style is asserted cell by cell, not by reading a `Style` off a
  widget.** `Modifier::BOLD` on the selected row and its absence on the neighbouring row are
  two assertions in the same test, so the test discriminates rather than asserting a
  constant — the same shape `routed_region_border_is_bold` already uses.
- **`Filter` gets the same gate as `Dashboard`.** A second state type in `src/ui/app.rs`
  outside `NODEFAULT-UI`'s reach would quietly reintroduce exactly the silent-default risk
  the gate exists to close, and extending the check to a type list costs one loop.
- **The 30-change and 17-change fixtures are generated, not written out.** A fixture with
  thirty hand-written rows would be unreadable and its expected strings unmaintainable; the
  tests build `change-00`..`change-29` in a loop and assert the *first* and *last* drawn row
  plus the marker position, which is what the viewport rule actually claims.

### Verification matrix

One row per spec scenario. `--lib` filters are **counted**, never trusted to a bare exit
status: `cargo test <filter>` exits 0 when the filter matches nothing. `testcount` is the
sourced helper carried forward from `tui-shell`.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| change-rows: Active rows render at both mandated widths | `view::list_rows_render_at_60_and_120` — the three exact 38- and 58-column strings, plus rows 5–17 blank | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 47` |
| change-rows: The row grammar places the marker, the name, and the progress cell | `list::active_row_grammar_at_38_and_58`, `list::progress_cell_is_dash_when_total_is_zero`, `list::every_row_is_exactly_the_requested_width`, and `list::the_selected_flag_marks_exactly_the_selected_change` | unit | none | `testcount --lib 'ui::list::tests::' 17` |
| change-rows: A name too long for the field is truncated with an ellipsis | `list::a_long_name_is_truncated_with_an_ellipsis` and `view::a_long_name_is_truncated_at_38_but_not_at_58` | unit + view | `TestBackend` for the second | both filters above |
| change-rows: A field too narrow for both drops the progress cell whole | `list::a_narrow_width_drops_the_progress_cell_whole` — widths 10, 9, 8, 1, 0, 38, 58 | unit | none | `testcount --lib 'ui::list::tests::' 17` |
| change-rows: The separator and archived rows render at both mandated widths | `list::archived_rows_carry_a_ten_column_date_field`, `list::an_undated_archived_row_uses_ten_spaces`, `list::the_separator_fills_the_width`, `list::every_row_is_exactly_the_requested_width`, and `view::separator_and_archived_rows_render_at_both_widths` | unit + view | `TestBackend` for the last | both filters |
| change-rows: No archived changes means no separator | `list::the_separator_is_emitted_only_when_archived_rows_follow` and `view::no_archived_changes_means_no_separator` | unit + view | `TestBackend` | both filters |
| change-rows: No repository names the directory searched, at both widths | `list::the_no_repository_block_is_three_rows` and `view::no_repository_names_the_directory_searched` | unit + view | `TestBackend` | both filters |
| change-rows: A repository with no changes at all | `list::the_four_message_states_are_distinct`, `list::rows_never_panic_at_any_width`, and `view::a_repository_with_no_changes_says_so` | unit + view | `TestBackend` | both filters |
| change-rows: No active changes with archived ones still browsable | `view::no_active_changes_keeps_archived_browsable` | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 47` |
| change-rows: Repository-level problems are named above the rows | `list::problem_rows_are_truncated_to_the_width` and `view::repository_problems_are_named_above_the_rows` | unit + view | `TestBackend` | both filters |
| change-rows: The detail region stays blank while the list fills | `view::the_detail_region_stays_blank_while_the_list_fills` and `list::rows_preserve_the_change_set_order` | view + unit | `TestBackend` | view and list filters |
| change-rows: The narrow detail route draws no rows | `view::the_narrow_detail_route_draws_no_rows` | view | `TestBackend` | view filter |
| change-rows: More changes than rows do not overflow the region | `view::more_changes_than_rows_do_not_overflow` | view | `TestBackend` | view filter |
| dashboard-loop: `Dashboard` has no `Default` and no site elides a field | `NODEFAULT-UI` over the type list `Dashboard Filter`, with both positive controls and three planted-violation controls, plus `app::tests::dashboard_destructures_into_exactly_seven_fields` and `app::tests::filter_destructures_into_exactly_two_fields` | command + unit | `find`, `grep` real | `sh $CHECKS/NODEFAULT-UI.sh`; `testcount --lib 'ui::app::tests::' 22` |
| dashboard-loop: The pure view files name no I/O API | `NOIO-VIEW` with its five-file existence guard and `terminal.rs` positive control | command | `grep` real | `sh $CHECKS/NOIO-VIEW.sh` |
| dashboard-loop: The shell never names the CLI seam | `NOCLI-SHELL` at `UI_MIN=8` with its `src/changes.rs` positive control | command | `find`, `grep` real | `sh $CHECKS/NOCLI-SHELL.sh` |
| dashboard-loop: Change literals live only in the gated file | `NOLIT-CHANGE` with its `src/changes.rs` positive control and its planted-violation control | command | `find`, `grep` real | `sh $CHECKS/NOLIT-CHANGE.sh` |
| dashboard-loop: Both quit keys quit and neither near-miss does | `app::quit_keys_and_their_near_misses` — five events under both filter modes | unit | none | `testcount --lib 'ui::app::tests::' 22` |
| dashboard-loop: A released quit key does not quit | `app::only_press_kind_acts` — Release, Repeat, Press under both modes | unit | none | same filter |
| dashboard-loop: Enter and Esc move between the two routes | `app::enter_and_esc_map_to_routes` | unit | none | same filter |
| dashboard-loop: `Esc` dismisses one layer at a time | `app::esc_dismisses_one_layer_at_a_time` — four `Back` actions from Detail + active filter, and the inactive-query variant | unit | none | same filter |
| dashboard-loop: Navigation and filter keys are distinguished from near misses | `app::navigation_and_filter_keys_are_distinguished` — eight events | unit | none | same filter |
| dashboard-loop: Non-key events are ignored without panicking | `app::non_key_events_are_ignored` — five events under both modes | unit | none | same filter |
| dashboard-loop: A scratch repository is loaded from disk with no binary present | `load::a_scratch_repository_is_loaded_from_files`, extended to assert `selected` 0 and an empty inactive `filter` | unit | real filesystem via `ScratchDir` | `testcount --lib 'ui::tests::load::' 5` |
| dashboard-loop: No repository above the starting directory | `load::no_repository_above_the_start` with its measured ancestor precondition | unit | real filesystem via `ScratchDir` | same filter |
| dashboard-loop: The configured archived count is passed through | `load::archived_count_from_config_is_honoured` | unit | real filesystem via `ScratchDir` | same filter |
| dashboard-loop: Loading writes nothing | `load::loading_writes_nothing` comparing two `testutil::snapshot` values | unit | real filesystem via `ScratchDir` | same filter |
| list-filtering: `/` starts filter mode from either route | `app::slash_starts_filter_mode_and_routes_to_list` and `view::slash_starts_filter_mode_and_the_list_is_shown` | unit + view | `TestBackend` | app and view filters |
| list-filtering: `q` types a character while filtering and does not quit | `app::filter_mode_types_printable_characters` and `app::ctrl_c_quits_from_filter_mode` | unit | none | `testcount --lib 'ui::app::tests::' 22` |
| list-filtering: `q` types a character while filtering and does not quit | `driver::filter_mode_is_passed_to_action_for` — a script of `/`, `q`, then `Ctrl-C`: the loop does not quit on the `q`, the query ends as `q`, and the summary is `frames: 3, polls: 3` | view + unit | `TestBackend`, scripted `EventSource` | `testcount --lib 'ui::driver::tests::' 8` |
| list-filtering: Backspace deletes, and on an empty query is inert | `app::backspace_deletes_and_is_inert_on_empty` | unit | none | app filter |
| list-filtering: `Esc` cancels the filter and `Enter` accepts it | `app::enter_accepts_the_filter_without_opening_detail` and the `Esc` half of `app::esc_dismisses_one_layer_at_a_time` | unit | none | app filter |
| list-filtering: A query narrows both tiers at both widths | `app::visible_applies_the_query_to_both_tiers` and `view::a_query_narrows_both_tiers` | unit + view | `TestBackend` | list and view filters |
| list-filtering: Matching ignores case | `app::matches_is_case_insensitive_substring_on_the_name` and `view::matching_ignores_case` | unit + view | `TestBackend` | list and view filters |
| list-filtering: A query matching only an archived change | `view::a_query_matching_only_an_archived_change` | view | `TestBackend` | view filter |
| list-filtering: A query matching nothing names itself | `view::a_query_matching_nothing_names_itself` | view | `TestBackend` | view filter |
| list-filtering: Shrinking the visible list clamps the selection | `app::selection_clamps_when_the_filter_shrinks_the_list` and `view::shrinking_the_visible_list_clamps_the_marker` | unit + view | `TestBackend` | app and view filters |
| list-filtering: The prompt replaces the hints while filtering, at both widths | `view::the_prompt_replaces_the_hints_while_filtering` | view | `TestBackend` | view filter |
| list-filtering: An accepted query leads the hint list, at both widths | `view::an_accepted_query_leads_the_hint_list` | view | `TestBackend` | view filter |
| list-filtering: A prompt longer than the footer keeps its tail | `view::a_prompt_longer_than_the_footer_keeps_its_tail` | view | `TestBackend` | view filter |
| list-selection: The first change is selected on startup at both widths | `view::the_selected_row_carries_the_marker_and_bold` | view | `TestBackend` | view filter |
| list-selection: `j`, `k`, and the arrows move the selection | `app::navigation_and_filter_keys_are_distinguished` for the mapping and `view::navigation_moves_the_marker` for the frame | unit + view | `TestBackend` | app and view filters |
| list-selection: Selection clamps at both ends rather than wrapping | `app::select_next_and_prev_clamp` and `view::selection_clamps_at_both_ends_on_screen` | unit + view | `TestBackend` | app and view filters |
| list-selection: Selection crosses the separator into the archived rows | `view::selection_crosses_the_separator` | view | `TestBackend` | view filter |
| list-selection: Navigation over an empty visible list is inert | `app::apply_never_leaves_selected_out_of_range` and `view::an_empty_visible_list_draws_no_marker` | unit + view | `TestBackend` | app and view filters |
| list-selection: A selection past the interior scrolls the slice at both widths | `view::a_selection_past_the_interior_scrolls_the_slice` | view | `TestBackend` | view filter |
| list-selection: The last change is reachable and the slice stops at the end | `view::the_last_change_is_reachable` | view | `TestBackend` | view filter |
| list-selection: The viewport is exact at its boundaries | `layout::viewport_is_zero_when_everything_fits`, `layout::viewport_centres_and_clamps`, `layout::viewport_boundaries_are_exact`, and `view::the_viewport_boundary_is_rendered` | unit + view | `TestBackend` for the last | `testcount --lib 'ui::layout::tests::' 9` and the view filter |
| list-selection: A resize changes the slice on the next frame | `view::resizing_changes_the_slice_on_the_next_frame` — one `Terminal<TestBackend>` resized 120x20 → 120x12 between draws | view | `Terminal<TestBackend>` | view filter |
| responsive-layout: Header, body, and footer occupy their rows at both widths | `view::frame_rows_at_60_and_120`, unchanged | view | `TestBackend` | view filter |
| responsive-layout: A one-row frame renders the header and nothing else | `view::one_row_frame_draws_header_only`, unchanged | view | `TestBackend` | view filter |
| responsive-layout: A two-row frame renders the header and the footer with no body | `view::two_row_frame_draws_no_body`, unchanged | view | `TestBackend` | view filter |
| responsive-layout: A one-column frame renders without panicking | `view::one_column_frame_does_not_panic`, **extended** with a 2x20 render so a zero-column interior is exercised | view | `TestBackend` | view filter |
| responsive-layout: The footer drops whole hints rather than truncating one | `view::footer_drops_whole_hints`, unchanged | view | `TestBackend` | view filter |
| responsive-layout: The routed region's border is bold and the other's is not | `view::routed_region_border_is_bold`, unchanged | view | `TestBackend` | view filter |
| responsive-layout: Interiors are blank at both widths | `view::region_interiors_are_blank`, **rewritten**: the detail interior is still blank cell-by-cell, the list interior now begins `No changes yet`, and rows 3–17 of the list interior are blank | view | `TestBackend` | view filter |
| responsive-layout: Rows do not overwrite the borders at either width | `view::rows_do_not_overwrite_the_borders` | view | `TestBackend` | view filter |
| change-rows: An archived row drops the progress cell, then the date, as the width falls | `list::archived_narrow_widths_drop_progress_then_date` — widths 20, 19, 14, 13, 3, 1, 0, plus 38 and 58 as controls, each asserted as an exact string **and** as an exact character count | unit | none | `testcount --lib 'ui::list::tests::' 17` |
| responsive-layout: A path that fits is right-aligned whole at both widths | `view::header_path_right_aligned_whole`, unchanged | view | `TestBackend` | view filter |
| responsive-layout: A path too long for the narrow header is shortened from the left | `view::header_path_shortened_from_the_left`, unchanged — it survives because the list interior holds `No changes yet`, which carries no `…` | view | `TestBackend` | view filter |
| responsive-layout: A header too narrow for any path shows only the label | `view::header_omits_the_path_when_too_narrow`, unchanged — at 16 columns the list interior is 14 wide and `No changes yet` is exactly 14 characters, so the body adds no `…` either | view | `TestBackend` | view filter |
| responsive-layout: No repository found is named in the header at both widths | `view::header_says_no_repository`, **rewritten**: its buffer-wide `!contains("/tmp/searched-from")` narrows to row 0, because the body's no-repository block now names that path on purpose | view | `TestBackend` | view filter |
| *(cross-cutting)* the whole change | `make check` — format, lint, `cargo test --all-features`, `cargo llvm-cov --fail-under-lines 80` — plus `NOSPAWN-GREP` at `MIN=16`, `NORAW-GREP`, `WIDTHS`, `LISTWIDTHS`, `NOWAIVER`, `GATE-MECH1`, `NOJSON-SEAM`, `DEPS`, `GRAPH-SNAP`, and `OPENSPEC-UNTOUCHED` | gate + command | as tabled above | `make check`; the check scripts |

## Decisions

**The row grammar is fixed-field and right-aligned, written against 38 columns.** A row is
`[marker][space][name field][space][progress]`, with the progress cell right-aligned so its
last character occupies the interior's final column, and the name field absorbing every
remaining column. At 38 with a `[4/9]` cell the name field is 30 columns; at 58 it is 50.
The `[-]` cell for a change with no tasks is three columns, so its name field is 32 and 52 —
the cell still ends in the final column, which is what makes a column of progress cells read
as a column. *Alternative considered:* a fixed name column with the progress floating after
it — rejected, because at 38 columns a fixed name width either wastes columns on short names
or truncates `migrate-ai-sdk-v7` while `[7/7]` sits in the middle of the row.

**Names truncate from the right; paths truncate from the left.** A change name is
identified by its head (`markdown-viewer` against `markdown-renderer`), a repository path by
its tail (its own directory name) — which is why `tui-shell` already shortens the header path
from the left. Both use `…` as the marker of what was dropped. The two rules share one
helper, `shorten_left`, with the name path calling a right-truncating sibling;
`shorten_for_header` is refactored onto the shared one in the same group, so there is one
implementation of each direction rather than three near-copies.

**A cell that does not fit is dropped whole, never cut short — and the drop order is
stated, not left to the implementation.** The progress cell goes first, as soon as the name
field would fall below one column; the ten-column date field and its following space go
second, when the name field would *still* fall below one column; the row then degenerates to
the active grammar, and below two columns it is the first `width` characters of `> `. Two
consequences are worth writing down because an off-by-one here is invisible at 38 and 58: a
drop reclaims the **separating space** as well as the cell, so the archived name field is
`width − 14 − len(progress)` with the progress cell and `width − 13` without it, not
`width − 14` in both; and every branch must still produce a row of exactly `width`
characters, which `list::every_row_is_exactly_the_requested_width` asserts over every fixture
in the module. This is the rule `responsive-layout`'s footer already uses for hints, and
applying it consistently is what makes the sub-20-column boundaries assertable as exact
strings rather than as "something reasonable".

**Archived rows carry a progress cell, departing from `SPEC.md`'s mock.** The mock's
archived row is `  2026-08-14 add-auth` with no progress. Two grammars means two truncation
rules, two boundary sets, and two places for a later change to forget one; one grammar with
an inserted ten-column date field is strictly less code and strictly more information — an
archived change is not necessarily complete. `SPEC.md`'s mock is corrected rather than
followed. *Alternative considered:* keep the mock exactly — rejected for the reason above,
and recorded as a spec correction rather than a silent divergence.

**No width is reserved for the agent badge.** `SPEC.md`'s mock shows a third column
(`> claude - working`, `done`, `schema unknown`). Reserving columns for it now would show a
permanent blank gutter in a 38-column interior for two phases, and `agent-attribution` would
inherit a width nobody measured. Instead the name field flexes: `agent-attribution` will
subtract a badge field from it exactly as the progress cell is subtracted today, and the row
grammar's arithmetic is written as a sequence of subtractions from the width so that adding
one more is a one-line change. `SPEC.md` is annotated to say so.

**Repository-level `ChangeSet::problems` render here; per-change `Change::problems` do
not.** A repository-level problem — an unreadable `openspec/changes/` — means the list itself
is incomplete, and nothing else in the plugin would ever surface it; leaving it unrendered
would make `degraded-states`' "audit, do not add" principle false in Phase 6. A per-change
problem belongs in the third mock column that `agent-attribution` owns, so it waits for the
change that decides that column's width. This is the one deliberate widening of the roadmap
row's scope, and it is one `Row` variant and two scenarios.

**`action_for` takes `filtering: bool`; `apply` owns the state machine.** The alternative,
`action_for(&Dashboard, &Event)`, would make a pure event→intent map depend on the whole
state and would put the layered-`Esc` decision in the same function as the key table. Keeping
`action_for` a table means its total-over-events property stays a table-shaped test, and the
layering (`filter mode` → `query` → `route` → nothing) is asserted once, in `apply`.
*Alternative considered:* two functions, `action_for_list` and `action_for_filter` —
rejected: the caller would then hold the mode branch, and `run_loop` is not where a key
policy belongs.

**`Esc` is a layered dismissal, refining `SPEC.md`'s "inert at the list root".** The landed
sentence is true only once there is nothing to dismiss. The new rule dismisses exactly one
layer per press and is still inert at the root, so nothing that was true stops being true;
`SPEC.md` is corrected to say which layers exist.

**Selection clamps; it does not wrap.** Wrapping from the last archived change back to the
first active one, across a separator, in a 16-row window, is disorienting and makes "am I at
the end?" unanswerable without counting. Clamping makes the end of the list a fact the user
can feel. *Alternative considered:* wrap — rejected.

**The viewport is derived on every draw and centres the selection.** The interior height is a
property of the current frame, exactly like `LayoutMode`, so storing an offset on `Dashboard`
would create the stale-after-resize bug the breakpoint's own design exists to avoid — and
`render` takes `&Dashboard`, so it could not update a stored one anyway. Deriving it forces a
rule that is a pure function of `(rows, cursor, height)`. Centre-then-clamp is **not** the
only such rule — it is the one chosen — and the trade-off is real: the list scrolls by one
on most keypresses rather than only at the edges. It is chosen because it keeps context on
both sides of the selection, which is what makes a 16-row window readable while moving.
*Alternatives considered:* store the offset on `Dashboard` and clamp it in `apply` —
rejected, because `apply` does not know the height either; the height reaches only
`render`. A page-anchored rule, `min((cursor / height) * height, rows - height)`, is
equally pure and scrolls once per page instead of once per keypress — rejected here
because it puts the selection at the top of the window immediately after a page turn and
gives no leading context, but it is the obvious thing to try if the per-keypress scroll
proves annoying in real use, and `live-refresh` — which replaces the `ChangeSet` under the
selection — is the change most likely to want the steadier viewport.

**`ui::list` returns plain `String`s, not ratatui `Line`s.** Styling is `view.rs`'s job: it
applies `Modifier::BOLD` to the selected row. If `list.rs` returned styled lines, the row
grammar's tests would compare span vectors instead of strings, and `list.rs` would import
`ratatui::style`, which is the first step towards a view that computes rather than draws.
*Alternative considered:* return `Vec<Line<'static>>` — rejected.

**`Change` fixtures live in `src/changes.rs`, behind `#[cfg(test)]`.** `change-model`'s
`GATE-MECH1` searches `src/changes.rs` for a `Default` or a `..`; a fixture builder in
`src/lib.rs`'s `testutil` would be a construction site outside it, quietly widening what the
gate does not cover — the same argument `tui-shell` used to put `changes::empty_set()` there.
`NOLIT-CHANGE` is added so the placement is checked rather than remembered.

**No `/ filter` hint is added to the default footer.** Adding a fourth default hint would
rewrite four landed `responsive-layout` scenarios' exact expected strings (the 18-, 20-,
60-, and 120-column footers) for a discoverability gain, and the filter announces itself the
moment it is used. Recorded as an open question for `degraded-states`, which owns the footer
audit. *Alternative considered:* add it and rewrite the four — rejected as churn in a landed
capability for no behavioural gain.

**`WIDTHS` gains a sibling, and the modules are split so that sibling needs no exemption
list.** `WIDTHS` reads `src/ui/view.rs` for `60` and `120` because those are *frame* widths.
`LISTWIDTHS` is the same script pointed at `src/ui/list.rs` for `38` and `58`, the two
*interior* widths, with its own floor and its own positive control. For that to be an
honest check, **every** function in `src/ui/list.rs` must be parameterised by a width —
which is why `viewport` lives in `ui::layout` beside `mode` and `split_frame` (it is
geometry: three numbers in, one out, no width) and why `matches`, `Dashboard::visible`, and
`Dashboard::visible_len` live in `ui::app` beside the state they read. A `list.rs` holding
width-free functions would have forced either meaningless `38` and `58` literals into tests
that have no width, or an exemption list — and `tui-shell` recorded that an exemption list
is how a width check rots into a rubber stamp. Placing them this way has a second payoff:
`app.rs` does not depend on `list.rs`, so there is no module cycle for `apply`'s clamp.
*Alternative considered:* one `list.rs` holding everything with a `viewport_`-prefixed
exemption — rejected for both reasons above.

**Every check's floor is measured at `main` before it is raised.** `NOSPAWN-GREP`'s `MIN`,
`NOCLI-SHELL`'s `UI_MIN`, `WIDTHS`' test floor, and every `testcount` minimum are read off
the tree in group 1 and recorded, then set to *measured + this change's addition*. This
repository has already shipped one check whose floor was asserted rather than measured and
was therefore unpassable; group 1's measure-first task is the response.

### `SPEC.md` corrections this change makes

Recorded here and executed in tasks.md group 10, with before/after logged in `planning-review.md`:

1. **List view mock** — replaced with the real 38-column rendering plus the row grammar in
   prose, since the landed mock's 48-character rows cannot occur.
2. **Archived row** — gains a progress cell in the mock, per the decision above.
3. **The third column** — annotated as `agent-attribution`'s and `degraded-states`', with
   the explicit statement that `list-view` reserves no width for it.
4. **Keys table, `/`** — gains the filter-mode semantics: printable keys type, `Backspace`
   deletes, `Enter` accepts, `Esc` cancels, `Ctrl-C` still quits, and **`q` does not quit
   while filtering**, which the landed table's unqualified `q → Quit` row contradicts.
5. **Keys table, `j`/`k`/arrows** — "Navigate" is made concrete: they move the list
   selection, clamped rather than wrapping, and the list scrolls to keep it visible.
6. **`Esc` at the list root** — refined to the layered dismissal.
7. **Degraded states** — a row is added for a filter matching no change.
8. **Degraded states** — the `ChangeSet::problems` rows gain the sentence naming *where*
   the reason is rendered, which was previously unassigned to any change.
9. **List view** — the sentence "A footer reports agents in the repository that could not
   be attributed to a change" is annotated as `agent-attribution`'s, so its absence here is
   not read as an omission.
10. **Keys table, `q`** — the row reads `q | Quit`, unqualified, which correction 4's own
    text calls out as contradicted by filter mode. Correction 4 rewrites the `/` row; this
    one rewrites the `q` row, because a reader consulting the table for `q` never reaches
    the `/` row.
11. **Keys table, `Enter`** — `Enter | Open change detail` is false while filtering, where
    it accepts the query and opens nothing.
12. **Keys table, `Esc`** — `Esc | Back to list` is false while filtering and false with a
    query set. Corrected in the row itself, not only in the sentence below the table, which
    correction 6 rewrites — leaving the two disagreeing inside one section is worse than
    leaving both stale.
13. **List view, "Progress comes from the CLI when available and from checkbox counts
    otherwise"** — false of what this change renders. `NOCLI-SHELL` mechanically forbids
    `src/ui/` naming the CLI seam, so the pane's list is file-sourced at every width until
    `live-refresh` wires the correction in. The sentence gains that qualification rather
    than being deleted: the dual-source model is still the design, it is just not yet
    reachable from the view.
14. **Responsive layout, "`Enter` and `Esc` move the route at **every** width"** — the
    sentence `tui-shell` explicitly froze *for `list-view` to inherit*. It is falsified in
    filter mode, where `Enter` accepts and `Esc` cancels without touching the route, and
    incomplete now that `/` also moves the route to `List`. Corrected in place, keeping the
    `Length(40)` / `Min(0)` half of the freeze untouched.
15. **Testing and quality gates → Unit-tested modules** — the `ui` bullet lists
    `ui::layout`, `ui::app`, `ui::view`, `ui::driver`, `ui::terminal`, and `ui::load`.
    `ui::list` is added: it gains 17 tests and a check of its own.
17. **Architecture → Module map, the `ui` row** — "Views, layout, key handling, terminal
    lifecycle, and the event loop" gains the row grammar and the dashboard's own state.
16. **Fixtures, "every view change performs no I/O at all"** — narrowed to "every view
    *test*". The claim is already false of `tui-shell`, whose `ui::tests::load::` tests open
    a `ScratchDir`, and this change's acceptance test does the same on purpose. The render
    seam is unaffected and says so: `render` takes a value, never a path, and `NOIO-VIEW`
    proves `view.rs` and `list.rs` name no filesystem API.

## Risks / Trade-offs

- **The row grammar is asserted as exact 38- and 58-character strings, so a one-column
  change to the layout reddens a dozen tests at once.** → That is the intent: the interiors
  are frozen by `responsive-layout`, and a change to them should be a deliberate, visible
  diff. The two widths are derived in one helper in the tests, so the *expectations* are
  written out but the geometry is not re-derived per test.
- **The centre-then-clamp viewport scrolls the list on most keypresses.** → Accepted, with
  the alternative (a stored offset) rejected above for a stronger reason. `detail-view` or a
  later polish change can revisit it if it proves annoying in real use; the rule is one
  function with a table-shaped test.
- **`WIDTHS` and `LISTWIDTHS` are heuristics — they read test source for literals.** → Both
  are floors, not proofs: a `60` in a comment satisfies `WIDTHS`. Each is paired with a
  `testcount` minimum, which is what proves the tests exist and run, and with per-scenario
  tasks that name the exact cells asserted at each width.
- **Adding two `Dashboard` fields breaks roughly thirty landed tests at once.** `..` is
  forbidden by `NODEFAULT-UI`, so every `Dashboard` literal in the crate must name the two
  new fields in the same commit: six sites, in `src/ui/app.rs`, `src/ui/mod.rs` (two, inside
  `load`), `src/ui/view.rs`, `src/ui/driver.rs`, and `src/lib.rs`'s `testutil`. → Group 2
  lists all six explicitly and its contract gate greps for `Dashboard {` as well as for
  `action_for(`, because neither `view.rs` nor `lib.rs` names `action_for` and a gate that
  looked only for the call site would miss two thirds of the sites.
- **`ui::view::render` grows to roughly twice its current length.** → The row composition is
  in `ui::list`, so `view.rs` gains a draw loop and two footer forms, not the grammar. If it
  grows further, `detail-view` is the change that should split it.
- **The `#[cfg(test)]` fixture module in `src/changes.rs` is instrumented by
  `cargo llvm-cov`.** → It is small and every constructor is called by the tests that need
  it; a fixture test asserting `conformance::assert_invariants` over each builder keeps it
  from drifting into dead code.
- **Repository-level problems are rendered here rather than in `degraded-states`.** → A
  deliberate, argued widening of the roadmap row; `IMPLEMENTATION-ORDER.md`'s `list-view`
  row is updated at archive time to say so, per the schema's archive guidance.

## Migration Plan

None required. No data, no format, no deployment order. A developer with the plugin linked
runs `make build` to replace the empty list frame with rows; the manifest, the pane
definitions, and the configuration file are untouched, so no re-link and no re-install
happens. Rollback is `git revert` of the change's commits followed by `make build`.

## Open Questions

None blocking. Three decisions are deliberately deferred with their resolution point already
recorded:

- Whether the footer should carry a permanent `/ filter` hint. Deferred to
  `degraded-states`, which owns the footer and key-hint audit.
- Whether `j` / `k` should rebind to scrolling the detail pane while `route` is
  `Route::Detail`. This change moves the list selection at both routes, which is right in
  wide mode and arguable in narrow. Deferred to `detail-view`, which introduces scrollable
  detail content and is the first change with a competing claim on those keys.
- Whether `action_for`'s second parameter should be an `InputMode` enum rather than a
  `bool`. A second mode is genuinely foreseeable — `detail-view`'s own open question above
  names the `j` / `k` collision — and a second `bool` parameter would be the wrong answer
  to it. It stays a `bool` here because this change has exactly two modes and speculative
  generality in a total key map is how a key table stops being readable; `detail-view` is
  the change that will have a real second axis and should widen the parameter then.
- Whether the selected change should be remembered by name across a filter change rather
  than by index. Index-with-clamp is specified here; name-tracking becomes worth its
  complexity only once `live-refresh` can replace the `ChangeSet` under the selection, and
  that change is where it belongs.

## Visual Design

Not applicable. This change builds a terminal view and no design source — no HTML mockup,
no design file — exists for it. The visual contract is `SPEC.md` → List view and
→ Responsive layout, transcribed into `change-rows`, `list-selection`, and `list-filtering`
as exact cell positions and exact row strings, with the nine corrections listed above
applied to `SPEC.md` as part of this change.
