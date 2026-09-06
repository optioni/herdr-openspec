## Context

`SPEC.md` → Degraded states holds **44 rows**. It is the plugin's never-fail-closed contract
and `PRD.md` → Success criteria repeats its opening sentence. It has grown across every phase
— `plugin-actions` alone added six rows plus the "### No terminal is not a degraded state"
carve-out — and no change has ever checked it as a whole.

A four-way audit of the table against the running plugin was run at HEAD (`89cb3b2`) before
this change was planned. Its findings, in the shape the roadmap asked for:

| Verdict | Rows | Meaning |
|---|---|---|
| **confirmed** — behaviour real, proof already at the row's own tier | 16 | Nothing to do but bind the proof to the row |
| **unproven at its own tier** — behaviour real, proof only below it | 21 | A test is missing, not behaviour |
| **spec-corrected** — behaviour real, the row's *wording* is measurably wrong | 5 | `SPEC.md` is what changes |
| **repaired** — the plugin does not honour the row | 1 | Named against the change that should have shipped it |
| **implemented** — new behaviour the roadmap assigns to this change | 1 | The `file mode` badge, row 2 |

Rows, by verdict, so the tally is checkable rather than asserted:

- **confirmed (16):** 1, 6, 7, 8, 9, 10, 11, 13, 14, 15, 26, 29, 35, 39, 41, 42
- **unproven at its own tier (21):** 3, 4, 5, 12, 16, 17, 19, 20, 21, 22, 24, 25, 27, 30, 31,
  32, 33, 34, 38, 40, 44

Row 31 sits with 27, not with the confirmed rows, and the correction is worth stating: its
wording ("named in `BinResolution::problems` rather than being silent") is satisfied at the
value level, and a unit test proves it — but `cli::worker_cli` drops that vector, so nothing
downstream can read it. That is the same true-but-unobservable state row 27 is in, and
classifying one as confirmed and the other as unproven would have been the plan contradicting
itself. Neither is a *repair*: no earlier change is in breach of its own wording.
- **spec-corrected (5):** 18, 28, 36, 37, 43
- **repaired (1):** 23
- **implemented (1):** 2

Row numbers are positions in the table as it stands at `89cb3b2`, counted from the first data
row. Task group 1 re-derives them against the tree rather than trusting this list.

**The audit found one row's behaviour genuinely missing, and this design flags it rather than
folding it in silently.** Row 23 — "`state::record` fails after a successful `agent start` →
the record failure is recorded as its own problem **alongside** the successful launch" — is not
honoured: `launch::Outcome::problem` is a single `Option<String>`, and
`src/launch.rs`'s `match cli.run(&prompt_refs) { Ok(_) => record_problem, Err(err) =>
Some(herdr_reason(&err)) }` **discards the record's reason** whenever the prompt also fails.
The mapping file is wrong and the pane says nothing about it. **`agent-launch` should have
shipped this**; its own spec even names the pair. It is repaired here because there is no
later change.

Two further findings sit next to it and are *not* behaviour gaps, but are recorded so a
reader can tell the categories apart:

- **Row 2's `file mode` badge does not exist**, and that is correct — the roadmap row assigns
  it to this change. It is the change's one piece of genuinely new rendered behaviour. It does
  **not** depend on `BinResolution::problems`: `cli::worker_cli_from_env` already returns an
  `Option`, so found-ness is available today. Surfacing `problems` is an adjacent fix for rows
  27 and 31, not a prerequisite for the badge.
- **`Change::problems` and `Config::problems` are populated and never read.** Four rows say
  "the reason is named" and one says "`Config::problems` names each fallback". Read literally,
  all five are satisfied: the reason *is* named, on a vector. Read as a reader would, none is
  — nothing renders them. No earlier change is in breach of its own wording, so these are
  counted as *unproven at their own tier*, and this change makes them observable. The
  distinction survives into `tasks.md`: an audit task that confirms a row is task group 1, and
  an implementation task that creates one is task group 3 or later.

Two items folded in from `HANDOFF.md`, both last-chance:

- **`WIRED` is red at HEAD**, reproduced (leg 2: `pub fn run()` holds a `match`). It went red
  because `plugin-actions` put the cwd-resolution branch into `run`'s own body, and it stayed
  red for a change because `WIRED` lives outside `make check`.
- **Twenty-eight gates sit outside `make check`.** Measured from the repository root at their
  recorded floors, **24 of 26 standing gates are green**; `WIRED` is red, and `EXTENDED` is a
  per-change ratchet that cannot be green on an unmodified tree. The blocker to extraction was
  never hermeticity — none invokes `cargo`, none uses a GNU-only flag, all complete in under a
  second.

## Goals / Non-Goals

**Goals:**

- Every one of the 44 rows bound to a named proving test, at the tier its own wording claims,
  and the binding itself checked by `make check` so it cannot rot.
- The `file mode` header badge, and the plumbing that lets the pane know it is in file mode.
- `Change::problems` and `Config::problems` rendered, so five rows stop being true-but-invisible.
- Row 23 repaired; `WIRED` repaired and, with 24 siblings, brought under `make check`.
- Eight `SPEC.md` corrections, each backed by a measurement.

**Non-Goals:**

- **No new degraded condition.** Every row exists; this change proves them.
- **No list-region row-grammar change** (Decision 3).
- **No `Change` type change.** `from_files` and `from_cli` both keep producing the same type
  and neither gains or loses a field, so the two-producer agreement is untouched.
- **No new dependency**, no process spawn outside `cli`, no I/O in a view, no write inside
  `openspec/`, no `#[allow]` added.
