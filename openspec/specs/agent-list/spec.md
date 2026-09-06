# agent-list Specification

## Purpose
Asking Herdr for the agents beside the pane: `herdr agent list` spawned through the
`HerdrCli` seam with exactly `["agent", "list"]` and no `--json` flag, and its
`{"id": …, "result": {"agents": […]}}` envelope parsed into `Agent` values. Absent
optional fields degrade, an entry without an identity is skipped, an unusable payload
names its reason and yields no agents, and a failed run is an unreachable socket
carrying the program's own reason — a supported standing state, not an error.

## Requirements

### Requirement: The agent list is asked for with exactly two arguments and no JSON flag

`agents::poll_once(cli: &dyn cli::HerdrCli) -> AgentSnapshot` SHALL call `cli.run` exactly
once, with the argument vector `["agent", "list"]` and nothing else — no `--json`, no
`--session`, no path, no filter.

The absence of `--json` is a **verified fact about Herdr 0.8.2, not a stylistic choice**, and
is recorded here because the obvious instinct is to add it. `herdr agent list` already emits
JSON on stdout as its only output format; passing `--json` is rejected with exit status **2**
and the text `usage: herdr agent list` on stderr, which this seam would surface as
`CliError::Failed` and the pane would render as a permanently unreachable socket on a machine
where the socket is fine. The argument vector SHALL be pinned by an equality assertion on the
`FakeCli` recorder, the way `changes-from-cli` pins `["list", "--json"]`, so a later change
cannot add a flag without a test going red.

`agents::poll_once` SHALL be the only function in the crate that names those arguments, and it
SHALL spawn nothing itself: `src/agents.rs` names no `process::Command`, no `Command::new`,
and no `Stdio`, so `src/cli.rs` stays the crate's single spawn site.

#### Scenario: One poll is exactly one `agent list` call

- **WHEN** `poll_once` is given a `FakeCli` with a response registered on the **`HerdrCli`**
  side for `["agent", "list"]`
- **THEN** the fake's recorded calls are exactly `[(Program::Herdr, ["agent", "list"])]` — one
  call, on the Herdr handle, with that vector and no other argument
- **AND** nothing was recorded on the `OpenspecCli` side, so a poll never reaches the
  `openspec` binary

#### Scenario: The seam module spawns no process and names no view type

- **WHEN** `src/agents.rs` is searched for `process::Command`, `Command::new`, and `Stdio`,
  and separately for `ratatui`, `Modifier`, `Style`, `Span`, `Rect`, `Frame`, and `Buffer` in
  its production slice
- **THEN** there is no match in either sweep: the module reaches the `herdr` program only
  through `cli::HerdrCli`, and it is plain data, so a polled agent never arrives at the view
  already styled
- **AND** it is paired with a positive control asserting that `src/agents.rs` **does** name
  `HerdrCli`, so a search that matched nothing because the module was gutted or renamed fails
  instead of reporting a clean tree
- **AND** the check fails when `src/agents.rs` is absent, rather than reporting a clean tree

### Requirement: A successful list is an envelope, not a bare array

`agents::parse_list(text: &str) -> Result<Listed, String>` SHALL read the shape Herdr 0.8.2
actually emits:

```json
{"id":"cli:agent:list","result":{"agents":[ … ],"type":"agent_list"}}
```

The agents live at `result.agents`, and `parse_list` SHALL return `Err(reason)` naming what
was missing when `result` is absent, when `result.agents` is absent, or when either is not of
the expected type. `SPEC.md` → Agent status by polling described the per-agent fields but not
this envelope; correcting that sentence is part of this change.

`Listed` SHALL carry exactly two fields: `agents: Vec<Agent>` and `problems: Vec<String>` —
the entries that could not be read, named one per line, so a payload carrying one unusable
entry still yields every other agent.

`agents::Agent` SHALL carry exactly **eight** fields, in this order:

```rust
pub struct Agent {
    pub name: Option<String>,          // Herdr's `name` — absent unless the agent was named
    pub kind: Option<String>,          // Herdr's `agent` — the agent KIND, e.g. "claude"
    pub status: AgentStatus,
    pub cwd: Option<PathBuf>,
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    pub terminal_title: Option<String>,
}
```

