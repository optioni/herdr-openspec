# agent-poller Specification

## Purpose
The cadence around `agent-list`: a non-blocking `AgentPoll` seam, a poll at roughly one
second that reports its own deadline so the loop can wake for it, the crate's second
worker thread reaching Herdr only through the seam, the `Dashboard` field carrying the
latest snapshot, and a composition root wired under test rather than only read by a
reviewer.

## Requirements

### Requirement: The poller seam is a trait whose every method is non-blocking

`agents::AgentPoll` SHALL be a trait whose every method is non-blocking, on exactly
`watch::FsEvents`' and `refresh::Refresher`'s terms:

```rust
pub trait AgentPoll: Send {
    /// Fire a poll if one is due and take the worker's answer if one is
    /// ready. Never blocks, never sleeps, never waits on a channel.
    fn drain(&mut self) -> Option<AgentSnapshot>;
    /// The time remaining before the next poll is due, or `None` when a
    /// poll is already in flight and there is nothing to wake for. Takes no
    /// `Instant`: the real implementation returns the value its own last
    /// `drain` computed.
    fn pending_in(&self) -> Option<Duration>;
}
```

`drain` SHALL return `None` when the worker has produced nothing new, and it SHALL do so
without waiting: the concrete implementation reads the worker's channel with `try_recv` and
never with `recv`, `recv_timeout`, `iter`, or a `for` loop over the receiver.

`agents::none()` SHALL return the inert implementation — `drain` always `None`, `pending_in`
always `None`, no thread, no process — which is what a test that does not care about agents
passes and what the poller degrades to when its worker is gone.

The non-blocking contract SHALL be **checked, not merely documented**, and the check SHALL be
`NOBLOCK` leg 3 extended to a **third** file rather than a new check of its own: leg 3's
`src/watch.rs` / `src/refresh.rs` list is literal, and a poller module absent from it is a
poller module nothing sweeps. `src/agents.rs`'s production slice SHALL name no `.recv(`,
`recv_timeout`, `.join()`, `park_timeout`, `thread::park`, `rx.iter()`, or `for … in …rx`
**before** its single `thread::spawn` — everything after that point is the worker body, which
may block freely — and it SHALL name `try_recv`.

The two guards that keep that leg from being dodged by moving code rather than fixing it SHALL
cover the new file too. Guard D — each seam module holds **exactly one** line-anchored
`#[cfg(test)]`, since the slicer truncates at the first and every sweep below it is silent —
SHALL loop over `src/watch.rs`, `src/refresh.rs`, and `src/agents.rs`. Guard E — the render-path
method is **declared above** the spawn, or the leg searches the worker half and reports OK on a
blocking implementation — SHALL be applied to `src/agents.rs` with `fn drain` as its subject,
taking the **last** matching line rather than the first: the file declares `fn drain` three
times, on the trait, on the inert implementation, and on the real one, and `head -1` would pin
the guard to the trait's abstract signature, which necessarily precedes every spawn. That is
the exact defect `live-refresh`'s own Change Review repaired in Guard E for `src/refresh.rs`,
and repeating it here would be repeating a known bug.

#### Scenario: The inert poller yields nothing and starts nothing

- **WHEN** `agents::none()` is drained ten times and `pending_in` is read after each
- **THEN** every `drain` returns `None` and every `pending_in` returns `None`
- **AND** no thread was spawned and no process was started, so a machine with no `herdr`
  program pays nothing for the seam existing

#### Scenario: The three seam modules never block on the render path

- **WHEN** `src/watch.rs`'s, `src/refresh.rs`'s, and `src/agents.rs`' production slices are
  swept for `.recv(`, `recv_timeout`, `.join()`, `park_timeout`, `thread::park`, `rx.iter()`,
  and `for … in …rx`, with `src/refresh.rs` and `src/agents.rs` cut at their single
  `thread::spawn` and only the half above it searched
- **THEN** there is no match in any of the three
- **AND** each of the three production slices names `try_recv`, checked as a positive control
  before its sweep, or its non-blocking receive is not there at all
- **AND** `src/agents.rs`' production slice names `mpsc` and `thread::spawn`, checked as a
  positive control, or the worker is not a worker and the cut is vacuous
