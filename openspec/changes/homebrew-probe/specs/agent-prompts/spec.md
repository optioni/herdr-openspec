## MODIFIED Requirements

### Requirement: `/opsx:*` is dropped, and the prompt names the resolved absolute path

The three `/opsx:apply`, `/opsx:continue`, and `/opsx:archive` prompt texts SHALL be removed.
They are one more branch and one more failure mode — they fail in any Claude Code that does
not have the `opsx` plugin installed — while the CLI shape works in all of them.

The prompt SHALL name the plugin's own **resolved absolute path** to `openspec`, not the bare
command `openspec`. Measured on the reference machine, a fresh interactive `zsh` with a reset
`PATH` reports `openspec not found` even though `.zshrc` references nvm, because nvm is
lazy-loaded — and, for a Homebrew install, because `brew shellenv` runs only from
`.zprofile`, which an interactive non-login shell does not read; the plugin's five-step probe — configuration, `PATH`, nvm, `npm prefix -g`, the
Homebrew prefixes — is
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

### Requirement: With no resolved `openspec` binary there is no prompt, and `a`/`c`/`s` say so

In file mode — no `openspec` binary resolved by any of the probe's five steps — there is no
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

- **WHEN** `start_collaborators` runs with the configured path, `PATH`, nvm, the
  `npm prefix -g` hook, and the Homebrew prefix list all unable to produce a usable binary
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
