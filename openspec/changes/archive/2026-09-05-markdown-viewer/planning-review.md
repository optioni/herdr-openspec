## Reviewed Artifacts

- `openspec/changes/markdown-viewer/proposal.md`
- `openspec/changes/markdown-viewer/design.md`
- `openspec/changes/markdown-viewer/tasks.md`
- `openspec/changes/markdown-viewer/specs/markdown-render/spec.md` (ADDED, 24 scenarios)
- `openspec/changes/markdown-viewer/specs/detail-scroll/spec.md` (ADDED, 18 scenarios)
- `openspec/changes/markdown-viewer/specs/dashboard-loop/spec.md` (MODIFIED, 10 scenarios)
- `openspec/changes/markdown-viewer/specs/list-selection/spec.md` (MODIFIED, 5 scenarios)
- `openspec/changes/markdown-viewer/specs/responsive-layout/spec.md` (MODIFIED + ADDED, 5)
- `openspec/changes/markdown-viewer/specs/plugin-build/spec.md` (MODIFIED, 6 scenarios)

68 scenarios in all; `design.md`'s verification matrix carries 68 rows plus one cross-cutting
row, mechanically diffed against the spec files with zero missing and zero orphan rows.

**Review method.** The finding pass was delegated to **three independent subagents**, none of
which wrote the plan and none of which was a fork of the planning session. Each was given one
slice — spec correctness and arithmetic; verification that cannot fail; design completeness
and task alignment — and each wrote its findings incrementally to a scratchpad file as it
went, so a mid-run failure could not lose a report. The checks reviewer **extracted all
sixteen fenced `sh` blocks from `tasks.md` and ran them**, against the real tree read-only and
against planted violations in throwaway copies; the design reviewer built a throwaway cargo
crate to verify three Rust feasibility claims. The reviewers edited nothing; this session
merged their findings and repaired the owning artifact.

## Reviewed Against

- This repository HEAD: `4f93fa70fec2512b449e5f6299066b1b7eff8c64`
- Sibling repositories: **Not applicable.** The plugin's two external contracts — the Herdr
  CLI surface and the OpenSpec CLI's JSON — are untouched by this change: nothing in it
  spawns a process, and `NOCLI-SHELL` forbids `src/ui/` naming the CLI seam at all.
- Working tree: clean apart from the untracked `openspec/changes/markdown-viewer/`, this
  change's own planning directory, which `OPENSPEC-UNTOUCHED` excludes by name.
- Toolchain facts verified live rather than remembered: `pulldown-cmark` **0.13.4**,
  `rust-version` **1.71.1**, default features `getopts` + `html`; with defaults off it adds
  exactly `pulldown-cmark` and `unicase v2.9.0` to the resolved graph (its `bitflags` and
  `memchr` are already present at the resolved versions) and no proc-macro;
  `ratatui::CompletedFrame::area` is a public `Rect` field in 0.30.2;
  `Block::bordered().inner` returns `Rect { x: 1, y: 1, 0, 0 }` for `Rect::new(0,0,1,1)` and
  `Rect { x: 0, y: 0, 0, 0 }` for `Rect::new(0,0,0,0)`.

## Gaps Found and Fixed

