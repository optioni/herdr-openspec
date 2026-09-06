# tasks-checklist Specification

## Purpose
TBD - created by archiving change tasks-tab. Update Purpose after archive.

## Requirements

### Requirement: The tracked-tasks tab renders a checklist, and every other tab does not

`ui::detail::content_lines` SHALL render the selected tab as a checklist — the grammar this
capability specifies — when, and only when, the selected change's `ArtifactRef` at
`detail.tab` carries `tracks_tasks == true`. Every other tab SHALL keep
`ui::markdown::lines` unchanged.

The decision SHALL be made in exactly one place, from `Change::artifacts[detail.tab]`, so
the drawn slice (`ui::view::render`) and the scroll clamp (`Dashboard::normalise_scroll`)
can never disagree about which grammar the tab holds. The decision SHALL NOT be made by
comparing the artifact's `id` to the string `tasks`, and SHALL NOT be made by inspecting a
resolved path's filename: `change-artifacts` and `cli-changes` set `tracks_tasks` from the
schema's `apply.tracks`-then-id-`tasks` rule, and a schema declaring `id: checklist` with
`generates: tasks.md` is the case an id comparison gets wrong.

When no artifact of the selected change carries `tracks_tasks == true` — a schema whose
`apply.tracks` matches nothing and which declares no artifact with id `tasks`, a schema
that failed to load, or a change with no artifacts at all — **no tab** renders the
checklist and every tab renders as markdown. No tab is added to, removed from, reordered
in, or hidden from the tab bar by this capability: `artifact-tabs` owns the bar and this
change adds no code to it.

#### Scenario: The tasks tab shows checkboxes and its siblings show markdown

- **WHEN** a `Dashboard` at `Route::Detail`, whose selected change carries the five `tdd`
  artifacts with `tracks_tasks` set at position 3 (`tasks`) and whose `detail.source` is
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n`, is rendered at 120x20 and at 60x20
  with `detail.tab == 3`
- **THEN** in each buffer the content area holds a progress-bar row, a blank row,
  `## 1. Setup`, `[x] 1.1 first`, and `[ ] 1.2 second`, in that order
- **AND** the same `Dashboard` with `detail.tab == 0` renders the identical source through
  `markdown::lines` instead: the content area's first row reads `## 1. Setup` with no
  progress-bar row above it, and the item rows read `- [x] 1.1 first` and
  `- [ ] 1.2 second` with their source bullets intact
- **AND** the tab bar in row 3 is byte-identical between the two renders, so switching tabs
  changed content and nothing else

#### Scenario: The tab is chosen by `tracks_tasks`, not by its id

- **WHEN** a `Dashboard` whose selected change carries two artifacts, `checklist` at
  position 0 with `tracks_tasks == true` and `tasks` at position 1 with
  `tracks_tasks == false`, and whose `detail.source` is `- [x] done\n`, is rendered at
  120x20 and at 60x20
- **THEN** with `detail.tab == 0` the content area holds the checklist grammar — a
  progress-bar row and a `[x] done` row
- **AND** with `detail.tab == 1` the content area holds `- [x] done` and no progress-bar
  row, even though that artifact's id is `tasks`
- **AND** neither render panics and the tab bar shows both cells in both cases

#### Scenario: A schema naming no tasks artifact leaves every tab as markdown

- **WHEN** a `Dashboard` whose selected change carries three artifacts, none with
  `tracks_tasks == true`, and whose `detail.source` is `- [ ] a\n`, is rendered at 120x20
  and at 60x20 with `detail.tab` at each of `0`, `1`, and `2`
- **THEN** no render shows a progress-bar row and every render shows `- [ ] a` verbatim
- **AND** the tab bar still holds all three cells in every render, so no tab was removed

#### Scenario: A `detail.tab` past the end of the artifact list renders no checklist

- **WHEN** `content_lines` is called at width `78` and at width `58` for a `Detail` whose
  `tab` is `7` over a change carrying two artifacts, the second of which has
  `tracks_tasks == true`, with a source of `- [x] a\n`
- **THEN** neither call panics and both return the markdown rendering, because index `7`
  names no artifact and therefore names no tracked-tasks artifact
- **AND** the same holds for a change carrying no artifacts at all

### Requirement: The checklist's line grammar

For a tracked-tasks tab whose source is `source` and whose change carries `progress`,
`ui::tasks::lines(source, progress, width)` SHALL produce, in order:

1. the progress-bar line — `tasks-progress-bar`'s single line, as one plain-faced segment
   — followed by one blank line, both omitted entirely when the bar renders as the empty
   string at that width;
