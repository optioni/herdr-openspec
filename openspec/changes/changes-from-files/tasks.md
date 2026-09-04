<!-- No outer-loop acceptance group: design.md → Test Strategy records why. This change's
     outermost surface is a library API — `src/main.rs` is untouched and nothing consumes a
     `ChangeSet` until `tui-shell` — so an acceptance test would drive an entry point that
     ships to nobody. `from_files` over a complete scratch repository *is* the integration,
     and it runs in the unit tier at unit-tier speed against its real collaborator, a
     directory tree.

     No group carries a `parallel-after` marker. Groups 2 through 8 all edit
     `src/changes.rs`, and one file is shared mutable state.

     Group order is inside-out: the two pure string rules (3, 4), then the filesystem edges
     that use them (5, 6), then enumeration (7), then the composition (8). Group 2 comes
     before all of them because it introduces the type every later group builds, and because
     the two-producer gate is worth freezing at the moment the type is frozen rather than
     after seven groups have piled onto it. Group 9's scans run again in 10.3, because group
     10 edits the file they scan.

     All nine concentration points from `openspec/config.yaml` are accounted for, and the
     four that do not bind are named as not binding rather than left to look satisfied.
     Scheduled: **nothing spawns outside `cli`** — 9.1 for the module source (guarded, and
     with the alternation typed as bare `|`) and 9.8 for the build graph, which is the half
     a source grep cannot see. **Nothing writes inside `openspec/`** — 6.5 for one change
     directory, including an absence assertion on the `tasks.md` the progress fallback names
     for every change whose glob matched nothing, and 8.6 for a whole repository; both are
     ordinary RED tests taking two `testutil::snapshot`s and comparing them, so a
     `create_dir_all` is visible rather than only a changed file. **An absent case for every external dependency** — the filesystem
     is the only external dependency this change has, and it has five absent cases: no
     `openspec/`, no `openspec/changes/`, and no `archive/` — all three in 7.8 — plus no
     artifact file (5.4) and no tasks file (6.4). **`from_files` and `from_cli` produce the same
     type** — the whole of group 2, whose 2.6 is the recorded compiler error proving the gate
     is real, plus 9.2 and 9.3, which pin the three ways the gate could be quietly disarmed
     (a `Default`, a `..` functional update inside a `Change` literal, and a field carrying
     derived state).
     **Coverage counts the whole crate** — 12.6 holds the floor, 1.2 forbids adding anything
     to `src/lib.rs` beyond the one `pub mod` line, and 9.4 checks that diff. Not applicable,
     deliberately: no view is added, so "views perform no I/O" and "view tests at 60 and 120
     columns" have nothing to bind to; no attribution path exists, so "attribution must not
     guess" is `agent-attribution`'s; and no agent is named, so the 32-character Herdr cap
     does not apply.

     Contract gate: two fire. 2.7 gates `Change`, `ChangeSet`, `ArtifactRef`, and `Origin` at
     the moment they are frozen — that is `changes-from-cli`'s entire consumed surface and
     the whole subject of `rules.design`'s "how are the two kept in agreement" question. 8.9
     gates `from_files` once the composition closes the public surface. This change consumes
     `schema`'s and `tasks`' published contracts without altering either, and alters no
     plugin manifest key, no `config.toml` key, and no keybinding, so the project rule's
     manifest/config gate has nothing to bind to.

     Persistence gate: none applies, and design.md → Persistence and Rollout says so per
     item. Nothing is stored, migrated, indexed, or backfilled. The two caches this change
     introduces live inside one `from_files` call and are dropped with it; 8.8 is the task
     that records why neither is a `static` and confirms none was introduced.

     Labels are honest about which tasks can fail. 2.6 and every task in group 9 are one-off
     probes or evidence over code an earlier task has already written, so they carry `CHECK`,
     never `RED`: the schema requires that label for evidence that cannot honestly produce a
     red result, and dressing it as RED is the "verification that cannot fail" failure mode
     this repository has already shipped twice. The two containment tests (6.5, 8.6) are the
     opposite case and are labelled `RED` accordingly — they are ordinary committed tests
     asserting a real behaviour ("this code writes nothing"), they fail before the function
     exists, and they would fail again if it started writing. Calling them `CHARACTERIZE`
     after the GREEN, as an earlier draft did, would have been the mirror-image dishonesty.

     None of group 9's command checks may be committed as a Rust test. A test that reads
     `Cargo.toml` and asserts two dependencies proves only that the string was typed twice;
     the command that consumes the manifest is the check. -->

## 1. Baselines and module scaffold
<!-- kind: operational -->

- [x] 1.1 CHECK: Before the first commit of this change, capture the two baselines group 9
      compares against, because both of its checks are green by construction without them —
      this project commits after every task group, so by group 9 any `Cargo.toml` edit is
      already in `HEAD` and a `git diff` against the working tree sees nothing. Record
      `git rev-parse HEAD` as the base SHA and save `cargo tree --edges normal > <baseline>`
      to a path **outside** the repository. Write both values into this task when you run it,
      so group 9 does not have to guess them

      **Base SHA:** `bf71421d1234264b34bfb6e3e11b474781ad0140`
      **Baseline `cargo tree`:** saved to
      `/private/tmp/claude-501/-Users-juusopiikkila-Code-herdr-openspec/d5be0e1c-512c-496f-bdf6-1ea630cd0644/scratchpad/cargo-tree-baseline.txt`
- [x] 1.2 CHANGE: Create `src/changes.rs` with a module doc comment naming the module's job
      in one sentence and pointing at
      `openspec/changes/changes-from-files/design.md`, matching the opening of
      `src/schema.rs` and `src/tasks.rs`; register it with one `pub mod changes;` line in
      `src/lib.rs` beside the existing five. Add **nothing else** to `src/lib.rs` and nothing
      at all to `src/main.rs` — logic in either is coverage's blind spot, which is the reason
      the crate keeps them thin
- [x] 1.3 VERIFY: `cargo build` succeeds with the empty module registered, so a failure in
      group 2 is never "the module is not registered"

## 2. The `Change` model and the two-producer gate
<!-- kind: behavior -->

Covers the `change-model` scenarios "The three-way status split is derived, not stored" (the
unit half; 9.3 owns the source scan), "Adding a field to `Change` fails to compile in the
shared conformance function", and the definition every later group builds against.

This group is deliberately first. `changes-from-cli` consumes this type unchanged, and
`rules.design` asks how the two producers are kept in agreement — the answer is a compile
error in `conformance::assert_invariants`, and freezing it here rather than after seven
groups of accumulated fields is what makes it a gate rather than a retrofit.

The conformance function's own behaviour is genuinely testable, which is why this group can
carry an honest RED without a filesystem: it accepts a well-formed value and rejects each
malformed one, and those are the assertions.

- [x] 2.1 RED: Write failing unit tests in `src/changes.rs` for
      `conformance::assert_invariants`, building `Change` values by hand with no filesystem:
      a well-formed active value passes; a value with an empty `name` panics; a value with
      an empty `schema` panics; a value holding an `ArtifactRef` with an empty `id` panics;
      a value carrying an empty string in `problems` panics; an `Active` value whose
      `dir`'s final component differs from `name` panics; and an `Archived` value whose
      `dir`'s final component *ends with* `name` passes while one that does not panics.
      Assert the panic and its message with `#[should_panic(expected = ...)]` so a function
      that asserts nothing cannot pass. Add one test that must **pass**: a value whose
      `artifacts` hold two refs with the *same* id. Artifact ids are not unique — the landed
      `schema-artifacts` capability requires a schema's list to be kept verbatim and "never
      de-duplicated", and it ships a scenario producing `zeta, alpha, middle, zeta` — so a
      uniqueness invariant would panic on a value `from_files` legitimately produces