- **AND** each of the three holds exactly one line-anchored `#[cfg(test)]`, so a second one
  above the spawn cannot hide the worker's whole body from this sweep and from `READONLY-UI`
- **AND** `src/agents.rs`' **last** `fn drain` line precedes its `thread::spawn` line, and the
  guard is proven able to fail by moving the real implementation's `drain` below `start` and
  observing it go red — under a `head -1` form it stays green, which is why it takes the last
  line
- **AND** the whole leg is proven able to fail against a copy whose `RealAgentPoll::drain`
  calls `self.result_rx.recv_timeout(d)`

### Requirement: The poll fires at roughly one second and reports its own deadline

`agents::POLL_INTERVAL` SHALL be `Duration::from_secs(1)`, pinned by an assertion the way
`watch::DEBOUNCE` and `ui::driver::TICK` are, so `SPEC.md` → Agent status by polling's
"roughly one-second intervals" has one place it is written down.

The **schedule** SHALL live on the render side of the seam and the **spawn** on the worker
side. `RealAgentPoll::drain` SHALL, in one call and in this order: read the clock **once**;
take the worker's answer with `try_recv`, and on an answer set the next due instant to that
same `now` plus `POLL_INTERVAL`; then, when no poll is in flight and the next due instant has
arrived or was never set, send one request to the worker; then leave behind the `Duration`
`pending_in` returns until the next call.

This split is what lets the loop wake for the poller without reading a clock itself. `NOBLOCK`
leg 2 forbids every file under `src/ui/` — tests included — from naming a clock, so a poller
that kept its schedule in the worker and reported nothing would leave `run_loop` with no way to
shorten its wait; the poller reports a `Duration` for exactly the reason `FsEvents::pending_in`
does. `src/agents.rs` becomes the crate's **second** clock binding, alongside
`watch::RealFsEvents::drain`; `SPEC.md` → Architecture and `AGENTS.md` both currently call that
one the crate's only clock binding, and correcting both sentences is part of this change.

At most **one** poll SHALL be in flight at a time. While one is, `pending_in` SHALL return
`None` — there is no deadline to wake for — and no further request SHALL be sent, so a socket
that answers slowly cannot queue an unbounded backlog of `herdr` spawns.

The **first** `drain` on a freshly started poller SHALL fire a request immediately rather than
waiting a second: a pane that opened next to a working agent should not show nothing for the
first second of its life.

Polling SHALL continue at the same interval after a failure of any kind. A backoff was
considered and rejected: the Herdr server can start, stop, and restart while a pane is open,
and a poller that slowed down or stopped after an unreachable socket would leave the pane
permanently blind to a server that came back. An unstartable `herdr` costs one `ENOENT` per
second, measured in microseconds, against a successful call measured at **8 milliseconds
median** on the reference machine.

`agents::poller_for_test` SHALL be the module's own test helper, declared **inside**
`mod tests` so the single line-anchored `#[cfg(test)]` count Guard D requires is unaffected. It
SHALL follow `refresh::worker_for_test`'s established shape exactly: it returns a `Box<dyn
AgentPoll>` whose `drain` **never yields a snapshot** — the test reads the worker's answers off
a raw `mpsc::Receiver<AgentSnapshot>` handed back beside it, with a deadline-bounded
`recv_timeout` — plus a third channel whose `Sender` the worker thread owns, so the worker's
return is observable when the poller is dropped. A single-consumer channel cannot answer both
`drain` and a raw receiver, which is why the helper's poller yields nothing rather than sharing.

Scenarios that need `drain`'s own return value — the schedule, the in-flight suppression, the
dead worker — use `agents::start` with a `FakeCli` or a scratch program instead, and never the
helper.

#### Scenario: The first drain polls immediately and the second does not

- **WHEN** a poller built by `agents::poller_for_test` over a scratch `#!/bin/sh` program is
  drained, the worker's single answer is awaited on the raw receiver the helper hands back with
  a deadline-bounded `recv_timeout`, and the poller is drained twice more
- **THEN** the scratch program's invocation log records exactly **one** `agent list` run across
  the three drains, so the first drain polled and the later ones did not
- **AND** every one of the three `drain` calls returned `None`, because this helper's poller
  never yields — the snapshot arrived on the raw receiver instead
- **AND** `pending_in` is `None` throughout, since the poll it fired is still in flight for the
  helper's poller and no answer ever reaches it

