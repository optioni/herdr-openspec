## Purpose
The text `agent prompt` carries, for each of the three launch intents. One CLI-driven
shape serves every agent kind, `claude` included: a short instruction to run the plugin's
own resolved `openspec` binary and follow what it returns. The `/opsx:*` shortcuts are
dropped — they are a Claude Code plugin's shortcut rather than a universal idea, they fail
in any Claude Code without the `opsx` plugin installed, and no other client has an
equivalent to translate to. What generalises is the CLI underneath, so adding a client
requires no mapping at all. `config.toml` keeps a per-kind override for the reader who
disagrees, and a pane with no resolved binary has no path to name and therefore no prompt
to send.

## ADDED Requirements

### Requirement: The built-in prompt is one CLI-driven shape per intent, identical for every kind

`launch::prompt_text` SHALL be a pure, total function producing the text for one intent, and
SHALL depend on the intent, the change name, and the resolved `openspec` path **only** —
never on the agent kind. It SHALL NOT panic for any input, including an empty change name
and a non-UTF-8 binary path, which SHALL be rendered with `Path::to_string_lossy` on exactly
the repository root's established terms.

```rust
pub fn prompt_text(
    intent: Intent,
    change: &str,
    openspec: &std::path::Path,
    overrides: &std::collections::BTreeMap<String, String>,
) -> String;
```

With no override, the text SHALL be exactly:

| Intent | Text |
|---|---|
| `Apply` | `Run: <openspec> instructions apply --change <change> --json. Follow the instruction it returns to implement this OpenSpec change.` |
| `Continue` | `Run: <openspec> status --change <change> --json. Create the next artifact it reports as ready, using <openspec> instructions <artifact-id> --change <change> --json.` |
| `Archive` | `Run: <openspec> archive <change> --yes. Report what it changed.` |

`Intent::Focus` SHALL never reach this function; `run_request` handles it through
`focus_args` instead.

Each is a **single** argument-vector element. The seam passes arguments to the program
directly with no shell, so no quoting is applied and none is needed; the text SHALL contain
no quote character, so a reader comparing the logged vector against the table above compares
byte for byte.

These three commands are measured working against `openspec` 1.13.0:
`instructions apply --change <name> --json` returns the apply instruction and the
context-file list even for a change whose artifacts are incomplete, naming what is missing;
`status --change <name> --json` names each artifact's `status` and its `requires` edges;
`archive <name>` takes `--yes` to skip its confirmation prompt. The prompt is short rather
than carrying the whole procedure because an agent started in this repository reads
`AGENTS.md`, which documents the OpenSpec workflow.

#### Scenario: Each intent produces its own text against the same binary and change

- **WHEN** `prompt_text` is called for `Apply`, `Continue`, and `Archive` with the change
  `2fa-support`, the path `/opt/bin/openspec`, and no overrides
- **THEN** the three strings are exactly the table's rows with `<openspec>` replaced by
  `/opt/bin/openspec` and `<change>` by `2fa-support`
- **AND** none contains a `'` or a `"` character
- **AND** none contains the text `/opsx:`

#### Scenario: The kind does not reach the prompt

- **WHEN** the same three calls are made while the resolved kind is `claude`, and again
  while it is `codex`, and again while it is `not-a-kind`
- **THEN** all three runs produce byte-identical text, because `prompt_text` takes no kind
  argument at all
- **AND** adding a fourth client therefore requires no change to this function and no new
  mapping entry anywhere

#### Scenario: An empty change name and a lossy path are rendered, not refused

- **WHEN** `prompt_text` is called for `Apply` with the change `""` and a binary path whose
  bytes are not valid UTF-8
- **THEN** it returns a string rather than panicking
- **AND** the path appears as `Path::to_string_lossy` rendered it, so a launch is never
  refused for a path this plugin cannot spell — the agent fails on it honestly instead

### Requirement: `/opsx:*` is dropped, and the prompt names the resolved absolute path

The three `/opsx:apply`, `/opsx:continue`, and `/opsx:archive` prompt texts SHALL be removed.
They are one more branch and one more failure mode — they fail in any Claude Code that does
not have the `opsx` plugin installed — while the CLI shape works in all of them.

The prompt SHALL name the plugin's own **resolved absolute path** to `openspec`, not the bare
command `openspec`. Measured on the reference machine, a fresh interactive `zsh` with a reset
`PATH` reports `openspec not found` even though `.zshrc` references nvm, because nvm is
lazy-loaded; the plugin's four-step probe — configuration, `PATH`, nvm, `npm prefix -g` — is
strictly more thorough than a shell lookup, so a bare `openspec` in the prompt would fail for
an agent even when the plugin itself found one.

The path SHALL be the one `resolve::openspec_bin` reported to the composition root, threaded
into the launcher, and SHALL NOT be re-probed by the launcher or by any file under `src/ui/`.

#### Scenario: The prompt carries the probe's own path, not a bare command

- **WHEN** a launch runs with the resolved binary at
  `/Users/x/.nvm/versions/node/v24.20.0/bin/openspec`
- **THEN** the logged `agent prompt` vector's fourth element begins
  `Run: /Users/x/.nvm/versions/node/v24.20.0/bin/openspec instructions apply`
