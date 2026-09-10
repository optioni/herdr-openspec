## Reviewed Artifacts

- `openspec/changes/pane-chrome/proposal.md`
- `openspec/changes/pane-chrome/design.md`
- `openspec/changes/pane-chrome/tasks.md`
- `openspec/changes/pane-chrome/specs/artifact-content/spec.md`
- `openspec/changes/pane-chrome/specs/artifact-tabs/spec.md`
- `openspec/changes/pane-chrome/specs/change-rows/spec.md`
- `openspec/changes/pane-chrome/specs/detail-header/spec.md`
- `openspec/changes/pane-chrome/specs/detail-scroll/spec.md`
- `openspec/changes/pane-chrome/specs/list-filtering/spec.md`
- `openspec/changes/pane-chrome/specs/list-selection/spec.md`
- `openspec/changes/pane-chrome/specs/mouse-input/spec.md`
- `openspec/changes/pane-chrome/specs/responsive-layout/spec.md`
- `openspec/changes/pane-chrome/specs/tasks-checklist/spec.md`
- `openspec/changes/pane-chrome/specs/view-palette/spec.md`

## Reviewed Against

- This repository HEAD: `1e658b5` at review start; repairs land at `3e87cfc`, `8cf35d0`,
  `683a776`.
- Sibling repository HEAD: `Not applicable` — `openspec-schemas` supplies the `tdd` schema and
  the agent definitions, both vendored by graft and unchanged by this review. No contract of
  this change crosses a repository boundary.
- Working tree: clean at review start, and clean at each of the three repair commits.

