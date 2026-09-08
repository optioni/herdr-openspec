# live-updates Specification

## Purpose
Covers the dashboard side of keeping the pane current: the `Refresh` state, the `r`
key that only sets a flag the loop later turns into a request, `Dashboard::adopt` preserving the
reader's selection by change **name** across a refresh that reorders the list, and the forced
reload that re-reads the artifact on screen without throwing a scrolled reader back to line one.
Its central obligation is that none of this may ever make the draw wait: the loop's live
steps run in a fixed order before the frame, every collaborator method is non-blocking, and that
claim is enforced structurally inside the seam modules rather than by any elapsed-time test.
It also fixes that the whole live tier writes nothing under `openspec/`, and that refresh
problems render as leading `!` rows between launch problems and change-set problems. The
watcher, the worker and the poller themselves are `watch-invalidation`'s, `refresh-worker`'s
and `agent-poller`'s.

## Requirements

### Requirement: `Dashboard::refresh` carries the live tier's state

`ui::app::Refresh` SHALL carry exactly four fields:

- `requested: bool` — set by `Action::Refresh` and by `ui::load` at startup, cleared by
  `run_loop` once it has asked the refresher for a full reload. It is the counterpart of
  `quit`: a pure state value the loop observes, so `Dashboard::apply` stays a pure function
  that reaches no collaborator.
- `reload: bool` — set by `adopt`, consumed by `sync_detail`. It forces a re-read of an
  unchanged `(change directory, tab)` key, which is what makes an edit to the artifact
  currently on screen visible: the cache key `artifact-content` compares does not change when
  a file's **content** does.
- `startup: Vec<String>` — the standing conditions the pane learns **once**, at startup, in
  causal order: the configuration's key-by-key fallbacks (`plugin-config`), then the
  `openspec` binary probe's (`openspec-binary`), then the watcher's failure to start. Written
  exactly once, by `run_wired` from `Collaborators::problems`, and **never** written again by
  anything: not by `r`, not by a refresh, not by a watcher error. They are true for the whole
  session and no reload re-derives them.
- `problems: Vec<String>` — problems the **live** tier reports as the session runs: a
  `FsEvents::drain` error, and a refresh worker reported stopped. Replaced wholesale by each
  new one, never grown, since a collaborator failing on every poll must not accumulate an
  unbounded list.

Splitting these two apart is this change's correction of a real, reachable defect.
`degraded-states` seeded all three startup sources into `problems`, and step 3 of the loop
below replaces `problems` **wholesale** on the first `FsEvents::drain` error — so the very
first watcher error erases the rows naming a missing `openspec` binary and every
configuration fallback, for the rest of the session, with nothing repopulating them. Driven
live, two real problem rows became one. The erasing case is not hypothetical: it is exactly
`SPEC.md`'s documented "`openspec/` is removed while the watcher runs" row, and it silently
falsifies two other rows of the same table that promise those reasons are named. The
wholesale replacement is deliberate and is kept; what changes is that it can no longer reach
a value it did not produce.

Both fields are what `ChangeSet::problems` is not: everything the CLI or the file walk
reports rides there instead and is replaced wholesale on every adopt, which is why none of
these may live there.

`Refresh` SHALL carry exactly these four fields and no more. `degraded-states`' `file_mode`
flag is a **`Dashboard`** field rather than a fifth one here, and that is a deliberate
placement rather than an arbitrary one: `requested`, `reload`, and `problems` are all consumed
or replaced as the session runs, while `file_mode` is decided once, at startup, and never
moves. `startup` is written once and never replaced, like `file_mode`, but it lives here
because it is a **list of reasons rendered beside `problems`** and separating the two lists
across two types would put the render order's two sources in different places. `dashboard-loop` carries its definition and the field count that follows from it.

`Detail` SHALL still carry exactly **five**: `reload` deliberately lives on `Refresh` rather
than on `Detail`, because `Dashboard` gains one field either way and putting it on `Refresh`
keeps `Detail`'s five unchanged.

The count of `Dashboard`'s own fields is **not** restated here. It was written as nine when
`live-refresh` landed and three later changes have added to it since without correcting the
sentence; `dashboard-loop`'s own requirement is the single place that number lives, and this
requirement now defers to it rather than carrying a second copy that drifts.

`Refresh` SHALL NOT implement `Default` — neither derived nor hand-written, anywhere in the
crate — and every construction and destructuring of it SHALL name all four fields with no
`..` rest, on exactly the terms `Dashboard`, `Filter`, and `Detail` are already bound.

#### Scenario: `Refresh` has no `Default` and no site elides a field

- **WHEN** every `*.rs` file under `src/` is searched for `impl Default for Refresh`, for a
  `Default` inside the `#[derive(...)]` immediately preceding `struct Refresh`, and for a `..`
  inside a `Refresh { … }` literal or pattern, brace-matched from the opening `{` to its
  partner so a multi-line elision rustfmt spread over several lines is seen
- **THEN** there is no match
- **AND** the same check runs over `Dashboard`, `Filter`, and `Detail` in the same invocation,
  and is judged against that type set's **own** counted minimum of literal or pattern spans —
  not a floor shared with the `Dashboard` set, which scans far more (`dashboard-loop`). The
  `Refresh` set's floor is measured at this change's base commit and written as that run's
  default when the gate becomes a repository file; the landed **90**, measured at **107**, is
  superseded by that measurement
- **AND** a compile-time companion destructures a `Dashboard` naming every one of its fields
  with no `..`, and a `Refresh` naming all **four**, so a field added to either breaks the
  build at that site rather than passing a source grep that never saw it. The `Dashboard` arm
  names **thirteen** fields after `degraded-states` adds `file_mode`