2. for each `tasks::Group` returned by `tasks::parse(source)`, in document order:
   - when the group carries a `Heading`, one line whose text is that heading's `#` markers
     reproduced from its `level`, a space, and its `text` verbatim, carrying
     `Face { heading: Some(level), .. }` so `ui::view::style_for` bolds it with no new
     `Face`-to-`Style` mapping;
   - one or more lines per `tasks::Item`, as specified below;
   - one blank line after every group but the last.

An item's line SHALL be a prefix followed by its text. The prefix is `item.indent` spaces,
then the three-character glyph `[x]` when `item.checked` and `[ ]` when it is not, then one
space. Every line SHALL carry `Face::plain()` except a heading line.

`item.indent` is `task-parsing`'s own count of the whitespace **characters** preceding the
bullet, not a column count, and this capability reproduces it as that many spaces without
reinterpreting it. The consequence, stated so it is a decision rather than a surprise: a
tab-indented item renders with **one** space of indent, because a tab is one character. That
matches the parse rather than second-guessing it, and re-deriving a column width here would
be a second indentation rule beside the one `task-parsing` already publishes.

The item's text SHALL be word-wrapped to `width - prefix_len` columns, with continuation
lines indented by `prefix_len` spaces so they align under the first line's text — a hanging
indent, the same shape `markdown-render` gives a list item. A single word longer than the
available text column SHALL be hard-split at that column rather than overflowing the
interior or being dropped, so a long path never silently loses its tail.

The indent SHALL be **dropped whole** when the prefix would not leave at least one text
column: `item.indent` spaces first, leaving `[x] ` alone; and when even that does not fit,
the glyph alone truncated by `ui::list::pad_or_truncate_right` at `width`. No line's text
SHALL exceed `width` characters, counted in `char`s.

`width == 0` SHALL return an empty vector, matching `ui::markdown::lines`.

`ui::tasks` SHALL name no `ratatui` type, on exactly the terms `ui::detail` and
`ui::markdown` do not, and SHALL name no filesystem, process, environment, network, or
standard-I/O API. It SHALL call `tasks::parse` and SHALL NOT reimplement it: this
capability renders an existing parse and introduces no second checkbox rule.

#### Scenario: Groups, headings, items, and separators at both mandated widths

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over the source
  `## 1. Setup\n\n- [x] 1.1 first\n- [ ] 1.2 second\n\n## 2. Build\n\n- [ ] 2.1 third\n`
  with `Progress { completed: 1, total: 3 }`
- **THEN** at each width the lines' texts are, in order: the progress bar, an empty line,
  `## 1. Setup`, `[x] 1.1 first`, `[ ] 1.2 second`, an empty line, `## 2. Build`, and
  `[ ] 2.1 third`
- **AND** exactly one blank line separates the two groups and none follows the last
- **AND** the two heading lines carry `Face { heading: Some(2), .. }` and every other line
  carries `Face::plain()`

#### Scenario: A nested item reproduces its own indent

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `- [ ] parent\n  - [x] child\n    - [ ] grandchild\n` with
  `Progress { completed: 1, total: 3 }`
- **THEN** at each width the three item lines read `[ ] parent`, `  [x] child`, and
  `    [ ] grandchild`, so the source's own two- and four-space indents are reproduced
- **AND** the list is flat: no line is dropped, merged, or re-ordered, because
  `tasks::parse` never nests

#### Scenario: A long item wraps with a hanging indent at both widths

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over a single
  unchecked item whose text is 200 characters of space-separated words, with
  `Progress { completed: 0, total: 1 }`
- **THEN** at each width no line's text exceeds that width
- **AND** the first item line begins `[ ] ` and every continuation line begins with exactly
  four spaces, aligning under the first line's text
- **AND** the 58-column call produces strictly more lines than the 78-column call, so the
  width genuinely reaches the wrap

#### Scenario: An unbreakable word is hard-split rather than lost

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over a single
  checked item whose text is one 300-character run with no spaces
- **THEN** at each width the concatenation of the item's lines, with the glyph prefix and
  the hanging indent removed, reproduces the 300 characters exactly
- **AND** no line's text exceeds the width it was called with

#### Scenario: The indent is dropped whole as the width collapses

- **WHEN** `ui::tasks::lines` is called at widths `78`, `58`, `12`, `6`, `5`, `4`, `3`,
  `2`, `1`, and `0` over the source `      - [x] alpha\n` (an indent of six) with
  `Progress { completed: 1, total: 1 }`
- **THEN** no call panics and no returned line's text exceeds its width
- **AND** at `78` and `58` the item line begins with six spaces then `[x] alpha`
- **AND** at a width where the six-space indent leaves no text column, the item line begins
  `[x]` at column zero — the indent was dropped whole rather than partially
