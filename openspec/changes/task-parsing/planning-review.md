## Reviewed Artifacts

- `openspec/changes/task-parsing/proposal.md`
- `openspec/changes/task-parsing/specs/task-checkboxes/spec.md`
- `openspec/changes/task-parsing/specs/task-groups/spec.md`
- `openspec/changes/task-parsing/design.md`
- `openspec/changes/task-parsing/tasks.md`

The finding pass was delegated to four independent reviewers, none of them a fork of the
planning session, each given one slice and told to report findings only and to append them
to a scratchpad file as they went rather than in a final message — `HANDOFF.md` records
two earlier review rounds lost to `529 Overloaded`:

- **A — capability coverage, scenario quality, cross-artifact contradictions.**
- **B — design completeness, test boundaries, and the "can this check fail?" audit.**
- **C — task alignment, TDD lifecycle discipline, concentration points.**
- **D — factual verification.** Not a document reviewer: given nine claims the artifacts
  make about the OpenSpec CLI, Rust, and this machine, and told to verify each by reading
  the installed source and running throwaway programs, never by recall.

D's assignment was worth its own reviewer. The entire change rests on reproducing a rule
that lives in someone else's JavaScript, and every one of the nine claims was checkable in
minutes against the installed package (`@fission-ai/openspec` 1.11.0), `rustc` 1.91.1, and
this filesystem. Eight came back verified as written; the ninth was verified with a
correction that changed a design decision's justification.

## Reviewed Against

- This repository HEAD: `9822c83fc224a446cb022377c9d9e71e6c26c4a5`
- Sibling repositories: `Not applicable` — this crate has no sibling, and the OpenSpec CLI
  is a published npm package rather than a repository whose contract is being co-designed.
  It was nonetheless read as an authority: `@fission-ai/openspec@1.11.0`,
  `dist/utils/task-progress.js`.
- Working tree: clean apart from this change's own directory,
  `openspec/changes/task-parsing/`, which is intentionally included.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md | The no-spawn scan was written `grep -nE 'std::process\|Command\|spawn\|…'`. Inside an ERE, `\|` is a *literal pipe*: the pattern matches nothing in any real source file, grep exits 1, and the leading `!` turns that into an unconditional pass. B verified this empirically against a probe file containing `Command::new(`. | Command rewritten with bare `\|` alternation; the table cell now states why the escaped form appears there and points at tasks.md 6.2 as the runnable copy. tasks.md 6.2 spells out both halves of the trap. | design.md → Test Strategy (matrix row and "Can each of these fail?"); tasks.md 6.2 |