#### Scenario: A watcher error does not erase the startup problems

- **WHEN** `run_loop` is driven over a `Dashboard` whose `refresh.startup` holds
  `openspec binary not found on PATH` and `config.toml: archived_count not set, using 5`, with
  an `FsEvents` double whose first `drain` returns `Err(WatchError)` and whose later drains
  return `Ok(None)`, over a script of two timeouts then `Char('q')`
- **THEN** `dashboard.refresh.startup` is unchanged afterwards, holding both entries in the
  same order
- **AND** `dashboard.refresh.problems` holds exactly one entry, the watcher's reason
- **AND** the rendered list's interior rows 0, 1, and 2 are the two startup rows followed by
  the watcher row, at both 120 and 60 columns — three `!`-marked rows where the behaviour
  before this change rendered one

#### Scenario: A forced refresh does not repopulate or duplicate the startup problems

- **WHEN** the same dashboard is driven through a `Char('r')` press and a `Merged` result, and
  then through a second watcher error
- **THEN** `refresh.startup` still holds exactly its two original entries, neither duplicated
  nor re-derived
- **AND** `refresh.problems` still holds exactly one entry, so the live list is still replaced
  wholesale rather than grown

### Requirement: `r` forces a full refresh and types itself while filtering

`ui::app::Action` SHALL carry exactly **seventeen** variants — the twelve `dashboard-loop`
names, `Refresh`, and `agent-launch`'s `LaunchApply`, `LaunchContinue`, `LaunchArchive`, and
`FocusAgent`. The count moves from thirteen here, in `tasks-checklist`, and in
`dashboard-loop` together, because all three state it and a number stated in three places
drifts in two of them.

While `filtering` is **false**, `KeyCode::Char('r')` with no modifiers SHALL map to
`Action::Refresh`. While `filtering` is **true** it SHALL map to `Action::FilterPush('r')`,
by `list-filtering`'s unchanged rule that every printable key types into the query — no
exception is carved out for it.

`Dashboard::apply(Action::Refresh)` SHALL set `refresh.requested` and change **nothing else**:
not `changes`, not `selected`, not `route`, not `detail`, not `filter`, not `quit`. It reaches
no collaborator, performs no I/O, and starts no work; `run_loop` is what turns the flag into a
request.

`Action::Refresh` SHALL NOT mutate `Dashboard::changes`, and the existing sweep proving that
of every action SHALL be extended from thirteen variants to **seventeen** rather than left at
thirteen, so an eighteenth variant added later still fails that test rather than slipping past
it. `agent-launch`'s four are the ones whose arms are least obviously read-only — each
eventually starts a process — which is why the sweep's fixture SHALL carry a reachable socket
and a non-empty visible list, so all four reach `launch::decide` rather than short-circuiting.

#### Scenario: `r` refreshes outside filter mode and types inside it

- **WHEN** `action_for` is called with a Press of `Char('r')` with no modifiers under
  `filtering` false, then under `filtering` true, then with `Char('r')` carrying `CONTROL`
  under both, then with `Char('R')` carrying `SHIFT` under both
- **THEN** the results are `Refresh`, `FilterPush('r')`, `Ignore`, `Ignore`,
  `Ignore`, and `FilterPush('R')`
- **AND** a `Release` and a `Repeat` of `Char('r')` both return `Ignore` under either mode, so
  a terminal reporting releases cannot refresh twice

#### Scenario: `Refresh` sets the flag and touches nothing else

- **WHEN** a `Dashboard` at `Route::Detail` with `selected` 1, `detail.tab` 2,
  `detail.scroll` 5, a non-empty filter query, and a two-change `ChangeSet` is given
  `Action::Refresh`
- **THEN** `refresh.requested` is true
- **AND** `route`, `selected`, `detail.tab`, `detail.scroll`, `filter`, `quit`, and `changes`
  are each `==` to their prior values
- **AND** applying `Refresh` a second time leaves the flag true and still changes nothing

#### Scenario: No action mutates the change set

- **WHEN** every one of the **seventeen** `Action` variants is applied in turn to a
  `Dashboard` whose selected change carries artifacts, whose `agents.reachable` is `true`, and
  whose visible list is non-empty, with `changes` compared for equality
  after each
- **THEN** `changes` is unchanged after every one
- **AND** the variant count is asserted to be exactly seventeen against an array the test
  builds through an exhaustive `match`, so an eighteenth variant is a compile error rather
  than a silent gap in the sweep
- **AND** after each of `LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent` the
  dashboard's `launch.pending` is `Some` or its `launch.problems` is non-empty, so the four
  reached the decision rather than satisfying the equality vacuously

### Requirement: Adopting a change set preserves the selection by name

`Dashboard::adopt(&mut self, changes: ChangeSet)` SHALL be a pure function of `&mut self` and
its argument — no I/O, no clock — that:

1. records the currently selected change's `name`, when one is selected;
2. replaces `self.changes`;
3. sets `selected` to the position of that name in the **new** `visible()` list, when the name
   is still there;
4. clamps `selected` the way every other list mutation does;
5. sets `refresh.reload`.

Preserving by **name** rather than by index is required, not preferred: a refresh can add a
change alphabetically above the selected one, and an index preserved across that shift silently
moves the reader to a different change while they are reading it. The name is resolved against
`visible()`, not against `changes.active`, because `selected` indexes the visible list and a
`/` filter may be active.

`adopt` SHALL NOT reset `detail.tab` and SHALL NOT reset `detail.scroll`: a refresh is not a
selection move, and a reader scrolled halfway down a design document must stay there when an
agent saves a file.

#### Scenario: The selection follows its change when the list shifts

