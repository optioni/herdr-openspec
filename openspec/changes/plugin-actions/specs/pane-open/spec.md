## ADDED Requirements

### Requirement: `open` and `open-tab` are subcommands of their own, with no flag grammar

`parse` SHALL classify the single argument `open` as `Invocation::Open` and the single
argument `open-tab` as `Invocation::OpenTab`. The binary SHALL NOT accept a flag on any
subcommand: `open --tab` SHALL be rejected exactly as `ui --tab` already is, carrying the
offending token. `usage()` SHALL list all three commands — `ui`, `open`, `open-tab` — one
line each.

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

#### Scenario: Usage names all three commands

- **WHEN** `usage()` is called and its `Commands:` block is split into lines
- **THEN** there are exactly three command lines, whose first whitespace-delimited tokens
  are `ui`, `open`, and `open-tab` — asserted as whole tokens, because `open` is a
  substring of `open-tab` and a `contains` check would be satisfied by either alone
- **AND** the summary line reads `usage: herdr-openspec <ui|open|open-tab>`

### Requirement: The invocation context is read from Herdr's injected environment through one injected lookup

Herdr injects the invocation context into every process it starts for a plugin action.
Measured against Herdr 0.8.2: the action's process working directory is the **plugin
root**, never the workspace's, and the context arrives as `HERDR_PLUGIN_CONTEXT_JSON`
alongside the discrete `HERDR_PLUGIN_ID`, `HERDR_WORKSPACE_ID`, and `HERDR_PANE_ID`
variables.

`open::context` SHALL take a `&dyn Fn(&str) -> Option<String>` lookup, exactly as
`config::config_dir` and `state::state_dir` do, and SHALL perform no other I/O. It SHALL
read, in this order of preference for each field:

| Field | Source |
|---|---|
| plugin id | `HERDR_PLUGIN_ID`, else the literal `herdr-openspec` |
| workspace id | `HERDR_PLUGIN_CONTEXT_JSON` → `workspace_id`, else `HERDR_WORKSPACE_ID` |
| workspace cwd | `HERDR_PLUGIN_CONTEXT_JSON` → `workspace_cwd`, else its `focused_pane_cwd`, else absent |
| focused pane id | `HERDR_PLUGIN_CONTEXT_JSON` → `focused_pane_id`, else `HERDR_PANE_ID` |

A value that is absent, empty, or whitespace-only SHALL be treated as absent, on
`config::non_blank`'s existing terms. `HERDR_PLUGIN_CONTEXT_JSON` that is not valid JSON,
or is valid JSON that is not an object, SHALL be ignored in favour of the discrete
variables rather than becoming a failure — a context this plugin cannot read is not a
reason to refuse to open a dashboard. Only an absent **workspace id** is fatal, because
neither call below can be addressed without it.

#### Scenario: A full context yields all four fields

- **WHEN** `context` is called with a lookup returning
  `HERDR_PLUGIN_CONTEXT_JSON` = `{"workspace_id":"w8","workspace_cwd":"/repo","tab_id":"w8:t1","focused_pane_id":"w8:p1","focused_pane_cwd":"/repo"}`
  and `HERDR_PLUGIN_ID` = `herdr-openspec`
- **THEN** it returns a context whose plugin id is `herdr-openspec`, workspace id is `w8`,
  workspace cwd is `/repo`, and focused pane id is `w8:p1`

#### Scenario: The discrete variables answer when the context JSON is absent

- **WHEN** `context` is called with a lookup returning only `HERDR_WORKSPACE_ID` = `w8`
  and `HERDR_PANE_ID` = `w8:p1`
- **THEN** it returns a context whose workspace id is `w8` and focused pane id is `w8:p1`
- **AND** its workspace cwd is absent

#### Scenario: Unreadable context JSON falls back rather than failing

- **WHEN** `context` is called with `HERDR_PLUGIN_CONTEXT_JSON` = `not json at all`,
  and again with `["w8"]` (valid JSON that is not an object), each time alongside
  `HERDR_WORKSPACE_ID` = `w8`
- **THEN** both calls return a context whose workspace id is `w8`
- **AND** neither returns an error

#### Scenario: The workspace cwd falls back to the focused pane's cwd

- **WHEN** `context` is called with `HERDR_PLUGIN_CONTEXT_JSON` =
  `{"workspace_id":"w8","focused_pane_cwd":"/repo","focused_pane_id":"w8:p1"}` — no
  `workspace_cwd` key at all
