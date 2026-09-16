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
`agent_kind` (documented at `README.md:88`) and `launch::start_args` already passes it to
`herdr agent start --kind`, so `agent_kind = "codex"` launches Codex correctly today — and then
sends it `/opsx:apply`, which it does not understand.

*(Corrected from earlier drafts: the configuration surface already exists, so this change
builds none. The interactive picker moved to `settings-window`, which absorbs it into one
overlay — this change stops at a problem row.)*

## What Changes

- **`agent_kind` resolves by precedence instead of a constant:**

  | | Source | Notes |
  |---|---|---|
  | 1 | `agent_kind` in `config.toml` | An explicit, hand-edited choice always wins. No picker, ever |
  | 2 | The recorded choice under `HERDR_PLUGIN_STATE_DIR` | What `settings-window` writes, once it lands |
  | 3 | The single installed integration | Unambiguous — no need to ask |
  | 4 | A problem row naming the installed integrations | Several installed, none chosen. Upgraded to an interactive picker by `settings-window` |
  | 5 | `claude` | Last resort only, when nothing is installed and nothing is configured |

- **Step 4 renders a problem row** naming the installed integrations and asking for an explicit
  `agent_kind`, rather than guessing between them. It does not block the other keys. The
  interactive picker that replaces this row is `settings-window`'s job — that change absorbs it
  into one overlay rather than this change building a second modal beside it.
- **`claude` stops being the default and becomes the last resort.** Step 5 exists because
  refusing to launch would fail closed, not because Claude Code is presumed.
- **The three actions resolve their prompt through the resolved kind — as two shapes, not
  twenty-three mappings:**

  **one** CLI-driven shape for every kind, `claude` included: a short instruction to run
  `openspec instructions apply --change <name>` and follow it. The `/opsx:apply` shortcut is
  **dropped** — it is one more branch and one more failure mode (it fails in any Claude Code
  without the `opsx` plugin installed), while the CLI shape works in all of them. Adding a
  client therefore requires **no mapping at all**. `config.toml` keeps per-kind overrides.
- **The prompt names the plugin's resolved absolute path to `openspec`, not the bare command.**
  Measured on the reference machine: a fresh interactive `zsh` with a reset `PATH` reports
  `openspec not found` even though `.zshrc` references nvm — it is lazy-loaded. The plugin's
  four-step probe (config, `PATH`, nvm, `npm prefix -g`) is *more* thorough than a shell lookup,
  so a bare `openspec` in the prompt would fail for an agent even when the plugin found one.
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

## Why the prompts are not per-agent

`openspec instructions apply --change <name> --json` returns the apply instructions and the
context-file list for a change — measured working against `openspec` 1.13.0. `continue` and
`archive` have equivalent CLI paths (`openspec status --change <name> --json` names the next
ready artifact; `openspec archive <name>` is mechanical). So the workflow is reachable from a
shell, and an agent that can run a shell command can follow it.

That makes "adapt the commands per agent" the wrong axis. `/opsx:apply` is not a Claude Code
*dialect* of a universal idea — it is a **shortcut** that happens to exist because the `opsx`
plugin is installed in that client. No other client has an equivalent to translate to, and
inventing one per kind would mean guessing at command grammars that do not exist. What
generalises is the CLI underneath.

The generic shape also leans on something already true: an agent started in this repository
reads `AGENTS.md`, which documents the OpenSpec workflow, so the prompt can be short rather
than carrying the whole procedure.

**`a`/`c`/`s` are hidden in file mode.** With no resolved binary there is no path to name, and
the measurement above shows the agent's own shell will not resolve `openspec` either — so the
prompt could not work. The `a/c/s launch` footer hint is dropped, the keys go inert on the
established "inert when nothing applies" terms, and pressing one records a problem row naming
the reason rather than failing silently. The header already badges `file mode`, so the context
is on screen.

**`g` is not hidden.** It focuses an agent that is already running, which needs no `openspec`
binary at all.

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

The interactive picker is **not** here — it belongs to `settings-window`, which subsumes it.

### Modified Capabilities

- `agent-launch`: the kind is resolved by precedence and the prompt by kind.
- `plugin-config`: per-kind prompt overrides; `agent_kind` becomes an override rather than a
  defaulted value.
- `plugin-state`: the recorded choice is **read**, beside `agent-names.toml`; nothing here
  writes it.
- `dashboard-loop`: `ui::app::Launch::problems`' stated bound rises from two entries to four.
- `responsive-layout`: the footer's two action hints stop sharing one condition — `a/c/s
  launch` needs a resolved binary, `g focus` does not.

An earlier draft claimed `dashboard-loop` needed no delta, on the grounds that step 4 is a
problem row rather than a mode, `Dashboard` gains no field (`file_mode` already exists), and
`action_for` binds no new key. All three are true and none of them was the question: the
capability's prose **pins the `problems` bound**, and `degraded-states` carried a
`dashboard-loop` delta when it made the same move from one entry to two. `responsive-layout`
was missed the same way — it states the footer rule normatively, and every earlier change that
touched those hints carried a delta for it. The lesson, recorded here because it generalises:
ask what another capability's **prose** already fixes, not only whether this change moves its
code.

## Impact

- `src/integration.rs` — **new**: parsing `herdr integration status` and the kind precedence,
  pure and outside `src/ui/`. Moves `src/lib.rs`'s `pub mod` set from 14 to 15, and with it
  `SPEC.md`'s module map and tested-modules list.
- `src/launch.rs` — prompt resolution, `Settings`, and the worker's once-per-session kind
  resolution.
- `src/config.rs` — `agent_kind` loses its `claude` default; prompt overrides added.
- `src/state.rs` — **reading** the recorded choice. This change never writes it;
  `settings-window` does.
- `README.md`, `SPEC.md` — the precedence table; `SPEC.md`'s degraded-states table gains rows,
  each bound in `tests/degraded-coverage.toml`.
- One new `HerdrCli` call from an existing consumer — no new spawn, no new seam file, the
  `ALLOWED` list unchanged. No new dependency.

## Questions resolved before the specs were written

1. **Is the precedence table right at step 3?** **Yes — step 3 is kept.** Exactly one
   installed integration is not a guess between candidates; it is the only candidate, and it
   is what makes this change useful to a Codex-only reader who has never opened `config.toml`.
2. **Is a problem row enough on its own?** **Yes, for this change.** The interactive picker is
   `settings-window`'s, which absorbs it into one overlay rather than this change building a
   second modal beside it. Step 4 stops at a row.
3. **Should step 5 warn loudly?** **Yes — it records a problem row.** The launch still goes
   ahead with `claude`, because refusing with no evidence at all would fail closed, but the
   one genuinely blind guess stops being silent.
4. **Is `integration status` read at startup or lazily?** **Lazily**, on the first `a`/`c`/`s`
   press, inside the launcher's own worker thread — which is already allowed to block, unlike
   the render path. A pane that never launches anything issues no `integration status` call,
   startup grows no subprocess, and the answer is cached for the session.
