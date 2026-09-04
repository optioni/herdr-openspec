## Context

`repo-foundation` left a crate with no dependencies, a `ui` placeholder, and nothing
that reads anything. `plugin-config` is the third Phase 1 row and the first change with
a data layer: `SPEC.md` → Data layer → Resolution chain names `openspec_bin`,
`agent_kind`, and `archived_count` as plugin configuration, and `SPEC.md` → Herdr
integration → Attributing an agent names a plugin-local state file recording the mapping
from a truncated agent name back to its change.

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
  `plugin link`: a throwaway plugin whose only pane dumped `env` was linked, opened with
  `herdr plugin pane open --no-focus`, and unlinked. Its pane received
  `HERDR_PLUGIN_CONFIG_DIR=~/.config/herdr/plugins/config/envprobe` — character-identical
  to what `herdr plugin config-dir envprobe` prints — and
  `HERDR_PLUGIN_STATE_DIR=~/.local/state/herdr/plugins/envprobe`, alongside
  `HERDR_ENV=1`, `HERDR_PLUGIN_ROOT`, `HERDR_SOCKET_PATH`, and the workspace, tab, and
  pane ids.

So the configuration directory arrives in the environment of every pane and action
process Herdr starts. No subprocess is needed at any tier, `subprocess-seam` is not a
dependency, and `SPEC.md` is corrected rather than worked around.

## Goals / Non-Goals

**Goals:**

- Resolve the plugin's configuration and state directories with no process spawn,
  inside a Herdr pane and outside one.
- Turn `config.toml` into a `Config` value that is total: every input, including
  garbage, produces a usable value and never an error.
- Make each fallback *observable*, so "the file was absent" and "the file was garbage"
  are distinguishable in a test and, later, on screen.
- Give `agent-launch` a pure, deterministic change-name-to-agent-name function and a
  durable place to record the mapping it loses to truncation.
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
this change `src/` still contains no `std::process::Command`, and `cli` still does not
exist. **No view is added**, so the rule that views perform no I/O is untouched.

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
pub fn load_from_env() -> Config;              // the one binding to std::env::var

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
  nothing prints — there is no view yet. `degraded-states` renders them.
- **Deployment:** none beyond the normal build. `Cargo.lock` gains entries, so the first
  CI run after this change repopulates `Swatinem/rust-cache`.

## Test Boundaries

This change takes no automated acceptance tier (see Test Strategy), so the first column
names each collaborator's treatment in the deterministic command checks and the one live
Herdr check that stand in for one.

| Dependency | In acceptance test (deterministic command and live checks) | In unit tests |
|---|---|---|
| Process environment | real — the live Herdr probe reads the actual variables Herdr injects into a pane | **replaced**: every function takes an environment lookup closure over a fixture map. `std::env::set_var` is never called — it is `unsafe` in edition 2024 and would race parallel tests |
| `std::env::var` | real — reached only through `config::load_from_env`, the single binding | not called; the one wrapper is exercised by a single test that asserts it agrees with an equivalent explicit lookup |
| Filesystem — configuration directory | real — the live probe compares the injected path with `herdr plugin config-dir` | real, but always a fresh scratch directory under `std::env::temp_dir()`, created and removed by the test. Never `$HOME`, never a repository |
| Filesystem — state directory | real, same scratch technique | real, scratch. This is the only directory any code in this change writes to |
| Repository `openspec/` tree | read-only; `openspec validate --strict` is the final gate | present as a fixture in one test purely to assert it is **not** modified |
| `toml` crate | real | real. Not replaced and not wrapped: it is a parser, not a collaborator with I/O, and faking it would test the fake |
| `herdr` binary and socket | real, and manual — the throwaway env-probe plugin, linked, opened, and unlinked | **not touched.** No production code path invokes `herdr`; a search for it in `src/` is itself a check |
| `openspec` binary | not invoked. `openspec_bin` is a string this change expands and returns; it is never executed, stat-ed, or resolved | not touched |
| `cargo`, `cargo metadata`, `cargo tree` | real — the dependency-set checks | real, as the test runner only |
| Terminal, `ratatui`, `crossterm` | not used — this change adds no view | not touched |
| Clock and randomness | not used. The temporary file name is `agent-names.toml.tmp-<pid>-<counter>` from an atomic counter, so a test can predict and assert its absence | not touched |
| `git` working tree | not used as a restore mechanism anywhere; no check damages a tracked file | not touched |
| `python3` | real — parses `cargo metadata` JSON in the dependency-set checks. `tomllib` is not needed, so the 3.11 floor `repo-foundation` hit does not apply | not touched |

