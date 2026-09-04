## MODIFIED Requirements

### Requirement: `Dashboard` is a plain state value with no rendering and no I/O

`ui::app::Dashboard` SHALL carry exactly seven fields: `repo: Option<PathBuf>` — the
repository root when one was found; `searched_from: PathBuf` — the directory the walk began
at, rendered by `change-rows`' no-repository state; `changes: changes::ChangeSet`;
`route: Route`, an enum of `List` and `Detail`; `quit: bool`, set by the quit action;
`selected: usize`, the index into the visible list defined by `list-selection`; and
`filter: Filter`, the query and mode defined by `list-filtering`.

`ui::app::Filter` SHALL carry exactly two fields: `query: String` and `active: bool`.

`Dashboard` SHALL carry no width, no layout mode, no column count, no scroll offset, no
interior height, no terminal handle, and no frame. Neither `Dashboard` nor `Filter` SHALL
implement `Default` — neither derived nor hand-written, anywhere in the crate — and every
construction and every destructuring of either SHALL name every field, with no `..` rest,
so a field added later fails to compile at each site rather than defaulting silently.
`change-model`'s existing gate does not reach these types: that gate is stated over
`Change`, `ChangeSet`, `ArtifactRef`, and `Origin` in `src/changes.rs`, and neither
`Dashboard` nor `Filter` is one of those nor there.

`src/ui/app.rs`, `src/ui/layout.rs`, `src/ui/list.rs`, `src/ui/view.rs`, and
`src/ui/driver.rs` SHALL name no filesystem, process, environment, network, or standard-I/O
API. Terminal work lives in `src/ui/terminal.rs` and startup loading in `src/ui/mod.rs`;
the five files above are the pure side of the render seam. A view test that needs a real
directory means logic leaked across that seam.

`Change` and `ChangeSet` literals SHALL appear only in `src/changes.rs`, test fixtures
included, so every construction site stays inside the file `change-model`'s gate searches.

#### Scenario: `Dashboard` has no `Default` and no site elides a field

The scenario's name is kept verbatim from `tui-shell` because a delta's scenario headers
are its merge key; its subject now covers `Filter` as well.

- **WHEN** every `*.rs` file under `src/` is searched, for each of the type names
  `Dashboard` and `Filter`, for `impl Default for <name>`, for a `Default` inside the
  `#[derive(...)]` immediately preceding `struct <name>`, and for a `..` appearing inside a
  `<name> { … }` literal or pattern
- **THEN** there is no match for either type
- **AND** the check fails when `src/ui/app.rs` is absent, and it is paired with a positive
  control asserting that `src/ui/app.rs` **does** contain the text `struct Dashboard` and
  the text `struct Filter`, so a search that matched nothing because it searched nothing
  fails instead of passing
- **AND** the check is proven able to fail: run against a copy of `src/` carrying
  `impl Default for Dashboard { … }`, again against a copy carrying
  `let Dashboard { quit, .. } = d;`, and again against a copy carrying
  `impl Default for Filter { … }`, it reports each violation
- **AND** a compile-time companion exists: a test destructures a `Dashboard` with an
  exhaustive pattern naming all seven fields and no `..`, and a second destructures a
  `Filter` naming both, so adding a field breaks the build at that site rather than passing
  a source grep that never saw it

#### Scenario: The pure view files name no I/O API

- **WHEN** `src/ui/app.rs`, `src/ui/layout.rs`, `src/ui/list.rs`, `src/ui/view.rs`, and
  `src/ui/driver.rs` are searched for `std::fs`, `std::io`, `std::env`, `std::process`,
  `std::net`, `File::`, `read_to_string`, and `Command`
- **THEN** there is no match in any of the five
- **AND** the check fails when any of the five files is absent, rather than reporting a
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

#### Scenario: Change literals live only in the gated file

- **WHEN** every `*.rs` file under `src/` other than `src/changes.rs` is searched for a
  `Change {` or `ChangeSet {` literal or pattern, the type name preceded by a
  non-identifier character so `ArtifactChange` and `Vec<&Change>` do not match
- **THEN** there is no match, so the view tests' fixtures are built by a constructor inside
  `src/changes.rs` rather than by literals the existing gate cannot see
- **AND** it is paired with a positive control asserting that `src/changes.rs` **does**
  contain `ChangeSet {`, and it fails when `src/changes.rs` is absent
- **AND** the check is proven able to fail: run against a copy of `src/` carrying a
  `Change {` literal in `src/ui/view.rs`, it reports it

### Requirement: Key handling is a pure, total function over events

`ui::app::action_for(event: &Event, filtering: bool) -> Action` SHALL map a terminal event
and the current filter mode to one of exactly nine actions — `Quit`, `OpenDetail`, `Back`,
`SelectNext`, `SelectPrev`, `FilterStart`, `FilterPush(char)`, `FilterPop`, `Ignore` — and
SHALL be total: every `Event` value, including mouse, paste, focus-gained, focus-lost, and
resize events, maps to one of them under either value of `filtering`, and none panics.

