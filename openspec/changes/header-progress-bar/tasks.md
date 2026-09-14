<!-- Every number below names the command that produced it. Every check is written out in
     full and was run against HEAD (cea1b5c + the planning commit); its exit status and the
     relevant output line are recorded beside it. -->

**Baseline at HEAD** — `cargo test --all-features` exit 0, `1311 passed` in the lib target
and 145 more across the integration targets; `make gates` exit 0.

**Group ordering: sequential, no `parallel-after` markers.** Groups 1 through 3 are
dependency-ordered, not narrative-ordered: group 2's `header_row` calls the `pub(crate)`
function group 1 creates, and group 3 asserts the buffer contents group 2 produces. Groups 1
and 2 additionally both write `src/ui/tasks.rs`. Group 4 reads the final line numbers of
groups 1 through 3, so it cannot start before they land.

**Counts this plan depends on** (`grep -rn "gauge_of" src/ tests/`, `grep -c "header_row(" <file>`):

| Fact | Value | Command |
|---|---|---|
| `gauge_of` sites in the crate | 3 — one definition and two calls, all in `src/ui/tasks.rs` | `grep -rn "gauge_of" src/ tests/` |
| `header_row(` call sites | 9 in `src/ui/detail.rs`, 3 in `src/ui/view.rs` | `grep -c "header_row(" src/ui/detail.rs src/ui/view.rs` |
| Gauge glyphs in `src/ui/detail.rs` | 0 (exit 1) | `grep -c '█\|░' src/ui/detail.rs` |
| Gauge glyphs in `src/ui/tasks.rs` | 19 | `grep -c '█\|░' src/ui/tasks.rs` |
| `covers` ranges pointing into the edited files | 4 — `src/ui/detail.rs` ×3, `src/ui/tasks.rs` ×1 | `grep -n 'src/ui/detail.rs:\|src/ui/tasks.rs:' tests/degraded-coverage.toml` |
| Longest change name in the repo | 22 (`foldable-spec-sections`) | `ls openspec/changes/ openspec/changes/archive/ \| sed 's/^[0-9-]\{11\}//' \| awk '{print length, $0}' \| sort -rn \| head -1` |

**The RED check used throughout.** `cargo test` exits 0 when a filter matches nothing, so a
bare filtered run cannot show RED. Every RED task below uses this counted form:

```sh
red () {
  out=$(cargo test --lib "$1" 2>&1 | grep -E "^test result" | head -1)
  n=$(echo "$out" | sed -E 's/.*ok\. ([0-9]+) passed.*/\1/')
  if [ "${n:-0}" -ge 1 ]; then echo "GREEN $1 ($n passed)"; return 0; else echo "RED   $1 (0 matched)"; return 1; fi
}
```

Run at HEAD: `red ui::detail::tests::header_gauge_fill` → `RED (0 matched)`, exit 1;
`red ui::detail::tests::header_unchanged_below_full_band` → `RED`, exit 1;
`red ui::tasks::tests::gauge_agrees_with_header` → `RED`, exit 1;
`red ui::view::tests::detail_header_gauge_on_every_tab` → `RED`, exit 1. Discriminating
control on the same helper: `red ui::detail::tests` → `GREEN (50 passed)`, exit 0.

## 1. The shared gauge run
<!-- kind: behavior -->

- [ ] 1.1 RED: In `src/ui/tasks.rs`, write `gauge_full_iff_complete_at_twelve` (the run is 12
  characters and holds no `░` exactly when `is_complete()`, over 11-of-12, 12-of-12, 0-of-12,
  99-of-100, 100-of-100) and `gauge_is_total_at_guards` (`gauge_of(p, 0)` and
  `gauge_of(0-of-0, 12)` both return the empty string without panicking). Confirm with
  `red ui::tasks::tests::gauge_full_iff_complete_at_twelve` and
  `red ui::tasks::tests::gauge_is_total_at_guards` — both must print `RED (0 matched)` before
  1.3, and the guard test must additionally fail to compile or panic rather than pass once
  written, since `gauge_of` divides by `total` today.
- [ ] 1.2 CHARACTERIZE: Write `progress_bar_output_unchanged` — `progress_bar` over widths
  0..=130 for 5 `Progress` values, plus the literal 78- and 58-column expectations for 4-of-9
  (68- and 48-column gauges holding 30 and 21 `█`). It is GREEN at HEAD by construction and
  must stay green through 1.3: it pins output this change must not move, which is a
  characterization, not a RED state to manufacture.
- [ ] 1.3 GREEN: Raise `gauge_of` from `fn` to `pub(crate) fn` and add the totality guard
  returning `String::new()` at `g == 0 || progress.total == 0`, per design.md → Decisions 5
  and 6. Neither production call site reaches either value, so `progress_bar`'s output must
  not move.
- [ ] 1.4 REFACTOR: Update `gauge_of`'s doc comment — it currently states `total == 0` is
  never passed, which the guard makes stale. State the totality contract and that no call
  site reaches it.