## Test Strategy

Tiers, fastest first:

- **unit** — `cargo test --all-features`, functions in `src/config.rs` and
  `src/state.rs`. Pure functions take fixtures; filesystem functions take a scratch
  directory. This is where 45 of the 48 scenarios live.
- **command check** — a shell command run once, its exit status and output inspected,
  recorded as a task. The tier for the three dependency-set scenarios, which are facts
  about `Cargo.toml` and the resolver rather than about running code.
- **manual (live Herdr)** — the env-probe plugin. One check, and the load-bearing one:
  it is the only evidence that the environment variables this whole design rests on are
  really injected.

**This change does not take the outer-loop acceptance test.** Its outermost surface is a
library API: `src/main.rs` is unchanged, the binary's observable behaviour is still the
`ui` banner, and nothing consumes `Config` until `repo-resolution`. An acceptance test
would have to invent an entry point that ships to nobody. The unit tier already drives
each function end to end against a real filesystem, which is every collaborator these
functions have; the one collaborator it cannot reach — Herdr's injected environment — is
covered by the manual check rather than by a fake pretending to be it.

Shorthand used below:

```sh
# A scratch directory per check; removed at the end of the task.
T=$(mktemp -d)
# Direct, normal-kind dependencies of the package, as JSON.
DEPS='import json,sys; p=json.load(sys.stdin)["packages"][0];
print(sorted(d["name"] for d in p["dependencies"] if d.get("kind") is None))'
```

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| Herdr supplies the directory | Fixture lookup with `HERDR_PLUGIN_CONFIG_DIR` and `HOME` set; assert the former wins; repeat with `HOME` absent and assert an identical result | unit | environment replaced by a closure | `cargo test --all-features config::` |
| Run outside a Herdr pane | Fixture lookup with only `HOME`; assert `$HOME/.config/herdr/plugins/config/herdr-openspec` | unit | environment replaced | `cargo test --all-features config::` |
| An empty environment variable is not a directory | Fixture lookup with the variable set to `""`; assert the `HOME` fallback, not an empty or relative path | unit | environment replaced | `cargo test --all-features config::` |
| Neither variable is available | Empty fixture lookup; assert `None`, then assert `load(None, …)` equals `Config::default()` with an empty `problems` | unit | environment replaced | `cargo test --all-features config::` |
| Resolution spawns nothing | Run the whole suite with a `PATH` from which `herdr` cannot resolve; separately, search `src/` for `Command`, `Stdio`, and `herdr` | command check | herdr deliberately unresolvable | `env PATH="/usr/bin:/bin" cargo test --all-features && ! grep -rnE 'process::Command\|Stdio\|herdr[[:space:]]' src/` |
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
| Run outside a Herdr pane with no XDG setting | Fixture lookup with only `HOME`, and again with `XDG_STATE_HOME=""`; assert `$HOME/.local/state/herdr/plugins/herdr-openspec` both times | unit | environment replaced | `cargo test --all-features state::` |
| No directory can be resolved at all | Empty fixture lookup; assert `None`, `read(None)` empty with no problems, and `record(None, …)` returns `Err` | unit | environment replaced | `cargo test --all-features state::` |
| A short kebab-case change name is unchanged | Derive for `add-token-refresh`; assert equality and a regex match | unit | none — pure | `cargo test --all-features state::` |
| A name of exactly 32 characters is not truncated | Derive for a 32-character name and for that name plus one character; assert the first is unchanged, the second is 32 characters, differs, and carries a `-xxxx` suffix | unit | none — pure | `cargo test --all-features state::` |
| A long change name is truncated with a suffix from the whole name | Derive for the two 27-character-prefix-sharing names; assert length ≤ 32, regex match, inequality, and idempotence | unit | none — pure | `cargo test --all-features state::` |
| Illegal characters and casing are normalised | Derive for `Add Token Refresh (v2)!`; assert `add-token-refresh-v2`, no leading or trailing `-`, no `--` | unit | none — pure | `cargo test --all-features state::` |
| A name that cannot begin an agent name is prefixed | Derive for `2fa-support` and `-leading-dash`; assert `c-2fa-support` and `leading-dash` | unit | none — pure | `cargo test --all-features state::` |
| A name with nothing usable in it | Derive for `""`, `!!!`, `---`; assert `change` each time | unit | none — pure | `cargo test --all-features state::` |
| A truncated name is recorded | Record a derived name for a 48-character change into an empty scratch directory; assert the file, its `[names]` entry, and the read-back | unit | real scratch filesystem | `cargo test --all-features state::` |
| An unchanged name is not recorded | Record for `add-token-refresh` into an uncreated scratch path; assert no file and no directory were created | unit | real scratch filesystem | `cargo test --all-features state::` |
| Recording the same pair twice changes nothing | Record twice; assert the second returns `Ok` and the bytes are identical | unit | real scratch filesystem | `cargo test --all-features state::` |
| A second mapping is added beside the first | Record two different pairs; assert both survive in the file and in the read-back | unit | real scratch filesystem | `cargo test --all-features state::` |
| Absent file and absent directory are both empty, not faults | Three reads — uncreated directory, empty directory, zero-byte file; assert empty mapping and empty `problems` each time | unit | real scratch filesystem | `cargo test --all-features state::` |
| A malformed file is empty and reports one problem | Write `[names`; assert empty mapping and one problem naming the file | unit | real scratch filesystem | `cargo test --all-features state::` |
| A bad entry is skipped and its neighbours survive | Write the three-entry table; assert exactly the good pair survives and two problems name the two bad keys | unit | real scratch filesystem | `cargo test --all-features state::` |
| `names` is present but is not a table | Write `names = "nope"`; assert empty mapping and one problem naming `names` | unit | real scratch filesystem | `cargo test --all-features state::` |
| The state directory is created on first record | Record into an uncreated path; assert the directory exists and holds exactly `agent-names.toml` | unit | real scratch filesystem | `cargo test --all-features state::` |
| The write is not visible until it is complete | Record over an existing file; assert the directory listing during and after holds no `.tmp-` entry, and that the implementation renames — a test that writes a sentinel into the target path and asserts `record` replaced it wholesale rather than truncating it in place | unit | real scratch filesystem | `cargo test --all-features state::` |
| Recording fails without panicking | Create a regular file at the state directory path; assert `Err`, the error text names the path, the file's bytes are unchanged, and no sibling `.tmp-` file exists | unit | real scratch filesystem | `cargo test --all-features state::` |
| A repository tree is untouched by a recording | Build a scratch fixture holding `openspec/changes/x/tasks.md`, snapshot listing, bytes, and mtimes, record into a separate scratch state directory, assert the snapshot is unchanged | unit | real scratch filesystem | `cargo test --all-features state::` |
| The configuration directory is not written to | Two distinct scratch directories; record; assert the configuration directory's listing is unchanged | unit | real scratch filesystem | `cargo test --all-features state::` |
| Exactly one binary target is produced at the release path | Filter `cargo metadata` targets by `kind` containing `bin`; assert one, named `herdr-openspec`; assert the built file is executable | command check | real cargo, real filesystem | `cargo metadata --no-deps --format-version 1 \| python3 -c 'import json,sys; t=[x for p in json.load(sys.stdin)["packages"] for x in p["targets"] if "bin" in x["kind"]]; assert [x["name"] for x in t]==["herdr-openspec"], t'` |
| The declared dependency set is exactly one crate | Assert the normal-kind dependency list is exactly `["toml"]`, that `Cargo.toml` carries `default-features = false` and the four features, and that `cargo build --locked` succeeds | command check | real cargo, real filesystem | `cargo metadata --no-deps --format-version 1 \| python3 -c "$DEPS"` then `cargo build --locked` |
| The dependency is genuinely needed rather than incidental | Copy `Cargo.toml` aside, remove the `toml` line, assert `cargo build` fails, restore from the copy, assert `cargo test --all-features` is green | command check | real cargo, temporary file copy (never `git checkout`) | `cp Cargo.toml "$T/"; …; cargo build; cp "$T/Cargo.toml" .` |

