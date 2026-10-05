Ordering: sequential. Groups 1 → 2 → 3 each need the previous group's code to exist:
- group 2's `resolve_cli_schema` and `locate_schemas` take the `SchemaLocations` that group 1
  adds;
- group 3's worker calls what group 2 adds.

So criterion 2 fires for every adjacent pair, and groups 1 and 2 also share `src/changes.rs`.
After group 0 the order runs inner-first for the same reason: the worker's GREEN cannot exist
before the producer's API. Group 4 re-points line ranges that groups 1–3 move, so it follows
them. Any remaining pair is ruled out by the standing parallelism veto in
`openspec/config.yaml` → rules → tasks (one crate, one compile).

The change directory is committed at planning time
(`docs(openspec): propose bundled-schema-resolution`), so `OPENSPEC-UNTOUCHED` is green from
group 0 on.

Planning-time selection counts come from
`cargo test --all-features --lib -- --list 2>/dev/null > "$T/list.txt"`, run at `e77dac3`
(1708 lines, from `wc -l < "$T/list.txt"`). `$T` is any scratch directory outside the
repository. The count check is:
`grep -o 'cargo test --lib [^ \`]*' openspec/changes/bundled-schema-resolution/design.md | awk '{print $4}' | grep -v '^<' | sort -u | while read f; do echo "$(grep -c -- "$f" "$T/list.txt") $f"; done`.
Its planning-time result is recorded in planning-review.md. An existing filter printing `0`,
or any filter printing `2` or more, is a failed check. Re-run it after each group's RED: that
group's new filters must then print `1`.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [x] 0.1 In `src/ui/mod.rs`'s `tests::wiring`, add a stand-in
      `openspec_script_schema_which(dir, log, root, schema_dir)`. It answers `list --json`
      with an empty change list rooted at `root`, and `schema which spec-driven --json` with
      `{"name":"spec-driven","source":"package","path":"<schema_dir>","shadows":[]}`. It
      logs its argv as its last act, per `openspec_script_failing`'s doc comment.
- [x] 0.2 RED: Write `a_package_schema_draws_no_not_vendored_row`, driven through
      `run_wired_staged` with `Config { openspec_bin: Some(script), .. }` and the two stages
      that design.md → Test Strategy names, at 120×20 and at 60×20. Make every assertion
      that section lists.
- [x] 0.3 Run `cargo test --all-features --lib ui::tests::wiring::a_package_schema_draws`,
      which must select 1 test. It must fail on the tab-id, `problems`, or `schema which`
      count assertion, with `completed == true`. A `completed == false` result is a harness
      fault, not a valid RED.
- [x] 0.4 Commit: `test(ui): pin a package-bundled schema drawing no not-vendored row`.

## 1. The file producer loads a located schema
<!-- kind: behavior -->

- [x] 1.1 RED: Write the change-artifacts tests named in design.md → Test Strategy:
      - `a_located_schema_gives_an_active_change_its_artifacts`;
      - `a_located_schema_gives_an_archived_change_its_artifacts`;
      - `from_files_without_locations_still_reports_not_vendored`;
      - `a_location_without_a_schema_file_names_the_location`;
      - `a_vendored_schema_ignores_the_locations`;
      - `changes::tests::overlay::a_members_located_schema_resolves`;
      - `ui::detail::tests::a_located_schema_renders_tabs_and_no_problem_row`, at `[78, 58]`.

      Expect a compile failure naming `SchemaLocations` and the new parameters. That
      failure is the missing interface.
- [x] 1.2 GREEN: Add `SchemaLocations` and `SchemaLoads`. Thread the locations through
      `load_schema_cached`, `build_change` (inside `SchemaLoads`, still seven parameters),
      `from_files_with_listing`, `from_files_owned`, and `overlay_family`, per design.md →
      Boundaries and Decision 9. `from_files` passes `&SchemaLocations::default()`.

      Update every existing caller to pass the default, using these planning-time counts:
      - `grep -c "from_files_owned(" src/changes.rs` → 15 (the definition, 1 production
        call, and 13 test calls);
      - the 4 `src/refresh.rs` sites
        (`grep -c "from_files_with_listing(\|overlay_family(" src/refresh.rs` → 4).

      Group 3 replaces the defaults in `src/refresh.rs`.
- [x] 1.3 REFACTOR: Keep `schema_load_problem` as the single renderer for a location's Outcome: no further refactor needed; the Ok/Err arms were folded into `parsed_load` and `failed_load` during GREEN, and `schema_load_problem` renders every failure.
      failure. Fold any duplicated `Ok`/`Err` arm in `load_schema_cached`, or record "no
      refactor needed".
