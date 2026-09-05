# detail-view — planning review

## Reviewed Artifacts

| Artifact | Size | Status |
|---|---|---|
| `proposal.md` | Why / What Changes / Non-Goals / Capabilities / Impact | reviewed, corrected |
| `specs/detail-header/spec.md` | 2 requirements, 9 scenarios | reviewed, corrected |
| `specs/artifact-tabs/spec.md` | 4 requirements, 19 scenarios | reviewed, corrected |
| `specs/artifact-content/spec.md` | 3 requirements, 15 scenarios | reviewed, corrected |
| `specs/dashboard-loop/spec.md` | 4 MODIFIED requirements, 20 scenarios | reviewed |
| `specs/detail-scroll/spec.md` | 4 MODIFIED requirements, 13 scenarios | reviewed, corrected |
| `specs/responsive-layout/spec.md` | 1 MODIFIED requirement, 3 scenarios | reviewed |
| `design.md` | Context, Goals, Boundaries, Contracts, Test Boundaries, Test Strategy, 79-row verification matrix, 14 Decisions, Risks, Visual Design | reviewed, corrected |
| `tasks.md` | 16 groups, 84 tasks, 11 reproduced check blocks, 9 extracted | reviewed, corrected |

Totals after correction: **18 requirements, 79 scenarios, 16 task groups, 84 tasks**, and a
verification matrix with **exactly 79 rows**, one per scenario, names matching mechanically.

`openspec validate detail-view --strict` → **Change 'detail-view' is valid**.

## Reviewed Against

- `SPEC.md` — Overview, Architecture (both seams, Module map), Data layer, User interface
  (Responsive layout, List view, Detail view, Keys), Degraded states, Testing (Unit-tested
  modules, View tests, Fixtures, Gates).
- `PRD.md` — Goals 1 and 5, the Reading stories, the four Non-goals, Success criteria.
- `openspec/config.yaml` — every rule under `proposal`, `specs`, `design`, `tasks`, and
  `planning-review`.
- `openspec/IMPLEMENTATION-ORDER.md` — the Phase 4 `detail-view` row as the scope boundary,
  and the "Notes on the ordering" paragraph recording that `markdown-viewer`, not this
  change, answered the `j` / `k` collision.
- The three landed Phase 4 changes' archives — `2026-09-04-tui-shell`,
  `2026-09-05-list-view`, `2026-09-05-markdown-viewer` — for the check blocks carried
  forward and for the task conventions.
- The live tree at `d3b798c`: `src/ui/{app,detail-absent,driver,event,layout,list,markdown,
  mod,terminal,view}.rs`, `src/changes.rs`, `src/lib.rs`, `Cargo.toml`, `rustfmt.toml`,
  `Makefile`.

## Method

Three reviewer subagents, none of which wrote the plan, dispatched in parallel with disjoint
assignments and each writing findings **incrementally** to a scratchpad file rather than
batching them into a final message. Two earlier reviewer rounds in this project died on
`529 Overloaded` and lost their reports; the incremental-file practice has meant zero losses
since.

- **Reviewer A — spec and design coherence.** Contracts, scope boundaries, PRD non-goals,
  spec quality, cross-artifact consistency, the claimed `SPEC.md` corrections, and every
  numeric claim in the specs recomputed by hand.
- **Reviewer B — the gates.** Extracted every fenced block to a scratch `$CHECKS`, **ran**
  each against the live tree, confirmed each archive-sourced block actually exists in the
  file it is extracted from, planted violations in scratch copies to confirm each control
  fires, and independently re-measured every floor.
- **Reviewer C — codebase compatibility.** Compiled the planned signatures against the real
  types in a scratch copy, including the `sync_detail` borrow, `run_loop`'s new parameter,
  and clippy `-D warnings`.

Between them: **6 CRITICAL**, **17 MAJOR**, and **24 MINOR** findings. Every CRITICAL and
MAJOR is fixed below. All three reviewers confirmed `git status --porcelain` clean but for
this change's own directory when they finished.

