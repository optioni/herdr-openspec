<!-- Planning-time baseline: commit `42e4869` on `main`. Every number below names the command
     that produced it, and every check was run at that commit with its exit status recorded. -->

**Groups 1 through 6 are sequential, and none is marked `parallel-after`.** The reason is a
compile-time chain rather than a narrative: group 2's tests cannot be written until
`changes::ArchivedScope` exists, group 3's cannot be written until `ChangeSet::archived_total`
does, groups 4 and 5 both read `Dashboard::sections`, and group 6 wires the two signatures
groups 1 and 2 changed. Groups 3 and 4 additionally edit the same file, `src/ui/app.rs`
(666 lines of production slice; `sed -n '1,/^#\[cfg(test)\]/p' src/ui/app.rs | wc -l`), and
one file is shared mutable state. Group 7 could in principle run beside group 6, but it
measures a floor over the type group 3 adds and would read a half-written tree.

**Planning-time checks, run at `42e4869`:**

| Check | Command | Result at HEAD | Why that result |
|---|---|---|---|
| A (RED) | `grep -rn "ArchivedScope\|archived_total\|SectionKey\|ToggleSection\|RowKind::Section" src/ tests/` | no output, **exit 1** | The behaviour does not exist yet; every group below turns part of this red |
| B (green + negative control) | `SCAN_MIN=135 /bin/sh scripts/gates/nodefault-ui.sh` | `150 literal/pattern spans scanned (>= 135)`, **exit 0** | The floor group 7 moves. Negative control: `SCAN_MIN=100000 …` prints `half B found only 150 … the scan is vacuous`, **exit 1** |
| C (green + negative control) | `make gates` | all gates OK, **exit 0** after the artifacts were committed | `OPENSPEC-UNTOUCHED` fails on an untracked file under `openspec/`, which is why the planning package is committed before the baseline is taken. Negative control: `cargo test --test gate_controls` (55 passed) plants a real defect per gate and requires each to exit non-zero |
| D (must-change) | `grep -rn "RowKind::Separator" src/ tests/ \| wc -l` | **8** | Must be `0` after group 5; the variant is replaced, not kept beside its successor |
| E (baseline) | `make check` | **green**, exit 0: 1 121 unit tests, 21 + 9 + 19 + 10 + 55 integration | The suite this change must leave green. An earlier run of the same command exited 2 in `gate_controls_catch_their_plants`, because this session edited `design.md` while it ran and that test asserts the repository tree is unmodified across a gate run — a real property of the test, not a flake, and worth knowing before running `make check` beside an editor |
| F (fixture size) | `ls openspec/changes/archive \| wc -l` | **28** | This repository's own archive — the pane shows five of them today |

## 1. `ArchivedScope`, `archived_total`, and the un-capped archive
<!-- kind: behavior -->

- [ ] 1.1 RED: Write failing tests in `src/changes.rs` for `The full archive is enumerated and counted under either scope`, `A collapsed archive opens no file beneath an archived change`, `An empty archive counts zero under either scope`, `An unreadable archive counts nothing and reports once, under either scope`, and `The two archived_total invariants hold under either scope`. Confirm each fails to compile on the missing `ArchivedScope` / `archived_total` rather than on fixture setup — check A above is the RED evidence.
- [ ] 1.2 GREEN: Add `changes::ArchivedScope` and `ChangeSet::archived_total`, drop `entries.truncate(archived_count)` from `archived_entries`, and thread the scope through `archived_entries`, `list_changes`, and `from_files` so `Names` builds no archived `Change` at all. Verify with `cargo test changes::tests::`.
- [ ] 1.3 GREEN: Carry `archived_total` through `changes::merge` untouched and set it to `0` in `changes::empty_set`, then extend `conformance::assert_invariants` with the set-level check that `archived.len()` is `0` or exactly `archived_total`. Verify the invariant rejects a hand-built inconsistent `ChangeSet`.
- [ ] 1.4 The permission-stripped fixture in 1.1 is the falsifiability of "a collapsed section costs no work": confirm the `Full` arm records a problem on that `Change` and the `Names` arm records none, so a `Names` path that resolved anyway goes red.
- [ ] 1.5 CHECK: Contract gate — `grep -rn "ChangeSet {" src/ tests/ | wc -l` reports **12** construction sites at HEAD; confirm the compiler named every one and that `python3 scripts/gates/gate-mech1.py` still reports no rest pattern and no functional update.
- [ ] 1.6 CHECK: Persistence gate — confirm no migration, backfill, cache invalidation, or index rebuild applies (design.md → Persistence and Rollout records `none` for each).
- [ ] 1.7 REFACTOR: Clean up while green, or state that none was needed.
- [ ] 1.8 Run `cargo test changes::` and `cargo test --all-features` — no regressions.

