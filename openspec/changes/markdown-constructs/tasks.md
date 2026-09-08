# Tasks — markdown-constructs

Reference: `proposal.md` for motivation, `specs/` for the requirements, `design.md` for the
boundaries, the allocation rule, and every decision cited below.

**Parallelism: none.** Groups 1, 2, and 4 all write `src/ui/markdown.rs`, and groups 3 and 5
both write `src/ui/view.rs`; one file is shared mutable state, and `cargo test` is a
whole-tree gate that would attribute one group's compile failure to the other. Group 6
(documents and the coverage map) reads what groups 1–5 produce. The sequence is real, not
inherited from the narrative.

**Planning-time checks.** Every check below was run against HEAD (`94efd85`) before this plan
shipped, and its exit status and relevant output line are recorded beside it. The baseline for
the whole gate is:

```sh
make check
```

— run at HEAD; its result is recorded in group 7.1 as the state this change must return to.

---

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

The end-to-end wiring **is** the risk here: a table must survive parse → fold → allocation →
wrap → alignment → face → palette → `Buffer::set_string` without reaching a border, and no
unit test on `Vec<Line>` can see the last three steps. Per design.md → Test Strategy.

- [ ] 0.1 Confirm the harness and the replaced collaborators design.md → Test Boundaries names:
  the view tier renders through `render_at` into a `ratatui::backend::TestBackend` at 120x20
  and 60x20, `detail.source` is set in memory, and no filesystem, `openspec` binary, or Herdr
  socket is reached. Verification: the new test compiles against the existing `render_at`
  helper in `src/ui/view.rs`'s test module with no new fixture type.

- [ ] 0.2 RED: Write `a_table_reaches_the_buffer_aligned` in `src/ui/view.rs`'s test module,
  for detail-scroll :: "A table reaches the buffer aligned and inside the region". Assert, at
  both widths: the `|` column offsets of the delimiter row equal those of every drawn row
  line; header cells report `Modifier::BOLD` while pipe and padding cells report none; the
  wrapping row occupies more rows at 60 than at 120; and no table cell lands on column 0, 39,
  40, or the last column.

- [ ] 0.3 Confirm it fails because the behaviour is missing, not because the harness is
  misconfigured. Check, run at HEAD:

  ```sh
  cargo test --all-features --lib a_table_reaches_the_buffer 2>&1 | tail -3
  ```

  HEAD: `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1112 filtered out` —
  the test does not exist, so the RED is real and not a harness fault. The related evidence
  that the behaviour itself is absent, also run at HEAD:

  ```sh
  grep -rn 'ENABLE_TABLES\|ENABLE_STRIKETHROUGH' src/ ; echo "exit=$?"
  ```

  HEAD: no output, `exit=1`.

---

## 1. The table block: parse, fold, and lay out
<!-- kind: behavior -->

- [ ] 1.1 RED: Write the failing unit tests in `src/ui/markdown.rs` for markdown-render's five
  table scenarios: `a_table_that_fits_renders_as_aligned_columns`,
  `a_wide_cell_wraps_within_its_column`, `a_wide_table_allocates_max_min_fairly`,
  `alignment_markers_pad_the_side_they_name`, and `a_ragged_table_keeps_its_declared_columns`.
  Each must name both `58` and `78` unsuffixed, or `MDWIDTHS` rejects it — the gate has no
  exemption list. Check, run at HEAD:

  ```sh
  cargo test --all-features --lib a_table_that_fits 2>&1 | tail -3
  ```

  HEAD: `0 passed; ... 1112 filtered out` — absent, as a RED check for new behaviour must be.

- [ ] 1.2 GREEN: Turn on `Options::ENABLE_TABLES` in `fold`'s `Parser::new_ext` call and add
  the `Tag::Table`/`TableHead`/`TableRow`/`TableCell` and matching `TagEnd` arms, accumulating
  alignments and cells into a `Table` value. Update the two wildcard comments to say which
  variants the new options can now produce. Verification: the fold-level assertions in 1.1's
  tests see the cells; the layout assertions still fail.