## Gaps Found and Fixed

### CRITICAL

**C1 — `NOTABSEAM`'s second leg was red on the unmodified tree.** *(Reviewer B, Reviewer C)*
The check as first drafted grepped `ratatui|Modifier|Style|Span|Rect|Frame|Buffer` over the
whole of `src/ui/list.rs` and asserted that file "already met" the standard. It does not:
`src/ui/list.rs:2` says "with no ratatui styling" and `:22` says "`ui::view` applies
`Modifier::BOLD` to the selected one" — both correct prose, both matched. The check would
have failed at its first run (task 4.3) with no code at fault, and the only ways to make it
green would have been to reword true comments or to exempt comments, which is how a
confinement check rots into a rubber stamp. **Fixed:** the second leg is removed; `NOTABSEAM`
searches `src/ui/detail.rs` and nothing else, on exactly `MDSEAM`'s single-file terms, and
the block records why the `list.rs` leg was written and rejected. Task 4.3 now carries the
comment-wording obligation for `src/ui/detail.rs` explicitly, with its own Red-when.

**C2 — the multi-line elision control targeted the wrong shape, and named a mitigation that
does not exist.** *(Reviewer B)* The draft's control 12 planted a three-line **pattern**
elision and claimed `NODEFAULT-UI`'s same-line grep would miss it while "the compile-time
companion" caught it. Both halves were false. `cargo fmt --check` collapses a short pattern
back to one line, which the grep *does* catch — so it is a formatting violation, not a blind
spot. And the compile-time companion (`src/ui/app.rs`) destructures **one** value: it detects
a field added to the type and stayed green with the elision planted elsewhere. The real blind
spot is the multi-line **struct-update literal** (`Detail { source: …, ..base.clone() }`),
which is rustfmt's own output and which nothing in the tree catches. **Fixed** three ways:
(a) `NODEFAULT-UI` is **edited on disk** — its half B is now a brace-matching python pass
that sees past a line break, verified at planning time to report 68 spans and 0 elisions on
`main` and to catch both a planted single-line pattern elision and a planted seven-line
struct-update literal, while not false-positiving on `for i in 0..10`; (b) control 13
replaces control 12 and requires all three outcomes to be recorded, including that the
companion stays green; (c) the false claim about the companion's reach is retired in the
check's own comment, in `design.md` → Boundaries, and in task 1.3's Red-when.

**C3 — `testcount --test-cli 5` was structurally unfailable.** *(Reviewer B)* The helper's
signature is `testcount <scope> <filter> <minimum>`; `5` bound to *filter* and the minimum was
empty. Reviewer B's real run: `TESTCOUNT OK: --test-cli filter '5' ran 5 tests (>= )`. Under
zsh `[ 0 -ge "" ]` is true, so it passes with zero tests; under bash it errors and is red on
every tree. **Fixed:** task 15.3 now reads `testcount --test-cli '' 5`, and the task states
why the empty filter is load-bearing.

**C4 — `ui::tests::detail:: 1` was already satisfied on `main`.** *(Reviewer B)* That module
exists and holds `markdown-viewer`'s `a_markdown_document_renders_and_scrolls_through_the_loop`,
so the floor gating the one test this change exists to turn green was met before a line was
written. **Fixed:** the floor is **2**, task 2.1 says it adds a *second* test function rather
than "extending" the module, and the test-module table records the measured 1.

**C5 — `WIDTHS_MIN=72` was unreachable.** *(Reviewer B)* Nine of the nineteen scenarios group
9 listed already have tests in `src/ui/view.rs`; only ten are new, so 57 + 10 = 67 < 72. The
gate would have been red against a correct implementation. **Fixed:** the floor is **67**,
group 9 is split into 9.1 (ten new tests, enumerated) and 9.2 (six existing tests updated,
each named), and the test-module table now carries a Measured / New / Target triple for every
module so the arithmetic is checkable rather than trusted.