- [x] 1.4 Run `cargo test --all-features --lib changes::` and
      `cargo test --all-features --lib ui::detail`. The 7 new tests and every existing test
      must pass, and `make lint` must exit 0.
- [x] 1.5 Commit: `feat(changes): load a not-vendored schema from a supplied location`.

## 2. The CLI tier shares and fills the locations
<!-- kind: behavior -->

- [x] 2.1 RED: In `changes::tests::schema_fallback`, write these tests, each counting
      invocations through the fake's record:
      - `a_remembered_location_is_loaded_without_a_spawn`;
      - `a_successful_lookup_fills_the_cache_for_the_next_call`;
      - `an_unusable_directory_is_not_remembered`;
      - `a_miss_does_not_outlive_the_call`;
      - `a_cycle_miss_is_named_without_a_second_spawn`;
      - `a_vendored_schema_wins_over_a_location`;
      - `a_parser_problem_read_by_both_producers_appears_once`.

      Also write the GUARD `locate_schemas_learns_nothing_for_a_vendored_schema`. Expect
      compile failures naming `CliCache::locations` and `locate_schemas`.
- [x] 2.2 GREEN: Add `CliCache`'s `locations` field and accessor.
      `resolve_cli_schema`/`resolve_cli_schema_uncached` take `&mut SchemaLocations` and
      follow design.md → Contracts: usable dirs first, then misses, then a spawn, with a miss
      recorded for every non-usable outcome. `from_cli_cached` passes `&mut cache.locations`
      and clears `misses` before it returns. The 23 test call sites
      (`grep -c "resolve_cli_schema(&fake" src/changes.rs` → 23) pass a fresh
      `&mut SchemaLocations::default()`.
- [x] 2.3 GREEN: Add `pub(crate) fn locate_schemas(cli, repo, files: &ChangeSet, cache: &mut CliCache) -> usize`,
      per design.md → Contracts and Decisions 6–8 and 10. It:
      - clears `misses`;
      - collects names from the built `active` and `archived` changes;
      - keeps the names whose `schema::load` returns `NotVendored`;
      - evicts unusable locations;
      - asks once for each remaining name;
      - drops the `CliCache` entries of every newly located name;
      - returns the count learned.
- [x] 2.4 CHECK: `changes::tests::from_cli_cached::a_rejected_schema_is_cached_like_any_other`
      passes unmodified (Decision 11). With `X='/fn a_rejected_schema_is_cached_like_any_other/,/^        }$/p'`,
      `diff <(git show e77dac3:src/changes.rs | sed -n "$X") <(sed -n "$X" src/changes.rs)`
      must exit 0 with no output. At planning time it exited 0 and selected 114 lines.
- [x] 2.5 CHECK: `make gates` exits 0, including `NOSPAWN-GREP`. Every new spawn goes
      through `OpenspecCli`.
- [x] 2.6 Run `cargo test --all-features --lib changes::`. The 8 new tests and every existing
      `schema_fallback`, `from_cli`, `from_cli_cached`, `merge`, and `join_artifacts` test must
      pass.
- [x] 2.7 Commit: `feat(changes): remember schema which locations in CliCache`.

## 3. The worker locates before it merges
<!-- kind: behavior -->

- [x] 3.1 RED: In `refresh::tests`, write these tests through `worker_for_test` with
      `recv_timeout(10s)` waits:
      - `an_active_package_schema_change_loses_its_problem_on_merged`;
      - `an_archived_package_schema_change_gets_its_artifacts`;
      - `a_remembered_location_serves_the_next_cycle`;
      - `a_vanished_location_is_asked_for_again`;
      - `a_failed_lookup_is_asked_once_per_cycle`;
      - `a_cached_failure_is_re_asked_once_located`;
      - `a_members_package_schema_change_resolves_in_merged_and_recheck`.

      Each must fail on a spawn count, an `is not vendored` assertion, or an artifact
      assertion.

      Also write the GUARDS `a_collapsed_archive_asks_nothing` and
      `a_vendored_schema_is_never_located`. These pass now.
  Note: `a_failed_lookup_is_asked_once_per_cycle` already passed pre-GREEN (a regression guard against double-asking); `a_collapsed_archive_asks_nothing` failed only on its post-`Full` leg.
