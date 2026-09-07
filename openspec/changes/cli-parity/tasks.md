<!-- Group ordering: groups 1, 2 and 8 edit `src/schema.rs`; groups 1, 2, 3, 4, 5, 6, 7 and 8
     all edit `src/changes.rs`; groups 6, 7 and 10 all edit `SPEC.md`'s degraded-states table
     and `tests/degraded-coverage.toml`, and 6 and 7 each assert that table's row count.
     Every pair shares a file, a dependency, or both, so no group carries a
     `parallel-after` marker. The sequence is real, not narrative. -->

<!-- No `## 0. Acceptance Test — Outer Loop` group: this change adds no rendered element,
     route, or key, and every scenario is assertable one layer below the view — see
     design.md → Test Strategy. -->

<!-- `cargo test <filter>` exits 0 when it matches nothing, so every VERIFY step below must
     show a non-zero `N passed`. Module paths are `changes::tests::<sub>` and
     `schema::tests::`; the glob and progress tests sit directly in `changes::tests`, and the
     CLI-fallback tests are in `changes::tests::schema_fallback`. Multiple filters go after
     `--` (`cargo test -- a b`), not as separate arguments. -->

## 1. A wrong-typed `apply.tracks` selects no tasks artifact
<!-- kind: behavior -->

- [x] 1.1 RED: Rewrite `src/schema.rs`'s `a_tracks_value_of_the_wrong_type_falls_back_to_the_id` as `a_tracks_value_of_the_wrong_type_yields_no_tasks_artifact`, driving a sequence, the integer `42`, and a mapping against a fixture declaring `{id: tasks, generates: tasks.md}`, and asserting no tasks artifact plus exactly one problem naming the key. Confirm it fails on the assertion, not on fixture setup.
  - HEAD evidence: `grep -n 'apply.tracks is not a string' src/schema.rs` → `314:` (exit 0), the `id_fallback` call the new test must stop reaching.
- [x] 1.2 RED: Add `a_wrong_typed_tracks_counts_tasks_md_the_same_pair_the_cli_reports` in `src/changes.rs`'s tests — a scratch change with `apply.tracks: 42`, artifact `{id: tasks, generates: tasks/**/*.md}`, `tasks/a.md` at 2/2, `tasks/b.md` at 0/2, no `tasks.md` — asserting `Progress { completed: 0, total: 0 }` and that no `ArtifactRef` carries `tracks_tasks`.
- [x] 1.3 GREEN: In `tasks_artifact`, replace the `None => id_fallback(...)` arm with one that records the wrong-typed problem and returns no artifact, leaving the null/absent/non-mapping branches on `id_fallback`.
- [x] 1.4 REFACTOR: Collapse `id_fallback`'s now-unused `extra` parameter if no caller still passes `Some`, or state that a caller still does and it stays.
- [x] 1.5 Run the group tests — `cargo test -- schema::tests:: changes::tests::a_wrong_typed_tracks` — no regressions in the six unchanged `tracks` scenarios.

## 2. `schema::load` rejects a name it cannot legally join
<!-- kind: behavior -->

- [x] 2.1 RED: Add three `schema::load` tests — a traversing name (`../../../../etc`) asserting the returned variant is the illegal-name one and **not** `NotVendored`, with a `testutil::snapshot` of the scratch tree before and after; the eight rejected shapes plus the accepted `spec-driven.v2`; and a load of this repository's vendored `tdd` asserting its artifacts in file order.
  - HEAD evidence: `grep -rn 'fn .*illegal_name' src/ tests/` → 0 matches (exit 1); `sed -n '428,430p' src/schema.rs` shows `load` joining `name` with no guard.
- [x] 2.2 GREEN: Add the `LoadError` variant for an illegal name (design.md → D3) and the `is_legal_name` guard at the top of `load`, before any join. Extend `load_error_problem` to render it.
- [x] 2.3 GREEN: Update the two compiler-enforced `LoadError` matches — `schema::load_error_problem` (`src/schema.rs:436`) and `changes::schema_load_problem` (`src/changes.rs:937`). These are the only two `cargo check` will flag; `resolve_cli_schema_uncached`'s catch-all is group 3's job.
- [x] 2.4 Run the group tests — `cargo test schema::tests::` — and confirm `cargo build` reports no non-exhaustive match. No refactor was needed: the guard is one early return and the variant is additive.

## 3. The CLI fallback tier's illegal-name branch is made explicit
<!-- kind: refactor -->