**C6 — the five-step `sync_detail` in `design.md` → Contracts does not compile.**
*(Reviewer C)* Written literally against the real types it fails twice: `E0506` assigning
`self.detail.tab` while `selected_change()`'s borrow is live, and `E0502` on
`self.detail.problems.clear()` even with the clamp removed. `selected_change()` borrows all
of `*self`, so cloning `dir` mid-way is not enough. **Fixed:** `design.md` now carries the
shape Reviewer C compiled — everything the read needs copied out in **one** expression, the
clamp computed inside the borrow and assigned after it ends — as a Rust block, plus the note
that the `RecordingReader` cannot itself *be* an `ArtifactReader` and needs a `RefCell` and a
closure wrapper at each call site.

### MAJOR

**M1 — the header-degradation scenario was vacuous.** *(Reviewer A, Reviewer C, independently)*
The width list `78, 58, 30, 26, 25, 20, 19, 18, 5, 2, 1, 0` never lands in the 7–12 band, so
the clause "`(tdd)` is absent while `[4/9]` is still present" was vacuously true and an
implementation that never drops the schema cell would have passed. **Fixed:** the list is
`78, 58, 13, 12, 7, 6, 5, 1, 0` — both boundaries of all three bands — the spec now states
the three bands explicitly, and task 4.1 says the list is not negotiable and why.

**M2 — the "no partial cell" assertion was unsatisfiable as written.** *(Reviewer A)*
`(tdd)` contains `(tdd` and `[4/9]` contains `[4/`, so the assertion failed even at width 78.
**Fixed:** rephrased as an implication — wherever a result contains `(tdd` it also contains
the whole `(tdd)`.

**M3 — `tab_bar`'s windowing was undefined when `selected` is out of range**, and at widths
0, 1, and 2. *(Reviewer A, Reviewer C)* The rule was stated over `cells start..=selected`,
which does not exist for an out-of-range `selected`, while totality explicitly admitted the
case and the scenario asserted only "no panic". **Fixed:** the spec now states that
`width == 0` returns an empty vector **before** every other branch, and that an out-of-range
`selected` computes the window as if it were `0` with no cell marked selected; the scenario
asserts the resulting window rather than the absence of a panic.

**M4 — the degenerate-height scenario never produced the interior it asserted about.**
*(Reviewer A)* Frame heights 3 and 4 give a detail interior of **zero** rows (body = h − 2,
interior = body − 2), so "the header row is still drawn where the interior has one" could
never fire. **Fixed:** the scenario samples 120x4, 120x5, 120x6, 120x7, 60x5, 60x6, 60x7,
1x20, and 2x20, and states the four-row cost per interior row, with a distinct assertion at
each height.

**M5 — six existing tests break and the plan named none of them.** *(Reviewer C)*
`the_detail_document_fills_the_interior_at_60_and_120`,
`the_detail_region_stays_blank_while_the_list_fills`, three tests reached through
`view.rs`'s `detail_dashboard` helper (whose `changes` is an empty set, which after this
change draws nothing at all), and `markdown-viewer`'s acceptance test, plus three
`assert_eq!(…, 4)` scroll literals in `src/ui/app.rs`. `make check` at task 10.4 would have
been the first gate to see any of it, seven groups after the decision. **Fixed:** task 9.2
names all six with their before and after expectations and is a task in its own right, and
task 10.1 names the acceptance-test and `app.rs` literal updates explicitly.

**M6 — the Test Boundaries table's filesystem row was wrong in both directions.**
*(Reviewer A)* It named `ui::tests::detail` as a real-filesystem test (it uses the recording
reader and no filesystem — the matrix said so itself) and omitted `ui::tests::load::`, which
does use a real `ScratchDir` and, after task 11.2, the real `read_artifact`. **Fixed.**

