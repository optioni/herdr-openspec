## Reviewed Artifacts

- proposal.md
- design.md
- tasks.md
- specs/openspec-binary/spec.md
- specs/setting-provenance/spec.md
- specs/refresh-worker/spec.md
- specs/agent-prompts/spec.md
- specs/quality-gates/spec.md (added during this review)

Four independent `planning-reviewer` agents made the finding pass, one slice each: (A) coverage,
delta fidelity and contradictions; (B) design, test boundaries and check falsifiability; (C)
task alignment and lifecycle; (D) factual verification. This log merges their findings. The
reviewers edited nothing.

## Reviewed Against

- This repository HEAD: `c261940`
- Sibling repository HEAD: Not applicable
- Working tree: clean apart from the untracked `openspec/changes/homebrew-probe/` planning
  files, which were intentionally included
- MODIFIED deltas diffed against the live spec at that HEAD: 10 of 10 blocks, plus 1 RENAMED:
  - openspec-binary: 5 blocks and the rename;
  - setting-provenance: 1;
  - refresh-worker: 1;
  - agent-prompts: 2;
  - quality-gates: 1.

  Every removed line is an intended edit, and no silent revert was found. One undeclared
  wording drop ("the **deferred** npm hook") is now declared in proposal.md; the name it
  referred to no longer exists. A trial `openspec archive` in a scratch copy applied cleanly
  (9 modified, 1 renamed at the time), and `openspec validate --specs --strict` passed 55/55
  afterwards (slice A). The trial also confirmed that RENAMED applies first, that MODIFIED
  must name the new header, and that MODIFIED refuses to drop a scenario.

## Gaps Found and Fixed