The live-Herdr check is not a spec scenario — no requirement can assert what Herdr
injects — so it is scheduled as a task rather than a matrix row: link a throwaway
env-dumping plugin, open its pane, and confirm `HERDR_PLUGIN_CONFIG_DIR` and
`HERDR_PLUGIN_STATE_DIR` are present and agree with `herdr plugin config-dir`. If that
check fails, the design's premise is gone and the fallback paths become the only path;
the task says so.

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
  and a live probe on this machine. It is also strictly better than a spawn: no 200ms
  process, no failure mode when `herdr` is not on `PATH`, and it works identically in a
  pane and in an action process.
- *(c) Spawn `herdr` from `config`.* Forbidden by `AGENTS.md` → Architecture rules and
  by `openspec/config.yaml` → tasks. Not considered further.
- *(d) Wait for `subprocess-seam` and reorder Phase 1.* Rejected: it would move three
  changes for a dependency that does not exist, and `IMPLEMENTATION-ORDER.md`'s stated
  reason for `plugin-config` preceding `repo-resolution` — "the binary probe chain
  starts with a configured path" — would still hold afterwards.

**`SPEC.md` is corrected in three places.** Resolution chain step 1 says the directory is
"reported by `herdr plugin config-dir herdr-openspec`"; it becomes the environment
variable, with that command named as the equivalent a human runs and as the fallback path
the plugin computes. The Overview stack list gains `toml`, which it omits although the
configuration format it specifies is TOML. The Architecture module map gains `config` and
`state` rows. `README.md` → Configuration keeps `herdr plugin config-dir` as the way a
*user* finds the directory — that is correct and stays — and gains a sentence about the
path used outside a Herdr pane.