- [x] 2.2 RED: Add the test that pins the derived-state rule from the type's side: construct
      the three-way split (`no-tasks`, `complete`, `in-progress`) from `progress.total`,
      `progress.completed`, and `progress.is_complete()` for three `Change` values, and
      assert each. Be honest about what this catches: on its own it is close to a tautology
      over `Progress`, and the check that actually goes red when someone adds a `status` field
      is 9.3's source scan. Its value is that it is the *worked example* a future reader finds
      when they wonder where the split lives, and it has to be deleted to make room for a
      stored one
- [x] 2.3 GREEN: Define `Origin`, `ArtifactRef`, `Change`, and `ChangeSet` exactly as
      design.md → Contracts specifies, deriving `Debug, Clone, PartialEq, Eq` and
      **deliberately not** `Default`. Write the reason for the missing `Default` into a doc
      comment on `Change` — one or two lines, naming the compile error it buys, because a
      derive that looks accidentally omitted gets added back
- [x] 2.4 GREEN: Implement `#[cfg(test)] pub(crate) mod conformance` with
      `assert_invariants(&Change)`, opening with the exhaustive `let Change { name, dir,
      origin, schema, artifacts, progress: _, problems } = change;` pattern carrying **no**
      `..` rest pattern. Every field must be *mentioned*, but a field no invariant reads must
      be bound as `field: _`, not as a plain name: a plain unused binding is
      `unused_variables`, which `-D warnings` turns into a clippy failure, and the obvious
      way out is the rest pattern that deletes the gate. `field: _` keeps the exhaustiveness
      error (`E0027: pattern does not mention field`) and produces no warning
- [x] 2.5 REFACTOR: Keep the invariant assertions one per statement with a distinct message
      each, so a failure names which invariant broke rather than which function it broke in
- [x] 2.6 CHECK: Prove the compile-time gate is real rather than asserting it. Add a
      throwaway field to `Change`, update **only** `from_files`' future construction site (or,
      at this point in the change, only the test builders), run
      `cargo test --all-features`, and record the compiler errors verbatim in this task.
      Expect **two** and record both: `E0063: missing field ... in initializer of Change` at
      each construction site not updated, and `E0027: pattern does not mention field` at
      `conformance::assert_invariants`. `rustc` names the field and the source line, not the
      enclosing function, and it *suggests* adding `..` — do not take the suggestion; that is
      the gate being offered back to you. Then revert. This cannot be a committed test,
      because a test that fails to compile fails the suite permanently; the recorded error is
      the evidence, and design.md → Test Strategy says so

      **Evidence.** Added `pub throwaway_probe_field: bool` to `Change`, updated only
      `well_formed_active`'s literal (leaving `well_formed_archived`'s unpatched), ran
      `cargo test --all-features`:

      ```
      error[E0027]: pattern does not mention field `throwaway_probe_field`
        --> src/changes.rs:74:13
      error[E0063]: missing field `throwaway_probe_field` in initializer of `changes::Change`
         --> src/changes.rs:149:9
          |
      149 |         Change {
          |         ^^^^^^ missing `throwaway_probe_field`
      ```

      Both name the field and the source line, not the enclosing function, exactly as
      predicted. `rustc` additionally suggested "include the missing field in the pattern"
      and "if you don't care about this missing field, you can explicitly ignore it" (via a
      rest pattern) at the `E0027` site — the gate being offered back, not taken. Reverted
      immediately after capturing this output; `cargo test --all-features changes::` passed
      11/11 both before and after the probe.
- [x] 2.7 CHECK: Contract gate. Re-read design.md → Contracts and confirm the frozen surface
      matches it field for field: seven fields on `Change`, three on `ChangeSet`, two on
      `ArtifactRef`, `Origin`'s two variants, no `Default`, no `#[non_exhaustive]`, no
      `status`, no ratio, no formatted string, no `lastModified`, and no producer
      discriminant. Confirm `progress` is a `Progress` and not an `Option<Progress>`, and
      that design.md → Decisions 9 records why that differs from `task-parsing`'s prediction

      Confirmed field-for-field against `src/changes.rs`: `Change` has exactly `name`,
      `dir`, `origin`, `schema`, `artifacts`, `progress`, `problems` (seven); `ChangeSet`
      has `active`, `archived`, `problems` (three); `ArtifactRef` has `id`, `paths` (two);
      `Origin` has `Active` and `Archived { date: Option<String> }` (two variants). No
      `Default`, no `#[non_exhaustive]`, no `status`/ratio/formatted-string/`lastModified`/
      discriminant field anywhere. `progress: crate::tasks::Progress`, not wrapped in
      `Option`, matching Decisions 9's correction of `task-parsing`'s prediction.
- [x] 2.8 Run the group tests — `cargo test --all-features changes::` — and confirm no
      regressions in the rest of the suite

      `cargo test --all-features` — full suite green (11/11 in `changes::`, no regressions
      in `config`, `state`, `resolve`, `schema`, `tasks`, or `tests/cli.rs`).

## 3. Splitting the archive date prefix
<!-- kind: behavior -->

Covers the `change-enumeration` scenarios "A normal archived directory splits into a date and
a name" (the pure half; 7.1 owns the `from_files` half), "An impossible date is still a date
prefix", "A malformed or absent prefix keeps the whole name", and "A prefix with nothing
after it keeps its whole name".

- [x] 3.1 RED: Write failing unit tests for `split_archive_name(&str) -> (Option<String>,
      String)` from the four scenarios above, every input a Rust string literal and no
      filesystem involved
- [x] 3.2 RED: Make the negative tests discriminating rather than merely green-able. Include
      `2026-1-1-single-digits` (a rule using a loose digit run accepts it), `20260814-nohyphen`
      (a rule matching digits and hyphens anywhere accepts it), `2026-08-14` with no trailing
      hyphen (a rule testing only the ten leading characters accepts it), and
      `2026-08-14-` (a rule that strips eleven characters unconditionally yields an empty
      name). Put a genuinely dated entry in the same test and assert its split too, so an
      implementation that never splits anything cannot pass by returning the input

      Verified RED: stubbed `split_archive_name` to always return `(None, dir_name)` and
      confirmed 3 of 15 tests failed with the wrong-value assertion (not a compile error) —
      `a_normal_archived_directory_splits_into_a_date_and_a_name`,
      `an_impossible_date_is_still_a_date_prefix`, and
      `a_malformed_or_absent_prefix_keeps_the_whole_name`'s dated case — before restoring
      the real implementation.
- [x] 3.3 GREEN: Implement `split_archive_name` as a hand-written character test for exactly
      the CLI's `^\d{4}-\d{2}-\d{2}-` — four ASCII digits, `-`, two, `-`, two, `-` — with no
      calendar validation, because the CLI writes with that pattern and validates no further.
      Return `(None, whole_name)` when the pattern does not match **or** when the remainder
      after it is empty
- [x] 3.4 REFACTOR: Confirm the function is total, allocation-light, and reads as the pattern
      it implements; state in the verification task if no refactor was needed

      No refactor needed: the function is a single left-to-right byte scan with no
      recursion or intermediate allocation until the two owned `String`s it must return,
      and reads directly as "four digits, hyphen, two digits, hyphen, two digits, hyphen".
- [x] 3.5 Run the group tests — `cargo test --all-features changes::` — no regressions

      15/15 passing; no regressions elsewhere in the suite.

## 4. Classifying a `generates` value
<!-- kind: behavior -->

Covers the `change-artifacts` scenarios "A plain filename is not a glob", "A brace expression
is not a glob and resolves to nothing" (the `is_glob` half; 5.6 owns the resolution half),
"Each of the three metacharacters makes a value a glob", "A wildcard inside a directory
segment is unsupported" (the `shape` half; 8.6 owns the recorded-problem half), "More than one
wildcard in the filename segment is unsupported", and "A `**` that is not the last directory
segment is unsupported" (the `shape` half).

