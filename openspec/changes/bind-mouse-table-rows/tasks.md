<!-- Groups are sequential and none carries a `parallel-after` marker. Groups 1, 2, 3 and 5
     all edit `tests/doc_contract.rs`. Group 4 edits `SPEC.md` — a different file, but its
     gate is not attributable either: tasks 1.4, 2.5, 3.6 and 5.9 each run
     `cargo test --test doc_contract`, which reads the very `SPEC.md` a concurrent group-4
     implementer would be mid-edit of, so one group would fail over another's half-written
     table. That is the schema's third criterion, not a cost judgment. -->

## Planning-time evidence

Every number and check below was run against HEAD `6c875b6`, and every one was independently
re-run by planning review. Reproduce with the commands shown.

**The hole is real, and this is the RED.** Plant a row describing a binding the pane no longer
has, then run the existing check:

```sh
cp SPEC.md /tmp/SPEC.md.bak
python3 - <<'PY'
import io
s = io.open("SPEC.md", encoding="utf-8").read()
anchor = "| Left click outside the help overlay's band"
plant = "| Left click on any other row of a foldable artifact's content | `Action::Click` naming that row's own content-line index (PLANTED) |\n"
assert s.count(anchor) == 1
io.open("SPEC.md", "w", encoding="utf-8").write(s.replace(anchor, plant + anchor, 1))
PY
cargo test --test doc_contract mouse_bindings_match_spec_md >/dev/null 2>&1; echo "PLANTED_EXIT=$?"
cp /tmp/SPEC.md.bak SPEC.md
cargo test --test doc_contract mouse_bindings_match_spec_md >/dev/null 2>&1; echo "CLEAN_EXIT=$?"
```

`PLANTED_EXIT=0`, `CLEAN_EXIT=0` — confirmed twice. The planted stale row **passes**.

`101` is the exit status of a Rust test binary whose assertion panics, confirmed empirically by
planting `` `Action::Bogus` `` (which the *existing* name-set leg does catch) and measuring
`BOGUS_EXIT=101`.

**The measured claim set.** Review built the first draft's `(kind, zone, action-name)` triple
set by running the real sweep: `TRIPLE_COUNT=104`, `KINDS_SEEN=14`, `ACTIVE_TRIPLES=18`
(86 `Ignore`). The 18 active triples include **both** `Down(Left) | DetailRow | Click` and
`Down(Left) | DetailRow | Select`, which is why the bare-name key cannot make the stale row
vacuous, and why Decision 2 keeps the payload constructor. Dump retained by review; runtime was
88.5 s in a debug build over six passes.

**Counts.**

```sh
awk '/^\| Gesture \| Action \|/{f=1} f&&/^\|/{n++} f&&!/^\|/{exit} END{print "pipe_rows="n" binding_rows="n-2}' SPEC.md
# pipe_rows=15 binding_rows=13

awk '/^\| Gesture \| Action \|/{f=1} f&&/^\|/{if($0~/`Zone::/) n++} f&&!/^\|/{exit} END{print n+0}' SPEC.md
# 0 — no row names a Zone today

awk '/^pub enum Zone \{/{f=1;next} f&&/^\}/{exit} f&&/^    [A-Z]/{n++} END{print n}' src/ui/layout.rs
# 6 — ListRow, List, DetailTab, DetailRow, Detail, Outside
# (an earlier draft used a bare grep here, which returns 24: it matched call sites too)

