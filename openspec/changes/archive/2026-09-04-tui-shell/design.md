## Context

Nine changes have landed. The crate reads configuration, resolves the repository root and
the `openspec` binary, parses schemas and task files, enumerates `openspec/changes/` into
`ChangeSet` values, and corrects them from the CLI behind the subprocess seam. Line
coverage at `main` is 98.93% over 7,635 lines, with 82 uncovered — the untestable residue
`subprocess-seam` and `plugin-config` argued for.

What is missing is the other seam. `SPEC.md` → Architecture names two: `cli`, the only
module permitted to spawn, and the render seam, where "views are pure functions from a
`Dashboard` state value to a ratatui frame". The second one does not exist yet.
`herdr-openspec ui` prints a banner and blocks on stdin.

This change builds the render seam and the shell that sits on it: terminal lifecycle,
event loop, quit, the 100-column breakpoint from `SPEC.md` → Responsive layout, and the
`TestBackend` harness that `list-view`, `markdown-viewer`, `detail-view`, `tasks-tab`, and
`degraded-states` all render into. It is the Phase 4 `tui-shell` row of
`openspec/IMPLEMENTATION-ORDER.md`, whose dependency is `changes-from-files` and
deliberately **not** `changes-from-cli`.

Constraints that shape everything below:

- A Herdr split pane is frequently 40–60 columns (`PRD.md` → Constraints).
- The plugin must never write inside `openspec/` (`PRD.md` → Non-goals).
- The 80% coverage floor is enforced and never waived (`quality-gates`).
- `cargo test` runs in parallel threads of one process, and this repository's tests spawn
  the crate's own binary. Anything that puts a *real* terminal into raw mode during
  `cargo test` corrupts the developer's session, so the design must make that unreachable
  rather than merely unlikely.

## Goals / Non-Goals

**Goals**

1. `herdr-openspec ui` opens a usable, quittable dashboard in a Herdr pane with no
   `openspec` binary installed.
2. The terminal is restored on every exit path — normal return, error return, and panic.
3. The 100-column breakpoint is a pure function of the frame area, verified at 60 and 120
   columns and at 99/100/101.
4. A `TestBackend` harness exists in `crate::testutil` that every later view change uses
   unchanged.
5. Nothing in `src/ui/` names a process-spawn API or the CLI seam, and the four pure files
   name no I/O API at all — all three checked mechanically, each with a positive control.

**Non-Goals**

- Change rows, selection, filtering, artifact tabs, markdown rendering, progress bars, the
  "no repository" and "no changes" **empty states** — `list-view`, `markdown-viewer`,
  `detail-view`, and `tasks-tab` own those. The body regions here are empty bordered
  frames, and their interiors are asserted blank so a later change adding content is a
  visible diff.
- The CLI path, the watcher, the debounce, the worker thread, the `r` key — `live-refresh`.
- Agents, the Herdr socket, action keys — Phase 5.
- Scrolling, mouse support, colour themes, configurable keys.

## Boundaries

| Module | Change | Pattern followed |
|---|---|---|
| `src/ui/mod.rs` (new) | `load`, `enter_if_terminal`, `run`, `StartError`; re-exports | Composition root, like `config::load_from_env` |
| `src/ui/app.rs` (new) | `Dashboard`, `Route`, `Action`, `action_for`, `Dashboard::apply` | Pure total transformation, like `tasks::parse` |
| `src/ui/layout.rs` (new) | `WIDE_MIN_WIDTH`, `LayoutMode`, `mode`, `split_frame`, `split_body` | Pure total transformation |
| `src/ui/view.rs` (new) | `render(&mut Frame, &Dashboard)` | The render seam's pure side |
| `src/ui/driver.rs` (new) | `run_loop`, `LoopSummary`, `LoopError`, `TICK` | Generic over `Backend` and `EventSource`, like `changes::from_cli` being generic over `&dyn OpenspecCli` |
| `src/ui/terminal.rs` (new) | `TerminalOps`, `CrosstermOps`, `TerminalGuard`, `TerminalError`, `restore_then`, `install_panic_hook` | The `cli` seam's shape: a trait, one real spawn-nothing-else implementation, fakes in tests |
| `src/ui/event.rs` (new) | `EventSource`, `CrosstermEvents`, `EventError` | Same |
| `src/lib.rs` | `pub mod ui;`, and `testutil` gains `render_at`, `row_text`, `cell` | Existing `#[cfg(test)] pub(crate) mod testutil` |
| `src/main.rs` | `Invocation::Ui` dispatches to `ui::run`; exit 3 / 1 mapping | Unchanged shape: read args, write streams, set status |
| `src/changes.rs` | `empty_set()` — a total `ChangeSet` constructor naming all three fields | Third construction site under `change-model`'s existing gate |
| `tests/cli.rs` | Banner tests replaced by non-terminal tests | Binary-integration tier, unchanged mechanism |
| `Cargo.toml` / `Cargo.lock` | `ratatui`; `rust-version` `1.85` → `1.88` | `plugin-build`'s argued-dependency rule |
| `tests/fixtures/build-graph.txt` (new) | Per-triple build-graph snapshot | New; replaces a prose enumeration that no longer scales |

