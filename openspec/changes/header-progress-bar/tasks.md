<!-- Every number below names the command that produced it. Every check is written out in
     full and was run against HEAD (0bdcb62); its exit status and the relevant output line
     are recorded beside it. -->

**Baseline at HEAD** — `cargo test --all-features` exit 0, `1311 passed` in the lib target
and 145 more across the integration targets; `make gates` exit 0.

**Twelve of the nineteen scenarios already have passing tests.** This is the fact that shapes
the plan. The repository binds a scenario to a test by snake-casing the scenario header and
carrying a ``/// `<capability>` :: "<scenario header>"`` doc comment, so those twelve tasks are
**rewrites in place** — keep the name, keep the doc comment, change the expectation. Writing a
new test per scenario instead would leave a duplicate pair, one permanently red, and five
existing tests unowned. design.md → Test Strategy carries the full scenario-to-test table; the
tasks below name the file and line for each.

**The measured red set.** Rather than estimate which tests break, a design-conformant gauge was
planted in `header_row` at planning time — 12 columns, first in the drop order, `total == 0`
branch before the budget, reproducing the spec's own strings — and the full suite run:
`cargo test --all-features --no-fail-fast` → `1308 passed; 3 failed` in the lib target, plus one
integration failure. **Four tests fail, not the twelve the plan first assumed:**

| Test | File:line | Why |
|---|---|---|
| `the_full_header_grammar_at_both_mandated_interior_widths` | `src/ui/detail.rs:868` | literal full-row expectation |
| `an_empty_schema_name_is_a_cell_of_two_characters_not_an_absent_one` | `src/ui/detail.rs:950` | literal full-row expectation |
| `the_header_reaches_the_buffer_without_crossing_the_region_border` | `src/ui/detail.rs:2191` | hard-coded `tail = " (tdd) [4/9]"` |
| `every_table_row_has_a_proof` | `tests/degraded_coverage.rs` | a shifted `covers` range — see group 3 |

Nine of the twelve existing tests **pass unchanged**, including all four `src/ui/view.rs`
detail-header tests, because those derive their expectation by calling `header_row` and the
detail-side ones assert width, cell presence and band membership rather than the row's bytes.
That is why the tasks below distinguish RED, RED-by-addition, and CHARACTERIZE: an expectation
edit alone leaves nine of them green before *and* after, which is a check that passes when it
should fail.

**Group ordering: sequential, no `parallel-after` markers.** Group 0 must precede everything
because it captures values that cease to exist once group 2 lands. Groups 1, 2 and 4 are
dependency-ordered, not narrative-ordered: group 2's `header_row` calls the `pub(crate)` function
group 1 creates, and group 4 asserts the buffer group 2 produces. Groups 1 and 2 both write
`src/ui/tasks.rs`. Group 3 re-anchors line numbers that only groups 1 and 2 move, and sits
directly after them so group 4's run is not polluted by a failure neither group caused.

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

- [ ] 0.1 CHECK: Record the `covers` fingerprints, before any edit:

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
- [ ] 0.2 CHECK: Capture the 26 pre-gauge header strings that
  `below_the_full_form_band_the_header_is_byte_identical` will assert as literals — print
  `header_row("add-token-refresh", "tdd", &Progress { completed: 4, total: 9 }, w)` for every
  `w` in `0..=25` against HEAD's implementation and save the output. These are what makes that
  test "pre-gauge"; recomputing them after group 2 would make it a tautology.
- [ ] 0.3 CHECK: Confirm `cargo test --all-features` is green and `make gates` exits 0, so any
  later red is attributable to this change.

## 1. The shared gauge run
<!-- kind: behavior -->

- [ ] 1.1a NOTE: `gauge_of(&Progress { completed: 4, total: 9 }, 0)` already returns the empty
  string today — `filled = 4 * 0 / 9 = 0` and both push loops are empty — so the `g == 0` arm of
  1.4's guard codifies existing behaviour. Only the `total == 0` arm removes a division by zero.
