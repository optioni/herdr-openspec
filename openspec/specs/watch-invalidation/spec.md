# watch-invalidation Specification

## Purpose
TBD - created by archiving change live-refresh. Update Purpose after archive.

## Requirements

### Requirement: The filesystem-watch crate is confined to one module

`notify` SHALL be named in `src/watch.rs` and nowhere else in the crate, `tests/` included,
so the watcher is replaceable by editing one file — the same confinement `src/cli.rs` has
for process spawning and `src/ui/markdown.rs` has for the markdown parser.

`src/watch.rs` SHALL name no `ratatui` type, so a watched path never reaches the view
already styled and every function in the module is assertable without a frame. It SHALL name
no process-spawn API either: a filesystem watch is not a reason to shell out to `fswatch`.

The check that enforces this SHALL be token-specific and path-qualified rather than matching
the bare word `notify`, and SHALL NOT match `EventKind`: `KeyEventKind` and `MouseEventKind`
appear at twelve sites of correct, unmodified code in `src/ui/app.rs`, so a bare `EventKind`
alternative would make the check red on a tree this change had not touched.

#### Scenario: `notify` is named in exactly one file

- **WHEN** every `*.rs` file under `src/` and `tests/` other than `src/watch.rs` is searched
  for `notify::`, `notify_types::`, `RecommendedWatcher`, `RecursiveMode`, `FsEventWatcher`,
  `INotifyWatcher`, `use notify`, and `extern crate notify`
- **THEN** there is no match
- **AND** it is paired with a positive control asserting that `src/watch.rs` **does** match
  the same pattern, checked before the file count, so a gutted module is reported as a
  vacuous exclusion rather than as a clean tree
- **AND** the check fails when `src/watch.rs` is absent, and when fewer than **23** files are
  searched — the count this change leaves behind
- **AND** the check is proven able to fail: run against a copy carrying `use notify::Event;`
  in `src/ui/driver.rs`, it names that file and line

#### Scenario: The confined module carries no view type and no spawn

- **WHEN** `src/watch.rs` is searched, comments included, for `ratatui`, `Modifier`, `Style`,
  `Span`, `Rect`, `Frame`, and `Buffer`, and separately for `process::Command`,
  `Command::new`, and `Stdio`
- **THEN** neither search matches
- **AND** the search is deliberately comment-inclusive, because a doc comment naming a
  `ratatui` type is itself the coupling this forbids — the module's prose says "the view"
- **AND** the check is proven able to fail against a copy whose `src/watch.rs` carries the
  comment `// a ratatui Style`

### Requirement: A touched path is classified to the changes it invalidates

`watch::classify(repo: &Path, path: &Path) -> Touch` SHALL be a pure, total function
returning one of exactly four values, from the path's position relative to
`<repo>/openspec/`:

| Path, relative to `<repo>/openspec/` | `Touch` |
|---|---|
| `changes/<name>/…` with at least one component after `<name>`, where `<name>` is not `archive` | `Change(<name>)` |
| `changes/<name>` exactly, where `<name>` is not `archive` | `Repository` — a change directory appearing or vanishing changes the list itself |
| `changes/archive` or anything below it | `Archived` |
| `changes`, `config.yaml`, `schemas/…`, or anything else at or below `<repo>/openspec/` | `Repository` |
| anything **not** below `<repo>/openspec/`, including a relative path | `Outside` |

The rule SHALL be conservative rather than clever: an unrecognised path under `openspec/` is
`Repository`, never `Outside`. A `Repository` classification costs one extra `openspec list
--json` and a full file re-read, both of which the plugin performs on every cycle anyway; a
wrong `Outside` would silently stop the pane updating.

`watch::invalidate(repo: &Path, paths: &[PathBuf]) -> Selection` SHALL fold a batch:
`Selection::All` when any path classifies to `Repository` or `Archived`, otherwise
`Selection::Only` of the `Change` names, and `Selection::Only` of the empty set when every
path is `Outside`. `Only(empty)` is meaningful and is not the same as doing nothing — the
worker still re-reads files and still runs `openspec list --json`, which is how a change
appearing or vanishing is noticed.

Neither function SHALL touch the filesystem, canonicalize, or read the clock: both are pure
functions of their arguments, so a path that no longer exists classifies exactly as one that
does.

#### Scenario: A file inside one change invalidates that change and no other

- **WHEN** `invalidate` is called with repo `/r` and the single path
  `/r/openspec/changes/alpha/tasks.md`, in a repository whose changes are `alpha` and `beta`
