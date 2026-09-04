## Reviewed Artifacts

- `proposal.md`
- `specs/repo-discovery/spec.md`
- `specs/openspec-binary/spec.md`
- `design.md`
- `tasks.md`

The finding pass was delegated to three independent reviewers, none of which wrote the
planning package and none of which was a fork of the planning session. Each received the
change directory, the repository's durable documents, the archived `plugin-config` change
as the quality bar, and one slice of the review list: capability coverage and scenario
quality; design completeness, test boundaries, and test strategy — with an explicit
instruction to *run* every shell command and try to make each one fail; and task
alignment, contradictions, and repository invariants. Each was given a scratchpad path and
told to append findings as it discovered them rather than only in its final message — the
mitigation `HANDOFF.md` asks for after two review rounds were lost to `529 Overloaded`.
All three reports survived and are the source of the table below. The reviewers edited
nothing; every repair was made in this session, in the artifact that owns it.

They returned **1 CRITICAL, 18 WARNING, 19 SUGGESTION** across 38 findings. The CRITICAL
and every WARNING is repaired below. Eighteen of the nineteen SUGGESTIONs are applied;
none was rejected on judgement, and the deferrals recorded at the end are pre-existing
design decisions the reviewers confirmed rather than new work.

One incidental note on process: a teammate agent cannot pass `name` when spawning
(`Teammates cannot spawn other teammates`), so the three reviewers were dispatched
unnamed. `HANDOFF.md` already records this.

## Reviewed Against

- This repository HEAD: `6b7987d32a3f9cb905424302d10c5ce25af800d0`
  (`docs(repo-resolution): add TDD task list`) — the proposal, both delta specs,
  design.md, and tasks.md were committed at that point, before the reviewers were
  dispatched, so all three read a fixed tree.
- Sibling repository HEAD: Not applicable. `herdr`, `openspec`, `npm`, and `node` are
  consumed as installed binaries, not as source siblings.
- Working tree: clean apart from this change's own planning directory. No source file is
  touched by this change yet; `src/resolve.rs` does not exist.
- Repairs were committed as `6aade98ad6eaa8cac7ae86acfebfa9e4a512d031`.

Environment facts confirmed during planning and re-confirmed independently during review,
on this machine (macOS, Herdr 0.8.2, Rust 1.91.1, openspec 1.11.0):

- `openspec` resolves to `~/.nvm/versions/node/v24.20.0/bin/openspec`, a **symbolic link**
  (`lrwxr-xr-x`) to `../lib/node_modules/@fission-ai/openspec/bin/openspec.js`, whose
  target is `-rwxr-xr-x`, 79 bytes. This is why the usability check must use
  `fs::metadata` and not `symlink_metadata`.
- All three installed node versions — `v24.18.0`, `v24.19.0`, `v24.20.0` — carry an
  `openspec` binary, so step 3's version ordering is live behaviour here.
- `npm prefix -g` prints `/Users/juusopiikkila/.nvm/versions/node/v24.20.0` — the same
  tree step 3 searches — and writes unrelated zsh-plugin noise to stderr
  (`npm:9: command not found: _omz_nvm_setup_completion`).
- `make check` at HEAD: green, **97.65% of 895 lines**, exit 0. The floor is 80% and is
  not at risk.
- `grep -rn '"openspec"' src/` **matches at HEAD** (`src/state.rs:653`), confirming that
  `plugin-config`'s program-name-literal check cannot be reused here.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs/repo-discovery | The requirement said a starting path "that names a regular file rather than a directory, SHALL produce that same not-found result", while its own scenario "A starting path naming a regular file is walked from its parent" said the same input yields `Found`. design.md's matrix row and the task both implement the scenario. Two clauses of one requirement contradicted each other on one input, and an implementer following the SHALL would have written a special case that turns a passing scenario red. | The SHALL is the wrong one and is rewritten: a regular-file start path is walked *from* that path, its parent is the first ancestor examined, and no special case is needed — a file has no `openspec` child, so the walk simply moves up. | specs/repo-discovery, requirement prose |