grep -c "fn mouse_table_claims\|fn compare_mouse_claims" tests/doc_contract.rs; echo "exit=$?"
# 0, exit=1 — neither function exists yet
```

## 1. The sweep retains its claims
<!-- kind: behavior -->

- [x] 1.1 RED: Write failing tests for `the_sweep_records_all_four_claim_axes` and
      `the_overlay_state_is_its_own_axis`. Assert concretely, so a zone/action mix-up fails:
      `(ScrollDown, ListRow, SelectNext)` present, `(Down(Left), DetailTab, SelectTab)`
      present, and `(ScrollDown, ListRow, ScrollDown)` **absent** — a test that only asserts
      the return type is satisfied by the compiler and cannot go red.
- [x] 1.2 GREEN: Add `sweep_mouse_claims(fixture, help_open) -> BTreeSet<Claim>`, with the
      outcome carrying the `Action` variant **plus its payload's constructor name** and the
      zone carrying its variant name only (per design.md → Decision 2). The zone is recovered
      by calling `ui::layout::zone` — `mouse_action` never returns it — and must mirror its
      precedence: `Zone::Outside` outside the frame, and no zone consulted at all while the
      overlay is open (per design.md → Decision 11). Cache per `(fixture, overlay)` on
      `swept_mouse_action_names`' existing `OnceLock` terms.
- [x] 1.3 REFACTOR: Re-express `swept_mouse_action_names` as a projection of the claim set so
      the two cannot disagree, keeping its signature and `BTreeSet<String>` return. Verify:
      `swept_action_names`, `select_is_the_only_mouse_only_action_by_name_and_count`, and
      `the_sweep_covers_the_mouse_under_both_overlay_states` all pass unedited — the last
      asserts both name sets by equality and is the one that will catch a wrong projection.
- [x] 1.4 Run `cargo test --test doc_contract` — green, no regressions.

## 2. The dashboard-fixture axis
<!-- kind: behavior -->

- [x] 2.1 RED: Write a failing test `the_clamp_is_observed_under_a_selection_fixture`,
      asserting `(Drag(Left), overlay closed, Zone::List, Select(Extend))` is among the swept
      claims. It fails at HEAD because `sweep_dashboard` pins `selection: None`
      (`tests/doc_contract.rs:2314`), so that arm is unreachable.
- [x] 2.2 GREEN: Give the sweep an explicitly listed fixture set — at minimum selection-absent
      and selection-present — and assert its length in the check's own source, on the terms
      `EXEMPT_ACTIONS` is pinned at two (per design.md → Decision 9).
- [x] 2.3 CHECK: Enumerate the claims the new fixture adds and record them in the commit
      message. They are claims the table must then cover, so group 4 writes rows against this
      list rather than against the pre-fixture set.
- [x] 2.4 VERIFY: Measure the resulting `cargo test --test doc_contract` wall time and record
      it in the commit message. If it exceeds roughly three minutes, apply design.md → Risks'
      stated lever (the second fixture on the wide frame only) and record the measurement.
- [x] 2.5 Run `cargo test --test doc_contract` — green, no regressions.

## 3. Row extraction: zones, payload tokens, and a closed gesture vocabulary
<!-- kind: behavior -->

- [x] 3.1 RED: Write failing tests for `an_unrecognised_gesture_phrase_is_an_error`,
      `a_mistyped_zone_token_is_named`, `a_zone_less_row_is_rejected_unless_it_is_the_catch_all`,
      `a_second_catch_all_is_an_error`, and `a_gutted_mouse_table_fails_as_a_broken_control`,
      each driving the extractor with a synthetic table string.
- [x] 3.2 GREEN: Add `documented_mouse_rows(spec_md) -> Result<Vec<Row>, String>` returning each
      row's gesture kinds, zone set, overlay state, outcomes, and its own source text. Add it
      **beside** `documented_mouse_actions` without altering that function or its callers — the
      real `SPEC.md` has no zone tokens until group 4, so a strict parse wired in here would
      break this group's own gate and `documented_mouse_actions_fails_on_a_missing_table`.
- [x] 3.3 GREEN: Accept one **or more** backticked `Zone` tokens per row, erroring by row and
      token on any that names no real variant (per design.md → Decision 4).
- [x] 3.4 GREEN: Add the closed gesture vocabulary as a `[(&str, &[MouseEventKind])]` table in
      the check's own source, covering the real table's five phrases — `Wheel down`, `Wheel up`,
      `Left click`, `Left drag`, `Anything else` — pinned to length 5, erroring by row and
      phrase on anything unlisted.
- [x] 3.5 GREEN: Accept a zone-less row only as the single `Ignore`-only catch-all, erroring on
      any other zone-less row and on a second catch-all, naming both rows in that case.
- [x] 3.6 Run `cargo test --test doc_contract` — green, no regressions.

## 4. `SPEC.md`'s mouse table names its zones, targets, and the overlay wheel
<!-- kind: operational -->

- [x] 4.1 CHECK: Record the starting state with the three `awk` commands above — 13 binding
      rows, 0 naming a `Zone`, 6 `Zone` variants.
- [x] 4.2 CHANGE: Add to each non-catch-all row every `Zone` it covers and its outcome's payload
      constructor: the two list-wheel rows name `Zone::List` and `Zone::ListRow`; the two
      detail-wheel rows name `Zone::Detail`, `Zone::DetailTab` and `Zone::DetailRow`; the click
      and drag rows name their single zone and their `Target::`/`SelectPhase::` constructor; the
      help-overlay row names the overlay-open state rather than a `Zone`; the catch-all stays
      zone-less.
      The zone sets are measured, not inferred — planning review computed them from the sweep:
      `ScrollDown`/`SelectNext` and `ScrollUp`/`SelectPrev` → `List`, `ListRow`;
      `ScrollDown`/`ScrollDown` and `ScrollUp`/`ScrollUp` → `Detail`, `DetailTab`, `DetailRow`;
      `Click(Change)` and `Click(Section)` → `ListRow`; `SelectTab` → `DetailTab`;
      `Click(DetailHeader)`, `Select(Begin)` and `Select(Extend)` → `DetailRow`.
      Thirteen distinct (gesture, outcome) pairs over nineteen active claims.
- [x] 4.3 CHANGE: Add a row for the wheel over an open help overlay — `Action::ScrollDown` and
      `Action::ScrollUp` from anywhere in the frame (`src/ui/driver.rs:340-341`), today
      documented only in the paragraph after the table. This is the new check finding a binding
      with no row.
- [x] 4.4 VERIFY: Re-run the second `awk` — it reports **11** rows bearing a `` `Zone:: ``
      token: 14 binding rows now, less the catch-all and less the overlay-state row. Run
      `cargo test --test doc_contract mouse_bindings_match_spec_md` — still green, since
      `backticked_action_variants` matches the literal `Action::` only.

