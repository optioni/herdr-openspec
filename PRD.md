# herdr-openspec — Product Requirements

**Status:** Draft
**Date:** 2026-09-03

## Problem

OpenSpec keeps the state of in-flight work on disk: a change's proposal, design,
specs, and task checklist. While an agent implements that change, the only way to
see where it stands is to stop, open the files, and read them — in the same
terminal the agent is working in, or in an editor outside it.

Herdr already runs several coding agents side by side and tracks which pane each
one occupies and whether it is working, idle, or blocked. What it cannot show is
*what* any of them is working on in OpenSpec terms: which change, which artifacts
exist, how many tasks remain.

The gap is a persistent, readable view of OpenSpec state that lives next to the
agents rather than competing with them for the terminal.

## Who it is for

The author, initially — one person running OpenSpec-driven work across many
repositories inside Herdr, frequently with more than one agent live at once.

Designed so that publishing to the Herdr plugin registry later is a matter of
documentation and prebuilt binaries, not a rewrite. Concretely, that means no
assumptions about a specific OpenSpec schema, agent kind, or repository layout
baked into the code; those are read from configuration or the OpenSpec CLI.

## Goals

1. **Read a change without leaving Herdr.** Open a pane, pick a change, and read
   its proposal, design, specs, and tasks with the same ease as reading a file.
2. **See progress at a glance.** Task completion per change, visible in the list
   without opening anything.
3. **Know which agent is on which change.** Where the association can be
   established honestly, show it — including when an agent is blocked and waiting.
4. **Start work on a change in one keystroke.** Launch an agent pane in the right
   directory with the right `/opsx:*` command already sent.
5. **Never fail closed.** A missing OpenSpec CLI, an unknown schema, or an
   unreachable Herdr socket degrades the view; it does not replace it with an error.

## Non-goals

- **Editing.** The pane does not write to OpenSpec files. Toggling a task checkbox
  would race the agent editing `tasks.md` in another pane.
- **Replacing `openspec-tui`.** That project remains a standalone TUI with its own
  embedded AI chat and PTY handling. This plugin is its read half, specialised for
  running inside Herdr, and the two evolve independently.
- **Orchestration.** Deciding what to implement next, in what order, across a
  phase, stays with the OpenSpec orchestrator agents. This plugin launches single
  changes on request; it does not drive a batch.
- **Authoring changes.** Creating proposals and artifacts stays in the agent, via
  the existing `/opsx:*` commands.
- **Windows support.** macOS and Linux only, matching the manifest's `platforms`.

## User stories

**Reading**

- As someone with a change in flight, I open the dashboard in a split pane and
  read the proposal and tasks while an agent works beside me.
- As someone returning to a repository after a break, I see the active changes and
  their task counts without running a command.
- As someone reviewing what shipped, I browse recently archived changes in the
  same view.

**Awareness**

- As someone running several agents, I see which change each named agent is on and
  its status, so a blocked agent is visible rather than discovered.
- As someone whose agents are not all attributable to changes, I see an honest
  count of unattributed agents in the repository rather than a wrong label on a row.

**Acting**

- As someone ready to start a change, I press one key and get a Claude pane in the
  repository root with `/opsx:apply <change>` already sent.
- As someone whose agent is already running, I press one key and jump to its pane.

**Degraded**

- As someone in a repository whose schema the CLI does not recognise, I still read
  the change's files, with the view marked as file mode.
- As someone who opened the dashboard outside Herdr, I still read changes, with
  agent features hidden.

## Success criteria

The product is working when, in normal use:

- The pane paints a usable change list immediately on open, before any Node
  subprocess has returned.
- A blocked agent is noticed from the dashboard rather than by cycling through panes.
- Reading a change's artifacts does not require leaving Herdr or opening an editor.
- Every degraded state in the specification renders content, not an error screen.

Enforced gates, failing the build when unmet:

- `cargo fmt --all -- --check` clean
- `cargo clippy --all-targets --all-features -- -D warnings` clean
- `cargo test --all-features` passing on macOS and Linux
- Line coverage at or above 80%

## Constraints and dependencies

- **Herdr** 0.7.0 or later, for the plugin manifest format and the `plugin`,
  `agent`, and `pane` CLI surfaces this relies on.
- **OpenSpec CLI** optional. When present it is authoritative; when absent the
  plugin reads `openspec/` directly. The CLI is a Node binary and is commonly
  installed under a version manager, so it will not always be on the `PATH` a
  plugin process inherits.
- **Rust toolchain** required to install today, since the build step compiles from
  source. Removed as a requirement when prebuilt release binaries are added.
- **Terminal width** as low as 40 columns in a split pane; the layout adapts rather
  than assuming a full screen.

## Risks

| Risk | Response |
|---|---|
| Agents cannot be attributed to changes in general — a terminal title is a summary, not a change id | Attribute only where the evidence is real (agent name, or launched by the plugin); report the rest as an unattributed count |
| OpenSpec CLI startup cost (200–400ms per call) makes the pane feel slow | Render from files immediately; treat CLI results as an asynchronous correction |
| Agent-status event hooks may not be valid plugin event targets | Poll `herdr agent list` over the socket instead; adopt hooks later only as an optimisation |
| Requiring a Rust toolchain suppresses adoption when published | Ship prebuilt binaries with a source fallback before listing in the registry |
| Duplicating `openspec-tui`'s parsing logic | Accepted: parsing is the cheap half, and the two tools have diverging runtime environments |

## Future

Deliberately deferred, in rough order of likely value:

1. Prebuilt release binaries and registry listing.
2. Agent-status event hooks replacing the poll.
3. Surfacing change progress onto the pane or tab itself via `herdr pane report-metadata`.
4. Attribution via worktree branch names, for agents the plugin did not launch.
