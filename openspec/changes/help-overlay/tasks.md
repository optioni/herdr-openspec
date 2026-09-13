<!-- Planning-time baselines, each with the command that produced it, run against HEAD
     f002895 on 2026-09-13. Every one is re-run in group 1 rather than trusted. -->

| Baseline | Command | Result at HEAD |
|---|---|---|
| `Action` variants | `python3` over `pub enum Action {` in `src/ui/app.rs` | **23** |
| `Dashboard` fields | `python3` over `pub struct Dashboard {` | **14** |
| `?` bound anywhere | `grep -n "Char('?')" src/ui/app.rs` | exit **1**, no match |
| `FOOTER_HINTS` | `grep -n FOOTER_HINTS src/ui/view.rs` | `[&str; 3]` |
| Files under `src/ui/` | `ls src/ui/*.rs \| wc -l` | **12** |
| `NOIO-VIEW` pure set | `make gates` | `9 pure files` |
| `COLWIDTH` pure set | `make gates` | `the eight pure view files` |
| `NOSPAWN` file count | `make gates` | `25 files checked under src (>= 25)` |
| `PALETTE` file count | `make gates` | `25 files searched (>= 25)` |
| `NODEFAULT-UI` runs | `grep -c nodefault-ui.sh Makefile` | **6** (lines 45-50) |
| Spec scenarios in this change | `grep -hc '^#### Scenario:' specs/*/spec.md \| paste -sd+ \| bc` | **100** |

**No outer-loop acceptance group.** design.md → Test Strategy states why: this change
reaches no process, no socket, and no terminal, so there is no end-to-end wiring a slower
tier could exercise that a `TestBackend` render and a swept pure function do not.

**Parallelism.** Groups 2 and 3 both edit `src/ui/app.rs`; groups 4, 5 and 6 each edit a
different file but every one of them needs `Action::ToggleHelp` and `Help` to exist first.
Group 7 is the only genuinely independent one — it edits `scripts/gates/*.sh`,
`tests/gate-controls.toml` and the `Makefile`, shares no file with any other group, and needs
only that `src/ui/help.rs` exists (group 4). It is marked `parallel-after: 4`. Everything
else is sequential, and the reason is one shared file: `src/ui/app.rs` for groups 2-3, and
`src/ui/view.rs` for groups 8-9.

---

## 1. Re-measure the baselines
<!-- kind: operational -->

- [ ] 1.1 CHECK: Re-run every command in the baseline table above and record any that moved.
  Seven changes are in flight in this checkout; a count taken at planning time and trusted at
  apply time is the defect this group exists to prevent.
- [ ] 1.2 CHECK: Run `make check` and confirm it is green before any edit, so a later failure is
  attributable to this change. Record the failing sub-command if it is not.

## 2. `Action::ToggleHelp` and the `?` key
<!-- kind: behavior -->

- [ ] 2.1 RED: Add tests in `src/ui/app.rs`'s `mod tests` named for the scenarios
  `?` maps to `ToggleHelp` outside filter mode and types inside it and
  `?` toggles the overlay and its near misses do not. Assert `Char('?')` under both `NONE` and
  `SHIFT` returns `ToggleHelp`; under `CONTROL` and `ALT`, and as `Release`/`Repeat`, returns
  `Ignore`; under `filtering` true returns `FilterPush('?')`; and that `Char('/')` with `SHIFT`
  returns `Ignore`. Confirm red: `cargo test ui::app::tests` fails on a missing `ToggleHelp`
  variant — RED at HEAD by construction, since `grep -n "Char('?')" src/ui/app.rs` exits 1.
- [ ] 2.2 GREEN: Add `ToggleHelp` to `Action` and the two `action_for` arms. Update the doc
  comment's count from twenty-three to twenty-four.
- [ ] 2.3 CHECK: Contract gate — re-read `SPEC.md` → Keys and confirm the key table there still
  matches `action_for`'s `filtering` false table, row for row.
- [ ] 2.4 Run `cargo test ui::app` — no regressions.

## 3. `Help` on `Dashboard`, and `apply`'s overlay layer
<!-- kind: behavior -->

