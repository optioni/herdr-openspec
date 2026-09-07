## MODIFIED Requirements

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
`watch::start(watch_root: &Path) -> (Box<dyn FsEvents>, Vec<String>)`, which SHALL watch
`watch_root` recursively and SHALL NOT return a `Result`: a watcher that will not start is a
degraded state, not a failure to open the pane.

**The composition root SHALL pass `<repo>/openspec`, never the repository root.**
`SPEC.md` → Refresh specifies "one recursive watch on `openspec/`"; passing the repository
root instead makes the watch recursive over `target/`, `.git/`, `node_modules/`, and every
other directory in the tree. The cost is not theoretical:

- every batch of purely-outside touches classifies to `Selection::Only` of the empty set,
  and the loop's step 3 still issues a refresh for it, which spawns `openspec list --json`
  before the selection is ever consulted — so an ordinary `cargo build` in the watched
  repository starts a Node process every debounce window for the whole build, plus a full
  `changes::from_files` directory re-walk each time;
- on Linux, a recursive `inotify` watch consumes one descriptor per directory, so a large
  `target/` can exhaust `max_user_watches` and the watch fails outright — a degraded pane
  caused entirely by watching directories the plugin never reads.

`watch::start` SHALL NOT perform the join itself. It watches exactly the path it is given, so
the module stays a watcher rather than acquiring knowledge of the repository layout, and its
problem string keeps naming the path actually watched.

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

#### Scenario: The composition root watches `openspec/`, not the repository root

- **WHEN** `ui::start_collaborators` is driven over a scratch repository containing both
  `openspec/changes/alpha/` and a sibling `target/` directory, with the watcher construction
  captured by a recording double standing in for `watch::start`
- **THEN** the path it was given is exactly `<repo>/openspec`, asserted as an equality
  against `repo.join("openspec")` rather than as a `starts_with`
- **AND** it is **not** the repository root, so a build artefact written under `target/`
  cannot reach the debounce at all

#### Scenario: A write outside `openspec/` produces no batch

- **WHEN** a real watcher is started on `<scratch>/openspec`, a file is written at
  `<scratch>/target/x.o`, and `drain` is polled for a bounded window
- **THEN** no poll returns `Ok(Some(_))` for that write
- **AND** a subsequent write to `<scratch>/openspec/changes/alpha/tasks.md` **does** reach a
  batch within the same deadline-polled loop, so the negative half is bounded by a positive
  control rather than by elapsed time alone

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
  as the list region's leading `!`-marked row **below** the startup problems `live-updates`
  now holds on their own field, and never becomes a `LoopError`, so no frame is
  skipped. That clause is verified **deterministically**, by a `ScriptedFs` whose `drain`
  returns `Err`, in `live-updates`'s "A watcher error is recorded once and the loop continues"
  — **not** by opening a real watcher on a removed tree and draining it for some elapsed
  period, which would be a negative assertion bounded by wall-clock time and therefore the
  very shape this change forbids
- **AND** if the watcher itself is left reporting nothing further, the dashboard is stale
  rather than broken, and `r` still forces a full re-read

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
path is `Outside`.

`Only(empty)` SHALL mean **nothing to do**, and the loop SHALL issue no refresh request for
it (`live-updates` → "The loop drives the live tier without ever waiting on it", step 3).
This reverses the previous reading, and the reason is that the previous reading's
justification no longer holds: it said the worker "still re-reads files and still runs
`openspec list --json`, which is how a change appearing or vanishing is noticed" — but a
change appearing or vanishing classifies as `Repository`, never as `Outside`, so an
all-`Outside` batch is by construction a batch that invalidates nothing. With the watch now
rooted at `<repo>/openspec` such a batch should not arrive at all; the short-circuit is the
second line of defence, and it is what stops a batch of build artefacts from spawning a Node
process per debounce window if one ever does.

`invalidate` and `classify` SHALL remain unchanged in signature and in return value: the
short-circuit is the **caller's**, so both stay pure, total functions asserted on their own
return values, and `Selection::Only(∅)` remains a value the worker handles correctly if it
is ever handed one directly.

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