- **THEN** it returns `Selection::Only({"alpha"})`
- **AND** the returned set does **not** contain `beta` — asserted as an exact set equality,
  not as a `contains`, because an assertion that everything was invalidated would pass either
  way and prove nothing
- **AND** the same holds for a nested path, `/r/openspec/changes/alpha/specs/x/spec.md`

#### Scenario: A change directory appearing or vanishing is a repository touch

- **WHEN** `classify` is called with `/r/openspec/changes/gamma` — the directory itself, with
  no component after it
- **THEN** it returns `Touch::Repository`, because the set of changes has changed and
  `openspec list --json` must be believed about membership
- **AND** `classify` on `/r/openspec/changes` returns `Repository` too

#### Scenario: The archive tier and the schema are repository touches

- **WHEN** `classify` is called with `/r/openspec/changes/archive`,
  `/r/openspec/changes/archive/2026-01-01-x/tasks.md`, `/r/openspec/config.yaml`, and
  `/r/openspec/schemas/tdd/schema.yaml`
- **THEN** the first two return `Touch::Archived` and the last two return `Touch::Repository`
- **AND** `invalidate` over a batch holding any one of the four returns `Selection::All`,
  because an archived change is permanently file-sourced and a schema edit changes every
  change's artifact list

#### Scenario: A path outside the repository is ignored

- **WHEN** `classify` is called with `/other/openspec/changes/alpha/tasks.md`, with `/r`
  itself, with `/r/README.md`, with the relative path `openspec/changes/alpha/tasks.md`, and
  with the empty path
- **THEN** every one returns `Touch::Outside`
- **AND** `invalidate` over a batch of only those returns `Selection::Only` of the empty set,
  not `Selection::All`, so an unrelated write elsewhere on the disk cannot force a full CLI
  reload

#### Scenario: A mixed batch unions its classifications

- **WHEN** `invalidate` is called with `[/r/openspec/changes/alpha/tasks.md,
  /r/openspec/changes/beta/design.md, /tmp/unrelated]`
- **THEN** it returns `Selection::Only({"alpha", "beta"})`
- **AND** with `/r/openspec/config.yaml` added to that same batch it returns `Selection::All`,
  because one repository-class path absorbs every per-change one

### Requirement: The debounce is a pure state machine over an injected instant

`watch::Debounce` SHALL coalesce touched paths over a window of **150 milliseconds**,
exposed as the constant `watch::DEBOUNCE`, and SHALL take the current instant as a
**parameter** on every method rather than reading a clock:

- `push(&mut self, paths: Vec<PathBuf>, now: Instant)` records paths and sets the window's
  end to `now + DEBOUNCE`, extending it on every later push — but **never past**
  `first_push + DEBOUNCE_MAX`, so the end is `min(now + DEBOUNCE, first_push + DEBOUNCE_MAX)`;
- `take_due(&mut self, now: Instant) -> Option<Vec<PathBuf>>` returns the accumulated paths
  and empties the state exactly when the window has ended, and `None` otherwise, including
  when nothing was ever pushed. "Empties the state" **includes the first-push instant the cap
  below is measured from**, so the next batch gets a fresh `DEBOUNCE_MAX` window. An
  implementation that clears the paths and the end but keeps `first_push` is green against
  every other assertion here and leaves its cap permanently in the past, so every later push
  is instantly due and the debounce is **off for the rest of the session**;
- `pending_in(&self, now: Instant) -> Option<Duration>` returns the time remaining, saturating
  at zero, or `None` when nothing is pending. It SHALL be exactly
  `self.end.map(|end| end.saturating_duration_since(now))` — **`end` is the receiver and `now`
  the argument**. `Instant - Instant` **panics** when its argument is the later of the two, and
  this is the one place that ordering is not guaranteed by the caller; the reversed operand
  order returns zero for the whole window rather than only past its end.

`watch::DEBOUNCE_MAX` SHALL be **1 second**, and the cap is not decoration. This is a
*sliding* window: without a ceiling, a writer saving more often than every 150ms defers the
batch forever. That writer is precisely this change's motivating case — an agent editing a
change's `tasks.md`, then its spec files, then its design — so an uncapped window would leave
the pane frozen exactly when it is most useful, and `SPEC.md` → Refresh's "approximately
150ms" would be unbounded.

Paths SHALL be coalesced without duplicates and in a deterministic order, so a batch is a set
rather than a log: FSEvents emits several events for one edit — a `Create(File)`, two
`Modify(Metadata(…))`, and a `Modify(Data(Content))` were measured for a single write — and
the pane must not run the CLI four times for one save.

