## Reviewed Artifacts

- `openspec/changes/subprocess-seam/proposal.md`
- `openspec/changes/subprocess-seam/specs/subprocess-seam/spec.md` (new capability)
- `openspec/changes/subprocess-seam/specs/openspec-binary/spec.md` (delta: REMOVED + ADDED)
- `openspec/changes/subprocess-seam/design.md`
- `openspec/changes/subprocess-seam/tasks.md`

Reviewed against `SPEC.md`, `PRD.md`, `AGENTS.md`, `openspec/config.yaml`,
`openspec/IMPLEMENTATION-ORDER.md`, the published `openspec/specs/openspec-binary/spec.md`,
and the crate itself (`src/resolve.rs`, `src/lib.rs`, `tests/cli.rs`, `Cargo.toml`,
`Makefile`).

The finding pass was delegated to **three independent subagents**, none of which wrote the
plan and none of which was a fork of the planning session, per `openspec/config.yaml`'s
`planning-review` rule. Each was given one slice and required to write findings
incrementally to a scratchpad file rather than only in a final message:

| Reviewer | Slice | Findings file |
|---|---|---|
| coverage | capability coverage, scenario quality, scenario↔matrix↔task, task lifecycle, the hand-over and end-to-end obligations | `scratchpad/review-coverage.md` |
| redness | "what would make this go red?" for every command, run experimentally against planted defects | `scratchpad/review-redness.md` |
| design | design completeness, test boundaries, contradictions, PRD non-goals, architecture invariants | `scratchpad/review-design.md` |

## Reviewed Against

- This repository HEAD: `071195f8dddeccf3de49a6846b3316527ac21b8c`
  (`docs(handoff): record the structural two-producer gate and measured costs`)
- Sibling repository HEAD: **Not applicable** — this change has no sibling repository. The
  `openspec/schemas/tdd/` and `.claude/agents/` trees are graft-vendored from
  `optioni/openspec-schemas` and are neither read nor written by this change.
- Working tree: clean apart from `openspec/changes/subprocess-seam/`, this change's own
  artifacts, intentionally included. No source file was modified during planning.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md → Test Strategy (`NPM-PATH`), tasks.md 6.2 | The hand-over red rested on the claim "`npm` is not on the `PATH` a plain shell inherits". **Measured and false**: `/bin/sh -c 'command -v npm'` prints `/opt/homebrew/bin/npm`. The plain-`PATH` run stays green today only because that Homebrew node is broken (`dyld: Library not loaded: libllhttp.9.3.dylib`, exit 134, empty stdout) — a `brew reinstall node` would flip it, and 6.2 told the implementer to *stop and investigate* on exactly that result | The precondition is now **measured, not asserted**: `NPM-PATH` runs `$NPMBIN/npm prefix -g` itself and aborts unless it exits 0 and prints an absolute path. 6.2's plain-`PATH` run is demoted to informational, with the broken-Homebrew fact recorded so a future green is not misread | design.md → Test Strategy (`NPM-PATH` block and the paragraph under it); design.md → Risks; tasks.md 6.2 |
