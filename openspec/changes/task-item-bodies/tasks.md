## 1. Retention in `tasks::parse`

<!-- kind: behavior -->

- [x] 1.1 CHECK: Generate `tests/fixtures/task-counts.txt` — one `path<TAB>completed<TAB>total` row per `tasks.md` under `openspec/changes/`, produced by `count` **at HEAD** — and commit it before touching `parse`. The fixture is the independent side of "Retention leaves every count in the archive unmoved"; produced after the edit it would agree with a wrong implementation by construction. The glob SHALL exclude `openspec/changes/task-item-bodies/` — the change's own `tasks.md` is one of the 45 files the glob matches today, and its pair moves every time a box in it is ticked. Expect **44** rows (`python3 scripts/measure-tasks-corpus.py --exclude task-item-bodies` → `files 44`).
- [x] 1.2 RED: Write failing unit tests in `src/tasks.rs` for `an_items_continuation_lines_are_retained_as_its_body`, `a_nested_sub_task_is_a_sibling_item_and_takes_its_own_body`, `a_fenced_block_indented_under_an_item_is_that_items_body`, `a_dedented_line_ends_the_body_and_is_not_attributed_to_the_item`, `an_item_with_nothing_after_it_carries_an_empty_body`, `a_groups_lifecycle_marker_is_retained_as_a_block_before_its_first_item`, `a_fenced_block_at_group_level_is_retained_between_the_items_it_sits_between`, `a_group_with_no_interstitial_content_carries_no_blocks`, `prose_between_items_is_retained_as_a_block`, and `a_checkbox_inside_a_fenced_block_is_an_item_not_body_text`. Each must fail because `Item` has no `body` and `Group` no `blocks` — not because a fixture is wrong.
- [x] 1.3 RED: Write `tests/task_corpus.rs` with `retention_leaves_every_count_in_the_archive_unmoved` (every file's `parse(text).progress()` against both `count(text)` and the 1.1 fixture row) and `every_retained_line_appears_exactly_once` (the partition, asserted in both directions). Each must print the file count it swept and fail on a zero sweep: a glob under a mistyped path exits clean.
- [x] 1.4 GREEN: Add `Item.body: String` and `Group.blocks: Vec<Block>` with `Block { text: String, after: usize }`, and implement `task-groups`' continuation rule in `parse` — a line continues the preceding item when blank or indented strictly past that item's `indent`; the body ends at the first heading, the first checkbox line at any indent, or the first non-blank line at or below it; trailing blanks stripped. `count` is not touched.
- [x] 1.5 GREEN: Emit the headingless leading group when it carries blocks but no items (design.md → Decision 1a). Measured: 27 of 44 corpus files carry a non-blank preamble totalling 736 lines, all of which the existing suppression rule drops, which is what 1.3's partition test fails on.
- [x] 1.6 CHANGE: Update the construction sites the two new fields break. Measured by planting the fields at planning time: **1** production site (`close_group`), **1** test helper (`fn item`), and **17** `super::Group { … }` literals in `src/tasks.rs`'s own tests — and exactly **one** existing test changes behaviour, `tasks::tests::prose_between_items_is_dropped_and_does_not_split_a_group`, which now sees a block and is rewritten against the delta's amended scenario. **Corrected at implementation: two did.** The second is `tasks::tests::an_empty_document_parses_to_no_tasks_and_no_problems`, whose prose-only half asserted `groups: vec![]` — under Decision 1a that prose is now a leading group carrying one block. The planning plant measured the fields alone, **before** 1.5's emission rule, which is why it could not see this one; the figure is a floor on the blast radius, not a ceiling. Its rewrite gains a `blanks_only` case beside it, the delta's "a document of blank lines alone yields no group at all" clause being otherwise unpinned.
- [x] 1.6a GREEN: Carry an open fence across a blank line in both a body and a block (`task-groups` → "A fenced block survives the blank line inside it"). Measured: 46 of 112 column-zero fenced blocks in the corpus contain a blank line, and a blank-terminates-always rule shreds every one of them into unpaired delimiters that `ui::markdown::lines` reflows as prose.
- [x] 1.7 CHECK: Contract gate — `tasks::Item` and `tasks::Group` are public and consumed outside this module. Re-inspect the diff for every consumer of both types and confirm the two added fields break no caller silently: `grep -rn 'tasks::Item\|tasks::Group\|\.items\b' src tests` and read each hit.
- [x] 1.8 REFACTOR: Fold the body/block accumulation into one named helper if the loop reads as three interleaved state machines, or record that the single pass is clearer left inline.
- [x] 1.9 VERIFY: `cargo test --lib tasks::` — 97 selected at HEAD (56 of them `tasks::tests::` proper; `tasks::` also selects the 41 in `ui::tasks`) plus the ones this group adds — and `cargo test --test task_corpus`, both green.

## 2. `tasks::task_number_len`

<!-- kind: behavior -->

- [x] 2.1 RED: Write failing tests for `a_numbered_item_reports_its_numbers_width`, `an_unnumbered_item_reports_zero`, and `a_malformed_number_is_not_a_number`, plus `the_exposed_skip_agrees_with_the_one_label_of_performs` in `tests/task_corpus.rs` over every archived item text. Each must fail because the function does not exist.
- [x] 2.2 GREEN: Expose `pub fn task_number_len(text: &str) -> usize` over the existing `skip_task_number`, and make `label_of` read the same helper rather than a second copy — `specs::clause_of` calling `tasks::role_of` is the shape to match. Confirm `label_of`'s own 97-test module stays green unmodified.
- [x] 2.3 VERIFY: `cargo test --lib tasks::` and `cargo test --test task_corpus` — green, the corpus test reporting a non-zero count of labelled items compared (expect ~2,500; a zero comparison is a failed check).

## 3. `ui::markdown::inline`

<!-- kind: behavior -->

- [x] 3.1 RED: Write failing tests for `a_fragments_inline_faces_are_set_and_its_text_is_unchanged`, `a_leading_block_marker_is_literal_text_not_a_block`, `a_fragment_wraps_and_hard_splits_exactly_as_a_paragraph_does`, and `empty_whitespace_and_zero_width_inputs_return_nothing`, each at widths `78` and `58`.
- [x] 3.2 GREEN: Add `pub fn inline(text: &str, width: u16) -> Vec<Line>` in `src/ui/markdown.rs`, inserting the CommonMark backslash escape immediately before the character that opens the block and only on a genuine block opener (design.md → Decision 4). The escape goes **after** a digit run, never before it; a bare `**` or `*` gets none; a leading `` ``` `` gets one. The `pulldown-cmark` option set does not move.
- [x] 3.3 CHECK: Re-run `/bin/sh scripts/gates/mdseam.sh` and `/bin/sh scripts/gates/mdwidths.sh` — the second requires every `#[test]` in `src/ui/markdown.rs` to name both `58` and `78` as unsuffixed literals, against a floor of `MD_MIN=35` with 40 present at HEAD.
- [x] 3.4 VERIFY: `cargo test --lib ui::markdown::` — 40 tests at HEAD plus this group's, green — and confirm the two failure modes design.md → Decision 4 measured: no rendered row begins with a literal `\`, and `inline("**RED**: x", 78)`'s leading segment carries `face.strong`.

## 4. The checklist grammar becomes `group_body`

<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests at widths `78` and `58` for `a_folded_group_and_an_unfolded_one_render_the_same_item_lines`, `an_items_body_is_drawn_under_it_at_its_hanging_indent`, `the_hanging_indent_falls_after_the_task_number`, `the_number_hang_is_dropped_before_the_prefix_is`, `a_fenced_block_in_an_items_body_renders_as_code_not_as_vanished_text`, `a_groups_block_renders_between_the_items_it_sits_between`, `an_items_inline_markdown_is_faced_rather_than_shown_as_markers`, `an_emphasised_label_degrades_to_unlabelled_rather_than_mis_coloured`, and `a_body_is_dropped_whole_with_the_prefix_it_hangs_from`. The planting probe under "Checks run at planning time" reproduces four of these failures at HEAD.
- [ ] 4.2 GREEN: Rename `ui::tasks::items` to `group_body(group: &crate::tasks::Group, width)` and move **all twelve** call sites: 2 production (`ui::tasks::lines`, `ui::detail.rs:775`) and 10 in tests (9 `super::items(` occurrences across 8 lines of `src/ui/tasks.rs`, plus `src/ui/detail.rs:4775`). The guard is `grep -rn 'super::items(\|tasks::items' src` — the narrower `tasks::items` pattern matches none of the `super::items(` form and would report success having checked 3 of 12.
- [ ] 4.2a CHECK: Contract gate — `group_body`'s signature changes from `&[Item]` to `&Group`. Re-inspect the diff at every one of the twelve sites and confirm no caller silently keeps flattening to an item slice, which is the defect this rename exists to remove.
- [ ] 4.3 GREEN: Render an item's text through `ui::markdown::inline` at `width - hang`, applying `tasks::label_of`'s offsets only to a first row whose leading segment is `Face::plain()` and long enough to hold `start + len` on character boundaries, and rendering unlabelled otherwise. A checked item stays one `muted` segment per row with inline faces dropped.
- [ ] 4.4 GREEN: Hang continuation and body rows at `prefix_len + tasks::task_number_len(&item.text)`, dropping the number hang whole back to `prefix_len` when it would leave no text column, before the existing indent-then-glyph prefix chain applies.
- [ ] 4.5 GREEN: Draw each item's `body` through `ui::markdown::lines` at `width - hang` prefixed by `hang` spaces, and each of the group's `blocks` through the same function at the full `width` with no prefix and one blank row either side, interleaved at the recorded `after` position. A group with no blocks emits no blank row. An item whose prefix has degraded to the glyph-only or truncated-glyph form contributes **no** body rows at all (design.md → Decision 9).
- [ ] 4.6 CHANGE: Update the existing `src/ui/tasks.rs` tests that pin the retired rules — `Line::text()` byte-identity (design.md → Decision 8) and the unfaced item text. Run `cargo test --lib ui::tasks::` after 4.3 and take the failing set as measured rather than predicted; 41 tests exist at HEAD.
- [ ] 4.6a REFACTOR: Remove `wrap_plain` (`src/ui/tasks.rs:395`) and `split_at_columns` (`:363`) once item text routes through `inline`, together with the doc comments justifying their duplication of `ui::markdown`'s own helpers. Both go dead at 4.3 and `-D warnings` fails task 8.3 while they remain.
- [ ] 4.7 CHECK: `/bin/sh scripts/gates/taskwidths.sh` and `/bin/sh scripts/gates/taskseam.sh` — the first requires every `#[test]` in the file to name both `58` and `78` against a floor of `TASK_MIN=22`, the second forbids a `ratatui` type or a filesystem edge and requires a live `tasks::parse(` call outside comments.
- [ ] 4.8 VERIFY: `cargo test --lib ui::tasks::` — green, no regression in the 41 tests HEAD carries.

## 5. The section walk stops flattening the parse

<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing view tests for `an_open_tracked_tasks_section_draws_item_bodies_and_group_blocks` and `collapsing_a_tracked_tasks_section_hides_its_bodies_and_blocks_with_its_items`, rendered at 120x20 and 60x20, plus `a_foldable_tasks_tab_draws_its_groups_as_fold_headers` re-run as regression. Every row the first adds must carry `ContentKind::Body` and resolve to no section under `section_at`.
- [ ] 5.2 GREEN: In `content_lines`' tracked-tasks branch, replace the `flat_map(|g| g.items)` flatten with a walk calling `group_body` per parsed group at `body_width`, so bodies and blocks reach the screen. `bar_lines` and the separator row keep the full `width` as `section-body-indent` left them.
- [ ] 5.3 CHANGE: Update whichever existing `src/ui/app.rs`, `src/ui/detail.rs`, and `src/ui/view.rs` tests the new rows break. `src/ui/app.rs:3887` asserts no row contains `Intro prose.` on a tracked-tasks tab and is the known one; take the rest from a `cargo test --lib` run after 5.2 rather than predicting them.
- [ ] 5.3a GREEN: `ui::tasks::lines` short-circuits to `No tasks yet` on `progress().total == 0` (`src/ui/tasks.rs:590`), which now discards a prose-only file's retained blocks. Draw the blocks beneath that row, or record in design.md why a file with no items renders none of its prose.
- [ ] 5.4 CHECK: `/bin/sh scripts/gates/detailwidths.sh`, `/bin/sh scripts/gates/widths.sh`, `/bin/sh scripts/gates/colwidth.sh`, and `/bin/sh scripts/gates/noio-view.sh` — every `#[test]` in `src/ui/detail.rs` must name `58` and `78`, every one in `src/ui/view.rs` must name `60` and `120`, and neither file may gain a `.chars()` measurement or an I/O API.
- [ ] 5.5 VERIFY: `cargo test --lib -- ui::detail ui::view ui::tasks ui::app` — green.

## 6. Change Review

<!-- kind: operational -->

- [ ] 6.1 CHECK: Dispatch an independent reviewer (not a fork of the implementing session) against proposal.md, all five delta specs, design.md, and tasks.md with the diff. Concentrate on: a corpus sweep that selects zero files and passes vacuously; the count fixture regenerated after the parse edit rather than before it; a body or block wrapped at `width` and then prefixed, overflowing an interior the region does not clip; the `inline` escape leaking a backslash into rendered text; and `label_of`'s byte offsets applied to a faced segment.
- [ ] 6.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 6.3 VERIFY: Confirm no blocking or unowned finding remains.

## 7. Documentation

<!-- kind: operational -->

- [ ] 7.1 CHECK: Run `grep -rn 'ui::tasks::items' SPEC.md CLAUDE.md openspec/specs/` and confirm it returns **10** lines, the count measured at planning time rather than read off. Two are this group's to rewrite by hand (`CLAUDE.md:126`, `openspec/specs/tasks-checklist/spec.md:10` — a `## Purpose`, which no delta can reach); the other eight sit in requirement bodies that **this change's own deltas replace at archive** (`tasks-checklist` ×6, `artifact-folds:1064`, `artifact-content:399`) and must NOT be hand-edited.
- [ ] 7.2 Rewrite in `CLAUDE.md`: the tracked-tasks paragraph of "Current repo state" (audience: any agent opening this repo). Replace the `ui::tasks::items` clause with `group_body` and state that an item's body and a group's blocks now render beside its items — one sentence replaced, not added, because the sentence it replaces becomes false at this commit.
- [ ] 7.3 Rewrite in `openspec/specs/tasks-checklist/spec.md`: `## Purpose` (audience: anyone implementing against the capability). A `## MODIFIED Requirements` delta cannot reach a Purpose and `tests/spec_purposes.rs` only checks it is non-empty, so nothing else would catch it — the same in-place edit `section-body-indent` → 3.2 made for the same reason.
- [ ] 7.3a Rewrite in `SPEC.md`: the tracked-tasks paragraph (audience: anyone implementing against the design contract). It states the tab renders "task groups under their headings with a `[✓]`/`[ ]` glyph per item" and that "**Every other** tab is rendered by `markdown-viewer`'s markdown viewer" — both false once item text, bodies, and blocks route through `ui::markdown`. `CLAUDE.md` itself makes this the binding site: where prose and `SPEC.md` disagree, `SPEC.md` wins. One sentence pair replaced, not added.
- [ ] 7.4 VERIFY: `grep -rn 'ui::tasks::items' SPEC.md CLAUDE.md` returns nothing and `grep -n 'ui::tasks::items' openspec/specs/tasks-checklist/spec.md` no longer matches line 10, while the eight delta-owned matches remain untouched — the deltas fold in at archive, not here. `/bin/sh scripts/gates/openspec-untouched.sh` exits 0.

## 8. Lint & Verify

<!-- kind: operational -->

- [ ] 8.1 CHECK: Inspect the intended verification commands and affected tiers — `tasks`, `ui::markdown`, `ui::tasks`, `ui::detail`, `ui::view`, and `ui::app` unit tests, the new `tests/task_corpus.rs` tier, and the `TASKSEAM`, `TASKWIDTHS`, `MDSEAM`, `MDWIDTHS`, `DETAILWIDTHS`, `WIDTHS`, `COLWIDTH`, and `NOIO-VIEW` gate scripts.
- [ ] 8.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 8.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 8.4 VERIFY: `make gates` — every script exits 0.
- [ ] 8.5 VERIFY: `cargo test --all-features` — green, at or above the 1,458 lib tests HEAD carries plus this change's, and `tests/task_corpus.rs` reporting a non-zero swept-file count.
- [ ] 8.6 VERIFY: `make coverage` — both the 80% total floor and the production-slice floor hold with no exclusion added.
- [ ] 8.7 VERIFY: `openspec validate task-item-bodies --strict` — passes.

## Ordering

Sequential. `openspec/config.yaml` records the standing parallelism veto — one crate, one
compile, and `make gates`, `make lint`, and the whole `--lib` suite each sweep the entire tree
— so criterion 3 fails for every pair in every change here and the pair walk is skipped per
that rule. Two pairs are in any case ruled out before the gate is reached: groups 1 and 2 both
edit `src/tasks.rs` and `tests/task_corpus.rs`, and groups 4 and 5 both edit `src/ui/tasks.rs`'s
callers — a shared file is the first criterion that fires for each.

No outer-loop acceptance group. Nothing in this change alters a client-visible interface whose
end-to-end **wiring** is the risk: `ui::detail` → `ui::tasks` → `ui::markdown` is already
wired, and every scenario this change adds is a grammar question inside one of those three.
Group 5's two view tests are the end-to-end tier and sit where the dependency order puts them,
not at the front.

## Checks run at planning time

All against HEAD `7d6edf3`, working tree clean before and after (`git status --porcelain`
empty).

**RED probe — groups 4.1 and 5.1.** Planted a temporary `mod probe` in `src/ui/tasks.rs`
driving `ui::tasks::lines` — the outermost function reachable today — at widths `78` and `58`;
reverted after running.

```
cargo test --lib ui::tasks::probe:: -- --nocapture
```

Exit non-zero, **4 of 4 tests executed and failed**, each having selected 3–5 rendered rows
(printed, so a probe rendering nothing would have failed its own `selected > 0` assertion):

| Probe | Selected | Failure at width 78 |
|---|---|---|
| `probe_body_is_retained` | 4 rows | `item body not drawn` — rows end at `[ ] 1.1 RED: write the test`; `covering the degraded path` is absent |
| `probe_group_block_is_retained` | 5 rows | `group block not drawn` — rows are bar, blank, `## 1. G`, `[ ] 1.1 a`, `[ ] 1.2 b`; the fence is gone |
| `probe_item_text_is_faced` | 3 rows | `markers still literal` — the row reads ``[ ] 1.1 RED: add the `Recorder` arm and **assert** it`` |
| `probe_hang_falls_after_the_number` | 5 rows | `continuation row hangs at "    alpha al", not 8 columns` |

**Entry-point probe — group 3.2.** Planted a temporary `mod probe` in `src/ui/markdown.rs`
handing each of `markdown-render`'s five fragments to `lines` at `78` and `58`; reverted after
running. **12 fragments rendered** (the five, plus seven further markers). Measured:

| Fragment | `lines` renders | Differs from the literal? |
|---|---|---|
| `# not a heading` | `# not a heading`, two segments, both `heading: Some(1)` | face only |
| `- not a bullet` | `• not a bullet` | text |
| `> not a quote` | `│ not a quote` | text |
| `1. not an ordered list` | `1. not an ordered list`, two segments | segmentation only |
| `--- not a rule` | `--- not a rule`, one plain segment | **no** |

So `lines` is distinguishable on **4 of 5** by `Vec<Line>` and on **2 of 5** by `Line::text()`.
This corrects the proposal's and design's claim that the block parser *eats* the marker: it
does not — `#` and `1. ` survive as text and are re-rendered, `-` and `>` are **replaced** by
`•` and `│`, and a heading face is wrongly set. The defect `inline` fixes is restyling and
mis-facing, not deletion. Also measured: `---` alone becomes a 78-column rule, `* `/`+ ` both become `• `, and both
`~~~` alone and a `` ``` `` fence opener return a **zero-element** vector — the fragment
vanishes rather than degrading to literal text, which is why design.md → Decision 4's escape
set now covers the fence and no longer covers `=`, `_`, or a bare digit run.

**Blast-radius plant — groups 1.6 and 5.3.** Planted `Item.body`, `Group.blocks`, and the
continuation rule in `src/tasks.rs`, mechanically filled the test-side literals, and ran the
whole suite; reverted, tree clean.

```
cargo test --lib
```

`1457 passed; 1 failed`. The one failure is
`tasks::tests::prose_between_items_is_dropped_and_does_not_split_a_group`, which compares a
whole `Tasks` value and now sees a block — the delta's amended scenario is its rewrite. The
compile cost was **20** literal sites: `close_group`, the `fn item` helper, and 18
`super::Group { … }` values, all inside `src/tasks.rs`.

**This plant under-counts the behaviour change by one, and the reason is in its own method.**
It planted the two fields and the continuation rule and stopped there — task 1.5's emission
rule for a blocks-only leading group was not part of it. So it could not see
`tasks::tests::an_empty_document_parses_to_no_tasks_and_no_problems`, whose prose-only half
asserts `groups: vec![]` and which 1.5 falsifies. Implementation measured **two** behaviour
changes, not one. A partial plant bounds a blast radius from below; reading its figure as
exact is the error, and it is recorded here rather than in a repair log because the method,
not the number, is what a later plant should copy differently.

**The preamble gap — group 1.5.** With the existing headingless-group suppression kept, the
same plant returned **zero groups** for `"Intro prose.\n\n"`, so 736 non-blank preamble lines
across 27 of 44 corpus files are dropped and `every_retained_line_appears_exactly_once` cannot
pass. Repaired during planning review: `task-groups` now carries the "An ATX heading at the
start of a line opens a group" requirement with the amended emission condition, and design.md
→ Decision 1a records the reasoning. Task 1.5 implements it.

**The fence gap — group 1.6a.** `parse`'s block rule as first drafted ended a block at any
blank line. Measured over the corpus, **46 of 112** column-zero fenced blocks contain one, so
that rule shreds 41% of the group-level fences this change exists to render into unpaired
delimiters, which `ui::markdown::lines` then reflows as prose with the `` ``` `` rows dropped.
Repaired in the `task-groups` delta with a fence exception covering bodies and blocks alike.

**Gates green at HEAD.** Each proves nothing on its own; the negative control for every one is
`tests/gate-controls.toml`, exercised by `cargo test --test gate_controls` (`5 passed`), which
copies the tree, plants a violation, and requires the script to exit non-zero.

| Gate | Result at HEAD | Its control row |
|---|---|---|
| `TASKSEAM` | OK, parse leg armed | `taskseam.sh` |
| `TASKWIDTHS` | `all 41 tasks tests name both 58 and 78`, floor `TASK_MIN=22` | `taskwidths.sh` |
| `MDSEAM` | `27 files searched (>= 25)` | `mdseam.sh` |
| `MDWIDTHS` | `all 40 markdown tests name both 58 and 78`, floor `MD_MIN=35` | `mdwidths.sh` |
| `DETAILWIDTHS` | `all 72 detail tests name both 58 and 78`, floor `DETAIL_MIN=38` | `detailwidths.sh` |
| `COLWIDTH` | `no char-count measurement in the nine pure view files` | four `colwidth-*` rows |
| `NOIO-VIEW` | `10 pure files carry no I/O API; positive control matched` | two `noio-view*` rows |

**Documentation checks — group 7.1.** `grep -rn 'ui::tasks::items' CLAUDE.md openspec/specs/`
matches **4 lines** at HEAD, which is what makes 7.4 falsifiable. Two of the four sit in
requirement bodies this change's deltas do not cover; see planning-review.md → Gaps.

**Numbers used in this plan.**

| Number | Command | Result |
|---|---|---|
| 1,458 lib tests | `cargo test --lib` | `1458 passed` |
| 97 `tasks` tests | `cargo test --lib tasks::tests::` | `97 passed` |
| 41 `ui::tasks` tests | `cargo test --lib ui::tasks::tests::` | `41 passed` |
| 40 `ui::markdown` tests | `cargo test --lib ui::markdown::tests::` | `40 passed` |
| 72 `ui::detail` tests | `cargo test --lib ui::detail::tests::` | `72 passed` |
| 157 `ui::view` tests | `cargo test --lib ui::view::tests::` | `157 passed` |
| Every corpus figure below | `python3 scripts/measure-tasks-corpus.py --exclude task-item-bodies` | committed, so a re-measure is one command |
| 44 corpus files (45 with this change's own) | that script, with and without `--exclude` | `44` / `45` |
| 2,842 items, all numbered | same | widths `{4: 2168, 5: 663, 6: 11}` |
| 1,924 items carrying a body (67.7%) | same, under Decision 2's rule | longest body **100** lines |
| 2,531 plain labels, 0 emphasised | same | — |
| 27 files / 736 preamble lines | same | `27`, `736` |
| 46 of 112 column-zero fences hold a blank line | same | the Decision-1 fence exception's evidence |
| 2 `tasks::Item` construction sites | `grep -rn 'Item {' src tests`, less `RowKind::Item`/`Category::Item` | `src/tasks.rs:285` and `:797` |
| 19 sites the two fields break | the plant above, `cargo build --tests` | 1 production, 1 helper, 17 test literals |
| 12 `items` call sites | `grep -rn 'super::items(\|tasks::items' src` | 2 production, 10 test |
| 10 stale `ui::tasks::items` sites | `grep -rn 'ui::tasks::items' SPEC.md CLAUDE.md openspec/specs/` | `10`, of which 8 are delta-owned |

`cargo test` takes one positional `TESTNAME`; a second filter must follow `--`, or the command
is rejected and runs nothing. Every multi-filter command above is written in the `--` form for
that reason.
