## MODIFIED Requirements

### Requirement: `Dashboard` is a plain state value with no rendering and no I/O

`ui::app::Dashboard` SHALL carry exactly nine fields: `repo: Option<PathBuf>` — the
repository root when one was found; `searched_from: PathBuf` — the directory the walk began
at, rendered by `change-rows`' no-repository state; `changes: changes::ChangeSet`;
`route: Route`, an enum of `List` and `Detail`; `quit: bool`, set by the quit action;
`selected: usize`, the index into the visible list defined by `list-selection`;
`filter: Filter`, the query and mode defined by `list-filtering`; `detail: Detail`, the
detail region's state defined by `detail-scroll`, `artifact-tabs`, and `artifact-content`;
and `refresh: Refresh`, the live tier's state defined by `live-updates`.

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

`Dashboard` SHALL carry no width, no layout mode, no column count, no interior height, no
terminal handle, and no frame. It **does** carry `detail.scroll` and `detail.tab`, and that
is not an exception to the rule: both are user-controlled positions, the counterparts of
`selected`, while the offset actually drawn is still derived on every draw by
`layout::scroll_offset` against the current content area's height. `detail.loaded` is not
geometry either: it is a record of what was read, not of how it was laid out. `refresh` is
not geometry: `requested` and `reload` are one-shot flags the loop consumes, and `problems` is
text. No *derived geometry* is stored.

`Dashboard` SHALL carry no watcher, no worker handle, no channel, and no `Instant`. The live
tier's collaborators reach the loop through `ui::driver::Live`, never through the state value,
so `Dashboard` stays `Clone`, `PartialEq`, and constructible in a test with no thread and no
filesystem.

None of `Dashboard`, `Filter`, `Detail`, and `Refresh` SHALL implement `Default` — neither
derived nor hand-written, anywhere in the crate — and every construction and every
destructuring of any of them SHALL name every field, with no `..` rest, so a field added later
fails to compile at each site rather than defaulting silently. `change-model`'s existing gate
does not reach these types: that gate is stated over `Change`, `ChangeSet`, `ArtifactRef`, and
`Origin` in `src/changes.rs`, and none of these four is one of those nor there.

`src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/layout.rs`, `src/ui/list.rs`,
`src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` SHALL name
no filesystem, process, environment, network, or standard-I/O API. Terminal work lives in
`src/ui/terminal.rs`, event reading in `src/ui/event.rs`, and startup loading and the one
artifact-read binding in `src/ui/mod.rs`; the **eight** files above are the pure side of the
render seam, and `live-refresh` adds no ninth — `src/watch.rs` and `src/refresh.rs` sit
outside `src/ui/` entirely, which is what keeps this set and the `NOCLI-SHELL` set unchanged.
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
literal of their own. The same holds for `src/watch.rs` and `src/refresh.rs`, which the same
tree-wide search now covers at a file count of **21**.

#### Scenario: `Dashboard` has no `Default` and no site elides a field

The scenario's name is kept verbatim from `tui-shell` because a delta's scenario headers are
its merge key; its subject is unchanged and only the type list and the field count move.

- **WHEN** every `*.rs` file under `src/` is searched, for each of the type names
  `Dashboard`, `Filter`, `Detail`, and `Refresh`, for `impl Default for <name>` — the target
  path-qualified or bare — for a `Default` inside the `#[derive(...)]` immediately preceding
  `struct <name>`, and for a `..` appearing inside a `<name> { … }` literal or pattern,
  brace-matched from the opening `{` to its partner so a multi-line elision rustfmt spread
  over several lines is seen
- **THEN** there is no match for any of the four
- **AND** the check fails when `src/ui/app.rs` is absent, and it is paired with a positive
  control asserting that `src/ui/app.rs` **does** contain `struct Dashboard`, `struct Filter`,
  `struct Detail`, and `struct Refresh`, anchored on both sides so a rename fails the control
  rather than leaving every leg searching for a name that is no longer there
- **AND** the search is judged against a counted minimum of literal or pattern spans, raised
  to **90** for this change and measured at **107** on the tree before it, so a broken pattern
  that scanned nothing fails rather than reporting a clean tree