The finding pass was delegated to four `planning-reviewer` subagents, one slice each, none of
which wrote the plan and none of which was a fork of this session: (A) capability coverage,
scenario quality, cross-artifact contradictions; (B) design completeness, test boundaries, the
falsifiability audit; (C) task alignment, lifecycle discipline, `parallel-after`; (D) factual
verification. They reported findings only and edited nothing. This session merged them,
verified each against the tree before acting on it, and made every repair below.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| Blocking | design.md | **123 of the verification matrix's 149 commands could not run.** Each passed two or three bare filters; `cargo test` takes one `TESTNAME`, so two bare filters exit 1 and the quoted form exits **0** having run nothing (`0 passed; 1222 filtered out`). The repo wrote this rule down after catching the same defect in `mouse-input` at three rows | Every row now names its test and runs exactly that test, via a bare-name `--lib` filter, which matches on the full path so the owning module cannot be got wrong | `design.md` → Verification matrix |
| Blocking | design.md | The 149 rows carried **four** distinct Verification strings and named **no test**, so nothing bound a scenario to the thing that proves it. Every row could be marked done with zero tests written | Each row names the test that proves it, stated as a contract: a landed test is renamed to it, or the RED task writes it under that name | `design.md` → Verification matrix |
| Blocking | tasks.md | A named test is still not enough — a `--lib` filter matching nothing also exits **0** — so the matrix had no mechanism that could fail | Added a conformance check that runs every command in the matrix and fails on any running zero tests | `tasks.md` → 10.2 |
| Blocking | tasks.md | **Task 5.3's verification was vacuous.** It grepped `` `  v `` where the fixtures write `"  v `. Measured: the task's pattern returns **0 at HEAD**, the real one **28**. Green before the work, green if skipped | Corrected the pattern, recorded 28 → 0 as the measured before/after | `tasks.md` → 5.3 |
| Blocking | proposal.md | Declared **6** modified capabilities; the change ships **11** delta specs. The five undeclared are exactly those commit `0437e05` added. `design.md` already counted eleven | All eleven declared, with a one-line reason for the five that carry no new behaviour of their own | `proposal.md` → Modified Capabilities |
| Blocking | tasks.md | **No `parallel-after` marker and no sequencing statement.** The schema treats silence as nobody having looked | Recorded as genuinely sequential with the shared files named: `view.rs` is written by five of six behaviour groups, `layout.rs` by two, `list.rs` by two | `tasks.md` → Sequencing |
| Blocking | design.md | **Contracts said three signatures change and omitted `split_body`**, the one whose call sites reach furthest | Four tabulated, with `split_body`'s seven call sites named | `design.md` → Contracts |
| Blocking | design.md | The Boundaries table omitted `src/ui/app.rs`, `src/ui/driver.rs` and `src/ui/mod.rs`, all of which **fail to compile** at task 1.2. `app.rs`'s `normalise_scroll` also moves the scroll clamp | Three rows added, each marked a compile error rather than a design choice | `design.md` → Boundaries |
| Blocking | specs/responsive-layout | **The `file mode` badge scenario asserted the badge at a width its own drop rule forbids.** An 18-column heading cannot hold `demo-repo` (9), a separating blank, and the badge (9). Its own prose said "the badge needs ten more" and concluded both are drawn. Applied correctly, 20 and 19 both drop it and the pair stops discriminating | Boundary moved to 21/20, the true edge of the rule | `specs/responsive-layout/spec.md` |
| Blocking | specs/responsive-layout, specs/detail-scroll | **Two deltas gave different answers for the same call.** `interior(Rect::new(0,0,1,1), Gutters::Both)` was `Rect::new(0,0,0,0)` in one and `Rect{x:1,y:1,w:0,h:0}` in the other. The second is right: the origin clamp at `src/ui/layout.rs:116` gives it, and `layout::tests::interior_agrees_with_a_bordered_block` pins it | `responsive-layout` corrected; the clamp's arithmetic written out | `specs/responsive-layout/spec.md` |
| Blocking | tasks.md | **Four scenarios had no owning task** (the heading row's two degraded states, the list heading's own bold/dim pair, the degenerate 1–3-column frame) and **three were tasked at the wrong tier** — task 1.3 asserts `split_frame`'s tuple only, leaving the drawn-buffer half, including `src/ui/view.rs:944 one_row_frame_draws_header_only`, unowned | New group 7 with four task pairs | `tasks.md` → group 7 |
| Blocking | tasks.md | **`tests/degraded-coverage.toml` goes red at a file no task named.** Its file-mode row's proof `file_mode_badge_is_dim_after_the_label` is named for the label this change deletes, and `covers = src/ui/view.rs:293-301` spans the deleted `render_header` | Named in 3.4 as part of the same commit as the badge | `tasks.md` → 3.4 |
| Blocking | tasks.md | **No task fixed the four `## Purpose` blocks this change falsifies** — a bordered body and emphasised border, a `bold` detail header, the tab bar as the interior's *second* row, the removed `HeaderTitle`/`DetailHeader`. `tests/spec_purposes.rs` checks only that a Purpose exists and is not the archive placeholder | Named task with the audience and the four sites | `tasks.md` → 8.4 |
| Blocking | tasks.md | **`widths.sh` and `listwidths.sh` were unnamed** though this group rewrites both their subjects. `widths.sh` requires every `#[test]` in `view.rs` to name both 60 and 120, floor 114 against 122 today — eight of headroom, which 3.2 spends. Task 3.5's divider test was described 120-only and would trip it | Both gates pinned; the divider test now required to assert at 60 | `tasks.md` → 3.7, 3.10 |
| Blocking | specs/change-rows | The overflow scenario claimed the seventeen drawn rows are `change-00`…`change-16`. Row 2 is the `active` section header, so they are the header plus `change-00`…`change-15` — as `src/ui/view.rs:2704` already asserts for the bordered layout | Corrected | `specs/change-rows/spec.md` |
| Blocking | proposal.md, specs/responsive-layout | The interior moved from sixteen rows to seventeen late; two sites still said the content area "gains two rows" / grows "to eighteen" against four saying one/seventeen | Corrected to seventeen at both | `proposal.md`, `specs/responsive-layout/spec.md` |
| Blocking | proposal.md | Put the tab bar on the detail interior's **third** row; `artifact-tabs` says **first** and task 4.3's buffer row 2 agrees. The heading and padding rows sit outside the interior | Corrected, and the three-row separation stated | `proposal.md` → What Changes, Modified Capabilities |
| Non-blocking | design.md | Test Strategy claimed two doc sites "fail loudly and name both sides". **One does.** `doc_contract` binds the mouse table by its `Action::` variant set, which this change does not touch, so the table's prose drifts silently; so do the fold-glyph examples and the Purposes | Rewritten to separate the one that fails from the three that drift, with the mechanism for each | `design.md` → Test Strategy, Risks |
| Non-blocking | tasks.md | Group 7 named `doc_contract`'s **terminal-seam names** leg, which has no subject here — this change adds and removes no crossterm terminal-mode function — and omitted the mouse table and `SPEC.md` § List view's `v`/`>` examples and prose | Rewritten as group 8 with the real sites and audiences | `tasks.md` → group 8 |
| Non-blocking | specs/list-selection | The resize scenario argued "16 to 8" and `viewport(31,21,8)`. The new interior falls to **nine**. `viewport(31,21,9)` is also 17, so the rendered rows were right by luck and the arithmetic was not | Corrected, with the half-height offset shown to be unchanged | `specs/list-selection/spec.md` |
| Non-blocking | specs/detail-scroll | Claimed `interior` and `Block::inner` "disagree on `y` by exactly one **and on `height` by exactly one**". They agree on height; both subtract two rows | Corrected | `specs/detail-scroll/spec.md` |
| Non-blocking | specs/responsive-layout | Asserted the detail heading at **column 41**, which is that region's left gutter and which the same file requires blank three times over | Corrected to 42 | `specs/responsive-layout/spec.md` |
| Non-blocking | specs/artifact-content | Still described a 16-row interior whose header and tab bar took two rows | 17 rows, three taken by bar, rule and padding; the change header is outside the interior now | `specs/artifact-content/spec.md` |
| Non-blocking | specs/artifact-tabs | Prose said the padding row is given up first; its own table and degenerate-height scenario both keep it at height 3 and drop the content line instead | Prose corrected — fixed chrome, variable content — with the table named as the contract | `specs/artifact-tabs/spec.md` |
| Non-blocking | tasks.md | **Four fused "RED then GREEN" boxes**, none recording a HEAD check, against five siblings that all do | Split, each RED carrying a measured HEAD command | `tasks.md` → 3.3–3.8, 4.3–4.4 |
| Non-blocking | tasks.md | **Groups 1, 2 and 5 never ran `cargo test`** — group 1 changes four signatures crate-wide and closed on the linters alone — and no group stated a refactor outcome | Each group closes on a test run and an explicit refactor outcome | `tasks.md` → 1.9–1.10, 2.3–2.5, 5.4–5.6 |
| Non-blocking | tasks.md | Operational groups carried no CHECK/CHANGE/VERIFY, and Change Review omitted that the reviewer must be a fresh agent rather than a fork | Markers added; the fresh-agent requirement stated | `tasks.md` → groups 8, 9, 10 |
| Non-blocking | tasks.md | The final group omitted `openspec validate --strict`, which the schema requires, and verified the write boundary with `git status --short openspec/` — which the task itself admits cannot attribute, since other sessions hold untracked directories there | `validate --strict` added; the write check moved to `readonly-ui.sh`, which has a subject and a planted control | `tasks.md` → 10.8, 10.10 |
| Non-blocking | tasks.md | Task 3.6 claimed "only indices that name the interior's **last** row should move". First rows move too, at `src/ui/view.rs:2849` and `:2863` | Corrected | `tasks.md` → 3.9 |
| Non-blocking | tasks.md | The baseline called `cargo test` "not reliably green" and attributed it to concurrent sessions, superseding nothing and citing no diagnosis — though the repo already has one | Cites `markdown-legibility` design.md (commit `027db45`), the `Stages` deadline mechanism, the empty-call-log signature that separates flake from regression, and a re-measurement: 1220/2 parallel, 29 serial | `tasks.md` → baseline |
| Non-blocking | design.md | Two matrix gate rows carried a `cargo test` command that runs no gate; two more pointed at `palette.sh` where the subject was `noio-view.sh` and `colwidth.sh` | Each gate row runs its script and `gate_controls` | `design.md` → Verification matrix |
| Non-blocking | specs/responsive-layout | The badge's left-shorten branch is **unreachable whenever a badge is drawn** — a drawn badge means the name already fits — but the two rules are written in separate paragraphs and read as though they compose | Stated explicitly | `specs/responsive-layout/spec.md` |
| Nit | specs/responsive-layout | Duplicated `## MODIFIED Requirements` header with an empty section between. `openspec validate --strict` passes regardless | Removed | `specs/responsive-layout/spec.md:571` |

