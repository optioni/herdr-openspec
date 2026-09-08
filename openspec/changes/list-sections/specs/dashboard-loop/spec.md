## MODIFIED Requirements

### Requirement: Key handling is a pure, total function over events

`ui::app::action_for(event: &Event, filtering: bool) -> Action` SHALL map a terminal event
and the current filter mode to one of exactly **eighteen** actions — `Quit`, `OpenDetail`,
`Back`, `Next`, `Prev`, `SelectTab(usize)`, `NextTab`, `PrevTab`, `FilterStart`,
`FilterPush(char)`, `FilterPop`, `Refresh`, `LaunchApply`, `LaunchContinue`, `LaunchArchive`,
`FocusAgent`, `ToggleSection`, `Ignore` — and SHALL be total: every `Event`
value, including mouse, paste, focus-gained, focus-lost, and resize events, maps to one of
them under either value of `filtering`, and none panics.

The count moved from thirteen to seventeen when `agent-launch` landed, not from nine to
thirteen: `HANDOFF.md`'s Phase 5 constraint 8 read the count off a stale doc comment in
`src/ui/app.rs` that still said "the nine outcomes" after four had been added. It moves to
**eighteen** here, with `ToggleSection` `list-sections`' one addition.

`SelectTab`, `NextTab`, and `PrevTab` are `detail-view`'s additions; `artifact-tabs` states
their keys and their effect. They are route-agnostic in the same sense `Next` and `Prev`
are: the detail region is drawn at both routes above the breakpoint, so a tab press at the
list route is immediately visible. `Refresh` is `live-refresh`'s addition and is
route-agnostic in a stronger sense: it names no region at all. `LaunchApply`,
`LaunchContinue`, `LaunchArchive`, and `FocusAgent` are `agent-launch`'s additions and are
route-agnostic in that same stronger sense: they act on the **selected** change, which is the
same change at either route. `ToggleSection` is `list-sections`' addition and is
route-agnostic in the same way: it folds the section the cursor is in, which is the same
section at either route, and `list-selection` owns what it does. It moves no existing key —
`Space` was `Ignore` outside filter mode and `FilterPush(' ')` inside it, and it stays
`FilterPush(' ')` inside it.

They are **four flat variants** rather than one variant carrying a payload. A payloaded
`Launch(Intent)` would let `no_action_mutates_changes`' hand-written `variants` array carry one
intent and silently omit the other two, which is precisely the failure that test's own comment
warns against ("an enumerate-by-hand test would silently miss it"); four flat variants make the
exhaustive `match` and the array enumerate the same four things.

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
| `KeyCode::Char('a')` with no modifiers | `LaunchApply` |
| `KeyCode::Char('c')` with **no** modifiers | `LaunchContinue` |
| `KeyCode::Char('s')` with no modifiers | `LaunchArchive` |
| `KeyCode::Char('g')` with no modifiers | `FocusAgent` |
| `KeyCode::Char(' ')` with no modifiers | `ToggleSection` |
| `KeyCode::Enter` with no modifiers | `OpenDetail` |
| `KeyCode::Esc` with no modifiers | `Back` |
| anything else, including `Char('Q')`, `Char('0')`, `Char('R')`, `Char('A')`, `Char('G')`, and `Char('q')`, `Char('r')`, `Char('a')`, `Char('s')`, or `Char('g')` with a modifier | `Ignore` |

`Char('c')` is the one key with two rows. Bare `c` is `LaunchContinue`; `c` with
`KeyModifiers::CONTROL` is `Quit`, and remains so under both values of `filtering`. The two are
distinguished by the modifier alone, which the mapping already matches on, so no key is
overloaded ambiguously and `Ctrl-C` never launches.

`action_for` SHALL NOT take the socket's reachability as a parameter and SHALL NOT consult it.
Whether an action key is *offered* is `agent-launch`'s decision, made in `Dashboard::apply`
against `agents.reachable`; the key-to-action mapping stays a pure function of the event and the
filter mode, so a socket that comes and goes never changes what a key means.

