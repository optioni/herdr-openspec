<!-- Every number below names the command that produced it. Every check is written out in
     full and was run against HEAD (0bdcb62); its exit status and the relevant output line
     are recorded beside it. -->

**Baseline at HEAD** — `cargo test --all-features` exit 0, `1311 passed` in the lib target
and 145 more across the integration targets; `make gates` exit 0.

**Twelve of the nineteen scenarios already have passing tests.** This is the fact that shapes
the plan. The repository binds a scenario to a test by snake-casing the scenario header and
carrying a ``/// `<capability>` :: "<scenario header>"`` doc comment, so those twelve tasks are
**rewrites in place** — keep the name, keep the doc comment, change the expectation.

**Corrected while implementing:** the binding *mechanism* is as described, but at HEAD only
**four** of the twelve tests actually carried the doc comment — three in `src/ui/detail.rs` (the
CJK, buffer-border and adversarial-names scenarios) and one in `src/ui/view.rs` (the
bold-and-uncoloured one). The other nine were bound by snake-cased **name** alone. Rewriting in
place is still right, and for the stronger reason: a second test per scenario would collide by
name with one that already exists. Group 2 added the missing doc comment to each of the nine as
part of its rewrite, so all twelve are bound both ways now. Writing a
new test per scenario instead would leave a duplicate pair, one permanently red, and five
existing tests unowned. design.md → Test Strategy carries the full scenario-to-test table; the
tasks below name the file and line for each.

**The measured red set.** Rather than estimate which tests break, a design-conformant gauge was
planted in `header_row` at planning time — 12 columns, first in the drop order, `total == 0`
branch before the budget, reproducing the spec's own strings — and the full suite run:
`cargo test --all-features --no-fail-fast` → `1308 passed; 3 failed` in the lib target.
**Three tests fail, not the twelve the plan first assumed** — plus, under some insertion counts
and not others, a fourth in the contract tier:

| Test | File:line | Why |
|---|---|---|
| `the_full_header_grammar_at_both_mandated_interior_widths` | `src/ui/detail.rs:868` | literal full-row expectation |
| `an_empty_schema_name_is_a_cell_of_two_characters_not_an_absent_one` | `src/ui/detail.rs:950` | literal full-row expectation |
| `the_header_reaches_the_buffer_without_crossing_the_region_border` | `src/ui/detail.rs:2191` | hard-coded `tail = " (tdd) [4/9]"` |
| `every_table_row_has_a_proof` | `tests/degraded_coverage.rs` | **insertion-count-dependent, not a given** — see group 3 |

Nine of the twelve existing tests **pass unchanged**, including all four `src/ui/view.rs`
detail-header tests, because those derive their expectation by calling `header_row` and the
detail-side ones assert width, cell presence and band membership rather than the row's bytes.
That is why the tasks below distinguish RED, RED-by-addition, and CHARACTERIZE: an expectation
edit alone leaves nine of them green before *and* after, which is a check that passes when it
should fail.

**Group ordering: sequential, no `parallel-after` markers.** Group 0 must precede everything
because it captures values that cease to exist once group 2 lands. Group 2 needs the
`pub(crate)` function group 1 creates, and both write `src/ui/tasks.rs`. Group 3 re-anchors line
numbers that only groups 1 and 2 move.

**Group 2 is large, and deliberately not split.** Every test the gauge makes move belongs beside
the implementation that moves it, because the orchestrator gates a behavior group on all tests
passing and a test written after the implementation cannot be RED. An earlier draft split the
view tier into its own later group and produced both faults at once — an unowned red at group 2's
gate, and three downstream tasks labelled RED that would have gone green on sight. The one
alternative, an `acceptance-red` outer-loop group whose gate is a *failing* test, was reconsidered
and still declined: this change adds no wiring — `ui::view::render` already calls `header_row`
and its signature does not move — so the risk it manages is not present here.

**Counts this plan depends on:**

