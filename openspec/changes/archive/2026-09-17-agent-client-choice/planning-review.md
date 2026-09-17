## Reviewed Artifacts

- `proposal.md`
- `specs/agent-launch/spec.md`, `specs/agent-prompts/spec.md`,
  `specs/dashboard-loop/spec.md`, `specs/integration-status/spec.md`,
  `specs/plugin-config/spec.md`, `specs/plugin-state/spec.md`,
  `specs/responsive-layout/spec.md` — seven delta specs, 113 scenarios
- `design.md`
- `tasks.md`

The finding pass was delegated to four `planning-reviewer` subagents dispatched simultaneously,
none of which wrote the planning package, each given one slice: (A) capability coverage, delta
fidelity, scenario quality and contradictions; (B) design completeness, test boundaries, and an
audit of whether each proposed check could fail; (C) task alignment, lifecycle discipline and
ordering; (D) factual verification of every empirical claim. This session merged the findings,
verified each against the repository itself, repaired the owning artifact, and wrote this log.
The reviewers edited nothing.

## Reviewed Against

- **This repository:** HEAD `8238737` (`chore(task-item-bodies): archive the change and fold its
  deltas`). Working tree clean apart from this change's own planning files, confirmed by
  `git status --porcelain` before and after every planted negative control.
- **Sibling repositories:** Not applicable. `~/Code/openspec-schemas` supplies the vendored
  `tdd` schema and agents, and this change alters neither.
- **Environment, measured rather than assumed:** herdr `0.9.0` at `/opt/homebrew/bin/herdr`;
  `openspec` `1.13.0` under nvm at `$HOME/.nvm/versions/node/v24.20.0/bin/openspec`.

## Gaps Found and Fixed

