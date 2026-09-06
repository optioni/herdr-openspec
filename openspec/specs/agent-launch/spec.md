# agent-launch Specification

## Purpose
The one place a keypress causes an outward, side-effecting action: `a` / `c` / `s`
start a coding agent on the selected change with `/opsx:apply`, `/opsx:continue`, or
`/opsx:archive`, and `g` focuses the pane of the agent already attributed to it. The
decision is a pure, total function that refuses — with a named reason — before it
reaches Herdr; a launch it allows is exactly three sequential `herdr` calls, each
carrying the previous call's output: `pane split` for a pane id, `agent start` for a
running agent under a derived, Herdr-legal name capped at 32 characters, and
`agent prompt` carrying the real change name. Every call goes through the `HerdrCli`
seam on the crate's third worker thread, so `agent start`'s readiness wait never
blocks the draw loop; a failure stops the launch at the call that failed and renders
Herdr's own reason as a leading problem row. The keys are offered only while the
Herdr socket is reachable, and the plugin still writes nothing inside `openspec/` —
the agent it starts does that.

## Requirements

### Requirement: The launch decision is a pure, total function that refuses before it reaches Herdr

`launch::decide` SHALL hold the whole policy of what a launch key does, and SHALL be a pure
total function of its five arguments:

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
   `Go(Request::Focus { pane_id: p })`.
3. `change` `None` → `Nothing`. There is nothing selected to launch onto.
4. The derived agent name, `state::agent_name(change)`, appearing in `live_names` →
   `Refuse`, naming the derived name and pointing at `g`.
5. Otherwise → `Go(Request::Launch { change, agent, intent })`.

`decide` SHALL NOT consult a terminal title, an agent kind, a pane's working directory, or any
field other than the five arguments above.

#### Scenario: An unreachable socket makes every action key inert

- **WHEN** `decide` is called with `reachable` `false`, `change` `Some("add-auth")`, `pane`
  `Some("w8:p3")`, and an empty `live_names`, once for each of `Apply`, `Continue`, `Archive`,
  and `Focus`
- **THEN** all four return `Decision::Nothing`
- **AND** no `Request` is produced on any of the four calls, so no Herdr call can follow and no
  problem row is produced either — an unreachable socket is silent, not a fault

#### Scenario: No selected change means no launch, and no agent means no focus

- **WHEN** `decide` is called with `reachable` `true`, `change` `None`, `pane` `None`, and an
  empty `live_names`, for `Apply`, `Continue`, and `Archive`
- **THEN** all three return `Decision::Nothing`
- **AND** the same call for `Focus` also returns `Decision::Nothing`
- **AND** the call for `Focus` with `pane` `Some("w8:p3")` and `change` `None` returns
  `Decision::Go(Request::Focus { pane_id: "w8:p3" })`, so focusing needs a pane and not a
  selection

#### Scenario: Each launch intent carries its own change and derived name

- **WHEN** `decide` is called with `reachable` `true`, `change` `Some("2fa-support")`, `pane`
  `None`, and an empty `live_names`, once for each of `Apply`, `Continue`, and `Archive`
- **THEN** each returns `Decision::Go(Request::Launch { change: "2fa-support", agent:
  "c-2fa-support", intent })` with that call's own `intent`
- **AND** the three results differ **only** in `intent`, so the change and the derived name are
  computed once and identically for all three keys

#### Scenario: A derived name already live in the session is refused before any Herdr call

- **WHEN** `decide` is called with `reachable` `true`, `change` `Some("2fa-support")`, and
  `live_names` `["c-2fa-support", "other"]`
- **THEN** it returns `Decision::Refuse(reason)` where `reason` names `c-2fa-support` and tells
  the reader to press `g`
- **AND** no `Request` is produced, so no pane is split and no stray pane can be left behind —
  the refusal is what keeps the common repeat-press from leaking a pane
- **AND** the same call with `live_names` `["c-2fa-support-x", "2fa-support"]` returns
  `Decision::Go`, because the match is on the **derived** name byte for byte and neither of
  those is it

