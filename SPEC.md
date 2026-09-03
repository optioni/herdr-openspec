# herdr-openspec — Design Specification

**Status:** Draft
**Date:** 2026-09-03
**Requirements:** see `PRD.md`

## Overview

`herdr-openspec` is a Herdr plugin providing a read-only dashboard for OpenSpec
state, plus the ability to launch an agent onto a change. It ships a single Rust
binary invoked two ways by the manifest: as a pane process rendering the TUI, and
as an action process that opens or focuses that pane.

**Stack:** Rust, `ratatui` + `crossterm` (TUI), `notify` (filesystem watching),
`serde_json`, `serde_yaml`, `pulldown-cmark` (markdown). Exact versions are pinned
to current stable releases at implementation time, not from memory.

## Architecture

Two boundaries define the design. Everything interesting lives between them.

**The subprocess seam.** All process spawning sits behind two traits:

```rust
trait OpenspecCli { fn run(&self, args: &[&str]) -> Result<String>; }
trait HerdrCli    { fn run(&self, args: &[&str]) -> Result<String>; }
```

Each has exactly one real implementation that spawns a process and returns stdout,
and a fake used by tests. No parsing, merging, or decision-making happens inside
either. This is what makes the coverage target reachable: the untestable residue
is two thin wrappers and `main`.

**The render seam.** Views are pure functions from a `Dashboard` state value to a
ratatui frame. They perform no I/O, so they are tested by rendering into a
`TestBackend` buffer at fixed widths.

**Module map:**

| Module | Responsibility |
|---|---|
| `resolve` | Locate the repository root and the `openspec` binary |
| `schema` | Parse `schema.yaml` into an ordered artifact list; identify the tasks artifact |
| `changes` | Build `Change` values from files and from CLI JSON |
| `tasks` | Parse markdown checkboxes into groups, items, and counts |
| `agents` | Attribute live Herdr agents to changes |
| `launch` | Split a pane, start an agent, send the `/opsx:*` prompt |
| `watch` | Filesystem watching and debounce |
| `ui` | Views, layout, key handling |
| `cli` | The two subprocess traits and their real implementations |

## Data layer

### Dual-source model

The two sources differ in latency and authority, and the design uses both rather
than treating one as a fallback for the other:

- **Files render immediately.** Walking `openspec/changes/` and parsing checkboxes
  costs well under a millisecond. The pane paints on open.
- **The CLI arrives and corrects.** `openspec` is a Node binary costing 200–400ms
  per invocation. It runs on a worker thread; when its JSON lands, the view is
  upgraded to schema-correct state.

The consequence is that an absent CLI degrades correctness slightly rather than
breaking the pane, and the file path is exercised on every launch rather than
being untested fallback code.

### Resolution chain

**Repository.** Start from the invocation context's workspace working directory
and walk up looking for `openspec/`. If none is found, render an empty state
naming the directory searched.

**Schema.** Read `openspec/config.yaml` for `schema:`. Load
`openspec/schemas/<name>/schema.yaml` when vendored — graft places it there — and
otherwise ask the CLI via `openspec schema`. The `artifacts` list becomes the tab
order. The tasks artifact is the one whose entry carries `role: tasks`; the id
`tasks` is conventional and must not be assumed.

**Changes.** `openspec list --json` yields
`{name, completedTasks, totalTasks, lastModified, status}` per change. The file
path enumerates subdirectories of `openspec/changes/` excluding `archive/`.

**Archived changes.** Directories under `openspec/changes/archive/` named
`YYYY-MM-DD-<name>`. Strip the date prefix, sort descending, show the five most
recent (`archived_count` in plugin configuration).

**Artifact files.** `openspec instructions apply --change <name> --json` returns
`contextFiles`, mapping artifact id to concrete paths — preferable to guessing
filenames. The file path falls back to `<id>.md` for file artifacts and `<id>/`
for directory artifacts.

**The `openspec` binary.** Probed in order, and cached for the session:

1. `openspec_bin` from plugin configuration — `config.toml` in the directory
   reported by `herdr plugin config-dir herdr-openspec`, which also holds
   `agent_kind`, `archived_count`, and the truncated-name mapping
