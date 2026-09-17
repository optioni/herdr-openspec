# Apply progress — settings-window

Handoff note for the apply phase. Written at a **group boundary**: `tasks.md` is current,
the working tree is clean, and every landed group is committed.

## Where things stand

**35 / 79 tasks complete — groups 0 through 6.** Next group is **7. The launcher seam**.

Paused because the 5-hour session limit reached 90% used. Resume at group 7; nothing is
half-done.

| Group | Commits | Gate |
|---|---|---|
| 0. Acceptance Test — Outer Loop RED | `1d8530e` | green (RED as intended) |
| 1. Rename the overlay layer | `b2baa67` | green |
| 2. Settings and provenance | `f899865`, `26ad478`, `ebf5401`, `a78f949` | green |
| 3. The settings panel view | `094d687`, `a270768`, `efb7f4b`, `f43508b` | green |
| 4. The key, the layer, and the cursor | `60f9575`, `8fff9d6`, `fe6d19a`, `5596706`, `78b498e`, `1d203d3`, `7f86a7c`, `c1f40d9` | green after one repair |
| 5. Editing `agent_kind` | `29b5159`, `cf7e8df`, `b598f05`, `894cee3` | green first pass |
| 6. Writing `settings.toml` | `1f53f21`, `768b67b`, `00f74f2`, `2db0df7` | green after one repair |

Each group's `tasks.md` marking is its own `chore(settings-window): mark group N complete`
commit; the orchestrator is `tasks.md`'s only writer.

## Current measured state

`cargo test --lib` → **1604 passed, 2 failed**. The two failures are group 0's
**intentionally RED** outer-loop acceptance tests and stay red until group 11:

- `ui::tests::wiring::committing_an_agent_kind_writes_settings_toml_and_touches_nothing_else`
- `ui::tests::wiring::committed_kind_reaches_the_launch_without_a_second_status_call`

They are the **only** tests permitted to fail. Every integration binary is green:
`doc_contract` 118, `gate_controls` 5, `ci_workflow` 21, `cli` 10, `coverage_prod` 19,
`degraded_coverage` 10 (1 ignored), `manifest` 4, `spec_purposes` 3, `task_corpus` 3.
`make gates`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features
-- -D warnings` are all clean. `make check` is red **only** on those two tests — do not
run it expecting green before group 11.

Counts moved so far: `Action` 25 → **26**, `Dashboard` 16 → **17** fields, `INVENTORY`
6 groups / 32 bindings → 6 / **33**, `Makefile:46`'s `NODEFAULT-UI` `SCAN_MIN` 333 → **350**
with `TYPES='Dashboard Filter Detail Sections Overlay Selection Edit'`, `src/state.rs`'s
production-slice `fs::write` count 1 → **2**.

Still at their HEAD values, deliberately: `CLAIM_COUNT` **15** (task 13.6 moves it),
`NOIO-VIEW` **10** pure files and `COLWIDTH` **nine** (task 10.2 moves them),
`scripts/gates/` **31** files and `STATED_GATE_SCRIPT_COUNT` **31** (tasks 10.3/10.7),
`NODEFAULT-UI` **8** recipe lines (task 10.4).

## Deferred work later groups must still do

- **`src/ui/settings.rs` is deliberately absent from `noio-view.sh`'s and `colwidth.sh`'s
  `PURE` lists.** Task **10.1** must prove that gap (plant `use std::fs;` and
  `.chars().count()`, record both gates still exiting 0 at counts 10 and 9) *before* 10.2
  closes it. Do not add the file early. `scripts/gates/settingswidths.sh` likewise does not
  exist yet — task 10.3.
- **`src/ui/settings.rs` holds 5 `#[test]`s, all naming both `60` and `120` unsuffixed.**
  That is the floor task 10.3's `settingswidths.sh` should measure. Re-count at dispatch;
  group 7's and 8's work may add more.
- **The `,` row in `INVENTORY`'s `Pane` group was pulled forward into group 4** (`c1f40d9`)
  so the tree would not sit red for five groups. `Pane` is already 4 → 5 rows and the
  inventory is at 33 bindings. Task **9.2** still owes `Scope::Settings`, the seventh group
  `While settings is open`, and its four rows: **6 → 7 groups, 33 → 37 bindings**.
- **Task 10.5** is already done by group 1 (`b2baa67`): `Makefile:46`'s `TYPES` moved off
  `Help` and `SCAN_MIN` was re-measured. It reads `350` now after group 4 re-measured it
  again. 10.5 reduces to verifying, not performing.

## Corrections found during apply, beyond the three the lead repaired