| Fact | Value | Command |
|---|---|---|
| `gauge_of` sites in the crate | 3 — the definition plus two calls, both inside `progress_bar` | `grep -rn "gauge_of" src/ tests/` |
| `header_row(` sites in `src/ui/view.rs` | 3 — one production (`:150`), two tests (`:5208`, `:6750`) | `grep -n "header_row(" src/ui/view.rs` |
| Existing tests that actually fail | 3 in `src/ui/detail.rs`, plus 1 in `tests/degraded_coverage.rs` | measured, see the red set above |
| Gauge glyphs in `src/ui/detail.rs` / `src/ui/tasks.rs` | 0 / 19 | `grep -c '█\|░' src/ui/detail.rs src/ui/tasks.rs` |
| `covers` ranges pointing into the edited files | 4 — `src/ui/detail.rs` ×3, `src/ui/tasks.rs` ×1 | `grep -n 'src/ui/detail.rs:\|src/ui/tasks.rs:' tests/degraded-coverage.toml` |
| Longest change name in the repo | 22 (`foldable-spec-sections`) | `{ ls openspec/changes/; ls openspec/changes/archive/ \| sed 's/^[0-9-]\{11\}//'; } \| sed 's#/$##' \| grep -vE '^\.\|:$\|^$' \| awk '{print length, $0}' \| sort -rn \| head -1` — two `ls` arguments emit a `dirname:` header that wins the numeric sort, so the filter is load-bearing |

**The RED protocol.** `cargo test` exits 0 when a filter matches nothing, so a filtered run
cannot tell "no such test" from "test failed" — and a test whose *name* is new reports the same
thing whether or not the behavior is missing. A RED task below is satisfied only by a run that
**matches and fails**:

```sh
redfail () {
  out=$(cargo test --lib "$1" 2>&1 | grep -E "^test result" | head -1)
  case "$out" in
    *FAILED*)      echo "RED    $1 -> $out"; return 0 ;;
    *"0 passed"*)  echo "ABSENT $1 (filter matched nothing — not RED)"; return 1 ;;
    "")            echo "ABSENT $1 (no result line — not RED)"; return 1 ;;
    *)             echo "GREEN  $1 -> $out (expected a failure)"; return 1 ;;
  esac
}
```

Verified at HEAD on all three branches. `redfail ui::detail::tests::header_zzz` →
`ABSENT (filter matched nothing)`, exit 1. `redfail ui::detail::tests::the_full_header_grammar`
→ `GREEN … 1 passed (expected a failure)`, exit 1. And with `PLANT` appended to that test's
expected string, the same call → `RED … test result: FAILED. 0 passed; 1 failed`, exit 0;
reverting returned it to `GREEN`, exit 1. So the helper reports RED only for a test that
matches and fails — which the previous `0 matched` form could not distinguish from a test that
does not exist, and which is why every RED task in this plan rewrites an expectation rather
than inventing a name.

## 0. Capture the pre-gauge baselines
<!-- kind: operational -->

Everything here records a value that stops existing once group 2 lands. It must run first.

- [x] 0.1 CHECK: Record the `covers` fingerprints, before any edit:

  ```sh
  grep -o 'src/[a-z_/]*\.rs:[0-9]*-[0-9]*' tests/degraded-coverage.toml | sort -u |
  while read -r entry; do
    path=${entry%%:*}; span=${entry##*:}; first=${span%%-*}; last=${span##*-}
    printf '%s  %s\n' "$(sed -n "${first},${last}p" "$path" | shasum | cut -c1-12)" "$entry"
  done > /tmp/covers-baseline.txt
  ```

  Negative control, run at planning time: inserting one line inside `header_row` changed the
  sha of all three `src/ui/detail.rs` entries (`diff` exit 1), while
  `cargo test --test degraded_coverage` still reported `10 passed` — the suite does not see it.
  Reverting made the diff empty again (exit 0). The check fires, is silent when clean, and
  catches what the suite misses.
- [x] 0.2 CHECK: Capture the 26 pre-gauge header strings that
  `below_the_full_form_band_the_header_is_byte_identical` will assert as literals — print
  `header_row("add-token-refresh", "tdd", &Progress { completed: 4, total: 9 }, w)` for every
  `w` in `0..=25` against HEAD's implementation and save the output. These are what makes that
  test "pre-gauge"; recomputing them after group 2 would make it a tautology.
- [x] 0.3 CHECK: Confirm `cargo test --all-features` is green and `make gates` exits 0, so any
  later red is attributable to this change.

## 1. The shared gauge run
<!-- kind: behavior -->