- **AND** at `0` the returned vector is empty

#### Scenario: A heading with no items still renders its heading

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `## 1. Empty\n\nsome prose\n\n## 2. Full\n\n- [ ] only\n` with
  `Progress { completed: 0, total: 1 }`
- **THEN** at each width both headings appear, `## 1. Empty` carries no item line beneath
  it, and `## 2. Full` carries `[ ] only`
- **AND** the prose line does not appear, because `tasks::parse` discards it

#### Scenario: A headingless leading group renders without a heading line

- **WHEN** `ui::tasks::lines` is called at width `78` and at width `58` over
  `- [x] loose\n\n## 1. Later\n\n- [ ] grouped\n` with
  `Progress { completed: 1, total: 2 }`
- **THEN** at each width `[x] loose` appears before `## 1. Later` with no heading line
  above it
- **AND** exactly one blank line separates the two groups

### Requirement: A source holding no task lines renders `No tasks yet`

When `tasks::parse(source)` yields no **items** — no group holds one, so
`Tasks::progress().total == 0` — the checklist SHALL render the progress-bar line, its blank
line, and exactly one further line reading `No tasks yet`, and SHALL render **no** heading
line even where the source carries headings.

The trigger is items, not groups, and that distinction is load-bearing rather than pedantic:
the landed `task-groups` capability requires `parse` to emit a group for **every** heading it
recognises, including one holding no items, so a prose file opening with `# Plan` returns one
group and zero items. A trigger stated over groups would never fire for exactly the documents
this state exists for. It does not conflict with "A heading with no items still renders its
heading" above, which governs a document that holds items *somewhere*: there, the empty
group is one group among several and its heading carries information about a section not yet
written; here, there is nothing to be a section of.

When `detail.source` is **empty**, `artifact-content`'s existing rule governs and
`No content yet` SHALL be rendered instead: an artifact file that does not exist is a
different state from one that exists and holds no tasks, and the two SHALL NOT be
conflated. `No tasks yet` and `No content yet` SHALL never both appear for the same tab.

#### Scenario: A prose-only tasks file reads `No tasks yet`

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change's tracked-tasks tab is
  selected, whose `detail.source` is `# Plan\n\nNothing checkable here.\n`, and whose
  change carries `Progress { completed: 0, total: 0 }`, is rendered at 120x20 and at 60x20
- **THEN** in each buffer the content area's first row holds `[-]` and its third row reads
  `No tasks yet`
- **AND** `No content yet` appears in neither buffer
- **AND** `# Plan` appears in neither buffer, even though `tasks::parse` returned one group
  for that heading: the trigger is zero items, and a document with no items renders no
  heading lines

#### Scenario: A missing tasks artifact still reads `No content yet`

- **WHEN** a `Dashboard` at `Route::Detail` whose selected change's tracked-tasks tab is
  selected, whose `progress` is `Progress { completed: 4, total: 9 }` — non-zero, because
  `change-artifacts`' `tasks.md` fallback counted a file the marked artifact's `generates`
  did not resolve to — and whose `detail.source` and `detail.problems` are both empty, is
  rendered at 120x20 and at 60x20
- **THEN** in each buffer the content area's first row reads `No content yet`
- **AND** no progress-bar row and no `No tasks yet` row appears, even though the change's
  `progress` is non-zero and the detail header one row above still shows `[4/9]`
- **AND** this is the one place the tab's content and the change's progress legitimately
  disagree; `SPEC.md`'s degraded-states table carries a row for it

#### Scenario: A read failure on the tasks tab names its reason and renders no checklist

- **WHEN** the same `Dashboard` has `detail.problems ==
  ["/repo/openspec/changes/x/tasks.md: permission denied"]` and an empty `detail.source`
  and is rendered at 120x20 and at 60x20
- **THEN** in each buffer the content area's first row begins
  `! /repo/openspec/changes/x/tasks.md:` and is exactly the interior width — 78 and 58
- **AND** neither `No content yet` nor `No tasks yet` nor a progress-bar row appears, so
  the tab says one thing about itself rather than two

### Requirement: The tasks tab is read-only and writes nothing

No key SHALL toggle, check, uncheck, insert, delete, or reorder a task item, and no code
path reachable from the dashboard SHALL open a file under `openspec/` for writing, create
a directory there, or change a modification time there. Rendering a checkbox is what makes
toggling one look natural; `PRD.md` → Non-goals forbids it because an agent may be editing
`tasks.md` in another pane and a write would race it.