- **WHEN** a `Dashboard` whose visible list is `[beta, gamma]` with `selected` 1 — `gamma` —
  adopts a change set whose active list is `[alpha, beta, gamma]`
- **THEN** `selected` is 2, still addressing `gamma`
- **AND** `detail.tab` and `detail.scroll` are unchanged
- **AND** `refresh.reload` is true

#### Scenario: The selection is clamped when its change is gone

- **WHEN** a `Dashboard` whose visible list is `[alpha, beta, gamma]` with `selected` 2 adopts
  a change set holding only `alpha`
- **THEN** `selected` is 0, the last visible index, rather than an index past the end
- **AND** adopting an empty change set from there leaves `selected` at 0 and
  `selected_change()` at `None`, and nothing panics

#### Scenario: A filter narrows what the name is resolved against

- **WHEN** a `Dashboard` with the filter query `a` — whose visible list is therefore
  `[alpha, beta]` out of an active list of `[alpha, beta, gamma]` — with `selected` 1
  (`beta`) adopts a set whose active list is `[alpha, apex, beta, gamma]`
- **THEN** `selected` is 2, addressing `beta` in the new visible list `[alpha, apex, beta]`
- **AND** the filter query itself is untouched by the adopt

### Requirement: A forced reload re-reads without losing the scroll

`Dashboard::sync_detail` SHALL take `refresh.reload` at the start of each call, clearing it,
and SHALL:

- re-read the selected tab when the `(change directory, tab)` key changed **or** the flag was
  set, rather than only on a key change;
- reset `detail.scroll` to `0` exactly when the **key** changed, never merely because a reload
  was forced.

That split is what lets an agent's save re-render the document the reader is looking at
without throwing them back to line one, while a tab or change move still starts at the top.
`layout::scroll_offset` and `Dashboard::normalise_scroll` still clamp the preserved offset
against whatever the new content's length turns out to be, so a document that shrank cannot
leave the offset past its end for more than one frame.

#### Scenario: A forced reload re-reads the same tab and keeps the offset

- **WHEN** a `Dashboard` whose `detail.loaded` is `(/repo/openspec/changes/alpha, 0)` and
  whose `detail.scroll` is `6` has `refresh.reload` set and is synced against a recording
  reader returning new, longer text for that path
- **THEN** the reader recorded one call for that path — the unchanged key did not suppress it
- **AND** `detail.source` holds the new text
- **AND** `detail.scroll` is still `6`
- **AND** `refresh.reload` is false afterwards, so one flag drives exactly one re-read

#### Scenario: An unchanged key with no forced reload re-reads nothing

- **WHEN** the same dashboard is synced twice with `refresh.reload` false
- **THEN** the recording reader recorded exactly **one** call across both syncs
- **AND** `detail.scroll` is unchanged, which is the landed behaviour this change must not
  disturb

#### Scenario: A tab move under a forced reload still resets the scroll

- **WHEN** a `Dashboard` scrolled to `6` has both `detail.tab` moved to a different artifact
  and `refresh.reload` set, and is then synced
- **THEN** the new tab's content is read and `detail.scroll` is `0`
- **AND** the reset is attributed to the key change, not to the flag: the previous scenario
  proves a forced reload alone preserves the offset

### Requirement: The loop drives the live tier without ever waiting on it

`ui::driver::run_loop(terminal, dashboard, events, live, read, tick)` SHALL take a
`ui::driver::Live` carrying `&mut dyn watch::FsEvents`, `&mut dyn refresh::Refresher`, —
`agent-polling`'s addition — `&mut dyn agents::AgentPoll`, and — `agent-launch`'s addition —
`&mut dyn launch::Launcher`, and SHALL perform, in this order,
once per iteration, **before** the draw:

1. when `dashboard.launch.pending` is `Some`, take it — leaving `None` — and
   `live.launcher.request(request)`, setting `dashboard.launch.in_flight` when and only when
   the request taken was a `Request::Launch`;
2. when `dashboard.refresh.requested` is set, `live.refresher.request(Selection::All)` and
   clear the flag;
3. `live.fs.drain()`; on `Ok(Some(paths))`, compute `watch::invalidate(repo, &paths)` and
   `refresh.request(selection)` **only when the selection is not `Selection::Only` of the
   empty set** — an empty selection invalidates nothing, and requesting one costs a full
   `changes::from_files` directory re-walk plus an `openspec list --json` Node start, because
   `changes::from_cli_cached` runs `list --json` before it ever consults the selection;
   on `Err(e)`, the reason replaces `dashboard.refresh.problems` wholesale — never grown, since
   a watcher failing on every poll must not accumulate an unbounded list, and never touching
   `dashboard.refresh.startup`, which the loop does not write at all;
4. `live.refresher.take_result()`; on `Some(Files(set))` or `Some(Merged(set))`,
   `dashboard.adopt(set)`; on `Some(Stopped(reason))`, the reason replaces
   `dashboard.refresh.problems` wholesale and **no** `adopt` runs — the change set on screen
   is the last true one and replacing it with an empty set would make a dead worker look like
   an empty repository;
5. `live.agents.drain()`; on `Some(snapshot)`, `dashboard.agents = snapshot` — replaced
   wholesale, never merged, because a poll that found no agents means there are no agents;
6. `live.launcher.drain()`; on `Some(outcome)`, clear `dashboard.launch.in_flight`, insert
   `outcome.named`'s `(agent, change)` pair
   into `dashboard.agent_names.names` when it is `Some`, and replace
   `dashboard.launch.problems` wholesale with `outcome.problems`' entries — never
   grown, on exactly step 3's terms, since a socket failing on every press must not accumulate
   an unbounded list;

