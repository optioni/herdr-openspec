# Tasks — markdown-constructs

Reference: `proposal.md` for motivation, `specs/` for the requirements, `design.md` for the
boundaries, the allocation rule, and every decision cited below.

**Parallelism: none, and here is the file map it rests on.** `src/ui/markdown.rs` is written
by groups **1, 2, and 3**; `src/ui/view.rs` by groups **0, 5, and 6**; `src/ui/palette.rs` by
group **4** alone; `SPEC.md` and `tests/degraded-coverage.toml` by group **7**; the gate
scripts and the `Makefile` by group **8**. Group 4 is the only group sharing no file with any
other — but its own verification, `cargo test --lib ui::palette`, compiles the whole crate, so
a half-written `src/ui/markdown.rs` from group 1 or 2 would fail group 4's gate and be
attributed to group 4. Criterion 3 therefore fails for it, and every other pair fails
criterion 1. Groups 7 and 8 are the closest call — different files, both depending only on
group 6 — and they stay sequential because group 8 measures floors on the **finished** tree
and group 8.3 copies that tree into a scratch directory, which a mid-edit `SPEC.md` would be
copied into.

**Planning-time checks.** Every check below was run against `94efd85` or `e63442b` before this
plan shipped, and its exit status and relevant output line are recorded beside it. `make check`
was run at `e63442b` and fails on exactly one gate — `OPENSPEC-UNTOUCHED`, because the
planning artifacts were untracked at the time. Committing them clears it; every other gate in
the recipe passed. That is the state this change must return to.

---

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

The end-to-end wiring **is** the risk: a table must survive parse → fold → allocation → wrap
→ alignment → face → palette → `Buffer::set_string` without reaching a border, and no unit
test on `Vec<Line>` can see the last three steps. Per design.md → Test Strategy.

- [x] 0.1 Confirm the harness and the replaced collaborators design.md → Test Boundaries
  names: the view tier renders through `render_at` into a `ratatui::backend::TestBackend` at
  120x20 and 60x20, `detail.source` is set in memory, and no filesystem, `openspec` binary, or
  Herdr socket is reached. Verification: the new test compiles against the existing `render_at`
  helper in `src/ui/view.rs`'s test module with no new fixture type.

- [x] 0.2 RED: Write `a_table_reaches_the_buffer_aligned` in `src/ui/view.rs`'s test module,
  for detail-scroll :: "A table reaches the buffer aligned and inside the region". Assert, at
  both widths: the `|` column offsets of the delimiter row equal those of every drawn row
  line; header cells report `Modifier::BOLD` while pipe and padding cells report none; the
  wrapping row occupies more rows at 60 than at 120; and the border columns **per buffer** —
  0/39/40/119 at 120 columns, 0/59 at 60 columns. The narrow buffer's list is different
  because below the breakpoint the detail region takes the whole body; asserting 39 and 40
  there would leave this test permanently red against a correct implementation.

- [x] 0.3 Confirm it fails because the behaviour is missing, not because the harness is
  misconfigured. Check, run at HEAD:

  ```sh
  cargo test --all-features --lib a_table_reaches_the_buffer 2>&1 | tail -3
  ```

  HEAD: `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1112 filtered out` — the
  test does not exist, so the RED is real. That the behaviour itself is absent, also at HEAD:

  ```sh
  grep -rn 'ENABLE_TABLES\|ENABLE_STRIKETHROUGH' src/ ; echo "exit=$?"
  ```

  HEAD: no output, `exit=1`.

---

## 1. The table block: parse, fold, and lay out
<!-- kind: behavior -->

- [x] 1.1 RED: Write the failing unit tests in `src/ui/markdown.rs` for markdown-render's six
  table scenarios: `a_table_that_fits_renders_as_aligned_columns`,
  `a_wide_cell_wraps_within_its_column`, `a_wide_table_allocates_max_min_fairly`,
  `alignment_markers_pad_the_side_they_name`, `a_ragged_table_keeps_its_declared_columns`, and
  `a_narrow_region_renders_one_cell_per_line`. Each must name both `58` and `78` unsuffixed,
  or `MDWIDTHS` rejects it — the gate has no exemption list. Check, run at HEAD:

  ```sh
  cargo test --all-features --lib a_table_that_fits 2>&1 | tail -3
  ```

  HEAD: `0 passed; ... 1112 filtered out` — absent, as a RED check for new behaviour must be.