## 2. The refresh request carries the archived scope
<!-- kind: behavior -->

- [ ] 2.1 RED: Write failing tests in `src/refresh.rs` for `The scope on the request is the scope the file tier runs under` and `drain_and_fold unions selections and takes the last scope`, and update `A refresh outstanding does not queue further selections` and `A forced refresh outstanding behind a narrower one is not lost` to the two-field request. Confirm they fail on the missing parameter.
- [ ] 2.2 GREEN: Add `refresh::Request { selection, archived }`, change `Refresher::request` to take both, drop `archived_count` from `refresh::start` and `worker_for_test`, and pass `request.archived` to `from_files`. Verify with `cargo test refresh::tests::`.
- [ ] 2.3 GREEN: Make `drain_and_fold` fold a `Request` — `Selection::union` for the selection, last-wins for the scope (per design.md → Decision 9) — and give the remembered `Selection::All` the most recent suppressed scope. Verify single-threaded against a receiver whose sender queued two requests and was dropped.
- [ ] 2.4 CHECK: Contract gate — confirm neither `Refresher` nor `RefreshResult` names `CliChanges`, `OpenspecCli`, or `from_cli`, by running `make gates` and reading `NOCLI-SHELL` and `NOBLOCK` OK.
- [ ] 2.5 Run `cargo test refresh::` and `cargo test --all-features` — no regressions.

## 3. The section model and the re-indexed cursor
<!-- kind: behavior -->

- [ ] 3.1 RED: Write failing tests in `src/ui/app.rs` for `The first target is selected on startup at both widths`, `j, k, and the arrows move the cursor over headers and changes`, `The cursor clamps at both ends rather than wrapping`, `The cursor crosses the archived header into the archived rows`, `A collapsed section's changes are neither visible nor addressable`, `Navigation over an empty visible list is inert`, and `Enter on a section header does nothing`.
- [ ] 3.2 GREEN: Add `SectionKey`, `Sections { collapsed: BTreeSet<SectionKey> }`, and `Target`, put `sections` on `Dashboard`, and implement `section_open`, `targets`, `visible`, `visible_len`, `selected_change`, and `archived_scope` (design.md → Decisions 2, 5, 8). Verify with `cargo test ui::app::tests::`.
- [ ] 3.3 GREEN: Clamp `selected` against `targets().len()` in `apply`, and make `OpenDetail` inert on a `Target::Section`. Verify the four launch keys stay inert there through the existing `launch::decide` path, with no new rule.
- [ ] 3.4 Re-index every landed test that sets `selected` to address the *n*th change (`grep -rn "selected" src/ui/*.rs | wc -l` reports **296** mentions at HEAD, of which the assignments are the subset to walk). Update the assertions rather than rewriting the tests, and confirm each still asserts what it asserted before.
- [ ] 3.5 CHECK: Contract gate — `Dashboard::selected_change()` keeps its signature; re-inspect its call sites in `src/ui/mod.rs`, `src/ui/app.rs`, and `src/ui/driver.rs` and confirm none needed a new branch.
- [ ] 3.6 Run `cargo test ui::` and `cargo test --all-features` — no regressions.

## 4. `Space`, `ToggleSection`, and the refresh it triggers
<!-- kind: behavior -->

