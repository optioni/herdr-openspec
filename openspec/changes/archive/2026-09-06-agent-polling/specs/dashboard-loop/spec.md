## MODIFIED Requirements

### Requirement: `Dashboard` is a plain state value with no rendering and no I/O

`ui::app::Dashboard` SHALL carry exactly **ten** fields: `repo: Option<PathBuf>` — the
repository root when one was found; `searched_from: PathBuf` — the directory the walk began
at, rendered by `change-rows`' no-repository state; `changes: changes::ChangeSet`;
`route: Route`, an enum of `List` and `Detail`; `quit: bool`, set by the quit action;
`selected: usize`, the index into the visible list defined by `list-selection`;
`filter: Filter`, the query and mode defined by `list-filtering`; `detail: Detail`, the
detail region's state defined by `detail-scroll`, `artifact-tabs`, and `artifact-content`;
`refresh: Refresh`, the live tier's state defined by `live-updates`; and
`agents: agents::AgentSnapshot`, the latest agent poll's outcome defined by `agent-poller`.

`ui::app::Filter` SHALL carry exactly two fields: `query: String` and `active: bool`.
`ui::app::Detail` SHALL carry exactly **five** fields: `source: String`, `scroll: usize`,
`tab: usize`, `problems: Vec<String>`, and `loaded: Option<(PathBuf, usize)>`. Three of them
are `detail-view`'s: `tab` is the selected artifact's position, `problems` names each
artifact file that could not be read, and `loaded` is the `(change directory, tab)` key whose
content `source` currently holds — the cache key `artifact-content`'s `sync_detail` compares
against, and the reason an unchanged selection re-reads nothing. `tasks-tab` adds **no**
field to any of the three: which grammar a tab renders is read from
`Change::artifacts[detail.tab].tracks_tasks` on every draw, never stored on the dashboard.

`ui::app::Refresh` is `live-refresh`'s addition and SHALL carry exactly **three** fields:
`requested: bool`, `reload: bool`, and `problems: Vec<String>`, defined by `live-updates`.
`Detail` is deliberately left at five: `reload` could have lived there, but `Dashboard` gains
one field either way and putting it on `Refresh` leaves `Detail`'s five construction-site
count untouched.

`agents::AgentSnapshot` is `agent-polling`'s addition and SHALL carry exactly **three** fields:
`agents: Vec<agents::Agent>`, `reachable: bool`, and `problem: Option<String>`, defined by
`agent-list`. It lives in `src/agents.rs` rather than in `src/ui/app.rs` because the poller that
produces it does, and because `src/ui/` may not name the CLI trait the poller reaches Herdr
through; the snapshot itself is plain data naming no trait, so the state value stays `Clone`,
`PartialEq`, and constructible in a test with no thread and no filesystem. It is a **sibling**
of `refresh`, not an entry on `Refresh::problems` and not one on `ChangeSet::problems`:
`refresh.problems` renders as a leading `!`-marked row, which an unreachable socket must not
produce, and `ChangeSet::problems` is replaced wholesale by `Dashboard::adopt` on every refresh,
which a standing condition must survive.

`Dashboard` SHALL carry no width, no layout mode, no column count, no interior height, no
terminal handle, and no frame. It **does** carry `detail.scroll` and `detail.tab`, and that
is not an exception to the rule: both are user-controlled positions, the counterparts of
`selected`, while the offset actually drawn is still derived on every draw by
`layout::scroll_offset` against the current content area's height. `detail.loaded` is not
geometry either: it is a record of what was read, not of how it was laid out. `refresh` is
not geometry: `requested` and `reload` are one-shot flags the loop consumes, and `problems` is
text. `agents` is not geometry: it is the most recent answer to a question, replaced wholesale.
No *derived geometry* is stored.

`Dashboard` SHALL carry no watcher, no worker handle, no poller, no channel, and no `Instant`.
The live tier's three collaborators reach the loop through `ui::driver::Live`, never through the
state value, so `Dashboard` stays `Clone`, `PartialEq`, and constructible in a test with no
thread and no filesystem.

