# pane-open Specification

## Purpose
Covers the `open` and `open-tab` subcommands that Herdr's action menu invokes to put the
dashboard on screen: reading the invocation context out of Herdr's injected environment
through one injected lookup, listing panes first so an existing dashboard in the workspace is
focused rather than duplicated, and issuing the exact `herdr plugin pane open` argument
vectors for a split placement targeting the invoking pane and for a tab placement in the
invoking workspace — never passing `--cwd`, which would break the manifest's relative
command. It also fixes how Herdr's own failure reasons are carried verbatim, which failures
stop the command and which degrade to opening anyway, and that this path renders nothing and
needs no terminal, leaving everything the opened pane then draws to the dashboard
capabilities.

## Requirements

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
- **AND** when neither key is present the workspace cwd is absent — `ui::run`'s own
  `startup_cwd` then falls back to its own OS working directory rather than a fixed value
  (`specs/plugin-build/spec.md` -> "The dashboard's own starting directory prefers the
  workspace context over the process cwd")

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

#### Scenario: A matching pane is focused and nothing is opened

- **WHEN** `open` runs with a workspace id of `w8`, and `herdr pane list` answers with a
  pane `{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8"}`
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

#### Scenario: Two matches focus the first in list order

- **WHEN** `herdr pane list` answers with two panes both matching, `w8:pG` before `w8:pH`
- **THEN** the focus call names `w8:pG`

#### Scenario: `open-tab` focuses a split dashboard, and `open` focuses a tab one

- **WHEN** `open-tab` runs while a pane labelled `OpenSpec` opened by `open` is listed for
  this workspace
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
                       --target-pane <focused pane id> --focus
```

`--workspace` SHALL NOT be passed. `--target-pane` SHALL be omitted when the context
carries no focused pane id, in which case Herdr targets the focused pane itself.

#### Scenario: The full split argument vector

- **WHEN** `open` opens with plugin id `herdr-openspec` and focused pane `w8:p1`
- **THEN** the argument vector is exactly
  `["plugin","pane","open","--plugin","herdr-openspec","--entrypoint","dashboard","--placement","split","--direction","right","--target-pane","w8:p1","--focus"]`
- **AND** it contains no `--workspace`, no `--no-focus`, and no `--cwd`

#### Scenario: No focused pane id omits `--target-pane` rather than passing an empty one

- **WHEN** the context carries no focused pane id
- **THEN** the argument vector contains neither `--target-pane` nor an empty argument
  after it

### Requirement: `open`/`open-tab` never pass `--cwd`

The first draft of this requirement had `open` pass `--cwd <workspace cwd>` — see the two
scenarios below this one for the specific vectors that carried it — reasoning that the
pane would otherwise inherit the **plugin root** rather than the workspace's own
repository (the empty-dashboard problem `plugin-actions`' own proposal names). Measured
live against Herdr 0.8.2, during this change's own group 10 manual check, `--cwd` does
something more consequential than set the pane's cwd: it changes what the manifest's
*relative* pane `command` (`./target/release/herdr-openspec`) resolves against.
`herdr plugin pane open --cwd /tmp …` failed outright —
`{"error":{"code":"plugin_pane_open_failed","message":"Unable to spawn
/tmp/./target/release/herdr-openspec because it does not exist"}}` — for **every**
workspace directory that does not itself hold a `target/release/herdr-openspec` binary,
which is effectively every real workspace. This directly contradicts Herdr's own
changelog ("Relative plugin commands now resolve from the plugin root", unqualified,
0.8.0) for the `--cwd`-given case; a `--cwd`-less open resolves correctly, exactly as the
changelog describes. `herdr-file-viewer` (installed locally, `min_herdr_version = 0.7.0`)
independently reaches the identical conclusion in its own shipped code: its launcher
script never passes `--cwd` to `plugin pane open` either.

`open` and `open-tab` SHALL therefore never pass `--cwd`, on any path. Every dashboard
pane this plugin opens carries the plugin root as its `cwd`. The pane's own starting
directory for the OpenSpec search is instead resolved by the process Herdr starts for
that pane — `ui::run` — from its own injected `HERDR_PLUGIN_CONTEXT_JSON` /
`HERDR_WORKSPACE_ID` environment, preferring the workspace cwd over the process's own OS
working directory. This is `AGENTS.md`'s already-documented fact that "every process
Herdr starts for a plugin" (`[[panes]]` no less than `[[actions]]`) receives this
injected context, read exactly where `open::context` already reads it — see
`specs/plugin-build/spec.md` -> "The dashboard's own starting directory prefers the
workspace context over the process cwd" for `ui::run`'s side of this correction.

#### Scenario: No `--cwd` is ever passed, regardless of what the context carries

- **WHEN** `open_args` is called with a context whose `workspace_cwd` is `Some("/repo")`,
  and again with one whose `workspace_cwd` is `None`
- **THEN** neither argument vector contains `--cwd`, for either placement

### Requirement: `open-tab` opens a tab in the invoking workspace

When no dashboard is listed, `open-tab` SHALL issue exactly:

```
herdr plugin pane open --plugin <plugin id> --entrypoint dashboard-tab
                       --placement tab --workspace <workspace id>
                       --focus
```

`--target-pane` and `--direction` SHALL NOT be passed: a tab placement needs no target
pane, and `--workspace` is measured to work for a tab where it fails for a split.

#### Scenario: The full tab argument vector

- **WHEN** `open-tab` opens with plugin id `herdr-openspec` and workspace `w8`
- **THEN** the argument vector is exactly
  `["plugin","pane","open","--plugin","herdr-openspec","--entrypoint","dashboard-tab","--placement","tab","--workspace","w8","--focus"]`
- **AND** it contains neither `--target-pane`, `--direction`, nor `--cwd`

#### Scenario: The two subcommands differ only in placement and target

- **WHEN** the split and tab argument vectors are built from the same context
- **THEN** they agree on `--plugin` and `--focus`
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

A `plugin pane open` failure SHALL stop the command, and no post-open listing or focus SHALL
follow it. A failed `plugin pane focus` **on an already-listed pane** SHALL be split on the
exit code the seam already carries, with no parsing of either payload:

| Focus failure | Behaviour |
|---|---|
| `CliError::Failed` with code `Some(2)` — a **usage** error, which is what a Herdr that does not know `plugin pane focus` produces | Warn, then fall through and open once. Refusing here would fail closed on a Herdr this manifest's `min_herdr_version` still declares supported |
| `CliError::Failed` with any other code, or `CliError::NotStarted` | Stop. Nothing is opened |

The asymmetry is deliberate. A systematic focus failure that fell through unconditionally
would leak one dashboard per keypress, which the exit-code split bounds to the one shape
that means "this Herdr has no such subcommand"; the recoverable case — a pane closed
between the listing and the focus — is a **domain** error (code 1) and is recovered by
invoking the action again, when the pane will no longer be listed.

This table governs the focus of a pane the **first** listing reported. It does **not** govern
the focus that follows a successful open, which warns on every failure shape — see "A newly
opened dashboard pane is focused after opening".

A failed or unparseable `herdr pane list` is the one degrade in the other direction: the
reason SHALL be written to stderr as a warning and the open call SHALL still be attempted,
because refusing to open is the "fail closed" behaviour this project forbids, and a socket
too broken to list is one whose open call will report its own reason.

`open` SHALL parse **only** `pane list`'s output — both of its `pane list` calls, and nothing
else. The `plugin pane open` response — measured
as `{"id":"cli:plugin","result":{"plugin_pane":{"entrypoint":…,"plugin_id":…,"pane":{"pane_id":…}}}}`,
a **different** envelope shape from `pane split`'s `result.pane.pane_id` — is not read at
all, so its shape cannot break this command. The post-open focus does not weaken this:
that step re-lists rather than reading the open response, precisely so no new envelope shape
enters the crate.

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
- **THEN** the first three Herdr calls are `pane list`, `plugin pane focus`, then
  `plugin pane open`, and the post-open listing and focus of "A newly opened dashboard pane is
  focused after opening" follow them
- **AND** `Report.warnings` carries the focus reason
- **AND** the process exits 0 when the open succeeds, so the user gets a dashboard rather
  than nothing on a Herdr this manifest still declares supported

#### Scenario: A failed listing warns and still opens

- **WHEN** `herdr pane list` fails with an error envelope
- **THEN** the reason appears on stderr
- **AND** a `plugin pane open` call is still made
- **AND** the process's exit status is the open call's outcome, 0 on success
- **AND** the pre-open dashboard pane ids are empty, so the post-open listing's first match is
  the pane focused

#### Scenario: An unparseable listing warns and still opens

- **WHEN** `herdr pane list` succeeds but its stdout is `not json`, or is valid JSON with no
  `result.panes` array
- **THEN** the reason appears on stderr, a `plugin pane open` call is still made, and the
  exit status is the open call's outcome

#### Scenario: A successful open is silent

- **WHEN** every call succeeds, the post-open listing identifies the pane that was opened, and
  no warning was recorded
- **THEN** the process exits 0
- **AND** stdout is empty and stderr is empty

### Requirement: A newly opened dashboard pane is focused after opening

`--focus` on the open call is **not sufficient for a tab**. Measured live against Herdr
0.9.0: `herdr plugin pane open --placement tab --workspace <id> --focus` creates the tab and
focuses the pane *within* it, but leaves that tab in the background — the user invokes the
action and nothing visibly happens. The same measurement shows `herdr plugin pane focus
<pane id>` **does** switch the workspace's active tab: focusing pane `wG:p0` flipped tab
`wG:tB` from `"focused":false` to `"focused":true` in the next `herdr tab list`.

After a `plugin pane open` that succeeds, `open::run` SHALL issue `herdr pane list` a second
time and, when that listing identifies the pane just opened (see the next requirement), SHALL
issue `plugin pane focus <pane id>` on it. This SHALL apply to **both** placements with no
placement branch: a split whose pane already took focus is unharmed by a second focus, and
one path is cheaper to specify and to test than two. `--focus` SHALL still be passed on the
open call, unchanged.

The second listing SHALL NOT be retried and SHALL NOT be delayed. Pane creation is
synchronous with respect to the listing API: measured on Herdr 0.9.0, a pane created by one
socket call (`herdr pane split wJ:p1 --direction right` → `wJ:p2`) is present in the very
next `herdr pane list`. There is no race for a retry to close.

Every failure **after** a successful open SHALL be recorded on `Report.warnings` and SHALL
leave `Report.outcome` as `Ok(())`:

| Post-open step | Behaviour |
|---|---|
| The second `pane list` fails (`CliError` of any variant) | Warn. No focus call is made |
| The second `pane list` succeeds but is unparseable, or carries no `result.panes` array | Warn. No focus call is made |
| The listing carries no dashboard pane for this workspace | Warn, naming the workspace id and that no dashboard pane was identified. No focus call is made |
| `plugin pane focus` fails, with **any** exit code or `CliError` variant | Warn |

There is deliberately **no exit-code split** here, unlike the focus of an already-listed pane
above it. That split exists to bound a systematic focus failure that would otherwise leak one
dashboard pane per keypress; nothing follows this focus, so a failure leaks nothing and the
pane is already open either way. Refusing to report success because the dashboard could not
be *raised* would be the fail-closed behaviour this project forbids.

#### Scenario: A tab open is followed by a listing and a focus

- **WHEN** `open-tab` runs with workspace `w8`, the first `herdr pane list` answers
  `{"result":{"panes":[]}}`, `plugin pane open` succeeds, and the second `pane list` answers
  with `{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8"}`
- **THEN** exactly four Herdr calls are made, in order: `["pane","list"]`,
  `["plugin","pane","open",…,"--placement","tab",…]`, `["pane","list"]`, then
  `["plugin","pane","focus","w8:pG"]`
- **AND** the process exits 0 with no warning recorded

#### Scenario: A split open takes the identical path, with no placement branch

- **WHEN** the same sequence runs for `open` (a split placement) instead
- **THEN** the same four calls are made in the same order, differing only in the open call's
  own argument vector
- **AND** the process exits 0

#### Scenario: A failed second listing warns and still reports success

- **WHEN** the open succeeds but the second `pane list` fails with an error envelope at exit 1
- **THEN** exactly three Herdr calls are made — no focus call follows
- **AND** `Report.warnings` carries Herdr's reason verbatim
- **AND** `Report.outcome` is `Ok`, so the process exits 0 with that warning on stderr

#### Scenario: An unparseable second listing warns and still reports success

- **WHEN** the second `pane list` succeeds but its stdout is `not json`, and again when it is
  valid JSON carrying no `result.panes` array
- **THEN** in both cases no focus call follows, a warning is recorded, and the process exits 0

#### Scenario: An empty second listing warns rather than focusing nothing

- **WHEN** the second `pane list` answers `{"result":{"panes":[]}}`, so no dashboard pane is
  identified
- **THEN** no focus call is made, and no call is made with an empty pane id
- **AND** a warning is recorded naming the workspace id `w8` and that no dashboard pane was
  identified, so the assertion has a subject rather than only a count
- **AND** the process exits 0

#### Scenario: A post-open focus failure warns, on every code, including the one that stops the pre-open focus

- **WHEN** the post-open `plugin pane focus` fails with the `plugin_pane_not_found` envelope
  at exit **1** — the shape that *stops* the command when it happens to an already-listed
  pane — and again when it fails as a usage error at exit **2**, and again with
  `CliError::NotStarted`
- **THEN** in all three cases `Report.outcome` is `Ok` and the process exits 0
- **AND** `Report.warnings` carries the reason verbatim in each

#### Scenario: The post-open focus names the newly opened pane, not the one already listed

This is the only scenario in which `run` passes a **non-empty** pre-open id list to the
chooser, and so the only one that can falsify an implementation that hardcodes an empty one.
Every other scenario either starts from an empty listing or ends with `before` and `after`
equal, where the chooser's fallback returns the same pane either way.

- **WHEN** the first `pane list` carries `{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8"}`,
  `plugin pane focus w8:pG` fails as a usage error at exit **2** so the command falls through
  and opens once, `plugin pane open` succeeds, and the second `pane list` carries `w8:pG`
  **and then** `w8:pH`, both matching
- **THEN** five Herdr calls are made, and the fifth is `["plugin","pane","focus","w8:pH"]`
- **AND** it is not `["plugin","pane","focus","w8:pG"]` — the pane already known not to be
  focusable, which is what an implementation ignoring the pre-open ids would raise
- **AND** the process exits 0 with the usage-error warning recorded

#### Scenario: A failed open makes no post-open call at all

- **WHEN** `plugin pane open` fails
- **THEN** exactly two Herdr calls are made — `pane list` then `plugin pane open`
- **AND** no second listing and no focus call follow
- **AND** the process exits 1, unchanged by this change

#### Scenario: The already-open path is untouched

- **WHEN** the first `pane list` already reports a dashboard pane for this workspace and
  `plugin pane focus` on it succeeds
- **THEN** exactly two Herdr calls are made, exactly as before this change
- **AND** no open call and no second listing follow

### Requirement: The pane to focus after opening is identified by difference from the pre-open listing

The pane to focus SHALL be identified by comparing the post-open listing against the
dashboard panes the **pre-open** listing reported, never by reading the open call's own
response. A post-open entry SHALL count as a dashboard pane on `existing_pane`'s same
three-part test, with one clause made explicit: a **non-empty** string `pane_id`, a `label`
equal to `open::DASHBOARD_LABEL`, and a `workspace_id` equal to the context's workspace id.

`existing_pane` accepts a `""` `pane_id` today — `src/open.rs:116` reads
`obj.get("pane_id").and_then(|v| v.as_str())` with no emptiness filter — while the live
`specs/pane-open/spec.md` already promises that "no focus call is made with an empty or
non-string pane id". That promise has no proving fixture at HEAD (`grep -n '"pane_id":""'
src/open.rs tests/*.rs` matches nothing) and the post-open focus is a **second** site that
would otherwise issue `plugin pane focus ""`. Because the extractor becomes the single
matcher (see below), one non-empty guard makes the existing promise true and keeps the new
site from repeating the gap.

The **first** post-open dashboard pane, in the listing's own order, whose `pane_id` was not
among the pre-open dashboard pane ids SHALL be chosen. When every post-open match was already
present before the open, or the pre-open listing produced no ids at all — it failed, was
unparseable, or reported no dashboard pane — the **first** post-open match SHALL be chosen.
When the post-open listing carries no match at all, no pane SHALL be chosen.

Difference is what makes this correct on the one path where a dashboard pane already existed
and one was opened anyway: the usage-error fall-through above, where `plugin pane focus`
failed at exit 2 and the command opened once regardless. Focusing the first match there would
raise the pane that was already known not to be focusable.

This decision SHALL be expressed as **two pure functions**, so every case below is unit-tested
with no Herdr and no process:

- an **extractor** from a listing and a workspace id to every dashboard pane id it carries, in
  the listing's own order, returning an error for a listing that is not JSON or carries no
  `result.panes` array. `existing_pane` SHALL be re-expressed in terms of it — its first
  element — so the three-part test exists in exactly one place and the pre-open and post-open
  listings cannot drift apart.
- a **chooser**, total over two id lists, returning the first `after` id absent from `before`,
  else `after`'s first element, else nothing.

#### Scenario: A pane absent before and present after is the one chosen

- **WHEN** the pre-open ids are empty and the post-open listing carries
  `{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8"}`
- **THEN** `w8:pG` is chosen

#### Scenario: A pre-existing dashboard is not chosen when a new one appears

- **WHEN** the pre-open ids are `["w8:pG"]` and the post-open listing carries `w8:pG` first
  and then `w8:pH`, both labelled `OpenSpec` in workspace `w8`
- **THEN** `w8:pH` is chosen, not the first match in listing order

#### Scenario: Two new matches choose the first in post-open order

- **WHEN** the pre-open ids are empty and the post-open listing carries `w8:pG` then `w8:pH`,
  both matching
- **THEN** `w8:pG` is chosen

#### Scenario: Every match already present falls back to the first

- **WHEN** the pre-open ids are `["w8:pG"]` and the post-open listing carries only `w8:pG`
- **THEN** `w8:pG` is chosen rather than nothing, because a dashboard for this workspace is
  what the user asked to be shown

#### Scenario: A pre-open id that has since vanished is ignored

- **WHEN** the pre-open ids are `["w8:pZ"]` — a pane closed between the two listings — and the
  post-open listing carries only `w8:pG`
- **THEN** `w8:pG` is chosen, and the vanished id changes nothing

#### Scenario: No post-open match chooses nothing

- **WHEN** the extractor is given `{"result":{"panes":[]}}` for workspace `w8`, and again a
  listing whose only entries carry no `label`, and again one whose only labelled entry reports
  `"workspace_id":"wA"`, and again one whose only otherwise-matching entry carries no string
  `pane_id`, and again one whose only otherwise-matching entry carries
  `{"pane_id":"","label":"OpenSpec","workspace_id":"w8"}`
- **THEN** it yields an empty id list in all five cases
- **AND** the chooser given any `before` and an empty `after` returns nothing
- **AND** `existing_pane` returns no match for that same empty-`pane_id` listing, which it
  does not do at HEAD

#### Scenario: An unparseable listing is an error both matchers report identically

- **WHEN** the extractor is given `not json`, and again valid JSON with no `result.panes`
  array
- **THEN** it returns an error naming the reason in both cases
- **AND** `existing_pane` returns that same error for the same input, because it is expressed
  through the extractor rather than repeating the three-part test

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
- **AND** neither recorded vector carries `--cwd`
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