2. `openspec` on `PATH`
3. `~/.nvm/versions/node/*/bin/openspec`
4. `$(npm prefix -g)/bin/openspec`

Steps 3 and 4 exist because the binary is commonly installed under a Node version
manager, and a plugin pane command does not run through a login shell.

### Refresh

One `notify` watcher on `openspec/`, debounced at approximately 150ms, invalidating
only the changes whose paths were touched. A manual refresh key forces a full
re-read including CLI calls.

## User interface

### Responsive layout

A Herdr split pane is frequently 40–60 columns, where a fixed two-column layout is
unusable.

- **100 columns or wider:** two columns — change list left, artifact detail right.
- **Narrower than 100 columns:** single column. The list is the root view; `Enter`
  opens detail and `Esc` returns.

### List view

One row per active change, then a separator, then the five most recent archived
changes:

```
> add-token-refresh    [4/9]  > claude - working
  fix-empty-basket     [7/7]  done
  migrate-ai-sdk-v7    [-]    schema unknown
  -- archived --------------------------------
  2026-08-14 add-auth
```

Progress comes from the CLI when available and from checkbox counts otherwise. A
footer reports agents in the repository that could not be attributed to a change.

### Detail view

A header carrying change name, schema, and progress; a tab bar built from the
schema's artifact list; content below.

The **tasks tab** renders task groups under their headings, a checkbox glyph per
item, and a progress bar. Every other tab is a markdown viewer built on
`pulldown-cmark`, supporting headings, lists, code blocks, emphasis, and links.

Tasks are **read-only by design**. Writing a checkbox from the pane would race the
agent editing `tasks.md` in another pane.

### Keys

| Key | Action |
|---|---|
| `j` / `k`, arrows | Navigate |
| `Enter` | Open change detail |
| `Esc` | Back to list |
| `1`–`9`, `[`, `]` | Switch artifact tab |
| `/` | Filter changes |
| `r` | Force refresh |
| `a` | Launch an agent with `/opsx:apply` |
| `c` | Launch an agent with `/opsx:continue` |
| `s` | Launch an agent with `/opsx:archive` |
| `g` | Focus the running agent for this change |
| `q` | Quit |

Action keys are hidden when the Herdr socket is unreachable.

## Herdr integration

### Agent status by polling

`herdr agent list` is polled at roughly one-second intervals. It returns, per
agent: `agent`, `agent_status`, `cwd`, `pane_id`, `tab_id`, `workspace_id`, and
`terminal_title`.

Polling is chosen over an event hook deliberately. The call is a Unix-socket
round trip costing milliseconds — unlike the Node CLI — and needs no manifest
hook. The confirmed plugin event names are `pane.created`, `pane.closed`,
`pane.exited`, `pane.focused`, the `tab.*` and `workspace.*` families, and
`worktree.created` / `worktree.opened`; agent-status events appear in the binary
but are not confirmed as valid hook targets. An event hook remains available later
as a pure optimisation.

### Attributing an agent to a change

Three tiers, and the design refuses to guess beyond them:

1. **Launched by the plugin.** `herdr agent start` takes a name positionally, and
   change names are already kebab-case, so the agent is named after the change and
   `agent list` yields the association directly. Names are capped at 32 characters
   (`[a-z][a-z0-9_-]{0,31}`); longer change names are truncated and the mapping is
   recorded in plugin-local state.
2. **Named manually.** Any live agent whose name equals a change name is
   attributed, making `herdr agent rename` a deliberate way to opt in.
3. **Everything else.** Agents whose `cwd` is inside the repository but which carry
   no change name are *not* attributed to any row. They are reported as a count in
   the footer.

### Launch flow

```
herdr pane split --cwd <repo> --direction right --no-focus   -> pane_id
herdr agent start <change> --kind <kind> --pane <pane_id>
herdr agent prompt <change> "/opsx:apply <change>"
```

`<kind>` comes from plugin configuration and defaults to `claude`; Herdr supports
more than twenty agent kinds. `g` focuses an existing agent via `herdr agent focus`.

### Manifest

