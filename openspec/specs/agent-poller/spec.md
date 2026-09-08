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

The requirement's name is kept verbatim from `agent-polling` because a delta's requirement
headers are its merge key. Its subject is unchanged — where the snapshot lives and what
replaces it — but "nothing renders it" was true of exactly one change, and the body below now
says which change reads it and through what.

`ui::app::Dashboard` SHALL carry a field, `agents: agents::AgentSnapshot`, holding the
most recent poll's outcome. `ui::load` SHALL initialise it, on both of its branches, to
`AgentSnapshot { agents: Vec::new(), reachable: false, problem: None }` — written as a literal
at each site, never through a `Default`, so the field count is enforced by the compiler.

`reachable: false` before the first poll completes is deliberate and is the same value an
unreachable socket produces. That is correct rather than a conflation: an unreachable socket
and a pane one tick old both have **no agents**, and the badge and the count are both driven by
`agents.agents` being empty rather than by `reachable` — they vanish on their own.
**`agent-launch` is the change that reads `reachable` directly**, in two places and nowhere
else: `ui::view`'s footer, which offers the `a/c/s launch` and `g focus` hints only when it is
true, and `Dashboard::apply`, which passes it to `launch::decide` so a launch key pressed
against an unreachable socket does nothing at all. Hiding the keys for the first second of a
pane's life is the right behaviour: there is no socket answer yet, and offering a key that
cannot work is worse than offering it a tick late.

A snapshot SHALL replace the field wholesale, never merge into it: a poll that found no agents
means there are no agents.

**`agent-attribution` is the change that began reading this field, and `agent-launch` is the
second.** The requirement's earliest form — no view SHALL read it — held for exactly one change,
which is why `agent-polling` landed the state and the render separately: a buffer assertion that
moved could then only have been broken by one of them. From `agent-attribution` onward
`ui::list` and `ui::view` read `agents.agents`, and **only** through
`Dashboard::attribution()`, whose output is keyed by change name; from `agent-launch` onward
`ui::view` additionally reads `agents.reachable` directly, which is a `bool` and needs no
attribution to interpret. `ui::detail`, `ui::tasks`, and `ui::markdown` still do not read the
field at all.

An agent that `agent-attribution` places **out of scope** — one whose `cwd` is absent, or lies
outside the resolved repository root — SHALL still change no pixel. That is what keeps this
requirement's byte-identity scenario meaningful rather than merely historical. The scenario's
fixtures SHALL hold `reachable` constant across the buffers they compare, since `reachable` now
moves two footer hints; comparing a reachable buffer against an unreachable one would be
comparing two different footers and would say nothing about scope.

#### Scenario: A polled snapshot changes no pixel at either width

- **WHEN** the same `Dashboard`, whose repository root is `/tmp/demo-repo`, is rendered at
  120x20 and at 60x20, once with `agents` in its initial state, once with `agents` holding two
  reachable agents whose `cwd` is `/somewhere/else` and whose names match changes on screen,
  once with `agents` holding one reachable agent carrying **no** `cwd` at all, and once with
  `agents` holding an unreachable-socket snapshot whose `problem` is a long string — **with
  `reachable` held at `false` in all four**, so the four footers are the same footer and the
  comparison is about scope alone
- **THEN** the four buffers are identical at each width, cell for cell
- **AND** the agents in the second and third cases are out of scope — a `cwd` outside the
  repository root, and an absent `cwd` — so `agent-attribution` badges nothing and counts
  nothing, and the byte-identity claim is about the scope rule rather than about the field
  being unread
- **AND** the discriminating control is the same dashboard with one of those agents moved to
  `cwd` `/tmp/demo-repo`: that buffer **differs** at both widths, so the identity above is a
  scope branch and not a constant
- **AND** a second discriminating control flips `reachable` to `true` on the first fixture
  alone: that buffer **differs** at both widths, in the footer and only in the footer, which is
  `agent-launch`'s addition and the reason the four fixtures above hold it constant
