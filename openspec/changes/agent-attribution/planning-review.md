## Reviewed Artifacts

- `openspec/changes/agent-attribution/proposal.md`
- `openspec/changes/agent-attribution/design.md`
- `openspec/changes/agent-attribution/tasks.md`
- `openspec/changes/agent-attribution/specs/agent-attribution/spec.md` (new capability)
- `openspec/changes/agent-attribution/specs/change-rows/spec.md` (delta)
- `openspec/changes/agent-attribution/specs/responsive-layout/spec.md` (delta)
- `openspec/changes/agent-attribution/specs/dashboard-loop/spec.md` (delta)
- `openspec/changes/agent-attribution/specs/agent-poller/spec.md` (delta)
- `openspec/changes/agent-attribution/specs/list-filtering/spec.md` (delta, **added by this
  review** — see A2)

The finding pass was delegated to **four independent reviewers**, none of which wrote the
package and none of which was a fork of the planning session, sliced as
`openspec/config.yaml` → `rules.planning-review` requires: (A) capability coverage, scenario
quality, and cross-artifact contradictions; (B) design completeness, test boundaries, and
whether each proposed check could fail at all; (C) task alignment, lifecycle discipline, and
`parallel-after` independence; (D) factual verification of every empirical claim. Each wrote
its findings to a scratchpad file as it went, per this repository's `529 Overloaded` history.
They reported 3 + 2 + 1 CRITICALs, 21 WARNINGs, and 14 SUGGESTIONs between them; this session
merged and repaired them.

## Reviewed Against

- This repository HEAD: `4e673690db3f8bc4d90480570ce1f43c3b904b5a`
- Sibling repositories: **Not applicable.** This crate has no sibling; the two external
  contracts it depends on are the `openspec` CLI's JSON and Herdr 0.8.2's socket protocol,
  both verified live against the installed binaries rather than read from a spec.
- Working tree: **clean** apart from the untracked `openspec/changes/agent-attribution/`
  directory this change owns. `git status --porcelain` reports exactly one line,
  `?? openspec/changes/agent-attribution/`, before and after every planted check.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs/dashboard-loop | The MODIFIED `Dashboard` scenario dropped four landed bullets: the four proven-able-to-fail plants, the multi-line `..` control, the seven-type compile-time destructuring companion, and the `AgentStatus` exemption. Six of seven mandated destructurings stopped being required | All four reproduced, updated to eleven fields, eight types, and a fifth plant on `Attribution` | specs/dashboard-loop/spec.md, scenario "`Dashboard` has no `Default` and no site elides a field" |
