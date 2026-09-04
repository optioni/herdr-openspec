## Context

`repo-foundation` left a crate with no dependencies, a `ui` placeholder, and nothing
that reads anything. `plugin-config` is the third Phase 1 row and the first change with
a data layer: `SPEC.md` → Data layer → Resolution chain names `openspec_bin`,
`agent_kind`, and `archived_count` as plugin configuration, and `SPEC.md` → Herdr
integration → Attributing an agent names a plugin-local state file recording the mapping
from a derived agent name back to its change.

**The one hard problem.** `SPEC.md` says the configuration directory is "the directory
reported by `herdr plugin config-dir herdr-openspec`". Spawning `herdr` is forbidden
outside `cli`, and `cli` does not exist until `subprocess-seam` in Phase 3 — three
phases after `repo-resolution` needs configuration. Taken literally, the roadmap ordering
is impossible.

It is impossible only because `SPEC.md` describes the *user-facing* way to find the
directory, not the way a plugin process learns it. Investigation of the installed Herdr
0.8.2 settled it:

- The binary carries the literals `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_CONFIG_DIR`,
  `HERDR_PLUGIN_STATE_DIR`, `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_ENTRYPOINT_ID`, and
  `HERDR_PLUGIN_CONTEXT_JSON`, adjacent in the same string table.
- Three installed third-party plugins already depend on them. `herdr-navigator`'s
  `src/paths.rs` reads `HERDR_PLUGIN_CONFIG_DIR` and `HERDR_PLUGIN_STATE_DIR` with XDG
  fallbacks; `worktrunk`'s `config.sh` reads `$HERDR_PLUGIN_CONFIG_DIR/config.toml` and
  returns empty when the variable is unset; `herdr-file-viewer` documents
  `$HERDR_PLUGIN_CONFIG_DIR` as "herdr-provided" in `ARCHITECTURE.md`, `CONTEXT.md`, and
  its example config.
- Confirmed empirically on this machine, the way `repo-foundation` confirmed
  `plugin link`, **for both entrypoint kinds**. A throwaway plugin whose only pane
  dumped `env` was linked, opened with `herdr plugin pane open --no-focus`, and
  unlinked; a second throwaway plugin declaring only an `[[actions]]` entry was invoked
  with `herdr plugin action invoke <id> --plugin <id>`. Both processes received
  `HERDR_PLUGIN_CONFIG_DIR=~/.config/herdr/plugins/config/<id>` — character-identical to
  what `herdr plugin config-dir <id>` prints — and
  `HERDR_PLUGIN_STATE_DIR=~/.local/state/herdr/plugins/<id>`, alongside `HERDR_ENV=1`,
  `HERDR_PLUGIN_ROOT`, `HERDR_PLUGIN_ID`, `HERDR_SOCKET_PATH`, and the workspace, tab,
  and pane ids. The pane process additionally carried `HERDR_PLUGIN_ENTRYPOINT_ID`; the
  action process carried `HERDR_PLUGIN_ACTION_ID` instead. Neither is needed here.

So the configuration directory arrives in the environment of every process Herdr starts
for a plugin, however it is entered. No subprocess is needed at any tier,
`subprocess-seam` is not a dependency, and `SPEC.md` is corrected rather than worked
around.

## Goals / Non-Goals

**Goals:**

- Resolve the plugin's configuration and state directories with no process spawn,
  inside a Herdr-started process and outside one.
- Turn `config.toml` into a `Config` value that is total: every input, including
  garbage, produces a usable value and never an error.
- Make each fallback *observable*, so "the file was absent" and "the file was garbage"
  are distinguishable in a test and, later, on screen.
- Give `agent-launch` a pure, deterministic change-name-to-agent-name function and a
  durable place to record the mapping it loses to normalisation and truncation.
- Add exactly one dependency, argued, with default features off.

**Non-Goals:**

- The `openspec` binary probe chain (`repo-resolution`), pane splitting and
  `herdr agent start` (`agent-launch`), any view (`ui` lands in Phase 4), any subprocess
  (`subprocess-seam`), and any re-read on change (`live-refresh`).
- Validating `agent_kind` against Herdr's list, or `openspec_bin` against the
  filesystem. Both would need a collaborator this change refuses to take.

## Boundaries

Two new modules, both on the pure side of the subprocess seam, neither in `SPEC.md`'s
module map today:

| Module | Responsibility | Pattern it follows |
|---|---|---|
| `config` | Resolve the configuration directory from an environment lookup; read and interpret `config.toml`; expand `openspec_bin` | `resolve` — a pure transformation over injected inputs, no spawn |
| `state` | Resolve the state directory; derive an agent name; read and atomically record `agent-names.toml` | `changes` — filesystem reads plus a pure core; the only module in the design that writes, and only outside any repository |

Collaborators touched: the process environment (injected, never mutated), the
filesystem (real, always under a scratch directory in tests), and the `toml` crate. Not
touched: the `openspec` binary, the `herdr` binary, the Herdr socket, the terminal,
`ratatui`, and any repository's `openspec/` tree.

**No process spawn is added outside `cli`.** No process spawn is added at all: after
this change `src/` still contains no `Command` and no `Stdio`, and `cli` still does not
exist. Note that the tree-wide check cannot simply forbid `std::process` — `src/main.rs`
already calls `std::process::exit`, which is an exit and not a spawn — so the tree-wide
pattern names the spawn types and the stricter "no process API at all" pattern is scoped
to `src/config.rs` and `src/state.rs`. That scoping is also what keeps the spec scenario
true after `subprocess-seam` creates `cli` precisely to hold `std::process::Command`.
**No view is added**, so the rule that views perform no I/O is untouched.