#### Scenario: Every combination is total

- **WHEN** `decide` is called with `change` `Some("")`, `pane` `Some("")`, `reachable` `true`,
  and `live_names` `[""]`, once per `Intent`
- **THEN** none of the four panics
- **AND** `Apply` returns `Decision::Go` carrying the derived name `change` — `state::agent_name`
  maps the empty string to that fallback — while `Focus` returns
  `Decision::Go(Request::Focus { pane_id: "" })`, both of which the launcher then fails on
  honestly rather than the decision guessing on the caller's behalf

### Requirement: The launched agent's name is Herdr-legal, capped at 32 characters, and recorded whenever it differs

The name passed to `herdr agent start` SHALL be `state::agent_name(change)` and never the raw
change name. Herdr 0.8.2 validates the name before it looks at the pane: measured live, a name
outside `[a-z][a-z0-9_-]{0,31}` returns exit **1** with
`{"error":{"code":"invalid_agent_name","message":"agent name must start with a lowercase letter
and contain only lowercase letters, digits, '-' or '_' (1-32 characters)"}}` on **stderr**, and
that is true of an over-long name, an upper-case name, and a name beginning with a digit alike.

After `herdr agent start` succeeds, and **before** the prompt is sent, the launcher SHALL call
`state::record(state_dir, agent, change)`. `plugin-state` already makes that call a no-op when
the derived name equals the change name and a write otherwise, so the mapping is recorded
whenever the derived name **differs from the change name at all** — not only when it was
truncated.

The launcher SHALL write nothing else, anywhere, and SHALL never write inside the repository.

#### Scenario: A change name past the 32-character cap is truncated, hashed, and recorded

- **WHEN** a launch is run for the change
  `a-very-long-change-name-that-exceeds-the-cap` (44 characters) with the state directory
  pointing at a scratch tree
- **THEN** the `agent start` argument vector's name argument is exactly 32 characters or fewer,
  matches `[a-z][a-z0-9_-]{0,31}`, and is `state::agent_name`'s own output for that change —
  the first 27 characters, trailing separators trimmed, plus `-` and a four-digit base-36
  FNV-1a suffix
- **AND** `agent-names.toml` in the scratch state directory afterwards holds that derived name
  mapped to `a-very-long-change-name-that-exceeds-the-cap`
- **AND** running the identical launch again on a fresh state directory produces the identical
  derived name, so the mapping is reproducible on another machine

#### Scenario: A legal-but-derived name is recorded too

- **WHEN** a launch is run for the change `2FA_Support!` — legal characters after folding, only
  12 characters, and therefore never truncated
- **THEN** the `agent start` name argument is **`c-2fa_support`**: ASCII-lowercased, `!`
  replaced by `-`, the trailing `-` trimmed, and `c-` prefixed because a digit cannot begin an
  agent name. The `_` **survives**, because `plugin-state`'s step 2 replaces only characters
  **outside** `[a-z0-9_-]` and `_` is inside it — this scenario is written with an underscore
  precisely so the derivation's one non-obvious step has a test
- **AND** `agent-names.toml` holds `c-2fa_support = "2FA_Support!"`, because the derived name
  differs from the change name — recording is not conditional on truncation
- **AND** the derived name is `state::agent_name`'s output and is not re-derived here: the
  change `2fa-support`, used by every other scenario in this capability, derives to
  `c-2fa-support` with a hyphen, and the two are different names for different changes
- **AND** a launch for the change `add-auth`, whose derived name is `add-auth` unchanged,
  writes **no** `agent-names.toml` at all: `state::record` short-circuits on equality before it
  consults the directory

#### Scenario: A derived name that collides with a live agent never reaches Herdr

- **WHEN** the change `a-very-long-change-name-that-exceeds-the-cap` has already been launched,
  so its 32-character derived name is live in the session, and the reader presses `a` on that
  same change again
