<!-- No "Acceptance Test — Outer Loop" group. design.md → Test Strategy says why: the only
     true end-to-end proof drives a real terminal and a real clipboard, which this repository
     forbids at every tier because `cargo test` spawns this binary. -->

<!-- NO `parallel-after` MARKER APPEARS BELOW, and that is a decision, not an omission.
     schema.yaml:327-331 requires that a failure stay attributable, and this project's gate is
     the whole-tree `make check` run in one shared working tree. Five pinned counts move in
     this change; a group that has moved its half while a neighbour has not reds the other's
     run, and neither failure is its own. The previous plan for this change marked two groups
     parallel and deadlocked at the first fan-out. Every group below is sequential and leaves
     the tree green. -->

<!-- Counts cited below, each with the command that produced it, run at HEAD 2026-09-15:
     Action 24, Target 4, Dashboard fields 15, Role 30 —
       `awk '/^pub enum X/,/^}/' <file> | grep -cE '^\s+[A-Z]'`
     INVENTORY 31 — `grep -c 'input:' src/ui/help.rs` minus the struct's own field
     help.rs: 11 lines say 42, 4 say 44 — `grep -c '\b42\b'`, `grep -c '\b44\b'`
     DetailLine: 13 sites — `grep -rc 'DetailLine' src/` → driver.rs 5, app.rs 8
     TerminalOps 6 methods — `awk '/pub trait TerminalOps/,/^}/' … | grep -c 'fn '` -->

## 1. `Role::Selected`
<!-- kind: behavior -->

- [x] 1.1 RED: Write the failing test for "`Selected` reverses and colours nothing".
      Check: `grep -qE '^    Selected,' src/ui/palette.rs` → **exit 1 at HEAD** (run
      2026-09-15).
- [x] 1.2 GREEN: Add the variant, its `table()` row, and an arm in each exhaustive `match`
      over `Role`. It carries `Modifier::REVERSED` and no colour, per design.md → Decision 7.
- [x] 1.3 CHECK: Move the role count. `awk '/^pub enum Role/,/^}/' src/ui/palette.rs |
      grep -cE '^\s+[A-Z]'` printed **30** at HEAD and must print **31**; update the
      "thirty-six rows" comment in `src/ui/palette.rs` if the table's row count moved with it.
- [x] 1.4 VERIFY: `cargo test ui::palette` and `make gates` green — `PALETTE` in particular,
      which sweeps all of `src/` including inline test modules.

## 2. `TerminalOps::write_clipboard`
<!-- kind: behavior -->

- [x] 2.1 RED: Write the failing tests for "The clipboard write is confined and reports its
      own failure" and "A clipboard write changes no terminal mode", extending `Recorder`.
      Check: `grep -q 'write_clipboard' src/ui/terminal.rs` → **exit 1 at HEAD**.
- [x] 2.2 GREEN: Add the seventh trait method, its `CrosstermOps` implementation writing the
      OSC 52 sequence, and the `Recorder` arm. `TerminalGuard`'s entry and teardown order is
      untouched.
- [x] 2.3 CHECK: Confirm the escape is confined. `grep -rl ']52;' src/ tests/` → must print
      exactly `src/ui/terminal.rs`. Negative control: paste the sequence into
      `src/ui/view.rs`, re-run, confirm two files print, remove it.
- [x] 2.4 VERIFY: `cargo test ui::terminal` and `make gates` green — `NORAW-GREP`'s per-name
      control still passes, since no crossterm name was added.

## 3. Word bounds and span-to-text
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests for "A click selects the whole token, not a fragment", "A
      single-line selection copies exactly the selected columns", and "A multi-line selection
      joins with newlines and drops padding".
- [ ] 3.2 GREEN: Add pure helpers in `src/ui/detail.rs` over `Vec<ContentRow>`: the word run
      at a cell, and the text of a span. A word is a maximal run of non-whitespace **display
      columns**, measured by `ui::layout::columns`, never `.chars()`.
- [ ] 3.3 VERIFY: `cargo test ui::detail` green, and `make gates` — `COLWIDTH` sweeps this
      file and will fail a `.chars().count()`.

## 4. `Selection` and the `Dashboard` field
<!-- kind: behavior -->

- [ ] 4.1 RED: Write the failing test for "`selection` starts empty and is cleared rather than
      reloaded".
      Check: `grep -q 'pub struct Selection' src/ui/app.rs` → **exit 1 at HEAD**.
- [ ] 4.2 GREEN: Add `Selection { anchor, focus, granularity }` with no `Default`, and the
      `selection: Option<Selection>` field. Every `Dashboard` construction site names it.