- [x] 4.1 RED: Write failing unit tests for `is_glob(&str)`, asserting `tasks.md`,
      `specs/{alpha,zeta}/spec.md`, and `design.md` are not globs while `specs/**/*.md`,
      `notes/file?.md`, and `notes/[ab].md` are. The brace case is the one an implementation
      gets wrong by being reasonable: `{` is **not** in the CLI's metacharacter set, and a
      value containing one takes the literal-path branch in both tools
- [x] 4.2 RED: Write failing unit tests for `shape(&str) -> Result<Shape, String>` covering
      the supported subset — `specs/**/*.md`, `specs/spec-*.md`, `specs/*`, `a/b/c.md` — and
      each unsupported shape: `specs/*/spec.md`, `specs/?eta/spec.md`, `specs/[az]*/spec.md`,
      `specs/*-*.md`, `specs/**/nested/*.md`, and `specs/**/nested/**/*.md`. Assert the `Err`
      message names the pattern verbatim, so a caller can print it
- [x] 4.3 RED: Add the four boundary inputs the scenarios do not name, with the answer the
      subset rule already determines — do not leave the implementer to "assert whichever it
      does", which is green by construction. `.` is not a glob and is `Shape::Literal(".")`,
      which 5.7 then resolves to nothing because a directory is not a regular file. `specs/`
      is not a glob and is `Shape::Literal("specs/")`, likewise nothing. `*` is a glob whose
      directory list is empty and whose file pattern is a bare `*`, matching every non-dot
      regular file directly in the change directory. `**` alone is a glob with an empty
      directory list, `recursive = true`, and a bare `*` file pattern — which is
      `Err`, because the subset's final segment must be a filename pattern and `**` is a
      directory segment. Assert exactly these
- [x] 4.4 GREEN: Implement `is_glob` as the CLI's `isGlobPattern` character for character —
      contains `*`, `?`, or `[` — with a comment naming the file and function it is copied
      from, not a paraphrase of the rule
- [x] 4.5 GREEN: Implement `shape`, splitting on `/` and applying design.md's five-line
      subset: every directory segment a literal free of `*?[` except that the **last**
      directory segment may be exactly `**`; the final segment a literal, or a literal prefix
      plus exactly one `*` plus a literal suffix. Everything else is `Err` naming the pattern
- [x] 4.6 REFACTOR: Extract the "segment contains a metacharacter" test used by both halves
      so the two cannot drift apart, keeping tests green
- [x] 4.7 Run the group tests — `cargo test --all-features changes::` — no regressions

## 5. Resolving one artifact to paths
<!-- kind: behavior -->

Covers the `change-artifacts` scenarios "An artifact whose filename differs from its id
resolves correctly", "A non-glob `generates` naming something that is not a regular file
resolves to nothing", "A symbolic link to a regular file is a resolved artifact", "A brace
expression is not a glob and resolves to nothing" (the resolution half), "A nested spec tree
resolves in path order", "A glob matching nothing is an empty path list, not a problem", "Dot
entries and non-files are skipped", "A directory symbolic link is not descended into", and "A
prefix-and-suffix file pattern is supported"; and the `change-model` scenarios "Tab order
follows the schema, not the filesystem" and "A missing artifact file is an empty path list,
not a missing tab".

Two functions, and the group's RED must cover both. `resolve_artifact` takes one `generates`
value; `change_artifacts` takes a whole `Schema` and is what the two `change-model` scenarios
are actually about, since neither "tab order" nor "a tab with no content" is expressible one
artifact at a time.

This is the first group with a real filesystem. Every tree is built under
`crate::testutil::ScratchDir`. **Canonicalize the input, never the expectation:** on macOS
`std::env::temp_dir()` sits under `/var/folders/...` and `/var` is a symlink to
`/private/var`, so a path this code builds by joining onto a raw `ScratchDir` root will never
equal a `canonicalize`d expectation. Pass `testutil::canonical(scratch.path())` in as the
directory and compare against plain joins onto that same canonical root. Do **not** reach for
a `canonicalize` inside the implementation to make an assertion pass — design.md → Decisions 6
forbids it, and doing so would silently reverse a decision.

- [x] 5.1 RED: Write failing unit tests for `resolve_artifact(change_dir, generates) ->
      (Vec<PathBuf>, Option<String>)` — the resolved paths and the one problem an unsupported
      shape records — one test per `change-artifacts` scenario above, named after it
- [x] 5.2 RED: Make "An artifact whose filename differs from its id" discriminating: vendor a
      probe schema declaring `id: plan` with `generates: implementation-plan.md`, write both
      `implementation-plan.md` and a decoy `plan.md`, and assert the resolved vector holds the
      first and **not** the second. Without the decoy, an implementation that builds paths
      from the id passes
- [x] 5.3 RED: Make the ordering test assert the whole ordered vector, not membership, and
      choose names that separate byte order from locale order — `specs/Beta/spec.md` must
      precede `specs/alpha/spec.md`, which is the opposite of what a locale-aware comparison
      produces
- [x] 5.4 RED: Write the absent cases as their own tests, each asserting an empty vector and
      **no** problem: an artifact file that does not exist, a `specs/` directory that does not
      exist, and a `specs/` directory that exists and is empty. An unwritten artifact is the
      normal state of a change in flight, and recording a problem for it would fill
      `degraded-states` with noise