- **AND** the check is proven able to fail: run against a copy of `src/` carrying
  `impl Default for Detail { … }`, again against a copy carrying
  `let Detail { source, .. } = d;`, and again against a copy carrying `#[derive(Default)]`
  immediately above `struct Refresh`, it reports each violation
- **AND** the planted `..` control is written in the **multi-line** form this tree's rustfmt
  output actually produces as well as the single-line form. The reason recorded in
  `markdown-viewer`'s version — "the compile-time companion is what covers it" — is
  **retired**: `detail-view` established that the companion destructures one value and so
  catches a field added to the type, never an elision at some other site, and replaced the
  same-line grep with the brace-matching pass named above. The companion is kept for what it
  genuinely does, below
- **AND** a compile-time companion exists: a test destructures a `Dashboard` with an
  exhaustive pattern naming all **nine** fields and no `..`, a second destructures a `Filter`
  naming both, a third destructures a `Detail` naming all five and no `..`, and a fourth
  destructures a `Refresh` naming all **three** and no `..`, so adding a field breaks the
  build at that site rather than passing a source grep that never saw it

#### Scenario: The pure view files name no I/O API

- **WHEN** `src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/layout.rs`, `src/ui/list.rs`,
  `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` are
  searched for `std::fs`, `std::io`, `std::env`, `std::process`, `std::net`, `File::`,
  `read_to_string`, `tasks::read`, and `Command`
- **THEN** there is no match in any of the eight
- **AND** the searched set is still exactly those eight after `live-refresh`: the watcher lives
  in `src/watch.rs` and the worker in `src/refresh.rs`, both outside `src/ui/`, so the pure set
  neither grows nor shrinks
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
  `ChangeSet` and never a `CliChanges`, so it is complete with no `openspec` binary installed
- **AND** this is the structural proof that the CLI is off the render path: `run_loop` cannot
  call the `openspec` binary because no file it lives in may name the trait that reaches it
- **AND** it is paired with a positive control asserting that `src/changes.rs` **does**
  name `OpenspecCli`, so the search is proven to be capable of matching
- **AND** the check fails when `src/ui/` holds fewer than **eleven** `*.rs` files, the count
  `tasks-tab` left behind and `live-refresh` does not change, so a merged or deleted module is
  a deliberate update to the invocation rather than a silent shrink of the searched set

#### Scenario: Change literals live only in the gated file

- **WHEN** every `*.rs` file under `src/` other than `src/changes.rs` is searched for a
  `Change {` or `ChangeSet {` literal or pattern, the type name preceded by a
  non-identifier character so `ArtifactChange` and `Vec<&Change>` do not match
- **THEN** there is no match, so the tab-bar, header, checklist, watcher, and worker tests'
  fixtures — which need changes carrying real `ArtifactRef` values — are built by constructors
  inside `src/changes.rs` rather than by literals the existing gate cannot see
- **AND** the searched set is now **21** files, `src/watch.rs` and `src/refresh.rs` included,
  and the check fails below that count
- **AND** it is paired with a positive control asserting that `src/changes.rs` **does**
  contain `ChangeSet {`, and it fails when `src/changes.rs` is absent
- **AND** the check is proven able to fail: run against a copy of `src/` carrying a
  `Change {` literal in `src/refresh.rs`, it reports it

#### Scenario: The render path names no channel, thread, lock, or clock

- **WHEN** `src/ui/driver.rs`'s production slice — everything above its first line-anchored
  `#[cfg(test)]` — is searched for `.recv(`, `recv_timeout`, `try_recv`, `.join()` with empty
  parentheses, `JoinHandle`, `thread::spawn`, `thread::sleep`, `mpsc`, `Mutex`, `RwLock`, and
  `Condvar`
- **THEN** there is no match: the loop reaches the watcher and the worker only through the two
  trait objects `ui::driver::Live` carries, and every method of both is non-blocking
- **AND** the `.join()` pattern is written with **empty** parentheses rather than as a bare
  `.join(`, because `Path::join` and `str::join` both take an argument and are ordinary
  correct code, while a `JoinHandle`'s `join` takes none — a bare pattern would be a false red
  the first time the loop built a path
