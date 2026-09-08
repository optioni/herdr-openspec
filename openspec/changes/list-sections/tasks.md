<!-- Planning-time baseline: commit `1437787` on `main`. `42e4869` differs only by tasks.md;
     `src/`, `Makefile`, `SPEC.md`, `README.md`, `AGENTS.md` and `openspec/changes/archive/`
     are byte-identical between the two, so every measurement below holds at both. -->

**Groups 1 through 7 are sequential, and none is marked `parallel-after`.** The reason is not
the compile chain an earlier draft of this line gave — that chain is real for 1→2 and 1→3 but
does **not** order 2→3 (`grep -n "Refresher\|refresh::" src/ui/app.rs` is empty at HEAD),
4→5 (group 4 writes `src/ui/app.rs`, group 5 writes `src/ui/list.rs` and `src/ui/view.rs`;
both merely *read* `Dashboard::sections`, which is the condition for independence), or 5→6.
The reason that does order every pair is the third criterion: **a failure must stay
attributable**, and every group here closes on `cargo test --all-features`, one whole-crate
compile and one whole-suite run. Two concurrent implementers in this tree would each see the
other's half-written file as their own red. This repository does dispatch code groups in
parallel where that criterion holds — `archive/2026-09-08-seam-resilience/tasks.md` groups 5,
6 and 7 carry `parallel-after` — so the sequential answer is stated rather than assumed.

Groups 8, 9 and 10 were examined too. **Group 9 (Documentation) is not marked
`parallel-after: 0`**, though it writes only `SPEC.md`, `README.md` and `AGENTS.md`, which no
code group writes: task 6.4 re-reads the `SPEC.md` → Resolution chain passage that 9.2
rewrites, so running them concurrently would have 6.4 read a moving target. Groups 8 and 10
are whole-change gates by definition and cannot precede the work they gate.

**Planning-time checks, run at `1437787`:**

| Check | Command | Result at HEAD | Why that result |
|---|---|---|---|
| A (RED) | `grep -rn "ArchivedScope\|archived_total\|SectionKey\|ToggleSection\|RowKind::Section" src/ tests/` | no output, **exit 1** | The behaviour does not exist yet; every behaviour group below turns part of this red |
| B (green + negative control) | `SCAN_MIN=135 /bin/sh scripts/gates/nodefault-ui.sh` | `150 literal/pattern spans scanned (>= 135)`, **exit 0** | The floor group 7 moves. Negative control: `SCAN_MIN=100000 …` prints `half B found only 150 … the scan is vacuous`, **exit 1** |
| C (green + negative control) | `make gates` | all gates OK, **exit 0** — but only once the change's own artifacts are committed | `OPENSPEC-UNTOUCHED` fails on any untracked file under `openspec/`, which is why group 7's note file must be `git add`ed before its gate runs. Negative control: `cargo test --test gate_controls` (**4** tests) plants a real defect per gate script and requires each to exit non-zero |
| D (must-change) | `grep -rn "RowKind::Separator" src/ tests/ \| wc -l` | **8** | Must be `0` after group 5 |
| E (baseline) | `make check` | **exit 0**: 1 121 unit tests, then 21 + 9 + 19 + 10 + 4 + 56 across the integration binaries; total coverage 96.07%, production 96.19% | Two caveats, both measured. (i) `gate_controls_catch_their_plants` asserts the repository tree is unmodified across a gate run, so editing a file while `make check` runs turns it red — that is the test working. (ii) **Two `ui::tests::wiring` tests are known intermittents**: `g_focuses_the_agent_the_launch_started` (recorded twice already, commit `3c23a1b`) and `every_launch_failure_renders_as_a_leading_row` (unrecorded before this change); three of four clean-tree runs at HEAD were red. Recognise these reds rather than debugging them; repairing them is out of scope |
| F (fixture size) | `ls openspec/changes/archive \| wc -l` | **28** | This repository's own archive — the pane shows five of them today |
| G (headroom) | `cargo llvm-cov --fail-under-lines 80` plus `scripts/coverage-prod.py` | production **96.19%** (4 120 / 4 283) against a **96%** floor | Roughly eight uncovered production lines of slack. `AGENTS.md`'s "does not fire until roughly 44%" describes the *total* floor, not this one |
| H (call-site census) | `grep -rc "from_files(" src/*.rs src/ui/*.rs` | `src/changes.rs` 22, `src/ui/detail.rs` **7**, `src/refresh.rs` 1, `src/ui/mod.rs` 2 | `src/ui/detail.rs`'s seven test call sites become compile errors in group 1; named so nobody meets them by surprise |