| CRITICAL | design.md → Test Strategy (`OPENSPEC-UNTOUCHED`), tasks.md 8.5 | The write-invariant check was blind to the violation it exists to catch: `git diff` lists **tracked** paths only, and a file written at runtime is untracked. Reviewer reproduced it — a planted `openspec/specs/foo/cache.md` produced an empty stray list and the check reported success. It also passed silently when run from a subdirectory | Added a `git ls-files --others --exclude-standard` sweep unioned with the diff, anchored both pathspecs with `:(top)` and both commands with `git -C "$ROOT"`, and added a `ROOT` guard. Re-verified at planning time: clean → OK; planted untracked file → FAIL naming it; bogus `BASE` → FAIL; run from `src/` → same answer as from the root. 8.5 now requires demonstrating that red once | design.md → Test Strategy (`OPENSPEC-UNTOUCHED`); tasks.md 8.5 |
| CRITICAL | design.md → Test Strategy, tasks.md 2.7 / 3.5 / 4.5 / 5.6 / 7.6 | Every group's VERIFY was a filtered `cargo test --all-features cli::`. **A filter matching zero tests exits 0** — verified: `cargo test --all-features zzz_no_such_test` returns 0. A renamed module, a mistyped filter, or tests never written would have passed silently, and 2.7 even claimed "all eight tests" with nothing counting them | Added a `TESTCOUNT` shell function that parses `test result: ok. N passed` and gates on a per-group minimum (8 / 12 / 20 / 27 / 31). Verified both polarities: real filter → OK at 42; nonexistent filter → FAIL at 0 | design.md → Test Strategy (`TESTCOUNT` block); tasks.md 2.7, 3.5, 4.5, 5.6, 7.6 |
| CRITICAL | design.md → Test Strategy, tasks.md 6.6 | Nothing in the plan could distinguish a completed hand-over from shipping `pub fn npm_prefix() -> Option<PathBuf> { None }` — the exact body being deleted. The chain tests inject their own hooks, the no-spawn greps say nothing about return values, and "no prefix" is a legitimate answer for the smoke test, so that implementation kept **everything** green with step 4 still dead. The matrix promised a "source-text check that `src/resolve.rs` names `crate::cli::npm_prefix`" that no block defined and no task ran | Added the `BINDING` block with three guards — (a) `resolve.rs` names `cli::npm_prefix`, (b) no `npm_prefix_deferred` survives under `src/`, (c) `npm_prefix()`'s own body delegates to the probe — plus three negative controls in a new task 8.3a. Added the behavioural half as a unit test in 6.4: `npm_prefix()` must equal `npm_prefix_via(Path::new("npm"))`, which is machine-independent yet red for a hardcoded `None` wherever a working `npm` exists | design.md → Test Strategy (`BINDING` block), → Decisions; specs/subprocess-seam ("One binding names the real `npm` program"); specs/openspec-binary ("The production binding is the real probe"); tasks.md 6.4, 6.6, 8.3a |
| WARNING | specs/subprocess-seam, design.md → Decisions, tasks.md 4.x | The fake was keyed on the argument vector alone, but one type implements both traits. A caller reaching for the wrong handle would be answered out of the other program's registration — "a caller's test passes while the caller spawned the wrong command", and invisible, because the panic cannot fire when the vector *is* registered. The `E0034` ambiguity on a bare `fake.run(..)` was also unwarned | Registrations and recorded calls are now keyed on the pair (program addressed, argument vector); the panic names both. Added the scenario "An `openspec` call is not answered from a `herdr` registration" and the disambiguation note (`OpenspecCli::run(&fake, ..)`) | specs/subprocess-seam (fake requirement + new scenario); design.md → Decisions, → verification matrix; tasks.md 4.1, 4.2, 4.5 |
| WARNING | specs/subprocess-seam, design.md → Contracts | `CliError` carried no argument vector. `changes-from-cli` drives four distinct invocations through one `RealOpenspecCli` and `agent-launch` four more through one `RealHerdrCli`; all eight failures would have rendered identically, falsifying design.md's own Observability claim that a caller can name the failure | Both variants now carry `args: Vec<String>`; the scenarios assert it | specs/subprocess-seam (trait contract + two scenarios); design.md → Contracts, → Persistence and Rollout; tasks.md 2.1, 2.2, 2.4 |
| WARNING | design.md → Decisions, specs/subprocess-seam | The seam trimmed **stderr** while returning stdout verbatim — having argued two paragraphs earlier that trimming is a decision and decisions belong outside the seam. Self-contradictory | Neither stream is trimmed. The inconsistency and its correction are recorded in Decisions rather than silently fixed | specs/subprocess-seam (trait contract, non-zero-exit scenario); design.md → Contracts, → Decisions; tasks.md 2.1, 2.4 |
| WARNING | design.md → Test Strategy (`NOSPAWN-GREP`) | The exclusion was written `! -name 'cli.rs'`, which excludes by **base name**: a future `src/ui/cli.rs` would be silently exempted with the file count still reading 8. Reviewer demonstrated it | Changed to `! -path "$SRC/cli.rs"`. Re-verified: a spawn planted at `ui/cli.rs` now fails the check, naming the file and line. Added it as spec scenario evidence and as negative control (d) in task 8.3 | design.md → Test Strategy; specs/subprocess-seam (two scenarios); tasks.md 8.3 |
| WARNING | design.md → Test Strategy (`DEPS`) | Only **normal** dependencies were pinned. A planted `assert_cmd` dev-dependency — the likeliest accidental addition for a change that starts spawning things — passed silently | Dev and build sets are now pinned empty too (they are empty today). Re-verified: passes on the real tree, and fails loudly on non-JSON input | design.md → Test Strategy; tasks.md 8.6 |
| WARNING | design.md → SPEC.md corrections, tasks.md 10.x | Correction 4 adds `cli` to SPEC.md's "Unit-tested modules" list, whose lead-in one line above reads "Each is a pure transformation, tested without a TUI or a subprocess:" — the new entry would contradict it. A fifth correction was missing | Added correction 5 and task 10.4a, narrowing the lead-in in place | design.md → SPEC.md corrections; tasks.md 10.4a |
| WARNING | design.md → SPEC.md corrections, tasks.md 10.x | design.md said "No roadmap correction is needed". The **row** is indeed correct, but `openspec/IMPLEMENTATION-ORDER.md`:21's ordering principle — "so no test ever spawns a real process" — is falsified by this change's ~20 scratch `#!/bin/sh` spawns | Added task 10.4b narrowing the principle to what it means (no test spawns the `openspec` or `herdr` binaries), with the row explicitly confirmed as needing no change | design.md → SPEC.md corrections; tasks.md 10.4b |
| WARNING | tasks.md group 10 | Six rewrite tasks with no CHECK before them and no VERIFY after. Three of them say "rewrite in place, do not append beside the stale claim" with nothing enforcing it | Added 10.0 (capture the stale phrases with counts) and 10.8 (a guarded, output-judged `grep -nF` proving all six are gone). Every phrase was re-chosen to lie **within one line** of the wrapped source, because a phrase spanning a line break cannot be matched by a line-oriented search. Demonstrated red against the tree as it stands: all six phrases found | tasks.md 10.0, 10.8 |
| WARNING | tasks.md 3.1 / 3.5 | The stdin-blocking scenario's red is environment-dependent: with the invoker's own stdin already at EOF it passes even with the null-stdin attachment removed — green by construction on a CI runner | 3.5 now carries an explicit one-time negative control: remove the null stdin, run from a shell whose stdin is an open terminal or fifo, confirm the hang, restore | tasks.md 3.5 |
| WARNING | design.md → Risks/Decisions/Migration Plan, tasks.md preamble | Eight stale task cross-references after group renumbering. The damaging one: Risks mitigated the hand-over hazard by pointing at "Task 5.2", a GREEN implementation task, when the observation is 6.2 | All corrected (5.2→6.2, 5.4→6.6, 10.5→11.5, group 5→6, group 6→7, group 8→10, 6.1→6.4, 11.6→11.5) | design.md → Decisions, → Risks, → Migration Plan, → verification matrix; tasks.md preamble |
| WARNING | tasks.md preamble, specs/subprocess-seam | The preamble claimed 7.4's snapshot "proves nothing is written anywhere". It proves nothing is written in the *snapshotted scratch tree*; the requirement's "not in the working directory" clause had no scenario | Added a second snapshot pair over `std::env::current_dir()` as scenario evidence and as a second matrix row; the preamble now states what the two snapshots actually cover | specs/subprocess-seam (writes-nothing scenario); design.md → verification matrix; tasks.md preamble, 7.4 |
| WARNING | design.md → Test Strategy (`NPM-PATH` note) | The design cited "oh-my-zsh plugin noise on stderr" as evidence that the `npm` binary writes noise there. The noise comes from an interactive-shell **function wrapper**; a child started with `Command::new` will very likely see clean stderr | The note now says so explicitly, and states that the stdout-only rule is proven instead by the scratch-program scenario and structurally by the decision function having no stderr parameter. `SPEC.md`'s warning is still honoured, but is no longer cited as evidence it does not supply | design.md → Test Strategy |
| WARNING | design.md → Contracts | Only `npm_prefix()` was published, so task 2.6's "signatures match Contracts exactly" gate could not check `npm_prefix_via`, which an implementer could privatize — breaking the end-to-end matrix row | `npm_prefix_via` is now in Contracts and in the Boundaries table, with the reason it is public | design.md → Contracts, → Boundaries |
| SUGGESTION | design.md → Test Strategy (`NOSPAWN-GREP`) | GNU `xargs` runs its command once with no input, making `grep` read stdin and hang. Guard C already makes that unreachable, but the reliance was implicit | Added a trailing `/dev/null` to `grep`'s argument list so the case is unreachable regardless of the guard. Re-verified all five polarities after the change | design.md → Test Strategy |
| SUGGESTION | tasks.md 8.1 | "Run each block exactly as written" was an instruction, not a mechanism | 8.1 now requires copying the blocks verbatim into a scratch shell file and recording its path, so the review in group 9 can diff what was run against the design | tasks.md 8.1 |
| SUGGESTION | tasks.md 11.4 / 11.5 | The baseline comparison had no red condition — coverage could fall 18 points and still pass the 80% floor | 11.5 adds a softer condition (more than one percentage point below the 98.66% baseline) alongside the hard floor, which is never lowered or waived; 11.4 gates on the total test count exceeding the baseline | tasks.md 11.4, 11.5 |
| SUGGESTION | design.md → verification matrix | A matrix cell rendered a Rust closure as `&\|\| ...`, and another as `\|\| None` — escaped pipes inside table cells, the visual shape of the Phase 1 ERE defect | Both rewritten in prose so no escaped pipe appears in a cell | design.md → verification matrix |
| SUGGESTION | design.md → Contracts, → Decisions | The "no `Utf8` variant" decision cited `SPEC.md`'s invalid-UTF-8 degraded row, which is about the plugin reading a *tasks file*, not about decoding a CLI's stdout | The citation is now labelled a precedent rather than a governing rule, and the substantive reason is stated on its own | design.md → Contracts, → Decisions |
| SUGGESTION | design.md → Test Boundaries | `python3`, `sed`, `sort`, and `xargs` were used by the command blocks but absent from the collaborator table | Added to the tooling row; the `git` row now names both halves of the write-invariant check | design.md → Test Boundaries |

