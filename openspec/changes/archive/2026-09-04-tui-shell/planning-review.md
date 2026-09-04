# tui-shell — planning review

## Reviewed Artifacts

- `openspec/changes/tui-shell/proposal.md`
- `openspec/changes/tui-shell/specs/terminal-lifecycle/spec.md`
- `openspec/changes/tui-shell/specs/dashboard-loop/spec.md`
- `openspec/changes/tui-shell/specs/responsive-layout/spec.md`
- `openspec/changes/tui-shell/specs/plugin-build/spec.md` (delta on a live capability)
- `openspec/changes/tui-shell/specs/quality-gates/spec.md` (delta on a live capability)
- `openspec/changes/tui-shell/design.md`
- `openspec/changes/tui-shell/tasks.md`

Read as context, not reviewed: `SPEC.md`, `PRD.md`, `AGENTS.md`, `openspec/config.yaml`,
`openspec/IMPLEMENTATION-ORDER.md`, `openspec/specs/plugin-build/spec.md`,
`openspec/specs/quality-gates/spec.md`, `openspec/specs/change-model/spec.md`,
`openspec/changes/archive/2026-09-04-changes-from-cli/{tasks,design}.md`, and the crate
source under `src/`.

**Three reviewers, none of which wrote the plan**, each given the change directory and one
slice, and each told to report findings only and to write them incrementally to a file so a
mid-run failure could not lose them:

| Reviewer | Slice | Findings file |
|---|---|---|
| 1 | Capability coverage, scenario quality, arithmetic, failure surface, scope, SPEC.md agreement | `scratchpad/review-specs.md` |
| 2 | Design completeness, Test Boundaries, verification matrix, technical feasibility against the real `ratatui` 0.30.2 source, coverage | `scratchpad/review-design.md` |
| 3 | Task alignment, lifecycle markers, and whether every check in tasks.md can actually go red — by extracting and **running** each one against the tree, a synthetic end state, planted violations, and a git clone | `scratchpad/review-tasks.md` |

The planning session merged the findings, repaired the owning artifact, and wrote this
file. The reviewers edited nothing.

## Reviewed Against

- **This repository's HEAD:** `89e9780079908b56d8fddb11491b086aa78ec4e0`
  (`docs(handoff): refine the budget floor and separate session from weekly units`).
- **Sibling repositories:** Not applicable. `github.com/optioni/openspec-schemas` supplies
  `openspec/schemas/tdd/` and `.claude/agents/` through graft, but this change touches no
  schema and no agent definition, so no sibling contract is in play.
- **Working tree at review time:** clean apart from this change's own artifact directory,
  `openspec/changes/tui-shell/`, which was intentionally included. No source file, no
  manifest, and no document outside that directory was modified during planning.

## Gaps Found and Fixed

### Blocking — the verification apparatus

