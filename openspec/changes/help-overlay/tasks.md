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

**Parallelism: none. Every group is sequential, and the reason is a shared file in each
case.** Groups 2 and 3 both edit `src/ui/app.rs`. Group 6 edits three files — `src/ui/help.rs`,
`src/ui/app.rs` (`normalise_help_scroll` is a `Dashboard` method, beside `normalise_scroll` at
`src/ui/app.rs:865`) and `src/ui/driver.rs` (the `run_loop` call site) — so it overlaps groups
2-3, 4 and 9. Groups 8 and 9 both edit `src/ui/view.rs`. Groups 9 and 10 each own a
documentation edit their own gate reads, so neither can be deferred past the other.

Group 7 was marked `parallel-after: 4` in an earlier draft and the marker is **withdrawn**; it
failed all three independence conditions. Its plants land in the real working tree, and
`src/ui/help.rs` — the file it plants `use std::fs;` and a `.chars().count()` into — is exactly
the file groups 4, 6 and 9 edit. Worse, `..Default::default()` in a `Binding` literal is a hard
compile error once `Binding` has no `Default`, so while that plant is in place **every**
concurrent group's `cargo test` fails on group 7's plant rather than on its own work. And
`make gates` sweeps the whole tree, so group 7's own runs would read another lane's half-written
file. A wrongly-marked pair means two agents in one working tree, which is worse than the serial
run it avoids.

---

## 1. Re-measure the baselines
<!-- kind: operational -->

- [x] 1.1 CHECK: Re-run every command in the baseline table above and record any that moved.
  Seven other changes are in flight in this checkout.
- [x] 1.2 CHECK: Run `make check` and confirm it is green before any edit, so a later failure is
  attributable to this change. Record the failing sub-command if it is not.

## 2. `Action::ToggleHelp` and the `?` key
<!-- kind: behavior -->

- [x] 2.1 RED: Add tests in `src/ui/app.rs`'s `mod tests` named for the scenarios
  `?` maps to `ToggleHelp` outside filter mode and types inside it and
  `?` toggles the overlay and its near misses do not. Assert `Char('?')` under both `NONE` and
  `SHIFT` returns `ToggleHelp`; under `CONTROL` and `ALT`, and as `Release`/`Repeat`, returns
  `Ignore`; under `filtering` true returns `FilterPush('?')`; and that `Char('/')` with `SHIFT`
  returns `Ignore`. Confirm red: `cargo test ui::app::tests` fails on a missing `ToggleHelp`
  variant — RED at HEAD by construction, since `grep -n "Char('?')" src/ui/app.rs` exits 1.
- [x] 2.2 GREEN: Add `ToggleHelp` to `Action` and the two `action_for` arms. Update the doc
  comment's count from twenty-three to twenty-four.
- [x] 2.3 CHECK: Contract gate — re-read `SPEC.md` → Keys and confirm the key table there still
  matches `action_for`'s `filtering` false table, row for row.
- [x] 2.4 REFACTOR: None expected — the change is one enum variant and two match arms. State
  explicitly that no refactor was needed if that holds.
- [x] 2.5 Run `cargo test ui::app` — no regressions.

## 3. `Help` on `Dashboard`, and `apply`'s overlay layer
<!-- kind: behavior -->

- [x] 3.1 RED: Add tests named for the scenarios The overlay opens and closes without moving the
  route, `Esc` closes the overlay before any other layer, The overlay layer suppresses every
  action but seven, The overlay's seven live actions act and nothing else moves, Both quit keys
  still quit from inside the overlay, The agent keys launch nothing while the overlay is open,
  The overlay swallows the seventeen inert actions, and `j` and `k` scroll the overlay rather
  than the frame beneath. The last two are `help-overlay`'s own and are not duplicates of the
  `dashboard-loop` pair above them: one drives eleven actions at `Route::List`, the other pins
  the route-agnostic scroll and the saturating underflow.
- [x] 3.2 GREEN: Add `pub struct Help { pub open: bool, pub scroll: usize }` with no `Default`,
  and the `help` field on `Dashboard`. Every construction site is a compile error until it names
  the field; fix each.
- [x] 3.3 GREEN: Add `apply`'s overlay branch ahead of the existing dispatch, per
  `specs/help-overlay/spec.md`'s table. Extend `Back`'s layer order with the overlay at the
  front. Leave the blanket `needs_archived_refresh()` rule running for every action.
- [x] 3.4 REFACTOR: If the overlay branch and the route dispatch share a scroll step, extract it;
  otherwise state that no refactor was needed.
- [x] 3.5 Run `cargo test ui::app` — no regressions.

## 4. `src/ui/help.rs` — the inventory and the row grammar
<!-- kind: behavior -->

