<!-- Numbers below each name the command that produced them, run against HEAD e9e00b9. -->

**Ordering.** Every group is sequential. The project rules record the standing parallelism
veto for this repository — one crate, one compile, and `make gates` / `make lint` / the whole
`--lib` suite sweep the entire tree, so criterion 3 (attributable failure) fails for every pair
in every change here. Cited rather than re-walked, per that rule. No group carries
`parallel-after`.

**Baseline measured at HEAD** (`git rev-parse --short HEAD` → `e9e00b9`, `git status
--porcelain` → clean):

| Quantity | Command | HEAD |
|---|---|---|
| `Action` variants | `python3` parse of `pub enum Action` in `src/ui/app.rs` | **25** |
| `,` bound anywhere | `grep -rn "Char(',')" src/ \| wc -l` | **0** |
| `help_band` call sites | `grep -rn "help_band" src/ \| wc -l` | **24** |
| `help.open` / `help.scroll` references | `grep -rno "help\.open\|help\.scroll" src/ \| wc -l` | **75** |
| `NOIO-VIEW` pure files | `scripts/gates/noio-view.sh` output | **10** |
| `COLWIDTH` pure files | `scripts/gates/colwidth.sh` output | **9** |
| `INVENTORY` groups / bindings | `grep -c "^    Group {"` / `grep -c 'input: "'` on `src/ui/help.rs` | **6** / **32** |
| `scripts/gates/` file count | `ls scripts/gates/ \| wc -l`, and `STATED_GATE_SCRIPT_COUNT` in `tests/ci_workflow.rs:688` | **31** / **31** |
| `settings.toml` writers in production | `fs::write` count in `src/state.rs` above `#[cfg(test)]` | **1** (`agent-names.toml` only) |
| `--lib` suite at HEAD | `cargo test --lib` | **1562 passed, 0 failed** (58.6s) |
| `doc_contract` tests | `cargo test --test doc_contract -- --list` | **115** |
| `CLAIM_COUNT` | `tests/doc_contract.rs:4608` | **15** (`CLAIM_COUNT_WORDS` is `[…; 6]`, topping at "fifteen") |
| `Outcome` fields / `Request` variants | `src/launch.rs:68`, `:35` | **2** / **2** |
| `resolve` probe enum | `src/resolve.rs:75` | `BinSource` — **not** `Step`; `BinResolution`, **not** `Resolved` |

**A zero-match `cargo test` filter exits 0.** Reproduced at planning time in a scratch crate:
`cargo test --lib no_such_test` → `running 0 tests … ok. 0 passed; 0 failed; 3 filtered out`,
rc 0. Every command below was therefore checked for what it *selects*, not only that it
passes; three that selected nothing were corrected, and `cargo test <a> <b>` is a cargo hard
error (`[TESTNAME]` takes one value), so multi-filter commands are written `cargo test -- <a> <b>`.

**Negative control, run at planning time.** `scripts/gates/noio-view.sh` is green at HEAD and
therefore proves nothing on its own, so the violation was planted and removed:

```sh
/bin/sh scripts/gates/noio-view.sh                       # exit 0 — "10 pure files carry no I/O API"
printf 'use std::fs;\n' | cat - src/ui/app.rs > /tmp/a && mv /tmp/a src/ui/app.rs
/bin/sh scripts/gates/noio-view.sh                       # exit 1 — "I/O API in a pure view file: src/ui/app.rs:1"
git checkout src/ui/app.rs && /bin/sh scripts/gates/noio-view.sh   # exit 0 again
```

Selected: 10 files scanned on every run; 1 line matched while planted, 0 after. The gate fires.

## 0. Acceptance Test — Outer Loop RED
<!-- kind: behavior -->

Taken because the headline claim is about what the **wired program** writes and calls — that a
committed kind reaches `settings.toml` and changes the next launch — which no unit test can
establish (design.md → Test Strategy).

