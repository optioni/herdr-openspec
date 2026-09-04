## Context

`repo-resolution` landed `find_repo`, so the crate can say *where* the repository is.
Nothing can yet say what is inside a change. The detail view's tab bar is the schema's
artifact list in schema order, and the tasks tab is one particular member of that list, so
`changes-from-files` cannot build a `Change` and `detail-view` cannot render a tab until
something reads the schema. This is the second row of Phase 2 in
`openspec/IMPLEMENTATION-ORDER.md`, and like every Phase 2 row it is a pure transformation:
no terminal, no subprocess, no writes.

**Facts read off the real OpenSpec CLI (1.11.0) and the real vendored schema before writing
this design.** Every one of them changed a decision, and two of them make `SPEC.md` wrong.

| Fact | Observed | Consequence |
|---|---|---|
| `grep -n role openspec/schemas/tdd/schema.yaml` | **no match**, exit 1 | `SPEC.md`'s `role: tasks` rule cannot be implemented — the key does not exist |
| `grep -rn role <openspec-pkg>/schemas` | **no match** | Not a quirk of the vendored schema; `role` is not in the format |
| `dist/core/artifact-graph/types.js` → `ArtifactSchema` | `id`, `generates`, `description`, `template`, `instruction?`, `requires[]` | The artifact keys that exist. No `role` |
| `dist/core/artifact-graph/types.js` → `SchemaYamlSchema` | `name`, `version`, `description?`, `artifacts[]` (min 1), `apply?` | `artifacts` is required and non-empty; `apply` is optional |
| `dist/core/artifact-graph/types.js` → `ApplyPhaseSchema` | `requires[]` (min 1), `tracks: relativePath \| null \| undefined`, `instruction?` | `apply.tracks` is the real mechanism, and it may be explicitly null |
| `dist/utils/task-progress.js` → `findTrackedTasksArtifact` | `tracks != null ? artifacts.find(a => a.generates === tracks) : artifacts.find(a => a.id === 'tasks')` | The rule to mirror, including that a `tracks` miss does **not** fall back to the id |
| `openspec/schemas/tdd/schema.yaml` | `apply.tracks: tasks.md`, and the `tasks.md` artifact's id is `tasks` | Both rules agree here — so every fixture must be one where they disagree |
| `dist/utils/change-metadata.js:117` | change `.openspec.yaml` → project `config.yaml` → `'spec-driven'` | Two sources `SPEC.md` never mentions: the per-change override and the default |
| `dist/core/artifact-graph/resolver.js:114` | project `openspec/schemas/` → `$XDG_DATA_HOME` → package built-in | Only tier 1 is readable from disk; tiers 2–3 are what a CLI fallback is *for* |
| `openspec schema --help` | subcommands `which`, `validate`, `fork`, `init` — **no dump command** | `SPEC.md`'s "ask the CLI via `openspec schema`" names a command that does not exist |
| `openspec schema which tdd --json` | `{"name","source","path","shadows"}`; the "experimental" note goes to **stderr**, stdout is clean JSON | The real Phase 3 fallback: it returns the schema's *directory*, whose `schema.yaml` this same parser then reads |
| `openspec status --change <n> --json` | `artifacts[]` with `id`, `outputPath`, `status`, `requires`, in schema order | `changes-from-cli` already plans this call and it already carries the ordered list |
| Indentation of `apply.instruction`'s block scalar in the vendored `tdd` schema | **4 spaces** — the same column as `    generates:` under an artifact | A line scanner reads that markdown as structure. This is the concrete reason the parser must be a YAML parser |
| `cargo search`/`cargo tree` over five YAML crates | `serde_yaml` deprecated; `serde_yml` a deprecated shim; `serde_yaml_ng`/`serde_norway` carry an `unsafe-libyaml` C transpile; `saphyr` pulls `thiserror` → `syn`/`quote`/`proc-macro2` | Four of five are disqualified or costly. `yaml-rust2` is 4 transitive crates, pure Rust, no `unsafe` in its source, no proc macro |
| Live `plugin-build` requirement | pins the normal dependency set at exactly `["toml"]` and the resolved graph package-for-package, and forbids a proc macro anywhere in it | Adding any YAML crate is a delta spec on a capability this change otherwise leaves alone — the forcing function working as intended |

## Goals / Non-Goals

**Goals**

- `schema::select` — which schema name applies to a repository, or to one change inside it,
  and which of the three sources answered.
- `schema::load` — a named schema read from `openspec/schemas/<name>/schema.yaml` into an
  ordered artifact list, with the three not-loaded reasons kept apart.
- `schema::parse` — the pure core, where forty of this change's scenarios live.
- The tasks-artifact rule as the CLI actually implements it, with fixtures where the
  `apply.tracks` rule and the id-`tasks` rule disagree.
- A YAML dependency chosen on measured evidence and argued here, plus the `plugin-build`
  delta that adding it requires.
- Correct `SPEC.md` where this change proves it wrong — `role: tasks`, the nonexistent
  `openspec schema` command, the deprecated `serde_yaml`, the missing default and override —
  and hand the CLI-fallback obligation to `changes-from-cli` on the roadmap.

**Non-Goals**

- No process spawn, no `cli` module, no CLI fallback. `openspec schema which` is Phase 3.
- No injected hook for that fallback. `repo-resolution` injected one for `npm prefix -g`
  because step 4 sat *inside* an ordered chain whose position had to be pinned today; here
  there is no chain — the disk load either produces a schema or names why it did not, and
  the CLI tier comes after all of it. See Decisions.
- No change enumeration, no artifact **file path** resolution, no task parsing, no
  rendering. Those are `changes-from-files`, `task-parsing`, and `detail-view`.
- No caching. `SPEC.md` specifies session caching for the binary probe and nothing else.
  The API is split so a caller that wants to avoid repeated work can (see Decisions).
- No YAML surface in the public API. No consumer sees a `Yaml` value, so the parser crate
  stays replaceable.
- No `description`, `template`, `instruction`, or `requires` in the model. The plugin
  renders none of them.
- No writes anywhere. No production path in this change creates, modifies, or removes
  anything.
- No Windows path handling. `platforms = ["macos", "linux"]`.

## Boundaries

