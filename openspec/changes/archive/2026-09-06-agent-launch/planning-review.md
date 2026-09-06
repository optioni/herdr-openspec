## Reviewed Artifacts

- `openspec/changes/agent-launch/proposal.md`
- `openspec/changes/agent-launch/design.md`
- `openspec/changes/agent-launch/tasks.md`
- `openspec/changes/agent-launch/specs/agent-launch/spec.md` (new capability)
- `openspec/changes/agent-launch/specs/agent-attribution/spec.md` (delta)
- `openspec/changes/agent-launch/specs/agent-poller/spec.md` (delta)
- `openspec/changes/agent-launch/specs/change-rows/spec.md` (delta)
- `openspec/changes/agent-launch/specs/dashboard-loop/spec.md` (delta)
- `openspec/changes/agent-launch/specs/list-filtering/spec.md` (delta)
- `openspec/changes/agent-launch/specs/live-updates/spec.md` (delta)
- `openspec/changes/agent-launch/specs/responsive-layout/spec.md` (delta)
- `openspec/changes/agent-launch/specs/subprocess-seam/spec.md` (delta)
- `openspec/changes/agent-launch/specs/tasks-checklist/spec.md` (delta)

The finding pass was delegated to **four independent reviewers**, none of which wrote the
package and none of which was a fork of the planning session, sliced as
`openspec/config.yaml` → `rules.planning-review` requires: (A) capability coverage, scenario
quality, and cross-artifact contradictions; (B) design completeness, test boundaries, and
whether each proposed check could fail at all; (C) task alignment, lifecycle discipline, and
`parallel-after` independence; (D) factual verification of every empirical claim, run against
the tree and against the installed Herdr 0.8.2. Each wrote its findings to a scratchpad file as
it went, per this repository's `529 Overloaded` history. They reported 2 + 1 + 1 + 1 CRITICALs,
4 + 6 + 7 + 3 WARNINGs, and 6 + 6 + 4 + 4 SUGGESTIONs between them; this session merged, checked,
and repaired them.

Two check-authoring passes were also delegated, before and during the review: one wrote and
proved `LAUNCHSEAM` and the `WIRED` and `NOIO-VIEW` edits against a scratch tree, and one wrote
and proved the `NOBLOCK` edits the review found missing.

## Reviewed Against

- This repository HEAD: `df36bcbb4e32f0df66a6952f7c77a5e650f51c6c`
- Sibling repositories: **Not applicable.** This crate has no sibling; the two external
  contracts it depends on are the `openspec` CLI's JSON and Herdr 0.8.2's socket protocol, and
  every Herdr claim in this package was verified live against the installed binary rather than
  read from a spec.
- Working tree: **clean** apart from the untracked `openspec/changes/agent-launch/` directory
  this change owns. `git status --porcelain` reports exactly one line,
  `?? openspec/changes/agent-launch/`, before and after every planted check.
- **Herdr safety.** Live probing created two panes and one agent in total, all closed within the
  same minute; `herdr pane list` and `herdr agent list` afterwards were byte-identical to before
  and focus was restored to the pane it started on. Every error-path probe used a nonexistent
  pane or agent target. No agent was left running.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | specs (missing delta requirement) | `openspec/specs/live-updates/spec.md` → "`r` forces a full refresh" states the `Action` count as **thirteen** in four places, including an executable claim ("an array the test builds through an exhaustive `match`"), and the change never modified it. The count was moved in `dashboard-loop` and `tasks-checklist` and missed here; `openspec validate --strict` cannot see it, because nothing structurally links the three | The requirement added to the MODIFIED set, reproduced whole with thirteen → **seventeen** in all four places, "a fourteenth variant" → "an eighteenth", and the sweep's fixture required to carry a reachable socket and a non-empty list so the four new arms are not exercised vacuously | specs/live-updates/spec.md |
