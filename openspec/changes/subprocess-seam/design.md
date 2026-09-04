## Context

`SPEC.md` → Architecture makes one claim the whole test strategy rests on: **`cli` is the
only module in this crate permitted to spawn a process.** Everything else — `resolve`,
`schema`, `tasks`, `changes`, and later `agents`, `launch`, and `ui` — is a pure
transformation reachable by unit tests, which is why the crate sits at 98.66% line
coverage over 4924 lines with an 80% floor.

That claim is currently unfalsifiable, because nothing spawns anything. `src/cli.rs` does
not exist. `resolve::openspec_bin`'s fourth probe step takes its collaborator as an
injected `&dyn Fn() -> Option<PathBuf>`, and the binding `repo-resolution` shipped —
`resolve::npm_prefix_deferred` — returns `None` unconditionally. `repo-resolution` also
planted a test, `resolve::tests::the_shipped_hook_yields_no_prefix`, whose entire purpose
is to go **red** the day this change wires that hook, so the hand-over cannot pass
silently.

Three later changes queue behind the seam: `changes-from-cli` (`openspec list --json`,
`openspec status`, `openspec instructions apply`, `openspec schema which`),
`agent-polling` (`herdr agent list` at ~1s), and `agent-launch` (`herdr pane split`,
`herdr agent start`, `herdr agent prompt`, `herdr agent focus`). Each of them needs a fake
to test against. Building the seam and the fake now, before the first caller, is the
ordering principle `openspec/IMPLEMENTATION-ORDER.md` states: *"the subprocess seam exists
before anything crosses it."*

Two constraints shape everything below. First, no parsing lives here — a `run` that
returned anything but stdout would put decision-making on the untestable side of the seam,
which is the exact failure the seam exists to prevent. Second, `plugin-config` shipped a
no-spawn check (`grep -rn '"herdr"' src/`) that false-positived on three legitimate
`.join("herdr")` path-construction sites and was therefore recorded as **not run**. This
change introduces the crate's first real `Command::new`, so it needs a check that can
actually tell a spawn from a path join — and one that cannot pass on an empty or missing
input.

## Goals / Non-Goals

**Goals:**

- `src/cli.rs` holding `OpenspecCli`, `HerdrCli`, one real implementation each, a shared
  error type, one recording fake, and the real `npm prefix -g` probe — and exactly one
  `Command::new` in the whole crate.
- The `npm prefix -g` hand-over driven to red, observed, recorded, and then replaced.
- A real spawn driven end to end through `resolve::openspec_bin` to
  `<prefix>/bin/openspec`, a join no test has ever exercised with a real process.
- A genuine architectural no-spawn check with stated red conditions, replacing the
  literal-matching one that could not distinguish `.join("herdr")` from a spawn.
- Coverage held at or above its current level, with the seam's real implementations
  covered rather than written off as residue.

**Non-Goals:**

- No parsing of any program's output beyond trimming the npm prefix, which is contractual.
- No caller. Nothing calls `OpenspecCli` or `HerdrCli` after this change except the tests
  that prove them.
- No worker thread, no polling loop, no debounce, no async runtime, no timeout policy.
- No resolution chain for the `herdr` binary.
- No new dependency, no manifest change, no config-format change, no view.

## Boundaries

| Piece | Module | Pattern it follows |
|---|---|---|
| `OpenspecCli`, `HerdrCli` traits | new `src/cli.rs` | `SPEC.md` → Architecture's own sketch, strengthened with an error type and `Send + Sync` |
| `CliError` | new `src/cli.rs` | `schema::LoadError` — a `Debug + Clone + PartialEq + Eq` enum of struct variants, so callers branch on a variant instead of matching message substrings |
| `RealOpenspecCli`, `RealHerdrCli` | new `src/cli.rs` | thin wrappers over one private spawn helper; the "untestable residue" `SPEC.md` names, except that here it is testable |
| `cli::npm_prefix_from` | new `src/cli.rs` | the crate's pure-decision-plus-thin-binding shape, as `config::env_lookup` is to `config`'s environment reads |
| `cli::npm_prefix_via` | new `src/cli.rs` | the spawning probe against an explicit program — published so the end-to-end scenario and the binding's delegation test can both drive it without touching `PATH` |
| `cli::npm_prefix` | new `src/cli.rs` | one-line binding to the real world, exactly like `config::env_lookup` |
| The recording fake | new `src/cli.rs`, `#[cfg(test)]` | `testutil`'s existing `#[cfg(test)] pub(crate)` placement in `src/lib.rs` |
| Fourth-step rebinding | `src/resolve.rs` | unchanged injection point; only the binding passed by `openspec_bin_from_env` changes |
| Scratch programs | `src/cli.rs` tests | `testutil::{ScratchDir, write_with_mode}`, already used by `resolve`, `config`, `state`, `schema`, `tasks`, `changes` |

`src/lib.rs` gains one line, `pub mod cli;`. `src/resolve.rs` loses `npm_prefix_deferred`
and its pinning test, and `openspec_bin_from_env` passes `&crate::cli::npm_prefix`.

**Naming collision, kept deliberately.** `tests/cli.rs` already exists and means "the
binary's command-line interface"; `src/cli.rs` means "the subprocess seam". `SPEC.md` names
the module `cli` in its module map, and renaming either file to avoid the collision would
put the code somewhere `SPEC.md` does not say it is. The two are never imported together.

**No view, no I/O in a view.** This change adds no `ui` code, so the render seam is not at
stake and the 60/120-column view-test rule has nothing to bind to.

**The `Change` type is not touched**, so `from_files`/`from_cli` agreement is not at stake
here. `changes-from-cli` inherits that obligation, and this change hands it the fake it
will use to discharge it.

## Contracts

The public surface is new and therefore additive to anything outside the crate — the crate
has no external consumer. Inside the crate, one item is removed.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    /// The program could not be started at all: absent, not executable, or an OS error.
    NotStarted { program: String, args: Vec<String>, reason: String },
    /// It started, ran, and exited non-zero.
    Failed { program: String, args: Vec<String>, code: Option<i32>, stderr: String },
}

pub trait OpenspecCli: Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }
pub trait HerdrCli:    Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }

pub struct RealOpenspecCli { /* program: PathBuf */ }
impl RealOpenspecCli { pub fn new(program: impl Into<PathBuf>) -> Self; pub fn program(&self) -> &Path; }

pub struct RealHerdrCli { /* program: PathBuf */ }
impl RealHerdrCli  { pub fn new(program: impl Into<PathBuf>) -> Self; pub fn program(&self) -> &Path; }
impl Default for RealHerdrCli { /* program = "herdr" */ }