While `filtering` is **true** the mapping SHALL be the one `list-filtering` states, in which
printable characters type into the query and only `Ctrl-C` quits. `1`–`9`, `[`, `]`, `r`, `a`,
`c`, `s`, and `g` are printable characters and are therefore query characters there, with no
exception carved out for any of them — and so is `' '`, which `list-filtering` states
explicitly because a space in a text field is the one printable character a reader might
expect to keep a command meaning.

`action_for` SHALL act only on key events whose `kind` is `KeyEventKind::Press`. A key event
with kind `Repeat` or `Release` SHALL map to `Ignore` under either value of `filtering`, so
a terminal that reports release events does not quit twice, navigate on a release, type a
character twice, refresh twice, or **launch a second agent**.

`Dashboard::apply(&mut self, action: Action)` SHALL:

- set `quit` on `Quit`;
- on `OpenDetail`, clear `filter.active` and change nothing else when `filter.active` is
  set — accepting a filter is not opening a detail — and otherwise set `route` to `Detail`
  and reset `detail.scroll` to `0`;
- on `Back`, dismiss exactly one layer, in this order: filter mode with its query when
  `filter.active` is set; else a non-empty `filter.query`; else `route` back to `List`,
  resetting `detail.scroll` to `0`; else nothing at all, so a stray `Esc` at the root cannot
  close the pane. `launch.problems` SHALL NOT be one of the layers: no key dismisses a launch
  problem, and the next launch outcome is what replaces it;
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
- on `ToggleSection`, fold or unfold exactly one section per `list-selection` — the one the
  cursor addresses, or the one the addressed change belongs to — and move `selected` to that
  section's header. It SHALL change nothing else: not `changes`, not `route`, not `detail`,
  not `filter`, not `quit`, not `agents`, not `agent_names`, and not `launch`, and it SHALL
  reach no collaborator;
- on `LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent`, map the action to the
  corresponding `launch::Intent`, call `launch::decide` with the selected change's name, the
  focus pane `attribution().panes` holds for it, `agents.reachable`, and the `name`s of the
  live agents, and write the result: `Decision::Nothing` changes nothing at all;
  `Decision::Refuse(reason)` replaces `launch.problems` with that one entry and leaves
  `launch.pending` alone; `Decision::Go(request)` sets `launch.pending` to `Some(request)` and
  clears `launch.problems`. **None of the four SHALL change `changes`, `selected`, `route`,
  `detail`, `filter`, `quit`, `refresh`, `agents`, or `agent_names`, and none SHALL reach a
  collaborator, spawn a process, touch the filesystem, or read a clock** — `apply` stays a pure
  function of `&mut self` and its argument, and `run_loop` is what turns `launch.pending` into a
  request to a collaborator that lives outside `src/ui/` entirely;
- change nothing on `Ignore`.

After applying **any** action, `apply` SHALL set `refresh.requested` to true when
`Dashboard::needs_archived_refresh()` holds — `list-sections`' rule, stated in full by
`list-selection`. Writing it once for every action rather than for a named subset is what
keeps a future section-opening key from forgetting it; it is the one place `apply` sets a
flag an action did not name, and it sets no other.

`apply` SHALL never panic, SHALL never leave `selected` addressing a target that is not
visible, and SHALL never leave `detail.scroll` unbounded for more than one frame — the
normalisation `detail-scroll` requires of `ui::driver::run_loop` is what bounds it. It MAY
leave `detail.tab` out of range for the selected change after a filter edit; `sync_detail`
is what restores that invariant, before the next draw rather than after it.

#### Scenario: The four action keys map, and their near misses do not

- **WHEN** `action_for` is called with `filtering` false and Presses of `Char('a')`,
  `Char('c')`, `Char('s')`, `Char('g')`, `Char('c')` with `CONTROL`, `Char('A')` with `SHIFT`,
  `Char('G')` with `SHIFT`, `Char('a')` with `CONTROL`, and `Char('s')` with `ALT`