| WARNING | specs/openspec-binary, design.md, tasks.md | **Nothing pinned step 3 before step 4.** No fixture populated the nvm tree and the npm-prefix hook together, so a chain probing npm-prefix first was green across all forty scenarios. This is the one adjacency the whole `BinSource` design exists to protect, and the one the reference machine cannot see: `npm prefix -g` prints the nvm version directory there, so both steps resolve to the same path. Two reviewers found it independently. | New scenario "The nvm tree outranks the npm prefix", with a populated nvm tree and a hook returning a *different* directory, asserting the nvm path and `BinSource::Nvm`. Matrix row and task 5.2 say why it exists. | specs/openspec-binary; design.md matrix; tasks.md 5.1, 5.2 |
| WARNING | specs/openspec-binary, tasks.md | **"An empty `PATH` entry is not the current directory" was green against the implementation it exists to reject.** Verified by a reviewer with a scratch Rust program: `Path::new("").join("openspec")` is the relative `openspec`, which under `cargo test` resolves against the crate root — a repository whose `openspec` child is a **directory** and is rejected by `is_file()` for an unrelated reason. A cwd-honouring implementation returns the same resolved path. | Step 2's candidate list is now a contract of its own: `path_candidates(&str) -> Vec<PathBuf>`, pure, no filesystem access, asserted directly on the input `":D:   :"` to be exactly `[D/openspec]`. The resolution assertion stays as the second half. design.md → Decisions records this as the general remedy for a rule whose effect is masked downstream. | specs/openspec-binary; design.md Contracts, Decisions, matrix; tasks.md 3.4, 3.8 |
| WARNING | specs/repo-discovery, tasks.md | **A relative starting path that cannot be canonicalized walks into the process's own working directory.** `Path::ancestors` ends a relative chain with the **empty** path (verified: `["nope/deeper", "nope", ""]`), and joining `openspec` to it produces a bare relative path. Under `cargo test` the working directory is the crate root, which *is* a repository — so `find_repo(Path::new("nope/deeper"))` would have returned `Found` with an empty root. A real bug, found before a line was written. | The requirement now states that the unresolved walk stops at the last non-empty ancestor and why; a new scenario asserts `NotFound` for exactly that input, naming the naive implementation it rejects; task 2.6 carries the rule. | specs/repo-discovery; design.md matrix; tasks.md 2.1, 2.2, 2.6 |
| WARNING | design.md, tasks.md, src/lib.rs | **Both "reads and never writes" guards were blind to the write they name.** `testutil::snapshot`'s `collect` (`src/lib.rs`) recurses into a directory but pushes an entry only for a non-directory, so an empty directory contributes nothing to a `Snapshot` — and `create_dir_all` is precisely the accidental write the tasks say the guards catch. The discovery fixture even included an empty `openspec/specs/`. design.md compounded it by promising `snapshot` was reused **unchanged**. | `snapshot` gains directory entries (path, empty bytes, own mtime) as task 1.4. Both scenarios and both tasks now say "including directories" and state that a file-only snapshot cannot see the defect. Because `config` and `state`'s existing assertions are equality comparisons, entries added to both sides leave them green — 1.4 re-runs them to prove it. The Boundaries table records the extension instead of claiming no change. | specs/repo-discovery; specs/openspec-binary; design.md Boundaries and matrix; tasks.md 1.4, 2.4, 5.5 |
| WARNING | tasks.md | **Groups 3 and 4 could not assert `BinSource`, but three of their scenarios required it.** The types and `openspec_bin` landed in group 5, while 3.1 and 4.1 scheduled scenarios whose THEN reads "and its source is the `PATH`/nvm step". An implementer at 3.1 would have had to invent a resolution type or silently drop the assertion. | The chain is grown group by group: group 3 introduces `BinSource`, `FoundBin`, `BinResolution`, and `openspec_bin(configured, env)` with steps 1 and 2; group 4 wires in step 3; group 5 widens the signature with the hook and adds step 4. The three configured-fall-through scenarios move to group 3, where steps 1 and 2 exist and they are genuinely testable. Group 3's preamble states the reasoning. The `npm_prefix` parameter is explicitly not added early with an ignored binding, which clippy would flag and which reads as a bug. | tasks.md groups 3, 4, 5 (renamed and renumbered) |
| WARNING | design.md | **`NOTOOLS` silently dropped the last `PATH` entry.** `printf '%s'` leaves the final `tr` line unterminated, so `while read` never processes it. Verified: `PATH` ended `.../emulo/0.6.0/bin` and `NOTOOLS` ended one entry earlier. Harmless on this machine, fatal if `~/.cargo/bin` were last — which is exactly where rustup's `.cargo/env` append puts it, and exactly the "the check fails for the wrong reason" trap the block's own comment warns against. | `printf '%s\n' "$PATH"`. Re-verified after the fix: last entry preserved. | design.md → Test Strategy; tasks.md 7.3 |
| WARNING | design.md | **`NOSPAWN-RUN`'s preconditions had no teeth.** The design asserted "without them the run proves nothing"; the reviewer ran the block against a `NOTOOLS` that still resolved `npm` and `node` and it **printed both paths and exited 0**. Without `set -e` a bare `! command -v` sequence is decoration. | Rewritten as explicit `if … then echo PRECONDITION FAILED … exit 1` loops over all five preconditions. Re-verified both ways: the good `NOTOOLS` passes, and a negative control with `npm` resolvable exits 1. | design.md → Test Strategy; tasks.md 7.3 |
| WARNING | design.md, tasks.md | The unparseable-version ordering rule ("after every parsed version, then by name descending") had no scenario and no discriminating fixture — the only unparsed fixture, `system`, was alone in its tree, so sorting it first passed. | The scenario is extended to a second tree holding a parsed version and an unparsed name together. **This session then found the reviewer's suggested fixture non-discriminating and replaced it**: plain name-descending sorts `vnightly` > `v9.99.99` > `v20.0.0` > `system` (computed, not assumed), so a `system` fixture passes against the wrong ordering and only a name sorting *above* the versions fails it. `vnightly` is the fixture. | specs/openspec-binary; design.md matrix; tasks.md 4.3 |
| WARNING | specs/openspec-binary | Nothing forbade recording a problem for a configured path that **works**. An implementation pushing a problem whenever `openspec_bin` is set passed every scenario, and would render a spurious fallback message in the header for a correctly configured user. | "The configured path wins over every other source" now asserts `problems.is_empty()`. Task 5.3 states the pairing explicitly: with 3.5's "configured fails and nothing else is found", the two directions of the `problems` contract are both pinned, and neither test alone is sufficient. | specs/openspec-binary; design.md matrix; tasks.md 5.3 |
| WARNING | specs/openspec-binary, tasks.md | The whitespace-only `PATH` rule was in the spec, absent from the tasks (3.7 skipped only *empty* entries), and unfalsifiable in its scenario. | One rule everywhere: an entry that is empty **or whitespace-only** is skipped, reusing the blank-value convention `config::non_blank` already establishes across this crate, and asserted through the new candidate list rather than through a resolved path. | specs/openspec-binary; tasks.md 3.4, 3.8 |
| WARNING | specs/openspec-binary, tasks.md | **The no-spawn scenario froze a tree-wide `src/` property that `subprocess-seam` is required to falsify** — "no file under `src/` names `process::Command`, `Command::new`, or `Stdio`" becomes a false archived requirement the moment `src/cli.rs` exists. This is the same defect `plugin-config`'s review repaired in its own version of this scenario, reintroduced from the other direction. | The normative clause is narrowed to `src/resolve.rs`, with the reason stated in the spec itself. The tree-wide grep is still run, as a **task-level** check that `subprocess-seam` will rescope; design.md and tasks.md 7.2 both say so, so it is not later mistaken for a requirement. | specs/openspec-binary; design.md → Test Strategy `SPAWN` block; tasks.md 7.2 |
| WARNING | design.md → Test Boundaries | The table contradicted itself on the `openspec` binary: "**not invoked**" in the acceptance column, while another row named `openspec validate --strict` as the final gate and task 10.7 runs it. A related hazard was unstated — 7.3's `NOTOOLS` deliberately makes `openspec` unresolvable while 10.7 needs it on `PATH`. | The row now separates the two: the **crate** never invokes it; the **workflow** invokes it once, read-only, and the two tasks therefore run on different `PATH`s. Task 7.3 says the same. | design.md → Test Boundaries; tasks.md 7.3 |
| WARNING | tasks.md 9.7, openspec/IMPLEMENTATION-ORDER.md | 9.7 instructed the implementer to "confirm the dependency graph needs no edge change" while the same task hands `subprocess-seam` the obligation to replace `resolve::npm_prefix_deferred` — which makes `subprocess-seam` depend on this change. Only the (correct) reverse direction was checked. | 9.7 now requires adding `repo-resolution --> subprocess-seam` to the Mermaid graph and to the row's `Depends on` column, noting that nothing reorders because the two are already in Phases 2 and 3. Named in proposal.md → Impact. | tasks.md 9.7; proposal.md |
| WARNING | tasks.md 9.8 | 9.8's *replacement* text for `AGENTS.md` → Current repo state named `changes-from-files` as the next change. It is not: it also depends on `schema-model` and `task-parsing`, neither of which has landed. The task would have written a false statement into the document every session reads first. (9.8's claim about the *current* text was accurate.) | 9.8 now names `schema-model`, and says explicitly to check the Phase 2 table before writing the name rather than assuming roadmap order equals dependency order. | tasks.md 9.8 |
| WARNING | specs/openspec-binary | The nvm step had no case for an absent `HOME` when `NVM_DIR` is also absent. tasks.md 4.5 supplied the answer; no scenario pinned it. The implementation it fails to reject is `env("HOME").unwrap()` — a panic, against a contract design.md states as "nothing returns `Result` and nothing panics". | The requirement gains the clause; the `NVM_DIR` scenario gains a third run with both variables absent, asserting no binary and no panic. Task 4.4 names it as the step's absent-dependency case. | specs/openspec-binary; design.md matrix; tasks.md 4.4 |
| WARNING | SPEC.md, openspec/IMPLEMENTATION-ORDER.md | Both say resolution is "tested against a synthetic filesystem", while design.md → Decisions argues explicitly against introducing a filesystem abstraction (faking symlinks, execute bits, and directory-versus-file would be testing the fake). Neither correction was scheduled, so the phrase would have invited the very thing the design rejects. | Folded into task 9.6: disambiguate the phrase in both documents to mean a purpose-built scratch tree under `std::env::temp_dir()`, not a faked filesystem layer. 9.1's re-read list now includes the Phase 2 `repo-resolution` row. | tasks.md 9.1, 9.6 |
| WARNING | design.md → Test Boundaries | Four collaborator rows were absent or wrong for what the tasks actually do: `git` (read by 1.1, 1.6, 7.4) was described only as "not used as a restore mechanism"; `make` (1.1, 10.6) was missing from the tooling row; `std::env::temp_dir()`'s acceptance cell said "not used" while 7.5 scans it; and `cargo tree` was listed while no task uses it. | All four corrected: a read-only `git` row naming the exact commands, `make` added, the `temp_dir` cell corrected, `cargo tree` dropped with a note that `plugin-config` needed it to pin a new build graph and this change adds no dependency. `mktemp` dropped alongside the unused `T=$(mktemp -d)` shorthand. | design.md → Test Boundaries and Test Strategy |
| SUGGESTION | design.md | `NOTOOLS` ended in a colon from `tr '\n' ':'` — an empty `PATH` entry meaning the current directory, contradicting this change's own requirement. | `sed 's/:$//'`. Verified absent after the fix. | design.md → Test Strategy; tasks.md 7.3 |
| SUGGESTION | design.md | The module-scoped `SPAWN` grep passes when `src/resolve.rs` is missing: grep exits 2 for a missing file and `!` inverts it into success, so a renamed or split module silently stops being checked. | Guarded with `[ -f src/resolve.rs ] &&`, with the reason stated in both the block and task 7.2. | design.md → Test Strategy; tasks.md 7.2 |
| SUGGESTION | design.md | `DEPS` used a bare `assert`, which `python3 -O` or an inherited `PYTHONOPTIMIZE` strips — turning the dependency check into a no-op that exits 0 for any dependency list. Verified: the assert form exits 0 under `PYTHONOPTIMIZE=1` against a crate with a second dependency. | Rewritten to `sys.exit("unexpected normal deps: …")`. Re-verified three ways: exit 0 on this crate, exit 0 under `PYTHONOPTIMIZE=1`, exit 1 with a message against a two-dependency fixture. | design.md → Test Strategy; tasks.md 1.1 |
| SUGGESTION | design.md, tasks.md | The spec scenario asked for `cargo` **and `rustc`** as preconditions; the design's command asserted only `cargo`, and 7.3 said "those four assertions" from a list of three-plus-one. Three artifacts, three counts. | All three now say five preconditions, and `rustc` is asserted. Verified resolvable on `NOTOOLS`. | design.md → Test Strategy and matrix; tasks.md 7.3 |
| SUGGESTION | design.md → verification matrix | `cargo test --all-features resolve::` exits 0 when the filter matches nothing (verified: a nonsense filter reports "0 passed … 5 filtered out", exit 0), so 40 of the matrix's Command cells were vacuously green if the tests ended up named without that path segment. | A note under the matrix: the filtered form is a convenience for running one group, the unfiltered `cargo test --all-features` in 10.4 is the gate, and 2.5 requires checking the filter actually names the new tests. Repeated in 10.1. | design.md → Test Strategy; tasks.md 10.1 |
| SUGGESTION | design.md → Contracts | Only `BinSource` carried derives, while the scenarios compare whole `RepoSearch` and `BinResolution` values with `assert_eq!` and print them on failure. An implementer following the contract literally writes types `assert_eq!` will not accept. | `#[derive(Debug, Clone, PartialEq, Eq)]` on all of them, matching `config::Config`, with a sentence saying the derives are part of what a consumer may rely on. | design.md → Contracts |
| SUGGESTION | design.md → Risks | Step 4's production join (`<prefix>/bin/openspec` plus the usability check) is dead until `subprocess-seam` and is exercised only by fixture closures until then. Unstated, it invites `subprocess-seam` to assume step 4 is "already proven" and ship only a stdout parser. | Two risk bullets, and task 9.7 now requires the roadmap row to state the obligation as end-to-end rather than as a binding swap. | design.md → Risks; tasks.md 9.7 |
| SUGGESTION | tasks.md 9.3, 9.7 | The one hard-won empirical fact — `npm prefix -g` must be read **stdout only, trimmed**, because `npm` writes zsh-plugin noise to stderr here — lived only in an archived design.md and a doc comment. The two forward-facing edits a `subprocess-seam` implementer actually reads carried nothing about it. | 9.3 adds it to `SPEC.md` (including that a non-zero exit or empty output means no prefix) and 9.7 carries it into the roadmap row. | tasks.md 9.3, 9.7 |
| SUGGESTION | specs/openspec-binary | Three scenario clauses asserted things the API cannot observe: "no candidate was constructed from an empty string", "without reading `PATH`", and "the check does **not** search for the program-name literal". | The first becomes the `path_candidates` assertion; the second is reworded to the observable determinism claim with the ordering requirement named as what pins it; the third is removed from the THEN and kept where it already lived, in design.md → Test Strategy. | specs/openspec-binary; design.md |
| SUGGESTION | tasks.md header | The concentration-point audit attributed the nvm absent case to 4.1, which is the RED task list rather than the absent case, and the containment citation predated the snapshot repair. | Attributions corrected to 4.4 and to 2.4/5.5 with the 1.4 dependency named. | tasks.md header comment |
| SUGGESTION | tasks.md group 6 | No persistence gate in the group that introduces a cached read, though `plugin-config` had one for its own stored file. | 6.7 added: state what the cache does and does not require, and confirm no `static` instance was added — the one change that would make the answer different. | tasks.md 6.7 |
| SUGGESTION | tasks.md group 2 | Contract-gate asymmetry: the binary chain got one (5.9), the repository walk none, though `changes-from-files` will join onto its result and `degraded-states` will print it. | 2.7 added, re-reading `SPEC.md` → Resolution chain → "Repository" at implementation time. | tasks.md 2.7 |
| SUGGESTION | tasks.md 9.9 | 9.9 was a CHECK sitting after every CHANGE in an operational group, and it may itself edit a file. | Reclassified as a CHANGE that either makes the `AGENTS.md` edit or records the decision not to, keeping the group's CHECK → CHANGE → VERIFY order intact. | tasks.md 9.9 |
| SUGGESTION | tasks.md 1.1 | 1.1 used `$DEPS` without saying it is a design.md shorthand that must be defined in the shell first. | Stated, along with which form to use and why the bare-`assert` form is not evidence. | tasks.md 1.1 |
| SUGGESTION | design.md → Test Strategy | `T=$(mktemp -d)` was declared with a cleanup instruction and used by no check or task — every fixture is a Rust `ScratchDir`. Dead shorthand invites an implementer to find a use for it. | Deleted, and `mktemp` dropped from the POSIX-userland boundary row. | design.md → Test Strategy and Test Boundaries |
| SUGGESTION | proposal.md → Impact | Impact described the `testutil` additions as an executable-file builder only, before the snapshot defect was known, and did not name the dependency-graph edit. | Both added. | proposal.md → Impact |