## 1. `ArchivedScope`, `archived_total`, and the un-capped archive
<!-- kind: behavior -->

- [x] 1.1 RED: Write failing tests in `src/changes.rs` for `The full archive is enumerated and counted under either scope`, `An unresolvable archived change is a Change problem under Full and absent under Names`, `An empty archive counts zero under either scope`, `A surviving scenario names its scope`, `An unreadable archive counts nothing and reports once, under either scope`, `An archived change keeps its file-derived values when the CLI arrives`, `A repository-level failure is recorded on the set, not on a change`, and `The two archived_total invariants hold under either scope`. The unresolvable-change fixture uses mode `0o000` — read **and** execute removed, the house idiom at `src/changes.rs:3241`, `:3268`, `:3579` — because stripping read alone leaves `open()` working beneath the directory and the `Full` control would record no problem.
- [x] 1.2 GREEN: Add `changes::ArchivedScope` and `ChangeSet::archived_total`, drop `entries.truncate(archived_count)` from `archived_entries`, and thread the scope through `archived_entries`, `list_changes`, and `from_files` so `Names` builds no archived `Change` at all. Verify with `cargo test changes::tests::`.
- [x] 1.3 GREEN: Carry `archived_total` through `changes::merge` untouched and set it to `0` in `changes::empty_set`. Add `conformance::assert_set_invariants(&ChangeSet)` as a **new** function beside `assert_invariants(&Change)`, destructuring `ChangeSet` exhaustively; leave `assert_invariants`' signature and every landed call site alone, per design.md → Boundaries.
- [x] 1.4 GREEN: Add the `#[cfg(test)]` thread-local read recorder to `schema::read_file` and `tasks::read`, declared at the **bottom** of each file directly above `mod tests` (design.md → Decision 16). Then write `A collapsed archive opens no file beneath an archived change` against it.
- [x] 1.5 CHECK: Confirm the recorder is a real plant, not a decoration: the `Names` leg must record zero paths beneath `openspec/changes/archive/` and the `Full` leg at least one per archived change. Delete the recorder's call in `schema::read_file` and confirm the `Full` control fails; restore it and confirm both legs pass.
- [x] 1.6 CHECK: Fix the seven `changes::from_files(root, 5)` call sites in `src/ui/detail.rs`'s tests (check H) and the two in `src/ui/mod.rs`. These are compile errors, not behaviour changes; each takes the scope its test intends.
- [x] 1.7 CHECK: Contract gate — `grep -rn "ChangeSet {" src/ tests/ | wc -l` reports **12** sites, all in `src/changes.rs`; confirm `python3 scripts/gates/gate-mech1.py` still reports no rest pattern and no functional update, which is what actually makes a new `ChangeSet` field a compile error at every one.
- [x] 1.8 CHECK: Persistence gate — with `testutil::snapshot` taken either side of a full `from_files` under both scopes, confirm the repository tree is byte-identical and `HERDR_PLUGIN_STATE_DIR` still holds exactly `agent-names.toml`. That is the persistence claim being made; an attestation with no second site would not be.
- [x] 1.9 REFACTOR: Clean up while green, or state that no refactor was needed.
- [x] 1.10 Run `cargo test --all-features` — no regressions beyond check E's two known intermittents.