None of `Dashboard`, `Filter`, `Detail`, and `Refresh` SHALL implement `Default` — neither
derived nor hand-written, anywhere in the crate — and every construction and every
destructuring of any of them SHALL name every field, with no `..` rest, so a field added later
fails to compile at each site rather than defaulting silently. The same SHALL hold for
`agents::Agent`, `agents::Listed`, and `agents::AgentSnapshot`, and the check SHALL be the same
check run a second time with its positive-control file parameterised to `src/agents.rs` rather
than a second copy of it on disk: two versions of one check is how a run and a record drift
apart. `change-model`'s existing gate does not reach any of these seven types: that gate is
stated over `Change`, `ChangeSet`, `ArtifactRef`, and `Origin` in `src/changes.rs`, and none of
these is one of those nor there.

`src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/layout.rs`, `src/ui/list.rs`,
`src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` SHALL name
no filesystem, process, environment, network, or standard-I/O API. Terminal work lives in
`src/ui/terminal.rs`, event reading in `src/ui/event.rs`, and startup loading, the one
artifact-read binding, and the composition root in `src/ui/mod.rs`; the **eight** files above
are the pure side of the render seam, and `agent-polling` adds no ninth — `src/watch.rs`,
`src/refresh.rs`, and `src/agents.rs` sit outside `src/ui/` entirely, which is what keeps this
set and the `NOCLI-SHELL` set unchanged, at **eleven** `*.rs` files under `src/ui/`.
A view test that needs a real directory means logic leaked across that seam.

`src/ui/tasks.rs` is `tasks-tab`'s addition to that set. It renders the tracked-tasks tab by
calling `tasks::parse` — a pure function over a `&str` — on the source
`Dashboard::sync_detail` already read through the injected reader, and never `tasks::read`,
which is the filesystem edge. Because `tasks::read` matches none of the search patterns
above, the searched set SHALL additionally be searched for `tasks::read`, so the one
filesystem call a checklist renderer would plausibly reach for is caught by the same check
rather than by nothing.

`Change` and `ChangeSet` literals SHALL appear only in `src/changes.rs`, test fixtures
included, so every construction site stays inside the file `change-model`'s gate searches.
`ui::detail`'s, `ui::tasks`'s, and `ui::view`'s tests SHALL therefore build changes carrying
artifacts through a constructor in `src/changes.rs` — `changes::fixture::with_artifacts`, and
`changes::fixture::track_tasks_at` for one carrying a marked artifact — rather than through a
literal of their own. The same holds for `src/watch.rs`, `src/refresh.rs`, and `src/agents.rs`,
which the same tree-wide search now covers at a file count of **22**.

#### Scenario: `Dashboard` has no `Default` and no site elides a field

The scenario's name is kept verbatim from `tui-shell` because a delta's scenario headers are
its merge key; its subject is unchanged and only the type list and the field count move.

- **WHEN** every `*.rs` file under `src/` is searched, for each of the type names
  `Dashboard`, `Filter`, `Detail`, `Refresh`, `Agent`, `Listed`, and `AgentSnapshot`, for
  `impl Default for <name>` — the target path-qualified or bare — for a `Default` inside the
  `#[derive(...)]` immediately preceding `struct <name>`, and for a `..` appearing inside a
  `<name> { … }` literal or pattern, brace-matched from the opening `{` to its partner so a
  multi-line elision rustfmt spread over several lines is seen
- **THEN** there is no match for any of the seven
- **AND** the check fails when `src/ui/app.rs` is absent, and it is paired with a positive
  control asserting that `src/ui/app.rs` **does** contain `struct Dashboard`, `struct Filter`,
  `struct Detail`, and `struct Refresh`, anchored on both sides so a rename fails the control
  rather than leaving every leg searching for a name that is no longer there
- **AND** the control's file is a **parameter**, defaulting to `src/ui/app.rs`, and the check is
  run a second time with it set to `src/agents.rs` for `Agent`, `Listed`, and `AgentSnapshot`,
  so the three new types are covered by the same executable file rather than by a fork of it
- **AND** the search is judged against a counted minimum of literal or pattern spans, raised
  to **90** for this change and measured at **107** on the tree before it, so a broken pattern
  that scanned nothing fails rather than reporting a clean tree
- **AND** the check is proven able to fail: run against a copy of `src/` carrying
  `impl Default for Detail { … }`, again against a copy carrying
  `let Detail { source, .. } = d;`, again against a copy carrying `#[derive(Default)]`
  immediately above `struct Refresh`, and again against a copy carrying
  `#[derive(Default)]` immediately above `struct Agent`, it reports each violation