## No Remaining Implementation-Blocking Gaps

None remain. Every blocking gap above is repaired in the artifact that owns it, and
`openspec validate pane-chrome --strict` passes at `683a776`.

What survived review unchanged is worth recording, because it is most of the plan and it is
the part that was measured rather than remembered:

- **The core geometry is correct and internally consistent.** 38/58 and 78/58, the interior at
  buffer row 2 by seventeen rows, the content area at rows 5–18, the divider at column 40 with
  blanks at 39 and 41, the detail region at 42–119, and `split_detail`'s parts summing to the
  interior were each checked against the arithmetic and against the current code. The
  disagreements found were stale re-baselines, not design errors.
- **The scenario set is complete and 1:1 with the matrix.** 149 scenarios across eleven specs,
  149 matrix rows, sorted diff byte-identical. Empty state, degenerate widths and heights,
  CJK, emoji, ZWJ and combining-mark names are specified rather than assumed.
- **D5's measurements are honest.** `\b78\b` appears exactly **162** times under `src/ui/`, as
  claimed, and exactly three gate scripts hard-code it.
- **D6's Unicode claim is correct.** `U+25BE`/`U+25B8` are East Asian Width `N`, unlike the
  `U+25BC`/`U+25B6` pair, so `layout::columns` measures them at one column and no
  ambiguous-width terminal setting widens them.
