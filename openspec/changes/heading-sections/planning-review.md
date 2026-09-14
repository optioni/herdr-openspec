## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/artifact-folds/spec.md`
- `specs/artifact-content/spec.md`
- `specs/detail-scroll/spec.md`
- `specs/tasks-checklist/spec.md`
- `specs/mouse-input/spec.md`
- `specs/list-selection/spec.md` — added *during* this review; see GAP-07

The finding pass was delegated to four `planning-reviewer` subagents, one slice each, run
simultaneously and given only the change directory: (A) capability coverage, scenario quality,
cross-artifact contradictions; (B) design completeness, test boundaries, falsifiability audit;
(C) task alignment, lifecycle discipline, `parallel-after` independence; (D) factual
verification by running commands. None of them wrote any part of the plan, and none edited a
file. This session merged their findings, verified each one independently, repaired the owning
artifact, and wrote this log.

## Reviewed Against

- This repository HEAD: `857163a13178f03b61fd6754b9df0106a58d7128`
- Sibling repositories: **Not applicable.** The change adds no dependency and crosses no
  repository boundary. `~/Code/openspec-schemas` is the graft source for
  `openspec/schemas/tdd/` and `.claude/agents/`, neither of which this change touches.
- Working tree: the `heading-sections` planning artifacts, intentionally included — they are
  the subject. `git status` showed no other modification at review time.
- `openspec` CLI 1.12.0, nvm-installed and not on the default `PATH`.

## Gaps Found and Fixed