- [x] 4.1 RED: Add `src/ui/help.rs` with `mod tests` holding tests named for The inventory's
  shape is asserted, not described, `Space` and `Esc` each appear under their route, Both quit
  keys have a row, and The inventory is a pure `'static` value with no construction cost. Assert
  six groups, the six titles and `scope` values in order, binding counts 5/7/4/4/5/6 summing to
  31, no empty group and no shared title. These are the **repaired** counts: groups 5 and 6 went
  from 3 and 5 to 5 and 6 in the planning review, and an earlier draft of this line still read
  `5/7/4/4/3/5` summing to 28. `specs/binding-inventory/spec.md`'s table is the authority.
- [x] 4.2 GREEN: Write `Scope`, `Binding`, `Group`, and `INVENTORY` per
  `specs/binding-inventory/spec.md`'s table. No `Default` on `Binding` or `Group`; every literal
  names every field.
- [x] 4.3 RED: Add view tests named for The grammar renders at 120 columns, The grammar renders
  at 60 columns, and The key column is measured in display columns, rendering into a
  `TestBackend` at both mandated widths.
- [x] 4.4 GREEN: Write the row grammar — group heading with its parenthesised scope, binding rows
  padded to the key column, blank row between groups — taking every style from
  `palette::style(Role::…)` and every width from `layout::columns`/`truncate_columns`.
- [x] 4.5 REFACTOR: Extract the key-column measurement if 4.4's grammar and 4.2's data ended up
  computing it twice, or state that no refactor was needed.
- [x] 4.6 Run `cargo test ui::help` — no regressions.

## 5. `layout::help_band`
<!-- kind: behavior -->

- [x] 5.1 RED: Add tests in `src/ui/layout.rs` named for The band's rectangle at both mandated
  widths, The band is total over degenerate and extreme rectangles, and The overlay does not move
  the breakpoint. Drive the totality test with zero-width, zero-height, 1x1, 120x1, 120x2, and
  `u16::MAX` rectangles against `content_rows` of 0, 1, 39, and `usize::MAX`.
- [x] 5.2 GREEN: Implement `help_band`, every subtraction saturating, and assert in the test that
  every returned rectangle lies inside the body it was given.
- [x] 5.3 REFACTOR: Fold `help_band`'s centring arithmetic into the existing saturating helpers
  if one already exists in `src/ui/layout.rs`, or state that no refactor was needed.
- [x] 5.4 Run `cargo test ui::layout` — no regressions.

## 6. Scrolling, the indicator, and the degraded frames
<!-- kind: behavior -->

- [ ] 6.1 RED: Add tests named for The overlay scrolls at both mandated sizes, A held key cannot
  run the window off the end, No indicator when the content fits, Degenerate frames render
  without panicking, and The reader is never trapped in a degenerate frame. The held-key test
  applies two hundred `Next` actions and then two hundred `Prev`, redrawing after each.
- [ ] 6.2 RED: Add the test named for The overlay's scroll is clamped where the detail region's
  is not, driving one `run_loop` iteration at 60x20 on `Route::List` with `help.scroll` `99` and
  asserting it lands on `22` while `detail.scroll` is untouched.
- [ ] 6.3 GREEN: Window the interior through `layout::scroll_offset(content_rows, help.scroll,
  interior_height)` — the signature is `(lines, scroll, height)`, confirmed at
  `src/ui/layout.rs:121` — and draw the `<first>-<last>/<total>` indicator into the bottom rule
  when and only when the content does not fit. No arrow glyphs, per design.md → Decision 4.
- [ ] 6.4 GREEN: Add `Dashboard::normalise_help_scroll(frame_area)` and call it in `run_loop`
  beside `normalise_scroll`, per design.md → Decision 10.
- [ ] 6.5 GREEN: Handle the degenerate branches — nothing at zero width or height, the top rule
  alone at one row, both rules and no interior at two.
- [ ] 6.6 REFACTOR: Collapse the three degenerate branches into one guard if they share a shape,
  or state that no refactor was needed.
- [ ] 6.7 Run `cargo test ui::help ui::driver` — no regressions.

## 7. Gate scripts and their planted controls
<!-- kind: operational -->

- [ ] 7.1 CHECK: Confirm `make gates` is green and record the three counts this group moves:
  `NOIO-VIEW OK: 9 pure files`, `COLWIDTH OK: … the eight pure view files`, and
  `grep -c nodefault-ui.sh Makefile` = 6.
- [ ] 7.2 CHANGE: Add `src/ui/help.rs` to `PURE` in `scripts/gates/noio-view.sh` and
  `scripts/gates/colwidth.sh`, and confirm they then report ten and nine.
- [ ] 7.3 CHANGE: Extend `scripts/gates/palette.sh`'s `PURE`-list leg to fail when **either**
  `src/ui/palette.rs` or `src/ui/help.rs` is missing from either list.