- [x] 1.1 NOTE: `gauge_of(&Progress { completed: 4, total: 9 }, 0)` already returns the empty
  string today — `filled = 4 * 0 / 9 = 0` and both push loops are empty — so the `g == 0` arm of
  1.5's guard codifies existing behaviour. Only the `total == 0` arm removes a division by zero.
- [x] 1.2 RED: In `src/ui/tasks.rs`, write `the_gauge_is_full_exactly_when_the_change_is_complete`,
  `the_promoted_function_is_total_at_both_guard_values` (only its `total == 0` assertion is
  RED; see 1.1), and the two saturation scenarios —
  `the_completeness_property_holds_at_the_saturation_boundary` and
  `a_saturating_progress_renders_a_full_gauge_and_a_full_percentage`. Each must carry the
  `Progress { completed: usize::MAX, total: usize::MAX }` clause at `g` of 12, 48 and 68; that
  clause is what fails against the shipped implementation, since every other value already
  passes. Confirm with `redfail`: each must report `FAILED`, not `ABSENT`.
- [x] 1.3 CHARACTERIZE: Write `the_bar_s_rendered_output_does_not_move` — `progress_bar` over
  widths 0..=130 for 5 `Progress` values, with the 78- and 58-column expectations for 4-of-9
  built independently of `progress_bar` rather than by calling it, per
  `full_grammar_is_byte_identical_to_pre_change_output`'s stated discipline at
  `src/ui/tasks.rs:1211`. Green at HEAD and must stay green through 1.5, except the one
  saturating input 1.3 deliberately moves.

  **Clarified while implementing:** "except the one saturating input" names the scenario 1.2
  writes (`a_saturating_progress_renders_a_full_gauge_and_a_full_percentage`), not a clause of
  this test. A CHARACTERIZE test must be green at HEAD *and* after 1.4/1.5; one carrying the
  post-widening saturating literal would be a second RED task under a CHARACTERIZE label. So
  this test asserts no-panic and fits-width over the whole `0..=130` x 5-`Progress` sweep —
  a bound that holds either side of the widening, since only the fill's *content* moves — plus
  the literal 78- and 58-column strings for the non-saturating 4-of-9 case. The saturating
  case's new content is 1.2's to assert.
- [x] 1.4 GREEN: Widen `gauge_of`'s and `percent_of`'s arithmetic to `u128`, dropping both
  `saturating_mul` calls. Per design.md → Decision 11: saturation yields the wrong quotient
  (`u64::MAX / u64::MAX == 1`), so a complete change renders one filled cell beside `1%`.
  Verify the ordinary values are unmoved — 4/9 at `g` 68 and 48 give 30 and 21, 3/10 at 12
  gives 3 — which 1.2 also pins.
- [x] 1.5 GREEN: Raise `gauge_of` to `pub(crate)` and add the totality guard returning
  `String::new()` at `g == 0 || progress.total == 0`, per design.md → Decisions 5 and 6.
- [x] 1.6 REFACTOR: Update `gauge_of`'s and `percent_of`'s doc comments — they state
  `total == 0` is never passed, name a saturating multiply, and hedge the fill property with
  "given `completed <= total`", all of which 1.3 and 1.4 make stale.
- [x] 1.7 CHECK: Contract gate. `gauge_of` becomes reachable from a second module, and two
  live requirements of `tasks-progress-bar` are MODIFIED; confirm `cargo test --lib ui::tasks`
  is green and that the only `progress_bar` output that moved is the saturating one 1.2
  excepts.
- [x] 1.8 Run `cargo test --lib ui::tasks` — no regressions.

## 2. The header's gauge cell — every test, then the implementation
<!-- kind: behavior -->

**Everything the gauge makes move lives in this one group, including the `src/ui/view.rs` tests.**
It cannot be split. `.claude/agents/apply-orchestrator.md` Step 4 runs the repository's
verification command after each group and, for a behavior group, requires **all tests to pass**
before the next group starts. So a test that goes red when 2.11 lands must be repaired inside
this group — and a test written *after* 2.11 cannot be RED at all, because the behavior it
asserts already exists by then. Splitting the view tier into a later group produced both faults
at once: an unowned red at this group's gate, and three tasks downstream labelled RED that would
have gone green the moment they were written.