**The `Change` type is neither introduced nor altered here.** `from_files` and
`from_cli` do not exist yet; `changes-from-files` will consume `Config::archived_count`
as a plain number and gains no field from this change.

## Contracts

The consumers are all future changes in this repository. Nothing outside it depends on
anything here.

```rust
// config
pub struct Config { openspec_bin: Option<PathBuf>, agent_kind: String,
                    archived_count: usize, problems: Vec<String> }
pub fn config_dir(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf>;
pub fn load(dir: Option<&Path>, env: &dyn Fn(&str) -> Option<String>) -> Config;
pub fn env_lookup() -> impl Fn(&str) -> Option<String>;   // the one call to std::env::var
pub fn load_from_env() -> Config;                          // load(config_dir(&e), &e)

// state
pub struct Mapping { names: BTreeMap<String, String>, problems: Vec<String> }
pub fn state_dir(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf>;
pub fn agent_name(change: &str) -> String;     // pure, total, deterministic
pub fn read(dir: Option<&Path>) -> Mapping;
pub fn record(dir: Option<&Path>, agent: &str, change: &str) -> std::io::Result<()>;
```

- **Error surface.** `config::load` and `state::read` are infallible by construction:
  every failure becomes a default plus a string in `problems`. Only `state::record`
  returns `Result`, because a failed write is a fact the caller must be able to notice;
  its callers treat the error as degraded and continue.
- **`record`'s order of operations is part of the contract.** The `agent == change`
  short-circuit runs *before* the directory is consulted, so `record(None, "x", "x")` is
  `Ok(())` while `record(None, "c-2fa-support", "2fa-support")` is `Err`. Recording an
  agent name already bound to a different change replaces the binding and is not a
  problem; the most recent launch is the live one.
- **`env_lookup` is separated from `load_from_env` on purpose.** `load_from_env` is a
  one-line composition with nothing left to assert; `env_lookup` is the single place the
  crate calls `std::env::var`, and it *can* be asserted against — a variable known to be
  present agrees with `std::env::var`, and a name nothing sets is `None`. Collapsing them
  would leave the crate's only contact with the real environment covered by a test that
  compares a value to itself.
- **Compatibility.** Purely additive — no existing signature changes, `src/main.rs` is
  untouched, and the `ui` behaviour `plugin-build` specifies is unaffected.
- **Not BREAKING.** `openspec/config.yaml` requires a **BREAKING** mark for a change to
  the plugin config format. This change *introduces* that format; there is no prior
  format and no user file to migrate. `agent-launch` extending it later would be.
- **Consumers named:** `repo-resolution` (`openspec_bin`), `changes-from-files`
  (`archived_count`), `agent-launch` (`agent_kind`, `agent_name`, `record`),
  `agent-attribution` (`read`), `degraded-states` (`problems`).

## Persistence and Rollout

- **Migration:** none. No prior on-disk format exists.
- **Backfill:** none. An absent `agent-names.toml` is a legitimate empty mapping.
- **Seeding:** none. `config.toml` is never created by the plugin; the README documents
  the keys and the user writes the file.
- **Cache invalidation:** none. Configuration is read once per process; there is no
  cache and nothing watches the file. `live-refresh` may revisit that and does not have
  to.
- **Index rebuild:** none.
- **Authorization:** none — a single-user local plugin. The one privilege question is
  *where* it writes: only inside `HERDR_PLUGIN_STATE_DIR`, never in the user-owned
  configuration directory and never inside a repository.
- **Observability:** the `problems` vectors are the whole of it. Nothing logs, and
  nothing prints — there is no view yet. `degraded-states` renders them, and this change
  adds the two rows to `SPEC.md`'s degraded-states table so it inherits them rather than
  inventing them.
- **Deployment:** none beyond the normal build. `Cargo.lock` gains entries, so the first
  CI run after this change repopulates `Swatinem/rust-cache`.

## Test Boundaries

This change takes no automated acceptance tier (see Test Strategy), so the first column
names each collaborator's treatment in the deterministic command checks and the one live
Herdr check that stand in for one.

