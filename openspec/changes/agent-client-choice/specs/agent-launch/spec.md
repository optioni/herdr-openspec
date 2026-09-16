## ADDED Requirements

### Requirement: An ambiguous agent kind stops the launch before any pane is split

`integration::resolve` returns `Choice::Ambiguous` when nothing is configured, nothing is
recorded, and **two or more** integrations are installed. It produces no kind, so there is no
`--kind` value for `agent start` to carry, and the launch SHALL stop rather than pick one.

The worker SHALL end the request at that point, before `pane split`, and report an `Outcome`
with:

- `named` `None` — no agent was started, so nothing may reach `Dashboard::agent_names`, on
  exactly the failed-split path's terms;
- `problems` holding **exactly one** entry, naming every installed kind in Herdr's own printed
  order and telling the reader to set `agent_kind`.

The entry SHALL be **front-loaded**: the key to set and the kinds to choose between come
first, before any explanation. This row is the whole answer to the key the reader just
pressed, and `pad_or_truncate_right` cuts it to the list region's **38**-column interior at
the wide layout — an explanation-first wording put both candidate kinds past that cut, so the
row said nothing the reader could act on at either mandated width. Both kinds and the key
SHALL survive the cut at 38 and at 58 for a two-kind fixture.

No Herdr call beyond `integration status` SHALL be issued: no pane is created, so none can be
left behind. `Dashboard::launch.in_flight` SHALL be cleared by the same `drain` that adopts the
outcome, on the established lifecycle — an ambiguous resolution SHALL NOT lock the launch keys
for the rest of the session.

This is the one resolution outcome that stops a launch, and it is not a contradiction of
"never fail closed": the evidence exists and points two ways at once, and choosing between two
clients the reader has both set up is exactly the guess `agent-attribution` refuses to make
about a terminal title. Where there is no evidence at all, `LastResort` launches `claude` and
warns. `settings-window` upgrades this row to an interactive picker; until then the row is the
answer.

The other three keys SHALL be unaffected: `g` still focuses, and the list, filter, and detail
routes are untouched — a launch that could not choose a client is not a broken pane.

#### Scenario: Two installed integrations stop the launch with one problem and no pane

- **WHEN** a launch runs against a scratch `herdr` program whose `integration status` reports
  both `claude` and `codex` as installed, with no `agent_kind` configured and no recorded kind
- **THEN** the invocation log holds exactly one entry, `integration status`
- **AND** no `pane split` entry appears, so no pane is created and none can be orphaned
- **AND** `outcome.named` is `None` and `outcome.problems` holds exactly one entry naming
  `claude`, `codex`, and `agent_kind`
- **AND** the scratch state directory is byte-identical to before the launch

#### Scenario: The ambiguous refusal clears the in-flight flag and leaves `g` working

- **WHEN** the outcome above is adopted by a `Dashboard` whose `launch.in_flight` was `true`
- **THEN** `launch.in_flight` is `false` afterwards
- **AND** a subsequent `g` with an attributed agent still returns
  `Decision::Go(Request::Focus { .. })`
- **AND** a subsequent `a` is not refused by the in-flight guard, so the launch keys are not
  locked for the rest of the session. It answers with the **same** refusal, re-derived from
  the cached `Choice`: configuration is read once per process and the choice is deliberately
  not invalidated, so acting on the row means editing `config.toml` and restarting the pane

#### Scenario: The ambiguous stop renders as one leading problem row at both widths

- **WHEN** that `Dashboard` is rendered at 120x20 and 60x20
- **THEN** the list region's interior row 0 begins `! ` and names both installed kinds and
  `agent_kind` at **both** widths — which is what the front-loading above is for, since the
  38-column interior cuts everything after roughly the first thirty-six columns
- **AND** the row is exactly the interior width — 38 at 120 and 58 at 60 — on the same
  `pad_or_truncate_right` terms as every other row
- **AND** the change list is drawn below it, so the pane stays usable

#### Scenario: A configured kind suppresses the stop entirely

- **WHEN** the same two-installed status is answered with `agent_kind = "codex"` configured
- **THEN** the launch proceeds and the log holds all four entries
- **AND** `outcome.problems` is empty, so the stop is caused by the absence of a choice and by
  nothing else in the fixture

## MODIFIED Requirements

### Requirement: The launch decision is a pure, total function that refuses before it reaches Herdr