## 2. The refresh request carries the archived scope
<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests in `src/refresh.rs` for `The scope on the request is the scope the file tier runs under` and `drain_and_fold unions selections and takes the last scope`, and update `A refresh outstanding does not queue further selections` and `A forced refresh outstanding behind a narrower one is not lost` to the two-field request.
- [x] 2.2 GREEN: Add `refresh::Request { selection, archived }`, change `Refresher::request` to take both, drop `archived_count` from `refresh::start` and `worker_for_test`, and pass `request.archived` to `from_files`. Verify with `cargo test refresh::tests::`.
- [x] 2.3 GREEN: Make `drain_and_fold` fold a `Request` — `Selection::union` for the selection, last-wins for the scope (design.md → Decision 9) — and give the remembered `Selection::All` the most recent suppressed scope. Verify single-threaded against a receiver whose sender queued two requests and was dropped.
- [x] 2.4 CHECK: Contract gate — confirm neither `Refresher` nor `RefreshResult` names `CliChanges`, `OpenspecCli`, or `from_cli`, by running `make gates` and reading `NOCLI-SHELL` and `NOBLOCK` OK.
- [x] 2.5 Run `cargo test --all-features` — no regressions, and state whether a refactor was needed.

## 3. The section model and the re-indexed cursor
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests in `src/ui/app.rs` for `The first target is selected on startup at both widths`, `j, k, and the arrows move the cursor over headers and changes`, `The cursor clamps at both ends rather than wrapping`, `The cursor crosses the archived header into the archived rows`, `A collapsed section's changes are neither visible nor addressable`, `Navigation over an empty visible list is inert`, `Enter on a section header does nothing`, and `A refresh keeps the cursor on the same change`.
- [ ] 3.2 GREEN: Add `SectionKey`, `Sections { collapsed: BTreeSet<SectionKey> }`, and `Target`, put `sections` on `Dashboard`, and implement `section_open`, `targets`, `visible`, `visible_len`, `selected_change`, and `archived_scope` (design.md → Decisions 2, 5, 8).
- [ ] 3.3 GREEN: Move **every** write to `selected` into the `targets()` index space: `clamp_selection` clamps against `targets().len()` for both callers, and `Dashboard::adopt`'s reselect resolves the found `visible()` position through `targets()` (design.md → Decision 14). `A refresh keeps the cursor on the same change` is the assertion that goes red against the off-by-headers form; a render assertion does not.
- [ ] 3.4 GREEN: Make `OpenDetail` inert on a `Target::Section`, and confirm the four launch keys are already inert there through `selected_change()` returning `None` and `launch::decide` answering `Decision::Nothing`, with no new rule.
- [ ] 3.5 CHECK: Re-index every landed test that addresses the *n*th change, in **both** files and by **both** means: `grep -rnE "selected|Action::(Next|Prev)" src/ui/*.rs` — 296 `selected` mentions, and 59 action-driven cursor moves (49 in `src/ui/app.rs`, 10 in `src/ui/view.rs`, e.g. `src/ui/view.rs:2180-2181`). Update the assertions to the value `targets()` now produces; do not re-point an assertion at whatever the code happens to return.
- [ ] 3.6 CHECK: Contract gate — re-inspect `selected_change()`'s production call sites, which are `src/ui/view.rs:56`, `src/ui/view.rs:120`, and `src/ui/detail.rs:1663` (`grep -rn "selected_change" src/` also reports 19 in `src/ui/app.rs`, 1 test-function name in `src/ui/mod.rs`, and **0** in `src/ui/driver.rs`). Confirm each handles the `None` a section cursor now returns.
- [ ] 3.7 Run `cargo test --all-features` — no regressions, and state whether a refactor was needed.

