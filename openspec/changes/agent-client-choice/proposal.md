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
- Not **BREAKING**: every new key is optional and absence means "as today".

## Non-Goals

- **Re-inventing `agent_kind`.** It exists, it works, it is documented. This change reads it.
- **Probing for available clients — including through Herdr.** Considered and rejected on
  measurement, see below.
- **A first-run picker.** There is no unconfigured state to catch: `agent_kind` defaults to
  `claude` and works. A picker would interrupt a flow that is already correct.
- Teaching non-Claude clients the OpenSpec workflow. This lets the user say what to send; it
  does not make `/opsx:apply` exist elsewhere.
- Per-change or per-repository selection.

## Why not ask Herdr which agents are available

Herdr does expose two surfaces, and both were measured against 0.9.0:

- `herdr agent start --kind` accepts a closed enum of **23** kinds (`pi, claude, codex, gemini,
  cursor, devin, agy, cline, omp, mastracode, opencode, copilot, kimi, kiro, droid, amp, grok,
  hermes, kilo, qodercli, qwen, maki, muse`). That list lives in `--help` **text**, not in a
  machine-readable API. Scraping help output is more brittle than not validating at all.
- `herdr integration status` lists 17 integrations with an installed/not-installed state. Two
  problems. It prints **plain text, not JSON** — unlike every payload this crate parses. And,
  decisively, it reports whether *Herdr's own status-reporting hook* is installed (the paths it
  names are `~/.copilot/hooks/herdr-agent-state.sh` and similar), **not** whether the agent's
  CLI exists or can be launched. On the reference machine `claude: current (v9)` and `codex:
  current (v8)`, everything else "not installed" — which says nothing about what is runnable.
  Using it as an availability signal would be wrong in both directions.

No probe is needed anyway: `herdr agent start` already fails with its own reason when a kind is
unsupported, and `launch` already carries that reason verbatim into a problem row. The
never-fail-closed path is the validation.

## Capabilities

### New Capabilities

- `agent-prompts`: the mapping from an action (apply / continue / archive) and an `agent_kind`
  to the text sent, its built-in default, and how an unmapped kind degrades.

### Modified Capabilities

- `agent-launch`: the prompt is resolved rather than hardcoded.
- `plugin-config`: the optional per-kind prompt overrides.

## Impact

- `src/launch.rs` — prompt resolution. It already receives `kind` as a parameter
  (`src/launch.rs:259`), so the value is in hand; only the prompt is hardcoded.
- `src/config.rs` — the optional overrides beside the existing `agent_kind`.
- `README.md`, `SPEC.md` — the `agent_kind` row gains the prompt half.
- `SPEC.md`'s degraded-states table gains a row, bound in `tests/degraded-coverage.toml`.
- No new dependency, no new spawn, no new seam: `launch` already reaches `herdr` through
  `HerdrCli`, and a different prompt string is not a new collaborator.

## Open Questions for Review

1. **Which kinds ship with a built-in mapping?** Claude Code certainly. A wrong built-in is
   worse than none, because the user will trust it — so probably only kinds whose command
   grammar can be written down from documentation rather than guessed.
2. **Config shape.** A table keyed by kind (`[prompts.codex]`) or three flat keys overridden
   per kind? The first is tidier; the second is easier to explain in `README.md`.
3. **Is a prompt-less kind a problem row or a config error at load?** The load path already
   accumulates config problems, so either fits — but they surface in different places.
