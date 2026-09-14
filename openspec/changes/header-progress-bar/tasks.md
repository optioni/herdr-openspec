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

**Group ordering: sequential, no `parallel-after` markers.** Group 0 must precede everything
because it captures values that cease to exist once group 2 lands. Groups 1 through 3 are
dependency-ordered, not narrative-ordered: group 2's `header_row` calls the `pub(crate)`
function group 1 creates, and group 3 asserts the buffer group 2 produces. Groups 1 and 2 both
write `src/ui/tasks.rs`. Group 4 reads the final line numbers of groups 1 through 3.

**Counts this plan depends on:**

| Fact | Value | Command |
|---|---|---|
| `gauge_of` sites in the crate | 3 — the definition plus two calls, both inside `progress_bar` | `grep -rn "gauge_of" src/ tests/` |
| `header_row(` sites in `src/ui/view.rs` | 3 — one production (`:150`), two tests (`:5208`, `:6750`) | `grep -n "header_row(" src/ui/view.rs` |
| Existing tests asserting the pre-gauge grammar | 5, all in `src/ui/detail.rs` | see group 2 |
| Gauge glyphs in `src/ui/detail.rs` / `src/ui/tasks.rs` | 0 / 19 | `grep -c '█\|░' src/ui/detail.rs src/ui/tasks.rs` |
| `covers` ranges pointing into the edited files | 4 — `src/ui/detail.rs` ×3, `src/ui/tasks.rs` ×1 | `grep -n 'src/ui/detail.rs:\|src/ui/tasks.rs:' tests/degraded-coverage.toml` |
| Longest change name in the repo | 22 (`foldable-spec-sections`) | `ls openspec/changes/ openspec/changes/archive/ \| sed 's/^[0-9-]\{11\}//' \| awk '{print length, $0}' \| sort -rn \| head -1` |

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

- [ ] 1.1 RED: In `src/ui/tasks.rs`, write `the_gauge_is_full_exactly_when_the_change_is_complete`,
  `the_promoted_function_is_total_at_both_guard_values`, and the two saturation scenarios —
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
``/// `detail-header` :: "…"`` doc comment; only the expectation changes.

- [ ] 2.1 RED: Rewrite `the_full_header_grammar_at_both_mandated_interior_widths`
  (`src/ui/detail.rs:868`) — name field 65/45 → 52/32, expected tail gains a 12-column gauge
  holding one `█` — and `an_empty_schema_name_is_a_cell_of_two_characters_not_an_absent_one`
  (`:950`), whose `contains("() [1/2]")` the gauge splits; assert the full expected row with
  six `█`. Confirm both report `FAILED` under `redfail`.
- [ ] 2.2 RED: Rewrite `the_cells_are_dropped_whole_in_order_as_the_row_narrows` (`:919`) —
  the `w >= 13` band assertion becomes `w >= 26`, and widths 26 and 25 join the sample so both
  boundaries of the new band are covered — and
  `a_long_name_is_truncated_with_an_ellipsis_never_overflowing_the_row` (`:898`), whose
  `find(" (tdd)")` name-field slice must be re-derived. Confirm `FAILED`.
- [ ] 2.3 RED: Rewrite `a_cjk_change_name_keeps_the_header_inside_its_region_at_both_mandated_widths`
  (`:2162`), where `(tdd)` is no longer immediately before the progress cell, and
  `header_row_is_total_over_adversarial_names_at_every_width` (`:2244`), crossing the five
  names with four `Progress` values — with the band clause scoped to `{4, 9}`, since a 43-column
  progress cell moves every boundary. Confirm `FAILED`.
- [ ] 2.4 RED: Rewrite `a_change_with_no_tasks_still_ends_its_row_in_the_same_column` (`:885`)
  to nine widths with a no-`█`/`░` clause, and write the two new tests
  `a_complete_change_renders_a_full_gauge` and
  `below_the_full_form_band_the_header_is_byte_identical` from 0.2's captured literals. The
  no-tasks rewrite is a CHARACTERIZE, not a RED: Decision 7 means its output must not move.
- [ ] 2.5 GREEN: Add `const HEADER_GAUGE_COLUMNS: u16 = 12;` to `src/ui/detail.rs` and extend
  `header_row` with the gauge cell: branch on `progress.total == 0` before computing the
  budget, name field `width - 3 - schema - 12 - progress`, gauge dropped first. Per design.md →
  Decisions 2, 3, 4 and 7.
