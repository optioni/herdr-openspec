## Why

`a`, `c`, and `s` launch an agent onto the selected change and send it `/opsx:apply`,
`/opsx:continue`, or `/opsx:archive`. Those are **Claude Code slash commands**.

Choosing the client is **already possible and already shipped**: `config.toml` carries
`agent_kind` (default `claude`, documented at `README.md:85`), and `launch::start_args` passes
it straight to `herdr agent start --kind`. Setting `agent_kind = "codex"` today launches Codex
correctly — and then sends it `/opsx:apply`, which it does not understand.

So the gap is not client selection. It is that **the prompt is hardcoded to one client's command
grammar** while the client it is sent to is configurable. The two halves disagree, and the
half that is missing is the smaller one.

*(Corrected from this proposal's first draft, which proposed building the configuration surface
that already exists.)*

## What Changes

- **The three actions resolve their prompt through the configured `agent_kind`**, instead of a
  hardcoded `/opsx:*` literal. Claude Code's mapping is the built-in default, so an existing
  install behaves exactly as it does today.
- **A kind with no known mapping is configurable, not fatal.** The user writes what to send;
  absent that, the action degrades to a problem row naming the kind, never a silent wrong
  prompt and never a blocked key.
- **`herdr integration status` is read once at startup** to warn when the configured
  `agent_kind`'s integration is missing — because agents launched under it will then report
  `agent_status: unknown` forever, silently degrading the badges, attribution, and `g`.
- Not **BREAKING**: every new key is optional and absence means "as today".

## Non-Goals

- **Re-inventing `agent_kind`.** It exists, it works, it is documented. This change reads it.
- **Probing for available clients.** Neither `PATH` nor Herdr is asked "can this be launched" —
  `herdr agent start`'s own failure answers that. `integration status` is read for a narrower,
  accurate purpose; see below.
- **A first-run picker.** There is no unconfigured state to catch: `agent_kind` defaults to
  `claude` and works. A picker would interrupt a flow that is already correct.
- Teaching non-Claude clients the OpenSpec workflow. This lets the user say what to send; it
  does not make `/opsx:apply` exist elsewhere.
- Per-change or per-repository selection.

## What Herdr's own surfaces are good for

Both were measured against 0.9.0, and they answer different questions.

`herdr agent start --kind` accepts a closed enum of **23** kinds (`pi, claude, codex, gemini,
cursor, devin, agy, cline, omp, mastracode, opencode, copilot, kimi, kiro, droid, amp, grok,
hermes, kilo, qodercli, qwen, maki, muse`) — but that list lives in `--help` **text**, not a
machine-readable API. Scraping help output is more brittle than not validating, and no
validation is needed: `herdr agent start` already fails with its own reason on an unsupported
kind, and `launch` already carries that reason verbatim into a problem row.

`herdr integration status` **is** worth reading, for the question it actually answers. It lists
17 integrations as installed or not, naming each hook's path (`~/.copilot/hooks/herdr-agent-state.sh`
and similar). What it reports is whether **Herdr's status-reporting hook** is installed — *not*
whether that agent's CLI exists or can be launched. A kind launches fine without its
integration; what you lose is status.

That distinction is the whole value here, because this dashboard **depends on agent status**:
the list region's agent badges, `agent-attribution`, and the `g` key all read what
`herdr agent list` reports. Measured on the reference machine: `claude: current (v9)` and
`codex: current (v8)`, everything else not installed — and the one live agent, a `claude` one,
reports `agent_status: working` rather than `unknown`.

So it is used as a **warning**, never as a capability check or an availability filter:
configuring an `agent_kind` whose integration is not installed means agents this plugin
launches will report `unknown` forever, and the pane should say so rather than let the user
discover it as a permanently blank badge. It is also a fair signal of which clients the user
actually works with — someone who ran `herdr integration install codex` uses Codex — which is
why it informs the warning rather than being ignored.

It has **no `--json` form** (only `--outdated-only`), so this is a plain-text parse: one line
per integration, `name: state`, with a path in parentheses. That parse lives on the testable
side of the CLI seam like every other, and a failed or unparseable `integration status` means
**no warning at all** — never a blocked launch.

## Capabilities

### New Capabilities

- `agent-prompts`: the mapping from an action (apply / continue / archive) and an `agent_kind`
  to the text sent, its built-in default, and how an unmapped kind degrades.
- `integration-status`: parsing `herdr integration status`, and warning when the configured
  kind's integration is absent.

### Modified Capabilities

- `agent-launch`: the prompt is resolved rather than hardcoded.
- `plugin-config`: the optional per-kind prompt overrides.

## Impact

- `src/launch.rs` — prompt resolution. It already receives `kind` as a parameter
  (`src/launch.rs:259`), so the value is in hand; only the prompt is hardcoded.
- `src/config.rs` — the optional overrides beside the existing `agent_kind`.
- `README.md`, `SPEC.md` — the `agent_kind` row gains the prompt half.
- `SPEC.md`'s degraded-states table gains a row, bound in `tests/degraded-coverage.toml`.
- One new `HerdrCli` call (`integration status`) from an existing consumer — no new spawn, no
  new seam file, and the `ALLOWED` list does not change.
- No new dependency.

## Open Questions for Review

1. **Which kinds ship with a built-in mapping?** Claude Code certainly. A wrong built-in is
   worse than none, because the user will trust it — so probably only kinds whose command
   grammar can be written down from documentation rather than guessed.
2. **Config shape.** A table keyed by kind (`[prompts.codex]`) or three flat keys overridden
   per kind? The first is tidier; the second is easier to explain in `README.md`.
3. **Is a prompt-less kind a problem row or a config error at load?** The load path already
   accumulates config problems, so either fits — but they surface in different places.
4. **Where does the integration warning go, and is it worth a startup call?** It is a
   `refresh.startup` entry by nature, beside the mouse-capture problem. Whether it is worth one
   more Herdr call on every pane open is the question — it could equally be read lazily, the
   first time `a`/`c`/`s` is pressed.