- **The Test Boundaries table is complete**, no task invents a boundary it does not name, and
  the decision to take no outer-loop acceptance test is structurally justified rather than
  convenient: `run_loop` draws through the function under test, and a real terminal is
  forbidden because `cargo test` spawns this binary.
- **No PRD non-goal is crossed and no code path writes inside `openspec/`.** Every touched
  module is on the pure side of both architecture seams; no filesystem, process or environment
  API is added; `NOIO-VIEW`'s nine-file list is unchanged.
- **The prototype is not read as a specification.** D2, D5 and D6 each state their decision,
  their rejected alternatives, and their reason in prose, and the Design ↔ contract
  reconciliation table records the three places the prototype diverges from the contract with
  the contract winning. The change is implementable without opening it.

One theme is worth carrying into implementation: this plan's *behaviour* specification was
unusually thorough, and its claims about the repository's own **test and documentation tier**
were written from memory rather than run. Both vacuous checks, the unrunnable commands, the
wrong `doc_contract` leg, and the unnamed `degraded-coverage.toml` share that single cause. The
matrix conformance check at 10.2 exists so that the same class of defect fails loudly next time
instead of passing quietly.

## Deferred Non-Blocking Notes

- **The false `is not vendored` problem row** is not fixed here. It reverses a written decision
  in `change-merge` and is proposed separately; recorded in `proposal.md` → Non-Goals and
  `design.md` → Goals / Non-Goals.
- **Dimming artifact tabs whose file is not yet written** needs a `present` flag on
  `ArtifactRef`, which changes what the pane *knows* rather than how it is drawn. Proposed
  separately as `artifact-presence`; recorded in `design.md` → Goals / Non-Goals and in the
  Design ↔ contract reconciliation table.
- **Fold-glyph ordering against `foldable-spec-sections`.** That change's Decision 9 adopts
  whichever pair the list region carries rather than writing a literal, and costs both orders.
  If `pane-chrome` lands first — the order its Risks recommends — that change ships `▾`/`▸` and
  inherits this change's accepted Ambiguous-width risk. No action is required here; task 5.4
  keeps the pair written in one place so the dependency stays satisfiable.
- **The `ui::tests::wiring` load flake** is a standing property of the suite, not of this
  change, and is not fixed here. Its mechanism, its signature, and how to tell it from a
  regression are recorded in `tasks.md` → baseline and in `markdown-legibility`'s design.
