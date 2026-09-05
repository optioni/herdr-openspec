## Context

Fourteen changes have landed. The dashboard reads `openspec/` once, at startup, from files
only, and then never again: `changes::from_cli` and `changes::merge` were built, fully
tested, and never called at runtime, and no code path has ever consulted the `openspec`
binary while the pane was open. `SPEC.md` → Dual-source model describes two sources of a
change, fast and authoritative; only the fast one has ever run.

This change closes both gaps at once, and they are the same gap: the reason a refresh is
hard is that the authoritative source costs 200–400ms per change and the render path must
not wait for it. Everything below follows from that one constraint.

Three properties of the landed tree make the shape possible rather than forced:

- `cli::OpenspecCli` was declared `Send + Sync` by `subprocess-seam` **for this change**
  ("`live-refresh` runs CLI calls on a worker thread"). A worker can hold an
  `Arc<dyn OpenspecCli>` with no redesign.
- `ui::driver::TICK` was documented by `tui-shell` as existing so "a later change adding
  periodic work has a wake-up already in place". The loop already wakes every 250ms.
- `NOCLI-SHELL` already forbids every file under `src/ui/` from naming `from_cli`,
  `OpenspecCli`, `HerdrCli`, `CliChanges`, or `npm_prefix`. Keeping the watcher and the
  worker **outside** `src/ui/` therefore makes "the CLI is off the render path" a
  structural fact a landed check already enforces, rather than a new claim needing a new
  argument.

Four hazards were named before planning began and every decision below is measured against
them: no timing-based test may assert something has already happened; the watcher must not
block the draw; views stay pure with no I/O; and invalidation is per change, not a full
reload.

## Goals / Non-Goals

**Goals:**

- One `notify` watcher on `<repo>/openspec/`, recursive, debounced at 150ms, confined to
  `src/watch.rs`.
- Per-change CLI invalidation: a touched path under `openspec/changes/alpha/` re-asks the
  CLI about `alpha` and about no other change, provably.
- Every CLI call on a worker thread. The render path calls no method that can block.
- `r` forces a full refresh; the same mechanism issues the startup request.
- Files paint, then the CLI corrects — visible in cell content at 60 and 120 columns.
- Every degraded path a test: no binary, a CLI that errors, a CLI that returns garbage, a
  watcher that will not start, `openspec/` removed while running.
- Zero new timing-based assertions, and a check that makes adding one fail.

**Non-Goals:**

- Any Phase 5 work — no `herdr agent list` poll, no attribution, no launch keys.
- A second refresh trigger: no polling fallback, no `pane.focused` hook, no timer.
- A visible refresh indicator, spinner, or `file mode` badge (`degraded-states`).
- Rewriting `changes::from_cli` or `changes::merge`; both are reused, one of them redefined.
- Lowering `TICK`, changing `Detail`'s five fields, or growing the pure-view set past eight.

## Boundaries

Two new modules, both **outside** `src/ui/`, each following an existing single-file
confinement pattern:

| New file | Follows | Confinement it accepts |
|---|---|---|
| `src/watch.rs` | `src/ui/markdown.rs` (`MDSEAM`) and `src/cli.rs` (`NOSPAWN-GREP`) | the only file naming `notify`; names no `ratatui` type; spawns no process; holds the crate's one `Instant::now()` binding |
| `src/refresh.rs` | `src/cli.rs` | the only file naming `thread::spawn`; reaches the `openspec` program only through `Arc<dyn OpenspecCli>`; spawns no process |

Existing modules touched:

| Module | What changes |
|---|---|
| `src/changes.rs` | `Selection`, `CliCache`, `from_cli_cached`; `from_cli` becomes a one-line call to it. `Change`, `ChangeSet`, `ArtifactRef`, and `Origin` are untouched, so `change-model`'s two-producer gate is unmoved |
| `src/ui/app.rs` | `Refresh` (three fields), `Dashboard::refresh` (the ninth field), `Action::Refresh` (the thirteenth variant), `Dashboard::adopt`, and `sync_detail`'s `force` |
| `src/ui/driver.rs` | `Live { fs, refresher }` and the three live-tier steps before the sync; the poll timeout |
| `src/ui/list.rs` | `rows` emits `refresh.problems` before `changes.problems` |
| `src/ui/mod.rs` | `ui::load` sets `refresh.requested`; `ui::run` wires `watch::start` and `refresh::start` |
| `src/lib.rs` | `pub mod watch; pub mod refresh;` and two `#[cfg(test)]` doubles in `testutil` |

The pure-view set stays **eight** files and `src/ui/` stays **eleven**: nothing is added
under `src/ui/`, which is what leaves `NOIO-VIEW`, `NOCLI-SHELL`, `READSEAM`, and
`READONLY-UI` running at their landed floors.

Two collaborators are injected as trait objects, on the shape `ui::event::EventSource`
established and `ArtifactReader` reinforced: **every method of both is non-blocking**, which
is what the render path's safety rests on.

```rust
pub trait FsEvents: Send {                                   // src/watch.rs
    fn drain(&mut self) -> Result<Option<Vec<PathBuf>>, WatchError>;
    fn pending_in(&self) -> Option<Duration>;
}

pub trait Refresher {                                        // src/refresh.rs
    fn request(&mut self, selection: Selection);
    fn take_result(&mut self) -> Option<RefreshResult>;
}
```

## Contracts

Additive within the crate; there is no external consumer. `herdr-plugin.toml`, the
`config.toml` format, the state-file format, the exit statuses, and the `ui` argument
surface are all untouched, so no re-link and no re-install is needed.

**`changes` (new, additive):**

```rust
pub enum Selection { All, Only(BTreeSet<String>) }
impl Selection { pub fn union(self, other: Selection) -> Selection }

#[derive(Default)]
pub struct CliCache { /* private: name -> (schema, artifacts, problems) */ }

pub fn from_cli_cached(cli: &dyn OpenspecCli, repo: &Path,
                       selection: &Selection, cache: &mut CliCache) -> CliChanges;
pub fn from_cli(cli: &dyn OpenspecCli, repo: &Path) -> CliChanges;  // unchanged signature
```

`from_cli`'s signature and behaviour are unchanged; its **body** becomes
`from_cli_cached(cli, repo, &Selection::All, &mut CliCache::default())`. Every landed
`cli-changes`, `schema-cli-fallback`, and `change-merge` test therefore stays green with no
edit, which is the compatibility evidence.

**`watch` (new):** `classify`, `invalidate`, `Touch`, `Debounce`, `DEBOUNCE`, `DEBOUNCE_MAX`,
`poll_timeout`, `FsEvents`, `WatchError`, `start`, `none`.

**`refresh` (new):** `Refresher`, `RefreshResult`, `start`, `none`.

**`ui::app` (additive but compile-breaking within the crate, by design):** `Dashboard` gains
a ninth field and `Action` a thirteenth variant. Neither type implements `Default` and
neither may be built with a `..` rest, so **every** construction site is an `E0063` until it
names the new field — the same compile-enforced mechanism `tasks-tab` used for
`ArtifactRef::tracks_tasks`, and the reason a nine-field `Dashboard` cannot be half-adopted.

