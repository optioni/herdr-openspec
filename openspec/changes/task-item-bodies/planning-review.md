## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/task-groups/spec.md`
- `specs/task-labels/spec.md`
- `specs/markdown-render/spec.md`
- `specs/tasks-checklist/spec.md`
- `specs/artifact-folds/spec.md`
- `specs/artifact-content/spec.md` — **added during this review** (gap C4)

## Reviewed Against

- This repository HEAD: `7d6edf3`
- Sibling repository HEAD: `~/Code/openspec-schemas` — Not applicable; no schema or agent
  change, and `openspec/schemas/tdd/` is untouched.
- Working tree: clean at review start apart from the intentionally included `tasks.md`,
  which this review wrote and then repaired. Every probe described below was planted and
  reverted, with `git status --porcelain` confirmed clean after each.
- MODIFIED deltas diffed against the live spec at that HEAD: **4 of 4** at review start, by
  extraction plus `diff -u` rather than by reading. No unaccounted difference and **no
  unnoticed revert** in any of them. Specifically, `proposal.md` → Impact's claim that the
  `artifact-folds` delta was re-diffed against the archived `section-body-indent` result is
  **confirmed**: the live requirement holds 23 scenarios, the delta holds 25, the delta is
  the live text plus exactly five hunks, and the sentence `section-body-indent` deleted
  ("Body rows SHALL NOT be indented by depth") is absent from both the delta and live,
  surviving only under `openspec/changes/archive/**`. Two further requirements were added to
  the MODIFIED set during this review (gaps C2 and C5) and were diffed on the same terms,
  bringing the total to **7 of 7**.