- [ ] 3.1 RED: Add tests named for the scenarios The overlay opens and closes without moving the
  route, `Esc` closes the overlay before any other layer, The overlay layer suppresses every
  action but four, The overlay's four live actions act and nothing else moves, Both quit keys
  still quit from inside the overlay, and The agent keys launch nothing while the overlay is
  open. The suppression test applies all seventeen inert actions and asserts the dashboard is
  equal field for field.
- [ ] 3.2 GREEN: Add `pub struct Help { pub open: bool, pub scroll: usize }` with no `Default`,
  and the `help` field on `Dashboard`. Every construction site is a compile error until it names
  the field; fix each.
- [ ] 3.3 GREEN: Add `apply`'s overlay branch ahead of the existing dispatch, per
  `specs/help-overlay/spec.md`'s table. Extend `Back`'s layer order with the overlay at the
  front. Leave the blanket `needs_archived_refresh()` rule running for every action.
- [ ] 3.4 REFACTOR: If the overlay branch and the route dispatch share a scroll step, extract it;
  otherwise state that no refactor was needed.
- [ ] 3.5 Run `cargo test ui::app` — no regressions.

## 4. `src/ui/help.rs` — the inventory and the row grammar
<!-- kind: behavior -->

- [ ] 4.1 RED: Add `src/ui/help.rs` with `mod tests` holding tests named for The inventory's
  shape is asserted, not described, `Space` and `Esc` each appear under their route, Both quit
  keys have a row, and The inventory is a pure `'static` value with no construction cost. Assert
  six groups, the six titles and `scope` values in order, binding counts 5/7/4/4/3/5 summing to
  28, no empty group and no shared title.
- [ ] 4.2 GREEN: Write `Scope`, `Binding`, `Group`, and `INVENTORY` per
  `specs/binding-inventory/spec.md`'s table. No `Default` on `Binding` or `Group`; every literal
  names every field.
- [ ] 4.3 RED: Add view tests named for The grammar renders at 120 columns, The grammar renders
  at 60 columns, and The key column is measured in display columns, rendering into a
  `TestBackend` at both mandated widths.
- [ ] 4.4 GREEN: Write the row grammar — group heading with its parenthesised scope, binding rows
  padded to the key column, blank row between groups — taking every style from
  `palette::style(Role::…)` and every width from `layout::columns`/`truncate_columns`.
- [ ] 4.5 Run `cargo test ui::help` — no regressions.

## 5. `layout::help_band`
<!-- kind: behavior -->

- [ ] 5.1 RED: Add tests in `src/ui/layout.rs` named for The band's rectangle at both mandated
  widths, The band is total over degenerate and extreme rectangles, and The overlay does not move
  the breakpoint. Drive the totality test with zero-width, zero-height, 1x1, 120x1, 120x2, and
  `u16::MAX` rectangles against `content_rows` of 0, 1, 39, and `usize::MAX`.
- [ ] 5.2 GREEN: Implement `help_band`, every subtraction saturating, and assert in the test that
  every returned rectangle lies inside the body it was given.
- [ ] 5.3 Run `cargo test ui::layout` — no regressions.

## 6. Scrolling, the indicator, and the degraded frames
<!-- kind: behavior -->

- [ ] 6.1 RED: Add tests named for The overlay scrolls at both mandated sizes, A held key cannot
  run the window off the end, No indicator when the content fits, Degenerate frames render
  without panicking, and The reader is never trapped in a degenerate frame. The held-key test
  applies two hundred `Next` actions and then two hundred `Prev`, redrawing after each.
- [ ] 6.2 GREEN: Window the interior through `layout::scroll_offset(help.scroll, content_rows,
  interior_height)`, and draw the `<first>-<last>/<total>` indicator into the bottom rule when
  and only when the content does not fit. No arrow glyphs, per design.md → Decision 4.
- [ ] 6.3 GREEN: Add `help.scroll` clamping to `run_loop`'s `normalise_scroll`, on the same terms
  as `detail.scroll`.
- [ ] 6.4 GREEN: Handle the degenerate branches — nothing at zero width or height, the top rule
  alone at one row, both rules and no interior at two.
- [ ] 6.5 Run `cargo test ui::help ui::driver` — no regressions.

## 7. Gate scripts and their planted controls
<!-- kind: operational -->
<!-- parallel-after: 4 -->

- [ ] 7.1 CHECK: Confirm `make gates` is green and record the three counts this group moves:
  `NOIO-VIEW OK: 9 pure files`, `COLWIDTH OK: … the eight pure view files`, and
  `grep -c nodefault-ui.sh Makefile` = 6.