| # | Severity | Source artifact | Problem | Repair | Now at |
|---|---|---|---|---|---|
| 1 | CRITICAL | tasks.md 4.3, 4.6; specs/responsive-layout; specs/quality-gates | `WIDTHS` requires the literals `60` **and** `120` in every `#[test]` in `src/ui/view.rs` and has **no exemption mechanism**, yet three of the sixteen tests as specified never named 60 — `one_column_frame_does_not_panic` (1, 1, 120), `footer_drops_whole_hints` (18, 20, 120), `header_omits_the_path_when_too_narrow` (16, 120). Task 4.6 demanded the check pass, which was unreachable. design.md described an exemption list the script does not have | Added the missing 60-column leg to all three scenarios and to task 4.3, each with a real assertion rather than a token render; deleted the exemption-list language from design.md's matrix row and Risks; stated in `quality-gates` that there is no exemption list and that a boundary width is exercised *in addition to* both mandated widths | specs/responsive-layout scenarios 3, 5, 15; specs/quality-gates ¶3; tasks.md 4.3, 4.6; design.md matrix + Risks |
| 2 | CRITICAL | tasks.md 4.1 | `render_at_touches_no_directory` snapshotted `std::env::temp_dir()` before and after a render. This crate creates and destroys 203 `ScratchDir`s directly under that directory across `cargo test`'s parallel threads. Reviewer 3 planted the equivalent probe and ran the real lib suite ten times: **red on 6 of 10**, with `herdr-openspec-test-<pid>-<n>` entries in the diff. It would have broken `make check` in groups 4–9 and 12 | The test now snapshots a `ScratchDir` it owns, with the full recursive `testutil::snapshot`, and additionally asserts row 0 spells `OpenSpec` and the last row begins `q quit`, so it cannot pass on two empty snapshots. The stronger claim — no view file can perform I/O at all — is `NOIO-VIEW` | tasks.md 4.1; specs/quality-gates scenario 1 |
| 3 | CRITICAL | tasks.md 1.5, 1.7 | `DEPS` leg 3, carried forward from `changes-from-cli`, asserts a sixteen-package `EXPECTED_GRAPH`, that all four triples resolve identically, and that no `syn`/`quote`/`proc-macro2`/`ryu` is present. Adding `ratatui` falsifies all three (reviewer 3 measured 72 packages and a `linux-raw-sys`-only macOS/Linux difference in a merged tree). None of task 1.5's edits touched it, and 1.7 demanded legs 1–4 pass | Task 1.5 now has a fourth edit: delete leg 3 and its `EXPECTED_GRAPH` and per-triple equality loop, recording that `GRAPH-SNAP` supersedes it — which is what design.md → Decisions already decided. `TRIPLES` is kept, since leg 4 uses it. Task 1.7 now reads "legs 1, 2 and 4" | tasks.md 1.5, 1.7 |
| 4 | WARNING | tasks.md 1.3 | The predicted `GRAPH-SNAP` failure message was "the proc-macro set is empty". Reproduced: it never reaches that assertion — the per-triple `lines >= 40` guard trips first with `GRAPH-SNAP FAIL: aarch64-apple-darwin resolved only 16 packages` | Prediction corrected to the message the script actually emits, with a note to record what the run prints rather than the assertion it would have reached | tasks.md 1.3 |
| 5 | WARNING | tasks.md 5.4 | The predicted partial failure was the `CrosstermOps` site-count leg. Reproduced against a synthetic end-of-group-5 tree: the check fails earlier, on `NORAW FAIL: searched only 15 files (expected >= 16)`, because `ui/driver.rs` and `ui/event.rs` do not exist until group 6. An implementer expecting one failure and handed another would reasonably suspect the threshold and weaken it | Prediction corrected, with an explicit "do not weaken the threshold — the guard is doing its job", and the full re-run still deferred to 9.4 where the count is 17 and `CrosstermOps` reports 2 sites | tasks.md 5.4 |
| 6 | WARNING | tasks.md group 12 | `config.yaml` → `rules.tasks` requires `make check` as the closing gate. Group 12 ran the four sub-commands individually but never the composite, and the last `make check` in the plan was 9.8 — before group 10's documentation edits and group 11's review fixes, so the finished tree was never put through the gate | Added 12.9: `make check`, last, naming the failing sub-command rather than summarising if it fails | tasks.md 12.9 |
| 7 | WARNING | tasks.md group 8 | The only behavior group with neither a REFACTOR task nor the "no refactor was needed" statement the schema requires | 8.6 is now `REFACTOR + VERIFY` and states it explicitly | tasks.md 8.6 |

### Blocking — specification gaps