- **THEN** the returned context's workspace cwd is `/repo`
- **AND** when neither key is present the workspace cwd is absent, and the split and tab
  argument vectors below omit `--cwd` rather than passing an empty one

#### Scenario: Blank values are absent

- **WHEN** `context` is called with `HERDR_WORKSPACE_ID` = `"   "` and
  `HERDR_PLUGIN_ID` = `""`, with no context JSON
- **THEN** it fails with a reason naming the workspace, because a whitespace-only
  workspace id is treated as absent
- **AND** when only `HERDR_PLUGIN_ID` is blank and a workspace id is present, the plugin id
  falls back to the literal `herdr-openspec`

#### Scenario: No workspace at all is the one fatal absence

- **WHEN** `context` is called with a lookup returning nothing for every variable
- **THEN** it returns an error whose text names `HERDR_WORKSPACE_ID` and states that the
  command must be invoked from Herdr
- **AND** no Herdr call is made

### Requirement: An already-open dashboard is focused, never duplicated

`herdr plugin pane open` is **not** idempotent: measured against Herdr 0.8.2, a second
invocation with the same `--plugin` and `--entrypoint` opens a **second** pane. Both
subcommands SHALL therefore run `herdr pane list` first and SHALL focus an existing
dashboard pane rather than opening another.

