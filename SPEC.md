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
`serde_json`, `serde_yaml`, `pulldown-cmark` (markdown), `toml` (plugin configuration
and state, both TOML). Exact versions are pinned to current stable releases at
implementation time, not from memory.

## Architecture

Two boundaries define the design. Everything interesting lives between them.

**The subprocess seam.** `cli` is the only module in this crate permitted to
spawn a process. Two traits carry the two programs it wraps directly:

```rust
trait OpenspecCli { fn run(&self, args: &[&str]) -> Result<String>; }
trait HerdrCli    { fn run(&self, args: &[&str]) -> Result<String>; }
```

Each has exactly one real implementation that spawns a process and returns stdout,
and a fake used by tests. No parsing, merging, or decision-making happens inside
either. A third spawn lives behind the same seam: the one-shot `npm prefix -g`
probe that `resolve::openspec_bin`'s fourth step needs. It is neither `openspec`
nor `herdr`, so it is not one of the two traits above — but it is still a process
spawn, and `cli` is still where it lives. `resolve` itself never spawns it: the
probe arrives as an injected `&dyn Fn() -> Option<PathBuf>`, the same shape by
which `resolve` and `config` take the process environment as a lookup closure,
so `resolve` stays pure and the seam still exists before anything crosses it.
This is what makes the coverage target reachable: the untestable residue is two
thin wrappers, the `npm prefix -g` binding, and `main`. (`config::env_lookup` is
a fourth one-line binding to the real world — the crate's single call to
`std::env::var` — but it is not untestable residue: it carries its own
assertions, comparing its result against `std::env::var` directly for a
variable known to be present and for one nothing sets, rather than being
covered only by the composition that calls it.)

**The render seam.** Views are pure functions from a `Dashboard` state value to a
ratatui frame. They perform no I/O, so they are tested by rendering into a
`TestBackend` buffer at fixed widths.

**Module map:**

| Module | Responsibility |
|---|---|
| `config` | Resolve the plugin's configuration directory from the process environment (no process spawn) and read `config.toml` into `openspec_bin`, `agent_kind`, `archived_count` |
| `state` | Resolve the plugin's state directory from the process environment; derive a Herdr-legal agent name from a change name; record and read back the mapping |
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
and walk up looking for `openspec/`, stopping at the innermost ancestor that
holds it — an `openspec/` several levels up is not preferred over one closer in.
A regular file named `openspec` does not count; only a directory (including one
reached through a symbolic link) does. The starting path is canonicalized
before the walk when the filesystem can resolve it, so a `..` component or a
symlinked working directory does not leak into the root the empty state
prints or a later change joins onto. If none is found, render an empty state
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

**The `openspec` binary.** Probed in order, and cached for the session, taking
the first usable candidate and probing no further:

1. `openspec_bin` from plugin configuration — `config.toml` in the directory
   Herdr injects as `HERDR_PLUGIN_CONFIG_DIR` into every plugin process it
   starts, pane or action alike, no subprocess required. That directory is
   the same one `herdr plugin config-dir herdr-openspec` reports for a human,
   and is also the fallback path the plugin computes for itself when run
   outside a Herdr-started process. `config.toml` also holds `agent_kind` and
   `archived_count`. The agent-name mapping lives separately, under
   `HERDR_PLUGIN_STATE_DIR` — see Herdr integration → Attributing an agent —
   because the plugin writes it and must not write into the directory the
   user hand-edits
2. `openspec` on `PATH`
3. `<nvm root>/versions/node/<version>/bin/openspec`, where the nvm root is
   `NVM_DIR` when it is set to a non-blank value and `$HOME/.nvm` otherwise,
   and version directories are tried newest first, ordered numerically (so
   `v10.0.0` precedes `v9.99.99`) with an unparseable name kept and sorted
   after every parsed version
4. `$(npm prefix -g)/bin/openspec`

A step matches only a candidate that is, following symbolic links, an
executable regular file — a directory named `openspec` does not qualify, and
neither does a file with no execute bit. The winning path is returned exactly
as the chain constructed it, never canonicalized, since it is the name a
later change spawns and a symbolic link is the stable, upgrade-surviving
form. A configured `openspec_bin` that is not usable does not win and does
not end the chain: the remaining steps still run, and the fallback is
recorded as a problem naming the configured path, so a user's typo degrades
visibly rather than either silently substituting a different binary or
failing closed.