- [ ] 1.1 RED: In `src/ui/tasks.rs`, write `the_gauge_is_full_exactly_when_the_change_is_complete`,
  `the_promoted_function_is_total_at_both_guard_values` (only its `total == 0` assertion is
  RED; see 1.1a), and the two saturation scenarios —
  `the_completeness_property_holds_at_the_saturation_boundary` and
  `a_saturating_progress_renders_a_full_gauge_and_a_full_percentage`. Each must carry the
  `Progress { completed: usize::MAX, total: usize::MAX }` clause at `g` of 12, 48 and 68; that
  clause is what fails against the shipped implementation, since every other value already
  passes. Confirm with `redfail`: each must report `FAILED`, not `ABSENT`.
- [ ] 1.2 CHARACTERIZE: Write `the_bar_s_rendered_output_does_not_move` — `progress_bar` over
  widths 0..=130 for 5 `Progress` values, with the 78- and 58-column expectations for 4-of-9
  built independently of `progress_bar` rather than by calling it, per
  `full_grammar_is_byte_identical_to_pre_change_output`'s stated discipline at
  `src/ui/tasks.rs:1211`. Green at HEAD and must stay green through 1.4, except the one
  saturating input 1.3 deliberately moves.
- [ ] 1.3 GREEN: Widen `gauge_of`'s and `percent_of`'s arithmetic to `u128`, dropping both
  `saturating_mul` calls. Per design.md → Decision 11: saturation yields the wrong quotient
  (`u64::MAX / u64::MAX == 1`), so a complete change renders one filled cell beside `1%`.
  Verify the ordinary values are unmoved — 4/9 at `g` 68 and 48 give 30 and 21, 3/10 at 12
  gives 3 — which 1.2 also pins.
- [ ] 1.4 GREEN: Raise `gauge_of` to `pub(crate)` and add the totality guard returning
  `String::new()` at `g == 0 || progress.total == 0`, per design.md → Decisions 5 and 6.
- [ ] 1.5 REFACTOR: Update `gauge_of`'s and `percent_of`'s doc comments — they state
  `total == 0` is never passed, name a saturating multiply, and hedge the fill property with
  "given `completed <= total`", all of which 1.3 and 1.4 make stale.
- [ ] 1.6 CHECK: Contract gate. `gauge_of` becomes reachable from a second module, and two
  live requirements of `tasks-progress-bar` are MODIFIED; confirm `cargo test --lib ui::tasks`
  is green and that the only `progress_bar` output that moved is the saturating one 1.2
  excepts.
- [ ] 1.7 Run `cargo test --lib ui::tasks` — no regressions.

## 2. The header's gauge cell
<!-- kind: behavior -->

Tasks 2.1–2.4 rewrite existing tests in place. Each keeps its name and its
``/// `detail-header` :: "…"`` doc comment; only the expectation changes. **The RED evidence
splits three ways** — see the measured red set above. Editing an expectation gives honest RED
only for the two literal-expectation tests in 2.1; for 2.2 and 2.3 the existing assertions pass
both before and after, so RED comes only from *adding* the scenario's new gauge claim, and a
task that stops at "update the expectation" will see green and move on.

- [ ] 2.1 RED: Rewrite `the_full_header_grammar_at_both_mandated_interior_widths`
  (`src/ui/detail.rs:868`) — name field 65/45 → 52/32, expected tail gains a 12-column gauge
  holding one `█` — and `an_empty_schema_name_is_a_cell_of_two_characters_not_an_absent_one`
  (`:950`), whose `contains("() [1/2]")` the gauge splits; assert the full expected row with a
  56-column name field at 78 and 36 at 58 (`58 - 3 - 2 - 12 - 5`) and six `█`. Both hold literal full-row expectations, so both report `FAILED` under `redfail` on the
  expectation edit alone.
- [ ] 2.2 RED-by-addition: Rewrite `the_cells_are_dropped_whole_in_order_as_the_row_narrows`
  (`:919`) — `w >= 13` becomes `w >= 26`, widths 26 and 25 join the sample — and
  `a_long_name_is_truncated_with_an_ellipsis_never_overflowing_the_row` (`:898`). Both pass
  unchanged against a gauge-bearing `header_row`, so each MUST also gain the delta's new claims:
  a 12-column gauge present at 78/58/26, "wherever it contains `█` or `░` it contains exactly
  twelve", and for the long name that the gauge is present and intact. Confirm `FAILED` only
  after those clauses are added.