- **AND** in particular the unreachable-socket `problem` appears nowhere on screen, and no
  `!`-marked row is added: `SPEC.md` → Degraded states records an unreachable socket as
  "runs as a standalone TUI", which is silence, not a message
- **AND** the 60-column case is asserted as well as the 120-column one, so the responsive
  breakpoint is covered rather than assumed

#### Scenario: `Dashboard` gains a field and every site is forced to name it

- **WHEN** the `NODEFAULT-UI` sweep runs over `src/` for `Dashboard`, `Filter`, `Detail`,
  `Refresh`, and `Launch`; again with its positive-control file parameterised to
  `src/agents.rs` for `Agent`, `Listed`, `AgentSnapshot`, and `Attribution`; and a third time
  with it parameterised to `src/launch.rs` for `Outcome`
- **THEN** none of the three runs finds an `impl Default`, a derived `Default`, or a `..` rest
  inside a literal or pattern of any of the ten types
- **AND** the compile-time companion destructures a `Dashboard` with an exhaustive pattern
  naming all **twelve** fields and no `..`, a `Launch` naming both, an `Agent` naming all
  **eight**, a `Listed` naming both, an `AgentSnapshot` naming all three, an `Attribution`
  naming all **three**, and a `launch::Outcome` naming both
- **AND** the parameterisation is a real change to the check rather than a second copy of it:
  the control file was hardcoded to `src/ui/app.rs`, and two versions of one check on disk is
  how a run and a record drift apart

#### Scenario: `Dashboard` still carries no thread, channel, or clock

- **WHEN** `Dashboard` is constructed in a test and cloned and compared
- **THEN** it is still `Clone`, `PartialEq`, and buildable with no thread, no filesystem, and no
  process: `AgentSnapshot`, `state::Mapping`, and `ui::app::Launch` are all plain data — a
  `launch::Request` names no trait, no channel, and no handle — and the poller and the launcher
  both reach the loop through `ui::driver::Live`, never through the state value

### Requirement: The composition root is driven by a test, not only read by a reviewer

`live-refresh` shipped a `ui::run` that never called `watch::start` or `refresh::start`. Every
test passed, because unit and view tests drove the seams directly and nothing exercised the
function that composes them; the whole live tier would have been permanently inert in the
shipped binary, and only Change Review caught it. `agent-polling` added a third collaborator
behind a third seam and could have reproduced that defect exactly, so the composition root SHALL
be split until a test can drive it.

`agent-attribution` inherited that exposure whole and extended it: a poll that reaches
`Dashboard.agents` and never reaches a rendered badge is the same defect one layer further on,
and unit and view tests cannot see it for the same reason. The outer-loop test SHALL therefore
assert a **rendered badge and a rendered count**, produced by the real poller
from a real scratch program, and every assertion it makes SHALL be one that a missing link can
turn red.

**`agent-launch` inherits it in its worst form yet**, and adds a requirement of its own: the
exposure is no longer only that a background tier is inert, but that a **key the reader presses
reaches nothing**. `launch::decide` can be perfect, `launch::start`'s worker can be perfect, and
`run_loop`'s dispatch step can be perfect, and the shipped binary can still never launch
anything, because no unit test presses a key against the function that composes them. The
outer-loop test SHALL therefore feed `run_wired` an actual `a` key event and observe the three
Herdr invocations, in order, at the seam — and an actual `g` event and observe the focus
invocation.

`ui::run` SHALL be split into three, beside the `load` `dashboard-loop` already owns:

```rust
pub struct Startup<'a> {
    pub cwd: &'a Path,
    pub config: &'a Config,
    pub herdr: &'a Path,
    pub state_dir: Option<&'a Path>,
}

pub struct Collaborators {
    pub fs: Box<dyn watch::FsEvents>,
    pub refresher: Box<dyn refresh::Refresher>,
    pub agents: Box<dyn agents::AgentPoll>,
    pub launcher: Box<dyn launch::Launcher>,
    pub problems: Vec<String>,
}

pub fn start_collaborators(
    repo: Option<&Path>, config: &Config, herdr: &Path, state_dir: Option<&Path>,
) -> Collaborators;

pub fn load(start: &Path, config: &Config, state_dir: Option<&Path>) -> Dashboard;

pub fn run_wired<B: Backend, E: EventSource>(
    terminal: &mut Terminal<B>, events: &mut E, startup: &Startup<'_>,
    read: ArtifactReader<'_>, tick: Duration,
) -> Result<Dashboard, StartError>;

pub fn run() -> Result<(), StartError>;
```