- [x] 1.2 CHANGE: Delete `a_table_renders_as_literal_source_rows` and drop the `"GFM table
  row"` entry from `unmodelled_constructs_render_as_source`'s `sources` array. Both pin the
  behaviour this group replaces and both go red the moment 1.3 lands, so they are removed
  before it rather than after. Check, run at HEAD:

  ```sh
  cargo test --all-features --lib a_table_renders_as_literal_source_rows 2>&1 | tail -3
  ```

  HEAD: `test ui::markdown::tests::a_table_renders_as_literal_source_rows ... ok` — green at
  HEAD, which is why it must be deleted rather than repaired.

- [x] 1.3 GREEN: Turn on `Options::ENABLE_TABLES` in `fold`'s `Parser::new_ext` call and add
  the `Tag::Table`/`TableHead`/`TableRow`/`TableCell` and matching `TagEnd` arms, accumulating
  alignments and cells into a `Table` value. Update **both** wildcard comments — the
  `Event::Start`/`End` one and the bare-`Event` one — to say which variants the new options
  can now produce (design.md → Decision 1). Verification: the fold-level assertions in 1.1's
  tests see the cells; the layout assertions still fail.

- [x] 1.4 GREEN: Replace `Block`'s `is_rule: bool` with `kind: BlockKind { Flow, Rule,
  Table(Table) }` and dispatch in `emit_block` (design.md → Decision 2). Verification:
  `cargo test --all-features --lib ui::markdown` — every markdown test **other than the two
  removed in 1.2** still passes, since `Flow` and `Rule` reproduce the previous two paths
  exactly.

- [x] 1.5 GREEN: Implement the column allocator: natural widths via `layout::columns`,
  `avail = width - (3n + 1)`, max-min fair capping, remainder by ascending index (design.md →
  Decision 4). Verification: `a_wide_table_allocates_max_min_fairly` passes at both widths.

- [x] 1.6 GREEN: Implement `emit_table`'s line grammar — a leading `|`, then per column
  ` ` + aligned cell + ` ` + `|` so a row line holds `n + 1` pipes, the delimiter line of `-`
  repeated `w[j] + 2`, header cells carrying `strong`, cell wrapping through the existing
  `wrap_prose` at `w[j]` columns, and a container's prefix on every line when the table is
  nested in a quote or list item (design.md → Decisions 3, 5, 6). Verification: 1.1's first
  five tests pass at both widths.

- [x] 1.7 GREEN: Implement the narrow fallback — one cell per line, wrapped to the full width,
  when `avail < n` (design.md → Decision 7), plus the zero-column guard that keeps
  `emit_table` total. Verification: `a_narrow_region_renders_one_cell_per_line` passes.

- [x] 1.8 REFACTOR: Fold the padding-and-pipe assembly through the existing `append` helper so
  adjacent plain segments merge, keeping segment counts in line with every other line kind.
  Verification: tests stay green; `Line::text()` assertions are unchanged.

- [x] 1.9 Run the group tests — no regressions beyond the two deletions 1.2 records:

  ```sh
  cargo test --all-features --lib ui::markdown
  ```

---

## 2. Table widths and totality
<!-- kind: behavior -->

- [x] 2.1 RED: Extend `composite_fixture` with a three-column table whose widest cell exceeds
  58 columns, so `no_line_exceeds_the_width_it_was_given` sweeps it at 0, 1, 2, 3, 10, 58, 78,
  and 200. Extend `lines_is_total_over_arbitrary_input`'s `pathological` array with the five
  new adversarial table sources the spec names — header-only, over-long row, short row,
  forty-column, and a 500-column CJK cell. The struck-run legs of both fixtures wait for
  group 3, which is what makes `ENABLE_STRIKETHROUGH` available.

- [x] 2.2 GREEN: Fix whatever the sweep and the totality test find. Verification: both tests
  pass at every listed width with no panic.

- [x] 2.3 Run the group tests — no regressions:

  ```sh
  cargo test --all-features --lib ui::markdown
  ```

---

## 3. The strikethrough face
<!-- kind: behavior -->

- [x] 3.1 RED: Write `a_struck_run_carries_the_face_and_composes` and
  `a_struck_run_split_across_a_wrap_keeps_its_face` in `src/ui/markdown.rs`, both naming 58 and
  78, and add the struck-run legs to `composite_fixture` and to
  `unmodelled_constructs_render_as_source`'s discriminating control. Check, run at HEAD:

  ```sh
  grep -n 'pub strikethrough' src/ui/markdown.rs ; echo "exit=$?"
  ```

  HEAD: no output, `exit=1` — the field does not exist, so the tests cannot compile.

- [x] 3.2 GREEN: Add `pub strikethrough: bool` to `Face`, turn on
  `Options::ENABLE_STRIKETHROUGH`, and add the `Tag::Strikethrough` / `TagEnd::Strikethrough`
  arms through the existing `push_faced` / `pop_faced` pair. Verification: 3.1's tests pass.

- [x] 3.3 GREEN: Add `strikethrough: false` to `src/ui/tasks.rs`'s `heading_line`.
  Verification: `cargo build --all-features` fails until this lands — the compiler is the
  forcing mechanism design.md → Contracts relies on.

- [x] 3.4 CHECK: Contract gate for `ui::markdown::Face` — confirm the added field is additive
  and that every literal either spreads a default or names the new field. `Role` is group 4's
  and is gated there. A line-wise grep cannot answer this: the `..` spread sits on a later
  line of a multi-line literal, so `grep -v '\.\.'` reports every literal as unspread. The
  check reads each literal's whole block:

  ```sh
  python3 - <<'PY'
  import re, pathlib
  hits = []
  for f in sorted(list(pathlib.Path("src").rglob("*.rs")) + list(pathlib.Path("tests").rglob("*.rs"))):
      lines = f.read_text().splitlines()
      for i, l in enumerate(lines):
          if not re.search(r"(?:^|[=:(,]\s*)(?:\w+::)*Face\s*\{\s*$", l):
              continue
          if re.search(r"\b(struct|impl|fn)\b", l):
              continue
          depth, body = 1, []
          for j in range(i + 1, len(lines)):
              depth += lines[j].count("{") - lines[j].count("}")
              if depth <= 0:
                  break
              body.append(lines[j])
          if not any(".." in b for b in body):
              hits.append(f"{f}:{i+1}")
  print("\n".join(hits) or "(none)"); print(f"count={len(hits)}")
  PY
  ```

  HEAD: `src/ui/tasks.rs:142`, `count=1` — the sole forcing site, which is what the spec
  claims. Negative control, executed at planning time: appending a second fully-spelled `Face`
  literal to `src/ui/view.rs` makes it report `count=2` and name the planted line; removing
  the plant returns it to `count=1`, with `git status --porcelain src/` clean.

- [x] 3.5 Run the group tests — no regressions:

  ```sh
  cargo test --all-features --lib ui::markdown && cargo test --all-features --lib ui::tasks
  ```

---

## 4. The palette role
<!-- kind: behavior -->

- [x] 4.1 RED: Extend `src/ui/palette.rs`'s tests for view-palette's three role scenarios —
  `every_role_is_answered_and_the_distinctions_are_real` gains `Strikethrough` in its
  exhaustive `match` and an assertion that its style equals no other role's;
  `each_roles_modifier_set_is_exactly_the_table` gains the `CROSSED_OUT` row and the "not
  `DIM`" discrimination; `the_coloured_set_is_exactly_the_table_and_every_colour_is_a_named_ansi_index`
  asserts `fg: None, bg: None`. Check, run at HEAD:

  ```sh
  grep -n 'Strikethrough' src/ui/palette.rs ; echo "exit=$?"
  ```

  HEAD: no output, `exit=1`.

- [x] 4.2 GREEN: Add `Role::Strikethrough` and its entry,
  `Style::default().add_modifier(Modifier::CROSSED_OUT)`, with no colour (design.md →
  Decision 9). Verification: 4.1's tests pass; the exhaustive `match` and the test module's
  own `label()` match both compile.

- [x] 4.3 CHECK: Contract gate for `ui::palette::Role` — confirm the variant is additive and
  that its one named consumer, `ui::view::style_for`, still compiles:

  ```sh
  cargo build --all-features && grep -n 'Role::Strikethrough' src/ui/view.rs
  ```

  The grep is expected to find nothing until group 5 lands; record that, because it is what
  says the consumer has not yet been updated rather than that the gate passed.

- [x] 4.4 Run the group tests — no regressions:

  ```sh
  cargo test --all-features --lib ui::palette
  ```

---

## 5. `style_for` and the rendered frame
<!-- kind: behavior -->

- [x] 5.1 RED: Extend `src/ui/view.rs`'s tests for the affected view scenarios, using their
  **real** names: append ` and ~~struck~~` to the sources of `faces_reach_the_buffer_as_styles`,
  `faces_reach_the_buffer_as_coloured_styles`, and `a_monochrome_reading_of_the_frame_is_unchanged`,
  asserting `CROSSED_OUT` with **no** foreground; add the struck-bold-link face to
  `heading_foreground_wins_over_a_code_span_inside_it`; add the `strikethrough == false`
  assertion to `a_plain_face_is_the_default_style`; extend
  `detail_content_never_overwrites_the_border` with a twelve-column table of 200-character
  cells and `a_degenerate_detail_interior_draws_nothing` with a table-source repetition. No
  test may name a `Color` literal — compare against `palette::style(role)`, since `PALETTE`
  searches `src/` including inline test modules.

- [x] 5.2 GREEN: Add the `Strikethrough` step to `style_for`'s fold, second, immediately after
  `Quoted` (design.md → Decision 9). Verification: 5.1's tests pass at both widths.

- [x] 5.3 REFACTOR: Clean up any duplicated table fixture between the view tests — or state
  that none was warranted.

- [x] 5.4 Run the group tests — no regressions:

  ```sh
  cargo test --all-features --lib ui::view
  ```

---

## 6. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [x] 6.1 VERIFY: Confirm `a_table_reaches_the_buffer_aligned` now passes end to end at both
  widths:

  ```sh
  cargo test --all-features --lib a_table_reaches_the_buffer
  ```

- [x] 6.2 REFACTOR: Clean up the acceptance test's fixture if group 1's own tests made part of
  it redundant — or state that no refactor was needed.

---

## 7. The degraded-states binding
<!-- kind: operational -->

Four sites must agree or `make check` goes red; design.md → Decision 10 gives the reasoning,
including why the scenario headers keep their names and why `covers` stays where it is.

- [x] 7.1 CHECK: Confirm the binding is green before touching it, so a later failure is
  attributable to this change:

  ```sh
  cargo test --all-features --test degraded_coverage 2>&1 | tail -3
  ```

  HEAD: `test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out`. The one
  ignored test is the coverage-map execution checker, which needs a coverage run — so this
  command does **not** prove a `covers` range is executed; `make coverage` is what does.

- [x] 7.2 CHANGE: Narrow `SPEC.md`'s degraded-states row from "(a table, a footnote,
  strikethrough, a task-list item)" to "(a footnote, a task-list item)", narrow the
  rendering-grammar prose in `SPEC.md` → User interface → Detail view the same way, and add one
  sentence there for the table grammar and one for the struck face.

- [x] 7.3 CHANGE: Edit `tests/degraded-coverage.toml`'s matching entry: `condition` to match
  `SPEC.md`'s new row byte-for-byte, and `why` reworded to name what the narrowed proof
  observes. Leave `verdict` at `unproven` and leave `covers` on the literal-text line
  (`Event::Text`), re-measured for the line shift — that line is what still makes footnotes and
  task-list items render literally, whereas the `Options` line is what makes the departed
  constructs leave the row. Verification:

  ```sh
  cargo test --all-features --test degraded_coverage
  ```

  It fails on a condition mismatch, a proof that is not a passing `#[test]`, or a `covers`
  range that does not resolve.