/// The one real binding: `npm_prefix_via(Path::new("npm"))`, and nothing else.
pub fn npm_prefix() -> Option<PathBuf>;
/// The same probe against an explicit program. Published, not private: the end-to-end
/// scenario and the binding's own delegation test both need it, and `changes-from-cli`
/// inherits it as the shape a future probe follows.
pub fn npm_prefix_via(program: &Path) -> Option<PathBuf>;
```

**Error surface.** Two variants, both recoverable. `NotStarted` is what an absent
`openspec` or an absent `herdr` produces, and both are documented degraded states, so no
caller may treat either variant as fatal. There is deliberately **no** UTF-8 variant:
undecodable bytes are decoded lossily, the way the OpenSpec CLI decodes a file it reads
(`SPEC.md` → Degraded states, the invalid-UTF-8 row — which is about a tasks file, so it
is a precedent for the choice rather than a rule that governs this case).

Both variants carry the **argument vector**. `changes-from-cli` drives four distinct
invocations through one `RealOpenspecCli` (`list`, `status`, `instructions apply`,
`schema which`) and `agent-launch` four more through one `RealHerdrCli` (`pane split`,
`agent start`, `agent prompt`, `agent focus`). Without the arguments, all eight failures
produce the same program name and a possibly empty stderr, and the Observability claim
below — that a caller can put a readable line on a `problems` vector — would be false.

**No pagination or streaming.** Every call is a one-shot `output()` that runs to
completion and buffers. `herdr agent list` and `openspec list --json` are small documents;
nothing in the roadmap streams.

**Compatibility.** Additive for all four future consumers (`changes-from-cli`,
`agent-polling`, `agent-launch`, `live-refresh`), none of which exists yet.
`resolve::npm_prefix_deferred` is **removed** — an internal break with exactly one caller,
`resolve::openspec_bin_from_env`, updated in the same change. Every test that injects its
own hook is unaffected, because the injection point does not move.

## Persistence and Rollout

- **Migration:** none. No data, no schema, no file format.
- **Backfill:** none.
- **Seeding:** none.
- **Cache invalidation:** none. `resolve::BinCache` is unchanged; the fourth step is
  probed at most once per cache, as before.
- **Index rebuild:** none.
- **Authorization:** none in the product sense. The one adjacent concern is that a spawned
  program inherits this process's environment and privileges; the design keeps that
  explicit by never mutating the environment and never changing the working directory.
- **Observability:** `CliError` carries the program name, the argument vector, the exit
  code, and stderr verbatim, so a caller can put a readable line on a `problems` vector
  that says *which* invocation failed. `cli` itself logs nothing and prints nothing — it
  has no side channel to the pane, and it does not trim stderr for the same reason it does
  not trim stdout.
- **Deployment:** none. Same binary, same manifest, same build step.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Outer-loop acceptance test | **not taken** — see Test Strategy for why | n/a |
| Filesystem (scratch tree under `std::env::temp_dir()`) | n/a | **real** — `testutil::ScratchDir` and `write_with_mode`, as every other filesystem-edge module does |
| The `openspec` binary | n/a | **never spawned.** `RealOpenspecCli` is pointed at scratch `#!/bin/sh` programs. Later changes use the fake |
| The `herdr` binary | n/a | **never spawned.** `RealHerdrCli`'s default program name is asserted on the held value, not by running it |
| `npm` (the real one, on `PATH`) | n/a | **real, in exactly one test** — the production binding's smoke test, which asserts only "no prefix, or an absolute path" and therefore passes whether or not `npm` exists. Every other npm behaviour uses a scratch program |
| `/bin/sh` | n/a | **real** — the scratch programs are shell scripts. Guaranteed on both supported platforms (`platforms = ["macos", "linux"]`) |
| The process environment | n/a | **replaced** in `resolve`'s tests by a lookup closure over a fixture map, unchanged. `cli` reads no environment variable at all, so it has nothing to replace. `std::env::set_var` is never called |
| The working directory | n/a | **real, read-only** — one scenario asserts the child inherits the test process's cwd. Nothing changes it |
| Threads | n/a | **real** — the `Send + Sync` scenarios spawn and join one thread; the stdin scenario polls a channel to a deadline rather than sleeping |
| The Herdr socket | n/a | **not touched.** No socket is opened anywhere in this change |
| The terminal / ratatui `TestBackend` | n/a | **not touched.** No view is added |
| `openspec/` (the repository's own OpenSpec tree) | n/a | **not touched.** Nothing in this change reads or writes a path under it; only this change's own artifacts are written, by the author |
| `cargo`, `cargo llvm-cov`, `make` | n/a | **real**, as the test runner and the gates |
| `find`, `grep`, `wc`, `xargs`, `sed`, `sort`, `python3` | n/a | **real**, in the command-level checks (`NOSPAWN-GREP`, `BINDING`, `DEPS`, `OPENSPEC-UNTOUCHED`). `python3` is used only by `DEPS`, to read `cargo metadata`'s JSON without adding a crate to do it |
| `git` | n/a | **real**, for the baseline SHA task 1.1 captures, and for both halves of `OPENSPEC-UNTOUCHED` — `git diff` against that SHA for tracked paths and `git ls-files --others` for untracked ones |
| Third-party crates | n/a | **none added.** Dependency set stays exactly `["toml", "yaml-rust2"]` |

## Test Strategy

**Tiers in this repository:** unit tests over the modules (`cargo test --all-features`);
view tests into a ratatui `TestBackend` (not applicable — no view); binary-integration
tests in `tests/` that spawn the crate's own binary (not applicable — `src/main.rs` is
untouched); and command-level checks that are shell blocks rather than Rust tests, run by
the implementer and recorded in tasks.md.

**No outer-loop acceptance test is taken.** The outermost surface this change ships is a
library API with no caller: `src/main.rs` is untouched, and nothing consumes `OpenspecCli`
or `HerdrCli` until `changes-from-cli`. An acceptance test would drive an entry point that
ships to nobody — the same reasoning `repo-resolution` recorded. The end-to-end obligation
this change carries is discharged differently and more usefully: the "A real spawn resolves
the npm-prefix binary" scenario drives a **real process spawn** through
`resolve::openspec_bin`'s whole chain to `<prefix>/bin/openspec`, which is the outermost
composition that actually exists.

### Command-level check blocks

These are written out here rather than inside a table cell, because a `|` cannot appear
unescaped in a Markdown table and an escaped `\|` inside an ERE means a **literal** pipe —
`grep -E 'a\|b'` matches the string `a|b` and finds nothing, so the check would pass
against the very code it is meant to catch. That defect was found in Phase 1 review. The
matrix rows below name these blocks by label.