| CRITICAL | design.md, tasks.md | The same scan had no existence guard. `grep` exits 2 on a missing file, and `!` converts that to a pass, so a renamed `src/tasks.rs` would make the check report success. `repo-resolution`'s own tasks.md documents this trap and ships the guard; this plan had dropped it. | `test -f src/tasks.rs &&` prepended in both artifacts, with the reason written into 6.2 so it is not "simplified" away. | design.md → Test Strategy; tasks.md 6.2 |
| CRITICAL | design.md, tasks.md | `git diff --exit-code -- Cargo.toml Cargo.lock` is green by construction here. It compares the working tree to the index, and this project commits after every task group — so a `Cargo.toml` edit made in group 1 is already in `HEAD` by the time group 6 runs, and the check passes over the very change it exists to catch. design.md had defended this row explicitly as one that "deserves the question", which made it worse. | New task 1.1 captures a base SHA and a `cargo tree` baseline before the first commit; 6.1 and 6.3 diff against them. `cargo tree` also gained a baseline, without which it was a printed report rather than a check. | tasks.md 1.1, 6.1, 6.3; design.md → Test Strategy (two matrix rows plus the fail-audit prose) |
| WARNING | design.md | Decision 8 justified the absent-versus-unreadable split with "the CLI makes exactly this distinction", including for invalid UTF-8. D measured it: Node's `readFile(f,'utf-8')` succeeds with replacement characters, so the CLI reports **1/2** for a file this module reports **0/0 plus a problem** for. That is a knowing divergence, not an inherited rule — and it produces exactly the "the number changes by itself" symptom design.md's Context opens with. | The claim is scoped to I/O errors. The divergence is named in three places, with lossy decoding considered and rejected (it assigns a confident count to unreadable bytes) and the dual-source model identified as the thing that resolves it. | design.md → Decisions 8 and Risks; specs/task-groups (the "Reading a task file degrades" requirement); tasks.md 5.5 |
| WARNING | proposal.md, tasks.md | `SPEC.md`'s degraded-states table gains no row for the state this change introduces: a tasks file that exists and cannot be read. Its nearest row covers only the *absent* case, which this change deliberately treats as not-a-problem. `IMPLEMENTATION-ORDER.md` forbids Phase 6's `degraded-states` from discovering new behaviour, and both `plugin-config` and `repo-resolution` added their own rows for that reason. | New documentation task adding the row, including the CLI divergence above so Phase 6 inherits it; proposal Impact updated. | tasks.md 8.4; proposal.md → Impact |
| WARNING | design.md, tasks.md | `Tasks::items()` appeared in the Contracts block with no requirement, no scenario, no task, and no named consumer — an uncovered `pub fn` under an 80% line floor. | Removed from the contract, with the reason recorded so it is not re-added; 3.7's GREEN says explicitly not to implement it. | design.md → Contracts; tasks.md 3.7 |
| WARNING | specs/task-groups, tasks.md | The corpus scenario's whole THEN was `parse(t).progress() == count(t)`, which an implementation counting nothing satisfies. The compensating absolute-count assertion lived only in design.md and tasks.md — and specs are what survive archiving. D found a second hole in the same test: every archived `tasks.md` in this repository is 100% complete, so a hardcoded `checked = true` satisfies the entire corpus. | The absolute-count clause moved into the scenario itself, with the requirement that the expected pair come from an oracle **outside this crate** (new task 4.2 obtains it by running the CLI's own regex in `node`). The headingless fixture is now deliberately mixed and its `completed` asserted. | specs/task-groups (both scenarios in "Grouping never changes what is counted"); design.md → Test Strategy; tasks.md 4.1–4.4 |
| WARNING | tasks.md | Group 3 froze `Tasks`, `Group`, `Item`, and `Heading` — `tasks-tab`'s entire consumed surface — with no contract gate, while `Progress` got one at the moment it was frozen. The complete-surface gate sat two groups later. | New contract gate 3.9, placed before the group's verification task and checking the model against `SPEC.md` → Detail view. | tasks.md 3.9; header comment (now "three fire") |
| WARNING | tasks.md | Three `task-checkboxes` scenarios carry a `parse` clause as well as a `count` clause. Group 1 correctly took only the `count` half, and group 3 never picked up the other — leaving `Item::text` trimming asserted nowhere against hostile input. | Group 1's preamble names the split; new task 3.2 writes the four text-fidelity tests, and group 3's coverage paragraph claims the `parse` halves explicitly. | tasks.md group 1 preamble, 3.2, group 3 preamble |
| WARNING | tasks.md | 4.2, 4.3, and 5.4 wore `RED` labels over work the plan's own prose admitted could not fail, and 5.4 packed "RED then GREEN" into one checkbox placed after the GREEN it was supposed to precede. The schema requires `CHECK` or `CHARACTERIZE` for evidence tasks that cannot honestly produce a red result. | Group 4 reclassified `refactor` with a CHARACTERIZE → CHANGE → REFACTOR → VERIFY lifecycle and a preamble saying why; group 5's containment test moved into the pre-GREEN block as 5.3 `CHARACTERIZE`, with a new 5.4 confirming the failures are the missing function. | tasks.md group 4 (whole group), 5.3–5.5; header comment |
| WARNING | tasks.md | Task 1.8's GREEN justified `char::is_whitespace` as "deliberately, so group 2 has a real RED", and 2.1 instructed the implementer to "fix group 1" if the tests came up green — that is, to regress correct code in order to manufacture a failure. C confirmed the split itself is honest (D verified `char::is_whitespace` genuinely accepts U+00A0 and so genuinely passes group 1), but the framing invited the opposite. | 1.9's GREEN now justifies the predicate on its own terms — it is the standard-library rule and it satisfies every group-1 scenario — and 2.1 says to keep the correct code and record the fact if a test is green on arrival. | tasks.md group 1 preamble, 1.9, 2.1 |
| WARNING | design.md | "It is scoped to `src/tasks.rs` because a tree-wide scan already exists from `repo-resolution`" is false. B checked the `Makefile`, CI, and `tests/`: no standing spawn scan exists anywhere; `repo-resolution`'s was a one-off run during that change. | Corrected in both artifacts, with the honest statement that nothing pins the no-spawn rule between changes and that making one standing would change the `quality-gates` capability. | design.md → Test Strategy; tasks.md group 6 preamble |
| WARNING | design.md | The Test Boundaries table declared the `openspec` binary "not invoked, not probed" while tasks 6.4 and 9.7 do both. | Row rewritten: absent from every *test*, consulted twice by one-off operational checks that are not gates and are not collaborators of any Rust test. A row for `git` and `cargo tree` was added, since group 6 uses both and silence is not an answer. | design.md → Test Boundaries |
| WARNING | design.md | "No function returns `Result`… the same shape `Schema` already has" overstates the precedent: `schema::parse`, `schema::load`, and `state::record` all return `Result`. | Narrowed to the crate's *composed, top-level* readers, with the two counter-examples named and the reason `tasks` takes the total form (no internal composer, no writes). | design.md → Contracts |
| WARNING | design.md | Decision 3 argued `count` avoids per-item allocation, while tasks.md 3.7 proposed extracting a shared line rule — read together they contradict. SPEC.md's own latency numbers ("well under a millisecond") also undercut the performance framing. | Decision 3 rewritten: two entry points over **one** shared rule, the split being what each does with the result; the honest framing is keeping the list-view path allocation-free by construction, not a measured problem. 3.8 now specifies a borrowed return so both goals hold. | design.md → Decisions 3; tasks.md 3.8 |
| WARNING | design.md | `rules.design` asks whether the change alters the `Change` type and how `from_files` and `from_cli` stay in agreement. The answer existed only in proposal.md, while tasks.md 1.10 sends the implementer to design.md to find it. | Added to design.md: `Change` does not exist yet, this change fixes the one field the two producers must agree on, and the agreement is a property of `Progress` staying free of derived state. | design.md → Contracts |
| WARNING | specs/task-checkboxes, specs/task-groups | Two SHALL clauses had no scenario and therefore no task: item text kept verbatim including its numbering prefix, and a heading's closing `##` sequence not stripped. 3.8's refactor touches the first of those paths. | One scenario added to each spec, both wired into the design matrix and into group 3's tasks. | specs/task-checkboxes ("A numbering prefix and inline markup are kept verbatim in the text"); specs/task-groups ("A closing hash sequence is kept, not stripped"); design.md matrix; tasks.md 3.2, 3.3 |
| WARNING | tasks.md | The header comment claimed all nine concentration points were accounted for; three were padded or mis-cited. The empty-document case was counted as an absent *dependency* when it is an absent *input*; the `from_files`/`from_cli` point cited group 4, which proves a different property (the two file-path entry points agreeing with each other); and the coverage point had no task checking that `src/main.rs` and `src/lib.rs` gained no logic. | All three corrected in place and named as corrections; 1.2 forbids adding logic to either file and 9.1 checks the diff. | tasks.md header comment, 1.2, 9.1 |
| SUGGESTION | specs/task-groups | `indent` did not say which whitespace alphabet it counts — observable on a BOM-prefixed file, where the two alphabets differ. | One clause added binding it to `task-checkboxes`' alphabet. | specs/task-groups ("An item carries its checked state, its text, and its indent") |
| SUGGESTION | design.md | `openspec/config.yaml` requires six boundary states to be covered, and nothing recorded which of them bind to a change that adds no view, no CLI call, and no repository walk. | A paragraph naming the one that binds (a missing artifact file), the five that belong to other modules, and two further boundary states this change adds on its own account. | design.md → Test Strategy |
| SUGGESTION | design.md | Decision 12 (`Progress` addition) reads as present-tense necessity, but D confirmed the vendored `tdd` schema's tasks artifact is a single `tasks.md` (`generates: tasks.md`, `apply.tracks: tasks.md`), so the operator is exercised only by its own unit test today. | Marked as forward-looking, with the reason it is still worth four lines. | design.md → Decisions 12 |
| SUGGESTION | proposal.md | The downstream-consumer list omitted `Heading`, `live-refresh`, and `degraded-states`, all of which design.md → Contracts names. | Completed. | proposal.md → Impact |
| SUGGESTION | tasks.md | The reviewer briefing in group 7 did not mention the checks this review found to be green by construction, so the implementation reviewer would have had no reason to look at them. | Three items added to 7.1: the base SHA in the manifest diff, both halves of the scan command, and the provenance of 4.3's absolute counts. Plus a check that `Tasks::items` was not reintroduced. | tasks.md 7.1 |