**`toml` 1.1.5, `default-features = false`, features `std`, `parse`, `display`,
`serde`.** Alternatives: hand-rolling a reader for three scalar keys — rejected, because
"never fail closed" plus a home-made parser means silently ignoring valid TOML a user
wrote, which is worse than an error; `basic-toml` and `toml_edit` — rejected, the former
is a thinner shim over the same serde machinery with no upside, the latter is a
format-preserving editor for a file we never rewrite. `toml`'s own MSRV is exactly
1.85, matching `Cargo.toml`'s `rust-version`, so the declared floor still holds.
`display` is needed because `state` emits TOML as well as reading it; `serde` is needed
because `toml::Table` is gated behind it — verified by compiling without it and reading
the error. Normal-graph cost measured with `cargo tree -e normal`: six transitive crates
(`serde_core`, `serde_spanned`, `toml_datetime`, `toml_parser`, `toml_writer`,
`winnow`) and no proc macro. `Cargo.lock` grows to sixteen entries, most of them
optional resolutions that are never built; that is why the replacement `plugin-build`
requirement asserts on the *direct* dependency list and `cargo tree`, not on a lock
count.

**The environment is an injected lookup, not `std::env::var` at each call site.** In
edition 2024 `std::env::set_var` is `unsafe`, and `cargo test` runs tests in parallel
threads of one process, so a test that sets a variable corrupts its neighbours.
Threading a `&dyn Fn(&str) -> Option<String>` through makes every environment-dependent
scenario a pure unit test, and confines the real binding to `config::load_from_env`.
Alternative considered: a `serial_test` dev-dependency plus `unsafe { set_var }` —
rejected, a second dependency and an `unsafe` block to avoid a one-line parameter.

**Two modules, `config` and `state`, not one.** They differ in direction and in
directory: configuration is read-only and lives in a place the user edits; state is
written by the plugin and lives in a place the user does not. Merging them would make
"nothing writes into the configuration directory" a convention rather than a structural
fact.

**A deterministic hash suffix, not a collision check against the recorded mapping.**
When truncation is needed, the agent name is the first 27 characters plus `-` plus four
base-36 digits of FNV-1a-32 over the whole original name. This keeps `agent_name` a pure
function of its argument, so the same change yields the same agent name in every process
and on every machine, and two changes sharing a 27-character prefix still differ.
Alternatives: consult the mapping file and append `-2` on collision — rejected, it makes
name derivation depend on I/O and on the order launches happened in, so the same change
gets different names on two machines; a longer hash — rejected, 32 characters is the hard
cap and 27 characters of readable prefix is worth more than two extra hash digits. FNV-1a
is chosen over `DefaultHasher` because `DefaultHasher`'s output is explicitly not
guaranteed stable across Rust releases, and this value is written to disk.

