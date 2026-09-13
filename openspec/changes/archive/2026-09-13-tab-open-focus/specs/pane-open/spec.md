## ADDED Requirements

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

## MODIFIED Requirements

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
all, so its shape cannot break this command. This holds after the post-open focus was added:
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
