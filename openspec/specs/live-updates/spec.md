# live-updates Specification

## Purpose
TBD - created by archiving change live-refresh. Update Purpose after archive.

## Requirements

### Requirement: `Dashboard::refresh` carries the live tier's state

`ui::app::Refresh` SHALL carry exactly three fields:

- `requested: bool` — set by `Action::Refresh` and by `ui::load` at startup, cleared by
  `run_loop` once it has asked the refresher for a full reload. It is the counterpart of
  `quit`: a pure state value the loop observes, so `Dashboard::apply` stays a pure function
  that reaches no collaborator.
- `reload: bool` — set by `adopt`, consumed by `sync_detail`. It forces a re-read of an
  unchanged `(change directory, tab)` key, which is what makes an edit to the artifact
  currently on screen visible: the cache key `artifact-content` compares does not change when
  a file's **content** does.
- `problems: Vec<String>` — problems that outlive a reload, namely a watcher that would not
  start. Everything the CLI or the file walk reports rides on `ChangeSet::problems` instead
  and is replaced wholesale on every adopt, which is why a watcher failure cannot live there.

`Dashboard` SHALL therefore carry exactly **nine** fields. `Detail` SHALL still carry exactly
**five**: `reload` deliberately lives on `Refresh` rather than on `Detail`, because
`Dashboard` gains one field either way and putting it on `Refresh` keeps `Detail`'s five
unchanged.

`Refresh` SHALL NOT implement `Default` — neither derived nor hand-written, anywhere in the
crate — and every construction and destructuring of it SHALL name all three fields with no
`..` rest, on exactly the terms `Dashboard`, `Filter`, and `Detail` are already bound.

#### Scenario: `Refresh` has no `Default` and no site elides a field

- **WHEN** every `*.rs` file under `src/` is searched for `impl Default for Refresh`, for a
  `Default` inside the `#[derive(...)]` immediately preceding `struct Refresh`, and for a `..`
  inside a `Refresh { … }` literal or pattern, brace-matched from the opening `{` to its
  partner so a multi-line elision rustfmt spread over several lines is seen
- **THEN** there is no match
- **AND** the same check runs over `Dashboard`, `Filter`, and `Detail` in the same invocation,
  and is judged against a counted minimum of **90** literal or pattern spans, measured at
  **107** on the tree before this change
- **AND** a compile-time companion destructures a `Dashboard` naming all **nine** fields with
  no `..`, and a `Refresh` naming all **three**, so a tenth field breaks the build at that
  site rather than passing a source grep that never saw it

### Requirement: `r` forces a full refresh and types itself while filtering

`ui::app::Action` SHALL carry exactly **thirteen** variants — the twelve `dashboard-loop`
names plus `Refresh`.

While `filtering` is **false**, `KeyCode::Char('r')` with no modifiers SHALL map to
`Action::Refresh`. While `filtering` is **true** it SHALL map to `Action::FilterPush('r')`,
by `list-filtering`'s unchanged rule that every printable key types into the query — no
exception is carved out for it.

`Dashboard::apply(Action::Refresh)` SHALL set `refresh.requested` and change **nothing else**:
not `changes`, not `selected`, not `route`, not `detail`, not `filter`, not `quit`. It reaches
no collaborator, performs no I/O, and starts no work; `run_loop` is what turns the flag into a
request.

`Action::Refresh` SHALL NOT mutate `Dashboard::changes`, and the existing sweep proving that
of every action SHALL be extended from twelve variants to thirteen rather than left at twelve,
so a fourteenth variant added later still fails that test rather than slipping past it.

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

- **WHEN** every one of the **thirteen** `Action` variants is applied in turn to a
  `Dashboard` whose selected change carries artifacts, with `changes` compared for equality
  after each
- **THEN** `changes` is unchanged after every one
- **AND** the variant count is asserted to be exactly thirteen against an array the test
  builds through an exhaustive `match`, so a fourteenth variant is a compile error rather
  than a silent gap in the sweep

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
`ui::driver::Live` carrying `&mut dyn watch::FsEvents` and `&mut dyn refresh::Refresher`, and
SHALL perform, in this order, once per iteration, **before** the draw:

1. when `dashboard.refresh.requested` is set, `live.refresher.request(Selection::All)` and
   clear the flag;
2. `live.fs.drain()`; on `Ok(Some(paths))`, `refresh.request(watch::invalidate(repo, &paths))`;
   on `Err(e)`, the reason replaces `dashboard.refresh.problems` wholesale — never grown, since
   a watcher failing on every poll must not accumulate an unbounded list;
3. `live.refresher.take_result()`; on `Some(Files(set))` or `Some(Merged(set))`,
   `dashboard.adopt(set)`;

then `sync_detail`, then the draw, then `normalise_scroll`, then a wait of
`watch::poll_timeout(tick, live.fs.pending_in())` for a terminal event.

