## Reviewed Artifacts

- `proposal.md`
- `specs/tasks-progress-bar/spec.md` (two MODIFIED requirements)
- `design.md`
- `tasks.md`

Reviewed by four independent reviewers, one slice each, dispatched simultaneously and given the
change directory only — never a fork of the session that wrote the package, so none inherited
the assumptions under audit. Slices: (A) capability coverage, delta fidelity, scenario quality,
cross-artifact contradictions; (B) design completeness, test boundaries, and the falsifiability
audit of every proposed check; (C) task alignment, lifecycle discipline, `parallel-after`;
(D) factual verification of every empirical claim, by running the command.

The division earned its cost. The one CRITICAL that would have damaged the implementation was
found by **all four** slices independently; the second CRITICAL was found by **D alone**, because
it is invisible to anyone who reads the claim instead of re-running the command.

## Reviewed Against

- This repository HEAD: `95abc1114b5ba01ec5d2219f82f4f638b6a3ee6d`
- Sibling repositories: **Not applicable** — no contract outside this repo is touched. The
  vendored `openspec/schemas/tdd/` and `.claude/agents/` are unmodified.
- Working tree: clean apart from this change's own four planning files, intentionally included.
- MODIFIED deltas diffed against the live spec at that HEAD: **2 of 2**, by
  `git show HEAD:openspec/specs/tasks-progress-bar/spec.md`. *The progress bar's grammar* —
  live `23-159`, reproduced in full, all five HEAD scenarios byte-identical; three accounted
  differences (the East Asian Width paragraph replaced, `shade alternation` → `glyph
  alternation`, one new scenario). *The gauge is segmented by group…* — live `394-532`,
  reproduced in full, all six HEAD scenarios present; differences are the glyph table, the
  `shade` → `glyph` wording, three new prose paragraphs, six scenario assertions reglyphed, one
  new AND clause, and two new scenarios. **No unnoticed revert in either block**, and neither is
  a byte-identical carry-back that would have belonged under ADDED or nowhere. The four
  properties declared carried — legibility floor, cumulative-flooring partition, empty-group
  index rule, no-separator decision — are byte-identical apart from the two word swaps.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| **CRITICAL** | `tasks.md` | 0.3 and 1.7 required `grep -rl '⢕\|⠌' src/` to read **1** after group 1, "naming `src/ui/tasks.rs` and no other file". It reads **2**: the same group rewrites `src/ui/view.rs`'s frame test, and the scenario it implements mandates asserting the per-span pairs `█`/`⢕` and `▒`/`⠌` by name. The cheapest way to satisfy the check as written was to stop naming braille in that file — i.e. to gut the one test that would go red against a production caller handing `progress_bar` an empty slice | Expected value → **2**, naming both files; kept the "no other file" half, which is the claim actually worth checking (`src/ui/detail.rs` draws an unsegmented gauge and must gain none) | `tasks.md` 0.3, 1.7 |