`live-refresh` makes that race concrete rather than hypothetical: the dashboard now holds an
open `notify` watch on `openspec/` while an agent writes to it, so the read-only guarantee is
under more pressure here than anywhere else. It is unchanged in substance — the watcher, the
worker, and every `Refresher` result are reads — and the source-level sweep it is proved by
now covers `src/watch.rs`, `src/refresh.rs`, `src/agents.rs`, and `src/launch.rs` alongside
`src/ui/`. `src/agents.rs` joined the searched set with `agent-polling`'s poller; this
requirement's prose still named only the first two and is corrected here.

**`agent-launch` is where this requirement is under the most pressure it has ever been**, and
the boundary it draws is the load-bearing sentence of that change: `a` / `c` / `s` start a
coding agent whose whole purpose is to edit files inside `openspec/`. The plugin still writes
nothing there. What it does is start a **separate process** in a **separate pane**, and every
byte that process writes it writes as itself, under its own working directory, through its own
tools — the plugin's causal contribution ends at `herdr agent prompt`. The boundary is the
process boundary, and the evidence is mechanical rather than argumentative: the launch tier's
only write is `state::record` into `HERDR_PLUGIN_STATE_DIR`, and the acceptance scenarios take a
byte-identical snapshot of the whole repository tree across a run in which an agent was
launched.

`ui::app::action_for` SHALL map no key to a task-mutating action, and `Dashboard::apply`
SHALL gain no arm that edits a `Change`, an `ArtifactRef`, or a `Progress`. `live-refresh`
added exactly **one** `Action` variant, `Refresh`, whose `apply` arm sets
`refresh.requested` and touches nothing else; `agent-launch` adds exactly **four** —
`LaunchApply`, `LaunchContinue`, `LaunchArchive`, and `FocusAgent` — whose `apply` arms write
only `Dashboard::launch`, a one-shot `Option<launch::Request>` and a `Vec<String>` of at most
one entry. **None of the four reaches a collaborator, spawns a process, opens a file, or reads
a clock**: they are the first four actions whose *eventual* effect is outward, and the whole
reason `apply` can stay pure is that what they produce is **data**. `ui::driver::run_loop` hands
that data to `launch::Launcher`, whose real implementation lives in `src/launch.rs`, outside
`src/ui/` entirely, and which is the only thing in the crate that turns it into a process.

The set of keys the dashboard responds to SHALL
be exactly the set `dashboard-loop`, `list-selection`, `list-filtering`, `detail-scroll`,
`artifact-tabs`, `live-updates`, and `agent-launch` specify. `live-updates`' addition is `r`,
which `SPEC.md` → Keys has documented as "Force refresh" since `repo-foundation`;
`agent-launch`'s additions are `a`, `c`, `s`, and `g`, which `SPEC.md` → Keys and `README.md`
→ Keys have both documented since `repo-foundation` and which this change is the one to make
true.

#### Scenario: Every printable key leaves the change tree byte-identical

- **WHEN** `run_loop` runs against a `TestBackend` of 120x20 and again of 60x20, over a
  `Dashboard` loaded by `ui::load` from a real `crate::testutil::ScratchDir` repository
  holding one change whose `tasks.md` has two checked and three unchecked items, with the
  tracked-tasks tab selected, driven by a scripted source delivering a Press of every
  ASCII printable character from `!` to `~`, then `Enter`, `Esc`, `Backspace`, `Tab`, and
  the four arrows, and finally `Ctrl-C`
- **THEN** the run terminates and `crate::testutil::snapshot` over that scratch directory
  is byte-identical to the snapshot taken before the run, including every file's bytes,
  size, and modification time
- **AND** re-reading `tasks.md` after the run yields `Progress { completed: 2, total: 5 }`,
  the same pair as before it
- **AND** the assertion discriminates: the same snapshot comparison fails when a single
  byte of `tasks.md` is deliberately rewritten between the two snapshots
- **AND** the ASCII sweep now includes `r`, which `live-updates` maps to `Action::Refresh`
  outside filter mode — so this scenario is also the proof that a refresh, like every other
  key, writes nothing. `live-updates`' own
  `ui::tests::live::a_live_watcher_over_the_tree_writes_nothing` makes the same claim with a
  **real** `notify` watch open on the tree throughout
- **AND** the sweep also includes `a`, `c`, `s`, and `g`, which `agent-launch` maps to its four
  actions, and the run is driven with `agents.reachable` **true** and a **recording** launcher
  double so all four reach `launch::decide` and produce real requests rather than being
  short-circuited — the snapshot claim is worth nothing if the keys did nothing. The recording
  launcher received requests; the tree is still byte-identical

