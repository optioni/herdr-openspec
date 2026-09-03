## Context

The repository holds `PRD.md`, `SPEC.md`, `AGENTS.md`, and the OpenSpec scaffolding and
nothing else — no `Cargo.toml`, no `Makefile`, no manifest. `SPEC.md` → Build and
distribution already fixes the shape: Herdr clones a plugin repository and runs the
`[[build]]` step in place, that step is `scripts/build.sh` rather than a bare
`cargo build --release` because a GUI or login-less Herdr launch may not have
`~/.cargo/bin` on `PATH`, and `SPEC.md` → Testing and quality gates fixes the four gates
and their exact commands.

Two constraints shape everything below. First, the coverage floor is 80% of lines and is
enforced from this change onward, on a crate whose only logic is argument
classification — so where that logic lives is a real decision, not a formality. Second,
`herdr plugin link .` must work when this change lands, which means a manifest and a
binary that a pane can actually run, months before the dashboard exists.

## Goals / Non-Goals

**Goals:**

- A crate that builds, formats, lints, tests, and clears the 80% coverage floor.
- One `make check` entry point that CI can call verbatim, so the two cannot diverge.
- A build script that works under the `PATH` Herdr actually gives it, and says something
  useful when Rust is genuinely missing.
- A manifest minimal enough to be honest — everything it declares works — and complete
  enough that `herdr plugin link .` succeeds.

**Non-Goals:**

- No TUI, no OpenSpec or Herdr reading, no `cli` subprocess seam, no `ui` view module.
  The architecture rules about the subprocess seam and pure views have nothing to bite on
  in this change; they start applying at `subprocess-seam` and `tui-shell`.
- No GitHub Actions workflow (`ci-pipeline`), no `[[actions]]` or `dashboard-tab`
  (`plugin-actions`), no prebuilt-binary fast path.
- The degraded states in `SPEC.md` — no `openspec/` directory, no active changes, a
  missing artifact file, a schema the CLI rejects, an unreachable Herdr socket, a pane
  under 100 columns — are not reachable here because nothing reads OpenSpec or Herdr yet.
  The one external-dependency degradation this change owns is "the toolchain is not where
  the build expected it", specified in `plugin-build` and `quality-gates`.

## Boundaries

| Piece | Where it lives | Pattern it follows |
|---|---|---|
| Argument classification, usage text, banner text | `src/lib.rs` (crate `herdr_openspec`) | The pure-transformation half of the architecture: no I/O, unit-tested directly |
| Argument reading, stdout/stderr writes, blocking on stdin, exit status | `src/main.rs` | The deliberately thin untestable residue named in `SPEC.md` → Architecture |
| End-to-end binary behaviour | `tests/cli.rs` | New tier for this repository; see Decisions |
| Build step | `scripts/build.sh` | `SPEC.md` → Build and distribution |
| Gates | `Makefile` | `SPEC.md` → Testing and quality gates, `AGENTS.md` → Quality gates |
| Plugin declaration | `herdr-plugin.toml` | `SPEC.md` → Herdr integration → Manifest, subset |
| Agent-facing guidance | `AGENTS.md` → "Current repo state" | Rewritten in place: the section asserts no `Cargo.toml` exists, which this change falsifies |

No module from the `SPEC.md` module map (`resolve`, `schema`, `changes`, `tasks`,
`agents`, `launch`, `watch`, `ui`, `cli`) is created, and **the `Change` type is neither
introduced nor altered here** — it arrives with `changes-from-files`, so the question of
keeping `from_files` and `from_cli` in agreement does not yet apply. In particular **no
process spawn is added outside `cli`**: production code in this change spawns nothing at
all. The only process spawning is in `tests/cli.rs`, which runs this crate's own binary;
it does not touch the `openspec` binary or the Herdr socket, so the seam has not moved.

## Contracts

Two interfaces gain a consumer outside this repository.

**`herdr-plugin.toml` → Herdr.** Additive: the file does not exist today. Consumer:
Herdr 0.7.0+, via `herdr plugin link .` and `herdr plugin install`. Shape is the subset
of `SPEC.md` → Manifest listed in `specs/plugin-manifest/spec.md`. Error surface: an
invalid manifest is rejected by `herdr plugin link` with a non-zero exit; a declared
command whose path does not exist produces a pane that starts and immediately exits. No
pagination or streaming. `plugin-actions` extends this manifest later with `[[actions]]`
and `dashboard-tab`; whether that extension is marked **BREAKING** is that change's
determination under the `openspec/config.yaml` rule, not something this design decides
for it.

