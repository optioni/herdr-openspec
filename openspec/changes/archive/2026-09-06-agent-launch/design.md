## Context

`agent-polling` opened the socket and `agent-attribution` gave the pane a meaning for what it
found there. Both are read-only. `agent-launch` is the change where a keypress reaches out of the
process, and every design decision below is downstream of that one fact.

Four things shape it, three of them measured against **Herdr 0.8.2 running live** on the
reference machine rather than read off `SPEC.md` — which the two prior Phase 5 changes each
found wrong about this socket, and which is wrong again here:

1. **`herdr pane split` requires `--direction`.** Without it, exit **2** and
   `usage: herdr pane split [<pane_id>|--pane ID|--current] --direction right|down …`, and no
   pane is created. `SPEC.md`'s launch flow already carries `--direction right --no-focus`; what
   it does not say is that the flag is mandatory rather than cosmetic.
2. **`pane split` returns an envelope, not a pane id.** Measured:
   `{"id":"cli:pane:split","result":{"pane":{…,"pane_id":"wD:pJ",…},"type":"pane_info"}}`.
   `SPEC.md`'s flow diagram writes `-> pane_id`, which is the value, not the payload. With no
   pane argument the split targets the **focused** pane — measured — which is the dashboard's own
   pane whenever one of its keys was pressed, so the roadmap's argument vector is right and the
   reason it is right is worth writing down.
3. **Every one of these commands reports failure on stderr with exit 1**, as a JSON error
   envelope: `invalid_agent_name`, `agent_name_taken`, `agent_pane_not_found`, `agent_not_found`,
   each measured. A *usage* error exits 2 instead. That is the mirror of the OpenSpec CLI, whose
   diagnostic goes to stdout and is therefore unavailable to `CliError::Failed` — so for Herdr
   the reason **is** available and the pane should carry it, exactly as
   `agents::herdr_error_problem` already does.
4. **`herdr agent start` blocks for up to thirty seconds** waiting for interactive readiness.
   That single fact decides the whole shape below: the launcher cannot be a synchronous call
   from the render path, and must be the crate's third worker thread.

Two further measurements settle questions the design would otherwise have guessed at.
`herdr agent focus` resolves a **pane id** or an agent **name**, but not a terminal id; and
`herdr agent start`'s name validation fires **before** it looks at the pane, rejecting anything
outside `[a-z][a-z0-9_-]{0,31}` — which is the cap `state::agent_name` already derives against
and `plugin-config` already ships a recorder for.

The tree this lands on is disciplined about three things that bear directly here. Views are pure
functions of `Dashboard`; `src/cli.rs` is the only module that may spawn; and `HANDOFF.md`'s
Phase 5 constraint 4 puts a standing condition on a `Dashboard` sibling field, never on
`ChangeSet::problems`, which `adopt` replaces wholesale.

And the failure mode `live-refresh` shipped is still the live one, in its worst form yet.
`ui::run` called neither `watch::start` nor `refresh::start`; every test passed; the whole live
tier would have been permanently inert. `agent-polling` answered with an outer-loop acceptance
test driving the real composition root, and `agent-attribution`'s reviewers then caught a
`WIRED` leg guarding the wrong link — a shipped `state_dir: None` would have left tier 1 dead
with every gate green. Here the exposure is not that a background tier is silent but that **a key
the reader presses reaches nothing**, which no unit test over `decide`, `launch::start`, or
`run_loop` can see.

## Goals / Non-Goals

**Goals:**

- One new module, `src/launch.rs`, outside `src/ui/`, holding the whole launch policy as pure
  functions plus one worker thread — nothing spawns outside `src/cli.rs`.
- Three sequential Herdr calls whose second and third demonstrably carry the first's output, with
  the argument vectors fixed and asserted verbatim.
- A `Dashboard::apply` that stays a pure function of `&mut self` while four new actions
  eventually start a process, because what `apply` produces is **data**.
- A refusal path that reaches Herdr **not at all** — no pane created — for the one failure a
  reader will hit repeatedly: pressing `a` twice on the same change.
- Action keys and their footer hints conditioned on `Dashboard::agents.reachable` and nothing
  else.
- `g` whose target is the pane of the agent the badge is describing, by construction rather than
  by coincidence.
- An outer-loop acceptance test that feeds the **real** `ui::run_wired` an actual `a` key event
  and observes the three invocations in order at the seam, and an actual `g` event and observes
  the fourth — with a recorded red plant per link.
- `SPEC.md`, `README.md`, and one shipped doc comment corrected where measurement disagrees with
  them.

**Non-Goals:**

- **No orchestration.** One key, one agent, one change; no launch-all, no queue, no "next
  change". `PRD.md` → Non-goals reserves that for the orchestrator agents.
- **No authoring.** Only `/opsx:apply`, `/opsx:continue`, and `/opsx:archive` are bound — the
  three that act on a change that already exists.
- **No `herdr pane close`, ever.** A pane this plugin created and could not use is left in place.
- **No key that dismisses a launch problem**, no confirmation prompt, no launch menu, no
  per-agent listing, no `herdr worktree list`, no manifest change, no configuration key, no new
  dependency, no new file under `src/ui/`.
- **No change to `Change`, `ChangeSet`, `from_files`, or `from_cli`**, and no fifth argument to
  `agents::attribute`.
- **No repair of `DEPS` / `GRAPH-SNAP`.** They have been red on `main` since `live-refresh` added
  `notify` without updating either; this change adds no dependency and edits neither script. See
  planning-review.md → Known-red inherited conditions, which also records that `HANDOFF.md`
  assigns that repair here and why it is declined.

## Boundaries

| Piece | Module | Pattern it follows |
|---|---|---|
| `Intent`, `Request`, `Decision`, `Outcome`, `decide` | `src/launch.rs` (new) | `agents::attribute` — plain data in, plain data out, no `Default` on the struct, every field named at every site, never a panic |
| `split_args`, `start_args`, `prompt_args`, `focus_args`, `prompt_text` | `src/launch.rs` | `agents::poll_once`'s `&["agent", "list"]` — the argument vector written once, beside its only caller |
| `pane_id` | `src/launch.rs` | `agents::parse_list` — `serde_json::Value` navigation, `Err(String)` naming what was missing, Herdr's own error envelope reported by `code` and `message` |
| `run_request`, the failure mapping | `src/launch.rs` | `agents::herdr_error_problem` — stderr and the OS reason carried verbatim, degrade never fail |
| `Launcher` trait, `none()` | `src/launch.rs` | `agents::AgentPoll` — non-blocking, an inert implementation, `Send` |
| `RealLauncher`, `start`, `worker_body` | `src/launch.rs` | `refresh::RealRefresher`/`start`/`worker_body` — one channel in, one out, `try_recv`, exactly one `thread::spawn`, worker body written below it |
| `Attribution::panes` | `src/agents.rs` | `Attribution::badges` — the same fold, updated in lockstep by the same comparison |
| `Action`'s four variants, the four `apply` arms | `src/ui/app.rs` | `Action::Refresh` and its arm — a one-shot flag on a sibling field, reaching no collaborator |
| `Dashboard::launch` | `src/ui/app.rs` | `Dashboard::refresh` — a plain state field of one-shot flag plus problem text, replaced never grown |
| `Live::launcher`, the two loop steps | `src/ui/driver.rs` | `Live::agents` and `drive_live_tier`'s drain — a fourth field on the struct, not a seventh parameter |
| `Collaborators::launcher`, `start_collaborators`'s fourth parameter | `src/ui/mod.rs` | `Collaborators::agents` and `Startup::herdr` — the environment-dependent value arrives as a parameter so a test drives it |
| The launch problem rows | `src/ui/list.rs` | `rows`' existing `refresh.problems` loop — a third source, same `! `-prefixed grammar, same `RowKind::Problem` |
| The two footer hints | `src/ui/view.rs` | `view::fit_hints` — two more entries in the hint list `render_footer` already builds and drops from the end |
| `RecordingLauncher`, `ScriptedLauncher`, `Stages`, the scratch `herdr` script's launch branches | `src/lib.rs` `testutil` + `ui::tests::wiring` | `RecordingRefresher`, `ScriptedAgents`, `UntilReady`, and `agent-polling`'s `herdr_script` |

**Why the launcher lives in `src/launch.rs` and not under `src/ui/`.** Three landed checks decide
it and none of them is negotiable. `NOCLI-SHELL` forbids every file under `src/ui/` from naming
`HerdrCli`, and the launcher's `start` takes an `Arc<dyn HerdrCli>` in its own signature.
`NOBLOCK` leg 1 forbids `src/ui/driver.rs` from naming a channel or a thread, and the launcher is
a thread with two channels. `READONLY-UI` forbids a write API under `src/ui/`, and the launcher
writes — through `state::record`, but it is still the module with a reason to. `SPEC.md` →
Architecture's module map has named a `launch` module since `repo-foundation`; this is the change
that creates it. It sits beside `src/watch.rs`, `src/refresh.rs`, and `src/agents.rs` for exactly
the reason those three do.

**Why `decide` is a pure function in `src/launch.rs` rather than a method on `Dashboard`.**
`agents::attribute` is the precedent, and the argument is identical: the whole launch **policy**
— what an unreachable socket means, what an empty selection means, what a name collision means —
is testable with no `Dashboard`, no `ChangeSet`, and no fixture, from five plain arguments.
`Dashboard::apply`'s four new arms become a two-line adapter each. It also keeps
`state::agent_name` out of `src/ui/`, which matters because `NOIO-VIEW`'s pattern now guards
`state::record` sitting next to it.

**Why `decide` takes `Option<&str>` for the change name.** `NOLIT-CHANGE` forbids a `Change {` or
`ChangeSet {` literal or pattern anywhere under `src/` except `src/changes.rs`, tests included,
and its pattern catches a `-> Change` signature as readily as a literal. A name slice needs no
fixture at all, and `src/launch.rs` then has no reason to name the type in its production code or
its thirty-six tests. The adapter that turns `selected_change()` into an `Option<&str>` is one
line in `src/ui/app.rs`, where `Change` is already in scope.

**Why `apply` sets a flag and `run_loop` dispatches it.** This is the load-bearing sentence of the
change and it is the direct answer to "the first change where a keypress causes an outward,
side-effecting action". `Dashboard::apply` is a pure function of `&mut self` and its argument, and
it stays one: the four new arms write `launch.pending` and `launch.problems` and nothing else,
reaching no collaborator, spawning nothing, opening nothing, reading no clock. What they produce
is a `launch::Request` — a value, `Clone`, `PartialEq`, comparable in a test with no thread. The
loop's step 1 takes that value and hands it to `Launcher::request`, whose real implementation
lives outside `src/ui/` and whose worker is what actually spawns. So the invariant
"no action mutates a change / a task item" survives unchanged, and it survives **structurally**
rather than by inspection: `tasks-checklist`'s sweep applies all seventeen variants and compares
`changes` field for field, and the sweep's fixture is now required to have a reachable socket and
a non-empty list so the four launch arms genuinely execute rather than short-circuiting.