**M7 — `design.md`'s Visual Design showed a twelve-artifact bar that omits the selected tab.**
*(Reviewer A)* The caption said "the last tab selected" and the drawing showed indices 7–10.
By the stated rule `start` is 8. **Fixed**, and the `learning-tool` header line and the
problem line were rebuilt with python so every drawn row is exactly 58 columns and the
problem line ends in the `…` the shared truncation rule produces.

**M8 — `ui::detail::tests:: 22` did not match the 19 tests the groups enumerate.**
*(Reviewer B)* **Fixed:** the floor is 19 (5 + 9 + 5), and groups 4, 5, and 6 raise it to 5,
14, and 19 in step.

**M9 — `ui::tests::load:: 7` was unreachable**, because task 11.2 *extended* the six existing
tests and added no test function. *(Reviewer B)* **Fixed:** 11.1 is now the update task (with
its own Red-when: the count must **not** rise there) and 11.2 adds exactly one new test
function, the written-nothing proof over the real binding.

**M10 — `ui::driver::tests:: 13` did not match the two new driver tests.** *(Reviewer B)*
**Fixed:** the floor is 12, and it is asserted in group 11 rather than group 10, where the
tests actually land.

**M11 — task 15.3's whole-suite floors used three inconsistent addition counts** (620 ⇒ +55,
240 ⇒ +68, the per-module table ⇒ +61), making `ui:: 240` unreachable. *(Reviewer B)*
**Fixed:** every floor is now derived from one measured base and one addition —
`565 + 53 = 618` for the library and `172 + 53 = 225` for `ui::` — and the 53 is itemised in
the test-module table.

**M12 — task 0.2's script count was wrong and self-contradictory** ("eighteen … — ten, in
fact"). *(Reviewer B)* **Fixed:** eleven reproduced plus nine extracted equals **twenty**,
both lists are written out, and the task requires `ls -1 "$CHECKS"` verbatim in the record.

**M13 — task 0.1 never measured the three per-submodule counts inside `src/ui/mod.rs`**,
despite three floors depending on them and the file's own "every floor is measured" claim.
*(Reviewer B)* **Fixed:** 0.1 measures `ui::tests::load::` (6), `ui::tests::detail::` (1),
and `ui::tests::start::` (6), plus the `ui::` subtotal (172) and `NODEFAULT-UI` half B's span
count (68).

**M14 — task 1.2's Red-when could never go red** (a grep count that only rises, and five of
whose hits are signatures). *(Reviewer B)* **Fixed:** the gate is the compiler plus the edited
`NODEFAULT-UI`, and the grep number is explicitly demoted to orientation.

**M15 — `design.md`'s matrix named filters and floors `tasks.md` never runs**
(`ui::app::tests::sync 12`, `read_artifact 1`). *(Reviewer B)* **Fixed:** the matrix was
regenerated so every command column is the exact invocation `tasks.md` uses.

**M16 — `READSEAM` was invoked at its stale default `UI_MIN=7`** in two tasks. *(Reviewer B)*
**Fixed:** `UI_MIN=9` at every invocation, and the measured baseline (8 files under `src/ui/`
excluding `mod.rs`) is recorded in 0.1.

**M17 — seven `testcount` tasks never sourced `TESTCOUNT.sh`**, which only *defines* a shell
function. *(Reviewer B)* **Fixed:** every one now begins `. $CHECKS/TESTCOUNT.sh`.

### MINOR, fixed

- The zero-artifact placeholder had no defined behaviour below twelve columns → it is
  right-truncated by the shared rule, with a scenario at width 8.
- `split_detail`'s degenerate-height `y` values were unpinned → pinned at `interior.y`,
  `interior.y + 1`, and `interior.y + 2`.
- One scenario was duplicated between `artifact-content` and `dashboard-loop` (the same
  `NOIO-VIEW` run) → removed from `artifact-content`, which now defers to `dashboard-loop`
  for that claim. Scenario count 80 → 79.
- The proposal's Docs list omitted Architecture, Responsive layout, and View tests → all
  three added.