## 4. `Space`, `ToggleSection`, and the refresh it triggers
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests for `Space maps to ToggleSection outside filter mode and types inside it`, `Space on a header folds and unfolds that section`, `Space inside a section folds it and moves the cursor to its header`, `An empty list makes Space inert`, `Opening an unresolved archive requests a refresh`, `A refresh does not undo a fold`, `Space types into the query rather than folding a section`, `A non-empty query forces every section open`'s three scenarios, and `The first character of a query requests the archive it needs`.
- [ ] 4.2 GREEN: Add `Action::ToggleSection` — the eighteenth variant — map `KeyCode::Char(' ')` with no modifiers to it while `filtering` is false, and implement the toggle in `apply` so it moves `selected` to that section's header and changes nothing else.
- [ ] 4.3 GREEN: Implement `needs_archived_refresh()` and set `refresh.requested` from it after **every** action in `apply`, not on a named subset (design.md → Decision 6). Verify the predicate self-clears once `archived.len() == archived_total`.
- [ ] 4.4 CHECK: Confirm `Dashboard::adopt` does not touch `sections`, so a live update never reopens what the reader folded.
- [ ] 4.5 CHECK: Update the hand-written `variants` array in `ui::app::tests::no_action_mutates_changes` to eighteen, and confirm the exhaustive `match` and the array enumerate the same set.
- [ ] 4.6 Run `cargo test --all-features` — no regressions, and state whether a refactor was needed.

## 5. Section rows and their styling
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing tests in `src/ui/list.rs` and `src/ui/view.rs` for `The section header and archived rows render at both mandated widths`, `A collapsed archived section shows its count and no rows`, `An expanded but unresolved archived section shows its header alone`, `An archived change carries a badge in the same column as an active one`, `A query against an unresolved archive counts from archived_total`, `A section header degrades by truncation at every width`, `No archived changes means no archived header`, `An archived row drops the progress cell, then the date, as the width falls`, `A collapsed but non-empty archive is not "no changes yet"`, `A collapsed active section shows its header and no message row`, and `No active changes with archived ones still browsable`. Every one asserts interior widths 38 and 58. The `ui::app::tests::` halves of `A query reaches a match inside a folded archive` and `The archived count under a query is the matched count` belong to group 4, where `section_open` lands; this group takes their render halves.
- [ ] 5.2 GREEN: Replace `RowKind::Separator` with `RowKind::Section { key, depth, collapsed }` and emit two section headers in place of the separator, each `[marker][space][glyph][space][label][space][(count)]` through `pad_or_truncate_right`.
- [ ] 5.3 GREEN: Split the single `let mut index = 0usize` at `src/ui/list.rs:430-493` into two counters (design.md → Decision 15): the running `visible()` counter fills `RowKind::Item { index }`, and the row's `targets()` position decides the selection marker.
- [ ] 5.4 GREEN: Implement the count rule — matched entries when the tier is resolved, `archived_total` when it is not — gate each header on a count greater than zero, and key the three empty-state message rows on the section **counts** rather than on visible rows (design.md → Decision 10).
- [ ] 5.5 GREEN: Map `RowKind::Section` to `Role::ListSeparator` in `ui::view`'s style table (design.md → Decision 3), with `Modifier::BOLD` when the row carries the cursor. Confirm no colour literal is written outside `src/ui/palette.rs` by running `make gates` and reading `PALETTE` OK.
- [ ] 5.6 CHECK: Shift every positional row assertion for the one added header row, across all three files that carry one: `src/ui/list.rs` (12 `rows[` indexings, `grep -c "rows\[" src/ui/list.rs`, plus the buffer-row assertion at `:2011`), `src/ui/view.rs`, and `src/ui/driver.rs` (`:2937`). Rewrite — do not delete — the two landed tests whose names encode the removed construct: `the_separator_is_emitted_only_when_archived_rows_follow` (`src/ui/list.rs:1403`) and `row_order_is_problems_then_active_then_separator_then_archived` (`:1428`).
- [ ] 5.7 CHECK: `grep -rn "RowKind::Separator" src/ tests/ | wc -l` reports **0** (check D was **8**), and `make gates` reports `COLWIDTH` OK, so the new cells are measured in display columns.
- [ ] 5.8 Run `cargo test --all-features` — no regressions, and state whether a refactor was needed.