### SPEC.md and roadmap corrections this change will make

`openspec/config.yaml` requires the before/after to be logged here. All six were confirmed
present at planning time with the line numbers below; task 10.0 re-confirms them and task
10.8 proves they are gone afterwards.

| # | Document / section | Before (verbatim, at `071195f`) | After |
|---|---|---|---|
| 1 | `SPEC.md`:28-29, Architecture → The subprocess seam | `trait OpenspecCli { fn run(&self, args: &[&str]) -> Result<String>; }` and the matching `HerdrCli` line — a bare `Result` with no error type and no thread bound | The shipped signature: `pub trait OpenspecCli: Send + Sync { fn run(&self, args: &[&str]) -> Result<String, CliError>; }`, plus one line on why the bound exists (`live-refresh` and `agent-polling` both hold one across a thread) |
| 2 | `SPEC.md`:42, same section | "the untestable residue is two thin wrappers, the `npm prefix -g` binding, and `main`" | The wrappers and the probe are covered by tests against scratch `#!/bin/sh` programs; the residue is the one-line `npm_prefix()` program binding plus `main` |
| 3 | `SPEC.md`:200-206, Data layer → Resolution chain | "Step 4 is unwired until `subprocess-seam` lands: … the binding `repo-resolution` ships always returns nothing … Whoever wires it must read `npm prefix -g`'s **stdout only**, trimmed" | The landed state: the hook stays injected (which is what keeps `resolve` pure), its production binding is `cli::npm_prefix`, and the stdout-only/trimmed rule is that binding's contract rather than an instruction to a future change. Removes more text than it adds |
| 4 | `SPEC.md`:396ff, Testing and quality gates → Unit-tested modules | (no `cli` entry) | Adds `cli`: the traits' contract and the npm probe, tested against scratch `#!/bin/sh` programs rather than the real `openspec`, `herdr`, or `npm` |
| 5 | `SPEC.md`:396, the same list's lead-in | "Each is a pure transformation, tested without a TUI or a subprocess:" | Narrowed: these modules are pure transformations tested without a TUI, and `cli` is the one exception, tested against scratch programs because performing a spawn is what it exists to do. Without this, correction 4 would ship a self-contradicting paragraph |
| 6 | `AGENTS.md`:29, Current repo state | "the binary chain's fourth probe step ships as an injected hook that always returns nothing until `subprocess-seam` wires it" | Rewritten in place to the landed state, with `subprocess-seam` added to the list of landed changes |
| 7 | `AGENTS.md`:123ff, Architecture rules, the spawn bullet | Describes the seam in the future tense | Rewritten: the seam exists, `src/cli.rs` is the one module permitted to name a process-spawn API, and the check guarding it excludes exactly that file and fails when the exclusion is vacuous |
| 8 | `openspec/IMPLEMENTATION-ORDER.md`:19-21, Ordering principles | "`OpenspecCli` and `HerdrCli` land as traits with fakes before the first change that shells out, so no test ever spawns a real process" | Narrowed to what it means: no test spawns the `openspec` or `herdr` binaries. This change spawns ~20 scratch shell programs in `cli`'s own tests, and `tests/cli.rs` already spawns the crate's own binary. The `subprocess-seam` **row** is correct and unchanged |