**`ui::driver::run_loop` (breaking within the crate):** gains a `live: Live<'_>` parameter,
placed fourth, **ahead of** `read` and `tick`, which shift to fifth and sixth at all eighteen
call sites. Six parameters, below clippy's `too_many_arguments` threshold of seven.
`LoopSummary` gains **no** field, so every landed `LoopSummary { frames, polls }` literal in
the suite is unchanged.

**`refresh` (`#[cfg(test)]` seam):**
`refresh::worker_for_test(repo, cli, archived_count) -> (Box<dyn Refresher>,
Receiver<RefreshResult>, Receiver<()>)`. Its **second** element is the worker's result
`Receiver`, handed to the test instead of being stored on the `Refresher`, so a test can
`recv_timeout` on it; its **third** receives from a channel whose `Sender` the worker thread
owns and drops only when its body returns. It exists because the real `Refresher` **owns** the
result `Receiver` that `take_result` reads: dropping the `Refresher` destroys the only handle
through which either "a result arrived" or "the worker returned" could be observed, and
`Box<dyn Refresher>` deliberately exposes only the non-blocking `take_result`. Polling a
method that is non-blocking by contract would be a ten-second hot spin.

It SHALL be declared **after** `start`, below the file's single `thread::spawn` and above
`mod tests`: `READONLY-UI` and `NOBLOCK` build a production slice by discarding everything
from a file's **first** line-anchored `#[cfg(test)]` to EOF, so a `#[cfg(test)]` item above
`start` hides the worker's whole body from both. Measured: with it there, `READONLY-UI`
reports green on a `std::fs::write` in the worker's start path. Both checks also require each
of the two new modules to hold exactly **one** line-anchored `#[cfg(test)]`, because a count
is a check while a placement rule is a convention.

**`refresh` (private, but named because a test drives it directly):**
`drain_and_fold(first: Selection, rx: &Receiver<Selection>) -> Selection`. Extracted so the
folding rule is provable on the test's own thread; a rule proved only through a live worker is
a rule proved by whichever interleaving happened to occur.

**Not BREAKING for a user:** `r` was `Action::Ignore`; no landed binding moves, and while
filtering `r` still types itself.

## Persistence and Rollout

- **Migration:** none. No on-disk format changes.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** the one cache this change adds, `CliCache`, is in-memory,
  per-worker, and lives exactly as long as the pane. It is invalidated per change by
  `Selection` and evicted wholesale for any change `openspec list --json` stops reporting.
  It is never written to a file. `resolve::BinCache` and `sync_detail`'s `detail.loaded` are
  the crate's two other caches; `detail.loaded` gains the `refresh.reload` override.
- **Index rebuild:** none.
- **Authorization:** none. The plugin is read-only and single-user.
- **Observability:** a watcher that will not start and every CLI failure surface as
  `!`-marked rows in the list region — the crate's only reporting channel. No logging
  framework is added; the crate has none and this change does not introduce one.
- **Deployment:** `make build` produces the same single binary. `notify` is compiled in, so
  a fresh `herdr plugin install` resolves five (macOS) or six (Linux) further packages;
  `Cargo.lock` is committed, so the resolution is the one this change verified.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The filesystem (`openspec/` tree) | **real** — `crate::testutil::ScratchDir`, driven through `ui::load` and the real `ui::read_artifact` | **real** for `watch::start`, `changes::from_cli_cached`'s write-nothing scenario, and `refresh`'s worker; **absent** for every pure function (`classify`, `invalidate`, `Debounce`, `poll_timeout`, `adopt`, `rows`, `render`), which take values |
| `notify`'s watcher and its OS backend (FSEvents / inotify) | **real** in exactly one acceptance scenario, `a_live_watcher_over_the_tree_writes_nothing`, which asserts only byte-identity; **replaced** by a scripted `FsEvents` double in the other two, so their exact-equality assertions on the recorded request vector cannot be appended to by a live watcher | **real** in exactly three `watch::tests::` scenarios (a real `ScratchDir` watch, a path that does not exist, and the write-nothing snapshot pair); **replaced** everywhere else |
| The system clock | **not reached** — the acceptance tests use a scripted `FsEvents`, and the one that opens a real watcher asserts only byte-identity, so no assertion depends on elapsed time | **injected as a parameter** for `Debounce` and absent from `poll_timeout`; **real** only inside `watch::RealFsEvents::drain`, whose one clock call is the crate's fourth one-line binding to the real world. `RealFsEvents::pending_in` returns the `Option<Duration>` that `drain` cached, so there is exactly one clock call and not two |
| The `openspec` binary | **never spawned** — a `cli::FakeCli` answers by argument vector | **never spawned**; `cli::FakeCli` throughout. `Command::new` stays in `src/cli.rs` alone, checked by `NOSPAWN-GREP` |
| The worker thread | **replaced** by a synchronous recording `Refresher` double, so the acceptance tests are deterministic | **real** in `refresh::tests::` only, where every assertion is made after a `recv_timeout(10s)` on the worker's result channel or its exit channel — the second and third elements of the `#[cfg(test)]` `refresh::worker_for_test` seam — returned an item. Never after polling the non-blocking `take_result`, which has no legal pacing |
| The terminal | **replaced** — `ratatui::backend::TestBackend` | **replaced** — `TestBackend`, and a recording `TerminalOps` double for the lifecycle. No test constructs `CrosstermOps` |
| Terminal input events | **replaced** — `testutil::Script`, a scripted `EventSource` | **replaced** — `testutil::Script` |
| The artifact read | **real** — `ui::read_artifact` over the scratch tree | **replaced** — `testutil::RecordingReader`, which is the only way "was not re-read" is assertable |
| The Herdr socket | **not reached** — this change names no `HerdrCli` | **not reached** |
| The process environment | **not reached** — `Config` values are built directly | **not reached**; `config::env_lookup` is untouched |
| `cargo`, `cargo tree`, `cargo metadata` | **not reached** | **real**, in the `DEPS` and `GRAPH-SNAP` command-level checks only |
| `git` | **not reached** | **real**, in `OPENSPEC-UNTOUCHED` only |
| `python3` | **not reached** | **real**, in `NODEFAULT-UI`, `GATE-MECH1`, `NOSLEEP`, and the `*WIDTHS` checks |

No task may invent a collaborator this table does not name. In particular: **no test may
start a real `herdr`, a real `openspec`, or a real terminal**, and **no `ui::` test may
start a thread or read a clock.** Exactly one `ui::` test opens a watcher —
`ui::tests::live::a_live_watcher_over_the_tree_writes_nothing` — and it asserts byte-identity
and a re-read `Progress` only, so nothing it claims can be affected by a watcher's timing.
Every other `ui::` test drives the scripted `FsEvents` double.

## Test Strategy

Four tiers, as `openspec/config.yaml` states:

1. **Unit** — pure functions over values. `cargo test --all-features --lib '<filter>::'`.
2. **View** — `ui::view::render` into a `TestBackend` at **60 and 120** columns, asserting
   named cells. Same command, filtered.
