# herdr-openspec

[![CI](https://github.com/optioni/herdr-openspec/actions/workflows/ci.yml/badge.svg)](https://github.com/optioni/herdr-openspec/actions/workflows/ci.yml)

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

Then open the dashboard from Herdr's action menu: **OpenSpec: dashboard** opens
it split from the current pane, and **OpenSpec: dashboard (tab)** opens it in a
new tab. Pressing either again focuses the existing dashboard rather than
opening a second one. The same pane is also reachable directly with
`herdr plugin pane open --plugin herdr-openspec --entrypoint dashboard`.

**An already-linked plugin must be relinked** (`herdr plugin link .`, or a
`plugin install` reinstall) or the Herdr server restarted for these two new
actions to appear — Herdr re-reads the manifest only then.

## Keys

| Key | Action |
|---|---|
| `j` / `k`, arrows | In the list, move the selection; in the detail content, scroll a line. Clamped at both ends rather than wrapping |
| `Enter` | Open change detail |
| `Esc` | Dismiss one layer: filter mode, then a leftover query, then detail. At the list root it does nothing |
| `1`–`9`, `[`, `]` | Switch artifact tab, from either the list or the detail. `0` is inert — tabs are 1-based |
| `/` | Filter changes |
| `r` | Force a full refresh — re-read every change from disk and re-ask the CLI about each one |
| `a` | Launch an agent with `/opsx:apply`. Inert — no call, no problem — with no change selected; refused with a reason (shown as a problem row) when the derived name is already running for this change |
| `c` | Launch an agent with `/opsx:continue`, on the same terms as `a` |
| `s` | Launch an agent with `/opsx:archive`, on the same terms as `a` |
| `g` | Focus the running agent for this change. Inert — no call, no problem — on a change with no attributed agent |
| `q` | Quit |
| `Ctrl-C` | Quit |

**While filtering, every printable key types into the query** — including `q`, `r`,
`j`, `k`, `a`, `c`, `s`, `g`, and the digits. `Backspace` deletes, `Enter` accepts,
`Esc` cancels, and the arrows still navigate. `Ctrl-C` always quits.

Only `q` and `Ctrl-C` close the pane; `Esc` never does.

Action keys, and the footer hints naming them (`a/c/s launch  g focus`), are
hidden when the Herdr socket is unreachable.

## Configuration

Optional, in `config.toml` inside the directory reported by:

```sh
herdr plugin config-dir herdr-openspec
```

Herdr supplies that same directory to the plugin itself as
`HERDR_PLUGIN_CONFIG_DIR`, so no subprocess is needed to find it while running.
Outside a Herdr-started process — for local testing, say — the plugin falls
back to `$HOME/.config/herdr/plugins/config/herdr-openspec`.

| Key | Default | Meaning |
|---|---|---|
| `openspec_bin` | auto-detected | Path to the `openspec` binary |
| `agent_kind` | `claude` | Herdr agent kind launched by `a` / `c` / `s`; does not affect `g`, which focuses whatever agent is already attributed regardless of kind |
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

`plugin link` does not build the crate — only a GitHub-managed
`plugin install` does that. Build the release binary yourself before opening
the pane:

```sh
make build
```

Run every quality gate — format, lint, tests, and the 80% coverage floor:

```sh
make check
```

## Licence

MIT