| Piece | Where | Pattern it follows |
|---|---|---|
| `select`, `load`, `resolve`, `parse`, and the value types | new `src/schema.rs` | `src/resolve.rs` and `src/config.rs`: a pure core, a thin filesystem edge, and one composition |
| `FileText` — absent / read / unreadable | `src/schema.rs`, crate-internal | The same move `resolve::path_candidates` makes: lift the one thing the filesystem contributes into a value, so the ordering logic above it is pure and its ten scenarios need no scratch tree |
| Blank-value rule | `config::non_blank` | Already shared by `config`, `state`, and `resolve`. A `schema:` of `"  "` is not a declaration, exactly as a blank `PATH` entry is not a candidate |
| `problems: Vec<String>` | `SchemaResolution` | `Config::problems` and `BinResolution::problems`, unchanged in shape. `degraded-states` renders one thing, not three |
| `pub mod schema;` | `src/lib.rs` | Alongside `config`, `resolve`, `state` |
| A fixture-repository builder | `src/lib.rs` → `testutil` | Beside `ScratchDir`, `write_with_mode`, `symlink`, `canonical`, `snapshot` — all reused unchanged, none extended |
| `yaml-rust2` | `Cargo.toml` | The `toml` dependency's shape: `default-features = false` plus an explicit feature list |

**No process spawn is added.** `src/schema.rs` names no process API at all; the `cli` module
still does not exist. The `openspec schema which <name> --json` invocation this design
*describes* is a roadmap obligation and a doc comment, never a call.

**No view is added.** `ui` does not exist yet, so the "views perform no I/O" and "view tests
at 60 and 120 columns" concentration points have nothing to bind to here. `detail-view` is
the first consumer that renders anything from this list.

**The `Change` type does not exist yet**, so the `from_files`/`from_cli` agreement
concentration point does not apply. `changes-from-files` introduces it and will carry
`Vec<Artifact>` into it; nothing in this change constrains that shape beyond exposing
`generates` so the file path can be derived rather than guessed.

## Contracts

Every consumer is a later change in this repository. Nothing outside it depends on any of
this. Every type derives `Debug, Clone, PartialEq, Eq` — matching `config::Config` and
`resolve::BinResolution` — because the scenarios compare whole values with `assert_eq!` and
print them on failure.

```rust
/// The schema assumed when nothing declares one. The OpenSpec CLI's own default.
pub const DEFAULT_SCHEMA: &str = "spec-driven";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact { pub id: String, pub generates: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSource { Change, Project, Default }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    /// The directory segment that located the file, not the file's own `name:`
    /// key — a forked schema whose directory was renamed still resolves by
    /// directory, and a mismatch between the two is recorded as a problem.
    pub name: String,
    /// Ordered as the `artifacts:` sequence lists them. This is the tab order.
    pub artifacts: Vec<Artifact>,
    /// The artifact holding the task checklist, cloned out of `artifacts`
    /// rather than indexed into it, so a consumer that filters cannot desync.
    pub tasks: Option<Artifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSchema { pub schema: Schema, pub problems: Vec<String> }

/// Why a named schema did not load. Three variants rather than one message,
/// because each has a different degraded row and `changes-from-cli` branches on
/// exactly one of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// No `schema.yaml` under `openspec/schemas/<name>/`. Normal for a schema
    /// the CLI ships rather than the repository. `changes-from-cli` repairs this
    /// tier with `openspec schema which <name> --json`.
    NotVendored { path: PathBuf },
    /// The path exists but its bytes could not be obtained.
    Unreadable { path: PathBuf, reason: String },
    /// The bytes were read and are not a usable schema.
    Invalid { path: PathBuf, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection { pub name: String, pub source: NameSource, pub problems: Vec<String> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaResolution {
    pub name: String,
    pub source: NameSource,
    pub schema: Result<Schema, LoadError>,
    pub problems: Vec<String>,
}

// --- pure core: no filesystem access anywhere below this line ---

/// What one configuration file contributed. Produced by the filesystem edge so
/// that the selection ordering above it is pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileText {
    /// Does not exist. Contributes nothing and records no problem.
    Absent,
    /// Read successfully.
    Read(String),
    /// Exists but could not be read; carries the I/O message.
    Unreadable(String),
}

/// `Ok(Some(name))` when the first document declares a non-blank string
/// `schema:`; `Ok(None)` when the text holds no document; `Err(reason)` when it
/// is not valid YAML, its first document is not a mapping, or `schema:` is
/// present with a non-string value.
pub(crate) fn schema_key(text: &str) -> Result<Option<String>, String>;

/// A single, non-escaping path segment: non-blank, no separator, not `.` or
/// `..`, not absolute, no NUL.
pub(crate) fn is_legal_name(name: &str) -> bool;

/// The three-source ordering, pure. `change` is `None` when no change directory
/// was supplied; `project` is always a path, possibly `Absent`.
pub(crate) fn declared_name(
    change: Option<(&Path, &FileText)>,
    project: (&Path, &FileText),
) -> Selection;

/// Parse one `schema.yaml` document. `Err` is the reason it is not a usable
/// schema at all; `Ok` carries the schema plus one problem per skipped entry.
pub fn parse(name: &str, text: &str) -> Result<ParsedSchema, String>;

// --- filesystem edge ---

/// Read a path into a `FileText`, mapping `NotFound` to `Absent`.
fn read_file(path: &Path) -> FileText;

/// Which schema applies to `repo`, or to `change_dir` inside it.
pub fn select(repo: &Path, change_dir: Option<&Path>) -> Selection;

/// Read and parse `<repo>/openspec/schemas/<name>/schema.yaml`.
pub fn load(repo: &Path, name: &str) -> Result<ParsedSchema, LoadError>;

/// The composition, for the common case.
pub fn resolve(repo: &Path, change_dir: Option<&Path>) -> SchemaResolution;
```

- **Error surface.** `resolve` is infallible: it always returns a `SchemaResolution`
  carrying a name, a source, either a schema or one of the three reasons, and the problems.
  `parse` and `load` return `Result` because they are the inner functions where "this is not
  a schema" is the honest answer; `resolve` converts an `Err` into a problem. Nothing
  panics — not on a path that does not exist, not on a directory where a file was expected,
  not on a YAML document that is a bare scalar, and not on indexing a node that is not a
  mapping.
- **`NameSource` is part of the contract, not a debugging aid.** Two sources routinely
  declare the same name, so a name alone cannot prove the ordering. This is the same
  argument `resolve::BinSource` makes, for the same reason.
- **`LoadError` has three variants rather than one string** because `changes-from-cli` must
  branch on `NotVendored` specifically, and a consumer matching substrings in a message is
  a consumer that breaks when the message is reworded.
- **`Schema::name` is the directory segment**, because that is what `openspec schema which`
  keys on and what the user configured. A disagreement with the file's `name:` is a problem,
  not a failure.
- **`Schema::tasks` is a clone, not an index.** An index into `artifacts` desyncs the moment
  a consumer filters or reorders the list, and two small `String`s are cheaper than that
  class of bug. It also makes the scenarios `assert_eq!`-comparable against a whole
  `Artifact` value.
- **`problems` has three producers:** selection fallbacks, per-entry skips, and a
  `LoadError`. An empty `problems` with a loaded schema means "everything read cleanly".