| CRITICAL | specs/agent-launch | The worked example asserted `2FA_Support!` derives to `c-2fa-support`. Verified by compiling `src/state.rs`'s derivation verbatim and running it: it derives to **`c-2fa_support`** — step 2 replaces only characters *outside* `[a-z0-9_-]`, and `_` is inside it. The document also claimed two different changes derive to one name. An implementer chasing the red test could have "fixed" the landed, archived `state::agent_name` | Both literals corrected, and the scenario now states *why* the underscore survives and that `2fa-support` (hyphen) is a different change with a different derived name. Kept as the example deliberately: it is the derivation's one non-obvious step and now has a test | specs/agent-launch/spec.md, scenario "A legal-but-derived name is recorded too" |
| CRITICAL | tasks.md, design.md | `NOBLOCK` was named as covering `src/launch.rs` but was **never edited**: its seam list is hardcoded, the preamble said the change edits two blocks, and task 0.5 applied two. Proven: a `Launcher::drain` that blocks the render path with `recv_timeout` leaves `NOBLOCK`, `LAUNCHSEAM`, `WATCHSEAM`, `WIRED`, and `NOIO-VIEW` **all green** — the only remaining net would hang rather than fail | Four `NOBLOCK` replacements written out in full (Guard D's list, an existence guard, a fourth leg-3 arm mirroring the `src/agents.rs` one with Guard E on `tail -1`, the OK line), proven at planning time against six plants with the unedited block green under four of them; preamble corrected to "edits three"; task 0.5 and 0.4 amended | tasks.md → Command-level checks, tasks 0.4, 0.5; design.md → Test Strategy |
| CRITICAL | tasks.md | Group 14 was marked `parallel-after: 0` and its own ordering rationale said it "edits no source file", while task 14.6 edited `src/cli.rs` and `src/ui/app.rs` — group 11's and group 7's files — and wrote a number ("seventeen") that group 7 makes true. All three parallelism criteria failed at once, and a wrongly marked pair costs two agents in one tree | 14.6 split and moved: the `src/ui/app.rs` correction became task **7.4b**, beside the edit that moves the count; the `src/cli.rs` correction folded into task **11.2**. Task 14.1's grep list trimmed to the three Markdown files, and the ordering section now states where the two doc comments went and why | tasks.md group 14, tasks 7.4b and 11.2, → Group ordering |
| WARNING | specs (missing delta requirement) | `openspec/specs/subprocess-seam/spec.md` → "Two traits carry the two programs" holds "one `RealHerdrCli` **four more** in `agent-launch` … all **eight** failures". The correction was written into a *different* requirement of the same capability, so the spec tree would have contradicted itself two requirements apart | The requirement added to the MODIFIED set, reproduced whole with **five** and **nine**, plus a new scenario asserting a four-element argument vector distinguishes one failure from another | specs/subprocess-seam/spec.md |
| WARNING | specs/agent-launch | Nothing asserted `Outcome::named` is `None` when a launch failed **before** `agent start`. A worker setting it unconditionally passed every scenario — and the loop inserts `named` into `Dashboard::agent_names`, so it would have badged a change with whatever agent the next poll returned: exactly the guess `agent-attribution` refuses | `named is None` bullets added to the failed-split, unusable-payload, failed-start, and focus scenarios; the `Some`-and-`Some` case made explicit on the failed-prompt scenario; and a **seventh plant** added to design.md's table, caught at the unit tier alone because the acceptance run's launch succeeds either way | specs/agent-launch/spec.md; design.md → Test Strategy; tasks.md 12.2 |
| WARNING | specs/agent-launch, tasks.md | The two wiring scenarios counted **absolute** entries in a log the one-second poller also writes. At `[agent list, agent list, pane split, agent start]` the "four entries" predicate fires before `agent prompt` is issued; the second scenario was off by one on its face and asserted "the last entry", which the next poll overwrites. The same timing hazard the plan rejects elsewhere | Every predicate and every ordering assertion now counts **non-`agent list`** entries only, and says why. Task 2.2 gained a "Red when" naming the race; task 2.3's "fifth entry" became "fourth non-`agent list`" | specs/agent-launch/spec.md; design.md → Test Strategy; tasks.md 2.2, 2.3 |
| WARNING | design.md, tasks.md | `WIRED` leg 6 half (i) is a **presence** check. Proven: hardcoding `"codex"` while naming `config.agent_kind` once elsewhere leaves `WIRED` green — and the single acceptance test asserting `--kind codex` stays green too, so nothing caught it | The two acceptance runs now use **different** configured kinds (`codex` and `gemini`), so a hardcoded value fails one of them. Stated in the scenario, in design.md's assertion list, and in task 2.3 | specs/agent-launch/spec.md; design.md → Test Strategy; tasks.md 2.3 |
| WARNING | tasks.md, design.md | The `EXTENDED` pair list held **35** new pairs while three artifacts said 34, 34, and 31, and the OK-line assertion said 42 against a real 43. A stale count invites deleting a pair to match it — and `EXTENDED` is the only red-capable guard on a modified test | Recounted from the list. Three further pairs added for extensions that had a design row and no guard (`timeouts_are_not_events`, `the_prompt_replaces_the_hints_while_filtering`, `an_accepted_query_leads_the_hint_list`), giving **38** new and **46** total. Every count corrected, and task 7.1's "seven landed tests" → eight, 8.1's three → four, 10.1's ten → twelve. Re-run against `main`: exactly 38 red, the 8 landed green | tasks.md → `EXTENDED` pair list, tasks 7.1, 8.1, 10.1, 13.1; design.md → Test Strategy |
| WARNING | design.md | One matrix row asked `ui::app::tests::a_refused_launch_records_the_reason` to assert "the recording `HerdrCli` saw nothing". Planted: that turns `NOCLI-SHELL` **and** `LAUNCHSEAM` leg 3 red, and it contradicted the row's own "Collaborators: none" | Restated: the app-tier test asserts `launch.pending` stays `None`; the "no Herdr call" claim is proved at the seam by `launch::tests::` | design.md → Test Strategy |
| WARNING | tasks.md, design.md | `READSEAM` (default `UI_MIN=7`, realized 10) and `MDSEAM` (default `MIN=16`, realized 22) were invoked **bare** — the exact defect the plan says it repaired for the widths gates. Task 13.3's audit was likewise scoped to widths gates only | Both given explicit floors (`UI_MIN=10`, `MIN=23`) in the floors list, in task 13.1, and as their own matrix rows; task 13.3's audit widened to every gate that reads a floor, with the scripts' own `${VAR:-N}` lines as the comparison | tasks.md → floors list, tasks 13.1, 13.3; design.md → Test Strategy |
| WARNING | tasks.md | Task 5.1's "Red when: any of them calls `thread::sleep` … `NOSLEEP` is what catches it" is false. Proven: a deadline-bounded sleep in `src/launch.rs` leaves all three legs green, because `SLEEP_MIN` is a lower bound and leg 2 sweeps `src/ui/` only | Restated honestly as a review point rather than a gate, naming what `NOSLEEP` does and does not see, and why a sleep there would be an elapsed-time assertion about a worker thread | tasks.md task 5.1 |
| WARNING | tasks.md | Task 1.5 required `WIRED` green at group 1, but task 1.3's enumeration never had `start_collaborators` call `launch::start` or read `config.agent_kind` — which task 11.2 read as first doing. An implementer following 1.3 literally would find 1.5 red with no instruction to fix it | 1.3 now has the skeleton call `launch::start` with `config.agent_kind` unconditionally; 11.2 reworded to name only what it adds (the state directory, the `launch::none()` arm) | tasks.md tasks 1.3, 11.2 |
| WARNING | tasks.md | The `NODEFAULT-UI` app-run floor was promised a raise "in task 7.4", which contained no re-measurement, and task 7.5 pinned it at the pre-change 174 — a floor a twelfth field necessarily clears | 7.4 gained the `SCAN_MIN=9999` re-measurement with a "Red when" requiring the realized figure to exceed 174; 7.5 and design.md now cite that measurement | tasks.md tasks 7.4, 7.5; design.md → Test Strategy |
| WARNING | design.md, tasks.md, proposal.md | The `GRAPH-SNAP` failure was described — following `HANDOFF.md` — as a stale `tests/fixtures/build-graph.txt`. Verified: the snapshot **was** regenerated (commit `574b87d`; it holds `notify v8.2.0`, `fsevent-sys`, `inotify`), the script passes its snapshot diff, and it fails four legs later on a hardcoded macOS/Linux platform assertion. The repair `HANDOFF.md` assigns to this change would therefore fix `DEPS` only | The real reason and the real first line recorded in all three artifacts, the decision to decline the repair moved into the proposal where a reader approves scope, and the corrected scope handed on. See "Known-red inherited conditions" below | proposal.md → Impact; design.md → Non-Goals; tasks.md tasks 0.4, 13.1 |
| WARNING | design.md, tasks.md | `Attribution {` was counted as "1 production, 4 test". Verified: 5 grep hits, of which **3 are not literals** (the `struct` declaration and two `-> Attribution {` return types), and both real literals are in `src/agents.rs`'s production slice — **0** test literals, since `#[cfg(test)]` starts below every hit. A task phrased as "4 test literals updated" could never go green | Both counts corrected, with what each of the five hits actually is | design.md → Contracts; tasks.md → Measured at planning time |
| WARNING | specs/agent-launch | The collision scenario's WHEN set up "two changes whose derived names collide" and then exercised one — a repeat press. The cross-change collision needs a different answer: the mapping badges whichever change launched last, so `g` on the other one is inert and the refusal's "press `g`" is unactionable | Setup narrowed to what it tests; a bullet added confirming `g` **does** work for the repeat-press case, and a second stating the cross-change collision is out of scope, why, and what happens | specs/agent-launch/spec.md |
| WARNING | proposal.md, design.md | `min_herdr_version` was kept at `0.7.0` on the argument that "every subcommand predates 0.8.2" — which is about subcommand *existence*, while the change depends on two 0.8.2-measured *shapes* it presents as corrections to prior belief. A Herdr returning a bare pane id would fail every launch at call 1 | Stated explicitly in both, with the reason the floor stays: the non-envelope payload is handled as a **degraded state** (`pane_id` returns `Err`, the launch stops before an agent is started, the reason is rendered) rather than by a manifest change, which would be a second BREAKING claim on an unmeasured hypothesis. Task 14.3 gained a seventh Degraded-states row for it | proposal.md → Impact; design.md → Boundaries; tasks.md task 14.3 |
| WARNING | tasks.md | Group 2 (`kind: behavior`) ordered a `CHANGE` task before its `RED` tasks. The work is test-side harness, so the intent was right and the label — which the apply flow reads — was wrong | 2.1 relabelled `RED`, with its test-only scope stated | tasks.md task 2.1 |
| WARNING | tasks.md | Eight behavior groups had neither a REFACTOR task nor the "no refactor was needed" statement the schema requires in its place | The clause added to all eight VERIFY tasks, naming why in each case; group 7's is a genuine look at four `apply` arms that gather the same five values | tasks.md tasks 4.5, 5.4, 6.3, 7.5, 8.3, 9.3, 10.4, 11.4 |
| SUGGESTION | tasks.md | Tasks 14.2–14.5 carried no lifecycle verb, and 14.4 no audience | All four prefixed `CHANGE:`; 14.4 given its audience | tasks.md group 14 |
| SUGGESTION | tasks.md | Group 12 was `kind: behavior` with no RED and no GREEN — it confirms and plants, adding no behaviour | Reclassified `operational`. Group 2 stays `behavior`, which is where the RED lives | tasks.md group 12 |
| SUGGESTION | specs/agent-launch, design.md | "An archived change launches on the same terms" asserted a *prompt* its assigned unit-tier test (`Dashboard::apply`) cannot observe | Restated in terms that test sees — `launch.pending` field-for-field identical to the active-change case — with the prompt claim moved to a second bullet naming the worker tier | specs/agent-launch/spec.md |
| SUGGESTION | specs/agent-launch | The `Path::to_string_lossy` SHALL had no scenario, so `to_str().ok_or(...)` — the natural alternative — would have failed no test | A bullet added to the three-calls scenario driving a non-UTF-8 repository root | specs/agent-launch/spec.md |
| SUGGESTION | design.md | Two of `rules.specs`' six mandated boundary states — a missing artifact file, a schema the CLI rejects — were neither covered nor declared inapplicable | Recorded in "What this change does not touch": the launch path reads no artifact and consults no schema, so both are unchanged from `detail-view` and `schema-cli-fallback` | design.md → Boundaries |
| SUGGESTION | design.md | Two `SCAN_MIN=<measured>` placeholders shipped unresolved, and three matrix rows had no entry (`launch::tests::argv::`, `::focus::`, `ui::tests::load::`) | Placeholders replaced with the task that measures each; three rows added | design.md → Test Strategy |
| SUGGESTION | design.md | Test Boundaries omitted the gated fake's release channel and `testutil::canonical` / the macOS `/private/var` symlink, both of which tasks reference | Two rows added | design.md → Test Boundaries |
| SUGGESTION | tasks.md | The `NOSLEEP / WATCHSEAM set` row gave one command and one value for two gates with different searched sets (`WATCHSEAM` excludes its own subject: 24 → 25, not 25 → 26) | Row split | tasks.md → Measured at planning time |
| SUGGESTION | tasks.md | Tasks 3.2 and 3.3 did not name their test functions while the design matrix did, so task 16.1's "every single-test filter names its function in full" could not be checked against them | The five names copied in | tasks.md tasks 3.2, 3.3 |
| SUGGESTION | tasks.md | The "Herdr `agent focus` targets" row's command column ran `herdr agent get`, and no row recorded that a Herdr **usage** error is plain text rather than JSON | Row relabelled with the `agent focus` probes actually run (against the already-focused pane, so focus did not move); a row added for `--kind nosuchkind` → exit 2, plain text; and a paragraph added to the parsing requirement forbidding any assumption that Herdr's stderr is JSON | tasks.md → Measured at planning time; specs/agent-launch/spec.md |
| SUGGESTION | proposal.md | "`src/cli.rs`'s **module** comment" — it is the `CliError` doc comment; and the same sentence's "eight failures" becomes nine, which no task said | Corrected, and task 11.2 now names both halves | proposal.md → Impact; tasks.md task 11.2 |
| SUGGESTION | design.md, tasks.md | The 907 breakdown was written in two different addend orders, neither matching the floors table | Aligned to the table's row order in both | design.md → Test Strategy; tasks.md |

## Findings examined and **not** taken

- **"`tasks.md` cites a command that does not produce the evidence" (reviewer D, A-1)** — the
  claim was that `herdr agent start --help` prints the top-level help and lists no kinds.
  Re-run on the reference machine, it prints the per-leaf help,
  `Usage: herdr agent start <NAME> --kind <KIND> --pane <ID>`, with
  `--kind <KIND> … [possible values: pi, claude, codex, …]`. The recorded command reproduces;
  the row stands. Recorded here so the disagreement is visible rather than silently resolved.
- **"Eleven `testcount` floors are already met by `main`" (reviewer B, B9)** — accepted as
  correct and deliberate. Those are the floors on *landed* filters that this change does not
  grow (`ui::tests::live:: 3`, `cli::tests::herdr_program 1`, and similar); they guard against a
  regression that deletes a landed test, not against a missing new one. Every floor on a filter
  this change **does** grow is above the count on `main`, which task 16.1 re-checks.
- **"`NOIO-VIEW` misses a path split across a line break" (reviewer B, B11)** — real and
  measured, but it is a property of every line-oriented grep gate in this repository and fixing
  it in one block would leave the other ten inconsistent. Reachability is low (rustfmt does not
  break `crate::state::record` across lines) and the compile-time consequence of the split call
  is the same. Consciously accepted; not this change's to fix.

## Known-red inherited conditions

Two command-level checks are **red on `main` at the base commit** and this change deliberately
leaves them so. Both are outside `make check`, so neither blocks any gate in this plan. Task
0.4 records each one's exact text at `BASE` and task 13.1 requires the text to be **byte-identical**
at the end, which is what distinguishes an inherited failure from one this change caused.

**`DEPS`** — red since `live-refresh` added the `notify` dependency without adding it to the
script's want-list. Leg 1a passes; leg 2a fails:

```
AssertionError: normal deps are ['notify', 'pulldown-cmark', 'ratatui', 'serde_json', 'toml', 'yaml-rust2'],
expected ['pulldown-cmark', 'ratatui', 'serde_json', 'toml', 'yaml-rust2']
DEPS FAIL: leg 2a
```

**`GRAPH-SNAP`** — also red since `live-refresh`, but **not** for the reason `HANDOFF.md`
records. The snapshot was regenerated: `git log -- tests/fixtures/build-graph.txt` gives
`574b87d feat(live-refresh): group 6 — the real watcher, and the dependency`, and the file holds
`notify v8.2.0`, `fsevent-sys v4.1.0`, `inotify v0.11.5`, and `inotify-sys v0.1.8`. The script
passes its `diff -u "$SNAP"` leg and fails four legs later on a hardcoded platform-difference
literal that `notify`'s backends invalidated:

```
GRAPH-SNAP FAIL: macOS/Linux differ by [fsevent-sys inotify inotify-sys linux-raw-sys ], expected [linux-raw-sys ]
```

`HANDOFF.md` assigns the repair to this change: "**Decision: `agent-launch` owns the repair** —
regenerate `tests/fixtures/build-graph.txt` and author a want-list matching the real dependency
set." **That assignment is declined, and its scope is corrected on the way out.** This change
adds no dependency and edits neither script, so folding in the repair would mean this change
blessing a gate whose want-list it had no reason to re-derive — and the assigned scope is wrong
twice: the snapshot needs no regenerating, and the fix `GRAPH-SNAP` actually needs is a widened
named platform difference (`fsevent-sys inotify inotify-sys linux-raw-sys`), which `HANDOFF.md`
does not mention at all. The repair belongs to a change that touches the dependency set, with
that corrected scope.

The broader lesson `HANDOFF.md` draws from this stands and is not disputed: **a check that lives
outside `make check` will rot, because nothing forces it to run.** Every gate this change adds
or edits is run from a task, at an explicit floor, with a proven-red plant recorded beside it.

## Residual risk after the repair pass

- **`WIRED` leg 6 half (i) remains a presence check.** The behavioural half is now two
  acceptance runs with two different `agent_kind` values, which is what actually catches a
  hardcoded literal. Recorded rather than strengthened: a leg that parsed
  `start_collaborators`' call site would be a source-level parser in a shell script, which is
  the shape that rots.
- **A cross-change derived-name collision is unattributable** and is stated as a limitation in
  `specs/agent-launch/spec.md` rather than fixed. It needs a second identifier the plugin does
  not have.
- **The launch-in-flight window is invisible.** Between the press and the outcome — up to thirty
  seconds, since `herdr agent start` waits that long for readiness — the pane shows nothing new.
  Deliberate; a pending indicator is a state of its own and a later change.
