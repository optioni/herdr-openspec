## MODIFIED Requirements

### Requirement: A pointer-motion event does not trigger a draw

`ui::driver::run_loop` SHALL NOT draw, SHALL NOT call `Dashboard::sync_detail`, and SHALL
NOT call `Dashboard::normalise_scroll` on the iteration following an `Event::Mouse` whose
kind is `MouseEventKind::Moved` or `MouseEventKind::Drag(_)`, and SHALL NOT count such an
iteration in `LoopSummary::frames`. It SHALL carry the previous drawn frame's `area`
forward, so a click arriving after any number of motion events still resolves against the
frame that is on screen.

This rule exists because mapping the event to `Action::Ignore` cannot prevent the draw: the
draw happens at the top of the iteration, before the event is read. Crossterm's
`EnableMouseCapture` writes `?1003h` — any-event tracking — so a terminal reports every
pointer move whether the pane wants it or not, and without this rule moving a pointer across
a Herdr split would re-render the whole detail document once per motion event.

The rule SHALL be confined to pointer motion. An ignored **key** SHALL still redraw, exactly
as `An ignored key redraws and keeps waiting` requires; a wheel event, a button press, a
button release, a resize, a focus change, a paste, and a timeout SHALL all still draw.

#### Scenario: Pointer motion does not cost a frame

- **WHEN** `run_loop` is driven at 120x40 with a scripted source yielding twenty
  `MouseEventKind::Moved` events at varying coordinates, then `q`
- **THEN** `LoopSummary::frames` is `1` — the frame drawn before the first event was read —
  and `LoopSummary::polls` is `21`
- **AND** the same run with twenty `MouseEventKind::Drag(MouseButton::Left)` events reports
  the same counts
- **AND** the same run with twenty `Char('z')` presses — an ignored key — reports `frames`
  `21`, so the exemption is scoped to pointer motion and did not become a general
  ignore-means-no-draw rule

#### Scenario: A click after motion still resolves against the drawn frame

- **WHEN** `run_loop` is driven at 120x40 with five `Moved` events and then a left press on
  the second change row, then `q`
- **THEN** the press selects that row, exactly as it does with no motion events before it
- **AND** the frame count is `2`: one before the first event, one after the press


**`text-selection` splits the two kinds this requirement has treated as one.**
`MouseEventKind::Moved` — the pointer crossing the frame with no button held — SHALL
continue to cost no frame, for exactly the reason given above. `MouseEventKind::Drag(_)` —
motion **with a button held** — SHALL draw, because the selection's focus moved and the
highlight is what tells the reader what they are selecting. A drag that did not redraw would
render the feature invisible while it was being used.

The exemption SHALL therefore be keyed on the event kind rather than on "any motion", and a
drag outside the selectable region SHALL still cost no frame: it resolves to
`Action::Ignore`, and an ignored action draws nothing on any path.

#### Scenario: A held-button drag draws and a free pointer motion does not

- **WHEN** the loop is driven with a `MouseEventKind::Moved` event over the detail content
  area, and separately with a `MouseEventKind::Drag(MouseButton::Left)` over the same point
  while a selection is in progress
- **THEN** the `Moved` event produces no draw
- **AND** the `Drag` event produces exactly one draw
- **AND** a `Drag` over `Zone::ListRow`, which resolves to `Action::Ignore`, produces no draw

## REMOVED Requirements

### Requirement: `Dashboard` carries fifteen fields, none defaulted and none elided

**Reason**: `text-selection` adds a sixteenth field, `selection: Option<Selection>`, and
this requirement's name carries the count. The requirement is not withdrawn — it is
re-stated under its new name in the ADDED block below. An OpenSpec requirement header is a
merge key, so a renamed header must be withdrawn under the old name and introduced under
the new one rather than edited in place; `help-overlay` made the same rename from fourteen
to fifteen, and `list-sections` from thirteen to fourteen.

**Migration**: None for a reader, for a `config.toml`, or for the plugin manifest. In the
crate, every `Dashboard` construction site names one further field; there is no `Default`
to absorb it, which is the point.

## ADDED Requirements

### Requirement: `Dashboard` carries sixteen fields, none defaulted and none elided