| Dependency | In acceptance test (deterministic command and live checks) | In unit tests |
|---|---|---|
| Process environment | real — the live Herdr probe reads the actual variables Herdr injects into a pane process and an action process | **replaced**: every function takes an environment lookup closure over a fixture map. `std::env::set_var` is never called — it is `unsafe` in edition 2024 and would race parallel tests |
| `std::env::var` | real — reached only through `config::env_lookup`, the single binding | called only inside `env_lookup`, which two assertions pin: a variable guaranteed present (`PATH`) agrees with `std::env::var`, and a name nothing sets is `None`. `std::env::temp_dir()` also reads `TMPDIR` in the scratch-directory helper; it reads and never mutates |
| Filesystem — configuration directory | real — the live probe compares the injected path with `herdr plugin config-dir` | real, but always a fresh scratch directory under `std::env::temp_dir()`, created and removed by the test. Never `$HOME`, never a repository |
| Filesystem — state directory | real, same scratch technique | real, scratch. This is the only directory any code in this change writes to |
| Repository `openspec/` tree | read-only; `openspec validate --strict` is the final gate | present as a fixture in one test purely to assert it is **not** modified |
| `toml` crate | real | real. Not replaced and not wrapped: it is a parser, not a collaborator with I/O, and faking it would test the fake |
| `herdr` binary and socket | real, and manual — two throwaway probe plugins, linked, exercised, and unlinked. Their ids are never `herdr-openspec` | **not touched.** No production code path invokes `herdr`; a search of `src/` for a process API and for `"herdr"` as a program name is itself a check |
| User's real Herdr directories (`~/.config/herdr/plugins/config/<id>`, `~/.local/state/herdr/plugins/<id>`) | real, and **destructive** — Herdr creates them for each throwaway probe id and the check removes them. Removal is by the exact path `herdr plugin config-dir <id>` prints, never by a glob, and only for a probe id | not touched |
| `openspec` binary | not invoked. `openspec_bin` is a string this change expands and returns; it is never executed, stat-ed, or resolved | not touched |
| `cargo`, `cargo metadata`, `cargo tree`, `cargo llvm-cov` | real — the dependency-set checks and the coverage gate | real, as the test runner only |
| POSIX shell userland (`sh`, `grep`, `env`, `cp`, `ln`, `mktemp`, `tr`) | real. macOS/BSD is the reference platform, so no GNU-only form appears in any check — no `stat -c`, no `sed -i` without an argument, no `grep -P`, and no `paste -sd:` without a file operand, which is a usage error on BSD and silently yields an empty string | not touched; modification times are compared in Rust through `std::fs::Metadata`, never in shell |
| Terminal, `ratatui`, `crossterm` | not used — this change adds no view | not touched |
| Clock and randomness | not used. The atomic write's temporary file is `agent-names.toml.tmp-<pid>-<counter>` from an atomic counter, and the test scratch directory is `herdr-openspec-test-<pid>-<counter>`, so a test can predict both names and assert their absence | not touched |
| `git` working tree | not used as a restore mechanism anywhere; the one check that damages a tracked file copies it aside and restores from the copy | not touched |
| `python3` | real — parses `cargo metadata` JSON in the dependency-set checks. `tomllib` is not needed, so the 3.11 floor `repo-foundation` hit does not apply | not touched |

## Test Strategy

Tiers, fastest first:

- **unit** — `cargo test --all-features`, functions in `src/config.rs` and
  `src/state.rs`. Pure functions take fixtures; filesystem functions take a scratch
  directory. 47 of the 57 scenarios live here.
- **command check** — a shell command run once, its exit status and output inspected,
  recorded as a task. Five scenarios: the four `plugin-build` facts about `Cargo.toml`
  and the resolver, and the spawn check, which is a fact about the compiled tree rather
  than about running code.
- **existing test** — `tests/ci_workflow.rs`, already in the repository. The five
  `ci-workflow` scenarios are carried unchanged by this change's delta, which amends only
  the requirement's rationale; they need no new work, and the row records that rather
  than leaving them unaccounted for.
- **manual (live Herdr)** — the two throwaway probe plugins. One check, and the
  load-bearing one: it is the only evidence that the environment variables this whole
  design rests on are really injected.

**This change does not take the outer-loop acceptance test.** Its outermost surface is a
library API: `src/main.rs` is unchanged, the binary's observable behaviour is still the
`ui` banner, and nothing consumes `Config` until `repo-resolution`. An acceptance test
would have to invent an entry point that ships to nobody. The unit tier already drives
each function end to end against a real filesystem, which is every collaborator these
functions have; the one collaborator it cannot reach — Herdr's injected environment — is
covered by the manual check rather than by a fake pretending to be it.

Shorthand used below. Every command is written for BSD userland.

```sh
# A scratch directory per check; removed at the end of the task.
T=$(mktemp -d)

# A PATH that keeps the toolchain and drops only the directory holding `herdr`.
# `env PATH=/usr/bin:/bin cargo ...` cannot be used: env execs through the NEW PATH,
# so cargo (in ~/.cargo/bin) is not found and the check fails for the wrong reason —
# the same defect the repo-foundation review repaired.
HERDR_DIR=$(dirname "$(command -v herdr)")
NOHERDR=$(printf '%s' "$PATH" | tr ':' '\n' | grep -vxF "$HERDR_DIR" | tr '\n' ':')

# Direct, normal-kind dependencies of the package, asserted — never printed.
DEPS='import json,sys
p = json.load(sys.stdin)["packages"][0]
normal = [d for d in p["dependencies"] if d.get("kind") is None]
assert sorted(d["name"] for d in normal) == ["toml"], normal
t = normal[0]
assert t["uses_default_features"] is False, t
assert sorted(t["features"]) == ["display", "parse", "serde", "std"], t'

# Exactly one bin target, named after the crate.
BIN='import json,sys
t = [x for p in json.load(sys.stdin)["packages"] for x in p["targets"] if "bin" in x["kind"]]
assert [x["name"] for x in t] == ["herdr-openspec"], t'
```

The three checks whose commands contain a pipe or a regex alternation are written here
rather than inside a table cell, because `|` cannot appear unescaped in a Markdown table
and an escaped `\|` inside an ERE means a *literal* pipe — `grep -E 'a\|b'` matches the
string `a|b` and finds nothing, so the check would pass against the very code it is
meant to catch. The matrix rows below name these by label.