- [ ] 4.1 RED: Write failing tests for `Space maps to ToggleSection outside filter mode and types inside it`, `Space on a header folds and unfolds that section`, `Space inside a section folds it and moves the cursor to its header`, `An empty list makes Space inert`, `Opening an unresolved archive requests a refresh`, `A refresh does not undo a fold`, `Space types into the query rather than folding a section`, and `The first character of a query requests the archive it needs`.
- [ ] 4.2 GREEN: Add `Action::ToggleSection` — the eighteenth variant — map `KeyCode::Char(' ')` with no modifiers to it while `filtering` is false, and implement the toggle in `apply` so it moves `selected` to that section's header and changes nothing else. Verify with `cargo test ui::app::tests::`.
- [ ] 4.3 GREEN: Implement `needs_archived_refresh()` and set `refresh.requested` from it after **every** action in `apply`, not on a named subset (design.md → Decision 6). Verify the predicate self-clears once `archived.len() == archived_total`.
- [ ] 4.4 GREEN: Confirm `Dashboard::adopt` does not touch `sections`, so a live update never reopens what the reader folded.
- [ ] 4.5 Update the hand-written `variants` array in `ui::app::tests::no_action_mutates_changes` to eighteen, and confirm the exhaustive `match` and the array enumerate the same set.
- [ ] 4.6 Run `cargo test ui::app::` and `cargo test --all-features` — no regressions.

## 5. Section rows and their styling
<!-- kind: behavior -->

- [ ] 5.1 RED: Write failing tests in `src/ui/list.rs` and `src/ui/view.rs` for `The section header and archived rows render at both mandated widths`, `A collapsed archived section shows its count and no rows`, `An expanded but unresolved archived section shows its header alone`, `A section header degrades by truncation at every width`, `No archived changes means no archived header`, `A query reaches a match inside a folded archive`, and `The archived count under a query is the matched count`. Every one asserts interior widths 38 and 58.
- [ ] 5.2 GREEN: Replace `RowKind::Separator` with `RowKind::Section { key, depth, collapsed }` and emit two section headers in place of the separator, each `[marker][space][glyph][space][label][space][(count)]` through `pad_or_truncate_right`. Verify with `cargo test ui::list::tests::`.
- [ ] 5.3 GREEN: Implement the count rule — matched entries when the tier is resolved, `archived_total` when it is not — and gate each header on a count greater than zero, keeping `No changes yet`, `No active changes`, and the two-row `No changes match` states keyed off the same counts (design.md → Decision 10).
- [ ] 5.4 GREEN: Map `RowKind::Section` to `Role::ListSeparator` in `ui::view`'s style table (design.md → Decision 3). Confirm no colour literal is written outside `src/ui/palette.rs` by running `make gates` and reading `PALETTE` OK.
- [ ] 5.5 Update the landed `Active rows render at both mandated widths` and `A watch problem leads the list, above a change-set problem` assertions for the one added header row, and confirm every change row below is byte-identical to its pre-change form.
- [ ] 5.6 CHECK: `grep -rn "RowKind::Separator" src/ tests/ | wc -l` reports **0** (check D was **8** at HEAD), and `make gates` reports `COLWIDTH` OK, so the new cells are measured in display columns.
- [ ] 5.7 Run `cargo test ui::list:: ui::view::` and `cargo test --all-features` — no regressions.

## 6. Startup, the composition root, and the driver
<!-- kind: behavior -->

- [ ] 6.1 RED: Write failing tests in `src/ui/mod.rs` for `Startup counts the archive without resolving it`, and update `A scratch repository is loaded from disk with no binary present`, `No repository above the starting directory`, `Loading writes nothing`, `load reads the agent-name mapping from the directory it was given`, and `An unusable mapping file is an empty mapping with a named problem` for the new `sections` and `archived_total` fields.
- [ ] 6.2 GREEN: Make `ui::load` call `changes::from_files(root, ArchivedScope::Names)`, stop reading `config.archived_count`, and seed `sections.collapsed` with exactly `SectionKey::Archived` on both branches. Verify the loaded `Dashboard` is equal across `archived_count` values 0, 3, and 7.
- [ ] 6.3 GREEN: Pass `dashboard.archived_scope()` with every request `run_loop` makes, and drop `archived_count` from the `refresh::start` call in `ui::run`'s composition root. Verify through the composition-root test `agent-polling` added rather than by reading the wiring.
- [ ] 6.4 CHECK: Contract gate — re-read `SPEC.md` → Resolution chain's `config.toml` description and confirm the documented configuration format still matches `src/config.rs`; the format itself is unchanged and only the key's effect is.
- [ ] 6.5 Run `cargo test --all-features` — green.

## 7. Gate floors
<!-- kind: operational -->