`Startup::state_dir` is `agent-attribution`'s addition: the directory `plugin-state` resolves
from `HERDR_PLUGIN_STATE_DIR`, `XDG_STATE_HOME`, or `HOME`, passed in as a **parameter** on
exactly `herdr`'s terms, so `run` performs the one environment read and a test drives
`run_wired` against a scratch state directory without touching the process environment.
`None` — no state directory could be resolved — SHALL be an ordinary case yielding an empty
mapping, never an error, and a launcher that reports a recording failure rather than refusing to
launch.

`load` SHALL read the mapping through `state::read(state_dir)` and place it on
`Dashboard::agent_names`. That read is the only new filesystem call the composition root makes,
and it lives in `src/ui/mod.rs` beside `read_artifact` for the same reason: every file under
`src/ui/` other than that one names no filesystem API.

`start_collaborators` SHALL start the real watcher and the real worker when a repository was
found and their inert doubles when one was not, and SHALL fold a watcher that would not start
into `problems`. It SHALL start the real **poller unconditionally**, with or without a
repository: Herdr agents exist independently of an OpenSpec repository, and `agent-launch` reads
`reachable` to decide whether to offer its keys in a pane that never found one. It SHALL start
the real **launcher when a repository was found** and `launch::none()` when one was not, on the
watcher's terms rather than the poller's: every launch argument vector carries the repository
root as `--cwd`, and a pane with no repository has no change to launch onto and no badge to
focus, so the inert double costs nothing and represents the truth.

`start_collaborators` SHALL take the state directory as a **fourth parameter**. That reverses
`agent-attribution`'s sentence — "`start_collaborators` SHALL NOT gain the state directory: the
mapping is state, not a collaborator" — and the reversal is stated rather than silent: with
`agent-launch` the mapping is **also** something a collaborator writes. `state::record` is a
filesystem write on a worker thread, and putting it anywhere but inside `src/launch.rs` would
either move a write onto the render path or put a second filesystem call into `src/ui/mod.rs`
that the loop would have to make between drains. The read stays `load`'s; the write is the
launcher's; the directory is passed to both.

`start_collaborators` SHALL pass `config.agent_kind` into the launcher, and **no production
file under `src/ui/` SHALL hold the literal `"claude"`**, on exactly the terms `agent-polling`
set for `"herdr"`. A hardcoded kind would ship a plugin that ignores its own documented
configuration key while every gate stayed green.

The `herdr` program is a **parameter** rather than the literal `"herdr"` precisely so a test
drives this function against a scratch `#!/bin/sh` program; `cli::HERDR_PROGRAM` holds the
literal and `run` is the only caller that passes it. The launcher and the poller SHALL be given
**separate** `Arc<dyn HerdrCli>` handles from two `cli::agent_cli_via` calls over that same
path, rather than sharing one: they run on different threads with different cadences, and
`agent_cli_via` constructs a value that holds only a path.

`run_wired` SHALL do everything `run` does once a terminal exists — `load`, then
`start_collaborators`, then move `problems` onto `dashboard.refresh.problems`, then build
`ui::driver::Live` from all **four** collaborators, then `run_loop` — and SHALL return the final
`Dashboard` so a test can assert on state the frame does not show. It takes a `Startup` struct
rather than four further parameters for cohesion, on the same grounds `Live` is a struct: the
four are one concept, "where this pane starts" — not clippy's `too_many_arguments`, which is
measured on this crate and toolchain to fire at **eight** parameters and would therefore be
tripped, not avoided, by the flattened form.

