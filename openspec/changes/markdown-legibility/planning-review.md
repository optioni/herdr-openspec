## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/markdown-render/spec.md` (5 MODIFIED requirements, 1 ADDED)
- `specs/tasks-checklist/spec.md` (3 MODIFIED requirements)
- `specs/degraded-coverage/spec.md` (1 MODIFIED requirement)

Reviewed by four `planning-reviewer` subagents, one slice each, none of them the session
that wrote the package: (A) capability coverage, scenario quality, cross-artifact
contradictions; (B) design completeness, test boundaries, and whether each proposed check
could fail at all; (C) task alignment, lifecycle discipline, `parallel-after` independence;
(D) factual verification of every empirical claim, by running the command.

## Reviewed Against

- This repository HEAD: `43de01e` — the checks were recorded against `8f069d0`, and
  `git diff --stat 8f069d0..43de01e -- src/ tests/ Cargo.toml scripts/ Makefile` is **empty**;
  all five intervening commits are documentation. Every source-level measurement below holds
  at both.
- Sibling repository HEAD: Not applicable. `~/Code/openspec-schemas` supplies the vendored
  schema and agents but no contract this change touches.
- Working tree: clean of source changes. `openspec/changes/markdown-legibility/` (this
  package) and `openspec/changes/foldable-spec-sections/` (another session's concurrent
  change) are untracked, which is why `scripts/gates/openspec-untouched.sh` exits 1 until
  this directory is committed — recorded as task 10.1 rather than treated as a defect.
- Two glyph plants were made in `src/ui/markdown.rs` during review to measure the blast
  radius and to give `COLWIDTH` a negative control. Both were reverted; `git status`
  confirms no source file is modified. One reviewer observed a plant mid-flight and flagged
  it as possible unapproved implementation — it was measurement, and no project code has
  been changed.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs, tasks | A task-list item's continuation indent is specified but **no check could fail on it**. `start_item` builds `cont_prefix` from `columns(&marker)` before `TaskListMarker` is seen, so a `first_prefix`-only arm hangs continuations at 2 columns instead of 4. Proven by planting groups 1–4: renders `[✓] alpha…` / `  india…` while both width sweeps stay green — a shorter indent never overruns, and no fixture wraps a task-list item | Added a wrapped-item clause asserting exactly four spaces on continuations at 58 and 78; added "recompute `cont_prefix`" to the task; recorded the whole trap as a new Decision | `specs/markdown-render` → task-list requirement + narrowing scenario; `tasks.md` 4.2; `design.md` → Decision 7 |