- [ ] 7.2 CHANGE: Add `src/ui/help.rs` to `PURE` in `scripts/gates/noio-view.sh` and
  `scripts/gates/colwidth.sh`, and confirm they then report ten and nine.
- [ ] 7.3 CHANGE: Extend `scripts/gates/palette.sh`'s `PURE`-list leg to fail when **either**
  `src/ui/palette.rs` or `src/ui/help.rs` is missing from either list.
- [ ] 7.4 CHANGE: Add a seventh `nodefault-ui.sh` line to the `Makefile` with
  `HOMEFILE=src/ui/help.rs TYPES='Binding Group'`, measuring its own `SCAN_MIN` by running the
  script bare and writing the measured floor on the recipe line — the one documented exception
  to "a gate's floor is its own script default".
- [ ] 7.5 CHECK: Negative control for each of 7.2-7.4, since all three pin an invariant that is
  already green. Plant `use std::fs;` in `src/ui/help.rs` and confirm `NOIO-VIEW` fires; plant
  `s.chars().count()` and confirm `COLWIDTH` fires; remove `src/ui/help.rs` from each `PURE` list
  in turn and confirm `PALETTE` fires; plant `..Default::default()` in a `Binding` literal and
  confirm `NODEFAULT-UI` fires. Remove each plant and confirm the gate goes quiet. Record both
  halves.
- [ ] 7.6 CHANGE: Add each of those plants to `tests/gate-controls.toml` so `tests/gate_controls.rs`
  executes them rather than this task having attested to them once.
- [ ] 7.7 VERIFY: Run `make gates` and `cargo test --test gate_controls` — both green.

## 8. The footer's leading hint
<!-- kind: behavior -->

- [ ] 8.1 RED: Update the landed footer assertions in `src/ui/view.rs`'s tests to the strings
  `specs/responsive-layout/spec.md` and the five other footer deltas now mandate. Every width in
  those scenarios moved by eight columns; the boundary scenarios moved to 61/60/52/51, 54/53,
  77/76, and 28/27.
- [ ] 8.2 GREEN: Change `FOOTER_HINTS` to `[&str; 4] = ["? help", "q quit", "Enter detail", "Esc
  back"]`. Nothing else in `render_footer` moves — the drop-from-the-end rule is unchanged.
- [ ] 8.3 CHECK: Contract gate — confirm `g focus` is dropped at 60 columns and that a scenario
  asserts it, per design.md → Decision 6. This is the change's one behavioural regression and it
  must be pinned by a test, not left as prose.
- [ ] 8.4 Run `cargo test ui::view` — no regressions.

## 9. Drawing the overlay, and the mouse
<!-- kind: behavior -->

- [ ] 9.1 RED: Add view tests named for The band's geometry at both mandated widths, The band
  paints every cell it covers, and The frame beneath is unchanged when the overlay closes. The
  third renders, toggles twice, and asserts the first and third buffers are byte-identical.
- [ ] 9.2 GREEN: Draw the band in `ui::view::render` after the body and before returning, leaving
  the footer row alone.
- [ ] 9.3 RED: Add tests in `src/ui/driver.rs` named for The wheel scrolls the overlay from every
  region, A click inside the band does nothing, A click outside the band dismisses it and selects
  nothing, The band's edges are inside it, Motion still costs no frame while the overlay is open,
  and Nothing in the overlay is mouse-only.
- [ ] 9.4 GREEN: Add `mouse_action`'s overlay branch ahead of the `zone` lookup, per
  `specs/mouse-input/spec.md`'s table.
- [ ] 9.5 REFACTOR: Clean up while green, or state that none was needed.
- [ ] 9.6 Run `cargo test ui` — no regressions.

## 10. The contract tier
<!-- kind: behavior -->

- [ ] 10.1 RED: Add to `tests/doc_contract.rs` the tests named for An action added without a help
  row fails `cargo test`, A binding removed from the driver and left in the help fails, The sweep
  finds the twenty-two bound actions and exactly two exemptions, The sweep covers the mouse under
  both overlay states, A binding added to the driver and not to the docs fails `make check`, The
  documented key set and the inventory agree at HEAD, and A gutted document fails as a broken
  control rather than a clean tree.