`run` SHALL then hold **no branch, no loop, and no field selection**: the terminal guard, the
panic hook, `config::load_from_env`, `state::state_dir(&config::env_lookup())`,
`startup_dir(&config::env_lookup(), &|| std::env::current_dir())`, the real `Terminal`,
`CrosstermEvents`, the `HERDR_PROGRAM` literal, `config::env_lookup()` and `cli::npm_prefix`
passed into `Startup` (`openspec-binary`), `read_artifact`, `TICK`, and one `run_wired` call.
Everything that can be miswired is below that line and is driven by a test.

`Startup` SHALL therefore carry **six** fields rather than four: `cwd`, `config`, `herdr`,
`state_dir`, and `degraded-states`' additions, the environment lookup and the `npm prefix -g`
hook. They belong to the same one concept the struct already names — "where this pane starts"
— and they are what lets an outer test drive a *failing* binary probe without consulting the
developer's machine (`openspec-binary`).

`startup_dir` is `degraded-states`' repair of a rule `plugin-actions` broke. Its cwd-resolution
fix wrote `match startup_cwd(&config::env_lookup()) { Some(cwd) => cwd, None =>
std::env::current_dir()? }` **into `run`'s own body**, which is a branch, and the `WIRED` check
went red on `main` and stayed red for a change — the check lived outside `make check`, so
nothing forced it to run. `degraded-states` both moves the decision and makes the check a
repository file (`quality-gates`).

`ui::startup_dir(env: &dyn Fn(&str) -> Option<String>, fallback: &dyn Fn() -> std::io::Result<PathBuf>)
-> std::io::Result<PathBuf>` SHALL hold that decision: `startup_cwd(env)` when it yields a
path, `fallback()` otherwise. Both arms SHALL be driven by a test, which is the whole point of
moving it — `run`'s own body reaches no test, so a decision left there is a decision nothing
ever runs. The fallback arrives as an **injected** closure rather than being called directly,
for the reason `AGENTS.md` already gives for the environment lookup: `cargo test` runs tests in
parallel threads of one process, so a test that changed the real working directory would
corrupt its neighbours. `run` is the one caller that passes `&|| std::env::current_dir()`.

Moving it into `startup_dir` rather than into `run_wired` is deliberate. `Startup::cwd` is a
`&Path` that every acceptance test and every construction site already builds; widening it to
`Option<&Path>` to let `run_wired` resolve the fallback would ripple through all of them to
make one two-line decision testable, and would put an `std::env` read inside the function
whose whole purpose is to be driveable from a test. A separately named, separately tested
function is the same guarantee at a fraction of the blast radius: the residue in `run` is a
call, and the decision is somewhere a test reaches.

Neither `run_wired` nor `start_collaborators` SHALL name `OpenspecCli`, `HerdrCli`, `from_cli`,
`CliChanges`, or `npm_prefix`: the handles arrive from `cli::worker_cli_from_env`,
`cli::agent_cli_via`, and `launch::start`, whose own signatures carry those types, which is the
mechanical reason `NOCLI-SHELL` stays green while the shell is wired to two real binaries.

#### Scenario: The real wiring polls a scratch Herdr and adopts what it says

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 over a `testutil::ScratchDir`
  repository holding one active change `alpha` whose `tasks.md` counts 4 of 9, with the **real**
  `ui::read_artifact`, a `Startup` naming that repository and a scratch state directory, a
  `Config` whose `openspec_bin` names a **usable** scratch `#!/bin/sh` program — so the binary
  probe chain stops at its first step and the real `openspec` binary is never spawned — a
  `herdr` path naming a second scratch program printing a one-agent `agent_list` envelope, and a
  `testutil::UntilReady` event source whose readiness predicate (a) waits for both scratch
  programs to log their first invocation, (b) then writes one byte to
  `openspec/changes/alpha/proposal.md`, and (c) then waits for the scratch `openspec` program's
  log to reach **two** invocations, before pressing `q`
- **THEN** `run_wired` returns `Ok(dashboard)` whose `agents.reachable` is true and whose
  `agents.agents` holds exactly one agent with `pane_id` `w8:p1`, proving `agents::start` was
  wired rather than `agents::none()`