**No process spawn is added.** `src/cli.rs` remains the only module naming
`process::Command`, `Command::new`, or `Stdio`, and `src/ui/` names none of them.
`NOSPAWN-GREP` is carried forward from `subprocess-seam` with one deliberate edit — Guard
C's file-count minimum becomes `MIN="${MIN:-8}"` so the same block can assert 8 before this
change and 14 after, instead of a count that stops discriminating as the crate grows. That
edit is recorded here because "carried forward byte-identically" has been this
repository's convention for three changes, and this is the first departure from it.

**Every new view is in `ui` and is a pure function of state.** No view takes a path, a
`Config`, a clock, or a CLI. `ui::load` is the one function in the module that touches the
filesystem, and it is not a view: it produces the `Dashboard` a view is then given. This is
enforced by `NOIO-VIEW`, which searches `app.rs`, `layout.rs`, `view.rs`, and `driver.rs`
for `std::fs`, `std::io`, `std::env`, `std::process`, `std::net`, `File::`,
`read_to_string`, and `Command`, with a positive control requiring `src/ui/terminal.rs` to
name `std::io`. Three files under `src/ui/` are deliberately outside that set and each for
a stated reason: `mod.rs` holds `load` and `run`, `terminal.rs` holds the terminal seam, and
`event.rs` holds `CrosstermEvents`, which reads the real event stream.

**`Dashboard` gets its own gate.** `change-model`'s no-`Default`/no-`..` mechanism is
stated over four types in `src/changes.rs` and does not reach a new type in
`src/ui/app.rs`. `NODEFAULT-UI` covers `Dashboard` with the same shape — a guarded grep for
`impl Default for Dashboard`, for `Default` in the derive above `struct Dashboard`, and for
a `..` inside a `Dashboard { … }` literal or pattern — paired with a compile-time
exhaustive destructuring in `app.rs`'s tests, which is the half a grep cannot do.

**`Change` is unaltered.** No field is added, removed, or retyped, so the `from_files` /
`from_cli` agreement is untouched. `changes::empty_set()` is a new construction site, not a
new producer: it names all three `ChangeSet` fields explicitly and is therefore already
covered by `change-model`'s mechanism-1 gate (`GATE-MECH1`, no `Default` and no `..` in
`src/changes.rs`), which this change re-runs rather than modifies.

## Contracts

New public API on the library crate. All of it is additive except the deletion of
`lib::banner`, which nothing outside `main.rs` and its own test calls.

```rust
// src/ui/mod.rs
pub enum StartError { NotATerminal, Terminal(TerminalError), Io(String) }
pub fn load(start: &Path, config: &Config) -> Dashboard;
pub fn enter_if_terminal(is_terminal: bool, ops: &dyn TerminalOps)
    -> Result<TerminalGuard<'_>, StartError>;
pub fn run() -> Result<(), StartError>;

// src/ui/app.rs
pub enum Route { List, Detail }
pub enum Action { Quit, OpenDetail, BackToList, Ignore }
pub struct Dashboard { pub repo: Option<PathBuf>, pub searched_from: PathBuf,
                       pub changes: ChangeSet, pub route: Route, pub quit: bool }
pub fn action_for(event: &Event) -> Action;
impl Dashboard { pub fn apply(&mut self, action: Action); }

// src/ui/layout.rs
pub const WIDE_MIN_WIDTH: u16 = 100;
pub enum LayoutMode { Narrow, Wide }
pub fn mode(width: u16) -> LayoutMode;
pub fn split_frame(area: Rect) -> (Rect, Rect, Rect); // header, body, footer
pub fn split_body(area: Rect, route: Route) -> (Option<Rect>, Option<Rect>); // list, detail

// src/ui/view.rs
pub fn render(frame: &mut Frame, dashboard: &Dashboard);

// src/ui/terminal.rs
pub struct TerminalError { pub op: &'static str, pub detail: String }
pub trait TerminalOps {
    fn enable_raw(&self)     -> Result<(), TerminalError>;
    fn enter_alternate(&self)-> Result<(), TerminalError>;
    fn leave_alternate(&self)-> Result<(), TerminalError>;
    fn disable_raw(&self)    -> Result<(), TerminalError>;
}
pub struct CrosstermOps;
pub struct TerminalGuard<'a> { /* &'a dyn TerminalOps */ }
impl<'a> TerminalGuard<'a> {
    pub fn enter(ops: &'a dyn TerminalOps) -> Result<Self, TerminalError>;
}
pub fn restore_then(ops: &dyn TerminalOps, next: &mut dyn FnMut());
pub fn install_panic_hook();

// src/ui/event.rs
pub struct EventError(pub String);
pub trait EventSource {
    fn next_event(&mut self, timeout: Duration) -> Result<Option<Event>, EventError>;
}
pub struct CrosstermEvents;

// src/ui/driver.rs
pub const TICK: Duration = Duration::from_millis(250);
pub struct LoopSummary { pub frames: usize, pub polls: usize }
pub enum LoopError { Draw(String), Events(EventError) }
pub fn run_loop<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>, dashboard: &mut Dashboard, events: &mut E, tick: Duration
) -> Result<LoopSummary, LoopError>;

// src/changes.rs
pub fn empty_set() -> ChangeSet;
```