**The `herdr-openspec` CLI surface.** Consumer: the manifest itself, and later Herdr's
action menu. Contract now: `ui` alone → banner on stdout, block until stdin EOF, exit 0;
every other argument list, including `ui` with a trailing argument → usage on stderr,
exit 2. `plugin-actions` adds `open` and `open --tab` alongside `ui`; the exit-2 default
is what lets a caller distinguish the new surface from today's.

**Agreement sites** — the string `herdr-openspec` and the path
`./target/release/herdr-openspec` must match across `Cargo.toml` (package and implicit
bin name), `herdr-plugin.toml` (`id` and the pane `command`), and `scripts/build.sh`
(which produces the artifact). Nothing in the toolchain checks this for us, so the
`plugin-manifest` scenario "The manifest path and the Cargo binary name agree" exists as
an explicit check over all three sites, and the manifest group carries a contract-gate
task.

## Persistence and Rollout

- **Migration:** none. No stored data exists.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. `target/` is git-ignored and rebuilt by the build step.
- **Index rebuild:** none.
- **Authorization:** none. The binary reads no credentials and opens no socket.
- **Observability:** none beyond the process exit status and the two output streams.
- **Deployment:** Herdr clones the repository and runs the `[[build]]` step in place;
  `herdr plugin link .` does the same from the working tree. There is no release
  artifact and no publish step in this change.
- **Documentation:** `AGENTS.md` → "Current repo state" is rewritten in the same change,
  because it currently instructs agents not to assume build, lint, or test commands
  exist. `README.md` → Install gains a caveat: it already advertises action-menu entries
  that arrive with `plugin-actions`.

## Test Boundaries

This change has no automated acceptance tier (see Test Strategy), so the first column
names the collaborator's treatment in the deterministic command checks that stand in for
one — the real `make`, `sh`, and `herdr` invocations recorded as `CHECK`/`VERIFY` tasks.

| Dependency | In acceptance test (deterministic command checks) | In unit tests |
|---|---|---|
| Filesystem (`target/`, repo tree, `HOME`) | real — the build script writes a real `target/`; the cargo-missing case uses a real temporary `HOME` | not touched; library functions are pure |
| `cargo` toolchain | real — invoked by `scripts/build.sh` and every `Makefile` gate | real, as the test runner only |
| `~/.cargo/env` | real when present; the absent case is produced by running the script under `env -i` with a temporary `HOME` | not touched |
| `clippy` component | real — `make lint`; the missing case uses a scratch `PATH` on which `cargo` resolves and `cargo-clippy` does not | not touched |
| `cargo-llvm-cov` | real — `make coverage`; the missing case uses the same scratch-`PATH` technique | not touched |
| `rustfmt` (bare, outside `cargo fmt`) | real — `rustfmt --check src/lib.rs`, to prove `rustfmt.toml` supplies the edition | not touched |
| `herdr` binary and socket | real, and manual — `herdr plugin link .` plus opening the pane. Not automated, and no gate depends on it | not touched |
| The built `herdr-openspec` binary | real — spawned by `tests/cli.rs` through `env!("CARGO_BIN_EXE_herdr-openspec")` | replaced: unit tests call the library functions directly and never spawn |
| Terminal and stdin | real — the pane; in `tests/cli.rs` stdin is `Stdio::null()` for the EOF cases and an open `Stdio::piped()` handle for the blocking case | not touched |
| `git` working tree | **not used as a restore mechanism.** The two gate-failure checks copy the file aside to a temporary path and restore from that copy, so an uncommitted implementation can never be destroyed by a revert | not touched |
| `python3` (with `tomllib`, 3.11+) | real — parses `herdr-plugin.toml` and `cargo metadata` JSON for the manifest checks. Where only Python < 3.11 is available, the fallback is `herdr plugin link .` itself plus a line-oriented read of the manifest | not touched |
| `make`, `sh`, and `dash` | real — `make` runs every gate; `sh -n` syntax-checks the build script, and `dash -n` additionally where `dash` exists, because `/bin/sh` on macOS is bash in POSIX mode and accepts bashisms | not touched |
| OpenSpec CLI, `openspec/` tree | not touched by any code path in this change; `openspec validate` is run read-only as a final gate | not touched |

