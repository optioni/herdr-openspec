## Context

`repo-resolution` landed `find_repo`, so the crate can say *where* the repository is.
Nothing can yet say what is inside a change. The detail view's tab bar is the schema's
artifact list in schema order, and the tasks tab is one particular member of that list, so
`changes-from-files` cannot build a `Change` and `detail-view` cannot render a tab until
something reads the schema. This is the second row of Phase 2 in
`openspec/IMPLEMENTATION-ORDER.md`, and like every Phase 2 row it is a pure transformation:
no terminal, no subprocess, no writes.

**Facts read off the real OpenSpec CLI and the real vendored schema before writing this
design.** Every one of them changed a decision, and two of them make `SPEC.md` wrong. The
CLI on the workflow's `PATH` here is **1.11.0** (nvm `v24.20.0`); the machine also carries
1.12.0 under `v24.18.0` and 1.9.0 under `v24.19.0`, and every fact below was re-confirmed
against 1.12.0 during review with no drift — so none of these is a quirk of one release.

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
  the CLI tier comes after all of it. What this change ships instead is `load_dir`, which
  takes the *directory* rather than a repository root, so Phase 3 is one call rather than a
  re-derivation. See Decisions.
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
| Test fixtures | `src/lib.rs` → `testutil`, **reused unchanged** | `ScratchDir` plus `std::fs::write` covers every fixture these scenarios need; `write_with_mode`, `symlink`, `canonical`, and `snapshot` already exist and none is extended. `snapshot` already records directory entries — `repo-resolution` added that — so the two containment guards see a `create_dir_all`. If a fixture turns out to want a builder it is added here rather than inside a test module (tasks.md 1.5) |
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
/// `schema:`.
///
/// `Ok(None)` — the normal, silent case — when the text holds no document at
/// all, **or** when its first document is a mapping carrying no `schema:` key.
/// That second branch is the ordinary shape of a repository that pins nothing
/// and of a `.openspec.yaml` carrying only `created:`, and it is the one an
/// implementation gets wrong by accident: `yaml-rust2` yields `Yaml::BadValue`
/// for a missing key, whose `as_str()` is `None` — indistinguishable from
/// `schema: [a]` or `schema: 42` through a string accessor. The wrong-type
/// branch must therefore be keyed on *the node exists and is not a
/// `Yaml::String`* (`!is_badvalue()`), never on `as_str().is_none()`.
///
/// `Err(reason)` when the text is not valid YAML, when its first document is
/// not a mapping, or when `schema:` is present with a non-string value —
/// including an explicit `null`.
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

/// Read a path into a `FileText`, mapping `NotFound` to `Absent` and every
/// other failure — including a UTF-8 decode failure — to `Unreadable`.
///
/// `pub(crate)`, like `FileText` and `declared_name`, and deliberately so:
/// `changes-from-files` lives in `src/changes.rs`, the same crate, and the
/// per-change loop is exactly the caller that wants to read
/// `openspec/config.yaml` once and hand the same `FileText` to `declared_name`
/// for every change. A private `read_file` would leave that caller with only
/// `select`, which re-reads both files on every call — the cost the Risks
/// section names — and the escape hatch this design credits would not exist.
pub(crate) fn read_file(path: &Path) -> FileText;

/// Which schema applies to `repo`, or to `change_dir` inside it.
pub fn select(repo: &Path, change_dir: Option<&Path>) -> Selection;

/// Read and parse `<dir>/schema.yaml`. `dir` is *any* schema directory: the
/// repository's own, `$XDG_DATA_HOME/openspec/schemas/<name>`, or the absolute
/// path `openspec schema which <name> --json` reports for a schema the CLI
/// package ships. `name` is only what the result is labelled with and what the
/// file's `name:` key is compared against.
pub fn load_dir(dir: &Path, name: &str) -> Result<ParsedSchema, LoadError>;

/// The repository tier: `load_dir(&repo.join("openspec").join("schemas").join(name), name)`.
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
- **`parse`'s `Err(String)` is a display string, never a value to match on.** It exists
  only to become `LoadError::Invalid { reason }` and to be shown to a user. The
  three-variant argument below is about the *category* of failure, which is what a consumer
  branches on; the sentence inside a category is prose. A consumer matching substrings in
  it is making the mistake `LoadError` exists to prevent.
- **`LoadError` has three variants rather than one string** because `changes-from-cli` must
  branch on `NotVendored` specifically, and a consumer matching substrings in a message is
  a consumer that breaks when the message is reworded.
- **`Schema::name` is the directory segment**, because that is what `openspec schema which`
  keys on and what the user configured. A disagreement with the file's `name:` is a problem,
  not a failure — and only a **present, non-blank string** `name:` can disagree. An absent
  `name:` (`Yaml::BadValue`) or one that is blank or not a string records nothing. This is
  not a detail: almost every fixture in this change omits `name:`, and a dozen of them assert
  `problems.is_empty()`, so an implementation comparing
  `dir != doc["name"].as_str().unwrap_or("")` turns half the suite red. Only the mismatch
  fixture carries a `name:` key at all.