| **CRITICAL** | `tasks.md` | 0.2 recorded `make gates` exits **0**. On the tree this plan produces it exits **2** — `OPENSPEC-UNTOUCHED FAIL: an untracked file exists inside openspec/: …/tasks.md`. The figure was measured before `tasks.md` was written; writing it is what turns the gate red | Recorded the true exit and the precondition: `git add openspec/changes/gauge-fill-contrast/` first, after which it exits 0 with 47 OK lines. Pointer added to 4.4 | `tasks.md` 0.2, 4.4 |
| WARNING | `specs/…/spec.md` | The grammar requirement's new scenario *Every glyph the bar can draw measures one column* was **green at HEAD**. The five glyphs are `char` literals, so `layout::columns` measures `⢕`/`⠌` whether or not the crate draws them — yet it sat in 1.1's RED list in a `behavior` group | Added a discriminating clause: the segmented run must hold ≥1 each of `█ ▒ ⢕ ⠌` and the empty-slice call ≥1 `░`. Red at HEAD. 1.1 now says which clause carries the RED | `spec.md` grammar scenario; `tasks.md` 1.1 |
| WARNING | `specs/…/spec.md` | *Segmentation is total* asserted "its **four** glyphs' counts sum to exactly `g`". The vocabulary is now **five**, and that scenario's `0..=130` sweep deliberately reaches fixtures where segmentation is skipped and the run is `█`/`░` — so the assertion was false at those widths | "five glyphs", named, with the reason the fifth appears | `spec.md` *Segmentation is total* |
| WARNING | `specs/…/spec.md` | *The fill boundary is the only change of character family* defined "that transition" two ways, one index apart: the block→braille **adjacency** versus `floor(g*25/47)`. Verified by independent reimplementation — at width 78 the adjacency is 34 and the quotient 35; at 58, 23 and 24. An implementer writing from the first two clauses gets an unattributable off-by-one RED | Reworded to the **first braille position**, with the adjacency stated as `floor(g*25/47) - 1` and both index pairs given | `spec.md` fill-boundary scenario |
| WARNING | `specs/…/spec.md` | The legibility floor's worst case counted `^## ` **headings**; `segmented_gauge` filters to `total > 0`. `agent-launch`'s 22 headings include five prose sections holding no items, so its real `n` is 17, floor 34, headroom 11 — not 44 and 1. Swept the whole archive by contributing groups: the true worst case is a **different change** | Replaced with `archive/2026-09-07-degraded-states` — 18 contributing groups, 115 items, cells `[115/115]`/`100%`, `g = 58-9-4-2 = 43` against floor 36, **seven** columns of headroom. Recorded why the old figure was wrong. Inherited from HEAD, not introduced here; no rendering ever changed, since too high a floor only skips segmentation | `spec.md` segmentation requirement (two paragraphs); `tasks.md` group 0 (two rows) |
| WARNING | `design.md` | Test Boundaries listed the Filesystem as **Absent** and named no replaced collaborator. The package's one replaced collaborator is the injected artifact reader — `testutil::RecordingReader::always` at `src/ui/view.rs:5032-5034` — and it is precisely what drives the **production** path to populate `groups` | Added an *Artifact reader — Replaced* row saying why that scenario is worth its cost | `design.md` → Test Boundaries |
| WARNING | `design.md` | The Verification Matrix carried one of the grammar requirement's six scenarios. A matrix naming only the new one is indistinguishable from one that forgot the other five | Added the five rows, annotated "carried unchanged", with their test locations and the reason no assertion in them moves | `design.md` → Verification Matrix |
| WARNING | `tasks.md`, `design.md` | 1.6 rewrites `segmented_gauge`'s doc comment from spec prose that says "span" throughout. `scripts/gates/taskseam.sh:26` greps the whole file, prose included, for `…\|Span\|…` case-sensitively — a sentence beginning "Spans are proportional…" turns `make gates` red. The existing comment says *stretch* for exactly this reason | "Keep the word **stretch**" added to 1.6; the limit recorded on design's TASKSEAM row | `tasks.md` 1.6; `design.md` → Gates |
| WARNING | `tasks.md` | 1.2 called all seven rewrites "regression rows, not new coverage". Six are; `segmentation_is_total_and_partitions_the_run_exactly` also gains the non-interleaving assertion over its full sweep, which no existing test makes | Split into 1.2 (six regression rows) and 1.3 (the seventh, named as new coverage) | `tasks.md` 1.2, 1.3 |
| WARNING | `tasks.md` | 1.7's `░`-survival leg used `grep -rc`, which prints a `path:0` line per file and so names all three view files whatever the answer is — green before the work and green if `░` were deleted | Replaced with `grep -rl … \| wc -l` → 3, naming the files, as its own task | `tasks.md` 1.8 |
| WARNING | `tasks.md` | Group 3 corrects a false claim at two sites and had **zero** executable evidence — its only VERIFY disclaimed itself, and `doc_contract` binds none of the glyph claims | Added 3.3: `grep -n '▓' SPEC.md AGENTS.md` and `grep -n 'all four' SPEC.md AGENTS.md` each select 0. Both select non-zero at HEAD, so each is red before and green after | `tasks.md` 3.3 |
| WARNING | `proposal.md` | The Why table stated the fill boundary is a 25-point step, unqualified. The alternations are in phase, so the straddling group's index decides: **odd** gives `▓`→`▒` (25 points), **even** gives `█`→`░` (75 points, already the strongest edge). The defect is real but hits about half of changes | Qualified, with the coin-flip framing — a headline edge legible only half the time is the defect | `proposal.md` → Why |
| SUGGESTION | `design.md` | Decision 3 claimed `⢕` occupies the right-hand dot column and `⠌` the left. False — decoded against the 8-dot layout, `⢕` is dots 1,3,5,8 (a checkerboard) and `⠌` dots 3,4 (a diagonal); both straddle both columns. The conclusion survives, the stated reason does not | Restated as shape difference, with the false version recorded so a later glyph swap does not reason from it | `design.md` → Decision 3 |
| SUGGESTION | `design.md` | "Modules touched: `src/ui/tasks.rs` only" contradicted group 1's own two-file list | "only **in production**", with the test module named | `design.md` → Modules touched |
| SUGGESTION | `proposal.md` | The Non-Goal said the EAW exposure is "left standing", but both requirements assert the change removes the **raggedness** and keeps only the mis-proportion | Split the two: raggedness goes as a side effect, mis-proportion stays and is recorded | `proposal.md` → Non-Goals |
| SUGGESTION | `proposal.md` | Both illustrations are labelled a 48-column gauge; neither mandated interior gives that `g` (66 at 78, 46 at 58). The rendered strings themselves are byte-correct at 48 | Labelled illustrative, with the real renders at both mandated widths given | `proposal.md` → Why / What Changes |
| SUGGESTION | `tasks.md` | Five grammar scenarios are deliberately carried unchanged and nothing said so, leaving a coverage audit unable to tell carry-forward from omission | Named all five in group 1's preamble with their test locations | `tasks.md` group 1 |
| SUGGESTION | `tasks.md` | Group 3's two change tasks carried no lifecycle marker | Prefixed `CHANGE:` | `tasks.md` 3.1, 3.2 |
| SUGGESTION | `tasks.md` | `AGENTS.md:145` ends the target paragraph and starts an unrelated `doc_contract`-bound claim on the same physical line; an in-place rewrite could delete its tail | Hazard named in 3.2 | `tasks.md` 3.2 |
| SUGGESTION | `tasks.md` | Two measured figures were slightly wrong: `filled_count` is at `:1178`, not `:1179`; `unicode-width` has three parents, not one (all via `ratatui` — the conclusion held) | Both corrected | `tasks.md` group 0 |
| SUGGESTION | `tasks.md` | The ordering note argued the two-file group boundary with source line numbers — a design decision being made in the checklist | Moved to `design.md`; a one-line pointer left behind | `tasks.md` ordering note |