```sh
# SPAWN — no spawn API anywhere, and no process API at all in this change's modules.
# `std::process` alone is NOT usable over the whole tree: src/main.rs legitimately calls
# std::process::exit, which is not a spawn. Verified: the module-scoped form stays clean
# against a doc comment naming `herdr plugin config-dir`, and the tree-wide form fires on
# a `use std::process::Command;`.
! grep -rnE 'process::Command|Command::new|Stdio' src/
! grep -nE 'std::process|Command|Stdio' src/config.rs src/state.rs
! grep -rn '"herdr"' src/
! grep -rn '"openspec"' src/

# NOHERDR-RUN — the suite passes with herdr unresolvable and the toolchain intact.
! env PATH="$NOHERDR" command -v herdr
env PATH="$NOHERDR" cargo test --all-features

# TREE — the normal build graph, exactly, and proc-macro-free.
cargo tree -e normal --prefix none | awk '{print $1}' | sort -u > "$T/tree.txt"
printf '%s\n' herdr-openspec serde_core serde_spanned toml toml_datetime \
  toml_parser toml_writer winnow | sort -u | diff - "$T/tree.txt"
! cargo tree -e normal | grep -E 'syn|quote|proc-macro2|serde_derive'

# METADATA — the two cargo metadata filters, piped.
cargo metadata --no-deps --format-version 1 | python3 -c "$DEPS"
cargo metadata --no-deps --format-version 1 | python3 -c "$BIN"
```

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| Herdr supplies the directory | Fixture lookup with `HERDR_PLUGIN_CONFIG_DIR` and `HOME` set; assert the former wins; repeat with `HOME` absent and assert an identical result | unit | environment replaced by a closure | `cargo test --all-features config::` |
| Run outside a Herdr pane | Fixture lookup with only `HOME`; assert `$HOME/.config/herdr/plugins/config/herdr-openspec` | unit | environment replaced | `cargo test --all-features config::` |
| An empty or blank environment variable is not a directory | Fixture lookup with the variable set to `""`, then to `"   "`; assert the `HOME` fallback both times, not an empty, relative, or three-space path | unit | environment replaced | `cargo test --all-features config::` |
| `XDG_CONFIG_HOME` is deliberately ignored | Fixture lookup with `XDG_CONFIG_HOME` and `HOME` set; assert the `HOME` path, and assert `state_dir` on the same lookup *does* honour `XDG_STATE_HOME`, so the asymmetry is pinned by a test rather than by prose | unit | environment replaced | `cargo test --all-features config::` |
| Neither variable is available | Empty fixture lookup; assert `None`, then assert `load(None, …)` equals `Config::default()` with an empty `problems` | unit | environment replaced | `cargo test --all-features config::` |
| Resolution spawns nothing | Run the suite on a `PATH` that keeps the toolchain and drops `herdr`'s directory, having first asserted `herdr` is genuinely unresolvable on it; then search for spawn APIs and for a program-name literal. The patterns match invocations, not the word `herdr`, so prose naming `herdr plugin config-dir` — which this change deliberately writes into `SPEC.md`, `README.md`, and the modules' own doc comments — cannot turn the check red | command check | herdr deliberately unresolvable | the `NOHERDR-RUN` and `SPAWN` blocks above |
| Every key is set | Write all three keys to a scratch `config.toml`, load, assert all three and an empty `problems` | unit | real scratch filesystem | `cargo test --all-features config::` |
| The file does not exist | Scratch directory with no file; assert defaults and empty `problems` | unit | real scratch filesystem | `cargo test --all-features config::` |
| The directory does not exist | Point at a scratch path never created; assert defaults, empty `problems`, and that the path still does not exist | unit | real scratch filesystem | `cargo test --all-features config::` |
| An empty file is not a malformed file | Zero-byte `config.toml`; assert defaults **and** `problems.is_empty()`, which is what separates this from the malformed case | unit | real scratch filesystem | `cargo test --all-features config::` |
| Only one key is set | `archived_count = 0` alone; assert 0 is honoured, the others default, `problems` empty | unit | real scratch filesystem | `cargo test --all-features config::` |
| Unrecognised keys are ignored | Known key plus an unknown key and an unknown table; assert the known value and empty `problems` | unit | real scratch filesystem | `cargo test --all-features config::` |
| The file is not valid TOML | `agent_kind = = "codex"`; assert full defaults and exactly one problem containing `config.toml` | unit | real scratch filesystem | `cargo test --all-features config::` |
| One key has the wrong type, the rest survive | Three keys, two of the wrong type; assert two defaults, one honoured value, and two problems naming the two keys | unit | real scratch filesystem | `cargo test --all-features config::` |
| A negative count is not a count | `archived_count = -1`; assert 5 and one problem naming the key | unit | real scratch filesystem | `cargo test --all-features config::` |
| The file cannot be read | Create `config.toml` as a *directory*; assert full defaults, one problem naming it, and no panic | unit | real scratch filesystem | `cargo test --all-features config::` |
| A tilde path is expanded | `~/…` with `HOME` in the fixture lookup; assert the joined absolute path | unit | real scratch filesystem, environment replaced | `cargo test --all-features config::` |
| A `$HOME` path is expanded and a bare tilde is the home directory | Two loads, `$HOME/bin/openspec` and `~`; assert both expansions | unit | real scratch filesystem, environment replaced | `cargo test --all-features config::` |
| An unexpandable value is returned verbatim | `~/bin/openspec` with `HOME` absent, and `~otheruser/bin/openspec` with `HOME` set; assert both come back unchanged | unit | real scratch filesystem, environment replaced | `cargo test --all-features config::` |
| An empty value is an absent value | `openspec_bin = "   "`; assert `None` and empty `problems` | unit | real scratch filesystem | `cargo test --all-features config::` |
| A path that does not exist is still returned | `/nowhere/openspec`; assert it is returned and that the path was never created | unit | real scratch filesystem | `cargo test --all-features config::` |
| A configuration read leaves the tree byte-identical | Snapshot the listing, bytes, and mtimes of a scratch directory holding `config.toml` and one unrelated file; load twice; assert the snapshot is unchanged and no extra entry appeared | unit | real scratch filesystem | `cargo test --all-features config::` |
| A missing configuration directory stays missing | Load from an uncreated scratch path; assert `!path.exists()` afterwards | unit | real scratch filesystem | `cargo test --all-features config::` |
| Herdr supplies the state directory | Fixture lookup with all three variables; assert `HERDR_PLUGIN_STATE_DIR` wins and differs from `config_dir` on the same lookup | unit | environment replaced | `cargo test --all-features state::` |
| `XDG_STATE_HOME` is honoured before `HOME` | Fixture lookup without the Herdr variable; assert `$XDG_STATE_HOME/herdr/plugins/herdr-openspec` | unit | environment replaced | `cargo test --all-features state::` |
| Run outside a Herdr pane with no XDG setting | Fixture lookup with only `HOME`, then with `XDG_STATE_HOME=""` and with `"   "`; assert `$HOME/.local/state/herdr/plugins/herdr-openspec` all three times | unit | environment replaced | `cargo test --all-features state::` |
| No directory can be resolved at all | Empty fixture lookup; assert `None`, `read(None)` empty with no problems, `record(None, "c-2fa-support", "2fa-support")` is `Err`, and `record(None, "x", "x")` is `Ok` — the short-circuit runs before the directory is needed | unit | environment replaced | `cargo test --all-features state::` |
| A short kebab-case change name is unchanged | Derive for `add-token-refresh`; assert equality and a regex match | unit | none — pure | `cargo test --all-features state::` |
| A name of exactly 32 characters is not truncated | Derive for `add-really-long-change-name-that` (32) and `add-really-long-change-name-thatx` (33); assert the first is unchanged and the second is exactly `add-really-long-change-name-mmky` | unit | none — pure | `cargo test --all-features state::` |
| A long change name is truncated with a suffix from the whole name | Derive for the alpha/beta pair; assert the two exact literals `add-really-long-change-name-8jqt` and `add-really-long-change-name-alft`, both ≤ 32, both matching the regex, differing, and idempotent | unit | none — pure | `cargo test --all-features state::` |
| A truncated name may be shorter than 32 characters | Derive for `abcdefghijklmnopqrstuvwxyz-abcdefg`; assert exactly `abcdefghijklmnopqrstuvwxyz-lhun`, 31 characters — the case that makes "at most 32" the rule rather than "exactly 32" | unit | none — pure | `cargo test --all-features state::` |
| Illegal characters and casing are normalised | Derive for `Add Token Refresh (v2)!`; assert `add-token-refresh-v2`, no leading or trailing `-`, no `--` | unit | none — pure | `cargo test --all-features state::` |
| A name that cannot begin an agent name is prefixed | Derive for `2fa-support` and `-leading-dash`; assert `c-2fa-support` and `leading-dash` | unit | none — pure | `cargo test --all-features state::` |
| A name with nothing usable in it | Derive for `""`, `!!!`, `---`; assert `change` each time | unit | none — pure | `cargo test --all-features state::` |
| A truncated name is recorded | Record a derived name for a 48-character change into an empty scratch directory; assert the file, its `[names]` entry, and the read-back | unit | real scratch filesystem | `cargo test --all-features state::` |
| An unchanged name is not recorded | Record for `add-token-refresh` into an uncreated scratch path; assert no file and no directory were created | unit | real scratch filesystem | `cargo test --all-features state::` |
| Recording the same pair twice changes nothing | Record twice; assert the second returns `Ok` and the bytes are identical | unit | real scratch filesystem | `cargo test --all-features state::` |
| A second mapping is added beside the first | Record two different pairs; assert both survive in the file and in the read-back | unit | real scratch filesystem | `cargo test --all-features state::` |
| The same agent name recorded for a different change replaces it | Record `change`→`!!!` and an unrelated pair, then `change`→`---`; assert the binding moved, the unrelated pair is untouched, the call is `Ok`, and no problem is reported | unit | real scratch filesystem | `cargo test --all-features state::` |
| Absent file and absent directory are both empty, not faults | Three reads — uncreated directory, empty directory, zero-byte file; assert empty mapping and empty `problems` each time | unit | real scratch filesystem | `cargo test --all-features state::` |
| A malformed file is empty and reports one problem | Write `[names`; assert empty mapping and one problem naming the file | unit | real scratch filesystem | `cargo test --all-features state::` |
| A bad entry is skipped and its neighbours survive | Write the four-entry table, with the two keys that need it quoted so the fixture is a file with bad entries rather than a syntax error; assert exactly the good pair survives and three problems name the three bad keys | unit | real scratch filesystem | `cargo test --all-features state::` |
| `names` is present but is not a table | Write `names = "nope"`; assert empty mapping and one problem naming `names` | unit | real scratch filesystem | `cargo test --all-features state::` |
| The state directory is created on first record | Record into an uncreated path; assert the directory exists and holds exactly `agent-names.toml` | unit | real scratch filesystem | `cargo test --all-features state::` |
| The file is replaced by rename, not written in place | Hard-link the existing `agent-names.toml` to a second path in the same directory, record a new pair, then assert the target holds the new mapping while the **link still holds the previous bytes**. That is true of a rename and false of every in-place write, including `std::fs::write`, whose truncation leaves no distinguishing tail — a content-only or "no `.tmp-` entry" assertion passes against the non-atomic implementation and must not be used | unit | real scratch filesystem, `std::fs::hard_link` | `cargo test --all-features state::` |
| Recording fails without panicking | Create a regular file at the state directory path; assert `Err`, the error text names the path, the file's bytes are unchanged, and no sibling `.tmp-` file exists | unit | real scratch filesystem | `cargo test --all-features state::` |
| A repository tree is untouched by a recording | Build a scratch fixture holding `openspec/changes/x/tasks.md`, snapshot listing, bytes, and mtimes, record into a separate scratch state directory, assert the snapshot is unchanged | unit | real scratch filesystem | `cargo test --all-features state::` |
| The configuration directory is not written to | Two distinct scratch directories, the configuration one holding a `config.toml`; record; assert its listing, bytes, and mtimes are all unchanged | unit | real scratch filesystem | `cargo test --all-features state::` |
| Exactly one binary target is produced at the release path | Assert one `bin` target named `herdr-openspec`, then build and assert the artifact is executable — the second half is the clause a new dependency could plausibly break, and metadata alone cannot fail for that reason | command check | real cargo, real filesystem | the `BIN` line of the `METADATA` block above, then `/bin/sh scripts/build.sh && test -x target/release/herdr-openspec` |
| The declared dependency set is exactly one crate | Assert — not print — that the normal-kind dependency list is exactly `["toml"]`, that its `uses_default_features` is `false`, and that its features are the four named, all read from resolved metadata rather than from the text of `Cargo.toml`; then `cargo build --locked` | command check | real cargo, real filesystem | the `DEPS` line of the `METADATA` block above, then `cargo build --locked` |
| The resolved build graph is small and proc-macro-free | Collect the package names from `cargo tree -e normal`, `diff` the sorted set against the eight expected (the crate itself plus seven), and assert none of `syn`, `quote`, `proc-macro2`, `serde_derive` appears | command check | real cargo | the `TREE` block above |
| The dependency is genuinely needed rather than incidental | Copy **both** `Cargo.toml` and `Cargo.lock` aside, remove the `toml` line, assert `cargo build` exits non-zero, restore both, then assert `cargo build --locked` exits 0 and `Cargo.lock` is byte-identical to the copy. Restoring only `Cargo.toml` leaves a lock the removal build rewrote, which breaks the requirement the same command is evidence for | command check | real cargo, temporary file copies (never `git checkout`) | `cp Cargo.toml Cargo.lock "$T/"` … `cp "$T/Cargo.toml" "$T/Cargo.lock" .` |
| Every `run:` step is a make invocation of a declared target | Unchanged by this change; the existing parity guard asserts it | existing test | real filesystem | `cargo test --all-features ci_workflow` |
| No environment mapping redefines what a gate does | Unchanged; existing guard | existing test | real filesystem | `cargo test --all-features ci_workflow` |
| The coverage threshold appears only in the Makefile | Unchanged; existing guard | existing test | real filesystem | `cargo test --all-features ci_workflow` |
| The composite target is not used | Unchanged; existing guard | existing test | real filesystem | `cargo test --all-features ci_workflow` |
| Every `run:` step is a single line | Unchanged; existing guard. The delta amends only the requirement's stated reason, which no scenario asserts, so no guard changes | existing test | real filesystem | `cargo test --all-features ci_workflow` |

