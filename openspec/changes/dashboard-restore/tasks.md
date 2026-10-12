Ordering: sequential throughout — `openspec/config.yaml` → rules → tasks records the standing
parallelism veto (one crate, one compile, every gate sweeps the whole tree), so no group is
marked `parallel-after`. Groups 1–5 additionally share `src/open.rs` or `src/lib.rs`.

Planning-time evidence at HEAD `e678de5` (code identical to `49e3fc7`), run 2026-10-12:

- `./target/debug/herdr-openspec restore; echo $?` → `error: unrecognized argument: restore`, `exit=2`.
- `cargo test -q --test cli restore` → `0 passed … 10 filtered out` (zero selected: RED by absence).
- `cargo test -q --lib open::` → 53; `--lib classifies_as` → 3; `--lib a_flag_after_a_subcommand` → 1; `--lib usage_` → 3; `--test manifest` → 4.
- `grep -c '^\[\[startup\]\]' herdr-plugin.toml` → `0`.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [x] 0.1 In `tests/cli.rs`, add `restore_replaces_stale_dashboards_and_leaves_a_live_one`: a `write_stub_herdr` script that appends each argv to a log in its `ScratchDir` and answers `pane list` with stale `w1:p2` (`"focused": true`), live `w1:p3`, and stale `w2:p5`, `pane process-info` with M6's verbatim JSON (`notes/measurements.md`), and exit 0 for `plugin pane open`/`pane close`; run the binary as `restore` under `scrubbed()` with piped stdio. Add `"restore"` to `every_run_pipes_and_scrubs_herdr`'s `names_open` set.
- [x] 0.2 RED: assert exit 0, empty stderr, and the full logged sequence — `pane list`; `process-info w1:p2`; `plugin pane open --plugin herdr-openspec --entrypoint dashboard --placement split --target-pane w1:p2 --direction right --focus`; `pane close w1:p2`; `process-info w1:p3`; `process-info w2:p5`; the same open with `w2:p5` and `--no-focus`; `pane close w2:p5` — and nothing else. Add `restore_exits_one_when_herdr_is_unreachable`: the stub exits 1 on `pane list`; assert exit 1, the reason on stderr, and a log of exactly one line.
- [x] 0.3 Confirm `cargo test -q --test cli restore_` selects 2 tests and both fail on `unrecognized argument: restore` (exit 2), not on stub setup.

## 1. The `restore` subcommand
<!-- kind: behavior -->

- [x] 1.1 RED: in `src/lib.rs` tests, add `restore_alone_classifies_as_the_restore_invocation`, extend `a_flag_after_a_subcommand_is_rejected` with `parse(&["restore", "--all"])`, and replace the three-command usage test with `usage_names_all_four_commands` (whole-token command lines; summary `<ui|open|open-tab|restore>`); update `tests/cli.rs`'s usage substring assertion (around line 219) to the new summary. Confirm they fail.
- [x] 1.2 GREEN: add `Invocation::Restore`, its `parse` arms, the `usage` line, and `open::placement_for`'s `Restore => None` arm; add the `main` arm calling `open::restore_from_env()` through the same reporter `run_open` uses. Stub `restore_from_env` to return an empty `Report` with `Ok(())` until group 4.
- [x] 1.3 REFACTOR: if `run_open` and the restore arm repeat the stderr/exit loop, share one reporter; otherwise record that none was needed.
- [x] 1.4 Run `cargo test -q --lib classifies_as` (4), `--lib a_flag_after_a_subcommand` (1), `--lib usage_`, and `--test cli` — green except the two group 0 tests, which now fail on the call sequence; counts recorded.

## 2. Stale-pane classification
<!-- kind: behavior -->

- [x] 2.1 RED: in `src/open.rs` tests, write `a_restored_shell_is_stale`, `a_live_dashboard_is_not_stale`, `a_command_the_user_started_is_not_stale`, `an_unreadable_answer_is_never_stale_and_says_why` (table-driven over every shape the spec lists), `an_unlabelled_pane_is_never_a_candidate`, and `plugin_id_reads_the_variable_and_falls_back` (set, blank, unset), using M6's verbatim JSON.
- [x] 2.2 GREEN: implement `classify(&str) -> Classification` (`Stale` / `Live` / `Unreadable(String)`), `restore_candidates(listing) -> Result<Vec<(String, bool)>, String>` (pane id, focused), and `plugin_id(env)`, and make `open::context` call `plugin_id`; all pure, per specs/dashboard-restore → "A stale dashboard pane is recognised from its process, not its label".
- [x] 2.3 REFACTOR: if `restore_candidates` and `dashboard_panes` repeat the label/pane-id walk, extract it once without changing `dashboard_panes`' tests; otherwise record that none was needed.
- [x] 2.4 Run `cargo test -q --lib open::` — green, count recorded (53 + new).