- [ ] 1.5 CHECK: Contract gate. `gauge_of` becomes reachable from a second module; confirm
  `progress_bar` is byte-identical by running `cargo test --lib ui::tasks` and by
  `grep -rn "gauge_of" src/ tests/` returning the same 3 sites plus group 2's new one later.
- [ ] 1.6 Run `cargo test --lib ui::tasks` — 22 tests plus the new ones, no regressions.

## 2. The header's gauge cell
<!-- kind: behavior -->

- [ ] 2.1 RED: In `src/ui/detail.rs`, write the unit tests for scenarios *The full header
  grammar at both mandated interior widths*, *A complete change renders a full gauge and an
  untouched one renders an empty gauge*, *An empty schema name is a cell of two characters,
  not an absent one*, and *A long name is truncated with an ellipsis, never overflowing the
  row* — at 78 and 58, with the name-field widths design.md → Test Strategy names (52/32,
  53/33, 56, 45/25). Confirm each prints `RED (0 matched)`.
- [ ] 2.2 RED: Write `header_cells_drop_whole` over widths 78, 58, 26, 25, 13, 12, 7, 6, 5, 1,
  0 — both boundaries of all four bands — asserting band membership and that no cell is ever
  cut short. Confirm `RED`; a sample inside one band only would not discriminate.
- [ ] 2.3 RED: Write `header_unchanged_below_full_band` — every width 0..=25 against 26
  literal expected strings — and `header_no_tasks` (0-of-0 at nine widths: no `█`/`░`, 68-
  and 48-column name fields). These two are the guard on Decisions 2 and 7; confirm `RED`.
- [ ] 2.4 RED: Write `header_cjk_name` and `header_total_over_adversarial` (5 names × 4
  `Progress` × widths 0..=130; exact `columns`, band order at 26/25/13/12/7/6/1, no gauge at
  `total == 0`). Confirm `RED`.
- [ ] 2.5 GREEN: Add `const HEADER_GAUGE_COLUMNS: u16 = 12;` to `src/ui/detail.rs` and extend
  `header_row` with the gauge cell: branch on `progress.total == 0` before computing the
  budget, name field `width - 3 - schema - 12 - progress`, and the gauge dropped first in the
  order. Per design.md → Decisions 2, 3, 4 and 7.
- [ ] 2.6 GREEN: Write `gauge_agrees_with_header` in `src/ui/tasks.rs`'s test module — the
  direct `gauge_of(p, 12)` result appears space-bounded inside `header_row(…)` at 78 and 58
  for 4-of-9, 7-of-7, and 0-of-7. It lands here rather than in group 1 because it needs both
  sides to exist.
- [ ] 2.7 CHECK: Contract gate. `header_row`'s signature must be unchanged, so `ui::view`'s
  one call site needs no edit: `grep -n "header_row(" src/ui/view.rs` must still show its 3
  sites with the same argument list, and `src/ui/view.rs:150` must be untouched in the diff.
- [ ] 2.8 REFACTOR: Fold the gauge's width arithmetic into the existing `i64` budget
  computation rather than a parallel one, keeping tests green — or state that none was needed.
- [ ] 2.9 Run `cargo test --lib ui::detail` and `cargo test --lib ui::tasks` — no regressions.

## 3. The header gauge in the rendered frame
<!-- kind: behavior -->

- [ ] 3.1 RED: In `src/ui/view.rs`, write `detail_header_names_selection` and
  `detail_header_follows_selection` at 120x20 and 60x20 — row 0 columns 42..119 and 1..58,
  five `█` for 4-of-9, BOLD at `Route::Detail` and DIM at `Route::List`, compared against
  `palette::style(role)` and never a colour literal. Confirm `RED`.
- [ ] 3.2 RED: Write `detail_header_gauge_on_every_tab` — with the `proposal` tab selected the
  heading carries the gauge and no `%` appears anywhere in the frame; switching to the
  tracked-tasks tab leaves the heading's gauge byte-identical. Confirm `RED`; this is the
  scenario the whole change exists for.
- [ ] 3.3 RED: Write `detail_header_archived_change` (`add-auth`, `(spec-driven)`, twelve `█`,
  45/25-column name fields, no date field), `detail_header_empty_visible_list` (every cell a
  default-styled space, no `█`/`░` in frame), and `detail_header_cjk_within_region` (col 59 a
  space at 60 wide; cols 39/41 spaces and col 40 `│` at 120 wide). Confirm `RED`.
- [ ] 3.4 GREEN: Update the 3 existing `header_row(` expectations in `src/ui/view.rs` that
  name the pre-gauge tail. No production code in `src/ui/view.rs` changes — per design.md →
  Contracts, the signature is unchanged; if any production line needs editing, stop and
  record why, because that falsifies Decision 10.
- [ ] 3.5 Run `cargo test --lib ui::view` — 133 tests plus the new ones, no regressions.

## 4. Re-anchor the degraded-coverage `covers` ranges
<!-- kind: operational -->

Groups 1 through 3 insert lines above four `covers` ranges in `tests/degraded-coverage.toml`,
which are line numbers. `validate_covers` only requires a range to be in bounds and hold one
non-comment line, so a slid range stays green while pointing at the wrong code.

