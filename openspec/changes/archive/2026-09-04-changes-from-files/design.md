## Context

Phase 2 has produced three answers and no consumer for them. `resolve::find_repo` returns a
repository root. `schema::resolve` turns a change directory into an ordered artifact list
and, when the schema declares one, the artifact holding the tasks. `tasks::read` counts the
checkboxes in a file it is handed. Nothing joins them, and nothing has ever opened
`openspec/changes/`.

Two constraints shape the join, and neither is negotiable.

**The list must be the CLI's list.** `changes-from-cli` will produce the same `Change` type
from `openspec list --json`, and `live-refresh` will layer the two on every refresh. A
directory the file path counts and the CLI does not is a row that appears and then vanishes;
a task count computed by a different rule is a number that changes by itself. So the
enumeration rule is not designed here — it is read out of the installed CLI and copied, the
way `task-parsing` copied `TASK_LINE_PATTERN`. The relevant source is
`@fission-ai/openspec@1.11.0`, `dist/core/list.js:81-87` for the listing and
`dist/utils/task-progress.js:120-133` for the counting, and every claim this document makes
about the CLI was read there or driven through a probe repository rather than recalled.

That reading produced three facts that change the design:

1. **A change is a directory and nothing more.** No `proposal.md`, no `.openspec.yaml`, no
   marker of any kind. The CLI's own discovery module records why: requiring `proposal.md`
   made `openspec show` miss changes that `openspec new change` had just scaffolded.
2. **The task counter falls back.** When the tasks artifact resolves to no files —
   a glob that matched nothing, an unresolvable schema, a schema naming no tasks
   artifact — the CLI substitutes the single path `<changeDir>/tasks.md` and counts it
   anyway. A file path that reported `0/0` in those cases would disagree with
   `openspec list --json` for exactly the changes whose schema is unusual, which is where a
   dashboard is most needed.
3. **The CLI cannot address an archived change at all.** `openspec instructions apply
   --change 2026-09-04-task-parsing` answers "change not found", and `openspec list --json`
   lists only active changes. Archived changes are permanently file-sourced.

**The type has two producers and will never have one.** `openspec/config.yaml` →
`rules.tasks` states the failure mode precisely: "a field added to one without the other is
a divergence the type system will not catch if it is optional." This design's answer is a
compile error, in the one place both producers meet, and it is written up in Contracts
rather than left as a convention.

## Goals / Non-Goals

**Goals:**

- One entry point, `changes::from_files(repo, archived_count) -> ChangeSet`, that paints the
  pane from disk with no `openspec` binary present.
- Enumeration and counting that agree with `openspec list --json` for every shape reachable
  on this repository and every shape probed against the real CLI.
- A `Change` type designed for two producers from the start, with the agreement enforced by
  the compiler rather than by a comment.
- Artifact paths resolved from the schema, so a schema whose artifact ids differ from its
  filenames renders correctly.
- Every behaviour reachable by a unit test over a pure function or a scratch directory tree,
  so the 80% floor is met by construction.

**Non-Goals:**

- Any subprocess, any injected subprocess hook, and any CLI parsing. Argued in Decisions 2.
- Any rendering. No `ui` code, no `TestBackend`, no 60/120-column tests.
- A general glob engine. The supported subset is deliberately small and everything outside
  it degrades visibly.
- Caching between calls, watching, or invalidation — `live-refresh` owns those.
- Merging, which needs two producers and arrives with the second.

## Boundaries

| Piece | Where it lives | Pattern it follows |
|---|---|---|
| `Change`, `ChangeSet`, `ArtifactRef`, `Origin` | new `src/changes.rs` | plain data deriving `Debug, Clone, PartialEq, Eq`, compared whole with `assert_eq!`, like `schema::Schema` and `resolve::BinResolution` |
| `from_files` | new `src/changes.rs` | the total, never-failing composed reader — no `Result` — like `config::load`, `state::read`, and `resolve::openspec_bin` |
| `problems` on three levels | new `src/changes.rs` | the `Vec<String>` every degrading value in this crate carries |
| `is_glob`, `split_archive_name`, `shape` | new `src/changes.rs` | pure functions of `&str`, like `schema::schema_key` and `resolve::path_candidates` |
| directory walks | new `src/changes.rs` | `resolve::nvm_candidates`' shape — `read_dir(..).flatten()`, every `Err` meaning "not there" — for the *inner* walks only. The two top-level reads (`openspec/changes/` and its `archive/`) must instead match on the `Result`, because they have to tell `NotFound` (no problem) from `PermissionDenied` (one problem); `flatten()` there would silently discard the distinction the degraded-state scenarios rest on |
| schema selection | `schema::read_file`, `schema::declared_name`, `schema::load` | the `pub(crate)` handoff `schema-model` published for this change, so `openspec/config.yaml` is read once rather than once per change |
| task counting | `tasks::read`, `tasks::Progress` | consumed unchanged; `Progress`'s `+=` is what sums a multi-file tasks artifact |
| conformance | `#[cfg(test)] mod conformance` in `src/changes.rs` | new; the shared gate both producers' tests call |
| module registration | `src/lib.rs` | one `pub mod changes;` line beside the existing five |
| scratch trees, snapshots | `crate::testutil` | `ScratchDir`, `snapshot`, `write_with_mode`, `symlink`, `canonical` already exist and are reused unchanged |

`changes::from_cli` will be added to this same file in Phase 3 — the crate keeps one file
per module and `SPEC.md`'s module map assigns both producers to `changes`. Nothing else is
touched: `src/main.rs`, `Cargo.toml`, `Cargo.lock`, `Makefile`, `herdr-plugin.toml`, and
`.github/workflows/ci.yml` are unmodified, which is why no group in tasks.md carries a
manifest or CI gate.

## Contracts

The module's public surface, additive in full — no consumer exists yet, so nothing here can
break one:

```rust
pub enum Origin { Active, Archived { date: Option<String> } }

pub struct ArtifactRef { pub id: String, pub paths: Vec<PathBuf> }

pub struct Change {                       // NOT Default, NOT non_exhaustive
    pub name: String,
    pub dir: PathBuf,
    pub origin: Origin,
    pub schema: String,
    pub artifacts: Vec<ArtifactRef>,
    pub progress: crate::tasks::Progress,
    pub problems: Vec<String>,
}

pub struct ChangeSet {
    pub active: Vec<Change>,
    pub archived: Vec<Change>,
    pub problems: Vec<String>,
}

pub fn from_files(repo: &Path, archived_count: usize) -> ChangeSet;
```

Internals reachable by a scenario, `pub(crate)` for the same reason `schema::declared_name`
is — they are the pure core the composed reader is assembled from, and testing them directly
is cheaper and more discriminating than reaching them through a directory tree:

```rust
pub(crate) fn is_glob(generates: &str) -> bool;

/// `(date, name)`. `date` is `None` when the CLI's `^\d{4}-\d{2}-\d{2}-` pattern does
/// not match, or matches with nothing after it; `name` is then the whole directory name.
pub(crate) fn split_archive_name(dir_name: &str) -> (Option<String>, String);

/// The final segment of a glob: a literal filename, or a literal prefix and suffix
/// around exactly one `*`. `Prefixed { prefix: "", suffix: "" }` is a bare `*`.
pub(crate) enum FilePattern { Literal(String), Prefixed { prefix: String, suffix: String } }

pub(crate) enum Shape {
    /// Not a glob: one relative path, joined and tested for being a regular file.
    Literal(String),
    /// `dirs` are the literal directory segments; `recursive` is true when the last
    /// directory segment was `**`.
    Glob { dirs: Vec<String>, recursive: bool, file: FilePattern },
}

/// `Err` is the reason a glob shape falls outside the supported subset, naming the
/// pattern verbatim so the caller can record it as a problem.
pub(crate) fn shape(generates: &str) -> Result<Shape, String>;

/// One artifact's paths, plus the one problem an unsupported shape records.
pub(crate) fn resolve_artifact(change_dir: &Path, generates: &str)
    -> (Vec<PathBuf>, Option<String>);

/// Every artifact of one schema, in the schema's declared order, plus their problems.
pub(crate) fn change_artifacts(change_dir: &Path, schema: &crate::schema::Schema)
    -> (Vec<ArtifactRef>, Vec<String>);

/// The change's task pair, applying the CLI's empty-list fallback to `<dir>/tasks.md`.
pub(crate) fn change_progress(change_dir: &Path, tasks: Option<&crate::schema::Artifact>)
    -> (crate::tasks::Progress, Vec<String>);
```

Error surface: `from_files` has none. It returns a value, never a `Result`, never panics,
and never `unwrap`s. `shape` returns a `Result` to its own composer only — the same
narrower precedent `task-parsing` recorded, where `schema::parse` and `schema::load` return
`Result` internally while every composed top-level reader in the crate is total.

**`from_files` takes `archived_count`, not `&Config`.** The whole of `Config` is not needed,
the coupling would be one-directional and untestable-by-inspection, and a `usize` argument
is what the scenarios about the limit actually vary. `openspec_bin`'s composition
(`openspec_bin_from_env`) is the crate's precedent for binding a `Config` to a pure function
at the call site rather than inside it.

### How `from_files` and `from_cli` are kept in agreement

`rules.design` requires this question answered explicitly. This change **introduces** the
`Change` type, so the answer is the design of the type rather than a compatibility note.
Four mechanisms, in decreasing order of how hard they are to defeat:

1. **`Change` derives no `Default` and implements none, and no producer writes a functional
   update.** Rust's struct-literal syntax requires every field to be named unless a
   functional-update expression (`..expr`) supplies the rest. Removing `Default` removes the
   expression a producer would actually reach for, but it does not remove the syntax:
   `Change { name, ..other }` compiles with no `Default` anywhere, which is why the absence
   of `Default` alone is **not** sufficient and this design does not claim it is. The rule is
   therefore the pair — no `Default`, and no `..` inside a `Change` literal — and the guarded
   source check (tasks.md 9.2) scans for both. With both held, adding a field is a compile
   error (`E0063: missing field ... in initializer of Change`) at every construction site,
   `from_files`' today and `from_cli`'s in Phase 3.

2. **One shared conformance function whose pattern is exhaustive.**
   `conformance::assert_invariants(&Change)` opens with

   ```rust
   let Change { name, dir, origin, schema, artifacts, progress, problems } = change;
   ```

   with no `..` rest pattern. A new field makes *that function* fail to compile, naming the
   field. Because it is the function **both** producers' test suites call, the person adding
   a field is forced into the one place where the question "what does this mean for the
   other source?" is unavoidable. This is the mechanism the rule is really asking for: a
   compile error in shared code, not in one producer's private code.

3. **No field whose absence is representable as a shrug.** `progress` is a `Progress`, not
   an `Option<Progress>`, because the CLI's verified fallback means every change always has
   a pair. `schema` is a `String` because selection is total. `artifacts` and `problems` are
   possibly-empty vectors. The one `Option` in the contract, `Origin::Archived { date }`, is
   a fact about a directory name that only one producer can ever observe, and it is inside a
   variant `from_cli` cannot construct at all. `task-parsing`'s design predicted an
   `Option<Progress>` here; measuring the CLI's fallback changed the answer, and this
   paragraph is the correction.

4. **No derived and no source-specific state.** No `status` (the CLI derives it from the
   pair; `Progress::is_complete` mirrors it), no ratio, no formatted string, no
   `lastModified`, and no producer discriminant. Every one of those is a field two producers
   could compute differently while both looking correct. A contract-gate CHECK task
   re-reads the definition before each of the two groups that freeze it.

Two consequences `changes-from-cli` inherits, recorded here because they are this change's
decisions and not that change's:

- **The merge joins the artifact lists by position, not by path and not by id.** Paths are
  not canonicalized here (Decisions 6) while the CLI's `contextFiles` returns canonicalized
  absolute paths, so the two producers' path *strings* legitimately differ — that rules out
  a path join. An id join looks safer and is not available either: the landed
  `schema-artifacts` capability requires a schema's artifact list to be kept verbatim and
  "never de-duplicated", and it ships a scenario producing the ids `zeta, alpha, middle,
  zeta`, so an id is not a key. What both producers *do* reproduce is the schema's declared
  order, because both build the list by iterating `Schema::artifacts`. Position is therefore
  the join, and `conformance::assert_invariants` deliberately does not assert that ids are
  unique.
