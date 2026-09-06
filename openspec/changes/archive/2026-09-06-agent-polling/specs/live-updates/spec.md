## MODIFIED Requirements

### Requirement: The loop drives the live tier without ever waiting on it

`ui::driver::run_loop(terminal, dashboard, events, live, read, tick)` SHALL take a
`ui::driver::Live` carrying `&mut dyn watch::FsEvents`, `&mut dyn refresh::Refresher`, and —
`agent-polling`'s addition — `&mut dyn agents::AgentPoll`, and SHALL perform, in this order,
once per iteration, **before** the draw:

1. when `dashboard.refresh.requested` is set, `live.refresher.request(Selection::All)` and
   clear the flag;
2. `live.fs.drain()`; on `Ok(Some(paths))`, `refresh.request(watch::invalidate(repo, &paths))`;
   on `Err(e)`, the reason replaces `dashboard.refresh.problems` wholesale — never grown, since
   a watcher failing on every poll must not accumulate an unbounded list;
3. `live.refresher.take_result()`; on `Some(Files(set))` or `Some(Merged(set))`,
   `dashboard.adopt(set)`;
4. `live.agents.drain()`; on `Some(snapshot)`, `dashboard.agents = snapshot` — replaced
   wholesale, never merged, because a poll that found no agents means there are no agents;

then `sync_detail`, then the draw, then `normalise_scroll`, then a wait of
`watch::poll_timeout(tick, watch::soonest(live.fs.pending_in(), live.agents.pending_in()))`
for a terminal event.

Step 1 SHALL run **before** step 2, not after: `ui::load`'s startup flag and a live watcher's
first-ever batch can both be pending on the very same, first iteration, and the recorded
request order that scenario proves — `[Selection::All, Selection::Only(...)]` — only holds
when the flag is turned into a request before the drain is consulted. An implementation that
checks the flag last would instead record `[Selection::Only(...), Selection::All]` on that same
iteration, failing "A filesystem batch becomes one selection" below. Applying a result at step
3, **before** the draw, is what makes a corrected change set visible in the very frame that
consumed it rather than the one after, and step 4 sits before the draw for the same reason.
Every one of the four steps is non-blocking by the traits' contract, so the sequence adds no
wait to the render path.

Step 4 SHALL be **independent of steps 1 to 3**, and that independence SHALL be asserted rather
than assumed: `Dashboard::adopt` replaces `changes` and preserves the selection by name, and it
SHALL neither read nor write `dashboard.agents`, so a refresh landing on the same iteration as a
poll cannot discard the poll. This is the reason the snapshot lives on its own field and not on
`ChangeSet::problems`, which `adopt` replaces wholesale on every refresh.

The poller SHALL contribute a **second** pending deadline to the one wait. `watch::soonest`
takes the minimum over the two `Option<Duration>` values, and the minimum is taken there rather
than inline in `src/ui/driver.rs` so it is asserted over all four combinations of present and
absent rather than only through frame counts.

`run_loop` SHALL NOT name a channel, a thread, a lock, a blocking receive, or a clock:
`src/ui/driver.rs`'s production code names none of `.recv(`, `recv_timeout`, `try_recv`,
`.join()`, `JoinHandle`, `thread::spawn`, `thread::sleep`, `mpsc`, `Mutex`, `RwLock`, or
`Condvar`, and no file under `src/ui/` — tests included — names `Instant::now`,
`SystemTime::now`, or `.elapsed()`. That holds unchanged with a third collaborator: the poller
keeps its own schedule inside `src/agents.rs`, which is outside the swept directory, and reports
its remaining time as a `Duration` exactly as `FsEvents::pending_in` does.

That is necessary and **not sufficient**, and the gap is named here rather than discovered
later: a sweep scoped to `src/ui/` says nothing about whether the three functions the loop calls
on every frame — `FsEvents::drain`, `Refresher::take_result`, and `AgentPoll::drain` — block
inside their own modules. A `drain` written as `self.rx.recv_timeout(Duration::from_millis(150))`,
a `take_result` written as `self.rx.recv_timeout(Duration::from_millis(400)).ok()`, and an agent
`drain` that ran the subprocess itself rather than handing it to a worker would satisfy every
clause above, every trait signature, and every frame-and-poll count this change asserts, while
delaying each draw by more than half a second. The traits' "every method is non-blocking" is a
doc comment, and a doc comment is not a check.