**Confirmed clean, with no repair needed** — recorded so a later reader knows these were
checked rather than skipped:

- **Capability coverage.** Both capabilities named in proposal.md have delta spec files at
  the right paths; nothing appears only in design.md or tasks.md; names fit the live
  tree's shape.
- **"Modified Capabilities: None" holds.** One reviewer read all six live specs. The live
  `plugin-config` requirement already forward-delegates probing to `repo-resolution`
  ("`openspec_bin` is expanded and never probed"), so no delta is owed there;
  `plugin-build`'s dependency-set requirement stays true because this change adds no
  dependency, and is re-verified by task 7.4 rather than amended.
- **Scenario form.** Every requirement has at least one scenario; every scenario heading
  is exactly four hashes; SHALL/MUST throughout.
- **Rust and macOS semantics the plan depends on**, compiled and run by a reviewer in a
  scratch program outside the repository: `fs::metadata` follows symlinks while
  `symlink_metadata` does not, a directory carries the execute bit, `canonicalize` on
  `$TMPDIR` returns `/private/var/...` on macOS, `Path::ancestors` terminates,
  `OnceLock::get_or_init` and `Option<PathBuf>::as_deref` behave as the contract assumes.
- **The `SPAWN` tree-wide grep** is clean at HEAD and fires on every planted spawn form,
  including an aliased `use std::process as p; p::Command::new(...)`.
