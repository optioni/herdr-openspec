## MODIFIED Requirements

### Requirement: The composition root is driven by a test, not only read by a reviewer

`live-refresh` shipped a `ui::run` that never called `watch::start` or `refresh::start`. Every
test passed, because unit and view tests drove the seams directly and nothing exercised the
function that composes them; the whole live tier would have been permanently inert in the
shipped binary, and only Change Review caught it. `agent-polling` adds a third collaborator
behind a third seam and can reproduce that defect exactly, so the composition root SHALL be
split until a test can drive it.

`agent-attribution` inherits that exposure whole and extends it: a poll that reaches
`Dashboard.agents` and never reaches a rendered badge is the same defect one layer further on,
and unit and view tests cannot see it for the same reason. The outer-loop test SHALL therefore
be extended to assert a **rendered badge and a rendered count**, produced by the real poller
from a real scratch program, and every assertion it makes SHALL be one that a missing link can
turn red.

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
    pub problems: Vec<String>,
}

pub fn start_collaborators(repo: Option<&Path>, config: &Config, herdr: &Path) -> Collaborators;

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
mapping, never an error.

`load` SHALL read the mapping through `state::read(state_dir)` and place it on
`Dashboard::agent_names`. That read is the only new filesystem call the composition root makes,
and it lives in `src/ui/mod.rs` beside `read_artifact` for the same reason: every file under
`src/ui/` other than that one names no filesystem API.

`start_collaborators` SHALL start the real watcher and the real worker when a repository was
found and their inert doubles when one was not, and SHALL fold a watcher that would not start
into `problems`. It SHALL start the real **poller unconditionally**, with or without a
repository: Herdr agents exist independently of an OpenSpec repository, and `agent-launch` reads
`reachable` to decide whether to offer its keys in a pane that never found one. The `herdr`
program is a **parameter** rather than the literal `"herdr"` precisely so a test drives this
function against a scratch `#!/bin/sh` program; `cli::HERDR_PROGRAM` holds the literal and `run`
is the only caller that passes it. `start_collaborators` SHALL NOT gain the state directory:
the mapping is state, not a collaborator, and it is read by `load`.

`run_wired` SHALL do everything `run` does once a terminal exists — `load`, then
`start_collaborators`, then move `problems` onto `dashboard.refresh.problems`, then build
`ui::driver::Live` from all three collaborators, then `run_loop` — and SHALL return the final
`Dashboard` so a test can assert on state the frame does not show. It takes a `Startup` struct
rather than four further parameters for cohesion, on the same grounds `Live` is a struct: the
four are one concept, "where this pane starts", and bundling them keeps the function at five
arguments. The struct is **not** justified by clippy's `too_many_arguments`: measured on this
crate and toolchain, that lint fires at **eight** parameters, not seven, so the flattened form
would not have tripped it either. `agent-polling` retired that claim in its design and left a
copy of it in this function's doc comment; `agent-attribution` deletes the copy, because a
false rationale left in shipped source is how the claim was propagated in the first place.

`run` SHALL then hold **no branch, no loop, and no field selection**: the terminal guard, the
panic hook, `config::load_from_env`, `state::state_dir(&config::env_lookup())`, `current_dir`,
the real `Terminal`, `CrosstermEvents`, the `HERDR_PROGRAM` literal, `read_artifact`, `TICK`,
and one `run_wired` call. Everything that can be miswired is below that line and is driven by a
test.

Neither `run_wired` nor `start_collaborators` SHALL name `OpenspecCli`, `HerdrCli`, `from_cli`,
`CliChanges`, or `npm_prefix`: the handles arrive from `cli::worker_cli_from_env` and
`cli::agent_cli_via`, whose own signatures carry those types, which is the mechanical reason
`NOCLI-SHELL` stays green while the shell is wired to two real binaries.

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
- **AND** the footer row reads exactly `q quit  Enter detail  Esc back  1 unattributed` at both
  widths: the unnamed in-scope agent is counted, and the out-of-scope agent named
  `nothing-like-a-change` is neither counted nor badged
- **AND** that count is discriminating in **both** directions — it reads `0` if the poller was
  never wired or the mapping was never read, and `2` if the repository-scope test were removed —
  which is what the landed `refresh.problems` assertion was not
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
  and the second naming a footer that reads `q quit  Enter detail  Esc back` with no count and
  list rows carrying no badge
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
- **AND** the buffer is **identical** to the reachable run's at both widths: the list still shows
  `alpha` at `[4/9]`, nothing is hidden, and no error screen replaces the pane
- **AND** no row carries a badge and the footer carries no count, so an unreachable socket
  renders the agentless pane exactly rather than an empty-looking agent feature. This is a
  supporting assertion and **not** a discriminating one — an unwired poller satisfies it too —
  and the badge scenario above is where the wiring is actually proved
- **AND** `run_wired` returns `Ok`, not `Err`: an unreachable socket is a supported state and
  never a `StartError`
- **AND** the change tree is byte-identical before and after — every path, its bytes, and its
  modification time — with a discriminating control rewriting one byte of `tasks.md` between two
  further snapshots and asserting they differ. This run holds a real watcher open on the tree
  while a real worker and a real poller run beside it, and writes nothing, which is where
  `live-updates`' "the live tier writes nothing inside the repository" is proved for a
  three-collaborator tier
- **AND** the scratch **state** directory is byte-identical before and after too, by the same
  two snapshots: this change reads `agent-names.toml` and never writes it, and `agent-launch` is
  the change that will

#### Scenario: A pane with no OpenSpec repository still polls for agents

- **WHEN** `run_wired` is driven at 120x20 over a `testutil::ScratchDir` holding **no**
  `openspec/` directory, with the same two scratch programs, a `Startup` whose `state_dir` is
  `None`, and a readiness predicate naming the `herdr` log alone
