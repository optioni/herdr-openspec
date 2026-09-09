## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/change-enumeration/spec.md`, `specs/change-model/spec.md`, `specs/change-rows/spec.md`,
  `specs/dashboard-loop/spec.md`, `specs/list-filtering/spec.md`, `specs/list-selection/spec.md`,
  `specs/plugin-config/spec.md`, `specs/refresh-worker/spec.md`

The finding pass was delegated to four `planning-reviewer` subagents dispatched simultaneously,
none of which wrote any part of the package, each given one slice: **A** capability coverage,
scenario quality and cross-artifact contradictions; **B** design completeness, test boundaries
and whether each proposed check could fail at all; **C** task alignment, lifecycle discipline
and `parallel-after` independence; **D** factual verification of every empirical claim, by
running the command or reading the source. They reported findings and edited nothing; this
session merged them, repaired the owning artifact, and wrote this log.

They returned **47 findings — 13 CRITICAL, 22 WARNING, 12 SUGGESTION** — and checked 68
empirical claims, of which **11 were false**. Slice D alone found seven false numbers that no
document review could have caught, which is the whole argument for that slice existing.

## Reviewed Against

- This repository HEAD: `1437787` (`docs(list-sections): add the TDD task plan`). Slice B and
  slice D both noted that `tasks.md`'s baseline header named `42e4869`; slice D verified that
  the two commits differ only in `tasks.md` and that `src/`, `Makefile`, `SPEC.md`, `README.md`,
  `AGENTS.md` and `openspec/changes/archive/` are byte-identical between them, so every
  measurement holds at both. The header now names `1437787`.
- Sibling repository HEAD: **Not applicable.** `~/Code/openspec-schemas` supplies the vendored
  `tdd` schema and the orchestration agents, but this change alters neither and depends on no
  contract of theirs.
- Working tree: the four planning artifacts were committed before the review so that
  `OPENSPEC-UNTOUCHED` would pass; `planning-review.md` itself was the only uncommitted file
  while the review ran. All three reviewers that ran commands reported `git status --short`
  empty on exit.

## Gaps Found and Fixed

### CRITICAL

| # | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| 1 | `design.md`, `specs/change-enumeration` | **In file mode the archived section could never be opened.** `refresh::start` returns the inert refresher unless it has both a repository and a CLI, so with no `openspec` binary there is no worker: `Space` would set `refresh.requested`, the loop would hand it to a refresher that records nothing, and the pane would show `v archived (28)` with no rows for the rest of the session — with Decision 7 deliberately rendering no message for that state. Today file mode *does* show archived rows, because `ui::load` resolves them itself, so the change as planned removed the only path that populates them without a worker and broke "never fail closed" in a documented mode | `ui::load` takes an `ArchivedScope` argument; the composition root passes `Full` when the probe resolved no binary and `Names` when it did. The cost rule is restated as "no work wherever a worker exists to do that work later". The rejected alternative — a file-only worker, which would also fix two standing file-mode limitations this change does not own — is recorded with its reason | `design.md` → Decision 13; `specs/dashboard-loop` ADDED startup requirement; `tasks.md` 6.2-6.4 |
| 2 | `design.md`, `specs/change-enumeration`, `tasks.md` | **The falsifiability plant could not fail.** The plan proved "a collapsed section costs no work" by stripping an archived directory's read permission and asserting the `Names` result carried no problem. But every problem a resolved archived change produces lands on `Change::problems`, and the `Names` arm returns no `Change`; `ChangeSet::problems` is built from `list_changes` alone and never merges upward. An implementation that resolved every archived change and discarded the result produced *exactly* the asserted tuple. Slice B further proved no black-box plant can exist, because the spec pins all four `ChangeSet` fields to scope-independent values | Replaced with a counting seam: a `#[cfg(test)]` thread-local path recorder in `schema::read_file` and `tasks::read`, asserting zero recorded paths beneath the archive under `Names` with the `Full` run as positive control. Declared at the bottom of each file, above `mod tests`, on `worker_for_test`'s precedent. The injected-builder alternative is recorded and rejected for the hole it leaves | `design.md` → Decision 16 and Test Strategy; `specs/change-enumeration` (new scenario + the counting-seam SHALL); `tasks.md` 1.4-1.5 |
| 3 | `design.md`, `tasks.md` | **`adopt` and `clamp_selection` write the wrong index space.** `Dashboard::adopt` reselects with `visible().iter().position(…)` and assigns straight to `selected`; `clamp_selection` clamps against `visible_len()`. Once `selected` indexes `targets()`, both are off by one or two — and both sides are `usize`, so nothing in the type system says so and the cursor moves on **every** adopted refresh. Found independently by slices B and D. The one matrix row crossing an adopt asserted `sections.collapsed` and the rendered header, and passes with the defect present | Both sites named and specified: the reselect resolves through `targets()`, `clamp_selection` clamps against `targets().len()` for both callers. A new scenario asserts `selected_change()` names the **same change** across a `Files` and a `Merged` adoption, which is the assertion that goes red against the defect | `design.md` → Decision 14; `specs/list-selection` (the index-space SHALL + `A refresh keeps the cursor on the same change`); `tasks.md` 3.3 |
| 4 | `specs/change-rows` | Four live requirements still specified `RowKind::Separator` and were in no delta — including the palette style table `tasks.md` 5.5 edits, and the scenario asserting `kind ∈ {Problem, Separator, Message}` | All four added as MODIFIED blocks with `Section` substituted | `specs/change-rows` (four MODIFIED requirements) |
| 5 | `specs/change-rows` | **The empty-state rule became undecidable.** The live rule picks `No active changes` when "no active change is visible but at least one archived change is", and a collapsed section's changes are not visible — so a repository with no active changes and 28 archived ones would render `No changes yet` above `> archived (28)`, and a collapsed populated active section would render `> active (9)` immediately followed by `No active changes` | The three message rows are keyed on section **counts**, not on visible rows, stated in both the spec and the design, with three new scenarios covering the collapsed-archive, collapsed-active, and no-active-with-archived cases | `specs/change-rows` (MODIFIED empty-state requirement); `design.md` → Decision 10; `tasks.md` 5.4 |
| 6 | `specs/dashboard-loop` | `Dashboard` is specified as carrying "exactly **thirteen** fields"; `sections` is a fourteenth. Two bound scenarios broke, including the compile-time companion that destructures all thirteen | Converted to REMOVED + ADDED under a renamed title (a MODIFIED block cannot rename the bound scenario), count corrected, `Sections` added to the swept type list | `specs/dashboard-loop` REMOVED + ADDED `Dashboard carries fourteen fields…` |
| 7 | `specs/dashboard-loop`, `specs/refresh-worker`, `specs/list-selection` | `Refresher::request`'s new parameter had no owning requirement, both deltas named the wrong loop step, and the **second** `request` call site — the `fs.drain()` → `watch::invalidate` path — was never told what scope to carry | The loop requirement is MODIFIED so steps 2 **and** 3 pass `dashboard.archived_scope()`, with the reason a watch event must resolve an open archive; both "step 3" references corrected to "step 2" | `specs/dashboard-loop` (MODIFIED loop requirement); `specs/refresh-worker`; `specs/list-selection` |
| 8 | `specs/list-filtering` | The match requirement was unmodified and three of its scenarios became false — one asserts a `-- archived ---` row, one asserts `selected` 0 addresses the first visible change, one asserts a clamp target of 1 that is now 3 | Added as a MODIFIED block with the emission sentence restated and all three scenarios re-indexed | `specs/list-filtering` (MODIFIED match requirement) |
| 9 | `specs/list-selection` | The viewport requirement was unmodified: its prose named "the separator" and its three rendering scenarios' arithmetic was wrong once the row vector grew by a header | Added as a MODIFIED block. Every asserted row **name** is preserved by advancing `selected` by one — `viewport(31, 21, 16) = 13` still lands on `change-12` — so the scenarios keep their meaning rather than being re-derived | `specs/list-selection` (MODIFIED viewport requirement) |
| 10 | `specs/change-rows` | The delta's own headline scenario was self-contradictory: it kept "row 2 spells `> add-token-refresh`" while adding "interior row 1 is the active section header", mixed buffer-row and interior-row conventions inside one file, left the `>` marker on a change while `selected` 0 now addresses a header, and miscounted the blank range | One convention fixed and stated (buffer rows, 0-based), every affected index shifted, `selected` stated in every scenario asserting a marker, and the marker-on-header case asserted explicitly | `specs/change-rows` (row-numbering paragraph + five scenarios) |
| 11 | `specs/change-model`, `design.md`, `tasks.md` | `conformance::assert_invariants` was given two incompatible signatures: the live spec binds it to `&Change` as mechanism 2's `E0027` guard, while the delta called it "on both results" — `ChangeSet` values | A **new** `assert_set_invariants(&ChangeSet)` beside it; `assert_invariants` untouched. The design also corrects which mechanism actually makes a new `ChangeSet` field a compile error: `gate-mech1.py`, not the destructuring guard | `specs/change-model`; `design.md` → Boundaries and Contracts; `tasks.md` 1.3 |
| 12 | `design.md` | **Tier 4 claimed a documentation binding that does not exist.** `doc_contract.rs` binds the module map, tested-modules list, MSRV, gate programs, manifest transcription, injected context and worker-thread count; `degraded_coverage.rs` binds the degraded-states table. None reads the List view, Keys, `config.toml`, README table or list-region passages group 9 rewrites, so `tasks.md` 9.5's check passed whether or not group 9 happened | The false claim is replaced by an explicit statement of what those tests do and do not bind, and group 9's evidence is now a re-read of each passage with its own `grep`, with the test run demoted to a no-regression check | `design.md` → Test Strategy; `tasks.md` 9.1, 9.6 |
| 13 | `proposal.md` | `change-model` was not in the Capabilities list although `ChangeSet` gains a field. Found by this session before the review and repaired then; the reviewers confirmed the list now matches the eight delta directories exactly | Added to Modified Capabilities with its reason | `proposal.md` → Capabilities |