- **THEN** the first four return `LaunchApply`, `LaunchContinue`, `LaunchArchive`, and
  `FocusAgent`; the fifth returns `Quit`; and the last four return `Ignore`
- **AND** with `filtering` true the same nine return `FilterPush('a')`, `FilterPush('c')`,
  `FilterPush('s')`, `FilterPush('g')`, `Quit`, `FilterPush('A')`, `FilterPush('G')`,
  `Ignore`, and `Ignore` — so `Ctrl-C` is the only one of the nine that still quits and none of
  the four launches while a filter is open
- **AND** a `Release` and a `Repeat` of each of `Char('a')`, `Char('c')`, `Char('s')`, and
  `Char('g')` return `Ignore` under both modes, so a terminal reporting releases cannot launch
  a second agent

#### Scenario: A launch action reaches no collaborator and starts no work

- **WHEN** a `Dashboard` with `agents.reachable` `true` and a selected change `add-auth` is
  given `LaunchApply`
- **THEN** `launch.pending` is
  `Some(Request::Launch { change: "add-auth", agent: "add-auth", intent: Apply })` and
  `launch.problems` is empty
- **AND** `changes`, `selected`, `route`, `detail`, `filter`, `quit`, `refresh`, `agents`, and
  `agent_names` are all unchanged, field for field
- **AND** no process was spawned, no file was read or written, and no clock was read — `apply`
  is a pure function of `&mut self` and its argument, and the dashboard value it produces is
  `Clone` and `PartialEq` as before

#### Scenario: A refused launch records the reason and produces no request

- **WHEN** a `Dashboard` with `agents.reachable` `true`, a selected change `2fa-support`, and a
  live agent named `c-2fa-support` is given `LaunchApply` three times in a row
- **THEN** `launch.pending` is `None` after every one of the three
- **AND** `launch.problems` holds exactly **one** entry after all three, naming
  `c-2fa-support` and `g` — replaced wholesale each time, never grown

#### Scenario: An unreachable socket makes the four keys change nothing

- **WHEN** a `Dashboard` with `agents.reachable` `false` and a selected change is given
  `LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent` in turn
- **THEN** `launch.pending` is `None` and `launch.problems` is empty after all four
- **AND** the whole dashboard is equal, field for field, to the one before the four actions

#### Scenario: Both quit keys quit and neither near-miss does

- **WHEN** `action_for` is called with `filtering` false and a Press of `Char('q')` with no
  modifiers, a Press of `Char('c')` with `CONTROL`, a Press of `Char('Q')` with `SHIFT`, a
  Press of `Char('q')` with `CONTROL`, and a Press of `Char('c')` with no modifiers
- **THEN** the first two return `Quit`, the next two return `Ignore`, and the fifth returns
  `LaunchContinue` — bare `c` is `agent-launch`'s key and was `Ignore` before it
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
- **AND** `launch.pending` and `launch.problems` are unchanged by all three, because `Esc` is
  not a layer that dismisses a launch problem

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
- **AND** a third `Dashboard` carrying one `launch.problems` entry still carries it after four
  consecutive `Back` actions

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
  `Event::Paste("r".to_string())`, `Event::Paste("a".to_string())`, and a mouse event, each
  under `filtering` false and again under `filtering` true
- **THEN** each returns `Ignore` under both
- **AND** in particular a paste whose text is the single character `q` neither quits nor
  types into the query, a paste whose text is `1` does not switch tabs, a paste whose text
  is `r` does not refresh, and a paste whose text is `a` **does not launch an agent**, so
  pasted content cannot close the pane, edit the filter, move the tab, start a CLI cycle, or
  spawn a process

#### Scenario: `Space` maps to `ToggleSection` outside filter mode and types inside it

- **WHEN** `action_for` is called with `filtering` false and a Press of `Char(' ')`, then with
  `filtering` true and the same Press, then with `filtering` false and a Press of `Char(' ')`
  carrying `KeyModifiers::CONTROL`, and finally with a `Release` and a `Repeat` of
  `Char(' ')` under both modes