## No Remaining Implementation-Blocking Gaps

None remain. Both CRITICALs are repaired and re-verified by re-running the command, not by
re-reading the claim: `make gates` exits 0 once the change directory is tracked, and the braille
post-condition now matches what a correct implementation produces.

Falsifiability, re-audited after repair. Every check in the plan can now fail:

- The two RED greps are red at HEAD, with the `⢕`/`⠌` leg's positive control recorded (it selects
  nothing on its own, and 0.5 says so rather than pretending otherwise).
- The 0.4 negative control genuinely fires — planting `// planted: ▓` into `src/ui/detail.rs`
  moved the count 2 → 3 and named that file; reverting returned it to 2 with a clean tree.
  Reproduced independently by reviewer B in a scratch copy.
- The `░` leg and group 3's prose checks, both previously green by construction, now have
  forms that go red.
- The grammar requirement's new scenario, previously green at HEAD, now carries a clause
  that is not.
- No Verification Matrix command is aimed at nothing: `cargo test --lib ui::tasks` selects **38**,
  `--lib ui::view` selects **156**, `--test doc_contract` selects **107**, `--lib ui::detail`
  selects **63**. Zero selected would be a failed check; none of them is zero.

The empty-slice guarantee — the property most at risk in a change whose scenarios otherwise all
hand-build their input — holds. `a_real_tasks_tab_renders_a_segmented_gauge_into_the_frame`
(`src/ui/view.rs:5161`) is the single test that reaches `progress_bar` through
`ui::detail::content_lines` with its fixture built by `sync_detail` through the injected reader,
and it survives the group-1 rewrite as specified. The CRITICAL was its only threat.

Baseline at HEAD, for drift detection during implementation: `cargo test --all-features` exits 0
with **1624 passed, 0 failed, 1 ignored** across ten binaries.

## Deferred Non-Blocking Notes

- **The glyph claims stay prose, not contract.** `grep -n 'gauge\|Ambiguous' tests/*.rs` selects
  0 lines, which is why the "all four are Ambiguous" claim could go wrong at three sites
  unnoticed. Binding each glyph's East Asian Width property in `tests/doc_contract.rs` needs a
  checked-in width table — the crate reaches `unicode-width` only through `ratatui` and declares
  no such dependency — and that is an argument to have on its own terms. Recorded in
  `design.md` → Decision 8, which also records what this change does instead: the grammar
  requirement's new scenario binds the property the prose is *about* (every glyph measures one
  column through `layout::columns`), leaving only the width **class** as prose.
- **The CJK mis-proportion is accepted and uncompensated.** Closing it needs every glyph in one
  width class, which means an all-braille bar whose filled half no longer reads as solid — the
  defect this change exists to repair. Recorded in `design.md` → Decision 7 and in both
  requirements.
- **Eleven absence-of-gauge assertions are partly blind to braille.** Sites in `src/ui/view.rs`
  and `src/ui/detail.rs` assert "no bar row" as `!contains('█') && !contains('░')`. Every fixture
  is `total == 0` or a header row, so none is wrong today. Not scheduled: extending them is
  unrelated to this change's behavior and would widen group 1 past the one function it edits.