While `filtering` is **false** the mapping SHALL be:

| Input | Action |
|---|---|
| `KeyCode::Char('q')` with no modifiers | `Quit` |
| `KeyCode::Char('c')` with `KeyModifiers::CONTROL` | `Quit` |
| `KeyCode::Char('j')` or `KeyCode::Down` with no modifiers | `SelectNext` |
| `KeyCode::Char('k')` or `KeyCode::Up` with no modifiers | `SelectPrev` |
| `KeyCode::Char('/')` with no modifiers | `FilterStart` |
| `KeyCode::Enter` with no modifiers | `OpenDetail` |
| `KeyCode::Esc` with no modifiers | `Back` |
| anything else, including `Char('Q')` and `Char('q')` with a modifier | `Ignore` |

While `filtering` is **true** the mapping SHALL be the one `list-filtering` states, in
which printable characters type into the query and only `Ctrl-C` quits.

`action_for` SHALL act only on key events whose `kind` is `KeyEventKind::Press`. A key
event with kind `Repeat` or `Release` SHALL map to `Ignore` under either value of
`filtering`, so a terminal that reports release events does not quit twice, navigate on a
release, or type a character twice.

`Dashboard::apply(&mut self, action: Action)` SHALL:

- set `quit` on `Quit`;
- on `OpenDetail`, clear `filter.active` and change nothing else when `filter.active` is
  set — accepting a filter is not opening a detail — and otherwise set `route` to `Detail`;
- on `Back`, dismiss exactly one layer, in this order: filter mode with its query when
  `filter.active` is set; else a non-empty `filter.query`; else `route` back to `List`; else
  nothing at all, so a stray `Esc` at the root cannot close the pane;
- move and clamp `selected` on `SelectNext` and `SelectPrev`, per `list-selection`;
- set `filter.active` and `route: List` on `FilterStart`, push on `FilterPush`, pop on
  `FilterPop`, clamping `selected` after each;
- change nothing on `Ignore`.

`apply` SHALL never panic and SHALL never leave `selected` addressing a change that is not
visible.

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

#### Scenario: Enter and Esc move between the two routes

- **WHEN** a `Dashboard` at `Route::List` with an empty, inactive filter is given the
  actions for a Press of `Enter`, then a Press of `Esc`, then a second Press of `Esc`
- **THEN** its route is `Detail`, then `List`, then still `List`
- **AND** `quit` is false after all three, so `Esc` at the root does not close the pane

#### Scenario: `Esc` dismisses one layer at a time

- **WHEN** a `Dashboard` at `Route::Detail` whose `filter.query` is `add` and whose
  `filter.active` is true is given four consecutive `Back` actions
- **THEN** after the first, `filter.active` is false and `filter.query` is empty; after the
  second, `route` is `List`; after the third and fourth, nothing has changed and `quit` is
  still false
- **AND** a second `Dashboard` at `Route::List` whose `filter.query` is `add` with
  `filter.active` **false** reaches an empty query on its first `Back` and changes nothing
  on its second

#### Scenario: Navigation and filter keys are distinguished from near misses

- **WHEN** `action_for` is called with `filtering` false and Presses of `Char('j')`,
  `Down`, `Char('k')`, `Up`, `Char('/')`, `Char('J')` with `SHIFT`, `Down` with `CONTROL`,
  and `Char('/')` with `CONTROL`
- **THEN** the first five return `SelectNext`, `SelectNext`, `SelectPrev`, `SelectPrev`,
  and `FilterStart`, and the last three return `Ignore`

#### Scenario: Non-key events are ignored without panicking

- **WHEN** `action_for` is called with `Event::Resize(60, 20)`, `Event::FocusGained`,
  `Event::FocusLost`, `Event::Paste("q".to_string())`, and a mouse event, each under
  `filtering` false and again under `filtering` true
- **THEN** each returns `Ignore` under both
- **AND** in particular a paste whose text is the single character `q` neither quits nor
  types into the query, so pasted content cannot close the pane or edit the filter

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
route SHALL start at `Route::List`, its `quit` flag at false, its `selected` at `0`, and
its `filter` with an empty query and `active` false.

#### Scenario: A scratch repository is loaded from disk with no binary present

- **WHEN** a scratch tree holding `openspec/changes/alpha/proposal.md` and
  `openspec/changes/alpha/tasks.md` with two checked and one unchecked task is loaded by
  `ui::load` from a subdirectory two levels below the root, with `archived_count` 5
- **THEN** the returned `Dashboard`'s `repo` is the canonicalized scratch root
- **AND** its `changes.active` holds exactly one change named `alpha` whose progress is
  2 of 3
- **AND** its `route` is `Route::List`, its `quit` is false, its `selected` is 0, and its
  `filter` is an empty, inactive query
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
- **AND** its `selected` is 0 and its `filter` is an empty, inactive query

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