## 6. Startup, the composition root, and file mode
<!-- kind: behavior -->

- [ ] 6.1 RED: Write failing tests in `src/ui/mod.rs` for `Startup counts the archive without resolving it` and `A pre-list-sections configuration loads unchanged`, and update `A scratch repository is loaded from disk with no binary present`, `No repository above the starting directory`, `Loading writes nothing`, `load reads the agent-name mapping from the directory it was given`, and `An unusable mapping file is an empty mapping with a named problem` for the new `sections`, `archived_total` and `ArchivedScope` argument.
- [ ] 6.2 GREEN: Give `ui::load` its fourth parameter, `archived: ArchivedScope`, stop reading `config.archived_count`, and seed `sections.collapsed` with exactly `SectionKey::Archived` on both branches. Verify the loaded `Dashboard` is equal across `archived_count` values 0, 3, and 7.
- [ ] 6.3 GREEN: In `ui::run`'s composition root, drop `config.archived_count` from the `refresh::start` call and pass `load` the scope the probe implies — `Names` when a binary resolved, `Full` when none did (design.md → Decision 13). Pass `dashboard.archived_scope()` at **both** of `run_loop`'s `request` call sites, step 2 and step 3.
- [ ] 6.4 RED then GREEN: Write `File mode opens the archive with no binary present` at the wiring tier — `ui::tests::wiring`, driving `ui::run_wired` with the probe resolving nothing, a `Space` press and a `q` press. This is the one outer-loop scenario this change takes, and the tier that would have caught the file-mode hole; confirm it fails for the missing behaviour before 6.3 lands, not for a misconfigured harness.
- [ ] 6.5 CHECK: Rewrite the landed test `archived_count_from_config_is_honoured`, which asserts the behaviour this change deletes. It becomes an assertion that the loaded `Dashboard` is **unchanged** across `archived_count` values — the same fixture proving the opposite property — rather than being deleted, so the key's inertness keeps a test of its own.
- [ ] 6.6 CHECK: Contract gate — re-read `SPEC.md` → Resolution chain's `config.toml` description and confirm the documented configuration **format** still matches `src/config.rs`. The format is unchanged; only the key's effect is, and group 9 is what records that.
- [ ] 6.7 Run `cargo test --all-features` — green, and state whether a refactor was needed.

## 7. Gate floors
<!-- kind: operational -->

- [ ] 7.1 CHECK: Re-run `SCAN_MIN=135 TYPES='Dashboard Filter Detail Sections' /bin/sh scripts/gates/nodefault-ui.sh` and record the span count it reports. At HEAD the same leg without `Sections` reports 150 spans and exits 0 (check B).
- [ ] 7.2 CHANGE: Add `Sections` to the first `NODEFAULT-UI` line's type set in the `Makefile` and raise that line's `SCAN_MIN` to the newly measured floor, leaving the other four legs' floors untouched.
- [ ] 7.3 CHANGE: Write `notes/gate-floors.md` in this change directory recording the command, the measured span count, and the floor chosen — then `git add` it. `OPENSPEC-UNTOUCHED` fails on any untracked file under `openspec/` (check C), so an unstaged note turns 7.4 red for a reason unrelated to the floor.
- [ ] 7.4 VERIFY: `make gates` — every gate OK. Negative control: raise the same line's `SCAN_MIN` above the measured count, confirm it prints `the scan is vacuous` and exits 1, restore it, and confirm the gate goes quiet again.

## 8. Change Review
<!-- kind: operational -->