**Where the "the plugin never writes inside `openspec/`" boundary sits.** At the process boundary,
and nowhere softer. The plugin's own writes are exactly one file, `agent-names.toml`, under
`HERDR_PLUGIN_STATE_DIR`. The agent it starts will edit files inside `openspec/` — that is the
entire point of `/opsx:apply` — and those edits are that process's, made under its own identity
through its own tools, in its own pane, after the plugin's causal contribution ended at
`herdr agent prompt`. The evidence is mechanical rather than rhetorical: the acceptance scenarios
snapshot the whole repository tree around a run in which an agent was launched and assert
byte-identity, against a scratch `herdr` program that starts nothing — which is the only honest
way to measure the plugin's own writes.

**No process spawn is added, anywhere.** `Command::new` still appears in `src/cli.rs` and nowhere
else; `NOSPAWN-GREP` at `MIN=23` and the new `LAUNCHSEAM` leg 1 are what say so rather than this
paragraph. `LAUNCHSEAM` exists precisely because `src/launch.rs` is the file whose *job* is to
open a pane, and so the single most plausible place in the crate for someone to reach for
`std::process::Command` directly.

**No new view is added.** The two footer hints are entries in a hint list `ui::view` already
builds; the launch problem rows are `RowKind::Problem` rows `ui::list` already emits and
`ui::view` already styles. Neither adds a region, a widget, or an I/O call, and both remain pure
functions of `Dashboard`.

**What this change does not touch.** `Change` is unchanged, so `from_files` and `from_cli` need
no new agreement and `changes::conformance::assert_invariants` is untouched. `notify`, the
watcher, and the refresh worker are untouched. `watch::soonest` still takes two arguments.
`herdr-plugin.toml` is untouched, so `min_herdr_version` stays `0.7.0`. Every subcommand used
here predates 0.8.2, but two of the *shapes* were measured on 0.8.2 and were previously believed
otherwise — `--direction` being required, and `pane split` returning an envelope rather than the
bare id `SPEC.md`'s flow diagram showed. A Herdr old enough to return a bare id would fail every
launch at call 1, and that is handled as a **degraded state** rather than by a version bump:
`launch::pane_id` returns `Err`, the launch stops before an agent is started, no mapping is
recorded, and the reason reaches a `!`-marked row. Bumping the floor would be a manifest change
— a second BREAKING claim — on a hypothesis nobody has measured against a 0.7.x binary.
`Cargo.toml` gains nothing.

Two of `openspec/config.yaml` -> `rules.specs`' six mandated boundary states are **not** covered
by this change's scenarios, and that is deliberate rather than an omission: **a missing artifact
file** and **a schema the CLI rejects**. The launch path reads no artifact and consults no
schema — `decide` takes a change *name*, `run_request` takes strings — so both states are
unchanged from `detail-view` and `schema-cli-fallback`, whose scenarios still hold. The other
four (no `openspec/` directory, no active changes, an unreachable socket, and a pane narrower
than 100 columns) each have scenarios here.

## Contracts

Every interface below is **additive** except four, each with one production call site:
`start_collaborators` gains a fourth parameter, `Collaborators` and `Live` each gain a field, and
`Attribution` gains a field.

```rust
// src/launch.rs  (new module)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent { Apply, Continue, Archive, Focus }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Launch { change: String, agent: String, intent: Intent },
    Focus { pane_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision { Nothing, Refuse(String), Go(Request) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub named: Option<(String, String)>,
    pub problem: Option<String>,
}

pub fn decide(
    intent: Intent, change: Option<&str>, pane: Option<&str>,
    reachable: bool, live_names: &[&str],
) -> Decision;

pub fn prompt_text(intent: Intent, change: &str) -> String;
pub fn pane_id(text: &str) -> Result<String, String>;

pub trait Launcher: Send {
    fn request(&mut self, request: Request);
    fn drain(&mut self) -> Option<Outcome>;
}
pub fn none() -> Box<dyn Launcher>;
pub fn start(
    cli: std::sync::Arc<dyn crate::cli::HerdrCli>,
    repo: std::path::PathBuf,
    kind: String,
    state_dir: Option<std::path::PathBuf>,
) -> Box<dyn Launcher>;

// src/agents.rs
pub struct Attribution {
    pub badges: std::collections::BTreeMap<String, AgentStatus>,
    pub panes: std::collections::BTreeMap<String, String>,   // NEW
    pub unattributed: usize,
}
// `attribute`'s signature is unchanged.

// src/ui/app.rs
pub enum Action { /* … thirteen landed … */ LaunchApply, LaunchContinue, LaunchArchive, FocusAgent }
pub struct Launch { pub pending: Option<crate::launch::Request>, pub problems: Vec<String> }
pub struct Dashboard { /* … eleven landed … */ pub launch: Launch }

// src/ui/driver.rs
pub struct Live<'a> {
    pub fs: &'a mut dyn crate::watch::FsEvents,
    pub refresher: &'a mut dyn crate::refresh::Refresher,
    pub agents: &'a mut dyn crate::agents::AgentPoll,
    pub launcher: &'a mut dyn crate::launch::Launcher,   // NEW
}
// `run_loop`'s signature is unchanged, at six parameters.

// src/ui/mod.rs
pub struct Collaborators { /* … four landed … */ pub launcher: Box<dyn crate::launch::Launcher> }
pub fn start_collaborators(
    repo: Option<&Path>, config: &Config, herdr: &Path, state_dir: Option<&Path>,
) -> Collaborators;
// `Startup`, `load`, `run_wired`, and `run` keep their signatures.
```

**Error surface.** None is added at any public boundary. `decide` returns a `Decision` for every
input and has no `Result`. `Launcher::request` returns `()`; `drain` returns
`Option<Outcome>`; neither can fail. `Outcome` carries the failure as text rather than as an
error type, on `AgentSnapshot::problem`'s established terms — a failed launch is a degraded state
the pane renders, not something a caller handles. `pane_id` is the one new `Result`, and it is
private to the module's own worker. `run_wired` returns `StartError` only for the two failures
`run_loop` already produces; a failed launch is never a `LoopError`.

**Pagination and streaming.** None. Each Herdr call is a single round trip returning one payload.

**Compatibility and consumers.** `launch`'s only consumers are `Dashboard::apply` (`decide`,
`Intent`, `Request`, `Decision`), `ui::driver::run_loop` (`Launcher`, `Request`, `Outcome`), and
`ui::mod::start_collaborators` (`start`, `none`). `Attribution::panes`' only consumer is
`Dashboard::apply`'s focus arm. No consumer outside this crate exists.

The call sites each signature change breaks were counted, not estimated:

| Changed item | Production sites | Test sites | Command |
|---|---|---|---|
| `Dashboard { … }` literal | **2** (both `load` branches) | every `ui::` test fixture that builds one — the `NODEFAULT-UI` half-B span count, **174** measured on `main` | `SCAN_MIN=9999 TYPES='Dashboard Filter Detail Refresh' sh $CHECKS/NODEFAULT-UI.sh` |
| `Attribution { … }` literal or pattern | **2**, both in `src/agents.rs`'s production slice (`attribute`'s early return and its final expression) | **0** | `grep -rn 'Attribution {' src/` → 5, of which 3 are not literals at all: the `struct` declaration and two `-> Attribution {` return types whose brace opens a function body |
| `Live { … }` literal | **1** (`run_wired`) | **40** (32 in `src/ui/driver.rs`, 8 in `src/ui/mod.rs`) | `grep -rn 'Live {' src/` → 41 |
| `start_collaborators(` call | **1** (`run_wired`) | **0** — no test calls it directly; the wiring tests reach it through `run_wired` | `grep -rn 'start_collaborators' src/` → 4, one of them a doc comment |
| `Startup { … }` literal | **1** (`ui::run`) | **1** — the shared `run_wired_at` helper | `grep -rn 'Startup {' src/` → 2 |

**A landed doc comment this change corrects.** `src/cli.rs`'s module comment on `CliError`
predicts that "`agent-launch` [drives] four more through one `RealHerdrCli`". It is **five** —
`pane split`, `agent start`, `agent prompt`, `agent focus`, and the `agent list` `agent-polling`
already drives — which strengthens rather than weakens that comment's actual argument, since the
whole point is that without the argument vector all five failures would render identically.

**A landed doc comment this change corrects a second time.** `src/ui/app.rs`'s `Action` enum still
opens "The nine outcomes a terminal event can map to". There have been thirteen since
`live-refresh`. `HANDOFF.md`'s Phase 5 constraint 8 read its number off that stale comment and
concluded `Action` "reaches thirteen variants" with this change; it reaches **seventeen**.

## Persistence and Rollout

- **Migration:** none. `agent-names.toml`'s format is `plugin-state`'s and is written exactly as
  `state::record` already writes it.
- **Backfill:** none. An empty mapping is the supported starting state and is what every pane has
  until its first launch.
- **Seeding:** none.
- **Cache invalidation:** none. `changes::CliCache` is untouched. `Dashboard::agent_names` is
  updated in memory by a launch outcome rather than re-read, which is the only "cache" involved
  and is `agent-attribution` → Decisions 7's stated plan.
- **Index rebuild:** none.
- **Authorization:** none in the plugin. Writing `agent-names.toml` is a write under the plugin's
  own state directory; every Herdr command is authorized by access to the Unix socket, which the
  operating system enforces. The plugin adds no privilege of its own and starts the agent under
  the same user in the same session.
- **Observability:** none added. The launch problem row and the footer hints are the only new
  surfaces and both are rendered rather than logged; the pane has no log destination, and adding
  one would violate "the plugin's own writes are scoped to its state directory".
- **Deployment:** none beyond the normal `make build`. No manifest change, no configuration key
  (`agent_kind` has shipped since `plugin-config`), and no `min_herdr_version` bump.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| The `herdr` program | **real spawn of a scratch `#!/bin/sh` program** at an absolute path, through `cli::agent_cli_via`; never the installed `herdr` binary | replaced by a recording `HerdrCli` fake answering by argument vector; `decide`, `prompt_text`, and `pane_id` reach it not at all |