Because `now` is a parameter, **no test of this type may sleep**. A test constructs
`Instant::now()` once as `t0` and asserts against `t0 + Duration::from_millis(149)` and
`t0 + Duration::from_millis(150)`. A test that slept and then asserted a window had already
elapsed is the defect this rule exists to make impossible.

#### Scenario: Nothing is due before the window and everything is due at it

- **WHEN** a `Debounce` is given `push([/r/openspec/changes/alpha/tasks.md], t0)` for a `t0`
  the test captured once
- **THEN** `take_due(t0)` and `take_due(t0 + 149ms)` both return `None`
- **AND** `take_due(t0 + 150ms)` returns `Some([…/alpha/tasks.md])`
- **AND** a second `take_due(t0 + 10s)` immediately after returns `None`, because taking a
  due batch empties the state
- **AND** no test in this requirement calls `thread::sleep`, `Instant::now()` more than once,
  or any other clock

#### Scenario: A later push extends the window

- **WHEN** a `Debounce` is given `push([a], t0)` and then `push([b], t0 + 100ms)`
- **THEN** `take_due(t0 + 150ms)` returns `None` — the window now ends at `t0 + 250ms`
- **AND** `take_due(t0 + 250ms)` returns both `a` and `b` in one batch
- **AND** `pending_in(t0 + 100ms)` is `Some(150ms)` and `pending_in(t0 + 250ms)` is
  `Some(0ms)`, while `pending_in` on an empty `Debounce` is `None`
- **AND** `pending_in(t0 + 400ms)` — a `now` **past** the window's end — is `Some(0ms)` and
  does not panic. This is the one assertion that discriminates
  `Instant::saturating_duration_since` from the obvious `end - now`, which panics; the three
  assertions above are all green against the panicking form

#### Scenario: A continuous writer cannot defer a batch forever

- **WHEN** a `Debounce` is pushed at `t0`, then at `t0 + 100ms`, `t0 + 200ms`, and so on
  through `t0 + 1100ms` — never leaving a 150ms gap, which is what an agent saving several
  artifacts in a burst looks like
- **THEN** `take_due(t0 + 1000ms)` returns `Some(batch)`, because the window's end was capped
  at `first_push + DEBOUNCE_MAX`, and the batch holds every path pushed up to that point
- **AND** without the cap `take_due` would return `None` at every one of those instants and
  the pane would never update while the agent was working
- **AND** after that `take_due`, a fresh `push([c], t0 + 2s)` is **not** due at `t0 + 2s` and
  **is** due at `t0 + 2s + 150ms` — the cap was reset with the batch, so a taken batch does not
  leave the debounce permanently open. This is the clause that discriminates an implementation
  which clears the paths and the end but not `first_push`
- **AND** `watch::DEBOUNCE_MAX` equals `Duration::from_secs(1)`, pinned by an assertion the
  way `DEBOUNCE` and `ui::driver::TICK` are
- **AND** every assertion in this scenario uses an injected `now`; no test in this
  requirement sleeps

#### Scenario: Repeated paths are coalesced

- **WHEN** a `Debounce` is given `push([a, a, b, a], t0)` and then `push([b], t0 + 10ms)` —
  the shape a single file save produces on macOS, where one write yielded four events
- **THEN** `take_due(t0 + 160ms)` returns exactly `[a, b]`, each once, in a deterministic
  order
- **AND** the resulting `Selection` therefore names each change once, so one save runs
  `openspec instructions apply` once rather than four times

#### Scenario: The window's length is pinned

- **WHEN** `watch::DEBOUNCE` is read
- **THEN** it equals `Duration::from_millis(150)`, pinned by an assertion the way
  `ui::driver::TICK` and `ui::layout::WIDE_MIN_WIDTH` are, so `SPEC.md` → Refresh's
  "approximately 150ms" has one place it is written down

### Requirement: The loop's wake-up shortens to the debounce deadline

`watch::poll_timeout(tick: Duration, pending_in: Option<Duration>) -> Duration` SHALL be a
pure function of two `Duration`s — reading no clock — returning `tick` when nothing is
pending and otherwise the smaller of `tick` and the remaining window, floored at
**1 millisecond**.

The floor is load-bearing rather than cosmetic: a zero timeout returned every iteration would
make `run_loop` spin without bound if a watcher ever reported a pending batch it then declined
to yield. One millisecond converts an **unbounded** spin into a **bounded** one — roughly a
thousand loop iterations a second, each of them a full `sync_detail`, draw, and
`normalise_scroll`. That is survivable long enough for a reader to notice and quit, not free;
the floor exists so a faulty `FsEvents` degrades to a hot pane rather than to a hung one. The
correct case never reaches it, because the next iteration's `drain` yields the batch and
`pending_in` returns `None` again.