```sh
# NOSPAWN-GREP — `cli` is the only module in the crate that spawns a process.
# Run as `sh scripts-free inline`; $SRC lets the same block be pointed at a copy of src/
# for the negative-control runs, which is what makes its red conditions demonstrable.
SRC="${SRC:-src}"
fail() { echo "NOSPAWN FAIL: $1" >&2; exit 1; }
SPAWN_RE='process::Command|Command::new|Stdio'

# Guard A — the tree and the one allowed spawner exist. A renamed, split, or moved seam
# (src/cli/mod.rs, say) must be a deliberate update to this block, never a silent stop.
[ -d "$SRC" ] || fail "no such directory: $SRC"
[ -f "$SRC/cli.rs" ] || fail "$SRC/cli.rs missing - the exclusion has nothing to exclude"

# Guard B — the excluded file actually spawns. Without this, a tree where the seam was
# gutted (or never written) passes, and the exclusion protects nothing.
grep -qE 'process::Command|Command::new' "$SRC/cli.rs" \
  || fail "$SRC/cli.rs names no spawn API - exclusion is vacuous"

# Guard C — the searched set is the crate's real module set, not an empty list. This is
# the `test -f`-shaped guard: grep exits 2 on a missing file and `!` would pass that.
# 8 is today's count (changes, config, lib, main, resolve, schema, state, tasks); a
# module deliberately removed is a deliberate edit here.
#
# The exclusion is BY PATH, not by base name. `! -name 'cli.rs'` would silently exempt a
# future `src/ui/cli.rs` — demonstrated at planning time: a spawn planted there passed
# the base-name form with the count still reading 8.
n=$(find "$SRC" -name '*.rs' ! -path "$SRC/cli.rs" | wc -l | tr -d ' ')
[ "$n" -ge 8 ] || fail "searched only $n files under $SRC (expected >= 8)"

# The check itself. Judged on OUTPUT EMPTINESS, never on a pipeline's exit status:
# grep exits 1 for no-match and 2 for a bad file, and `!` turns BOTH into a pass, which
# is the "verification that cannot fail" defect this repository keeps finding.
# The trailing /dev/null makes grep's argument list unconditionally non-empty, so the
# GNU-xargs empty-input case cannot make grep read stdin and hang. Guard C already makes
# that unreachable; this costs nothing and removes the reliance on it.
hits=$(find "$SRC" -name '*.rs' ! -path "$SRC/cli.rs" -print0 \
       | xargs -0 -I{} grep -nE "$SPAWN_RE" {} /dev/null 2>&1 || true)
[ -z "$hits" ] || { echo "NOSPAWN FAIL: spawn API outside $SRC/cli.rs:" >&2
                    echo "$hits" >&2; exit 1; }
echo "NOSPAWN OK: $n files checked under $SRC, only $SRC/cli.rs may spawn"

# MODULE-SCOPED, stricter, and unweakened from repo-resolution and its predecessors:
# these five files name no process API at all, spawning or not. The `[ -f ... ]` guards
# are load-bearing for the same exit-code-2 reason.
for f in src/resolve.rs src/config.rs src/state.rs src/schema.rs src/tasks.rs; do
  [ -f "$f" ] || { echo "MODULE-SCOPED FAIL: $f missing" >&2; exit 1; }
done
m=$(grep -nE 'std::process|Command|Stdio' src/resolve.rs src/config.rs src/state.rs \
    src/schema.rs src/tasks.rs || true)
[ -z "$m" ] || { echo "MODULE-SCOPED FAIL:" >&2; echo "$m" >&2; exit 1; }
```

Verified at planning time on the real tree: `NOSPAWN-GREP` **fails** on `src` today
(guard A: no `src/cli.rs` yet); **passes** on a copy with a spawning `cli.rs` added
(8 files checked, and the three real `.join("herdr")` sites in `src/config.rs` and
`src/state.rs` do not fire); **fails** on that copy with `Command::new("openspec")`
planted in `schema.rs`; **fails** on that copy with `cli.rs` emptied of its spawn
(guard B); and **fails** on a copy holding only `cli.rs` (guard C: `0 < 8`). The
module-scoped block passes on the tree as it stands.

**Why this replaces `plugin-config`'s check.** `! grep -rn '"herdr"' src/` fires on
`src/config.rs:59`, `src/state.rs:34`, and `src/state.rs:44` — three legitimate
`.join("herdr")` path constructions. A program-name literal cannot distinguish a path
component from an `argv[0]`, which is why that check was recorded as not-run and is not
inherited here. `NOSPAWN-GREP` matches process-API names only.

```sh
# NOSPAWN-RUN — the whole suite passes with npm, node, and openspec all unresolvable and
# the toolchain intact. Inherited from repo-resolution unchanged; its five preconditions
# come FIRST and must ABORT. A bare `! command -v ...` sequence without them exits 0 even
# when npm and node are still resolvable, which is verification stopping short of the
# step it vouches for.
NOTOOLS=$(printf '%s\n' "$PATH" | tr ':' '\n' | while IFS= read -r d; do
  [ -n "$d" ] || continue
  if [ -x "$d/npm" ] || [ -x "$d/node" ] || [ -x "$d/openspec" ]; then continue; fi
  printf '%s\n' "$d"
done | tr '\n' ':' | sed 's/:$//')
for t in npm node openspec; do
  if env PATH="$NOTOOLS" sh -c "command -v $t" >/dev/null 2>&1; then
    echo "PRECONDITION FAILED: $t still resolvable on NOTOOLS" >&2; exit 1
  fi
done
for t in cargo rustc; do
  env PATH="$NOTOOLS" sh -c "command -v $t" >/dev/null 2>&1 || {
    echo "PRECONDITION FAILED: $t not resolvable on NOTOOLS" >&2; exit 1; }
done
env PATH="$NOTOOLS" cargo test --all-features
```

```sh
# NPM-PATH — the environment the hand-over red must be observed in. Its precondition is
# MEASURED, never asserted: the point is not "npm is on PATH" but "the npm this process
# will spawn actually yields a prefix", which is the only condition under which the
# pinning test can go red at all.
NPMBIN="$HOME/.nvm/versions/node/v24.20.0/bin"
[ -x "$NPMBIN/npm" ] || { echo "PRECONDITION FAILED: no npm at $NPMBIN" >&2; exit 1; }
p=$("$NPMBIN/npm" prefix -g 2>/dev/null) || {
  echo "PRECONDITION FAILED: $NPMBIN/npm prefix -g exited non-zero" >&2; exit 1; }
case "$p" in /*) ;; *) echo "PRECONDITION FAILED: prefix not absolute: [$p]" >&2; exit 1;; esac
echo "PRECONDITION OK: npm prefix -g -> $p"
env PATH="$NPMBIN:$PATH" cargo test --all-features resolve::tests::the_shipped_hook_yields_no_prefix
```

**Measured at planning time, and it corrects an assumption this design first made.** The
first draft claimed `npm` is not on the `PATH` a plain shell inherits. That is **false**
here: `/bin/sh -c 'command -v npm'` prints `/opt/homebrew/bin/npm`, and `node` likewise —
only `openspec` is nvm-only. What is true is narrower and less stable: that Homebrew node
is currently broken (`dyld: Library not loaded: libllhttp.9.3.dylib`), so
`/opt/homebrew/bin/npm prefix -g` exits `134` with **empty stdout** and the probe correctly
reports no prefix. A `brew reinstall node` would flip that at any moment. So the plain-PATH
run in task 6.2 is **informational only** — its result is a fact about the machine, not a
prediction — and the red is guaranteed by the measured precondition above, not by an
assumption about `PATH`.

The nvm `npm` was measured directly: exit `0`, stdout
`/Users/<user>/.nvm/versions/node/v24.20.0\n`, stderr two lines of oh-my-zsh plugin noise
(`npm:9: command not found: _omz_nvm_setup_completion`). **That stderr noise comes from an
interactive-shell function wrapper, not from the `npm` binary**, so a child started with
`Command::new` will very likely see clean stderr. `SPEC.md`'s warning is still worth
honouring — it costs nothing and the wrapper is real for anyone running the command by
hand — but it must not be cited as evidence that the binary itself emits noise. The
stdout-only rule is proven by the scratch-program scenario, which writes a *different,
plausible* path to stderr, and structurally by the decision function having no stderr
parameter at all.