- [x] 5.5 RED: Write the symlink pair — a `proposal.md` that is a link to a real file
      (resolves, and the path returned is the link's own, not its target) and one that
      dangles (resolves to nothing, records nothing) — and the directory-link test, whose
      `specs/loop` points back at `specs/`. The last one must assert both that the call
      terminates and that the result is exactly the one real file; a test that only asserts
      termination passes for an implementation that returns everything
- [x] 5.6 RED: Write the brace test as a *resolution* test, not only an `is_glob` test: build
      `specs/alpha/spec.md` and `specs/zeta/spec.md`, ask for `specs/{alpha,zeta}/spec.md`,
      and assert an empty vector — the same answer the real CLI gives for the same tree
- [x] 5.7 RED: Write failing unit tests for `change_artifacts(change_dir, &Schema) ->
      (Vec<ArtifactRef>, Vec<String>)` covering the two `change-model` scenarios this group
      claims: over a `tdd`-shaped schema and a change directory holding only `tasks.md` and
      `proposal.md`, assert five `ArtifactRef`s with ids `proposal`, `specs`, `design`,
      `tasks`, `planning-review` **in that order**, two of them carrying one path each and
      three carrying none, and no problem recorded for the three. Assert the whole ordered
      vector; a per-id lookup would pass for an implementation that emits the artifacts in
      filesystem order
- [x] 5.8 GREEN: Implement the non-glob branch: join, then accept only a regular file reached
      **through** symbolic links (`std::fs::metadata`, never `symlink_metadata`), matching the
      CLI's `statSync().isFile()`
- [x] 5.9 GREEN: Implement the glob branch over `shape`'s output: walk the literal prefix
      directory, recursing only when the shape is recursive, never descending into a directory
      reached through a symbolic link, skipping every entry whose name begins with `.`,
      accepting only regular files through links, and sorting the result by full path
      ascending in byte order
- [x] 5.10 GREEN: Implement `change_artifacts`, producing one `ArtifactRef` per schema
      artifact **in schema order** and collecting each unsupported-shape problem. Emit one ref
      per artifact entry even when two entries share an id — `schema-artifacts` requires the
      list to be kept verbatim and never de-duplicated. This is the function group 8 composes;
      it does not read tasks and does not know what a change is
- [x] 5.11 REFACTOR: Extract the "is this a regular file through links" test into one helper
      used by both branches, matching `resolve::is_usable_binary`'s shape, and keep the walk
      free of `unwrap` — every `read_dir` error inside the walk means "nothing there", as in
      `resolve::nvm_candidates`
- [x] 5.12 Run the group tests — `cargo test --all-features changes::` — no regressions

## 6. Task progress and the CLI's fallback
<!-- kind: behavior -->

Covers the `change-artifacts` scenarios "A single tasks file gives the change's progress", "A
glob-shaped tasks artifact sums across its files", "A schema naming no tasks artifact still
counts `tasks.md`", "A change with no tasks file at all is zero, not complete", "An unreadable
tasks file is zero plus one named problem", and "A change directory is byte-identical after
resolution and counting".

The fallback is the whole point of this group and it is the part an implementation omits by
being reasonable: when the tasks artifact resolves to no files, the CLI substitutes
`<change dir>/tasks.md` and counts it anyway. Omitting it makes the file path report `0/0`
for exactly the changes whose schema is unusual.

- [x] 6.1 RED: Write failing unit tests for `change_progress(change_dir, Option<&Artifact>)
      -> (Progress, Vec<String>)`, one per scenario above, each building its tree under
      `ScratchDir`
- [x] 6.2 RED: Write the summing test against an **absolute** expected pair rather than a
      self-consistent one. Build `tasks.md` at 1/2, `sub/tasks.md` at 2/3, and
      `sub/deeper/tasks.md` at 0/1 under a tasks artifact generating `**/tasks.md`, and
      assert `Progress { completed: 3, total: 6 }`. Record in a comment beside the assertion
      that the same tree was driven through the real `openspec list --json` at planning time
      and reported `completedTasks: 3, totalTasks: 6`, so the expected value comes from an
      oracle outside this crate. `completed` is load-bearing as well as `total`: a file set
      that is entirely unchecked would let a hardcoded `checked = false` pass
- [x] 6.3 RED: Write the fallback as **three** separate tests, one per way the file list can
      come back empty — a schema declaring no tasks artifact, a schema that failed to load so
      no artifact is available at all, and a tasks artifact whose glob matched nothing — each
      with a real `tasks.md` at 1/2 in the change directory and each asserting 1/2. An
      implementation with the fallback missing returns 0/0 for all three. Build each probe
      schema so that it genuinely produces the case it names, and confirm that by asserting a
      *different* progress in each variant if the trees allow it — three schemas that are all
      silently invalid would collapse all three tests into one
- [x] 6.4 RED: Write the two remaining edge tests. Unreadable: a *directory* named `tasks.md`,
      asserting 0/0 and exactly one problem naming the path — `tasks::read` already produces
      that problem, and this pins that it is propagated onto the change rather than dropped.
      Absent: no `tasks.md` and no tasks artifact, asserting 0/0, an empty problems list, and
      `is_complete() == false` — the last assertion is what separates "no tasks" from "all
      tasks done", the same three-way split the CLI makes
- [x] 6.5 RED: Write the containment test. Take a `testutil::snapshot` of a change directory
      holding a nested `specs/` tree and a `tasks.md`, run resolution and counting three
      times, snapshot again, and assert equality. Then, on a change directory with **no**
      `tasks.md`, assert that the path the fallback names still does not exist afterwards —
      that is the one path the implementation constructs for every change whose glob matched
      nothing, and an `OpenOptions` written the wrong way round would create it. This is a
      `RED` and not a `CHARACTERIZE`: "this code writes nothing" is a behaviour, the test
      fails before `change_progress` exists, and it fails again if the function starts writing
- [x] 6.6 GREEN: Implement `change_progress`: resolve the tasks artifact's `generates` through
      group 5's resolver, substitute `[change_dir.join("tasks.md")]` when the result is empty,
      read each target with `tasks::read`, and accumulate with `Progress`'s `+=`. Cite the
      CLI's function and line range in a comment
- [x] 6.7 REFACTOR: Keep the fallback visible as one named expression rather than buried in an
      `if`, since it is the behaviour a future reader is most likely to "simplify" away; state
      in the verification task if no other refactor was needed
- [x] 6.8 Run the group tests — `cargo test --all-features changes::` — no regressions

## 7. Enumerating active and archived changes
<!-- kind: behavior -->

Covers the `change-enumeration` scenarios "A directory with no marker file is a change", "A
regular file is not a change", "A symbolic link to a directory is not a change", "The archive
exclusion is by exact name", "A dot-directory is a change", "Case and digits order by byte,
not by locale", "A normal archived directory splits into a date and a name" (the `from_files`
half), "A dot-prefixed archive directory is not an archived change", "Two archived directories
can strip to the same name", "Dated entries come newest first", "Two entries sharing a date
order by name descending", "An undated entry sorts after every dated one", "The limit keeps
the most recent entries", "A limit larger than the archive keeps everything", "A repository
with no `openspec` directory yields an empty set", "No active changes still lists the
archive", "An unreadable changes directory is one named problem", "An unreadable archive
leaves the active list intact", and "An `archive` that is a regular file is not an archive";
and the `change-model` scenario "A directory entry whose name is not valid UTF-8 is skipped,
not fatal".

- [x] 7.1 RED: Write failing unit tests for the two listing functions — active names from
      `<repo>/openspec/changes/` and archived entries from `<repo>/openspec/changes/archive/`
      — one per scenario above, each building its tree under `ScratchDir` and passing
      `testutil::canonical(scratch.path())` in as the root
- [x] 7.2 RED: Make the marker-file test discriminating: include an entirely empty directory,
      one holding only `.openspec.yaml`, and one holding only `proposal.md`, and assert all
      three are listed. An implementation that requires any marker fails on at least one, and
      the CLI removed exactly that requirement because it hid freshly scaffolded changes
- [x] 7.3 RED: Make the symlink test use `DirEntry::file_type`, not `Path::is_dir`, the
      distinguishing case. Build a real directory, a symbolic link pointing at it, and a
      dangling link, and assert exactly one change is listed. `Path::is_dir` follows links and
      would list two — a divergence from the CLI that no other test in this group catches
- [x] 7.4 RED: Make the exclusion test prove it is exact rather than a prefix: `archive`,
      `archives-not-excluded`, and `archive-notes` in one tree, asserting the last two are
      listed and the first is not
- [x] 7.5 RED: Write the dot-rule test as **one** test asserting both halves at once, because
      the two listings are deliberately opposite and a test of either alone reads like an
      inconsistency: `.dot-change/` under `openspec/changes/` is listed, and
      `.hidden-archived/` under `archive/` is not, in the same repository
- [x] 7.6 RED: Make the ordering tests assert whole ordered vectors. For active changes use
      `Beta`, `alpha`, `10-late`, `2-early` so byte order and locale order differ visibly. For
      archived changes assert three things separately: the dated-descending order; the
      same-date pair ordering by name descending, which a sort that leaves ties to the
      filesystem fails non-deterministically; and the mixed case where one dated and two
      undated entries must come out dated-first with the undated pair descending by name — a
      plain descending sort of raw names puts a letter-initial name above every date and fails
      this last one
- [x] 7.7 RED: Write the `archived_count` tests: seven dated entries at 5, two entries at 50,
      and the same two at 0, each asserting the exact resulting list
- [x] 7.8 RED: Write the degraded-directory tests: no `openspec/` at all, no
      `openspec/changes/`, no `archive/`, an `archive` that is a regular file — all four
      empty and problem-free — then `openspec/changes/` at mode `0o000` and `archive/` at mode
      `0o000`, each asserting exactly **one** problem at the right level and the correct
      surviving list. The `changes/` case is the discriminating one: `read_dir` on the archive
      beneath an unreadable parent returns `EACCES`, not `NotFound`, so an implementation that
      walks the archive unconditionally records a second, redundant problem and fails the
      "exactly one" assertion. Set the mode with `std::fs::set_permissions` — not
      `testutil::write_with_mode`, which writes a file — and **restore it immediately after
      the call, before any assertion**, so a failing assertion cannot leave a directory
      `ScratchDir::drop` is unable to remove. The crate has no precedent for this: every
      existing "unreadable" test puts a directory where a file was expected, which needs no
      permission manipulation at all