- [ ] 0.1 Extend the `run_wired` harness with the replaced collaborators design.md → Test
      Boundaries names: scratch `openspec` and `herdr` programs with an invocation log,
      `ScratchDir` state and config directories, `TestBackend` terminal.
- [ ] 0.2 RED: Write the failing end-to-end test for `plugin-state` :: "Nothing outside the
      commit writes the file" and `agent-launch` :: "The next launch uses the committed kind
      and issues no status call" — drive `,`, `Enter`, `Next`, `Enter`, then `a`.
- [ ] 0.3 Confirm both fail because `Action::ToggleSettings` does not exist, not because the
      harness is misconfigured: `grep -rn "Char(',')" src/ | wc -l` → 0 at HEAD, so the
      keypress is inert and the panel is never reached.

## 1. Rename the overlay layer
<!-- kind: refactor -->

- [ ] 1.1 CHARACTERIZE: Run `cargo test -- ui::help ui::app ui::layout ui::driver ui::view` and
      record the passing count. These are the tests that must stay green and unchanged in
      substance through the rename.
- [ ] 1.2 REFACTOR: Rename `ui::app::Help` to `Overlay`, replace `open: bool` with
      `panel: Option<Panel>` where `Panel` is `Help | Settings`, and add `edit: Option<Edit>`
      — three fields, per `dashboard-loop` :: "`Dashboard` carries sixteen fields". `Dashboard`
      stays at sixteen: `help` becomes `overlay`. 75 references measured by
      `grep -rno "help\.open\|help\.scroll" src/ | wc -l`.
- [ ] 1.3 REFACTOR: Rename `ui::layout::help_band` to `overlay_band` across its 24 call sites
      (`grep -rn "help_band" src/ | wc -l`). Geometry unchanged — same `x`, `width`, `height`,
      `y`.
- [ ] 1.4 VERIFY: Run the characterization tests from 1.1 — same count, all green, no
      assertion weakened. Then `make gates` — `NODEFAULT-UI` must still pass with `Overlay`,
      `Panel`, and `Edit` in its scanned sets.

## 2. Settings and provenance
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing tests in a new `src/settings.rs` for `setting-provenance` ::
      "Three settings in a fixed order, whatever the inputs", "Every `integration::Source`
      maps to a `Provenance` and back to one label", "Every probe step is named by the step
      that won", "`Unresolved` is file mode's own label and is not an error", "The
      `Configured` rule outranks the shortlist rule", "Read-only settings stay read-only
      however they were resolved", "An empty shortlist never reaches the view as `Kind`", "A
      pending kind renders as resolving and edits nothing", "An ambiguous kind has no effective
      value and offers both candidates".
- [ ] 2.2 GREEN: Implement `Setting`, `Provenance`, `Editable`, `Reason`, `KindResolution`, and
      `settings(config, binary, kind)`. `Provenance` derives from `integration::Source` by a
      total `From` with no wildcard arm and carries `resolve::BinSource` whole — the real type
      names are `BinSource` and `BinResolution` (`src/resolve.rs:75,97`); `resolve::Step` and
      `resolve::Resolved` do not exist. `Pending` and `Ambiguous` come from `kind: None` and
      `Choice::Ambiguous`, neither of which carries a `Source` (design.md → Decisions 6).
- [ ] 2.3 GREEN: Register `pub mod settings;` in `src/lib.rs`, and in the same task add its row
      to `SPEC.md`'s Module map and the `settings::` token under `SPEC.md` → `### Unit-tested
      modules`. `tests/doc_contract.rs:251 module_map_matches_lib_rs` and `:353
      tested_modules_names_every_module` both go red on a `pub mod` with no row, so this is one
      task, not a documentation afterthought.
- [ ] 2.4 GREEN: Add the `tests/doc_contract.rs` claim that `src/settings.rs`' production slice
      names no I/O API, no clock, and no `ratatui` type — the **sixteenth** claim
      (`CLAIM_COUNT` is 15 at HEAD), on the eleventh's and fourteenth's pattern, since no
      `make gates` script sweeps outside `src/ui/`. Raising the claim count moves four sites
      together; that is task 13.6.
- [ ] 2.5 CHECK: Plant `use std::fs;` above the `#[cfg(test)]` line in `src/settings.rs`,
      confirm `cargo test --test doc_contract` goes red naming that file, remove it, confirm
      green. Record both halves.