**38 findings: 5 CRITICAL, 21 WARNING, 12 SUGGESTION.** Every CRITICAL and every WARNING is
repaired below. Nine SUGGESTIONs were either repaired in passing or recorded as deferred
notes; three were declined with a reason.

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/detail-scroll` | The `interior` agreement scenario listed `Rect::new(0,0,0,0)` among its inputs but pinned only width and height. `Block::inner` **clamps the advanced origin** to the rect's own edges, so at `0x0` it returns `x: 0` while the literal reading gives `x: 1`; `Rect` compares all four fields, so the scenario as written could not pass. | The requirement now states the clamp explicitly and the scenario asserts whole `Rect` values with both degenerate results named. Measured against ratatui 0.30.2, not assumed. | `specs/detail-scroll/spec.md` → "The drawn slice is derived on every draw"; `tasks.md` 6.1 |
| CRITICAL | `specs/detail-scroll` | "every cell of the `# Title` row reports `Modifier::BOLD`" contradicted `markdown-render`'s "a line SHALL NOT be padded to the width" — the columns past the heading are never written. A test written to the spec would fail. | Narrowed to the seven cells spelling `# Title`, with the reason stated in the scenario. | `specs/detail-scroll/spec.md` → "Faces reach the buffer as styles at both widths" |
| CRITICAL | `tasks.md` | The `Dashboard`-literal inventory was wrong by half: it claimed seven sites across five files. Measured: **28 grep hits, 14 real sites, six files**, including **four in `src/ui/list.rs`** — a file `design.md` → Boundaries did not name at all, while `tasks.md` declares that table the contract. The task's own "Red when" would have fired and halted the group. | The sweep is now a CHECK task with the measured per-file distribution, and `src/ui/list.rs` and the corrected `src/lib.rs` role are rows in the Boundaries table. | `tasks.md` 0.1 and 1.1; `design.md` → Boundaries; `proposal.md` → Impact |
| CRITICAL | `design.md`, `tasks.md` | The acceptance test was to reuse the scripted `EventSource` double from `src/ui/driver.rs`'s tests. That `mod tests` has **no visibility modifier**, so the module is private to `ui::driver` and unreachable from the sibling `crate::ui::tests::detail`; marking the item `pub(crate)` does not make the path legal. One of the task's two stated options did not compile, inside the one group that must stay assertion-shaped for seven groups. | The `Script` double and `press` helper are **lifted to `crate::testutil`** as an explicit task; the dead option is deleted; the Test Boundaries row, the Boundaries table, and `proposal.md` → Impact all say so. | `tasks.md` 1.1 and 1.3; `design.md` → Boundaries, Test Boundaries, Decisions; `proposal.md` → Impact |
| CRITICAL | `tasks.md` | Group 0 was `kind: behavior` but put a GREEN task (`Detail` + the `Dashboard` field) **before** its RED, and mixed a plumbing change with a behaviour test — two `tdd` schema MUSTs broken at once. | Split into `## 1` (**operational**: CHECK the sites → CHANGE → VERIFY) and `## 2` (**behavior**: RED → confirm → VERIFY), with the baseline group moved to `## 0`. All thirteen following groups renumbered; `design.md`'s eight stale task and group references corrected in the same pass. | `tasks.md` groups 0–13; `design.md` → Test Strategy, Decisions, matrix |
| WARNING | `specs/markdown-render` | The `width - 2` content rule for bullets and quotes was never stated and underflows at widths 1–3, which the totality scenario exercises. | Stated as `width.saturating_sub(prefix)`, with the zero-columns case defined (prefix truncated, no content). | `specs/markdown-render/spec.md` → requirement 1; `tasks.md` 4.3 |
| WARNING | `specs/markdown-render` | Hard-splitting an over-long token was asserted in three scenarios and never required. | Added as a SHALL in requirement 1, covering prose and code alike. | `specs/markdown-render/spec.md` → requirement 1 |
| WARNING | `specs/detail-scroll`, `specs/dashboard-loop` | `FilterStart` moves the route to `List` but was not in the scroll-reset list, so the blanket sentence and the enumerated `apply` contract disagreed. | Reset is now stated as "every action that moves the route", enumerating all three arms, with the filter-layer `Back` explicitly excluded. Scenario renamed and extended to cover all four cases. | `specs/detail-scroll/spec.md`; `specs/dashboard-loop/spec.md`; `tasks.md` 6.3, 6.5 |
| WARNING | `specs/detail-scroll` | The drawing requirement never mentioned `scroll_offset`, contradicting its own scroll-past-the-end scenario. | The requirement now names the slice and its offset. | `specs/detail-scroll/spec.md` → requirement 1 |
| WARNING | `specs/markdown-render` | Images (`![alt](x)`) are core CommonMark, were unspecified and undeferred, and would have vanished silently — the exact failure the change exists to prevent. | New rule (alt text with `link`; `[img]` when the alt is empty) and a new scenario; a matrix row and a test in group 4. | `specs/markdown-render/spec.md`; `design.md` matrix; `tasks.md` 4.1 |
| WARNING | `specs/markdown-render`, `specs/responsive-layout` | The 78/58 freeze was stated normatively in two capabilities. | `responsive-layout` owns it; `markdown-render` now consumes it with a pointer. | `specs/markdown-render/spec.md` → requirement 1 |
| WARNING | `specs/detail-scroll` | Three scenarios asserted `- line-00` without naming the `detail.source` that produces it. | Each now names the twenty-item fixture. | `specs/detail-scroll/spec.md` |
| WARNING | `specs/detail-scroll` | The resize scenario could not be driven: `run_loop` borrows the `Terminal` mutably for its whole run, so no scripted source can resize the backend from inside it. | Re-specified as the `draw` + `normalise_scroll` pair driven directly — one loop iteration, decomposed — with the reason stated in the scenario. | `specs/detail-scroll/spec.md`; `design.md` matrix; `tasks.md` 8.1 |
| WARNING | `specs/detail-scroll` | The first detail scenario asserted change-row grammar in the list region while naming no changes. | Names one active change and the exact 38-column row string, verified by construction. | `specs/detail-scroll/spec.md`; `tasks.md` 7.3 |
| WARNING | `specs/detail-scroll` | `ui::load`'s `NotFound` arm — the no-repository state, and a second `Dashboard` construction site — was covered by no scenario. | The startup scenario now covers both arms; task 8.1's test does too. | `specs/detail-scroll/spec.md`; `tasks.md` 8.1 |
| WARNING | `tasks.md` | `GRAPH-SNAP`'s four named absences had **no negative control**: task 10.2's plant appended a line to the snapshot, which only trips `diff -u`, because the absence loop reads the regenerated graph and never `$SNAP`. | Replaced with a two-step control: turn on the `html` feature in a copy, regenerate the snapshot there so the diff passes, then run the check and require the absence to fire. The limitation is also documented inside the block. | `tasks.md` 10.2 item 17; the `GRAPH-SNAP` block |
| WARNING | `tasks.md` | `DEPS` item 16's single plant could only reach leg 2a — the script exits there — so leg 2b's own violation (an *omitted* `features` key, which `cargo metadata` cannot see) had no plant at all. | Split into two plants, (a) for leg 2a and (b) for leg 2b, with the distinction stated. | `tasks.md` 10.2 item 16 |
| WARNING | `tasks.md` | Task 0.2 listed `GRAPH-SNAP` among the checks "expected to fail" and then said to record it as passing. | Moved to the expected-to-pass list, with the reason (it is what makes the later regeneration reviewable). | `tasks.md` 0.2 |
| WARNING | `tasks.md`, `specs/plugin-build` | The at-the-floor MSRV package list was stated as four; the check actually reports **nine**, including `herdr-openspec` itself. The live spec's wording read as exhaustive and never was. | The delta spec now says the set is **reported, not asserted**, and names the nine; the task records it verbatim rather than comparing to a fixed list; the live-spec correction is logged as an archive-time consequence. | `specs/plugin-build/spec.md`; `tasks.md` 3.5, 11.8 |
| WARNING | `design.md` | Eight references to task and group numbers disagreed with `tasks.md`, one of them pointing at a real-but-different task. | All corrected to the renumbered scheme in one pass. | `design.md` → Test Strategy, Decisions, matrix |
| WARNING | `tasks.md` | Group 3 (operational) put CHANGE before its evidence CHECK. | A CHECK task now records `DEPS` leg 2a's failure and `GRAPH-SNAP`'s pass before the manifest is touched. | `tasks.md` 3.1 |
| WARNING | `tasks.md`, `design.md` | `layout::interior` was to be implemented in the app group and tested a group later — untested saturating arithmetic, and a RED that would be green on arrival in the next group. | `interior`'s RED and GREEN both moved into group 6, ahead of its first caller; group 7's RED narrows to `scroll_offset`, and the one test there that *is* green on arrival is called out as a deliberate freeze. | `tasks.md` 6.1, 6.2, 7.1; `design.md` → Decisions, matrix |
| WARNING | `specs/detail-scroll`, `tasks.md` | `quoted → Modifier::DIM` was one of five mapping clauses and the only one no test touched — deletable with every test still green. | The faces fixture gains a `> quoted` line and a fifth positive assertion at both widths; the matrix row says five. | `specs/detail-scroll/spec.md`; `tasks.md` 7.3; `design.md` matrix |
| WARNING | `design.md` | The "exhaustive on purpose" shell-utility enumeration omitted `mv`, `cat`, and `dirname`, all used in the check blocks, and `make`. | All four added. | `design.md` → Test Boundaries |
| WARNING | `tasks.md`, `design.md` | The `Action` rename's call sites were given as `src/ui/driver.rs`, which names **no** variant; the real sites are `src/ui/app.rs` (six) and `src/ui/view.rs`'s tests (five). The `Dashboard`-field sites and the rename sites had been merged into one list. | Both sets are now stated separately and measured. | `tasks.md` 6.4; `design.md` → Contracts |
| WARNING | `tasks.md` | Task 0.3's `MDSEAM` ordering control used `MIN=17`, which the planted copy passes anyway, so the "control fires before the count" claim was never exercised. | Changed to `MIN=99`, where only the ordering can produce the expected message. | `tasks.md` 0.3 |
| WARNING | `tasks.md` | Task 0.1 attributed `MDSEAM`'s baseline to the `src/changes.rs` exclusion. `MDSEAM` excludes the file this change *adds*, so its count is 17 before and 17 after — not 16 → 17. | Split into its own measured bullet, with the difference called out in the check block and in the reviewer brief. | `tasks.md` 0.1, the `MDSEAM` block, 12.1 |
| WARNING | `design.md` | The Test Boundaries "process environment" row said "not read" in the command-level column, while nine checks are parameterised by environment variables. | Corrected to "real", enumerating them. | `design.md` → Test Boundaries |
| SUGGESTION (fixed) | `specs/responsive-layout`, `design.md` | A scenario about `src/ui/markdown.rs` was titled "Every row-grammar test…", `ui::list`'s term. | Renamed to "Every markdown test…"; matrix row updated. | `specs/responsive-layout/spec.md`; `design.md` matrix |
| SUGGESTION (fixed) | `tasks.md` | Task 9.1 led with a bare filtered `cargo test`, whose failure mode is "exits 0 with nothing matched" — the only such case in the file. | Replaced with the counted `testcount` form. | `tasks.md` 9.1 |
| SUGGESTION (fixed) | `design.md` | `line_text_concatenates_its_segments` — the test that pins `Line::text()`, which every other assertion runs through — appeared in no matrix row. | Added to the "No line exceeds the width" row. | `design.md` matrix |
| SUGGESTION (fixed) | `tasks.md` | "`cargo build --locked` must fail" is order-fragile: any intervening `cargo tree` or `cargo metadata` rewrites the lock first. | The task now requires it as the **very next** cargo command and says why. | `tasks.md` 3.2 |
| SUGGESTION (fixed) | `design.md` | "both call sites" of `Block::bordered().inner`; there is exactly one. | Corrected to one, with the file and function named. | `design.md` → Boundaries, Decisions |
| SUGGESTION (fixed) | `tasks.md` | Task 3.4's diff review was prose. | Made mechanical: `git diff --numstat` must read exactly `8 0`. | `tasks.md` 3.4 |
| SUGGESTION (declined) | `tasks.md` | `DEPS` leg 5 does five cold builds; a shared `CARGO_TARGET_DIR` would speed it up. | Declined for this change: `DEPS` already carries four deliberate edits, and a fifth for speed alone widens the diff a reviewer must read against a check whose whole value is that it is byte-comparable to the archived one. Recorded as a deferred note. | — |
| SUGGESTION (declined) | `tasks.md` | Four checks share the variable name `MIN`. | Declined: each is a separate `sh` invocation with its own environment, they are never sourced together, and renaming them would break byte-identity with the archived blocks. The three distinct baselines are called out in 0.1 and in the reviewer brief instead. | `tasks.md` 0.1, 12.1 |
| SUGGESTION (declined) | `specs/*` | `Route::Detail` with `filter.active` true is unreachable through `apply`. | Declined: it is reachable by direct construction, which is how the landed `dashboard-loop` scenario already builds it, and the layered-`Esc` contract is stated over states rather than over reachable paths. | — |
| SUGGESTION (noted) | `SPEC.md` | The degraded-states table does not yet carry the unmodelled-markdown row `markdown-render` cites. | Correct, and it is task 11.6's job — the specs are written against the tree this change produces. | `tasks.md` 11.6 |