`agent-polling` adds a **second** pending deadline beside the debounce's, and the loop has one
wait to serve both. `watch::soonest(a: Option<Duration>, b: Option<Duration>) -> Option<Duration>`
SHALL therefore be a pure function of two optional `Duration`s — reading no clock — returning
`None` when both are `None`, the present one when exactly one is present, and the smaller when
both are. `run_loop` SHALL wait
`poll_timeout(tick, soonest(live.fs.pending_in(), live.agents.pending_in()))`.

`poll_timeout`'s own signature SHALL NOT change, and the minimum SHALL NOT be taken inline in
`src/ui/driver.rs`. Both are deliberate. Keeping `poll_timeout` at two arguments leaves every
landed scenario and every landed assertion on it untouched, so a change about agents does not
re-open the debounce's contract; and a named function is directly asserted over all four
combinations of present and absent, whereas an inline `min` in the driver would be exercised
only through frame counts that pass whichever way it was written. `soonest` lives in
`src/watch.rs` beside `poll_timeout` because that is its only consumer and the two are one
piece of arithmetic; it names nothing about agents and nothing about the filesystem.

`ui::driver::TICK` SHALL remain 250 milliseconds. The worst-case latency from a file write to
a corrected frame is therefore `TICK` (discovering the event) plus `DEBOUNCE` (coalescing it)
plus the CLI's own 200–400ms — the post-debounce half of which this function removes by
waking the loop exactly at the window's end rather than at the next tick.

#### Scenario: The timeout is the tick when nothing is pending

- **WHEN** `poll_timeout(Duration::from_millis(250), None)` is called
- **THEN** it returns `Duration::from_millis(250)`

#### Scenario: The timeout is the remaining window when that is shorter

- **WHEN** `poll_timeout(250ms, Some(90ms))` and `poll_timeout(250ms, Some(400ms))` are called
- **THEN** they return `90ms` and `250ms` respectively — the smaller of the two, never the
  pending value unclamped
- **AND** `poll_timeout(250ms, Some(0ms))` returns `1ms`, never `0ms`, so a watcher reporting
  a pending batch it does not yield cannot spin the loop

#### Scenario: One tick serves two pollers

- **WHEN** `soonest` is called with `(None, None)`, `(Some(90ms), None)`, `(None, Some(40ms))`,
  `(Some(90ms), Some(40ms))`, and `(Some(40ms), Some(90ms))`
- **THEN** it returns `None`, `Some(90ms)`, `Some(40ms)`, `Some(40ms)`, and `Some(40ms)`
  respectively — commutative, and never the larger of two present values
- **AND** `poll_timeout(250ms, soonest(Some(900ms), Some(120ms)))` is `120ms`, so an agent poll
  becoming due inside the tick shortens the wait exactly as a debounce deadline does
- **AND** `poll_timeout(250ms, soonest(Some(900ms), Some(3s)))` is `250ms`, so a deadline
  further away than the tick never lengthens the wait
- **AND** `soonest` reads no clock: it is asserted with literal `Duration` values and takes no
  `Instant`, so no view test can reach a clock through it

### Requirement: The watcher seam is a trait with one real implementation

`watch::FsEvents` SHALL be a trait whose every method is **non-blocking**:

```rust
pub trait FsEvents: Send {
    fn drain(&mut self) -> Result<Option<Vec<PathBuf>>, WatchError>;
    fn pending_in(&self) -> Option<Duration>;
}
```

`drain` SHALL return `Ok(None)` when no debounced batch is ready — never blocking, never
sleeping, never waiting on a channel — and `Ok(Some(paths))` for a non-empty batch. It SHALL
never return `Ok(Some(vec![]))`: an empty batch and no batch are the same fact and having two
spellings of it invites a caller to request a refresh for nothing.

The non-blocking contract SHALL be **checked, not merely documented**: `src/watch.rs`'s
production slice SHALL name no `.recv(`, `recv_timeout`, `.join()`, `park_timeout`, or
`thread::park`, and SHALL name `try_recv`. A doc comment saying "non-blocking" is not a
check, and `fn drain(&mut self) -> Result<Option<Vec<PathBuf>>, WatchError>` is a perfectly
good signature for a function that blocks for 150 milliseconds on every frame — an
implementation that did exactly that would satisfy every other gate this change carries.

`pending_in` takes **no** `Instant`, unlike `Debounce::pending_in`. `RealFsEvents` SHALL store
the `Option<Duration>` its previous `drain` computed from `debounce.pending_in(now)`, using
the same `now` that `drain` already captured, and return it verbatim. That is what keeps the
crate's clock bindings at exactly **one**, and it is sound because `run_loop` calls `drain`
and then `pending_in` in the same iteration, so the value is at most one statement old.

