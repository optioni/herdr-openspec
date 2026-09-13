## Why

`a`, `c`, and `s` launch an agent onto the selected change and send it `/opsx:apply`,
`/opsx:continue`, or `/opsx:archive` — **Claude Code slash commands** — and they do it under
`agent_kind`, which defaults to `claude` whether or not the user has ever used Claude Code.

Two things are wrong, and neither is the one this proposal originally claimed.

**The default is a guess we do not have to make.** `herdr integration status` reports which
agent integrations the user has actually installed. On the reference machine that is
`claude: current (v9)` and `codex: current (v8)`, with fifteen others absent. Defaulting to
`claude` for a Codex-only user launches the wrong client on a keypress, silently.

**The prompt is hardcoded while the client is configurable.** `config.toml` already carries
`agent_kind` (documented at `README.md:85`) and `launch::start_args` already passes it to
`herdr agent start --kind`, so `agent_kind = "codex"` launches Codex correctly today — and then
sends it `/opsx:apply`, which it does not understand.

*(Two corrections from earlier drafts: the configuration surface already exists, and the
first-run picker — dropped on the grounds that no unconfigured state could occur — is
reinstated, because "several integrations installed and none chosen" is exactly that state.)*

## What Changes

- **`agent_kind` resolves by precedence instead of a constant:**

  | | Source | Notes |
  |---|---|---|
  | 1 | `agent_kind` in `config.toml` | An explicit, hand-edited choice always wins. No picker, ever |
  | 2 | The recorded choice under `HERDR_PLUGIN_STATE_DIR` | What the picker wrote last time |
  | 3 | The single installed integration | Unambiguous — no need to ask |
  | 4 | The picker | Several installed, none chosen |
  | 5 | `claude` | Last resort only, when nothing is installed and nothing is configured |

- **A picker opens on `a`/`c`/`s`** when the resolution reaches step 4, listing the installed
  integrations, and records the choice so it is asked once. The key that opened it proceeds
  with the chosen kind.
- **`claude` stops being the default and becomes the last resort.** Step 5 exists because
  refusing to launch would fail closed, not because Claude Code is presumed.
- **The three actions resolve their prompt through the resolved kind**, with Claude Code's
  mapping built in and per-kind overrides in `config.toml`. An unmapped kind degrades to a
  problem row naming it — never a silent wrong prompt, never a blocked key.
- **Behaviour change without a config edit.** A user with no `agent_kind` set and an
  integration other than `claude` installed will now launch that one. That is the point, and it
  is worth calling out rather than discovering.

## Non-Goals

- **Probing `PATH`, or asking Herdr "can this be launched".** `herdr agent start` fails with its
  own reason on an unsupported kind and `launch` already carries it into a problem row.
- **Treating `integration status` as an availability check.** It reports whether Herdr's
  status-reporting hook is installed, *not* whether a CLI exists — a kind launches fine without
  its integration; what is lost is status. It is used here as evidence of **what the user has
  chosen to set up**, which is a different and defensible reading.
- **Writing the choice into `config.toml`.** The plugin's writes are scoped to its state
  directory; the configuration directory is the user's to hand-edit, and this change must not
  start writing into it.
- Teaching non-Claude clients the OpenSpec workflow.
- Per-change or per-repository selection.

## What Herdr's surfaces actually provide

`herdr integration status` lists 17 integrations as installed or not, naming each hook's path.
It has **no `--json` form** (only `--outdated-only`), so this is a plain-text parse — one line
per integration — living on the testable side of the CLI seam like every other. A failed or
unparseable read collapses to step 5 with a problem row, never a blocked launch.

Its second use is a warning in its own right: launching a kind whose integration is absent
means every agent this plugin starts reports `agent_status: unknown` forever, silently
degrading the list badges, `agent-attribution`, and `g`. Measured: the one live agent is a
`claude` one, integration `current (v9)`, reporting `working` rather than `unknown`.

`herdr agent start --kind`'s closed 23-kind enum is **not** used — it lives in `--help` text
rather than a machine-readable API, and scraping help output is more brittle than the failure
path already in place.

## Capabilities

### New Capabilities

- `agent-prompts`: action + kind → the text sent, its built-in default, and how an unmapped
  kind degrades.
- `integration-status`: parsing `herdr integration status`, and the missing-integration warning.
- `client-picker`: when the picker opens, what it renders, which keys it takes, what it records.

### Modified Capabilities

- `agent-launch`: the kind is resolved by precedence and the prompt by kind.
- `plugin-config`: per-kind prompt overrides; `agent_kind` becomes an override rather than a
  defaulted value.
- `plugin-state`: the recorded choice, beside `agent-names.toml`.
- `dashboard-loop`: a modal route changes what keys mean while open, as `/` filter mode does.

## Impact

- `src/launch.rs` — kind resolution and prompt resolution.
- `src/config.rs` — `agent_kind` loses its `claude` default; overrides added.
- `src/state.rs` — recording the choice.
- `src/ui/` — the picker view (pure), its route, and key handling.
- `README.md`, `SPEC.md` — the precedence table; `SPEC.md`'s degraded-states table gains rows,
  each bound in `tests/degraded-coverage.toml`.
- One new `HerdrCli` call from an existing consumer — no new spawn, no new seam file, the
  `ALLOWED` list unchanged. No new dependency.

## Open Questions for Review

1. **Is the precedence table right at step 3?** Auto-selecting a single installed integration
   is convenient and also the one step that acts without asking.
2. **Does the picker block the keypress or launch afterwards?** Choosing and then launching in
   one gesture is nicer and makes the modal harder to specify.
3. **Should step 5 warn loudly?** Reaching `claude` as a last resort with nothing installed is
   exactly the case that used to be silent.
4. **Is `integration status` read at startup or lazily on first `a`/`c`/`s`?** Lazy costs
   nothing on panes that never launch anything.