### WARNING

| # | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| 14 | `design.md` | The outer-loop tier was declined on a structural claim that is false: the exit-status-3 guard is on `ui::run`, not `ui::run_wired`, and `src/ui/mod.rs`'s `mod wiring` already holds 26 tests driving the composition root with a real watcher, worker and spawned stub programs. That tier would have caught CRITICAL 1 | The change now **takes one** wiring scenario — the file-mode one — and the paragraph says plainly that the tier exists and everything else is declined on cost | `design.md` → Test Strategy; `tasks.md` 6.4 |
| 15 | `design.md` | The Test Boundaries table said no test spawns a process, touches the Herdr socket, starts a watcher or runs a worker — all contradicted by `tasks.md` 6.3's own wiring verification. Its clock row was false for the twelve `refresh::tests::` rows, which `recv_timeout` at ten sites, and those rows were labelled tier "unit" while driving a real thread and a real filesystem walk | A wiring row added naming the spawned stub programs, real watcher and real worker; the clock row reworded to distinguish a deadlock guard from a timing assertion; the twelve rows relabelled scratch tree | `design.md` → Test Boundaries and the matrix |
| 16 | `design.md`, `proposal.md` | `Boundaries` and `Impact` omitted `src/ui/mod.rs`, which holds the whole subject of group 6, and `src/ui/detail.rs`, whose seven `from_files` test call sites become compile errors | Both added, with the call-site census recorded | `design.md` → Boundaries; `proposal.md` → Impact; `tasks.md` check H and 1.6 |
| 17 | `design.md`, `specs/change-rows` | Decision 10's premise — "with a query the tier is always resolved" — is false for one cycle, and no scenario covered the window where the archive is open, unresolved and under a query | Premise corrected and a scenario added for a query typed against an unresolved archive | `design.md` → Decision 10; `specs/change-rows` |
| 18 | `design.md`, `specs/change-rows` | `RowKind::Item { index }` and the selection marker share one counter at `src/ui/list.rs:430-493`, and after this change they are two different numbers | Split into two counters, specified as a decision and a task | `design.md` → Decision 15; `tasks.md` 5.3 |
| 19 | `proposal.md`, `design.md`, `specs/change-enumeration` | **"22 archived changes on disk, five in the pane, seventeen invisible" is false** — there are 28, so 23 are hidden, and `tasks.md`'s own check F already recorded 28. `proposal.md` also contradicted itself, illustrating `> archived (21)` in one place and `(22)` in another, and "four times the archived-tier file work" is 5.6× | Re-measured once with the command recorded; every prose number now reads 28 / five / 23 / 5.6× and the illustration uses this repository's real counts. Fixture values inside spec scenarios stay at 22, which is what a fixture is | `proposal.md`; `design.md` → Context |
| 20 | `proposal.md` | The date-grouping deferral claimed "no archived name yet exceeds the narrow layout's 19-column name field (longest is 18)". The longest is **19** (`markdown-constructs`), and 19 is the **wide** layout's field; the narrow one is 39 | Restated accurately, which leaves the deferral's conclusion intact but its evidence true | `proposal.md` → Non-Goals |
| 21 | `tasks.md` | Task 3.5's contract gate named `src/ui/driver.rs`, which has **zero** `selected_change()` references, and omitted `src/ui/view.rs:56` and `:120` — the two production sites that consume the `None` a header cursor returns | File list corrected against the grep, with the census recorded | `tasks.md` 3.6 |
| 22 | `tasks.md`, `design.md` | Task 3.4's re-index searched only for `selected`, missing 59 tests that drive the cursor with `Action::Next`/`Prev` (10 of them in `src/ui/view.rs`); and `design.md` → Risks claimed the re-index was its own group running *before* any new behaviour, which is impossible since `targets()` must exist first | Search widened to both means and both files; the Risks line corrected to describe a labelled CHECK task inside the group that introduces `targets()` | `tasks.md` 3.5; `design.md` → Risks |
| 23 | `tasks.md` | The one-added-header-row shift was scoped to two tests; at least a dozen positional assertions move across three files, and two landed tests encode `separator` in their own names | Task rewritten naming the file set, the measured count, and both tests — to be **rewritten, not deleted** | `tasks.md` 5.6 |
| 24 | `tasks.md` | Task 7.3 writes `notes/gate-floors.md` under `openspec/` and 7.4 immediately runs `make gates`, which `OPENSPEC-UNTOUCHED` fails on for any untracked file there — the same trap this session's own first baseline run hit | `git add` folded into 7.3, and into 8.2 for any note the review produces | `tasks.md` 7.3, 8.2 |
| 25 | `tasks.md` | Group 9 was `operational` but ran CHANGE → CHECK with no VERIFY, inverting the required lifecycle | Restructured to CHECK → CHANGE → VERIFY, with the leading CHECK recording each passage's current text | `tasks.md` group 9 |
| 26 | `tasks.md`, `README.md` | Task 9.3 added "no effect on the list" while leaving the same cell's "listed below the separator" standing, which group 5 also makes false — and its own claim "nothing is added" was then wrong | The whole cell is rewritten, and the task says so | `tasks.md` 9.4 |
| 27 | `tasks.md` | Two amended `change-model` scenarios carried new `archived_total` assertions that no RED task named, so both clauses could land unasserted | Both scenario names added to the RED task | `tasks.md` 1.1 |
| 28 | `tasks.md` | The ordering audit stopped at group 7, and three of its five stated edges were justified by reasons that do not order them | Audit extended to groups 8-10 and the real serialising reason stated: every group closes on a whole-crate `cargo test`, so no pair can fail attributably. Group 9's specific reason for not being `parallel-after: 0` is recorded | `tasks.md` preamble |
| 29 | `tasks.md` | Check C called `gate_controls` 55 tests; it has **4** (55 was `doc_contract`'s, now 56), and check E's baseline omitted three test binaries | Both corrected against the run | `tasks.md` checks C and E |
| 30 | `tasks.md` | **The baseline is not reliably green.** Three of four clean-tree runs at HEAD were red on `ui::tests::wiring` — `g_focuses_the_agent_the_launch_started`, recorded twice already, and `every_launch_failure_renders_as_a_leading_row`, previously unrecorded. Task 10.5's "no test count lower than the baseline" fires on both | Both names and the observed rate recorded in check E and in the design's Risks, so an apply session recognises the red instead of debugging its own work. Repairing them stays out of scope, as `markdown-constructs` already concluded for the first | `tasks.md` check E, 10.5; `design.md` → Risks |
| 31 | `specs/change-rows` | The ADDED archived requirement dropped the landed scenario `An archived change carries a badge in the same column as an active one`, losing the badge-column-equality proof and the assertion that the row above the archived rows reserves no badge column — the property the new `Section` row inherits | Restated in the ADDED requirement, retargeted at the section header | `specs/change-rows` |
| 32 | `specs/plugin-config` | `The configuration's fallbacks reach the pane` was unmodified and asserts "`ui::load` consults `config.archived_count` and nothing else", false in both halves after this change, with a trailing assertion that is no longer observable | Added as a MODIFIED block; the trailing assertion becomes a byte-identical-buffer check across three `archived_count` values, which is the observable form of "the value reaches nothing" | `specs/plugin-config` |
| 33 | `specs/change-rows` | Wrong pad count: `  v active (9)` is 14 display columns at a 38-column interior, so 24 spaces follow, not 25 | Corrected, with the column count shown | `specs/change-rows` |
| 34 | `specs/list-filtering`, `specs/change-rows` | Four scenarios asserted a `>` marker without stating `selected`, and two pure-`rows` degradation scenarios asserted "the row is exactly …" while `rows` now returns a header first | `selected` stated in every WHEN that asserts a marker, and the degradation scenarios name which returned row they assert (`rows()[1]`) | `specs/list-filtering`, `specs/change-rows` |
| 35 | `design.md` | Decision 9's last-wins rule leaves one case unacknowledged: a remembered `(All, Full)` still fires after the reader folds, because folding issues no request | One line accepting the single wasted cycle rather than adding a mechanism | `design.md` → Decision 9 |

Six further WARNINGs were duplicates across slices — the file-mode hole (B), the `adopt` index space (B and D), the archive count (A and D), the `assert_invariants` signature (A, B and D), the `notes/gate-floors.md` staging (C and D), and the doc-location details (C and D) — and are recorded once above, at the row that repairs them.

### SUGGESTION

All twelve were taken except one, which is accepted with a reason:

- **Taken:** the `0o000` fixture mode named explicitly; `AGENTS.md` named instead of the
  `CLAUDE.md` symlink; both `SPEC.md` sections named in the documentation task; a "state whether
  a refactor was needed" clause on every behaviour group's closing task; lifecycle labels on the
  mid-group tasks and `4.4` relabelled `CHECK`; the `ui::app::tests::` halves of two scenarios
  moved to the group where `section_open` lands; two scenarios that were covered in substance
  but named by no task now named; the persistence gate bound to a `testutil::snapshot` instead
  of being an attestation; the stale-`Full` cycle acknowledged; `"sixteen other actions"`
  corrected to seventeen; and the two `## Purpose` paragraphs that `openspec archive` cannot
  update — a delta carries requirements, not a Purpose — given their own post-archive task.
- **Accepted without change:** slice D noted that "12 construction sites" is really a count of
  `grep` matching lines rather than of construction sites. It is, and the number is right (slice
  C confirmed 12 independently). The task no longer rests on it: `gate-mech1.py` is what enforces
  the property, and the count is context.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL is repaired in the artifact that owns it, every WARNING is repaired
or recorded above with its reason, and `openspec validate list-sections --strict` reports the
change valid. The verification matrix was regenerated after the spec repairs and carries one row
for each of the **132** spec scenarios (it carried 76 before the review).

One decision is recorded rather than resolved, and it is deliberate: **the file-only refresh
worker** (design.md → Decision 13) is the better long-term shape and would also fix two standing
file-mode limitations this change does not own — `r` does nothing, and watcher events do
nothing, because there is no worker to receive them. This change takes the smaller fix instead
and says why. That is a scope judgement, not an open question, and it needs no user input; a
future change that wants live refresh in file mode will find the argument here.

## Deferred Non-Blocking Notes

- **The two `ui::tests::wiring` intermittents** are recorded in `tasks.md` check E and in
  `design.md` → Risks, with their observed failure rate. Their resolution point is a future
  change; `markdown-constructs` already reached that conclusion for the first of them, and this
  review's contribution is naming the second, which had never been recorded.
- **A date grouping nested under `archived`** stays deferred, as `proposal.md` states, and the
  section model carries a `key` and a `depth` so that a later change can add one without
  reworking the collapse state, the cursor index, or the force-open rule.
- **Renaming `Role::ListSeparator` to `Role::ListSection`** is deferred with its reason in
  `design.md` → Decision 3: a `RowKind` names what a row is and a palette `Role` names how it
  looks, `view-palette` already records `AgentBadge(Unknown)` sharing this role, and the rename
  would restate four of that capability's requirements for no change to a rendered cell.

## Repairs Made During Implementation

| # | Source Artifact | Problem | Repair | Found by |
|---|---|---|---|---|
| I1 | `specs/list-selection/spec.md` | Scenario *A refresh keeps the cursor on the same change* opened on "three active changes" while its own `selected` 4, "`selected` is 5" and "`visible()` position of 3" clauses only hold for **two**. With three active, `targets()` index 4 is the archived header, not `add-auth`. | Precondition corrected to "two active changes"; every other clause of the scenario was already right and is unchanged. | Group 3's implementer, building the test against the scenario |
| I2 | `specs/change-rows/spec.md` | Scenario *An archived row drops the progress cell, then the date, as the width falls* named the change row `rows()[1]`, contradicting the same file's own emission requirement: an active section whose count is zero emits a `No active changes` message row *before* the archived header, so the change row is `rows()[2]`. The landed `the_four_message_states_are_distinct` confirms the message/header/item order. | Index corrected to `rows()[2]`, with the two preceding rows named so the scenario states its own row order. The width assertions themselves were right and are unchanged. | Group 5's implementer, building the test against the scenario |
| I3 | `specs/dashboard-loop/spec.md` | Scenario *Startup counts the archive without resolving it* illustrated the render as `  > archived (7)`, on an archive-only fixture where the archived header is target 0 and so carries the cursor — the real row is `> > archived (7)`, marker and collapsed glyph colliding per Decision 4 — and omitted the `No active changes` row that an empty active section puts above it. | Render clause restated with both rows and the correct marker. The scenario's `archived_total`, inertness and `needs_archived_refresh()` clauses were right and are unchanged. | Group 6's implementer, building the test against the scenario |

**Reviewed Against, updated:** implementation proceeds from `74a0b2e`, which differs from the
reviewed `1437787` only by this change's own planning artifacts (`git show --stat 74a0b2e`
touches nothing outside `openspec/changes/list-sections/`). No source, `Makefile`, `SPEC.md`,
`README.md`, `AGENTS.md`, or archive file moved, so every measurement in `tasks.md`'s check
table still holds.