## 5. The two-way comparator
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing tests for `a_row_describing_a_removed_binding_fails_as_vacuous`,
      `two_rows_differing_only_in_payload_are_told_apart`,
      `a_binding_with_no_row_fails_as_undocumented`,
      `the_overlay_axis_keeps_the_two_passes_apart`, and
      `a_third_row_joining_a_known_collision_fails`, each calling the comparator with one real
      side and one hand-built side (per design.md → Test Boundaries).
- [ ] 5.2 GREEN: Add `compare_mouse_claims(rows, claims) -> Result<(), String>` as a pure
      function over both arguments, reporting uncovered claims and vacuous rows, and reporting
      **both** when both occur rather than returning at the first.
- [ ] 5.3 GREEN: Make a vacuous-row report name the claims actually observed at that row's own
      zones, so the reader is told what the row should have said.
- [ ] 5.4 RED: Write `an_empty_catch_all_fails` — the comparator called with a hand-built claim
      set holding no `Ignore` claim. It must be driven synthetically: at HEAD roughly 86 of the
      104 claims are `Ignore`, so this rule can never fire against the real tree.
- [ ] 5.5 GREEN: Give the catch-all the remainder of the `Ignore` claims, require it to claim at
      least one, and add the known-collision list pinned by count (per design.md → Decision 10).
- [ ] 5.6 GREEN: Wire the comparator into `mouse_bindings_match_spec_md` as legs 2 and 3,
      keeping the name-set equality as leg 1. Verify `CLAIM_COUNT` is still 13 and
      `documented_claim_count_matches_the_file` passes untouched.
- [ ] 5.7 REFACTOR: Now that `SPEC.md` carries the tokens, fold `documented_mouse_actions` into
      a projection of `documented_mouse_rows` so one parse serves both — or state why the
      lenient parse must survive for `documented_mouse_actions_fails_on_a_missing_table`.
- [ ] 5.8 VERIFY: Re-run the planting script from **Planning-time evidence**, with the planted
      row carrying `` `Zone::DetailRow` `` and `` `Target::DetailLine` `` and **replacing** the
      real content-row row rather than sitting beside it. It must report `PLANTED_EXIT=101`
      with a message naming **vacuity**, not a missing zone; and `CLEAN_EXIT=0`.
- [ ] 5.9 Run `cargo test --test doc_contract` — green, no regressions.

## 6. Change Review
<!-- kind: operational -->

- [ ] 6.1 CHECK: Dispatch an independent reviewer against proposal, specs, design, and tasks.
- [ ] 6.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING, re-run affected tests.
- [ ] 6.3 VERIFY: Confirm no blocking or unowned finding remains.

## 7. Documentation
<!-- kind: operational -->

- [ ] 7.1 Rewrite in `SPEC.md`: § Testing and quality gates → Doc-conformance checks
      (audience: contributors) — the mouse entry becomes an executed row-by-row binding,
      replacing the claim that the mouse and key legs differ in *kind*, which is what made the
      weaker one look deliberate. It must stay exactly **one** bullet:
      `spec_md_doc_conformance_claim_count` counts the `- ` bullets in that section and
      `documented_claim_count_matches_the_file` compares the total against `CLAIM_COUNT` = 13.
- [ ] 7.2 Rewrite in `AGENTS.md`: the contract-tier paragraph's "the documented mouse bindings"
      item (audience: agents working in this repo) — say it is bound by executing `mouse_action`
      at every cell, so a future agent does not repeat `mouse-text-selection`'s assumption that
      editing the table alone is safe. `CLAUDE.md` is a **symlink** to `AGENTS.md`, not a copy,
      so this is the whole edit. The claim count stays thirteen, so no numeral moves.

## 8. Lint & Verify
<!-- kind: operational -->

- [ ] 8.1 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 8.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings.
- [ ] 8.3 VERIFY: `make gates` — every hygiene gate.
- [ ] 8.4 VERIFY: `cargo test --all-features` — green, contract tier included.
- [ ] 8.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` and the production-slice floor. No file
      under `src/` is touched and the coverage export names 28 files all under `src/`, so both
      figures must be unchanged: **95.32% lines** (95.49% regions — `--fail-under-lines` governs
      the lines column) and 96.33% production.
- [ ] 8.6 VERIFY: `openspec validate bind-mouse-table-rows --strict`.