| CRITICAL | proposal, tasks | `SPEC.md` § Detail view states **five** things this change reverses (the `[x]` glyph, the soft break, the `> ` quote prefix, the table's `\|` grammar, task-list items as literal source) and no task edited it. `CLAUDE.md` makes `SPEC.md` the winner where prose and spec disagree, and `tests/doc_contract.rs`'s nine claims do not cover that paragraph | Added a task rewriting `SPEC.md:485-510`; widened Impact; replaced the detection check, since `grep 'a footnote, a task-list item'` misses the line-wrapped occurrence at `:508-509` — `grep -c 'task-list item' SPEC.md` → 2 finds both | `tasks.md` 7.1, 7.3; `proposal.md` → Impact |
| CRITICAL | proposal, design, tasks | The stated file scope was **2 files against a measured 25 failures**. Planting only group 2's two substitutions breaks `ui::view` 10, `ui::markdown` 6, `ui::app` 4, `ui::driver` 3, `ui::tests::detail` 2 — 19 outside the two production files | Added `src/ui/view.rs`, `src/ui/mod.rs`, `src/ui/app.rs`, `src/ui/driver.rs` as test-re-baseline-only rows, with the shared `format!("- line-{i:02}\n")` fixture named as the single cause | `design.md` → Boundaries; `proposal.md` → Impact; `tasks.md` 2.4 |
| CRITICAL | design, tasks | Three groups verified with commands that **cannot run the tests they break** — `cargo test --lib ui::tasks` matches none of the `ui::view::tests::*` Dashboard tests, so a group would report "no regressions" while red. Task 5.1 also said "write failing tests", which would have produced a new test beside two untouched red ones | Every behaviour group now verifies with `cargo test --lib ui::` (549 tests); 5.1 says **rewrite the existing** `ui::view::tests::*`; three matrix rows re-pointed to `ui::view` | `tasks.md` 1.5, 2.5, 3.6, 4.5, 5.1, 5.4, 6.6; `design.md` → matrix |
| WARNING | specs | The new `TaskListMarker` arm had **no narrow-width or totality coverage** — the two sweep scenarios' fixtures held no task-list item, so the only coverage was 58 and 78 | MODIFIED `markdown-render`'s first requirement into the delta; both sweeps gain checked, unchecked, quote-nested and degenerate task-list inputs | `specs/markdown-render` → sweep scenarios; `tasks.md` 6.0; `design.md` → 5 matrix rows |
| WARNING | tasks | Every check snippet wrote `58u16`, which the `MDWIDTHS` gate cannot see: it requires each `#[test]` to name 58 and 78 and finds them with `\b(\d+)\b` — `re.findall(r'\b(\d+)\b','[58u16, 78]')` → `['78']`. A snippet lifted in as written fails `make gates` at the last task | All six snippets rewritten unsuffixed, with the measurement recorded in the file header | `tasks.md` header, 1.1, 2.1, 3.1, 4.1 |
| WARNING | proposal, design, tasks | "No gate changes" was false. `src/ui/markdown.rs` holds exactly 34 tests and `mdwidths.sh` defaults `MD_MIN=34` — the floor sits at its true value and group 6 adds the 35th, with nothing failing if it is left behind | Added the `MD_MIN` 34 → 35 task; corrected the claim in both artifacts | `tasks.md` 6.4; `proposal.md` → Impact; `design.md` → Non-Goals, Boundaries |
| WARNING | tasks | Task 3.3 said "the four scenario literals". `grep -c 'allocated_widths(' src/ui/markdown.rs` → **9**, and the helper itself does `.trim_matches('\|').split('\|')` with `columns(run) - 2`, neither valid against a `├─┼─┤` delimiter | Split into three tasks: rewrite the helper and `field`'s stale doc comment, re-baseline the eight call sites plus two direct-literal tests, and re-baseline the container test that groups 2 and 3 both hit | `tasks.md` 3.3, 3.4, 3.5 |
| WARNING | tasks | Group 6's RED **could not fail** — reached in plan order its test is green, because groups 1–4 already landed the glyphs it pins. Group 5 carried no recorded check at all | 6.1 relabelled CHECK with a two-part negative control (plant a two-column glyph; plant a non-recomputed `cont_prefix` and show the sweeps stay green); 5.0 records group 5's check | `tasks.md` 5.0, 6.1, 6.3 |
| WARNING | design | The Test Boundaries table contradicted three of its own matrix rows — Filesystem "replaced" vs `ScratchDir` real, collaborators "not reached" vs stub `FsEvents`, Herdr "not reached" vs a real spawned process | Each cell now distinguishes what this change *adds* from what the twelve carried `degraded-coverage` rows reach | `design.md` → Test Boundaries |
| WARNING | design, tasks | The plan prescribed a re-baseline **measurement contradicts**: `paragraph_wraps_at_58_and_78`'s fixture is one source line with no soft break. The fold breaks exactly **one** test of 1222, `a_soft_break_starts_a_new_line` — the plan pointed an implementer at a correct literal, the precise failure mode its own Risks section names | Row corrected to "unchanged existing test"; task 1.3 narrowed to the one test, and its single-line heuristic scoped to group 1 (it would misclassify `src/ui/driver.rs:904`, whose fixture is one line per item) | `design.md` → matrix, blast-radius note; `tasks.md` 1.3 |
| WARNING | design | Two matrix commands were wrong: `A detail.tab past the end…` pointed at `ui::tasks`, but its test is `ui::detail::tests::tab_past_the_end` | Corrected to `ui::detail`; the other 45 rows verified against their named tests | `design.md` → matrix |
| WARNING | specs | The footnote fixture was described two ways, and the fold made the difference load-bearing: `degraded-coverage` said "on separate lines" then asserted rendered line count equals source line count, which fails if the lines are adjacent | Both scenarios now quote the same three-line source, blank line included, with the reason stated | `specs/degraded-coverage` |
| WARNING | specs | "Exactly those seven glyphs" was unfalsifiable as written — `✓` enters the set only if the fixture's item is checked, and an ordered list (`2.` from source `1.`) or an image (`[img]`, `src/ui/markdown.rs:602`) would each add a non-source character that is not a glyph | Fixture pinned to a checked **and** unchecked item, with the ordered-list and image exclusions stated as deliberate | `specs/markdown-render` → ADDED requirement |
| SUGGESTION | proposal, design | Four factual overstatements, each corrected to what measurement supports: the wrap band is 89–**92** (lines 37, 65, 67); glow's margin is **four** columns, not two; `tui-markdown`'s `syntect`/`ansi-to-tui` ride the **default** `highlight-code` feature, so it is one crate under this repo's uniform `default-features = false`; and the Ambiguous glyph list omitted `├`/`┤`, understating the risk being accepted | Corrected in place | `proposal.md`; `design.md` → Context, Decision 5 |
| SUGGESTION | proposal, design | The glow raggedness claim said "byte-identical". Measured at a matched content width (`-w 82`), stripping margin and pad: **48 of 130 lines identical, the first 32 consecutively**, first divergence being glow's `•` against our `-` | Restated to the measured figures; the conclusion is unchanged and better supported | `proposal.md` → Why; `design.md` → Context, Decision 5 |
| SUGGESTION | tasks | `CLAUDE.md` is a **symlink** to `AGENTS.md`; a `Write` through it would replace the link with a regular file | Documentation tasks now name `AGENTS.md` and require `Edit` | `tasks.md` 9.1, 9.2 |
| SUGGESTION | tasks | Two in-code sites the plan missed: `src/ui/markdown.rs:2311` builds a **second** copy of the option set inside the very test the matrix leans on for its parser-event assertion, and `:630`/`:634` document the old set | Task naming all four sites, with the silent-disagreement consequence stated | `tasks.md` 4.3 |
| SUGGESTION | tasks | Group 6 asked for an all-constructs fixture the file already has — `composite_fixture()` (`src/ui/markdown.rs:1211`) covers all but two of the needed constructs | Extend the existing fixture rather than adding a second beside it | `tasks.md` 6.2 |
| SUGGESTION | tasks | The sequencing note named only group 4 as group 7's dependency, and 10.4 re-ran `make coverage` that `make check` (`Makefile:71`) had just run | Both dependencies named, with the third parallelism condition addressed; 10.4 reworded as an assertion on 10.2's output | `tasks.md` header, 10.4 |
| SUGGESTION | proposal | The roadmap claim went stale mid-review: `IMPLEMENTATION-ORDER.md:191-195` now records this change and fixes an archive order, and `pane-chrome` shares the `tasks-checklist` capability | Restated as "unplanned in origin, planned in sequence", naming the shared capability and that `pane-chrome` rebases onto this change, not the reverse | `proposal.md` → Non-Goals |

One reviewer finding was **rejected**: that the matrix pointed *A schema naming no tasks
artifact leaves every tab as markdown* at the wrong test. It conflated
`no_tasks_artifact_renders_every_tab_as_markdown` (`src/ui/detail.rs`, which backs a
`degraded-coverage` scenario) with `no_marked_artifact_renders_markdown` (`src/ui/view.rs`),
whose fixture — source `- [ ] a\n`, three ids, tabs 0/1/2 at 120x20 and 60x20 — matches this
scenario's WHEN exactly. The row stands.

One reviewer claim was **corrected back**: "line-for-line identical" for the glow comparison,
measured at 48 of 130. Recorded here so it is not carried forward as confirmed.

## No Remaining Implementation-Blocking Gaps

None remain. All four CRITICALs are repaired in the artifact that owns each, every WARNING is
resolved rather than accepted, and `openspec validate markdown-legibility --strict` passes.

The one decision that genuinely required user input was taken before drafting and is recorded
as `design.md` → Decision 4: this change reverses two argued decisions and widens `SPEC.md`'s
standing East-Asian-Ambiguous exposure from artifact content to the pane's own chrome. The
trade-off was put to the user with the measurements in hand — six of seven new glyphs are
Ambiguous, and the ASCII glyphs they replace are not — and answered in favour of the glyphs.

## Deferred Non-Blocking Notes

- **The suite is green at HEAD (1222/0) but flaky under load, and this change does not fix
  that.** `crate::testutil::Stages` (`src/lib.rs:660-724`) bounds 29 `ui::tests::wiring::*`
  tests with a 5-second **wall-clock** `DEADLINE` and force-presses `q` on expiry, so a loaded
  machine cuts runs short before the staged keys fire. Measured idle: 29/29 in ~3.8s, full
  suite 1222/0. Measured with several agents running cargo here: 7, 11 and 10 failures of 29
  across three runs taking 35–51s. The tenfold slowdown is the cause. A flake is identifiable
  by signature — a count asserted against `0` with an **empty** log — where a real defect gives
  a wrong call list. Pre-existing and out of scope: this change touches no collaborator, thread
  or clock. Resolution point recorded in `design.md` → Test Strategy and `tasks.md`'s header.
  Deserves its own change: 5 seconds is thin on a CI runner with fewer cores than the reference
  machine, and CI runs these 29 tests on both `ubuntu-latest` and `macos-latest`.
- **Two capability Purposes go stale at archive time.** `markdown-render`'s and
  `tasks-checklist`'s `## Purpose` sections state behaviour this change reverses, delta specs
  carry no Purpose, and `tests/spec_purposes.rs` rejects only an empty or placeholder one.
  Resolution point recorded as `tasks.md` 7.5.