`Event`, `KeyEvent`, `KeyCode`, `KeyEventKind`, and `KeyModifiers` are
`ratatui::crossterm::event` types, used directly rather than translated into a crate-local
input enum. A translation layer would be a second total mapping to keep in step with
crossterm's, for no gain: the pure files already name no I/O, and crossterm's event types
are plain data.

**Consumers.** Only this repository. `list-view`, `markdown-viewer`, `detail-view`,
`tasks-tab`, `agent-polling`, and `degraded-states` are the downstream changes; each
extends `Dashboard` and `render` rather than replacing them. Not breaking: the plugin
manifest, the `config.toml` format, and every key already documented in `SPEC.md` → Keys
are unchanged, and `Ctrl-C` is added rather than rebinding anything.

## Persistence and Rollout

- **Migration:** none. No stored data, no schema, no on-disk format.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. `resolve::BinCache` is not consulted by this change.
- **Index rebuild:** none.
- **Authorization:** none. The plugin is a local read-only process.
- **Observability:** none beyond the exit statuses in `plugin-build`. No logging is added;
  a TUI has no stream to log to while it owns the screen.
- **Deployment impact:** `herdr plugin link .` users must rebuild (`make build`) to get the
  dashboard instead of the banner; the manifest is unchanged, so no re-link is needed. The
  MSRV moves to 1.88, which CI already exceeds (`dtolnay/rust-toolchain@stable`, 1.91.1 on
  the reference machine).

## Test Boundaries

Every collaborator this change touches, in every tier. "Replaced" always names what
replaces it.

| Dependency | In binary-integration tests (`tests/cli.rs`) | In unit and view tests (`--lib`) | In command-level checks |
|---|---|---|---|
| The **terminal** (raw mode, alternate screen) | **never reached** — the binary is spawned with stdout piped, so `ui::run` returns `NotATerminal` before any `TerminalOps` call | **replaced** by a recording `TerminalOps` double; `CrosstermOps` is never constructed | **replaced** by `NORAW-GREP`, which proves the crossterm mode functions appear only in `src/ui/terminal.rs` |
| The **rendering surface** | not reached | **replaced** by `ratatui::backend::TestBackend`, at 60 and 120 columns, plus 1, 2, 16, 18, 20, 99, 100, 101 for boundary scenarios | — |
| The **event stream** | not reached | **replaced** by a scripted `EventSource` double that records each call's timeout and errors when its script runs out, so a loop that never quits fails loudly instead of hanging | — |
| The **filesystem** | real: the spawned binary reads nothing before exiting 3 | **real**, through `crate::testutil::ScratchDir` under `std::env::temp_dir()` for `ui::load` only; every view test builds its `Dashboard` in memory and touches no directory | real, in `OPENSPEC-UNTOUCHED` and the graph checks |
| The **`openspec` binary** | not reached | **absent by construction**: `ui` takes no `OpenspecCli`, and `NOCLI-SHELL` proves `src/ui/` names none | **replaced** by `NOCLI-SHELL` |
| The **Herdr socket / `herdr` binary** | not reached | not reached — Phase 5 | — |
| The **process environment** | real, inherited by the spawned child; no variable is set for these tests | **replaced** by the injected `&dyn Fn(&str) -> Option<String>` lookup `config` already takes; `ui::load` takes a `&Config` and never reads the environment itself | — |
| The **process's own binary** | **real**: `env!("CARGO_BIN_EXE_herdr-openspec")`, the existing tier | not used | — |
| The **wall clock** | not read | not read; the loop's `tick` is a parameter, and the double records the value it was passed rather than measuring elapsed time | — |
| **`cargo`, `git`, `python3`, and the POSIX shell utilities `find`, `grep`, `xargs`, `sort`, `sed`, `awk`, `cut`, `tr`, `wc`, `diff`, `printf`, `mktemp`, `cp`, `rm`, `mkdir`** | — | — | **real**, in the command-level checks only (`NOSPAWN-GREP`, `NOIO-VIEW`, `NOCLI-SHELL`, `NORAW-GREP`, `NODEFAULT-UI`, `GATE-MECH1`, `DEPS`, `GRAPH-SNAP`, `WIDTHS`, `NOWAIVER`, `OPENSPEC-UNTOUCHED`, `TESTCOUNT`). The enumeration is exhaustive on purpose: a new tool in a check block is then a reviewable addition here rather than a silent one |
| **crates.io** | — | — | **real**, once, when `Cargo.lock` is resolved and the graph snapshot is generated |

No task may introduce a collaborator absent from this table.

## Test Strategy

Four tiers, matching the repository's existing ones.

- **Unit / view (`cargo test --all-features --lib`)** — everything pure: `layout::mode`,
  `action_for`, `Dashboard::apply`, `view::render` through `TestBackend`, `TerminalGuard`
  through the recording double, `run_loop` through `TestBackend` plus the scripted event
  source, and `ui::load` against a `ScratchDir`. This is where nearly all of the change is
  verified.
- **Binary integration (`cargo test --all-features --test cli`)** — the real binary, spawned
  with stdout piped: the exit statuses and the non-blocking guarantee.