`launch::decide` SHALL hold the whole policy of what a launch key does, and SHALL be a pure
total function of its seven arguments:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent { Apply, Continue, Archive, Focus }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Launch { change: String, agent: String, intent: Intent },
    Focus { pane_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision { Nothing, Refuse(String), Go(Request) }

pub fn decide(
    intent: Intent,
    change: Option<&str>,
    pane: Option<&str>,
    reachable: bool,
    live_names: &[&str],
    in_flight: bool,
    file_mode: bool,
) -> Decision;
```

It SHALL perform no filesystem, process, environment, network, or terminal I/O, SHALL read no
clock and no global state, SHALL spawn nothing, and SHALL never panic for any combination of
arguments, including an empty `change`, an empty `pane`, an empty `live_names`, and every
`Intent`.

`Decision::Nothing` means the key does nothing at all — no Herdr call, no problem row, no state
change beyond clearing nothing. `Decision::Refuse` means the key is answered with a problem
string and no Herdr call whatsoever. `Decision::Go` means the request is handed to the launcher.

The order of decision SHALL be exactly:

1. `reachable == false` → `Nothing`. An unreachable socket is the documented standalone-TUI
   state; the action keys are not offered, so pressing one is not an error.
2. `Intent::Focus` with `pane` `None` → `Nothing`; with `pane` `Some(p)` →
   `Go(Request::Focus { pane_id: p })`. Focus is exempt from every test below: it
   splits no pane, starts no agent, sends no prompt, needs no `openspec` binary, and is
   exactly what a user waiting on a slow launch should still be able to press.
3. `file_mode == true` → `Refuse`, naming that no `openspec` binary was found and that
   `a`, `c`, and `s` cannot tell an agent how to run the workflow without one.
4. `change` `None` → `Nothing`. There is nothing selected to launch onto.
5. `in_flight == true` → `Refuse`, naming that a launch is already running and to wait.
6. The derived agent name, `state::agent_name(change)`, appearing in `live_names` →
   `Refuse`, naming the derived name and pointing at `g`.
7. Otherwise → `Go(Request::Launch { change, agent, intent })`.

Step 3 is new. In file mode the prompt this plugin would send names an absolute `openspec`
path it does not have, and the measurement in `agent-prompts` shows the launched agent's own
shell will not resolve `openspec` either — so there is no prompt that could work. It precedes
step 4 deliberately: pressing `a` in file mode says why whether or not anything is selected,
which is the more useful and the more deterministic answer. The header already badges
`file mode`, and the footer drops the `a/c/s launch` hint, so the refusal confirms a context
that is already on screen rather than introducing it.

Step 3 follows step 2 rather than preceding it, because `g` needs no binary.

Step 5 closes a window the `live_names` test at step 6 cannot: `live_names` comes
from the **last** `herdr agent list` snapshot, and `herdr agent start` is measured to block
for up to **thirty seconds** waiting for interactive readiness. Between the press and the
agent appearing in a poll, `live_names` does not contain the derived name, the launcher's
request channel is unbounded, and `Dashboard::launch.pending` is a one-shot slot drained
every iteration — so a second press runs a **second** full `pane split` + `agent start` +
`agent prompt` sequence in series behind the first. The observable damage is a stray pane
that nothing ever closes plus an `agent start` that collides on the duplicate name. The
vulnerable window is the whole launch duration plus poll lag, not the one-second poll
interval.

Step 5 precedes step 6 because an in-flight launch is the more specific and more recent
fact; a name already live is a standing condition the poller reports.

`decide` SHALL NOT consult a terminal title, an agent kind, a pane's working directory, or any
field other than the seven arguments above. In particular it SHALL NOT consult the resolved
agent kind: the kind is resolved lazily on the worker thread after the decision is made, so
`decide` stays reachable with no `Dashboard`, no `ChangeSet`, and no fixture.

#### Scenario: An unreachable socket makes every action key inert

- **WHEN** `decide` is called with `reachable` `false`, `change` `Some("add-auth")`, `pane`
  `Some("w8:p3")`, an empty `live_names`, `in_flight` `false`, and `file_mode` `false`, once
  for each of `Apply`, `Continue`, `Archive`, and `Focus`
- **THEN** all four return `Decision::Nothing`
- **AND** no `Request` is produced on any of the four calls, so no Herdr call can follow and no
  problem row is produced either — an unreachable socket is silent, not a fault
- **AND** the same four calls with `file_mode` `true` also return `Decision::Nothing`, so an
  unreachable socket still outranks file mode and a pane with neither is silent rather than
  doubly noisy

#### Scenario: File mode refuses the three launch keys and names the missing binary

- **WHEN** `decide` is called with `reachable` `true`, `file_mode` `true`, `change`
  `Some("2fa-support")`, an empty `live_names`, and `in_flight` `false`, once for each of
  `Apply`, `Continue`, and `Archive`
- **THEN** all three return `Decision::Refuse(reason)` whose text names the absent `openspec`
  binary
- **AND** no `Request` is produced on any of the three, so no pane is split and no agent is
  started in a pane that could not be told what to do
- **AND** the same three calls with `change` `None` return the same three refusals, because
  step 3 precedes the selection test
- **AND** the same three calls with `file_mode` `false` return `Decision::Go`, so the refusal
  is caused by the flag and by nothing else in the fixture

#### Scenario: Focus is exempt from file mode

- **WHEN** `decide` is called with `Intent::Focus`, `reachable` `true`, `file_mode` `true`,
  `pane` `Some("w8:p3")`, and `change` `None`
- **THEN** it returns `Decision::Go(Request::Focus { pane_id: "w8:p3" })`
- **AND** `g` therefore works in a pane with no `openspec` binary, because focusing an agent
  that is already running sends no prompt and needs no path

#### Scenario: No selected change means no launch, and no agent means no focus

- **WHEN** `decide` is called with `reachable` `true`, `change` `None`, `pane` `None`, an
  empty `live_names`, `in_flight` `false`, and `file_mode` `false`, for `Apply`, `Continue`,
  and `Archive`
- **THEN** all three return `Decision::Nothing`
- **AND** the same call for `Focus` also returns `Decision::Nothing`
- **AND** the call for `Focus` with `pane` `Some("w8:p3")` and `change` `None` returns
  `Decision::Go(Request::Focus { pane_id: "w8:p3" })`, so focusing needs a pane and not a
  selection

#### Scenario: Each launch intent carries its own change and derived name

- **WHEN** `decide` is called with `reachable` `true`, `change` `Some("2fa-support")`, `pane`
  `None`, an empty `live_names`, `in_flight` `false`, and `file_mode` `false`, once for each
  of `Apply`, `Continue`, and `Archive`
- **THEN** each returns `Decision::Go(Request::Launch { change: "2fa-support", agent:
  "c-2fa-support", intent })` with that call's own `intent`
- **AND** the three results differ **only** in `intent`, so the change and the derived name are
  computed once and identically for all three keys
- **AND** none of the three carries an agent kind: the kind is not a field of `Request` and is
  resolved on the worker thread, after this decision

#### Scenario: A derived name already live in the session is refused before any Herdr call

- **WHEN** `decide` is called with `reachable` `true`, `change` `Some("2fa-support")`,
  `in_flight` `false`, `file_mode` `false`, and `live_names` `["c-2fa-support", "other"]`
- **THEN** it returns `Decision::Refuse(reason)` where `reason` names `c-2fa-support` and tells
  the reader to press `g`
- **AND** no `Request` is produced, so no pane is split and no stray pane can be left behind —
  the refusal is what keeps the common repeat-press from leaking a pane
- **AND** the same call with `live_names` `["c-2fa-support-x", "2fa-support"]` returns
  `Decision::Go`, because the match is on the **derived** name byte for byte and neither of
  those is it

#### Scenario: A second press while a launch is in flight is refused, not queued

- **WHEN** `decide` is called with `reachable` `true`, `change` `Some("2fa-support")`, an
  **empty** `live_names` — the poller has not yet seen the agent, which is the whole point —
  `file_mode` `false`, and `in_flight` `true`, once for each of `Apply`, `Continue`, and
  `Archive`
- **THEN** all three return `Decision::Refuse(reason)` whose text says a launch is already
  running and to wait
- **AND** no `Request` is produced on any of the three, so no second `pane split` can run and
  no stray pane is left behind
- **AND** the same three calls with `in_flight` `false` return `Decision::Go`, so the refusal
  is caused by the flag and by nothing else in the fixture

#### Scenario: Focus still works while a launch is in flight

- **WHEN** `decide` is called with `Intent::Focus`, `reachable` `true`, `pane`
  `Some("w8:p3")`, `file_mode` `false`, and `in_flight` `true`
- **THEN** it returns `Decision::Go(Request::Focus { pane_id: "w8:p3" })`
- **AND** the user waiting through a thirty-second `agent start` can still press `g` to look
  at an agent already running, which is the behaviour the in-flight guard must not take away

#### Scenario: Every combination is total

- **WHEN** `decide` is called with `change` `Some("")`, `pane` `Some("")`, `reachable` `true`,
  `live_names` `[""]`, and `in_flight` both `false` and `true`, crossed with `file_mode` both
  `false` and `true`, once per `Intent` in each case
- **THEN** none of the sixteen panics
- **AND** with `in_flight` `false` and `file_mode` `false`, `Apply` returns `Decision::Go`
  carrying the derived name `change` — `state::agent_name` maps the empty string to that
  fallback — while `Focus` returns `Decision::Go(Request::Focus { pane_id: "" })`, both of
  which the launcher then fails on honestly rather than the decision guessing on the caller's
  behalf
- **AND** with `in_flight` `true` and `file_mode` `false`, `Apply` returns `Decision::Refuse`
  and `Focus` still returns `Decision::Go`
- **AND** with `file_mode` `true`, `Apply`, `Continue`, and `Archive` all return
  `Decision::Refuse` whatever `in_flight` is, while `Focus` still returns `Decision::Go`

### Requirement: A launch is exactly three Herdr calls, in order, the second and third carrying the first's output

`launch`'s worker SHALL issue, through `cli::HerdrCli` and nothing else, exactly these argument
vectors, in this order, stopping at the first that fails:

1. `["pane", "split", "--cwd", <repository root>, "--direction", "right", "--no-focus"]`
2. `["agent", "start", <derived name>, "--kind", <resolved kind>, "--pane", <pane id from 1>]`
3. `["agent", "prompt", <derived name>, <prompt text>]`

`--direction` is not optional: measured against Herdr 0.8.2, omitting it exits **2** with
`usage: herdr pane split [<pane_id>|--pane ID|--current] --direction right|down …` and creates
no pane. No pane argument is given, and that is deliberate: measured, `pane split` with no pane
argument splits the **focused** pane, which is the dashboard's own pane whenever one of its keys
was pressed. `--no-focus` keeps the reader in the dashboard, which is what makes `g` useful.

The launch is still exactly these three calls. Exactly one further Herdr call,
`["integration", "status"]`, SHALL precede the **first** launch of a session and no other, as
`integration-status` specifies: it resolves the kind call 2 carries, its answer is cached for
the rest of the process, and it is not part of the launch sequence. A `Request::Focus` SHALL
issue neither it nor these three.

The prompt text SHALL be `agent-prompts`' CLI-driven shape for the intent, built from the
resolved absolute `openspec` path and the change name and overridable per kind from
`config.toml` — each a **single** argument vector element. The seam passes arguments to the
program directly with no shell, so no quoting is applied and none is needed. The `/opsx:apply`,
`/opsx:continue`, and `/opsx:archive` texts are **removed**: they are a Claude Code plugin's
shortcut rather than a universal idea and fail in any Claude Code without the `opsx` plugin
installed, while the CLI shape works in every client that can run a shell command. `--wait`
SHALL NOT be passed: measured, an un-waited `agent prompt` returns as soon as the text is
submitted, and waiting would hold the worker for the length of the agent's turn.

`<resolved kind>` SHALL be `integration::resolve`'s answer — the five-step precedence over
`Config::agent_kind`, the recorded choice under `HERDR_PLUGIN_STATE_DIR`, and the installed
integrations — and SHALL NOT be `Config::agent_kind` read directly. `Config::agent_kind` no
longer has a documented default of `claude`; it is an override, absent when unset. The literal
`claude` SHALL live in `src/integration.rs` and SHALL NOT appear in a production file anywhere
under `src/ui/`.

The repository root SHALL be rendered with `Path::to_string_lossy`, so a non-UTF-8 root is
passed lossily rather than failing the launch.

The scenario titled "Each intent sends its own `/opsx:*` command and nothing else" keeps that
title verbatim although its body now asserts the CLI shape. A `MODIFIED` block replaces the
whole requirement, so a renamed scenario is indistinguishable from a dropped one and
`openspec validate --strict` refuses it; the title is therefore read as the concern it names —
the per-intent prompt text — and the body is the contract.

#### Scenario: The three calls appear in order with the split's own pane id

- **WHEN** a launch for `2fa-support` with `agent_kind` `codex` and the resolved binary
  `/opt/bin/openspec` runs against a scratch `herdr` program that logs each argument vector,
  answers `integration status` with the seventeen measured lines, and answers `pane split` with
  `{"id":"cli:pane:split","result":{"pane":{"pane_id":"wD:pJ","tab_id":"wD:t2",
  "workspace_id":"wD"},"type":"pane_info"}}`
- **THEN** the log holds exactly four entries in this order: `integration status`,
  `pane split --cwd <root> --direction right --no-focus`,
  `agent start c-2fa-support --kind codex --pane wD:pJ`, and
  `agent prompt c-2fa-support Run: /opt/bin/openspec instructions apply --change 2fa-support --json. Follow the instruction it returns to implement this OpenSpec change.`
- **AND** the `--pane` value is `wD:pJ`, the id the first launch call's own payload named — a
  launch that passed any other value would be reading the pane id from somewhere other than
  that call
- **AND** `--kind` is `codex`, not `claude`, so the configured kind wins the precedence and is
  threaded through rather than hardcoded
- **AND** the prompt is one argument-vector element containing no quote character and no
  occurrence of `/opsx:`
- **AND** the same launch against a repository root whose bytes are **not valid UTF-8** still
  produces the same four entries, with `--cwd` carrying `Path::to_string_lossy`'s rendering —
  a launch is never refused for a path this plugin cannot spell

#### Scenario: Each intent sends its own `/opsx:*` command and nothing else

- **WHEN** the same launch is run three times for the change `add-auth` against the resolved
  binary `/opt/bin/openspec`, once per intent, within one worker
- **THEN** the `agent prompt` entry is
  `agent prompt add-auth Run: /opt/bin/openspec instructions apply --change add-auth --json. Follow the instruction it returns to implement this OpenSpec change.`,
  then the `Continue` row of `agent-prompts`' table, then its `Archive` row, respectively
- **AND** the prompt text is one argument-vector element in each case, containing no quote
  character
- **AND** the `pane split` and `agent start` entries are byte-identical across all three runs,
  so the intent changes the prompt and nothing else
- **AND** exactly one `integration status` entry appears across all three runs, before the
  first, because the resolution is cached for the session

#### Scenario: An archived change launches on the same terms as an active one

- **WHEN** the selected change is an archived one named `add-auth` and `a` is pressed
- **THEN** `launch.pending` is
  `Some(Request::Launch { change: "add-auth", agent: "add-auth", intent: Apply })` — identical,
  field for field, to what the same press produces on an **active** change of that name
- **AND** the same request run through the worker produces an `agent prompt add-auth` entry
  whose text is `agent-prompts`' `Apply` row for `add-auth`, unchanged
- **AND** the plugin makes no judgement about whether that command suits an archived change:
  deciding what to run next is `PRD.md` → Non-goals' orchestration, which belongs to the agent

### Requirement: A failed call stops the launch at that call and carries Herdr's own reason

Each of the three calls SHALL be checked, and the first failure SHALL end the launch. Herdr
writes its diagnostic to **stderr** as a JSON error envelope and exits non-zero, so
`cli::CliError::Failed`'s `stderr` carries the reason and the reported problem SHALL include it
verbatim, on `agents::herdr_error_problem`'s established terms — unlike the OpenSpec CLI, whose
diagnostic goes to stdout and is therefore unavailable.

The launcher SHALL NOT issue `herdr pane close`, ever. A pane it created and could not use is
left in place with its id named in the problem, because a failed `agent start` includes the
readiness-timeout case, in which an agent may be starting and closing the pane would kill it.

`launch::Outcome::problem: Option<String>` SHALL become `problems: Vec<String>`, and every
reason the worker accumulates SHALL be pushed onto it in the order it occurred.

This is `degraded-states`' repair of a defect `agent-launch` shipped. `SPEC.md` → Degraded
states says of a `state::record` failure after a successful `agent start`: "the record failure
is recorded as its own problem **alongside** the successful launch". A single `Option<String>`
cannot hold two reasons, and the landed worker writes
`match cli.run(&prompt_refs) { Ok(_) => record_problem, Err(err) => Some(herdr_reason(&err)) }`
— so on the one path where **both** fail, the record's reason is silently discarded and the
reader is told only that the prompt failed. The mapping file is wrong, the pane says nothing
about it, and the next session badges nothing.

`Dashboard::launch.problems` is already a `Vec<String>`, and `agent-launch`'s own requirement
that it is "replaced wholesale by the next outcome or refusal, and cleared by a success" is
unchanged: an outcome replaces the vector with its own, however many entries that is. The
claim that it "holds at most two entries" SHALL be restated as **at most four** — the kind
resolution contributes at most two (`integration-status` fixes which two, and summarises
`parse`'s per-line problems into one so the figure does not grow with the input), and the
record failure and the prompt failure are the only other pair that can co-occur — so the
leading-rows cost `change-rows` reasons about is bounded and stated rather than assumed.

The resolution's problems SHALL be pushed **first**, before the three calls run, because they
occurred first: a status read that failed, unparseable output, an absent integration for the
chosen kind, or the last-resort warning all precede `pane split`. A resolution problem SHALL
NOT stop the launch **whenever a kind was produced** — so a launch whose resolution warned and
whose three calls succeeded reports `named` `Some` beside a non-empty `problems`, on exactly
the failed-prompt path's established terms. The one resolution outcome that produces no kind,
`Choice::Ambiguous`, stops the launch instead, and the requirement below owns it.

The three failure points SHALL be distinguishable in what they leave behind:

| Failed call | Pane | Agent | Mapping | Prompt |
|---|---|---|---|---|
| `pane split` | none created | none | not recorded | not sent |
| `agent start` | created, left, id named | none | not recorded | not sent |
| `agent prompt` | created, left | running | **recorded, or its failure named** | not sent |

The scenario titled "A success clears both entries" keeps that title verbatim for the reason
the requirement above gives, although the vector it clears now holds up to three.

#### Scenario: A failed split leaves nothing behind

- **WHEN** the scratch `herdr` program answers `pane split` with exit `1` and
  `{"error":{"code":"pane_split_failed","message":"no space to split"}}` on stderr
- **THEN** the invocation log holds exactly two entries, `integration status` and `pane split`
- **AND** the outcome's problem names `pane_split_failed` and `no space to split`
- **AND** `outcome.named` is **`None`** — no agent was started, so nothing may be recorded and
  nothing may reach `Dashboard::agent_names`. A worker that filled `named` unconditionally would
  badge the change with whatever agent the next poll returned, which is exactly the guess
  `agent-attribution` refuses
- **AND** the scratch state directory is byte-identical to before the launch

#### Scenario: A failed start leaves the pane and names it

- **WHEN** the split succeeds with pane id `wD:pJ` and `agent start` exits `1` with
  `{"error":{"code":"agent_pane_not_found","message":"agent target wD:pJ not found"}}`
- **THEN** the invocation log holds exactly three entries — `integration status`, `pane split`,
  `agent start` — and no fourth
- **AND** the outcome's problem names `agent_pane_not_found`, the derived agent name, and the
  pane id `wD:pJ`
- **AND** `outcome.named` is **`None`**: `agent start` failed, so no agent exists to attribute
- **AND** the log holds no `pane close` entry — the plugin never closes a pane it created

#### Scenario: A failed prompt leaves a running, un-prompted agent that is still attributable

- **WHEN** the split and the start both succeed and `agent prompt` exits `1` with
  `{"error":{"code":"agent_blocked","message":"agent is blocked"}}`
- **THEN** the invocation log holds all four entries
- **AND** `agent-names.toml` holds the derived name mapped to the change, because the mapping is
  recorded between the start and the prompt — the agent exists and the pane must be able to
  badge it even though the prompt did not land
- **AND** the outcome reports **both**: `named` is `Some((derived name, change))`, so the loop
  updates `Dashboard::agent_names`, **and** `problems` holds exactly one entry, naming
  `agent_blocked`. This is one of two paths on which `named` is `Some` beside a non-empty
  `problems` — a resolution warning is the other — and it is what distinguishes "the agent
  exists but was not prompted" from every failure before `agent start`

#### Scenario: A failed recording does not undo a successful start

- **WHEN** the split and the start both succeed and the state directory is a path that cannot be
  created (an existing regular file)
- **THEN** the prompt is still sent — the fourth log entry is present
- **AND** the outcome carries the derived name and change, so the in-memory mapping is correct
  for this session even though the file is not
- **AND** `problems` holds exactly one entry, naming the state directory path and the I/O
  reason, rather than claiming the launch failed

#### Scenario: A failed recording and a failed prompt are both reported

- **WHEN** the split and the start both succeed, the state directory is a path that cannot be
  created (an existing regular file), **and** `agent prompt` then exits `1` with
  `{"error":{"code":"agent_blocked","message":"agent is blocked"}}`
- **THEN** `outcome.problems` holds **two** entries, in the order they occurred: the record
  failure naming the state directory path first, then the prompt failure naming `agent_blocked`
- **AND** `named` is `Some((derived name, change))`: the agent is running and must stay
  attributable, which neither failure changes
- **AND** the list region's first two interior rows, rendered at 120x20 and again at 60x20,
  are `! `-marked and name the state directory and `agent_blocked` in that order — so the pair
  is observable in the pane and not only in the value
- **AND** the landed code returns only the prompt's reason on this path, so this scenario fails
  against `main` before the repair and passes after it

#### Scenario: Two resolution problems, a failed recording, and a failed prompt are all four reported

- **WHEN** `integration status` exits `1` with
  `{"error":{"code":"integration_unavailable","message":"no socket"}}` and nothing is
  configured or recorded, so resolution reports the read failure **and** the last-resort
  warning; the state directory is a path that cannot be created; **and** `agent prompt` then
  exits `1` with `{"error":{"code":"agent_blocked","message":"agent is blocked"}}`
- **THEN** `outcome.problems` holds **four** entries, in the order they occurred: the read
  failure, the last-resort warning, the record failure naming the state directory path, then
  the prompt failure naming `agent_blocked`
- **AND** `named` is `Some((derived name, change))` and `agent start`'s `--kind` is `claude`
- **AND** this is the maximum: no combination produces a fifth entry, because the resolution
  contributes at most one problem per kind and every earlier failure point returns immediately
  with exactly what it has accumulated
- **AND** the same run against a seventeen-line unparseable status still yields four, not
  twenty, because the per-line problems are summarised into one

#### Scenario: A success clears both entries

- **WHEN** a launch in which all three calls, the record, and a resolution that warns about
  nothing succeed follows a launch that left four problems
- **THEN** `launch.problems` is empty afterwards
- **AND** the list's first interior row is a change row again at both widths, so the vector is
  replaced wholesale rather than grown

### Requirement: The launch keys are offered only when the socket is reachable, and type themselves while filtering

The footer SHALL carry two further hints, `a/c/s launch` and `g focus`, in that order,
immediately after `Esc back` and **before** the `<n> unattributed` count. `g focus` SHALL be
present exactly when `Dashboard::agents.reachable` is `true`. `a/c/s launch` SHALL be present
exactly when `Dashboard::agents.reachable` is `true` **and** `Dashboard::file_mode` is `false`.
Both conditions SHALL be read from those two `Dashboard` fields and from nowhere else — never
from `ChangeSet::problems`, which `Dashboard::adopt` replaces wholesale on every refresh.

The two hints part company in file mode because the keys do: `g` focuses an agent that is
already running and needs no `openspec` binary, while `a`, `c`, and `s` must name an absolute
path to one in the prompt they send. Offering a key that will only ever answer with a refusal
is worse than not offering it, and the header already badges `file mode`, so the reason is on
screen before the reader presses anything. The keys are **not** removed from
`ui::help::INVENTORY`: they remain bound, `action_for` still produces their `Action`s, and
`launch::decide` answers a press with a named reason rather than with silence.

A single compound hint is used rather than four separate ones because four do not fit the
mandated 60-column footer: with `help-overlay`'s leading `? help`,
`? help  q quit  Enter detail  Esc back  a apply  c continue  s archive`
is 70 columns, so `fit_hints` would drop `s archive` and `g focus` at 60 — the first five hints
reach 59 columns and the sixth needs eleven more — and offer the reader an arbitrary subset of
the action keys at the narrow width. The compound form is kept
for that reason, unchanged.

`help-overlay` does move one measured outcome: the reachable footer is now **61** columns, so
`g focus` no longer fits the mandated 60-column frame and is dropped there. That is accepted
rather than worked around. The original objection to dropping an action hint was that the
reader would be left with "no indication that the other two exist", and `? help` is now the
first hint on the row: the overlay it opens lists `g` under `Agents` with its own description,
so a hint dropped at a narrow width is a hint the reader can still find. The footer is the
always-visible minimum; the overlay is the full list.

While filtering, `a`, `c`, `s`, and `g` SHALL type themselves into the query like every other
printable key, and the footer SHALL show the filter prompt alone — the action hints are dropped
with the rest.

#### Scenario: The action hints appear at both mandated widths when the socket is reachable

- **WHEN** a `Dashboard` with `agents.reachable` `true`, `file_mode` `false`, no filter, and no
  unattributed agents is rendered at 120x20 and at 60x20
- **THEN** the footer row at 120 reads exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch  g focus`, padded to the frame width
- **AND** that row is **61** columns of text, so at 60 the footer reads exactly
  `? help  q quit  Enter detail  Esc back  a/c/s launch` followed by eight spaces: `a/c/s
  launch` fits whole and `g focus` is dropped whole, never cut to `g focu`
- **AND** the reachable pane therefore still names an action key at the mandated narrow width,
  which is what the compound hint exists to guarantee

#### Scenario: File mode drops the launch hint and keeps the focus hint

- **WHEN** the same `Dashboard` with `agents.reachable` `true` and `file_mode` `true` is
  rendered at 120x20 and 60x20
- **THEN** the footer row reads exactly `? help  q quit  Enter detail  Esc back  g focus` at
  120 — **47** columns — padded to the frame width
- **AND** `a/c/s launch` appears nowhere in either buffer
- **AND** at 60 the same row fits whole, so `g focus` survives at the narrow width here
  precisely because the hint it was competing with is absent
- **AND** the header still badges `file mode` at 120, so the reason the hint is gone is on
  screen in the same frame

#### Scenario: An unreachable socket hides both hints at both widths

- **WHEN** the same `Dashboard` with `agents.reachable` `false` is rendered at 120x20 and 60x20
- **THEN** the footer row reads exactly `? help  q quit  Enter detail  Esc back`, padded, at
  both
- **AND** neither `a/c/s launch` nor `g focus` appears anywhere in either buffer, so a pane
  with no socket is offered no action key
- **AND** the same holds with `file_mode` `true` as well, so an unreachable socket is not made
  noisier by also being in file mode

#### Scenario: The count is dropped before the action hints as the width falls

- **WHEN** a `Dashboard` with `agents.reachable` `true`, `file_mode` `false`, and one
  unattributed agent is rendered at 120x20 and at 60x20
- **THEN** at 120 the footer reads
  `? help  q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed` — **77**
  columns
- **AND** at 60 it reads `? help  q quit  Enter detail  Esc back  a/c/s launch` — the full row
  is 77 columns, so `fit_hints` drops the count whole and then `g focus` whole, in that order,
  rather than cutting either
- **AND** with `agents.reachable` `false` at 60 the count reappears, reading
  `? help  q quit  Enter detail  Esc back  1 unattributed` — **54** columns — because the two
  hints it was competing with are absent

#### Scenario: The action keys type into the query while filtering

- **WHEN** the filter is active with an empty query and `a`, then `c`, then `s`, then `g` are
  pressed
- **THEN** each maps to `Action::FilterPush` of its own character and the query becomes `acsg`
- **AND** no launch request is produced by any of the four presses
- **AND** the footer shows `/acsg_` and neither action hint, at both mandated widths
- **AND** the same four presses with `file_mode` `true` behave identically, so file mode
  changes what the keys do only outside filter mode

### Requirement: A launch failure renders as a leading problem row and is replaced, never grown

`Dashboard::launch.problems` SHALL hold at most **four** entries: the last outcome's
failures — at most four, when the kind resolution reports both of its problems and both
`state::record` and `agent prompt` then fail on the same launch — or the last refusal `decide`
produced, which is always exactly one. It SHALL be replaced wholesale on every outcome and every refusal — never
appended to — so a reader who presses `a` on a failing socket ten times sees the same row or
rows, not ten.

A successful outcome SHALL clear it. There is no key to dismiss it and none is added: the row
answers the key that produced it and is replaced by the answer to the next.

`ui::list::rows` SHALL emit it as a `RowKind::Problem` row with the same `! `-prefixed grammar
every other problem row uses, **above** the refresh problems and the change-set problems.
Launch problems lead because they are the only rows produced by a key the reader has just
pressed; a watcher that would not start is a condition, while this is an answer.

#### Scenario: A refused launch renders one row at both widths

- **WHEN** a `Dashboard` whose selected change is `2fa-support` and whose live agents include one
  named `c-2fa-support` receives `a`, and is rendered at 120x20 and 60x20
- **THEN** the list region's interior row 0 begins `! ` and names `c-2fa-support` and `g`
- **AND** the row is exactly the interior width — 38 at 120 and 58 at 60 — padded or truncated
  with a trailing `…` by the same `pad_or_truncate_right` every other row uses
- **AND** pressing `a` twice more leaves exactly one such row

#### Scenario: A file-mode refusal renders one row at both widths

- **WHEN** a `Dashboard` with `file_mode` `true`, `agents.reachable` `true`, and a selected
  change receives `a`, and is rendered at 120x20 and 60x20
- **THEN** the list region's interior row 0 begins `! ` and names the absent `openspec` binary
- **AND** the row is exactly the interior width at each, on the same `pad_or_truncate_right`
  terms as every other row
- **AND** pressing `c` and then `s` leaves exactly one such row, replaced rather than grown

#### Scenario: Four outcome problems render as four leading rows

- **WHEN** a `Dashboard` adopts an `Outcome` whose `problems` holds the status-read failure,
  the last-resort warning, a state-directory failure, and `agent_blocked`, in that order, and
  is rendered at both widths
- **THEN** interior rows 0, 1, 2, and 3 are those four, in that order, each `! `-marked
- **AND** a fifth problem row is not present, so the bound this requirement states is the
  bound the pane shows
- **AND** the change list is still reachable below them at 20 rows, so the worst case leaves a
  usable pane

#### Scenario: A launch problem leads the refresh and change-set problems

- **WHEN** a `Dashboard` carries one `launch.problems` entry `launch failed`, one
  `refresh.problems` entry `watch failed`, and one `changes.problems` entry
  `openspec/changes unreadable`, rendered at both widths
- **THEN** interior rows 0, 1, and 2 are those three, in that order
- **AND** all three carry `RowKind::Problem`, so `ui::view` styles them identically and none is
  addressable by `selected`

#### Scenario: A later success clears an earlier failure

- **WHEN** a launch fails, its problem row is drawn, and a subsequent launch succeeds
- **THEN** `launch.problems` is empty and the row is gone on the next frame
- **AND** the rendered buffer is byte-identical to the frame before the first failure

### Requirement: A real keypress reaches the three Herdr calls in the shipped composition root

`ui::run_wired` — the real `ui::load`, the real `state::read`, the real `state::recorded_kind`,
the real `watch::start`, the real `refresh::start`, the real `agents::start`, the real
`launch::start`, the real `integration::parse`, the real `integration::resolve`, the real
`ui::read_artifact`, the real `action_for`, the real `Dashboard::apply`, the real
`ui::driver::run_loop`, the real `list::rows`, and the real `view::render` — SHALL be driven by a
test that feeds it an actual `a` key event and observes the three Herdr invocations, in order, at
the seam.

This requirement exists because of a shipped defect, not a hypothesis: `live-refresh` shipped a
`ui::run` that called neither `watch::start` nor `refresh::start`, every test passed, and the
whole live tier would have been permanently inert. `agent-attribution` then found a `WIRED` check
guarding the wrong link, where a shipped `state_dir: None` would have left tier 1 dead with every
gate green. A launcher that is never reached from a key is the same defect one layer on, and
unit tests over `decide`, over `launch::start`, and over `run_loop` cannot see it.

The composition root SHALL start the launcher for a repository it found and `launch::none()`
otherwise, and SHALL pass a `launch::Settings` carrying the repository root, `Config::agent_kind`,
the recorded kind read from the state directory, the per-kind prompt overrides, the resolved
`openspec` path as `Option<PathBuf>`, and the resolved state directory. `Settings` SHALL NOT
implement `Default`, on exactly `Launch`'s and `agents::AgentSnapshot`'s terms: it is constructed
at one site and a defaulted `repo` or `openspec_bin` would be silently wrong there.

The resolved `openspec` path passed into `Settings` SHALL be the same one
`start_collaborators` already computed for the CLI seam — `resolve::openspec_bin`'s
`found.path` — and `Collaborators::file_mode` SHALL be `true` exactly when it is `None`, so the
dashboard's badge, the dropped footer hint, and `decide`'s refusal all follow one fact.

**All six `Settings` threads SHALL be guarded, each by a test that fails when that thread is
cut.** Four already are: the repository root, `Config::agent_kind` (by the two runs that
disagree deliberately, `codex` and `gemini`), the resolved binary (by the plant that hardcodes
it to `None`), and the state directory. The remaining two — the **recorded kind** and the
**prompt overrides** — SHALL each get a scenario below.

They need one because nothing else can see them. `WIRED`'s name list does not carry
`state::recorded_kind` or `config.prompts`, so a `start_collaborators` that never calls the
first and passes an empty map for the second satisfies every gate in `make check`. That is
exactly the shape of the defect this requirement already cites: `agent-attribution`'s shipped
`state_dir: None`, which left a whole tier dead with every gate green.

No production file under `src/ui/` SHALL hold the literal `"claude"`, on exactly the terms
`agent-polling` set for `"herdr"`; after this change the literal lives in `src/integration.rs`
and no longer in `src/config.rs`.

#### Scenario: Pressing `a` splits a pane, starts an agent, and sends the prompt

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 against a scratch repository holding
  the active change `2fa-support`, a scratch state directory, a scratch `openspec` program, a
  scratch `herdr` program that logs every argument vector, and a `Config` whose `agent_kind` is
  `Some("codex")`; the event source yields timeouts until the log holds its first `agent list`
  entry, then presses `a`, then yields timeouts until the log holds **four entries that are not
  `agent list`**, then presses `q`
- **THEN** the **non-`agent list`** entries of the log are exactly `integration status` followed
  by the three launch calls, in order, with `agent start`'s
  `--pane` equal to the pane id `pane split`'s own payload named and `--kind` equal to `codex`
- **AND** the `agent prompt` entry's text names the scratch `openspec` program's own absolute
  path and contains no `/opsx:`
- **AND** every predicate and every ordering assertion in this scenario counts **only**
  non-`agent list` entries. The poller writes to the same log on its own one-second cadence, so
  an absolute entry count is a race: at `[agent list, agent list, pane split, agent start]` a
  "four entries" predicate is satisfied one call early and the ordering assertion fails for a
  reason that has nothing to do with the launcher
- **AND** the scratch state directory's `agent-names.toml` holds
  `c-2fa-support = "2fa-support"`
- **AND** the returned `Dashboard`'s `agent_names.names` holds the same pair, and
  `launch.pending` is `None` and `launch.problems` is empty
- **AND** the repository tree is byte-identical to before the run: the plugin wrote nothing
  inside `openspec/`

#### Scenario: Pressing `g` after the launch focuses the pane the launch created

- **WHEN** `run_wired` is driven as above but with `agent_kind` `Some("gemini")`, the scratch
  `herdr` program's `agent list` branch
  begins reporting the started agent — `name` `c-2fa-support`, `cwd` the canonicalized scratch
  root, `pane_id` the one it handed out — once `agent start` has been called, and the event
  source presses `a`, waits for the started agent to appear in a poll, presses `g`, waits for a
  **fifth** non-`agent list` entry, then presses `q`
- **THEN** the last **non-`agent list`** entry is `agent focus <that pane id>`
- **AND** `agent start`'s `--kind` is `gemini` here and `codex` in the scenario above: two
  different configured kinds across the two runs, so a `start_collaborators` that hardcoded
  either one fails one of them. A presence check on `config.agent_kind` alone cannot catch a
  hardcoded value, which is why the two runs disagree deliberately
- **AND** exactly one `integration status` entry appears across the whole run, so `g` added
  none
- **AND** the `2fa-support` row carries a badge, so the mapping written by the launch was read
  back through attribution's first tier within the same run
- **AND** no second `pane split` entry appears: `g` focuses, it does not launch

#### Scenario: With nothing configured, the sole installed integration is what launches

- **WHEN** `run_wired` is driven as above with `Config::agent_kind` `None`, no recorded kind in
  the scratch state directory, and the scratch `herdr` program answering `integration status`
  with seventeen lines in which only `codex` reads `current (v8)`
- **THEN** `agent start`'s `--kind` is `codex`
- **AND** `launch.problems` is empty, so a sole installed integration launches without asking
  and without a row
- **AND** the same run with the status naming **both** `claude` and `codex` as installed
  produces **no** `pane split` entry at all, and `launch.problems` holds exactly one entry
  naming both kinds and `agent_kind` — the refusal, rendered rather than guessed

#### Scenario: The recorded kind reaches `--kind` and outranks the installed evidence

- **WHEN** `run_wired` is driven with `Config::agent_kind` `None`, a scratch `settings.toml`
  containing `agent_kind = "gemini"`, and an `integration status` reporting **both** `claude`
  and `codex` as installed — a fixture in which `gemini` is installed by nothing
- **THEN** `agent start`'s `--kind` is `gemini`
- **AND** `launch.problems` holds exactly one entry, the absent-integration warning naming
  `gemini`, and the launch still completes
- **AND** a composition root that never calls `state::recorded_kind` reaches `Choice::Ambiguous`
  instead, emits **no** `pane split` entry, and fails this scenario on both the missing call and
  the missing `--kind` — so the thread is guarded by a disagreement between two outcomes rather
  than by a presence check

#### Scenario: A per-kind prompt override reaches the logged `agent prompt`

- **WHEN** `run_wired` is driven with `agent_kind = "codex"` and a `config.toml` carrying
  `[prompts.codex]` with `apply = "work on {change} using {openspec}"`
- **THEN** the logged `agent prompt` element is exactly
  `work on 2fa-support using <the scratch openspec program's absolute path>`
- **AND** it is not the built-in `Apply` text, so the override travelled
  `config::load` → `Settings` → the worker → `prompt_text` end to end
- **AND** a composition root that passes an empty override map logs the built-in text and fails
  this scenario, while still passing every other wiring scenario

#### Scenario: An unreachable socket leaves every key inert and the pane a working TUI

- **WHEN** `run_wired` is driven with a `herdr` path that does not exist, and the event source
  presses `a`, then `g`, then `q`
- **THEN** the run returns `Ok(dashboard)` with `agents.reachable` `false`
- **AND** `launch.pending` is `None`, `launch.problems` is empty, and `agent_names.names` is
  empty
- **AND** no file was created in the scratch state directory and the repository tree is
  byte-identical
- **AND** the footer carries neither action hint at either mandated width, while the change list
  renders normally

#### Scenario: File mode leaves `a` refusing and `g` working in the shipped root

- **WHEN** `run_wired` is driven with the configured path, `PATH`, nvm, and the `npm prefix -g`
  hook all unable to produce a usable binary, against a working scratch `herdr` program, and the
  event source presses `a`, then `q`
- **THEN** the returned `Dashboard` has `file_mode` `true`
- **AND** the log holds no `integration status` and no `pane split` entry, so the refusal
  happened in `decide` and nothing reached the launcher's worker
- **AND** `launch.problems` holds exactly one entry naming the absent `openspec` binary
- **AND** the footer carries `g focus` and not `a/c/s launch` at both mandated widths

#### Scenario: The wiring test fails when the launcher is replaced by the inert double

- **WHEN** `start_collaborators` is planted with `launch::none()` in place of `launch::start`
- **THEN** the launch scenario above fails on the four missing log entries, on the absent
  `agent-names.toml`, and on the empty `agent_names.names`
- **AND** the plant where `run_loop` stops handing `launch.pending` to the launcher fails those
  same assertions **and** leaves `launch.pending` `Some`, so the two plants are distinguishable
  by their assertion sets rather than merely both being red
- **AND** the plant where `Settings::openspec_bin` is hardcoded to `None` fails the first
  scenario on the missing `pane split` while leaving the file-mode scenario green, so that
  thread is guarded by a disagreement between two runs rather than by a presence check
