## Reviewed Artifacts

- `openspec/changes/live-refresh/proposal.md`
- `openspec/changes/live-refresh/design.md`
- `openspec/changes/live-refresh/tasks.md`
- `openspec/changes/live-refresh/specs/watch-invalidation/spec.md` (new)
- `openspec/changes/live-refresh/specs/refresh-worker/spec.md` (new)
- `openspec/changes/live-refresh/specs/live-updates/spec.md` (new)
- `openspec/changes/live-refresh/specs/dashboard-loop/spec.md` (modified)
- `openspec/changes/live-refresh/specs/cli-changes/spec.md` (modified)
- `openspec/changes/live-refresh/specs/artifact-content/spec.md` (modified)
- `openspec/changes/live-refresh/specs/change-rows/spec.md` (modified)
- `openspec/changes/live-refresh/specs/plugin-build/spec.md` (modified)
- `openspec/changes/live-refresh/specs/quality-gates/spec.md` (modified)
- `openspec/changes/live-refresh/specs/tasks-checklist/spec.md` (modified — **added during review**)
- `openspec/changes/live-refresh/specs/detail-scroll/spec.md` (modified — **added during review**)

## Reviewed Against

- This repository HEAD: `f9b42e8d11ead35049297451996d77e70869bd2f`
- Sibling repositories: **Not applicable.** This plugin has no sibling repository contract. The
  two external programs it names — `openspec` (`@fission-ai/openspec` 1.11.0) and `herdr` — are
  reached through the `cli` seam and were not touched by this change; `herdr` is not reached at
  all. The one external contract that *was* read is `notify` 8.2.0's own source in
  `~/.cargo/registry`, for the API, feature, MSRV, and drop semantics recorded in `design.md`
  → Decisions 13, 24, and 25 and in Risks.
- Working tree: clean apart from `openspec/changes/live-refresh/`, which is this change's own
  planning package. Verified with `git status --porcelain` after every planted-violation revert
  in every review round, and again at the end.

**Four independent reviewer subagents** ran, none of which wrote the plan, none of which was a
fork of the planning session, and each writing findings incrementally to a scratchpad file:

| Reviewer | Slice | Verdict |
|---|---|---|
| gates | the twenty-six command-level checks: extract them, run them, plant violations, verify every floor and every string-replacement anchor | 0 CRITICAL, 6 WARNING, 4 SUGGESTION |
| timing | every wait, sleep, deadline, clock read, and concurrency claim; the "must not block the draw" argument | 3 CRITICAL, 8 WARNING, 3 SUGGESTION |
| coherence | delta-spec completeness, scenario↔matrix↔task traceability, `openspec/config.yaml` rule compliance, `SPEC.md`/`AGENTS.md` truth | 5 CRITICAL, 9 WARNING, 5 SUGGESTION |
| fixes (second round) | verify every CRITICAL was closed and hunt for damage the repairs introduced | 2 new CRITICAL, 6 WARNING, 3 SUGGESTION |