- **Compatibility.** Purely additive. No existing signature changes; `src/main.rs`,
  `src/config.rs`, `src/state.rs`, and `src/resolve.rs` are untouched.
- **Not BREAKING.** No manifest key, no `config.toml` key, no keybinding.
- **Consumers named:** `changes-from-files` (`select` per change, `load` once per distinct
  name, `Artifact::generates` to derive artifact file paths), `detail-view`
  (`Schema::artifacts` as the tab order), `tasks-tab` (`Schema::tasks`),
  `changes-from-cli` (`LoadError::NotVendored` as its trigger to ask the CLI),
  `degraded-states` (`problems`, and the three `LoadError` variants).

## Persistence and Rollout

- **Migration:** none. No on-disk format is introduced, written, or altered.
- **Backfill:** none.
- **Seeding:** none. Every fixture is built by the test that uses it and removed on drop,
  except the one scenario that reads this repository's own vendored `tdd` schema read-only.
- **Cache invalidation:** none — nothing is cached. `select` and `load` are separate entry
  points precisely so `changes-from-files` can call `load` once per distinct name instead of
  once per change, without this module owning a cache lifetime.
- **Index rebuild:** none.
- **Authorization:** none — a single-user local plugin. The privilege question is what it
  *reads*: three files under the repository root and nothing else. It writes nothing, which
  two scenarios assert.
- **Observability:** `SchemaResolution::problems` and the `LoadError` variants are the whole
  of it, matching `Config::problems` and `BinResolution::problems`. Nothing logs and nothing
  prints; there is no view yet. This change adds the corresponding rows to `SPEC.md` →
  Degraded states so `degraded-states` inherits them rather than inventing them.
- **Deployment:** none beyond the normal build. `Cargo.lock` **does** move — five packages
  are added to the normal graph — so `Swatinem/rust-cache`'s key changes once and CI
  recompiles; no workflow edit is needed.

## Test Boundaries

This change takes no automated acceptance tier (see Test Strategy), so the first column
names each collaborator's treatment in the deterministic command checks that stand in for
one.

| Dependency | In acceptance test (deterministic command checks) | In unit tests |
|---|---|---|
| Filesystem — scratch trees | not used | **real**, always a fresh `testutil::ScratchDir` under `std::env::temp_dir()`. Never a directory on the machine's real `PATH` or in `$HOME` |
| Filesystem — this repository's own tree | **real, read-only** — the dependency, graph, MSRV and spawn checks read `Cargo.toml`, `Cargo.lock`, and `src/` | **real, read-only, exactly one scenario**: "The repository's own vendored `tdd` schema loads" reads `openspec/schemas/tdd/schema.yaml` through `env!("CARGO_MANIFEST_DIR")` — never through the process working directory, which a test runner may change |
| Process environment | not manipulated. No check strips `PATH`; unlike `repo-resolution`, nothing here reads an environment variable | **not used at all.** No function in this change takes an environment lookup, because none needs one — the repository root arrives as an argument from `resolve::find_repo` |
| `std::env::var` | not reached | not reached. `config::env_lookup` is untouched |
| `std::env::temp_dir()` | **real, read-only** — the leak check scans it for `herdr-openspec-test-` directories | real, and read-only. It is where `ScratchDir` puts every fixture |
| Symbolic links | not used | **not used.** No scenario in this change depends on link behaviour; `read_to_string` follows links and that is all this module needs |
| Unix permission bits | not used | **not used.** `testutil::write_with_mode` exists but this change's fixtures need no explicit mode; nothing here tests an execute bit |
| YAML parsing (`yaml-rust2`) | present in the build graph and asserted there | **real, never faked.** Faking it would be testing the fake: block-scalar indentation, flow style, and anchors are exactly the behaviours under test |
| `toml` crate | present in the build graph, unchanged | **not used by this change.** `schema` parses no TOML |
| `openspec` binary | **never invoked by this crate**, and this change adds no code path that could. It *is* invoked by the workflow, once, as the read-only `openspec validate schema-model --strict` gate | not touched |
| `npm`, `node`, `herdr` binaries and the Herdr socket | not used. No production path invokes any of them | not touched |
| Clock and randomness | not used. No test sleeps, polls, or reads the clock; `ScratchDir` names come from pid plus an atomic counter | not touched |
| Threads | none. Nothing in this change spawns one, and no test does | not touched |
| Terminal, `ratatui`, `crossterm` | not used — this change adds no view | not touched |
| `cargo`, `cargo metadata`, `cargo tree`, `cargo llvm-cov`, `make` | **real** — the dependency-set, build-graph, MSRV, coverage, and `make check` gates. `cargo tree` is used here, unlike in `repo-resolution`, because this change *does* alter the build graph and the live requirement pins it package-for-package | real, as the test runner only |
| `python3` | **real** — parses `cargo metadata` and `cargo tree` output for the dependency and MSRV checks. No `tomllib`, so no 3.11 floor | not touched |
| POSIX shell userland (`sh`, `grep`, `awk`, `sort`, `tr`, `sed`) | **real.** macOS/BSD is the reference platform: no `stat -c`, no `sed -i` without an argument, no `grep -P`, and no `paste -sd:` (a usage error on BSD that silently yields an empty string) | not touched; modification times are read in Rust through `std::fs::Metadata`, never through `stat` |
| `git` | **real, read-only** — `git status --short` at baseline, and `git diff --stat -- Cargo.lock` to confirm the lock moved by exactly the expected additions. Never `checkout`, `restore`, or `reset` | not touched |
| The graft-vendored `openspec/schemas/tdd/` | **read, never written.** One unit scenario parses it; the containment scenarios prove no run modifies it | read-only, same |
| Network | **not used.** `cargo build --locked` and `cargo metadata` resolve from the committed lock and the local registry cache after the one-time fetch in task 1.3 | not touched |

## Test Strategy

Tiers, fastest first:

- **unit** — `cargo test --all-features schema::`, functions in `src/schema.rs`. **41 of the
  47 scenarios live here**, and 30 of those reach no filesystem at all: `schema_key`,
  `is_legal_name`, `declared_name`, and `parse` are pure functions over `&str` and
  `FileText` values.
- **command check** — a shell command run once, its exit status inspected, recorded as a
  task. **Six scenarios:** `plugin-build`'s five, which are facts about the build graph
  rather than about running code, and "The schema module names no process API", which is a
  fact about the source text.

**This change does not take the outer-loop acceptance test.** Its outermost surface is a
library API. `src/main.rs` is untouched, the binary's observable behaviour is still the `ui`
banner, and no caller consumes a `Schema` until `changes-from-files`. An acceptance test
would have to drive an entry point that ships to nobody — the same argument `plugin-config`
and `repo-resolution` made, for the same reason. The unit tier reaches every collaborator
these functions have: a real filesystem for three files, and a real YAML parser.

