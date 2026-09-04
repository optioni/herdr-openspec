## Context

Phase 2 is pure transformations. `resolve` finds the repository and the `openspec`
binary, `schema` says which artifact holds a change's task checklist, and `config` and
`state` are done. What is missing is the step between "this file is the tasks artifact"
and "this change is `[4/9]`": nothing in the crate reads a task file.

Two consumers are already scheduled and they pull in different directions.
`changes-from-files` (the very next change) wants one cheap number per change for a list
row. `tasks-tab` (Phase 4) wants the whole document arranged into groups so it can draw
headings, checkbox glyphs, and a progress bar. And a third constraint sits over both:
`changes-from-cli` will produce the *same* `Change` type from `openspec list --json`,
whose `completedTasks` and `totalTasks` are computed by the CLI's own checkbox counter.
Files paint the pane, the CLI corrects it — so if the two counters disagree, the visible
symptom is a number that changes by itself a few hundred milliseconds after the pane
opens, for a change nobody edited.

That makes the central question of this change not "how should markdown checkboxes be
parsed" but "what exactly does `@fission-ai/openspec` count". It is answerable: the rule
lives in `dist/utils/task-progress.js` of the installed package (v1.11.0 on the
reference machine) and is one regular expression with a comment block explaining why it
is as permissive as it is:

```js
const TASK_LINE_PATTERN = /^\s*[-*]\s*\[([\sxX])\]\s*(.*)/;
```

Its own documentation records two decisions this design inherits rather than re-litigates:
leading whitespace is allowed so nested sub-tasks count like their parents, and fenced or
commented checkboxes are counted deliberately, because "every rule for deciding which
fence is real has an input where a stray or unbalanced ``` swallows genuine tasks" —
over-counting a documented example is loud and bypassable, under-counting a real task is
silent.

Constraints from this repository: no process spawn outside `cli` (and this change needs
none — there is nothing to defer, so no injected hook is introduced); nothing writes
inside `openspec/`; the 80% line-coverage floor; and adding a dependency is a decision to
argue rather than assume.

## Goals / Non-Goals

**Goals:**

- A `tasks` module with three entry points — `count`, `parse`, `read` — that between them
  serve a list row, a detail view, and a degraded state.
- Counts that equal the OpenSpec CLI's for the same bytes, provably and by construction,
  including the edge cases the CLI's own comments call out.
- A document model stable enough that `changes-from-files` and `changes-from-cli` agree
  on it without either one converting, and that `tasks-tab` can render without
  reshaping.
- Every behaviour reachable by a unit test over a pure function or a scratch directory,
  so the coverage floor is met by construction rather than by chasing it afterwards.

**Non-Goals:**

- Rendering anything. No `ui` code, no `TestBackend`, no 60/120-column view tests: this
  change adds no view, so the responsive-breakpoint concentration point has nothing to
  bind to.
- Deciding *which* file holds the tasks (`changes-from-files`), or asking the CLI for
  counts (`changes-from-cli`).
- CommonMark conformance. Setext headings, link reference definitions, lazy continuation,
  and tight/loose list semantics are all out of scope and stay out.
- Writing, toggling, or normalising a task file.

## Boundaries

| Piece | Where it lives | Pattern it follows |
|---|---|---|
| `count`, `parse` | new `src/tasks.rs` | pure functions of `&str`, like `schema::declared_name` and `resolve::path_candidates` |
| `read` | new `src/tasks.rs` | the filesystem edge kept to one function that maps every `Err` to a recorded problem, like `schema::read_file` and `config::load` |
| `Progress`, `Item`, `Heading`, `Group`, `Tasks` | new `src/tasks.rs` | plain data deriving `Debug, Clone, PartialEq, Eq`, so scenarios compare whole values with `assert_eq!`, like `resolve::BinResolution` and `schema::Schema` |
| `Tasks::problems` | new `src/tasks.rs` | the `Vec<String>` problems list every degrading value in this crate carries (`Config`, `BinResolution`, `Mapping`, `Schema`) |
| module registration | `src/lib.rs` | one `pub mod tasks;` line beside the existing four |
| corpus fixture | `tests/fixtures/tasks/` | `SPEC.md` → Fixtures; pulled in with `include_str!` so no test resolves a live repository path |
| scratch trees, snapshots | `crate::testutil` | `ScratchDir`, `snapshot`, `write_with_mode` already exist and are reused unchanged |

Nothing else is touched. `cli` does not exist yet and is not created here. `main.rs`,
`Cargo.toml`, `Makefile`, `herdr-plugin.toml`, and `.github/workflows/ci.yml` are
unmodified, which is why no group in tasks.md carries a manifest or CI gate.

## Contracts

The module's public surface, additive in full — no consumer exists yet, so nothing here
can break one:

```rust
pub struct Progress { pub completed: usize, pub total: usize }   // Copy
impl Progress { pub fn is_complete(&self) -> bool }              // total > 0 && completed == total
impl std::ops::Add for Progress
impl std::ops::AddAssign for Progress