- **AND** the planted `..` control is written in the **multi-line** form this tree's rustfmt
  output actually produces as well as the single-line form. The reason recorded in
  `markdown-viewer`'s version — "the compile-time companion is what covers it" — is
  **retired**: `detail-view` established that the companion destructures one value and so
  catches a field added to the type, never an elision at some other site, and replaced the
  same-line grep with the brace-matching pass named above. The companion is kept for what it
  genuinely does, below
- **AND** a compile-time companion exists: a test destructures a `Dashboard` with an
  exhaustive pattern naming all **ten** fields and no `..`, a second destructures a `Filter`
  naming both, a third destructures a `Detail` naming all five and no `..`, a fourth
  destructures a `Refresh` naming all **three** and no `..`, a fifth destructures an `Agent`
  naming all **eight**, a sixth destructures a `Listed` naming both, and a seventh destructures
  an `AgentSnapshot` naming all **three**, so adding a field breaks the build at that site
  rather than passing a source grep that never saw it
- **AND** `agents::AgentStatus` is deliberately outside the swept type list: the check's positive
  control is anchored on `struct <T> {`, so an enum cannot be added to it without breaking that
  control, and a `Default` on the enum would change nothing because every construction site is a
  `match` arm naming a variant

#### Scenario: The pure view files name no I/O API

- **WHEN** `src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/layout.rs`, `src/ui/list.rs`,
  `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` are
  searched for `std::fs`, `std::io`, `std::env`, `std::process`, `std::net`, `File::`,
  `read_to_string`, `tasks::read`, and `Command`
- **THEN** there is no match in any of the eight
- **AND** the searched set is still exactly those eight after `agent-polling`: the watcher lives
  in `src/watch.rs`, the worker in `src/refresh.rs`, and the poller in `src/agents.rs`, all
  three outside `src/ui/`, so the pure set neither grows nor shrinks
- **AND** the check fails when any of the eight files is absent, rather than reporting a
  clean tree
- **AND** it is paired with a positive control asserting that `src/ui/terminal.rs` **does**
  name `std::io`, so a search that matched nothing because it searched nothing fails instead
  of passing
- **AND** the check is proven able to fail against a copy carrying `use std::fs;` inside
  `src/ui/tasks.rs`, which is the file `tasks-tab` added to the set, and against a copy
  carrying `crate::tasks::read(p)` inside `src/ui/detail.rs`, which is the pattern that change
  added to the search

#### Scenario: The shell never names the CLI seam

- **WHEN** every `*.rs` file under `src/ui/` is searched for `from_cli`, `OpenspecCli`,
  `HerdrCli`, `CliChanges`, and `npm_prefix`
- **THEN** there is no match: the dashboard shell reaches OpenSpec state only through
  `changes::from_files` and through `refresh::RefreshResult`, whose two variants carry a
  `ChangeSet` and never a `CliChanges`, and it reaches Herdr only through `agents::AgentPoll`,
  whose `drain` carries an `AgentSnapshot` and never a `HerdrCli`, so it is complete with no
  `openspec` binary installed and no Herdr socket reachable
- **AND** this is the structural proof that both external programs are off the render path:
  `run_loop` cannot call the `openspec` binary or the `herdr` binary because no file it lives
  in may name the traits that reach them
- **AND** `src/ui/mod.rs` composes both real handles without naming either trait, because
  `cli::worker_cli_from_env` and `cli::agent_cli_via` carry the types in their own signatures
- **AND** it is paired with a positive control asserting that `src/changes.rs` **does**
  name `OpenspecCli`, so the search is proven to be capable of matching
- **AND** the check fails when `src/ui/` holds fewer than **eleven** `*.rs` files, the count
  `tasks-tab` left behind and neither `live-refresh` nor `agent-polling` changes, so a merged or
  deleted module is a deliberate update to the invocation rather than a silent shrink of the
  searched set

#### Scenario: Change literals live only in the gated file

- **WHEN** every `*.rs` file under `src/` other than `src/changes.rs` is searched for a
  `Change {` or `ChangeSet {` literal or pattern, the type name preceded by a
  non-identifier character so `ArtifactChange` and `Vec<&Change>` do not match