Severity is the reviewer's, kept where this session's own verification agreed with it.

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/artifact-content/spec.md` | GAP-01. The MODIFIED block for "The content area renders the artifact…" dropped **five** normative paragraphs present in the live spec at `:330–359`, including the total display-width property that this change's own `artifact-folds` delta points at, and the only site stating the `viewport`-vs-`scroll_offset` choice the change depends on. A MODIFIED block replaces the whole requirement at archive time, so all five would have been deleted from the repository. | Restored all five, reworded where this change genuinely affects them: plain faces now cover `items` and the blank separator row; the width property states that a header's depth indent is inside the budget; the offset paragraph notes the tasks tab now reaches the foldable arm. | `specs/artifact-content/spec.md` → "The content area renders the artifact, its problems, or `No content yet`" |
| CRITICAL | `design.md`, `tasks.md` | GAP-02. The splitter was sited in `src/ui/detail.rs`. `scripts/gates/detailwidths.sh` requires **every** `#[test]` in that file to name both `58` and `78` and carries no exemption list by design; the splitter measures no width, so its six tests name neither. `make gates` would have gone red at task 1.6 and stayed red through the final verify. | Re-sited to `src/ui/app.rs`, beside `sync_detail` (the only consumer) and the file-label rule at `:152`. Measured: the six `*WIDTHS` gates name one file each and none is `app.rs`, which is still swept by `COLWIDTH`, `NOIO-VIEW`, `NOBLOCK`, `NODEFAULT-UI`, `READONLY-UI`. Costs zero capability deltas and zero gate edits — cheaper than all three repairs the reviewer proposed. | `design.md` → D11 (rewritten), Boundaries; `proposal.md` → Impact; `tasks.md` group 1; all six deltas |
| CRITICAL | `tasks.md`, `design.md` | GAP-03. `ui::tasks::item_lines` **already exists** at `src/ui/tasks.rs:264` as a private per-item helper called from `lines` at `:356`. The planned `item_lines(source, width)` is a duplicate definition and would not compile. Separately, a string-taking form could not be called from `lines` at all, which holds parsed `tasks::Group` values and would have to re-serialise each group — the exact duplication the extraction exists to remove. | Renamed to `items`, and its parameter changed to `&[crate::tasks::Item]`. `lines` calls `items(&group.items, width)`; `content_lines` reaches it through `tasks::parse(&section.text)`. Swept all 27 other new identifiers and test names for collisions: none. | `design.md` → D9, Contracts; `specs/tasks-checklist/spec.md`; `tasks.md` group 5 |
| CRITICAL | `specs/artifact-folds/spec.md` | GAP-04. Two scenarios disagreed on the section count for the same task-file fixture — "holds two" against "holds three entries". Three is correct under the preamble rule; the first was counting the splitter's return rather than `detail.sections`. `tasks.md` put both tests in one group, so it would have surfaced as a contradictory RED pair. | Corrected to three, and the sentence now says explicitly that the splitter's return and `detail.sections` are different counts. | `specs/artifact-folds/spec.md` → "A spec file splits and a task file splits" |
| CRITICAL | `specs/artifact-content/spec.md` | GAP-05. "The tracked-tasks tab concatenates rather than folding" expected four header rows, one of which its own visibility rule hides: seeding leaves the first file section collapsed, which hides its group. Re-derived independently and confirmed — three headers are drawn, not four. | Fixture changed so both files are incomplete and all four headers are genuinely visible, and the collapsed case added as a second clause, since a collapsed parent hiding its own group is the more interesting half. | `specs/artifact-content/spec.md` |
| CRITICAL | `specs/artifact-content/spec.md` | GAP-06. "A foldable tab's body is headers…" was carried **byte-identical** from the live spec and therefore contradicted this delta's own new requirement, which mandates a blank separator after a non-empty open body. | Inserted the blank row in the THEN clause. | `specs/artifact-content/spec.md` |
| WARNING | `proposal.md` | GAP-07. `list-selection` had no delta, but its scenario "resolves to **one path**" stopped implying "one section" the moment a single file could split — and this change repaired the identical fixture in `artifact-folds` while leaving that copy. | Added `specs/list-selection/spec.md` MODIFYing that requirement to say "one path **holding prose**", with a clause stating why. Added to the proposal's capability list and to the verification matrix (8 scenarios), with a confirmation task. | `specs/list-selection/spec.md`; `proposal.md` → Modified Capabilities; `design.md` matrix; `tasks.md` 7.5 |
| WARNING | `design.md` | GAP-08. D4 concluded fence tracking was "not repairing a live defect", from a measurement scoped to spec files only. Re-measured across the archive: **1297 ATX-shaped lines inside fences across 37 archived `tasks.md` files** (1491 across 148 prose/task artifacts). Task files split, so a fence-blind splitter would produce hundreds of phantom sections per file and wreck depth normalisation for the real headings. The original conclusion was backwards. | D4 rewritten with the scan split by corpus, and the conclusion reversed. The measurement row in `tasks.md` group 0 is now two rows, so a re-run cannot be misread. | `design.md` → D4; `tasks.md` group 0 |
| WARNING | `design.md`, `tasks.md` | GAP-09. The Risks bullet claimed the new blank separator row was contained because no pre-change fixture has an open non-empty section on a foldable tab. Planting the separator in a `git archive HEAD` tree and running the suite gives `1315 passed; 5 failed`. The first repair plan — grep for the adjacency pattern — cannot work: one failure is `expanded` resolving to `{0,1}` instead of `{0,2}`, because the extra row shifts which section `detail.scroll` addresses. | Risk rewritten with the measured count. Task 4.3 now runs the suite and names the **expected count of five** plus all five test paths, which catches both an over-broad separator and one that never fires. | `design.md` → Risks; `tasks.md` 4.3, group 0 |
| WARNING | `tasks.md` | GAP-10. Group 5 was `refactor` but introduced new public functions, and scheduled its evidence task **after** the change task. Its scenario also asserted `items`' output against `lines`' output, which cannot fail once `lines` calls `items` — the tautology `design.md` → D9 argues against in as many words. | Reclassified `behavior`, RED moved first, and the scenario's two comparison clauses replaced with literal assertions. Contract gate added. | `tasks.md` group 5; `specs/tasks-checklist/spec.md` |
| WARNING | `tasks.md`, `design.md` | GAP-11. Group 9's GREEN — "confirm `normalise_scroll` branches on `Detail::foldable()`; change it if it does not" — had nothing to implement: it already does, at `src/ui/app.rs:998`, as do the other call sites. Its RED tests would also have been green before being written, since groups 3–8 make the tab foldable. Separately, the "no outer-loop acceptance group" note argued from `ui::run`'s exit 3 that no outer tier existed, ruling out a process-level test nobody proposed while the `run_loop` tier plainly did exist. | Groups 6, 7 and 9 merged into one group whose RED is the three `run_loop` rows — genuinely failing at HEAD. That gives real RED→GREEN with no red window, which a group-0 outer loop could not: it would sit red across five commits, and AGENTS.md requires every commit to pass `make check`. Both the note and `design.md` → Test Strategy now give that as the reason. | `tasks.md` header note, group 6; `design.md` → Test Strategy |
| WARNING | `tasks.md` | GAP-12. Eight steps named tests that do not exist under those names — scenario titles used as function names — and one named the `NODEFAULT-UI` gate as if it were a test. Task 3.4 said "add" for five tests that already exist, which would have produced duplicate `fn` names and a compile error, and 4.1 said "write" for a test that exists at `src/ui/detail.rs:716`. | Every reference replaced with the real function name and file, each verified present by `grep -rn "fn <name>" src/`. "Add" changed to "confirm" in 3.4, "write" to "extend" in 4.1. The gate scenario now names the gate invocation and its planted control. | `tasks.md` 3.4, 4.1, 4.4, 5.1, 6.1, 6.2, 6.6, 7.3, 7.4 |
| WARNING | `tasks.md` | GAP-13. Group 11's document edits ran before its net-size CHECK, inverting the operational lifecycle. | CHECK promoted to 9.1; edits renumbered beneath it. | `tasks.md` group 9 |
| WARNING | `tasks.md` | GAP-14. Three capability `## Purpose` blocks would ship contradicting their own requirements — `artifact-folds`' Purpose repeats the Decision 8 claim this change reverses, verbatim. A delta carries no Purpose block and `openspec archive` never rewrites one, so an untouched Purpose ships stale; this repository has paid for that twice before. | Added task 9.2 naming all three files. Precedent checked: `header-progress-bar` rewrote `openspec/specs/` Purpose blocks during implementation (`4c15adb`), so this belongs in the Documentation group rather than at archive time. `detail-scroll`, `mouse-input` and `list-selection` Purposes were read and stay true. | `tasks.md` 9.2 |
| WARNING | `specs/detail-scroll/spec.md` | GAP-15. Found by this session while working out group ordering. The clamp scenario used "twenty task lines under **one** heading" to demonstrate the **cursor** rule. One heading yields one section, so `foldable()` is false and the offset rule applies — the scenario asserted the wrong clamp, and both its halves were non-foldable, so it discriminated nothing. | Two headings versus none, differing only in the property under test. | `specs/detail-scroll/spec.md` |
| WARNING | `specs/detail-scroll/spec.md` | GAP-16. The new `j` scenario said two presses land on the "third group's header row". Rows 0 and 1 are the progress bar and its blank line, so they land on the **first**. | Corrected, and extended with two further presses so the cursor is shown walking the rendered list rather than a fixed section index. | `specs/detail-scroll/spec.md` |
| WARNING | `design.md` | GAP-17. A leftover from this session's own module rename: "`ui::detail` is entirely new and has one consumer". The module is not new; only the two functions are. | Rewritten. | `design.md` → Contracts |
| WARNING | `proposal.md` | GAP-18. The task-group figures were measured over 33 archived changes; there are 37, and the minimum is 5, not 6. | Recounted and corrected, with the command retained. | `proposal.md` → Why |
| WARNING | `tasks.md` | GAP-19. "67 construction sites" is the grep's line count, not the number of sites to edit: 3 matches are the `tests/gate-controls.toml` planted defect and 1 is the struct definition. | Row restated as **63** with the per-file breakdown, plus the finding that the plant matches the struct's *header* line and therefore keeps working when fields are added. | `tasks.md` group 0, 2.2 |
| WARNING | `tasks.md` | GAP-20. The sequential-ordering rationale said "six groups write `src/ui/detail.rs`" (four do) and contradicted itself three sentences later about group 5. Neither number carried a command. | Rewritten with measured file sets and the command that produced them, and with group 5's parallelism analysis stated explicitly: it passes tests 1 and 2 and is rejected on attributability alone. | `tasks.md` header note |
| WARNING | `design.md` | GAP-21. The verification matrix omitted the 8 scenarios of the `list-selection` delta added by GAP-07. | 8 rows added; a re-run of the coverage check reports 97 delta scenarios and 97 matrix rows, 0 missing. | `design.md` → Test Strategy |
| SUGGESTION | `specs/artifact-folds/spec.md` | GAP-22. Three rationale sentences were dropped from the `fold_glyph` paragraph, including the one arguing that an agreement assertion would be tautological — the same argument `design.md` → D9 relies on. No SHALL was lost. | Restored while the block was being edited anyway. | `specs/artifact-folds/spec.md` |
| WARNING | `design.md` | GAP-24. The Test Boundaries table closes with "No task may invent a boundary this table does not name", but its nine rows covered only the unit, view and loop tiers. Tasks 1.6, 4.6, 6.8, 9.6 and 10.4 shell out to the **gate tier** — real subprocesses over the real working tree — and two matrix rows already named the scratch-directory tree as their collaborator. The sentence was false of its own table. | Two rows added: the gate scripts, and `tests/gate_controls.rs`'s planted defects. The other direction is clean — no task invents a unit, view or loop collaborator the table omits. | `design.md` → Test Boundaries |
| SUGGESTION | `design.md` | GAP-23. Two gate hazards in the splitter's own code were unpriced: `COLWIDTH` sweeps `src/ui/app.rs` whole-file for `.chars().count()`, and `NOBLOCK`'s pattern matches a bare zero-argument `.join()` — exactly where a round-trip reassembly reaches. | Both recorded in Boundaries and in tasks 1.4 and 1.5, with the safe form named. | `design.md` → Boundaries; `tasks.md` 1.4, 1.5 |