Shorthand used below. Every command is written for BSD userland and every one has been run,
with a negative control, before being written here.

```sh
# DEPS — the declared normal dependency set and each crate's resolved features.
# Written with an explicit sys.exit rather than a bare `assert`, which `python3 -O` or an
# inherited PYTHONOPTIMIZE strips: verified that the assert form exits 0 under
# PYTHONOPTIMIZE=1 against the wrong dependency list. This form exits 1 with a message.
DEPS='import json, sys
p = json.load(sys.stdin)["packages"][0]
normal = {d["name"]: d for d in p["dependencies"] if d.get("kind") is None}
if sorted(normal) != ["toml", "yaml-rust2"]:
    sys.exit("unexpected normal deps: %r" % (sorted(normal),))
want = {"toml": ["display", "parse", "serde", "std"], "yaml-rust2": []}
for name, d in normal.items():
    if d["uses_default_features"] is not False:
        sys.exit("%s: default features are on" % name)
    if sorted(d["features"]) != want[name]:
        sys.exit("%s: features are %r, expected %r" % (name, sorted(d["features"]), want[name]))'
# Run: cargo metadata --no-deps --format-version 1 | python3 -c "$DEPS"
# Verified: exit 0 against a probe crate declaring both; exit 1 with
# "unexpected normal deps: ['toml']" against this crate at HEAD, and still exit 1 under
# PYTHONOPTIMIZE=1.

# GRAPH — the resolved normal build graph, package for package.
GRAPH_WANT='arraydeque foldhash hashbrown hashlink serde_core serde_spanned toml toml_datetime toml_parser toml_writer winnow yaml-rust2'
got=$(cargo tree -e normal --prefix none | awk 'NF {print $1}' | grep -vx 'herdr-openspec' | sort -u | tr '\n' ' ' | sed 's/ $//')
[ "$got" = "$GRAPH_WANT" ] || { echo "graph mismatch: [$got]" >&2; exit 1; }
# `awk 'NF {print $1}'` rather than `cut`, because cargo indents nothing under --prefix none
# but does emit a trailing blank line. `grep -vx` on the root name so a rename fails the
# check rather than silently passing. Verified: matches against a probe crate declaring both
# dependencies, and mismatches with [serde_core serde_spanned toml ...] against this crate
# at HEAD.

# MSRV — every package in the NORMAL graph, not every package cargo metadata resolves.
# The intersection is load-bearing: `cargo metadata` also reports syn, serde_derive and
# indexmap, which cargo never builds here.
MSRV='import json, subprocess, sys
FLOOR = (1, 85, 0)
def v(s):
    parts = [int(x) for x in s.split(".")]
    while len(parts) < 3: parts.append(0)
    return tuple(parts[:3])
tree = subprocess.run(["cargo", "tree", "-e", "normal", "--prefix", "none"],
                      capture_output=True, text=True)
if tree.returncode != 0:
    sys.exit("cargo tree failed: " + tree.stderr.strip())
names = {l.split()[0] for l in tree.stdout.splitlines() if l.strip()}
if len(names) < 2:
    sys.exit("cargo tree produced no dependencies; refusing to pass vacuously")
meta = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1"],
                                 capture_output=True, text=True, check=True).stdout)
checked = 0
for p in meta["packages"]:
    if p["name"] not in names or p.get("rust_version") is None: continue
    checked += 1
    if v(p["rust_version"]) > FLOOR:
        sys.exit("%s %s declares rust-version %s > 1.85" % (p["name"], p["version"], p["rust_version"]))
if checked == 0:
    sys.exit("no package in the normal graph declared a rust-version; check is vacuous")
print("msrv ok: %d normal-graph packages declare an MSRV, none above 1.85" % checked)'
# Run: python3 -c "$MSRV"
# Verified: prints "msrv ok: 12 ..." and exits 0 against the probe crate; with FLOOR lowered
# to (1,60,0) it exits 1 naming "hashbrown 0.17.1 declares rust-version 1.85.0 > 1.85". The
# two vacuity guards exist because both inputs can legitimately come back empty.
```

The spawn check is written outside a table cell because `|` cannot appear unescaped in a
Markdown table and an escaped `\|` inside an ERE matches a *literal* pipe, so the check
would pass against the very code it is meant to catch.

```sh
# SPAWN — no spawn API anywhere in the tree, and no process API at all in the new module.
# `std::process` alone is NOT usable tree-wide: src/lib.rs legitimately calls
# std::process::id and src/main.rs calls std::process::exit, neither of which is a spawn.
! grep -rnE 'process::Command|Command::new|Stdio' src/
[ -f src/schema.rs ] && ! grep -nE 'std::process|Command|Stdio' src/schema.rs
# The `[ -f ... ]` guard is load-bearing: grep exits 2 for a missing file and `!` turns that
# into a pass, so a renamed or split module would silently stop being checked. Verified
# three ways on this tree: the tree-wide form is clean at HEAD; both forms fire on a planted
# `use std::process::Command;` in a copy of src/; and the module-scoped form correctly FAILS
# today, because src/schema.rs does not exist yet.
#
# The tree-wide half is a TASK-level check, not a frozen requirement — `subprocess-seam`
# creates `src/cli.rs` containing exactly these APIs and must rescope it. Only the
# module-scoped half is normative, in specs/schema-artifacts/spec.md. This is the same
# narrowing `repo-resolution`'s review made, applied from the start here.
#
# There is deliberately NO `! grep -rn '"openspec"' src/` check. It is already false at HEAD
# (src/state.rs and src/resolve.rs both join the literal "openspec" into paths) and this
# change necessarily adds more of them — `openspec/schemas/<name>/schema.yaml` is the path it
# reads. A program-name-literal search cannot distinguish a path component from an argv[0].
```

**Where each check stops.** `DEPS`, `GRAPH`, and `MSRV` are evidence about the *resolved
build graph* — they would not catch a `[dev-dependencies]` entry that spawns, which is why
the dependency kind is filtered explicitly rather than assumed. `SPAWN` is evidence about
the *source text* — it would not catch a spawn reached through a dependency, which is what
`GRAPH`'s exact-list equality covers from the other side. Neither is proof; the two together
plus a module that takes no environment lookup and no injected collaborator are the
argument. Stated here so no later change inherits a stronger claim than was made.