| # | Severity | Source artifact | Problem | Repair | Now at |
|---|---|---|---|---|---|
| 8 | WARNING | specs/dashboard-loop | The requirement that `Dashboard` implement no `Default` and name every field at every site had **no scenario**, and the proposal's claim that `change-model`'s existing gate covers it is false: that gate is stated over `Change`, `ChangeSet`, `ArtifactRef`, and `Origin` in `src/changes.rs`. `Dashboard { route, .. }` would have compiled with nothing noticing, and the only guard was a bullet in the reviewer brief | Added the scenario, plus a `NODEFAULT-UI` check (guarded grep for `impl Default for Dashboard`, for `Default` in the derive above `struct Dashboard`, and for `..` inside a `Dashboard { … }` literal or pattern, with a `struct Dashboard` positive control), plus a compile-time companion test that destructures all five fields with no `..`. Three negative controls added to task 9.5 | specs/dashboard-loop scenario 1; design.md Boundaries + matrix; tasks.md check block, 3.1, 3.4, 9.3, 9.5, 12.7 |
| 9 | WARNING | specs/dashboard-loop | "SHALL carry exactly **four** fields … making **five** fields in total" — self-contradictory, in the requirement whose whole point is that every site names every field | Rewritten as one list of exactly five | specs/dashboard-loop ¶1 |
| 10 | WARNING | specs/quality-gates | "view scenario" was undefined. Five `dashboard-loop` driver scenarios assert on buffer content and name one width each; under the requirement's plain reading they violate it, under `WIDTHS`'s scope they are invisible | The requirement now defines a view scenario as one whose assertions are on a `Buffer` produced by `ui::view::render`, and puts loop-sequencing scenarios explicitly out of scope | specs/quality-gates ¶2 |
| 11 | WARNING | specs/responsive-layout | "every interior cell SHALL be a space with the default style" is unimplementable as worded. Measured: an untouched cell's style is `fg(Reset).bg(Reset).underline_color(Reset)`, equal to neither `Style::default()` nor `Style::reset()`, because `ratatui-crossterm` re-enables `underline-color` through its own defaults. The obvious assertion is red with no bug behind it, and the likely "fix" is to drop the style half | Pinned to the constructible value `ratatui::buffer::Cell::default().style()`, with the reason stated, and added to `region_interiors_are_blank`'s task so the clause is actually verified — it is the assertion that catches `Block::style` being set where `Block::border_style` was meant | specs/responsive-layout ¶ Interiors; tasks.md 4.3 |
| 12 | WARNING | specs/responsive-layout | `A = width − 9` is an unsigned subtraction that underflows at widths 0–8 — exactly the widths the one-column scenario exercises — and the requirement never permitted the label truncation the 1x20 scenario asserts | `A` is floored at zero, and a clause added for widths below 8 truncating the label | specs/responsive-layout ¶ Header |
| 13 | WARNING | design.md → Test Boundaries | The command-level row named nine tools; the checks additionally run `diff`, `tr`, `wc`, `cut`, `mktemp`, `printf`, `cp`, `rm`, and `mkdir`. The table's own closing sentence forbids exactly that, and task 9.1 gates on it, so the confirmation could not honestly be given | Row extended to the full enumeration, with a note that exhaustiveness is the point: a new tool is then a reviewable addition | design.md Test Boundaries |
| 14 | SUGGESTION → taken | specs/dashboard-loop, design.md | `load::archived_count_from_config_is_honoured` was a fourth planned test with no scenario and no matrix row — and it is the only place the `Config` collaborator's value is observed, so it is the group's most load-bearing test, not its least | Added the scenario (seven dated archive directories read at `archived_count` 3 and again at 7) and its matrix row | specs/dashboard-loop scenario 14; design.md matrix |
| 15 | SUGGESTION → taken | specs/terminal-lifecycle | "A recording double satisfies the trait without a terminal" asserted "it compiles", which every other scenario in the file already proves by using the double | Retitled and rewritten around the discriminating claim: `CrosstermOps` is named only where it is defined and where `run` wires it, at least twice, never under `tests/` | specs/terminal-lifecycle scenario 2 |
| 16 | SUGGESTION → taken | specs/plugin-build, specs/terminal-lifecycle | The tree-wide raw-mode grep had two owners; a future edit could satisfy one while breaking the other | `terminal-lifecycle` keeps the grep. `plugin-build`'s scenario is reduced to what is genuinely its own — every `tests/cli.rs` spawn site pipes stdout — and says so | specs/plugin-build scenario 3 |
| 17 | SUGGESTION → taken | specs/plugin-build, design.md | The `ratatui` feature row claimed `underline-color` is dropped by `default-features = false`. It is not: `ratatui` declares `ratatui-crossterm` with **its** defaults on, and those include it. `crossterm` likewise re-enables `std` | Corrected in the dependency table and in design.md → Decisions, where it is the fact that explains finding 11 to whoever hits it | specs/plugin-build dependency table; design.md Decisions |
| 18 | SUGGESTION → taken | design.md → Decisions | The proc-macro rationale named `strum`, `thiserror`, and `derive_more` — ordinary library crates — and omitted `darling_macro`, `rustversion`, and `derive_more-impl`. The spec's allowlist was already correct and empirically exact; only the rationale was wrong, which is what a reader consults when the check later fails | Replaced with the spec's eight names and where each enters | design.md Decisions |
| 19 | SUGGESTION → taken | specs/responsive-layout, design.md | The 0/1/2 height branch was justified as "the solver's behaviour is not part of its contract" — true but weak. Measured: at height 1 `Layout::vertical([Length(1), Min(0), Length(1)])` gives the single row to the **footer**, so the naive split renders `q quit` where `OpenSpec` belongs | Replaced with the measured fact, which also tells the implementer what the RED test looks like if the branch is deleted | specs/responsive-layout ¶ Degenerate; design.md Decisions |
| 20 | SUGGESTION → taken | specs/responsive-layout | The width-1 footer branch (no hint fits at all) was reachable but unasserted; and eleven scenarios never named the `ChangeSet` they render, which `list-view` will need to know when it changes these assertions | Added the `row 19 is a single space` assertion at 1x20, and named `changes::empty_set()` once in the requirement preamble rather than in eleven scenarios | specs/responsive-layout |
| 21 | SUGGESTION → taken | tasks.md | Housekeeping that would have cost implementation time: `TESTCOUNT.sh` must be **sourced**, not executed, and no task said so; `DEPS` is a `sh` block, not the Python one, so 1.1's `DEPS.py` was wrong; the preamble attributed the `DEPS` edits to 1.3 rather than 1.5; `WORK` was used by 9.5 but never exported; the dead `--all-targets` `TESTCOUNT` scope selected a branch that never passed the flag; `NOCLI-SHELL`'s `[ -ge 7 ]` had zero slack against an end state of exactly 7 files; `WIDTHS` could be fooled by a `#[test]` inside a doc comment; `NOIO-VIEW`'s comment explained why `mod.rs` and `terminal.rs` are unsearched but not `event.rs`; 9.5's `$WORK` rationale described the wrong mechanism | All fixed: sourcing instruction, `DEPS.sh`, pointer to 1.5, `WORK` exported in 1.1, dead scope deleted, `UI_MIN` parameterised the way `NOSPAWN-GREP`'s `MIN` already is, `WIDTHS` strips doc-comment lines and splits on a line-anchored attribute with its remaining limits stated, `event.rs` named in the comment, 9.5's rationale corrected to `cp -R` failing loudly | tasks.md check blocks, 1.1, 9.5 |
| 22 | SUGGESTION → taken | tasks.md 8.5, 4.3 | 8.5's filtered `cargo test --test cli <name>` had no counted minimum — the pattern the change's own preamble forbids — and `testcount --test-cli '' 5` cannot discriminate on its own, since 5 is the HEAD baseline. Separately, 101 columns was asserted only at the layout tier though the spec names it as a rendered width | 8.5 now requires reading the run's `filtered out` arithmetic and states what actually makes 8.4 non-skippable (8.3 deletes `lib::banner`, so a surviving `ui_prints_placeholder_banner` fails the run). The view test is renamed `breakpoint_is_exact_at_the_boundary` and renders 60, 99, 100, 101, and 120 | tasks.md 8.5, 4.3; design.md matrix |
| 23 | Format | specs/plugin-build | `openspec validate --strict` refuses a MODIFIED requirement that drops a scenario name the live spec has, so the build-graph scenario could not be renamed to `…matches the committed snapshot on every supported triple` | Title restored verbatim to `The resolved build graph is small and proc-macro-free`, with a note in the scenario saying the title is kept for that reason and what its content now means | specs/plugin-build |