```sh
# BINDING — the hand-over actually happened. This is the ONE check that separates a real
# rebinding from `pub fn npm_prefix() -> Option<PathBuf> { None }`, which is the exact
# body being deleted and which every other check in this change lets through: the chain
# tests inject their own hooks, NOSPAWN-GREP and NOSPAWN-RUN say nothing about what a
# function returns, and "no prefix" is a legitimate answer for the smoke test.
[ -f src/resolve.rs ] || { echo "BINDING FAIL: src/resolve.rs missing" >&2; exit 1; }
[ -d src ] || { echo "BINDING FAIL: no src/" >&2; exit 1; }
# (a) resolve names the real probe where it passes its fourth-step hook.
grep -qE 'cli::npm_prefix' src/resolve.rs \
  || { echo "BINDING FAIL: src/resolve.rs does not name cli::npm_prefix" >&2; exit 1; }
# (b) the placeholder is gone everywhere, not merely shadowed.
gone=$(grep -rn 'npm_prefix_deferred' src/ || true)
[ -z "$gone" ] || { echo "BINDING FAIL: placeholder survives:" >&2; echo "$gone" >&2; exit 1; }
# (c) the real binding does not hardcode an empty answer. Matched on the function's own
# body, so `{ None }` and `-> Option<PathBuf> { None }` both fire.
body=$(sed -n '/pub fn npm_prefix()/,/^}/p' src/cli.rs)
[ -n "$body" ] || { echo "BINDING FAIL: no npm_prefix() found in src/cli.rs" >&2; exit 1; }
printf '%s\n' "$body" | grep -qE 'npm_prefix_via' \
  || { echo "BINDING FAIL: npm_prefix() does not delegate to the probe:" >&2
       printf '%s\n' "$body" >&2; exit 1; }
echo "BINDING OK"
```

Negative controls for `BINDING`, run against scratch copies the same way `NOSPAWN-GREP`'s
are: a `resolve.rs` with the `cli::npm_prefix` reference removed must fail (a); a tree
still holding `npm_prefix_deferred` anywhere must fail (b); and a `cli.rs` whose
`npm_prefix()` body is `None` must fail (c). Guard (c) is deliberately structural rather
than behavioural, because no assertion on the *value* `npm_prefix()` returns can be both
machine-independent and red for a hardcoded `None` — the unit test in group 6 covers the
behavioural half by comparing `npm_prefix()` against the probe pointed at the same program.

```sh
# DEPS — the dependency set is unchanged, normal AND dev AND build. An explicit exit, not
# a bare `assert`, which `python3 -O` or an inherited PYTHONOPTIMIZE strips (verified in
# an earlier phase: the assert form exits 0 for any dependency list). Dev and build are
# pinned empty because they are empty today, so a planted `assert_cmd` dev-dependency —
# the likeliest accidental addition when a change starts spawning things — cannot pass.
DEPS='import json,sys
p = json.load(sys.stdin)["packages"][0]
def names(kind): return sorted(d["name"] for d in p["dependencies"] if d.get("kind") == kind)
if names(None) != ["toml", "yaml-rust2"]: sys.exit("unexpected normal deps: %r" % (names(None),))
if names("dev") != []: sys.exit("unexpected dev deps: %r" % (names("dev"),))
if names("build") != []: sys.exit("unexpected build deps: %r" % (names("build"),))
print("DEPS OK")'
cargo metadata --no-deps --format-version 1 | python3 -c "$DEPS"
```

```sh
# OPENSPEC-UNTOUCHED — nothing under openspec/ changed except this change's own artifacts.
# Diffed against the BASE SHA captured in task 1.1, never against the index: this project
# commits per task group, so `git diff --exit-code` between working tree and index passes
# over the very change it exists to catch.
#
# `git diff` alone is NOT enough and the first draft of this block was wrong about that:
# it lists tracked paths only, and a file the plugin WRITES at runtime is untracked, so
# the one violation this check exists to catch was invisible to it. Demonstrated at
# planning time: a planted `openspec/specs/foo/cache.md` produced an empty stray list and
# the check reported success. The `ls-files --others` sweep is the half that catches it.
#
# `:(top)` anchors both pathspecs to the repository root and `git -C "$ROOT"` anchors the
# reported paths there too, so the check gives the same answer from any working directory.
# The first draft did neither and reported success when run from `src/`.
[ -n "$BASE" ] || { echo "FAIL: BASE sha not set" >&2; exit 1; }
ROOT=$(git rev-parse --show-toplevel) || { echo "FAIL: not a git repo" >&2; exit 1; }
git -C "$ROOT" rev-parse --verify "$BASE^{commit}" >/dev/null \
  || { echo "FAIL: bad BASE" >&2; exit 1; }
stray=$( { git -C "$ROOT" diff --name-only "$BASE" -- ':(top)openspec/'
           git -C "$ROOT" ls-files --others --exclude-standard -- ':(top)openspec/'; } \
         | grep -v '^openspec/changes/subprocess-seam/' | sort -u || true)
[ -z "$stray" ] || { echo "FAIL: wrote inside openspec/ outside this change:" >&2
                     echo "$stray" >&2; exit 1; }
echo "OPENSPEC-UNTOUCHED OK"
```

