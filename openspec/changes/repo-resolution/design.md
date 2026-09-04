## Context

`plugin-config` landed `Config::openspec_bin`, `agent_kind`, and `archived_count` and
deliberately validated none of them: "this change hands over a value and validates
nothing about the path it names". This change is the other half — the first consumer of
that value, and the first code in the crate that touches a filesystem it did not create.

Two questions have to be answered before anything else in Phase 2 can run:

- **Which directory is the repository?** `changes-from-files` enumerates
  `openspec/changes/`; it needs a root. `SPEC.md` → Data layer → Resolution chain says
  to walk up from the invocation context's working directory looking for `openspec/`,
  and to render an empty state naming the directory searched when nothing is found.
- **Where is the `openspec` binary?** `changes-from-cli` invokes it. `SPEC.md` gives a
  four-step chain: the configured path, `PATH`, the nvm version directories, and
  `$(npm prefix -g)/bin/openspec`, "probed in order, and cached for the session".

**The constraint that shapes the whole design:** Phase 2 is pure transformations. No
terminal, no subprocess, no writes. Step 4 of the chain needs `npm prefix -g`, which is
a process spawn, and `openspec/IMPLEMENTATION-ORDER.md` puts the `cli` seam in Phase 3.
The step cannot be implemented here and must not be silently dropped, because dropping
it means the chain's ordering is never exercised past step 3 and `subprocess-seam`
inherits an unspecified insertion point.

**Facts gathered on the reference machine (macOS, Herdr 0.8.2, Rust 1.91.1) before
writing this design**, because every one of them changes a decision:

| Fact | Observed | Consequence |
|---|---|---|
| `command -v openspec` | `~/.nvm/versions/node/v24.20.0/bin/openspec`, `lrwxr-xr-x`, a **symlink** to `../lib/node_modules/@fission-ai/openspec/bin/openspec.js` | The usability check must follow symlinks — `fs::metadata`, never `symlink_metadata`, whose `is_file()` is false for a link |
| The symlink target | `-rwxr-xr-x`, 79 bytes | A `.js` file with a shebang. "Executable regular file" is the right test; an extension check would reject it |
| `ls ~/.nvm/versions/node` | `v24.18.0`, `v24.19.0`, `v24.20.0`, **all three holding `openspec`** | Step 3 has to choose. Version ordering is live behaviour on this machine, not a hypothetical |
| `npm prefix -g` | `/Users/juusopiikkila/.nvm/versions/node/v24.20.0` — the same tree step 3 searches | Step 4 is redundant *here*, which is why deferring it costs this machine nothing. It is not redundant for a system-node install |
| `npm prefix -g` on stderr | `npm:9: command not found: _omz_nvm_setup_completion` (a zsh plugin) | Whoever wires step 4 must read **stdout only** and trim it. Recorded for `subprocess-seam` |
| `NVM_DIR` | `/Users/juusopiikkila/.nvm` in an interactive shell | Set by nvm's shell hook. A Herdr pane process does not run a login shell, so it may be absent — the default must remain `$HOME/.nvm` |
| `grep -rn '"openspec"' src/` | **matches today**, `src/state.rs:653` (`join("openspec")` in a fixture) | The program-name-literal check `plugin-config` used cannot be reused. `HANDOFF.md` predicted exactly this |
| `grep -rnE 'process::Command\|Command::new\|Stdio' src/` | no match; fires on a planted `use std::process::Command;` in a copy of `src/` | The API-name check still works and is the one to keep |
| `dirname "$(command -v npm)"` | `.` — `npm` is a **shell function** in this user's zsh | Any `PATH`-stripping recipe must not resolve tool directories through `command -v` in the interactive shell |
| Removing only the nvm bin directory from `PATH` | `openspec` unresolvable but `npm` and `node` still resolvable (Homebrew) | The no-spawn run must strip *every* directory holding any of the three, tested per directory with `-x` |

## Goals / Non-Goals

**Goals**

- `resolve::find_repo` — the upward walk, with the innermost match winning and the
  not-found result naming where the search began.
- `resolve::openspec_bin` — the four-step chain, reporting the winning step, treating a
  mis-configured `openspec_bin` as a fall-through with a recorded problem.
- A step-4 seam that is real, exercised, and honest about being unwired.
- A session cache that is a value, not a global.
- Correct `SPEC.md` where this change proves it wrong, and hand the step-4 obligation to
  `subprocess-seam` in `openspec/IMPLEMENTATION-ORDER.md`.

**Non-Goals**

- No process spawn, no `cli` module, no new dependency.
- No reading of anything *inside* `openspec/` — that is `schema-model` and
  `changes-from-files`.
- No derivation of the starting directory from Herdr's environment. It is an argument.
- No execution or version-probing of the binary that is found.
- No concurrency test of the cache: `OnceLock` is std, and pinning std's guarantees is
  not this change's work. Nothing here spawns a thread.