- The proposal said only `pad_or_truncate_right` was raised to `pub(crate)`; the design and
  tasks say both it and `progress_cell` → the proposal now says both.
- The proposal did not address `openspec/config.yaml`'s "mark as **BREAKING** any change to a
  keybinding" rule head-on → a bullet now states that `1`–`9`, `[`, and `]` all mapped to
  `Action::Ignore` before this change, so nothing that worked before behaves differently.
- `run_loop`'s call-site count was unstated → **ten**, measured by Reviewer C with rustc, and
  now named in the proposal, the design's Migration Plan, and task 1.5.
- `ui::read_artifact` was written in group 1 but not tested until group 11 → its two
  assertions move to group 1, RED before GREEN, and group 11 keeps only the startup path.
- `detail_destructures_into_exactly_two_fields` becomes a lie when `Detail` grows to five
  fields → task 1.3 requires the rename.
- Task 12.3's control 12 assumed rustfmt spreads a five-field pattern over three lines; it is
  seven → the control now says seven and requires `cargo fmt --check` to accept the plant.
- Three new controls added: the path-qualified `impl Default for crate::ui::app::Detail`
  (which the pre-edit half A silently missed), a range expression proving half B does not
  false-positive, and a vacuity control on half B's `SCAN_MIN` guard.
- A new task 0.2b diffs the five "comments move, code does not" blocks against
  `markdown-viewer`'s versions with comment lines stripped, so "byte-identical executable
  logic" is proven rather than asserted.
- `SPEC.md` corrections went from eight to **ten**: Reviewer A found that the change also
  makes Testing → View tests wrong (it names `ui::markdown` and `ui::view` as the modules
  asserting the 58/78 pair; `src/ui/detail.rs` is a third) and that the no-change-selected
  degraded state deserves its own table row rather than being folded into another.
  Correction 6's "before" claim was overstated — two rows already say the artifact list is
  empty — and is now scoped to what they do *not* say.

### Verified correct, and therefore not repaired

- **Every arithmetic claim in the acceptance test.** Reviewer C traced task 2.1 through the
  real `split_frame`, `split_body`, `interior`, `markdown::lines`, and `scroll_offset`: the
  content area is fourteen rows, `scroll == 6`, `- line-06` on the first content row and
  `- line-19` on the last, at **both** 120x20 and 60x20.
- **The tab-bar offsets.** Reviewer A recomputed 0, 12, 21, 31, 40 and the 57-column total
  for the five tdd artifacts, the 13/11 cell widths for `artifact-01`…`artifact-12`, and
  confirmed the 78-column window holds strictly more cells than the 58-column one at all
  three sampled selections — so that comparison is a real assertion, not a tautology.
- **The header name-field widths** (65 at 78, 45 at 58) and the empty-schema `()` case.
- **All ten archive-sourced check blocks exist** in the files task 0.1 names. Reviewer B
  grepped each label — no repeat of the `list-view` "check that did not exist" failure.
- **The "byte-identical / one deliberate edit" claims are true.** Reviewer B diffed each
  carried-forward block against `markdown-viewer`'s with comments stripped; only
  `OPENSPEC-UNTOUCHED`'s exclusion path and `NOIO-VIEW`'s 6→7 `PURE` list differ, exactly as
  declared. (`NODEFAULT-UI` is now openly edited, which is C2's fix.)
- **Every measurement in task 0.1** re-measured independently: 565 lib tests, 57/30/12/10/13
  `#[test]` counts, 17/17/17/9/8/19 file counts, 46/29 `Dashboard` and 25/20 `Detail` sites,
  98.04% line coverage over 12,267 lines, and `BASE == HEAD`.
- **`NOIO-VIEW`, `NOTABSEAM`, and `DETAILWIDTHS` fail cleanly** with a named missing-file
  error against the current tree, and `READSEAM` fails as *vacuous* rather than clean — no
  check passes against a tree that does not yet contain what it guards.
- **16 of 18 controls fired** on Reviewer B's first pass, including the empirical confirmation
  that `DETAILWIDTHS` fails **closed** on `78u16` and that `OPENSPEC-UNTOUCHED` catches an
  untracked file where `git diff --exit-code` exits 0.
- **`DEPS` and `GRAPH-SNAP` pass unchanged**, so this change's premise that no dependency,
  feature, or proc-macro moves holds.
- **`clippy::too_many_arguments` does not fire** on `run_loop`'s five parameters (threshold
  7), and `-D warnings` and `fmt --check` are clean on the whole planned shape.
