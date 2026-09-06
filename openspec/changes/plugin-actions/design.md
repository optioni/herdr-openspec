## Context

The dashboard exists and works, but nothing summons it. `herdr-plugin.toml` declares one
`dashboard` pane and no `[[actions]]`, so the only way in is typing
`herdr plugin pane open --plugin herdr-openspec --entrypoint dashboard` by hand — which
`README.md` already admits while simultaneously advertising two action-menu entries that do
not exist. `HANDOFF.md`'s doc-claim rule names this change as the one that makes those two
rows true.

Everything below that is not already in `SPEC.md` was **measured live against the installed
Herdr 0.8.2**, by linking a scratch copy of the proposed manifest as a throw-away plugin,
opening and closing real panes, and invoking a real action whose command dumped its
environment. Six of those measurements contradict or extend `SPEC.md`, and one of them is a
latent bug in the shipped manifest.

| Measured against Herdr 0.8.2 | Consequence |
|---|---|
| A plugin action's process cwd is the **plugin root**, not the workspace's (Herdr 0.8.0: "Relative plugin commands now resolve from the plugin root") | A `--cwd`-less pane opens on the plugin root. For a GitHub-installed plugin that is `~/.config/herdr/plugins/github/…`, where `resolve::find_repo` finds no `openspec/` — the dashboard would be permanently empty |
| The invocation context arrives as `HERDR_PLUGIN_CONTEXT_JSON` (with `workspace_id`, `workspace_cwd`, `tab_id`, `focused_pane_id`, `focused_pane_cwd`) plus `HERDR_PLUGIN_ID`, `HERDR_WORKSPACE_ID`, `HERDR_PANE_ID`, `HERDR_BIN_PATH`, `HERDR_SOCKET_PATH` | The `--cwd` and the target are readable with no subprocess, exactly as `HERDR_PLUGIN_CONFIG_DIR` already is |
| An action's stdout and stdin are **not** a terminal | `open` must never require one, and must never reach status 3 |
| `herdr plugin pane open` is **not idempotent** — a second identical call opens a second pane | "Open or focus" has to be implemented by the plugin |
| `herdr pane list` exposes **no plugin ownership**; a plugin pane is distinguished only by a `label` field carrying the manifest pane's `title`, which ordinary panes omit entirely | The matcher keys on `label` + `workspace_id`. **Corrected in group 10** (Decision 6): originally `+ cwd` too, dropped once `--cwd` stopped being passed at all — see Decision 6 |
| A `split`-placement plugin pane targets an **existing** pane — the focused one by default, or `--target-pane <id>`. Naming a non-focused `--workspace <id>` with no target fails `invalid_params` "split and zoomed plugin panes target an existing pane; use target_pane_id" at exit 1. `--direction` is **optional** here, unlike `herdr pane split` | `open` passes `--target-pane`, never `--workspace`; `open-tab` passes `--workspace`, never `--target-pane` |
| `herdr plugin pane focus <PANE_ID>` exists in 0.8.2 and reaches the server (a bogus id answers `plugin_pane_not_found`, exit 1). It appears **nowhere** in Herdr's 110 KB changelog, so its first version is unknown and `herdr-file-viewer` (min 0.7.0) still works around focus with a `pane zoom` cycle | The focus call is used, and a **usage**-shaped failure (exit 2) falls through to opening once rather than leaving the user with nothing — see Decision 4 |
| The open response is `{"id":"cli:plugin","result":{"plugin_pane":{"entrypoint":…,"plugin_id":…,"pane":{"pane_id":…}}}}` — a **different envelope** from `pane split`'s `result.pane.pane_id` | Recorded in `SPEC.md`; deliberately **not parsed** by this plugin |
| The `plugin` family fails in the two shapes `agent-launch` already recorded: domain error = JSON envelope on **stderr**, exit **1**; usage error = plain text on **stderr**, exit **2** | The launcher's opaque-reason policy carries over unchanged |
| Herdr's changelog puts manifest-declared actions, managed plugin panes, plugin pane placement, and plugin invocation-context/env injection all in **0.7.0** | `min_herdr_version` stays `0.7.0`; this change does not touch it |
| Linking the proposed manifest verbatim succeeds, and `herdr plugin action list` echoes both actions with their titles; `placement = "tab"` is accepted | The manifest block in `SPEC.md` is right about everything except the tab action's command |

## Goals / Non-Goals

**Goals:**

- Two action-menu entries that open the dashboard, in a split or in a new tab.
- Pressing either twice focuses the dashboard instead of stacking panes.
- The pane opens on the **workspace's** repository, not the plugin root.
- The manifest, the Cargo bin name, and `README.md`'s advertised titles are tied together
  by a check that runs inside `make check` and therefore cannot rot.