- **`from_cli` must re-sort by name.** `openspec list --json` defaults to
  most-recently-modified first. Adopting that order would require reproducing the CLI's
  recursive maximum-mtime walk in `from_files`; adopting name order costs `from_cli` one
  `sort_by`. The alternative is a list that reorders itself when the CLI lands.

Named consumers and what each one takes:

| Consumer | Takes | Why the shape holds for it |
|---|---|---|
| `changes-from-cli` | every field, and `conformance::assert_invariants` | produces the same type from JSON; merges the `active` list by name and each change's artifacts by id |
| `tui-shell`, `list-view` | `ChangeSet`, `Change::{name, origin, progress}` | one row per active change, then the separator, then the archived rows |
| `detail-view` | `Change::{name, schema, progress, artifacts}` | header and tab bar; an empty `ArtifactRef::paths` is the "No content yet" tab |
| `tasks-tab` | the tasks artifact's resolved paths | re-reads them through `tasks::read` to render groups |
| `live-refresh` | `Change::dir` | maps a watcher event's path back to the change to invalidate |
| `agent-attribution` | `Change::name` | joins live agent names against the active list |
| `degraded-states` | all three `problems` lists | the rows it has to prove true |

## Persistence and Rollout

- **Migration:** none. Nothing is stored.
- **Backfill:** none.
- **Seeding:** none. Every test tree is built at run time under `ScratchDir` and removed on
  drop; no fixture is checked in by this change.
- **Cache invalidation:** none between calls. Two caches exist *within* one `from_files`
  call — the project `openspec/config.yaml` text, and loaded schemas by name — both owned by
  the call and dropped with it, never `static`, for `resolve::BinCache`'s stated reason: the
  suite runs in parallel threads of one process and a process-global cache would let the
  first test decide the answer for every other.
- **Index rebuild:** none.
- **Authorization:** none. Filesystem reads the user's own process already has; a permission
  failure is a recorded problem rather than an error.
- **Observability:** none beyond the three `problems` lists, which `degraded-states`
  surfaces.
- **Deployment:** none. No manifest, no CI, no build-script change. The release binary's
  behaviour is unchanged because nothing calls the module yet.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem | not applicable — no acceptance test (see Test Strategy) | **real**, always. Every tree is built under `testutil::ScratchDir` in `std::env::temp_dir()`; no faked filesystem layer, matching `resolve`, `schema`, and `tasks`. Three capabilities beyond writing files are used and are named here so no task invents them: `testutil::symlink` for file and directory links, `std::fs::set_permissions` for a mode-`0o000` directory (new to this crate — see Risks), and `testutil::canonical` applied to the *root passed in*, never to an expectation |