`name` and `kind` SHALL be separate fields carrying separate JSON keys, and the distinction is
load-bearing rather than cosmetic. Herdr's `agent` key holds the agent **kind** — every one of
the two agents on the reference machine reports `"agent":"claude"` — while the name a user or
this plugin gives an agent lives under `name`, which Herdr **omits entirely** when unset.
`SPEC.md` → Agent status by polling lists `agent` and no `name`, and `SPEC.md` → Attributing
an agent to a change then attributes "any live agent whose name equals a change name": read
together, those two sentences would have had `agent-attribution` matching change names against
the string `claude` for every agent in the session. Correcting the field list is part of this
change, and this requirement is where the corrected contract is written down.

`AgentStatus` SHALL be an enum of exactly `Working`, `Idle`, `Blocked`, `Done`, and `Unknown`,
matching Herdr's own `AgentStatus` schema, and `Unknown` SHALL be what any other string, and an
absent `agent_status`, decode to.

`Agent`, `Listed`, and `AgentSnapshot` SHALL NOT implement `Default` — neither derived nor
hand-written, anywhere in the crate — and every construction and every destructuring of any of
the three SHALL name every field with no `..` rest, so a field added later fails to compile at
each site rather than defaulting silently. This is `dashboard-loop`'s existing rule for
`Dashboard`, `Filter`, `Detail`, and `Refresh`, extended to the three structs this change adds,
and it matters more here than there: a parser filling eight mostly-optional fields out of JSON
is exactly the shape `..Default::default()` is reached for.

`AgentStatus` is deliberately **outside** that rule, and the exemption is stated rather than
left as an oversight. The gate that enforces it is a source sweep anchored on `struct <T> {`,
so an enum cannot be added to its type list without breaking its own positive control; and a
`Default` on `AgentStatus` would change nothing, because every construction site is a `match`
arm naming a variant and adding a sixth variant is already a compile error at that `match`.

`Agent` deliberately carries **no** `terminal_id`, even though Herdr's schema marks it
required. Nothing in Phase 5 addresses a terminal: `herdr agent focus`, `herdr agent prompt`,
and `herdr pane split` all take a pane or an agent name, which `pane_id` and `name` already
carry. A field no consumer reads is a field every construction site must still fill.

#### Scenario: The reference payload parses into one agent

- **WHEN** `parse_list` is given the payload captured verbatim from Herdr 0.8.2 —
  `{"id":"cli:agent:list","result":{"agents":[{"agent":"claude","agent_session":{"agent":"claude","kind":"id","source":"herdr:claude","value":"0e80c276-952e-4150-b32f-06cc6247ce01"},"agent_status":"idle","cwd":"/repo","focused":true,"foreground_cwd":"/repo","pane_id":"w8:p1","revision":35,"state_change_seq":963,"tab_id":"w8:t1","terminal_id":"term_65a34df386c314","terminal_title":"✳ a title","terminal_title_stripped":"a title","workspace_id":"w8"}],"type":"agent_list"}}`
- **THEN** it returns `Ok(Listed { agents, problems })` with `problems` empty and `agents`
  holding exactly one `Agent`
- **AND** that agent is `Agent { name: None, kind: Some("claude"), status: AgentStatus::Idle,
  cwd: Some("/repo"), pane_id: "w8:p1", tab_id: "w8:t1", workspace_id: "w8", terminal_title:
  Some("✳ a title") }` — asserted as one whole-value equality, not field by field, so a
  field silently left unset fails
- **AND** `name` is `None` even though `kind` is `Some("claude")`, which is the whole point of
  the two being separate fields

#### Scenario: An empty session lists no agents and is not an error

- **WHEN** `parse_list` is given `{"id":"cli:agent:list","result":{"agents":[],"type":"agent_list"}}`
- **THEN** it returns `Ok(Listed { agents: [], problems: [] })`
- **AND** this is distinct from an unreachable socket: a reachable socket with nothing running
  is a successful poll that found nothing, and `agent-attribution` must be able to tell the
  two apart

#### Scenario: A named agent carries its name

- **WHEN** `parse_list` is given a one-entry payload whose object additionally holds
  `"name":"agent-polling"`
- **THEN** the parsed agent's `name` is `Some("agent-polling")` and its `kind` is still
  `Some("claude")`

#### Scenario: Every status string decodes, and an unrecognised one is `Unknown`

- **WHEN** `parse_list` is given a payload of five entries whose `agent_status` values are
  `"working"`, `"idle"`, `"blocked"`, `"done"`, and `"unknown"`, and then again with a sixth
  entry whose value is `"reticulating"`, and again with a seventh whose `agent_status` key is
  absent, and again with an eighth whose `agent_status` is the number `3`
