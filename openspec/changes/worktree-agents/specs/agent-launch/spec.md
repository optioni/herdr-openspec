## ADDED Requirements

### Requirement: A launch onto a worktree copy runs in that worktree

`launch::Request::Launch` SHALL gain one field, the checkout its pane is split in:

```rust
pub enum Request {
    Launch { change: String, agent: String, intent: Intent, root: Option<std::path::PathBuf> },
    Focus { pane_id: String },
    Resolve,
}
```

`launch::decide` SHALL NOT change in signature or policy, and SHALL build every
`Request::Launch` with `root: None`: which checkout a row's copy lives in is a fact about the
change set, not about whether a key may launch. When `decide` answers `Decision::Go(Request::Launch
{ .. })`, `Dashboard` SHALL set `root` to `Some(w.root)` for the first entry `w` of
`changes.worktrees` whose root the selected change's `dir` lies under, and SHALL leave it `None`
otherwise — so a change the pane shows from its own checkout launches exactly as before
`worktree-agents`, byte for byte. The worker SHALL use `root` as call 1's `--cwd` when present,
per this capability's three-call requirement, and `launch::Settings::repo` otherwise.

The prompt is unchanged. It names the plugin's resolved absolute `openspec` path and the change
name, and the `openspec` CLI resolves its repository from the launched agent's own working
directory — so an agent started in a worktree runs the workflow against the worktree's own
`openspec/`, which holds the very copy the row shows. The derived agent name is unchanged too,
so an agent already live for a change — in either checkout — still refuses a second launch for
it, as `decide` already specifies.

`g` SHALL be unaffected: `Request::Focus` names a pane, not a checkout.

#### Scenario: The worktree's root is filled from the selected row

- **WHEN** a `Dashboard` whose `changes.worktrees` holds `(/w/feat, "feat")`, whose Herdr socket
  is reachable, and which is not in file mode, has the active change `x` selected with `dir`
  `/w/feat/openspec/changes/x`, and `a` is pressed
- **THEN** `launch.pending` is
  `Some(Request::Launch { change: "x", agent: "x", intent: Apply, root: Some("/w/feat") })`
- **AND** the same press with `x`'s `dir` under the pane's own root yields `root: None`

#### Scenario: A refusal is unchanged by the root

- **WHEN** the same worktree row is selected while an agent named `x` is live, and `a` is
  pressed
- **THEN** `launch.pending` is `None` and `launch.problems` holds the landed "already running for
  this change" reason, so the checkout never lets a second agent onto one change

#### Scenario: `decide` never fills the root

- **WHEN** `launch::decide` is called with every `Intent` that launches, a change name, a
  reachable socket, no live names, nothing in flight, and not in file mode
- **THEN** each answer is `Decision::Go(Request::Launch { .., root: None })`

## MODIFIED Requirements

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

- **WHEN** the worker runs `Request::Launch { change: "x", agent: "x", intent: Apply, root:
  Some("/w/feat") }` against a scratch `herdr` program that logs each argument vector, with
  `launch::Settings::repo` `/r`
- **THEN** the `pane split` entry is exactly
  `pane split --cwd /w/feat --direction right --no-focus`
- **AND** the `agent start` and `agent prompt` entries are byte-identical to the same launch with
  `root: None`, whose `pane split` carries `--cwd /r`, so the root moves the pane and nothing else