## Test Strategy

Tiers used here, fastest first:

- **unit** — `cargo test --all-features`, library functions in `src/lib.rs`, no I/O.
- **binary integration** — `cargo test --all-features`, `tests/cli.rs`, spawning the
  crate's own binary via `env!("CARGO_BIN_EXE_herdr-openspec")`.
- **command check** — a shell command run once and its exit status and output inspected,
  recorded as a task. This is the tier for plumbing: build script, `Makefile` gates,
  manifest. Per the schema, a manifest key gets the command that consumes it as its
  check, not a test asserting the key back — and none of these checks may be committed
  as a Rust test.
- **manual** — Herdr linking and opening the pane, on a machine with Herdr installed.

**This change does not take the outer-loop acceptance test.** There is no client-visible
in-process entry point whose end-to-end wiring could be driven by a test: the change's
outermost surfaces are a shell script, a `Makefile`, and a TOML file, whose real
invocation *is* the strongest available evidence and is already scheduled as command
checks. A group-0 acceptance test would either re-run those same commands from inside
`cargo test` — recursively invoking cargo — or assert on file contents, which the schema
explicitly calls out as a test that cannot fail honestly. The three `tests/cli.rs`
scenarios are the end-to-end tier for the binary's own behaviour; they sit in group 2,
next to the code they drive.

Shorthand used in the commands below:

```sh
# A PATH on which cargo resolves but a named cargo subcommand does not.
T=$(mktemp -d); mkdir -p "$T/bin"; ln -s "$(command -v cargo)" "$T/bin/cargo"
NOTOOL="$T/bin:/usr/bin:/bin"
# A PATH from which every ~/.cargo entry has been removed.
STRIPPED=$(printf '%s' "$PATH" | tr ':' '\n' | grep -v '\.cargo' | tr '\n' ':')
```

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| Build succeeds with cargo already on PATH | Run the script, assert exit 0, the artifact is executable, and stderr carries no `~/.cargo/env` notice | command check | real filesystem, real cargo | `/bin/sh scripts/build.sh 2>err.log && test -x target/release/herdr-openspec && ! grep -q 'cargo/env' err.log` |
| Build succeeds when cargo is reachable only through `~/.cargo/env` | First assert cargo is genuinely unresolvable on the stripped `PATH`, then run the script and assert exit 0 *and* the sourced-notice on stderr, so the fallback branch is proven to have run | command check | real filesystem, real cargo, real `~/.cargo/env` | `env PATH="$STRIPPED" command -v cargo && exit 1; env PATH="$STRIPPED" /bin/sh scripts/build.sh 2>err.log; grep -q 'cargo/env' err.log` |
| Cargo cannot be found at all | Capture the artifact mtime, run under a cleared environment, assert non-zero, the two strings on stderr, and an unchanged mtime | command check | real filesystem, temporary `HOME`, cargo deliberately absent | `mtime() { stat -f %m "$1" 2>/dev/null \|\| stat -c %Y "$1"; }` — BSD and GNU `stat` differ — then `M=$(mtime target/release/herdr-openspec); env -i HOME=$(mktemp -d) PATH=/usr/bin:/bin /bin/sh scripts/build.sh 2>err.log; test $? -ne 0; grep -q rustup.rs err.log; test "$(mtime target/release/herdr-openspec)" = "$M"` |
| Script is POSIX shell, not bash | Syntax-check under `sh`, and under `dash` where available; search unanchored for every bashism the scenario names | command check | real filesystem, `sh`, `dash` when present | `sh -n scripts/build.sh; command -v dash >/dev/null && dash -n scripts/build.sh; ! grep -nE '\[\[\|\bfunction[[:space:]]\|\bsource[[:space:]]\|^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*=\(' scripts/build.sh` |
| Exactly one binary target is produced at the release path | Filter `cargo metadata` targets by `kind` containing `bin`, assert the count is 1 and the name matches; assert the built file is executable | command check | real cargo, real filesystem | `cargo metadata --no-deps --format-version 1 \| python3 -c 'import json,sys; t=[x for p in json.load(sys.stdin)["packages"] for x in p["targets"] if "bin" in x["kind"]]; assert [x["name"] for x in t]==["herdr-openspec"], t'` |
| No third-party dependencies are pulled in | Count `[[package]]` entries in `Cargo.lock` — the check `--offline` cannot make, because a warm registry lets a dependent build succeed offline | command check | real filesystem, real cargo | `test "$(grep -c '^\[\[package\]\]' Cargo.lock)" = 1 && cargo build --release --offline` |
| `ui` prints the placeholder banner | Spawn with `ui` and `Stdio::null()` stdin; assert exit 0, stdout contains the id, `CARGO_PKG_VERSION`, and the not-implemented line, stderr empty | binary integration | real binary, null stdin | `cargo test --all-features ui_prints_placeholder_banner` |
| `ui` holds the process open until stdin closes | Spawn with `Stdio::piped()` stdin left open; assert `try_wait()` is `None` after a short interval, then drop the handle and assert exit 0 | binary integration | real binary, real pipe | `cargo test --all-features ui_holds_open_until_stdin_closes` |
| Unknown subcommand | Spawn with `wat`; assert exit 2, stderr lists `ui` and echoes `wat`, stdout empty. Plus a unit test that the classifier rejects `wat` | binary integration + unit | real binary; library called directly | `cargo test --all-features unknown_subcommand` |
| Extra arguments after `ui` | Spawn with `ui --tab` and `Stdio::null()` stdin; assert exit 2 and `--tab` on stderr, and that it did not block. Plus a unit test for the classifier | binary integration + unit | real binary; library called directly | `cargo test --all-features extra_arguments_after_ui` |
| No subcommand | Spawn with no arguments; assert exit 2, stderr usage, stdout empty. Plus a unit test that the classifier rejects an empty argument list | binary integration + unit | real binary; library called directly | `cargo test --all-features no_subcommand` |
| All gates pass on a clean tree | Run the composite target, confirm the four commands run in order and it exits 0; then confirm the two non-gate targets are reachable | command check | real cargo, clippy, cargo-llvm-cov | `make check && make build && make fmt && git diff --quiet` |
| Format gate fails and stops the run | Copy a source file aside, misformat the original, run the composite, assert non-zero at the format gate and no later gate ran, restore from the copy | command check | real cargo fmt, temporary file copy (not `git checkout`) | `cp src/lib.rs "$T/lib.rs"; …; make check; cp "$T/lib.rs" src/lib.rs` |
| Lint gate fails on a clippy warning | Copy the file aside, add a clippy-triggering line, run the lint target, assert clippy reported it, restore from the copy | command check | real clippy, temporary file copy | `cp src/lib.rs "$T/lib.rs"; …; make lint; cp "$T/lib.rs" src/lib.rs` |
| Coverage passes at HEAD with the scaffold's only logic | Run the coverage target at HEAD and read the reported line percentage | command check | real cargo-llvm-cov | `make coverage` |
| The floor actually fails a build below it | Run the same command with an unreachable threshold and assert it exits non-zero — proving the flag gates rather than reports | command check | real cargo-llvm-cov | `cargo llvm-cov --fail-under-lines 100; test $? -ne 0` |
| `cargo-llvm-cov` is not installed | Run the coverage target on a `PATH` where cargo resolves but the subcommand does not; assert non-zero and the install command in stderr | command check | real cargo, cargo-llvm-cov deliberately unresolvable | `env PATH="$NOTOOL" make coverage 2>err.log; test $? -ne 0; grep -q 'cargo install cargo-llvm-cov' err.log` |
| The `clippy` component is not installed | Same technique for the lint target | command check | real cargo, cargo-clippy deliberately unresolvable | `env PATH="$NOTOOL" make lint 2>err.log; test $? -ne 0; grep -q 'rustup component add clippy' err.log` |
| The repository is formatted at HEAD | Run the format gate at HEAD | command check | real cargo fmt | `make fmt-check` |
| A bare `rustfmt` uses the configured edition | Run `rustfmt` directly with no `--edition`; it defaults to edition 2015 unless `rustfmt.toml` supplies one, so this fails if the file is missing or wrong | command check | real rustfmt, real `rustfmt.toml` | `rustfmt --check src/lib.rs` |
| Herdr is not installed | Run both entry points with `herdr` unavailable, and confirm by search that neither file invokes it | command check | herdr deliberately absent | `! grep -nE '(^\|[^-])herdr[[:space:]]' Makefile scripts/build.sh; make check && /bin/sh scripts/build.sh` |
| Herdr links the working tree | Link the working tree and list plugins | manual | real Herdr 0.7.0+ | `herdr plugin link . && herdr plugin list` |
| The dashboard pane launches the binary and stays open | Open the `dashboard` pane and observe the banner and that the pane does not close | manual | real Herdr, real terminal | Herdr pane menu |
| The manifest parses and its declared paths resolve | Parse the manifest with `tomllib`, assert the seven required keys carry exactly the specified values, and assert both declared paths are executable | command check | real filesystem, python3 3.11+ | `python3 -c 'import tomllib,os; m=tomllib.load(open("herdr-plugin.toml","rb")); …'` |
| No action or tab-pane entry is declared | Same parse: assert no `actions` key and no pane with id `dashboard-tab`; confirm no `OpenSpec:` entry in Herdr's menu | command check + manual | real filesystem, real Herdr | same parse, plus the Herdr action menu |
| The manifest path and the Cargo binary name agree | Compare the manifest pane command's basename with the single `bin` target name from `cargo metadata`, and confirm the file exists after the build | command check | real filesystem, real cargo | the manifest parse plus the `cargo metadata` filter above |