then `sync_detail`, then the draw, then `normalise_scroll`, then a wait of
`watch::poll_timeout(tick, watch::soonest(live.fs.pending_in(), live.agents.pending_in()))`
for a terminal event.

Step 1 leads the iteration because it answers a key pressed at the **end** of the previous one:
`Dashboard::apply` sets `launch.pending` after the wait, and the dispatch is the first thing the
next iteration does, so a launch starts within one tick of the press rather than one frame
later. `watch::soonest` still takes **two** arguments and gains no third: the launcher has no
schedule and no `pending_in`, because it acts only when a key is pressed and therefore has
nothing to wake for.

Step 2 SHALL run **before** step 3, not after: `ui::load`'s startup flag and a live watcher's
first-ever batch can both be pending on the very same, first iteration, and the recorded
request order that scenario proves — `[Selection::All, Selection::Only(...)]` — only holds
when the flag is turned into a request before the drain is consulted. An implementation that
checks the flag last would instead record `[Selection::Only(...), Selection::All]` on that same
iteration, failing "A filesystem batch becomes one selection" below. Applying a result at step
4, **before** the draw, is what makes a corrected change set visible in the very frame that
consumed it rather than the one after, and steps 5 and 6 sit before the draw for the same
reason — step 6 in particular, because the mapping it writes is what
`Dashboard::attribution()` reads two steps later to badge the change that was just launched.
Every one of the six steps is non-blocking by the traits' contract, so the sequence adds no
wait to the render path.

Step 5 SHALL be **independent of steps 1 to 4**, and that independence SHALL be asserted rather
than assumed: `Dashboard::adopt` replaces `changes` and preserves the selection by name, and it
SHALL neither read nor write `dashboard.agents`, so a refresh landing on the same iteration as a
poll cannot discard the poll. This is the reason the snapshot lives on its own field and not on
`ChangeSet::problems`, which `adopt` replaces wholesale on every refresh. Step 6 SHALL be
independent of all five for the same reason and by the same argument: `adopt` neither reads nor
writes `agent_names` or `launch`, so a refresh landing on the same iteration as a launch outcome
cannot discard the recorded pair or the reported problem.

The poller SHALL contribute a **second** pending deadline to the one wait. `watch::soonest`
takes the minimum over the two `Option<Duration>` values, and the minimum is taken there rather
than inline in `src/ui/driver.rs` so it is asserted over all four combinations of present and
absent rather than only through frame counts.

`run_loop` SHALL NOT name a channel, a thread, a lock, a blocking receive, or a clock:
`src/ui/driver.rs`'s production code names none of `.recv(`, `recv_timeout`, `try_recv`,
`.join()`, `JoinHandle`, `thread::spawn`, `thread::sleep`, `mpsc`, `Mutex`, `RwLock`, or
`Condvar`, and no file under `src/ui/` — tests included — names `Instant::now`,
`SystemTime::now`, or `.elapsed()`. That holds unchanged with a third and a fourth collaborator:
the poller keeps its own schedule inside `src/agents.rs`, which is outside the swept directory,
and reports its remaining time as a `Duration` exactly as `FsEvents::pending_in` does, and the
launcher inside `src/launch.rs` keeps no schedule at all.

That is necessary and **not sufficient**, and the gap is named here rather than discovered
later: a sweep scoped to `src/ui/` says nothing about whether the four functions the loop calls
on every frame — `FsEvents::drain`, `Refresher::take_result`, `AgentPoll::drain`, and
`Launcher::drain` — block
inside their own modules. A `drain` written as `self.rx.recv_timeout(Duration::from_millis(150))`,
a `take_result` written as `self.rx.recv_timeout(Duration::from_millis(400)).ok()`, an agent
`drain` that ran the subprocess itself rather than handing it to a worker, and a
`Launcher::request` that ran the three Herdr calls on the caller's thread would satisfy every
clause above, every trait signature, and every frame-and-poll count this change asserts, while
delaying each draw by more than half a second — the launcher's version by up to the
**thirty seconds** `herdr agent start` waits for interactive readiness, which is the worst
render-path stall this crate could ship and the whole reason the launcher is a worker thread
rather than a synchronous call. The traits' "every method is non-blocking" is a
doc comment, and a doc comment is not a check.

The claim SHALL therefore additionally be checked **inside the four seam modules**:
`src/watch.rs`'s production slice names no blocking receive at all and does name `try_recv`;
`src/refresh.rs`'s, `src/agents.rs`', and `src/launch.rs`'s production slices name no blocking
receive **before**
their single `thread::spawn` — everything after that point is the worker body, which may block
freely — and all three do name `try_recv`. Together with the two `src/ui/` legs, those are the
mechanical form of "the watcher must not block the draw", "the poller must not block the draw",
"the launcher must not block the draw", and "no view test may be timing-based".

A **runtime** test for the same claim is deliberately not written: any test that fails when a
frame takes 300 milliseconds is an elapsed-time assertion, which is hazard 1 reintroduced in
the one place this change exists to eliminate it. The gate is structural on purpose.

Neither `src/watch.rs`, `src/refresh.rs`, nor `src/ui/driver.rs` SHALL name `HerdrCli`. The
watcher and the refresh worker reach no Herdr socket, so an unreachable socket — a supported
state `SPEC.md` → Degraded states already records — cannot degrade, delay, or fail a refresh;
and `src/ui/driver.rs` reaches the poller only through `AgentPoll` and the launcher only through
`Launcher`, neither of which carries a CLI
type, which is what keeps `NOCLI-SHELL` green over the whole of `src/ui/`. The socket is reached
from exactly **two** modules, `src/agents.rs` and `src/launch.rs`, and through one trait object
each. `live-refresh`
stated this clause as "the live tier reaches no Herdr socket"; `agent-polling` narrowed it to the
three files it always meant, because the live tier then had a fourth member that did, and
`agent-launch` leaves the same three named while adding a second member that reaches the
socket.