- **`NOSPAWN-RUN`'s suite run discriminates**: a planted `Command::new("npm")` test exits
  0 on the normal `PATH` and **101** on `NOTOOLS`.
- **The verification matrix** is 1:1 with the spec scenarios, verified by script before
  and after the repairs — now 42 rows against 42 scenarios, no duplicates, no orphans in
  either direction, and every scenario named in a task.
- **Group structure**: exactly one valid kind marker per group; behavior groups preserve
  RED → GREEN → REFACTOR; operational groups put their evidence task first; no
  `parallel-after` marker (groups 2–6 share one file, which is shared mutable state).
- **No RED task for plumbing**, and none of the repository's known verification traps is
  reintroduced: `env PATH=… cargo`, `paste -sd:`, a `print()`-terminated dependency
  filter, `cargo build --offline`, the tautological composition test, and
  `! grep -rn '"openspec"' src/` — the last confirmed false at HEAD.
- **No timing-shaped test.** The cache tests count probe invocations with an
  `AtomicUsize` read after both calls return; nothing sleeps, polls, or reads the clock.
- **Repository invariants**: no production write anywhere, let alone inside `openspec/`;
  no process spawn and no early `cli` module; the 80% coverage floor neither lowered nor
  excluded; no task edits the graft-vendored `openspec/schemas/tdd/` or `.claude/agents/`;
  no PRD non-goal crossed; correctly not marked **BREAKING**.