- [x] 3.2 GREEN: In `worker_body`, at all four sites design.md → Boundaries names:
      - pass `cache.locations()`;
      - keep a clone of step 1's `list_changes` results;
      - in step 2, call `locate_schemas`, and when it returns `> 0`, rebuild `files` from
        that clone before `from_cli_cached`.
- [x] 3.3 CHECK (guard controls): Delete the `NotVendored` filter in `locate_schemas`. Then
      `cargo test --all-features --lib -- a_collapsed_archive_asks_nothing a_vendored_schema_is_never_located locate_schemas_learns_nothing`
      must select 3 tests and fail all 3. Restore the filter, and the same command must
      pass 3.
  Result: with the filter deleted the 3 selected tests all failed; restored, all 3 pass.
- [x] 3.4 CHECK: `make gates` exits 0, with `NOBLOCK` and `READONLY-UI` included.
      `grep -c "thread::spawn" src/refresh.rs` still prints 5, its planning-time value.
- [x] 3.5 REFACTOR: If step 1 and the step-2 rebuild repeat the build lines, extract one
      private helper in `src/refresh.rs`. Otherwise record "no refactor needed".
  Outcome: no refactor needed (the step-1 and rebuild calls differ in ownership: clones vs moves).
- [x] 3.6 Run `cargo test --all-features --lib refresh::`. The 9 new tests and the 8 carried
      worker scenarios must pass.
- [x] 3.7 Commit: `feat(refresh): locate not-vendored schemas before merging`.

## 4. Re-point the coverage map
<!-- kind: operational -->

- [x] 4.1 CHECK: For every `covers` entry in `tests/degraded-coverage.toml` naming
      `src/changes.rs` or `src/refresh.rs`, write the range's text at the planning commit to
      `$T/covers-before.txt`, using `git show e77dac3:<file> | sed -n '<a>,<b>p'`.
      At planning time, `grep -o '"src/\(changes\|refresh\).rs:[0-9-]*"' tests/degraded-coverage.toml | wc -l`
      → 24. Then run `make covers-check` and record its result.
- [x] 4.2 CHANGE: Re-point every moved range so that its text at HEAD holds the same
      statements as `$T/covers-before.txt`. Only a range whose code this change rewrote may
      differ, and each such range must cover its replacement.
      Done: 13 ranges moved unchanged; 9 unchanged in place. Two were rewritten:
      - `changes.rs:1497-1541` ("Schema unknown to the CLI") became
        `ask_schema_location` + `resolve_cli_schema_uncached`;
      - `changes.rs:1991-2007` became `2179-2195`, differing only in
        `schema_cache` → `schema_loads`.
      Also re-pointed `tests/degraded_coverage.rs`'s hard-coded `is_attribute` fixture
      (`src/changes.rs:2345-2346` → `2534-2535`, the same `#[test]` + `#[should_panic]`
      pair), as `homebrew-probe` did in `0e2d056`.
- [x] 4.3 VERIFY: `make covers-check` exits 0. A side-by-side diff of before and after range
      texts shows only the rewritten ranges changed.
- [x] 4.4 Commit: `test(covers): re-point line ranges moved by schema location sharing`.

## 5. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [x] 5.1 VERIFY: `cargo test --all-features --lib ui::tests::wiring::a_package_schema_draws`
      passes with 1 test selected and `completed == true`.
- [x] 5.2 VERIFY: Run `make build`, then re-run the tmux repro against
      `~/Code/dungeons-and-dragons`. After the first refresh, neither the active
      `ssh-tui-server` nor an expanded archived change may show a `! … is not vendored`
      row, and the archived change lists `proposal specs design tasks`.
      Done at e45b4d3: `ssh-tui-server` and archived `structured-player-actions` both draw
      `proposal specs design tasks` with no `! ` row after the first refresh.
- [x] 5.3 REFACTOR: Remove any harness duplication that 0.1 introduced, or record "none".
      None: `openspec_script_schema_which` shares only the `list --json` payload literal with
      `openspec_script`, which each stand-in states for its own semantics.
- [x] 5.4 Commit if anything changed: `test(ui): tidy the package-schema acceptance harness`.
      Nothing changed; no commit.

## 6. Documentation
<!-- kind: operational -->

- [x] 6.1 CHECK: Two greps must each print 1 match, as they did at planning time:
      - ``grep -c 'ask `openspec schema which' SPEC.md``, for the Schema paragraph's fallback
        sentence at line 169;
      - `grep -c "^| Schema not vendored" SPEC.md`, for the degraded row.
- [x] 6.2 Rewrite `SPEC.md` → Schema paragraph (audience: implementers). Replace the "When
      the first tier misses, ask `openspec schema which`" sentence with the shared-location
      rule: the worker remembers the location, both producers load from it, and file mode
      has no third tier. Net change: about +2 lines.