- **THEN** `decide` returns `Decision::Refuse` naming that derived name and pointing at `g`, and
  the scratch `herdr` program's invocation log gains **no** entry at all
- **AND** the pane count is unchanged, because the refusal happens before `pane split`
- **AND** `g` on that change **does** focus, because the live agent's name is in the mapping and
  tier 1 badges the row — the refusal's advice is therefore actionable, which is the case this
  scenario covers
- **AND** the *other* collision — two **different** changes whose derived names coincide, which
  step 6's four base-36 digits and step 4's `change` fallback both make possible — is out of
  scope and is stated rather than left to be discovered: the mapping badges whichever change was
  launched last, so `g` on the *other* change finds no entry in `panes` and is inert, and the
  refusal row's "press `g`" is then advice the reader cannot act on. Attributing it correctly
  would need a second identifier the plugin does not have

#### Scenario: A collision Herdr sees anyway is reported with Herdr's own reason

- **WHEN** the poll snapshot did not yet carry the colliding agent, so `decide` returns
  `Decision::Go`, and the scratch `herdr` program answers `agent start` with exit `1` and
  `{"error":{"code":"agent_name_taken","message":"agent name c-2fa-support is already used; …"}}`
  on stderr
- **THEN** the launch's outcome carries a problem naming `agent_name_taken`, the derived name,
  and the pane id the split produced
- **AND** the launcher issues no `pane close` call: the split pane is left in place, named in
  the problem, for the reader to close

### Requirement: A launch is exactly three Herdr calls, in order, the second and third carrying the first's output

`launch`'s worker SHALL issue, through `cli::HerdrCli` and nothing else, exactly these argument
vectors, in this order, stopping at the first that fails:

1. `["pane", "split", "--cwd", <repository root>, "--direction", "right", "--no-focus"]`
2. `["agent", "start", <derived name>, "--kind", <configured kind>, "--pane", <pane id from 1>]`
3. `["agent", "prompt", <derived name>, <prompt text>]`

`--direction` is not optional: measured against Herdr 0.8.2, omitting it exits **2** with
`usage: herdr pane split [<pane_id>|--pane ID|--current] --direction right|down …` and creates
no pane. No pane argument is given, and that is deliberate: measured, `pane split` with no pane
argument splits the **focused** pane, which is the dashboard's own pane whenever one of its keys
was pressed. `--no-focus` keeps the reader in the dashboard, which is what makes `g` useful.

The prompt text SHALL be `/opsx:apply <change>` for `Intent::Apply`, `/opsx:continue <change>`
for `Intent::Continue`, and `/opsx:archive <change>` for `Intent::Archive`, each a **single**
argument vector element — the seam passes arguments to the program directly with no shell, so a
space in the text needs no quoting and SHALL NOT be given any. `--wait` SHALL NOT be passed:
measured, an un-waited `agent prompt` returns as soon as the text is submitted, and waiting
would hold the worker for the length of the agent's turn.

`<configured kind>` SHALL be `Config::agent_kind`, whose documented default is `claude`. It
SHALL NOT be a literal anywhere under `src/ui/`.

The repository root SHALL be rendered with `Path::to_string_lossy`, so a non-UTF-8 root is
passed lossily rather than failing the launch.

#### Scenario: The three calls appear in order with the split's own pane id

- **WHEN** a launch for `2fa-support` with `agent_kind` `codex` runs against a scratch `herdr`
  program that logs each argument vector and answers `pane split` with
  `{"id":"cli:pane:split","result":{"pane":{"pane_id":"wD:pJ","tab_id":"wD:t2",
  "workspace_id":"wD"},"type":"pane_info"}}`
- **THEN** the log holds exactly three entries in this order:
  `pane split --cwd <root> --direction right --no-focus`,
  `agent start c-2fa-support --kind codex --pane wD:pJ`, and
  `agent prompt c-2fa-support /opsx:apply 2fa-support`
- **AND** the `--pane` value is `wD:pJ`, the id the first call's own payload named — a launch
  that passed any other value would be reading the pane id from somewhere other than call 1