### Checked and found clean

Recorded because a negative result is evidence, and the absence of these rows would otherwise
read as nobody having looked.

- **The other five MODIFIED blocks lost nothing.** `artifact-folds` 16/16 scenarios,
  `detail-scroll` 4/4, `tasks-checklist` 12/12, `mouse-input` 9/9, `list-selection` 8/8, each
  diffed against its live spec. GAP-01 was the only real loss.
- **No further capability needs a delta.** `tasks-progress-bar`, `detail-header`,
  `responsive-layout`, `dashboard-loop`, `doc-conformance`, `quality-gates`, `artifact-tabs`,
  `view-palette`, `markdown-render`, `degraded-coverage`, `binding-inventory`, `help-overlay`,
  `change-artifacts`, `schema-artifacts` all checked against the live tree.
- **Scenario quality**: no vague scenario. Concrete fixtures, named indices, both mandated
  widths, explicit reader call counts. The scenario defects above are wrong *values*, not
  vagueness.
- **No RED task is aimed at plumbing.** The change touches no manifest, lockfile, CI config, or
  formatter setting.
- **Persistence and contract gates** are present where they are owed, after GAP-10 added the
  one that was missing.
- **All eleven design decisions** have at least one task.
- **Task prose** is within budget; no task argues with the implementer.
- **`tasks.md` has not outgrown `design.md`** — 193 lines against 566.
- **The `Change` type is untouched**, so `changes::from_files` and `changes::from_cli` need no
  work to stay in agreement and `changes::merge` is not involved.
