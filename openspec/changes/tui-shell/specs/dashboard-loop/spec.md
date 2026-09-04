## ADDED Requirements

### Requirement: `Dashboard` is a plain state value with no rendering and no I/O

`ui::app::Dashboard` SHALL carry exactly five fields: `repo: Option<PathBuf>` — the
repository root when one was found; `searched_from: PathBuf` — the directory the walk began
at, kept for the empty state `list-view` will render; `changes: changes::ChangeSet`;
`route: Route`, an enum of `List` and `Detail`; and `quit: bool`, set by the quit action.

`Dashboard` SHALL carry no width, no layout mode, no column count, no terminal handle, and
no frame. It SHALL NOT implement `Default` — neither derived nor hand-written, anywhere in
the crate — and every construction and every destructuring SHALL name every field, with no
`..` rest, so a field added later fails to compile at each site rather than defaulting
silently. `change-model`'s existing gate does not reach this type: that gate is stated over
`Change`, `ChangeSet`, `ArtifactRef`, and `Origin` in `src/changes.rs`, and `Dashboard` is
neither of those nor there.

`src/ui/app.rs`, `src/ui/layout.rs`, `src/ui/view.rs`, and `src/ui/driver.rs` SHALL name no
filesystem, process, environment, network, or standard-I/O API. Terminal work lives in
`src/ui/terminal.rs` and startup loading in `src/ui/mod.rs`; the four files above are the
pure side of the render seam.

#### Scenario: `Dashboard` has no `Default` and no site elides a field

- **WHEN** every `*.rs` file under `src/` is searched for `impl Default for Dashboard`, for
  a `Default` inside the `#[derive(...)]` immediately preceding `struct Dashboard`, and for
  a `..` appearing inside a `Dashboard { … }` literal or pattern
- **THEN** there is no match
- **AND** the check fails when `src/ui/app.rs` is absent, and it is paired with a positive
  control asserting that `src/ui/app.rs` **does** contain the text `struct Dashboard`, so a
  search that matched nothing because it searched nothing fails instead of passing
- **AND** the check is proven able to fail: run against a copy of `src/` carrying
  `impl Default for Dashboard { … }`, and again against a copy carrying
  `let Dashboard { quit, .. } = d;`, it reports each violation
- **AND** a compile-time companion exists: a test destructures a `Dashboard` with an
  exhaustive pattern naming all five fields and no `..`, so adding a sixth field breaks the
  build at that site rather than passing a source grep that never saw it

#### Scenario: The pure view files name no I/O API

- **WHEN** `src/ui/app.rs`, `src/ui/layout.rs`, `src/ui/view.rs`, and `src/ui/driver.rs`
  are searched for `std::fs`, `std::io`, `std::env`, `std::process`, `std::net`,
  `File::`, `read_to_string`, and `Command`
- **THEN** there is no match in any of the four
- **AND** the check fails when any of the four files is absent, rather than reporting a
  clean tree
- **AND** it is paired with a positive control asserting that `src/ui/terminal.rs` **does**
  name `std::io`, so a search that matched nothing because it searched nothing fails
  instead of passing

#### Scenario: The shell never names the CLI seam

- **WHEN** every `*.rs` file under `src/ui/` is searched for `from_cli`, `OpenspecCli`,
  `HerdrCli`, `CliChanges`, and `npm_prefix`
- **THEN** there is no match: the dashboard shell reaches OpenSpec state only through
  `changes::from_files`, so it is complete with no `openspec` binary installed
- **AND** it is paired with a positive control asserting that `src/changes.rs` **does**
  name `OpenspecCli`, so the search is proven to be capable of matching
- **AND** the check fails when `src/ui/` holds no `*.rs` file at all

### Requirement: Key handling is a pure, total function over events

`ui::app::action_for(event: &Event) -> Action` SHALL map a terminal event to one of exactly
four actions — `Quit`, `OpenDetail`, `BackToList`, `Ignore` — and SHALL be total: every
`Event` value, including mouse, paste, focus-gained, focus-lost, and resize events, maps to
one of them, and none panics.

The mapping SHALL be:

| Input | Action |
|---|---|
| `KeyCode::Char('q')` with no modifiers | `Quit` |
| `KeyCode::Char('c')` with `KeyModifiers::CONTROL` | `Quit` |
| `KeyCode::Enter` with no modifiers | `OpenDetail` |
| `KeyCode::Esc` with no modifiers | `BackToList` |
| anything else, including `Char('Q')` and `Char('q')` with a modifier | `Ignore` |