`ui::app::Dashboard` SHALL carry exactly **sixteen** fields: `repo: Option<PathBuf>` — the
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
launch tier's state defined by `agent-launch`; `file_mode: bool`, `degraded-states`'
addition; `help: Help`, the help overlay's layer state defined by `help-overlay`,
`help-overlay`'s one addition to this type; and `selection: Option<Selection>`,
`text-selection`'s one addition.

`selection` is `None` when no span is selected and otherwise carries an anchor, a focus, and
a granularity — armed, word, row, or span. The anchor and focus are each a line index into
`ui::detail::content_lines` and a display column; the granularity is what lets consecutive presses at one
cell arm, then select a word, then select a row without the pane naming a clock, which
`NOBLOCK` forbids under `src/ui/`. It is **one** field
rather than two because the pair is meaningless apart: an anchor with no focus selects
nothing, and every read of either reads both. `Option` bounds it by construction — there is
at most one selection, and clearing it is assigning `None` rather than remembering to reset
two coordinates.

It is a `Dashboard` field and a **sibling** of `detail` rather than an eighth `Detail` field,
for the reason `help` is a sibling of `filter`: `Detail` is reloaded wholesale by
`sync_detail` on a tab switch, a selection change, or an adopted refresh, and a selection
that lived inside it would be silently discarded by a reload rather than deliberately
cleared by one. The clearing is a rule `text-selection` states, not an accident of where the
field sits. It carries plain data — no trait, no handle, no thread — so the state value stays
`Clone`, `PartialEq`, and constructible in a test, and it joins `NODEFAULT-UI`'s scanned sets
on exactly `Filter`, `Refresh`, and `Launch`'s terms.

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

`ui::app::Help` is `help-overlay`'s addition and SHALL carry exactly **two** fields:
`open: bool` — whether the overlay is drawn — and `scroll: usize` — the index of the first
inventory row visible in its band, a user-controlled position on exactly `detail.scroll`'s
terms and not derived geometry, clamped on every draw by
`ui::layout::scroll_offset`. It is a `Dashboard` field and a **sibling** of `filter` rather
than a third `Route` variant, for the reason `list-filtering` already established for
`filter.active`: the overlay is a layer over whichever route is current, and closing it must
return the reader to that route, which a `Route::Help` variant would have to remember
separately. It carries plain data — no trait, no handle, no thread — so the state value stays
`Clone`, `PartialEq`, and constructible in a test.

`ui::app::Filter` SHALL carry exactly two fields: `query: String` and `active: bool`.
`ui::app::Detail` SHALL carry exactly **seven** fields: `sections: Vec<ArtifactSection>`,
`scroll: usize`, `tab: usize`, `problems: Vec<String>`, `loaded: Option<(PathBuf, usize)>`,
`expanded: BTreeSet<usize>`, and `drawn_width: Option<u16>`. This sentence read "**five**" and
named a `source: String` until `help-overlay`; `foldable-spec-sections` replaced `source` with
`sections` and added `expanded` and `drawn_width` (`src/ui/app.rs:184-206`) without carrying the
count back here, and `pane-chrome`'s own delta did not either. Corrected in passing under the
rule Decision 9 states: a known-false sentence inside a block this change must copy anyway is
repaired rather than reproduced. Three of them
are `detail-view`'s: `tab` is the selected artifact's position, `problems` names each
artifact file that could not be read, and `loaded` is the `(change directory, tab)` key whose
content `source` currently holds — the cache key `artifact-content`'s `sync_detail` compares
against, and the reason an unchanged selection re-reads nothing. `tasks-tab` adds **no**
field to any of the three: which grammar a tab renders is read from
`Change::artifacts[detail.tab].tracks_tasks` on every draw, never stored on the dashboard.

`ui::app::Refresh` is `live-refresh`'s addition and SHALL carry exactly **three** fields:
`requested: bool`, `reload: bool`, and `problems: Vec<String>`, defined by `live-updates`.
`Detail` was deliberately left at five when `live-refresh` landed: `reload` could have lived
there, but `Dashboard` gains one field either way and putting it on `Refresh` left `Detail`'s
construction-site count untouched. It has since grown to seven by `foldable-spec-sections`, as
the corrected sentence above records; the argument for where `reload` went is unaffected.

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