- **Every documentation task's claim about what its target document currently says** was
  checked line by line against `SPEC.md`, `AGENTS.md`, and `openspec/IMPLEMENTATION-ORDER.md`
  and found accurate. Only 9.8's *replacement* text was wrong, and it is repaired above.
- **Roadmap fit**: the plan matches the Phase 2 `repo-resolution` row, correctly depends
  on the archived `plugin-config`, and correctly does not depend on `subprocess-seam`.
- **Coverage**: `cargo llvm-cov --fail-under-lines 80` at HEAD is 97.65% of 895 lines.
  Every function in the Contracts block has at least one scenario behind it, including
  both one-line bindings.
- **The central design decision** — step 4 as an injected closure rather than an early
  `cli` seam — was attacked directly by one reviewer, which found no better alternative
  among the three design.md rejects and confirmed the hand-over is durably recorded in
  three places. Its two residues are now stated rather than implied.

## No Remaining Implementation-Blocking Gaps

None remain. The one CRITICAL and all eighteen WARNINGs are repaired in the artifact that
owns each; eighteen of nineteen SUGGESTIONs are applied and the nineteenth was a
restatement of a decision design.md already carries. Every repaired shell command was
executed after the repair, with a negative control where one exists: the `NOTOOLS`
recipe now preserves the last `PATH` entry and leaves no trailing colon, the precondition
block exits 1 when a tool is still resolvable, and the dependency filter exits 1 with a
message against a two-dependency fixture and is immune to `PYTHONOPTIMIZE`.

One repair was made against a reviewer's own suggestion rather than by adopting it: the
unparseable-version fixture. The reviewer proposed `system` alongside `v20.0.0`; sorting
those two by plain name descending yields `v20.0.0` first, which is the *right* answer, so
the fixture would have passed against the wrong implementation. `vnightly` sorts above
`v20.0.0` and is the fixture that discriminates. The ordering was computed, not assumed.

`openspec validate repo-resolution --strict` reports the change valid.

## Deferred Non-Blocking Notes

Two, both pre-existing decisions the reviewers confirmed rather than new work, and both
with their resolution point already recorded:

1. **An unreadable `PATH` or nvm directory has no scenario.** Creating one portably and
   removing it after a panicking test is more risk than the case is worth. The
   implementation treats every filesystem `Err` as "not a candidate" rather than
   unwrapping, tasks 3.7 and 4.6 require that shape, and design.md → Risks records it as
   a deliberate coverage gap rather than an unnoticed one.
2. **`BinCache` has no concurrency test.** `OnceLock` is std, pinning std's guarantees is
   not this change's work, and nothing here spawns a thread — a barrier-based test would
   be exactly the timing-shaped test this repository forbids. Recorded in design.md →
   Non-Goals and in the Test Boundaries "Threads" row.
