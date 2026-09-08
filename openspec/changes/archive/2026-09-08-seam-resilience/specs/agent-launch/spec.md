## MODIFIED Requirements

### Requirement: The launch decision is a pure, total function that refuses before it reaches Herdr

`launch::decide` SHALL hold the whole policy of what a launch key does, and SHALL be a pure
total function of its six arguments:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent { Apply, Continue, Archive, Focus }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Launch { change: String, agent: String, intent: Intent },
    Focus { pane_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision { Nothing, Refuse(String), Go(Request) }

pub fn decide(
    intent: Intent,
    change: Option<&str>,
    pane: Option<&str>,
    reachable: bool,
    live_names: &[&str],
    in_flight: bool,
) -> Decision;
```

It SHALL perform no filesystem, process, environment, network, or terminal I/O, SHALL read no
clock and no global state, SHALL spawn nothing, and SHALL never panic for any combination of
arguments, including an empty `change`, an empty `pane`, an empty `live_names`, and every
`Intent`.

`Decision::Nothing` means the key does nothing at all — no Herdr call, no problem row, no state
change beyond clearing nothing. `Decision::Refuse` means the key is answered with a problem
string and no Herdr call whatsoever. `Decision::Go` means the request is handed to the launcher.

The order of decision SHALL be exactly:

1. `reachable == false` → `Nothing`. An unreachable socket is the documented standalone-TUI
   state; the action keys are not offered, so pressing one is not an error.
2. `Intent::Focus` with `pane` `None` → `Nothing`; with `pane` `Some(p)` →
   `Go(Request::Focus { pane_id: p })`. Focus is exempt from the in-flight test below: it
   splits no pane, starts no agent, and is exactly what a user waiting on a slow launch
   should still be able to press.
3. `change` `None` → `Nothing`. There is nothing selected to launch onto.
4. `in_flight == true` → `Refuse`, naming that a launch is already running and to wait.
5. The derived agent name, `state::agent_name(change)`, appearing in `live_names` →
   `Refuse`, naming the derived name and pointing at `g`.
6. Otherwise → `Go(Request::Launch { change, agent, intent })`.

Step 4 is new and closes a window the `live_names` test at step 5 cannot: `live_names` comes
from the **last** `herdr agent list` snapshot, and `herdr agent start` is measured to block
for up to **thirty seconds** waiting for interactive readiness. Between the press and the
agent appearing in a poll, `live_names` does not contain the derived name, the launcher's
request channel is unbounded, and `Dashboard::launch.pending` is a one-shot slot drained
every iteration — so a second press runs a **second** full `pane split` + `agent start` +
`agent prompt` sequence in series behind the first. The observable damage is a stray pane
that nothing ever closes plus an `agent start` that collides on the duplicate name. The
vulnerable window is the whole launch duration plus poll lag, not the one-second poll
interval.

Step 4 precedes step 5 because an in-flight launch is the more specific and more recent
fact; a name already live is a standing condition the poller reports.

`decide` SHALL NOT consult a terminal title, an agent kind, a pane's working directory, or any
field other than the six arguments above.

#### Scenario: An unreachable socket makes every action key inert

- **WHEN** `decide` is called with `reachable` `false`, `change` `Some("add-auth")`, `pane`
  `Some("w8:p3")`, an empty `live_names`, and `in_flight` `false`, once for each of `Apply`,
  `Continue`, `Archive`, and `Focus`
- **THEN** all four return `Decision::Nothing`
- **AND** no `Request` is produced on any of the four calls, so no Herdr call can follow and no
  problem row is produced either — an unreachable socket is silent, not a fault

#### Scenario: No selected change means no launch, and no agent means no focus

- **WHEN** `decide` is called with `reachable` `true`, `change` `None`, `pane` `None`, an
  empty `live_names`, and `in_flight` `false`, for `Apply`, `Continue`, and `Archive`
- **THEN** all three return `Decision::Nothing`
- **AND** the same call for `Focus` also returns `Decision::Nothing`
- **AND** the call for `Focus` with `pane` `Some("w8:p3")` and `change` `None` returns
  `Decision::Go(Request::Focus { pane_id: "w8:p3" })`, so focusing needs a pane and not a
  selection

#### Scenario: Each launch intent carries its own change and derived name

- **WHEN** `decide` is called with `reachable` `true`, `change` `Some("2fa-support")`, `pane`
  `None`, an empty `live_names`, and `in_flight` `false`, once for each of `Apply`,
  `Continue`, and `Archive`
- **THEN** each returns `Decision::Go(Request::Launch { change: "2fa-support", agent:
  "c-2fa-support", intent })` with that call's own `intent`
- **AND** the three results differ **only** in `intent`, so the change and the derived name are
  computed once and identically for all three keys

#### Scenario: A derived name already live in the session is refused before any Herdr call

- **WHEN** `decide` is called with `reachable` `true`, `change` `Some("2fa-support")`,
  `in_flight` `false`, and `live_names` `["c-2fa-support", "other"]`
- **THEN** it returns `Decision::Refuse(reason)` where `reason` names `c-2fa-support` and tells
  the reader to press `g`
- **AND** no `Request` is produced, so no pane is split and no stray pane can be left behind —
  the refusal is what keeps the common repeat-press from leaking a pane
- **AND** the same call with `live_names` `["c-2fa-support-x", "2fa-support"]` returns
  `Decision::Go`, because the match is on the **derived** name byte for byte and neither of
  those is it

#### Scenario: A second press while a launch is in flight is refused, not queued

- **WHEN** `decide` is called with `reachable` `true`, `change` `Some("2fa-support")`, an
  **empty** `live_names` — the poller has not yet seen the agent, which is the whole point —
  and `in_flight` `true`, once for each of `Apply`, `Continue`, and `Archive`
- **THEN** all three return `Decision::Refuse(reason)` whose text says a launch is already
  running and to wait
- **AND** no `Request` is produced on any of the three, so no second `pane split` can run and
  no stray pane is left behind
- **AND** the same three calls with `in_flight` `false` return `Decision::Go`, so the refusal
  is caused by the flag and by nothing else in the fixture

#### Scenario: Focus still works while a launch is in flight

- **WHEN** `decide` is called with `Intent::Focus`, `reachable` `true`, `pane`
  `Some("w8:p3")`, and `in_flight` `true`
- **THEN** it returns `Decision::Go(Request::Focus { pane_id: "w8:p3" })`
- **AND** the user waiting through a thirty-second `agent start` can still press `g` to look
  at an agent already running, which is the behaviour the in-flight guard must not take away

#### Scenario: Every combination is total

- **WHEN** `decide` is called with `change` `Some("")`, `pane` `Some("")`, `reachable` `true`,
  `live_names` `[""]`, and `in_flight` both `false` and `true`, once per `Intent` in each case
- **THEN** none of the eight panics
- **AND** with `in_flight` `false`, `Apply` returns `Decision::Go` carrying the derived name
  `change` — `state::agent_name` maps the empty string to that fallback — while `Focus`
  returns `Decision::Go(Request::Focus { pane_id: "" })`, both of which the launcher then
  fails on honestly rather than the decision guessing on the caller's behalf
- **AND** with `in_flight` `true`, `Apply` returns `Decision::Refuse` and `Focus` still
  returns `Decision::Go`

### Requirement: The launcher seam is a trait whose every method is non-blocking, and the worker is the crate's third thread

```rust
pub trait Launcher: Send {
    fn request(&mut self, request: Request);
    fn drain(&mut self) -> Option<Outcome>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub named: Option<(String, String)>,
    pub problems: Vec<String>,
}

pub fn none() -> Box<dyn Launcher>;
pub fn start(
    cli: std::sync::Arc<dyn crate::cli::HerdrCli>,
    repo: std::path::PathBuf,
    kind: String,
    state_dir: Option<std::path::PathBuf>,
) -> Box<dyn Launcher>;
pub fn settle(launcher: &mut dyn Launcher, budget: std::time::Duration) -> Option<Outcome>;
pub const SETTLE_BUDGET: std::time::Duration;
```

Both trait methods SHALL be non-blocking, on exactly `watch::FsEvents`', `refresh::Refresher`'s,
and `agents::AgentPoll`'s terms: the render path calls them on every iteration and neither may
wait on anything. `request` SHALL send on a channel and return; `drain` SHALL `try_recv` and
return.

The trait SHALL carry **no** `pending_in`. The launcher has no schedule of its own — it acts only
when a key is pressed — so it contributes nothing to the loop's wake-up and
`watch::soonest(fs, agents)` is unchanged. It therefore reads **no** clock, anywhere.

`launch::start` SHALL spawn the crate's third worker thread, on `refresh::start`'s and
`agents::start`'s shape: one request channel in, one result channel out, the worker body written
**below** the single `thread::spawn` in the file so `NOBLOCK`'s leg 3 can cut the production
slice there. The worker SHALL reach the `herdr` program only through `Arc<dyn HerdrCli>`;
`src/launch.rs` SHALL name no process-spawn API, and `src/cli.rs` SHALL remain the crate's
single spawn site.

**A dead worker SHALL be observable**, on exactly `agents::RealAgentPoll`'s model and for
exactly `refresh::Refresher`'s reason. The real implementation SHALL NOT discard a `SendError`
from `request` nor collapse `TryRecvError::Disconnected` into `None` in `drain`. Concretely,
the landed implementation makes a dead launcher indistinguishable from a working one for the
rest of the session: the user presses `a`, `decide` returns `Go`, `Dashboard` clears
`launch.problems` and sets `launch.pending`, the loop takes it and the send silently fails, and
the frame renders exactly as though a launch were under way — for every subsequent `a`, `c`,
`s`, and `g`. On the first `SendError` or first `Disconnected` the implementation SHALL latch a
`dead` flag; the next `drain` SHALL return `Some(Outcome { named: None, problems: vec![reason] })`
exactly once, naming the launcher's worker as stopped; and every `drain` after that SHALL return
`None` while every `request` is discarded.

**A launch in flight at exit SHALL be given a bounded chance to finish.** `launch::settle`
SHALL poll `drain` until an `Outcome` arrives or `budget` elapses, and SHALL return that
`Outcome` or `None`. `SETTLE_BUDGET` SHALL be a named constant, pinned by an assertion, and
SHALL be **35 seconds** — above the measured thirty `herdr agent start` spends waiting for
interactive readiness, and below `cli::RUN_DEADLINE`. `settle` SHALL be a free function
declared **below** `src/launch.rs`'s single `thread::spawn`, so `NOBLOCK`'s leg-3 cut already
excludes it and no gate is weakened to admit it: the trait's two methods stay non-blocking and
the render path still calls only those.

`ui::run_wired` SHALL call `launch::settle` after `run_loop` returns and before it returns,
whatever the loop's outcome, and SHALL fold a returned `Outcome`'s `named` pair into the
recorded mapping exactly as step 6 of the loop does. `src/ui/mod.rs` names only the constant
and the function, never a clock or a join, so `NOBLOCK` leg 2 is unaffected.

The reason is a real, reachable data loss: `run_request` performs `pane split`, then
`agent start`, then `state::record`, then `agent prompt`. `main` calls `exit(0)` as soon as
`ui::run` returns, the three workers are detached and never joined, and dropping
`Collaborators` only disconnects channels. Pressing `a` and then `q` inside the thirty-second
window exits the process with the worker between calls: the pane is already split and the
orphaned `herdr agent start` child keeps running, so an agent **appears** — but
`state::record` never runs and `agent prompt` never runs. The user is left with a live agent
that received no `/opsx:apply` prompt and whose derived name no later dashboard can attribute
either, because the mapping was never written.

`settle` SHALL NOT be called when nothing is in flight, so the ordinary `q` costs nothing:
`ui::run_wired` SHALL consult `Dashboard::launch.in_flight` and skip the call when it is
`false`.

`launch::none()` SHALL be the inert implementation: `request` discards, `drain` is always
`None`, no thread and no process, and it never reports a dead worker — it has none. It is what
`start_collaborators` uses when no repository was found. `settle` over it SHALL return `None`
immediately rather than waiting out the budget.

`Outcome::named` SHALL carry `(derived agent name, change name)` exactly when an agent was
started, so the loop can keep `Dashboard::agent_names` current without re-reading the file, and
`None` for a focus and for a launch that failed before `agent start`. `Outcome::problems` SHALL
carry every failure the worker accumulated, in the order it occurred, and be empty on complete
success. `Outcome` SHALL NOT implement `Default`, derived or hand-written, anywhere in the
crate, and every construction and destructuring SHALL name both fields with no `..` rest, on
exactly `AgentSnapshot`'s and `Attribution`'s terms.

#### Scenario: The inert launcher answers nothing and starts nothing

- **WHEN** `launch::none()` is given a `Request::Launch` and then drained ten times
- **THEN** every `drain` returns `None`
- **AND** no thread is started and no process is spawned, which is what makes a dashboard with
  no repository cost nothing
- **AND** `launch::settle` over it returns `None` promptly, asserted against a deadline-polled
  clock rather than after a fixed sleep, so a no-repository pane does not pay the budget on
  quit

#### Scenario: The real launcher answers on a later drain, never on the requesting one

- **WHEN** `launch::start` is given a recording `HerdrCli` fake that answers all three calls, a
  request is made, and `drain` is polled until it answers or a bounded deadline passes
- **THEN** the `drain` immediately after `request` returns `None` — the work is on the worker
  thread, not the caller's
- **AND** a later `drain` returns `Some(Outcome)` with empty `problems`
- **AND** every `drain` call returns within a time that is not a function of the fake's own
  latency, so the render path never waits on the worker

#### Scenario: Dropping the launcher stops its worker

- **WHEN** a `Launcher` from `launch::start` is dropped
- **THEN** the worker's request channel disconnects and the worker returns, on exactly
  `refresh::worker_body`'s and `agents::worker_body`'s lifecycle
- **AND** a `drain` on a launcher whose worker has already stopped answers `None` rather than
  panicking or blocking

#### Scenario: A dead launcher worker is reported once and then stops being reported

- **WHEN** a real `Launcher` whose worker has exited — the test drops the worker's result
  `Sender` and its request `Receiver` — is given a `Request::Launch` and then drained three
  times, with a further `request` between each
- **THEN** the first `drain` returns `Some(Outcome { named: None, problems: [reason] })` whose
  text names the launcher's worker as stopped
- **AND** the second and third return `None`, so a dead launcher degrades to silence rather
  than to a growing list
- **AND** the reported problem reaches `Dashboard::launch.problems`, so the very first press
  after the worker died renders a row instead of rendering as a launch under way

#### Scenario: A launch in flight at quit is settled rather than orphaned

- **WHEN** `run_wired` is driven over a scripted event source that presses `a` and then `q`,
  with a `HerdrCli` fake whose `agent start` answers only after the loop has already returned
- **THEN** `run_wired` does not return until `launch::settle` has taken the `Outcome` or the
  budget has elapsed
- **AND** the fake records all three calls — `pane split`, `agent start`, `agent prompt` — in
  order, so the prompt that makes the agent useful is not lost to the exit
- **AND** the recorded agent-name mapping contains the derived name, so a later dashboard can
  attribute the agent that this one started

#### Scenario: Quitting with nothing in flight pays no budget

- **WHEN** `run_wired` is driven over a scripted event source that presses only `q`
- **THEN** it returns promptly, asserted against a deadline-polled clock, and `launch::settle`
  was never called
- **AND** the returned `Dashboard` is byte-identical in its launch state to the same run
  before this change, so the ordinary quit is unaffected

#### Scenario: The settle budget is a named constant and is asserted

- **WHEN** `launch::SETTLE_BUDGET` is read
- **THEN** it equals `Duration::from_secs(35)`
- **AND** it is strictly greater than the thirty seconds `herdr agent start` is measured to
  spend waiting for interactive readiness and strictly less than `cli::RUN_DEADLINE`, both
  asserted, so the budget cannot silently drift below the wait it exists to cover

## ADDED Requirements

### Requirement: The dashboard tracks whether a launch is in flight

`Dashboard::launch` SHALL carry `in_flight: bool`, and it SHALL be the single source of the
sixth argument `launch::decide` now takes. Its lifecycle SHALL be exactly:

- set to `true` at the moment the loop hands `launch.pending`'s `Request::Launch` to
  `Launcher::request` — never when `Dashboard::apply` sets `pending`, so the flag describes
  work actually handed over rather than work merely intended;
- left `false` for a `Request::Focus`, which starts no agent and splits no pane and therefore
  cannot leak one;
- cleared to `false` when `Launcher::drain` yields an `Outcome`, in the same step that folds
  the outcome's `named` pair and problems into the dashboard;
- cleared to `false` when the launcher reports its worker dead, since no answer will ever
  come.

The flag SHALL NOT be derived from `launch.pending`, which is a one-shot slot emptied on the
very next iteration, nor from `agents.agents`, which lags the launch by up to a poll interval
plus the launch's own duration — the two sources whose gap is exactly the window this flag
closes.

`Dashboard::apply_launch_action` SHALL pass `self.launch.in_flight` to `decide` and SHALL
otherwise be unchanged: the decision stays in `launch::decide` and no policy is duplicated in
`src/ui/app.rs`.

#### Scenario: The flag is set on hand-over and cleared on outcome

- **WHEN** a `Dashboard` is driven through one loop iteration with a `Request::Launch` in
  `launch.pending` over a scripted launcher that answers on a later drain
- **THEN** `launch.in_flight` is `true` after the iteration that handed the request over
- **AND** it is still `true` on every iteration until the launcher answers
- **AND** it is `false` on the iteration whose `drain` yielded the `Outcome`, in the same
  iteration that recorded the outcome's `named` pair

#### Scenario: A focus request never sets the flag

- **WHEN** the same drive is performed with a `Request::Focus` in `launch.pending`
- **THEN** `launch.in_flight` is `false` after the hand-over and stays `false`
- **AND** a subsequent `a` on the same dashboard is not refused, so focusing does not lock
  the launch keys

#### Scenario: A second launch key while in flight renders a problem row and issues nothing

- **WHEN** a `Dashboard` with `launch.in_flight` `true`, a selected change, `reachable`
  `true`, and empty `live_names` is given `Action::LaunchApply`
- **THEN** `launch.problems` holds exactly one entry saying a launch is already running
- **AND** `launch.pending` is `None`, so the next iteration hands nothing to the launcher and
  no second `pane split` can run
- **AND** the rendered list's interior row 0 is that `!`-marked problem row at both 120 and
  60 columns, so the user's second press is answered visibly rather than ignored

#### Scenario: A dead launcher clears the flag

- **WHEN** a launch is in flight and the launcher's next `drain` reports its worker stopped
- **THEN** `launch.in_flight` is `false` afterwards
- **AND** the launch keys work again, so a dead worker does not additionally lock the keys
  for the rest of the session on top of the row it already renders