None of `Dashboard`, `Filter`, `Detail`, `Refresh`, `Launch`, `Help`, `help::Binding`, and
`help::Group` SHALL implement `Default` —
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
below. `help-overlay` adds **three** to the swept set — `Help` in `src/ui/app.rs`, which SHALL be
named on that run's own `TYPES` list, taking it to `Dashboard Filter Detail Sections Help` and
its `SCAN_MIN` from 206 to 308, and `help::Binding` and
`help::Group`, which need one further parameterisation of the same script with `HOMEFILE` set to
`src/ui/help.rs`. `make gates` runs `scripts/gates/nodefault-ui.sh` **six** times today
(`grep -c nodefault-ui.sh Makefile` = 6, lines 45-50); this change makes it **seven**. The reason is `Binding`'s: it is thirty-one `'static` literals, which is
exactly the shape a `..Default::default()` rest is tempting in, and a field added to it later
would otherwise silently become the empty string at thirty-one sites at once.
`change-model`'s existing gate does not reach any of these thirteen types: that gate is
stated over `Change`, `ChangeSet`, `ArtifactRef`, and `Origin` in `src/changes.rs`, and none of
these is one of those nor there.

`src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/help.rs`, `src/ui/layout.rs`, `src/ui/list.rs`,
`src/ui/markdown.rs`, `src/ui/palette.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and
`src/ui/driver.rs` SHALL name
no filesystem, process, environment, network, or standard-I/O API. Terminal work lives in
`src/ui/terminal.rs`, event reading in `src/ui/event.rs`, and startup loading, the one
artifact-read binding, the mapping read, and the composition root in `src/ui/mod.rs`; the
**ten** files above are the pure side of the render seam. None of `agent-polling`,
`agent-attribution`, and `agent-launch` added one — `src/watch.rs`, `src/refresh.rs`,
`src/agents.rs`, and `src/launch.rs` sit outside `src/ui/` entirely, which is what kept this
set and the `NOCLI-SHELL` set unchanged through all three;
`agent-launch` adds one file to the crate, `src/launch.rs`, and none to `src/ui/`.
Two changes since have added one each. `view-palette` added `src/ui/palette.rs` and said so in
its own spec — `NOIO-VIEW`'s `PURE` list of **nine** files — but did not carry the correction
back into the sentence above, which still read **eight** and omitted the file; that drift is
repaired here rather than left for the next reader to trip over. `help-overlay` adds
`src/ui/help.rs`, the inventory and the overlay's renderer, taking the set to **ten** and
`src/ui/` to **thirteen** `*.rs` files.
A view test that needs a real directory means logic leaked across that seam.

`state::read` is a filesystem call and SHALL be named only in `src/ui/mod.rs` among the files
under `src/ui/`, on exactly `tasks::read`'s terms below: the ten pure files SHALL
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
  `Dashboard`, `Filter`, `Detail`, `Refresh`, `Launch`, `Help`, `Binding`, `Group`, `Agent`,
  `Listed`, `AgentSnapshot`,
  `Attribution`, and `Outcome`, for `impl Default for <name>` — the target path-qualified or
  bare — for a `Default` inside the `#[derive(...)]` immediately preceding `struct <name>`, and
  for a `..` appearing inside a `<name> { … }` literal or pattern, brace-matched from the
  opening `{` to its partner so a multi-line elision rustfmt spread over several lines is seen
- **THEN** there is no match for any of the thirteen
- **AND** the check fails when its positive-control file is absent, and it is paired with a
  positive control asserting that `src/ui/app.rs` **does** contain `struct Dashboard`,
  `struct Filter`, `struct Detail`, `struct Refresh`, `struct Launch`, and `struct Help`,
  anchored on both
  sides so a rename fails the control rather than leaving every leg searching for a name that is
  no longer there
- **AND** the control's file is a **parameter**, defaulting to `src/ui/app.rs`, and the check is
  run a second time with it set to `src/agents.rs` for `Agent`, `Listed`, `AgentSnapshot`, and
  `Attribution`, a **third** time with it set to `src/launch.rs` for `Outcome`, and — this
  change's addition — once more with it set to `src/ui/help.rs` for `Binding` and `Group`, so
  every
  type outside `src/ui/app.rs` is covered by the same executable file rather than by a fork of it