- [ ] 7.4 CHANGE: Add a seventh `nodefault-ui.sh` line to the `Makefile` with
  `HOMEFILE=src/ui/help.rs TYPES='Binding Group'`. Measure its `SCAN_MIN` by running the script
  bare and write that floor on the recipe line, as the other six do.
- [ ] 7.5 CHANGE: Add a planted control per gate to `tests/gate-controls.toml` — `use std::fs;`
  in `src/ui/help.rs` for `NOIO-VIEW`, `s.chars().count()` there for `COLWIDTH`, `src/ui/help.rs`
  struck from each `PURE` list for `PALETTE`, and `..Default::default()` in a `Binding` literal
  for `NODEFAULT-UI`. `tests/gate_controls.rs` copies the tree to a `testutil::ScratchDir` and
  applies each plant there, which is the boundary design.md → Test Boundaries names.
- [ ] 7.6 CHECK: Negative control — run `cargo test --test gate_controls` and confirm each of
  the four plants makes its gate exit non-zero, and that `make gates` on the unplanted tree exits
  zero. Record both halves; all four pin invariants that are already green, so the planted half
  is the only evidence any of them can fail.
- [ ] 7.7 CHANGE: Add `scripts/gates/helpwidths.sh` on `detailwidths.sh`'s pattern and compose
  it into the `gates:` recipe. `src/ui/help.rs` is otherwise the only view module with a
  both-widths mandate and no gate counting it (`Makefile:39,40,42,64,66` carry the other five).
  Measure its floor from the tree once group 4's and group 6's tests exist, and add its plant to
  `tests/gate-controls.toml`.
- [ ] 7.8 VERIFY: Run `make gates` and `cargo test --test gate_controls` — both green.

## 8. The footer's leading hint
<!-- kind: behavior -->

- [ ] 8.1 RED: Update the landed footer assertions in `src/ui/view.rs`'s tests to the strings
  `specs/responsive-layout/spec.md` and the five other footer deltas now mandate. Every width in
  those scenarios moved by eight columns; the boundary scenarios moved to 61/60/52/51, 54/53,
  77/76, and 28/27.
- [ ] 8.2 RED: Update the four wiring-tier footer assertions in `src/ui/mod.rs` that the
  prepend breaks — byte-exact `assert_eq!`s at lines 3544 and 3546
  (`a_polled_agent_reaches_a_rendered_badge`), `footer_row.contains("g focus")` at 3646 inside a
  `for width in [120, 60]` loop (`a_keypress_launches_an_agent`), and
  `row.starts_with("q quit")` at 4423 (`a_refused_capture_is_named_last`). The two 60-column
  ones lose `g focus` per design.md → Decision 6; line 3437 renders at 120 only and survives.
  Confirm with `grep -n 'q quit\|g focus' src/ui/mod.rs` that five sites exist and four move.
- [ ] 8.3 GREEN: Change `FOOTER_HINTS` to `[&str; 4] = ["? help", "q quit", "Enter detail", "Esc
  back"]`. Nothing else in `render_footer` moves — the drop-from-the-end rule is unchanged.
- [ ] 8.4 CHECK: Contract gate — confirm a scenario asserts that `g focus` is dropped at 60
  columns, per design.md → Decision 6.
- [ ] 8.5 REFACTOR: None expected — the change is one const's length and contents. State
  explicitly that no refactor was needed if that holds.
- [ ] 8.6 Run `cargo test ui::view ui::tests::wiring` — no regressions.

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
- [ ] 9.5 GREEN: Add the dismissing-click row to `SPEC.md` → Keys' **mouse** table. It must land
  in this group, not in Documentation: `tests/doc_contract.rs:1850`'s `documented_mouse_actions`
  compares that table against `mouse_action`'s own `Action::` variants, so 9.4 turns it red and
  only this row turns it green again.
- [ ] 9.6 CHECK: Contract gate — re-run `cargo test --test doc_contract documented_mouse` and
  confirm the table and the function agree.
- [ ] 9.7 REFACTOR: Clean up while green, or state that none was needed.
- [ ] 9.8 Run `cargo test ui` — no regressions.

## 10. The contract tier
<!-- kind: behavior -->

- [ ] 10.1 RED: Add to `tests/doc_contract.rs` the tests named for An action added without a help
  row fails `cargo test`, A binding removed from the driver and left in the help fails, The sweep
  finds the twenty-two bound actions and exactly two exemptions, The sweep covers the mouse under
  both overlay states, A binding added to the driver and not to the docs fails `make check`, The
  documented key set and the inventory agree at HEAD, and A gutted document fails as a broken
  control rather than a clean tree.