Step 1 SHALL run **before** step 2, not after: `ui::load`'s startup flag and a live watcher's
first-ever batch can both be pending on the very same, first iteration, and the recorded
request order that scenario proves — `[Selection::All, Selection::Only(...)]` — only holds
when the flag is turned into a request before the drain is consulted. An implementation that
checks the flag last would instead record `[Selection::Only(...), Selection::All]` on that same
iteration, failing "A filesystem batch becomes one selection" below. Applying a result at step
3, **before** the draw, is what makes a corrected change set visible in the very frame that
consumed it rather than the one after. Every one of the three steps is non-blocking by the
traits' contract, so the sequence adds no wait to the render path.

`run_loop` SHALL NOT name a channel, a thread, a lock, a blocking receive, or a clock:
`src/ui/driver.rs`'s production code names none of `.recv(`, `recv_timeout`, `try_recv`,
`.join()`, `JoinHandle`, `thread::spawn`, `thread::sleep`, `mpsc`, `Mutex`, `RwLock`, or
`Condvar`, and no file under `src/ui/` — tests included — names `Instant::now`,
`SystemTime::now`, or `.elapsed()`.

That is necessary and **not sufficient**, and the gap is named here rather than discovered
later: a sweep scoped to `src/ui/` says nothing about whether the two functions the loop calls
on every frame — `FsEvents::drain` and `Refresher::take_result` — block inside their own
modules. A `drain` written as `self.rx.recv_timeout(Duration::from_millis(150))` and a
`take_result` written as `self.rx.recv_timeout(Duration::from_millis(400)).ok()` would satisfy
every clause above, every trait signature, and every frame-and-poll count this change asserts,
while delaying each draw by more than half a second. The traits' "every method is
non-blocking" is a doc comment, and a doc comment is not a check.

The claim SHALL therefore additionally be checked **inside the two seam modules**:
`src/watch.rs`'s production slice names no blocking receive at all and does name `try_recv`;
`src/refresh.rs`'s production slice names no blocking receive **before** its single
`thread::spawn` — everything after that point is the worker body, which may block freely — and
does name `try_recv`. Together with the two `src/ui/` legs, those are the mechanical form of
"the watcher must not block the draw" and "no view test may be timing-based".

A **runtime** test for the same claim is deliberately not written: any test that fails when a
frame takes 300 milliseconds is an elapsed-time assertion, which is hazard 1 reintroduced in
the one place this change exists to eliminate it. The gate is structural on purpose.

The live tier SHALL reach **no Herdr socket**: neither `src/watch.rs`, `src/refresh.rs`, nor
`src/ui/driver.rs` names `HerdrCli`, so an unreachable socket — a supported state `SPEC.md` →
Degraded states already records — cannot degrade, delay, or fail a refresh. Agent polling is
`agent-polling`'s second poller, and this change deliberately leaves the socket untouched.

`ui::load` SHALL produce a `Dashboard` whose `refresh` is `{ requested: true, reload: false,
problems: [] }`, so the CLI correction is asked for on the first iteration with no keypress,
through the same step 1 the `r` key uses. `LoopSummary` SHALL be unchanged: the live tier is
observed through the doubles' own recorders, not through a fourth counter.

#### Scenario: The startup request is issued before the first wait

- **WHEN** `run_loop` is driven at 120x20 with a recording `Refresher` double whose
  `take_result` is always `None`, a `Dashboard` from `ui::load` with `refresh.requested` set,
  and a script of one `Char('q')` Press
- **THEN** it returns `Ok(LoopSummary { frames: 1, polls: 1 })`
- **AND** the double recorded exactly one `request`, carrying `Selection::All`
- **AND** `dashboard.refresh.requested` is false afterwards, so a single flag produces a
  single request rather than one per frame
- **AND** the same run at 60x20 behaves identically

#### Scenario: A result is adopted before the frame that shows it

- **WHEN** `run_loop` is driven at 120x20 and 60x20 over a dashboard whose file-sourced
  `alpha` reads `[4/9]`, with a `Refresher` double whose **first** `take_result` returns
  `Merged(set)` in which `alpha` reads `[7/9]`, and a script of one `Char('q')` Press
- **THEN** it returns `Ok(LoopSummary { frames: 1, polls: 1 })` — one frame, not two
- **AND** the buffer's list row for `alpha` ends in `[7/9]` at both widths, so the result
  reached the very frame that consumed it
- **AND** the double recorded exactly **one** `take_result` call for that one iteration, so
  the loop neither drains the channel in a spin nor waits for a second result

#### Scenario: A filesystem batch becomes one selection

- **WHEN** `run_loop` is driven over a dashboard whose repo is `/r`, with an `FsEvents` double
  whose first `drain` returns `Ok(Some([/r/openspec/changes/alpha/tasks.md]))` and whose later
  drains return `Ok(None)`, and a script of two `Ok(None)` timeouts then `Char('q')`