- **AND** every `*.rs` file under `src/ui/`, **tests included**, is searched for
  `Instant::now`, `SystemTime::now`, `.elapsed()`, and the bare path prefix `Instant::`, and
  there is no match. This leg is deliberately whole-file rather than production-only: a test
  that reads the clock is exactly the timing flake this change exists not to reintroduce, and
  the debounce takes `now` as a parameter so no view test has a reason to call it
- **AND** because that sweep is **comment-inclusive**, no doc comment under `src/ui/` may
  spell a clock path either: a comment reading "the crate's one `Instant::now()` lives in
  `watch::RealFsEvents::drain`" fails the check on otherwise-correct code, which is the
  `NOTABSEAM` failure mode this repository has already shipped once. Every doc comment under
  `src/ui/` therefore says "the clock" or "a clock", never `Instant::now()` — the same rule
  `markdown-render` imposes on `src/ui/markdown.rs`'s prose about `ratatui`, stated here so
  it is a documented constraint rather than a surprise
- **AND** a third leg searches **inside the two seam modules**, which the first two never
  reach: `src/watch.rs`'s production slice names no `.recv(`, `recv_timeout`, `.join()`,
  `park_timeout`, or `thread::park`, and `src/refresh.rs`'s production slice names none of
  those **before** its single `thread::spawn`. That is what makes "every method of both traits
  is non-blocking" a check rather than a doc comment: `FsEvents::drain` and
  `Refresher::take_result` are called on every frame, and a `recv_timeout` in either would
  satisfy legs 1 and 2 while delaying every draw
- **AND** it is paired with four positive controls, checked before their sweeps:
  `src/refresh.rs`'s production slice must name `mpsc` and `thread::spawn`; `src/watch.rs`
  must name a clock — or the patterns are broken, or the worker is not a worker, or the
  debounce grew a hidden clock somewhere else; and **both** seam modules' production slices
  must name `try_recv`, or their non-blocking receives are not there at all
- **AND** the check is proven able to fail against a copy carrying `use std::sync::mpsc;` in
  `src/ui/driver.rs`'s production slice, against a copy carrying `Instant::now()` anywhere
  under `src/ui/`, against a copy whose `RealFsEvents::drain` calls `rx.recv_timeout(d)`, and
  against a copy whose `Refresher::take_result` does the same
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
  correct sleeps before this change — `src/cli.rs`'s
  `a_program_that_reads_stdin_returns_rather_than_blocking` and `tests/cli.rs`'s two
  `try_wait` polls, one of which carries the comment recording the flake that produced the
  rule. A blanket rule would have been red on an unmodified tree and the only ways out would
  have been deleting three correct tests or exempting two files
- **AND** the scan is judged against a floor on the number of sleep sites it **found** —
  measured at **three** before this change and **four** after, the fourth being
  `watch::tests::start_on_a_real_directory_yields_the_touched_path`'s poll — so a broken
  pattern that matched nothing fails rather than reporting a clean tree
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
- **AND** the check is proven able to fail against a copy carrying a `#[test]` function in
  `src/ui/layout.rs` that sleeps 200 milliseconds and then asserts

### Requirement: Key handling is a pure, total function over events

`ui::app::action_for(event: &Event, filtering: bool) -> Action` SHALL map a terminal event
and the current filter mode to one of exactly **thirteen** actions — `Quit`, `OpenDetail`,
`Back`, `Next`, `Prev`, `SelectTab(usize)`, `NextTab`, `PrevTab`, `FilterStart`,
`FilterPush(char)`, `FilterPop`, `Refresh`, `Ignore` — and SHALL be total: every `Event`
value, including mouse, paste, focus-gained, focus-lost, and resize events, maps to one of
them under either value of `filtering`, and none panics.

`SelectTab`, `NextTab`, and `PrevTab` are `detail-view`'s additions; `artifact-tabs` states
their keys and their effect. They are route-agnostic in the same sense `Next` and `Prev`
are: the detail region is drawn at both routes above the breakpoint, so a tab press at the
list route is immediately visible. `Refresh` is `live-refresh`'s addition and is
route-agnostic in a stronger sense: it names no region at all.