The claim SHALL therefore additionally be checked **inside the three seam modules**:
`src/watch.rs`'s production slice names no blocking receive at all and does name `try_recv`;
`src/refresh.rs`'s and `src/agents.rs`' production slices name no blocking receive **before**
their single `thread::spawn` — everything after that point is the worker body, which may block
freely — and both do name `try_recv`. Together with the two `src/ui/` legs, those are the
mechanical form of "the watcher must not block the draw", "the poller must not block the draw",
and "no view test may be timing-based".

A **runtime** test for the same claim is deliberately not written: any test that fails when a
frame takes 300 milliseconds is an elapsed-time assertion, which is hazard 1 reintroduced in
the one place this change exists to eliminate it. The gate is structural on purpose.

Neither `src/watch.rs`, `src/refresh.rs`, nor `src/ui/driver.rs` SHALL name `HerdrCli`. The
watcher and the refresh worker reach no Herdr socket, so an unreachable socket — a supported
state `SPEC.md` → Degraded states already records — cannot degrade, delay, or fail a refresh;
and `src/ui/driver.rs` reaches the poller only through `AgentPoll`, a trait carrying no CLI
type, which is what keeps `NOCLI-SHELL` green over the whole of `src/ui/`. The socket is reached
from exactly one module, `src/agents.rs`, and through exactly one trait object. `live-refresh`
stated this clause as "the live tier reaches no Herdr socket"; `agent-polling` narrows it to the
three files it always meant, because the live tier now has a fourth member that does.

`ui::load` SHALL produce a `Dashboard` whose `refresh` is `{ requested: true, reload: false,
problems: [] }` and whose `agents` is `{ agents: [], reachable: false, problem: None }`, so the
CLI correction is asked for on the first iteration with no keypress, through the same step 1 the
`r` key uses, and the first agent poll goes out on the same iteration through step 4.
`LoopSummary` SHALL be unchanged: the live tier is observed through the doubles' own recorders,
not through a fourth or fifth counter.

#### Scenario: The startup request is issued before the first wait

- **WHEN** `run_loop` is driven at 120x20 with a recording `Refresher` double whose
  `take_result` is always `None`, an inert `AgentPoll`, a `Dashboard` from `ui::load` with
  `refresh.requested` set, and a script of one `Char('q')` Press
- **THEN** it returns `Ok(LoopSummary { frames: 1, polls: 1 })`
- **AND** the double recorded exactly one `request`, carrying `Selection::All`
- **AND** `dashboard.refresh.requested` is false afterwards, so a single flag produces a
  single request rather than one per frame
- **AND** the same run at 60x20 behaves identically

#### Scenario: A result is adopted before the frame that shows it

- **WHEN** `run_loop` is driven at 120x20 and 60x20 over a dashboard whose file-sourced
  `alpha` reads `[4/9]`, with a `Refresher` double whose **first** `take_result` returns
  `Merged(set)` in which `alpha` reads `[7/9]`, an inert `AgentPoll`, and a script of one
  `Char('q')` Press
- **THEN** it returns `Ok(LoopSummary { frames: 1, polls: 1 })` — one frame, not two
- **AND** the buffer's list row for `alpha` ends in `[7/9]` at both widths, so the result
  reached the very frame that consumed it
- **AND** the double recorded exactly **one** `take_result` call for that one iteration, so
  the loop neither drains the channel in a spin nor waits for a second result

#### Scenario: A filesystem batch becomes one selection

- **WHEN** `run_loop` is driven over a dashboard whose repo is `/r`, with an `FsEvents` double
  whose first `drain` returns `Ok(Some([/r/openspec/changes/alpha/tasks.md]))` and whose later
  drains return `Ok(None)`, an inert `AgentPoll`, and a script of two `Ok(None)` timeouts then
  `Char('q')`
- **THEN** the `Refresher` double recorded exactly two requests: the startup
  `Selection::All`, then `Selection::Only({"alpha"})`
