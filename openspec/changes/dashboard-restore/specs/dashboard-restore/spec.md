## ADDED Requirements

### Requirement: A stale dashboard pane is recognised from its process, not its label

A Herdr server restart keeps a dashboard pane's `pane_id`, its `OpenSpec` label, and its
workspace, and replaces its process with a fresh shell (`notes/measurements.md` M2). The
label alone therefore cannot tell a live dashboard from a dead one, and neither can
`shell_pid`: a live dashboard is spawned with no shell above it, so it reports its own pid
as `shell_pid` too (M5).

A **candidate** SHALL be a `herdr pane list` entry carrying a non-empty string `pane_id` and
a `label` equal to `open::DASHBOARD_LABEL`, in any workspace. A candidate SHALL be **stale**
exactly when `herdr pane process-info --pane <pane_id>` answers with JSON whose
`result.process_info` carries an integer `shell_pid` and a `foreground_processes` array of
exactly one element, and that element's integer `pid` equals `shell_pid` and it carries a
string `name` other than `herdr-openspec` (the binary name `tests/manifest.rs` already binds to the manifest).
Every other answer — more than one foreground process, a different pid, the dashboard's own
name, a missing or mistyped field, unparseable JSON, or a failed call — SHALL classify the
candidate as **not stale**. Classification is a pure function of the `process-info` stdout;
it performs no I/O of its own.

#### Scenario: A restored shell is stale

- **WHEN** `process-info` answers with `"shell_pid":49095` and one foreground process
  `{"name":"zsh","argv":["-zsh"],"pid":49095}`
- **THEN** the candidate is stale

#### Scenario: A live dashboard is not stale

- **WHEN** `process-info` answers with `"shell_pid":38099` and one foreground process
  `{"name":"herdr-openspec","argv":["/plugins/herdr-openspec/target/release/herdr-openspec","ui"],"pid":38099}`
- **THEN** the candidate is not stale

#### Scenario: A command the user started in the restored shell is not stale

- **WHEN** `process-info` answers with `"shell_pid":49095` and one foreground process
  `{"name":"vim","pid":49210}`
- **THEN** the candidate is not stale
- **AND** the same holds when `foreground_processes` holds two elements, one of which has
  `pid` `49095`

#### Scenario: An unreadable answer is never stale

- **WHEN** the `process-info` stdout is not JSON, and again when it is JSON with no
  `shell_pid`, with a string `shell_pid`, with no `foreground_processes`, with an empty
  `foreground_processes` array, or with a sole foreground process carrying no `name`
- **THEN** in every case the candidate is not stale

#### Scenario: An unlabelled pane is never a candidate

- **WHEN** `herdr pane list` lists a pane with no `label`, a pane labelled `Files`, and a
  pane labelled `OpenSpec` whose `pane_id` is the empty string
- **THEN** none of the three is a candidate and no `process-info` call names any of them

### Requirement: A stale dashboard pane is replaced in its own slot

Replacing a stale pane `X` SHALL be exactly two Herdr calls, in this order:

1. `herdr plugin pane open --plugin <plugin id> --entrypoint dashboard --placement split
   --target-pane X --direction right <focus flag>`, where `<plugin id>` is `HERDR_PLUGIN_ID`
   (falling back to `herdr-openspec`, on `open::context`'s existing terms) and
   `<focus flag>` is `--focus` when the caller asks for focus and `--no-focus` otherwise;
2. `herdr pane close X`.

Splitting `X` and then closing it leaves the new pane in `X`'s exact rectangle, in `X`'s tab,
for a split dashboard and a tab dashboard alike (M3). The `dashboard` entrypoint is used for
both, because the operation is always a split of `X`; the manifest's `dashboard-tab` entry
is never named. `--cwd` SHALL NOT be passed, on `pane-open`'s existing terms.

If the open call fails, `X` SHALL NOT be closed: a stale shell left in place costs the user
one manual step, while a closed pane with nothing opened beside it loses the slot. If the
close call fails after a successful open, the replacement stays open and the failure is
reported as a warning.

#### Scenario: The full call sequence for one stale pane

- **WHEN** stale pane `w1:p3` is replaced with `HERDR_PLUGIN_ID=herdr-openspec`, focus not
  requested
- **THEN** the calls are
  `["plugin","pane","open","--plugin","herdr-openspec","--entrypoint","dashboard","--placement","split","--target-pane","w1:p3","--direction","right","--no-focus"]`
  then `["pane","close","w1:p3"]`
- **AND** neither call contains `--cwd`

#### Scenario: A failed open closes nothing

- **WHEN** the `plugin pane open` call exits non-zero with stderr `plugin_pane_open_failed`
- **THEN** no `pane close` call is made
- **AND** the failure, carrying Herdr's stderr verbatim, is reported

#### Scenario: A failed close keeps the replacement and warns

- **WHEN** the open succeeds and `pane close w1:p3` then exits non-zero
- **THEN** the failure is reported as a warning and not as the operation's error

### Requirement: `restore` replaces every stale dashboard pane in the session

`herdr-openspec restore`, run by Herdr as the manifest's `[[startup]]` hook, SHALL make one
`herdr pane list` call, classify every candidate in the listing's own order, and replace
each stale one. It SHALL request focus for a replacement exactly when the listing reports
the stale pane `"focused": true`, so a restart does not move the user's focus. A candidate
that is not stale SHALL be left untouched, so a live dashboard surviving a live handoff — when
Herdr runs startup hooks again — is never replaced. One candidate's failure SHALL NOT stop the
sweep.

`restore` SHALL NOT require a workspace id: Herdr's startup environment names no workspace,
and the sweep is session-wide. It starts no thread, never renders, and needs no terminal.

It SHALL write every warning and failure to stderr, one line each, naming the pane id it
concerns, and SHALL exit 0 whenever the listing succeeded — Herdr records a startup hook's
output in its plugin command log and does not stop the server on a failed hook, so the exit
status carries only whether the sweep could start. It SHALL exit 1 when `herdr pane list`
itself fails or its stdout carries no `result.panes` array, making no further call.

#### Scenario: Two stale dashboards in two workspaces are both replaced

- **WHEN** `herdr pane list` lists `w1:p2` (labelled `OpenSpec`, `"focused": true`) and
  `w2:p5` (labelled `OpenSpec`, not focused), and both classify as stale
- **THEN** `w1:p2` is replaced with `--focus` and then `w2:p5` with `--no-focus`
- **AND** the process exits 0 with nothing on stderr

#### Scenario: A live dashboard is left alone

- **WHEN** the only candidate's `process-info` reports a foreground process named
  `herdr-openspec`
- **THEN** the only calls are `["pane","list"]` and the one `process-info` call
- **AND** the process exits 0

#### Scenario: No dashboard pane at all

- **WHEN** `herdr pane list` answers with `"panes": []`, and again with panes none of which
  carries the `OpenSpec` label
- **THEN** `["pane","list"]` is the only call made, and the process exits 0

#### Scenario: One failed replacement does not stop the sweep

- **WHEN** two stale candidates are listed, `w1:p2` then `w1:p3`, and the open call for
  `w1:p2` fails
- **THEN** `w1:p3` is still classified and replaced
- **AND** stderr carries one line naming `w1:p2` and Herdr's stderr, and the process exits 0

#### Scenario: Herdr unreachable

- **WHEN** `herdr pane list` cannot be started, exits non-zero, or answers with JSON lacking
  `result.panes`
- **THEN** no further Herdr call is made
- **AND** stderr carries the reason and the process exits 1

#### Scenario: No workspace in the environment is not a failure

- **WHEN** `restore` runs with neither `HERDR_WORKSPACE_ID` nor `HERDR_PLUGIN_CONTEXT_JSON`
  set
- **THEN** the sweep runs exactly as it does with them set