## Decisions

**Library plus thin `main`, rather than everything in `main.rs`.** `cargo llvm-cov`
measures the whole crate, and this crate has almost no code, so a handful of uncovered
lines is a large fraction. Putting classification, usage text, and the banner in
`src/lib.rs` makes them directly unit-testable and leaves `src/main.rs` at a few lines of
I/O. Alternative considered: everything in `main.rs` with `#[cfg(test)]` tests — rejected
because the I/O and the logic would then be interleaved in the same functions, which is
exactly the shape `SPEC.md` → Architecture says to avoid, and it would not survive the
arrival of the real `ui` module. `main` must set its failure status with
`std::process::exit(2)`, never `process::abort`, because the coverage profile is written
at exit and an abort discards it.

**A `tests/cli.rs` tier that spawns the crate's own binary.** The `ui`, blocking, and
usage scenarios are about exit codes, stream contents, and *not* exiting, which a unit
test on a pure function cannot observe. Cargo sets `CARGO_BIN_EXE_herdr-openspec` for
integration tests, so this needs no dependency and no path guessing. This is *not* a
breach of the "nothing spawns a process outside `cli`" rule: that rule is about the
`openspec` and `herdr` binaries in production code. It is recorded in Test Boundaries so
a later reviewer does not read it as seam drift. Alternative considered: asserting only
on the pure parse result and leaving exit codes unverified — rejected because the exit-2
default is the contract `plugin-actions` will rely on.

**A trivial covered unit is accepted, not a lowered floor.** The change ships real, if
small, logic and tests it, rather than waiving the gate for a scaffold. `cargo llvm-cov`
also collects profiles from subprocesses, so the `tests/cli.rs` runs are expected to
cover most of `src/main.rs` as well; the design does not depend on that — the floor is
cleared by the library tests alone. The floor's enforcement is itself checked, by running
the same command with an unreachable threshold and confirming it fails.

**`ui` blocks on stdin EOF rather than exiting immediately.** A pane whose command exits
at once is a closed pane, which makes the day-one dogfooding claim hollow. A PTY never
reaches EOF, so the pane stays visible; a closed stdin in tests reaches EOF at once.
Blocking costs two lines in `main` and is replaced wholesale by the event loop in
`tui-shell`. Alternative considered: `sleep` in a loop — rejected as a busy placeholder
with no natural exit.

**The manifest ships the pane only.** Declaring `[[actions]]` now would put two entries in
Herdr's action menu that invoke an `open` subcommand this change does not implement, which
would exit 2. Deferring them to `plugin-actions` keeps the rule that everything the
manifest declares actually works.