### Non-blocking, accepted as written

- **`WIDTHS` remains a heuristic.** Reviewer 3 demonstrated three false-green modes: a `60`
  in a comment, an unrelated `60` literal, and (before the repair) a `#[test]` inside a doc
  comment inflating the count. The third is fixed; the first two are inherent to a source
  grep and are now stated in the check's own header, in design.md → Risks, and in the
  `quality-gates` scenario. The proof that the tests exist and run is
  `testcount --lib 'ui::view::tests::' 16`, not `WIDTHS`.
- **`OPENSPEC-UNTOUCHED`'s untracked sweep honours the user's global gitignore.**
  `git ls-files --others --exclude-standard` respects `core.excludesFile`, so a runtime write
  named `.DS_Store` or `*.log` under `openspec/` could be invisible. Dropping
  `--exclude-standard` would sweep `target/` into every run. Accepted: the plugin writes
  only under `HERDR_PLUGIN_STATE_DIR`, and this change adds no write path at all.
- **`NODEFAULT-UI` half B matches a same-line `..` only.** A multi-line elision inside a
  `Dashboard { … }` literal would escape the grep. That is precisely why the compile-time
  destructuring companion exists; the two halves are complementary by design, as
  `change-model`'s own two mechanisms are.
- **`Length(40)` leaves a 38-column interior.** Named as a decision, written into `SPEC.md`
  by group 10, and left to `list-view` to decide how a row is shortened for it — with the
  constraint known in advance rather than discovered against.