`run_wired` SHALL seed `dashboard.refresh.startup` — **not** `dashboard.refresh.problems` —
from `Collaborators::problems`, in the causal order `degraded-states`' Decision 4 mandates,
and SHALL do so exactly once. `run_wired` SHALL additionally call
`launch::settle(&mut *collaborators.launcher, launch::SETTLE_BUDGET)` after `run_loop`
returns, when and only when `dashboard.launch.in_flight` is set, folding any returned
`Outcome`'s `named` pair into `dashboard.agent_names.names` exactly as step 6 does
(`agent-launch`). It SHALL name the constant and the function and nothing else: no clock, no
join, no channel, so `NOBLOCK` leg 2's sweep over `src/ui/` stays green unweakened.

`ui::load` SHALL produce a `Dashboard` whose `refresh` is `{ requested: true, reload: false,
startup: [], problems: [] }`, whose `agents` is `{ agents: [], reachable: false, stalled:
false, problem: None }`, and whose
`launch` is `{ pending: None, problems: [], in_flight: false }`, so the
CLI correction is asked for on the first iteration with no keypress, through the same step 2 the
`r` key uses, the first agent poll goes out on the same iteration through step 5, and nothing is
launched until a key is pressed.
`LoopSummary` SHALL be unchanged: the live tier is observed through the doubles' own recorders,
not through a fourth, fifth, or sixth counter.

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

#### Scenario: A batch that invalidates nothing issues no refresh

- **WHEN** `run_loop` is driven over a dashboard whose repo is `/r`, with an `FsEvents` double
  whose first `drain` returns `Ok(Some([/r/target/debug/build.log, /r/.git/index]))` — a batch
  every path of which classifies `Outside`, so `watch::invalidate` yields `Selection::Only`
  of the empty set — and whose later drains return `Ok(None)`, over a script of two timeouts
  then `Char('q')`
- **THEN** the `Refresher` double recorded exactly **one** request, the startup
  `Selection::All`, and nothing for the batch
- **AND** the assertion is an exact equality on the recorded request vector, so an
  implementation that requested the empty selection records two and fails
- **AND** `dashboard.refresh.problems` is empty: a batch that invalidates nothing is not a
  problem, it is a non-event

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
- **AND** the adopted `ChangeSet` did not clear the snapshot, proving step 5 is independent of
  step 4
- **AND** every cell of the buffer is identical to the same run with an inert `AgentPoll`, so
  the snapshot changed state without changing the frame

#### Scenario: An unreachable socket never becomes a problem row

- **WHEN** `run_loop` is driven with an `AgentPoll` double whose first `drain` returns
  `Some(AgentSnapshot { agents: [], reachable: false, stalled: false, problem: Some("herdr agent list exited 1: server_not_running") })`
- **THEN** `dashboard.refresh.problems` is still empty and no `!`-marked row appears at either
  width
- **AND** `dashboard.agents.problem` carries that text, so the reason is available to a later
  change without being rendered by this one
- **AND** `run_loop` returns `Ok`: an unreachable socket is never a `LoopError`

#### Scenario: A stalled poller becomes a problem row and withdraws the badges

- **WHEN** `run_loop` is driven at 120x20 and 60x20 over a dashboard whose visible
  `2fa-support` row carries a `working` badge from an earlier snapshot, with an `AgentPoll`
  double whose next `drain` returns
  `Some(AgentSnapshot { agents: [], reachable: false, stalled: true, problem: Some("herdr agent list has not answered in 5s") })`,
  over a script of one timeout then `Char('q')`
- **THEN** an `!`-marked row carrying that text is rendered in the list region at both widths
- **AND** the `2fa-support` row no longer carries a badge, because the snapshot replaced the
  agents wholesale
- **AND** `run_loop` returns `Ok`, and `dashboard.refresh.problems` and
  `dashboard.refresh.startup` are both untouched: the stall row is sourced from
  `dashboard.agents`, not copied into either problem list

#### Scenario: A stopped refresh worker becomes a problem row and keeps the list

- **WHEN** `run_loop` is driven at 120x20 and 60x20 over a dashboard holding two active
  changes, with a `Refresher` double whose first `take_result` returns
  `Some(RefreshResult::Stopped("refresh worker stopped"))` and whose later results are `None`,
  over a script of two timeouts then `Char('q')`
- **THEN** `dashboard.refresh.problems` holds exactly that one entry and it is rendered as an
  `!`-marked row at both widths
- **AND** `dashboard.changes` still holds both changes: no `adopt` ran, so a dead worker does
  not empty the list
- **AND** `run_loop` returns `Ok`, and a second `Stopped` on a later iteration would replace
  rather than grow the entry

#### Scenario: An inert live tier leaves the loop exactly as it was

- **WHEN** `run_loop` is driven with `watch::none()`, `refresh::none()`, `agents::none()`, and
  `launch::none()`
  and the same scripts every landed `dashboard-loop` scenario uses
- **THEN** every landed frame count, poll count, buffer assertion, and reader call count is
  unchanged
- **AND** this is the case a machine with no `openspec` binary, no working watcher, and no
  Herdr socket runs, so the dashboard degrades to exactly the behaviour that shipped in
  `live-refresh`
- **AND** `launch::none()` in particular discards every request and answers `None` from every
  `drain`, so a key pressed against it leaves `launch.problems` empty rather than reporting a
  failure the reader cannot act on

#### Scenario: A launch request reaches the launcher and its outcome reaches the dashboard