### Verification matrix

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| Stdout is returned verbatim on success | Scratch program prints two lines; assert `Ok("line one\nline two\n")` including the trailing newline, so a `.trim()` inside the seam fails | unit | real scratch filesystem, real `/bin/sh` | `cargo test --all-features cli::` |
| Stderr never reaches the success value | Scratch program writes to both streams; assert the `Ok` string equals the stdout payload and contains neither stderr token | unit | real scratch filesystem, real `/bin/sh` | `cargo test --all-features cli::` |
| A non-zero exit is a failure carrying the code and stderr | Scratch program writes both streams and exits 3; assert `Err(CliError::Failed { code: Some(3), stderr: "boom", .. })` and that the stdout payload is nowhere in the error | unit | real scratch filesystem, real `/bin/sh` | `cargo test --all-features cli::` |
| A program that cannot be started is a failure, not a panic | Point the implementation at an uncreated scratch path; assert `Err(CliError::NotStarted)` naming it, and assert the path still does not exist | unit | real scratch filesystem | `cargo test --all-features cli::` |
| Invalid UTF-8 on stdout is decoded lossily rather than failing | Scratch program emits `61 FF 62` via `printf '\141\377\142'`; assert `Ok("a\u{FFFD}b")` | unit | real scratch filesystem, real `/bin/sh` | `cargo test --all-features cli::` |
| Empty stdout with a zero exit is success, not a failure | Scratch program is `exit 0`; assert `Ok("")` | unit | real scratch filesystem, real `/bin/sh` | `cargo test --all-features cli::` |
| Arguments reach the program in order and unaltered | Scratch program prints each `"$@"` on its own line; call with `["list","--json","a b","--","-x"]`; assert the exact five-line output, `a b` on one line | unit | real scratch filesystem, real `/bin/sh` | `cargo test --all-features cli::` |
| A trait object crosses a thread boundary | Build `Arc<dyn OpenspecCli>` and `Arc<dyn HerdrCli>`, call once inline and once on a spawned-and-joined thread; assert equal results. Compiling at all is half the proof | unit | real thread, real scratch filesystem | `cargo test --all-features cli::` |
| The constructed path is the program that runs | Two scratch programs printing `A` and `B`; construct with `B/prog`; assert `Ok("B")` — and that constructing with `A/prog` yields `Ok("A")`, so the assertion discriminates | unit | real scratch filesystem, real `/bin/sh` | `cargo test --all-features cli::` |
| No argument is added and the working directory is inherited | Scratch program prints `$#` and `pwd`; call with `&[]`; assert `0` and `std::env::current_dir()` | unit | real scratch filesystem, real `/bin/sh`, real cwd (read-only) | `cargo test --all-features cli::` |
| The default Herdr program name is `herdr` | `RealHerdrCli::default().program()` equals `Path::new("herdr")`; no spawn, so it passes with no Herdr installed | unit | none | `cargo test --all-features cli::` |
| A program that reads stdin returns rather than blocking | Scratch program is `cat`; run it on a thread reporting through an `mpsc` channel; `recv_timeout`-poll to a 30s deadline in a loop; assert a result arrived and is `Ok("")`. No `sleep`-then-assert anywhere | unit | real thread, real channel, real `/bin/sh` | `cargo test --all-features cli::` |
| Invocations are recorded in call order | Fake called with three vectors; assert `calls()` equals those three in order | unit | none — the fake is the subject | `cargo test --all-features cli::` |
| A response is matched by the exact argument vector | Register `Ok("A")` for `["list","--json"]` and `Ok("B")` for `["list"]`; call the longer; assert `Ok("A")` | unit | none | `cargo test --all-features cli::` |
| An `openspec` call is not answered from a `herdr` registration | Register `Ok("openspec answer")` on the `OpenspecCli` side only; call the same vector through the `HerdrCli` handle inside `#[should_panic]`; then register both sides with different responses and assert each handle gets its own | unit | none | `cargo test --all-features cli::` |
| An unregistered invocation panics naming the program and the vector | `#[should_panic(expected = "lst")]` around a call with an unregistered vector, plus an assertion that the message also names the program addressed. The `expected` substring is the misspelled command, not a generic word, so a panic from unrelated setup does not satisfy it | unit | none | `cargo test --all-features cli::` |
| Queued responses are returned in order and the last one repeats | Register two responses for one vector; call four times; assert `["first","second","second","second"]` | unit | none | `cargo test --all-features cli::` |
| A failure can be registered | Register `Err(CliError::Failed { code: Some(1), .. })`; assert the returned value equals it exactly | unit | none | `cargo test --all-features cli::` |
| The fake is usable from another thread | `Arc<dyn OpenspecCli>` over the fake; one call inline, one on a joined thread; assert both responses and that `calls()` holds both | unit | real thread | `cargo test --all-features cli::` |
| Two fakes are independent | Two fakes, same vector, different responses; assert each returns its own and records exactly one call | unit | none | `cargo test --all-features cli::` |
| A trailing newline is trimmed off the prefix | `npm_prefix_from(true, b"/opt/homebrew\n")` equals `Some(PathBuf::from("/opt/homebrew"))` | unit | none — pure function, no process | `cargo test --all-features cli::` |
| Surrounding whitespace is trimmed | `npm_prefix_from(true, b"  /usr/local  \n")` equals `Some("/usr/local")` | unit | none | `cargo test --all-features cli::` |
| Empty or whitespace-only output is no prefix | Table over `b""`, `b"\n"`, `b"   \t \n"`; all `None` | unit | none | `cargo test --all-features cli::` |
| A non-zero exit is no prefix even with output | `npm_prefix_from(false, b"/opt/homebrew\n")` is `None`, so failure beats plausible output | unit | none | `cargo test --all-features cli::` |
| Invalid UTF-8 on stdout is decoded lossily and then trimmed | `npm_prefix_from(true, &[0x2F,0x61,0xFF,0x0A])` equals `Some("/a\u{FFFD}")` — not `None`, since only trimming and emptiness produce nothing | unit | none | `cargo test --all-features cli::` |
| Stderr noise does not reach the prefix | Scratch program prints a scratch prefix to stdout and `zsh: plugin warning` plus a wrong path to stderr; assert the probe yields the stdout prefix and that the result's string holds no stderr token | unit | real scratch filesystem, real `/bin/sh` | `cargo test --all-features cli::` |
| A program that cannot be started is no prefix | Probe an uncreated scratch path; assert `None` and no panic | unit | real scratch filesystem | `cargo test --all-features cli::` |
| A real spawn resolves the npm-prefix binary | Call `resolve::openspec_bin` with no configured path, an environment lookup whose `PATH` names one empty directory, and a fourth-step hook that is a closure calling `cli::npm_prefix_via(&fake_npm)`; with `N/bin/openspec` at mode 0755, assert `FoundBin { path: N/bin/openspec, source: NpmPrefix }`. The hook performs the real spawn rather than returning a literal path | unit | real scratch filesystem, real `/bin/sh`, real process spawn | `cargo test --all-features cli::` |
| A failing real spawn resolves nothing | Same call with a scratch program that prints the prefix but exits 1; assert `found == None` and `problems.is_empty()` | unit | real scratch filesystem, real `/bin/sh`, real process spawn | `cargo test --all-features cli::` |
| The binding delegates to the probe rather than answering for itself | Assert `cli::npm_prefix()` equals `cli::npm_prefix_via(Path::new("npm"))`. Machine-independent, and — unlike an assertion on the value alone — red for a hardcoded `None` body wherever a working `npm` exists | unit | real `npm` when present, real process spawn | `cargo test --all-features cli::` |
| The binding yields either nothing or an absolute path | Assert the result is `None`, or else a path for which `is_absolute()` holds. Names no machine-specific value, so it passes with npm installed, with npm broken, and on `NOTOOLS` | unit | real `npm` when present, real process spawn | `cargo test --all-features cli::` |
| The placeholder is gone and the real binding is named where it is used | The `BINDING` block: (a) `src/resolve.rs` names `cli::npm_prefix`, (b) no `npm_prefix_deferred` survives under `src/`, (c) `npm_prefix()`'s own body delegates to the probe. Guards abort on a missing `src/resolve.rs`, a missing `src/`, or an `npm_prefix()` the extractor cannot find | command check | real `grep`, `sed` | the `BINDING` block |
| The placeholder is gone and the real binding is named where it is used | Negative controls: a copy with the `cli::npm_prefix` reference stripped from `resolve.rs`, a copy still holding `npm_prefix_deferred`, and a copy whose `npm_prefix()` body is `None` — all three must fail, naming which guard fired | command check | real `grep`, `sed`, scratch copies | the `BINDING` block, run three times |
| No file under `src/` outside `src/cli.rs` names a spawn API | `NOSPAWN-GREP` on `src` | command check | real `find`, `grep`, `wc` | the `NOSPAWN-GREP` block |
| A path join is not mistaken for a spawn | `NOSPAWN-GREP` passes on `src` while `src/config.rs:59`, `src/state.rs:34`, `src/state.rs:44` hold `.join("herdr")` — **and** fails on a copy of `src/` with `Command::new("openspec")` planted in a non-`cli` file. Both halves are run; the pass alone proves nothing | command check | real `find`, `grep`, a scratch copy of `src/` | the `NOSPAWN-GREP` block, run twice with `SRC` |
| A vacuous exclusion fails the check | `NOSPAWN-GREP` on a copy with `cli.rs` deleted (guard A), on a copy with `cli.rs` emptied of its spawn (guard B), on a copy holding only `cli.rs` (guard C), and on a copy with the spawn planted at `ui/cli.rs` — all four must exit non-zero, the last proving the exclusion is by path and not by base name | command check | real `find`, `grep`, scratch copies of `src/` | the `NOSPAWN-GREP` block, run four times with `SRC` |
| The suite passes with npm, node, and openspec unresolvable | `NOSPAWN-RUN`, preconditions first and aborting | command check | real cargo, real `PATH`, npm/node/openspec deliberately unresolvable | the `NOSPAWN-RUN` block |
| A run and a probe leave the scratch tree byte-identical | `testutil::snapshot` before and after a successful run, a failing run, an unstartable run, and a full npm probe; assert equality. The snapshot records directories and mtimes, not just files | unit | real scratch filesystem, real `/bin/sh` | `cargo test --all-features cli::` |
| A run and a probe leave the scratch tree byte-identical | Second row for the second piece of evidence: the same four operations, snapshotted around the test process's own working directory with `testutil::shallow_snapshot` (a non-recursive listing of cwd's direct entries — path, is-dir, mtime; never bytes, never descends), so the requirement's "not in the working directory" clause is covered rather than implied by the scratch tree. **Not** the full recursive `snapshot` used for the scratch-tree row above: under `cargo test`, cwd is this crate's own repository root, whose `target/` directory alone holds tens of thousands of build-artifact files; a recursive byte-comparing walk of it was measured during implementation at 7.59s per test run for no more discriminating evidence than the shallow form, since nothing in `cli` computes a path relative to the current directory — any stray write this seam caused would land as a new, removed, or modified top-level entry, which the shallow snapshot catches directly. Verified during implementation to retain genuine discriminating power: a planted stray top-level file was caught and named in the failure | unit | real cwd (read-only), real `/bin/sh` | `cargo test --all-features cli::` |
| The hook is still injected, so the chain stays pure | The existing `resolve` chain tests, unchanged: a closure hook returning a scratch directory resolves `N/bin/openspec` with the npm-prefix source | unit | real scratch filesystem, closure hook | `cargo test --all-features resolve::` |
| The production binding is the real probe | The `BINDING` block's three guards, run with their negative controls. `openspec_bin_from_env`'s existing configured-binary test is **not** evidence for this scenario and is not counted as such: step 1 returns before step 4 is reached, so it passes with any hook at all — including a closure that returns no prefix | command check | real `grep`, `sed`, scratch copies | the `BINDING` block |
| Resolution spawns no process | `NOSPAWN-RUN` for the runtime half; the module-scoped block for the source-text half (`src/resolve.rs` names no `std::process`, `Command`, or `Stdio` at all, comments included) | command check | real cargo, real `PATH`, real `grep` | the `NOSPAWN-RUN` and `NOSPAWN-GREP` blocks |