- [x] 7.9 RED: Write the invalid-UTF-8 test as a **pure unit test over the name-decoding
      step**, given an `OsString` built with `std::os::unix::ffi::OsStringExt::from_vec` over
      bytes that are not valid UTF-8, beside one built from a normal name. Do **not** create
      such a directory: APFS rejects the name with `EILSEQ` (errno 92) at `mkdir`, so a
      filesystem test fails on the reference machine for a reason unrelated to the plugin,
      while the behaviour still matters on Linux. Assert the normal name is returned, the
      undecodable one is not, exactly one problem names it, and nothing panics
- [x] 7.10 GREEN: Implement the active listing: `read_dir`, keep entries whose
      `file_type()?.is_dir()` holds and whose name is not exactly `archive`, decode names to
      `String` and record a problem on the set for each that will not decode, sort ascending
      by byte order. Cite `dist/core/list.js`'s filter in a comment
- [x] 7.11 GREEN: Implement the archived listing: the same directory test, excluding names
      beginning with `.`, splitting each with `split_archive_name`, ordering dated entries by
      date descending then name descending, placing undated entries after all dated ones
      ordered by name descending, and truncating to `archived_count`
- [x] 7.12 GREEN: Handle the two top-level `read_dir` results by **matching on the `Result`**,
      not by `flatten()`: `NotFound` is no problem at all, and any other error is exactly one
      problem at the set level. `flatten()` is right for the inner walks and wrong here — it
      discards the distinction the degraded-state scenarios rest on. Short-circuit the archive
      walk when the `openspec/changes/` read itself failed, so the parent's permission error
      is not reported twice
- [x] 7.13 REFACTOR: Extract the shared "directory entries that are directories, decoded"
      helper used by both listings so the two cannot drift on the symlink rule. Do **not**
      extend it to cover the dot rule: the active listing keeps dot entries and the archived
      listing drops them, deliberately, and a shared filter is exactly how that difference
      would be collapsed by a later "simplification". Keep tests green
- [x] 7.14 Run the group tests — `cargo test --all-features changes::` — no regressions

## 8. `from_files`: the composition and its degraded states
<!-- kind: behavior -->

Covers the `change-model` scenarios "A fully written active change becomes one value", "An
archived change carries the date split off its directory name", "A change whose schema did not
load is still a complete value", "The same repository read twice produces equal values, even
after a touch", "Every value a producer builds satisfies the shared invariants", "An archived
change keeps its file-derived values when the CLI arrives", "A repository-level failure is
recorded on the set, not on a change", and "Every degradation lands on a problems list rather
than in a return type"; the `change-enumeration` scenario "The repository tree is byte-identical
after enumeration"; and the `change-artifacts` scenarios "A change's own declaration wins over
the project's", "A repository declaring no schema falls back to the default", "A schema that is
not vendored leaves the artifact list empty", "A wildcard inside a directory segment is
unsupported" (the recorded-problem half), and "A `**` that is not the last directory segment is
unsupported" (the sibling-artifacts half).

- [x] 8.1 RED: Write failing unit tests for `from_files(repo, archived_count) -> ChangeSet`,
      one per scenario above, each building a complete scratch repository — `openspec/`,
      `openspec/config.yaml`, `openspec/schemas/tdd/schema.yaml`, `openspec/changes/`, and an
      `archive/` — with a small helper that writes a minimal vendored schema so each test
      states only what it varies. Pass `testutil::canonical(scratch.path())` in as the root
- [x] 8.2 RED: Write the headline test as a whole-value `assert_eq!` against a constructed
      `Change`, not a field-by-field walk, so a field the composition forgets to fill is
      caught by the same assertion that checks the ones it fills
- [x] 8.3 RED: Write the two-schema test with **both** schemas vendored and with artifact
      lists that differ, asserting each change's `schema` **and** that their `artifacts` ids
      differ. A `schema`-name-only assertion passes for an implementation that resolves the
      name per change and then loads one schema for the whole repository
- [x] 8.4 RED: Write the remaining composition tests: the "same name active and archived" pair
      (two distinct values in two lists with different `dir` values, neither list filtering the
      other); the "read twice, even after a touch" test, which advances an unrelated file's
      modification time between the two reads and still asserts `==`, so a `lastModified` field
      would fail it where a plain double read of an untouched tree would not; and the
      conformance sweep, passing every `Change` from a five-change repository through
      `conformance::assert_invariants`
- [x] 8.5 RED: Write the two degraded-composition tests. All-three-failures: an unreadable
      `archive/`, one change declaring an unvendored schema, and another whose `tasks.md` is a
      directory — asserting both active changes are present and each problem appears **exactly
      once** at the right level; a test asserting only "some problem exists" passes for an
      implementation that records the same message on every change. Unsupported glob: a
      vendored probe schema whose `specs` artifact generates `specs/*/spec.md`, asserting the
      change records one problem naming that artifact while its other artifacts still resolve
      normally
- [x] 8.6 RED: Write the whole-repository containment test. Take a `testutil::snapshot` of a
      complete scratch repository, run `from_files` three times, snapshot again, and assert
      equality. Additionally assert that `openspec/changes/archive/` still does not exist for
      a repository that had none, and that no `tasks.md` was created in any change directory
      that lacked one. Labelled `RED` for the reason 6.5 gives: "this code writes nothing" is a
      behaviour, and this test fails both before `from_files` exists and after it starts writing
- [x] 8.7 GREEN: Implement `from_files`: read `<repo>/openspec/config.yaml` **once** with
      `schema::read_file`; list active and archived directories with group 7's functions; for
      each change read its `.openspec.yaml`, call `schema::declared_name`, look the resolved
      name up in a call-local `HashMap` cache and `schema::load` it on a miss; build the
      artifacts with group 5's `change_artifacts` and the progress with group 6's
      `change_progress`; assemble each `Change` by naming every field, with no `..` functional
      update. Order the selection problems before the load problems before the artifact
      problems before the task problems, matching `schema::resolve`'s existing rule that a
      composition never rebuilds a problem list from its last step alone
- [x] 8.8 CHECK: Persistence gate. Confirm the two caches introduced here — the project config
      text and the schema map — are both local to the `from_files` call and that no `static`,
      `OnceLock`, or `lazy` value was added anywhere in `src/changes.rs`. Record that no
      migration, backfill, index rebuild, or cross-call cache invalidation applies, as
      design.md → Persistence and Rollout states per item. Run this **after** 8.7's GREEN, so
      a cache introduced by the very code it inspects is something it can see

      `grep -n 'static\|OnceLock\|lazy' src/changes.rs` — no match. `project_config_text`
      is a local `crate::schema::FileText` and `schema_cache` is a local
      `HashMap<String, CachedSchemaLoad>`, both owned by `from_files`'s stack frame and
      passed down by reference; neither survives the call. No migration, backfill, index
      rebuild, or cross-call invalidation applies — confirmed per design.md → Persistence
      and Rollout.
- [x] 8.9 CHECK: Contract gate. Re-read design.md → Contracts and confirm the complete public
      surface matches: `from_files`'s signature, the four public types, and the two
      obligations recorded for `changes-from-cli` — that it joins the artifact lists by
      **position** rather than by path or by id, and that it re-sorts the active list by name.
      Confirm no `pub fn` was added that no scenario reaches, since an uncovered public
      function is a coverage cost with no caller

      `grep -n '^pub fn\|^pub struct\|^pub enum' src/changes.rs` shows exactly `Origin`,
      `ArtifactRef`, `Change`, `ChangeSet`, and `from_files(repo: &Path, archived_count: usize) -> ChangeSet`
      — nothing else public. The two `changes-from-cli` obligations (position-based join,
      re-sort by name) are recorded in this module's doc comments and in design.md →
      Contracts, not yet exercised since `from_cli` does not exist until Phase 3.
- [x] 8.10 REFACTOR: Collapse any duplication between the active and archived per-change paths
      into one function taking the origin, so an artifact rule cannot come to differ between an
      active change and an archived one, keeping tests green

      Already collapsed in 8.7's GREEN: `build_change` takes `origin: Origin` and is the one
      function `from_files` calls for every active and every archived directory; no
      refactor step introduced further duplication to remove.
