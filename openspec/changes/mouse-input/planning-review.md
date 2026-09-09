## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/mouse-input/spec.md` (new capability)
- `specs/terminal-lifecycle/spec.md` (modified)
- `specs/dashboard-loop/spec.md` (modified)
- `specs/responsive-layout/spec.md` (added)
- `specs/list-selection/spec.md` (added)
- `specs/artifact-tabs/spec.md` (added)
- `specs/detail-scroll/spec.md` (added)
- `specs/change-rows/spec.md` (added)
- `specs/quality-gates/spec.md` (modified — added during this review)

The finding pass was delegated to four `planning-reviewer` subagents, none of which wrote
the plan, sliced as the schema directs: (A) capability coverage, scenario quality, and
cross-artifact contradictions; (B) design completeness, test boundaries, and whether each
proposed check could fail at all; (C) task alignment, lifecycle discipline, and
`parallel-after` independence; (D) factual verification of every empirical claim. They
reported findings and edited nothing. Every finding below was re-verified in this session
against the source or by running the command before being acted on; three reviewer claims
were checked and **not** repaired, and are recorded as such at the end.

## Reviewed Against

- This repository HEAD: `a156f9a` for every code-level measurement. The planning artifacts
  were committed on top as `65924be`, `702bf9c`, `3e42819`, and `86bf590`;
  `git diff --stat a156f9a HEAD -- src scripts Makefile SPEC.md AGENTS.md tests openspec/specs`
  is empty, so every measurement holds at both.
- Sibling repositories: `Not applicable`. This change touches no contract shared with
  `~/Code/openspec-schemas` or any other repository.