- No Windows path handling. `platforms = ["macos", "linux"]`, and the execute-bit test is
  a Unix concept.

## Boundaries

| Piece | Where | Pattern it follows |
|---|---|---|
| `find_repo`, `openspec_bin`, `BinCache` | new `src/resolve.rs` | `src/config.rs`: a pure core taking injected collaborators, plus a thin filesystem edge, plus one composition binding the real world |
| The environment lookup | threaded as `&dyn Fn(&str) -> Option<String>` | Identical to `config::config_dir` and `state::state_dir`. `AGENTS.md` → Conventions requires it; `std::env::set_var` is `unsafe` in edition 2024 and races parallel tests |
| The npm-prefix hook | `&dyn Fn() -> Option<PathBuf>` | The same injection shape, one step further out: the collaborator that does not exist yet is a closure rather than a trait, because a trait with one method and one implementation is the closure with extra ceremony |
| `openspec_bin_from_env` | `src/resolve.rs` | `config::load_from_env`: one composition, nothing left to assert inside it |
| `pub mod resolve;` | `src/lib.rs` | Alongside `config` and `state` |
| An executable-file builder and a symlink helper for fixtures | `src/lib.rs` → `testutil` | Beside the existing `ScratchDir` and `snapshot`, both of which this change reuses unchanged |

**No process spawn is added.** `src/resolve.rs` names no process API at all. The `cli`
module still does not exist; `subprocess-seam` creates it. The `npm prefix -g` invocation
this change *describes* is a doc comment and a spec sentence, never a call.

**No view is added.** `ui` does not exist yet. Nothing in this change renders, so the
"views perform no I/O" and "view tests at 60 and 120 columns" concentration points have
nothing to bind to here.

**The `Change` type does not exist yet**, so the `from_files`/`from_cli` agreement
concentration point does not apply. `changes-from-files` introduces it.

## Contracts

Every consumer is a future change in this repository. Nothing outside it depends on any
of this.

```rust
// resolve — repository discovery
pub enum RepoSearch {
    Found { root: PathBuf },
    NotFound { searched_from: PathBuf },
}
pub fn find_repo(start: &Path) -> RepoSearch;

// resolve — the binary chain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinSource { Configured, Path, Nvm, NpmPrefix }

pub struct FoundBin { pub path: PathBuf, pub source: BinSource }
pub struct BinResolution { pub found: Option<FoundBin>, pub problems: Vec<String> }

pub fn openspec_bin(
    configured: Option<&Path>,
    env: &dyn Fn(&str) -> Option<String>,
    npm_prefix: &dyn Fn() -> Option<PathBuf>,
) -> BinResolution;

/// Step 4's collaborator. Returns `None` until `subprocess-seam` replaces it.
pub fn npm_prefix_deferred() -> Option<PathBuf>;

/// The single composition against the real process environment.
pub fn openspec_bin_from_env(config: &crate::config::Config) -> BinResolution;

// resolve — the session cache
#[derive(Default)]
pub struct BinCache { /* OnceLock<BinResolution> */ }
impl BinCache {
    pub fn get_or_probe(&self, probe: impl FnOnce() -> BinResolution) -> &BinResolution;
}
```

- **Error surface.** Both entry points are infallible. `find_repo` returns an enum whose
  `NotFound` arm carries the datum the empty state needs; `openspec_bin` returns a
  `BinResolution` whose `problems` follow `Config::problems` exactly — a `Vec<String>`
  of human-readable fallbacks, rendered by `degraded-states`, produced here. Nothing
  returns `Result` and nothing panics, including on a path that does not exist, a path
  that is a file where a directory was expected, and a directory that cannot be read.
- **`problems` has exactly one producer:** step 1. Steps 2–4 finding nothing is normal.
  An empty `problems` with `found: None` means "no CLI installed"; a non-empty one means
  "the user configured something that does not work", and the two must stay
  distinguishable — that is the whole reason `problems` exists on this type.
- **`BinSource` is part of the contract, not a debugging aid.** It is how a test proves
  the chain's *order* rather than proving that some path came back. Two steps can
  legitimately produce the same path (on the reference machine, step 3 and step 4 do),
  so path equality alone cannot distinguish them.
- **The returned binary path is not canonicalized.** It is the path as the chain built
  it. The real install is a symlink, and the symlink is the stable name to execute.
- **The repository root *is* canonicalized** when the filesystem can resolve the
  starting path, so that `..` and symlinked working directories do not leak into a path
  the empty state prints or `changes-from-files` joins onto. These two rules point in
  opposite directions on purpose; the reasons are different and both are stated in the
  specs.
- **Compatibility.** Purely additive. No existing signature changes, `src/main.rs` is
  untouched, and the `ui` behaviour `plugin-build` and `plugin-manifest` specify is
  unaffected.