- **THEN** the first five decode to `Working`, `Idle`, `Blocked`, `Done`, and `Unknown`
  respectively
- **AND** the sixth, seventh, and eighth all decode to `AgentStatus::Unknown` and produce **no**
  entry in `problems`: an unrecognised status is a forward-compatible fact about a newer Herdr,
  not a broken payload, and the plugin never fails closed on one

### Requirement: Absent optional fields degrade, and a missing identity skips one entry

Only seven of Herdr's `AgentInfo` fields are required by its own schema — `terminal_id`,
`agent_status`, `workspace_id`, `tab_id`, `pane_id`, `focused`, and `revision` — and the
serializer **omits** every field whose value is null or default. `agent`, `cwd`, `name`, and
`terminal_title` are therefore routinely absent from a real payload, and `parse_list` SHALL
treat every one of them as `None` rather than as a defect.

`pane_id`, `tab_id`, and `workspace_id` are the three of those seven this crate reads. An entry
missing any of them, holding a non-string in any of them, or not being a JSON object at all
SHALL be **skipped** with one line in `problems` naming the fault and the entry's position in
the array, and every other entry in the same payload SHALL still parse. This is the crate's
established shape — a single bad row does not fail an entire read — and it is why `problems`
exists beside `agents` rather than the whole parse returning `Err`.

#### Scenario: An entry with only the required identity fields still parses

- **WHEN** `parse_list` is given a one-entry payload holding only
  `{"pane_id":"w1:p1","tab_id":"w1:t1","workspace_id":"w1","focused":false,"revision":1,"terminal_id":"t","agent_status":"working"}`
- **THEN** it returns one `Agent` whose `name`, `kind`, `cwd`, and `terminal_title` are all
  `None` and whose `status` is `Working`
- **AND** `problems` is empty: an agent Herdr has not named, in a pane whose working directory
  it does not report, is an ordinary agent

#### Scenario: One entry missing `pane_id` is skipped and the others survive

- **WHEN** `parse_list` is given a three-entry payload whose middle entry has no `pane_id` key
- **THEN** `agents` holds the first and third entries, in that order
- **AND** `problems` holds exactly one line, naming `pane_id` and the index `1`
- **AND** the same holds when the middle entry's `pane_id` is the number `7` rather than a
  string, and again when it is `tab_id` and `workspace_id` that are missing

#### Scenario: An entry that is not an object is skipped, not fatal

- **WHEN** `parse_list` is given a three-entry payload whose middle entry is the string
  `"claude"`, and again with the number `7`, and again with `null`
- **THEN** in each case `agents` holds the first and third entries and `problems` holds exactly
  one line naming the index `1` and the fact that the entry was not an object
- **AND** the whole parse still returns `Ok`: one malformed row does not discard the rows around
  it, which is the same rule the missing-`pane_id` scenario states one level down

#### Scenario: A `cwd` outside the repository is still parsed

- **WHEN** `parse_list` is given an entry whose `cwd` is `/somewhere/else`
- **THEN** the agent parses with `cwd: Some("/somewhere/else")`
- **AND** no filtering, canonicalization, or repository comparison happens here: deciding which
  agents are "in the repository" is `agent-attribution`'s work, and doing it in the parser
  would put a filesystem call inside a pure function

### Requirement: An unusable payload names its reason and yields no agents

`parse_list` SHALL return `Err(reason)` — never a panic and never a silently empty `Ok` — for
text that is not JSON, for JSON that is not an object, for an object with no `result`, for a
`result` with no `agents`, and for an `agents` value that is not an array. The reason SHALL
name what was looked for.

When the text is instead Herdr's **error** envelope — `{"id":…,"error":{"code":…,"message":…}}`
— `parse_list` SHALL return `Err` naming the `code` and the `message`, rather than the generic
"no `result`" reason. Herdr writes that envelope to **stderr** and exits non-zero, so this
branch is defensive rather than the normal path; it exists because a payload that already
explains itself should not be reported as a shape mismatch.

#### Scenario: Text that is not JSON is a named reason

- **WHEN** `parse_list` is given `usage: herdr agent list` — the exact text Herdr 0.8.2 writes
  when the call is malformed
- **THEN** it returns `Err(reason)` whose text names the parse failure
- **AND** it does not panic and returns no partial list