### Verified correct, and therefore not repaired

Recorded because the absence of a finding is itself evidence:

- **All five MODIFIED requirements** match their live headers verbatim and carry **all 24**
  live scenario names verbatim. Nothing is silently deleted at archive time.
- **Every concrete number** in the specs was re-derived independently: the greedy wraps at 58
  and 78 including the exactly-78 inclusive boundary; the 57-character bullet and quote first
  lines; the 130-`x` splits (58+58+14 and 78+52); `Rect::new(41,2,78,16)` and
  `Rect::new(1,2,58,16)`; max scroll 4 in a 16-row interior; rows 2 and 17 at scroll 0, 2, 4
  and 99; the `scroll_offset` boundary table; `frames: 11, polls: 11` traced through
  `run_loop`; and the 120x30 resize giving a 26-row interior and scroll 0.
- **No check in the plan is structurally incapable of going red.** All sixteen blocks were
  extracted and run; every existence guard, positive control and planted violation fires.
  `TESTCOUNT`'s `[ $? -eq 0 ]` after a `case` does propagate cargo's status — re-verified
  under `/bin/sh`, `bash`, `zsh` and `dash`.
- **Every counted floor is `measured baseline + this change's addition`.** All baselines
  reproduce on the tree (16 / 16 / 17 / 8 / 18 / 47 / 17 / 516), and the per-module deltas add
  to 48, giving 564.