- `SPEC.md` corrected against the running binary.

**Non-Goals:**

- No toggle-off / close-on-repeat, no cross-workspace switching, no zoom cycling.
- No `[[events]]` or `[[startup]]` hooks, no keybinding installation.
- No change to `min_herdr_version`, `platforms`, `[[build]]`, or the `dashboard` pane's
  own id/title/placement/command.
- No new dependency, no second bin target, no dashboard behaviour change.
- `DEPS` and `GRAPH-SNAP` stay red and stay `spec-purposes`' — this change adds no
  dependency and touches neither script, so repairing them here would be widening.

## Boundaries

| Piece | Follows |
|---|---|
| `src/open.rs` (new) | `src/launch.rs` exactly: a top-level module, the crate's **third** `cli::HerdrCli` consumer (after `src/agents.rs` and `src/launch.rs`; `src/cli.rs` declares the trait and `src/ui/mod.rs` only forwards a constructed handle) and the **fifth** file on the seam gates' `ALLOWED` list — two different counts, reaching the `herdr` program only through `Arc<dyn HerdrCli>`/`&dyn HerdrCli`, naming no spawn API and no `ratatui` type |
| `open::context(&dyn Fn(&str) -> Option<String>)` | `config::config_dir` / `state::state_dir`: the environment reaches it as an injected lookup, never `std::env::var` |
| `open::run(&dyn HerdrCli, …) -> Report` | `launch::run_request`: the impure driver is one function over the trait object, with the whole policy in pure functions beside it |
| `open::run_from_env` | `cli::worker_cli_from_env`: a one-line production binding, so `src/main.rs` never names `RealHerdrCli` or `agent_cli_via` and the two seam gates stay satisfied |
| `Invocation::Open` / `Invocation::OpenTab` | The landed `Invocation::Ui` — flat tokens, no flag state |
| `tests/manifest.rs` (new) | `tests/ci_workflow.rs`: a committed Rust parity guard over non-Rust files, run by `cargo test` and therefore by `make check` and CI |

`src/open.rs` sits at the top level of `src/`, never under `src/ui/`, because `NOCLI-SHELL`
forbids any file there from naming `HerdrCli`. It starts no thread and holds no state: unlike
`launch`, `agents`, `watch`, and `refresh`, it never runs beside a render loop — it is a
one-shot process that exits. `NOBLOCK`'s seam-module list therefore does **not** gain a
fifth file.

## Contracts

The consumer outside this crate is **Herdr**, through `herdr-plugin.toml`.

- **Shape:** two `[[actions]]` (`open`, `open-tab`) and two `[[panes]]` (`dashboard` split,
  `dashboard-tab` tab). The `dashboard` pane is byte-identical to the landed one.
- **Error surface:** an action's failure is a non-zero exit with the reason on stderr,
  which Herdr records in `herdr plugin log`. Nothing is written to stdout.
- **Compatibility: additive for Herdr, BREAKING as a manifest change** by this project's
  own rule. No existing entry changes; `herdr plugin pane open --entrypoint dashboard`
  keeps working. The `plugin-manifest` spec's negative requirement ("Actions and the tab
  pane are deliberately absent") is removed, which is why the change is marked breaking.
- **Consumers affected:** Herdr's action menu and keybinding layer; `README.md`'s Install
  section; nothing inside the crate except `src/main.rs`.
- **The `Change` type is not altered**, and neither `changes::from_files` nor
  `changes::from_cli` is touched: `open` never constructs, reads, or serialises a `Change`.
  Recorded because `openspec/config.yaml` → `design` requires the statement either way.

## Persistence and Rollout

- **Migration:** none — no data, no schema, no state file format change.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. `open` reads no cache and writes none; the pane it opens
  does its own startup read.
- **Index rebuild:** none.
- **Authorization:** none beyond Herdr's own — the plugin acts as the user, through the
  same socket every other `herdr` command uses. `open` writes nothing anywhere: not into
  `openspec/`, not into `HERDR_PLUGIN_STATE_DIR`, not into the config directory.
- **Observability:** the reason for any failure goes to stderr, which Herdr captures per
  action (`herdr plugin log`). No new logging surface.
- **Deployment:** `herdr plugin link .` (or a GitHub `plugin install`) re-reads the
  manifest; a user with the plugin already linked must relink or restart the server for
  the new actions to appear. Noted in `README.md`.

## Test Boundaries