- **THEN** there is no match, so the tab-bar, header, checklist, watcher, worker, and poller
  tests' fixtures — which need changes carrying real `ArtifactRef` values — are built by
  constructors inside `src/changes.rs` rather than by literals the existing gate cannot see
- **AND** the searched set is now **22** files, `src/watch.rs`, `src/refresh.rs`, and
  `src/agents.rs` included, and the check fails below that count
- **AND** it is paired with a positive control asserting that `src/changes.rs` **does**
  contain `ChangeSet {`, and it fails when `src/changes.rs` is absent
- **AND** the check is proven able to fail: run against a copy of `src/` carrying a
  `Change {` literal in `src/agents.rs`, it reports it

#### Scenario: The render path names no channel, thread, lock, or clock

- **WHEN** `src/ui/driver.rs`'s production slice — everything above its first line-anchored
  `#[cfg(test)]` — is searched for `.recv(`, `recv_timeout`, `try_recv`, `.join()` with empty
  parentheses, `JoinHandle`, `thread::spawn`, `thread::sleep`, `mpsc`, `Mutex`, `RwLock`, and
  `Condvar`
- **THEN** there is no match: the loop reaches the watcher, the worker, and the poller only
  through the three trait objects `ui::driver::Live` carries, and every method of all three is
  non-blocking
- **AND** the `.join()` pattern is written with **empty** parentheses rather than as a bare
  `.join(`, because `Path::join` and `str::join` both take an argument and are ordinary
  correct code, while a `JoinHandle`'s `join` takes none — a bare pattern would be a false red
  the first time the loop built a path
- **AND** every `*.rs` file under `src/ui/`, **tests included**, is searched for
  `Instant::now`, `SystemTime::now`, `.elapsed()`, and the bare path prefix `Instant::`, and
  there is no match. This leg is deliberately whole-file rather than production-only: a test
  that reads the clock is exactly the timing flake this change exists not to reintroduce, and
  both the debounce and the poller's schedule keep their clocks outside `src/ui/`
- **AND** the outer-loop wiring test `agent-polling` adds under `src/ui/mod.rs` does **not**
  break this leg: the deadline it waits to lives in `testutil::UntilReady` in `src/lib.rs`,
  which is outside the swept directory, and the test names no clock of its own
- **AND** because that sweep is **comment-inclusive**, no doc comment under `src/ui/` may
  spell a clock path either: a comment reading "the crate's one `Instant::now()` lives in
  `watch::RealFsEvents::drain`" fails the check on otherwise-correct code, which is the
  `NOTABSEAM` failure mode this repository has already shipped once. Every doc comment under
  `src/ui/` therefore says "the clock" or "a clock", never `Instant::now()` — the same rule
  `markdown-render` imposes on `src/ui/markdown.rs`'s prose about `ratatui`, stated here so
  it is a documented constraint rather than a surprise
- **AND** a third leg searches **inside the three seam modules**, which the first two never
  reach: `src/watch.rs`'s production slice names no `.recv(`, `recv_timeout`, `.join()`,
  `park_timeout`, or `thread::park`, and `src/refresh.rs`'s and `src/agents.rs`' production
  slices name none of those **before** their single `thread::spawn`. That is what makes "every
  method of all three traits is non-blocking" a check rather than a doc comment:
  `FsEvents::drain`, `Refresher::take_result`, and `AgentPoll::drain` are called on every frame,
  and a `recv_timeout` in any of them would satisfy legs 1 and 2 while delaying every draw
- **AND** it is paired with six positive controls, checked before their sweeps:
  `src/refresh.rs`'s and `src/agents.rs`' production slices must each name `mpsc` and
  `thread::spawn`; `src/watch.rs` must name a clock — or the patterns are broken, or a worker is
  not a worker, or the debounce grew a hidden clock somewhere else; and **all three** seam
  modules' production slices must name `try_recv`, or their non-blocking receives are not there
  at all
- **AND** Guard D counts the line-anchored `#[cfg(test)]` attributes of **all three** seam
  modules and requires exactly one each, since the slicer truncates at the first and every
  sweep below it is silent; and Guard E requires the render-path method to be declared above the
  spawn in both `src/refresh.rs` (`fn take_result`) and `src/agents.rs` (`fn drain`), each taken
  as the **last** matching line rather than the first, because the first is the trait's abstract
  signature and necessarily precedes every spawn — the exact defect `live-refresh`'s own Change
  Review repaired