**`Config` and `Mapping` carry a `problems: Vec<String>` instead of returning
`Result`.** "Never fail closed" makes an error return wrong: there is no caller who could
do anything but substitute the default. But silence is also wrong — without `problems`,
"the file was absent" and "the file was garbage" produce identical values, so no test can
tell them apart and no future view can tell the user their configuration was ignored. The
vector makes each degraded path observable at the unit tier, which is what lets eight of
the scenarios above be written at all. Nothing renders it in this change, by design;
`degraded-states` owns that, and the field is named in this change's Contracts so that
change inherits it rather than inventing it.

**Fallback paths copy `herdr-navigator`'s, including its asymmetry.** The configuration
fallback ignores `XDG_CONFIG_HOME` and uses `$HOME/.config/...`; the state fallback
honours `XDG_STATE_HOME`. That is what the shipping plugin does and what Herdr supplied
in the live probe. The asymmetry is inherited rather than reasoned from first
principles — recorded as a risk below. It only matters outside a Herdr pane, where there
is no Herdr to disagree with and usually no file to find.

**No `tempfile` dev-dependency.** A ten-line test helper creates a uniquely named
directory under `std::env::temp_dir()` and removes it, which is what `herdr-navigator`'s
own test does. Adding a second crate to the graph to save ten lines would undercut the
argument made for the first one. Trade-off: a panicking test leaks a scratch directory
under `/tmp`.

**`rust-version = "1.85"` is unchanged and now means something.** It was the edition
floor with nothing to constrain it; `toml` 1.1.5 declares the same MSRV, so the number
is now the tightest true statement about the crate. An MSRV job remains `ci-pipeline`'s
call, as `repo-foundation` recorded.

## Risks / Trade-offs

- **The environment-variable contract is Herdr's, not ours, and is undocumented in the
  public docs this repository can cite** → Confirmed three ways (binary strings, three
  dependent plugins, a live probe) and re-confirmed by a task in this change. If a future
  Herdr drops the variables, the fallback paths still resolve to the directory
  `herdr plugin config-dir` reports, so the failure degrades to "configuration is read
  from the right place anyway" rather than to a crash. `min_herdr_version` stays at
  0.7.0; the variables were not introduced by 0.8.
- **The `XDG_CONFIG_HOME` asymmetry may be wrong on Linux** → Only reachable outside a
  Herdr pane. Verified against macOS only; noted here rather than guessed at. If a Linux
  user reports it, honouring `XDG_CONFIG_HOME` is a one-line change with a scenario
  already shaped for it.
- **`problems` is produced and never rendered** → Deliberate, argued above, and assigned
  to `degraded-states`. The risk is that it is forgotten; the Contracts section names the
  consumer so the archive of this change carries the obligation forward.
- **Two dashboards recording at once** → The rename is atomic, so no reader sees a
  partial file, but a genuine last-writer-wins race can drop one of two mappings written
  in the same instant. Accepted: the loss costs one agent's attribution badge until the
  next launch, and locking a file to protect a hint would be the wrong trade.
- **A hand-edited `agent-names.toml` could carry an agent name Herdr rejects** →
  Entries are validated against `[a-z][a-z0-9_-]{0,31}` on read and skipped with a
  problem, so a bad line cannot reach `herdr agent start`.
- **Adding the first dependency makes `cargo build` slower and CI's cache colder once**
  → Six crates, no proc macro. `Swatinem/rust-cache` repopulates on the first run.

## Migration Plan

None needed. There is no prior on-disk format, no deployed consumer, and no data to
backfill: `config.toml` has never been read and `agent-names.toml` has never been
written. Rollback is `git revert` of the change's commits; the only artifact it could
leave behind on a machine is an `agent-names.toml` under `HERDR_PLUGIN_STATE_DIR`, which
no other program reads and which a later version reads or ignores harmlessly.

Deploy order within the change: `toml` and the module skeleton first, then `config`, then
`state`, then the documentation corrections, then the live-Herdr confirmation. Nothing
outside the crate has to change in step.

## Visual Design

Not applicable. This change adds no user-facing view and no email template — `ui` is
untouched and lands in Phase 4 — so there is no design source to import.

## Open Questions

None. The one question that would have blocked planning — how a plugin process learns
its configuration directory without spawning `herdr` — was answered empirically before
this document was written, and the answer is recorded in Context with the evidence that
settled it.