While `filtering` is **false** the mapping SHALL be:

| Input | Action |
|---|---|
| `KeyCode::Char('q')` with no modifiers | `Quit` |
| `KeyCode::Char('c')` with `KeyModifiers::CONTROL` | `Quit` |
| `KeyCode::Char('j')` or `KeyCode::Down` with no modifiers | `Next` |
| `KeyCode::Char('k')` or `KeyCode::Up` with no modifiers | `Prev` |
| `KeyCode::Char('1')`–`Char('9')` with no modifiers | `SelectTab(digit - 1)` |
| `KeyCode::Char(']')` with no modifiers | `NextTab` |
| `KeyCode::Char('[')` with no modifiers | `PrevTab` |
| `KeyCode::Char('/')` with no modifiers | `FilterStart` |
| `KeyCode::Char('r')` with no modifiers | `Refresh` |
| `KeyCode::Enter` with no modifiers | `OpenDetail` |
| `KeyCode::Esc` with no modifiers | `Back` |
| anything else, including `Char('Q')`, `Char('0')`, `Char('R')`, and `Char('q')` or `Char('r')` with a modifier | `Ignore` |

While `filtering` is **true** the mapping SHALL be the one `list-filtering` states, in which
printable characters type into the query and only `Ctrl-C` quits. `1`–`9`, `[`, `]`, and `r`
are printable characters and are therefore query characters there, with no exception carved
out for any of them.

`action_for` SHALL act only on key events whose `kind` is `KeyEventKind::Press`. A key event
with kind `Repeat` or `Release` SHALL map to `Ignore` under either value of `filtering`, so
a terminal that reports release events does not quit twice, navigate on a release, type a
character twice, or refresh twice.

`Dashboard::apply(&mut self, action: Action)` SHALL:

- set `quit` on `Quit`;
- on `OpenDetail`, clear `filter.active` and change nothing else when `filter.active` is
  set — accepting a filter is not opening a detail — and otherwise set `route` to `Detail`
  and reset `detail.scroll` to `0`;
- on `Back`, dismiss exactly one layer, in this order: filter mode with its query when
  `filter.active` is set; else a non-empty `filter.query`; else `route` back to `List`,
  resetting `detail.scroll` to `0`; else nothing at all, so a stray `Esc` at the root cannot
  close the pane;
- on `Next` and `Prev`, move and clamp `selected` per `list-selection` when `route` is
  `List`, and move `detail.scroll` by one line per `detail-scroll` when `route` is `Detail`,
  never both; and, at `Route::List` only, reset `detail.tab` and `detail.scroll` to `0`
  exactly when `selected` changed value;
- on `SelectTab`, `NextTab`, and `PrevTab`, move `detail.tab` per `artifact-tabs`, resetting
  `detail.scroll` to `0` exactly when `detail.tab` changed value;
- set `filter.active` and `route: List` on `FilterStart`, resetting `detail.scroll` to `0`
  because that too is a route move, push on `FilterPush`, pop on `FilterPop`, clamping
  `selected` after each;
- set `refresh.requested` on `Refresh` and change nothing else at all — not `changes`, not
  `selected`, not `route`, not `detail`, not `filter`, not `quit` — reaching no collaborator
  and starting no work, so `apply` stays a pure function of `&mut self` and its argument;
- change nothing on `Ignore`.

`apply` SHALL never panic, SHALL never leave `selected` addressing a change that is not
visible, and SHALL never leave `detail.scroll` unbounded for more than one frame — the
normalisation `detail-scroll` requires of `ui::driver::run_loop` is what bounds it. It MAY
leave `detail.tab` out of range for the selected change after a filter edit; `sync_detail`
is what restores that invariant, before the next draw rather than after it.

#### Scenario: Both quit keys quit and neither near-miss does

- **WHEN** `action_for` is called with `filtering` false and a Press of `Char('q')` with no
  modifiers, a Press of `Char('c')` with `CONTROL`, a Press of `Char('Q')` with `SHIFT`, a
  Press of `Char('q')` with `CONTROL`, and a Press of `Char('c')` with no modifiers