Counts after repair: 12 requirements and **40 scenarios** across two capability specs; every
scenario appears in design.md's verification matrix and in exactly one tasks.md group;
**66 tasks in 9 groups**; `openspec validate task-parsing --strict` reports the change
valid.

## No Remaining Implementation-Blocking Gaps

None remain. The three CRITICALs were all of one kind — a verification command that could
not fail — and all three are repaired in both the artifact that specifies them and the
task that runs them. No unresolved decision requires user input.

Two findings were **consciously accepted rather than repaired**, each with its reason:

- **The invalid-UTF-8 divergence from the CLI is kept, not eliminated.** Lossy decoding
  would preserve numerical agreement and was rejected: it assigns a confident count to
  bytes nobody can read. The divergence is now written into the spec, the design's Risks,
  and the degraded-states row this change adds, so it is inherited rather than
  rediscovered.
- **"A file-derived count and a CLI-shaped count compare equal" is a weak scenario.** B is
  right that no CLI is in the loop and that it can only fail for reasons other rows already
  cover. It is kept because it is the only place the *type-shape* claim — that a CLI
  response's pair constructs this value with no conversion — is written down as a test, and
  it is one assertion.

## Deferred Non-Blocking Notes

- **`tests/fixtures/tasks/` is a fixture shape `SPEC.md` → Fixtures does not describe.**
  That section names three fixture *repositories*; this change adds a directory of
  standalone task documents beside them. Additive, not contradictory, and left alone
  deliberately: `changes-from-files` is the change that will actually build fixture
  repositories, and it is the right place to widen that section rather than widening it
  here for a directory of five files.
- **The roadmap's `task-parsing` row understates its test-surface dependency.** The row
  says `repo-foundation`; the scratch-tree and containment tests also use
  `testutil::ScratchDir` and `testutil::snapshot`, which `plugin-config` introduced and
  `repo-resolution` extended. Both have landed, so this is recorded as a note in the row
  (task 8.5) rather than as a new edge in the Mermaid graph, and the decision not to add
  the edge is recorded so its absence is not read as an oversight.
- **Nothing pins the no-spawn rule between changes.** Group 6's scan runs once. Wiring a
  standing tree-wide scan into `make check` would change the `quality-gates` capability,
  which is a different change's work; this is now stated plainly in design.md and tasks.md
  instead of being implied to already exist.