#### Scenario: A poll in flight suppresses the next request and reports no deadline

- **WHEN** a poller built by `agents::poller_for_test` is drained five times in succession, so
  the worker's answers never reach it and the first poll stays in flight
- **THEN** the scratch program's log records exactly **one** run, not five
- **AND** every one of those five `pending_in` reads returned `None`, so
  `watch::poll_timeout` leaves the tick alone while a poll is outstanding

#### Scenario: The interval is one second and is asserted, not assumed

- **WHEN** `agents::POLL_INTERVAL` is read
- **THEN** it equals `Duration::from_secs(1)`
- **AND** the assertion exists so a later change moving it is a deliberate edit rather than a
  silent drift from `SPEC.md`

#### Scenario: A dead worker is reported once and then stops being reported

- **WHEN** a poller's worker channel is disconnected — the helper drops the worker's result
  `Sender` — and the poller is drained three times
- **THEN** the first `drain` returns `Some(AgentSnapshot { agents: [], reachable: false, problem:
  Some(text) })` whose text names the poller's worker as stopped
- **AND** the second and third `drain` calls return `None` and send no further request, so a
  dead worker degrades to silence rather than to a spin
- **AND** `pending_in` is `None` from then on

### Requirement: The worker is the crate's second thread and reaches Herdr only through the seam

`agents::start(cli: Arc<dyn cli::HerdrCli>) -> Box<dyn AgentPoll>` SHALL spawn exactly one
thread, and `src/agents.rs` SHALL hold exactly one `thread::spawn`. The worker's body SHALL
block on its request channel, call `agents::poll_once` for each request, and send the resulting
`AgentSnapshot` back, returning when either channel disconnects — the same lifecycle
`refresh::worker_body` already has, so dropping the poller drops the request `Sender` and the
thread returns.

`cli::HerdrCli` being `Send + Sync` is what makes this legal, and it is now load-bearing for a
second consumer rather than theoretical: `refresh` already holds an `Arc<dyn OpenspecCli>`
across a thread, and this worker holds an `Arc<dyn HerdrCli>` the same way.

The worker SHALL spawn no process itself: it reaches the `herdr` program only through the trait
object, and `src/cli.rs` remains the crate's single spawn site.

#### Scenario: A started poller polls a scratch program and yields its agents

- **WHEN** `agents::start` is given `cli::agent_cli_via` over a scratch `#!/bin/sh` program that
  prints a one-agent `agent_list` envelope, and `drain` is called in a deadline-bounded loop
  until it returns `Some(_)`
- **THEN** the snapshot is `reachable: true` with one agent whose `pane_id` is `w8:p1`
- **AND** the scratch program's own invocation log records exactly one run whose arguments were
  `agent list`
- **AND** no test in this change spawns the real `herdr` binary: the seam's own tests use
  scratch `#!/bin/sh` programs, exactly as `subprocess-seam`'s do

#### Scenario: A scratch program that fails is a reachable-false snapshot

- **WHEN** `agents::start` is given a scratch program that exits `1` after writing Herdr's
  `server_not_running` error envelope to stderr, and `drain` is called until it returns
  `Some(_)`
- **THEN** the snapshot is `AgentSnapshot { agents: [], reachable: false, problem: Some(text) }`
  and `text` names the exit code and `server_not_running`

#### Scenario: An unreachable socket is recovered from, not backed off

- **WHEN** `agents::start` is given a scratch program whose **first** run exits `1` and whose
  every later run prints a one-agent `agent_list` envelope — it reads and rewrites a counter
  file to decide — and `drain` is called in a deadline-bounded loop past `POLL_INTERVAL` until
  a second snapshot arrives
- **THEN** the first snapshot is `reachable: false` and the second is `reachable: true` with one
  agent
- **AND** the scratch program's log records exactly two runs, so the poller neither backed off
  nor stopped after the failure
- **AND** this is the scenario that proves the standing state is a state the pane comes back
  from: `SPEC.md` → Degraded states says an unreachable socket runs as a standalone TUI, and a
  poller that gave up would make that permanent on a server restart

#### Scenario: Dropping the poller stops the thread

- **WHEN** a started poller is dropped and the worker's exit is observed through the channel
  `poller_for_test` hands back for that purpose