- Not extracting `EXTENDED`, `OPENSPEC-UNTOUCHED`, or `TESTCOUNT` (Decision 9).
- Not repairing `state::record`'s rewrite semantics (Decision 8) — `SPEC.md` is corrected
  instead, with the reason recorded.

## Boundaries

| Module | What changes | Pattern followed |
|---|---|---|
| `src/ui/mod.rs` | `startup_dir` added; `run`'s branch removed; `start_collaborators` folds config and binary problems into `problems` and reports `file_mode`; `run_wired` sets it | `run`/`run_wired` split is `agent-poller`'s; the injected-closure fallback is `AGENTS.md`'s environment-lookup rule |
| `src/ui/app.rs` | `Dashboard` gains `file_mode: bool` | Plain data, no `Default`, every site names it — `dashboard-loop`'s existing rule |
| `src/ui/view.rs` | header draws the dim badge and re-bases the shortening arithmetic | `responsive-layout`'s existing right-aligned header |
| `src/ui/detail.rs` | `content_lines` prepends the change's own problems | Byte-for-byte the `! `-prefixed grammar it already uses for `Detail::problems` |
| `src/cli.rs` | `worker_cli` surrenders `BinResolution::problems` rather than dropping it | Still spawns only through the seam; no parsing added |
| `src/launch.rs` | `Outcome::problem` → `problems: Vec<String>` | `Dashboard::launch.problems` is already a `Vec` |
| `src/agents.rs`, `src/changes.rs`, `src/schema.rs`, `src/state.rs`, `src/ui/markdown.rs` | tests only, plus doc comments stating measured semantics | — |
| `scripts/gates/` | 25 new scripts | `deps.sh` / `build-graph.sh`, landed by `spec-purposes` |
| `Makefile`, `tests/ci_workflow.rs` | `gates` recipe grows; the recipe↔directory correspondence is asserted both ways | `every_gate_the_makefile_composes_runs_in_ci` |
| `tests/degraded_coverage.rs`, `tests/degraded-coverage.toml` | new | `tests/spec_purposes.rs`, landed by `spec-purposes` |

**No process spawn is added outside `cli`.** `startup_dir` reads the injected environment
lookup and calls an injected closure; neither names a spawn API. `Command::new` stays confined
to `src/cli.rs` and `NOSPAWN-GREP` — now a repository file — enforces it.

**No view gains I/O.** `content_lines` stays a pure total function of its three arguments;
the header badge reads a `bool` already on the `Dashboard`. The eight pure files under
`src/ui/` are unchanged in that respect and `NOIO-VIEW` proves it.

## Contracts

Every interface changed here is **internal to this crate**; the plugin's external contracts —
the Herdr manifest, `config.toml`'s format, and the keymap — are untouched, so the change is
**not BREAKING** under this project's rule.

- `launch::Outcome`: `problem: Option<String>` → `problems: Vec<String>`. Additive in
  meaning, source-breaking at three call sites, all inside this crate
  (`src/launch.rs`'s worker, `src/ui/driver.rs`'s drain, and the tests). Error surface
  unchanged: the strings are the same strings, and a second one now survives where it was
  dropped.
- `cli::worker_cli`: returns the resolution's problems alongside the handle instead of
  consuming them. One production caller (`ui::start_collaborators`) and its tests.
- `ui::app::Dashboard`: thirteenth field. Every construction site names it — `NODEFAULT-UI`
  and the compile-time destructuring companion both fail otherwise, which is the point.
- `ui::startup_dir`: new, `pub(crate)`. One production caller, `ui::run`.

## Persistence and Rollout

- **Migration:** none. No stored format changes.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. `detail.loaded`'s `(change directory, tab)` key is unchanged;
  the extra leading lines are computed per draw, not cached.
- **Index rebuild:** none.
- **Authorization:** none — the plugin has no actors and no permissions.
- **Observability:** the change *is* observability: five previously unread problem vectors
  reach the pane. Nothing is logged to a file and no telemetry is added.
- **Deployment:** none beyond `make build`. `agent-names.toml` written by an older build stays
  readable; nothing about its format changes.

## Test Boundaries

