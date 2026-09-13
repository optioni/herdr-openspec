## Reviewed Artifacts

- `proposal.md`
- `design.md`
- `tasks.md`
- `specs/help-overlay/spec.md` (new capability)
- `specs/binding-inventory/spec.md` (new capability)
- `specs/dashboard-loop/spec.md`
- `specs/responsive-layout/spec.md`
- `specs/mouse-input/spec.md`
- `specs/doc-conformance/spec.md`
- `specs/view-palette/spec.md`
- `specs/list-filtering/spec.md`
- `specs/agent-launch/spec.md`
- `specs/agent-poller/spec.md`
- `specs/agent-attribution/spec.md`
- `specs/quality-gates/spec.md`

Twelve capability deltas, 116 scenarios, 116 verification-matrix rows.

## Reviewed Against

- This repository HEAD: `7aa8f17` (review began at `5249315`; two repair commits landed
  mid-review, `53e721e` and `7aa8f17`)
- Sibling repository `~/Code/openspec-schemas` HEAD: **Not applicable** — this change vendors
  nothing new and edits no vendored file
- Working tree: clean but for this change's own planning files, which were intentionally
  included. One reviewer read the tree while repairs were uncommitted and briefly reported
  already-fixed text as current; see Method notes.

## Method

The finding pass was delegated to **four** `planning-reviewer` subagents, none of which wrote
the planning package, dispatched simultaneously with one slice each and told to report findings
only: **(A)** capability coverage, scenario quality, cross-artifact contradictions; **(B)**
design completeness, test boundaries, and whether each proposed check could fail at all;
**(C)** task alignment, lifecycle discipline, `parallel-after` independence; **(D)** factual
verification by running commands. This session merged the findings, verified each one
independently before acting on it, repaired the owning artifact, and wrote this file. No
reviewer edited anything.

**The pattern worth carrying forward: every one of the four highest-value findings came from
executing something, not from reading it.** Slice B ran the sweep its own spec mandated and got
a different answer. Slice C ran `grep` over `src/ui/mod.rs` and found four broken assertions no
task owned. This session read `src/ui/layout.rs:121` and found a transposed signature. A
document review found the contradictions; it could not have found these.

## Gaps Found and Fixed