- **No scope bleed.** No `tasks-tab` grouped-checkbox rendering, no `live-refresh` watcher,
  worker thread, or `r` key; the `j` / `k` binding is not re-opened; nothing names
  `changes::from_cli`, `cli::OpenspecCli`, or `cli::HerdrCli`.
- **No PRD non-goal is crossed.** No write to any OpenSpec file, no orchestration, no change
  authoring, no Windows code.

## `SPEC.md` corrections this change makes, before and after

The ten corrections are enumerated in `design.md` → Decisions and applied in task 13.1. Their
before/after text is recorded there at apply time, one entry per correction, in this section.
Running totals across Phase 4: `tui-shell` 9, `list-view` 17, `markdown-viewer` 12,
`detail-view` **10**.

**1. Detail view — the three-way row split.**
Before: "A header carrying change name, schema, and progress (`detail-view`); a tab bar
built from the schema's artifact list, with `1`–`9` / `[` / `]` switching between tabs
(`detail-view`); content below, resolved for whichever artifact the selected tab names
(`detail-view`)."
After: states the interior is split by `layout::split_detail` into a header row, a tab-bar
row, and the remaining content rows, naming the mechanism rather than only the three
things.

**2. Detail view — above-nine and zero-artifact behaviour.**
Before: silent on positions past nine and on an empty artifact list.
After: a new paragraph states `1`–`9` address the first nine positions directly, `[`/`]`
reach every position and clamp at both ends without wrapping, positions past nine carry
no digit in their label, and an empty artifact list renders a single `no artifacts` cell.

**3. Detail view — the no-change-selected state.**
Before: silent on what the region shows when no change is selected.
After: states the region is blank — no header, no tab bar, no content — exactly when the
visible list is empty, and that a selected change always draws a header, a tab bar, and
at least one content line.

**4. Keys — the `1`–`9`/`[`/`]` row.**
Before: `| \`1\`–\`9\`, \`[\`, \`]\` | Switch artifact tab (\`detail-view\`) |`
After: adds that the keys act at both routes (naming why — the wide layout draws the
detail region at the list route too), that `0` is inert, and that while filtering they
type themselves into the query like any other printable key.

**5. Responsive layout — the content area's height.**
Before: "The two constraints above produce four mandated interiors, one pair per
region, each **16 rows** at the mandated 20-row frame: ..." with no further division
stated.
After: appends that the detail region's sixteen rows are further divided by
`layout::split_detail` into a header row, a tab-bar row, and a fourteen-row content
area, and that the scroll clamp is computed against the content area's height, not the
interior's.

**6. Degraded states — an unreadable artifact file, and the two schema rows' tab-bar
consequence.**
Before: the "Schema not vendored" and "Schema unreadable or invalid" rows say the
artifact list is empty but not what the detail region draws for such a change; no row
covers an artifact file that exists and cannot be read.
After: both existing rows gain "The detail region's tab bar renders `no artifacts` for
such a change (`detail-view`)"; one new row states that an unreadable artifact file
still shows its tab, with a `!`-marked problem line above the content, and that "No
content yet" is not also shown.

**7. Degraded states — no change selected.**
Before: no row for a detail region with nothing selected.
After: one new row states the region is blank in that case, and that the list region
already names the empty state.

