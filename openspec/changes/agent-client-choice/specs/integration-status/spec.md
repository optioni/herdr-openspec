## Purpose
Which coding-agent client the pane launches, decided from evidence rather than from a
constant. `herdr integration status` reports, one line per integration, which agent
integrations the user has actually set up; this capability parses that plain text — there
is no `--json` form — and folds it into a five-step precedence that puts an explicit
`config.toml` choice first, a recorded choice second, a sole installed integration third,
a refusal naming the candidates fourth, and `claude` last. Parsing and precedence are pure
and total: the read itself happens once per session, lazily, on the launcher's own worker
thread, and an unreadable or unparseable status degrades to the last resort with a named
problem rather than blocking a launch.

## ADDED Requirements

### Requirement: `herdr integration status` parses to an ordered list of kinds and their installed state

`integration` SHALL be a new top-level module, `src/integration.rs`, holding the status
parser and the kind precedence and nothing else. It SHALL be pure and total on exactly
`src/specs.rs`' terms: no filesystem, process, environment, network, or standard-I/O API,
no clock, no global mutable state, no `ratatui` type, no `HerdrCli` handle, and no panic
for any input, including the empty string. It SHALL NOT be placed under `src/ui/`.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Integration {
    pub kind: String,
    pub status: String,
    pub installed: bool,
}