| A non-UTF-8 filename | not applicable | **not used.** APFS rejects an invalid UTF-8 name with `EILSEQ`, so the behaviour is verified by a unit test over the name-decoding step given an `OsString` built with `OsStringExt::from_vec`, and no such directory is ever created |
| The live repository tree (`openspec/`) | not applicable | **never read and never written.** No test resolves a path inside the working repository. `task-parsing`'s decision 11 established this and it holds unchanged: a test that read `openspec/changes/` would change its expectations every time a change is created or archived |
| Checked-in fixtures (`tests/fixtures/`) | not applicable | **none added.** This change builds every tree at run time — see Decisions 8 for why, and for the `SPEC.md` → Fixtures correction that goes with it |
| The `openspec` binary | not applicable | **absent from every test.** Not invoked, not probed, not faked, and no injected hook stands in for it. It is consulted twice **outside** the suite by one-off operational checks that are not gates: task 9.5 re-reads the four shipped files this design copies rules from, to confirm they are current, and task 12.8 runs `openspec validate`. Neither is a collaborator of any Rust test, and neither is spawned by crate code |
| A probe OpenSpec repository outside this tree | not applicable | **real, and outside the Rust suite.** Task 9.6 drives the real `openspec list --json` over a throwaway repository under the scratchpad to confirm the enumeration rules empirically. Recorded as a command with its output, never committed as a Rust test — the suite must not need a Node binary |
| Process environment (`std::env::var`) | not applicable | **not consulted.** `from_files` takes a root path and a count; no environment lookup closure is introduced, because nothing here depends on the environment |
| Process spawn | not applicable | **absent**, and asserted so: a guarded source scan over `src/changes.rs` is a scenario, not a convention |
| `git` and `cargo tree` (group 9's checks) | not applicable | **real**, and outside the Rust suite. `git diff` against a base SHA captured in task 1.1 before this change's first commit, and `cargo tree --edges normal` against a baseline captured at the same moment. Recorded as commands with pass/fail outcomes, never as Rust tests |
| Herdr socket | not applicable | **absent.** Not reachable from this module and not referenced |
| Terminal / ratatui | not applicable | **absent.** No view is added, so no `TestBackend` appears |
| Wall clock, threads, network | not applicable | **absent.** Every function is deterministic and single-threaded. No test sleeps, polls, or races; the one place a clock could have entered — a `lastModified` field — is deliberately not on the type |
| Third-party crates | not applicable | **none added.** `toml` and `yaml-rust2` remain the whole dependency set and neither is used by this module |

## Test Strategy

**No outer-loop acceptance group.** This change's outermost surface is a library API.
`src/main.rs` is untouched, `herdr-openspec ui` still prints its placeholder, and no caller
consumes a `ChangeSet` until `tui-shell`. An acceptance test would drive an entry point that
ships to nobody, and the boundary it would "integrate" — a real directory tree — is already
the unit tier's real collaborator. `from_files` over a complete scratch repository *is* the
integration, and it runs in the unit tier at unit-tier speed.

**Which of the project's six required boundary states bind here.** `openspec/config.yaml` →
`rules.specs` requires every change to cover "no openspec directory, no active changes, a
missing artifact file, a schema the CLI rejects, an unreachable Herdr socket, and a pane
narrower than 100 columns". Four bind here, and all four are covered by this change's own scenarios: **no
openspec directory** and **no active changes** are enumeration scenarios; **a missing
artifact file** is an empty `ArtifactRef::paths`; and **a schema the CLI rejects** binds in
its disk-side form — a schema that is not vendored, which is the same `outside-in-tdd`
situation `SPEC.md` → Degraded states names, distinguished there from the CLI-rejection row
that `changes-from-cli` owns. The remaining two belong to modules this change does not
touch: the Herdr socket is `agent-polling`'s and the 100-column breakpoint is `tui-shell`'s.
This change's own additional boundary states, beyond the six: an unreadable changes
directory, an unreadable archive directory, an archive entry with no date prefix, an
`archived_count` larger than the archive, a directory entry name that is not valid UTF-8, an
artifact that exists as both `<id>.md` and `<id>/`, and an unsupported glob shape.

**Where the `<id>.md`-and-`<id>/` collision lands.** The brief names it as a boundary case
and this design dissolves it rather than deciding it: resolution is driven by `generates`,
which names exactly one of the two for any given artifact. Under the `tdd` schema, `specs`
generates `specs/**/*.md` and a file named `specs.md` beside the directory is matched by no
artifact and is invisible; `tasks` generates `tasks.md` and a directory named `tasks/` is
invisible for the same reason — except through the tasks fallback, which names
`<dir>/tasks.md` and finds a directory there, producing the "unreadable tasks file" problem
that `tasks::read` already specifies. Both halves are scenarios (`change-artifacts`, "A
non-glob `generates` naming something that is not a regular file resolves to nothing" and
"An unreadable tasks file is zero plus one named problem"), and the collision is only a
collision for an implementation that guesses from the id, which is the thing this change
stops doing.

Every row's command is `cargo test --all-features changes::` unless stated otherwise, and
`make check` is the composite gate.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| A fully written active change becomes one value | `from_files` over a complete scratch repository; whole-`Change` `assert_eq!` | unit | real filesystem | `cargo test --all-features changes::` |
| An archived change carries the date split off its directory name | `from_files` over a scratch repository with one archived directory | unit | real filesystem | `cargo test --all-features changes::` |
| A change whose schema did not load is still a complete value | scratch repository declaring an unvendored schema; asserts empty `artifacts`, one problem, and a counted `progress` | unit | real filesystem | `cargo test --all-features changes::` |
| The three-way status split is derived, not stored | Guarded source scan over `src/changes.rs` for `status`, a ratio, a percentage, and a producer discriminant, plus a unit test reading the split off `Progress` | unit + operational check | the source tree | `cargo test --all-features changes::` and tasks.md 9.3 |
| The same repository read twice produces equal values, even after a touch | Two `from_files` calls with an unrelated file's mtime advanced between them, compared with `==` | unit | real filesystem | `cargo test --all-features changes::` |
| Tab order follows the schema, not the filesystem | Scratch change with two of five artifacts written; asserts the five ids in schema order | unit | real filesystem | `cargo test --all-features changes::` |
| A missing artifact file is an empty path list, not a missing tab | Same tree; asserts the `design` ref present with empty `paths` and no problem | unit | real filesystem | `cargo test --all-features changes::` |
| Adding a field to `Change` fails to compile in the shared conformance function | Deliberate probe: add a field, run `cargo test`, record both compiler errors (`E0063` at each construction site, `E0027` at the conformance pattern), revert. A one-off check whose output is recorded, not a committed test | operational check | `cargo`, the source tree | tasks.md 2.6 |
| `Change` gaining a `Default` is caught by a source check | Two guarded scans, exit statuses consumed, each run first against a probe copy carrying the defect so the red is observed before the green: one for a `Default` derive or impl, one for a `..` inside a `Change` literal or in the conformance pattern | operational check | the source tree | tasks.md 9.2 |
| Every value a producer builds satisfies the shared invariants | `conformance::assert_invariants` called on all five `Change` values from a five-change scratch repository | unit | real filesystem | `cargo test --all-features changes::` |
| An archived change keeps its file-derived values when the CLI arrives | Scratch repository holding an active and an archived change of the same name; asserts two distinct values in two lists | unit | real filesystem | `cargo test --all-features changes::` |
| A repository-level failure is recorded on the set, not on a change | Scratch repository with `openspec/changes/` at mode `0o000`; asserts empty lists and one set-level problem | unit | real filesystem | `cargo test --all-features changes::` |
| A directory entry whose name is not valid UTF-8 is skipped, not fatal | Unit test over the name-decoding step, given an `OsString` built from invalid bytes — **not** a directory tree: APFS rejects such a name with `EILSEQ`, so a filesystem test would fail on the reference machine for an unrelated reason | unit | none (pure) | `cargo test --all-features changes::` |
| Every degradation lands on a problems list rather than in a return type | One scratch repository carrying all three failures; asserts each appears exactly once at the right level | unit | real filesystem | `cargo test --all-features changes::` |
| A directory with no marker file is a change | Scratch repository with three marker-free change directories | unit | real filesystem | `cargo test --all-features changes::` |
| A regular file is not a change | Scratch repository with a file beside a directory | unit | real filesystem | `cargo test --all-features changes::` |
| A symbolic link to a directory is not a change | `testutil::symlink` to a real directory and a dangling link, both in `openspec/changes/` | unit | real filesystem | `cargo test --all-features changes::` |
| The archive exclusion is by exact name | Scratch repository with `archive`, `archives-not-excluded`, `archive-notes` | unit | real filesystem | `cargo test --all-features changes::` |
| A dot-prefixed archive directory is not an archived change | One scratch repository asserting both rules at once: `.hidden-archived/` absent from the archived list while `.dot-change/` is present in the active list | unit | real filesystem | `cargo test --all-features changes::` |
| A dot-directory is a change | Scratch repository with `.dot-change` and `plain` | unit | real filesystem | `cargo test --all-features changes::` |
| Case and digits order by byte, not by locale | Scratch repository with four names spanning digits, upper, lower; asserts the exact order | unit | real filesystem | `cargo test --all-features changes::` |
| A normal archived directory splits into a date and a name | `split_archive_name` unit test plus the `from_files` assertion | unit | none (pure) + real filesystem | `cargo test --all-features changes::` |
| An impossible date is still a date prefix | `split_archive_name("9999-99-99-far-future")` | unit | none (pure) | `cargo test --all-features changes::` |
| A malformed or absent prefix keeps the whole name | `split_archive_name` over three malformed shapes | unit | none (pure) | `cargo test --all-features changes::` |
| A prefix with nothing after it keeps its whole name | `split_archive_name("2026-08-14-")` | unit | none (pure) | `cargo test --all-features changes::` |
| Two archived directories can strip to the same name | `from_files` over an archive holding two dates of one name | unit | real filesystem | `cargo test --all-features changes::` |
| Dated entries come newest first | `from_files` over three dated directories created in a shuffled order | unit | real filesystem | `cargo test --all-features changes::` |
| Two entries sharing a date order by name descending | `from_files` over two same-date directories; asserts the exact ordered pair | unit | real filesystem | `cargo test --all-features changes::` |
| An undated entry sorts after every dated one | `from_files` over one dated and two undated directories; asserts the exact order | unit | real filesystem | `cargo test --all-features changes::` |
| The limit keeps the most recent entries | Seven dated directories, `archived_count` 5 | unit | real filesystem | `cargo test --all-features changes::` |
| A limit larger than the archive keeps everything | Two directories at count 50, then the same tree at count 0 | unit | real filesystem | `cargo test --all-features changes::` |
| A repository with no `openspec` directory yields an empty set | `from_files` over a bare `ScratchDir`, plus an absence assertion on the paths looked for | unit | real filesystem | `cargo test --all-features changes::` |
| No active changes still lists the archive | Scratch repository whose `changes/` holds only `archive/` | unit | real filesystem | `cargo test --all-features changes::` |
| An unreadable changes directory is one named problem | `std::fs::set_permissions` to mode `0o000` on `openspec/changes/`, restored immediately after the call and before any assertion; asserts **exactly one** problem, which fails for an implementation that also walks the archive beneath it and collects its `EACCES` | unit | real filesystem | `cargo test --all-features changes::` |
| An unreadable archive leaves the active list intact | Same technique on `archive/` only | unit | real filesystem | `cargo test --all-features changes::` |
| An `archive` that is a regular file is not an archive | Scratch repository with a regular file named `archive` | unit | real filesystem | `cargo test --all-features changes::` |
| The repository tree is byte-identical after enumeration | Two `testutil::snapshot`s around a full `from_files`, compared with `assert_eq!`, plus absence assertions on the paths the walk looked for | unit | real filesystem | `cargo test --all-features changes::` |
| A change's own declaration wins over the project's | Scratch repository with two changes under two schemas, both vendored | unit | real filesystem | `cargo test --all-features changes::` |
| A repository declaring no schema falls back to the default | Scratch repository vendoring only `tdd`, with no project config | unit | real filesystem | `cargo test --all-features changes::` |
| A schema that is not vendored leaves the artifact list empty | Change declaring `outside-in-tdd` | unit | real filesystem | `cargo test --all-features changes::` |
| An artifact whose filename differs from its id resolves correctly | Vendored probe schema with `id: plan`, `generates: implementation-plan.md`; asserts the resolved path and the **absence** of any `plan.md` path | unit | real filesystem | `cargo test --all-features changes::` |
| A non-glob `generates` naming something that is not a regular file resolves to nothing | Probe schema generating `notes`, with a directory of that name | unit | real filesystem | `cargo test --all-features changes::` |
| A symbolic link to a regular file is a resolved artifact | `testutil::symlink` for `proposal.md`, then a dangling one | unit | real filesystem | `cargo test --all-features changes::` |
| A plain filename is not a glob | `is_glob("tasks.md")` | unit | none (pure) | `cargo test --all-features changes::` |
| A brace expression is not a glob and resolves to nothing | `is_glob` plus a resolution over a tree that would match if braces were expanded | unit | none (pure) + real filesystem | `cargo test --all-features changes::` |
| Each of the three metacharacters makes a value a glob | `is_glob` over three values | unit | none (pure) | `cargo test --all-features changes::` |
| A nested spec tree resolves in path order | Four-file `specs/` tree; asserts the exact ordered vector | unit | real filesystem | `cargo test --all-features changes::` |
| A glob matching nothing is an empty path list, not a problem | Absent `specs/`, then an empty `specs/` | unit | real filesystem | `cargo test --all-features changes::` |
| Dot entries and non-files are skipped | `specs/` holding a dot file, a dot directory, a directory named `looks-like.md`, and one real file | unit | real filesystem | `cargo test --all-features changes::` |
| A directory symbolic link is not descended into | `specs/loop` linking back at `specs/`; asserts termination and the exact result | unit | real filesystem | `cargo test --all-features changes::` |
| A prefix-and-suffix file pattern is supported | `specs/spec-*.md` over three files | unit | real filesystem | `cargo test --all-features changes::` |
| A wildcard inside a directory segment is unsupported | `shape` over three patterns, plus one `from_files` assertion for the recorded problem | unit | none (pure) + real filesystem | `cargo test --all-features changes::` |
| More than one wildcard in the filename segment is unsupported | `shape("specs/*-*.md")` | unit | none (pure) | `cargo test --all-features changes::` |
| A `**` that is not the last directory segment is unsupported | `shape` over two patterns, plus an assertion that sibling artifacts still resolve | unit | none (pure) + real filesystem | `cargo test --all-features changes::` |
| A single tasks file gives the change's progress | Scratch change with a 4/9 `tasks.md` | unit | real filesystem | `cargo test --all-features changes::` |
| A glob-shaped tasks artifact sums across its files | Probe schema tracking `**/tasks.md` over three task files at 1/2, 2/3, 0/1; asserts 3/6 — the pair the real CLI reported for the identical tree | unit | real filesystem | `cargo test --all-features changes::` |
| A schema naming no tasks artifact still counts `tasks.md` | Three variants: no tasks artifact, unloadable schema, glob matching nothing | unit | real filesystem | `cargo test --all-features changes::` |
| A change with no tasks file at all is zero, not complete | Empty change directory | unit | real filesystem | `cargo test --all-features changes::` |
| An unreadable tasks file is zero plus one named problem | A directory named `tasks.md` | unit | real filesystem | `cargo test --all-features changes::` |
| A change directory is byte-identical after resolution and counting | Two snapshots around resolution and counting, plus an absence assertion on the `tasks.md` the fallback named | unit | real filesystem | `cargo test --all-features changes::` |
| (cross-cutting) The module names no process API | Deterministic source scan over `src/changes.rs`, guarded so a missing file fails | operational check | the source tree | tasks.md 9.1 |
| (cross-cutting) The copied CLI rules still match the installed CLI | Re-read all four sites this design copies: `dist/core/list.js` (the active filter), `dist/utils/task-progress.js` (the empty-list fallback), `dist/core/artifact-graph/outputs.js` (`isGlobPattern` and the `statSync().isFile()` rule), and `dist/commands/workflow/instructions.js` (the `contextFiles` shape 11.1 corrects `SPEC.md` to); record package version | operational check | the installed `@fission-ai/openspec` | tasks.md 9.5 |
| (cross-cutting) The enumeration rules match the CLI empirically | Drive the real `openspec list --json` over a throwaway probe repository holding every enumeration shape; compare against `from_files` over the same tree | operational check | the real CLI, a scratchpad repository | tasks.md 9.6 |
| (cross-cutting) The dependency set is unchanged | `git diff --exit-code "$BASE"..HEAD -- Cargo.toml Cargo.lock`, where `$BASE` is the SHA captured in task 1.1 | operational check | `git`, the manifest, the lockfile | tasks.md 9.7 |
| (cross-cutting) The resolved build graph is unchanged | `diff` of `cargo tree --edges normal` against the baseline captured in task 1.1 | operational check | `cargo`, the resolved graph | tasks.md 9.8 |
| (cross-cutting) Coverage floor holds with the module added | The gate itself | operational check | the whole crate | `make coverage` |

**Can each of these fail?** Every row was put through that question, and the four classes of
answer are recorded here so a reviewer can check the reasoning rather than the intent.

- **The source scans (no process API, no `Default`, no derived state).** Each is guarded by
  `test -f src/changes.rs &&`, so a renamed or missing file fails rather than passing on
  grep's exit 2, and each alternation is typed as **bare `|` characters**: inside an ERE,
  `\|` matches a literal pipe, so `grep -E 'a\|b'` matches the string `a|b`, finds nothing
  in any real source file, and produces a check that passes unconditionally. `task-parsing`
  shipped that defect and its review caught it; the runnable forms live in tasks.md 9.1,
  9.2, and 9.3 rather than in this table, where a cell-escaped pipe would reintroduce it.
  Each is additionally run once against a deliberately doctored copy so the red is observed
  before the green is trusted.
- **The dependency-set and build-graph diffs.** Both name a base captured in task 1.1,
  before this change's first commit. `git diff --exit-code` alone compares the working tree
  to the index, and this project commits after every task group, so a `Cargo.toml` edit made
  in group 1 is already in `HEAD` by group 7 and the check passes green over the very change
  it exists to catch. `cargo tree` with nothing to compare against is a printed report, not
  a check. Both go red if any dependency is added at any point in the change, including one
  arriving through a feature flag.
- **The counting and enumeration tests.** The failure mode to avoid is a test that passes
  because both sides are empty. Three defences: the summing scenario asserts an **absolute**
  pair (3/6) obtained from the real CLI over an identical tree at planning time rather than
  from the code under test; every negative enumeration scenario places a genuine change in
  the same tree and asserts the exact resulting list, so an implementation that lists
  nothing cannot pass by agreeing with itself; and every ordering scenario asserts the whole
  ordered vector rather than a membership test.
- **The compile-time gate (`assert_invariants`'s exhaustive pattern).** This one cannot be a
  committed test, because a test that fails to compile fails the suite permanently. It is a
  one-off probe (task 2.6): add a field, observe `E0027: pattern does not mention field` at
  the conformance pattern and `E0063: missing field` at each construction site, revert, and
  record both error texts — noting that `rustc` names the field and line rather than the
  enclosing function, and that it suggests adding the very `..` the gate forbids. That is the honest
  form — the mechanism is real, and the evidence that it is real is a recorded compiler
  error rather than an assertion that could not have failed.
- **The containment snapshots.** `testutil::snapshot` records directory entries, file bytes,
  and modification times, so a `create_dir_all` — the likeliest accidental write, and the
  one a listing-only comparison misses — is visible. Each snapshot test additionally asserts
  that a path the walk looked for and did not find still does not exist, because the
  fallback path `<dir>/tasks.md` is named by the implementation for every change whose glob
  matched nothing, and creating it would be an easy mistake to make with an `OpenOptions`
  call written the wrong way round.

## Decisions

**1. The enumeration and counting rules are copied from the CLI, not designed.** The same
argument `task-parsing` made for `TASK_LINE_PATTERN` applies with more force here, because
this change owns the *set* of rows as well as their numbers. The alternative — require a
marker file so an accidental directory is not shown as a change — is the more defensible
rule in isolation and the wrong one here: the CLI removed exactly that requirement, on
purpose, because it hid changes that `openspec new change` had just scaffolded. Adopting the
CLI's rule costs a paragraph and buys agreement; inventing one costs a row that appears and
disappears as the CLI result lands.

Copying is applied to the two rules a user can observe through `openspec list --json` — what
counts as an active change, and what a change's task pair is. It is deliberately **not**
applied to the archived listing. Two archived listings do exist in the CLI —
`dist/utils/item-discovery.js:41-53`, behind shell completion, and
`dist/commands/validate.js:360-373` — but neither is a command whose output a user reads,
neither splits the date prefix, and both simply sort raw directory names, so there is
nothing there to agree with. The plugin's own rule (strip the prefix, dated first, undated
last) is written down in `change-enumeration` instead. Both listings do exclude
dot-prefixed entries, which is the one thing the archive rule borrows from them.

**2. No subprocess and no deferred subprocess hook.** `repo-resolution` shipped
`npm_prefix_deferred`, an injected `&dyn Fn() -> Option<PathBuf>` returning nothing, with a
test pinning the empty result so `subprocess-seam` would go red. This change deliberately
does not do the same thing, and the difference is worth stating because the superficial
shape is similar.

`resolve::openspec_bin` is **one ordered chain** whose fourth step needs a process. There is
no way to express steps 1 to 3 without deciding what step 4 does, so the step had to be
present and inert. `changes::from_files` and `changes::from_cli` are **two whole producers**
of one type. The seam between them is the function boundary, which already exists and is
already testable. Adding a deferred `contextFiles` hook inside `from_files` would put CLI
knowledge inside the file reader, which is precisely the divided reader `SPEC.md` → Data
layer rejects when it insists the two are "not primary and fallback but fast and
authoritative". The Phase 3 hand-over is instead pinned by the compile-time gate in
Contracts, which is a stronger guarantee than an inert closure: an inert closure goes red
when someone forgets to replace it, while a compile error goes red when someone forgets to
*consider* the other producer.

**3. No new dependency for globbing.** Three options were weighed.
`glob` and `globset` would both express the pattern in one line; `globset` pulls
`regex-automata` and `aho-corasick` into a crate that has two dependencies today, and
`glob`, while small, still would not reproduce `fast-glob`'s behaviour — the CLI's matcher
is `picomatch` through `fast-glob`, whose semantics differ from `glob`'s at exactly the
edges that matter (dot handling, `**` at zero levels, and a `?` in a directory segment that
`fast-glob` silently resolves to nothing). Buying a dependency and *still* having a
divergence to document is the worst of both.
Reproducing `picomatch` by hand was rejected as far out of scope.
What is left is a supported subset small enough to specify in five lines, covering every
shape the vendored `tdd` schema and the CLI's own built-ins use, with everything outside it
degrading visibly. `Cargo.toml` is untouched, so `plugin-build`'s argued-dependency-set
requirement holds unchanged and needs no delta.

**4. The unsupported-glob case records a problem and yields nothing.** The alternative —
fall back to "every file under the pattern's static prefix" — was rejected because it
over-reports, and over-reporting is only harmless for a tab. For the *tasks* artifact it
produces a task count that is wrong in a way nobody can see until the CLI corrects it, which
is the one symptom the dual-source model exists to prevent. Yielding nothing plus a named
problem is wrong in a way that is visible and that `degraded-states` can render.

**5. `**` must be the last directory segment.** `specs/**/nested/*.md` is expressible in
`fast-glob` and is not in the subset. Supporting it would mean matching a suffix of the path
as well as a prefix, which is the point where a subset stops being a subset and starts being
a matcher. No OpenSpec schema uses the shape, and it degrades with a named problem rather
than silently.

**6. Paths are not canonicalized.** The CLI canonicalizes every path it reports;
`from_files` joins onto the repository root that `resolve::find_repo` already canonicalized
and stops there. Canonicalizing per artifact file is a syscall per file per refresh for a
value whose only use is opening the file, and a canonicalized path is not the name the user
recognises — the same argument `repo-resolution` made for not canonicalizing the resolved
binary. The reason it costs nothing is Contracts' merge rule: Phase 3 keys on the artifact
id, so the two producers never compare path strings.

**7. Directory symbolic links are not followed during a glob walk, and change directories
that are symbolic links are not listed.** The two rules have different sources. Not listing
a symlinked *change* directory is the CLI's behaviour, reproduced exactly and for the
agreement reason. Not descending into a symlinked directory during the artifact walk is a
knowing divergence: `fast-glob` is configured to follow links, and the CLI defends itself
against the resulting cycles by *throwing*, which this plugin may not do because it never
fails closed. Refusing to descend makes a cycle impossible without needing a depth cap or a
visited set, and a file reachable only through a link inside a change directory is not a
shape any OpenSpec workflow produces. Recorded in Risks.

**8. Every test tree is built at run time; no fixture is checked in, and `SPEC.md` →
Fixtures is corrected rather than deferred again.** That section promises three checked-in
fixture *repositories* under `tests/fixtures/`, and `task-parsing`'s review deferred the
correction to this change on the assumption that this is where fixture repositories get
built. Measuring the scenarios changed the answer twice over.

First, this change cannot use them: it needs an empty directory, a symbolic link, a dangling
symbolic link, and a directory at mode `0o000`, and git can store the second and third and
neither of the others. Two fixture mechanisms for one module is a cost paid in every future
edit.

Second, the obvious place to hand them on — `list-view` — cannot use them either, because
`SPEC.md`'s own render seam says views are pure functions from a `Dashboard` state value to
a frame and perform no I/O. A view test builds a `ChangeSet` value; it never opens a
repository. So no change in the roadmap has a reason to read a checked-in fixture
repository, and the section is describing a mechanism that never arrives.

`SPEC.md` → Fixtures is therefore rewritten to describe the two mechanisms the crate
actually uses: run-time `ScratchDir` trees for every filesystem edge, and `include_str!`
corpora such as the existing `tests/fixtures/tasks/` for pure parsers whose input is bytes.
That also discharges `task-parsing`'s deferred note about `tests/fixtures/tasks/` being a
shape the section does not describe.

**8a. `SPEC.md` assigns "how progress falls back" to `tasks-tab`, and this change takes it
back.** The Degraded states row for "Schema loads with no tasks artifact" ends "(its
rendering, and how progress falls back, are `tasks-tab`'s decision)". Rendering still is.
The fallback is not, and cannot be: `openspec list --json` already reports a pair for such a
change, computed from `<changeDir>/tasks.md`, so leaving the question open until Phase 4
would mean shipping a file path that disagrees with the CLI for two whole phases. The clause
is corrected as part of this change rather than left to contradict `change-artifacts`.

**9. `progress` is a `Progress`, not an `Option<Progress>`.** `task-parsing`'s design
predicted the `Option`, reasoning that a schema naming no tasks artifact has no progress to
report. Reading `getTaskProgressDetailForChange` shows the CLI substitutes
`<changeDir>/tasks.md` and reports a pair regardless, so the `Option`'s `None` case would be
a state the CLI never produces — a field one producer could fill and the other could not,
which is the divergence shape this change exists to prevent. Whether the tasks *tab* is
shown remains a schema question (`Schema::tasks`), answered where `detail-view` already
holds the schema, and `SPEC.md` → Degraded states already assigns that decision to
`tasks-tab`.

**10. Active changes sort by name, and no timestamp is carried.** Discussed in Contracts:
the ordering has to be decided by whichever producer lands first, because two producers
disagreeing about order is a visible reorder. Name-ascending in byte order is deterministic,
machine-independent, needs no `mtime` walk, and matches the plain `.sort()` the CLI's own
identifier listings use (`dist/utils/item-discovery.js:21`). It is deliberately *not*
`openspec list --sort name`, which uses `localeCompare`; nothing in the plugin ever consumes
that ordering, since both producers sort for themselves.
The rejected alternative, most-recently-modified first, matches `openspec list --json`'s
default display order and would require `from_files` to reproduce a recursive maximum-mtime
walk with a directory-mtime fallback — reproducible, but a second rule to keep in agreement
in exchange for an ordering no specified view asks for.

**11. `openspec/config.yaml` is read once and schemas are cached by name, per call.**
`schema-model` made `read_file`, `FileText`, and `declared_name` `pub(crate)` for this
caller specifically, and honouring that is what keeps a repository of twenty changes from
parsing the same YAML twenty times on every watcher event. The cache is a local `HashMap`
owned by the `from_files` call, never a `static`, for `BinCache`'s reason: parallel test
threads share a process.

**12. Three capabilities rather than one.** `change-model` is separated from the two
producers deliberately: `changes-from-cli` will add requirements to it, and a capability
that both producers amend is where the agreement rule belongs. Splitting enumeration from
artifact resolution follows the two halves of the roadmap row and keeps each spec's
scenarios reachable by a test that does not have to build the other half's tree.

## Risks / Trade-offs

- **The CLI's enumeration or counting rule changes in a later release** → the plugin drifts
  back into disagreement silently. Mitigated by recording the exact source files, functions,
  and package version here, and by a CHECK task that re-reads the installed package at
  implementation time rather than trusting this document. Not eliminable: this is an
  agreement with a program that ships on its own schedule, and the honest mitigation is that
  the drift is discoverable in two `grep`s by anyone who wonders.
- **The CLI is internally inconsistent about dot-directories** → `openspec list --json`
  includes them, while the discovery module behind `openspec show` and shell completion
  excludes them. The plugin cannot match both. It matches `list`, because that is the
  command `changes-from-cli` parses and the one whose disagreement would be visible in the
  pane. The consequence is that a stray `.something/` under `openspec/changes/` appears as a
  change row — loudly, and identically in `openspec list`.
- **The supported glob subset is narrower than the CLI's** → a schema using `specs/*/spec.md`
  or `[az]*` in a directory segment shows an empty tab and a named problem where the CLI
  would show content. Accepted: no OpenSpec schema in circulation uses those shapes, the
  degradation is visible rather than silent, and the CLI path supplies the correct list when
  it arrives.
- **Not descending into directory symbolic links** → a file reachable only through a link
  inside a change directory is invisible to the file path. Accepted, and preferred to the
  CLI's own answer, which is to throw on a detected cycle.
- **`ScratchDir` paths are not canonical on macOS** → `std::env::temp_dir()` sits under
  `/var/folders/...` and `/var` is a symlink to `/private/var`, so a resolved path built by
  joining onto a raw `ScratchDir` root will not equal a `canonicalize`d expectation.
  Decisions 6 forbids `from_files` from canonicalizing, so the fix is at the *input*: every
  test passes `testutil::canonical(scratch.path())` as the repository root and compares
  against plain joins onto that same canonical root. Adding a `canonicalize` inside the
  implementation to make a test pass would silently reverse Decision 6.
- **Byte-order path sorting differs from JavaScript's UTF-16 code-unit sorting above the
  basic multilingual plane** → two artifact paths whose names differ only in astral-plane
  characters could order differently in the two producers. Accepted as unreachable in
  practice and recorded rather than mitigated; both orders are total and deterministic.
- **Reading a whole repository on every refresh** → `SPEC.md` prices the walk at "well under
  a millisecond" and this change adds a per-change schema lookup on top, served by the
  per-call cache. Unmeasured, and left unmeasured deliberately: `live-refresh` owns
  invalidation and is the change that will find out if this is wrong.
- **A permission-denied test needs a mode `0o000` directory, and the crate has no precedent
  for one** → `plugin-config`'s and `schema`'s "unreadable" tests all put a *directory where
  a file was expected*, which needs no permission manipulation at all; that trick cannot
  produce an unreadable *directory*, so this change introduces the technique. The risk is
  that a test panicking between removing and restoring the mode leaves a directory
  `ScratchDir::drop` cannot remove. Mitigated by setting the mode with
  `std::fs::set_permissions` (not `testutil::write_with_mode`, which writes a file), calling
  `from_files`, restoring the mode immediately, and only then asserting — so no assertion
  can fail while the directory is still locked.

## Migration Plan

None required, in either direction. The module is additive, nothing calls it yet, no
persisted data exists, and the release binary's behaviour is byte-identical before and
after. Rollback is deleting the file and its `pub mod` line.

## Visual Design

Not applicable. This change adds no user-facing view and no email template — it adds a
library module with no rendering at all. No design source exists or is needed.

## Open Questions

None. Three questions that would otherwise be open are answered upstream or downstream
rather than here: which artifact holds a change's tasks is `schema-artifacts`' already-landed
rule; how a change's tasks are *rendered* is `tasks-tab`'s; and how CLI results are merged
over these values is `changes-from-cli`'s, constrained by the two obligations Contracts
records for it.