- **AND** the scratch `herdr` program's invocation log records at least one run whose arguments
  were exactly `agent list`, and **no** run whose arguments began `pane split`: polling alone
  launches nothing
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

#### Scenario: A polled agent reaches a rendered badge and a rendered count

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 over a `testutil::ScratchDir`
  repository holding two active changes — `2fa-support` and `alpha`, each with a `tasks.md`
  counting 4 of 9 — with a scratch state directory whose `agent-names.toml` holds
  `c-2fa-support = "2fa-support"`, and a scratch `herdr` program printing an `agent_list`
  envelope of **four** agents whose `cwd` is the repository root for three of them and
  `/definitely/elsewhere` for the fourth: `c-2fa-support` `working`, `alpha` `blocked`, one with
  no `name` at all and `agent` `claude` and status `idle`, and — the fourth, out of scope —
  one named `nothing-like-a-change` and `working`. The fourth agent's name is deliberately
  chosen to match **no** change on screen: a name that collided with `alpha` would still be
  folded into `alpha`'s badge once the repository-scope test were removed (precedence, not the
  count, would absorb it), which would make the scope-removal control below pass for the wrong
  reason — measured, and corrected here rather than left to be rediscovered at group 8
- **THEN** the list row for `2fa-support` carries the badge character `w` and the row for
  `alpha` carries `b`, at both widths, in the badge column `change-rows` specifies