## Corrections to durable documents this change makes

All nine are `SPEC.md` edits, made by tasks 10.1 and 10.2 during implementation, listed here
so the before/after is on the record rather than only in a diff.

| # | Section | Before | After |
|---|---|---|---|
| 1 | Overview → Stack | "Rust, `ratatui` + `crossterm` (TUI)" | Same list, with `crossterm` marked as reached through `ratatui`'s re-export and **not** a declared dependency |
| 2 | Architecture → Module map, `ui` row | "Views, layout, key handling" | "Views, layout, key handling, terminal lifecycle, and the event loop" |
| 3 | User interface → Keys | The table ends `\| `q` \| Quit \|` | A `Ctrl-C` row beside it, and a note that `Esc` at the list root is inert rather than a quit |
| 4 | Degraded states | "Every condition renders usable content rather than an error screen:" followed by the table | Unchanged sentence, unchanged table, plus a short paragraph **below** the table headed so it is visibly not a row: no terminal is a precondition failure, not a degradation — there is nothing to render into — exit status 3 with a message naming the reason. Kept outside the table deliberately, so neither that sentence nor `PRD.md` → Success criteria ("Every degraded state in the specification renders content, not an error screen") becomes false; `PRD.md` is therefore not edited by this change |
| 5 | Testing → Unit-tested modules | The list ends at `resolve::find_repo` and `resolve::openspec_bin` | Adds `ui::layout`, `ui::app`, `ui::view`, `ui::driver`, `ui::terminal`, and `ui::load`, naming `TestBackend` and the `TerminalOps` double as what stands in for the terminal |
| 6 | User interface → Responsive layout | "**100 columns or wider:** two columns — change list left, artifact detail right." — no widths given | Records the constraints this change freezes: `Length(40)` for the list, `Min(0)` for detail, so every column beyond 100 goes to detail |
| 7 | User interface → Responsive layout | "`Enter` opens detail and `Esc` returns" appears only under the narrow bullet | States that the route moves at **every** width: below 100 it selects which region is visible, at or above 100 which region is emphasised |
| 8 | User interface → List view | The mock's widest row is `> add-token-refresh    [4/9]  > claude - working` — 48 characters — with no note | Annotated as illustrating *content*, not width: 48 characters cannot fit a 38-column interior, and `list-view` owns how a row is shortened |
| 9 | Testing → Fixtures | "Two mechanisms, not checked-in fixture repositories" | "Three", adding `tests/fixtures/build-graph.txt` — a committed snapshot of tool output compared against a live `cargo tree` run, which is neither a `ScratchDir` tree nor an `include_str!` corpus |