- [ ] 2.6 Run the group tests — `cargo test settings::` **and** `cargo test --test doc_contract`
      as two commands. One command cannot do both: `--test doc_contract` restricts to that
      binary, so it would skip the `src/settings.rs` unit tests entirely, and no doc_contract
      test name contains `settings::`, so the filter would select zero and still exit 0.

## 3. The settings panel view
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests in a new `src/ui/settings.rs` for `settings-window` :: "The
      panel renders every setting at both mandated widths", "A long path is truncated rather
      than wrapped or overflowing", "The panel degrades rather than panicking at any frame
      size". Every test asserts at both 60 and 120 columns.
- [ ] 3.2 GREEN: Implement `ui::settings::render` — heading row, top rule, six setting rows
      (value row plus indented source row), bottom rule. `─` is the only chrome glyph.
- [ ] 3.3 GREEN: Measure every width through `ui::layout::truncate_columns`; name no
      `.chars()` measure, which `COLWIDTH` will sweep once the file joins its `PURE` list.
- [ ] 3.4 RED then GREEN: `responsive-layout` :: "Both panels are centred by the one function"
      — a `#[test]` in `src/ui/layout.rs` calling `overlay_band` at `content_rows` 42 and 7
      against both a 120x40 and a 60x20 body. Group 1 renames the function but adds no test, so
      without this the scenario has none.
- [ ] 3.5 Run the group tests — `cargo test -- ui::settings ui::layout` — no regressions.

## 4. The key, the layer, and the cursor
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests for `settings-window` :: "`,` toggles the panel and its near
      misses do not", "The panels swap rather than stacking", "`Esc` closes the settings panel
      before any other layer", "The cursor walks settings, not rendered rows, and saturates",
      "The band scrolls only when the cursor would leave it"; `dashboard-loop` :: "`,` maps to
      `ToggleSettings` and moves no existing key", "`Dashboard` still carries sixteen fields
      after the overlay is generalised", "Both panels open is unrepresentable"; `help-overlay`
      :: "`?` and `,` swap panels rather than stacking them", "`,` swaps to the settings panel
      from inside the help".
- [ ] 4.2 GREEN: Add `Dashboard::settings: settings::PanelState { rows, cursor }` — the
      **seventeenth** field — and populate `rows` at startup from `ui::load`. Two plumbing
      edits, not one: drop the underscore on `_config` (`src/ui/mod.rs:481`), and retain the
      `FoundBin` in `run_at` before `resolution` is **moved** into `cli::worker_cli`
      (`src/ui/mod.rs:215`), passing it to `load` as a further parameter — `found.source` has
      no production reader today, so `Provenance::Probe` has no data source until this lands.
      Do not probe twice. Update every construction site and the compile-time companion that
      destructures all fields with no `..` rest.
- [ ] 4.3 GREEN: Add `Action::ToggleSettings` (25 → 26) and bind `,` under
      `KeyModifiers::NONE` only, typing as `FilterPush(',')` while filtering.
- [ ] 4.4 GREEN: Add the settings dispatch layer to `Dashboard::apply`, taking precedence over
      the route and filter dispatches on the help layer's terms, and extend the help layer's
      answered set to eight with `ToggleSettings` swapping the panel.
- [ ] 4.5 GREEN: Move the row cursor with `Next`/`Prev` through `ui::layout::viewport`, the
      primitive the foldable detail path already uses.
- [ ] 4.6 GREEN: Wire `ui::settings::render` into `ui::view::render` behind
      `overlay.panel == Some(Panel::Settings)`.