Every check the reviewers ran was run against the real tree, not reasoned about. Every planted
violation was reverted and the tree confirmed clean.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs/refresh-worker | `queued_requests_are_folded` **cannot fail**: the union of `Only({a}) ∪ Only({b}) ∪ All` is `All`, so "nothing outside the folded selection" is a tautology; `from_cli_cached` applies to every uncached change, so the worker's first cycle is a full apply regardless; and "the last `Merged` result" is knowable only by a timeout followed by a negative assertion — a negative assertion across a thread boundary, the exact defect the plan's own preamble forbids | Scenario replaced: `drain_and_fold(first, &rx)` is extracted as a named private function and called **directly on the test's own thread**, over a receiver pre-loaded with two selections whose sender has been dropped. The threaded version is deleted with the reason recorded, so it cannot be reinstated as "corroboration" | `specs/refresh-worker/spec.md` → "Queued requests are folded into one cycle"; `design.md` → Decisions 22; `tasks.md` 7.1 (third Red-when), 7.2 |
| CRITICAL | specs/refresh-worker | `dropping_the_refresher_disconnects_the_channel` is **unwritable**: the real `Refresher` owns the result `Receiver`, so dropping it destroys the only witness, and `Box<dyn Refresher>` cannot be downcast. The natural repair, `rx.recv_timeout(10s).is_err()`, passes on `Err(Timeout)` — i.e. green precisely when the worker leaks | A `#[cfg(test)]` seam is specified: `worker_for_test(…) -> (Box<dyn Refresher>, Receiver<RefreshResult>, Receiver<()>)`, whose third element the worker's `Sender` drops on return. The assertion is `assert!(matches!(…, Err(RecvTimeoutError::Disconnected)))`, **never** `is_err()`, with that stated as a Red-when | `specs/refresh-worker/spec.md` → the seam paragraph and "Dropping the refresher ends the worker"; `design.md` → Contracts; `tasks.md` 7.1 (second Red-when), 7.2 |
| CRITICAL | tasks.md (`NOBLOCK`) | The "watcher must not block the draw" evidence did not reach the two files that can block. `NOBLOCK` greps `src/ui/driver.rs` and `src/ui/**` only; `src/watch.rs` and `src/refresh.rs` appeared **solely as positive controls**. A `drain` written `rx.recv_timeout(150ms)` and a `take_result` written `rx.recv_timeout(400ms)` satisfy `NOCLI-SHELL`, both existing legs, `NOSLEEP`, `WATCHSEAM`, every trait signature, and every frame-and-poll count — while delaying each draw by half a second. `proposal.md`'s "by construction rather than by comment" was false for exactly those two files | **`NOBLOCK` leg 3 added and verified**: `src/watch.rs` may name no blocking receive at all; `src/refresh.rs` none **before** its single `thread::spawn`; both must name `try_recv` as positive controls. A runtime latency test was deliberately **not** added — it would be an elapsed-time assertion, hazard 1 reintroduced. `proposal.md`'s claim narrowed to what is proven | `tasks.md` → the `NOBLOCK` block (leg 3, Guards D and E); `specs/live-updates/spec.md`; `specs/dashboard-loop/spec.md` → "The render path names no channel, thread, lock, or clock"; `design.md` → Decisions 21; `proposal.md` |
| CRITICAL | specs (missing delta) | `openspec/specs/tasks-checklist/spec.md` states the `Action` enum "holds exactly the **twelve** variants". This change makes it thirteen, and `tasks-checklist` was not in Modified Capabilities — the live spec would have been left false and its landed test unextended | `tasks-checklist` added as a MODIFIED delta carrying all three of its live scenario names with full bodies; task 8.2 extends `no_action_mutates_a_task_item` to sweep thirteen | `specs/tasks-checklist/spec.md` (new file); `proposal.md` → Modified Capabilities; `tasks.md` 8.2; `design.md` matrix (3 rows) |
| CRITICAL | specs (missing delta) | `openspec/specs/detail-scroll/spec.md` states the `Dashboard` companion "continues to name all **eight**" and pins the no-`Default` check to the type list `Dashboard Filter Detail`. This change makes `Dashboard` nine fields and runs the check over four types | `detail-scroll` added as a MODIFIED delta carrying both its live scenario names with full bodies, updated to nine fields and four types, and recording that `Detail` deliberately stays at five | `specs/detail-scroll/spec.md` (new file); `proposal.md` → Modified Capabilities; `design.md` matrix (2 rows) |
| CRITICAL | design.md | The matrix named `watch::tests::a_removed_openspec_directory_keeps_the_loop_drawing`, which **no task wrote** and for which the `watch::tests::` floor budgeted no room. As designed it was also a "drain for N seconds and assert no `Err` ever came" shape — a negative assertion bounded by elapsed time | The row now names `ui::driver::tests::a_watch_error_is_recorded_once` (a `ScriptedFs` returning `Err`, deterministic) plus `ui::view::tests::a_removed_repo_shows_the_problem_row`, both tasked. The spec scenario says explicitly why the real-watcher form was rejected | `design.md` matrix; `specs/watch-invalidation/spec.md` → "`openspec/` is removed while the watcher runs"; `tasks.md` 9.1 (third Red-when) |
| CRITICAL | specs/dashboard-loop | The "Loading writes nothing" delta gained a clause requiring a real `watch::start` inside `ui::tests::load::loading_writes_nothing` — which no task implemented and which contradicted the design's own rule that no `ui::` test may open a watcher | Clause replaced: the watcher half of the claim is `watch::tests::a_started_watcher_writes_nothing`, and the boundary rule stays absolute for `ui::tests::load::`. The Test Boundaries rule was separately amended to name the single `ui::` test that legitimately opens one | `specs/dashboard-loop/spec.md`; `design.md` → Test Boundaries closing rule and the matrix row |
| CRITICAL | specs/refresh-worker | The mandatory `openspec/config.yaml` boundary case "a schema the CLI rejects" was uncovered — and this change makes it newly load-bearing, because the new `CliCache` caches problems: such a change either stops being re-asked forever or is re-asked every cycle, and neither was specified | One requirement sentence and one scenario added: a rejected schema is cached like any other, so it costs one `instructions apply` per selection rather than one per cycle, and `r` re-asks. A fourteenth `changes::tests::` test carries it; the floor moves 193 → 194 | `specs/refresh-worker/spec.md`; `tasks.md` 3.1, 3.5; `design.md` matrix |
| CRITICAL | tasks.md (`worker_for_test`) | *(second round)* The seam introduced to fix the drop scenario returned `(Box<dyn Refresher>, Receiver<()>)` — the **exit** channel — yet three scenarios read `RefreshResult` off it. The result receiver was still unreachable, moving the unwritable-test defect from one scenario onto three | Signature changed to a three-tuple: second element the result `Receiver`, third the exit channel, with `take_result` on the returned `Refresher` documented as always `None` in these tests | `specs/refresh-worker/spec.md`; `design.md` → Contracts and Test Boundaries; `tasks.md` 7.2 |
| CRITICAL | tasks.md (`prod()` stripper) | *(second round)* `READONLY-UI` and `NOBLOCK` build a production slice by discarding everything from a file's **first** line-anchored `#[cfg(test)]` to EOF — an assumption verified for `src/ui/*.rs`, but this change *mandates a second one* in `src/refresh.rs` (`worker_for_test`). Reproduced: with it above `start()`, `READONLY-UI` reports **OK** on a `std::fs::write` in the worker's start path, and `NOBLOCK`'s spawn control false-reds on correct code | **Guard D added to both checks**: each seam module must hold exactly **one** line-anchored `#[cfg(test)]`, failing with the reason. A count is a check; a placement rule is a convention. The spec and task also state the placement | `tasks.md` → `NOBLOCK` and `READONLY-UI` blocks; `specs/refresh-worker/spec.md`; `design.md` → Contracts; `tasks.md` 1.5, 7.2, 7.5 |
| WARNING | tasks.md (`NOSLEEP`) | Leg 2's absolute ban covered `src/watch.rs`, which forced the one real-watcher test into a `yield_now` busy-spin — a core held for the whole window and, on a loaded two-core runner, competing for CPU with the very `notify` thread producing the event it waits for. The stated justification ("a sleep could make an assertion premature") is false inside a deadline-bounded poll, and `tests/cli.rs:141` already carries the comment proving it | Leg 2 narrowed to `src/ui/`; the poll sleeps **10ms**, the shape this repository already uses. `SLEEP_MIN` rises 3 → 4 at group 6 | `tasks.md` → `NOSLEEP` block, 5.4, 6.4, 6.6, 7.5, 14.6; `specs/dashboard-loop/spec.md`; `specs/watch-invalidation/spec.md`; `design.md` → Decisions 25, Risks |
| WARNING | tasks.md (`NOSLEEP`) | *(second round)* That narrowing reopened the hole in exactly the file group 6 writes: leg 1's span splitter attributes a sleeping helper to the **preceding** test, and group 6 puts a correct deadline-bounded test in `src/watch.rs`. A 300ms `settle()` below it passes — reproduced | **Leg 2b added**: the two seam modules may name a sleep **at most once each**, which is the plan's own claim ("exactly one sleep, in the one real-watcher test") made checkable. Verified firing on a planted second sleep | `tasks.md` → `NOSLEEP` block, 6.6 |
| WARNING | tasks.md (`NOBLOCK`) | *(second round)* Four blocking shapes passed leg 3, the worst silently: a module doc comment naming `thread::spawn` on line 1 armed the awk cut at line 1 and **disarmed the whole refresh half**; `rx.iter()` and `for r in &rx` block and name no `recv`; and `take_result` moved below `start` lands in the worker half | The cut now ignores comment lines (`^[^/]*thread::spawn`); the leg-3 pattern gains `rx\.iter\(\)` and the `for … in …rx` form, **anchored on a receiver-shaped binding** rather than a bare `\.iter\(\)`, which would false-red on `Vec::iter()`; **Guard E** requires `take_result` to be declared above the spawn. All four verified firing | `tasks.md` → `NOBLOCK` block; `specs/refresh-worker/spec.md`; `tasks.md` 1.5, 7.5 |
| WARNING | tasks.md (`NOBLOCK`) | *(second round, found while verifying)* The awk form `awk -v re="$BLOCK3_RE" '… $0 ~ re …'` **fails open**: an ERE passed through `-v` is re-escaped as a string literal and awk rejects it with `illegal primary in regular expression` on stderr while the leg still printed OK. Measured | awk now only *cuts* the file; the matching is grep's. One tool, one job, with the failure mode recorded in the block's own comment | `tasks.md` → `NOBLOCK` leg 3 |
| WARNING | tasks.md (`NOBLOCK`) | Leg 2's `CLOCK_RE` carries a bare `Instant::` and is comment-inclusive, so a doc comment reading "the crate's one `Instant::now()` lives in `watch::RealFsEvents::drain`" — the sentence tasks 9.4 and 13.2 push toward — turns it red on correct code. The `NOTABSEAM` failure mode, which this repository shipped once already | The rule is now stated in the block's header and in the spec: every doc comment under `src/ui/` says "the clock", never `Instant::now()`, on the same terms `markdown-render` imposes on `src/ui/markdown.rs`'s prose. Prose without `::` verified safe | `tasks.md` → `NOBLOCK` block, 0.4; `specs/dashboard-loop/spec.md` |
| WARNING | specs/watch-invalidation | The debounce extended on every push with **no ceiling**, so a writer saving more often than every 150ms — an agent editing `tasks.md`, then a spec, then a design doc, which is this change's *motivating case* — defers the batch forever and the pane never updates while it is most useful | `DEBOUNCE_MAX` (1 second) added: the window's end is `min(now + DEBOUNCE, first_push + DEBOUNCE_MAX)`, with a scenario that pushes every 100ms through `t0 + 1100ms` and asserts `take_due(t0 + 1000ms)` is `Some`. Watch floor 21 → 22 | `specs/watch-invalidation/spec.md`; `tasks.md` 5.1, 5.2, 5.5; `design.md` → Decisions 23 |
| WARNING | specs/watch-invalidation | *(second round)* `take_due` "empties the state" never said whether `first_push` is part of it. An implementation clearing the paths and the end but not `first_push` is green against **every** scenario and leaves its cap permanently in the past — the debounce off for the rest of the session | The clause now names `first_push` explicitly, and the continuous-writer scenario gains a push-after-take assertion that discriminates it | `specs/watch-invalidation/spec.md` |
| WARNING | specs/watch-invalidation | `pending_in`'s "saturating at zero" had no scenario for `now > end`, and the obvious `end - now` **panics** (`Instant - Instant` panics when its argument is the later of the two). The operand order of `saturating_duration_since` was never stated either, and the wrong order returns zero for the whole window | The exact expression is written into the requirement (`end.saturating_duration_since(now)` — `end` the receiver), and `pending_in(t0 + 400ms) == Some(0ms)` added as the discriminating assertion, with a third Red-when in task 5.1 | `specs/watch-invalidation/spec.md`; `tasks.md` 5.1 |
| WARNING | specs/watch-invalidation, design.md | `FsEvents::pending_in(&self)` takes no `Instant` while `Debounce::pending_in(&self, now)` does. `RealFsEvents::pending_in` would therefore either call `Instant::now()` — making the "one clock binding" claim false and giving `src/watch.rs` two — or return a cached value the contract never mentioned. Nothing chose, and `NOBLOCK`'s clock control is satisfied either way | `RealFsEvents` stores the `Option<Duration>` its previous `drain` computed and returns it verbatim; sound because `run_loop` calls the two in the same iteration. Added as a third Red-when on task 6.5 | `specs/watch-invalidation/spec.md`; `design.md` → Decisions 24, Test Boundaries; `tasks.md` 6.5 |
| WARNING | tasks.md, design.md | Task 11.1 asserted an **exact equality** on `refresher.requests()` while a **live** OS watcher was attached. It holds only because the ~110 scripted keypresses finish inside the 150ms debounce window — a budget nothing states, nothing asserts, and `cargo llvm-cov`'s 2–5× instrumentation erodes. A test passing because a race resolved favourably | Split into two `ui::tests::live::` tests: `r_forces_a_refresh_through_the_loop` (scripted `FsEvents`, exact request equality — sound because nothing can append) and `a_live_watcher_over_the_tree_writes_nothing` (real watcher, byte-identity only). Floor 2 → 3 | `tasks.md` 11.1, 11.2, 11.4; `specs/live-updates/spec.md`; `design.md` matrix and Test Boundaries |
| WARNING | tasks.md (`MDWIDTHS`) | `MD_MIN=23` sat **below** the tree's own 24 `#[test]` in `src/ui/markdown.rs` — the one floor that was already satisfied and therefore could not fail | Raised to 24, exactly met, with the reason recorded in task 0.1 | `tasks.md` 0.1, 0.3, 14.6 |
| WARNING | tasks.md, design.md, proposal.md | The check inventory did not add up three ways: "eighteen blocks" against a list of twenty-one; "twenty-five files" against 5 + 21 = 26; "twenty-three gates" in `design.md`; and the proposal named `OPENSPEC-UNTOUCHED` as edited while omitting `READONLY-UI`, which is | Settled at **26 files / 25 gates + `TESTCOUNT.sh`**, 21 extracted (19 unchanged, 2 edited in task 6.2), 5 reproduced; every downstream count corrected | `tasks.md` preamble, 0.1, 0.2, 14.1; `design.md` → Test Strategy; `proposal.md` → Impact |
| WARNING | tasks.md | `GATE-MECH1.py`'s label sits on its block's **second** line, behind a shebang. A first-line-only extractor silently drops it | Task 0.1 now says to scan each block's first **two** lines, and task 0.3 treats a missing `GATE-MECH1.py` as an extraction defect | `tasks.md` preamble, 0.1 |
| WARNING | design.md | Six matrix rows named tests no task wrote, by name — `action_count_is_thirteen`, `an_unchanged_key_re_reads_nothing`, `poll_timeout_never_returns_zero`, `a_watch_problem_leads_the_list`, and two `ui::list::tests::` names — and one more (`queued_requests_are_folded`) survived the C1 repair | Every `Verification` cell reconciled to the exact name its task writes; verified mechanically afterwards, scenario↔matrix now **111 ↔ 111, set-identical** | `design.md` verification matrix |
| WARNING | tasks.md | Two new `artifact-content` behavioural clauses (a forced reload clears `detail.problems`; the flag is cleared on the empty-list path) had matrix rows but no task | Task 8.2 gains an explicit `sync_detail` extension list alongside its key-test list | `tasks.md` 8.2 |
| WARNING | proposal.md | The `quality-gates` bullet said the uncoverable residue "gains the real watcher's spawn and the worker's thread body" while the delta says the opposite — both modules are covered by real tests | Proposal bullet rewritten to match the delta | `proposal.md` → Modified Capabilities |
| WARNING | design.md | Contracts claimed `read` and `tick` "keep their positions" when `live` is inserted fourth; they shift to fifth and sixth at all eighteen call sites, which is what task 1.6 actually does | Corrected | `design.md` → Contracts |
| WARNING | tasks.md | Five "deferred to" pointers in task 0.4 named a task that does not run the plant (6.5 → 6.6, 1.6 → 1.10, and the `NOSLEEP` row of the first-green table) | All retargeted; the first-green table rebuilt and `NOSLEEP` removed from it, since both its legs are green on `main` | `tasks.md` preamble, 0.4 |
| WARNING | tasks.md | Task 14.1's own inventory said "eleven `testcount` invocations across nine filters"; there are thirteen across eight | Corrected, with the eight named | `tasks.md` 14.1 |
| WARNING | proposal.md, tasks.md | The proposal promised a `SPEC.md` → Testing edit that group 13 did not make, and group 13 made a Stack edit the proposal did not list | Group 13 gains task 13.6 (Testing → Unit-tested modules and View tests); the proposal's Docs list now names Stack too, and the group renumbers to 13.1–13.9 | `tasks.md` 13.6–13.9; `proposal.md` → Impact |
| WARNING | specs/live-updates | The mandatory boundary case "an unreachable Herdr socket" was discharged only in `design.md`'s Test Boundaries table, never in a spec | One clause added: the live tier names no `HerdrCli`, so an unreachable socket cannot degrade, delay, or fail a refresh | `specs/live-updates/spec.md` |
| WARNING | specs/watch-invalidation | `poll_timeout`'s 1ms floor was described as "costs nothing" and "bounds the spin", overstating what it does: ~1000 full draws a second is bounded, not free | Prose corrected — the floor makes a faulty `FsEvents` degrade to a hot pane rather than a hung one | `specs/watch-invalidation/spec.md`; `design.md` → Decisions 15 |
| WARNING | tasks.md (`NOSLEEP`) | The span splitter's docstring claimed "every later part is one test function". A part is a test **plus everything defined after it**, so a sleeping helper below a correct test inherits its verdict — reproduced by the gates reviewer | The docstring and the spec now state the limit, and leg 2/2b are named as what closes it where this change's tests live | `tasks.md` → `NOSLEEP` block; `specs/dashboard-loop/spec.md` |
| SUGGESTION | tasks.md | `NOBLOCK` leg 1's OK line ("names no channel, thread, or blocking wait") overstated what a token grep proves — a receive behind a type alias passes | The claim is now scoped in the spec: leg 1's grep plus the trait contract plus leg 3, with the alias case named as leg 3's | `specs/dashboard-loop/spec.md` |
| SUGGESTION | tasks.md | Task 0.3 invoked `DEPS.sh` without the `WORK` it hard-requires | Corrected to `WORK=$WORK sh $CHECKS/DEPS.sh`, with the guard named | `tasks.md` 0.3 |
| SUGGESTION | tasks.md | Task 6.2 said "three in `GRAPH-SNAP`"; there are four replacements across two labelled edits | Corrected | `tasks.md` 6.2 |
| SUGGESTION | tasks.md | Task 12.1 named six of `openspec/config.yaml`'s nine concentration points | The remaining three are now named with the reason each is trivially satisfied here, so a reviewer confirms rather than infers | `tasks.md` 12.1 |
| SUGGESTION | tasks.md | Task 1.7's Red-when claimed a check ("`NOSLEEP` leg 2 and `NOBLOCK` leg 2 both search `src/lib.rs`?") it does not have | Rewritten to state the gap plainly and point task 12.1's reviewer at it. A Red-when claiming a check it does not have is worse than none | `tasks.md` 1.7 |