- Working tree: clean apart from this change's own artifacts under
  `openspec/changes/mouse-input/`, which were intentionally included.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/terminal-lifecycle/spec.md` | The panic path was promised by `proposal.md` and implemented by task 1.3, but no delta covered `A panic restores the terminal`, whose three scenarios pin exact call lists that `disable_mouse` invalidates. The implementer would have hit four red tests with no spec authorising the new lists. | Added the requirement as a MODIFIED block with `disable_mouse` leading each restoring list, the worker-thread scenario deliberately unchanged, and the "byte-identical" clause restated as "differs by exactly that one leading entry". Found while verifying it: `The dashboard refuses to start when stdout is not a terminal` asserts the same list in its `Ok` arm and needed the same treatment. | `specs/terminal-lifecycle/spec.md` → both MODIFIED blocks; `tasks.md` 1.2; `design.md` matrix, 5 rows |
| CRITICAL | `design.md`, `proposal.md` | Both stated that ignoring motion events "keeps the render loop from redrawing on every pointer move". Measured false: crossterm 0.29.0's `EnableMouseCapture` writes `?1003h` — any-event tracking (`crossterm-0.29.0/src/event.rs:321-335`) — and `run_loop` draws at the top of every iteration before the event is read, so an `Action::Ignore` still costs a full frame. `design.md` → Risks contradicted its own Non-Goals two sections later. | Decided at planning time rather than left to group 1: `run_loop` skips the draw, the sync, and the normalisation for a `Moved`/`Drag(_)` event, carries the previous `area` forward, and does not count the skipped iteration in `LoopSummary::frames`. Both alternatives (accept the cost; hand-write the narrower CSI) are recorded with their reasons. Bound by a frame-count scenario — the only observable that distinguishes a drawn frame from a skipped one. | `design.md` → Decision 12; `specs/dashboard-loop/spec.md` → new requirement, 2 scenarios; `proposal.md` → Non-Goals; `tasks.md` 6.1, 6.5, 6.7 |
| CRITICAL | `design.md`, `tasks.md` 8.3 | "`wired.sh`'s leg 1 name list gains `mouse_problem`, so a `run` that stops threading the guard's reason fails" — structurally impossible. Leg 1 greps the whole production slice of `src/ui/mod.rs` (`wired.sh:158-163`), and `pub struct Startup` is declared in that slice at `:94`; the field declaration alone satisfies the name. Reviewer D reproduced it: gate green on exactly the defect it was claimed to catch. | A body-scoped **leg 5c** instead, mirroring leg 5's two-part shape (`$body` must name `mouse_problem(`, and must not hardcode `mouse_problem: None`), with a Guard-A positive control on the defining file. Leg 1 stays at thirteen names, so `wired.sh:246`'s message stays true. The gate-control plant is rewritten as one the `[[control]]` find/replace format can express. | `specs/quality-gates/spec.md`; `design.md` → Boundaries; `tasks.md` 8.3, 8.4, 8.5 |
| CRITICAL | `tasks.md` 6.4, `design.md` matrix | Three matrix commands were name filters matching zero tests, and `cargo test --lib the_full_key_table` prints `running 0 tests … 1158 filtered out` and **exits 0**. `app::tests::the_full_key_table_is_unchanged` exists nowhere and no task wrote it; the real terminal tests are `enable_raw_failure_attempts_nothing_further` and `alternate_screen_failure_unwinds_raw_mode`. Task 6.4 was the plan's only stated proof that no key moved, and half of it ran nothing. | Repointed all three rows at the tests that exist, at their line numbers. Stated the rule once for the whole matrix rather than per row: a filtered command that reports `0 passed` is a failure, not a pass. Recorded as planning-time check L2 with its measured output. | `design.md` → Test Strategy preamble and 5 rows; `tasks.md` check L2, 1.1, 1.2, 6.4 |
| CRITICAL | `tasks.md` group 10 | Group 10 was classified `behavior` with a RED task that could not honestly fail: group 9 writes the `SPEC.md` and `AGENTS.md` passages first, so a correctly written doc-binding test passes the moment it is written. The only way to see RED was a stub — the manufactured RED the schema forbids. Its 10.3 was also a `CHANGE` verb inside a behavior group. | Reclassified `operational`, CHECK → CHANGE → VERIFY, with the negative control as its evidence: write the test, delete the passage in a scratch copy, show it fires, restore it. | `tasks.md` group 10 |
| WARNING | `specs/quality-gates/` (absent) | `tasks.md` 8.3 changed `wired.sh`'s required-name list, whose size `openspec/specs/quality-gates/spec.md:922` states as "thirteen". A specified invariant would have been edited with no delta authorising it. | Added a `quality-gates` delta and listed the capability under `proposal.md` → Modified Capabilities. It states that `mouse_problem` does **not** join leg 1 and why, and adds a scenario for leg 5c that also asserts leg 1 stays green against the same planted tree. | `specs/quality-gates/spec.md`; `proposal.md` → Capabilities |
| WARNING | `specs/terminal-lifecycle/spec.md` | The `NORAW-GREP` positive control is one-of-any (`noraw-grep.sh:13-15`), satisfied by `enable_raw_mode` alone, so the extended pattern would cover the two capture commands vacuously — and task 8.2 closed only the file side, leaving the pattern side open. | The control becomes per name and two-directional: for each of the six, the name must match `RAW_RE` **and** appear in `src/ui/terminal.rs`. Gate controls go from three to four, one plant per direction for the capture pair. | `specs/terminal-lifecycle/spec.md`; `tasks.md` 8.2, 8.5 |
| WARNING | `specs/mouse-input/spec.md` | `mouse_bindings_match_spec_md` had no stated extraction rule and read English prose, while every existing `doc_contract` leg states its rule and carries a parser control. The naive form passes on a resolver with swapped arms. | Each mouse-table row must carry its `Action` variant in backticks; the test compares that set against `Action::` identifiers in `mouse_action`'s own body, cut from the production slice — not the file, whose inline test module names every variant. Added a scenario requiring failure on an unreadable side plus a parser control. | `specs/mouse-input/spec.md` → documentation requirement, +1 scenario |
| WARNING | `specs/mouse-input/spec.md`, `design.md` | `MouseEvent::modifiers` was never mentioned. Nothing said whether `Shift+click` or `Ctrl+wheel` acts, while `action_for`'s spec is meticulous about exactly that — and the change documents holding `Shift` as the drag-select escape hatch. | Modifiers are ignored, stated in the resolver requirement, added to the totality scenario's cross product, and argued in a Decision: the `Shift` override is the terminal's and is applied before any sequence is sent, so refusing modified presses would only break the readers whose terminals honour it. | `specs/mouse-input/spec.md`; `design.md` → Decision 13 |
| WARNING | `tasks.md` check L, `design.md` → Contracts | Check L reported 11 `Startup` construction sites; five of those hits are `ProbedStartup {`, a different test-local struct at `src/ui/mod.rs:2755`. There are 6. `design.md` said five. Neither was right, and the check could not move — 11 before and 11 after — so task 7.4 re-asserted it as a contract gate that passes at HEAD with no work done. | Corrected to 6 in both, with the six line numbers. Task 7.4's check replaced with one whose subject is the field and which actually changes: `grep -c 'mouse_problem:' src/ui/mod.rs`, `0` at HEAD and `6` after. Recorded that `nodefault-ui.sh` never reads `Startup` — it loops only over its `$TYPES` argument. | `tasks.md` check L, 7.2, 7.4; `design.md` → Contracts |
| WARNING | `tasks.md` top note | The sequential-groups justification claimed "every group here closes on `cargo test --all-features`, one whole-crate compile and one whole-suite run". None does — every group closes on a filtered `cargo test --lib …`, and the precedent it cited ran broader commands and was still marked parallel. | Replaced with the real discriminator, criterion 2 rather than criterion 3: `grep -n 'ui::app' src/ui/layout.rs src/ui/detail.rs src/ui/list.rs` reports **24** sites, so groups 2, 3 and 4 all compile against the `src/ui/app.rs` group 5 rewrites. A filter selects which tests run, not which crate is built. | `tasks.md` lines 15-33 |
| WARNING | `design.md` → Test Boundaries | The table named twelve collaborators but not the panic hook, though both artifacts claim capture is released "on panic alike" — and adding `disable_mouse` to the shared `restore_then` breaks four tests no task named. | Added a panic-hook row saying what it is: not reached, untested by construction because installing one is process-global, with its body `restore_then_if` driven through the recording double. The four tests are named in task 1.2, along with the two that must stay unchanged and why. | `design.md` → Test Boundaries; `tasks.md` 1.2 |
| WARNING | `specs/list-selection/spec.md` | The requirement said `Action::Click` "SHALL change nothing but `selected`, `route`, `detail.tab`, `detail.scroll`, and `sections.collapsed`" while the next requirement and `mouse-input` both assert a header click sets `refresh.requested`. | Added `refresh.requested` to the enumerated set, scoped to how it is actually reached: only through `apply`'s blanket post-action rule, never written by a `Click` arm. | `specs/list-selection/spec.md` |
| WARNING | `tasks.md` 4.1/6.1 | Two `artifact-tabs` click scenarios fell between groups — 4.1 scoped to the four addressing scenarios, 6.1's wording named only `mouse-input`'s and `dashboard-loop`'s. | 6.1 now names both by test name. | `tasks.md` 6.1 |
| WARNING | `tasks.md` groups 2, 4, 7 | Three behavior groups had no REFACTOR task and did not state none was needed, which the schema requires of the verification task. | Appended the statement to each closing task. | `tasks.md` 2.4, 4.4, 7.6 |
| WARNING | `tasks.md` group 1 | Group 1 changes a trait with three implementors — `design.md` → Contracts calls it breaking — and carried no contract gate, leaving check M unbound to any task. | Added 1.5 as the contract gate over the three `impl TerminalOps for` sites. | `tasks.md` 1.5 |
| WARNING | `tasks.md` 2.1/3.1 | `responsive-layout`'s fifth scenario, "The hit test agrees with what was drawn", had no task: group 2 scoped to the four pure scenarios and no later group picked it up. | Added to 3.1's RED list, where it belongs — it is a view test in `src/ui/view.rs`, which group 3 already writes; putting it in group 2 would have made groups 2 and 3 share a file. | `tasks.md` 2.1, 3.1 |
| SUGGESTION | `proposal.md` | Three false claims: the hit test returns `Target` (it returns `Zone`); "`change-rows`: a click on a section header toggles it" (that delta holds only `row_at`); "`color-palette` is independent of it" (its `Tab::x` is what `tab_at` resolves against). Impact named five source files where the design names eight, and no checked-in verification file. | All three corrected, with the `Target`→`Zone` rename explained in place rather than silently applied. Impact rewritten to eight source files plus the five checked-in verification files, and a What Changes bullet added for the refused-capture degraded state. | `proposal.md` → What Changes, Capabilities, Impact |
| SUGGESTION | `design.md` → Decision 8 | "putting the resolver in `app` would close a module cycle" is false: Rust permits intra-crate module cycles and this crate already has one (`app.rs:557` → `ui::detail`, `detail.rs:34` → `ui::list`, `list.rs:6` → `ui::app`). A future reader would have inherited a constraint that does not exist. | Dropped the cycle argument; the decision now rests on its real reason, that `driver` is where the frame `area` already lives. | `design.md` → Decision 8 |
| SUGGESTION | `design.md` → Boundaries, `tasks.md` 2.3 | "`noio-view.sh`'s and `colwidth.sh`'s nine-file `PURE` lists" — `colwidth.sh`'s is eight and omits `src/ui/layout.rs`, so task 2.3 cited a gate that never reads the file it was checking. | Corrected to nine and eight, and 2.3 now cites only `noio-view.sh`, naming why `colwidth.sh` cannot apply. | `design.md` → Boundaries; `tasks.md` 2.3 |
| SUGGESTION | `design.md` → Test Boundaries | "`CrosstermOps` is never constructed outside `ui::run`" — it is also constructed at `src/ui/terminal.rs:163`, inside `install_panic_hook`. | Corrected to name both sites and what `NORAW-GREP` leg 2 actually proves. | `design.md` → Test Boundaries |
| SUGGESTION | `design.md` matrix | The row for "The other buttons and the non-press kinds are inert" claimed the test "asserts the launcher double received nothing" — impossible, since `mouse_action` is pure and takes no `Launcher`, so the assertion is true before the function exists. | Restated as a pure assertion on the returned `Action`, naming the four launch variants it must not be, with the collaborator column corrected to none. | `design.md` → Test Strategy |
| SUGGESTION | `tasks.md` 5.4 | `Action` has a third hardcoded enumeration site the task omitted: `assert_eq!(variants.len(), 18, "the eighteen variants this crate specifies")` at `src/ui/app.rs:2030`. | Named in 5.4 alongside the array and the enum. | `tasks.md` 5.4 |
| SUGGESTION | `tasks.md` group 9 | Group 9 was four CHANGE-shaped tasks with no CHECK and no VERIFY, and its independence from group 8 was never examined. | Added 9.1 as a CHECK over what is stale and 9.7 as a VERIFY, plus 9.6 correcting `AGENTS.md`'s "seven further claims" doc-conformance count that group 10 makes nine. Marked `parallel-after: 7`: groups 8 and 9 share no file and neither touches a `.rs` file. | `tasks.md` group 9, ordering note |
| SUGGESTION | `tasks.md` 13.3 | 13.3 re-ran `make coverage`, which 13.2's `make check` already runs (`Makefile`: `check: fmt-check lint gates test coverage`), against a project rule asking for `make check` as the single gate. | Folded into 13.2 as its named sub-command, keeping the "add tests, never lower a floor" instruction. | `tasks.md` 13.3 |
| SUGGESTION | `specs/mouse-input/spec.md` | The new capability will get `openspec archive`'s `TBD - created by archiving change` placeholder, on which `tests/spec_purposes.rs` fails `cargo test` — HEAD's tip commit is that exact trap firing on the previous change. Nothing in the plan wrote a Purpose. | Added 13.5 requiring the paragraph immediately after archiving, verified by `cargo test --test spec_purposes`. | `tasks.md` 13.5 |
| SUGGESTION | `tasks.md` baseline note | The note said the working tree held the artifacts untracked; they had since been committed. | Restated with the four commits and the `git diff --stat` that shows the code baseline is unmoved. | `tasks.md` header |

Three reviewer claims were checked and deliberately **not** repaired:

- **`tasks.md` check I's plant line number.** Reviewer D reported `5294` against the
  recorded `5295`. Both are right for their own plant: the recorded run used
  `printf '\n// EnableMouseCapture\n' >> src/ui/app.rs`, whose leading newline puts the
  comment two lines past the file's 5293. The command is now written out in full beside the
  number so the run reproduces.
- **`wired.sh:246`'s hardcoded "thirteen names present".** Flagged as going stale. It does
  not: the repair uses leg 5c rather than a fourteenth leg-1 name, so thirteen stays true.
  Task 8.4 extends the message to mention leg 5c rather than restating a count.
- **Marking groups 2, 3 and 4 `parallel-after: 1`.** Reviewer C observed correctly that the
  precedent (`seam-resilience` groups 5-7) ran broader commands and was still parallel. They
  stay sequential on criterion 2, not criterion 3 — the 24 `ui::app` sites above are a real
  dependency, and the schema directs marking conservatively.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL is repaired in the artifact that owns it, and every WARNING is
either repaired or recorded above with the reason it was not. `openspec validate mouse-input
--strict` reports the change valid, all **84** spec scenarios appear in `design.md`'s
verification matrix, and no decision is left that requires user input.

## Deferred Non-Blocking Notes

- **A pre-existing test flake on `main`, outside this change.**
  `ui::tests::wiring::g_focuses_the_agent_the_launch_started` failed in two of three full
  `cargo test --all-features` runs at `a156f9a`, and
  `wiring::an_unreachable_scratch_herdr_is_a_standalone_tui` in one, while
  `cargo test --all-features --lib ui::tests::wiring:: -- --test-threads=1` passed 27/27.
  Both spawn scratch `herdr` programs and race under parallel execution. Recorded at the top
  of `tasks.md` and again in task 13.2 so a failure confined to those two names is not
  misread as this change's regression. It is a bug fix in its own right and deliberately not
  folded into this change's scope.