- [ ] 4.3 CHECK: Move the field count in the same group as the field.
      `awk '/^pub struct Dashboard/,/^}/' src/ui/app.rs | grep -cE '^\s+pub [a-z_]+:'` printed
      **15** and must print **16**; update the destructure companion and every assertion that
      spells fifteen.
- [ ] 4.4 CHECK: Add `Selection` to the `TYPES` list on `Makefile:46`
      (`TYPES='Dashboard Filter Detail Sections Help'`) — the gate sweeps only the types the
      recipe names, so without this it passes identically before and after this group and
      proves nothing. Re-measure that line's `SCAN_MIN=308` per `notes/gate-floors.md`'s
      convention and update it.
      Check: `grep -c 'Selection' Makefile` → **0** at HEAD, must print **1**.
- [ ] 4.5 VERIFY: `cargo test ui::app` and `make gates` green, and confirm the gate is now
      falsifiable for this type: add `..Default::default()` to a `Selection` literal, run
      `make gates`, see `NODEFAULT-UI` fail, remove it, see it pass.

## 5. `Action::Select`, the `DetailLine` removal, and every count they move
<!-- kind: behavior -->
<!-- Deliberately one group. Adding the variant breaks two exhaustive matches that do not
     compile until their arms exist, and `compare_key_atoms` tolerates no residual between
     INVENTORY and the two documents. Splitting it leaves the tree red between groups, which
     is what deadlocked the previous plan. -->

- [ ] 5.1 RED: Write failing tests for the resolver and the press counting — "A drag begins
      only in the detail content area", "A section header stays clickable and is never
      selectable", "One press arms, two select a word, three select the row", "A press
      elsewhere restarts the count", "A dragged span arms rather than widening", "A
      non-foldable tab's content is selectable too".
      Check: `grep -q 'Action::Select\b' src/ui/app.rs` → **exit 1 at HEAD**.
- [ ] 5.2 GREEN: Add `Action::Select` carrying its phase, resolve left press and left drag in
      `mouse_action` by zone, and add the `apply` arm. Selection must not inherit
      `detail_row_click`'s `!foldable()` short-circuit (design.md → Decision 8).
- [ ] 5.3 GREEN: Remove `Target::DetailLine` and every arm reaching it.
      `grep -rc 'DetailLine' src/` printed `driver.rs:5, app.rs:8` at HEAD — **13 sites** — and
      must print none.
- [ ] 5.4 CHECK: Add the `Mouse` group row and move the counts it forces, all here.
      `grep -c 'input:' src/ui/help.rs` printed **32** (31 bindings plus the struct field) and
      must print **33**; `grep -c '\b42\b' src/ui/help.rs` printed **11** and must print 0;
      `grep -c '\b44\b' src/ui/help.rs` printed **4** and must print 0. The two clamp
      **values** 5 and 25 become 6 and 26, and `assert_eq!(band.height, 44)` becomes 45.
- [ ] 5.5 CHECK: Add the arms and move the counts in `tests/doc_contract.rs` — the
      `action_name` arm, `union.len()` 24 → 25, `bound.len()` 22 → 23, the sweep test renamed
      to drop its numeral, and `the_sweep_covers_the_mouse_under_both_overlay_states`'
      `expected_closed` set (`tests/doc_contract.rs:2828`) from seven names to eight.
- [ ] 5.5b CHECK: Move the **four hand-enumerations of `Action` in `src/ui/app.rs`'s own test
      module**, none of which any gate finds for you: `assert_known_variant`'s exhaustive
      match (`:5034`, a compile error until its arm exists), the `variants` array and its
      `assert_eq!(…, 24)` (`:5063`, `:5104`), `seventeen_inert_actions() -> [Action; 17]`
      (`:5383`) with its three call sites, its test name, and the prose comment at `:861`.
      Check: the assertion is wrapped across lines, so match the text not the call —
      `grep -cE '\[Action; 17\]|the twenty-four variants' src/ui/app.rs` → **2** at HEAD
      (`:5383` and `:5104`), must print **0**.
- [ ] 5.6 CHECK: Add the drag row to **`SPEC.md`'s `| Gesture | Action |` table only**
      (`SPEC.md:660`). The binding check that fires is `mouse_bindings_match_spec_md`
      (`tests/doc_contract.rs:2029`), a set-equality between that table's backticked
      `Action::` names and `mouse_action`'s body, so `Action::Select` in the resolver requires
      the row. Do **not** touch `README.md` → Keys: `grep -c '| Gesture' README.md` → **0**,
      it has no gesture table, and `compare_key_atoms` skips the `Mouse` group entirely
      (`tests/doc_contract.rs:2586`, `:2976`), so a row added there names a key atom the
      inventory does not and fails the opposite direction.