| The Herdr Unix socket | never reached; the scratch program stands in for the whole round trip | never reached |
| A **real Herdr pane or agent** | **never**, in any tier. The live probing that produced this design's measurements was manual, is recorded in the proposal, and cleaned up every pane and agent it created | never |
| The `openspec` program | **real spawn of a scratch `#!/bin/sh` program**, reached through `Config::openspec_bin` so the probe chain stops at step 1 | not reached; `refresh::none()` or `testutil::RecordingRefresher` |
| The filesystem (repository tree) | **real** — a `testutil::ScratchDir` holding one or two changes, walked by the real `changes::from_files` and read by the real `ui::read_artifact` | **absent** for `launch::tests::decide::`, `::argv::`, and `::pane_id::`, which take strings; **absent** for `ui::list`, `ui::view`, `ui::app`, and `ui::driver` tests, which build `ChangeSet` values through `changes::fixture` |
| The plugin **state directory**, read | **real** — a `testutil::ScratchDir`, read by the real `state::read` from the real `ui::load` | **real scratch tree** in `ui::tests::load::` and `launch::tests::run_request::`; absent everywhere else |
| The plugin **state directory**, written | **real** — the real `state::record` writes `agent-names.toml` into a `testutil::ScratchDir`, and a snapshot pair over that directory asserts exactly one file appears | **real scratch tree** in the `launch::tests::run_request::` tests that assert recording; a **read-only** path (an existing regular file where the directory should be) for the recording-failure test |
| The repository tree, **written to** | **never** by the plugin — every launch scenario takes a `testutil::snapshot` pair over the repository and asserts byte-identity, with a discriminating control that rewrites one byte and asserts the comparison differs | not applicable |
| The filesystem watcher (`notify`) | **real** — `watch::start` over the scratch tree | replaced by `testutil::ScriptedFs`, or absent |
| The refresh worker thread | **real** — `refresh::start` | replaced by `testutil::RecordingRefresher`, or absent |
| The agent poller thread | **real** — `agents::start` against the scratch `herdr` program | replaced by `testutil::ScriptedAgents`, or absent |
| The **launcher** worker thread | **real** — `launch::start` against the scratch `herdr` program | **real** in `launch::tests::seam::`, over a fake `HerdrCli`; replaced by `testutil::RecordingLauncher` / `ScriptedLauncher` in `ui::driver::tests::`; `launch::none()` in every landed loop test |
| The terminal | **replaced** — `ratatui::backend::TestBackend` at 120x20 and 60x20; no real terminal exists in the test process | replaced identically for view tests; absent for `launch`'s and `agents`' own tests |
| The terminal event source | **replaced** — `testutil::Stages`, a staged extension of `UntilReady`: it yields timeouts until each stage's predicate holds, presses that stage's key, and finishes with `q` | replaced by `testutil::Script` |
| The clock | **real**, and only inside `testutil::UntilReady` / `testutil::Stages` in `src/lib.rs` — never under `src/ui/`, never inside a view test, and never inside `src/launch.rs`, which reads no clock at all | not read at all |
| The process environment | **replaced** — `Config` is constructed directly and `Startup::state_dir` is a parameter, so no `std::env::set_var` and no `PATH` mutation | replaced identically |
| The **gated fake `HerdrCli`**'s release channel | not used — the acceptance runs drive a real scratch program, which answers immediately | **real** `std::sync::mpsc` inside `launch::tests::seam::the_real_launcher_answers_on_a_later_drain`, held by the test and released by it; the only synchronisation this change's tests own, and the reason none of them sleeps |
| `testutil::canonical` and the macOS `/private/var` symlink | **real** — every scratch `herdr` payload embeds `testutil::canonical(scratch.path())`, because `attribute`'s containment test compares components and `/var` is a symlink to `/private/var` | not reached |
| Herdr's own JSON schema | **not** consulted at runtime; the shapes are pinned by fixture payloads captured verbatim from Herdr 0.8.2 during this change's own live probing | same fixtures |
| The `openspec` CLI's JSON | untouched by this change; the scratch program answers `list --json` exactly as `agent-polling`'s does | not reached |

## Test Strategy

Three tiers as `openspec/config.yaml` defines them — unit tests over the pure modules; view tests
rendering into a `TestBackend` at 60 and 120 columns; scratch trees and scratch `#!/bin/sh`
programs under `std::env::temp_dir()` — plus this repository's fourth kind of evidence, the
**command-level check**: a shell script with positive controls and a proven-red plant, run from a
task rather than from `cargo test`.

**This change takes the outer-loop acceptance test, and it is the reason the whole plan is shaped
around one.** `ui::tests::wiring::` gains three tests driving `ui::run_wired` — the real
`ui::load`, the real `state::read`, the real `watch::start`, the real `refresh::start`, the real
`agents::start`, the real `launch::start`, the real `ui::read_artifact`, the real `action_for`,
the real `Dashboard::apply`, the real `run_loop`, the real `list::rows`, and the real
`view::render` — against a scratch repository, a scratch state directory, and two scratch
programs. The first **feeds it an actual `a` key event and observes the three Herdr invocations,
in order, at the seam**; the second feeds it `a` then `g` and observes the fourth; the third
feeds it `a` and `g` against an absent `herdr` and observes nothing at all.

The load-bearing assertions are chosen so each can fail, and so that no two plants fail the
identical *set*:

- the scratch `herdr` log's **non-`agent list`** entries are `pane split`, then `agent start`,
  then `agent prompt`, **in order**, and `agent start`'s `--pane` equals the pane id
  `pane split`'s own payload printed.
  The pane-id equality is what proves call 2 read call 1's output rather than a constant.
  Every predicate and every count in these tests filters out `agent list`, because the poller
  writes to the same log on its own one-second cadence and an absolute entry count is therefore
  a race — at `[agent list, agent list, pane split, agent start]` a "four entries" predicate
  fires one call early;
- `--kind` is `codex` in the first test and `gemini` in the second, because the two acceptance
  `Config`s set different `agent_kind` values. **Two** different values is what makes the claim
  discriminating: a `start_collaborators` that hardcoded either one passes the other test, and
  `WIRED` leg 6 half (i) is a presence check that a hardcoded literal beside a named
  `config.agent_kind` would survive;
- `agent-names.toml` in the scratch state directory holds `c-2fa-support = "2fa-support"` — the
  derived name, proving `state::agent_name` ran and `state::record` wrote;
- the returned `Dashboard`'s `agent_names.names` holds the same pair, proving the outcome came
  back through `Launcher::drain` and the loop applied it;
- `launch.pending` is `None` and `launch.problems` is empty at the end;
- the footer carries `a/c/s launch  g focus` at both widths;
- the repository tree is byte-identical, with a discriminating control.

**Six plants are run and observed red during implementation, each reverted and each recorded
verbatim**, and their assertion sets are pairwise distinct by construction:

| Plant | Fails on | Distinguished from the others by |
|---|---|---|
| `agents::none()` in `start_collaborators` | the footer hints (`reachable` is false), the log's three entries, `agent-names.toml`, and `agent_names.names` | it is the only one that turns the **footer** assertion red |
| `launch::none()` in `start_collaborators` | the log's three entries, `agent-names.toml`, `agent_names.names` | `launch.pending` still ends `None` |
| `run_loop` not dispatching `launch.pending` | the same three | `launch.pending` ends **`Some`** |
| `run_request` passing a constant `--pane w0:p0` | the ordered-argv assertion, on the pane id alone | all three log entries are present; only the `--pane` value is wrong |
| `state::record` not called | `agent-names.toml` absent | the log holds all three entries and `agent_names.names` still ends empty |
| `drive_live_tier` dropping `Outcome::named` | `agent_names.names` empty | `agent-names.toml` **is** present, which no other plant leaves |
| `run_request` setting `named` before `agent start` rather than after | `launch::tests::run_request::a_failed_split_leaves_nothing_behind`'s and `::an_unusable_payload_stops_before_agent_start`'s `named is None` bullets | it is the only one every acceptance assertion stays green under — the acceptance run's launch succeeds, so `named` is `Some` either way. It is caught at the unit tier alone, which is why those two bullets exist |