- **THEN** the first two return `Quit` and the last three return `Ignore`
- **AND** with `filtering` true the same five events return `FilterPush('q')`, `Quit`,
  `FilterPush('Q')`, `Ignore`, and `FilterPush('c')`, so exactly one of them still quits

#### Scenario: A released quit key does not quit

- **WHEN** `action_for` is called with `Char('q')` carrying kind `Release`, then with
  `Char('q')` carrying kind `Repeat`, then with `Char('q')` carrying kind `Press`, each
  under `filtering` false and again under `filtering` true
- **THEN** under `filtering` false the first two return `Ignore` and the third returns
  `Quit`
- **AND** under `filtering` true the first two return `Ignore` and the third returns
  `FilterPush('q')`, so a release cannot type a character either
- **AND** the same holds for `Char('1')`, `Char(']')`, and `Char('r')`: a `Release` or
  `Repeat` of any of them returns `Ignore` under both modes, so a terminal reporting releases
  cannot switch tabs twice or refresh twice

#### Scenario: Enter and Esc move between the two routes

- **WHEN** a `Dashboard` at `Route::List` with an empty, inactive filter and
  `detail.scroll` of `0` is given the actions for a Press of `Enter`, then a Press of `Esc`,
  then a second Press of `Esc`
- **THEN** its route is `Detail`, then `List`, then still `List`
- **AND** `quit` is false after all three, so `Esc` at the root does not close the pane
- **AND** `detail.scroll` is `0` after each, since both route moves reset it
- **AND** `detail.tab` is unchanged by all three, because a route move is not a change move
- **AND** `refresh.requested` is unchanged by all three, because a route move is not a refresh

#### Scenario: `Esc` dismisses one layer at a time

- **WHEN** a `Dashboard` at `Route::Detail` whose `filter.query` is `add`, whose
  `filter.active` is true, and whose `detail.scroll` is `3` is given four consecutive `Back`
  actions
- **THEN** after the first, `filter.active` is false and `filter.query` is empty, and
  `detail.scroll` is still `3` — dismissing the filter layer is not leaving the route; after
  the second, `route` is `List` and `detail.scroll` is `0`; after the third and fourth,
  nothing has changed and `quit` is still false
- **AND** a second `Dashboard` at `Route::List` whose `filter.query` is `add` with
  `filter.active` **false** reaches an empty query on its first `Back` and changes nothing
  on its second

#### Scenario: Navigation and filter keys are distinguished from near misses

- **WHEN** `action_for` is called with `filtering` false and Presses of `Char('j')`,
  `Down`, `Char('k')`, `Up`, `Char('/')`, `Char('J')` with `SHIFT`, `Down` with `CONTROL`,
  and `Char('/')` with `CONTROL`
- **THEN** the first five return `Next`, `Next`, `Prev`, `Prev`, and `FilterStart`, and the
  last three return `Ignore`
- **AND** Presses of `Char('1')`, `Char('9')`, `Char(']')`, and `Char('[')` return
  `SelectTab(0)`, `SelectTab(8)`, `NextTab`, and `PrevTab`, while `Char('0')`, `Char('{')`,
  and `Char(']')` with `CONTROL` return `Ignore`
- **AND** a Press of `Char('r')` returns `Refresh`, while `Char('R')` with `SHIFT` and
  `Char('r')` with `CONTROL` both return `Ignore`

#### Scenario: Non-key events are ignored without panicking

- **WHEN** `action_for` is called with `Event::Resize(60, 20)`, `Event::FocusGained`,
  `Event::FocusLost`, `Event::Paste("q".to_string())`, `Event::Paste("1".to_string())`,
  `Event::Paste("r".to_string())`, and a mouse event, each under `filtering` false and again
  under `filtering` true
- **THEN** each returns `Ignore` under both
- **AND** in particular a paste whose text is the single character `q` neither quits nor
  types into the query, a paste whose text is `1` does not switch tabs, and a paste whose text
  is `r` does not refresh, so pasted content cannot close the pane, edit the filter, move the
  tab, or start a CLI cycle

### Requirement: The loop draws before it waits and stops when quit is set