**Three scenarios stated numbers that did not add up** (I1, I2, I3 above), each found by the
implementer building a test against it, in three different spec files and by three different
mechanisms — a precondition inconsistent with its own conclusions, a row index contradicting
its own file's emission requirement, and an illustrative render string written without the
selection marker. None was caught by the four-slice planning review, which checked 68 empirical
claims against the repository but could not check a scenario's internal arithmetic against
behaviour that did not exist yet. That is the standing limit of a pre-implementation review, and
it is why each repair is logged here rather than silently absorbed.

**The wiring-tier intermittents are environmental, not code.** Measured at group 3: the same
commit runs `ui::tests::` in **4.1s, green**, inside a `/tmp` git worktree and in **33s with
six failures** in `/Users/juusopiikkila/Code/herdr-openspec`, and the pre-change baseline
`74a0b2e` behaves identically in that worktree (4.07s, green). The main checkout single-threaded
is a clean 1137/1137. So the intermittents recorded in `tasks.md` check E belong to the checkout,
not to any commit; verify a suspected regression with `cargo test --lib -- --test-threads=1`
before treating a wiring failure as one.

## Change Review (group 8)

An independent `outside-in-tdd-reviewer` was dispatched against the artifacts and the diff
`74a0b2e..08e0804`, with the five concentration points `tasks.md` 8.1 names. It returned
**3 CRITICAL, 2 WARNING, 3 SUGGESTION**. Every CRITICAL is fixed; both WARNINGs are resolved;
all three SUGGESTIONs are taken.