`action_for` SHALL act only on key events whose `kind` is `KeyEventKind::Press`. A key
event with kind `Repeat` or `Release` SHALL map to `Ignore`, so a terminal that reports
release events does not quit twice or navigate on the release of a key already handled on
its press.

`Dashboard::apply(&mut self, action: Action)` SHALL set `quit` on `Quit`, set `route` to
`Detail` on `OpenDetail`, set `route` to `List` on `BackToList`, and change nothing on
`Ignore`. `BackToList` while already on `List` SHALL be a no-op and SHALL NOT quit, so a
stray `Esc` at the root cannot close the pane.

#### Scenario: Both quit keys quit and neither near-miss does

- **WHEN** `action_for` is called with a Press of `Char('q')` with no modifiers, a Press of
  `Char('c')` with `CONTROL`, a Press of `Char('Q')` with `SHIFT`, a Press of `Char('q')`
  with `CONTROL`, and a Press of `Char('c')` with no modifiers
- **THEN** the first two return `Quit` and the last three return `Ignore`

#### Scenario: A released quit key does not quit

- **WHEN** `action_for` is called with `Char('q')` carrying kind `Release`, then with
  `Char('q')` carrying kind `Repeat`, then with `Char('q')` carrying kind `Press`
- **THEN** the first two return `Ignore` and the third returns `Quit`

#### Scenario: Enter and Esc move between the two routes

- **WHEN** a `Dashboard` at `Route::List` is given the actions for a Press of `Enter`, then
  a Press of `Esc`, then a second Press of `Esc`
- **THEN** its route is `Detail`, then `List`, then still `List`
- **AND** `quit` is false after all three, so `Esc` at the root does not close the pane

#### Scenario: Non-key events are ignored without panicking

- **WHEN** `action_for` is called with `Event::Resize(60, 20)`, `Event::FocusGained`,
  `Event::FocusLost`, `Event::Paste("q".to_string())`, and a mouse event
- **THEN** each returns `Ignore`
- **AND** in particular a paste whose text is the single character `q` does not quit, so
  pasted content cannot close the pane

### Requirement: The loop draws before it waits and stops when quit is set

`ui::driver::run_loop(terminal, dashboard, events, tick)` SHALL be generic over any
`ratatui::backend::Backend` and any `ui::event::EventSource`, so tests drive it with a
`TestBackend` and a scripted event source and no terminal exists in the test process.

Each iteration SHALL draw the frame **first** and then wait up to `tick` for an event, so
the pane is painted before any input is read. After applying an event's action, the loop
SHALL break when `dashboard.quit` is set, without drawing again.

`EventSource::next_event(&mut self, timeout: Duration) -> Result<Option<Event>,
EventError>` SHALL return `Ok(None)` for a timeout with no event. A timeout SHALL NOT end
the loop and SHALL NOT be treated as an event.

On success `run_loop` SHALL return `LoopSummary { frames, polls }`, counting draws
performed and `next_event` calls made. A draw error SHALL end the loop with
`LoopError::Draw` carrying the backend error's `Display` text; an event-source error SHALL
end it with `LoopError::Events`. Neither SHALL panic, and neither SHALL be retried in a
loop that could spin.

`ui::driver::TICK` SHALL be 250 milliseconds and SHALL be what `ui::run` passes, so a later
change adding periodic work has a wake-up already in place.

#### Scenario: The first frame is on screen before the first event is read

- **WHEN** `run_loop` is driven over a `Terminal<TestBackend>` at 120x20 with an event
  source whose script is **empty**, so its first `next_event` call returns
  `Err(EventError)`
- **THEN** `run_loop` returns `Err(LoopError::Events)`
- **AND** the backend's buffer nevertheless spells `OpenSpec` at row 0 column 0 and holds
  `┌` at row 1 column 0 and at row 1 column 40, so a complete frame was drawn before the
  failing wait — a loop that waited first would leave the buffer blank

#### Scenario: Timeouts are not events and do not end the loop

- **WHEN** `run_loop` is driven at 60x20 with a script of three `Ok(None)` timeouts
  followed by a Press of `Char('q')`
- **THEN** it returns `Ok(LoopSummary { frames: 4, polls: 4 })`
- **AND** the dashboard's `quit` is true
- **AND** the event source recorded that every `next_event` call was made with the `tick`
  the caller passed, not a hard-coded value

