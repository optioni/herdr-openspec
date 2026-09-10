# Change Review

The review that ran **after** implementation. `planning-review.md` is the separate log of the
review that ran before it, and per `tasks.md` → 9.3 nothing below is written there.

## Reviewed Against

- This repository HEAD at review start: `b3cf2be`. Every line number a finding cites is as of
  that commit and drifts under the repairs.
- Baseline for the diff: `0437e05`, the commit at which the planning package was complete.
  13 files, roughly +1869/−1341.
- Working tree: clean at review start.

The finding pass was delegated to one `outside-in-tdd-reviewer` subagent. Per `tasks.md` → 9.2
it was **a fresh agent, not a fork** of any implementing session: a fork inherits the
assumptions under audit and confirms them. It reported findings only and edited nothing; it
probed on a throwaway copy in the scratchpad rather than in this checkout. This session
verified each finding against the tree before acting on it.

## Findings

Severity, owner, and whether it blocks. Repairs are recorded in the row.

| # | Severity | Site | Problem | Blocks | Repair |
|---|---|---|---|---|---|
| 1 | Critical | `src/ui/layout.rs` — `zone` | Hardcodes `Gutters::LeftOnly`, so at the narrow layout the hit test builds a 59-column interior where the draw path built 58. `tab_bar` windows the artifact list by that width, so a click resolves to a tab other than the one drawn, and cells past the drawn bar resolve to an off-screen tab | **yes** | **Resolved** — `81f1608` (RED), `8ce5d4a` (GREEN) |
| 2 | Critical | `src/ui/app.rs` — `normalise_scroll` | Hardcodes `Gutters::Both`, so at the wide layout the scroll clamp is computed against a 77-column content area where the draw uses 78. `content_lines` wraps by width, so `detail.scroll` can be left past the end of a document that fits the content area entirely | **yes** | **Resolved** — `81f1608` (RED), `8ce5d4a` (GREEN) |
| 3 | Critical | `design.md` → Verification matrix | 58 of the 145 rows naming a test name one that exists nowhere in `src/` or `tests/`. The code groups renamed only tests they touched; no task covered the untouched landed tests the matrix also renamed. Task 10.2 fails on every one | **yes** | **Resolved** — `0f496b3`; all 149 rows now name an existing test |
| 4 | Warning | `src/ui/view.rs` — `region_interiors_are_blank` | Sweeps the old bordered rectangle (rows 2–17, columns 41–118). Stays green only because the new blank area is a superset. The scenario's discriminating clause — that the same dashboard with one active change added is no longer blank — is absent, so the test cannot tell blankness from a constant | no |**Resolved** |
| 5 | Warning | `src/ui/view.rs` — `rows_do_not_overwrite_the_borders` | Passes `selected: 0`, which addresses a section header, so the detail region is blank for the whole test and none of the four things the scenario names is exercised. Omits the `Route::Detail` 60x20 leg and the "column 119 does carry content" assertion. `design.md` → Risks names this test as where a future trailing-gutter regression would fail; it no longer would, though `the_divider_has_a_blank_column_on_each_side_at_120_columns` and `content_never_overwrites_the_detail_region_s_border` do cover the property | no |**Resolved** |
| 6 | Warning | `src/ui/layout.rs` — `scroll_offset_is_exact_at_its_boundaries` | Uses height `16` throughout where `detail-scroll` specifies nine tuples at height `14`, and says explicitly that the heights are the content area's fourteen rows rather than the interior's seventeen. `16` is the stale interior figure this change moved | no |**Resolved** |
| 7 | Warning | `src/ui/view.rs`, `src/ui/layout.rs` | One clause of "The divider column is a width branch, not a constant" is asserted nowhere: that in a 100-column buffer the detail interior is `Rect::new(42, 2, 58, 17)`. The divider half is covered by `the_breakpoint_is_exact_at_99_100_and_101_columns` | no |**Resolved** |
| 8 | Warning | `tests/degraded-coverage.toml` | The re-baselined `covers` range for the file-mode row spans a doc comment, a constant and a signature — none of the badge-drawing body. `validate_covers` needs only one non-blank non-comment line in range, so the row passes vacuously and would keep passing if the badge code moved out from under it | no |**Resolved** |
| 9 | Warning | `src/ui/mod.rs` — `detail_interior` | Still returns `Block::bordered().inner(detail)`, a rectangle the draw path no longer computes, with callers correcting it by hand (`interior.y + 4`, `interior.height - 3`, `interior.y - 1`). The arithmetic lands right today, but the helper's doc comment claims it "fails loudly if the frame split ever moves", which is now false | no |**Resolved** |

### Suggestions

- `tasks.md` cited commit `0a5a0f1` for the `src/ui/palette.rs` doc fix; no such object exists.
  The real commit is `e01249b`. **Repaired.** This was the orchestrator's own transcription error.
- **Open, declined for now.** `src/ui/view.rs` — the badge scenario's additive clause asks for a 120x20 row 0 byte-identical
  to the unbadged one, but the test compares columns 1..10 of a 120-column buffer against
  columns 1..10 of a 21-column one. It holds only because both begin `demo-repo`.