- **Command-level checks** — the architectural invariants a Rust test cannot express:
  spawn containment (`NOSPAWN-GREP`), view purity (`NOIO-VIEW`), CLI absence from the shell
  (`NOCLI-SHELL`), raw-mode containment (`NORAW-GREP`), `Dashboard`'s no-`Default` /
  name-every-field gate (`NODEFAULT-UI`), the both-widths floor (`WIDTHS`), the coverage
  floor's integrity (`NOWAIVER`), the `openspec/` write invariant (`OPENSPEC-UNTOUCHED`),
  and the dependency graph (`DEPS`, `GRAPH-SNAP`). Every one carries a positive control and
  an existence guard, and every one is proven able to fail against a planted violation in
  task 9.5.
- **Gates** — `make check`, unchanged.

**The outer loop is taken.** End-to-end wiring is exactly the risk here: `main` → `ui::run`
→ terminal check → guard → loop is a path no unit test crosses, and getting it wrong in the
direction that matters — entering raw mode from a test process — damages the developer's
session rather than failing a test. Group 0 writes the failing `tests/cli.rs` test for the
headline scenario (`ui` with stdout piped exits 3 without blocking) before any
implementation, and group 8 closes it. The outer test is cheap: one process spawn, the
tier this repository already runs.

### Why each behaviour sits where it does

- **`view::render` is tested at the view tier, not through `run_loop`.** Driving a render
  through the loop would make every layout assertion depend on the event script, and a
  layout regression would surface as a loop-test failure. `run_loop`'s own tests assert
  only what the loop is responsible for: draw ordering, counts, and that the frame
  reflected the last applied action.
- **The resize behaviour is split.** `run_loop` cannot resize its own backend — it owns the
  `Terminal` — so "a resize event does not quit and does not error" is a loop test, and
  "the layout follows the current frame area" is a view test that resizes a
  `Terminal<TestBackend>` between two draws. Asserting the second through the loop would
  need shared mutable access to the backend, which buys nothing the two tests do not
  already prove.
- **`CrosstermOps` and `CrosstermEvents` are not tested.** Calling the real
  `enable_raw_mode` under `cargo test` would succeed on a developer's machine — crossterm
  opens `/dev/tty`, which exists in an interactive shell — and leave the terminal in raw
  mode when a test then fails. That is a worse outcome than uncovered lines. They are
  four-plus-two one-line bindings with no branch; `NORAW-GREP` proves nothing else in the
  crate can reach the underlying functions, which is the property that actually matters.
- **`install_panic_hook` is not tested; `restore_then` is.** Installing a panic hook is
  process-global and `cargo test` runs tests in parallel threads of one process, so a test
  that installed one would corrupt its neighbours — the same reasoning `AGENTS.md` already
  records for `std::env::set_var`. The hook's *body* is extracted into `restore_then`, a
  pure function taking the ops and a `&mut dyn FnMut()` stand-in for the previous hook, and
  that is asserted directly.

### Verification matrix