- **The verification matrix is complete**: 68 scenarios, 68 rows plus one cross-cutting row,
  zero missing, zero orphan.
- **No PRD non-goal is crossed**, no code path writes inside `openspec/`, no process spawn is
  added outside `src/cli.rs`, and the 80% coverage floor is neither lowered nor excluded.
- **No `detail-view` or `tasks-tab` scope leaked in**: no tab bar, no change header, no
  `1`–`9` / `[` / `]`, no "No content yet".

### `SPEC.md` corrections this change makes, before and after

Planned in `design.md` → Decisions and executed in `tasks.md` group 11. The "before" text is
quoted from `SPEC.md` at `4f93fa7`.

1. **User interface → Detail view, the viewer sentence.**
   *Before:* "Every other tab is a markdown viewer built on `pulldown-cmark`, supporting
   headings, lists, code blocks, emphasis, and links."
   *After:* the rendering grammar — what each construct becomes, that soft breaks are
   preserved rather than re-flowed, that code and raw HTML are hard-split rather than clipped,
   that a link's destination is not printed and an image renders its alt text, and that
   tables, footnotes, strikethrough, and task lists render as literal text.
   *Why:* the sentence is the whole of the viewer's contract today and under-determines every
   decision this change had to make.
2. **User interface → Detail view, ownership.**
   *Before:* "A header carrying change name, schema, and progress; a tab bar built from the
   schema's artifact list; content below." — one undivided description.
   *After:* each sentence annotated with the change that owns it (`detail-view`, `tasks-tab`,
   `markdown-viewer`).
   *Why:* without it, an auditor reads `detail-view`'s absence here as this change's omission.