- [x] 7.4 CHANGE: Narrow the live `openspec/specs/markdown-render/spec.md` `## Purpose`, whose
  sentence "Constructs the parser is deliberately not configured for — tables, footnotes,
  strikethrough — render as their literal source" this change makes false. A delta carries only
  `## ADDED`/`## MODIFIED` blocks and never a `## Purpose`, and `tests/spec_purposes.rs` checks
  only non-emptiness, so nothing else would catch it.

- [x] 7.5 VERIFY: Confirm the binding is green and the narrowing is not vacuous:

  ```sh
  git diff --stat SPEC.md tests/degraded-coverage.toml openspec/specs/markdown-render/spec.md
  cargo test --all-features --test degraded_coverage
  ```

---

## 8. Gate floors
<!-- kind: operational -->

Per `AGENTS.md`, a gate's floor is its own script default; `NODEFAULT-UI` is the one recorded
multi-subject exception whose floors live on the `Makefile` line.

- [x] 8.1 CHECK: Record the floors before the change so a move is measured, not guessed.
  Checks, run at HEAD:

  ```sh
  /bin/sh scripts/gates/mdwidths.sh 2>&1 | tail -1
  /bin/sh scripts/gates/widths.sh 2>&1 | tail -1
  SCAN_MIN=135 /bin/sh scripts/gates/nodefault-ui.sh 2>&1 | tail -2
  ```

  HEAD: `MDWIDTHS OK: all 26 markdown tests name both 58 and 78` (script default `MD_MIN=26`,
  exactly met, so any added test moves it); `WIDTHS` default `WIDTHS_MIN=113` over
  `src/ui/view.rs`, which groups 0, 5, and 6 all add tests to; `NODEFAULT-UI OK (half B): 150
  literal/pattern spans scanned (>= 135)`.