`ui::driver::run_loop(terminal, dashboard, events, live, read, tick)` SHALL be generic over
any `ratatui::backend::Backend` and any `ui::event::EventSource`, so tests drive it with a
`TestBackend` and a scripted event source and no terminal exists in the test process. `read`
is the `artifact-content` reader: a `&dyn Fn(&Path) -> Result<String, String>`, so no
filesystem API is named in `src/ui/driver.rs` and tests drive the loop with an in-memory
double.

`live` is `live-refresh`'s addition: a `ui::driver::Live` carrying `&mut dyn
watch::FsEvents` and `&mut dyn refresh::Refresher`, both of whose every method is
non-blocking. It is a struct rather than two further parameters so the signature stays at six
arguments and the two collaborators are named as one concept.

Each iteration SHALL, in this order:

1. `live.fs.drain()`, and on a non-empty batch request `watch::invalidate(repo, &paths)` of
   `live.refresher`; on `Err`, push the reason onto `dashboard.refresh.problems` once and
   continue;
2. `live.refresher.take_result()`, and on `Some(_)` adopt the carried `ChangeSet` through
   `Dashboard::adopt`;
3. when `dashboard.refresh.requested` is set, request `Selection::All` and clear the flag;
4. `dashboard.sync_detail(read)`;
5. draw the frame;
6. `dashboard.normalise_scroll(area)`;
7. wait up to `watch::poll_timeout(tick, live.fs.pending_in())` for an event.

Steps 1 to 3 precede the sync and the draw, so a result taken this iteration is visible in the
frame this iteration draws rather than the next; and every one of them is non-blocking, so the
pane is still painted with the selected artifact's content before any input is read — not
blank on the first frame and filled on the second. After applying an event's action, the loop
SHALL break when `dashboard.quit` is set, without syncing or drawing again.

`EventSource::next_event(&mut self, timeout: Duration) -> Result<Option<Event>,
EventError>` SHALL return `Ok(None)` for a timeout with no event. A timeout SHALL NOT end
the loop and SHALL NOT be treated as an event.

On success `run_loop` SHALL return `LoopSummary { frames, polls }`, counting draws
performed and `next_event` calls made. `LoopSummary` SHALL gain **no** field for the live
tier: requests taken and results adopted are observed through the doubles' own recorders,
which keeps every landed `LoopSummary { frames, polls }` literal in the suite unchanged. A
draw error SHALL end the loop with `LoopError::Draw` carrying the backend error's `Display`
text; an event-source error SHALL end it with `LoopError::Events`. A **watch** error SHALL
NOT end it and SHALL NOT become a `LoopError`: a watcher that fails is a degraded state, and
the pane keeps drawing from files. Neither `LoopError` SHALL panic, and neither SHALL be
retried in a loop that could spin.

`ui::driver::TICK` SHALL be 250 milliseconds and SHALL be what `ui::run` passes. It is now
the *upper bound* on a wait rather than the wait itself: `watch::poll_timeout` shortens it to
the debounce window's remaining time whenever one is pending, so the loop wakes at the moment
a batch becomes due rather than at the next tick.

#### Scenario: The first frame is on screen before the first event is read

- **WHEN** `run_loop` is driven over a `Terminal<TestBackend>` at 120x20 with an event
  source whose script is **empty**, so its first `next_event` call returns
  `Err(EventError)`, an inert `Live` (`watch::none()` and `refresh::none()`), and a reader
  returning `# proposal\n` for every path
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
  the caller passed, not a hard-coded value — an inert `FsEvents` returns `None` from
  `pending_in`, so `poll_timeout` returns the tick unchanged
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

### Requirement: Startup state is read from files only

`ui::load(start: &Path, config: &Config) -> Dashboard` SHALL call `resolve::find_repo` on
`start` and then, when a root was found, `changes::from_files(root, config.archived_count)`.
It SHALL make no CLI call, spawn no process, start no thread, start no watcher, and consult
no `openspec` binary, so the dashboard opens with a complete change list on a machine where
`openspec` is not installed. It SHALL read no artifact file either: `load` produces a
`Dashboard` whose `detail` is empty in every field, and `sync_detail` — driven by the loop,
with the injected reader — is what fills it. That keeps `load`'s cost proportional to the
change list rather than to the total size of every artifact in the repository.

