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
- **AND** the **seventeen** other actions' mappings are unchanged: the same table of inputs
  `agent-launch` asserted returns exactly the same actions under both modes, so `Space`
  gaining a meaning moved no existing key
- **AND** `KeyCode::Char(' ')` is the only new row in the `filtering` false table, and
  `filtering` true gains no row at all

### Requirement: The loop draws before it waits and stops when quit is set

`ui::driver::run_loop(terminal, dashboard, events, live, read, tick)` SHALL be generic over
any `ratatui::backend::Backend` and any `ui::event::EventSource`, so tests drive it with a
`TestBackend` and a scripted event source and no terminal exists in the test process. `read`
is the `artifact-content` reader: a `&dyn Fn(&Path) -> Result<String, String>`, so no
filesystem API is named in `src/ui/driver.rs` and tests drive the loop with an in-memory
double.

`live` is `live-refresh`'s addition, extended by `agent-polling` and again by `agent-launch`: a
`ui::driver::Live` carrying `&mut dyn watch::FsEvents`, `&mut dyn refresh::Refresher`,
`&mut dyn agents::AgentPoll`, and `&mut dyn launch::Launcher`, all
of whose every method is non-blocking. It is a struct rather than four further parameters so
the signature stays at **six** arguments and the four collaborators are named as one concept —
cohesion, not a lint. Measured on this crate and toolchain, clippy's `too_many_arguments` fires
at **eight** parameters, not seven, so a seventh would not have tripped it; the crate's single
`#[allow(clippy::too_many_arguments)]`, on `changes::build_change`, is itself vestigial for the
same reason. That correction is recorded here so a later change does not inherit a forcing
constraint that does not exist and choose a worse shape believing it had no option.

`Live` SHALL be constructed only with all four fields named and no `..` rest, so a collaborator
added to the loop fails to compile at every construction site — the compile-time half of the
guarantee whose behavioural half is `agent-launch`'s wiring test. It cannot implement `Default`
at all, since every field is a `&mut dyn` reference, so no source sweep is needed for it; it is
also outside `NODEFAULT-UI`'s reach, whose positive control anchors on `struct <T> {` and cannot
match a type generic over a lifetime.

**Both `request` call sites carry the scope**, and that is `list-sections`' change to this
requirement. Step 2 is the `refresh.requested` path — the startup request, `r`, and the
`needs_archived_refresh()` rule `list-selection` states — and step 3 is the watch-invalidate
path. A watch event that lands while the archived section is open must resolve that section
too; passing `Names` there would silently empty an archive the reader had just opened, on the
next file change.

Each iteration SHALL, in this order:

1. when `dashboard.launch.pending` is `Some`, take it — leaving `None` — and hand it to
   `live.launcher.request`;
2. when `dashboard.refresh.requested` is set, request `Selection::All` **and**
   `dashboard.archived_scope()` of `live.refresher`, and clear the flag;
3. `live.fs.drain()`, and on a non-empty batch request `watch::invalidate(repo, &paths)` **and**
   `dashboard.archived_scope()` of `live.refresher`; on `Err`, the reason replaces
   `dashboard.refresh.problems` wholesale and the loop continues;
4. `live.refresher.take_result()`, and on `Some(_)` adopt the carried `ChangeSet` through
   `Dashboard::adopt`;
5. `live.agents.drain()`, and on `Some(snapshot)` replace `dashboard.agents` with it;
6. `live.launcher.drain()`, and on `Some(outcome)` insert `outcome.named`'s
   `(agent, change)` pair into `dashboard.agent_names.names` when it is `Some`, and replace
   `dashboard.launch.problems` wholesale with `outcome.problem`'s zero or one entry;
7. `dashboard.sync_detail(read)`;
8. draw the frame;
9. `dashboard.normalise_scroll(area)`;
10. wait up to
    `watch::poll_timeout(tick, watch::soonest(live.fs.pending_in(), live.agents.pending_in()))`
    for an event.

Step 1 leads because the request it hands over answers a key the reader pressed on the previous
iteration, and every step below it is a background tier's own business. Step 6 follows step 5 so
a launch that has just recorded a mapping is visible to the very next `attribution()` call, in
the frame step 8 draws. `watch::soonest` takes **two** arguments and gains no third: the launcher
has no schedule of its own and no `pending_in`, so it never shortens the loop's wait.

Steps 1 to 6 precede the sync and the draw, so a result, a snapshot, or an outcome taken this
iteration is visible in the frame this iteration draws rather than the next; and every one of
them is non-blocking, so the pane is still painted with the selected artifact's content before
any input is read — not blank on the first frame and filled on the second. After applying an
event's action, the loop SHALL break when `dashboard.quit` is set, without syncing or drawing
again; a `launch.pending` set by the last action before a quit is therefore **not** dispatched,
which is deliberate: a reader who launches and immediately quits has closed the pane, and
starting a process on the way out would be a side effect with nothing left to show it.

