## Reviewed Artifacts

- `openspec/changes/list-view/proposal.md`
- `openspec/changes/list-view/specs/change-rows/spec.md` (4 requirements, 14 scenarios)
- `openspec/changes/list-view/specs/list-selection/spec.md` (2 requirements, 9 scenarios)
- `openspec/changes/list-view/specs/list-filtering/spec.md` (3 requirements, 12 scenarios)
- `openspec/changes/list-view/specs/dashboard-loop/spec.md` (3 MODIFIED requirements, 14 scenarios)
- `openspec/changes/list-view/specs/responsive-layout/spec.md` (3 MODIFIED requirements, 12 scenarios)
- `openspec/changes/list-view/design.md`
- `openspec/changes/list-view/tasks.md`

Read as context, not reviewed as artifacts: `SPEC.md`, `PRD.md`, `AGENTS.md`,
`openspec/IMPLEMENTATION-ORDER.md`, `openspec/config.yaml`, the landed
`openspec/specs/{dashboard-loop,responsive-layout}/spec.md`, the archived
`openspec/changes/archive/2026-09-04-tui-shell/` and
`openspec/changes/archive/2026-09-04-changes-from-cli/`, and the current `src/` tree.

## Reviewed Against

- This repository HEAD: `f184be4ff061867ace6f0a5ddaf847351b9b8323`
  (`chore(openspec): archive tui-shell change`)
- Sibling repository HEAD: **Not applicable.** This change touches no sibling repository,
  no external service, and no shared identifier. `~/Code/openspec-schemas` supplies the
  `tdd` schema and the orchestration agents by graft, and this change modifies neither.
- Working tree: clean apart from the intentionally included, untracked
  `openspec/changes/list-view/` — the planning package under review.

### How the finding pass was run

Four reviewers, none of which wrote the plan and none of which was a fork of the planning
session, each given the change directory and one slice of the review list. Every one wrote
its findings **incrementally to a scratchpad file** as it confirmed them rather than only
in a final message — two earlier reviewer rounds in this project died on `529 Overloaded`
and lost their reports entirely.

| Reviewer | Slice | Findings |
|---|---|---|
| 1 — check executability | Extract every check block byte-identically, run it against the real tree, then against a planted violation for each thing it claims to catch; hunt for checks structurally incapable of failing; verify every measured baseline | 2 CRITICAL, 4 WARNING, 5 SUGGESTION |
| 2 — coverage and arithmetic | Reimplement the row grammar in Python and machine-compare every literal string, footer, shortened path, viewport value and filter example; scenario → matrix → task coverage; delta completeness; regression risk to landed tests | 2 CRITICAL, 7 WARNING, 6 SUGGESTION |
| 3 — round-2 verification | Re-run every round-1 repair against a planted violation; sweep every landed test for assertions the change falsifies; recount every floor | 0 CRITICAL, 3 WARNING, 3 SUGGESTION |
| 4 — durable-document consistency | `SPEC.md`, `PRD.md`, `AGENTS.md`, roadmap; scope boundary; the plan's own `SPEC.md` corrections, and the ones it missed; a technical opinion on each design decision | 0 CRITICAL, 10 WARNING, 12 SUGGESTION |

Reviewers 1 and 2 ran against the first draft; 3 and 4 ran against the repaired package,
so round 3's "0 CRITICAL" is a re-verification of the round-1 repairs rather than a second
opinion on the same text. Every check the plan carries was extracted and **run**, and every
planted violation was **reproduced**; no finding below is argued-only unless it says so.

## Gaps Found and Fixed

