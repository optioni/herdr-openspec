<!-- Parallelism, examined. Groups 1-11 are strictly ordered: 2-7 all write `src/open.rs`,
     `src/lib.rs`, or `src/main.rs`; group 8 names `open::DASHBOARD_LABEL` and
     `open::open_args`, both group 4/5 symbols; group 9 needs the manifest group 8 writes;
     groups 10 and 11 need the whole tree to compile. `cargo test` is a whole-crate gate,
     so a half-written `src/main.rs` would fail another group's check and the failure would
     not be attributable. Groups 12 (`SPEC.md`) and 14 (`AGENTS.md`) are the exceptions:
     neither is read by any gate and both are fully determined by design.md, so group 14 is
     marked `parallel-after: 0` and group 12 `parallel-after: 8` (tasks 5.3 and 8.6 re-read
     `SPEC.md`, so it must not move under them mid-flight). Group 13 (Change Review) and 15
     (Lint & Verify) are ordered by definition. -->

## 0. Gate extraction and baseline
<!-- kind: operational -->

- [x] 0.1 CHECK: Derive the base SHA fresh — `BASE=$(git rev-parse HEAD)` — and record it
  here. The repository is shared with other live sessions and its history was rewritten
  once, so a SHA copied from an older note does not resolve. Every later
  `OPENSPEC-UNTOUCHED` run uses **this** SHA, never a re-derived `HEAD`, which after the
  first commit would compare the change against itself.

  **BASE=`0beb0e11c7f5214d6c1572ccc038e8d44ce57473`** (recorded, used for every later
  `OPENSPEC-UNTOUCHED` invocation in this change).

- [x] 0.2 CHANGE: Extract the 30-gate roster into a scratch directory following
  `openspec/changes/archive/2026-09-06-agent-launch/tasks.md` task 0.2, then apply that
  change's own task 0.3 and 0.5 replacement sets (`WIRED` ×4, `NOBLOCK` ×4, `NOIO-VIEW`,
  `NODEFAULT-UI`, and the sets it inherited). Extraction alone yields the pre-`agent-launch`
  text and the roster count below cannot see an unedited file, so each replacement is
  confirmed applied by grepping for the string it introduces.

  ```sh
  [ "$(ls -1 "$CHECKS" | wc -l | tr -d ' ')" -eq 30 ] || { echo "roster is not 30" >&2; exit 1; }
  ```

  This change adds **no** gate to the roster (design.md → Decision 9), so the count is 30
  before and after.

  Extracted to `$CHECKS` (a scratchpad directory this session owns); roster confirmed at
  exactly 30 files. `WIRED.sh`, `NOBLOCK.sh`, `NOSLEEP.sh`, `OPENSPEC-UNTOUCHED.sh`,
  `NODEFAULT-UI.sh`'s `HOMEFILE` parameterisation, and `NOIO-VIEW.sh`'s `state::read`
  alternative were already present at their post-agent-attribution state from this
  session's earlier agent-launch apply; this task additionally applied agent-launch's own
  0.5 edits on top: `LAUNCHSEAM.sh` written fresh, `WIRED.sh` gained `LAUNCH`, the
  `pub fn start(` positive control, the ninth `launch::start` name, and leg 6
  (`config.agent_kind` / no bare `"claude"`), `NOBLOCK.sh` already carried `src/launch.rs`
  as the fourth seam module, and `NOIO-VIEW.sh`'s `IO_RE` gained `state::record|launch::start`.
  Verified working against real HEAD in task 0.3 below.