- [x] 8.11 Run the whole suite — `cargo test --all-features` — and confirm no regressions in
      `config`, `state`, `resolve`, `schema`, or `tasks`

## 9. Invariants that are commands, not tests
<!-- kind: operational -->

Covers the `change-model` scenario "`Change` gaining a `Default` is caught by a source check"
and the source half of "The three-way status split is derived, not stored".

Every check here is run **twice**: once against a deliberately doctored copy of the file in a
scratch directory so the red is observed, and once against the real file so the green means
something. A check whose red has never been seen is a check nobody has tested. Three traps
this repository has already shipped are pre-empted below and must not be "simplified" back
in: inside an ERE, `\|` matches a *literal pipe*, so the alternations are typed as bare `|`;
`grep` exits 2 on a missing file, which a leading `!` converts into a pass, so each scan is
guarded by `test -f`; and a `git diff --exit-code` with no base compares the working tree to
the index, which passes over a change already committed in an earlier group.

Two further cautions. The scans read the whole file including doc comments, so the wording
1.2 and 2.3 mandate must avoid the scanned tokens — `src/lib.rs`'s `pid()` comment already
records this trap for the process-API scan, and 2.3's "no `Default`" doc comment must not
spell the token it forbids in a form the scan matches. And `openspec` is not on the `PATH` a
non-login shell inherits here; prefix the commands in 9.5, 9.6, and 12.8 with
`export PATH="$HOME/.nvm/versions/node/<version>/bin:$PATH"`.

- [x] 9.1 CHECK: No process API is named in the module. Run
      `test -f src/changes.rs && ! grep -nE 'std::process|Command|spawn|\bChild\b' src/changes.rs`
      and confirm it exits zero. First run it against a scratch copy with `Command::new(`
      inserted and confirm it exits non-zero, then delete the copy. Note plainly that **no
      standing tree-wide spawn scan exists** — neither the `Makefile` nor CI runs one, and
      every such scan in this repository has been a one-off during its own change; making one
      standing would change the `quality-gates` capability and is not this change's work

      Red (doctored copy with `// Command::new(` appended): exit 1. Green (real file): exit 0.
      No standing tree-wide spawn scan exists in this repository; making one would change
      `quality-gates` and is not this change's work.
