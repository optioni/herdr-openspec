<!-- No "Acceptance Test — Outer Loop" group. design.md → Test Strategy states why: the only
     true end-to-end proof drives a real terminal, which this repository forbids at every tier
     because `cargo test` spawns this binary. The outermost honest test is `run_loop` over a
     `TestBackend`, which is group 3. -->

<!-- Counts cited below, each with the command that produced it, run at HEAD:
     31 bindings   — `grep -c 'input:' src/ui/help.rs` → 32, less the struct's own field
     6 groups      — `grep -c '^    Group {' src/ui/help.rs` → 6
     15 fields     — `awk '/^pub struct Dashboard/,/^}/' src/ui/app.rs | grep -cE '^\s+pub [a-z_]+:'` → 15
     42 rows       — `grep -n '\b42\b' src/ui/help.rs` → line 290 (the doc comment) and lines
                     902, 921, 927, 932 (four assertions in the overlay's own test module) -->

## 1. The terminal seam: capture can be set on a live guard
<!-- kind: behavior -->
<!-- Shares no file with group 2 and needs nothing from it; the two may run together. -->

- [ ] 1.1 RED: Write failing tests in `src/ui/terminal.rs` named for the spec scenarios
      "Releasing and re-entering records exactly the two operations", "Teardown is
      unconditional whatever state capture is left in", and "A start-up refusal is not
      overwritten by a later success", extending the existing `Recorder` double.
      Check: `grep -q 'set_mouse' src/ui/terminal.rs` → **exit 1 at HEAD** (run 2026-09-15),
      so the behaviour is absent and the tests cannot pass by accident.
- [ ] 1.2 GREEN: Add `TerminalGuard::set_mouse(&self, on: bool) -> Result<(), String>`
      calling `TerminalOps::enable_mouse`/`disable_mouse` and returning the `TerminalError`'s
      own `Display` text. It must not read or write `mouse_problem` (per design.md →
      Decision 7 and the spec's start-up-refusal scenario).
- [ ] 1.3 CHECK: Confirm `restore_then` is byte-identical to HEAD.
      Check: `git diff HEAD -- src/ui/terminal.rs | grep -c '^[-+].*disable_mouse()'` → must
      be **0** for the `restore_then` body; the unconditional call is the panic guarantee.
- [ ] 1.4 Run the group tests — `cargo test ui::terminal` green, no regressions.

## 2. `Dashboard::mouse_capture` and `Action::ToggleMouse`
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing tests in `src/ui/app.rs` named for "`m` toggles at both routes
      and types while filtering", "`mouse_capture` starts true and only the toggle moves it",
      and "The keyboard is unaffected by a released capture".
      Checks at HEAD (run 2026-09-15): `grep -q 'ToggleMouse' src/ui/app.rs` → **exit 1**;
      `grep -q 'mouse_capture' src/ui/app.rs` → **exit 1**.
- [ ] 2.2 GREEN: Add the `mouse_capture: bool` field initialised `true`, the
      `Action::ToggleMouse` variant, and `action_for`'s `m` arm at both routes and not while
      filtering. Every `Dashboard` construction site names the new field — there is no
      `Default` to absorb it.
- [ ] 2.3 CHECK: Update the compile-time destructure companion and the field-count assertions
      from fifteen to sixteen. Check: `awk '/^pub struct Dashboard/,/^}/' src/ui/app.rs |
      grep -cE '^\s+pub [a-z_]+:'` → must print **16** (printed 15 at HEAD).
- [ ] 2.4 Run the group tests — `cargo test ui::app` green, no regressions.

## 3. The injected capture seam and the loop
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests in `src/ui/mod.rs`'s loop suite named for "A refused
      release is a problem row and no state change" and "A refused release leaves the state it
      failed to leave", driving a closure seam that answers `Err`.
      Check: `grep -q 'Fn(bool)' src/ui/mod.rs` → **exit 1 at HEAD** (run 2026-09-15).
- [ ] 3.2 GREEN: Thread a `&dyn Fn(bool) -> Result<(), String>` into `run_loop` beside the
      artifact reader, bind it in `mod.rs` to `guard.set_mouse`, and apply `ToggleMouse` by
      calling it and setting `mouse_capture` only on `Ok` (per design.md → Decisions 1 and 8).
- [ ] 3.3 CHECK: Confirm the seam did not leak into a view. Check:
      `grep -rln 'Fn(bool)' src/ui/ | grep -v 'src/ui/mod.rs'` → must print **nothing**.
      Negative control: add `fn _p(_: &dyn Fn(bool) -> Result<(),String>) {}` to
      `src/ui/view.rs`, re-run, confirm it prints `src/ui/view.rs`, then remove it.
- [ ] 3.4 VERIFY: `make gates` green — in particular `NOBLOCK` and `NOIO-VIEW`, which the new
      seam is shaped to satisfy by being a `Fn` rather than a channel.

## 4. `Role::MouseOff` and the footer badge
<!-- kind: behavior -->
<!-- parallel-after: 2 -->
<!-- Touches `src/ui/palette.rs` and `src/ui/view.rs` only; shares no file with group 5. -->

- [ ] 4.1 RED: Write failing render tests named for "The badge is drawn first and is additive",
      "The badge survives a width that drops every hint", "A released capture drops `g focus`
      at the mandated narrow width", and "Both badges can be on screen at once and are
      distinguishable".
      Checks at HEAD (run 2026-09-15): `grep -q 'MouseOff' src/ui/palette.rs` → **exit 1**;
      `grep -q 'mouse off (m)' src/ui/view.rs` → **exit 1**.
- [ ] 4.2 GREEN: Add `Role::MouseOff` — `DIM`, and not equal to `palette::style(Role::FileMode)`
      — and render `mouse off (m)` at column 0 before `? help` when `mouse_capture` is false,
      never dropped for width.
- [ ] 4.3 CHECK: Confirm no colour literal escaped the palette. Check: `make gates` → the
      `PALETTE` gate green, which searches all of `src/` including inline test modules.
- [ ] 4.4 Run the group tests — `cargo test ui::view ui::palette` green, no regressions.

## 5. The inventory row and the counts it moves
<!-- kind: behavior -->
<!-- parallel-after: 2 -->
<!-- Touches `src/ui/help.rs` only. -->

- [ ] 5.1 RED: Write a failing test named for "`m` has a row in the `Pane` group", and update
      "The inventory's shape is asserted, not described" to expect counts 5, 7, 4, 5, 5, 6
      summing to 32.
      Check: `grep -c 'input:' src/ui/help.rs` → **32** at HEAD, which is 31 bindings plus the
      struct's own field; it must print **33** after this group.
- [ ] 5.2 GREEN: Add the `m` binding to the `Pane` group with a description naming **both**
      directions, and move the overlay's row count from 42 to 43 (32 bindings + 6 headings +
      5 blanks).
- [ ] 5.3 CHECK: Update the four sites the old row count reaches. Check:
      `grep -n '\b42\b' src/ui/help.rs` → at HEAD prints line 290 (the doc comment) and lines
      902, 921, 927, 932 (assertions, including the `1-37/42` and `6-42/42` indicator
      strings); after this group it must print none of them.
- [ ] 5.4 VERIFY: `cargo test ui::help` and `cargo test --test doc_contract` green — the
      executed binding sweep must report `ToggleMouse` once with `EXEMPT_ACTIONS` still
      length two.

## 6. Documentation
<!-- kind: operational -->

- [ ] 6.1 Rewrite in `SPEC.md`: the drag-to-select paragraph at lines 690-694 (audience:
      maintainers and readers of the pane). It currently claims the bypass "requires holding
      `Option` (macOS) or `Shift` (most Linux terminals)"; `Option` was measured **not** to
      work in Ghostty under any mode set. Replace with `Shift`, name Ghostty on macOS as what
      was measured, state that the other terminals are unmeasured, and name `m` as the
      modifier-free escape. Stays useful because it is the sentence the next reader will
      believe. Check: `grep -c 'Option\` (macOS)' SPEC.md` → **1** at HEAD, must be **0** after.
- [ ] 6.2 Add in `SPEC.md`: a Keys row for `m`, and a degraded-states row for a refused
      capture change (audience: maintainers). Both are bound to computable second sites —
      Keys to `ui::help::INVENTORY`, the degraded row to `tests/degraded-coverage.toml` — so
      a drift fails `cargo test` rather than being re-read by a human.
- [ ] 6.3 Add in `README.md`: the `m` row in Keys (audience: users). `tests/doc_contract.rs`
      binds this table to `INVENTORY`, so omitting it fails the Test gate.
- [ ] 6.4 Add in `tests/degraded-coverage.toml`: the new row's proving test name, pointing at
      the test group 3 wrote. Check: `cargo test --test degraded_coverage` green.

## 7. The corrected bypass claim is bound to a test
<!-- kind: behavior -->

- [ ] 7.1 RED: Write a failing leg in `tests/doc_contract.rs` for "The documented bypass names
      what was measured": the paragraph names `Shift`, names the measured terminal, names `m`,
      and does **not** claim the `Option` bypass.
      Check: the leg must fail at HEAD, where `SPEC.md` still claims `Option` — confirm the
      failure message names the `Option` claim and not a missing section.
- [ ] 7.2 GREEN: The leg passes once group 6.1 has landed; no production code changes here.
- [ ] 7.3 CHECK: Negative control, recorded per this repository's rule for a guard that is
      green. Restore the `Option` sentence in `SPEC.md`, confirm the leg fires, restore the
      correction, confirm it goes quiet. Record both halves in the change's Change Review.
- [ ] 7.4 VERIFY: `cargo test --test doc_contract` green.

## 8. Change Review
<!-- kind: operational -->

- [ ] 8.1 CHECK: Dispatch an independent reviewer against proposal, specs, design, and tasks.
- [ ] 8.2 CHANGE: Fix every CRITICAL, resolve or accept each WARNING, re-run affected tests.
- [ ] 8.3 VERIFY: Confirm no blocking or unowned finding remains.

## 9. Lint & Verify
<!-- kind: operational -->

- [ ] 9.1 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 9.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 warnings.
- [ ] 9.3 VERIFY: `make gates` — every hygiene gate green, `NORAW-GREP` and its per-name
      control included, since this change adds a second caller of the capture pair.
- [ ] 9.4 VERIFY: `cargo test --all-features` — green, contract tier included.
- [ ] 9.5 VERIFY: `cargo llvm-cov --fail-under-lines 80` and the production-slice floor.
- [ ] 9.6 VERIFY: `openspec validate mouse-text-selection --strict`.