| WARNING | tasks.md, design.md, planning-review.md | *(found by re-running every check at the end)* `main` moved during planning: `f9b42e8` ("chore(openspec): put the environment rules in config.yaml instead of every prompt") landed on top of `cdfd656`. Every `OPENSPEC-UNTOUCHED` invocation named `BASE=cdfd656`, so the very first run of the change's most important read-only gate would have reported `openspec/config.yaml` as a stray write inside `openspec/` — a false red that an implementer's cheapest recovery ("re-export `BASE` from the current `HEAD`") silently defeats, because every commit this change makes would then be inside the baseline | `BASE` updated to `f9b42e8` in all fifteen invocations, and task 0.1 now records **why** the sha in this file is not to be trusted over `git rev-parse HEAD`. Confirmed the new commit touched `openspec/config.yaml` and nothing else (`git diff --name-only cdfd656 f9b42e8`), that it adds `context:` prose and no new `rules:` entry, that every rule it states this plan already honours, and that the library test count is unchanged at 673 — so every measured baseline stands | `tasks.md` 0.1 and every `OPENSPEC-UNTOUCHED` invocation; `planning-review.md` → Reviewed Against |

### `SPEC.md` corrections

**None yet.** Planning found no statement in `SPEC.md` that this change makes false and that is
not already scheduled for correction — `SPEC.md` → Data layer → Refresh, Architecture, Module
map, Keys, Degraded states, Stack, and Testing are all *underspecified* rather than wrong about
this change, and task group 13 rewrites each of them. The running totals for prose corrections
made by earlier changes stand at `tui-shell` 9, `list-view` 17, `markdown-viewer` 12,
`detail-view` 10, `tasks-tab` 6; `live-refresh`'s figure is recorded at task 13.9 once the six
edits land, since a rewrite of an underspecified section is not the same as a correction of a
false one and the two should not be conflated in the running total.