- [ ] 5.7 VERIFY: `make check` green — the whole gate, not one target. This group is the one
      most able to leave the tree inconsistent.

## 6. Painting the highlight
<!-- kind: behavior -->

- [ ] 6.1 RED: Write failing render tests for "The span is highlighted at both mandated
      widths", "A selected cell keeps its own role and gains the reversal", and "The highlight
      persists after release and clears on the next interaction".
- [ ] 6.2 GREEN: In `src/ui/view.rs`, patch each selected cell's existing style with
      `palette::style(Role::Selected)` rather than replacing it (design.md → Decision 7).
- [ ] 6.3 VERIFY: `cargo test ui::view` green at 120x20 and 60x20, and `make gates` —
      `DETAILWIDTHS` requires both figures in every test in the swept files.

## 7. Copy on completion
<!-- kind: behavior -->

- [ ] 7.1 RED: Write failing tests for "The clipboard write cannot be confirmed, and the pane
      claims nothing" and the `Err` half of it, driving a `Recorder` that fails the write.
- [ ] 7.2 GREEN: Thread `ClipboardWriter<'a> = &'a dyn Fn(&str) -> Result<(), String>` into
      `run_loop` beside `ArtifactReader` and bind it in `src/ui/mod.rs` to the guard's
      `write_clipboard` — `run_loop` has no `TerminalOps` handle today, so there is otherwise
      no call site (design.md → Decision 11).
- [ ] 7.2b GREEN: Call it on the completing phase only — a drag's finish, the second press,
      the third press — never on a first press. A failure is stored on `Selection::problem`
      and rendered as a detail-region row; a success renders nothing (Decision 12).
- [ ] 7.3 VERIFY: `cargo test ui::` green, and `make gates` — `NOBLOCK`, which must still find
      no clock under `src/ui/`, since the press counting is state-based by design.

## 8. The degraded-states row
<!-- kind: operational -->

- [ ] 8.1 CHECK: Read `SPEC.md` → Degraded states and pick the row's position among the
      existing ones.
- [ ] 8.2 CHANGE: Add the row for a failed clipboard write, and bind it in
      `tests/degraded-coverage.toml` to the test group 7 wrote.
- [ ] 8.3 VERIFY: `cargo test --test degraded_coverage` green.

## 9. Documentation and the twelfth claim
<!-- kind: operational -->

- [ ] 9.1 CHANGE: Rewrite `SPEC.md`'s drag-to-select paragraph (audience: maintainers and
      readers). It claims the bypass "requires holding `Option` (macOS) or `Shift` (most Linux
      terminals)"; `Option` was measured not to work in Ghostty under any mode set. Name
      `Shift`, name what it was measured on, say the other terminals are unmeasured, and say
      the cost no longer applies inside the detail content area.
      Check: `grep -c 'Option\` (macOS)' SPEC.md` → **1** at HEAD, must be **0**.
- [ ] 9.2 CHANGE: Add the OSC 52 confinement leg to `tests/doc_contract.rs`, with the vacuity
      guard, and move `AGENTS.md`'s "eleven further claims" and `SPEC.md` → § Doc-conformance
      checks to twelve.
- [ ] 9.3 CHECK: Negative control for 9.1, recorded per this repository's rule for a green
      guard. Restore the `Option` sentence, confirm the leg fires, restore the correction,
      confirm it goes quiet. Record both halves in the Change Review.
- [ ] 9.4 VERIFY: `cargo test --test doc_contract` green.

## 10. Change Review
<!-- kind: operational -->

- [ ] 10.1 CHECK: Dispatch an independent reviewer against proposal, specs, design, and tasks.
- [ ] 10.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING, re-run affected tests.
- [ ] 10.3 VERIFY: Confirm no blocking or unowned finding remains.

## 11. Lint & Verify
<!-- kind: operational -->

- [ ] 11.1 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 11.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings.
- [ ] 11.3 VERIFY: `make gates` — every hygiene gate, `PALETTE`, `NORAW-GREP`, `NOBLOCK`,
      `COLWIDTH` and `NODEFAULT-UI` included.
- [ ] 11.4 VERIFY: `cargo test --all-features` — green, contract tier included.
- [ ] 11.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` and the production-slice floor.
- [ ] 11.6 VERIFY: `openspec validate mouse-text-selection --strict`.