#### Scenario: No action mutates a task item

- **WHEN** every `ui::app::Action` variant is applied in turn to a `Dashboard` whose
  selected change carries a tracked-tasks artifact, whose `agents.reachable` is `true`, and
  whose visible list is non-empty
- **THEN** the `Dashboard`'s `changes` field is `==` to its value before the application
  for every variant, so no action edits a `Change`, an `ArtifactRef`, or a `Progress`
- **AND** the `Action` enum holds exactly the **seventeen** variants `dashboard-loop`,
  `list-filtering`, `artifact-tabs`, `live-updates`, and `agent-launch` specify, with no
  toggle, check, edit, save, or write variant among them. `live-updates`' `Refresh` is the
  thirteenth, and it sets a `bool` on `Dashboard::refresh`; `agent-launch`'s `LaunchApply`,
  `LaunchContinue`, `LaunchArchive`, and `FocusAgent` are the fourteenth through seventeenth,
  and each writes only `Dashboard::launch`. **None of the five reaches a collaborator, performs
  any I/O, or starts any work of its own**: the launch actions produce a `launch::Request`
  value, and `run_loop` — not `apply` — is what hands it to something that can act on it
- **AND** the count is bumped from thirteen to seventeen **deliberately**, as this scenario's
  own subject rather than as a repair to a failing assertion: the number exists so a mutating
  action cannot be added silently, and moving it is the moment to state what the four new ones
  do and do not touch
- **AND** the variant count is asserted against an array the test builds through an
  exhaustive `match`, so an eighteenth variant is a compile error rather than a silent gap in
  the sweep
- **AND** after each of the four launch variants the dashboard's `launch.pending` is `Some` or
  its `launch.problems` is non-empty, which is what proves the fixture reached the decision
  rather than satisfying "changes is unchanged" vacuously through an unreachable socket

#### Scenario: The dashboard names no write API

- **WHEN** every file under `src/ui/`, **plus `src/watch.rs`, `src/refresh.rs`,
  `src/agents.rs`, and `src/launch.rs`**, is
  searched for a filesystem-write API — `fs::write`, `File::create`, `OpenOptions`,
  `fs::remove_`, `fs::create_dir`, `fs::rename`, `fs::copy`, `set_permissions`,
  `fs::hard_link`, and `fs::soft_link` — over each file's production slice, with
  `#[cfg(test)]` modules stripped
- **THEN** there is no match
- **AND** the four extra modules are in the searched set because `live-refresh`,
  `agent-polling`, and `agent-launch` put the watcher, the worker, the poller, and the launcher
  there: a watcher is one edit away from a marker file, a worker one edit away from writing
  beside the tree it reads, and the launcher is the module with an actual reason to write —
  none of which would have been visible to a sweep scoped to `src/ui/`
- **AND** `src/launch.rs` passes it while genuinely persisting the agent-name mapping, which is
  the point: it writes only by calling `state::record`, and every write API this check names
  lives in `src/state.rs`, inside the state directory `plugin-state` confines it to. A launcher
  that reached for `fs::write` itself — to log, to mark, to cache — would be caught here
- **AND** it is paired with a positive control asserting that `src/state.rs`'s production
  slice **does** name a write API, and with a second control proving the `#[cfg(test)]`
  stripper is not vacuous — `src/ui/mod.rs`'s test slice must name a write API its
  production slice does not
- **AND** the check is proven able to fail against a copy carrying
  `let _ = std::fs::write(path, "x");` inside `ui::read_artifact`, against one carrying
  the same call in `src/watch.rs`'s production code, and against one carrying it in
  `src/launch.rs`'s production code

#### Scenario: A launched agent's writes are not the plugin's writes

- **WHEN** `ui::run_wired` is driven over a real scratch repository, a real scratch state
  directory, and a scratch `herdr` program, and an `a` press launches an agent end to end —
  the three Herdr calls are logged, the mapping is recorded, and the returned dashboard's
  `agent_names` carries the pair
- **THEN** `crate::testutil::snapshot` over the repository tree is byte-identical to the
  snapshot taken before the run, including every file's bytes, size, and modification time
- **AND** the only file written anywhere by the plugin is `agent-names.toml` inside the scratch
  **state** directory, proven by a second snapshot pair over that directory showing exactly one
  file appearing
- **AND** the boundary is stated rather than implied: what the plugin started would, against a
  real Herdr and a real agent, go on to edit `openspec/`; the scratch program starts nothing, so
  this scenario measures the plugin's own writes and nothing else, which is exactly the claim
  `PRD.md` → Non-goals makes