#### Scenario: A well-formed envelope with the wrong contents is a named reason

- **WHEN** `parse_list` is given, in turn, `{}`, `{"result":{}}`,
  `{"result":{"agents":{}}}`, and `[]`
- **THEN** each returns `Err(reason)`, and each reason names the key or the type that was
  expected
- **AND** none of the four panics

#### Scenario: Herdr's own error envelope is reported by its code and message

- **WHEN** `parse_list` is given
  `{"id":"cli:agent:list","error":{"code":"server_not_running","message":"no herdr server is running at /p/herdr.sock"}}`
- **THEN** it returns `Err(reason)` whose text contains both `server_not_running` and the
  message
- **AND** the reason does not read as a missing-`result` shape complaint, because the payload
  said what was wrong

### Requirement: A failed run is an unreachable socket carrying the program's own reason

`agents::poll_once` SHALL map `cli.run`'s outcome to an `AgentSnapshot` carrying exactly three
fields — `agents: Vec<Agent>`, `reachable: bool`, and `problem: Option<String>`:

- `Ok(text)` that parses: `AgentSnapshot { agents, reachable: true, problem }`, where `problem`
  is `None` when `Listed::problems` is empty and otherwise carries those lines joined into one
  string. A payload with one bad entry is still a **reachable** socket.
- `Ok(text)` that does not parse: `AgentSnapshot { agents: [], reachable: false, problem:
  Some(reason) }`.
- `Err(CliError::Failed { code, stderr, .. })`: `reachable: false`, with `problem` naming
  `herdr agent list`, the exit code, and **stderr**.
- `Err(CliError::NotStarted { reason, .. })`: `reachable: false`, with `problem` naming
  `herdr agent list` and the operating system's reason.

Carrying stderr is correct **here specifically**, and the contrast is worth stating because
`SPEC.md` → Degraded states currently generalises the opposite. The OpenSpec CLI writes its
diagnostic to stdout, so `CliError::Failed`'s stderr is empty for it and the row recording
"the reason is unavailable to the plugin" is true. Herdr does the reverse: an unreachable
socket exits **1** with an empty stdout and a JSON error envelope on **stderr**, so for
`herdr` the reason *is* available and the pane should carry it. That row is scoped to the
`openspec` CLI as part of this change.

`reachable: false` SHALL NOT become a `!`-marked problem row. `SPEC.md` → Degraded states
records an unreachable socket as "runs as a standalone TUI; agent column and action keys
hidden" — silence, not a message — and `Dashboard::refresh.problems`, which is what renders as
a leading `!` row, is therefore the wrong home for it. This is also why `problem` is a sibling
field rather than an entry on `ChangeSet::problems`: `Dashboard::adopt` replaces that vector
wholesale on every refresh, so a standing condition put there would flicker away on the next
file result.

#### Scenario: An unreachable socket is a snapshot, not an error

- **WHEN** `poll_once` is given a `FakeCli` whose `HerdrCli` response for `["agent", "list"]`
  is `Err(CliError::Failed { program: "herdr", args: ["agent","list"], code: Some(1), stderr:
  "{\"id\":\"cli:agent:list\",\"error\":{\"code\":\"server_not_running\",\"message\":\"no herdr server is running at /p/herdr.sock\"}}" })`
- **THEN** it returns `AgentSnapshot { agents: [], reachable: false, problem: Some(text) }`
- **AND** `text` names `herdr agent list`, the exit code `1`, and `server_not_running`, so the
  reason a user would need is present rather than discarded
- **AND** `poll_once` returns normally: it has no error type and cannot fail

#### Scenario: No `herdr` program at all is the standalone-TUI case

- **WHEN** `poll_once` is given a `FakeCli` whose response is
  `Err(CliError::NotStarted { program: "herdr", args: ["agent","list"], reason: "No such file or directory (os error 2)" })`
- **THEN** it returns `reachable: false` with a `problem` naming the program and that reason
- **AND** `agents` is empty, so nothing downstream can read a stale agent out of a snapshot
  taken while the socket was unreachable

#### Scenario: A payload with one bad entry is still reachable

- **WHEN** `poll_once` is given a successful response whose payload holds three entries, one of
  them missing `pane_id`
- **THEN** it returns `reachable: true` with two agents
- **AND** `problem` is `Some(text)` naming the skipped entry, so a partial read is visible
  without being mistaken for an unreachable socket