**`edition = "2024"`, `rust-version = "1.85"`, no third-party dependencies.** The edition
is chosen here — `SPEC.md` does not name one — because 2024 is the current stable edition
and nothing in the crate needs an older one. `rust-version` records the *edition floor*:
edition 2024 stabilised in Rust 1.85, so an older toolchain fails with a clear "requires
rustc 1.85" message rather than a syntax error. It is deliberately below the "Rust stable
(1.91+ at time of writing)" that `AGENTS.md` records as the toolchain actually in use, and
nothing in this change builds on 1.85 — the floor stays unverified until `ci-pipeline`
adds an MSRV job, if it ever wants one. `Cargo.toml` declares no `license` key: the crate
is never published to crates.io, the repository carries no `LICENSE` file, and adding one
is not this roadmap row's work. `SPEC.md` names the eventual dependency set (`ratatui`,
`crossterm`, `notify`, `serde_json`, `serde_yaml`, `pulldown-cmark`), and
`openspec/config.yaml` says adding one is a decision to argue in a proposal. None is
needed to print a banner, so each arrives with the change that first needs it, at the
version checked at that time.

**`make check` stays exactly the four gates, with their commands copied verbatim.** A
fifth step (for example `sh -n` over the build script) would put the `Makefile` out of
step with `PRD.md`, `SPEC.md`, and `AGENTS.md`, all of which name four. The script's
syntax check runs as a verification task in its own group instead, and shell linting in
CI is left to `ci-pipeline`. For the same reason `coverage` is `cargo llvm-cov
--fail-under-lines 80` verbatim, without the `--all-features` that `test` carries: the
divergence is inherited from `SPEC.md` and is harmless until the crate has a feature, at
which point it is `ci-pipeline`'s to reconcile.

## Risks / Trade-offs

- **Coverage sits just above the floor on a nearly empty crate; one uncovered line in
  `main` could tip it under** → keep `main` at the minimum, and let the `tests/cli.rs`
  subprocess runs cover it; if the figure is still marginal, the answer is more library
  tests, never a lower threshold.
- **`herdr plugin link .` cannot be verified on a machine without Herdr 0.7.0+** → the two
  Herdr scenarios are marked manual, and a separate scenario proves no gate depends on
  Herdr, so the change is still fully verifiable without it. If Herdr is unavailable at
  implementation time, record that the manual check was not run rather than claiming it.
- **The cargo-missing path of `scripts/build.sh` is easy to write and easy to get subtly
  wrong** (for example `set -e` aborting on a failed `command -v`) → its check runs the
  script under `env -i`, which is the real failure mode Herdr would produce, and asserts
  the artifact mtime is unchanged so a silent build cannot masquerade as the error path.
- **A fallback check that never exercises the fallback** → the `~/.cargo/env` scenario
  first asserts that `cargo` is genuinely unresolvable on the stripped `PATH`, and the
  script announces on stderr when it sources the file, so the check fails if the first
  probe succeeded through a Homebrew, asdf, or mise cargo the strip missed.
- **A gate-failure check that destroys uncommitted work** → the format and lint failure
  checks copy the file aside and restore from that copy. `git checkout --` is not used:
  it would discard the group-2 implementation along with the injected damage if the group
  were not yet committed.
- **`python3` with `tomllib` may be absent on a stock macOS** (Command Line Tools ships
  3.9) → the manifest checks fall back to `herdr plugin link .` plus a line-oriented read
  when `tomllib` is unavailable; the fallback is named in Test Boundaries.
- **`sh -n` proves little on macOS**, where `/bin/sh` is bash in POSIX mode and accepts
  `[[` and `source` → the check adds `dash -n` where `dash` exists and an unanchored
  search for the specific bashisms, so an indented `source` is caught on either platform.
- **The binary name appears in three files with nothing enforcing agreement** → the
  contract-gate task in the manifest group re-reads `SPEC.md` → Manifest and compares all
  three sites.

## Migration Plan

None needed. This change only adds files to a repository that has no build, no
deployment, and no stored state. Rollback is `git revert`; nothing outside the repository
holds state that would be left inconsistent. The one externally visible effect is that
`herdr plugin link .` starts succeeding, and `herdr plugin unlink` reverses it.

## Visual Design

Not applicable. This change builds no user-facing view and no email template — its
outputs are a shell script, a `Makefile`, a TOML manifest, and a banner line. No design
source exists or is needed. The dashboard's visual design is settled in `SPEC.md` →
User interface and is implemented from `tui-shell` onward.

## Open Questions

None blocking. The manifest format, the gate commands, the build-script behaviour, and
the coverage threshold are fixed by `SPEC.md` and `PRD.md`; this change implements them
rather than choosing them. The two things it does choose — the Rust edition and the
`rust-version` floor, neither of which `SPEC.md` names — are argued in Decisions and are
cheap to revisit before any dependency exists.