- **WHEN** `run_loop` is driven at 120x20 and at 60x20 over a recording `Launcher` double, a
  `Dashboard` whose `agents.reachable` is true and whose visible list holds `2fa-support`, and
  a script of a Press of `Char('a')`, then two `Ok(None)` timeouts, then `Char('q')`; the double
  answers its **second** `drain` with
  `Outcome { named: Some(("c-2fa-support", "2fa-support")), problems: [] }`
- **THEN** the double recorded exactly **one** request,
  `Request::Launch { change: "2fa-support", agent: "c-2fa-support", intent: Apply }`
- **AND** `dashboard.agent_names.names` afterwards holds `c-2fa-support -> 2fa-support`, and
  `dashboard.launch.problems` is empty
- **AND** `dashboard.launch.in_flight` was `true` on the iterations between the hand-over and
  that second `drain`, and is `false` afterwards
- **AND** the double recorded exactly one `drain` call per iteration, so the loop neither drains
  twice per frame nor skips a frame
- **AND** the exact equality on the recorded request vector is what makes this discriminating:
  an implementation that dispatched on every iteration while `pending` stayed `Some` records
  three requests and fails

#### Scenario: A launch failure is recorded once and the loop continues

- **WHEN** a `Launcher` double answers its first three `drain` calls with
  `Outcome { named: None, problems: ["herdr pane split exited with code 1: no space"] }` and
  `None` thereafter, over a script of four timeouts then `Char('q')`
- **THEN** `run_loop` returns `Ok(LoopSummary { frames: 5, polls: 5 })` — a failed launch is
  never a `LoopError` and never ends the loop
- **AND** `dashboard.launch.problems` holds exactly **one** entry naming the reason, not three:
  the field is replaced wholesale, on exactly `refresh.problems`' terms
- **AND** that problem is rendered as the list region's first `!`-marked row at both widths,
  above any refresh problem

### Requirement: The live tier writes nothing inside the repository

Watching a directory, classifying a path, requesting a refresh, running the CLI producer,
adopting a result, splitting a pane, starting an agent, and sending it a prompt SHALL all leave
the repository untouched. No code path in `src/watch.rs`, `src/refresh.rs`, `src/agents.rs`,
`src/launch.rs`, or the
loop's live tier SHALL create, modify, remove, or rename any file under `openspec/`.

`agent-launch` adds the live tier's **first write of any kind**, and confines it: the launcher
calls `state::record`, which `plugin-state` confines to the directory
`HERDR_PLUGIN_STATE_DIR` resolves, and calls nothing else that writes. The distinction this
requirement now has to carry is between the plugin's writes and the **launched agent's** writes.
The plugin starts a coding agent in another pane whose purpose is to edit files under
`openspec/`; those edits are that process's, made under its own identity through its own tools,
and the plugin's causal contribution ends at `herdr agent prompt`. Every scenario below measures
the plugin's own writes, against a scratch `herdr` program that starts nothing — which is the
only honest way to measure them.

This is the same guarantee `tasks-checklist` proved for the tracked-tasks tab, made again
because this change is the first to hold an open handle on the tree while an agent writes to
it, and because `openspec validate <change> --strict` — read-only — is the only `openspec`
subcommand the plugin may ever run beyond `list` and `instructions apply`.

The proof SHALL be a snapshot comparison over a real scratch repository taken around a full
loop run, together with a discriminating control showing the same comparison **fails** when a
single byte of that tree is rewritten, since an equality assertion that cannot fail proves
nothing.

#### Scenario: A full live run leaves the change tree byte-identical

- **WHEN** a `testutil::ScratchDir` repository holding one change with a `tasks.md` is
  snapshotted; a `Dashboard` is built from it by `ui::load`; `run_loop` is driven at 120x20
  and again at 60x20 with the **real** `ui::read_artifact`, a real `watch::start` over that
  tree, a scripted `Refresher`, and a key script that presses `r` and then every ASCII
  printable character from `!` to `~`; and the tree is snapshotted again
- **THEN** the two snapshots are identical — every path, its bytes, and its modification time
- **AND** re-reading the change's `tasks.md` still yields the same `Progress`
- **AND** a discriminating control rewrites one byte of `tasks.md` between two further
  snapshots and asserts they differ, proving the comparison can fail
- **AND** this scenario asserts **only** byte-identity and the re-read `Progress`. It makes no
  claim about `refresher.requests()`, and that separation is deliberate: an exact equality on
  a vector a **live** OS watcher can append to is protected only by the run finishing inside
  the 150ms debounce window — a budget nothing states, nothing asserts, and `cargo llvm-cov`'s
  instrumentation erodes. The request-vector claim is made in the scenario below, against a
  scripted `FsEvents` that can contribute nothing

#### Scenario: `r` requests exactly one full refresh through the loop

- **WHEN** the same scratch repository is driven through `run_loop` at 120x20 and again at
  60x20, with the **real** `ui::read_artifact`, a `ScriptedFs` whose `drain` always returns
  `Ok(None)`, a `RecordingRefresher`, and the same key script — `r`, then every ASCII
  printable character from `!` to `~`, then `Enter`, `Esc`, `Backspace`, `Tab`, the four
  arrows, and `Ctrl-C`
- **THEN** `refresher.requests()` is **exactly** `[Selection::All, Selection::All]` — the
  startup request `ui::load`'s flag produced, and the one `r` produced — and nothing else, so
  no other printable key refreshes and `r` refreshes exactly once per press
- **AND** the exact equality is sound here precisely because no live watcher is attached: a
  scripted `FsEvents` returning `Ok(None)` can contribute no third request under any timing
- **AND** while filtering, `r` contributes no request at all: the sweep's `/` starts filter
  mode and every later printable key, `r` included, types into the query