- **AND** `--kind` is `codex`, not `claude`, so the configured kind is threaded through rather
  than hardcoded
- **AND** the same launch against a repository root whose bytes are **not valid UTF-8** still
  produces the same three entries, with `--cwd` carrying `Path::to_string_lossy`'s rendering —
  a launch is never refused for a path this plugin cannot spell

#### Scenario: Each intent sends its own `/opsx:*` command and nothing else

- **WHEN** the same launch is run three times for the change `add-auth`, once per intent
- **THEN** the third logged entry is `agent prompt add-auth /opsx:apply add-auth`,
  `agent prompt add-auth /opsx:continue add-auth`, and
  `agent prompt add-auth /opsx:archive add-auth` respectively
- **AND** the prompt text is one argument-vector element in each case, containing exactly one
  space and no quote character
- **AND** the first two logged entries are byte-identical across all three runs, so the intent
  changes the prompt and nothing else

#### Scenario: An archived change launches on the same terms as an active one

- **WHEN** the selected change is an archived one named `add-auth` and `a` is pressed
- **THEN** `launch.pending` is
  `Some(Request::Launch { change: "add-auth", agent: "add-auth", intent: Apply })` — identical,
  field for field, to what the same press produces on an **active** change of that name
- **AND** the same request run through the worker produces
  `agent prompt add-auth /opsx:apply add-auth`, unchanged
- **AND** the plugin makes no judgement about whether that command suits an archived change:
  deciding what to run next is `PRD.md` → Non-goals' orchestration, which belongs to the agent

### Requirement: The pane id is read from `pane split`'s envelope, and an unusable payload stops the launch

`launch::pane_id(text: &str) -> Result<String, String>` SHALL navigate Herdr's envelope to
`result.pane.pane_id` and return it, following `agents::parse_list`'s style: a
`serde_json::Value`, then each key in turn, with `Err(reason)` naming what was missing or
wrong-shaped. It SHALL be pure and SHALL never panic for any input, including the empty string.

It SHALL report Herdr's own error envelope — `{"id":…,"error":{"code":…,"message":…}}` — by its
`code` and `message` rather than as a generic shape mismatch, even though a real failure exits
non-zero and reaches the caller as `CliError::Failed` instead: the payload is defended against
because the seam returns stdout verbatim and cannot vouch for it.

Nothing anywhere in the launch tier SHALL assume that Herdr's stderr is JSON. Measured, an
**argument** error — `herdr agent start … --kind nosuchkind` — exits **2** with the plain text
`unsupported interactive agent kind: nosuchkind`, while a **domain** error exits 1 with an
envelope. `CliError::Failed`'s stderr is therefore carried into the reported problem verbatim
and never parsed, on `agents::herdr_error_problem`'s established terms.

When `pane_id` returns `Err`, the launch SHALL stop: no `agent start` call is issued and no
mapping is recorded.

#### Scenario: A well-formed envelope yields the pane id

- **WHEN** `pane_id` is given
  `{"id":"cli:pane:split","result":{"pane":{"agent_status":"unknown","cwd":"/r","pane_id":
  "wD:pJ","tab_id":"wD:t2","workspace_id":"wD"},"type":"pane_info"}}`
- **THEN** it returns `Ok("wD:pJ")`
- **AND** trailing whitespace and a trailing newline on the payload change nothing

#### Scenario: Every unusable payload is an error, never a panic and never a guess

- **WHEN** `pane_id` is given, in turn: the empty string; `not json`; `[]`; `{}`;
  `{"result":{}}`; `{"result":{"pane":{}}}`; `{"result":{"pane":{"pane_id":7}}}`; and
  `{"id":"cli:pane:split","error":{"code":"pane_not_found","message":"no such pane"}}`
- **THEN** each returns `Err(reason)` and none panics
- **AND** the last one's reason names both `pane_not_found` and `no such pane`, so Herdr's own
  words reach the reader rather than "payload has no result object"
- **AND** none of the eight returns `Ok`, so no launch can proceed on a payload with no pane id