A seventh and eighth are run against the **source** rather than the suite, closing links no test
reaches: `start_collaborators` passing a literal `"claude"` instead of `config.agent_kind`
(caught by `WIRED`'s new leg 6, both halves verified red at planning time), and `src/launch.rs`
reaching for `std::process::Command` directly (caught by `LAUNCHSEAM` leg 1 and by
`NOSPAWN-GREP` at `MIN=23`, both verified red at planning time).

**Group 1 is a structure-only skeleton, before the outer-loop RED**, on
`agent-attribution`'s and `agent-polling`'s pattern and for the same reason: the acceptance test
in group 2 names a four-field `Live`, a twelve-field `Dashboard`, and `launch::Request`, and the
crate would not compile without them — and a tree that does not compile makes every later group's
`testcount` unreachable rather than red, because `cargo llvm-cov --ignore-run-fail` ignores a
failing *run*, never a failing *compile*.

**Widths gates are passed their explicit floors, always.** `agent-polling` invoked `WIDTHS` and
`LISTWIDTHS` **bare** in its final pass, so they ran at the block defaults of 16 and 17 rather
than the floors then in force; `agent-attribution` repaired those two and left `MDWIDTHS`,
`TASKWIDTHS`, and `DETAILWIDTHS` bare. Every widths gate in this plan carries its floor
explicitly: `WIDTHS_MIN=94`, `LIST_MIN=29`, `MD_MIN=24`, `TASK_MIN=16`, `DETAIL_MIN=23` — the
last three at their measured realized counts, which is the first time they have been pinned.

Commands below are `testcount --lib '<filter>' <minimum>` — the repository's wrapper that runs
`cargo test --all-features <filter>` and fails unless at least `<minimum>` tests **passed**,
because a `cargo test` filter matching nothing exits 0. Every filter guarding a single test is
written out in full. Check scripts are run as `sh $CHECKS/<LABEL>.sh` from the extracted,
byte-identical copies task 0.2 writes.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| agent-launch: An unreachable socket makes every action key inert | `launch::tests::decide::an_unreachable_socket_makes_every_key_inert`, all four intents | unit | none | `testcount --lib launch::tests::decide:: 7` |
| agent-launch: No selected change means no launch, and no agent means no focus | `launch::tests::decide::no_change_means_no_launch_and_no_agent_means_no_focus` | unit | none | `testcount --lib launch::tests::decide:: 7` |
| agent-launch: Each launch intent carries its own change and derived name | `launch::tests::decide::each_intent_carries_its_own_change_and_name`, asserting the three differ only in `intent` | unit | none | `testcount --lib launch::tests::decide:: 7` |
| agent-launch: A derived name already live in the session is refused before any Herdr call | `launch::tests::decide::a_live_derived_name_is_refused`, both halves | unit | none | `testcount --lib launch::tests::decide::a_live_derived_name_is_refused 1` |
| agent-launch: Every combination is total | `launch::tests::decide::every_combination_is_total`, four intents over empty strings | unit | none | `testcount --lib launch::tests::decide:: 7` |
| agent-launch: A change name past the 32-character cap is truncated, hashed, and recorded | `launch::tests::run_request::a_name_past_the_cap_is_truncated_hashed_and_recorded`, over a real scratch state directory — the `openspec/config.yaml` concentration point on the 32-character cap | unit | **real** scratch state dir, fake `HerdrCli` | `testcount --lib launch::tests::run_request::a_name_past_the_cap 1` |
| agent-launch: A legal-but-derived name is recorded too | `launch::tests::run_request::a_legal_but_derived_name_is_recorded_too` and `::an_unchanged_name_writes_no_file` | unit | **real** scratch state dir, fake `HerdrCli` | `testcount --lib launch::tests::run_request::a_legal_but_derived_name_is_recorded_too 1`; `testcount --lib launch::tests::run_request::an_unchanged_name_writes_no_file 1` |
| agent-launch: A derived name that collides with a live agent never reaches Herdr | `launch::tests::decide::a_live_derived_name_is_refused`'s collision half, plus `ui::app::tests::a_refused_launch_records_the_reason_and_produces_no_request` asserting `launch.pending` stays `None`. That app-tier test names **no** `HerdrCli`: `NOCLI-SHELL` and `LAUNCHSEAM` leg 3 both forbid the name under `src/ui/`, so the claim that no Herdr call was made is proved there by the absence of a request and at the seam by `launch::tests::` | unit | none | `testcount --lib launch::tests::decide::a_live_derived_name_is_refused 1` |
| agent-launch: A collision Herdr sees anyway is reported with Herdr's own reason | `launch::tests::run_request::a_collision_herdr_sees_is_reported_with_its_reason`, asserting no `pane close` in the fake's log | unit | fake `HerdrCli` | `testcount --lib launch::tests::run_request::a_collision_herdr_sees 1` |
| agent-launch: The three calls appear in order with the split's own pane id | `launch::tests::run_request::the_three_calls_appear_in_order_with_the_splits_pane_id` | unit | fake `HerdrCli`, real scratch state dir | `testcount --lib launch::tests::run_request::the_three_calls_appear_in_order 1` |
| agent-launch: The three calls appear in order with the split's own pane id | `ui::tests::wiring::a_keypress_launches_an_agent` at 120 and 60 | acceptance | scratch tree, scratch state dir, both scratch programs, `notify`, three real threads | `testcount --lib ui::tests::wiring:: 7` |
| agent-launch: A launch is exactly three Herdr calls (the argument vectors themselves) | `launch::tests::argv::`'s five — `split_args_are_exact`, `start_args_are_exact`, `prompt_args_are_exact`, `focus_args_are_exact`, and `a_non_utf8_repo_root_is_lossy` | unit | none | `testcount --lib launch::tests::argv:: 5` |
| agent-launch: `g` focuses the pane of the agent whose status the badge shows (the argument vector) | `launch::tests::focus::a_focus_request_is_one_call` and `::a_failed_focus_is_reported` | unit | fake `HerdrCli` | `testcount --lib launch::tests::focus:: 2` |
| dashboard-loop: Startup state is read from files only (the twelfth field's initial value) | `ui::tests::load::load_initialises_an_empty_launch_tier`, asserting `launch` is `{ pending: None, problems: [] }` on both `load` branches | unit | **real** scratch directory | `testcount --lib ui::tests::load::load_initialises_an_empty_launch_tier 1` |
| agent-launch: Each intent sends its own `/opsx:*` command and nothing else | `launch::tests::prompt::each_intent_has_its_own_opsx_command` and `::the_prompt_text_is_one_argument` | unit | none | `testcount --lib launch::tests::prompt:: 2` |
| agent-launch: An archived change launches on the same terms as an active one | `ui::app::tests::an_archived_change_launches_on_the_same_terms` | unit | none | `testcount --lib ui::app::tests::an_archived_change_launches_on_the_same_terms 1` |
| agent-launch: A well-formed envelope yields the pane id | `launch::tests::pane_id::a_well_formed_envelope_yields_the_pane_id` | unit | none | `testcount --lib launch::tests::pane_id:: 3` |
| agent-launch: Every unusable payload is an error, never a panic and never a guess | `launch::tests::pane_id::every_unusable_payload_is_an_error` (eight inputs) and `::an_error_envelope_is_reported_by_code_and_message` | unit | none | `testcount --lib launch::tests::pane_id:: 3` |
| agent-launch: An unusable payload stops the launch before an agent is started | `launch::tests::run_request::an_unusable_payload_stops_before_agent_start` | unit | fake `HerdrCli`, real scratch state dir | `testcount --lib launch::tests::run_request::an_unusable_payload_stops 1` |
| agent-launch: A failed split leaves nothing behind | `launch::tests::run_request::a_failed_split_leaves_nothing_behind`, with a state-directory snapshot pair | unit | fake `HerdrCli`, **real** scratch state dir | `testcount --lib launch::tests::run_request::a_failed_split 1` |
| agent-launch: A failed start leaves the pane and names it | `launch::tests::run_request::a_failed_start_leaves_the_pane_and_names_it`, asserting no `pane close` entry | unit | fake `HerdrCli` | `testcount --lib launch::tests::run_request::a_failed_start 1` |
| agent-launch: A failed prompt leaves a running, un-prompted agent that is still attributable | `launch::tests::run_request::a_failed_prompt_leaves_a_recorded_agent`, asserting `named` **and** `problem` are both `Some` | unit | fake `HerdrCli`, **real** scratch state dir | `testcount --lib launch::tests::run_request::a_failed_prompt 1` |
| agent-launch: A failed recording does not undo a successful start | `launch::tests::run_request::a_failed_recording_does_not_undo_the_start`, against a state-dir path that is an existing regular file | unit | fake `HerdrCli`, **real** unwritable path | `testcount --lib launch::tests::run_request::a_failed_recording 1` |
| agent-launch: `g` focuses an agent attributed through the mapping | `ui::app::tests::focus_resolves_the_pane_for_each_attribution_tier`, mapping half | unit | none | `testcount --lib ui::app::tests::focus_resolves_the_pane_for_each_attribution_tier 1` |
| agent-launch: `g` focuses an agent attributed through the mapping | `ui::tests::wiring::g_focuses_the_agent_the_launch_started` at 120 and 60 | acceptance | as above, the scratch `herdr` reporting the started agent after `agent start` | `testcount --lib ui::tests::wiring:: 7` |
| agent-launch: `g` focuses an agent attributed by name | `ui::app::tests::focus_resolves_the_pane_for_each_attribution_tier`, name half | unit | none | `testcount --lib ui::app::tests::focus_resolves_the_pane_for_each_attribution_tier 1` |
| agent-launch: `g` on a change no tier could attribute does nothing | `ui::app::tests::g_on_an_unattributed_change_does_nothing`, including the empty-list half | unit | none | `testcount --lib ui::app::tests::g_on_an_unattributed_change_does_nothing 1` |
| agent-launch: `g` with an unreachable socket does nothing | `launch::tests::decide::an_unreachable_socket_makes_every_key_inert`'s `Focus` call, plus `ui::view::tests::an_unreachable_socket_hides_both_hints` at 60 and 120 | unit + view | `TestBackend` (replaced) | `testcount --lib launch::tests::decide:: 7`; `testcount --lib ui::view::tests::an_unreachable_socket_hides_both_hints 1` |
| agent-launch: The inert launcher answers nothing and starts nothing | `launch::tests::seam::the_inert_launcher_answers_nothing` | unit | none | `testcount --lib launch::tests::seam:: 4` |
| agent-launch: The real launcher answers on a later drain, never on the requesting one | `launch::tests::seam::the_real_launcher_answers_on_a_later_drain`, against a **gated** fake `HerdrCli` blocking on a channel the test releases, then a `yield_now` deadline poll — **no `thread::sleep`**, so `NOSLEEP`'s floor stays at five | unit | **real** thread, gated fake `HerdrCli` | `testcount --lib launch::tests::seam::the_real_launcher_answers_on_a_later_drain 1` |
| agent-launch: Dropping the launcher stops its worker | `launch::tests::seam::dropping_the_launcher_stops_its_worker` | unit | **real** thread | `testcount --lib launch::tests::seam:: 4` |
| agent-launch: The action hints appear at both mandated widths when the socket is reachable | `ui::view::tests::the_action_hints_follow_esc_back_when_reachable` | view | `TestBackend` (replaced) | `testcount --lib ui::view::tests::the_action_hints_follow_esc_back_when_reachable 1` |
| agent-launch: An unreachable socket hides both hints at both widths | `ui::view::tests::an_unreachable_socket_hides_both_hints`, with the byte-identity claim | view | replaced | `testcount --lib ui::view::tests::an_unreachable_socket_hides_both_hints 1` |
| agent-launch: The count is dropped before the action hints as the width falls | `ui::view::tests::the_count_is_dropped_before_the_action_hints` at 69, 68, 60, and 120 | view | replaced | `testcount --lib ui::view::tests::the_count_is_dropped_before_the_action_hints 1` |
| agent-launch: The action keys type into the query while filtering | `ui::app::tests::the_four_action_keys_map_and_their_near_misses_do_not`'s filtering half, plus `ui::view::tests::the_action_hints_survive_a_filter` at 60 and 120 | unit + view | replaced | `testcount --lib ui::app::tests::the_four_action_keys_map 1`; `testcount --lib ui::view::tests::the_action_hints_survive_a_filter 1` |
| agent-launch: A refused launch renders one row at both widths | `ui::list::tests::a_launch_problem_is_the_lists_first_row` at 38 and 58, plus `ui::app::tests::a_refused_launch_records_the_reason_and_produces_no_request` for the three-press half | unit | none | `testcount --lib ui::list::tests::a_launch_problem_is_the_lists_first_row 1` |
| agent-launch: A launch problem leads the refresh and change-set problems | `ui::list::tests::launch_refresh_and_change_problems_in_order` at 38 and 58 | unit | none | `testcount --lib ui::list::tests::launch_refresh_and_change_problems_in_order 1` |
| agent-launch: A later success clears an earlier failure | `ui::driver::tests::a_launch_outcome_updates_the_mapping_and_replaces_the_problem` | unit | scripted launcher | `testcount --lib ui::driver::tests::a_launch_outcome_updates_the_mapping 1` |
| agent-launch: Pressing `a` splits a pane, starts an agent, and sends the prompt | `ui::tests::wiring::a_keypress_launches_an_agent` at 120 and 60 | acceptance | scratch tree, scratch state dir, both scratch programs, `notify`, three real threads | `testcount --lib ui::tests::wiring::a_keypress_launches_an_agent 1` |
| agent-launch: Pressing `g` after the launch focuses the pane the launch created | `ui::tests::wiring::g_focuses_the_agent_the_launch_started` at 120 and 60 | acceptance | as above | `testcount --lib ui::tests::wiring::g_focuses_the_agent_the_launch_started 1` |
| agent-launch: An unreachable socket leaves every key inert and the pane a working TUI | `ui::tests::wiring::an_unreachable_socket_leaves_every_key_inert`, with both snapshot pairs | acceptance | as above, `herdr` path absent | `testcount --lib ui::tests::wiring::an_unreachable_socket_leaves_every_key_inert 1` |
| agent-launch: The wiring test fails when the launcher is replaced by the inert double | the seven plants above, each run, recorded verbatim, and reverted | plant | as above | `testcount --lib ui::tests::wiring:: 7` under each plant |
| dashboard-loop: The artifact read is confined to one binding | `READSEAM` at `UI_MIN=10`, its measured realized count, passed **explicitly** — it has been invoked bare since `detail-view` and therefore running at its block default of 7 | check | source tree | `UI_MIN=10 sh $CHECKS/READSEAM.sh` |
| markdown-render: `pulldown_cmark` is named only in `src/ui/markdown.rs` | `MDSEAM` at `MIN=23`, its file count after `src/launch.rs` lands, passed **explicitly** — bare it runs at its block default of 16 | check | source tree | `MIN=23 sh $CHECKS/MDSEAM.sh` |
| agent-attribution: An agent in the repository with no matching name is counted, never assigned | landed `agents::tests::attribute::an_unmatched_in_scope_agent_is_counted`, extended with the empty-`panes` claim | unit | none | `testcount --lib agents::tests::attribute:: 19` |
| agent-attribution: The name tier reads `name` and never the agent kind | landed test, extended with the `panes` pair | unit | none | `testcount --lib agents::tests::attribute:: 19` |
| agent-attribution: A terminal title naming a change attributes nothing | landed test, extended with the empty-`panes` claim | unit | none | `testcount --lib agents::tests::attribute:: 19` |
| agent-attribution: Every empty and absent input is total, not a panic | landed test, extended with `panes` on all five calls | unit | none | `testcount --lib agents::tests::attribute:: 19` |
| agent-attribution: The pane map and the badge map always hold the same keys | `agents::tests::attribute::the_pane_map_and_the_badge_map_agree`, over five landed fixtures | unit | none | `testcount --lib agents::tests::attribute::the_pane_map_and_the_badge_map_agree 1` |
| agent-attribution: Precedence is total and order-independent | landed test, extended so `panes` follows each winner as ranks are peeled | unit | none | `testcount --lib agents::tests::attribute:: 19` |
| agent-attribution: A tie keeps the first agent's pane | `agents::tests::attribute::a_tie_keeps_the_first_agents_pane`, both orders | unit | none | `testcount --lib agents::tests::attribute::a_tie_keeps_the_first_agents_pane 1` |
| agent-attribution: One agent per change carries its own status unchanged | landed test, extended with the five `panes` entries | unit | none | `testcount --lib agents::tests::attribute:: 19` |
| agent-attribution: A refresh that reorders the list moves the badge with its change | landed `ui::app::tests::attribution_follows_adopt_by_name`, extended with `panes` | unit | none | `testcount --lib ui::app::tests::attribution_ 3` |
| agent-attribution: An unreachable socket yields no badge, no count, and no problem | landed `ui::view::tests::an_unreachable_socket_renders_the_agentless_pane`, extended with the empty `panes` and empty `launch.problems` claims | view | replaced | `testcount --lib ui::view::tests::an_unreachable_socket_renders_the_agentless_pane 1` |
| agent-attribution: The `/` filter hides rows without changing the count | landed `ui::app::tests::attribution_ignores_the_filter` and `ui::view::tests::the_count_survives_a_filter`, extended with the reachable arm and the `g`-under-filter claim | unit + view | replaced | `testcount --lib ui::app::tests::attribution_ 3`; `testcount --lib ui::view::tests::the_count_survives_a_filter 1` |
| dashboard-loop: `Dashboard` has no `Default` and no site elides a field | `NODEFAULT-UI` run three times, plus the twelve compile-time destructuring companions and two exhaustive enum matches, and seven recorded plants. Two of the three floors are **measured during implementation**, not at planning time, because the types they sweep do not exist at `BASE`: tasks 7.4 and 5.4 each read the realized count from a `SCAN_MIN=9999` FAIL line and pin it | check + compile | source tree | `SCAN_MIN=<task 7.4's measurement, above 174> TYPES='Dashboard Filter Detail Refresh Launch' sh $CHECKS/NODEFAULT-UI.sh`; `HOMEFILE=src/agents.rs TYPES='Agent Listed AgentSnapshot Attribution' SCAN_MIN=100 sh $CHECKS/NODEFAULT-UI.sh`; `HOMEFILE=src/launch.rs TYPES='Outcome' SCAN_MIN=<task 5.4's measurement, at least 20> sh $CHECKS/NODEFAULT-UI.sh` |
| dashboard-loop: `Dashboard` has no `Default` and no site elides a field | `ui::app::tests::dashboard_names_launch_at_every_site`, an exhaustive destructuring of all twelve fields with no `..` | unit | none | `testcount --lib ui::app::tests::dashboard_names_launch_at_every_site 1` |
| dashboard-loop: The pure view files name no I/O API | `NOIO-VIEW` with `state::record` and `launch::start` added to `IO_RE`, plus a planted `crate::state::record(...)` in `src/ui/app.rs` and a planted `crate::launch::start(...)` in `src/ui/driver.rs`, both observed red, and the unedited block observed **green** under both | check | source tree | `sh $CHECKS/NOIO-VIEW.sh` |
| dashboard-loop: The shell never names the CLI seam | `NOCLI-SHELL` at `UI_MIN=11`, unchanged — this change adds no file under `src/ui/` | check | source tree | `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh` |
| dashboard-loop: Change literals live only in the gated file | `NOLIT-CHANGE` at `MIN=23`, plus a planted `Change {` in `src/launch.rs` observed red — the check that keeps `decide`'s signature an `Option<&str>` | check | source tree | `MIN=23 sh $CHECKS/NOLIT-CHANGE.sh` |
| dashboard-loop: The render path names no channel, thread, lock, or clock | `NOBLOCK`, **edited** — Guard D's file list, an existence guard, a fourth leg-3 arm mirroring the `src/agents.rs` one, and its own Guard E on `tail -1`; all four replacements written out in tasks.md and proven at planning time against six plants, with the unedited block observed **green** under four of them | check | source tree | `sh $CHECKS/NOBLOCK.sh` |
| dashboard-loop: No test sleeps and then asserts something has already happened | `NOSLEEP` at `SLEEP_MIN=5 MIN=26` — the file count moves for `src/launch.rs`, the sleep-site floor does not, because the launcher's seam tests use a gated fake and `yield_now` | check | source tree | `SLEEP_MIN=5 MIN=26 sh $CHECKS/NOSLEEP.sh` |
| dashboard-loop: The four action keys map, and their near misses do not | `ui::app::tests::the_four_action_keys_map_and_their_near_misses_do_not`, both filter modes and the Release/Repeat sweep | unit | none | `testcount --lib ui::app::tests::the_four_action_keys_map_and_their_near_misses_do_not 1` |
| dashboard-loop: A launch action reaches no collaborator and starts no work | `ui::app::tests::a_launch_action_reaches_no_collaborator_and_starts_no_work`, asserting the other nine fields equal | unit | none | `testcount --lib ui::app::tests::a_launch_action_reaches_no_collaborator 1` |
| dashboard-loop: A refused launch records the reason and produces no request | `ui::app::tests::a_refused_launch_records_the_reason_and_produces_no_request`, three presses, one entry | unit | none | `testcount --lib ui::app::tests::a_refused_launch_records_the_reason 1` |
| dashboard-loop: An unreachable socket makes the four keys change nothing | `ui::app::tests::an_unreachable_socket_makes_the_four_keys_change_nothing`, whole-value equality | unit | none | `testcount --lib ui::app::tests::an_unreachable_socket_makes_the_four_keys 1` |
| dashboard-loop: Both quit keys quit and neither near-miss does | landed test, extended: bare `c` is now `LaunchContinue` where it was `Ignore` | unit | none | `testcount --lib ui::app::tests:: 72` |
| dashboard-loop: A released quit key does not quit | landed test, unchanged | unit | none | `testcount --lib ui::app::tests:: 72` |
| dashboard-loop: Enter and Esc move between the two routes | landed test, extended with the `launch` fields' invariance | unit | none | `testcount --lib ui::app::tests:: 72` |
| dashboard-loop: `Esc` dismisses one layer at a time | landed test, extended with the third dashboard whose launch problem survives four `Back`s | unit | none | `testcount --lib ui::app::tests:: 72` |
| dashboard-loop: Navigation and filter keys are distinguished from near misses | landed test, unchanged | unit | none | `testcount --lib ui::app::tests:: 72` |
| dashboard-loop: Non-key events are ignored without panicking | landed test, extended with `Event::Paste("a")` | unit | none | `testcount --lib ui::app::tests:: 72` |
| dashboard-loop: A pending launch request is handed over exactly once | `ui::driver::tests::a_pending_launch_request_is_handed_over_exactly_once`, exact equality on the recorded request vector | unit | recording launcher | `testcount --lib ui::driver::tests::a_pending_launch_request_is_handed_over_exactly_once 1` |
| dashboard-loop: A launch outcome updates the mapping and replaces the problem | `ui::driver::tests::a_launch_outcome_updates_the_mapping_and_replaces_the_problem` | unit | scripted launcher | `testcount --lib ui::driver::tests::a_launch_outcome_updates_the_mapping 1` |
| dashboard-loop: A quit on the same event as a launch dispatches nothing | `ui::driver::tests::a_quit_on_the_same_event_as_a_launch_dispatches_nothing`, both orders | unit | recording launcher | `testcount --lib ui::driver::tests::a_quit_on_the_same_event_as_a_launch 1` |
| dashboard-loop: The first frame is on screen before the first event is read | landed test, amended for the four-field `Live` | unit | inert `Live` | `testcount --lib ui::driver::tests:: 34` |
| dashboard-loop: Timeouts are not events and do not end the loop | landed test, amended and extended with the launcher's absence from `soonest` | unit | inert `Live` | `testcount --lib ui::driver::tests:: 34` |
| dashboard-loop: A backend draw failure ends the loop rather than spinning | landed test, amended | unit | inert `Live` | `testcount --lib ui::driver::tests:: 34` |
| dashboard-loop: Ctrl-C ends the loop | landed test, extended with the recording launcher's zero requests | unit | recording launcher | `testcount --lib ui::driver::tests:: 34` |
| dashboard-loop: An ignored key redraws and keeps waiting | landed test, amended | unit | inert `Live` | `testcount --lib ui::driver::tests:: 34` |
| dashboard-loop: A route change is visible in the next frame | landed test, amended | unit | inert `Live` | `testcount --lib ui::driver::tests:: 34` |
| dashboard-loop: `Live` cannot be built without naming the poller | `ui::driver::tests::live_cannot_be_built_without_naming_the_launcher`, an exhaustive four-field destructuring with no `..` | unit + compile | none | `testcount --lib ui::driver::tests::live_cannot_be_built_without_naming_the_launcher 1` |
| agent-poller: A polled snapshot changes no pixel at either width | landed `ui::view::tests::agents_change_no_pixel`, extended: `reachable` held `false` across the four fixtures, plus a second discriminating control flipping it to `true` | view | replaced | `testcount --lib ui::view::tests::agents_change_no_pixel 1` |
| agent-poller: `Dashboard` gains a field and every site is forced to name it | the three `NODEFAULT-UI` invocations plus the twelve destructuring companions | check + unit | source tree | as the `NODEFAULT-UI` row above; `testcount --lib ui::app::tests::dashboard_names_launch_at_every_site 1` |
| agent-poller: `Dashboard` still carries no thread, channel, or clock | landed `ui::app::tests::dashboard_is_clone_and_eq_with_agents`, extended to carry a non-empty `launch` | unit | none | `testcount --lib ui::app::tests::dashboard_is_clone 1` |
| agent-poller: The real wiring polls a scratch Herdr and adopts what it says | landed `ui::tests::wiring::the_real_wiring_polls_a_scratch_herdr`, amended for the four-field `Live` and extended with "no `pane split` entry" | acceptance | scratch tree, `notify`, both scratch programs, three real threads | `testcount --lib ui::tests::wiring:: 7` |
| agent-poller: A polled agent reaches a rendered badge and a rendered count | landed `ui::tests::wiring::a_polled_agent_reaches_a_rendered_badge`, extended: the footer now reads the 69-column form at 120 and the 53-column form at 60 | acceptance | as above plus a scratch state directory | `testcount --lib ui::tests::wiring:: 7` |
| agent-poller: The wiring test fails when the poller is replaced by the inert double | the `agents::none()`, `watch::none()`, `refresh::none()`, `Mapping::default()`, `rows`-ignoring, and footer-ignoring plants, re-run and re-recorded against the amended tests | plant | as above | `testcount --lib ui::tests::wiring:: 7` under each plant |
| agent-poller: An unreachable scratch Herdr leaves the pane a working standalone TUI | landed `ui::tests::wiring::an_unreachable_scratch_herdr_is_a_standalone_tui`, extended: the footer difference is now the two hints plus the count, and the state directory stays empty | acceptance | as above, `herdr` path absent | `testcount --lib ui::tests::wiring:: 7` |
| agent-poller: A pane with no OpenSpec repository still polls for agents | landed `ui::tests::wiring::no_repository_still_polls_for_agents`, extended: `launch::none()` is the launcher, the hints **are** shown, and `a` then `g` produce no `herdr` call beyond `agent list` | acceptance | scratch `herdr` (real), inert watcher, worker, and launcher | `testcount --lib ui::tests::wiring:: 7` |
| agent-poller: The residue left in `run` is small enough to read | `WIRED` with `launch::start` as leg 1's ninth name, a third positive control on `src/launch.rs`, and the new **leg 6**; four plants observed red at planning time | check | source tree | `sh $CHECKS/WIRED.sh` |
| change-rows: Active rows render at both mandated widths | landed test, unchanged | unit + view | replaced | `testcount --lib ui::list::tests:: 29` |
| change-rows: A badged row carries its status between the name and the progress cell | landed test, unchanged | unit + view | replaced | `testcount --lib ui::list::tests:: 29`; `LIST_MIN=29 sh $CHECKS/LISTWIDTHS.sh` |
| change-rows: An unattributed agent badges nothing | landed test, unchanged | unit | none | `testcount --lib ui::list::tests:: 29` |
| change-rows: A watch problem leads the list, above a change-set problem | landed test, extended with a leading launch problem and the empty-`launch.problems` control | unit | none | `testcount --lib ui::list::tests::refresh_and_change_problems_in_order 1` |
| change-rows: The row grammar places the marker, the name, and the progress cell | landed test, unchanged | unit | none | `testcount --lib ui::list::tests:: 29` |
| change-rows: A name too long for the field is truncated with an ellipsis | landed test, unchanged | unit | none | `testcount --lib ui::list::tests:: 29` |
| change-rows: A field too narrow for both drops the progress cell whole | landed test, unchanged | unit | none | `testcount --lib ui::list::tests:: 29` |
| live-updates: The startup request is issued before the first wait | landed test, amended for the four-field `Live` | unit | inert launcher | `testcount --lib ui::driver::tests:: 34` |
| live-updates: A result is adopted before the frame that shows it | landed test, amended | unit | inert launcher | `testcount --lib ui::driver::tests:: 34` |
| live-updates: A filesystem batch becomes one selection | landed test, amended | unit | inert launcher | `testcount --lib ui::driver::tests:: 34` |
| live-updates: A watcher error is recorded once and the loop continues | landed test, amended | unit | inert launcher | `testcount --lib ui::driver::tests:: 34` |
| live-updates: The wait shortens to the debounce deadline | landed test, amended | unit | inert launcher | `testcount --lib ui::driver::tests:: 34` |
| live-updates: The wait shortens to whichever poller is due first | landed test, amended — `soonest` still takes two arguments | unit | inert launcher | `testcount --lib ui::driver::tests:: 34` |
| live-updates: An agent snapshot reaches the frame that consumed it and survives a refresh | landed test, amended | unit | inert launcher | `testcount --lib ui::driver::tests:: 34` |
| live-updates: An unreachable socket never becomes a problem row | landed test, extended with `launch.problems` still empty | unit | inert launcher | `testcount --lib ui::driver::tests:: 34` |
| live-updates: An inert live tier leaves the loop exactly as it was | landed test, extended with `launch::none()` and the discarded-request claim | unit | fully inert `Live` | `testcount --lib ui::driver::tests:: 34` |
| live-updates: A launch request reaches the launcher and its outcome reaches the dashboard | `ui::driver::tests::a_pending_launch_request_is_handed_over_exactly_once` and `::a_launch_outcome_updates_the_mapping_and_replaces_the_problem` | unit | recording and scripted launchers | `testcount --lib ui::driver::tests::a_pending_launch_request_is_handed_over_exactly_once 1` |
| live-updates: A launch failure is recorded once and the loop continues | `ui::driver::tests::a_launch_failure_is_recorded_once_and_the_loop_continues` | unit | scripted launcher | `testcount --lib ui::driver::tests::a_launch_failure_is_recorded_once 1` |
| live-updates: A full live run leaves the change tree byte-identical | landed `ui::tests::live::` test, amended for the four-field `Live` | unit | **real** scratch tree and watcher | `testcount --lib ui::tests::live:: 3` |
| live-updates: `r` requests exactly one full refresh through the loop | landed test, amended | unit | **real** scratch tree | `testcount --lib ui::tests::live:: 3` |
| live-updates: The source-level guarantee holds tree-wide | `READONLY-UI` at `UI_MIN=11 EXTRA='src/watch.rs src/refresh.rs src/agents.rs src/launch.rs'`, plus a planted `std::fs::write` in `src/launch.rs` observed red | check | source tree | `UI_MIN=11 EXTRA='src/watch.rs src/refresh.rs src/agents.rs src/launch.rs' sh $CHECKS/READONLY-UI.sh` |
| live-updates: A watch problem is the list's first row | landed test, unchanged — its fixture carries no launch problem | unit | none | `testcount --lib ui::list::tests:: 29` |
| live-updates: Refresh problems precede change-set problems | landed test, extended with a leading launch problem and the empty control | unit | none | `testcount --lib ui::list::tests::refresh_and_change_problems_in_order 1` |
| live-updates: No refresh problem draws no extra row | landed test, extended with the empty-`launch.problems` byte-identity claim | unit | none | `testcount --lib ui::list::tests:: 29` |
| live-updates: A launch problem is the list's first row at both widths | `ui::list::tests::a_launch_problem_is_the_lists_first_row` at 38 and 58, plus `ui::list::tests::a_launch_problem_row_carries_no_badge` | unit | none | `testcount --lib ui::list::tests::a_launch_problem_is_the_lists_first_row 1`; `testcount --lib ui::list::tests::a_launch_problem_row_carries_no_badge 1` |
| responsive-layout: Header, body, and footer occupy their rows at both widths | landed test, extended: `agents.reachable` pinned `false` | view | replaced | `testcount --lib ui::view::tests:: 94` |
| responsive-layout: The action hints follow `Esc back` when the socket is reachable | `ui::view::tests::the_action_hints_follow_esc_back_when_reachable` at 60 and 120 | view | replaced | `testcount --lib ui::view::tests::the_action_hints_follow_esc_back_when_reachable 1` |
| responsive-layout: The action hints are dropped whole, `g focus` first | `ui::view::tests::the_action_hints_are_dropped_whole_g_focus_first` at 53, 52, 44, 43 | view | replaced | `testcount --lib ui::view::tests::the_action_hints_are_dropped_whole_g_focus_first 1` |
| responsive-layout: A one-row frame renders the header and nothing else | landed test, unchanged | view | replaced | `testcount --lib ui::view::tests:: 94` |
| responsive-layout: A two-row frame renders the header and the footer with no body | landed test, unchanged | view | replaced | `testcount --lib ui::view::tests:: 94` |
| responsive-layout: A one-column frame renders without panicking | landed test, extended with the reachable arm changing no cell at one column | view | replaced | `testcount --lib ui::view::tests:: 94` |
| responsive-layout: The footer drops whole hints rather than truncating one | landed test, extended: `reachable` pinned `false` | view | replaced | `testcount --lib ui::view::tests:: 94` |
| responsive-layout: The unattributed count is the footer's last hint at both widths | landed `ui::view::tests::the_unattributed_count_is_the_last_hint`, extended with the reachable arm's 69- and 53-column rows | view | replaced | `testcount --lib ui::view::tests::the_unattributed_count_is_the_last_hint 1` |
| responsive-layout: The count is reported with an empty change list | landed test, extended with the reachable arm's unchanged list interior | view | replaced | `testcount --lib ui::view::tests::the_count_is_reported_with_an_empty_list 1` |
| responsive-layout: The count is dropped whole before the three key hints | landed `ui::view::tests::the_count_drops_before_the_key_hints`, extended with the 69/68 boundary | view | replaced | `testcount --lib ui::view::tests::the_count_drops_before_the_key_hints 1` |
| responsive-layout: The filter prompt replaces the count along with the hints | landed `ui::view::tests::the_count_survives_a_filter`, extended with both reachable forms | view | replaced | `testcount --lib ui::view::tests::the_count_survives_a_filter 1` |
| list-filtering: The prompt replaces the hints while filtering, at both widths | landed test, extended with the reachable arm | view | replaced | `testcount --lib ui::view::tests:: 94` |
| list-filtering: An accepted query leads the hint list, at both widths | landed test, extended with the 75- and 59-column reachable forms | view | replaced | `testcount --lib ui::view::tests:: 94` |
| list-filtering: The action keys type into the query rather than launching | `ui::view::tests::the_action_hints_survive_a_filter` plus `ui::app::tests::the_four_action_keys_map_and_their_near_misses_do_not` | view + unit | replaced | `testcount --lib ui::view::tests::the_action_hints_survive_a_filter 1` |
| list-filtering: A prompt longer than the footer keeps its tail | landed test, extended with the reachable arm | view | replaced | `testcount --lib ui::view::tests:: 94` |
| responsive-layout / list-filtering: every view test names both widths | `WIDTHS` at its raised floor, passed **explicitly** | check | source tree | `WIDTHS_MIN=94 sh $CHECKS/WIDTHS.sh` |
| tasks-checklist: Every printable key leaves the change tree byte-identical | the landed `ui::tests::detail::tasks_tab_is_read_only`, extended: driven with `agents.reachable` true and a recording launcher, so `a`/`c`/`s`/`g` really reach `decide` while the snapshot pair still holds. It is a **modified** test, so no `testcount` floor can see the extension; `EXTENDED`'s `src/ui/mod.rs:tasks_tab_is_read_only:launcher` pair is what does | unit | **real** scratch tree, recording launcher | `PAIRS='<the forty-six>' sh $CHECKS/EXTENDED.sh`; `testcount --lib ui::tests::detail::tasks_tab_is_read_only 1` |
| tasks-checklist: No action mutates a task item | landed `ui::app::tests::no_action_mutates_changes`, extended: exhaustive match, seventeen-entry array, the `17` literal, and the reachable non-empty fixture | unit | none | `testcount --lib ui::app::tests::no_action_mutates_changes 1` |
| tasks-checklist: The dashboard names no write API | `READONLY-UI` with `src/launch.rs` in `EXTRA`, plus the planted `fs::write` in it observed red | check | source tree | `UI_MIN=11 EXTRA='src/watch.rs src/refresh.rs src/agents.rs src/launch.rs' sh $CHECKS/READONLY-UI.sh` |
| tasks-checklist: A launched agent's writes are not the plugin's writes | `ui::tests::wiring::a_keypress_launches_an_agent`'s two snapshot pairs and the discriminating control | acceptance | **real** scratch tree and state dir | `testcount --lib ui::tests::wiring::a_keypress_launches_an_agent 1` |
| live-updates: `r` refreshes outside filter mode and types inside it | landed `ui::app::tests::r_maps_to_refresh_outside_filter_mode`, unchanged — `r` keeps its meaning and the four new keys take four unused letters | unit | none | `testcount --lib ui::app::tests::r_maps_to_refresh_outside_filter_mode 1` |
| live-updates: `Refresh` sets the flag and touches nothing else | landed `ui::app::tests::refresh_sets_requested_and_changes_nothing_else`, extended with `launch` among the fields asserted unchanged | unit | none | `testcount --lib ui::app::tests::refresh_sets_requested_and_changes_nothing_else 1` |
| live-updates: No action mutates the change set | the same `ui::app::tests::no_action_mutates_changes` `tasks-checklist` names — one test serving both capabilities' statements of the count, which is why they must move together | unit | none | `testcount --lib ui::app::tests::no_action_mutates_changes 1` |
| subprocess-seam: A four-element argument vector distinguishes one failure from another | `cli::tests::a_prompt_argument_with_a_space_survives_the_seam`, driving a scratch program that exits 1 and asserting the two `args` vectors differ | unit | **real** spawn of a scratch program | `testcount --lib cli::tests::a_prompt_argument_with_a_space_survives_the_seam 1` |
| subprocess-seam: Stdout is returned verbatim on success | landed `cli::tests::` test, unchanged — this change adds no argument, no retry, no timeout, and no parsing to the seam | unit | **real** spawn of a scratch program | `testcount --lib cli::tests:: 41` |
| subprocess-seam: Stderr never reaches the success value | landed test, unchanged | unit | **real** scratch program | `testcount --lib cli::tests:: 41` |
| subprocess-seam: A non-zero exit is a failure carrying the code and stderr | landed test, unchanged — and the reason `agents::herdr_error_problem` and `launch`'s failure mapping can both carry Herdr's own words | unit | **real** scratch program | `testcount --lib cli::tests:: 41` |
| subprocess-seam: A program that cannot be started is a failure, not a panic | landed test, unchanged — an absent `herdr` is what the unreachable-socket acceptance run drives | unit | **real** absent path | `testcount --lib cli::tests:: 41` |
| subprocess-seam: Invalid UTF-8 on stdout is decoded lossily rather than failing | landed test, unchanged | unit | **real** scratch program | `testcount --lib cli::tests:: 41` |
| subprocess-seam: Empty stdout with a zero exit is success, not a failure | landed test, unchanged | unit | **real** scratch program | `testcount --lib cli::tests:: 41` |
| subprocess-seam: Arguments reach the program in order and unaltered | landed test, unchanged — the property the four-element prompt vector above exercises one step further | unit | **real** scratch program | `testcount --lib cli::tests:: 41` |
| subprocess-seam: A trait object crosses a thread boundary | landed test, unchanged; `launch::start` is the crate's third consumer of that property and its `Arc<dyn HerdrCli>` crosses to the launcher's worker | unit | **real** thread | `testcount --lib cli::tests:: 41` |
| subprocess-seam: The binding spawns the program it was given | landed `cli::tests::` test, extended with `cli::tests::a_prompt_argument_with_a_space_survives_the_seam` | unit | **real** spawn of a scratch program | `testcount --lib cli::tests::a_prompt_argument_with_a_space_survives_the_seam 1` |
| subprocess-seam: The default program name is written down once | landed `cli::tests::herdr_program_is_the_bare_name` plus `WIRED` leg 4 | unit + check | none | `testcount --lib cli::tests::herdr_program 1`; `sh $CHECKS/WIRED.sh` |
| subprocess-seam: The shell still names no CLI trait | `NOCLI-SHELL` at `UI_MIN=11`, plus the new `LAUNCHSEAM` leg 3 confining the handle to four files | check | source tree | `UI_MIN=11 sh $CHECKS/NOCLI-SHELL.sh`; `MIN=22 sh $CHECKS/LAUNCHSEAM.sh` |
| subprocess-seam: `cli` is the only module in the crate that spawns a process | `NOSPAWN-GREP` at `MIN=23` plus `LAUNCHSEAM` leg 1, with a planted `Command::new` in `src/launch.rs` observed red under both | check | source tree | `MIN=23 sh $CHECKS/NOSPAWN-GREP.sh`; `MIN=22 sh $CHECKS/LAUNCHSEAM.sh` |
| every "landed test, extended" row above | `EXTENDED` at its new pair list, with a removal planted and observed red | check | source tree | `PAIRS='<the list in tasks.md>' sh $CHECKS/EXTENDED.sh` |
| the change writes nothing inside `openspec/` | `OPENSPEC-UNTOUCHED`, `BASE` re-derived fresh at run time and never hardcoded | check | source tree | `BASE=$(git rev-parse HEAD) CHANGE=agent-launch sh $CHECKS/OPENSPEC-UNTOUCHED.sh` |

**Test-count arithmetic.** Every target is the count measured on `main` at the base commit plus
the number of new test functions this change enumerates, written out so it can be checked rather
than trusted. Measured on `main` at `df36bcb`: **840** library tests in total.

| Filter | Measured | New | Target |
|---|---|---|---|
| `launch::tests::` | 0 | 36 | 36 |
| `agents::tests::attribute::` | 17 | 2 | 19 |
| `ui::app::tests::` | 63 | 9 | 72 |
| `ui::view::tests::` | 88 | 6 | 94 |
| `ui::list::tests::` | 25 | 4 | 29 |
| `ui::driver::tests::` | 29 | 5 | 34 |
| `ui::tests::load::` | 9 | 1 | 10 |
| `ui::tests::wiring::` | 4 | 3 | 7 |
| `cli::tests::` | 40 | 1 | 41 |

Library total: `840 + 36 + 2 + 9 + 5 + 4 + 6 + 1 + 1 + 3` = **907** — the addends in the table's
own row order — asserted once and nowhere
else. `launch::tests::`'s 36 break down as: `decide::` 7, `argv::` 5, `prompt::` 2, `pane_id::` 3,
`run_request::` 10, `focus::` 2, `seam::` 4, plus 3 shape tests (`Outcome`'s exhaustive
destructuring and the two enum matches).

**A modified test is not covered by an aggregate floor**, and this change has **thirty-eight** of them — counted from `EXTENDED`'s own pair list rather than estimated.
`testcount --lib ui::list::tests:: 29` is satisfied by the four *new* tests alone, so skipping
every extension stays green. `EXTENDED` is what closes that: its pair list carries
`agent-attribution`'s eight landed pairs **plus** this change's thirty-eight, forty-six in all,
each naming the token its
extension must add, span-isolated to the named function's body. All thirty-eight were run
against `main` at planning time and all thirty-eight were red, while the eight landed ones were
green — which is how the run is known to discriminate rather than to fail uniformly.

**Coverage.** Measured on `main` at the base commit with `cargo llvm-cov --summary-only`:
**97.14% of lines, 22,257 lines total, 637 uncovered** — the **line** figure, not the region
count, which is 36,413 and is not interchangeable with it. The floor stays
`--fail-under-lines 80` and is neither lowered nor given an exclusion. The untestable residue
this change adds is nil: `run` gains no line, `start_collaborators` gains a `match` arm both
branches of which the wiring tests drive, and `launch::start`'s thread is exercised by
`launch::tests::seam::` exactly as `refresh::start`'s and `agents::start`'s are.

## Visual Design

**Non-visual in the design-source sense, and there is no design source to import.** This change
is user-facing — it adds two hints to a rendered footer and one row to a rendered list — but the
surface is a 38- or 58-column terminal buffer, not an HTML view or an email template. Its
"design" is the character grammar, and that lives where every other row grammar in this
repository lives: in `responsive-layout`'s, `list-filtering`'s, `change-rows`', and
`live-updates`' spec deltas as exact strings and exact column counts, asserted against a
`ratatui::backend::TestBackend` at both mandated widths. No `design/` directory is created and no
asset is imported.

## Decisions

**1. The launcher is the crate's third worker thread, not a synchronous call.** Measured,
`herdr agent start` blocks up to **thirty seconds** waiting for interactive readiness — the
default `--timeout`. A synchronous launch would freeze the pane for that long: no redraw, no key,
no poll. Alternatives: pass a short `--timeout` and call inline (**rejected** — it trades a long
freeze for a short one and makes the plugin fail launches that would have succeeded); reuse the
agent poller's worker (**rejected** — a thirty-second launch would stall the one-second poll, and
the poller's `in_flight` guard would silently suppress a second of polling per launch); make the
launch fire-and-forget with no outcome (**rejected** — the mapping must be recorded and the
failure must be shown, and both need an answer back).

**2. `agent start`'s and `agent prompt`'s timeouts are left at Herdr's defaults.** No `--timeout`
and no `--wait` are passed. Alternatives: pin `--timeout 30000` explicitly (**rejected** — it
freezes today's default into the argument vector, so a future Herdr that tuned it would be
overridden by a number nobody chose); pass `--wait` on `agent prompt` (**rejected** — measured,
it holds the call for the length of the agent's whole turn, which would occupy the worker thread
for minutes and delay every subsequent launch).

**3. The plugin never runs `herdr pane close`.** A failed `agent start` leaves the split pane in
place, named in the reported problem. Alternative: close the pane the launch created
(**rejected** — `agent start`'s failure set includes the readiness *timeout*, in which an agent
may be starting; closing would kill it, and a plugin whose error path destroys a running process
is worse than one that leaves an empty pane a person can close in one keystroke). The cost is a
stray pane on a rare path, and Decision 4 removes the common cause of it.

**4. A derived name already live in the session is refused before any Herdr call.**
`launch::decide` compares the derived name against the `name`s in the latest poll snapshot and
returns `Decision::Refuse`, so the common repeat-press — `a` on a change whose agent is already
running — creates no pane at all. Alternatives: attempt it and let Herdr answer `agent_name_taken`
(**rejected** on its own, though it remains the *race* path: the split has already happened by
then, so every repeat press would leak a pane); focus the existing agent instead (**rejected** —
a key that silently means something else is worse than a key that says why it did nothing, and
`g` already does the focusing); derive a second, distinct name (**rejected** — the mapping's
whole value is that the same change always derives the same name on every machine).

**5. `Action` gains four flat variants, not one carrying an `Intent`.** Seventeen, not fifteen.
Alternative: `Action::Launch(Intent)` plus `Action::FocusAgent` (**rejected**, and the reason is
specific rather than aesthetic: `no_action_mutates_changes` builds its `variants` array by hand,
and its own comment says an enumerate-by-hand test "would silently miss" a variant. A payloaded
variant lets that array carry `Launch(Apply)` and omit `Continue` and `Archive` while the
exhaustive `match` stays green — reintroducing exactly the gap the exhaustive match exists to
close). Four flat variants make the match and the array enumerate the same four things.

**6. `apply` produces a request; `run_loop` dispatches it.** This is `Action::Refresh`'s pattern
one step further on, and it is what keeps `Dashboard::apply` pure in the first change where a key
has an outward effect. Alternatives: `run_loop` matching on the `Action` after calling `apply` and
building the request itself (**rejected** — the launch policy would then live in
`src/ui/driver.rs`, which `NOBLOCK` and `NOCLI-SHELL` both constrain, and `apply` would no longer
be the one place a key's meaning is written); `apply` taking `&mut Live` (**rejected** — it makes
the purest function in the crate reach a collaborator, and every one of its sixty-three tests
would need one).

**7. `Attribution` gains `panes` rather than `g` re-resolving the agent.** Alternatives: a second
`herdr agent list` call from the launcher when `g` is pressed (**rejected** — a second round trip
on the key path, a second payload to parse, a second failure mode, and a window in which the
badge and the focus target disagree); resolving the pane in `Dashboard::apply` by re-running the
tier logic (**rejected** — it duplicates `attribute`'s three tiers in `src/ui/app.rs`, which is
where they must not be). Building `panes` in the same fold as `badges` makes "the badge and the
focus target are the same agent" structurally true rather than separately tested.

**8. `g` targets the `pane_id`, not the agent name.** Measured, `herdr agent focus` resolves
either. `pane_id` is present on **every** agent Herdr lists; `name` is present only on one
someone named. Using the pane also removes a rename race between the poll and the keypress.
Alternative: the name (**rejected** for both reasons, and it would have been the tempting choice
because tiers 1 and 2 both key on the name).

**9. The footer shows one compound hint, `a/c/s launch`, plus `g focus`.** Alternatives: four
separate hints (**rejected** — measured, `q quit  Enter detail  Esc back  a apply  c continue  s
archive` is 62 columns, so at the mandated 60-column frame `fit_hints` would drop `s archive` and
`g focus` and show the reader two of the four action keys with nothing saying the others exist);
no footer hint at all (**rejected** — `SPEC.md` → Degraded states requires that the action keys
be *hidden* when the socket is unreachable, which is only meaningful if they are shown when it is
not); a status line of its own (**rejected** — a second row is a layout change and this change
adds no region).

**10. The unattributed count is dropped at 60 columns when the socket is reachable.** The full
reachable footer with a count is 69 columns. The drop rule is `agent-attribution`'s, unchanged —
last hint first — and the hint list grew. Alternative: place the action hints **after** the count
so the count keeps its 60-column slot (**rejected** — it inverts the priority
`responsive-layout` already states, that key hints tell a reader how to drive the pane and the
count tells them something to act on later; the count is also still shown at 60 whenever the
socket is unreachable, which is when there is nothing else to say).

**11. Launch problems lead the list, above refresh and change-set problems.** Alternative: below
refresh problems, following the landed "standing above transient" ordering (**rejected** — a
launch problem is the only row in the pane produced by a key the reader has just pressed, and
burying the reply under two standing conditions is how a reader concludes the key did nothing).
It holds at most one entry and is cleared by a success, so leading costs at most one row.

**12. `state::record` runs between `agent start` and `agent prompt`.** Alternatives: before the
split (**rejected** — it would record a mapping for an agent that may never exist); after the
prompt (**rejected** — a failed prompt leaves a *running* agent that the pane must still be able
to badge, and a mapping written only on full success would strand it in tier 3). A recording
failure is reported but does not fail the launch: the in-memory mapping is still correct for the
session.

**13. `start_collaborators` takes the state directory as a fourth parameter**, reversing
`agent-attribution`'s explicit "SHALL NOT gain the state directory". The reason it gave — "the
mapping is state, not a collaborator" — was true while the mapping was only read. It is now also
written, by a worker thread, and the alternatives are worse: have the loop call `state::record`
between drains (**rejected** — a filesystem write on the render path, in a module `READONLY-UI`
covers), or have `launch::start` read the environment itself (**rejected** — `std::env::set_var`
is `unsafe` in edition 2024 and races parallel tests, which is why `Startup::herdr` and
`Startup::state_dir` exist at all).

**14. The poller and the launcher get separate `Arc<dyn HerdrCli>` handles.** Alternative: share
one (**rejected** — `RealHerdrCli` holds only a program path, so sharing saves nothing, and an
`Arc` shared between two threads with different cadences suggests a shared resource that does not
exist).

**15. `LAUNCHSEAM` is a new check rather than a widening of `AGENTSEAM`.** Alternative: add
`src/launch.rs` to `AGENTSEAM`'s subject (**rejected** — `AGENTSEAM`'s legs 1 and 2 are scoped to
one file by design, and a two-file version would report "one of them" rather than which). The
`ALLOWED` list for the Herdr handle **is** shared and grows to four files in both checks, which is
the one thing they legitimately agree about.

## Risks / Trade-offs

- **A failed `agent start` leaves a stray pane** → Decision 3 accepts it deliberately over the
  alternative of killing a possibly-starting agent; Decision 4 removes the common cause by
  refusing a known-live name before any pane is created; and the reported problem names the pane
  id so the reader can close it in one keystroke.
- **The refusal in Decision 4 reads the *latest poll*, so a race remains** → An agent that appeared
  since the last poll (up to one second) still reaches Herdr and comes back `agent_name_taken`,
  with a stray pane. Both paths are specified and tested; the window is one poll interval and the
  cost is one empty pane.
- **A thirty-second `agent start` occupies the launcher's worker for that long** → A second launch
  pressed during it queues on the request channel rather than being dropped, and the pane keeps
  drawing, polling, and refreshing throughout because the worker is not the render thread. What it
  does **not** do is tell the reader a launch is in flight; that is a deliberate non-goal (no
  spinner, no pending row) and would be a later change with a state of its own.
- **`launch.problems` is replaced wholesale, so two rapid failures show only the second** → The
  same rule `refresh.problems` already has, for the same reason: a socket failing on every press
  must not grow an unbounded list.
- **An archived change can be launched with `/opsx:apply`** → Deliberate. Deciding which command
  suits which change is orchestration, which `PRD.md` → Non-goals reserves for the agent. The
  plugin sends what the key says and lets the agent judge.
- **`c` is now a live key where it was inert** → It is the one key with two rows in the mapping
  table: bare `c` launches, `Ctrl-C` quits, distinguished by the modifier the table already
  matches on. `Ctrl-C` is asserted to still quit under both filter modes, and a `Release` or
  `Repeat` of `c` is asserted to do nothing.
- **The acceptance tests spawn three real threads and two real programs per run, twice each (120
  and 60)** → The same cost `agent-polling` and `agent-attribution` already pay; the launcher's
  worker is idle unless a key is pressed, and the scratch `herdr` program answers in
  milliseconds.
- **`DEPS` and `GRAPH-SNAP` stay red** → Inherited, not caused, and recorded in
  planning-review.md with the exact failure text so a reviewer does not mistake it for a
  regression.

## Migration Plan

None is needed. No stored format changes, no data is migrated, no deployment step is added, and
the change is additive in the shipped binary: a pane with no reachable socket renders
byte-identically to the current release, and a pane with one gains two footer hints and four keys.
Rollback is `git revert` of the change's commits. The one thing a rollback leaves behind is an
`agent-names.toml` holding entries a reverted plugin would still read and honour — `plugin-state`
already specifies that file's format and `agent-attribution` already reads it, so a stale entry
degrades to the fall-through tier rather than to a fault.

## Open Questions

None. The four questions that would otherwise be open were settled by live measurement against
Herdr 0.8.2 before this document was written — whether `--direction` is optional (it is not),
what `pane split` returns (an envelope), whether `agent focus` accepts a pane id (it does, and not
a terminal id), and what a name collision does (`agent_name_taken`, exit 1, after the pane
already exists) — and all four are recorded above and corrected in `SPEC.md`. The one deliberately
deferred question, what to show while a launch is in flight, is recorded as a Risk with its
rejected fix rather than left open.
