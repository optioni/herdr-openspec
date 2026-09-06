# AGENTS.md

## Project overview

`herdr-openspec` is a [Herdr](https://herdr.dev) plugin: a read-only OpenSpec
dashboard rendered in a Herdr pane, written in Rust. It lists active and archived
changes, renders their artifacts as schema-driven tabs, shows task progress, maps
live Herdr agents onto changes, and launches an agent onto a change on request.

The durable sources of truth:

- **`PRD.md`** — requirements, goals, non-goals, risks.
- **`SPEC.md`** — the design contract every change implements.
- **`openspec/IMPLEMENTATION-ORDER.md`** — the roadmap and dependency graph.
- **`openspec/changes/<change>/`** — artifacts for an in-flight change.

Where prose and `SPEC.md` disagree, `SPEC.md` wins. When a change reveals that the
spec is wrong, update the spec as part of that change rather than letting the two drift.

## Current repo state

`repo-foundation`, `ci-pipeline`, `plugin-config`, `repo-resolution`,
`schema-model`, `task-parsing`, `changes-from-files`, `subprocess-seam`,
`changes-from-cli`, `tui-shell`, `list-view`, `markdown-viewer`,
`detail-view`, `tasks-tab`, and `live-refresh` have landed: the crate builds with six third-party dependencies (`toml`,
`yaml-rust2`, `serde_json`, `ratatui` — reached through `ratatui::crossterm`'s
re-export, not a direct dependency — `pulldown-cmark`, and `notify`), `make check` runs
all four quality gates locally and in CI, the
crate reads `config.toml` and derives and records agent-name mappings under
`HERDR_PLUGIN_STATE_DIR`, it can locate the OpenSpec repository root and the
`openspec` binary — the binary chain's fourth probe step is an injected hook
whose production binding, `cli::npm_prefix`, runs the real `npm prefix -g`
probe behind the subprocess seam — it reads the repository's schema and
produces the ordered artifact list including the tasks artifact, it parses a
task file into groups, items, and completion counts that agree with the
CLI's own, and it enumerates `openspec/changes/` into the
`Change`/`ChangeSet` values the dashboard renders — active and archived,
with every artifact resolved and every change's task progress counted by
the CLI's own fallback rule, file-sourced when no `openspec` binary is
present at all, and corrected by `changes::from_cli`/`changes::merge` when
it is: the CLI's schema, progress, and artifacts replace the file's for
every change it reports, joined by position, archived changes staying
permanently file-sourced. `herdr-openspec ui` now opens a real dashboard:
raw mode and the alternate screen entered and left in a fixed, mirrored
order (restored on normal return, error return, and panic alike), a
draw-then-wait event loop — `q`/`Ctrl-C` to quit, `j`/`k`/arrows to move the
list selection at the list route or scroll the detail content at the detail
route, `/` to filter (where printable keys type instead of commanding and
only `Ctrl-C` still quits), `Enter`/`Esc` to move between the list and detail
routes outside filter mode — and a 100-column breakpoint deciding a one- or
two-region body. The list region now fills with real rows — active changes,
a separator, then archived ones, with selection, scrolling, and a `/` filter
— and the detail region now shows the selected change's own header (name,
schema, progress), an artifact tab bar built from the schema's declared
order and switched with `1`–`9`/`[`/`]`, and that tab's content, read
through an injected `&dyn Fn(&Path) -> Result<String, String>` reader
(`ui::read_artifact` is the one production binding, in `src/ui/mod.rs`) and
resolved once per `(change directory, tab)` rather than on every frame;
the content is scrollable and clamped against the content area's own
height, below the header and tab-bar rows, so a held key cannot run it
away. The tab the schema marks as tracking tasks (`ArtifactRef::tracks_tasks`,
set by position, never by id or filename) renders `ui::tasks`' grammar
instead of markdown: a progress bar showing the change's own `progress`
followed by task groups under their headings with a `[x]`/`[ ]` glyph per
item — read-only, with no key that toggles one. `ui` refuses to start with exit status 3 when stdout is not a terminal, which is
also what keeps `cargo test` (which spawns this binary) from ever putting a
real terminal into raw mode.

The pane is no longer file-once: a recursive `notify` watch on `openspec/`
(confined to `src/watch.rs`) and a worker thread (confined to
`src/refresh.rs`) drive live updates, neither reached from `src/ui/` — a
touched path is debounced and classified to the affected change, the
worker answers with the fast file-sourced set then the CLI-merged one, and
the render loop adopts whichever is ready on every frame without ever
waiting for either. `r` forces the same full refresh the pane issues once
at startup. A third collaborator, the Herdr agent poller (`src/agents.rs`,
also outside `src/ui/`), polls `herdr agent list` on its own roughly
one-second cadence and answers on its own non-blocking seam, so one wait
now serves all three.

Important files:

- `SPEC.md` — architecture and design contract.
- `PRD.md` — product requirements.
- `openspec/IMPLEMENTATION-ORDER.md` — phased roadmap with a Mermaid dependency graph.
- `openspec/config.yaml` — project context and per-artifact rules.
- `openspec/schemas/tdd/schema.yaml` — the active schema (vendored by graft).
- `.claude/agents/` — OpenSpec orchestration agents (vendored by graft).
- `graft.toml` / `graft.lock` — what is vendored, and at which commit.
- `Cargo.toml` — crate manifest.
- `Makefile` — the four quality gates behind `make check`.
- `.github/workflows/ci.yml` — runs the same gates on `ubuntu-latest` and
  `macos-latest`, coverage on Linux only.
- `scripts/build.sh` — the Herdr `[[build]]` step.
- `herdr-plugin.toml` — the Herdr plugin manifest.

## Environment

- **Rust** stable (1.91+ at time of writing). Two one-time components:
  `rustup component add clippy` and `cargo install cargo-llvm-cov`.
- **Herdr** 0.7.0 or later, for the plugin manifest format and the `plugin`,
  `agent`, and `pane` CLI surfaces.
- **OpenSpec CLI** (`@fission-ai/openspec`) — optional for the plugin at runtime,
  required for the workflow below. Installed under nvm here, so it is not always on
  the `PATH` a non-login shell inherits.
- **Platforms:** macOS and Linux. Windows is out of scope.

## Vendored files — do not edit in place