The finding pass was delegated to four `planning-reviewer` subagents dispatched
simultaneously, one slice each — (A) capability coverage, delta fidelity, scenario quality;
(B) design completeness, test boundaries, check falsifiability; (C) task alignment and
lifecycle; (D) factual verification. None wrote any artifact. Reviewers B and D reached the
Decision 4 defect independently by different methods, and this session reproduced it a third
way before accepting it.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md | Decision 4 **prepends** the backslash escape. A backslash escapes only ASCII punctuation, so `\1. first` parses to `Text("\1. first")` — a visible backslash on every one of the 2,842 archived items, all of which open with a digit run. It also shifts `label_of`'s byte offsets by one, slicing `" CHEC"` — a **wrong colour**, the error direction `task-labels` forbids. | Escape now inserted immediately before the character that opens the block, and **after** a digit run. `1\. first` is measured to render the fragment verbatim. | `design.md` → Decision 4; `tasks.md` → 3.2 |
| CRITICAL | design.md | Decision 4's escape set (`#`, `-`, `*`, `+`, `>`, `~`, `=`, `_`, digits) fires on inline-emphasis openers and destroys them: `\*stressed* opening` loses emphasis entirely, `\**RED**: …` downgrades strong to emphasis. That falsifies `markdown-render`'s "recognised exactly as `lines` recognises them" and `tasks-checklist`'s "An emphasised label degrades to unlabelled", whose THEN requires `face.strong`. Two archived items open with `**` today. | Trigger narrowed to genuine block openers (marker **plus a space**, a real ordered-list start, a whole-line rule run, a fence). `=`, `_` and the bare digit run dropped; the backtick fence **added** — `lines("```rust …")` returns zero rows, the one marker that deletes the fragment. | `design.md` → Decision 4; `specs/markdown-render/spec.md`; `tasks.md` → 3.2, 3.4 |
| CRITICAL | specs/task-groups/spec.md | "a blank line SHALL end a block" shreds fenced blocks. Measured: **46 of 112** column-zero fences in the corpus contain a blank line; each becomes two or more blocks with unpaired delimiters, which `ui::markdown::lines` reflows as prose and strips the ``` rows from — a differently-broken rendering of 41% of the blocks this change exists to fix. | Fence exception added for blocks **and** item bodies, with an unterminated fence running to the end of its group. New scenario "A fenced block survives the blank line inside it". | `specs/task-groups/spec.md` → "A group carries the blocks…"; `tasks.md` → 1.6a |
| CRITICAL | specs/task-groups/spec.md | The live requirement "An ATX heading at the start of a line opens a group" emits the headingless leading group **only when it holds an item**, so a preamble's retained blocks are discarded with the group. Measured: **27 of 44** corpus files open with non-blank content above their first heading, **736** lines, so the delta's own "Every retained line appears exactly once" could not pass. Covered by no delta and no Decision. | Requirement added to the MODIFIED set with the condition amended to "at least one item **or** at least one block", plus two scenarios; the reasoning and its bounded cost recorded as a Decision. | `specs/task-groups/spec.md`; `design.md` → Decision 1a; `proposal.md` → Capabilities; `tasks.md` → 1.5 |
| CRITICAL | proposal.md | `artifact-content` is modified in fact but had no delta and was absent from Capabilities. Two of its normative sentences name `ui::tasks::items` and its "items-only grammar" — both inside a requirement no other delta reaches, so after archive a live spec would name a function that no longer exists. | New delta created carrying that requirement with both sentences moved to `group_body` and the group grammar, plus one scenario. Capability listed in the proposal. | `specs/artifact-content/spec.md` (new); `proposal.md` → Modified Capabilities |
| CRITICAL | specs/artifact-folds/spec.md | The requirement "`Space` toggles the artifact section the cursor is on or in" carries a scenario fixing the preamble rows at "the progress-bar row and its blank line, and those two only", justified by `ui::tasks::items` rendering "task items and nothing else". Both become false once the preamble's prose is its group's position-0 block. Covered by no delta. | Requirement added to the delta with the row set and justification amended. What keeps the action inert is unchanged and now says so: a body row is not a fold target whatever drew it. | `specs/artifact-folds/spec.md`; `proposal.md` → Capabilities |
| CRITICAL | design.md | Two Verification Matrix rows ran `cargo test --lib ui::detail ui::view`, which cargo **rejects** — 0 tests selected. It is the command the whole `artifact-folds` regression tier rested on, so that tier bound nothing. | Rewritten to the `--` form, measured at 229 selected. | `design.md` → Verification Matrix |
| CRITICAL | tasks.md | Task 7.1 named 4 stale `ui::tasks::items` sites; the command returns **10**. 7.4 then asserted the grep returns nothing, which is unsatisfiable — 8 of the 10 are delta-owned and must not be hand-edited. | 7.1 restated at the measured 10 with each site assigned to its owner; 7.4 narrowed to the two this group actually rewrites. | `tasks.md` → 7.1, 7.4 |
| CRITICAL | tasks.md | Task 1.1's fixture glob expects 44 rows but matches **45** — the change's own `tasks.md`, whose counts move every time a box in it is ticked, making the "unmoved counts" fixture self-falsifying. | Glob now excludes the change's own directory; expectation restated at 44 against a committed measurement script. | `tasks.md` → 1.1 |
| WARNING | tasks.md | No documentation task updated `SPEC.md`, whose tracked-tasks paragraph states the tab renders items with a glyph and that "**Every other** tab is rendered by `markdown-viewer`'s markdown viewer" — both false after this change. `CLAUDE.md` makes `SPEC.md` the binding site on disagreement. | Task 7.3a added, naming the document, section, audience, and the sentence pair it replaces. | `tasks.md` → 7.3a |
| WARNING | tasks.md | Task 4.2 claimed "its two call sites" and guarded the rename with `grep -rn 'tasks::items' src`, a pattern matching **none** of the 9 `super::items(` occurrences — it would report success having checked 3 of 12 sites. | Count corrected to 12 with the split named, and the guard widened to `grep -rn 'super::items(\|tasks::items' src`. | `tasks.md` → 4.2 |
| WARNING | tasks.md | Groups 1 and 4 both change interfaces other modules consume (two new public fields; a changed public signature) with no contract-gate CHECK, which the schema mandates. | CHECK tasks 1.7 and 4.2a added before each group's VERIFY. | `tasks.md` → 1.7, 4.2a |
| WARNING | tasks.md | `wrap_plain` and `split_at_columns` in `src/ui/tasks.rs` go dead once item text routes through `inline`; `-D warnings` fails task 8.3 while they remain, and no task removes them. | REFACTOR task 4.6a added, covering both functions and the doc comments justifying their duplication. | `tasks.md` → 4.6a |
| WARNING | design.md | The matrix filed 23 carried `artifact-folds` scenarios as "the other sixteen", and filed the two amended depth-0/depth-1 scenarios as "carried unchanged, keep their tests" — so the clause covering body rows and blocks had no check that could fail. | Count corrected to 21 carried + 2 amended + 2 new against `grep -c`, and the amended and `Space` scenarios given their own matrix rows. | `design.md` → Verification Matrix |
| WARNING | design.md | Test Boundaries called the filesystem "absent" in every `src/` test, but group 5's two view scenarios drive the section walk through `Dashboard::sync_detail`, which reaches the injected artifact reader. | Row added naming the reader as **replaced** by the injected closure, with the reason a real one would break `NOIO-VIEW`. | `design.md` → Test Boundaries |
| WARNING | tasks.md | `ui::tasks::lines` short-circuits to `No tasks yet` on a zero item count, discarding a prose-only file's newly retained blocks. Reached by no task or scenario. | Task 5.3a added requiring the blocks to be drawn or the omission recorded in design.md. | `tasks.md` → 5.3a |
| WARNING | proposal.md, design.md, specs | Corpus numbers measured at `f7b6d54` when the corpus was 41 files. Eight were **stale** (files, kept, dropped, fences, items, number widths, group-level fences, files-with-a-fence) and four were **false** at any commit: truncated items (1,827/66% → 1,924/67.7%), longest dropped run (41 → **100**), plain labels (2,369 → 2,531), and items-average-three-lines (→ 4.72 body lines, so a 12-item group grows to ~68 rows, not ~40). | Swept in **one pass** across `proposal.md`, `design.md`, `specs/markdown-render`, `specs/tasks-checklist` and `tasks.md`, then re-verified by a single command. The measurement is now a committed script, `scripts/measure-tasks-corpus.py`, so the next re-measure is one command rather than a re-derivation. | all five artifacts; `scripts/measure-tasks-corpus.py` |
| WARNING | proposal.md, design.md, specs/markdown-render | "handing the fragment to a block parser … **eats its marker**" is false. Measured: `# not a heading` keeps its `#` and gains a wrong heading face; `- ` and `> ` are *replaced* by `• ` and `│ `; only a fence opener deletes anything. | Restated as restyled / re-segmented / mis-faced / dropped, in all three artifacts, with the fence named as the one deletion case. | `proposal.md`, `design.md` → Decision 3, `specs/markdown-render/spec.md` |
| SUGGESTION | proposal.md | The loss table's three sub-rows double-count: 169 `\|` table rows sit inside the fenced blocks counted above them. | Table row removed and the non-partition stated explicitly. | `proposal.md` → Why |
| SUGGESTION | proposal.md | "stays within three capabilities" while Capabilities listed five (now six). | Restated as six. | `proposal.md` → Non-Goals |
| SUGGESTION | tasks.md | Group 3 ran RED → GREEN → VERIFY → CHECK → VERIFY, putting a gate CHECK after a VERIFY. | Reordered to RED → GREEN → CHECK → VERIFY, with the escape assertions folded into the final VERIFY. | `tasks.md` → 3.3, 3.4 |
| SUGGESTION | tasks.md | `cargo test --lib tasks::` selects 97, of which 41 are `ui::tasks`; the module itself holds 56. The plan read 97 as the module's own count. | Split stated at the two sites that cite it. | `tasks.md` → 1.9 |
| SUGGESTION | tasks.md | The blast-radius plant was recorded as 18 `super::Group` literals / 20 sites; recount gives 17 / 19. | Corrected. | `tasks.md` → 1.6, Checks run at planning time |
| SUGGESTION | tasks.md | "`~~~` alone renders an empty line" — it returns a zero-element vector, which matters if a test asserts a length. | Corrected, and tied to the fence's addition to the escape set. | `tasks.md` → Checks run at planning time |

## Repairs Made During Implementation

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| SUGGESTION | tasks.md | The blast-radius plant measured **one** existing test as changing behaviour; implementation measured **two**. The plant planted the two fields and the continuation rule and stopped, so it never ran under task 1.5's blocks-only leading-group emission — which is what falsifies `tasks::tests::an_empty_document_parses_to_no_tasks_and_no_problems`' prose-only half (`groups: vec![]`). Scope, contracts and test boundaries are unchanged; only the figure was wrong, and it was wrong **low**. | Both sites restated at two, with the method named: a partial plant bounds a blast radius from below, so its figure is a floor rather than an exact count. The second test's rewrite gains a `blanks_only` case, the delta's "a document of blank lines alone yields no group at all" clause having been otherwise unpinned. | `tasks.md` → 1.6, Checks run at planning time |

## No Remaining Implementation-Blocking Gaps

None remain. All nine CRITICALs are repaired in the artifact that owns each, `openspec
validate task-item-bodies --strict` passes, and the two structural conclusions that survived
review unchanged are worth stating because they bound the work: delta fidelity is clean on
all seven MODIFIED requirements with no unnoticed revert, and the blast radius is small —
`tasks::Item` has exactly two construction sites, and a planted retention implementation
broke exactly **one** of the 1,458 existing lib tests.

No unresolved decision requires user input.

## Deferred Non-Blocking Notes

- **A second fold level** — folding an item's body rather than folding it with its section.
  Deferred in `proposal.md` → Non-Goals and argued in `design.md` → Decision 11; it needs the
  detail cursor to address items, which is a `detail-scroll` change. The re-measured growth
  figure (a 12-item group reaching ~68 rows, not ~40) strengthens that decision rather than
  changing it.
- **Corpus counts go stale by construction.** Every number in these artifacts was true at the
  commit it was measured and drifts as changes archive. `scripts/measure-tasks-corpus.py` is
  the resolution point: re-run it rather than re-deriving, and the archived spec text carries
  the shape of the claim ("every item carries a number", "no emphasised label") which does not
  drift, beside the figure that does.
- **`tests/gate_controls.rs` flaked once** during review — `4 passed; 1 failed` on one of
  three runs, green on the other two. Not this change's doing and not repaired here, but task
  8.4 and 8.5 should know it before reading a red run as a regression this change caused.
- **An unterminated fence split across an item body and a trapped item's body** renders as
  literal text in each. Measured 0 occurrences across all corpus files; recorded in
  `specs/task-groups` rather than given a rendering scenario of its own.