| Dependency | In acceptance test (`run_wired`, view) | In unit tests |
|---|---|---|
| Filesystem (repository tree) | **real** — `testutil::ScratchDir` fixture repositories | **real** scratch trees for `resolve`/`changes`/`tasks`/`schema`/`state`; not touched by pure functions |
| Filesystem (plugin state dir) | **real** scratch directory passed as `Startup::state_dir` | **real** scratch directory |
| Filesystem (plugin config dir) | **replaced** — a `Config` value is constructed directly | **replaced** — `Config` constructed directly |
| `openspec` binary | **replaced** — scratch `#!/bin/sh` program named by `Config::openspec_bin`, logging its argv; the real binary is never spawned | **replaced** — stub `OpenspecCli` returning canned stdout or `CliError` |
| `npm` (fourth probe step) | **replaced** — the injected probe hook, reached through `Startup` (Decision 14); never the real `cli::npm_prefix` | **replaced** — injected hook |
| Herdr socket / `herdr` binary | **replaced** — scratch `#!/bin/sh` program logging argv and returning canned envelopes or failures | **replaced** — stub `HerdrCli` |
| Terminal | **replaced** — ratatui `TestBackend` at 120x20 and 60x20; `EventSource` replaced by `testutil::UntilReady` | not reached |
| Real terminal modes (`enable_raw_mode` etc.) | **never reached** — `NORAW-GREP` forbids any file but `src/ui/terminal.rs` naming them, tests included | never reached |
| `notify` filesystem watcher | **real** wherever a scenario names a watcher outcome — the second-CLI-cycle tests, `startup_problems_are_ordered_*`, and `a_watch_failure_keeps_the_loop_drawing`; **replaced** by `watch::none()` elsewhere | **replaced** |
| Process environment | **replaced** — injected `&dyn Fn(&str) -> Option<String>`; no test calls `std::env::set_var` | **replaced** |
| Process working directory | **replaced** — `startup_dir`'s fallback is an injected closure, so no test changes the real cwd | **replaced** |
| `herdr-openspec` binary itself (`open`/`open-tab`) | **real process** spawned by `tests/cli.rs`, with a stub `herdr` first on `PATH` | n/a |
| Git | **not touched by any test.** It *is* driven for real by the plan's own operational checks — `BASE` capture, `OPENSPEC-UNTOUCHED`, the signing sweep — which are commands the implementer runs, not collaborators a test replaces | not touched |
| Network | **never** | never |

No task may introduce a boundary this table does not name. In particular: **no test may open
a real Herdr socket, spawn the real `openspec` binary, or put the developer's terminal into
raw mode.**

## Test Strategy

Tiers, and the command that runs each:

| Tier | What it is | Command |
|---|---|---|
| `unit` | pure functions and edge modules, no TUI | `cargo test --all-features <filter>` |
| `view` | rendered into a `TestBackend` at **both** mandated widths | `cargo test --all-features <filter>` |
| `outer` | the real `ui::run_wired` driven end to end over scratch programs | `cargo test --all-features <filter>` |
| `integration` | the real binary spawned as a process | `cargo test --all-features --test cli` |
| `source` | a `scripts/gates/*.sh` grep gate | `sh scripts/gates/<name>.sh` |
| `meta` | a `cargo test` target reading repository files | `cargo test --all-features --test <name>` |

**This change takes the outer-loop acceptance test, and it is load-bearing.** Ten of its
scenarios drive the real `ui::run_wired` rather than the components it composes — the six
launch failures, `g` on an unattributed change, the two startup-problem scenarios, and the
watcher pair. `live-refresh` nearly shipped a permanently inert live tier with every test
green because its tests drove the seams directly; a degraded-state test that only drives a
widget proves the widget, not that the state is reachable in the running plugin.

**Every verification command in this change must be able to fail**, and each is proved so by
planting the defect it guards, running it red, removing the plant, and running it green —
task group 13, with the tree clean afterwards.