- [x] 8.2 CHANGE: Re-run all three on the finished tree. Raise `scripts/gates/mdwidths.sh`'s
  `MD_MIN` and `scripts/gates/widths.sh`'s `WIDTHS_MIN` defaults to the new measured counts.
  Raise the `Makefile`'s view-set `SCAN_MIN` only if the measured span count moved.

- [x] 8.3 VERIFY: Confirm the gates still catch a planted defect rather than merely passing:

  ```sh
  cargo test --all-features --test gate_controls
  ```

  HEAD: `test result: ok. 4 passed; 0 failed` in 44.87s. Green at HEAD proves nothing alone,
  which is why the recorded plants in `tests/gate-controls.toml` are the other half.

---

## 9. Change Review
<!-- kind: operational -->

- [x] 9.1 CHECK: Dispatch an independent reviewer — not a fork of the implementing session —
  with only `proposal.md`, the four spec deltas, `design.md`, `tasks.md`, and the diff.
  Concentration points for this change, beyond the standing ones: (a) every table scenario has
  a test that would go red if `emit_table` were deleted, not one that passes on an empty
  result — the alignment scenario in particular, whose first draft used one-column-wide cells
  and could not discriminate alignment at all; (b) the four degraded-row sites agree —
  `SPEC.md`, `tests/degraded-coverage.toml`, the `degraded-coverage` spec's scenario, and the
  proof test — plus the live `markdown-render` Purpose; (c) no test outside `src/ui/palette.rs`
  names a `Color` literal; (d) the narrow fallback fires at a width where it actually fires;
  (e) every test name cited in an artifact resolves to a real `fn` in the tree.