- [x] 3.1 CHARACTERIZE: Add `an_illegal_schema_name_is_not_repaired_by_the_cli` — `resolve_cli_schema` called directly with `../../../../etc`, an `OpenspecCli` fake holding no registration, asserting no schema, exactly one problem naming the value, and an empty recorded-invocation list. It is green as soon as group 2 lands, because the catch-all at `src/changes.rs:1375` already produces this; record that it passes before the refactor.
- [x] 3.2 REFACTOR: Replace `resolve_cli_schema_uncached`'s catch-all `Err(err) =>` arm with explicit arms for `Unreadable`, `Invalid`, and the illegal-name variant, so a future `LoadError` variant cannot silently inherit this branch and the compiler flags it instead.
- [x] 3.3 VERIFY: Run the unchanged characterization tests — `cargo test changes::tests::schema_fallback` and `cargo test changes::tests::an_illegal_schema_name` — no regressions across the five existing fallback scenarios.

## 4. `parse_apply` refuses an illegal `schemaName`
<!-- kind: behavior -->

- [x] 4.1 RED: Add `from_cli` tests for a payload carrying `"schemaName": "../../../../etc"` among two good changes, and a table-driven test over `"   "`, `"."`, `".."`, `"a/b"`, `"a\\b"`, an absolute path, the accepted `spec-driven` and `spec-driven.v2`, the trimmed `" tdd "`, and `""` through the existing missing-field branch. Assert the change drops, one problem names it and the value, no `schema which` invocation is recorded, and the string reaches no `Change::schema`.
  - HEAD evidence: `grep -c 'is_legal_name' src/changes.rs` → `0` (exit 1) — no guard exists on this path.
- [x] 4.2 GREEN: Apply `schema::is_legal_name` in `parse_apply`'s `schemaName` read and store the trimmed value, returning the existing per-change parse failure with a message naming the rejected value.
- [x] 4.3 Run the group tests — `cargo test changes::tests::from_cli` — no regressions in the four existing per-change failure scenarios. No refactor was needed: the guard replaces one predicate inside an existing helper.

## 5. Every diagnostic the seam carried reaches the problem row
<!-- kind: behavior -->

- [ ] 5.1 RED: Add `two_different_spawn_failures_produce_two_different_problems`, running `from_cli` twice with `NotStarted` reasons `No such file or directory (os error 2)` and `Exec format error (os error 8)`, asserting the two problem strings differ. Widen the existing absent-binary test to assert the reason text.
  - HEAD evidence: `grep -n 'reason: _' src/changes.rs` → `1308:` (exit 0) — the binding that discards it.
- [ ] 5.2 RED: Add four `Failed`-arm tests — `an_exec_failure_of_the_openspec_shim_reports_its_own_stderr` (`code: Some(127)`, `stderr: "env: node: No such file or directory\n"`, asserting the code, the vector and that text with no trailing newline), `a_multi_line_stderr_contributes_only_its_first_non_blank_line_trimmed` (**exact whole-string equality**, the only place the trim is observable), `a_note_banner_is_skipped_and_the_next_line_carried` (three runs: banner-then-diagnosis, banner alone, padded banner), and `a_whitespace_only_stderr_appends_nothing` (byte-identical to the empty-stderr problem).
  - HEAD evidence: `grep -n 'stderr: _' src/changes.rs` → `1316:`, the `Failed` arm binding that discards it. `cli_error_problem` runs `src/changes.rs:1302-1324`; its `Failed` arm is `:1312-1322`.
  - Measured shapes, both reproduced on this machine: `env -i PATH=/usr/bin:/bin "$(readlink -f ~/.nvm/versions/node/v24.18.0/bin/openspec)" list --json` → `exit=127`, `stdout=[]`, `stderr=[env: node: No such file or directory]`; and `openspec schema which nosuchschema --json` from `/tmp` → `exit=1`, the real answer on **stdout**, `stderr=[Note: Schema commands are experimental and may change.]`.
- [ ] 5.3 RED: Add `an_exec_failure_during_the_fallback_tier_carries_its_stderr` — the same `Failed` shape answered for `["schema", "which", …]`, plus an empty-stderr control and a banner-only run that must be byte-identical to it. `cli_error_problem` has three call sites (`src/changes.rs:1371`, `:1529`, `:1600`), so the fallback tier must be shown to inherit the rule rather than assumed to. Also widen the existing `NotStarted` fallback assertion to expect the carried reason.
- [ ] 5.4 GREEN: In `cli_error_problem`, bind `reason` in the `NotStarted` arm and append it; bind `stderr` in the `Failed` arm and append the first non-blank line whose trimmed form does not begin `Note: `, trimmed, when one exists. Rewrite the function's doc comment, which currently states the rationale this change disproves.
- [ ] 5.5 CHANGE: Correct `a_schema_the_cli_rejects_removes_one_change_and_keeps_the_others` (`src/changes.rs:5256`), whose fake supplies `stderr: "Unknown schema \"outside-in-tdd\""` at `:5294` and asserts at `:5303` that the problem does **not** contain it — the assertion this change inverts. Set that fake's `stderr` to `""`, which is what `instructions apply` was measured producing, and keep the `:5303` assertion as the proof that no reason is invented.
- [ ] 5.6 Run the group tests — `cargo test -- changes::tests:: schema::tests::` — and update any remaining assertion that pinned the old `NotStarted` string. Every other `Failed` assertion using `stderr: ""` must stay byte-identical; if one moves, the whitespace-only test is wrong. No refactor was needed: `cli_error_problem` gains two bindings and one line-selecting helper and stays a single `match`.