- [x] 6.3 Rewrite the **Behaviour** cell of `SPEC.md` → Degraded states → "Schema not
      vendored" only. With a binary, the row and the empty tab bar show until the worker's
      first locate pass, which is one cycle. In file mode they show permanently. Leave the
      **Condition** cell byte-identical.
- [x] 6.4 Add a row to `openspec/IMPLEMENTATION-ORDER.md` for `bundled-schema-resolution`,
      marked as unplanned post-roadmap work, in the existing rows' format, depending on
      `homebrew-probe`.
- [x] 6.5 VERIFY: `cargo test --all-features --test degraded_coverage --test doc_contract`
      passes.
- [x] 6.6 Commit: `docs: record shared schema locations and the narrowed not-vendored row`.

## 7. Change Review
<!-- kind: operational -->

- [x] 7.1 CHECK: Dispatch `outside-in-tdd-reviewer` with only this change's artifacts and
      `git diff e77dac3..HEAD`. Point it first at:
      - the spawn counts per cycle;
      - the invalidation of cached entries on a learned location;
      - the first-cycle flash being the only remaining "not vendored" row;
      - file mode being unchanged.
- [x] 7.2 CHANGE: Fix every CRITICAL. Resolve or accept each WARNING, with a one-line reason
      recorded here. Re-run the affected tests.
      No CRITICAL. Dispositions:
      - W1 (idle re-check leg untested; `cache.locations()` there revertible with all tests
        green): fixed in fc70898. The member's second `status` answer makes the re-check send;
        the default-locations plant now fails the test.
      - W2 (docs overclaimed "only a first-cycle flash"): fixed in 3930b4c. SPEC's row and the
        roadmap row name the first pass that *sees* the name, the member-only limitation
        (Decision 8), and a failing lookup.
      - S1 (an evicted location whose re-ask fails triggers no rebuild, so that cycle's
        `Merged` names the old directory): accepted. It is transient (the next step 1 has no
        such dir and names the repository path), needs a broken CLI, and fixing it would
        change `locate_schemas`' return contract.
      - S2 (`Found`/`Row` aliases): accepted as a false-positive workaround. The comment now
        forbids literals through them (fc70898).
      - S3 (the binary-present half of the "Schema not vendored" row was unbound): fixed in
        c8cac0b. Second proof `a_package_schema_draws_no_not_vendored_row`, covers over
        `load_schema_cached`'s location arm.
      - S4 (`is_file` pre-check hid `load_dir`'s own error): fixed in c73c120/cedd5de. Every
        covers range re-verified byte-identical after the -8 shift.
      - S5 (3.1 bookkeeping): `a_failed_lookup_is_asked_once_per_cycle` went green before
        GREEN because no locate pass existed. The "CLI tier ignores `misses`" plant falsifies
        it. `a_collapsed_archive_asks_nothing`'s `Full` leg was a genuine RED.
      - OCR (step 1 clones the listing every cycle): accepted, as 3.5 recorded. The clone
        buys the "no second walk" guarantee.
- [x] 7.3 VERIFY: No blocking or unowned finding remains.
- [x] 7.4 Commit: `chore(bundled-schema-resolution): record the Change Review`.

## 8. Lint & Verify
<!-- kind: operational -->

- [x] 8.1 CHECK: The affected tiers are:
      - unit (`changes`, `refresh`);
      - view (`ui::detail`);
      - wiring (`ui::tests::wiring`);
      - contract (`degraded_coverage`, `doc_contract`).

      `make test` runs all of them.
- [x] 8.2 VERIFY: `make fmt-check` is clean.
- [x] 8.3 VERIFY: `make lint` reports 0 warnings. Clippy with `-D warnings` is this crate's
      type and lint gate.
- [x] 8.4 VERIFY: `make gates` exits 0.
- [x] 8.5 VERIFY: `make covers-check` exits 0.
- [x] 8.6 VERIFY: `make test` is green.
- [x] 8.7 VERIFY: `make coverage` holds both the 80% total and the production-slice floor.
- [x] 8.8 VERIFY: `make check` is green as the single gate. If it fails, name the failing
      sub-command here.
      Green at 33f940d: lib 1733/1733, contract tier 124/124, line coverage 95.13% total,
      production slice 96.32% (floor 96%).
- [x] 8.9 VERIFY: `openspec validate bundled-schema-resolution --strict` reports the change
      as valid.