- **AND** the search is judged against a counted minimum of literal or pattern spans that is
  **per type set**, not one number shared across the runs: the `src/ui/app.rs` set, the
  `src/agents.rs` set, the `src/launch.rs` set, and — `degraded-states`' fourth run — the
  `Refresh` set — and `help-overlay`'s own, the `src/ui/help.rs` set covering `Binding` and
  `Group` — each carry their own floor, measured on the tree at this change's base commit
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
  exhaustive pattern naming all **sixteen** fields and no `..`, a companion destructures a
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
  addition — `file_mode` explicitly, and `help`, `help-overlay`'s: adding a field is a compile
  error at each site until it does, which is the whole reason the type carries no `Default`

#### Scenario: The pure view files name no I/O API

- **WHEN** `src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/help.rs`, `src/ui/layout.rs`,
  `src/ui/list.rs`, `src/ui/markdown.rs`, `src/ui/palette.rs`, `src/ui/tasks.rs`,
  `src/ui/view.rs`, and `src/ui/driver.rs` are
  searched for `std::fs`, `std::io`, `std::env`, `std::process`, `std::net`, `File::`,
  `read_to_string`, `tasks::read`, `state::read`, `state::record`, `launch::start`, and
  `Command`
- **THEN** there is no match in any of the ten
- **AND** the searched set was exactly eight after `agent-polling`,
  `agent-attribution`, and `agent-launch`: the watcher lives in `src/watch.rs`, the worker in
  `src/refresh.rs`, the poller in `src/agents.rs`, and the launcher in `src/launch.rs`, all four
  outside `src/ui/`, and both attribution and the launch decision are pure functions there too,
  so the pure set neither grew nor shrank across those three
- **AND** it is ten here: `view-palette` added `src/ui/palette.rs` and `help-overlay` adds
  `src/ui/help.rs`, and the check fails when either is absent from the list rather than
  sweeping nine files and reporting a clean tree
- **AND** `state::read` joins the searched names for `agent-attribution`: the mapping read is a
  filesystem call, it lives in `src/ui/mod.rs` beside `read_artifact`, and a view reaching for
  it directly is the one leak that change made plausible
- **AND** `state::record` and `launch::start` join them for `agent-launch`: the first is the
  crate's only write outside a test and the second spawns its third worker thread, and a view
  reaching for either directly is the leak this change makes plausible
- **AND** the check fails when any of the **ten** files is absent, rather than reporting a
  clean tree — ten, not eight: the historical bullet three above names the set `agent-launch`
  left behind, and this bullet names the set the check sweeps *here*
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
- **AND** it is proven able to fail against a copy carrying `use std::fs;` inside
  `src/ui/help.rs`, which is the file this change adds to the set — a planted control of its
  own in `tests/gate-controls.toml`, on `gate-integrity`'s "executed, not attested" standard,
  rather than an assertion that the existing controls happen to cover the new file

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

#### Scenario: Every field is named at every construction site

- **WHEN** every `*.rs` file under `src/` is searched for a `..` inside a `Dashboard { … }` or
  a `Sections { … }` literal or pattern, brace-matched from the opening `{` to its partner
- **THEN** there is no match, so no construction site elides `file_mode` or `sections`
- **AND** the compile-time companion destructures a `Dashboard` naming all **sixteen** fields
  with no `..`, so a seventeenth breaks the build at that site, and a further companion
  destructures a `Sections` naming its one field
- **AND** `impl Default for Dashboard` and `impl Default for Sections` appear nowhere in the
  crate, derived or hand-written

#### Scenario: `selection` starts empty and is cleared rather than reloaded

- **WHEN** `ui::load` is called over a scratch repository, and separately `run_wired` is
  driven over the same repository
- **THEN** the returned `Dashboard`'s `selection` is `None` in both cases
- **AND** a dashboard holding a selection, stepped with a tab switch, has `selection` `None`
  afterwards and a freshly reloaded `detail`
- **AND** the clearing happened at the site `text-selection` names, not as a side effect of
  `sync_detail` replacing `Detail`, which would leave the rule untested

## MODIFIED Requirements

### Requirement: Key handling is a pure, total function over events