#### Scenario: The source-level guarantee holds tree-wide

- **WHEN** every `*.rs` file's production slice under `src/ui/`, plus `src/watch.rs`,
  `src/refresh.rs`, `src/agents.rs`, and `src/launch.rs`, is searched for `fs::write`,
  `File::create`, `OpenOptions`,
  `fs::remove_`, `fs::create_dir`, `fs::rename`, `fs::copy`, `set_permissions`,
  `fs::hard_link`, and `fs::soft_link`
- **THEN** there is no match
- **AND** `src/launch.rs` passes it while genuinely persisting the agent-name mapping, which is
  the point rather than a loophole: it writes only by calling `state::record`, and every write
  API named above lives in `src/state.rs`. A launcher that reached for `fs::write` itself — to
  log an invocation, to mark a pane, to cache a payload — would be caught here
- **AND** `#[cfg(test)]` modules are stripped before the search, because `ui::tests::load`
  legitimately calls `create_dir_all` to build the scratch repository it drives `ui::load`
  against, and a whole-file sweep would be red on a tree nobody had touched
- **AND** the stripper is proven non-vacuous: `src/ui/mod.rs`'s **test** slice must name a
  write API its production slice does not, or "strip `#[cfg(test)]`" is silently exempting
  the whole file
- **AND** the working tree at the base commit, including untracked and ignored files, holds
  no addition under `openspec/` outside this change's own artifact directory
- **AND** the runtime half of the same claim for the watcher specifically is
  `watch::tests::a_started_watcher_writes_nothing`: a snapshot pair taken around
  `watch::start` over a real scratch tree and the watcher's drop, so opening and closing an
  OS watch is proved to be a read. That test lives in `watch::tests::` rather than in
  `ui::tests::load::`, because no `ui::` test may open a watcher
- **AND** the runtime half for the launcher is `agent-launch`'s own acceptance scenario: a
  snapshot pair over the scratch repository taken around a run in which `a` was pressed and all
  three Herdr calls were logged, together with a second snapshot pair over the scratch **state**
  directory showing exactly one file — `agent-names.toml` — appearing there. One pair proves the
  repository was not written; the other proves the write that did happen went where
  `plugin-state` says it goes, rather than proving nothing was written at all

### Requirement: The list region's leading rows name refresh problems first

The requirement's name is kept verbatim from `live-refresh` because a delta's requirement
headers are its merge key; its subject is unchanged and only the number of problem sources
moves.

`ui::list::rows` SHALL emit one `RowKind::Problem` row for each of the following, in this
order, before every other row:

1. each entry of `Dashboard::launch.problems`;
2. `Dashboard::agents.problem` when — and only when — `Dashboard::agents.stalled` is set;
3. each entry of `Dashboard::refresh.startup`;
4. each entry of `Dashboard::refresh.problems`;
5. each entry of `ChangeSet::problems`.

All five use the existing `! `-prefixed grammar
`change-rows` already specifies, truncated with `…` at the interior width by the same
`pad_or_truncate_right` every other row uses.

A **stalled** agent snapshot's problem is rendered and a non-stalled one's is not, which is
the whole purpose of `agent-poller`'s `stalled` flag. An unreachable or erroring socket stays
silent — it is the documented standalone-TUI state — while a socket that has stopped
answering entirely is the one case where the pane is showing information it can no longer
stand behind, and saying nothing there is how a reader concludes stale badges are current
ones. It sits directly below the launch problems because, like them, it explains why a key
the reader just pressed did nothing.

`refresh.startup` precedes `refresh.problems` because it is the older and more general fact:
a missing `openspec` binary explains the whole session, a watcher error explains this moment.
This ordering is what makes the split of the two fields visible — before it, the first
watcher error erased the startup rows entirely.

Launch problems come first — `agent-launch`'s addition — because they are the only rows in the
list that answer a key the reader has just pressed. A watcher that would not start is a
condition and a `ChangeSet` problem is a fact about the tree; a failed launch is a reply, and
burying a reply under two standing conditions is how a reader concludes the key did nothing.
`launch.problems` holds at most one entry, is replaced wholesale by the next outcome or refusal,
and is cleared by a success, so it costs at most one row and cannot accumulate.

The rejected alternative is recorded rather than left implicit: placing launch problems
**below** refresh problems would follow the landed "standing above transient" ordering, and is
rejected because it optimises for a condition the reader has already seen at the expense of the
one they are waiting for.

Refresh problems come next because they are the ones that outlive a reload: a watcher that
would not start is a standing condition, while a `ChangeSet` problem is re-derived from the
tree on every cycle and may vanish on the next one.

The `repo.is_none()` early return SHALL be unchanged. A dashboard with no repository starts no
watcher, so it can carry no refresh problem, and — with `launch::none()` as its launcher and no
selected change for `launch::decide` to act on — no launch problem either; the no-repository
block stays the three rows `change-rows` specifies. A dashboard with no repository also polls
no agents through `agents::none()`, so it can carry no stall row either, and its
`refresh.startup` can hold only the configuration and probe fallbacks — never a watcher
problem, since no watcher was started.

#### Scenario: All five problem sources render in their specified order

- **WHEN** a `Dashboard` carries one `launch.problems` entry `launch failed`, an
  `agents` snapshot with `stalled: true` and `problem: Some("herdr agent list has not answered
  in 5s")`, two `refresh.startup` entries `openspec binary not found` and
  `config.toml: archived_count not set`, one `refresh.problems` entry `watch failed`, and one
  `changes.problems` entry `openspec/changes unreadable`, rendered at 120x20 and 60x20
- **THEN** interior rows 0 through 5 are, in order, `launch failed`, the stall reason, the two
  startup reasons in the order they were seeded, `watch failed`, and
  `openspec/changes unreadable`, and row 6 is the first change row