| # | Sev | From | Problem | Repair | Location |
|---|---|---|---|---|---|
| 1 | CRITICAL | C | Groups 3, 6 and 7 changed a signature whose call site moved two groups later. `cargo test --lib` builds the whole crate, so each group's own verification task could not have compiled. | Every signature-changing group now moves its call sites in the same group; the ordering note, which had cited one-compilation-unit as the reason *for* the broken order, was rewritten. | `tasks.md` ordering note, groups 3, 6, 7 |
| 2 | CRITICAL | C | Task 2.4 removed the `claude` default from `src/config.rs` one group before the three tests asserting it (`src/config.rs:352`, `:375`, `:413`) were rewritten, and group 2's own gate selected none of them. | 2.4 no longer touches `src/config.rs`; the literal moves in 3.3, in the same group that rewrites those assertions. | `tasks.md` 2.4, 3.3 |
| 3 | CRITICAL | A | `responsive-layout` pins the footer rule normatively and solely in terms of reachability. Splitting `a/c/s launch`'s condition without a delta would leave two live specs contradicting each other about a rendered row. | Added a `MODIFIED` delta splitting the two hints' conditions, plus one file-mode scenario. Diffed against HEAD: only the intended sentence and the new scenario moved. | `specs/responsive-layout/spec.md` (new) |
| 4 | CRITICAL | A | `dashboard-loop` pins `Launch::problems` at "at most two". The proposal explicitly claimed the capability needed no delta, having checked only whether its *code* moved. | Added a `MODIFIED` delta raising the bound; proposal corrected, with the generalisable lesson recorded. Precedent confirmed: `degraded-states` carried the same delta when it raised the bound from one to two. | `specs/dashboard-loop/spec.md` (new), `proposal.md` |
| 5 | CRITICAL | A, B | The stated bound of three was false by this change's own scenarios: resolution could contribute two, or one per malformed line plus a warning — *n*+2, so a 17-line format change meant 18 leading problem rows. | `parse`'s per-line problems are summarised into one entry at the launch boundary; resolution capped at two (one per kind, mutually exclusive members); total bound four, stated in every site that carries it. | `integration-status`, `agent-launch` (×4 sites), `dashboard-loop`, `design.md` Decision 11 |
| 6 | CRITICAL | A | `Choice::Ambiguous` returns no kind, but `agent-launch` asserted a kind is always produced and that resolution never stops a launch; the only statement otherwise was one bullet inside a wiring scenario, and `tasks.md` said "refuse nothing". | New `ADDED` requirement owning the stop — before `pane split`, one problem, `named` `None`, no pane, `in_flight` cleared — with four scenarios; the contradicting sentence and the task were fixed. | `specs/agent-launch/spec.md`, `tasks.md` 7.6 |
| 7 | CRITICAL | B | Four of the five verification commands selected **zero** tests and exited **zero**. The filter is a substring of the full test path and the tests sit under a `tests` module: `launch::decide` → 0, `launch::tests::decide` → 10; `src/ui/mod.rs` *is* module `ui`, so `ui::mod::wiring` → 0, `ui::tests::wiring` → 29. Nine `decide` rows, six prompt rows, seven wiring rows and four group gates rested on it. | Every filter corrected and re-measured. `tasks.md` now carries a baseline table and requires each gate to report a count **strictly greater** than its baseline. | `design.md` matrix + Decision 16, `tasks.md` header and all nine gates |
| 8 | CRITICAL | B | Two of the six values `Settings` threads — the recorded kind and the prompt overrides — were guarded at no tier. `WIRED`'s name list carries neither, so a root that never calls `state::recorded_kind` and passes an empty override map satisfied every gate. This is `agent-attribution`'s shipped `state_dir: None` defect in a new place. | Two wiring scenarios added, each guarded by a disagreement between two outcomes rather than a presence check, plus the tasks that write them. | `specs/agent-launch/spec.md`, `tasks.md` 9.2 |
| 9 | WARNING | D | The scenario claiming to prove the last-` (` split rule could not fail: `Integration` exposed no `status`, so both split rules agreed on every line Herdr prints, including the "path with a space" case the design named as decisive. `split_once(" (")` passed every scenario. | `status` is now a field, asserted directly, plus a synthetic `not installed (v1)` line where the two rules disagree on `installed` itself. The false justification was removed from both files. | `integration-status`, `design.md` Decision 14 + Risks |
| 10 | WARNING | C | Declaring `pub mod integration;` in group 1 left `cargo test --test doc_contract` red until group 11 — nine groups of red `make check` no task explained. | The `SPEC.md` module-map and tested-modules edit moved into group 1, so the group ends green. | `tasks.md` 1.4 |
| 11 | WARNING | C, B | `agent-prompts` :: "No production file still produces an `/opsx:` prompt" named tier doc-contract, but no task wrote that claim; the substitute was a one-off shell grep leaving no committed guard. `grep -rn opsx scripts/gates/ tests/ Makefile` → 0 matches. | New task writing the claim into `tests/doc_contract.rs`, falsified by a planted `/opsx:apply`. | `tasks.md` 11.3 |
| 12 | WARNING | C | Groups 3 and 7 changed interfaces with consumers named in design.md → Contracts and carried no contract gate; the only coverage was a catch-all several groups later. | Contract gates added to both. | `tasks.md` 3.5, 7.7 |
| 13 | WARNING | C, D | Task 11.3 pointed at `notes/gate-floors.md` as an existing root path and claimed it recorded all seven floors. `ls notes/` → no such directory; the only copies are change-local, inside two archived changes. | Path corrected to `openspec/changes/agent-client-choice/notes/gate-floors.md`, which the task now states it creates. | `tasks.md` 11.1 |
| 14 | WARNING | A | `SETTLE_BUDGET`'s 35-second rationale reasons about `agent start`'s 30-second readiness wait as the launch's dominant term; this change added a Herdr call before `pane split`. | Measured rather than argued: `herdr integration status` returns in under 10 ms (below `/usr/bin/time -p`'s resolution, 5 runs of 5), so the 5-second margin survives and the constant does not move. Recorded as an accepted, measured exposure. | `design.md` → Risks |
| 15 | WARNING | A | `proposal.md` → Impact said `src/state.rs` **records** the choice, contradicting every other artifact and the write boundary. | Corrected to reading; `src/integration.rs` added to Impact. | `proposal.md` |
| 16 | WARNING | A, D | `proposal.md` cited `openspec` 1.12.0 while design.md and `agent-prompts` cited 1.13.0, the version actually measured. | Swept all artifacts; single value 1.13.0. | `proposal.md` |
| 17 | WARNING | D | Task called the new doc-contract claim "the twelfth"; `SPEC.md` lists 13 and `AGENTS.md:289` says "thirteen". | Corrected to fourteenth, with the `AGENTS.md` count word added to the documentation group. | `tasks.md` 11.2, 13.4 |
| 18 | WARNING | D | `proposal.md` cited `README.md:85` for the `agent_kind` row; it is line 88. | Corrected. | `proposal.md` |
| 19 | SUGGESTION | C | Six behavior groups had no REFACTOR task and no statement that none was needed. | Added to all six. | `tasks.md` |
| 20 | SUGGESTION | C | Task 11.4 was labelled CHECK but wrote a test, and the group's verify did not re-run the suite it extended. | Relabelled CHANGE; 11.5 now runs `cargo test --test doc_contract`. | `tasks.md` 11.2, 11.5 |
| 21 | SUGGESTION | C | Task 6.1 referred to "the seven carried `decide` scenarios" without naming them. | All six named. | `tasks.md` 6.1 |
| 22 | SUGGESTION | C | Four tasks restated design rationale instead of citing it. | Replaced with `per design.md → Decisions N`. | `tasks.md` |
| 23 | SUGGESTION | A | The "no standalone `openspec`" assertion would misfire if the fixture change name contained `openspec`. | Fixture name pinned to `2fa-support` in the acceptance-test task. | `tasks.md` 0.2 |
| 24 | SUGGESTION | A | `PRD.md:52` names `/opsx:*` as the authoring route and no task updated it. | Documentation task added. | `tasks.md` 13.5 |
| 25 | SUGGESTION | D | Two documentation task counts were wrong: "three stale rows" (four are named) and "six sites" (the grep prints seven lines across six sites). | Both corrected with the command that produced them. | `tasks.md` 13.1, 13.2 |
| 26 | SUGGESTION | B | Tasks 4.3 and 7.6 were the same persistence check, both confirming only that design.md says "none". | Deduplicated; group 4 renumbered. | `tasks.md` |
| 27 | SUGGESTION | self | Found while verifying #7: three gate commands passed two filters positionally. `cargo test --lib config:: launch::` is not a two-filter run — it exits 1 with `error: unexpected argument`. The working form is `cargo test --lib -- config:: launch::`, measured at 69 = 24 + 45. | All three rewritten with `--`; the rule and the measurement recorded beside the baselines. | `tasks.md` header, 3.7, 6.5, 7.11, 8.6 |
| 28 | WARNING | B | The worker's behaviour on `Request::Launch` with `openspec_bin` `None` was unspecified, and task 10.2's third plant predicted a failure that would not occur: `Collaborators::file_mode` comes from `cli.is_none()` (`src/ui/mod.rs:211`), **not** from `Settings`, so hardcoding the binary to `None` leaves `decide` returning `Go` and `pane split` running. | The worker now refuses such a request before resolution and before any Herdr call, with one named problem; the plant's expected assertion set was restated to match. | `specs/agent-prompts/spec.md`, `tasks.md` 10.2 |
| 29 | WARNING | B | Test Boundaries omitted `refresh::Refresher`, which `start_collaborators` starts and which is the collaborator that actually executes the scratch `openspec` program. The Clock row said "not read", but the acceptance tier depends on `testutil::Stages`' **30-second deadline shared across every stage** (`src/lib.rs:689`) — and this change takes one wiring scenario to four non-`agent list` entries and another to five under it. | Refresh-worker row added; Clock row restated; a task added measuring both wiring scenarios' wall time against that deadline, with `ui::tests::wiring`'s measured 43.4 s for 29 tests as the reference point. | `design.md` → Test Boundaries + Risks, `tasks.md` 10.3 |
| 30 | WARNING | B | `agent-prompts` carried an unobservable THEN — "the launcher is started with `openspec_bin` `None`" cannot be asserted, because `Collaborators::launcher` is a `Box<dyn Launcher>` exposing only `request` and `drain`. | Restated behaviourally: the two runs differ at the seam (a problem row and an empty log versus a completed launch) rather than by inspecting a trait object. | `specs/agent-prompts/spec.md` |
| 31 | SUGGESTION | B | `design.md` presented the worker's below-the-`thread::spawn` placement as gate-enforced. `NOBLOCK`'s `BLOCK3_RE` matches channel receives, joins and parks — not `cli.run`, which already blocks above the spawn at `src/launch.rs:267` today. | Restated as a sound convention rather than a check, at the point the design claims it. | `design.md` → Boundaries |
| 32 | SUGGESTION | B | Task 11.2's needle set omitted `ratatui`. `LAUNCHSEAM` covers `HerdrCli`, but no `make gates` script sweeps `src/integration.rs` at all, so `ratatui` was covered by nothing. | Added to the claim's needle set. | `tasks.md` 11.2 |