**8. Architecture — the render seam and the Module map's `ui` row.**
Before (render seam): "Views are pure functions from a `Dashboard` state value to a
ratatui frame. They perform no I/O, so they are tested by rendering into a
`TestBackend` buffer at fixed widths." No mention of an injected reader.
After: appends that `ui::read_artifact` is the crate's third one-line binding to the
real world, alongside `resolve`'s `npm prefix -g` hook and `config::env_lookup`, naming
the injected `&dyn Fn` shape and that `src/ui/mod.rs` is the one place under `src/ui/`
naming `read_to_string`.
Before (Module map, `ui` row): "Views (the change-row grammar and markdown rendering
included ...), layout, the dashboard's own state (selection, the `/` filter, and the
detail scroll offset), key handling, terminal lifecycle, and the event loop".
After: adds the detail region's header/tab-bar/content grammar to what `ui` holds, and
the selected artifact tab and the injected artifact-read binding to the dashboard's own
state.

**9. Testing → Unit-tested modules — `ui::detail`, and `ui::load` corrected.**
Before: names `ui::load` as though it were a module (it is a function in
`src/ui/mod.rs`; there is no `src/ui/load.rs`) and omits `ui::detail` entirely.
After: names `ui::detail` (plain data, no I/O, parameterised by width) alongside the
existing list, and corrects the reference to `ui::mod`'s `load` and `read_artifact`
functions. The `ui::load` half is pre-existing drift `markdown-viewer` did not catch;
fixed here because this change is the one adding a module to that list.

**10. Testing → View tests — `ui::detail` alongside `ui::markdown` and `ui::view`.**
Before: "the detail region's interior at 58 and 78 (`ui::markdown`, `ui::view`)".
After: adds `ui::detail` as a third module asserting that pair directly, so the
paragraph and `DETAILWIDTHS` agree.

Two of the ten are pre-existing drift rather than consequences of this change, and are fixed
here because this is the change that touches the same sentences:

- Testing → Unit-tested modules names **`ui::load`** as though it were a module. It is a
  function in `src/ui/mod.rs`; there is no `src/ui/load.rs`. `markdown-viewer` did not catch
  it.
- Architecture → Module map's `ui` row lists what the module holds and has not been updated
  since `markdown-viewer` added `ui::markdown`.

## No Remaining Implementation-Blocking Gaps

Every spec scenario appears in the design's verification matrix (79 for 79, names matched
mechanically) and has at least one task. No task invents a collaborator the Test Boundaries
table does not name. No code path writes inside `openspec/`, and the one new **read** path
carries its own written-nothing proof over a real filesystem. The change crosses no PRD
non-goal.

`openspec validate detail-view --strict` passes.

## Deferred Non-Blocking Notes

- **`openspec/IMPLEMENTATION-ORDER.md`'s `detail-view` row** should gain a note that
  `tasks-tab` inherits the tab bar and the content area and only replaces `content_lines`'
  markdown branch for one tab. It cannot be edited during implementation:
  `OPENSPEC-UNTOUCHED` forbids writing anywhere under `openspec/` outside this change's own
  directory. Recorded as task 13.4, the way `list-view` (10.4b) and `markdown-viewer` (11.8)
  recorded theirs, and to be done at archive time.
- **`README.md`'s keymap** still owes `j`/`k` at the detail route from `markdown-viewer`, and
  now owes `1`–`9`, `[`, and `]` as well. Deferred to the end of Phase 4 per commit
  `cdcc160`; task 13.4 carries the reminder.
- **A per-change remembered tab** (moving back to a change reopens the tab you were on) is
  recorded in `design.md` → Open Questions as deliberately not built, with the shape it would
  take if real use asks for it.
- **`NODEFAULT-UI`'s edited half B should be promoted** into `openspec-schemas` or into a
  future change's carried-forward set for `src/changes.rs`'s `GATE-MECH1` too: the same
  multi-line blind spot exists there in principle. Out of scope here.

## Change Review (implementation time)

To be filled in at task 14.2: each finding from the `outside-in-tdd-reviewer` subagent, and
its resolution.