One row per spec scenario. `--lib` filters are counted, never trusted to a bare exit
status.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| terminal-lifecycle: The real implementation is the only place naming a terminal-mode function | `NORAW-GREP` with its vacuity guard and its `src/ui/terminal.rs` positive control | command | `find`, `grep`, `xargs` real | `sh $CHECKS/NORAW-GREP.sh` |
| terminal-lifecycle: No test constructs the real terminal implementation | `NORAW-GREP`'s second leg: `CrosstermOps` is named only in `src/ui/terminal.rs` (its definition) and `src/ui/mod.rs` (`run`'s wiring), so every test that takes `&dyn TerminalOps` uses the double | command | `find`, `grep` real | `sh $CHECKS/NORAW-GREP.sh` |
| terminal-lifecycle: Normal lifetime records the four operations mirrored | `guard::normal_lifetime_is_enter_enter_leave_disable` | unit | double | `testcount --lib 'ui::terminal::tests::' 8` |
| terminal-lifecycle: Raw mode fails and nothing else is attempted | `guard::enable_raw_failure_attempts_nothing_further` | unit | double | same filter |
| terminal-lifecycle: The alternate screen fails and raw mode is unwound | `guard::alternate_screen_failure_unwinds_raw_mode` | unit | double | same filter |
| terminal-lifecycle: Teardown errors are swallowed rather than panicking in Drop | `guard::teardown_errors_do_not_panic_and_both_are_attempted` | unit | double | same filter |
| terminal-lifecycle: Unwinding past the guard still restores | `guard::a_panic_still_restores` via `std::panic::catch_unwind` | unit | double | same filter |
| terminal-lifecycle: The hook restores before the previous hook runs | `guard::restore_then_restores_before_delegating` | unit | double + a recording closure | same filter |
| terminal-lifecycle: The refusal touches no terminal operation | `start::not_a_terminal_records_no_call` and its `true` control | unit | double | `testcount --lib 'ui::tests::start::' 3` |
| responsive-layout: Header, body, and footer occupy their rows at both widths | `view::frame_rows_at_60_and_120` | view | `TestBackend` | `testcount --lib 'ui::view::tests::' 16` |
| responsive-layout: A one-row frame renders the header and nothing else | `view::one_row_frame_draws_header_only` | view | `TestBackend` | same filter |
| responsive-layout: A two-row frame renders the header and the footer with no body | `view::two_row_frame_draws_no_body` | view | `TestBackend` | same filter |
| responsive-layout: A one-column frame renders without panicking | `view::one_column_frame_does_not_panic` | view | `TestBackend` | same filter |
| responsive-layout: The footer drops whole hints rather than truncating one | `view::footer_drops_whole_hints` | view | `TestBackend` | same filter |
| responsive-layout: At 120 columns both regions are drawn with the divider at column 40 | `view::wide_draws_two_regions_divided_at_40` | view | `TestBackend` | same filter |
| responsive-layout: At 60 columns only the routed region is drawn | `view::narrow_draws_only_the_list_region` | view | `TestBackend` | same filter |
| responsive-layout: At 60 columns the detail route replaces the list region | `view::narrow_detail_route_replaces_the_list_region` | view | `TestBackend` | same filter |
| responsive-layout: The breakpoint is exact at 99, 100, and 101 columns | `layout::mode_is_narrow_below_100` and `layout::mode_is_wide_at_100_and_above`, plus `view::breakpoint_is_exact_at_the_boundary` rendering 60, 99, 100, 101, and 120 | unit + view | `TestBackend` for the second | `testcount --lib 'ui::layout::tests::' 6` and the view filter |
| responsive-layout: The mode follows the current frame, not the startup size | `view::resizing_the_backend_changes_the_next_frame` | view | `Terminal<TestBackend>` resized between draws | view filter |
| responsive-layout: The routed region's border is bold and the other's is not | `view::routed_region_border_is_bold` | view | `TestBackend` | view filter |
| responsive-layout: Interiors are blank at both widths | `view::region_interiors_are_blank` | view | `TestBackend` | view filter |
| responsive-layout: A path that fits is right-aligned whole at both widths | `view::header_path_right_aligned_whole` | view | `TestBackend` | view filter |
| responsive-layout: A path too long for the narrow header is shortened from the left | `view::header_path_shortened_from_the_left` | view | `TestBackend` | view filter |
| responsive-layout: A header too narrow for any path shows only the label | `view::header_omits_the_path_when_too_narrow` | view | `TestBackend` | view filter |
| responsive-layout: No repository found is named in the header at both widths | `view::header_says_no_repository` | view | `TestBackend` | view filter |
| dashboard-loop: `Dashboard` has no `Default` and no site elides a field | `NODEFAULT-UI` with its `struct Dashboard` positive control and its two planted-violation controls, plus `app::tests::keys::dashboard_destructures_into_exactly_five_fields` as the compile-time companion | command + unit | `grep` real | `sh $CHECKS/NODEFAULT-UI.sh`; `testcount --lib 'ui::app::tests::' 9` |
| dashboard-loop: The pure view files name no I/O API | `NOIO-VIEW` with its four-file existence guard and `terminal.rs` positive control | command | `grep` real | `sh $CHECKS/NOIO-VIEW.sh` |
| dashboard-loop: The shell never names the CLI seam | `NOCLI-SHELL` with its `src/changes.rs` positive control | command | `find`, `grep` real | `sh $CHECKS/NOCLI-SHELL.sh` |
| dashboard-loop: Both quit keys quit and neither near-miss does | `keys::quit_keys_and_their_near_misses` | unit | none | `testcount --lib 'ui::app::tests::' 9` |
| dashboard-loop: A released quit key does not quit | `keys::only_press_kind_acts` | unit | none | same filter |
| dashboard-loop: Enter and Esc move between the two routes | `keys::enter_and_esc_move_between_routes` | unit | none | same filter |
| dashboard-loop: Non-key events are ignored without panicking | `keys::non_key_events_are_ignored` | unit | none | same filter |
| dashboard-loop: The first frame is on screen before the first event is read | `driver::first_frame_precedes_the_first_poll` | view + unit | `TestBackend`, scripted source | `testcount --lib 'ui::driver::tests::' 6` |
| dashboard-loop: Timeouts are not events and do not end the loop | `driver::timeouts_are_not_events` | view + unit | `TestBackend`, scripted source | same filter |
| dashboard-loop: Ctrl-C ends the loop | `driver::ctrl_c_ends_the_loop` | view + unit | `TestBackend`, scripted source | same filter |
| dashboard-loop: An ignored key redraws and keeps waiting | `driver::ignored_input_redraws_and_continues` | view + unit | `TestBackend`, scripted source | same filter |
| dashboard-loop: A route change is visible in the next frame | `driver::route_change_shows_in_the_next_frame` | view + unit | `TestBackend`, scripted source | same filter |
| dashboard-loop: A backend draw failure ends the loop rather than spinning | `driver::a_draw_failure_stops_before_polling` | unit | failing backend double, scripted source | same filter |
| dashboard-loop: A scratch repository is loaded from disk with no binary present | `load::a_scratch_repository_is_loaded_from_files` | unit | real filesystem via `ScratchDir` | `testcount --lib 'ui::tests::load::' 4` |
| dashboard-loop: No repository above the starting directory | `load::no_repository_above_the_start` with its measured ancestor precondition | unit | real filesystem via `ScratchDir` | same filter |
| dashboard-loop: The configured archived count is passed through | `load::archived_count_from_config_is_honoured` — seven dated archive directories read with `archived_count` 3 and again with 7 | unit | real filesystem via `ScratchDir` | `testcount --lib 'ui::tests::load::' 4` |
| dashboard-loop: Loading writes nothing | `load::loading_writes_nothing` comparing two `testutil::snapshot` values | unit | real filesystem via `ScratchDir` | same filter |
| plugin-build: `ui` with stdout piped exits 3 without blocking | `tests/cli.rs::ui_without_a_terminal_exits_three` — the outer-loop test | binary integration | the crate's own binary, real | `cargo test --all-features --test cli ui_without_a_terminal_exits_three` |
| plugin-build: The three failing statuses are distinct | `tests/cli.rs::failing_statuses_are_distinct` | binary integration | the crate's own binary, real | `testcount --test-cli '' 5` — an integration test's path is its bare function name, so a `cli::` filter would match nothing and exit 0 |
| plugin-build: Every binary-integration run pipes stdout | Inspection of `tests/cli.rs`'s five spawn sites, each setting `Stdio::piped()` or `Stdio::null()` on stdout, recorded in task 9.1; the tree-wide raw-mode containment is `terminal-lifecycle`'s row above, not restated | binary integration | the crate's own binary, real | `cargo test --all-features --test cli` plus the source inspection |
| plugin-build: Exactly one binary target is produced at the release path | `DEPS` leg 1 | command | `cargo`, `sh` real | `sh $CHECKS/DEPS.sh` |
| plugin-build: The declared dependency set is exactly the argued crates | `DEPS` leg 2, extended with the `ratatui` row and the "crossterm not declared, crossterm resolved" pair | command | `cargo`, `python3` real | same |
| plugin-build: Every package in the normal build graph declares an MSRV no higher than the crate's | `DEPS` leg 4, floor read from `Cargo.toml` | command | `cargo`, `python3` real | same |
| plugin-build: The resolved build graph is small and proc-macro-free (title kept verbatim; content is now the committed-snapshot comparison) | `GRAPH-SNAP` — four `cargo tree` runs compared to `tests/fixtures/build-graph.txt`, plus the proc-macro allowlist equality and the `linux-raw-sys`-only difference | command | `cargo`, `python3` real | `sh $CHECKS/GRAPH-SNAP.sh` |
| plugin-build: Each dependency is genuinely needed rather than incidental | `DEPS` leg 5, four legs, run in a throwaway copy | command | `cargo` real | `sh $CHECKS/DEPS.sh` |
| plugin-build: No JSON parsing reaches the subprocess seam | `NOJSON-SEAM`, carried forward from `changes-from-cli` | command | `grep` real | `sh $CHECKS/NOJSON-SEAM.sh` |
| quality-gates: The harness renders a state value with no repository on disk | `testutil::tests::render_at_touches_no_directory` — a `testutil::snapshot` of a `ScratchDir` the test owns, **not** `std::env::temp_dir()` (this crate creates and destroys hundreds of `ScratchDir`s directly under it across parallel `cargo test` threads, which made an early draft of this test red on roughly six runs in ten for reasons unrelated to rendering — finding 2 of the planning review) | unit | real filesystem, one owned directory | `testcount --lib 'testutil::tests::' 3` |
| quality-gates: Both widths are exercised for every view scenario | `WIDTHS` — every `#[test]` in `src/ui/view.rs` names both `60` and `120`, with a floor of 16 tests found. No exemption list: every boundary-width scenario also renders at 60 and 120 as its contrasting control | command | `python3` real | `sh $CHECKS/WIDTHS.sh`, paired with `testcount --lib 'ui::view::tests::' 16` |
| quality-gates: The coverage floor is unchanged by the new module | `make coverage` at the unchanged 80% floor, plus a grep proving no `#[coverage` attribute and no `llvm-cov` exclusion flag entered the tree | gate + command | `cargo-llvm-cov` real | `make coverage`; `sh $CHECKS/NOWAIVER.sh` |