- **THEN** the worker returns rather than leaking a thread for the pane's lifetime
- **AND** the observation is a deadline-bounded receive, never a fixed sleep followed by an
  assertion

### Requirement: The dashboard carries the latest snapshot and nothing renders it

`ui::app::Dashboard` SHALL carry a tenth field, `agents: agents::AgentSnapshot`, holding the
most recent poll's outcome. `ui::load` SHALL initialise it, on both of its branches, to
`AgentSnapshot { agents: Vec::new(), reachable: false, problem: None }` — written as a literal
at each site, never through a `Default`, so the field count is enforced by the compiler.

`reachable: false` before the first poll completes is deliberate and is the same value an
unreachable socket produces. That is correct rather than a conflation: `agent-attribution` and
`agent-launch` hide the agent column and the action keys on `!reachable`, and hiding them for
the first second of a pane's life is the right behaviour, whereas showing an empty agent column
that then disappears would not be.

A snapshot SHALL replace the field wholesale, never merge into it: a poll that found no agents
means there are no agents.

**No view SHALL read this field in this change.** `ui::list`, `ui::detail`, `ui::view`,
`ui::tasks`, and `ui::markdown` are untouched, and every landed buffer assertion in the suite
SHALL be byte-identical after this change. Rendering is `agent-attribution`'s work; landing the
state and the render in one change would make it impossible to tell which of the two broke a
buffer assertion.

#### Scenario: A polled snapshot changes no pixel at either width

- **WHEN** the same `Dashboard` is rendered at 120x20 and at 60x20, once with `agents` in its
  initial state and once with `agents` holding two reachable agents and once with `agents`
  holding an unreachable-socket snapshot whose `problem` is a long string
- **THEN** the three buffers are identical at each width, cell for cell
- **AND** in particular the unreachable-socket `problem` appears nowhere on screen, and no
  `!`-marked row is added: `SPEC.md` → Degraded states records an unreachable socket as
  "runs as a standalone TUI", which is silence, not a message
- **AND** the 60-column case is asserted as well as the 120-column one, so the responsive
  breakpoint is covered rather than assumed

#### Scenario: `Dashboard` gains a field and every site is forced to name it

- **WHEN** the `NODEFAULT-UI` sweep runs over `src/` for `Dashboard`, `Filter`, `Detail`, and
  `Refresh`, and again with its positive-control file parameterised to `src/agents.rs` for
  `Agent`, `Listed`, and `AgentSnapshot`
- **THEN** neither run finds an `impl Default`, a derived `Default`, or a `..` rest inside a
  literal or pattern of any of the seven types
- **AND** the compile-time companion destructures a `Dashboard` with an exhaustive pattern
  naming all **ten** fields and no `..`, an `Agent` naming all **eight**, a `Listed` naming
  both, and an `AgentSnapshot` naming all three
- **AND** the parameterisation is a real change to the check rather than a second copy of it:
  the control file was hardcoded to `src/ui/app.rs`, and two versions of one check on disk is
  how a run and a record drift apart

#### Scenario: `Dashboard` still carries no thread, channel, or clock

- **WHEN** `Dashboard` is constructed in a test and cloned and compared
- **THEN** it is still `Clone`, `PartialEq`, and buildable with no thread, no filesystem, and no
  process: `AgentSnapshot` is plain data, and the poller itself reaches the loop through
  `ui::driver::Live`, never through the state value

### Requirement: The composition root is driven by a test, not only read by a reviewer

`live-refresh` shipped a `ui::run` that never called `watch::start` or `refresh::start`. Every
test passed, because unit and view tests drove the seams directly and nothing exercised the
function that composes them; the whole live tier would have been permanently inert in the
shipped binary, and only Change Review caught it. `agent-polling` adds a third collaborator
behind a third seam and can reproduce that defect exactly, so the composition root SHALL be
split until a test can drive it.

`ui::run` SHALL be split into three:

```rust
pub struct Startup<'a> { pub cwd: &'a Path, pub config: &'a Config, pub herdr: &'a Path }

pub struct Collaborators {
    pub fs: Box<dyn watch::FsEvents>,
    pub refresher: Box<dyn refresh::Refresher>,
    pub agents: Box<dyn agents::AgentPoll>,
    pub problems: Vec<String>,
}

pub fn start_collaborators(repo: Option<&Path>, config: &Config, herdr: &Path) -> Collaborators;

pub fn run_wired<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>, events: &mut E, startup: &Startup<'_>,
    read: ArtifactReader<'_>, tick: Duration,
) -> Result<Dashboard, StartError>;

pub fn run() -> Result<(), StartError>;
```