`load` SHALL set `refresh.requested` to **true**, `refresh.reload` to false, and
`refresh.problems` to empty. Setting the flag is not a CLI call: it is a state value the
loop's step 3 turns into the startup request, so the startup path and the `r` key share one
mechanism and are tested once. `ui::run` — not `load` — is what starts the watcher and the
worker, and it is where a watcher that would not start contributes its problem string.

When `find_repo` reports `NotFound`, `load` SHALL produce a `Dashboard` whose `repo` is
`None`, whose `searched_from` is the directory the search reported, and whose `changes` is
`changes::empty_set()` — a total constructor in `src/changes.rs` naming every field of
`ChangeSet` explicitly, so the crate's existing no-`Default` gate covers it. Its
`refresh.requested` SHALL still be true: the request is harmless, because `ui::run` passes
`refresh::none()` when there is no repository and the inert refresher records nothing.

`load` SHALL always return a `Dashboard`, never a `Result`, and SHALL never panic. Its
route SHALL start at `Route::List`, its `quit` flag at false, its `selected` at `0`, its
`filter` with an empty query and `active` false, and its `detail` empty in all five fields.

#### Scenario: A scratch repository is loaded from disk with no binary present

- **WHEN** a scratch tree holding `openspec/changes/alpha/proposal.md` and
  `openspec/changes/alpha/tasks.md` with two checked and one unchecked task is loaded by
  `ui::load` from a subdirectory two levels below the root, with `archived_count` 5
- **THEN** the returned `Dashboard`'s `repo` is the canonicalized scratch root
- **AND** its `changes.active` holds exactly one change named `alpha` whose progress is
  2 of 3
- **AND** its `route` is `Route::List`, its `quit` is false, its `selected` is 0, and its
  `filter` is an empty, inactive query
- **AND** its `refresh` is `{ requested: true, reload: false, problems: [] }`
- **AND** no `openspec` binary was consulted and no thread was started: `load` takes no
  `OpenspecCli` argument and no `Refresher`, and the source checks above prove `src/ui/`
  names neither the CLI seam nor a thread

#### Scenario: No repository above the starting directory

- **WHEN** `ui::load` is called with a fresh scratch directory, after the test has walked
  that directory's ancestors and **asserted** that none of them holds an `openspec`
  directory — a measured precondition, failing with a message naming the offending
  ancestor rather than an assumed one, matching how `resolve`'s own `NotFound` test
  guards the same fixture
- **THEN** the returned `Dashboard`'s `repo` is `None`
- **AND** its `searched_from` is the directory the search reported
- **AND** its `changes.active`, `changes.archived`, and `changes.problems` are all empty
- **AND** its `selected` is 0 and its `filter` is an empty, inactive query
- **AND** its `refresh.problems` is empty: no watcher was started, so none could fail

#### Scenario: The configured archived count is passed through

- **WHEN** a scratch repository holding seven directories under `openspec/changes/archive/`,
  each named `YYYY-MM-DD-<name>` with seven distinct dates, is loaded by `ui::load` with a
  `Config` whose `archived_count` is 3
- **THEN** `changes.archived` holds exactly 3 entries, the three most recent by date
- **AND** loading the same tree with `archived_count` 7 yields 7, so the value is read from
  the `Config` rather than being a literal inside `load`

#### Scenario: Loading writes nothing

- **WHEN** a scratch repository holding one change with a `proposal.md` and a `tasks.md` is
  snapshotted, `ui::load` is called over it, and it is snapshotted again
- **THEN** the two snapshots are identical — every path, its mode, and its bytes
- **AND** the same holds after `Dashboard::sync_detail` is driven over that dashboard with
  the real `ui::read_artifact` binding, so resolving and reading an artifact's content leaves
  the tree byte-identical too
- **AND** the equivalent claim for the watcher is made by
  `watch::tests::a_started_watcher_writes_nothing`, **not** here: `ui::load` starts no
  watcher, and the Test Boundaries rule that no `ui::` test may open one is absolute rather
  than carrying an exemption for this test