### CRITICAL

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | tasks.md | The plan instructed the implementer to extract `DEPS.sh` byte-identically from `changes-from-cli`'s archive and said it "needs **no** edit". Extracted and run, it **fails on today's tree**: `normal deps are ['ratatui', 'serde_json', 'toml', 'yaml-rust2'], expected ['serde_json', 'toml', 'yaml-rust2']`. `tui-shell` made four edits to its own copy and recorded them **only as prose in a checkbox**. Task 1.2 says "Red when: any fails, in which case this change is not the place to fix it" — so the change would halt on its second task with nowhere to go | The four edits (ratatui in leg 2's `want`; the crossterm undeclared-but-resolved pair; a leg-5 case for ratatui; leg 3 deleted in favour of `GRAPH-SNAP`) applied and the whole block **written out inline**, so no later change re-derives them from an archived checkbox. Verified green on the real tree (legs 1a, 1b, 2a, 2b, 2a-bis, 2c, 4; leg 5 green in ~3 min), and verified able to go red by adding a dependency | tasks.md → Command-level checks, the `DEPS` block; the "Carried forward unchanged" paragraph; tasks 1.1, 1.2, 9.1, 12.7 |
| CRITICAL | tasks.md | Task 7.4 demanded both that `cargo test --all-features` still fail on group 0's acceptance test **and** that `testcount --lib 'ui::tests::load::' 5` pass. The acceptance test lives under that filter, so the two halves are mutually unsatisfiable — reproduced: `testcount` returns 1 on its exit guard, and its `sed` does not match `test result: FAILED.` either | 7.4 now runs `cargo test --all-features --lib ui::tests::load::` and asserts the summary reads **4 passed; 1 failed** with the one failure named; the counted form moved to 8.1, once the loop is closed. Reproduced: `4 passed; 1 failed` is exactly what this harness prints | tasks.md tasks 7.4 and 8.1 |
| CRITICAL | tasks.md | Adding two `Dashboard` fields with `..` forbidden breaks **every** `Dashboard` literal in the crate — 27 landed tests fail to compile — and no task named the sites. Task 2.7's contract gate grepped only `action_for(` and `Action::`, which neither `src/ui/view.rs` nor `src/lib.rs` names, so it could not see two thirds of them. Task 2.8's "fail on exactly the acceptance test" was unreachable | New task **2.2b** names all seven sites, measured: `src/ui/mod.rs:97` and `:105`, `src/ui/app.rs:97` and `:260` (the last a destructuring **pattern**, the site a literal-only sweep loses), `src/ui/view.rs:145`, `src/ui/driver.rs:86`, `src/lib.rs:216` — and records that the grep returns 14 lines of which 7 are not sites. Task 2.7 now greps `Dashboard[[:space:]]*{` as a third interface | tasks.md tasks 2.2b, 2.7, 2.8; proposal.md → Impact ("Test churn"); design.md → Risks |
| CRITICAL | specs/responsive-layout/spec.md | `view::header_says_no_repository` asserts `!buffer_contains(&buf, "/tmp/searched-from")` over the **whole buffer**, and the landed `responsive-layout` scenario it verifies says "neither buffer names the directory the search started from". This change's no-repository body renders exactly that path, on purpose — and the delta did not modify that requirement, so the change would silently falsify a landed spec | The whole `The header names the repository root, shortened from the left when narrow` requirement is now a MODIFIED block with all four scenarios; the "No repository" scenario's final assertion narrows from the buffer to **row 0**, and the test is required to additionally assert the body **does** name it, so the change is a narrowing rather than a weakening | specs/responsive-layout/spec.md; design.md matrix (4 new rows); tasks.md task 6.1 |

### WARNING

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| WARNING | tasks.md | `testcount --lib 'ui::driver::tests::' 7` is the **measured baseline plus zero** — driver has 7 tests today and group 7 adds one, so the floor was already satisfied and the new test unguarded | Floor raised to 8 everywhere; the aggregate raised from +66 to +67 lib tests | tasks.md convention table, 7.4, 12.4; design.md matrix |
| WARNING | tasks.md | `NODEFAULT-UI`'s `TYPES="${TYPES:-…}"` rewrites an explicitly empty `TYPES` back to the default, so the emptiness guard was unreachable; and `TYPES=" "` printed `NODEFAULT-UI OK: no Default for [ ]` — a vacuous pass | `${TYPES-…}` (no colon) plus a `case` guard rejecting whitespace-only. Task 1.3 now requires both `TYPES=""` and `TYPES=" "` to abort | tasks.md `NODEFAULT-UI` block; task 1.3 |
| WARNING | tasks.md | `NODEFAULT-UI`'s `grep -q "struct $T"` is unanchored: renaming `Dashboard` to `DashboardState` makes the positive control **pass** while every leg searches for a name that is gone (reproduced), and an unrelated `FilterState` with a `Default` causes a false failure | Both `struct $T` greps anchored as `struct[[:space:]]+$T[[:space:]]*\{`, and half B's elision grep gained a leading non-identifier guard. Task 1.3 gained the rename control, using `Dashboard` (round 3: a `Filter`-based control is unperformable at 1.3, since `struct Filter` does not exist until group 2) | tasks.md `NODEFAULT-UI` block; task 1.3 |
| WARNING | tasks.md | Task 9.2's planted violations 8 and 9, written as the plan gave them (`#[test] fn only_at_60() { … }` on one line), do **not** make `WIDTHS` or `LISTWIDTHS` fail — both split on a line-anchored `^[ \t]*#\[test\][ \t]*$`. Reproduced both ways. The task tells the implementer to paste source text uncompiled, so this was a false negative control on the two width checks this change adds | Both plants rewritten as three-line fenced blocks with `#[test]` alone on its own line, and the reason recorded in the task | tasks.md task 9.2 |
| WARNING | tasks.md | Task 2.2b's stop condition ("Red when: the grep returns a seventh site") fired on `src/ui/app.rs:260`, a site the plan does handle — the `let Dashboard { … }` destructuring pattern in `dashboard_destructures_into_exactly_five_fields` | The site list is now seven, measured, with the pattern called out as "the one site a literal sweep loses", and the stop condition rephrased to "a site outside those seven" | tasks.md task 2.2b |
| WARNING | specs/change-rows/spec.md | The archived row's narrow-width **drop order** was unspecified, and the drop branch was off by one: reclaiming the progress cell also reclaims its separating space, so the name field is `width − 13` there and `width − 14 − len(progress)` otherwise. The naive reading produced rows of 18 characters at width 19. `rows_never_panic_at_any_width` walked straight through widths 9/10/13/14 asserting only length | Drop order stated (progress, then the date field, then degenerate to the active grammar, then the first `width` characters of `> `) with exact strings at widths 20, 19, 14, 13, 3, 1, 0; a new scenario and a new test `list::archived_narrow_widths_drop_progress_then_date`; the arithmetic written into design.md → Decisions. Reviewer 3 reimplemented the grammar from the spec prose alone and confirmed 11 row shapes × widths 0..60 are each exactly `width` characters | specs/change-rows/spec.md; design.md → Decisions and matrix; tasks.md task 4.3; list floor 16 → 17 |
| WARNING | specs/change-rows/spec.md | The emission-order rule said the empty active list yields "exactly one `Message` row" while two other requirements in the same file mandate two rows (no match) and three (no repository) | The rule now names the row count per state | specs/change-rows/spec.md |
| WARNING | proposal.md | Impact put the `Change` fixtures in `src/lib.rs`'s `testutil`, where `design.md` forbids them and `NOLIT-CHANGE` would fail; and omitted `src/ui/layout.rs` and `src/ui/mod.rs` from the changed-file list | Impact corrected, and the test-churn consequence of the two new `Dashboard` fields added | proposal.md → Impact |
| WARNING | design.md | Task 12.8 runs `openspec validate list-view --strict`, introducing the nvm-installed `openspec` CLI — a collaborator absent from a Test Boundaries table the tasks call exhaustive | A distinct row added for "the `openspec` CLI as a planning tool", stated as real in exactly one place and explicitly separate from the runtime `openspec` binary row, which stays absent by construction | design.md → Test Boundaries |
| WARNING | design.md | The verification matrix named `app::enter_and_esc_move_between_routes`, a test that neither exists nor is planned; the real name is `enter_and_esc_map_to_routes` | Corrected | design.md matrix |
| WARNING | design.md, tasks.md | Five landed `responsive-layout` scenarios appeared in the matrix with no task, so nothing would have re-run them | New task 6.4 runs all eight carried-unchanged tests by name and counts them, and records the two that survive only because `No changes yet` is exactly fourteen characters | tasks.md task 6.4 |
| WARNING | design.md | "`Action` gains six variants and renames one" cannot be reconciled with the spec's "exactly nine actions": it gains five and renames one | Corrected | design.md → Boundaries |
| WARNING | design.md | Two wrong cross-references: the `SPEC.md` corrections are executed in group 10, not 9, and the planted violations are in task 9.2 with the guards proven at 1.3, not "group 8"; and Test Strategy said group 7 closes the outer loop where tasks.md says group 8 | All three corrected | design.md → Test Strategy and → `SPEC.md` corrections |
| WARNING | design.md | The viewport decision claimed centre-then-clamp is "the only such rule that keeps the selection visible at both ends". False: a page-anchored `min((cursor / height) * height, rows − height)` is equally pure and scrolls once per page | The uniqueness claim removed; centre-then-clamp kept for the reason that actually holds (context on both sides of the selection in a 16-row window), with the page-anchored rule recorded as the alternative to try, and `live-refresh` named as the change most likely to want it | design.md → Decisions |
| WARNING | design.md, tasks.md | `SPEC.md` corrections 10–17: the Keys table's `q`, `Enter`, and `Esc` rows are each falsified in filter mode and none was among the nine — correction 4 even *names* the `q → Quit` row as the contradiction and then writes the fix onto the `/` row; "Progress comes from the CLI when available" is false of what the pane renders, since `NOCLI-SHELL` forbids `src/ui/` naming the CLI seam; the frozen "`Enter` and `Esc` move the route at **every** width" is falsified; Unit-tested modules omits `ui::list`; the Fixtures section's "every view change performs no I/O at all" is falsified by this change's own acceptance test (and, unnoticed, by `tui-shell` already); the Module map's `ui` row is stale | Eight further corrections added, and task 10.2 now corrects **five rows** rather than three — a reader consults the Keys table one row at a time, so a qualification on the `/` row does not reach someone looking up `q`. New task 10.2b carries the four non-Keys sentences | design.md → `SPEC.md` corrections (now 17); tasks.md tasks 10.2, 10.2b |

### SUGGESTION — applied

| Source Artifact | Problem | Repair |
|---|---|---|
| specs, tasks | `a-very-long-change-name-that-will-not-fit-here` is 46 characters, not 45 (the derived strings were right; only the prose was wrong) | Corrected in both |
| tasks.md | `NOLIT-CHANGE`'s `MIN` default is the pre-change measurement and was never raised | Run with `MIN=16` at 9.1 and 12.7 |
| tasks.md | `NOLIT-CHANGE` also matches `impl Change {`, `struct Change {`, and doc comments (false positives), and is dodged by an aliased import (false negative) | Known limits written into the block, with the pointer that `GATE-MECH1`'s compile-time companion remains the primary mechanism |
| tasks.md | `OPENSPEC-UNTOUCHED`'s untracked sweep uses `--exclude-standard`, so a **gitignored** runtime write under `openspec/` is invisible — precisely the kind of file someone adds a gitignore line for | A third `--others --ignored --exclude-standard` sweep added, recorded as a deliberate edit to the carried-forward block |
| tasks.md | `WIDTHS`/`LISTWIDTHS` scan `\b(\d+)\b`, which does not see `38u16` | Written into `LISTWIDTHS`' known-limits comment with the instruction to write widths unsuffixed |
| tasks.md | Task 7.1 said "the three existing scratch-tree tests"; there are four, and only two need the new assertions | Corrected, naming which two |
| tasks.md | Task 5.1 cross-referenced task 4.2 for the row strings; they are in 4.3 | Corrected |
| tasks.md | Task 6.4 said "seven" and named eight | Corrected |
| specs/change-rows | Message and problem rows had no truncation rule and were unpinned below three columns, yet `rows_never_panic_at_any_width` asserts an exact width there | Both pinned: same pad-or-truncate rule, and every row of every kind is exactly `width` characters at every width including 0, 1, and 2 |
| specs/change-rows | Two incompatible answers for a `Dashboard` with `repo: None` and non-empty `problems` | Precedence stated (the no-repository block replaces everything), with the note that `ui::load` cannot build such a value anyway — so the rule exists to keep `rows` total over what a *test* can construct |
| tasks.md | The active row's "below two columns" boundary is the one width the narrow test skipped | Widths 3 and 2 added to `a_narrow_width_drops_the_progress_cell_whole` |
| design.md | The `/ filter` hint decision overstated its own cost by a factor of two — two landed scenarios, not four | Corrected |
| tasks.md | `SPEC.md`'s `j`/`k`/arrows row groups four keys as one behaviour, which filter mode makes false | Task 10.2 now says so |
| tasks.md | Nothing would prompt the `IMPLEMENTATION-ORDER.md` edit the widened scope needs — `config.yaml`'s archive guidance covers a row that "split, merged, or moved", not one that widened | New task 10.4b records it, Spec refs cell included, for the archive step to catch |

### SUGGESTION — accepted without repair

- **The `/ filter` hint stays off the default footer.** Adding it rewrites two landed
  scenarios' exact strings for a discoverability gain the filter announces the moment it is
  used. Resolution point recorded in design.md → Open Questions (`degraded-states`).
- **`action_for` keeps a `bool` rather than an `InputMode` enum.** Reviewer 4 is right that
  a second mode is foreseeable — `detail-view`'s own open question names the `j`/`k`
  collision — and that a second `bool` would be the wrong answer to it. It stays a `bool`
  because this change has exactly two modes and speculative generality in a total key map is
  how a key table stops being readable. Recorded in design.md → Open Questions, assigned to
  `detail-view`.
- **"A filter matching no change" goes in the degraded-states table.** Reviewer 4 objects
  that it is a user state, not a degradation, and enlarges what `degraded-states` must
  audit. Accepted as-is: that table's own opening sentence is "every condition renders
  usable content rather than an error screen", and a no-match state is exactly such a
  condition; leaving it out is how a state ends up owned by nobody.
- **AGENTS.md carries the two interior widths in Architecture rules.** Reviewer 4 would
  prefer a narrower home. There is no narrower file — this repository keeps no nested agent
  guidance — and the constraint is genuinely cross-cutting for every later view change.
- **`view::header_omits_the_path_when_too_narrow` survives by exactly zero columns** (a
  16-column frame leaves a 14-column interior and `No changes yet` is 14 characters). Not
  repaired, because making it robust would mean changing a message string to avoid an
  incidental collision; instead task 6.4 records *why* it survives, so a future change that
  edits either string knows what it is about to break.
- Six tests named in tasks.md appear in no design.md matrix row (helper-level tests such as
  `list::rows_preserve_the_change_set_order`); four now do, the rest are intentionally
  below the matrix's granularity.

## `SPEC.md` corrections this change makes

Seventeen, executed by tasks.md group 10 and listed in design.md → "`SPEC.md` corrections
this change makes". Nine came out of implementation planning; eight more came out of this
review. The four load-bearing ones, before and after:

| # | Before (`SPEC.md` today) | After |
|---|---|---|
| 1 | The List view mock, whose widest row is 48 characters, plus `tui-shell`'s caveat that it "illustrates *content*, not width" and that "how a row is shortened to fit 38 columns is `list-view`'s decision" | The real 38-column rendering, plus the row grammar in prose — marker, name field, right-aligned progress cell, and the archived row's ten-column date field — so `agent-attribution` knows which field it subtracts from |
| 4, 10–12 | `\| `q` \| Quit \|`, `\| `Enter` \| Open change detail \|`, `\| `Esc` \| Back to list \|`, `\| `/` \| Filter changes \|`, all unqualified | Each row carries its own filter-mode qualification: `q` types a `q` while filtering, `Enter` accepts the query, `Esc` dismisses one layer, `/` opens the mode and moves the route to the list. A reader consults this table one row at a time |
| 13 | "Progress comes from the CLI when available and from checkbox counts otherwise." | Qualified: the rendered list is file-sourced at every width until `live-refresh` wires the correction in — `NOCLI-SHELL` mechanically forbids `src/ui/` naming the CLI seam. The dual-source model is still the design |
| 14 | "`Enter` and `Esc` move the route at **every** width … Both the `Length(40)`/`Min(0)` constraints and the every-width route rule are frozen here for `list-view`, `detail-view`, and `agent-attribution` to inherit." | The route half is corrected in place — in filter mode `Enter` accepts and `Esc` cancels without touching the route, and `/` now also moves it to `List`. The `Length(40)`/`Min(0)` half of the freeze is untouched and is what the whole row grammar is written against |

The remaining thirteen (2, 3, 5–9, 15–17) are listed in design.md with their reasons: the
archived row's progress cell, the third mock column's ownership, the `j`/`k` navigation
semantics, the layered `Esc` sentence, two new degraded-states rows, the unattributed-agent
footer annotation, `ui::list` in the unit-tested modules list, the Fixtures section's
no-I/O claim, and the Module map's `ui` row.

## No Remaining Implementation-Blocking Gaps

None remain. All four CRITICALs are repaired and each repair was **re-verified by a
reviewer that did not make it**: the inlined `DEPS` block extracts byte-identically and runs
green on the real tree (and red on demand); the hardened `NODEFAULT-UI` fires on all eight
of its controls, including the rename control the unanchored form silently passed; the
archived drop order reproduces from the spec prose alone at every width 0..60; and
`4 passed; 1 failed` is what this harness actually prints for task 7.4.

Coverage is closed in both directions and machine-checked: 61 spec scenarios, 61 rows in
design.md's verification matrix, no orphan either way, and every test or check the matrix
names appears in tasks.md. No task names a collaborator absent from the Test Boundaries
table. The change crosses no `PRD.md` non-goal — it writes nothing, orchestrates nothing,
authors nothing, and assumes no Windows — and the `openspec/` write invariant is proven
twice, by a snapshot comparison in `load::loading_writes_nothing` and by
`OPENSPEC-UNTOUCHED`'s three-way sweep against a base SHA, untracked and gitignored files
included.

One scope decision is deliberate rather than unresolved: rendering repository-level
`ChangeSet::problems` widens the roadmap's `list-view` row. Reviewer 4 checked the argument
and it holds — `degraded-states` is roadmap-mandated to be additive-free, no other change
owns the list's leading rows, and the alternative is a field that is written and never
displayed. Task 10.4b records the `IMPLEMENTATION-ORDER.md` edit for archive time, since
`OPENSPEC-UNTOUCHED` correctly forbids making it now.

## Deferred Non-Blocking Notes

Each is recorded in design.md → Open Questions with the change that resolves it:

- Whether the footer should carry a permanent `/ filter` hint — `degraded-states`, which
  owns the footer and key-hint audit.
- Whether `j` / `k` rebind to scrolling the detail pane at `Route::Detail` — `detail-view`,
  the first change with a competing claim on those keys.
- Whether `action_for`'s second parameter becomes an `InputMode` enum — `detail-view`, for
  the same reason.
- Whether the selection is remembered by name rather than by index across a filter change —
  `live-refresh`, the first change that can replace the `ChangeSet` under the selection.
- Whether the viewport becomes page-anchored rather than centred — recorded in design.md →
  Decisions as the alternative to try, with `live-refresh` named as its likely occasion.

## Change Review (post-implementation, group 11)

An independent `outside-in-tdd-reviewer` — a fresh subagent, not a fork of the
implementing session — reviewed the finished diff (`68b66e9`..`HEAD`, the ten
commits of this change) against the planning artifacts above. It ran `cargo
fmt --check`, `cargo clippy --all-targets --all-features -D warnings`, and
`cargo test --all-features --lib` itself (516 passed, 0 failed; 98.25% line
coverage at review time) rather than trusting the implementer's commit
messages, independently re-derived the row-grammar arithmetic at every
narrow-width boundary the spec pins, re-planted nine of the eleven
architectural-check violations in a throwaway copy (all nine caught), and ran
34 of its own mutations against `src/ui/app.rs` and `src/ui/list.rs` to find
tests that could not fail (31 of 34 were caught by the existing suite).

**0 CRITICAL. 2 WARNING, both fixed. 6 SUGGESTION: 4 fixed, 2 accepted without repair.**

### WARNING — fixed

| Finding | Repair |
|---|---|
| `esc_dismisses_one_layer_at_a_time` never asserted the route after the *first* `Back`, so "dismiss exactly **one** layer" was untested — a mutation that reset the route unconditionally on every `Back` left the suite green | Added `assert_eq!(d.route, Route::Detail)` after the first `Back`. Reproduced: the planted mutation now fails at that line |
| `action_for_is_total_over_a_keycode_sweep`'s filter-mode sweep explicitly skipped `Up`/`Down`, and nothing else asserted list-filtering's "`Up`/`Down` still navigate while filtering" mapping — deleting both arms from `action_for`'s filter-mode table left the suite green | Added the two assertions to `navigation_and_filter_keys_are_distinguished`. Reproduced: the planted deletion now fails |

### SUGGESTION — fixed

| Finding | Repair |
|---|---|
| `progress_cell_is_dash_when_total_is_zero` only compared the two rows' last characters; a `[0/0]` cell would have passed it | Added `assert!(rows[2].text.contains("[-]"))` |
| `rows()`'s "no-repository block replaces every other row, problem rows included" had no scenario — a guard narrowed to `repo.is_none() && problems.is_empty()` stayed green | Added a case to `the_no_repository_block_is_three_rows` building a `Dashboard` with `repo: None` and a non-empty `problems`, asserting still exactly 3 rows and no `Problem` row |
| `AGENTS.md`'s "Current repo state" still read the pre-change "`q`/`Ctrl-C` to quit and `Enter`/`Esc` to move between the list and detail routes" — the exact unqualified claim `SPEC.md`'s Keys corrections exist to kill, just in the other file; task 10.4 scoped only the "empty bordered frames" sentence, so nothing prompted this one | Rewrote the sentence to name the list-selection and filter-mode keys and note `Enter`/`Esc` route-moving is now qualified to outside filter mode |
| `tasks.md` task 11.1 said "the 56 spec scenarios" (the five delta specs hold 61); `design.md` → Visual Design said "the nine corrections" (the list has seventeen, and was itself misnumbered 15/17/16) | Both corrected; the numbering fixed to 1–17 in order |

### SUGGESTION — accepted without repair

- **Extracted check scripts under the scratchpad are missing each block's leading `# LABEL — …` comment line** — the extractor consumed it as the label. Behaviourally inert (every check body is otherwise byte-identical, every check ran, every planted violation was caught), and the scratchpad is not part of the committed change — nothing to repair in the repository. Recorded so a future run of this change's checks from a fresh scratchpad extraction is not surprised by the same cosmetic gap.
- **`src/ui/driver.rs`'s `filter_mode_is_passed_to_action_for` test renders the dashboard at 120x20 with no assertion**, present only to name the second mandated width per task 7.1's own instruction. `driver.rs` is not covered by the `WIDTHS` check, so nothing mechanically required it — noted as the one place a width literal is decorative rather than load-bearing, not a defect.

No finding was unowned or left blocking. All fixes were re-verified by reproducing the mutation that had previously escaped and confirming it now fails.