#### Scenario: An unusable payload stops the launch before an agent is started

- **WHEN** the scratch `herdr` program answers `pane split` with exit `0` and the body `{}`
- **THEN** the invocation log holds exactly one entry, the split
- **AND** the outcome carries a problem naming the missing `result` object, and `outcome.named`
  is **`None`**
- **AND** no `agent-names.toml` is written, because the mapping is recorded only after
  `agent start` succeeds

### Requirement: A failed call stops the launch at that call and carries Herdr's own reason

Each of the three calls SHALL be checked, and the first failure SHALL end the launch. Herdr
writes its diagnostic to **stderr** as a JSON error envelope and exits non-zero, so
`cli::CliError::Failed`'s `stderr` carries the reason and the reported problem SHALL include it
verbatim, on `agents::herdr_error_problem`'s established terms — unlike the OpenSpec CLI, whose
diagnostic goes to stdout and is therefore unavailable.

The launcher SHALL NOT issue `herdr pane close`, ever. A pane it created and could not use is
left in place with its id named in the problem, because a failed `agent start` includes the
readiness-timeout case, in which an agent may be starting and closing the pane would kill it.

The three failure points SHALL be distinguishable in what they leave behind:

| Failed call | Pane | Agent | Mapping | Prompt |
|---|---|---|---|---|
| `pane split` | none created | none | not recorded | not sent |
| `agent start` | created, left, id named | none | not recorded | not sent |
| `agent prompt` | created, left | running | **recorded** | not sent |

#### Scenario: A failed split leaves nothing behind

- **WHEN** the scratch `herdr` program answers `pane split` with exit `1` and
  `{"error":{"code":"pane_split_failed","message":"no space to split"}}` on stderr
- **THEN** the invocation log holds exactly one entry
- **AND** the outcome's problem names `pane_split_failed` and `no space to split`
- **AND** `outcome.named` is **`None`** — no agent was started, so nothing may be recorded and
  nothing may reach `Dashboard::agent_names`. A worker that filled `named` unconditionally would
  badge the change with whatever agent the next poll returned, which is exactly the guess
  `agent-attribution` refuses
- **AND** the scratch state directory is byte-identical to before the launch

#### Scenario: A failed start leaves the pane and names it

- **WHEN** the split succeeds with pane id `wD:pJ` and `agent start` exits `1` with
  `{"error":{"code":"agent_pane_not_found","message":"agent target wD:pJ not found"}}`
- **THEN** the invocation log holds exactly two entries and no third
- **AND** the outcome's problem names `agent_pane_not_found`, the derived agent name, and the
  pane id `wD:pJ`
- **AND** `outcome.named` is **`None`**: `agent start` failed, so no agent exists to attribute
- **AND** the log holds no `pane close` entry — the plugin never closes a pane it created

#### Scenario: A failed prompt leaves a running, un-prompted agent that is still attributable

- **WHEN** the split and the start both succeed and `agent prompt` exits `1` with
  `{"error":{"code":"agent_blocked","message":"agent is blocked"}}`
- **THEN** the invocation log holds all three entries
- **AND** `agent-names.toml` holds the derived name mapped to the change, because the mapping is
  recorded between the start and the prompt — the agent exists and the pane must be able to
  badge it even though the prompt did not land
- **AND** the outcome reports **both**: `named` is `Some((derived name, change))`, so the loop
  updates `Dashboard::agent_names`, **and** `problem` names `agent_blocked`. This is the one
  path on which both fields are `Some`, and it is what distinguishes "the agent exists but was
  not prompted" from every failure before `agent start`

#### Scenario: A failed recording does not undo a successful start

- **WHEN** the split and the start both succeed and the state directory is a path that cannot be
  created (an existing regular file)
- **THEN** the prompt is still sent — the third log entry is present
- **AND** the outcome carries the derived name and change, so the in-memory mapping is correct
  for this session even though the file is not
- **AND** the outcome's problem names the state directory path and the I/O reason, rather than
  claiming the launch failed