| Dependency | In acceptance test (`tests/cli.rs`) | In unit tests (`src/open.rs`) | In the contract test (`tests/manifest.rs`) |
|---|---|---|---|
| The `herdr` program | **never started** — every run scrubs `HERDR_*` from the child's environment so the command fails at context-reading, before any spawn | **replaced** by `cli::FakeCli` through `&dyn HerdrCli` | not touched |
| The Herdr socket | never reached | never reached | never reached |
| The `openspec` program | not touched | not touched | not touched |
| The process environment | **real**, and deliberately scrubbed of `HERDR_*` per child | **replaced** by an injected `&dyn Fn(&str) -> Option<String>` closure | not read |
| The terminal | **real but piped** — stdout and stderr are pipes, stdin at EOF | never touched; `src/open.rs` names no terminal API | not touched |
| The filesystem | only the built binary, spawned by path | not touched — no path is opened by `open` | **real, read-only**: `herdr-plugin.toml`, `README.md`, `target/release/herdr-openspec` |
| `herdr-plugin.toml` | not read | not read | **real** |
| `README.md` | not read | not read | **real** |
| The Cargo bin name | `env!("CARGO_BIN_EXE_herdr-openspec")` | not used | `env!("CARGO_PKG_NAME")` and `env!("CARGO_BIN_EXE_herdr-openspec")` — **no `cargo metadata` subprocess** |
| A live Herdr 0.8.2 | not used | not used | not used — the three link/open scenarios are a **manual live CHECK task**, kept out of `make check` by `quality-gates` → "The gates do not depend on Herdr" |
| `openspec/` | never written; proved by `OPENSPEC-UNTOUCHED` against the `BASE` task 0.1 recorded | never read | never read |
| The working tree | `tests/cli.rs` writes only under `std::env::temp_dir()` through `testutil::ScratchDir` (the scratch `herdr` stub and its recording file), never into the repository | untouched | untouched |
| The user's live Herdr session | **mutated, and restored**, by group 10 alone: `plugin link` / `unlink`, panes and tabs opened and closed, actions invoked. Every step names its undo, and the group ends by confirming `herdr pane list` and `herdr plugin list` match their pre-task state | not touched | not touched |
| The repository's own files, temporarily | **mutated, and reverted**, by the planted-defect tasks 7.4, 9.6, and 11.1 (`src/main.rs`, `herdr-plugin.toml`, `README.md`, a scratch `src/zz_probe.rs`). Each names its revert and each group ends on a clean `git status` | not touched | not touched |

## Test Strategy

