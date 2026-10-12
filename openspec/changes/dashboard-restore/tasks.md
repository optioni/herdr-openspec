Ordering: sequential throughout — `openspec/config.yaml` → rules → tasks records the standing
parallelism veto (one crate, one compile, every gate sweeps the whole tree), so no group is
marked `parallel-after`. Groups 1–5 additionally all edit `src/open.rs` or `src/lib.rs`.

Planning-time evidence at HEAD `49e3fc7` (commands run 2026-10-12):

- `./target/debug/herdr-openspec restore; echo $?` → `error: unrecognized argument: restore`, `exit=2`.
- `cargo test -q --test cli restore` → `0 passed … 10 filtered out` (zero selected: the
  acceptance test does not exist yet).
- `cargo test -q --lib open::` → `53 passed`; `cargo test -q --test manifest` → `4 passed`.
- `grep -c '^\[\[startup\]\]' herdr-plugin.toml` → `0`.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [ ] 0.1 In `tests/cli.rs`, add `restore_replaces_a_stale_dashboard_and_leaves_a_live_one`: a `write_stub_herdr` script that logs every argv line to a file and answers `pane list` with a stale `w1:p2` (`"focused": true`) and a live `w1:p3`, `pane process-info` per design.md → Test Boundaries' M2/M5 shapes, and exit 0 for `plugin pane open`/`pane close`; run the binary as `restore` under `scrubbed()` with no workspace variables.
- [ ] 0.2 RED: assert exit 0, empty stderr, and the logged argv sequence `pane list`, `process-info w1:p2`, `plugin pane open … --target-pane w1:p2 … --focus`, `pane close w1:p2`, `process-info w1:p3`, and nothing else. Add `restore_exits_one_when_herdr_is_unreachable` (stub exits 1 on `pane list`).
- [ ] 0.3 Confirm `cargo test -q --test cli restore` selects 2 tests and both fail on `unrecognized argument: restore` (exit 2), not on stub setup.

## 1. The `restore` subcommand
<!-- kind: behavior -->

- [ ] 1.1 RED: in `src/lib.rs` tests, write `restore_alone_classifies_as_the_restore_invocation`, extend the flag-rejection test with `parse(&["restore", "--all"])`, and rewrite the usage test to `usage_names_all_four_commands` (whole-token command lines, summary `<ui|open|open-tab|restore>`); confirm they fail to compile or fail.
- [ ] 1.2 GREEN: add `Invocation::Restore`, its `parse` arms, and the `usage` line; add the `main` arm calling `open::restore_from_env()` through the existing `report_output` path, with no branch of its own (per design.md → Boundaries). Stub `restore_from_env` to return `Report { warnings: vec![], outcome: Ok(()) }` until group 4.
- [ ] 1.3 REFACTOR: none expected; if `run_open` and the restore arm duplicate the stderr/exit loop, share one reporter.
- [ ] 1.4 Run `cargo test -q --lib parse` and `cargo test -q --lib usage` — green, counts recorded; the group 0 tests now fail on the call sequence instead.

## 2. Stale-pane classification
<!-- kind: behavior -->

- [ ] 2.1 RED: in `src/open.rs` tests, write `a_restored_shell_is_stale`, `a_live_dashboard_is_not_stale`, `a_command_the_user_started_is_not_stale`, `an_unreadable_answer_is_never_stale` (table-driven over every shape the spec lists), and `an_unlabelled_pane_is_never_a_candidate`, using the JSON recorded in `notes/measurements.md` M2/M5.
- [ ] 2.2 GREEN: implement `is_stale(process_info: &str) -> bool` and `restore_candidates(listing: &str) -> Result<Vec<(String, bool)>, String>` (pane id, focused), both pure, per specs/dashboard-restore → "A stale dashboard pane is recognised from its process, not its label".
- [ ] 2.3 REFACTOR: if `restore_candidates` and `dashboard_panes` share the label/pane-id walk, extract it once without changing `dashboard_panes`' existing tests.
- [ ] 2.4 Run `cargo test -q --lib open::` — green, count recorded.

## 3. Replacing a stale pane in its slot
<!-- kind: behavior -->

- [ ] 3.1 RED: write `the_full_call_sequence_for_one_stale_pane`, `a_failed_open_closes_nothing`, and `a_failed_close_keeps_the_replacement_and_warns` against `cli::FakeCli`, asserting recorded argv exactly (including the absence of `--cwd`).
- [ ] 3.2 GREEN: implement `replace(cli, plugin_id, pane_id, focus) -> Result<Vec<String>, String>` (warnings on success, the open failure as `Err`), per specs/dashboard-restore → "A stale dashboard pane is replaced in its own slot" and design.md → Decision 4.
- [ ] 3.3 Run `cargo test -q --lib open::` — green, count recorded.

## 4. The session-wide sweep
<!-- kind: behavior -->

- [ ] 4.1 RED: write `two_stale_dashboards_in_two_workspaces_are_both_replaced`, `a_live_dashboard_is_left_alone`, `no_dashboard_pane_at_all`, `one_failed_replacement_does_not_stop_the_sweep`, and `herdr_unreachable_makes_no_further_call` against `FakeCli`.
- [ ] 4.2 GREEN: implement `restore(cli, plugin_id) -> Report` and replace the group 1 stub with `restore_from_env()`, which reads `HERDR_PLUGIN_ID` through `config::env_lookup()` with `open::context`'s fallback and requires no workspace (design.md → Decisions 6 and 8).
- [ ] 4.3 Run `cargo test -q --lib open::` and `cargo test -q --test cli restore` — unit green; the acceptance tests pass or fail only on a stated, remaining gap.