Step 2 preceding step 3 is `live-updates`' rule and is restated here rather than contradicted:
this requirement previously listed the drain first, which disagreed with `live-updates` and with
the shipped loop, and `agent-polling` corrects it while adding the agent drain.

`EventSource::next_event(&mut self, timeout: Duration) -> Result<Option<Event>,
EventError>` SHALL return `Ok(None)` for a timeout with no event. A timeout SHALL NOT end
the loop and SHALL NOT be treated as an event.

On success `run_loop` SHALL return `LoopSummary { frames, polls }`, counting draws
performed and `next_event` calls made. `LoopSummary` SHALL gain **no** field for the live
tier: requests taken, results adopted, snapshots drained, and launch outcomes applied are
observed through the doubles' own recorders, which keeps every landed
`LoopSummary { frames, polls }` literal in the suite
unchanged. A draw error SHALL end the loop with `LoopError::Draw` carrying the backend error's
`Display` text; an event-source error SHALL end it with `LoopError::Events`. Neither a **watch**
error, an **unreachable Herdr socket**, nor a **failed launch** SHALL end it or become a
`LoopError`: all three are degraded states, and the pane keeps drawing from files. Neither
`LoopError` SHALL panic, and neither SHALL be retried in a loop that could spin.

`ui::driver::TICK` SHALL be 250 milliseconds and SHALL be what `ui::run` passes. It is now
the *upper bound* on a wait rather than the wait itself: `watch::poll_timeout` shortens it to
whichever of the debounce window and the agent poll is due sooner, so the loop wakes at the
moment either becomes due rather than at the next tick.

#### Scenario: A pending launch request is handed over exactly once

- **WHEN** `run_loop` is driven at 60x20 over a recording launcher double, with a script of a
  Press of `Char('a')`, then two `Ok(None)` timeouts, then a Press of `Char('q')`, against a
  `Dashboard` whose `agents.reachable` is `true` and whose visible list holds `add-auth`
- **THEN** the recording launcher received exactly **one** `Request`, and it is
  `Request::Launch { change: "add-auth", agent: "add-auth", intent: Apply }`
- **AND** `dashboard.launch.pending` is `None` on every iteration after the first that followed
  the press, so a request cannot be handed over twice
- **AND** the launcher received nothing at all on the iterations before the press

#### Scenario: A launch outcome updates the mapping and replaces the problem

- **WHEN** `run_loop` is driven at 60x20 over a scripted launcher whose first `drain` answers
  `Outcome { named: None, problem: Some("split failed") }` and whose second answers
  `Outcome { named: Some(("c-2fa-support", "2fa-support")), problem: None }`
- **THEN** after the first, `dashboard.launch.problems` is exactly `["split failed"]` and
  `agent_names.names` is unchanged
- **AND** after the second, `dashboard.launch.problems` is **empty** — replaced wholesale, so a
  success clears the earlier failure — and `agent_names.names` holds
  `c-2fa-support -> 2fa-support`
- **AND** `changes`, `agents`, and `refresh` are unchanged by both

#### Scenario: A quit on the same event as a launch dispatches nothing

- **WHEN** `run_loop` is driven with a script of a Press of `Char('a')` immediately followed by
  a Press of `Char('q')` on the very next `next_event` call, over a recording launcher
- **THEN** the launcher received exactly **one** request — the one dispatched on the iteration
  between the two presses
- **AND** driving the same script with the `q` press first leaves the launcher with **zero**
  requests, because the loop broke before the next iteration's step 1

#### Scenario: The first frame is on screen before the first event is read

- **WHEN** `run_loop` is driven over a `Terminal<TestBackend>` at 120x20 with an event
  source whose script is **empty**, so its first `next_event` call returns
  `Err(EventError)`, an inert `Live` (`watch::none()`, `refresh::none()`,
  `agents::none()`, and `launch::none()`), and a reader returning `# proposal\n` for every path
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
  tick unchanged, and the launcher contributes nothing because it has no `pending_in` at all
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
- **AND** the recording launcher received **zero** requests, so `Ctrl-C` did not launch

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

The scenario's name is kept verbatim from `agent-polling` because a delta's scenario headers are
its merge key; its subject is unchanged and only the field count moves.

- **WHEN** a compile-time companion in `ui::driver`'s tests destructures a `Live` with an
  exhaustive pattern naming all four fields and no `..` rest
- **THEN** the crate compiles, and adding a fifth field to `Live` breaks the build at that
  companion and at every construction site — `ui::run_wired` and every test that builds one
- **AND** the companion is the discriminating evidence rather than "the crate compiles at all":
  compilation alone would still succeed if a later change gave `Live` a `..` rest at one site