| Severity | Source Artifact | Problem | Repair | Updated Location |
|---|---|---|---|---|
| CRITICAL | design.md, tasks.md | Giving `start_collaborators` an 8th parameter fails `clippy::too_many_arguments` under `-D warnings`. Measured on rustc 1.91.1: `too many arguments (8/7)`. The repo has no `#[allow(` in `src/`. (Slices B, C and D independently.) | A `ProbeBindings { env, npm_hook, homebrew_prefixes }` bundle takes the function from 7 parameters to 6. Alternatives are recorded. `make lint` is added to groups 1 and 2. | design.md → Boundaries, Contracts, Decision 7; tasks.md 1.5, 2.2, 2.7; proposal.md → What Changes, Impact |
| CRITICAL | specs/openspec-binary, design.md, tasks.md | `a_full_probe_leaves_the_filesystem_byte_identical` returns at step 2 (`PATH=d` holds an executable), so extending its list never reaches step 5. Its "missing `bin/` is caught" claim was vacuous. (Slices B and D.) | The scenario and test are reworked so that steps 1–4 all miss and step 5 resolves `B` through `[E, B]`, asserting `BinSource::Homebrew`. | specs/openspec-binary "A full probe…"; design.md matrix; tasks.md 1.2 |
| WARNING | tasks.md, design.md | "Pass `&[]` at the other 13 `Startup {` literals" is wrong. 7 of the 14 grep hits are `ProbedStartup {`, and `run_wired_probed`'s literal (`:3361`) must forward `p.homebrew_prefixes`. | Every literal and call site is enumerated by line and by what it passes. | design.md → Contracts; tasks.md 2.2 |
| WARNING | specs/openspec-binary | The composition-root block said `run` passes and names `cli::npm_prefix`. It actually passes `cli::npm_probe_hook`, and `NOCLI-SHELL` forbids `npm_prefix` under `src/ui/`. It also said `start_collaborators` "reaches it through `worker_cli_from_env`", which is false since degraded-states. | Corrected to `npm_probe_hook`. The `worker_cli_from_env` sentence is now past tense. | specs/openspec-binary "The composition root injects…" |
| WARNING | specs/refresh-worker | The block still said universally that the resolved shim's first line is `#!/usr/bin/env node`. That contradicts its own new Homebrew scenario: the formula rewrites it to `#!/opt/homebrew/opt/node/bin/node`. | Qualified to npm installs (steps 3 and 4), with one sentence on Homebrew's absolute shebang. | specs/refresh-worker, first paragraph |
| WARNING | proposal.md, specs | `quality-gates` pins `WIRED`'s required-name count at "thirteen" (already fourteen at HEAD), and the plan appended a fifteenth leg-1 name without a delta. | `quality-gates` delta added. The count is corrected to fourteen. `HOMEBREW_PREFIXES` gets a body-scoped leg 8 instead of a leg-1 entry, and its new scenario has three recorded plants. | specs/quality-gates (new); proposal.md → Capabilities, Impact; design.md → Decision 8 |
| WARNING | specs/openspec-binary | "Step 5 SHALL read no environment variable" had no scenario, so an implementation reading `HOMEBREW_PREFIX` passed everything. | AND clause added: with an empty list and `HOMEBREW_PREFIX` set to a usable scratch prefix, no binary resolves. | specs/openspec-binary "A Homebrew prefix is searched…"; tasks.md 1.2 |
| WARNING | design.md, tasks.md | The post-archive `## Purpose` rewrite had no owning task. `spec_purposes.rs` checks presence only. | Task 6.9 added, with a grep check. | tasks.md 6.9; design.md → Migration Plan |
| WARNING | tasks.md | The acceptance test's only RED was a compile error, not a behavioural failure. | 2.2 adds the field and bundle while the probe still receives `&[]`, and records the failure on `!dashboard.file_mode` before 2.3 forwards the list. | tasks.md 2.2–2.3; design.md → Test Strategy |
| WARNING | tasks.md | The untracked change directory makes `make gates` (`OPENSPEC-UNTOUCHED`) fail from group 3, and only 6.1 said to commit it. | The change directory is committed at planning time, and the preamble says so. | tasks.md preamble |
| WARNING | tasks.md, design.md | The source-text check had no defined cut and no plant, and could be satisfied by the constant's own definition or a comment. `openspec_bin_from_env` also has no production caller. | Cut from `pub fn openspec_bin_from_env(` to the next line that is exactly `}`, with comments stripped and a planted `&[]` in 1.5. The off-live-path status is recorded. | design.md → Boundaries, Test Strategy; tasks.md 1.2, 1.5 |
| WARNING | tasks.md | Check 2.4's `grep -c HOMEBREW_PREFIXES … → 1` fails on a correct tree once a doc comment names the constant, and it scanned one file only. | Replaced by a comment-stripped sweep of every test slice under `src/` except `src/resolve.rs`, with a negative control. Leg 8 proves presence in `run`. | tasks.md 2.5 |
| SUGGESTION | specs/openspec-binary | A relative or empty injected prefix would resolve against the pane's cwd, which step 2 forbids. | Requirement text now says a non-absolute entry contributes nothing, proven through a pure `homebrew_candidates` on `path_candidates`' terms, because a resolution test cannot tell a skipped entry from a missed one at the crate root. | specs/openspec-binary step-5 paragraph and "…tried in list order…"; design.md → Boundaries; tasks.md 1.2 |
| SUGGESTION | specs/openspec-binary | The "absent prefix" re-run used two existing directories. | One re-run prefix is now a path that was never created. | specs/openspec-binary "Nothing anywhere…" |
| SUGGESTION | specs, design.md | The spec implied the `Homebrew` label always names the installer, while design accepted `/usr/local` false positives. | The step-5 paragraph states that anything at `<prefix>/bin/openspec` is reported as `Homebrew`. | specs/openspec-binary step-5 paragraph; design.md → Decisions 3 and 5 |
| SUGGESTION | specs/agent-prompts | "Because nvm is lazy-loaded" is no longer the whole reason on the reference machine: `brew shellenv` runs only from `.zprofile`. | Reason added in the delta; `SPEC.md:1011` is added to task 5.1. | specs/agent-prompts; tasks.md 5.1 |
| SUGGESTION | tasks.md | Code doc comments that go stale (`resolve.rs:235-247`, `:322-327`, `ui/mod.rs:165`, `:190-192`, `launch.rs:288`) were owned by no task. | Listed in design and assigned to 1.3 and 2.3. | design.md → Boundaries; tasks.md 1.3, 2.3 |
| SUGGESTION | tasks.md | 2.3's contract diff ran against the working tree after group 1 had been committed, and its pattern could not match the new variant. | Diff against `c261940`, with `Homebrew` and `ProbeBindings` in the pattern. | tasks.md 2.4 |
| SUGGESTION | design.md | Acceptance-test arms did not discriminate on their own on a machine that has Homebrew, and the badge was unasserted. | Adds a stand-in-log assertion and a header-badge assertion. The duplicate arm in `run_wired_probes_through_the_injected_hook` is dropped. | design.md → Test Strategy, matrix |
| SUGGESTION | design.md | Factual errors: `cli.rs:439` (actually 435–437); `BinCache` described as used in production (it is not); the Herdr and git rows said "stand-in" (they are non-existent paths); `SPEC.md` credited with "fourth-step" (it says "fourth step"); `settings.rs:192` called a construction (it is an `==`); a weak test named for the prompt scenario. | Each corrected. The prompt row now names `each_intent_produces_its_own_text_against_the_same_binary_and_change`. | design.md → Context, Persistence, Test Boundaries, Decision 1, Contracts, matrix |
| SUGGESTION | proposal.md | "`worktree-agents` … shares no file" was false: both changes edit `SPEC.md` and `AGENTS.md`. | Reworded to "shares no capability; both edit different paragraphs". | proposal.md → Impact; design.md → Risks; tasks.md 5.2 |
| SUGGESTION | design.md | Security was unaddressed: step 5 executes from outside `PATH`. | Risks line added. | design.md → Risks |

Sweep after repair:

- `openspec validate homebrew-probe --strict` → valid.
- `grep -c "^#### Scenario"` over the five deltas → 26 + 5 + 5 + 4 + 5 = 45 scenarios, each
  with a matrix row.
- Re-counting every `cargo test --lib` filter in design.md against the `c261940` test list
  gives 32 existing filters at exactly 1 and 8 new filters at 0.
- No artifact still says "13 `Startup {` literals", "leg 1 gains `HOMEBREW_PREFIXES`", or
  `cli.rs:439`.

## No Remaining Implementation-Blocking Gaps

None remain. Both CRITICALs are repaired in the owning artifacts, and no decision needs user
input.

## Deferred Non-Blocking Notes

- **Carried scenarios with partial existing coverage** (slice D), left as they stand:
  - `run_wired_probes_through_the_injected_hook` runs at 120 columns only. The new
    acceptance test covers both widths for the Homebrew arm.
  - `file_mode_leaves_a_refusing_and_g_working_in_the_shipped_root` has no resolving control
    arm.
  - `g_still_works_with_no_binary` drives `handle`, not `decide`.

  These are pre-existing coverage gaps in scenarios this change carries unchanged, and are
  recorded in design.md → Risks.
- **Specs outside this change that list the probe's first four inputs** (`agent-launch`,
  `dashboard-loop`, `agent-poller`) stay satisfiable and are recorded in design.md → Risks.
  `agent-launch` is held by `worktree-agents`.