### Verification matrix

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| responsive-layout :: A path that fits is right-aligned whole at both widths | existing `ui::view` test re-run unchanged | view | terminal replaced (TestBackend) | `cargo test --all-features ui::view::tests::header` |
| responsive-layout :: A path too long for the narrow header is shortened from the left | existing test re-run unchanged | view | terminal replaced | `cargo test --all-features ui::view::tests::header` |
| responsive-layout :: A header too narrow for any path shows only the label | existing test re-run unchanged | view | terminal replaced | `cargo test --all-features ui::view::tests::header` |
| responsive-layout :: No repository found is named in the header at both widths | existing test re-run unchanged | view | terminal replaced | `cargo test --all-features ui::view::tests::header` |
| responsive-layout :: The badge is drawn dim after the label at both widths | new `file_mode_badge_is_dim_after_the_label` — asserts cells **and** `Modifier::DIM` | view | terminal replaced | `cargo test --all-features file_mode_badge` |
| responsive-layout :: A false flag renders the header that landed before this change | new `no_badge_is_byte_identical_to_the_landed_header` — whole-buffer equality both widths | view | terminal replaced | `cargo test --all-features no_badge_is_byte_identical` |
| responsive-layout :: The badge takes its columns from the path, not from the label | new `badge_rebases_the_shortening_arithmetic` — column spans at 60 and 120 | view | terminal replaced | `cargo test --all-features badge_rebases` |
| responsive-layout :: A header too narrow for the badge drops it whole | new `badge_drops_whole_below_eighteen_columns` — 17/18/60 | view | terminal replaced | `cargo test --all-features badge_drops_whole` |
| dashboard-loop :: `file_mode` is set by the composition root and by nothing else | new `load_never_claims_file_mode` (unit) + `run_wired_sets_file_mode_from_the_probe` (outer) | unit + outer | fs real, `openspec` replaced (scratch program) | `cargo test --all-features file_mode` |
| dashboard-loop :: The thirteenth field is named at every construction site | `NODEFAULT-UI` at its measured `SCAN_MIN`, plus the compile-time destructuring companion | source | none | `sh scripts/gates/nodefault-ui.sh` |
| dashboard-loop :: `Dashboard` has no `Default` and no site elides a field | existing check, now a repository file | source | none | `sh scripts/gates/nodefault-ui.sh` |
| dashboard-loop :: The pure view files name no I/O API | existing check, now a repository file | source | none | `sh scripts/gates/noio-view.sh` |
| dashboard-loop :: The shell never names the CLI seam | existing check, now a repository file | source | none | `sh scripts/gates/nocli-shell.sh` |
| dashboard-loop :: Change literals live only in the gated file | existing check, now a repository file | source | none | `sh scripts/gates/nolit-change.sh` |
| dashboard-loop :: The render path names no channel, thread, lock, or clock | existing check, now a repository file | source | none | `sh scripts/gates/noblock.sh` |
| dashboard-loop :: No test sleeps and then asserts something has already happened | existing check, now a repository file | source | none | `sh scripts/gates/nosleep.sh` |
| live-updates :: `Refresh` has no `Default` and no site elides a field | the `Refresh` type-set run of `NODEFAULT-UI`, at that set's own measured floor — the bare invocation's default `TYPES` never examines `Refresh` | source | none | `TYPES='Refresh' SCAN_MIN=<measured> sh scripts/gates/nodefault-ui.sh` |
| openspec-binary :: A configured path that cannot be used reaches the list as a problem row | new `unusable_openspec_bin_renders_as_a_problem_row` driving `run_wired` | outer + view | fs real, `openspec` replaced, `herdr` replaced, terminal replaced | `cargo test --all-features unusable_openspec_bin` |
| openspec-binary :: No binary anywhere is reported as file mode rather than as an error | new `no_binary_is_file_mode_not_an_error` | outer | fs real, `openspec` absent, terminal replaced | `cargo test --all-features no_binary_is_file_mode` |
| openspec-binary :: A resolved binary contributes nothing | new `a_resolved_binary_adds_no_problem_and_no_badge` — whole-buffer equality | outer + view | fs real, `openspec` replaced | `cargo test --all-features a_resolved_binary_adds_no_problem` |
| plugin-config :: A malformed key renders as a leading problem row at both widths | new `config_fallback_renders_as_a_leading_problem_row` | outer + view | fs real, terminal replaced | `cargo test --all-features config_fallback_renders` |
| plugin-config :: Configuration problems precede binary and watcher problems | new `startup_problems_are_ordered_config_then_binary_then_watcher` | outer + view | fs real, `notify` real (unwatchable root), terminal replaced | `cargo test --all-features startup_problems_are_ordered` |
| plugin-config :: A clean configuration contributes nothing | new `a_clean_config_adds_no_row` — whole-buffer equality | outer + view | fs real, terminal replaced | `cargo test --all-features a_clean_config_adds_no_row` |
| artifact-content :: A change whose schema will not parse names the reason in the detail region | new `change_problems_render_above_the_content` | view | terminal replaced | `cargo test --all-features change_problems_render_above` |
| artifact-content :: A change problem and a tab problem are both shown, change first | new `change_problem_precedes_tab_problem` | view | terminal replaced | `cargo test --all-features change_problem_precedes` |
| artifact-content :: No selected change contributes no lines | new `no_selected_change_contributes_no_lines` — empty vec + blank region | unit + view | terminal replaced | `cargo test --all-features no_selected_change_contributes` |
| artifact-content :: The line count drives the scroll clamp | new `change_problems_are_inside_the_scrolled_region` — 20 entries, 40 `j` presses | view | terminal replaced | `cargo test --all-features change_problems_are_inside_the_scrolled` |
| agent-poller :: `startup_dir` prefers the workspace cwd and falls back when there is none | new `startup_dir_prefers_the_workspace_cwd` — counter-instrumented closure | unit | env replaced, cwd replaced | `cargo test --all-features startup_dir_prefers` |
| agent-poller :: `startup_dir` propagates a failing fallback rather than panicking | new `startup_dir_propagates_a_failing_fallback` | unit | env replaced, cwd replaced | `cargo test --all-features startup_dir_propagates` |
| agent-poller :: A Herdr context with no workspace cwd is the fallback case, not a failure | new `startup_dir_treats_a_contextless_environment_as_fallback` | unit | env replaced, cwd replaced | `cargo test --all-features startup_dir_treats` |
| agent-poller :: The residue left in `run` is small enough to read | `WIRED`, extracted, with the new `startup_dir(` leg | source | none | `sh scripts/gates/wired.sh` |
| agent-poller :: The real wiring polls a scratch Herdr and adopts what it says | existing outer-loop test re-run | outer | fs real, both binaries replaced, `notify` real | `cargo test --all-features ui::tests::wiring` |
| agent-poller :: A polled agent reaches a rendered badge and a rendered count | existing test re-run | outer + view | as above | `cargo test --all-features ui::tests::wiring` |
| agent-poller :: The wiring test fails when the poller is replaced by the inert double | existing test re-run | outer | as above | `cargo test --all-features ui::tests::wiring` |
| agent-poller :: An unreachable scratch Herdr leaves the pane a working standalone TUI | existing test re-run | outer + view | as above | `cargo test --all-features ui::tests::wiring` |
| agent-poller :: A pane with no OpenSpec repository still polls for agents | existing test re-run | outer | as above | `cargo test --all-features ui::tests::wiring` |
| agent-launch :: A failed split leaves nothing behind | existing test, updated for `problems` | unit | `herdr` replaced, state dir real | `cargo test --all-features launch::tests` |
| agent-launch :: A failed start leaves the pane and names it | existing test, updated for `problems` | unit | as above | `cargo test --all-features launch::tests` |
| agent-launch :: A failed prompt leaves a running, un-prompted agent that is still attributable | existing test, updated for `problems` | unit | as above | `cargo test --all-features launch::tests` |
| agent-launch :: A failed recording does not undo a successful start | existing test, updated for `problems` | unit | as above | `cargo test --all-features launch::tests` |
| agent-launch :: A failed recording and a failed prompt are both reported | **new** `a_failed_record_and_a_failed_prompt_are_both_reported` (unit) + a view row-pair assertion | unit + view | `herdr` replaced, state dir real, terminal replaced | `cargo test --all-features a_failed_record_and_a_failed_prompt` |
| agent-launch :: A success clears both entries | new `a_success_clears_both_entries` | outer + view | `herdr` replaced, terminal replaced | `cargo test --all-features a_success_clears_both_entries` |
| agent-list :: Every unreachable path yields an empty agent list | new `every_unreachable_path_yields_no_agents` — four failing stubs plus a reachable control | unit | `herdr` replaced | `cargo test --all-features every_unreachable_path` |
| agent-list :: An unreachable socket renders a list with no badge column at both widths | new `unreachable_socket_renders_no_badge_column` — whole-buffer equality against the reachable-empty control | view | terminal replaced | `cargo test --all-features unreachable_socket_renders_no_badge` |
| plugin-state :: Reading an unusable mapping leaves the file byte-identical | new `reading_an_unusable_mapping_writes_nothing` — snapshot around the call | unit | state dir real | `cargo test --all-features reading_an_unusable_mapping` |
| plugin-state :: Recording after an unusable read drops what the parse could not recover | new `recording_drops_what_the_parse_could_not_recover` — raw bytes read back | unit | state dir real | `cargo test --all-features recording_drops_what_the_parse` |
| plugin-state :: Recording over a clean mapping preserves every entry | new `recording_over_a_clean_mapping_preserves_every_entry` | unit | state dir real | `cargo test --all-features recording_over_a_clean_mapping` |
| degraded-coverage :: The map covers the table at HEAD | new `tests/degraded_coverage.rs::every_table_row_has_a_proof` | meta | repository files real | `cargo test --all-features --test degraded_coverage` |
| degraded-coverage :: A row added to SPEC without a proof fails the build | planted-defect run in the working tree, reverted; `git diff --exit-code SPEC.md` proves none survived | meta | repository files real | task group 11 |
| degraded-coverage :: A renamed test fails the binding rather than passing vacuously | planted-defect run, two plants (bad proof name; reworded condition) | meta | repository files real | task group 11 |
| degraded-coverage :: A `view` tier pointing at a test that renders nothing fails | planted-defect run, two plants (a function in `src/changes.rs`; a non-rendering function inside `src/ui/view.rs`) | meta | repository files real | task group 11 |
| degraded-coverage :: An empty table is a failure, not a vacuous pass | new `an_empty_table_fails_the_floor` — parser driven against a fixture string | unit | none | `cargo test --all-features an_empty_table_fails_the_floor` |
| degraded-coverage :: Every row carries one of the five verdicts | new `tests/degraded_coverage.rs::every_row_carries_a_verdict` — reads the `verdict` key in `tests/degraded-coverage.toml`, which archiving does not move | meta | repository files real | `cargo test --all-features --test degraded_coverage` |
| degraded-coverage :: A schema the CLI rejects falls back per change and names the reason | new `a_cli_rejected_schema_names_its_reason_per_change` | view | terminal replaced | `cargo test --all-features a_cli_rejected_schema_names` |
| degraded-coverage :: A schema that is not vendored and one that will not parse both empty the tab bar | new `an_unusable_schema_renders_no_artifacts` — both sub-cases, both widths | view | terminal replaced | `cargo test --all-features an_unusable_schema_renders_no_artifacts` |
| degraded-coverage :: A schema with no tasks artifact renders every tab as markdown and still counts | new `no_tasks_artifact_renders_every_tab_as_markdown` — three tabs × two widths | view | terminal replaced | `cargo test --all-features no_tasks_artifact_renders_every_tab` |
| degraded-coverage :: An unsupported `generates` glob empties one artifact and names why | new `an_unsupported_glob_names_its_reason_and_spares_the_others` | view | terminal replaced | `cargo test --all-features an_unsupported_glob_names_its_reason` |
| degraded-coverage :: A tasks file that cannot be read is zero tasks with a named reason | new `an_unreadable_tasks_file_is_zero_with_a_named_reason` — directory and invalid-UTF-8 fixtures | view | fs real (scratch), terminal replaced | `cargo test --all-features an_unreadable_tasks_file_is_zero` |
| degraded-coverage :: Each of the six launch failures renders as a leading problem row | new `every_launch_failure_renders_as_a_leading_row` — six `run_wired` runs, both widths | outer + view | `herdr` replaced (scratch program), fs real, terminal replaced | `cargo test --all-features every_launch_failure_renders` |
| degraded-coverage :: `g` with no attributed agent changes nothing the pane shows | new `g_with_no_agent_renders_the_same_buffer` — whole-buffer equality + empty argv log | outer + view | `herdr` replaced, terminal replaced | `cargo test --all-features g_with_no_agent_renders_the_same_buffer` |
| degraded-coverage :: A CLI root disagreement and a non-zero exit both leave the file numbers standing | new `a_failed_cli_cycle_keeps_the_file_numbers` | outer + view | `openspec` replaced, fs real, terminal replaced | `cargo test --all-features a_failed_cli_cycle_keeps_the_file_numbers` |
| degraded-coverage :: A watcher failure and a mid-run removal both keep the loop drawing | new `a_watch_failure_keeps_the_loop_drawing` | outer + view | `notify` real, fs real, terminal replaced | `cargo test --all-features a_watch_failure_keeps_the_loop_drawing` |
| degraded-coverage :: An out-of-scope agent and a worktree agent are both invisible | new `out_of_scope_and_worktree_agents_are_invisible` — whole-buffer equality + in-scope control | view | terminal replaced | `cargo test --all-features out_of_scope_and_worktree_agents` |
| degraded-coverage :: A duplicate artifact id is accepted by this crate and stays file-mode | new `a_duplicate_artifact_id_parses_and_renders` — driven from YAML bytes | unit + view | fs real (scratch), terminal replaced | `cargo test --all-features a_duplicate_artifact_id_parses` |
| degraded-coverage :: A footnote, strikethrough, and a table each render as literal source | new `unmodelled_constructs_render_as_source` — four constructs, both markdown widths | view | terminal replaced | `cargo test --all-features unmodelled_constructs_render_as_source` |
| degraded-coverage :: The one-shot commands' degrades are proved by exit status and stderr | new/extended `tests/cli.rs` cases, including the empty-argv-log assertion | integration | real binary spawned, `herdr` replaced by a stub on `PATH` | `cargo test --all-features --test cli` |
| quality-gates :: Both hygiene gates are checked-in files invoked from the Makefile | extended `tests/ci_workflow.rs` — recipe↔directory correspondence, both directions | meta | repository files real | `cargo test --all-features --test ci_workflow` |
| quality-gates :: Each extracted gate's default floor is the measured one | every gate run bare, then re-run with its floor one above measured | source | none | task groups 12 and 13, recorded in `notes/gate-floors.md` |
| quality-gates :: The three excluded gates are named, with reasons, where a reader will meet them | `AGENTS.md` edit plus a `tests/ci_workflow.rs` assertion that no script exists for the three | meta | repository files real | `cargo test --all-features --test ci_workflow` |
| openspec-binary :: An outer test drives a failing probe without touching the machine | new `run_wired_probes_through_the_injected_hook` — fixture env map, `npm` hook returning `None`, plus a resolving control | outer | env replaced, `npm` replaced, `openspec` replaced, terminal replaced | `cargo test --all-features run_wired_probes_through_the_injected_hook` |
| openspec-binary :: The composition root is the one place the real bindings are named | `WIRED`'s name legs, extended with `config::env_lookup(` and `cli::npm_prefix` | source | none | `sh scripts/gates/wired.sh` |
| quality-gates :: The three clauses are checked and each can fail | `deps.sh` gains `notify`'s `default-features`; `build-graph.sh` gains `kqueue`, `kqueue-sys`, `notify-debouncer-*`; each planted | source | none | `make gates`, then task group 13 |
| quality-gates :: A gate that cannot fail is itself a failure | a positive-control removal, a floor-plus-one run, and a subject plant for every script under `scripts/gates/` | source | none | task group 13, recorded in `notes/planted-defects.md` |