1. **`tasks.md`'s baseline table says "`doc_contract` 115 tests".** That is the
   `#[test]`-occurrence count in the source. `cargo test --test doc_contract -- --list`
   reports **113** at HEAD `afc7246`, both before and after group 1. Use the runner count;
   it is 118 now.
2. **Task 1.4 conflicted with task 10.5.** `nodefault-ui.sh`'s positive control is anchored
   on the type *name*, so the `Help` → `Overlay` rename fails `Makefile:46` until its `TYPES`
   list moves. 1.4 demands green gates at the end of group 1; 10.5 describes the edit.
   Resolved by doing the Makefile edit in group 1. Also, 1.4's prose says `Panel` joins the
   scanned set — it cannot: that gate's positive control is
   `grep -qE "struct[[:space:]]+$T[[:space:]]*\{"` and `Panel` is an enum, which is exactly
   why 10.5's replacement list names `Overlay`, `Selection` and `Edit` but not `Panel`.
3. **Task 4.2's two line numbers had drifted** by this change's own earlier groups:
   `_config` was at `src/ui/mod.rs:484` not 481, and the `cli::worker_cli` move at `:216`
   not 215. Expect the same for any line number in groups 7–14; re-measure at dispatch.
4. **`tests/degraded-coverage.toml` binds `SPEC.md`'s degraded-states rows to proving tests
   by file and line range, and this has broken three times** (`f43508b`, `7f86a7c`, and once
   inside `2db0df7`). Any insertion into `src/ui/view.rs` or `src/ui/mod.rs` shifts a range
   onto comment lines and fails `every_table_row_has_a_proof`. Re-align to where the code
   actually is; never widen the range or weaken the test.

## The two gate rejections, and why they matter to the groups ahead

- **Group 4** left three tests red beyond the two intended: two `covers` ranges shifted by
  task 4.6's insertion into `src/ui/view.rs`, and `Action::ToggleSettings` bound with no
  `INVENTORY` row. Repaired in `7f86a7c` and `c1f40d9`.
- **Group 6** put `state::record_kind`'s one call site **after** `driver::run_loop` returned,
  writing at most the session's last committed kind on the way out of the pane. **The whole
  suite was green with that bug in it.** It was rejected because task 6.3 says "from the
  commit in `src/ui/mod.rs`'s loop"; because group 0's acceptance test presses `a` *inside*
  the loop after the commit, so the outer loop could never have closed; because design.md's
  cache-invalidation bullet requires `Launcher::set_kind` at commit time with **no pane
  restart**; and because a killed pane silently lost the reader's choice.

  Repaired in `2db0df7` with a **`KindRecorder` seam on `ClipboardWriter`'s own terms** — an
  injected closure bound in `src/ui/mod.rs` and called from `driver::maybe_record_kind_commit`
  — so `src/ui/driver.rs` still names no filesystem API, and
  `the_commit_writes_settings_toml_while_the_loop_is_still_running` now pins it.

  **Group 7 hooks `Launcher::set_kind` at that same commit point.** The seam is already
  there; do not invent a second one.

## Standing constraints every remaining implementer needs

- **Two pre-existing drifts this change does NOT own and must NOT fix**, recorded in
  `notes/apply-findings.md`: `ui::app::Refresh` carries **four** fields (`requested`,
  `reload`, `startup`, `problems`) and `agents::AgentSnapshot` carries **four** (`agents`,
  `reachable`, `stalled`, `problem`), where `dashboard-loop`'s carried-forward prose says
  three each. Every literal names all four.
- The `settings-window` band-scroll scenario asserts the window by **equality against
  `ui::layout::viewport`**, which **centres** the cursor. Do not try to satisfy a "scroll
  only as far as needed" reading.
- `openspec/schemas/tdd/` and `.claude/agents/` are graft-vendored; never edit them. Nothing
  writes inside `openspec/` except this change's own directory.
- Never lower, waive, or add an exclusion to the 80% line-coverage floor or the
  production-slice floor. If coverage falls short, add tests.
- Commit locally after each task group, Conventional Commits, ending with the
  `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>` line. Never push,
  never open a PR. Stage explicit paths — never `git add -A`/`-u`.
- The `openspec` CLI is nvm-installed and not on the default PATH: prepend
  `$HOME/.nvm/versions/node/v24.20.0/bin` in every shell that invokes it.

## How to resume

Continue the apply from **group 7**, dispatching one `outside-in-tdd-implementer` per task
group in order, gating each with `cargo test --all-features --no-fail-fast` plus `make gates`,
`cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`
before marking its `tasks.md` lines. Group **12** is the Change Review group — dispatch
`outside-in-tdd-reviewer` there, not a fork.