- [ ] 10.2 GREEN: Write `action_name(Action) -> &'static str` as an exhaustive `match` with no
  wildcard arm, the two sweeps, and the three-leg comparison per
  `specs/doc-conformance/spec.md`. Assert the exemption list holds exactly `FilterPush` and
  `Ignore` and has length two. The mouse sweep's first run yields **seven** names including
  `SelectTab`; if it yields six the fixture's selected change is carrying no artifacts.
- [ ] 10.2a RED: Add the tests named for A row naming the wrong key fails, Every non-mouse row
  parses and agrees at HEAD, and An unparseable spelling fails rather than skipping.
- [ ] 10.2b GREEN: Write the `input` parser and the per-row assertion
  `action_for(press(code, mods), filtering_for(scope)) == binding.action`, with the
  every-row-parsed totality assertion. Without this the action-set check passes against an
  inventory that names the wrong key for every row (`specs/binding-inventory/spec.md`).
- [ ] 10.3 CHECK: Negative control — the sweep must fail in **both** directions. Temporarily
  delete `action_for`'s `Char('r')` arm and confirm the check names `Refresh` as documented but
  unreachable; restore it. Temporarily remove a `Binding` and confirm it names the action as
  bound but undocumented; restore it. Record both.
- [ ] 10.4 CHECK: Confirm the landed `documented_mouse_actions` check still passes with the new
  mouse row in `SPEC.md` → Keys' mouse table.
- [ ] 10.5 GREEN: Add the `?` row to `SPEC.md` → Keys' key table and to `README.md` → Keys.
  Legs 2 and 3 of the new check read those two documents, so they must land here or 10.1's
  tests cannot go green in their own group.
- [ ] 10.6 REFACTOR: Collapse the two sweeps' shared setup if it duplicates, or state that no
  refactor was needed.
- [ ] 10.7 Run `cargo test --test doc_contract` — no regressions.

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

- [ ] 12.0 CHECK: Confirm the passages this group edits still say what it expects — that
  `AGENTS.md:339` still reads "The pure set is **nine** files" and already names
  `src/ui/palette.rs`, and that `AGENTS.md:247` still reads "nine further claims". If either has
  moved, a concurrent change edited it and this group's text needs rebasing.
  **Both sentences live in `AGENTS.md` only.** `SPEC.md` carries no pure-set count and no
  pure-view file list anywhere, and its § Doc-conformance checks is an uncounted bullet list;
  an earlier draft of 12.0 and 12.2 sent this group hunting `SPEC.md` for a sentence reading
  "eight" that is in fact in the **landed `openspec/specs/dashboard-loop/spec.md`**, which this
  change repairs through its own delta rather than by editing `SPEC.md`. Do not go looking.
  The `SPEC.md`/`README.md` → Keys rows are **not** here: groups 9 and 10 own them, because a
  gate in each of those groups reads them.
- [ ] 12.1 CHANGE: Rewrite in `SPEC.md` → Module map's `ui` row and § Unit-tested modules
  (audience: this repository's agents) — mention the overlay and the binding inventory in prose.
  No check enforces this: both map checks read `src/lib.rs`'s top-level `pub mod` set, so a
  submodule is invisible to them (`specs/doc-conformance/spec.md`).
- [ ] 12.2 CHECK: **No `SPEC.md` edit is owed here — record that and move on.** An earlier draft
  of this line ordered the pure-view set corrected in `SPEC.md` → Architecture from eight to ten.
  `SPEC.md` has no such passage: `grep -n 'pure set' SPEC.md` and a search for any
  `src/ui/<file>.rs` in the pure list both return nothing, and its render-seam paragraph states
  the property in prose while enumerating no file. The prose nine → ten edit is **task 12.4's**,
  on `AGENTS.md:339`, and the landed-spec drift design.md → Decision 9 names is repaired by this
  change's own `dashboard-loop` delta. Confirm both by running those two greps and record the
  result.
- [ ] 12.3 CHANGE: `AGENTS.md:247`'s `tests/doc_contract.rs` claim count goes from **nine** to
  **ten**, this change adding the inventory/key-table binding. `SPEC.md` § Doc-conformance
  checks carries the same claims as an **uncounted bullet list**, so its share is a **tenth
  bullet** naming the new binding — an addition, not a changed numeral. Both per
  `specs/doc-conformance/spec.md`, whose two bullets were re-pointed for exactly this reason.
- [ ] 12.4 CHANGE: Rewrite in `AGENTS.md` → Architecture rules (audience: every future session) — the pure
  set is **ten** files, naming `src/ui/help.rs`; and one new durable rule, in place rather than
  appended: the bindings are data in `src/ui/help.rs` and are bound to `action_for`/`mouse_action`
  by executing them, so a new keybinding needs a `Binding` row or `cargo test` fails. Net add is
  under ten lines; the pure-set sentence is a correction, not an addition.
- [ ] 12.5 CHANGE: Rewrite in `openspec/IMPLEMENTATION-ORDER.md` — add a row for this change beside
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