## Decisions

**Decision 1 — The `file mode` badge goes on the frame header, after the label.**
`SPEC.md` says "a dim `file mode` badge in the header" and the plugin has two headers: the
frame header (`ui::view::render_header`) and the detail region's change header
(`ui::detail::header_row`). A missing binary is a fact about the *process* — true of every
change in the pane — while the change header describes one change, and the table already
gives the per-change equivalent its own row ("Schema unknown to the CLI"). *Alternative:* the
change header, rejected because the badge would then repeat on every change and say nothing
about which one was affected.

**Decision 2 — `file_mode` is a `Dashboard` field, not a fourth `Refresh` field.**
`Refresh`'s three fields are all consumed or replaced as the session runs; `file_mode` is
decided once at startup and never moves. *Alternative:* `Refresh::file_mode`, rejected —
placing it there would have required modifying `live-updates`' definition *and*
`dashboard-loop`'s field count, for a worse fit. *Alternative:* deriving it from
`Change::origin`, rejected — with zero changes, or before the first CLI cycle, "every change
is file-sourced" is indistinguishable from "no binary exists", which is a different row.

**Decision 3 — The per-change problem indicator goes in the detail region, not the list's
third column.** `SPEC.md` → List view records that `degraded-states` "plans a per-change
problem indicator in this same third column on an archived row; the two cannot both occupy it,
and `degraded-states` must place its indicator elsewhere or specify a precedence." This design
takes the first option. A one-column cell can say only *that* something is wrong; the four
rows it would serve all end "**and the reason is named**", and a reason is a path plus an error
string. The detail region already renders exactly that grammar for `Detail::problems`, has the
width for it, and needs no new cell, no new drop rank, and no precedence rule against the agent
badge. *Alternative:* a `!` cell beside the badge with a stated precedence — rejected: it would
have modified `change-rows`' whole row-grammar requirement to add a marker that names nothing,
while leaving the reason as unreadable as it is today.