- **AND** no request was made on the iterations whose `drain` returned `Ok(None)`
- **AND** the assertion is an exact equality on the recorded request vector, so an
  implementation that requested `All` for every batch fails rather than passing

#### Scenario: A watcher error is recorded once and the loop continues

- **WHEN** an `FsEvents` double's `drain` returns `Err(WatchError)` on its first two calls and
  `Ok(None)` thereafter, with an inert `AgentPoll` and a script of three timeouts then
  `Char('q')`
- **THEN** `run_loop` returns `Ok(LoopSummary { frames: 4, polls: 4 })` — a watch error is
  never a `LoopError` and never ends the loop
- **AND** `dashboard.refresh.problems` holds exactly one entry naming the reason, not two: a
  watcher failing on every poll must not grow an unbounded problem list
- **AND** that problem is rendered as the list region's first `!`-marked row at both widths

#### Scenario: The wait shortens to the debounce deadline

- **WHEN** `run_loop` is driven with `tick` of 250ms, an `FsEvents` double whose `pending_in`
  returns `Some(90ms)` on its first call and `None` thereafter, and an inert `AgentPoll` whose
  `pending_in` is always `None`
- **THEN** the scripted event source recorded its first `next_event` timeout as `90ms` and
  every later one as `250ms`
- **AND** with `pending_in` returning `Some(0ms)` the recorded timeout is `1ms`, never `0ms`

#### Scenario: The wait shortens to whichever poller is due first

- **WHEN** `run_loop` is driven with `tick` of 250ms over an `FsEvents` double whose
  `pending_in` returns `Some(900ms)`, `Some(90ms)`, `None` on successive calls and an
  `AgentPoll` double whose `pending_in` returns `Some(40ms)`, `Some(900ms)`, `Some(3s)` on
  successive calls, with a script of two timeouts then `Char('q')`
- **THEN** the event source recorded its three timeouts as `40ms`, `90ms`, and `250ms`
- **AND** the third value proves a deadline further away than the tick never lengthens the wait
- **AND** an implementation that consulted only the watcher records `900ms`, `90ms`, `250ms`
  and fails on the first value, which is what makes this scenario discriminating rather than
  decorative

#### Scenario: An agent snapshot reaches the frame that consumed it and survives a refresh

- **WHEN** `run_loop` is driven at 120x20 and 60x20 with an `AgentPoll` double whose **first**
  `drain` returns `Some(AgentSnapshot { agents: [one agent], reachable: true, problem: None })`
  and whose later drains return `None`, a `Refresher` double whose first `take_result` returns
  `Merged(set)`, and a script of two timeouts then `Char('q')`
- **THEN** `dashboard.agents.reachable` is true afterwards and `dashboard.agents.agents` holds
  that one agent
- **AND** the double recorded exactly one `drain` call per iteration — three calls over three
  iterations — so the loop neither polls twice per frame nor skips a frame
- **AND** the adopted `ChangeSet` did not clear the snapshot, proving step 4 is independent of
  step 3
- **AND** every cell of the buffer is identical to the same run with an inert `AgentPoll`, so
  the snapshot changed state without changing the frame

#### Scenario: An unreachable socket never becomes a problem row

- **WHEN** `run_loop` is driven with an `AgentPoll` double whose first `drain` returns
  `Some(AgentSnapshot { agents: [], reachable: false, problem: Some("herdr agent list exited 1: server_not_running") })`
- **THEN** `dashboard.refresh.problems` is still empty and no `!`-marked row appears at either
  width
- **AND** `dashboard.agents.problem` carries that text, so the reason is available to a later
  change without being rendered by this one
- **AND** `run_loop` returns `Ok`: an unreachable socket is never a `LoopError`

#### Scenario: An inert live tier leaves the loop exactly as it was

- **WHEN** `run_loop` is driven with `watch::none()`, `refresh::none()`, and `agents::none()`
  and the same scripts every landed `dashboard-loop` scenario uses
- **THEN** every landed frame count, poll count, buffer assertion, and reader call count is
  unchanged
- **AND** this is the case a machine with no `openspec` binary, no working watcher, and no
  Herdr socket runs, so the dashboard degrades to exactly the behaviour that shipped in
  `live-refresh`