- [ ] 8.1 CHECK: Dispatch an independent `outside-in-tdd-reviewer` against `proposal.md`, all eight spec deltas, `design.md`, `tasks.md`, and the diff — not this session's reasoning. Concentration points: whether the read recorder actually distinguishes the two scopes or was quietly reduced to a value assertion; whether the `selected` re-index in 3.5 weakened an assertion it was only supposed to move; whether any write to `selected` still uses a `visible()` index; whether `archived_total` and `archived.len()` can disagree on any path through `merge` or `adopt`; and whether any new width arithmetic bypassed `layout::columns`.
- [ ] 8.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, note SUGGESTIONs, re-run the affected tests, and `git add` any note file the review produces before the next gate run.
- [ ] 8.3 VERIFY: Confirm no blocking or unowned finding remains.

## 9. Documentation
<!-- kind: operational -->

- [ ] 9.1 CHECK: Locate each target passage and record what it currently says: `grep -n "archived_count" SPEC.md README.md AGENTS.md` and `grep -n "^### List view\|^### Keys" SPEC.md`. This group's evidence is that re-read, **not** a test: `tests/doc_contract.rs` binds the module map, tested-modules list, MSRV, gate programs, manifest transcription, injected context and worker-thread count, and `tests/degraded_coverage.rs` binds the degraded-states table — none of them reads the passages below, so `cargo test --test doc_contract` passes whether or not this group is done (design.md → Test Strategy).
- [ ] 9.2 CHANGE: Rewrite in `SPEC.md`, two sections (audience: every future change to this pane) — the Data-layer *Archived changes* paragraph at line 184, and *List view* at lines 367-379 whose sample block still draws `-- archived ----`. Replace the cap with the two section headers, the fold, and the count. Durable because the sample block is what the next row-grammar change is written against.
- [ ] 9.3 CHANGE: Rewrite in `SPEC.md`: Keys, and Resolution chain's `config.toml` description (audience: same) — add `Space`, and restate `archived_count` as accepted-but-inert. Replaces a claim that is now false rather than merely incomplete.
- [ ] 9.4 CHANGE: Rewrite `README.md:86`'s whole `archived_count` cell (audience: a reader configuring the plugin). It currently reads "Archived changes listed below the separator", which is false on both counts after group 5; the replacement says the key is accepted, parsed, and has no effect on the list. A rewrite, not an addition.
- [ ] 9.5 CHANGE: Rewrite the list-region description in **`AGENTS.md`** (audience: every agent session, loaded every time). `CLAUDE.md` is a symlink to it, so edit `AGENTS.md` itself. Replace "a separator, then archived ones" with the two foldable sections and `Space`; `archived_count` appears nowhere in that file, so nothing else there goes stale. Net size unchanged.
- [ ] 9.6 CHANGE: After `openspec archive` applies these deltas, rewrite the `## Purpose` paragraph of `openspec/specs/change-rows/spec.md` (it still says "an archived **separator** emitted only when archived rows follow it") and of `openspec/specs/list-selection/spec.md` (it still says the selection "never lands on a **separator**, problem or message row"). A delta carries requirements, not a Purpose, so `openspec archive` cannot update these and `tests/spec_purposes.rs` guards only that a Purpose exists, not that it is true.
- [ ] 9.7 VERIFY: Re-run 9.1's greps and confirm no target passage still describes the separator or the cap, and that `cargo test --test degraded_coverage --test doc_contract` is green — the latter as a no-regression check, not as evidence this group happened.

## 10. Lint & Verify
<!-- kind: operational -->

- [ ] 10.1 CHECK: Inspect the intended verification commands and affected tiers — `make check` runs format, lint, gates, test, and coverage, and is the single gate this repository ends on.
- [ ] 10.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 10.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 10.4 VERIFY: `make gates` — every gate OK.
- [ ] 10.5 VERIFY: `cargo test --all-features` — green against check E's baseline, with the two known `ui::tests::wiring` intermittents re-run before being treated as a regression.
- [ ] 10.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` and the production-slice floor beside it — no floor lowered, no exclusion added. Check G leaves roughly eight uncovered production lines of slack, so run this before concluding.
- [ ] 10.7 VERIFY: `make check` as the single gate, naming the failing sub-command if it fails.
- [ ] 10.8 VERIFY: `openspec validate list-sections --strict` — valid.