## Decisions

**`ratatui` 0.30.2, and no direct `crossterm` declaration.** `SPEC.md` names the stack as
"ratatui + crossterm". Declaring both would put two `crossterm` version requirements in the
manifest, and cargo will happily resolve a second `crossterm` if they ever disagree — at
which point `CrosstermBackend<Stdout>` and a `KeyEvent` come from different crates and the
error is a type mismatch nobody expects. `ratatui` re-exports the exact `crossterm` its
`ratatui-crossterm` backend resolved, so using `ratatui::crossterm` makes disagreement
impossible. One consequence worth knowing before writing a style assertion: `ratatui` declares
`ratatui-crossterm` with **its** defaults on, and those include `underline-color`, so
`default-features = false` is narrower on its face than in effect and an untouched cell's
`Style` is `fg(Reset).bg(Reset).underline_color(Reset)` — equal to neither
`Style::default()` nor `Style::reset()`. The `crossterm` feature likewise re-enables `std`.
*Alternative considered:* declare `crossterm = "0.29"` directly and keep
`ratatui`'s features minimal — rejected, because the version would have to be kept in lock
step with `ratatui-crossterm`'s by hand, forever, with no check that would notice.

**0.30.2 rather than 0.29.x, accepting `rust-version` 1.88.** `AGENTS.md` requires checking
the current stable version rather than remembering one; `cargo info ratatui` reports 0.30.2
as latest. It requires Rust 1.88, which raises this crate's declared MSRV from 1.85. CI runs
`dtolnay/rust-toolchain@stable` and the reference machine is on 1.91.1, so nothing breaks
today. *Alternative considered:* pin 0.29.x to hold 1.85 — rejected: the MSRV is a claim
about what this crate supports, not a goal in itself; no consumer depends on it; and 0.29's
`Backend` has no associated `Error` type, so `run_loop`'s generic signature would have to be
rewritten when the upgrade eventually happens.