`start_collaborators` SHALL start the real watcher and the real worker when a repository was
found and their inert doubles when one was not, and SHALL fold a watcher that would not start
into `problems`. It SHALL start the real **poller unconditionally**, with or without a
repository: Herdr agents exist independently of an OpenSpec repository, and `agent-launch` reads
`reachable` to decide whether to offer its keys in a pane that never found one. The `herdr`
program is a **parameter** rather than the literal `"herdr"` precisely so a test drives this
function against a scratch `#!/bin/sh` program; `cli::HERDR_PROGRAM` holds the literal and `run`
is the only caller that passes it.

`run_wired` SHALL do everything `run` does once a terminal exists — `load`, then
`start_collaborators`, then move `problems` onto `dashboard.refresh.problems`, then build
`ui::driver::Live` from all three collaborators, then `run_loop` — and SHALL return the final
`Dashboard` so a test can assert on state the frame does not show. It takes a `Startup` struct
rather than three further parameters for cohesion, on the same grounds `Live` is a struct: the
three are one concept, "where this pane starts", and bundling them keeps the function at five
arguments rather than seven.

`run` SHALL then hold **no branch, no loop, and no field selection**: the terminal guard, the
panic hook, `config::load_from_env`, `current_dir`, the real `Terminal`, `CrosstermEvents`, the
`HERDR_PROGRAM` literal, `read_artifact`, `TICK`, and one `run_wired` call. Everything that can
be miswired is below that line and is driven by a test.

Neither `run_wired` nor `start_collaborators` SHALL name `OpenspecCli`, `HerdrCli`, `from_cli`,
`CliChanges`, or `npm_prefix`: the handles arrive from `cli::worker_cli_from_env` and
`cli::agent_cli_via`, whose own signatures carry those types, which is the mechanical reason
`NOCLI-SHELL` stays green while the shell is wired to two real binaries.

#### Scenario: The real wiring polls a scratch Herdr and adopts what it says

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 over a `testutil::ScratchDir`
  repository holding one active change `alpha` whose `tasks.md` counts 4 of 9, with the **real**
  `ui::read_artifact`, a `Startup` naming that repository, a `Config` whose `openspec_bin` names
  a **usable** scratch `#!/bin/sh` program — so the binary probe chain stops at its first step
  and the real `openspec` binary is never spawned — a `herdr` path naming a second scratch
  program printing a one-agent `agent_list` envelope, and a `testutil::UntilReady` event source
  whose readiness predicate (a) waits for both scratch programs to log their first invocation,
  (b) then writes one byte to `openspec/changes/alpha/proposal.md`, and (c) then waits for the
  scratch `openspec` program's log to reach **two** invocations, before pressing `q`
- **THEN** `run_wired` returns `Ok(dashboard)` whose `agents.reachable` is true and whose
  `agents.agents` holds exactly one agent with `pane_id` `w8:p1`, proving `agents::start` was
  wired rather than `agents::none()`
- **AND** the scratch `herdr` program's invocation log records at least one run whose arguments
  were exactly `agent list`
- **AND** the scratch `openspec` program's log records **two or more** runs, proving both that
  `refresh::start` was wired rather than `refresh::none()` and that `watch::start` was wired
  rather than `watch::none()`: the first run is `ui::load`'s startup request, and a **second**
  one exists only if a real watcher reported the write at (b), the debounce released it, and
  `watch::invalidate` turned it into a further request
- **AND** `dashboard.refresh.problems` is empty. This is a supporting assertion and **not** the
  watcher's discriminating one: `watch::start` returns an empty `problems` on success and
  `watch::none()` is wired with an empty `problems` too, so emptiness alone is satisfied by
  both arms and would have let an unwired watcher through. The second `openspec` invocation is
  the assertion that separates them
- **AND** the buffer's list row for `alpha` ends in `[4/9]`, so the **real** `ui::load` and the
  **real** artifact reader ran