- [ ] 10.2 GREEN: Write `action_name(Action) -> &'static str` as an exhaustive `match` with no
  wildcard arm, the two sweeps, and the four-leg comparison per
  `specs/doc-conformance/spec.md`. Assert the exemption list holds exactly `FilterPush` and
  `Ignore` and has length two.
- [ ] 10.3 CHECK: Negative control — the sweep must fail in **both** directions. Temporarily
  delete `action_for`'s `Char('r')` arm and confirm the check names `Refresh` as documented but
  unreachable; restore it. Temporarily remove a `Binding` and confirm it names the action as
  bound but undocumented; restore it. Record both.
- [ ] 10.4 CHECK: Confirm the landed `documented_mouse_actions` check still passes with the new
  mouse row in `SPEC.md` → Keys' mouse table.
- [ ] 10.5 Run `cargo test --test doc_contract` — no regressions.

## 11. Change Review
<!-- kind: operational -->

- [ ] 11.1 CHECK: Dispatch `outside-in-tdd-reviewer` against proposal.md, all twelve spec files,
  design.md, and the diff. Do not fork this session.
- [ ] 11.2 CHECK: Point the reviewer at this change's own concentration points first — the action
  sweep's ability to fail in both directions, the seventeen suppressed actions, the six
  capabilities carrying byte-exact footer strings, and whether `src/ui/help.rs` reaches any I/O.
- [ ] 11.3 CHANGE: Fix every CRITICAL, resolve or accept each WARNING with a one-line reason, note
  SUGGESTIONs, re-run affected tests.
- [ ] 11.4 VERIFY: Confirm no blocking or unowned finding remains.

## 12. Documentation
<!-- kind: operational -->

- [ ] 12.1 Rewrite in `SPEC.md` → Keys (audience: this repository's agents and maintainers) — add
  the `?` row to the key table and one row to the mouse table for the dismissing click. Both
  tables are machine-bound by `tests/doc_contract.rs`, so this is the site those checks read.
- [ ] 12.2 Rewrite in `SPEC.md` → Module map and § Unit-tested modules — add `ui::help`. Both are
  bound to `src/ui/`'s contents by `tests/doc_contract.rs` and fail until they name it.
- [ ] 12.3 Rewrite in `SPEC.md` → Architecture — the pure view set goes from nine files to ten.
  This passage still reads **eight** and omits `src/ui/palette.rs`; correct both, per design.md →
  Decision 9.
- [ ] 12.4 Rewrite in `README.md` → Keys (audience: a user installing the plugin) — add `?`. Bound
  by the same contract check.
- [ ] 12.5 Rewrite in `AGENTS.md` → Architecture rules (audience: every future session) — the pure
  set is **ten** files, naming `src/ui/help.rs`; and one new durable rule, in place rather than
  appended: the bindings are data in `src/ui/help.rs` and are bound to `action_for`/`mouse_action`
  by executing them, so a new keybinding needs a `Binding` row or `cargo test` fails. Net add is
  under ten lines; the pure-set sentence is a correction, not an addition.
- [ ] 12.6 Rewrite in `openspec/IMPLEMENTATION-ORDER.md` — add a row for this change beside
  `doc-conformance` and `foldable-spec-sections` as unplanned post-roadmap work, and record in the
  in-flight section that six capabilities here carry byte-exact footer strings.

## 13. Lint & Verify
<!-- kind: operational -->

- [ ] 13.1 CHECK: Inspect the intended verification commands and affected tiers — unit, view,
  contract, gate, and wiring, all reached by `make test` and `make gates`.
- [ ] 13.2 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 13.3 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 13.4 VERIFY: `cargo build --all-features` — 0 errors. Rust has no separate type checker.
- [ ] 13.5 VERIFY: `cargo test --all-features` — green.
- [ ] 13.6 VERIFY: `make gates` — every gate green, with `NOIO-VIEW` reporting ten pure files and
  `COLWIDTH` nine.
- [ ] 13.7 VERIFY: `cargo llvm-cov --fail-under-lines 80` and the production-slice floor — green.
  Never lower, waive, or exclude; if coverage falls short, add tests.
- [ ] 13.8 VERIFY: `make check` as the single gate, naming the failing sub-command if it fails.
- [ ] 13.9 VERIFY: `openspec validate help-overlay --strict` — valid.