**Decision 4 — Startup problems join `refresh.problems` rather than becoming a fourth row
source.** Configuration fallbacks and binary-probe fallbacks share `refresh.problems`' exact
lifetime: standing conditions that outlive every reload. Putting them there means the list's
row order is unchanged, `change-rows` needs no delta, and the reader meets them in causal
order — configuration, then probe, then watcher. *Alternative:* a fourth vector rendered above
the other three, rejected as a second mechanism for one lifetime.

**Decision 5 — `WIRED` is repaired with a named `startup_dir` function, not by moving the
branch into `run_wired`.** `HANDOFF.md` records the latter as "the recorded fix", and it is
rejected on measurement: `Startup::cwd` is a `&Path` that every acceptance test and every
construction site already builds, so widening it to `Option<&Path>` ripples through all of
them, and it would put a `std::env::current_dir()` read inside the one function whose purpose
is to be driveable from a test. `startup_dir(env, fallback)` puts the decision somewhere both
arms are driven, with a fraction of the blast radius. This is **not** the "keyword-free
laundering" `WIRED` leg 2's own documented limit warns about — laundering hides an *untested*
decision from the grep; this moves the decision to a *tested* function and adds a leg
requiring `run`'s body to name it, so a body that drops the call fails too.

**Decision 6 — Gate floors live as each script's own default, invoked bare.** This project has
recorded **five** instances of a gate run without its floor and therefore at a block default.
The durable fix is to make the default *be* the measured floor, so a bare invocation is a
correct invocation. The `Makefile` names only *subjects* — `LAUNCHSEAM`'s two entry points,
`NODEFAULT-UI`'s four type sets — never floors, so no floor exists in two places.
*Alternative:* floors on the `Makefile` line, rejected as a second copy that drifts — with one
stated exception: where a gate runs over several **subjects** and its floor is a property of
the subject rather than of the gate, the floor accompanies its subject on the recipe line.
`NODEFAULT-UI` is the only such gate, and it is not optional there: its span count differs by
an order of magnitude between the `Dashboard` type set and the `Refresh` one, so a single
default passes vacuously for one run or fails legitimately for the other. That is one number
per subject, each still written in exactly one place.
*Alternative:* self-deriving floors from a committed counts fixture, on `build-graph.sh`'s
model — rejected as a change of its own; the design states honestly that a count floor is a
vacuity guard that weakens, never falsifies, as the tree grows, and that each gate's positive
control is what actually proves the search saw its subject.