There is deliberately **no `NOTOOLS`/`NOSPAWN-RUN` PATH-stripping run** in this change.
`repo-resolution` needed one because its subject was `PATH` itself; nothing here reads an
environment variable, so stripping the environment would prove nothing that the two checks
above do not already prove, and the recipe's known traps — a dropped last entry, a trailing
colon, `env PATH=… cargo` hiding cargo itself — are cost with no matching evidence.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A change's own declaration wins over the project's | `declared_name` over two `FileText::Read` values; assert `Selection { name: "tdd", source: Change, problems: [] }`, then re-run with `change: None` and assert `Project`/`spec-driven`, so precedence is what is under test | unit | pure — no filesystem | `cargo test --all-features schema::` |
| The project declaration answers when the change declares nothing | `declared_name(Some((path, &Absent)), (path, &Read(...)))`; assert `Project` | unit | pure | `cargo test --all-features schema::` |
| Nothing declared anywhere yields the default | Both `Absent`; assert `spec-driven`, `Default`, and `problems.is_empty()` — the empty-problems clause is what rejects an implementation that records a fallback for a file that never existed | unit | pure | `cargo test --all-features schema::` |
| No change directory is supplied at all | `select` against a scratch repository, `change_dir: None`, with a `.openspec.yaml` planted **beside the repository root**; assert `Project` and that the planted file was not consulted | unit | real scratch filesystem | `cargo test --all-features schema::` |
| A blank declaration is not a declaration | `schema: "   "` and, in the same test, `schema: ""`; assert `Project`/`tdd` and exactly one problem naming `.openspec.yaml` for each | unit | pure | `cargo test --all-features schema::` |
| A declaration of the wrong type is not a declaration | Four values in one test — a nested mapping, a sequence, `null`, and a top-level scalar document; each asserts `Default` and exactly one problem. An implementation that string-formats any YAML node returns `{...}` as a name and fails | unit | pure | `cargo test --all-features schema::` |
| A file that is not valid YAML falls through and is named | `.openspec.yaml` indented with a tab; assert `Project`/`tdd`, one problem naming the file, and that the problem text contains the parser's own message rather than a generic string | unit | pure | `cargo test --all-features schema::` |
| An unreadable file falls through and is named | `select` against a scratch repository whose `openspec/config.yaml` is a **directory**; assert `Default`, one problem naming the path, and no panic | unit | real scratch filesystem | `cargo test --all-features schema::` |
| An empty or comment-only document declares nothing without complaint | `""` and `"# nothing\n"` in one test; assert `Default` and `problems.is_empty()` for both. `yaml-rust2` returns **zero documents** for each, so an implementation indexing `[0]` panics here | unit | pure | `cargo test --all-features schema::` |
| Only the first YAML document is consulted | Two `---`-separated documents declaring `tdd` then `other`; assert `tdd` and no problem | unit | pure | `cargo test --all-features schema::` |
| A name containing a path separator is rejected and falls through | `schema: ../../etc` in the change file, `schema: tdd` in the project file; assert `Project`/`tdd` and one problem containing `../../etc` | unit | pure | `cargo test --all-features schema::` |
| Every rejected shape is rejected, and a legal name with a dot is not | Six values in one test — `..`, `.`, `a/b`, `/abs`, one containing `\0`, and `v1.2-tdd`; assert `Default` plus one problem for the first five and acceptance for the sixth. Without the sixth, `is_legal_name` returning `false` unconditionally passes | unit | pure | `cargo test --all-features schema::` |
| An illegal project name still falls through to the default | `schema: ../escape` in the project file only; assert `Default` and one problem | unit | pure | `cargo test --all-features schema::` |
| A repository tree is byte-identical after selection | Snapshot paths **including directories**, bytes, and mtimes of a scratch repository through `testutil::snapshot`; run `select` twice; assert snapshot equality | unit | real scratch filesystem | `cargo test --all-features schema::` |
| A missing configuration file is not created | `select` against a repository with `openspec/` but no `config.yaml`, and a change directory that does not exist; assert neither exists afterwards and the result is `Default` | unit | real scratch filesystem | `cargo test --all-features schema::` |
| The repository's own vendored `tdd` schema loads | `load(Path::new(env!("CARGO_MANIFEST_DIR")), "tdd")`; assert name `tdd`, first artifact id `proposal`, an artifact `tasks`/`tasks.md`, an artifact `specs` whose `generates` is the glob verbatim, and no problem. Asserts stable properties only, never the exact list, because the file is graft-vendored and may gain an artifact | unit | **real repository tree, read-only** | `cargo test --all-features schema::` |
| Artifact order is the file's order, not alphabetical | `parse` a fixture listing `zeta`, `alpha`, `middle`; assert exactly that order. A sorted implementation returns `alpha` first and a reversed one returns `middle` first — a two-artifact fixture distinguishes neither | unit | pure | `cargo test --all-features schema::` |
| A schema that is not vendored is reported as such and not as broken | `load` a scratch repository holding only `openspec/schemas/tdd/`, asking for `spec-driven`; assert `Err(LoadError::NotVendored { path })` by **variant**, and that the path names `openspec/schemas/spec-driven/schema.yaml` | unit | real scratch filesystem | `cargo test --all-features schema::` |
| A schema directory with no `schema.yaml` is not vendored | `openspec/schemas/half/` created empty; assert `NotVendored`, not `Unreadable` | unit | real scratch filesystem | `cargo test --all-features schema::` |
| A `schema.yaml` that cannot be read is reported as unreadable | `openspec/schemas/odd/schema.yaml` created as a **directory**; assert `Unreadable` carrying the I/O message, and no panic. This is the scenario that separates `NotVendored` from `Unreadable`, which a `Path::exists()`-based implementation conflates | unit | real scratch filesystem | `cargo test --all-features schema::` |
| The directory name and the file's `name:` key disagreeing is recorded | `openspec/schemas/renamed/schema.yaml` declaring `name: original`; assert `Schema::name == "renamed"` and exactly one problem containing both words | unit | real scratch filesystem | `cargo test --all-features schema::` |
| Only the first YAML document of the schema file is used | Two `---`-separated schema documents; assert the artifact list is exactly the first document's | unit | pure | `cargo test --all-features schema::` |
| `apply.tracks` selects an artifact whose id is not `tasks` | Fixture with `checklist`/`tasks.md` and `tasks`/`notes.md`, `apply.tracks: tasks.md`; assert `tasks == Some(checklist)` **and** `!= Some(tasks)`. This is the headline scenario: an id-first implementation — which is what `SPEC.md` as written would have produced — fails it | unit | pure | `cargo test --all-features schema::` |
| An absent `apply` block falls back to the artifact with id `tasks` | Fixture with `proposal`/`proposal.md` and `tasks`/`checklist.md`, no `apply:`; assert `tasks` is selected and its `generates` is `checklist.md`, so the assertion is on the whole artifact and not on a name that both rules would produce | unit | pure | `cargo test --all-features schema::` |
| An `apply` block without `tracks`, and an explicit `tracks: null`, both fall back to the id | Two fixtures asserted in one test. They differ in the parse tree — `BadValue` versus `Null` — and an implementation branching on key presence rather than on `is_null()` behaves differently for them | unit | pure | `cargo test --all-features schema::` |
| A `tracks` value matching nothing yields no tasks artifact even when an id `tasks` exists | Fixture with `tasks`/`tasks.md` and `design`/`design.md`, `apply.tracks: nowhere.md`; assert `tasks.is_none()` and one problem naming `nowhere.md`. An implementation that falls back to the id on a miss returns `Some(tasks)` — and that fallback is exactly what the CLI's `find` does not do | unit | pure | `cargo test --all-features schema::` |
| A schema with neither a `tracks` match nor an id `tasks` loads without one | Fixture with only `proposal` and `design`; assert the schema loads with both artifacts, `tasks.is_none()`, and one problem. An implementation returning `Err` here fails closed | unit | pure | `cargo test --all-features schema::` |
| `tracks` matches on `generates`, not on a filename suffix | Fixture with `a`/`sub/tasks.md` and `b`/`tasks.md`, `apply.tracks: tasks.md`; assert `b`. An implementation comparing `Path::file_name` returns `a`, which is first in the list | unit | pure | `cargo test --all-features schema::` |
| A `tracks` value of the wrong type falls back to the id | `apply.tracks` as a sequence, with an id `tasks` present; assert `tasks` selected and one problem naming the key | unit | pure | `cargo test --all-features schema::` |
| Entries missing `id` or `generates` are skipped and the rest survive | Four-entry fixture; assert the list is exactly `proposal`, `tasks`, exactly two problems naming positions 1 and 2, and that the tasks artifact is still found | unit | pure | `cargo test --all-features schema::` |
| A blank `id` or `generates` is skipped like an absent one | `id: "  "` and `generates: ""` alongside one valid artifact; assert one artifact and two problems | unit | pure | `cargo test --all-features schema::` |
| An entry that is not a mapping is skipped | A bare string and a nested sequence in `artifacts:`; assert one artifact, two problems, no panic | unit | pure | `cargo test --all-features schema::` |
| A `generates` that escapes the change directory is skipped | `../outside.md`, `/etc/passwd`, `a/../../b.md` rejected; `specs/**/*.md` and `sub/dir/file.md` accepted, all in one test. Without the accepted pair, a predicate rejecting every value containing `/` or `.` passes | unit | pure | `cargo test --all-features schema::` |
| Unused keys being absent or malformed does not make an entry unusable | One artifact with only `id` and `generates`, one with `requires` as a string; assert both load and no problem is recorded. An implementation validating the whole CLI artifact schema fails this and would fail closed on a schema the plugin can render | unit | pure | `cargo test --all-features schema::` |
| A schema whose every entry is unusable is invalid rather than empty | Two entries, neither with an `id`; assert `Err` (→ `Invalid`), not an `Ok` with an empty artifact list | unit | pure | `cargo test --all-features schema::` |
| Invalid YAML is reported with the parser's own message | Tab-indented schema file; assert `Invalid` naming the path, the problem carrying the parser message, and no panic | unit | pure for `parse`; real scratch filesystem for the `LoadError` mapping | `cargo test --all-features schema::` |
| An empty, comment-only, or non-mapping document is invalid | Four inputs in one test — empty, comment-only, `just text`, and a top-level sequence; assert `Err` for each and no panic. The first two return **zero documents** from `yaml-rust2`, so an implementation indexing `[0]` panics | unit | pure | `cargo test --all-features schema::` |
| An absent, non-sequence, or empty `artifacts:` key is invalid | Three inputs in one test — no key, `artifacts: nope`, `artifacts: []`; assert `Err` for each, with the empty-sequence case asserted explicitly | unit | pure | `cargo test --all-features schema::` |
| Flow style, anchors, and CRLF all parse | Three inputs in one test; assert two ordered artifacts and `tasks` for the flow-style one, the aliased artifact for the anchor one, and byte-for-byte equal results for the `\r\n` and `\n` forms. **This is the scenario that distinguishes a YAML parser from a line scanner**, and it is the test that fails if the dependency is ever replaced with a hand-rolled subset | unit | pure | `cargo test --all-features schema::` |
| A schema file whose block scalars mimic structure still parses correctly | Fixture whose `apply.instruction: \|` block contains `- id: fake`, `generates: fake.md`, and `tracks: fake.md` at the same column an artifact's keys use; assert exactly the two real artifacts and the real tracks target. Modelled on the vendored `tdd` schema, whose `apply.instruction` content genuinely sits at that column | unit | pure | `cargo test --all-features schema::` |
| A repository tree is byte-identical after loading | Snapshot including directories, bytes, and mtimes; `resolve` twice, then once more naming a schema that is not vendored; assert equality after all three | unit | real scratch filesystem | `cargo test --all-features schema::` |
| The schema module names no process API | The `SPAWN` block above, both halves | command check | real `grep` over `src/` | see `SPAWN` |
| Exactly one binary target is produced at the release path | `cargo metadata --no-deps` for `bin` targets, then `/bin/sh scripts/build.sh` and a check that `target/release/herdr-openspec` exists and is executable. Carried unchanged from the live requirement and re-verified because the dependency set moved | command check | real `cargo`, real `sh` | `cargo metadata --no-deps --format-version 1`; `/bin/sh scripts/build.sh` |
| The declared dependency set is exactly the argued crates | The `DEPS` filter | command check | real `cargo metadata`, real `python3` | `cargo metadata --no-deps --format-version 1 \| python3 -c "$DEPS"` |
| Every package in the normal build graph declares an MSRV no higher than the crate's | The `MSRV` filter | command check | real `cargo tree`, `cargo metadata`, `python3` | `python3 -c "$MSRV"` |
| The resolved build graph is small and proc-macro-free | The `GRAPH` block, plus an explicit `grep` for `syn`, `quote`, `proc-macro2`, `serde_derive`, and `encoding_rs` — kept even though the exact-list equality implies it, so a later relaxation of the list to a subset still catches a proc macro | command check | real `cargo tree` | see `GRAPH` |
| Each dependency is genuinely needed rather than incidental | Remove `toml` from `Cargo.toml`, confirm `cargo build` fails; restore; remove `yaml-rust2`, confirm `cargo build` fails; restore both and confirm `cargo build --locked` and `cargo test --all-features` are green. Edits are made and reverted in the working tree with the file's content restored from `git show HEAD:Cargo.toml`, never with `git checkout` on a tree holding other uncommitted work | command check | real `cargo`, real `git` (read-only) | `cargo build` |

