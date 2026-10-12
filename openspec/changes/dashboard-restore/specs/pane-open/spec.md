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
its own slot", with the replacement opened `--focus`; no `plugin pane focus` call names the
stale pane. A successful replacement open is a successful open: "A newly opened dashboard
pane is focused after opening" then applies unchanged — the second `pane list`, and a
`plugin pane focus` on the pane it identifies — because `--focus` alone does not raise a pane
in a background tab. The stale pane's id is among the pre-open ids and is gone from the
second listing, so "The pane to focus after opening is identified by difference from the
pre-open listing" identifies the replacement. If the replacement open fails, the command
stops exactly as a failed `plugin pane open` does (exit 1, Herdr's reason verbatim, the stale
pane not closed, no post-open call). If only the close fails, the failure is a warning and the
post-open listing and focus still follow. A match that is not stale, or whose classification
call fails or answers with something unparseable, SHALL be focused exactly as before — an
unclassifiable pane is never replaced, and the classification failure is a warning. Only the
first match is classified; later matches are neither classified nor replaced by `open`.

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
- **AND** the second `pane list` reports `w8:pK` labelled `OpenSpec` in `w8` and no `w8:pG`
- **THEN** the calls after the classification are
  `["plugin","pane","open","--plugin","herdr-openspec","--entrypoint","dashboard","--placement","split","--target-pane","w8:pG","--direction","right","--focus"]`,
  `["pane","close","w8:pG"]`, `["pane","list"]`, then `["plugin","pane","focus","w8:pK"]`
- **AND** no `["plugin","pane","focus","w8:pG"]` call is made
- **AND** the process exits 0

#### Scenario: A failed replacement open stops the command and closes nothing

- **WHEN** the first match `w8:pG` classifies as stale and the replacement `plugin pane open`
  exits 1 with `{"error":{"code":"plugin_pane_open_failed"}}`
- **THEN** no `pane close`, no second `pane list`, and no focus call follow
- **AND** the process exits 1 with Herdr's stderr verbatim

#### Scenario: A failed close after a successful replacement warns and still focuses

- **WHEN** the replacement open succeeds and `pane close w8:pG` exits 1
- **THEN** the second `pane list` and the focus of the pane it identifies still follow
- **AND** the close failure is a warning and the process exits 0

#### Scenario: An unclassifiable match is focused, never replaced

- **WHEN** the first match is `w8:pG` and `herdr pane process-info --pane w8:pG` exits
  non-zero, and again when it answers with JSON carrying no `foreground_processes` array
- **THEN** in both cases the next call is `["plugin","pane","focus","w8:pG"]`
- **AND** no `plugin pane open` and no `pane close` call is made
- **AND** the classification failure is written to stderr as a warning naming `w8:pG`, and
  the process exits 0
- **AND** when instead `process-info` answers with a live dashboard (`"name":"herdr-openspec"`)
  no warning is recorded

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

`open` SHALL parse **only** `pane list`'s and `pane process-info`'s output — both of its
`pane list` calls and the one classification of its first match (`dashboard-restore`'s
addition) — and nothing else. The `plugin pane open` response — measured
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
- **AND** `pane process-info` classified it as not stale
- **THEN** exactly three Herdr calls were made — `pane list`, `pane process-info`, then
  `plugin pane focus`
- **AND** no `plugin pane open` call follows
- **AND** the process exits 1 with the reason on stderr

#### Scenario: A usage-error focus warns and opens once

- **WHEN** a matching pane was listed but `plugin pane focus` fails with plain text at exit
  **2** — what a Herdr that does not know the subcommand produces
- **AND** `pane process-info` classified it as not stale
- **THEN** the first four Herdr calls are `pane list`, `pane process-info`,
  `plugin pane focus`, then `plugin pane open`, and the post-open listing and focus of "A newly opened dashboard pane is
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

After a `plugin pane open` that succeeds — including the replacement open of a stale match
(`dashboard-restore`'s addition), after its `pane close` — `open::run` SHALL issue `herdr pane list` a second
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

This and the stale-match replacement (`dashboard-restore`'s addition) are the only scenarios
in which `run` passes a **non-empty** pre-open id list to the chooser, and so the only ones
that can falsify an implementation that hardcodes an empty one.
Every other scenario either starts from an empty listing or ends with `before` and `after`
equal, where the chooser's fallback returns the same pane either way.

- **WHEN** the first `pane list` carries `{"pane_id":"w8:pG","label":"OpenSpec","workspace_id":"w8"}`,
  `pane process-info` classifies it as not stale, `plugin pane focus w8:pG` fails as a usage
  error at exit **2** so the command falls through and opens once, `plugin pane open`
  succeeds, and the second `pane list` carries `w8:pG` **and then** `w8:pH`, both matching
- **THEN** six Herdr calls are made, and the sixth is `["plugin","pane","focus","w8:pH"]`
- **AND** it is not `["plugin","pane","focus","w8:pG"]` — the pane already known not to be
  focusable, which is what an implementation ignoring the pre-open ids would raise
- **AND** the process exits 0 with the usage-error warning recorded

#### Scenario: A failed open makes no post-open call at all

- **WHEN** `plugin pane open` fails
- **THEN** exactly two Herdr calls are made — `pane list` then `plugin pane open`
- **AND** no second listing and no focus call follow
- **AND** the process exits 1, unchanged by this change

#### Scenario: The already-open path is untouched

- **WHEN** the first `pane list` already reports a dashboard pane for this workspace,
  `pane process-info` classifies it as not stale, and `plugin pane focus` on it succeeds
- **THEN** exactly three Herdr calls are made — `pane list`, `pane process-info`, then
  `plugin pane focus`
- **AND** no open call and no second listing follow

## REMOVED Requirements

### Requirement: `open` and `open-tab` are subcommands of their own, with no flag grammar

**Reason**: Replaced by "`open`, `open-tab`, and `restore` are subcommands of their own, with no flag grammar". The requirement's own title names the subcommands, and adding `restore` changes that title, which a MODIFIED block cannot do.

**Migration**: None for users. The `parse`/`usage` tests asserting three commands move to the replacing requirement's four-command scenarios.