| # | Severity | Finding | Resolution |
|---|---|---|---|
| C1 | CRITICAL | **Task 6.3 was never implemented although marked `[x]`.** `run_wired` still passed `ArchivedScope::Full` unconditionally, under a comment describing the fix that had not been applied — so every startup in normal mode resolved the whole archive, the cost `proposal.md`'s "a collapsed section costs no work, not just no rows" forbids. | Implemented: `find_repo` resolves the root, `start_collaborators` runs **before** `load`, and `load` takes `Full` only when `collaborators.file_mode`. The stale comment is gone. |
| C2 | CRITICAL | **The wiring control did not discriminate.** It asserted `changes.archived.is_empty()` on the post-`run_loop` dashboard, where emptiness comes from `adopt` running under `archived_scope()` = `Names`, not from `load`'s scope — so it passed with C1's shim in place. | Rewritten to assert on `load`'s own file work, through group 1's thread-local read recorders (`load` runs on the test's thread; the worker does not). Verified: with the shim reinstated the control fails, naming the eight archive files `load` opened. |
| C3 | CRITICAL | **`make coverage`, and so `make check`, was red at HEAD.** Three `covers` ranges in `tests/degraded-coverage.toml` had drifted onto unrelated code as this change moved lines in `src/ui/view.rs`, `src/ui/app.rs` and `src/refresh.rs`. | All four repaired (the `startup_cwd` row shifted again while fixing C1). `make coverage` green: production **96.25%** (4259/4425) against the 96% floor, no floor lowered and no exclusion added. |
| W1 | WARNING | **Nothing tested that either `request` call site carried `dashboard.archived_scope()`** — `RecordingRefresher` discarded the argument, and the reviewer's mutation to a constant `Names` left the whole suite green. | `RecordingRefresher` now records the scope on a second vector paired by position, so the ~30 landed `requests()` assertions keep their shape. New test `both_request_call_sites_carry_the_dashboards_archived_scope` drives the loop collapsed and open and requires the scopes to differ. Verified: the reviewer's constant-`Names` mutation now fails it. |
| W2 | WARNING | The comment above C1's shim described an implementation not in the tree, which is what created the impression that `load` performed a second `find_repo` walk. | Gone with C1. The second walk now genuinely exists and is the cheap filesystem kind the comment describes. |
| S1 | SUGGESTION | The doc comment on `an_archived_row_drops_the_progress_cell_then_the_date_as_the_width_falls` still argued against the `rows()[1]` claim that repair I2 had already corrected. | Comment rewritten to the repaired index. The test keeps locating its row by `RowKind`, which stays correct at every width including those where the row degrades to the empty string — robustness, not evasion. |
| S2 | SUGGESTION | Group 1 kept two `changes::` tests as scratch-tree tests where `design.md`'s Test Strategy table specifies hand-built `ChangeSet` unit tests. | Accepted deliberately: the scratch-tree form covers `from_files`' real enumeration, which a hand-built set cannot, and the reviewer found no coverage lost. Recorded here rather than amending the table. |
| S3 | SUGGESTION | `change-enumeration`'s REMOVED-requirement Reason still read "twenty-two / five / seventeen", the numbers corrected to 28 / five / 23 everywhere else. | Restated as twenty-eight / five / twenty-three. |

### Concentration points: all five clean

The reviewer answered each with the command it ran, not an assurance.

| Point | Verdict | How it was checked |
|---|---|---|
| The read recorder distinguishes the two scopes | **Yes**, verified in both directions | Deleting the recorder call in `schema::read_file` makes the `Full` positive control fail with `must open at least one .openspec.yaml per archived change, got 0` |
| The 3.5 re-index weakened no assertion | **No weakening** | Every removed `assert` hunk in `app.rs`/`list.rs`/`view.rs`/`driver.rs` has a shifted replacement with a stated reason; `rows.len()` went 3→4 rather than being dropped; no test computes its expectation from `targets()` |
| No write to `selected` uses a `visible()` index | **None** | The six production writes — `:330`, `:344`, `:473`, `:748`, `:839`, `:841` — are all in the `targets()` space |
| `archived_total` and `archived.len()` cannot disagree | **Cannot** | All four production `ChangeSet` constructions read; `adopt` replaces the set wholesale and derives neither field |
| No new width arithmetic bypasses `layout::columns` | **None** | `section_row_text` is the only new grammar site and reaches width only through `truncate_columns`/`pad_or_truncate_right`; the degradation test asserts `columns(&header) == width` at seven widths |

`changes::fixture::set`'s narrowing blocks nothing: `archived_total` is `pub`, so the
unresolved-archive tests assign it after construction and `changes::tests` writes the literal
directly. No builder was needed and none was added.

**The pattern C1 and C2 form is the durable lesson.** A task was marked complete, and its own
report quoted replacement lines that were not in the tree, because the test written to prove
the behaviour asserted on a value that a later step in the same function overwrites. Neither
`cargo test` nor `make gates` could see it; only reading the tree could. Between groups this
change gated on the suite and the hygiene gates but **not** on `make coverage`, which is what
let C3 survive seven groups — the final gate is `make check`, and running less than that
between groups is what made three CRITICALs reachable at all.