- **AND** the footer row reads exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` at 120 and
  `q quit  Enter detail  Esc back  a/c/s launch  g focus` at 60 — the socket is reachable, so
  `agent-launch`'s two hints are offered, and at 60 the count no longer fits and is dropped
  whole. The unnamed in-scope agent is counted at 120, and the out-of-scope agent named
  `nothing-like-a-change` is neither counted nor badged at either width
- **AND** that count is discriminating in **both** directions at 120 — it reads `0` if the
  poller was never wired or the mapping was never read, and `2` if the repository-scope test
  were removed — which is what the landed `refresh.problems` assertion was not
- **AND** `2fa-support`'s badge is discriminating on its own: it appears only if the state
  directory was read, the mapping resolved, and the badge rendered, so replacing
  `Startup::state_dir` with `None` turns exactly that assertion red while `alpha`'s stays green
- **AND** the readiness predicate waits for the scratch `herdr` program's log to hold at least
  one entry and writes nothing to the repository, so the run makes the byte-identity claim the
  scenario below makes rather than forfeiting it

#### Scenario: The wiring test fails when the poller is replaced by the inert double

- **WHEN** `start_collaborators` is edited to pass `agents::none()` in place of
  `agents::start(...)`, and the two scenarios above are run
- **THEN** both fail, the first naming `agents.reachable` as false and `agents.agents` as empty,
  and the second naming a footer that reads `q quit  Enter detail  Esc back` — with neither the
  action hints nor the count, since `reachable` is false — and list rows carrying no badge
- **AND** this plant is run and recorded during implementation, and the edit reverted, because a
  wiring test that cannot fail is exactly the artifact that let `live-refresh`'s inert live tier
  through
- **AND** the same plant is repeated for `watch::none()` in place of `watch::start`, and for
  `refresh::none()` in place of `refresh::start` — the two collaborators that were actually
  missing in `live-refresh` — and each is observed red: the watcher plant on the scratch
  `openspec` log stopping at **one** entry, and the worker plant on that log being empty
- **AND** three further plants are run and recorded against the badge scenario alone, each
  reverted: `load` passing `Mapping::default()` instead of `state::read(state_dir)`, which turns
  the `2fa-support` badge red and leaves `alpha`'s green; `ui::list::rows` ignoring
  `attribution()`, which turns both badges red and leaves the footer green; and
  `ui::view`'s footer ignoring `attribution()`, which turns the count red and leaves both badges
  green. Each names a different link, so no single assertion is standing in for the whole chain
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
- **AND** the buffer is **identical** to the reachable run's at both widths in the list region,
  and differs from it in the footer alone — the two action hints and the count are absent — so
  nothing is hidden, no error screen replaces the pane, and the difference is exactly the one
  `agent-launch` introduced. The list still shows `alpha` at `[4/9]`
- **AND** no row carries a badge and the footer carries no count and no action hint, so an
  unreachable socket renders the agentless pane exactly rather than an empty-looking agent
  feature. The badge half is a supporting assertion and **not** a discriminating one — an
  unwired poller satisfies it too — and the badge scenario above is where the wiring is actually
  proved
- **AND** `run_wired` returns `Ok`, not `Err`: an unreachable socket is a supported state and
  never a `StartError`
- **AND** the change tree is byte-identical before and after — every path, its bytes, and its
  modification time — with a discriminating control rewriting one byte of `tasks.md` between two
  further snapshots and asserting they differ. This run holds a real watcher open on the tree
  while a real worker, a real poller, and a real launcher run beside it, and writes nothing,
  which is where `live-updates`' "the live tier writes nothing inside the repository" is proved
  for a four-collaborator tier
- **AND** the scratch **state** directory is byte-identical before and after too, by the same
  two snapshots: nothing was launched, so nothing was recorded

#### Scenario: A pane with no OpenSpec repository still polls for agents

- **WHEN** `run_wired` is driven at 120x20 over a `testutil::ScratchDir` holding **no**
  `openspec/` directory, with the same two scratch programs, a `Startup` whose `state_dir` is
  `None`, and a readiness predicate naming the `herdr` log alone
- **THEN** `run_wired` returns `Ok(dashboard)` with `repo` `None`, the list region's
  no-repository empty state on screen, and `agents.reachable` **true** with one agent
- **AND** the scratch `openspec` program's log is **empty** and `refresh.problems` is empty: with
  no repository, `refresh::start` and `watch::start` are correctly the inert doubles and cost
  nothing — no thread, no process, no watch — and `launch::none()` is the launcher for the same
  reason, so no third worker thread starts either
- **AND** `dashboard.agent_names.names` is empty and `dashboard.agent_names.problems` is empty:
  an unresolvable state directory is an ordinary absent input, not a fault
- **AND** the footer carries no count and the three no-repository rows carry no badge, because
  with no repository no agent is in scope, however many the poller returned
- **AND** the footer **does** carry `a/c/s launch` and `g focus`, because the socket is
  reachable — and pressing any of the four keys still does nothing, because there is no selected
  change and no badge, which the event source proves by pressing `a` and `g` before `q` and the
  assertions prove by the scratch `herdr` log holding only `agent list` runs
- **AND** this is the asymmetry the requirement states: the watcher, the worker, and the
  launcher are about a repository, the poller is about the Herdr session, and only the first
  three are skipped

#### Scenario: The residue left in `run` is small enough to read

- **WHEN** `src/ui/mod.rs`'s production slice is searched for `run_wired`,
  `start_collaborators`, `watch::start`, `refresh::start`, `agents::start`, `launch::start`,
  `worker_cli_from_env`, `agent_cli_via`, and `state::read`
- **THEN** every one of the nine is present, so a collaborator dropped from the wiring — or a
  mapping read dropped from `load` — is a failing grep as well as a failing test
- **AND** `src/ui/mod.rs` names none of `OpenspecCli`, `HerdrCli`, `from_cli`, `CliChanges`, or
  `npm_prefix`, which `NOCLI-SHELL` already checks over the whole of `src/ui/`
- **AND** the check is paired with a positive control — `src/cli.rs` **does** name
  `agent_cli_via` — so a renamed binding fails the control rather than leaving the sweep
  searching for a name that is no longer there
- **AND** a second positive control asserts that `src/state.rs` **does** define
  `pub fn read(`, anchored, so the mapping-read name is not a search for a function that has
  moved, and a **third** asserts that `src/launch.rs` **does** define `pub fn start(`,
  anchored, for the same reason
- **AND** `pub fn run()`'s own body resolves `state::state_dir(` and does **not** hold the
  literal `state_dir: None`, which is the link no test reaches: every acceptance test builds its
  own `Startup`, so a shipped `None` would leave the mapping tier dead with every other gate
  green
- **AND** `pub fn run()`'s own body names `startup_dir(`, and holds none of the keywords `if`,
  `match`, `for`, `while`, `loop`, or `else` — the two halves together, because either alone
  passes on a defect: a body with no keyword but no `startup_dir` call has silently dropped the
  workspace cwd and reopened the bug `plugin-actions` fixed, and a body naming `startup_dir`
  beside a reinstated `match` has grown the residue back
- **AND** `start_collaborators`'s own body names `config.agent_kind`, and **no** production
  slice under `src/ui/` holds the literal `"claude"` — the same shape as the `"herdr"` rule,
  guarding the same class of defect one configuration key over. Both halves are proven able to
  fail at planning time, by a copy passing `"claude".to_string()` and by a copy dropping the
  `agent_kind` read

#### Scenario: `startup_dir` prefers the workspace cwd and falls back when there is none

- **WHEN** `ui::startup_dir` is called with an environment lookup that yields a
  `HERDR_PLUGIN_CONTEXT_JSON` naming a workspace cwd of `/tmp/workspace-a`, and a fallback
  closure that would return `/tmp/never-used`
- **THEN** it returns `Ok("/tmp/workspace-a")`
- **AND** the fallback closure was **not** called, proved by a counter the closure increments
- **AND** the same call with a lookup that yields nothing returns `Ok("/tmp/never-used")` with
  the counter at one, so both arms are driven and the preference is the resolution's and not
  the closure's

#### Scenario: `startup_dir` propagates a failing fallback rather than panicking

- **WHEN** `ui::startup_dir` is called with a lookup that yields no workspace cwd and a
  fallback closure returning `Err(std::io::Error::from(std::io::ErrorKind::NotFound))`
- **THEN** it returns that `Err` unchanged
- **AND** it does not panic, and does not substitute a path of its own — a process whose
  working directory has been removed is a real state, and `run`'s `?` is what turns it into
  the exit status

#### Scenario: A Herdr context with no workspace cwd is the fallback case, not a failure

- **WHEN** `ui::startup_dir` is called with a lookup yielding a `HERDR_PLUGIN_CONTEXT_JSON`
  that parses but carries **no** workspace cwd, and separately with one that does not parse at
  all
- **THEN** both return the fallback's path, not an error
- **AND** neither reports a problem: `open::context`'s `workspace_id` requirement is
  deliberately ignored here, and `SPEC.md` → Degraded states' last row states exactly this —
  the dashboard falls back to its own process cwd rather than refusing

### Requirement: A poll that has not answered within a bounded number of intervals is reported

`RealAgentPoll` holds at most one poll in flight and clears the flag only when an answer
arrives. A wedged Herdr socket therefore parks the worker inside the seam's spawn
indefinitely: `in_flight` stays set, `drain` never yields, `pending_in` stays `None`, and —
because the worker is alive and its channel connected — the landed dead-worker path never
fires either. The pane renders its **last** snapshot for the rest of the session: stale
badges, `reachable: true`, action keys still offered, and no signal anywhere that the
information is minutes old. This is the one failure mode in the whole live tier that
produces confidently wrong content rather than degraded content.

`agents::STALL_AFTER` SHALL be a named constant, pinned by an assertion the way
`agents::POLL_INTERVAL` is, and SHALL be **five** `POLL_INTERVAL`s — five seconds. Five is
chosen against the measured 8-millisecond median for `herdr agent list`: it is three orders
of magnitude above the normal answer and still well inside a human's patience, and it sits
below `cli::RUN_DEADLINE` (60 seconds), which is the seam's own eventual resolution of the
same hang.

`RealAgentPoll::drain` SHALL, using the clock reading it already takes once per call,
compare `now` against the instant the outstanding poll was sent. When a poll has been in
flight for at least `STALL_AFTER` and the stall has not yet been reported for this episode,
`drain` SHALL return

```rust
Some(AgentSnapshot { agents: vec![], reachable: false, stalled: true, problem: Some(reason) })
```

where `reason` names the command and the elapsed interval, and SHALL mark the episode
reported so the same stall is announced exactly once rather than on every frame.

`AgentSnapshot` SHALL gain the field `stalled: bool`, and SHALL keep its existing rule that
it implements no `Default` and that every construction and destructuring names every field
with no `..` rest, so a stall cannot be introduced or dropped implicitly anywhere in the
crate. `stalled` SHALL be `false` on every snapshot the worker produces, including a
failure: an `herdr` that answers with an error is reachable and answering, which is a
different fact from one that does not answer at all.

Emptying `agents` and clearing `reachable` is deliberate and is the point of the
requirement: it withdraws the badges the pane can no longer stand behind, and it withdraws
the `a`/`c`/`s`/`g` keys, which `agent-launch` offers only while the socket is reachable —
pressing one against a wedged socket would queue a launch behind the very call that is
stuck. `stalled` is what separates this from the ordinary unreachable-socket state, which is
silent by design: `live-updates` renders a stalled snapshot's `problem` as a leading
`!`-marked row, and a non-stalled one's not at all.

When the outstanding poll finally answers, the answer SHALL be delivered normally, SHALL
clear `in_flight` and the reported-stall marker, and SHALL replace the stalled snapshot
wholesale — so a socket that recovers restores the badges and the keys on the next frame
with no further action from the user.

`pending_in` SHALL continue to return `None` while a poll is in flight, stalled or not:
there is no deadline to wake for, and the loop's own `TICK` already brings it back.

The poller SHALL NOT cancel, retry, or re-issue the outstanding poll. Cancellation belongs
to the seam's `RUN_DEADLINE` (`subprocess-seam`), and a second `herdr` spawn against a wedged
socket is one more stuck process, not a recovery.

#### Scenario: A poll outstanding past the stall threshold is announced once

- **WHEN** a poller built over a `FakeCli` whose `agent list` never answers is drained
  repeatedly with an injected clock advanced past `STALL_AFTER` between drains, for ten
  drains in total
- **THEN** exactly one `drain` returns `Some(AgentSnapshot { agents: [], reachable: false,
  stalled: true, problem: Some(text) })`, and `text` names `herdr agent list` and the
  elapsed interval
- **AND** every other `drain` returns `None`, so the stall costs one row rather than one row
  per frame
- **AND** no second `agent list` request reached the worker across all ten drains

#### Scenario: A poll answering normally never reports a stall

- **WHEN** a poller over a `FakeCli` that answers immediately is drained, its answer taken,
  and the sequence repeated three times with the injected clock advanced by one
  `POLL_INTERVAL` between rounds
- **THEN** every delivered snapshot carries `stalled: false`
- **AND** no snapshot carries an empty `agents` list while the fake was reporting agents, so
  the stall path cannot fire on a healthy socket

#### Scenario: A stalled socket that recovers restores the badges

- **WHEN** a stall has been reported, and the outstanding poll then answers with two agents
- **THEN** the next `drain` returns `Some(AgentSnapshot { agents: [two agents], reachable:
  true, stalled: false, problem: None })`
- **AND** a subsequent stall in a later episode is reported again, so the marker is
  per-episode rather than once per process

#### Scenario: An `herdr` that answers with an error is not a stall

- **WHEN** a poller over a `FakeCli` whose `agent list` returns `Err(CliError::Failed { .. })`
  is drained and its answer taken
- **THEN** the snapshot carries `reachable: false`, `problem: Some(Herdr's own reason)`, and
  `stalled: false`
- **AND** it therefore renders as the landed silent standalone-TUI state and not as a
  leading `!`-marked row, because an answering socket that says no is a different fact from
  one that says nothing

#### Scenario: The stall threshold is a named constant and is asserted

- **WHEN** `agents::STALL_AFTER` is read
- **THEN** it equals `5 * agents::POLL_INTERVAL`, written as that product rather than as a
  bare `Duration::from_secs(5)`, so moving the poll interval moves the threshold with it
- **AND** it is strictly less than `cli::RUN_DEADLINE`, asserted, so the pane always tells
  the user about a hang before the seam gives up on it