- [x] 9.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason,
  note SUGGESTIONs, and re-run the affected tests.

  **Outcome: 0 CRITICAL, 3 WARNINGs (all fixed), 3 SUGGESTIONs (two applied, one recorded).**
  Every WARNING was a test that could not fail, each demonstrated by the reviewer with its
  own mutation — this project's own recorded defect class, and the reason the review is
  dispatched to a session that did not write the code.

  | Severity | Finding | Resolution |
  |---|---|---|
  | WARNING | `emit_table`'s container-prefix branch was specified and implemented but exercised by no fixture: replacing the prefix push with a discard left all 1120 tests green | Added `a_table_inside_a_container_carries_the_prefix_on_every_line`, covering a quote and a list item at both widths, with the leg that fails if the prefix is carried but the width is not reduced for it. Re-ran the reviewer's mutation: it now fails. |
  | WARNING | the narrow fallback's "header cells still carrying `strong`" had no face assertion: emitting every segment plain left all 1120 tests green | Added the face column to `a_narrow_region_renders_one_cell_per_line`'s width-8 leg. Re-ran the mutation: it now fails. |
  | WARNING | `detail-scroll`'s border scenario claimed its twelve-column table exercised the one-cell-per-line fallback; it does not — `3n + 1` is 37, so `avail` is 41 and 21, both at least `n` | Added a fifteen-column leg, whose `4n + 1` of 61 the 78-column interior clears and the 58-column one does not, asserting pipes at 120 and none at 60; the spec bullet is corrected with a `Corrected during Change Review` note, on the convention `degraded-coverage` already uses. |
  | SUGGESTION | the ragged-table guard drove the parser with `ENABLE_TABLES` alone, not the set `fold` passes | Applied: it now passes `ENABLE_TABLES \| ENABLE_STRIKETHROUGH`. |
  | SUGGESTION | `narrow_rows` skipped a whitespace-only cell while the spec says "an empty cell emitting nothing" | Applied: the guard is now `r.text.is_empty()`. Output is identical either way — `wrap_prose` emits no line for a whitespace-only cell — so this closes a wording gap, not a behaviour one. |
  | SUGGESTION | group 10's work landed before the review's diff was cut | Recorded: group 10 ran ahead of group 9 because its `AGENTS.md` sentence is independent of every finding, and the reviewer confirmed the landed sentence matches 10.2. |

  `MDWIDTHS`' floor is re-measured once more for the added test: `MD_MIN` 33 -> 34.