| CRITICAL | specs/dashboard-loop | `ui::load`'s signature is owned by "Startup state is read from files only", which the change did not modify — after archive the specs tree would state both a two-parameter and a three-parameter `load` | That requirement added to the MODIFIED set, reproduced in full with `state_dir`, the `agent_names` initial value on both branches, and two new scenarios that give tasks 7.1's tests a home | specs/dashboard-loop/spec.md |
| CRITICAL | specs/agent-attribution | "Every empty and absent input is total" left the agent set unspecified for two of its five calls and asserted `unattributed == 0` where tier 3 requires `1`; a stub returning an empty `Attribution` satisfied four of five | All five calls given explicit inputs; call 2 now asserts `unattributed == 1` and says why the weaker form is wrong | specs/agent-attribution/spec.md |
| CRITICAL | tasks.md, design.md | `WIRED`'s new `state::read` name guards a link fifteen tests already drive, while the untested link — the value `run` passes into `Startup::state_dir` — stayed open. A shipped `state_dir: None` leaves tier 1 permanently dead with every gate green | A **leg 5** added to `WIRED`, requiring `state::state_dir(` in `run`'s own body and forbidding `state_dir: None`. Verified at planning time in four states: red at HEAD, green when wired, red under `state_dir: None`, red under a renamed `state::read` | tasks.md → Command-level checks, `WIRED.sh` replacement 5; design.md → Test Strategy matrix |
| CRITICAL | tasks.md | Group 1's acceptance test named a four-field `Startup` that group 6 added, so the crate would not compile from task 1.4 to 6.2 — and `llvm-cov --ignore-run-fail` ignores a failing run, never a failing compile, so every intervening `testcount` was unreachable rather than red | A structure-only **group 1 skeleton** (`kind: refactor`) lands the inert shapes first, on `agent-polling`'s pattern; every later group renumbered | tasks.md group 1; design.md → Test Strategy |
| CRITICAL | tasks.md | Task 6.3's `Startup` count ("three landed tests to amend") and design's ("four in `ui::tests::wiring`") were both wrong: there are **two** `Startup {` literals in the crate, one of them a shared test helper | Counts replaced with measured ones and the commands that produced them, including the 13 `ui::load(` test call sites nobody had counted | design.md → Contracts; tasks.md → Measured table and task 1.3 |
| WARNING | proposal.md | Said the badge is "dropped whole between the progress cell and the date field" — the alternative design.md → Decisions 3 explicitly rejects | Corrected to "before the progress cell", with the reason | proposal.md → What Changes |
| WARNING | specs/agent-poller | "The dashboard carries the latest snapshot and nothing renders it" was left unmodified while this change makes it false; its byte-identity scenario survived only because its fixture's agent happened to be out of scope | Requirement added to the MODIFIED set under its landed header (the merge key), the "no view reads this field" clause narrowed to one change, and the scenario's agents made explicitly out of scope with a discriminating in-scope control | specs/agent-poller/spec.md |
| WARNING | proposal.md, specs | `list-filtering` states its footer hint list exhaustively and would have been left false, silently — its scenarios pass because their fixtures carry no agents | New delta spec adding the count as the trailing hint, plus the capability listed in the proposal | specs/list-filtering/spec.md; proposal.md → Capabilities |
| WARNING | specs/change-rows | The progress cell's drop **trigger** ("when the name field would fall below one column") was deleted from both requirements, leaving no sentence stating when it drops | Restored on both, with the badge's condition added ahead of it and "never cut short" kept | specs/change-rows/spec.md |
| WARNING | specs/change-rows | "An unattributed agent badges nothing" asserted a footer its assigned `ui::list::tests::` test cannot see, and its other bullet passed with the whole feature deleted | Footer bullet moved to the capability that owns the row; a discriminating half added — the same agent renamed to a change **does** badge | specs/change-rows/spec.md |
| WARNING | specs/agent-attribution, design.md | "An unusable mapping file leaves the name tier working" required a real `state::read` inside a unit test design.md → Test Boundaries says has no filesystem | Scenario restated in terms `attribute` can be given (an empty `BTreeMap`) and renamed; the real `state::read` half moved to `dashboard-loop`'s new `load` scenarios | specs/agent-attribution/spec.md; design.md → Test Boundaries |
| WARNING | specs | No scenario covered the badge or the count against an empty change list, and nothing stated that a `Message` or `Separator` row is never badged | Both added: a bullet on the problem-row scenario, and a new `responsive-layout` scenario rendering `changes::empty_set()` with two unattributable agents | specs/change-rows/spec.md; specs/responsive-layout/spec.md |
| WARNING | design.md, tasks.md | The verification matrix named four new list tests and three new app tests where its own count table said five and four | Matrix rows added and named; the archived badged-drop scenario now names its own new test rather than a modified landed one | design.md → Test Strategy |
| WARNING | tasks.md | The 32-character agent-name cap `openspec/config.yaml` requires a test for appeared only inside the reviewer brief, never as work | A seventeenth `attribute` test and its scenario: a 54-character change name is unreachable by the name tier and reachable only through the mapping. Floors 16 → 17, total 838 → 840 | specs/agent-attribution/spec.md; tasks.md group 3 |
| WARNING | design.md, tasks.md | Nine "landed test, extended" rows were verified only by an aggregate `testcount` floor the **new** tests satisfy on their own — skipping every extension stayed green | New `EXTENDED` check: eight `<file>:<test fn>:<token>` pairs, span-isolated. Verified red on all eight at HEAD, green against already-present tokens, red on a neighbouring-test token, and both guards fired | tasks.md → Command-level checks; task 6.4, 9.1 |
| WARNING | tasks.md | Five list-test and three view-test extensions were scheduled **after** their group's GREEN task, so they were never observed red | Moved into each group's RED task, with an explicit CHECK confirming the failures are the missing behaviour | tasks.md tasks 5.1, 5.2, 6.1, 6.2, 7.1, 7.3 |
| WARNING | tasks.md | Task 7.2's "no two plants fail the *same* assertion" is unsatisfiable — three of the six necessarily fail the footer assertion | Restated as the identical *set* of assertions, naming the badge assertions as what separates them | tasks.md task 8.2; design.md → Test Strategy |
| WARNING | design.md | `openspec/config.yaml` → `rules.design` requires an explicit confirmation that no process spawn was added outside `cli`, and no matrix row ran `NOSPAWN-GREP` | Both added, plus the parallel statement that no new view is added | design.md → Boundaries; tasks.md task 3.4 |
| WARNING | tasks.md | `testcount --lib ui::list::tests::a_badged 1` matches four functions; several single-test floors were satisfiable by any one of them | Every single-test filter written out in full, and the reason stated once at the top of the file | tasks.md throughout |
| WARNING | design.md, tasks.md | Two matrix commands carried `SCAN_MIN=<measured>` placeholders and task 2.4 resolved one to a vacuous `SCAN_MIN=1` | Both measured and written in: **165** for the `src/ui/app.rs` run, **86** for the `src/agents.rs` run, each with the command that produced it | design.md → Test Strategy; tasks.md → Measured table |
| WARNING | tasks.md | The group-ordering audit covered groups 2–6 only, never examined group 9 (documentation), and its premise "each writes a different file" was contradicted by task 3.2 reaching group 6's file | Audit rewritten: the shared-file exception recorded, and group 10 marked `parallel-after: 0` with its three criteria stated | tasks.md → Group ordering |
| WARNING | tasks.md | Group 9 (documentation) was `operational` with no CHECK task, so its VERIFY signed off edits nothing had inspected | A 10.1 CHECK greps each target passage first and fails when one is not where the task says | tasks.md task 10.1 |
| WARNING | tasks.md | The `WIDTHS` / `LISTWIDTHS` narrative was wrong in both halves: `detail-view` landed 67 (not 72), and `tasks-tab` and `live-refresh` both passed explicit floors — only `agent-polling` ran them bare | Narrative corrected to name `live-refresh`'s 81 and `agent-polling`'s bare invocation; the new floors (88, 25) were already right | tasks.md → Command-level checks; design.md → Test Strategy |
| WARNING | tasks.md | Task 0.6's second half ran "once task 6.4 has landed `state::read`" — a group-0 VERIFY signing off work six groups later, and the wrong task number besides | Split: 0.6 covers `NOIO-VIEW` alone; `WIRED`'s falsifiability moved to task 1.4, where the wiring first exists | tasks.md tasks 0.6, 1.4 |
| SUGGESTION | specs/change-rows | Interior column indices were 0-based while buffer positions in the same scenarios were 1-based | Base stated once, in full | specs/change-rows/spec.md |
| SUGGESTION | specs/agent-poller | "`ui::run` SHALL be split into three" while the block below declared four items | Reworded to "into three, beside the `load` `dashboard-loop` already owns" | specs/agent-poller/spec.md |
| SUGGESTION | specs/agent-attribution | "no fourth source of evidence SHALL be consulted" contradicted the repository scope, which decides an agent's fate before any tier | Reworded: scope places the agent, then three tiers attribute it | specs/agent-attribution/spec.md |
| SUGGESTION | specs | `agent-poller` promises the agent column is hidden on `!reachable`, and this design derives everything from `agents.agents` being empty | Stated explicitly in the modified requirement: emptiness is the mechanism, `reachable` is `agent-launch`'s to read | specs/agent-poller/spec.md |
| SUGGESTION | design.md | One test function (`the_count_survives_a_filter`) is assigned to two scenarios with different fixtures, under a floor of 1 | Both rows now say so and name the function in full | design.md → Test Strategy |
| SUGGESTION | specs/change-rows | A JSON-tier fact ("`agent_status` was a string this crate does not recognise") stated at the view tier, where the status is already decoded | Restated as `AgentStatus::Unknown`, with the forward-compatibility reason kept | specs/change-rows/spec.md |
| SUGGESTION | proposal.md | "`agent-polling` retired the claim in its design" — it retired it in Boundaries and repeated it verbatim in its own Decisions 8 | Corrected; the shipped copy is named as the one this change deletes | proposal.md → Impact |
| SUGGESTION | design.md | `render_footer`'s signature changes and Contracts did not say so | Added, marked private and named for completeness | design.md → Contracts |
| SUGGESTION | tasks.md | "moves the invocation floor of two more" understated what the later section lists | Rewritten to state exactly what is added, edited, re-floored, and left alone | tasks.md → How to read this file |