The RED evidence splits three ways — see the measured red set above. Only 2.1 and 2.2 fail on an
expectation edit alone; 2.3, 2.4, 2.7 and 2.8 pass unchanged and go red only once the scenario's
new gauge claim is *added*; 2.5 and the second half of 2.8 cannot honestly fail and say so.

Tasks 2.1–2.9 rewrite existing tests in place, each keeping its name and its
``/// `<capability>` :: "…"`` doc comment; only the expectation changes.

- [x] 2.1 RED: Rewrite `the_full_header_grammar_at_both_mandated_interior_widths`
  (`src/ui/detail.rs:868`) — name field 65/45 → 52/32, expected tail gains a 12-column gauge
  holding one `█` — and `an_empty_schema_name_is_a_cell_of_two_characters_not_an_absent_one`
  (`:950`), whose `contains("() [1/2]")` the gauge splits; assert the full expected row with a
  56-column name field at 78 and 36 at 58 (`58 - 3 - 2 - 12 - 5`) and six `█`. Both hold literal
  full-row expectations, so both report `FAILED` under `redfail` on the edit alone.
- [x] 2.2 RED: Rewrite `the_header_reaches_the_buffer_without_crossing_the_region_border`
  (`src/ui/detail.rs:2191`), whose hard-coded `tail = " (tdd) [4/9]"` is sliced at
  `first_col + interior - tail.len()`; it becomes the gauge-bearing tail, keeping the
  column-indexed slicing. This is the third measured red and it belongs here, not downstream: it
  is a `src/ui/detail.rs` test that 2.11 breaks.
- [x] 2.3 RED-by-addition: Rewrite `the_cells_are_dropped_whole_in_order_as_the_row_narrows`
  (`:919`) — `w >= 13` becomes `w >= 26`, widths 26 and 25 join the sample — and
  `a_long_name_is_truncated_with_an_ellipsis_never_overflowing_the_row` (`:898`). Both pass
  unchanged against a gauge-bearing `header_row`, so each MUST also gain the delta's new claims:
  a 12-column gauge present at 78/58/26, "wherever it contains `█` or `░` it contains exactly
  twelve", and for the long name that the gauge is present and intact.
- [x] 2.4 RED-by-addition: Rewrite
  `a_cjk_change_name_keeps_the_header_inside_its_region_at_both_mandated_widths` (`:2162`),
  where `(tdd)` is no longer immediately before the progress cell, and
  `header_row_is_total_over_adversarial_names_at_every_width` (`:2244`), crossing the five names
  with four `Progress` values — band clause scoped to `{4, 9}`, since a 43-column progress cell
  moves every boundary.
- [x] 2.5 CHARACTERIZE: Rewrite `a_change_with_no_tasks_still_ends_its_row_in_the_same_column`
  (`:885`) to nine widths with a no-`█`/`░` clause, and write the new
  `below_the_full_form_band_the_header_is_byte_identical` from 0.2's captured literals — which
  must also assert the row **differs** at 78 and 58, both for `DETAILWIDTHS` and because that is
  what makes it discriminating. `a_change_with_no_tasks_still_ends_its_row_in_the_same_column`
  cannot honestly be RED: Decision 7 fixes the `total == 0` row as unchanged, so it is green at
  HEAD and MUST stay green through 2.11 — it fails against an implementation that reserves the
  gauge's columns before deciding whether it fits.

  **Corrected while implementing:** `below_the_full_form_band_the_header_is_byte_identical` was
  labelled CHARACTERIZE here too, and it is **not**. Its first clause — the 26 literals at widths
  0..=25 — is a characterization that holds either side of 2.11. Its second clause, added during
  planning review so the test would be discriminating and satisfy `DETAILWIDTHS`, asserts the row
  **differs** from its pre-gauge string at 78 and 58; before 2.11 lands `header_row` *is* the
  pre-gauge grammar, so the two sides are identical and the `assert_ne!` fails. Measured:
  `assertion left != right failed: width 78`. It is genuinely RED, and was recorded as RED. The
  stale label is the earlier "cannot be RED" framing not reconciled with the clause that review
  later added.
- [x] 2.6 RED: Write the new `a_complete_change_renders_a_full_gauge` in `src/ui/detail.rs` —
  7-of-7 gives twelve `█` and no `░`, 0-of-7 twelve `░` and no `█`, 6-of-7 at least one `░`.