- [ ] 4.7 Run the group tests — `cargo test -- ui::app ui::help ui::view ui::settings` — no
      regressions.

## 5. Editing `agent_kind`
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing tests for `settings-window` :: "An edit begins, changes a
      candidate, and commits", "`Esc` cancels the edit and a second `Esc` closes the panel",
      "`Enter` on a read-only setting begins no edit", "The shortlist is the installed kinds,
      in Herdr's order, and wraps", "A committed kind outside the shortlist starts the edit at
      the first entry", "No installed integration makes the row non-editable with a reason", "A
      configured `agent_kind` refuses the edit and names the file", "The refusal is per
      setting, not per panel".
- [ ] 5.2 GREEN: Implement `Edit` — begin on `OpenDetail`, commit on `OpenDetail`, cancel on
      `Back`; `Next`/`Prev` cycle the shortlist with wrap while an edit is in progress.
- [ ] 5.3 GREEN: Render the candidate rather than the committed value in the value row while
      an edit is in progress, at both mandated widths.
- [ ] 5.4 Run the group tests — `cargo test -- ui::app ui::settings` — no regressions.

## 6. Writing `settings.toml`
<!-- kind: behavior -->

- [ ] 6.1 RED: Write failing tests for `plugin-state` :: "A commit creates the file with the
      chosen kind", "A commit rewrites rather than merges", "A write failure is reported and
      does not lose the session's value", "Nothing is written outside the state directory".
      Use `testutil::ScratchDir`; the filesystem is real here (design.md → Test Boundaries).
- [ ] 6.2 GREEN: Implement `state::record_kind` on `state::record`'s atomicity — temp file in
      the same directory, then rename. Create the state directory when absent and nothing
      above it.
- [ ] 6.3 GREEN: Call it from the commit in `src/ui/mod.rs`'s loop — never from `src/ui/app.rs`,
      which performs no I/O — and record a failure as a problem row.
- [ ] 6.4 CHECK: Persistence gate — confirm no migration, backfill, cache invalidation, or
      index rebuild applies beyond `Launcher::set_kind` (group 7), per design.md → Persistence
      and Rollout.
- [ ] 6.5 CHECK: Contract gate — re-read `SPEC.md`'s `settings.toml` description and confirm
      the one-key schema it documents still matches what `record_kind` emits.
- [ ] 6.6 Run the group tests — `cargo test -- state:: ui::` — no regressions.

## 7. The launcher seam
<!-- kind: behavior -->

- [ ] 7.1 RED: Write failing tests for `agent-launch` :: "The ambiguous outcome opens the
      settings panel on `agent_kind`", "No other outcome touches the overlay", "The next launch
      uses the committed kind and issues no status call", "A cancelled edit changes nothing the
      launcher sees", "`set_kind` returns without blocking".
- [ ] 7.2 GREEN: Take `launch::Outcome` from two fields to **four** — adding `picker: bool`,
      set only by `Choice::Ambiguous`, and `resolution: Option<KindResolution>` — and add
      `Request::Resolve`, sent when `,` opens the panel and answered from the worker's session
      cache when it has one. Open the settings panel on the `agent_kind` row when `drain` adopts
      an outcome whose `picker` is set.
- [ ] 7.3 GREEN: Add `Launcher::set_kind(&mut self, kind: String)` and `Request::Resolve`,
      replacing the session cache with `Choice::Use { kind, source: Source::Recorded }`. There
      are **five** `impl Launcher` sites across **two** files — `NoLauncher`, `RealLauncher`,
      `TestLauncher` in `src/launch.rs`; `RecordingLauncher`, `ScriptedLauncher` in
      `src/lib.rs`; none in `src/ui/mod.rs`. `NoLauncher` is a second **production** impl, not
      a double — it is what file mode gets — so `set_kind` there is a no-op and `Resolve`
      answers nothing.
