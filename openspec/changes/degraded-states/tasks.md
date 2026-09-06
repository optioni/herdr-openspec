<!-- Ordering: sequential. Groups 3-9 all write to `src/ui/view.rs`, and groups 2-6 to
     `src/ui/mod.rs`; one file is shared mutable state. Groups 11-12 touch no `src/` file and
     were the one candidate pair, but every group's closing task runs `make check`, a
     whole-tree gate that would attribute one group's failure to the other. No
     `parallel-after` marker is set anywhere, deliberately. -->

<!-- Every floor below is written as measured-at-HEAD plus enumerated-new, with the command
     that produced the measurement. All measurements were taken from the repository root at
     `89cb3b2` while this plan was written, and re-verified by an independent reviewer. `$C` is
     `openspec/changes/archive/2026-09-06-spec-purposes/notes/extracted-gates`. -->

## 0. Baseline and measurement
<!-- kind: operational -->

- [x] 0.1 CHECK: Capture the base SHA **once** and record it in `notes/baseline.md`:
  `git rev-parse HEAD` (`89cb3b2` while this plan was written). Every later
  `OPENSPEC-UNTOUCHED` run reads that recorded value and first confirms it still resolves
  (`git cat-file -e $BASE^{commit}`); it does **not** re-derive `BASE` as the current `HEAD`,
  which would diff `HEAD` against a clean tree and pass over the writes the check exists to
  catch. If the recorded SHA stops resolving — this repository's history was rewritten once —
  re-derive it as the parent of this change's first commit and record why.
- [x] 0.2 CHECK: Record the coverage baseline as the **line** figure:
  `cargo llvm-cov --fail-under-lines 80 2>&1 | tail -3`. Expected at HEAD: **97.05% of
  25,674 lines**. The floor is never lowered and no exclusion is ever added.
- [x] 0.3 CHECK: Reproduce `WIRED` red and paste the failure into `notes/baseline.md`:
  `sh $C/WIRED.sh` — **run at planning time, exit 1:**
  `WIRED FAIL (leg 2): 'pub fn run()' holds a branch or a loop: 5:    let cwd = match startup_cwd(&crate::config::env_lookup()) {`
- [x] 0.4 CHECK: Record every gate's realized count into `notes/gate-floors.md` by running each
  **from the repository root** — run from its own directory each fails "no src directory",
  which is a false failure. Measured at planning time and independently re-verified; each
  number is the gate's own `OK` line:

  | Gate | Floor variable (default) | Measured at HEAD | Invocation |
  |---|---|---|---|
  | `NOSPAWN-GREP` | `MIN` (20) | 24 | `MIN=24 sh $C/NOSPAWN-GREP.sh` |
  | `NOLIT-CHANGE` | `MIN` (20) | 24 | `MIN=24 sh $C/NOLIT-CHANGE.sh` |
  | `MDSEAM` | `MIN` (16) | 24 | `MIN=24 sh $C/MDSEAM.sh` |
  | `WATCHSEAM` | `MIN` (20) | 28 | `MIN=27 sh $C/WATCHSEAM.sh` → "28 files searched" |
  | `AGENTSEAM` | `MIN` (20), `ALLOWED` (3 files) | 24, 5 files | `MIN=23 ALLOWED='src/cli.rs src/agents.rs src/ui/mod.rs src/launch.rs src/open.rs' sh $C/AGENTSEAM.sh` |
  | `LAUNCHSEAM` (`src/launch.rs`) | `MIN` (22), `ALLOWED` (4 files) | 24, 5 files | as above, `ENTRY='pub fn start\('` |
  | `LAUNCHSEAM` (`src/open.rs`) | `MIN`, `LAUNCH`, `ENTRY` | 24 | as above, `LAUNCH=src/open.rs ENTRY='pub fn run_from_env\('` |
  | `NOCLI-SHELL` | `UI_MIN` | 11 | `UI_MIN=11 sh $C/NOCLI-SHELL.sh` |
  | `READSEAM` | `UI_MIN` (7) | 10 | `UI_MIN=10 sh $C/READSEAM.sh` |
  | `NOBLOCK` | `UI_MIN` | 11 | `UI_MIN=11 sh $C/NOBLOCK.sh` |
  | `READONLY-UI` | `UI_MIN` (10), `EXTRA` (**empty**) | 11, EXTRA measured in 0.5 | `sh $C/READONLY-UI.sh` — bare runs `EXTRA` empty, weaker than `plugin-actions` ran it |
  | `NORAW-GREP` | none exposed (16 hardcoded inside) | 28 files searched | `sh $C/NORAW-GREP.sh` |
  | `NOSLEEP` | `MIN` (20), `SLEEP_MIN` (3) | 28, **6** | `SLEEP_MIN=6 MIN=28 sh $C/NOSLEEP.sh` |
  | `NODEFAULT-UI` | `SCAN_MIN` (60), `TYPES`, `HOMEFILE` | **per type set** — see 0.5 | `sh $C/NODEFAULT-UI.sh` → 126 spans for the default set |
  | `GATE-MECH1` | none | 25 files, 84 constructions | `python3 $C/GATE-MECH1.py` |
  | `WIDTHS` | `WIDTHS_MIN` (16) | 94 | `WIDTHS_MIN=94 sh $C/WIDTHS.sh` |
  | `LISTWIDTHS` | `LIST_MIN` (17) | 29 | `sh $C/LISTWIDTHS.sh` |
  | `MDWIDTHS` | `MD_MIN` (23) | 24 | `sh $C/MDWIDTHS.sh` |
  | `TASKWIDTHS` | `TASK_MIN` (14) | 16 | `sh $C/TASKWIDTHS.sh` |
  | `DETAILWIDTHS` | `DETAIL_MIN` | 23 | `DETAIL_MIN=23 sh $C/DETAILWIDTHS.sh` |
  | `NOJSON-SEAM`, `NOTABSEAM`, `TASKSEAM`, `NOWAIVER` | none | structural | bare |

  Confirm each variable's **name** against the script before 11.2 edits it — three of the
  width gates take `LIST_MIN`/`MD_MIN`/`TASK_MIN`, not the gate's own name.
