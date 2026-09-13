## Why

`a`, `c`, and `s` launch an agent onto the selected change — and they launch **Claude Code**,
because what they actually send is `/opsx:apply`, `/opsx:continue`, and `/opsx:archive`. Those
are Claude Code slash commands. A user running Codex, GitHub Copilot CLI, or anything else gets
a session that receives a string its client does not understand.

So "let the user pick their client" is not a setting with three values. The binding from an
**action** (apply / continue / archive) to the **text sent to a client** is the thing that
varies, and today it is hardcoded to one client's command grammar. Picking a client without
picking that mapping just moves the breakage.

Unplanned work past Phase 6, and the largest of the ideas currently queued.

## What Changes

- **A client is a named entry in configuration**, carrying what Herdr should start and how each
  of the three actions is phrased for it. Claude Code ships as the built-in default, so an
  existing install behaves exactly as it does today with no configuration at all.
- **The three keys resolve their prompt through the selected client** rather than through a
  hardcoded `/opsx:*` literal.
- **A first-run prompt.** Pressing `a`/`c`/`s` with no client configured opens a picker instead
  of launching, and records the choice. Pressing it again launches.
- An unknown, malformed, or removed client entry degrades to a problem row and the built-in
  default — it never blocks the launch key and never replaces the dashboard with an error.
- **BREAKING** if the plugin config format gains a required key. The intent is that it does not:
  every new key is optional, and absence means "Claude Code, as today".

## Non-Goals

- Detecting installed clients on the machine. Probing `PATH` for four binaries is a different
  change with its own failure surface; configuration is explicit here.
- Per-change or per-repository client selection. One choice per plugin install.
- Teaching non-Claude clients the OpenSpec workflow. If a client has no equivalent of
  `/opsx:apply`, this change lets the user write what to send; it does not make the workflow
  work there.
- Editing Herdr's own agent integrations, or anything under `herdr integration`.
- Launching more than one agent per change, which `agent-launch` already refuses.
- A general settings surface. This is one setting, not a preferences system — resist the pull.

## Capabilities

### New Capabilities

- `agent-client`: the client registry — what a client entry is, how the three actions map to
  prompts, what the built-in default is, and how a bad entry degrades.
- `client-picker`: the first-run modal — when it opens, what it renders, which keys it takes,
  and what it records.

### Modified Capabilities

- `agent-launch`: the prompt is resolved from the selected client rather than hardcoded.
- `plugin-config`: the new optional configuration surface.
- `plugin-state`: where the selection is recorded — under `HERDR_PLUGIN_STATE_DIR`, beside
  `agent-names.toml`, never in the repository and never in the config directory the user
  hand-edits.
- `dashboard-loop`: a modal route changes what keys mean while it is open, the way `/` filter
  mode already does.

## Impact

- `src/launch.rs` — prompt resolution; it stays outside `src/ui/` and keeps its worker thread.
- `src/config.rs` — the client entries.
- `src/state.rs` — recording the selection.
- `src/ui/` — a new modal view, pure, plus the route and key handling in `app`/`driver`.
- `SPEC.md` — the degraded-states table gains rows, each bound in `tests/degraded-coverage.toml`.
- No new dependency expected. No new process spawn: the launcher already reaches `herdr` through
  `HerdrCli`, and a different prompt string is not a new seam.

## Open Questions for Review

1. **Where the choice lives.** State dir (per-install, written by the plugin) or config dir
   (hand-editable, user-owned)? The prompts are something a user will want to edit by hand,
   which argues config; the *selection* is something the plugin writes, which argues state.
   Probably both, split — worth settling before design.
2. **Whether the picker is a modal at all.** A modal is a new route and new key semantics. The
   cheaper shape is a problem row saying "no client configured; edit config.toml" — worse
   ergonomics, far less machinery. This is the main thing to iterate on.
3. **Which clients ship as built-ins.** Claude Code certainly. Codex and Copilot CLI only if
   their prompt grammar can be written down accurately — a wrong built-in is worse than none,
   because the user will trust it.