### Requirement: `g` focuses the pane of the agent whose status the badge shows

`g` SHALL resolve its target through `Attribution::panes` — the pane id of the agent whose status
won the badge's precedence fold for the selected change — and SHALL issue exactly
`["agent", "focus", <pane id>]`, one Herdr call and no other.

The pane id, not the agent name, SHALL be the target. Measured against Herdr 0.8.2, `agent
focus` resolves a pane id and an agent name but **not** a terminal id; every agent in `herdr
agent list` carries a `pane_id`, while `name` is present only on an agent someone named, so the
pane id is the handle that exists for every agent the badge can show.

`g` on a change with no badge SHALL be inert: no Herdr call, no problem row. There is nothing to
focus, and saying so in a problem row would make every stray `g` a fault.

#### Scenario: `g` focuses an agent attributed through the mapping

- **WHEN** the selected change is `2fa-support`, the live agent `c-2fa-support` in pane `wD:pJ`
  is attributed to it through the plugin-local mapping, the socket is reachable, and `g` is
  pressed
- **THEN** exactly one Herdr call is issued, `agent focus wD:pJ`
- **AND** no `pane split`, `agent start`, or `agent prompt` call is issued
- **AND** `outcome.named` is `None` and `agent-names.toml` is unchanged: focusing records
  nothing

#### Scenario: `g` focuses an agent attributed by name

- **WHEN** the selected change is `add-auth`, a live agent named `add-auth` in pane `w8:p6` is
  attributed to it by the exact-name tier, and `g` is pressed
- **THEN** exactly one Herdr call is issued, `agent focus w8:p6`
- **AND** the target is the pane id, not the name `add-auth`, even though Herdr would resolve
  either

#### Scenario: `g` on a change no tier could attribute does nothing

- **WHEN** the selected change is `add-auth`, one in-scope live agent named `scratch-work`
  exists — counted as unattributed by the third tier and badging no row — and `g` is pressed
- **THEN** no Herdr call is issued at all
- **AND** no problem row appears, and the rendered buffer is byte-identical to the frame before
  the key was pressed
- **AND** the same is true when the visible change list is empty, so there is no selection to
  resolve

#### Scenario: `g` with an unreachable socket does nothing

- **WHEN** `Dashboard::agents.reachable` is `false` and `g` is pressed
- **THEN** no Herdr call is issued and no problem row appears
- **AND** the footer carries neither `a/c/s launch` nor `g focus`, so the key was never offered

### Requirement: The launcher seam is a trait whose every method is non-blocking, and the worker is the crate's third thread

```rust
pub trait Launcher: Send {
    fn request(&mut self, request: Request);
    fn drain(&mut self) -> Option<Outcome>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub named: Option<(String, String)>,
    pub problem: Option<String>,
}

pub fn none() -> Box<dyn Launcher>;
pub fn start(
    cli: std::sync::Arc<dyn crate::cli::HerdrCli>,
    repo: std::path::PathBuf,
    kind: String,
    state_dir: Option<std::path::PathBuf>,
) -> Box<dyn Launcher>;
```

Both methods SHALL be non-blocking, on exactly `watch::FsEvents`', `refresh::Refresher`'s, and
`agents::AgentPoll`'s terms: the render path calls them on every iteration and neither may wait
on anything. `request` SHALL send on a channel and return; `drain` SHALL `try_recv` and return.

The trait SHALL carry **no** `pending_in`. The launcher has no schedule of its own — it acts only
when a key is pressed — so it contributes nothing to the loop's wake-up and
`watch::soonest(fs, agents)` is unchanged. It therefore reads **no** clock, anywhere.

`launch::start` SHALL spawn the crate's third worker thread, on `refresh::start`'s and
`agents::start`'s shape: one request channel in, one result channel out, the worker body written
**below** the single `thread::spawn` in the file so `NOBLOCK`'s leg 3 can cut the production
slice there. The worker SHALL reach the `herdr` program only through `Arc<dyn HerdrCli>`;
`src/launch.rs` SHALL name no process-spawn API, and `src/cli.rs` SHALL remain the crate's
single spawn site.