**Decision 7 — Twenty-five gates are extracted, not a subset.** `HANDOFF.md` records the
opposite intent — "Worth doing as its own change, not smuggled into a hygiene pass" — and this
design overrides it deliberately rather than by oversight. Two things changed since that note:
`WIRED` then went red on `main` inside a single change, demonstrating the note's own thesis
against it; and this is the last row of the roadmap, so "its own change" names a change that
will not exist. The user's brief for this change asked for exactly this judgment to be made
here. Measured: none invokes
`cargo`, none uses a GNU-only flag (`grep -P`, `sed -i`, `stat`, `sort -V`), all run in under
a second, and 24 of 26 are green at their recorded floors today. The only reason not to
extract was cost, and cost is not a reason available to the last change in the roadmap.

**Decision 8 — `state::record`'s rewrite semantics are documented and tested, not repaired.**
Row 28 says "nothing already on disk is lost". True of `read`; false of `record`, which
rewrites the file from the entries the parse recovered. Making `record` refuse to write on a
malformed file would leave a freshly launched agent unattributable for the rest of the
session — a worse failure than losing mappings the plugin itself wrote and will write again.
`SPEC.md` is corrected to state what happens, and a test pins both halves so a later reversal
is a deliberate edit to a failing test.

**Decision 9 — Two gates are not extracted, and one is split.** `EXTENDED` is a per-change
ratchet whose hardcoded pairs belong to `agent-attribution` and are already stale — extracting
it composes a permanently red step into `make check`. `TESTCOUNT` is a shell function other
checks source, not a check. Both are named with their reason in `AGENTS.md`, and
`tests/ci_workflow.rs` asserts no script exists for either, so the exclusion is visible in the
tree.

`OPENSPEC-UNTOUCHED` is **split** rather than excluded, which the first draft of this design
got wrong. Its tracked-diff leg genuinely needs a per-change `BASE`; its two `git ls-files`
legs need nothing and hold on any tree. Extracting those is worth more than it looks: it is the
only mechanical guard on the PRD non-goal that nothing writes inside `openspec/`, and leaving
it wholly outside `make check` would leave that non-goal enforced by prose. The `BASE` leg
stays a per-change invocation, run with `CHANGE=degraded-states` and a `BASE` **captured once**
at task 0.1 — never re-derived as the current `HEAD`, which would compare `HEAD` to a clean
tree and pass over the very writes it exists to catch.

**Decision 10 — The coverage map is keyed on the row's condition column, verbatim.** A
positional index would survive a reworded row silently; a hash would not name what broke.
Keying on the text means a reworded row reports as one uncovered condition plus one orphan
entry, which is exactly the two-sided message a reader needs. The floor is 44 rather than an
equality so a future degraded state can be added.

**Decision 11 — Row 23 is repaired by widening `Outcome`, not by joining two strings.**
`SPEC.md` says the record failure is "recorded as its **own** problem alongside the successful
launch", and `Dashboard::launch.problems` is already a `Vec<String>` rendered one row each.
Joining with `; ` would produce one row claiming to be one problem. The `at most one entry`
claim in `agent-launch`'s spec is restated as **at most two**, which is the true bound.

**Decision 12 — Eight `SPEC.md` corrections, each backed by a measurement.**