- [ ] 2.6 GREEN: Write `the_header_s_gauge_and_the_bar_s_gauge_agree` in `src/ui/tasks.rs`'s
  test module — `gauge_of(p, 12)` appears space-bounded inside `header_row(…)` at 78 and 58 for
  4-of-9, 7-of-7 and 0-of-7. It lands here because it needs both sides to exist.
- [ ] 2.7 CHECK: Contract gate. `header_row`'s signature must be unchanged and
  `src/ui/view.rs:150` untouched in the diff — that is Decision 10, and an edit there falsifies
  it.
- [ ] 2.8 REFACTOR: Fold the gauge's width arithmetic into the existing `i64` budget
  computation rather than a parallel one, or state that none was needed.
- [ ] 2.9 Run `cargo test --lib ui::detail` and `cargo test --lib ui::tasks` — green, with
  every rewritten test passing on its new expectation.

## 3. The header gauge in the rendered frame
<!-- kind: behavior -->

Every test here asserts literal glyph counts and literal tails. None may assert
`assert_eq!(buffer, header_row(…))`: that is the shape already at `src/ui/view.rs:5208` and
`:6750`, which cannot fail on the gauge because it calls the function under test.

- [ ] 3.1 RED: Rewrite `the_header_names_the_selected_change_at_both_mandated_widths`
  (`src/ui/view.rs:5191`) and `moving_the_selection_moves_the_header` (`:5230`) to assert the
  literal tail `(tdd) █████░░░░░░░ [4/9]` and five `█`, with BOLD at `Route::Detail` and DIM at
  `Route::List` compared against `palette::style(role)`. Confirm `FAILED` under `redfail`.
- [ ] 3.2 RED: Rewrite `the_header_reaches_the_buffer_without_crossing_the_region_border` —
  which lives in **`src/ui/detail.rs:2191`**, not `view.rs` — whose `tail = " (tdd) [4/9]"`
  becomes the gauge-bearing tail, keeping its column-indexed slicing. Confirm `FAILED`.
- [ ] 3.3 RED: Rewrite `an_archived_change_s_header_carries_its_stripped_name_and_its_own_schema`
  (`src/ui/view.rs:5257`) for name fields 45/25 and twelve `█`, and
  `an_empty_visible_list_leaves_the_whole_detail_interior_blank` (`:5279`) to add the
  no-`█`/`░`-anywhere clause. Confirm the first reports `FAILED`; the second is a CHARACTERIZE,
  since a blank region must stay blank.
- [ ] 3.4 RED: Write the new `the_gauge_is_present_on_an_artifact_tab` — with the `proposal` tab
  selected the heading carries the gauge and no `%` appears anywhere in the frame; switching to
  the tracked-tasks tab leaves the heading's gauge byte-identical. This is the scenario the
  change exists for. Confirm `FAILED`.
- [ ] 3.5 CHECK: Confirm `src/ui/view.rs` needs no further edit — its two existing `header_row`
  expectations are computed by calling the function and are invariant to this change, so no
  task rewrites them.
- [ ] 3.6 Run `cargo test --lib ui::view` — no regressions.

## 4. Re-anchor the degraded-coverage `covers` ranges
<!-- kind: operational -->

Groups 1 through 3 insert lines above four `covers` ranges in `tests/degraded-coverage.toml`,
which are line numbers. `validate_covers` only requires a range to be in bounds and hold one
non-comment line, so a slid range stays green while naming the wrong code.

- [ ] 4.1 CHANGE: Re-run 0.1's command and, for every entry whose sha moved, update its range
  so it names the same code as `/tmp/covers-baseline.txt`. Expect `src/ui/detail.rs` ×3 and
  `src/ui/tasks.rs` ×1 to move and nothing else; investigate any other mover rather than
  re-pointing it.
- [ ] 4.2 VERIFY: Re-run and diff against the baseline — every sha equal. Then
  `cargo test --test degraded_coverage` green.

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
- [ ] 7.5 VERIFY: `make test` — green, at or above the 1311 lib tests HEAD reported.
- [ ] 7.6 VERIFY: `make coverage` — at or above the 80% line floor and the production-slice
  floor. Never lower, waive, or exclude; add tests if it falls short.
- [ ] 7.7 VERIFY: `make check` as the single gate — exit 0, naming the failing sub-command if
  it fails.
- [ ] 7.8 VERIFY: `openspec validate header-progress-bar --strict` — valid.