- **`Schema::tasks` is a clone, not an index, and it carries an invariant.** When `Some`, it
  is always equal to one element of `artifacts`; `parse` is the only producer and it never
  builds one that is not. A consumer that filters `artifacts` — `list-view`'s `/` filter and
  `detail-view`'s tab numbering are both in scope later — must re-derive or clear it rather
  than assume the two stay consistent, because the struct as declared permits a `tasks` that
  is no longer in the list. An index would desync the same way and more silently; a
  `tasks_id: Option<String>` key would not desync at all, and is rejected only because every
  scenario here asserts a whole `Artifact` with `assert_eq!` and a bare id would push a
  lookup into each of them. The invariant is stated because it is the price of that choice.
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
| Filesystem — this repository's own tree | **real, read-only.** `DEPS`, `GRAPH`, `MSRV` and `SPAWN` read `Cargo.toml`, `Cargo.lock` and `src/`. `NEEDED` — the only check that edits a manifest — runs against a **copy** of the crate under `std::env::temp_dir()` and never touches the working tree, because `cargo build` rewrites `Cargo.lock` during resolution. `BUILD` writes only to the gitignored `target/`. Group 1 does of course edit `Cargo.toml` and `Cargo.lock`; that is the change, not a check | **real, read-only, exactly one scenario**: "The repository's own vendored `tdd` schema loads" reads `openspec/schemas/tdd/schema.yaml` through `env!("CARGO_MANIFEST_DIR")` — never through the process working directory, which a test runner may change |
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

- **unit** — `cargo test --all-features schema::`, functions in `src/schema.rs`. **44 of the
  50 scenarios live here**, and 32 of those reach no filesystem at all: `schema_key`,
  `is_legal_name`, `declared_name`, and `parse` are pure functions over `&str` and
  `FileText` values.
- **command check** — a shell command run once, its exit status inspected, recorded as a
  task. **Six scenarios:** `plugin-build`'s five, which are facts about the build graph
  and the build script rather than about running code, and "The schema module names no
  process API", which is a fact about the source text.

**This change does not take the outer-loop acceptance test.** Its outermost surface is a
library API. `src/main.rs` is untouched, the binary's observable behaviour is still the `ui`
banner, and no caller consumes a `Schema` until `changes-from-files`. An acceptance test
would have to drive an entry point that ships to nobody — the same argument `plugin-config`
and `repo-resolution` made, for the same reason. The unit tier reaches every collaborator
these functions have: a real filesystem for three files, and a real YAML parser.

Shorthand used below. Every command is written for BSD userland, and **every one was run on
this machine with a negative control** before being written here — the control is named under
each block. Nothing is described in prose that a task is expected to spell out itself: the
one check in `repo-resolution` that went wrong did so in its spelling, not its intent.

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
#
# `cargo metadata` cannot distinguish `features = []` from an omitted `features` key — both
# report `[]` — so the delta spec's "written out on the face of the manifest" clause is
# checked by reading Cargo.toml, not by this filter. Stated so nobody believes DEPS covers it.
```

```sh
# GRAPH — the resolved normal build graph, package for package, on each supported triple.
#
# NOT `cargo tree --target all`: verified that on this graph it additionally lists syn,
# quote, proc-macro2, serde_derive and unicode-ident, reached through serde_core's optional
# `derive` feature, none of which is ever built. An exact-list check against that output
# would be unsatisfiable. `--target <triple>` needs no installed std; it is a resolution.
#
# `if grep ...; then exit 1; fi` rather than `! grep ...` throughout: POSIX says `set -e`
# ignores a command prefixed with `!`, so a sequence of bare `! grep` lines lets an early
# violation be masked by a later line succeeding (verified: exit 0 with a planted spawn).
GRAPH_WANT='arraydeque foldhash hashbrown hashlink serde_core serde_spanned toml toml_datetime toml_parser toml_writer winnow yaml-rust2'
for t in aarch64-apple-darwin x86_64-apple-darwin \
         aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu; do
  out=$(cargo tree -e normal --target "$t" --prefix none) \
    || { echo "FAIL: cargo tree failed for $t" >&2; exit 1; }
  got=$(printf '%s\n' "$out" | awk 'NF {print $1}' | grep -vx 'herdr-openspec' \
        | sort -u | tr '\n' ' ' | sed 's/ $//')
  [ -n "$got" ] || { echo "FAIL: empty graph for $t; refusing to pass vacuously" >&2; exit 1; }
  [ "$got" = "$GRAPH_WANT" ] || { echo "FAIL: $t graph mismatch: [$got]" >&2; exit 1; }
  if printf '%s\n' "$out" | awk 'NF {print $1}' \
     | grep -qxE 'syn|quote|proc-macro2|serde_derive|encoding_rs'; then
    echo "FAIL: $t graph contains a proc macro or encoding_rs" >&2; exit 1
  fi
done
echo "graph ok: identical on all four supported triples, no proc macro, no encoding_rs"
# `grep -qxE` matches the whole field, so a future package whose name merely *contains* one
# of these strings is not a false failure. `grep -vx 'herdr-openspec'` on the root name means
# a package rename fails the check rather than silently passing.
# Verified: exit 0 against a probe crate carrying both dependencies, identical on all four
# triples; exit 1 naming the mismatch against this crate at HEAD. A probe carrying a
# `[target."cfg(windows)".dependencies] thiserror` entry still passes — correctly, because
# the manifest declares `platforms = ["macos", "linux"]` and that dependency is in no
# supported build. That is the exact edge of this check and it is stated rather than implied.
```

```sh
# MSRV — every package in the NORMAL graph on the supported triples, against the floor read
# from Cargo.toml rather than from a second literal copy of it. The scenario is a claim about
# the relationship between two values; a hardcoded floor only ever reads one of them.
MSRV='import json, subprocess, sys
TARGETS = ["aarch64-apple-darwin", "x86_64-apple-darwin",
           "aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu"]
def v(s):
    p = [int(x) for x in s.split(".")]
    while len(p) < 3: p.append(0)
    return tuple(p[:3])