- [x] 2.7 RED-by-addition: Rewrite `the_header_names_the_selected_change_at_both_mandated_widths`
  (`src/ui/view.rs:5191`) and `moving_the_selection_moves_the_header` (`:5230`) to assert the
  literal tail `(tdd) █████░░░░░░░ [4/9]` and five `█`, with BOLD at `Route::Detail` and DIM at
  `Route::List` compared against `palette::style(role)`. Both pass unchanged today because they
  derive their expectation by calling `header_row`; replacing that derivation with the literal is
  what makes them capable of failing at all.
- [x] 2.8 RED-by-addition: Rewrite
  `an_archived_change_s_header_carries_its_stripped_name_and_its_own_schema` (`:5257`) for name
  fields 45/25 and twelve `█`; then CHARACTERIZE
  `an_empty_visible_list_leaves_the_whole_detail_interior_blank` (`:5279`) by adding the
  no-`█`/`░`-anywhere clause — that one cannot be RED, since a blank region must stay blank.
- [x] 2.9 RED: Write the new `the_gauge_is_present_on_an_artifact_tab` in `src/ui/view.rs` —
  with the `proposal` tab selected the heading carries the gauge and no `%` appears anywhere in
  the frame; switching to the tracked-tasks tab leaves the heading's gauge byte-identical. This
  is the scenario the change exists for, and the one test here that is RED for free.
- [x] 2.10 CHECK: Every `#[test]` written or rewritten in `src/ui/detail.rs` and
  `src/ui/tasks.rs` must contain the bare literals `78` and `58`. `DETAILWIDTHS` and
  `TASKWIDTHS` enforce this over **every** test in those two files with no exemption list, and
  strip only `///` and `//!` lines — so a doc comment cannot satisfy it, and an inline `//`
  naming them would be gaming a gate whose own header calls an exemption list "how a width check
  rots into a rubber stamp". Run `/bin/sh scripts/gates/detailwidths.sh` and
  `/bin/sh scripts/gates/taskwidths.sh` now, not at 6.4.
- [x] 2.11 GREEN: Add `const HEADER_GAUGE_COLUMNS: u16 = 12;` to `src/ui/detail.rs` and extend
  `header_row` with the gauge cell: branch on `progress.total == 0` before computing the budget,
  name field `width - 3 - schema - 12 - progress`, gauge dropped first. Per design.md →
  Decisions 2, 3, 4 and 7.
- [x] 2.12 GREEN: Write `the_header_s_gauge_and_the_bar_s_gauge_agree` in `src/ui/tasks.rs`'s
  test module — `gauge_of(p, 12)` appears space-bounded inside `header_row(…)` at 78 and 58 for
  4-of-9, 7-of-7 and 0-of-7. It lands here because it needs both sides to exist.
- [x] 2.13 CHECK: Contract gate. `header_row`'s signature must be unchanged and
  `src/ui/view.rs:150` untouched in the diff — that is Decision 10, and an edit there falsifies
  it. Confirm too that `:6750` and
  `the_detail_header_is_bold_and_uncoloured_at_both_mandated_widths` (`:6729`) needed no edit:
  both derive their expectation from `header_row` and are invariant to this change.

  **Corrected while implementing:** this task also listed `:5208`, which contradicts 2.7. That
  line is the `let expected = header_row(…)` derivation *inside*
  `the_header_names_the_selected_change_at_both_mandated_widths`, and 2.7 exists precisely to
  replace it with a literal — "replacing that derivation with the literal is what makes them
  capable of failing at all". Being invariant to the change is what makes that test worth
  rewriting, not what exempts it. The contract gate is `src/ui/view.rs:150`, the production call
  site, which is untouched; `:6750` and `:6729` are genuinely unedited.
- [x] 2.14 REFACTOR: Fold the gauge's width arithmetic into the existing `i64` budget computation
  rather than a parallel one, or state that none was needed.
- [x] 2.15 Run `cargo test --lib ui::detail`, `ui::tasks` and `ui::view` — **all green**, which
  is this group's gate. `ui::view` is 132 tests at HEAD plus the one new.
  `cargo test --test degraded_coverage` may or may not fail here depending on how many lines
  2.11 inserted; group 3 owns that either way and it is not a defect in this group's work.