- [ ] 1.3 GREEN: Replace `Block`'s `is_rule: bool` with `kind: BlockKind { Flow, Rule,
  Table(Table) }` and dispatch in `emit_block` (design.md → Decision 2). Verification:
  `cargo test --all-features --lib ui::markdown` — every pre-existing markdown test still
  passes, since `Flow` and `Rule` reproduce the previous two paths exactly.

- [ ] 1.4 GREEN: Implement the column allocator: natural widths via `layout::columns`,
  `avail = width - (3n + 1)`, max-min fair capping, remainder by ascending index (design.md →
  Decision 4). Verification: `a_wide_table_allocates_max_min_fairly` passes at both widths.

- [ ] 1.5 GREEN: Implement `emit_table`'s line grammar — the leading `|`, per column
  ` ` + aligned cell + ` ` + `|`, the delimiter line of `-` repeated `w[j] + 2`, header cells
  carrying `strong`, and cell wrapping through the existing `wrap_prose` at `w[j]` columns
  (design.md → Decisions 3, 5, 6, 8). Verification: 1.1's first four tests pass at both widths.

- [ ] 1.6 GREEN: Implement the narrow fallback — one cell per line, wrapped to the full width,
  when `avail < n` (design.md → Decision 7). Verification: write and pass
  `a_narrow_region_renders_one_cell_per_line`, which asserts no `|` appears at widths 1–8 and
  that the pipe grammar returns at 9.

- [ ] 1.7 REFACTOR: Fold the padding-and-pipe assembly through the existing `append` helper so
  adjacent plain segments merge, keeping segment counts in line with every other line kind.
  Verification: tests stay green; `Line::text()` assertions are unchanged.

- [ ] 1.8 Run the group tests — no regressions:

  ```sh
  cargo test --all-features --lib ui::markdown
  ```

---

## 2. Widths, totality, and the narrowed degraded row in `ui::markdown`
<!-- kind: behavior -->

- [ ] 2.1 RED: Extend `composite_fixture` with a three-column table whose widest cell exceeds
  58 columns and with a struck run, so `no_line_exceeds_the_width_it_was_given` sweeps them at
  0, 1, 2, 3, 10, 58, 78, and 200. Extend `lines_is_total_over_arbitrary_input`'s
  `pathological` array with the six new adversarial sources the spec names — unterminated
  `~~struck`, header-only table, over-long row, short row, forty-column table, and a
  500-column CJK cell.

- [ ] 2.2 GREEN: Fix whatever the sweep and the totality test find. Verification: both tests
  pass at every listed width with no panic.

- [ ] 2.3 CHANGE: Narrow `unmodelled_constructs_render_as_source` to the footnote and
  task-list sources, keeping its `ui::tasks::lines` carve-out leg, and delete
  `a_table_renders_as_literal_source_rows`, whose subject has left the unmodelled set. Check,
  run at HEAD:

  ```sh
  cargo test --all-features --lib a_table_renders_as_literal_source_rows 2>&1 | tail -3
  ```

  HEAD: `test ui::markdown::tests::a_table_renders_as_literal_source_rows ... ok` — green at
  HEAD because it pins the behaviour this change replaces, which is why it is deleted rather
  than repaired.

- [ ] 2.4 Run the group tests — no regressions:

  ```sh
  cargo test --all-features --lib ui::markdown
  ```

---

## 3. The strikethrough face
<!-- kind: behavior -->

- [ ] 3.1 RED: Write `a_struck_run_carries_the_face_and_composes` and
  `a_struck_run_split_across_a_wrap_keeps_its_face` in `src/ui/markdown.rs`, both naming 58 and
  78. Check, run at HEAD:

  ```sh
  grep -n 'pub strikethrough' src/ui/markdown.rs ; echo "exit=$?"
  ```

  HEAD: no output, `exit=1` — the field does not exist, so the tests cannot compile, let alone
  pass.

- [ ] 3.2 GREEN: Add `pub strikethrough: bool` to `Face`, turn on
  `Options::ENABLE_STRIKETHROUGH`, and add the `Tag::Strikethrough` / `TagEnd::Strikethrough`
  arms through the existing `push_faced` / `pop_faced` pair. Verification: both 3.1 tests pass.

- [ ] 3.3 GREEN: Add `strikethrough: false` to `src/ui/tasks.rs`'s `heading_line`, which names
  all `Face` fields explicitly. Verification: `cargo build --all-features` — it does not
  compile until this lands, which is the forcing site design.md → Contracts relies on.

- [ ] 3.4 CHECK: Contract gate — re-inspect `ui::markdown::Face` and `ui::palette::Role`
  against design.md → Contracts and confirm both changes are additive and that every named
  consumer (`ui::view::style_for`, `ui::tasks::heading_line`) compiles. Command:

  ```sh
  cargo build --all-features && grep -rn '\.\.Face::plain()' src/ | wc -l
  ```

  Record the count; every one of those sites is unaffected by an added field, and any site not
  matching that pattern is a second forcing site the gate must name.

- [ ] 3.5 Run the group tests — no regressions:

  ```sh
  cargo test --all-features --lib ui::markdown && cargo test --all-features --lib ui::tasks
  ```

---

## 4. The palette role
<!-- kind: behavior -->

- [ ] 4.1 RED: Extend `src/ui/palette.rs`'s tests for view-palette's three role scenarios —
  the exhaustive-`match` role list gains `Strikethrough`, `each_roles_modifier_set_*` gains the
  `CROSSED_OUT` row and the "not `DIM`" discrimination, `the_coloured_set_*` asserts
  `fg: None, bg: None`, and the totality test asserts `Strikethrough`'s style is equal to no
  other role's. Check, run at HEAD:

  ```sh
  grep -n 'Strikethrough' src/ui/palette.rs ; echo "exit=$?"
  ```

  HEAD: no output, `exit=1`.

- [ ] 4.2 GREEN: Add `Role::Strikethrough` and its entry,
  `Style::default().add_modifier(Modifier::CROSSED_OUT)`, with no colour (design.md →
  Decision 9). Verification: 4.1's tests pass; the exhaustive `match` compiles.

- [ ] 4.3 Run the group tests — no regressions:

  ```sh
  cargo test --all-features --lib ui::palette
  ```

---

## 5. `style_for` and the rendered frame
<!-- kind: behavior -->

- [ ] 5.1 RED: Extend `src/ui/view.rs`'s tests for the four affected view scenarios — append
  ` and ~~struck~~` to the sources of `faces_reach_the_buffer_as_styles`,
  `faces_reach_the_buffer_as_coloured_styles`, and
  `a_monochrome_reading_of_the_frame_is_unchanged`, asserting `CROSSED_OUT` with **no**
  foreground; add the struck-bold-link face to `heading_foreground_wins_over_a_code_span`; add
  the `strikethrough == false` assertion to `a_plain_face_is_the_default_style`. No test may
  name a `Color` literal — compare against `palette::style(role)` (`PALETTE` searches
  `src/`, inline test modules included).

- [ ] 5.2 GREEN: Add the `Strikethrough` step to `style_for`'s fold, second, immediately after
  `Quoted` (design.md → Decision 9). Verification: 5.1's tests pass at both widths.

- [ ] 5.3 CHANGE: Extend `content_never_overwrites_the_detail_border` with a twelve-column
  table of 200-character cells, and `a_degenerate_detail_interior_draws_nothing` with a
  table-source repetition, per detail-scroll's two amended scenarios. Verification: both pass
  at 120x20, 60x20, and every degenerate size the existing test already sweeps.

- [ ] 5.4 Run the group tests — no regressions:

  ```sh
  cargo test --all-features --lib ui::view
  ```

---

## 6. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 6.1 VERIFY: Confirm `a_table_reaches_the_buffer_aligned` now passes end to end at both
  widths:

  ```sh
  cargo test --all-features --lib a_table_reaches_the_buffer
  ```

- [ ] 6.2 REFACTOR: Clean up the acceptance test's fixture if group 1's own tests made part of
  it redundant — or state that no refactor was needed.

---

## 7. The degraded-states binding
<!-- kind: operational -->

Four sites must agree or `make check` goes red; design.md → Decision 10 gives the reasoning
and the verdict argument.

- [ ] 7.1 CHECK: Confirm the binding is green at HEAD before touching it, so a later failure is
  attributable to this change:

  ```sh
  cargo test --all-features --test degraded_coverage 2>&1 | tail -3
  ```

  HEAD: `test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out`.

- [ ] 7.2 CHANGE: Narrow `SPEC.md`'s degraded-states row from "(a table, a footnote,
  strikethrough, a task-list item)" to "(a footnote, a task-list item)", and narrow the
  rendering-grammar prose in `SPEC.md` → User interface → Detail view the same way, adding one
  sentence for the table grammar and one for the struck face.

- [ ] 7.3 CHANGE: Edit `tests/degraded-coverage.toml`'s matching entry: `condition` to match
  `SPEC.md`'s new row byte-for-byte, `why` reworded to name what the narrowed proof observes,
  and `covers` re-pointed at the new `Options` line in `src/ui/markdown.rs`. Leave `verdict`
  at `unproven` (design.md → Decision 10). Verification:

  ```sh
  cargo test --all-features --test degraded_coverage
  ```

  It fails on any of: a condition mismatch, a proof that is not a passing `#[test]`, a `covers`
  range that does not resolve, or a `covers` range the suite does not execute.