**Accepted rather than repaired**, each with its reason:

- *(SUGGESTION, reviewer B)* The `changes.problems` / `refresh.problems`-empty bullet in
  "An unreachable socket yields no badge, no count, and no problem" cannot fail, because
  `attribution()` takes `&self` and could not mutate them. **Kept**: it documents the
  invariant `agent-polling` had to state in prose, and the scenario's discriminating half —
  the buffer being byte-identical with `problem` set and with it `None` — sits beside it.
- *(SUGGESTION, reviewer B)* The new `src/state.rs` positive control in `WIRED` was described
  but never planted against. **Repaired in passing rather than accepted**: it is now one of the
  four `WIRED` states measured at planning time, and it fires.
- *(SUGGESTION, reviewer D)* `agent-polling`'s own design still repeats the retired
  `too_many_arguments` claim in its Decisions 8. **Not corrected here**: it is an archived
  change's artifact, and this repository does not edit archived changes. The shipped source
  copy is the one that matters and this change deletes it.

## No Remaining Implementation-Blocking Gaps

**None remain.** Every CRITICAL from all four slices is repaired in the artifact that owns it,
every WARNING is repaired or accepted with a reason above, and
`openspec validate agent-attribution --strict` is clean. All 64 spec scenarios have a row in
design.md's verification matrix — checked mechanically, not by reading — and every row names a
tier, its collaborators, and a command. No unresolved decision requires user input.