Severity is as assigned after this session verified the finding, not as first reported.

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | `specs/help-overlay` | Specified that `run_loop`'s `normalise_scroll` also clamp `help.scroll`. That function returns early when the detail region is not drawn — which `detail-scroll` **requires** of it — and the detail region is not drawn at `Route::List` below the breakpoint, which is exactly the 60x20 fixture the held-key scenario uses. The clamp would have been absent in the one case the scenario exists to pin. | Second normaliser, `Dashboard::normalise_help_scroll`, called beside the first. No `detail-scroll` delta needed; the two read different geometry anyway. | `specs/help-overlay`, `specs/dashboard-loop` (loop requirement, step 9), design.md → Decision 10, tasks 6.2/6.4 |
| CRITICAL | `tasks.md` | Groups 9 and 10 could not go green before group 12. `documented_mouse_actions` and the new check's prose legs read `SPEC.md`/`README.md` → Keys, whose edits were parked in Documentation. Change Review would have run against a red tree. | Doc rows moved into the groups whose own gates read them; group 12 keeps only what no check enforces. | tasks 9.5, 9.6, 10.5, 12.0-12.5 |
| CRITICAL | `tasks.md`, `design.md` | Four byte-exact footer assertions in `src/ui/mod.rs`'s `mod wiring` break on the `? help` prepend (lines 3544, 3546, 3646, 4423). No task owned them; `src/ui/mod.rs` was not in design.md's Modules touched. `cargo test` would have been red from group 8 onward. | Task 8.2 owns the four; file added to Boundaries; Test Strategy's "two scenarios" corrected to four. | tasks 8.2, design.md → Boundaries, Test Strategy |
| CRITICAL | `specs/binding-inventory` | The mouse sweep's expected first-run set was six names. Executing it yields **seven**: `mouse_action` maps `Zone::DetailTab` through `detail::tab_at` to `Action::SelectTab` (`src/ui/driver.rs:235-245`), reachable exactly with the multi-tab fixture the spec mandates. The natural "fix" — narrowing the fixture — would have silently removed the mouse's tab and detail-header coverage from the whole anti-drift check. | Set corrected to seven, with the citation and an explicit instruction never to narrow the fixture. | `specs/binding-inventory` |
| CRITICAL | `specs/binding-inventory` | The anti-drift check bound `action` and never `input`. An `INVENTORY` reading `k  scroll down` or `Ctrl-C  refresh` passed every proposed check — every action named once, counts right, and the overlay wrong about which key does what, which is the only thing a reader opens it for. The bookkeeping was bound; the goal was not. | New requirement: parse each key row's `input` and assert `action_for(press(code, mods), filtering_for(scope)) == binding.action`, with an every-row-parsed totality assertion. | `specs/binding-inventory` (new requirement, 3 scenarios), tasks 10.2a/10.2b |
| CRITICAL | `specs/binding-inventory` | The `While filtering` group of three could not cover `action_for`'s **six** non-typing filter rows (`src/ui/app.rs:1213-1225`). `↑` and `↓` had no row, and the action-set check was structurally blind to it because `Prev`/`Next` are already named by group 1 — a set comparison cannot see a missing key whose action is spoken for elsewhere. | Group 5 → five bindings, group 6 → six (five rows cannot name six mouse actions). Inventory 28 → **31** bindings, 39 → **42** content rows; every band figure re-propagated. | `specs/binding-inventory`, `specs/help-overlay`, `specs/responsive-layout`, `specs/mouse-input` |
| CRITICAL | `tasks.md`, `design.md` | `parallel-after: 4` on group 7 failed all three independence conditions. Its plants landed in the **real** working tree, in `src/ui/help.rs` — the file groups 4, 6 and 9 edit — and `..Default::default()` on a type with no `Default` is a hard compile error, so every concurrent group's `cargo test` would have failed on group 7's plant. It also contradicted design.md's own Test Boundaries row. | Marker withdrawn; plants moved into `tests/gate-controls.toml` and the `ScratchDir` copy. **No group in this change is parallel** — recorded as a finding. | tasks.md parallelism note, tasks 7.5/7.6, design.md → Decision 11 |
| CRITICAL | `specs/help-overlay` | `ui::layout::scroll_offset` is `(lines, scroll, height)` (`src/ui/layout.rs:121`). All three call sites had the two `usize` arguments transposed — which compiles and silently clamps against the wrong bound. | Argument order corrected everywhere; scenarios now assert the exact clamped values (`5`, `25`) so a transposition goes red rather than plausible. | `specs/help-overlay`, tasks 6.3, design.md → Decision 10 note + Risks |
| CRITICAL | `specs/help-overlay`, `specs/dashboard-loop` | The two deltas gave `Char('/')`+`SHIFT` **different actions** — `FilterStart` in one, `Ignore` in the other. `action_for`'s filter arm matches `KeyModifiers::NONE` only, so `Ignore` is correct. Two implementers writing RED tests from the two specs would have written opposite assertions. | `specs/help-overlay` corrected. | `specs/help-overlay` |
| CRITICAL | `specs/help-overlay` | The scroll-indicator scenario asserted `11-37/39` after ten `Next` at 120x40 — a string the requirement's own formula cannot produce at any offset, and self-contradictory about whether it was clamped. | Corrected to the clamped value; now `6-42/42` under the revised inventory. | `specs/help-overlay` |
| WARNING | `specs/doc-conformance` | Claimed adding `src/ui/help.rs` fails the module-map and tested-modules checks until `SPEC.md` names it. **False** — `pub_mod_names` reads `src/lib.rs`'s top-level `pub mod` set and `SPEC.md` carries one `ui` row for all of `src/ui/`. The scenario asserting the failure was unfalsifiable, which is the exact defect this capability exists to forbid. | Requirement rewritten to state the granularity honestly, bind what has a computable second site, and **assert the invisibility** so a future check extension is told rather than surprised. | `specs/doc-conformance`, tasks 12.1 |
| WARNING | `specs/help-overlay`, `specs/dashboard-loop` | The overlay's suppression silently contradicted four capabilities owning those actions (`detail-scroll`, `artifact-tabs`, `artifact-folds`, `live-updates`). Their scenarios stay green only because fixtures default `help.open` false — post-archive self-contradiction, invisible to the build. | One precedence clause in `dashboard-loop` binding every capability that owns an action, rather than four more deltas. A capability added later inherits it. | `specs/dashboard-loop` |
| WARNING | `specs/help-overlay`, `specs/binding-inventory` | Nothing pinned the overlay's content when the Herdr socket is unreachable — and the entire accepted cost of dropping `g focus` rests on "the overlay lists `g` anyway", argued in three places with no test. | New scenario: `agents.reachable` false renders a band byte-identical to the reachable one. | `specs/help-overlay` |
| WARNING | `design.md` | Test Boundaries said the `openspec` binary and Herdr socket were "not reached at all". False — ten inherited wiring scenarios spawn scratch `#!/bin/sh` programs. The no-acceptance-group argument was resting on it. | Both rows reworded; acceptance argument restated as "no **new** collaborator", which is the reason that survives. | design.md → Test Boundaries, Test Strategy |
| WARNING | `design.md` | Thirteen verification-matrix rows carried the wrong tier/collaborators/command, two of them inverted. The matrix had been generated by assigning a tier per capability — wrong for a MODIFIED block, which drags inherited scenarios along, several of which are `make gates` greps and one of which spawns a real scratch `openspec`. | Every row reclassified from its own scenario body. Gate rows 2 → 15; `coverage` and `wiring` tiers added; per-tier commands stated. | design.md → Test Strategy + matrix |
| WARNING | `specs/responsive-layout`, `tasks.md` | `src/ui/help.rs` would have been the only view module carrying a both-widths mandate with nothing counting it; the other five have `detailwidths.sh`, `listwidths.sh`, `mdwidths.sh`, `taskwidths.sh`, `widths.sh` (`Makefile:39,40,42,64,66`). | `helpwidths.sh` added on `detailwidths.sh`'s pattern, with its own planted control. | `specs/responsive-layout`, tasks 7.7 |
| WARNING | `specs/help-overlay` | Requirement and scenario headers read "six inputs" over a table naming seven actions, and "four live actions" against 24 − 17 = 7. | Renamed to seven; the spec now states the 17 + 7 = 24 accounting so the two lists are exhaustive between them. | `specs/help-overlay`, `specs/dashboard-loop` |
| WARNING | `specs/agent-launch` | Claimed the four-separate-hints form would drop three hints at 60 columns. Arithmetic: the first five reach 59, so only `s archive` and `g focus` drop. | Corrected to two, with the 59-column figure named. | `specs/agent-launch` |
| WARNING | `specs/binding-inventory` | "No allocation happened" had no observable second site; comparing two reads of a const is tautological. | Replaced with `const _: &[Group] = INVENTORY;` — a compile-time item, so a non-const-evaluable inventory fails to compile. Purity greps delegated to the gates that own them rather than counted twice. | `specs/binding-inventory` |
| WARNING | `specs/help-overlay` | Two scenarios asserted "no process was spawned, no file was read or written, and no clock was read" against `apply`, which takes `&mut self` and an `Action`, holds no handle, and cannot do any of those. They passed against an empty implementation. | Replaced with the field-for-field equality plus the `NOIO-VIEW` sweep — the evidence that can actually fire. | `specs/help-overlay` |
| WARNING | `specs/quality-gates` | A scenario bullet was commentary dressed as an assertion ("the row-0 half of this claim is removed rather than updated…"), and it orphaned the preceding line. | Justification moved into the requirement prose, as `view-palette`'s block does; the bullet is now an outcome. | `specs/quality-gates` |
| WARNING | `tasks.md` | Six behavior groups had no REFACTOR task and no "no refactor was needed" statement; group 12 was operational with no CHECK evidence task; group 9 changed a consumer-facing interface with no contract gate. | All added. | tasks 2.4, 4.5, 5.3, 6.6, 8.5, 9.6, 10.6, 12.0 |
| WARNING | `tasks.md` | Two GREEN-before-RED inversions, one of them introduced by this session's own repair (8.2 before 8.4). | Reordered; 8.1/8.2 are now one contiguous RED block and group 6 renumbered. | tasks group 6, group 8 |
| SUGGESTION | `specs/dashboard-loop` | Inherited from the live spec: `Detail` "carries exactly **five** fields" naming a `source: String`. It carries **seven** (`src/ui/app.rs:184-206`) — `foldable-spec-sections` replaced `source` with `sections` and added `expanded` and `drawn_width` without carrying the count back. | Corrected in passing under Decision 9's rule. | `specs/dashboard-loop` |
| SUGGESTION | `specs/doc-conformance` | Legs 1-2 restated `binding-inventory`'s action-sweep contract. | Reduced to a citation; the three legs renumbered. | `specs/doc-conformance` |
| SUGGESTION | `specs/dashboard-loop` | Scenario titled "The fourteenth field…" inside the ADDED "fifteen fields" requirement. | Renamed. | `specs/dashboard-loop` |
| SUGGESTION | `tasks.md`, `design.md` | Four task lines justified rather than instructed; the parallelism note's file map was wrong twice (group 6 touches three files; `normalise_scroll` is `src/ui/app.rs:865`, not `driver.rs`). | Trimmed and corrected. | tasks 1.1, 7.4, 7.6, 8.4; design.md → Boundaries |
| SUGGESTION | `proposal.md` | `settings-window` was named as merely overlapping `dashboard-loop`. It overlaps **three** capabilities and plans the same overlay machinery from the same `/`-filter precedent. | Full collision table added, naming the route-versus-layer disagreement as the load-bearing one. | proposal.md → Sequencing cost |

