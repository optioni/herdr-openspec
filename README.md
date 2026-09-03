# herdr-openspec

A [Herdr](https://herdr.dev) plugin that puts a read-only OpenSpec dashboard in a
pane next to your agents.

Browse active and archived changes, read their artifacts, watch task progress, see
which agent is working on which change — and launch an agent onto a change with one
keystroke.

**Status: in development.** The design is settled (see [SPEC.md](SPEC.md) and
[PRD.md](PRD.md)); the implementation is being built change by change under
[`openspec/IMPLEMENTATION-ORDER.md`](openspec/IMPLEMENTATION-ORDER.md).

## Why

OpenSpec keeps the state of in-flight work on disk. Herdr already knows which agents
are running and whether they are working, idle, or blocked — but not *what* they are
working on in OpenSpec terms. This plugin closes that gap without taking the terminal
away from the agents.

## Install

```sh
herdr plugin install optioni/herdr-openspec
```

Herdr clones the repository and compiles it in place, so a Rust toolchain is
required for now. Prebuilt release binaries are planned.

Then open the dashboard from Herdr's action menu: **OpenSpec: dashboard** (split
pane) or **OpenSpec: dashboard (tab)**.

## Keys

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

## Configuration

Optional, in `config.toml` inside the directory reported by:

```sh
herdr plugin config-dir herdr-openspec
```

| Key | Default | Meaning |
|---|---|---|
| `openspec_bin` | auto-detected | Path to the `openspec` binary |
| `agent_kind` | `claude` | Herdr agent kind launched by `a` / `c` / `s` |
| `archived_count` | `5` | Archived changes listed below the separator |

The OpenSpec CLI is optional. When it is present the dashboard uses it as the
authority on schemas, artifacts, and progress; when it is absent, or when it
rejects the repository's schema, the dashboard reads `openspec/` directly and marks
itself as being in file mode.

## Development

Requires Rust, plus two one-time components:

```sh
rustup component add clippy
cargo install cargo-llvm-cov
```

Link the working tree into Herdr instead of installing from GitHub:

```sh
herdr plugin link .
```

Run every quality gate — format, lint, tests, and the 80% coverage floor:

```sh
make check
```

## Relationship to openspec-tui

`openspec-tui` is a standalone OpenSpec terminal UI with its own embedded AI chat
and PTY handling. This plugin is its read half, specialised for running inside
Herdr: it drops the PTY layer because Herdr owns agent panes natively, and gains
agent awareness that a standalone TUI cannot have. The two are independent.

## Licence

MIT
