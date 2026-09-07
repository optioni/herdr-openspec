## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/artifact-content/spec.md`
- `specs/change-rows/spec.md`
- `specs/detail-header/spec.md`
- `specs/detail-scroll/spec.md`
- `specs/list-filtering/spec.md`
- `specs/markdown-render/spec.md`
- `specs/responsive-layout/spec.md`
- `specs/tasks-checklist/spec.md`
- `specs/tasks-progress-bar/spec.md`

The finding pass was delegated to **four independent reviewers**, none of them a fork of the
planning session, each given the change directory and one slice: (A) capability coverage,
scenario quality, and cross-artifact contradictions; (B) design completeness, test boundaries,
and whether each proposed check could fail at all; (C) task alignment, lifecycle discipline,
and `parallel-after` independence; (D) factual verification — every empirical claim the
artifacts make about the codebase, the dependency graph, ratatui's API, or the environment,
checked by running the command or reading the source. Reviewers reported findings and edited
nothing. This session merged them, repaired the owning artifact, and wrote this file.

Reviewer D's slice earned its place: it found four stale or wrong numbers that three document
reviews could not, and it independently reproduced the `columns` oracle defect that reviewers
A and B found by reading.

## Reviewed Against

- This repository HEAD: `d5cc8b0` (`docs(openspec): salvage in-flight proposal work from the
  session limit`)
- Sibling repository HEAD: `Not applicable` — this change touches no sibling. Its two
  vendored trees (`openspec/schemas/tdd/`, `.claude/agents/`, from `optioni/openspec-schemas`)
  are not read or written by anything here.