3. **Filesystem-edge** — a real `ScratchDir` for `watch::start`, `refresh`'s worker, and
   `changes`' write-nothing claims.
4. **Command-level checks** — twenty-six files in `$CHECKS`: twenty-five gates run by label, plus `TESTCOUNT.sh`, which is sourced rather than run.

**This change takes the outer-loop acceptance test.** `ui::app::action_for` →
`Dashboard::apply` → `run_loop`'s live tier → `Refresher` → `Dashboard::adopt` →
`sync_detail` → `ui::view::render` is a path no unit test crosses, and the dual-source claim
("files paint, the CLI corrects") is a claim about what the **loop** shows in successive
frames rather than about what a function returns. It lives in a new module
`ui::tests::live::`, is written RED in group 2, and closes in group 11.

Two structural notes that the matrix below depends on:

- **The per-change invalidation rule is proven single-threaded**, on
  `changes::from_cli_cached` with a recording fake, by an **exact equality** on the recorded
  argument vectors. The worker is proven only to carry a request and return two results. A
  negative assertion ("beta was not re-asked about") made across a thread boundary is the
  defect class this split removes.
- **Every wait in this change waits for a condition.** The two places anything is waited for
  are `refresh::tests::`' `recv_timeout(10s)` — on the worker's result channel, and on the
  exit channel `worker_for_test` hands back — and `watch::tests::`' deadline-bounded poll of
  `drain`, which sleeps 10ms between iterations. Neither sleeps and then asserts. `NOSLEEP`
  makes reintroducing that shape a check failure, and its leg 1 accepts both shapes because
  both re-test the condition after every wait.
- **Every threaded assertion is positive, and made after a `recv_timeout` returned an item.**
  The one place a negative assertion was reachable — "no cycle ran `instructions apply` for
  a change outside the folded selection" — is deleted and replaced by a direct, single-threaded
  test of `drain_and_fold` (Decision 22), so no claim in this change depends on an
  interleaving. The one place channel *disconnection* is asserted matches
  `RecvTimeoutError::Disconnected` explicitly rather than `is_err()`, because `Err(Timeout)`
  is exactly the failure that scenario exists to catch.