`ui::app::action_for(event: &Event, filtering: bool) -> Action` SHALL map a terminal event
and the current filter mode to one of exactly **twenty-five** actions — `Quit`,
`OpenDetail`, `Back`, `Next`, `Prev`, `SelectNext`, `SelectPrev`, `ScrollDown`, `ScrollUp`,
`Click(Target)`, `SelectTab(usize)`, `NextTab`, `PrevTab`, `FilterStart`,
`FilterPush(char)`, `FilterPop`, `Refresh`, `LaunchApply`, `LaunchContinue`, `LaunchArchive`,
`FocusAgent`, `ToggleSection`, `ToggleHelp`, `Select`, `Ignore` — and SHALL be total: every `Event`
value, including mouse, paste, focus-gained, focus-lost, and resize events, maps to one of
them under either value of `filtering`, and none panics.

`action_for` SHALL map **no key** to the **six** actions `mouse-input` and
`text-selection` add between them — `mouse-input`'s five, and `Select`. `SelectNext`,
`SelectPrev`, `ScrollDown`, `ScrollUp`, and `Click(Target)` exist because a mouse event names
the region or the row it landed on, where a key does not; `ui::driver::mouse_action` is the
only producer of them, and `mouse-input` states what it produces. `Select` is the sixth, and the one that can never gain a key: selecting rendered text is a
pointing gesture, which is `mouse-input`'s single named mouse-only exemption. The first five
are five flat variants
rather than two carrying a direction or a region, for the same reason the four launch
variants are flat: an enumerate-by-hand test must enumerate the same things the exhaustive
`match` does.

The count moved from thirteen to seventeen when `agent-launch` landed, not from nine to
thirteen: `HANDOFF.md`'s Phase 5 constraint 8 read the count off a stale doc comment in
`src/ui/app.rs` that still said "the nine outcomes" after four had been added. It moved to
**eighteen** with `ToggleSection`, `list-sections`' one addition, to
**twenty-three** with `mouse-input`'s five, moved to **twenty-four** with `help-overlay`'s one, `ToggleHelp`,
and moves to **twenty-five** here, with `text-selection`'s one, `Select`.

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

`ToggleHelp` is `help-overlay`'s addition and is route-agnostic in that same stronger sense:
it opens a layer over whichever route is current and leaves `route` alone. It moves no
existing key either — `Char('?')` was `Ignore` outside filter mode and `FilterPush('?')`
inside it, and it stays `FilterPush('?')` inside it. It is matched under **both**
`KeyModifiers::NONE` and `KeyModifiers::SHIFT`, which is the one row in this table with two
accepted modifier values for one meaning: on a US layout `?` is `Shift`+`/` and terminals
disagree about whether the shift modifier is reported beside the shifted character, so
matching only `NONE` would make the key work on some terminals and not others.
`help-overlay` owns what the action then does.

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
| `KeyCode::Char('?')` with no modifiers **or** with `KeyModifiers::SHIFT` | `ToggleHelp` |
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
expect to keep a command meaning, and so is `'?'`, which `help-overlay` states explicitly for
the same reason: a reader filtering for a change whose name contains `?` must be able to type
it, and the help is one `Esc` away.

`action_for` SHALL act only on key events whose `kind` is `KeyEventKind::Press`. A key event
with kind `Repeat` or `Release` SHALL map to `Ignore` under either value of `filtering`, so
a terminal that reports release events does not quit twice, navigate on a release, type a
character twice, refresh twice, or **launch a second agent**.

`Dashboard::apply(&mut self, action: Action)` SHALL:

- set `quit` on `Quit`;
- on `OpenDetail`, clear `filter.active` and change nothing else when `filter.active` is
  set — accepting a filter is not opening a detail — and otherwise set `route` to `Detail`
  and reset `detail.scroll` to `0`;
- on `Back`, dismiss exactly one layer, in this order: **the help overlay when `help.open`
  is set**, resetting `help.scroll` to `0`; else filter mode with its query when
  `filter.active` is set; else a non-empty `filter.query`; else `route` back to `List`,
  resetting `detail.scroll` to `0`; else nothing at all, so a stray `Esc` at the root cannot
  close the pane. The overlay is `help-overlay`'s addition and is the **outermost** layer: it
  is drawn over everything else, so it is the first thing an `Esc` must take away, and a
  reader who opened the help while filtering gets their query back rather than losing it.
  `launch.problems` SHALL NOT be one of the layers: no key dismisses a launch problem, and the
  next launch outcome is what replaces it;