#### Scenario: Ctrl-C ends the loop

- **WHEN** `run_loop` is driven at 60x20 with a single Press of `Char('c')` carrying
  `KeyModifiers::CONTROL`
- **THEN** it returns `Ok(LoopSummary { frames: 1, polls: 1 })` and the dashboard's `quit`
  is true

#### Scenario: An ignored key redraws and keeps waiting

- **WHEN** `run_loop` is driven at 60x20 with a Press of `Char('Q')`, then a resize event,
  then a Press of `Char('q')`
- **THEN** it returns `Ok(LoopSummary { frames: 3, polls: 3 })`
- **AND** the dashboard's route is still `List`, so neither input navigated

#### Scenario: A route change is visible in the next frame

- **WHEN** `run_loop` is driven at 60x20 with a Press of `Enter` followed by a Press of
  `Char('q')`
- **THEN** it returns `Ok(LoopSummary { frames: 2, polls: 2 })`
- **AND** the final buffer's row 1 spells `Detail` starting at column 1 and the string
  `Changes` appears nowhere, so the second frame reflected the route the first event set

#### Scenario: A backend draw failure ends the loop rather than spinning

- **WHEN** `run_loop` is driven over a backend whose `draw` returns an error, with an event
  source whose script would supply a `q` press
- **THEN** it returns `Err(LoopError::Draw)` carrying the backend error's text
- **AND** the event source recorded **zero** `next_event` calls, proving the loop stopped
  at the failed draw rather than continuing past it

### Requirement: Startup state is read from files only

`ui::load(start: &Path, config: &Config) -> Dashboard` SHALL call `resolve::find_repo` on
`start` and then, when a root was found, `changes::from_files(root, config.archived_count)`.
It SHALL make no CLI call, spawn no process, and consult no `openspec` binary, so the
dashboard opens with a complete change list on a machine where `openspec` is not installed.

When `find_repo` reports `NotFound`, `load` SHALL produce a `Dashboard` whose `repo` is
`None`, whose `searched_from` is the directory the search reported, and whose `changes` is
`changes::empty_set()` — a total constructor in `src/changes.rs` naming every field of
`ChangeSet` explicitly, so the crate's existing no-`Default` gate covers it.

`load` SHALL always return a `Dashboard`, never a `Result`, and SHALL never panic. Its
route SHALL start at `Route::List` and its `quit` flag at false.

#### Scenario: A scratch repository is loaded from disk with no binary present

- **WHEN** a scratch tree holding `openspec/changes/alpha/proposal.md` and
  `openspec/changes/alpha/tasks.md` with two checked and one unchecked task is loaded by
  `ui::load` from a subdirectory two levels below the root, with `archived_count` 5
- **THEN** the returned `Dashboard`'s `repo` is the canonicalized scratch root
- **AND** its `changes.active` holds exactly one change named `alpha` whose progress is
  2 of 3
- **AND** its `route` is `Route::List` and its `quit` is false
- **AND** no `openspec` binary was consulted: `load` takes no `OpenspecCli` argument, and
  the source check above proves `src/ui/` names none

#### Scenario: No repository above the starting directory

- **WHEN** `ui::load` is called with a fresh scratch directory, after the test has walked
  that directory's ancestors and **asserted** that none of them holds an `openspec`
  directory — a measured precondition, failing with a message naming the offending
  ancestor rather than an assumed one, matching how `resolve`'s own `NotFound` test
  guards the same fixture
- **THEN** the returned `Dashboard`'s `repo` is `None`
- **AND** its `searched_from` is the directory the search reported
- **AND** its `changes.active`, `changes.archived`, and `changes.problems` are all empty

#### Scenario: The configured archived count is passed through

- **WHEN** a scratch repository holding seven directories under `openspec/changes/archive/`,
  each named `YYYY-MM-DD-<name>` with seven distinct dates, is loaded by `ui::load` with a
  `Config` whose `archived_count` is 3
- **THEN** `changes.archived` holds exactly 3 entries, the three most recent by date
- **AND** loading the same tree with `archived_count` 7 yields 7, so the value is read from
  the `Config` rather than being a literal inside `load`

#### Scenario: Loading writes nothing

- **WHEN** a full recursive snapshot of the scratch repository — every entry's path, bytes,
  and modification time — is taken immediately before `ui::load` and again immediately
  after
- **THEN** the two snapshots are equal, so opening the dashboard modified nothing under
  `openspec/`