`AGENTS.md` gains two in-place rewrites (task 10.3): the "Current repo state" paragraph,
which says the dashboard is not implemented and `ui` prints a placeholder banner, and the
"Views do no I/O" architecture bullet, which gains `src/ui/terminal.rs` as the only file
permitted to name a crossterm terminal-mode function, with the reason. Net size roughly
neutral: the placeholder sentence goes, the raw-mode rule arrives.

`openspec/IMPLEMENTATION-ORDER.md`'s Phase 4 `tui-shell` row was checked and needs **no**
correction: it names crossterm setup and teardown, the ratatui event loop, quit handling,
the 100-column breakpoint, and the `TestBackend` harness, and all five are what this change
builds. Task 10.4 re-checks it at implementation time and records explicitly that nothing
moved, if nothing did.

## No Remaining Implementation-Blocking Gaps

None remain. Specifically:

- **Every capability in proposal.md has a delta spec**, five for five, and both modified
  names match the live `openspec/specs/` tree. The `plugin-build` REMOVED block names the
  live requirement's header verbatim and carries Reason and Migration; the MODIFIED block
  carries the entire requirement, all six scenario names included.
- **Every one of the 54 spec scenarios has a verification-matrix row** — checked by set
  equality, not by eye — and every row names a tier, its collaborators, and a command.
- **Every scenario has a task**, and every task's filtered test run is gated on a counted
  minimum rather than an exit status.
- **No task invents a collaborator** the Test Boundaries table does not name, after
  finding 13's repair.
- **No PRD non-goal is crossed.** No write path anywhere under `openspec/` — proven by a
  snapshot comparison in the suite and by `OPENSPEC-UNTOUCHED` against a base SHA covering
  untracked files as well as tracked ones. No orchestration, no change authoring, no
  editing of OpenSpec files, no Windows.
- **No code path writes inside `openspec/`.** `ui::load` reads; nothing in the change opens
  a file for writing.
- **The roadmap's `tui-shell → changes-from-files` edge is kept, mechanically.**
  `NOCLI-SHELL` proves `src/ui/` names none of `from_cli`, `OpenspecCli`, `HerdrCli`,
  `CliChanges`, or `npm_prefix`, with a `src/changes.rs` positive control.
- **Every technical claim the design makes about `ratatui` 0.30.2 was verified by
  compiling and running against the real crate**, not from memory: the `crossterm`
  re-export, `Backend`'s associated `Error` type (so `LoopError::Draw(String)` needs no
  extra bound), `TestBackend::resize`, buffer indexing, the block title landing at column
  x+1, `Modifier::BOLD` landing on the corner cell, `Event::Paste`/`FocusGained`/`Mouse`
  all constructible, degenerate rects not panicking, and a 1-column header rendering `O`.
- **Every check in tasks.md was extracted and run** against the tree at HEAD, a synthetic
  end state, nine planted violations, and a git clone. All discriminate; none is
  structurally incapable of going red.
- `openspec validate tui-shell --strict` reports **valid**.

No unresolved decision requires user input.

## Deferred Non-Blocking Notes

Each has its resolution point already recorded in design.md or tasks.md.

- **Whether `ui` should render a plain-text summary instead of exiting 3 when stdout is a
  pipe.** design.md → Open Questions defers it to `degraded-states`, which owns the audit of
  every degraded row.
- **Whether the header should carry the `file mode` badge** from `SPEC.md`'s degraded-states
  table. Deferred to `degraded-states`, which the table's own row assigns it to. This change
  adds no badge.
- **How a change row is shortened for a 38-column interior.** `list-view`'s decision, taken
  with the `Length(40)` constraint written into `SPEC.md` by task 10.2 rather than
  discovered against it.
- **An MSRV CI job.** The `1.88` floor is checked by `DEPS` leg 4 reading `Cargo.toml`
  against every normal-graph package's `rust-version`, but no runner pins 1.88. design.md →
  Risks names this as `ci-pipeline`'s business, not this change's.