meta = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1"],
                                 capture_output=True, text=True, check=True).stdout)
root = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                                 capture_output=True, text=True, check=True).stdout)["packages"][0]
if not root.get("rust_version"):
    sys.exit("root package declares no rust-version; the floor is undefined")
FLOOR = v(root["rust_version"])
keys = set()
for t in TARGETS:
    r = subprocess.run(["cargo", "tree", "-e", "normal", "--target", t, "--prefix", "none"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit("cargo tree failed for %s: %s" % (t, r.stderr.strip()))
    for line in r.stdout.splitlines():
        f = line.split()
        if len(f) >= 2: keys.add((f[0], f[1].lstrip("v")))
if len(keys) < 2:
    sys.exit("cargo tree produced no dependencies; refusing to pass vacuously")
checked, at_floor = 0, []
for p in meta["packages"]:
    if (p["name"], p["version"]) not in keys or p.get("rust_version") is None: continue
    checked += 1
    if v(p["rust_version"]) > FLOOR:
        sys.exit("%s %s declares rust-version %s > %s"
                 % (p["name"], p["version"], p["rust_version"], root["rust_version"]))
    if v(p["rust_version"]) == FLOOR: at_floor.append(p["name"])
if checked == 0:
    sys.exit("no package in the normal graph declared a rust-version; check is vacuous")
print("msrv ok: floor %s from Cargo.toml; %d normal-graph packages declare an MSRV; "
      "at the floor: %s" % (root["rust_version"], checked, sorted(at_floor)))'
# Run: python3 -c "$MSRV"
# Matching on (name, version) rather than on name alone: a dev- or build-dependency can
# resolve a second version of a normal-graph package, whose MSRV would otherwise be checked
# as though it were in the build. The two vacuity guards exist because both inputs can come
# back empty. The failure message formats the floor rather than hardcoding it, so a negative
# control reads correctly.
# Verified: prints "msrv ok: floor 1.85 from Cargo.toml; 12 normal-graph packages declare an
# MSRV; at the floor: ['hashbrown', 'hashlink', 'serde_spanned', 'toml', 'toml_datetime',
# 'toml_parser', 'toml_writer', 'yaml-rust2', 'herdr-openspec']" against the probe crate; with
# the floor forced to 1.60 it exits 1 naming "hashbrown 0.17.1 declares rust-version 1.85.0
# > 1.60". Note nine packages tie at 1.85 — yaml-rust2 is not uniquely the tightest, which
# is why the spec scenario reports the set rather than naming one crate.
```

```sh
# BUILD — one bin target, and a build script that actually ran.
cargo metadata --no-deps --format-version 1 | python3 -c 'import json, sys
p = json.load(sys.stdin)["packages"][0]
bins = sorted(t["name"] for t in p["targets"] if "bin" in t["kind"])
if bins != ["herdr-openspec"]: sys.exit("bin targets: %r" % (bins,))'
rm -f target/release/herdr-openspec
/bin/sh scripts/build.sh || { echo "FAIL: scripts/build.sh exited non-zero" >&2; exit 1; }
[ -x target/release/herdr-openspec ] \
  || { echo "FAIL: binary missing or not executable after build" >&2; exit 1; }
# The `rm -f` is what turns this from an observation into a check: the binary survives from
# any earlier build or `make build`, so without it the existence assertion is satisfied by a
# leftover regardless of what the script did — and scripts/build.sh has a real failure path
# ("error: cargo not found", exit 1) that an existence-only check sails past.
```

```sh
# NEEDED — each dependency is load-bearing. Runs entirely in a throwaway copy.
#
# `cargo build` REWRITES Cargo.lock during resolution, BEFORE it reaches the compile error
# this check waits for: verified that removing `toml` deletes fourteen package blocks from
# the lock. An in-tree edit-and-restore of Cargo.toml alone therefore leaves the tree
# unbuildable under `--locked` and silently discards the resolution this change verified, and
# an interrupt between the edit and the restore leaves it broken with no recovery step.
R=$(pwd)
P=$(mktemp -d)/needed; mkdir -p "$P"
cp -R "$R/src" "$R/tests" "$P/"
cp "$R/Cargo.toml" "$R/Cargo.lock" "$R/rustfmt.toml" "$P/"
cd "$P"; fail=0
for dep in toml yaml-rust2; do
  cp "$R/Cargo.toml" Cargo.toml
  grep -q "^$dep = " Cargo.toml \
    || { echo "FAIL: $dep is not declared; nothing to remove" >&2; fail=1; continue; }
  grep -v "^$dep = " Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
  if cargo build >/dev/null 2>&1; then
    echo "FAIL: the crate still builds without $dep" >&2; fail=1
  else
    echo "ok: removing $dep breaks the build"
  fi
done
cd "$R"; rm -rf "$(dirname "$P")"
[ "$fail" = 0 ] || exit 1
# The `grep -q "^$dep = "` precondition is what stops the second leg passing for the wrong
# reason: without it, a manifest that never carried yaml-rust2 makes the removal a no-op and
# the build fails anyway, and the check records a pass for a step it never performed.
# Verified against this crate at HEAD: "ok: removing toml breaks the build", then
# "FAIL: yaml-rust2 is not declared; nothing to remove", exit 1, with `git status --short`
# still empty afterwards.
```

```sh
# SPAWN — no spawn API anywhere in the tree, and no process API at all in the new module.
# `std::process` alone is NOT usable tree-wide: src/lib.rs legitimately calls
# std::process::id and src/main.rs calls std::process::exit, neither of which is a spawn.
#
# Positive controls come FIRST, because a grep that cannot see the tree exits 0 under `!`
# and reports clean forever. Verified with the real BSD grep: from a directory with no src/,
# `! grep -rnE ... src/` prints "No such file or directory" and exits 0.
[ -d src ] || { echo "FAIL: src/ missing; the tree-wide grep would pass vacuously" >&2; exit 1; }
grep -q 'pub mod schema' src/lib.rs \
  || { echo "FAIL: positive control - grep cannot see src/lib.rs" >&2; exit 1; }
[ -f src/schema.rs ] \
  || { echo "FAIL: src/schema.rs missing; module check would pass vacuously" >&2; exit 1; }
if grep -rnE 'process::Command|Command::new|Stdio' src/; then
  echo "FAIL: a spawn API appears somewhere under src/" >&2; exit 1
fi
if grep -nE 'std::process|Command|Stdio' src/schema.rs; then
  echo "FAIL: a process API appears in src/schema.rs" >&2; exit 1
fi
echo "spawn ok: no spawn API under src/, no process API in src/schema.rs"
# Written as `if grep ...; then exit 1; fi`, never `! grep ...`: `set -e` is defined to ignore
# a `!`-prefixed command, so two bare `! grep` lines let a tree-wide violation be masked by a
# clean module (verified: exit 0 with a planted spawn in src/state.rs and a clean schema.rs).
# Verified five ways: fails at HEAD (no src/schema.rs); passes on a simulated post-change
# tree; fails on a plant in src/schema.rs; fails on a plant in src/state.rs with schema.rs
# clean; fails when run from a directory with no src/.
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

**And none of the six is a standing gate.** `make check` is `fmt-check lint test coverage`,
and CI runs exactly those four; `DEPS`, `GRAPH`, `MSRV`, `BUILD`, `NEEDED`, and `SPAWN` run
once each, as tasks, at the commit that lands this change. What pins the resolution
afterwards is the committed `Cargo.lock`, not a check. So a later `cargo update` that raises
an MSRV or drags in a proc macro fails **nothing** until someone re-runs these blocks — which
is why the Risks section says the resolution point is "the next change that alters the
dependency set", and does not claim a gate fires on its own. Wiring them into `make check`
would make that claim true and is deliberately out of scope here: it changes the
`quality-gates` capability, which this change otherwise leaves alone.

There is deliberately **no `NOTOOLS`/`NOSPAWN-RUN` PATH-stripping run** in this change.
`repo-resolution` needed one because its subject was `PATH` itself; nothing here reads an
environment variable, so stripping the environment would prove nothing that the two checks
above do not already prove, and the recipe's known traps — a dropped last entry, a trailing
colon, `env PATH=… cargo` hiding cargo itself — are cost with no matching evidence.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A change's own declaration wins over the project's | `declared_name` over two `FileText::Read` values; assert `Selection { name: "tdd", source: Change, problems: [] }`, then re-run with `change: None` and assert `Project`/`spec-driven`, so precedence is what is under test | unit | pure — no filesystem | `cargo test --all-features schema::` |
| The project declaration answers when the change declares nothing | `declared_name(Some((path, &Absent)), (path, &Read(...)))`; assert `Project` | unit | pure | `cargo test --all-features schema::` |
| Nothing declared anywhere yields the default | Both `Absent`; assert `spec-driven`, `Default`, and `problems.is_empty()` — the empty-problems clause is what rejects an implementation that records a fallback for a file that never existed. A second run in the same test drives `select` against a scratch directory holding no `openspec/` at all | unit | pure, plus one real scratch directory for the no-`openspec/` half | `cargo test --all-features schema::` |
| A file that declares other keys but no `schema:` records no problem | `schema_key` over a mapping carrying `context:` and `rules:` and no `schema:`, and over a `.openspec.yaml` carrying only `created:`; assert `Ok(None)` for both, then `declared_name` over the pair and assert `Default` with `problems.is_empty()`. **The load-bearing case:** an absent key is `Yaml::BadValue`, whose `as_str()` is `None` — the same observable as `schema: [a]`, which must record a problem. An implementation keyed on `as_str().is_none()` rather than on `is_badvalue()` fails here and nowhere else | unit | pure | `cargo test --all-features schema::` |
| A fallback at both sources records both problems, in order | `.openspec.yaml` blank and `openspec/config.yaml` carrying a mapping-valued `schema:`; assert `Default`, `problems.len() == 2`, the first naming `.openspec.yaml` and the second naming `config.yaml`. The only scenario where both sources fail at once, and the only one that rejects a first-wins, last-wins, or `Option<String>` implementation | unit | pure | `cargo test --all-features schema::` |
| No change directory is supplied at all | `select(repo, None)` against `<scratch>/repo` whose `openspec/config.yaml` says `schema: tdd`, with `schema: planted` written to **both** `<scratch>/repo/.openspec.yaml` (what a `change_dir = repo` bug reads) and `<scratch>/.openspec.yaml` (what a `change_dir = repo.parent()` bug reads); assert the name is `tdd` and never `planted`. Both the distinct third name and the in-repo plant are load-bearing: a planted file agreeing with the project's, or planted only outside the scratch tree, is green against every implementation. Both plants sit inside the `ScratchDir`, so drop still removes them | unit | real scratch filesystem | `cargo test --all-features schema::` |
| A blank declaration is not a declaration | `schema: "   "` and, in the same test, `schema: ""`; assert `Project`/`tdd` and exactly one problem naming `.openspec.yaml` for each | unit | pure | `cargo test --all-features schema::` |
| A declaration of the wrong type is not a declaration | Four values in one test — a nested mapping, a sequence, `null`, and a top-level scalar document; each asserts `Default` and exactly one problem. An implementation that string-formats any YAML node returns `{...}` as a name and fails | unit | pure | `cargo test --all-features schema::` |
| A file that is not valid YAML falls through and is named | `.openspec.yaml` indented with a tab; assert `Project`/`tdd`, one problem naming the file, and that the problem text contains the parser's own message rather than a generic string | unit | pure | `cargo test --all-features schema::` |
| An unreadable file falls through and is named | `select` against a scratch repository whose `openspec/config.yaml` is a **directory**; assert `Default`, one problem naming the path, and no panic | unit | real scratch filesystem | `cargo test --all-features schema::` |
| An empty or comment-only document declares nothing without complaint | `""` and `"# nothing\n"` in one test; assert `Default` and `problems.is_empty()` for both. `yaml-rust2` returns **zero documents** for each, so an implementation indexing `[0]` panics here | unit | pure | `cargo test --all-features schema::` |
| Only the first YAML document is consulted | Two `---`-separated documents declaring `tdd` then `other`; assert `tdd` and no problem | unit | pure | `cargo test --all-features schema::` |
| A name containing a path separator is rejected and falls through | `schema: ../../etc` in the change file, `schema: tdd` in the project file; assert `Project`/`tdd` and one problem containing `../../etc`. No clause asserts "nothing outside the repository was read": the API cannot observe it, and a clause that restates the previous THEN is the shape `repo-resolution`'s review removed three of | unit | pure | `cargo test --all-features schema::` |
| Every rejected shape is rejected, and a legal name with a dot is not | Six values in one test — `..`, `.`, `a/b`, `a\b`, `/abs`, one containing NUL, and `v1.2-tdd`; assert `Default` plus one problem for the first six and acceptance for the seventh. The NUL fixture's YAML source must be `schema: "a\0b"` — the double-quoted escape, i.e. the Rust literal `"schema: \"a\\0b\"\n"` — because a literal NUL in a plain scalar is silently truncated to the legal name `a` (verified) and the test then goes red for a reason unrelated to `is_legal_name`. Without the seventh value, `is_legal_name` returning `false` unconditionally passes | unit | pure | `cargo test --all-features schema::` |
| An illegal project name still falls through to the default | `schema: ../escape` in the project file only; assert `Default` and one problem | unit | pure | `cargo test --all-features schema::` |
| A repository tree is byte-identical after selection | Snapshot paths **including directories**, bytes, and mtimes of a scratch repository through `testutil::snapshot`; run `select` twice; assert snapshot equality | unit | real scratch filesystem | `cargo test --all-features schema::` |
| A missing configuration file is not created | `select` against a repository with `openspec/` but no `config.yaml`, and a change directory that does not exist; assert neither exists afterwards and the result is `Default` | unit | real scratch filesystem | `cargo test --all-features schema::` |
| The repository's own vendored `tdd` schema loads | `load(Path::new(env!("CARGO_MANIFEST_DIR")), "tdd")`; assert the artifact ids are **exactly** `[proposal, specs, design, tasks, planning-review]` in order, the name is `tdd`, `tasks` is selected via the file's real `apply.tracks`, `specs`'s `generates` is the glob verbatim, and `problems.is_empty()`. The exact list rather than a containment check: a parser that loses `design` or `planning-review` — the two longest `instruction:` block scalars, and exactly what a block-scalar defect would eat — satisfies every "contains" clause. `CARGO_MANIFEST_DIR`, never the process working directory | unit | **real repository tree, read-only** | `cargo test --all-features schema::` |
| Artifact order is the file's order, not alphabetical | `parse` a fixture listing `zeta` (`requires: [middle]`), `alpha` (`requires: [zeta]`), `middle`, and a fourth entry repeating the id `zeta`; assert exactly those four in file order. The edges and the repeat are load-bearing: the requirement forbids sorting, de-duplicating, **and** re-ordering by `requires`, and a fixture with no edges and no repeat is green against a topological sort and against an insertion-ordered de-duplicating map | unit | pure | `cargo test --all-features schema::` |
| A schema that is not vendored is reported as such and not as broken | `load` a scratch repository holding only `openspec/schemas/tdd/`, asking for `spec-driven`; assert `Err(LoadError::NotVendored { path })` by **variant**, and that the path names `openspec/schemas/spec-driven/schema.yaml`; then, in the same test, `resolve(...)` and assert `problems.len() == 1` naming that path, since `LoadError` carries no `problems` and only the composition renders one. A third run against a repository with no `openspec/schemas/` directory at all asserts the same variant | unit | real scratch filesystem | `cargo test --all-features schema::` |
| A schema directory with no `schema.yaml` is not vendored | `openspec/schemas/half/` created empty; assert `NotVendored`, not `Unreadable` | unit | real scratch filesystem | `cargo test --all-features schema::` |
| A `schema.yaml` that cannot be read is reported as unreadable | `openspec/schemas/odd/schema.yaml` created as a **directory**; assert `Unreadable` carrying the I/O message, then `resolve(...).problems.len() == 1` naming the path, and no panic. This is the scenario that separates `NotVendored` from `Unreadable`, which a `Path::exists()`-based implementation conflates | unit | real scratch filesystem | `cargo test --all-features schema::` |
| The directory name and the file's `name:` key disagreeing is recorded | `openspec/schemas/renamed/schema.yaml` declaring `name: original`; assert `Schema::name == "renamed"` and exactly one problem containing both words; then, in the same test, three more fixtures — no `name:` key, a blank `name:`, and a mapping-valued `name:` — each loading with the directory's name and **no** problem. Without those three, an implementation comparing `dir != doc["name"].as_str().unwrap_or("")` passes here and turns every other fixture in the change red, since almost all of them omit `name:` | unit | real scratch filesystem | `cargo test --all-features schema::` |
| Only the first YAML document of the schema file is used | Two `---`-separated schema documents; assert the artifact list is exactly the first document's | unit | pure | `cargo test --all-features schema::` |
| `apply.tracks` selects an artifact whose id is not `tasks` | Fixture with `checklist`/`tasks.md` and `tasks`/`notes.md`, `apply.tracks: tasks.md`; assert `tasks == Some(checklist)` **and** `!= Some(tasks)`. This is the headline scenario: an id-first implementation — which is what `SPEC.md` as written would have produced — fails it | unit | pure | `cargo test --all-features schema::` |
| An absent `apply` block falls back to the artifact with id `tasks` | Fixture with `proposal`/`proposal.md` and `tasks`/`checklist.md`, no `apply:`; assert `tasks` is selected and its `generates` is `checklist.md`, so the assertion is on the whole artifact and not on a name that both rules would produce | unit | pure | `cargo test --all-features schema::` |
| An `apply` block without `tracks`, and an explicit `tracks: null`, both fall back to the id | Two fixtures asserted in one test. They differ in the parse tree — `BadValue` versus `Null` — and an implementation branching on key presence rather than on "absent or null" behaves differently for them. A third fixture in the same test makes `apply:` itself a bare scalar and asserts the id fallback with no panic, which is the indexing-a-non-mapping failure reached from the other direction | unit | pure | `cargo test --all-features schema::` |
| A `tracks` value matching nothing yields no tasks artifact even when an id `tasks` exists | Fixture with `tasks`/`tasks.md` and `design`/`design.md`, `apply.tracks: nowhere.md`; assert `tasks.is_none()` and one problem naming `nowhere.md`. An implementation that falls back to the id on a miss returns `Some(tasks)` — and that fallback is exactly what the CLI's `find` does not do | unit | pure | `cargo test --all-features schema::` |
| A schema with neither a `tracks` match nor an id `tasks` loads without one | Fixture with only `proposal` and `design`; assert the schema loads with both artifacts, `tasks.is_none()`, and one problem. An implementation returning `Err` here fails closed | unit | pure | `cargo test --all-features schema::` |
| `tracks` matches on `generates`, not on a filename suffix | Fixture with `a`/`sub/tasks.md` and `b`/`tasks.md`, `apply.tracks: tasks.md`; assert `b`. An implementation comparing `Path::file_name` returns `a`, which is first in the list | unit | pure | `cargo test --all-features schema::` |
| A `tracks` value of the wrong type falls back to the id | `apply.tracks` as a sequence, with an id `tasks` present; assert `tasks` selected and one problem naming the key | unit | pure | `cargo test --all-features schema::` |
| Entries missing `id` or `generates` are skipped and the rest survive | Four-entry fixture; assert the list is exactly `proposal`, `tasks` and exactly two problems naming positions 1 and 2. Deliberately **no** `Schema::tasks` clause: the tasks rule is group 5's, and this scenario lands in group 4 | unit | pure | `cargo test --all-features schema::` |
| A blank `id` or `generates` is skipped like an absent one | `id: "  "` and `generates: ""` alongside one valid artifact; assert one artifact and two problems | unit | pure | `cargo test --all-features schema::` |
| An entry that is not a mapping is skipped | A bare string and a nested sequence in `artifacts:`; assert one artifact, two problems, no panic | unit | pure | `cargo test --all-features schema::` |
| A `generates` that escapes the change directory is skipped | `../outside.md`, `/etc/passwd`, `a/../../b.md` rejected; `specs/**/*.md` and `sub/dir/file.md` accepted, all in one test. Without the accepted pair, a predicate rejecting every value containing `/` or `.` passes | unit | pure | `cargo test --all-features schema::` |
| Unused keys being absent or malformed does not make an entry unusable | One artifact with only `id` and `generates`, one with `requires` as a string; assert both load and no problem is recorded. An implementation validating the whole CLI artifact schema fails this and would fail closed on a schema the plugin can render | unit | pure | `cargo test --all-features schema::` |
| A schema whose every entry is unusable is invalid rather than empty | Two entries, neither with an `id`; assert `Err` (→ `Invalid`), not an `Ok` with an empty artifact list | unit | pure | `cargo test --all-features schema::` |
| Invalid YAML is reported with the parser's own message | Tab-indented schema file; assert `Invalid` naming the path, the problem carrying the parser message, and no panic | unit | pure for `parse`; real scratch filesystem for the `LoadError` mapping | `cargo test --all-features schema::` |
| An empty, comment-only, or non-mapping document is invalid | Four inputs in one test — empty, comment-only, `just text`, and a top-level sequence; assert `Err` for each and no panic. The first two return **zero documents** from `yaml-rust2`, so an implementation indexing `[0]` panics | unit | pure | `cargo test --all-features schema::` |
| An absent, non-sequence, or empty `artifacts:` key is invalid | Three inputs in one test — no key, `artifacts: nope`, `artifacts: []`; assert `Err` for each, with the empty-sequence case asserted explicitly | unit | pure | `cargo test --all-features schema::` |
| Flow style, anchors, and CRLF all parse | Three inputs in one test; assert two ordered artifacts for the flow-style one, the aliased artifact for the anchor one, and equal `Schema` values for the `\r\n` and `\n` forms; group 5 extends this test with the `Schema::tasks` assertion once the rule exists. **This is the scenario that distinguishes a YAML parser from a line scanner**, and it is the test that fails if the dependency is ever replaced with a hand-rolled subset | unit | pure | `cargo test --all-features schema::` |
| A schema file whose block scalars mimic structure still parses correctly | Fixture whose `apply.instruction: \|` block contains `- id: fake`, `generates: fake.md`, and `tracks: fake.md` at the same column an artifact's keys use; assert exactly the two real artifacts with no `fake` entry. Modelled on the vendored `tdd` schema, whose `apply.instruction` content genuinely sits at that column. The other half of this scenario — that the *tracks target* comes from the real `apply.tracks` and not from the one inside the block scalar — is asserted in group 5, where the rule exists; group 4 can only assert the artifact list | unit | pure | `cargo test --all-features schema::` |
| A composed resolution carries the selection's problems as well as the load's | `resolve` against a scratch repository whose change `.openspec.yaml` is invalid YAML and whose `openspec/config.yaml` declares `schema: absent`; assert name `absent`, source `Project`, `Err(NotVendored)`, and `problems.len() == 2` with the selection's first. The only scenario where both halves contribute, and the one that rejects a composition rebuilding `problems` from the load step alone | unit | real scratch filesystem | `cargo test --all-features schema::` |
| A repository tree is byte-identical after loading | Snapshot including directories, bytes, and mtimes; `resolve` twice, then once more naming a schema that is not vendored; assert equality after all three | unit | real scratch filesystem | `cargo test --all-features schema::` |
| The schema module names no process API | The `SPAWN` block above, run as one script whose single exit status is inspected — positive controls first, `if grep` rather than `! grep` | command check | real `grep` over `src/` | see `SPAWN` |
| Exactly one binary target is produced at the release path | The `BUILD` block above: the `bin`-target filter, then `rm -f` the binary, then `scripts/build.sh` asserted to **exit 0**, then the executable check. Carried from the live requirement and strengthened, because an existence-only check is satisfied by a leftover from any earlier build | command check | real `cargo`, real `sh`, writes only to the gitignored `target/` | see `BUILD` |
| The declared dependency set is exactly the argued crates | The `DEPS` filter, plus a direct read of `Cargo.toml` for the `features = []` clause, which `cargo metadata` cannot distinguish from an omitted key | command check | real `cargo metadata`, real `python3`, `Cargo.toml` read as text | `cargo metadata --no-deps --format-version 1 \| python3 -c "$DEPS"` |
| Every package in the normal build graph declares an MSRV no higher than the crate's | The `MSRV` block above: floor read from the root package's `rust-version`, `(name, version)` matching, four triples, two vacuity guards, and the set at the floor printed rather than a single crate named | command check | real `cargo tree`, `cargo metadata`, `python3` | `python3 -c "$MSRV"` |
| The resolved build graph is small and proc-macro-free | The `GRAPH` block above, over all four supported triples, with the proc-macro/`encoding_rs` `grep -qxE` written into it — kept even though the exact-list equality implies it, so a later relaxation of the list to a subset still catches a proc macro | command check | real `cargo tree` | see `GRAPH` |
| Each dependency is genuinely needed rather than incidental | The `NEEDED` block above, run entirely in a throwaway copy under `mktemp -d`. The working tree is never edited: `cargo build` rewrites `Cargo.lock` during resolution before it reaches the compile error, so an in-tree edit-and-restore of `Cargo.toml` alone leaves the tree unbuildable under `--locked`. The `grep -q` precondition on each leg stops a removal that is a no-op from being recorded as a pass | command check | real `cargo`; a copied crate under `std::env::temp_dir()`; the working tree read-only | see `NEEDED` |

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

**Does the plugin need to read `schema.yaml` at all?** The one alternative that removes the
dependency entirely deserves a paragraph rather than a row in the Context table.
`openspec status --change <n> --json` already returns `artifacts[]` with `id`, `outputPath`,
`status` and `requires`, **in schema order** — the whole content of `Schema::artifacts` plus
the tab order, per change, from a command `changes-from-cli` already plans to run. A plugin
that only ever asked the CLI would need no YAML crate and no parser.

Rejected, on `SPEC.md`'s dual-source model: "files paint the pane immediately, CLI results
arrive asynchronously and correct them". The CLI is a Node binary costing 200–400ms per
invocation and it does not exist as a collaborator until Phase 3, so a CLI-only schema means
the pane opens with no tabs and no tasks tab until the first call returns — and shows nothing
at all when `openspec` is not installed, which `SPEC.md` names as a supported state. The file
tier is the one that must exist. What this does change is the *scope* of the CLI tier: it is
not merely a fallback for a schema that is not vendored, it is also the corrector for one
that is, and `changes-from-cli` gets both from the same call.

**No injected hook for the CLI fallback, unlike `repo-resolution`'s step 4.** The asymmetry
is deliberate and is worth stating so nobody reads it as an oversight. `repo-resolution`
injected a `&dyn Fn() -> Option<PathBuf>` because `npm prefix -g` was **step 4 of an ordered
chain**: without it, steps 1–3's ordering was the only thing ever exercised, and
`subprocess-seam` would have had to re-derive where step 4 belonged. Here there is no chain.
The disk tier either produces a schema or names one of three reasons, and the CLI tier is a
strictly later consumer. A hook would be a parameter with one production binding, one test
binding, and no ordering to protect — ceremony rather than a seam.

But the fallback position that argument retreats to has to actually work, and the obvious
version of it does not. `openspec schema which spec-driven --json` reports
`/…/node_modules/@fission-ai/openspec/schemas/spec-driven` — an absolute directory **outside
the repository**, with no `openspec/` segment — so a `load(repo, name)` that builds
`<repo>/openspec/schemas/<name>/schema.yaml` cannot be pointed at it. Worse than a flat no:
the `$XDG_DATA_HOME` tier *would* fit that signature and the package tier would not, so a
Phase 3 implementer would get a function that silently covers one of the two CLI tiers.

**So `load_dir(dir, name)` is the primitive and `load(repo, name)` is the convenience.**
`load_dir` reads `<dir>/schema.yaml` for any schema directory — the repository's,
`$XDG_DATA_HOME`'s, or the one the CLI reports — and `load` is one `join`. It costs no new
scenario (every existing `load` scenario exercises it) and it means Phase 3 is a call rather
than a re-derivation of the `NotFound → NotVendored` / `IsADirectory → Unreadable` /
`parse-Err → Invalid` mapping, which is the natural property of this change. That is what
makes the no-hook argument sound rather than merely cheap: the seam is the *signature*, not
a closure. The obligation is also written onto the roadmap, where a `changes-from-cli`
implementer will read it.

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
  in the graph** — it ties with `hashbrown`, `hashlink`, `toml`, `toml_datetime`,
  `toml_parser`, `toml_writer` and `serde_spanned`, all at `1.85`. A future release that
  raises it breaks the crate's declared MSRV silently, because cargo will happily resolve it.
  → Partially mitigated, and the limit is stated rather than overclaimed: the MSRV clause had
  prose and no scenario and now has one, but **the check is not a standing gate**.
  `make check` is `fmt-check lint test coverage` and CI runs exactly those; `MSRV` runs once,
  in this change's task list. What pins the resolution afterwards is the committed
  `Cargo.lock`. Resolution point: the next change that alters the dependency set re-runs the
  blocks — and if that proves too weak in practice, the honest fix is a `deps-check` target
  inside `make check`, which is a `quality-gates` change and not this one's.
- **`yaml-rust2` is the stable-numbered sibling of `saphyr`, which the same maintainer
  positions as its successor.** → Both were released on the same day (2026-08-18), and
  `yaml-rust2` carries 14.6M recent downloads against `saphyr`'s 757k, so it is being
  released in lockstep rather than abandoned. `saphyr` is unusable here today regardless, on
  the proc-macro constraint. The migration cost if that changes is bounded by the decision
  above: no YAML type appears in any signature, so it is one module's body. Resolution point:
  the next change that alters the dependency set and re-runs `GRAPH` — not a `cargo update`,
  which fires nothing, for the reason the bullet above states.
- **`yaml-rust2` rejects duplicate mapping keys; the JavaScript `yaml` parser the CLI uses
  accepts them, last-wins.** A schema with a duplicated key therefore loads for the CLI and
  is `Invalid` for the plugin. → Accepted, and it is the safe direction: the plugin degrades
  visibly with the parser's message rather than silently picking a different value than the
  CLI did. The same `Err` fires on the two files a **user actually hand-edits** —
  `openspec/config.yaml` (this repository's own is eighty-plus lines with nested block
  scalars, which is exactly where a duplicated `rules:` or `context:` key happens) and a
  change's `.openspec.yaml` — where the effect is not a visible `Invalid` schema but a
  *silent fall-through to the next source* carrying one problem string. Recorded for both
  files so a future bug report is diagnosable rather than mysterious.
- **One unit scenario reads this repository's own graft-vendored `openspec/schemas/tdd/`.**
  A `graft sync` that changes that file could break a test that has nothing to do with the
  change being made. → Accepted: the artifact-list assertion is **exact**, not a containment
  check — a parser that silently drops `design` or `planning-review`, the two longest block
  scalars and precisely what a block-scalar defect would eat, satisfies any "contains" clause,
  so the exact form is what makes the scenario worth having. It is read through
  `env!("CARGO_MANIFEST_DIR")`, never the process working directory. The scenario is worth the
  coupling: it is the only test that proves the model matches the file the plugin will actually
  meet, and it is the test that would have caught `role: tasks` before a line was written. A
  graft update that adds or renames an artifact *will* turn this test red, on purpose — that is
  a one-line fixture update with a real question behind it, not a flake.
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
  that needs `subprocess-seam`. But the **command it will use, and the function that consumes
  it**, are both settled and recorded, because `SPEC.md`'s `openspec schema` does not exist:
  `openspec schema which <name> --json` returns `{"name","source","path","shadows"}`, where
  `path` is an **absolute schema directory** — for a CLI-shipped schema, one outside the
  repository entirely, with no `openspec/` segment — which is handed to
  `load_dir(path, name)`. Its stdout is clean JSON; the "experimental" note goes to stderr.
  `openspec status --change <n> --json` separately returns the ordered `artifacts[]` per
  change, which `changes-from-cli` already plans to call.
- *Should `Schema` carry `requires` so the tab bar could show dependency order?* No.
  `SPEC.md` says the `artifacts` list *is* the tab order; a second ordering with no
  requirement behind it is a field nothing reads.
- *Should an unknown schema name be validated against the vendored directory listing before
  loading?* No. `load` reads one path and reports `NotVendored` when it is absent — listing
  `openspec/schemas/` first would be a second filesystem call to produce a worse message, and
  the CLI's own "available: [...]" hint belongs to the CLI tier that will have it.
