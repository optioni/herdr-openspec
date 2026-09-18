# Change review — findings and disposition

Written by task 12.2, against the independent change review of `settings-window`. Every
finding below is either **fixed** (with the commit that fixed it) or **consciously
accepted** (with the reason). This file is task 12.2's durable record; task 12.3 checks
against it.

## CRITICALs

### CRITICAL 1 — the agent-launch scenario "File mode answers both additions inertly" had no test

**Fixed** — `8882022`. Task 7.1's RED list omitted this `agent-launch` scenario and group 7
was marked complete without it; `NoLauncher::set_kind` (a second **production**
implementation, not a test double) was exercised only against the real `Launcher` and
`RecordingLauncher`, never against `NoLauncher` itself.

Added `launch::tests::seam::the_inert_launcher_ignores_set_kind` (the `src/launch.rs` half)
and `ui::tests::wiring::file_mode_answers_both_additions_inertly` (the run-time + view
half, design.md's row for this scenario): a true no-repository dashboard — `ScratchDir`
holding no `openspec/` anywhere above it, so `start_collaborators` hands out
`launch::none()` — opens `,` at both mandated widths and asserts `overlay.panel`, the
`Pending`/non-editable `agent_kind` row, no `Outcome` ever produced, and `set_kind`'s
inertness.

### CRITICAL 2 — dashboard-loop's ADDED requirement stated six wrong field counts

**Fixed** — `4730fbf`. The compile-time-companion bullet at
`specs/dashboard-loop/spec.md:833-843` (archived into `openspec/specs/` verbatim on
archive, and what a future change reads to learn what breaks the build when it adds a
field) understated six counts: `Dashboard` sixteen instead of seventeen, `Detail` five
instead of seven, `Refresh` "three" instead of four, `Launch` "both" instead of three,
`launch::Outcome` "two"/"both" instead of four (named twice), and `AgentSnapshot` "three"
instead of four. The `:1073-1074` duplicate ("naming all sixteen fields ... so a
seventeenth breaks the build") was corrected the same way, to seventeen/eighteenth.

Two of these (`Refresh`, `AgentSnapshot`) had previously been deferred in
`notes/apply-findings.md`'s "Recorded, not repaired" section as belonging to landed
changes (`live-refresh`, `agent-polling`). That reasoning is correct for the *descriptive*
`SHALL carry exactly **three** fields` prose at `spec.md:634-650`, which stays untouched —
but not for the compile-companion bullet, which re-authors the requirement wholesale on
archive. `apply-findings.md` now records that narrower reversal, with the reasoning for
why the two locations are treated differently.

### CRITICAL 3 — quality-gates' MODIFIED requirement stated the wrong gate count

**Fixed** — `7cb0087`. `specs/quality-gates/spec.md` said `NODEFAULT-UI` has **seven** type
sets and seven `SCAN_MIN` values, at two locations (the requirement's own body and the
"Each extracted gate's default floor" scenario). `Makefile:47-55` carries **nine**
`nodefault-ui.sh` recipe lines, matching this change's own `design.md:77` and
`specs/setting-provenance/spec.md:148`. A MODIFIED block replaces the whole requirement,
so this would have archived the wrong count into the very requirement governing the gate
this change extended.

## WARNINGs

### W4 — the two acceptance tests could not tell a right-reason pass from a deadline pass

**Fixed** — `cb654c8`. Eight of nine stage predicates in
`committing_an_agent_kind_writes_settings_toml_and_touches_nothing_else` and
`committed_kind_reaches_the_launch_without_a_second_status_call` were `|| true`, and
`run_wired_staged` discarded `testutil::Stages::completed_every_stage()`, so neither test
could tell "every stage fired because its predicate went true" apart from "the 30s deadline
forced an early `q`". `run_wired_staged` now returns `completed_every_stage()` as a third
tuple element (every other call site updated to destructure it, mostly as `_completed`);
both acceptance tests assert it, and stage2 (the `j` that moves the row cursor onto
`agent_kind`) now waits for the herdr log to actually contain `"integration status"`.

### W5 — the mouse-table negation disclaimer faked its own coverage

**Fixed** — `cfd549b`. `SPEC.md`'s "click outside the band" row ended "**Not**
`Action::ToggleHelp`: ...". `tests/doc_contract.rs`'s `backticked_action_variants` has no
negation awareness — it just finds `"Action::"` substrings — so the row's declared outcome
set was `{Back, ToggleHelp}`; a future regression reintroducing the exact bug Decision 8
fixed would already look "covered". The same bug existed on the code side:
`mouse_action`'s own doc comment and its "not `Action::ToggleHelp`" comment both named the
literal substring inside the function's production slice, which
`mouse_bindings_match_spec_md` also scans textually — so fixing only `SPEC.md` failed that
test outright. Both sides now drop the `Action::` qualifier from the negation. Confirmed:
the row's declared set is now `{Back}` alone (verified by running
`mouse_bindings_match_spec_md` and reading `tests/doc_contract.rs:1918-1932`'s leg 2/3
still pass over that narrowed set).

### W6 — the "Herdr socket unreachable" degraded-states row was bound to unrelated code

**Fixed** — `b21d670`. `tests/degraded-coverage.toml` bound the row to
`Dashboard::attribution()` (`src/ui/app.rs:1916-1930`), which reads `agents.agents`,
`repo`, `changes`, and `agent_names.names` — never `problem` or `reachable`. Instrumented
and hot, so the check passed while proving nothing; the pre-change binding was equally
unrelated (my own `0449612` rebinding was cosmetic, as the review found). Rebound to
`src/ui/list.rs:456-460`: the code that actually explains the fixture's own claim — a
non-stalled snapshot's `problem` stays silent by construction, gated on
`dashboard.agents.stalled`, which `an_unreachable_socket_renders_the_agentless_pane`'s
fixture never sets. Verified: `python3 scripts/coverage-prod.py target/llvm-cov.json
tests/degraded-coverage.toml` → `COVERAGE-PROD OK: production 96.48% (6223/6450) >= floor
96%` (also full re-run at the end of this task; see the final report).

### W8 — the write-boundary comments stated a false invariant

**Fixed** — `5275dd5`. `src/ui/driver.rs:265-267` and `src/ui/app.rs:1195-1199` both
asserted "`Dashboard::apply` is the only place that ever changes the `agent_kind` row's
value". `Dashboard::adopt_launch_outcome` replaces the whole row whenever an adopted
`Outcome`'s `resolution` is `Some`. The code is safe only because `drive_live_tier` —
`adopt_launch_outcome`'s one caller — runs at the top of `run_loop`'s iteration, strictly
before `before_kind` is read, so any such replacement is already folded into `before` by
the time the comparison runs; neither comment stated that ordering. Both comments now name
the real invariant, and `ui::driver::tests::adopting_a_resolved_kind_never_calls_record_kind`
drains a resolution-bearing `Outcome` with a recording `record_kind` in a single-iteration
drive and asserts zero calls while also asserting the row's value visibly changed.

### W10 — help-overlay's row-grammar counts had no delta and would archive stale

**Fixed** — `d9285c1`. `openspec/specs/help-overlay/spec.md:176-178` says "six groups,
thirty-two bindings ... `content_rows` is therefore **43**"; truth is seven groups,
thirty-seven bindings, `content_rows` **50** (`src/ui/help.rs:332-334`). This change's own
`specs/help-overlay/spec.md` touched only lines 3, 87, and 102 — none of them this
requirement. Added a MODIFIED requirement reproducing "The overlay's row grammar is a rule
row, groups, and a rule row" wholesale with the corrected arithmetic, and a **Reason**
paragraph naming why it was missing.

### W11 — two legs of a MODIFIED scenario were unexercised

**Fixed (softened, per the review's own second option)** — `c192d82`.
`specs/quality-gates/spec.md`'s "Each extracted gate's default floor is the measured one"
scenario asserted two further legs as `THEN`/`AND` claims — a floor set one above the
measured count exits non-zero, and no floor appears in both a script's default and the
`Makefile`'s recipe line — that no automated check exercises;
`tests/ci_workflow.rs:657`'s correspondence check matches only `scripts/gates/<name>`
tokens and never inspects an `env`/`SCAN_MIN` prefix. Rather than inventing the probes,
the scenario now keeps its one proven `THEN` and states the other two as design intent in
prose, saying plainly that neither is machine-checked yet.

### W12 — the settings panel's Pending-kind state had no view test

**Fixed** — `a25ee7c`. `specs/settings-window/spec.md:129` "The panel draws before the kind
has resolved" had no view test at either mandated width: every fixture in
`src/ui/settings.rs` built `settings::settings` with a **resolved** kind. Added
`fixture_rows_pending()` (`kind: None`) and
`the_panel_draws_before_the_kind_has_resolved`, naming both `120x40` and `60x20` and
asserting `Provenance::Pending`, `Editable::No{Reason::Resolving}`, and unbracketed,
unembellished `"resolving"` source text. `SETTINGSWIDTHS`' floor moved `5 -> 6` in the same
commit (the script's own default, per "a gate's floor is its own script default"), and the
`settingswidths-narrowed` `gate-controls.toml` plant's expected count and comment were
updated to the new true-both-width count it now drops from (`6 -> 5`).

## Consciously accepted — not built

### W7 — the coverage tier is dark for the duration of every outside-in change

**Consciously accepted; not fixed.** As the review itself frames it, this is a
repository-level gate-architecture defect, not `settings-window`'s: `Makefile:27`'s `cargo
llvm-cov` run aborts on a red outer-loop test before `coverage-prod.py` (`Makefile:29`) is
ever reached, and `Makefile:77` orders `test` before `coverage` regardless — darkening the
`covers` execution check, the anti-vacuity guards, and both coverage floors, locally and in
CI, for every group of every outside-in change. The only in-`cargo test` stand-in
(`tests/degraded_coverage.rs:386-389`) accepts any range whose first non-empty line is not
a comment, which is how a struct field declaration survived seven groups, and the review
found roughly ten more rows in the same shape across unrelated capabilities.

Fixing it means reordering `make check`, adding something like `cargo llvm-cov
--ignore-run-fail` or a separate `make covers-check` target, hardening the binding to a
minimum instrumented fraction, and re-binding roughly ten rows across capabilities this
change does not own. That is a change of its own to propose, not something to fold into
`settings-window`'s task 12.2. **Follow-up: propose a new OpenSpec change — working name
`coverage-tier-hardening` — scoped to (1) making `make check` still reach
`coverage-prod.py` on a red-but-instrumented suite, (2) tightening
`tests/degraded_coverage.rs`'s anti-vacuity guard beyond "first non-empty line is not a
comment", and (3) auditing and re-binding the ~10 rows the review found in the same
vacuous shape.** W6 above fixed only the one row this change itself disturbed; the other
~10 are this follow-up's to find and fix.

## SUGGESTIONs

All fixed in `f423f9a`:

- `tasks.md:328` — task 12.1's wording said "all nine delta specs"; there are eleven.
  Wording only; no checkbox touched (task 12.2's one permitted `tasks.md` edit).
- `src/ui/app.rs:2262` (now `:2267`) — `action_for`'s doc comment said "one of the
  twenty-four actions"; there are twenty-six.
- `src/integration.rs:14-15` — said a new pure-view file would move `NOIO-VIEW`'s "ten pure
  files" and `COLWIDTH`'s "nine pure view files"; now eleven and ten.
- `specs/responsive-layout/spec.md:145` — the parenthetical "(the help panel's inventory)"
  beside `content_rows` of `42` was false (real count is 50); reworded to describe what `42`
  actually is. The literal `42` is untouched.
- `specs/dashboard-loop/spec.md:723` — "thirty-two `'static` literals ... at thirty-two
  sites"; `INVENTORY` now holds thirty-seven.
