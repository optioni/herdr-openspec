## ADDED Requirements

### Requirement: A launch onto a worktree copy runs in that worktree

`launch::Request::Launch` carries a `root: Option<PathBuf>` — the checkout its pane is split in —
which `launch::decide` always builds as `None`, per this capability's decision requirement. When
`decide` answers `Decision::Go` with a `Request::Launch`, `Dashboard` SHALL set its `root` to
`Some(w.root)` where `w` is `worktrees::member_of(&changes.worktrees, &change.dir)` for the
selected change — the one derivation `worktree-changes` defines for "which member's copy is this
row", matching the change's directory against each member's `<root>/openspec/changes` — and
SHALL leave it `None` when that returns `None`. A change the pane shows from its own checkout
therefore launches exactly as before `worktree-agents`, byte for byte, including when the pane
itself runs inside a worktree nested under the main checkout. The worker SHALL use `root` as
call 1's `--cwd` when present, per this capability's three-call requirement, and
`launch::Settings::repo` otherwise.

Every `Request::Launch` literal or pattern this change writes, production or test, SHALL name
all four fields: `NODEFAULT-UI` scans every `Launch { … }` span in the crate and fails on a `..`
rest pattern.

The prompt is unchanged. It names the plugin's resolved absolute `openspec` path and the change
name, and the `openspec` CLI resolves its repository from the launched agent's own working
directory — measured with a nested scratch tree, it reports the innermost `openspec/` — so an
agent started in a worktree runs the workflow against the worktree's own `openspec/`, which
holds the very copy the row shows. The derived agent name is unchanged too, so an agent already
live for a change — in either checkout — still refuses a second launch for it.

`g` SHALL be unaffected in rule: `Request::Focus` names a pane, not a checkout, and the pane is
the one `agent-attribution` placed on the row — now including an agent running in a member
worktree.

#### Scenario: The worktree's root is filled from the selected row

- **WHEN** a `Dashboard` whose repository root is `/r`, whose `changes.worktrees` holds
  `(/w/feat, "feat")`, whose Herdr socket is reachable, and which is not in file mode, has the
  active change `x` selected with `dir` `/w/feat/openspec/changes/x`, and `a` is pressed
- **THEN** `launch.pending` is
  `Some(Request::Launch { change: "x", agent: "x", intent: Apply, root: Some("/w/feat") })`
- **AND** the same press with `x`'s `dir` `/r/openspec/changes/x` yields
  `Some(Request::Launch { change: "x", agent: "x", intent: Apply, root: None })`

#### Scenario: A pane inside a nested worktree launches its own rows in place

- **WHEN** the `Dashboard`'s repository root is `/r/.worktrees/feat`, its `changes.worktrees`
  holds `(/r, "main")`, and `a` is pressed first on a change whose `dir` is
  `/r/.worktrees/feat/openspec/changes/x` and then on one whose `dir` is `/r/openspec/changes/y`
- **THEN** the first `launch.pending` carries `root: None` and the second carries
  `root: Some("/r")`, although both directories lie under `/r`

#### Scenario: A refusal is unchanged by the root, and `g` reaches the worktree agent

- **WHEN** the worktree row of the first scenario is selected while one `Working` agent named
  `x`, in pane `w8:p7`, runs at `cwd` `/w/feat`, and `a` is pressed, then `g`
- **THEN** after `a`, `launch.pending` is `None` and `launch.problems` holds the landed "already
  running for this change" reason
- **AND** after `g`, `launch.pending` is `Some(Request::Focus { pane_id: "w8:p7" })` — the agent
  is placed on the row only because its worktree is a family member, so the same dashboard with
  `changes.worktrees` emptied yields `None` after `g`

#### Scenario: `decide` never fills the root

- **WHEN** `launch::decide` is called with each of `Apply`, `Continue`, and `Archive`, the change
  `x`, a reachable socket, no live names, nothing in flight, and not in file mode
- **THEN** each answer equals `Decision::Go(Request::Launch { change: "x", agent: "x", intent,
  root: None })` with that call's own `intent`, asserted as a whole literal

## MODIFIED Requirements

### Requirement: The launch decision is a pure, total function that refuses before it reaches Herdr

`launch::decide` SHALL hold the whole policy of what a launch key does, and SHALL be a pure
total function of its seven arguments:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent { Apply, Continue, Archive, Focus }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Launch { change: String, agent: String, intent: Intent, root: Option<std::path::PathBuf> },
    Focus { pane_id: String },
    Resolve,
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
7. Otherwise → `Go(Request::Launch { change, agent, intent, root: None })`. `decide` never
   fills `root`: which checkout a row's copy lives in is a fact about the change set, not about
   whether a key may launch, and `Dashboard` fills it afterwards (`worktree-agents`).

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
  "c-2fa-support", intent, root: None })` with that call's own `intent`
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

1. `["pane", "split", "--cwd", <checkout root>, "--direction", "right", "--no-focus"]`
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

`<checkout root>` SHALL be the `root` the `Request::Launch` carries when it carries one — the
OpenSpec root of the worktree the selected change's copy lives in, per this capability's
worktree-launch requirement — and otherwise the repository root `launch::Settings::repo` names,
exactly as before `worktree-agents`. It SHALL be rendered with `Path::to_string_lossy`, so a
non-UTF-8 root is passed lossily rather than failing the launch.

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
  `Some(Request::Launch { change: "add-auth", agent: "add-auth", intent: Apply, root: None })`
  — identical, field for field, to what the same press produces on an **active** change of that
  name in the pane's own checkout
- **AND** the same request run through the worker produces an `agent prompt add-auth` entry
  whose text is `agent-prompts`' `Apply` row for `add-auth`, unchanged
- **AND** the plugin makes no judgement about whether that command suits an archived change:
  deciding what to run next is `PRD.md` → Non-goals' orchestration, which belongs to the agent

#### Scenario: A launch onto a worktree copy splits in that worktree

- **WHEN** the worker's `handle` is given `Request::Launch { change: "x", agent: "x", intent:
  Apply, root: Some("/w/feat") }` with `launch::Settings::repo` `/r`, over a `FakeCli` whose
  Herdr side answers only the vectors registered for `--cwd /w/feat`
- **THEN** the recorded `pane split` call is exactly
  `pane split --cwd /w/feat --direction right --no-focus`
- **AND** the `agent start` and `agent prompt` entries are byte-identical to the same launch with
  `root: None`, whose `pane split` carries `--cwd /r`, so the root moves the pane and nothing else