**The proc-macro prohibition becomes an allowlist.** `plugin-build` currently forbids any
proc-macro crate in the normal build graph. That is unsatisfiable with `ratatui`:
Eight proc-macro crates arrive with the dependency — `darling_macro`, `derive_more-impl`,
`document-features`, `indoc`, `instability`, `rustversion`, `strum_macros`, and
`thiserror-impl` — reached through `ratatui-core` (`strum_macros`, `thiserror-impl`, and
`rustversion` beneath `compact_str`/`castaway`), `ratatui-widgets` (`indoc`, `instability`,
and `darling_macro` beneath it), and `crossterm` (`derive_more-impl`, `document-features`). Weakening the rule
to "some proc macros are fine" would delete the property; enumerating the eight keeps it
mechanical — a ninth still fails, and a future direct dependency still has to be argued.
*Alternative considered:* a hand-rolled terminal layer with no widget crate — rejected as a
rewrite of the ecosystem's solved problem, and `SPEC.md` names `ratatui` as the stack.

**The build graph moves from prose to a committed snapshot.** The existing scenario
enumerates sixteen packages by name in the spec text. The graph is now roughly eighty and
differs by one package between macOS and Linux. `tests/fixtures/build-graph.txt` holds the
per-triple sorted sets; the check compares against it and fails on any difference. It is
exactly as brittle as `Cargo.lock`, which is already committed, and every change is a
reviewable diff instead of an edit to a paragraph.

**Degenerate heights are branched on explicitly, not handed to the solver.** Measured
against ratatui 0.30.2: `Layout::vertical([Length(1), Min(0), Length(1)])` at height 1
gives the single row to the **footer**, not the header — so the naive split renders
`q quit` where `OpenSpec` belongs, and `responsive-layout`'s one-row scenario would fail
for a reason nobody would guess. At height 2 it gives one row each to header and footer,
which happens to be right; relying on that is relying on unspecified solver behaviour.
Branching on 0, 1, and 2 costs three lines and makes the RED test obvious to whoever
deletes them.

**Refuse to start when stdout is not a terminal.** Three reasons, in increasing order of
force. A TUI cannot render into a pipe. A clear exit status is more useful to a script than
a screenful of escape sequences. And decisively: `tests/cli.rs` spawns this binary, and
crossterm's `enable_raw_mode` opens `/dev/tty` rather than stdout — in an interactive
`cargo test` that call *succeeds*, and a subsequent test failure would leave the
developer's shell in raw mode. The `IsTerminal` check on stdout makes that branch
unreachable from the suite by construction rather than by care. *Alternative considered:*
render a plain-text summary when stdout is a pipe — rejected as scope this change has no
specification for; `degraded-states` may revisit it.

**Exit status 3 for "no terminal".** 0 is success, 2 is already the argument-usage
rejection `plugin-build` specifies, and 1 is the generic start failure. A distinct status
lets a script tell "you gave me bad arguments" from "I need a terminal" without parsing
stderr.

**An RAII guard *and* a panic hook.** The guard alone restores on unwind, since `Drop` runs
during unwinding — that is what the `catch_unwind` test proves. The hook exists for
ordering: without it, the default hook prints the panic message onto the raw alternate
screen, which is then torn down, and the user sees nothing. *Alternative considered:*
`std::panic::set_hook` only, with no guard — rejected, because it does nothing for the
error-return path and nothing under `panic = "abort"`.

**A `TerminalOps` trait rather than testing crossterm.** This mirrors the `cli` seam
exactly: a trait, one real implementation that does nothing but call the underlying
function and map its error, and a recording double in tests. It buys the four ordering
scenarios — enter order, mirrored leave order, and the two partial-failure unwinds — none
of which could be asserted against a real terminal without one being attached.

**`run_loop` returns `LoopSummary { frames, polls }`.** Draw count and poll count are
otherwise unobservable, and they are what distinguishes "drew before waiting" from "waited
first", and "a timeout is not an event" from "a timeout is an event that redraws". A
`()` return would make those scenarios assert only the final buffer, which both
implementations produce identically.

**The scripted event source errors when exhausted.** A double that returned `Ok(None)`
forever would turn a loop bug into a hung test — the worst failure mode for a suite that
runs in CI with a 30-minute timeout. Erroring makes an unquitting loop fail in
milliseconds with a message naming the exhausted script.

**Region bodies are left empty, and asserted empty.** Filling them with placeholder text
would be content `list-view` immediately deletes, and an assertion on that text would go red
for the right reason at the wrong time. Asserting the interiors are blank is a real
constraint now — it catches chrome accidentally bleeding into the body — and turns into a
visible, intentional diff when the first view lands.