- Working tree: clean apart from this change's own directory and the eight other in-flight
  change proposals under `openspec/changes/`, none of which this change reads or writes.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/responsive-layout/spec.md` | The `columns` oracle — "the index of the first cell whose symbol is the buffer's reset symbol" — is wrong for every wide cluster. `set_stringn` calls `Cell::reset()` on the trailing cells of a multi-column cluster, and a reset cell is byte-identical to an untouched one, so the scan reports `1` for `日本語`, `🎉`, and the ZWJ family against a true `6`, `2`, `2`. Four of seven fixtures fail, and an implementer "fixing" `columns` to satisfy the oracle would build exactly the bug this change removes. | The oracle is now the `x` that `Buffer::set_stringn(0, 0, s, usize::MAX, Style::default())` **returns** — the cursor it advanced to, by definition the cells consumed. The scenario states the rejected oracle and why, and notes that `set_string` returns `()` and cannot serve. | Scenario "`columns` agrees with what the buffer consumed" |
| CRITICAL | `specs/responsive-layout/spec.md` | The wide-path fixture `/home/dev/日本語のリポジトリ名前です` is 23 characters and **36** columns, not the "thirty-six characters and forty-eight display columns" claimed. At 60 columns `A` is 51, so it fits whole and **every** THEN clause is unreachable — no ellipsis is drawn at either width. | Fixture replaced with `/home/dev/workspaces/日本語のリポジトリ名前がとても長いディレクトリ` — 44 characters, **67** columns, measured — chosen to exceed `A` both badged (41) and unbadged (51). A new clause states what a `char`-counted shortening would have kept, so the scenario discriminates between the two measures instead of merely exercising one. | Scenario "A wide-character path is shortened by columns and stays inside the header" |
| CRITICAL | `specs/change-rows/spec.md` | Same defect: `/home/dev/日本語のディレクトリ名前です` is **38** columns and the block shortens against interiors of 38 and 58, so "begins with `…`" is false at both widths. | Fixture replaced with a **65**-column path, stated with the reason it must exceed the wider interior, plus a clause naming what a `char` count would have kept at each width. | Scenario "The no-repository block shortens its search path by columns" |
| CRITICAL | `specs/artifact-content/spec.md` | The narrow-frame expectations confused frame width with content width. Content area is frame − 2, so frame 15 → `No content y…`, 14 → `No content …`, 13 → `No content…`. The spec asserted `No content…` at 15, `No content y…` at 14, and `No content …` at 13 — the last two **exceeding** their content area, contradicting the same scenario's own bound and reintroducing the overflow the change exists to remove. | All three corrected, with the frame-versus-interior relationship stated inline, and 1x20/2x20 given their own clause. The requirement's prose now says every `width` in it is the content area's. | Requirement "The content area renders the artifact, its problems, or `No content yet`" and its narrow-frame scenario |
| WARNING | `specs/change-rows/spec.md`, `specs/tasks-checklist/spec.md`, `specs/detail-header/spec.md` | Three capabilities used ADDED where MODIFIED was required. A MODIFIED block replaces a requirement; an ADDED one leaves the original standing — so the archived specs would have kept "counted in `char`s", "exactly its width in characters", and "every row of every kind SHALL be exactly `width` characters, so a caller may index them without a bounds check", each contradicting the new requirement in the same file. | Four requirements extracted **verbatim** from the live specs and added as MODIFIED blocks, then edited sentence by sentence: `change-rows`' separator/archived and empty-state requirements, `tasks-checklist`' line grammar, `detail-header`'s header row. The bounds-check guarantee is explicitly **narrowed** rather than silently dropped — a caller may take the first *n* columns with `truncate_columns`, and may not index by `char` or byte. | `specs/change-rows/spec.md`, `specs/tasks-checklist/spec.md`, `specs/detail-header/spec.md` → `## MODIFIED Requirements` |
| WARNING | `specs/markdown-render/spec.md` | The new drop-a-cluster rule silently contradicted the landed code-block requirement, which promises verbatim reproduction and that "a split preserves every character". Content loss inside a block whose own requirement forbids it. | That requirement added as a MODIFIED block with an explicit, bounded carve-out: the split is at a grapheme boundary in columns, and exactly one cluster is dropped, only when it is wider than the whole region. At every width ≥ 2 and for every width-1 cluster the verbatim promise is unchanged. | `specs/markdown-render/spec.md` → "Code blocks and raw HTML are reproduced verbatim and hard-split" |
| WARNING | `design.md`, `specs/responsive-layout/spec.md` | **Decision 7 had the ZWJ direction backwards.** It claimed ratatui budgets the sum of a sequence's clusters while terminals compose it, so the pane over-reserves. Measured: `graphemes(true)` treats a ZWJ sequence as **one** cluster and ratatui measures it at **2** — agreeing with a composing terminal. The real divergence is a *non*-composing terminal painting six columns, so the pane **under**-reserves, which the unconditional "no line ever exceeds its region" forbade. | Decision 7 rewritten, naming the correction. The Unicode promise is now scoped — "never exceeds, **as ratatui measures it**, the region" — with the non-composing terminal named as an accepted, uncompensatable limit, alongside its sibling case (East Asian Ambiguous characters under a CJK locale). The proposal bullet and Test Boundaries carry the same scoping. | `design.md` → Decision 7, Test Boundaries; `specs/responsive-layout/spec.md` → the Unicode promise; `proposal.md` → What Changes |
| WARNING | `design.md`, `specs/change-rows/spec.md` | `ui::list::truncate_left` **does not exist**. The real functions are `shorten_left` and `shorten_left_row`; the latter is a third measuring site named nowhere. A spec mandated behaviour for a function that is not there. | Renamed at all five sites, and `shorten_left_row` given its own sentence in both the spec and Decision 3 — un-padded sibling for the header, padded form for the no-repository row, both measuring in columns. | `design.md` → Boundaries, Contracts, Decision 3; `specs/change-rows/spec.md` → ADDED requirement and empty-state MODIFIED block |
| WARNING | `specs/responsive-layout/spec.md` | `truncate_columns`' byte-offset derivation was unspecified, and its own scenario could not catch the natural wrong implementation. `styled_graphemes` drops control clusters, so summing returned symbol lengths yields an offset shifted by the dropped bytes — reviewer B measured a real panic (`byte index 5 is not a char boundary`) on `ab`+BEL+`日本語`, violating the function's own no-panic promise. | The derivation is now normative — each symbol's own byte offset within the original `&str`, never a running sum — and a BEL fixture added to the scenario with the reason spelled out. | Requirement "Display width is measured in terminal columns by one pair of primitives" and its `truncate_columns` scenario |
| WARNING | `design.md`, `tasks.md` | **The COLWIDTH positive control never exercised the sweep's own regex.** It grepped `layout.rs` for `cell_width`/`styled_graphemes` — different patterns — so a corrupted sweep regex would print `COLWIDTH OK` over a tree full of violations. Decision 9's claim about it was false. | The script now runs its **own sweep pattern** against a synthesised line holding all three forms and fails when it does not match; the `layout.rs` check is kept, demoted to proving the exemption is not vacuous. Both controls re-run at planning time: the self-test fires on a corrupted pattern (exit 1), and the script still exits 1 on control 2 at HEAD. Task 8.3 now carries three negative controls instead of two. | `tasks.md` gate block and 8.3; `design.md` → Decision 9 |
| WARNING | `design.md`, `tasks.md` | The width gates (`LISTWIDTHS`, `MDWIDTHS`, `TASKWIDTHS`, `DETAILWIDTHS`, `WIDTHS`) require **every** `#[test]` in their file to name both mandated widths as bare literals and each states it has **no exemption list**. Roughly eight new sweep and narrow-frame tests name neither, so each would fail its gate on arrival — and groups 2, 3, and 4 never ran their own gate, so the break would surface three to six groups downstream. Decision 4 asserted the change "needs no gate edit at all". | Decision 4 gained a "What this does cost" section naming the collision and the remedy `taskwidths.sh`'s own header records. `tasks.md` gained a header block requiring every new test to name its module's mandated pair bare, and groups 2, 3, and 4 now run `listwidths.sh`, `mdwidths.sh`, and `taskwidths.sh` in their verification task. | `design.md` → Decision 4; `tasks.md` header and 2.5 / 3.5 / 4.4 |
| WARNING | `design.md` | Test Boundaries omitted four collaborators `run_loop` actually holds — `refresh::Refresher`, `agents::AgentPoll`, `launch::Launcher`, and `EventSource` — the last already named in the matrix's own Collaborators column with no boundary row. `Launcher` matters most: `herdr agent start` blocks up to thirty seconds. | Four rows added, each stating replaced and how. A closing paragraph states the one thing no tier here can observe — a terminal shaping a cluster differently from ratatui — and why no tier would buy that evidence. | `design.md` → Test Boundaries |
| WARNING | `design.md` | Gate floors were left where they are, though the change only adds tests and AGENTS.md requires a gate's default to be its **true measured floor**, not a value that happens to pass. | New task 8.6 re-measures each `#[test]` count and raises `LIST_MIN`, `MD_MIN`, `TASK_MIN`, `DETAIL_MIN`, `WIDTHS_MIN`; Decision 4 records the rule and today's measured values. | `tasks.md` → 8.6; `design.md` → Decision 4; `proposal.md` → Impact |
| WARNING | `design.md`, `specs/responsive-layout/spec.md` | Task 6.3 rewrites the footer's budget and COLWIDTH forces every `chars().count()` out of `src/ui/view.rs`, but no delta covered the footer. The footer's hint literals are ASCII and their stated counts stay true, but the **query** in it is the reader's own text and is not ASCII-bound. | The display-width requirement now states that the footer's budget and the filter prompt's keep-the-tail truncation are measured in columns, names both owning requirements, and records that no hint literal's length changes and no landed footer assertion moves. | `specs/responsive-layout/spec.md` → the confinement requirement |
| WARNING | `tasks.md` | Reviewer D: `#[test]` count for `src/ui/tasks.rs` recorded as **46**; the real count is **16**. Task 4.3 named a quantity that does not exist. Compounding it, tasks 2.4/3.5/4.3/5.3/6.4 labelled `#[test]` counts as "existing width assertions", which are different numbers entirely. | Both corrected against re-run commands. The header block now carries three separate, separately-commanded number sets: 29 production sites, the per-file **assertion** counts (list 8, markdown 11, tasks 11, detail 13, view 18), and the `#[test]` counts (tasks 16), with a line saying which is which. Each REFACTOR task now names its assertion count. | `tasks.md` header and 2.4 / 3.4 / 4.3 / 5.3 / 6.4 |
| WARNING | `tasks.md` | Task 8.1 instructed running `sh scripts/gates/colwidth.sh` "at HEAD", where it exits **127** — the file does not exist until 8.2 creates it — and by group 8 the control is already satisfied, so the step is not re-runnable where it sits. | 8.1 rewritten to confirm the recorded RED evidence by running the body from a scratch path before 8.2 creates the file; the header block states the same. | `tasks.md` → gate block and 8.1 |
| WARNING | `tasks.md` | Group 10 (Documentation) was `operational` but carried no CHECK/CHANGE/VERIFY lifecycle, and no VERIFY at all — though `tests/degraded_coverage.rs` and `tests/spec_purposes.rs` bind `SPEC.md` to named tests, so a doc edit there is not risk-free. | Rewritten as 10.1 CHECK (enumerate the sentences to correct) → 10.2–10.4 CHANGE → 10.5 VERIFY (`cargo test --test spec_purposes`, `--test degraded_coverage`). | `tasks.md` → group 10 |
| SUGGESTION | `tasks.md` | A `CHECK:` marker sat mid-`behavior` group (3.4), mixing operational vocabulary into a RED→GREEN→REFACTOR lifecycle; groups 3 and 7 had no REFACTOR step, unlike every other behavior group. | The `mdseam` check folded into group 3's unlabelled verification tail; groups 3 and 7 each given a REFACTOR task, group 1's "otherwise record that none was needed" escape included. | `tasks.md` → 3.4 / 3.5, 7.4 |
| SUGGESTION | `tasks.md` | The parallelism note framed groups 1–8 as a flat chain, when the real edge is 1 → {2..7} → 8, all blocked only by whole-crate compilation. | Note rewritten to state the shape, so a future worktree-isolated run knows what is genuinely orderable. | `tasks.md` → header |
| SUGGESTION | `proposal.md` | The 120x10 damage description overstated what persists. `render_body` draws the list then the detail region, and a `Block` never clears interior symbols — so the erased **list** border at column 39 is permanent, but the detail block's left border and the detail header row are both repainted later in the same frame. Also, `[4/9]` is pushed into the detail region, not "off-screen". | Rewritten with the measured arithmetic (48 columns from a 38-column region), naming which overwrite persists, which is repaired, and where the visible corruption actually lands — with a sentence on why the precision matters to anyone reproducing it. | `proposal.md` → Why |
| SUGGESTION | `proposal.md`, `design.md` | "All 269 files under `openspec/changes/` and `openspec/specs/`" matches no count at any commit (290 at `d1942bb`, 298 at HEAD, 353 in the tree). | The count is dropped; the claim now rests on the **set** — exactly thirty distinct non-ASCII codepoints, each measured at 1 — which reviewer D confirmed by set difference. `design.md` says why the count was removed. | `proposal.md` → Why; `design.md` → Context |
| SUGGESTION | `proposal.md` | "roughly twenty-five production sites" against a measured **29**. | Corrected to 29, marked as measured. | `proposal.md` → What Changes |
| SUGGESTION | `design.md`, `specs/tasks-checklist/spec.md` | Off-by-one prose in three places: design's Context said the literals overflow "at frames narrower than 15 and 13" (the true frames are 15 and 13 **and narrower**), and `tasks-checklist` said `No tasks yet` overran "below 13" and truncates to `No tasks ye…` at 12 — it is 12 columns, so it **fits** at 12 and first truncates at 11. | All corrected, each stating that the widths quoted are the content area's. The `No tasks yet` scenario gained 14x20 (fits whole) and 15x20 (padded) so the truncation boundary is sampled on both sides. | `design.md` → Context; `specs/tasks-checklist/spec.md` → the `No tasks yet` requirement and its narrow-frame scenario |
| SUGGESTION | `specs/responsive-layout/spec.md`, `design.md` | `ui::markdown`'s `split_at_char` is a `char_indices` column budget that none of COLWIDTH's three patterns can see; widening the pattern would need the exemption list the gate exists without. | Recorded as a stated limit in both the requirement and Decision 9, naming what does prove those sites were rewritten (`markdown-render`'s wide-character and 500-column-CJK scenarios). Task 3.2 now names `split_at_char` explicitly. | `specs/responsive-layout/spec.md`; `design.md` → Decision 9; `tasks.md` → 3.2 |
| SUGGESTION | `specs/tasks-progress-bar/spec.md` | The clause "both results' `layout::columns` equals their `chars().count()`" is a tautology for an all-width-1 fixture and cannot fail, yet was framed as a proof. | Replaced with a byte-identical comparison against the pre-change output, which does fail if the gauge run was recomputed against a different measure. | Scenario "The full grammar at both mandated interior widths" |
| SUGGESTION | `specs/list-filtering/spec.md`, `design.md` | Scenario "The fold is total and allocates no surprise" asserted nothing about allocation. | Renamed to "The fold is total and its documented edge cases hold", in the spec and in the verification matrix. Reviewer D independently confirmed all four linguistic claims in it by compiling a throwaway program: `ΟΔΟΣ` → `οδος` with a final U+03C2, `İ` → `i`+U+0307, and NFC/NFD `ä` not matching each other. | `specs/list-filtering/spec.md`; `design.md` → Test Strategy |
| SUGGESTION | `specs/responsive-layout/spec.md` | The confinement rule was stated broadly ("no file under `src/ui/`") while the enforceable list covered seven files, excluding `src/ui/mod.rs` and `src/ui/terminal.rs`. | The requirement now states the seven **are** the whole reach, with the verified reason: neither excluded file renders, and neither holds such a measurement in production code (`awk | grep -c` reports 0 for both). | Requirement "Display width is measured in terminal columns by one pair of primitives" |
| SUGGESTION | `design.md`, `tasks.md` | Design risk "a 0..=130 sweep is slow — measured before merging" had no task; scenario coverage lost track once the MODIFIED blocks carried their landed scenarios forward. | Sweep timing folded into 11.1 with the stated narrowing rule. The verification matrix extended from 59 rows to **82**, one per scenario, and 9.3's count corrected from 58 to 82 with the carried-forward split named. | `tasks.md` → 11.1, 9.3; `design.md` → Test Strategy |

Confirmed by reviewers and left unchanged, recorded so a later reader does not re-litigate
them: all 7 original MODIFIED blocks reproduce their live requirement in full with
byte-identical headers; every `file:line` reference in the plan is accurate; the 29-site count
and its per-file breakdown reproduce exactly; `unicode-width 0.2.2` and
`unicode-segmentation 1.13.3` are already in `Cargo.lock` and in all four triples of
`tests/fixtures/build-graph.txt`; `deps.sh` asserts exactly six normal dependencies and
`plugin-build`/`quality-gates` hard-code that six; `noio-view.sh`'s `PURE` list is exactly
eight files; every kind marker is present and correct and every behavior group opens with RED;
group 11's commands byte-match the `Makefile`'s recipes; and `tasks.md` (2119 words) is well
under `design.md` (5177 words).

**Decision 1 was independently re-verified twice** — once by reviewer B, which built a scratch
crate against ratatui 0.30.2 and measured that summing `cell_width()` over `styled_graphemes`
returns exactly `set_stringn`'s consumed `x` for all eight probe strings, and once by the team
lead against the vendored source. `Span::styled_graphemes` and `CellWidth` are public and
re-exported (`ratatui/src/lib.rs:517` and `:480`), and `set_stringn` at
`ratatui-core-0.1.2/src/buffer/buffer.rs:352` performs `.map(|symbol| (symbol, symbol.cell_width()))`
— literally the same call, not an approximation. The rejection of a direct `unicode-width`
dependency is also confirmed factually: `str::cell_width` is `UnicodeWidthStr::width` **plus**
`count_halfwidth_sound_marks`, and `Span::width()` is the bare form. **The dependency count
stays at six and `deps.sh` is untouched.**

## No Remaining Implementation-Blocking Gaps

None remain. All four CRITICAL findings are repaired in the artifact that owned them, every
WARNING is repaired rather than accepted, and every SUGGESTION is either repaired or recorded
as a stated limit with the evidence that covers it instead. `openspec validate view-fidelity
--strict` reports the change valid, the verification matrix carries one row per each of the 82
spec scenarios, and both of the COLWIDTH gate's controls were re-run at planning time and
observed to fire.

No unresolved decision requires user input. The four findings the change owns each have a
decided behaviour: display columns through ratatui's own measure (Decision 1), the two
literals padded and proved by an all-widths sweep (Decision 4), `Enter` a no-op at the detail
route (Decision 5), and the filter folding all of Unicode (Decision 6).

## Deferred Non-Blocking Notes

- **The display-width primitives do not get their own capability or module.** They land in
  `ui::layout` under `responsive-layout` because a ninth file would move the pure-view-set
  count that `dashboard-loop` owns and `noio-view.sh` enforces, and this change does not own
  `dashboard-loop`. Resolution point recorded in `design.md` → Decision 2, which states what
  moving them would cost.
- **`split_at_char` and any future `char_indices` cut are invisible to `COLWIDTH`.** Widening
  the pattern would need the exemption list the gate deliberately lacks. Resolution point:
  `design.md` → Decision 9, with the scenarios that do cover those sites named.
- **A terminal that shapes a cluster differently from ratatui can still overrun a region.**
  Accepted, not deferred work: it is undetectable from a process writing to a pty. Recorded in
  `specs/responsive-layout/spec.md`'s Unicode promise, `design.md` → Decision 7, and Test
  Boundaries, which states that no tier in this plan could observe it.