- [ ] 2.3 RED-by-addition: Rewrite
  `a_cjk_change_name_keeps_the_header_inside_its_region_at_both_mandated_widths` (`:2162`),
  where `(tdd)` is no longer immediately before the progress cell, and
  `header_row_is_total_over_adversarial_names_at_every_width` (`:2244`), crossing the five names
  with four `Progress` values — band clause scoped to `{4, 9}`, since a 43-column progress cell
  moves every boundary. Both pass unchanged today; the gauge-presence and no-gauge-at-`total==0`
  clauses are what make them fail. Confirm `FAILED` after adding them.
- [ ] 2.4a CHECK: Every `#[test]` written or rewritten in `src/ui/detail.rs` and
  `src/ui/tasks.rs` must contain the bare literals `78` and `58`. `DETAILWIDTHS` and
  `TASKWIDTHS` enforce this over **every** test in those two files with no exemption list, and
  strip only `///` and `//!` lines — so a doc comment cannot satisfy it and an inline `//`
  naming them would be gaming a gate whose own header calls an exemption list "how a width
  check rots into a rubber stamp". Four planned tests need a real assertion at both widths to
  comply; the specs now carry it. Run `/bin/sh scripts/gates/detailwidths.sh` and
  `/bin/sh scripts/gates/taskwidths.sh` after each group rather than discovering it at 7.4.
- [ ] 2.4 CHARACTERIZE: Rewrite `a_change_with_no_tasks_still_ends_its_row_in_the_same_column`
  (`:885`) to nine widths with a no-`█`/`░` clause, and write the new
  `below_the_full_form_band_the_header_is_byte_identical` from 0.2's captured literals. Neither
  can honestly be RED: Decision 7 fixes the `total == 0` row as unchanged, and the second asserts
  byte-identity below width 26. Both are green at HEAD and MUST stay green through 2.6 — that is
  their whole purpose, and they fail against an implementation that reserves the gauge's columns
  before deciding whether it fits.
- [ ] 2.5 RED: Write the new `a_complete_change_renders_a_full_gauge` — 7-of-7 gives twelve `█`
  and no `░`, 0-of-7 twelve `░` and no `█`, 6-of-7 at least one `░`. Confirm `FAILED`.
- [ ] 2.6 GREEN: Add `const HEADER_GAUGE_COLUMNS: u16 = 12;` to `src/ui/detail.rs` and extend
  `header_row` with the gauge cell: branch on `progress.total == 0` before computing the budget,
  name field `width - 3 - schema - 12 - progress`, gauge dropped first. Per design.md →
  Decisions 2, 3, 4 and 7.
- [ ] 2.7 GREEN: Write `the_header_s_gauge_and_the_bar_s_gauge_agree` in `src/ui/tasks.rs`'s test
  module — `gauge_of(p, 12)` appears space-bounded inside `header_row(…)` at 78 and 58 for
  4-of-9, 7-of-7 and 0-of-7. It lands here because it needs both sides to exist.
- [ ] 2.8 CHECK: Contract gate. `header_row`'s signature must be unchanged and
  `src/ui/view.rs:150` untouched in the diff — that is Decision 10, and an edit there falsifies
  it.
- [ ] 2.9 REFACTOR: Fold the gauge's width arithmetic into the existing `i64` budget computation
  rather than a parallel one, or state that none was needed.
- [ ] 2.10 Run `cargo test --lib ui::detail` and `cargo test --lib ui::tasks` — green.
  `cargo test --test degraded_coverage` may now fail naming a `covers` range; group 3 owns that
  and it is expected here, not a defect in this group's work.

## 3. Re-anchor the degraded-coverage `covers` ranges
<!-- kind: operational -->

This runs **immediately after the two groups that insert lines**, not at the end. Groups 1 and 2
shift four `covers` ranges in `tests/degraded-coverage.toml`, which are line numbers, and the
consequence is nondeterministic rather than merely silent — measured both ways at planning time
against a design-conformant gauge plant:

- a **one-line** insert inside `header_row` left `cargo test --test degraded_coverage` at
  `10 passed`: all three `src/ui/detail.rs` fingerprints moved while the suite saw nothing,
  because each shifted range still happened to hold a line of code;
