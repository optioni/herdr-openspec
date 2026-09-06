## Context

Phase 5 opens the Herdr side of the plugin. `agent-polling` is its first row: poll
`herdr agent list` at roughly one second, parse the result, and hold it where
`agent-attribution` and `agent-launch` can reach it. Nothing renders yet.

The tree it lands on is not neutral. `live-refresh` shipped the live tier three months of
constraints ago and its Change Review found `ui::run` calling neither `watch::start` nor
`refresh::start` — every test green, the whole live tier permanently inert in the shipped
binary. This change adds a **third** background collaborator behind a **third** seam and can
reproduce that defect exactly, so the composition root is split here until a test can drive it.

Three facts about Herdr 0.8.2, measured live on the reference machine rather than read off
`SPEC.md`, shape the design:

1. `herdr agent list` emits JSON on stdout with **no `--json` flag**; passing one exits `2`
   with `usage: herdr agent list` on stderr.
2. The payload is an **envelope** — `{"id":…,"result":{"agents":[…],"type":"agent_list"}}` —
   and each entry carries `name` (the agent's name, **omitted when unset**) separately from
   `agent` (the agent **kind**, `"claude"` on every live agent here). `SPEC.md` lists `agent`
   and no `name`, which would have had `agent-attribution` matching change names against the
   string `claude`.
3. An unreachable socket exits **1**, writes nothing to stdout, and writes a JSON error
   envelope to **stderr** — the opposite of the `openspec` CLI, whose diagnostics go to stdout
   and whose reason `CliError::Failed` therefore cannot carry.

A successful call was measured at **8ms median** (min 7.3, max 9.4, ten runs). That is small,
and it is still too large to sit on the render path: `NOBLOCK`'s contract is that the loop
blocks on nothing but the terminal, and an 8ms subprocess once a second would make that claim
false by design rather than by accident. Hence a second worker thread.

## Goals / Non-Goals

**Goals:**

- One module, `src/agents.rs`, holding the `Agent` value, the parse, the non-blocking
  `AgentPoll` seam, and the worker — outside `src/ui/`, exactly as `watch` and `refresh` are.
- A poller that reports its own remaining time as a `Duration`, so one tick serves two pollers
  and no clock is read under `src/ui/`.
- An unreachable socket as a **supported standing state**: recorded, never rendered as a
  problem row, never an error, never a reason to stop polling.
- A composition root that a test drives, so the `live-refresh` wiring defect cannot recur
  silently.
- `SPEC.md` and `AGENTS.md` corrected where reality disagrees with them.

**Non-Goals:**

- Attribution, the agent column, the footer count (`agent-attribution`).
- Launching, focusing, `a`/`c`/`s`/`g`, `herdr pane split` (`agent-launch`).
- Any rendering at all. Every landed buffer assertion stays byte-identical.
- An event hook. The confirmed plugin event names carry no agent-status event, and the call is
  a Unix-socket round trip costing milliseconds.
- A new dependency. `serde_json` already ships.
- Any change to `Action`, to any keybinding, or to the manifest. This change is **not
  BREAKING**.

## Boundaries

| Piece | Module | Pattern it follows |
|---|---|---|
| `Agent`, `AgentStatus`, `Listed`, `AgentSnapshot` | `src/agents.rs` (new) | `changes::Change` — plain data, no `Default`, every field named at every site |
| `parse_list` | `src/agents.rs` | `changes::parse_list` — `serde_json::Value` navigation, `Vec<String>` problems, never a panic |
| `poll_once` | `src/agents.rs` | `changes::from_cli_cached`'s error mapping — `cli_error_problem`-shaped text, degrade never fail |
| `AgentPoll` trait, `none()` | `src/agents.rs` | `watch::FsEvents` — non-blocking, `drain` + `pending_in`, an inert implementation |
| `RealAgentPoll`, `start`, `worker_body` | `src/agents.rs` | `refresh::RealRefresher`/`start`/`worker_body` — one channel out, `try_recv` in, one `thread::spawn`, worker body below it |
| The schedule and its clock | `src/agents.rs` | `watch::RealFsEvents::drain` — one clock read per call, the `Duration` cached for `pending_in` |
| `agent_cli_via`, `HERDR_PROGRAM` | `src/cli.rs` | `cli::npm_prefix_via`/`npm_prefix` — a parameterised spawner plus a one-line binding to the real program |
| `soonest` | `src/watch.rs` | `watch::poll_timeout` — a pure `Duration` function reading no clock, beside its only consumer |
| `Dashboard.agents` | `src/ui/app.rs` | `Dashboard.refresh` — a sibling state field, not an entry on `ChangeSet::problems` |
| `Live.agents` | `src/ui/driver.rs` | `Live.fs`/`Live.refresher` — a third field on the struct, not a seventh parameter |
| `Startup`, `Collaborators`, `start_collaborators`, `run_wired` | `src/ui/mod.rs` | `ui::read_artifact` — the composition root shrunk until only a binding is left untested |
| `ScriptedAgents`, `UntilReady` | `src/lib.rs` `testutil` | `ScriptedFs`, `RecordingRefresher`, `Script` |

**Where the poller must not live, and why it is a constraint rather than a preference.** The
landed `NOCLI-SHELL` check forbids every file under `src/ui/` from naming `HerdrCli`. A poller
under `src/ui/` would need an exemption, and an exemption is how a check rots into a rubber
stamp. `src/agents.rs` sits beside `src/watch.rs` and `src/refresh.rs` for the same reason those
two do.

**Why a third field on `Live` and not a seventh `run_loop` parameter.** Cohesion: the three
collaborators are one concept, the loop's live tier, and `Live` was introduced by `live-refresh`
with this exact extension in mind. **The lint argument this project has repeated since
`live-refresh` is false and is retired here:** measured on this crate and toolchain, clippy's
`too_many_arguments` fires at **eight** parameters, not seven, so a seventh `run_loop` parameter
would not have tripped it — and the crate's single
`#[allow(clippy::too_many_arguments)]`, on `changes::build_change` (seven parameters), produces
no warning when removed. The shape decision stands on its own; the forcing constraint does not
exist, and recording that here stops a later change choosing a worse shape believing it had no
option.

**Why the snapshot is a sibling field and not a problem.** `Dashboard::adopt` replaces
`ChangeSet::problems` wholesale on every refresh, so a standing condition put there would vanish
on the next file result — and `refresh.problems` renders as a leading `!`-marked row, which
`SPEC.md` → Degraded states forbids for an unreachable socket ("runs as a standalone TUI",
which is silence). `Dashboard.agents` is therefore its own field, replaced wholesale by each
poll and read by nothing in this change.

## Contracts

Every interface below is **additive**. No landed signature changes except `ui::run`'s internal
split, which has one consumer (`main.rs`, through `lib::run`) and keeps its own signature.

```rust
// src/agents.rs
pub const POLL_INTERVAL: Duration = Duration::from_secs(1);

pub enum AgentStatus { Working, Idle, Blocked, Done, Unknown }

pub struct Agent {
    pub name: Option<String>, pub kind: Option<String>, pub status: AgentStatus,
    pub cwd: Option<PathBuf>, pub pane_id: String, pub tab_id: String,
    pub workspace_id: String, pub terminal_title: Option<String>,
}
pub struct Listed { pub agents: Vec<Agent>, pub problems: Vec<String> }
pub struct AgentSnapshot { pub agents: Vec<Agent>, pub reachable: bool, pub problem: Option<String> }

pub fn parse_list(text: &str) -> Result<Listed, String>;
pub fn poll_once(cli: &dyn crate::cli::HerdrCli) -> AgentSnapshot;

pub trait AgentPoll: Send {
    fn drain(&mut self) -> Option<AgentSnapshot>;
    fn pending_in(&self) -> Option<Duration>;
}
pub fn none() -> Box<dyn AgentPoll>;
pub fn start(cli: std::sync::Arc<dyn crate::cli::HerdrCli>) -> Box<dyn AgentPoll>;

// src/cli.rs
pub const HERDR_PROGRAM: &str = "herdr";
pub fn agent_cli_via(program: &Path) -> std::sync::Arc<dyn HerdrCli>;

// src/watch.rs
pub fn soonest(a: Option<Duration>, b: Option<Duration>) -> Option<Duration>;

// src/ui/driver.rs
pub struct Live<'a> {
    pub fs: &'a mut dyn crate::watch::FsEvents,
    pub refresher: &'a mut dyn crate::refresh::Refresher,
    pub agents: &'a mut dyn crate::agents::AgentPoll,
}

// src/ui/mod.rs
pub struct Startup<'a> { pub cwd: &'a Path, pub config: &'a Config, pub herdr: &'a Path }
pub struct Collaborators {
    pub fs: Box<dyn crate::watch::FsEvents>,
    pub refresher: Box<dyn crate::refresh::Refresher>,
    pub agents: Box<dyn crate::agents::AgentPoll>,
    pub problems: Vec<String>,
}
pub fn start_collaborators(repo: Option<&Path>, config: &Config, herdr: &Path) -> Collaborators;
pub fn run_wired<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>, events: &mut E, startup: &Startup<'_>,
    read: ArtifactReader<'_>, tick: Duration,
) -> Result<Dashboard, StartError>;
pub fn run() -> Result<(), StartError>;   // unchanged signature
```

**Error surface.** `poll_once` has none: it returns an `AgentSnapshot` for every outcome. The
only `Result` in the module is `parse_list`, whose `Err` is a `String` reason, and `poll_once`
turns that into `reachable: false` with the reason on `problem`. `run_wired` returns
`StartError` only for the failures `run_loop` already produces — a draw error or an event-source
error. An unreachable socket is never a `StartError`.

**Streaming and pagination.** None. `herdr agent list` returns one payload; there is no cursor
and no follow mode.

**Consumers.** `agent-attribution` reads `Dashboard.agents.agents` and `.reachable`;
`agent-launch` reads `.reachable` to hide its keys. Both are downstream in this repository and
neither exists yet.

## Persistence and Rollout

- **Migration:** none. No stored format changes.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. The poller holds no cache; each snapshot replaces the last.
  `changes::CliCache` is untouched.
- **Index rebuild:** none.
- **Authorization:** none in the plugin. `herdr agent list` is authorized by access to the Unix
  socket, which the operating system enforces; the plugin adds nothing and bypasses nothing.
- **Observability:** the snapshot's `problem` field is the only new diagnostic surface, and it
  is deliberately not rendered in this change. No logging is added: the pane has no log
  destination, and writing one would violate "the plugin's own writes are scoped to its state
  directory".
- **Deployment:** none beyond the normal `make build`. No manifest change, so `min_herdr_version`
  stays `0.7.0` — `agent list` predates 0.8.2 and this change relies on no 0.8-only field.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The `herdr` program | **real spawn of a scratch `#!/bin/sh` program** at an absolute path, through `cli::agent_cli_via`; never the installed `herdr` binary | replaced by `cli::FakeCli` registered on the `HerdrCli` side |
| The Herdr Unix socket | never reached; the scratch program stands in for the whole socket round trip | never reached |
| The `openspec` program | **real spawn of a scratch `#!/bin/sh` program**, reached through `Config::openspec_bin` so the probe chain stops at step 1 and the installed binary is never spawned | replaced by `cli::FakeCli`, or absent entirely (`refresh::none()`) |
| The filesystem (repository tree) | **real** — a `testutil::ScratchDir` holding one change, walked by the real `changes::from_files` and read by the real `ui::read_artifact` | real scratch trees in `agents`' own tests only where a scratch program must exist on disk; otherwise absent |
| The filesystem watcher (`notify`) | **real** — `watch::start` over the scratch tree, so an inert double wired in its place is caught | replaced by `testutil::ScriptedFs` |
| The refresh worker thread | **real** — `refresh::start`, answering from the scratch `openspec` program | replaced by `testutil::RecordingRefresher` |
| The agent poller thread | **real** — `agents::start`, answering from the scratch `herdr` program | replaced by `testutil::ScriptedAgents`; `agents::poller_for_test` exposes the raw result channel for the module's own tests |
| The terminal | **replaced** — `ratatui::backend::TestBackend` at 120x20 and 60x20; no real terminal exists in the test process | replaced identically |
| The terminal event source | **replaced** — `testutil::UntilReady`, which yields timeouts until a predicate holds or a deadline passes and then presses `q` | replaced by `testutil::Script` |
| The clock | **real**, and only inside `testutil::UntilReady` in `src/lib.rs` — never under `src/ui/`, never inside a view test | real inside `src/agents.rs` (`RealAgentPoll::drain`) and `src/watch.rs`; `soonest` and `poll_timeout` take `Duration`s and read none |
| The process environment | **replaced** — `Config` is constructed directly, never read from the environment; `Startup::herdr` is a parameter, so no `std::env::set_var` and no `PATH` mutation | replaced identically |
| Herdr's own JSON schema | **not** consulted at runtime; the shapes it defines are pinned by fixture payloads captured verbatim from Herdr 0.8.2 | same fixtures |
| The worker's exit channel | **real** — `agents::poller_for_test` hands back a third channel whose `Sender` the worker owns, so its return after a drop is observed rather than assumed | same |
| The repository tree, **written to** | the reachable-wiring test's readiness predicate deliberately writes one byte to `openspec/changes/alpha/proposal.md`, because a real filesystem event is the only thing that distinguishes a real watcher from the inert double; that test therefore makes no byte-identity claim, and the unreachable-socket test — which writes nothing — makes it instead | not applicable |
| The plugin state directory | untouched; this change writes nothing anywhere | untouched |

## Test Strategy

Three tiers, as `openspec/config.yaml` defines them: unit tests over the pure modules; view
tests rendering into a `TestBackend` at 60 and 120 columns; scratch trees and scratch
`#!/bin/sh` programs under `std::env::temp_dir()` for anything that must really exist. A fourth
kind of evidence, used throughout this repository, is a **command-level check** — a shell script
with positive controls and a proven-red plant, run from a task rather than from `cargo test`.

**This change takes the outer-loop acceptance test, and it is the reason for the `ui::run`
split.** `ui::tests::wiring::the_real_wiring_polls_a_scratch_herdr` drives `ui::run_wired` — the
composition root itself — with the real `ui::load`, the real `watch::start`, the real
`refresh::start`, the real `agents::start`, and the real `ui::read_artifact`, against a scratch
repository and two scratch programs. Each of the three
collaborators has its **own** discriminating assertion: the poller's is `agents.reachable`, the
worker's is the scratch `openspec` program's invocation log being non-empty, and the watcher's is
that same log reaching **two** entries after the test's own predicate writes one byte into the
change directory. `refresh.problems` being empty is a supporting assertion and deliberately not
the watcher's discriminator — `watch::start` returns an empty `problems` on success and
`watch::none()` is wired with an empty one too, so emptiness is satisfied by both arms and would
have let an unwired watcher through the very test built to catch one.
Every one of the three is planted and observed red
during implementation. Unit and view tests drive the components; only this one drives the thing
that composes them, which is precisely what `live-refresh` lacked.

Commands below are `testcount --lib <filter> <minimum>` — the repository's wrapper that runs
`cargo test --all-features <filter>` and fails unless at least `<minimum>` tests **passed**,
because a `cargo test` filter matching nothing exits 0. Check scripts are run as
`sh $CHECKS/<LABEL>.sh` from the extracted, byte-identical copies task 0.1 writes.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| agent-list: One poll is exactly one `agent list` call | `agents::tests::poll::args_are_exactly_agent_list` asserting `FakeCli::calls()` equality | unit | `FakeCli` (replaced) | `testcount --lib agents::tests::poll:: 6` |
| agent-list: The seam module spawns no process and names no view type | `AGENTSEAM` check with its four guards and five planted violations | check | source tree | `MIN=22 sh $CHECKS/AGENTSEAM.sh` |
| agent-list: The reference payload parses into one agent | `agents::tests::parse::the_reference_payload_parses` whole-value equality | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: An empty session lists no agents and is not an error | `agents::tests::parse::an_empty_agents_array_is_ok_and_empty` | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: A named agent carries its name | `agents::tests::parse::a_named_agent_carries_its_name` | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: Every status string decodes, and an unrecognised one is `Unknown` | `agents::tests::parse::every_status_decodes_and_others_are_unknown` over eight entries | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: An entry with only the required identity fields still parses | `agents::tests::parse::only_the_required_fields_still_parses` | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: One entry missing `pane_id` is skipped and the others survive | `agents::tests::parse::a_bad_entry_is_skipped_and_named`, three variants | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: An entry that is not an object is skipped, not fatal | `agents::tests::parse::a_non_object_entry_is_skipped`, three variants | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: A `cwd` outside the repository is still parsed | `agents::tests::parse::a_cwd_outside_the_repo_is_still_parsed` | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: Text that is not JSON is a named reason | `agents::tests::parse::non_json_is_a_named_reason` | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: A well-formed envelope with the wrong contents is a named reason | `agents::tests::parse::wrong_shapes_are_named_reasons`, four payloads | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: Herdr's own error envelope is reported by its code and message | `agents::tests::parse::herdrs_error_envelope_is_reported_by_code` | unit | none | `testcount --lib agents::tests::parse:: 13` |
| agent-list: An unreachable socket is a snapshot, not an error | `agents::tests::poll::a_failed_run_is_an_unreachable_snapshot` | unit | `FakeCli` (replaced) | `testcount --lib agents::tests::poll:: 6` |
| agent-list: No `herdr` program at all is the standalone-TUI case | `agents::tests::poll::not_started_is_an_unreachable_snapshot` | unit | `FakeCli` (replaced) | `testcount --lib agents::tests::poll:: 6` |
| agent-list: A payload with one bad entry is still reachable | `agents::tests::poll::a_partial_payload_is_still_reachable` | unit | `FakeCli` (replaced) | `testcount --lib agents::tests::poll:: 6` |
| agent-poller: The inert poller yields nothing and starts nothing | `agents::tests::seam::the_inert_poller_yields_nothing` | unit | none | `testcount --lib agents::tests::seam:: 3` |
| agent-poller: The three seam modules never block on the render path | `NOBLOCK` legs 1–3 with Guards A–E and six positive controls | check | source tree | `sh $CHECKS/NOBLOCK.sh` |
| agent-poller: The first drain polls immediately and the second does not | `agents::tests::worker::the_first_drain_polls_immediately` via `poller_for_test` | unit | scratch `#!/bin/sh` (real), real thread | `testcount --lib agents::tests::worker:: 7` |
| agent-poller: A poll in flight suppresses the next request and reports no deadline | `agents::tests::worker::a_poll_in_flight_suppresses_the_next` | unit | scratch program (real), real thread | `testcount --lib agents::tests::worker:: 7` |
| agent-poller: The interval is one second and is asserted, not assumed | `agents::tests::seam::poll_interval_is_one_second` | unit | none | `testcount --lib agents::tests::seam:: 3` |
| agent-poller: A dead worker is reported once and then stops being reported | `agents::tests::worker::a_dead_worker_is_reported_once` | unit | dropped `Sender` (real channel) | `testcount --lib agents::tests::worker:: 7` |
| agent-poller: A started poller polls a scratch program and yields its agents | `agents::tests::worker::a_started_poller_yields_the_scratch_programs_agents` | unit | scratch program (real), real thread | `testcount --lib agents::tests::worker:: 7` |
| agent-poller: A scratch program that fails is a reachable-false snapshot | `agents::tests::worker::a_failing_scratch_program_is_unreachable` | unit | scratch program (real), real thread | `testcount --lib agents::tests::worker:: 7` |
| agent-poller: An unreachable socket is recovered from, not backed off | `agents::tests::worker::a_failed_poll_is_recovered_from`, a counter-file scratch program failing once then succeeding | unit | scratch program (real), real thread | `testcount --lib agents::tests::worker:: 7` |
| agent-poller: Dropping the poller stops the thread | `agents::tests::worker::dropping_the_poller_stops_the_thread`, deadline-bounded `recv_timeout` | unit | real thread | `testcount --lib agents::tests::worker:: 7` |
| agent-poller: A polled snapshot changes no pixel at either width | `ui::view::tests::agents_change_no_pixel` at 120 and 60 | view | `TestBackend` (replaced) | `testcount --lib ui::view::tests::agents_ 1` |
| agent-poller: `Dashboard` gains a field and every site is forced to name it | `NODEFAULT-UI` run twice (default control file, then `HOMEFILE=src/agents.rs`) plus the seven exhaustive-destructuring companions in `ui::app::tests::` | check + unit | source tree | `sh $CHECKS/NODEFAULT-UI.sh`; `HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot' SCAN_MIN=<measured> sh $CHECKS/NODEFAULT-UI.sh`; `testcount --lib 'ui::app::tests::' 59` |
| agent-poller: `Dashboard` still carries no thread, channel, or clock | `ui::app::tests::dashboard_is_clone_and_eq_with_agents` | unit | none | `testcount --lib ui::app::tests::dashboard_is_clone 1` |
| agent-poller: The real wiring polls a scratch Herdr and adopts what it says | `ui::tests::wiring::the_real_wiring_polls_a_scratch_herdr` at 120 and 60 | acceptance | scratch tree, `notify`, both scratch programs, two real threads (all real); terminal and events replaced | `testcount --lib ui::tests::wiring:: 3` |
| agent-poller: The wiring test fails when the poller is replaced by the inert double | three plants (`agents::none()`, `watch::none()`, `refresh::none()`), each run and reverted, each recorded verbatim | plant | as above | `testcount --lib ui::tests::wiring:: 3` under each plant |
| agent-poller: An unreachable scratch Herdr leaves the pane a working standalone TUI | `ui::tests::wiring::an_unreachable_scratch_herdr_is_a_standalone_tui` at 120 and 60 | acceptance | as above, `herdr` path absent | `testcount --lib ui::tests::wiring:: 3` |
| agent-poller: A pane with no OpenSpec repository still polls for agents | `ui::tests::wiring::no_repository_still_polls_for_agents` | acceptance | scratch `herdr` (real), inert watcher and worker, terminal and events replaced | `testcount --lib ui::tests::wiring:: 3` |
| agent-poller: The residue left in `run` is small enough to read | `WIRED` check: seven required names in `src/ui/mod.rs`'s production slice plus a positive control on `src/cli.rs` | check | source tree | `sh $CHECKS/WIRED.sh` |
| dashboard-loop: `Dashboard` has no `Default` and no site elides a field | `NODEFAULT-UI`, both invocations, plus four planted violations | check | source tree | `sh $CHECKS/NODEFAULT-UI.sh` |
| dashboard-loop: The pure view files name no I/O API | `READONLY-UI` re-run with `src/agents.rs` added to its `EXTRA` list, plus a planted `std::fs::write` in that file's production slice | check | source tree | `UI_MIN=11 EXTRA='src/watch.rs src/refresh.rs src/agents.rs' sh $CHECKS/READONLY-UI.sh` |
| dashboard-loop: The shell never names the CLI seam | `NOCLI-SHELL` at `UI_MIN=11`, re-run after `src/ui/mod.rs` is wired to a second binary | check | source tree | `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh` |
| dashboard-loop: Change literals live only in the gated file | `NOLIT-CHANGE` at `MIN=22` with a planted `Change {` in `src/agents.rs` | check | source tree | `MIN=22 sh $CHECKS/NOLIT-CHANGE.sh` |
| dashboard-loop: The render path names no channel, thread, lock, or clock | `NOBLOCK` all three legs, Guards A–E, six positive controls, six planted violations | check | source tree | `sh $CHECKS/NOBLOCK.sh` |
| dashboard-loop: No test sleeps and then asserts something has already happened | `NOSLEEP` at `SLEEP_MIN=4`, both legs, with its self-contained negative control | check | source tree | `SLEEP_MIN=4 MIN=25 sh $CHECKS/NOSLEEP.sh` |
| dashboard-loop: The first frame is on screen before the first event is read | landed `ui::driver::tests::` test, amended only to build a three-field `Live` | unit | `TestBackend`, inert seams (replaced) | `testcount --lib ui::driver::tests:: 29` |
| dashboard-loop: Timeouts are not events and do not end the loop | landed `ui::driver::tests::` test, amended for the third field | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| dashboard-loop: A backend draw failure ends the loop rather than spinning | landed `ui::driver::tests::` test, amended for the third field | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| dashboard-loop: Ctrl-C ends the loop | landed `ui::driver::tests::` test, amended for the third field | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| dashboard-loop: An ignored key redraws and keeps waiting | landed `ui::driver::tests::` test, amended for the third field | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| dashboard-loop: A route change is visible in the next frame | landed `ui::driver::tests::` test, amended for the third field | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| dashboard-loop: `Live` cannot be built without naming the poller | the crate compiling at all, plus `NODEFAULT-UI` extended over `Live` | compile | none | `cargo test --all-features` (the companion is a compiled test) |
| live-updates: The startup request is issued before the first wait | landed `ui::driver::tests::` test, amended for the third field | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| live-updates: A result is adopted before the frame that shows it | landed `ui::driver::tests::` test, amended for the third field | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| live-updates: A filesystem batch becomes one selection | landed `ui::driver::tests::` test, amended for the third field | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| live-updates: A watcher error is recorded once and the loop continues | landed `ui::driver::tests::` test, amended for the third field | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| live-updates: The wait shortens to the debounce deadline | landed `ui::driver::tests::` test, amended so the poller's `pending_in` is `None` | unit | replaced | `testcount --lib ui::driver::tests:: 29` |
| live-updates: The wait shortens to whichever poller is due first | `ui::driver::tests::the_wait_takes_the_soonest_of_two_deadlines`, exact equality on three recorded timeouts | unit | `ScriptedFs`, `ScriptedAgents`, `Script` (all replaced) | `testcount --lib ui::driver::tests::the_wait_takes_the_soonest 1` |
| live-updates: An agent snapshot reaches the frame that consumed it and survives a refresh | `ui::driver::tests::a_snapshot_reaches_the_frame_and_survives_adopt` at 120 and 60 | unit + view | replaced | `testcount --lib ui::driver::tests::a_snapshot_reaches 1` |
| live-updates: An unreachable socket never becomes a problem row | `ui::driver::tests::an_unreachable_socket_is_not_a_problem_row` at 120 and 60 | unit + view | replaced | `testcount --lib ui::driver::tests::an_unreachable_socket 1` |
| live-updates: An inert live tier leaves the loop exactly as it was | the landed `ui::driver::tests::` and `ui::tests::live::` suites passing unchanged with `agents::none()` added | unit | replaced | `testcount --lib ui::driver::tests:: 29`; `testcount --lib ui::tests::live:: 3` |
| subprocess-seam: The binding spawns the program it was given | `cli::tests::agent_cli_via_spawns_what_it_was_given`, three scratch programs | unit | scratch `#!/bin/sh` (real) | `testcount --lib cli::tests::agent_cli 3` |
| subprocess-seam: The default program name is written down once | `cli::tests::herdr_program_is_the_bare_name` plus a grep for the literal outside `src/cli.rs` | unit + check | none | `testcount --lib cli::tests::herdr_program 1`; `sh $CHECKS/WIRED.sh` |
| subprocess-seam: The shell still names no CLI trait | `NOCLI-SHELL` at `UI_MIN=11` | check | source tree | `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh` |
| watch-invalidation: The timeout is the tick when nothing is pending | landed `watch::tests::` test, unchanged | unit | none | `testcount --lib watch::tests:: 31` |
| watch-invalidation: The timeout is the remaining window when that is shorter | landed `watch::tests::` test, unchanged | unit | none | `testcount --lib watch::tests:: 31` |
| watch-invalidation: One tick serves two pollers | `watch::tests::soonest_*`, four combinations plus the two `poll_timeout` compositions | unit | none | `testcount --lib watch::tests::soonest 4` |

**Test-count arithmetic.** Every target is the count measured on `main` plus the number of new
test functions the group enumerates, written out so it can be checked rather than trusted.
Measured on `main` at the base commit: **752** library tests in total.

| Filter | Measured | New | Target |
|---|---|---|---|
| `agents::tests::parse::` | 0 | 13 | 13 |
| `agents::tests::poll::` | 0 | 6 | 6 |
| `agents::tests::seam::` | 0 | 3 | 3 |
| `agents::tests::worker::` | 0 | 7 | 7 |
| `cli::tests::` | 37 | 3 | 40 |
| `watch::tests::` | 27 | 4 | 31 |
| `ui::app::tests::` | 55 | 4 | 59 |
| `ui::view::tests::` | 81 | 1 | 82 |
| `ui::driver::tests::` | 22 | 7 | 29 |
| `ui::tests::wiring::` | 0 | 3 | 3 |
| `testutil::tests::` | 8 | 2 | 10 |

Library total: `752 + 13 + 6 + 3 + 7 + 3 + 4 + 4 + 1 + 7 + 3 + 2` = **805**. A scenario that
*modifies* an existing test adds nothing to the count, and thirteen of this change's scenarios
do exactly that — every landed `ui::driver::tests::` scenario is amended only to build a
three-field `Live`.

**Coverage.** Measured on `main` at the base commit with `cargo llvm-cov --summary-only`:
**97.16% of lines, 18,898 lines total, 537 uncovered** — the **line** figure, not the region
count, which is 31,485 and is not interchangeable with it. The floor stays
`--fail-under-lines 80` and is neither lowered nor given an exclusion; the untestable residue
this change adds is `cli::agent_cli_via`'s production caller in `ui::run` and the `HERDR_PROGRAM`
literal, both inside the shrunken `run` that has no branch to miss.

## Decisions

**1. A worker thread, not a synchronous spawn on the render path.** Measured, `herdr agent list`
costs 8ms median. Alternatives: call it from `AgentPoll::drain` directly (rejected — it makes
`NOBLOCK`'s "the render path blocks on nothing but the terminal" false by design, and the check's
leg 3 would need an exemption for the one module most likely to need it); or call it from the
existing refresh worker (rejected — that worker's cadence is event-driven and its cycle takes
200–400ms, so a 1s agent poll would either stall behind a CLI cycle or force the worker to
multiplex two unrelated schedules). `cli::HerdrCli: Send + Sync` exists for exactly this, and
`refresh` already proved the shape.

**2. The schedule lives on the render side, the spawn on the worker side.** `RealAgentPoll::drain`
reads the clock once, decides whether a poll is due, sends at most one request, and leaves a
`Duration` behind for `pending_in`. Alternative: let the worker sleep `POLL_INTERVAL` between
polls and have `pending_in` always return `None` (rejected — `NOBLOCK` leg 2 forbids a clock
under `src/ui/`, so the loop would have no way to shorten its wait for the poller, and the
handoff's "two pollers, one tick" constraint would be unsatisfiable). The cost is that
`src/agents.rs` becomes the crate's **second** clock binding, which `SPEC.md` and `AGENTS.md`
are corrected to say.

**3. `watch::soonest` rather than a third `poll_timeout` parameter or an inline `min`.**
Alternatives: widen `poll_timeout` to take two pendings (rejected — it re-opens every landed
assertion on a function this change has no business changing); take the minimum inline in
`src/ui/driver.rs` (rejected — it would be exercised only through frame counts, which pass
whichever way it is written, and this repository's dominant defect class is exactly a check that
cannot fail). `soonest` is asserted over all four presence combinations directly.

**4. `Dashboard.agents` is a sibling field carrying `agents::AgentSnapshot`, not a new
`ui::app::Agents` type.** A second struct would duplicate three fields and force a conversion
whose only purpose is to satisfy `NODEFAULT-UI`'s hardcoded control file. Parameterising that
control file instead is a smaller edit to a check the repository already strengthens
deliberately, and it brings `Agent` — a struct with eight mostly-optional fields, the exact shape
`..Default::default()` is reached for — under the same gate.

**5. No backoff after a failure.** Alternatives: exponential backoff, or stopping permanently on
`CliError::NotStarted`. Both rejected: the Herdr server can stop and restart while a pane is
open, and a poller that slowed or stopped would leave the pane blind to a server that came back.
An absent binary costs one `ENOENT` per second, microseconds each, against an 8ms successful
call. Simplicity wins and there is no recovery hole.

**6. A failed poll clears the agent list rather than keeping the last one.** This is in tension
with `SPEC.md` → Degraded states' "A CLI cycle fails after the worker already sent its
file-sourced result | the pane keeps the numbers the file read produced", and the tension is
resolved by the more specific row: "Herdr socket unreachable | agent column and action keys
hidden". Hidden, not stale. A stale agent badge pointing at a pane that no longer exists is
worse than no badge.

**7. `ui::run` is split into `run` / `run_wired` / `start_collaborators`.** Alternatives: leave
`run` whole and cover the wiring with a source-level grep alone (rejected — a grep proves a name
appears, not that its result reaches `Live`; `live-refresh`'s bug would have survived a grep for
`watch::start` if the call had existed and its result had been dropped); or inject the
collaborators into `run_wired` as values (rejected — then `start_collaborators`, the function the
bug actually lived in, is the untested one again). Splitting so that the untested residue holds
no branch, no loop, and no field selection is the smallest honest answer, and the `herdr` program
becomes a parameter for the same reason `npm_prefix_via`'s is.

**8. `Startup` is a struct.** `run_wired` would otherwise take seven parameters — terminal,
events, cwd, config, herdr, read, tick — which is clippy's `too_many_arguments` threshold
exactly. Bundling the three startup inputs takes it to five and follows `Live`'s own precedent.

**9. The outer-loop test waits on a predicate, not a sleep.** `testutil::UntilReady` yields
`Ok(None)` — after a `std::thread::yield_now()`, which has no duration and so is deliberately
outside `NOSLEEP`'s pattern — until a caller-supplied predicate has held for a settle window, or
until a deadline passes, and then presses `q`. Its clock lives in `src/lib.rs`, never under
`src/ui/`, so `NOBLOCK` leg 2 stays green. The deadline is the backstop that turns a wiring
regression into a red assertion rather than a hung suite.

**10. `openspec` is reached through `Config::openspec_bin` in the acceptance test.** The probe
chain's first step is a configured path, so a **usable** scratch program there stops the chain
and the installed `openspec` binary is never spawned. Alternative: mutate `PATH` (rejected —
`std::env::set_var` is `unsafe` in edition 2024 and races parallel tests, which `AGENTS.md`
forbids outright).

**11. `Agent` omits `terminal_id`, which Herdr's schema marks required.** Nothing in Phase 5
addresses a terminal: `herdr agent focus`, `herdr agent prompt`, and `herdr pane split` take a
pane or an agent name, which `pane_id` and `name` already carry. Alternative: carry all seven
required fields (rejected — a field no consumer reads is a field every construction site must
still fill, and `NODEFAULT-UI` makes that cost real). If `agent-launch` turns out to need it, it
adds one field and one line to the parser.

**12. `NODEFAULT-UI`'s half B is repaired, not merely parameterised.** Measured on `main` at
planning time: with a `..d` elision planted in `src/ui/app.rs`, the check printed its half-B FAIL
line to stderr and then fell through to its final `echo` and **exited 0**. A shell heredoc does
not propagate the interpreter's status, so half B has been unable to fail since `detail-view`
rewrote it as a python pass, and every task judging the check by exit status has been passing over
it. The repair is `|| exit 1` on the heredoc's command line. This is recorded as a decision rather
than as a task footnote because it changes what a landed gate means for every future change, not
only this one. Alternative: leave it and add a separate check (rejected — two versions of one
check on disk is how a run and a record drift apart).

**13. `make check` is suspended between groups 2 and 10, in favour of its four sub-commands with
`cargo llvm-cov --ignore-run-fail`.** The outer-loop acceptance test is deliberately red for that
span and `cargo llvm-cov` hard-fails on any failing test, so the composite target cannot be the
gate there. The cost is that a *second* failing test during the span is only distinguishable by
reading the run's output, which is why every group in the span states the exact expected failure
count. Alternative: place the acceptance test last (rejected — it stops being an outer loop, and
the wiring defect it exists to catch is exactly the one that survives being tested at the end).

**14. The `openspec` invocation count, not `refresh.problems`, is the watcher's discriminator.**
See Test Strategy. Alternative: assert on `refresh.problems` (rejected as unfalsifiable — both
arms produce an empty vector); alternative: expose a watcher-started flag on `Dashboard`
(rejected — state added only to make a test pass, and `dashboard-loop` forbids the state value
carrying anything about the watcher).

## Risks / Trade-offs

- **The acceptance test waits on real threads and real subprocesses, and could flake** → the wait
  is a predicate poll with a settle window and a deadline, never a fixed sleep; the predicate is
  a file the scratch program writes, so it is a fact rather than an elapsed time; the deadline
  path fails the assertion loudly instead of hanging; and every wait shape is the one
  `tests/cli.rs` and `watch::tests` already use.
- **`src/agents.rs` becomes the crate's second clock binding, weakening a stated invariant** →
  the invariant is corrected in `SPEC.md` and `AGENTS.md` in this change rather than left to
  drift, and `NOBLOCK` leg 2's actual guarantee — no clock under `src/ui/` — is unchanged and
  still checked.
- **`NOBLOCK` leg 3's file list is literal, and a third module could be added to the tree without
  being added to the list** → the list is extended here, and Guard D's loop and Guard E's
  ordering rule are extended with it; the extension is proven by planting a `recv_timeout` in
  `RealAgentPoll::drain` and observing red.
- **Guard E under `head -1` silently passes a moved implementation** → `agents`' guard takes the
  **last** `fn drain` line, and the plant that proves it is moving `RealAgentPoll::drain` below
  `start` and observing red, exactly the repair `live-refresh`'s Change Review made for
  `src/refresh.rs`.
- **A poll every second for the pane's whole life is a subprocess per second** → measured at 8ms
  and off the render path; `SPEC.md` already sanctions the cadence; and the alternative, an event
  hook, targets event names Herdr does not confirm.
- **Herdr could change the payload shape in a later release** → every field is optional in the
  parser except `pane_id`, `tab_id`, and `workspace_id` — three of the seven Herdr's own schema
  marks required, and the three this crate reads; an unrecognised
  status decodes to `Unknown` with no problem recorded; and an unusable payload degrades to a
  reachable-false snapshot rather than to a panic.
- **`Dashboard` gaining a tenth field touches every construction site** → that is the point;
  `NODEFAULT-UI` and the exhaustive-destructuring companions make it a compile error rather than
  a silent default.
- **The `ui::run` split is the largest edit to a landed file in this change** → its signature is
  unchanged, its one consumer is `lib::run`, and the split is covered by the acceptance test that
  did not exist before it.

## Migration Plan

None. There is no persisted state, no stored format, and no protocol version. The change is
additive at every interface, so a rollback is a revert: nothing downstream reads the new field
yet, and the plugin behaves exactly as `live-refresh` left it if the poller is removed. Deploy
order is a single `make build`.

## Open Questions

None. The three questions that would otherwise be open were resolved by running Herdr 0.8.2
live rather than by reading `SPEC.md`: the `--json` flag does not exist, the payload is an
envelope carrying a `name` distinct from `agent`, and an unreachable socket reports itself on
stderr with exit status 1. Each is recorded as a `SPEC.md` correction this change carries.

**Visual design source:** none. This change renders nothing and modifies no view; the section is
skipped deliberately rather than invented.