3. **User interface → Detail view, the interiors.** *Before:* nothing. *After:* 78 columns by
   16 rows at 120x20, 58 by 16 at 60x20 in the detail route. *Why:* `list-view` froze 38 and
   58 for the list; `detail-view` and `tasks-tab` need the same fact for the other region.
4. **User interface → Responsive layout.** *Before:* the `Length(40)` / `Min(0)` constraints
   with no interiors. *After:* both regions' interiors, with the note that 78 is a property of
   the mandated frame width rather than a layout constant, since the detail column is `Min(0)`.
5. **User interface → Keys, the `j` / `k` / arrows row.**
   *Before:* "Move the list selection, clamped at both ends rather than wrapping; the list
   scrolls to keep it visible. While filtering, the arrows still navigate, but `j` and `k`
   type themselves into the query instead."
   *After:* the same, plus the route split — the detail content scrolls by one line at
   `Route::Detail`.
   *Why:* this change makes the unqualified row false, and it is the row a reader consults.
6. **User interface → Keys, the `1`–`9` / `[` / `]` row.** *Before:* "Switch artifact tab."
   *After:* the same, annotated as `detail-view`'s. *Why:* a reader would otherwise expect tab
   switching to work in the change that fills the detail region.
7. **Overview → Stack.** *Before:* "`pulldown-cmark` (markdown)" — no version, no feature
   decision, while `yaml-rust2` carries a pointer to where its choice is argued. *After:* the
   same pointer, to this change's `design.md`. *Why:* so a later change does not re-open it.