- [ ] 7.4 CHECK: Contract gate — `Launcher` gains a method and `Request` a variant, which
      breaks every implementor. Confirm by compiling that all **five** named in 7.3 are
      updated, and that `NoLauncher`'s no-op is deliberate rather than inherited.
- [ ] 7.5 VERIFY: `make gates` — `NOBLOCK` must still pass and no file under `src/ui/` names the
      lock type. Do **not** treat this as evidence that `set_kind` is non-blocking:
      `noblock.sh`'s `BLOCK3_RE` (`:105`) names no `Mutex`, `RwLock`, or `lock()`, and its
      lock-naming leg is scoped to `src/ui/`, so it is green either way. The FIFO-ordering test
      in 7.1 is that evidence (design.md → Decisions 14).
- [ ] 7.6 CHECK: Persistence gate — this is the group that performs the cache invalidation, so
      confirm here that `set_kind` and `Request::Resolve` need no migration, backfill, or index
      rebuild, per design.md → Persistence and Rollout.
- [ ] 7.7 Run the group tests — `cargo test -- launch:: ui::app` — no regressions.

## 8. Mouse
<!-- kind: behavior -->

- [ ] 8.1 RED: Write failing tests for `mouse-input` :: "A click on a setting row selects it and
      begins no edit", "A click outside the band dismisses whichever panel is open", "A click
      outside cancels an edit rather than closing the panel". Both mandated widths.
- [ ] 8.2 GREEN: Add `Target::Setting(usize)` and resolve a click inside the band against the
      setting rows; heading, rule, and blank rows resolve to `Ignore`.
- [ ] 8.3 GREEN: Change the click-outside row from `Action::ToggleHelp` to `Action::Back`
      (design.md → Decisions 8) and update the help panel's own click-outside tests, which
      assert the old action.
- [ ] 8.4 Run the group tests — `cargo test -- ui::driver ui::app` — no regressions.

## 9. The binding inventory
<!-- kind: behavior -->

- [ ] 9.1 RED: Write failing tests for `binding-inventory` :: "The settings group is present and
      names the reinterpreted keys", "The settings group renders at both mandated widths", "The
      sweep covers the mouse under all three overlay states", "`ToggleSettings` has a `Pane` row
      and no exemption", and the updated "The sweep's totals and the exemption set are pinned".
- [ ] 9.2 GREEN: Add `Scope::Settings` with the suffix `settings panel`; add the `,` row to
      `Pane` (4 → 5) and group 7 `While settings is open` with its four rows (6 → 7 groups,
      32 → 37 bindings).
- [ ] 9.3 GREEN: Extend `tests/doc_contract.rs`'s mouse sweep from two overlay states to three
      and raise the swept union from 25 to 26 names and the compared set from 23 to 24.
- [ ] 9.4 CHECK: Delete the `,` row from `INVENTORY` and confirm `cargo test --test doc_contract`
      goes red naming `ToggleSettings` as bound but not documented; restore it and confirm
      green. Record both halves.
- [ ] 9.5 Run the group tests — `cargo test --test doc_contract` **and** `cargo test ui::help`
      as two commands, for the reason 2.6 gives: no doc_contract test name contains `ui::help`,
      so the combined form selects zero and exits 0.

## 10. Gates and the counts they pin
<!-- kind: operational -->

- [ ] 10.1 CHECK: `make gates` is **green** with `src/ui/settings.rs` present and unlisted —
      both `PURE` lists are fixed literals, so an unlisted file is simply unswept. Prove the gap
      instead of assuming it: plant `use std::fs;` and `.chars().count()` in
      `src/ui/settings.rs`, run `noio-view.sh` and `colwidth.sh`, and record exit 0 with counts
      10 and 9 — the file is invisible to both. Leave the plants in place for 10.2.