| CRITICAL | `tasks.md`, `design.md` | The 28 → **31** binding and 39 → **42** content-row repair propagated to the four spec files and to neither of the other two artifacts. `tasks.md:94-95` still instructed the group-4 implementer to assert counts `5/7/4/4/3/5` summing to 28 — three bindings short, and two of the three are the `While filtering` rows the repair existed to add. `design.md:6,362,511` still read "five gestures" and "Thirty-nine content rows". An implementer reading its own task line would have written the RED test against the pre-repair inventory and the spec would have lost. Found in group 1's gate, before group 4 was dispatched. | Counts corrected in both artifacts to `5/7/4/4/5/6` summing to 31, six gestures, and forty-two content rows. The specs were already right and are unchanged. | tasks 4.1; design.md → Context, Decision 4, Risks |

| WARNING | `specs/doc-conformance`, `tasks.md` | Reported by slice **D**, arriving after this file was first written, and verified here. Two bullets mandated that **`SPEC.md`'s** "the pure set is nine files" and "`tests/doc_contract.rs` carries nine further claims" each read **ten**. `SPEC.md` carries neither sentence: it has no pure-set count and no pure-view file list anywhere (its render-seam passage states the property in prose and enumerates nothing), and its § Doc-conformance checks is an uncounted nine-bullet list. Both sentences live only in `AGENTS.md`, at `:339` and `:247`. The requirement was therefore unsatisfiable — the exact unfalsifiable guard its own preceding paragraph refuses once, three lines above. It also mis-sent tasks 12.0 and 12.2 hunting `SPEC.md` for a sentence reading "eight and omitting `src/ui/palette.rs`" — text that is in the **landed `dashboard-loop` capability spec**, not in `SPEC.md`, and that this change's own delta already repairs. | Both bullets re-pointed at `AGENTS.md` alone, with the omission stated and argued rather than silent; `SPEC.md`'s share reduced to gaining a **tenth bullet** in § Doc-conformance checks, which is an addition and not a changed numeral. Tasks 12.0/12.2/12.3 re-pointed to match. | `specs/doc-conformance`, tasks 12.0, 12.2, 12.3 |
| WARNING | `proposal.md`, `design.md`, `specs/view-palette` | Reported by slice **D** and verified here. All three said the overlay "reuses **six**" palette roles while enumerating **five** — `RegionRule`, `RegionHeadingFocused`, `Strong`, `ListRow`, `ListSeparator`. Confirmed five by collecting every `Role::` token in `specs/help-overlay/spec.md`: the same five, no sixth anywhere in the change. Decision 8's whole argument is a count comparison — five reused against four that would have been minted — so the wrong numeral weakens the one claim the decision rests on. | Corrected to **five** in all three places. | `proposal.md`, `design.md` → Decision 8, `specs/view-palette` |