- the **full** gauge (a ten-line insert) made it fail with
  `row "An artifact file exists and cannot be read (permission error, I/O error)": covers entry
  src/ui/detail.rs:245-251 holds no line of code (blank or comment-only)` — a loud failure
  naming a degraded-states row that has nothing to do with the edit.

`validate_covers` only requires a range to be in bounds and hold one non-comment line, so which
outcome you get depends on where the shifted range lands. Re-anchoring here keeps the confusing
loud case out of group 4's test run.

- [ ] 3.1 CHANGE: Re-run 0.1's command and, for every entry whose sha moved, update its range so
  it names the same code as `/tmp/covers-baseline.txt`. Expect `src/ui/detail.rs` ×3 and
  `src/ui/tasks.rs` ×1 to move and nothing else; investigate any other mover rather than
  re-pointing it. **Four distinct ranges, five lines to edit**: `src/ui/detail.rs:265-272` is
  written in two `covers` arrays, at `tests/degraded-coverage.toml:76` and again at `:116`, and
  0.1's `sort -u` collapses them — updating one site leaves 3.2's diff non-empty.
- [ ] 3.2 VERIFY: Re-run and diff against the baseline — every sha equal — then
  `cargo test --test degraded_coverage` green at `10 passed`.

## 4. The header gauge in the rendered frame
<!-- kind: behavior -->

Every test here asserts literal glyph counts and literal tails. None may assert
`assert_eq!(buffer, header_row(…))`: that is the shape already at `src/ui/view.rs:5208` and
`:6750`, which cannot fail on the gauge because it calls the function under test — which is also
why all four existing `src/ui/view.rs` detail-header tests pass unchanged against a gauge-bearing
header, measured. Only 4.3's new test is RED for free here.

- [ ] 4.1 RED-by-addition: Rewrite `the_header_names_the_selected_change_at_both_mandated_widths`
  (`src/ui/view.rs:5191`) and `moving_the_selection_moves_the_header` (`:5230`) to assert the
  literal tail `(tdd) █████░░░░░░░ [4/9]` and five `█`, with BOLD at `Route::Detail` and DIM at
  `Route::List` compared against `palette::style(role)`. Both pass unchanged today because they
  derive their expectation from `header_row`; replacing that derivation with the literal is what
  produces RED.
- [ ] 4.2 RED-by-addition: Rewrite
  `an_archived_change_s_header_carries_its_stripped_name_and_its_own_schema` (`:5257`) for name
  fields 45/25 and twelve `█`, and CHARACTERIZE
  `an_empty_visible_list_leaves_the_whole_detail_interior_blank` (`:5279`) by adding the
  no-`█`/`░`-anywhere clause — the second cannot be RED, since a blank region must stay blank.
- [ ] 4.3 RED: Rewrite `the_header_reaches_the_buffer_without_crossing_the_region_border` —
  which lives in **`src/ui/detail.rs:2191`**, not `view.rs` — whose hard-coded
  `tail = " (tdd) [4/9]"` is sliced at `first_col + interior - tail.len()`; it is one of the
  three measured genuine reds. Then write the new `the_gauge_is_present_on_an_artifact_tab`:
  with the `proposal` tab selected the heading carries the gauge and no `%` appears anywhere in
  the frame, and switching to the tracked-tasks tab leaves the heading's gauge byte-identical.
  That is the scenario the change exists for. Confirm `FAILED` for both.
- [ ] 4.4 CHECK: Confirm `src/ui/view.rs` needs no further edit beyond 4.1 and 4.2 — its two
  `header_row` expectations at `:5208` and `:6750` are computed by calling the function and are
  invariant to this change, and `the_detail_header_is_bold_and_uncoloured_at_both_mandated_widths`
  (`:6729`) passes unchanged, measured.
- [ ] 4.5 Run `cargo test --lib ui::view` and `cargo test --lib ui::detail` — no regressions;
  `ui::view` is 132 tests at HEAD plus the one new.

## 5. Change Review
<!-- kind: operational -->