- **THEN** the `Refresher` double recorded exactly two requests: the startup
  `Selection::All`, then `Selection::Only({"alpha"})`
- **AND** no request was made on the iterations whose `drain` returned `Ok(None)`
- **AND** the assertion is an exact equality on the recorded request vector, so an
  implementation that requested `All` for every batch fails rather than passing

#### Scenario: A watcher error is recorded once and the loop continues

- **WHEN** an `FsEvents` double's `drain` returns `Err(WatchError)` on its first two calls and
  `Ok(None)` thereafter, with a script of three timeouts then `Char('q')`
- **THEN** `run_loop` returns `Ok(LoopSummary { frames: 4, polls: 4 })` — a watch error is
  never a `LoopError` and never ends the loop
- **AND** `dashboard.refresh.problems` holds exactly one entry naming the reason, not two: a
  watcher failing on every poll must not grow an unbounded problem list
- **AND** that problem is rendered as the list region's first `!`-marked row at both widths

#### Scenario: The wait shortens to the debounce deadline

- **WHEN** `run_loop` is driven with `tick` of 250ms and an `FsEvents` double whose
  `pending_in` returns `Some(90ms)` on its first call and `None` thereafter
- **THEN** the scripted event source recorded its first `next_event` timeout as `90ms` and
  every later one as `250ms`
- **AND** with `pending_in` returning `Some(0ms)` the recorded timeout is `1ms`, never `0ms`

#### Scenario: An inert live tier leaves the loop exactly as it was

- **WHEN** `run_loop` is driven with `watch::none()` and `refresh::none()` and the same
  scripts every landed `dashboard-loop` scenario uses
- **THEN** every landed frame count, poll count, buffer assertion, and reader call count is
  unchanged
- **AND** this is the case a machine with no `openspec` binary and no working watcher runs, so
  the dashboard degrades to exactly the behaviour that shipped in `tasks-tab`

### Requirement: The live tier writes nothing inside the repository

Watching a directory, classifying a path, requesting a refresh, running the CLI producer, and
adopting a result SHALL all be reads. No code path in `src/watch.rs`, `src/refresh.rs`, or the
loop's live tier SHALL create, modify, remove, or rename any file under `openspec/`.

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

- **WHEN** every `*.rs` file's production slice under `src/ui/`, plus `src/watch.rs` and
  `src/refresh.rs`, is searched for `fs::write`, `File::create`, `OpenOptions`,
  `fs::remove_`, `fs::create_dir`, `fs::rename`, `fs::copy`, `set_permissions`,
  `fs::hard_link`, and `fs::soft_link`
- **THEN** there is no match
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

### Requirement: The list region's leading rows name refresh problems first

`ui::list::rows` SHALL emit one `RowKind::Problem` row for each entry of
`Dashboard::refresh.problems`, in order, **followed** by one for each entry of
`ChangeSet::problems`, before every other row. Both use the existing `! `-prefixed grammar
`change-rows` already specifies, truncated with `…` at the interior width by the same
`pad_or_truncate_right` every other row uses.

Refresh problems come first because they are the ones that outlive a reload: a watcher that
would not start is a standing condition, while a `ChangeSet` problem is re-derived from the
tree on every cycle and may vanish on the next one.

The `repo.is_none()` early return SHALL be unchanged. A dashboard with no repository starts no
watcher, so it can carry no refresh problem, and the no-repository block stays the three rows
`change-rows` specifies.

#### Scenario: A watch problem is the list's first row

- **WHEN** a `Dashboard` with one active change and `refresh.problems` holding
  `filesystem watch unavailable for /r/openspec: No path was found` is rendered at 120x20 and
  at 60x20
- **THEN** the list region's interior row 0 begins `! filesystem watch unavailable` at both
  widths, and its row 1 is the change row
- **AND** the row is exactly the interior width — 38 columns at 120 and 58 at 60 — padded or
  truncated with a trailing `…`, never wrapped onto a second row

#### Scenario: Refresh problems precede change-set problems

- **WHEN** a `Dashboard` carries one `refresh.problems` entry `watch failed` and one
  `changes.problems` entry `openspec/changes unreadable`, rendered at both widths
- **THEN** interior row 0 is the `watch failed` row and row 1 is the
  `openspec/changes unreadable` row, in that order
- **AND** both carry `RowKind::Problem`, so `ui::view` styles them identically and neither is
  addressable by `selected`

#### Scenario: No refresh problem draws no extra row

- **WHEN** a `Dashboard` with empty `refresh.problems` and one active change is rendered at
  both widths
- **THEN** the list region's interior row 0 is the change row, exactly as it is today
- **AND** the rendered buffer is byte-identical to the same dashboard rendered before this
  change added the field, so the common case gained no row