The one place `SPEC.md` is arguably *wrong* today — Module map has no `refresh` row, so the
worker has no home in the map at all — is repaired by task 13.2, and is recorded here as a
correction rather than an addition: the map claims to list every module.

## No Remaining Implementation-Blocking Gaps

Confirmed. All ten CRITICALs (eight from the first three review rounds, two from the
verification round) are closed, each verified by re-running the affected check against a
planted violation or by re-reading the repaired artifact. Every WARNING is either repaired
above or recorded in Deferred Non-Blocking Notes with its resolution point.

Mechanical re-checks after every repair:

- `openspec validate live-refresh --strict` → **valid**.
- Scenario ↔ verification matrix: **111 ↔ 111**, set-identical in both directions.
- Test arithmetic: 673 measured + `14 + 11 + 11 + 5 + 8 + 7 + 8 + 3 + 5 + 3 = 75` new = **748**
  library tests; 260 + 26 = **286** under `ui::`. Every per-group RED task enumerates exactly
  its target's worth of new test names, and every `testcount` invocation matches the table.
- Every one of the twenty-six check files extracted from the current `tasks.md` and archives,
  run against `f9b42e8`: twenty green at their stated floors, three red with the exact guard
  message task 0.3 predicts, and `DEPS`/`GRAPH-SNAP` green unedited and red once edited — the
  mirror image that proves the edits are load-bearing.