A caution about the Command column: `cargo test --all-features schema::` exits 0 when the
filter matches nothing — a nonsense filter reports "0 passed … N filtered out" and returns
0. The filtered form is a convenience for running one group; the gate is the unfiltered
`cargo test --all-features` in the final group, and the first RED task requires checking
that the filter actually names the new tests.

## Decisions

**A YAML parser is added rather than a hand-rolled subset.** This is the change's central
decision and it was made on measured evidence, not preference.

*Hand-roll the narrow subset.* Rejected. The argument for it is real — only five facts are
needed (`schema:`, and per artifact `id` and `generates`, plus `apply.tracks`) — and it is
the argument `repo-resolution` accepted when it rejected `glob` and `which`. It does not
transfer. `split(':')` and one `read_dir` are total functions with obvious semantics; YAML
is a hundred-page specification, and the two files being read are **not authored by this
project**. `openspec/config.yaml` is hand-edited by the user and this repository's own
already contains a block scalar and nested quoted mappings; `schema.yaml` may come from any
schema author. Concretely, the vendored `tdd` schema's `apply.instruction` block scalar is
indented to **four spaces** — the same column as `    generates:` under an artifact — and
contains markdown headings, fenced code, and `key: value`-shaped prose. A block-scalar-aware
indentation scanner handles that, but it still mis-reads flow style, anchors and aliases,
`---` document markers, and quoted keys, all of which are legal here. The failure mode is
the bad one: a *silently wrong tab order*, with nothing to notice it. A parse error at least
degrades visibly.