## 3. Replacing a stale pane in its slot
<!-- kind: behavior -->

- [x] 3.1 RED: write `the_full_call_sequence_for_one_stale_pane`, `a_failed_open_closes_nothing`, and `a_failed_close_keeps_the_replacement_and_warns` against `cli::FakeCli`, asserting the recorded argv exactly, including the absence of `--cwd`.
- [x] 3.2 GREEN: implement `replace(cli, plugin_id, pane_id, focus) -> Result<Vec<String>, String>` (warnings on success, the open failure as `Err`), per specs/dashboard-restore → "A stale dashboard pane is replaced in its own slot".
- [x] 3.3 REFACTOR: none expected — `replace` is two calls; record so, or extract an argv builder shared with `open_args` if one emerges.
- [x] 3.4 Run `cargo test -q --lib open::` — green, count recorded.

## 4. The session-wide sweep
<!-- kind: behavior -->

- [x] 4.1 RED: write `two_stale_dashboards_in_two_workspaces_are_both_replaced`, `a_live_dashboard_is_left_alone`, `no_dashboard_pane_at_all`, `one_failed_replacement_does_not_stop_the_sweep`, `an_unreadable_candidate_is_left_alone_and_warned_about`, and `herdr_unreachable_makes_no_further_call` against `FakeCli`.
- [x] 4.2 GREEN: implement `restore(cli, plugin_id) -> Report` and replace the group 1 stub with `restore_from_env()` (reads `plugin_id(&config::env_lookup())`, requires no workspace), per specs/dashboard-restore → "`restore` replaces every stale dashboard pane in the session" and design.md → Decisions 8 and 10.
- [x] 4.3 REFACTOR: fold any warning-formatting duplicated between `restore` and `open::run` into one helper naming the pane id; otherwise record that none was needed.
- [x] 4.4 Run `cargo test -q --lib open::` and `cargo test -q --test cli restore_` — unit green and both acceptance tests green; counts recorded.

## 5. `open`/`open-tab` replace a stale match
<!-- kind: behavior -->

- [x] 5.1 RED: give the four existing `open::run` tests that list a matching pane a live `process-info` answer and the new call indices and counts — `cross_placement_focus`, `focus_domain_error_stops_and_opens_nothing` (3 calls), `focus_usage_error_warns_and_opens_once`, `the_already_open_path_is_untouched` (3 calls) — and `the_post_open_focus_names_the_newly_opened_pane` (6 calls). Add `a_stale_match_is_replaced_in_place_and_the_replacement_focused`, `a_failed_replacement_open_stops_and_closes_nothing`, `a_failed_close_after_a_successful_replacement_warns_and_still_focuses`, and `an_unclassifiable_match_is_focused_never_replaced` (failing call; JSON without `foreground_processes`; a live answer recording no warning).
- [x] 5.2 GREEN: in `open::run`, classify the first match before focusing; on `Stale`, `replace(…, focus = true)` and continue into the existing post-open re-list and focus; on `Live` focus silently; on `Unreadable` push the reason and focus, per specs/pane-open and design.md → Decisions 6 and 9.
- [x] 5.3 REFACTOR: if the replacement branch duplicates the post-open block, route both through one function; otherwise record that none was needed.
- [x] 5.4 Run `cargo test -q --lib open::` — green, count recorded; the only pre-existing tests changed are the five 5.1 names.

## 6. Manifest
<!-- kind: operational -->