- **AND** `run_loop`'s parameter count is still six, and no
  `#[allow(clippy::too_many_arguments)]` is added by this change — checked as a diff against the
  base commit, not as a tree-wide grep, since `src/changes.rs` already carries one

## REMOVED Requirements

### Requirement: `Dashboard` is a plain state value with no rendering and no I/O

**Reason**: `sections: Sections` is a fourteenth field, so the requirement's opening count,
its swept-type list, and its `The thirteenth field is named at every construction site`
scenario all change together. A MODIFIED block cannot rename that scenario, and leaving it
named for the thirteenth field while it asserts the fourteenth would be exactly the stale
name this repository's own conventions forbid.

**Migration**: None for a reader. In the crate, every `Dashboard { … }` literal and pattern
names one more field, which is a compile error at each site rather than a silent default —
`Sections` implements no `Default` either.

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

### Requirement: `Dashboard` carries fourteen fields, none defaulted and none elided

`ui::app::Dashboard` SHALL carry exactly **fourteen** fields: `repo: Option<PathBuf>` — the
repository root when one was found; `searched_from: PathBuf` — the directory the walk began
at, rendered by `change-rows`' no-repository state; `changes: changes::ChangeSet`;
`route: Route`, an enum of `List` and `Detail`; `quit: bool`, set by the quit action;
`selected: usize`, the index into the **visible targets** defined by `list-selection` —
section headers and visible changes in emission order, which is `list-sections`' change to
what this index addresses rather than to its type; `sections: Sections`, the per-session
collapse state defined by `list-selection`, carrying a `BTreeSet<SectionKey>` of the sections
the reader has folded;
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
  exhaustive pattern naming all **fourteen** fields and no `..`, a companion destructures a
  `Sections` naming its one field and no `..`, a second destructures a `Filter`
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

#### Scenario: The fourteenth field is named at every construction site

- **WHEN** every `*.rs` file under `src/` is searched for a `..` inside a `Dashboard { … }` or
  a `Sections { … }` literal or pattern, brace-matched from the opening `{` to its partner
- **THEN** there is no match, so no construction site elides `file_mode` or `sections`
- **AND** the compile-time companion destructures a `Dashboard` naming all **fourteen** fields
  with no `..`, so a fifteenth breaks the build at that site, and a further companion
  destructures a `Sections` naming its one field
- **AND** `impl Default for Dashboard` and `impl Default for Sections` appear nowhere in the
  crate, derived or hand-written

### Requirement: Startup state is read from files only, with the archive counted and not resolved

`ui::load(start: &Path, config: &Config, state_dir: Option<&Path>, archived: ArchivedScope)
-> Dashboard` SHALL call `resolve::find_repo` on `start` and then, when a root was found,
`changes::from_files(root, archived)`. It SHALL NOT read `config.archived_count`, which
`list-sections` leaves accepted and inert (`plugin-config`).

The scope is a **parameter rather than a constant**, and the composition root SHALL choose it
from whether a worker will exist to resolve the archive later: `ArchivedScope::Names` when the
binary probe resolved an `openspec` binary, and `ArchivedScope::Full` when it did not.
`refresh::start` returns the inert refresher unless it has both a repository and a CLI, so in
**file mode** there is no worker at all — a `Space` on the archived header would set
`refresh.requested`, the loop would hand it to a refresher that records nothing, and the
section would stay open and empty for the rest of the session. Resolving the archive once at
startup is what keeps file mode from failing closed; `design.md` → Decision 13 records the
alternative, a file-only worker, and why it is deferred.
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
loop's step **2** turns into the startup request, so the startup path and the `r` key share
one mechanism and are tested once. `ui::run` — not `load` — is what starts the watcher and the
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

#### Scenario: File mode opens the archive with no binary present

- **WHEN** `ui::run_wired` is driven at the wiring tier over a scratch repository whose
  archive holds four dated directories, with the binary probe resolving **nothing** — no
  configured `openspec_bin`, nothing on the injected `PATH`, no nvm tree, and an
  `npm prefix -g` hook returning nothing — and an event source that presses `Space` on the
  archived header and then `q`
- **THEN** the final dashboard's `changes.archived` holds all four changes with their schemas
  and task counts resolved, and `archived_total` is 4
- **AND** the rendered list at 120x20 and at 60x20 holds `  v archived (4)` followed by the
  four rows, so the archive is reachable with no `openspec` binary installed
- **AND** `file_mode` is true and the header carries its `file mode` badge, so the pane is in
  the mode this scenario is about rather than accidentally resolving a binary
- **AND** the same run with the probe resolving a scratch `openspec` program leaves
  `changes.archived` empty until the worker answers, so `Full` at startup is the file-mode
  branch and not the general one

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