*`serde_yaml`.* Rejected: `0.9.34+deprecated`, archived and unmaintained. `SPEC.md`'s stack
line still names it, which is corrected by this change.

*`serde_yml`.* Rejected: `0.0.13`, published as a deprecated compatibility shim.

*`serde_yaml_ng` / `serde_norway`.* Rejected. Both are maintained forks, both are
proc-macro-free, and either would work. Both drag eight transitive crates including
`unsafe-libyaml` — a machine translation of libyaml's C into `unsafe` Rust — and pull
`serde` and `indexmap` for a job that needs an untyped tree walk over five keys. Deriving
types would additionally reintroduce `serde_derive`, a proc macro the live `plugin-build`
requirement forbids, so the serde integration that is their whole point could not be used
anyway.

*`saphyr`.* Rejected on a hard constraint, not a preference: `saphyr-parser` depends on
`thiserror`, which pulls `thiserror-impl`, `syn`, `quote`, and `proc-macro2` into the
**normal** build graph. `plugin-build` forbids exactly that. It is also at `0.0.12`.

*`yaml-rust2` 0.12.0.* **Chosen.** Four transitive crates (`arraydeque`, `hashlink`,
`hashbrown`, `foldhash`), pure Rust with no `unsafe` in its own source, no proc macro, no C.
`default-features = false` drops `encoding_rs` — the `encoding` feature exists only for
`load_from_bytes` BOM and UTF-16 detection, and this crate reads through
`std::fs::read_to_string`. MIT OR Apache-2.0, `rust-version` `1.85.0`, published
2026-08-18, 14.6M recent downloads. Verified against the real vendored `tdd` schema, the
CLI's bundled `spec-driven` schema, and this repository's own `config.yaml`, all three
parsed correctly, plus fourteen adversarial inputs: it never panicked, returned `Err` on
tabs and on duplicate keys, returned **zero documents** for empty and comment-only input,
and gave `is_badvalue()` rather than a panic for every missing key and every index into a
non-mapping.

**The public API exposes no YAML type.** `parse` takes `&str` and returns a `Schema`.
`yaml-rust2` appears in no signature, so replacing it later is a change to one module's
body. This is what makes the dependency a decision that can be revisited cheaply.

**Three `LoadError` variants rather than one message.** `changes-from-cli` must ask the CLI
for a schema that is *not vendored* and must not ask it for one that is present and broken —
asking would produce the same failure more slowly, and would hide a malformed file behind a
CLI error. A consumer distinguishing those by matching substrings in a problem string breaks
the first time the message is reworded. This is the same argument `resolve::BinSource` makes:
the *reason* is part of the contract, not a debugging aid.

**No injected hook for the CLI fallback, unlike `repo-resolution`'s step 4.** The asymmetry
is deliberate and is worth stating so nobody reads it as an oversight. `repo-resolution`
injected a `&dyn Fn() -> Option<PathBuf>` because `npm prefix -g` was **step 4 of an ordered
chain**: without it, steps 1–3's ordering was the only thing ever exercised, and
`subprocess-seam` would have had to re-derive where step 4 belonged. Here there is no chain.
The disk tier either produces a schema or names one of three reasons, and the CLI tier is a
strictly later consumer that will call `load` again against the directory
`openspec schema which` reports. A hook would be a parameter with one production binding,
one test binding, and no ordering to protect — ceremony rather than a seam. The obligation
is instead handed over on the roadmap, where a `changes-from-cli` implementer will read it.

**`select` and `load` are separate entry points, with `resolve` as the convenience.**
`changes-from-files` iterates changes, and each change may declare its own schema. A single
`resolve(repo, Some(change_dir))` per change would re-read and re-parse a 663-line
`schema.yaml` once per change. Splitting the two lets that caller select per change and load
once per *distinct name* without this module owning a cache, a lifetime, or an invalidation
rule — which is why "no caching" is a non-goal rather than a deferral.

**The per-change `.openspec.yaml` override is in scope, widening the roadmap row.**
`IMPLEMENTATION-ORDER.md` says only "parse `openspec/config.yaml` for `schema:`". The CLI
resolves a change's own `.openspec.yaml` first and defaults to `spec-driven` last, and
`SPEC.md`'s own list view and degraded-states table are already per-change ("`learning-tool`
declares schema `outside-in-tdd`, which the installed CLI rejects"). Omitting the override
would make the dashboard render the repository schema's tabs for a change that declares a
different one — wrong, and silently so. The alternative, leaving it to `changes-from-files`,
would have that change either re-open `schema` or invent the rule. It is ~15 lines and four
scenarios here. The roadmap row is corrected rather than the behaviour dropped.

**A `tracks` miss does not fall back to the id.** This is the subtlest rule in the change and
it is a mirror of the CLI, not a choice: `findTrackedTasksArtifact` returns
`artifacts.find(a => a.generates === tracks)` and stops. A schema declaring
`tracks: nowhere.md` has no tasks artifact even when an artifact with id `tasks` exists.
Falling back would make the plugin disagree with the CLI about which file holds a change's
progress — the two sources are supposed to converge, and `SPEC.md`'s dual-source model rests
on that. The scenario asserts the non-fallback explicitly.