### Verification matrix

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| watch-invalidation: `notify` is named in exactly one file | `WATCHSEAM` legs 1 + guards, plus a planted `use notify::Event;` in `src/ui/driver.rs` | check | real grep | `MIN=23 sh $CHECKS/WATCHSEAM.sh` |
| watch-invalidation: The confined module carries no view type and no spawn | `WATCHSEAM` legs 2 and 3, plus a planted `// a ratatui Style` | check | real grep | `MIN=23 sh $CHECKS/WATCHSEAM.sh` |
| watch-invalidation: A file inside one change invalidates that change and no other | `watch::tests::a_change_file_invalidates_only_that_change` | unit | none (values) | `cargo test --lib 'watch::tests::'` |
| watch-invalidation: A change directory appearing or vanishing is a repository touch | `watch::tests::the_change_directory_itself_is_a_repository_touch` | unit | none | same |
| watch-invalidation: The archive tier and the schema are repository touches | `watch::tests::archive_and_schema_are_repository_touches` | unit | none | same |
| watch-invalidation: A path outside the repository is ignored | `watch::tests::a_path_outside_the_repository_is_outside` | unit | none | same |
| watch-invalidation: A mixed batch unions its classifications | `watch::tests::a_mixed_batch_unions_its_classifications` | unit | none | same |
| watch-invalidation: Nothing is due before the window and everything is due at it | `watch::tests::nothing_is_due_before_the_window` | unit | injected `now` | same |
| watch-invalidation: A later push extends the window | `watch::tests::a_later_push_extends_the_window` | unit | injected `now` | same |
| watch-invalidation: A continuous writer cannot defer a batch forever | `watch::tests::a_continuous_writer_cannot_defer_forever` + `debounce_max_is_one_second` (folded into it) | unit | injected `now` | same |
| watch-invalidation: Repeated paths are coalesced | `watch::tests::repeated_paths_are_coalesced` | unit | injected `now` | same |
| watch-invalidation: The window's length is pinned | `watch::tests::debounce_window_is_150_milliseconds` | unit | none | same |
| watch-invalidation: The timeout is the tick when nothing is pending | `watch::tests::poll_timeout_is_the_tick_when_nothing_is_pending` | unit | none | same |
| watch-invalidation: The timeout is the remaining window when that is shorter | `watch::tests::poll_timeout_is_the_remaining_window_and_never_zero` | unit | none | same |
| watch-invalidation: A watcher that will not start degrades and names the reason | `watch::tests::start_on_a_missing_path_degrades` | fs-edge | real `notify` (deterministic `PathNotFound`) | same |
| watch-invalidation: A real watch reports a written file, polled to a deadline | `watch::tests::start_on_a_real_directory_yields_the_touched_path` | fs-edge | real `notify`, real `ScratchDir` | same |
| watch-invalidation: The inert watcher never yields | `watch::tests::no_fs_events_never_yields` | unit | none | same |
| watch-invalidation: `openspec/` is removed while the watcher runs | `ui::driver::tests::a_watch_error_is_recorded_once` (a `ScriptedFs` returning `Err`, deterministic — never a real watcher drained for some elapsed period) + `ui::view::tests::a_removed_repo_shows_the_problem_row` at 60 and 120 | unit + view | scripted `FsEvents`, `TestBackend` | `cargo test --lib 'ui::driver::tests::'`; `cargo test --lib 'ui::view::tests::'` |
| refresh-worker: `All` absorbs and two `Only` sets union | `changes::tests::selection_union_*` (two tests) | unit | none | `cargo test --lib 'changes::tests::'` |
| refresh-worker: An unrelated change is not re-asked about | `changes::tests::only_reruns_the_named_change` — exact equality on `FakeCli::calls()` | unit | `cli::FakeCli` | same |
| refresh-worker: A selected change that is not cached is still re-asked about | `changes::tests::only_still_runs_an_uncached_change` | unit | `cli::FakeCli` | same |
| refresh-worker: A cached change takes its progress from the fresh list | `changes::tests::cached_progress_comes_from_the_fresh_list` | unit | `cli::FakeCli` | same |
| refresh-worker: A change that vanished from the list is evicted | `changes::tests::an_unlisted_cached_change_is_evicted` | unit | `cli::FakeCli` | same |
| refresh-worker: A `list --json` failure leaves the cache untouched | `changes::tests::a_list_failure_leaves_the_cache_untouched` | unit | `cli::FakeCli` | same |
| refresh-worker: Garbage for one change leaves the others intact | `changes::tests::garbage_apply_payload_leaves_the_others_intact` | unit | `cli::FakeCli` | same |
| refresh-worker: A change whose schema the CLI rejects is cached like any other | `changes::tests::a_rejected_schema_is_cached_like_any_other` — exact equality on `FakeCli::calls()` for the second call | unit | `cli::FakeCli` | same |
| refresh-worker: Producing changes from a cache writes nothing | `changes::tests::from_cli_cached_writes_nothing` (snapshot pair) | fs-edge | real `ScratchDir`, `cli::FakeCli` | same |
| refresh-worker: The inert refresher answers nothing and starts no thread | `refresh::tests::no_refresher_never_yields` | unit | none | `cargo test --lib 'refresh::tests::'` |
| refresh-worker: No binary means no worker | `refresh::tests::start_without_a_binary_is_inert` | unit | none | same |
| refresh-worker: One request produces the file result and then the merged one | `refresh::tests::the_worker_answers_with_files_then_merged` — `recv_timeout(10s)` twice | fs-edge + thread | real thread, real `ScratchDir`, `cli::FakeCli` | same |
| refresh-worker: A CLI that fails still produces the file result | `refresh::tests::a_failing_cli_still_sends_the_files_result` | fs-edge + thread | as above | same |
| refresh-worker: Queued requests are folded into one cycle | `refresh::tests::drain_and_fold_unions_queued_requests` — the extracted `drain_and_fold` called directly, no thread — **and** `changes::tests::selection_union_*` | unit; unit | none; none | `cargo test --lib 'refresh::tests::'`; `cargo test --lib 'changes::tests::'` |
| refresh-worker: Dropping the refresher ends the worker | `refresh::tests::dropping_the_refresher_disconnects_the_channel` — asserts channel disconnection, never a thread count | fs-edge + thread | real thread | `cargo test --lib 'refresh::tests::'` |
| refresh-worker: The worker writes nothing inside the repository | `refresh::tests::the_worker_writes_nothing` (snapshot pair around a full cycle) | fs-edge + thread | real `ScratchDir`, `cli::FakeCli` | same |
| live-updates: `Refresh` has no `Default` and no site elides a field | `NODEFAULT-UI` with a fourth type, plus the `Refresh` destructuring companion | check + unit | real grep | `TYPES="Dashboard Filter Detail Refresh" SCAN_MIN=90 sh $CHECKS/NODEFAULT-UI.sh` |
| live-updates: `r` refreshes outside filter mode and types inside it | `ui::app::tests::r_maps_to_refresh_outside_filter_mode` | unit | none | `cargo test --lib 'ui::app::tests::'` |
| live-updates: `Refresh` sets the flag and touches nothing else | `ui::app::tests::refresh_sets_requested_and_changes_nothing_else` | unit | none | same |
| live-updates: No action mutates the change set | `ui::app::tests::no_action_mutates_changes` (landed, modified: an exhaustive thirteen-variant array and a thirteen count assertion) + `ui::app::tests::no_action_mutates_a_task_item` (landed, same modification) | unit | none | same |
| live-updates: The selection follows its change when the list shifts | `ui::app::tests::adopt_keeps_the_selection_by_name` | unit | none | same |
| live-updates: The selection is clamped when its change is gone | `ui::app::tests::adopt_clamps_when_the_change_is_gone` | unit | none | same |
| live-updates: A filter narrows what the name is resolved against | `ui::app::tests::adopt_resolves_the_name_against_the_filtered_list` | unit | none | same |
| live-updates: A forced reload re-reads the same tab and keeps the offset | `ui::app::tests::a_forced_reload_keeps_the_scroll` | unit | `RecordingReader` | same |
| live-updates: An unchanged key with no forced reload re-reads nothing | `ui::app::tests::the_selected_tabs_file_is_read_once_and_reused` (landed, extended with `refresh.reload` false) | unit | `RecordingReader` | same |
| live-updates: A tab move under a forced reload still resets the scroll | `ui::app::tests::a_tab_move_under_a_forced_reload_resets_the_scroll` | unit | `RecordingReader` | same |
| live-updates: The startup request is issued before the first wait | `ui::driver::tests::the_startup_request_precedes_the_first_wait` | unit | scripted doubles, `TestBackend` | `cargo test --lib 'ui::driver::tests::'` |
| live-updates: A result is adopted before the frame that shows it | `ui::driver::tests::a_result_is_adopted_before_the_frame` at 60 and 120 | view | scripted doubles, `TestBackend` | same |
| live-updates: A filesystem batch becomes one selection | `ui::driver::tests::an_fs_batch_becomes_one_selection` — exact equality on the recorded requests | unit | scripted `FsEvents` | same |
| live-updates: A watcher error is recorded once and the loop continues | `ui::driver::tests::a_watch_error_is_recorded_once` + `ui::view::tests::a_watch_problem_is_the_first_row` at 60 and 120 | unit + view | scripted `FsEvents`, `TestBackend` | same; `cargo test --lib 'ui::view::tests::'` |
| live-updates: The wait shortens to the debounce deadline | `ui::driver::tests::the_wait_shortens_to_the_debounce_deadline` — asserts `Script::timeouts()` | unit | scripted doubles | `cargo test --lib 'ui::driver::tests::'` |
| live-updates: An inert live tier leaves the loop exactly as it was | every landed `ui::driver::tests::` scenario, re-run unchanged with `watch::none()`/`refresh::none()` | unit | inert doubles | same |
| live-updates: A full live run leaves the change tree byte-identical | `ui::tests::live::a_live_watcher_over_the_tree_writes_nothing` (snapshot pair + discriminating control, asserting byte-identity only) at 60 and 120 | acceptance | real `ScratchDir`, real `watch::start`, real `read_artifact`, scripted `Refresher` | `cargo test --lib 'ui::tests::live::'` |
| live-updates: `r` requests exactly one full refresh through the loop | `ui::tests::live::r_forces_a_refresh_through_the_loop` at 60 and 120 — exact equality on `requests()`, sound because its `ScriptedFs` can contribute nothing | acceptance | real `ScratchDir`, scripted `FsEvents`, `RecordingRefresher`, real `read_artifact` | `cargo test --lib 'ui::tests::live::'` |
| live-updates: The source-level guarantee holds tree-wide | `READONLY-UI` extended to `src/watch.rs` and `src/refresh.rs`, plus `OPENSPEC-UNTOUCHED` | check | real grep, real git | `UI_MIN=11 EXTRA="src/watch.rs src/refresh.rs" sh $CHECKS/READONLY-UI.sh`; `BASE=f9b42e8 sh $CHECKS/OPENSPEC-UNTOUCHED.sh` |
| live-updates: A watch problem is the list's first row | `ui::view::tests::a_watch_problem_is_the_first_row` at 60 and 120 | view | `TestBackend` | `cargo test --lib 'ui::view::tests::'` |
| live-updates: Refresh problems precede change-set problems | `ui::list::tests::refresh_and_change_problems_in_order` at 38 and 58 + `ui::view::tests::refresh_problems_precede_change_problems` at 60 and 120 | unit + view | `TestBackend` | `cargo test --lib 'ui::list::tests::'`; `cargo test --lib 'ui::view::tests::'` |
| live-updates: No refresh problem draws no extra row | `ui::list::tests::refresh_problems_lead_the_rows` (its no-problem control) + `ui::view::tests::no_refresh_problem_draws_no_extra_row` | unit + view | `TestBackend` | same |
| dashboard-loop: `Dashboard` has no `Default` and no site elides a field | `NODEFAULT-UI` (four types, `SCAN_MIN=90`) + the nine-field / three-field destructuring companions | check + unit | real grep | `TYPES="Dashboard Filter Detail Refresh" SCAN_MIN=90 sh $CHECKS/NODEFAULT-UI.sh` |
| dashboard-loop: The pure view files name no I/O API | `NOIO-VIEW`, unchanged and re-run | check | real grep | `sh $CHECKS/NOIO-VIEW.sh` |
| dashboard-loop: The shell never names the CLI seam | `NOCLI-SHELL` at `UI_MIN=11` | check | real grep | `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh` |
| dashboard-loop: Change literals live only in the gated file | `NOLIT-CHANGE` at `MIN=21` + a planted `Change {` in `src/refresh.rs` | check | real grep | `MIN=21 sh $CHECKS/NOLIT-CHANGE.sh` |
| dashboard-loop: The render path names no channel, thread, lock, or clock | `NOBLOCK` legs 1 and 2, plus two planted violations | check | real grep | `UI_MIN=11 sh $CHECKS/NOBLOCK.sh` |
| dashboard-loop: No test sleeps and then asserts something has already happened | `NOSLEEP` legs 1 and 2, with its built-in self-controls and a planted sleep-then-assert | check | real grep | `MIN=24 SLEEP_MIN=3 sh $CHECKS/NOSLEEP.sh` |
| dashboard-loop: Both quit keys quit and neither near-miss does | landed `ui::app::tests::` test, unchanged | unit | none | `cargo test --lib 'ui::app::tests::'` |
| dashboard-loop: A released quit key does not quit | landed test, **extended** with `Char('r')` Release/Repeat | unit | none | same |
| dashboard-loop: Enter and Esc move between the two routes | landed test, **extended** with a `refresh.requested` assertion | unit | none | same |
| dashboard-loop: `Esc` dismisses one layer at a time | landed test, unchanged | unit | none | same |
| dashboard-loop: Navigation and filter keys are distinguished from near misses | landed test, **extended** with `r`/`R`/`Ctrl-r` | unit | none | same |
| dashboard-loop: Non-key events are ignored without panicking | landed test, **extended** with `Paste("r")` | unit | none | same |
| dashboard-loop: The first frame is on screen before the first event is read | landed `ui::driver::tests::` test, **extended** with an inert `Live` | unit | inert doubles, `TestBackend` | `cargo test --lib 'ui::driver::tests::'` |
| dashboard-loop: Timeouts are not events and do not end the loop | landed test, extended with an inert `Live` | unit | inert doubles | same |
| dashboard-loop: A backend draw failure ends the loop rather than spinning | landed test, extended with an inert `Live` | unit | inert doubles, failing backend | same |
| dashboard-loop: Ctrl-C ends the loop | landed test, extended with an inert `Live` | unit | inert doubles | same |
| dashboard-loop: An ignored key redraws and keeps waiting | landed test, extended with an inert `Live` | unit | inert doubles | same |
| dashboard-loop: A route change is visible in the next frame | landed test, extended with an inert `Live` | unit | inert doubles | same |
| dashboard-loop: A scratch repository is loaded from disk with no binary present | landed `ui::tests::load::` test, **extended** with the `refresh` assertion | fs-edge | real `ScratchDir` | `cargo test --lib 'ui::tests::load::'` |
| dashboard-loop: No repository above the starting directory | landed test, extended with `refresh.problems` empty | fs-edge | real `ScratchDir` | same |
| dashboard-loop: The configured archived count is passed through | landed test, unchanged | fs-edge | real `ScratchDir` | same |
| dashboard-loop: Loading writes nothing | landed test, unchanged — the watcher half of the claim is `watch::tests::a_started_watcher_writes_nothing`, because no `ui::` test may open a watcher | fs-edge | real `ScratchDir` | same |
| artifact-content: The selected tab's file is read once and reused | landed `ui::app::tests::` test, extended with `refresh.reload` false | unit | `RecordingReader` | `cargo test --lib 'ui::app::tests::'` |
| artifact-content: A forced reload re-reads the same key and keeps the scroll | `ui::app::tests::a_forced_reload_keeps_the_scroll` | unit | `RecordingReader` | same |
| artifact-content: A tab move under a forced reload still resets the scroll | `ui::app::tests::a_tab_move_under_a_forced_reload_resets_the_scroll` | unit | `RecordingReader` | same |
| artifact-content: Switching the tab re-reads, and so does switching the change | landed test, unchanged | unit | `RecordingReader` | same |
| artifact-content: Two changes with the same name are distinguished by directory | landed test, unchanged | unit | `RecordingReader` | same |
| artifact-content: A multi-file artifact is concatenated in path order with a separating newline | landed test, unchanged | unit | `RecordingReader` | same |
| artifact-content: An unreadable file names its reason and does not lose its siblings | landed test, **extended** with the forced-reload clearing clause | unit | `RecordingReader` | same |
| artifact-content: An artifact with no resolved paths reads nothing at all | landed test, unchanged | unit | `RecordingReader` | same |
| artifact-content: A tab out of range for the newly selected change is clamped before the read | landed test, unchanged | unit | `RecordingReader` | same |
| artifact-content: An empty visible list clears the detail | landed test, **extended** with the flag-is-still-cleared clause | unit | `RecordingReader` | same |
| artifact-content: The loop syncs before it draws | landed `ui::driver::tests::` test, extended with an inert `Live` | unit | inert doubles | `cargo test --lib 'ui::driver::tests::'` |
| change-rows: Active rows render at both mandated widths | landed `ui::view::tests::` test, **extended** with the byte-identical-buffer clause | view | `TestBackend` | `cargo test --lib 'ui::view::tests::'` |
| change-rows: A watch problem leads the list, above a change-set problem | `ui::view::tests::refresh_problems_precede_change_problems` at 60 and 120 + `ui::list::tests::refresh_and_change_problems_in_order` at 38 and 58 | view + unit | `TestBackend` | same; `cargo test --lib 'ui::list::tests::'` |
| change-rows: The row grammar places the marker, the name, and the progress cell | landed test, unchanged | unit | none | `cargo test --lib 'ui::list::tests::'` |
| change-rows: A name too long for the field is truncated with an ellipsis | landed test, unchanged | view | `TestBackend` | `cargo test --lib 'ui::view::tests::'` |
| change-rows: A field too narrow for both drops the progress cell whole | landed test, unchanged, plus the new `ui::list::tests::a_refresh_problem_row_degrades_at_narrow_widths` for the problem row's own degradation at widths 1 and 0 | unit | none | `cargo test --lib 'ui::list::tests::'` |
| cli-changes: A two-change repository drives exactly three invocations | landed `changes::tests::` test, **extended** with the same call through `from_cli_cached` | unit | `cli::FakeCli` | `cargo test --lib 'changes::tests::'` |
| cli-changes: No status invocation is made even when a change's apply call fails | landed test, unchanged | unit | `cli::FakeCli` | same |
| cli-changes: `from_cli` names no process-spawn API | `NOSPAWN-GREP` at `MIN=21` | check | real grep | `MIN=21 sh $CHECKS/NOSPAWN-GREP.sh` |
| plugin-build: Exactly one binary target is produced at the release path | `DEPS` leg 1 | check | real cargo | `WORK=$WORK sh $CHECKS/DEPS.sh` |
| plugin-build: The declared dependency set is exactly the argued crates | `DEPS` legs 2a, 2b, 2a-bis, 2c, 2d (edited on disk: six crates, `notify`'s two-sided defaults-off clause, and the no-debouncer clause) | check | real cargo | same |
| plugin-build: Every package in the normal build graph declares an MSRV no higher than the crate's | `DEPS` leg 4, unedited | check | real cargo | same |
| plugin-build: The resolved build graph is small and proc-macro-free | `GRAPH-SNAP`, edited on disk (four-name macOS/Linux delta) against a regenerated `tests/fixtures/build-graph.txt` | check | real cargo | `sh $CHECKS/GRAPH-SNAP.sh` |
| plugin-build: Each dependency is genuinely needed rather than incidental | `DEPS` leg 5, edited on disk to add `needed notify "watch::RealFsEvents"` | check | real cargo | `WORK=$WORK sh $CHECKS/DEPS.sh` |
| plugin-build: No JSON parsing reaches the subprocess seam | `NOJSON-SEAM`, unchanged | check | real grep | `sh $CHECKS/NOJSON-SEAM.sh` |
| tasks-checklist: Every printable key leaves the change tree byte-identical | landed `ui::tests::detail::tasks_tab_is_read_only`, unchanged — its `!`..`~` sweep already presses `r` | acceptance | real `ScratchDir`, real `read_artifact`, `TestBackend` | `cargo test --lib 'ui::tests::detail::'` |
| tasks-checklist: No action mutates a task item | landed `ui::app::tests::no_action_mutates_a_task_item`, **modified** to sweep thirteen variants and assert the count is thirteen | unit | none | `cargo test --lib 'ui::app::tests::'` |
| tasks-checklist: The dashboard names no write API | `READONLY-UI` with `EXTRA="src/watch.rs src/refresh.rs"`, plus its two planted controls | check | real grep | `UI_MIN=11 EXTRA="src/watch.rs src/refresh.rs" sh $CHECKS/READONLY-UI.sh` |
| detail-scroll: `Detail` has no `Default` and no site elides a field | `NODEFAULT-UI` over four types + the four destructuring companions, `Detail`'s naming five and `Dashboard`'s naming nine | check + unit | real grep | `TYPES="Dashboard Filter Detail Refresh" SCAN_MIN=90 sh $CHECKS/NODEFAULT-UI.sh` |
| detail-scroll: Startup leaves the detail empty and unscrolled | landed `ui::tests::load::startup_leaves_the_detail_empty_and_unscrolled`, **extended** with `refresh.reload` false at both construction sites | fs-edge | real `ScratchDir` | `cargo test --lib 'ui::tests::load::'` |
| quality-gates: The harness renders a state value with no repository on disk | landed `testutil` test, unchanged | view | `TestBackend` | `cargo test --lib 'testutil::'` |
| quality-gates: Both widths are exercised for every view scenario | `WIDTHS` at `WIDTHS_MIN=81` + `testcount --lib 'ui::view::tests::' 81` | check | real grep, real cargo | `WIDTHS_MIN=81 sh $CHECKS/WIDTHS.sh` |
| quality-gates: The coverage floor is unchanged by the new module | `make coverage` + `NOWAIVER` | check | real cargo | `make coverage`; `sh $CHECKS/NOWAIVER.sh` |

Every one of the change's **111** scenarios appears at least once: 103 in the nine spec
files the first review round saw, plus three in `tasks-checklist`, two in `detail-scroll`, one
new `watch-invalidation` scenario (the deferral cap), one new `refresh-worker` scenario (a
rejected schema), and one new `live-updates` scenario (the split `r`-requests claim).

## Decisions

**1. Two new modules outside `src/ui/`, rather than one module inside it.** The watcher and
the worker could have lived under `src/ui/` as loop collaborators. Putting them outside is
what makes the landed `NOCLI-SHELL` check — "no file under `src/ui/` names `OpenspecCli`" —
the structural proof that the CLI is off the render path, at zero cost. It also leaves the
`NOIO-VIEW` pure set at eight and `src/ui/`'s file count at eleven, so four landed checks
run at their landed floors and only their tree-wide file counts move.
*Alternative rejected:* one `src/ui/live.rs`. It would have forced `NOCLI-SHELL` to grow an
exemption, which is how a confinement check rots into a rubber stamp.

**2. The debounce takes `now` as a parameter.** `Debounce::push(paths, now)` and
`take_due(now)` are pure. The one real `Instant::now()` call sits in
`watch::RealFsEvents::drain`, the crate's fourth one-line binding to the real world beside
`cli::npm_prefix`, `config::env_lookup`, and `ui::read_artifact`.
*This is the single decision that answers hazard 1.* Every window assertion is
`take_due(t0 + 149ms)` is `None`, `take_due(t0 + 150ms)` is `Some(_)` — deterministic on a
cold machine, a loaded machine, and a machine whose clock is being adjusted.
*Alternative rejected:* a `Clock` trait. A parameter is smaller, needs no double, and cannot
be forgotten at a call site the way an injected trait object can be `Instant::now()`-ed
around.

**3. The debounce lives behind the `FsEvents` seam, not in `run_loop`.** `drain()` returns a
batch only when the window has closed, and `pending_in()` reports the remaining time as a
`Duration`. `run_loop` therefore needs no clock at all, which is what makes the `NOBLOCK`
leg-2 rule ("no file under `src/ui/` names `Instant::now`, tests included") satisfiable
rather than aspirational.
*Alternative rejected:* passing a `&dyn Fn() -> Instant` into `run_loop`, mirroring
`ArtifactReader`. It would have put a clock into the render seam's tests, which is the
exact place a timing flake is most likely to be reintroduced.

**4. The worker sends two results per request, `Files` then `Merged`.** The file re-read is
sub-millisecond and the CLI cycle is 200–400ms per change; splitting them means the loop
never waits for either, `run_loop` performs no I/O (so `NOIO-VIEW` is unweakened), and the
dual-source model becomes an assertion about two successive frames rather than a comment.
*Alternative rejected:* `run_loop` calling `changes::from_files` itself when a batch
arrives. That would put `std::fs` into `src/ui/driver.rs` and break the pure set — the
change would have failed at its one job.

**5. `Selection` narrows CLI work only; files are always re-read in full.** Walking
`openspec/changes/` costs well under a millisecond and is the only way to learn a change
appeared or vanished. A "selective" file read would be a second enumeration rule and a
second place the two producers could disagree.

**6. `openspec list --json` runs on every cycle, whatever the selection.** It carries the
repository-root guard and every change's progress pair. This is what makes a mis-classified
path *harmless*: a change whose artifacts were wrongly left cached still shows the correct
`[completed/total]` on the very next cycle, because that number never comes from the cache.

**7. `from_cli` is redefined, not duplicated.** `from_cli(cli, repo)` becomes
`from_cli_cached(cli, repo, &Selection::All, &mut CliCache::default())`. One implementation,
and every landed `cli-changes` and `schema-cli-fallback` test constrains both entry points.
*Alternative rejected:* a second function. Two CLI producers is exactly the divergence
`change-model`'s two-producer gate exists to prevent, one level up.

**8. Classification is conservative: an unrecognised path under `openspec/` is
`Repository`.** A wrong `Repository` costs one extra `list --json` on a cycle that was
happening anyway. A wrong `Outside` silently stops the pane updating — a failure nobody
notices until they trust a stale number. The per-change rule is proven by an exact
set-equality test on `changes/<name>/…`; the fallback is proven not to swallow it.

**9. `adopt` preserves the selection by name, not by index.** A refresh can add a change
alphabetically above the selected one; an index preserved across that shift silently moves
the reader to a different change mid-read. The name is resolved against `visible()`, because
`selected` indexes the visible list and a `/` filter may be active.

**10. `refresh.reload` forces a re-read without resetting the scroll; a key change still
resets it.** `sync_detail`'s cache key is `(change dir, tab)` and does not change when a
file's **content** does — so without the flag, an agent's save would be invisible on the tab
the reader is looking at. Resetting the scroll only on a key change is what keeps a reader
where they were. Both halves are separately asserted, so neither can be attributed to the
other.

**11. `reload` lives on `Refresh`, not on `Detail`.** `Dashboard` gains one field either
way; putting it on `Refresh` leaves `Detail` at exactly five fields and leaves every
`Detail { … }` literal in the suite untouched. `dashboard-loop`'s "exactly five" clause
survives unedited.

**12. No debouncer crate.** `notify-debouncer-mini` 0.7.0 costs five further packages —
`tempfile`, `fastrand`, `getrandom`, `once_cell` among them — as **normal** dependencies of
the shipped binary; `notify-debouncer-full` costs two (`file-id`, for rename tracking this
plugin does not do) and declares MSRV 1.85. Both were measured, not estimated. The
hand-rolled `Debounce` is about twenty lines, adds zero packages, and — decisively — takes
`now` as a parameter, which neither crate offers.

**13. `macos_fsevent`, not `macos_kqueue`, and not `default-features = false` alone.**
Measured: `default-features = false` with no feature **fails to compile on macOS**
(`error[E0432]: unresolved import fsevent_sys`), because `notify`'s fsevent module is gated
on `not(feature = "macos_kqueue")`. FSEvents recurses in the kernel — one watch for the whole
subtree; `notify`'s kqueue backend opens a file descriptor per entry and reports only that
*something* in a directory changed. The chosen spelling resolves a graph byte-identical to a
bare `notify = "8.2.0"`, since `default = ["macos_fsevent"]`, and satisfies `DEPS` leg 2a's
`uses_default_features is False` clause.

**14. `LoopSummary` gains no field.** Requests and results are observed through the doubles'
own recorders. Adding a field would have edited eight landed `LoopSummary { frames, polls }`
literals for no assertion the recorders do not already carry.

**15. `TICK` stays 250ms; `poll_timeout` shortens the wait instead.** Lowering `TICK` would
modify a pinned `dashboard-loop` value and cost CPU on an idle pane. `poll_timeout(tick,
pending_in)` is a pure function of two `Duration`s that removes the post-debounce half of the
latency; the pre-debounce half (up to one tick to notice an event) is the accepted cost.

**16. `CliCache` implements `Default`, and that is not a violation.**
`change-model`'s gate covers `Change`, `ChangeSet`, `ArtifactRef`, and `Origin` in
`src/changes.rs`; a private-fielded cache is none of those, and clippy's
`new_without_default` would demand it anyway. Recorded here so a reviewer does not read it as
a breach.

**17. The per-change-invalidation proof is single-threaded.** `changes::from_cli_cached` is
called directly with a recording fake and an **exact equality** on the recorded argument
vectors. A negative assertion made across a thread boundary — "beta was not called" — can
pass because an interleaving happened to be favourable, which is the same defect as a timing
assertion wearing different clothes. The worker's own tests prove only that a request goes in
and two results come out, and every one of them asserts **after** receiving a result, which
is a real happens-before edge rather than a hope.

**18. `NOSLEEP` forbids the sleep-then-assert *shape*, not `sleep`.** Measured at planning
time: `main` already carries **three** sleeps, all correct — `src/cli.rs`'s
`a_program_that_reads_stdin_returns_rather_than_blocking` and `tests/cli.rs`'s two `try_wait`
polls, one of which carries a comment recording the very flake that produced the rule. A
blanket prohibition would have been red on an unmodified tree and the only ways out would
have been deleting three correct tests or exempting two files — `NOTABSEAM` shipped red on an
unmodified tree in this repository once already. The check instead requires every `#[test]`
span naming a sleep to also name a `deadline` and a loop, carries a floor on the number of
sites it **found** (three), and runs a **self-contained negative control on every
invocation**: a synthetic sleep-then-assert span must be reported and a synthetic
deadline-bounded poll must not, or the scan declares itself broken.

**19. `WATCHSEAM` does not match `EventKind`.** Measured at planning time: a bare `EventKind`
alternative — the obvious one, since it is `notify`'s own event type — matches `KeyEventKind`
and `MouseEventKind` at **twelve** sites of correct, unmodified code in `src/ui/app.rs`. The
pattern is path-qualified and token-specific instead, and the bare word `notify` is excluded
too, being an ordinary English verb.

**20. `NOBLOCK` matches `.join()` with empty parentheses, not a bare `.join(`.** `Path::join`
and `str::join` both take an argument and are ordinary correct code; a `JoinHandle`'s `join`
takes none. A bare pattern would be a false red the first time the loop built a path.

**21. `NOBLOCK` has a third leg, inside the two seam modules.** Legs 1 and 2 search
`src/ui/driver.rs` and `src/ui/**` only, so they cannot see a blocking `FsEvents::drain` or a
blocking `Refresher::take_result` — the two functions the loop calls on **every frame**. A
`drain` written `self.rx.recv_timeout(Duration::from_millis(150))` and a `take_result` written
`self.rx.recv_timeout(Duration::from_millis(400)).ok()` satisfy `NOCLI-SHELL`, both existing
`NOBLOCK` legs, `NOSLEEP`, `WATCHSEAM`, every trait signature, and every frame-and-poll count
this change asserts — while delaying each draw by more than half a second. Leg 3 forbids a
blocking receive in `src/watch.rs` at all, and in `src/refresh.rs` before its single
`thread::spawn` (everything after that point is the worker body, which may block freely), and
requires `try_recv` in both as its positive control.
*Alternative rejected:* a runtime test asserting a frame takes under N milliseconds. That is
an elapsed-time assertion — hazard 1 reintroduced in the one place this change exists to
eliminate it. The gate is structural on purpose.

**22. The folding rule is proved single-threaded, and the threaded version is deleted.**
`drain_and_fold` is extracted as a named private function and driven directly from a test with
a pre-loaded receiver. The obvious threaded version — send `Only({a})`, `Only({b})`, `All`,
then read the fake's calls "after the last `Merged` result" — cannot fail: the union is `All`
so "nothing outside the folded selection" is a tautology; the worker's cache starts empty so
its first cycle applies to every change anyway; and *which* result is the last depends purely
on interleaving. A deleted vacuous test is strictly better than a green one.

**23. `Debounce` caps its deferral at `DEBOUNCE_MAX` (1 second).** A sliding window with no
ceiling starves indefinitely under a writer saving more often than every 150ms — which is
exactly this change's motivating case, an agent editing `tasks.md`, then a spec, then a design
doc. Without the cap the pane freezes precisely when it is most useful. The cap is a `min` in
`push` and is proved by an injected-`now` scenario, so it costs no test time.

**24. `pending_in` on the seam takes no `Instant`; `RealFsEvents` caches what `drain`
computed.** `FsEvents::pending_in(&self)` returns the `Option<Duration>` the previous `drain`
stored from `debounce.pending_in(now)`, using the same `now` `drain` already captured. The
alternative — `pending_in` calling `Instant::now()` itself — would give `src/watch.rs` two
clock bindings and make `SPEC.md` → Architecture's "one binding" claim false. Sound because
`run_loop` calls `drain` and then `pending_in` in the same iteration, so the value is at most
one statement old.

**25. The one real-watcher poll sleeps 10ms between iterations rather than spinning.** A sleep
*inside* a deadline-bounded poll cannot make an assertion premature — the condition is
re-tested after every sleep, which is the whole point and exactly what `tests/cli.rs` already
does here. A `yield_now` spin would hold a core for the window and, on a loaded two-core
runner, compete for CPU with the `notify` thread producing the event it waits for. `NOSLEEP`
leg 2 is therefore scoped to `src/ui/` rather than banning sleep in `src/watch.rs` too; leg 1
governs that file and accepts the deadline-bounded shape.

**Visual design source:** none exists for this repository and this change adds no new visual
grammar of its own — the one row it draws is `change-rows`' landed `! `-prefixed problem row.
The section is skipped deliberately rather than invented.

## Risks / Trade-offs

- **A real-watcher test could be flaky on a filesystem that does not support change
  notification** → Exactly **one** scenario opens a real watch and waits for an event, at a
  **ten-second** deadline, with a failure message naming the written path and stating that
  the filesystem may not support change notification, so a platform limitation is
  distinguishable from a defect. Every other watcher scenario is either the deterministic
  `PathNotFound` failure or a scripted double.
- **The one real-watcher poll costs wall-clock time in one test** → It sleeps 10ms between
  iterations of a deadline-bounded loop, so the happy path costs about the 150ms debounce
  window and roughly fifteen wake-ups, and the failure path costs ten seconds of an idle
  thread rather than ten seconds of a pegged core (Decision 25).
- **`notify`'s `EventHandler for Sender` swallows send errors (`let _ = self.send(event)`)** →
  Not a panic risk in either field drop order. Dropping `RealFsEvents` drops our `Receiver`;
  `notify`'s thread's later sends then fail silently by construction. On macOS the watcher's
  `Drop` joins its thread first; on Linux `INotifyWatcher::drop`'s two `unwrap`s are on
  `notify`'s **internal** control channel and waker, not on our event channel, and nothing in
  this design can kill that thread out from under a live watcher. Recorded so a later reader
  does not re-derive it.
- **`notify`'s own `FsEventWatcher::drop` busy-waits (`while CFRunLoopIsWaiting(runloop) == 0
  { yield_now() }`) before joining** → It is on the pane's shutdown path and on the drop of
  the two real-watcher tests' watchers, never on the render path, and it is `notify`'s code
  rather than this crate's. Recorded so it is not mistaken for a defect this change introduced.
- **macOS `FsEventWatcher` re-spawns its thread on every `watch()`/`unwatch()`** → `watch` is
  called exactly once, from `watch::start`; there is no `unwatch`, no re-watch, and no
  restart-on-error path anywhere in this design — the removed-directory case deliberately
  chooses "stale rather than broken".
- **`Dashboard`'s ninth field touches every literal in the suite** → That is the mechanism,
  not the cost: no `Default` and no `..` means `E0063` at every site, which is how
  `tasks-tab` landed `ArtifactRef::tracks_tasks` across 32 sites without a silent default.
- **The worker could outlive a `Dashboard` and send into a dead channel** → `mpsc::Sender`'s
  send fails silently once the receiver is dropped, and the worker returns on that failure.
  Asserted by channel disconnection, never by a thread count, because Linux's shutdown paths
  signal without joining.
- **A wrong classification leaves a change's artifacts stale** → Progress is always fresh
  (decision 6), so the visible cost is an artifact list one cycle behind, and `r` corrects it
  immediately.
- **Worst-case latency from save to corrected frame is `TICK` + `DEBOUNCE` + the CLI's own
  200–400ms, about 800ms** → Accepted. `SPEC.md` says "approximately 150ms" of the *debounce*,
  not of end-to-end latency, and `poll_timeout` already removes the post-debounce half.
- **`notify` 9.0.0 will declare MSRV 1.88, exactly this crate's floor** → 8.2.0 is the latest
  **stable** and declares 1.77; the pre-release is not adopted, and `DEPS` leg 4 prints the
  at-the-floor set on every run, so the day it changes is visible in a check's output.
- **`GRAPH-SNAP`'s hard-coded `[ "$d" = "linux-raw-sys " ]` becomes false** → Known before
  the first line of code: the check is **edited on disk** in group 6, written to
  `$CHECKS/GRAPH-SNAP.sh`, shown to have landed with a diff, and run. `plugin-build`'s
  modified requirement states the new four-name list.

## Migration Plan

None needed. No on-disk format changes, no manifest change, and no configuration change, so a
linked plugin needs no re-link and an installed one needs only the ordinary rebuild
`herdr plugin install` already performs. Rollback is `git revert` plus `cargo build`: the six
packages `notify` adds are removed by the same revert of `Cargo.toml` and `Cargo.lock`.

Deploy order within the change is the task groups' order: the dependency lands in group 6,
which is also where `DEPS`, `GRAPH-SNAP`, and `tests/fixtures/build-graph.txt` are edited and
regenerated together, so no intermediate commit leaves a manifest and a snapshot disagreeing.

## Open Questions

None. Three questions were resolved during planning and are recorded rather than left open:

1. *Does the debounce belong in `run_loop` or behind the seam?* Behind the seam (decision 3),
   because it is what keeps a clock out of `src/ui/`.
2. *Does a `Selection` narrow the file read too?* No (decision 5): the file read is the only
   source of list membership and costs nothing.
3. *Is `notify`'s `default-features = false` enough on macOS?* No — measured, it fails to
   compile (decision 13).