- [x] 9.3 VERIFY: Confirm no blocking or unowned finding remains.

  No CRITICAL was raised, every WARNING is fixed rather than accepted, and both applicable
  SUGGESTIONs are applied. Nothing is deferred to a later change.

---

## 10. Documentation
<!-- kind: operational -->

`SPEC.md`'s own edits are group 7's, because they are load-bearing for the degraded-states
binding rather than merely descriptive; this group carries only the agent-facing rule.

- [x] 10.1 CHECK: Confirm `AGENTS.md` carries no existing sentence about the parser's option
  set, so this is an add rather than a rewrite. Command, run at HEAD:

  ```sh
  grep -n -i 'subset\|unmodelled\|does not model\|literal source' AGENTS.md ; echo "exit=$?"
  ```

  HEAD: no output, `exit=1` — the subset prose lives only in `SPEC.md`, which group 7 owns.

- [x] 10.2 CHANGE: Add one sentence to `AGENTS.md`'s existing `pulldown_cmark`/`Color`
  confinement rule under Architecture rules (audience: every agent session, which loads this
  file whole): the option set is exactly `ENABLE_TABLES | ENABLE_STRIKETHROUGH`, and turning on
  a further flag without a rendering path makes that construct **vanish** into `fold`'s `_ =>
  {}` wildcards rather than degrade to literal text. Net +2 lines to one rule. Durable because
  no gate can see a wrongly-added flag. Edit `AGENTS.md` itself — `CLAUDE.md` is a symlink to
  it, so editing both would be editing one file twice.

- [x] 10.3 VERIFY: `cargo test --all-features --test doc_contract` — the module map, gate-path
  and manifest claims are unaffected, and this confirms the edit broke none of them.

---

## 11. Lint & Verify
<!-- kind: operational -->

- [x] 11.1 CHECK: Inspect the intended verification commands and affected tiers. The affected
  tiers are unit (`ui::markdown`, `ui::palette`), view (`ui::view`), gates (`MDWIDTHS`,
  `WIDTHS`, `MDSEAM`, `PALETTE`, `COLWIDTH`, `NOIO-VIEW`, `NODEFAULT-UI`), and contract
  (`tests/degraded_coverage.rs`, `tests/gate_controls.rs`, `tests/doc_contract.rs`). All are
  inside `make check`.

- [x] 11.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.

- [x] 11.3 VERIFY: `cargo fmt --all -- --check` — clean.

- [x] 11.4 VERIFY: `cargo build --all-features` — 0 errors. (Rust's compiler is the type
  checker; there is no separate one.)

- [x] 11.5 VERIFY: `cargo test --all-features` — green.

- [x] 11.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` plus the production-slice floor from
  `scripts/coverage-prod.py`. Never lower, waive, or exclude — add tests if it falls short.

- [x] 11.7 VERIFY: `make check` — the single gate, green, with the change artifacts committed
  so `OPENSPEC-UNTOUCHED` passes. Name the failing sub-command if it fails.

- [x] 11.8 VERIFY: `openspec validate markdown-constructs --strict`.

  **Result, on the finished tree:** `make check` exit 0 — every gate in the recipe green,
  `OPENSPEC-UNTOUCHED` included, with the change artifacts committed. Coverage: total
  96.09% against the 80% floor, production slice **96.19%** (4120/4283) against its floor
  of 96. `cargo test --all-features` 1121 lib tests plus every contract-tier suite green;
  `gate_controls` 4 passed, so every gate is still executed against its recorded plant.
  `openspec validate markdown-constructs --strict`: valid.

  One flake observed and not attributable to this change:
  `ui::tests::wiring::g_focuses_the_agent_the_launch_started` failed once in a full run —
  it read the scratch `herdr` log before the launcher's `agent focus` had landed, seeing
  three calls where it asserts four. It passed on four consecutive re-runs and in both
  `make check` runs, and the Change Review session hit it once on its own independent run —
  so it is not local to one session's machine state. It is a pre-existing threaded wiring
  test this change touches no part of; recorded here rather than repaired, because
  repairing it is a different change's scope.
