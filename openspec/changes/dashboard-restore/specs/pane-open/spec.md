## ADDED Requirements

### Requirement: `open`, `open-tab`, and `restore` are subcommands of their own, with no flag grammar

`parse` SHALL classify the single argument `open` as `Invocation::Open`, the single
argument `open-tab` as `Invocation::OpenTab`, and the single argument `restore` as
`Invocation::Restore` (`dashboard-restore`'s addition). The binary SHALL NOT accept a flag on any
subcommand: `open --tab` SHALL be rejected exactly as `ui --tab` already is, carrying the
offending token. `usage()` SHALL list all four commands — `ui`, `open`, `open-tab`, `restore` —
one line each.

`SPEC.md` → Herdr integration → Manifest shows the tab action's command as
`["./target/release/herdr-openspec", "open", "--tab"]`, which contradicts
`openspec/IMPLEMENTATION-ORDER.md`'s Phase 6 row ("the `open` and `open-tab` binary
subcommands"). The flat form is chosen and `SPEC.md` is corrected: `parse` stays a total
function over a flat token list with no flag state, and its landed rejection scenario for
`ui --tab` keeps its meaning rather than becoming an exception to a flag grammar that
exists elsewhere.

#### Scenario: `open` alone classifies as the split-pane invocation

- **WHEN** `parse(&["open"])` is called
- **THEN** it returns `Invocation::Open`
- **AND** no filesystem, process, or environment access occurs during classification

#### Scenario: `open-tab` alone classifies as the tab invocation

- **WHEN** `parse(&["open-tab"])` is called
- **THEN** it returns `Invocation::OpenTab`

#### Scenario: A flag after a subcommand is rejected, carrying the token

- **WHEN** `parse(&["open", "--tab"])` is called
- **THEN** it returns `Invocation::Reject(Some("--tab"))`
- **AND** `parse(&["open-tab", "x"])` returns `Invocation::Reject(Some("x"))`
- **AND** `parse(&["ui", "--tab"])` still returns `Invocation::Reject(Some("--tab"))`,
  unchanged by this change
- **AND** `parse(&["restore", "--all"])` returns `Invocation::Reject(Some("--all"))`

#### Scenario: `restore` alone classifies as the restore invocation

- **WHEN** `parse(&["restore"])` is called
- **THEN** it returns `Invocation::Restore`
- **AND** no filesystem, process, or environment access occurs during classification

#### Scenario: Usage names all four commands

- **WHEN** `usage()` is called and its `Commands:` block is split into lines
- **THEN** there are exactly four command lines, whose first whitespace-delimited tokens
  are `ui`, `open`, `open-tab`, and `restore` — asserted as whole tokens, because `open` is
  a substring of `open-tab` and a `contains` check would be satisfied by either alone
- **AND** the summary line reads `usage: herdr-openspec <ui|open|open-tab|restore>`

## MODIFIED Requirements

### Requirement: An already-open dashboard is focused, never duplicated

`herdr plugin pane open` is **not** idempotent: measured against Herdr 0.8.2, a second
invocation with the same `--plugin` and `--entrypoint` opens a **second** pane. Both
subcommands SHALL therefore run `herdr pane list` first and SHALL focus an existing
dashboard pane rather than opening another.

`herdr pane list` exposes no plugin ownership: measured, a plugin pane is distinguished
from an ordinary pane only by a `label` field carrying the manifest pane's `title`, which
ordinary panes omit entirely. A listed pane SHALL be treated as this workspace's dashboard
when **all three** hold: it carries a string `pane_id`, its `label` equals
`open::DASHBOARD_LABEL` (the constant the manifest's two pane titles are checked against),
and its `workspace_id` equals the context's workspace id. The first match in the list's own
order SHALL be used.

**`cwd` is not compared, corrected from the first draft of this requirement** (which
matched on `cwd` too, mirroring the context's workspace cwd). Every dashboard pane this
plugin opens now carries the **plugin root** as its `cwd`, identically, regardless of
workspace — see "`open`/`open-tab` never pass `--cwd`" below — so the value no longer
discriminates anything, and comparing it would only ever match or fail to match by
coincidence.

Both manifest panes carry the title `OpenSpec`, so `open` and `open-tab` are deliberately
indistinguishable in `pane list` and each will focus the other's pane. This is the intended
behaviour — "show me the dashboard" has one answer per workspace — and is why the tab pane
is not given a distinct title.

**A matching pane may be stale** (`dashboard-restore`'s addition). After a Herdr server
restart a dashboard pane keeps its id, its `OpenSpec` label, and its workspace, but its
process is replaced by a fresh shell (`dashboard-restore` → "A stale dashboard pane is
recognised from its process, not its label"). Before focusing the first match, both
subcommands SHALL classify it with `herdr pane process-info --pane <id>`. A **stale** match
SHALL be replaced in place by `dashboard-restore` → "A stale dashboard pane is replaced in
its own slot", with the replacement opened `--focus`, and no `plugin pane focus` call is
made. A match that is not stale, or whose classification call fails or answers with
something unparseable, SHALL be focused exactly as before — an unclassifiable pane is never
replaced. Only the first match is classified; later matches are neither classified nor
replaced by `open`.

#### Scenario: A matching pane is focused and nothing is opened

- **WHEN** `open` runs with a workspace id of `w8`, and `herdr pane list` answers with a
  pane `{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8"}`
- **AND** `herdr pane process-info --pane w8:pG` reports a foreground process other than
  the pane's own shell
- **THEN** exactly three Herdr calls are made: `["pane","list"]`, then
  `["pane","process-info","--pane","w8:pG"]`, then `["plugin","pane","focus","w8:pG"]`
- **AND** no `plugin pane open` call is made
- **AND** the process exits 0

#### Scenario: No listed pane matches, so one is opened

- **WHEN** `herdr pane list` answers with panes carrying no `label` at all, and again when
  it answers with an empty `"panes": []`
- **THEN** in both cases the second call is `plugin pane open`, not `plugin pane focus`

#### Scenario: A matching entry with no usable `pane_id` is not a match

- **WHEN** the only otherwise-matching entry carries no `pane_id`, or a `pane_id` that is
  not a JSON string
- **THEN** it is skipped and a pane is opened instead
- **AND** no focus call is made with an empty or non-string pane id

#### Scenario: A labelled pane in another workspace is not this workspace's dashboard

- **WHEN** the context's workspace id is `w8` and the only labelled pane reports
  `"workspace_id":"wA"`
- **THEN** no focus call is made and a pane is opened instead

#### Scenario: Two matches focus the first in list order

- **WHEN** `herdr pane list` answers with two panes both matching, `w8:pG` before `w8:pH`
- **THEN** the focus call names `w8:pG`

#### Scenario: `open-tab` focuses a split dashboard, and `open` focuses a tab one

- **WHEN** `open-tab` runs while a pane labelled `OpenSpec` opened by `open` is listed for
  this workspace
- **THEN** that pane is focused and no tab is created
- **AND** the symmetric case — `open` while only the tab dashboard is listed — focuses the
  tab's pane

#### Scenario: A stale match is replaced in place and the replacement focused

- **WHEN** `open` runs with a workspace id of `w8`, `herdr pane list` lists
  `{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8"}`, and
  `herdr pane process-info --pane w8:pG` answers with `"shell_pid":4100` and exactly one
  foreground process, `{"name":"zsh","pid":4100}`
- **THEN** the calls after the classification are
  `["plugin","pane","open","--plugin","herdr-openspec","--entrypoint","dashboard","--placement","split","--target-pane","w8:pG","--direction","right","--focus"]`
  then `["pane","close","w8:pG"]`
- **AND** no `["plugin","pane","focus","w8:pG"]` call is made
- **AND** the process exits 0

#### Scenario: An unclassifiable match is focused, never replaced

- **WHEN** the first match is `w8:pG` and `herdr pane process-info --pane w8:pG` exits
  non-zero, and again when it answers with JSON carrying no `foreground_processes` array
- **THEN** in both cases the next call is `["plugin","pane","focus","w8:pG"]`
- **AND** no `plugin pane open` and no `pane close` call is made
- **AND** the classification failure is written to stderr as a warning, and the process
  exits 0

## REMOVED Requirements

### Requirement: `open` and `open-tab` are subcommands of their own, with no flag grammar

**Reason**: Replaced by "`open`, `open-tab`, and `restore` are subcommands of their own, with no flag grammar", which adds the `restore` subcommand and a four-command usage text; a MODIFIED block cannot retitle the "Usage names all three commands" scenario.

**Migration**: None for users. The `parse`/`usage` tests asserting three commands move to the replacing requirement's four-command scenarios.