`openspec/schemas/tdd/` and `.claude/agents/` are vendored from
`github.com/optioni/openspec-schemas` by [graft](https://github.com/optioni/graft)
and are listed in `graft.lock`. Editing them here is pointless: the next
`graft sync` overwrites the change silently. Edit the source repository and re-sync.

Repository-specific guidance belongs in `openspec/config.yaml` under `context` and
`rules`, not in the vendored schema.

## OpenSpec workflow

The active schema is `tdd`:

```text
proposal -> specs -> design -> tasks -> planning-review
```

Non-trivial work goes through a change proposal before implementation. Bug fixes,
typos, and small refactors do not.

```sh
openspec list
openspec status --change "<change>" --json
openspec instructions <artifact-id> --change "<change>" --json
openspec validate --strict
```

Run `openspec validate --strict` before presenting a proposal as ready. After an
approved change is implemented and verified, run `openspec archive` so the specs
stay in sync.

## Quality gates

All four are enforced in CI, invoking the same `make` targets individually — with
coverage on Linux only — and are available locally behind one composite target:

```sh
make check
```

| Gate | Command |
|---|---|
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| Test | `cargo test --all-features` |
| Coverage | `cargo llvm-cov --fail-under-lines 80` |

Coverage is a floor that catches drift, not the mechanism that produces tests — the
`tdd` schema drives RED → GREEN → REFACTOR, so tests come first by construction.

## Architecture rules

Two boundaries carry the design. Respect them, or the coverage target becomes
unreachable and the tests become integration tests by accident.

- **Nothing spawns a process outside `cli`.** `src/cli.rs` is the one module in the
  crate permitted to name a process-spawn API (`process::Command`, `Command::new`,
  `Stdio`) — `OpenspecCli` and `HerdrCli` are traits whose real implementations do
  nothing but spawn and return stdout. Parsing, merging, and decisions live on the
  testable side of that seam. This is checked, not aspirational: a tree-wide grep
  excludes exactly `src/cli.rs` by path (never by base name, so a future
  `src/ui/cli.rs` is still caught) and fails if that exclusion is vacuous — if
  `src/cli.rs` is missing, or itself names no spawn API. Plugin context — the
  configuration directory, the state directory, the plugin root, and the
  workspace, tab, and pane ids — arrives in the environment of every process Herdr
  starts for a plugin (`HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR`,
  `HERDR_PLUGIN_ROOT`, and friends), so reading it is never a reason to spawn
  `herdr` from anywhere in the crate. The seam also parses nothing: it returns
  stdout verbatim, and every JSON parse lives on the testable side of it —
  `serde_json` must never appear in `src/cli.rs`, checked the same way.
- **Views do no I/O.** They are pure functions from state to a ratatui frame, tested
  by rendering into a `TestBackend` buffer at 60 and 120 columns. The pure set is
  **eight** files — `src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/layout.rs`,
  `src/ui/list.rs`, `src/ui/markdown.rs`, `src/ui/tasks.rs`, `src/ui/view.rs`, and
  `src/ui/driver.rs` — none of which names a filesystem, process, environment,
  network, or standard-I/O API (`src/ui/tasks.rs` calls `tasks::parse`, a pure
  function over `&str`, and never `tasks::read`, the filesystem edge, which the
  search also names). `src/ui/terminal.rs`
  is the only file in the crate permitted to name a crossterm terminal-mode function
  (`enable_raw_mode`, `disable_raw_mode`, `EnterAlternateScreen`,
  `LeaveAlternateScreen`) — checked the same tree-wide-grep-with-a-positive-control
  way as the subprocess seam above, `tests/` included. A test that reaches the real
  terminal implementation corrupts the developer's own session, because `cargo test`
  spawns this binary.
- **The artifact read is confined to one binding.** `src/ui/mod.rs` is the only
  file under `src/ui/` permitted to name `read_to_string`, and `ui::read_artifact`
  — its one production binding, calling `std::fs::read_to_string` — is the only
  place naming that function itself; `Dashboard::sync_detail` and `run_loop` take
  it as an injected `&dyn Fn(&Path) -> Result<String, String>`, never by name.
  Checked the same tree-wide-grep-with-a-positive-control way as the subprocess
  seam, with a second leg proving the injection is real rather than decorative.
- **The list region's two mandated interior widths are 38 and 58 columns** — the
  wide layout's `Length(40)` list column and the narrow layout's 60-column frame,
  each less two border columns. Every row-grammar test in `ui::list` asserts both.
- **`pulldown_cmark` is named only in `src/ui/markdown.rs`, and that module names
  no `ratatui` type.** The markdown parser stays replaceable by editing one file,
  on the same terms `src/cli.rs` is the crate's only process spawner; styling a
  segment is `ui::view`'s job, never `ui::markdown`'s.
- **The detail region's two mandated interior widths are 78 and 58 columns** — the
  wide layout's `Min(0)` detail column at the mandated 120-column frame and the
  narrow layout's 60-column frame in the detail route, each less two border
  columns. Every test in `ui::markdown`, `ui::detail`, and `ui::tasks` asserts
  both — all three because every public function there is parameterised by
  width, which is what makes an exemption-free width check possible.
- **The render path blocks on nothing but the terminal, and reads no clock.**
  `src/watch.rs`, `src/refresh.rs`, and `src/agents.rs` — all three outside
  `src/ui/` — hold the filesystem watcher, the refresh worker thread, and
  the Herdr agent poller thread; `run_loop` reaches them only through the
  non-blocking `FsEvents`/`Refresher`/`AgentPoll` trait objects, never a
  channel, a lock, or `Instant::now()` directly. The poller lives outside
  `src/ui/` for the same reason the other two do: `NOCLI-SHELL` forbids any
  file under `src/ui/` from naming `HerdrCli`, and a poller placed there
  would need an exemption from that check. Checked two ways: no file under
  `src/ui/` (tests included) names a blocking-wait or clock API, and,
  separately, none of the three seam modules' own production code blocks
  before the point each hands off to its background thread — a check
  inside the three files themselves, because a sweep scoped to `src/ui/`
  alone cannot see a `drain` or a `take_result` that blocks in its own
  module.

Further invariants from `SPEC.md`:

- **Never fail closed.** A missing OpenSpec CLI, an unknown schema, or an
  unreachable Herdr socket degrades the view. It never replaces it with an error screen.
- **The plugin's own writes are scoped to its state directory.** The dashboard reads
  `openspec/` and never writes there — an agent may be editing `tasks.md` in another
  pane. The one thing the plugin itself writes, the agent-name mapping, goes under
  `HERDR_PLUGIN_STATE_DIR` and nowhere else — never into the repository, and never
  into the configuration directory the user hand-edits.
- **Checkbox counting follows the OpenSpec CLI's rule exactly**, fenced and commented
  checkboxes included, because the file path and the CLI path must report the same
  progress for the same change (`SPEC.md` → Dual-source model). On the CLI side that
  count comes from `openspec list --json`'s `completedTasks`/`totalTasks` pair, never
  from `instructions apply`'s own `progress` field, which disagrees whenever a
  schema's `apply.tracks` is a glob.

Do not attribute an agent to a change on weak evidence. A terminal title is a
summary, not a change id. Unattributable agents are reported as a count, not guessed at.
`herdr agent list` is **session-global** — measured live, it returns byte-identical output
regardless of working directory and lists agents from other repositories — so repository
scope is this plugin's job, never Herdr's: every attribution tier is scoped to the
resolved repository, not only the fallback count. The name-match tier reads an agent's
`name` field, never its `agent` field, which is the agent *kind* (`"claude"` on every live
agent measured) and not a change identifier.

## Development

```sh
herdr plugin link .    # load the working tree as a plugin (does not build)
make build              # build the release binary the pane runs
make check              # every gate
```

## Conventions

- **Commits:** Conventional Commits — `type(scope): description`. Commit after each
  task or logical task group while implementing a change.
- **Branching:** committing straight to `main` is fine unless the change itself
  calls for isolation.
- **Language:** English for code, comments, commits, and documentation.
- **Dependencies:** check the current stable version before adding one; do not rely
  on remembered version numbers.
- **Environment-dependent code takes an injected lookup.** A function that needs the
  process environment takes a `&dyn Fn(&str) -> Option<String>` rather than calling
  `std::env::var` directly, and the crate confines that one real call to a single
  binding. `std::env::set_var` is `unsafe` in edition 2024 and `cargo test` runs
  tests in parallel threads of one process, so a test that sets a real environment
  variable corrupts its neighbours.