8. **Architecture → Module map, the `ui` row.** *Before:* "Views (the change-row grammar
   included), layout, the dashboard's own state (selection and the `/` filter), key handling,
   terminal lifecycle, and the event loop." *After:* plus the markdown rendering, now the
   largest single thing in the module.
9. **Testing and quality gates → Unit-tested modules.** *Before:* the `ui` bullet lists
   `ui::layout`, `ui::app`, `ui::list`, `ui::view`, `ui::driver`, `ui::terminal`, and
   `ui::load`. *After:* `ui::markdown` added, with its own width check.
10. **Testing and quality gates → View tests.** *Before:* "at both 60 and 120 columns so the
    responsive breakpoint is genuinely covered." *After:* all three pairs named — 60/120 for
    frames, 38/58 for list interiors, 58/78 for detail interiors. *Why:* two capabilities now
    assert interior widths and the sentence is silent about which pair applies where.
11. **Degraded states, new row.** *Before:* no row for markdown the viewer does not model.
    *After:* one, stating that such source renders as its literal text rather than being
    dropped. *Why:* the state is reachable today, and `degraded-states`' "confirm, do not add"
    principle would otherwise be false in Phase 6.
12. **Degraded states, the missing-artifact row.** *Before:* "Artifact file missing | Tab is
    still shown and renders 'No content yet'." *After:* the same, annotated as `detail-view`'s.
    *Why:* this change deliberately does not implement it.

`AGENTS.md` gains, in **Current repo state**, what the detail region now does and what still
fills it; and in **Architecture rules**, two durable constraints — `pulldown_cmark` is named
only in `src/ui/markdown.rs` and that module names no ratatui type, and the detail region's
two mandated interiors are 78 and 58 with every `ui::markdown` test asserting both.

## No Remaining Implementation-Blocking Gaps

None remain. All five CRITICALs and all twenty-one WARNINGs are repaired in the artifact that
owns each; the twelve SUGGESTIONs are fixed, declined with a stated reason, or recorded below.
No unresolved decision requires user input.