## 6. The symlink divergence becomes a proven degraded-states row
<!-- kind: operational -->

- [ ] 6.1 CHECK: Add `a_non_looping_directory_symlink_resolving_inside_the_change_is_skipped_too` (link to `../inner`, target inside the change directory), `a_symlink_resolving_outside_the_change_is_skipped_without_failing_closed`, and `a_glob_shaped_tasks_artifact_behind_a_symlink_is_the_known_limit` (asserting `1/2`, with the CLI's measured `1/5` as a documented constant citing `dist/core/artifact-graph/outputs.js:93`). Confirm all three pass.
  - HEAD evidence: `grep -rn 'fn .*symlink' src/ tests/` → 3 matches, none of these three.
- [ ] 6.2 CHECK: Run the negative control — swap `collect_glob_matches`' `entry.file_type()` for `std::fs::metadata`, confirm the two inside-resolving tests fail (record which, and that the pre-existing `a_directory_symbolic_link_is_not_descended_into` also fires), revert, confirm green, and confirm `git diff --quiet src/changes.rs`.
- [ ] 6.3 CHANGE: Add the `SPEC.md` degraded-states row for a `generates` glob crossing a symlinked directory **whose target resolves inside the change directory**, stating the artifact-list scope, the `followSymbolicLinks: true` reference, and that the CLI tier corrects it. Say in the row that an outside-resolving link is a different case where the CLI fails closed.
- [ ] 6.4 CHANGE: Add the matching `[[row]]` to `tests/degraded-coverage.toml` with all five keys — `condition` byte-identical to the new table cell, `tier = "unit"`, `proof = ["a_non_looping_directory_symlink_resolving_inside_the_change_is_skipped_too"]`, `verdict = "confirmed"`, and a `why` naming what the proof does and does not show (it proves the plugin skips, not that the CLI follows).
- [ ] 6.5 CHANGE: Raise `MIN_ROWS` in `tests/degraded_coverage.rs:17` from 44 to 45, keeping the floor at its true measured value.
- [ ] 6.6 VERIFY: `cargo test --test degraded_coverage` — green, with a non-zero pass count.

## 7. The archive race becomes a proven degraded-states row
<!-- kind: operational -->

- [ ] 7.1 CHECK: Add `changes_are_paired_by_name_not_by_position` (two lists in opposite order, each CLI entry carrying a self-naming schema) and `a_change_archived_mid_cycle_appears_in_both_lists_for_one_cycle` (two merges, the second with an empty CLI list). Confirm both pass.
  - HEAD evidence: `grep -rn 'fn .*paired_by_name' src/ tests/` → 0 matches (exit 1); `grep -rn 'fn .*archived_mid_cycle' src/ tests/` → 0 matches (exit 1).
- [ ] 7.2 CHECK: Run two negative controls, one per test — pair `merge` by `zip` index and confirm the by-name test fails; then make `merge` drop a CLI change whose name is in `files.archived` and confirm the archive-race test fails. Revert each, confirm green, and confirm `git diff --quiet src/changes.rs`.
- [ ] 7.3 CHANGE: Add the `SPEC.md` degraded-states row for a change archived between the worker's `list --json` call and its file walk, stated at the merged-`ChangeSet` level (the name is in both `active` and `archived`), naming the one-cycle lifetime and the reason the merge does not defend against it (design.md → D6).
- [ ] 7.4 CHANGE: Add the matching `[[row]]` to `tests/degraded-coverage.toml` with all five keys, `tier = "unit"`, `proof = ["a_change_archived_mid_cycle_appears_in_both_lists_for_one_cycle"]`, and `verdict = "confirmed"`.
- [ ] 7.5 CHANGE: Raise `MIN_ROWS` in `tests/degraded_coverage.rs` from 45 to 46.
- [ ] 7.6 VERIFY: `cargo test --test degraded_coverage` — green, with a non-zero pass count.

## 8. Dead branch and misattached doc block
<!-- kind: refactor -->

- [ ] 8.1 CHARACTERIZE: Confirm "Two empty lists join to an empty list" and "A declaration of the wrong type is not a declaration" are green and untouched — `cargo test -- changes::tests::join_artifacts schema::tests::`.
- [ ] 8.2 REFACTOR: Delete `join_artifacts`' `if file.is_empty() && cli.is_empty()` branch (`src/changes.rs:1263`), which the next branch subsumes. `change-merge`'s delta already renumbers the published rule list from six rules to five to match.
- [ ] 8.3 REFACTOR: Move the `Ok(Some(name))`/`Ok(None)`/`Err(reason)` and `!is_badvalue()` doc block (`src/schema.rs:44-59`) from `first_document` onto `schema_key` (`:70`), leaving `first_document` the two-line doc at `:60-64` that is already its own.
- [ ] 8.4 VERIFY: Run the unchanged characterization tests — no regressions — and confirm `cargo clippy --all-targets --all-features -- -D warnings` reports nothing new.

## 9. Change Review
<!-- kind: operational -->

- [ ] 9.1 CHECK: Dispatch an independent reviewer (not a fork of this session) against proposal.md, all five delta specs, design.md, and the diff. Concentration points: that each of the 59 scenarios names a test that would go red if its behavior were deleted; that no test reaches a real `openspec` binary or a Herdr socket; that every `cargo test` filter used reports a non-zero pass count; and that the two documented divergences are bound to proofs whose negative controls were actually run.
- [ ] 9.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run the affected tests.
- [ ] 9.3 VERIFY: Confirm no blocking or unowned finding remains.

## 10. Documentation
<!-- kind: operational -->

- [ ] 10.1 Rewrite in `AGENTS.md`: "Current repo state" (audience: every future session) — replace "joined by position" (`AGENTS.md:56`) with "paired by name, with artifacts within a change joined by position". `CLAUDE.md` is a symlink to it, so one edit serves both.
- [ ] 10.2 Rewrite in `SPEC.md`: "Resolution chain" (audience: every future change) — state that a `tracks` value which is *present* and matches nothing yields no tasks artifact, so the wrong-typed case falls under the same clause. A few words, not a new paragraph.
- [ ] 10.3 Rewrite in `SPEC.md`: "Degraded states" (audience: every future change) — widen the "Schema loads with no tasks artifact" row, whose condition cell says `apply.tracks` matches nothing **and no artifact has id `tasks`**, since after group 1 a wrong-typed `tracks` reaches that state with an id-`tasks` artifact present. Update the same row's `condition` in `tests/degraded-coverage.toml`, which keys on that cell verbatim.
- [ ] 10.4 Rewrite in `SPEC.md`: "Degraded states" (audience: every future change) — amend the invalid-UTF-8 row's "the one case" claim, now that group 6 adds a second knowing file-vs-CLI divergence. Edit the **Behaviour** cell only: the condition cell is the coverage map's key and must stay byte-identical.
- [ ] 10.5 Rewrite in `SPEC.md`: "Degraded states" (audience: every future change) — the row "An `openspec` command exits non-zero" (`SPEC.md:760`) says "The reason is unavailable to the plugin". Measured, that is false for an exec failure: stdout was empty and stderr carried the whole answer. Reword the **Behaviour** cell to "openspec's own diagnostic goes to stdout and is unavailable; a stderr line, when present, is carried" — condition cell byte-identical, since it is the coverage map's key.
- [ ] 10.6 Update that row's entry in `tests/degraded-coverage.toml` (lines 237-242): its `proof` is `["a_failed_cli_cycle_keeps_the_file_numbers"]` with `verdict = "unproven"`, and that test shows the file numbers survive, not that a stderr line is carried. Add `an_exec_failure_of_the_openspec_shim_reports_its_own_stderr` to `proof`, set `verdict = "confirmed"`, move `tier` from `"outer"` to `"unit"`, and write a `why` naming both halves and recording that the outer proof is the stronger one. The tier move is forced, not preferred: `tests/degraded_coverage.rs` applies its render check **per proof entry**, so an `outer` row whose second proof renders nothing fails the gate (design.md → D8). Change only this row's five keys — the checker is the sibling `gate-integrity` change's to fix.

## 11. Lint & Verify
<!-- kind: operational -->

- [ ] 11.1 CHECK: Confirm the affected tiers are the unit tier plus `tests/degraded_coverage.rs`, that no view test or manifest test needed changing, and that no manifest, config format, or keybinding changed. `make check` is the single gate; name the failing sub-command if it fails.
- [ ] 11.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 11.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 11.4 VERIFY: `make gates` — every hygiene gate green, each script run bare with no `MIN`/`SCAN_MIN` override except `NODEFAULT-UI`'s five.
- [ ] 11.5 VERIFY: `cargo test --all-features` — green. It was green at HEAD before this change (exit code 0).
- [ ] 11.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — the coverage floor holds; add tests rather than lowering it.
- [ ] 11.7 VERIFY: `openspec validate cli-parity --strict` — valid.