- **Nothing spawns a process outside `src/cli.rs`**, no file under `src/ui/` gains an I/O,
  blocking-wait, or clock API, and the plugin's own writes stay scoped to
  `HERDR_PLUGIN_STATE_DIR`. The change crosses no PRD non-goal.
- **The `NODEFAULT-UI` planted control survives** the two new `ArtifactSection` fields: it
  matches the struct's header line, not its body.
- **Every cited line reference** resolves to what it claims — `src/ui/app.rs:152`, `:998`,
  `src/ui/tasks.rs:264`, `src/ui/view.rs:4022`, `src/ui/detail.rs:716`, `:2023`,
  `openspec/specs/detail-header/spec.md:187`, `:381`.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL and WARNING above is repaired in the artifact that owns it, and
`openspec validate heading-sections --strict` reports the change valid.

Two items are recorded as user-facing judgement calls rather than gaps, and are carried in
`proposal.md` → Open Questions for the Change Review rather than blocking implementation:
whether the tasks tab's move to the line-cursor model is acceptable given that `j`/`k` stop
scrolling by line, and whether a completed group starting collapsed is the right default given
that it makes the tab's opening shape depend on file content. Both were user decisions at
proposal time and neither is a planning defect.

A second reviewer observation is **recorded rather than repaired**: tasks 2.3, 6.7 and 10.1
are prose-only CHECK steps that cannot mechanically fail. All three are the contract gate, the
persistence gate, and the verification-command inspection that the `tdd` schema mandates by
name and in that form. An inspection step is what those gates are; rewriting them into
assertions would be inventing a check the schema does not ask for. Every gate the tasks
actually invoke does carry a planted negative control.

One reviewer suggestion was **considered and declined**: marking group 5 `parallel-after: 0`.
It passes two of the three parallelism tests, but `make check` is a whole-tree gate and this
repository's rules forbid a worktree, so a concurrent failure would not stay attributable.
The reason is now written into `tasks.md` rather than left as silence.

## Deferred Non-Blocking Notes

- **The sticky heading line** was dropped from this change by user decision, not deferred by
  omission. Its resolution point is recorded in `proposal.md` → What Changes: it becomes its
  own follow-up once folding is proven. `responsive-layout` is untouched as a result.
- **`tasks::parse` is not taught about fences.** The splitter tracks them; the counting rule
  does not, and must keep agreeing with the OpenSpec CLI's. Recorded in `proposal.md` →
  Impact, and out of scope by the Non-Goals.