| # | Row | Correction | Measurement |
|---|---|---|---|
| 1 | Watcher will not start | "above every other problem row" → "above every change-set problem row, and below a launch problem row" | `ui::list::rows` emits launch → refresh → changes; `a_launch_problem_renders_above_a_watch_problem` already asserts it at both widths |
| 2 | `openspec/` removed while watching | same overstatement, same correction | same |
| 3 | `open`/`open-tab` focus domain error | "carried **verbatim** from Herdr" → carried with `herdr_reason`'s `herdr exited with code <n>: ` prefix | `src/open.rs::herdr_reason` |
| 4 | `agent-names.toml` unusable | "nothing already on disk is lost" → the read is non-destructive; the next successful `record` rewrites from the recovered entries alone | `src/state.rs::record` re-reads and re-serialises |
| 5 | Herdr socket unreachable | "agent column … hidden" → the column is empty because an unreachable snapshot carries no agents; the view applies no rule of its own | `agents::poll_once` returns `agents: Vec::new()` on every `reachable: false` path |
| 6 | `openspec` binary not found | the badge's position, width, dim styling, and drop rule stated, so the row names something checkable | this change |
| 7 | List view prose (`SPEC.md` line ~368) | the recorded third-column collision is resolved: the indicator is placed in the detail region instead, and the paragraph says so rather than leaving a plan open | Decision 3 |
| 8 | An agent in a linked worktree | "recorded pending a change that reads `herdr worktree list`" → a standing, accepted limitation | the roadmap ends here; there is no such change to be pending on |

Correction 7 carries an accepted trade-off, stated rather than glossed: below 100 columns the
list and the detail are separate routes, so a per-change reason costs one keypress to reach.
That is the price of naming the reason at all, against a one-column marker that names
nothing.

**Decision 14 — `Startup` carries the probe's environment lookup and `npm` hook.** Without
this the change's own outer tests are unsound: `start_collaborators` reaches the probe through
`cli::worker_cli_from_env`, which calls `resolve::openspec_bin_from_env`, which reads the real
`PATH` and spawns the real `npm`. Every landed outer test dodges it by naming a *usable*
`openspec_bin` so the chain stops at step one — but the scenarios this change adds exist
precisely to drive the case where nothing resolves. Injecting both bindings through `Startup`
is the rule `AGENTS.md` already states for the environment, applied to the composition that
assembles the probe. *Alternative:* leaving the probe as it is and asserting only on
`file_mode`, rejected — the test would then pass or fail depending on whether the developer has
`openspec` installed, which is the definition of a check that cannot be trusted either way.
*Alternative:* a `#[cfg(test)]` override inside `resolve`, rejected — a production seam that
exists only in test builds is not a seam.

**Decision 13 — The vestigial `#[allow(clippy::too_many_arguments)]` in `src/changes.rs` is
deleted.** `HANDOFF.md` records it as verified removable and asks the next change touching
that file to remove it. This change touches it, and is the last one that can. Verified again
during implementation: with it removed, `cargo clippy --all-targets --all-features -- -D
warnings` emits nothing, because the default threshold fires at **eight** parameters and
`build_change` takes seven.

## Risks / Trade-offs

- **The change is large — 18 task groups over three loosely-coupled bodies of work (audit,
  rendering, gates).** → Groups are ordered so each commits independently: the gate extraction
  (groups 12–13) touches no `src/` file and can be abandoned at a group boundary without
  leaving the rendering work half-done, and vice versa.
- **A count floor written today rots as the tree grows.** → Stated rather than engineered
  around (Decision 6); each gate's positive control is the real proof, and the floor is the
  second line of defence. `AGENTS.md` records the re-measure step.
- **Proving twenty-eight gates able to fail is the most mechanical, most skippable group in
  the change, and skipping it is exactly how a gate becomes a rubber stamp.** → Group 13 splits
  it into three uniform sweeps — remove each gate's positive-control file, run each count floor
  at plus one, and plant each gate's subject defect — rather than one bespoke argument per
  gate. About seven scripts record their measured plant in their own header; the rest have one
  derived and **written into the header**, so the next reader can re-run the proof.
- **The coverage map could be satisfied by a weak test.** → The map records a `tier` and a
  `why` per row, and groups 7–9 plant the *absence of the degraded state* against each `view`
  proof, so a test that would pass against a pane rendering nothing fails the exercise.
- **`Dashboard`'s thirteenth field breaks every construction site.** → Deliberate: that is
  what `NODEFAULT-UI` and the destructuring companion exist for, and the compiler enumerates
  the sites.
- **Adding leading lines to the detail content area changes the scroll clamp.** → A scenario
  pins it, and `detail-scroll`'s clamp already computes against `content_lines`' full length,
  so the change is to the input, not to the rule.
- **`Outcome::problems` is source-breaking inside the crate.** → Three call sites, all named
  in Contracts; the compiler finds them.

## Migration Plan

None required. No stored data changes shape, no format is versioned, and the plugin is not
published anywhere. Deploy order is `make build`; rollback is `git revert` of the change's
commits. An `agent-names.toml` written by an older build is read identically by this one.

## Visual Design

Not applicable. The change modifies a terminal UI; there is no HTML or email design source in
this repository, and none is invented.

## Open Questions

None. The two items `HANDOFF.md` left open — `WIRED`'s repair and the unextracted gates — are
resolved by Decisions 5, 6, 7, and 9. The one behavioural defect the audit found (row 23) is
repaired here and named against `agent-launch` in Context and in `notes/audit.md`, rather than
being left for a change that does not exist.