Every check this plan relies on was run at planning time against the real tree and its exit
status recorded: `EXTENDED` and `WIRED` are red at HEAD for the behaviour they guard;
`NOIO-VIEW`, `NOLIT-CHANGE`, and `OPENSPEC-UNTOUCHED` are green with a planted violation
observed red and removed; `LISTWIDTHS`, `WIDTHS`, and both `NODEFAULT-UI` invocations are red
at their raised floors and green at the measured ones.

## Deferred Non-Blocking Notes

- **Linked worktrees are out of scope**, and the resolution point is recorded: design.md →
  Decisions 4 states the two rejected fixes, and tasks.md task 10.4 writes the limitation into
  `SPEC.md` → Degraded states so a later change can pick it up deliberately.
- **`degraded-states` inherits a column collision.** It plans a per-change problem indicator in
  the third column on an archived row, which this change now uses for the badge. Recorded in
  design.md → Risks and written into `SPEC.md` → List view by task 10.3, rather than into
  `openspec/IMPLEMENTATION-ORDER.md`, which nothing but `openspec archive` may write.
- **The crate's single vestigial `#[allow(clippy::too_many_arguments)]`** on
  `changes::build_change` is still there. `HANDOFF.md` asks the next change that touches
  `src/changes.rs` to remove it; this change does not touch that file, so it stays deferred.
- **`agent-polling` invoked `WIDTHS`, `LISTWIDTHS`, `MDWIDTHS`, `TASKWIDTHS`, and
  `DETAILWIDTHS` bare** in its final gate pass, so all five ran at their block defaults rather
  than their set floors. This change repairs the two it touches by passing `WIDTHS_MIN=88` and
  `LIST_MIN=25` explicitly; the other three are left for the change that next moves them, and
  are noted here so the pattern is visible rather than rediscovered.