- **AND** the check is proven able to fail against a copy carrying `use std::sync::mpsc;` in
  `src/ui/driver.rs`'s production slice, against a copy carrying `Instant::now()` anywhere
  under `src/ui/`, against a copy whose `RealFsEvents::drain` calls `rx.recv_timeout(d)`,
  against a copy whose `Refresher::take_result` does the same, against a copy whose
  `RealAgentPoll::drain` does the same, and against a copy that moves `RealAgentPoll::drain`
  below `agents::start`
- **AND** the leg-1 pattern is token-based and its OK line claims only what it checked: a
  blocking receive reached through a type alias (`for _ in rx.iter()`) names no `mpsc`, no
  `.recv(`, and no `.join()`, and would pass. The trait contract plus leg 3 are what carry
  that case, not leg 1's grep

#### Scenario: No test sleeps and then asserts something has already happened

- **WHEN** every `*.rs` file under `src/` and `tests/` is split at line-anchored `#[test]`
  attributes and every span naming `thread::sleep`, `sleep_ms`, `park_timeout`, or
  `thread::park` is inspected
- **THEN** every such span also names a `deadline` and a `while` or `loop`, so the sleep is
  the pause inside a deadline-bounded poll rather than a fixed wait followed by an assertion
- **AND** the check is **not** a blanket prohibition, because the tree already carried three
  correct sleeps before `live-refresh` — `src/cli.rs`'s
  `a_program_that_reads_stdin_returns_rather_than_blocking` and `tests/cli.rs`'s two
  `try_wait` polls, one of which carries the comment recording the flake that produced the
  rule. A blanket rule would have been red on an unmodified tree and the only ways out would
  have been deleting three correct tests or exempting two files
- **AND** the scan is judged against a floor on the number of sleep sites it **found** —
  measured at **four** before `agent-polling` and **five** after: most of this change's own
  waits are `recv_timeout` against a deadline inside `src/agents.rs`'s test module or
  `std::thread::yield_now` inside `testutil::UntilReady`, neither of which is a sleep, but
  proving "a poll in flight suppresses the next request" against the real seam needs a
  scratch program answering after a genuine bounded delay, checked by a deadline-bounded
  `thread::sleep` poll on `watch::RealFsEvents`'s own established terms — the fifth site,
  and the reason the floor moves rather than staying flat. A broken pattern that matched
  nothing still fails rather than reporting a clean tree
- **AND** `yield_now` remains deliberately outside the pattern: it has no duration, so a loop
  around it is a condition poll and cannot make an assertion premature
- **AND** the check carries a self-contained negative control run on **every** invocation, not
  only at plant time: a synthetic span reading
  `#[test] fn c() { std::thread::sleep(d); assert!(happened); }` must be reported, and a
  synthetic deadline-bounded poll must not be, or the scan is declared broken and the check
  fails
- **AND** the splitter's known limit is stated rather than discovered: a "span" is one
  `#[test]` function **plus everything defined after it** up to the next `#[test]`, so a
  sleeping helper placed below a correct deadline-bounded test inherits that test's verdict.
  The second leg is what closes it where this change's own tests live
- **AND** a second leg forbids a sleep **at all** under `src/ui/`: every test of the render
  seam drives scripted doubles and an injected `now`, so there is nothing to wait for. It
  deliberately does **not** cover `src/watch.rs`, whose one real-watcher test polls to a
  deadline with a 10ms sleep between iterations — the same shape `tests/cli.rs` already uses,
  and the shape leg 1 accepts. Banning it there would have forced a `yield_now` busy-spin
  that holds a core for the whole window and competes for CPU with the `notify` thread
  producing the event it waits for
- **AND** the leg still covers `src/ui/mod.rs` after `agent-polling`'s outer-loop test lands
  there: that test waits through `testutil::UntilReady`, whose own wait is `yield_now` and
  whose clock lives in `src/lib.rs`, so nothing under `src/ui/` sleeps
- **AND** the check is proven able to fail against a copy carrying a `#[test]` function in
  `src/ui/layout.rs` that sleeps 200 milliseconds and then asserts