- on `Next` and `Prev`, move and clamp `selected` per `list-selection` when `route` is
  `List`, and move `detail.scroll` by one line per `detail-scroll` when `route` is `Detail`,
  never both; and, at `Route::List` only, reset `detail.tab` and `detail.scroll` to `0`
  exactly when `selected` changed value. `Next` and `Prev` SHALL be exactly a route
  dispatch over the four region-explicit actions below and SHALL hold no arithmetic of
  their own: `Next` does what `SelectNext` does at `Route::List` and what `ScrollDown` does
  at `Route::Detail`, and `Prev` does what `SelectPrev` and `ScrollUp` do on the same terms,
  so a key and a wheel over the same region can never diverge;
- on `SelectNext` and `SelectPrev`, move and clamp `selected` per `list-selection` and reset
  `detail.tab` and `detail.scroll` to `0` exactly when `selected` changed value, **at either
  route** — these name the list region rather than inheriting the route, which is what lets
  the wide layout's two regions scroll independently;
- on `ScrollDown` and `ScrollUp`, move `detail.scroll` by one line per `detail-scroll`, **at
  either route**, changing `selected`, `detail.tab`, and `route` not at all;
- on `Click(target)`, do nothing when `target` is absent from `targets()`; otherwise act per
  `mouse-input` — a `Target::Section(key)` toggles that section and moves the cursor to its
  header exactly as `ToggleSection` does, and a `Target::Change(i)` moves the cursor to that
  row (resetting `detail.tab` and `detail.scroll` to `0` when it moved), or, when the cursor
  is already there and `route` is `List`, sets `route` to `Detail` and resets `detail.scroll`
  to `0`. It SHALL reach no collaborator, spawn no process, touch no filesystem, and read no
  clock;
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
- on `ToggleHelp`, set `help.open` to its negation and reset `help.scroll` to `0`,
  changing nothing else at all — not `changes`, not `selected`, not `route`, not `detail`,
  not `filter`, not `quit`, not `refresh`, not `agents`, not `agent_names`, not `launch`, and
  not `sections` — and reaching no collaborator. `help-overlay` owns what the overlay then
  renders;
- change nothing on `Ignore`.

**The overlay layer takes precedence over every dispatch above, and over every other
capability that owns an action's semantics.** `Dashboard::apply`'s per-action behaviour is
specified in several places — `detail-scroll` owns `ScrollDown`/`ScrollUp` and
`SelectNext`/`SelectPrev`, `artifact-tabs` owns `SelectTab`/`NextTab`/`PrevTab`,
`artifact-folds` and `list-selection` own `ToggleSection`, `live-updates` owns `Refresh`, and
`agent-launch` owns the four launch actions. Several of those are worded "regardless of
`route`", which remains true: the overlay is not a route. This clause binds all of them at
once, so none needs a delta of its own and none is left contradicting the overlay after
archive. A capability added later that owns an action inherits it without being edited. When `help.open` is set,
`apply` SHALL dispatch per `help-overlay`'s table instead: `Quit` quits, `ToggleHelp` and
`Back` close the overlay, `Next`/`ScrollDown` and `Prev`/`ScrollUp` move `help.scroll` by one
line, and **every one of the other seventeen actions changes nothing at all**. The four
launch actions and `Refresh` being inert there is the load-bearing half of the overlay's
read-only claim: a reader who opened the help to find out what `a` does must be able to press
it without spawning an agent. `Quit` is the one action that still acts, because a modal that
traps the reader is a worse failure than one that lets a quit through.

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
  `filter.active` is true, whose `help.open` is **false**, and whose `detail.scroll` is `3`
  is given four consecutive `Back` actions
- **THEN** after the first, `filter.active` is false and `filter.query` is empty, and
  `detail.scroll` is still `3` — dismissing the filter layer is not leaving the route; after
  the second, `route` is `List` and `detail.scroll` is `0`; after the third and fourth,
  nothing has changed and `quit` is still false
- **AND** the same dashboard with `help.open` **true** needs a fifth `Back` to reach the
  same end state: the first closes the overlay and leaves `filter.active` true with the
  query still `add`, and the remaining four behave exactly as the four above
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
- **AND** the mouse event still returns `Ignore` from `action_for` under both modes after
  `mouse-input` lands: the mouse is resolved by `ui::driver::mouse_action`, which takes the
  frame geometry `action_for` never receives, so this mapper stays key-only