```toml
id = "herdr-openspec"
name = "OpenSpec"
version = "0.1.0"
min_herdr_version = "0.7.0"
platforms = ["macos", "linux"]

[[build]]
command = ["/bin/sh", "scripts/build.sh"]

[[actions]]
id = "open"
title = "OpenSpec: dashboard"
contexts = ["workspace"]
command = ["./target/release/herdr-openspec", "open"]

[[actions]]
id = "open-tab"
title = "OpenSpec: dashboard (tab)"
contexts = ["workspace"]
command = ["./target/release/herdr-openspec", "open", "--tab"]

[[panes]]
id = "dashboard"
title = "OpenSpec"
placement = "split"
command = ["./target/release/herdr-openspec", "ui"]

[[panes]]
id = "dashboard-tab"
title = "OpenSpec"
placement = "tab"
command = ["./target/release/herdr-openspec", "ui"]
```

## Degraded states

Every condition renders usable content rather than an error screen:

| Condition | Behaviour |
|---|---|
| No `openspec/` found while walking up | Empty state naming the directory searched |
| `openspec` binary not found | File mode, with a dim `file mode` badge in the header |
| Schema unknown to the CLI | Per-change fall back to file mode. This is real: `learning-tool` declares schema `outside-in-tdd`, which the installed CLI rejects |
| No active changes | Empty state; archived changes remain browsable |
| Artifact file missing | Tab is still shown and renders "No content yet" |
| Herdr socket unreachable | Runs as a standalone TUI; agent column and action keys hidden |
| Pane narrower than 100 columns | Single-column list and detail |

## Testing and quality gates

### Unit-tested modules

Each is a pure transformation, tested without a TUI or a subprocess:

- `changes::from_files` and `changes::from_cli` — both produce the same `Change`
  type, from fixture trees and fixture JSON respectively
- `schema::artifacts` — `schema.yaml` to ordered tabs, including `role: tasks`
  detection
- `tasks::parse` — markdown checkboxes to grouped items and counts
- `agents::attribute` — agent-list JSON plus change list to per-change badges,
  covering all three tiers including the deliberate non-attribution case
- `resolve::openspec_bin` — the four-step probe chain against a synthetic filesystem

### View tests

Views render into a ratatui `TestBackend` and assert on the resulting buffer, at
both 60 and 120 columns so the responsive breakpoint is genuinely covered.

### Fixtures

Under `tests/fixtures/`: a `tdd`-schema repository with active and archived
changes; a repository with no `openspec/` directory; a repository declaring a
schema the CLI does not recognise.

### Gates

Enforced identically locally and in CI, behind a single `make check` target so the
two cannot diverge:

| Gate | Command |
|---|---|
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| Test | `cargo test --all-features` |
| Coverage | `cargo llvm-cov --fail-under-lines 80` |

`cargo-llvm-cov` is chosen over `tarpaulin`, which is Linux-first and unreliable on
Apple Silicon. CI runs on `ubuntu-latest` and `macos-latest` with `Swatinem/rust-cache`;
coverage runs once, on Linux.

Two one-time setup steps are required: `rustup component add clippy` and
`cargo install cargo-llvm-cov`.

## Build and distribution

Herdr installs a GitHub-managed plugin by cloning the repository at a resolved
commit and running the `[[build]]` step in place, after install confirmation and
before registering the plugin. There is no release process to satisfy:
`herdr plugin install <owner>/<repo>` is sufficient. `herdr plugin link .` — used
for local development — does **not** run build commands; the local author builds
the working tree themselves (`make build` or `scripts/build.sh`) before or after
linking.

The build step is `scripts/build.sh` rather than a bare `cargo build --release`,
because Herdr may be launched without `~/.cargo/bin` on `PATH` — a GUI or
login-less launch — in which case a direct cargo invocation fails even though Rust
is installed. The script sources `~/.cargo/env` when present, reports clearly if
cargo is still missing, and then builds.

Before publishing, that script gains a fast path: download a version-matched
prebuilt binary from GitHub Releases, verify its SHA-256, and fall back to
compiling on any miss. That removes the Rust toolchain from the install
requirements. Registry listing at `https://assets.herdr.dev/plugins/index.json` is
a separate submission, independent of releases.