`herdr pane list` exposes no plugin ownership: measured, a plugin pane is distinguished
from an ordinary pane only by a `label` field carrying the manifest pane's `title`, which
ordinary panes omit entirely. A listed pane SHALL be treated as this workspace's dashboard
when **all four** hold: it carries a string `pane_id`, its `label` equals
`open::DASHBOARD_LABEL` (the constant the manifest's two pane titles are checked against),
its `workspace_id` equals the context's workspace id, and its `cwd` equals the context's
workspace cwd. When the context carries no workspace cwd, the `cwd` test SHALL be skipped
rather than matching every pane. The first match in the list's own order SHALL be used.

The `cwd` comparison SHALL be `std::path::Path` equality — component-wise, so a trailing
separator does not defeat it — and SHALL NOT canonicalize: `open` performs no filesystem
access, and both strings originate from Herdr itself (the `--cwd` this plugin passed, and
the `cwd` Herdr echoes back), so a canonicalizing comparison would buy nothing and cost a
`stat`.

Both manifest panes carry the title `OpenSpec`, so `open` and `open-tab` are deliberately
indistinguishable in `pane list` and each will focus the other's pane. This is the intended
behaviour — "show me the dashboard" has one answer per workspace — and is why the tab pane
is not given a distinct title.

#### Scenario: A matching pane is focused and nothing is opened

- **WHEN** `open` runs with a workspace id of `w8` and workspace cwd `/repo`, and
  `herdr pane list` answers with a pane `{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8","cwd":"/repo"}`
- **THEN** exactly two Herdr calls are made: `["pane","list"]` then
  `["plugin","pane","focus","w8:pG"]`
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

#### Scenario: A labelled pane rooted elsewhere is not this workspace's dashboard

- **WHEN** the context's workspace cwd is `/repo` and the only labelled pane in `w8`
  reports `"cwd":"/Users/x/.config/herdr/plugins/github/herdr-openspec-abc"` — the plugin
  root a `--cwd`-less open would have produced
- **THEN** no focus call is made and a pane is opened instead

#### Scenario: With no workspace cwd known, the label and workspace alone decide

- **WHEN** the context carries no workspace cwd and one labelled pane in `w8` exists with
  any `cwd`
- **THEN** it is focused

#### Scenario: Two matches focus the first in list order

- **WHEN** `herdr pane list` answers with two panes both matching, `w8:pG` before `w8:pH`
- **THEN** the focus call names `w8:pG`

#### Scenario: `open-tab` focuses a split dashboard, and `open` focuses a tab one

- **WHEN** `open-tab` runs while a pane labelled `OpenSpec` opened by `open` is listed for
  this workspace and cwd
- **THEN** that pane is focused and no tab is created
- **AND** the symmetric case — `open` while only the tab dashboard is listed — focuses the
  tab's pane

### Requirement: `open` splits the pane the action was invoked from

Measured against Herdr 0.8.2: a `split`-placement plugin pane **requires an existing target
pane**. `herdr plugin pane open --placement split --workspace <id>` fails with
`{"error":{"code":"invalid_params","message":"split and zoomed plugin panes target an
existing pane; use target_pane_id"}}` and exit 1 whenever that workspace is not the focused
one, while `--target-pane <id>` succeeds. `--direction` is optional here — unlike
`herdr pane split`, which requires it.

When no dashboard is listed, `open` SHALL issue exactly:

```
herdr plugin pane open --plugin <plugin id> --entrypoint dashboard
                       --placement split --direction right
                       --target-pane <focused pane id> --cwd <workspace cwd> --focus
```

`--workspace` SHALL NOT be passed. `--target-pane` SHALL be omitted when the context
carries no focused pane id, in which case Herdr targets the focused pane itself. `--cwd`
SHALL be omitted when the context carries no workspace cwd; the pane then inherits the
plugin root, which is the pre-change behaviour and is named as a degraded state rather than
a failure.

#### Scenario: The full split argument vector

- **WHEN** `open` opens with plugin id `herdr-openspec`, focused pane `w8:p1`, and
  workspace cwd `/repo`
- **THEN** the argument vector is exactly
  `["plugin","pane","open","--plugin","herdr-openspec","--entrypoint","dashboard","--placement","split","--direction","right","--target-pane","w8:p1","--cwd","/repo","--focus"]`
- **AND** it contains no `--workspace` and no `--no-focus`

#### Scenario: No focused pane id omits `--target-pane` rather than passing an empty one

- **WHEN** the context carries no focused pane id
- **THEN** the argument vector contains neither `--target-pane` nor an empty argument
  after it

#### Scenario: No workspace cwd omits `--cwd` and the pane inherits the plugin root

- **WHEN** the context carries no workspace cwd
- **THEN** the argument vector contains no `--cwd`
- **AND** the call is still made, because a dashboard rooted at the plugin root is a
  degraded view rather than a refusal

### Requirement: `open-tab` opens a tab in the invoking workspace

When no dashboard is listed, `open-tab` SHALL issue exactly:

```
herdr plugin pane open --plugin <plugin id> --entrypoint dashboard-tab
                       --placement tab --workspace <workspace id>
                       --cwd <workspace cwd> --focus
```

`--target-pane` and `--direction` SHALL NOT be passed: a tab placement needs no target
pane, and `--workspace` is measured to work for a tab where it fails for a split.

#### Scenario: The full tab argument vector

- **WHEN** `open-tab` opens with plugin id `herdr-openspec`, workspace `w8`, and workspace
  cwd `/repo`
- **THEN** the argument vector is exactly
  `["plugin","pane","open","--plugin","herdr-openspec","--entrypoint","dashboard-tab","--placement","tab","--workspace","w8","--cwd","/repo","--focus"]`
- **AND** it contains neither `--target-pane` nor `--direction`

#### Scenario: The two subcommands differ only in placement and target

- **WHEN** the split and tab argument vectors are built from the same context
- **THEN** they agree on `--plugin`, `--cwd`, and `--focus`
- **AND** they differ on exactly four keys: `--entrypoint` (`dashboard` against
  `dashboard-tab`), `--placement` (`split` against `tab`), `--direction right` and
  `--target-pane` (split only), and `--workspace` (tab only)

### Requirement: Herdr's own reason is carried verbatim, and a failed listing degrades to opening

Measured against Herdr 0.8.2, the `plugin` command family fails in exactly the two shapes
`agent-launch` already recorded for the `pane` and `agent` families: a **domain** error is a
JSON envelope on **stderr** with exit **1** — `{"error":{"code":"plugin_pane_not_found","message":"plugin pane entrypoint 'nope' not found"},"id":"cli:plugin"}` — and a **usage**
error is plain text on stderr with exit **2** (`missing required --plugin`). Neither shape
SHALL be parsed. `open::run` SHALL treat the seam's `CliError` text as an opaque reason and
return it on an `open::Report`; the **process** — `src/main.rs`, through the pure
`open::report_output` below — is what writes to stderr and exits **1**.

A `plugin pane open` failure SHALL stop the command. A failed `plugin pane focus` SHALL be
split on the exit code the seam already carries, with no parsing of either payload:

| Focus failure | Behaviour |
|---|---|
| `CliError::Failed` with code `Some(2)` — a **usage** error, which is what a Herdr that does not know `plugin pane focus` produces | Warn, then fall through and open once. Refusing here would fail closed on a Herdr this manifest's `min_herdr_version` still declares supported |
| `CliError::Failed` with any other code, or `CliError::NotStarted` | Stop. Nothing is opened |

The asymmetry is deliberate. A systematic focus failure that fell through unconditionally
would leak one dashboard per keypress, which the exit-code split bounds to the one shape
that means "this Herdr has no such subcommand"; the recoverable case — a pane closed
between the listing and the focus — is a **domain** error (code 1) and is recovered by
invoking the action again, when the pane will no longer be listed.

A failed or unparseable `herdr pane list` is the one degrade in the other direction: the
reason SHALL be written to stderr as a warning and the open call SHALL still be attempted,
because refusing to open is the "fail closed" behaviour this project forbids, and a socket
too broken to list is one whose open call will report its own reason.

`open` SHALL parse **only** `pane list`'s output. The `plugin pane open` response — measured
as `{"id":"cli:plugin","result":{"plugin_pane":{"entrypoint":…,"plugin_id":…,"pane":{"pane_id":…}}}}`,
a **different** envelope shape from `pane split`'s `result.pane.pane_id` — is not read at
all, so its shape cannot break this command.

#### Scenario: A domain error is carried verbatim and stops the command

- **WHEN** `plugin pane open` fails with the `plugin_pane_not_found` envelope above at exit 1
- **THEN** `Report.outcome` is `Err` containing `plugin_pane_not_found` and
  `plugin pane entrypoint 'nope' not found`, and the process exits 1
- **AND** stdout is empty

#### Scenario: A usage error on the open call is treated identically, without being parsed

- **WHEN** `plugin pane open` fails with the plain text `missing required --plugin` at exit 2
- **THEN** the process exits 1, not 2
- **AND** the reason contains `missing required --plugin` verbatim

#### Scenario: An unstartable `herdr` names the program

- **WHEN** the `herdr` program cannot be started at all
- **THEN** the process exits 1
- **AND** the reason names `herdr` and the operating system's reason

#### Scenario: A domain-error focus stops the command and opens nothing

- **WHEN** a matching pane was listed but `plugin pane focus` fails with
  `plugin_pane_not_found` at exit **1**
- **THEN** exactly two Herdr calls were made — `pane list` then `plugin pane focus`
- **AND** no `plugin pane open` call follows
- **AND** the process exits 1 with the reason on stderr

#### Scenario: A usage-error focus warns and opens once

- **WHEN** a matching pane was listed but `plugin pane focus` fails with plain text at exit
  **2** — what a Herdr that does not know the subcommand produces
- **THEN** exactly three Herdr calls were made: `pane list`, `plugin pane focus`, then
  `plugin pane open`
- **AND** `Report.warnings` carries the focus reason
- **AND** the process exits 0 when the open succeeds, so the user gets a dashboard rather
  than nothing on a Herdr this manifest still declares supported

#### Scenario: A failed listing warns and still opens

- **WHEN** `herdr pane list` fails with an error envelope
- **THEN** the reason appears on stderr
- **AND** a `plugin pane open` call is still made
- **AND** the process's exit status is the open call's outcome, 0 on success

#### Scenario: An unparseable listing warns and still opens

- **WHEN** `herdr pane list` succeeds but its stdout is `not json`, or is valid JSON with no
  `result.panes` array
- **THEN** the reason appears on stderr, a `plugin pane open` call is still made, and the
  exit status is the open call's outcome

#### Scenario: A successful open is silent

- **WHEN** every call succeeds and no warning was recorded
- **THEN** the process exits 0
- **AND** stdout is empty and stderr is empty

### Requirement: The process's output and exit status are computed by pure functions

`src/main.rs` SHALL contain no logic a test cannot reach. Both of the decisions `main`
would otherwise make SHALL be pure, total functions in `src/open.rs`:

- `open::placement_for(&Invocation) -> Option<Placement>` — `Invocation::Open` maps to
  `Placement::Split`, `Invocation::OpenTab` to `Placement::Tab`, and every other variant to
  `None`, so a dispatch that sent `open-tab` to the split placement is a unit-test failure
  rather than an invisible one.
- `open::report_output(&Report) -> (Vec<String>, i32)` — the lines to write to stderr, in
  order (every warning, then the error if there is one), and the exit status: `0` when the
  outcome is `Ok`, `1` when it is `Err`. Never `2`, never `3`.

`main`'s two new arms SHALL do nothing but call `open::run_from_env`, write those lines,
and exit with that status.

#### Scenario: Each subcommand maps to its own placement

- **WHEN** `placement_for` is called with `Invocation::Open` and with `Invocation::OpenTab`
- **THEN** it returns `Some(Placement::Split)` and `Some(Placement::Tab)` respectively
- **AND** it returns `None` for `Invocation::Ui` and for `Invocation::Reject(..)`

#### Scenario: The exit status and stderr lines follow the report

- **WHEN** `report_output` is called with a report carrying two warnings and an `Err`
- **THEN** it returns those two warnings followed by the error text, and status `1`
- **AND** a report with warnings and an `Ok` outcome returns the warnings and status `0`,
  and an empty report returns no lines and status `0`

### Requirement: The open family needs no terminal and renders nothing

`open` and `open-tab` SHALL never enter raw mode, never enter the alternate screen, and
never construct a `ratatui` terminal. Measured against Herdr 0.8.2, a plugin action's
stdout and stdin are **not** a terminal, so a subcommand that required one could never run
from the action menu at all. Neither subcommand SHALL exit with status 3, which stays
`ui`'s "not a terminal" status alone.

#### Scenario: `open` outside Herdr fails promptly rather than hanging or rendering

- **WHEN** the built binary is run as `herdr-openspec open` with stdout and stderr piped,
  stdin left at EOF, and every `HERDR_*` variable removed from the child's environment so
  a suite running inside a Herdr pane cannot inherit one
- **THEN** it exits 1 within a ten-second deadline that fails the test if the process is
  still alive
- **AND** stderr names `HERDR_WORKSPACE_ID`
- **AND** stdout is empty, and the status is never 3

#### Scenario: `main` really routes each subcommand to its own placement

- **WHEN** the built binary is run as `herdr-openspec open` and as
  `herdr-openspec open-tab` with `PATH` pointing at a scratch directory holding a
  `#!/bin/sh` program named `herdr` that appends its argument vector to a file and answers
  `pane list` with an empty `{"result":{"panes":[]}}`, and with a synthetic
  `HERDR_WORKSPACE_ID` and `HERDR_PLUGIN_CONTEXT_JSON` supplied
- **THEN** both exit 0
- **AND** the recorded argument vectors show `--placement split --direction right` for
  `open` and `--placement tab --workspace` for `open-tab`, so a `main` that dispatched
  both to one placement fails — the wiring defect a status-only assertion cannot see
- **AND** the recorded vectors carry the `--cwd` the synthetic context named
- **AND** `herdr-openspec open --tab` still exits 2 with usage on stderr

#### Scenario: The open path names no terminal API

- **WHEN** `src/open.rs` is searched for `enable_raw_mode`, `disable_raw_mode`,
  `EnterAlternateScreen`, `LeaveAlternateScreen`, `ratatui`, `Frame`, `Rect`, `Buffer`, and
  `Style`
- **THEN** none appears, in production code or in tests
- **AND** the search is proved non-vacuous by matching the same pattern in
  `src/ui/terminal.rs`

### Requirement: `src/open.rs` reaches the `herdr` program only through the trait object

`src/open.rs` SHALL be the crate's **third** `cli::HerdrCli` consumer, after `src/agents.rs`
and `src/launch.rs` — `src/cli.rs` declares the trait rather than consuming it, and
`src/ui/mod.rs` only passes a constructed handle along. It becomes the **fifth** file on the
seam gates' `ALLOWED` list, and the two counts are not the same number. It SHALL name no
process-spawn API
(`process::Command`, `Command::new`, `Stdio`) and SHALL live at the top level of `src/`,
never under `src/ui/`, where naming `HerdrCli` is forbidden outright.

`src/main.rs` SHALL NOT name `HerdrCli`, `RealHerdrCli`, or `agent_cli_via`: it SHALL call
one `open::run_from_env`-shaped binding that constructs the real handle inside
`src/open.rs`, keeping `main` at reading arguments, dispatching, writing stderr, and setting
the exit status.

#### Scenario: The module spawns nothing

- **WHEN** `src/open.rs` is searched for `process::Command`, `Command::new`, and `Stdio`
- **THEN** nothing matches
- **AND** the tree-wide search excluding `src/cli.rs` still reports no spawn API anywhere,
  proved non-vacuous by `src/cli.rs` itself matching

#### Scenario: The Herdr handle stays inside the five allowed files

- **WHEN** every `.rs` file under `src/` and `tests/` except `src/cli.rs`,
  `src/agents.rs`, `src/ui/mod.rs`, `src/launch.rs`, and `src/open.rs` is searched for
  `HerdrCli`, `RealHerdrCli`, and `agent_cli_via`
- **THEN** nothing matches, `src/main.rs` included
- **AND** the file count actually searched is at or above the floor this change measures,
  so an exclusion that swallowed the tree would fail rather than pass

#### Scenario: No file under `src/ui/` names the open path's CLI seam

- **WHEN** every `.rs` file under `src/ui/` is searched for `HerdrCli`
- **THEN** nothing matches, unchanged by this change