The one judgement worth flagging rather than hiding: this change **answers an open question
`list-view` deferred to `detail-view`** — whether `j` / `k` rebind to the detail pane. The
roadmap's prediction was off by one row, because `markdown-viewer`, not `detail-view`, is the
change that introduces scrollable detail content. The answer is recorded in
`specs/dashboard-loop`, `specs/list-selection`, `specs/detail-scroll`, and `design.md` →
Decisions, marked **BREAKING** in `proposal.md` as a keybinding change, and flagged for
`IMPLEMENTATION-ORDER.md` at archive time so `detail-view` does not re-open it.

## Deferred Non-Blocking Notes

Each has its resolution point already recorded in `design.md` → Open Questions or in
`tasks.md`:

- **Table layout.** The `ENABLE_TABLES` extension stays off and tables render as literal
  source rows. Worth revisiting only together with a column-layout pass; `degraded-states`
  audits the row that records it.
- **Link destinations.** Not shown. `detail-view` is the first change that will know whether a
  destination names a sibling artifact the pane could open.
- **Heading colour.** Levels are carried by their `#` markers because the crate has no colour
  policy; whichever change adopts a palette owns this.
- **Memoising the rendered line vector.** Recomputed per draw and per normalisation today;
  `live-refresh` is the change that gains an invalidation signal and can memoise correctly.
- **Display-column widths.** Measured in `char`s, sharing `ui::list`'s documented limitation;
  resolving one without the other would leave the crate with two width rules.
- **`DEPS` leg 5's five cold builds.** A shared `CARGO_TARGET_DIR` would speed them up;
  declined here to keep the block byte-comparable with the archived one, and worth doing in a
  change that already has reason to touch it.
- **Archive-time edits**, listed in `tasks.md` 11.8: `IMPLEMENTATION-ORDER.md`'s
  `markdown-viewer` row (the `plugin-build` delta, the keybinding change, the extra
  constructs, and a `Degraded states` spec ref), the sibling note about the answered open
  question, and the `plugin-build` MSRV wording that reaches the live spec only on archive.

## Change Review (implementation time)

Dispatched to a fresh `outside-in-tdd-reviewer` subagent (agent id `af40e507ec06a4b41`) at
task 12.1, against `proposal.md`, `design.md`, `tasks.md`, all six spec files, and
`git diff c07299a..HEAD` — none of the implementing session's own reasoning. The reviewer
independently re-ran 9 of the 17 planted-violation controls, all thirteen grep/python checks,
`DEPS` (legs 1a–4), `GRAPH-SNAP`, `OPENSPEC-UNTOUCHED`, and a corpus sweep of every `.md` file
in the repository at seven widths, and wrote findings incrementally to a scratchpad file.

**2 CRITICAL, 3 WARNING, 5 SUGGESTION.** All repaired.