- **AND** the element contains no occurrence of the standalone word `openspec` that is not
  part of that absolute path

#### Scenario: No production file still produces an `/opsx:` prompt

- **WHEN** the crate's production slice is searched for the literal `/opsx:`
- **THEN** it appears in no production file
- **AND** the three launch scenarios in `agent-launch` that asserted
  `agent prompt <name> /opsx:apply <change>` are restated against the CLI shape, so the
  removal is proven by a passing assertion rather than by an absence

### Requirement: A per-kind override from `config.toml` replaces the built-in text

`Config` SHALL carry per-kind prompt overrides, and when the resolved kind has an override
for the intent being launched, that text SHALL be used in place of the built-in one. The
override SHALL be applied after kind resolution, so an override is keyed by the kind the
plugin actually resolved and never by the one it might have chosen.

Two placeholders SHALL be substituted in an override, at every occurrence:

| Placeholder | Replaced by |
|---|---|
| `{openspec}` | the resolved absolute `openspec` path |
| `{change}` | the change name |

Any other brace-delimited text SHALL be left verbatim — the plugin SHALL NOT reject it, warn
about it, or attempt to expand it, so an override written for a future placeholder degrades
to literal text rather than to a failed launch. An override that contains neither placeholder
SHALL be sent exactly as written.

An override SHALL be looked up by the exact intent name `apply`, `continue`, or `archive`. A
kind with an override for one intent and not another SHALL use the built-in text for the
others.

#### Scenario: An override replaces one intent's text and leaves the others built-in

- **WHEN** `config.toml` carries
  `[prompts.codex]` with `apply = "work on {change} using {openspec}"`, and a launch for
  `2fa-support` resolves the kind `codex` against `/opt/bin/openspec`
- **THEN** `Apply`'s prompt is exactly `work on 2fa-support using /opt/bin/openspec`
- **AND** `Continue`'s and `Archive`'s prompts are the built-in table rows, unchanged
- **AND** the same launch with the resolved kind `claude` uses the built-in `Apply` text, so
  the override is keyed by kind

#### Scenario: A placeholder appearing twice is substituted twice

- **WHEN** an override reads `{change}: run {openspec}, then {openspec} status, for {change}`
- **THEN** every one of the four occurrences is replaced
- **AND** the result contains no `{change}` and no `{openspec}`

#### Scenario: An unknown placeholder is left verbatim

- **WHEN** an override reads `apply {change} with {agent} at {schema}`
- **THEN** `{change}` is substituted and `{agent}` and `{schema}` appear literally in the
  sent text
- **AND** no problem is reported, so an override written against a later version of this
  plugin degrades to literal text rather than to a refusal

#### Scenario: An override with no placeholder is sent as written

- **WHEN** an override reads `follow AGENTS.md`
- **THEN** the sent text is exactly `follow AGENTS.md`
- **AND** it is one argument-vector element, so an override containing spaces needs no
  quoting and is given none

### Requirement: With no resolved `openspec` binary there is no prompt, and `a`/`c`/`s` say so

In file mode — no `openspec` binary resolved by any of the probe's four steps — there is no
absolute path to name, and the measurement above shows the launched agent's own shell will
not resolve `openspec` either, so no prompt this capability can build would work.

`launch::prompt_text` SHALL therefore never be reached in file mode: `launch::decide` refuses
first, as `agent-launch` specifies. The composition root SHALL pass the resolved binary as
`Option<PathBuf>`, derived from the same `resolve::openspec_bin` result that decides
`Collaborators::file_mode`.

**The worker SHALL refuse a `Request::Launch` carrying `openspec_bin` `None`**, before
resolution and before any Herdr call, reporting `named` `None` and exactly one problem naming
the absent binary. `Collaborators::file_mode` is computed from the CLI handle
(`cli.is_none()`), **not** from `Settings`, so the two are separate values that a defect can
drive apart: a composition root that passed `None` for the binary while resolving a CLI would
leave `decide` seeing `file_mode` `false`, returning `Go`, and handing the worker a request it
has no path for. The refusal is what makes that state observable instead of a panic or a
prompt naming an empty path, and it is why the third plant in `agent-launch`'s wiring
requirement fails on a **missing `integration status` and `pane split`** rather than on the
prompt text alone.

`g` SHALL NOT be affected: focusing an agent that is already running sends no prompt and
needs no binary.

#### Scenario: File mode carries no path and builds no prompt

- **WHEN** `start_collaborators` runs with the configured path, `PATH`, nvm, and the
  `npm prefix -g` hook all unable to produce a usable binary
- **THEN** `Collaborators::file_mode` is `true`, and pressing `a` records a problem row naming
  the absent `openspec` binary while the invocation log stays empty of `pane split`
- **AND** a control run whose probe resolves a binary has `file_mode` `false` and completes the
  launch, so the two runs differ observably at the seam rather than by inspecting
  `Collaborators::launcher`, which is a `Box<dyn Launcher>` exposing only `request` and `drain`
  and cannot be asserted against directly

#### Scenario: `g` still works with no binary

- **WHEN** a pane in file mode has a selected change with a live attributed agent and `g` is
  pressed
- **THEN** `decide` returns `Decision::Go(Request::Focus { .. })`
- **AND** the worker issues `agent focus <pane id>` and builds no prompt at all