Corrections 1-5 are `SPEC.md`; 6-7 are `AGENTS.md`; 8 is the roadmap. Eleven such
corrections came out of Phases 1-2; these are the next eight.

## No Remaining Implementation-Blocking Gaps

None remain. Every CRITICAL is repaired in the artifact that owns it, every WARNING is
either repaired or recorded below with a reason, and `openspec validate subprocess-seam
--strict` reports the change valid.

Coverage after repair: **40 spec scenarios across the two spec files, 40 present in
design.md's verification matrix, 0 orphan matrix rows, and every one carrying a task.**
Verified mechanically, not by inspection.

Nothing requires user input. Two decisions were resolved during planning rather than
deferred: whether the real implementations are tested by spawning (yes — scratch
`#!/bin/sh` programs at absolute paths, which keeps the no-tools suite run green), and how
the hand-over red is made observable (a measured `npm prefix -g` precondition, not an
assumption about `PATH`).

## Deferred Non-Blocking Notes

- **Whether Herdr injects a variable naming its own binary path** is `agent-polling`'s
  question, not this change's. `RealHerdrCli` takes its program path from its constructor
  and defaults to the bare name `herdr`, so answering it later changes no API here.
  Resolution point recorded in design.md → Decisions and → Open Questions.
- **A pre-existing `thread::sleep`-then-assert at `tests/cli.rs:44`** was noticed by the
  redness reviewer. It belongs to `repo-foundation`, not to this change, and this change's
  own deadline-sensitive test (the stdin scenario) polls `recv_timeout` to a deadline
  instead. Left alone deliberately rather than widening scope; if it ever flakes, it is a
  bug fix, not a proposal.
- **`src/cli.rs` and `tests/cli.rs` share a base name** and mean different things (the
  subprocess seam; the binary's command-line interface). Kept, because `SPEC.md`'s module
  map names the module `cli`. Both files get a doc comment naming the other — tasks.md 1.4.