pub struct Heading { pub level: u8, pub text: String }
pub struct Item    { pub checked: bool, pub text: String, pub indent: usize }
pub struct Group   { pub heading: Option<Heading>, pub items: Vec<Item> }
impl Group { pub fn progress(&self) -> Progress }

pub struct Tasks { pub groups: Vec<Group>, pub problems: Vec<String> }
impl Tasks { pub fn progress(&self) -> Progress }

pub fn count(text: &str) -> Progress;
pub fn parse(text: &str) -> Tasks;
pub fn read(path: &std::path::Path) -> Tasks;
```

No `Tasks::items()` convenience iterator is published. It was in an earlier draft of this
table and had no requirement, no scenario, and no named consumer: `tasks-tab` walks
`groups` because it renders them, and `Tasks::progress` is the only aggregate anyone
asked for. A `pub fn` with no caller is uncovered surface under an 80% line floor.

Error surface: there is none. No function returns `Result`, none panics, none has a
failure the caller must handle — every degradation is an empty value plus a string on
`Tasks::problems`. That is the shape of this crate's *composed, top-level* readers —
`config::load`, `state::read`, `resolve::openspec_bin`, `schema::select` — and it is what
"never fail closed" means at this layer. Note the precedent is narrower than "nothing in
this crate returns `Result`": `schema::parse` and `schema::load` return `Result` to their
own composer, and `state::record` returns `io::Result` because it writes. `tasks` has no
internal composer to report to and writes nothing, so the total form applies to all three
of its entry points. There is no pagination and no streaming: a task file is a few
kilobytes read whole.

Named consumers and what each one takes:

| Consumer | Takes | Why the shape holds for it |
|---|---|---|
| `changes-from-files` | `read` and `Progress` | one call per change per refresh; `Progress` is the only field it puts on `Change` |
| `changes-from-cli` | `Progress` only | builds it from `completedTasks`/`totalTasks`; equality with a file-derived value is the merge policy's precondition |
| `tasks-tab` | `Tasks`, `Group`, `Item`, `Heading` | renders groups in order, indents by `Item::indent`, draws the bar from `Group::progress` and `Tasks::progress` |
| `live-refresh` | `read` | re-invoked on a watch event; must stay cheap and must not write |
| `degraded-states` | `Tasks::problems` | the row for an unreadable artifact |

**Does this change alter the `Change` type?** It cannot: `Change` does not exist yet —
`changes-from-files` introduces it. What this change does is fix the one field on which
`from_files` and `from_cli` will have to agree, and that is why `Progress` is specified
here rather than left to whichever of the two lands first. `from_files` will obtain it by
calling `count` (or summing `count` over a directory-shaped artifact's files);
`from_cli` will obtain it by reading `completedTasks` and `totalTasks` out of
`openspec list --json`. Because `Progress` is two `usize` counts and nothing else, those
two constructions are the same value with no conversion, and the merge policy compares
them with `==`. Keeping them in agreement is therefore a property of *this* type staying
free of derived state: no ratio, no formatted string, no "unknown" variant. A change
whose schema names no tasks artifact is represented by the *absence* of progress on
`Change` (an `Option`, `changes-from-files`' decision), never by a `Progress` with a
sentinel value.

## Persistence and Rollout

- **Migration:** none. Nothing is stored.
- **Backfill:** none.
- **Seeding:** none — the corpus fixture is a checked-in test input, not seed data.
- **Cache invalidation:** none. `parse` and `read` are recomputed per call; no
  memoisation is introduced, deliberately (`live-refresh` owns invalidation, and a cache
  here would have to be invalidated by a watcher this change cannot see).
- **Index rebuild:** none.
- **Authorization:** none — a filesystem read the user's own process already has, and a
  permission failure is a recorded problem rather than an error.
- **Observability:** none beyond `Tasks::problems`, which `degraded-states` surfaces.
- **Deployment:** none. No manifest, no CI, no build-script change; the release binary's
  behaviour is unchanged because nothing calls the module yet.

## Test Boundaries

| Dependency | In acceptance test | In unit tests |
|---|---|---|
| Filesystem | not applicable — no acceptance test (see Test Strategy) | **real**, always: `read`'s tests build trees under `ScratchDir` in `std::env::temp_dir()`; no faked filesystem layer, matching `resolve` and `schema` |
| Corpus fixture (`tests/fixtures/tasks/*.md`) | not applicable | **real**, embedded at compile time with `include_str!`; no path is resolved at run time |
| The live repository tree (`openspec/`) | not applicable | **never touched.** No test reads or asserts against a path in the working repository; the archived `tasks.md` used as a corpus is copied into `tests/fixtures/tasks/`, not read from `openspec/changes/archive/` |
| The `openspec` binary | not applicable | **absent from every test.** Not invoked, not probed, not faked by any test in the suite; its counting *rule* is reproduced in the implementation instead. It is nonetheless consulted twice **outside** the suite, by one-off operational checks that are not gates: task 6.4 reads its installed `dist/utils/task-progress.js` to confirm the rule is still current, and task 9.7 runs `openspec validate`. Both are human-run commands whose output is recorded; neither is a collaborator of any Rust test, and neither is spawned by crate code — the binary itself remains `subprocess-seam`'s |
| `git` and `cargo tree` (group 6's checks) | not applicable | **real**, and outside the Rust suite. `git diff` against a base SHA captured before the first commit of this change, and `cargo tree --edges normal` against a baseline captured at the same moment. Recorded as commands with pass/fail outcomes, never committed as Rust tests — a test asserting a manifest key proves only that a string was typed twice |
| Herdr socket | not applicable | **absent.** Not reachable from this module and not referenced |
| Terminal / ratatui | not applicable | **absent.** No view is added, so no `TestBackend` appears |
| Process environment (`std::env::var`) | not applicable | **not consulted.** `read` takes a path; no environment lookup closure is introduced, because there is no environment-dependent behaviour to inject |
| Process spawn | not applicable | **absent**, and asserted so: a source scan over `src/tasks.rs` is a scenario, not a convention |
| Wall clock, threads, network | not applicable | **absent.** Every function is deterministic and single-threaded; no test sleeps, polls, or races |
| Third-party crates | not applicable | **none added.** `toml` and `yaml-rust2` remain the whole dependency set and neither is used by this module |

## Test Strategy

**No outer-loop acceptance group.** This change's outermost surface is a library API.
`src/main.rs` is untouched, `herdr-openspec ui` still prints its placeholder, and no
caller consumes a `Tasks` or a `Progress` until `changes-from-files`. An acceptance test
would have to drive an entry point that ships to nobody, and the boundary it would
"integrate" — a `&str` and a filesystem path — is already the unit tier's real
collaborator. Every scenario below is reachable by `cargo test --all-features tasks::`,
which is the fastest tier that can express any of them.

**Which of the project's six required boundary states bind here.** `openspec/config.yaml`
→ `rules.specs` requires every change to cover "no openspec directory, no active changes,
a missing artifact file, a schema the CLI rejects, an unreachable Herdr socket, and a pane
narrower than 100 columns". Exactly one of the six is this change's: **a missing artifact
file**, covered by "An absent file is zero tasks and no problem" — and it is the one the
module's whole degraded path is built around. The other five bind to modules this change
does not touch: repository discovery is `resolve`'s and has landed; enumerating active
changes is `changes-from-files`'; a schema the CLI rejects is `changes-from-cli`'s; the
Herdr socket is `agent-polling`'s; and the 100-column breakpoint is `tui-shell`'s. Two
further boundary states this change adds on its own account, beyond the six: an empty
document, and a task file that exists and cannot be read.

Every row's command is `cargo test --all-features tasks::` unless stated otherwise, and
`make check` is the composite gate.

| Spec Scenario | Verification | Tier | Collaborators | Command |
|---|---|---|---|---|
| The four canonical shapes are task lines | Unit test over `count` and `parse` on a four-line document | unit | none (pure) | `cargo test --all-features tasks::` |
| Whitespace around the bullet and the box is optional | Unit test asserting both counts and both trimmed texts | unit | none | `cargo test --all-features tasks::` |
| A `+` bullet and an ordered marker are not task lines | Unit test asserting `total == 0` for three lines | unit | none | `cargo test --all-features tasks::` |
| A malformed box is not a task line | Unit test over five malformed lines, asserting `total == 0` | unit | none | `cargo test --all-features tasks::` |
| A bullet with no checkbox and prose that mentions one are not task lines | Unit test over three lines, asserting `total == 0` | unit | none | `cargo test --all-features tasks::` |
| A numbering prefix and inline markup are kept verbatim in the text | Unit test over `parse` comparing both item texts byte for byte against the source substrings | unit | none | `cargo test --all-features tasks::` |
| Both letter cases count as done | Unit test asserting `completed == 2` | unit | none | `cargo test --all-features tasks::` |
| A space, a tab, and a non-breaking space are all empty boxes | Unit test building the three lines from explicit code points | unit | none | `cargo test --all-features tasks::` |
| A checked item's state survives grouping | Unit test over `parse`, asserting per-item `checked` and group progress | unit | none | `cargo test --all-features tasks::` |
| A checkbox inside a fenced code block counts | Unit test asserting `total == 2` on a document with a fence | unit | none | `cargo test --all-features tasks::` |
| A checkbox inside an HTML comment counts | Unit test asserting `total == 2` on a document with a comment | unit | none | `cargo test --all-features tasks::` |
| A block-quoted checkbox does not count, because the quote marker is not a bullet | Unit test asserting `total == 0` on `> - [x] quoted` | unit | none | `cargo test --all-features tasks::` |
| A byte-order mark before the first bullet does not hide the task | Unit test over a `\u{feff}`-prefixed document | unit | none | `cargo test --all-features tasks::` |
| A next-line character before a bullet is not indentation | Unit test over a `\u{85}`-prefixed line asserting `total == 0` | unit | none | `cargo test --all-features tasks::` |
| A next-line character inside the box is not an empty box | Unit test over `- [\u{85}] x` asserting `total == 0` | unit | none | `cargo test --all-features tasks::` |
| A CRLF document counts and reads the same as its LF twin | Unit test comparing whole `Tasks` values for the two documents, plus an assertion that no string contains `\r` | unit | none | `cargo test --all-features tasks::` |
| The last line counts without a trailing newline | Unit test on a document with no terminator | unit | none | `cargo test --all-features tasks::` |
| An empty document has no tasks and is not an error | Unit test over `""` and over prose-only text, asserting empty groups and empty problems | unit | none | `cargo test --all-features tasks::` |
| A fully checked file is complete | Unit test asserting `is_complete()` | unit | none | `cargo test --all-features tasks::` |
| A file with no tasks is not complete | Unit test asserting `!is_complete()` at 0/0 | unit | none | `cargo test --all-features tasks::` |
| Progress values from several files sum | Unit test over `+` and `+=` | unit | none | `cargo test --all-features tasks::` |
| A file-derived count and a CLI-shaped count compare equal | Unit test constructing a `Progress` from a literal pair and comparing with `count`'s result | unit | none | `cargo test --all-features tasks::` |
| Two headings yield two groups holding their own items | Unit test comparing the whole `Tasks` value | unit | none | `cargo test --all-features tasks::` |
| Items before the first heading form an unnamed leading group | Unit test asserting two groups and the first's `heading == None` | unit | none | `cargo test --all-features tasks::` |
| A file opening with a heading has no empty leading group | Unit test asserting group count is exactly 2 and no group has `heading == None` | unit | none | `cargo test --all-features tasks::` |
| A closing hash sequence is kept, not stripped | Unit test asserting the heading text is exactly `Group ##` | unit | none | `cargo test --all-features tasks::` |
| Indented, over-long, and unspaced hashes are not headings | Unit test asserting one unnamed group holding three items | unit | none | `cargo test --all-features tasks::` |
| A deeper heading closes the group above rather than nesting | Unit test asserting two groups at levels 2 and 3 with one item each | unit | none | `cargo test --all-features tasks::` |
| Repeated and empty headings are all kept | Unit test asserting four groups with the exact heading texts | unit | none | `cargo test --all-features tasks::` |
| Prose between items is dropped and does not split a group | Unit test asserting one group with two items | unit | none | `cargo test --all-features tasks::` |
| A nested sub-task is a sibling item carrying its indent | Unit test asserting indents `[0, 2, 1]` and group progress | unit | none | `cargo test --all-features tasks::` |
| Indent does not affect membership of the preceding group | Unit test asserting the item is in group `G` and `H` is empty | unit | none | `cargo test --all-features tasks::` |
| A real change's task file counts the same both ways | Unit test looping a five-document corpus (one embedded real archived `tasks.md`) asserting `parse(t).progress() == count(t)` | unit | corpus fixture (real, `include_str!`) | `cargo test --all-features tasks::` |
| A real change's task file counts the same both ways | Second row: the same corpus test also asserts the copied archived document's absolute `completed` and `total` against a pair derived by an **independent oracle** — the CLI's own regex run over the same bytes in `node`, recorded in a comment beside the assertion — so an implementation counting nothing cannot pass by agreeing with itself, and the expected pair is not read out of the code under test | unit | corpus fixture (real); the CLI's rule as a one-off oracle at fixture-copy time | `cargo test --all-features tasks::` |
| Text with no heading still agrees | Unit test on a headingless document of five task lines, two checked, asserting `completed == 2` as well as `total == 5`. The `completed` half is load-bearing: every archived `tasks.md` in the corpus is 100% complete, so without it a hardcoded `checked = true` passes the whole corpus | unit | none | `cargo test --all-features tasks::` |
| An absent file is zero tasks and no problem | Unit test over a `ScratchDir` path with no file at it | unit | real filesystem | `cargo test --all-features tasks::` |
| A directory where a file was expected is one named problem | Unit test passing a real directory path, asserting exactly one problem containing it | unit | real filesystem | `cargo test --all-features tasks::` |
| A file of invalid UTF-8 is one named problem, not a parse result | Unit test writing `0xFF 0xFE` bytes and reading them | unit | real filesystem | `cargo test --all-features tasks::` |
| A readable file is parsed exactly as its text would be | Unit test asserting `read(path) == parse(text)` for the written text | unit | real filesystem | `cargo test --all-features tasks::` |
| A task tree is byte-identical after reading | Unit test taking two `testutil::snapshot`s around three `read` calls and asserting equality, plus absence of the non-existent path | unit | real filesystem | `cargo test --all-features tasks::` |
| The module names no process API | Deterministic source scan over `src/tasks.rs`, guarded so a missing file fails rather than passes | operational check | the source tree | `test -f src/tasks.rs && ! grep -nE 'std::process\|Command\|spawn\|\boutput\(\|\bstatus\(' src/tasks.rs` — **note the escaping**: written inside a table cell the `\|` are cell-escaped pipes and must be typed as bare `|` in the shell, which is why tasks.md 6.2 carries the runnable form and this cell does not |
| (cross-cutting) Coverage floor holds with the module added | The gate itself | operational check | the whole crate | `make coverage` |
| (cross-cutting) The dependency set is unchanged | `git diff --exit-code <base>..HEAD -- Cargo.toml Cargo.lock`, where `<base>` is the SHA captured in task 1.1 before this change's first commit | operational check | `git`, the manifest, the lockfile | `git diff --exit-code "$TASK_PARSING_BASE"..HEAD -- Cargo.toml Cargo.lock` |
| (cross-cutting) The resolved build graph is unchanged | `cargo tree --edges normal` compared against the baseline captured in task 1.1, catching a dependency arriving through a feature flag rather than a manifest line | operational check | `cargo`, the resolved graph | `diff <(cargo tree --edges normal) "$TASK_PARSING_TREE_BASELINE"` |
| (cross-cutting) The CLI's counting rule is still the rule this design copied | Re-read `dist/utils/task-progress.js` in the installed package and compare the pattern character by character; record package version | operational check | the installed `@fission-ai/openspec` | `grep -n TASK_LINE_PATTERN "$(dirname "$(readlink -f "$(command -v openspec)")")/../dist/utils/task-progress.js"` |

**Can each of these fail?** Every row was put through that question, and four of them
failed it on the first pass. What follows is what each is after repair, and what it takes
to make it go red — three of the four were caught by this change's planning review and
are recorded there:

- **The no-process scan.** Its exit status is consumed, not printed, and it is guarded by
  `test -f` so a renamed or missing `src/tasks.rs` fails rather than passing on grep's
  exit 2. The alternation must be typed as bare `|` characters: `grep -E 'a\|b'` matches
  the *literal* string `a|b`, matches nothing in any real source file, and produces a
  check that passes unconditionally. tasks.md 6.2 is the runnable copy for that reason.
  It goes red the moment anyone reaches for a subprocess in this module. It is scoped to
  `src/tasks.rs` rather than the tree because it is a per-change check, not a standing
  gate — and to be plain about it, **no standing tree-wide spawn scan exists**: neither
  the `Makefile` nor CI runs one, and `repo-resolution`'s equivalent scan was likewise a
  one-off run during that change. Making one standing would change the `quality-gates`
  capability and is not this change's work.
- **The dependency-set diff.** It must name a base SHA. `git diff --exit-code` alone
  compares the working tree to the index, and this project commits after every task
  group, so a `Cargo.toml` edit made in group 1 is already in `HEAD` by the time group 6
  runs and the check passes green over the very change it exists to catch. Task 1.1
  captures the SHA before the first commit; group 6 diffs `<base>..HEAD`. It goes red if
  any dependency is added at any point in the change.
- **The build-graph diff.** Same defect, same repair: a `cargo tree` run with nothing to
  compare against is a printed report. Task 1.1 captures the baseline; group 6 diffs
  against it. It catches the case the manifest diff cannot — a dependency arriving
  through a feature flag.
- **The corpus equality test.** It could pass vacuously if `count` and `parse` both
  returned zero, and the absolute-count row is what closes that. The absolute pair must
  come from an oracle outside this crate (the CLI's regex run over the same bytes in
  `node`), or the "expected" value is derived from the code under test. A second hole is
  narrower and just as real: every archived `tasks.md` in this repository is 100%
  complete, so a hardcoded `checked = true` satisfies the whole corpus — which is why the
  headingless fixture is deliberately mixed and its `completed` asserted.

## Decisions

**1. The counting rule is copied from the CLI, not designed.** The alternative — write
the rule a careful markdown parser would produce, skipping fenced and commented
checkboxes — was considered and rejected. It is the more defensible rule in isolation and
the wrong one here: the file path and the CLI path must produce the same number, the CLI
is the side that cannot be changed, and its authors have already recorded that they tried
the fence-aware rule and withdrew it because unbalanced fences silently swallow real
tasks. Adopting it costs one paragraph of explanation and buys exact agreement.

**2. No new dependency.** Three candidates were considered.
`pulldown-cmark` is the one the project context names for markdown, and it is
disqualified by decision 1 rather than by weight: a CommonMark event stream *cannot*
produce the CLI's counts, because it hides checkboxes inside code blocks and HTML blocks
by construction. Using it would mean parsing the document twice under two rules.
`regex` would express the pattern in one line, but pulls `regex-syntax` and
`aho-corasick` into a crate that has two dependencies today, for a pattern a
twenty-line hand scanner covers exactly.
`pulldown-cmark` remains `markdown-viewer`'s to add in Phase 4 for the *other* tabs,
where CommonMark conformance is the requirement rather than the obstacle — this decision
does not pre-empt that one.

**3. Two entry points over one shared line rule.** `count` does not build a `Tasks` and
throw it away: it runs the same per-line decision and increments two counters, so
`changes-from-files` can price a list row without allocating a `String` per item. That is
a modest saving — SPEC.md prices walking `openspec/changes/` and parsing checkboxes at
"well under a millisecond", so this is about keeping the list-view path allocation-free
by construction rather than about a measured problem, and the honest framing is
tidiness, not speed.

Two entry points do **not** mean two rules. Task 3.7 extracts the line decision into one
function both call; what stays separate is only what each does with the result. The
alternative — a single `parse` with `count` as `parse(text).progress()` — was rejected
because it makes the cheap path allocate proportionally to the number of items on every
refresh of every change, for a value the list view discards immediately. The residual
risk that the two entry points drift is covered by the corpus equality scenario, which is
what makes this decision safe to hold rather than merely defensible.

**4. Groups are a flat sequence, not a tree.** A `### Inner` following `## Outer` closes
`Outer` rather than nesting. The detail view renders a scrolling list and can indent by
`Heading::level`; a tree would force every consumer — including the progress sum — to
flatten it again. Rejected alternative: nest by level, which additionally has no
well-defined answer for a level-4 heading following a level-2 one.

**5. Headings are recognised at column zero only.** CommonMark admits up to three leading
spaces. Since the parser deliberately has no fence state (decision 6), an indented `#`
inside a code block would otherwise fabricate a group. Requiring column zero costs
nothing on real OpenSpec task files, all of which start their headings there, and removes
the one way structure can be invented from a code sample.

**6. The parser has no fence, comment, or block-quote state at all — for headings as well
as for items.** One scan, one line rule, no modes. A heading fabricated inside a fenced
sample is a cosmetic defect in one tab; a *count* changed by fence state is a
disagreement with the CLI. Keeping the parser stateless means the two can never diverge
by a state machine going wrong in the middle of a document.

**7. The whitespace alphabet is `\s` plus U+FEFF minus U+0085, in one named helper.**
JavaScript's `\s` and Rust's `char::is_whitespace` differ at exactly two code points.
U+FEFF matters in practice — a UTF-8 BOM sits immediately before a file's first
character, so a BOM-prefixed `tasks.md` would lose its first task to `is_whitespace`
alone. U+0085 matters not at all in practice and is excluded anyway, because "the CLI's
set" is a rule that can be checked, while "the CLI's set, roughly" is not. Rejected
alternative: strip a leading BOM once at the top of the scan, which fixes the realistic
case but leaves the alphabet subtly wrong everywhere else in the line.

**8. `read` lives in `tasks`, and an absent file records no problem.** The alternative was
to let `changes-from-files` read the bytes and call `parse`, keeping `tasks` perfectly
pure. Rejected: the absent-versus-unreadable distinction is a property of *task files* —
the CLI makes exactly this distinction for I/O errors, treating `ENOENT` as zero tasks
and pushing every other read failure onto an `unreadable` list — and pushing it outward
would have `changes-from-files` re-derive it, with nothing keeping the two in step. This
mirrors `schema`, which owns its own `read_file` edge for the same reason.

The inheritance stops at I/O errors, and the artifacts should not claim more. **Invalid
UTF-8 is a measured divergence, not an inherited rule**: Node reads with replacement
characters, so the CLI still returns a count for a file holding invalid bytes, while
Rust's UTF-8 read fails and this module reports zero tasks with a named problem. Lossy
decoding was considered as the way to preserve agreement and rejected — it assigns a
confident count to bytes nobody can read, and this is precisely the case the dual-source
model handles well: the file path reports nothing and says why, the CLI path arrives and
corrects it. The divergence is recorded in the `task-groups` requirement text and in
Risks below rather than being left for someone to discover.

**9. Item text and heading text are verbatim after trimming.** The `1.2` numbering prefix
is kept, no trailing `##` closing sequence is stripped, and no inline markdown is
resolved. The CLI keeps the description verbatim too, so this is one more place where not
being clever is what makes the two agree. `tasks-tab` may render inline emphasis later;
that is a rendering decision over an unmodified string.

**10. Prose between items is dropped.** The model holds groups and items and nothing else.
A user who wants the surrounding narrative has the markdown viewer on every other tab,
and `tasks-tab`'s specified content is groups, glyphs, and a bar.

**11. The corpus fixture is copied into `tests/fixtures/tasks/`, not read from the live
archive.** Reading `openspec/changes/archive/.../tasks.md` at test time would be legal
(reads are permitted) but would make the suite's expected counts change whenever a change
is archived, and would put a live repository path inside a test — the thing
`repo-resolution` established must not happen. `include_str!` embeds the bytes at compile
time, so there is no path resolution at run time at all.

**12. `Progress` is `Copy` and supports `+` / `+=`.** A schema's tasks artifact may be a
directory glob resolving to several files, and the CLI sums them into one pair
(`getTaskProgressDetailForChange` loops the resolved file list and accumulates). Summing
belongs to whoever resolved the paths — `changes-from-files` — so this change ships the
arithmetic and not the file enumeration. This is admittedly forward-looking: the vendored
`tdd` schema's tasks artifact is a single `tasks.md` (`generates: tasks.md`, and
`apply.tracks: tasks.md`), so on this repository the sum is always over one file and the
operator is exercised only by its own unit test until a glob-shaped schema appears. It is
four lines and it is the difference between `changes-from-files` summing correctly and
`changes-from-files` inventing its own accumulator.

## Risks / Trade-offs

- **The CLI's counting rule changes in a later `@fission-ai/openspec` release** → the
  plugin would silently drift back into disagreement. Mitigated by recording the exact
  pattern, the file it came from, and the package version in `SPEC.md`, and by a CHECK
  task that re-reads the installed package at implementation time rather than trusting
  this document. Not fully eliminable: this is an agreement with a program that ships on
  its own schedule, and the honest mitigation is that the drift is discoverable in one
  `grep` by anyone who wonders.
- **Counting fenced and commented checkboxes over-reports documents that show examples**
  → accepted, deliberately, and it is the CLI's number: a change whose `tasks.md`
  documents a checkbox in a code sample reads one task heavier in both tools, visibly and
  identically. The alternative failure — a real task neither tool counts — is silent.
- **Two counting implementations (`count` and `parse`) can drift** → the corpus
  equality scenario is the guard, and it carries an absolute-count assertion so it cannot
  pass by both sides being empty.
- **The exotic code-point scenarios (U+FEFF, U+0085, U+00A0) have no real-world corpus** →
  they are pinned by direct unit tests built from explicit escapes rather than from a
  fixture file, so an editor normalising the fixture cannot quietly disarm them.
- **A task file holding invalid UTF-8 gets a count from the CLI and none from the file
  path** → the one knowing divergence in this change, measured rather than assumed: Node
  decodes with replacement characters and reports (for example) 1/2, while this module
  reports 0/0 plus a problem. Mitigated by the dual-source model doing exactly what it
  exists for — the CLI's count arrives and corrects the pane — and by `Tasks::problems`
  naming the file so the blank is explained rather than mysterious. Accepted rather than
  eliminated: the alternative, lossy decoding, assigns a confident count to unreadable
  bytes. Recorded in the `task-groups` requirement text so `degraded-states` inherits it.
- **`indent` counts characters, not display columns** → a file mixing tabs and spaces
  indents slightly oddly in the tasks tab. Accepted: the count is unaffected, the CLI has
  no opinion, and `tasks-tab` can widen the rule later without touching the counting path.

## Migration Plan

None required, in either direction. The module is additive, nothing calls it yet, no
persisted data exists, and the release binary's behaviour is byte-identical before and
after. Rollback is deleting the file and its `pub mod` line.

## Open Questions

None. Two questions that would otherwise be open are answered upstream rather than here:
which file holds a change's tasks is `schema-artifacts`' already-landed rule, and how a
directory-shaped tasks artifact expands to several files is `changes-from-files`', which
this change serves with `Progress` addition.
