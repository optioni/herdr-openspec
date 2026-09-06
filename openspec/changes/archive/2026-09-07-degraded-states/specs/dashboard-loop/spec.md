## MODIFIED Requirements

### Requirement: `Dashboard` is a plain state value with no rendering and no I/O

`ui::app::Dashboard` SHALL carry exactly **thirteen** fields: `repo: Option<PathBuf>` — the
repository root when one was found; `searched_from: PathBuf` — the directory the walk began
at, rendered by `change-rows`' no-repository state; `changes: changes::ChangeSet`;
`route: Route`, an enum of `List` and `Detail`; `quit: bool`, set by the quit action;
`selected: usize`, the index into the visible list defined by `list-selection`;
`filter: Filter`, the query and mode defined by `list-filtering`; `detail: Detail`, the
detail region's state defined by `detail-scroll`, `artifact-tabs`, and `artifact-content`;
`refresh: Refresh`, the live tier's state defined by `live-updates`;
`agents: agents::AgentSnapshot`, the latest agent poll's outcome defined by `agent-poller`;
`agent_names: state::Mapping`, the plugin-local agent-name mapping defined by
`plugin-state` and consumed by `agent-attribution`'s first tier; `launch: Launch`, the
launch tier's state defined by `agent-launch`; and `file_mode: bool`, `degraded-states`'
addition.

`file_mode` is true exactly when the `openspec` binary probe resolved no usable binary, so the
pane's change list is file-sourced for the whole session and no CLI result will ever correct
it. It is a `Dashboard` field rather than a fourth `Refresh` field because it is decided once,
at startup, and never moves: `requested` and `reload` are one-shot flags the loop consumes and
`problems` is replaced by a watcher error on any iteration, while `file_mode` is a fact about
the machine the pane is running on. It is set by `run_wired` from what
`start_collaborators` reports, and by `ui::load` to `false` — `load` consults no binary, so it
cannot know, and the composition root is the one place that does. `responsive-layout` is its
only reader: the dim `file mode` badge in the header. Nothing else branches on it, and in
particular `ui::list` does not — a file-sourced change list is a complete change list, not a
degraded one, and marking its rows would say otherwise.

`agent_names` is `agent-attribution`'s addition. It is read **once**, by `ui::load`, from the
state directory `Startup` carries, and is not re-read per frame: `state::read` is filesystem
I/O and `Dashboard` is constructed outside the render path, which is what keeps every view a
pure function of this value. `agent-launch` is the change that keeps it current in memory as
the launcher records new pairs: a successful `Launcher::drain` outcome inserts its
`(derived agent name, change name)` pair into `agent_names.names` in the loop, so the badge for
a just-launched agent appears without a second file read. Holding `state::Mapping` rather than
its bare `names` map keeps the file's own problems where `plugin-state` put them, available to
`degraded-states` without a second read.