Four tiers, fastest first: **unit** (`cargo test --lib`, everything in `src/open.rs` and
`src/lib.rs`), **contract** (`cargo test --test manifest`), **binary-integration**
(`cargo test --test cli`, spawning the crate's own binary), and **command-level gates**
(shell/python checks run per task). One tier is deliberately outside all of them: a
**manual live check** against the installed Herdr, because `make check` may not depend on
Herdr.

**This change takes the outer-loop acceptance test.** `tests/cli.rs` runs the real built
binary as `open` and as `open-tab` with `PATH` pointing at a scratch directory holding a
`#!/bin/sh` program named `herdr` that records its argument vector and answers `pane list`
with an empty pane array. `cli::HERDR_PROGRAM` is the bare name `herdr`, resolved through
`PATH`, so this drives `main` → `parse` → `placement_for` → `run_from_env` → `open::run` →
the real spawn, end to end, and lets the test assert the **placement each subcommand
produced**. A status-only assertion could not: with `HERDR_*` scrubbed both subcommands die
identically inside `open::context`, and a `main` that dispatched `open-tab` to the split
placement would pass — the `live-refresh` defect class one level in. A second, in-process
outer loop drives `open::run` against `FakeCli` and asserts the **exact recorded
`(program, argv)` sequence**, so a call issued to the wrong program or with a silently
reordered vector fails.

**The Command column names exact test functions, not filters.** `cargo test <filter>`
matching nothing exits 0, so a filter that does not substring-match a real test name is a
green check that ran nothing. Every name below is the name tasks.md creates. The binding
floor on top of them is `TESTCOUNT`, which fails below a measured minimum.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| `open` alone classifies as the split-pane invocation | `parse(&["open"])` equals `Invocation::Open` | unit | none | `cargo test --lib open_alone_classifies_as_open` |
| `open-tab` alone classifies as the tab invocation | `parse(&["open-tab"])` equals `Invocation::OpenTab` | unit | none | `cargo test --lib open_tab_alone_classifies_as_open_tab` |
| A flag after a subcommand is rejected, carrying the token | three `parse` assertions, `ui --tab` included | unit | none | `cargo test --lib a_flag_after_a_subcommand_is_rejected` |
| Usage names all three commands | the landed `usage_lists_ui`, extended to split the `Commands:` block and assert three whole first tokens | unit | none | `cargo test --lib usage_lists_ui` |
| A full context yields all four fields | `open::context` over a fixture closure | unit | injected env lookup | `cargo test --lib context_full` |
| The discrete variables answer when the context JSON is absent | same, JSON key absent | unit | injected env lookup | `cargo test --lib context_from_discrete_variables` |
| Unreadable context JSON falls back rather than failing | two closures: non-JSON, and a JSON array | unit | injected env lookup | `cargo test --lib context_ignores_unreadable_json` |
| The workspace cwd falls back to the focused pane's cwd | context with `focused_pane_cwd` only, then with neither | unit | injected env lookup | `cargo test --lib context_workspace_cwd_falls_back_to_focused_pane_cwd` |
| Blank values are absent | whitespace-only and empty values | unit | injected env lookup | `cargo test --lib context_treats_blank_as_absent` |
| No workspace at all is the one fatal absence | empty closure; error names `HERDR_WORKSPACE_ID` | unit | none — `context` takes no `HerdrCli`; no call is made because `run_from_env` returns before constructing one, structural by inspection of its early-`match` shape | `cargo test --lib context_without_a_workspace_is_the_one_fatal_absence` |
| A matching pane is focused and nothing is opened | `run` against `FakeCli`; exact two-call `calls()` sequence | unit (outer loop) | `FakeCli` | `cargo test --lib cross_placement_focus` |
| The label and workspace id alone decide a match | matcher unit test | unit | none | `cargo test --lib matches_label_and_workspace` |
| No listed pane matches, so one is opened | unlabelled panes, then `"panes": []` | unit | `FakeCli` | `cargo test --lib no_match_opens_instead` |
| A matching entry with no usable `pane_id` is not a match | entry with no `pane_id`, and with a non-string one | unit | `FakeCli` | `cargo test --lib entry_without_a_pane_id_is_no_match` |
| A labelled pane in another workspace is not this workspace's dashboard | matcher unit test + `run` call sequence | unit | `FakeCli` | `cargo test --lib other_workspace_is_no_match` |
| Two matches focus the first in list order | matcher returns `w8:pG` | unit | none | `cargo test --lib two_matches_take_the_first` |
| `open-tab` focuses a split dashboard, and `open` focuses a tab one | two `run` cases over one listing | unit | `FakeCli` | `cargo test --lib cross_placement_focus` |
| The full split argument vector | `open_args(Placement::Split, …)` equals the literal vector | unit | none | `cargo test --lib split_argv` |
| No focused pane id omits `--target-pane` rather than passing an empty one | argv holds neither the flag nor an empty element | unit | none | `cargo test --lib split_argv_without_target_pane` |
| No `--cwd` is ever passed, regardless of what the context carries | argv holds no `--cwd` whether `workspace_cwd` is `Some` or `None`, for either placement | unit | none | `cargo test --lib no_cwd_regardless_of_context` |
| The full tab argument vector | `open_args(Placement::Tab, …)` equals the literal vector | unit | none | `cargo test --lib tab_argv` |
| The two subcommands differ only in placement and target | one test diffing the two vectors on exactly four keys | unit | none | `cargo test --lib the_two_vectors_differ_only_in_placement_and_target` |
| A domain error is carried verbatim and stops the command | `CliError::Failed` code 1; `Report.outcome` is `Err` holding both strings | unit | `FakeCli` | `cargo test --lib open_domain_error_stops` |
| A usage error on the open call is treated identically, without being parsed | `CliError::Failed` code 2 on the **open** call; same assertion | unit | `FakeCli` | `cargo test --lib open_usage_error_stops` |
| An unstartable `herdr` names the program | `CliError::NotStarted`; reason names `herdr` | unit | `FakeCli` | `cargo test --lib herdr_not_started_names_the_program` |
| A domain-error focus stops the command and opens nothing | `calls()` has exactly two entries, neither an open | unit | `FakeCli` | `cargo test --lib focus_domain_error_stops_and_opens_nothing` |
| A usage-error focus warns and opens once | `calls()` has exactly three entries ending in an open; warning recorded; outcome `Ok` | unit | `FakeCli` | `cargo test --lib focus_usage_error_warns_and_opens_once` |
| A failed listing warns and still opens | `Report.warnings` non-empty, second call is an open, outcome `Ok` | unit | `FakeCli` | `cargo test --lib listing_failure_warns_and_still_opens` |
| An unparseable listing warns and still opens | two cases: `not json`, and JSON without `result.panes` | unit | `FakeCli` | `cargo test --lib listing_unparseable_warns_and_still_opens` |
| A successful open is silent | `Report` has no warnings and `Ok` outcome | unit | `FakeCli` | `cargo test --lib successful_open_is_silent` |
| Each subcommand maps to its own placement | `placement_for` over all four `Invocation` shapes | unit | none | `cargo test --lib placement_for_maps_each_subcommand` |
| The exit status and stderr lines follow the report | `report_output` over three reports | unit | none | `cargo test --lib report_output_follows_the_report` |
| `open` outside Herdr fails promptly rather than hanging or rendering | real binary, `HERDR_*` scrubbed, ten-second deadline, status 1, stderr names `HERDR_WORKSPACE_ID` | binary-integration (outer loop) | real process, piped streams | `cargo test --test cli open_outside_herdr_exits_one` |
| `main` really routes each subcommand to its own placement | real binary against a scratch `#!/bin/sh` `herdr` on `PATH`; recorded argv shows split vs tab, and that neither carries `--cwd` | binary-integration (outer loop) | real process, scratch program, synthetic env | `cargo test --test cli main_routes_each_subcommand_to_its_own_placement` |
| The open path names no terminal API | grep of `src/open.rs`, with `src/ui/terminal.rs` as the positive control | command-level gate | none | `LAUNCHSEAM` open leg, leg 2 |
| The module spawns nothing | grep of `src/open.rs`; tree-wide sweep excluding `src/cli.rs` | command-level gate | none | `MIN=24 sh $CHECKS/NOSPAWN-GREP.sh`; `LAUNCHSEAM` open leg, leg 1 |
| The Herdr handle stays inside the five allowed files | leg 3 of both seam gates, with `src/open.rs` added to `ALLOWED` | command-level gate | none | `LAUNCHSEAM` / `AGENTSEAM` leg 3 at `MIN=23` |
| No file under `src/ui/` names the open path's CLI seam | `NOCLI-SHELL` at its measured floor | command-level gate | none | `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh` |
| Herdr links the working tree | `herdr plugin link .`, then `plugin list` and `plugin action list`; output pasted into the task | manual live check | real Herdr 0.8.2 | task 10.2 |
| The dashboard pane launches the binary and stays open | open the `dashboard` entrypoint, confirm the pane, close it | manual live check | real Herdr 0.8.2 | task 10.3 |
| The tab pane opens in its own tab | open `dashboard-tab` with `--placement tab`, confirm the new tab, close it | manual live check | real Herdr 0.8.2 | task 10.4 |
| The manifest parses and its declared paths resolve | `tests/manifest.rs` parses with `toml`; asserts the tables and `scripts/build.sh`, not the release artifact | contract | real files | `cargo test --test manifest manifest_parses_and_declares_the_two_tables` |
| The manifest path and the Cargo binary name agree | four basename comparisons against `CARGO_PKG_NAME` and `CARGO_BIN_EXE_…`'s stem | contract | real files | `cargo test --test manifest manifest_paths_and_the_cargo_binary_name_agree` |
| A renamed binary fails the gate | planted-defect run: six edits in turn, each confirmed red, then reverted | contract (planted defect) | real files | task 9.6 |
| The pane title and the pane matcher are the same string | titles, ids, and placements against `open::DASHBOARD_LABEL` and `open_args`' literals | contract | real files | `cargo test --test manifest pane_titles_ids_and_placements_match_open_args` |
| README and the manifest agree on the action titles | whitespace-collapsed README; longer title asserted separately; deferral sentence absent | contract | real files | `cargo test --test manifest readme_and_the_manifest_agree_on_the_action_titles` |
| `ui` with stdout piped exits 3 without blocking | the landed test, unchanged | binary-integration | real process | `cargo test --test cli ui_without_a_terminal_exits_three` |
| The three failing statuses are distinct | the landed test, extended to assert the new usage text | binary-integration | real process | `cargo test --test cli failing_statuses_are_distinct` |
| The open family never reaches status 3 | statuses 1, 1, 2 for `open`, `open-tab`, `open --tab` | binary-integration | real process | `cargo test --test cli open_never_reaches_status_three` |
| Every binary-integration run pipes stdout | a **new** executing inspection test reading `tests/cli.rs` through `file!()`; the landed scenario was manual inspection, not a test | binary-integration | the test file's source | `cargo test --test cli every_run_pipes_and_scrubs_herdr` |

Coverage: reported as the **line** figure from `cargo llvm-cov`, never the region count.
`src/open.rs` is written to be almost entirely pure, with one untestable line
(`run_from_env`'s construction of the real handle) on `cli::worker_cli_from_env`'s terms.

## Decisions

1. **Two flat subcommands, not `open --tab`.** `SPEC.md`'s manifest block shows
   `["open", "--tab"]`; `IMPLEMENTATION-ORDER.md` says "the `open` and `open-tab` binary
   subcommands". They contradict, so one had to be corrected. Flat wins: `parse` stays a
   total function over a token list with no flag state, its landed `ui --tab` rejection
   keeps its meaning instead of becoming an exception, and the action id, the subcommand,
   and the pane id (`open-tab` / `dashboard-tab`) line up. *Alternative:* teach `parse` a
   flag grammar — rejected as new parser surface bought for one boolean.
2. **Open **or focus**, with the label as the only ownership signal.** `pane list` exposes
   no plugin id, so the matcher is `label == DASHBOARD_LABEL` **and** same `workspace_id`.
   **Corrected in group 10**: the first draft added **and** same `cwd`, dropped once
   `--cwd` stopped being passed at all (Decision 6) — every pane this plugin opens now
   carries the plugin root as its `cwd`, identically, so the conjunct no longer
   discriminates anything. *Alternative:* record the opened pane id in
   `HERDR_PLUGIN_STATE_DIR` and focus that — rejected: it adds a write to a command that
   otherwise writes nothing, and a stale record survives a pane the user closed, which is
   strictly worse than re-reading the live listing.
3. **One shared label for both panes.** Both `[[panes]]` keep `title = "OpenSpec"`, so
   `open` and `open-tab` focus each other's pane. "Show me the dashboard" has one answer
   per workspace. *Alternative:* `title = "OpenSpec (tab)"` to tell them apart — rejected:
   it would open a second dashboard in the same workspace whenever the user reached for
   the other entry, which is the duplication this change exists to prevent.
4. **A failed `pane list` degrades to opening; a failed `focus` degrades only on a usage
   error.** Not knowing whether a dashboard exists is the plain "never fail closed" case —
   open one, warn. The focus call is the interesting one, and the first draft of this
   design got it wrong by refusing outright: `herdr plugin pane focus` appears nowhere in
   Herdr's changelog and `herdr-file-viewer` (min 0.7.0) still works around focus with a
   `pane zoom` cycle, so on some Herdr the manifest declares supported, an unconditional
   refusal means **nothing visibly happens** on every second invocation — failing closed on
   a declared-supported version. The repair uses the exit code the seam already carries, so
   nothing is parsed: code **2** is the measured usage shape, which is what "this Herdr has
   no such subcommand" looks like, and falls through to opening once; anything else is a
   domain failure — including the recoverable "the pane closed between the listing and the
   focus", which is recovered by invoking the action again — and stops. The leak an
   unconditional fallback would cause is bounded to one extra dashboard on an old Herdr,
   which is the degraded behaviour this project prefers to nothing at all.
   *Alternative:* raise `min_herdr_version` — rejected, it would cross a Non-Goal to buy
   less than the exit-code split does.
5. **The open response is not parsed at all.** Only `pane list`'s output is. Exit status
   already carries success, and the open envelope's measured shape
   (`result.plugin_pane.pane.pane_id`) differs from `pane split`'s (`result.pane.pane_id`),
   so parsing it would add a failure mode for information nothing needs.
6. **CORRECTED during this change's own group 10 live check — `--cwd` is never passed to
   `plugin pane open` at all.** The first draft of this decision read: "`--cwd` comes from
   the injected context, not from `std::env::current_dir`. The action's cwd is the plugin
   root; using it would ship a dashboard that resolves no repository once installed from
   GitHub. `workspace_cwd`, else `focused_pane_cwd`, else omit the flag and accept the
   degraded pane rather than refuse." Measured live against Herdr 0.8.2:
   `herdr plugin pane open --cwd /tmp …` fails outright —
   `{"error":{"code":"plugin_pane_open_failed","message":"Unable to spawn
   /tmp/./target/release/herdr-openspec because it does not exist"}}` — because Herdr
   resolves the manifest's **relative** pane `command` against `--cwd` too, not only
   against the plugin root, contradicting its own changelog's unqualified "Relative plugin
   commands now resolve from the plugin root" (0.8.0) for the `--cwd`-given case. This is
   not a corner case: it fails for every real workspace directory, which is exactly the
   case this flag existed to serve. `herdr-file-viewer` (installed locally, min 0.7.0)
   independently reaches the same conclusion — its shipped launcher never passes `--cwd`
   either. **Repair:** `open_args` never emits `--cwd`, on any path; `existing_pane` drops
   its `cwd` parameter (every pane this plugin opens now carries the plugin root
   identically, so `cwd` no longer discriminates); and `ui::run` gains `startup_cwd`,
   which prefers the workspace cwd from its own injected `HERDR_PLUGIN_CONTEXT_JSON` /
   `HERDR_WORKSPACE_ID` — read through the same `open::context` — over
   `std::env::current_dir()`, falling back to it when no Herdr context is present at all.
   This is the one place this change crosses its own "No dashboard behaviour change"
   Non-Goal, and it does so because the alternative (keep `--cwd`) does not work at all;
   see `specs/plugin-build/spec.md` -> "The dashboard's own starting directory prefers the
   workspace context over the process cwd" and `HANDOFF.md`'s "a doc claim is fixed by the
   change that makes it true" — the same principle applied to a design decision found wrong
   at implementation time rather than a stale doc.
7. **`--target-pane` for the split, `--workspace` for the tab.** Forced by measurement, not
   preference: `--placement split --workspace <id>` is `invalid_params` unless that
   workspace happens to be focused, and a tab needs no target. Passing the context's
   `focused_pane_id` makes the split deterministic rather than dependent on ambient focus.
8. **The manifest contract becomes `tests/manifest.rs`, inside `make check`, and asserts
   only values with a second site.** Every named gate in this repository lives outside
   `make check`, and two have been red on `main` for three changes because nothing forces
   them to run; the manifest is the one contract with a consumer outside the crate. Scope
   is what keeps it from being the "test that asserts a config key" the schema warns about:
   it pins the four command basenames, both pane titles, ids, and placements, the two
   action command tails, and both action titles — each against a **second** site in code or
   in `README.md`, where disagreement is silent. It does **not** pin `version`,
   `min_herdr_version`, or `platforms`: those have no second site, a wrong value is
   rejected loudly by `herdr plugin link` (task 10.2), and pinning `version` would make a
   legitimate bump red. It also does not assert that `target/release/herdr-openspec` exists
   — `make check` never runs `make build` and `cargo test` builds the debug profile, so
   that assertion would be red on every clean checkout and in CI, which is exactly the
   rot this decision exists to avoid. It takes the binary name from `env!` rather than
   spawning `cargo metadata`, so it needs no subprocess and — per `quality-gates` → "The
   gates do not depend on Herdr" — no Herdr. *Alternative:* a 31st shell gate — rejected
   for the reason above.
9. **`LAUNCHSEAM` gains a parameter instead of the roster gaining a gate.** Its positive
   control becomes `ENTRY="${ENTRY:-pub fn start\(}"`, and it is invoked a second time as
   `LAUNCH=src/open.rs ENTRY='pub fn run_from_env\(' ALLOWED='… src/open.rs' MIN=23`. Legs
   1, 2, and 3 apply to `src/open.rs` verbatim. The roster stays at **30**, so the
   `ls -1 "$CHECKS" | wc -l` count check is unchanged and no new artefact can rot.
10. **`run` returns a `Report`, and both of `main`'s decisions are pure functions.**
    `pub struct Report { warnings: Vec<String>, outcome: Result<(), String> }` keeps every
    decision assertable without capturing a stream. `open::placement_for(&Invocation)` and
    `open::report_output(&Report) -> (Vec<String>, i32)` carry the subcommand-to-placement
    mapping and the status mapping, so `main`'s two new arms hold no branch of their own —
    `plugin-build`'s own requirement forbids logic in `main` that a test cannot reach, and
    the success path (exit 0, silent) is otherwise unreachable without a live Herdr.
12. **The outer loop drives a scratch `herdr` on `PATH`, not a scrubbed environment
    alone.** With `HERDR_*` removed both subcommands fail identically inside
    `open::context`, so a status-only acceptance test cannot tell `open` from `open-tab`
    and would pass a `main` that dispatched both to one placement. `cli::HERDR_PROGRAM` is
    the bare name `herdr`, resolved through `PATH`, so a `#!/bin/sh` recorder in a scratch
    directory plus a synthetic context drives the whole path and lets the test assert the
    placement each subcommand produced. *Alternative:* trust the unit test on
    `placement_for` — rejected: nothing would then join it to `main`'s dispatch, which is
    the `live-refresh` defect class exactly.

11. **No thread, no `NOBLOCK` entry.** `open` is a one-shot process, not a collaborator
    beside a render loop, so it is not added to `NOBLOCK`'s seam-module list and does not
    need `NOBLOCK`'s Guard D. It is added to `READONLY-UI`'s `EXTRA` list, because the
    "the plugin writes only its state directory" property does apply to it.

## Risks / Trade-offs

- **`herdr plugin pane focus` could be newer than `min_herdr_version = "0.7.0"`.** It is
  measured working on 0.8.2 and Herdr's changelog puts managed plugin panes in 0.7.0, but
  no 0.7.x binary is available to test, and `herdr-file-viewer` (min 0.7.0) works around
  focus with a `pane zoom --on/--off` cycle, commenting that "herdr has no focus-by-id"
  as of 0.7.1. → On a Herdr that does not know the subcommand the call fails as a usage
  error, the reason lands in `herdr plugin log`, and no pane is leaked (Decision 4). The
  floor stays `repo-foundation`'s; a bump, if one is ever needed, belongs to whichever
  change measures a real 0.7.x failure. Recorded in `SPEC.md`.
- **Herdr 0.8.0 changed how relative plugin commands resolve** ("now resolve from the
  plugin root"). The manifest's `./target/release/herdr-openspec` is relative and predates
  this change. → Out of scope and unchanged; noted so a later relative-path bug is not
  attributed here.
- **RETIRED, corrected by group 10.** The first draft of this risk read: "The `cwd` match
  is string-shaped and could silently never fire" — worrying that Herdr might canonicalize
  `--cwd` before echoing it back in `pane list`, defeating a naive string comparison. Moot:
  `--cwd` is never passed at all any more (Decision 6), so the matcher no longer compares
  `cwd` in the first place. If a grep brings you here, this is the correction, not a live
  risk.
- **Two invocations racing still open two dashboards** — both list, both see nothing, both
  open. → Accepted. Decision 2 rejects the state-file fix for reasons that still hold, and
  the window is one socket round trip behind a human keypress.
- **The label matcher would collide with another plugin whose pane title is `OpenSpec`.**
  → Accepted, and recorded in `SPEC.md` as a known limitation: `pane list` offers nothing
  narrower, and — **corrected by group 10**, `cwd` dropped from the matcher entirely — a
  collision now requires only a second plugin opening a same-titled pane in the same
  workspace, not also the same repository.
- **RETIRED, inverted by group 10.** The first draft of this risk read: "A dashboard
  opened by the old README command (no `--cwd`) will not be matched, since its cwd is the
  plugin root, so the action opens a second one" — but every dashboard this plugin opens
  now has the plugin root as its `cwd` (Decision 6), the same as the old README command
  always produced, so that pane **is** matched and focused, not duplicated. Verified live
  in group 10: a pane opened via the direct `herdr plugin pane open` command was correctly
  focused by a subsequent `open` action invocation.
- **`DEPS` and `GRAPH-SNAP` are red on `main` before this change starts.** → Task 0.3
  records both failures verbatim as a baseline so a reviewer cannot attribute them here;
  the repair is `spec-purposes`'.
- **The three manifest link/open scenarios cannot run in CI.** → They are a manual live
  CHECK task whose recorded output is pasted into `tasks.md`; the automatable half of the
  same requirement is `tests/manifest.rs`, which does run in CI.
- **`cargo test` may run inside a Herdr pane**, inheriting a live `HERDR_*` context and
  letting a binary-integration run reach the real socket. → Every `open` run in
  `tests/cli.rs` scrubs `HERDR_*` from the child's environment with `Command::env_remove`,
  and an inspection test asserts that it does.

## Migration Plan

No data migration, no backfill, no rollback tooling. Deploy order is a single commit
sequence on `main`. A user with the plugin already linked relinks (or restarts the Herdr
server) to pick up the new manifest; a user installing fresh gets it on first install.
Rollback is `git revert` of the manifest commit — the `dashboard` pane is untouched, so a
revert leaves the pre-change entry point working exactly as before.

## Visual Design

Not applicable: this change builds no user-facing view and no email template. It adds two
menu entries whose only rendered artefact is a title string Herdr draws, and it opens the
already-designed dashboard pane unchanged. No design source exists or is needed.

## Open Questions

None. The one question that was open — whether `SPEC.md`'s `["open", "--tab"]` or
`IMPLEMENTATION-ORDER.md`'s `open-tab` is the contract — is settled by Decision 1 and
closed by a `SPEC.md` correction task.

`HANDOFF.md` → "Known-deferred doc fixes" item 3 (`IMPLEMENTATION-ORDER.md` crediting
`plugin-actions` with `min_herdr_version`/`platforms`) was **re-checked and is already
fixed**: the Phase 6 row reads "(`min_herdr_version` and `platforms` ship in
`repo-foundation`, not here.)". No task re-fixes it.