The live-Herdr check is not a spec scenario — no requirement can assert what Herdr
injects — so it is scheduled as a task rather than a matrix row: link two throwaway
plugins, one declaring a pane and one an action, exercise each, and confirm
`HERDR_PLUGIN_CONFIG_DIR` and `HERDR_PLUGIN_STATE_DIR` are present in both and agree
with `herdr plugin config-dir`. If that check fails, the design's premise is gone and
the fallback paths become the only path; the task says so, and records the Herdr version
it ran against so the archive says what was actually proven.

## Decisions

**Read `HERDR_PLUGIN_CONFIG_DIR` from the environment; never spawn `herdr plugin
config-dir`.** This is the change's central decision and the resolution of the tension
in Context. Alternatives:

- *(a) Inject the directory as a parameter now and defer the real lookup to
  `subprocess-seam`.* This is what the plan would have to do if the environment did not
  carry the directory, and it is unsatisfying: `repo-resolution`, two changes later,
  would have to be written against a value nobody supplies, and the "read `config.toml`"
  row of Phase 1 would ship without ever having read one. Note that the *good* half of
  (a) is kept anyway — `config::load` takes the directory as a parameter, and the
  environment lookup is itself injected — so the pure core is exactly as testable as (a)
  would have made it. What (a) would have cost is a real edge, and the environment
  supplies one.