- **THEN** `run_wired` returns `Ok(dashboard)` with `repo` `None`, the list region's
  no-repository empty state on screen, and `agents.reachable` **true** with one agent
- **AND** the scratch `openspec` program's log is **empty** and `refresh.problems` is empty: with
  no repository, `refresh::start` and `watch::start` are correctly the inert doubles and cost
  nothing — no thread, no process, no watch
- **AND** `dashboard.agent_names.names` is empty and `dashboard.agent_names.problems` is empty:
  an unresolvable state directory is an ordinary absent input, not a fault
- **AND** the footer carries no count and the three no-repository rows carry no badge, because
  with no repository no agent is in scope, however many the poller returned
- **AND** this is the asymmetry the requirement states: the watcher and the worker are about a
  repository, the poller is about the Herdr session, and only the first two are skipped

#### Scenario: The residue left in `run` is small enough to read

- **WHEN** `src/ui/mod.rs`'s production slice is searched for `run_wired`,
  `start_collaborators`, `watch::start`, `refresh::start`, `agents::start`,
  `worker_cli_from_env`, `agent_cli_via`, and `state::read`
- **THEN** every one of the eight is present, so a collaborator dropped from the wiring — or a
  mapping read dropped from `load` — is a failing grep as well as a failing test
- **AND** `src/ui/mod.rs` names none of `OpenspecCli`, `HerdrCli`, `from_cli`, `CliChanges`, or
  `npm_prefix`, which `NOCLI-SHELL` already checks over the whole of `src/ui/`
- **AND** the check is paired with a positive control — `src/cli.rs` **does** name
  `agent_cli_via` — so a renamed binding fails the control rather than leaving the sweep
  searching for a name that is no longer there
- **AND** a second positive control asserts that `src/state.rs` **does** define
  `pub fn read(`, anchored, so the eighth name is not a search for a function that has moved

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
and a pane one tick old both have **no agents**, and every rendering this plugin does of the
agent tier is driven by `agents.agents` being empty rather than by `reachable` — the badge and
the count both vanish on their own. `agent-launch` is the change that reads `reachable`
directly, to hide its action keys, and hiding them for the first second of a pane's life is
the right behaviour.

A snapshot SHALL replace the field wholesale, never merge into it: a poll that found no agents
means there are no agents.

**`agent-attribution` is the change that reads this field, and it is the only one.** The
requirement's earlier form — no view SHALL read it — held for exactly one change, which is why
`agent-polling` landed the state and the render separately: a buffer assertion that moved could
then only have been broken by one of them. From this change onward `ui::list` and `ui::view`
read it, and **only** through `Dashboard::attribution()`, whose output is keyed by change name;
`ui::detail`, `ui::tasks`, and `ui::markdown` still do not read it at all.

An agent that `agent-attribution` places **out of scope** — one whose `cwd` is absent, or lies
outside the resolved repository root — SHALL still change no pixel. That is what keeps this
requirement's byte-identity scenario meaningful rather than merely historical.

#### Scenario: A polled snapshot changes no pixel at either width

- **WHEN** the same `Dashboard`, whose repository root is `/tmp/demo-repo`, is rendered at
  120x20 and at 60x20, once with `agents` in its initial state, once with `agents` holding two
  reachable agents whose `cwd` is `/somewhere/else` and whose names match changes on screen,
  once with `agents` holding one reachable agent carrying **no** `cwd` at all, and once with
  `agents` holding an unreachable-socket snapshot whose `problem` is a long string
- **THEN** the four buffers are identical at each width, cell for cell
- **AND** the agents in the second and third cases are out of scope — a `cwd` outside the
  repository root, and an absent `cwd` — so `agent-attribution` badges nothing and counts
  nothing, and the byte-identity claim is about the scope rule rather than about the field
  being unread
- **AND** the discriminating control is the same dashboard with one of those agents moved to
  `cwd` `/tmp/demo-repo`: that buffer **differs** at both widths, so the identity above is a
  scope branch and not a constant
- **AND** in particular the unreachable-socket `problem` appears nowhere on screen, and no
  `!`-marked row is added: `SPEC.md` → Degraded states records an unreachable socket as
  "runs as a standalone TUI", which is silence, not a message
- **AND** the 60-column case is asserted as well as the 120-column one, so the responsive
  breakpoint is covered rather than assumed

#### Scenario: `Dashboard` gains a field and every site is forced to name it

- **WHEN** the `NODEFAULT-UI` sweep runs over `src/` for `Dashboard`, `Filter`, `Detail`, and
  `Refresh`, and again with its positive-control file parameterised to `src/agents.rs` for
  `Agent`, `Listed`, `AgentSnapshot`, and `Attribution`
- **THEN** neither run finds an `impl Default`, a derived `Default`, or a `..` rest inside a
  literal or pattern of any of the eight types
- **AND** the compile-time companion destructures a `Dashboard` with an exhaustive pattern
  naming all **eleven** fields and no `..`, an `Agent` naming all **eight**, a `Listed` naming
  both, an `AgentSnapshot` naming all three, and an `Attribution` naming both
- **AND** the parameterisation is a real change to the check rather than a second copy of it:
  the control file was hardcoded to `src/ui/app.rs`, and two versions of one check on disk is
  how a run and a record drift apart

#### Scenario: `Dashboard` still carries no thread, channel, or clock

- **WHEN** `Dashboard` is constructed in a test and cloned and compared
- **THEN** it is still `Clone`, `PartialEq`, and buildable with no thread, no filesystem, and no
  process: `AgentSnapshot` and `state::Mapping` are both plain data, and the poller itself
  reaches the loop through `ui::driver::Live`, never through the state value