- Every planted violation reverted; `git status --porcelain` shows only
  `openspec/changes/live-refresh/`.

No unresolved decision requires user input.

## Deferred Non-Blocking Notes

- **`NOSLEEP` leg 1's span limit** — a sleeping helper written below a correct deadline-bounded
  test in `src/changes.rs`, `src/cli.rs`, or `tests/*.rs` inherits that test's verdict. Closed
  for `src/ui/` by leg 2 and for the two seam modules by leg 2b; elsewhere it stays a reviewer's
  job. Recorded in the block's own docstring and in `specs/dashboard-loop`'s scenario, and
  planted deliberately at task 0.4 so the limit is a measured fact rather than a surprise.
- **`NOBLOCK` leg 1's token patterns** — a blocking receive behind a type alias
  (`for _ in rx.iter()` where `rx: SomeAlias`) passes leg 1. Leg 3 catches it in the two seam
  modules, which is where such code would live; in `src/ui/driver.rs` the trait contract is what
  carries it. The OK line is scoped accordingly rather than overclaiming.
- **Worst-case refresh latency of about 800ms** (one `TICK` to notice + `DEBOUNCE` + the CLI's
  200–400ms). Accepted and argued in `design.md` → Risks; `poll_timeout` already removes the
  post-debounce half. Revisited only if `degraded-states` finds it visible in use.
- **`notify` 9.0.0 will declare MSRV exactly 1.88**, this crate's floor. 8.2.0 (MSRV 1.77) is
  adopted; `DEPS` leg 4 prints the at-the-floor set on every run, so the day that changes is
  visible in a check's output rather than in a contributor's build.
- **`openspec/IMPLEMENTATION-ORDER.md`'s `live-refresh` row and the `degraded-states` row's
  scope** — deferred to archive time by task 13.8, as `list-view`, `markdown-viewer`,
  `detail-view`, and `tasks-tab` all deferred the same obligation, because `OPENSPEC-UNTOUCHED`
  forbids writing under `openspec/` outside this change's own directory while it is in flight.
- **The `plugin-build` snapshot legitimately differs between macOS and Linux**, and this change
  widens that difference from one package to four. Both platform sets are in the committed
  fixture and both are regenerated from either host, so nothing here needs a Linux runner —
  recorded because a future reader regenerating on one platform may expect a one-package delta.