**Two landed drifts repaired in passing**, both inside blocks this change had to copy anyway,
under the rule design.md → Decision 9 states: `dashboard-loop`'s pure-view list read *eight*
and omitted `src/ui/palette.rs`; `quality-gates`' `TestBackend` scenario asserted an `OpenSpec`
row 0 that `pane-chrome` deleted.

## Independently Verified at HEAD

Not reported by a reviewer — run by this session while merging, and recorded because a number
nobody measured is the defect class slice D exists for:

- **116 spec scenarios, 116 matrix rows**, names one-to-one, no omission or duplicate.
- **Every geometry figure** re-derived from `content_rows = 42`: 120x40 → band 39 / interior 37
  / max offset 5 / `1-37/42`; 60x20 → 19 / 17 / 25 / `1-17/42`; 120x60 → band 44 / y 7 / rows
  7-50 / no indicator. No stale 28-binding or 39-row figure survives anywhere.
- **Both `scroll_offset` call sites** read `lines` first.
- **All 15 claimed footer widths** across seven specs, recomputed from the hint list: base 38,
  +launch 52, +focus 61, full 77, +count 54; `/add` 44 / 60 / 83 / 58; `/be` 59 / 82 / 57. Zero
  mismatches.
- **Seven other changes in flight** (`openspec list`, 2026-09-13) — the proposal's count.
- **`─` is already drawn by the crate** (`src/ui/markdown.rs:20`, `const RULE`), which is the
  band-not-box argument's premise.