- **Not BREAKING.** No manifest key, no config key, no keybinding.
- **Consumers named:** `changes-from-files` (`find_repo`, and `archived_count` from
  `plugin-config`), `changes-from-cli` (`openspec_bin`, `BinCache`),
  `subprocess-seam` (replaces `npm_prefix_deferred`), `degraded-states`
  (`BinResolution::problems`, `RepoSearch::NotFound`), `tui-shell` (holds the `BinCache`
  for the process).

## Persistence and Rollout

- **Migration:** none. No on-disk format is introduced, read, or altered.
- **Backfill:** none.
- **Seeding:** none. Every fixture is built by the test that uses it and removed on drop.
- **Cache invalidation:** the one cache is per-process and per-value, and is never
  invalidated within a process — that is the specified behaviour ("cached for the
  session"). `live-refresh`'s `r` key re-reads changes, not the binary path; if it ever
  needs to, it constructs a new `BinCache`, which is why the cache is a value.
- **Index rebuild:** none.
- **Authorization:** none — a single-user local plugin. The privilege question is what it
  *reads*: directory listings under `PATH`, `NVM_DIR`/`$HOME/.nvm`, and the walked
  ancestor chain. It writes nothing anywhere, which two scenarios assert.
- **Observability:** `BinResolution::problems` is the whole of it, matching
  `Config::problems`. Nothing logs and nothing prints; there is no view yet. This change
  adds the corresponding row to `SPEC.md` → Degraded states so `degraded-states`
  inherits it rather than inventing it.
- **Deployment:** none beyond the normal build. No dependency is added, so `Cargo.lock`
  does not move and `Swatinem/rust-cache` is unaffected.

## Test Boundaries

This change takes no automated acceptance tier (see Test Strategy), so the first column
names each collaborator's treatment in the deterministic command checks that stand in
for one.

| Dependency | In acceptance test (deterministic command checks) | In unit tests |
|---|---|---|
| Process environment | real — the no-spawn run manipulates the real `PATH` of the `cargo test` process | **replaced**: every function takes an environment lookup closure over a fixture map. `std::env::set_var` is never called |
| `std::env::var` | real — reached only through `config::env_lookup`, which `plugin-config` already pins with its own assertions | called only inside `env_lookup`. `openspec_bin_from_env` reaches it; its one test configures a scratch executable so step 1 short-circuits before `PATH`, `HOME`, or `NVM_DIR` are read, which is what makes that test deterministic |
| `std::env::temp_dir()` | not used | real, and read-only — it reads `TMPDIR`. It is where `ScratchDir` puts every fixture |
| Filesystem — `PATH` directories | not used | real, but always fresh scratch directories. Never a directory on the machine's real `PATH` |
| Filesystem — nvm tree | not used | real scratch. `NVM_DIR` and `HOME` in the fixture lookup point at scratch trees; the user's real `~/.nvm` is never read by a unit test |
| Filesystem — npm prefix | not used | real scratch. Reached only through the injected hook, which in tests is a closure returning a scratch path |
| Filesystem — repository tree | read-only. `openspec validate --strict` is the final gate and reads it | real scratch. One fixture holds `openspec/changes/x/tasks.md` purely to assert it is **not** modified |
| Symbolic links | not used | real, via `std::os::unix::fs::symlink`. Four scenarios depend on link behaviour, and the real install is a symlink |
| Unix permission bits | not used | real, via `std::os::unix::fs::PermissionsExt`. Modes `0755` and `0644` are set explicitly by the fixture builder; no test relies on a default umask |
| `openspec` binary | **not invoked.** The chain stats a path and returns it; nothing executes it, reads its version, or opens it | not touched. Every fixture "binary" is a 20-byte text file with the execute bit set |
| `npm` binary | **not invoked.** Step 4's collaborator is a closure | not touched |
| `node` binary | not used | not touched |
| `herdr` binary and socket | not used. No production code path invokes `herdr`, and this change adds no Herdr surface | not touched |
| `toml` crate | present in the build graph, unchanged | **not used by this change.** `resolve` parses no TOML; it receives an already-parsed `Config` |
| Clock and randomness | not used. The cache test counts probe invocations with an `AtomicUsize`; no test sleeps, polls, or reads the clock | not touched. `ScratchDir` names come from pid plus an atomic counter |
| Threads | none. Nothing in this change spawns one, and no test does | not touched |
| Terminal, `ratatui`, `crossterm` | not used — this change adds no view | not touched |
| `cargo`, `cargo metadata`, `cargo tree`, `cargo llvm-cov` | real — the no-spawn run, the unchanged-dependency check, and the coverage gate | real, as the test runner only |
| `python3` | real — parses `cargo metadata` JSON for the unchanged-dependency check. No `tomllib`, so no 3.11 floor | not touched |
| POSIX shell userland (`sh`, `grep`, `tr`, `dirname`, `mktemp`) | real. macOS/BSD is the reference platform: no `stat -c`, no `sed -i` without an argument, no `grep -P`, and no `paste -sd:`, which is a usage error on BSD and silently yields an empty string | not touched; modification times are compared in Rust through `std::fs::Metadata`, never through `stat` |
| `git` working tree | not used as a restore mechanism. No check in this change modifies a tracked file | not touched |
| The user's real `~/.nvm`, `~/.config/herdr`, `~/.local/state/herdr` | **not touched, and not read.** Unlike `plugin-config`, this change has no destructive check and links no probe plugin | not touched |

## Test Strategy

Tiers, fastest first:

- **unit** — `cargo test --all-features resolve::`, functions in `src/resolve.rs`.
  Pure ordering logic takes fixture closures; everything filesystem-shaped takes a
  scratch tree. **39 of the 40 scenarios live here.**
- **command check** — a shell command run once, its exit status inspected, recorded as a
  task. **One scenario:** "Resolution spawns no process", which is a fact about the
  compiled tree and about the process environment, not about running code.

There is no "existing test" tier row in this change: it declares no modified capability,
so no live spec's scenarios are carried. The live `plugin-build` requirement that the
normal-kind dependency list is exactly `["toml"]` is *re-verified* as task 7.4 rather
than as a matrix row, because it is not one of this change's scenarios — but a
resolution written with a `glob` or `which` crate would break it, so it is checked
rather than assumed.

**This change does not take the outer-loop acceptance test.** Its outermost surface is a
library API. `src/main.rs` is untouched, the binary's observable behaviour is still the
`ui` banner, and no caller consumes `RepoSearch` or `BinResolution` until
`changes-from-files`. An acceptance test would have to drive an entry point that ships to
nobody — the same argument `plugin-config` made, for the same reason. The unit tier
reaches every collaborator these functions have: a real filesystem and an environment
that is a closure. The only collaborator it cannot reach is the process spawn in step 4,
and the honest treatment of that is not a fake pretending to be `npm` — it is the
deferred hook, plus a test asserting the hook is empty today, plus the command check
proving nothing spawns.

Shorthand used below. Every command is written for BSD userland.

```sh
# A scratch directory per check; removed at the end of the task.
T=$(mktemp -d)

# NOTOOLS — a PATH with every directory holding npm, node, or openspec removed, and the
# Rust toolchain intact. Do NOT build this from `dirname "$(command -v npm)"`: in this
# user's zsh, `npm` is a shell function, so that yields `.` and the recipe silently
# removes nothing (verified). Do NOT use `env PATH=/usr/bin:/bin cargo ...` either: env
# execs through the NEW PATH and cargo lives in ~/.cargo/bin, so the check fails for the
# wrong reason. Test each directory directly instead.
NOTOOLS=$(printf '%s' "$PATH" | tr ':' '\n' | while IFS= read -r d; do
  [ -n "$d" ] || continue
  if [ -x "$d/npm" ] || [ -x "$d/node" ] || [ -x "$d/openspec" ]; then continue; fi
  printf '%s\n' "$d"
done | tr '\n' ':')

# Direct, normal-kind dependencies of the package, asserted — never printed.
# Unchanged from `plugin-config`: this change must not move it.
DEPS='import json,sys
p = json.load(sys.stdin)["packages"][0]
normal = [d for d in p["dependencies"] if d.get("kind") is None]
assert sorted(d["name"] for d in normal) == ["toml"], normal'
```

The two checks that contain a pipe or a regex alternation are written here rather than
inside a table cell, because `|` cannot appear unescaped in a Markdown table and an
escaped `\|` inside an ERE means a *literal* pipe — `grep -E 'a\|b'` matches the string
`a|b` and finds nothing, so the check would pass against the very code it is meant to
catch. The matrix row below names these by label.

```sh
# SPAWN — no spawn API anywhere, and no process API at all in the new module.
# `std::process` alone is NOT usable tree-wide: src/main.rs legitimately calls
# std::process::exit, which is not a spawn. Verified on this tree: the tree-wide form is
# clean today and fires on a planted `use std::process::Command;` in a copy of src/.
! grep -rnE 'process::Command|Command::new|Stdio' src/
! grep -nE 'std::process|Command|Stdio' src/resolve.rs
#
# There is deliberately NO `! grep -rn '"openspec"' src/` here. That check, which
# `plugin-config` used, is already false at HEAD — src/state.rs:653 joins the literal
# "openspec" to build a fixture path — and this change necessarily adds more of them.
# A program-name-literal search cannot distinguish a path component from an argv[0].

# NOSPAWN-RUN — the suite passes with npm, node, and openspec all unresolvable and the
# toolchain intact. The three assertions come FIRST: without them the run proves nothing,
# because a PATH that still resolves npm would pass whether or not anything spawns it.
for t in npm node openspec; do ! env PATH="$NOTOOLS" sh -c "command -v $t"; done
env PATH="$NOTOOLS" sh -c 'command -v cargo'
env PATH="$NOTOOLS" cargo test --all-features
```

**Where each check stops.** `SPAWN` is evidence about the *source text* — it would not
catch a spawn reached through a dependency, which is why the dependency set is checked
separately (7.4). `NOSPAWN-RUN` is evidence that the *test suite* needs none of the three
binaries; it is not proof that production code never spawns, because a spawn whose error
is swallowed would still pass. The two together, plus the closure-shaped step-4
collaborator that has no other implementation to call, are the argument. Stated here so
no later change inherits a stronger claim than was made.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| The starting directory is itself the repository | Scratch `R` with `R/openspec/`; assert `Found { root }` equals `canonicalize(R)`, not its parent and not `R/openspec` | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| The repository is an ancestor several levels up | Scratch `R/a/b/c` with only `R/openspec/`; assert the root is `canonicalize(R)` | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| The innermost repository wins | Both `R/openspec/` and `R/inner/openspec/`; start at `R/inner/deep`; assert `canonicalize(R/inner)` — and that it is not `canonicalize(R)`, which is the assertion that fails if the walk runs to the outermost match | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| A regular file named `openspec` is not a repository | `R/openspec` written as a file, `P/openspec/` a directory in `R`'s parent; assert `canonicalize(P)`. An `exists()`-based walk returns `R` | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| A symbolic link to a directory is a repository | `R/openspec` symlinked to a real directory elsewhere in the scratch tree; assert `canonicalize(R)` | unit | real scratch filesystem, `std::os::unix::fs::symlink` | `cargo test --all-features resolve::` |
| A dangling symbolic link named `openspec` is not a repository | `R/openspec` symlinked to a non-existent path, `P/openspec/` real; assert `canonicalize(P)`. A `symlink_metadata`-based check returns `R` | unit | real scratch filesystem, symlink | `cargo test --all-features resolve::` |
| A starting path containing `..` is resolved before the walk | Start at `R/a/../a`; assert the root is exactly `canonicalize(R)` **and** that its string contains no `..`. A lexical walk yields `R/a/..` | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| A starting path naming a regular file is walked from its parent | Start at `R/a/notes.md` with `R/openspec/`; assert `canonicalize(R)` | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| A starting directory that does not exist is not an error | Start at `S/nope`, never created; assert `NotFound { searched_from }` equals `S/nope` verbatim — not empty, not `.`, not canonicalized — and that `S/nope` still does not exist afterwards | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| No repository anywhere up to the filesystem root | Assert first, with an explanatory message, that no ancestor of the scratch directory holds an `openspec` directory; then assert `NotFound` naming `canonicalize(S)`, and that the call returned at all — a walk that failed to stop at `/` hangs the suite rather than passing | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| A repository tree is byte-identical after discovery | Snapshot paths, bytes, and mtimes of `R` holding `openspec/changes/x/tasks.md`, `openspec/specs/`, and `README.md` through `std::fs::Metadata`; run discovery twice from `R/openspec/changes/x`; assert snapshot equality. Reuses `testutil::snapshot` unchanged | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| A missing starting directory is not created | Discovery from an uncreated scratch path; assert neither it nor any intermediate directory exists afterwards, and the scratch root's listing is unchanged | unit | real scratch filesystem | `cargo test --all-features resolve::` |
| An executable regular file is usable | One `PATH` entry `D`, `D/openspec` mode `0755`, nothing else present; assert the path and `BinSource::Path` | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| A file without an execute bit is skipped | `PATH=A:B`, `A/openspec` mode `0644`, `B/openspec` mode `0755`; assert `B/openspec`. An `is_file()`-only check returns `A` | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| A directory named `openspec` is skipped | `PATH=A:B`, `A/openspec` a directory, `B/openspec` executable; assert `B/openspec`. A mode-bits-only check returns `A`, since a directory has the execute bit | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| A symbolic link to an executable file is usable and is returned unresolved | `A/openspec` symlinked to `T/openspec.js` mode `0755`; assert the result is `A/openspec` **and not** `T/openspec.js`, so the canonicalize-the-result implementation fails | unit | real scratch filesystem, symlink | `cargo test --all-features resolve::` |
| A dangling symbolic link is skipped | `A/openspec` symlinked to a non-existent path, `B/openspec` executable; assert `B/openspec` | unit | real scratch filesystem, symlink | `cargo test --all-features resolve::` |
| The configured path wins over every other source | Three distinct executables — configured `C/openspec`, `PATH` `D/openspec`, nvm `H/.nvm/versions/node/v20.0.0/bin/openspec`; assert `C/openspec` and `BinSource::Configured`, and assert in the same test that dropping the configured argument yields `D/openspec`, so the ordering claim is what is under test | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| `PATH` wins when nothing is configured | Nothing configured, `PATH` and nvm both populated with different executables; assert the `PATH` one and `BinSource::Path` | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| `PATH` entries are searched left to right | `PATH=A:B`, both executable; assert `A/openspec`; then re-run with `PATH=B:A` and assert `B/openspec`, so the test cannot pass by accident of directory-creation order | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| An empty `PATH` entry is not the current directory | `PATH=":D:"`; assert the absolute `D/openspec`, and assert the result is not the relative path `openspec` | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| An absent or blank `PATH` contributes nothing | Three runs — `PATH` absent, `""`, `"   "` — each with an npm prefix holding an executable; assert all three give the npm-prefix path and `BinSource::NpmPrefix` | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| The nvm tree is searched when `PATH` has nothing | `PATH` a directory with no `openspec`; `HOME` a scratch home holding `.nvm/versions/node/v24.20.0/bin/openspec` executable; assert that path and `BinSource::Nvm` | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| Node versions are ordered numerically, not lexically | `v9.99.99` and `v10.0.0` both executable; assert the `v10.0.0` path. Lexical ordering returns `v9.99.99` | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| A version directory without a usable binary is skipped | `v22.0.0/` empty, `v21.0.0/bin/openspec` mode `0644`, `v20.0.0/bin/openspec` mode `0755`; assert the `v20.0.0` path, so neither higher version ended the step | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| A version directory whose name is not a version is still eligible | Only `system/bin/openspec`, executable; assert that path and `BinSource::Nvm` | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| `NVM_DIR` overrides the default nvm root | Two scratch trees, one under `NVM_DIR` and one under `HOME/.nvm`, holding different executables; assert the `NVM_DIR` one, then re-run with `NVM_DIR="   "` and assert the `HOME` one | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| The npm prefix is the last resort | Nothing configured, `PATH` barren, no nvm tree, hook returns `N` holding `N/bin/openspec` executable; assert that path and `BinSource::NpmPrefix` | unit | real scratch filesystem, hook replaced by a closure | `cargo test --all-features resolve::` |
| An npm prefix without a usable binary yields nothing | Hook returns a directory with no `bin/openspec`; assert no binary and empty `problems` | unit | real scratch filesystem, hook replaced | `cargo test --all-features resolve::` |
| Nothing anywhere is a supported state, not a fault | Nothing configured, `PATH` absent, no nvm tree, hook returns `None`; assert no binary **and** `problems.is_empty()` — the second half is what separates this from the configured-wrong case | unit | environment replaced, hook replaced | `cargo test --all-features resolve::` |
| A configured path that does not exist falls through to `PATH` | Configured `G/openspec` never created, `PATH` holds `D/openspec` executable; assert `D/openspec`, `BinSource::Path`, exactly one problem containing `G/openspec`, and that `G/openspec` still does not exist | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| A configured path that is not executable falls through | Configured file mode `0644`, `PATH` holds an executable; assert the `PATH` path and exactly one problem naming the configured path | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| A configured path fails and nothing else is found | Configured path absent from disk, every other source empty; assert no binary **and** exactly one problem naming it. Paired with "Nothing anywhere", this is the assertion that makes `problems` load-bearing | unit | real scratch filesystem, environment replaced | `cargo test --all-features resolve::` |
| The shipped hook yields no prefix | Assert `npm_prefix_deferred()` is `None`, with the assertion message naming `subprocess-seam` as the change that will make it fail | unit | none — the deferred binding | `cargo test --all-features resolve::` |
| Resolution spawns no process | Assert `npm`, `node`, and `openspec` are each unresolvable on `NOTOOLS` and that `cargo` is resolvable, then run the suite on it; then the two source-text patterns. What each half does *not* prove is stated above the matrix | command check | real cargo, real `PATH`, npm/node/openspec deliberately unresolvable | the `NOSPAWN-RUN` and `SPAWN` blocks above |
| A second lookup does not re-probe | A probe closure incrementing an `AtomicUsize`; call `get_or_probe` twice; assert the counter is exactly 1 and both results are equal. No sleeping, no clock — the counter is read after both calls have returned | unit | none — pure, single-threaded | `cargo test --all-features resolve::` |
| A negative result is cached too | Same shape, probe returns `found: None`; assert counter 1 and no binary on both calls | unit | none — pure | `cargo test --all-features resolve::` |
| Two caches are independent | Two `BinCache` values, different probe results; assert each returns its own. A `static` cache fails this | unit | none — pure | `cargo test --all-features resolve::` |
| The composition honours a configured binary | Call `openspec_bin_from_env` with a `Config` whose `openspec_bin` is a scratch executable; assert that exact path and `BinSource::Configured`. Deterministic because step 1 short-circuits before the real `PATH` is read; discriminating because no other step could produce a scratch path | unit | **real** process environment via `config::env_lookup`, real scratch filesystem | `cargo test --all-features resolve::` |
| A full probe leaves the filesystem byte-identical | Snapshot paths, bytes, and mtimes of a scratch tree holding a `PATH` directory, an nvm tree, and an npm prefix; run a full four-step probe twice; assert snapshot equality and that no entry appeared | unit | real scratch filesystem, environment replaced, hook replaced | `cargo test --all-features resolve::` |

Forty scenarios, forty matrix rows, matched one-to-one by name.

## Decisions

**Step 4 is an injected closure, not a trait, not a deletion, and not a stub inside
`resolve`.** Three alternatives were considered.

*Delete step 4 until Phase 3.* Rejected: the chain's shape would then be three steps, and
`subprocess-seam` would have to re-open `resolve`, re-derive the insertion point, and
re-test the ordering. Worse, nothing would record that a fourth step was ever intended —
the roadmap row says "four-step", and a three-step implementation silently contradicts it.

*Introduce the `cli` trait now, with a no-op implementation.* Rejected: it moves Phase 3's
seam into Phase 2 to serve one call that is not even an `OpenspecCli` call (`npm` is
neither `openspec` nor `herdr` — see the `SPEC.md` correction below), and
`IMPLEMENTATION-ORDER.md` is explicit that the seam exists before anything crosses it,
not before anything needs it.

*A `&dyn Fn() -> Option<PathBuf>` parameter, bound in production to a function returning
`None`.* Chosen. It follows the pattern `AGENTS.md` already mandates for the environment
(`&dyn Fn(&str) -> Option<String>`), it makes step 4's ordering fully testable **today**
with a closure returning a scratch path, and it leaves `subprocess-seam` a one-function
change with a failing test to greet it. The cost is honest and recorded: until Phase 3,
a user whose only `openspec` is reachable via `npm prefix -g` gets file mode. On the
reference machine `npm prefix -g` resolves *into the nvm tree step 3 already searches*,
so that population is small.

**A trait was not chosen over a closure** because a trait with one method, one
production implementation, and one test implementation is a closure with ceremony. The
`cli` traits exist because they have two methods' worth of surface and two real consumers;
this has neither.

**`BinSource` is returned.** Without it, "the configured path wins" is asserted by
comparing a path — and on a machine where two steps yield the same path (the reference
machine, steps 3 and 4), such a test is green whichever step actually ran. Returning the
step makes the ordering the thing under test. This is the direct application of the
"tests that cannot fail" concentration point to a chain of fallbacks.

**A mis-configured `openspec_bin` falls through and is reported, rather than winning or
stopping the chain.** Winning would mean returning a path that cannot be executed, which
pushes the failure into `changes-from-cli` where its cause is invisible. Stopping would
fail closed, which `SPEC.md` forbids outright. Falling through silently would hide a
user's typo forever. Falling through *with a problem* is the only option that both
degrades and stays honest, and it reuses `Config::problems`' established shape so
`degraded-states` has one thing to render rather than two.

**Usability is `fs::metadata` + regular file + any execute bit.** `fs::metadata` follows
symlinks; `symlink_metadata` does not, and the installed CLI *is* a symlink — a
`symlink_metadata`-based check rejects the one binary on this machine. The regular-file
clause exists because a directory carries the execute bit, so mode bits alone accept a
directory named `openspec`. "Any execute bit" rather than "owner execute" because a
binary installed by another user with mode `0711` is still runnable.

**The binary path is returned unresolved; the repository root is canonicalized.** These
look inconsistent and are not. The binary path will be handed to a process spawn, where
the symlink is the stable, upgrade-surviving name — canonicalizing it would pin a
specific `node_modules` path that an `npm update` invalidates. The repository root will
be *joined onto* and *printed*, where a `..` component or an unresolved symlink produces
a path the user cannot act on.

**Node version ordering is numeric, with unparseable names last.** Lexical ordering puts
`v9.99.99` above `v10.0.0`, and nvm users routinely hold both majors. Unparseable names
(`system`, `iojs-v3.3.1`) are kept rather than dropped, because dropping them would make
an `nvm alias`-heavy setup resolve nothing at all; they sort last because a real version
directory is the better guess when both exist.

**`PATH` is split on `':'` by hand, and empty entries are dropped.** `std::env::split_paths`
is the cross-platform form, but it takes an `OsStr` while the injected lookup is
`String`-typed, and on Unix it *yields* empty entries rather than dropping them — so the
filtering would have to be written either way. POSIX says an empty `PATH` entry means the
current directory; honouring that in a plugin pane, which starts in whatever directory
the user's workspace is rooted at, would resolve an executable from a location the user
never nominated. The manifest declares `platforms = ["macos", "linux"]`, so the
hand-written split costs no portability that is in scope.

**The session cache is a value, not a `static`.** `cargo test` runs the suite in parallel
threads of *one process*, so a `static OnceLock` cache would be shared by every test and
the first test to probe would decide the answer for all of them — a class of flake that
appears as a failure in a test that does not mention caching. A value also lets
`live-refresh` construct a fresh cache if it ever needs to re-probe, without an
invalidation API. `OnceLock::get_or_init` supplies the whole implementation, which is why
the wrapper is one line; the test that justifies the wrapper is the probe counter, which
fails against a no-cache implementation.

**No filesystem abstraction is introduced.** Faking the filesystem would mean testing the
fake: symlink following, execute bits, and directory-versus-file are precisely the
behaviours under test, and they are the ones a hand-written fake gets wrong. The
repository's established technique is a real scratch directory under
`std::env::temp_dir()`, removed on drop, and `testutil::ScratchDir` already exists.

**No new dependency.** `glob` would supply step 3's `versions/node/*` and `which` would
supply step 2; both are one `read_dir` and one `split(':')` respectively, and `which`
additionally shells out on some platforms. `plugin-build`'s live requirement asserts the
normal-kind dependency list is exactly `["toml"]`, so adding either would need a delta
spec into a capability this change otherwise leaves alone — a good forcing function, and
the answer here is that neither earns it.

**`SPEC.md` → Architecture is wrong and is corrected here.** It says "All process
spawning sits behind two traits", `OpenspecCli` and `HerdrCli`. Step 4 spawns `npm`,
which is neither, so as written the specification either forbids step 4 or forces `npm`
to masquerade as one of the two. The correction states the invariant that is actually
load-bearing — `cli` is the only module that spawns — and names the `npm prefix -g`
probe as a third thing living there, injected into `resolve` as a closure so `resolve`
stays pure. The same sentence's "untestable residue is two thin wrappers and `main`"
enumeration gains it. `IMPLEMENTATION-ORDER.md`'s `subprocess-seam` row gains the probe
so the obligation is handed over rather than lost — the same technique `plugin-config`
used to hand `agent-launch` the derived-agent-name correction.

## Risks / Trade-offs

- **Step 4 is unwired until Phase 3, so a system-node install with no nvm and no `PATH`
  entry resolves nothing and the dashboard runs in file mode.** → Accepted and bounded.
  File mode is a fully supported state that `SPEC.md` already specifies, the user can set
  `openspec_bin` to fix it immediately, and the gap closes in `subprocess-seam`. The
  spec test on the empty hook is what stops the deferral from being forgotten.
- **The no-spawn evidence is source-text and environment, not instrumentation.** A spawn
  reached through a dependency, or one whose error is swallowed, would pass both halves.
  → Mitigated by the third leg: the dependency set is asserted unchanged (7.4), and step
  4's collaborator is a closure with exactly one production binding that returns `None`.
  The limitation is stated above the matrix rather than left for a reader to discover.
- **`grep`-based checks are pattern matching, and this change adds path components that
  spell `openspec`.** → The program-name-literal check is deliberately *not* inherited
  from `plugin-config`; it is already false at HEAD and the design says so, so nobody
  reintroduces it believing it once passed.
- **`canonicalize` resolves symlinks, so on macOS a scratch path under `$TMPDIR` comes
  back as `/private/var/...`.** A test asserting a raw `ScratchDir` path against a
  canonicalized root fails on macOS and passes on Linux. → Every discovery test compares
  against `canonicalize(expected)`, and the tasks say so explicitly, because this is the
  most likely way for the suite to go green locally and red in CI or vice versa.
- **A `PATH` entry that is an unreadable directory.** `read_dir`/`metadata` return `Err`;
  the candidate is skipped. → No scenario builds an unreadable directory (creating one
  portably, and cleaning it up after a panic, is more risk than the case is worth), but
  the implementation treats every `Err` as "not a candidate" rather than unwrapping, and
  the tasks require that shape. Recorded as a deliberate coverage gap rather than an
  unnoticed one.
- **The upward walk visits every ancestor to `/`.** On a deeply nested path this is a
  handful of `metadata` calls; it is bounded by path depth and runs once per repository
  lookup. → No mitigation needed, but no depth limit is invented either: a limit would be
  a number with no principle behind it.
- **`openspec_bin_from_env`'s one test reads the real environment.** → It configures a
  scratch executable, so step 1 short-circuits and the real `PATH`, `HOME`, and `NVM_DIR`
  are never consulted. If that short-circuit ever moves, the test becomes machine-
  dependent — which is why the ordering is itself a spec scenario with its own test.

## Migration Plan

None applies. No data is stored, no format changes, no consumer exists yet, and there is
nothing to roll back beyond reverting the commit. Deploy order is irrelevant: the change
is a library addition to a plugin that is built from source at install time.

## Visual Design

Not applicable. This change adds no user-facing view and no email template — `ui` does
not exist yet, and `src/main.rs` is untouched. No design source exists or is needed.

## Open Questions

None blocking. Two deliberately settled rather than left open:

- *Should `find_repo` stop at a `.git` boundary?* No. `SPEC.md` says "walk up looking for
  `openspec/`" and nothing more; a `.git` ceiling is a rule with real consequences (a
  repository nested inside another would resolve differently) that nothing has asked for.
  If a user reports a surprising root, that is the moment to add it, with a scenario.
- *Should the nvm step also consider `$NVM_BIN`?* No. nvm exports it only inside a shell
  where nvm has been sourced; a Herdr pane process is exactly the case where it is
  absent, so it would add a code path that the environments needing step 3 never take.