## 3. Re-anchor the degraded-coverage `covers` ranges
<!-- kind: operational -->

This runs **immediately after the two groups that insert lines**, not at the end. Groups 1 and 2
shift four `covers` ranges in `tests/degraded-coverage.toml`, which are line numbers.
`validate_covers` only requires a range to be in bounds and hold one non-comment line, so the
consequence depends entirely on **where the shifted range lands** — not on how far it moves.
Simulated over the real file, the range `src/ui/detail.rs:245-251` lands on code (and so passes
while naming the wrong code) at shifts of 1, 5, 15, 21 and 25, and lands wholly inside a
doc-comment block (failing loudly) only at a shift of about 10:

```
cargo test --test degraded_coverage
  row "An artifact file exists and cannot be read (permission error, I/O error)":
  covers entry src/ui/detail.rs:245-251 holds no line of code (blank or comment-only)
```

So do not expect that failure, and do not treat its absence as evidence the ranges are still
correct — silence is the *likelier* outcome and the more dangerous one. Re-anchor unconditionally
against the baseline shas, which detect the slide either way.

- [x] 3.1 CHANGE: Re-run 0.1's command and, for every entry whose sha moved, update its range so
  it names the same code as `/tmp/covers-baseline.txt`. Expect `src/ui/detail.rs` ×3 and
  `src/ui/tasks.rs` ×1 to move and nothing else; investigate any other mover rather than
  re-pointing it. **Four distinct ranges, five lines to edit**: `src/ui/detail.rs:265-272` is
  written in two `covers` arrays, at `tests/degraded-coverage.toml:76` and again at `:116`, and
  0.1's `sort -u` collapses them — updating one site leaves 3.2's diff non-empty.
- [x] 3.2 VERIFY: Re-run and diff against the baseline — every sha equal — then
  `cargo test --test degraded_coverage` green at `10 passed`.

## 4. Change Review
<!-- kind: operational -->

- [x] 4.1 CHECK: Dispatch the `outside-in-tdd-reviewer` subagent against proposal.md, both spec
  deltas, design.md, and the diff. Do not fork this session. Concentrate it on: every rewritten
  test still carrying its original name and doc comment; no new view test asserting the buffer
  against `header_row`; the band boundaries at 26/25/13/12/7/6/1; and whether any colour
  literal entered a render assertion.
- [x] 4.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
  one-line reason, note SUGGESTIONs, and re-run affected tests.
- [x] 4.3 VERIFY: Confirm no blocking or unowned finding remains.

  **Outcome.** The `outside-in-tdd-reviewer` was dispatched against the artifacts and the diff,
  concentrated on the four failure modes 4.1 names. It reported **no CRITICAL**, one WARNING,
  and five SUGGESTIONs, and confirmed all four concentration areas clean: every rewritten test
  keeps its name and carries a scenario doc comment; no new or rewritten view test asserts the
  buffer against `header_row`; all four band boundaries are entered from both sides; and no
  colour literal reached a render assertion.

  - **WARNING (fixed, `ab727d1`).** The ADDED requirement in `specs/tasks-progress-bar` still
    described the fill as "integer arithmetic with a saturating multiply" — the mechanism the
    MODIFIED requirement above it replaces — and the paragraph beneath it spoke from a
    pre-change vantage. `openspec archive` merges a delta verbatim, so the live capability would
    have shipped one requirement whose two paragraphs disagree. Both restated in `u128` terms.
    This is the failure 5.1/5.2 guard for `## Purpose`, reached by a different route.
  - **SUGGESTIONs 1–4 (applied, `ab727d1`).** Three tests asserted a clause their scenario
    states only indirectly — the 68/48- and 45/25-column name fields, and the BOLD claim's "the
    gauge's own cells included", which sampled only the progress cell's five columns. Each is
    now asserted directly. Four `tasks-progress-bar` scenarios bound only by design.md's table
    gained a doc comment, so the capability is bound both ways throughout.
  - **SUGGESTION 5 (noted, no change).** `percent_of`'s `as u64` cast can wrap for a `Progress`
    with `completed > total`, where the old saturating path clamped. The parser cannot produce
    such a value and both outputs are nonsense at that input, so there is no behaviour to
    preserve and nothing to guard.

  No blocking finding remains, and every finding is owned above.