- **THEN** the first returns `ToggleSection`, the second returns `FilterPush(' ')`, and the
  last three under each mode return `Ignore`
- **AND** the sixteen other actions' mappings are unchanged: the same table of inputs
  `agent-launch` asserted returns exactly the same actions under both modes, so `Space`
  gaining a meaning moved no existing key
- **AND** `KeyCode::Char(' ')` is the only new row in the `filtering` false table, and
  `filtering` true gains no row at all

## REMOVED Requirements

### Requirement: Startup state is read from files only

**Reason**: `ui::load` no longer passes `config.archived_count` to `changes::from_files` — the
key is accepted and inert as of `list-sections` (`plugin-config`) — and it now seeds a fold
state the requirement did not have a field for. Both the requirement's opening sentence and
its `The configured archived count is passed through` scenario assert the behaviour this
change removes, so the requirement is restated under a name that says what startup actually
does rather than edited around its own title.

**Migration**: None for a reader or for a `config.toml`. In the crate, `ui::load` keeps its
signature — `config` is still a parameter, still read for `openspec_bin` and `agent_kind` —
and every caller is unchanged.

## ADDED Requirements

### Requirement: Startup state is read from files only, with the archive counted and not resolved

`ui::load(start: &Path, config: &Config, state_dir: Option<&Path>) -> Dashboard` SHALL call
`resolve::find_repo` on `start` and then, when a root was found,
`changes::from_files(root, ArchivedScope::Names)`. It SHALL NOT read `config.archived_count`,
which `list-sections` leaves accepted and inert (`plugin-config`); the scope comes from the
initial collapse state below, and passing `Names` is what keeps startup's cost proportional
to the active tier however large the archive has grown.
It SHALL make no CLI call, spawn no process, start no thread, start no watcher, and consult
no `openspec` binary, so the dashboard opens with a complete change list on a machine where
`openspec` is not installed. It SHALL read no artifact file either: `load` produces a
`Dashboard` whose `detail` is empty in every field, and `sync_detail` — driven by the loop,
with the injected reader — is what fills it. That keeps `load`'s cost proportional to the
change list rather than to the total size of every artifact in the repository.

`state_dir` is `agent-attribution`'s addition, and the plugin-local mapping is the **one**
further file `load` reads. `load` SHALL set `agent_names` to `state::read(state_dir)` — on
**both** the found and the not-found branch, since Herdr agents exist independently of an
OpenSpec repository — and SHALL treat every unusable input as an ordinary absent one:
`state_dir` of `None`, an absent directory, an absent `agent-names.toml`, and an empty file
all yield `Mapping::default()` with no problem, while a malformed file yields an empty
mapping carrying `state::read`'s own problem string. `load` SHALL NOT resolve the state
directory itself: it arrives as a parameter, so the crate's one `std::env::var` binding stays
where `plugin-config` put it and a test drives `load` against a scratch directory without
touching the process environment.

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
`filter` with an empty query and `active` false, its `sections` with `collapsed` holding
exactly `SectionKey::Archived`, its `detail` empty in all five fields, and
its `agents` the inert `AgentSnapshot` — empty, `reachable` false, no problem — which the
loop's first poll replaces.

`selected` `0` therefore addresses the **active section header** on a repository with at
least one active change, not the first change, which is `list-selection`'s change to what
that index means rather than a change to the value.

#### Scenario: A scratch repository is loaded from disk with no binary present

- **WHEN** a scratch tree holding `openspec/changes/alpha/proposal.md` and
  `openspec/changes/alpha/tasks.md` with two checked and one unchecked task is loaded by
  `ui::load` from a subdirectory two levels below the root, with `archived_count` 5 and
  `state_dir` `None`
- **THEN** the returned `Dashboard`'s `repo` is the canonicalized scratch root
- **AND** its `changes.active` holds exactly one change named `alpha` whose progress is
  2 of 3