```sh
# TESTCOUNT — a `cargo test` filter that matches NOTHING exits 0. Verified at planning
# time: `cargo test --all-features this_name_does_not_exist` prints
# "0 passed; ... filtered out" and returns 0. Every group whose VERIFY is a filtered run
# ("cargo test --all-features cli::") therefore passes if the module is renamed, if the
# tests are never written, or if a typo is made in the filter. The gate is a counted
# minimum, taken from the group's own RED task.
#   usage: testcount <filter> <minimum>
testcount() {
  out=$(cargo test --all-features "$1" 2>&1) || { printf '%s\n' "$out" >&2; return 1; }
  n=$(printf '%s\n' "$out" | sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' \
      | awk '{t+=$1} END{print t+0}')
  [ "$n" -ge "$2" ] || { echo "TESTCOUNT FAIL: filter '$1' ran $n tests, expected >= $2" >&2
                         return 1; }
  echo "TESTCOUNT OK: filter '$1' ran $n tests (>= $2)"
}
```

**Where each check stops.** `NOSPAWN-GREP` is evidence about *source text*: it cannot
catch a spawn reached through a dependency, which is why `DEPS` checks the dependency set
separately. `NOSPAWN-RUN` is evidence that the *suite* needs none of the three binaries;
it is not proof that production code never spawns, since a spawn whose error is swallowed
still passes — and after this change `cli` deliberately contains such a spawn.
`OPENSPEC-UNTOUCHED` is evidence about committed content against a base SHA, not about
transient writes. The three together, plus the closure-shaped injection point that
`resolve` has no other way to reach, are the argument. Stated so no later change inherits
a stronger claim than was made.

## Decisions

**One shared `CliError`, not one per trait.** The failure modes are identical — a program
that would not start, and a program that failed — and a caller holding both traits would
otherwise write a conversion for no reason. *Alternative:* an associated `type Error` on
each trait, rejected because it makes `dyn OpenspecCli` awkward and buys nothing.

**`Send + Sync` supertraits, decided now rather than later.** `live-refresh` runs CLI calls
on a worker thread and `agent-polling` polls on another. Adding the bound later would force
every implementation and every fake to be revisited. The cost is that the fake must use a
mutex rather than a `RefCell`, which is a two-line difference. *This strengthens
`SPEC.md`'s trait sketch, which omits both the bound and the error type — logged as a
SPEC.md correction below.*

**Stdout returned verbatim, not trimmed.** `openspec list --json` is fed to a parser that
does not care, and `herdr agent list` likewise; trimming is a caller's decision, and the
one place trimming is contractual — the npm prefix — does it explicitly and is tested for
it. A seam that trimmed would also make "stdout verbatim" untestable. *Alternative:* trim
in `run`, rejected because it is a decision, and decisions belong outside the seam.

**Neither stream is trimmed.** The first draft trimmed stderr while leaving stdout
verbatim, having argued two paragraphs earlier that trimming is a decision and decisions
belong outside the seam. That was inconsistent, and review caught it. Both streams are now
returned as the program produced them, lossily decoded and nothing else; a caller trims
when it renders.

**Lossy UTF-8 decoding, with no `Utf8` error variant.** `SPEC.md` → Degraded states records
that the OpenSpec CLI decodes a tasks file lossily and still reports a count. That is a
precedent for the choice rather than a rule that governs stdout decoding, and the design
says so rather than over-claiming the citation. The substantive reason stands on its own: a
third variant would force every caller to handle a case that, for JSON output, means the
payload was already unusable and the parse will fail anyway. *Alternative:* a `Utf8`
variant, rejected.