`launch::none()` SHALL be the inert implementation: `request` discards, `drain` is always
`None`, no thread and no process. It is what `start_collaborators` uses when no repository was
found.

`Outcome::named` SHALL carry `(derived agent name, change name)` exactly when an agent was
started, so the loop can keep `Dashboard::agent_names` current without re-reading the file, and
`None` for a focus and for a launch that failed before `agent start`. `Outcome::problem` SHALL
carry the failure and be `None` on complete success. `Outcome` SHALL NOT implement `Default`,
derived or hand-written, anywhere in the crate, and every construction and destructuring SHALL
name both fields with no `..` rest, on exactly `AgentSnapshot`'s and `Attribution`'s terms.

#### Scenario: The inert launcher answers nothing and starts nothing

- **WHEN** `launch::none()` is given a `Request::Launch` and then drained ten times
- **THEN** every `drain` returns `None`
- **AND** no thread is started and no process is spawned, which is what makes a dashboard with
  no repository cost nothing

#### Scenario: The real launcher answers on a later drain, never on the requesting one

- **WHEN** `launch::start` is given a recording `HerdrCli` fake that answers all three calls, a
  request is made, and `drain` is polled until it answers or a bounded deadline passes
- **THEN** the `drain` immediately after `request` returns `None` — the work is on the worker
  thread, not the caller's
- **AND** a later `drain` returns `Some(Outcome)` with `problem` `None`
- **AND** every `drain` call returns within a time that is not a function of the fake's own
  latency, so the render path never waits on the worker

#### Scenario: Dropping the launcher stops its worker

- **WHEN** a `Launcher` from `launch::start` is dropped
- **THEN** the worker's request channel disconnects and the worker returns, on exactly
  `refresh::worker_body`'s and `agents::worker_body`'s lifecycle
- **AND** a `drain` on a launcher whose worker has already stopped answers `None` rather than
  panicking or blocking

### Requirement: The launch keys are offered only when the socket is reachable, and type themselves while filtering

The footer SHALL carry two further hints, `a/c/s launch` and `g focus`, in that order,
immediately after `Esc back` and **before** the `<n> unattributed` count, present exactly when
`Dashboard::agents.reachable` is `true`. The condition SHALL be read from `Dashboard::agents` and
from nowhere else — never from `ChangeSet::problems`, which `Dashboard::adopt` replaces wholesale
on every refresh.

A single compound hint is used rather than four separate ones because four do not fit the
mandated 60-column footer: `q quit  Enter detail  Esc back  a apply  c continue  s archive`
is 62 columns, so `fit_hints` would drop `s archive` and `g focus` and offer the reader an
arbitrary subset of the action keys at the narrow width.

While filtering, `a`, `c`, `s`, and `g` SHALL type themselves into the query like every other
printable key, and the footer SHALL show the filter prompt alone — the action hints are dropped
with the rest.

#### Scenario: The action hints appear at both mandated widths when the socket is reachable

- **WHEN** a `Dashboard` with `agents.reachable` `true`, no filter, and no unattributed agents is
  rendered at 120x20 and at 60x20
- **THEN** the footer row reads exactly
  `q quit  Enter detail  Esc back  a/c/s launch  g focus`, padded to the frame width, at both
- **AND** that row is 53 columns of text, so both hints fit inside the 60-column footer whole

#### Scenario: An unreachable socket hides both hints at both widths

- **WHEN** the same `Dashboard` with `agents.reachable` `false` is rendered at 120x20 and 60x20
- **THEN** the footer row reads exactly `q quit  Enter detail  Esc back`, padded, at both
- **AND** the buffer is byte-identical to the frame the same dashboard produced before this
  change existed, so a pane with no socket gained nothing

#### Scenario: The count is dropped before the action hints as the width falls