- **AND** all six carry `RowKind::Problem` and none is addressable by `selected`
- **AND** each is exactly the interior width — 38 columns at 120 and 58 at 60 — so the
  ordering assertion is made at both widths rather than only at the wide one

#### Scenario: A non-stalled agent problem draws no row

- **WHEN** the same dashboard is rendered with the snapshot's `stalled` set to `false`, its
  `problem` unchanged
- **THEN** the stall row is absent and the remaining five rows shift up by one, byte-identical
  to the same dashboard with an empty `agents.problem`
- **AND** this is the documented silent standalone-TUI state, so an unreachable socket still
  costs no row

#### Scenario: A watch problem is the list's first row

- **WHEN** a `Dashboard` with one active change and `refresh.problems` holding
  `filesystem watch unavailable for /r/openspec: No path was found` is rendered at 120x20 and
  at 60x20
- **THEN** the list region's interior row 0 begins `! filesystem watch unavailable` at both
  widths, and its row 1 is the change row
- **AND** the row is exactly the interior width — 38 columns at 120 and 58 at 60 — padded or
  truncated with a trailing `…`, never wrapped onto a second row

#### Scenario: Refresh problems precede change-set problems

- **WHEN** a `Dashboard` carries one `launch.problems` entry `launch failed`, one
  `refresh.problems` entry `watch failed`, and one
  `changes.problems` entry `openspec/changes unreadable`, rendered at both widths
- **THEN** interior row 0 is the `launch failed` row, row 1 is the `watch failed` row, and row 2
  is the `openspec/changes unreadable` row, in that order
- **AND** all three carry `RowKind::Problem`, so `ui::view` styles them identically and none is
  addressable by `selected`
- **AND** the same dashboard with `launch.problems` emptied renders rows 0 and 1 byte-identically
  to the two-row list this scenario specified before `agent-launch` existed

#### Scenario: No refresh problem draws no extra row

- **WHEN** a `Dashboard` with empty `launch.problems`, empty `refresh.problems`, and one active
  change is rendered at
  both widths
- **THEN** the list region's interior row 0 is the change row, exactly as it is today
- **AND** the rendered buffer is byte-identical to the same dashboard rendered before this
  change added the field, so the common case gained no row
- **AND** the same holds for `launch.problems`: a pane that has launched nothing, or whose last
  launch succeeded, renders byte-identically to one that has no launch tier at all

#### Scenario: A launch problem is the list's first row at both widths

- **WHEN** a `Dashboard` with one active change and `launch.problems` holding
  `herdr agent start exited with code 1: agent name c-2fa-support is already used` is rendered
  at 120x20 and at 60x20
- **THEN** the list region's interior row 0 begins `! herdr agent start exited` at both widths,
  and its row 1 is the change row
- **AND** the row is exactly the interior width — 38 columns at 120 and 58 at 60 — padded or
  truncated with a trailing `…`, never wrapped onto a second row
- **AND** at 38 columns the row is visibly truncated with `…` while at 58 it is longer, so the
  truncation is a width branch rather than unconditional
- **AND** the row carries no badge cell and is not addressable by `selected`, whatever
  `attribution().badges` holds

### Requirement: The per-frame artifact read is an accepted, bounded cost

`run_loop` calls `Dashboard::sync_detail(read)` before every draw, and the production reader
is `std::fs::read_to_string`. This is the one genuine blocking filesystem call left on the
render path, and this requirement exists so the decision to keep it is recorded rather than
discovered by a later reader as an oversight.

The read SHALL remain on the render path, and it SHALL remain bounded by the cache
`artifact-content` already specifies: `sync_detail` resolves content once per
`(change directory, tab)` key and re-reads only when that key changes or when
`refresh.reload` is set by an `adopt`. In steady state — a held `j`, an idle pane, a
scrolling reader — no read happens at all, and the frame costs nothing.

A read therefore happens on exactly three occasions: a tab switch, a change selection that
moves the detail region, and the first frame after a refresh is adopted. Each is a direct
consequence of an action the user just took or of new content arriving, each reads one
artifact file, and each is the frame whose whole purpose is to show that file. Moving the
read to the refresh worker would buy a smoother frame on a slow filesystem at the cost of a
fourth request/response protocol, a second cache with its own invalidation, and a tab switch
that renders empty for one frame before filling in — a worse pane in the common case to
protect the rare one.

This SHALL be recorded as a **deliberate** decision and SHALL NOT be treated as a violation
of "the render path blocks on nothing but the terminal", whose subject is the four background
collaborators. `AGENTS.md` and `SPEC.md` SHALL say so explicitly, so the exception is one
sentence a reader can find rather than an inconsistency they must reconstruct.

The cache SHALL be proved to hold, since it is the entire basis of the acceptance: an
implementation that re-read on every frame would satisfy every rendering assertion in this
capability while stalling the loop on every frame of a held key.

#### Scenario: A held key causes no read

- **WHEN** `run_loop` is driven at 120x20 and 60x20 over a counting reader with a script of
  ten `Char('j')` presses at the detail route on a change whose selected tab does not change
- **THEN** the reader was called exactly **once** across all ten frames
- **AND** the rendered content is identical on every frame, so the cache served every frame
  after the first

#### Scenario: A tab switch and an adopted refresh each cause exactly one read

- **WHEN** the same drive presses `]` once, then takes a `Merged` result on a later iteration
- **THEN** the counting reader was called exactly twice more: once for the new tab, once for
  the adopted set's forced `reload`
- **AND** neither call happened on an iteration that changed neither the key nor `reload`, so
  the read is caused by the event and not by the frame