- **`settings-window` overlaps three capabilities** and plans an overlay route
  (`openspec/changes/settings-window/proposal.md:69-77`).
- `make gates` green at HEAD, with `NOIO-VIEW OK: 9 pure files` and `COLWIDTH OK: … eight pure
  view files` — the two counts this change moves to ten and nine.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL and WARNING above is repaired in the owning artifact and
`openspec validate help-overlay --strict` passes.

Three slices (A, B, C) reported complete and were closed out. **Slice D — factual verification —
did not return before this file was first written.** Its checklist was not skipped: every item on
it was executed by this session instead and is recorded under *Independently Verified at HEAD*
above, including the two checks it was specifically scoped to (the footer arithmetic across all
seven specs, and the in-flight/overlap claims).

**Slice D has since reported, during implementation, and this file is amended rather than
treated as closed** — which is what the paragraph above reserved. Its report came in two passes.
The first, against the superseded `5249315`, re-raised four items this session had already
repaired (the `scroll_offset` argument order, the indicator string, the mouse sweep's missing
`SelectTab`, and the footer arithmetic) and is closed as stale on those. The second, against
`a1cbda5`, independently re-derived every footer width, every band figure, and the 31/42 counts
and found **no mismatch** — an agreeing second measurement of the numbers this file's
*Independently Verified* section had measured once.

Two of its findings were **live at `a1cbda5`** and neither had been found by any other slice or
by this session: the `SPEC.md` misattribution and the five-versus-six role count, both now
repaired and logged in the table above. Both were found the way this file's Method section says
the valuable findings are found — by running a grep rather than by re-reading a sentence. The
`SPEC.md` one is the more instructive: it sat inside the one requirement whose own prose forbids
exactly that defect, which is why re-reading did not catch it in four prior passes.

## Deferred Non-Blocking Notes

- **Justification is duplicated between design.md and the specs** — the band-versus-box
  argument, the no-arrow-glyphs argument, the `Quit`-exception argument and Decision 10 each
  appear in near-identical form in both. The specs are the merge target and get archived, so
  the duplication doubles the surface a future change must keep in agreement. Not repaired
  here: the leak runs design → spec, nothing normative lives only in design.md, and collapsing
  it during a review that has already rewritten both would risk dropping a clause. Resolution
  point: whoever rebases these deltas before archive.
- **Four full-frame scenarios are filed under `ui::help::tests`** but assert things only
  `ui::view::render` produces (the footer row, cells outside the band). They also sit outside
  `widths.sh`'s count of `#[test]`s in `src/ui/view.rs`. Left as-is: the new `helpwidths.sh`
  covers the module's own both-widths rule, and moving the tests is an implementation-time
  judgment the group-9 implementer is better placed to make. Resolution point: tasks 9.1-9.2.
- **`EXTENDED`, `TESTCOUNT`, and `OPENSPEC-UNTOUCHED`'s tracked-diff leg** remain outside
  `make gates`, as `quality-gates` already records. Unchanged by this change and not in scope.