- [x] 6.1 CHECK: in `tests/manifest.rs`, add `startup` to the required keys and add `the_startup_hook_runs_the_restore_subcommand` (exactly one `[[startup]]`, whose only key is `command`, of exactly two elements: the panes' path, then a token `parse` classifies as `Invocation::Restore`); run `cargo test -q --test manifest` — RED, 2 failing.
- [x] 6.2 CHANGE: add `[[startup]] command = ["./target/release/herdr-openspec", "restore"]` and set `min_herdr_version = "0.7.5"` in `herdr-plugin.toml`; mirror both in `SPEC.md` → Herdr integration → Manifest's transcription, with `[[startup]]` at the same table position (`spec_manifest_block_order_matches`), and rewrite that section's min-version paragraph.
- [x] 6.3 CHECK: contract gate — re-read `SPEC.md` → Manifest against `herdr-plugin.toml` for every key and table.
- [x] 6.4 VERIFY: `cargo test -q --test manifest` (6) and `cargo test -q --test doc_contract` — green, counts recorded.

## 7. Degraded-states rows
<!-- kind: operational -->

- [x] 7.1 CHECK: run `cargo test -q --test degraded_coverage` and record the green baseline count.
- [x] 7.2 CHANGE: add the six rows of design.md → Decision 12 to `SPEC.md` → Degraded states and bind each in `tests/degraded-coverage.toml` with `tier`, `proof`, `verdict`, `why`, and `covers`; re-point the six existing `src/open.rs` `covers` ranges that group 5 shifted; change the `0.7.0` floor in the row at `SPEC.md` ~1229 and its verbatim `condition` at `tests/degraded-coverage.toml` ~207 to `0.7.5` together.
- [x] 7.3 VERIFY: `cargo test -q --test degraded_coverage` — green, count = baseline + 6.

## 8. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [x] 8.1 VERIFY: `cargo test -q --test cli restore_` — 2 selected, 2 passed.
- [x] 8.2 REFACTOR: fold any stub-script duplication in `tests/cli.rs` into its existing helpers, or record that none was needed.

## 9. Live check (isolated Herdr)
<!-- kind: operational -->

- [x] 9.1 CHECK: `make build`; start an isolated Herdr under tmux with `HOME` and `XDG_CONFIG_HOME` pointed at a scratch directory and every `HERDR_*` variable unset; confirm `herdr session list --json` reports the scratch session directory; `herdr plugin link .` into it and record `plugin list` and `plugin action list --plugin herdr-openspec`.
- [x] 9.2 CHANGE: open one `dashboard` and one `dashboard-tab` pane, record their ids and `herdr pane layout`, stop the server, and restart it; record from `herdr plugin log` that the hook ran, its environment (`HERDR_SOCKET_PATH`, workspace variables) and output, whether a client was attached when it ran, and from `pane list`/`pane layout`/`pane get` whether each dashboard returned live in its slot with the repo as cwd.
- [x] 9.3 VERIFY: run `restore` again by hand inside the instance and confirm it makes no open or close call; invoke the `open` action on a deliberately stale pane (restart with the hook disabled via `herdr plugin disable`) and confirm it replaces and raises it; stop the instance and delete the scratch directory. Write every observation into `notes/measurements.md` and any Risk it resolves or confirms into design.md.

## 10. Change Review
<!-- kind: operational -->

- [x] 10.1 CHECK: dispatch the `outside-in-tdd-reviewer` agent with the change artifacts and the diff from the planning-review commit to `HEAD` only.
- [x] 10.2 CHANGE: fix every CRITICAL, resolve or accept each WARNING with a one-line reason, re-run affected tests.
- [x] 10.3 VERIFY: confirm no blocking or unowned finding remains.

## 11. Documentation
<!-- kind: operational -->

- [x] 11.1 Add in `README.md` → Install (audience: users installing the plugin): the Herdr 0.7.5 minimum, and two lines saying dashboards come back after a Herdr restart and that an idle shell labelled `OpenSpec` is treated as a dead dashboard. README states neither today, so nothing is replaced.
- [x] 11.2 Rewrite in `SPEC.md` (audience: future implementers): Overview's "invoked two ways by the manifest" (now three), the module-map row for `open` (adds `restore`), the Unit-tested modules entry for `open::` (adds the new pure functions), and Opening the pane's existing-pane paragraph (the stale rule); add a short "Restoring after a restart" subsection, which has no other home.
- [x] 11.3 Rewrite in `PRD.md` line ~107: "Herdr 0.7.0 or later" → 0.7.5 (audience: product readers).
- [x] 11.4 Rewrite in `AGENTS.md` (`CLAUDE.md` is a symlink to it; audience: every future session): Environment's "Herdr 0.7.0 or later" → 0.7.5; the "Two more binary subcommands, `open` and `open-tab`" sentence and the `herdr-plugin.toml` declares… sentence to name `restore` and `[[startup]]`; and `dashboard-restore` in the landed-changes list. Net growth stays under ten lines.

## 12. Lint & Verify
<!-- kind: operational -->

- [ ] 12.1 CHECK: the affected tiers are unit (`--lib`), contract (`--test manifest`, `--test doc_contract`, `--test degraded_coverage`), and acceptance (`--test cli`); `make check` runs all of them.
- [ ] 12.2 VERIFY: `make lint` — 0 warnings.
- [ ] 12.3 VERIFY: `make fmt-check` — clean.
- [ ] 12.4 VERIFY: `make gates` — every gate OK (no seam list moved, per design.md → Boundaries).
- [ ] 12.5 VERIFY: `make test` — green.
- [ ] 12.6 VERIFY: `make coverage` — total ≥ 80% and the production floor holds.
- [ ] 12.7 VERIFY: `make check` as the single gate, naming the failing sub-command if it fails; then `openspec validate dashboard-restore --strict`.
- [ ] 12.8 After `openspec archive`, replace the generated `## Purpose` placeholder in `openspec/specs/dashboard-restore/spec.md` with a real one-paragraph purpose and re-run `cargo test -q --test spec_purposes`.