**The header shortens the path from the left.** A repository is identified by its own
directory name, which is at the end of the path; truncating from the right would leave every
repository under the same parent looking identical at 60 columns. The `…` prefix marks that
something was dropped.

**`Constraint::Length(40)` for the list column, `Min(0)` for detail.** At exactly 100
columns that is 40 and 60. Every column beyond 100 goes to detail, where markdown is read;
a percentage split would grow the list column, which holds fixed-width rows, for nothing.
`SPEC.md` gives no width for either column, so this is a decision made here and inherited by
`list-view`, `detail-view`, and `agent-attribution` — which is why group 10 writes it into
`SPEC.md` rather than leaving it implicit. It has one immediate consequence worth naming:
`Length(40)` leaves a 38-column interior, and `SPEC.md`'s own List view mock has rows up to
48 characters (`> add-token-refresh    [4/9]  > claude - working`), so that mock illustrates
content rather than width. Group 10 annotates it as such; how a row is shortened to 38
columns is `list-view`'s decision, taken with the constraint already known rather than
discovered against it. *Alternative considered:* widening the list column to 48 —
rejected, because at exactly 100 columns it would leave 52 for markdown, and the detail
side is the one that has to be readable at the breakpoint.

**`Route` on `Dashboard`, `LayoutMode` derived from the frame.** The route is state the user
changed and must persist across frames; the layout mode is a function of the terminal's
current size and must never be stale. Storing the mode would create exactly the resize bug
the breakpoint exists to avoid.

**`changes::empty_set()` lives in `src/changes.rs`.** `ui::load` needs a `ChangeSet` for the
no-repository case. Constructing the literal in `src/ui/mod.rs` would put a construction
site outside the file `change-model`'s no-`Default`/no-`..` gate searches, quietly widening
what the gate does not cover. One total constructor in the gated file costs three lines and
keeps the gate's scope true.

## Risks / Trade-offs

- **The MSRV moves to 1.88 without an MSRV CI job.** → `DEPS` leg 4 reads the floor from
  `Cargo.toml` and compares it against every normal-graph package's declared
  `rust-version`, so the claim is checked even though no runner pins 1.88. Adding an MSRV
  job is `ci-pipeline`'s business, not this change's.
- **Eight proc-macro crates enter the build, lengthening a cold compile.** → Accepted; the
  gates are not timed, and the allowlist makes any further growth a reviewable failure.
- **`tests/fixtures/build-graph.txt` goes stale on any dependency bump.** → That is the
  point: the check fails, and refreshing it is a deliberate one-line command recorded in
  the task that bumps the dependency.
- **The `WIDTHS` check is a heuristic — it reads test source for the literals `60` and
  `120`.** → It cannot prove an assertion is meaningful: a `60` in a comment satisfies it,
  and a `#[test]` inside a doc comment inflates its count. It has **no exemption list** —
  every one of `responsive-layout`'s sixteen scenarios renders at both widths, boundary
  scenarios adding 1, 16, 18, 20, 99, 100, or 101 on top — and it is paired with
  `testcount --lib 'ui::view::tests::' 16`, which is what proves the tests exist and run,
  and with the per-scenario tasks, which name the exact cells asserted at each width.
- **The `#[cfg(test)]` doubles are instrumented too.** `cargo llvm-cov` counts test code,
  so group 6's `FailingBackend` — ten `Backend` methods of which `clear`, `clear_region`,
  and `window_size` are never called by `Terminal` — contributes uncovered lines of its own,
  as does `main.rs`'s exit-1 arm, which no test can reach because `StartError::Terminal` and
  `StartError::Io` arise only from a real terminal. → Immaterial at a projected ~98%, but it
  is why the recorded total will move a little more than the residue list alone suggests.
- **`ui::run`'s body after the terminal check is uncovered.** → It is twelve lines of
  wiring with no branch. Every branch it composes — the terminal check, the guard, the
  load, the loop — is covered on its own. The alternative, covering it, requires a real
  terminal in the suite, which this design exists to prevent.
- **A very narrow pane (under 20 columns) shows almost nothing useful.** → Out of scope:
  `PRD.md` names 40 columns as the low end, and the degenerate-size scenarios prove the
  renderer does not panic below it.

## Migration Plan

None required. No data, no format, no deployment order. A developer with the plugin linked
runs `make build` to replace the banner with the dashboard; the manifest, the pane
definitions, and the configuration file are untouched, so no re-link and no re-install
happens. Rollback is `git revert` of the change's commits followed by `make build`.

## Open Questions

None blocking. Two decisions are deliberately deferred with their resolution point already
recorded:

- Whether `ui` should render a plain-text summary instead of exiting 3 when stdout is a
  pipe. Deferred to `degraded-states`, which owns the audit of every degraded row.
- Whether the header should carry the `file mode` badge from `SPEC.md`'s degraded-states
  table. Deferred to `degraded-states`, which the table's own row assigns it to; this change
  adds no badge.

## Visual Design

Not applicable. This change builds a terminal view, and no design source — no HTML mockup,
no design file — exists for it. The visual contract is `SPEC.md` → Responsive layout,
transcribed into `responsive-layout`'s scenarios as exact cell positions.