- [ ] 5.1 CHECK: Dispatch the `outside-in-tdd-reviewer` subagent against proposal.md, both spec
  deltas, design.md, and the diff. Do not fork this session. Concentrate it on: every rewritten
  test still carrying its original name and doc comment; no new view test asserting the buffer
  against `header_row`; the band boundaries at 26/25/13/12/7/6/1; and whether any colour
  literal entered a render assertion.
- [ ] 5.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
  one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 5.3 VERIFY: Confirm no blocking or unowned finding remains.

## 6. Documentation
<!-- kind: operational -->

- [ ] 6.1 Rewrite `openspec/specs/detail-header/spec.md`'s `## Purpose` (lines 3–14) — it
  enumerates three cells "with the schema cell and then the progress cell dropped whole", which
  this change makes wrong. A delta carries no Purpose block and `openspec archive` never
  rewrites one from a delta, so an untouched Purpose ships stale; this repository has already
  paid for that twice (commits `9b63ca6`, `a156f9a`). Audience: anyone reading the live
  capability.
- [ ] 6.2 Rewrite `openspec/specs/tasks-progress-bar/spec.md`'s `## Purpose` (lines 3–12) —
  it describes the gauge only as part of the tracked-tasks line, never as the crate's one
  shared run with a second renderer. Same archive-time reason as 6.1.
- [ ] 6.3 Rewrite `SPEC.md` § Detail view, line 492 — "a header carrying change name, schema,
  and progress in the interior's first row" gains the gauge cell, its fixed 12 columns and its
  first place in the drop order, and "the interior's first row" is corrected to the region's
  heading row, which `pane-chrome` made stale. Audience: anyone implementing against the design
  contract, where `SPEC.md` wins over prose.
- [ ] 6.4 Rewrite `AGENTS.md` line 79 — "the selected change's own header (name, schema,
  progress)" becomes name, schema, gauge, progress, and states the durable seam rule: the crate
  has **one** gauge (`ui::tasks::gauge_of`) beside its one progress cell
  (`ui::list::progress_cell`), and a third rendering of progress calls them rather than
  formatting its own. Rewrite in place; do not append.
- [ ] 6.5 CHECK: Confirm `SPEC.md`'s degraded-states row *A marked tab's artifact resolves to
  no file…* is left **unchanged**, and record why: its claim that the header "still shows the
  counted pair" stays true with a gauge beside it, and its text is the merge key binding it to
  `tests/degraded-coverage.toml`'s `condition`, so rewording costs an identical edit at two
  further sites for no gain in accuracy.
- [ ] 6.6 VERIFY: `cargo test --test doc_contract` and `cargo test --test degraded_coverage`
  green.

## 7. Lint & Verify
<!-- kind: operational -->

- [ ] 7.1 CHECK: Confirm the affected tiers are the lib unit tests (`ui::detail`, `ui::tasks`,
  `ui::view`), the contract tier (`doc_contract`, `degraded_coverage`), and the gate tier
  (`COLWIDTH` and `NOIO-VIEW` both sweep `src/ui/detail.rs` and `src/ui/tasks.rs`).
- [ ] 7.2 VERIFY: `make lint` — `cargo clippy --all-targets --all-features -- -D warnings`,
  0 errors. Rust has no separate type-check step; clippy's build is it.
- [ ] 7.3 VERIFY: `make fmt-check` — clean.
- [ ] 7.4 VERIFY: `make gates` — exit 0, including `COLWIDTH OK` naming the nine pure files and
  `OPENSPEC-UNTOUCHED OK` (commit the artifacts first; it fails on untracked files under
  `openspec/`).
- [ ] 7.5 VERIFY: `make test` — green, at or above the 1311 lib tests HEAD reported. If
  `gate_controls_catch_their_plants` fails with "the real working tree changed while running the
  gate controls", that is its whole-tree digest reacting to a concurrent write, not a defect in
  this change; re-run `cargo test --test gate_controls` with nothing else touching the tree.
- [ ] 7.6 VERIFY: `make coverage` — at or above the 80% line floor and the production-slice
  floor. Never lower, waive, or exclude; add tests if it falls short.
- [ ] 7.7 VERIFY: `make check` as the single gate — exit 0, naming the failing sub-command if
  it fails.
- [ ] 7.8 VERIFY: `openspec validate header-progress-bar --strict` — valid.