### Requirement: The loop draws before it waits and stops when quit is set

`ui::driver::run_loop(terminal, dashboard, events, live, read, tick)` SHALL be generic over
any `ratatui::backend::Backend` and any `ui::event::EventSource`, so tests drive it with a
`TestBackend` and a scripted event source and no terminal exists in the test process. `read`
is the `artifact-content` reader: a `&dyn Fn(&Path) -> Result<String, String>`, so no
filesystem API is named in `src/ui/driver.rs` and tests drive the loop with an in-memory
double.

`live` is `live-refresh`'s addition, extended by `agent-polling`: a `ui::driver::Live` carrying
`&mut dyn watch::FsEvents`, `&mut dyn refresh::Refresher`, and `&mut dyn agents::AgentPoll`, all
of whose every method is non-blocking. It is a struct rather than three further parameters so
the signature stays at **six** arguments and the three collaborators are named as one concept —
cohesion, not a lint. Measured on this crate and toolchain, clippy's `too_many_arguments` fires
at **eight** parameters, not seven, so a seventh would not have tripped it; the crate's single
`#[allow(clippy::too_many_arguments)]`, on `changes::build_change`, is itself vestigial for the
same reason. That correction is recorded here so a later change does not inherit a forcing
constraint that does not exist and choose a worse shape believing it had no option.

`Live` SHALL be constructed only with all three fields named and no `..` rest, so a collaborator
added to the loop fails to compile at every construction site — the compile-time half of the
guarantee whose behavioural half is `agent-poller`'s wiring test. It cannot implement `Default`
at all, since every field is a `&mut dyn` reference, so no source sweep is needed for it; it is
also outside `NODEFAULT-UI`'s reach, whose positive control anchors on `struct <T> {` and cannot
match a type generic over a lifetime.

Each iteration SHALL, in this order:

1. when `dashboard.refresh.requested` is set, request `Selection::All` of `live.refresher` and
   clear the flag;
2. `live.fs.drain()`, and on a non-empty batch request `watch::invalidate(repo, &paths)` of
   `live.refresher`; on `Err`, the reason replaces `dashboard.refresh.problems` wholesale and
   the loop continues;
3. `live.refresher.take_result()`, and on `Some(_)` adopt the carried `ChangeSet` through
   `Dashboard::adopt`;
4. `live.agents.drain()`, and on `Some(snapshot)` replace `dashboard.agents` with it;
5. `dashboard.sync_detail(read)`;
6. draw the frame;
7. `dashboard.normalise_scroll(area)`;
8. wait up to
   `watch::poll_timeout(tick, watch::soonest(live.fs.pending_in(), live.agents.pending_in()))`
   for an event.

Steps 1 to 4 precede the sync and the draw, so a result or a snapshot taken this iteration is
visible in the frame this iteration draws rather than the next; and every one of them is
non-blocking, so the pane is still painted with the selected artifact's content before any input
is read — not blank on the first frame and filled on the second. After applying an event's
action, the loop SHALL break when `dashboard.quit` is set, without syncing or drawing again.

Step 1 preceding step 2 is `live-updates`' rule and is restated here rather than contradicted:
this requirement previously listed the drain first, which disagreed with `live-updates` and with
the shipped loop, and `agent-polling` corrects it while adding step 4.

`EventSource::next_event(&mut self, timeout: Duration) -> Result<Option<Event>,
EventError>` SHALL return `Ok(None)` for a timeout with no event. A timeout SHALL NOT end
the loop and SHALL NOT be treated as an event.

On success `run_loop` SHALL return `LoopSummary { frames, polls }`, counting draws
performed and `next_event` calls made. `LoopSummary` SHALL gain **no** field for the live
tier: requests taken, results adopted, and snapshots drained are observed through the doubles'
own recorders, which keeps every landed `LoopSummary { frames, polls }` literal in the suite
unchanged. A draw error SHALL end the loop with `LoopError::Draw` carrying the backend error's
`Display` text; an event-source error SHALL end it with `LoopError::Events`. Neither a **watch**
error nor an **unreachable Herdr socket** SHALL end it or become a `LoopError`: both are
degraded states, and the pane keeps drawing from files. Neither `LoopError` SHALL panic, and
neither SHALL be retried in a loop that could spin.