Exactly one real implementation SHALL exist, holding a `notify::RecommendedWatcher`, the
`std::sync::mpsc::Receiver` it sends to, and a `Debounce`. It SHALL be constructed through
`watch::start(root: &Path) -> (Box<dyn FsEvents>, Vec<String>)`, which SHALL watch `root`
recursively and SHALL NOT return a `Result`: a watcher that will not start is a degraded
state, not a failure to open the pane.

`watch::none()` SHALL return the inert implementation — `drain` always `Ok(None)`,
`pending_in` always `None` — which is what `start` returns on failure and what `ui::run`
passes when no repository was found.

The real implementation SHALL hold the crate's **one** binding from `Debounce`'s injected
`now` to the real clock, `Instant::now()`, alongside the crate's three existing one-line
bindings to the real world (`cli::npm_prefix`, `config::env_lookup`, `ui::read_artifact`).
No file under `src/ui/` SHALL name `Instant::now`, `SystemTime::now`, or `.elapsed()`, tests
included — a view test reading a clock is precisely the timing flake this design removes.

#### Scenario: A watcher that will not start degrades and names the reason

- **WHEN** `watch::start` is called with a path that does not exist — `notify`'s `watch`
  returns `ErrorKind::PathNotFound` for it, deterministically and with no timing involved
- **THEN** it returns the inert implementation, whose `drain` is `Ok(None)` and whose
  `pending_in` is `None` forever
- **AND** the returned problem list holds exactly one string naming the path and the reason
- **AND** nothing panics and no `Result` reaches the caller, so the dashboard still opens and
  still paints from files

#### Scenario: A real watch reports a written file, polled to a deadline

- **WHEN** `watch::start` is called on a `testutil::ScratchDir`, a file is written at
  `<scratch>/changes/alpha/tasks.md`, and `drain` is polled in a loop that ends at a deadline
  **ten seconds** in the future
- **THEN** some poll returns `Ok(Some(paths))` whose paths include one whose final two
  components are `alpha` and `tasks.md` — compared by suffix, because macOS FSEvents returns
  canonicalized paths and `/var/folders/…` arrives as `/private/var/folders/…`
- **AND** the assertion is on a **condition reached within a deadline**, never on a state
  after a fixed sleep: the test's failure message names the written path and says the
  filesystem may not support change notification, so a genuine platform limitation is
  distinguishable from a defect
- **AND** the poll sleeps **10 milliseconds** between iterations, the same shape
  `tests/cli.rs`'s two `try_wait` polls already use in this crate. A sleep *inside* a
  deadline-bounded poll cannot make an assertion premature, because the condition is
  re-tested after every sleep; a `yield_now` spin would instead hold a core for the whole
  window and, on a loaded two-core runner, compete for CPU with the very `notify` thread
  producing the event it waits for
- **AND** the whole scratch tree is byte-identical before and after, proving a watch is a read

#### Scenario: The inert watcher never yields

- **WHEN** `watch::none()`'s `drain` and `pending_in` are called ten times each
- **THEN** every `drain` returns `Ok(None)` and every `pending_in` returns `None`
- **AND** no request is ever produced from it, so a dashboard with no repository, or one whose
  watcher failed, behaves exactly as the landed file-only dashboard did

#### Scenario: `openspec/` is removed while the watcher runs

- **WHEN** a repository is loaded, and the whole `openspec/` directory is then removed
- **THEN** the file producer's next reload reports an empty change set with **no** problem of
  its own — `changes::from_files`'s already-landed rule for a missing `openspec/` directory,
  unmodified by this change: an absent directory means "not an OpenSpec repository", not a
  read failure, and carries no entry on `ChangeSet::problems`
- **AND** the loop keeps drawing regardless: a `drain` returning `Err(WatchError)` is recorded
  on `refresh.problems` instead — the pane's actual signal that something is wrong — rendered
  as the list region's leading `!`-marked row, and never becomes a `LoopError`, so no frame is
  skipped. That clause is verified **deterministically**, by a `ScriptedFs` whose `drain`
  returns `Err`, in `live-updates`'s "A watcher error is recorded once and the loop continues"
  — **not** by opening a real watcher on a removed tree and draining it for some elapsed
  period, which would be a negative assertion bounded by wall-clock time and therefore the
  very shape this change forbids
- **AND** if the watcher itself is left reporting nothing further, the dashboard is stale
  rather than broken, and `r` still forces a full re-read