**The real implementations are tested, not written off as residue.** `SPEC.md` calls them
"untestable residue", and they are untestable only if you insist on never spawning
anything. Pointing them at scratch `#!/bin/sh` programs proves the four properties that
actually matter — stdout only, stderr excluded, no argument added, non-zero exit is an
error — and keeps them out of the uncovered column. This does **not** breach
`IMPLEMENTATION-ORDER`'s "no test ever spawns a real process": that principle is about the
`openspec` and `herdr` binaries in *later* changes' tests, and `tests/cli.rs` already
spawns this crate's own binary under the same reading. Every spawn here names an absolute
scratch path, so `NOSPAWN-RUN` stays green. *Alternative:* a third `Runner` trait so `cli`
itself could be tested with a fake — rejected, it moves the residue one layer down, adds a
trait `SPEC.md` does not name, and leaves the real spawn just as unproven.

**`npm_prefix_from(success, stdout)` as a pure function, with a one-line spawner.** Taking
only the exit success and the stdout bytes is the *structural* proof that stderr cannot
influence the result — stronger than a test, because there is no parameter to read it
from. Every trimming and emptiness rule is then a table test with no process at all.

**`npm_prefix_via(program)` beside `npm_prefix()`.** The end-to-end scenario must drive a
real spawn deterministically on any machine, including one with no npm. Parameterizing the
program lets a scratch script stand in for `npm` without touching `PATH` —
`std::env::set_var` is `unsafe` in edition 2024 and races parallel tests, which
`AGENTS.md` forbids outright. `npm_prefix()` stays a one-line binding, exactly like
`config::env_lookup`.

**The fake panics on an unregistered vector.** Returning `Ok("")` would let a caller's test
pass while the caller spawned the wrong command — precisely the class of defect the seam
exists to prevent, and invisible because every consumer of this seam is required to
degrade rather than fail. A panic names the vector and fails the test that caused it.
*Alternative:* return a distinctive `Err`, rejected because a never-fail-closed caller
swallows it by design.

**The fake keys responses by the pair (program addressed, argument vector), with a
per-pair queue whose last entry repeats.** `agent-polling` calls the same command every
second and wants one registration; `live-refresh` wants a refresh to return changed data
and registers two. A single global FIFO would make every test order-dependent.
*Alternative:* prefix matching, rejected — it would let `["list"]` answer
`["list", "--json"]`, which is the wrong command.

Keying on the **pair** rather than the vector alone is a repair review forced. One type
implements both traits, so a vector-only key would answer a caller that reached for the
`HerdrCli` handle out of an `OpenspecCli` registration — precisely "a caller's test passes
while the caller spawned the wrong command", and invisible, because the panic cannot fire
when the vector *is* registered. Two consequences the implementer must expect: `calls()`
records pairs, not bare vectors; and with both traits in scope a bare `fake.run(..)` is
E0034-ambiguous, so the tests disambiguate with
`OpenspecCli::run(&fake, ..)` / `HerdrCli::run(&fake, ..)`.
*Alternative:* two separate fake types, rejected — the recording and queueing logic is
identical and would be duplicated, and a test needing both would still hold two values.

**The fake is `#[cfg(test)] pub(crate)`.** It follows `testutil`'s existing placement and
contributes nothing to the release binary. **Correction, found in Change Review:** an
earlier draft of this entry also claimed it "contributes nothing to the coverage
denominator" — that is false. `cargo llvm-cov` measures the test binary, which compiles
in every `#[cfg(test)]` item including `mod tests` itself, and `cli.rs`'s reported line
count (567 per `cargo llvm-cov`, against roughly 380 lines outside `mod tests`) confirms
test code is counted, not excluded. `#[cfg(test)]` placement keeps the fake out of the
**release binary** only; it is not a coverage-denominator mechanism, and this change does
not rely on it being one — every trait, error variant, and probe function this change adds
is exercised directly, so the 80% floor holds regardless of how test scaffolding is
counted.

**`RealHerdrCli` takes a program path and defaults to the bare name `herdr`.** `openspec`
has a four-step resolution chain because `plugin-config` and `repo-resolution` built one;
`herdr` has none, and inventing one here would be unplanned scope. Failing to start
`herdr` is already the documented "Herdr socket unreachable" degraded state. A later change
that discovers Herdr injects its own binary path into the plugin environment can pass it
to the same constructor with no API change.

**Stdin is closed.** A pane process has no keyboard attached to a subprocess; a program
that reads stdin would otherwise block the pane forever. The alternative — inheriting the
pane's stdin — would let a stray `openspec` prompt hang the dashboard.

**The hand-over is repointed first and deleted second.** Task group 6 changes
`npm_prefix_deferred`'s *body* to call `cli::npm_prefix()` while keeping its name and
signature, runs the pinning test, and records the outcome verbatim — then deletes both.
Deleting the function outright would turn the hand-over into a compile error, which is a
red of a sort but not the observation `repo-resolution` asked for. See the risk below
about *which* `PATH` the observation must run on.

**The no-spawn check is judged on output emptiness, not on an exit code, and carries three
guards.** Rationale and the five verified red conditions are in the `NOSPAWN-GREP` block
above. The short form: `!` on a `grep` pipeline turns exit 2 (bad file) into a pass; a
`find | xargs` with no input can read stdin on GNU; an exclusion is worthless if the
excluded file might not exist or might not spawn; and an exclusion written by base name
silently exempts a future `src/ui/cli.rs`, which review demonstrated. All are guarded, and
all five failure modes were demonstrated at planning time against real copies of `src/`.

**The hand-over gets its own check, separate from the no-spawn one.** `NOSPAWN-GREP` says
nothing about what a function returns, the chain tests inject their own hooks, and "no
prefix" is a legitimate answer — so `pub fn npm_prefix() -> Option<PathBuf> { None }`, the
exact body being deleted, would keep every other check in this change green with step 4
still dead. `BINDING` is the one thing that fails on it, and its guard (c) is structural
because no assertion on the *value* `npm_prefix()` returns can be both machine-independent
and red for that body. The behavioural half is group 6's delegation test.

**The working-directory half of "running a program writes nothing" is proven with a
shallow, non-recursive snapshot, not the full recursive one used for the scratch
tree.** Found during implementation, not at planning time: a full recursive
`testutil::snapshot` around `std::env::current_dir()` is correct but, under `cargo
test`, cwd is this crate's own repository root — `target/` alone holds tens of
thousands of build-artifact files, and a byte-comparing recursive walk of it measured
at 7.59s for this one test, on every future run. `testutil::shallow_snapshot` — added
alongside it — lists only cwd's direct entries (path, is-dir, mtime; never bytes,
never descends), which is exactly as discriminating for this seam: nothing in `cli`
computes a path relative to the current directory, so a stray write would surface as a
new, removed, or modified top-level entry, which the shallow form catches directly.
Verified by planting a stray top-level file and confirming the test named it, then
removing the plant and confirming green. *Alternative:* changing the test process's
own cwd to a scratch directory for the test's duration, rejected — `cargo test` runs
tests in parallel threads of one process, and `std::env::set_current_dir` is
process-global, so it would race every other concurrently running test exactly the way
`AGENTS.md` forbids `std::env::set_var` from doing.