- [ ] 7.4 VERIFY: Confirm the binding is green again and that the change is not vacuous — the
  row's text differs from HEAD's:

  ```sh
  git diff --stat SPEC.md tests/degraded-coverage.toml && \
    cargo test --all-features --test degraded_coverage
  ```

---

## 8. Gate floors
<!-- kind: operational -->

Per `AGENTS.md`, a gate's floor is its own script default; `NODEFAULT-UI` is the one recorded
multi-subject exception whose floors live on the `Makefile` line.

- [ ] 8.1 CHECK: Record the floors at HEAD so a move is measured, not guessed. Checks, run at
  HEAD:

  ```sh
  /bin/sh scripts/gates/mdwidths.sh 2>&1 | tail -1
  SCAN_MIN=135 /bin/sh scripts/gates/nodefault-ui.sh 2>&1 | tail -2
  ```

  HEAD: `MDWIDTHS OK: all 26 markdown tests name both 58 and 78` (script default `MD_MIN=26`,
  so the floor is exactly met and any added test moves it); `NODEFAULT-UI OK (half B): 150
  literal/pattern spans scanned (>= 135)`.

- [ ] 8.2 CHANGE: Re-run both on the finished tree and raise `scripts/gates/mdwidths.sh`'s
  `MD_MIN` default to the new measured `#[test]` count. Raise the `Makefile`'s view-set
  `SCAN_MIN` only if the measured span count moved; leave it otherwise.