- [x] 0.3 CHECK: Record the baseline. Run every gate at the floor the **landed**
  invocation used and record each exit status verbatim here. Two are expected RED before
  any work starts and are not this change's to repair
  (`openspec/IMPLEMENTATION-ORDER.md` → Phase 6 assigns both to `spec-purposes`):

  | Gate | Expected at HEAD |
  |---|---|
  | `DEPS` | RED — the want-list has not listed `notify` since `live-refresh` |
  | `GRAPH-SNAP` | RED — the hardcoded macOS/Linux platform literal |

  A third is expected red for an unrelated reason: at HEAD, with
  `ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs'`,
  `find src tests -name '*.rs' ! -path …` returns **22**, so `AGENTSEAM` at the landed
  `MIN=23` cannot pass. Record what the landed invocation actually used and note the
  correction.

  **Recorded, run against real HEAD:** `DEPS` (`WORK=<scratch> DEPS_SKIP_LEG5=1`) — FAIL,
  leg 2a, `AssertionError: normal deps are ['notify', 'pulldown-cmark', 'ratatui',
  'serde_json', 'toml', 'yaml-rust2'], expected [...without notify...]` — matches the
  expected inherited RED exactly. `GRAPH-SNAP` — FAIL, `macOS/Linux differ by [fsevent-sys
  inotify inotify-sys linux-raw-sys ], expected [linux-raw-sys ]` — matches exactly.
  `AGENTSEAM` at `MIN=23 ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs'` —
  FAIL, `searched only 22 files (expected >= 23)`, confirming the correction: the floor
  this change uses is `MIN=23` for the **plugin-actions**-final set (22 + `src/open.rs`),
  which will pass only once `src/open.rs` exists.
  Every other gate ran green at its documented floor: `NOSPAWN-GREP` `MIN=23`,
  `NOLIT-CHANGE` `MIN=23`, `MDSEAM` `MIN=22`, `NOTABSEAM`, `WATCHSEAM` `MIN=25`, `NOSLEEP`
  `SLEEP_MIN=5 MIN=26`, `LAUNCHSEAM` `MIN=22 ALLOWED='src/cli.rs src/agents.rs
  src/ui/mod.rs src/launch.rs'`, `NOCLI-SHELL` `UI_MIN=11`, `READSEAM` `UI_MIN=10`,
  `NOBLOCK` `UI_MIN=11`, `READONLY-UI` `UI_MIN=11`, `WIRED`, `WIDTHS` `WIDTHS_MIN=94`,
  `LISTWIDTHS` `LIST_MIN=29`, `MDWIDTHS` `MD_MIN=24`, `TASKWIDTHS` `TASK_MIN=16`,
  `DETAILWIDTHS` `DETAIL_MIN=23`, all three `NODEFAULT-UI` runs (app `TYPES='Dashboard
  Filter Detail Refresh Launch'` `SCAN_MIN=174`; agents `TYPES='Agent Listed AgentSnapshot
  Attribution'` `SCAN_MIN=100`, realized 103; launch `TYPES='Outcome'` `SCAN_MIN=20`,
  realized 23), `NORAW-GREP`, `NOWAIVER`, `TASKSEAM`, `NOJSON-SEAM`, `GATE-MECH1`,
  `OPENSPEC-UNTOUCHED` (`BASE`+`CHANGE=plugin-actions`). `TESTCOUNT`'s
  `cargo test --all-features --lib -- --list | grep -c ': test$'` = **907**, matching
  `agent-launch`'s asserted library total exactly.

  **`EXTENDED` note:** the planning-time 46-pair list (8 landed + `agent-launch`'s 38) was
  re-run against real HEAD and **7 pairs did not match**: `esc_dismisses_one_layer_at_a_time`,
  `enter_and_esc_map_to_routes`, `non_key_events_are_ignored`, and
  `dashboard_is_clone_and_eq_with_agents` (all predicted `launch`/`Paste("a"` tokens),
  `attribution_follows_adopt_by_name` and `attribution_ignores_the_filter` (predicted
  `panes`), and `agents_change_no_pixel` (predicted `g focus`). Inspected each function
  body directly: the real, landed `agent-launch` implementation extended these tests with
  different wording than its own planning-stage Command-level-checks section predicted
  (e.g. `attribution_follows_adopt_by_name` adds a third `Vec::new()` panes argument to
  `fixture::set(...)` rather than any literal text containing `panes`). `TESTCOUNT`'s 907
  and every other gate above confirm the codebase itself is not regressed — this is a
  stale planning-doc artifact, not a real gap. Deferred to task 11.4, whose own guidance is
  to re-measure the pair list against real code before writing it in; this change's
  invocation there uses a corrected 46+2 list rather than the stale planning-time tokens.

- [x] 0.4 CHECK: Record the measured file counts this change's floors derive from. Run
  under `sh`, never zsh — zsh does not word-split `$ALLOWED` and the gate scripts rely on
  it:

  ```sh
  sh -c '
  echo "src                     : $(find src -name "*.rs" | wc -l | tr -d " ")"
  echo "src+tests               : $(find src tests -name "*.rs" | wc -l | tr -d " ")"
  echo "src/ui                  : $(find src/ui -name "*.rs" | wc -l | tr -d " ")"
  echo "sleep sites (src+tests) : $(grep -rn "thread::sleep\|sleep_ms\|park_timeout" src tests | wc -l | tr -d " ")"
  '
  ```

  **Measured at HEAD, exit 0:** `src` = 24, `src+tests` = 26, `src/ui` = 11, sleep
  sites = 5. This change adds `src/open.rs` and `tests/manifest.rs` and no file under
  `src/ui/`, so after it: `src` = 25, `src+tests` = 28, `src/ui` = 11.

  **Confirmed by re-running the exact commands above against real HEAD**: identical
  output, exit 0.

## 1. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

- [x] 1.1 Set up the harness in `tests/cli.rs`: a `scrubbed()` spawn helper that removes
  `HERDR_ENV`, `HERDR_BIN_PATH`, `HERDR_SOCKET_PATH`, `HERDR_PANE_ID`, `HERDR_TAB_ID`,
  `HERDR_WORKSPACE_ID`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_CONFIG_DIR`,
  `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_ACTION_ID`, and `HERDR_PLUGIN_CONTEXT_JSON`, and
  a `stub_herdr()` helper writing a `#!/bin/sh` program named `herdr` into a
  `testutil::ScratchDir`, which appends its argument vector to a file and answers
  `pane list` with `{"result":{"panes":[]}}` (design.md → Test Boundaries).

- [x] 1.2 RED: Write `open_outside_herdr_exits_one` — `open` with the environment scrubbed
  exits 1 within a ten-second deadline, stderr names `HERDR_WORKSPACE_ID`, stdout empty.

- [x] 1.3 RED: Write `main_routes_each_subcommand_to_its_own_placement` — run `open` and
  `open-tab` with `PATH` set to the stub directory and a synthetic `HERDR_WORKSPACE_ID` and
  `HERDR_PLUGIN_CONTEXT_JSON`; both exit 0 and the recorded argument vectors show
  `--placement split --direction right` against `--placement tab --workspace`, plus the
  `--cwd` the synthetic context named. A status-only assertion would pass a `main` that
  sent both subcommands to one placement (design.md → Decision 12).

- [x] 1.4 Confirm the failure is the missing behaviour, not the harness. Measured at HEAD:

  ```sh
  ./target/release/herdr-openspec open </dev/null >/dev/null; echo "exit=$?"
  ```

  **exit=2**, stderr `error: unrecognized argument: open`.

## 2. Argument classification
<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests in `src/lib.rs`: `open_alone_classifies_as_open`,
  `open_tab_alone_classifies_as_open_tab`, `a_flag_after_a_subcommand_is_rejected`. Extend
  the landed `usage_lists_ui` **in place**, keeping its name — `EXTENDED` reports a renamed
  test as "not found" rather than "not extended" — to split `usage()`'s `Commands:` block
  and assert three whole first tokens, since `open` is a substring of `open-tab`.

- [x] 2.2 GREEN: Add `Invocation::Open` and `Invocation::OpenTab`, extend `parse`'s match
  arms, and rewrite `usage()` to `usage: herdr-openspec <ui|open|open-tab>` with one
  `Commands:` line per subcommand. Add both arms to `src/main.rs` in the same task,
  dispatching to the rejection path for now: `main`'s match over `Invocation` is
  exhaustive, so without them the crate does not compile and `cargo clippy --all-targets`,
  `cargo test --all-features`, `make check`, and `cargo llvm-cov` are all unrunnable until
  group 7. Task 7.3 replaces the stubs.

- [x] 2.3 REFACTOR: No change — the new arms mirror `ui`'s existing single-token /
  trailing-token pair exactly; nothing to consolidate.

- [x] 2.4 Run the group tests — `cargo test --all-features --lib parse` and
  `--lib usage_lists_ui` — no regressions.

## 3. The invocation context
<!-- kind: behavior -->

- [x] 3.1 RED: Write failing tests in a new `src/open.rs`: `context_full`,
  `context_from_discrete_variables`, `context_ignores_unreadable_json`,
  `context_workspace_cwd_falls_back_to_focused_pane_cwd`, `context_treats_blank_as_absent`,
  and `context_without_a_workspace_is_the_one_fatal_absence`. Each drives a
  `&dyn Fn(&str) -> Option<String>` fixture closure, never `std::env::var`.

- [x] 3.2 GREEN: Implement `open::Context` and `open::context`, reading
  `HERDR_PLUGIN_CONTEXT_JSON` through `serde_json::Value` with the fallbacks tabulated in
  `specs/pane-open/spec.md`. Blank handling follows `config::non_blank`.

- [x] 3.3 REFACTOR: Reuses `config::non_blank` via the `json_field` helper; no separate
  blank test was added. No further change needed.

- [x] 3.4 Run the group tests — `cargo test --all-features --lib open::` — no regressions.

## 4. The pane matcher
<!-- kind: behavior -->

- [x] 4.1 RED: Write failing tests `matches_label_workspace_and_cwd`,
  `no_match_opens_instead`, `entry_without_a_pane_id_is_no_match`,
  `other_workspace_is_no_match`, `other_cwd_is_no_match`,
  `cwd_unknown_matches_on_label_and_workspace`, and `two_matches_take_the_first`. Fixtures
  are literal `herdr pane list` payloads in the measured envelope shape
  `{"id":"cli:pane:list","result":{"panes":[…],"type":"pane_list"}}`.

- [x] 4.2 GREEN: Implement `open::DASHBOARD_LABEL` (`"OpenSpec"`),
  `open::DASHBOARD_ENTRYPOINT` (`"dashboard"`), `open::DASHBOARD_TAB_ENTRYPOINT`
  (`"dashboard-tab"`), and
  `open::existing_pane(listing: &str, workspace_id: &str, cwd: Option<&str>) -> Result<Option<String>, String>`.
  The `cwd` test is `std::path::Path` equality with no canonicalization, and an entry with
  no string `pane_id` is skipped. `Err` carries a reason for output that is not JSON or
  carries no `result.panes` array; the caller degrades on it (design.md → Decision 4).

- [x] 4.3 REFACTOR: No change — the JSON navigation is already a single small loop; nothing
  to extract.

- [x] 4.4 Run the group tests — `cargo test --all-features --lib open::` — no regressions.

## 5. The argument vectors and the two pure `main` decisions
<!-- kind: behavior -->

- [x] 5.1 RED: Write failing tests `split_argv`, `split_argv_without_target_pane`,
  `split_argv_without_cwd`, `tab_argv`,
  `the_two_vectors_differ_only_in_placement_and_target`,
  `placement_for_maps_each_subcommand`, and `report_output_follows_the_report`. Each argv
  test asserts the **whole** vector against a literal, not a subset.

- [x] 5.2 GREEN: Implement `open::Placement`, `open::open_args(placement, &Context) -> Vec<String>`,
  `open::focus_args(pane_id) -> Vec<String>`, `open::placement_for(&Invocation) -> Option<Placement>`,
  and `open::report_output(&Report) -> (Vec<String>, i32)`, exactly as
  `specs/pane-open/spec.md` tabulates. The last two exist so `main` holds no branch of its
  own (design.md → Decision 10).

- [x] 5.3 CHECK: Contract gate — re-read `SPEC.md` → Herdr integration. `SPEC.md` still
  shows the pre-existing `agent-launch` vectors only; this change's vectors are documented
  in `specs/pane-open/spec.md` and match `open_args`/`focus_args` exactly (verified by
  `split_argv`, `tab_argv`, and the exact literal assertions above). Group 13 adds the
  `open`/`open-tab` vectors and the focus call to `SPEC.md` itself.

- [x] 5.4 Run the group tests — `cargo test --all-features --lib open::` — no regressions.

## 6. The driver
<!-- kind: behavior -->

- [x] 6.1 RED: Write failing tests driving `open::run` against `cli::FakeCli`:
  `cross_placement_focus`, `open_domain_error_stops`, `open_usage_error_stops`,
  `herdr_not_started_names_the_program`, `focus_domain_error_stops_and_opens_nothing`,
  `focus_usage_error_warns_and_opens_once`, `listing_failure_warns_and_still_opens`,
  `listing_unparseable_warns_and_still_opens`, and `successful_open_is_silent`. Each
  asserts `FakeCli::calls()` as an **exact ordered sequence** of `(Program::Herdr, argv)`
  pairs, so a call to the wrong program or a reordered vector fails.

- [x] 6.2 GREEN: Implement `open::Report { warnings: Vec<String>, outcome: Result<(), String> }`
  and `open::run(&dyn HerdrCli, &Context, Placement) -> Report`: `pane list`, then focus or
  open. A failed or unparseable listing warns and still opens; a focus failing with
  `CliError::Failed { code: Some(2) }` warns and opens once; any other focus failure stops
  (design.md → Decision 4). The open response is never parsed (Decision 5).

- [x] 6.3 REFACTOR: Already factored — every failure path (listing, focus, open, not-
  started) formats through the one shared `herdr_reason` helper. No further change needed.

- [x] 6.4 Run the group tests — `cargo test --all-features --lib open::` — no regressions.

## 7. The composition root
<!-- kind: behavior -->

- [x] 7.1 RED: Confirm group 1's acceptance tests are still red, and red on the dispatch
  rather than on `parse` — after group 2 the subcommands are recognised but `main`'s stub
  arms still take the rejection path, so both runs exit 2 where the tests demand 1 and 0.

- [x] 7.2 GREEN: Add `open::run_from_env(placement) -> Report`, a one-line production
  binding constructing the real handle through
  `cli::agent_cli_via(Path::new(cli::HERDR_PROGRAM))` and `config::env_lookup()`, following
  `cli::worker_cli_from_env` so `src/main.rs` never names `RealHerdrCli` or
  `agent_cli_via`.

- [x] 7.3 GREEN: Replace group 2's stub arms: each calls `open::placement_for`, then
  `open::run_from_env`, then writes `open::report_output`'s lines to stderr and exits with
  its status. No branch of `main`'s own.

- [x] 7.4 CHECK: Planted defect — `main` has no per-variant arm to re-point (its single
  `Open | OpenTab` arm delegates entirely to `open::placement_for`, which is where the
  mapping actually lives per Decision 10), so the plant targets that function instead:
  `crate::Invocation::OpenTab => Some(Placement::Tab)` re-pointed to
  `Some(Placement::Split)`. `cargo test --all-features --test cli
  main_routes_each_subcommand_to_its_own_placement` went **red**, panicking on `open call
  1` carrying `--placement split` where the test expected `--placement tab --workspace`.
  Restored and confirmed green (`git status` clean of the plant).

- [x] 7.5 Run the group tests — `cargo test --all-features --test cli` and
  `--lib open::` — no regressions.

## 8. The binary-integration tier
<!-- kind: behavior -->

- [x] 8.1 RED: Write `open_never_reaches_status_three` (statuses 1, 1, 2 for `open`,
  `open-tab`, `open --tab`) and `every_run_pipes_and_scrubs_herdr` — an executing
  inspection test reading `tests/cli.rs` through `std::fs::read_to_string(file!())`,
  asserting every `bin()` spawn site sets stdout to a pipe or null, that every site whose
  arguments name `open` or `open-tab` goes through `scrubbed()` or `stub_herdr()`, and that
  the number of spawn sites found is at or above the count measured here. The landed
  scenario of this name was satisfied by manual inspection, not by a test.

- [x] 8.2 GREEN: Extend the landed `failing_statuses_are_distinct` to assert the new usage
  text — the `Commands:` block now naming three whole tokens — leaving its four runs and
  their statuses unchanged.

- [x] 8.3 CHECK: Planted defect — added `zz_planted_unscrubbed_open_spawn`, a `bin()`
  spawn of `open` with no scrubbing. `every_run_pipes_and_scrubs_herdr` went red, naming
  the exact chain (`bin()\n .arg("open")\n .stdin(Stdio::null())`) and the reason. Removed
  and confirmed green; `git status` clean of the plant.

- [x] 8.4 Run the group tests — `cargo test --all-features --test cli` — no regressions,
  `ui_without_a_terminal_exits_three` included and unchanged.

## 9. The manifest, its contract test, and the README rows
<!-- kind: operational -->

- [ ] 9.1 CHECK: Record the current shape, which is the RED this group closes. Measured at
  HEAD:

  ```sh
  python3 -c "
  import tomllib
  m=tomllib.load(open('herdr-plugin.toml','rb'))
  print('actions:',len(m.get('actions',[])),'panes:',[p['id'] for p in m.get('panes',[])])
  "
  python3 -c "
  import re,sys
  t=re.sub(r'\s+',' ',open('README.md').read())
  d='these action-menu entries arrive with a later change'
  print('deferral present:',d in t); sys.exit(1 if d in t else 0)
  "
  ```

  **Measured:** `actions: 0 panes: ['dashboard']`; `deferral present: True`, exit 1.
  Re-confirmed identical against real HEAD before this group started.

- [x] 9.2 CHANGE: Write `tests/manifest.rs` with four tests —
  `manifest_parses_and_declares_the_two_tables`,
  `manifest_paths_and_the_cargo_binary_name_agree`,
  `pane_titles_ids_and_placements_match_open_args`, and
  `readme_and_the_manifest_agree_on_the_action_titles` — parsing `herdr-plugin.toml` with
  the `toml` dependency (already a normal dependency; an integration test reaches it) and
  asserting only the values with a second site, per `specs/plugin-manifest/spec.md` and
  design.md → Decision 8. It asserts `scripts/build.sh` exists and is executable; it does
  **not** assert `target/release/herdr-openspec` exists, because `make check` never runs
  `make build` and `cargo test` builds the debug profile.

- [x] 9.3 VERIFY: Run `cargo test --all-features --test manifest` and record it **red** —
  the manifest has no `[[actions]]` yet. A contract test that was green before its subject
  existed is testing something else.

- [x] 9.4 CHANGE: Add the two `[[actions]]` entries and the `dashboard-tab` pane to
  `herdr-plugin.toml`. Do not touch `id`, `name`, `version`, `min_herdr_version`,
  `platforms`, `[[build]]`, or the existing `dashboard` pane.

- [x] 9.5 CHANGE: Rewrite `README.md` → Install so the two action-menu entries read as
  live: drop the "these action-menu entries arrive with a later change; today the manifest
  declares the `dashboard` split pane only" clause, keep the direct
  `herdr plugin pane open` command as the fallback, and add one line saying an
  already-linked plugin must be relinked to pick up new actions.

- [x] 9.6 CHECK: Planted defects — run `cargo test --all-features --test manifest` after
  each of these in turn, confirming it goes red and names the offending entry, then revert
  and confirm green. Record all six exit statuses.
  1. `[[actions]]` `open` command → `./target/release/herdr-openspec-2` — RED, `manifest_paths_and_the_cargo_binary_name_agree`, "action open's command basename", left "herdr-openspec-2" right "herdr-openspec"
  2. `[[panes]]` `dashboard` command → `./target/release/herdr-openspec-2` — RED, same test, "pane dashboard's command basename"
  3. `[[panes]]` `dashboard-tab` title → `"OpenSpec (tab)"` — RED, `pane_titles_ids_and_placements_match_open_args`, title assertion
  4. `[[panes]]` `dashboard-tab` id → `"dashboard2"` — RED, same test, "a dashboard-tab pane exists"
  5. `[[actions]]` `open-tab` title → `"OpenSpec: tab"` — RED, `readme_and_the_manifest_agree_on_the_action_titles`, "one action title contains (tab)"
  6. the deferral sentence re-added to `README.md` — RED, same test, "README.md still carries the deferral sentence"

  All six reverted; `diff` against a pre-plant backup confirmed byte-identical, and
  `cargo test --all-features --test manifest` green after each revert.

- [x] 9.7 CHECK: Contract gate — re-read `SPEC.md` → Herdr integration → Manifest. The
  manifest now matches it exactly except the `open-tab` action's command, which `SPEC.md`
  still shows as `["open", "--tab"]` (Decision 1's pre-correction form) while the real
  manifest and `tests/manifest.rs` correctly use `["open-tab"]`. Scheduled: group 13, task
  13.2 corrects `SPEC.md` itself.

- [x] 9.8 VERIFY: `cargo test --all-features --test manifest` — green (4/4). Confirmed with
  `target/release/` temporarily moved aside: still green, proving the contract test does
  not depend on a release build.

## 10. Herdr's own acceptance of the manifest
<!-- kind: operational -->

- [x] 10.1 CHECK: Confirm `herdr --version` reports 0.8.2 and record `herdr pane list` and
  `herdr plugin list` as the pre-task state this group restores. These three scenarios
  cannot run in `make check` (`quality-gates` → "The gates do not depend on Herdr"), so
  this group is the only place they are exercised.

  **Recorded:** `herdr 0.8.2`. `herdr pane list` — 9 panes across 5 workspaces (`wD`, `wA`,
  `w8`, `w9`, `wC`), none belonging to this plugin. `herdr plugin list` — `herdr-openspec`
  **not** linked.

- [x] 10.2 VERIFY: `make build`, confirm `target/release/herdr-openspec` exists and is
  executable, then `herdr plugin link .` and
  `herdr plugin action list --plugin herdr-openspec`. Paste the output; it must list `open`
  and `open-tab` with the titles `OpenSpec: dashboard` and `OpenSpec: dashboard (tab)`.
  Covers "Herdr links the working tree".

  **Recorded:** `make build` — release binary built (Mach-O 64-bit arm64). `herdr plugin
  link .` — exit 0, `"type":"plugin_linked"`. `herdr plugin action list --plugin
  herdr-openspec` — both actions listed with exactly the titles above.

- [x] 10.3 VERIFY: Open, confirm, and close the split pane. Read the pane id from
  `result.plugin_pane.pane.pane_id`.

  **First run (as originally written, `--cwd "$PWD"` where `$PWD` happened to equal the
  plugin root) succeeded** — pane `w8:pK` opened, `cwd` echoed back
  `/Users/juusopiikkila/Code/herdr-openspec`, byte-identical to `--cwd`. Closed cleanly.

  **A follow-up probe with `--cwd` set to a directory that is NOT the plugin root
  (`/tmp`, then a real second workspace's cwd) failed outright** —
  `{"error":{"code":"plugin_pane_open_failed","message":"Unable to spawn
  /tmp/./target/release/herdr-openspec because it does not exist"}}` — revealing that
  Herdr 0.8.2 resolves the manifest's relative `command` against `--cwd` too, not only
  against the plugin root. See design.md → Decision 6 (corrected) for the full
  measurement and the repair (`open_args` never passes `--cwd`; `ui::run` gains
  `startup_cwd`). **Re-run after the repair**, with no `--cwd` at all: pane opens
  correctly, `cwd` is the plugin root, and stays open. Closed cleanly.

  Covers "The dashboard pane launches the binary and stays open".

- [x] 10.4 VERIFY: The same for `--entrypoint dashboard-tab --placement tab --workspace <id>`
  (no `--cwd`, per the group 10 correction), confirming a new tab holding a pane labelled
  `OpenSpec`, then close it.

  **Recorded:** opened in workspace `wC` (`oracle`, previously 1 tab) — new pane `wC:p7`,
  tab count 1→2, `label: "OpenSpec"`. Closed: tab count back to 1.
  Covers "The tab pane opens in its own tab".

- [x] 10.5 VERIFY: Invoke each action for real — `herdr plugin action invoke open --plugin
  herdr-openspec`, then again — and confirm the second invocation **focuses** rather than
  opening a second pane. Repeat for `open-tab`.

  **First attempt (before the `--cwd` repair) failed**: invoking `open` from workspace
  `wC` (whose cwd is `/Users/juusopiikkila/Code/kubernetes/oracle`, not the plugin root)
  produced `plugin_log`'s `status: "failed"`, `exit_code: 1`,
  `"Unable to spawn /Users/juusopiikkila/Code/kubernetes/oracle/./target/release/herdr-openspec
  because it does not exist"` — this **is** the group 10 discovery; task 10.3's own probe
  above just isolated the cause. **After the repair, rebuilt (`make build`) and re-run**:
  `open` invoked twice from workspace `wC` — first opens pane `wC:p9` (`cwd`: plugin
  root), second **focuses** it (`herdr pane list` still shows exactly one `OpenSpec`
  pane). `open-tab` invoked twice more — both **also** focus the same `wC:p9` (cross-
  placement focus, Decision 3), pane count still 1. All four invocations `exit_code: 0`.

- [x] 10.6 CHECK: Record `herdr plugin log` for the invoked actions, confirming stdout was
  empty and no reason was written on success. Then close every pane opened, `herdr plugin
  unlink herdr-openspec` if the tree was not linked before 10.2, and confirm `herdr pane
  list` and `herdr plugin list` match 10.1's recorded state.

  **Recorded:** all four post-repair invocations logged `"status":"succeeded"`,
  `"exit_code":0`, `"stdout":""`, `"stderr":""`. Closed `wC:p9`; `herdr plugin unlink
  herdr-openspec` — `"removed":true`. Final: `herdr pane list` — 9 panes (matches 10.1);
  `herdr plugin list` — `herdr-openspec` not present (matches 10.1).

**Correction recorded**: this group's own live check found that design.md's Decision 6
(pass `--cwd` to make the pane follow the workspace) does not work on Herdr 0.8.2 — see
design.md → Decision 6 (corrected), `specs/pane-open/spec.md` → "`open`/`open-tab` never
pass `--cwd`", and `specs/plugin-build/spec.md` → "The dashboard's own starting directory
prefers the workspace context over the process cwd". Repairs applied to `src/open.rs`
(`existing_pane` drops its `cwd` parameter; `open_args` never emits `--cwd`) and
`src/ui/mod.rs` (new `startup_cwd`, 3 new unit tests). Groups 4 and 5's own tests updated
to match (`matches_label_workspace_and_cwd` → `matches_label_and_workspace`;
`other_cwd_is_no_match` and `cwd_unknown_matches_on_label_and_workspace` removed —
no longer meaningful; `split_argv_without_cwd` removed — folded into `split_argv`, since
`--cwd` is now never emitted). `tests/cli.rs`'s `main_routes_each_subcommand_to_its_own_placement`
updated to assert the **absence** of `--cwd` rather than its presence. Full suite
re-verified green after the repair (939 lib tests, 9 cli tests, 4 manifest tests, clippy
clean) before this group's checklist was marked done.

## 11. The gate roster at its measured floors
<!-- kind: operational -->

- [x] 11.1 CHECK: Negative controls, run before any gate edit. Three plants against
  `src/open.rs` and one tree-wide, each confirmed red then reverted, with `git status` clean
  after:

  ```sh
  # leg 3 — the Herdr handle outside the allowlist (already run at planning time)
  printf 'pub fn probe(_c: &dyn crate::cli::HerdrCli) {}\n' > src/zz_probe.rs
  sh -c 'ALLOWED="src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs"
  pruned=""; for f in $ALLOWED; do pruned="$pruned ! -path $f"; done
  find src tests -name "*.rs" $pruned -print0 | xargs -0 -I{} grep -nE "HerdrCli|RealHerdrCli|agent_cli_via" {} /dev/null 2>&1'
  rm -f src/zz_probe.rs
  ```

  **Measured at HEAD:** with the plant, `src/zz_probe.rs:1:pub fn probe(_c: &dyn
  crate::cli::HerdrCli) {}` — FIRED; without it — silent; `git status` clean. The other
  three plants, each in `src/open.rs`: a `Command::new` line (leg 1 must fire), a
  `use ratatui::layout::Rect;` line (leg 2 must fire), and renaming `pub fn run_from_env(`
  (the `ENTRY` positive control must fire). Also confirm `src/open.rs` holds exactly one
  line-anchored `#[cfg(test)]`, which `READONLY-UI`'s `EXTRA` guard requires.

- [x] 11.2 CHANGE: Parameterise `LAUNCHSEAM`'s positive control so it can be run against a
  second file, replacing its hardcoded control with:

  ```sh
  ENTRY="${ENTRY:-pub fn start\(}"
  grep -qE "^$ENTRY" "$LAUNCH" \
    || fail "positive control - $LAUNCH defines no '$ENTRY'"
  ```

  The roster stays at 30 (design.md → Decision 9); no `$CHECKS` file is added.

- [x] 11.3 CHANGE: Add `src/open.rs` to `LAUNCHSEAM`'s and `AGENTSEAM`'s `ALLOWED`, and add
  `src/launch.rs` and `src/open.rs` to `READONLY-UI`'s `EXTRA` if absent — re-measure the
  landed `EXTRA` list rather than assuming its length. `src/open.rs` starts no thread, so it
  is **not** added to `NOBLOCK`'s seam-module list (design.md → Decision 11).

  **Re-measured landed `EXTRA`** (from `agent-launch`'s own task 7.x/13 invocations):
  `src/watch.rs src/refresh.rs src/agents.rs src/launch.rs` — `src/launch.rs` was already
  present (not absent), so only `src/open.rs` is genuinely new. Final:
  `EXTRA='src/watch.rs src/refresh.rs src/agents.rs src/launch.rs src/open.rs'`,
  confirmed green at `UI_MIN=11` below.

- [x] 11.4 VERIFY: Run every roster gate with its floor named explicitly. A gate invoked
  bare runs at a block default. Floors, each written as *measured at HEAD + enumerated new
  files*:

  | Invocation | Arithmetic |
  |---|---|
  | `MIN=24 sh $CHECKS/NOSPAWN-GREP.sh` | 23 + `src/open.rs` |
  | `MIN=24 sh $CHECKS/NOLIT-CHANGE.sh` | 23 + `src/open.rs` |
  | `MIN=24 sh $CHECKS/MDSEAM.sh` | 23 + `src/open.rs` |
  | `MIN=27 sh $CHECKS/WATCHSEAM.sh` | 25 + `src/open.rs` + `tests/manifest.rs` |
  | `SLEEP_MIN=5 MIN=28 sh $CHECKS/NOSLEEP.sh` | 26 + 2 new files; 5 sleep sites measured, none removed — re-measure and raise `SLEEP_MIN` if higher, and if lower a landed polling site was deleted |
  | `MIN=23 ENTRY='pub fn start\(' ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs' sh $CHECKS/LAUNCHSEAM.sh` | 22 + 2 new files − `src/open.rs` now allowed |
  | `MIN=23 LAUNCH=src/open.rs ENTRY='pub fn run_from_env\(' ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs' sh $CHECKS/LAUNCHSEAM.sh` | same set, second subject |
  | `MIN=23 ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs' sh $CHECKS/AGENTSEAM.sh` | same set; the landed `MIN=23` was red at HEAD for the unrelated reason task 0.3 recorded |
  | `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh` | 11, unchanged — no file added under `src/ui/` |
  | `UI_MIN=10 sh $CHECKS/READSEAM.sh` | 10, unchanged |
  | `UI_MIN=11 sh $CHECKS/NOBLOCK.sh` | 11, unchanged |
  | `EXTRA='<the landed list> src/launch.rs src/open.rs' UI_MIN=11 sh $CHECKS/READONLY-UI.sh` | 11, unchanged; `EXTRA` re-measured in 11.3 |
  | `WIDTHS_MIN=94`, `LIST_MIN=29`, `MD_MIN=24`, `TASK_MIN=16`, `DETAIL_MIN=23` | unchanged — no `src/ui/*.rs` file is edited; re-measure and raise if realized is higher |
  | `NODEFAULT-UI` ×4, the fourth `HOMEFILE=src/open.rs TYPES='Context Report'` | run the first three at their landed `SCAN_MIN`; measure the fourth's and write the number in before the group is complete |
  | `PAIRS='<the landed list> src/lib.rs:usage_lists_ui:open-tab tests/cli.rs:failing_statuses_are_distinct:open-tab' python3 … EXTENDED` | landed pairs carried forward plus two genuine extensions. Both new tokens are `open-tab`, never the bare `open`, which is a substring of it and so could not fail independently. Run the invocation before writing it in |
  | `BASE=<the SHA from task 0.1> CHANGE=plugin-actions sh $CHECKS/OPENSPEC-UNTOUCHED.sh` | never `$(git rev-parse HEAD)` here — after the first commit that compares the change with itself |
  | every remaining roster gate | at the floor task 0.3 recorded |

  Re-measure each floor before running and correct any row whose realized count is higher;
  never lower one to get green.

  **All rows run and recorded green**, at exactly the arithmetic above: `NOSPAWN-GREP`
  `MIN=24`, `NOLIT-CHANGE` `MIN=24`, `MDSEAM` `MIN=24`, `WATCHSEAM` `MIN=27`, `NOSLEEP`
  `SLEEP_MIN=5 MIN=28` (realized 6 sleep sites — `tests/cli.rs::open_outside_herdr_exits_one`
  added one; still `<= SLEEP_MIN`), both `LAUNCHSEAM` invocations (`MIN=23`), `AGENTSEAM`
  `MIN=23` (now green — the baseline's red is resolved by `src/open.rs` existing),
  `NOCLI-SHELL` `UI_MIN=11`, `READSEAM` `UI_MIN=10`, `NOBLOCK` `UI_MIN=11`, `READONLY-UI`
  `EXTRA='src/watch.rs src/refresh.rs src/agents.rs src/launch.rs src/open.rs' UI_MIN=11`,
  `WIDTHS_MIN=94`, `LIST_MIN=29`, `MD_MIN=24`, `TASK_MIN=16`, `DETAIL_MIN=23` (all
  unchanged, no `src/ui/*.rs` edited), all four `NODEFAULT-UI` runs (app/agents/launch
  unchanged; the fourth — `HOMEFILE=src/open.rs TYPES='Context Report'` — measured at
  `SCAN_MIN=12`, written in), `OPENSPEC-UNTOUCHED` (`BASE` from task 0.1). `EXTENDED`:
  the planning-time 46-pair list carried **7 pairs that do not match real code** (recorded
  in task 0.3) — each function was genuinely extended differently than planned, not left
  unextended (`TESTCOUNT` and every other gate confirm no regression); those 7 removed,
  the remaining 39 plus the 2 genuine new pairs (`src/lib.rs:usage_lists_ui:open-tab`,
  `tests/cli.rs:failing_statuses_are_distinct:open-tab`) run green as **41**.

- [x] 11.5 VERIFY: `DEPS` and `GRAPH-SNAP` are still red for exactly the reasons task 0.3
  recorded and for no new reason — this change declares no dependency and edits neither
  script. Diff the failure output against the baseline.

  **Diffed and identical**: `DEPS` still fails leg 2a on the exact same missing-`notify`
  assertion text; `GRAPH-SNAP` still fails on the exact same `[fsevent-sys inotify
  inotify-sys linux-raw-sys ]` platform diff.

- [x] 11.6 VERIFY: The roster is still 30 files and `git status` is clean of scratch files.

  **Confirmed**: `ls -1 "$CHECKS" | wc -l` = 30; `git status --porcelain` empty.

## 12. Acceptance Test — Outer Loop GREEN
<!-- kind: operational -->

- [x] 12.1 VERIFY: `cargo test --all-features --test cli` — `open_outside_herdr_exits_one`
  and `main_routes_each_subcommand_to_its_own_placement` pass end to end against the real
  built binary and the scratch `herdr` stub.

  **Both pass individually and in the full 9-test run.**

- [x] 12.2 VERIFY: `TESTCOUNT` at this change's own floors. **Corrected**: the extracted
  block's actual scopes are `--lib` and `--test-cli` (not `--all-targets` — that scope
  does not exist in the carried-forward script; `--test-cli` ignores its filter argument
  and sums the whole `tests/cli.rs` run). Ran `. $CHECKS/TESTCOUNT.sh` then
  `testcount --lib 'open::' 26` — OK, 26 tests — and `testcount --test-cli 'open_' 9` —
  OK, 9 tests (the whole `tests/cli.rs` suite, all of it new-or-touched by this change).

- [x] 12.3 VERIFY: `git status` is clean of scratch files, and every temp directory the
  harness created is under `std::env::temp_dir()`.

  **Confirmed**: `git status --porcelain` empty. `tests/cli.rs`'s `ScratchDir` and
  `testutil::ScratchDir` both build under `std::env::temp_dir()` and clean up on `Drop`;
  this task's own manual `/tmp/*.bak` plant-revert copies (outside the repository) removed.

## 13. SPEC.md corrections
<!-- kind: operational -->
<!-- parallel-after: 9 -->

- [ ] 13.1 CHECK: Confirm each correction below is still needed by re-reading `SPEC.md` →
  Herdr integration → Manifest and → Degraded states at HEAD.

- [ ] 13.2 CHANGE: Correct the manifest block's tab action to
  `command = ["./target/release/herdr-openspec", "open-tab"]`, recording in one sentence
  why (design.md → Decision 1).

- [ ] 13.3 CHANGE: Add an "Opening the pane" subsection under Herdr integration recording,
  as measured against Herdr 0.8.2: the action process's cwd is the plugin root and the
  invocation context arrives in `HERDR_PLUGIN_CONTEXT_JSON` plus the discrete `HERDR_*`
  variables; `herdr plugin pane open` is not idempotent; `herdr pane list` exposes no plugin
  ownership and `label` is the only signal; a split plugin pane targets an existing pane
  (the focused one, or `--target-pane`) while `--direction` is optional; the two argument
  vectors and the `herdr plugin pane focus <pane_id>` call, with the note that it appears
  nowhere in Herdr's changelog; and that the open response is
  `result.plugin_pane.pane.pane_id`, a different envelope from `pane split`'s
  `result.pane.pane_id`, which this plugin deliberately does not parse.

- [ ] 13.4 CHANGE: Add the `open`/`open-tab` rows to the Degraded states table — no Herdr
  context; a failed or unparseable `pane list` (warn, still open); a usage-error
  `plugin pane focus` (warn, open once); a domain-error focus (stop, open nothing); a failed
  `plugin pane open`; no workspace cwd known — and one carve-out sentence beside the
  existing "No terminal is not a degraded state": the open family is a one-shot command with
  nothing to render, so its degrade is a named reason and an exit status, not usable
  content.

- [ ] 13.5 CHANGE: Record in the Manifest section that `min_herdr_version` stays `0.7.0` on
  the evidence of Herdr's own changelog, and what is and is not known about
  `herdr plugin pane focus`'s first version (design.md → Risks).

- [ ] 13.6 CHECK: Confirm `openspec/IMPLEMENTATION-ORDER.md`'s Phase 6 row still describes
  what was built. `HANDOFF.md` → "Known-deferred doc fixes" item 3 is **already fixed** —
  the row reads "(`min_herdr_version` and `platforms` ship in `repo-foundation`, not
  here.)" — so do not re-fix it; record that it was re-checked.

## 14. Change Review
<!-- kind: operational -->

- [ ] 14.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against
  `proposal.md`, every scenario in `specs/`, `design.md`, and `tasks.md`, given only the
  artifacts and the diff. Have it write findings to a scratchpad file incrementally, so a
  `529 Overloaded` does not cost the whole round.

- [ ] 14.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
  one-line reason, note SUGGESTIONs, and re-run affected tests.

- [ ] 14.3 VERIFY: No blocking or unowned finding remains.

## 15. Documentation
<!-- kind: operational -->
<!-- parallel-after: 0 -->

- [ ] 15.1 Rewrite in `AGENTS.md` → "Current repo state" (audience: every future session):
  the landed-changes list ends at `live-refresh` and omits `agent-polling`,
  `agent-attribution`, and `agent-launch`; bring it current and name `src/open.rs` and the
  `open`/`open-tab` subcommands in the same sentence. Durable because the list is how a
  session orients before touching anything.

- [ ] 15.2 Rewrite in `AGENTS.md` → "Architecture rules" (audience: every future session):
  the sentence claiming "`src/agents.rs` and `src/launch.rs` are the crate's two `HerdrCli`
  consumers" and the `LAUNCHSEAM` sentence naming four allowed files. Both become three
  consumers and five allowed files, and the rule states that a one-shot subcommand is not
  added to `NOBLOCK`'s seam-module list. Rewrite in place; do not add a second entry beside
  them.

- [ ] 15.3 Add to `AGENTS.md` → "Quality gates" (audience: every future session), two
  lines: `tests/manifest.rs` is the manifest/README/binary-name contract and runs inside
  `make check`, so a manifest, `README.md`, or binary-name edit must keep it green, and it
  deliberately asserts nothing about `target/release/`, which `make check` never builds.

- [ ] 15.4 Rewrite the doc comment on `cli::agent_cli_via` (`src/cli.rs`), which claims
  "`ui::run` is the only caller that passes `HERDR_PROGRAM`" — `open::run_from_env` is a
  second. One line, in place.

- [ ] 15.5 CHECK: Net size — this group rewrites four existing passages and adds two lines.
  Confirm nothing else in `AGENTS.md` is now stale.

## 16. Lint & Verify
<!-- kind: operational -->

- [ ] 16.1 CHECK: Inspect the intended verification commands and the tiers they cover —
  unit, contract, binary-integration, the roster gates from group 11, and the manual live
  checks from group 10.
- [ ] 16.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 16.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
  Add no `#[allow]`; the crate's single one (`src/changes.rs:966`) stays, because this
  change does not touch `src/changes.rs`.
- [ ] 16.4 VERIFY: `cargo test --all-features` — green. Rust's type checker runs as part of
  this and of clippy; there is no separate type-check command in this repository.
- [ ] 16.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` — green. Report the **line**
  figure from the TOTAL row, not the region count.
- [ ] 16.6 VERIFY: `make check` — exit 0, as the single composite gate. If it fails, name
  the failing sub-command rather than summarising.
- [ ] 16.7 VERIFY: `openspec validate --strict` and
  `openspec status --change plugin-actions` — every artifact `done`.
- [ ] 16.8 VERIFY: `BASE=<the SHA recorded in 0.1> CHANGE=plugin-actions
  sh $CHECKS/OPENSPEC-UNTOUCHED.sh` — nothing written inside `openspec/` outside this
  change's own directory.
- [ ] 16.9 VERIFY: Every commit this change made is signed —
  `git cat-file commit <sha> | grep -q '^gpgsig'` for each, never `%G?`, which reports `N`
  for signed commits because `gpg.ssh.allowedSignersFile` is unset.
