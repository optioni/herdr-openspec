## Context

A change's schema is resolved by two producers. `changes::from_files` calls `schema::load(repo,
name)`, which only knows the repository tier `openspec/schemas/<name>/`. `changes::from_cli_cached`
does the same, and on `LoadError::NotVendored` asks `openspec schema which <name> --json` for the
schema's directory (`resolve_cli_schema_uncached`, `src/changes.rs:1497`). That answer is
cached in a `HashMap` owned by one `from_cli_cached` call, so it is lost at the end of the
call.

`spec-driven` lives only in the CLI package's own tier. So, for a repository on that schema:

- the file producer records `<repo>/openspec/schemas/spec-driven/schema.yaml is not
  vendored: no schema.yaml there` on every change it builds;
- `merge` keeps that file-side message beside the CLI's corrected artifacts, because
  `change-merge` forbids dropping problems;
- archived changes are built only by the file producer, so they never get artifacts.

`degraded-states` now draws a change's problems as `!` rows in its detail pane, which is what
made the first point visible. Both were reproduced live in `~/Code/dungeons-and-dragons` with
openspec 1.13.2.

The refresh worker (`src/refresh.rs::worker_body`) already owns a `CliCache` for its whole
lifetime and threads it into `from_cli_cached`. Step 1 builds the file set through
`from_files_with_listing` and sends it. Step 2 derives the worktree family, runs
`from_cli_cached`, merges, and overlays. The idle re-check overlays the remembered merged set
through `overlay_family`, which calls `from_files_owned` per member.

## Goals / Non-Goals

**Goals:**

- When an `openspec` binary is present, a schema the CLI can locate never produces a "not
  vendored" problem: not for active changes, archived ones, or worktree members.
- `schema which` runs once per schema name per worker, not once per cycle.
- The fast file answer still spawns nothing, and the render path is untouched.

**Non-Goals:**

- File mode keeps today's behaviour exactly.
- No filtering of problems in `merge`.
- No caching of parsed schemas across cycles.
- No change to `Change`, `ChangeSet`, or any view.

## Boundaries

| Module | Change | Pattern followed |
|---|---|---|
| `src/changes.rs` | New `SchemaLocations` (a `dirs` map from name to directory, and a `misses` map from name to rendered problem) held as a field of `CliCache`, with a read accessor. The file producer's per-call schema cache becomes `SchemaLoads<'a> { cache: HashMap<String, CachedSchemaLoad>, locations: &'a SchemaLocations }`, so `build_change` keeps its seven parameters (Decision 9). `load_schema_cached` falls back to `schema::load_dir` on `NotVendored` when the locations hold a usable directory. `from_files_with_listing`, `from_files_owned` and `overlay_family` take `&SchemaLocations` and build a `SchemaLoads` from it. `resolve_cli_schema` and `resolve_cli_schema_uncached` take `&mut SchemaLocations` and consult and fill it. New `locate_schemas(cli, repo, &ChangeSet, &mut CliCache) -> usize`. | The same file already owns both producers, `CliCache`, and the `OpenspecCli` calls for `schema which`. The new spawn site is the existing trait object, not a new process API. |
| `src/refresh.rs` | `worker_body`: step 1 passes `cache.locations()` to `from_files_with_listing` (line 545) and to its `overlay_family` (line 554), keeping a clone of `list_changes`' three results. Step 2 calls `locate_schemas` and, when it returns more than 0, rebuilds `files` with `from_files_with_listing` from that clone (no second walk, so `base_archive_dirs` cannot drift from it) before `from_cli_cached`. Step 2's `overlay_family` (line 581) and the idle re-check's (line 502) pass `cache.locations()`. That is all four call sites. | The existing worker-thread pattern. Everything new runs below the single `thread::spawn`, so `NOBLOCK`'s pre-spawn slice is unchanged. |
| `src/ui/` | none | — |
| `src/cli.rs` | none | The `OpenspecCli` trait is used as is; no process is spawned outside `cli`. |
| `SPEC.md`, `IMPLEMENTATION-ORDER.md` | Prose: the Schema paragraph, the "Schema not vendored" degraded row's **Behaviour** cell (the **Condition** cell is unchanged, because `tests/degraded_coverage.rs` binds rows by condition), and a roadmap row. | Docs move with the change. |
| `tests/degraded-coverage.toml` | Re-point the `covers` line ranges in `src/changes.rs` and `src/refresh.rs` that the edit moves. Each re-pointed range must hold the same statements as before. `covers-check` cannot tell, because it accepts any range that holds an executed statement. | As `homebrew-probe` did, plus a before/after text comparison. |

`from_files(repo, scope)` keeps its signature and passes `&SchemaLocations::default()`, so
`ui`'s file-mode path and every existing caller compile and behave unchanged.

**The `Change` type is not altered.** The two producers agree because both read the same
`SchemaLocations` and load through the same `schema::load_dir`. Within step 2 the file set
the merge sees is built with every location the CLI producer will use in that same cycle.
`locate_schemas` runs first and fills the map, and `from_cli_cached` can only add a location
for a name the locate pass did not see. That happens only for a CLI-only change, or for a change
whose CLI-reported schema name differs from its file-declared one. `change-merge`'s join
rules already govern that disagreement.

## Contracts

Every new signature is crate-internal (`pub(crate)` or `pub` inside a binary crate with no
external consumer). This is additive.

- `CliCache` gains a private `locations: SchemaLocations` field and
  `pub(crate) fn locations(&self) -> &SchemaLocations`. It still derives `Default`;
  `SchemaLocations` derives `Debug, Clone, Default, PartialEq, Eq`. It is not one of
  `change-model`'s no-`Default` types.
- `locate_schemas` returns the number of locations it newly learned. It never returns a
  `Result` and records no problem.
- Every lookup outcome that leaves no usable location becomes a `misses` entry, whether it
  ran inside `locate_schemas` or `resolve_cli_schema_uncached`. The entry holds exactly the
  problem string that outcome renders today: `cli_error_problem` for a `CliError`, the
  "payload is unusable" format, or `schema_load_problem` for a directory with no regular
  `schema.yaml`. `from_cli_cached` can then render it byte-identically without a spawn.
- `misses` is cleared at the start of `locate_schemas` and when `from_cli_cached` returns
  (Decision 11). Nothing else clears it.
- When `locate_schemas` learns a location for a name, it removes every `CliCache` per-change
  entry whose `schema` equals that name (Decision 10).
- `resolve_cli_schema(cli, repo, name, cache, locations: &mut SchemaLocations)` gains its
  last parameter. All 23 test call sites pass a fresh `&mut SchemaLocations::default()`,
  which reproduces today's per-call behaviour exactly.
- A location is usable when `<dir>/schema.yaml` is a regular file. That check is the only
  new filesystem read and is done with `Path::is_file`. A remembered location that is not
  usable is dropped by `locate_schemas` before it decides whether to ask.

## Persistence and Rollout

Migration: none. Backfill: none. Seeding: none. Cache invalidation: the location map lives
in worker memory only. A stale entry is evicted when its `schema.yaml` disappears, and a
vendored schema shadows it with no eviction. Index rebuild: none. Authorization: none.
Observability: none beyond the existing problem rows. Deployment: ships with the binary;
`make build`.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem (repository tree, schema directories) | real: `ScratchDir` trees | real: `ScratchDir` trees |
| `openspec` binary | replaced: a scratch shell script answering `list --json` and `schema which spec-driven --json`, logging its argv last, reached through `Config { openspec_bin: Some(script) }` | replaced: `FakeCli` in `changes::tests` and `refresh::tests`, recording invocations. An unregistered call panics (`src/cli.rs:637`), so the registration set is the allow-list |
| Herdr socket | replaced: a non-existent `herdr` path. Not exercised | not reached |
| Terminal | replaced: `TestBackend` at 120×20 and 60×20 | not reached: the rendering clauses call the pure `ui::detail::tab_bar` and `content_lines` at interior widths 78 and 58 |
| `git` | replaced: a non-existent `git` path. Not exercised | replaced: `FakeCli`'s `git` registrations in every `refresh::tests` worker test, as today. The member worker scenario registers one member that owns `alpha`. `changes::tests::overlay` calls `from_files_owned` directly and reaches no git |
| Process environment | replaced: `no_env` | not reached |
| Clock | real: the worker's `recheck` passed as a few milliseconds by `worker_for_test` | not reached |

## Test Strategy

This change **takes the outer-loop acceptance test**. The symptom is only observable once the
worker, the producers, and the render are composed. The test is
`a_package_schema_draws_no_not_vendored_row` in `ui::tests::wiring`, driven through
`run_wired_staged` with `Config { openspec_bin: Some(script), .. }`, as
`run_wired_sets_file_mode_from_the_probe` reaches its stand-in. The scratch repository
declares `schema: spec-driven` with one active change, `alpha`. The stand-in `openspec`
answers `list --json` with an empty change list, so `alpha` stays a file change and only the
locate pass can fix it. It answers `schema which spec-driven` with a scratch schema
directory. Each argv is logged as the script's last act.

There are two stages: `(a "list" line is in the log, 'j')` and `(always, 'q')`. In a cycle,
`list` is logged after `schema which` and is also logged at HEAD, so RED does not wait out
the deadline. Each stage settles for 300ms, which is the window for the merged result to be
adopted. The `j` moves the cursor from the section header onto `alpha`, so its detail is
drawn.

At 120×20 the test asserts:
- `completed` is true;
- the detail region shows the four tab ids;
- no row contains `│ ! `. Asserting on the `is not vendored` text would be vacuous, because
  a scratch path truncates it off-screen;
- the returned dashboard's `alpha` has empty `problems`. This is the discriminator;
- exactly one log line starts with `schema which`.

At 60×20 it asserts the `alpha` row and the empty `problems`. At HEAD it goes RED on the tab
ids, the `problems`, and the zero `schema which` count.

Unit tiers: `changes::tests` for the producer, the locations, and `locate_schemas`, decided
single-threaded. `refresh::tests` for the worker's spawn counts across cycles, through
`worker_for_test`.

Three new tests are **guards**: they pass before GREEN because they assert that something
does not happen. Each one is shown RED against one planted defect, `locate_schemas` with its
`NotVendored` filter removed, and then green with the plant reverted:
- `a_collapsed_archive_asks_nothing`;
- `a_vendored_schema_is_never_located`;
- `locate_schemas_learns_nothing_for_a_vendored_schema`.

Commands are focused `cargo test --lib <filter>`; `make check` runs everything.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| refresh-worker: One request produces the file result and then the merged one | existing `the_worker_answers_with_files_then_merged`, unchanged | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::the_worker_answers_with_files_then_merged` |
| refresh-worker: A worktree copy reaches the merged result first and the file result after | existing `a_worktree_copy_reaches_the_merged_result_first_and_the_file_result_after`, unchanged | unit (worker) | ScratchDir, FakeCli (openspec + git) | `cargo test --lib refresh::tests::a_worktree_copy_reaches` |
| refresh-worker: A CLI that fails still produces the file result | existing `a_failing_cli_still_sends_the_files_result` | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::a_failing_cli_still_sends` |
| refresh-worker: Queued requests are folded into one cycle | existing `drain_and_fold_unions_selections_and_takes_the_last_scope` and `changes::tests::from_cli_cached::selection_union_*` | unit | none | `cargo test --lib drain_and_fold` |
| refresh-worker: Dropping the refresher ends the worker | existing `dropping_the_refresher_disconnects_the_channel` | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::dropping_the_refresher` |
| refresh-worker: The worker writes nothing inside the repository | existing `the_worker_writes_nothing`, unchanged; FakeCli's registration set is the command allow-list | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::the_worker_writes_nothing` |
| refresh-worker: The scope on the request is the scope the file tier runs under | existing `the_scope_on_the_request_is_the_scope_the_file_tier_runs_under` | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::the_scope_on_the_request` |
| refresh-worker: `drain_and_fold` unions selections and takes the last scope | existing `drain_and_fold_unions_selections_and_takes_the_last_scope` | unit | none | `cargo test --lib drain_and_fold` |
| refresh-worker: A member's change of a located schema resolves in the merged result and on re-check | new `a_members_package_schema_change_resolves_in_merged_and_recheck` | unit (worker) | ScratchDir, FakeCli (openspec + git) | `cargo test --lib refresh::tests::a_members_package_schema` |
| refresh-worker: An active change of a package-bundled schema loses its problem row on the merged result | new `an_active_package_schema_change_loses_its_problem_on_merged` | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::an_active_package_schema` |
| refresh-worker: An active change of a package-bundled schema… (rendered) | new acceptance `a_package_schema_draws_no_not_vendored_row` | wiring | ScratchDir, script `openspec`, TestBackend | `cargo test --lib ui::tests::wiring::a_package_schema_draws` |
| refresh-worker: An archived change of a package-bundled schema gets its artifacts | new `an_archived_package_schema_change_gets_its_artifacts` | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::an_archived_package_schema` |
| refresh-worker: A remembered location serves the next cycle without a spawn | new `a_remembered_location_serves_the_next_cycle` | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::a_remembered_location` |
| refresh-worker: A collapsed archive asks nothing about archived schemas | new GUARD `a_collapsed_archive_asks_nothing` (green before GREEN; control: plant the NotVendored-filter removal) | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::a_collapsed_archive_asks_nothing` |
| refresh-worker: A location that vanished is dropped and asked for again | new `a_vanished_location_is_asked_for_again` | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::a_vanished_location` |
| refresh-worker: A failed lookup is asked once per cycle and named once | new `a_failed_lookup_is_asked_once_per_cycle` | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::a_failed_lookup` |
| refresh-worker: A change cached while its lookup failed is re-asked once the schema is located | new `a_cached_failure_is_re_asked_once_located` | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::a_cached_failure_is_re_asked` |
| refresh-worker: A vendored schema is never located (worker leg) | new GUARD `a_vendored_schema_is_never_located` (control: the same plant) | unit (worker) | ScratchDir, FakeCli | `cargo test --lib refresh::tests::a_vendored_schema_is_never_located` |
| refresh-worker: A vendored schema is never located (rebuild decision) | new GUARD `locate_schemas_learns_nothing_for_a_vendored_schema` (control: the same plant) | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::locate_schemas_learns_nothing` |
| schema-cli-fallback: A remembered location is loaded without a spawn | new `a_remembered_location_is_loaded_without_a_spawn` | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::a_remembered_location_is_loaded` |
| schema-cli-fallback: A successful lookup fills the cache for the next call | new `a_successful_lookup_fills_the_cache_for_the_next_call` | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::a_successful_lookup_fills` |
| schema-cli-fallback: An unusable directory is named and not remembered | new `an_unusable_directory_is_not_remembered` | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::an_unusable_directory` |
| schema-cli-fallback: A miss does not outlive the call that consumed it | new `a_miss_does_not_outlive_the_call` | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::a_miss_does_not_outlive` |
| schema-cli-fallback: A miss recorded earlier in the cycle is named without a second spawn | new `a_cycle_miss_is_named_without_a_second_spawn` | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::a_cycle_miss` |
| schema-cli-fallback: A schema vendored after it was located wins over the location | new `a_vendored_schema_wins_over_a_location` | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::a_vendored_schema_wins` |
| schema-cli-fallback: A parser problem from the CLI-named schema reaches every change using it | existing, unchanged | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::a_parser_problem_from_the_cli_named` |
| schema-cli-fallback: A parser problem read by both producers appears once after the merge | new `a_parser_problem_read_by_both_producers_appears_once` | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::a_parser_problem_read_by_both` |
| schema-cli-fallback: A schema absent from the repository is loaded from the directory the CLI names | existing, unchanged | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::a_schema_absent_from_the_repository` |
| schema-cli-fallback: A vendored schema never reaches the CLI | existing, unchanged | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::a_vendored_schema_never_reaches` |
| schema-cli-fallback: An unreadable vendored schema is not repaired by the CLI | existing, unchanged | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::an_unreadable_vendored_schema` |
| schema-cli-fallback: An invalid vendored schema is not repaired by the CLI | existing, unchanged | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::schema_fallback::an_invalid_vendored_schema` |
| schema-cli-fallback: An illegal schema name is not repaired by the CLI | existing, unchanged | unit | ScratchDir, FakeCli | `cargo test --lib changes::tests::an_illegal_schema_name_is_not_repaired` |
| change-artifacts: An active change of a located schema has its artifacts and no problem | new `a_located_schema_gives_an_active_change_its_artifacts` | unit | ScratchDir | `cargo test --lib changes::tests::a_located_schema_gives_an_active` |
| change-artifacts: An active change… (rendered at 120 and 60 columns, i.e. detail interior widths 78 and 58) | new `a_located_schema_renders_tabs_and_no_problem_row`, through `tab_bar` and `content_lines` at `[78, 58]` | view | ScratchDir | `cargo test --lib ui::detail::tests::a_located_schema_renders` |
| change-artifacts: An archived change of a located schema has its artifacts | new `a_located_schema_gives_an_archived_change_its_artifacts` | unit | ScratchDir | `cargo test --lib changes::tests::a_located_schema_gives_an_archived` |
| change-artifacts: Without locations the not-vendored problem is unchanged | new value-level `from_files_without_locations_still_reports_not_vendored`; the rendered clause stays `ui::detail::tests::an_unusable_schema_renders_no_artifacts` | unit + view | ScratchDir | `cargo test --lib changes::tests::from_files_without_locations` |
| change-artifacts: A location without a schema file names the location | new `a_location_without_a_schema_file_names_the_location` | unit | ScratchDir | `cargo test --lib changes::tests::a_location_without_a_schema_file` |
| change-artifacts: A vendored schema ignores the locations | new `a_vendored_schema_ignores_the_locations` | unit | ScratchDir | `cargo test --lib changes::tests::a_vendored_schema_ignores` |
| change-artifacts: A worktree member's change of a located schema resolves | new `a_members_located_schema_resolves` | unit | ScratchDir | `cargo test --lib changes::tests::overlay::a_members_located_schema` |
| change-merge: A file-side message survives beside a corrected artifact list | existing, unchanged | unit | none | `cargo test --lib changes::tests::merge::a_file_side_message_survives` |
| change-merge: Duplicate messages from both producers are collapsed | existing, unchanged | unit | none | `cargo test --lib changes::tests::merge::duplicate_messages_from_both` |
| change-merge: A join problem is appended after both producers' problems | existing, unchanged | unit | none | `cargo test --lib changes::tests::merge::a_join_problem_is_appended` |
| change-merge: Equal-length lists with equal ids take the CLI's paths, positionally | existing, unchanged | unit | none | `cargo test --lib changes::tests::join_artifacts::equal_length_lists` |
| change-merge: A duplicate id is joined by index rather than collapsed | existing, unchanged | unit | none | `cargo test --lib changes::tests::join_artifacts::a_duplicate_id` |
| change-merge: An empty file list takes the CLI's list, which is the repaired case | existing, unchanged | unit | none | `cargo test --lib changes::tests::join_artifacts::an_empty_file_list` |
| change-merge: An empty CLI list keeps the file's list | existing, unchanged | unit | none | `cargo test --lib changes::tests::join_artifacts::an_empty_cli_list` |
| change-merge: Differing lengths keep the file list and name both counts | existing, unchanged | unit | none | `cargo test --lib changes::tests::join_artifacts::differing_lengths` |
| change-merge: A differing id at one index keeps the file list and names the index | existing, unchanged | unit | none | `cargo test --lib changes::tests::join_artifacts::a_differing_id` |
| change-merge: Two empty lists join to an empty list | existing, unchanged | unit | none | `cargo test --lib changes::tests::join_artifacts::two_empty_lists` |

## Decisions

1. **Fix at the producer, not in the merge.** The file producer stops creating the message
   when a location exists.
   *Alternative:* `merge` drops a file-side "not vendored" string when the CLI's schema
   resolved. Rejected: it matches on diagnostic prose, it reverses `change-merge`'s
   never-drop rule, and it does nothing for archived changes.
2. **Remember a directory, not a parsed schema.** `schema.yaml` is re-read through
   `load_dir` on every build, which costs well under a millisecond.
   *Alternative:* caching the `Schema`. Rejected: an edit to a user-tier schema outside the
   repository would never be seen, because `watch` only watches `openspec/`.
3. **Locations live in `CliCache`.** It is already worker-lifetime, already threaded into
   `from_cli_cached`, and already exempt from the no-`Default` gate.
   *Alternative:* a separate worker-local struct. Rejected: `from_cli_cached` would need
   a second `&mut` parameter for the same purpose.
4. **The locate pass runs in step 2, after `Files` is sent.** This keeps the first answer at
   file speed.
   *Alternative:* locating before step 1. Rejected: it puts a node start-up (about 300ms)
   in front of the first frame on every pane open.
   *Alternative:* suppressing the message in `Files` whenever a binary exists. Rejected: it
   lies when the lookup then fails.
   The cost is a first-cycle flash of the row, stated in the spec.
5. **Rebuild the file set only when something was learned.** After the first cycle this is
   almost never, so steady-state cost is unchanged.
   *Alternative:* always rebuilding, which doubles file reads every cycle. Rejected.
   *Alternative:* patching only the affected changes in place. Rejected: it adds a fourth
   `Change` construction site for `change-model`'s gate to bind.
6. **Misses live for one cycle.** One spawn per failing name per cycle, shared by the locate
   pass and `from_cli_cached`.
   *Alternative:* a permanent negative cache. Rejected: a schema installed later is never
   seen until the pane restarts.
   *Alternative:* no record at all. Rejected: two spawns per cycle per failing name.
7. **The locate pass records no problem.** An active change's failure is still named by
   `from_cli_cached` from the miss record. An archived change of an unlocatable schema shows
   only the file producer's "not vendored" message, which is true. A second message for one
   fault would be noise.
8. **Names come from the base file set only.** The active list, plus the archived list when
   it was built. A schema used only by a worktree member's change, and by no base change, is
   not located (see Risks).
   *Alternative:* collecting names from the overlay too. Rejected: the overlay is computed
   after the merge, so collecting from it means a second rebuild path.
9. **`build_change` keeps seven parameters.** The locations travel inside the per-call
   `SchemaLoads` value that replaces the bare `HashMap` cache. Clippy's `too_many_arguments`
   fires at eight, and `make lint` runs with `-D warnings`.
   *Alternative:* an `#[allow]`. Rejected: `degraded-states` removed exactly such an
   allow from this function.