pub fn parse(text: &str) -> (Vec<Integration>, Vec<String>);
```

`status` SHALL carry the parsed status text verbatim. It exists so the split rule below is
**observable**: with only `kind` and `installed`, a first-` (` split and a last-` (` split agree
on every line Herdr actually prints, so no assertion over the real corpus can tell a correct
parser from `split_once(" (")`. Exposing the field is what makes the rule falsifiable.

`parse` SHALL read one integration per non-blank line in the shape
`<kind>: <status> (<path>)`, measured against Herdr 0.9.0:

```text
pi: not installed (/Users/x/.pi/agent/extensions/herdr-agent-state.ts)
claude: current (v9) (/Users/x/.claude/hooks/herdr-agent-state.sh)
codex: current (v8) (/Users/x/.codex/herdr-agent-state.sh)
```

The kind SHALL be the text before the **first** `: `, trimmed. The status SHALL be
everything after it up to the **last** ` (` on the line, trimmed — not the first, because
`current (v9) (/path)` carries a parenthesised version before the parenthesised path and a
first-match split would yield the status `current`. `installed` SHALL be `false` when and
only when the status is exactly `not installed`; every other status, `current (v9)` and any
future `outdated (v7)` alike, SHALL be `installed` `true`, because this capability asks
whether the user set the integration up, not which version they set up.

Order SHALL be preserved as Herdr printed it. A line with no `: ` separator, an empty kind,
or no ` (` SHALL be skipped and SHALL contribute one problem naming the line, while every
well-formed line in the same output is still returned — `state::read`'s established
per-entry rule.

#### Scenario: The measured 17-line status parses whole

- **WHEN** `parse` is given the seventeen-line output measured on the reference machine, in
  which `claude` reads `current (v9)`, `codex` reads `current (v8)`, and the other fifteen
  read `not installed`
- **THEN** it returns seventeen `Integration` values in Herdr's own order, the first being
  `Integration { kind: "pi", status: "not installed", installed: false }`
- **AND** exactly two carry `installed` `true`, and their kinds are `claude` and `codex`
- **AND** the problems vector is empty

#### Scenario: A version in the status does not become the status

- **WHEN** `parse` is given the single line
  `claude: current (v9) (/Users/x/.claude/hooks/herdr-agent-state.sh)`
- **THEN** it returns one `Integration` with kind `claude`, `status` **`current (v9)`**, and
  `installed` `true`
- **AND** the `status` assertion is what discriminates the rules: a first-` (` split yields
  `current`, which is a different string and fails the assertion, while both splits would
  agree on `installed` and so prove nothing on their own

#### Scenario: A parenthesised version on an absent integration discriminates the split

- **WHEN** `parse` is given the synthetic line `codex: not installed (v1) (/p/hook.sh)`
- **THEN** it returns `status` `not installed (v1)` and `installed` **`true`**
- **AND** a first-` (` split would yield `status` `not installed` and `installed` **`false`**,
  so this one line separates a correct parser from `split_once(" (")` on `installed` alone —
  the case no line Herdr actually prints can reach, because every measured `not installed`
  line carries no parenthesised version

#### Scenario: An unrecognised status is treated as installed

- **WHEN** `parse` is given `codex: outdated (v3) (/Users/x/.codex/herdr-agent-state.sh)`
- **THEN** the returned `Integration` carries `installed` `true`
- **AND** no problem is reported: an out-of-date integration is one the user set up, which
  is the only question this capability asks

#### Scenario: A malformed line is skipped and the rest survive

- **WHEN** `parse` is given three lines — a well-formed `claude` line, the line
  `this is not an integration line`, and a well-formed `codex` line
- **THEN** it returns exactly two `Integration` values, for `claude` and `codex`
- **AND** exactly one problem is reported, and it names the skipped line
- **AND** no panic occurs

#### Scenario: Empty and blank input yield nothing and report nothing

- **WHEN** `parse` is given the empty string, and again a string of three blank lines
- **THEN** both return an empty `Vec<Integration>`
- **AND** both report no problem, because a blank read is not a malformed read — the
  caller's own failure path, not this one, reports a status that could not be obtained

### Requirement: The agent kind is resolved by a five-step precedence, as a pure total function

`integration::resolve` SHALL hold the whole precedence and SHALL be a pure, total function
of its three arguments, performing no I/O of any kind and never panicking:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source { Configured, Recorded, SoleIntegration, LastResort }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    Use { kind: String, source: Source },
    Ambiguous { installed: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub choice: Choice,
    pub problems: Vec<String>,
}

pub fn resolve(
    configured: Option<&str>,
    recorded: Option<&str>,
    integrations: &[Integration],
) -> Resolved;
```

The order SHALL be exactly:

1. `configured` non-blank → `Use { kind, source: Configured }`. A hand-edited `config.toml`
   choice always wins, and no further evidence is consulted.
2. `recorded` non-blank → `Use { kind, source: Recorded }`. This is what `settings-window`
   will write under `HERDR_PLUGIN_STATE_DIR`; this change reads it and never writes it.
3. Exactly one `Integration` with `installed` `true` → `Use { kind, source: SoleIntegration }`.
   One installed integration is not a guess between candidates; it is the only candidate.
4. Two or more installed → `Ambiguous { installed }`, carrying their kinds in Herdr's own
   order. The plugin SHALL NOT pick between them, and the caller answers with a problem row.
5. Zero installed → `Use { kind: "claude", source: LastResort }`.

Step 5 exists so the pane never fails closed, not because Claude Code is presumed:
`SPEC.md` → Never fail closed forbids replacing a working action with a refusal when no
evidence is available at all. Step 4 refuses instead, because there the evidence exists and
points two ways at once, and picking one would be exactly the guess
`agent-attribution` refuses to make about a terminal title.

The literal `claude` SHALL appear in `src/integration.rs` and SHALL NOT appear in any
production file under `src/ui/`, on exactly the terms `WIRED`'s leg 6 already enforces — the
gate that greps every production slice under `src/ui/` for `"claude"`, and which also requires
`src/ui/mod.rs`'s own production slice to name `config.agent_kind`. Threading
`Config::agent_kind` into `launch::Settings` keeps that second condition satisfied.

#### Scenario: A configured kind beats every other source

- **WHEN** `resolve` is called with `configured` `Some("codex")`, `recorded` `Some("gemini")`,
  and integrations in which only `claude` is installed
- **THEN** it returns `Use { kind: "codex", source: Configured }`
- **AND** the recorded choice and the sole installed integration are both ignored, so a
  hand-edited `config.toml` is never overruled by evidence the plugin gathered itself

#### Scenario: A recorded choice beats the evidence but not the configuration

- **WHEN** `resolve` is called with `configured` `None`, `recorded` `Some("gemini")`, and
  integrations in which `claude` and `codex` are installed
- **THEN** it returns `Use { kind: "gemini", source: Recorded }`
- **AND** the same call with `recorded` `None` returns
  `Ambiguous { installed: ["claude", "codex"] }`, so the recorded value is what suppressed
  the refusal and nothing else in the fixture is

#### Scenario: A sole installed integration is used without asking

- **WHEN** `resolve` is called with `configured` `None`, `recorded` `None`, and the
  seventeen measured integrations in which only `codex` is installed
- **THEN** it returns `Use { kind: "codex", source: SoleIntegration }`
- **AND** no problem is reported: a Codex-only user who has never opened `config.toml`
  launches Codex, which is the behaviour change this capability exists to make

#### Scenario: Two installed integrations are refused, not guessed between

- **WHEN** `resolve` is called with `configured` `None`, `recorded` `None`, and the
  seventeen measured integrations, in which `claude` and `codex` are installed
- **THEN** it returns `Ambiguous { installed: ["claude", "codex"] }`, in Herdr's own
  printed order
- **AND** `problems` is empty: `Ambiguous` is the answer, and turning it into a reason the
  reader sees is `agent-launch`'s job, not this function's
- **AND** no kind is returned at all, so no caller can accidentally launch the first one

#### Scenario: Nothing installed and nothing configured reaches `claude` and says so

- **WHEN** `resolve` is called with `configured` `None`, `recorded` `None`, and seventeen
  integrations of which none is installed
- **THEN** it returns `Use { kind: "claude", source: LastResort }`
- **AND** `problems` holds exactly one entry, naming that no agent integration is installed
  and no `agent_kind` is configured, and that `claude` is being launched as a last resort
- **AND** the launch still goes ahead — the entry is a warning beside a working action, not
  a refusal, because this is the case that is silent today

#### Scenario: A blank configured or recorded value is not a value

- **WHEN** `resolve` is called with `configured` `Some("")`, then `Some("   ")`, then
  `Some("\t")`, in each case with `recorded` `None` and only `codex` installed
- **THEN** all three return `Use { kind: "codex", source: SoleIntegration }`
- **AND** the same three values passed as `recorded` with `configured` `None` behave
  identically, so blankness is rejected at both steps by the same rule
- **AND** a configured value with surrounding whitespace, `Some("  codex  ")`, returns
  `Use { kind: "codex", source: Configured }` — trimmed, not rejected

#### Scenario: Every combination is total

- **WHEN** `resolve` is called with each of `None`, `Some("")`, and `Some("x")` for both
  `configured` and `recorded`, against each of an empty slice, a slice with one installed
  entry, and a slice with two installed entries
- **THEN** none of the twenty-seven calls panics
- **AND** every one returns either a `Use` whose `kind` is non-blank or an `Ambiguous` whose
  `installed` holds at least two entries — there is no third shape and no empty kind

### Requirement: A chosen kind whose integration is absent is warned about and launched anyway

When `resolve` returns `Use` with source `Configured` or `Recorded` and that kind is **not**
among the installed integrations, it SHALL report exactly one problem naming the kind and
saying that its integration is not installed, so every agent the plugin starts under it will
report status `unknown`. The kind SHALL still be returned and the launch SHALL still proceed.

This is not an availability check and SHALL NOT be treated as one: `integration status`
reports whether Herdr's status-reporting hook is installed, not whether the client's
executable exists. A kind launches fine without its integration; what is lost is the status
that `agent-poller`, `agent-attribution`, and the list badges all read. Measured on the
reference machine, the one live agent is a `claude` agent whose integration reads
`current (v9)` and which reports `working` rather than `unknown`.

`SoleIntegration` cannot reach this path by construction, and `LastResort` already carries
its own problem, so this warning SHALL be reported for the first two sources only.

#### Scenario: A configured kind with no integration warns and still resolves

- **WHEN** `resolve` is called with `configured` `Some("cursor")` against the seventeen
  measured integrations, in which `cursor` reads `not installed`
- **THEN** it returns `Use { kind: "cursor", source: Configured }`
- **AND** `problems` holds exactly one entry naming `cursor` and the word `unknown`
- **AND** the same call with `configured` `Some("codex")`, which **is** installed, returns
  the same shape with an **empty** `problems`, so the warning is caused by the absence and
  by nothing else

#### Scenario: A kind Herdr never listed is warned about on the same terms

- **WHEN** `resolve` is called with `configured` `Some("not-a-kind")` against the seventeen
  measured integrations
- **THEN** it returns `Use { kind: "not-a-kind", source: Configured }` with one problem
- **AND** the plugin SHALL NOT consult `herdr agent start --kind`'s closed 23-value enum to
  reject it: that list lives in `--help` text rather than a machine-readable API, and
  `agent start` fails with its own reason on an unsupported kind, which `agent-launch`
  already carries into a problem row

#### Scenario: A sole integration and a last resort carry no absence warning

- **WHEN** `resolve` resolves to `SoleIntegration` for `codex`, and again to `LastResort`
  for `claude` with nothing installed
- **THEN** the `SoleIntegration` result reports no problem at all
- **AND** the `LastResort` result reports exactly one problem, the nothing-installed
  warning, and not a second one about `claude`'s own integration being absent

### Requirement: The status is read at most once per session, lazily, and never at startup

`herdr integration status` SHALL be invoked at most **once** per process, and only from the
launcher's worker thread on the first `Request::Launch` it handles. It SHALL NOT be invoked
by `ui::start_collaborators`, by `ui::run_wired`, by the render loop, or by any file under
`src/ui/`. A pane whose reader never presses `a`, `c`, or `s` SHALL issue no
`integration status` call at all.

The resolved `Choice` SHALL be cached in the worker for the rest of the session, so a second
and every later launch reuse it and issue no further status call. `Request::Focus` SHALL
never trigger the read: `g` focuses an agent that is already running and needs no kind.

**The resolution's own problems SHALL be reported on the launch that produced them and on
no other.** They describe a read and a decision that happened once, so replaying them on
every later launch would report an event that did not recur — and `agent-launch`'s "A
success clears both entries" is unsatisfiable within one worker if they are replayed, since
a cached resolution that warned would keep re-warning through a launch in which everything
succeeded. `Choice::Ambiguous` is the one exception, and it is not an exception to this
rule: its problem is **re-derived** from the cached `Choice` on every press, because it is
the refusal itself rather than a warning beside a working action, and a key that refuses
must say why every time it is pressed.

Reading it lazily rather than at startup is what keeps this capability off the composition
root's critical path and out of the render path entirely — `SPEC.md` → "The render path
blocks on nothing but the terminal" — and costs nothing on a pane that never launches
anything.

#### Scenario: The first launch reads the status and the second does not

- **WHEN** a worker is driven with two `Request::Launch` values in turn against a scratch
  `herdr` program that logs every argument vector
- **THEN** the log holds exactly one `integration status` entry
- **AND** it precedes the first launch's `pane split`
- **AND** the second launch's three calls follow with no second `integration status`
  between them, so the resolution is cached rather than re-read

#### Scenario: Focus never reads the status

- **WHEN** a worker is driven with a `Request::Focus` and nothing else
- **THEN** the log holds exactly one entry, `agent focus <pane id>`
- **AND** no `integration status` entry appears, so `g` costs no extra Herdr call

#### Scenario: Startup issues no status call

- **WHEN** `run_wired` is driven against a scratch `herdr` program and the event source
  presses only `q`
- **THEN** the log holds no `integration status` entry at all, whatever the poller wrote
- **AND** the same run with `a` pressed before `q` does hold exactly one, so the absence
  above is caused by the key never being pressed

### Requirement: An unusable status read degrades to the last resort and never blocks a launch

A failed `herdr integration status` call — a non-zero exit, a failure to start, or a
timeout — SHALL be reported as exactly one problem carrying Herdr's own reason on
`agents::herdr_error_problem`'s established terms, and resolution SHALL then continue as
though **no** integrations were reported. Output that parses to no integrations at all
SHALL be treated the same way.

That means the precedence still runs: a configured or recorded kind is still honoured with
no warning about its integration — the absence cannot be established when the status could
not be read — and with neither set, the result is `claude` under `LastResort` with its own
problem beside the read failure. A launch SHALL NOT be refused because the status was
unreadable.

**The resolution SHALL contribute at most two problems to a launch's `Outcome`**, and the two
are of distinct kinds, at most one each:

1. **Obtaining the status** — a call failure carrying Herdr's reason, **or** a single summary
   naming how many lines could not be parsed. The two are mutually exclusive: a call that
   failed produces no output to parse.
2. **The kind chosen** — the last-resort warning, **or** the absent-integration warning. Also
   mutually exclusive, because they belong to different `Source` values.

`parse`'s per-line problems SHALL be summarised into that one entry when they reach the
outcome, rather than forwarded one per line. `parse` itself still returns them individually —
they are useful and asserted at the unit tier — but a seventeen-line status whose format
changed would otherwise put eighteen `! ` rows above the change list, which is not a bound.
`change-rows` reasons about that cost, so it is stated here and fixed at two.

#### Scenario: A failed status call still launches the configured kind

- **WHEN** the scratch `herdr` program answers `integration status` with exit `1` and
  `{"error":{"code":"integration_unavailable","message":"no socket"}}` on stderr, and
  `config.toml` sets `agent_kind = "codex"`
- **THEN** the launch proceeds and `agent start`'s `--kind` is `codex`
- **AND** `outcome.problems` holds exactly one entry, naming `integration_unavailable` and
  `no socket`
- **AND** no second problem claims `codex`'s integration is absent, because the read that
  would have established that never succeeded

#### Scenario: A failed status call with nothing configured reaches `claude` with two problems

- **WHEN** the same failing `integration status` is answered with no `agent_kind`
  configured and no recorded choice
- **THEN** the launch proceeds and `agent start`'s `--kind` is `claude`
- **AND** `outcome.problems` holds exactly two entries in occurrence order: the read failure
  naming `integration_unavailable` first, then the last-resort warning
- **AND** `named` is `Some((derived name, change))`, so a status the plugin could not read
  costs the reader an explanation and nothing else

#### Scenario: Unparseable output is not a failed call

- **WHEN** `integration status` exits `0` and prints three lines none of which parse
- **THEN** the launch proceeds under `LastResort` with `claude`
- **AND** `outcome.problems` holds exactly **two** entries: one summary naming that three
  lines could not be parsed, then the last-resort warning
- **AND** `parse` called directly on that same input returns **three** problems, one per line,
  so the summary happens on the way to the outcome and the unit tier keeps the detail
- **AND** the same run against a seventeen-line status none of which parses still yields
  exactly two entries, so the bound does not grow with the input
- **AND** the launch is not refused, so a Herdr whose output format changed degrades the
  explanation rather than the pane