Steps 3 and 4 exist because the binary is commonly installed under a Node version
manager, and a plugin pane command does not run through a login shell. Step 4 is
unwired until `subprocess-seam` lands: `resolve::openspec_bin` takes the npm
prefix as an injected hook, and the binding `repo-resolution` ships always
returns nothing, so a system-node install with no nvm tree and no `PATH` entry
resolves nothing today and the dashboard runs in file mode until that change
replaces the binding. Whoever wires it must read `npm prefix -g`'s **stdout
only**, trimmed — on the reference machine `npm` writes unrelated shell-plugin
noise to stderr — and treat a non-zero exit or empty output as no prefix.

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

1. **Launched by the plugin.** `herdr agent start` takes a name positionally, but a
   change name is not always a legal Herdr agent name
   (`[a-z][a-z0-9_-]{0,31}`) — it may start with a digit, carry illegal
   characters, or simply run past 32 characters. The plugin derives an agent
   name from the change name by a pure, total function: ASCII-lowercase;
   replace every character outside `[a-z0-9_-]` with `-`; collapse runs of
   `-` and trim leading/trailing `-`/`_`; fall back to `change` if nothing is
   left; prefix `c-` if the result cannot legally start an agent name; and,
   past 32 characters, keep the first 27 characters (trimmed of any trailing
   separator) plus `-` and a four-digit lowercase base-36 suffix derived from
   an FNV-1a hash of the whole original change name, so the same change
   always derives the same agent name on every machine. A mapping from the
   derived agent name back to the change name is recorded in plugin-local
   state (under `HERDR_PLUGIN_STATE_DIR`, never beside `config.toml`)
   whenever the derived name **differs from the change name at all** — not
   only when it was truncated. `2fa-support` is only 13 characters but still
   becomes `c-2fa-support` and still needs the mapping to be attributable. A
   derived name already bound to a different change is rebound rather than
   rejected: the most recent launch is the live one. Launch flow's
   `herdr agent start <change>` and `herdr agent prompt <change>` below refer
   to this *derived* name, not the raw change name; correcting those two
   lines to say so explicitly is `agent-launch`'s work, planned from this
   paragraph.
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
| `config.toml` malformed, unreadable, or a key of the wrong type | The affected key falls back to its documented default while every other key that parsed correctly is still honoured; `Config::problems` names each fallback |
| `agent-names.toml` unusable (malformed, unreadable, or an entry Herdr would reject) | Empty or partial mapping; attribution falls back to the name-equality tier, and nothing already on disk is lost |
| A configured `openspec_bin` that does not name a usable binary | Falls through to the remaining probe steps rather than winning or ending the chain; the fallback is named in `BinResolution::problems` rather than being silent |

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
- `resolve::find_repo` and `resolve::openspec_bin` — the upward walk for
  `openspec/` and the four-step binary probe chain, both tested against a
  purpose-built scratch directory tree under `std::env::temp_dir()`, not a
  faked filesystem layer

### View tests

Views render into a ratatui `TestBackend` and assert on the resulting buffer, at
both 60 and 120 columns so the responsive breakpoint is genuinely covered.

### Fixtures

Under `tests/fixtures/`: a `tdd`-schema repository with active and archived
changes; a repository with no `openspec/` directory; a repository declaring a
schema the CLI does not recognise.

### Gates

Every gate command is written once, in the `Makefile`. Locally, `make check` runs all
four in order and stops at the first failure:

| Gate | Command |
|---|---|
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| Test | `cargo test --all-features` |
| Coverage | `cargo llvm-cov --fail-under-lines 80` |

`cargo-llvm-cov` is chosen over `tarpaulin`, which is Linux-first and unreliable on
Apple Silicon. CI invokes the same targets individually rather than the composite —
`make fmt-check`, `make lint`, and `make test` on both `ubuntu-latest` and
`macos-latest` with `Swatinem/rust-cache`, and `make coverage` once, on Linux.

Two one-time setup steps are required for local development — `rustup component add
clippy` and `cargo install cargo-llvm-cov` — since CI obtains `clippy` from the
toolchain action and `cargo-llvm-cov` from `taiki-e/install-action`.

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