- **AND** the three wiring assertions are each **separately** discriminating and each names its
  own collaborator: the poller's is `agents.reachable`, the worker's is the `openspec` log being
  non-empty, and the watcher's is that log reaching two entries. A single missing collaborator
  therefore names itself rather than failing one shared assertion
- **AND** this scenario makes **no** byte-identity claim, because its own predicate writes to the
  tree on purpose. That claim is made by the unreachable-socket scenario below, which writes
  nothing

#### Scenario: The wiring test fails when the poller is replaced by the inert double

- **WHEN** `start_collaborators` is edited to pass `agents::none()` in place of
  `agents::start(...)`, and the scenario above is run
- **THEN** it fails, naming `agents.reachable` as false and `agents.agents` as empty
- **AND** this plant is run and recorded during implementation, and the edit reverted, because a
  wiring test that cannot fail is exactly the artifact that let `live-refresh`'s inert live tier
  through
- **AND** the same plant is repeated for `watch::none()` in place of `watch::start`, and for
  `refresh::none()` in place of `refresh::start` — the two collaborators that were actually
  missing in `live-refresh` — and each is observed red: the watcher plant on the scratch
  `openspec` log stopping at **one** entry, and the worker plant on that log being empty
- **AND** the `UntilReady` source's deadline path is what makes each plant fail rather than hang:
  its readiness predicate never becomes true when a collaborator is missing, so the run ends at
  the deadline and the assertion reports the missing collaborator. The deadline path is
  exercised on every run of the unreachable-socket scenario below, not only under a plant

#### Scenario: An unreachable scratch Herdr leaves the pane a working standalone TUI

- **WHEN** the same run is driven with a `herdr` path naming a program that does not exist, and a
  readiness predicate that waits only for the scratch `openspec` program's first log entry and
  writes nothing — the `herdr` log can never appear, and a predicate waiting for it would spend
  the whole deadline on every run
- **THEN** `run_wired` returns `Ok(dashboard)` with `agents.reachable` false, `agents.agents`
  empty, and `agents.problem` naming the program and the operating system's reason
- **AND** the buffer is **identical** to the reachable run's at both widths: the list still shows
  `alpha` at `[4/9]`, nothing is hidden, and no error screen replaces the pane
- **AND** `run_wired` returns `Ok`, not `Err`: an unreachable socket is a supported state and
  never a `StartError`
- **AND** the change tree is byte-identical before and after — every path, its bytes, and its
  modification time — with a discriminating control rewriting one byte of `tasks.md` between two
  further snapshots and asserting they differ. This run holds a real watcher open on the tree
  while a real worker and a real poller run beside it, and writes nothing, which is where
  `live-updates`' "the live tier writes nothing inside the repository" is proved for a
  three-collaborator tier

#### Scenario: A pane with no OpenSpec repository still polls for agents

- **WHEN** `run_wired` is driven at 120x20 over a `testutil::ScratchDir` holding **no**
  `openspec/` directory, with the same two scratch programs and a readiness predicate naming the
  `herdr` log alone
- **THEN** `run_wired` returns `Ok(dashboard)` with `repo` `None`, the list region's
  no-repository empty state on screen, and `agents.reachable` **true** with one agent
- **AND** the scratch `openspec` program's log is **empty** and `refresh.problems` is empty: with
  no repository, `refresh::start` and `watch::start` are correctly the inert doubles and cost
  nothing — no thread, no process, no watch
- **AND** this is the asymmetry the requirement states: the watcher and the worker are about a
  repository, the poller is about the Herdr session, and only the first two are skipped

#### Scenario: The residue left in `run` is small enough to read

- **WHEN** `src/ui/mod.rs`'s production slice is searched for `run_wired`,
  `start_collaborators`, `watch::start`, `refresh::start`, `agents::start`,
  `worker_cli_from_env`, and `agent_cli_via`
- **THEN** every one of the seven is present, so a collaborator dropped from the wiring is a
  failing grep as well as a failing test
- **AND** `src/ui/mod.rs` names none of `OpenspecCli`, `HerdrCli`, `from_cli`, `CliChanges`, or
  `npm_prefix`, which `NOCLI-SHELL` already checks over the whole of `src/ui/`
- **AND** the check is paired with a positive control — `src/cli.rs` **does** name
  `agent_cli_via` — so a renamed binding fails the control rather than leaving the sweep
  searching for a name that is no longer there