#### Scenario: `Space` maps to `ToggleSection` outside filter mode and types inside it

- **WHEN** `action_for` is called with `filtering` false and a Press of `Char(' ')`, then with
  `filtering` true and the same Press, then with `filtering` false and a Press of `Char(' ')`
  carrying `KeyModifiers::CONTROL`, and finally with a `Release` and a `Repeat` of
  `Char(' ')` under both modes
- **THEN** the first returns `ToggleSection`, the second returns `FilterPush(' ')`, and the
  last three under each mode return `Ignore`
- **AND** every other key's mapping is unchanged: the same table of inputs
  `agent-launch` asserted returns exactly the same actions under both modes, so `Space`
  gaining a meaning moved no existing key
- **AND** `KeyCode::Char(' ')` is the only new row in the `filtering` false table, and
  `filtering` true gains no row at all

#### Scenario: `?` maps to `ToggleHelp` outside filter mode and types inside it

- **WHEN** `action_for` is called with `filtering` false and a Press of `Char('?')` with
  `KeyModifiers::NONE`, then the same Press with `KeyModifiers::SHIFT`, then with
  `KeyModifiers::CONTROL`, then with `KeyModifiers::ALT`, and finally a `Release` and a
  `Repeat` of `Char('?')` under both modifier values
- **THEN** the first two return `ToggleHelp` and every one of the rest returns `Ignore`, so a
  terminal reporting releases cannot toggle the overlay twice
- **AND** with `filtering` true, `Char('?')` with `NONE` and with `SHIFT` both return
  `FilterPush('?')`, and neither opens the overlay
- **AND** `Char('/')` with `SHIFT` returns `Ignore` under `filtering` false — it is the filter
  key carrying a stray modifier, not `?`, and the two are distinguished by the `KeyCode` the
  terminal reports rather than by the physical key
- **AND** every other key's mapping is unchanged under both modes: the same table of inputs
  `mouse-input` asserted returns exactly the same actions, so `?` gaining a meaning moved no
  existing key, and `filtering` true gains no row at all

#### Scenario: The overlay layer suppresses every action but seven

- **WHEN** a `Dashboard` at `Route::List` with `agents.reachable` true, six active changes,
  `selected` `2`, `detail.tab` `1`, and `help.open` true is given each of `OpenDetail`,
  `SelectTab(3)`, `NextTab`, `PrevTab`, `FilterStart`, `FilterPush('a')`, `FilterPop`,
  `Refresh`, `LaunchApply`, `LaunchContinue`, `LaunchArchive`, `FocusAgent`, `ToggleSection`,
  `SelectNext`, `SelectPrev`, `Click(Target::Change(0))`, and `Ignore` in turn
- **THEN** the dashboard after all seventeen is equal, field for field, to the one before
  them, but for `refresh.requested` where `needs_archived_refresh()` holds — the blanket rule
  runs after every action and the overlay does not suppress it
- **AND** in particular `launch.pending` is `None`, `launch.problems` is empty, no process was
  spawned, no file was read or written, and no clock was read
- **AND** the same seventeen applied to the identical dashboard with `help.open` **false**
  change it in the ways the bullets above require, so the suppression is the overlay's and not
  the dashboard's

#### Scenario: The overlay's seven live actions act and nothing else moves

- **WHEN** a `Dashboard` at `Route::Detail` with `detail.scroll` `4`, `selected` `1`, and
  `help.open` true is given `Next`, `ScrollDown`, `Prev`, `ScrollUp`, and `Prev`
- **THEN** `help.scroll` is `1`, `2`, `1`, `0`, and `0` after each, saturating at zero rather
  than underflowing
- **AND** `detail.scroll` is `4` and `selected` is `1` after all five, so the overlay's scroll
  is not the detail region's and not the list's
- **AND** a sixth action, `Back`, sets `help.open` false and `help.scroll` `0`, and a seventh,
  `Quit`, sets `quit` — both from inside the overlay


#### Scenario: No key reaches `Select` at either filter mode

- **WHEN** `action_for` is called with every key in the existing table, under `filtering`
  false and again under `filtering` true
- **THEN** none returns `Action::Select`, so the mouse-only exemption is real rather than
  asserted
- **AND** every other key returns exactly the action it returned before this change