- **Resolved** (`58429fd`). `src/ui/view.rs` — `a_selection_past_the_interior_scrolls_the_slice` asserted row 17 held
  `change-27` where `list-selection` asks for the last interior row, now 18, holding
  `change-28`. Asserting a middle row rather than the boundary is the one thing that scenario
  exists to pin, though `the_last_change_is_reachable` does cover row 18 correctly.
- `src/ui/view.rs` — `render_region` recomputes the list interior by hand as
  `area.width.saturating_sub(2)` / `area.x + 1` rather than reading `interior(area,
  Gutters::Both)`: a **fourth** copy of the gutter arithmetic, the same class of duplication
  that produced findings 1 and 2. **Resolved** — `5ab19f4`. The suggestion did not apply as
  stated: `interior` also advances `y` by two and cuts `height` by two, where the heading row
  sits at `area.y`. The repair therefore reads only `interior`'s `.x` and `.width`, which above
  the function's existing early-return guard are arithmetically identical to the hand-written
  pair; 1223 tests pass unchanged, so it removes a duplicate rather than changing behaviour.
- A **fifth** site, found while repairing the first two: `src/ui/driver.rs`'s `tab_bar_row` test
  helper hardcodes `Gutters::Both` unconditionally. Harmless today only because its fixtures
  use artifact sets small enough that windowing is identical at 77 and 78 columns. A sweep of
  `src/ui/*.rs` for hand-written region geometry found no sixth site, so once this one is fixed
  the derivation lives in exactly one place.

### A note on findings 1, 2 and the fourth site

They share one cause, and it is the one `design.md` → D3 predicted. D3 rejected a second
`interior` function because "two functions is two places for the origin clamp to be got wrong";
the change then created four places to choose or recompute the *argument* to the one function.
`render_body` derived it, `zone` and `normalise_scroll` hardcoded opposite constants, and
`render_region` recomputed the arithmetic inline. All 1221 tests passed over both defects,
because no test rendered and hit-tested the narrow detail route together.

`specs/detail-scroll/spec.md` already forbids finding 2 in words — `ui::view::render` and
`Dashboard::normalise_scroll` "cannot derive different content heights from the same frame" —
which is worth recording: the requirement was written, reviewed, and then not verified by any
test.

### Count reconciliation

The reviewer reported 56 rows naming a nonexistent test; this session measured 58, distinct,
with no duplicates, over the same corpus. **Settled at 58** — all 58 were repointed to a landed
test whose body was read and confirmed, and the arbiter agrees: every one of the 149 rows now
names a test that exists, and `tasks.md` → 10.2's conformance check finds nothing. The two-row
discrepancy in the reviewer's count is not worth chasing further now that the population is
empty.

One row is no longer uniquely bound: rows 314 and 326 both name
`the_breakpoint_is_exact_at_99_100_and_101_columns`, which genuinely proves both scenarios. Row
314's own unproven clause is finding 7.

## What Survived Review

Confirmed rather than merely not disproved:

- **The mandated interior widths hold as whole rectangles.**
  `interior_reserves_two_rows_and_the_gutters_its_gutters_names` and
  `the_detail_interior_is_78_columns_at_120_and_58_at_60` assert `Rect::new(1,2,38,17)`,
  `Rect::new(1,2,58,17)` and `Rect::new(42,2,78,17)`. The width gates concur: `LISTWIDTHS` 48
  tests naming 38 and 58, `DETAILWIDTHS` 42, `MDWIDTHS` 34 and `TASKWIDTHS` 22 naming 58 and 78.

## Resolution

Every finding is resolved except one suggestion, declined with its reason.

Repairs landed in `81f1608`, `8ce5d4a`, `0f496b3`, `5ab19f4`, `885f1a3`, `bf6227e`, `58429fd`
and `2ff8bb1`. Two of them changed behaviour; the rest strengthened tests that passed while
proving less than their scenario, or removed a duplicate of arithmetic `ui::layout` owns.

**Only one repair caught a live defect through its own RED step.** Finding 5's test, restored to
the fixture its scenario names, fails on the tree as it stood:

```
assertion `left == right` failed: the wide detail region's gutter-free last column carries content
  left: " "
 right: "x"
```

That is the assertion `design.md` → Risks nominates as the place a future change quietly
re-adding a trailing gutter would be caught, and it was absent. Findings 4, 6 and 9 closed
coverage gaps rather than defects: the geometry under them was already right.

**Declined:** the badge scenario's additive clause compares columns 1..10 of a 120-column buffer
against columns 1..10 of a 21-column one, holding only because both begin `demo-repo`. It is a
weak comparison rather than a wrong one, the property it means to check is covered by the
scenario's other legs, and tightening it is a test-fixture change with no failing case behind
it. Recorded here so a future reader does not mistake the silence for nobody having looked.

**Matrix follow-through.** Finding 7's repair created
`the_detail_interior_is_58_columns_at_the_100_column_breakpoint`, and matrix row 314 — which had
been sharing row 326's test — now binds to it. All 149 rows name an existing test and no test is
named by two rows. Two rows' Tier column said `src/ui/app.rs, src/ui/driver.rs inline` where the
proving test is in `src/ui/view.rs`; both corrected.
