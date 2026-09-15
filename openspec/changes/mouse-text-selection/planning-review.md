## Reviewed Artifacts

`proposal.md`, `design.md`, `tasks.md`, and all nine delta specs — `specs/text-selection`,
`specs/mouse-input`, `specs/dashboard-loop`, `specs/binding-inventory`, `specs/help-overlay`,
`specs/view-palette`, `specs/terminal-lifecycle`, `specs/list-selection`,
`specs/doc-conformance` — plus `notes/measurements.md` as the evidence they rest on.

Reviewed by four agents, one slice each, none of which wrote the package: (A) capability
coverage and cross-artifact contradictions, (B) design completeness, test boundaries and
falsifiability, (C) task alignment and lifecycle discipline, (D) factual verification of every
empirical claim.

## Reviewed Against

- This repository HEAD: `8659331` at review time; refreshed during apply at `6513310`
- Sibling repository HEAD: Not applicable
- Working tree: clean

Slice D noted HEAD moved mid-review as slices A–C were being applied, and re-verified against
the later commit; `src/`, `tests/`, `SPEC.md`, `AGENTS.md`, `README.md`, `Makefile` and
`scripts/` are byte-identical across that range, so every code-side count holds for both.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md, tasks.md | `run_loop` takes no `TerminalOps` handle — the guard is built in `ui::run` and never travels inward — so "call `write_clipboard` on completion" had **no reachable call site** | Thread `ClipboardWriter<'a> = &'a dyn Fn(&str) -> Result<(), String>` into `run_loop` beside `ArtifactReader`, which already solves this problem this way | design.md → Decision 11, Boundaries, Test Boundaries; tasks 7.2; `specs/text-selection` |
| CRITICAL | design.md, specs | A failed clipboard write had no durable home; all five `!`-marked lists are replaced wholesale by their own producers, and the sixteen-field pin forbade a dedicated field | `Selection::problem: Option<String>`, created and cleared exactly when the reason becomes and stops being true; rendered as a **detail-region** row | design.md → Decision 12; `specs/text-selection`, `specs/dashboard-loop`, `specs/terminal-lifecycle` |
| CRITICAL | specs/mouse-input | The re-`ADDED` requirement body still **mandated** `Target::DetailLine` — resolution table, `Target` block, two `apply` arms — contradicting its own scenarios | Purged; the table row now produces `Action::Select`, and `Target` carries one variant for this, not two | `specs/mouse-input` |
| CRITICAL | tasks.md | Group 5 missed four hand-enumerations of `Action` in `src/ui/app.rs`'s test module plus `doc_contract`'s `expected_closed` set — the deadlock class recurring | Added task 5.5b naming each site and a check that prints 2 at HEAD and must print 0 | tasks 5.5, 5.5b |
| CRITICAL | tasks.md | Task 5.6 named the wrong check: `compare_key_atoms` **skips** the `Mouse` group, and `README.md` has no gesture table, so the instructed edit would fail in the opposite direction | Restated against `mouse_bindings_match_spec_md` and `SPEC.md`'s `\| Gesture \| Action \|` table alone | tasks 5.6 |
| CRITICAL | tasks.md | `NODEFAULT-UI` sweeps only the types its `Makefile` recipe names; `Selection` was on none, so task 4.4's claim was false and the gate proved nothing | Group 4 now edits `Makefile:46`, re-measures `SCAN_MIN`, and plants a `..Default::default()` to prove the gate fires | tasks 4.4, 4.5 |
| CRITICAL | specs/mouse-input | A paragraph carried from `foldable-spec-sections` said the mouse-table doc leg "cannot fail for the two new rows" — true there, false here, and contradicting this delta's own scenario twelve lines below | Inverted, with the reason recorded: `Action::Select` is not in the table, so the leg goes red until `SPEC.md` names it | `specs/mouse-input` |
| CRITICAL | specs/dashboard-loop | `Action` is pinned at exactly twenty-four **with every variant enumerated**; only the motion and field-count requirements had been modified | `MODIFIED` block: twenty-five, `Select` enumerated, the running tally extended, plus a scenario proving no key reaches it | `specs/dashboard-loop` |
| CRITICAL | specs/view-palette | The per-role modifier table is exhaustive over all thirty roles and bound by a scenario; `Selected` had no row | `MODIFIED` block adding `\| `Selected` \| `REVERSED` \|` | `specs/view-palette` |
| CRITICAL | specs/view-palette | The uncoloured-role set is asserted **by name**, and an uncoloured `Selected` lands in it — the live test would have gone red with no spec authorising it | `MODIFIED` block naming `Selected` in that set, with the non-collision against `DetailSectionSelected` stated | `specs/view-palette` |
| CRITICAL | specs/help-overlay | The scroll requirement carries ten `42`/`44` literals and two clamp values, none advanced | `MODIFIED` block with every literal re-derived to 43/45/6/26 | `specs/help-overlay` |
| CRITICAL | specs/binding-inventory | The closed-overlay mouse sweep asserts a set of **seven** names by equality; `Select` makes it eight | Scenario corrected to eight, naming `Select` | `specs/binding-inventory` |
| WARNING | specs (several) | Counts stale inside blocks copied verbatim: `18 + 7 = 24` (it is 25), capitalised `Seventeen` missed by a lowercase replace, the overlay's inert count in prose and in a scenario's enumerated list, and `Binding`'s "thirty-one `'static` literals" — a count belonging to `INVENTORY` | All corrected; a case-insensitive sweep for every count word this change moves now runs over every delta | `specs/dashboard-loop`, `specs/help-overlay` |
| WARNING | tasks.md | Task 5.5b's own check printed 1, not the 2 it claimed — the assertion is wrapped across four lines | Matched on the message text instead; re-run and confirmed 2 | tasks 5.5b |
| CRITICAL | specs/mouse-input | Found **during apply**, group 5. The re-`ADDED` requirement's scenario "A click on a task group's header folds that group" ended with a clause carried from the pre-selection requirement: a headless task file's two presses "both return `Action::Ignore`". That contradicts design.md → Decision 8, this same delta's `specs/text-selection` scenario "A non-foldable tab's content is selectable too", and task 5.1's own named RED test. A headless tracked-tasks file still draws real content rows, so a uniform resolver returns `Action::Select` there | Clause corrected to `Action::Select` at its arming phase, with the reason and the carry recorded inline. Implementation had already followed Decision 8, so no code moved | `specs/mouse-input` |

## No Remaining Implementation-Blocking Gaps

None remain. `openspec validate mouse-text-selection --strict` is clean, every count in the
package was derived by running a command rather than by reading, and each of the four review
slices' findings has been applied to the artifact that owns it.

Two classes of defect were found in **both** review rounds and are recorded here so the next
change in this repository can look for them directly:

1. **A count in a name is a merge key.** `openspec validate --strict` refuses a `MODIFIED`
   block that renames a scenario, so a numeral in a *scenario* header costs a
   `REMOVED`+`ADDED` exactly as one in a requirement header does. Two requirements were
   re-stated with the numeral dropped from the name so the next change pays a `MODIFIED`.
2. **A `MODIFIED` block carries its whole body**, so every count inside it is that change's
   responsibility — including counts belonging to a different capability entirely, which is
   how `Binding`'s literal count went stale inside `dashboard-loop`.

## Deferred Non-Blocking Notes

- **No auto-scroll when a drag runs off the content area's edge.** Stated as a non-goal in
  `proposal.md` and as Decision 10 in `design.md`; the resolution point is a follow-up change,
  and Copilot CLI's own behaviour here is unmeasured so copying it would be guessing.
- **The `?1002`/`?1015` modes remain enabled and unused.** Measured to buy nothing for
  selection. `?1003` is now load-bearing and must not be removed; the other two are
  dead-cost removal for a separate change with its own argument.
- **The `Shift`+drag bypass is documented only for Ghostty on macOS.** iTerm2, Terminal.app
  and the Linux terminals are unmeasured, and `SPEC.md` is required to say so rather than
  generalising from one terminal.