- [x] 0.5 CHECK: Measure `NODEFAULT-UI`'s `SCAN_MIN` for **each** of its four type sets
  separately (the `src/ui/app.rs` `Dashboard`/`Filter`/`Detail` set measures 126; the
  `Refresh` set, the `src/agents.rs` set, and the `src/launch.rs` set each measure their own,
  and at least one is below the script's default of 60). One shared floor is rejected in
  design.md → Decision 6. Record all four.
- [x] 0.6 CHECK: Measure `READONLY-UI` with `EXTRA='src/watch.rs src/refresh.rs src/agents.rs
  src/launch.rs src/open.rs'`, confirm each file exists, and record the resulting count — this
  becomes the extracted script's default.
- [x] 0.7 VERIFY: `make check` is green at HEAD before any edit — the control that a later red
  belongs to this change.

## 1. Audit every row of the degraded-states table
<!-- kind: operational -->
<!-- No behaviour changes here. The evidence is `notes/audit.md` plus the `verdict` key of the
     coverage map group 10 builds; task 1.5's check is what fails when a row has no entry. -->

- [x] 1.1 CHECK: Re-derive the table's row count from the tree. Use **this** parse rather than
  a `grep -c '^| '`, which misses the `|---|---|` separator (no space after the pipe) and
  reports one too many:

  ```sh
  python3 - <<'PY'
  import re
  t = open('SPEC.md').read()
  s = t.index('## Degraded states'); e = t.index('### No terminal is not a degraded state')
  rows = [l for l in t[s:e].splitlines() if l.startswith('|')]
  data = [l for l in rows
          if not re.match(r'^\|[\s\-:|]*$', l) and not l.startswith('| Condition')]
  print(len(data))
  PY
  ```

  **Run at planning time — prints `44`**; first row `No \`openspec/\` found while walking up`,
  last row `The dashboard process opened by \`open\`/\`open-tab\` finds no workspace cwd…`. This
  is the same parse `tests/degraded_coverage.rs` implements in group 10, so the plan's floor
  and the checker's floor come from one rule.
- [x] 1.2 CHECK: Confirm the number matches the 44 in proposal.md, design.md, and
  `specs/degraded-coverage/spec.md`. If `SPEC.md` has moved since planning, correct all three
  and the coverage floor together.
- [x] 1.3 CHANGE: For each row, record in `notes/audit.md`: the condition verbatim, the
  production code implementing it (file, line, deciding expression), the test that proves it
  today (or `none`), and one of the **five** verdicts — `confirmed`, `unproven`,
  `spec-corrected`, `repaired`, `implemented`. The same five, and no others, are the legal
  values of the coverage map's `verdict` key. Design.md → Context carries the pre-planning
  audit's verdicts and its row-number partition; this task **re-checks** each against the tree
  rather than transcribing it.
- [x] 1.4 CHANGE: For every `repaired` verdict, name the earlier archived change that should
  have shipped the behaviour. Design.md names one (`agent-launch`, row 23). A row that turns
  out to need repair and is not that one is **reported to the user as a finding** before it is
  folded in.
- [x] 1.5 CHECK: Every parsed condition appears exactly once in `notes/audit.md` with one of
  the five verdicts. A one-off script here; group 10 turns the machine-checkable half into
  `tests/degraded_coverage.rs`, keyed on `tests/degraded-coverage.toml` rather than on a path
  under `openspec/changes/`, which archiving moves.
- [x] 1.6 Commit the audit.

## 2. `startup_dir`, `WIRED`'s repair, and `WIRED` as a repository file
<!-- kind: behavior -->
<!-- `WIRED` is extracted here rather than in group 11 because this group must edit it, and
     the archived copy under openspec/changes/archive/ may not be written to. -->

- [x] 2.1 RED: Write failing unit tests for `ui::startup_dir` —
  `startup_dir_prefers_the_workspace_cwd`, `startup_dir_propagates_a_failing_fallback`,
  `startup_dir_treats_a_contextless_environment_as_fallback`. Each injects the environment
  lookup and a counter-instrumented fallback closure, so neither the process environment nor
  the process working directory is touched. RED at HEAD because `startup_dir` does not exist —
  `ui::startup_cwd` (`src/ui/mod.rs:220`, three tests) is a different function and stays.
- [x] 2.2 GREEN: Add `pub(crate) fn startup_dir(env: &dyn Fn(&str) -> Option<String>, fallback:
  &dyn Fn() -> std::io::Result<PathBuf>) -> std::io::Result<PathBuf>` to `src/ui/mod.rs`,
  holding the decision `run` holds today.
- [x] 2.3 GREEN: Replace the `match` in `pub fn run()` with
  `let cwd = startup_dir(&crate::config::env_lookup(), &|| std::env::current_dir())?;`.
- [x] 2.4 CHANGE: Copy `$C/WIRED.sh` to `scripts/gates/wired.sh` — the archived copy is under
  `openspec/`, which nothing in this repository may write to — and add leg 5b there: `run`'s
  body must **name** `startup_dir(`. Both halves are needed (design.md → Decision 5); leg 2
  alone passes on a body that dropped the call.
- [x] 2.5 VERIFY: `sh scripts/gates/wired.sh` exits 0. Plant the negative control — reinstate
  the `match` — confirm leg 2 fires, remove the plant, confirm green.
- [x] 2.6 VERIFY: Plant the second control — delete the `startup_dir(` call and inline
  `std::env::current_dir()?` — confirm leg 5b fires, remove the plant.
- [x] 2.7 VERIFY: `git diff --name-only` names only `src/ui/mod.rs` and
  `scripts/gates/wired.sh`; nothing under `openspec/` was written.
- [x] 2.8 Run `cargo test --all-features startup_dir` and `make check`. Commit.

## 3. The probe seam, `file_mode`, and the startup problems that reach the pane
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests for `run_wired_probes_through_the_injected_hook` (fixture
  environment map with no `PATH`, `npm` hook returning `None`, **plus** a resolving control
  where the hook returns a scratch prefix holding a usable `openspec`),
  `load_never_claims_file_mode`, `run_wired_sets_file_mode_from_the_probe`,
  `unusable_openspec_bin_renders_as_a_problem_row`, `no_binary_is_file_mode_not_an_error`,
  `a_resolved_binary_adds_no_problem_and_no_badge`,
  `config_fallback_renders_as_a_leading_problem_row`,
  `startup_problems_are_ordered_config_then_binary_then_watcher`, `a_clean_config_adds_no_row`.
- [ ] 3.2 GREEN: Add the environment lookup and the `npm prefix -g` hook to `ui::Startup`
  (design.md → Decision 14), have `start_collaborators` reach the probe through
  `resolve::openspec_bin` and `cli::worker_cli` rather than `worker_cli_from_env`, and pass
  `config::env_lookup()` and `cli::npm_prefix` from `run`. Without this the file-mode tests
  read the developer's `PATH` and spawn the real `npm`.
- [ ] 3.3 GREEN: Add `file_mode: bool` to `ui::app::Dashboard` and name it at every
  construction site. The compiler enumerates them.
- [ ] 3.4 GREEN: Change `cli::worker_cli` to surrender `BinResolution::problems` alongside the
  handle instead of dropping it.
- [ ] 3.5 GREEN: In `start_collaborators`, fold `Config::problems`, then the resolution's
  problems, into the `problems` vector already handed to `run_wired`, and report `file_mode`.
  `run_wired` assigns it; `ui::load` sets `false` on both branches.
- [ ] 3.6 CHECK: The two whole-buffer-equality tests in 3.1
  (`a_resolved_binary_adds_no_problem_and_no_badge`, `a_clean_config_adds_no_row`) each carry a
  **discriminating control** — a sibling assertion on a dashboard that *does* produce the row,
  so the equality cannot be satisfied by both buffers being empty.
- [ ] 3.7 CHECK: Contract gate — re-inspect `cli::worker_cli`'s and `ui::Startup`'s shapes
  against design.md → Contracts, and every caller. `sh scripts/gates/wired.sh`,
  `UI_MIN=11 sh $C/NOCLI-SHELL.sh`, and `UI_MIN=11 sh $C/NOBLOCK.sh` stay green —
  `start_collaborators` must still name no `npm_prefix`.
- [ ] 3.8 REFACTOR: Extract the problem ordering into one named helper if
  `start_collaborators` has grown past readable; otherwise state that no refactor was needed.
- [ ] 3.9 Run `make check`. Commit.

## 4. The `file mode` header badge
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing view tests `file_mode_badge_is_dim_after_the_label`,
  `no_badge_is_byte_identical_to_the_landed_header`, `badge_rebases_the_shortening_arithmetic`,
  `badge_drops_whole_below_eighteen_columns`. The dim assertion reads the buffer's `Modifier`,
  not only its characters. Every one names **both 60 and 120** — the drop test renders at 17,
  18, 60 **and** 120, because `WIDTHS` requires every view test to name both mandated widths
  and a 17/18/60 test would fail it.
- [ ] 4.2 CHECK: `no_badge_is_byte_identical_to_the_landed_header` carries a discriminating
  control: the same fixture with `file_mode` true must differ, so the equality is not satisfied
  by two blank headers.
- [ ] 4.3 GREEN: Draw `file mode` in columns 9–17 with `Modifier::DIM` when
  `dashboard.file_mode`; re-base the shortening arithmetic to `width - 19` with the badge and
  `width - 9` without, floored at zero either way.
- [ ] 4.4 GREEN: Drop the badge whole below 18 columns, before the path is shortened.
- [ ] 4.5 VERIFY: The four landed header scenarios pass unchanged — they build dashboards with
  `file_mode` false, so their column spans must not move.
- [ ] 4.6 VERIFY: `WIDTHS` at its raised floor (11.4) and `sh $C/NOIO-VIEW.sh`.
- [ ] 4.7 Run `make check`. Commit.

## 5. Per-change problems in the detail region
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing view tests `change_problems_render_above_the_content`,
  `change_problem_precedes_tab_problem`, `no_selected_change_contributes_no_lines`,
  `change_problems_are_inside_the_scrolled_region` (twenty entries, forty `j` presses, a
  fourteen-row content area). Both widths.
- [ ] 5.2 GREEN: In `ui::detail::content_lines`, prepend one `! `-prefixed line per entry of
  the selected change's own `problems`, above the `Detail::problems` lines, using the same
  `pad_or_truncate_right` call and plain face.
- [ ] 5.3 VERIFY: `sh $C/NOTABSEAM.sh` and `sh $C/NOIO-VIEW.sh` green, and a test drives
  `content_lines` with `change: None`, an empty `Detail`, and width `0` without panicking.
- [ ] 5.4 VERIFY: `detail-scroll`'s clamp computes against the full returned length, so no
  clamp code changes; state explicitly if it turns out otherwise.
- [ ] 5.5 Run `make check`. Commit.

## 6. Row 23 repair — a launch outcome carries every problem
<!-- kind: behavior -->
<!-- The one behavioural defect the audit found. `agent-launch` should have shipped it. -->

- [x] 6.1 RED: Write `a_failed_record_and_a_failed_prompt_are_both_reported` — split and start
  succeed, the state directory is an existing regular file, `agent prompt` then exits 1 with
  `agent_blocked`. Assert `problems` holds **two** entries in occurrence order and `named` is
  `Some`. RED at HEAD: `src/launch.rs:308-313` returns only the prompt's reason.
- [x] 6.2 RED: Write the view half — the two reasons as the list's first two `! `-marked
  interior rows, in that order, both widths — and `a_success_clears_both_entries`.
- [x] 6.3 GREEN: Change `launch::Outcome::problem: Option<String>` to `problems: Vec<String>`
  and push each accumulated reason. Update the three internal call sites.
- [x] 6.4 CHANGE: Update the four landed `src/launch.rs` failure tests for the new shape
  without weakening any assertion — each still names its own reason and its own `named` value.
  Update the `launch::Outcome` compile-time destructuring companion.
- [x] 6.5 CHECK: Contract gate — re-inspect `Outcome` against design.md → Contracts, and
  confirm `agent-launch`'s "at most one entry" claim reads "at most two" in the delta spec and
  in `SPEC.md`.
- [x] 6.6 Run `cargo test --all-features launch` and `make check`. Commit.

## 7. Proofs: the schema, artifact, and tasks rows
<!-- kind: operational -->
<!-- These rows' behaviour already exists and is correct, so there is no honest RED for it.
     Each test is instead proved by planting the ABSENCE of the degraded state, showing the
     test red, removing the plant, and showing it green. -->

- [ ] 7.1 CHANGE: Write `a_cli_rejected_schema_names_its_reason_per_change`,
  `an_unusable_schema_renders_no_artifacts` (not-vendored and unparseable sub-cases),
  `no_tasks_artifact_renders_every_tab_as_markdown`,
  `an_unsupported_glob_names_its_reason_and_spares_the_others`,
  `an_unreadable_tasks_file_is_zero_with_a_named_reason` (directory and invalid-UTF-8
  fixtures). All at both mandated widths.
- [ ] 7.2 CHECK: Plant **one defect per test**, not one for the group. For the four that assert
  a line is *present*, the plant clears the source vector before rendering. For
  `an_unusable_schema_renders_no_artifacts`'s not-vendored half and for
  `no_tasks_artifact_renders_every_tab_as_markdown`, which assert an *absence*, the plant must
  add what must not be there — a fabricated artifact list, and a `tracks_tasks` flag on one
  artifact respectively. Record each plant and its red output in `notes/planted-defects.md`.
- [ ] 7.3 CHECK: After every plant is removed, `git diff --name-only` names only this group's
  intended files and no scratch fixture is left behind.
- [ ] 7.4 Run `make check`. Commit.

## 8. Proofs: the launch, agent, and CLI/watcher rows
<!-- kind: operational -->

- [ ] 8.1 CHANGE: Write `every_launch_failure_renders_as_a_leading_row` — six `run_wired` runs
  against a scratch `herdr` failing at each of the six points, each driven by a real `a`
  keypress, each asserting the first interior row and the argv log. Run (f) asserts the refusal
  sentence in full at 120 columns and an **empty** log.
- [ ] 8.2 CHANGE: Write `g_with_no_agent_renders_the_same_buffer`,
  `every_unreachable_path_yields_no_agents` (four failing stubs plus a reachable control),
  `unreachable_socket_renders_no_badge_column`,
  `out_of_scope_and_worktree_agents_are_invisible` (absent cwd, foreign cwd, `.worktrees` cwd,
  plus an in-scope control).
- [ ] 8.3 CHANGE: Write `a_failed_cli_cycle_keeps_the_file_numbers` (root disagreement and
  non-zero exit) and `a_watch_failure_keeps_the_loop_drawing` (unwatchable root, and
  `openspec/` removed mid-run, including that `r` still forces a refresh).
- [ ] 8.4 CHECK: Plant **one defect per test** — eight tests, eight plants. Enumerate them in
  the task before running: badge the out-of-scope agent; return a non-empty `agents` on an
  unreachable poll; drop the launch problem before rendering; make `g` issue a focus call;
  swallow the watcher's reason; let a failed CLI cycle blank the progress pair; and so on.
  Record each in `notes/planted-defects.md`.
- [ ] 8.5 CHECK: Each of the three whole-buffer-equality tests
  (`g_with_no_agent_renders_the_same_buffer`, `unreachable_socket_renders_no_badge_column`,
  `out_of_scope_and_worktree_agents_are_invisible`) names its **discriminating control** in the
  task, so an equality satisfied by two empty buffers is impossible.
- [ ] 8.6 CHECK: No test spawned the real `openspec` or `herdr`; `git diff --name-only` names
  only this group's intended files.
- [ ] 8.7 Run `make check`. Commit.

## 9. Proofs: markdown, state, and the one-shot commands
<!-- kind: operational -->

- [ ] 9.1 CHANGE: Write `unmodelled_constructs_render_as_source` — footnote reference and
  definition, strikethrough, GFM table row, and task-list item, each on a non-tracked tab, at
  58 and 78 columns, asserting rendered line count equals source line count; plus the
  tracked-tab carve-out.
- [ ] 9.2 CHANGE: Write `reading_an_unusable_mapping_writes_nothing`,
  `recording_drops_what_the_parse_could_not_recover` (raw bytes read back), and
  `recording_over_a_clean_mapping_preserves_every_entry`.
- [ ] 9.3 CHANGE: Write `a_duplicate_artifact_id_parses_and_renders`, driven from YAML bytes
  through `schema::parse` rather than from a hand-built `Schema`.
- [ ] 9.4 CHANGE: Extend `tests/cli.rs` for rows 40–43: replace the tautological
  `err.to_lowercase().contains("herdr")` assertion (`src/open.rs:420`) with two separate
  assertions — the variable name, and the "must be invoked from Herdr" clause — and add the
  missing **empty argv log** assertion by putting a logging stub `herdr` first on `PATH`.
- [ ] 9.5 CHECK: Plant one defect per test. For 9.4 the plant is a build in which
  `open::run_from_env` issues a `pane list` call **before** the workspace-id check, so the
  stub's log is non-empty — a stub that merely succeeds leaves the log assertion green and
  proves nothing. Record each in `notes/planted-defects.md`.
- [ ] 9.6 CHECK: `git diff --name-only` names only this group's intended files. Run
  `make check`. Commit.

## 10. Remove the vestigial clippy allow
<!-- kind: refactor -->
<!-- Separated from group 9 because it is a `src/` edit with no test of its own; `HANDOFF.md`
     asks the next change touching `src/changes.rs` to make it, and this is the last one. -->

- [ ] 10.1 CHARACTERIZE: `cargo clippy --all-targets --all-features -- -D warnings` is green at
  HEAD, and `cargo test --all-features changes::` is green — the behaviour this refactor must
  not disturb.
- [ ] 10.2 REFACTOR: Delete the single `#[allow(clippy::too_many_arguments)]` in
  `src/changes.rs`. `build_change` takes seven parameters and the lint fires at eight
  (design.md → Decision 13); do not cite seven as the constraint anywhere.
- [ ] 10.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors, no
  `#[allow]` remains anywhere: `grep -rn '#\[allow(' src/ tests/` is empty. Commit.

## 11. The coverage map and its checker
<!-- kind: behavior -->

- [ ] 11.1 RED: Write `tests/degraded_coverage.rs` with `every_table_row_has_a_proof`,
  `every_row_carries_a_verdict`, and `an_empty_table_fails_the_floor`. RED at HEAD: neither the
  target nor `tests/degraded-coverage.toml` exists.
- [ ] 11.2 GREEN: Write the parser (table between `## Degraded states` and `### No terminal is
  not a degraded state`, first pipe cell trimmed, header and separator skipped — the same parse
  as 1.1) and the six failure conditions in `specs/degraded-coverage/spec.md`. Condition 5
  checks the named function's **own body** for `TestBackend`, not its file: only three of the
  eleven files under `src/ui/` name it, and a file-level rule is both too coarse there and
  unsatisfiable for a proof belonging in `detail.rs` or `markdown.rs`.
- [ ] 11.3 GREEN: Write `tests/degraded-coverage.toml` — one `[[row]]` per table row with
  `condition`, `tier` (`view` | `outer` | `unit` | `integration`), `proof`, `verdict`, `why`.
  Every `proof` name comes from a test that now exists.
- [ ] 11.4 CHECK: Plant each of the six failure conditions in turn — an extra `SPEC.md` row; a
  bad `proof` identifier; a one-character reworded `condition`; a `view` tier pointed at a
  function in `src/changes.rs`; a `view` tier pointed at a non-rendering function *inside*
  `src/ui/view.rs`; a table cut to its header — confirm each fires with its own message, and
  remove each plant. Plants against `SPEC.md` are made in the working tree and reverted;
  `git diff --exit-code SPEC.md` afterwards is the proof none survived.
- [ ] 11.5 CHECK: Plant a seventh — a `verdict` of `probably-fine` — and confirm
  `every_row_carries_a_verdict` fires.
- [ ] 11.6 Run `cargo test --all-features --test degraded_coverage` and `make check`. Commit.

## 12. Extract the standing gates into `scripts/gates/`
<!-- kind: operational -->

- [ ] 12.1 CHECK: Confirm each gate named in `specs/quality-gates/spec.md` is green at its
  measured floor before extraction (0.4–0.6 recorded them; `WIRED` went green in group 2). A
  gate red at extraction time is reported, not extracted.
- [ ] 12.2 CHANGE: Copy each remaining gate into `scripts/gates/<lowercase-name>.sh` (and
  `gate-mech1.py`), editing **only** each floor default to its measured value, so a bare
  invocation is an invocation at the right floor. `wired.sh` already landed in group 2. Record
  any other edit and why.
- [ ] 12.3 CHANGE: Extract `OPENSPEC-UNTOUCHED`'s two `git ls-files` legs as
  `scripts/gates/openspec-untouched.sh`, leaving the `BASE`-diff leg as a per-change
  invocation (design.md → Decision 9). It is the only mechanical guard on the PRD non-goal
  that nothing writes inside `openspec/`.
- [ ] 12.4 CHANGE: Set `READONLY-UI`'s default `EXTRA` to the five files measured in 0.6, and
  `AGENTSEAM`/`LAUNCHSEAM`'s default `ALLOWED` to the five-file list, so neither runs weaker
  bare than `plugin-actions` ran it. Fix `NORAW-GREP`'s hardcoded `16` to its measured 28.
- [ ] 12.5 CHANGE: Raise each test-count floor to measured-plus-enumerated-new, and note the
  gates count `#[test]` functions **per file**, so the arithmetic is per file rather than per
  task group: `WIDTHS_MIN` = 94 + the view tests groups 4–9 add; `LIST_MIN` = 29 + those added
  to `src/ui/list.rs`; `MD_MIN` = 24 + those added to `src/ui/markdown.rs`; `TASK_MIN` = 16 +
  those added to `src/ui/tasks.rs`; `DETAIL_MIN` = 23 + those added to `src/ui/detail.rs`.
  Enumerate the new tests by name and file, show each sum, and re-measure — using the
  re-measured value only if it is not lower.
- [ ] 12.6 CHANGE: Close the three dependency clauses `spec-purposes` parked: `notify`'s
  `default-features = false` in `deps.sh`; `kqueue` and `kqueue-sys` among `build-graph.sh`'s
  named absences; `notify-debouncer-*` as an absence in one of the two.
- [ ] 12.7 CHANGE: Add each script to the `Makefile`'s `gates` recipe — bare, except
  `LAUNCHSEAM`'s two subjects and `NODEFAULT-UI`'s four type sets. `NODEFAULT-UI`'s lines carry
  each subject's own `SCAN_MIN` from 0.5, because that floor is a property of the subject
  (design.md → Decision 6); no other recipe line carries a floor.
- [ ] 12.8 CHANGE: Extend `tests/ci_workflow.rs` to assert the recipe↔directory correspondence
  in both directions, and that no script exists for `EXTENDED` or `TESTCOUNT`.
- [ ] 12.9 CHECK: Plant a script in `scripts/gates/` the recipe does not name — confirm 12.8
  fires; plant a recipe line naming a file that does not exist — confirm it fires; remove both.
- [ ] 12.10 VERIFY: `make gates` exits 0, every script prints an `OK` line, and
  `time make gates` completes in under **15 seconds** on the reference machine (measured: all
  29 scripts ran in ~3.5 s at planning time, the slowest at 0.58 s). Report rather than accept
  a figure above that.
- [ ] 12.11 VERIFY: `make check` green; `git diff --name-only` names no file outside this
  group's intended set. Commit.

## 13. Every gate proved able to fail
<!-- kind: operational -->

- [ ] 13.1 CHECK: For **every** script under `scripts/gates/` — the two that landed with
  `spec-purposes`, `wired.sh`, and everything group 12 added — remove the file its **positive
  control** names and confirm the gate fails on the control rather than passing vacuously;
  restore it. This is the uniform, mechanical proof that a grep gate can fail, and it applies
  to all of them.
- [ ] 13.2 CHECK: For every gate carrying a count floor, run it once with the floor one above
  measured and confirm it exits non-zero — proved at planning time to fire. The floor is
  load-bearing, not decorative.
- [ ] 13.3 CHECK: For every gate, plant the **subject** defect it exists to catch — a spawn API
  in `src/ui/list.rs` for `NOSPAWN-GREP`, a `ratatui` type in `src/ui/markdown.rs` for
  `MDSEAM`, and so on. About seven of the scripts record their measured plant in their own
  header; for the rest, derive one, run it, and **write it into the script's header** so the
  next reader can re-run the proof rather than taking it on trust.
- [ ] 13.4 VERIFY: `git status --porcelain` is empty and `git diff $BASE --stat` names no file
  outside this change's intended set. No planted defect survived.
- [ ] 13.5 VERIFY: `make check` green. Commit.

## 14. `SPEC.md` corrections
<!-- kind: operational -->

- [ ] 14.1 CHANGE: Apply the seven corrections in design.md → Decision 12, each with its
  measurement recorded in `notes/audit.md` beside the row.
- [ ] 14.2 CHANGE: Apply an eighth: row 30's "recorded pending a change that reads `herdr
  worktree list`" is a forward reference to a change that will not exist. Restate it as a
  standing, accepted limitation.
- [ ] 14.3 CHANGE: Rewrite the List view paragraph recording the third-column collision so it
  states the resolution — the indicator lives in the detail region — rather than leaving a plan
  open. Record the accepted trade-off: below 100 columns the list and detail are separate
  routes, so a per-change reason costs one keypress to reach.
- [ ] 14.4 CHECK: No correction rewords a **condition** column without the coverage map being
  updated in the same commit: re-run `cargo test --all-features --test degraded_coverage` after
  14.1 and fix any orphan in the map rather than reverting the correction.
- [ ] 14.5 CHECK: Nothing wrote inside `openspec/`. Run `sh scripts/gates/openspec-untouched.sh`,
  and the `BASE`-diff leg as `CHANGE=degraded-states BASE=<the SHA recorded in 0.1> sh
  $C/OPENSPEC-UNTOUCHED.sh` — with the recorded `BASE`, never the current `HEAD`, and with
  `CHANGE` set, since the script's default names an archived change that no longer exists.
  `SPEC.md` is at the repository root and outside that check; state so.
- [ ] 14.6 VERIFY: `make check` green. Commit.

## 15. Change Review
<!-- kind: operational -->

- [ ] 15.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against
  proposal.md, every spec scenario, design.md, tasks.md, and the diff against the recorded
  `BASE`. Instruct it to write findings to a scratchpad file incrementally; two reviewer rounds
  have been lost to `529 Overloaded` on this project.
- [ ] 15.2 CHECK: Point the reviewer at the concentration points that bind here: a view test
  that would pass against a pane rendering nothing; a whole-buffer-equality test with no
  discriminating control; a gate extracted with a floor below its measured value; a coverage
  map entry whose `tier` is cheaper than the row's wording; a plant that could not redden the
  test it was written for; and any row this change repaired without naming the earlier change
  that owed it.
- [ ] 15.3 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
  one-line reason, note SUGGESTIONs, re-run affected tests.
- [ ] 15.4 VERIFY: No blocking or unowned finding remains. Commit.

## 16. Documentation
<!-- kind: operational -->

- [ ] 16.1 Rewrite in `AGENTS.md` → Quality gates (audience: every future session) — the table
  names five gates and describes `make gates` as two scripts. Replace that with the extracted
  set, the rule that a gate's floor is its own default so a bare invocation is correct, the one
  stated exception (a multi-subject gate's per-subject floor), and the two exclusions with
  their reasons. Net: replaces the two-script paragraph rather than adding beside it.
- [ ] 16.2 Add in `AGENTS.md` → Further invariants (audience: every future session) — one rule,
  ≤5 lines: `SPEC.md`'s degraded-states table is machine-bound to named tests in
  `tests/degraded-coverage.toml`, so a row added or reworded without a proof fails `make
  check`.
- [ ] 16.3 Rewrite in `AGENTS.md` → Current repo state (audience: every future session) — one
  sentence each for the `file mode` badge, the per-change problem lines, and `Startup`'s
  injected probe bindings. Delete any sentence this change makes false rather than appending.
- [ ] 16.4 Rewrite `openspec/IMPLEMENTATION-ORDER.md`'s `degraded-states` row (audience: anyone
  reading the roadmap afterwards) — it says "Nothing here should be new behaviour", which the
  badge, the detail-region problem lines, and the gate extraction make false. State what the
  change actually did. `openspec/config.yaml` → `operations.archive` already requires this at
  archive time; doing it here means the archived row is right on the first read.
- [ ] 16.5 Rewrite `HANDOFF.md` (audience: the next session, and the user) — the project is
  complete when this change archives. Replace the two open items (`WIRED` red; the unextracted
  gates) and the three parked dependency clauses with their resolutions, remove the "next
  action" pointing at Phase 5, and state plainly what is and is not left. Delete what is now
  false; this file has grown across the whole project and a closing pass that only adds would
  leave it unreadable.
- [ ] 16.6 CHECK: Confirm `README.md` needs no edit — its two deferred rows were closed by
  their own changes. Fix any still-stale claim here; this is the last change that can.

## 17. Lint & Verify
<!-- kind: operational -->

- [ ] 17.1 CHECK: Inspect the intended verification commands and the tiers this change
  affected — unit, view, outer, integration, source, and meta.
- [ ] 17.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 17.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors, and
  no `#[allow]` was added anywhere.
- [ ] 17.4 VERIFY: `make gates` — every script exits 0.
- [ ] 17.5 VERIFY: `cargo test --all-features` — green.
- [ ] 17.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — report the **line** figure and
  confirm it did not fall below the 0.2 baseline. If it did, add tests; the floor is never
  lowered and no exclusion is ever added.
- [ ] 17.7 VERIFY: `make check` — the single gate, green.
- [ ] 17.8 VERIFY: `openspec validate degraded-states --strict`, and
  `openspec validate --specs --strict` still passing at 40/40 — `spec-purposes` made that pass
  and this change must not regress it. (`export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$PATH"`.)
- [ ] 17.9 VERIFY: Every commit in the change is signed, and the sweep is **not vacuous** — a
  loop over zero commits reports success:

  ```sh
  n=$(git rev-list --count $BASE..HEAD); echo "commits: $n"; [ "$n" -gt 0 ] || echo "VACUOUS"
  for s in $(git rev-list $BASE..HEAD); do
    git cat-file commit "$s" | grep -q '^gpgsig' || echo "UNSIGNED $s"
  done
  ```

  Expect a commit count above zero and no `UNSIGNED` line. Never use `%G?`:
  `gpg.ssh.allowedSignersFile` is unset and it reports `N` for signed commits. Never work
  around a signing failure with `--no-gpg-sign` — a failure means the vault is locked; stop and
  surface it.