| Severity | Problem | Repair | Location |
|---|---|---|---|
| CRITICAL | A **loose** markdown list (any blank line anywhere in a bullet list makes CommonMark wrap *every* item in `Start(Paragraph)`, even single-line ones) lost its `- ` marker and gained a spurious blank line between items. Root cause: `finish()` early-returns without `reset_ambient()` when the currently-configured block (the `Item`) never received content, and `start_paragraph` then unconditionally overwrote `first_prefix`/`category`, discarding the item's own marker. No test covered a loose list. | `start_paragraph` now checks `self.category != Category::Item` before overwriting the prefix/category, so a paragraph opening while an item's context is still live (nothing finalised since) keeps that item's marker. New test `a_loose_list_keeps_its_marker_and_no_blank_between_items` at both widths. | `src/ui/markdown.rs` — `Folder::start_paragraph`, new test |
| CRITICAL | This change introduced **15 `..base` struct-update elisions** on `Dashboard`/`Detail` across new test helpers in `app.rs` (9), `driver.rs` (3), and `view.rs` (3) — zero existed at BASE. `dashboard-loop` requires "every construction … SHALL name every field, with no `..` rest, so a field added later fails to compile at each site rather than defaulting silently." `NODEFAULT-UI`'s half B is line-anchored (`grep`, one line at a time) and cannot see a multi-line `Dashboard { …, ..some_fn() }`, so all fifteen were invisible to the check that exists to catch exactly this. | All fifteen sites rewritten to name every field explicitly, no `..` anywhere. Re-verified: `grep -rn '^\s*\.\.' src/ui/*.rs src/lib.rs` shows only `Face`/`Config`/`Recorder` `Default` uses, none of the three gated types. | `src/ui/app.rs`, `src/ui/driver.rs`, `src/ui/view.rs` |
| WARNING | `specs/markdown-render/spec.md`'s table scenario named `\| Gate \| Command \|`, but the shipped test uses `\| Gate \| Runner \|` (renamed at task 10.1 to stop tripping `NOIO-VIEW`'s `Command` pattern) — the spec was never updated to match. | Spec scenario text corrected to `Runner`. | `specs/markdown-render/spec.md` → "A table renders as its literal source text" |
| WARNING | The "Inline constructs become separate segments" scenario named an exact fixture sentence that, at width 58, splits `the`/`design` across a wrap boundary — the shipped test uses a shorter sentence that fits on one line at both widths, but the spec was never corrected to match. | Spec scenario rewritten to the shipped fixture, with a note explaining why the sentence must fit on one line at both widths. | `specs/markdown-render/spec.md` → "Inline constructs become separate segments carrying their faces" |
| WARNING | `tasks.md` task 2.1 still describes the acceptance test's original two-`j`-press script; task 7.6 extended it to ten, recorded only in a commit message and a source comment. | A note appended to task 2.1 pointing to 7.6 as the test's final shape. | `tasks.md` 2.1 |
| SUGGESTION (fixed) | Three stale doc comments: `render_body` said the detail region was "left untouched"; `Dashboard`'s doc said "all seven fields"; `composite_fixture`'s comment referenced "group 4's simplified fold" after group 5 replaced it. | All three corrected. | `src/ui/view.rs`, `src/ui/app.rs`, `src/ui/markdown.rs` |
| SUGGESTION (fixed) | `Dashboard::normalise_scroll` re-implemented `layout::scroll_offset`'s clamp inline instead of calling it, once the function existed (it landed a group later, in group 7). | Now calls `layout::scroll_offset` directly. | `src/ui/app.rs` |
| SUGGESTION (fixed) | `a_faced_run_split_across_a_wrap_keeps_its_face` asserted the exact wrapped strings at width 58 only, not 78, though the spec scenario requires both. | Added the 78-width exact-string assertion. | `src/ui/markdown.rs` |
| SUGGESTION (noted, not changed) | `Folder::start_item`'s `.expect("Item event outside an open List")` panics on a state pulldown-cmark's own grammar guarantees cannot occur. | Left as is: a documented, unreachable-per-the-parser's-own-invariant panic with a clear message is the right shape here, not a bug. | `src/ui/markdown.rs` |
| SUGGESTION (latent, not reproduced) | `hard_split_group` reads only `group.first()`; if a verbatim line's content ever arrived as two `Run`s in one group, the second would be silently dropped. The reviewer could not reproduce this against real pulldown-cmark output — `push_verbatim` always closes a group after each line — so it is latent rather than confirmed. | Left as is; recorded here as a known latent assumption (one `Run` per hard-split group) should the verbatim-accumulation logic change later. | `src/ui/markdown.rs` — `hard_split_group` |

All fixes verified: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D
warnings`, `cargo test --all-features` (565 lib tests, up one for the new loose-list test), and
the full sixteen-check architectural suite (`NOSPAWN-GREP`, `NOIO-VIEW`, `NOCLI-SHELL`,
`NORAW-GREP`, `NODEFAULT-UI`, `NOLIT-CHANGE`, `MDSEAM`, `WIDTHS`, `LISTWIDTHS`, `MDWIDTHS`,
`NOWAIVER`, `OPENSPEC-UNTOUCHED`) all pass, all green after every fix.