### SPEC.md corrections this change carries

`AGENTS.md`: *"Where prose and `SPEC.md` disagree, `SPEC.md` wins. When a change reveals
that the spec is wrong, update the spec as part of that change."* Four corrections, each a
task in group 10 with a before/after recorded in `planning-review.md`:

1. **Architecture → The subprocess seam, the trait snippet.** Reads
   `trait OpenspecCli { fn run(&self, args: &[&str]) -> Result<String>; }` — a bare
   `Result` with no error type and no thread bound. Replace with the real signature,
   including `Send + Sync` and `CliError`.
2. **Architecture → The subprocess seam, the residue sentence.** "the untestable residue
   is two thin wrappers, the `npm prefix -g` binding, and `main`" — after this change the
   wrappers and the probe are covered, and the residue is the one-line `npm_prefix()`
   program binding plus `main`. Rewrite to say so, and note that the wrappers are proven
   against scratch programs.
3. **Data layer → Resolution chain, the "Step 4 is unwired until `subprocess-seam` lands"
   paragraph.** Entirely superseded. Rewrite to describe the landed state: the hook stays
   injected, its production binding is `cli::npm_prefix`, and the stdout-only/trimmed rule
   stays as the binding's contract rather than as an instruction to a future change.
4. **Testing and quality gates → Unit-tested modules.** Add `cli`: the traits' contract and
   the npm probe, tested against scratch `#!/bin/sh` programs rather than the real
   `openspec`, `herdr`, or `npm`.
5. **Testing and quality gates → Unit-tested modules, the lead-in sentence.** It reads
   "Each is a pure transformation, tested without a TUI or a subprocess:" — which
   correction 4's new entry contradicts directly, one line below it. Review caught this;
   adding the entry without narrowing the lead-in would ship a self-contradicting
   paragraph. Narrow it to say that these modules are pure transformations tested without
   a TUI, and that `cli` is the one exception, tested against scratch programs because a
   spawn is the thing it exists to perform.

Plus `AGENTS.md` → Current repo state, whose sentence "the binary chain's fourth probe step
ships as an injected hook that always returns nothing until `subprocess-seam` wires it"
becomes false the moment this lands, and `AGENTS.md` → Architecture rules, whose spawn
bullet describes the seam in the future tense.

**One roadmap correction is needed**, and it is not the row. The row was re-read at
planning time and describes exactly this change, including the end-to-end obligation. But
`openspec/IMPLEMENTATION-ORDER.md` → Ordering principles says the seam lands "before the
first change that shells out, **so no test ever spawns a real process**" — and this change
spawns roughly twenty scratch `#!/bin/sh` programs in its own tests. The principle's intent
is sound and `tests/cli.rs` is an existing precedent for the narrower reading (it spawns
this crate's own binary), but as written the sentence is now false. It is narrowed to say
what it means: no test spawns the `openspec` or `herdr` binaries. Recorded in
`planning-review.md` with the before/after, like the SPEC.md corrections.

## Risks / Trade-offs

**The pinning test may not go red, because `npm` is not on the plain `PATH`.** The
reference machine installs `npm` under nvm, deliberately off the default `PATH` — the same
fact that makes step 3 and step 4 exist at all. A bare `cargo test` after a *correct*
repointing would therefore still see `None` and stay **green**, and the hand-over signal
would be silently lost. → Task 6.2 runs the observation through the `NPM-PATH` block, whose
precondition is **measured, not assumed**: it runs `$NPMBIN/npm prefix -g` itself and
aborts unless that exits 0 and prints an absolute path. The plain-`PATH` run is recorded as
informational only. If the test is green **with** the measured precondition satisfied, the
binding was not actually repointed — that is an investigation, never an adjustment of the
test.

An earlier draft of this risk got the mechanism wrong and review corrected it: it claimed
`npm` is not on the `PATH` a plain shell inherits, which is false here
(`/bin/sh -c 'command -v npm'` → `/opt/homebrew/bin/npm`). The plain-`PATH` run stays green
today only because that Homebrew node is broken, which a `brew reinstall node` would undo.
Measuring the precondition rather than asserting a `PATH` fact is what makes the red
survive that.

**Spawning in unit tests could reintroduce environment dependence.** → Every spawn names an
absolute scratch path; the sole exception is `npm_prefix()`'s smoke test, whose assertion
holds whether or not `npm` exists. `NOSPAWN-RUN` proves the whole suite survives with npm,
node, and openspec unresolvable.

**`#!/bin/sh` scratch programs assume a POSIX shell at that path.** → Both supported
platforms guarantee it (`platforms = ["macos", "linux"]`; Windows is a PRD non-goal), and
CI runs `ubuntu-latest` and `macos-latest`. If a script ever fails to start, the test fails
loudly as `NotStarted` rather than passing vacuously.

**A test that spawns is slower than one that does not.** → Roughly a millisecond per
`/bin/sh` start, over about twenty tests. The stdin-blocking scenario is the only one with
a deadline, and it polls to it rather than sleeping, so it costs nothing when passing.

**`Stdio` is in the forbidden-token list but `cli` may legitimately need it.** →
`NOSPAWN-GREP` excludes `src/cli.rs` from the tree-wide half, and guard B only requires
`cli.rs` to name `Command`. The module-scoped half never covered `cli.rs`.

**Coverage could dip if the real implementations end up partly unreachable.** → Task 11.5
re-measures and compares against the baseline recorded in 1.1. The floor is 80% and is
never lowered, waived, or given an exclusion; if a line is genuinely unreachable, the
answer is to shrink it to one line, not to exempt it.

**The `src/cli.rs` / `tests/cli.rs` name collision could confuse a later reader.** → Both
files get a module doc comment naming the other and saying which is which.

## Migration Plan

No deploy, no data, no rollout. The change is a library addition plus one internal
rebinding, landing on `main` as a sequence of per-task-group Conventional Commits.

Order inside the change matters and is fixed by the task groups: `cli`'s error type and
spawn helper first (2), the real implementations (3), the fake (4), the npm probe (5), then
the hand-over (6), then the end-to-end join (7). Group 6 is the only one that touches
`src/resolve.rs`, and it repoints before it deletes so the red is observable.

Rollback is `git revert` of the group commits in reverse order; nothing outside the crate
observes the change. No branch is created — `AGENTS.md` permits committing straight to
`main` unless the change needs isolation, and this one does not. Nothing is pushed and no
PR is opened.

## Visual Design

Not applicable. This change builds no user-facing view and no email template — it adds a
library module with no rendering surface. No design source exists or is needed.

## Open Questions

None. Two things were resolved during planning rather than left open: whether the real
implementations should be tested by spawning (yes — see Decisions), and which `PATH` the
hand-over red must be observed on (the nvm one — see Risks). One item is deliberately
deferred rather than open: whether Herdr injects a variable naming its own binary path is
`agent-polling`'s question, and answering it changes no API here.