- *(b) The environment variable.* **Chosen.** Evidence in Context: the strings in the
  binary, three shipping plugins depending on it, one documenting it as herdr-provided,
  and live probes of both a pane process and an action process on this machine. It is
  also strictly better than a spawn: no 200ms process, no failure mode when `herdr` is
  not on `PATH`, and it works identically in a pane and in an action process.
- *(c) Spawn `herdr` from `config`.* Forbidden by `AGENTS.md` → Architecture rules and
  by `openspec/config.yaml` → tasks. Not considered further.
- *(d) Wait for `subprocess-seam` and reorder Phase 1.* Rejected: it would move three
  changes for a dependency that does not exist, and `IMPLEMENTATION-ORDER.md`'s stated
  reason for `plugin-config` preceding `repo-resolution` — "the binary probe chain
  starts with a configured path" — would still hold afterwards.

**`SPEC.md` is corrected in five places, and `AGENTS.md` in three.** Resolution chain
step 1 says the directory is "reported by `herdr plugin config-dir herdr-openspec`"; it
becomes the environment variable, with that command named as the equivalent a human runs
and as the fallback path the plugin computes, and with the mapping file moved from the
config directory to the state directory. The Overview stack list gains `toml`, which it
omits although the configuration format it specifies is TOML. The Architecture module map
gains `config` and `state` rows, and its "the untestable residue is two thin wrappers and
`main`" sentence is re-checked against the new `env_lookup` binding. Degraded states
gains a row for each of the two new degraded reads, so `degraded-states` inherits them.
Herdr integration → Attributing an agent describes only truncation, but `agent_name` also
lowercases, replaces, trims, substitutes, and prefixes, and a mapping is recorded
whenever the derived name differs — `2fa-support` becomes `c-2fa-support` at 13
characters and still needs the mapping; `agent-attribution` is planned from that
paragraph and would otherwise implement a lookup for over-long names only. `AGENTS.md`'s
two architecture bullets are rewritten in place rather than appended to: the spawn bullet
gains the corollary that plugin context comes from the environment, and the never-write
bullet gains the fact that the plugin now writes, and where. `README.md` → Configuration
keeps `herdr plugin config-dir` as the way a *user* finds the directory — that is correct
and stays — and gains a sentence about the path used outside a Herdr process.
`openspec/IMPLEMENTATION-ORDER.md`'s own `plugin-config` row repeats the `herdr plugin
config-dir` claim and is corrected here rather than at archive time, because
`repo-resolution` is planned from it.

**`toml`, at least 1.1.5, `default-features = false`, features `std`, `parse`,
`display`, `serde`.** Alternatives: hand-rolling a reader for three scalar keys —
rejected, because "never fail closed" plus a home-made parser means silently ignoring
valid TOML a user wrote, which is worse than an error; `basic-toml` and `toml_edit` —
rejected, the former is a thinner shim over the same serde machinery with no upside, the
latter is a format-preserving editor for a file we never rewrite. `toml` 1.1.5's own MSRV
is exactly 1.85, matching `Cargo.toml`'s `rust-version`, so the declared floor still
holds — and the requirement now says the declared version's MSRV must stay at or below
it, so a later bump cannot silently falsify the number. `display` is needed because
`state` emits TOML as well as reading it; `serde` is needed because `toml::Table` is
gated behind it — verified by compiling without it and reading the error. **The four
features are exactly `toml`'s own defaults**, so naming them explicitly prunes nothing
today; it is a policy, so that a future change to the crate's defaults arrives as a
reviewable diff rather than as a silent addition. Normal-graph cost measured with
`cargo tree -e normal`: six transitive crates (`serde_core`, `serde_spanned`,
`toml_datetime`, `toml_parser`, `toml_writer`, `winnow`) and no proc macro. `Cargo.lock`
grows to sixteen entries, most of them optional resolutions that are never built; that is
why the replacement `plugin-build` requirement asserts on the *direct* dependency list
and on `cargo tree`, not on a lock count.

**The environment is an injected lookup, not `std::env::var` at each call site.** In
edition 2024 `std::env::set_var` is `unsafe`, and `cargo test` runs tests in parallel
threads of one process, so a test that sets a variable corrupts its neighbours.
Threading a `&dyn Fn(&str) -> Option<String>` through makes every environment-dependent
scenario a pure unit test, and confines the real binding to `config::env_lookup` — which
is itself asserted against, rather than being a line the plan claims is covered and is
not. Alternative considered: a `serial_test` dev-dependency plus `unsafe { set_var }` —
rejected, a second dependency and an `unsafe` block to avoid a one-line parameter.

**Two modules, `config` and `state`, not one.** They differ in direction and in
directory: configuration is read-only and lives in a place the user edits; state is
written by the plugin and lives in a place the user does not. Merging them would make
"nothing writes into the configuration directory" a convention rather than a structural
fact.

**A deterministic hash suffix, not a collision check against the recorded mapping.**
When truncation is needed, the agent name is the first 27 characters (trailing separators
trimmed) plus `-` plus the 32-bit FNV-1a hash of the whole original name, reduced modulo
36⁴ and rendered as four zero-padded base-36 digits. The reduction is stated because a
`u32` needs seven base-36 digits, and "first four" and "last four" would produce
different names for the same change — a value written to disk cannot carry that
ambiguity. Keeping `agent_name` a pure function of its argument is what makes the same
change yield the same agent name in every process and on every machine. Alternatives:
consult the mapping file and append `-2` on collision — rejected, it makes name
derivation depend on I/O and on the order launches happened in, so the same change gets
different names on two machines; a longer hash — rejected, 32 characters is the hard cap
and 27 characters of readable prefix is worth more than two extra hash digits. FNV-1a is
chosen over `DefaultHasher` because `DefaultHasher`'s output is explicitly not guaranteed
stable across Rust releases, and this value is written to disk. Collisions remain
possible — four base-36 digits is 1,679,616 values and every unusable name maps to
`change` — so the recording rule settles them explicitly: a name rebound to a different
change replaces the old binding, because the most recent launch is the live one.

**`Config` and `Mapping` carry a `problems: Vec<String>` instead of returning
`Result`.** "Never fail closed" makes an error return wrong: there is no caller who could
do anything but substitute the default. But silence is also wrong — without `problems`,
"the file was absent" and "the file was garbage" produce identical values, so no test can
tell them apart and no future view can tell the user their configuration was ignored. The
vector makes each degraded path observable at the unit tier, which is what lets eight of
the scenarios above be written at all. Nothing renders it in this change, by design;
`degraded-states` owns that, the field is named in this change's Contracts, and the two
conditions gain rows in `SPEC.md`'s degraded-states table so the obligation survives the
archive.

**Fallback paths copy `herdr-navigator`'s, including its asymmetry.** The configuration
fallback ignores `XDG_CONFIG_HOME` and uses `$HOME/.config/...`; the state fallback
honours `XDG_STATE_HOME`. That is what the shipping plugin does and what Herdr supplied
in the live probes. The asymmetry is inherited rather than reasoned from first
principles, and it is now pinned by a negative scenario rather than by prose alone, so
"fixing" it silently is not possible. It only matters outside a Herdr-started process,
where there is no Herdr to disagree with and usually no file to find. Recorded as a risk
below.

**No `tempfile` dev-dependency.** A ten-line test helper creates a directory named
`herdr-openspec-test-<pid>-<counter>` under `std::env::temp_dir()` and removes it on
drop, which is what `herdr-navigator`'s own test does. Adding a second crate to the graph
to save ten lines would undercut the argument made for the first one. Trade-off: a
panicking test leaks a scratch directory under `/tmp`, which is why the name is
predictable — a check can look for it.

**`rust-version = "1.85"` is unchanged and now means something.** It was the edition
floor with nothing to constrain it; `toml` 1.1.5 declares the same MSRV, so the number
is now the tightest true statement about the crate. An MSRV job remains `ci-pipeline`'s
call, as `repo-foundation` recorded.

## Risks / Trade-offs

- **The environment-variable contract is Herdr's, not ours, and is undocumented in the
  public docs this repository can cite** → Confirmed four ways (binary strings, three
  dependent plugins, a live pane probe, a live action probe) and re-confirmed by a task
  in this change. If a future Herdr drops the variables, the fallback paths still resolve
  to the directory `herdr plugin config-dir` reports, so the failure degrades to
  "configuration is read from the right place anyway" rather than to a crash.
- **`min_herdr_version` stays at 0.7.0, and not because 0.7 is known to inject these
  variables** → Every piece of evidence in this change is about the installed 0.8.2;
  nothing here tests 0.7.x and nothing can. The floor stays because their *absence* is a
  supported state that falls back to the same paths, not because their presence is
  established. A later change that needs a Herdr-injected value with no computable
  fallback — `HERDR_PLUGIN_ID`, `HERDR_PLUGIN_CONTEXT_JSON`, the workspace cwd — must
  revisit the floor rather than inherit this one's reasoning.
- **The `XDG_CONFIG_HOME` asymmetry may be wrong on Linux** → Only reachable outside a
  Herdr-started process, where `config.toml` will not usually exist. Verified against
  macOS only; noted here rather than guessed at, and pinned by a scenario so the
  behaviour cannot drift unnoticed. If a Linux user reports it, honouring
  `XDG_CONFIG_HOME` is a one-line change and the negative scenario is the one to invert.
- **`cargo build --locked` is verified once, not continuously** → `make check` runs
  `cargo test --all-features`, whose command text is pinned by the live `quality-gates`
  requirement, so adding `--locked` there would need a second delta into a capability
  this change otherwise does not touch. The new requirement is therefore scoped to what
  is actually enforced: the change that introduces or alters a dependency verifies
  `--locked` at the commit that lands it. Lock drift between such changes is caught by
  the next one's group-1 CHECK. Recorded here rather than left implied.
- **`problems` is produced and never rendered** → Deliberate, argued above, and assigned
  to `degraded-states` in three places: the Contracts section, the `SPEC.md`
  degraded-states rows this change adds, and the archive of this change.
- **Two dashboards recording at once** → The rename is atomic, so no reader sees a
  partial file, but a genuine last-writer-wins race can drop one of two mappings written
  in the same instant. Accepted: the loss costs one agent's attribution badge until the
  next launch, and locking a file to protect a hint would be the wrong trade.
- **A hand-edited `agent-names.toml` could carry an agent name Herdr rejects** →
  Entries are validated against `[a-z][a-z0-9_-]{0,31}` on read and skipped with a
  problem, so a bad line cannot reach `herdr agent start`.
- **The live probe is destructive under the user's real `~/.config`** → Herdr creates a
  configuration and a state directory for every linked plugin, including the throwaway
  probes, and the task removes them. The mitigation is in the Test Boundaries row: the
  probe ids are never `herdr-openspec`, and removal uses the exact path
  `herdr plugin config-dir <id>` prints rather than a constructed or globbed one.
- **Adding the first dependency makes `cargo build` slower and CI's cache colder once**
  → Six crates, no proc macro. `Swatinem/rust-cache` repopulates on the first run.

## Migration Plan

None needed. There is no prior on-disk format, no deployed consumer, and no data to
backfill: `config.toml` has never been read and `agent-names.toml` has never been
written. Rollback is `git revert` of the change's commits; the only artifact it could
leave behind on a machine is an `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`, which
no other program reads and which a later version reads or ignores harmlessly.

Deploy order within the change: `toml` and the module skeleton first, then `config`, then
`state`, then the environment and dependency confirmations, then the documentation
corrections. Nothing outside the crate has to change in step.

## Visual Design

Not applicable. This change adds no user-facing view and no email template — `ui` is
untouched and lands in Phase 4 — so there is no design source to import.

## Open Questions

None. The one question that would have blocked planning — how a plugin process learns
its configuration directory without spawning `herdr` — was answered empirically before
this document was written, for both entrypoint kinds, and the answer is recorded in
Context with the evidence that settled it.