- [ ] 8.3 VERIFY: Confirm the gates still catch a planted defect rather than merely passing:

  ```sh
  cargo test --all-features --test gate_controls
  ```

  HEAD: `test result: ok. 4 passed; 0 failed` in 44.87s — green at HEAD, which is why 8.3 is
  paired with the recorded plants in `tests/gate-controls.toml` rather than standing alone.

---

## 9. Change Review
<!-- kind: operational -->

- [ ] 9.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session —
  with only `proposal.md`, the four spec deltas, `design.md`, `tasks.md`, and the diff.
  Concentration points for this change, beyond the standing ones: (a) every table scenario has
  a test that would go red if `emit_table` were deleted, not one that passes on an empty
  result; (b) the four degraded-row sites agree — `SPEC.md`, `tests/degraded-coverage.toml`,
  the `degraded-coverage` spec's scenario, and the proof test; (c) no test outside
  `src/ui/palette.rs` names a `Color` literal; (d) the narrow fallback and the ragged-row rule
  are exercised at a width where they actually fire, not only asserted; (e) `Face`'s new field
  is named at every site that spells the fields out, found by grep rather than by compile alone.

- [ ] 9.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason,
  note SUGGESTIONs, and re-run the affected tests.

- [ ] 9.3 VERIFY: Confirm no blocking or unowned finding remains.

---

## 10. Documentation
<!-- kind: operational -->

- [ ] 10.1 Rewrite in `SPEC.md`: the markdown rendering-grammar paragraph under User interface
  → Detail view (audience: anyone reading the design contract). Replaces the clause "a
  construct the parser does not model — a table, a footnote, strikethrough, a task-list item —
  renders as its literal source text" with the narrowed set plus one sentence each for the
  table grammar and the struck face. Net change is roughly zero lines: the table grammar
  replaces the clause it makes false. Durable because `SPEC.md` is the contract every later
  change implements, and a stale rendering grammar there is what a future reader would build
  against. Landed as 7.2; listed here so the document is not edited twice.

- [ ] 10.2 Rewrite in `AGENTS.md`: the existing `pulldown_cmark`/`ratatui::style::Color`
  confinement rule under Architecture rules (audience: every agent session, which loads this
  file whole). It currently reads as if the parser is configured for a fixed subset; correct
  the sentence in place to say the option set is `ENABLE_TABLES | ENABLE_STRIKETHROUGH` and
  that turning on a further flag without a rendering path is how a construct **vanishes**
  rather than degrading. Net: rewrite, not add — one sentence replaces one sentence. Durable
  because the wildcard `_ => {}` arms in `fold` make a wrongly-added flag silent, and no gate
  can see it.

---

## 11. Lint & Verify
<!-- kind: operational -->

- [ ] 11.1 CHECK: Inspect the intended verification commands and affected tiers. The affected
  tiers are unit (`ui::markdown`, `ui::palette`), view (`ui::view`), gates (`MDWIDTHS`,
  `MDSEAM`, `PALETTE`, `COLWIDTH`, `NOIO-VIEW`, `NODEFAULT-UI`), and contract
  (`tests/degraded_coverage.rs`, `tests/gate_controls.rs`). All are inside `make check`.

- [ ] 11.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.

- [ ] 11.3 VERIFY: `cargo fmt --all -- --check` — clean.

- [ ] 11.4 VERIFY: `cargo build --all-features` — 0 errors. (Rust's compiler is the type
  checker; there is no separate one.)

- [ ] 11.5 VERIFY: `cargo test --all-features` — green.

- [ ] 11.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` plus the production-slice floor from
  `scripts/coverage-prod.py`. Never lower, waive, or exclude — add tests if it falls short.

- [ ] 11.7 VERIFY: `make check` — the single gate, green. Name the failing sub-command if it
  fails.

- [ ] 11.8 VERIFY: `openspec validate markdown-constructs --strict`.