- **WHEN** a `Dashboard` with `agents.reachable` `true` and one unattributed agent is rendered at
  120x20 and at 60x20
- **THEN** at 120 the footer reads
  `q quit  Enter detail  Esc back  a/c/s launch  g focus  1 unattributed`
- **AND** at 60 it reads `q quit  Enter detail  Esc back  a/c/s launch  g focus` — the count is
  69 columns in and does not fit, and `fit_hints` drops it whole rather than cutting it
- **AND** with `agents.reachable` `false` at 60 the count reappears, because the two hints it was
  competing with are absent

#### Scenario: The action keys type into the query while filtering

- **WHEN** the filter is active with an empty query and `a`, then `c`, then `s`, then `g` are
  pressed
- **THEN** each maps to `Action::FilterPush` of its own character and the query becomes `acsg`
- **AND** no launch request is produced by any of the four presses
- **AND** the footer shows `/acsg_` and neither action hint, at both mandated widths

### Requirement: A launch failure renders as a leading problem row and is replaced, never grown

`Dashboard::launch.problems` SHALL hold at most one entry: the last outcome's failure, or the
last refusal `decide` produced. It SHALL be replaced wholesale on every outcome and every
refusal — never appended to — so a reader who presses `a` on a failing socket ten times sees one
row, not ten.

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

`ui::run_wired` — the real `ui::load`, the real `state::read`, the real `watch::start`, the real
`refresh::start`, the real `agents::start`, the real `launch::start`, the real
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
otherwise, and SHALL pass `Config::agent_kind` and the resolved state directory into it. No
production file under `src/ui/` SHALL hold the literal `"claude"`, on exactly the terms
`agent-polling` set for `"herdr"`.

#### Scenario: Pressing `a` splits a pane, starts an agent, and sends the prompt

- **WHEN** `run_wired` is driven at 120x20 and again at 60x20 against a scratch repository holding
  the active change `2fa-support`, a scratch state directory, a scratch `openspec` program, a
  scratch `herdr` program that logs every argument vector, and a `Config` whose `agent_kind` is
  `codex`; the event source yields timeouts until the log holds its first `agent list` entry,
  then presses `a`, then yields timeouts until the log holds **three entries that are not
  `agent list`**, then presses `q`
- **THEN** the **non-`agent list`** entries of the log are exactly the three launch calls, in
  order, with `agent start`'s
  `--pane` equal to the pane id `pane split`'s own payload named and `--kind` equal to `codex`
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

- **WHEN** `run_wired` is driven as above but with `agent_kind` `gemini`, the scratch `herdr`
  program's `agent list` branch
  begins reporting the started agent — `name` `c-2fa-support`, `cwd` the canonicalized scratch
  root, `pane_id` the one it handed out — once `agent start` has been called, and the event
  source presses `a`, waits for the started agent to appear in a poll, presses `g`, waits for a
  **fourth** non-`agent list` entry, then presses `q`
- **THEN** the last **non-`agent list`** entry is `agent focus <that pane id>`
- **AND** `agent start`'s `--kind` is `gemini` here and `codex` in the scenario above: two
  different configured kinds across the two runs, so a `start_collaborators` that hardcoded
  either one fails one of them. A presence check on `config.agent_kind` alone cannot catch a
  hardcoded value, which is why the two runs disagree deliberately
- **AND** the `2fa-support` row carries a badge, so the mapping written by the launch was read
  back through attribution's first tier within the same run
- **AND** no second `pane split` entry appears: `g` focuses, it does not launch

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

#### Scenario: The wiring test fails when the launcher is replaced by the inert double

- **WHEN** `start_collaborators` is planted with `launch::none()` in place of `launch::start`
- **THEN** the launch scenario above fails on the three missing log entries, on the absent
  `agent-names.toml`, and on the empty `agent_names.names`
- **AND** the plant where `run_loop` stops handing `launch.pending` to the launcher fails those
  same assertions **and** leaves `launch.pending` `Some`, so the two plants are distinguishable
  by their assertion sets rather than merely both being red