## 5. `open`/`open-tab` replace a stale match
<!-- kind: behavior -->

- [ ] 5.1 RED: update `a_matching_pane_is_focused…` to script a live `process-info` and expect three calls; add `a_stale_match_is_replaced_in_place_and_the_replacement_focused` and `an_unclassifiable_match_is_focused_never_replaced` (process-info failing; JSON without `foreground_processes`). Give the two-matches and open-tab/open-focus tests a live classification for the first match.
- [ ] 5.2 GREEN: in `open::run`, classify the first match before focusing; on stale, call `replace(…, focus = true)` and return its outcome; on not-stale or failure, focus as today, pushing a classification failure onto `warnings` (design.md → Decision 7).
- [ ] 5.3 Run `cargo test -q --lib open::` — green, count recorded; every existing `open::run` test still passes or was changed by 5.1 only.

## 6. Manifest
<!-- kind: operational -->

- [ ] 6.1 CHECK: in `tests/manifest.rs`, add `the_startup_hook_runs_the_restore_subcommand` (exactly one `[[startup]]`, its command's path equal to the panes', its argument parsing as `Invocation::Restore`) and add `startup` to the required keys; run `cargo test -q --test manifest` — RED, 1 failing (no `[[startup]]`).
- [ ] 6.2 CHANGE: add `[[startup]] command = ["./target/release/herdr-openspec", "restore"]` and set `min_herdr_version = "0.7.5"` in `herdr-plugin.toml`; update `SPEC.md` → Herdr integration → Manifest's transcription and its min-version paragraph to match.
- [ ] 6.3 CHECK: contract gate — re-read `SPEC.md` → Manifest against `herdr-plugin.toml` and run `cargo test -q --test doc_contract` (the manifest transcription claim) and `cargo test -q --test manifest`; both green, counts recorded.
- [ ] 6.4 VERIFY: `herdr plugin link --help` is not run against the user's session; the loud check for `min_herdr_version` is group 9's live load.

## 7. Degraded-states rows
<!-- kind: operational -->

- [ ] 7.1 CHECK: add the five rows design.md → Decision 10 names to `SPEC.md` → Degraded states, then run `cargo test -q --test degraded_coverage` — RED, naming each unbound row.
- [ ] 7.2 CHANGE: bind each row in `tests/degraded-coverage.toml` to the group 2–5 test that proves it.
- [ ] 7.3 VERIFY: `cargo test -q --test degraded_coverage` — green, count recorded.

## 8. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 8.1 VERIFY: `cargo test -q --test cli restore` — 2 selected, 2 passed.
- [ ] 8.2 REFACTOR: fold any stub-script duplication in `tests/cli.rs` into the existing helpers, or record that none was needed.

## 9. Live check
<!-- kind: operational -->

- [ ] 9.1 CHECK: `make build`, then in a throwaway session (`herdr --session restore-test` under tmux, `HERDR_SOCKET_PATH` pointed at it — never the default session) open one split and one tab dashboard, restart the session, and confirm both panes classify stale via `herdr pane process-info`.
- [ ] 9.2 CHANGE: run `HERDR_SOCKET_PATH=… ./target/release/herdr-openspec restore` once with the tmux client attached and once after detaching it; record in `notes/measurements.md` whether each replacement took its slot (`herdr pane layout`), which context the replacement received (`herdr pane get` cwd), and whether opening without an attached client worked.
- [ ] 9.3 VERIFY: run `restore` a second time and confirm it makes no open or close call (the replacements are live); then `herdr session delete restore-test`. Record that the hook's own invocation by Herdr is verified only after a GitHub install, per design.md → Risks.

## 10. Change Review
<!-- kind: operational -->

- [ ] 10.1 CHECK: dispatch the `outside-in-tdd-reviewer` agent with the change artifacts and `git diff 49e3fc7..HEAD` only.
- [ ] 10.2 CHANGE: fix every CRITICAL, resolve or accept each WARNING with a one-line reason, re-run affected tests.
- [ ] 10.3 VERIFY: confirm no blocking or unowned finding remains.

## 11. Documentation
<!-- kind: operational -->

- [ ] 11.1 Rewrite in `README.md`: the minimum Herdr version (0.7.0 → 0.7.5) and the subcommand list (audience: users installing the plugin) — and add two lines saying dashboards come back after a Herdr restart and that an idle shell labelled `OpenSpec` is treated as a dead dashboard.
- [ ] 11.2 Rewrite in `SPEC.md` → Herdr integration → Opening the pane: the existing-pane paragraph gains the stale-pane rule, and a short "Restoring after a restart" subsection replaces nothing (audience: future implementers; the restore contract has no other home).
- [ ] 11.3 Rewrite in `AGENTS.md` → Current repo state and Environment: name `dashboard-restore` among landed changes and change "Herdr 0.7.0 or later" to 0.7.5 (audience: every future session); add nothing else.

## 12. Lint & Verify
<!-- kind: operational -->

- [ ] 12.1 CHECK: the affected tiers are unit (`--lib`), contract (`--test manifest`, `--test doc_contract`, `--test degraded_coverage`), and acceptance (`--test cli`); `make check` runs all of them.
- [ ] 12.2 VERIFY: `make lint` — 0 warnings.
- [ ] 12.3 VERIFY: `make fmt-check` — clean.
- [ ] 12.4 VERIFY: `make gates` — every gate OK (no seam list moved, per design.md → Boundaries).
- [ ] 12.5 VERIFY: `make test` — green.
- [ ] 12.6 VERIFY: `make coverage` — total ≥ 80% and the production floor holds.
- [ ] 12.7 VERIFY: `make check` as the single gate, naming the failing sub-command if it fails; then `openspec validate dashboard-restore --strict`.