## 5. Documentation
<!-- kind: operational -->

- [x] 5.1 Rewrite `openspec/specs/detail-header/spec.md`'s `## Purpose` (lines 3–14) — it
  enumerates three cells "with the schema cell and then the progress cell dropped whole", which
  this change makes wrong. A delta carries no Purpose block and `openspec archive` never
  rewrites one from a delta, so an untouched Purpose ships stale; this repository has already
  paid for that twice (commits `9b63ca6`, `a156f9a`). Audience: anyone reading the live
  capability.
- [x] 5.2 Rewrite `openspec/specs/tasks-progress-bar/spec.md`'s `## Purpose` (lines 3–12) —
  it describes the gauge only as part of the tracked-tasks line, never as the crate's one
  shared run with a second renderer. Same archive-time reason as 6.1.
- [x] 5.3 Rewrite `SPEC.md` § Detail view, line 492 — "a header carrying change name, schema,
  and progress in the interior's first row" gains the gauge cell, its fixed 12 columns and its
  first place in the drop order, and "the interior's first row" is corrected to the region's
  heading row, which `pane-chrome` made stale. Audience: anyone implementing against the design
  contract, where `SPEC.md` wins over prose.
- [x] 5.4 Rewrite `AGENTS.md` line 79 — "the selected change's own header (name, schema,
  progress)" becomes name, schema, gauge, progress, and states the durable seam rule: the crate
  has **one** gauge (`ui::tasks::gauge_of`) beside its one progress cell
  (`ui::list::progress_cell`), and a third rendering of progress calls them rather than
  formatting its own. Rewrite in place; do not append.
- [x] 5.5 CHECK: Confirm `SPEC.md`'s degraded-states row *A marked tab's artifact resolves to
  no file…* is left **unchanged**, and record why: its claim that the header "still shows the
  counted pair" stays true with a gauge beside it, and its text is the merge key binding it to
  `tests/degraded-coverage.toml`'s `condition`, so rewording costs an identical edit at two
  further sites for no gain in accuracy.

  **Confirmed.** `SPEC.md:1017` is byte-identical to its pre-change text: the only line this
  change adds containing "counted pair" is the § Detail view prose 5.3 rewrites
  (`git diff 8672d6c -- SPEC.md | grep -E '^[+-].*counted pair'` returns that one added line and
  nothing removed). Its merge key in `tests/degraded-coverage.toml:115` is therefore untouched
  too, and `cargo test --test degraded_coverage` is green at 10 passed.
- [x] 5.6 VERIFY: `cargo test --test doc_contract` and `cargo test --test degraded_coverage`
  green.

## 6. Lint & Verify
<!-- kind: operational -->

- [ ] 6.1 CHECK: Confirm the affected tiers are the lib unit tests (`ui::detail`, `ui::tasks`,
  `ui::view`), the contract tier (`doc_contract`, `degraded_coverage`), and the gate tier
  (`COLWIDTH` and `NOIO-VIEW` both sweep `src/ui/detail.rs` and `src/ui/tasks.rs`).
- [ ] 6.2 VERIFY: `make lint` — `cargo clippy --all-targets --all-features -- -D warnings`,
  0 errors. Rust has no separate type-check step; clippy's build is it.
- [ ] 6.3 VERIFY: `make fmt-check` — clean.
- [ ] 6.4 VERIFY: `make gates` — exit 0, including `COLWIDTH OK` naming the nine pure files and
  `OPENSPEC-UNTOUCHED OK` (commit the artifacts first; it fails on untracked files under
  `openspec/`).
- [ ] 6.5 VERIFY: `make test` — green, at or above the 1311 lib tests HEAD reported. If
  `gate_controls_catch_their_plants` fails with "the real working tree changed while running the
  gate controls", that is its whole-tree digest reacting to a concurrent write, not a defect in
  this change; re-run `cargo test --test gate_controls` with nothing else touching the tree.
- [ ] 6.6 VERIFY: `make coverage` — at or above the 80% line floor and the production-slice
  floor. Never lower, waive, or exclude; add tests if it falls short.
- [ ] 6.7 VERIFY: `make check` as the single gate — exit 0, naming the failing sub-command if
  it fails.
- [ ] 6.8 VERIFY: `openspec validate header-progress-bar --strict` — valid.