**An unusable artifact entry is skipped; an unusable schema is not.** One entry missing an
`id` costs one tab; the rest of the tab bar is still correct and useful, so skipping with a
named problem is the degrade. A schema whose every entry is unusable, or whose `artifacts:`
is absent or empty, would render a detail view with zero tabs and no explanation — that is
`Invalid`, which at least says why. The line is drawn where the result stops being usable.

**Keys the plugin does not render are not validated.** The CLI's Zod schema requires
`description` and `template` on every artifact; this module ignores both. Rejecting a schema
over a field the dashboard never shows would fail closed on a schema the plugin can render
perfectly, which `SPEC.md` forbids outright. `generates` *is* validated for traversal,
because `changes-from-files` joins it onto a change directory — the same
relative-path constraint the CLI applies, applied for a reason this crate has of its own.

**`generates` is carried verbatim, not resolved.** `specs/**/*.md` stays a glob. Turning it
into a path — or into a directory-artifact flag — is `changes-from-files`' decision, made
with a change directory in hand, and it is a decision this module has no information to make.

**No filesystem abstraction, and no environment lookup.** The scratch-tree technique
`plugin-config` established and `repo-resolution` reused is what these tests need; faking a
filesystem would mean testing the fake. And unlike `config`, `state`, and `resolve`, nothing
here reads an environment variable at all — the repository root arrives as an argument from
`find_repo` — so this module takes no `&dyn Fn(&str) -> Option<String>`. Adding one for
symmetry would be an unused parameter.

**`Schema::tasks` is a cloned `Artifact`, not an index.** An index into `artifacts` is
correct only while nobody filters the list, and `list-view`'s `/` filter and `detail-view`'s
tab numbering are both in scope for later changes. Two small `String`s are cheaper than that
class of bug, and a whole-value `assert_eq!` is what the scenarios want anyway.

## Risks / Trade-offs

- **A dependency is added to a crate whose live spec pins the dependency set to one crate.**
  → Accepted deliberately: that requirement exists to force the argument, and the argument is
  above. The delta is written as REMOVED + ADDED rather than MODIFIED because the count is in
  a scenario *heading* and `openspec validate --strict` rejects a MODIFIED requirement that
  omits a scenario name the live spec still carries. Recorded in proposal.md so the shape is
  not mistaken for a retired capability; `plugin-build` keeps its two other requirements.
- **`yaml-rust2`'s `rust-version` is `1.85.0` — exactly this crate's floor, and the tightest
  in the graph.** A future `yaml-rust2` release that raises it breaks the crate's declared
  MSRV silently, because cargo will happily resolve it. → The MSRV clause had prose and no
  scenario; this change adds one, so the next bump fails a gate rather than a contributor's
  build.
- **`yaml-rust2` is the stable-numbered sibling of `saphyr`, which the same maintainer
  positions as its successor.** → Both were released on the same day (2026-08-18), and
  `yaml-rust2` carries 14.6M recent downloads against `saphyr`'s 757k, so it is being
  released in lockstep rather than abandoned. `saphyr` is unusable here today regardless, on
  the proc-macro constraint. The migration cost if that changes is bounded by the decision
  above: no YAML type appears in any signature, so it is one module's body. Resolution point:
  whenever `plugin-build`'s graph check fails after a `cargo update`.
- **`yaml-rust2` rejects duplicate mapping keys; the JavaScript `yaml` parser the CLI uses
  accepts them, last-wins.** A schema with a duplicated key therefore loads for the CLI and
  is `Invalid` for the plugin. → Accepted, and it is the safe direction: the plugin degrades
  visibly with the parser's message rather than silently picking a different value than the
  CLI did. Recorded here so a future bug report is diagnosable rather than mysterious.
- **One unit scenario reads this repository's own graft-vendored `openspec/schemas/tdd/`.**
  A `graft sync` that changes that file could break a test that has nothing to do with the
  change being made. → The scenario asserts only stable properties — parses, non-empty, first
  artifact `proposal`, an artifact `tasks`/`tasks.md`, the `specs` glob verbatim — never the
  exact list, so adding an artifact upstream does not break it. It is read through
  `env!("CARGO_MANIFEST_DIR")`, never the process working directory. The scenario is worth
  the coupling: it is the only test that proves the model matches the file the plugin will
  actually meet, and it is the test that would have caught `role: tasks` before a line was
  written.
- **Reading `openspec/config.yaml` per change is O(changes) file reads.** → Bounded and
  small; `select`/`load` are split so a caller can avoid the expensive half, and the
  expensive half is the 663-line parse, not the 83-line one. If it ever matters,
  `changes-from-files` caches by name — it has the loop, so it has the information.
- **`Schema::tasks` duplicates two `String`s already present in `artifacts`.** → Deliberate;
  see Decisions. The alternative is an index that desyncs under filtering.
- **The no-spawn evidence is source text plus the build graph, not instrumentation.** A spawn
  reached through a dependency would pass `SPAWN`. → `GRAPH`'s exact-list equality is the
  other half: the five added packages are named, and any sixth fails the check. Stated so no
  later change inherits a stronger claim.
- **`Cargo.lock` moves, so the `repo-resolution` habit of asserting it byte-identical does
  not apply here.** → Inverted deliberately: the task asserts the lock changed by exactly the
  expected additions and that `cargo build --locked` succeeds at the landing commit, which is
  what the live requirement actually demands.

## Migration Plan

None applies. No data is stored, no format changes, and no consumer exists yet — rolling
back is reverting the commit, including the `Cargo.toml` and `Cargo.lock` lines. Deploy order
is irrelevant: the change is a library addition to a plugin built from source at install
time. The one externally visible effect is that a fresh `herdr plugin install` compiles five
more crates, which `scripts/build.sh` handles with no edit.

## Open Questions

None blocking. Three settled here rather than left open:

- *Should the module ask the CLI when a schema is not vendored?* No, not in this change —
  that needs `subprocess-seam`. But the **command it will use** is settled and recorded,
  because `SPEC.md`'s `openspec schema` does not exist: `openspec schema which <name> --json`
  returns `{"name","source","path","shadows"}`, and `path` is the schema *directory* whose
  `schema.yaml` this same `parse` then reads. Its stdout is clean JSON; the "experimental"
  note goes to stderr. `openspec status --change <n> --json` separately returns the ordered
  `artifacts[]` per change, which `changes-from-cli` already plans to call.
- *Should `Schema` carry `requires` so the tab bar could show dependency order?* No.
  `SPEC.md` says the `artifacts` list *is* the tab order; a second ordering with no
  requirement behind it is a field nothing reads.
- *Should an unknown schema name be validated against the vendored directory listing before
  loading?* No. `load` reads one path and reports `NotVendored` when it is absent — listing
  `openspec/schemas/` first would be a second filesystem call to produce a worse message, and
  the CLI's own "available: [...]" hint belongs to the CLI tier that will have it.