- [ ] 4.1 CHECK: Run this at HEAD **before** starting group 1 and save the output as the
  baseline:

  ```sh
  grep -o 'src/[a-z_/]*\.rs:[0-9]*-[0-9]*' tests/degraded-coverage.toml | sort -u |
  while read -r entry; do
    path=${entry%%:*}; span=${entry##*:}; first=${span%%-*}; last=${span##*-}
    printf '%s  %s\n' "$(sed -n "${first},${last}p" "$path" | shasum | cut -c1-12)" "$entry"
  done
  ```

  Negative control, run at planning time: inserting one line inside `header_row` changed the
  sha of all three `src/ui/detail.rs` entries (`diff` exit 1, lines 45–47), while
  `cargo test --test degraded_coverage` still reported `10 passed` — the suite does not see
  it. Reverting the insert made the diff empty again (exit 0). The check fires, is silent
  when clean, and catches what the suite misses.
- [ ] 4.2 CHANGE: Re-run the command after group 3 and, for every entry whose sha moved,
  update its range in `tests/degraded-coverage.toml` so it names the same code as the
  baseline. Expect `src/ui/detail.rs` ×3 and `src/ui/tasks.rs` ×1 to move and nothing else;
  investigate any other mover rather than re-pointing it.
- [ ] 4.3 VERIFY: Re-run the command and diff against the baseline — every sha equal. Then
  `cargo test --test degraded_coverage` green.

## 5. Change Review
<!-- kind: operational -->

- [ ] 5.1 CHECK: Dispatch the `outside-in-tdd-reviewer` subagent against proposal.md, both
  spec deltas, design.md, and the diff. Do not fork this session — the reviewer must not
  inherit the reasoning that produced the code. Concentrate it on: every one of the 19 spec
  scenarios having a test that would go red if the behavior were deleted; the band boundaries
  at 26/25/13/12/7/6/1; and whether any colour literal entered a render assertion.
- [ ] 5.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
  one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 5.3 VERIFY: Confirm no blocking or unowned finding remains.

## 6. Documentation
<!-- kind: operational -->

- [ ] 6.1 Rewrite in `SPEC.md` § Detail view, line 492 (audience: anyone implementing against
  the design contract) — the sentence reads "a header carrying change name, schema, and
  progress in the interior's first row". Name the gauge cell, its fixed 12 columns, and its
  first place in the drop order, and correct "the interior's first row" to the region's
  heading row, which `pane-chrome` made stale. Durable because `SPEC.md` wins over prose
  wherever the two disagree, and this is the sentence a reader learns the header's grammar
  from.
- [ ] 6.2 Rewrite in `AGENTS.md` line 79 (audience: every future session, loaded into each
  one) — "the detail region now shows the selected change's own header (name, schema,
  progress)" becomes name, schema, gauge, progress, and states the durable seam rule: the
  crate has **one** gauge (`ui::tasks::gauge_of`) beside its one progress cell
  (`ui::list::progress_cell`), and a third rendering of a change's progress calls them rather
  than formatting its own. Rewrite in place; do not append a second entry.
- [ ] 6.3 CHECK: Confirm `SPEC.md`'s degraded-states row *A marked tab's artifact resolves to
  no file…* is left **unchanged**, and record why: its claim that the header "still shows the
  counted pair" stays true with a gauge beside it, and its text is the merge key binding it to
  `tests/degraded-coverage.toml`'s `condition` — rewording it would require an identical edit
  at two further sites for no gain in accuracy.
- [ ] 6.4 VERIFY: `cargo test --test doc_contract` and `cargo test --test degraded_coverage`
  green, since both bind documented claims to computed sites.

## 7. Lint & Verify
<!-- kind: operational -->

- [ ] 7.1 CHECK: Confirm the affected tiers are the lib unit tests (`ui::detail`, `ui::tasks`,
  `ui::view`), the contract tier (`doc_contract`, `degraded_coverage`), and the gate tier
  (`COLWIDTH` and `NOIO-VIEW` both sweep `src/ui/detail.rs` and `src/ui/tasks.rs`).
- [ ] 7.2 VERIFY: `make lint` — `cargo clippy --all-targets --all-features -- -D warnings`,
  0 errors. Rust has no separate type-check step; clippy's build is it.
- [ ] 7.3 VERIFY: `make fmt-check` — clean.
- [ ] 7.4 VERIFY: `make gates` — exit 0, including `COLWIDTH OK` naming the nine pure files
  and `OPENSPEC-UNTOUCHED OK` (commit the artifacts first; it fails on untracked files under
  `openspec/`).
- [ ] 7.5 VERIFY: `make test` — green, at or above the 1311 lib tests HEAD reported.
- [ ] 7.6 VERIFY: `make coverage` — at or above the 80% line floor and the production-slice
  floor. Never lower, waive, or exclude; add tests if it falls short.
- [ ] 7.7 VERIFY: `make check` as the single gate — exit 0, naming the failing sub-command if
  it fails.
- [ ] 7.8 VERIFY: `openspec validate header-progress-bar --strict` — valid.