- [ ] 10.2 CHANGE: Add `src/ui/settings.rs` to `noio-view.sh`'s `PURE` (10 → 11) and
      `colwidth.sh`'s `PURE` (9 → 10). Each gate's count is a **hardcoded string**, not a
      computed one — `noio-view.sh:63` says "10 pure files", `colwidth.sh:46` says "the nine
      pure view files", and `noio-view.sh:3,33` says "PURE is NINE files" and "from nine files
      to TEN" — so edit all four literals too, or the messages stay wrong while the gates pass.
      With the 10.1 plants still in place both gates must now exit 1 naming the file; remove the
      plants and confirm exit 0 with counts 11 and 10.
- [ ] 10.3 CHANGE: Add `scripts/gates/settingswidths.sh` on **`helpwidths.sh`**'s pattern — a
      count against a floor (`HELP_MIN`'s shape, `scripts/gates/helpwidths.sh:23`) plus the
      vacuity leg, measured from `src/ui/settings.rs`'s own tests. Not `detailwidths.sh`, which
      requires *every* `#[test]` to name both widths; `specs/responsive-layout/spec.md` specifies
      the count-against-a-floor shape.
- [ ] 10.4 CHANGE: Add `HOMEFILE=src/settings.rs TYPES='Setting KindResolution PanelState'` as
      `NODEFAULT-UI`'s **ninth** scanned set (`grep -c nodefault-ui.sh Makefile` is 8 at HEAD),
      on its own recipe line with its own measured `SCAN_MIN`, recorded in
      `notes/gate-floors.md`. Only **structs** may be named: the gate's positive control is
      `grep -qE "struct[[:space:]]+$T[[:space:]]*\{"`, so `Provenance`, `Editable`, `Reason`,
      and `Panel` cannot join and are covered by exhaustive `match` instead.
- [ ] 10.5 CHANGE: Update the existing first `NODEFAULT-UI` line (`Makefile:46`) from
      `TYPES='Dashboard Filter Detail Sections Help Selection'` to
      `'Dashboard Filter Detail Sections Overlay Selection Edit'`, and re-measure its
      `SCAN_MIN` from 333. That gate's positive control is anchored on the name, so the
      `Help` → `Overlay` rename fails it until this lands — which is what the anchoring is for.
- [ ] 10.6 CHANGE: Bind `settingswidths.sh` to a planted defect in `tests/gate-controls.toml`,
      **and add the three controls `help-overlay` set as precedent for a new pure view file** —
      mirroring its `noio-view-help-hit`, `colwidth-sweep-help`, and `palette-help-unswept`
      entries — and widen `scripts/gates/palette.sh:37-41` from two hard-coded names to three so
      it requires `src/ui/settings.rs` in both `PURE` lists. Without these, the file silently
      dropping out of either list is invisible.
- [ ] 10.7 CHANGE: Bind `settingswidths.sh` into `tests/ci_workflow.rs`' recipe enumeration,
      and raise `STATED_GATE_SCRIPT_COUNT` in `tests/ci_workflow.rs:688` from 31 to 32. The
      matching figure moves through `specs/quality-gates/spec.md`, this change's own delta —
      **never** by editing `openspec/specs/` in place, which `OPENSPEC-UNTOUCHED`'s tracked-diff
      leg exists to refuse.
- [ ] 10.8 VERIFY: Run `/bin/sh scripts/gates/settingswidths.sh` against a copy of the tree with
      a both-widths test narrowed to 120 alone, confirm non-zero; restore, confirm zero. Record
      the count it reports on each run.
- [ ] 10.9 VERIFY: `make gates` — every gate green, and each printed count matches the figure
      this group set.

## 11. Acceptance Test — Outer Loop GREEN
<!-- kind: behavior -->

- [ ] 11.1 VERIFY: Confirm the group 0 tests now pass end to end — the commit reaches
      `settings.toml`, the next launch carries `--kind`, and the invocation log holds no second
      `integration status`.
- [ ] 11.2 REFACTOR: Fold any harness setup duplicated between group 0 and groups 6 and 7 into
      one helper, or state that none was.

## 12. Change Review
<!-- kind: operational -->

- [ ] 12.1 CHECK: Dispatch an independent reviewer — not a fork of this session — against
      proposal.md, all nine delta specs, design.md, and the diff.
- [ ] 12.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a
      one-line reason, note SUGGESTIONs, and re-run affected tests.
- [ ] 12.3 VERIFY: Confirm no blocking or unowned finding remains.

## 13. Documentation
<!-- kind: operational -->

- [ ] 13.1 Rewrite in `README.md` → Keys (audience: users): add the `,` row. Rewrite rather
      than add — the Keys table is bound to `ui::help::INVENTORY` by `tests/doc_contract.rs`,
      so it goes red until it matches.
- [ ] 13.2 Rewrite in `SPEC.md` → Keys and → the mouse table (audience: maintainers): add `,`
      and the setting-row click, and change the click-outside row to `Back`. The mouse table is
      bound row by row by executing `mouse_action`, so a stale row fails `cargo test`.
- [ ] 13.3 Rewrite in `SPEC.md` → the write-boundary sentence and the `settings.toml`
      description (audience: maintainers): the plugin's own writes become
      `agent-names.toml` **and** `settings.toml`. This corrects a sentence that currently names
      one file and would otherwise be false.
- [ ] 13.4 Rewrite in `SPEC.md` → degraded-states table (audience: maintainers): add rows for
      "no installed integration, `agent_kind` not editable" and "`settings.toml` write failed",
      each bound to its proving test in `tests/degraded-coverage.toml`.
- [ ] 13.5 Rewrite in `AGENTS.md` (audience: agents) the **nine** sentences this change
      falsifies, none of which is machine-bound except the first, so the rest drift silently:
      `:308` the claim count and its enumerated list; `:336` and `:342` `NODEFAULT-UI`'s two
      "eight"s → nine; `:403` the quoted "ten pure files" / "nine pure view files"; `:445-449`
      the pure set's count **and** its enumerated file list, which gains `src/ui/settings.rs`;
      `:512` `COLWIDTH`'s "nine" → ten; `:517` "The **six** `*WIDTHS` gates" → seven; and the
      **two** write-boundary sentences at `:545` and `:552`. `CLAUDE.md` is a symlink to
      `AGENTS.md`, so this is one file. All corrections, no additions.
- [ ] 13.6 Rewrite the doc-conformance claim count at all four sites together, or
      `tests/doc_contract.rs` stays red: `CLAIM_COUNT` 15 → 16 (`tests/doc_contract.rs:4608`),
      `CLAIM_COUNT_WORDS` extended to `[(&str, usize); 7]` with `("sixteen", 16)` (`:4613`),
      `AGENTS.md:308` "fifteen further claims" → "sixteen", and a new bullet under `SPEC.md` →
      `### Doc-conformance checks` naming `src/settings.rs`' no-I/O claim. That section holds 16
      bullets for 15 claims — the last, "A claim with no second site is argued in review, not
      checked", is a meta-statement and must stay last, so insert the new claim above it.

## 14. Lint & Verify
<!-- kind: operational -->

- [ ] 14.1 CHECK: Inspect the intended verification commands and affected tiers — unit, view,
      run-time, contract, and gate are all touched by this change.
- [ ] 14.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 14.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 14.4 VERIFY: `make gates` — every gate green.
- [ ] 14.5 VERIFY: `cargo test --all-features` — green, and the count is at or above the
      baseline recorded in 1.1.
- [ ] 14.6 VERIFY: `cargo llvm-cov --fail-under-lines 80`, and the production-slice floor from
      `scripts/coverage-prod.py` — both green. Never lower a floor; add tests instead.
- [ ] 14.7 VERIFY: `make check` as the single gate, naming the failing sub-command if it fails.
- [ ] 14.8 VERIFY: `openspec validate settings-window --strict`.