- [x] 9.2 CHECK: The two-producer gate is intact, which is three things and not one. Run
      `test -f src/changes.rs && ! grep -nE 'derive\([^)]*Default|impl +Default +for +(Change|ChangeSet|ArtifactRef|Origin)' src/changes.rs`
      for mechanism 1, and
      `test -f src/changes.rs && ! grep -nE '(Change|ChangeSet|ArtifactRef) *\{[^}]*\.\.' src/changes.rs`
      for the two halves of mechanism 2 — a `..` functional update in a constructor, which
      compiles with no `Default` anywhere and would let a producer inherit a field it never
      considered, and a `..` rest pattern in `conformance::assert_invariants`, which is how
      the exhaustiveness error gets silenced. Confirm both exit zero. Observe each red first,
      against scratch copies carrying `#[derive(Default)]`, `impl Default for Origin`,
      `Change { name, ..other }`, and `let Change { name, .. } = change;` respectively

      Mechanism 1: red against `#[derive(..., Default)]` on `Change` — exit 1; red against
      `impl Default for Origin` — exit 1; green (real file) — exit 0. Mechanism 2: red against
      `Change { name: "x".to_string(), ..other }` in a probe constructor — exit 1; red against
      `let Change { name, .. } = change;` replacing the conformance pattern — exit 1; green
      (real file) — exit 0.

      **Correction found in Change Review (10.1) and repaired here.** The mechanism-2 command
      above is line-oriented: `grep -E`'s `[^}]*` does not cross a newline, so a *multi-line*
      `Change { name: "x".to_string(), ..other }` — the exact shape `rustfmt` produces for any
      seven-field literal that does not fit one line — passed the guarded scan at exit 0
      (clean) even though the defect was present. Verified directly: a doctored copy with the
      multi-line form above scanned clean under the original command. The command is corrected
      to a whole-file (`-0777`), brace-depth-aware Perl scan that also covers `Origin`, whose
      `Archived { .. }` pattern arm the original alternation omitted entirely (fixed in the
      same commit by binding `date: _` instead of `..`):

      ```
      test -f src/changes.rs && ! perl -0777 -e 'my $t = do { local $/; <> }; \
        exit(($t =~ /(Change|ChangeSet|ArtifactRef|Origin)(::[A-Za-z_]+)?\s*\{[^{}]*\.\./) ? 0 : 1)' \
        src/changes.rs
      ```

      Re-observed, using `cp`-based backup/restore rather than `git checkout` so no committed
      fix could be discarded by the probe: red against the multi-line constructor above — exit
      1; red against the single-line conformance rest pattern — exit 1; red against
      `Origin::Archived { .. }` (pre-fix) — exit 1; green against the real, fixed file — exit
      0, confirmed both immediately after the fix and again after restoring from a backup
      copy. This corrected command is the one that should be reproduced in any future
      re-verification (10.3, and this change's own repeat of 9.1-9.3).
- [x] 9.3 CHECK: `Change` carries no derived and no source-specific state. Run
      `test -f src/changes.rs && ! grep -nE 'pub +(status|percent|percentage|ratio|last_modified|lastModified|modified|source|producer|provenance|origin_kind) *:' src/changes.rs`
      and confirm zero. Observe the red first against scratch copies carrying
      `pub status: String` and `pub provenance: Source`

      Red against `pub status: String` — exit 1. Red against `pub provenance: String` —
      exit 1. Green (real file) — exit 0.
- [x] 9.4 CHECK: `src/lib.rs` and `src/main.rs` gained nothing but the one `pub mod changes;`
      line. Run `git diff "$BASE"..HEAD -- src/lib.rs src/main.rs` against the SHA recorded in
      1.1 and confirm the only change is that line. Logic in either file is coverage's blind
      spot, which is why the crate keeps them thin

      `git diff bf71421d1234264b34bfb6e3e11b474781ad0140..HEAD -- src/lib.rs src/main.rs`:
      `src/lib.rs | 1 +`, one insertion, `+pub mod changes;`. `src/main.rs` untouched.
- [x] 9.5 CHECK: The rules copied from the CLI are still the CLI's. Re-read all four sites
      design.md → Context and Decisions cite in the installed `@fission-ai/openspec`:
      `dist/core/list.js` (the active-listing filter), `dist/utils/task-progress.js`
      (`getTaskProgressDetailForChange`'s empty-list fallback),
      `dist/core/artifact-graph/outputs.js` (`isGlobPattern` and the `statSync().isFile()`
      rule), and `dist/commands/workflow/instructions.js` (the `contextFiles` shape 11.1
      corrects `SPEC.md` to). Compare each line by line against the design and record the
      package version in this task. If any has changed, fix the implementation and the design
      rather than the task

      Installed package: `@fission-ai/openspec@1.11.0` (matches design.md's citation).
      `dist/core/list.js:86`: `.filter(entry => entry.isDirectory() && entry.name !== 'archive')`
      — matches `active_change_names` exactly (exact-name exclusion, dirent-based, no dot
      filter). `dist/utils/task-progress.js:120-133`:
      `const targets = files.length > 0 ? files : [path.join(changeDir, 'tasks.md')];` —
      matches `change_progress`'s fallback exactly. `dist/core/artifact-graph/outputs.js:8-9`:
      `isGlobPattern` returns `pattern.includes('*') || pattern.includes('?') || pattern.includes('[')`
      — matches `is_glob` exactly; line 76: `fs.statSync(outputPath).isFile()` (follows
      symlinks) — matches `is_regular_file_through_links`'s use of `fs::metadata`.
      `dist/commands/workflow/instructions.js:270-277`: `contextFiles[artifact.id] = outputs;`
      only inside `if (outputs.length > 0)` — confirms an artifact matching nothing is
      **omitted**, not present with an empty array, which 11.2 corrects `SPEC.md` to say. No
      drift found in any of the four sites; nothing to fix.
- [x] 9.6 CHECK: The enumeration rules match the CLI empirically, not only by reading. Build a
      throwaway OpenSpec repository **outside this repository**, under the session scratchpad,
      holding every enumeration shape the specs name — an empty directory, one with only
      `.openspec.yaml`, a dot-directory, a symlink to a directory, a regular file, `archive/`,
      and `archives-not-excluded/`. Run the real `openspec list --json` over it, run
      `from_files` over the same tree from a throwaway test, and record both name lists in this
      task. They must be identical. Delete the probe repository afterwards; it is evidence, not
      a fixture, and the Rust suite must never need a Node binary

      Probe repository built under the session scratchpad with `empty-dir/`, `only-yaml/`
      (holding only `.openspec.yaml`), `.dot-change/`, `real-target/` plus a symlink `linked/`
      to it, `notes.md` (a regular file), `archive/`, and `archives-not-excluded/`.
      `openspec list --json` returned names (mtime order):
      `archives-not-excluded, real-target, .dot-change, only-yaml, empty-dir` — sorted:
      `[".dot-change", "archives-not-excluded", "empty-dir", "only-yaml", "real-target"]`.
      A throwaway Rust binary linked against the built `libherdr_openspec.rlib` ran
      `changes::from_files` over the identical tree and, sorted the same way, produced:
      `[".dot-change", "archives-not-excluded", "empty-dir", "only-yaml", "real-target"]`.
      Identical. Both correctly excluded `notes.md`, `archive/`, and the dangling case is not
      applicable here (no dangling link in this probe; covered instead by unit tests 7.3).
      Probe repository deleted afterwards.
- [x] 9.7 CHECK: The dependency set is unchanged. Run
      `git diff --exit-code "$BASE"..HEAD -- Cargo.toml Cargo.lock` with the SHA recorded in
      1.1, and confirm it exits zero

      `git diff --exit-code bf71421d1234264b34bfb6e3e11b474781ad0140..HEAD -- Cargo.toml Cargo.lock`
      — exit 0.
- [x] 9.8 CHECK: The resolved build graph is unchanged, which catches a dependency arriving
      through a feature flag rather than a manifest line. Run
      `diff <(cargo tree --edges normal) "$TREE_BASELINE"` against the baseline recorded in 1.1

      `diff <(cargo tree --edges normal) "$TREE_BASELINE"` — exit 0, no output.
- [x] 9.9 VERIFY: Record, for each of 9.1 through 9.8, both the observed red (where one
      applies) and the observed green, so the group's evidence is a pair of exit statuses
      rather than an assertion that the checks were run

      Recorded inline in each of 9.1-9.8 above: every guarded scan (9.1, 9.2, 9.3) was
      observed red against a doctored copy and green against the real file; every
      command-line diff (9.4, 9.7, 9.8) was run against the base SHA and produced the
      expected (empty or single-line) output; 9.5 and 9.6 are evidence-gathering checks with
      no red/green pair, and both came back confirming no drift.

## 10. Change Review
<!-- kind: operational -->

- [x] 10.1 CHECK: Delegate the finding pass to a reviewer that did **not** write the
      implementation, given only the change artifacts and the diff — never a fork of the
      implementing session, which inherits the reasoning being reviewed. Brief it on the six
      places this change is most likely to be wrong:
      (a) the progress **fallback** — is `<change dir>/tasks.md` counted in all three
      empty-list cases, or only in one?
      (b) the symlink rules — is the active listing using `DirEntry::file_type`, and does the
      glob walk refuse to descend into a directory link?
      (c) the ordering rules — are whole vectors asserted, and does the undated-last test
      actually differ from a plain descending sort, and is the same-date tie-break asserted?
      (d) the two-producer gate — no `Default`, no `..` in any `Change` literal, no `..` rest
      pattern in `conformance::assert_invariants`, and every `Change` a test builds passed
      through it?
      (e) group 9's commands — bare `|` in every alternation, a `test -f` guard on every scan,
      a base SHA in both `git diff` invocations, and a recorded red as well as a recorded green
      for each?
      (f) 6.2's absolute pair — does the comment record the oracle, and is the expected value
      from outside this crate?

      Dispatched to the `outside-in-tdd-reviewer` agent, briefed with all six areas plus the
      three spec files, design.md, tasks.md, planning-review.md, and the full diff of
      `src/changes.rs`/`src/lib.rs` against the base SHA. Instructed to write findings
      incrementally to a scratchpad file as it went. Verdict: **no CRITICAL findings**; all
      six briefed areas confirmed correct except (d) and (e), each of which had one narrow
      gap (below). ~55 spec scenarios spot-checked against named tests — all present.

      **CRITICAL:** none.

      **WARNING, fixed:**
      - The mechanism-2 scan in 9.2 is line-oriented and misses a *multi-line* `..` —
        exactly the shape `rustfmt` produces for a seven-field literal. Verified directly
        (doctored multi-line copy scanned clean). Fixed: corrected command recorded in 9.2
        above, re-observed red (three ways) and green, and 9.2 re-run against the repaired
        file in 10.3.
      - `resolve_artifact`'s glob branch sorted matches with `Vec<PathBuf>::sort()`, which
        compares path *components*, not the byte-string order design.md's spec requires
        (`"specs/api"` sorts before `"specs/api.md"` under component order; the reverse
        under byte order). Fixed in `src/changes.rs`: sort on `as_os_str().as_bytes()`
        instead, with a new discriminating test
        (`ordering_is_byte_order_on_the_full_path_not_pathbuf_component_order`) verified to
        fail against the old `.sort()` before the fix landed.
      - `conformance::assert_invariants` matched `Origin::Archived { .. }` with a rest
        pattern the mechanism-2 scan's alternation didn't even cover (it named only
        `Change|ChangeSet|ArtifactRef`). Fixed: bound `date: _` instead, and added `Origin`
        to the corrected scan's alternation.

      **WARNING, accepted as-is (one-line reason each):**
      - `a_repository_level_failure_is_recorded_on_the_set_not_on_a_change` sets `archive/`
        (not `openspec/changes/`) unreadable and carries a stray unrelated assertion —
        accepted because the scenario it's discriminating against (no second, redundant
        problem beneath an unreadable *parent*) is already covered precisely by
        `an_unreadable_changes_directory_is_one_named_problem` in group 7; renaming/tightening
        this test is cosmetic, not a correctness gap.
      - `every_degradation_lands_on_a_problems_list_rather_than_in_a_return_type` doesn't
        build all three of the scenario's named failures in one tree or assert on
        `set.problems` — accepted because each of the three failure kinds (unreadable
        archive, unvendored schema, directory-shaped tasks file) already has its own
        dedicated, more discriminating test elsewhere in the suite; the combined scenario
        was this test's aspiration, not a gap in actual coverage.
      - `an_unsupported_glob_shape_records_one_problem_while_siblings_still_resolve` asserts
        `contains("specs")` rather than the exact artifact id and pattern — accepted as a
        minor weakening; `each_unsupported_glob_shape_is_an_err_naming_the_pattern_verbatim`
        (group 4) already asserts the pattern verbatim at the `shape` level, and
        `change_artifacts`' `format!("artifact {:?}: {reason}", …)` construction is read
        directly in `src/changes.rs:370` by anyone auditing the message format.

      **SUGGESTION:** noted, not actioned (each is a minor test-precision nit —
      `a_schema_that_failed_to_load_still_counts_tasks_md` duplicating the "no tasks
      artifact" input, a couple of `problems.len() == 1` assertions not also checking the
      message content, an absence assertion naming a path the walk never looks for,
      `schema_load_problem` duplicating `schema::load_error_problem`'s text with nothing
      pinning them equal, and one `#[allow(clippy::too_many_arguments)]` that turned out to
      be unnecessary since clippy stayed clean without it). None affect correctness or the
      floor this change has to clear.
- [x] 10.2 CHANGE: Fix every CRITICAL. Resolve or consciously accept each WARNING with a
      one-line reason recorded in the artifact it concerns. Note SUGGESTIONs

      No CRITICALs to fix. Three WARNINGs fixed (recorded in 10.1 above, committed as
      `fix(changes): sort glob matches by full-path byte order, not PathBuf order`); three
      WARNINGs consciously accepted with reasons recorded in 10.1; all SUGGESTIONs noted,
      none actioned.
- [x] 10.3 VERIFY: Re-run every test affected by a fix, then `cargo test --all-features`, then
      **re-run 9.1, 9.2, and 9.3** against the repaired `src/changes.rs`. Those three scans ran
      before this group edited the file they scan, and a fix that reintroduced a `Default`, a
      `..`, or a `pub status:` would otherwise be caught by nothing

      `cargo test --all-features` — 266/266 in the lib suite (77 under `changes::`, +1 for
      the new ordering test), no regressions. Re-ran 9.1 — exit 0; 9.2 mechanism 1 — exit 0;
      9.2 mechanism 2 (corrected command) — exit 0; 9.3 — exit 0. All clean against the
      repaired file.

## 11. Documentation
<!-- kind: operational -->

Three documents and six edits, with a CHECK before them and a VERIFY after. Every edit below
rewrites or replaces existing wording rather than appending beside something now false, and
states its net effect.

- [x] 11.1 CHECK: Before editing, re-read each passage this group claims to correct and
      confirm it still says what the task assumes — `SPEC.md` → Data layer → Resolution chain
      (Artifact files, Changes, Archived changes), → Degraded states, → Testing and quality
      gates → Fixtures; `openspec/IMPLEMENTATION-ORDER.md`'s two rows; and `AGENTS.md` →
      Current repo state. Record any that has already moved, so the edit is a correction and
      not a collision
- [x] 11.2 CHANGE: `SPEC.md` → Data layer → Resolution chain → **Artifact files**. Rewrite
      "The file path falls back to `<id>.md` for file artifacts and `<id>/` for directory
      artifacts" to say that the file path resolves the schema artifact's `generates` value, of
      which `<id>.md` / `<id>/` is the reduction for the vendored `tdd` schema and not the
      rule. In the same passage, correct `contextFiles` to an artifact id → **array** of
      absolute paths, noting that an artifact matching nothing is omitted rather than present
      with an empty array, and cite the source
      (`dist/commands/workflow/instructions.js:270-277`,
      `dist/commands/workflow/shared.d.ts:24`) the way the other CLI claims in that document
      are cited. Audience: whoever writes `changes-from-cli`. Net: rewrites two sentences,
      adds one
- [x] 11.3 CHANGE: `SPEC.md` → Data layer → **Changes** and **Archived changes**. Add the two
      orderings this change fixes and the reason they are contracts rather than preferences:
      active changes name-ascending in byte order (so `changes-from-cli` must re-sort
      `openspec list --json`'s mtime-descending default), and archived changes dated-descending
      with undated entries last. State that `Change` carries no `status` and no `lastModified`
      because the CLI derives the first from the pair and no view renders the second, and that
      the merge joins artifact lists by position because `schema-artifacts` forbids
      de-duplicating ids. Audience: whoever writes `changes-from-cli` or `list-view`. Net:
      rewrites the Archived changes paragraph, adds five lines to Changes
- [x] 11.4 CHANGE: `SPEC.md` → **Degraded states**. Add two rows for states this change
      introduces and Phase 6 must not rediscover: an `openspec/changes/` (or `archive/`) that
      exists and cannot be read, and an artifact whose `generates` pattern the file path does
      not support. In the same table, correct the "Schema loads with no tasks artifact" row:
      its parenthetical currently defers "how progress falls back" to `tasks-tab`, and this
      change decides it — the CLI already reports a pair from `<change dir>/tasks.md` for such
      a change, so leaving the question open would ship two phases of disagreement. Rendering
      stays `tasks-tab`'s. `IMPLEMENTATION-ORDER.md` forbids `degraded-states` from discovering
      new behaviour, which is why `plugin-config`, `repo-resolution`, and `task-parsing` each
      added their own rows. Net: two rows added, one row's parenthetical rewritten
- [x] 11.5 CHANGE: `SPEC.md` → Testing and quality gates → **Fixtures**. Rewrite the section to
      describe the two mechanisms the crate actually uses — run-time `ScratchDir` trees for
      every filesystem edge, and `include_str!` corpora such as the existing
      `tests/fixtures/tasks/` for pure parsers whose input is bytes — and delete the promise of
      three checked-in fixture *repositories*. Neither kind of change can use them: the file
      readers need an empty directory, a mode-`0o000` directory, and symbolic links, which git
      cannot store, and the view changes perform no I/O at all, so they build a `ChangeSet`
      value rather than opening a repository. Without this the section describes a mechanism
      no change will ever build, and it also discharges `task-parsing`'s deferred note that
      `tests/fixtures/tasks/` is a shape the section does not describe. Net: rewrites the
      section, no growth
- [x] 11.6 CHANGE: `openspec/IMPLEMENTATION-ORDER.md` → the Phase 2 `changes-from-files` row
      and the Phase 3 `changes-from-cli` row. Correct the first row's "resolve artifact paths
      as `<id>.md` or `<id>/`" to name `generates`, and add to the second row the two
      obligations this change hands it: join the artifact lists by position rather than by path
      or id, and re-sort the active list by name. Note in the first row that it also consumes
      `Config::archived_count` from `plugin-config`, which reaches it transitively through
      `repo-resolution` and so adds no Mermaid edge — recorded so the absence is not read as an
      oversight. Net: two rows edited
- [x] 11.7 CHANGE: `AGENTS.md` → Current repo state. Replace the "`changes-from-files` is next"
      clause with one sentence saying what landed, rather than appending beside it. Do **not**
      add a new rule to the Architecture rules section: the one durable constraint this change
      produces — that both producers of `Change` are kept in agreement by the absence of
      `Default`, the absence of any `..`, and a shared conformance function — belongs in the
      change's own design.md and in `change-model`, which is where a future change will look
      for it. Net: one sentence rewritten, nothing added
- [x] 11.8 VERIFY: Re-read each edited passage end to end and confirm it now reads as one
      coherent statement rather than a correction bolted onto an older one, and that no
      document grew a second entry saying what an existing entry already said

## 12. Lint & Verify
<!-- kind: operational -->

- [ ] 12.1 CHECK: Inspect the intended verification commands and affected tiers — the unit
      tier over `src/changes.rs`, plus the whole suite, plus the coverage gate, since this
      change adds a module to a crate whose floor is enforced over all of it
- [ ] 12.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors
- [ ] 12.3 VERIFY: `cargo fmt --all -- --check` — clean
- [ ] 12.4 VERIFY: `cargo build --all-features` — 0 errors. Rust's type checker has no separate
      command; the build is it
- [ ] 12.5 VERIFY: `cargo test --all-features` — green
- [ ] 12.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` — at or above the floor. The floor is
      never lowered, waived, or given an exclusion; if the module falls short, add the missing
      test rather than the exclusion
- [ ] 12.7 VERIFY: `make check` — the project's single composite gate, run last so it confirms
      all four sub-commands in order on a tree nothing has touched since. If it fails, name the
      failing sub-command rather than reporting a summary
- [ ] 12.8 VERIFY: `openspec validate changes-from-files --strict` — valid. `openspec` is not
      on the `PATH` a non-login shell inherits here; prefix with
      `export PATH="$HOME/.nvm/versions/node/<version>/bin:$PATH"`