10. **Learning a location invalidates that schema's cached CLI entries.** Otherwise a change
    cached while its lookup failed keeps the failure problem on every cycle in which it is
    not selected, and that stale problem survives the merge.
    *Alternative:* re-asking every change on a learned location. Rejected: it spends one
    `instructions apply` per unrelated change.
11. **Misses are cleared when `from_cli_cached` returns, as well as when a locate pass
    starts.** Within a worker cycle, the locate pass's misses are still visible to the
    `from_cli_cached` call that follows it. Two bare `from_cli_cached` calls never share a
    miss, so `refresh-worker`'s "re-ask on `Selection::All`" contract and its test
    `a_rejected_schema_is_cached_like_any_other` hold unchanged.
    *Alternative:* clearing only at the locate pass. Rejected: that test's third call would
    stop re-asking.

## Risks / Trade-offs

- [The first cycle on a fresh pane flashes the `!` row for ~300ms] → Accepted and stated in
  `refresh-worker`. The second cycle onwards serves from the remembered location.
- [A `brew upgrade openspec` moves the Cellar path mid-session] → The usable check evicts
  the entry. That cycle's `Files` result names the old directory once, and its `Merged`
  result is correct. Covered by "A location that vanished is dropped and asked for again".
- [A schema name used only by a worktree member is never located] → Member changes almost
  always share the base's schema. Recorded as a known limitation; the row it leaves is the
  same true "not vendored" message as today.
- [A schema the CLI rejects costs one `schema which` spawn per cycle, where before it cost
  one per cycle only when that change was selected] → One extra node spawn on the worker
  thread per cycle, only for a repository whose schema the CLI does not know
  (`learning-tool`'s `outside-in-tdd`). The render path is unaffected.
- [Line ranges in `tests/degraded-coverage.toml` move, and `covers-check` passes any range
  that holds an executed statement, so a drifted range goes unnoticed] → A dedicated task
  saves each range's source text before group 1 and compares it after re-pointing.

## Migration Plan

None needed. Behaviour changes only inside the worker, there is no persisted state, and
rollback is reverting the commits.

## Open Questions

None. File-mode package-path probing is explicitly out of scope (proposal Non-Goals).
