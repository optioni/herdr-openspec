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
`detail-view`, `tasks-tab`, `live-refresh`, `agent-polling`,
`agent-attribution`, `agent-launch`, and `plugin-actions` have landed: the crate builds with six third-party dependencies (`toml`,
`yaml-rust2`, `serde_json`, `ratatui` — reached through `ratatui::crossterm`'s
re-export, not a direct dependency — `pulldown-cmark`, and `notify`), `make check` runs
every quality gate locally and in CI — `COLWIDTH` among them, sweeping every pure view
file other than `src/ui/layout.rs` for a `.chars()`-based width measurement now that
`ui::layout::columns`/`truncate_columns` are the crate's only one, agreeing by
construction with what `ratatui::buffer::Buffer::set_string` itself consumes — the
crate reads `config.toml` and derives and records agent-name mappings under
`HERDR_PLUGIN_STATE_DIR`, it can locate the OpenSpec repository root and the
`openspec` binary — the binary chain's fourth probe step and the environment
lookup its `PATH`/nvm steps read both arrive as fields on `Startup`
(`degraded-states`' addition), injected on the same terms as `state_dir`:
`run` passes `config::env_lookup()` for the environment and
`cli::npm_probe_hook` (itself a thin wrapper around the real `npm prefix -g`
binding, named to keep the CLI seam's own name out of `src/ui/`) for the
fourth-step hook, so a test can drive "nothing resolves" without touching
the real process environment. When every probe step comes up empty, the
dashboard runs in **file mode** — no `openspec` binary backs it, every
change is file-sourced, and the header badges `file mode` once the header is
wide enough to hold it — never an error screen. Every change's own
unresolved problems (an absent schema, an unparseable one, a
`tasks`-artifact that could not be found) render as a row in that change's
own detail pane, above whichever artifact-specific problems that tab
already showed, so a bad schema degrades one change's detail region rather
than the dashboard as a whole. It reads the repository's schema and
produces the ordered artifact list including the tasks artifact, it parses a
task file into groups, items, and completion counts that agree with the
CLI's own, and it enumerates `openspec/changes/` into the
`Change`/`ChangeSet` values the dashboard renders — active and archived,
with every artifact resolved and every change's task progress counted by
the CLI's own fallback rule, file-sourced when no `openspec` binary is
present at all, and corrected by `changes::from_cli`/`changes::merge` when
it is: the CLI's schema, progress, and artifacts replace the file's for
every change it reports, paired by name, with artifacts within a change
joined by position, archived changes staying permanently file-sourced. `herdr-openspec ui` now opens a real dashboard:
raw mode and the alternate screen entered and left in a fixed, mirrored
order (restored on normal return, error return, and panic alike), a
draw-then-wait event loop — `q`/`Ctrl-C` to quit, `j`/`k`/arrows to move the
list selection at the list route or scroll the detail content at the detail
route, `/` to filter (where printable keys type instead of commanding and
only `Ctrl-C` still quits), `Enter`/`Esc` to move between the list and detail
routes outside filter mode — and a 100-column breakpoint deciding a one- or
two-region body. The list region now fills with real rows under two foldable
section headers, `active` then `archived`, each carrying a glyph, a label and
an honest count, with `Space` toggling the section the cursor is on or in and
archived starting collapsed — with selection, scrolling, and a `/` filter that
forces every section open for as long as the query is non-empty
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

Each polled agent is attributed to a change (name-equality against a
`HERDR_PLUGIN_STATE_DIR`-recorded mapping, falling back to a count of
unattributed agents rather than a guess) and badged in the list; `a`/`c`/`s`
launch an agent onto the selected change with `/opsx:apply`/`continue`/`archive`
through a fourth collaborator, the launcher (`src/launch.rs`, also outside
`src/ui/`, the crate's third worker thread), and `g` focuses the agent already
running for it — both inert with no problem recorded when nothing applies, both
refused with a reason rendered as a problem row when the derived name is
already live. Two more binary subcommands, `open` and `open-tab` (`src/open.rs`,
the crate's third `HerdrCli` consumer and the fifth file on the seam gates'
`ALLOWED` list), open or focus the dashboard pane from Herdr's action menu —
`herdr-plugin.toml` declares both `[[actions]]` and a second, tab-placed
`[[panes]]` entry alongside the original split one. Neither subcommand renders
or needs a terminal, and neither ever passes `--cwd` to `herdr plugin pane
open`: Herdr 0.8.2 was measured to resolve the manifest's relative pane
`command` against `--cwd` too, not only against the plugin root, so `ui::run`
instead reads the workspace's own cwd from its injected Herdr context
(`ui::startup_cwd`) in preference to `std::env::current_dir()`.

Important files:

- `SPEC.md` — architecture and design contract.
- `PRD.md` — product requirements.
- `openspec/IMPLEMENTATION-ORDER.md` — phased roadmap with a Mermaid dependency graph.
- `openspec/config.yaml` — project context and per-artifact rules.
- `openspec/schemas/tdd/schema.yaml` — the active schema (vendored by graft).
- `.claude/agents/` — OpenSpec orchestration agents (vendored by graft).
- `graft.toml` / `graft.lock` — what is vendored, and at which commit.
- `Cargo.toml` — crate manifest.
- `Makefile` — the quality gates behind `make check`.
- `scripts/gates/` — the checked-in hygiene gates `make gates` invokes.
- `.github/workflows/ci.yml` — runs the same gates on `ubuntu-latest` and
  `macos-latest`, coverage on Linux only.
- `scripts/build.sh` — the Herdr `[[build]]` step.
- `herdr-plugin.toml` — the Herdr plugin manifest.

## Environment

- **Rust** — `Cargo.toml`'s `rust-version = "1.88"` is the supported floor; the
  reference machine runs 1.91. Two one-time components:
  `rustup component add clippy` and `cargo install cargo-llvm-cov`.
- **python3** — required by `make check`: `scripts/gates/deps.sh` parses `cargo
  metadata`'s JSON through it. Present on both GitHub runners and on the reference
  machine; no crate is added to do this instead.
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

Five are enforced in CI, invoking the same `make` targets individually — with
coverage on Linux only — and are available locally behind one composite target:

```sh
make check
```

| Gate | Command |
|---|---|
| Format | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| Hygiene gates | `make gates` |
| Test | `cargo test --all-features` |
| Coverage | `cargo llvm-cov --fail-under-lines 80` |

Coverage is enforced at two floors from that one run: the total above, and a
production-slice floor computed from the same `cargo llvm-cov` JSON export
(`scripts/coverage-prod.py`). The production floor is the falsifiable one — the total
does not fire until production coverage falls below roughly 44%, so a future change that
lowers only the total would otherwise still look compliant.

Coverage is a floor that catches drift, not the mechanism that produces tests — the
`tdd` schema drives RED → GREEN → REFACTOR, so tests come first by construction.

This repository has a **contract tier**, run by `cargo test` and therefore by the Test
gate above, with three members: `tests/manifest.rs` (the manifest/README/binary-name
triangle — a `herdr-plugin.toml`, `README.md`, or binary-name edit that drifts one
against another fails it, deliberately asserting nothing about `target/release/`, which
`make check` never builds), `tests/degraded_coverage.rs` (`SPEC.md`'s degraded-states
table bound to a named, passing proving test per row), and `tests/doc_contract.rs`
(seven further claims — the module map, the tested-modules list, the worker-thread
count, the MSRV, the gate-path programs, the manifest transcription, and the injected
OpenSpec context — each bound to the repository file that determines it; see `SPEC.md`
→ § Testing and quality gates → Doc-conformance checks). The rule all three share: **a
documented claim with a computable second site is bound to that site inside `cargo
test`, not left to a human re-reading it.** Durable because a future change adding a
module, a worker thread, or a gate program will otherwise not know why its `make check`
went red — the failure names both sides of the disagreement.

`make gates` is not two scripts — it is every hygiene gate this project has ever argued
for, extracted into its own repository file under `scripts/gates/` and composed into the
`Makefile`'s `gates:` recipe (`degraded-states`): a tree-wide grep, a per-file-set seam
check, a width assertion, a dependency argument, a build-graph snapshot, and a coverage-map
checker among them. **A gate's floor is its own script default** — every line in the
recipe runs the script bare, with no `MIN`/`SCAN_MIN` override, because the default is
kept at the gate's true measured floor rather than a value someone must remember to pass.
The one stated exception is a **multi-subject gate**: `NODEFAULT-UI` scans five distinct
type sets across the codebase (the view-layer dashboard types, `Refresh`, `Launch`,
`src/agents.rs`'s set, and `src/launch.rs`'s `Outcome`), and one shared default would
either pass vacuously for the smallest set or fail legitimately for the largest — so each
of its five recipe lines carries its own explicit `SCAN_MIN`, the only floors that live on
the `Makefile` line rather than the script default (`notes/gate-floors.md` in the
`degraded-states` change records how each was measured). Every gate is executed against a
recorded planted defect, not merely attested to catch one: `tests/gate-controls.toml`
binds each script under `scripts/gates/` to a plant, and `tests/gate_controls.rs` copies
the tree to a scratch directory, applies it, and requires that gate to exit non-zero — so
a script neutered to `exit 0` fails `cargo test` even though `make gates` alone would not
catch it. `tests/ci_workflow.rs` proves a narrower thing beside it: the recipe names every
script under `scripts/gates/` and vice versa, so an extracted gate can never silently drop
out of `make gates` — which says nothing on its own about whether the gate can still fail.

Three gates guard no standing, repository-wide invariant and are excluded from this tier,
or only partly so: `EXTENDED` is a per-change ratchet — a hardcoded list of test-name
pairs belonging to one past change, several already stale — that would compose a
permanently red step into `make check`; `TESTCOUNT` is a shell function other checks
source, not a check with a subject of its own; and `OPENSPEC-UNTOUCHED` is **split** — its
`git ls-files` legs need no `BASE` commit and are extracted as
`scripts/gates/openspec-untouched.sh`, run in `make gates`, while its tracked-diff leg
needs a `BASE` captured at the start of a change and stays a per-change invocation.
`scripts/gates/` therefore holds no file for `EXTENDED` or `TESTCOUNT`, but does hold one
for `OPENSPEC-UNTOUCHED`'s extracted half.

Two things are deliberately **not** composed into `make gates`/`make check`, each with its
own reason: `make gates-full` (`DEPS_FULL=1 /bin/sh scripts/gates/deps.sh`) runs the
`DEPS` legs that rebuild the crate — once for the release binary, once per dependency
removed — and has its own CI job on every push instead, because that cost does not belong
in every local run; and every capability spec's `## Purpose` is guarded by
`tests/spec_purposes.rs` inside `cargo test` rather than by a `make gates` script, because
`openspec validate --specs --strict` needs `node` and the `openspec` binary and so cannot
be wired into `make check` at all.

## Architecture rules

Two boundaries carry the design. Respect them, or the coverage target becomes
unreachable and the tests become integration tests by accident.

- **Nothing spawns a process outside `cli`.** `src/cli.rs` is the one module in the
  crate permitted to name a process-spawn API (`process::Command`, `Command::new`,
  `Stdio`) — `OpenspecCli` and `HerdrCli` are traits whose real implementations spawn
  and return stdout, and `RealOpenspecCli` alone also accepts two constructor
  arguments neither trait decides for itself: a caller-supplied working directory,
  the one lever that works because `openspec` resolves its own root from the
  process's cwd and has no flag naming one, and a caller-supplied one-entry `PATH`
  overlay, because `openspec` is an `#!/usr/bin/env node` shim installed beside the
  very `node` it needs, so the probe chain reaches steps 3 and 4 exactly when the
  child's inherited `PATH` cannot exec it — `RealHerdrCli` keeps both prohibitions.
  Parsing, merging, and decisions live on the testable side of that seam.
  `src/agents.rs`, `src/launch.rs`,
  and `src/open.rs` are the crate's **three** `HerdrCli` consumers, reaching it only
  through the trait object; none names a spawn API itself. This is checked, not
  aspirational: a tree-wide grep (`NOSPAWN-GREP`) excludes exactly `src/cli.rs` by path (never by
  base name, so a future `src/ui/cli.rs` is still caught) and fails if that
  exclusion is vacuous — if `src/cli.rs` is missing, or itself names no spawn API.
  `LAUNCHSEAM` checks the same property from `src/launch.rs`'s and `src/open.rs`'s
  side (run a second time, `ENTRY` pointed at `src/open.rs`'s own entry point): no
  spawn API, no `ratatui` type, and the `HerdrCli` handle confined to exactly
  **five** files — `src/cli.rs`, `src/agents.rs`, `src/ui/mod.rs`,
  `src/launch.rs`, and `src/open.rs`. `src/open.rs` starts no thread, unlike the
  other three consumers, so it is not added to `NOBLOCK`'s seam-module list — a
  one-shot subcommand that exits is not a collaborator beside the render loop.
  Plugin context — the configuration directory, the state directory, the plugin
  root, the workspace, tab, and pane ids, and (as `HERDR_PLUGIN_CONTEXT_JSON`,
  parsed by `open::context`) the workspace's own **cwd** — arrives in the
  environment of every process Herdr starts for a plugin (`HERDR_PLUGIN_CONFIG_DIR`,
  `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_ROOT`, and friends) — a `[[panes]]`
  process no less than an `[[actions]]` one, measured live in `plugin-actions`'
  own Change Review — so reading it is never a reason to spawn `herdr` from
  anywhere in the crate. The seam also parses nothing: it returns
  stdout verbatim, and every JSON parse lives on the testable side of it —
  `serde_json` must never appear in `src/cli.rs`, checked the same way.
- **Views do no I/O.** They are pure functions from state to a ratatui frame, tested
  by rendering into a `TestBackend` buffer at 60 and 120 columns. The pure set is
  **nine** files — `src/ui/app.rs`, `src/ui/detail.rs`, `src/ui/layout.rs`,
  `src/ui/list.rs`, `src/ui/markdown.rs`, `src/ui/palette.rs`, `src/ui/tasks.rs`,
  `src/ui/view.rs`, and
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
- **`pulldown_cmark` is named only in `src/ui/markdown.rs`, and `ratatui::style::Color`
  only in `src/ui/palette.rs`.** One rule, two seams: the markdown parser and the colour
  table each stay replaceable by editing one file, on the same terms `src/cli.rs` is the
  crate's only process spawner. `ui::markdown` names no `ratatui` type, and styling a
  segment is `ui::view`'s job, never `ui::markdown`'s; deciding what a span *looks* like
  is `ui::palette`'s, never a render call site's — every other file asks
  `palette::style(Role::…)` for a semantic role instead of naming a colour. `MDSEAM` and
  `PALETTE` enforce these the same tree-wide-grep-with-a-positive-control way, and
  `PALETTE` searches the whole of `src/`, inline `#[cfg(test)]` modules included: a render
  test asserts a cell's colour by comparing it against `palette::style(role)`, and the
  colour literals are written down exactly once, in `src/ui/palette.rs`'s own tests. The
  gate also fails when its exclusion goes **vacuous** — `src/ui/palette.rs` missing, or
  present but naming no `Color` — and when that file drops out of `noio-view.sh`'s or
  `colwidth.sh`'s `PURE` list, so the new module cannot silently stop being swept.
  The parser's option set is exactly `ENABLE_TABLES | ENABLE_STRIKETHROUGH`: turning on a
  further flag without also writing that construct's rendering path makes it **vanish**
  into `fold`'s `_ => {}` wildcards rather than degrade to literal text, and no gate can
  see a wrongly-added flag.
- **The detail region's two mandated interior widths are 78 and 58 columns** — the
  wide layout's `Min(0)` detail column at the mandated 120-column frame and the
  narrow layout's 60-column frame in the detail route, each less two border
  columns. Every test in `ui::markdown`, `ui::detail`, and `ui::tasks` asserts
  both — all three because every public function there is parameterised by
  width, which is what makes an exemption-free width check possible.
- **Every width computation under `src/ui/` is measured in terminal display columns, never
  a `char` count.** `ui::layout::columns`/`truncate_columns` are the crate's only measure,
  agreeing by construction with what `Buffer::set_string` itself consumes; `COLWIDTH` sweeps
  the other eight pure view files for `.chars().count()`/`.chars().take(`/a `Vec<char>`
  collect, because a `chars().count()` written later is silently correct against this
  project's own ASCII fixtures and wrong against anything else.
- **The render path blocks on nothing but the terminal, and reads no clock.**
  `src/watch.rs`, `src/refresh.rs`, `src/agents.rs`, and `src/launch.rs` — all
  four outside `src/ui/` — hold the filesystem watcher, the refresh worker
  thread, the Herdr agent poller thread, and the launcher's worker thread;
  `run_loop` reaches them only through the non-blocking
  `FsEvents`/`Refresher`/`AgentPoll`/`Launcher` trait objects, never a
  channel, a lock, or `Instant::now()` directly. All four live outside
  `src/ui/` for the same reason: `NOCLI-SHELL` forbids any file under
  `src/ui/` from naming `HerdrCli`, and any of them placed there would need
  an exemption from that check — `herdr agent start` alone blocks up to
  thirty seconds waiting for interactive readiness, which is why the
  launcher cannot be a synchronous call from the render path either. Checked
  two ways: no file under `src/ui/` (tests included) names a blocking-wait or
  clock API, and, separately, none of the four seam modules' own production
  code blocks before the point each hands off to its background thread — a
  check inside the four files themselves, because a sweep scoped to
  `src/ui/` alone cannot see a `drain` or a `take_result` that blocks in its
  own module. The one accepted exception is the per-frame artifact read —
  `Dashboard::sync_detail(read)`'s `std::fs::read_to_string` before every
  draw — bounded by the `artifact-content` cache keyed on `(change
  directory, tab)`, so it fires only on a tab switch, a selection change, or
  an adopted refresh, never on a held key (`seam-resilience` -> Decision 10).
- **The write boundary is the process, not the tree.** `herdr agent start`
  launches a process that will itself write inside `openspec/` — an agent
  editing `tasks.md` is the point of launching it, and that write is not
  this plugin's. The plugin's own writes stay exactly `agent-names.toml`
  under `HERDR_PLUGIN_STATE_DIR`, launched agent or not.

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
- **`SPEC.md`'s degraded-states table is machine-bound to named tests.**
  `tests/degraded-coverage.toml` names, for each of that table's rows, a test that proves
  it; `tests/degraded_coverage.rs` fails `make check` if a row is added or reworded with
  no matching proof, or if a proof's own test stops existing or stops passing.

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