| 33 | SUGGESTION | B | The repaired filters used `<module>::tests::…`, which hardcodes the very test-module name a future rename would move. | Measured alternatives and adopted a bare leaf token where it selects exactly the intended set (`decide` → 10, `wiring` → 29), kept the qualified form where no bare token isolates it (`prompt` → **9** against `launch::tests::prompt`'s **2**), and recorded that the count — not the spelling — is the mechanism, since `cargo test` exits 0 on an empty selection with no flag to change it. | `tasks.md` header and gates |
| 34 | SUGGESTION | B | Three production doc comments this change falsifies were owned by no task and read by no gate: `src/launch.rs:18`'s `Intent` comment (the one remaining `/opsx:` site), `:55-61`'s `Outcome` "at most two" bound, and `:70`'s "six arguments". | Tasks added in the groups that change each. | `tasks.md` 6.5, 7.10 |
| 35 | SUGGESTION | B | The `SETTLE_BUDGET` exposure was recorded in Risks but not in Persistence and Rollout, where the exit path belongs. | One line added there, cross-referencing Risks. | `design.md` → Persistence and Rollout |


## Verified Clean

Recorded because each is a failure mode that was actively looked for and not found:

- **Delta fidelity across all seven original `MODIFIED` blocks** (reviewer A, by splitting both
  files on `### Requirement:`, matching by title, and unified-diffing each pair): every changed
  line accounted for as an intended edit, **no silent revert of landed work**. This is the
  highest-consequence failure available here, because an archived `MODIFIED` block overwrites
  the live requirement whole. The two blocks added during repair (#3, #4) were diffed the same
  way, with the same result.
- **All 13 load-bearing factual claims CONFIRMED** (reviewer D), including every count in
  design.md → Boundaries and the three negative controls, which D reproduced independently in a
  scratch copy.
- **Kind markers**: 15 groups, 15 valid markers, evidence task first in every group.
- **No task invents a boundary the Test Boundaries table omits** (reviewer B, having walked
  tasks 0.1, 1.1, 4.x, 7.x, 9.x and 11.2); the two gaps found were omissions in the table
  itself, repaired as #29.
- **Seam compliance** (reviewer B, against `scripts/gates/noblock.sh` and
  `scripts/gates/launchseam.sh`): the new module keeps `LAUNCHSEAM`'s `ALLOWED` at five files,
  `NOBLOCK`'s Guards D and E for `src/launch.rs` survive — `fn drain`'s last occurrence stays
  above the single `thread::spawn` — and nothing blocking lands on the render path, because
  `run_request` is reachable only from `worker_body` below the spawn.
- **No PRD non-goal crossed**, and **no code path this plan adds writes inside `openspec/`** —
  `settings.toml` is read-only here, the plugin's writes stay `agent-names.toml` under the state
  directory, and a wiring scenario asserts the repository tree byte-identical after a full
  launch.
- **Artifact ratio healthy**: `tasks.md` 374 lines against `design.md` 484, so decisions are not
  being made in the checklist.
- **Verification matrix complete**: 113 rows against 113 scenarios, cross-checked
  programmatically by name after every repair, with no duplicate and no orphan.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL and WARNING above is repaired in the artifact that owns it, and
`openspec validate agent-client-choice --strict` reports the change valid.

Two of this review's findings share one root cause worth stating, because it is the thing a
future planning session here will get wrong the same way: **both missed deltas (#3, #4) were
missed by asking whether this change moves another capability's code, when the question is
whether that capability's prose already fixes the thing being changed.** It is recorded in
`proposal.md` and in design.md → Decisions 15 rather than only here.

A second, narrower lesson is recorded in design.md → Decisions 16: a prescribed command is not
evidence until someone has run it and read **how many tests it selected**. Four commands in this
plan exited zero while binding nothing.

## Deferred Non-Blocking Notes

- **`herdr agent start --kind`'s enum is not closed.** Measured by reviewer D:
  `--kind antigravity-cli` is accepted although `--help` never lists it, while `--kind notakind`
  exits 2. This strengthens rather than threatens the Non-Goal of not validating a kind against
  that list — the list is not authoritative, and `agent start`'s own failure is. No artifact
  change is owed; the Non-Goal already says the enum is not used.
- **`Outcome::problems`' rendering has no cap in `src/ui/list.rs`.** The bound is enforced by
  what the worker produces, not by the view. That is consistent with how every other problem
  vector in this crate works, and the bound is now genuinely four, so no view-side cap is added.
  Its resolution point is the scenario "Four outcome problems render as four leading rows",
  which asserts no fifth row.
- **`launch::Settings`' `NODEFAULT-UI` floor is measured after the code lands**, not now, since
  the floor is a property of the written type. Its resolution point is `tasks.md` 11.1, which
  records the measurement in the change's own `notes/gate-floors.md`.