- **AND** its `route` is `Route::List`, its `quit` is false, its `selected` is 0, its
  `filter` is an empty, inactive query, and its `sections.collapsed` holds exactly
  `SectionKey::Archived`
- **AND** its `refresh` is `{ requested: true, reload: false, problems: [] }`
- **AND** its `agent_names.names` and `agent_names.problems` are both empty
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
- **AND** its `changes.active`, `changes.archived`, and `changes.problems` are all empty and
  its `changes.archived_total` is `0`
- **AND** its `selected` is 0, its `filter` is an empty, inactive query, and its
  `sections.collapsed` holds exactly `SectionKey::Archived` — the same seed on both branches,
  since a repository found later must not open with a different fold
- **AND** its `refresh.problems` is empty: no watcher was started, so none could fail

#### Scenario: Startup counts the archive without resolving it

- **WHEN** a scratch repository holding seven directories under `openspec/changes/archive/`,
  each named `YYYY-MM-DD-<name>` with seven distinct dates, is loaded by `ui::load` with a
  `Config` whose `archived_count` is 3
- **THEN** `changes.archived` is empty and `changes.archived_total` is 7
- **AND** loading the same tree with `archived_count` 7, and again with 0, yields exactly the
  same `Dashboard`, so the key is inert rather than read
- **AND** `sections.collapsed` holds exactly `SectionKey::Archived`, `targets()` names the
  archived section, and rendering at 120x20 and at 60x20 shows `  > archived (7)` and no
  archived name
- **AND** `needs_archived_refresh()` is false, so a session that never opens the archive never
  requests it

#### Scenario: Loading writes nothing

- **WHEN** a scratch repository holding one change with a `proposal.md` and a `tasks.md` is
  snapshotted, `ui::load` is called over it, and it is snapshotted again
- **THEN** the two snapshots are identical — every path, its mode, and its bytes
- **AND** the same holds after `Dashboard::sync_detail` is driven over that dashboard with
  the real `ui::read_artifact` binding, so resolving and reading an artifact's content leaves
  the tree byte-identical too
- **AND** a scratch **state** directory holding an `agent-names.toml` is snapshotted around
  the same call and is identical too: `load` reads the mapping and never writes it, and
  `agent-launch` is the change that will
- **AND** the equivalent claim for the watcher is made by
  `watch::tests::a_started_watcher_writes_nothing`, **not** here: `ui::load` starts no
  watcher, and the Test Boundaries rule that no `ui::` test may open one is absolute rather
  than carrying an exemption for this test

#### Scenario: `load` reads the agent-name mapping from the directory it was given

- **WHEN** `ui::load` is called over a scratch repository with a `state_dir` naming a second
  scratch directory holding `agent-names.toml` with `[names]` and
  `c-2fa-support = "2fa-support"`
- **THEN** the returned `Dashboard`'s `agent_names.names` holds exactly that one pair and
  `agent_names.problems` is empty
- **AND** calling `load` again with the same repository and `state_dir` `None` returns a
  `Dashboard` whose `agent_names.names` is empty, so the pair came from the directory rather
  than from anywhere else
- **AND** `Dashboard::attribution()` on the first result badges `2fa-support` for an in-scope
  agent named `c-2fa-support`, and on the second badges nothing — the mapping reaching the
  dashboard is observable in the attribution, not only in the field

#### Scenario: An unusable mapping file is an empty mapping with a named problem

- **WHEN** `ui::load` is called with a `state_dir` naming a scratch directory whose
  `agent-names.toml` is not valid TOML
- **THEN** the returned `Dashboard`'s `agent_names.names` is empty and
  `agent_names.problems` holds exactly one entry naming the file
- **AND** `load` still returns a complete `Dashboard`: `repo`, `changes`, `route`,
  `selected`, `filter`, `detail`, `refresh`, and `agents` are all exactly what the same call
  produces with a well-formed mapping, so an unusable mapping degrades the badge tier and
  nothing else
- **AND** nothing renders that problem: `agent_names.problems` reaches no row and no hint in
  this change, and `degraded-states` is the change that owns whether it ever does