- [ ] 7.1 CHECK: Re-run `SCAN_MIN=135 /bin/sh scripts/gates/nodefault-ui.sh` with `Sections` added to the leg's `TYPES`, and record the span count it reports. At HEAD the same command reports 150 spans and exits 0 (check B).
- [ ] 7.2 CHANGE: Add `Sections` to the first `NODEFAULT-UI` line's type set in the `Makefile` and raise that line's `SCAN_MIN` to the newly measured floor, keeping the other four legs' floors untouched.
- [ ] 7.3 CHANGE: Write `notes/gate-floors.md` in this change directory recording the command, the measured span count, and the floor chosen, the way `degraded-states` recorded the five it set.
- [ ] 7.4 VERIFY: `make gates` — every gate OK. Negative control: raising the same line's `SCAN_MIN` above the measured count must print `the scan is vacuous` and exit 1; restore it and confirm the gate goes quiet again.

## 8. Change Review
<!-- kind: operational -->

- [ ] 8.1 CHECK: Dispatch an independent `outside-in-tdd-reviewer` against `proposal.md`, all eight spec deltas, `design.md`, `tasks.md`, and the diff — not this session's reasoning. Concentration points for this change: whether any test asserts a collapsed section's *cost* rather than only its rows; whether the `selected` re-index in 3.4 quietly weakened an assertion it was only supposed to move; whether `archived_total` and `archived.len()` can disagree on any path `merge` or `adopt` takes; and whether any new width arithmetic bypassed `layout::columns`.
- [ ] 8.2 CHANGE: Fix every CRITICAL, resolve or consciously accept each WARNING with a one-line reason, note SUGGESTIONs, and re-run the affected tests.
- [ ] 8.3 VERIFY: Confirm no blocking or unowned finding remains.

## 9. Documentation
<!-- kind: operational -->

- [ ] 9.1 Rewrite in `SPEC.md`: List view (audience: every future change to this pane) — replace the `-- archived ----` sample block and the "five most recent (`archived_count`)" sentence at lines 184 and 369-379 with the two section headers, the fold, and the count. Durable because the sample block is what the next row-grammar change is written against, and it would otherwise describe a separator the crate no longer emits.
- [ ] 9.2 Rewrite in `SPEC.md`: Keys, and Resolution chain's `config.toml` description (audience: same) — add `Space`, and restate `archived_count` as accepted-but-inert. Replaces the current claim that the key limits the archived list, which is now false rather than merely incomplete.
- [ ] 9.3 Rewrite in `README.md`: the configuration table (audience: a reader configuring the plugin) — `archived_count` gains "no effect on the list" beside its default. A key documented to do something it no longer does is the defect this task removes; nothing is added.
- [ ] 9.4 Rewrite in `CLAUDE.md`: the list-region description (audience: every agent session, loaded every time) — replace "a separator, then archived ones" with the two foldable sections and `Space`, and delete the `archived_count` truncation clause from the `changes-from-files` sentence. Net change is a rewrite, not an addition: the paragraph is the same length and the stale clause goes.
- [ ] 9.5 CHECK: Confirm `SPEC.md`'s degraded-states table needs no new row — an expanded-but-unresolved section is a normal moment, not a degraded state (design.md → Decision 7) — and that `cargo test --test degraded_coverage --test doc_contract` stays green.

## 10. Lint & Verify
<!-- kind: operational -->

- [ ] 10.1 CHECK: Inspect the intended verification commands and the tiers they cover — `make check` runs format, lint, gates, test, and coverage, and is the single gate this repository ends on.
- [ ] 10.2 VERIFY: `cargo fmt --all -- --check` — clean.
- [ ] 10.3 VERIFY: `cargo clippy --all-targets --all-features -- -D warnings` — 0 errors.
- [ ] 10.4 VERIFY: `make gates` — every gate OK.
- [ ] 10.5 VERIFY: `cargo test --all-features` — green, with no test count lower than the 1 121 + 21 + 9 + 19 + 10 + 55 baseline recorded in check E.
- [ ] 10.6 VERIFY: `cargo llvm-cov --fail-under-lines 80` and the production-slice floor beside it — no floor lowered, no exclusion added.
- [ ] 10.7 VERIFY: `make check` as the single gate, naming the failing sub-command if it fails.
- [ ] 10.8 VERIFY: `openspec validate list-sections --strict` — valid.