`ui::driver::TICK` SHALL be 250 milliseconds and SHALL be what `ui::run` passes. It is now
the *upper bound* on a wait rather than the wait itself: `watch::poll_timeout` shortens it to
whichever of the debounce window and the agent poll is due sooner, so the loop wakes at the
moment either becomes due rather than at the next tick.

#### Scenario: The first frame is on screen before the first event is read

- **WHEN** `run_loop` is driven over a `Terminal<TestBackend>` at 120x20 with an event
  source whose script is **empty**, so its first `next_event` call returns
  `Err(EventError)`, an inert `Live` (`watch::none()`, `refresh::none()`, and
  `agents::none()`), and a reader returning `# proposal\n` for every path
- **THEN** `run_loop` returns `Err(LoopError::Events)`
- **AND** the backend's buffer nevertheless spells `OpenSpec` at row 0 column 0 and holds
  `┌` at row 1 column 0 and at row 1 column 40, so a complete frame was drawn before the
  failing wait — a loop that waited first would leave the buffer blank

#### Scenario: Timeouts are not events and do not end the loop

- **WHEN** `run_loop` is driven at 60x20 with a script of three `Ok(None)` timeouts
  followed by a Press of `Char('q')`, over an inert `Live`
- **THEN** it returns `Ok(LoopSummary { frames: 4, polls: 4 })`
- **AND** the dashboard's `quit` is true
- **AND** the event source recorded that every `next_event` call was made with the `tick`
  the caller passed, not a hard-coded value — an inert `FsEvents` and an inert `AgentPoll`
  both return `None` from `pending_in`, so `soonest` is `None` and `poll_timeout` returns the
  tick unchanged
- **AND** the recording reader recorded exactly **one** call across the whole run, because
  four iterations over an unchanged selection with no adopt re-read nothing

#### Scenario: A backend draw failure ends the loop rather than spinning

- **WHEN** `run_loop` is driven over a backend whose `draw` returns an error, with an event
  source whose script would supply a `q` press and an inert `Live`
- **THEN** it returns `Err(LoopError::Draw)` carrying the backend error's text
- **AND** the event source recorded **zero** `next_event` calls, proving the loop stopped
  at the failed draw rather than continuing past it
- **AND** the reader recorded **one** call, because the sync precedes the draw and the
  failure is in the draw

#### Scenario: Ctrl-C ends the loop

- **WHEN** `run_loop` is driven at 60x20 with a single Press of `Char('c')` carrying
  `KeyModifiers::CONTROL` and an inert `Live`
- **THEN** it returns `Ok(LoopSummary { frames: 1, polls: 1 })` and the dashboard's `quit`
  is true

#### Scenario: An ignored key redraws and keeps waiting

- **WHEN** `run_loop` is driven at 60x20 with a Press of `Char('Q')`, then a resize event,
  then a Press of `Char('q')`, over an inert `Live`
- **THEN** it returns `Ok(LoopSummary { frames: 3, polls: 3 })`
- **AND** the dashboard's route is still `List`, so neither input navigated

#### Scenario: A route change is visible in the next frame

- **WHEN** `run_loop` is driven at 60x20 with a Press of `Enter` followed by a Press of
  `Char('q')`, over an inert `Live`
- **THEN** it returns `Ok(LoopSummary { frames: 2, polls: 2 })`
- **AND** the final buffer's row 1 spells `Detail` starting at column 1 and the string
  `Changes` appears nowhere, so the second frame reflected the route the first event set

#### Scenario: `Live` cannot be built without naming the poller

- **WHEN** a compile-time companion in `ui::driver`'s tests destructures a `Live` with an
  exhaustive pattern naming all three fields and no `..` rest
- **THEN** the crate compiles, and adding a fourth field to `Live` breaks the build at that
  companion and at every construction site — `ui::run_wired` and every test that builds one
- **AND** the companion is the discriminating evidence rather than "the crate compiles at all":
  compilation alone would still succeed if a later change gave `Live` a `..` rest at one site
- **AND** `run_loop`'s parameter count is still six, and no
  `#[allow(clippy::too_many_arguments)]` is added by this change — checked as a diff against the
  base commit, not as a tree-wide grep, since `src/changes.rs` already carries one