`ui::app::Launch` is `agent-launch`'s addition and SHALL carry exactly **two** fields:
`pending: Option<launch::Request>` — the one-shot request `apply` produced and the loop has not
yet handed to the launcher — and `problems: Vec<String>` — the last outcome's failure or the
last refusal, replaced wholesale and never grown, holding **at most two** entries — a
`state::record` failure and an `agent prompt` failure are the only pair that can co-occur
(`agent-launch`'s repaired row 23; see `specs/agent-launch/spec.md`). It is a **sibling**
of `refresh` for the same reason `agents` is: `refresh.problems` is replaced wholesale by a
watcher error on any iteration, and `ChangeSet::problems` is replaced wholesale by
`Dashboard::adopt` on every refresh, so a launch's answer put in either would vanish before the
reader saw it. `pending` carries plain data — a `launch::Request` names no trait, no handle, and
no thread — so the state value stays `Clone`, `PartialEq`, and constructible in a test.

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
which a standing condition must survive. `reachable` is read for the first time by
`agent-launch`, which offers or withholds the action keys and their footer hints on it.

`agents::Attribution` is `agent-attribution`'s addition and SHALL carry exactly **three** fields:
`badges: BTreeMap<String, agents::AgentStatus>`, `panes: BTreeMap<String, String>`, and
`unattributed: usize` — `panes` being `agent-launch`'s addition, the pane id of the agent whose
status won each change's badge. It is **not** a field on `Dashboard`.
`Dashboard::attribution(&self) -> agents::Attribution` derives it on every call
from `repo`, `changes`, `agents.agents`, and `agent_names`, beside `visible()`, `visible_len()`,
and `selected_change()`, which are derived on every call for the same reason: a badge attached
by index would drift the moment `adopt` reordered the list, and a badge stored at all would be a
second copy of state the dashboard already holds. `g`'s focus target is read from `panes` at the
moment the key is applied, never stored.

`Dashboard` SHALL carry no width, no layout mode, no column count, no interior height, no
terminal handle, and no frame. It **does** carry `detail.scroll` and `detail.tab`, and that
is not an exception to the rule: both are user-controlled positions, the counterparts of
`selected`, while the offset actually drawn is still derived on every draw by
`layout::scroll_offset` against the current content area's height. `detail.loaded` is not
geometry either: it is a record of what was read, not of how it was laid out. `refresh` is
not geometry: `requested` and `reload` are one-shot flags the loop consumes, and `problems` is
text. `agents` is not geometry: it is the most recent answer to a question, replaced wholesale.
`agent_names` is not geometry: it is a file's contents, read once and kept current in memory.
`launch` is not geometry: `pending` is a one-shot request the loop consumes, exactly as
`refresh.requested` is, and `problems` is text. No *derived geometry* is stored, and neither is
the attribution derived from all of it.

`Dashboard` SHALL carry no watcher, no worker handle, no poller, no launcher, no channel, and no
`Instant`. The live tier's four collaborators reach the loop through `ui::driver::Live`, never
through the state value, so `Dashboard` stays `Clone`, `PartialEq`, and constructible in a test
with no thread and no filesystem.

None of `Dashboard`, `Filter`, `Detail`, `Refresh`, and `Launch` SHALL implement `Default` —
neither derived nor hand-written, anywhere in the crate — and every construction and every
destructuring of any of them SHALL name every field, with no `..` rest, so a field added later
fails to compile at each site rather than defaulting silently. The same SHALL hold for
`agents::Agent`, `agents::Listed`, `agents::AgentSnapshot`, `agents::Attribution`, and
`launch::Outcome`, and the check SHALL be the same check run further times with its
positive-control file parameterised to `src/agents.rs` and to `src/launch.rs` rather than
further copies of it on disk: two versions of one check is how a run and a record drift apart.
`state::Mapping` is deliberately **outside** that set: it derives
`Default`, it did so before this change, and `plugin-state`'s `state::read` returns
`Mapping::default()` on four separate absent-input paths, so removing the derive would replace
four total returns with four literals for no gain. `launch::Request`, `launch::Intent`, and
`launch::Decision` are outside it too, for the reason `agents::AgentStatus` already is: they are
enums, the check's positive control is anchored on `struct <T> {`, and an enum cannot join the
swept list without breaking that control. What the check enforces about `Request` is the
enclosing `Launch` literal, which must name `pending` explicitly at every site; what covers the
enums themselves is an exhaustive `match` with no wildcard arm in the compile-time companions
below. `change-model`'s existing gate does not reach any of these ten types: that gate is
stated over `Change`, `ChangeSet`, `ArtifactRef`, and `Origin` in `src/changes.rs`, and none of
these is one of those nor there.

`src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/layout.rs`, `src/ui/list.rs`,
`src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` SHALL name
no filesystem, process, environment, network, or standard-I/O API. Terminal work lives in
`src/ui/terminal.rs`, event reading in `src/ui/event.rs`, and startup loading, the one
artifact-read binding, the mapping read, and the composition root in `src/ui/mod.rs`; the
**eight** files above are the pure side of the render seam, and none of `agent-polling`,
`agent-attribution`, and `agent-launch` adds a ninth — `src/watch.rs`, `src/refresh.rs`,
`src/agents.rs`, and `src/launch.rs` sit outside `src/ui/` entirely, which is what keeps this
set and the `NOCLI-SHELL` set unchanged, at **eleven** `*.rs` files under `src/ui/`.
`agent-launch` adds one file to the crate, `src/launch.rs`, and none to `src/ui/`.
A view test that needs a real directory means logic leaked across that seam.

`state::read` is a filesystem call and SHALL be named only in `src/ui/mod.rs` among the files
under `src/ui/`, on exactly `tasks::read`'s terms below: the eight pure files SHALL
additionally be searched for `state::read`, so the one filesystem call an attribution renderer
would plausibly reach for is caught by the same check rather than by nothing. `agent-launch`
adds two more names to that same search for the same reason: `state::record`, which is the
crate's only write outside a test, and `launch::start`, which is the call that spawns the third
worker thread. `src/ui/driver.rs` names `launch::Launcher`, `launch::Request`, and
`launch::Outcome` — plain data and a non-blocking trait, none of which is an I/O API — and
`src/ui/app.rs` names `launch::decide`, a pure function; neither names `launch::start`, and the
search is what keeps that true.

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
literal of their own. The same holds for `src/watch.rs`, `src/refresh.rs`, `src/agents.rs`, and
`src/launch.rs`, which the same tree-wide search now covers at a file count of **23**.
`agents::attribute` therefore takes the change names as a `&[&str]` slice rather than a
`&[Change]`: it needs nothing else from the type, and the slice keeps `src/agents.rs` free of
any reason to name a `Change` literal, in its tests as well as its production code.
`launch::decide` takes the change name as an `Option<&str>` for the same reason, so
`src/launch.rs` never names the type either.

#### Scenario: `Dashboard` has no `Default` and no site elides a field

The scenario's name is kept verbatim from `tui-shell` because a delta's scenario headers are
its merge key; its subject is unchanged and only the type list and the field count move.

- **WHEN** every `*.rs` file under `src/` is searched, for each of the type names
  `Dashboard`, `Filter`, `Detail`, `Refresh`, `Launch`, `Agent`, `Listed`, `AgentSnapshot`,
  `Attribution`, and `Outcome`, for `impl Default for <name>` — the target path-qualified or
  bare — for a `Default` inside the `#[derive(...)]` immediately preceding `struct <name>`, and
  for a `..` appearing inside a `<name> { … }` literal or pattern, brace-matched from the
  opening `{` to its partner so a multi-line elision rustfmt spread over several lines is seen
- **THEN** there is no match for any of the ten
- **AND** the check fails when its positive-control file is absent, and it is paired with a
  positive control asserting that `src/ui/app.rs` **does** contain `struct Dashboard`,
  `struct Filter`, `struct Detail`, `struct Refresh`, and `struct Launch`, anchored on both
  sides so a rename fails the control rather than leaving every leg searching for a name that is
  no longer there
- **AND** the control's file is a **parameter**, defaulting to `src/ui/app.rs`, and the check is
  run a second time with it set to `src/agents.rs` for `Agent`, `Listed`, `AgentSnapshot`, and
  `Attribution`, and a **third** time with it set to `src/launch.rs` for `Outcome`, so every
  type outside `src/ui/app.rs` is covered by the same executable file rather than by a fork of it
- **AND** the search is judged against a counted minimum of literal or pattern spans that is
  **per type set**, not one number shared across the runs: the `src/ui/app.rs` set, the
  `src/agents.rs` set, the `src/launch.rs` set, and — `degraded-states`' fourth run — the
  `Refresh` set each carry their own floor, measured on the tree at this change's base commit
  and written as that run's own default when the gate becomes a repository file
  (`quality-gates`). Measured at `89cb3b2` by running the extracted script: **126** spans for
  the `src/ui/app.rs` set. Every floor is re-measured in the implementation's first task rather
  than copied, and a single shared floor is explicitly rejected — the `Refresh` set scans far
  fewer spans than the `Dashboard` set, so one number either passes vacuously for one run or
  fails legitimately for the other
- **AND** the check is proven able to fail: run against a copy of `src/` carrying
  `impl Default for Detail { … }`, again against a copy carrying
  `let Detail { source, .. } = d;`, again against a copy carrying `#[derive(Default)]`
  immediately above `struct Refresh`, again against a copy carrying `#[derive(Default)]`
  immediately above `struct Agent`, again against a copy carrying `#[derive(Default)]`
  immediately above `struct Attribution`, and — `agent-launch`'s two further plants — again
  against a copy carrying `#[derive(Default)]` immediately above `struct Launch` and again
  against one carrying it above `struct Outcome`, it reports each violation
- **AND** the planted `..` control is written in the **multi-line** form this tree's rustfmt
  output actually produces as well as the single-line form. The reason recorded in
  `markdown-viewer`'s version — "the compile-time companion is what covers it" — is
  **retired**: `detail-view` established that the companion destructures one value and so
  catches a field added to the type, never an elision at some other site, and replaced the
  same-line grep with the brace-matching pass named above. The companion is kept for what it
  genuinely does, below
- **AND** a compile-time companion exists: a test destructures a `Dashboard` with an
  exhaustive pattern naming all **thirteen** fields and no `..`, a second destructures a `Filter`
  naming both, a third destructures a `Detail` naming all five and no `..`, a fourth
  destructures a `Refresh` naming all **three** and no `..`, a fifth destructures a `Launch`
  naming both and no `..` — `launch::Outcome`'s companion below naming its two fields after
  `agent-launch`'s `problem` becomes `problems` — a sixth destructures an `Agent`
  naming all **eight**, a seventh destructures a `Listed` naming both, an eighth destructures
  an `AgentSnapshot` naming all **three**, a ninth destructures an `Attribution` naming all
  **three**, and a tenth destructures a `launch::Outcome` naming both, so adding a field breaks
  the build at that site rather than passing a source grep that never saw it
- **AND** two further compile-time companions cover the enums the sweep cannot reach: one
  matches a `launch::Intent` exhaustively over `Apply`, `Continue`, `Archive`, and `Focus` with
  no wildcard arm, and one matches a `launch::Request` exhaustively over `Launch` and `Focus`
  with no wildcard arm and no `..` inside either variant's pattern
- **AND** `agents::AgentStatus`, `launch::Intent`, `launch::Request`, and `launch::Decision` are
  deliberately outside the swept type list: the check's positive control is anchored on
  `struct <T> {`, so an enum cannot be added to it without breaking that control, and a
  `Default` on an enum would change nothing because every construction site is a `match` arm or
  a variant name
- **AND** every `Dashboard` literal in the crate names `launch` and — `degraded-states`'
  addition — `file_mode` explicitly: adding a field is a compile error at each site until it
  does, which is the whole reason the type carries no `Default`

#### Scenario: The pure view files name no I/O API

- **WHEN** `src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/layout.rs`, `src/ui/list.rs`,
  `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` are
  searched for `std::fs`, `std::io`, `std::env`, `std::process`, `std::net`, `File::`,
  `read_to_string`, `tasks::read`, `state::read`, `state::record`, `launch::start`, and
  `Command`
- **THEN** there is no match in any of the eight
- **AND** the searched set is still exactly those eight after `agent-polling`,
  `agent-attribution`, and `agent-launch`: the watcher lives in `src/watch.rs`, the worker in
  `src/refresh.rs`, the poller in `src/agents.rs`, and the launcher in `src/launch.rs`, all four
  outside `src/ui/`, and both attribution and the launch decision are pure functions there too,
  so the pure set neither grows nor shrinks
- **AND** `state::read` joins the searched names for `agent-attribution`: the mapping read is a
  filesystem call, it lives in `src/ui/mod.rs` beside `read_artifact`, and a view reaching for
  it directly is the one leak that change made plausible
- **AND** `state::record` and `launch::start` join them for `agent-launch`: the first is the
  crate's only write outside a test and the second spawns its third worker thread, and a view
  reaching for either directly is the leak this change makes plausible
- **AND** the check fails when any of the eight files is absent, rather than reporting a
  clean tree
- **AND** it is paired with a positive control asserting that `src/ui/terminal.rs` **does**
  name `std::io`, so a search that matched nothing because it searched nothing fails instead
  of passing
- **AND** the check is proven able to fail against a copy carrying `use std::fs;` inside
  `src/ui/tasks.rs`, which is the file `tasks-tab` added to the set, and against a copy
  carrying `crate::tasks::read(p)` inside `src/ui/detail.rs`, which is the pattern that change
  added to the search, and against a copy carrying `crate::state::read(dir)` inside
  `src/ui/app.rs`, which is the pattern `agent-attribution` added, and against copies carrying
  `crate::state::record(dir, a, c)` and `crate::launch::start(cli, r, k, s)` inside
  `src/ui/driver.rs`, which are the two this change adds

#### Scenario: The shell never names the CLI seam

- **WHEN** every `*.rs` file under `src/ui/` is searched for `from_cli`, `OpenspecCli`,
  `HerdrCli`, `CliChanges`, and `npm_prefix`
- **THEN** there is no match: the dashboard shell reaches OpenSpec state only through
  `changes::from_files` and through `refresh::RefreshResult`, whose two variants carry a
  `ChangeSet` and never a `CliChanges`, it reaches Herdr's agent list only through
  `agents::AgentPoll`, whose `drain` carries an `AgentSnapshot` and never a `HerdrCli`, and it
  reaches Herdr's launch commands only through `launch::Launcher`, whose `request` takes a
  `launch::Request` and whose `drain` carries a `launch::Outcome` — neither a `HerdrCli` — so it
  is complete with no `openspec` binary installed and no Herdr socket reachable
- **AND** this is the structural proof that both external programs are off the render path:
  `run_loop` cannot call the `openspec` binary or the `herdr` binary because no file it lives
  in may name the traits that reach them
- **AND** `src/ui/mod.rs` composes all three real handles without naming either trait, because
  `cli::worker_cli_from_env`, `cli::agent_cli_via`, and `launch::start` carry the types in their
  own signatures
- **AND** it is paired with a positive control asserting that `src/changes.rs` **does**
  name `OpenspecCli`, so the search is proven to be capable of matching
- **AND** the check fails when `src/ui/` holds fewer than **eleven** `*.rs` files, the count
  `tasks-tab` left behind and which none of `live-refresh`, `agent-polling`,
  `agent-attribution`, and `agent-launch` changes, so a merged or deleted module is a deliberate
  update to the invocation rather than a silent shrink of the searched set

#### Scenario: Change literals live only in the gated file

- **WHEN** every `*.rs` file under `src/` other than `src/changes.rs` is searched for a
  `Change {` or `ChangeSet {` literal or pattern, the type name preceded by a
  non-identifier character so `ArtifactChange` and `Vec<&Change>` do not match
- **THEN** there is no match, so the tab-bar, header, checklist, watcher, worker, poller, and
  launcher tests' fixtures — which need changes carrying real `ArtifactRef` values — are built by
  constructors inside `src/changes.rs` rather than by literals the existing gate cannot see
- **AND** the searched set is now **23** files, `src/watch.rs`, `src/refresh.rs`,
  `src/agents.rs`, and `src/launch.rs` included, and the check fails below that count
- **AND** `src/launch.rs` needs no `Change` at all: `launch::decide` takes the change name as an
  `Option<&str>` and `launch::Request` carries a `String`, so nothing in the launch tier has a
  reason to name the type — the same choice `agents::attribute`'s `&[&str]` made, for the same
  reason
- **AND** it is paired with a positive control asserting that `src/changes.rs` **does**
  contain `ChangeSet {`, and it fails when `src/changes.rs` is absent
- **AND** the check is proven able to fail: run against a copy of `src/` carrying a
  `Change {` literal in `src/launch.rs`, it reports it

#### Scenario: The render path names no channel, thread, lock, or clock

- **WHEN** `src/ui/driver.rs`'s production slice — everything above its first line-anchored
  `#[cfg(test)]` — is searched for `.recv(`, `recv_timeout`, `try_recv`, `.join()` with empty
  parentheses, `JoinHandle`, `thread::spawn`, `thread::sleep`, `mpsc`, `Mutex`, `RwLock`, and
  `Condvar`
- **THEN** there is no match: the loop reaches the watcher, the worker, the poller, and the
  launcher only through the four trait objects `ui::driver::Live` carries, and every method of
  all four is non-blocking
- **AND** the `.join()` pattern is written with **empty** parentheses rather than as a bare
  `.join(`, because `Path::join` and `str::join` both take an argument and are ordinary
  correct code, while a `JoinHandle`'s `join` takes none — a bare pattern would be a false red
  the first time the loop built a path
- **AND** every `*.rs` file under `src/ui/`, **tests included**, is searched for
  `Instant::now`, `SystemTime::now`, `.elapsed()`, and the bare path prefix `Instant::`, and
  there is no match. This leg is deliberately whole-file rather than production-only: a test
  that reads the clock is exactly the timing flake this change exists not to reintroduce, and
  both the debounce and the poller's schedule keep their clocks outside `src/ui/`. The launcher
  reads no clock at all, anywhere, which is why `watch::soonest` still takes two arguments
- **AND** the outer-loop wiring tests under `src/ui/mod.rs` do **not**
  break this leg: the deadline they wait to lives in `testutil::UntilReady` in `src/lib.rs`,
  which is outside the swept directory, and the tests name no clock of their own
- **AND** because that sweep is **comment-inclusive**, no doc comment under `src/ui/` may
  spell a clock path either: a comment reading "the crate's one `Instant::now()` lives in
  `watch::RealFsEvents::drain`" fails the check on otherwise-correct code, which is the
  `NOTABSEAM` failure mode this repository has already shipped once. Every doc comment under
  `src/ui/` therefore says "the clock" or "a clock", never `Instant::now()` — the same rule
  `markdown-render` imposes on `src/ui/markdown.rs`'s prose about `ratatui`, stated here so
  it is a documented constraint rather than a surprise
- **AND** a third leg searches **inside the four seam modules**, which the first two never
  reach: `src/watch.rs`'s production slice names no `.recv(`, `recv_timeout`, `.join()`,
  `park_timeout`, or `thread::park`, and `src/refresh.rs`'s, `src/agents.rs`', and
  `src/launch.rs`'s production
  slices name none of those **before** their single `thread::spawn`. That is what makes "every
  method of all four traits is non-blocking" a check rather than a doc comment:
  `FsEvents::drain`, `Refresher::take_result`, `AgentPoll::drain`, and `Launcher::drain` are
  called on every frame,
  and a `recv_timeout` in any of them would satisfy legs 1 and 2 while delaying every draw. The
  launcher is the sharpest case: `herdr agent start` blocks for up to **thirty seconds**, so a
  `Launcher::request` that ran the calls inline would freeze the pane for that long
- **AND** it is paired with **eight** positive controls, checked before their sweeps:
  `src/refresh.rs`'s, `src/agents.rs`', and `src/launch.rs`'s production slices must each name
  `mpsc` and
  `thread::spawn`; `src/watch.rs` must name a clock — or the patterns are broken, or a worker is
  not a worker, or the debounce grew a hidden clock somewhere else; and **all four** seam
  modules' production slices must name `try_recv`, or their non-blocking receives are not there
  at all
- **AND** Guard D counts the line-anchored `#[cfg(test)]` attributes of **all four** seam
  modules and requires exactly one each, since the slicer truncates at the first and every
  sweep below it is silent; and Guard E requires the render-path method to be declared above the
  spawn in `src/refresh.rs` (`fn take_result`), `src/agents.rs` (`fn drain`), and
  `src/launch.rs` (`fn drain`), each taken
  as the **last** matching line rather than the first, because the first is the trait's abstract
  signature and necessarily precedes every spawn — the exact defect `live-refresh`'s own Change
  Review repaired
- **AND** the check is proven able to fail against a copy carrying `use std::sync::mpsc;` in
  `src/ui/driver.rs`'s production slice, against a copy carrying `Instant::now()` anywhere
  under `src/ui/`, against a copy whose `RealFsEvents::drain` calls `rx.recv_timeout(d)`,
  against a copy whose `Refresher::take_result` does the same, against a copy whose
  `RealAgentPoll::drain` does the same, against a copy whose `RealLauncher::drain` does the
  same, and against a copy that moves `RealLauncher::drain` below `launch::start`
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
  measured at **four** before `agent-polling` and **five** after, and staying at **five** here.
  `agent-launch` adds **no** sleep site, and that is a design constraint rather than an
  observation: the launcher's "the drain immediately after a request answers `None`" claim is
  proved with a **gated fake `HerdrCli`** that blocks on a channel the test releases, and the
  "a later drain answers" half with a `yield_now` deadline poll on `testutil::UntilReady`'s
  terms. A `thread::sleep` there would have been an elapsed-time assertion about a worker
  thread, which is hazard 1 in the one tier most able to reintroduce it. A sixth site is
  therefore a deliberate raise rather than drift, and a broken pattern that matched
  nothing still fails rather than reporting a clean tree
- **AND** the file-count floor moves from **25** to **26**, `src/launch.rs` included, so a scan
  that silently stopped searching the new module fails rather than reporting a clean tree
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
- **AND** the leg still covers `src/ui/mod.rs` after `agent-launch`'s two further outer-loop
  tests land
  there: those tests wait through `testutil::UntilReady`, whose own wait is `yield_now` and
  whose clock lives in `src/lib.rs`, so nothing under `src/ui/` sleeps
- **AND** the check is proven able to fail against a copy carrying a `#[test]` function in
  `src/ui/layout.rs` that sleeps 200 milliseconds and then asserts

#### Scenario: `file_mode` is set by the composition root and by nothing else

- **WHEN** `ui::load` is called over a scratch repository with any `Config` and any
  `state_dir`
- **THEN** the returned `Dashboard`'s `file_mode` is `false`, on every branch including the
  no-repository one: `load` consults no binary and so may not claim one is missing
- **AND** `run_wired` driven over the same repository with an environment in which every probe
  step fails returns a dashboard whose `file_mode` is `true`
- **AND** the same run with a `Config` whose `openspec_bin` names a usable scratch
  `#!/bin/sh` program returns a dashboard whose `file_mode` is `false`, so the flag is the
  probe's answer and not a constant

#### Scenario: The thirteenth field is named at every construction site

- **WHEN** every `*.rs` file under `src/` is searched for a `..` inside a `Dashboard { … }`
  literal or pattern, brace-matched from the opening `{` to its partner
- **THEN** there is no match, so no construction site elides `file_mode`
- **AND** the compile-time companion destructures a `Dashboard` naming all **thirteen** fields
  with no `..`, so a fourteenth breaks the build at that site
- **AND** `impl Default for Dashboard` appears nowhere in the crate, derived or hand-written
